/*!
* This module defines the command line interface for the application using clap.
*/

mod combine;
mod completions;
mod gendata;
mod midi_broker;
mod run;
mod test_robot;
mod train;

use std::{fmt::Display, num::NonZeroU8};

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use nestify::nest;
use serde::{Deserialize, Serialize};

pub use crate::tui::start_tui as tui;
pub use combine::combine;
pub use completions::update_completions;
pub use gendata::gendata;
pub use midi_broker::broke;
pub use run::run;
pub use test_robot::robot;
pub use train::train;

use crate::activation::Activation;

const METRONOME_PORT: u16 = 5432;
const FEEL_PORT: u16 = 4321;
const MIDI_PORT: u16 = 6543;
const OSC_PORT: u16 = 30000;

#[derive(Parser, Debug)]
pub struct Arguments {
    /// The subcommand to run
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Train(TrainArgs),
    Run(RunArgs),
    GenerateData(GenerateDataArgs),
    Completions(CompletionsArgs),
    MidiBroker(MidiBrokerArgs),
    Combine(CombinerArgs),
    Tui(TuiArgs),
    Robot(RobotArgs),
}

#[derive(Args, Debug, Clone)]
pub struct RunArgs {
    /// The amount of ms between evaluations
    #[arg(short, long, default_value_t = 2.0)]
    pub timestep: f64,

    /// The name of the model
    #[arg(short, long, default_value = "good")]
    pub model: String,

    /// List the available model names
    #[arg(short, long, default_value_t = false)]
    pub list: bool,

    /// Port on which the network output is published using zmq
    #[arg(long, default_value_t = FEEL_PORT)]
    pub network_port: u16,

    // Port on which to listen for metronome value
    #[arg(long, default_value_t = METRONOME_PORT)]
    pub metronome_port: u16,

    /// Port on which the midi inputs come in
    #[arg(long, default_value_t = MIDI_PORT)]
    pub midi_port: u16,

    /// Port on which the network output is published using osc
    #[arg(short, long, default_value_t = OSC_PORT)]
    pub osc_port: u16,
}

impl Default for RunArgs {
    fn default() -> Self {
        RunArgs {
            timestep: 2.0,
            model: "3_8".into(),
            list: false,
            network_port: FEEL_PORT,
            metronome_port: METRONOME_PORT,
            midi_port: MIDI_PORT,
            osc_port: OSC_PORT,
        }
    }
}

#[derive(Args, Debug, Serialize, Deserialize, Default)]
pub struct TrainArgs {
    /// The size of the reservoir
    #[arg(short = 'n', long, default_value_t = 100)]
    pub size: usize,

    /// The amount of iterations
    #[arg(long, default_value_t = 300)]
    pub iter: u64,

    /// The width (# timesteps) of an input
    #[arg(long, default_value_t = 30)]
    pub width: usize,

    /// The width of a target value
    #[arg(long = "tw", default_value_t = 1)]
    pub target_width: usize,

    /// Training step: learning rate
    #[arg(short, long = "rate", default_value_t = 0.05)]
    pub learning_rate: f64,

    /// Neuron leak rate
    #[arg(long = "lr", default_value_t = 0.1)]
    pub leak_rate: f64,

    /// Regularization parameter
    #[arg(short, long = "reg", default_value_t = 1e-2)]
    pub regularization: f64,

    /// Amount of reservoir inputs
    #[arg(short, long, default_value_t = 1)]
    pub inputs: usize,

    /// Amount of reservoir outputs
    #[arg(short, long, default_value_t = 1)]
    pub outputs: usize,

    /// connectivity
    #[arg(short, long, default_value_t = 0.2)]
    pub connectivity: f64,

    /// spectral radius
    #[arg(long = "sr", default_value_t = 0.97)]
    pub spectral_radius: f64,

    /// The amount of ms between evaluations
    #[arg(short, long, default_value_t = 2.0)]
    pub timestep: f64,

    /// Do not prematurely stop if the error increases
    #[arg(long = "nostop", default_value_t = false)]
    pub dont_stop_early: bool,

    /// The name of the train data
    #[arg(short, long, default_value = "default")]
    pub data: String,

    /// List the available data names
    #[arg(long, default_value_t = false)]
    pub list_data: bool,

    /// Split between train and test
    #[arg(long, default_value_t = 0.8)]
    pub split: f64,

    /// Use the grid structure
    #[arg(long, default_value_t = false)]
    pub grid: bool,

    /// Use python-exported data
    #[arg(long)]
    pub npy: Option<String>,

    /// Shift the target data to achieve delay compensation [ms]
    #[arg(long)]
    pub shift: Option<u8>,

