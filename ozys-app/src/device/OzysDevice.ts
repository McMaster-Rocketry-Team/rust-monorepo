export type OzysChannelState =
  | {
      connected: false
    }
  | {
      connected: true
      enabled: boolean
      name: string
      id: string
    }

export type OzysDeviceInfo = {
  name: string
  id: string
  model: string
  channels: OzysChannelState[]
}

// The device sends this data to the app every 10ms
// The readings list contains 20 samples during the last 10ms
// Sampling rate is 2kHz
export type OzysChannelRealtimeReadings = {
  timestamp: number

  // Absolute reading values
  // Unit is 1, e.g. 0.01 = 1% of length change
  // 20 readings long, interval between readings is 0.5ms
  readings: Float32Array

  // Standard deviation of all the samples inside the 10ms interval
  // 20 values long, one value for each reading
  readingNoises: Float32Array
}

// The device sends this data to the app every 100ms
// The data contains FFT during the last 100ms
export type OzysChannelRealtimeFFT = {
  timestamp: number

  // FFT for frequencies below 2kHz, with 10Hz resolution
  // 200 values long
  // e.g. fft_0_to_2k[0] is the power of the 0-10Hz band in the last 100ms
  //      fft_0_to_2k[1] is the power of the 10-20Hz band in the last 100ms
  fft0To2k: Float32Array

  // FFT for frequencies between 2kHz and 20kHz, with 50Hz resolution
  // 360 values long
  // e.g. fft_2k_to_20k[0] is the power of the 2k-2.05k band in the last 100ms
  fft2kTo20k: Float32Array
}

export interface OzysDevice {
  get_device_info(): Promise<OzysDeviceInfo>
  rename_device(name: string): Promise<void>
  rename_channel(channelId: string, name: string): Promise<void>
  control_channel(channelId: string, enabled: boolean): Promise<void>
  control_recording(enabled: boolean): Promise<void>

  on_realtime_readings(
    callback: (channelId: string, data: OzysChannelRealtimeReadings) => void,
  ): () => void

  on_realtime_fft(
    callback: (channelId: string, data: OzysChannelRealtimeFFT) => void,
  ): () => void

  disconnect(): void
}
