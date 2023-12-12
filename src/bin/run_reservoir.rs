use std::{
    error::Error,
    fs,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use clap::Parser;
use guier::Gui;
use ndarray::Array1;
use neuroner::{
    oscutil,
    reservoir::{data::neuroner_dir, Reservoir},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    /// The amount of ms between evaluations
    #[arg(short, long, default_value_t = 1)]
    timestep: u32,

    /// The name of the model
    #[arg(short, long, default_value = "model")]
    model: String,

    /// List the available model names
    #[arg(short, long, default_value_t = false)]
    list: bool,

    /// Port on which the network output is published
    #[arg(short, long, default_value_t = 4321)]
    port: u16,
}

fn list_models() -> Result<(), Box<dyn Error>> {
    // get the data dir for this app
    let dir = neuroner_dir()?;

    for (i, path) in (fs::read_dir(dir)?).enumerate() {
        let name = path.unwrap().file_name();
        let name = name.to_str().unwrap().split(".").collect::<Vec<&str>>()[0];
        println!("{i:3}: {name}");
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // list models if that argument was passed
    if args.list {
        return list_models();
    }

    // get parsed arguments
    let timestep = Duration::from_millis(args.timestep.into());
    let model = args.model;
    let port = args.port;

    // open the selected network
    let mut model_path = neuroner_dir().unwrap();
    model_path.push(model + ".bin");
    let mut nw = Reservoir::load_from_file(model_path)?;

    // set up midi input connection
    let last_input = Arc::new(RwLock::new(0));
    let mut last_known_input: u64 = 0;
    let _midi_in = midier::create_midi_input_and_connect(
        |stamp, _msg, input| {
            *input.write().unwrap() = stamp;
        },
        last_input.clone(),
    );

    // set up network output connection
    let context = zmq::Context::new();
    let publisher = context.socket(zmq::PUB)?;
    publisher.bind(&format!("tcp://*:{}", port))?;

    // set up osc output Socket
    let osc_sock = std::net::UdpSocket::bind("0.0.0.0:30000").unwrap();

    let mut gui = Gui::new("Neuroner");
    gui.add_row("output", 0.0);

    // main loop
    loop {
        let start = Instant::now();

        // input zero on non-inputs, 1 on inputs
        let mut input = Array1::zeros(nw.inputs);
        let user_input = *last_input.read().unwrap();
        if last_known_input != user_input {
            input[0] = 1.0;
            last_known_input = user_input;
        }

        // apply one timestep
        nw.forward(&input);

        // wait the remaining time
        let passed_time = Instant::now().duration_since(start);
        if timestep < passed_time {
            continue;
        }
        let remaining = timestep - passed_time;

        // show and publish output
        let output = nw.get_output(0);
        publisher.send(output.to_be_bytes().as_slice(), 0)?;
        oscutil::send_osc_msg("/neuroner", vec![osc::OscType::Float(output)], &osc_sock);

        gui.update_row("output", &output);
        gui.show();

        std::thread::sleep(remaining);
    }

    #[allow(unreachable_code)]
    Ok(())
}
