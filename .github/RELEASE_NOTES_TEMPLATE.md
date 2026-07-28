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

Each platform archive ships `mine` (or `mine.exe`) plus README and LICENSE,
with a matching `.sha256` sidecar. Verify with
`sha256sum -c mine-<platform>.tar.gz.sha256`.
