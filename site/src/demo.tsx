// The live pets on the home page, all Kuchu's own pet from the app's source. One stands above the whole page, as it stands
// above your windows: it starts in front of a VS Code window or a terminal with a copy of its card, while the window goes
// through a Claude Code run, so that Allow on the card moves the run on. Then it walks from section to section as you
// scroll, and can be dragged anywhere. Another shows how it asks for you, and four show its moods, playing its scenes
// together when asked. Their eyes follow the pointer, as they do on the desktop.

import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { createRoot } from 'react-dom/client'
import { Pet, SCENE_MS } from '../../app/src/pet/Pet'
import { lookToward, type PetMood } from '../../app/src/pet/petMood'

type Progress = 'running' | 'done' | 'failed'
type Step = { text: string; progress: Progress }
type Card = { kind: 'steps'; steps: Step[] } | { kind: 'ask' } | { kind: 'answer'; text: string }
type Choice = 'allow' | 'deny' | 'terminal'
/** Where the run is shown: Kuchu follows Claude Code through its hooks, so either way it is the same to Kuchu. */
type Surface = 'code' | 'term'

/** The frames of the run, recorded from Claude Code in VS Code. The first is the one the page opens on. */
const FRAMES = ['ask', 'typed', 'working', 'after', 'done', 'denied'] as const
type Frame = (typeof FRAMES)[number]
const frameUrl = (f: Frame) => `${import.meta.env.BASE_URL}hero/${f}.webp`

const SESSION = 'Date utils test failures'
const FOLDER = 'c:\\tmp\\date-utils'
const FILE = 'c:\\tmp\\date-utils\\src\\date.ts'
/** How long the card waits for a click on Allow before the demo answers it itself. */
const ASK_MS = 9000
/** A long wait in the demo is cut into these, so that a pause takes hold at once. */
const WAIT_SLICE_MS = 100

const reduced = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches
/** Asked for less motion, Kuchu stays in the hero and does not walk the page. */
const roams = !reduced()
const phone = window.matchMedia('(max-width: 640px)')
const tablet = window.matchMedia('(max-width: 1000px)')

/** A pet's width, smaller on a tablet and on a phone. */
function usePetSize(wide: number, small: number, middle = wide) {
  const pick = () => (phone.matches ? small : tablet.matches ? middle : wide)
  const [size, setSize] = useState(pick)
  useEffect(() => {
    const change = () => setSize(pick())
    phone.addEventListener('change', change)
    tablet.addEventListener('change', change)
    return () => {
      phone.removeEventListener('change', change)
      tablet.removeEventListener('change', change)
    }
  }, [wide, small, middle])
  return size
}

