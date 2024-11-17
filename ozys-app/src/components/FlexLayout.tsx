import { Layout, Model, TabNode, IJsonModel, Actions } from 'flexlayout-react'
import 'flexlayout-react/style/light.css'
import layout from '../models/layout.json'

import Devices from './devices/Devices'

// Random error even though it works and everything matches the types and enums in the docs
// https://rawgit.com/caplin/FlexLayout/demos/demos/v0.8/typedoc/types/IBorderLocation.html
const model = Model.fromJson(layout)

// add new tab?

// const a = new Actions()

// function x(){
//     a.addNode(json, toNodeId, location, index, select?)
// }

export default function FlexLayout() {
  const factory = (node: TabNode) => {
    var tab = node.getName()

    // render different components
    if (tab === 'Devices') {
      return <Devices />
    } else if (tab === 'Strain Graph') {
      return <button>{node.getName()}</button>
    } else if (tab === 'Spectrogram') {
      return <button>{node.getName()}</button>
    } else {
      return <h1>hi</h1>
    }
  }

  return <Layout model={model} factory={factory} />
}
