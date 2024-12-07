import {
  OzysChannelRealtimeFft,
  OzysChannelRealtimeReadings,
  OzysDevice,
} from './OzysDevice'

export class MockOzysDevice extends OzysDevice {
  private intervalIds: number[] = []

  constructor() {
    super({
      name: 'Mock OZYS Device',
      id: crypto.randomUUID(),
      model: 'OZYS V3',
      isRecording: false,
      channels: [
        {
          connected: true,
          enabled: true,
          name: 'Channel 1',
          id: crypto.randomUUID(),
        },
        {
          connected: true,
          enabled: true,
          name: 'Channel 2',
          id: crypto.randomUUID(),
        },
        {
          connected: true,
          enabled: false,
          name: 'Channel 3',
          id: crypto.randomUUID(),
        },
        {
          connected: false,
        },
      ],
    })

    this.startRealtimeReadings()
    this.startRealtimeFFT()
  }

  disconnect(): void {
    this.intervalIds.forEach((id) => clearInterval(id))
  }

  async untilAligned(): Promise<void> {
    const remainder = 100 - (Date.now() % 100)
    await new Promise((resolve) => setTimeout(resolve, remainder))
  }

  private async startRealtimeReadings() {
    await this.untilAligned()
    this.intervalIds.push(
      setInterval(() => {
        let timestamp = Date.now()
        timestamp = Math.round(timestamp / 10) * 10
        for (const channel of this.deviceInfo.channels) {
          if (!channel.connected || !channel.enabled) {
            continue
          }
          const readings = new Float32Array(20)
          for (let j = 0; j < 20; j++) {
            readings[j] = Math.sin(
              2 * Math.PI * 5 * (timestamp + j * 0.5) * 0.001,
            )
            readings[j] +=
              Math.sin(2 * Math.PI * 60 * (timestamp + j * 0.5) * 0.001) * 0.1
          }
          const readingNoises = new Float32Array(20).fill(0)
          const data: OzysChannelRealtimeReadings = {
            timestamp,
            readings,
            noises: readingNoises,
          }
          for (let k = 0; k < this.readingCallbacks.length; k++) {
            this.readingCallbacks[k](channel.id, data)
          }
        }
      }, 10),
    )
  }

  private async startRealtimeFFT() {
    await this.untilAligned()
    this.intervalIds.push(
      setInterval(() => {
        let timestamp = Date.now()
        timestamp = Math.round(timestamp / 100) * 100
        for (const channel of this.deviceInfo.channels) {
          if (!channel.connected || !channel.enabled) {
            continue
          }
          const fft0To2k = new Float32Array(200).fill(0)
          const fft2kTo20k = new Float32Array(360).fill(0)
          const data: OzysChannelRealtimeFft = {
            timestamp,
            fft0To2k,
            fft2kTo20k,
          }
          for (let j = 0; j < this.fftCallbacks.length; j++) {
            this.fftCallbacks[j](channel.id, data)
          }
        }
      }, 100),
    )
  }
}
