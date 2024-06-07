use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread::{self, sleep},
    time::{Duration, Instant},
};

use crate::{
    guier::Gui,
    messages::{CombinerMessage, MidiNoteMessage},
    midier,
    robot::{self, WaveType},
};

use midi_control::{ControlEvent, KeyEvent, MidiMessage};

use super::{ArpeggioArgs, CCArgs, CombinerArgs};
use crate::arpeggio::Arpeggio;

pub struct NetworkPeriod {
    pub length: usize,
    pub activity: Vec<f32>,
    // the leak rate of the period adjustment
    pub leak_rate: f32,
    init: usize,
}

impl NetworkPeriod {
    pub fn new(length: usize, leak_rate: f32) -> Self {
        Self {
            length,
            activity: vec![0.0; length],
            leak_rate,
            init: 0,
        }
    }

    /// Update the activity of the network at a certain phase
    ///
    /// # Arguments
    /// * `phase` - the phase of the network
    /// * `value` - the value of the network at the phase
    pub fn update(&mut self, phase: f64, value: f32) {
        let idx = (phase * self.length as f64) as usize;
        let val = self.activity.get_mut(idx).unwrap();
        let lr = match self.init >= self.length {
            true => {
                self.init += 1;
                1.0
            }
            false => self.leak_rate,
        };
        *val = (1.0 - lr) * *val + lr * value;
    }

    pub fn get(&self, phase: f64) -> f32 {
        let idx = (phase * self.length as f64) as usize;
        *self.activity.get(idx).unwrap()
    }
}

type Prediction = (Instant, f32);

pub struct PredictionBuffer {
    pub buffer: VecDeque<Prediction>,
}

impl PredictionBuffer {
    pub fn new(shift: f64, timestep: f64) -> Self {
        // max needed size for delay compenstation, add 10 for why-not's sake
        let size = (shift / timestep) as usize + 10;

        let mut deque = VecDeque::with_capacity(size);
        let instant = Instant::now();

        for _ in 0..size {
            deque.push_back((instant, 0.0));
        }

        Self { buffer: deque }
    }

    pub fn add(&mut self, prediction: Prediction) {
        self.buffer.pop_back();
        self.buffer.push_front(prediction);
    }

    pub fn get_closest(&self, instant: Instant) -> (f32, f64) {
        let mut closest = self.buffer.front().unwrap();
        let mut min_diff = duration_pos_neg(closest.0, instant);
        for pred in self.buffer.iter() {
            let diff = duration_pos_neg(pred.0, instant);
            if diff < min_diff {
                min_diff = diff;
                closest = pred;
            }
        }

        (closest.1, min_diff)
    }
}

fn duration_pos_neg(moment_one: Instant, moment_two: Instant) -> f64 {
    let diff = moment_one.duration_since(moment_two).as_secs_f64();
    if diff == 0.0 {
        moment_two.duration_since(moment_one).as_secs_f64()
    } else {
        diff
    }
}

/// Non-blocking receiving calls to the receiver until the last value has been retreived
fn get_last_sent<T>(rx: &mpsc::Receiver<T>) -> Option<T> {
    let mut last = None;
    while let Ok(val) = rx.try_recv() {
        last = Some(val);
    }
    last
}

fn threshold_nw(nw_output: f32, threshold: f32) -> bool {
    nw_output > threshold
}

#[allow(unused)]
fn freq_subd_to_wait(time_wait: f64, subd: u8) -> f64 {
    time_wait / f64::from(subd)
}

fn ms_to_dur(ms: f64) -> Duration {
    Duration::from_secs_f64(ms / 1000.0)
}

fn s_to_dur(s: f64) -> Duration {
    Duration::from_secs_f64(s)
}

fn show_time(start: Instant, time: Instant) -> String {
    let elapsed = time - start;
    format!("{:.1} ms", elapsed.as_secs_f64() * 1000.0)
}

