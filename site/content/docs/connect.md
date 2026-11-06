---
title: Connecting Kuchu to Claude Code
nav: Connect to Claude Code
order: 3
description: Kuchu hears Claude Code through hooks in Claude Code's settings.json. Connect adds them after showing you the change, keeps a copy of the file first, and Disconnect takes them out.
---
Kuchu hears what Claude Code is doing through Claude Code's **hooks**: a few lines in Claude Code's `settings.json`
that run Kuchu's helper on each event. Kuchu adds them only when you click Connect, after showing you exactly what
changes.

## Connect

1. Open Kuchu's Settings. The first time Kuchu starts, they open by themselves; later, use the tray icon,
   **Settings…**.
2. Go to **Claude Code** and click **Connect**.
3. Kuchu shows the lines it will add to `settings.json`, and where it keeps a copy of the file as it is now. Nothing
   else of yours changes.
4. Click **Connect** again. Settings say **Connected**, and Claude Code sessions show up beside Kuchu at once.

Kuchu writes the file only if it has not changed since you saw the change. It keeps the last five copies, beside the
file, named `settings.json.kuchu-backup-…`.

## What the change is

For each event Kuchu follows, a hook that runs Kuchu's helper, `kuchu-relay`, with the event's name. On Windows the
command is a quoted path with forward slashes, because Claude Code runs hooks through Git Bash:

```
"C:/Users/you/AppData/Local/dev.kuchu.app/bin/kuchu-relay.exe" Stop
```

From the Microsoft Store, the helper lives in `.kuchu\bin` in your home folder instead, where Claude Code can find
it. [How it works](/docs/how-it-works/) says what the helper does and doesn't do.

## Disconnect

In Settings, under **Claude Code**, click **Disconnect**. Kuchu shows the lines it will take out, then removes only its
own hooks. From the Microsoft Store, Disconnect also removes the helper from your home folder. Disconnect before you
uninstall Kuchu; see [Uninstalling Kuchu](/docs/uninstall/).

## What Settings may say

- **Not connected yet**: Kuchu's hooks are not in `settings.json`. Click Connect.
- **Connected**: everything is in place.
- **Connected, but out of date**: some of Kuchu's hooks are missing, or point at an older place. Click **Update**,
  which shows the change first, like Connect.
- **Claude Code's settings could not be read**: the file is not valid JSON. Kuchu will not touch a file it cannot read;
  fix the file, and Settings check again.
- **disableAllHooks is on in this file**: Claude Code runs no hooks at all, Kuchu's included, until you remove that
  setting.

Kuchu uses the settings in `CLAUDE_CONFIG_DIR` if you have set it, and `~/.claude/settings.json` otherwise.