    /// The train update that's used for offline training
    #[arg(long, default_value = "inv", value_enum)]
    pub mode: TrainMode,

    /// The activation function to use
    #[arg(long = "act", default_value = "tanh", value_enum)]
    pub activation: Activation,
}

#[derive(ValueEnum, Debug, Clone, Serialize, Deserialize, Default)]
pub enum TrainMode {
    #[default]
    Inv,
    Grad,
}

impl Display for TrainMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrainMode::Inv => write!(f, "pseudo inverse"),
            TrainMode::Grad => write!(f, "MSE gradient descent"),
        }
    }
}

nest! {
    #[derive(Args, Default)]
    #[derive(Debug, Serialize, Deserialize)]*
    pub struct GenerateDataArgs {
        /// The algorithm used to generate rhythmic patterns
        #[command(subcommand)]
        pub algorithm:
            #[derive(Subcommand)]
            pub enum RhythmAlgorithm {
                Euclidean(EucledeanArgs),
                NPDAG(NPDAGArgs),
                PolyEuclidean(PolyEuclideanArgs),
            },

        /// Chebyshev density (amount of zeros per beat, approximately)
        #[arg(short, long)]
        pub density: Option<u8>,

        /// Offset for chebyshev point generation
        #[arg(long, default_value_t = 0.0)]
        pub offset: f64,

        /// The output name to write the data to (saved at $XDG_DATA_HOME/neuroner/traindata/{name}/)
        #[arg(short, long, default_value = "default")]
        pub output: String,

        /// The beats per minute on which the model is trained
        #[arg(short, long, default_value_t = 120.0)]
        pub bpm: f64,

        /// The variance to apply to the input data (is actually std dev)
        #[arg(short, long, default_value_t = 5.0)]
        pub variance: f64,

        /// The amount with which to scale (speed up) the rhythm compared to the base bpm
        #[arg(short, long, default_value_t = 1)]
        pub scale: u8,

        /// The amount of seconds of data to generate
        #[arg(short, long = "dur", default_value_t = 10.0)]
        pub duration_s: f64,

        /// Should the data generate a steady-state input phase (timesteps)
        #[arg(long, default_value_t = 0)]
        pub steady_state: usize,
    }
}

impl Default for RhythmAlgorithm {
    fn default() -> Self {
        RhythmAlgorithm::Euclidean(EucledeanArgs::default())
    }
}

impl Display for RhythmAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RhythmAlgorithm::Euclidean(_) => write!(f, "Euclidean"),
            RhythmAlgorithm::NPDAG(_) => write!(f, "NP-DAG"),
            RhythmAlgorithm::PolyEuclidean(_) => write!(f, "Poly Euclidean"),
        }
    }
}

#[derive(Args, Debug, Serialize, Deserialize)]
pub struct EucledeanArgs {
    /// The amount of pulses in the euclidean rhythm
    #[arg(short, long)]
    pub n: Vec<usize>,

    /// The amount of onsets in the euclidean rhythm
    #[arg(short, long)]
    pub k: Vec<usize>,
}

impl Default for EucledeanArgs {
    fn default() -> Self {
        EucledeanArgs {
            n: vec![8],
            k: vec![3],
        }
    }
}

#[derive(Args, Debug, Serialize, Deserialize)]
pub struct NPDAGArgs {}

#[derive(Args, Debug, Serialize, Deserialize)]
pub struct PolyEuclideanArgs {
    /// The amount of pulses in the euclidean rhythm
    #[arg(short, long, default_value_t = 16)]
    pub n: usize,

    /// The amount of onsets in the euclidean rhythm
    #[arg(short, long, default_value_t = 5)]
    pub k: usize,

    /// The amount of pulses in the euclidean rhythm
    #[arg(short, long, default_value_t = 16)]
    pub n_in: usize,

    /// The amount of onsets in the euclidean rhythm
    #[arg(short, long, default_value_t = 5)]
    pub k_in: usize,

    /// The scaling between the user and the system
    #[arg(short, long, default_value_t = 1)]
    pub scale: u8,
}

#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// The shell for which to generate completions (only zsh works)
    #[arg(short, long, default_value = "zsh", value_enum)]
    pub shell: Shell,
}

