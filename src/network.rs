use ndarray::{Array, Array1, Array2};
use ndarray_rand::{rand_distr::Uniform, RandomExt};

use crate::activation::Activation;

#[derive(Clone)]
pub struct NeuralNetwork {
    pub layers: Vec<Layer>,
}

pub struct NeuralNetworkBuilder(NeuralNetwork);

#[derive(Clone)]
pub struct Layer {
    pub size: usize,
    pub weights_in: Array2<f32>,
    pub bias_in: Array1<f32>,
    pub activation: Activation,
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            size: 0,
            weights_in: Array2::zeros((1, 2)),
            bias_in: Array1::zeros(2),
            activation: Default::default(),
        }
    }
}

impl Layer {
    fn output_from(&self, inputs: Array1<f32>) -> Array1<f32> {
        let mut outputs = self.weights_in.dot(&inputs);

        println!("This weights size {:?}", self.weights_in.shape());

        outputs = outputs + &self.bias_in;

        outputs
            .iter_mut()
            .for_each(|x| *x = self.activation.apply(*x));

        outputs
    }
}

impl NeuralNetworkBuilder {
    /// add layers with given neuron counts to the Neural Network
    pub fn with_layers(&mut self, layers: &[usize]) -> &mut Self {
        for i in 1..layers.len() {
            // randomize initial weights and biases between 0.0 and 0.1
            let weights: Array2<f32> =
                Array::random((layers[i], layers[i - 1]), Uniform::new(-0.05f32, 0.1f32));
            // let biases: Array1<f32> = Array::random(layers[i], Uniform::new(0.01f32, 0.1f32));
            // let weights: Array2<f32> = Array::ones((layers[i], layers[i - 1]));

            let biases: Array1<f32> = Array::ones(layers[i]);

            let layer = Layer {
                size: layers[i],
                weights_in: weights,
                bias_in: biases,
                ..Default::default()
            };

            self.0.layers.push(layer);
        }

        self
    }

    pub fn build(&self) -> NeuralNetwork {
        self.0.clone()
    }
}

impl NeuralNetwork {
    pub fn new() -> NeuralNetworkBuilder {
        return NeuralNetworkBuilder(NeuralNetwork { layers: Vec::new() });
    }

    // pub fn train(input: Array2<f32>, labels: Array2<f32>) {}

    pub fn forward(&self, input: Array1<f32>) -> Result<Array1<f32>, String> {
        if input.len() != self.layers[0].weights_in.shape()[1] {
            return Err("input has incompatible dimension".to_string());
        }

        let mut output: Array1<f32>;
        output = input;

        for (i, layer) in self.layers.iter().enumerate() {
            println!("Applying layer {}", i + 1);
            output = layer.output_from(output);
        }

        Ok(output)
    }
}
