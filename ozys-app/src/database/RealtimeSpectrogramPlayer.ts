import { OzysChannelRealtimeFft } from '../device/OzysDevice'
import { CircularBuffer } from '../utils/CircularBuffer'
import { StrainGraphPlayerOptions } from './RealtimeStrainGraphPlayer'
import { Resampler } from './Resampler'
import { Resampler2D } from './Resampler2D'

export type SpectrogramPlayerOptions = StrainGraphPlayerOptions & {
  minFrequency: number
  maxFrequency: number
  frequencySampleCount: number
}

export class RealtimeSpectrogramPlayer {
  private lastTimestamp: number = -1
  private timeAxisResampler: Resampler2D | undefined
  private outputData: CircularBuffer<{
    timestamp: number
    fft: Float32Array
  } | null> // FIXME

  // time axis, sample / second
  private targetSampleRate: number

  // time axis, ms / sample
  private targetSampleDuration: number

  private targetMinFrequency: number

  // frequency axis, sample / Hz
  private targetFrequencySampleRate: number

  private frequencyAxisResampleBuffer: Float32Array

  constructor(
    private channelId: string,
    options: SpectrogramPlayerOptions,
    private onDisplose: () => void,
  ) {
    console.log('RealtimeFftPlayer created', channelId, options)
    this.targetSampleRate = options.sampleCount / (options.duration / 1000)
    this.targetSampleDuration = 1000 / this.targetSampleRate

    this.targetMinFrequency = options.minFrequency
    this.targetFrequencySampleRate =
      options.frequencySampleCount /
      (options.maxFrequency - options.minFrequency)
    this.frequencyAxisResampleBuffer = new Float32Array(
      options.frequencySampleCount,
    )

    this.outputData = new CircularBuffer(options.sampleCount)
  }

  onRealtimeFft(channelId: string, data: OzysChannelRealtimeFft) {
    if (channelId !== this.channelId) {
      return
    }

    if (this.timeAxisResampler === undefined) {
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

    this.resampleFrequencyAxis(data)

    const timeAxisResampled = this.timeAxisResampler!.next(
      this.frequencyAxisResampleBuffer,
    )

    for (let i = 0; i < timeAxisResampled.length; i++) {
      this.outputData.addLast({
        timestamp: timeAxisResampled[i].timestamp,
        fft: timeAxisResampled[i].readings,
      })
    }

    this.lastTimestamp = data.timestamp
  }

  private hzToBufferI(hz: number) {
    return Math.floor(
      (hz - this.targetMinFrequency) * this.targetFrequencySampleRate,
    )
  }

  private resampleFrequencyAxis(data: OzysChannelRealtimeFft): Float32Array {
    // resample 0-2k Hz
    let frequencyAxisResampler = new Resampler(
      0,
      1 / 10,
      this.targetFrequencySampleRate,
      0,
    )

    let lastBufferI = -1
    for (let i = 0; i < data.fft0To2k.length; i++) {
      const resampledFrequencies = frequencyAxisResampler.next(data.fft0To2k[i])
      for (const {
        timestamp: mFrequency,
        reading: amplitude,
      } of resampledFrequencies) {
        const bufferI = this.hzToBufferI(mFrequency / 1000)
        if (bufferI >= this.frequencyAxisResampleBuffer.length) {
          return this.frequencyAxisResampleBuffer
        } else if (bufferI < 0) {
          continue
        }
        this.frequencyAxisResampleBuffer[bufferI] = amplitude
        if (bufferI - lastBufferI > 1) {
          console.warn('Gap in frequency axis', bufferI, lastBufferI)
        }
        lastBufferI = bufferI
      }
    }
    const _2kBufferI = this.hzToBufferI(2000)
    while (_2kBufferI - lastBufferI > 0) {
      this.frequencyAxisResampleBuffer[lastBufferI + 1] =
        this.frequencyAxisResampleBuffer[lastBufferI]
      lastBufferI++
    }

    // resample 2k-20k Hz
    frequencyAxisResampler = new Resampler(
      2000 * 1000,
      1 / 50,
      this.targetFrequencySampleRate,
      0,
    )

    for (let i = 0; i < data.fft2kTo20k.length; i++) {
      const resampledFrequencies = frequencyAxisResampler.next(
        data.fft2kTo20k[i],
      )
      for (const {
        timestamp: mFrequency,
        reading: amplitude,
      } of resampledFrequencies) {
        const bufferI = this.hzToBufferI(mFrequency / 1000)
        if (bufferI >= this.frequencyAxisResampleBuffer.length) {
          return this.frequencyAxisResampleBuffer
        } else if (bufferI < 0) {
          continue
        }
        this.frequencyAxisResampleBuffer[bufferI] = amplitude
        if (bufferI - lastBufferI > 1) {
          console.warn('Gap in frequency axis', bufferI, lastBufferI)
        }
        lastBufferI = bufferI
      }
    }
    const _20kBufferI = this.hzToBufferI(20000)
    while (_20kBufferI - lastBufferI > 0) {
      this.frequencyAxisResampleBuffer[lastBufferI + 1] =
        this.frequencyAxisResampleBuffer[lastBufferI]
      lastBufferI++
    }

    return this.frequencyAxisResampleBuffer
  }

  private createResampler(sourceTimestamp: number) {
    this.timeAxisResampler = new Resampler2D(
      this.frequencyAxisResampleBuffer.length,
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
