// The pet's window: Kuchu in one corner of a small transparent window above every other one. The core (src-tauri)
// sizes and places the window, reads where the pointer is (for the eyes, and to let clicks through everywhere but the
// pet) and passes the tray's choices; this page draws the pet, moves it, and plays its tones.

import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect, useRef, useState } from 'react'
import { Pet } from './pet/Pet'
import { lookToward, type PetMood } from './pet/petMood'
import { playTone, toneFor } from './pet/petSound'

/** Where the pointer is from the pet's centre (logical px), and whether it is on the pet: from the core's poll. */
interface Pointer {
  dx: number
  dy: number
  over: boolean
}

/** Until sessions arrive (step 2), the tray's "Try the moods" plays each one. */
const TOUR: { mood: PetMood; ms: number }[] = [
  { mood: 'thinking', ms: 2600 },
  { mood: 'working', ms: 3200 },
  { mood: 'needs', ms: 3200 },
  { mood: 'failed', ms: 2600 },
  { mood: 'done', ms: 0 },
]

/** How far the pointer moves with the button down before it is a drag, not a click. */
const DRAG_PX = 4

export function PetWindow() {
  const [width, setWidth] = useState(96)
  const [mood, setMood] = useState<PetMood>('idle')
  const [look, setLook] = useState({ x: 0, y: 0 })
  const [over, setOver] = useState(false)
  const sounds = useRef(true)
  const tour = useRef<ReturnType<typeof setTimeout>[]>([])

  useEffect(() => {
    if (!isTauri()) return
    const play = () => {
      tour.current.forEach(clearTimeout)
      let at = 0
      tour.current = TOUR.map((step) => {
        const t = setTimeout(() => setMood(step.mood), at)
        at += step.ms
        return t
      })
    }
    const off = [
      listen<Pointer>('kuchu://pointer', (e) => {
        setLook(lookToward(e.payload))
        setOver(e.payload.over)
      }),
      listen<number>('kuchu://size', (e) => setWidth(e.payload)),
      listen<boolean>('kuchu://sounds', (e) => (sounds.current = e.payload)),
      listen('kuchu://try-moods', play),
    ]
    return () => off.forEach((p) => void p.then((stop) => stop()))
  }, [])

  // A small tone when the pet starts to need you, or a session is done or failed.
  const was = useRef<PetMood>(mood)
  useEffect(() => {
    const tone = toneFor(was.current, mood)
    was.current = mood
    if (tone && sounds.current) playTone(tone)
  }, [mood])

  // Pressed and moved: the window is dragged (the system moves it, so it keeps up anywhere). Pressed and let go: a
  // click, which marks a done or failed session as seen.
  const press = useRef<{ x: number; y: number } | null>(null)
  const height = Math.round((width * 154) / 120)
  const stage = { '--pet-w': `${width}px`, '--pet-h': `${height}px` } as React.CSSProperties

  return (
    <div className="stage" style={stage}>
      <div
        className={`bubble${over ? ' is-over' : ''}`}
        role="img"
        aria-label={`Kuchu: ${mood}`}
        onPointerDown={(e) => {
          if (e.button === 0) press.current = { x: e.screenX, y: e.screenY }
        }}
        onPointerMove={(e) => {
          const p = press.current
          if (!p || Math.hypot(e.screenX - p.x, e.screenY - p.y) < DRAG_PX) return
          press.current = null
          if (isTauri()) void getCurrentWindow().startDragging()
        }}
        onPointerUp={() => {
          if (!press.current) return
          press.current = null
          tour.current.forEach(clearTimeout)
          if (mood === 'done' || mood === 'failed') setMood('idle')
        }}
      >
        <Pet size={width} mood={mood} look={look} />
      </div>
    </div>
  )
}
