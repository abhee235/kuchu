// OwlBody.tsx — a dusk-blue owl: huge amber eyes in a pale heart of a face, a small orange beak, tufts for ears and wings for arms.

import { Creature, type Look } from './Creature'

const SKULL = 'M60 14 C88 14 107 30 107 54 C107 74 86 87 60 87 C34 87 13 74 13 54 C13 30 32 14 60 14 Z'

export function OwlBody({ id, u, heart }: { id: string; u: (name: string) => string; heart: string }) {
  const look: Look = {
    defs: (
      <>
        <radialGradient id={`${id}owl-coat`} cx="0.34" cy="0.2" r="1">
          <stop offset="0" stopColor="#a9b6dc" />
          <stop offset="0.5" stopColor="#7d8cbd" />
          <stop offset="1" stopColor="#535f92" />
        </radialGradient>
        <linearGradient id={`${id}owl-limb`} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#56639a" />
          <stop offset="0.4" stopColor="#8e9dcb" />
          <stop offset="1" stopColor="#4f5c91" />
        </linearGradient>
        <radialGradient id={`${id}owl-cream`} cx="0.42" cy="0.32" r="0.8">
          <stop offset="0" stopColor="#fffdf6" />
          <stop offset="1" stopColor="#ece2cf" />
        </radialGradient>
        <radialGradient id={`${id}owl-iris`} cx="0.42" cy="0.38" r="0.72">
          <stop offset="0" stopColor="#ffe48f" />
          <stop offset="0.6" stopColor="#f5a92c" />
          <stop offset="1" stopColor="#c8710f" />
        </radialGradient>
        <linearGradient id={`${id}owl-foot`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#f6ad4e" />
          <stop offset="1" stopColor="#dc8420" />
        </linearGradient>
      </>
    ),
    clip: <path d={SKULL} />,
    coat: u('owl-coat'),
    limb: u('owl-limb'),
    paw: u('owl-foot'),
    ear: <path className="pet-shell" d="M21 36 C17 28 16 20 17 12 C19 10 21 10 23 11 C29 14 36 18 41 22 Z" fill={u('owl-coat')} />,
    earX: [28, 92],
    head: (
      <>
        <path className="pet-shell" d={SKULL} fill={u('owl-coat')} />
        <ellipse cx="38" cy="24" rx="14" ry="4.6" transform="rotate(-20 38 24)" fill={u('shine')} />
      </>
    ),
    nape: <path className="pet-mark" d="M42 34 q6 6 12 0 M60 34 q6 6 12 0 M36 46 q6 6 12 0 M54 46 q6 6 12 0 M72 46 q6 6 12 0 M44 58 q6 6 12 0 M62 58 q6 6 12 0" />,
    belly: (
      <>
        <ellipse cx="60" cy="109" rx="19" ry="15" fill={u('owl-cream')} />
        <path className="pet-mark" d="M47 104 q3.5 4 7 0 M57 104 q3.5 4 7 0 M67 104 q3.5 4 7 0 M52 111 q3.5 4 7 0 M62 111 q3.5 4 7 0" />
      </>
    ),
    back: <path className="pet-mark" d="M46 98 q5 5 10 0 M64 98 q5 5 10 0 M40 107 q5 5 10 0 M55 107 q5 5 10 0 M70 107 q5 5 10 0" />,
    chest: (
      <>
        <circle cx="60" cy="96.6" r="4" fill={u('ball')} />
        <circle className="pet-bulb-ember" cx="60" cy="96.6" r="4" fill={u('ember')} />
        <circle className="pet-spark" cx="58.8" cy="95.4" r="1" />
      </>
    ),
    tail: <path className="pet-shell" d="M80 120 C90 122 96 128 98 134 C90 133 82 130 76 125 Z" fill={u('owl-limb')} />,
    face: (
      <>
        {/* Its heart of a face, and the soft dip of feathers between its eyes. */}
        <path d="M60 37 C50 25 22 27 18 50 C15 69 38 83 60 84 C82 83 105 69 102 50 C98 27 70 25 60 37 Z" fill={u('owl-cream')} />
        <path className="pet-brow" d="M47 33 Q55 34 60 40 Q65 34 73 33" />
      </>
    ),
    eye: (x) => (
      <>
        <circle cx={x} cy="52" r="15" fill="#3d3a44" fillOpacity="0.14" />
        <circle cx={x} cy="52" r="13.4" fill={u('owl-iris')} />
        <circle cx={x} cy="52" r="7.6" fill="#1d1a22" />
        <circle className="pet-spark" cx={x - 3.6} cy="47.4" r="3" />
        <circle className="pet-spark" cx={x + 3.2} cy="55.6" r="1.3" />
      </>
    ),
    eyes: { x: [41, 79], y: 52, w: 12 },
    nose: <path d="M60 58 L65.6 63.6 Q63 69 60 71 Q57 69 54.4 63.6 Z" fill={u('owl-foot')} stroke="#b76a14" strokeWidth="0.7" strokeLinejoin="round" />,
    mouth: {
      smile: 'M51 74.4 Q60 80 69 74.4',
      frown: 'M52 79 Q60 74 68 79',
      hmm: 'M54 76.4 H66',
      ponder: 'M53 77 Q61 78 68 74',
      wide: 'M47 73 Q60 84 73 73',
      yawn: [60, 76.4, 6, 6],
      o: [60, 77, 2.2],
      wow: [60, 77, 3],
    },
  }
  return <Creature id={id} u={u} heart={heart} look={look} clip="owl-head" />
}
