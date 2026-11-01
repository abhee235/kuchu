// Pet.tsx — Kuchu, the desk robot, with a wink on its screen (a lit eye, the saffron pointer for the other, a smile),
// shaded to read as a glossy toy. It acts out what Claude Code is doing: blinks and glances when idle (a timer,
// so nothing runs in between), a thought bubble while a session thinks, typing at a small laptop while it works, a wave
// when it needs you, a hop every few seconds when a session is done until the pet is clicked, a frown when one failed.
// Its eyes and head follow the pointer. Asked for less motion, it keeps its expressions and drops the movement.

import { useEffect, useId, useState } from 'react'
import './pet.css'
import type { PetMood } from './petMood'

/** The drawing: 120 wide, 154 tall from y = -14 (room for the thought bubble). */
const RATIO = 154 / 120
/** The wink's pointer, at the screen's right eye. */
const POINTER = 'M66 39 v13.2 l3.5 -3.1 l2.4 5.4 l2.3 -1 l-2.3 -5.2 h4.25 Z'

/** Blinks every 3 to 6 seconds (one in four a wink) and, idle, a glance around now and then. */
function useMoment(on: boolean, idle: boolean): string | null {
  const [now, setNow] = useState<string | null>(null)
  useEffect(() => {
    if (!on || window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) return
    let timer: ReturnType<typeof setTimeout>
    let glances = 0
    const next = () => {
      timer = setTimeout(
        () => {
          if (document.visibilityState !== 'visible') return next()
          const glance = idle && ++glances % 4 === 0
          setNow(glance ? 'glance' : Math.random() < 0.25 ? 'wink' : 'blink')
          timer = setTimeout(() => (setNow(null), next()), glance ? 1600 : 170)
        },
        2600 + Math.random() * 3600,
      )
    }
    next()
    return () => clearTimeout(timer)
  }, [on, idle])
  return now
}

