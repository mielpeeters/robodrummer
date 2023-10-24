#![allow(unused_imports)]
use std::{io, thread, time::Duration};

use ndarray::Array1;
use neuroner::full_network::FullNetwork;

const SIZE: usize = 300;

fn main() -> Result<(), String> {
    let mut nw = FullNetwork::new()
        .with_size_input_outputs(SIZE, 1, 1, 0.35)
        .build();

    nw.scale(Some(2.00));

    let mut wtr = csv::Writer::from_path("out.csv").unwrap();

    let mut counter = 0;

    let mut input: f32 = 0.0;

    loop {
        for _ in 0..2000 {
            // println!("{nw}");
            nw.forward(&Array1::from_vec(vec![input]));
            // println!("OUTPUT ({counter}) --> {}", nw.output[0]);
            counter += 1;
            wtr.write_record(&[
                format!("{}", counter).as_str(),
                format!("{}", nw.output[0]).as_str(),
            ])
            .unwrap();
        }

        println!("Input: ");
        let mut answer = String::new();
        io::stdin().read_line(&mut answer).unwrap();
        let answer: Vec<&str> = answer.split('\n').collect();
        input = answer[0].parse().unwrap();

        if input == 123.0 {
            break;
        }

        // wtr.write_record(&["INPUT CHANGE", &answer[0]]).unwrap();
    }

    wtr.flush().unwrap();

    Ok(())
}
