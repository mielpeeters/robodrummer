use std::{collections::VecDeque, time::Instant};

use make_csv::{csv_entry, csv_start, python};
use rustfft::{algorithm::Radix4, num_complex::Complex, Fft, FftDirection};

use crate::metronomer::{frequency::FrequencyComponent, spectrum::Spectrum};

const MINIMUM_HITS_FOR_FOURIER: u32 = 5;
const MINIMUM_FREQUENCY: f64 = 40.0 / 60.0;
const MAXIMUM_FREQUENCY: f64 = 210.0 / 60.0;
const WINDOW_LENGTH: f64 = 5.0;

const BAND_WIDTH: f64 = 0.3;

/// The InputWindow keeps memory of the input hits and fourier related data
pub struct InputWindow {
    start: Instant,
    /// The window keeps track of times of hits
    ///
    /// example:
    /// - first hit at time 100
    /// - second hit at time 120
    /// - third hit at time 150
    /// then we have the resulting window: (50, 20, 0)
    ///
    /// - fourth hit at time 170
    /// then we have the resulting window: (70, 50, 20)
    pub window: VecDeque<u128>,
    scratch: Vec<Complex<f64>>,
    fft: Radix4<f64>,
    pub best_frequency: f64,
    /// The time between each hit
    sample_period: f64,
    pub min_band: f64,
    pub max_band: f64,
    /// The number of hits
    pub hit_count: u32,
}

fn index_to_frequency(i: usize, size: usize, sample_period: f64) -> f64 {
    let offset_index = if i > size / 2 {
        i as f64 - size as f64
    } else {
        i as f64
    };

    offset_index / size as f64 / sample_period
}

/// The different options for what to perform on a hit
#[derive(PartialEq)]
pub enum HitAction {
    /// Do nothing
    NoFourier,
    /// Calculate the fourier transform once for every period
    Interval(u8),
    /// Calculate the fourier transform every hit
    Fourier,
    /// Calculate the fourier transform every period and adjust the band
    BandedInterval(u8),
}

impl InputWindow {
    pub fn new_with_size(size: usize, sample_period: f64) -> Self {
        let window = VecDeque::from(vec![0; size]);

        let start = Instant::now();

        let scratch = vec![Complex { re: 0.0, im: 0.0 }; size];

        let fft = Radix4::new(size, FftDirection::Forward);

        let best_frequency = 2.0;

        InputWindow {
            start,
            window,
            scratch,
            fft,
            best_frequency,
            sample_period,
            min_band: MINIMUM_FREQUENCY,
            max_band: MAXIMUM_FREQUENCY,
            hit_count: 0,
        }
    }

    /// Add a hit to the input window, using the given HitAction
    /// Returns true if the best frequency was updated
    pub fn hit(&mut self, action: HitAction) -> bool {
        self.hit_count += 1;

        if self.hit_count == 1 {
            self.start = Instant::now();
        }

        // calculate the amount of periods between start and now
        let current_sampling_period = Instant::now().duration_since(self.start).as_millis() as f64
            / 1000.0
            / self.sample_period;

        // implement a queue behaviour with the window
        self.window
            .push_front(current_sampling_period.round() as u128);
        self.window.pop_back();

        // calculate fourier only when the action asks for it and we have enough hits
        if action == HitAction::NoFourier {
            return false;
        }

        match action {
            HitAction::Interval(interval) | HitAction::BandedInterval(interval) => {
                if self.hit_count % interval as u32 != 0 {
                    return false;
                }

                // if we can't find dominant frequency, we don't do anything
                let Some(mut frequencies) = self.fft() else {
                    return false;
                };

                // add frequency multiples to the spectrum to get fundamentals knowledge
                frequencies.spectral_sum();

                // update our best estimated frequency
                self.best_frequency = InputWindow::dominant_frequency_between(
                    &frequencies,
                    self.min_band,
                    self.max_band,
                    Some(self.best_frequency),
                )
                .0;
            }
            _ => {}
        }

        if let HitAction::BandedInterval(_) = action {
            self.set_band();
        }

        true
    }

    pub fn show_window(&self) {
        println!("Window: {:?}", self.window);
    }

    pub fn set_band(&mut self) {
        self.min_band = self.best_frequency * (1.0 - BAND_WIDTH / 2.0);
        self.max_band = self.best_frequency * (1.0 + BAND_WIDTH / 2.0);

        if self.min_band < 40.0 / 60.0 {
            self.min_band = 40.0 / 60.0;
        }

        if self.max_band > 210.0 / 60.0 {
            self.max_band = 210.0 / 60.0;
        }
    }

