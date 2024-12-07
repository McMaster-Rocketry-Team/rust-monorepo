import { observer } from 'mobx-react-lite'
import { useTabAtom } from '../../workspace/useTabAtom'
import { useOzysDevicesManager } from '../../device/OzysDevicesManager'
import { useEffect, useId, useMemo, useRef } from 'react'
import { produce } from 'immer'
import { RealtimeReadingsPlayer } from '../../database/RealtimeReadingsPlayer'
import { Remote } from 'comlink'
import { useGetIsMounted, useIntervalWhen, useWillUnmount } from 'rooks'
import { Mutex } from 'async-mutex'
import { autorun } from 'mobx'

export const StrainGraph = observer(() => {
  const getIsMounted = useGetIsMounted()
  const devicesManager = useOzysDevicesManager()
  const [selectedChannels, setSelectedChannels] = useTabAtom<string[]>(
    'selectedChannels',
    [],
  )
  const playersRef = useRef<Map<string, Remote<RealtimeReadingsPlayer>>>()
  const playersMutexRef = useRef<Mutex>()

  if (!playersRef.current) {
    playersRef.current = new Map()
    playersMutexRef.current = new Mutex()
  }

  // remove selected channels that are not in allChannels
  // autorun is needed here because devicesManager.activeChannels
  // is not a react state (but a mobx computed property)
  useEffect(
    () =>
      autorun(() => {
        const activeChannelIds = devicesManager.activeChannels.map(
          (channel) => channel.channel.id,
        )
        setSelectedChannels((old) =>
          old.filter((channelId) => activeChannelIds.includes(channelId)),
        )
      }),
    [],
  )

  useEffect(() => {
    playersMutexRef.current!.runExclusive(async () => {
      // create new players for selected channels
      for (const selectedChannel of selectedChannels) {
        if (!playersRef.current!.has(selectedChannel)) {
          const player = await devicesManager.createRealtimeReadingsPlayer(
            selectedChannel,
            60, // TODO
            0,
          )
          if (!getIsMounted()) return
          playersRef.current!.set(selectedChannel, player)
        }
      }

      // dispose players for unselected channels
      for (const [channelId, player] of playersRef.current!.entries()) {
        if (!selectedChannels.includes(channelId)) {
          player.dispose()
          playersRef.current!.delete(channelId)
        }
      }
    })
  }, [selectedChannels])

  useIntervalWhen(
    async () => {
      for (const player of playersRef.current!.values()) {
        const newData = await player.getNewData()
        for (const data of newData) {
          console.log(data)
          
        }
      }
    },
    50,
    true,
  )

  useWillUnmount(() => {
    playersMutexRef.current!.runExclusive(async () => {
      for (const player of playersRef.current!.values()) {
        player.dispose()
      }
    })
  })

  return (
    <div>
      <p>Strain Graph</p>
      <div>
        {devicesManager.activeChannels.map(({ device, channel }) => (
          <CheckBox
            key={channel.id}
            label={`${device.deviceInfo.name} - ${channel.name}`}
            checked={selectedChannels.includes(channel.id)}
            onChange={(checked) =>
              setSelectedChannels(
                produce((draft) => {
                  if (checked) {
                    draft.push(channel.id)
                  } else {
                    draft.splice(draft.indexOf(channel.id), 1)
                  }
                }),
              )
            }
          />
        ))}
      </div>
    </div>
  )
})

const CheckBox = (props: {
  label: string
  checked: boolean
  onChange: (checked: boolean) => void
}) => {
  const id = useId()

  return (
    <div className='flex items-center mb-4'>
      <input
        id={id}
        type='checkbox'
        checked={props.checked}
        onChange={(e) => props.onChange(e.target.checked)}
        className='mr-2 border'
      />
      <label htmlFor={id}>{props.label}</label>
    </div>
  )
}
