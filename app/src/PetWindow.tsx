// The pet's window: Kuchu in a bottom corner of a small transparent window above every other one, with what Claude
// Code is doing beside it (one line, or a few) and a card for each permission request. The core sizes and places the
// window, reads where the pointer is (for the eyes, and to let clicks through everywhere but the pet and its cards),
// and sends the sessions as one view; this page draws them, moves the pet, and plays its tones.

import { getCurrentWindow } from '@tauri-apps/api/window'
import { List, MessageCircleQuestion, ShieldAlert, X } from 'lucide-react'
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { core, on, SIZES, type Approval, type Card, type Line, type Prefs, type Progress, type View } from './api'
import { Pet } from './pet/Pet'
import { lookToward } from './pet/petMood'
import { playTone } from './pet/petSound'
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
/** Tones closer together than this ring once. */
const TONE_GAP_MS = 1200
/** Claude thinks again between its steps, but one tool handing over to the next passes through thinking for a moment
 *  too: after work, the pet shows thinking only once it has lasted this long. */
const THINK_AFTER_MS = 800
/** A turn's end shows this long (done's happy eye, failed's frown), then the pet goes back to a life of its own: an
 *  answer waiting for you is the card's and the sessions list's to say, and a pet held in one face for as long as you
 *  are away does not look alive. */
const ENDED_MS = 4000
/** The list of other sessions stays this long after the pointer leaves it. */
const LIST_LINGER_MS = 300
/** The stack of other sessions' cards: its tallest (it scrolls past this), the gap between cards, and the room it
 *  reaches past them for their shadows (as in styles.css). */
const LIST_MAX_PX = 320
const LIST_GAP_PX = 6
const LIST_PAD_PX = 8

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
  // "+N" switched on: the other sessions stay listed above the card, whichever session it shows.
  const [held, setHeld] = useState(false)

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

  const mood = useSettledMood(view.mood, Math.max(0, ...view.sessions.map((s) => s.ended ?? 0)))
  // A small tone for each session on its own: one done, one needing you, one that did not finish. (The pet's mood is
  // everyone's: one session done while another works would never change it.) Two at once ring once.
  const sounds = useRef(false)
  sounds.current = !!prefs?.sounds
  useEffect(() => {
    let last = -TONE_GAP_MS
    return on<PetMood>('tone', (tone) => {
      const now = performance.now()
      if (!sounds.current || now - last < TONE_GAP_MS) return
      last = now
      playTone(tone)
    })
  }, [])

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
    // The list of other sessions opens above the card, outside it: it takes the mouse too.
    const parts = [petRef.current, sideRef.current, ...document.querySelectorAll<HTMLElement>('.card-others')].filter((el): el is HTMLElement => !!el)
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
            card && (
              <SessionCard
                card={card}
                lines={prefs?.card === 'lines'}
                leaving={leaving === key}
                onClose={() => setClosed(key)}
                held={held}
                onHold={setHeld}
              />
            )
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

/** The pet's mood as it comes, except that thinking after work shows only once it lasts (see THINK_AFTER_MS), and a
 *  turn's end (`ended`, the latest of any session's) only for a moment (see ENDED_MS). */
function useSettledMood(mood: PetMood, ended: number): PetMood {
  const [shown, setShown] = useState(mood)
  useEffect(() => {
    if (mood === 'thinking' && shown === 'working') {
      const t = setTimeout(() => setShown(mood), THINK_AFTER_MS)
      return () => clearTimeout(t)
    }
    setShown(mood)
    if (mood === 'done' || mood === 'failed') {
      const t = setTimeout(() => setShown('idle'), ENDED_MS)
      return () => clearTimeout(t)
    }
  }, [mood, ended])
  return shown
}

/** One card per session, mood and turn: a card hidden in one turn comes back in the next. Thinking and working are one
 *  here, so a card hidden while Claude is at it stays hidden from one step to the next. */
function cardKey(card: Card) {
  const mood = card.mood === 'thinking' ? 'working' : card.mood
  return `${card.session}:${mood}:${card.at}`
}

/** Which session, and what it is doing now (or the answer's first lines once done). A click opens the sessions. With
 *  other sessions at work, "+N" lists them while hovered, or until it is clicked off once clicked on; a click on one
 *  switches the card to it. */
