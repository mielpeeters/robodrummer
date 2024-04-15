use std::{thread, time::Duration};

use crate::midier::create_midi_output_and_connect;
use make_csv::{csv_entry, csv_start, csv_stop, python};
use midi_control::MidiMessageSend;
use ndarray::Array1;

use crate::{add_data, reservoir::Reservoir, series::*, trainutil::add_series_data};

pub fn send_beat(conn: &mut midir::MidiOutputConnection, num: u32) {
    let ch = match num {
        1 => midi_control::Channel::Ch1,
        2 => midi_control::Channel::Ch2,
        _ => midi_control::Channel::Ch1,
    };

    conn.send_message(midi_control::note_on(ch, 60, 100))
        .unwrap();
}

pub fn play_model(mut model: Box<Reservoir>) {
    model.reset_state();

    let mut midi_out = create_midi_output_and_connect().unwrap();

    let mut curr = [model.get_output(0), model.get_output(1)];
    // let mut speed = [0.0; 2];

    const PERIOD: usize = 2;
    // const TRANS_MILIS: i32 = 50;

    let mut test_inputs: Vec<Array1<f64>> = Vec::new();

    let zero = constant(0.0);
    let one = constant(1.0);

    add_data!( test_inputs <-  [one, zero]; 1000 * PERIOD);
    add_data!( test_inputs <-  [zero, one]; 1000 * PERIOD);
    add_data!( test_inputs <-  [one, zero]; 1000 * PERIOD);

    let mut wtr = csv_start!("out.csv");
    csv_entry!(wtr <- "t", "out 0", "out 1");

    for (idx, i) in test_inputs.iter().enumerate() {
        model.forward(i);

        csv_entry!(wtr <- idx, model.output[0], model.output[1]);

        let old_loc = curr;
        // let old_speed = speed;

        curr = [model.get_output(0), model.get_output(1)];
        // speed = [curr[0] - old_loc[0], curr[1] - old_loc[1]];

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

    csv_stop!(wtr);
    python!("plot.py");
}
