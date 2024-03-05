use std::{
    error::Error,
    fmt::Display,
    ops::{Index, IndexMut},
};

use super::{EucledeanArgs, GenerateDataArgs};

/// A Rhythmic Pattern is just a collection of onsets and silent rests
///
/// This is modeled as a simple vector of booleans
struct RhythmPattern(Vec<bool>);

impl RhythmPattern {
    fn new(n: usize) -> Self {
        let pattern: Vec<bool> = vec![false; n];

        Self(pattern)
    }
}

impl Index<usize> for RhythmPattern {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for RhythmPattern {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Display for RhythmPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for b in &self.0 {
            if *b {
                write!(f, "●")?;
            } else {
                write!(f, "◦")?;
            }
        }
        write!(f, "]")?;
        Ok(())
    }
}

fn euclidean(args: EucledeanArgs) -> RhythmPattern {
    let mut pattern = RhythmPattern::new(args.n);
    let real_step = args.n as f64 / args.k as f64;

    for i in 0..args.k {
        let index = (real_step * i as f64).floor() as usize;
        pattern[index] = false;
    }

    pattern
}

/// Generate input-output data to train the reservoir, based on the given arguments
///
/// This function uses research knowledge about rhythmic patterns to generate input-output data
///
/// # Result
/// This function writes to a csv file
pub fn gendata(args: GenerateDataArgs) -> Result<(), Box<dyn Error>> {
    // pseudo code

    // arguments:
    // - algorithm: an enum of possible rhythm generating algorithms
    //      - Euclidean
    //      - NP-DAG
    // - parameters for the sub-algorithm...

    let pattern = match args.algorithm {
        super::RhythmAlgorithm::Euclidean(e) => euclidean(e),
        // TODO: NP-DAG algorithm implementation
        super::RhythmAlgorithm::NPDAG(_) => todo!(),
    };

    Ok(())
}
