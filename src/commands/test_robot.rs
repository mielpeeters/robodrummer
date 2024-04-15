use std::{
    error::Error,
    sync::{atomic::AtomicBool, Arc},
    thread::sleep,
    time::{Duration, Instant},
};

use make_csv::{csv_entry, csv_start};

use crate::{midier, robot};

use super::{RobotArgs, RobotCommand};

const TIMEOUT: Duration = Duration::from_millis(500);
const BEAT_INCR: Duration = Duration::from_millis(50);

pub fn sweep() -> Result<(), Box<dyn Error>> {
    // set up incoming MIDI connection (robot's output) (listen for any channel)
    let rx = midier::setup_midi_receiver(None)?;

    // set up outgoing audio connection
    let beat = Arc::new(AtomicBool::new(false));
    let wave = robot::WaveType::Pulse(0.5);
    let _tx = robot::start(beat.clone(), wave);

    // start the csv output
    let mut writer = csv_start!("data/sweep.csv");
    csv_entry!(writer <- "beat_time", "elapsed");

    let mut beat_time = Duration::from_millis(1000);

    // start the sweep
    loop {
        // send an output beat
        let start = Instant::now();
        beat.store(true, std::sync::atomic::Ordering::Relaxed);

        // UNSURE: it might be that the midi signal gets lost here, or that there is some
        // desynchronization issues (say the robot hits the pad twice, this will lead to a midi
        // queue of 2, and we only get out one...)

        // get midi answer
        if let Ok(_msg) = rx.recv_timeout(TIMEOUT) {
            let elapsed = start.elapsed().as_secs_f64();
            csv_entry!(writer <- beat_time.as_secs_f64(), elapsed)
        } else {
            println!(
                "Missed beat with inter-beat time: {:?}",
                beat_time.as_secs_f64()
            );
        };

        // wait for the next beat
        let passed = start.elapsed();
        if passed < beat_time {
            sleep(beat_time - passed);
        }

        beat_time -= BEAT_INCR;
        if beat_time < Duration::from_millis(100) {
            break;
        }
    }

    Ok(())
}

pub fn test_waves() -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub fn robot(args: RobotArgs) -> Result<(), Box<dyn Error>> {
    match args.command {
        RobotCommand::Sweep => sweep(),
        RobotCommand::WaveType => test_waves(),
    }
}
