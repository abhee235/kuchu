// FrogBody.tsx — a green frog: its eyes up on two domes above its head, a smile from one side of its face to the other, and a little
// orange bow tie.

import { Creature, type Look } from './Creature'

const SKULL = 'M60 31 C88 31 110 41 110 60 C110 78 88 87 60 87 C32 87 10 78 10 60 C10 41 32 31 60 31 Z'
const DOMES = [36, 84]
const DOME_Y = 31

export function FrogBody({ id, u, heart }: { id: string; u: (name: string) => string; heart: string }) {
  const look: Look = {
    defs: (
      <>
        <radialGradient id={`${id}frog-coat`} cx="0.34" cy="0.18" r="1">
          <stop offset="0" stopColor="#cdec86" />
          <stop offset="0.5" stopColor="#8ccf58" />
          <stop offset="1" stopColor="#559c39" />
        </radialGradient>
        <linearGradient id={`${id}frog-limb`} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#5ea33f" />
          <stop offset="0.4" stopColor="#a4dc6c" />
          <stop offset="1" stopColor="#58993a" />
        </linearGradient>
        <radialGradient id={`${id}frog-belly`} cx="0.42" cy="0.3" r="0.85">
          <stop offset="0" stopColor="#fbfde8" />
          <stop offset="1" stopColor="#e2f0b4" />
        </radialGradient>
        <radialGradient id={`${id}frog-white`} cx="0.42" cy="0.36" r="0.72">
          <stop offset="0" stopColor="#ffffff" />
          <stop offset="1" stopColor="#efe9d2" />
        </radialGradient>
      </>
    ),
    clip: (
      <>
        <path d={SKULL} />
        {DOMES.map((x) => (
          <circle key={x} cx={x} cy={DOME_Y} r="15" />
        ))}
      </>
    ),
    coat: u('frog-coat'),
    limb: u('frog-limb'),
    paw: u('frog-belly'),
    head: (
      <>
        {DOMES.map((x) => (
          <circle key={x} className="pet-shell" cx={x} cy={DOME_Y} r="15" fill={u('frog-coat')} />
        ))}
        <path className="pet-shell" d={SKULL} fill={u('frog-coat')} />
        {/* The seam where each dome meets its head would otherwise show as a line across it. */}
        {DOMES.map((x) => (
          <ellipse key={x} cx={x} cy={DOME_Y + 7} rx="13.4" ry="8" fill={u('frog-coat')} />
        ))}
        <ellipse cx="33" cy="20" rx="7" ry="3" transform="rotate(-24 33 20)" fill={u('shine')} />
      </>
    ),
    tuft: (
      <>
        <path d="M60 32 V25" fill="none" stroke="#559c39" strokeWidth="2.2" strokeLinecap="round" />
        <path d="M60 26 C53 26 49 22 48 17 C54 16 59 19 60 26 Z" fill={u('frog-limb')} stroke="#4c8c33" strokeWidth="0.8" strokeLinejoin="round" />
        <path d="M60 26 C65 25 69 21 70 15 C64 15 60 19 60 26 Z" fill={u('frog-limb')} stroke="#4c8c33" strokeWidth="0.8" strokeLinejoin="round" />
      </>
    ),
    nape: (
      <>
        <circle cx="46" cy="52" r="5" fill="#559c39" fillOpacity="0.55" />
        <circle cx="72" cy="46" r="3.6" fill="#559c39" fillOpacity="0.55" />
        <circle cx="66" cy="62" r="6" fill="#559c39" fillOpacity="0.55" />
      </>
    ),
    belly: <ellipse cx="60" cy="110" rx="20" ry="15" fill={u('frog-belly')} />,
    back: (
      <>
        <circle cx="50" cy="104" r="4.4" fill="#559c39" fillOpacity="0.55" />
        <circle cx="68" cy="100" r="3.2" fill="#559c39" fillOpacity="0.55" />
        <circle cx="66" cy="113" r="4.8" fill="#559c39" fillOpacity="0.55" />
      </>
    ),
    chest: (
      <>
        <path d="M60 96 L47 90.6 Q45 96.5 47 102.4 Z" fill={u('ball')} stroke="#b85d10" strokeWidth="0.7" strokeLinejoin="round" />
        <path d="M60 96 L73 90.6 Q75 96.5 73 102.4 Z" fill={u('ball')} stroke="#b85d10" strokeWidth="0.7" strokeLinejoin="round" />
        <circle cx="60" cy="96.4" r="3.6" fill={u('ball')} stroke="#b85d10" strokeWidth="0.7" />
        <circle className="pet-bulb-ember" cx="60" cy="96.4" r="3.6" fill={u('ember')} />
      </>
    ),
    face: (
      <>
        <ellipse cx="23" cy="62" rx="7.5" ry="4.4" fill="#f4907a" fillOpacity="0.42" />
        <ellipse cx="97" cy="62" rx="7.5" ry="4.4" fill="#f4907a" fillOpacity="0.42" />
      </>
    ),
    eye: (x) => (
      <>
        <circle cx={x} cy={DOME_Y} r="11.8" fill={u('frog-white')} stroke="#4c8c33" strokeOpacity="0.5" strokeWidth="0.7" />
        <circle cx={x} cy={DOME_Y + 0.6} r="6.8" fill="#1b2118" />
        <circle className="pet-spark" cx={x - 2.6} cy={DOME_Y - 2.4} r="2.5" />
        <circle className="pet-spark" cx={x + 2.4} cy={DOME_Y + 3.2} r="1.1" />
      </>
    ),
    eyes: { x: [36, 84], y: DOME_Y, w: 10 },
    nose: (
      <>
        <circle cx="55.6" cy="52" r="1.3" fill="#3f7a2a" />
        <circle cx="64.4" cy="52" r="1.3" fill="#3f7a2a" />
      </>
    ),
    mouth: {
      smile: 'M26 60 Q60 82 94 60',
      frown: 'M38 74 Q60 60 82 74',
      hmm: 'M42 68 H78',
      ponder: 'M42 71 Q62 72 80 63',
      wide: 'M22 57 Q60 94 98 57',
      yawn: [60, 68, 12, 10],
      o: [62, 68, 3],
      wow: [60, 68, 4.4],
    },
  }
  return <Creature id={id} u={u} heart={heart} look={look} clip="frog-head" />
}
