use std::{fmt::Display, time::Instant};

use ndarray::{Array, Array1, Array2, Axis, Dimension};
use ndarray_linalg::{Eig, Inverse};
use ndarray_rand::{rand_distr::StandardNormal, RandomExt};
use rand::{distributions::Uniform, rngs::ThreadRng, Rng};

use crate::{
    activation::Activation,
    constants::{self, DAMPING},
};

#[derive(Clone)]
pub struct FullNetwork {
    state: Array1<f32>,
    pub output: Array1<f32>,
    weights_in_res: Array2<f32>,
    weights_res_res: Array2<f32>,
    weights_out_res: Array2<f32>,
    pub weights_res_out: Array2<f32>,
    bias_res: Array1<f32>,
    bias_out: Array1<f32>,
    size: usize,
    inputs: usize,
    outputs: usize,
    pub activation: Activation,
    pub damp_coef: f32,
    learning_rate: f32,
    regularization: f32,
    gradient: Array2<f32>,
}

pub struct FullNetworkBuilder(FullNetwork);

impl Display for FullNetwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "This network has {} internal neurons, {} inputs, and {} outputs",
            self.state.len(),
            self.inputs,
            self.outputs
        )?;
        writeln!(
            f,
            "<<------------------------------------------------------->>"
        )?;
        writeln!(f, "State: \n{}", self.state)?;
        writeln!(
            f,
            "<<------------------------------------------------------->>"
        )?;
        writeln!(f, "Input weights: \n{}", self.weights_in_res)?;
        writeln!(
            f,
            "<<------------------------------------------------------->>"
        )?;
        writeln!(f, "Resonant weights: \n{}", self.weights_res_res)?;
        writeln!(
            f,
            "<<------------------------------------------------------->>"
        )?;
        writeln!(f, "Res bias: \n{}", self.bias_res)?;
        writeln!(
            f,
            "<<------------------------------------------------------->>"
        )?;
        writeln!(f, "Output weights: \n{}", self.weights_res_out)?;
        writeln!(
            f,
            "<<------------------------------------------------------->>"
        )?;
        writeln!(f, "Resulting output: \n{}", self.output)
    }
}

fn connectivity<D>(arr: &mut ndarray::Array<f32, D>, conn_fract: f64, rng: &mut ThreadRng)
where
    D: Dimension,
{
    arr.iter_mut().for_each(|x| {
        if !rng.gen_bool(conn_fract) {
            *x = 0.0;
        }
    });
}

fn either_or<D>(arr: &mut Array<f32, D>, either: f32, or: f32, fract: f64, rng: &mut ThreadRng)
where
    D: Dimension,
{
    arr.iter_mut().for_each(|x| {
        if *x != 0.0 {
            if rng.gen_bool(fract) {
                *x = either;
            } else {
                *x = or;
            }
        }
    })
}

impl FullNetworkBuilder {
    pub fn with_size_input_outputs(
        &mut self,
        size: usize,
        inputs: usize,
        outputs: usize,
        conn: f64,
    ) -> &mut Self {
        self.0.size = size;
        self.0.inputs = inputs;
        self.0.outputs = outputs;

        let state = Array1::zeros(size);
        let output = Array1::zeros(outputs);
        let gradient = Array2::zeros((outputs, size));

        let mut weights_in_res: Array2<f32> = Array::random((size, inputs), StandardNormal);
        let mut weights_res_res: Array2<f32> = Array::random((size, size), StandardNormal);
        let mut weights_res_out: Array2<f32> = Array::random((outputs, size), StandardNormal);
        let mut weights_out_res: Array2<f32> = Array::random((size, outputs), StandardNormal);

        let mut rng = rand::thread_rng();
        connectivity(&mut weights_res_res, conn, &mut rng);
        connectivity(&mut weights_in_res, conn, &mut rng);
        connectivity(&mut weights_res_out, conn, &mut rng);

        if constants::OUTPUT_NEURON_DIRECT_FEEDBACK {
            weights_out_res = weights_res_out.t().to_owned();
        } else {
            connectivity(&mut weights_out_res, conn, &mut rng);
        }

        either_or(&mut weights_out_res, -0.05, 0.05, 0.5, &mut rng);

        // let bias_res: Array1<f32> = Array::random((size,), StandardNormal);
        let bias_res: Array1<f32> = Array::random((size,), Uniform::new(-0.01, 0.01));
        let bias_out: Array1<f32> = Array::random((outputs,), Uniform::new(0.0, 0.01));

        self.0.state = state;
        self.0.output = output;

        self.0.weights_in_res = weights_in_res;
        self.0.weights_res_res = weights_res_res;
        self.0.weights_out_res = weights_out_res;
        self.0.weights_res_out = weights_res_out;

        self.0.bias_res = bias_res;
        self.0.bias_out = bias_out;

        self.0.gradient = gradient;

        self
    }

