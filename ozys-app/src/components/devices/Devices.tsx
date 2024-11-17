import { invoke } from '@tauri-apps/api/core'
import { useEffect, useState } from 'react'
import DeviceCard from './DeviceCard'

import { DeviceType } from '../../types'

export default function Devices() {
  const [deviceList, setDeviceList] = useState<DeviceType[]>([])

  useEffect(() => {
    async function enumerateDevices() {
      let list: DeviceType[] = []
      const devices: any[] = (await invoke(
        'ozys_enumerate_devices',
      )) as DeviceType[]

      devices.forEach((device) => {
        list.push(device)
      })
      setDeviceList(list)
    }

    enumerateDevices()
  })

  return (
    <div className='w-full h-full'>
      {/* <p>{deviceList ? JSON.stringify(deviceList) : 'No devices found'}</p> */}

      <div className='flex p-4'>
        {deviceList.map((device) => (
          <DeviceCard deviceData={device} />
        ))}{' '}
      </div>
    </div>
  )
}