function SessionCard({ card, lines, leaving, onClose, held, onHold }: {
  card: Card
  lines: boolean
  leaving: boolean
  onClose: () => void
  held: boolean
  onHold: (held: boolean) => void
}) {
  const [hovered, setList] = useState(false)
  const list = (held || hovered) && card.others.length > 0
  const [room, setRoom] = useState<{ below: boolean; max: number } | null>(null)
  const cardRef = useRef<HTMLDivElement>(null)
  const listRef = useRef<HTMLDivElement>(null)
  const shut = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  // The list stacks on the card, edge to edge: above it where the window has the room, else below if there is more
  // room there, never past the window's edge (it scrolls instead). Measured before it is drawn, again as it grows.
  useLayoutEffect(() => {
    const box = cardRef.current?.getBoundingClientRect()
    const el = listRef.current
    if (!list || !box || !el) return setRoom(null)
    const need = Math.min(el.scrollHeight, LIST_MAX_PX)
    const above = box.top - LIST_GAP_PX + LIST_PAD_PX
    const below = window.innerHeight - box.bottom - LIST_GAP_PX + LIST_PAD_PX
    const under = above < need && below > above
    const max = Math.max(0, Math.min(LIST_MAX_PX, under ? below : above))
    setRoom((r) => (r?.below === under && r.max === max ? r : { below: under, max }))
  })
  const open = () => {
    clearTimeout(shut.current)
    setList(true)
  }
  const close = () => {
    clearTimeout(shut.current)
    shut.current = setTimeout(() => setList(false), LIST_LINGER_MS)
  }
  const hold = () => {
    clearTimeout(shut.current)
    setList(false)
    onHold(!held)
  }
  const follow = (session: string) => {
    void core.follow(session)
    setList(false)
  }
  return (
    <div
      ref={cardRef}
      className={`card${lines ? ' is-lines' : ''}${leaving ? ' is-leaving' : ''}`}
      role="status"
      aria-live="polite"
      onPointerEnter={() => clearTimeout(shut.current)}
      onPointerLeave={close}
    >
      <button type="button" className="card-body" onClick={() => void core.openSessions(card.session)} title="Your sessions">
        <span className="card-title">{card.title}</span>
        <Steps line={card.line} progress={card.progress} lines={card.lines} few={lines} answer={card.mood === 'done'} />
      </button>
      {card.others.length > 0 && (
        <button
          type="button"
          className={`card-more${held ? ' is-on' : ''}`}
          aria-pressed={held}
          aria-expanded={list}
          aria-label={`${card.others.length} more at work: keep them shown`}
          title={held ? 'Show the others only on hover' : 'Keep the others shown'}
          onPointerEnter={open}
          onClick={hold}
        >
          <span>+{card.others.length}</span>
        </button>
      )}
      <span className="card-actions">
        <button type="button" aria-label="Your sessions" title="Your sessions" onClick={() => void core.openSessions()}>
          <List size={16} />
        </button>
      </span>
      <button type="button" className="card-close" aria-label="Hide this" title="Hide this" onClick={onClose}>
        <X size={12} strokeWidth={3} />
      </button>
      {list && (
        <div
          ref={listRef}
          className={`card-others${room?.below ? ' is-below' : ''}`}
          style={room ? { maxHeight: room.max } : { visibility: 'hidden' }}
          role="menu"
          aria-label="Sessions at work"
        >
          {card.others.map((o) => (
            <button key={o.session} type="button" role="menuitem" className="card-other" title={o.name} onClick={() => follow(o.session)}>
              <span className="card-other-name">{o.name}</span>
              <Steps line={o.line} progress={o.progress} lines={o.lines} few={lines} needs={o.mood === 'needs'} />
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

/** A session's line, marked running, done or failed when it is a step (a shield when it waits on you). With "a few
 *  lines", steps read up to the one under way, with what Claude said between them; an answer reads down from its first
 *  line. */
function Steps({ line, progress, lines, few, answer = false, needs = false }: {
  line: string
  progress: Progress | null
  lines: Line[]
  few: boolean
  answer?: boolean
  needs?: boolean
}) {
  const last = lines[lines.length - 1]
  const rest = answer ? lines.filter((l) => l.text !== line) : last?.text === line ? lines.slice(0, -1) : lines
  const more = (few ? (answer ? rest.slice(0, 2) : rest.slice(-2)) : []).map((l, i) => (
    <span key={i} className="card-line is-past">
      {l.progress ? <StepMark progress={l.progress} /> : !answer && <SaidMark />}
      <span className="card-text">{l.text}</span>
    </span>
  ))
  return (
    <>
      {!answer && more}
      <span className="card-line">
        {needs ? <ShieldAlert className="card-other-needs" /> : progress && <StepMark progress={progress} />}
        <span className="card-text">{line}</span>
      </span>
      {answer && more}
    </>
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
      {approval.cwd && <div className="approval-where">{approval.cwd}</div>}
      <div className="approval-what">{what}</div>
      {approval.detail && approval.kind !== 'question' && <pre className="approval-detail">{approval.detail}</pre>}
      {!approval.whole && <div className="approval-note">Too long for the card: answer it in Claude Code.</div>}
      <div className="approval-actions">
        {approval.kind === 'question' && approval.question ? (
          approval.question.options.map((o, i) => (
            <button key={o} type="button" className={`btn is-small${i === 0 ? ' is-primary' : ''}`} onClick={() => choose('answer', o)}>
              {o}
            </button>
          ))
        ) : approval.kind === 'question' ? null : (
          <>
            {approval.whole && (
              <button type="button" className="btn is-small is-primary" onClick={() => choose('allow')}>
                {approval.kind === 'plan' ? 'Approve plan' : 'Allow'}
              </button>
            )}
            {approval.whole && approval.always && (
              <button type="button" className="btn is-small" onClick={() => choose('always')}>
                {approval.always.label}
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
      {approval.whole && approval.always && (
        <div className="approval-rule">
          “{approval.always.label}” allows <code>{approval.always.what}</code>, {approval.always.scope}.
        </div>
      )}
    </div>
  )
}
