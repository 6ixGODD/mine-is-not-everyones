#!/usr/bin/env sh
set -eu
repo="${MINE_REPO:-https://github.com/6ixGODD/mine-is-not-everyones.git}"
ref="${MINE_REF:-main}"
install_dir="${MINE_HOME:-${XDG_DATA_HOME:-$HOME/.local/share}/mine-is-not-everyones}"

command -v git >/dev/null 2>&1 || { echo "git is required but was not found in PATH" >&2; exit 1; }

if [ -d "$install_dir/.git" ]; then
  git -C "$install_dir" fetch --tags --prune
  git -C "$install_dir" checkout "$ref"
  git -C "$install_dir" pull --ff-only
elif [ -e "$install_dir" ]; then
  echo "install directory exists but is not a Git checkout: $install_dir" >&2
  exit 1
else
  mkdir -p "$(dirname "$install_dir")"
  git clone --branch "$ref" "$repo" "$install_dir"
fi

"$install_dir/scripts/install.sh"
echo "MINE installed from $install_dir"
