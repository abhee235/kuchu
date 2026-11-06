// Creature.tsx — the body every pet with two eyes shares, on the robot's skeleton: the same named parts in the same places, so that every
// mood and scene in pet.css moves it. Its left eye is `.pet-eye` and its right eye `.pet-pointer`. Each pet gives only what is its own, as
// a `Look`: its colours, head, ears, eyes, nose and mouth. The faces a mood needs (shut eyes, hearts, a spiral) are worked out here from
// where its eyes are.

import type { ReactNode } from 'react'
import './creature.css'

export interface Look {
  /** Its gradients. */
  defs: ReactNode
  /** The shapes its face stays inside, so a face looking aside never leaves its head. */
  clip: ReactNode
  /** Fills: its coat, its arms, its feet, and its hands if they are not like its feet. */
  coat: string
  limb: string
  paw: string
  hand?: string
  /** Its head, and what sits behind it, on it and at the back of it. */
  head: ReactNode
  ear?: ReactNode
  /** How far from the middle its ears stand, for turning round. */
  earX?: [number, number]
  tuft?: ReactNode
  nape: ReactNode
  /** Its body: the front, the back (seen only as it turns round), and what it wears on its chest. */
  belly: ReactNode
  back: ReactNode
  chest: ReactNode
  tail?: ReactNode
  /** The marks on its face, behind its eyes. */
  face: ReactNode
  /** One open eye. */
  eye: (x: number) => ReactNode
  eyes: { x: [number, number]; y: number; w: number }
  nose: ReactNode
  mouth: {
    smile: string
    /** What shows with its smile: a dog's tongue. */
    grin?: ReactNode
    frown: string
    hmm: string
    ponder: string
    /** The wide smile of the face it flips to. */
    wide: string
    yawn: [number, number, number, number]
    o: [number, number, number]
    wow: [number, number, number]
  }
}

/** An eye smiling, a shut one, and one squeezed tight, from its middle and half its width. */
const happy = (x: number, y: number, w: number) => `M${x - w} ${y + 3} Q${x} ${y - 9} ${x + w} ${y + 3}`
const shut = (x: number, y: number, w: number) => `M${x - w} ${y - 2} Q${x} ${y + 6.5} ${x + w} ${y - 2}`
const squeeze = (x: number, y: number, way: 1 | -1) => `M${x - 8 * way} ${y - 6.5} L${x + 5 * way} ${y} L${x - 8 * way} ${y + 6.5}`

function Leg({ side, x, foot, paw }: { side: 'l' | 'r'; x: number; foot: number; paw: string }) {
  return (
    <g className={`pet-leg-${side}`}>
      <rect className="pet-joint" x={x} y="120" width="10" height="10" rx="3" />
      <rect className="pet-shell" x={foot} y="126" width="22" height="10.5" rx="5.2" fill={paw} />
      <path className="pet-toes" d={`M${foot + 6.5} 132.6 v2.6 M${foot + 11.5} 132.6 v2.6 M${foot + 16.5} 132.6 v2.6`} />
    </g>
  )
}

function Arm({ side, x, limb, paw }: { side: 'l' | 'r'; x: number; limb: string; paw: string }) {
  return (
    <g className={`pet-arm-${side}`}>
      <rect className="pet-shell" x={x} y="93" width="11" height="26" rx="5.5" fill={limb} />
      <ellipse className="pet-shell" cx={x + 5.5} cy="116.4" rx="6" ry="5" fill={paw} />
    </g>
  )
}

