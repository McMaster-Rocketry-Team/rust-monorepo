export class Resampler {
  private filter: Filter
  private sourceSampleDuration: number
  private targetSampleDuration: number
  private sourceI = 0
  private sampleI = 0
  private nextSampleTimestamp: number
  private lastReading: number | undefined

  constructor(
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

    if (sourceSampleRate > targetSampleRate) {
      this.filter = new Butterworth2ndLP(sourceSampleRate, targetSampleRate / 4)
    } else {
      this.filter = new NoopFilter()
    }

    this.sourceSampleDuration = 1000 / sourceSampleRate
    this.targetSampleDuration = 1000 / targetSampleRate

    this.nextSampleTimestamp = this.targetSampleOffset
    if (this.nextSampleTimestamp < 0) {
      this.sampleI++
      this.nextSampleTimestamp += this.targetSampleDuration
    }
  }

  next(reading: number): Array<{
    timestamp: number
    reading: number
  }> {
    let filteredReading = this.filter.process(reading)
    if (this.lastReading === undefined) {
      // let the filter reach steady state
      while (Math.abs(filteredReading - reading) / reading > 0.01) {
        filteredReading = this.filter.process(reading)
      }

      this.lastReading = filteredReading
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
        reading: this.lerp(this.lastReading, filteredReading, t),
      })

      this.sampleI++
      this.nextSampleTimestamp =
        this.sampleI * this.targetSampleDuration + this.targetSampleOffset
    }

    this.lastReading = filteredReading
    return results
  }

  private lerp(a: number, b: number, t: number) {
    return a + t * (b - a)
  }
}

interface Filter {
  process(input: number): number
}

class NoopFilter implements Filter {
  process(input: number): number {
    return input
  }
}

// 2nd order Butterworth low-pass filter
class Butterworth2ndLP implements Filter {
  private b0: number
  private b1: number
  private b2: number
  private a1: number
  private a2: number

  // Private a0 to store the a0 coefficient for normalization
  private a0: number

  private x1: number // x[n-1]
  private x2: number // x[n-2]
  private y1: number // y[n-1]
  private y2: number // y[n-2]

  constructor(samplingRate: number, cutoffFrequency: number) {
    // Calculate the normalized frequency
    const omega = (2 * Math.PI * cutoffFrequency) / samplingRate
    const cosOmega = Math.cos(omega)
    const sinOmega = Math.sin(omega)

    // Quality factor for Butterworth filter
    const Q = 1 / Math.sqrt(2)

    // Calculate alpha
    const alpha = sinOmega / (2 * Q)

    // Calculate filter coefficients
    this.b0 = (1 - cosOmega) / 2
    this.b1 = 1 - cosOmega
    this.b2 = (1 - cosOmega) / 2
    this.a0 = 1 + alpha
    this.a1 = -2 * cosOmega
    this.a2 = 1 - alpha

    // Normalize the coefficients
    this.b0 /= this.a0
    this.b1 /= this.a0
    this.b2 /= this.a0
    this.a1 /= this.a0
    this.a2 /= this.a0

    // Initialize previous input and output samples
    this.x1 = 0
    this.x2 = 0
    this.y1 = 0
    this.y2 = 0
  }

  public process(input: number): number {
    // Apply the difference equation
    const output =
      this.b0 * input +
      this.b1 * this.x1 +
      this.b2 * this.x2 -
      this.a1 * this.y1 -
      this.a2 * this.y2

    // Update the stored samples
    this.x2 = this.x1
    this.x1 = input
    this.y2 = this.y1
    this.y1 = output

    return output
  }
}
