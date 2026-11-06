---
title: Questions about Kuchu
description: Answers about Kuchu, the desk pet for Claude Code: what it is, what it costs, privacy, safety, which systems it runs on, and how it works with Claude Code.
---
## What is Kuchu?

A small desk pet for Claude Code. It sits above your windows and shows what your Claude Code sessions are doing, with
a card beside it for each session's steps, and it lets you answer Claude Code's permission requests from that card.

## Does Kuchu work with Claude Code in VS Code?

Yes. Kuchu follows Claude Code through its hooks, so a session in the VS Code extension shows up beside the pet just as
one in a terminal does. Kuchu sits above your windows, so you can see it while you work in the editor.

## Is Kuchu made by Anthropic?

No. Kuchu is an independent, open-source app that works with Claude Code. It is not made or endorsed by Anthropic.
Claude and Claude Code are trademarks of Anthropic.

## Is it free?

Yes. Kuchu is free and open source, under the MIT licence, with no ads and no purchases.

## What do I need?

Claude Code, installed separately and signed in, on Windows 10 (version 1809 or later), Windows 11, or macOS (in
preview).

## How do I see what Claude Code is doing without watching the terminal?

Install Kuchu and connect it to Claude Code. The pet shows whether Claude is thinking, working, waiting for you or
done, and its card shows the session's current step, for every session in VS Code, any terminal or another editor.

## Can I approve Claude Code's permission requests from a popup?

Yes. When Claude Code asks to run a command or change a file, Kuchu shows the request on its card with Allow and Deny.
Claude Code still asks in its own window too, and the first answer counts.

## Can Kuchu allow something without me?

Never. A request is allowed only by your click. If you don't answer within about 100 seconds, it goes back to Claude
Code. A timeout never allows and never denies.

## Why do I rarely see permission cards?

Claude Code's default Auto mode lets Claude decide most requests itself, so few reach you. Switch to manual mode with
Shift+Tab in Claude Code to be asked about each one.

## Does Kuchu send my code or prompts anywhere?

No. Kuchu collects nothing and sends nothing anywhere: no account, no analytics. Events go from Claude Code to Kuchu
through a pipe on your own computer, and nothing from your sessions is written to disk. See the
[privacy policy](/privacy/).

## Does it slow Claude Code down?

No. Claude Code hands each event to a tiny helper, which passes it to Kuchu and exits at once. If Kuchu is not
running, the helper gives up within 300 milliseconds and does nothing.

## Does it change my Claude Code settings?

Only when you click Connect or Disconnect, and only after showing you the exact change. It keeps a copy of the file
first, and adds or removes nothing but its own hooks.

## Does it need administrator rights?

No. Kuchu installs for your account only and is never a service.

## Does it work with several Claude Code sessions at once?

Yes. Each session has its own card. The card shows the latest, and "+N" lists the others; click one to follow it.

## Can I talk to Claude from Kuchu?

Yes. Start a conversation in any folder from your sessions list. Kuchu runs your own Claude Code for it, so Claude
answers under your own account.

## Does it work on Linux?

Not yet. Windows comes first, then macOS; Linux later.

## How do I uninstall it?

Click Disconnect in Kuchu's Settings first, then uninstall it as usual. See [Uninstalling Kuchu](/docs/uninstall/).

## Where do I report a problem or an answer from Claude?

Open an issue on [GitHub]({{REPO}}/issues). In a conversation started from Kuchu, the flag in its header opens one for
you.
