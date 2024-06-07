# Hyper files selected for plotting

## Goal : showing the effect of leaky rate, spectral radius, neuron count, and regularization

-> let's set the neuron count to 95 for the examples
-> let's set the regularization to 0.0001 for the examples
-> then, spectral radius and leaky rate can be adjusted

- low spectral radius, low leaky rate: `hypers_n_95_a0.01_r0.8_l0.0001_count0.csv`
  Notes: moves quite slow, is not able to reach the peaks
- low spectral radius, higher leaky reate: `hypers_n_95_a0.203_r0.8_l0.0001_count1.csv`
  Notes: dies out quickly, is able to generate peaks but they are very low
- high spectral radius, low leaky rate: `hypers_n_95_a0.01_r1.1_l0.0001_count1.csv`
  Notes: hits the peaks quite well but high spectral radius generates unwanted oscillations
- high spectral radius, high leaky rate: `hypers_n_95_a0.203_r1.1_l0.0001_counto.csv`
  Notes: hits the peaks very well, no unwanted osciallations, but high peaks
  could be solved with regularization and a lower spectral radius

solution:
`hypers_n95_a0.100_r0.950_l0.002_count0.csv`
