import { makeAutoObservable } from 'mobx'
import { OzysDevice } from './OzysDevice'
import {
  createContext,
  PropsWithChildren,
  useContext,
  useLayoutEffect,
  useState,
} from 'react'

class OzysDevicesManager {
  public devices: OzysDevice[] = []

  constructor() {
    makeAutoObservable(this)
  }

  addDevice(device: OzysDevice) {
    device.onRealtimeReadings((channelId, data) => {
      // Send the data to worker thread
    })
    device.onRealtimeFft((channelId, data) => {
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

  disconnectAllDevices() {
    this.devices.forEach((device) => device.disconnect())
    this.devices = []
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
