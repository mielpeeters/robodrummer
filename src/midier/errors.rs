/*!
  This module defines some errors which are used throughout this crate.
*/

#![allow(dead_code)]

use std::{error::Error, fmt::Display};

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum MidiError {
    CantCreateMidiOut,
    CantCreateMidiIn,
    CantConnectMidi,
    PortNotOpen,
    DeviceNotFound(String),
}

impl Display for MidiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MidiError::CantCreateMidiOut => {
                write!(f, "Can't create a midi output port...")
            }
            MidiError::CantCreateMidiIn => {
                write!(f, "Can't create a midi input port...")
            }
            MidiError::CantConnectMidi => {
                write!(f, "Can't connect to that midi port...")
            }
            MidiError::DeviceNotFound(d) => {
                write!(f, "Device {} not found", d)
            }
            MidiError::PortNotOpen => {
                write!(f, "Port is not available")
            }
        }
    }
}

impl Error for MidiError {}
