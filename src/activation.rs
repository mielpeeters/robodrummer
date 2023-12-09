/*!
  This module defines and implements the available activation functions.
*/

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub enum Activation {
    ReLu,
    Sigmoid,
    Tanh,
}

impl Activation {
    pub fn apply(&self, input: f32) -> f32 {
        match self {
            Activation::ReLu => relu(input),
            Activation::Sigmoid => sigmoid(input),
            Activation::Tanh => tanh(input),
        }
    }
}

pub fn relu(input: f32) -> f32 {
    input.max(0.0)
}

pub fn sigmoid(input: f32) -> f32 {
    1.0 / (1.0 + (-input).exp())
}

pub fn tanh(input: f32) -> f32 {
    2.0 / (1.0 + (-2.0 * input).exp()) - 1.0
}
