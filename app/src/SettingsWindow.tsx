// Kuchu's Settings: the pet (which one, size, see-through, sounds, starting at sign-in), what shows beside it and what a click
// opens, connecting to Claude Code (the change to settings.json shown before it is made, a copy kept), and permission
// requests. Each change reaches the pet at once.

import { BookOpen, Bot, CircleAlert, CircleCheck, ExternalLink, Globe, MessageSquareText, Plug, RefreshCw, ShieldCheck, Sparkles, TriangleAlert, Volume2 } from 'lucide-react'
import { useEffect, useState } from 'react'
import { core, on, SIZES, type HookPlan, type HookReport, type Prefs, type SitePage, type Start, type Update } from './api'
import { Pet, type PetKind } from './pet/Pet'
import robot3d from './pet/robot3d.webp'
import type { PetMood } from './pet/petMood'
import { playTone, TONE_MOODS } from './pet/petSound'
import { StepMark } from './StepMark'

type Tab = 'pet' | 'beside' | 'claude' | 'requests'

/** The pets in the gallery: each one's name, and what sets it apart. */
const PETS: readonly { pet: PetKind; name: string; about: string }[] = [
  { pet: 'robot', name: 'Robot', about: 'One lit eye, and a pointer for the other.' },
  { pet: 'cat', name: 'Cat', about: 'A ginger tabby with a bell.' },
  { pet: 'dog', name: 'Dog', about: 'A brown pup with a patch over one eye.' },
  { pet: 'owl', name: 'Owl', about: 'Huge amber eyes that miss nothing.' },
  { pet: 'frog', name: 'Frog', about: 'Eyes on top and a very wide smile.' },
  { pet: 'duck', name: 'Duck', about: 'You explain your bugs to it.' },
]

/** Pets on their way: in the gallery, with no way to choose them yet. To open one, give it a `pet` and move it up to PETS. */
const COMING: readonly { name: string; about: string; picture: string }[] = [{ name: '3D robot', about: 'A real 3D pet on two wheels.', picture: robot3d }]

const TABS: { id: Tab; label: string; icon: React.ReactNode }[] = [
  { id: 'pet', label: 'Pet', icon: <Bot /> },
  { id: 'beside', label: 'Beside the pet', icon: <MessageSquareText /> },
  { id: 'claude', label: 'Claude Code', icon: <Plug /> },
  { id: 'requests', label: 'Permission requests', icon: <ShieldCheck /> },
]

/** Where to read more: Kuchu's own website, which is always the place to look. */
const LINKS: { page: SitePage; label: string; icon: React.ReactNode }[] = [
  { page: 'docs', label: 'Help and docs', icon: <BookOpen /> },
  { page: 'changelog', label: 'What’s new', icon: <Sparkles /> },
  { page: 'home', label: 'Kuchu website', icon: <Globe /> },
]

