// The pet's window: Kuchu in a bottom corner of a small transparent window above every other one, with what Claude
// Code is doing beside it (one line, or a few) and a card for each permission request. The core sizes and places the
// window, reads where the pointer is (for the eyes, and to let clicks through everywhere but the pet and its cards),
// and sends the sessions as one view; this page draws them, moves the pet, and plays its tones.

import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { core, on, SIZES, type Approval, type Card, type Prefs, type View } from './api'
import { Pet } from './pet/Pet'
import { lookToward } from './pet/petMood'
import { playTone, toneFor } from './pet/petSound'
import type { PetMood } from './pet/petMood'

interface Pointer {
  dx: number
  dy: number
  over: boolean
}

/** How far the pointer moves with the button down before it is a drag, not a click. */
const DRAG_PX = 4
/** A click this soon after a card appears does not count: it was meant for what was there before. */
const SETTLING_MS = 600

const EMPTY: View = { mood: 'idle', card: null, approvals: [], sessions: [] }

export function PetWindow() {
  const [view, setView] = useState<View>(EMPTY)
  const [prefs, setPrefs] = useState<Prefs | null>(null)
  const [right, setRight] = useState(true)
  const [look, setLook] = useState({ x: 0, y: 0 })
  const [over, setOver] = useState(false)
  const [closed, setClosed] = useState<string | null>(null)

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
      }),
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
            card && <SessionCard card={card} lines={prefs?.card === 'lines'} onClose={() => setClosed(cardKey(card))} />
          )}
        </div>
      )}
      <div
        ref={petRef}
        className={`bubble${over && prefs?.seeThrough ? ' is-over' : ''}`}
        role="button"
        aria-label={`Kuchu: ${mood}`}
        onPointerDown={(e) => {
          if (e.button === 0) press.current = { x: e.screenX, y: e.screenY }
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
        <Pet size={width} mood={mood} look={look} />
      </div>
    </div>
  )
}

function cardKey(card: Card) {
  return `${card.session}:${card.mood}`
}

/** Which project, and what it is doing now (or the answer's first lines once done). A click opens the sessions. */
function SessionCard({ card, lines, onClose }: { card: Card; lines: boolean; onClose: () => void }) {
  const more = lines ? card.lines.filter((l) => l !== card.line).slice(-2) : []
  return (
    <div className={`card is-${card.mood}`} role="status" aria-live="polite">
      <button type="button" className="card-body" onClick={() => void core.openSessions()} title="Open the sessions">
        <span className="card-title">
          <span className="dot" aria-hidden="true" />
          {card.title}
        </span>
        {more.map((l, i) => (
          <span key={i} className="card-line is-past">
            {l}
          </span>
        ))}
        <span className="card-line">{card.line}</span>
      </button>
      <button type="button" className="card-close" aria-label="Hide this" title="Hide this" onClick={onClose}>
        ×
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
  const heading =
    approval.kind === 'question' ? 'Has a question' : approval.kind === 'plan' ? 'Has a plan ready' : approval.title
  return (
    <div className="card approval" role="alertdialog" aria-label={`${approval.project}: ${heading}`}>
      <div className="card-title">
        <span className="dot" aria-hidden="true" />
        {approval.project}
        {more > 0 && <span className="more">+{more} waiting</span>}
      </div>
      <div className="approval-what">{approval.question ? approval.question.text : heading}</div>
      {approval.detail && approval.kind !== 'question' && <pre className="approval-detail">{approval.detail}</pre>}
      <div className="approval-actions">
        {approval.kind === 'question' && approval.question ? (
          approval.question.options.map((o) => (
            <button key={o} type="button" className="btn" onClick={() => choose('answer', o)}>
              {o}
            </button>
          ))
        ) : approval.kind === 'question' ? null : (
          <>
            <button type="button" className="btn is-primary" onClick={() => choose('allow')}>
              {approval.kind === 'plan' ? 'Approve plan' : 'Allow'}
            </button>
            {approval.always && (
              <button type="button" className="btn" onClick={() => choose('always')}>
                {approval.always}
              </button>
            )}
            <button type="button" className="btn" onClick={() => choose('deny')}>
              {approval.kind === 'plan' ? 'Keep planning' : 'Deny'}
            </button>
          </>
        )}
        <button type="button" className="btn is-quiet" onClick={() => choose('terminal')} title="Answer it in Claude Code instead">
          In Claude Code
        </button>
      </div>
    </div>
  )
}
