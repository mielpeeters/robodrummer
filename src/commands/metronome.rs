#![allow(unreachable_code)]
use std::{thread, time::Duration};

use crate::guier::Gui;
use crate::metronomer::inputwindow::{HitAction, InputWindow};
use clap::Parser;

use std::sync::{Arc, Mutex};

use super::MetronomeArgs;

// Current fft size is 2^10 = 1024
const FFT_SIZE_POW: u32 = 10;
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

fn gather_data(data: Arc<Mutex<InputWindow>>, midi_sub: zmq::Socket) {
    loop {
        let msg = midi_sub.recv_msg(0).unwrap();
        let msg = msg.as_str().unwrap();

        log::info!("Received: {}", msg);

        data.lock().unwrap().hit(HitAction::BandedInterval(3))
    }
}

pub fn metronome(args: MetronomeArgs) -> Result<(), Box<dyn std::error::Error>> {
    // input data
    let window = Arc::new(Mutex::new(InputWindow::new_with_size(
        FFT_SIZE,
        SAMPLE_PERIOD,
    )));

    let context = zmq::Context::new();
    let publisher = context.socket(zmq::PUB)?;
    publisher.bind(&format!("tcp://*:{}", args.metronome_port))?;

    // subscriber for midi data
    let midi_sub = context.socket(zmq::SUB)?;
    midi_sub.connect(&format!("tcp://localhost:{}", args.midi_port))?;
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
    gui.add_row("input_count", window.lock().unwrap().hit_count);
    gui.add_row("best BPM", 120);

    gui.show();

    // main loop
    loop {
        // best frequency atm
        let max_freq = {
            let w = window.lock().unwrap();
            let hc = w.hit_count;
            if hc != last_hc {
                gui.update_row("input_count", &hc);
                gui.show();
                last_hc = hc;
            }
            w.best_frequency
        };

        if last_best == max_freq {
            // sleep a bit for performance reasons
            thread::sleep(Duration::from_millis(100));
            continue;
        }

        last_best = max_freq;

        let msg = max_freq.to_be_bytes();
        publisher.send(msg.as_slice(), 0)?;

        gui.update_row("best BPM", &(max_freq * 60.0));
        gui.show();

        // sleep for a bit
        thread::sleep(Duration::from_millis(200));
    }
    Ok(())
}