fn drum_robot_loop(
    args: CombinerArgs,
    drum_args: super::DrumArgs,
    mut gui: Gui,
    metro_rx: mpsc::Receiver<f64>,
    nw_rx: mpsc::Receiver<f32>,
    tui_sender: Option<Sender<CombinerMessage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // inform the user of robotic output
    gui.add_row("Robot", "");
    gui.add_row("Now", 0);
    gui.add_row("Quantize Interval", 0);
    gui.add_row("Sent Beat For Time", 0);
    gui.add_row("metro", 0);

    // create the beat sender
    let beat_send = Arc::new(AtomicBool::new(false));

    // start the output stream
    let _stream = robot::start(Arc::clone(&beat_send), WaveType::Saw(0.15));

    // wait for the first metronome signal
    let quantization_interval = metro_rx.recv()? / (args.subdivision as f64);
    let mut quantization_interval = s_to_dur(quantization_interval);

    // Get the output delay duration
    let delay = ms_to_dur(drum_args.delay);

    let mut next_metro = Instant::now() + quantization_interval - delay;

    let predictions = Arc::new(Mutex::new(PredictionBuffer::new(
        drum_args.shift,
        drum_args.timestep,
    )));

    let loop_time = Duration::from_millis(5);
    let mut iter = 0;

    let start = Instant::now();

    let predictions_clone = Arc::clone(&predictions);
    let shift = ms_to_dur(drum_args.shift);
    let _handle = thread::spawn(move || {
        loop {
            let now = Instant::now();
            // get last network output
            if let Some(nw) = get_last_sent(&nw_rx) {
                let future = now + shift;
                log::debug!("Got prediction for {}", show_time(start, future));
                predictions_clone.lock().unwrap().add((future, nw));
            }

            sleep(Duration::from_millis(1));
        }
    });

    loop {
        let now = Instant::now();

        gui.update_row("Now", &show_time(start, now));

        // check if we are on a quantized interval
        if now > next_metro {
            // get the prediction value at Instant::now() + delay
            let (prediction, err) = predictions.lock().unwrap().get_closest(now + delay);

            // HACK: this threshold could be smarter
            if err * 1000.0 < drum_args.timestep * 5.0 {
                log::debug!(
                    "Prediction: {:.2}, Error: {:.2} ms",
                    prediction,
                    err * 1000.0
                );
                // send the beat if the network output is above threshold
                if threshold_nw(prediction, args.threshold) {
                    beat_send.store(true, Ordering::Relaxed);
                    gui.update_row("Sent Beat For Time", &show_time(start, now + delay));
                }

                // update the next metronome instant
                next_metro += quantization_interval;
            }
        }

        // update the metronome value if needed
        quantization_interval = get_last_sent(&metro_rx)
            .map(|m| {
                let res = ms_to_dur(m * 1000.0 / (args.subdivision as f64));
                gui.update_row(
                    "Quantize Interval",
                    &format!("{:.1} ms", res.as_secs_f64() * 1000.0),
                );
                gui.show();
                res
            })
            .unwrap_or(quantization_interval);

        if iter % 10 == 0 {
            if let Some(sender) = &tui_sender {
                if sender.send(CombinerMessage::Heartbeat).is_err() {
                    break;
                }
            }
        }

        if iter % 2 == 0 {
            gui.show();
        }

        iter += 1;

        let remaining = loop_time - now.elapsed();
        if remaining > Duration::from_secs(0) {
            sleep(remaining);
        }
    }

    Ok(())
}

fn drum_loop(
    args: CombinerArgs,
    drum_args: super::DrumArgs,
    mut gui: Gui,
    wait_rx: mpsc::Receiver<f64>,
    nw_rx: mpsc::Receiver<f32>,
    tui_sender: Option<Sender<CombinerMessage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // GUI output
    gui.add_row("BPM", 120);
    gui.add_row("subdivision", args.subdivision);

    // split of into the robotic output if desired
    if let super::DrumOutput::Robot = drum_args.output {
        return drum_robot_loop(args, drum_args, gui, wait_rx, nw_rx, tui_sender.clone());
    };

    // connect to the midi output
    let mut midi_out = midier::create_midi_output_and_connect(args.device)?;

    gui.add_row("playing", false);

    let mut playing = false;
    let mut local_bpm = 120.0;

    // wait for the first metronome signal
    let mut waiting_time = wait_rx.recv()?;

    loop {
        let nw_output = get_last_sent(&nw_rx);

        if let Some(nw_play) = nw_output {
            playing = threshold_nw(nw_play, args.threshold);
            // TODO: only update on actual change
            gui.update_row("playing", &nw_play);
            gui.show();
        }

        if playing {
            midier::send_note(&mut midi_out, 3 - 1, args.note, 100);
        }

        waiting_time = get_last_sent(&wait_rx).unwrap_or(waiting_time);
        let wt = waiting_time / f64::from(args.subdivision);
        let wait_dur = Duration::from_secs_f64(wt);

        let bpm = 60.0 / waiting_time;

        if bpm != local_bpm {
            gui.update_row("BPM", &bpm);
            gui.show();
            local_bpm = bpm;
        }

        if let Some(sender) = &tui_sender {
            if sender.send(CombinerMessage::Heartbeat).is_err() {
                break;
            }
        }

        sleep(wait_dur);
    }

    Ok(())
}

