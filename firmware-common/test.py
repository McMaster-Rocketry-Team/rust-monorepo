import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from scipy.fft import fft, fftfreq

# Read the CSV file
file_path = 'test-data/106.baro_tier_1.csv'  # Replace with your actual file path
data = pd.read_csv(file_path)

# Extract the pressure values
pressure_values = data['pressure'].dropna().values

# Perform FFT
N = len(pressure_values)
T = 1.0 / 200.0  # Sample spacing (you can adjust this based on your data)
yf = fft(pressure_values)
xf = fftfreq(N, T)[:N//2]

# Filter out frequencies below 20 Hz
mask = xf >= 20
xf_filtered = xf[mask]
yf_filtered = 2.0/N * np.abs(yf[0:N//2])[mask]

# Plot the FFT result
plt.figure()
plt.plot(xf_filtered, yf_filtered)
plt.title('FFT of Pressure Values (Frequencies >= 20 Hz)')
plt.xlabel('Frequency (Hz)')
plt.ylabel('Amplitude')
plt.grid()
plt.show()