/*!
* Messages that are sent to the TUI to inform about the child thread's states'
*/

use std::fmt::Display;

pub enum MidiTuiMessage {
    /// A message about the midi note(s) that were played
    MidiNotes(Vec<u8>),
    /// An error occurred
    Error(String),
    /// Heartbeat to check if connection is alive
    Heartbeat,
}

pub enum NetworkMessage {
    /// The network's last output
    Output(f64),
}

pub enum CombinerMessage {
    /// The output state at some percentage in the loop
    Output((f64, f64)),
}

impl Display for MidiTuiMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MidiTuiMessage::MidiNotes(m) => write!(f, "{:?}", m),
            MidiTuiMessage::Error(e) => write!(f, "{}", e),
            MidiTuiMessage::Heartbeat => write!(f, "Heartbeat"),
        }
    }
}

impl Display for NetworkMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkMessage::Output(o) => write!(f, "{}", o),
        }
    }
}
