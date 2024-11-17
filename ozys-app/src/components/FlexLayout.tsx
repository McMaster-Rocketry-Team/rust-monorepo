import {
  Layout,
  Model,
  TabNode,
  TabSetNode,
  BorderNode,
  ITabSetRenderValues,
} from 'flexlayout-react'
import 'flexlayout-react/style/light.css'

import Devices from './devices/Devices'
import { useMemo, useRef } from 'react'

import addIcon from '../assets/add.svg'
import { defaultLayout } from '../workspace/defaultLayout'
import { onAllowDrop } from '../workspace/onAllowDrop'
import { TabIdProvider } from '../workspace/useTabId'
import { StrainGraph } from './straingraph/StrainGraph'
import { useDebounce } from 'rooks'

export default function FlexLayout() {
  // Refs and state
  const layoutRef = useRef<Layout | null>(null)

  const initModel = useMemo(() => {
    let model = Model.fromJson(defaultLayout)
    try {
      const initModelJson = JSON.parse(localStorage.getItem('model')!)
      model = Model.fromJson(initModelJson)
    } catch (e) {}
    model.setOnAllowDrop(onAllowDrop)
    return model
  }, [])

  const saveModel = useDebounce((model: Model) => {
    localStorage.setItem('model', JSON.stringify(model.toJson()))
  }, 500)

  const factory = (node: TabNode) => {
    const tab = node.getName()
    let component
    if (tab === 'Devices') {
      component = <Devices />
    } else if (tab === 'Strain Graph') {
      component = <StrainGraph />
    } else if (tab === 'Spectrogram') {
      component = <button>{node.getName()}</button>
    } else {
      component = <h1>Unknown Tab</h1>
    }
    return <TabIdProvider value={node.getId()}>{component}</TabIdProvider>
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
      model={initModel}
      factory={factory}
      onRenderTabSet={newTab}
      realtimeResize
      onModelChange={saveModel}
    />
  )
}
