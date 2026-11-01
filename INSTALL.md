# Installing Kuchu

These steps are written for Claude Code, run on the person's own computer when they ask for Kuchu. They install it
for the person's own account only: no administrator rights, no `sudo`, never a service.

Two things are the person's, not yours:

- **Their Claude Code settings.** Do not edit `~/.claude/settings.json` (or `$CLAUDE_CONFIG_DIR/settings.json`), and do
  not add Kuchu's hooks to it in any other way. `kuchu setup` opens Kuchu's Settings, where the person clicks
  **Connect** after Kuchu has shown them the change. That click is how Kuchu gets connected.
- **What gets installed.** If a checksum does not match, stop: delete the download and tell them. Never skip the check.

Releases are on the Releases page of the repository this file is in; below, `<repo>` stands for that repository's
address (`https://github.com/<owner>/<name>`). If it has no release yet, Kuchu cannot be installed this way: say so,
and point to "Build" in the README.

## What is in a release

| File | For |
|---|---|
| `kuchu-windows-x64.zip` | Windows 10 and 11 on Intel or AMD (also runs on Arm, emulated) |
| `kuchu-windows-arm64.zip` | Windows 11 on Arm |
| `kuchu-macos-arm64.tar.gz` | macOS 11 or later on Apple silicon |
| `kuchu-macos-x64.tar.gz` | macOS 11 or later on Intel |
| `SHA256SUMS` | the SHA-256 of each file above |

A Windows zip holds `kuchu.exe` and `kuchu-relay.exe`; a macOS archive holds `Kuchu.app`, with the relay inside it.
The relay is the small program Claude Code runs for each hook event: Kuchu copies it to a fixed place of its own, and
the hooks point there.

## Windows

Run this in Bash (Claude Code's shell on Windows is Git Bash).

```bash
set -euo pipefail
case "${PROCESSOR_IDENTIFIER:-}" in ARM*) asset=kuchu-windows-arm64.zip ;; *) asset=kuchu-windows-x64.zip ;; esac
repo="<repo>"
work="$(mktemp -d)"
curl -fsSL -o "$work/$asset" "$repo/releases/latest/download/$asset"
curl -fsSL -o "$work/SHA256SUMS" "$repo/releases/latest/download/SHA256SUMS"
(cd "$work" && grep "  $asset\$" SHA256SUMS | sha256sum -c -)   # must print "<asset>: OK"

dest="$(cygpath -u "$LOCALAPPDATA")/Programs/Kuchu"
[ -x "$dest/kuchu.exe" ] && "$dest/kuchu.exe" quit || true     # an older Kuchu, if one runs
mkdir -p "$dest"
unzip -o -q "$work/$asset" -d "$dest"
rm -rf "$work"
"$dest/kuchu.exe" setup
```

Kuchu's commands write only into a pipe, as above. From PowerShell, add `| Out-String`:
`& "$env:LOCALAPPDATA\Programs\Kuchu\kuchu.exe" status | Out-String`.

## macOS

```bash
set -euo pipefail
case "$(uname -m)" in arm64) asset=kuchu-macos-arm64.tar.gz ;; *) asset=kuchu-macos-x64.tar.gz ;; esac
repo="<repo>"
work="$(mktemp -d)"
curl -fsSL -o "$work/$asset" "$repo/releases/latest/download/$asset"
curl -fsSL -o "$work/SHA256SUMS" "$repo/releases/latest/download/SHA256SUMS"
(cd "$work" && grep "  $asset\$" SHA256SUMS | shasum -a 256 -c -)   # must print "<asset>: OK"

app="$HOME/Applications/Kuchu.app"
[ -x "$app/Contents/MacOS/kuchu" ] && "$app/Contents/MacOS/kuchu" quit || true
mkdir -p "$HOME/Applications"
rm -rf "$app"
tar -xzf "$work/$asset" -C "$HOME/Applications"
rm -rf "$work"
"$app/Contents/MacOS/kuchu" setup
```

## After `kuchu setup`

`setup` puts the relay in place, adds Kuchu to the account's own list of apps that start at sign-in, and starts Kuchu
with its Settings open. It prints what it did. Tell the person, in plain words:

- Kuchu is on their screen and starts when they sign in (they can switch that off in Settings → Pet).
- To see their Claude Code sessions beside Kuchu, they click **Connect** in the Settings window. Kuchu shows the exact
  change to their `settings.json` first and keeps a copy of the file. Sessions that are already open show up after
  they are restarted.

Do not click Connect for them. `kuchu status` says how things stand at any time:

```
Kuchu 0.1.0
  Program:                 C:\Users\you\AppData\Local\Programs\Kuchu\kuchu.exe
  Running:                 yes
  Starts when you sign in: yes
  Relay:                   C:\Users\you\AppData\Local\dev.kuchu.app\bin\kuchu-relay.exe
  Claude Code:             connected (C:\Users\you\.claude\settings.json)
```

## Updating

Run the same steps again: they quit the running Kuchu, put the new one in its place, and run `setup`, which replaces
the relay. The person stays connected; nothing in `settings.json` changes.

## Uninstalling

1. Run `kuchu uninstall` (`"$dest/kuchu.exe" uninstall` on Windows, `"$app/Contents/MacOS/kuchu" uninstall` on
   macOS, with `dest` and `app` as above).
2. If it says Kuchu's hooks are still in `settings.json`, it has opened Kuchu's Settings: ask the person to click
   **Disconnect** there (Kuchu shows the change first), then run `kuchu uninstall` again. It removes nothing until
   then.
3. Once it succeeds (Kuchu quit, no longer starting at sign-in, its relay, settings and data gone), delete the program
   itself: `rm -rf "$dest"` on Windows, `rm -rf "$app"` on macOS.

The copies Kuchu kept of `settings.json` (`settings.json.kuchu-backup-…`, beside it) are the person's: leave them.

## Not signed yet

Releases are not code-signed yet; signed builds come before the first public release. Files downloaded with `curl`
as above are not marked as coming from the internet, so Windows and macOS open them without a warning; the SHA-256
check is what makes sure they are the release's own files.
