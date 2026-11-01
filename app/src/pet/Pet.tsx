// Pet.tsx — Kuchu, the desk robot, with a wink on its screen (a lit eye, the saffron pointer for the other, a smile),
// shaded to read as a glossy toy. It acts out what Claude Code is doing: blinks and glances when idle (a timer,
// so nothing runs in between), a thought bubble while a session thinks, typing at a small laptop while it works, a wave
// when it needs you, three hops when a session is done and a happy face until the pet is clicked, a frown when one
// failed.
// The pet beside your windows (`lively`) also has a life of its own when there is no work, in short scenes, each with
// a wind-up, the action and a settle: a yawn behind its hand, a stretch on tiptoe, checking its watch and sighing, a
// little dance, its antenna springing, a look around, its screen flipping to another face, or a short walk. After a
// long while with nothing happening it nods off and naps, until your pointer comes near or a session starts; it wakes
// with a start and a yawn. It walks while it is dragged, smiles under the pointer, and gets heart eyes when petted (the
// pointer moved to and fro over it).
// Its eyes and head follow the pointer. Asked for less motion, it keeps its expressions and drops the movement.

import { useEffect, useId, useRef, useState } from 'react'
import './pet.css'
import type { PetMood } from './petMood'

/** The drawing: 120 wide, 154 tall from y = -14 (room for the thought bubble). */
const RATIO = 154 / 120
/** The wink's pointer, at the screen's right eye. */
const POINTER = 'M66 39 v13.2 l3.5 -3.1 l2.4 5.4 l2.3 -1 l-2.3 -5.2 h4.25 Z'
/** A heart, 12 wide, its centre at 0, 0. */
const HEART = 'M0 -1.6 C-1.2 -5.6 -6 -4.8 -6 -1.2 C-6 1.8 -2.4 4.2 0 6 C2.4 4.2 6 1.8 6 -1.2 C6 -4.8 1.2 -5.6 0 -1.6 Z'

/** What Kuchu does now and then with no work, and how long each scene takes (ms, as pet.css plays it). A stroll
 *  lasts as long as its walk. */
const ACTIVITIES = { stretch: 3800, yawn: 4400, look: 2600, bored: 4600, flip: 2600, dance: 3800, boing: 3000, stroll: 0 } as const
/** Besides those: walking on the spot, nodding off before a nap, and the start of waking. */
type Activity = keyof typeof ACTIVITIES | 'pace' | 'drowsy' | 'startle'
const DROWSY_MS = 5000
const STARTLE_MS = 1200
/** One scene every 15 to 35 seconds. */
const EVERY_MS = [15_000, 35_000] as const
/** Idle this long with nothing near it, it naps. */
const NAP_AFTER_MS = 4 * 60_000
/** Walking on the spot (no room for a stroll): each way this long. */
const PACE_MS = 1500
/** A stroll's speed (logical px a second) and its pause before coming back, as the core walks it. */
const STROLL_SPEED = 70
const STROLL_PAUSE_MS = 900
/** Heart eyes this long after being petted. */
const PETTED_MS = 1800

