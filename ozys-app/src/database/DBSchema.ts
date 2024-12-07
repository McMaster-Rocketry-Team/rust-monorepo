export type DBSchema = {
  readings: ReadingsTable
}

export type ReadingsTable = {
  timestamp: number
  deviceId: string
  channelId: string
//   readings: Uint8Array,
//   noises: Uint8Array,
}
