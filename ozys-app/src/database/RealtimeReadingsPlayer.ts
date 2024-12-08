import { OzysChannelRealtimeReadings } from '../device/OzysDevice'
import { CircularBuffer } from '../utils/CircularBuffer'
import { Resampler } from './Resampler'

export type PlayerWindowOptions = {
  windowDuration: number
  windowSampleCount: number
  windowStartTimestamp: number
}

export class RealtimeReadingsPlayer {
  private lastTimestamp: number = -1
  private windowDuration: number
  private resampler: Resampler | undefined
  private targetSampleRate: number
  private targetSampleDuration: number
  private outputData: CircularBuffer<{
    timestamp: number
    reading: number
  } | null>

  constructor(
    private channelId: string,
    options: PlayerWindowOptions,
    private onDisplose: () => void,
  ) {
    console.log('RealtimeReadingsPlayer created', channelId, options)
    this.targetSampleRate =
      options.windowSampleCount / (options.windowDuration / 1000)
    this.targetSampleDuration = 1000 / this.targetSampleRate

    this.outputData = new CircularBuffer(options.windowSampleCount)
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
      this.createResampler(data.timestamp)
    } else if (data.timestamp !== this.lastTimestamp + 10) {
      console.log(
        `Gap between last timestamp and current timestamp is not 10ms. Last timestamp: ${this.lastTimestamp}, Current timestamp: ${data.timestamp}`,
        channelId,
      )
      // null means there is a gap in the data
      this.outputData.addLast(null)
      this.createResampler(data.timestamp)
    }

    for (const reading of data.readings) {
      const resampled = this.resampler!.next(reading)
      if (resampled) {
        this.outputData.addLast(resampled)
      }
    }

    this.lastTimestamp = data.timestamp

    // remove data points in outputData that are older than windowDuration
    const lastResampled = this.outputData.peek(-1)
    if (lastResampled) {
      const windowStartTimestamp = lastResampled.timestamp - this.windowDuration
      while (!this.outputData.isEmpty()) {
        let data = this.outputData.peek(0)
        if (data === null || data!.timestamp < windowStartTimestamp) {
          this.outputData.removeFirst()
        } else {
          break
        }
      }
    }
  }

  private createResampler(sourceTimestamp: number) {
    this.resampler = new Resampler(
      sourceTimestamp,
      2000,
      this.targetSampleRate,
      -(sourceTimestamp % this.targetSampleDuration),
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