export function Pet({ size, mood = 'idle', look, float = false }: { size: number; mood?: PetMood; look?: { x: number; y: number }; float?: boolean }) {
  // A pet too small to read a blink (a loading mark) does not blink.
  const moment = useMoment(size >= 32 && (mood === 'idle' || mood === 'working' || mood === 'thinking'), mood === 'idle')
  // Gradients by id, this pet's own: several pets on a page must not share them.
  const id = useId().replace(/[^\w-]/g, '')
  const u = (name: string) => `url(#${id}${name})`
  const style = {
    width: size,
    height: Math.round(size * RATIO),
    '--lx': `${((look?.x ?? 0) * 5).toFixed(2)}px`,
    '--ly': `${((look?.y ?? 0) * 4).toFixed(2)}px`,
    '--tilt': `${((look?.x ?? 0) * 5).toFixed(2)}deg`,
  } as React.CSSProperties
  return (
    <span className={`pet is-${mood}${moment ? ` is-${moment}` : ''}${float ? ' is-float' : ''}`} style={style} aria-hidden="true">
      <svg viewBox="0 -14 120 154">
        <defs>
          {/* Light from the top left: the shell brightest there, warm grey toward the bottom right. */}
          <radialGradient id={`${id}shell`} cx="0.34" cy="0.24" r="0.95">
            <stop offset="0" stopColor="#ffffff" />
            <stop offset="0.42" stopColor="#f7f1e7" />
            <stop offset="0.78" stopColor="#e4d9c8" />
            <stop offset="1" stopColor="#cdbfaa" />
          </radialGradient>
          <linearGradient id={`${id}capsule`} x1="0" y1="0" x2="1" y2="0">
            <stop offset="0" stopColor="#d9ccb8" />
            <stop offset="0.38" stopColor="#fffdf9" />
            <stop offset="0.72" stopColor="#efe6d9" />
            <stop offset="1" stopColor="#cbbca6" />
          </linearGradient>
          <radialGradient id={`${id}shine`} cx="0.5" cy="0.5" r="0.5">
            <stop offset="0" stopColor="#ffffff" stopOpacity="0.95" />
            <stop offset="1" stopColor="#ffffff" stopOpacity="0" />
          </radialGradient>
          <radialGradient id={`${id}screen`} cx="0.45" cy="0.35" r="0.75">
            <stop offset="0" stopColor="#2c3242" />
            <stop offset="0.7" stopColor="#181b24" />
            <stop offset="1" stopColor="#0c0d12" />
          </radialGradient>
          <linearGradient id={`${id}glare`} x1="0" y1="0" x2="0.6" y2="1">
            <stop offset="0" stopColor="#ffffff" stopOpacity="0.22" />
            <stop offset="0.55" stopColor="#ffffff" stopOpacity="0.04" />
            <stop offset="1" stopColor="#ffffff" stopOpacity="0" />
          </linearGradient>
          <radialGradient id={`${id}ball`} cx="0.35" cy="0.3" r="0.75">
            <stop offset="0" stopColor="#ffd8a3" />
            <stop offset="0.45" stopColor="#f29a33" />
            <stop offset="1" stopColor="#b85d10" />
          </radialGradient>
          <radialGradient id={`${id}led`} cx="0.4" cy="0.35" r="0.7">
            <stop offset="0" stopColor="#ffffff" />
            <stop offset="1" stopColor="#ffe7c8" />
          </radialGradient>
          <radialGradient id={`${id}ledglow`} cx="0.5" cy="0.5" r="0.5">
            <stop offset="0.45" stopColor="#fff1dc" stopOpacity="0.35" />
            <stop offset="1" stopColor="#fff1dc" stopOpacity="0" />
          </radialGradient>
          <linearGradient id={`${id}pointer`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="#ffb453" />
            <stop offset="1" stopColor="#e2770f" />
          </linearGradient>
          <radialGradient id={`${id}ao`} cx="0.5" cy="0.5" r="0.5">
            <stop offset="0" stopColor="#3a2a18" stopOpacity="0.28" />
            <stop offset="1" stopColor="#3a2a18" stopOpacity="0" />
          </radialGradient>
        </defs>

        <ellipse className="pet-shadow" cx="60" cy="137" rx="34" ry="4.5" fill={u('ao')} />
        {/* Thinking: a thought bubble, its dots rising in turn. */}
        <g className="pet-think">
          <circle className="pet-cloud" cx="93" cy="11" r="2.2" />
          <circle className="pet-cloud" cx="98.5" cy="4.5" r="3.2" />
          <rect className="pet-cloud" x="99" y="-13.5" width="22" height="13" rx="6.5" />
          <circle className="pet-dot" cx="104.5" cy="-7" r="1.7" />
          <circle className="pet-dot pet-d2" cx="110" cy="-7" r="1.7" />
          <circle className="pet-dot pet-d3" cx="115.5" cy="-7" r="1.7" />
        </g>
        <g className="pet-whole">
          {/* Feet and legs. */}
          <rect className="pet-joint" x="45" y="122" width="9" height="8" rx="3" />
          <rect className="pet-joint" x="66" y="122" width="9" height="8" rx="3" />
          <rect className="pet-shell" x="38" y="127" width="20" height="9" rx="4.5" fill={u('shell')} />
          <rect className="pet-shell" x="62" y="127" width="20" height="9" rx="4.5" fill={u('shell')} />
          <rect className="pet-saffron" x="40" y="133.5" width="16" height="2.5" rx="1.25" />
          <rect className="pet-saffron" x="64" y="133.5" width="16" height="2.5" rx="1.25" />
          <ellipse cx="44" cy="129.5" rx="5" ry="1.6" fill={u('shine')} />
          <ellipse cx="68" cy="129.5" rx="5" ry="1.6" fill={u('shine')} />
          {/* The body, its chest light, and the head's shadow on it. */}
          <rect className="pet-shell" x="32" y="88" width="56" height="38" rx="14" fill={u('shell')} />
          <ellipse cx="44" cy="96" rx="9" ry="4" transform="rotate(-18 44 96)" fill={u('shine')} />
          <rect className="pet-plate" x="49" y="98" width="22" height="15" rx="5" />
          <circle cx="60" cy="105.5" r="3.4" fill={u('ball')} />
          <circle className="pet-spark" cx="58.9" cy="104.3" r="0.9" />
          <ellipse cx="60" cy="90" rx="24" ry="4.5" fill={u('ao')} />
          {/* Arms, in front of the body so they reach the laptop. */}
          <g className="pet-arm-l">
            <rect className="pet-shell" x="20" y="93" width="11" height="26" rx="5.5" fill={u('capsule')} />
            <circle cx="25.5" cy="115" r="2.1" fill={u('ball')} />
          </g>
          <g className="pet-arm-r">
            <rect className="pet-shell" x="89" y="93" width="11" height="26" rx="5.5" fill={u('capsule')} />
            <circle cx="94.5" cy="115" r="2.1" fill={u('ball')} />
          </g>
          {/* Working: a small laptop, lines of code appearing on it. */}
          <g className="pet-laptop">
            <rect x="39" y="95" width="42" height="18" rx="3.5" fill="#262a35" />
            <rect className="pet-code" x="44" y="100" width="20" height="2" rx="1" fill="#ee8a1e" />
            <rect className="pet-code pet-c2" x="44" y="104" width="28" height="2" rx="1" fill="#fff6ea" fillOpacity="0.7" />
            <rect className="pet-code pet-c3" x="44" y="108" width="14" height="2" rx="1" fill="#fff6ea" fillOpacity="0.7" />
            <rect className="pet-caret" x="60" y="107.5" width="1.6" height="3" fill="#ee8a1e" />
            <rect x="34" y="112.5" width="52" height="5" rx="2.5" fill="#d9d0c3" />
          </g>
          {/* The head: antenna, side lights, shell, the screen and its glass, the face. */}
          <g className="pet-head">
            <rect className="pet-joint" x="52" y="84" width="16" height="7" rx="3" />
            <line x1="60" y1="16" x2="60" y2="7" stroke="#cfc5b6" strokeWidth="3" strokeLinecap="round" />
            <circle className="pet-bulb" cx="60" cy="5" r="4.6" fill={u('ball')} />
            <circle className="pet-spark pet-bulb-spark" cx="58.6" cy="3.6" r="1.2" />
            <circle cx="11" cy="52" r="6.2" fill={u('ball')} />
            <circle cx="109" cy="52" r="6.2" fill={u('ball')} />
            <circle className="pet-spark" cx="9.3" cy="50" r="1.3" />
            <circle className="pet-spark" cx="107.3" cy="50" r="1.3" />
            <rect className="pet-shell" x="13" y="14" width="94" height="74" rx="30" fill={u('shell')} />
            <ellipse cx="34" cy="22" rx="16" ry="5.5" transform="rotate(-14 34 22)" fill={u('shine')} />
            <rect className="pet-bevel" x="23.5" y="24.6" width="73" height="53" rx="19.5" />
            <rect x="25" y="26" width="70" height="50" rx="18" fill={u('screen')} />
            <rect className="pet-rim" x="25.6" y="26.6" width="68.8" height="48.8" rx="17.4" />
            <path d="M30 40 Q30 28 43 28 H86 Q72 36 52 44 Q38 49 30 56 Z" fill={u('glare')} />
            <g className="pet-face">
              <g className="pet-eye">
                <g className="pet-eye-dot">
                  <circle cx="46" cy="48" r="13" fill={u('ledglow')} />
                  <circle cx="46" cy="48" r="6.5" fill={u('led')} />
                  <circle className="pet-spark" cx="44" cy="45.8" r="1.4" />
                </g>
                <path className="pet-line pet-happy" d="M39.5 50 Q46 41 52.5 50" />
              </g>
              <g className="pet-pointer">
                <path className="pet-pointer-glow" d={POINTER} />
                <path d={POINTER} fill={u('pointer')} stroke={u('pointer')} strokeWidth="1.6" strokeLinejoin="round" />
                <circle className="pet-ripple" cx="66" cy="39" r="5" />
              </g>
              <path className="pet-line pet-smile" d="M51 62 Q60 68.5 69 62" />
              <path className="pet-line pet-frown" d="M52 66.5 Q60 60.5 68 66.5" />
              <path className="pet-line pet-hmm" d="M55 64 H65" />
              <g className="pet-mark">
                <rect x="58.4" y="58" width="3.2" height="6" rx="1.6" fill="#fff6ea" />
                <circle cx="60" cy="67.5" r="1.7" fill="#fff6ea" />
              </g>
            </g>
          </g>
        </g>
      </svg>
    </span>
  )
}
