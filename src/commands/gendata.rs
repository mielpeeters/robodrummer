use std::{
    error::Error,
    fmt::Display,
    ops::{Index, IndexMut},
};

use ndarray_rand::rand_distr::StandardNormal;
use rand::Rng;

use crate::data::data_dir;

use super::GenerateDataArgs;

/// A Rhythmic Pattern is just a collection of onsets and silent rests
///
/// This is modeled as a simple vector of booleans
pub struct RhythmPattern(Vec<bool>);

impl RhythmPattern {
    pub fn new(n: usize) -> Self {
        let pattern: Vec<bool> = vec![false; n];

        Self(pattern)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn show(&self) {
        println!("\x1b[1mRhythm Pattern:\x1b[0m\n\x1b[38;5;214m{self}\x1b[0m");
    }
}

impl Index<usize> for RhythmPattern {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for RhythmPattern {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Display for RhythmPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in &self.0 {
            if *b {
                write!(f, "⏺")?;
            } else {
                write!(f, "·")?;
            }
        }
        Ok(())
    }
}

fn euclidean(n: usize, k: usize) -> RhythmPattern {
    let mut pattern = RhythmPattern::new(n);
    let real_step = n as f64 / k as f64;

    log::info!("Generating Euclidean rhythm with n = {} and k = {}", n, k);
    log::info!("Real step: {}", real_step);
    for i in 0..k {
        let index = (real_step * i as f64).ceil() as usize;
        log::info!("Setting index {} to true", index);
        pattern[index] = true;
    }

    pattern
}

fn pattern_to_csv(pattern: &RhythmPattern, args: &GenerateDataArgs) -> Result<(), Box<dyn Error>> {
    let mut data_path = data_dir()?;

    // if the user supplied an extension, remove it
    let name = args.output.split('.').next().unwrap();

    // metadata file to store parameters for later use
    let mut meta_path = data_path.clone();
    meta_path.push(format!("{name}.toml"));

    // the actual data file
    data_path.push(format!("{name}.csv"));

    log::info!("Writing to files: {:?} and {:?}", data_path, meta_path);

    // checking if user wants to overwrite
    if data_path.exists() {
        log::warn!("File already exists, checking with user.");
        println!("The file \x1b[1malready exists\x1b[0m, do you want to overwrite it? [y/N]");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" {
            log::warn!("User chose not to overwrite, exiting.");
            println!("Exiting without writing to file.");
            return Ok(());
        }
    }

    // write the metadata to the metadata file
    let metadata = toml::to_string(args)?;
    std::fs::write(meta_path, metadata)?;

    // now write the actual data
    let mut csv_writer = csv::Writer::from_path(data_path)?;

    // write the header
    csv_writer.write_record(["t", "input", "target"])?;

    // calculate time between two pulses
    // TODO: input beat scaling
    let mspb = 60000.0 / args.bpm;
    let ms_per_pulse = mspb * args.scale as f64 / pattern.len() as f64;

    let mut time_ms = 0_f64;
    let mut pattern_index = 0;
    let mut remaining_inputs = 0;
    let mut t = 0;
    let mut t_next_input = 0;
    let mut beat_index = 0;
    let mut rng = rand::thread_rng();

    let mut steady = false;
    let steady_duration = args.steady_state as f64;

    while time_ms < args.duration_s * 1000.0 + steady_duration {
        let mut target = "";
        if time_ms % ms_per_pulse < args.timestep {
            if pattern[pattern_index % pattern.len()] {
                target = "1";
            } else {
                target = "0";
            }
            pattern_index += 1;
        }

        if args.steady_state > 0 && time_ms > args.duration_s * 1000.0 && !steady {
            log::info!("Entering steady state phase");
            steady = true;
        }

        let mut input = "0";
        if !steady && t == t_next_input {
            remaining_inputs = (args.width / args.timestep) as i32;
            beat_index += 1;

            // calculate the next input time
            let mut ms_next_beat: f64 = beat_index as f64 * mspb;
            let mut offset: f64 = rng.sample(StandardNormal);
            offset *= args.variance;
            ms_next_beat += offset;
            log::trace!("Input beat offset: \x1b[1m{:+.3} ms\x1b[0m", offset);

            t_next_input = (ms_next_beat / args.timestep).round() as i32;
        }

        if remaining_inputs > 0 {
            input = "1";
            remaining_inputs -= 1;
        }

        csv_writer.write_record([t.to_string(), input.to_string(), target.to_string()])?;

        time_ms += args.timestep;
        t += 1;
    }

    Ok(())
}

#[allow(unused)]
fn poly_pattern_to_csv(
    input_pattern: &RhythmPattern,
    target_pattern: &RhythmPattern,
    args: &GenerateDataArgs,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}

/// Generate input-output data to train the reservoir, based on the given arguments
///
/// This function uses research knowledge about rhythmic patterns to generate input-output data
///
/// # Result
/// This function writes to a csv file
pub fn gendata(args: GenerateDataArgs) -> Result<(), Box<dyn Error>> {
    // pseudo code

    // arguments:
    // - algorithm: an enum of possible rhythm generating algorithms
    //      - Euclidean
    //      - NP-DAG
    // - parameters for the sub-algorithm...

    let target_pattern = match &args.algorithm {
        super::RhythmAlgorithm::Euclidean(e) => euclidean(e.n, e.k),
        // TODO: NP-DAG algorithm implementation
        super::RhythmAlgorithm::NPDAG(_) => todo!("NP-DAG algorithm not implemented"),
        super::RhythmAlgorithm::PolyEuclidean(p) => euclidean(p.n, p.k),
    };

    target_pattern.show();

    pattern_to_csv(&target_pattern, &args)
}
