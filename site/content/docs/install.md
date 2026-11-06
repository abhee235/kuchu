---
title: Installing Kuchu
nav: Install
order: 2
description: Install Kuchu from the Microsoft Store, with the Windows installer, on macOS, or by asking Claude Code. Kuchu installs for your account only and needs no administrator rights.
---
Kuchu installs for your account only. It needs no administrator rights and is never a Windows service. It does need
Claude Code, installed separately.

## Windows: the Microsoft Store

The easiest way on Windows 10 (version 1809 or later) and Windows 11.

[Get Kuchu from the Microsoft Store]({{STORE}})

The Store version is signed by Microsoft, so Windows opens it without a warning, and it updates itself through the
Store: Windows closes Kuchu for a moment and starts the new one. It is for x64 PCs for now; on an Arm PC, use the
installer below.

## Windows: the installer

From the [latest release]({{RELEASES}}), download `kuchu-windows-x64-setup.exe`, or
`kuchu-windows-arm64-setup.exe` on an Arm PC, and open it.

The installer is not code-signed yet, so Windows says "Windows protected your PC". Click **More info**, then
**Run anyway**. To check the file first, compare its SHA-256 with the release's `SHA256SUMS`:

```
Get-FileHash .\kuchu-windows-x64-setup.exe
```

Each release's files are also attested by GitHub: `gh attestation verify <file> -R abhee235/kuchu` confirms that the
release workflow built them.

## macOS (preview)

From the [latest release]({{RELEASES}}), download `kuchu-macos-arm64.tar.gz` (or `kuchu-macos-x64.tar.gz` on an Intel
Mac), open it, and drag `Kuchu.app` into Applications. The first time, macOS refuses to open it: in System Settings,
Privacy & Security, click **Open Anyway**.

The macOS version has not been tried on many Macs yet. If something is off, please
[open an issue]({{REPO}}/issues).

## Ask Claude Code

Say this in any Claude Code session:

```
Install Kuchu from https://github.com/abhee235/kuchu
```

Claude Code downloads the latest release, checks its checksum (and its attestation, if GitHub's CLI is signed in), and
installs Kuchu for your account. It stops if a check fails. It never touches your Claude Code settings: connecting is
your click in Kuchu's Settings.

## From the source code

Kuchu is open source. To build it yourself you need Rust and Node 20 or later; the
[README]({{REPO}}#build) has the steps.

## Next

The first time Kuchu starts, it opens its Settings. [Connect it to Claude Code](/docs/connect/).
