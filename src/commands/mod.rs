pub mod run;
pub mod train;

use clap::Args;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Train(TrainArgs),
    Run(RunArgs),
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// The amount of ms between evaluations
    #[arg(short, long, default_value_t = 2)]
    pub timestep: u32,

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
    pub learning_rate: f32,

    /// Neuron leak rate
    #[arg(long = "lr", default_value_t = 0.1)]
    pub leak_rate: f32,

    /// Regularization parameter
    #[arg(short, long = "reg", default_value_t = 1e-0)]
    pub regularization: f32,

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
    pub spectral_radius: f32,

    /// The amount of ms between evaluations
    #[arg(short, long, default_value_t = 2.0)]
    pub timestep: f32,

    /// Do not prematurely if the error increases
    #[arg(long = "nostop", default_value_t = false)]
    pub dont_stop_early: bool,
}