export function Creature({ id, u, heart, look, clip }: { id: string; u: (name: string) => string; heart: string; look: Look; clip: string }) {
  const { eyes, mouth } = look
  const [lx, rx] = eyes.x
  const { y, w } = eyes
  const faceAt = (x: number) => `translate(${x} ${y})`
  return (
    <g className="pet-whole">
      <defs>
        {look.defs}
        <clipPath id={`${id}${clip}`}>{look.clip}</clipPath>
      </defs>

      {look.tail && <g className="pet-tail">{look.tail}</g>}

      <Leg side="l" x={45} foot={37} paw={look.paw} />
      <Leg side="r" x={65} foot={61} paw={look.paw} />

      <g className="pet-torso">
        <rect className="pet-shell" x="32" y="87" width="56" height="39" rx="19" fill={look.coat} />
        {look.belly}
        <ellipse cx="60" cy="90" rx="24" ry="4.5" fill={u('ao')} />
      </g>
      <g className="pet-hatch">{look.back}</g>
      <g className="pet-chest">{look.chest}</g>

      <Arm side="l" x={20} limb={look.limb} paw={look.hand ?? look.paw} />
      <Arm side="r" x={89} limb={look.limb} paw={look.hand ?? look.paw} />

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
        {look.ear && (
          <>
            <g className="pet-ear-l" data-x={look.earX?.[0]}>
              {look.ear}
            </g>
            <g className="pet-ear-r" data-x={look.earX?.[1]}>
              <g transform="matrix(-1 0 0 1 120 0)">{look.ear}</g>
            </g>
          </>
        )}
        <g className="pet-skull">{look.head}</g>
        {look.ear && (
          <>
            <g className="pet-ear-near pet-ear-l-near" data-x={look.earX?.[0]}>
              {look.ear}
            </g>
            <g className="pet-ear-near pet-ear-r-near" data-x={look.earX?.[1]}>
              <g transform="matrix(-1 0 0 1 120 0)">{look.ear}</g>
            </g>
          </>
        )}
        {look.tuft && <g className="pet-antenna">{look.tuft}</g>}
        <g className="pet-nape">{look.nape}</g>
        <g className="pet-screen">
          <g clipPath={u(clip)}>
            <g className="pet-face">
              {look.face}
              <g className="pet-eye">
                <g className="pet-eye-dot">{look.eye(lx)}</g>
                <path className="pet-line pet-happy" d={happy(lx, y, w)} />
              </g>
              <g className="pet-pointer">
                <g className="pet-eye-dot">{look.eye(rx)}</g>
                <path className="pet-line pet-happy" d={happy(rx, y, w)} />
              </g>
              <path className="pet-line pet-shut" d={shut(lx, y, w)} />
              <path className="pet-line pet-shut" d={shut(rx, y, w)} />
              <path className="pet-line pet-squeeze" d={squeeze(lx, y, 1)} />
              <path className="pet-line pet-squeeze" d={squeeze(rx, y, -1)} />
              <g transform={faceAt(lx)}>
                <path className="pet-heart" d={heart} />
              </g>
              <g transform={faceAt(rx)}>
                <path className="pet-heart" d={heart} />
              </g>
              <g transform={`translate(${lx - 46} ${y - 48})`}>
                <path className="pet-line pet-spiral" d="M46 48 a1.6 1.6 0 0 1 3.2 0 a3.2 3.2 0 0 1 -6.4 0 a4.8 4.8 0 0 1 9.6 0 a6.4 6.4 0 0 1 -12.8 0" />
              </g>
              <g transform={`translate(${rx - 70} ${y - 50})`}>
                <g className="pet-ask">
                  <path d="M66 44 a4.5 4.5 0 1 1 6.6 4 c-1.6 0.9 -2.4 1.9 -2.4 3.6" />
                  <circle cx="70.2" cy="56.4" r="1.8" />
                </g>
              </g>
              {/* Its nose stays; its mouth is the part that changes. */}
              {look.nose}
              <g className="pet-smile">
                {mouth.grin}
                <path className="pet-line" d={mouth.smile} />
              </g>
              <path className="pet-line pet-frown" d={mouth.frown} />
              <path className="pet-line pet-hmm" d={mouth.hmm} />
              <g className="pet-yawn">
                <ellipse cx={mouth.yawn[0]} cy={mouth.yawn[1]} rx={mouth.yawn[2]} ry={mouth.yawn[3]} fill="#6e2c26" />
                <ellipse className="pet-tongue" cx={mouth.yawn[0]} cy={mouth.yawn[1] + mouth.yawn[3] * 0.54} rx={mouth.yawn[2] * 0.6} ry={mouth.yawn[3] * 0.34} />
                <ellipse className="pet-line" cx={mouth.yawn[0]} cy={mouth.yawn[1]} rx={mouth.yawn[2]} ry={mouth.yawn[3]} />
              </g>
              <circle className="pet-line pet-o" cx={mouth.o[0]} cy={mouth.o[1]} r={mouth.o[2]} />
              <path className="pet-line pet-ponder" d={mouth.ponder} />
              <circle className="pet-line pet-wow" cx={mouth.wow[0]} cy={mouth.wow[1]} r={mouth.wow[2]} />
            </g>
            {/* The face it flips to: both eyes happy, a wide smile. */}
            <g className="pet-face-alt">
              {look.face}
              <path className="pet-line" d={happy(lx, y, w)} />
              <path className="pet-line" d={happy(rx, y, w)} />
              {look.nose}
              <path className="pet-line" d={mouth.wide} />
            </g>
          </g>
        </g>
      </g>
    </g>
  )
}
