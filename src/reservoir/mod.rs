use std::fmt::Display;

use ndarray::{Array, Array1, Array2, Axis, Dimension};
use ndarray_linalg::{Eig, SVD};
use ndarray_rand::{rand_distr::StandardNormal, RandomExt};
use rand::{distributions::Uniform, rngs::ThreadRng, Rng};
use serde::{Deserialize, Serialize};

use crate::{activation::Activation, commands::TrainArgs, constants};

pub mod data;

/// A ESN (Echo State Network) reservoir.
#[derive(Clone, Serialize, Deserialize)]
pub struct Reservoir {
    /// internal state of the reservoir
    state: Array1<f64>,
    /// output layer of the reservoir
    pub output: Array1<f64>,
    /// input weights (input -> reservoir)
    weights_in_res: Array2<f64>,
    /// resonant weights (reservoir -> reservoir)
    weights_res_res: Array2<f64>,
    /// output feedback weights (output -> reservoir)
    weights_out_res: Array2<f64>,
    /// output weights (reservoir -> output)
    /// these are the to-be-trained weights
    pub weights_res_out: Array2<f64>,
    /// bias for the reservoir
    bias_res: Array1<f64>,
    /// bias for the output layer
    bias_out: Array1<f64>,
    /// size of the reservoir
    size: usize,
    /// number of inputs
    pub inputs: usize,
    /// number of outputs
    outputs: usize,
    /// activation function
    pub activation: Activation,
    /// leak rate of the neurons
    pub leak_rate: f64,
    /// learning rate of the network
    learning_rate: f64,
    /// number of warm-up steps (currently unused)
    warm_up: usize,
    /// regularization parameter lambda
    regularization: f64,
}

/// A builder for the Reservoir struct.
pub struct ReservoirBuilder(Reservoir);

impl Display for Reservoir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "This network has {} internal neurons, {} inputs, and {} outputs",
            self.state.len(),
            self.inputs,
            self.outputs
        )?;
        writeln!(f, "State: \n{}", self.state)?;
        writeln!(f, "Input weights: \n{}", self.weights_in_res)?;
        writeln!(f, "Resonant weights: \n{}", self.weights_res_res)?;
        writeln!(f, "Res bias: \n{}", self.bias_res)?;
        writeln!(f, "Output weights: \n{}", self.weights_res_out)?;
        writeln!(f, "Resulting output: \n{}", self.output)
    }
}

/// ensure that the ndarray has zero-entries for approx. `(1 - conn_fract)` fraction of entries.
fn connectivity<D>(arr: &mut Array<f64, D>, conn_fract: f64, rng: &mut ThreadRng)
where
    D: Dimension,
{
    arr.iter_mut().for_each(|x| {
        if !rng.gen_bool(conn_fract) {
            *x = 0.0;
        }
    });
}

fn pseudo_inverse(matrix: &Array2<f64>, regularization: f64) -> Array2<f64> {
    let (Some(u), sigma, Some(vt)) = matrix.svd(true, true).unwrap() else {
        panic!("SVD failed");
    };

    let mut s = Array2::zeros((matrix.shape()[0], sigma.len()));

    for (i, val) in sigma.iter().enumerate() {
        // if the value is very small, we set it to zero (which may be weird?)
        if *val < 1e-9 {
            s[[i, i]] = 0.0;
        } else {
            s[[i, i]] = 1.0 / (val + regularization);
        }
    }

    let pseudo_inv = vt.t().dot(&s.t()).dot(&u.t());

    pseudo_inv
}

/// For all non-zero entries of the array, set it to either `either`, or `or`.
/// `either` is selected with a probability of `fract`.
fn either_or<D>(arr: &mut Array<f64, D>, either: f64, or: f64, fract: f64, rng: &mut ThreadRng)
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

