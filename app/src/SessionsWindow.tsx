// The sessions list: every Claude Code session Kuchu follows, the latest first: what it is called, what it is doing
// now (or how it ended), and its folder. Opened by a click on the pet, the tray, or Ctrl+Alt+K; it hides when it
// loses the focus.

import { getCurrentWindow } from '@tauri-apps/api/window'
import { Settings, X } from 'lucide-react'
import { useEffect, useState } from 'react'
import { ago, core, MOOD_LABEL, on, type HookReport, type View } from './api'
import { Pet } from './pet/Pet'

export function SessionsWindow() {
  const [view, setView] = useState<View | null>(null)
  const [hooks, setHooks] = useState<HookReport | null>(null)
  const [, tick] = useState(0)

  useEffect(() => {
    void core.view().then(setView)
    void core.hooksStatus().then(setHooks)
    const offs = [on<View>('view', setView), on<HookReport>('hooks', setHooks)]
    // Times stay current ("2 min") while the list is open; Esc hides it.
    const timer = setInterval(() => tick((n) => n + 1), 30_000)
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && void getCurrentWindow().hide()
    const onFocus = () => void core.hooksStatus().then(setHooks)
    window.addEventListener('keydown', onKey)
    window.addEventListener('focus', onFocus)
    return () => {
      offs.forEach((off) => off())
      clearInterval(timer)
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('focus', onFocus)
    }
  }, [])

  const sessions = view?.sessions ?? []
  const connected = hooks?.state === 'installed' && !hooks.disabled

  return (
    <main className="sheet">
      <header className="sheet-head">
        <span className="brand">
          <Pet size={22} mood={view?.mood ?? 'idle'} />
          Kuchu
        </span>
        <span className="grow" />
        <button type="button" className="icon-btn" title="Settings" aria-label="Settings" onClick={() => void core.openSettings()}>
          <Settings />
        </button>
        <button type="button" className="icon-btn" title="Close (Esc)" aria-label="Close" onClick={() => void getCurrentWindow().hide()}>
          <X />
        </button>
      </header>

      {hooks && !connected && (
        <button type="button" className="notice" onClick={() => void core.openSettings()}>
          <strong>{hooks.disabled ? 'Claude Code runs no hooks right now' : 'Kuchu does not hear Claude Code yet'}</strong>
          <span>
            {hooks.disabled
              ? 'disableAllHooks is on in your settings, so nothing reaches Kuchu.'
              : 'Connect it in Settings. You see the change before anything is written.'}
          </span>
        </button>
      )}

      {sessions.length === 0 ? (
        <div className="empty">
          <Pet size={56} mood="idle" />
          <p className="title">No Claude Code sessions right now</p>
          <p className="muted">Start one in a terminal or your editor. It shows up here, and beside Kuchu.</p>
        </div>
      ) : (
        <ul className="list">
          {sessions.map((s) => (
            <li key={s.id} className="row">
              <div className="row-head">
                <span className="row-title" title={s.prompt || s.name}>
                  {s.name}
                </span>
                <span className={`state${s.mood === 'needs' ? ' is-needs' : ''}`}>{MOOD_LABEL[s.mood]}</span>
                <button type="button" className="forget" title="Forget this session" aria-label={`Forget ${s.name}`} onClick={() => void core.forget(s.id)}>
                  <X />
                </button>
              </div>
              {s.line && <div className="row-line">{s.line}</div>}
              <div className="row-meta" title={s.cwd}>
                {s.project} · {ago(s.updated)}
              </div>
            </li>
          ))}
        </ul>
      )}
      <footer className="sheet-foot">
        <kbd>Ctrl</kbd> <kbd>Alt</kbd> <kbd>K</kbd> opens this from anywhere
      </footer>
    </main>
  )
}
