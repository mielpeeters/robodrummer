use std::{
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use crate::guier::Gui;
use midi_control::{ControlEvent, KeyEvent, MidiMessage, MidiMessageSend};

use super::{ArpeggioArgs, CCArgs, CombinerArgs};
use crate::arpeggio::Arpeggio;

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

fn drum_loop(
    args: CombinerArgs,
    mut gui: Gui,
    wait_rx: mpsc::Receiver<f64>,
    nw_rx: mpsc::Receiver<f32>,
    mut midi_out: midir::MidiOutputConnection,
) -> Result<(), Box<dyn std::error::Error>> {
    // GUI output
    gui.add_row("BPM", 120);
    gui.add_row("subdivision", args.subdivision);
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

        std::thread::sleep(wait_dur);
    }
}

fn map_model_to_cc(
    model_output: f32,
    min: &mut f32,
    max: &mut f32,
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
    } else if model_output > *max {
        *max = model_output;
    }

    let model_range = *max - *min;
    let model_output_normalized = (model_output - *min) / model_range;
    (model_output_normalized * f32::from(cc_range)) as u8 + cc_offset
}

fn cc_loop(
    _args: CombinerArgs,
    cc_args: CCArgs,
    mut gui: Gui,
    nw_rx: mpsc::Receiver<f32>,
    mut midi_out: midir::MidiOutputConnection,
) -> Result<(), Box<dyn std::error::Error>> {
    // GUI output
    gui.add_row("cc target", cc_args.cc);
    gui.add_row("value", 0);

    let mut cc_out;
    let mut model_output = 0.0;

    let mut model_min = 0.0;
    let mut model_max = 0.0;

    loop {
        model_output = get_last_sent(&nw_rx).unwrap_or(model_output);

        cc_out = map_model_to_cc(
            model_output,
            &mut model_min,
            &mut model_max,
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

        gui.update_row("value", &cc_out);
        gui.show();

        std::thread::sleep(Duration::from_millis(10));
    }
}

fn arpeggio_loop(
    args: CombinerArgs,
    arp_args: ArpeggioArgs,
    mut gui: Gui,
    nw_rx: mpsc::Receiver<f32>,
    wait_rx: mpsc::Receiver<f64>,
    context: zmq::Context,
    midi_out: midir::MidiOutputConnection,
) -> Result<(), Box<dyn std::error::Error>> {
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
                std::thread::sleep(Duration::from_secs_f32(arpeggio.duration));
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

        std::thread::sleep(wait_dur);
    }
}

pub fn combine(args: CombinerArgs) -> Result<(), Box<dyn std::error::Error>> {
    // connect to the midi output
    let midi_out = midier::create_midi_output_and_connect()?;

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

    match args.output {
        super::OutputMode::Drum => drum_loop(args, gui, wait_rx, nw_rx, midi_out),
        super::OutputMode::Arpeggio(arp_args) => {
            arpeggio_loop(args, arp_args, gui, nw_rx, wait_rx, context, midi_out)
        }
        super::OutputMode::CC(cc_args) => cc_loop(args, cc_args, gui, nw_rx, midi_out),
    }
}
