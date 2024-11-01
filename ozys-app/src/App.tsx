import { useState } from 'react'
import reactLogo from './assets/react.svg'
import { invoke } from '@tauri-apps/api/core'
import './App.css'

function App() {
  const [enumResult, setEnumResult] = useState('')

  return (
    <main className='container'>
      <h1>Welcome to Tauri + React</h1>

      <div className='row'>
        <a href='https://vitejs.dev' target='_blank'>
          <img src='/vite.svg' className='logo vite' alt='Vite logo' />
        </a>
        <a href='https://tauri.app' target='_blank'>
          <img src='/tauri.svg' className='logo tauri' alt='Tauri logo' />
        </a>
        <a href='https://reactjs.org' target='_blank'>
          <img src={reactLogo} className='logo react' alt='React logo' />
        </a>
      </div>
      <p>Click on the Tauri, Vite, and React logos to learn more.</p>

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
