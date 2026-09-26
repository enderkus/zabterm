---
title: "Configuration"
lead: "One TOML file holds a few global settings and one or more profiles, each pointing at a Zabbix server."
---

## Location

zabterm reads `$XDG_CONFIG_HOME/zabterm/config.toml`, which is `~/.config/zabterm/config.toml` on both Linux and macOS unless you changed `XDG_CONFIG_HOME`.

```sh
zabterm init            # write a commented starter config
zabterm init --force    # overwrite an existing one
zabterm config          # print the path in use
zabterm -c ./other.toml # use a different file for this run
```

## Full example

```toml
# Profile used when -p/--profile is not given.
default_profile = "prod"

# auto, tokyo-night, catppuccin, gruvbox, nord, everforest,
# rose-pine, kanagawa, matte-black
theme = "auto"

# Seconds between API polls (minimum 2).
refresh_interval = 10

[notifications]
desktop = true
min_severity = "average"

[profiles.prod]
url = "https://zabbix.example.com"
token_cmd = "op read op://ops/zabbix-prod/token"

[profiles.staging]
url = "https://zabbix.staging.example.com"
token_env = "ZABBIX_STAGING_TOKEN"

[profiles.lab]
url = "https://10.0.0.5/zabbix"
username = "Admin"
password = "zabbix"
verify_tls = false
```

## Global settings

| Key | Default | Meaning |
|---|---|---|
| `default_profile` | first profile | Profile to use when `-p` is not given. |
| `theme` | `"auto"` | Color theme. See [Themes]({{ '/guide/themes/' | relative_url }}). |
| `refresh_interval` | `10` | Seconds between polls. Values below 2 are raised to 2. |
| `notifications.desktop` | `true` | Send native desktop notifications. In-app toasts are always on. |
| `notifications.min_severity` | `"average"` | Ignore problems below this severity for notifications. |

Severity names: `not_classified`, `information`, `warning`, `average`, `high`, `disaster`.

## Profiles

Each `[profiles.NAME]` table describes one Zabbix server.

| Key | Required | Meaning |
|---|---|---|
| `url` | yes | Frontend URL. `/api_jsonrpc.php` is added if missing. Subpaths are fine. |
| `token` | one of these | API token in plain text. |
| `token_env` | | Name of an environment variable holding the token. |
| `token_cmd` | | Shell command that prints the token. |
| `username` + `password` | | Log in with `user.login` instead of a token. |
| `verify_tls` | no, `true` | Set to `false` for self-signed certificates. |

Switch profile per run:

```sh
zabterm -p staging
zabterm check -p staging
```

## Credentials

zabterm uses the first of these that is set:

1. `ZABTERM_TOKEN` environment variable
2. `token_env`
3. `token_cmd`
4. `token`
5. `username` and `password`

### Keeping tokens out of the file

`token_cmd` runs through `sh -c` and uses whatever it prints, trimmed. Anything that can print a secret works:

```toml
token_cmd = "pass show zabbix/prod"
token_cmd = "op read op://ops/zabbix-prod/token"
token_cmd = "secret-tool lookup service zabbix"
token_cmd = "security find-generic-password -s zabbix -w"   # macOS Keychain
```

It runs once at startup, so a password manager prompt appears before the UI does.

### Username and password

zabterm opens a session with `user.login`. If the session expires while it runs, it logs in again on the next refresh. Tokens are still the better choice: they can be scoped, expire and be revoked without touching the user's password.

## Environment variables

| Variable | Effect |
|---|---|
| `ZABTERM_URL` | Adds a profile named `env` with this URL and makes it the default. No config file needed. |
| `ZABTERM_TOKEN` | Overrides the token of whichever profile is active. |
| `ZABTERM_TRUECOLOR` | `1` forces 24-bit color, `0` forces 256 colors. |
| `XDG_CONFIG_HOME` | Changes where the config is looked up. |

## Command line

```text
zabterm [OPTIONS] [COMMAND]

Commands:
  init    Write a starter config to ~/.config/zabterm/config.toml
  check   Verify that the active profile can reach and authenticate to Zabbix
  config  Print the config file path

Options:
  -p, --profile <PROFILE>  Profile from the config file to use
  -c, --config <CONFIG>    Path to an alternative config file
  -t, --theme <THEME>      Override the theme for this run
  -h, --help               Print help
  -V, --version            Print version
```
