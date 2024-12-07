import * as Comlink from 'comlink'
import {
  OzysChannelRealtimeFft,
  OzysChannelRealtimeReadings,
} from '../device/OzysDevice'
import Dexie, { EntityTable } from 'dexie'
import { RealtimeReadingsPlayer } from './RealtimeReadingsPlayer'

class DBReadingsRowBuilder {
  timestamp: number = -1
  readings: Float32Array = new Float32Array(200)
  noises: Float32Array = new Float32Array(200)
  nextDataI: number | undefined

  constructor(private deviceId: string, private channelId: string) {}

  addData(data: OzysChannelRealtimeReadings) {
    if (data.timestamp % 100 === 0) {
      // First data
      this.timestamp = data.timestamp
      this.nextDataI = 1
      this.readings.set(data.readings)
      this.noises.set(data.noises)
    } else if (
      this.nextDataI &&
      data.timestamp === this.timestamp + this.nextDataI * 10
    ) {
      // Subsequent data
      this.readings.set(data.readings, this.nextDataI * 20)
      this.noises.set(data.noises, this.nextDataI * 20)
      this.nextDataI++
    }
  }

  isFull() {
    return this.nextDataI === 10
  }

  buildAndClear(): DBReadingsRow {
    const row = new DBReadingsRow()
    row.timestamp = this.timestamp
    row.deviceId = this.deviceId
    row.channelId = this.channelId
    row.readings = this.readings
    row.noises = this.noises
    this.timestamp = -1
    this.nextDataI = undefined
    return row
  }
}

// Stores 100ms worth of readings (200 readings)
// instead of 10ms worth of readings from OzysChannelRealtimeReadings
class DBReadingsRow {
  // aligned to the start of the 100ms interval
  timestamp!: number
  deviceId!: string
  channelId!: string
  readings!: Float32Array
  noises!: Float32Array
}

class DBFftRow {
  timestamp!: number
  deviceId!: string
  channelId!: string
  fft0To2k!: Float32Array
  fft2kTo20k!: Float32Array
}

type DBType = Dexie & {
  readings: EntityTable<OzysChannelRealtimeReadings, 'timestamp'>
  ffts: EntityTable<DBFftRow, 'timestamp'>
}

class DatabaseWorker {
  private db: DBType
  private readingsRowCache: Map<string, DBReadingsRowBuilder> = new Map()
  private realtimeReadingsPlayers: Map<string, RealtimeReadingsPlayer> =
    new Map()

  constructor() {
    this.db = new Dexie('db') as DBType
    this.db.version(1).stores({
      readings: '[timestamp+deviceId+channelId]',
    })
    this.db.version(2).stores({
      ffts: '[timestamp+deviceId+channelId]',
    })
    this.db.readings.mapToClass(DBReadingsRow)
    this.db.ffts.mapToClass(DBFftRow)
  }

  async init() {}

  async onRealtimeReadings(
    deviceId: string,
    channelId: string,
    data: OzysChannelRealtimeReadings,
  ) {
    for (const player of this.realtimeReadingsPlayers.values()) {
      player.onRealtimeReadings(channelId, data)
    }

    const cacheKey = `${deviceId}:${channelId}`
    let builder = this.readingsRowCache.get(cacheKey)
    if (!builder) {
      builder = new DBReadingsRowBuilder(deviceId, channelId)
      this.readingsRowCache.set(cacheKey, builder)
    }
    builder.addData(data)
    if (builder.isFull()) {
      await this.db.readings.add(builder.buildAndClear())
    }
  }

  async onRealtimeFft(
    deviceId: string,
    channelId: string,
    data: OzysChannelRealtimeFft,
  ) {
    await this.db.ffts.add({
      deviceId,
      channelId,
      ...data,
    })
  }

  async createRealtimeReadingsPlayer(
    channelId: string,
    sampleRate: number,
    targetSampleOffset: number,
  ) {
    const id = crypto.randomUUID()
    const player = new RealtimeReadingsPlayer(
      channelId,
      sampleRate,
      targetSampleOffset,
      () => {
        this.realtimeReadingsPlayers.delete(id)
      },
    )
    this.realtimeReadingsPlayers.set(id, player)
    return Comlink.proxy(player)
  }
}

const obj = new DatabaseWorker()

export type DatabaseWorkerType = typeof obj

Comlink.expose(obj)
