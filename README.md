# Kuchu

**To install Kuchu, ask Claude Code to follow [INSTALL.md](INSTALL.md).**

A small desk pet for Claude Code. It sits above your windows and shows what your sessions are doing: it thinks while
Claude reads your message, works while tools run, waves when a session needs you and hops when one is done. Its card
says which project and what is happening now, and a permission request can be answered from it.

Windows and macOS first; Linux later. Work in progress: see [PLAN.md](PLAN.md).

## What it does

- **Every session, on its own card.** Beside the pet: the project and what Claude is doing now (or the answer's first
  line once done, for a few seconds), in one line or a few. With several at work, "+N" lists the others and switches
  the card to the one you pick. A click on the pet lists every session.
- **Permission requests on the pet.** Allow, Deny, or hand it back to Claude Code. Nothing is allowed without a click.
- **Talk to Claude from Kuchu.** Start a conversation in a folder from the sessions list, answer it, stop it, or take
  it on in a terminal. Sessions started in a terminal or an editor are followed, not typed into: Claude Code takes
  their messages only there.
- **A life of its own.** With no work it stretches, yawns, looks around, taps a foot, flips its screen, whistles or
  takes a short walk, and naps after a few quiet minutes. It walks when you drag it and likes being petted.
- **Its own Settings.** Size, see-through under the pointer, sounds, what shows beside the pet, what a click opens,
  and connecting to Claude Code: the change to `settings.json` is shown first, and a copy of the file is kept.
- **Out of the way.** It never takes the focus, and clicks go through everywhere but the pet and its cards. The tray
  icon and Ctrl+Alt+K always bring it back.
- **Yours alone.** Kuchu is installed for your account only and starts when you sign in, from your account's own list
  of startup apps. It is never a service and never runs with administrator rights.

## How it works

Claude Code runs `kuchu-relay`, a small native program, for each hook event. The relay hands the event to Kuchu and
exits at once; it never holds Claude Code up, and when Kuchu is not running it does nothing. What Claude Code sends
is in [docs/hooks.md](docs/hooks.md).

## Build

Requirements: Rust (stable) and Node 20+.

```
cargo build --release -p kuchu-relay
cd app && npm ci && npx tauri build --no-bundle
```

Both end up in `target/release/`. `target/release/kuchu setup` then does what an install does (see INSTALL.md), and
`kuchu status` says how Kuchu stands.

## Licence

The code is under the [MIT licence](LICENSE). The name Kuchu, the character and the sounds are not: see
[LICENSE-ASSETS.md](LICENSE-ASSETS.md).
