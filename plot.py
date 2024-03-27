import matplotlib.pyplot as plt
import pandas as pd
import sys
# import scipy.fftpack
# import numpy as np

# if argument is given, it is the path to the csv file
file = "out.csv"
if len(sys.argv) > 1:
    file = sys.argv[1]

df = pd.read_csv(file)

df = df.set_index('t')

plot_columns = [col for col in df.columns if col != "target_0"]

fig, ax = plt.subplots(figsize=(15, 7))

df[plot_columns].plot(ax = ax)
try:
    df["target_0"].plot(ax = ax, color="black", marker="o", markersize=2, label="target")
    plt.legend()
except:
    pass


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
