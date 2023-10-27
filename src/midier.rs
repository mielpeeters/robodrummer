use std::{thread, time::Duration};

use midi_control::MidiMessageSend;
use ndarray::Array1;

use crate::{errors, full_network::FullNetwork, series::*, trainutil::add_series_data};

pub fn create_midi_output_and_connect() -> Result<midir::MidiOutputConnection, errors::NeuronError>
{
    let Ok(output) = midir::MidiOutput::new("ESN_Midi") else {
        return Err(errors::NeuronError::CantCreateMidiOut);
    };

    println!("Available Midi ports to connect to:");
    let ports = output.ports();
    for (i, port) in ports.iter().enumerate() {
        println!("{i}:   {}", output.port_name(&port).unwrap());
    }

    println!("\nSelect one: ");

    let num: usize = text_io::read!();
    let name = output.port_name(ports.get(num).unwrap()).unwrap();

    let Ok(conn) = output.connect(ports.get(num).unwrap(), "ESN_Midi_conn") else {
        return Err(errors::NeuronError::CantConnectMidi);
    };

    println!("Connected to Midi device \x1b[1m{}\x1b[0m!", name);

    Ok(conn)
}

pub fn send_beat(conn: &mut midir::MidiOutputConnection, num: u32) {
    let ch = match num {
        1 => midi_control::Channel::Ch1,
        2 => midi_control::Channel::Ch2,
        _ => midi_control::Channel::Ch1,
    };

    conn.send_message(midi_control::note_on(ch, 60, 100))
        .unwrap();
}

pub fn play_model(mut model: Box<FullNetwork>) {
    model.reset_state();

    let mut midi_out = create_midi_output_and_connect().unwrap();

    let mut curr = [model.get_output(0), model.get_output(1)];
    let mut speed = [0.0; 2];

    const PERIOD: i32 = 2;
    const TRANS_MILIS: i32 = 50;

    let mut test_inputs: Vec<Array1<f32>> = Vec::new();

    let zero = constant(0.0);
    let one = constant(1.0);
    let one_to_zero = linear(300, 1.0, 0.0);
    let zero_to_one = linear(300, 0.0, 1.0);

    add_series_data(
        &mut test_inputs,
        &[one.as_ref(), zero.as_ref()],
        0..1000 * PERIOD,
    );
    add_series_data(
        &mut test_inputs,
        &[one_to_zero.as_ref(), zero_to_one.as_ref()],
        0..TRANS_MILIS,
    );
    add_series_data(
        &mut test_inputs,
        &[zero.as_ref(), one.as_ref()],
        0..1000 * PERIOD,
    );
    add_series_data(
        &mut test_inputs,
        &[zero_to_one.as_ref(), one_to_zero.as_ref()],
        0..TRANS_MILIS,
    );
    add_series_data(
        &mut test_inputs,
        &[one.as_ref(), zero.as_ref()],
        0..1000 * PERIOD,
    );

    for i in test_inputs {
        model.forward(&i);

        let old_loc = curr;
        let old_speed = speed;

        curr = [model.get_output(0), model.get_output(1)];
        speed = [curr[0] - old_loc[0], curr[1] - old_loc[1]];

        if old_loc[0] <= 0.0 && curr[0] > 0.0 {
            send_beat(&mut midi_out, 1);
        }

        if old_loc[1] <= 0.0 && curr[1] > 0.0 {
            send_beat(&mut midi_out, 2);
        }

        // if old_speed[0] <= 0.0 && speed[0] > 0.0 {
        //     send_beat(&mut midi_out, 1);
        // }

        // if old_speed[1] <= 0.0 && speed[1] > 0.0 {
        //     send_beat(&mut midi_out, 2);
        // }

        thread::sleep(Duration::from_millis(4));
    }
}
