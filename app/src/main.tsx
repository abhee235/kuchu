import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Pet } from './pet/Pet'
import { PetWindow } from './PetWindow'
import { SessionsWindow } from './SessionsWindow'
import { SettingsWindow } from './SettingsWindow'
import './styles.css'

// One page for every window, chosen by the window's name. `?icon`: the pet large and still on a clear page, to make
// the app's icons from (`tauri icon`).
const icon = new URLSearchParams(location.search).has('icon')
const label = isTauri() ? getCurrentWindow().label : 'pet'
document.documentElement.dataset.window = icon ? 'icon' : label

function Root() {
  if (icon)
    return (
      <div className="icon-stage">
        <Pet size={740} mood="idle" />
      </div>
    )
  if (label === 'sessions') return <SessionsWindow />
  if (label === 'settings') return <SettingsWindow />
  return <PetWindow />
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
)
