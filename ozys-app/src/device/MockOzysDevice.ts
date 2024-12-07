import {
  OzysChannelRealtimeFFT,
  OzysChannelRealtimeReadings,
  OzysDevice,
  OzysDeviceInfo,
} from './OzysDevice'

class MockOzysDevice implements OzysDevice {
    private deviceInfo: OzysDeviceInfo = {
        name: 'Mock OZYS Device',
        id: 'mock-ozys-device',
        model: 'OZYS V3',
        channels: [
            {
                connected: true,
                enabled: true,
                name: 'Channel 1',
                id: 'channel-1',
            },
            {
                connected: true,
                enabled: true,
                name: 'Channel 2',
                id: 'channel-2',
            },
            {
                connected: true,
                enabled: false,
                name: 'Channel 3',
                id: 'channel-3',
            },
            {
                connected: false
            },
        ],
    }
    
    private readingCallbacks: Array<(channelId: string, data: OzysChannelRealtimeReadings) => void> = []
    private fftCallbacks: Array<(channelId: string, data: OzysChannelRealtimeFFT) => void> = []
    private intervalIds: number[] = []
    
    constructor() {
        this.startRealtimeReadings()
        this.startRealtimeFFT()
    }
    
    async rename_device(name: string) {
        this.deviceInfo.name = name
    }
    async rename_channel(channelId: string, name: string) {
        for (const channel of this.deviceInfo.channels) {
            if (channel.connected && channel.id === channelId) {
                channel.name = name
                return
            }
        }
    }
    async control_channel(channelId: string, enabled: boolean) {
        for (const channel of this.deviceInfo.channels) {
            if (channel.connected && channel.id === channelId) {
                channel.enabled = enabled
                return
            }
        }
    }
    async control_recording(enabled: boolean) {
        // Not implemented
    }
    
    async get_device_info() {
        return this.deviceInfo
    }
    
    on_realtime_readings(callback: (channelId: string, data: OzysChannelRealtimeReadings) => void): () => void {
        this.readingCallbacks.push(callback)
        return () => {
            this.readingCallbacks = this.readingCallbacks.filter(cb => cb !== callback)
        }
    }
    
    on_realtime_fft(callback: (channelId: string, data: OzysChannelRealtimeFFT) => void): () => void {
        this.fftCallbacks.push(callback)
        return () => {
            this.fftCallbacks = this.fftCallbacks.filter(cb => cb !== callback)
        }
    }
    
    disconnect(): void {
        this.intervalIds.forEach(id => clearInterval(id))
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
                    const data: OzysChannelRealtimeReadings = { timestamp, readings, readingNoises }
                    for (let k = 0; k < this.readingCallbacks.length; k++) {
                        this.readingCallbacks[k](channel.id, data)
                    }
                }
            }, 10)
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
                    const data: OzysChannelRealtimeFFT = { timestamp, fft0To2k, fft2kTo20k }
                    for (let j = 0; j < this.fftCallbacks.length; j++) {
                        this.fftCallbacks[j](channel.id, data)
                    }
                }
            }, 100)
        )
    }
}
