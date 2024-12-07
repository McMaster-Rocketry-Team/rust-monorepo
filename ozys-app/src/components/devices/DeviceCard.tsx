import { observer } from 'mobx-react-lite'
import { OzysDevice } from '../../device/OzysDevice'
import ChannelCard from './ChannelCard'

export const DeviceCard = observer((props: { device: OzysDevice }) => {
  const deviceInfo = props.device.deviceInfo

  return (
    <div className='flex flex-col w-full pb-8 gap-4 p-4'>
      <div className='flex justify-between mx-2'>
        <h1 className='text-lg font-semibold'>{deviceInfo.name}</h1>
        <div className='w-2 bg-slate-300 rounded-full'></div>
      </div>

      {deviceInfo.channels
        .filter((channel) => channel.connected)
        .map((channel) => (
          <ChannelCard key={channel.id} sensorData={channel} />
        ))}
    </div>
  )
})
