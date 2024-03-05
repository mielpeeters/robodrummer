/*!
* Data handling for the reservoir: saving and loading models
*/

use std::fs;
use std::io::Write;
use std::{error::Error, path::PathBuf};

use dirs::data_dir;

use super::Reservoir;

pub fn neuroner_dir() -> Result<PathBuf, Box<dyn Error>> {
    // get the data dir for this app
    let mut path = data_dir().expect("Should get the data directory");

    path.push("neuroner");
    path.push("models");

    if !path.exists() {
        fs::create_dir_all(&path)?;
    }

    Ok(path)
}

pub fn list_models() -> Result<(), Box<dyn Error>> {
    // get the data dir for this app
    let dir = neuroner_dir()?;

    println!("\x1b[1;92mTrained Models:\x1b[0m");
    let clr = 214;
    for (i, path) in (fs::read_dir(dir)?).enumerate() {
        let name = path.unwrap().file_name();
        let name = name.to_str().unwrap().split('.').collect::<Vec<&str>>()[0];
        println!("{i:3}: \x1b[38;5;{}m{name}\x1b[0m", clr as usize + i);
    }

    Ok(())
}

impl Reservoir {
    pub fn save_to_file(&self, filename: PathBuf) -> Result<(), Box<dyn Error>> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(filename)?;

        let model = bincode::serialize(self)?;

        file.write_all(model.as_slice())?;

        Ok(())
    }

    pub fn load_from_name(model_name: &str) -> Result<Self, Box<dyn Error>> {
        let mut path = neuroner_dir()?;

        path.push(model_name.to_string() + ".bin");

        let bytes = fs::read(path)?;

        let model: Self = bincode::deserialize(bytes.as_slice())?;

        Ok(model)
    }

    pub fn load_from_file(filename: PathBuf) -> Result<Self, Box<dyn Error>> {
        let bytes = fs::read(filename)?;

        let model: Self = bincode::deserialize(bytes.as_slice())?;

        Ok(model)
    }

    pub fn save(&self, name: &str) -> Result<(), Box<dyn Error>> {
        let mut path = neuroner_dir()?;

        path.push(name.to_string() + ".bin");

        self.save_to_file(path)?;

        Ok(())
    }
}
