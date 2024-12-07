import { makeAutoObservable } from 'mobx'
import { OzysDevice } from './OzysDevice'

class OzysDevicesManager {
  public devices: OzysDevice[] = []

  constructor() {
    makeAutoObservable(this)
  }

  addDevice(device: OzysDevice) {
    device.onRealtimeReadings((channelId, data) => {
      // Send the data to worker thread
    })
    device.onRealtimeFFT((channelId, data) => {
      // Send the data to worker thread
    })
    this.devices.push(device)
  }

  disconnectDevice(deviceId: string) {
    const i = this.devices.findIndex(
      (device) => device.deviceInfo.id === deviceId,
    )
    if (i >= 0) {
      const device = this.devices.splice(i, 1)[0]
      device.disconnect()
    }
  }
}
