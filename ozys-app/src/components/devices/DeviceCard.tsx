import ChannelCard from './ChannelCard'

import { ChannelType } from '../../types'

export default function DeviceCard({ ...props }) {
  const name = props.deviceData.name
  const id = props.deviceData.id
  const model = props.deviceData.model
  const channels: ChannelType[] = props.deviceData.channels

  return (
    <div className='flex flex-col w-full pb-8 gap-4'>
      <div className='flex justify-between mx-2'>
        <h1 className='text-lg font-semibold'>{name}</h1>
        <div className='w-2 bg-slate-300 rounded-full'></div>
      </div>

      {channels
        .filter((channel) => channel.state === 'Connected')
        .map((channel) => (
          <ChannelCard key={channel.id} sensorData={channel} />
        ))}
    </div>
  )
}
