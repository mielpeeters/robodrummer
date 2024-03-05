/*!
* This module defines the command line interface for the application using clap.
*/

mod gendata;
mod run;
mod train;

use clap::Args;

use clap::{Parser, Subcommand};

pub use gendata::gendata;
pub use run::run;
pub use train::train;

#[derive(Parser, Debug)]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Train(TrainArgs),
    Run(RunArgs),
    GenerateData(GenerateDataArgs),
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

#[derive(Args, Debug)]
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

    /// inputs
    #[arg(short, long, default_value_t = 1)]
    pub inputs: usize,

    /// outputs
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

    /// Do not prematurely if the error increases
    #[arg(long = "nostop", default_value_t = false)]
    pub dont_stop_early: bool,

    /// Amount of beat examples to train from
    #[arg(short, long, default_value_t = 20)]
    pub beat_len: usize,
}

#[derive(Args, Debug)]
pub struct GenerateDataArgs {
    /// The algorithm used to generate rhythmic patterns
    #[command(subcommand)]
    pub algorithm: RhythmAlgorithm,

    /// The output file to write the data to (saved at $XDG_DATA_HOME/neuroner/traindata/)
    #[arg(short, long, default_value = "data.csv")]
    pub output: String,

    /// The amount of ms between evaluations
    #[arg(short, long, default_value_t = 2.0)]
    pub timestep: f64,

    /// The beats per minute on which the model is trained
    #[arg(short, long, default_value_t = 120.0)]
    pub bpm: f64,

    /// The variance to apply to the input data
    #[arg(short, long, default_value_t = 5.0)]
    pub variance: f64,

    /// The amount with which to scale (speed up) the rhythm compared to the base bpm
    #[arg(short, long, default_value_t = 1)]
    pub scale: u8,
}

#[derive(Subcommand, Debug)]
pub enum RhythmAlgorithm {
    Euclidean(EucledeanArgs),
    NPDAG(NPDAGArgs),
}

#[derive(Args, Debug)]
pub struct EucledeanArgs {
    /// The amount of pulses in the euclidean rhythm
    #[arg(short, long, default_value_t = 16)]
    pub n: usize,

    /// The amount of onsets in the euclidean rhythm
    #[arg(short, long, default_value_t = 5)]
    pub k: usize,
}

#[derive(Args, Debug)]
pub struct NPDAGArgs {}
