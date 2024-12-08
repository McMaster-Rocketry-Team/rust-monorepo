import { Remote } from 'comlink'
import type { RealtimeReadingsPlayer } from '../../database/RealtimeReadingsPlayer'
import { OzysDevicesManager } from '../../device/OzysDevicesManager'
import { Mutex } from 'async-mutex'
import { CircularBuffer } from '../../utils/CircularBuffer'
import { debounce } from 'lodash-es'

export type SelectedChannel = {
  channelId: string
  color: string
}

export class StrainGraphCanvas {
  private players: Map<
    string,
    {
      player: Remote<RealtimeReadingsPlayer>
      width: number
      readings: CircularBuffer<{
        timestamp: number
        reading: number
      } | null>
    }
  > = new Map()
  private playersMutex = new Mutex()
  private canvas: HTMLCanvasElement
  private ctx: CanvasRenderingContext2D
  private selectedChannels: SelectedChannel[] = []
  private disposed = false
  private isDrawing = false

  private width!: number
  private height!: number
  private sampleRate!: number
  private sampleDuration!: number

  constructor(
    private duration: number,
    private container: HTMLDivElement,
    private devicesManager: OzysDevicesManager,
  ) {
    this.canvas = document.createElement('canvas')
    this.canvas.width = container.clientWidth
    this.canvas.height = container.clientHeight
    container.appendChild(this.canvas)
    this.ctx = this.canvas.getContext('2d')!

    this.draw = this.draw.bind(this)
    this.recreatePlayers = debounce(this.recreatePlayers.bind(this), 100)
    this.resize = this.resize.bind(this)

    this.resize(true)
    container.addEventListener('resize', () => this.resize())
  }

  async draw(selectedChannels: SelectedChannel[]) {
    if (this.disposed) return
    if (this.isDrawing){
        console.warn("Skipping frame")
        return
    }
    this.isDrawing = true
    const now = Date.now()

    // start is inclusive
    let start = now - this.duration + this.sampleDuration
    start -= start % this.sampleDuration

    // end is also inclusive
    const end = start + this.duration - this.sampleDuration

    const channelsDiff = this.diffSelectedChannels(
      this.selectedChannels,
      selectedChannels,
    )
    this.selectedChannels = selectedChannels
    this.playersMutex.runExclusive(async () => {
      for (const { channelId } of channelsDiff.added) {
        const player = await this.devicesManager.createRealtimeReadingsPlayer(
          channelId,
          {
            windowDuration: this.duration,
            windowSampleCount: this.width,
            windowStartTimestamp: Date.now() - this.duration,
          },
        )
        this.players.set(channelId, {
          player,
          width: this.width,
          readings: new CircularBuffer(this.width),
        })
      }
      for (const { channelId } of channelsDiff.removed) {
        this.players.get(channelId)?.player.dispose()
        this.players.delete(channelId)
      }
    })

    for (const { channelId } of selectedChannels) {
        const player = this.players.get(channelId)
        if (!player) continue
  
        const newData = await player.player.getNewData()
        const readings = player.readings
        for (const data of newData) {
          readings.addLast(data)
        }
    }

    this.ctx.clearRect(0, 0, this.width, this.height)
    for (const { channelId, color } of selectedChannels) {
      const player = this.players.get(channelId)
      if (!player) continue
      const readings = player.readings

      this.ctx.beginPath()
      this.ctx.strokeStyle = color
      this.ctx.lineWidth = 1

      let firstPoint = true
      readings.forEach((reading) => {
        if (reading === null) {
          this.ctx.stroke()
          this.ctx.beginPath()
          firstPoint = true
        } else {
          const x = Math.round(
            (reading.timestamp - start) / this.sampleDuration,
          )
          const y = reading.reading * 10 + this.height / 2
        //   console.log("x, y", x, y)
          if (firstPoint) {
            this.ctx.moveTo(x, y)
            firstPoint = false
          } else {
            this.ctx.lineTo(x, y)
          }
        }
      })

      this.ctx.stroke()
    }

    this.isDrawing = false
  }

  dispose() {
    if (this.disposed) return
    this.disposed = true
    this.canvas.remove()
    this.playersMutex.runExclusive(async () => {
      for (const player of this.players.values()) {
        player.player.dispose()
      }
    })
  }

  private diffSelectedChannels(
    old: SelectedChannel[],
    newChannels: SelectedChannel[],
  ) {
    const removed = old.filter(
      (oldChannel) =>
        !newChannels.find(
          (newChannel) => newChannel.channelId === oldChannel.channelId,
        ),
    )
    const added = newChannels.filter(
      (newChannel) =>
        !old.find(
          (oldChannel) => oldChannel.channelId === newChannel.channelId,
        ),
    )
    return { removed, added }
  }

  private resize(initial: boolean = false) {
    this.canvas.width = this.container.clientWidth
    this.canvas.height = this.container.clientHeight
    this.width = this.container.clientWidth
    this.height = this.container.clientHeight
    this.sampleRate = this.width / (this.duration / 1000)
    this.sampleDuration = this.duration / this.width

    if (!initial) {
      console.log('resize to', this.width, this.height)
      this.recreatePlayers()
    }
  }

  private recreatePlayers() {
    this.playersMutex.runExclusive(async () => {
      const newPlayers = new Map()
      for (const channelId of this.players.keys()) {
        const newPlayer =
          await this.devicesManager.createRealtimeReadingsPlayer(channelId, {
            windowDuration: this.duration,
            windowSampleCount: this.width,
            windowStartTimestamp: Date.now() - this.duration,
          })
        newPlayers.set(channelId, {
          player: newPlayer,
          width: this.width,
          readings: new CircularBuffer(this.width),
        })
      }

      const oldPlayers = this.players
      this.players = newPlayers
      for (const player of oldPlayers.values()) {
        player.player.dispose()
      }
    })
  }
}
