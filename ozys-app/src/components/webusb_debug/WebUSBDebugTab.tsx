import { useState } from 'react'
import { useTabAtom } from '../../workspace/useTabAtom'

export const WebUSBDebugTab = () => {
  const [device, setDevice] = useState<USBDevice | null>(null)

  return (
    <div>
      <h1>WebUSB Debug</h1>
      <p>
        {device
          ? `Device: ${device.productName} ${device.manufacturerName} ${device.serialNumber}`
          : 'No device selected'}
      </p>
      <button
        type='button'
        className='border border-gray-400'
        onClick={async () => {
          // TODO we need to check if the browser supports WebUSB
          // TODO we need to get a usb product id from https://pid.codes/
          // For testing, OZYS V2 is 0x1209:0x0002, OZYS V3 is 0x1209:0x0003
          const device = await navigator.usb.requestDevice({
            filters: [
              { vendorId: 0x1209, productId: 0x0002 },
              { vendorId: 0x1209, productId: 0x0003 },
            ],
          })
          await device.open()
          device.selectConfiguration(1)
          device.claimInterface(1)

          setDevice(device)
        }}
      >
        List Devices
      </button>
      <button
        type='button'
        className='border border-gray-400'
        onClick={async () => {
          const result = await device!.controlTransferIn(
            {
              requestType: 'vendor',
              recipient: 'device',
              request: 101,
              value: 201,
              index: 0,
            },
            5,
          )
          console.log('Control Transfer Result:', result)
          alert('Device response: ' + new TextDecoder().decode(result.data))
        }}
      >
        Send Control Transfer
      </button>
      <button
        type='button'
        className='border border-gray-400'
        onClick={async () => {
          const result = await device!.isochronousTransferIn(1, [64])
          console.log(
            'Isochronous Transfer Result:',
            result.packets[0]?.status,
            result.packets[0]?.data?.getUint8(1),
          )
        }}
      >
        Receive Isochronous Transfer
      </button>
    </div>
  )
}
