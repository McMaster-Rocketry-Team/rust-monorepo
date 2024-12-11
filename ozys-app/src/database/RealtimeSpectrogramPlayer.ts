import { OzysChannelRealtimeFft } from "../device/OzysDevice"
import { CircularBuffer } from "../utils/CircularBuffer"
import { StrainGraphPlayerOptions } from "./RealtimeStrainGraphPlayer"
import { Resampler2D } from "./Resampler2D"

export type SpectrogramPlayerOptions = StrainGraphPlayerOptions & {
  minFrequency: number
  maxFrequency: number
}

export class RealtimeSpectrogramPlayer {
  private lastTimestamp: number = -1
  private windowDuration: number
  private resampler0To2k: Resampler2D | undefined
  private resampler2kTo20k: Resampler2D | undefined
  private targetSampleRate: number
  private targetSampleDuration: number
  private outputData: CircularBuffer<OzysChannelRealtimeFft | null>

  constructor(
    private channelId: string,
    options: SpectrogramPlayerOptions,
    private onDisplose: () => void,
  ) {
    console.log('RealtimeFftPlayer created', channelId, options)
    this.targetSampleRate =
      options.sampleCount / (options.duration / 1000)
    this.targetSampleDuration = 1000 / this.targetSampleRate

    this.outputData = new CircularBuffer(options.sampleCount)
    this.windowDuration = options.duration
  }

  onRealtimeFft(channelId: string, data: OzysChannelRealtimeFft) {
    if (channelId !== this.channelId) {
      return
    }

    if (this.resampler0To2k === undefined || this.resampler2kTo20k === undefined) {
      this.createResampler(data.timestamp)
    } else if (data.timestamp !== this.lastTimestamp + 100) {
      console.log(
        `Gap between last timestamp and current timestamp is not 100ms. Last timestamp: ${this.lastTimestamp}, Current timestamp: ${data.timestamp}`,
        channelId,
      )
      // null means there is a gap in the data
      this.outputData.addLast(null)
      this.createResampler(data.timestamp)
    }

    const resampled0To2k = this.resampler0To2k!.next(data.fft0To2k)
    const resampled2kTo20k = this.resampler2kTo20k!.next(data.fft2kTo20k)

    for (let i = 0; i < resampled0To2k.length; i++) {
      this.outputData.addLast({
        timestamp: resampled0To2k[i].timestamp,
        fft0To2k: resampled0To2k[i].readings,
        fft2kTo20k: resampled2kTo20k[i].readings,
      })
    }

    this.lastTimestamp = data.timestamp
  }

  private createResampler(sourceTimestamp: number) {
    this.resampler0To2k = new Resampler2D(
      200,
      sourceTimestamp,
      10,
      this.targetSampleRate,
      -(sourceTimestamp % this.targetSampleDuration),
    )
    this.resampler2kTo20k = new Resampler2D(
      360,
      sourceTimestamp,
      10,
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
    console.log('RealtimeFftPlayer disposed', this.channelId)
    this.onDisplose()
  }
}