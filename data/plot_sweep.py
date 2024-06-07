import matplotlib.pyplot as plt   
import pandas as pd
import numpy as np

from matplotlib import rcParams
rcParams['font.family'] = 'sans-serif'
rcParams['font.sans-serif'] = ['IBM Plex Sans']

df = pd.read_csv('sweep-18-04-2024.csv')


# Plot the data
fig, ax = plt.subplots()

# Create the scatter plot
df = df[df['beat_time'] < 0.55]
ax = df.plot.scatter(ax=ax, x='beat_time', y='elapsed', c='DarkBlue')

# Calculate the best-fit line
a, b = np.polyfit(df['beat_time'], df['elapsed'], 1)

# Add the best-fit line to the plot
ax.plot(df['beat_time'], a*df['beat_time'] + b, color='red')

# Set labels and title
ax.set_xlabel('Inter-Beat Time [$s$]')
ax.set_ylabel('Delay Time [$s$]')
ax.set_title('Scatter Plot of Elapsed Time by Wave Type')

# Display the plot
plt.show()