// TODO: make every peak audible somehow
fn map_model_to_cc(
    model_output: f32,
    min: &mut f32,
    max: &mut f32,
    max_max: f32,
    cc_range: u8,
    cc_offset: u8,
    non_negative: bool,
) -> u8 {
    // update model range
    if model_output < *min {
        *min = model_output;
        if non_negative {
            *min = min.max(0.0);
        }
    } else if model_output * 0.7 > *max {
        *max = model_output * 0.7;
        if *max > max_max {
            *max = max_max;
        }
    }

    let model_range = *max - *min;
    let mut model_output_normalized = (model_output - *min) / model_range;
    model_output_normalized = model_output_normalized.min(1.0).max(0.0);
    // emphasize high outputs (peaks)
    model_output_normalized = model_output_normalized.powf(0.5);

    (model_output_normalized * f32::from(cc_range)) as u8 + cc_offset
}

fn cc_loop(
    args: CombinerArgs,
    cc_args: CCArgs,
    mut gui: Gui,
    nw_rx: mpsc::Receiver<f32>,
    metro_rx: mpsc::Receiver<f64>,
    tui_sender: Option<Sender<CombinerMessage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // connect to the midi output
    let mut midi_out = midier::create_midi_output_and_connect(args.device)?;

    // GUI output
    gui.add_row("cc target", cc_args.cc);
    gui.add_row("period", 0.0);
    gui.add_row("phase", 0.0);
    gui.add_graph("value", 0.0, 1.0);

    let mut cc_out;
    let mut model_output = 0.0;

    let mut model_min = 0.0;
    let mut model_max = 0.0;

    // get period time from metronome
    let mut period = metro_rx.recv()? * 4.0;

    // create network period
    // TODO: adjust the first value using some testing
    // probably should be lower
    let mut nw_periodic = NetworkPeriod::new(40, 0.15);

    let mut start: Instant = Instant::now();
    let mut phase = 0.0;

    loop {
        let passed_time = (Instant::now() - start).as_secs_f64();
        start = Instant::now();
        phase += passed_time / period;
        phase %= 1.0;
        model_output = get_last_sent(&nw_rx).unwrap_or(model_output);
        nw_periodic.update(phase, model_output);

        let val = nw_periodic.get(phase);

        cc_out = map_model_to_cc(
            val,
            &mut model_min,
            &mut model_max,
            // HACK: tmp
            40.0,
            cc_args.width.get(),
            cc_args.offset,
            cc_args.non_negative,
        );

        let msg = MidiMessage::ControlChange(
            (cc_args.channel - 1).into(),
            ControlEvent {
                control: cc_args.cc,
                value: cc_out,
            },
        );

        log::info!("Sending message: {:?}", &msg);

        let buffer: Vec<u8> = msg.into();
        if let Err(e) = midi_out.send(buffer.as_slice()) {
            log::error!("Error sending midi message: {}", e);
        };

        let rcv_period = get_last_sent(&metro_rx);
        if let Some(new_period) = rcv_period {
            period = new_period * 4.0;
        }

        gui.update_row("value", &cc_out);
        gui.update_row("period", &period);
        gui.update_row("phase", &phase);
        gui.replace_graph("value", &nw_periodic.activity);
        gui.show();

        if let Some(sender) = &tui_sender {
            if sender
                .send(CombinerMessage::Output((0.0, val.into())))
                .is_err()
            {
                // stop thread if sender is disconnected
                break;
            }
        }

        sleep(Duration::from_millis(10));
    }

    Ok(())
}

