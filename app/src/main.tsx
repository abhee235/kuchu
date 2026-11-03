import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { StrictMode, useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { Pet, SCENE_MS } from './pet/Pet'
import type { PetMood } from './pet/petMood'
import { PetWindow } from './PetWindow'
import { SessionsWindow } from './SessionsWindow'
import { SettingsWindow } from './SettingsWindow'
import '@fontsource-variable/geist'
import './styles.css'

// One page for every window, chosen by the window's name. `?icon`: the pet large and still on a clear page, to make
// the app's icons from (`tauri icon`). `?pose=thinking:9,working:4`: the pet large, acting out each mood for so many
// seconds in turn, to watch and take pictures of its expressions; a step can also be one of its scenes (`spin`,
// `pop`…) or `hello`, the wave as a message is sent, and `?pose=scenes` plays them all. Nothing there comes from
// Claude Code: it is the pet alone, on made-up moods.
const query = new URLSearchParams(location.search)
const icon = query.has('icon')
const pose = query.get('pose')
const label = isTauri() ? getCurrentWindow().label : 'pet'
document.documentElement.dataset.window = icon ? 'icon' : pose ? 'pose' : label

const MOODS: PetMood[] = ['idle', 'thinking', 'working', 'needs', 'done', 'failed']

/** A scene's step in the preview: the scene, then this long standing still before the next. */
const SCENE_REST_MS = 1200

interface Step {
  name: string
  mood: PetMood
  scene?: string
  hello?: boolean
  ms: number
}

function Poses({ plan }: { plan: string }) {
  const names = plan === 'scenes' ? ['hello', ...Object.keys(SCENE_MS)] : plan.split(',')
  const steps = names
    .map((step) => step.split(':'))
    .flatMap(([name = '', seconds]): Step[] => {
      const ms = (Number(seconds) || 0) * 1000
      if (MOODS.includes(name as PetMood)) return [{ name, mood: name as PetMood, ms: ms || 6000 }]
      if (name === 'hello') return [{ name, mood: 'thinking', hello: true, ms: ms || 4000 }]
      if (name in SCENE_MS) return [{ name, mood: 'idle', scene: name, ms: ms || SCENE_MS[name]! + SCENE_REST_MS }]
      return []
    })
  // Each step starts afresh (the pet drawn anew), so a scene plays from its start even when it comes round again.
  const [turn, setTurn] = useState({ at: 0, since: Date.now() })
  useEffect(() => {
    if (!steps.length) return
    const t = setTimeout(() => setTurn({ at: (turn.at + 1) % steps.length, since: Date.now() }), steps[turn.at]!.ms)
    return () => clearTimeout(t)
  }, [turn, steps.length])
  const step = steps[turn.at]
  return (
    <div className="pose-stage">
      <Pet key={turn.since} size={360} mood={step?.mood ?? 'idle'} scene={step?.scene} asked={step?.hello ? turn.since : 0} />
      {steps.length > 1 && <div className="pose-name">{step?.name}</div>}
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
