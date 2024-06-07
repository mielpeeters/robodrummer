use std::{
    error::Error,
    sync::{atomic::AtomicBool, Arc},
    thread::sleep,
    time::{Duration, Instant},
};

use make_csv::{csv_entry, csv_start, csv_stop};

use crate::{
    midier,
    robot::{self, WaveType},
    utils::get_last_sent_timeout,
};

use super::commands::{RobotArgs, RobotCommand};

const TIMEOUT: Duration = Duration::from_millis(500);
const BEAT_INIT: Duration = Duration::from_millis(500);
const BEAT_CALIB: Duration = Duration::from_millis(300);
const BEAT_INCR: Duration = Duration::from_micros(12500);
const MEASUREMENT_COUNT: u32 = 2;
const WAVE_TEST_COUNT: u32 = 20;

pub fn sweep() -> Result<(), Box<dyn Error>> {
    // set up incoming MIDI connection (robot's output) (listen for any channel)
    let rx = midier::setup_midi_receiver(None, None, None)?;

    // set up outgoing audio connection
    let beat = Arc::new(AtomicBool::new(false));
    let wave = robot::WaveType::Saw(0.15);
    let _tx = robot::start(beat.clone(), wave);

    // start the csv output
    let mut writer = csv_start!("data/sweep.csv");
    csv_entry!(writer <- "beat_time", "elapsed");

    let mut beat_time = BEAT_INIT;

    // initialize the connection
    sleep(Duration::from_secs(2));

    let mut count = 0;

    // start the sweep
    loop {
        // get al rogue midi signals
        let _ = get_last_sent_timeout(&rx, Duration::from_millis(100));

        println!("Sending beat with inter-beat time: {:?}", beat_time);
        let start = Instant::now();
        beat.store(true, std::sync::atomic::Ordering::Relaxed);
        // get midi answer
        if let Some(msg) = get_last_sent_timeout(&rx, TIMEOUT) {
            let elapsed = start.elapsed().as_secs_f64();
            println!("Received: {:?}", msg);
            println!("\ttook: {:.1} ms", elapsed * 1000.0);
            csv_entry!(writer <- beat_time.as_secs_f64(), elapsed)
        } else {
            println!(
                "Missed beat with inter-beat time: {:?}",
                beat_time.as_secs_f64()
            );
        };

        writer.flush()?;

        // wait for the next beat
        let passed = start.elapsed();
        if passed < beat_time {
            sleep(beat_time - passed);
        }

        count += 1;
        if count % MEASUREMENT_COUNT == 0 {
            beat_time -= BEAT_INCR;
        }

        if beat_time < Duration::from_millis(100) {
            break;
        }
    }

    csv_stop!(writer);

    Ok(())
}

pub fn meas_delay() -> Result<(), Box<dyn Error>> {
    // set up incoming MIDI connection (robot's output) (listen for any channel)
    let rx = midier::setup_midi_receiver(None, None, None)?;

    let wave = WaveType::Saw(0.15);

    // start csv output
    let mut writer = csv_start!("data/calibrate.csv");
    csv_entry!(writer <- "wave_type", "elapsed");

    // initialize the connection
    sleep(Duration::from_secs(2));

    // set up outgoing audio connection
    let beat = Arc::new(AtomicBool::new(false));
    let (stream, _, _) = robot::start(beat.clone(), wave);

    let mut measurements = vec![];

    println!("\x1b[?1049h");

    for _ in 0..40 {
        // get al rogue midi signals
        let _ = get_last_sent_timeout(&rx, Duration::from_millis(100));

        // send an output beat
        println!("Sending {wave:?} beat");
        let start = Instant::now();
        beat.store(true, std::sync::atomic::Ordering::Relaxed);

        // get midi answer
        if let Some(_msg) = get_last_sent_timeout(&rx, TIMEOUT) {
            let elapsed = start.elapsed().as_secs_f64();
            measurements.push(elapsed);
            print!("\rtook: {:.1} ms", elapsed * 1000.0);
            csv_entry!(writer <- format!("{:?}", wave), elapsed)
        } else {
            println!("Missed beat with wave type: {:?}", wave);
        };

        writer.flush()?;

        // wait for the next beat
        let passed = start.elapsed();
        if passed < BEAT_CALIB {
            sleep(BEAT_CALIB - passed);
        }
    }

    println!("\x1b[?1049l");

    // technically not needed
    drop(stream);

    csv_stop!(writer);

    let avg = measurements.iter().sum::<f64>() / measurements.len() as f64;

    println!("Average delay: {:.1} ms", avg * 1000.0);

    Ok(())
}

pub fn test_waves() -> Result<(), Box<dyn Error>> {
    // set up incoming MIDI connection (robot's output) (listen for any channel)
    let rx = midier::setup_midi_receiver(None, None, None)?;

    let waves = vec![
        WaveType::Saw(0.15),
        WaveType::Pulse(0.15),
        WaveType::SlowSaw(0.15),
        WaveType::Slope(0.15),
    ];

    // start csv output
    let mut writer = csv_start!("data/test_waves.csv");
    csv_entry!(writer <- "wave_type", "elapsed");

    // initialize the connection
    sleep(Duration::from_secs(2));

    for wave in &waves {
        // set up outgoing audio connection
        let beat = Arc::new(AtomicBool::new(false));
        let (stream, _, _) = robot::start(beat.clone(), *wave);

        for _ in 0..WAVE_TEST_COUNT {
            // get al rogue midi signals
            let _ = get_last_sent_timeout(&rx, Duration::from_millis(100));

            // send an output beat
            println!("Sending {wave:?} beat");
            let start = Instant::now();
            beat.store(true, std::sync::atomic::Ordering::Relaxed);

            // get midi answer
            if let Some(msg) = get_last_sent_timeout(&rx, TIMEOUT) {
                let elapsed = start.elapsed().as_secs_f64();
                println!("Received: {:?}", msg);
                println!("\ttook: {:.1} ms", elapsed * 1000.0);
                csv_entry!(writer <- format!("{:?}", wave), elapsed)
            } else {
                println!("Missed beat with wave type: {:?}", wave);
            };

            writer.flush()?;

            // wait for the next beat
            let passed = start.elapsed();
            if passed < BEAT_INIT {
                sleep(BEAT_INIT - passed);
            }
        }

        // technically not needed
        drop(stream);
    }

    csv_stop!(writer);

    Ok(())
}

pub fn robot(args: RobotArgs) -> Result<(), Box<dyn Error>> {
    match args.command {
        RobotCommand::Sweep => sweep(),
        RobotCommand::WaveType => test_waves(),
        RobotCommand::Delay => meas_delay(),
    }
}
