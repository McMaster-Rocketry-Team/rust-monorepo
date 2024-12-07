import { makeAutoObservable } from 'mobx'
import {
  OzysChannelRealtimeFFT,
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
    makeAutoObservable(this)
  }

  disconnect(): void {
    this.intervalIds.forEach((id) => clearInterval(id))
  }

  private startRealtimeReadings() {
    this.intervalIds.push(
      setInterval(() => {
        const timestamp = Date.now()
        for (const channel of this.deviceInfo.channels) {
          if (!channel.connected || !channel.enabled) {
            continue
          }
          const readings = new Float32Array(20)
          for (let j = 0; j < 20; j++) {
            readings[j] = Math.sin((timestamp + j * 0.5) * 0.001)
          }
          const readingNoises = new Float32Array(20).fill(0)
          const data: OzysChannelRealtimeReadings = {
            timestamp,
            readings,
            readingNoises,
          }
          for (let k = 0; k < this.readingCallbacks.length; k++) {
            this.readingCallbacks[k](channel.id, data)
          }
        }
      }, 10),
    )
  }

  private startRealtimeFFT() {
    this.intervalIds.push(
      setInterval(() => {
        const timestamp = Date.now()
        for (const channel of this.deviceInfo.channels) {
          if (!channel.connected || !channel.enabled) {
            continue
          }
          const fft0To2k = new Float32Array(200).fill(0)
          const fft2kTo20k = new Float32Array(360).fill(0)
          const data: OzysChannelRealtimeFFT = {
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
