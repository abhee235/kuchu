# Kuchu's privacy policy

Kuchu collects nothing and sends nothing anywhere. It has no account, no analytics, no ads and no updater, and it
never connects to the internet itself.

## What it reads, on your computer only

- **What Claude Code tells it while you work.** Claude Code runs Kuchu's relay for each hook event (a message sent, a
  tool started or finished, a permission asked, a turn done). The relay hands the event to Kuchu on your computer,
  through a local pipe. Kuchu shows it beside the pet and keeps it in memory only.
- **Your sessions' transcripts**, which Claude Code keeps on your computer: only the name Claude Code gave a session
  and what Claude said just before a step, to show on the card.
- **Claude Code's settings.json**, to show whether Kuchu is connected, and to add or remove Kuchu's own hooks when you
  click Connect or Disconnect, after it has shown you the change. It keeps a copy of the file beside it first.

## What it keeps

- Its own settings (the pet's size and place, your choices in Settings, the folders you started conversations in), in
  its own folder on your computer.
- A copy of its relay, where Claude Code's hooks run it: in Kuchu's own folder, or, installed from the Microsoft
  Store, in the `.kuchu` folder in your home folder, only while Kuchu is connected.
- The copies of Claude Code's settings.json it makes before each change, beside the file: the last few only.

Nothing from your sessions is ever written to disk by Kuchu.

## Removing it

Click Disconnect in Kuchu's Settings before you uninstall it: Kuchu's hooks come out of Claude Code's settings.json,
and, from the Store, its relay out of your home folder. Uninstalling then removes Kuchu and its settings. The copies
of settings.json stay, as yours to keep or delete.

## Conversations started from Kuchu

When you start a conversation from Kuchu, it runs Claude Code on your computer, which talks to Anthropic as it always
does, under Anthropic's terms and privacy policy. Kuchu itself sends nothing. The flag in a conversation opens a page
on GitHub in your browser, to report an answer; nothing goes with it.

## Questions

Open an issue at https://github.com/abhee235/kuchu/issues.
