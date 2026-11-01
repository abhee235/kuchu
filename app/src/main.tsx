import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Pet } from './pet/Pet'
import { PetWindow } from './PetWindow'
import './styles.css'

// `?icon`: the pet large and still on a clear page, to make the app's icons from (`tauri icon`).
const icon = new URLSearchParams(location.search).has('icon')

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    {icon ? (
      <div className="icon-stage">
        <Pet size={740} mood="idle" />
      </div>
    ) : (
      <PetWindow />
    )}
  </StrictMode>,
)
