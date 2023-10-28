/*!
  This module defines and implements the available activation functions.
*/

pub fn relu(input: f32) -> f32 {
    input.max(0.0)
}

pub fn sigmoid(input: f32) -> f32 {
    1.0 / (1.0 + (-input).exp())
}

pub fn tanh(input: f32) -> f32 {
    2.0 / (1.0 + (-2.0 * input).exp()) - 1.0
}