function Mark({ progress }: { progress: Progress }) {
  return (
    <span className={`mark is-${progress}`} role="img" aria-label={progress === 'running' ? 'Running' : progress === 'done' ? 'Done' : 'Did not finish'}>
      {progress === 'done' && (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3.5" strokeLinecap="round" strokeLinejoin="round">
          <path d="M20 6 9 17l-5-5" />
        </svg>
      )}
      {progress === 'failed' && (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3.5" strokeLinecap="round">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      )}
    </span>
  )
}

/** The eyes follow the pointer anywhere on the page, as Kuchu's do on the screen. */
function useLook(ref: React.RefObject<HTMLElement | null>) {
  const [look, setLook] = useState({ x: 0, y: 0 })
  const [near, setNear] = useState(false)
  useEffect(() => {
    const move = (e: PointerEvent) => {
      const r = ref.current?.getBoundingClientRect()
      if (!r) return
      const dx = e.clientX - (r.left + r.width / 2)
      const dy = e.clientY - (r.top + r.height * 0.38)
      setLook(lookToward({ dx, dy }))
      setNear(Math.hypot(dx, dy) < 150)
    }
    window.addEventListener('pointermove', move, { passive: true })
    return () => window.removeEventListener('pointermove', move)
  }, [ref])
  return { look, near }
}

type Line = [kind: string, text: string]
const SAID: Line = ['is-you', '> fix the failing date test']
const READ: Line[] = [
  ['is-step', '\u2022 Read src/date.ts'],
  ['is-step', '\u2022 Read test/date.test.ts'],
]
const DIFF: Line[] = [
  ['is-step', '\u2022 Edit src/date.ts'],
  ['is-del', '  3 -   return new Date(year, month, day)'],
  ['is-add', '  3 +   return new Date(year, month - 1, day)'],
]
/** The same run as the VS Code frames, as it reads in a terminal. */
const TERMINAL: Record<Frame, Line[]> = {
  typed: [SAID],
  working: [SAID, ...READ, ['is-dim', 'Thinking\u2026']],
  ask: [
    SAID,
    ...READ,
    ...DIFF,
    ['is-gap', ''],
    ['is-you', 'Make this edit to src/date.ts?'],
    ['is-pick', '> 1. Yes'],
    ['is-out', '  2. Yes, allow all edits this session'],
    ['is-out', '  3. No'],
  ],
  after: [SAID, ...READ, ...DIFF, ['is-dim', 'Modified']],
  done: [
    SAID,
    ...READ,
    ...DIFF,
    ['is-gap', ''],
    ['is-out', '\u2022 parseDate passed the 1-based month straight to Date, which expects 0-based months.'],
  ],
  denied: [SAID, ...READ, ...DIFF, ['is-gap', ''], ['is-out', '\u2022 The edit was declined, so nothing changed.']],
}

function Terminal({ frame }: { frame: Frame }) {
  return (
    <div className="term">
      <div className="term-bar">
        <span />
        <span />
        <span />
        <em>date-utils</em>
      </div>
      <div className="term-body">
        {TERMINAL[frame].map(([kind, text], i) => (
          <div key={i} className={kind}>
            {text}
          </div>
        ))}
        {(frame === 'typed' || frame === 'done' || frame === 'denied') && (
          <div className="is-you">
            {frame === 'typed' ? '' : '> '}
            <span className="caret" />
          </div>
        )}
      </div>
    </div>
  )
}

/** A scene or wave asked for on the row of moods: the page's own Kuchu joins in. */
const players = new Set<(name: string) => void>()

/** Its width in the hero, the largest it gets: elsewhere a transform makes it smaller, so that it can grow as it goes. */
const BIG = 230
/** The drawing's height for that width: 120 wide, 154 tall. */
const TALL = (BIG * 154) / 120
/** How fast it walks and climbs, in px a second, and how fast it falls when let go. */
const WALK_SPEED = 620
const CLIMB_SPEED = 900
const GRAVITY = 2800
const THROW_SPEED = 1100
/** How far above the bottom of the screen it stands. */
const FLOOR_PX = 8
/** Out of the hero once its place there comes this close to the top of the screen, under the header. */
const HERO_TOP_PX = 96
/** How long it keeps a section's mood at a time, unless it stays, and how long it rests before taking it up again. */
const REACTION_MS = 5200
const REST_MS = 4000
/** Let go in the hero, it stays this long before walking back to the card. */
const STAY_MS = 9000
/** How long the note saying it can be dragged stays, and how near the screen's edge its middle may come. */
const TIP_MS = 8000
const NOTE_EDGE_PX = 90
/** Petting: the pointer turning back this many times over it within this long, as on the desktop. */
const PET_TURNS = 4
const PET_WITHIN_MS = 1500
/** A short walk of its own: its speed in px a second, and how long it waits before coming back, as the core walks it. */
const STROLL_SPEED = 70
const STROLL_PAUSE_MS = 900

/** Where it goes on the way down the page. A section with a `spot` holds a place for it; without one, or with that place
 *  out of sight, it stands on the bottom of the screen, in the margin on one `side`, clear of the words. */
type Stop = { at: string; spot?: string; side: 'left' | 'right'; mood?: PetMood; scene?: string; stays?: boolean }
const STOPS: Stop[] = [
  { at: '.hero', spot: '.demo-pet', side: 'right' },
  { at: '.gains', side: 'right', mood: 'needs' },
  { at: '.life', side: 'left', scene: 'dance' },
  { at: '.more', side: 'right', mood: 'working' },
  { at: '.faq', side: 'left', mood: 'thinking' },
  { at: '.get', spot: '.get-pet', side: 'right', mood: 'done', stays: true },
]

const clamp = (n: number, low: number, high: number) => Math.min(high, Math.max(low, n))

/** Kuchu above the page. It runs on its own clock, outside React, so that moving it costs no rendering: React only hears
 *  when what it is doing changes. All distances are in screen px, its feet the point it stands on. */
function PagePet({ mood, asked }: { mood: PetMood; asked: number }) {
  const root = useRef<HTMLDivElement>(null)
  const note = useRef<HTMLDivElement>(null)
  const [walk, setWalk] = useState(0)
  const [over, setOver] = useState(false)
  const [home, setHome] = useState(true)
  const [reaction, setReaction] = useState<{ mood?: PetMood; scene?: string } | null>(null)
  const [scene, setScene] = useState<string>()
  const [waved, setWaved] = useState(0)
  const [petted, setPetted] = useState<number | undefined>()
  const [tip, setTip] = useState(false)
  const { look, near } = useLook(root)
  const stroke = useRef({ x: 0, way: 0, turns: [] as number[] })
  const m = useRef({
    x: 0,
    y: 0,
    scale: 1,
    vx: 0,
    vy: 0,
    lean: 0,
    /** Held by the pointer, where on the pet it took hold, and when it last moved. */
    held: false,
    moved: false,
    gx: 0,
    gy: 0,
    at: 0,
    /** Falling after being let go, and then staying where it landed until the page moves on. */
    falling: false,
    stayed: 0,
    /** Standing in a section's place, moving with the page. */
    docked: true,
    stop: 0,
    arrived: true,
    /** A short walk of its own, this far from where it stands. */
    aside: 0,
    tipped: false,
    dragged: false,
  }).current

  const stroll = useCallback(
    (d: number) => {
      if (m.docked || m.stayed) return Promise.resolve(0)
      const dx = m.x > window.innerWidth / 2 ? -d : d
      m.aside = dx
      setTimeout(() => (m.aside = 0), (d / STROLL_SPEED) * 1000 + STROLL_PAUSE_MS)
      return Promise.resolve(d)
    },
    [m],
  )

  // Asked from the row of moods, it plays along.
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined
    const play = (name: string) => {
      clearTimeout(timer)
      setScene(undefined)
      if (name === 'hello') return setWaved(Date.now())
      requestAnimationFrame(() => {
        setScene(name)
        timer = setTimeout(() => setScene(undefined), SCENE_MS[name] ?? 3000)
      })
    }
    players.add(play)
    return () => {
      players.delete(play)
      clearTimeout(timer)
    }
  }, [])

  useLayoutEffect(() => {
    const el = root.current
    if (!el) return
    const places = STOPS.map((s) => ({
      ...s,
      section: document.querySelector(s.at),
      place: s.spot ? document.querySelector(s.spot) : null,
    }))
    const column = document.querySelector('.gains')
    let zoom = 1
    const measure = () => (zoom = parseFloat(getComputedStyle(document.body).zoom) || 1)
    measure()
    window.addEventListener('resize', measure)

    let last = performance.now()
    let frame = 0
    let reacting: ReturnType<typeof setTimeout> | undefined
    let lastScroll = window.scrollY
    let flow = 0
    let walking = 0
    let walkedAt = 0
    let hint: ReturnType<typeof setTimeout> | undefined

    // Where it wants to be now, on the screen, and at what size.
    const goal = (i: number) => {
      const vw = document.documentElement.clientWidth
      const vh = window.innerHeight
      const p = places[i]
      const r = p.place?.getBoundingClientRect()
      if (r && r.width > 0 && r.top > HERO_TOP_PX && r.bottom < vh + r.height * 0.35)
        return { x: r.left + r.width / 2, y: r.bottom, scale: r.width / (BIG * zoom), place: true }
      const size = clamp(vw * 0.09, 84, 130)
      const margin = column ? column.getBoundingClientRect().left + 20 : 0
      const side = Math.max(size / 2 + 8, margin / 2)
      return { x: (p.side === 'right' ? vw - side : side) + m.aside, y: vh - FLOOR_PX, scale: size / (BIG * zoom), place: false }
    }

    const start = goal(0)
    m.x = start.x
    m.y = start.y
    m.scale = start.scale

    const tick = (now: number) => {
      frame = requestAnimationFrame(tick)
      const dt = Math.min((now - last) / 1000, 0.05) || 0.016
      last = now
      const vw = document.documentElement.clientWidth
      const vh = window.innerHeight

      // Which section the page has reached.
      let i = 0
      places.forEach((p, n) => {
        if (p.section && p.section.getBoundingClientRect().top < vh * 0.55) i = n
      })
      if (i !== m.stop) {
        m.stop = i
        m.arrived = false
        m.stayed = 0
        clearTimeout(reacting)
        setReaction(null)
      }

      const to = goal(i)
      const half = (BIG * zoom * m.scale) / 2
      if (m.held) {
        // The pointer moves it. Held still, it forgets the speed it was thrown at.
        m.vx *= 0.9
        m.vy *= 0.9
        if (m.moved) {
          const size = clamp(vw * 0.09, 84, 130) / (BIG * zoom)
          m.scale += (size - m.scale) * Math.min(1, dt * 8)
        }
      } else if (m.falling) {
        m.vy += GRAVITY * dt
        m.vx *= Math.pow(0.25, dt)
        m.x += m.vx * dt
        m.y += m.vy * dt
        if (m.x < half || m.x > vw - half) {
          m.x = clamp(m.x, half, vw - half)
          m.vx = -m.vx * 0.5
        }
        if (m.y >= vh - FLOOR_PX) {
          m.y = vh - FLOOR_PX
          if (Math.abs(m.vy) < 260) {
            m.vx = m.vy = 0
            m.falling = false
            m.stayed = now
            setWaved(Date.now())
          } else {
            m.vy = -m.vy * 0.34
            m.vx *= 0.7
          }
        }
      } else if (m.stayed && !(i === 0 && now - m.stayed > STAY_MS)) {
        // Put down, it stays there, on the screen, until the page reaches another section.
        m.vx = m.vy = 0
        m.y = Math.min(m.y, vh - FLOOR_PX)
      } else {
        m.stayed = 0
        if (m.docked && to.place) {
          m.x = to.x
          m.y = to.y
          m.scale = to.scale
          m.vx = m.vy = 0
        } else {
          m.docked = false
          const dx = to.x - m.x
          const dy = to.y - m.y
          m.vx = clamp(dx * 5, -WALK_SPEED, WALK_SPEED)
          m.vy = clamp(dy * 6, -CLIMB_SPEED, CLIMB_SPEED)
          m.x += m.vx * dt
          m.y += m.vy * dt
          m.scale += (to.scale - m.scale) * Math.min(1, dt * 5)
          if (Math.hypot(dx, dy) < 6) {
            if (to.place) m.docked = true
            if (!m.arrived) {
              m.arrived = true
              const s = places[i]
              // While the page stays here it keeps at what it does here, with a rest in between.
              const act = () => {
                setReaction({ mood: s.mood, scene: s.scene })
                if (s.stays) return
                reacting = setTimeout(
                  () => {
                    setReaction(null)
                    reacting = setTimeout(act, REST_MS)
                  },
                  s.scene ? (SCENE_MS[s.scene] ?? 3000) : REACTION_MS,
                )
              }
              if (s.mood || s.scene) act()
            }
          }
        }
      }
      setHome(i === 0 && m.docked)

      // The first time it comes out on its own, a note says it can be dragged.
      if (!m.tipped && !m.docked && !m.dragged && Math.abs(m.vx) < 30 && i > 0) {
        m.tipped = true
        setTip(true)
        hint = setTimeout(() => setTip(false), TIP_MS)
      }

      // The page moving past it: it leans, and walks on the spot, the way the page is going.
      flow = flow * 0.85 + ((window.scrollY - lastScroll) / dt) * 0.15
      lastScroll = window.scrollY
      const settled = m.docked || m.falling || !!m.stayed
      let legs = 0
      if (m.held || (!settled && Math.abs(m.vx) > 24)) legs = Math.abs(m.vx) > 24 ? Math.sign(m.vx) : 0
      else if (!settled && Math.abs(flow) > 80) legs = Math.sign(flow)
      if (legs !== walking && now - walkedAt > 160) {
        walking = legs
        walkedAt = now
        setWalk(legs)
      }
      const lean = m.docked ? 0 : clamp(flow * 0.006, -7, 7) + clamp(m.vx * 0.006, -6, 6)
      m.lean += (lean - m.lean) * Math.min(1, dt * 10)

      const x = (m.x - (BIG * zoom) / 2) / zoom
      const y = (m.y - TALL * zoom) / zoom
      el.style.transform = `translate3d(${x.toFixed(1)}px, ${y.toFixed(1)}px, 0) rotate(${m.lean.toFixed(2)}deg) scale(${m.scale.toFixed(4)})`
      if (note.current) {
        const across = clamp(m.x, NOTE_EDGE_PX, vw - NOTE_EDGE_PX)
        note.current.style.transform = `translate3d(${(across / zoom).toFixed(1)}px, ${((m.y - TALL * zoom * m.scale - 6) / zoom).toFixed(1)}px, 0) translate(-50%, -100%)`
      }
    }
    frame = requestAnimationFrame(tick)
    return () => {
      cancelAnimationFrame(frame)
      window.removeEventListener('resize', measure)
      clearTimeout(reacting)
      clearTimeout(hint)
    }
  }, [m])

  const down = (e: React.PointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId)
    m.held = true
    m.moved = false
    m.falling = false
    m.stayed = 0
    m.gx = e.clientX - m.x
    m.gy = e.clientY - m.y
    m.at = e.timeStamp
  }
  const move = (e: React.PointerEvent<HTMLDivElement>) => {
    if (m.held) {
      const x = e.clientX - m.gx
      const y = e.clientY - m.gy
      if (!m.moved && Math.hypot(x - m.x, y - m.y) <= 4) return
      m.moved = true
      m.dragged = true
      m.docked = false
      setTip(false)
      const dt = Math.max(e.timeStamp - m.at, 8) / 1000
      m.vx = m.vx * 0.6 + ((x - m.x) / dt) * 0.4
      m.vy = m.vy * 0.6 + ((y - m.y) / dt) * 0.4
      m.x = x
      m.y = y
      m.at = e.timeStamp
      return
    }
    // Petting: the pointer going to and fro over it.
    const s = stroke.current
    const way = Math.sign(e.clientX - s.x)
    s.x = e.clientX
    if (way === 0) return
    const now = performance.now()
    if (s.way !== 0 && way !== s.way) s.turns = [...s.turns.filter((t) => now - t < PET_WITHIN_MS), now]
    s.way = way
    if (s.turns.length >= PET_TURNS) {
      s.turns = []
      setPetted(Date.now())
    }
  }
  const up = () => {
    m.held = false
    if (m.moved) {
      m.vx = clamp(m.vx, -THROW_SPEED, THROW_SPEED)
      m.vy = clamp(m.vy, -THROW_SPEED, THROW_SPEED)
      m.falling = true
    } else {
      // A click: it waves, and stays in its place.
      setWaved(Date.now())
      m.stayed = m.docked ? 0 : performance.now()
    }
  }

  const shown = home ? mood : (reaction?.mood ?? 'idle')
  return (
    <>
      <div
        ref={root}
        className="page-pet"
        onPointerDown={down}
        onPointerMove={move}
        onPointerUp={up}
        onPointerCancel={up}
        onPointerEnter={() => setOver(true)}
        onPointerLeave={() => setOver(false)}
      >
        <Pet
          size={BIG}
          mood={scene || reaction?.scene ? 'idle' : shown}
          look={scene || reaction?.scene ? undefined : look}
          near={near}
          over={over}
          walk={walk}
          petted={petted}
          asked={Math.max(asked, waved)}
          scene={scene ?? reaction?.scene}
          stroll={stroll}
          lively
        />
      </div>
      <div ref={note} className={`page-pet-note${tip ? ' is-on' : ''}`}>
        Drag me anywhere
      </div>
    </>
  )
}

