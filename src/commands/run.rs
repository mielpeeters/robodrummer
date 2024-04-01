use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::guier::Gui;
use crate::{data::list_models, oscutil, reservoir::Reservoir};
use ndarray::Array1;

/// run the selected model, determined by the parameters in args.
pub fn run(args: super::RunArgs) -> Result<(), Box<dyn Error>> {
    // list models if that argument was passed
    if args.list {
        return list_models();
    }

    // get parsed arguments
    let model = args.model;
    let zmq_port_pub = args.network_port;
    let zmq_port_sub = args.metronome_port;

    // open the selected network
    let mut nw = Reservoir::load_from_name(&model)?;

    // generate the sparse representation for efficient multiplication
    nw.generate_sparse();

    // set up network output connection
    let context = zmq::Context::new();
    let publisher = context.socket(zmq::PUB)?;
    publisher.bind(&format!("tcp://*:{}", zmq_port_pub))?;

    // listen for metronome value
    let subscriber = context.socket(zmq::SUB)?;
    subscriber.connect(&format!("tcp://localhost:{}", zmq_port_sub))?;
    subscriber.set_subscribe(b"")?;

    let metronome: Arc<Mutex<f64>> = Arc::new(Mutex::new(2.0));
    let mt = Arc::clone(&metronome);
    let _handle = std::thread::spawn(move || loop {
        let msg = subscriber.recv_bytes(0).unwrap();
        let value = f64::from_be_bytes(msg.try_into().unwrap());
        *mt.lock().unwrap() = value;
    });

    // listen for midi inputs
    let midi_in = context.socket(zmq::SUB)?;
    midi_in.connect(&format!("tcp://localhost:{}", args.midi_port))?;
    midi_in.set_subscribe(b"")?;

    // set up osc output Socket
    let osc_sock = oscutil::create_socket(args.osc_port);

    let mut gui = Gui::new("Neuroner");
    let mut output = 0.0;
    gui.add_row("output", output);
    gui.show();

    let mut input_steps_remaining = 0;

    // main loop
    loop {
        let start = Instant::now();

        // input zero on non-inputs, 1 on inputs
        let mut input = Array1::zeros(nw.inputs);
        if midi_in.recv_bytes(zmq::DONTWAIT).is_ok() {
            // HACK: this parameter is controlled by the training input pulse width...
            input_steps_remaining = 20;
        }

        if input_steps_remaining > 0 {
            input_steps_remaining -= 1;
            input[0] = 1.0;
        }

        // apply one timestep
        nw.forward(&input);

        let adjusted_timestep = {
            let m = metronome.lock().unwrap();
            let a = args.timestep * 2.0 / *m;

            Duration::from_millis(a as u64)
        };

        // show and publish output
        let new_output = nw.get_output(0);
        publisher.send((new_output as f32).to_be_bytes().as_slice(), 0)?;
        oscutil::send_osc_msg(
            "/neuroner",
            vec![osc::OscType::Float(new_output as f32)],
            &osc_sock,
        );

        if new_output != output {
            output = new_output;
            gui.update_row("output", &output);
            gui.show();
        }

        log::debug!("Adjusted timestep: {:?}", adjusted_timestep);

        // wait the remaining time
        let passed_time = Instant::now().duration_since(start);
        if adjusted_timestep < passed_time {
            continue;
        }

        let remaining = adjusted_timestep - passed_time;
        log::debug!("Remaining: {:?}", remaining);

        std::thread::sleep(remaining);
    }
}
