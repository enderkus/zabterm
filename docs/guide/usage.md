---
title: "Views and keys"
lead: "Three views and a host page, all driven from the keyboard. Press ? inside zabterm for the same list."
---

## Global keys

| Key | Action |
|---|---|
| <kbd>1</kbd> <kbd>2</kbd> <kbd>3</kbd> | Dashboard, Hosts, Problems |
| <kbd>tab</kbd> <kbd>shift-tab</kbd> | Next / previous view |
| <kbd>r</kbd> | Refresh now |
| <kbd>t</kbd> | Cycle theme |
| <kbd>o</kbd> | Open the current page in the Zabbix frontend |
| <kbd>?</kbd> | Help |
| <kbd>q</kbd> <kbd>ctrl-c</kbd> | Quit |

## Lists

| Key | Action |
|---|---|
| <kbd>j</kbd> <kbd>k</kbd> <kbd>↓</kbd> <kbd>↑</kbd> | Move |
| <kbd>g</kbd> <kbd>G</kbd> <kbd>home</kbd> <kbd>end</kbd> | Top / bottom |
| <kbd>ctrl-d</kbd> <kbd>ctrl-u</kbd> <kbd>pgdn</kbd> <kbd>pgup</kbd> | Page down / up |
| <kbd>/</kbd> | Filter. Type, then <kbd>⏎</kbd> to keep it or <kbd>esc</kbd> to clear |
| <kbd>⏎</kbd> <kbd>l</kbd> | Open the selected host |

## Dashboard

![Dashboard]({{ '/assets/screenshots/dashboard.png' | relative_url }})

- **Tiles.** Hosts (up / down), availability of enabled hosts, active problems (unacknowledged count and worst severity), average CPU and memory with the busiest host. Unreachable hosts are left out of the averages because their values are stale.
- **Problems by severity.** A stacked bar of active, unsuppressed problems.
- **Fleet chart.** The six busiest hosts over the last 30 minutes. <kbd>m</kbd> switches between CPU and memory.
- **Latest problems.** Newest first, with age and a ✓ when acknowledged.
- **Hosts.** Same order as the Hosts view. <kbd>j</kbd> <kbd>k</kbd> select, <kbd>⏎</kbd> opens the host.

## Hosts

![Hosts]({{ '/assets/screenshots/hosts.png' | relative_url }})

| Key | Action |
|---|---|
| <kbd>s</kbd> | Cycle sort: problems, CPU, memory, name |
| <kbd>/</kbd> | Filter by name, address or host group |
| <kbd>⏎</kbd> | Open host page |
| <kbd>o</kbd> | Open the host's latest data in the frontend |

Status markers: <span style="color:var(--green)">●</span> available, <span style="color:var(--red)">●</span> unreachable (name in red, values muted because they are stale), <span style="color:var(--muted)">○</span> unknown, <span style="color:var(--accent)">◆</span> in maintenance, <span class="muted">✕</span> disabled.

The problems column shows one colored counter per severity, or a ✓ when the host is clean.

## Host page

| Key | Action |
|---|---|
| <kbd>[</kbd> <kbd>]</kbd> | Time range: 15m, 1h, 6h, 24h, 7d |
| <kbd>tab</kbd> <kbd>i</kbd> | Graphs / Latest data |
| <kbd>/</kbd> | Filter items by name or key (Latest data) |
| <kbd>esc</kbd> <kbd>h</kbd> <kbd>backspace</kbd> | Back to the list you came from |

**Graphs** shows CPU, memory, network in and out for the busiest interface, and 1 minute load, read from history for the chosen range. The side panel lists filesystems and the host's active problems.

**Latest data** lists every item on the host with its key, last value formatted with its units, and when it was last updated.

The page refreshes on every poll, like the rest of the app.

## Problems

![Problems]({{ '/assets/screenshots/problems.png' | relative_url }})

| Key | Action |
|---|---|
| <kbd>a</kbd> | Acknowledge the selected problem |
| <kbd>+</kbd> <kbd>s</kbd> / <kbd>-</kbd> | Raise / lower the minimum severity |
| <kbd>h</kbd> | Hide or show acknowledged problems |
| <kbd>/</kbd> | Filter by problem name, host or tag |
| <kbd>⏎</kbd> | Open the problem's host |
| <kbd>o</kbd> | Open the host's problems in the frontend |

Problems are sorted by severity, then newest first. Acknowledged and suppressed problems are dimmed.

### Acknowledging

<kbd>a</kbd> opens a small dialog. Type an optional message and press <kbd>⏎</kbd>. If the trigger allows manual close, <kbd>tab</kbd> toggles **close problem too**. <kbd>esc</kbd> cancels. The list refreshes right after.

## Opening the frontend

<kbd>o</kbd> opens the matching page with `xdg-open` on Linux or `open` on macOS:

| Where you are | Opens |
|---|---|
| Dashboard | Dashboards |
| Hosts, host page | Latest data for that host |
| Problems | Problems filtered to that host |