    pub fn with_learning_rate(&mut self, lr: f32) -> &mut Self {
        self.0.learning_rate = lr;
        self
    }

    pub fn with_regularization(&mut self, lamba: f32) -> &mut Self {
        self.0.regularization = lamba;
        self
    }

    pub fn build(&self) -> FullNetwork {
        self.0.clone()
    }
}

impl FullNetwork {
    pub fn new() -> FullNetworkBuilder {
        FullNetworkBuilder(FullNetwork {
            state: Array1::zeros(0),
            output: Array1::zeros(0),
            weights_in_res: Array2::zeros((0, 0)),
            weights_res_res: Array2::zeros((0, 0)),
            weights_out_res: Array2::zeros((0, 0)),
            weights_res_out: Array2::zeros((0, 0)),
            bias_res: Array1::zeros(0),
            bias_out: Array1::zeros(0),
            size: 0,
            inputs: 0,
            outputs: 0,
            activation: Activation::Tanh,
            damp_coef: DAMPING,
            learning_rate: 0.1,
            regularization: 0.5,
            gradient: Array2::zeros((0, 0)),
        })
    }

    pub fn scale(&mut self, target: Option<f32>) {
        let (eig, _) = self.weights_res_res.eig().unwrap();

        let max_eig = eig[0].norm();

        println!("{max_eig}");

        let target = target.unwrap_or(1.0);

        self.weights_res_res = &self.weights_res_res * target / (max_eig);
    }

    pub fn select_output(&mut self, coord: (usize, usize)) {
        let shape = self.weights_res_out.shape();
        let mut weights_res_out: Array2<f32> = Array::zeros((shape[0], shape[1]));
        *weights_res_out.get_mut(coord).unwrap() = 1.0;

        self.weights_res_out = weights_res_out;
    }

    pub fn reset_state(&mut self) {
        self.state = Array1::zeros(self.state.len());
        self.output = Array1::zeros(self.outputs);
    }

    pub fn adjust_damping(&mut self, amount: f32) {
        self.damp_coef += amount;
    }

    pub fn forward(&mut self, input: &Array1<f32>) {
        let mut new_state = self.weights_res_res.dot(&self.state);
        new_state = new_state + self.weights_in_res.dot(input);
        new_state = new_state + self.weights_out_res.dot(&self.output);
        new_state = new_state + &self.bias_res;

        new_state.mapv_inplace(|x| self.activation.apply(x));

        self.state = self.damp_coef * &self.state + (1.0 - self.damp_coef) * &new_state;

        self.output = self.weights_res_out.dot(&self.state); // + &self.bias_out;
    }

    pub fn get_output(&self, output_id: usize) -> f32 {
        *self.output.get(output_id).unwrap()
    }

    pub fn train(&mut self, inputs: &[Array1<f32>], targets: &[Array1<f32>]) -> f64 {
        let mut states: Array2<f32> = Array2::zeros((self.state.len(), inputs.len()));
        let mut outputs: Array2<f32> = Array2::zeros((self.outputs, inputs.len()));
        let mut states_vec: Vec<Array1<f32>> = Vec::with_capacity(inputs.len());
        let mut error: f64 = 0.0;

        // calculate all states
        // self.reset_state();
        for (j, input) in inputs.iter().enumerate() {
            self.forward(input);
            states_vec.push(self.state.clone());
            self.output
                .iter()
                .enumerate()
                .for_each(|(i, output)| error += (targets[j][i] - output).powi(2) as f64)
        }

        // create X matrix
        for (i, mut row) in states.axis_iter_mut(Axis(0)).enumerate() {
            for (j, col) in row.iter_mut().enumerate() {
                *col = states_vec[j][i];
            }
        }

        // create Y matrix
        for (i, mut row) in outputs.axis_iter_mut(Axis(0)).enumerate() {
            for (j, col) in row.iter_mut().enumerate() {
                *col = targets[j][i];
            }
        }

        let mut weights = outputs.dot(&states.t());

        // the most expensive calculation! (almost 40% of training time...)
        let xxt = states.dot(&states.t());

        let lambdas = self.regularization * Array2::eye(self.state.len());

        let mut tmp = xxt + lambdas;

        tmp = tmp.inv().unwrap();

        weights = weights.dot(&tmp);

        self.weights_res_out =
            (1.0 - self.learning_rate) * &self.weights_res_out + self.learning_rate * weights;

        error
    }
}
