use std::ops::Range;

use ndarray::Array1;

pub fn add_series_data(
    container: &mut Vec<Array1<f32>>,
    generators: &[&dyn Fn(i32) -> f32],
    range: Range<i32>,
) {
    for i in range {
        let data_element: Vec<f32> = generators.iter().map(|f| f(i)).collect();
        container.push(Array1::from_vec(data_element));
    }
}
