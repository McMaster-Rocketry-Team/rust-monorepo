import { Remote } from 'comlink'
import type { RealtimeReadingsPlayer } from '../../database/RealtimeReadingsPlayer'
import { OzysDevicesManager } from '../../device/OzysDevicesManager'
import { Mutex } from 'async-mutex'
import { CircularBuffer } from '../../utils/CircularBuffer'

type selectedChannel = {
  channelId: string
  color: string
}

export class StrainGraphCanvas {
  private players: Map<
    string,
    {
      player: Remote<RealtimeReadingsPlayer>
      readings: CircularBuffer<{
        timestamp: number
        reading: number
      } | null>
    }
  > = new Map()
  private playersMutex = new Mutex()
  private canvas: HTMLCanvasElement
  private ctx: CanvasRenderingContext2D
  private width!: number
  private height!: number
  private selectedChannels: selectedChannel[] = []

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

    this.resize()
    container.addEventListener('resize', this.resize.bind(this))

    this.draw = this.draw.bind(this)
  }

  async draw(selectedChannels: selectedChannel[]) {
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
          readings: new CircularBuffer(this.width),
        })
      }
      for (const { channelId } of channelsDiff.removed) {
        this.players.get(channelId)?.player.dispose()
        this.players.delete(channelId)
      }
    })

    this.ctx.clearRect(0, 0, this.width, this.height)

    for (const { channelId, color } of selectedChannels) {
      const player = this.players.get(channelId)
      if (!player) continue

      const newData = await player.player.getNewData()
      const readings = player.readings
      for (const data of newData) {
        readings.addLast(data)
      }

      this.ctx.beginPath()
      this.ctx.strokeStyle = color

      readings.forEach((reading) => {
        if (reading === null) {
          this.ctx.stroke()
          this.ctx.beginPath()
        } else {
          // TODO draw line
        }
      })

      this.ctx.stroke()
    }
  }

  dispose() {}

  private diffSelectedChannels(
    old: selectedChannel[],
    newChannels: selectedChannel[],
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

  private resize() {
    console.log('resize')
    this.canvas.width = this.container.clientWidth
    this.canvas.height = this.container.clientHeight
    this.width = this.container.clientWidth
    this.height = this.container.clientHeight
  }
}
