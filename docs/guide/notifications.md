---
title: "Notifications"
lead: "zabterm compares every poll with the previous one and tells you what changed."
---

## What triggers one

| Event | Toast | Desktop |
|---|---|---|
| New problem at or above `min_severity` | ▲ in the severity color | yes |
| Problem at or above `min_severity` resolved | ✓ Resolved | yes |
| Host becomes unreachable | ✕ Unreachable, with Zabbix's error | yes, urgent |
| Host reachable again | ✓ Back online | yes |
| Acknowledge done, theme change, API error | toast only | no |

Suppressed problems (for example during maintenance) never notify. Nothing fires for what was already there when zabterm started, only for changes after that. To avoid a flood after an outage, at most three new and three resolved problems are announced per poll.

Toasts stack in the top right corner and disappear after six seconds.

## Settings

```toml
[notifications]
desktop = true            # false keeps toasts but skips desktop notifications
min_severity = "average"  # not_classified, information, warning, average, high, disaster
```

## Linux

zabterm runs `notify-send -a zabterm`. High and Disaster problems and unreachable hosts are sent with urgency `critical`, everything else `normal`. On Omarchy, mako shows them. Any notification daemon that implements the freedesktop spec works (dunst, swaync, GNOME, KDE).

If `notify-send` is missing, the toast still appears; install `libnotify` to get desktop notifications.

## macOS

zabterm uses `osascript` to post to Notification Center. macOS lists these under **Script Editor**, so the first time you may need to allow notifications for it in **System Settings → Notifications**.

## Try it

A quick way to see notifications work is to stop one of your agents for a minute or two. You'll get an **Unreachable** toast once Zabbix marks the host down, then the agent-unavailable problem, and **Back online** plus **Resolved** when you start it again.
