// CatBody.tsx — Kuchu as a ginger cat, on the robot's skeleton: the same named parts in the same places, so that every mood and
// scene in pet.css moves it too. Its left eye is `.pet-eye` and its right eye `.pet-pointer`, its fur tuft takes the antenna's
// part and its collar bell the antenna's light. What only a cat has, its tail and its own settings of those parts, is in cat.css.

import './cat.css'
import './creature.css'

/** One of its eyes: an amber iris, a slit of a pupil, and the light caught in it. */
function Eye({ x, u }: { x: number; u: (name: string) => string }) {
  return (
    <g className="pet-eye-dot">
      <ellipse cx={x} cy="50" rx="8.6" ry="9.4" fill={u('cat-iris')} />
      <ellipse cx={x} cy="50" rx="3" ry="7.2" fill="#2a170b" />
      <circle className="pet-spark" cx={x - 2.8} cy="45.6" r="2.3" />
      <circle className="pet-spark" cx={x + 2.6} cy="53.4" r="1" />
    </g>
  )
}

/** One of its ears, drawn pointing up and to the left and mirrored for the other side. */
function Ear({ right, u }: { right?: boolean; u: (name: string) => string }) {
  return (
    <g transform={right ? 'matrix(-1 0 0 1 120 0)' : undefined}>
      <path className="pet-shell" d="M17 40 C15 26 17 12 23 5 C25 3 27 3 29 5 C37 11 46 17 53 22 Z" fill={u('cat-fur')} />
      <path d="M24 33 C22 25 23 17 26 12 C32 16 38 20 44 24 Z" fill={u('cat-inner')} />
    </g>
  )
}

