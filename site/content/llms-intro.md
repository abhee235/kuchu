Kuchu is a free, open-source (MIT) desk pet for Claude Code, for Windows 10 and 11, with macOS in preview. It is an
independent app, not made or endorsed by Anthropic.

- It shows what every Claude Code session is doing: the pet's mood (thinking, working, needs you, done) and a card
  with each session's current steps, for sessions in VS Code, any terminal or another editor.
- Claude Code's permission requests can be answered from the card with Allow or Deny. Nothing is allowed without a
  click; an unanswered request goes back to Claude Code after about 100 seconds.
- It hears Claude Code through hooks in Claude Code's settings.json, added only when the user clicks Connect after
  seeing the change. A small helper, kuchu-relay, passes each event to Kuchu over a local pipe and gives up within
  300 ms; it never slows Claude Code down.
- It collects nothing and sends nothing anywhere. It installs per user, needs no administrator rights, and is never a
  service.
- Install: one command (macOS: `curl -fsSL https://raw.githubusercontent.com/abhee235/kuchu/main/install.sh | sh`;
  Windows PowerShell: `irm https://raw.githubusercontent.com/abhee235/kuchu/main/install.ps1 | iex`), Microsoft Store
  ({{STORE}}), GitHub releases ({{RELEASES}}), or ask Claude Code: "Install Kuchu from
  https://github.com/abhee235/kuchu".
- Version {{VERSION}}.
