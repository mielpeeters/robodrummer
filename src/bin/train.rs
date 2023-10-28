extern crate blas_src;
extern crate openblas_src;

use indicatif::{ProgressBar, ProgressStyle};
use ndarray::Array1;
use neuroner::{
    add_data,
    constants::REGULARIZATION,
    csv_entry, csv_start,
    full_network::FullNetwork,
    midier::play_model,
    python,
    series::{constant, linear, say, sine_speed_up, sine_with},
    trainutil::add_series_data,
};

const SIZE: usize = 150;
const ITER: u64 = 200;

fn main() -> Result<(), String> {
    let mut nw = FullNetwork::new()
        .with_size_input_outputs(SIZE, 3, 2, 0.4)
        .with_learning_rate(0.01)
        .with_regularization(REGULARIZATION)
        .build();

    nw.scale(Some(1.00));

    let mut targets: Vec<Array1<f32>> = Vec::new();
    let mut inputs: Vec<Array1<f32>> = Vec::new();

    let data_len = 600;

    let zero = constant(0.0);
    let one = constant(1.0);
    let zero_to_one = linear(2000, 0.0, 1.0);

    let sine_100 = sine_with(100, 8.0, 0.0, 0.0);
    let sine_200 = sine_with(200, 8.0, 0.0, 0.0);

    let sine_100_speed_up = sine_speed_up(100, 8.0, 0.5, 2000);
    let sine_200_speed_up = sine_speed_up(200, 8.0, 0.5, 2000);

    add_data!(targets <- [sine_100, sine_200]; data_len);
    add_data!(inputs  <- [one, zero, zero]; data_len);

    add_data!(targets <- [sine_200, sine_100]; data_len);
    add_data!(inputs  <- [zero, one, zero]; data_len);

    add_data!(targets <- [sine_100, sine_200]; data_len);
    add_data!(inputs  <- [one, zero, zero]; data_len);

    add_data!(targets <- [sine_200, sine_100]; data_len);
    add_data!(inputs  <- [zero, one, zero]; data_len);

    add_data!(targets <- [sine_200_speed_up, sine_100_speed_up]; 2000);
    add_data!(inputs  <- [zero, one, zero_to_one]; 2000);

    let pb = ProgressBar::new(ITER);
    pb.set_style(
        ProgressStyle::with_template(
            "Training...  {bar:40.green/black}  {pos} / {len}  eta: {eta}",
        )
        .unwrap()
        .progress_chars("━━─"),
    );

    let mut errors = Vec::new();

    for _ in 0..ITER {
        let error = nw.train(&inputs, &targets);
        errors.push(error);
        pb.inc(1);
    }

    say("Training is finished.");

    pb.finish();

    // plot target and network output graph
    {
        let mut wtr = csv_start!("out.csv");
        csv_entry!(wtr <- "t", "nw_0", "target_0", "nw_1", "target_1");

        for i in 0..targets.len() {
            nw.forward(&inputs[i]);
            let trgt = &targets[i];
            csv_entry!(wtr <- i, nw.output[0], trgt[0], nw.output[1], trgt[1]);
        }
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
    let one_to_zero = linear(300, 1.0, 0.0);
    let zero_to_one = linear(300, 0.0, 1.0);
    // let one_to_zero_long = linear(1000, 1.0, 0.0);
    let zero_to_one_long = linear(1000, 0.0, 1.0);

    add_data!(test_inputs <- [one, zero, zero]; 1000);
    add_data!(test_inputs <- [zero, one, zero_to_one_long]; 1000);
    add_data!(test_inputs <- [one, zero, one]; 1000);
    add_data!(test_inputs <- [one_to_zero, zero_to_one, one_to_zero]; 300);
    add_data!(test_inputs <- [zero, one, zero]; 2000);
    add_data!(test_inputs <- [one, one, zero]; 2000);

    {
        let mut wtr = csv_start!("out.csv");
        // wtr.write_record(&["t", "nw_0", "nw_1", "input_0", "input_1"])
        csv_entry!(wtr <- "t", "nw_0", "input_0");

        nw.reset_state();

        for i in 0..test_inputs.len() {
            nw.forward(&test_inputs[i]);

            csv_entry!(wtr <- i, nw.output[0], test_inputs[i][0]);
        }
    }
    python!("plot.py");

    play_model(Box::new(nw));

    Ok(())
}
