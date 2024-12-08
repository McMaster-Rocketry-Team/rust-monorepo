import { action, computed, makeObservable, observable } from 'mobx'
import { OzysDevice } from './OzysDevice'
import {
  createContext,
  PropsWithChildren,
  useContext,
  useLayoutEffect,
  useState,
} from 'react'
import DatabaseWorker from '../database/DatabaseWorker?worker'
import * as Comlink from 'comlink'
import type { DatabaseWorkerType } from '../database/DatabaseWorker'
import type { PlayerWindowOptions } from '../database/RealtimeReadingsPlayer'

export class OzysDevicesManager {
  public devices: OzysDevice[] = []
  private dbWorkerScript = new DatabaseWorker()
  private dbWorker = Comlink.wrap<DatabaseWorkerType>(this.dbWorkerScript)

  get activeChannels() {
    const result = []
    for (const device of this.devices) {
      for (const channel of device.deviceInfo.channels) {
        if (channel.connected && channel.enabled) {
          result.push({
            device,
            channel,
          })
        }
      }
    }

    return result
  }

  constructor() {
    this.dbWorker.init()
    makeObservable(this, {
      devices: observable,
      activeChannels: computed,
      addDevice: action,
      disconnectDevice: action,
      disconnectAllDevices: action,
    })
    console.log('OzysDevicesManager created')
  }

  getDeviceAndChannel(channelId: string) {
    for (const device of this.devices) {
      for (const channel of device.deviceInfo.channels) {
        if (channel.connected && channel.id === channelId) {
          return { device, channel }
        }
      }
    }
    return null
  }

  addDevice(device: OzysDevice) {
    device.onRealtimeReadings((channelId, data) => {
      this.dbWorker.onRealtimeReadings(channelId, data)
    })
    device.onRealtimeFft((channelId, data) => {
      this.dbWorker.onRealtimeFft(channelId, data)
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

  disconnectAllDevices() {
    this.devices.forEach((device) => device.disconnect())
    this.devices = []
    this.dbWorkerScript.terminate()
    console.log('OzysDevicesManager terminated')
  }

  async createRealtimeReadingsPlayer(
    channelId: string,
    windowOptions: PlayerWindowOptions,
  ) {
    return await this.dbWorker.createRealtimeReadingsPlayer(
      channelId,
      windowOptions,
    )
  }
}

const ozysDevicesManagerContext = createContext<OzysDevicesManager | undefined>(
  undefined,
)

export const OzysDevicesManagerProvider = (props: PropsWithChildren) => {
  const [manager, setManager] = useState<OzysDevicesManager | undefined>()
  useLayoutEffect(() => {
    const manager = new OzysDevicesManager()
    setManager(manager)
    return () => {
      manager.disconnectAllDevices()
      setManager(undefined)
    }
  }, [])

  if (!manager) {
    return null
  }

  return (
    <ozysDevicesManagerContext.Provider value={manager}>
      {props.children}
    </ozysDevicesManagerContext.Provider>
  )
}

export const useOzysDevicesManager = () => {
  return useContext(ozysDevicesManagerContext)!
}
