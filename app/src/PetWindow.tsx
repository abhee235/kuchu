// The pet's window: Kuchu in a bottom corner of a small transparent window above every other one, with what Claude
// Code is doing beside it (one line, or a few) and a card for each permission request. The core sizes and places the
// window, reads where the pointer is (for the eyes, and to let clicks through everywhere but the pet and its cards),
// and sends the sessions as one view; this page draws them, moves the pet, and plays its tones.

import { getCurrentWindow } from '@tauri-apps/api/window'
import { List, MessageCircleQuestion, ShieldAlert, X } from 'lucide-react'
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { core, on, SIZES, type Approval, type Card, type Prefs, type View } from './api'
import { Pet } from './pet/Pet'
import { lookToward } from './pet/petMood'
import { playTone, toneFor } from './pet/petSound'
import { SaidMark, StepMark } from './StepMark'
import type { PetMood } from './pet/petMood'

interface Pointer {
  dx: number
  dy: number
  over: boolean
  card: boolean
}

/** How far the pointer moves with the button down before it is a drag, not a click. */
const DRAG_PX = 4
/** A click this soon after a card appears does not count: it was meant for what was there before. */
const SETTLING_MS = 600
/** How long an answer stays beside the pet (unless Settings keeps it), then how long it takes to fade. */
const ANSWER_MS = 8000
const FADE_MS = 350
/** The pointer this close to the pet's centre wakes it from a nap. */
const NEAR_PX = 150
/** Petting: the pointer turning back this many times over the pet within this long. */
const PET_TURNS = 4
const PET_WITHIN_MS = 1500

const EMPTY: View = { mood: 'idle', card: null, approvals: [], sessions: [] }

