use crate::{
    commands::{HyperArgs, TrainArgs},
    reservoir::Reservoir,
    trainutil::create_progress_bar,
};

const COUNT: usize = 2;

/// Defines the hyperparameter space to train models in,
/// and generate plots and other data for.
#[derive(Debug, serde::Deserialize)]
struct HyperparameterSpace {
    /// Number of neurons in the reservoir
    n_neurons: NeuronSpace,
    /// Leaky rate
    alpha: AlphaSpace,
    /// Spectral radius
    rho: RhoSpace,
    /// Regularization parameter
    lambda: LambdaSpace,
}

struct HyperparameterSet {
    n_neurons: usize,
    alpha: f64,
    rho: f64,
    lambda: f64,
}

impl HyperparameterSet {
    fn to_args(&self) -> TrainArgs {
        TrainArgs {
            size: self.n_neurons,
            iter: 100,
            width: 30,
            target_width: 1,
            learning_rate: 0.1,
            leak_rate: self.alpha,
            regularization: self.lambda,
            inputs: 1,
            outputs: 1,
            connectivity: 0.2,
            spectral_radius: self.rho,
            timestep: 2.0,
            dont_stop_early: false,
            data: "3_8".into(),
            list_data: false,
            split: 0.9,
            grid: false,
            npy: None,
            shift: None,
            mode: crate::commands::TrainMode::Inv,
            activation: crate::activation::Activation::Tanh,
        }
    }
}

struct HyperparameterIter(HyperparameterSpace, usize);

/// Number of neurons in the reservoir
#[derive(Debug, serde::Deserialize)]
struct NeuronSpace {
    min: usize,
    max: usize,
    count: usize,
}

/// Leaky rate
#[derive(Debug, serde::Deserialize)]
struct AlphaSpace {
    min: f64,
    max: f64,
    count: usize,
}

/// Spectral radius
#[derive(Debug, serde::Deserialize)]
struct RhoSpace {
    min: f64,
    max: f64,
    count: usize,
}

/// Regularization parameter
#[derive(Debug, serde::Deserialize)]
struct LambdaSpace {
    min: f64,
    max: f64,
    count: usize,
}

impl HyperparameterSpace {
    fn len(&self) -> usize {
        self.n_neurons.count * self.alpha.count * self.rho.count * self.lambda.count
    }
}

fn space_index_to_value(min: f64, max: f64, count: usize, index: usize) -> f64 {
    if count == 1 {
        return min;
    }
    min + (index as f64 / (count as f64 - 1.0)) * (max - min)
}

impl IntoIterator for HyperparameterSpace {
    type Item = HyperparameterSet;

    type IntoIter = HyperparameterIter;

    fn into_iter(self) -> Self::IntoIter {
        HyperparameterIter(self, 0)
    }
}

impl Iterator for HyperparameterIter {
    type Item = HyperparameterSet;

    fn next(&mut self) -> Option<Self::Item> {
        if self.1 >= self.0.len() {
            return None;
        }

        let n_neurons = space_index_to_value(
            self.0.n_neurons.min as f64,
            self.0.n_neurons.max as f64,
            self.0.n_neurons.count,
            self.1 % self.0.n_neurons.count,
        ) as usize;
        let alpha = space_index_to_value(
            self.0.alpha.min,
            self.0.alpha.max,
            self.0.alpha.count,
            (self.1 / self.0.n_neurons.count) % self.0.alpha.count,
        );
        let rho = space_index_to_value(
            self.0.rho.min,
            self.0.rho.max,
            self.0.rho.count,
            (self.1 / (self.0.n_neurons.count * self.0.alpha.count)) % self.0.rho.count,
        );
        let lambda = space_index_to_value(
            self.0.lambda.min,
            self.0.lambda.max,
            self.0.lambda.count,
            (self.1 / (self.0.n_neurons.count * self.0.alpha.count * self.0.rho.count))
                % self.0.lambda.count,
        );

        self.1 += 1;

        Some(HyperparameterSet {
            n_neurons,
            alpha,
            rho,
            lambda,
        })
    }
}

fn test_hypers(
    hypers: &HyperparameterSet,
    count: usize,
) -> Result<(f64, Box<Reservoir>), Box<dyn std::error::Error>> {
    let args = hypers.to_args();

    let mut nw = Reservoir::from_args(&args);
    let error = nw.train(&args)?;
    nw.plot(
        &args,
        format!(
            "data/hypers_n{}_a{:.3}_r{:.3}_l{:.3}_count{}.svg",
            hypers.n_neurons, hypers.alpha, hypers.rho, hypers.lambda, count
        )
        .as_str(),
    )?;
    Ok((error, Box::new(nw)))
}

pub fn hyper(args: HyperArgs) -> Result<(), Box<dyn std::error::Error>> {
    let hyperfile = std::fs::read_to_string(args.path)?;
    let hyper: HyperparameterSpace = toml::from_str(&hyperfile)?;

    let pb = create_progress_bar("Testing Hypers...", (hyper.len() * COUNT) as u64);

    for hypers in hyper.into_iter() {
        for c in 0..COUNT {
            let (_error, _nw) = test_hypers(&hypers, c)?;
            pb.inc(1);
        }
    }

    pb.finish();

    Ok(())
}
