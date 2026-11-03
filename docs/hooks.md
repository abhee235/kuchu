# What Claude Code sends Kuchu

Measured with `kuchu-relay record` on Claude Code 2.1.310 (Windows 11, 2026-11-01). The payloads, with personal paths
replaced, are in [`fixtures/claude-code-2.1.310/`](../fixtures/claude-code-2.1.310).

## How a hook runs

- **On Windows, through Git Bash**, whichever terminal started Claude Code: `$0` in the hook command was
  `C:/Program Files/Git/usr/bin/bash` for a session started from PowerShell. So the command is a quoted path with
  forward slashes, then arguments: `"C:/…/kuchu-relay.exe" <event>`.
- The event's JSON comes on stdin; the hook's `env` settings reach the relay.
- `CLAUDE_CODE_ENTRYPOINT` is inherited from whatever started the session (it read `claude-vscode` for a session
  started from a terminal inside the editor), so it is a hint, not a fact.

## The events and what Kuchu reads from them

Every event carries `session_id`, `cwd`, `transcript_path` and `hook_event_name`; most carry `prompt_id` (the
message that started the work), `permission_mode` and `effort`.

| Event | Also carries | Kuchu shows |
|---|---|---|
| SessionStart | `source` (`startup`, `resume`, `compact`) | a new session (after compacting, the same one) |
| UserPromptSubmit | `prompt` | thinking |
| PreToolUse | `tool_name`, `tool_input`, `tool_use_id` | working: the tool and its target |
| PostToolUse | `tool_response`, `duration_ms`, `tool_use_id` | the step done (`tool_response` is dropped by the relay) |
| PostToolUseFailure | `error`, `is_interrupt`, `duration_ms`, `tool_use_id` | the step failed |
| SubagentStart, SubagentStop | `agent_id`, `agent_type`; Stop also `last_assistant_message` (the helper's own) | nothing: a helper's end is not the session's |
| PreCompact | `trigger` (`manual`, …), `custom_instructions` | "Making room in its memory…" |
| PermissionRequest | `tool_name`, `tool_input`, `permission_suggestions` (no `tool_use_id`) | needs you: an approval card |
| Stop | `last_assistant_message`, `background_tasks`, `session_crons`, `stop_hook_active` | done, with the answer's first line |
| SessionEnd | `reason` | the session gone |

Each step's mark comes from the tool call's `tool_use_id`, the same in PreToolUse and PostToolUse (or
PostToolUseFailure): running from the one, done or failed from the other.

What Claude says between tools is not in any event. Kuchu reads it from the transcript (`transcript_path`): the text
written just before the tool call with that `tool_use_id`. The transcript is written a moment after the hook runs, so
this is tried when the tool starts and again when it ends. In some sessions Claude Code keeps a text as an empty
thinking block instead; that text cannot be shown.

Recorded on 2.1.310 with `claude -p`: PostToolUseFailure (a Read of a missing file), SubagentStart and SubagentStop
(a helper agent), PreCompact (`/compact` on a resumed session), and a PermissionRequest for a Read outside the folder
(Claude Code offers an `addRules` suggestion). In `-p` runs, plan mode ends without a request and AskUserQuestion is
not offered, so those two requests, Notification and StopFailure still need an interactive session to record.
(From Git Bash, an argument like `/compact` is turned into a Windows path; run it from PowerShell.)

## Answering a permission request

- **Allow**, checked on 2.1.310 (a Write that was blocked went through):
  `{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}` on stdout, exit 0.
- **No answer**: print nothing and exit 0; Claude Code asks as it would have.
- **Allow from Kuchu**, checked end to end (2026-11-01): Claude Code asked to write a file, Kuchu's card showed it,
  a click on Allow reached the relay, and the file was written.
- **No answer** in a run without a terminal (`claude -p`): Claude Code treats it as a refusal and says so.
- **Answered in Claude Code's own window** while the card shows (seen 2026-11-03, a question in the editor): Claude
  Code goes on, but the relay's connection stays open, and the card stayed with it. The card now goes when that
  call's PostToolUse or PostToolUseFailure arrives, or the turn ends.
- Deny (with a `message`), "Always" (`updatedPermissions` with Claude Code's first `permission_suggestions` entry),
  answers to a question and plan approval (`updatedInput`) are built from the recorded shapes and covered by tests;
  each is checked live before the first release.
- A `claude -p` run ends its session right after `Stop`: a session that ends with its answer unseen stays until the
  pet is clicked.

## A conversation started from Kuchu

Kuchu runs one Claude Code process per conversation, measured on 2.1.310 (recordings in
[`fixtures/claude-code-2.1.310/stream/`](../fixtures/claude-code-2.1.310/stream)):

- `claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages
  --session-id <id>` in the chosen folder; later processes for the same conversation use `--resume <id>`.
- Each message is one line on stdin: `{"type":"user","message":{"role":"user","content":"…"}}`. A message sent while
  Claude works is taken into the same turn (one `result` for both).
- What comes back, one JSON object a line: `system`/`init` at the start of each turn; `stream_event` with
  `text_delta` pieces as Claude writes; `assistant` with each whole block (`text`, `tool_use`, `thinking`); `result`
  at the end of the turn (`subtype` `success`, or `error_during_execution` with `terminal_reason`
  `aborted_streaming` when stopped). Lines with a `parent_tool_use_id` are a helper's own.
- Stopping: `{"type":"control_request","request_id":"…","request":{"subtype":"interrupt"}}` on stdin; Claude Code
  answers with a `control_response` and ends the turn. The same process takes the next message.
- Permission prompts go to the PermissionRequest hook, as in any `claude -p` run, so they show on Kuchu like any
  other session's. A stopped turn sends no Stop to the hooks.
- Closing stdin lets the process finish its turn and exit. A terminal takes the conversation on with
  `claude --resume <id>`; started from Kuchu, it must get the new console's own input and output (through `start`
  on Windows), or Claude Code finds no terminal and exits.

## Sessions that stop without a word

A session whose terminal is closed sends no SessionEnd, and one stopped with Esc sends no Stop. When a session Kuchu
shows as busy has been silent for 15 seconds, Kuchu asks `claude agents --json` (measured on 2.1.310: about 0.2 s; it
lists every running session, interactive, background or `-p`, with its `sessionId` and a `status` of `busy` or
`idle`; recording in [`fixtures/claude-code-2.1.310/cli/`](../fixtures/claude-code-2.1.310/cli)). A session it no
longer lists is gone; one it lists as idle is no longer busy. If Claude Code cannot be asked, nothing changes, and a
silent session still falls back to idle after 30 minutes.
