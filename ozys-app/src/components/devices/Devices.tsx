import { observer } from 'mobx-react-lite'
import { useOzysDevicesManager } from '../../device/OzysDevicesManager'
import { MockOzysDevice } from '../../device/MockOzysDevice'
import { DeviceCard } from './DeviceCard'

export const Devices = observer(() => {
  const devicesManager = useOzysDevicesManager()

  return (
    <div className='w-full h-full relative'>
      {devicesManager.devices.map((device) => (
        <DeviceCard key={device.deviceInfo.id} device={device} />
      ))}{' '}
      <div className='absolute right-0 bottom-0 m-4'>
        <button
          className='border'
          onClick={() => {
            devicesManager.addDevice(new MockOzysDevice())
          }}
        >
          Add Mock Device
        </button>
        <br />
        <button className='border'>Add USB Device</button>
      </div>
    </div>
  )
})
