use std::{fmt::Display, fs, path::PathBuf, time::Instant};

use ndarray::{s, Array, Array1, Array2, ArrayView1, Axis, Dimension, Ix2};
use ndarray_linalg::{Eig, Inverse, SVD};
use ndarray_npy::ReadNpyExt;
use ndarray_rand::{rand_distr::StandardNormal, RandomExt};
use rand::{distributions::Uniform, rngs::ThreadRng, Rng};
use rand_distr::num_traits::Zero;
use serde::{Deserialize, Serialize};
use sprs::prod::mul_acc_mat_vec_csr;

use crate::{activation::Activation, commands::TrainArgs, constants};

use self::data::NpyMetaData;

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
    /// sparse matrix representation (res -> res)
    weights_rr_sparse: Option<sprs::CsMat<f64>>,
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
    /// The number of neurons that are visible to the output (potentially)
    visible_count: usize,
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

fn pseudo_inverse(matrix: &Array2<f64>, regularization: f64) -> Result<Array2<f64>, String> {
    let start = std::time::Instant::now();
    let Ok(svd_result) = matrix.svd(true, true) else {
        return Err("SVD failed @ calculation".to_string());
    };

    let (Some(u), sigma, Some(vt)) = svd_result else {
        return Err("SVD failed @ unpacking into U, Sigma, V".to_string());
    };

    let mut s = Array2::zeros((matrix.shape()[0], matrix.shape()[1]));

    for (i, val) in sigma.iter().enumerate() {
        // if the value is very small, we set it to zero (which may be weird?)
        if *val < 1e-9 {
            s[[i, i]] = 0.0;
        } else {
            s[[i, i]] = 1.0 / (val + regularization);
        }
    }

    let pseudo_inv = vt.t().dot(&s.t()).dot(&u.t());
    log::debug!("Pseudo-inverse calculation: {:?}", start.elapsed());

    Ok(pseudo_inv)
}

/// For all non-zero entries of the array, set it to either `either`, or `or`.
/// `either` is selected with a probability of `fract`.
#[allow(unused)]
fn either_or<T, D>(arr: &mut Array<T, D>, either: T, or: T, fract: f64, rng: &mut ThreadRng)
where
    D: Dimension,
    T: std::cmp::PartialEq + Zero + Clone,
{
    arr.iter_mut().for_each(|x| {
        if *x != T::zero() {
            if rng.gen_bool(fract) {
                *x = either.clone();
            } else {
                *x = or.clone();
            }
        }
    })
}

/// keep only those diagonals that are provided in the `diagonals` slice
/// numbering of diagonals: 0 is the main diagonal, 1 is the first diagonal below it, etc.
/// i.e. element (i, j) is on the k-th diagonal if i - j == k
fn mask_n_diagonals<T>(a: &mut Array<T, Ix2>, diags: &[i32])
where
    T: Zero,
{
    a.indexed_iter_mut().for_each(|(d, x)| {
        if !diags.contains(&(d.0 as i32 - d.1 as i32)) {
            *x = T::zero();
        }
    });
}

#[allow(unused)]
fn mask_first_n_rows<T>(a: &mut Array2<T>, n: usize)
where
    T: Zero,
{
    a.indexed_iter_mut().for_each(|(d, x)| {
        if d.0 > n {
            *x = T::zero();
        }
    });
}

