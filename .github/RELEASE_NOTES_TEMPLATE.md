# Release notes template

This file is the hand-written portion of each GitHub Release body. The release
workflow prepends the git-cliff-generated changelog and appends this file's
contents, so a release note = auto changelog + short install/usage section.

Keep it short. Installation detail lives in the README; here only summarize
how to get this release running.

---

## Install

One command (no Rust toolchain required):

- **Windows (PowerShell):** `irm https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.ps1 | iex`
- **macOS / Linux:** `curl -fsSL https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.sh | sh`

This downloads the prebuilt `mine` binary for your platform, puts it on PATH,
and runs `mine setup`, which detects installed coding agents and installs MINE
(Skills + MCP server) into the ones you choose.

## Upgrade

If `mine` is already installed: `mine update`.

## Assets

Each platform archive ships `mine` (or `mine.exe`) plus README and LICENSE, with
a matching `.sha256` sidecar. Verify with `sha256sum -c mine-<platform>.tar.gz.sha256`.
