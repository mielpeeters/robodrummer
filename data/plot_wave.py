import matplotlib.pyplot as plt   
import pandas as pd
import numpy as np

from matplotlib import rcParams
rcParams['font.family'] = 'sans-serif'
rcParams['font.sans-serif'] = ['IBM Plex Sans']

# Load the data from a CSV file
df = pd.read_csv('test_waves.csv')

# Calculate the average elapsed time for each wave_type
averages = df.groupby('wave_type')['elapsed'].mean()

# Plot the data
fig, ax = plt.subplots()

# Plot each wave_type in a different horizontal column
for wave_type, group in df.groupby('wave_type'):
    ax.scatter(group['wave_type'], group['elapsed'], label=wave_type)

# Plot the average elapsed time for each wave_type
for wave_type, average in averages.items():
    ax.scatter(wave_type, average, color='black', marker='x')

# Add legend
ax.legend()

# Set labels and title
ax.set_xlabel('Wave Type')
ax.set_ylabel('Delay Time [$s$]')
ax.set_title('Scatter Plot of Elapsed Time by Wave Type')

# Display the plot
plt.show()
