/*!
  This module provides some functionality for training the network.
*/
use std::{ops::Range, process};

use indicatif::{ProgressBar, ProgressStyle};
use ndarray::Array1;

pub fn say(what_to_say: &str) {
    process::Command::new("spd-say")
        .arg("-y")
        .arg("male5")
        .arg(what_to_say)
        .output()
        .unwrap();
}

pub fn add_series_data(
    container: &mut Vec<Array1<f32>>,
    generators: &[&dyn Fn(i32) -> f32],
    range: Range<usize>,
) {
    for i in range {
        let data_element: Vec<f32> = generators.iter().map(|f| f(i as i32)).collect();
        container.push(Array1::from_vec(data_element));
    }
}

pub fn add_data(container: &mut Vec<Array1<f32>>, data: &[f32]) {
    container.push(Array1::from_iter(data.iter().cloned()));
}

pub fn create_progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::with_template(
            "Training...  {bar:40.green/black}  {pos} / {len}  eta: {eta}",
        )
        .unwrap()
        .progress_chars("━━─"),
    );

    pb
}

#[macro_export]
macro_rules! add_data {
    ($target:ident <- [$($shape: ident),*]; $amount: expr) => {
        add_series_data(
            &mut $target,
            &[$($shape.as_ref()),*],
            0..$amount,
        );
    };
}
