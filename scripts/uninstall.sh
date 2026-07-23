#!/usr/bin/env sh
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
skills_root="$repo_root/skills"
targets="${*:-all}"
[ "$targets" = "all" ] && targets="pi claude codex opencode"
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
  for source in "$skills_root"/*; do
    [ -d "$source" ] || continue
    dest="$dest_root/$(basename "$source")"
    if [ -L "$dest" ]; then rm -f "$dest" && echo "removed $dest"; fi
  done
done
