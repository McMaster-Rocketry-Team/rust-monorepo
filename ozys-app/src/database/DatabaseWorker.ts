import * as Comlink from 'comlink'
import {
  OzysChannelRealtimeFft,
  OzysChannelRealtimeReadings,
} from '../device/OzysDevice'
import Dexie, { EntityTable } from 'dexie'
import {
  PlayerWindowOptions,
  RealtimeReadingsPlayer,
} from './RealtimeReadingsPlayer'
import { CircularBuffer } from '../utils/CircularBuffer'

// Stores 100ms worth of readings (200 readings)
// instead of 10ms worth of readings from OzysChannelRealtimeReadings
class DBReadingsRow {
  // aligned to the start of the 100ms interval
  timestamp!: number
  channelId!: string
  readings!: Float32Array
  noises!: Float32Array

  constructor(channelId: string, readings: OzysChannelRealtimeReadings[]) {
    this.timestamp = readings[0].timestamp
    this.channelId = channelId
    this.readings = new Float32Array(200)
    this.noises = new Float32Array(200)
    for (let i = 0; i < 10; i++) {
      this.readings.set(readings[i].readings, i * 20)
      this.noises.set(readings[i].noises, i * 20)
    }
  }

  splitInto10msIntervals(): OzysChannelRealtimeReadings[] {
    const result: OzysChannelRealtimeReadings[] = []
    for (let i = 0; i < 10; i++) {
      result.push({
        timestamp: this.timestamp + i * 10,
        readings: this.readings.slice(i * 20, (i + 1) * 20),
        noises: this.noises.slice(i * 20, (i + 1) * 20),
      })
    }
    return result
  }
}

class DBFftRow {
  timestamp!: number
  channelId!: string
  fft0To2k!: Float32Array
  fft2kTo20k!: Float32Array
}

type DBType = Dexie & {
  readings: EntityTable<DBReadingsRow, 'timestamp'>
  ffts: EntityTable<DBFftRow, 'timestamp'>
}

class DatabaseWorker {
  private db: DBType
  private readingsCacheMap: Map<
    string,
    CircularBuffer<OzysChannelRealtimeReadings>
  > = new Map()
  private realtimeReadingsPlayers: Map<string, RealtimeReadingsPlayer> =
    new Map()

  constructor() {
    this.db = new Dexie('db') as DBType
    this.db.version(1).stores({
      readings: '[channelId+timestamp]',
      ffts: '[channelId+timestamp]',
    })
    this.db.readings.mapToClass(DBReadingsRow)
    this.db.ffts.mapToClass(DBFftRow)
  }

  async init() {}

  async onRealtimeReadings(
    channelId: string,
    readings: OzysChannelRealtimeReadings,
  ) {
    for (const player of this.realtimeReadingsPlayers.values()) {
      player.onRealtimeReadings(channelId, readings)
    }

    let readingsCache = this.readingsCacheMap.get(channelId)
    if (!readingsCache) {
      readingsCache = new CircularBuffer(50)
      this.readingsCacheMap.set(channelId, readingsCache)
    }
    readingsCache.addLast(readings)

    if ((readings.timestamp - 90) % 100 === 0) {
      const last10Readings = readingsCache.lastN(10)
      if (
        last10Readings.length === 10 &&
        last10Readings[0].timestamp === readings.timestamp - 90
      ) {
        await this.db.readings.add(new DBReadingsRow(channelId, last10Readings))
      }
    }
  }

  async onRealtimeFft(channelId: string, data: OzysChannelRealtimeFft) {
    await this.db.ffts.add({
      channelId,
      ...data,
    })
  }

  async createRealtimeReadingsPlayer(
    channelId: string,
    windowOptions: PlayerWindowOptions,
  ) {
    const id = crypto.randomUUID()
    const player = new RealtimeReadingsPlayer(channelId, windowOptions, () => {
      this.realtimeReadingsPlayers.delete(id)
    })

    const start = performance.now()
    // Fill the player with readings from the database
    const rows = await this.db.readings
      .where('[channelId+timestamp]')
      .between(
        [channelId, windowOptions.windowStartTimestamp],
        [
          channelId,
          windowOptions.windowStartTimestamp + windowOptions.windowDuration,
        ],
      )
      .toArray()

    let lastReadingTimestamp = -1
    for (const dbReadingsRow of rows) {
      for (const readings of dbReadingsRow.splitInto10msIntervals()) {
        player.onRealtimeReadings(channelId, readings)
        lastReadingTimestamp = readings.timestamp
      }
    }

    let readingsCache = this.readingsCacheMap.get(channelId)
    if (readingsCache) {
      readingsCache.toArray().forEach((readings) => {
        if (readings.timestamp > lastReadingTimestamp) {
          player.onRealtimeReadings(channelId, readings)
        }
      })
    }
    console.info(`Took ${performance.now() - start}ms to process data for player`)

    this.realtimeReadingsPlayers.set(id, player)
    return Comlink.proxy(player)
  }
}

const obj = new DatabaseWorker()

export type DatabaseWorkerType = typeof obj

Comlink.expose(obj)
