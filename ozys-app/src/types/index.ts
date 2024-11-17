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
