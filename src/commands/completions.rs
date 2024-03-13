use std::io::BufWriter;

use clap::{Command, CommandFactory};
use clap_complete::{generate, Generator};
use dirs::home_dir;

use super::{Arguments, CompletionsArgs};

fn write_completions<G: Generator>(gen: G, cmd: &mut Command) {
    let mut output = BufWriter::new(Vec::new());
    generate(gen, cmd, cmd.get_name().to_string(), &mut output);
    let bytes = output.into_inner().unwrap();
    let string = String::from_utf8(bytes).unwrap();

    // write to file ~/.zfunc/_neuroner
    let path = home_dir().unwrap().join(".zfunc/_neuroner");
    std::fs::write(path, string).unwrap();
}

pub fn update_completions(args: CompletionsArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Arguments::command();
    write_completions(args.shell, &mut cmd);
    Ok(())
}
