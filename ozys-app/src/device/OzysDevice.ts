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

export type OzysChannelRealtimeData = {
  timestamp: number
  readings: number[]
  reading_noises: number[]
  fft_0_to_2k: number[]
  fft_2k_to_20k: number[]
}

export interface OzysDevice {
  get_device_info(): Promise<OzysDeviceInfo>
  rename_device(name: string): Promise<void>
  rename_channel(channelId: string, name: string): Promise<void>
  control_channel(channelId: string, enabled: boolean): Promise<void>
  control_recording(enabled: boolean): Promise<void>

  on_realtime_data(
    callback: (channelId: string, data: OzysChannelRealtimeData) => void,
  ): () => void
}