/** Blinks every 3 to 6 seconds (one in four a wink) and, idle, a glance around now and then. */
function useMoment(on: boolean, idle: boolean): string | null {
  const [now, setNow] = useState<string | null>(null)
  useEffect(() => {
    if (!on || reducedMotion()) return
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

/** Kuchu's own life while there is no work: a scene now and then (never the same twice running), nodding off into a
 *  nap after a long quiet, and a start and a yawn when woken. `stroll` walks it `dx` px and back, and says how far it
 *  really went. */
function useLife(on: boolean, near: boolean, stroll?: (dx: number) => Promise<number>) {
  const [activity, setActivity] = useState<Activity | null>(null)
  const [pace, setPace] = useState(0)
  const [asleep, setAsleep] = useState(false)
  const [waking, setWaking] = useState<Activity | null>(null)
  const quietSince = useRef(Date.now())
  const nearNow = useRef(near)
  nearNow.current = near

  useEffect(() => {
    if (near || !on) quietSince.current = Date.now()
    if (asleep && (near || !on)) {
      setAsleep(false)
      if (!on) return
      setWaking('startle')
      const yawn = setTimeout(() => setWaking('yawn'), STARTLE_MS)
      const done = setTimeout(() => setWaking(null), STARTLE_MS + ACTIVITIES.yawn)
      return () => {
        clearTimeout(yawn)
        clearTimeout(done)
      }
    }
  }, [near, on, asleep])

  useEffect(() => {
    if (!on || asleep) return
    const still = reducedMotion()
    let live = true
    let timer: ReturnType<typeof setTimeout>
    let last: Activity | null = null
    const after = (ms: number, then: () => void) => {
      timer = setTimeout(() => live && then(), ms)
    }
    const end = () => {
      setActivity(null)
      setPace(0)
      next()
    }
    const next = () =>
      after(EVERY_MS[0] + Math.random() * (EVERY_MS[1] - EVERY_MS[0]), async () => {
        if (document.visibilityState !== 'visible') return next()
        if (!nearNow.current && Date.now() - quietSince.current >= NAP_AFTER_MS) {
          // It nods off first, catching itself twice, then sleeps.
          setActivity('drowsy')
          return after(DROWSY_MS, () => setAsleep(true))
        }
        const choices = (Object.keys(ACTIVITIES) as Activity[]).filter((a) => a !== last && (a !== 'stroll' || (stroll && !still)))
        const pick = choices[Math.floor(Math.random() * choices.length)]!
        last = pick
        if (pick !== 'stroll') {
          setActivity(pick)
          return after(ACTIVITIES[pick as keyof typeof ACTIVITIES], end)
        }
        const went = Math.abs(await stroll!(40 + Math.random() * 50).catch(() => 0))
        if (!live) return
        if (went > 0) {
          setActivity('stroll')
          return after((2 * went * 1000) / STROLL_SPEED + STROLL_PAUSE_MS + 400, end)
        }
        // No room to go anywhere: it walks on the spot, one way, then the other.
        setActivity('pace')
        setPace(-1)
        after(PACE_MS, () => {
          setPace(1)
          after(PACE_MS, end)
        })
      })
    next()
    return () => {
      live = false
      clearTimeout(timer)
      setActivity(null)
      setPace(0)
    }
  }, [on, asleep, stroll])

  return { activity: waking ?? activity, pace, asleep }
}

/** Heart eyes for a moment after each petting. */
function usePetted(petted?: number) {
  const [now, setNow] = useState(false)
  useEffect(() => {
    if (!petted) return
    setNow(true)
    const t = setTimeout(() => setNow(false), PETTED_MS)
    return () => clearTimeout(t)
  }, [petted])
  return now
}

function reducedMotion() {
  return !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
}

export function Pet({
  size,
  mood = 'idle',
  look,
  float = false,
  lively = false,
  near = false,
  over = false,
  walk = 0,
  petted,
  stroll,
}: {
  size: number
  mood?: PetMood
  look?: { x: number; y: number }
  float?: boolean
  /** The pet beside your windows: it has a life of its own when idle. */
  lively?: boolean
  /** The pointer is close: a napping pet wakes. */
  near?: boolean
  /** The pointer is on it. */
  over?: boolean
  /** It is being carried or strolling: -1 to the left, 1 to the right. */
  walk?: number
  /** When it was last petted. */
  petted?: number
  stroll?: (dx: number) => Promise<number>
}) {
  const life = useLife(lively && mood === 'idle', near || over, stroll)
  const loved = usePetted(petted)
  // A pet too small to read a blink (a loading mark) does not blink; nor does one busy with something else.
  const blinks = size >= 32 && (mood === 'idle' || mood === 'working' || mood === 'thinking') && !life.activity && !life.asleep && !loved
  const moment = useMoment(blinks, mood === 'idle')
  const walking = walk || life.pace
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
  const classes = [
    'pet',
    `is-${mood}`,
    moment && `is-${moment}`,
    float && 'is-float',
    life.asleep && 'is-asleep',
    !life.asleep && loved && 'is-petted',
    !life.asleep && !loved && life.activity && `is-m-${life.activity}`,
    over && !life.asleep && !loved && !life.activity && (mood === 'idle' || mood === 'done') && 'is-hovered',
    walking && 'is-walking',
    walking < 0 && 'is-walk-left',
    walking > 0 && 'is-walk-right',
  ]
  return (
    <span className={classes.filter(Boolean).join(' ')} style={style} aria-hidden="true">
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
          {/* Faces slide across the screen, never past its edge. */}
          <clipPath id={`${id}glass`}>
            <rect x="25" y="26" width="70" height="50" rx="18" />
          </clipPath>
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
        {/* Napping: a z now and then, drifting up. */}
        <g className="pet-zs">
          <path className="pet-z" d="M90 14 h5 l-5 6 h5" />
          <path className="pet-z pet-z2" d="M96 6 h6 l-6 7 h6" />
          <path className="pet-z pet-z3" d="M103 -3 h7 l-7 8 h7" />
        </g>
        {/* Petted: hearts rising over its head. */}
        <g className="pet-love">
          <g transform="translate(44 4) scale(0.7)">
            <path className="pet-love-heart" d={HEART} />
          </g>
          <g transform="translate(78 0) scale(0.55)">
            <path className="pet-love-heart pet-love2" d={HEART} />
          </g>
        </g>
        <g className="pet-whole">
          {/* Legs and feet, each its own so they can step. */}
          <g className="pet-leg-l">
            <rect className="pet-joint" x="45" y="122" width="9" height="8" rx="3" />
            <rect className="pet-shell" x="38" y="127" width="20" height="9" rx="4.5" fill={u('shell')} />
            <rect className="pet-saffron" x="40" y="133.5" width="16" height="2.5" rx="1.25" />
            <ellipse cx="44" cy="129.5" rx="5" ry="1.6" fill={u('shine')} />
          </g>
          <g className="pet-leg-r">
            <rect className="pet-joint" x="66" y="122" width="9" height="8" rx="3" />
            <rect className="pet-shell" x="62" y="127" width="20" height="9" rx="4.5" fill={u('shell')} />
            <rect className="pet-saffron" x="64" y="133.5" width="16" height="2.5" rx="1.25" />
            <ellipse cx="68" cy="129.5" rx="5" ry="1.6" fill={u('shine')} />
          </g>
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
            <g className="pet-antenna">
              <line x1="60" y1="16" x2="60" y2="7" stroke="#cfc5b6" strokeWidth="3" strokeLinecap="round" />
              <circle className="pet-bulb" cx="60" cy="5" r="4.6" fill={u('ball')} />
              <circle className="pet-spark pet-bulb-spark" cx="58.6" cy="3.6" r="1.2" />
            </g>
            <circle cx="11" cy="52" r="6.2" fill={u('ball')} />
            <circle cx="109" cy="52" r="6.2" fill={u('ball')} />
            <circle className="pet-spark" cx="9.3" cy="50" r="1.3" />
            <circle className="pet-spark" cx="107.3" cy="50" r="1.3" />
            <rect className="pet-shell" x="13" y="14" width="94" height="74" rx="30" fill={u('shell')} />
            <ellipse cx="34" cy="22" rx="16" ry="5.5" transform="rotate(-14 34 22)" fill={u('shine')} />
            <rect className="pet-bevel" x="23.5" y="24.6" width="73" height="53" rx="19.5" />
            <rect x="25" y="26" width="70" height="50" rx="18" fill={u('screen')} />
            <rect className="pet-rim" x="25.6" y="26.6" width="68.8" height="48.8" rx="17.4" />
            <g clipPath={u('glass')}>
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
                {/* Eyes shut (a nap, a stretch, a yawn), and hearts for being petted. */}
                <path className="pet-line pet-shut" d="M39.5 47.5 Q46 53.5 52.5 47.5" />
                <path className="pet-line pet-shut" d="M63.5 47.5 Q70 53.5 76.5 47.5" />
                <g transform="translate(46 48)">
                  <path className="pet-heart" d={HEART} />
                </g>
                <g transform="translate(70 47)">
                  <path className="pet-heart" d={HEART} />
                </g>
                <path className="pet-line pet-smile" d="M51 62 Q60 68.5 69 62" />
                <path className="pet-line pet-frown" d="M52 66.5 Q60 60.5 68 66.5" />
                <path className="pet-line pet-hmm" d="M55 64 H65" />
                <ellipse className="pet-line pet-yawn" cx="60" cy="64.5" rx="3.6" ry="4.4" />
                <circle className="pet-line pet-o" cx="63" cy="64" r="1.9" />
                <g className="pet-mark">
                  <rect x="58.4" y="58" width="3.2" height="6" rx="1.6" fill="#fff6ea" />
                  <circle cx="60" cy="67.5" r="1.7" fill="#fff6ea" />
                </g>
              </g>
              {/* The face its screen flips to: both eyes happy, a wide smile. */}
              <g className="pet-face-alt">
                <path className="pet-line" d="M39.5 50 Q46 41 52.5 50" />
                <path className="pet-line" d="M63.5 50 Q70 41 76.5 50" />
                <path className="pet-line" d="M49 61 Q60 71 71 61" />
              </g>
              <rect className="pet-scan" x="25" y="26" width="70" height="2.5" fill="#fff6ea" />
              <rect className="pet-dim" x="25" y="26" width="70" height="50" fill="#000" />
            </g>
            {/* A hand to its mouth (the yawn): a forearm in front of the face; the arms themselves are behind the
                head. */}
            <g className="pet-hand">
              <rect className="pet-shell" x="60.5" y="63" width="11" height="25" rx="5.5" fill={u('capsule')} />
              <ellipse className="pet-shell" cx="66" cy="64.5" rx="6.6" ry="5.4" fill={u('capsule')} />
              <circle cx="66" cy="65.5" r="2.1" fill={u('ball')} />
            </g>
          </g>
        </g>
        {/* Whistling: notes rising past the top of its head, in front of it. */}
        <g className="pet-notes">
          <g className="pet-note">
            <circle cx="104" cy="22" r="2.4" />
            <path d="M106.2 22 V13 l4 1.4" />
          </g>
          <g className="pet-note pet-note2">
            <circle cx="104" cy="22" r="2" />
            <path d="M105.9 22 V14.5 l3.4 1.2" />
          </g>
        </g>
      </svg>
    </span>
  )
}
