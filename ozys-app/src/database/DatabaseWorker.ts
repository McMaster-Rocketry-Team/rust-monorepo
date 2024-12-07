import { expose } from 'comlink'
import {
  OzysChannelRealtimeFft,
  OzysChannelRealtimeReadings,
} from '../device/OzysDevice'

// Import Dexie
import Dexie, { EntityTable, Table } from 'dexie'

type DBType = Dexie & {
  readings: EntityTable<OzysChannelRealtimeReadings, 'timestamp'>
  ffts: EntityTable<OzysChannelRealtimeFft, 'timestamp'>
}

class DatabaseWorker {
  private db: DBType

  constructor() {
    this.db = new Dexie('db') as DBType
    this.db.version(1).stores({
      readings: '[timestamp+deviceId+channelId]',
    })
    this.db.version(2).stores({
      ffts: '[timestamp+deviceId+channelId]',
    })
  }

  async init() {}

  async onRealtimeReadings(
    deviceId: string,
    channelId: string,
    data: OzysChannelRealtimeReadings,
  ) {
    await this.db.readings.add(data)
  }

  async onRealtimeFft(
    deviceId: string,
    channelId: string,
    data: OzysChannelRealtimeFft,
  ) {
    await this.db.ffts.add(data)
  }
}

const obj = new DatabaseWorker()

export type DatabaseWorkerType = typeof obj

expose(obj)
