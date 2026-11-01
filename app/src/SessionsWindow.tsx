// The sessions list: every Claude Code session Kuchu follows, the latest first: its project, what it is doing now
// (or how it ended), and the message that started it. Opened by a click on the pet, the tray, or Ctrl+Alt+K; it hides
// when it loses the focus.

import { getCurrentWindow } from '@tauri-apps/api/window'
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
    <main className="sheet sessions">
      <header className="sheet-head">
        <span className="brand">
          <Pet size={22} mood={view?.mood ?? 'idle'} />
          Kuchu
        </span>
        <span className="grow" />
        <button type="button" className="icon-btn" title="Settings" aria-label="Settings" onClick={() => void core.openSettings()}>
          ⚙
        </button>
        <button type="button" className="icon-btn" title="Close (Esc)" aria-label="Close" onClick={() => void getCurrentWindow().hide()}>
          ×
        </button>
      </header>

      {hooks && !connected && (
        <button type="button" className="notice" onClick={() => void core.openSettings()}>
          <strong>{hooks.disabled ? 'Claude Code’s hooks are switched off' : 'Kuchu does not hear Claude Code yet'}</strong>
          <span>{hooks.disabled ? 'disableAllHooks is on in your settings, so nothing reaches Kuchu.' : 'Connect it in Settings: you see the change before anything is written.'}</span>
        </button>
      )}

      {sessions.length === 0 ? (
        <div className="empty">
          <Pet size={56} mood="idle" />
          <p>No Claude Code sessions right now.</p>
          <p className="muted">Start one in a terminal or your editor: it shows up here, and beside Kuchu.</p>
        </div>
      ) : (
        <ul className="list">
          {sessions.map((s) => (
            <li key={s.id} className={`row is-${s.mood}`}>
              <div className="row-head">
                <span className="dot" aria-hidden="true" />
                <span className="row-title" title={s.cwd}>
                  {s.project}
                </span>
                <span className="chip">{MOOD_LABEL[s.mood]}</span>
                <span className="muted">{ago(s.updated)}</span>
                <button type="button" className="forget" title="Forget this session" aria-label={`Forget ${s.project}`} onClick={() => void core.forget(s.id)}>
                  ×
                </button>
              </div>
              {s.line && <div className="row-line">{s.line}</div>}
              {s.prompt && <div className="row-prompt muted">“{s.prompt}”</div>}
            </li>
          ))}
        </ul>
      )}
      <footer className="sheet-foot muted">Ctrl+Alt+K opens this from anywhere</footer>
    </main>
  )
}
