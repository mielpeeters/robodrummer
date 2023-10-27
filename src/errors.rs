#![allow(dead_code)]

use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum NeuronError {
    /// Index was not in allowed range (index, length)
    IndexOutOfRange(usize, usize),
    CantCreateMidiOut,
    CantConnectMidi,
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
        }
    }
}

impl Error for NeuronError {}
