import { Layout, Model, TabNode, IJsonModel, Actions } from 'flexlayout-react'
import 'flexlayout-react/style/light.css'

var json: IJsonModel = {
  global: { tabEnablePopout: false, tabSetEnableMaximize: false },
  borders: [],
  layout: {
    type: 'row',
    weight: 100,
    children: [
      {
        type: 'tabset',
        weight: 15,
        children: [
          {
            type: 'tab',
            name: 'One',
            component: 'button',
          },
        ],
      },
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
                name: 'Two',
                component: 'button',
              },
            ],
          },
          {
            type: 'tabset',
            weight: 50,
            children: [
              {
                type: 'tab',
                name: 'Three',
                component: 'button',
              },
            ],
          },
        ],
      },
    ],
  },
}

// const a = new Actions()

// function x(){
//     a.addNode(json, toNodeId, location, index, select?)
// }

const model = Model.fromJson(json)

export default function FlexLayout() {
  const factory = (node: TabNode) => {
    var component = node.getComponent()
    if (component === 'button') {
      return <button>{node.getName()}</button>
    }
  }

  return <Layout model={model} factory={factory} />
}
