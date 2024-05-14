import matplotlib.pyplot as plt
import pandas as pd
import sys
# import scipy.fftpack
# import numpy as np

# if argument is given, it is the path to the csv file
file = "out.csv"
if len(sys.argv) > 1:
    file = sys.argv[1]

output = None
if len(sys.argv) > 2:
    # this means we'll write the plot to a file
    output = sys.argv[2]

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


if output:
    plt.savefig(output, format='svg', dpi=300)

else:
    plt.show()