export function PetWindow() {
  const [view, setView] = useState<View>(EMPTY)
  const [prefs, setPrefs] = useState<Prefs | null>(null)
  const [right, setRight] = useState(true)
  const [look, setLook] = useState({ x: 0, y: 0 })
  const [over, setOver] = useState(false)
  const [onCard, setOnCard] = useState(false)
  const [near, setNear] = useState(false)
  const [walk, setWalk] = useState(0)
  const [petted, setPetted] = useState<number | undefined>()
  const stroke = useRef({ x: 0, way: 0, turns: [] as number[] })
  const [closed, setClosed] = useState<string | null>(null)
  const [leaving, setLeaving] = useState<string | null>(null)

  useEffect(() => {
    void core.view().then(setView)
    void core.prefs().then(setPrefs)
    void core.isRight().then(setRight)
    const offs = [
      on<View>('view', setView),
      on<Prefs>('prefs', setPrefs),
      on<boolean>('side', setRight),
      on<Pointer>('pointer', (p) => {
        setLook(lookToward(p))
        setOver(p.over)
        setOnCard(p.card)
        setNear(Math.hypot(p.dx, p.dy) < NEAR_PX)
        // Petting: the pointer going to and fro over the pet.
        const s = stroke.current
        const way = Math.sign(p.dx - s.x)
        s.x = p.dx
        if (!p.over || way === 0) return
        const now = performance.now()
        if (s.way !== 0 && way !== s.way) s.turns = [...s.turns.filter((t) => now - t < PET_WITHIN_MS), now]
        s.way = way
        if (s.turns.length >= PET_TURNS) {
          s.turns = []
          setPetted(Date.now())
        }
      }),
      on<number>('walk', setWalk),
    ]
    return () => offs.forEach((off) => off())
  }, [])

  const mood = view.mood
  // A small tone when the pet starts to need you, or a session is done or failed.
  const was = useRef<PetMood>(mood)
  useEffect(() => {
    const tone = toneFor(was.current, mood)
    was.current = mood
    if (tone && prefs?.sounds) playTone(tone)
  }, [mood, prefs?.sounds])

  const approval = view.approvals[0]
  const card = view.card && prefs?.card !== 'off' && cardKey(view.card) !== closed ? view.card : null
  const wide = !!approval || !!card
  useEffect(() => void core.setWide(wide), [wide])

  // An answer shows for a few seconds, then fades, unless Settings keeps it until the pet is clicked. While the
  // pointer is on it, it stays.
  const key = card ? cardKey(card) : null
  const brief = card?.mood === 'done' && !!prefs && !prefs.keepAnswer
  useEffect(() => {
    if (!brief || onCard) return
    const fade = setTimeout(() => setLeaving(key), ANSWER_MS)
    const gone = setTimeout(() => setClosed(key), ANSWER_MS + FADE_MS)
    return () => {
      clearTimeout(fade)
      clearTimeout(gone)
      setLeaving(null)
    }
  }, [brief, onCard, key])

  // What takes the mouse: the pet first, then the card beside it. Measured after every layout (and when the core
  // resizes the window), sent only when it changed.
  const petRef = useRef<HTMLDivElement>(null)
  const sideRef = useRef<HTMLDivElement>(null)
  const sent = useRef('')
  const [, setResized] = useState(0)
  useLayoutEffect(() => {
    const parts = [petRef.current, sideRef.current].filter((el): el is HTMLDivElement => !!el)
    const rects = parts.map((el) => {
      const r = el.getBoundingClientRect()
      return { x: Math.round(r.left), y: Math.round(r.top), w: Math.round(r.width), h: Math.round(r.height) }
    })
    const key = JSON.stringify(rects)
    if (key !== sent.current) {
      sent.current = key
      void core.setRects(rects)
    }
  })
  useEffect(() => {
    const again = () => setResized((n) => n + 1)
    window.addEventListener('resize', again)
    return () => window.removeEventListener('resize', again)
  }, [])

  // A stroll goes toward the middle of the screen: left from the right half, right from the left.
  const stroll = useCallback((d: number) => core.stroll(right ? -d : d), [right])

  const press = useRef<{ x: number; y: number } | null>(null)
  const width = SIZES[prefs?.size ?? 'base']
  const height = Math.round((width * 154) / 120)
  const stage = { '--pet-w': `${width}px`, '--pet-h': `${height}px` } as React.CSSProperties
  const side = right ? 'is-right' : 'is-left'

  return (
    <div className={`stage ${side}${wide ? ' is-wide' : ''}`} style={stage}>
      {wide && (
        <div ref={sideRef} className="beside">
          {approval ? (
            <ApprovalCard key={approval.id} approval={approval} more={view.approvals.length - 1} />
          ) : (
            card && <SessionCard card={card} lines={prefs?.card === 'lines'} leaving={leaving === key} onClose={() => setClosed(key)} />
          )}
        </div>
      )}
      <div
        ref={petRef}
        className={`bubble${over && prefs?.seeThrough ? ' is-over' : ''}`}
        role="button"
        aria-label={`Kuchu: ${mood}`}
        onPointerDown={(e) => {
          if (e.button !== 0) return
          press.current = { x: e.screenX, y: e.screenY }
          if (walk) void core.stopStroll()
        }}
        onPointerMove={(e) => {
          const p = press.current
          if (!p || Math.hypot(e.screenX - p.x, e.screenY - p.y) < DRAG_PX) return
          press.current = null
          void getCurrentWindow().startDragging()
        }}
        onPointerUp={() => {
          if (!press.current) return
          press.current = null
          void core.petClicked()
        }}
      >
        <Pet
          size={width}
          mood={mood}
          look={look}
          lively={prefs?.lively ?? true}
          near={near}
          over={over}
          walk={walk}
          petted={petted}
          stroll={stroll}
        />
      </div>
    </div>
  )
}

/** One card per session, mood and turn: a card hidden in one turn comes back in the next. */
function cardKey(card: Card) {
  return `${card.session}:${card.mood}:${card.at}`
}

