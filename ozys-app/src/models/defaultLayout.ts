import { IJsonModel } from 'flexlayout-react'

export const defaultLayout: IJsonModel = {
  global: {
    enableEdgeDock: false,
    tabEnablePopout: false,
    tabEnableRename: false,
    tabSetEnableMaximize: false,
    tabSetMinWidth: 200,
    tabSetMinHeight: 200,
  },
  borders: [
    {
      type: 'border',
      location: 'left',
      selected: 0,
      size: 320,
      minSize: 300,
      children: [
        {
          type: 'tab',
          name: 'Devices',
          enableClose: false,
        },
      ],
    },
  ],
  layout: {
    type: 'row',
    weight: 100,
    children: [
      {
        type: 'row',
        id: '#21d1a37a-e3d9-4fc6-80a2-b3dfd2ce963c',
        weight: 50,
        children: [
          {
            type: 'tabset',
            weight: 50,
            children: [
              {
                type: 'tab',
                name: 'Strain Graph',
              },
            ],
          },
          {
            type: 'tabset',
            weight: 50,
            children: [
              {
                type: 'tab',
                name: 'Spectrogram',
              },
            ],
          },
        ],
      },
    ],
  },
}
