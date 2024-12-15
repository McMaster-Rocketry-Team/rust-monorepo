import { expect, test } from 'vitest'
import { RealtimeSpectrogramPlayer } from './RealtimeSpectrogramPlayer'
import { OzysChannelRealtimeFft } from '../device/OzysDevice'

test('Resample frequency axis 0-20khz', () => {
  const player = new RealtimeSpectrogramPlayer(
    'channelId',
    {
      duration: 10,
      sampleCount: 10,
      startTimestamp: 0,
      minFrequency: 0,
      maxFrequency: 20000,
      frequencySampleCount: 20000,
    },
    () => {},
  )

  const data: OzysChannelRealtimeFft = {
    timestamp: 0,
    fft0To2k: new Float32Array(200),
    fft2kTo20k: new Float32Array(360),
  }
  data.fft0To2k.fill(1)
  data.fft2kTo20k.fill(2)

  const resampledData = player['resampleFrequencyAxis'](data)
  let ones = 0
  let twos = 0
  for (let i = 0; i < resampledData.length; i++) {
    if (resampledData[i] === 1) {
      ones++
    } else if (resampledData[i] === 2) {
      twos++
    }
  }
  expect(ones).toBe(2000)
  expect(twos).toBe(20000 - 2000)
})

test('Resample frequency axis 1k-3khz', () => {
  const player = new RealtimeSpectrogramPlayer(
    'channelId',
    {
      duration: 10,
      sampleCount: 10,
      startTimestamp: 0,
      minFrequency: 1000,
      maxFrequency: 3000,
      frequencySampleCount: 4000,
    },
    () => {},
  )

  const data: OzysChannelRealtimeFft = {
    timestamp: 0,
    fft0To2k: new Float32Array(200),
    fft2kTo20k: new Float32Array(360),
  }
  data.fft0To2k.fill(1)
  data.fft2kTo20k.fill(2)

  const resampledData = player['resampleFrequencyAxis'](data)
  let ones = 0
  let twos = 0
  for (let i = 0; i < resampledData.length; i++) {
    if (resampledData[i] === 1) {
      ones++
    } else if (resampledData[i] === 2) {
      twos++
    }
  }
  expect(ones).toBe(2000)
  expect(twos).toBe(2000)
})