impl ReservoirBuilder {
    pub fn with_size_input_outputs(
        mut self,
        size: usize,
        inputs: usize,
        outputs: usize,
        conn: f64,
    ) -> Self {
        self.0.size = size;
        self.0.inputs = inputs;
        self.0.outputs = outputs;

        let state = Array1::zeros(size);
        let output = Array1::zeros(outputs);

        let mut weights_in_res: Array2<f64> = Array::random((size, inputs), StandardNormal);
        let mut weights_res_res: Array2<f64> = Array::random((size, size), StandardNormal);
        let mut weights_res_out: Array2<f64> = Array::random((outputs, size), StandardNormal);
        let mut weights_out_res: Array2<f64> = Array::random((size, outputs), StandardNormal);

        let mut rng = rand::thread_rng();
        connectivity(&mut weights_res_res, conn, &mut rng);
        connectivity(&mut weights_in_res, conn, &mut rng);
        connectivity(&mut weights_res_out, conn, &mut rng);

        if constants::OUTPUT_NEURON_DIRECT_FEEDBACK {
            // with direct feedback, the out -> res weights are the transpose of the res -> out weights
            // TODO: play with the value of this constant to see which effect it has on training
            // efficiency
            weights_out_res = weights_res_out.t().to_owned();
        } else {
            connectivity(&mut weights_out_res, conn, &mut rng);
        }

        either_or(&mut weights_out_res, -0.1, 0.1, 0.5, &mut rng);

        // let bias_res: Array1<f64> = Array::random((size,), StandardNormal);
        let bias_res: Array1<f64> = Array::random((size,), Uniform::new(-0.01, 0.01));
        let bias_out: Array1<f64> = Array::random((outputs,), Uniform::new(-0.01, 0.01));

        self.0.state = state;
        self.0.output = output;

        self.0.weights_in_res = weights_in_res;
        self.0.weights_res_res = weights_res_res;
        self.0.weights_out_res = weights_out_res;
        self.0.weights_res_out = weights_res_out;

        self.0.bias_res = bias_res;
        self.0.bias_out = bias_out;

        self
    }

    pub fn with_leak_rate(mut self, lambda: f64) -> Self {
        self.0.leak_rate = lambda;
        self
    }

    pub fn with_learning_rate(mut self, lr: f64) -> Self {
        self.0.learning_rate = lr;
        self
    }

    pub fn with_regularization(mut self, lamba: f64) -> Self {
        self.0.regularization = lamba;
        self
    }

    pub fn build(self) -> Reservoir {
        self.0
    }
}

impl Reservoir {
    pub fn new_builder() -> ReservoirBuilder {
        ReservoirBuilder(Reservoir {
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
            leak_rate: 0.95,
            learning_rate: 0.1,
            warm_up: 50,
            regularization: 0.0,
        })
    }

    pub fn from_args(args: &TrainArgs) -> Reservoir {
        let mut nw = Reservoir::new_builder()
            .with_size_input_outputs(args.size, args.inputs, args.outputs, args.connectivity)
            .with_learning_rate(args.learning_rate)
            .with_leak_rate(args.leak_rate)
            .with_regularization(args.regularization)
            .build();

        nw.scale(Some(args.spectral_radius));

        nw
    }

    pub fn set_weights_out(&mut self, weights: Array2<f64>) {
        self.weights_res_out = weights;
    }

    pub fn scale(&mut self, target: Option<f64>) {
        let (eig, _) = self.weights_res_res.eig().unwrap();

        let max_eig = eig[0].norm();

        let target = target.unwrap_or(1.0);

        self.weights_res_res = &self.weights_res_res * target / (max_eig);
    }

    pub fn reset_state(&mut self) {
        self.state = Array1::zeros(self.state.len());
        self.output = Array1::zeros(self.outputs);
    }

    pub fn adjust_damping(&mut self, amount: f64) {
        self.leak_rate += amount;
    }

