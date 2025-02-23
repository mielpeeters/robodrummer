use std::sync::mpsc::Sender;
// #![allow(unreachable_code)]
use std::thread;

use crate::guier::Gui;
use crate::messages::{MetronomeMessage, MidiNoteMessage};
use crate::metronomer::inputwindow::{HitAction, InputWindow};
use clap::Parser;

use std::sync::{Arc, Condvar, Mutex};

use super::MetronomeArgs;

// Current fft size is 2^10 = 1024
const FFT_SIZE_POW: u32 = 11;
const FFT_SIZE: usize = 2i32.pow(FFT_SIZE_POW) as usize;

const SAMPLE_PERIOD: f64 = 0.05;
const SAMPLE_RATE: f64 = 1.0 / SAMPLE_PERIOD;
const MAX_FREQ: f64 = SAMPLE_RATE / 2.0;
const FREQ_STEP: f64 = MAX_FREQ * 2.0 / FFT_SIZE as f64;

#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    /// Port to publish on
    #[clap(long, default_value_t = 5432)]
    metronome_port: u16,

    /// Port to publish on
    #[clap(long, default_value_t = 6543)]
    midi_port: u16,
}

fn gather_data(data: Arc<(Mutex<InputWindow>, Condvar)>, midi_sub: zmq::Socket) {
    loop {
        let msg = midi_sub.recv_bytes(0).unwrap();
        let msg: MidiNoteMessage = bincode::deserialize(&msg).unwrap();

        if !msg.is_input() {
            continue;
        }

        if data.0.lock().unwrap().hit(HitAction::BandedInterval(3)) {
            // notify main thread about potentially changed best_frequency
            data.1.notify_one();
        }
    }
}

pub fn metronome(
    args: MetronomeArgs,
    tui_sender: Option<Sender<MetronomeMessage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // input data
    let window = Arc::new((
        Mutex::new(InputWindow::new_with_size(FFT_SIZE, SAMPLE_PERIOD)),
        Condvar::new(),
    ));

    let context = zmq::Context::new();
    let publisher = context.socket(zmq::PUB)?;
    publisher.bind(&format!(
        "ipc:///tmp/zmq_robodrummer_{}",
        args.metronome_port
    ))?;

    // subscriber for midi data
    let midi_sub = context.socket(zmq::SUB)?;
    midi_sub.connect(&format!("ipc:///tmp/zmq_robodrummer_{}", args.midi_port))?;
    midi_sub.set_subscribe(b"")?;

    // writer for input data
    let wdw = Arc::clone(&window);
    let _handle = thread::spawn(move || gather_data(wdw, midi_sub));

    let mut last_best = 0.0;
    let mut last_hc = 0;

    let mut gui = Gui::new(&format!(
        "Metronomer (max freq: {:2.0}, freq step: {:4.3})",
        MAX_FREQ, FREQ_STEP
    ));
    gui.add_row("input count", window.0.lock().unwrap().hit_count);
    gui.add_row("best BPM", 120);

    if tui_sender.is_some() {
        gui.disable();
    }

    gui.show();

    // main loop
    loop {
        let (window, condvar) = &*window;
        let mut w = window.lock().unwrap();
        while last_hc == w.hit_count {
            w = condvar.wait(w).unwrap();
        }
        // best frequency atm
        let max_freq = w.best_frequency;
        last_hc = w.hit_count;

        drop(w);

        if last_best == max_freq {
            continue;
        }

        last_best = max_freq;

        let msg = max_freq.to_be_bytes();
        publisher.send(msg.as_slice(), 0)?;

        if gui.enabled {
            gui.update_row("best BPM", &(max_freq * 60.0));
            gui.update_row("input count", &last_hc);
            gui.show();
        }

        if let Some(sender) = &tui_sender {
            if sender.send(MetronomeMessage::Tempo(max_freq)).is_err() {
                // this connection is dead, close this thread!
                break;
            }
        }
    }

    Ok(())
}
