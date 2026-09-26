---
title: "Themes"
lead: "zabterm paints with your palette. On Omarchy it simply matches whatever theme you have on."
---

## Choosing a theme

```toml
theme = "auto"
```

Or for a single run:

```sh
zabterm -t gruvbox
```

Inside the app, <kbd>t</kbd> cycles through every theme and shows its name in a toast. That choice lasts until you quit; set `theme` in the config to keep it.

## auto

`auto` (or `omarchy`) looks for the active Omarchy theme in `~/.config/omarchy/current/theme` and builds the palette from:

1. `colors.toml` (newer Omarchy releases: `background`, `foreground`, `accent`, `color0` to `color15`), or
2. `alacritty.toml` (the `[colors.primary]`, `[colors.normal]` and `[colors.bright]` tables).

It checks that directory every couple of seconds. Switch themes with Omarchy and zabterm repaints itself, with a toast saying which theme it picked up.

Without Omarchy, `auto` uses tokyo-night.

## Built-in themes

| Name | |
|---|---|
| `tokyo-night` | Omarchy's default |
| `catppuccin` | Mocha |
| `gruvbox` | Dark |
| `nord` | |
| `everforest` | Dark |
| `rose-pine` | |
| `kanagawa` | |
| `matte-black` | |

## How colors are used

Every theme is reduced to the same handful of roles, so all of them read the same way:

| Role | Used for |
|---|---|
| accent | Active tab, selection marker, keys, first chart series |
| green | Available hosts, healthy meters, "all clear" |
| yellow → red | Meter levels at 70% and 90% |
| blue, yellow, orange, red | Severity badges: <span class="sev info">INFO</span> <span class="sev warn">WARN</span> <span class="sev avg">AVG</span> <span class="sev high">HIGH</span> <span class="sev dstr">DSTR</span> |

The background is never painted, so terminal transparency and blur keep working.

## Color depth

zabterm uses 24-bit color. When it can tell the terminal can't do that (Apple Terminal without `COLORTERM`), it maps every color to the nearest of the 256 standard ones. Force either mode:

```sh
ZABTERM_TRUECOLOR=1 zabterm   # always 24-bit
ZABTERM_TRUECOLOR=0 zabterm   # always 256 colors
```
