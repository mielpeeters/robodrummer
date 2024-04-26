use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread::sleep,
    time::{Duration, Instant},
};

use crate::{
    guier::Gui,
    midier,
    robot::{self, WaveType},
    tui::messages::CombinerMessage,
};

use midi_control::{ControlEvent, KeyEvent, MidiMessage, MidiMessageSend};

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

fn freq_subd_to_wait(time_wait: f64, subd: u8) -> f64 {
    time_wait / f64::from(subd)
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
    gui.add_row("Network", 0);
    gui.add_row("Wait Secs", 0);

    // create the beat sender
    let beat_send = Arc::new(AtomicBool::new(false));

    // start the output stream
    let _stream = robot::start(Arc::clone(&beat_send), WaveType::Pulse(0.3));

    // wait for the first metronome signal
    let mut metro = metro_rx.recv()?;
    let mut nw_output = 0.0;

    // create an agenda of future events
    let output_agenda: Arc<Mutex<VecDeque<Instant>>> = Arc::new(Mutex::new(VecDeque::new()));

    // start the output thread using the output agenda
    let local_output_agenda = Arc::clone(&output_agenda);
    let _handle = std::thread::spawn(move || loop {
        // NOTE: this assumes that the outputs are ordered
        let front_val = {
            let mut out = local_output_agenda.lock().unwrap();
            out.pop_front()
        };
        match front_val {
            Some(time) => {
                let now = Instant::now();
                let time_left = time.checked_duration_since(now);
                if let Some(wait) = time_left {
                    // TMP: this one should be made a lot more accurate probably
                    sleep(wait);
                }
                beat_send.store(true, Ordering::Relaxed);
            }
            None => {
                // NOTE: this could lead to some additional delays
                sleep(Duration::from_millis(5));
            }
        }
    });

    loop {
        // check if we need to send an output in some time
        nw_output = get_last_sent(&nw_rx).unwrap_or(nw_output);
        gui.update_row("Network", &nw_output);
        if threshold_nw(nw_output, args.threshold) {
            let now = Instant::now();
            let next_time = now + Duration::from_secs_f64(drum_args.offset)
                - Duration::from_secs_f64(drum_args.delay);
            let mut out = output_agenda.lock().unwrap();
            out.push_back(next_time);
            drop(out);
        }

        // get the last metronome reading
        metro = get_last_sent(&metro_rx).unwrap_or(metro);
        let wait_secs = freq_subd_to_wait(metro, args.subdivision);
        gui.update_row("Wait Secs", &wait_secs);
        gui.show();

        // send output to tui if needed
        if let Some(sender) = &tui_sender {
            if sender
                .send(CombinerMessage::Output((0.0, nw_output.into())))
                .is_err()
            {
                // stop thread if sender is disconnected
                break;
            }
        }

        sleep(Duration::from_secs_f64(wait_secs));
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
    let mut midi_out = midier::create_midi_output_and_connect()?;

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
            midier::send_note(&mut midi_out, 1, 51, 50);
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

        sleep(wait_dur);
    }
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
    _args: CombinerArgs,
    cc_args: CCArgs,
    mut gui: Gui,
    nw_rx: mpsc::Receiver<f32>,
    metro_rx: mpsc::Receiver<f64>,
    tui_sender: Option<Sender<CombinerMessage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // connect to the midi output
    let mut midi_out = midier::create_midi_output_and_connect()?;

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

        if let Err(e) = midi_out.send_message(msg) {
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
    let midi_out = midier::create_midi_output_and_connect()?;

    // GUI output
    gui.add_row("chord", "None");
    gui.add_row("BPM", 120);
    gui.add_row("subdivision", args.subdivision);

    // set up midi chord listener
    let midi_sub = context.socket(zmq::SUB).unwrap();
    midi_sub.connect(&format!("tcp://localhost:{}", arp_args.midi_port))?;
    midi_sub.set_subscribe(b"")?;

    let (chord_tx, chord_rx) = mpsc::channel();
    let _handle = std::thread::spawn(move || loop {
        let msg = midi_sub.recv_bytes(0).unwrap();
        let mut chord = msg.to_vec();
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

            midi_out.lock().unwrap().send_message(MidiMessage::NoteOn(
                (arp_args.channel - 1).into(),
                KeyEvent {
                    key: arpeggio.next(),
                    value: 127,
                },
            ))?;

            let midi_local = Arc::clone(&midi_out);
            let to_stop = arpeggio.chord[arpeggio.current];
            let _handle = std::thread::spawn(move || {
                sleep(Duration::from_secs_f32(arpeggio.duration));
                midi_local
                    .lock()
                    .unwrap()
                    .send_message(MidiMessage::NoteOff(
                        (arp_args.channel - 1).into(),
                        KeyEvent {
                            key: to_stop,
                            value: 0,
                        },
                    ))
                    .unwrap();
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
    metronome.connect(&format!("tcp://localhost:{}", args.metro_port))?;
    // listen to all messages from the metronome publisher
    metronome.set_subscribe(b"")?;

    // connect to the rhythmic feel publisher
    let feel = context.socket(zmq::SUB).unwrap();
    feel.connect(&format!("tcp://localhost:{}", args.feel_port))?;
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

    match args.output {
        super::OutputMode::Drum(d) => drum_loop(args, d, gui, wait_rx, nw_rx, tui_sender),
        super::OutputMode::Arpeggio(arp_args) => {
            arpeggio_loop(args, arp_args, gui, nw_rx, wait_rx, context, tui_sender)
        }
        super::OutputMode::CC(cc_args) => cc_loop(args, cc_args, gui, nw_rx, wait_rx, tui_sender),
    }
}
