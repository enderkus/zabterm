---
title: "Installation"
lead: "Pick whichever fits your system. All three give you the same single binary called zabterm."
---

## Install script (Omarchy, Linux, macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/enderkus/zabterm/main/install.sh | sh
```

The script:

1. Detects your OS and CPU and downloads the matching binary from the latest [GitHub release]({{ site.github_url }}/releases).
2. Installs it to `~/.local/bin/zabterm`.
3. On Linux, writes `~/.local/share/applications/zabterm.desktop` and installs the icon, so zabterm shows up in your app launcher (Walker on Omarchy).
4. Writes a starter config with `zabterm init` if you don't have one yet. An existing config is never touched.

Running it again upgrades zabterm in place.

| Variable | Effect |
|---|---|
| `ZABTERM_VERSION=v0.1.1` | Install a specific release instead of the latest. |
| `ZABTERM_BIN_DIR=/some/dir` | Install somewhere other than `~/.local/bin`. |

> If `~/.local/bin` is not on your `PATH`, the script tells you. Omarchy already has it there.

## Homebrew (macOS, Linux)

```sh
brew install enderkus/tap/zabterm
```

Upgrade with `brew upgrade zabterm`.

## Cargo (from source)

With a Rust toolchain from [rustup.rs](https://rustup.rs):

```sh
cargo install --git https://github.com/enderkus/zabterm
```

Or from a clone:

```sh
git clone https://github.com/enderkus/zabterm
cd zabterm
cargo install --path .
```

## Manual download

Every [release]({{ site.github_url }}/releases) has `.tar.gz` archives for:

| Target | Platform |
|---|---|
| `x86_64-unknown-linux-gnu` | Linux, Intel/AMD |
| `aarch64-unknown-linux-gnu` | Linux, ARM |
| `aarch64-apple-darwin` | macOS, Apple Silicon |
| `x86_64-apple-darwin` | macOS, Intel |

Extract it and put `zabterm` anywhere on your `PATH`.

## Uninstall

```sh
# install script
rm ~/.local/bin/zabterm ~/.local/share/applications/zabterm.desktop
rm -f ~/.local/share/applications/icons/zabterm.png ~/.local/share/icons/hicolor/*/apps/zabterm.*

# Homebrew
brew uninstall zabterm

# your settings, if you want them gone too
rm -r ~/.config/zabterm
```
