// What the pages get from Kuchu's core, and what they ask of it (src-tauri/src/lib.rs, hub.rs, hooks.rs).

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { PetKind } from './pet/Pet'
import type { PetMood } from './pet/petMood'

export interface Session {
  id: string
  /** What the session is called: Claude Code's name for it, else its first message, else its folder. */
  name: string
  project: string
  cwd: string
  mood: PetMood
  line: string
  lines: Line[]
  prompt: string
  updated: number
  ended?: number | null
  /** When its last message was sent: the pet waves hello. */
  turn: number
}

/** Where a step stands: its tool running, finished, or failed (or cut short). */
export type Progress = 'running' | 'done' | 'failed'

/** A step (with where it stands), or a line of an answer (without). */
export interface Line {
  text: string
  progress: Progress | null
}

export interface Card {
  session: string
  title: string
  /** The project folder's name. */
  project: string
  /** The app it runs in, when a click can go there. */
  host: string | null
  line: string
  /** Where `line` stands, when it is a step. */
  progress: Progress | null
  lines: Line[]
  mood: PetMood
  /** When its turn ended, or started while it is under way. */
  at: number
  /** The other sessions at work or waiting on you, the latest first: the card can be switched to any of them. */
  others: Other[]
  /** The card shows this session because it was chosen, not because it is the latest. */
  pinned: boolean
}

/** Another session at work, as the card lists it: with "a few lines", its steps too. */
export interface Other {
  session: string
  name: string
  project: string
  host: string | null
  mood: PetMood
  line: string
  /** Where `line` stands, when it is a step. */
  progress: Progress | null
  lines: Line[]
}

export interface Approval {
  id: number
  session: string
  project: string
  /** The session's folder, always shown: a name is the model's to choose, a folder is not. */
  cwd: string
  kind: 'permission' | 'question' | 'plan'
  title: string
  /** The command, file or address it is about, whole, or cut and marked as cut when too long to read. */
  detail: string
  /** Whether `detail` is all of it. When it is not, Allow is not offered. */
  whole: boolean
  /** Claude Code's own "don't ask again" offer: the button's label, what it allows, where that is kept, and whether
      what it allows is a rule to show as code rather than words. */
  always?: { label: string; what: string; scope: string; code: boolean } | null
  question?: { text: string; options: string[] } | null
}

/** A conversation started from Kuchu (its id is the Claude Code session's). */
export interface Chat {
  id: string
  cwd: string
  project: string
  /** Working on a turn, ready for the next message, or taken on in a terminal. */
  status: 'working' | 'ready' | 'handed'
  entries: ChatEntry[]
  updated: number
}

/** Your message, Claude's text, a tool it used, or what Kuchu says about the conversation. */
export interface ChatEntry {
  kind: 'you' | 'claude' | 'step' | 'note'
  text: string
  /** A step's: where it stands. */
  progress?: Progress
}

export interface View {
  mood: PetMood
  card: Card | null
  approvals: Approval[]
  sessions: Session[]
}

export interface Prefs {
  pet: PetKind
  size: 'small' | 'base' | 'big'
  seeThrough: boolean
  card: 'line' | 'lines' | 'off'
  /** An answer stays beside the pet until it is clicked; otherwise it shows for a few seconds. */
  keepAnswer: boolean
  /** The other sessions at work stay shown beside the card; otherwise they show while "+N" is hovered. */
  keepOthers: boolean
  sounds: boolean
  click: 'sessions' | 'settings'
  approvals: boolean
  /** With no work, Kuchu plays now and then and naps after a long quiet. */
  lively: boolean
  /** Folders conversations were started in from Kuchu, the latest first. */
  folders: string[]
}

/** Whether Kuchu starts when you sign in. */
export interface Start {
  on: boolean
  /** The entry is there, but switched off in the system's own list of startup apps. */
  paused: boolean
  /** The program it starts, when it is another copy of Kuchu. */
  elsewhere: string | null
  /** Windows starts Kuchu itself (installed from the Store): switched on and off in Windows' own Settings. */
  byWindows: boolean
}

export interface HookReport {
  state: 'installed' | 'missing' | 'outdated' | 'unreadable'
  why?: string | null
  path: string
  disabled: boolean
}

export interface DiffLine {
  kind: '+' | '-' | ' ' | '…'
  text: string
}

