export interface DeviceType {
  name: string
  id: string
  model: string
  channels: ChannelType[]
}

export interface ChannelType {
  state: string
  enabled: string
  name: string
  id: string
}

export interface RealtimeData {
  readings: number[];
  reading_noises: number[];
  fft_0_to_2k: number[];
  fft_2k_to_20k: number[];
}