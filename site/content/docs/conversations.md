---
title: Conversations with Claude from Kuchu
nav: Conversations
order: 6
description: Start a conversation with Claude in any folder from Kuchu's sessions list. It runs your own Claude Code, so Claude answers under your own account; stop it, or carry it on in a terminal.
---
Besides following your sessions, Kuchu can start one: a conversation with Claude in a folder you choose, without
opening a terminal.

## Start one

1. Open your sessions: click the pet, or press **Ctrl+Alt+K**.
2. Click **New conversation**.
3. Choose a folder: one you used before, or another one.
4. Say what Claude should do, and press **Enter** to send. **Shift+Enter** starts a new line.

Claude answers in the same window, with each step it takes. Permission requests show on the pet's card, as for any
session.

## While Claude works

- **Add to what Claude is doing**: type and send, and Claude takes it in.
- **Stop**: with the message box empty, the send button becomes a stop button.
- **Continue in a terminal**: once Claude has finished, this opens the conversation in a terminal with
  `claude --resume`, where it carries on as any Claude Code session.
- **Report an answer**: the flag opens a new issue on GitHub in your browser, to report an answer from Claude that is
  wrong or inappropriate. Nothing is sent with it.

A conversation's Claude Code process closes after ten quiet minutes; the next message takes the conversation up
again.

## Under your own account

Kuchu runs your own Claude Code for a conversation, in the folder you chose, so Claude answers under your own Claude
account, its plan and its terms, with your Claude Code settings. Kuchu sends nothing anywhere itself.

Sessions you started in a terminal or an editor are followed, not typed into: Claude Code takes their messages only
there.
