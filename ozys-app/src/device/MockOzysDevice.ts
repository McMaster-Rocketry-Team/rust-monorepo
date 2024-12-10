import {
  OzysChannelRealtimeFft,
  OzysChannelRealtimeReadings,
  OzysDevice,
} from './OzysDevice'

export class MockOzysDevice extends OzysDevice {
  private static i = 1
  private intervalIds: number[] = []

  constructor() {
    super({
      name: `Mock OZYS ${MockOzysDevice.i}`,
      id: `mock-ozys-${MockOzysDevice.i}`,
      model: 'OZYS V3',
      isRecording: false,
      channels: [
        {
          connected: true,
          enabled: true,
          name: 'Channel 1',
          id: `mock-ozys-${MockOzysDevice.i}-channel-1`,
        },
        {
          connected: true,
          enabled: true,
          name: 'Channel 2',
          id: `mock-ozys-${MockOzysDevice.i}-channel-2`,
        },
        {
          connected: true,
          enabled: false,
          name: 'Channel 3',
          id: `mock-ozys-${MockOzysDevice.i}-channel-3`,
        },
        {
          connected: false,
        },
      ],
    })

    this.startRealtimeReadings()
    this.startRealtimeFFT()

    MockOzysDevice.i++
  }

  disconnect(): void {
    this.intervalIds.forEach((id) => clearInterval(id))
  }

  async untilAligned(): Promise<void> {
    const remainder = 100 - (Date.now() % 100)
    await new Promise((resolve) => setTimeout(resolve, remainder))
  }

  private async startRealtimeReadings() {
    const generateData = (timestamp: number) => {
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
            Math.sin(2 * Math.PI * 60 * (timestamp + j * 0.5) * 0.001) * 0.5
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
    }

    await this.untilAligned()
    let lastTimestamp: undefined | number
    this.intervalIds.push(
      setInterval(() => {
        let timestamp = Date.now()
        timestamp = Math.round(timestamp / 10) * 10
        if (!lastTimestamp) {
          lastTimestamp = timestamp
          return
        }

        for (let t = lastTimestamp; t < timestamp; t += 10) {
          generateData(t)
        }
        lastTimestamp = timestamp
      }, 10),
    )
  }

  private async startRealtimeFFT() {
    const generateData = (timestamp: number) => {
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
    }

    await this.untilAligned()
    let lastTimestamp: undefined | number
    this.intervalIds.push(
      setInterval(() => {
        let timestamp = Date.now()
        timestamp = Math.round(timestamp / 100) * 100
        if (!lastTimestamp) {
          lastTimestamp = timestamp
          return
        }

        for (let t = lastTimestamp; t < timestamp; t += 100) {
          generateData(t)
        }
        lastTimestamp = timestamp
      }, 100),
    )
  }
}