impl ReservoirBuilder {
    pub fn from_size_input_outputs(
        mut self,
        size: usize,
        inputs: usize,
        outputs: usize,
        conn: f64,
    ) -> Self {
        self.0.size = size;
        self.0.inputs = inputs;
        self.0.outputs = outputs;
        self.0.visible_count = size;

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

        // either_or(&mut weights_out_res, -0.1, 0.1, 0.5, &mut rng);

        // let bias_res: Array1<f64> = Array::random((size,), StandardNormal);
        // let bias_res: Array1<f64> = Array::random((size,), Uniform::new(-0.01, 0.01));
        let bias_res: Array1<f64> = Array::zeros((size,));
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

    /// Create a reservoir with a grid structure.
    ///
    /// The size is the square root of the amount of neurons (the length of one side of the grid)
    pub fn from_grid(mut self, size: usize, inputs: usize, outputs: usize) -> Self {
        // create the default reservoir
        self = self.from_size_input_outputs(size * size, inputs, outputs, 1.0);

        // set the output connectivity
        let mut rng = rand::thread_rng();
        connectivity(&mut self.0.weights_res_out, 0.2, &mut rng);

        // set the input to the first row of the reservoir
        // mask_first_n_rows(&mut self.0.weights_in_res, size);

        let size = size as i32;
        // use the EuESN P neuron reservoir structure (grid connections ltr, rtl, ttb, btt)
        mask_n_diagonals(&mut self.0.weights_res_res, &[0, 1, -1, size, -size]);

        self
    }

    pub fn from_npy(mut self, metadatapath: PathBuf) -> Self {
        let metadata =
            toml::from_str::<NpyMetaData>(&fs::read_to_string(metadatapath).unwrap()).unwrap();

        // initialize parameters
        self.0.size = metadata.n;
        self.0.inputs = 1;
        self.0.outputs = 1;

        // initialize state and output neurons
        self.0.state = Array1::zeros(self.0.size);
        self.0.output = Array1::zeros(self.0.outputs);

        // initialize resonant weights
        let res_res_reader = fs::File::open(metadata.res_res_path).unwrap();
        let res = Array2::<f32>::read_npy(res_res_reader).unwrap();
        self.0.weights_res_res = res.mapv(|x| x as f64);

        // initialize input weights
        let in_res_reader = fs::File::open(metadata.in_res_path).unwrap();
        let res = Array2::<f32>::read_npy(in_res_reader).unwrap();
        self.0.weights_in_res = res.mapv(|x| x as f64);

        // initialize reservoir bias weights
        let bias_reader = fs::File::open(metadata.bias_path).unwrap();
        let bias = Array1::<f32>::read_npy(bias_reader).unwrap();
        self.0.bias_res = bias.mapv(|x| x as f64);

        // initialize feedback weights (no feedback, thus zero)
        self.0.weights_out_res = Array2::zeros((self.0.size, self.0.outputs));

        // do not use output bias
        self.0.bias_out = Array1::zeros(self.0.outputs);

        // initialize output weights to the pre-trained values
        let out_res_reader = fs::File::open(metadata.out_path).unwrap();
        let res = Array2::<f32>::read_npy(out_res_reader).unwrap();
        self.0.weights_res_out = res.mapv(|x| x as f64);

        // set visible parameter to ensure that o neurons won't be trained
        self.0.visible_count = metadata.n / 3;

        self.leak_rate(metadata.leak_rate)
    }

    pub fn activation(mut self, activation: Activation) -> Self {
        self.0.activation = activation;
        self
    }

    pub fn leak_rate(mut self, lambda: f64) -> Self {
        self.0.leak_rate = lambda;
        self
    }

    pub fn learning_rate(mut self, lr: f64) -> Self {
        self.0.learning_rate = lr;
        self
    }

    pub fn regularization(mut self, lamba: f64) -> Self {
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
            visible_count: 0,
            inputs: 0,
            outputs: 0,
            activation: Activation::Linear,
            leak_rate: 0.95,
            learning_rate: 0.1,
            warm_up: 50,
            regularization: 0.0,
            weights_rr_sparse: None,
        })
    }

    pub fn from_args(args: &TrainArgs) -> Reservoir {
        let nw = if args.grid {
            Reservoir::new_builder().from_grid(args.size, args.inputs, args.outputs)
            // .leak_rate(0.001)
        } else if let Some(npy) = &args.npy {
            Reservoir::new_builder().from_npy(npy.into())
        } else {
            Reservoir::new_builder().from_size_input_outputs(
                args.size,
                args.inputs,
                args.outputs,
                args.connectivity,
            )
        };

        let mut nw = nw
            .learning_rate(args.learning_rate)
            .leak_rate(args.leak_rate)
            .regularization(args.regularization)
            .activation(args.activation.clone())
            .build();

        if args.npy.is_none() {
            nw.scale(Some(args.spectral_radius));
        }

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

    /// forward the externally supplied state, and return the output array
    pub fn forward_external(&self, state: &mut Array1<f64>, input: &Array1<f64>) -> Array1<f64> {
        let mut new_state = match &self.weights_rr_sparse {
            Some(sparse) => {
                let mut res = Array::<f64, _>::zeros(state.len());
                mul_acc_mat_vec_csr(sparse.view(), state.view(), res.view_mut());
                res
            }
            None => self.weights_res_res.dot(state),
        };

        new_state = new_state + self.weights_in_res.dot(input);
        new_state += &self.bias_res;
        new_state.mapv_inplace(|x| self.activation.apply(x));
        *state *= 1.0 - self.leak_rate;
        *state += &(self.leak_rate * &new_state);

        self.weights_res_out
            .dot(&state.slice(s![..self.visible_count]))
    }

    pub fn forward(&mut self, input: &Array1<f64>) {
        // make sure the sparse representation is available
        self.generate_sparse();

        // 1: Reservoir -> Reservoir
        let start = Instant::now();
        // If the sparse matrix is available, use that one.
        let mut new_state = match &self.weights_rr_sparse {
            Some(sparse) => {
                let mut res = Array::<f64, _>::zeros(self.state.len());
                log::trace!("Using sparse matrix multiplication!!");
                mul_acc_mat_vec_csr(sparse.view(), &self.state, res.view_mut());
                res
            }
            None => self.weights_res_res.dot(&self.state),
        };
        let resres_time = start.elapsed();
        new_state = new_state + self.weights_in_res.dot(input);
        new_state += &self.bias_res;
        new_state.mapv_inplace(|x| self.activation.apply(x));
        self.state = (1.0 - self.leak_rate) * &self.state + self.leak_rate * &new_state;

        // 2: Reservoir -> Output
        self.output = self
            .weights_res_out
            .dot(&self.state.slice(s![..self.visible_count])); // + &self.bias_out;

        log::trace!("\x1b[1mForward Timing:\x1b[0m");
        log::trace!("ResRes: {:?}", resres_time);
    }

    pub fn get_output(&self, output_id: usize) -> f64 {
        *self.output.get(output_id).unwrap()
    }

    pub fn get_visible_state(&self) -> ArrayView1<f64> {
        self.state.slice(s![..self.visible_count])
    }

    /// Perform some gradient descent training steps on the network,
    /// using MSE as the loss function.
    ///
    /// returns the average squared error
    pub fn train_mse_grad(
        &mut self,
        inputs: &[Array1<f64>],
        targets: &[Option<Array1<f64>>],
    ) -> f64 {
        assert!(inputs.len() == targets.len());

        // initialize a zero state vector
        let mut state: Array1<f64> = Array1::zeros(self.size);

        // keep track of the gradient of the SE w.r.t. the output weights
        let mut grad: Array2<f64> = Array2::zeros(self.weights_res_out.dim());

        let mut error: f64 = 0.0;
        let mut target_count = 0;

        let mut grad_tmp = Array2::zeros(self.weights_res_out.dim());

        for (target, input) in targets.iter().zip(inputs) {
            let output = self.forward_external(&mut state, input);

            // calculate diff error / diff output
            let Some(target) = target else {
                continue;
            };

            target_count += 1;

            let diff = output - target;

            // add to the error
            error += diff.dot(&diff);

            // calculate the gradient for this timestep
            let diff_arr = diff.into_shape((self.outputs, 1)).unwrap();
            let state_arr = state
                .slice(s![..self.visible_count])
                .into_shape((self.visible_count, 1))
                .unwrap();
            grad_tmp.assign(&diff_arr.dot(&state_arr.t()));

            grad += &grad_tmp;
        }

        // average the gradient and error
        grad /= target_count as f64;

        // apply the gradient
        self.weights_res_out = &self.weights_res_out - self.learning_rate * grad;

        error
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
        targets: &[Option<Array1<f64>>],
        offset: usize,
    ) -> f64 {
        // if the target times are given, we only train the network at those times
        // otherwise, we train at all times
        let train_instants_count = targets.iter().filter(|x| x.is_some()).count();
        let mut states: Array2<f64> = Array2::zeros((self.visible_count, train_instants_count));
        let mut target_outputs: Array2<f64> = Array2::zeros((self.outputs, train_instants_count));
        let mut error: f64 = 0.0;

        let mut column_idx = 0;

        // calculate all states
        for (j, input) in inputs.iter().skip(offset).enumerate() {
            self.forward(input);

            // only train the specified times
            let Some(target) = &targets[j] else {
                continue;
            };

            // save the state at this target time to the states matrix
            let slice = self.state.slice(s![..self.visible_count]);

            // this slice will become a column in the states matrix
            assert!(slice.len() == states.shape()[0]);
            states
                .column_mut(column_idx)
                .iter_mut()
                .zip(slice.iter())
                .for_each(|(a, b)| *a = *b);

            // save the output at the target time to the target_outputs matrix
            self.output
                .iter()
                .enumerate()
                .for_each(|(i, output)| error += (target[i] - output).powi(2));

            column_idx += 1;
        }

        // get rid of none values
        let targets = targets.iter().flatten().collect::<Vec<_>>();

        // get these target outputs into a ndarray matrix
        target_outputs
            // iterate over columns
            .axis_iter_mut(Axis(1))
            // zip with target instances (each column is one moment in time)
            .zip(targets.iter())
            .for_each(|(mut col, target)| {
                // assign each row in this column
                col.iter_mut().zip(target.iter()).for_each(|(a, b)| *a = *b);
            });

        // pseudo-inverse calculation -> doesn't allow for regularization!
        let pseudo_inv = pseudo_inverse(&states, self.regularization);

        let new_weights = match pseudo_inv {
            Ok(pinv) => target_outputs.dot(&pinv),
            Err(_) => {
                // SVD failed, use the regular Moore-Penrose pseudo-inverse method

                let yxt = target_outputs.dot(&states.t());
                let xxt = states.dot(&states.t());
                let lambdas = self.regularization * Array2::eye(self.visible_count);
                let xxt_lambda_inv = (xxt + lambdas).inv().unwrap();
                yxt.dot(&xxt_lambda_inv)
            }
        };

        self.weights_res_out =
            (1.0 - self.learning_rate) * &self.weights_res_out + self.learning_rate * new_weights;

        error
    }

    /// Set the sparse representation if it doesn't already exist
    pub fn generate_sparse(&mut self) {
        if self.weights_rr_sparse.is_some() {
            return;
        }

        let result = sprs::CsMat::csr_from_dense(self.weights_res_res.view(), -1.0);
        self.weights_rr_sparse = Some(result);
    }
}

#[cfg(test)]
mod tests {
    use make_csv::{csv_entry, csv_start, csv_stop};

    use crate::data::load_train_data;

    use super::*;

    #[test]
    fn example_run_for_plots() {
        let data_name = "default";
        let model_name = "3_8";

        let timestep = 20.0;
        let width = 10;
        let target_width = 1;

        let mut nw = Reservoir::load_from_name(model_name).unwrap();

        let (inputs, _targets) =
            load_train_data(data_name, timestep, width, target_width, None).unwrap();

        let mut wtr = csv_start!("data/width_10.csv");

        csv_entry!(wtr <- "t", "input", "nw");

        for (i, input) in inputs.iter().enumerate() {
            nw.forward(input);

            csv_entry!(wtr <- i, input[0], nw.output[0]);
        }

        csv_stop!(wtr);
    }
}