fn arpeggio_loop(
    args: CombinerArgs,
    arp_args: ArpeggioArgs,
    mut gui: Gui,
    nw_rx: mpsc::Receiver<f32>,
    wait_rx: mpsc::Receiver<f64>,
    context: zmq::Context,
    tui_sender: Option<Sender<CombinerMessage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // connect to the midi output
    let midi_out = midier::create_midi_output_and_connect(args.device)?;

    // GUI output
    gui.add_row("chord", "None");
    gui.add_row("BPM", 120);
    gui.add_row("subdivision", args.subdivision);

    // set up midi chord listener
    let midi_sub = context.socket(zmq::SUB).unwrap();
    midi_sub.connect(&format!(
        "ipc:///tmp/zmq_robodrummer_{}",
        arp_args.midi_port
    ))?;
    midi_sub.set_subscribe(b"")?;

    let (chord_tx, chord_rx) = mpsc::channel();
    let _handle = std::thread::spawn(move || loop {
        let msg = midi_sub.recv_bytes(0).unwrap();
        let msg: MidiNoteMessage = bincode::deserialize(&msg).unwrap();
        let MidiNoteMessage::InputNotes(mut chord) = msg else {
            continue;
        };
        chord.reverse();
        if chord_tx.send(chord).is_err() {
            break;
        }
    });

    let mut playing = false;
    let mut local_bpm = 120.0;
    let mut arpeggio = Arpeggio::new(&[40; 3], arp_args.duration, 12);

    // wait for the first metronome signal
    let mut waiting_time = wait_rx.recv()?;

    let midi_out = Arc::new(Mutex::new(midi_out));

    loop {
        let nw_output = get_last_sent(&nw_rx);

        if let Some(nw_play) = nw_output {
            playing = threshold_nw(nw_play, args.threshold);
            // TODO: only update on actual change
            gui.update_row("playing", &nw_play);
            gui.show();
        }

        if playing {
            if let Some(chord) = get_last_sent(&chord_rx) {
                arpeggio.update_chord(&chord);
                gui.update_row("chord", &format!("{:?}", &chord));
                gui.show();
            };

            let msg: Vec<u8> = MidiMessage::NoteOn(
                (arp_args.channel - 1).into(),
                KeyEvent {
                    key: arpeggio.next_note(),
                    value: 127,
                },
            )
            .into();
            midi_out.lock().unwrap().send(msg.as_slice())?;

            let midi_local = Arc::clone(&midi_out);
            let to_stop = arpeggio.chord[arpeggio.current];
            let _handle = std::thread::spawn(move || {
                sleep(Duration::from_secs_f32(arpeggio.duration));
                let msg: Vec<u8> = MidiMessage::NoteOff(
                    (arp_args.channel - 1).into(),
                    KeyEvent {
                        key: to_stop,
                        value: 0,
                    },
                )
                .into();
                midi_local.lock().unwrap().send(msg.as_slice()).unwrap();
            });
        }

        waiting_time = get_last_sent(&wait_rx).unwrap_or(waiting_time);
        let wt = waiting_time / f64::from(args.subdivision);
        let wait_dur = Duration::from_secs_f64(wt);

        let bpm = 60.0 / waiting_time;

        if bpm != local_bpm {
            gui.update_row("BPM", &bpm);
            gui.show();
            local_bpm = bpm;
        }

        if let Some(sender) = &tui_sender {
            if sender
                .send(CombinerMessage::Output((
                    0.0,
                    nw_output.unwrap_or(0.0).into(),
                )))
                .is_err()
            {
                break;
            }
        }

        sleep(wait_dur);
    }

    Ok(())
}

pub fn combine(
    args: CombinerArgs,
    tui_sender: Option<Sender<CombinerMessage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // connect to the metronome publisher
    let context = zmq::Context::new();
    let metronome = context.socket(zmq::SUB).unwrap();
    metronome.connect(&format!("ipc:///tmp/zmq_robodrummer_{}", args.metro_port))?;
    // listen to all messages from the metronome publisher
    metronome.set_subscribe(b"")?;

    // connect to the rhythmic feel publisher
    let feel = context.socket(zmq::SUB).unwrap();
    feel.connect(&format!("ipc:///tmp/zmq_robodrummer_{}", args.feel_port))?;
    // listen to all messages from the rhythmic feel publisher
    feel.set_subscribe(b"")?;

    // keep track of metronome output
    let (wait_tx, wait_rx) = mpsc::channel();
    let _handle = std::thread::spawn(move || loop {
        let msg = metronome.recv_bytes(0).unwrap();
        let freq = f64::from_be_bytes(msg.try_into().unwrap());
        if wait_tx.send(1.0 / freq).is_err() {
            break;
        }
    });

    // keep track of rhythmic feel output
    let (nw_tx, nw_rx) = mpsc::channel();
    let _handle = std::thread::spawn(move || loop {
        let msg = feel.recv_bytes(0).unwrap();
        let nw_output = f32::from_be_bytes(msg.try_into().unwrap());
        if nw_tx.send(nw_output).is_err() {
            break;
        }
    });

    let mut gui = Gui::new("Combiner");
    gui.add_row("output mode", &args.output);
    if tui_sender.is_some() {
        // don't print to stdout if the tui is running
        gui.disable();
    }

    let output = args.output.clone();

    match output {
        super::OutputMode::Drum(d) => drum_loop(args, d, gui, wait_rx, nw_rx, tui_sender),
        super::OutputMode::Arpeggio(arp_args) => {
            arpeggio_loop(args, arp_args, gui, nw_rx, wait_rx, context, tui_sender)
        }
        super::OutputMode::CC(cc_args) => cc_loop(args, cc_args, gui, nw_rx, wait_rx, tui_sender),
    }
}
