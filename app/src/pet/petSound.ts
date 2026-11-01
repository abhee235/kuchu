// petSound.ts — Kuchu's small tones: made in the page (no sound files), short, one when a session starts to need you,
// when one is done, and when one failed, each session on its own (the core says when). Never while a state lasts, never
// for thinking or working (they change too often).

import type { PetMood } from './petMood'

/** Each tone: its notes (Hz), how long each rings (s), the wave, and how loud at its start (0..1). The first version
 *  (0.06 to 0.08, a tenth of a second) could not be heard over a laptop's speakers at an ordinary volume. */
const TONES: Partial<Record<PetMood, { notes: number[]; each: number; wave: OscillatorType; gain: number }>> = {
  // a two-note ping: something waits for you
  needs: { notes: [880, 880], each: 0.16, wave: 'triangle', gain: 0.32 },
  // a small rising chime: done
  done: { notes: [659, 988], each: 0.22, wave: 'sine', gain: 0.3 },
  // a soft falling pair: it did not finish
  failed: { notes: [440, 330], each: 0.24, wave: 'sine', gain: 0.26 },
}

/** The tones, in the order Settings plays them. */
export const TONE_MOODS = ['needs', 'done', 'failed'] as const satisfies readonly PetMood[]

let audio: AudioContext | undefined

export function playTone(mood: PetMood): void {
  const tone = TONES[mood]
  if (!tone) return
  try {
    audio ??= new AudioContext()
    // A context made before any click on the page may start suspended: wake it (the tone follows once it runs).
    if (audio.state === 'suspended') void audio.resume()
    const start = audio.currentTime + 0.02
    tone.notes.forEach((hz, i) => {
      const at = start + i * (tone.each * 0.6)
      const osc = audio!.createOscillator()
      const gain = audio!.createGain()
      osc.type = tone.wave
      osc.frequency.value = hz
      // A soft start and a bell-like end, so no note clicks.
      gain.gain.setValueAtTime(0, at)
      gain.gain.linearRampToValueAtTime(tone.gain, at + 0.01)
      gain.gain.exponentialRampToValueAtTime(0.0001, at + tone.each)
      osc.connect(gain).connect(audio!.destination)
      osc.start(at)
      osc.stop(at + tone.each + 0.02)
    })
  } catch {
    /* no audio here: the pet still shows it */
  }
}
