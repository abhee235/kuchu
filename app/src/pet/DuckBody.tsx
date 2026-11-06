// DuckBody.tsx — a yellow bath duck, the one you explain your bugs to: two plain shiny eyes, a wide orange bill across its face that does
// its talking, a curl on its head and a small blue neckerchief.

import { Creature, type Look } from './Creature'

const SKULL = 'M60 15 C86 15 106 31 106 53 C106 73 86 87 60 87 C34 87 14 73 14 53 C14 31 34 15 60 15 Z'

export function DuckBody({ id, u, heart }: { id: string; u: (name: string) => string; heart: string }) {
  const look: Look = {
    defs: (
      <>
        <radialGradient id={`${id}duck-coat`} cx="0.34" cy="0.2" r="1">
          <stop offset="0" stopColor="#fff6b0" />
          <stop offset="0.5" stopColor="#ffdc4a" />
          <stop offset="1" stopColor="#efb017" />
        </radialGradient>
        <linearGradient id={`${id}duck-wing`} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#efb52a" />
          <stop offset="0.4" stopColor="#ffe36b" />
          <stop offset="1" stopColor="#ecae1e" />
        </linearGradient>
        <radialGradient id={`${id}duck-belly`} cx="0.42" cy="0.3" r="0.85">
          <stop offset="0" stopColor="#fffbdc" />
          <stop offset="1" stopColor="#ffec9c" />
        </radialGradient>
        <linearGradient id={`${id}duck-bill`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#ffb04a" />
          <stop offset="1" stopColor="#ec7d16" />
        </linearGradient>
        <linearGradient id={`${id}duck-scarf`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#55a6e0" />
          <stop offset="1" stopColor="#2f79bd" />
        </linearGradient>
      </>
    ),
    clip: <path d={SKULL} />,
    coat: u('duck-coat'),
    limb: u('duck-wing'),
    paw: u('duck-bill'),
    hand: u('duck-wing'),
    head: (
      <>
        <path className="pet-shell" d={SKULL} fill={u('duck-coat')} />
        <ellipse cx="38" cy="25" rx="14" ry="4.6" transform="rotate(-20 38 25)" fill={u('shine')} />
      </>
    ),
    tuft: <path d="M56 17 C53 9 59 3 67 6 C63 7 61.5 10 64 13 C65 14.5 65 16 64 17 Z" fill={u('duck-coat')} stroke="#c98f12" strokeWidth="0.9" strokeLinejoin="round" />,
    nape: <path className="pet-mark" d="M46 40 q7 6 14 0 q7 6 14 0 M42 54 q9 6 18 0 q9 6 18 0" />,
    belly: <ellipse cx="60" cy="110" rx="19" ry="14.5" fill={u('duck-belly')} />,
    back: <path className="pet-mark" d="M42 100 q9 7 18 0 q9 7 18 0 M44 110 q8 6 16 0 q8 6 16 0" />,
    chest: (
      <>
        <path d="M43 90 Q60 99 77 90 L60 106 Z" fill={u('duck-scarf')} stroke="#25639c" strokeWidth="0.7" strokeLinejoin="round" />
        <circle cx="60" cy="96.6" r="3.8" fill={u('ball')} />
        <circle className="pet-bulb-ember" cx="60" cy="96.6" r="3.8" fill={u('ember')} />
        <circle className="pet-spark" cx="58.9" cy="95.4" r="0.9" />
      </>
    ),
    tail: <path className="pet-shell" d="M82 112 C92 114 101 108 106 96 C110 106 105 120 92 125 C88 126 84 124 82 122 Z" fill={u('duck-coat')} />,
    face: (
      <>
        <ellipse cx="26" cy="56" rx="7.5" ry="4.6" fill="#ff9d5c" fillOpacity="0.4" />
        <ellipse cx="94" cy="56" rx="7.5" ry="4.6" fill="#ff9d5c" fillOpacity="0.4" />
      </>
    ),
    eye: (x) => (
      <>
        <circle cx={x} cy="44" r="7" fill="#1f1a14" />
        <circle className="pet-spark" cx={x - 2.4} cy="41.2" r="2.5" />
        <circle className="pet-spark" cx={x + 2.4} cy="46.8" r="1.1" />
      </>
    ),
    eyes: { x: [40, 80], y: 44, w: 9 },
    nose: (
      <>
        {/* Its bill stays; the line along it is the part that changes. */}
        <path className="pet-bill" d="M33 59 C41 49 79 49 87 59 C89 65 77 69 60 69 C43 69 31 65 33 59 Z" fill={u('duck-bill')} />
        <ellipse cx="50" cy="55.6" rx="9" ry="2.2" fill="#fff" fillOpacity="0.32" />
        <path className="pet-nostril" d="M53.6 58 h1.6 M64.8 58 h1.6" />
      </>
    ),
    mouth: {
      smile: 'M37 62 Q60 72 83 62',
      grin: <path className="pet-bill" d="M44 67 Q60 79 76 67 Q60 71 44 67 Z" fill={u('duck-bill')} />,
      frown: 'M40 68 Q60 61 80 68',
      hmm: 'M42 65 H78',
      ponder: 'M42 67 Q62 69 80 61',
      wide: 'M31 58 Q60 82 89 58',
      yawn: [60, 72.6, 11, 8],
      o: [60, 72.4, 2.8],
      wow: [60, 73, 4],
    },
  }
  return <Creature id={id} u={u} heart={heart} look={look} clip="duck-head" />
}
