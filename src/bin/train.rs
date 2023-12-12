extern crate blas_src;
extern crate openblas_src;

use std::error::Error;

use make_csv::{csv_entry, csv_start, python};
use ndarray::Array1;
use neuroner::{
    add_data,
    reservoir::Reservoir,
    series::{self, say},
    trainutil::{add_series_data, create_progress_bar},
};
use text_io::read;

const SIZE: usize = 30;
const ITER: u64 = 100;

fn main() -> Result<(), Box<dyn Error>> {
    let mut nw = Reservoir::new_builder()
        .with_size_input_outputs(SIZE, 1, 1, 0.4)
        .with_learning_rate(0.01)
        .with_damping_coef(0.95)
        .build();

    nw.scale(None);

    let mut targets: Vec<Array1<f32>> = Vec::new();
    let mut inputs: Vec<Array1<f32>> = Vec::new();

    let zero = series::constant(0.0);
    let one = series::constant(1.0);

    let beat = series::impulse_pause(1.0);

    let data_len = 200;
    add_data!(inputs  <- [beat]; data_len);
    add_data!(targets <- [zero]; data_len);
    add_data!(inputs  <- [beat]; data_len);
    add_data!(targets <- [zero]; data_len);

    add_data!(inputs  <- [zero]; data_len);
    add_data!(targets <- [one]; data_len);
    add_data!(inputs  <- [zero]; data_len);
    add_data!(targets <- [one]; data_len);

    add_data!(inputs  <- [beat]; data_len);
    add_data!(targets <- [zero]; data_len);
    add_data!(inputs  <- [beat]; data_len);
    add_data!(targets <- [zero]; data_len);

    add_data!(inputs  <- [zero]; data_len);
    add_data!(targets <- [one]; data_len);
    add_data!(inputs  <- [zero]; data_len);
    add_data!(targets <- [one]; data_len);

    add_data!(inputs  <- [beat]; data_len);
    add_data!(targets <- [zero]; data_len);
    add_data!(inputs  <- [beat]; data_len);
    add_data!(targets <- [zero]; data_len);
    add_data!(inputs  <- [beat]; data_len);
    add_data!(targets <- [zero]; data_len);

    add_data!(inputs  <- [zero]; data_len);
    add_data!(targets <- [one]; data_len);
    add_data!(inputs  <- [zero]; data_len);
    add_data!(targets <- [one]; data_len);

    add_data!(inputs  <- [beat]; data_len);
    add_data!(targets <- [zero]; data_len);
    add_data!(inputs  <- [beat]; data_len);
    add_data!(targets <- [zero]; data_len);
    add_data!(inputs  <- [beat]; data_len);
    add_data!(targets <- [zero]; data_len);

    add_data!(inputs  <- [zero]; data_len);
    add_data!(targets <- [one]; data_len);
    add_data!(inputs  <- [zero]; data_len);
    add_data!(targets <- [one]; data_len);

    let pb = create_progress_bar(ITER);

    let mut errors = Vec::new();

    for _ in 0..ITER {
        let error = nw.train_step(&inputs, &targets);
        errors.push(error);
        pb.inc(1);
    }

    say("Training is finished.");

    pb.finish();

    // plot target and network output graph
    {
        let mut wtr = csv_start!("out.csv");
        csv_entry!(wtr <- "t", "nw_0", "target_0", "input_0");

        for i in 0..targets.len() {
            nw.forward(&inputs[i]);
            let trgt = &targets[i];
            csv_entry!(wtr <- i, nw.output[0], trgt[0], inputs[i][0]);
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

    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [zero]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
    add_data!(test_inputs <- [beat]; data_len);
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
