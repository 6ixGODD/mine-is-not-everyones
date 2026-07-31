#!/usr/bin/env bash
# Find stale execution-plan references in tracked implementation artifacts.
#
# A stable release must not depend on the temporary docs/plan/ workspace. Keep
# comments focused on durable behavior, not the historical plan that introduced
# it. For an intentional literal fixture, exempt only that fixture line with
# an immediately preceding reason comment:
#   // mine-release-allow-plan-reference: protocol fixture
#   let input = "Plan 08-2";
#
# Usage (run from the root of the repository under review):
#   bash references/scan-plan-refs.sh [--check] [--] [pathspec ...]
#
# The script is bundled with the mine-plan-review Skill and runs against the
# current working directory's Git repository. It must not assume the target
# repository ships a copy of itself.
#
# Without --check, print findings and exit zero so the command is useful while
# repairing a tree. With --check, exit one when an unexempted finding exists;
# this is the release-closure gate used by mine-plan-review.
set -euo pipefail

CHECK=false
while (($#)); do
    case "$1" in
        --check) CHECK=true; shift ;;
        --help|-h)
            sed -n '1,18p' "$0"
            exit 0
            ;;
        --) shift; break ;;
        -*) printf 'error: unknown option: %s\n' "$1" >&2; exit 2 ;;
        *) break ;;
    esac
done

# Operate on the repository under review: the current working directory's
# Git toplevel. The script lives in the reviewer's Skill directory and is run
# against arbitrary target repositories, so it never assumes its own location.
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || {
    echo 'error: scan-plan-refs.sh must run inside a Git worktree' >&2
    exit 2
}
cd "$REPO_ROOT"

# Match a plan identifier, not normal domain uses such as `plan release`.
PLAN_PATTERN='(^|[^[:alnum:]_])[Pp]lan[[:space:]-]*[0-9]'
ALLOW_MARKER='mine-release-allow-plan-reference:'

# The default scans every tracked file except documentation and temporary
# planning state, so it works for Go (cmd/, internal/, pkg/, *.go), Python
# (*.py), TypeScript (src/, *.ts), monorepos, and any layout. A pathspec
# argument still narrows the scan. Historical plans/reports/design are
# documentation, not stale implementation references; scanning them would
# make every release fail by construction. A file literally named
# scan-plan-refs.sh is skipped by basename, so the script can safely review
# its own host Skill directory without its usage docs tripping the pattern
# it detects.
if (($#)); then
    PATHS=("$@")
else
    PATHS=(':(exclude)docs/design/**' ':(exclude)docs/design-backup-*/**' ':(exclude)docs/plan/**' ':(exclude)docs/README.md' ':(exclude)README.md' ':(exclude)README.zh-CN.md' ':(exclude)tests/fixtures/**' ':(exclude)**/testdata/**')
fi

findings=0
while IFS= read -r -d '' file; do
    [[ "$(basename "$file")" == 'scan-plan-refs.sh' ]] && continue
    # `grep` returns 1 for no match; that is expected. Its output is processed
    # line-by-line so the allow marker applies only to the exact matching line.
    while IFS= read -r match; do
        line_number=${match%%:*}
        content=${match#*:}
        previous_line=''
        if ((line_number > 1)); then
            previous_line=$(awk -v n="$((line_number - 1))" 'NR == n { print; exit }' "$file")
        fi
        if [[ "$content" == *"$ALLOW_MARKER"* ]] || [[ "$previous_line" == *"$ALLOW_MARKER"* ]]; then
            continue
        fi
        printf '%s:%s: %s\n' "$file" "$line_number" "$content"
        findings=$((findings + 1))
    done < <(grep -nE "$PLAN_PATTERN" -- "$file" || true)
done < <(git ls-files -z -- "${PATHS[@]}")

if ((findings == 0)); then
    echo 'No unexempted plan references found.'
    exit 0
fi

printf '\nFound %d unexempted plan reference(s).\n' "$findings" >&2
printf 'Rewrite historical comments as durable behavior, or use %s immediately before an intentional fixture line.\n' "$ALLOW_MARKER" >&2

if "$CHECK"; then
    exit 1
fi
