#!/usr/bin/env sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
skills_root="$repo_root/skills"
targets="${*:-all}"
[ "$targets" = "all" ] && targets="pi claude codex"

root_for() {
  case "$1" in
    pi) printf '%s/.pi/agent/skills' "$HOME" ;;
    claude) printf '%s/.claude/skills' "$HOME" ;;
    codex) printf '%s/.codex/skills' "$HOME" ;;
    opencode) printf '%s/.config/opencode/skills' "$HOME" ;;
    *) echo "unknown target: $1" >&2; exit 2 ;;
  esac
}

for target in $targets; do
  dest_root=$(root_for "$target")
  mkdir -p "$dest_root"
  for source in "$skills_root"/*; do
    [ -d "$source" ] || continue
    name=$(basename "$source")
    dest="$dest_root/$name"
    if [ -L "$dest" ]; then rm -f "$dest"; fi
    if [ -e "$dest" ]; then
      echo "destination exists and is not a symlink: $dest" >&2
      exit 1
    fi
    ln -s "$source" "$dest"
    echo "link  $dest -> $source"
  done
done

echo "Installed MINE skills for: $targets"
echo "OpenCode also discovers the Claude-compatible installation."
