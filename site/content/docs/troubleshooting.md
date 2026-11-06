---
title: Troubleshooting Kuchu
nav: Troubleshooting
order: 9
description: Fixes for common problems with Kuchu and Claude Code: a hook error on every message, the pet not reacting, no permission cards, Windows warnings, and more.
---
## Claude Code shows "hook error" on every message

Claude Code prints something like *UserPromptSubmit hook error: Failed with non-blocking status code* under each
message, and the pet does not move.

- **Kuchu 0.1.5 or older, on a PC without Visual C++**: Kuchu's helper could not start there. Update to Kuchu 0.1.6 or
  later: the Microsoft Store updates by itself; otherwise install the [latest release]({{RELEASES}}). Start Kuchu once
  after updating, so it puts the new helper in place.
- **Kuchu was uninstalled without disconnecting**, and its helper is gone: Claude Code still runs the hooks. Install
  Kuchu again and click **Disconnect**, or remove the hooks whose command runs `kuchu-relay` from Claude Code's
  `settings.json`. See [Uninstalling Kuchu](/docs/uninstall/).

## The pet does not react to Claude Code

1. **Is Kuchu running?** Its icon is in the tray (the menu bar on a Mac). If the pet is hidden, choose **Show Kuchu**
   there.
2. **Is it connected?** In Settings, **Claude Code** should say **Connected**. If it says **out of date**, click
   **Update**.
3. **Are hooks switched off?** If Settings say *disableAllHooks is on*, Claude Code runs no hooks at all.
4. **Is the session using the same settings?** A session started with a different `CLAUDE_CONFIG_DIR`, or with
   `--setting-sources` that leave out your user settings, does not run Kuchu's hooks.
5. **Is another Kuchu running?** Only one Kuchu can listen per account. Quit the others from their tray icons.

## No permission cards

Claude Code's default **Auto** mode decides most requests itself, so few reach you. Switch to **manual** with
**Shift+Tab** in Claude Code to be asked about each one. Also check that **Answer them on Kuchu** is on in Settings,
**Permission requests**. See [Permission requests](/docs/permissions/).

## "Windows protected your PC"

The installer from GitHub is not code-signed yet. Click **More info**, then **Run anyway**; or install Kuchu from the
[Microsoft Store]({{STORE}}), which Microsoft signs. Compare the file's SHA-256 with the release's `SHA256SUMS` first
if you like.

## macOS will not open Kuchu

The first time, macOS refuses an app from outside the App Store. In System Settings, **Privacy & Security**, click
**Open Anyway**.

## Kuchu disappeared after a Store update

Kuchu 0.1.5 did not ask Windows to start it again after an update. Start it once from the Start menu; from 0.1.6 on, it
comes back by itself.

## The list of other sessions was cut off

Before 0.1.6, with several sessions and the pet high on the screen, the "+N" list could scroll with its last card cut
off. Update to 0.1.6 or later: the list now makes room for itself.

## Something else

Please [open an issue]({{REPO}}/issues) with what you see, your system, and Kuchu's version (in Settings, beside its
name).
