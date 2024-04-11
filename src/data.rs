use std::{collections::VecDeque, error::Error, io::Read};

use ndarray::{array, Array1};
use serde::{Deserialize, Serialize};

use crate::commands::{GenerateDataArgs, RhythmAlgorithm, TrainArgs};

#[derive(Serialize, Deserialize)]
pub struct TrainData {
    pub inputs: VecDeque<(f64, bool)>,
    pub targets: VecDeque<(f64, bool)>,
}

const TRAIN_DATA_HEIGHT: f64 = 1.0;

fn neuroner_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    // get the data dir for this app
    let mut path = dirs::data_dir().expect("Should get the data directory");

    path.push("neuroner");

    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    }

    Ok(path)
}

pub fn models_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    // get the data dir for this app
    let mut path = neuroner_dir()?;
    path.push("models");

    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    }

    Ok(path)
}

pub fn data_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    // get the data dir for this app
    let mut path = neuroner_dir()?;
    path.push("traindata");

    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    }

    Ok(path)
}

fn show_model_meta(metadata: &TrainArgs) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "     - size: \x1b[38;5;216m{}\x1b[0m\n",
        metadata.size
    ));
    output.push_str(&format!(
        "     - mode: \x1b[38;5;216m{}\x1b[0m\n",
        metadata.mode
    ));
    let structure = match metadata.npy {
        Some(_) => "Euler ESN",
        None => "Random ESN",
    };
    output.push_str(&format!(
        "     - structure: \x1b[38;5;216m{}\x1b[0m\n",
        structure
    ));
    output.push_str(&format!(
        "     - dataset: \x1b[38;5;216m{}\x1b[0m",
        metadata.data
    ));
    output
}

pub fn get_model_metadata(name: &str) -> Result<TrainArgs, Box<dyn Error>> {
    let meta_path = models_dir()?.join(format!("{}.toml", name));

    if meta_path.exists() {
        let toml_string = std::fs::read_to_string(meta_path)?;
        let metadata: TrainArgs = toml::from_str(&toml_string)?;
        Ok(metadata)
    } else {
        Err(format!("No metadata exists for model name {}", name).into())
    }
}

pub fn model_metadata_string(name: &str) -> String {
    match get_model_metadata(name) {
        Ok(metadata) => show_model_meta(&metadata),
        Err(_) => "NO METADATA".into(),
    }
}

pub fn list_models() -> Result<(), Box<dyn std::error::Error>> {
    // get the data dir for this app
    let dir = models_dir()?;
    let mut seen_names = vec![];

    for (_, path) in (std::fs::read_dir(dir)?).enumerate() {
        let name = path.unwrap().file_name();
        let name = name
            .to_str()
            .unwrap()
            .split('.')
            .next()
            .unwrap()
            .to_string();

        if !seen_names.contains(&name) {
            seen_names.push(name);
        }
    }

    println!("\x1b[1;92mTrained Models:\x1b[0m");
    let clr = 214;
    for (i, name) in seen_names.iter().enumerate() {
        let meta_info = model_metadata_string(name);

        println!("{i:3}: \x1b[38;5;{}m{name}\x1b[0m", clr as usize + i);
        println!("{}", meta_info);
    }

    Ok(())
}

fn show_data_meta(metadata: &GenerateDataArgs) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "     - algorithm: \x1b[38;5;216m{}\x1b[0m\n",
        metadata.algorithm
    ));
    if let RhythmAlgorithm::Euclidean(e) = &metadata.algorithm {
        output.push_str(&format!("       - k: \x1b[38;5;70m{}\x1b[0m\n", e.k));
        output.push_str(&format!("       - n: \x1b[38;5;70m{}\x1b[0m\n", e.n));
    }
    output.push_str(&format!(
        "     - bpm: \x1b[38;5;216m{}\x1b[0m\n",
        metadata.bpm
    ));
    output.push_str(&format!(
        "     - variance: \x1b[38;5;216m{}\x1b[0m\n",
        metadata.variance
    ));
    output.push_str(&format!(
        "     - scale: \x1b[38;5;216m{}\x1b[0m",
        metadata.scale
    ));
    output
}

pub fn get_data_metadata(name: &str) -> Result<GenerateDataArgs, Box<dyn Error>> {
    let meta_path = data_dir()?.join(format!("{}.toml", name));

    if meta_path.exists() {
        let toml_string = std::fs::read_to_string(meta_path)?;
        let metadata = toml::from_str(&toml_string)?;
        Ok(metadata)
    } else {
        Err(format!("No metadata exists for data name {}", name).into())
    }
}

