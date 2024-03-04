use clap::Parser;
use neuroner::commands::run::run;
use neuroner::commands::train::train;
use neuroner::commands::Arguments;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize the logging system
    env_logger::init();

    let args = Arguments::parse();
    match args.command {
        neuroner::commands::Command::Train(t) => train(t),
        neuroner::commands::Command::Run(r) => run(r),
    }
}