export function CatBody({ id, u, heart }: { id: string; u: (name: string) => string; heart: string }) {
  return (
    <g className="pet-whole">
      <defs>
        {/* Light from the top left, as on the robot: the fur brightest there, deeper toward the bottom right. */}
        <radialGradient id={`${id}cat-fur`} cx="0.34" cy="0.22" r="0.98">
          <stop offset="0" stopColor="#ffd9a6" />
          <stop offset="0.45" stopColor="#f7ae5e" />
          <stop offset="0.8" stopColor="#e88f37" />
          <stop offset="1" stopColor="#c86f20" />
        </radialGradient>
        <linearGradient id={`${id}cat-limb`} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#d9842e" />
          <stop offset="0.4" stopColor="#fbbc72" />
          <stop offset="1" stopColor="#d37c28" />
        </linearGradient>
        <radialGradient id={`${id}cat-cream`} cx="0.4" cy="0.3" r="0.85">
          <stop offset="0" stopColor="#fffaf0" />
          <stop offset="1" stopColor="#fbe2c3" />
        </radialGradient>
        <linearGradient id={`${id}cat-inner`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#f6a999" />
          <stop offset="1" stopColor="#fbd0be" />
        </linearGradient>
        <radialGradient id={`${id}cat-iris`} cx="0.42" cy="0.38" r="0.72">
          <stop offset="0" stopColor="#fff0a8" />
          <stop offset="0.55" stopColor="#f2bd3e" />
          <stop offset="1" stopColor="#b9761a" />
        </radialGradient>
        <linearGradient id={`${id}cat-collar`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#d9503a" />
          <stop offset="1" stopColor="#a52d1f" />
        </linearGradient>
        {/* A face slides across its head, never past its edge, as one does on the robot's screen. */}
        <clipPath id={`${id}cat-head`}>
          <path d="M60 15 C84 15 104 29 107 51 C109 69 89 87 60 87 C31 87 11 69 13 51 C16 29 36 15 60 15 Z" />
        </clipPath>
      </defs>

      {/* Its tail, behind everything, swaying from where it joins the body. */}
      <g className="pet-tail">
        <path d="M82 120 C98 126 111 117 107 97" fill="none" stroke="#c86f20" strokeWidth="11" strokeLinecap="round" />
        <path d="M82 120 C98 126 111 117 107 97" fill="none" stroke={u('cat-fur')} strokeWidth="9" strokeLinecap="round" />
        <path className="pet-tail-ring" d="M99 119.5 l5 4 M106.5 112 l5.5 2 M108 103 l5.5 -0.5" />
        <path d="M107.6 100 C107.4 98.6 107.2 97.8 107 97" fill="none" stroke="#fff4e4" strokeWidth="9" strokeLinecap="round" />
      </g>

      {/* Legs and paws, each its own so they can step. */}
      <g className="pet-leg-l">
        <rect className="pet-joint" x="45" y="120" width="10" height="10" rx="3" />
        <rect className="pet-shell" x="37" y="126" width="22" height="10.5" rx="5.2" fill={u('cat-cream')} />
        <path className="pet-toes" d="M43.5 132.6 v2.6 M48.5 132.6 v2.6 M53.5 132.6 v2.6" />
      </g>
      <g className="pet-leg-r">
        <rect className="pet-joint" x="65" y="120" width="10" height="10" rx="3" />
        <rect className="pet-shell" x="61" y="126" width="22" height="10.5" rx="5.2" fill={u('cat-cream')} />
        <path className="pet-toes" d="M66.5 132.6 v2.6 M71.5 132.6 v2.6 M76.5 132.6 v2.6" />
      </g>

      {/* The body with its pale belly; its back, seen only as it turns round, has its stripes. */}
      <g className="pet-torso">
        <rect className="pet-shell" x="32" y="87" width="56" height="39" rx="19" fill={u('cat-fur')} />
        <ellipse cx="60" cy="110" rx="17.5" ry="14" fill={u('cat-cream')} />
        <path className="pet-stripe" d="M34.5 103 l7 1.4 M34 111 l7 0.4 M85.5 103 l-7 1.4 M86 111 l-7 0.4" />
        <ellipse cx="60" cy="90" rx="24" ry="4.5" fill={u('ao')} />
      </g>
      <g className="pet-hatch">
        <path className="pet-stripe" d="M46 98 Q60 104 74 98 M45 106 Q60 112 75 106 M47 114 Q60 119 73 114" />
      </g>

      {/* Its collar and bell on the front; the bell glows when it needs you, as the robot's antenna does. */}
      <g className="pet-chest">
        <path d="M43 90 Q60 98 77 90 L77 94 Q60 102 43 94 Z" fill={u('cat-collar')} />
        <circle cx="60" cy="101" r="4.4" fill={u('ball')} />
        <circle className="pet-bulb-ember" cx="60" cy="101" r="4.4" fill={u('ember')} />
        <path className="pet-bell-slit" d="M57.4 102.6 H62.6" />
        <circle className="pet-spark" cx="58.6" cy="99.6" r="1" />
      </g>

      {/* Arms, in front of the body so they reach the laptop. */}
      <g className="pet-arm-l">
        <rect className="pet-shell" x="20" y="93" width="11" height="26" rx="5.5" fill={u('cat-limb')} />
        <ellipse className="pet-shell" cx="25.5" cy="116.4" rx="6" ry="5" fill={u('cat-cream')} />
      </g>
      <g className="pet-arm-r">
        <rect className="pet-shell" x="89" y="93" width="11" height="26" rx="5.5" fill={u('cat-limb')} />
        <ellipse className="pet-shell" cx="94.5" cy="116.4" rx="6" ry="5" fill={u('cat-cream')} />
      </g>

      {/* Working: the same small laptop, lines of code appearing on it. */}
      <g className="pet-laptop">
        <rect x="39" y="95" width="42" height="18" rx="3.5" fill="#262a35" />
        <rect className="pet-code" x="44" y="100" width="20" height="2" rx="1" fill="#ee8a1e" />
        <rect className="pet-code pet-c2" x="44" y="104" width="28" height="2" rx="1" fill="#fff6ea" fillOpacity="0.7" />
        <rect className="pet-code pet-c3" x="44" y="108" width="14" height="2" rx="1" fill="#fff6ea" fillOpacity="0.7" />
        <rect className="pet-caret" x="60" y="107.5" width="1.6" height="3" fill="#ee8a1e" />
        <rect x="34" y="112.5" width="52" height="5" rx="2.5" fill="#d9d0c3" />
      </g>

      <path className="pet-spring" d="M60 90 l-5 -2 l10 -4 l-10 -4 l10 -4 l-10 -4 l10 -4 l-10 -4 l5 -2" />

      <g className="pet-head">
        <rect className="pet-joint pet-neck" x="52" y="84" width="16" height="7" rx="3" />
        {/* The ears stand on the head and swing round with it, a copy of each in front of the head for the one turned
            towards you. */}
        <g className="pet-ear-l" data-x="31">
          <Ear u={u} />
        </g>
        <g className="pet-ear-r" data-x="89">
          <Ear right u={u} />
        </g>
        <g className="pet-skull">
          <path
            className="pet-shell"
            d="M60 15 C84 15 104 29 107 51 C109 69 89 87 60 87 C31 87 11 69 13 51 C16 29 36 15 60 15 Z"
            fill={u('cat-fur')}
          />
          <ellipse cx="36" cy="25" rx="15" ry="5" transform="rotate(-20 36 25)" fill={u('shine')} />
        </g>
        <g className="pet-ear-near pet-ear-l-near" data-x="31">
          <Ear u={u} />
        </g>
        <g className="pet-ear-near pet-ear-r-near" data-x="89">
          <Ear right u={u} />
        </g>
        {/* The tuft on its head: the part the robot's antenna plays. */}
        <g className="pet-antenna">
          <path d="M53 19 C54 12 57 9 60 6 C60.5 10 62 12 64 8 C65.5 12 66.5 15 67 19 Z" fill="#e88f37" stroke="#c86f20" strokeWidth="0.8" strokeLinejoin="round" />
        </g>
        {/* The back of its head, seen only as it turns round. */}
        <g className="pet-nape">
          <path className="pet-stripe" d="M60 28 V46 M46 31 Q47 40 50 46 M74 31 Q73 40 70 46 M38 44 Q60 58 82 44" />
        </g>
        {/* Its face, all of it turning towards what it looks at. */}
        <g className="pet-screen">
          <g clipPath={u('cat-head')}>
          <g className="pet-face">
            <path className="pet-stripe" d="M60 22 V31 M50 24 l2 7 M70 24 l-2 7" />
            <ellipse cx="60" cy="67" rx="23" ry="15.5" fill={u('cat-cream')} />
            <ellipse cx="29" cy="65" rx="7" ry="4.2" fill="#f4907a" fillOpacity="0.32" />
            <ellipse cx="91" cy="65" rx="7" ry="4.2" fill="#f4907a" fillOpacity="0.32" />
            <path className="pet-whisker" d="M31 59 L15 55 M31 64.5 L14 65 M89 59 L105 55 M89 64.5 L106 65" />
            <g className="pet-eye">
              <Eye x={42} u={u} />
              <path className="pet-line pet-happy" d="M33 53 Q42 41 51 53" />
            </g>
            <g className="pet-pointer">
              <Eye x={78} u={u} />
              <path className="pet-line pet-happy" d="M69 53 Q78 41 87 53" />
            </g>
            <path className="pet-line pet-shut" d="M33 48 Q42 56.5 51 48" />
            <path className="pet-line pet-shut" d="M69 48 Q78 56.5 87 48" />
            <path className="pet-line pet-squeeze" d="M34 43.5 L47 50 L34 56.5" />
            <path className="pet-line pet-squeeze" d="M86 43.5 L73 50 L86 56.5" />
            <g transform="translate(42 50)">
              <path className="pet-heart" d={heart} />
            </g>
            <g transform="translate(78 50)">
              <path className="pet-heart" d={heart} />
            </g>
            <g transform="translate(-4 2)">
              <path className="pet-line pet-spiral" d="M46 48 a1.6 1.6 0 0 1 3.2 0 a3.2 3.2 0 0 1 -6.4 0 a4.8 4.8 0 0 1 9.6 0 a6.4 6.4 0 0 1 -12.8 0" />
            </g>
            <g transform="translate(8 2)">
              <g className="pet-ask">
                <path d="M66 44 a4.5 4.5 0 1 1 6.6 4 c-1.6 0.9 -2.4 1.9 -2.4 3.6" />
                <circle cx="70.2" cy="56.4" r="1.8" />
              </g>
            </g>
            {/* Its nose stays; its mouth is the part that changes. */}
            <path className="pet-nose" d="M56.6 58.4 Q60 57.2 63.4 58.4 Q62.4 61.2 60 62 Q57.6 61.2 56.6 58.4 Z" />
            <path className="pet-line pet-smile" d="M60 62 V64.4 M51 65 Q55.5 69.6 60 64.4 Q64.5 69.6 69 65" />
            <path className="pet-line pet-frown" d="M60 62 V64.4 M53 70 Q60 63 67 70" />
            <path className="pet-line pet-hmm" d="M60 62 V65 M55 66.6 H65" />
            <g className="pet-yawn">
              <ellipse cx="60" cy="70" rx="6" ry="7" fill="#7a2f22" />
              <ellipse className="pet-tongue" cx="60" cy="73.8" rx="3.6" ry="2.4" />
              <ellipse className="pet-line" cx="60" cy="70" rx="6" ry="7" />
            </g>
            <circle className="pet-line pet-o" cx="62" cy="68" r="2" />
            <path className="pet-line pet-ponder" d="M60 62 V64 M55 67 Q61 68 67 64.6" />
            <circle className="pet-line pet-wow" cx="60" cy="68.6" r="3" />
          </g>
          {/* The face it flips to: both eyes happy, a wide smile. */}
          <g className="pet-face-alt">
            <ellipse cx="60" cy="67" rx="23" ry="15.5" fill={u('cat-cream')} />
            <path className="pet-line" d="M33 53 Q42 41 51 53" />
            <path className="pet-line" d="M69 53 Q78 41 87 53" />
            <path className="pet-nose" d="M56.6 58.4 Q60 57.2 63.4 58.4 Q62.4 61.2 60 62 Q57.6 61.2 56.6 58.4 Z" />
            <path className="pet-line" d="M60 62 V64 M48 64 Q54 74 60 64 Q66 74 72 64" />
          </g>
          </g>
        </g>
      </g>
    </g>
  )
}
