---
title: "Omarchy"
lead: "zabterm was built with Omarchy in mind: it looks like the rest of your desktop, launches from Walker and talks through mako."
---

[![Built for Omarchy: App](https://raw.githubusercontent.com/tcballard/omarchy-badges/75975e5b5bf75e7ede3764bcd2950046f7abfe2c/badges/v1/omarchy-app.svg)](https://github.com/tcballard/omarchy-badges)

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/enderkus/zabterm/main/install.sh | sh
```

This puts the binary in `~/.local/bin`, adds a launcher entry with the zabterm icon and writes a starter config. Open Walker and type `zabterm`, or run it from any terminal.

If the icon does not show up after an upgrade from an older version, run the install script again; it rewrites the launcher entry.

## Theme

Keep the default:

```toml
theme = "auto"
```

zabterm reads the colors of your current Omarchy theme and follows along when you switch, no restart needed. See [Themes]({{ '/guide/themes/' | relative_url }}) for how it picks them up.

## Notifications

With `desktop = true` (the default), problems and host outages go to `notify-send`, so they appear as regular mako notifications and respect your do-not-disturb settings. See [Notifications]({{ '/guide/notifications/' | relative_url }}).

## A key binding

Add a line to `~/.config/hypr/bindings.conf`, using the terminal you run:

```ini
bind = SUPER SHIFT, Z, exec, alacritty --class zabterm -e zabterm
```

The `--class` gives the window its own app id, handy for window rules. For example, to open it floating and centered:

```ini
windowrule = float, class:^(zabterm)$
windowrule = size 80% 80%, class:^(zabterm)$
windowrule = center, class:^(zabterm)$
```

Adjust the syntax to your Hyprland version if needed.

## Secrets

If you keep secrets in a password manager, point `token_cmd` at it instead of pasting the token into the config:

```toml
[profiles.prod]
url = "https://zabbix.example.com"
token_cmd = "op read op://ops/zabbix-prod/token"
```
