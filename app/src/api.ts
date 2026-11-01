// What the pages get from Kuchu's core, and what they ask of it (src-tauri/src/lib.rs, hub.rs, hooks.rs).

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { PetMood } from './pet/petMood'

export interface Session {
  id: string
  /** What the session is called: Claude Code's name for it, else its first message, else its folder. */
  name: string
  project: string
  cwd: string
  mood: PetMood
  line: string
  lines: string[]
  prompt: string
  updated: number
  ended?: number | null
}

export interface Card {
  session: string
  title: string
  line: string
  lines: string[]
  mood: PetMood
  /** When its turn ended, or started while it is under way. */
  at: number
}

export interface Approval {
  id: number
  session: string
  project: string
  kind: 'permission' | 'question' | 'plan'
  title: string
  detail: string
  always?: string | null
  question?: { text: string; options: string[] } | null
}

export interface View {
  mood: PetMood
  card: Card | null
  approvals: Approval[]
  sessions: Session[]
}

export interface Prefs {
  size: 'small' | 'base' | 'big'
  seeThrough: boolean
  card: 'line' | 'lines' | 'off'
  /** An answer stays beside the pet until it is clicked; otherwise it shows for a few seconds. */
  keepAnswer: boolean
  sounds: boolean
  click: 'sessions' | 'settings'
  approvals: boolean
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
  diff: DiffLine[]
  fingerprint: string
  unchanged: boolean
  respaced: boolean
}

export type ChoiceKind = 'allow' | 'deny' | 'always' | 'answer' | 'terminal'

export const SIZES = { small: 64, base: 96, big: 128 } as const

export const core = {
  view: () => invoke<View>('view'),
  prefs: () => invoke<Prefs>('get_prefs'),
  setPrefs: (patch: Partial<Prefs>) => invoke<Prefs>('set_prefs', { patch }),
  petClicked: () => invoke('pet_clicked'),
  forget: (session: string) => invoke('forget', { session }),
  decide: (id: number, kind: ChoiceKind, answer?: string) => invoke('decide', { id, choice: { kind, answer } }),
  approvalShown: (id: number) => invoke('approval_shown', { id }),
  setWide: (wide: boolean) => invoke('set_wide', { wide }),
  setRects: (rects: { x: number; y: number; w: number; h: number }[]) => invoke('set_rects', { rects }),
  isRight: () => invoke<boolean>('is_right'),
  hooksStatus: () => invoke<HookReport>('hooks_status'),
  hooksPlan: (install: boolean) => invoke<HookPlan>('hooks_plan', { install }),
  hooksApply: (install: boolean, fingerprint: string) => invoke<{ backup: string | null; report: HookReport }>('hooks_apply', { install, fingerprint }),
  openSettings: () => invoke('open_settings'),
  openSessions: () => invoke('open_sessions'),
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
