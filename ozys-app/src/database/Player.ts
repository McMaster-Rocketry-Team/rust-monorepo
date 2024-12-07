import { OzysChannelRealtimeReadings } from '../device/OzysDevice'
import { RingBuffer } from 'ring-buffer-ts'
import { Resampler } from './Resampler'

export class RealtimeReadingsPlayer {
  private lastTimestamp: number = -1
  private resampler: Resampler | undefined
  private outputData: RingBuffer<{
    timestamp: number
    reading: number
  }> = new RingBuffer(200)

  constructor(
    private deviceId: string,
    private channelId: string,
    private sampleRate: number,
    private targetSampleOffset: number,
  ) {}

  async onRealtimeReadings(
    deviceId: string,
    channelId: string,
    data: OzysChannelRealtimeReadings,
  ) {
    if (deviceId !== this.deviceId || channelId !== this.channelId) {
      return
    }

    if (
      this.resampler === undefined ||
      data.timestamp !== this.lastTimestamp + 10
    ) {
      this.resampler = new Resampler(
        2000,
        this.sampleRate,
        this.targetSampleOffset,
      )
    }

    for (const reading of data.readings) {
      const resampled = this.resampler.next(reading)
      if (resampled) {
        this.outputData.add(resampled)
      }
    }
  }

  getNewData() {
    const result = this.outputData.toArray()
    this.outputData.clear()
    return result
  }
}
