---
title: How Kuchu works
head: How Kuchu works: Claude Code hooks and a local pipe
nav: How it works
order: 10
description: Kuchu hears Claude Code through hooks, a tiny helper and a local pipe. It never slows Claude Code down, never allows anything itself and sends nothing out.
---
## Hooks, a helper and a pipe

Claude Code can run a program on events in a session: a message sent, a tool started or finished, a permission
asked, a turn done. These are its **hooks**. When you connect Kuchu, it adds hooks for these events to Claude Code's
settings:

`SessionStart`, `SessionEnd`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PostToolUseFailure`,
`PermissionRequest`, `Notification`, `Stop`, `StopFailure` and `PreCompact`.

Each runs Kuchu's helper, `kuchu-relay`, a small native program, with the event on its input. The helper hands the
event to Kuchu over a local pipe, `\.\pipe\kuchu-<you>` on Windows or `~/.kuchu/kuchu.sock` on a Mac, and exits.
Kuchu turns the events into the pet's mood and the cards.

## Never in Claude Code's way

- The helper reaches Kuchu within 300 ms or gives up, and always exits cleanly. When Kuchu is not running, it does
  nothing, and Claude Code carries on as if Kuchu were not there.
- It drops what tools returned and shortens long text before passing an event on. A permission request's input stays
  whole, so what you allow is exactly what runs.
- It prints nothing back to Claude Code, except a decision you made on a permission card.

## Permission requests

For a permission request, the helper waits for your answer. Each side gives up before the one it waits on: Kuchu
after 108 seconds, the helper after 110, Claude Code's hook after 120. So an unanswered request always goes back to
Claude Code, which asks in its own window. Kuchu answers only with what Claude Code offered: allow, deny, one of its
own suggestions, or your answer to its question.

## Yours alone

- The pipe is your account's alone: on Windows, only your account may open it, and the helper checks that the other
  end is your own Kuchu before it sends anything.
- Kuchu reads a session's transcript only for its name and what Claude said between steps, and only a plain file in
  Claude Code's own folder.
- Nothing from your sessions is written to disk by Kuchu, and nothing goes over the network.

The full security model, with every threat considered and how it is handled, is in the
[threat model]({{REPO}}/blob/main/docs/threat-model.md). What Claude Code actually sends, as measured, is in
[docs/hooks.md]({{REPO}}/blob/main/docs/hooks.md).
