use indicatif::{ProgressBar, ProgressStyle};
use ndarray::Array1;
use neuroner::{
    constants::REGULARIZATION,
    full_network::FullNetwork,
    series::{constant, linear, saw_with, sine_with},
    trainutil::add_series_data,
};

const SIZE: usize = 150;
const ITER: u64 = 100;

fn main() -> Result<(), String> {
    let mut nw = FullNetwork::new()
        .with_size_input_outputs(SIZE, 2, 1, 0.5)
        .with_learning_rate(0.01)
        .with_regularization(REGULARIZATION)
        .build();

    nw.scale(Some(1.00));

    let mut counter = 0;

    let mut targets: Vec<Array1<f32>> = Vec::new();
    let mut inputs: Vec<Array1<f32>> = Vec::new();

    let data_len = 600;

    let zero = constant(0.0);
    let one = constant(1.0);

    let sine_100 = sine_with(100, 8.0, 0.0, 0.0);
    let sine_200 = sine_with(200, 8.0, 0.0, 0.0);

    add_series_data(&mut targets, &[sine_100.as_ref()], 0..data_len);
    add_series_data(&mut inputs, &[one.as_ref(), zero.as_ref()], 0..data_len);

    add_series_data(&mut targets, &[sine_200.as_ref()], 0..data_len);
    add_series_data(&mut inputs, &[zero.as_ref(), one.as_ref()], 0..data_len);

    add_series_data(&mut targets, &[sine_100.as_ref()], 0..data_len);
    add_series_data(&mut inputs, &[one.as_ref(), zero.as_ref()], 0..data_len);

    add_series_data(&mut targets, &[sine_200.as_ref()], 0..data_len);
    add_series_data(&mut inputs, &[zero.as_ref(), one.as_ref()], 0..data_len);

    let pb = ProgressBar::new(ITER);
    pb.set_style(
        ProgressStyle::with_template(
            "Training...  {bar:40.green/black}  {pos} / {len}  eta: {eta}",
        )
        .unwrap()
        .progress_chars("━━─"),
    );

    for i in 0..ITER {
        nw.train(&inputs, &targets);
        pb.inc(1);

        if i == ITER - 1 {
            let mut wtr = csv::Writer::from_path("out.csv").unwrap();
            wtr.write_record(&["t", "nw_0", "target_0"]).unwrap();

            for i in 0..targets.len() {
                let trgt = &targets[i];
                counter += 1;
                nw.forward(&inputs[i]);
                wtr.write_record(&[
                    format!("{}", counter).as_str(),
                    format!("{}", nw.output[0]).as_str(),
                    format!("{}", trgt[0]).as_str(),
                ])
                .unwrap();
            }

            counter = 0;

            wtr.flush().unwrap();
            drop(wtr);

            std::process::Command::new("python3")
                .arg("plot.py")
                .output()
                .unwrap();
        }
    }

    pb.finish();

    let mut test_inputs: Vec<Array1<f32>> = Vec::new();

    add_series_data(&mut test_inputs, &[one.as_ref(), zero.as_ref()], 0..1000);
    add_series_data(&mut test_inputs, &[zero.as_ref(), one.as_ref()], 0..1000);
    add_series_data(&mut test_inputs, &[one.as_ref(), zero.as_ref()], 0..1000);
    let one_to_zero = linear(100, 1.0, 0.0);
    let zero_to_one = linear(100, 0.0, 1.0);
    add_series_data(
        &mut test_inputs,
        &[one_to_zero.as_ref(), zero_to_one.as_ref()],
        0..100,
    );
    add_series_data(&mut test_inputs, &[zero.as_ref(), one.as_ref()], 0..2000);
    add_series_data(&mut test_inputs, &[one.as_ref(), one.as_ref()], 0..2000);

    let mut wtr = csv::Writer::from_path("out.csv").unwrap();
    wtr.write_record(&["t", "nw_0", "input_0", "input_1"])
        .unwrap();

    nw.reset();

    counter = 0;

    for i in 0..test_inputs.len() {
        counter += 1;
        nw.forward(&test_inputs[i]);
        wtr.write_record(&[
            format!("{}", counter).as_str(),
            format!("{}", nw.output[0]).as_str(),
            format!("{}", test_inputs[i][0]).as_str(),
            format!("{}", test_inputs[i][1]).as_str(),
        ])
        .unwrap();
    }

    wtr.flush().unwrap();
    drop(wtr);

    std::process::Command::new("python3")
        .arg("plot.py")
        .output()
        .unwrap();

    Ok(())
}
