import matplotlib.pyplot as plt
import pandas as pd
import scipy.fftpack
# import numpy as np

df = pd.read_csv('out.csv')

df.set_index('t').plot()

# plt.savefig("plot.png")
plt.show()

fig = plt.figure()

# nw = df["nw_0"]
# N = len(nw)
# dt = 1

# acc = nw.values.flatten()

# fft = scipy.fftpack.rfft(acc) * dt
# freq = scipy.fftpack.rfftfreq(N, d=dt)

# FFT = abs(fft)

# plt.plot(freq, FFT)

# nw = df["target_0"]
# N = len(nw)
# dt = 1

# acc = nw.values.flatten()

# fft = scipy.fftpack.rfft(acc) * dt
# freq = scipy.fftpack.rfftfreq(N, d=dt)

# FFT = abs(fft)

# plt.plot(freq, FFT)

# plt.show()
