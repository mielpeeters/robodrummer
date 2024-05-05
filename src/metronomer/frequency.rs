use std::fmt::Display;

#[derive(PartialEq, Copy, Clone, Debug)]
/// (frequency, amount)
pub struct FrequencyComponent(pub f64, pub f64);

impl PartialOrd for FrequencyComponent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl Default for FrequencyComponent {
    fn default() -> Self {
        Self(0.0, 0.0)
    }
}

impl Display for FrequencyComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:3}V @ {:4.2} Hz", self.1, self.0)
    }
}
