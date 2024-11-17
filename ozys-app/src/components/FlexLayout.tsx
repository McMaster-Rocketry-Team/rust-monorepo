import {
  Layout,
  Model,
  TabNode,
  Actions,
  TabSetNode,
  BorderNode,
  ITabSetRenderValues,
} from 'flexlayout-react'
import 'flexlayout-react/style/light.css'
import layout from '../models/layout.json'

import Devices from './devices/Devices'
import { useRef } from 'react'

import addIcon from '../assets/add.svg'

// Random error but it works and everything matches the types and enums in the docs
// https://rawgit.com/caplin/FlexLayout/demos/demos/v0.8/typedoc/types/IBorderLocation.html
const model = Model.fromJson(layout)

export default function FlexLayout() {
  // Refs and state
  const layoutRef = useRef<Layout | null>(null)

  const factory = (node: TabNode) => {
    const tab = node.getName()
    if (tab === 'Devices') {
      return <Devices />
    } else if (tab === 'Strain Graph' || tab === 'Spectrogram') {
      return <button>{node.getName()}</button>
    } else {
      return <h1>Unknown Tab</h1>
    }
  }

  const onAddFromTabSetButton = (node: TabSetNode | BorderNode) => {
    if (layoutRef.current) {
      
      // Temporary, will add a popup menu to select tab type
      const addedTab = layoutRef.current.addTabToTabSet(node.getId(), {
        type: 'tab',
        name: 'Strain Graph',
      })
      console.log('Added tab:', addedTab)
    }
  }

  const newTab = (
    node: TabSetNode | BorderNode,
    renderValues: ITabSetRenderValues,
  ) => {
    if (node instanceof TabSetNode) {
      renderValues.stickyButtons.push(
        <button
          key='add-button'
          style={{
            width: '1.1em',
            height: '1.1em',
            border: 'none',
            background: 'transparent',
            cursor: 'pointer',
          }}
          title='Add Tab'
          onClick={() => onAddFromTabSetButton(node)}
        >
          <img
            src={addIcon}
            alt='Add'
            key='Add button'
            style={{ width: '1.1em', height: '1.1em' }}
            className='flexlayout__tab_toolbar_button'
          />
        </button>,
      )
    }
  }

  return (
    <Layout
      ref={layoutRef}
      model={model}
      factory={factory}
      onRenderTabSet={newTab}
    />
  )
}
