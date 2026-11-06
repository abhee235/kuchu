// The live pets on the home page, all Kuchu's own pet from the app's source: one standing in front of an editor where a
// Claude Code session runs, with a copy of its card, so that the session's edit lands in the file once it is allowed; one
// showing how it asks for you; one that plays its scenes when asked; and one that waves as you reach the end. Their eyes
// follow the pointer, as they do on the desktop.

import { useEffect, useRef, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { Pet, SCENE_MS } from '../../app/src/pet/Pet'
import { lookToward, type PetMood } from '../../app/src/pet/petMood'

type Progress = 'running' | 'done' | 'failed'
type Step = { text: string; progress: Progress }
type Card = { kind: 'steps'; steps: Step[] } | { kind: 'ask' } | { kind: 'answer'; text: string }
type Choice = 'allow' | 'deny' | 'terminal'

const SESSION = 'Fix the failing date test'
const PROMPT = 'fix the failing date test'
/** How long the card waits for a click on Allow before the demo answers it itself. */
const ASK_MS = 7000
/** A long wait in the demo is cut into these, so that a pause takes hold at once. */
const WAIT_SLICE_MS = 100

/** The file open in the editor. The line Claude changes stands apart, so it can show as a change. */
const CODE_BEFORE = ['export function parseDate(text: string): Date {', "  const [year, month, day] = text.split('-').map(Number)"]
const LINE_OLD = '  return new Date(year, month, day)'
const LINE_NEW = '  return new Date(year, month - 1, day)'
const CODE_AFTER = [
  '}',
  '',
  'export function addDays(date: Date, days: number): Date {',
  '  const next = new Date(date)',
  '  next.setDate(next.getDate() + days)',
  '  return next',
  '}',
  '',
  'export function isWeekend(date: Date): boolean {',
  '  const day = date.getDay()',
  '  return day === 0 || day === 6',
  '}',
]
const TOKENS = /(\b(?:export|function|const|return|new)\b|'[^']*'|\b\d+\b|\b[A-Z]\w*\b|\b\w+(?=\())/g

/** The editor's side bar: files, search, source control, extensions. */
const SIDE_BAR = [
  'M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7zM14 2v5a1 1 0 0 0 1 1h5',
  'm21 21-4.3-4.3M19 11a8 8 0 1 1-16 0 8 8 0 0 1 16 0',
  'M6 3v12M18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6M6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6M18 9a9 9 0 0 1-9 9',
  'M3 3h7v7H3zM14 3h7v7h-7zM14 14h7v7h-7zM3 14h7v7H3z',
]

const reduced = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches
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

/** A line of the file, coloured as an editor would. */
function Code({ text }: { text: string }) {
  return (
    <>
      {text.split(TOKENS).map((part, i) => {
        if (i % 2 === 0) return part
        const kind = part.startsWith("'") ? 'is-text' : /^\d/.test(part) ? 'is-num' : /^[A-Z]/.test(part) ? 'is-type' : /^(export|function|const|return|new)$/.test(part) ? 'is-key' : 'is-call'
        return (
          <span key={i} className={kind}>
            {part}
          </span>
        )
      })}
    </>
  )
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

function HeroDemo() {
  const [mood, setMood] = useState<PetMood>('idle')
  const [card, setCard] = useState<Card | null>(null)
  const [term, setTerm] = useState<string[]>([])
  const [typed, setTyped] = useState('')
  const [asked, setAsked] = useState(0)
  const [paused, setPaused] = useState(false)
  const [edit, setEdit] = useState<'before' | 'editing' | 'after'>('before')
  const answer = useRef<((c: Choice) => void) | null>(null)
  const stage = useRef<HTMLDivElement>(null)
  const petRef = useRef<HTMLDivElement>(null)
  // The story holds still while paused, or while nobody can see it.
  const held = useRef({ paused: false, away: false })
  const size = usePetSize(236, 132, 176)
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
    const say = (line: string) => setTerm((t) => [...t, line])
    const steps: Step[] = []
    const show = () => setCard({ kind: 'steps', steps: [...steps] })
    const step = async (text: string, line: string, ms: number, end: Progress = 'done') => {
      steps.push({ text, progress: 'running' })
      setMood('working')
      show()
      say(line)
      await wait(ms)
      steps[steps.length - 1].progress = end
      show()
    }

    const run = async () => {
      for (;;) {
        steps.length = 0
        setTerm([])
        setTyped('')
        setCard(null)
        setMood('idle')
        setEdit('before')
        await wait(1400)
        for (let i = 1; i <= PROMPT.length; i++) {
          setTyped(PROMPT.slice(0, i))
          await wait(reduced() ? 0 : 55)
        }
        await wait(500)
        say(`> ${PROMPT}`)
        setTyped('')
        setAsked(Date.now())
        setMood('thinking')
        setCard({ kind: 'steps', steps: [] })
        await wait(2200)
        await step('Reading src/date.ts', 'Reading src/date.ts', 1500)
        await step('Running npm test', 'Running npm test', 1800, 'failed')
        say('  1 failed, 23 passed')
        setMood('thinking')
        await wait(1600)
        say('Wants to edit src/date.ts')
        setMood('needs')
        setCard({ kind: 'ask' })
        const choice = await Promise.race([
          new Promise<Choice>((go) => (answer.current = go)),
          wait(ASK_MS).then(() => 'allow' as Choice),
        ])
        answer.current = null
        if (choice === 'terminal') {
          setCard(null)
          setMood('needs')
          say('Do you want to make this edit? Yes')
          await wait(1400)
        }
        if (choice === 'deny') {
          say('Edit denied')
          setMood('done')
          setCard({ kind: 'answer', text: 'Left date.ts as it was. The test still fails at parseDate.' })
          say('Left date.ts as it was. The test still fails at parseDate.')
        } else {
          setEdit('editing')
          await step('Editing src/date.ts', 'Editing src/date.ts', 1400)
          setEdit('after')
          await step('Running npm test', 'Running npm test', 1700)
          say('  24 passed')
          setMood('done')
          setCard({ kind: 'answer', text: 'Fixed the off-by-one in parseDate. All 24 tests pass.' })
          say('Fixed the off-by-one in parseDate. All 24 tests pass.')
        }
        await wait(5200)
        setCard(null)
        setMood('idle')
        await wait(5000)
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

  return (
    <div ref={stage} className="stage" role="group" aria-label="A demo of Kuchu following a Claude Code session">
      <div className="ide" aria-hidden="true">
        <div className="ide-title">
          <span className="ide-menu">File Edit Selection View</span>
          <em>date.ts - date-utils - Visual Studio Code</em>
        </div>
        <div className="ide-body">
          <div className="ide-bar">
            {SIDE_BAR.map((d) => (
              <svg key={d} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
                <path d={d} />
              </svg>
            ))}
          </div>
          <div className="ide-claude">
            <div className="ide-head">Claude Code</div>
            <div className="ide-log">
              {term.map((line, i) => (
                <div key={i} className={line.startsWith('>') ? 'is-you' : line.startsWith('  ') ? 'is-out' : ''}>
                  {line.replace(/^> /, '')}
                </div>
              ))}
            </div>
            <div className="ide-input">
              {typed ? typed : <span>Ask Claude to edit your code</span>}
              {typed && <i className="caret" />}
            </div>
          </div>
          <div className="ide-file">
            <div className="ide-tab">date.ts</div>
            <div className="ide-code">
              {[
                ...CODE_BEFORE.map((text) => [text, '']),
                ...(edit === 'before' ? [[LINE_OLD, '']] : edit === 'editing' ? [[LINE_OLD, 'is-gone'], [LINE_NEW, 'is-new']] : [[LINE_NEW, '']]),
                ...CODE_AFTER.map((text) => [text, '']),
              ].map(([text, kind], i) => (
                <div key={i} className={`ide-line ${kind}`}>
                  <b>{i + 1}</b>
                  <span>
                    <Code text={text} />
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>
        <div className="ide-status">
          <span>main</span>
          <span>TypeScript</span>
        </div>
      </div>
      <button type="button" className="btn is-small is-quiet demo-pause" aria-pressed={paused} onClick={pause}>
        {paused ? 'Play demo' : 'Pause demo'}
      </button>
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
                <div className="ask-what">Editing date.ts</div>
                <code className="ask-detail">src/date.ts</code>
                <div className="ask-buttons">
                  <button type="button" className="btn is-primary is-small" onClick={() => choose('allow')}>
                    Allow
                  </button>
                  <button type="button" className="btn is-small" onClick={() => choose('deny')}>
                    Deny
                  </button>
                  <button type="button" className="btn is-small is-quiet" onClick={() => choose('terminal')}>
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
        <div ref={petRef} className="demo-pet">
          <Pet size={size} mood={mood} look={look} near={near} asked={asked} lively />
        </div>
      </div>
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

function PlayPet() {
  const [scene, setScene] = useState<string>()
  const [asked, setAsked] = useState(0)
  const ref = useRef<HTMLDivElement>(null)
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const size = usePetSize(204, 156)
  const { look, near } = useLook(ref)
  const play = (name: string) => {
    clearTimeout(timer.current)
    setScene(undefined)
    if (name === 'hello') return setAsked(Date.now())
    requestAnimationFrame(() => {
      setScene(name)
      timer.current = setTimeout(() => setScene(undefined), SCENE_MS[name] ?? 3000)
    })
  }
  return (
    <div className="play">
      <div ref={ref} className="play-pet">
        <Pet size={size} look={scene ? undefined : look} near={near} scene={scene} asked={asked} lively />
      </div>
      <div className="play-buttons" role="group" aria-label="Make Kuchu play">
        {SCENES.map(([name, label]) => (
          <button key={name} type="button" className="btn is-small" onClick={() => play(name)}>
            {label}
          </button>
        ))}
      </div>
    </div>
  )
}

/** Waves as it comes into view, the page's goodbye. */
function WavePet() {
  const [asked, setAsked] = useState(0)
  const ref = useRef<HTMLDivElement>(null)
  const size = usePetSize(168, 124)
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
  play: <PlayPet />,
  needs: <Pet size={76} mood="needs" />,
  wave: <WavePet />,
}

for (const el of document.querySelectorAll<HTMLElement>('[data-pet]')) {
  el.replaceChildren()
  createRoot(el).render(PETS[el.dataset.pet ?? 'hero'])
}
