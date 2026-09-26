---
title: "Troubleshooting"
lead: "The common problems and how to fix them. Start with zabterm check; it prints the config, profile and endpoint it uses and the exact error."
---

## "no profiles configured"

There is no config file and `ZABTERM_URL` is not set. Run `zabterm init`, then edit the file it prints.

## "profile 'x' has no credentials"

The profile has a `url` but no `token`, `token_env`, `token_cmd` or `username`/`password`. The placeholder token from `zabterm init` counts as missing.

## "Not authorized" or "Session terminated"

- The token was deleted, expired or disabled. Create a new one under **User settings → API tokens**.
- The user behind the token was disabled or lost API access in its role.
- With `token_env`, check the variable is exported in the shell that starts zabterm.

## "Connection refused", DNS or timeout errors

`url` must be the address of the **frontend**, the one you open in a browser, not the Zabbix server port 10051. Check it with:

```sh
curl -s https://zabbix.example.com/api_jsonrpc.php \
  -H 'Content-Type: application/json-rpc' \
  -d '{"jsonrpc":"2.0","method":"apiinfo.version","params":{},"id":1}'
```

## Certificate errors

For self-signed or internal CAs, either add the CA to your system trust store or set `verify_tls = false` on that profile.

## CPU, memory or disk show "-"

The host has no items with the [keys zabterm reads]({{ '/guide/internals/' | relative_url }}#item-keys), for example SNMP devices or hosts on other templates. They still appear in lists and problems, and the host page shows all their items.

## Colors look wrong

- Apple Terminal: zabterm falls back to 256 colors automatically. Try `ZABTERM_TRUECOLOR=1` if your version supports 24-bit color, or use Ghostty, iTerm2, kitty or Alacritty.
- Inside tmux, enable true color with `set -as terminal-features ",*:RGB"`.

## No desktop notifications

- `notifications.desktop` must be `true` and the problem at or above `min_severity`.
- Linux: `notify-send` must exist (package `libnotify`) and a notification daemon must be running.
- macOS: allow notifications for **Script Editor** in System Settings.

## No icon in the launcher

Run the install script again; it installs the icon and rewrites the launcher entry. Then close and reopen the launcher.

## Still stuck?

[Open an issue]({{ site.github_url }}/issues) with the output of `zabterm check` (the token is never printed) and your Zabbix version.
