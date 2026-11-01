// The sessions list: every Claude Code session Kuchu follows, the latest first: what it is called, what it is doing
// now (or how it ended), and its folder. Opened by a click on the pet, the tray, or Ctrl+Alt+K; it hides when it
// loses the focus. From here a conversation with Claude can start in a folder, and one started here can be answered,
// stopped, or taken on in a terminal. (Sessions started in a terminal or an editor are only followed: Claude Code
// takes their messages only there.)

import { getCurrentWindow } from '@tauri-apps/api/window'
import { ArrowLeft, ArrowUp, Folder, FolderOpen, MessageSquare, Plus, Settings, Square, SquareTerminal, Wrench, X } from 'lucide-react'
import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { ago, core, MOOD_LABEL, on, type Chat, type ChatEntry, type HookReport, type Prefs, type Session, type View } from './api'
import { Pet } from './pet/Pet'

type Screen = { kind: 'list' } | { kind: 'new' } | { kind: 'chat'; id: string }

/** How many folders a new conversation offers. */
const FOLDERS = 6

export function SessionsWindow() {
  const [view, setView] = useState<View | null>(null)
  const [hooks, setHooks] = useState<HookReport | null>(null)
  const [prefs, setPrefs] = useState<Prefs | null>(null)
  const [chats, setChats] = useState<Record<string, Chat>>({})
  const [screen, setScreen] = useState<Screen>({ kind: 'list' })
  const [, tick] = useState(0)

  useEffect(() => {
    void core.view().then(setView)
    void core.hooksStatus().then(setHooks)
    void core.prefs().then(setPrefs)
    void core.chats().then((list) => setChats(Object.fromEntries(list.map((c) => [c.id, c]))))
    // A click on a conversation's card opens it here.
    const openAsked = () => void core.takeOpen().then((id) => id && setScreen({ kind: 'chat', id }))
    openAsked()
    const offs = [
      on<View>('view', setView),
      on<HookReport>('hooks', setHooks),
      on<Prefs>('prefs', setPrefs),
      on<Chat>('chat', (c) => setChats((all) => ({ ...all, [c.id]: c }))),
      on<string>('chat-gone', (id) => {
        setChats((all) => {
          const { [id]: _, ...rest } = all
          return rest
        })
        // A conversation forgotten while open: back to the list.
        setScreen((now) => (now.kind === 'chat' && now.id === id ? { kind: 'list' } : now))
      }),
      on('open', openAsked),
    ]
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
  const chat = screen.kind === 'chat' ? chats[screen.id] : undefined
  const session = chat && sessions.find((s) => s.id === chat.id)

  const close = (
    <button type="button" className="icon-btn" title="Close (Esc)" aria-label="Close" onClick={() => void getCurrentWindow().hide()}>
      <X />
    </button>
  )
  const back = (
    <button type="button" className="icon-btn" title="Your sessions" aria-label="Back to your sessions" onClick={() => setScreen({ kind: 'list' })}>
      <ArrowLeft />
    </button>
  )

  if (screen.kind === 'new') {
    const folders = [...new Set([...(prefs?.folders ?? []), ...sessions.map((s) => s.cwd).filter(Boolean)])].slice(0, FOLDERS)
    return (
      <main className="sheet">
        <header className="sheet-head">
          {back}
          <span className="sheet-title">New conversation</span>
          <span className="grow" />
          {close}
        </header>
        <NewChat
          folders={folders}
          onStarted={(id) =>
            void core.chats().then((list) => {
              setChats(Object.fromEntries(list.map((c) => [c.id, c])))
              setScreen({ kind: 'chat', id })
            })
          }
        />
      </main>
    )
  }

  if (chat) {
    const handed = chat.status === 'handed'
    return (
      <main className="sheet">
        <header className="sheet-head">
          {back}
          <span className="sheet-title" title={chat.cwd}>
            <span className="sheet-name">{session?.name || chatName(chat)}</span>
            <span className="sheet-sub">{chat.project}</span>
          </span>
          <span className="grow" />
          {!handed && (
            <button
              type="button"
              className="icon-btn"
              disabled={chat.status === 'working'}
              title={chat.status === 'working' ? 'Continue in a terminal (once Claude has finished)' : 'Continue in a terminal'}
              aria-label="Continue in a terminal"
              onClick={() => void core.chatTerminal(chat.id).catch((e) => alertIn(chat.id, String(e)))}
            >
              <SquareTerminal />
            </button>
          )}
          {close}
        </header>
        <Conversation chat={chat} session={session} />
      </main>
    )
  }

  // Sessions Kuchu follows, and conversations started here that Claude Code no longer reports (their process
  // closed), the latest first.
  const rows: { id: string; session?: Session; chat?: Chat; updated: number }[] = [
    ...sessions.map((s) => ({ id: s.id, session: s, chat: chats[s.id], updated: s.updated })),
    ...Object.values(chats)
      .filter((c) => !sessions.some((s) => s.id === c.id))
      .map((c) => ({ id: c.id, chat: c, updated: c.updated })),
  ].sort((a, b) => b.updated - a.updated)

  return (
    <main className="sheet">
      <header className="sheet-head">
        <span className="brand">
          <Pet size={22} mood={view?.mood ?? 'idle'} />
          Kuchu
        </span>
        <span className="grow" />
        <button type="button" className="icon-btn" title="New conversation" aria-label="New conversation" onClick={() => setScreen({ kind: 'new' })}>
          <Plus />
        </button>
        <button type="button" className="icon-btn" title="Settings" aria-label="Settings" onClick={() => void core.openSettings()}>
          <Settings />
        </button>
        {close}
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

      {rows.length === 0 ? (
        <div className="empty">
          <Pet size={56} mood="idle" />
          <p className="title">No Claude Code sessions right now</p>
          <p className="muted">Start one in a terminal or your editor, or here. It shows up in this list, and beside Kuchu.</p>
          <button type="button" className="btn is-small" onClick={() => setScreen({ kind: 'new' })}>
            <Plus />
            New conversation
          </button>
        </div>
      ) : (
        <ul className="list">
          {rows.map(({ id, session: s, chat: c, updated }) => {
            const name = s?.name || (c ? chatName(c) : '')
            const open = c ? () => setScreen({ kind: 'chat', id }) : undefined
            return (
              <li
                key={id}
                className={`row${open ? ' is-open' : ''}`}
                onClick={open}
                onKeyDown={(e) => open && e.key === 'Enter' && open()}
                tabIndex={open ? 0 : undefined}
                role={open ? 'button' : undefined}
              >
                <div className="row-head">
                  {c && <MessageSquare className="row-mark" aria-label="Started here: you can answer it" />}
                  <span className="row-title" title={s?.prompt || name}>
                    {name}
                  </span>
                  <span className={`state${s?.mood === 'needs' ? ' is-needs' : ''}`}>{s ? MOOD_LABEL[s.mood] : c ? CHAT_LABEL[c.status] : ''}</span>
                  <button
                    type="button"
                    className="forget"
                    title="Forget this session"
                    aria-label={`Forget ${name}`}
                    onClick={(e) => {
                      e.stopPropagation()
                      void core.forget(id)
                    }}
                  >
                    <X />
                  </button>
                </div>
                {(s?.line || c) && <div className="row-line">{s?.line || (c ? lastLine(c) : '')}</div>}
                <div className="row-meta" title={s?.cwd ?? c?.cwd}>
                  {s?.project ?? c?.project} · {ago(updated)}
                </div>
              </li>
            )
          })}
        </ul>
      )}
      <footer className="sheet-foot">
        <kbd>Ctrl</kbd> <kbd>Alt</kbd> <kbd>K</kbd> opens this from anywhere
      </footer>
    </main>
  )
}

const CHAT_LABEL: Record<Chat['status'], string> = { working: 'Working', ready: 'Idle', handed: 'In a terminal' }

/** A conversation's name until Claude Code gives it one: its first message. */
function chatName(chat: Chat) {
  return chat.entries.find((e) => e.kind === 'you')?.text.split('\n')[0] ?? chat.project
}

function lastLine(chat: Chat) {
  const last = [...chat.entries].reverse().find((e) => e.kind !== 'you')
  return last ? last.text.split('\n').find((l) => l.trim()) ?? '' : ''
}

/** An error from the core, said in the conversation's own window. */
function alertIn(id: string, text: string) {
  window.dispatchEvent(new CustomEvent('kuchu-chat-error', { detail: { id, text } }))
}

/** A new conversation: the folder Claude works in, and the first message. */
function NewChat({ folders, onStarted }: { folders: string[]; onStarted: (id: string) => void }) {
  const [folder, setFolder] = useState<string | null>(folders[0] ?? null)
  const [text, setText] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const shown = folder && !folders.includes(folder) ? [folder, ...folders].slice(0, FOLDERS) : folders
  const choose = () => void core.pickFolder().then((f) => f && setFolder(f))
  const send = () => {
    if (!folder || !text.trim() || busy) return
    setBusy(true)
    setError(null)
    core.chatStart(folder, text).then(onStarted, (e) => {
      setError(String(e))
      setBusy(false)
    })
  }
  return (
    <div className="new">
      <div className="new-label">Where Claude works</div>
      <div className="folders" role="radiogroup" aria-label="Folder">
        {shown.map((f) => (
          <button key={f} type="button" role="radio" aria-checked={f === folder} className="folder" title={f} onClick={() => setFolder(f)}>
            <Folder />
            <span className="folder-name">{baseName(f)}</span>
            <span className="folder-path">{parentOf(f)}</span>
          </button>
        ))}
        <button type="button" className="folder is-pick" onClick={choose}>
          <FolderOpen />
          <span className="folder-name">Choose a folder…</span>
        </button>
      </div>
      <span className="grow" />
      {error && <p className="note-warn talk-error">{error}</p>}
      <Composer
        value={text}
        onChange={setText}
        onSend={send}
        disabled={!folder || busy}
        placeholder={folder ? `What should Claude do in ${baseName(folder)}?` : 'Choose a folder first'}
      />
    </div>
  )
}

/** A conversation started here: what was said and done, and the box to answer in. */
function Conversation({ chat, session }: { chat: Chat; session?: Session }) {
  const [text, setText] = useState('')
  const [error, setError] = useState<string | null>(null)
  const scroller = useRef<HTMLDivElement>(null)
  const pinned = useRef(true)
  const working = chat.status === 'working'

  useEffect(() => {
    const onError = (e: Event) => {
      const { id, text } = (e as CustomEvent<{ id: string; text: string }>).detail
      if (id === chat.id) setError(text)
    }
    window.addEventListener('kuchu-chat-error', onError)
    return () => window.removeEventListener('kuchu-chat-error', onError)
  }, [chat.id])

  // Follows what Claude writes, unless you scrolled up to read.
  useLayoutEffect(() => {
    const el = scroller.current
    if (el && pinned.current) el.scrollTop = el.scrollHeight
  })

  const send = () => {
    const t = text.trim()
    if (!t) return
    setText('')
    setError(null)
    pinned.current = true
    core.chatSend(chat.id, t).catch((e) => {
      setError(String(e))
      setText(t)
    })
  }

  return (
    <>
      <div
        ref={scroller}
        className="talk"
        role="log"
        aria-live="polite"
        onScroll={(e) => {
          const el = e.currentTarget
          pinned.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40
        }}
      >
        {chat.entries.map((entry, i) => (
          <Said key={i} entry={entry} />
        ))}
        {working && (
          <div className="talk-working">
            <Pet size={18} mood={session?.mood === 'needs' ? 'needs' : 'working'} />
            {session?.mood === 'needs' ? session.line : 'Working…'}
          </div>
        )}
      </div>
      {error && <p className="note-warn talk-error">{error}</p>}
      {chat.status === 'handed' ? (
        <p className="talk-handed">This conversation goes on in a terminal.</p>
      ) : (
        <Composer
          value={text}
          onChange={setText}
          onSend={send}
          onStop={working ? () => void core.chatStop(chat.id) : undefined}
          placeholder={working ? 'Add to what Claude is doing…' : 'Reply to Claude…'}
        />
      )}
    </>
  )
}

function Said({ entry }: { entry: ChatEntry }) {
  switch (entry.kind) {
    case 'you':
      return <div className="say is-you">{entry.text}</div>
    case 'claude':
      return (
        <div className="say is-claude">
          <Prose text={entry.text} />
        </div>
      )
    case 'step':
      return (
        <div className="say-step">
          <Wrench />
          <span>{entry.text}</span>
        </div>
      )
    case 'note':
      return <div className="say-note">{entry.text}</div>
  }
}

/** Claude's Markdown, read plainly: code blocks as code, the rest as text without its marks. */
function Prose({ text }: { text: string }) {
  // Split on fenced code (one still streaming runs to the end): even parts are text, odd parts code.
  const parts = text.split(/```[^\n]*\n?([\s\S]*?)(?:```|$)/)
  return (
    <>
      {parts.map((part, i) =>
        i % 2 ? (
          <pre key={i} className="say-code">
            {part.replace(/\n$/, '')}
          </pre>
        ) : (
          part.trim() && (
            <p key={i} className="say-text">
              {plain(part.trim())}
            </p>
          )
        ),
      )}
    </>
  )
}

function plain(text: string) {
  const bare = text
    .replace(/^#{1,6}\s+/gm, '')
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/\*\*([^*\n]+)\*\*/g, '$1')
  return bare.split(/(`[^`\n]+`)/).map((bit, i) => (bit.length > 2 && bit.startsWith('`') && bit.endsWith('`') ? <code key={i}>{bit.slice(1, -1)}</code> : bit))
}

/** The message box: Enter sends, Shift+Enter starts a new line; while Claude works and the box is empty, Stop. */
function Composer({
  value,
  onChange,
  onSend,
  onStop,
  placeholder,
  disabled,
}: {
  value: string
  onChange: (v: string) => void
  onSend: () => void
  onStop?: () => void
  placeholder: string
  disabled?: boolean
}) {
  const ready = value.trim().length > 0 && !disabled
  const box = useRef<HTMLTextAreaElement>(null)
  useEffect(() => box.current?.focus(), [])
  return (
    <div className="composer">
      <textarea
        ref={box}
        rows={1}
        value={value}
        placeholder={placeholder}
        aria-label={placeholder}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
            e.preventDefault()
            if (ready) onSend()
          }
        }}
      />
      {onStop && !ready ? (
        <button type="button" className="send is-stop" title="Stop" aria-label="Stop" onClick={onStop}>
          <Square />
        </button>
      ) : (
        <button type="button" className="send" title="Send (Enter)" aria-label="Send" disabled={!ready} onClick={onSend}>
          <ArrowUp />
        </button>
      )}
    </div>
  )
}

function baseName(path: string) {
  return path.replace(/[\\/]+$/, '').split(/[\\/]/).pop() || path
}

function parentOf(path: string) {
  const parts = path.replace(/[\\/]+$/, '').split(/[\\/]/)
  return parts.length > 1 ? parts.slice(0, -1).join(path.includes('\\') ? '\\' : '/') : ''
}
