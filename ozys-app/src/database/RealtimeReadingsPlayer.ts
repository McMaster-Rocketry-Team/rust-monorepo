import { OzysChannelRealtimeReadings } from '../device/OzysDevice'
import { RingBuffer } from 'ring-buffer-ts'
import { Resampler } from './Resampler'

export type PlayerWindowOptions = {
  windowDuration: number
  windowSampleCount: number
  windowStartTimestamp: number
}

export class RealtimeReadingsPlayer {
  private lastTimestamp: number = -1
  private windowDuration: number
  private sampleRate: number
  private targetSampleOffset: number
  private resampler: Resampler | undefined
  private outputData: RingBuffer<{
    timestamp: number
    reading: number
  } | null>

  constructor(
    private channelId: string,
    options: PlayerWindowOptions,
    private onDisplose: () => void,
  ) {
    console.log('RealtimeReadingsPlayer created', channelId)
    this.sampleRate = options.windowSampleCount / (options.windowDuration / 1000)
    const sampleDuration = 1000 / this.sampleRate
    this.targetSampleOffset = options.windowStartTimestamp % sampleDuration

    this.outputData = new RingBuffer(options.windowSampleCount)
    this.windowDuration = options.windowDuration
  }

  async onRealtimeReadings(
    channelId: string,
    data: OzysChannelRealtimeReadings,
  ) {
    if (channelId !== this.channelId) {
      return
    }

    if (this.resampler === undefined) {
      this.createResampler()
    } else if (data.timestamp !== this.lastTimestamp + 10) {
      console.log(
        `Gap between last timestamp and current timestamp is not 10ms. Last timestamp: ${this.lastTimestamp}, Current timestamp: ${data.timestamp}`,
        channelId,
      )
      // null means there is a gap in the data
      this.outputData.add(null)
      this.createResampler()
    }

    for (const reading of data.readings) {
      const resampled = this.resampler!.next(reading)
      if (resampled) {
        this.outputData.add(resampled)
      }
    }

    this.lastTimestamp = data.timestamp

    // remove data points in outputData that are older than windowDuration
    const lastResampled = this.outputData.getLast()
    if (lastResampled) {
      const windowStartTimestamp = lastResampled.timestamp - this.windowDuration
      while (!this.outputData.isEmpty()) {
        let data = this.outputData.getFirst()
        if (data === null || data!.timestamp < windowStartTimestamp) {
          this.outputData.removeFirst()
        } else {
          break
        }
      }
    }
  }

  private createResampler() {
    this.resampler = new Resampler(
      2000,
      this.sampleRate,
      this.targetSampleOffset,
    )
  }

  getNewData() {
    const result = this.outputData.toArray()
    this.outputData.clear()
    return result
  }

  dispose() {
    console.log('RealtimeReadingsPlayer disposed', this.channelId)
    this.onDisplose()
  }
}
