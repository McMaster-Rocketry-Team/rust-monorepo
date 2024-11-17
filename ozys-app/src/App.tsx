import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import './App.css'

import Button from './components/Button'
import Dashboard from "./components/Dashboard"

function App() {
  const [enumResult, setEnumResult] = useState('')

  return (
    <main>

      {/* <Button>test button</Button> */}

      <Dashboard/>

      <button
        type='button'
        onClick={async () => {
          setEnumResult(JSON.stringify(await invoke('ozys_enumerate_devices')))
        }}
      >
        Enumerate Devices
      </button>
      <p>{enumResult}</p>
    </main>
  )
}

export default App