export function SettingsWindow() {
  const [prefs, setPrefs] = useState<Prefs | null>(null)
  const [hooks, setHooks] = useState<HookReport | null>(null)
  const [tab, setTab] = useState<Tab>('pet')
  const [version, setVersion] = useState('')
  useEffect(() => {
    void core.prefs().then(setPrefs)
    void core.version().then(setVersion)
    void core.hooksStatus().then((h) => {
      setHooks(h)
      if (h.state !== 'installed') setTab('claude')
    })
    const offs = [on<Prefs>('prefs', setPrefs), on<HookReport>('hooks', setHooks)]
    return () => offs.forEach((off) => off())
  }, [])
  if (!prefs) return null
  const set = (patch: Partial<Prefs>) => void core.setPrefs(patch).then(setPrefs)
  const current = TABS.find((t) => t.id === tab)!

  return (
    <div className="settings">
      <div className="settings-nav">
        <span className="brand">
          <Pet size={24} mood="idle" />
          Kuchu
          <span className="brand-version">{version}</span>
        </span>
        <nav className="settings-tabs" role="tablist" aria-orientation="vertical">
          {TABS.map((t) => (
            <button key={t.id} type="button" role="tab" aria-selected={t.id === tab} className="tab" onClick={() => setTab(t.id)}>
              {t.icon}
              {t.label}
              {t.id === 'claude' && hooks && hooks.state !== 'installed' && <span className="state-dot" aria-label="Not connected" />}
            </button>
          ))}
        </nav>
        <div className="settings-links">
          {LINKS.map((l) => (
            <button key={l.page} type="button" className="tab is-link" onClick={() => void core.openPage(l.page)}>
              {l.icon}
              {l.label}
              <ExternalLink className="tab-out" />
            </button>
          ))}
          <UpdateRow />
        </div>
      </div>

      <section className="settings-pane" role="tabpanel" aria-label={current.label}>
        <h2>{current.label}</h2>
        <div className="settings-rows">
          {tab === 'pet' && (
            <>
              <Row label="Your pet" desc="Who sits on your desk. Each does all the same things, with its own face." stacked>
                <div className="choices" role="radiogroup" aria-label="Your pet">
                  {PETS.map(({ pet, name, about }) => (
                    <button key={pet} type="button" role="radio" aria-checked={prefs.pet === pet} className="choice" onClick={() => set({ pet })}>
                      <span className="choice-picture is-size">
                        <Pet kind={pet} size={64} mood="idle" />
                      </span>
                      <span className="choice-name">{name}</span>
                      <span className="muted">{about}</span>
                    </button>
                  ))}
                  {COMING.map(({ name, about, picture }) => (
                    <button key={name} type="button" role="radio" aria-checked="false" aria-disabled="true" disabled className="choice is-coming">
                      <span className="choice-picture is-size">
                        <img src={picture} alt="" />
                      </span>
                      <span className="choice-name">{name}</span>
                      <span className="muted">{about}</span>
                      <span className="choice-soon">Coming soon</span>
                    </button>
                  ))}
                </div>
              </Row>
              <Row label="Size" desc="How big Kuchu is on your screen." stacked>
                <div className="choices" role="radiogroup" aria-label="Size">
                  {(['small', 'base', 'big'] as const).map((size) => (
                    <button key={size} type="button" role="radio" aria-checked={prefs.size === size} className="choice" onClick={() => set({ size })}>
                      <span className="choice-picture is-size">
                        <Pet size={Math.round(SIZES[size] * 0.62)} mood="idle" />
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
              <Row
                label="Plays when there is no work"
                desc="Now and then Kuchu stretches, yawns, looks around, taps a foot, flips its face, whistles, pops its head up or takes a short walk. After a few quiet minutes it naps; your pointer or a session wakes it."
              >
                <Toggle label="Plays when there is no work" on={prefs.lively} set={(lively) => set({ lively })} />
              </Row>
              <Row label="Sounds" desc="A short tone when a session needs you, and when one is done or did not finish.">
                <div className="inline">
                  <button type="button" className="btn is-small" onClick={() => TONE_MOODS.forEach((m, i) => setTimeout(() => playTone(m), i * 750))}>
                    <Volume2 />
                    Hear them
                  </button>
                  <Toggle label="Sounds" on={prefs.sounds} set={(sounds) => set({ sounds })} />
                </div>
              </Row>
              <StartAtSignIn />
            </>
          )}

          {tab === 'beside' && (
            <>
              <Row
                label="What shows beside the pet"
                desc="While a session works: what it is called and what it is doing now. Once done, the answer's first line. A click on it opens your sessions."
                stacked
              >
                <div className="choices" role="radiogroup" aria-label="What shows beside the pet">
                  {(
                    [
                      ['line', 'One line', 'Its name and its step.'],
                      ['lines', 'A few lines', 'Its last steps, or the answer’s first lines.'],
                      ['off', 'Nothing', 'Kuchu alone: its face says it all.'],
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
              <Row
                label="Keep an answer until you click Kuchu"
                desc="Otherwise an answer shows for a few seconds, or as long as your pointer is on it, then fades. Kuchu keeps its happy face until you click it."
              >
                <Toggle label="Keep an answer until you click Kuchu" on={prefs.keepAnswer} set={(keepAnswer) => set({ keepAnswer })} />
              </Row>
              <Row
                label="Keep other sessions shown"
                desc="With several sessions at work, each stays on a card of its own beside this one. Otherwise they show while your pointer is on the +N badge. A click on the badge switches this too."
              >
                <Toggle label="Keep other sessions shown" on={prefs.keepOthers} set={(keepOthers) => set({ keepOthers })} />
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
            </>
          )}

          {tab === 'claude' && <Connect report={hooks} onReport={setHooks} />}

          {tab === 'requests' && (
            <Row
              label="Answer them on Kuchu"
              desc="When Claude Code asks to run a command or change a file, Kuchu shows it with Allow and Deny. Nothing is allowed without your click. If you don't answer, Claude Code asks in its own window after about 100 seconds; “In Claude Code” hands it back at once."
            >
              <Toggle label="Answer them on Kuchu" on={prefs.approvals} set={(approvals) => set({ approvals })} />
            </Row>
          )}
        </div>
      </section>
    </div>
  )
}

/** At the foot of the list: whether a newer Kuchu is out. A copy from the Store is told by Windows, each time Settings opens; any other
 *  copy cannot know without going online, which Kuchu never does, so its button opens the website. */
function UpdateRow() {
  const [update, setUpdate] = useState<Update | null>(null)
  const [installing, setInstalling] = useState(false)
  const check = () => {
    setUpdate(null)
    void core.updateStatus().then(setUpdate)
  }
  useEffect(check, [])
  const install = () => {
    setInstalling(true)
    // Windows closes Kuchu to finish; if this comes back, the Store's own page has taken over.
    void core.updateInstall().finally(() => setInstalling(false))
  }
  if (!update) {
    return (
      <div className="update-row">
        <RefreshCw />
        Checking for updates…
      </div>
    )
  }
  if (update.state === 'site') {
    return (
      <button type="button" className="tab is-link" onClick={() => void core.openPage('changelog')}>
        <RefreshCw />
        Check for updates
        <ExternalLink className="tab-out" />
      </button>
    )
  }
  if (update.state === 'current') {
    return (
      <div className="update-row">
        <RefreshCw />
        Up to date
        <button type="button" className="text-link" onClick={check}>
          Check again
        </button>
      </div>
    )
  }
  if (update.state === 'unknown') {
    return (
      <div className="update-row">
        <RefreshCw />
        Could not check
        <button type="button" className="text-link" onClick={() => void core.openStore()}>
          Open the Store
        </button>
      </div>
    )
  }
  return (
    <div className="update-row is-ready">
      <span className="update-note">
        <RefreshCw />
        A new version is ready
      </span>
      <button type="button" className="btn is-small is-primary" disabled={installing} onClick={install}>
        {installing ? 'Updating…' : 'Update now'}
      </button>
      {installing && <span>Kuchu closes for a moment, then starts again.</span>}
    </div>
  )
}

/** Connecting Kuchu to Claude Code: the hooks' state, and the change to settings.json shown before it is made. */
function Connect({ report, onReport }: { report: HookReport | null; onReport: (r: HookReport) => void }) {
  const [plan, setPlan] = useState<HookPlan | null>(null)
  const [done, setDone] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const look = (install: boolean) => {
    setError(null)
    setDone(null)
    core.hooksPlan(install).then(setPlan, (e) => setError(String(e)))
  }
  const confirm = () => {
    if (!plan) return
    core.hooksApply(plan.install, plan.fingerprint).then(
      ({ backup, report }) => {
        onReport(report)
        setPlan(null)
        setDone(
          (plan.install ? 'Connected. New Claude Code sessions show up beside Kuchu at once.' : 'Disconnected. Kuchu’s hooks are gone from settings.json.') +
            (backup ? ` Your settings as they were are kept at ${backup}.` : ''),
        )
      },
      (e) => setError(String(e)),
    )
  }

  const state = report?.state
  const Icon = state === 'installed' ? CircleCheck : state === 'unreadable' ? CircleAlert : TriangleAlert
  return (
    <>
      <div className={`status is-${state ?? 'missing'}`}>
        <Icon />
        <div className="status-text">
          <strong>
            {state === 'installed'
              ? 'Connected'
              : state === 'outdated'
                ? 'Connected, but out of date'
                : state === 'unreadable'
                  ? 'Claude Code’s settings could not be read'
                  : 'Not connected yet'}
          </strong>
          <span>
            {state === 'unreadable'
              ? report?.why
              : state === 'installed'
                ? 'Every Claude Code session, in any terminal or editor, shows up beside Kuchu.'
                : 'Kuchu listens through hooks in Claude Code’s settings. You see the change before anything is written, and a copy of the file is kept first.'}
          </span>
          {report && <span className="muted">{report.path}</span>}
          {(state === 'installed' || state === 'outdated') && <span className="muted">Before you uninstall Kuchu, disconnect it here, so Claude Code stops running its hooks.</span>}
          {report?.disabled && <span className="note-warn">disableAllHooks is on in this file: Claude Code runs no hooks at all, Kuchu’s included.</span>}
        </div>
        {!plan && state !== 'unreadable' && (
          <div className="inline">
            {state !== 'missing' && (
              <button type="button" className="btn is-small" onClick={() => look(false)}>
                Disconnect
              </button>
            )}
            {state !== 'installed' && (
              <button type="button" className="btn is-small is-primary" onClick={() => look(true)}>
                {state === 'outdated' ? 'Update' : 'Connect'}
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
                {plan.install ? 'These lines will be added to' : 'These lines will be taken out of'} <code>{plan.path}</code>. Nothing else of yours changes.
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
              <p className="muted">
                First, a copy of the file as it is now is kept at {plan.backup}. Kuchu keeps the last {plan.keptCopies} such copies.
              </p>
            </>
          )}
          <div className="inline">
            <button type="button" className="btn is-small" onClick={() => setPlan(null)}>
              Cancel
            </button>
            {!plan.unchanged && (
              <button type="button" className="btn is-small is-primary" onClick={confirm}>
                {plan.install ? 'Connect' : 'Disconnect'}
              </button>
            )}
          </div>
        </div>
      )}
      {done && <p className="note-ok">{done}</p>}
      {error && <p className="note-warn">{error}</p>}
      <p className="muted connect-help">
        Something not working?{' '}
        <button type="button" className="text-link" onClick={() => void core.openPage('troubleshooting')}>
          See what to check
        </button>
      </p>
    </>
  )
}

/** A session's steps as the card tells them, round and round, for the pictures of each choice. */
const STEPS: { mood: PetMood; line: string; ms: number }[] = [
  { mood: 'thinking', line: 'Thinking…', ms: 1600 },
  { mood: 'working', line: 'Reading main.rs', ms: 1500 },
  { mood: 'working', line: 'Run the tests', ms: 1700 },
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
    <span className="choice-picture">
      {mode !== 'off' && (
        <span className={`mini-card${mode === 'lines' ? ' is-lines' : ''}`}>
          <span className="mini-title">Fix the login bug</span>
          {past.map((s) => (
            <span key={s.line} className="mini-line is-past">
              <StepMark progress="done" small />
              <span className="mini-text">{s.line}</span>
            </span>
          ))}
          <span key={step.line} className="mini-line">
            {step.mood === 'working' && <StepMark progress="running" small />}
            <span className="mini-text">{step.line}</span>
          </span>
        </span>
      )}
      <span className="mini-pet">
        <Pet size={46} mood={step.mood} />
      </span>
    </span>
  )
}

function Row({ label, desc, stacked, children }: { label: string; desc: string; stacked?: boolean; children: React.ReactNode }) {
  return (
    <div className={`row-setting${stacked ? ' is-stacked' : ''}`}>
      <div>
        <div className="row-label">{label}</div>
        <p className="row-desc">{desc}</p>
      </div>
      {children}
    </div>
  )
}

/** Starting at sign-in: your account's own startup entry, never a service. Read from the system each time, since it
 *  can also be switched off there. */
function StartAtSignIn() {
  const [start, setStart] = useState<Start | null>(null)
  const [why, setWhy] = useState<string | null>(null)
  useEffect(() => void core.startAtSignIn().then(setStart), [])
  if (!start) return null
  if (start.byWindows) {
    return (
      <Row
        label="Starts when you sign in"
        desc="Windows starts Kuchu when you sign in. To switch that off, open Windows’ Settings, then Apps, then Startup."
      >
        <span className="muted">In Windows’ Settings</span>
      </Row>
    )
  }
  const change = (on: boolean) => {
    setWhy(null)
    core.setStartAtSignIn(on).then(setStart, (e: unknown) => setWhy(String(e)))
  }
  const on = start.on && !start.paused && !start.elsewhere
  const desc = start.paused
    ? 'It was switched off in Windows’ startup apps. Switch it on here to start Kuchu when you sign in again.'
    : start.elsewhere
      ? `It starts another copy of Kuchu: ${start.elsewhere}. Switch it on here to start this one instead.`
      : 'Kuchu opens by itself when you sign in, from your account’s own list of startup apps. It is never a service.'
  return (
    <Row label="Starts when you sign in" desc={why ?? desc}>
      <Toggle label="Starts when you sign in" on={on} set={change} />
    </Row>
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
