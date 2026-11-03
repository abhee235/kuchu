// turn.ts — Kuchu turning in depth, though it is drawn flat: round on the spot (about the up axis) or a backflip
// (about the axis from side to side). Each part has a place in depth and is moved, narrowed and shown or hidden as it
// would be if the pet were solid: the screen and the chest light face the front, a panel and a hatch the back; the
// head and the body are boxes, so side on they still show their depth and the pet never thins to a line; the ears,
// arms, legs and antenna stand out from the middle and swing round it, and one that goes behind the head or the body
// is hidden. pet.css does the rest of each scene around the turn (the crouch, the jump, the dizzy wobble).

/** How a part turns. A box keeps its depth; a face on the front or the back narrows to nothing side on; a knob keeps
 *  its size and only moves. An ear is hidden while it is well in front or behind; its copy over the head ("near") is
 *  shown while it is in front. A limb (an arm, a leg, the antenna) turns over with the body in a backflip, about the
 *  joint it hangs from; going round, an arm is hidden while it is behind the body. */
type Kind = 'box' | 'front' | 'back' | 'knob' | 'ear' | 'near' | 'limb'

interface Part {
  sel: string
  kind: Kind
  /** Its middle in the drawing (the viewBox's units; a limb's joint), and how far it is in front of the pet's middle
   *  (+) or behind. */
  x: number
  y: number
  z?: number
  /** A box's width, height and depth. */
  w?: number
  h?: number
  d?: number
  /** An arm: hidden behind the body. */
  hides?: boolean
}

/** The head is 94 wide and 64 deep, the body 56 wide and 40 deep: the screen and the chest light sit on their fronts. */
const PARTS: Part[] = [
  { sel: '.pet-skull', kind: 'box', x: 60, y: 51, w: 94, h: 74, d: 64 },
  { sel: '.pet-screen', kind: 'front', x: 60, y: 51, z: 32 },
  { sel: '.pet-nape', kind: 'back', x: 60, y: 51, z: -32 },
  { sel: '.pet-ear-l', kind: 'ear', x: 11, y: 52 },
  { sel: '.pet-ear-r', kind: 'ear', x: 109, y: 52 },
  { sel: '.pet-ear-l-near', kind: 'near', x: 11, y: 52 },
  { sel: '.pet-ear-r-near', kind: 'near', x: 109, y: 52 },
  { sel: '.pet-antenna', kind: 'limb', x: 60, y: 16 },
  { sel: '.pet-neck', kind: 'knob', x: 60, y: 87.5 },
  { sel: '.pet-torso', kind: 'box', x: 60, y: 107, w: 56, h: 38, d: 40 },
  { sel: '.pet-chest', kind: 'front', x: 60, y: 105.5, z: 20 },
  { sel: '.pet-hatch', kind: 'back', x: 60, y: 106.5, z: -20 },
  { sel: '.pet-arm-l', kind: 'limb', x: 25.5, y: 96, hides: true },
  { sel: '.pet-arm-r', kind: 'limb', x: 94.5, y: 96, hides: true },
  { sel: '.pet-leg-l', kind: 'limb', x: 49.5, y: 124 },
  { sel: '.pet-leg-r', kind: 'limb', x: 70.5, y: 124 },
]
/** The line it turns round: the middle of the drawing across, and its waist up and down for the backflip. */
const MIDDLE_X = 60
const MIDDLE_Y = 80
/** An ear this far in front of the head or behind it is out of its place at the head's edge. */
const EAR_OUT = 8
/** An arm this far behind the body is hidden by it. */
const ARM_BEHIND = 6
/** Where it stands every this many degrees of the turn. */
const STEP_DEG = 5

export interface Turn {
  /** Round on the spot, or over backwards. */
  way: 'round' | 'flip'
  turns: number
  delay: number
  ms: number
  /** How far round it is (0 to 1) at each moment (0 to 1) of the turn. */
  pace: (t: number) => number
  /** The left arm's swing at each moment (degrees, as pet.css swings it; the right arm's is the opposite). */
  arms: (t: number) => number
}

