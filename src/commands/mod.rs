/*!
* This module defines the command line interface for the application using clap.
*/

mod completions;
mod gendata;
mod run;
mod train;

use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;
use serde::{Deserialize, Serialize};

pub use completions::update_completions;
pub use gendata::gendata;
pub use run::run;
pub use train::train;

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
}

#[derive(Args, Debug)]
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
    #[arg(short, long, default_value_t = 4321)]
    pub zmq_port_pub: u16,

    // Port on which to listen for metronome value
    #[arg(short, long, default_value_t = 5432)]
    pub zmq_port_sub: u16,

    /// Port on which the network output is published using osc
    #[arg(short, long, default_value_t = 30000)]
    pub osc_port: u16,
}

#[derive(Args, Debug, Serialize, Deserialize, Default)]
pub struct TrainArgs {
    /// The size of the reservoir
    #[arg(short = 'n', long, default_value_t = 100)]
    pub size: usize,

    /// The amount of iterations
    #[arg(long, default_value_t = 300)]
    pub iter: u64,

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
}

#[derive(Args, Debug, Serialize, Deserialize)]
pub struct GenerateDataArgs {
    /// The algorithm used to generate rhythmic patterns
    #[command(subcommand)]
    pub algorithm: RhythmAlgorithm,

    /// The output name to write the data to (saved at $XDG_DATA_HOME/neuroner/traindata/{name}/)
    #[arg(short, long, default_value = "default")]
    pub output: String,

    /// The amount of ms between evaluations
    #[arg(short, long, default_value_t = 2.0)]
    pub timestep: f64,

    /// The beats per minute on which the model is trained
    #[arg(short, long, default_value_t = 120.0)]
    pub bpm: f64,

    /// The variance to apply to the input data (is actually std dev)
    #[arg(short, long, default_value_t = 5.0)]
    pub variance: f64,

    /// The amount with which to scale (speed up) the rhythm compared to the base bpm
    #[arg(short, long, default_value_t = 1)]
    pub scale: u8,

    /// The width of the input pulses
    #[arg(short, long, default_value_t = 20)]
    pub width: usize,

    /// The amount of seconds of data to generate
    #[arg(short, long = "dur", default_value_t = 10.0)]
    pub duration_s: f64,
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
pub enum RhythmAlgorithm {
    Euclidean(EucledeanArgs),
    NPDAG(NPDAGArgs),
    PolyEuclidean(PolyEuclideanArgs),
}

#[derive(Args, Debug, Serialize, Deserialize)]
pub struct EucledeanArgs {
    /// The amount of pulses in the euclidean rhythm
    #[arg(short, long, default_value_t = 16)]
    pub n: usize,

    /// The amount of onsets in the euclidean rhythm
    #[arg(short, long, default_value_t = 5)]
    pub k: usize,
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
