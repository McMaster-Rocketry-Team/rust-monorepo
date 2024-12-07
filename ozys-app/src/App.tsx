import './App.css'

import Dashboard from './components/Dashboard'
import { OzysDevicesManagerProvider } from './device/OzysDevicesManager'

function App() {
  return (
    <OzysDevicesManagerProvider>
      <main>
        {/* <Button>test button</Button> */}

        <Dashboard />
      </main>
    </OzysDevicesManagerProvider>
  )
}

export default App
