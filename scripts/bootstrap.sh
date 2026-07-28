#!/usr/bin/env sh
# MINE bootstrap installer (Linux / macOS).
#
# Run from anywhere -- no clone, no Rust toolchain required:
#   curl -fsSL https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.sh | sh
#
# Pin a published release tag:
#   MINE_REF=v0.1.0 sh -c "$(curl -fsSL https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.sh)"
#
# Responsibilities:
#   1. Clone or update a managed MINE source checkout (used only for Skills).
#   2. Link the five Skills into the discovering agent directories
#      (Pi / Claude Code / Codex; OpenCode shares the Claude-compatible set).
#   3. Download the prebuilt `mine` binary for this platform from the matching
#      GitHub Release and install it on PATH.
#
# Environment overrides: MINE_REPO, MINE_REF (tag or "latest"),
# MINE_HOME, MINE_BIN_DIR, MINE_RELEASE_ACCOUNT, MINE_RELEASE_REPO.
set -eu

repo="${MINE_REPO:-https://github.com/6ixGODD/mine-is-not-everyones.git}"
ref="${MINE_REF:-latest}"
install_dir="${MINE_HOME:-${XDG_DATA_HOME:-$HOME/.local/share}/mine-is-not-everyones}"
bin_dir="${MINE_BIN_DIR:-$HOME/.local/bin}"

notify() { printf '%s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || die "$1 is required but was not found in PATH"; }
need git
need tar

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1"; }
  fetch_file() { curl -fsSL -o "$2" "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO- "$1"; }
  fetch_file() { wget -qO "$2" "$1"; }
else
  die "curl or wget is required to download the prebuilt binary"
fi

release_account="${MINE_RELEASE_ACCOUNT:-6ixGODD}"
release_repo="${MINE_RELEASE_REPO:-mine-is-not-everyones}"

# --- 1. Managed source checkout (Skills only) ----------------------
clone_ref=""
[ "$ref" = "latest" ] || clone_ref="--branch=$ref"

if [ -d "$install_dir/.git" ]; then
  git -C "$install_dir" fetch --tags --prune
  if [ "$ref" = "latest" ]; then
    head_ref="$(git -C "$install_dir" symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's|refs/remotes/origin/||' || true)"
    if [ -n "$head_ref" ]; then
      git -C "$install_dir" checkout "$head_ref"
    else
      git -C "$install_dir" checkout master 2>/dev/null || git -C "$install_dir" checkout main
    fi
  else
    git -C "$install_dir" checkout "$ref"
  fi
  git -C "$install_dir" pull --ff-only
elif [ -e "$install_dir" ]; then
  die "install directory exists but is not a Git checkout: $install_dir"
else
  mkdir -p "$(dirname "$install_dir")"
  # shellcheck disable=SC2086
  git clone $clone_ref "$repo" "$install_dir"
fi

# --- 2. Link Skills --------------------------------------------------
"$install_dir/scripts/install.sh" "$@"
notify "Skills linked."

# --- 3. Download the prebuilt binary ---------------------------------
if [ "$ref" = "latest" ]; then
  api="https://api.github.com/repos/$release_account/$release_repo/releases/latest"
  tag="$(fetch "$api" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"
  [ -n "$tag" ] || die "could not resolve the latest release tag from $api"
else
  tag="$ref"
fi

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
fetch_file "$url" "$tmpdir/$asset" || die "download failed for $asset (tag $tag). The release may not include this platform or the tag may not exist yet."
tar xzf "$tmpdir/$asset" -C "$tmpdir"
[ -f "$tmpdir/mine" ] || die "archive $asset did not contain a 'mine' binary"

mkdir -p "$bin_dir"
mv -f "$tmpdir/mine" "$bin_dir/mine"
chmod +x "$bin_dir/mine"
notify "Installed mine to $bin_dir/mine (release $tag)"

# Verify, and tell the user how to put it on PATH if it is not already.
case ":$PATH:" in
  *":$bin_dir:"*)
    "$bin_dir/mine" --version || die "binary verification failed"
    ;;
  *)
    "$bin_dir/mine" --version || die "binary verification failed"
    notify "Add $bin_dir to your PATH to run 'mine' from any shell, e.g.:"
    notify "  export PATH=\"$bin_dir:\$PATH\""
    ;;
esac

notify "MINE installed successfully."