use std::error::Error;

use crate::{
    commands::TrainMode,
    data::{list_data, load_train_data, models_dir},
    reservoir::Reservoir,
    trainutil::create_progress_bar,
};
use make_csv::{csv_entry, csv_start, python};
use ndarray::{Array1, Array2};
use text_io::try_read;

fn save_trained_model(
    nw: &mut Reservoir,
    name: &str,
    args: &super::TrainArgs,
) -> Result<(), Box<dyn Error>> {
    println!("Saving model as {}", name);
    nw.reset_state();
    nw.save(name)?;
    let meta_path = models_dir()?.join(format!("{}.toml", name));
    let metadata = toml::to_string(&args)?;
    std::fs::write(meta_path, metadata)?;

    Ok(())
}

fn analyze(
    train_inputs: &[Array1<f64>],
    test_inputs: &[Array1<f64>],
    targets: &[Option<Array1<f64>>],
    errors: &[f64],
    nw: &mut Reservoir,
    args: &super::TrainArgs,
) {
    {
        // plot target and network output graph
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

    {
        // plot error graph
        let mut wtr = csv_start!("out.csv");
        csv_entry!(wtr <- "t", "error");

        for (i, error) in errors.iter().enumerate() {
            csv_entry!(wtr <- i, error);
        }
    }
    python!("plot.py");

    {
        // plot test graph
        let mut wtr = csv_start!("out.csv");
        let mut int_wtr = csv_start!("int_states.csv");

        csv_entry!(wtr <- "t", "nw_0", "input_0");
        if args.npy.is_some() {
            csv_entry!(int_wtr <- "t", "state_0", "state_1", "state_2", "state_3");
        }

        nw.reset_state();

        for (i, input) in test_inputs.iter().enumerate() {
            nw.forward(input);

            csv_entry!(wtr <- i, nw.output[0], input[0]);
            let states = nw.get_visible_state();
            if args.npy.is_some() {
                csv_entry!(int_wtr <- i, states[0], states[10], states[20], states[35]);
            }
        }

        // also add some zeros to see the steady state behaviour
        for i in 0..1000 {
            let input: Array1<f64> = Array1::zeros(args.inputs);
            nw.forward(&input);
            csv_entry!(wtr <- i + test_inputs.len(), nw.output[0], input[0]);
            let states = nw.get_visible_state();
            if args.npy.is_some() {
                csv_entry!(int_wtr <- i, states[0], states[10], states[20], states[35]);
            }
        }
    }
    python!("plot.py");
    if args.npy.is_some() {
        python!("plot.py", "int_states.csv");
    }
}

pub fn train(args: super::TrainArgs) -> Result<(), Box<dyn Error>> {
    if args.list_data {
        list_data()?;
        return Ok(());
    }

    log::info!("Training arguments: {:#?}", args);

    let mut nw = Reservoir::from_args(&args);

    nw.generate_sparse();

    // TODO: get the input data from a file
    let shift = match args.shift {
        Some(shift) => Some((shift as f64 / args.timestep).round() as usize),
        None => None,
    };

    let (inputs, targets) = load_train_data(
        &args.data,
        args.timestep,
        args.width,
        args.target_width,
        shift,
    )?;

    // get data and perform splits
    let train_len = (inputs.len() as f64 * args.split) as usize;
    let train_inputs = &inputs[0..train_len];
    let test_inputs = &inputs[(inputs.len() as f64 * args.split) as usize..];
    let targets = &targets[0..train_len];

    let pb = create_progress_bar(args.iter);

    let mut errors = Vec::with_capacity(args.iter as usize);

    // keep history of output weights to jump back to a previous better version
    // to get rid of the weird training behaviour (which will need to be investigated further)
    let mut weight_history: Array2<f64> = Array2::zeros(nw.weights_res_out.dim());
    let mut best_weights = nw.weights_res_out.clone();
    let mut lowest_error = std::f64::MAX;
    let mut last_error = 0.0;

    for i in 0..args.iter {
        // save the history before any adjustments
        weight_history.assign(&nw.weights_res_out);

        let error = match args.mode {
            TrainMode::Inv => nw.train_step(train_inputs, targets),
            TrainMode::Grad => nw.train_mse_grad(train_inputs, targets),
        };

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

    analyze(train_inputs, test_inputs, targets, &errors, &mut nw, &args);

    print!("Save this model? [filename]: ");
    let answer: Result<String, _> = try_read!();
    let Ok(name) = answer else {
        return Ok(());
    };

    // nw.generate_sparse();
    save_trained_model(&mut nw, &name, &args)?;

    Ok(())
}
