/*!
* Messages that are sent to the TUI to inform about the child thread's states'
*/

use std::fmt::Display;

use serde::{Deserialize, Serialize};

pub enum MidiTuiMessage {
    /// A message about the midi note(s) that were played
    MidiNotes(Vec<u8>),
    /// An error occurred
    Error(String),
    /// Heartbeat to check if connection is alive
    Heartbeat,
    /// Midi options the user can select between
    MidiOptions(Vec<String>),
    /// The selected midi option
    MidiSelected(usize),
}

pub enum NetworkMessage {
    /// The network's last output
    Output(f64),
}

pub enum MetronomeMessage {
    /// The metronome's last tempo estimate (in Hz)
    Tempo(f64),
    /// Midi options the user can select between
    MidiOptions(Vec<String>),
    /// The selected midi option
    MidiSelected(usize),
}

pub enum CombinerMessage {
    /// The output state at some percentage in the loop
    Output((f64, f64)),
    Heartbeat,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MidiNoteMessage {
    /// The midi notes of the user input
    InputNotes(Vec<u8>),
    /// A midi note at the output
    OutputNote,
}

impl Display for MidiTuiMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MidiTuiMessage::MidiNotes(m) => write!(f, "{:?}", m),
            MidiTuiMessage::Error(e) => write!(f, "{}", e),
            MidiTuiMessage::Heartbeat => write!(f, "Heartbeat"),
            MidiTuiMessage::MidiOptions(m) => write!(f, "{:?}", m),
            MidiTuiMessage::MidiSelected(s) => write!(f, "Midi selected: {}", s),
        }
    }
}

impl Display for NetworkMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkMessage::Output(o) => write!(f, "{:.3}", o),
        }
    }
}

impl Display for MetronomeMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetronomeMessage::Tempo(t) => write!(f, "{:.2}", t),
            MetronomeMessage::MidiOptions(m) => write!(f, "{:?}", m),
            MetronomeMessage::MidiSelected(s) => write!(f, "Midi selected: {}", s),
        }
    }
}

impl MidiNoteMessage {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let msg: MidiNoteMessage = bincode::deserialize(bytes)?;
        Ok(msg)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let bytes = bincode::serialize(self)?;
        Ok(bytes)
    }

    pub fn is_input(&self) -> bool {
        matches!(self, MidiNoteMessage::InputNotes(_))
    }
}