    pub fn create_fft_buffer(&self) -> Option<Vec<Complex<f64>>> {
        let capacity = self.window.len();
        let mut buffer = Vec::with_capacity(capacity);

        // Keep track of the current window index, being the hit that is to be placed in the buffer
        // next
        let mut current_window_index = 0;

        let mut local_hit_count = 0;

        for i in 0..capacity {
            // self.window[0] is the most recent hit, containing a sample period index
            // if i gets larger than this, it means we are looking in the past where there are no hits, so skip
            // Also skip if the time is larger than the window length
            if self.window[0] < i as u128 || i as f64 * self.sample_period > WINDOW_LENGTH {
                buffer.insert(0, Complex { re: 0.0, im: 0.0 });
                continue;
            }

            // "time" being in terms of sampling periods
            let current_time = self.window[0] - i as u128;

            let to_scale = |i: usize| 1.0 - (i as f64 * self.sample_period / WINDOW_LENGTH);

            let mut j = current_window_index;
            let val = loop {
                match self.window[j].cmp(&current_time) {
                    std::cmp::Ordering::Equal => {
                        current_window_index = j;
                        local_hit_count += 1;
                        break 1.0;
                    }
                    // we have gone too far back in time, and conclude there was no hit at this time
                    std::cmp::Ordering::Less => break 0.0,
                    std::cmp::Ordering::Greater => {}
                }
                j += 1;
            };

            buffer.insert(
                0,
                Complex {
                    re: val * to_scale(j),
                    im: 0.0,
                },
            );
        }

        if local_hit_count < MINIMUM_HITS_FOR_FOURIER {
            return None;
        }

        Some(buffer)
    }

    fn fft(&mut self) -> Option<Spectrum> {
        let mut buffer = self.create_fft_buffer()?;

        self.fft
            .process_with_scratch(&mut buffer, &mut self.scratch);

        Some(
            buffer
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    FrequencyComponent(
                        index_to_frequency(i, self.window.len(), self.sample_period),
                        x.norm(),
                    )
                })
                .collect(),
        )
    }

    fn dominant_frequency_between(
        spectrum: &Spectrum,
        min_freq: f64,
        max_freq: f64,
        current_best: Option<f64>,
    ) -> FrequencyComponent {
        spectrum
            .band_pass(min_freq, max_freq)
            .into_iter()
            .max_by(|one, two| one.partial_cmp(two).expect("oops"))
            .unwrap()
    }

    pub fn plot_decision(&mut self) {
        let mut frequencies = match self.fft() {
            Some(f) => f,
            None => {
                return;
            }
        };

        frequencies.spectral_sum();

        {
            let mut wtr = csv_start!("out.csv");
            csv_entry!(wtr <- "t", "$\\text{Value}$");

            let Some(buffer) = self.create_fft_buffer() else {
                panic!("No buffer found");
            };

            for (i, val) in buffer.iter().enumerate() {
                csv_entry!(wtr <- i as f64 * self.sample_period, val.re);
            }
        }
        let _ = python!("plot_freq.py");

        {
            let mut wtr = csv_start!("out.csv");
            csv_entry!(wtr <- "f", "$|F|$");
            for freq in &frequencies.0 {
                csv_entry!(wtr <- freq.0, freq.1);
            }
        }

        let _ = python!("plot_freq.py");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IW_SIZE: usize = 2_i32.pow(10) as usize;
    const SAMPLE_PERIOD: f64 = 0.05;
    const HIT_INTERVAL: i32 = (0.250 / SAMPLE_PERIOD) as i32;
    const HIT_COUNT: u128 = 30;

    #[test]
    fn regular_hits_all() {
        let mut iw = InputWindow::new_with_size(IW_SIZE, SAMPLE_PERIOD);

        // manually create the window VecDeque
        let mut w = VecDeque::from(vec![0; IW_SIZE]);

        // input are regularly spaced hits, at 250ms, thus 5 periods apart
        for i in 0..HIT_COUNT {
            w.push_front(i * (HIT_INTERVAL as u128));
            w.pop_back();
        }

        iw.window = w;

        // plot the decision
        iw.plot_decision();
    }

    #[test]
    fn regular_hits_three() {
        let mut iw = InputWindow::new_with_size(IW_SIZE, SAMPLE_PERIOD);

        // manually create the window VecDeque
        let mut w = VecDeque::from(vec![0; IW_SIZE]);

        // input are regularly spaced hits, at 250ms, thus 5 periods apart
        for i in 0..HIT_COUNT {
            if i % 4 == 0 {
                continue;
            }
            w.push_front(i * (HIT_INTERVAL as u128));
            w.pop_back();
        }

        iw.window = w;

        // plot the decision
        iw.plot_decision();
    }

    #[test]
    fn human_hits_three() {
        let mut iw = InputWindow::new_with_size(IW_SIZE, SAMPLE_PERIOD);

        // manually create the window VecDeque
        let mut w = VecDeque::from(vec![0; IW_SIZE]);

        let offsets = [0, -1, 0, 0, 1, 0];

        // input are regularly spaced hits, at 250ms, thus 5 periods apart
        for i in 0..HIT_COUNT {
            if i % 4 == 0 {
                continue;
            }
            let period = i as i32 * HIT_INTERVAL + offsets[i as usize % 6];
            w.push_front(period as u128);
            w.pop_back();
        }

        iw.window = w;

        // plot the decision
        iw.plot_decision();
    }

    #[test]
    fn human_hits_all() {
        let mut iw = InputWindow::new_with_size(IW_SIZE, SAMPLE_PERIOD);

        // manually create the window VecDeque
        let mut w = VecDeque::from(vec![0; IW_SIZE]);

        let offsets = [0, -1, 0, 0, 1, 0];

        // input are regularly spaced hits, at 250ms, thus 5 periods apart
        for i in 0..HIT_COUNT {
            let period = i as i32 * HIT_INTERVAL + offsets[i as usize % 6];
            w.push_front(period as u128);
            w.pop_back();
        }

        iw.window = w;

        // plot the decision
        iw.plot_decision();
    }
}
