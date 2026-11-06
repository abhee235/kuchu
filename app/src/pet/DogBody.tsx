// DogBody.tsx — a brown pup: a dark patch over one eye, soft ears hanging at the sides of its head, a big black nose, and its tongue out
// when it smiles.

import { Creature, type Look } from './Creature'

const SKULL = 'M60 16 C86 16 106 30 107 52 C108 72 88 87 60 87 C32 87 12 72 13 52 C14 30 34 16 60 16 Z'

export function DogBody({ id, u, heart }: { id: string; u: (name: string) => string; heart: string }) {
  const look: Look = {
    defs: (
      <>
        <radialGradient id={`${id}dog-coat`} cx="0.34" cy="0.22" r="0.98">
          <stop offset="0" stopColor="#e6c39a" />
          <stop offset="0.5" stopColor="#c99868" />
          <stop offset="1" stopColor="#99683d" />
        </radialGradient>
        <linearGradient id={`${id}dog-limb`} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#a87649" />
          <stop offset="0.4" stopColor="#d8ad7e" />
          <stop offset="1" stopColor="#a06e42" />
        </linearGradient>
        <radialGradient id={`${id}dog-cream`} cx="0.4" cy="0.3" r="0.85">
          <stop offset="0" stopColor="#fffaf1" />
          <stop offset="1" stopColor="#f3e2ca" />
        </radialGradient>
        <linearGradient id={`${id}dog-ear`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#7f4f2e" />
          <stop offset="1" stopColor="#57341e" />
        </linearGradient>
        <radialGradient id={`${id}dog-patch`} cx="0.4" cy="0.3" r="0.8">
          <stop offset="0" stopColor="#b98052" />
          <stop offset="1" stopColor="#99643b" />
        </radialGradient>
        <linearGradient id={`${id}dog-collar`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#3b95a6" />
          <stop offset="1" stopColor="#23697a" />
        </linearGradient>
      </>
    ),
    clip: <path d={SKULL} />,
    coat: u('dog-coat'),
    limb: u('dog-limb'),
    paw: u('dog-cream'),
    head: (
      <>
        <path className="pet-shell" d={SKULL} fill={u('dog-coat')} />
        <ellipse cx="38" cy="26" rx="13" ry="4.4" transform="rotate(-22 38 26)" fill={u('shine')} />
        {/* Its ears hang over the sides of its head. */}
        <path className="pet-shell" d="M30 19 C14 18 3 32 4 52 C5 65 12 73 19 69 C26 65 24 48 34 26 Z" fill={u('dog-ear')} />
        <path className="pet-shell" d="M90 19 C106 18 117 32 116 52 C115 65 108 73 101 69 C94 65 96 48 86 26 Z" fill={u('dog-ear')} />
      </>
    ),
    tuft: <path d="M54 18 C55 13 57 11 60 8 C61 11 63 11 66 10 C66.4 13 66.4 16 66 18 Z" fill="#a87649" stroke="#7f4f2e" strokeWidth="0.8" strokeLinejoin="round" />,
    nape: <ellipse cx="60" cy="46" rx="22" ry="17" fill="#8a5a36" fillOpacity="0.5" />,
    belly: <ellipse cx="60" cy="110" rx="17.5" ry="14" fill={u('dog-cream')} />,
    back: <ellipse cx="60" cy="106" rx="19" ry="12" fill="#8a5a36" fillOpacity="0.55" />,
    chest: (
      <>
        <path d="M43 90 Q60 98 77 90 L77 94.6 Q60 102.6 43 94.6 Z" fill={u('dog-collar')} />
        <circle cx="60" cy="101.6" r="4.2" fill={u('ball')} />
        <circle className="pet-bulb-ember" cx="60" cy="101.6" r="4.2" fill={u('ember')} />
        <circle className="pet-spark" cx="58.7" cy="100.2" r="1" />
      </>
    ),
    tail: (
      <>
        <path d="M84 116 C96 116 105 108 103 94" fill="none" stroke="#99683d" strokeWidth="10.5" strokeLinecap="round" />
        <path d="M84 116 C96 116 105 108 103 94" fill="none" stroke={u('dog-coat')} strokeWidth="8.5" strokeLinecap="round" />
        <path d="M103.4 97 C103.3 96 103.2 95 103 94" fill="none" stroke="#fff4e4" strokeWidth="8.5" strokeLinecap="round" />
      </>
    ),
    face: (
      <>
        {/* The pale blaze up its face, its muzzle, and the patch over its right eye: its mark. */}
        <path d="M52 30 Q60 25 68 30 L72 58 H48 Z" fill={u('dog-cream')} />
        <ellipse cx="60" cy="68" rx="25" ry="17" fill={u('dog-cream')} />
        <ellipse cx="80" cy="46" rx="16" ry="17.5" transform="rotate(10 80 46)" fill={u('dog-patch')} />
        <ellipse cx="33" cy="66" rx="6.5" ry="3.8" fill="#ee8f7a" fillOpacity="0.3" />
        <ellipse cx="87" cy="66" rx="6.5" ry="3.8" fill="#ee8f7a" fillOpacity="0.3" />
      </>
    ),
    eye: (x) => (
      <>
        <ellipse cx={x} cy="47" rx="8" ry="9" fill="#2a170f" />
        <ellipse cx={x} cy="48.4" rx="5.4" ry="6.2" fill="#5a3320" fillOpacity="0.7" />
        <circle className="pet-spark" cx={x - 2.8} cy="43.2" r="2.5" />
        <circle className="pet-spark" cx={x + 2.6} cy="50.6" r="1.1" />
      </>
    ),
    eyes: { x: [41, 80], y: 47, w: 9 },
    nose: (
      <>
        <path d="M51 58.4 Q60 54.4 69 58.4 Q68 65 60 67.4 Q52 65 51 58.4 Z" fill="#2c2422" />
        <ellipse cx="56.6" cy="58.8" rx="2.8" ry="1.5" fill="#fff" fillOpacity="0.42" />
      </>
    ),
    mouth: {
      smile: 'M60 67.4 V70.6 M48 70 Q54 77 60 70.6 Q66 77 72 70',
      grin: <path className="pet-grin" d="M55 73.4 H66 V78.4 a5.5 5.5 0 0 1 -11 0 Z" />,
      frown: 'M60 67.4 V70.6 M52 78 Q60 71 68 78',
      hmm: 'M60 67.4 V71.4 M54 73.4 H66',
      ponder: 'M60 67.4 V70.4 M54 75 Q61 76 68 71.6',
      wide: 'M60 67.4 V69.4 M46 69 Q53 81 60 69.4 Q67 81 74 69',
      yawn: [60, 75, 7, 7.6],
      o: [62, 75, 2.2],
      wow: [60, 75.4, 3.3],
    },
  }
  return <Creature id={id} u={u} heart={heart} look={look} clip="dog-head" />
}
