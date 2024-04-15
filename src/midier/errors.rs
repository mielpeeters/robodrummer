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
        }
    }
}

impl Error for MidiError {}
