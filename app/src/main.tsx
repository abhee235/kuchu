import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { StrictMode, useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { Pet } from './pet/Pet'
import type { PetMood } from './pet/petMood'
import { PetWindow } from './PetWindow'
import { SessionsWindow } from './SessionsWindow'
import { SettingsWindow } from './SettingsWindow'
import '@fontsource-variable/geist'
import './styles.css'

// One page for every window, chosen by the window's name. `?icon`: the pet large and still on a clear page, to make
// the app's icons from (`tauri icon`). `?pose=thinking:9,working:4`: the pet large, acting out each mood for so many
// seconds in turn, to watch and take pictures of its expressions.
const query = new URLSearchParams(location.search)
const icon = query.has('icon')
const pose = query.get('pose')
const label = isTauri() ? getCurrentWindow().label : 'pet'
document.documentElement.dataset.window = icon ? 'icon' : pose ? 'pose' : label

const MOODS: PetMood[] = ['idle', 'thinking', 'working', 'needs', 'done', 'failed']

function Poses({ plan }: { plan: string }) {
  const steps = plan
    .split(',')
    .map((step) => step.split(':'))
    .filter(([mood]) => MOODS.includes(mood as PetMood))
    .map(([mood, seconds]) => ({ mood: mood as PetMood, ms: (Number(seconds) || 6) * 1000 }))
  const [at, setAt] = useState(0)
  useEffect(() => {
    if (steps.length < 2) return
    const t = setTimeout(() => setAt((at + 1) % steps.length), steps[at]!.ms)
    return () => clearTimeout(t)
  }, [at, steps.length])
  return (
    <div className="pose-stage">
      <Pet size={360} mood={steps[at]?.mood ?? 'idle'} />
    </div>
  )
}

function Root() {
  if (icon)
    return (
      <div className="icon-stage">
        <Pet size={740} mood="idle" />
      </div>
    )
  if (pose) return <Poses plan={pose} />
  if (label === 'sessions') return <SessionsWindow />
  if (label === 'settings') return <SettingsWindow />
  return <PetWindow />
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
)
