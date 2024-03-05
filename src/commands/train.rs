extern crate blas_src;
extern crate openblas_src;

use std::error::Error;

use crate::{
    add_data,
    reservoir::Reservoir,
    series,
    trainutil::{add_data, add_series_data, create_progress_bar},
};
use make_csv::{csv_entry, csv_start, python};
use ndarray::{Array1, Array2};
use text_io::read;

pub fn train(args: super::TrainArgs) -> Result<(), Box<dyn Error>> {
    log::info!("Training with {:#?}", args);

    let mut nw = Reservoir::from_args(&args);

    let mut targets: Vec<Array1<f64>> = Vec::new();
    let mut target_indexes: Vec<usize> = Vec::new();
    let mut inputs: Vec<Array1<f64>> = Vec::new();

    let beat = series::impulse_width_pause(1.0, 30);
    let beat_low = series::impulse_width_pause(0.2, 15);
    let zero = series::constant(0.0);

    // one beat per 500ms
    // args.timestep is given in miliseconds
    let data_len = (500.0 / args.timestep) as usize;
    let beat_len = args.beat_len;
    // add beats
    for _ in 0..beat_len {
        add_data!(inputs  <- [beat]; data_len);
    }

    // target data: more dynamic swing beat
    for i in 0..beat_len {
        add_data(&mut targets, &[1.0]);
        target_indexes.push(i * data_len);
        add_data(&mut targets, &[0.0]);
        target_indexes.push(i * data_len + data_len * 2 / 8);
        add_data(&mut targets, &[1.0]);
        target_indexes.push(i * data_len + data_len * 5 / 8);
        add_data(&mut targets, &[0.0]);
        target_indexes.push(i * data_len + data_len * 6 / 8);
        // add_data(&mut targets, &[1.0]);
        // target_indexes.push(i * data_len + data_len * 7 / 8);
    }

    let pb = create_progress_bar(args.iter);

    let mut errors = Vec::with_capacity(args.iter as usize);

    // keep history of output weights to jump back to a previous better version
    // to get rid of the weird training behaviour (which will need to be investigated further)
    let mut weight_history: Array2<f64>;
    let mut best_weights = nw.weights_res_out.clone();
    let mut lowest_error = std::f64::MAX;
    let mut last_error = 0.0;

    for i in 0..args.iter {
        weight_history = nw.weights_res_out.clone();
        let error = nw.train_step(&inputs, &targets, Some(&target_indexes));
        errors.push(error);
        pb.inc(1);

        if args.dont_stop_early {
            continue;
        }

        if error < lowest_error {
            best_weights = weight_history.clone();
            lowest_error = error;
        }

        let diff = (last_error - error).abs();
        if diff < 1e-4 {
            log::info!("Stopping early at iteration {}", i);
            break;
        }

        last_error = error;
    }

    pb.finish();
    nw.set_weights_out(best_weights);

    let weights_sum = nw
        .weights_res_out
        .iter()
        .map(|x| x.abs())
        .fold(0.0, |acc, x| acc + x);

    log::info!("Sum of output weights: {}", weights_sum);

    // say("Training is finished.");

    // plot target and network output graph
    {
        let mut wtr = csv_start!("out.csv");
        csv_entry!(wtr <- "t", "nw_0", "target_0", "input_0");

        let mut cnt = 0;
        (0..inputs.len()).for_each(|i| {
            nw.forward(&inputs[i]);
            if target_indexes.contains(&i) {
                csv_entry!(wtr <- i, nw.output[0], targets[cnt][0], inputs[i][0]);
                cnt += 1;
            } else {
                csv_entry!(wtr <- i, nw.output[0], "", inputs[i][0]);
            }
        });
    }
    python!("plot.py");

    // plot error graph
    {
        let mut wtr = csv_start!("out.csv");
        csv_entry!(wtr <- "t", "error");

        for (i, error) in errors.iter().enumerate() {
            csv_entry!(wtr <- i, error);
        }
    }
    python!("plot.py");

    let mut test_inputs: Vec<Array1<f64>> = Vec::new();

    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [beat_low]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat_low]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat_low]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat_low]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [zero]; data_len);

    {
        let mut wtr = csv_start!("out.csv");
        csv_entry!(wtr <- "t", "nw_0", "input_0");
        // csv_entry!(wtr <- "t", "nw_0", "input_0");

        nw.reset_state();

        for (i, input) in test_inputs.iter().enumerate() {
            nw.forward(input);

            csv_entry!(wtr <- i, nw.output[0], input[0]);
            // csv_entry!(wtr <- i, nw.output[0], test_inputs[i][0]);
        }
    }
    python!("plot.py");

    print!("Save this model? [filename]: ");
    let answer: String = read!();
    if answer.is_empty() {
        return Ok(());
    }

    nw.reset_state();
    nw.save(&answer)?;

    Ok(())
}