nest! {
    #[derive(Args, Debug, Clone)]
    pub struct MidiBrokerArgs {
        /// The port on which to publish midi messages
        #[arg(short, long, default_value_t = MIDI_PORT)]
        pub port: u16,

        /// The mode to operate on
        #[arg(short, long, default_value = "single", value_enum)]
        pub mode:
            #[derive(ValueEnum, Clone, Debug, Default)]
            pub enum BrokerMode {
                #[default]
                Single,
                Chord,
            },

        /// The amount of notes that make up a chord
        #[arg(short, long, default_value_t = 3)]
        pub chord_size: usize,

        /// The channel to filter inputs on
        #[arg(long = "ch")]
        pub channel: Option<u8>,

        /// The device to listen on
        #[arg(long)]
        pub device: Option<String>,
    }
}

impl Default for MidiBrokerArgs {
    fn default() -> Self {
        MidiBrokerArgs {
            port: MIDI_PORT,
            mode: BrokerMode::Single,
            chord_size: 3,
            channel: None,
            device: None,
        }
    }
}

impl MidiBrokerArgs {
    pub fn channel_str(&self) -> String {
        self.channel
            .map(|c| format!("channel {}", c))
            .unwrap_or("all channels".into())
    }

    pub fn mode_str(&self) -> String {
        match self.mode {
            BrokerMode::Single => "single notes".into(),
            BrokerMode::Chord => format!("chords of size {}", self.chord_size),
        }
    }
}

#[derive(Args, Debug)]
pub struct CombinerArgs {
    /// port of the metronome publisher
    #[arg(short, long, default_value_t = METRONOME_PORT)]
    metro_port: u16,

    /// port of the rhythmic feel publisher
    #[arg(short, long, default_value_t = FEEL_PORT)]
    feel_port: u16,

    /// threshold for model output selection
    #[arg(short, long, default_value_t = 0.5)]
    threshold: f32,

    /// Subdivision of metronome beat
    #[arg(short, long, default_value_t = 1)]
    subdivision: u8,

    /// output mode
    #[command(subcommand)]
    output: OutputMode,
}

#[derive(Subcommand, Debug)]
pub enum OutputMode {
    Drum(DrumArgs),
    Arpeggio(ArpeggioArgs),
    CC(CCArgs),
}

impl Display for OutputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputMode::Drum(d) => write!(f, "drum ({})", d.output),
            OutputMode::Arpeggio(_) => write!(f, "arpeggio"),
            OutputMode::CC(_) => write!(f, "cc"),
        }
    }
}

nest! {
    /// Arguments for the drum output mode
    #[derive(Args, Debug, Copy, Clone)]
    pub struct DrumArgs {
        /// Select either MIDI or Robot output
        #[arg(short, long, default_value = "midi", value_enum)]
        pub output:
            #[derive(ValueEnum, Clone, Debug, Copy)]
            pub enum DrumOutput {
                /// Output to midi, without delay compensation
                pub Midi,
                /// Output to audio, used by the robot to
                /// control the motor.
                /// This does include delay compensation.
                pub Robot,
            },
        /// The offset at which the rhythmic activity model was trained
        #[arg(short, long, default_value_t = 0.0)]
        pub offset: f64,

        /// The amount of delay compensation to apply
        #[arg(short, long, default_value_t = 0.0)]
        pub delay: f64,
    }
}

impl Display for DrumOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DrumOutput::Midi => write!(f, "midi"),
            DrumOutput::Robot => write!(f, "robot"),
        }
    }
}

#[derive(Args, Debug, Copy, Clone)]
pub struct CCArgs {
    /// The midi CC value to adjust
    #[arg(short, long, default_value_t = 3)]
    pub cc: u8,

    /// The midi channel to output on
    #[arg(long = "ch", default_value = "1")]
    pub channel: u8,

    /// the width of the adjustment
    #[arg(short, long, default_value = "127")]
    pub width: NonZeroU8,

    /// Offset value around which to evolve the cc output
    #[arg(short, long, default_value_t = 0)]
    pub offset: u8,

    /// The output cannot go below the offset
    #[arg(short, long, default_value_t = false)]
    pub non_negative: bool,
}

#[derive(Args, Debug, Copy, Clone)]
pub struct ArpeggioArgs {
    /// The port on which to listen for midi chords
    #[arg(short, long, default_value_t = MIDI_PORT)]
    pub midi_port: u16,

    /// The midi channel to output on
    #[arg(long = "ch", default_value = "1")]
    pub channel: u8,

    /// Duration of arpeggio notes
    #[arg(short, long, default_value_t = 0.5)]
    pub duration: f32,
}

#[derive(Args, Debug)]
pub struct TuiArgs {}

#[derive(Args, Debug)]
pub struct RobotArgs {
    /// The test to run
    #[command(subcommand)]
    pub command: RobotCommand,
}

#[derive(Subcommand, Debug)]
pub enum RobotCommand {
    Sweep,
    WaveType,
}
