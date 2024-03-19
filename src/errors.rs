/*!
  This module defines some errors which are used throughout this crate.
*/

#![allow(dead_code)]

use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum NeuronError {
    /// Index was not in allowed range (index, length)
    IndexOutOfRange(usize, usize),
    CantCreateMidiOut,
    CantConnectMidi,
    FileNotFound(String),
    DataNotFound(String),
    ModelNotFound(String),
}

impl Display for NeuronError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NeuronError::IndexOutOfRange(idx, ln) => {
                write!(f, "Index was out of range: idx: {} and ln: {}", idx, ln)
            }
            NeuronError::CantCreateMidiOut => {
                write!(f, "Can't create a midi output port...")
            }
            NeuronError::CantConnectMidi => {
                write!(f, "Can't connect to that midi port...")
            }
            NeuronError::FileNotFound(s) => write!(f, "File `{}` not found.", s),
            NeuronError::DataNotFound(d) => write!(f, "Data `{}` does not extist.", d),
            NeuronError::ModelNotFound(m) => write!(f, "Model `{}` does not extist.", m),
        }
    }
}

impl Error for NeuronError {}
