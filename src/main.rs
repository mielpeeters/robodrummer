extern crate blas_src;
extern crate openblas_src;

use clap::Parser;

use robodrummer::commands::broke;
use robodrummer::commands::combine;
use robodrummer::commands::dev;
use robodrummer::commands::gendata;
use robodrummer::commands::metronome;
use robodrummer::commands::run;
use robodrummer::commands::train;
use robodrummer::commands::tui;
use robodrummer::commands::update_completions;
use robodrummer::commands::Arguments;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize the logging system
    env_logger::init();

    let args = Arguments::parse();
    match args.command {
        robodrummer::commands::Command::Train(t) => train(t),
        robodrummer::commands::Command::Run(r) => run(r, None),
        robodrummer::commands::Command::GenerateData(g) => gendata(g),
        robodrummer::commands::Command::Completions(c) => update_completions(c),
        robodrummer::commands::Command::MidiBroker(m) => broke(m, None),
        robodrummer::commands::Command::Combine(c) => combine(c, None, None),
        robodrummer::commands::Command::Tui(t) => tui(t),
        robodrummer::commands::Command::Dev(d) => dev(d),
        robodrummer::commands::Command::Metronome(m) => metronome(m, None),
    }
}
