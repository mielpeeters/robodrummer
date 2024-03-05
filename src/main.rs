use clap::Parser;

use neuroner::commands::gendata;
use neuroner::commands::run;
use neuroner::commands::train;
use neuroner::commands::Arguments;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize the logging system
    env_logger::init();

    let args = Arguments::parse();
    match args.command {
        neuroner::commands::Command::Train(t) => train(t),
        neuroner::commands::Command::Run(r) => run(r),
        neuroner::commands::Command::GenerateData(g) => gendata(g),
    }
}