export interface HookPlan {
  install: boolean
  path: string
  backup: string
  /** How many such copies stay beside the file. */
  keptCopies: number
  diff: DiffLine[]
  fingerprint: string
  unchanged: boolean
  respaced: boolean
}

export type ChoiceKind = 'allow' | 'deny' | 'always' | 'answer' | 'terminal'

export const SIZES = { small: 64, base: 96, big: 128 } as const

/** Kuchu's own pages on its website, by name: the core holds the addresses. */
export type SitePage = 'home' | 'docs' | 'changelog' | 'troubleshooting'

export const core = {
  view: () => invoke<View>('view'),
  /** Kuchu's version, for Settings. */
  version: () => invoke<string>('version'),
  prefs: () => invoke<Prefs>('get_prefs'),
  setPrefs: (patch: Partial<Prefs>) => invoke<Prefs>('set_prefs', { patch }),
  petClicked: () => invoke('pet_clicked'),
  forget: (session: string) => invoke('forget', { session }),
  /** Brings forward the app a session runs in. False if it has gone, or Windows would not. */
  openApp: (session: string) => invoke<boolean>('open_app', { session }),
  decide: (id: number, kind: ChoiceKind, answer?: string) => invoke('decide', { id, choice: { kind, answer } }),
  approvalShown: (id: number) => invoke('approval_shown', { id }),
  setWide: (wide: boolean) => invoke('set_wide', { wide }),
  /** How far the window can grow for the list of other sessions: up, and down past the pet. */
  listRoom: () => invoke<[number, number]>('list_room'),
  /** How much taller the window must be for the list of other sessions, above the card or below it; 0 once it closes. */
  setRoom: (more: number, below: boolean) => invoke('set_room', { more, below }),
  setRects: (rects: { x: number; y: number; w: number; h: number }[]) => invoke('set_rects', { rects }),
  isRight: () => invoke<boolean>('is_right'),
  stroll: (dx: number) => invoke<number>('stroll', { dx }),
  stopStroll: () => invoke('stop_stroll'),
  follow: (session: string | null) => invoke('follow', { session }),
  startAtSignIn: () => invoke<Start>('start_at_sign_in'),
  setStartAtSignIn: (on: boolean) => invoke<Start>('set_start_at_sign_in', { on }),
  hooksStatus: () => invoke<HookReport>('hooks_status'),
  hooksPlan: (install: boolean) => invoke<HookPlan>('hooks_plan', { install }),
  hooksApply: (install: boolean, fingerprint: string) => invoke<{ backup: string | null; report: HookReport }>('hooks_apply', { install, fingerprint }),
  openSettings: () => invoke('open_settings'),
  openSessions: (session?: string) => invoke('open_sessions', { session: session ?? null }),
  takeOpen: () => invoke<string | null>('take_open'),
  pickFolder: () => invoke<string | null>('pick_folder'),
  chats: () => invoke<Chat[]>('chats'),
  chatStart: (cwd: string, text: string) => invoke<string>('chat_start', { cwd, text }),
  chatSend: (session: string, text: string) => invoke('chat_send', { session, text }),
  chatStop: (session: string) => invoke('chat_stop', { session }),
  chatTerminal: (session: string) => invoke('chat_terminal', { session }),
  /** Opens one of Kuchu's own pages, in the browser. */
  openPage: (page: SitePage) => invoke('open_page', { page }),
  /** Opens the page for reporting an answer from Claude, in the browser. */
  reportAnswer: () => invoke('report_answer'),
}

/** Listens to one of the core's events for as long as the effect lives. */
export function on<T>(event: string, handle: (payload: T) => void): () => void {
  const off: Promise<UnlistenFn> = listen<T>(`kuchu://${event}`, (e) => handle(e.payload))
  return () => void off.then((stop) => stop())
}

/** "now", "5 min", "3 h", or a day. */
export function ago(ms: number, now = Date.now()): string {
  const mins = Math.round((now - ms) / 60_000)
  if (mins < 1) return 'now'
  if (mins < 60) return `${mins} min`
  if (mins < 24 * 60) return `${Math.round(mins / 60)} h`
  return new Date(ms).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })
}

export const MOOD_LABEL: Record<PetMood, string> = {
  idle: 'Idle',
  thinking: 'Thinking',
  working: 'Working',
  needs: 'Needs you',
  done: 'Done',
  failed: 'Did not finish',
}
