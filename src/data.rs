use ndarray::Array1;

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
pub fn load_train_data(name: &str) -> Result<Data, Box<dyn std::error::Error>> {
    let mut data_path = data_dir()?;
    data_path.push(format!("{}.csv", name));

    let mut rdr = csv::Reader::from_path(data_path)?;

    let mut inputs = vec![];
    let mut targets = vec![];

    for result in rdr.records() {
        let record = result?;
        let input: Array1<f64> = record
            .iter()
            .skip(1)
            .take(1)
            .map(|x| x.parse().unwrap())
            .collect();
        let target: Option<Array1<f64>> = record
            .iter()
            .skip(2)
            .take(1)
            .map(|x| x.parse().ok())
            .collect();

        inputs.push(input);
        targets.push(target);
    }

    Ok((inputs, targets))
}
