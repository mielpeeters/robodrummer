/*!
  This module defines and implements the available activation functions.
*/

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, ValueEnum, Default, Debug)]
pub enum Activation {
    #[default]
    Tanh,
    ReLu,
    Sigmoid,
    Linear,
}

impl Activation {
    pub fn apply(&self, input: f64) -> f64 {
        match self {
            Activation::ReLu => relu(input),
            Activation::Sigmoid => sigmoid(input),
            Activation::Tanh => tanh(input),
            Activation::Linear => input,
        }
    }
}

pub fn relu(input: f64) -> f64 {
    input.max(0.0)
}

pub fn sigmoid(input: f64) -> f64 {
    1.0 / (1.0 + (-input).exp())
}

pub fn tanh(input: f64) -> f64 {
    2.0 / (1.0 + (-2.0 * input).exp()) - 1.0
}
