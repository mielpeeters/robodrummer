#[derive(Default, Clone)]
pub enum Activation {
    #[default]
    /// ReLu function (----/)
    ReLu,
    /// Sigmoid function (1 / 1 + e(-x))
    Sigmoid,
    /// Tangens Hyperbolicus
    Tanh,
}

impl Activation {
    pub fn apply(&self, input: f32) -> f32 {
        match self {
            Self::ReLu => relu(input),
            Self::Sigmoid => sigmoid(input),
            Self::Tanh => tanh(input),
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
