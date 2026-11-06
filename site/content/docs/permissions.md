---
title: Answering permission requests
nav: Permission requests
order: 5
description: When Claude Code asks to run a command or change a file, Kuchu can show the request with Allow and Deny. Nothing is allowed without your click, and an unanswered request goes back to Claude Code.
---
When Claude Code needs your permission, to run a command or change a file, Kuchu can show the request on its card, so
you can answer without finding the right terminal.

## The card

A permission card shows:

- which session is asking, and in which folder;
- what it wants to do: **Running a command**, **Editing a file**, and so on;
- the command or the file, in full.

And your choices:

- **Allow**: this once.
- **Allow … for now** (when Claude Code offers it): for example, allow edits to files without asking, for the rest of
  this session. The card says exactly what that allows.
- **Deny**: Claude is told no, and carries on from there.
- **In Claude Code**: hands the request back; answer it in Claude Code's own window.

When Claude asks you a question with choices, the card shows them as buttons. When it has a plan ready, the card
offers **Approve plan** and **Keep planning**.

## What Kuchu will never do

- **Allow anything by itself.** Only your click allows.
- **Hide part of what you allow.** If the whole command does not fit on the card, Kuchu offers no Allow and says to
  answer it in Claude Code. Hidden characters, such as ones that reverse text, are shown as marks.
- **Run out the clock against you.** If you don't answer within about 100 seconds, the request goes back to Claude
  Code, which asks in its own window. A timeout never allows and never denies.

Claude Code asks in its own window at the same time, so you can answer in either place; the first answer counts, and
the card goes away when the request is answered in Claude Code.

## When cards don't show

- **Claude Code's permission mode.** In its default **Auto** mode, Claude decides on most tool calls itself, so few
  requests reach you, and so few reach Kuchu. In **manual** mode, every request asks you. Press **Shift+Tab** in
  Claude Code to switch between modes.
- **The setting.** In Kuchu's Settings, **Permission requests**, **Answer them on Kuchu** must be on.
- **Not connected.** See [Connecting to Claude Code](/docs/connect/).