function HeroDemo() {
  const [mood, setMood] = useState<PetMood>('needs')
  const [card, setCard] = useState<Card | null>({ kind: 'ask' })
  const [frame, setFrame] = useState<Frame>('ask')
  const [asked, setAsked] = useState(0)
  const [paused, setPaused] = useState(false)
  const [surface, setSurface] = useState<Surface>('code')
  /** Chosen by hand, the screen stays. Otherwise each run shows the other one. */
  const chosen = useRef(false)
  const answer = useRef<((c: Choice) => void) | null>(null)
  const stage = useRef<HTMLDivElement>(null)
  const petRef = useRef<HTMLDivElement>(null)
  // The story holds still while paused, or while nobody can see it.
  const held = useRef({ paused: false, away: false })
  const size = usePetSize(230, 112, 190)
  const { look, near } = useLook(petRef)

  useEffect(() => {
    const seen = new IntersectionObserver(([e]) => (held.current.away = !e.isIntersecting))
    if (stage.current) seen.observe(stage.current)
    return () => seen.disconnect()
  }, [])

  useEffect(() => {
    let live = true
    const wait = async (ms: number) => {
      for (let left = ms; left > 0; ) {
        const slice = Math.min(left, WAIT_SLICE_MS)
        await new Promise((go) => setTimeout(go, slice))
        if (!live) throw new Error('gone')
        if (!held.current.paused && !held.current.away && !document.hidden) left -= slice
      }
    }
    const steps: Step[] = []
    const show = () => setCard({ kind: 'steps', steps: [...steps] })
    const step = async (text: string, ms: number) => {
      steps.push({ text, progress: 'running' })
      setMood('working')
      show()
      await wait(ms)
      steps[steps.length - 1].progress = 'done'
      show()
    }

    const run = async () => {
      for (let first = true; ; first = false) {
        steps.length = 0
        // It opens on the ask, the moment the page is about. From the second time round the run leads up to it.
        if (!first) {
          setCard(null)
          setMood('idle')
          if (!chosen.current) setSurface((s) => (s === 'code' ? 'term' : 'code'))
          setFrame('typed')
          await wait(2600)
          setFrame('working')
          setAsked(Date.now())
          setMood('thinking')
          setCard({ kind: 'steps', steps: [] })
          await wait(1800)
          await step('Reading src/date.ts', 1200)
          await step('Reading test/date.test.ts', 1200)
          setMood('thinking')
          await wait(900)
        }
        setFrame('ask')
        setMood('needs')
        setCard({ kind: 'ask' })
        // With reduced motion nothing moves on until it is asked to.
        const choice = await Promise.race([
          new Promise<Choice>((go) => (answer.current = go)),
          reduced() ? new Promise<Choice>(() => {}) : wait(ASK_MS).then(() => 'allow' as Choice),
        ])
        answer.current = null
        if (choice === 'terminal') {
          setCard(null)
          await wait(1400)
        }
        if (choice === 'deny') {
          setFrame('denied')
          setMood('done')
          setCard({ kind: 'answer', text: 'The edit was declined, so nothing changed.' })
        } else {
          setFrame('after')
          await step('Editing src/date.ts', 1500)
          setFrame('done')
          setMood('done')
          setCard({ kind: 'answer', text: 'parseDate passed the 1-based month straight to Date, which expects 0-based months.' })
        }
        if (reduced()) return
        await wait(6000)
      }
    }
    run().catch(() => {})
    return () => {
      live = false
    }
  }, [])

  const choose = (c: Choice) => answer.current?.(c)
  const pause = () => {
    held.current.paused = !paused
    setPaused(!paused)
  }
  const show = (s: Surface) => {
    chosen.current = true
    setSurface(s)
  }

  return (
    <div ref={stage} className="stage" role="group" aria-label="A demo of Kuchu following a Claude Code session in VS Code">
      <div className="desk-window">
        <div className="surface" role="group" aria-label="Where Claude Code is running">
          <button type="button" aria-pressed={surface === 'code'} onClick={() => show('code')}>
            VS Code
          </button>
          <button type="button" aria-pressed={surface === 'term'} onClick={() => show('term')}>
            Terminal
          </button>
        </div>
        <div
          className="window"
          role="img"
          aria-label={surface === 'code' ? 'Claude Code in the side panel of VS Code, working on a failing date test' : 'Claude Code in a terminal, working on a failing date test'}
        >
          {FRAMES.map((f) => (
            <img key={f} className={surface === 'code' && f === frame ? 'is-on' : undefined} src={frameUrl(f)} width={1898} height={1039} alt="" />
          ))}
          {surface === 'term' && <Terminal frame={frame} />}
        </div>
        <div className="pet-spot">
          {card && (
            <div className={`demo-card${card.kind === 'ask' ? ' is-ask' : ''}`}>
              {card.kind === 'ask' ? (
                <>
                  <div className="ask-head">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                      <path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z" />
                      <path d="M12 8v4M12 16h.01" />
                    </svg>
                    <strong>{SESSION}</strong>
                  </div>
                  <div className="ask-where">{FOLDER}</div>
                  <div className="ask-what">Editing date.ts</div>
                  <code className="ask-detail">{FILE}</code>
                  <div className="ask-buttons">
                    <button type="button" className="btn is-primary is-small" onClick={() => choose('allow')}>
                      Allow
                    </button>
                    <button type="button" className="btn is-small ask-extra" onClick={() => choose('allow')}>
                      Allow edits for now
                    </button>
                    <button type="button" className="btn is-small" onClick={() => choose('deny')}>
                      Deny
                    </button>
                    <button type="button" className="btn is-small is-quiet ask-extra" onClick={() => choose('terminal')}>
                      In Claude Code
                    </button>
                  </div>
                </>
              ) : (
                <>
                  <strong className="demo-title">{SESSION}</strong>
                  {card.kind === 'answer' ? (
                    <span className="demo-answer">{card.text}</span>
                  ) : card.steps.length === 0 ? (
                    <span className="demo-step is-said">Thinking…</span>
                  ) : (
                    card.steps.slice(-3).map((s, i) => (
                      <span key={i} className={`demo-step${i === Math.min(card.steps.length, 3) - 1 ? ' is-now' : ''}`}>
                        <Mark progress={s.progress} />
                        {s.text}
                      </span>
                    ))
                  )}
                </>
              )}
            </div>
          )}
          {/* Kuchu's place: on the page it stands here from above, so that it can walk off. */}
          <div ref={petRef} className="demo-pet" style={roams ? { width: size, height: (size * TALL) / BIG } : undefined}>
            {!roams && <Pet size={size} mood={mood} look={look} near={near} asked={asked} lively />}
          </div>
        </div>
      </div>
      {roams && createPortal(<PagePet mood={mood} asked={asked} />, document.body)}
      <button type="button" className="demo-pause" aria-label={paused ? 'Play demo' : 'Pause demo'} onClick={pause}>
        <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          {paused ? (
            <path d="M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z" />
          ) : (
            <>
              <rect x="14" y="3" width="5" height="18" rx="1" />
              <rect x="5" y="3" width="5" height="18" rx="1" />
            </>
          )}
        </svg>
      </button>
    </div>
  )
}