/** Which session, and what it is doing now (or the answer's first lines once done). A click opens the sessions. */
function SessionCard({ card, lines, leaving, onClose }: { card: Card; lines: boolean; leaving: boolean; onClose: () => void }) {
  // Steps read up to the one under way, each marked running, done or failed, with what Claude said between them; an
  // answer reads down from its first line.
  const answer = card.mood === 'done'
  const last = card.lines[card.lines.length - 1]
  const rest = answer ? card.lines.filter((l) => l.text !== card.line) : last?.text === card.line ? card.lines.slice(0, -1) : card.lines
  const more = (lines ? (answer ? rest.slice(0, 2) : rest.slice(-2)) : []).map((l, i) => (
    <span key={i} className="card-line is-past">
      {l.progress ? <StepMark progress={l.progress} /> : !answer && <SaidMark />}
      <span className="card-text">{l.text}</span>
    </span>
  ))
  return (
    <div className={`card${lines ? ' is-lines' : ''}${leaving ? ' is-leaving' : ''}`} role="status" aria-live="polite">
      <button type="button" className="card-body" onClick={() => void core.openSessions(card.session)} title="Your sessions">
        <span className="card-title">{card.title}</span>
        {!answer && more}
        <span className="card-line">
          {card.progress && <StepMark progress={card.progress} />}
          <span className="card-text">{card.line}</span>
        </span>
        {answer && more}
      </button>
      <span className="card-actions">
        <button type="button" aria-label="Your sessions" title="Your sessions" onClick={() => void core.openSessions()}>
          <List size={16} />
        </button>
      </span>
      <button type="button" className="card-close" aria-label="Hide this" title="Hide this" onClick={onClose}>
        <X size={12} strokeWidth={3} />
      </button>
    </div>
  )
}

/** A permission request: what it wants to do, on what, and the choices. Nothing is decided without a click. */
function ApprovalCard({ approval, more }: { approval: Approval; more: number }) {
  const shownAt = useRef(0)
  useEffect(() => {
    // Told once it is on the screen; until then the request is the terminal's.
    const frame = requestAnimationFrame(() => {
      shownAt.current = performance.now()
      void core.approvalShown(approval.id)
    })
    return () => cancelAnimationFrame(frame)
  }, [approval.id])
  const choose = (kind: Parameters<typeof core.decide>[1], answer?: string) => {
    if (performance.now() - shownAt.current < SETTLING_MS) return
    void core.decide(approval.id, kind, answer)
  }
  const what = approval.kind === 'question' ? approval.question?.text || 'Has a question for you' : approval.kind === 'plan' ? 'Has a plan ready' : approval.title
  return (
    <div className="approval" role="alertdialog" aria-label={`${approval.project}: ${what}`}>
      <div className="approval-head">
        {approval.kind === 'question' ? <MessageCircleQuestion size={18} /> : <ShieldAlert size={18} />}
        <span className="name">{approval.project}</span>
        {more > 0 && <span className="more">+{more} waiting</span>}
      </div>
      <div className="approval-what">{what}</div>
      {approval.detail && approval.kind !== 'question' && <pre className="approval-detail">{approval.detail}</pre>}
      <div className="approval-actions">
        {approval.kind === 'question' && approval.question ? (
          approval.question.options.map((o, i) => (
            <button key={o} type="button" className={`btn is-small${i === 0 ? ' is-primary' : ''}`} onClick={() => choose('answer', o)}>
              {o}
            </button>
          ))
        ) : approval.kind === 'question' ? null : (
          <>
            <button type="button" className="btn is-small is-primary" onClick={() => choose('allow')}>
              {approval.kind === 'plan' ? 'Approve plan' : 'Allow'}
            </button>
            {approval.always && (
              <button type="button" className="btn is-small" onClick={() => choose('always')}>
                {approval.always}
              </button>
            )}
            <button type="button" className="btn is-small" onClick={() => choose('deny')}>
              {approval.kind === 'plan' ? 'Keep planning' : 'Deny'}
            </button>
          </>
        )}
        <button type="button" className="btn is-small is-quiet" onClick={() => choose('terminal')} title="Answer it in Claude Code instead">
          In Claude Code
        </button>
      </div>
    </div>
  )
}
