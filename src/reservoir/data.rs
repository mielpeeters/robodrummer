/*!
* Data handling for the reservoir: saving and loading models
*/

use std::fs;
use std::io::Write;
use std::{error::Error, path::PathBuf};

use serde::Deserialize;

use super::Reservoir;
use crate::data::models_dir;

#[derive(Debug, Deserialize)]
pub struct NpyMetaData {
    pub leak_rate: f64,
    pub n: usize,
    pub res_res_path: String,
    pub in_res_path: String,
    pub bias_path: String,
    pub out_path: String,
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
        let mut path = models_dir()?;

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
        let mut path = models_dir()?;

        path.push(name.to_string() + ".bin");

        self.save_to_file(path)?;

        Ok(())
    }
}
