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
