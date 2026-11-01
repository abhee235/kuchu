// Kuchu's Settings: connecting it to Claude Code (the change to settings.json shown before it is made, a copy kept),
// the pet's size, see-through and sounds, what shows beside it, what a click opens, and permission requests. Each
// change reaches the pet at once.

import { useEffect, useState } from 'react'
import { core, on, SIZES, type HookPlan, type HookReport, type Prefs } from './api'
import { Pet } from './pet/Pet'
import type { PetMood } from './pet/petMood'
import { playTone, TONE_MOODS } from './pet/petSound'

export function SettingsWindow() {
  const [prefs, setPrefs] = useState<Prefs | null>(null)
  useEffect(() => {
    void core.prefs().then(setPrefs)
    return on<Prefs>('prefs', setPrefs)
  }, [])
  if (!prefs) return null
  const set = (patch: Partial<Prefs>) => void core.setPrefs(patch).then(setPrefs)

  return (
    <main className="settings">
      <header className="settings-head">
        <Pet size={56} mood="idle" />
        <div>
          <h1>Kuchu</h1>
          <p className="muted">A desk pet for Claude Code: it shows what your sessions are doing, and asks you when one needs you.</p>
        </div>
      </header>

      <Connect />

      <section>
        <h2>The pet</h2>
        <Row label="Size" desc="How big Kuchu is on the screen." stacked>
          <div className="choices is-three" role="radiogroup" aria-label="Size">
            {(['small', 'base', 'big'] as const).map((size) => (
              <button key={size} type="button" role="radio" aria-checked={prefs.size === size} className="choice" onClick={() => set({ size })}>
                <span className="choice-picture is-size">
                  <Pet size={Math.round(SIZES[size] * 0.6)} mood="idle" />
                </span>
                <span className="choice-name">{size === 'small' ? 'Small' : size === 'base' ? 'Base' : 'Big'}</span>
                <span className="muted">{SIZES[size]} px</span>
              </button>
            ))}
          </div>
        </Row>
        <Row label="See-through under the pointer" desc="Kuchu fades while your pointer is on it, so what is behind it shows. It still takes clicks and drags.">
          <Toggle label="See-through under the pointer" on={prefs.seeThrough} set={(seeThrough) => set({ seeThrough })} />
        </Row>
        <Row label="Sounds" desc="A short tone when a session needs you, and when one is done or did not finish.">
          <div className="inline">
            <button type="button" className="btn" onClick={() => TONE_MOODS.forEach((m, i) => setTimeout(() => playTone(m), i * 750))}>
              ♪ Hear them
            </button>
            <Toggle label="Sounds" on={prefs.sounds} set={(sounds) => set({ sounds })} />
          </div>
        </Row>
      </section>

      <section>
        <h2>What Claude Code is doing, beside the pet</h2>
        <Row label="Beside the pet" desc="While a session works: its project and what it is doing now. Once done, the answer's first line. A click on it opens your sessions." stacked>
          <div className="choices is-three" role="radiogroup" aria-label="Beside the pet">
            {(
              [
                ['line', 'One line', 'The project and its step.'],
                ['lines', 'A few lines', 'Its last steps, or the answer’s first lines.'],
                ['off', 'Nothing', 'The pet alone: its face says it all.'],
              ] as const
            ).map(([card, name, about]) => (
              <button key={card} type="button" role="radio" aria-checked={prefs.card === card} className="choice" onClick={() => set({ card })}>
                <CardPreview mode={card} />
                <span className="choice-name">{name}</span>
                <span className="muted">{about}</span>
              </button>
            ))}
          </div>
        </Row>
        <Row label="Clicking Kuchu opens" desc="Ctrl+Alt+K opens your sessions from anywhere, and the tray icon always brings Kuchu back.">
          <Segmented
            value={prefs.click}
            options={[
              ['sessions', 'Your sessions'],
              ['settings', 'Settings'],
            ]}
            set={(click) => set({ click })}
          />
        </Row>
      </section>

      <section>
        <h2>Permission requests</h2>
        <Row
          label="Answer them on Kuchu"
          desc="When Claude Code asks to run a command or change a file, Kuchu shows it with Allow and Deny. Nothing is allowed without your click; if you don't answer, Claude Code asks in its own window after about 100 seconds, and “In Claude Code” hands it back at once."
        >
          <Toggle label="Answer them on Kuchu" on={prefs.approvals} set={(approvals) => set({ approvals })} />
        </Row>
      </section>
    </main>
  )
}

