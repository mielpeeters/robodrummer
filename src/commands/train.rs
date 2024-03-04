extern crate blas_src;
extern crate openblas_src;

use std::error::Error;

use crate::{
    add_data,
    reservoir::Reservoir,
    series,
    trainutil::{add_data, add_series_data, create_progress_bar, say},
};
use make_csv::{csv_entry, csv_start, python};
use ndarray::Array1;
use text_io::read;

pub fn train(args: super::TrainArgs) -> Result<(), Box<dyn Error>> {
    log::info!("Training with {:#?}", args);

    let mut nw = Reservoir::from_args(&args);

    let mut targets: Vec<Array1<f32>> = Vec::new();
    let mut target_indexes: Vec<usize> = Vec::new();
    let mut inputs: Vec<Array1<f32>> = Vec::new();

    let beat = series::impulse_width_pause(1.0, 30);
    let zero = series::constant(0.0);

    // one beat per 500ms
    // args.timestep is given in miliseconds
    let data_len = (500.0 / args.timestep) as usize;
    // add beats
    for _ in 0..20 {
        add_data!(inputs  <- [beat]; data_len);
    }

    // target data: twice every beat (and go down two times too)
    for i in 0..20 {
        add_data(&mut targets, &[1.0]);
        target_indexes.push(i * data_len);
        add_data(&mut targets, &[0.0]);
        target_indexes.push(i * data_len + data_len / 8);
        add_data(&mut targets, &[1.0]);
        target_indexes.push(i * data_len + 2 * data_len / 4);
        add_data(&mut targets, &[0.0]);
        target_indexes.push(i * data_len + 3 * data_len / 4);
    }

    let pb = create_progress_bar(args.iter);

    let mut errors = Vec::with_capacity(args.iter as usize);

    // keep history of output weights to jump back to a previous better version
    // to get rid of the weird training behaviour (which will need to be investigated further)
    let mut weights_history = [nw.weights_res_out.clone(), nw.weights_res_out.clone()];

    for i in 0..args.iter {
        weights_history[(i % 2) as usize] = nw.weights_res_out.clone();
        let error = nw.train_step(&inputs, &targets, Some(&target_indexes));
        errors.push(error);
        pb.inc(1);

        if args.dont_stop_early {
            continue;
        }

        if errors.len() >= 2 && errors.last().unwrap() > errors.get(errors.len() - 2).unwrap() {
            nw.set_weights_out(weights_history[(1 - (i % 2)) as usize].clone());
            println!("Premature finish");
            break;
        }
    }

    say("Training is finished.");

    pb.finish();

    // plot target and network output graph
    {
        let mut wtr = csv_start!("out.csv");
        csv_entry!(wtr <- "t", "nw_0", "target_0", "input_0");

        let mut cnt = 0;
        (0..targets.len()).for_each(|i| {
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

    let mut test_inputs: Vec<Array1<f32>> = Vec::new();

    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
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
