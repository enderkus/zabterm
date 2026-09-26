#!/bin/sh
# Install the latest zabterm release binary.
#
#   curl -fsSL https://raw.githubusercontent.com/enderkus/zabterm/main/install.sh | sh
#
# Installs to ~/.local/bin (override with ZABTERM_BIN_DIR), adds an app
# launcher entry on Linux (so it shows up in Omarchy's menu), and writes a
# starter config if there isn't one yet. Pin a version with ZABTERM_VERSION=v0.1.0.
set -eu

REPO="enderkus/zabterm"
BIN_DIR="${ZABTERM_BIN_DIR:-$HOME/.local/bin}"

say() { printf '\033[1;34m::\033[0m %s\n' "$1"; }
die() { printf '\033[31mx\033[0m %s\n' "$1" >&2; exit 1; }

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)          target="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64|Linux-arm64) target="aarch64-unknown-linux-gnu" ;;
  Darwin-arm64)          target="aarch64-apple-darwin" ;;
  Darwin-x86_64)         target="x86_64-apple-darwin" ;;
  *) die "no prebuilt binary for $(uname -s) $(uname -m); build from source: cargo install --git https://github.com/$REPO" ;;
esac

command -v curl >/dev/null || die "curl is required"
command -v tar >/dev/null || die "tar is required"

version="${ZABTERM_VERSION:-}"
if [ -z "$version" ]; then
  version=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)
  [ -n "$version" ] || die "could not find the latest release"
fi

name="zabterm-$version-$target"
url="https://github.com/$REPO/releases/download/$version/$name.tar.gz"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

say "Downloading zabterm $version ($target)"
curl -fsSL "$url" -o "$tmp/zabterm.tar.gz" || die "download failed: $url"
tar -xzf "$tmp/zabterm.tar.gz" -C "$tmp"

mkdir -p "$BIN_DIR"
install -m 755 "$tmp/$name/zabterm" "$BIN_DIR/zabterm"
say "Installed $BIN_DIR/zabterm"

if [ "$(uname -s)" = "Linux" ]; then
  apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
  mkdir -p "$apps"
  cat > "$apps/zabterm.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=zabterm
GenericName=Monitoring
Comment=Terminal UI for Zabbix
Exec=$BIN_DIR/zabterm
Terminal=true
Icon=utilities-system-monitor
Categories=System;Monitor;
Keywords=zabbix;monitoring;problems;hosts;
EOF
  say "Added app launcher entry"
fi

config="${XDG_CONFIG_HOME:-$HOME/.config}/zabterm/config.toml"
if [ ! -f "$config" ]; then
  "$BIN_DIR/zabterm" init >/dev/null
  say "Wrote starter config to $config"
fi

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) say "Add $BIN_DIR to your PATH to run zabterm from anywhere" ;;
esac

printf '\n  Set your Zabbix URL and API token in %s\n  then run: \033[1mzabterm check\033[0m and \033[1mzabterm\033[0m\n\n' "$config"
