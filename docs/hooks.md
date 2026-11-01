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
| SessionStart | `source` (`startup`, …) | a new session |
| UserPromptSubmit | `prompt` | thinking |
| PreToolUse | `tool_name`, `tool_input`, `tool_use_id` | working: the tool and its target |
| PostToolUse | `tool_response`, `duration_ms`, `tool_use_id` | the step done (`tool_response` is dropped by the relay) |
| PermissionRequest | `tool_name`, `tool_input`, `permission_suggestions` (no `tool_use_id`) | needs you: an approval card |
| Stop | `last_assistant_message`, `background_tasks`, `session_crons`, `stop_hook_active` | done, with the answer's first line |
| SessionEnd | `reason` | the session gone |

Not seen yet (they need an interactive session or a particular case): Notification, PostToolUseFailure,
StopFailure, SubagentStart, SubagentStop, PreCompact, and the AskUserQuestion and ExitPlanMode requests. They are
recorded before the step that uses them.

## Answering a permission request

- **Allow**, checked on 2.1.310 (a Write that was blocked went through):
  `{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}` on stdout, exit 0.
- **No answer**: print nothing and exit 0; Claude Code asks as it would have.
- **Allow from Kuchu**, checked end to end (2026-11-01): Claude Code asked to write a file, Kuchu's card showed it,
  a click on Allow reached the relay, and the file was written.
- **No answer** in a run without a terminal (`claude -p`): Claude Code treats it as a refusal and says so.
- Deny (with a `message`), "Always" (`updatedPermissions` with Claude Code's first `permission_suggestions` entry),
  answers to a question and plan approval (`updatedInput`) are built from the recorded shapes and covered by tests;
  each is checked live before the first release.
- A `claude -p` run ends its session right after `Stop`: a session that ends with its answer unseen stays until the
  pet is clicked.
