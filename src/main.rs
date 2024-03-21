extern crate blas_src;
extern crate openblas_src;

use clap::Parser;

use neuroner::commands::combine;
use neuroner::commands::gendata;
use neuroner::commands::run;
use neuroner::commands::train;
use neuroner::commands::update_completions;
use neuroner::commands::Arguments;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize the logging system
    env_logger::init();

    let args = Arguments::parse();
    match args.command {
        neuroner::commands::Command::Train(t) => train(t),
        neuroner::commands::Command::Run(r) => run(r),
        neuroner::commands::Command::GenerateData(g) => gendata(g),
        neuroner::commands::Command::Completions(c) => update_completions(c),
        neuroner::commands::Command::Combine(c) => combine(c),
    }
}
