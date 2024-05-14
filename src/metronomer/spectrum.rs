use crate::metronomer::frequency::FrequencyComponent;

#[derive(Default, Clone)]
pub struct Spectrum(pub Vec<FrequencyComponent>);

impl Spectrum {
    pub fn band_pass(&self, low: f64, high: f64) -> Spectrum {
        self.into_iter()
            .filter(|x| x.0 < high)
            .filter(|x| x.0 > low)
            .collect()
    }

    pub fn high_pass(&self, cutoff: f64) -> Spectrum {
        self.into_iter().filter(|x| x.0 > cutoff).collect()
    }

    pub fn low_pass(&self, cutoff: f64) -> Spectrum {
        self.into_iter().filter(|x| x.0 < cutoff).collect()
    }

    pub fn normalize(&mut self) {
        let max = self.0.iter().map(|x| x.1).fold(0.0_f64, |a, b| a.max(b));

        for freq in self.0.iter_mut() {
            freq.1 /= max;
        }
    }

    pub fn spectral_sum(&mut self) {
        let mut freqs = Vec::<f64>::new();

        let mut spectrum = self.clone();
        spectrum.normalize();

        for freq in self.into_iter() {
            freqs.push(freq.0);
        }

        freqs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        freqs.dedup();

        // get smallest difference between frequencies
        let min_diff = freqs
            .windows(2)
            .map(|x| x[1] - x[0])
            .fold(f64::INFINITY, |a, b| a.min(b));

        // start adding the frequency multiples to the spectrum
        for l in 2..5 {
            for freq in self.into_iter() {
                let lower_harmonic = freq.0 / l as f64;
                let upper_harmonic = freq.0 * l as f64;

                // update the spectrum by adding the value to the lower and upper

                let component = spectrum
                    .0
                    .iter_mut()
                    .find(|x| (x.0 - lower_harmonic).abs() <= min_diff / 2.0);
                if let Some(component) = component {
                    component.1 += freq.1 / 2.0;
                }

                let component = spectrum
                    .0
                    .iter_mut()
                    .find(|x| (x.0 - upper_harmonic).abs() <= min_diff / 2.0);

                if let Some(component) = component {
                    component.1 += freq.1 / 2.0;
                }
            }
        }
        *self = spectrum;
    }
}

pub struct SpectrumIterator<'a> {
    spectrum: &'a Spectrum,
    index: usize,
}

impl<'a> IntoIterator for &'a Spectrum {
    type Item = FrequencyComponent;

    type IntoIter = SpectrumIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SpectrumIterator {
            spectrum: self,
            index: 0,
        }
    }
}

impl<'a> Iterator for SpectrumIterator<'a> {
    type Item = FrequencyComponent;

    fn next(&mut self) -> Option<FrequencyComponent> {
        if self.spectrum.0.len() > self.index {
            let res = Some(self.spectrum.0[self.index]);
            self.index += 1;
            res
        } else {
            None
        }
    }
}

impl FromIterator<FrequencyComponent> for Spectrum {
    fn from_iter<T: IntoIterator<Item = FrequencyComponent>>(iter: T) -> Self {
        let res: Vec<FrequencyComponent> = iter.into_iter().collect();
        Spectrum(res)
    }
}