/** Plays `turn` on the pet under `root`; returns the animations, to cancel if the scene ends first. */
export function play(root: Element, turn: Turn): Animation[] {
  const steps = Math.ceil((turn.turns * 360) / STEP_DEG)
  return PARTS.flatMap((part) => {
    const el = root.querySelector<SVGGraphicsElement>(part.sel)
    if (!el) return []
    // The origin pet.css gives the part (an arm's shoulder), which every transform here is measured from.
    const [ox = 0, oy = 0] = getComputedStyle(el).transformOrigin.split(' ').map(parseFloat)
    const moves: Keyframe[] = []
    const shows: Keyframe[] = []
    for (let i = 0; i <= steps; i++) {
      const t = i / steps
      const at = place(part, turn.way, turn.pace(t) * turn.turns * 2 * Math.PI)
      const swing = part.hides ? turn.arms(t) * (part.x < MIDDLE_X ? 1 : -1) : 0
      moves.push({ offset: t, transform: transform(at, turn.way, part, ox, oy, swing) })
      shows.push({ offset: t, opacity: at.shown ? 1 : 0, easing: 'steps(1, end)' })
    }
    const timing = { duration: turn.ms, delay: turn.delay }
    return [el.animate(moves, timing), el.animate(shows, timing)]
  })
}

/** Where a part shows at angle `a`: moved along the turn's axis, narrowed by `scale`, and whether it is seen. */
function place(part: Part, way: Turn['way'], a: number) {
  const cos = Math.cos(a)
  const sin = Math.sin(a)
  const z = part.z ?? 0
  // Its distance from the middle across (round) or up and down (flip), and how far in front of the middle it is.
  const off = way === 'round' ? part.x - MIDDLE_X : part.y - MIDDLE_Y
  const across = way === 'round' ? off * cos + z * sin : off * cos - z * sin
  const front = way === 'round' ? -off * sin + z * cos : off * sin + z * cos
  let scale = 1
  let shown = true
  if (part.kind === 'box') {
    const size = way === 'round' ? part.w! : part.h!
    scale = (size * Math.abs(cos) + part.d! * Math.abs(sin)) / size
  } else if (part.kind === 'front') {
    scale = cos
    shown = cos > 0.05
  } else if (part.kind === 'back') {
    scale = cos
    shown = cos < -0.05
  } else if (part.kind === 'ear') {
    shown = Math.abs(front) <= EAR_OUT
  } else if (part.kind === 'near') {
    shown = way === 'round' && front > EAR_OUT
  } else if (part.kind === 'limb') {
    if (way === 'flip') scale = cos
    else shown = !part.hides || front > -ARM_BEHIND
  }
  return { move: across - off, scale, shown }
}

/** The transform that narrows a part about its own middle (a limb's joint) and moves it, measured from its origin
 *  (ox, oy), with an arm's swing about its shoulder first. */
function transform(at: { move: number; scale: number }, way: Turn['way'], part: Part, ox: number, oy: number, swing: number) {
  const s = Math.abs(at.scale) < 0.01 ? 0.01 * Math.sign(at.scale || 1) : at.scale
  const turn = swing ? ` rotate(${swing.toFixed(1)}deg)` : ''
  if (way === 'round') return `translate(${((1 - s) * (part.x - ox) + at.move).toFixed(2)}px, 0px) scale(${s.toFixed(3)}, 1)${turn}`
  return `translate(0px, ${((1 - s) * (part.y - oy) + at.move).toFixed(2)}px) scale(1, ${s.toFixed(3)})${turn}`
}

/** Round three times, slow to start and faster each time, the arms flung out a little. */
export const SPIN: Turn = {
  way: 'round',
  turns: 3,
  delay: 400,
  ms: 2700,
  pace: (t) => t ** 1.5,
  arms: (t) => 22 * Math.min(1, t * 6, (1 - t) * 6),
}

/** Over backwards once, quickest at the top, the arms kept up over its head as pet.css swung them for the jump. */
export const FLIP: Turn = {
  way: 'flip',
  turns: 1,
  delay: 720,
  ms: 1020,
  pace: (t) => t * t * (3 - 2 * t),
  arms: () => 150,
}
