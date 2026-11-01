// petMood.ts — what Kuchu shows, and where its eyes look. Pure, so each rule is tested.

export type PetMood = 'idle' | 'thinking' | 'working' | 'needs' | 'done' | 'failed'

/** An answer's first line of words, as one line: Markdown's marks dropped. */
export function firstLine(markdown: string): string {
  const line = markdown.split('\n').map((l) => l.trim()).find((l) => /[\p{L}\p{N}]/u.test(l)) ?? ''
  return line.replace(/^(?:[#>]+\s*|[-*+]\s+|\d+[.)]\s+)+/, '').replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/[*_`~]/g, '').slice(0, 160)
}

/** Where the eyes look, as -1..1 each way, from the pointer's offset from the pet's centre: toward the pointer, more
 *  the further it is up to `reach`, and straight ahead when it is far away (`ignore`) or right on the pet. */
export function lookToward(offset: { dx: number; dy: number }, reach = 160, ignore = 900): { x: number; y: number } {
  const d = Math.hypot(offset.dx, offset.dy)
  if (d < 8 || d > ignore) return { x: 0, y: 0 }
  const k = Math.min(1, d / reach) / d
  return { x: Math.round(offset.dx * k * 100) / 100, y: Math.round(offset.dy * k * 100) / 100 }
}
