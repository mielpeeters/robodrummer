use crate::metronomer::frequency::FrequencyComponent;

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
