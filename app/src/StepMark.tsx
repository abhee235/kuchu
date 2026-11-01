// A step's mark: a turning ring while its tool runs, a green check once it is done, a grey cross if it failed or was
// cut short. And the mark for what Claude said between its steps.

import { Check, MessageCircle, X } from 'lucide-react'
import type { Progress } from './api'

const LABEL: Record<Progress, string> = { running: 'Running', done: 'Done', failed: 'Did not finish' }

export function StepMark({ progress, small }: { progress: Progress; small?: boolean }) {
  return (
    <span className={`mark is-${progress}${small ? ' is-small' : ''}`} role="img" aria-label={LABEL[progress]}>
      {progress === 'done' && <Check strokeWidth={3.5} />}
      {progress === 'failed' && <X strokeWidth={3.5} />}
    </span>
  )
}

export function SaidMark() {
  return (
    <span className="mark is-said" role="img" aria-label="Claude said">
      <MessageCircle strokeWidth={2.5} />
    </span>
  )
}
