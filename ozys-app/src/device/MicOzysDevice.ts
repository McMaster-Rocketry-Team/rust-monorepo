import { OzysDevice } from './OzysDevice'

export class MicOzysDevice extends OzysDevice {
  private static i = 1
  private audioContext: AudioContext | undefined
  private stream: MediaStream | undefined
  private scriptProcessor: ScriptProcessorNode | undefined

  constructor() {
    super({
      name: `Microphone ${MicOzysDevice.i}`,
      id: `mic-ozys-${MicOzysDevice.i}`,
      model: 'OZYS V3',
      isRecording: false,
      channels: [
        {
          connected: true,
          enabled: true,
          name: 'Channel 1',
          id: `mic-ozys-${MicOzysDevice.i}-channel-1`,
        },
        {
          connected: false,
        },
        {
          connected: false,
        },
        {
          connected: false,
        },
      ],
    })

    MicOzysDevice.i++

    this.init()
  }

  private async init() {
    // Step 1: Access the microphone
    this.stream = await navigator.mediaDevices.getUserMedia({ audio: true })

    // Step 2: Create an AudioContext
    this.audioContext = new AudioContext()

    // Step 3: Create a MediaStreamSource from the microphone input
    const source = this.audioContext.createMediaStreamSource(this.stream)

    // Step 4: Create an AnalyserNode
    const analyser = this.audioContext.createAnalyser()
    analyser.fftSize = 4096
    const bufferLength = analyser.frequencyBinCount
    const frequencyData = new Float32Array(bufferLength)

    // Step 5: Create a ScriptProcessorNode
    const bufferSize = 4096
    this.scriptProcessor = this.audioContext.createScriptProcessor(
      bufferSize,
      1,
      1,
    )

    // Connect nodes: source -> analyser -> scriptProcessor -> destination
    source.connect(analyser)
    analyser.connect(this.scriptProcessor)
    this.scriptProcessor.connect(this.audioContext.destination)

    // Step 6: Handle audio processing in the onaudioprocess event
    let lastTimestamp: undefined | number
    this.scriptProcessor.onaudioprocess = (e) => {
      // Get FFT data
      // length 2048
      analyser.getFloatFrequencyData(frequencyData)

      // length 4096
      const timeDomainData = e.inputBuffer.getChannelData(0)

      // Process the data
      const timestamp = Math.round(Date.now() / 100) * 100
      if (lastTimestamp === undefined) {
        lastTimestamp = timestamp
      } else if (timestamp === lastTimestamp) {
        // onaudioprocess gets called 44100 / 4096 = 10.77 times per second
        // We only want to process the data every 100ms
        // This is not the proper way to do this
        // because we are dropping data
        // but it's good enough for debugging purposes
        return
      }
      const channel = this.deviceInfo.channels[0]
      if (channel.connected && channel.enabled) {
        for (let i = 0; i < 10; i++) {
          const t = timestamp - 100 + i * 10
          const readings = new Float32Array(20)
          const noises = new Float32Array(20)
          for (let j = 0; j < 20; j++) {
            readings[j] = timeDomainData[i * 400 + j * 20] * 50
          }

          for (let k = 0; k < this.readingCallbacks.length; k++) {
            this.readingCallbacks[k](channel.id, {
              timestamp: t,
              readings,
              noises,
            })
          }
        }

        const fft0To2k = frequencyData.slice(0, 200)
        const fft2kTo20k = new Float32Array(360)
        for (let i = 200; i < 2000; i += 5) {
          const avg =
            (frequencyData[i] +
              frequencyData[i + 1] +
              frequencyData[i + 2] +
              frequencyData[i + 3] +
              frequencyData[i + 4]) /
            5
          fft2kTo20k[(i - 200) / 5] = avg
        }

        for (let k = 0; k < this.fftCallbacks.length; k++) {
          this.fftCallbacks[k](channel.id, {
            timestamp: timestamp - 100,
            fft0To2k,
            fft2kTo20k,
          })
        }
      }

      lastTimestamp = timestamp
    }
  }

  disconnect(): void {
    this.stream?.getTracks().forEach((track) => track.stop())
    this.scriptProcessor?.disconnect()
    this.audioContext?.close()
  }
}
