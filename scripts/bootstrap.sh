#!/usr/bin/env sh
# MINE bootstrap installer (Linux / macOS).
#
# Run from anywhere -- no clone, no Rust toolchain required:
#   curl -fsSL https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.sh | sh
#
# This loader only fetches the prebuilt `mine` binary for the current platform
# from the matching GitHub Release and runs `mine setup`, which handles version
# checking, coding-agent detection, the interactive selector, and MCP/Skill
# installation. Pin a version with MINE_REF=v0.1.0. Extra args pass through to
# `mine setup` (e.g. `sh ... -- --agents claude,codex --yes`).
set -eu

ref="${MINE_REF:-latest}"
bin_dir="${MINE_BIN_DIR:-$HOME/.local/bin}"

notify() { printf '%s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO- "$1"; }
else
  die "curl or wget is required to download the prebuilt binary"
fi

release_account="${MINE_RELEASE_ACCOUNT:-6ixGODD}"
release_repo="${MINE_RELEASE_REPO:-mine-is-not-everyones}"

# --- Resolve the release tag ----------------------------------------
if [ "$ref" = "latest" ]; then
  api="https://api.github.com/repos/$release_account/$release_repo/releases/latest"
  tag="$(fetch "$api" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"
  [ -n "$tag" ] || die "could not resolve the latest release tag from $api (publish a v* tag first)"
else
  tag="$ref"
fi

# --- Download the prebuilt binary -----------------------------------
case "$(uname -s):$(uname -m)" in
  Linux:x86_64)        target="x86_64-unknown-linux-gnu" ;;
  Darwin:arm64)        target="aarch64-apple-darwin" ;;
  Darwin:x86_64)      target="x86_64-apple-darwin" ;;
  *) die "unsupported platform: $(uname -s) $(uname -m). Prebuilt binaries are published for Linux x86_64, macOS arm64/x86_64, and Windows x86_64." ;;
esac

asset="mine-$target.tar.gz"
url="https://github.com/$release_account/$release_repo/releases/download/$tag/$asset"
notify "Downloading $url"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
fetch "$url" >"$tmpdir/$asset" 2>/dev/null || die "download failed for $asset (tag $tag). The release may not include this platform or the tag may not exist yet."
tar xzf "$tmpdir/$asset" -C "$tmpdir"
[ -f "$tmpdir/mine" ] || { find "$tmpdir" -name mine -type f -exec cp {} "$tmpdir/mine" \; ; }
[ -f "$tmpdir/mine" ] || die "archive $asset did not contain a 'mine' binary"

mkdir -p "$bin_dir"
mv -f "$tmpdir/mine" "$bin_dir/mine"
chmod +x "$bin_dir/mine"
notify "mine $tag installed to $bin_dir/mine"
notify ""

# --- Run mine setup -------------------------------------------------
# Delegate everything (banner, version check, agent detection, interactive
# selector, MCP+Skill install) to the binary itself.
case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *) export PATH="$bin_dir:$PATH" ;;
esac

exec "$bin_dir/mine" setup "$@"
