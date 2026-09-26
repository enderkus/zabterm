---
title: "Quick start"
lead: "From zero to a live dashboard in about two minutes."
---

## 1. Create an API token in Zabbix

In the Zabbix frontend open **User settings → API tokens → Create API token**, give it a name such as `zabterm`, optionally an expiry date, and copy the token it shows you. You only see it once.

The token acts as the user it belongs to, so zabterm sees exactly the hosts that user can see. Acknowledging problems needs a user role that allows it.

> No token? Username and password work too, see [Configuration]({{ '/guide/configuration/' | relative_url }}#credentials).

## 2. Write the config

```sh
zabterm init
```

This creates `~/.config/zabterm/config.toml` with permissions `600`. Open it and set your URL and token:

```toml
default_profile = "local"

[profiles.local]
url = "https://zabbix.example.com"
token = "paste-the-token-here"
```

`url` is the address of your Zabbix frontend, the same one you open in a browser. zabterm adds `/api_jsonrpc.php` itself, and subpaths like `https://example.com/zabbix` work.

## 3. Check the connection

```sh
zabterm check
```

```text
  config   /home/you/.config/zabterm/config.toml
  profile  local
  endpoint https://zabbix.example.com/api_jsonrpc.php
✓ Zabbix 8.0.0 · 22 hosts · 9 active problems
```

If it fails, the message says why. The [troubleshooting]({{ '/guide/troubleshooting/' | relative_url }}) page covers the usual suspects.

## 4. Run it

```sh
zabterm
```

Press <kbd>?</kbd> for all keys, <kbd>1</kbd> <kbd>2</kbd> <kbd>3</kbd> to switch views and <kbd>q</kbd> to quit.

## Without a config file

For a one-off session you can skip the file entirely:

```sh
ZABTERM_URL=https://zabbix.example.com ZABTERM_TOKEN=... zabterm
```
