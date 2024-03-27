use std::{collections::VecDeque, io::Read};

use ndarray::{array, Array1};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TrainData {
    pub inputs: VecDeque<(f64, bool)>,
    pub targets: VecDeque<(f64, bool)>,
}

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

pub fn get_model_metadata(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let meta_path = models_dir()?.join(format!("{}.toml", name));

    let meta_info = if meta_path.exists() {
        let toml_string = std::fs::read_to_string(meta_path)?;
        let toml_value: toml::Value = toml::from_str(&toml_string)?;
        let toml_table = toml_value.as_table().unwrap();

        let mut info = vec![];

        let keys_to_show = ["inputs", "outputs", "size", "timestep", "sr", "leak_rate"];

        toml_table.iter().for_each(|(k, v)| {
            if keys_to_show.contains(&k.as_str()) {
                info.push(format!(
                    "\x1b[38;5;245m{}\x1b[0m: \x1b[38;5;250m{}\x1b[0m",
                    k, v
                ));
            }
        });

        info
    } else {
        ["NO METADATA".to_string()].to_vec()
    };

    Ok(meta_info.join(", "))
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
        let meta_info = get_model_metadata(name)?;

        println!("{i:3}: \x1b[38;5;{}m{name}\x1b[0m", clr as usize + i);
        println!("    ({})", meta_info);
    }

    Ok(())
}

pub fn get_data_metadata(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let meta_path = data_dir()?.join(format!("{}.toml", name));

    let meta_info = if meta_path.exists() {
        let toml_string = std::fs::read_to_string(meta_path)?;
        let toml_value: toml::Value = toml::from_str(&toml_string)?;
        let toml_table = toml_value.as_table().unwrap();

        let mut info = vec![];

        let keys_to_show = ["algorithm", "timestep", "bpm", "variance", "scale"];

        toml_table.iter().for_each(|(k, v)| {
            if keys_to_show.contains(&k.as_str()) {
                info.push(format!(
                    "\x1b[38;5;245m{}\x1b[0m: \x1b[38;5;250m{}\x1b[0m",
                    k, v
                ));
            }
        });

        info
    } else {
        ["NO METADATA".to_string()].to_vec()
    };

    Ok(meta_info.join(", "))
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
        let meta_info = get_data_metadata(name)?;
        println!("{i:3}: \x1b[38;5;{}m{name}\x1b[0m", clr as usize + i);
        println!("    ({})", meta_info);
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
            let target_val = match train_data.targets[0].1 {
                true => 1.0,
                false => 0.0,
            };
            // this timestep is a target time
            target = Some(array![target_val]);
            train_data.targets.pop_front();
        }

        if train_data.inputs[0].0 <= time_ms {
            train_data.inputs.pop_front();
            remaining_inputs = input_width;
        }

        let mut input_val = 0.0;

        if remaining_inputs > 0 {
            input_val = 1.0;
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

    Ok((inputs, targets))
}
