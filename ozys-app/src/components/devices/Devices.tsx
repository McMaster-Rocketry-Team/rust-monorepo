import { invoke } from '@tauri-apps/api/core'
import { useEffect, useState } from 'react'
import DeviceCard from './DeviceCard'

import { DeviceType, RealtimeData } from '../../types'

// logging stuff
import {
  warn,
  debug,
  trace,
  info,
  error,
  attachConsole,
  attachLogger,
} from '@tauri-apps/plugin-log'


export default function Devices() {
  const [deviceList, setDeviceList] = useState<DeviceType[]>([])

  const [deviceData, setDeviceData] = useState<RealtimeData[]>([])

  async function poll() {
    const data = (await invoke('ozys_poll_realtime_data', {
      deviceId: 'mock-ozys-device',
    })) as RealtimeData[]

    setDeviceData(data)

    // info('data: ' + JSON.stringify(data))
  }


  useEffect(() => {
    const polling = setInterval(() => {
      poll()
    }, 10)

    return () => {clearInterval(polling)}
  }, [deviceData])



  useEffect(() => {
    async function enumerateDevices() {
      let list: DeviceType[] = []
      const devices = (await invoke('ozys_enumerate_devices')) as DeviceType[]

      devices.forEach((device) => {
        list.push(device)
      })
      setDeviceList(list)
    }

    enumerateDevices()
  }, [])

  return (
    <div className='w-full h-full'>
      <p>{deviceList ? JSON.stringify(deviceList) : 'No devices found'}</p>

      <p>{deviceData ? JSON.stringify(deviceList) : 'No devices LIST'}</p>

      <h1>{JSON.stringify(deviceData)}</h1>

      <div className='flex p-4'>
        {deviceList.map((device) => (
          <DeviceCard key={device.id} deviceData={device} />
        ))}{' '}
      </div>
    </div>
  )
}
