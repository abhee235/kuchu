---
title: Uninstalling Kuchu
head: Uninstall Kuchu and remove its Claude Code hooks
nav: Uninstall
order: 8
description: How to remove Kuchu completely, whichever way you installed it. Disconnect first, so Claude Code stops running Kuchu's hooks.
---
## First, disconnect

In Kuchu's Settings, under **Claude Code**, click **Disconnect**, and confirm. This takes Kuchu's hooks out of Claude
Code's `settings.json`, so Claude Code stops running Kuchu's helper. From the Microsoft Store, it also removes the
helper from your home folder.

## Then remove Kuchu

- **From the Microsoft Store**: right-click Kuchu in the Start menu and choose **Uninstall**, or use Windows'
  Settings, **Apps**. Windows removes Kuchu and its settings.
- **With the Windows installer**: Windows' Settings, **Apps**, Kuchu, **Uninstall**. Tick **Delete the application
  data** to remove its settings too. If you have not disconnected, Kuchu asks you to first.
- **On macOS, or unzipped by hand**: run `kuchu uninstall`. It removes Kuchu's startup entry, its helper, its settings
  and its data, then tells you where the program itself is, to delete it. It removes nothing until you have
  disconnected.

## What stays

The copies Kuchu kept of Claude Code's `settings.json` (`settings.json.kuchu-backup-…`, beside the file) are yours:
keep or delete them.

## Uninstalled without disconnecting?

Kuchu's hooks stay in Claude Code's `settings.json`. From the Microsoft Store, the helper stays too, in the `.kuchu`
folder in your home folder, and does nothing while Kuchu is gone. To tidy up, either install Kuchu again and click
Disconnect, or remove the hooks whose command runs `kuchu-relay` from `settings.json` yourself, and delete the
`.kuchu` folder.

If the helper itself is gone, Claude Code shows a hook error on each event until the hooks are removed.
