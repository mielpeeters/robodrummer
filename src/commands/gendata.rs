use std::{
    collections::VecDeque,
    error::Error,
    f64::consts::PI,
    fmt::Display,
    io::Write,
    ops::{Index, IndexMut},
};

use ndarray_rand::rand_distr::StandardNormal;
use rand::Rng;

use crate::data::{data_dir, TrainData};

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

    /// Convert the rhythm pattern to a time series
    /// (time, onset) pairs
    pub fn to_time_period<F>(&self, interpolate: F, period: f64) -> Vec<(f64, bool)>
    where
        F: Fn(f64) -> Vec<f64>,
    {
        let timestep = period / self.len() as f64;

        let mut time_series = Vec::new();

        let hit_times: Vec<f64> = self
            .0
            .iter()
            .enumerate()
            .filter_map(|(i, &b)| if b { Some(i as f64 * timestep) } else { None })
            .collect();

        for (i, hit_time) in hit_times.iter().enumerate() {
            // add this hit
            time_series.push((*hit_time, true));

            // interpolate to next hit
            let next_hit_time = if i < hit_times.len() - 1 {
                hit_times[i + 1]
            } else {
                // interpolate to next repetition
                period
            };

            let interp_times = interpolate(next_hit_time - *hit_time);
            for t in interp_times {
                time_series.push((t + *hit_time, false));
            }
        }

        time_series
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

fn uniform(density: f64) -> Box<dyn Fn(f64) -> Vec<f64>> {
    let func = move |width: f64| {
        // amount of points to generate
        let n = (width * density + 0.00001).floor() as usize;

        let dist = width / n as f64;

        (1..n).map(|i| i as f64 * dist).collect()
    };

    Box::new(func)
}

fn chebyshev(density: f64, offset: f64) -> Box<dyn Fn(f64) -> Vec<f64>> {
    let func = move |width: f64| {
        // amount of points to generate
        let eff_width = width - offset * 2.0;
        let n = (eff_width * density + 0.00001).floor() as usize - 1;

        log::debug!("Width: {}, Density: {}, N: {}", width, density, n);

        (0..n)
            .rev()
            .map(|i| {
                let res = eff_width / 2.0
                    * (((2.0 * i as f64 + 1.0) * PI / (2.0 * n as f64)).cos() + 1.0);
                log::debug!("Chebyshev {i}: {}", res + offset);
                res + offset
            })
            .collect()
    };

    Box::new(func)
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

fn generate_input_times(mspb: f64, var: f64, duration: f64) -> Vec<f64> {
    let mut rng = rand::thread_rng();

    // amount of beats to generate input data for
    let n = (duration * 1000.0 / mspb) as usize;
    let mut times = Vec::with_capacity(n);

    for i in 0..n {
        let mut offset: f64 = rng.sample(StandardNormal);
        offset *= var;
        if i == 0 {
            offset = 0.0;
        }
        let time = (i as f64 * mspb) + offset;
        times.push(time);
    }

    times
}

fn pattern_to_csv(pattern: &RhythmPattern, args: &GenerateDataArgs) -> Result<(), Box<dyn Error>> {
    let data_path = data_dir()?;

    // if the user supplied an extension, remove it
    let name = args.output.split('.').next().unwrap();

    // metadata file to store parameters for later use
    let mut meta_path = data_path.clone();
    meta_path.push(format!("{name}.toml"));

    // the csv data file
    let mut csv_path = data_path.clone();
    csv_path.push(format!("{name}.csv"));

    // the binary serialized data file
    let mut bin_path = data_path.clone();
    bin_path.push(format!("{name}.bin"));

    log::info!(
        "Writing to files: {:?}, {:?} and {:?}",
        bin_path,
        csv_path,
        meta_path
    );

    // checking if user wants to overwrite
    if csv_path.exists() {
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

    // open the bin file
    let mut bin_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(bin_path)
        .unwrap();

    // now write the actual data
    let mut csv_writer = csv::Writer::from_path(csv_path)?;

    // write the header
    csv_writer.write_record(["t", "input", "target"])?;

    // calculate time between two pulses
    // TODO: input beat scaling
    let mspb = 60000.0 / args.bpm;
    let mspb_target = mspb * args.scale as f64;

    // create one period of the target pattern
    let interpolate = match args.density {
        Some(d) => chebyshev(d as f64 / mspb_target, args.offset),
        None => uniform(pattern.len() as f64 / mspb_target),
    };

    let period: Vec<(f64, bool)> = pattern.to_time_period(interpolate, mspb_target);

    let n_periods = (args.duration_s * 1000.0 / mspb_target) as usize;
    let mut targets: VecDeque<(f64, bool)> = (0..n_periods)
        .flat_map(|i| {
            period
                .iter()
                .map(move |&(time, flag)| (time + mspb_target * (i as f64), flag))
        })
        .collect();
    let mut inputs: VecDeque<(f64, bool)> =
        generate_input_times(mspb, args.variance, args.duration_s)
            .iter()
            .map(|x| (*x, true))
            .collect();

    // create the data object
    let train_data = TrainData {
        inputs: inputs.clone(),
        targets: targets.clone(),
    };
    let train_data = bincode::serialize(&train_data)?;
    bin_file.write_all(&train_data)?;

    while !targets.is_empty() && !inputs.is_empty() {
        // write either an input, a target or both
        match targets[0].0.total_cmp(&inputs[0].0) {
            // next data is target
            std::cmp::Ordering::Less => {
                csv_writer.write_record([
                    targets[0].0.to_string(),
                    "".to_string(),
                    match targets[0].1 {
                        true => "1".to_string(),
                        false => "0".to_string(),
                    },
                ])?;
                targets.pop_front().unwrap();
            }
            // next data is input
            std::cmp::Ordering::Greater => {
                csv_writer.write_record([
                    inputs[0].0.to_string(),
                    match inputs[0].1 {
                        true => "1".to_string(),
                        false => "0".to_string(),
                    },
                    "".to_string(),
                ])?;
                inputs.pop_front().unwrap();
            }
            // next data is both
            std::cmp::Ordering::Equal => {
                csv_writer.write_record([
                    inputs[0].0.to_string(),
                    match inputs[0].1 {
                        true => "1",
                        false => "0",
                    }
                    .to_string(),
                    match targets[0].1 {
                        true => "1",
                        false => "0",
                    }
                    .to_string(),
                ])?;
                targets.pop_front().unwrap();
                inputs.pop_front().unwrap();
            }
        }
    }

    // TODO:

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