/** Connecting Kuchu to Claude Code: the hooks' state, and the change to settings.json shown before it is made. */
function Connect() {
  const [report, setReport] = useState<HookReport | null>(null)
  const [plan, setPlan] = useState<HookPlan | null>(null)
  const [done, setDone] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    void core.hooksStatus().then(setReport)
    return on<HookReport>('hooks', setReport)
  }, [])

  const look = (install: boolean) => {
    setError(null)
    setDone(null)
    core.hooksPlan(install).then(setPlan, (e) => setError(String(e)))
  }
  const confirm = () => {
    if (!plan) return
    core.hooksApply(plan.install, plan.fingerprint).then(
      ({ backup, report }) => {
        setReport(report)
        setPlan(null)
        setDone(
          (plan.install ? 'Connected: Kuchu now hears Claude Code. New sessions show up at once.' : 'Disconnected: Kuchu’s hooks are gone from settings.json.') +
            (backup ? ` Your settings as they were are kept at ${backup}.` : ''),
        )
      },
      (e) => setError(String(e)),
    )
  }

  const state = report?.state
  return (
    <section className="connect">
      <h2>Claude Code</h2>
      <div className={`status is-${state ?? 'missing'}`}>
        <span className="dot" aria-hidden="true" />
        <div className="grow">
          <strong>
            {state === 'installed'
              ? 'Connected'
              : state === 'outdated'
                ? 'Connected, but out of date'
                : state === 'unreadable'
                  ? 'settings.json could not be read'
                  : 'Not connected yet'}
          </strong>
          <span className="muted">
            {state === 'unreadable'
              ? report?.why
              : state === 'installed'
                ? `Kuchu’s hooks are in ${report?.path}.`
                : `Kuchu listens through hooks in ${report?.path ?? 'settings.json'}. You see the change before anything is written, and a copy of the file is kept first.`}
          </span>
          {report?.disabled && <span className="warn">disableAllHooks is on in this file: Claude Code runs no hooks at all, Kuchu’s included.</span>}
        </div>
        {!plan && state !== 'unreadable' && (
          <div className="inline">
            {state !== 'installed' && (
              <button type="button" className="btn is-primary" onClick={() => look(true)}>
                {state === 'outdated' ? 'Update' : 'Connect'}
              </button>
            )}
            {state !== 'missing' && (
              <button type="button" className="btn" onClick={() => look(false)}>
                Disconnect
              </button>
            )}
          </div>
        )}
      </div>

      {plan && (
        <div className="plan">
          {plan.unchanged ? (
            <p>Nothing to change.</p>
          ) : (
            <>
              <p>
                {plan.install ? 'These lines will be added to' : 'These lines will be taken out of'} <code>{plan.path}</code>. Nothing else of yours
                changes.
                {plan.respaced && ' The file is spaced differently from how Claude Code writes it, so it will be written its way.'}
              </p>
              <pre className="diff">
                {plan.diff.map((l, i) => (
                  <span key={i} className={`diff-line is-${l.kind === '+' ? 'add' : l.kind === '-' ? 'del' : l.kind === '…' ? 'gap' : 'same'}`}>
                    {l.kind === '…' ? '  ⋯' : `${l.kind} ${l.text}`}
                    {'\n'}
                  </span>
                ))}
              </pre>
              <p className="muted">First, a copy of the file as it is now is kept at {plan.backup}.</p>
            </>
          )}
          <div className="inline">
            {!plan.unchanged && (
              <button type="button" className="btn is-primary" onClick={confirm}>
                {plan.install ? 'Connect' : 'Disconnect'}
              </button>
            )}
            <button type="button" className="btn" onClick={() => setPlan(null)}>
              Cancel
            </button>
          </div>
        </div>
      )}
      {done && <p className="ok">{done}</p>}
      {error && <p className="warn">{error}</p>}
    </section>
  )
}

/** A session's steps as the card tells them, round and round, for the pictures of each choice. */
const STEPS: { mood: PetMood; line: string; ms: number }[] = [
  { mood: 'thinking', line: 'Thinking…', ms: 1600 },
  { mood: 'working', line: 'Reading main.rs', ms: 1500 },
  { mood: 'working', line: 'Running: the tests', ms: 1700 },
  { mood: 'done', line: 'All 23 tests pass.', ms: 2600 },
]

function useStep() {
  const [i, setI] = useState(0)
  useEffect(() => {
    const t = setTimeout(() => setI((n) => (n + 1) % STEPS.length), STEPS[i]!.ms)
    return () => clearTimeout(t)
  }, [i])
  return i
}

function CardPreview({ mode }: { mode: 'line' | 'lines' | 'off' }) {
  const i = useStep()
  const step = STEPS[i]!
  const past = mode === 'lines' ? STEPS.slice(Math.max(1, i - 1), i).filter((s) => s.mood === 'working') : []
  return (
    <span className="choice-picture is-desk">
      {mode !== 'off' && (
        <span className="mini-card">
          <span className="mini-title">my-app</span>
          {past.map((s) => (
            <span key={s.line} className="mini-line is-past">
              {s.line}
            </span>
          ))}
          <span key={step.line} className="mini-line">
            {step.line}
          </span>
        </span>
      )}
      <span className="mini-pet">
        <Pet size={44} mood={step.mood} />
      </span>
    </span>
  )
}

function Row({ label, desc, stacked, children }: { label: string; desc: string; stacked?: boolean; children: React.ReactNode }) {
  return (
    <div className={`row-setting${stacked ? ' is-stacked' : ''}`}>
      <div className="row-text">
        <div className="row-label">{label}</div>
        <div className="muted">{desc}</div>
      </div>
      <div className="row-control">{children}</div>
    </div>
  )
}

function Toggle({ label, on, set }: { label: string; on: boolean; set: (on: boolean) => void }) {
  return (
    <button type="button" role="switch" aria-checked={on} aria-label={label} className={`toggle${on ? ' is-on' : ''}`} onClick={() => set(!on)}>
      <span className="knob" />
    </button>
  )
}

function Segmented<T extends string>({ value, options, set }: { value: T; options: readonly (readonly [T, string])[]; set: (v: T) => void }) {
  return (
    <div className="segmented" role="radiogroup">
      {options.map(([v, label]) => (
        <button key={v} type="button" role="radio" aria-checked={value === v} className={value === v ? 'is-on' : ''} onClick={() => set(v)}>
          {label}
        </button>
      ))}
    </div>
  )
}