pub fn data_metadata_string(name: &str) -> String {
    match get_data_metadata(name) {
        Ok(metadata) => show_data_meta(&metadata),
        Err(_) => "NO METADATA".into(),
    }
}

pub fn list_data() -> Result<(), Box<dyn std::error::Error>> {
    let dir = data_dir()?;

    println!("\x1b[1;92mTraining Data:\x1b[0m");
    let mut seen_names = vec![];

    for (_, path) in (std::fs::read_dir(dir)?).enumerate() {
        let name = path.unwrap().file_name();
        let name = name
            .to_str()
            .unwrap()
            .split('.')
            .next()
            .unwrap()
            .to_string();

        if !seen_names.contains(&name) {
            seen_names.push(name.clone());
        }
    }

    let clr = 214;
    for (i, name) in seen_names.iter().enumerate() {
        let meta_info = data_metadata_string(name);
        println!("{i:3}: \x1b[38;5;{}m{name}\x1b[0m", clr as usize + i);
        println!("{}", meta_info);
    }

    Ok(())
}

pub type Data = (Vec<Array1<f64>>, Vec<Option<Array1<f64>>>);

/// Load the training data from a `.bin` file
///
/// The training data is stored in a time-domain format,
/// and needs to be converted to a discrete timestep format.
/// This is where the parameters `timestep` and `input_width` come into play.
///
/// # Arguments
/// - `name`: The name of the training data file
/// - `timestep`: The time between each timestep
/// - `input_width`: The number of timesteps to consider as input
/// - `shift`: the amount of timesteps to shift the target data into the future
pub fn load_train_data(
    name: &str,
    timestep: f64,
    input_width: usize,
    target_width: usize,
    shift: Option<usize>,
) -> Result<Data, Box<dyn std::error::Error>> {
    let mut data_path = data_dir()?;
    data_path.push(format!("{}.bin", name));

    let mut data_file = std::fs::OpenOptions::new().read(true).open(data_path)?;

    let mut data = vec![];
    data_file.read_to_end(&mut data)?;

    let mut train_data: TrainData = bincode::deserialize(data.as_slice())?;

    let mut time_ms = 0.0;
    let mut remaining_inputs = 0;

    let mut inputs = vec![];
    let mut targets = vec![];

    while !train_data.targets.is_empty() && !train_data.inputs.is_empty() {
        // this timestep's target
        let mut target = None;
        if train_data.targets[0].0 <= time_ms {
            // this timestep is a target time
            let target_val = match train_data.targets[0].1 {
                true => TRAIN_DATA_HEIGHT,
                false => 0.0,
            };
            target = Some(array![target_val]);
            train_data.targets.pop_front();
        }

        if train_data.inputs[0].0 <= time_ms {
            train_data.inputs.pop_front();
            remaining_inputs = input_width;
        }

        let mut input_val = 0.0;

        if remaining_inputs > 0 {
            input_val = TRAIN_DATA_HEIGHT;
            remaining_inputs -= 1;
        }

        let input = array![input_val];

        inputs.push(input);
        targets.push(target);

        time_ms += timestep;
    }

    // move the first targets
    if let Some(shift) = shift {
        let mut new_targets = vec![];
        targets
            .iter()
            .skip(shift)
            .for_each(|x| new_targets.push(x.clone()));

        // add None for the shifted targets
        for _ in 0..shift {
            new_targets.push(None);
        }

        // we expect these to be equal length
        assert_eq!(targets.len(), new_targets.len());

        println!("Shifted targets by {} timesteps", shift);

        targets = new_targets;
    }

    // we expect the inputs and targets to be the same length
    assert_eq!(inputs.len(), targets.len());

    // post-process the targets to ensure the target width value is enforced
    if target_width > 1 {
        let left = target_width / 2 - 1;
        let right = target_width / 2;
        let indices: Vec<usize> = targets
            .iter()
            .enumerate()
            .filter_map(|(i, x)| if x.is_some() { Some(i) } else { None })
            .collect();

        indices.iter().for_each(|i| {
            let l = (i - left).max(0);
            let r = (i + right).min(targets.len() - 1);
            for j in l..=r {
                targets[j] = targets[*i].clone();
            }
        });
    }

    Ok((inputs, targets))
}
