use std::error::Error;

use crate::{
    data::{list_data, load_train_data, models_dir},
    reservoir::Reservoir,
    trainutil::create_progress_bar,
};
use make_csv::{csv_entry, csv_start, python};
use ndarray::{Array1, Array2};
use text_io::read;

pub fn train(args: super::TrainArgs) -> Result<(), Box<dyn Error>> {
    if args.list_data {
        list_data()?;
        return Ok(());
    }

    log::info!("Training with {:#?}", args);

    let mut nw = Reservoir::from_args(&args);

    // TODO: get the input data from a file
    let data = load_train_data(&args.data)?;

    // get data and perform splits
    let inputs = data.0;
    let train_len = (inputs.len() as f64 * args.split) as usize;
    let train_inputs = &inputs[0..train_len as usize];
    let test_inputs = &inputs[(inputs.len() as f64 * args.split) as usize..];
    let targets = &data.1[0..train_len as usize];

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
        let error = nw.train_step(train_inputs, targets);
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

        (0..train_inputs.len()).for_each(|i| {
            nw.forward(&train_inputs[i]);
            match &targets[i] {
                Some(t) => {
                    csv_entry!(wtr <- i, nw.output[0], t[0], train_inputs[i][0]);
                }
                None => {
                    csv_entry!(wtr <- i, nw.output[0], "", train_inputs[i][0]);
                }
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

    // plot test graph
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

        // also add some zeros to see the steady state behaviour
        for i in 0..1000 {
            let input: Array1<f64> = Array1::zeros(args.inputs);
            nw.forward(&input);
            csv_entry!(wtr <- i + test_inputs.len(), nw.output[0], input[0]);
        }
    }
    python!("plot.py");

    print!("Save this model? [filename]: ");
    let answer: String = read!();
    if answer.is_empty() {
        return Ok(());
    }

    // nw.generate_sparse();
    nw.reset_state();
    nw.save(&answer)?;
    let meta_path = models_dir()?.join(format!("{}.toml", answer));
    let metadata = toml::to_string(&args)?;
    std::fs::write(meta_path, metadata)?;

    Ok(())
}
