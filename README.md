# zabterm

[![Built for Omarchy: App](https://raw.githubusercontent.com/tcballard/omarchy-badges/75975e5b5bf75e7ede3764bcd2950046f7abfe2c/badges/v1/omarchy-app.svg)](https://github.com/tcballard/omarchy-badges)
[![CI](https://github.com/enderkus/zabterm/actions/workflows/ci.yml/badge.svg)](https://github.com/enderkus/zabterm/actions/workflows/ci.yml)

A fast, keyboard-driven terminal UI for Zabbix, for Linux and macOS. Built in Rust with
[ratatui](https://ratatui.rs), styled to feel at home on
[Omarchy](https://omarchy.org): it picks up your current Omarchy theme and
follows along live when you switch themes.

```
 ◆ zabterm   1 Dashboard   2 Hosts 3   3 Problems 0     ● live  ·  local  ·  Zabbix 8.0.0  ·  33ms  ·  14:49:35

╭ Hosts ─────────────╮╭ Availability ───────╮╭ Problems ──────────╮╭ Avg CPU ────────────╮╭ Avg Memory ────────╮
│         ▀▀█        ││     ▀█  █▀█ █▀█     ││         █▀█        ││       ▀▀█ ▀▀█       ││       ▀▀█ █▀▀      │
│          ▀█        ││      █  █ █ █ █     ││         █ █        ││        ▀█ █▀▀       ││       █▀▀ █▀█      │
│         ▀▀▀        ││     ▀▀▀ ▀▀▀ ▀▀▀ %   ││         ▀▀▀        ││       ▀▀▀ ▀▀▀ %     ││       ▀▀▀ ▀▀▀ %    │
│                    ││                     ││                    ││                     ││                    │
│  ● 3 up   ● 0 down ││  agents reachable   ││      all clear     ││ peak test-agen… 35% ││ peak Zabbix s… 30% │
╰────────────────────╯╰─────────────────────╯╰────────────────────╯╰─────────────────────╯╰────────────────────╯
╭ Problems by severity ────────────────────────────────────────────────────────────────────────────────────────╮
│━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━│
│■ Disaster 0     ■ High 0     ■ Average 0     ■ Warning 0     ■ Information 0     ■ Not classified 0          │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
╭ Fleet CPU · last 30m ─────────────────────────────────────────────╮╭ Latest problems ────────────────────────╮
│50% │                                           ┌─────────────────┐││                                         │
│    │                                           │test-agent-2 35% │││                                         │
│    │                                           │Zabbix server 35%│││                                         │
│    │                                           │test-agent-1 26% │││                                         │
│    │                                           └─────────────────┘││                                         │
│    │                                                            ⢀ ││                    ✓                    │
│25% │                                                            ⡜ ││                All clear                │
│    │                                                           ⢀⡇ ││           no active problems            │
│    │                                                           ⢸⠁ ││                                         │
│    │                                                           ⡎  ││                                         │
│    │                                                          ⢀⠇  ││                                         │
│0%  │ ⠠⠤⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠚   ││                                         │
│    └──────────────────────────────────────────────────────────────││                                         │
│14:19                            14:34                          now││                                         │
╰ m show memory ────────────────────────────────────────────────────╯╰─────────────────────────────────────────╯
╭ Hosts ───────────────────────────────────────────────────────────────────────────────────────────────────────╮
│   Host                                                   CPU · 30m             Memory            Problems    │
│●  Zabbix server                                          ▁▁▁▁▁▁▁▁▁▁▁▁▂█   35%  ━━━───────   30%  ✓           │
│●  test-agent-1                                           ▁▂▂▂▂▂▂▂▂▁▂▂▂█   26%  ━━────────   24%  ✓           │
│●  test-agent-2                                           ▁▁▁▁▁▁▁▁▁▁▁▁▂█   35%  ━━━───────   26%  ✓           │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
 j/k select   ⏎ open   m cpu/mem   tab next   t theme   o web   ? help   q quit       updated 3s ago  every 10s
```

## Features

- **Dashboard**: big-number tiles (hosts, availability, problems, fleet CPU
  and memory), a severity distribution bar, a 30 minute fleet CPU/memory
  chart, the latest problems feed and a host table with inline sparklines.
- **Hosts**: every host with availability, CPU/memory meters, disk, load,
  uptime and per-severity problem counters. Filter with `/`, cycle sort with
  `s`, and get a live CPU/memory preview of the selected host.
- **Host page**: braille charts for CPU, memory, network in/out and load over
  15m, 1h, 6h, 24h or 7d; filesystem usage; the host's active problems; and a
  searchable **Latest data** tab with every item and its last value.
- **Problems**: severity badges, age, tags, a details pane, a minimum
  severity filter and hide-acknowledged toggle. Press `a` to acknowledge with
  a message, and close the problem too if the trigger allows it.
- **Notifications**: new problems, resolved problems and hosts becoming
  unreachable or coming back online pop up as in-app toasts and as desktop
  notifications (`notify-send` on Linux, Notification Center on macOS).
- **Themes**: `auto` reads `~/.config/omarchy/current/theme` and live-reloads.
  Built-ins: tokyo-night, catppuccin, gruvbox, nord, everforest, rose-pine,
  kanagawa, matte-black. Press `t` to cycle.
- **Profiles**: keep prod, staging and lab in one config and pick one with
  `-p`. Secrets can come from an env var or any command (`pass`, `op`, ...).
- **Open in browser**: `o` jumps to the same host or problem in the Zabbix
  frontend.

Works with the Zabbix 7.x and 8.0 API (Bearer token auth, `hostgroups`).

## Install

**Omarchy / Linux / macOS**, prebuilt binary into `~/.local/bin` (on Linux
this also adds zabterm to the app launcher):

```bash
curl -fsSL https://raw.githubusercontent.com/enderkus/zabterm/main/install.sh | sh
```

**Homebrew** (macOS and Linux):

```bash
brew install enderkus/tap/zabterm
```

**From source** (any platform with a Rust toolchain from
[rustup.rs](https://rustup.rs)):

```bash
cargo install --git https://github.com/enderkus/zabterm
```

Prebuilt binaries for Linux (x86_64, aarch64) and macOS (Apple Silicon) are
attached to each [release](https://github.com/enderkus/zabterm/releases).

### Platform notes

| | Linux | macOS |
|---|---|---|
| Config | `~/.config/zabterm/config.toml` | `~/.config/zabterm/config.toml` |
| Desktop notifications | `notify-send` (mako, dunst, ...) | Notification Center via `osascript` |
| Open in browser (`o`) | `xdg-open` | `open` |
| Theme `auto` | follows Omarchy if present | tokyo-night |
| Colors | 24-bit | 24-bit; Apple Terminal falls back to 256 colors |

Force color depth with `ZABTERM_TRUECOLOR=1` or `ZABTERM_TRUECOLOR=0`.

## Configure

```bash
zabterm init      # writes ~/.config/zabterm/config.toml (mode 600)
zabterm check     # verifies URL + credentials, prints host/problem counts
zabterm           # go
```

The config lives at `$XDG_CONFIG_HOME/zabterm/config.toml` (default
`~/.config/zabterm/config.toml`):

```toml
default_profile = "local"
theme = "auto"            # follows Omarchy, falls back to tokyo-night
refresh_interval = 10     # seconds

[notifications]
desktop = true
min_severity = "average"  # not_classified, information, warning, average, high, disaster

[profiles.local]
url = "http://localhost:8080"
token = "..."             # Zabbix: User settings -> API tokens

[profiles.prod]
url = "https://zabbix.example.com"
token_cmd = "op read op://ops/zabbix-prod/token"
# token_env = "ZABBIX_TOKEN"
# username = "Admin"
# password = "zabbix"
# verify_tls = false
```

Credentials are resolved in this order: `ZABTERM_TOKEN`, `token_env`,
`token_cmd`, `token`, then `username`/`password` (opens a session with
`user.login`). Setting `ZABTERM_URL` and `ZABTERM_TOKEN` is enough to run without
any config file.

## Keys

| Key | Action |
|-----|--------|
| `1` `2` `3` / `tab` | Dashboard, Hosts, Problems |
| `j` `k` `↑` `↓` `g` `G` `ctrl-d` `ctrl-u` | Move |
| `⏎` | Open host |
| `/` | Filter (hosts, problems, latest data) |
| `s` | Sort hosts / raise min severity |
| `+` `-` | Min severity (problems) |
| `h` | Hide acknowledged (problems) |
| `a` | Acknowledge problem |
| `[` `]` | Chart time range (host page) |
| `tab` / `i` | Graphs / Latest data (host page) |
| `m` | Dashboard chart: CPU / memory |
| `t` | Cycle theme |
| `o` | Open in Zabbix web |
| `r` | Refresh now |
| `?` | Help |
| `q` | Quit |

## Omarchy

The install script already adds zabterm to the app launcher. You can also
bind it to a key in `~/.config/hypr/bindings.conf`, using whichever
terminal you run:

```
bind = SUPER SHIFT, Z, exec, alacritty --class zabterm -e zabterm
```

## How it works

A background task owns the API client and polls on `refresh_interval`:
`host.get`, `problem.get`, `trigger.get` (to map problems to hosts) and two
`item.get` calls for the handful of keys the overview needs
(`system.cpu.util`, `vm.memory.util`, `system.cpu.load[all,avg1]`,
`system.uptime`, `net.if.*`, `vfs.fs.dependent.size[*,pused]`). Sparklines
are seeded once from `history.get` and then extended from `lastvalue`, so
steady-state polling does not re-read history. The host page reads
`history.get` for its chosen range. The UI thread never waits on the
network.
