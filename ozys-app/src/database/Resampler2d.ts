import { Butterworth2ndLP, CascadedFilter, Filter, NoopFilter } from './Filter'

export class Resampler2D {
  private filters: Filter[]
  private sourceSampleDuration: number
  private targetSampleDuration: number
  private sourceI = 0
  private sampleI = 0
  private nextSampleTimestamp: number
  private lastReading: Float32Array | undefined

  constructor(
    dataWidth: number,
    private sourceTimestampStart: number,
    sourceSampleRate: number,
    targetSampleRate: number,
    private targetSampleOffset: number,
  ) {
    console.log(
      'sourceSampleRate',
      sourceSampleRate,
      'targetSampleRate',
      targetSampleRate,
    )

    this.filters = new Array(dataWidth)
    if (sourceSampleRate > targetSampleRate) {
      for (let i = 0; i < dataWidth; i++) {
        this.filters[i] = new CascadedFilter([
          new Butterworth2ndLP(sourceSampleRate, targetSampleRate / 4),
          new Butterworth2ndLP(sourceSampleRate, targetSampleRate / 4),
        ])
      }
    } else {
      for (let i = 0; i < dataWidth; i++) {
        this.filters[i] = new NoopFilter()
      }
    }

    this.sourceSampleDuration = 1000 / sourceSampleRate
    this.targetSampleDuration = 1000 / targetSampleRate

    this.nextSampleTimestamp = this.targetSampleOffset
    if (this.nextSampleTimestamp < 0) {
      this.sampleI++
      this.nextSampleTimestamp += this.targetSampleDuration
    }
  }

  next(readings: Float32Array): Array<{
    timestamp: number
    readings: Float32Array
  }> {
    let filteredReadings = new Float32Array(readings.length)
    for (let i = 0; i < readings.length; i++) {
      const filter = this.filters[i]
      filteredReadings[i] = filter.process(readings[i])
    }

    if (this.lastReading === undefined) {
      // let the filter reach steady state
      for (let i = 0; i < readings.length; i++) {
        const filter = this.filters[i]
        const reading = readings[i]

        while (Math.abs(filteredReadings[i] - reading) / reading > 0.01) {
          filteredReadings[i] = filter.process(reading)
        }
      }

      this.lastReading = filteredReadings
      return []
    }

    const interpolatableStart = (this.sourceI - 1) * this.sourceSampleDuration
    const interpolatableEnd = interpolatableStart + this.sourceSampleDuration

    this.sourceI++

    const results = []
    while (
      this.nextSampleTimestamp >= interpolatableStart &&
      this.nextSampleTimestamp <= interpolatableEnd
    ) {
      const t =
        (this.nextSampleTimestamp - interpolatableStart) /
        this.sourceSampleDuration
      results.push({
        timestamp: this.sourceTimestampStart + this.nextSampleTimestamp,
        readings: this.lerp2d(this.lastReading, filteredReadings, t),
      })

      this.sampleI++
      this.nextSampleTimestamp =
        this.sampleI * this.targetSampleDuration + this.targetSampleOffset
    }

    this.lastReading = filteredReadings
    return results
  }

  private lerp2d(a: Float32Array, b: Float32Array, t: number) {
    const result = new Float32Array(a.length)
    for (let i = 0; i < a.length; i++) {
      result[i] = a[i] + t * (b[i] - a[i])
    }
    return result
  }
}