/** Scenes the pet plays now and then on its own, on demand. */
const SCENES: [string, string][] = [
  ['hello', 'Wave'],
  ['dance', 'Dance'],
  ['flip', 'Flip its screen'],
  ['pop', 'Pop up'],
  ['glitch', 'Glitch'],
  ['yawn', 'Yawn'],
]

/** What each mood means, for the row of pets. */
const MOODS: [PetMood, string, string][] = [
  ['thinking', 'Thinks', 'while Claude reads'],
  ['working', 'Works', 'while tools run'],
  ['needs', 'Asks', 'when Claude needs a yes'],
  ['done', 'Cheers', 'when Claude is done'],
]

function MoodPet({ mood, scene, asked }: { mood: PetMood; scene?: string; asked: number }) {
  const ref = useRef<HTMLDivElement>(null)
  const size = usePetSize(210, 132, 150)
  const { look, near } = useLook(ref)
  // A scene plays from rest, so each one leaves its mood for as long as the scene lasts.
  return (
    <div ref={ref} className="mood-pet">
      <Pet size={size} mood={scene ? 'idle' : mood} look={scene ? undefined : look} near={near} scene={scene} asked={asked} />
    </div>
  )
}

/** The row of moods, and the buttons that set the whole row playing. The last one asked for stays lit. */
function LifePets() {
  const [scene, setScene] = useState<string>()
  const [asked, setAsked] = useState(0)
  const [lit, setLit] = useState(SCENES[0][0])
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const play = (name: string) => {
    clearTimeout(timer.current)
    setScene(undefined)
    setLit(name)
    players.forEach((p) => p(name))
    if (name === 'hello') return setAsked(Date.now())
    requestAnimationFrame(() => {
      setScene(name)
      timer.current = setTimeout(() => setScene(undefined), SCENE_MS[name] ?? 3000)
    })
  }
  return (
    <>
      <div className="play-buttons" role="group" aria-label="Make Kuchu play">
        {SCENES.map(([name, label]) => (
          <button key={name} type="button" className={`btn${name === lit ? ' is-primary' : ''}`} onClick={() => play(name)}>
            {label}
          </button>
        ))}
      </div>
      <ul className="moods">
        {MOODS.map(([mood, verb, when]) => (
          <li key={mood}>
            <MoodPet mood={mood} scene={scene} asked={asked} />
            <strong>{verb}</strong>
            <span>{when}</span>
          </li>
        ))}
      </ul>
    </>
  )
}

/** Kuchu asking for you, at the size of the panel it stands in. */
function NeedsPet() {
  return <Pet size={usePetSize(226, 124, 190)} mood="needs" />
}

/** Waves as it comes into view, the page's goodbye, when Kuchu is not walking the page. */
function WavePet() {
  const [asked, setAsked] = useState(0)
  const ref = useRef<HTMLDivElement>(null)
  const size = usePetSize(212, 124)
  const { look, near } = useLook(ref)
  useEffect(() => {
    const seen = new IntersectionObserver(([e]) => e.isIntersecting && setAsked(Date.now()), { threshold: 0.6 })
    if (ref.current) seen.observe(ref.current)
    return () => seen.disconnect()
  }, [])
  return (
    <div ref={ref}>
      <Pet size={size} look={look} near={near} asked={asked} lively />
    </div>
  )
}

const PETS: Record<string, React.ReactNode> = {
  hero: <HeroDemo />,
  life: <LifePets />,
  needs: <NeedsPet />,
  wave: roams ? null : <WavePet />,
}

for (const el of document.querySelectorAll<HTMLElement>('[data-pet]')) {
  el.replaceChildren()
  createRoot(el).render(PETS[el.dataset.pet ?? 'hero'])
}