    pub fn forward(&mut self, input: &Array1<f64>) {
        let mut new_state = self.weights_res_res.dot(&self.state);
        new_state = new_state + self.weights_in_res.dot(input);
        // NOTE: this does not seem to have any effect on the trained result...
        new_state = new_state + self.weights_out_res.dot(&self.output);
        //
        // new_state += &self.bias_res;

        new_state.mapv_inplace(|x| self.activation.apply(x));

        self.state = (1.0 - self.leak_rate) * &self.state + self.leak_rate * &new_state;

        self.output = self.weights_res_out.dot(&self.state); // + &self.bias_out;
                                                             // self.output.mapv_inplace(|x| (self.activation)(x));
    }

    pub fn get_output(&self, output_id: usize) -> f64 {
        *self.output.get(output_id).unwrap()
    }

    /// Train the reservoir using the pseudo-inverse method.
    ///
    /// # Arguments
    /// - `inputs` - A slice of input arrays
    /// - `targets` - A slice of target arrays
    /// - `target_times` - An optional slice of times at which the targets are to be reached
    ///
    /// # Returns
    /// The squared error of the training step
    ///
    /// # Note
    /// - This method does not reset the state of the reservoir, so the user can decide when to do so
    ///   themselves.
    /// - The targets slice can be either the length of the inputs slice, or the length of the target_times Vec.
    ///   This gives the user the flexibility to specify the target output at specific times only,
    ///   or to provide the target output at every time step.
    /// - The returned error is the error the model makes at the moment, thus before the training
    ///   step
    pub fn train_step(
        &mut self,
        inputs: &[Array1<f64>],
        targets: &[Array1<f64>],
        target_times: Option<&Vec<usize>>,
    ) -> f64 {
        // if the target times are given, we only train the network at those times
        // otherwise, we train at all times
        let train_instants_count = if let Some(times) = target_times {
            assert!(
                targets.len() == times.len(),
                "The number of targets should be the same length as the specified times in target_times."
                );
            assert!(
                times.len() <= inputs.len(),
                "There cannot be more specified training times than network input values."
            );
            times.len()
        } else {
            inputs.len()
        };

        let mut states: Array2<f64> = Array2::zeros((self.state.len(), train_instants_count));
        let mut target_outputs: Array2<f64> = Array2::zeros((self.outputs, train_instants_count));
        let mut states_vec: Vec<Array1<f64>> = Vec::with_capacity(train_instants_count);
        let mut error: f64 = 0.0;

        // calculate all states
        let mut target_index = 0;
        for (j, input) in inputs.iter().enumerate() {
            self.forward(input);

            // only train at the specified times
            if let Some(times) = target_times {
                if !times.contains(&j) {
                    continue;
                }
            }

            states_vec.push(self.state.clone());
            self.output
                .iter()
                .enumerate()
                .for_each(|(i, output)| error += (targets[target_index][i] - output).powi(2));
            target_index += 1;
        }

        // create X matrix (states)
        for (i, mut row) in states.axis_iter_mut(Axis(0)).enumerate() {
            for (j, col) in row.iter_mut().enumerate() {
                *col = states_vec[j][i];
            }
        }

        // create Y matrix (target outputs)
        for (i, mut row) in target_outputs.axis_iter_mut(Axis(0)).enumerate() {
            for (j, col) in row.iter_mut().enumerate() {
                *col = targets[j][i];
            }
        }

        // pseudo-inverse calculation -> doesn't allow for regularization!
        let pseudo_inv = pseudo_inverse(&states, self.regularization);

        let yxt = target_outputs.dot(&pseudo_inv);

        // let mut yxt = target_outputs.dot(&states.t());

        // // the most expensive calculation! (almost 40% of training time...)
        // let xxt = states.dot(&states.t());

        // let lambdas = self.regularization * Array2::eye(self.state.len());

        // let xxt_lambda_inv = (xxt + lambdas).inv().unwrap();

        // yxt = yxt.dot(&xxt_lambda_inv);

        self.weights_res_out =
            (1.0 - self.learning_rate) * &self.weights_res_out + self.learning_rate * yxt;

        error
    }
}
