param(
    [string]$Repository = $(if ($env:MINE_REPO) { $env:MINE_REPO } else { "https://github.com/6ixGODD/mine-is-not-everyones.git" }),
    [string]$Ref = $(if ($env:MINE_REF) { $env:MINE_REF } else { "latest" }),
    [string]$BinDirectory = $(if ($env:MINE_BIN_DIR) { $env:MINE_BIN_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\mine" }),
    [Parameter(ValueFromRemainingArguments=$true)]
    [string[]]$SetupArgs
)

# MINE bootstrap installer (Windows).
#
# Run from anywhere -- no clone, no Rust toolchain required:
#   irm https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.ps1 | iex
#
# This loader only fetches the prebuilt `mine` binary for the current platform
# from the matching GitHub Release and runs `mine setup`, which handles version
# checking, coding-agent detection, the interactive selector, and MCP/Skill
# installation. Pin a version with MINE_REF=v0.1.0.

$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$ReleaseAccount = if ($env:MINE_RELEASE_ACCOUNT) { $env:MINE_RELEASE_ACCOUNT } else { "6ixGODD" }
$ReleaseRepo = if ($env:MINE_RELEASE_REPO) { $env:MINE_RELEASE_REPO } else { "mine-is-not-everyones" }

# --- Resolve the release tag ----------------------------------------
if ($Ref -eq "latest") {
    $apiUrl = "https://api.github.com/repos/$ReleaseAccount/$ReleaseRepo/releases/latest"
    try {
        $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "mine-bootstrap" } -ErrorAction Stop
    } catch [System.Net.WebException] {
        throw "No published release found at $apiUrl. Publish a v* tag first, or set MINE_REF to an existing tag. GitHub response: $($_.Exception.Message)"
    }
    $tag = $release.tag_name
} else {
    $tag = $Ref
}
if (-not $tag) { throw "Could not resolve release tag for $ReleaseAccount/$ReleaseRepo." }

# --- Download the prebuilt binary -----------------------------------
# Friendly platform names match the release artifact filenames
# (mine-windows-x86_64.zip, mine-linux-x86_64.tar.gz, mine-macos-arm64.tar.gz).
$target = "windows-x86_64"
$asset = "mine-$target.zip"
$assetUrl = "https://github.com/$ReleaseAccount/$ReleaseRepo/releases/download/$tag/$asset"
Write-Host "Downloading $assetUrl"

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $tmp | Out-Null
$zip = Join-Path $tmp $asset
try {
    Invoke-WebRequest -Uri $assetUrl -OutFile $zip -Headers @{ "User-Agent" = "mine-bootstrap" } -ErrorAction Stop
} catch [System.Net.WebException] {
    throw "Download failed for $asset (tag $tag). The release may not include this platform yet, or tag $tag does not exist. GitHub response: $($_.Exception.Message)"
}
Expand-Archive -Path $zip -DestinationPath $tmp -Force

$srcExe = Join-Path $tmp "mine.exe"
if (-not (Test-Path -LiteralPath $srcExe)) {
    # The archive may stage the binary under a staging directory.
    $staged = Get-ChildItem -Path $tmp -Recurse -Filter "mine.exe" | Select-Object -First 1
    if ($staged) { $srcExe = $staged.FullName }
}
if (-not (Test-Path -LiteralPath $srcExe)) {
    throw "mine.exe not found in archive $asset"
}

New-Item -ItemType Directory -Force -Path $BinDirectory | Out-Null
$destExe = Join-Path $BinDirectory "mine.exe"
Move-Item -Force -LiteralPath $srcExe -Destination $destExe

# Persist user PATH so 'mine' is callable from new shells.
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not ($userPath -like "*$BinDirectory*")) {
    $newPath = if ($userPath) { "$BinDirectory;$userPath" } else { $BinDirectory }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Added $BinDirectory to your user PATH."
    # Also make it usable in this shell without a restart.
    $env:PATH = "$BinDirectory;$env:PATH"
}

Write-Host "mine $tag installed to $destExe"
Write-Host ""

# --- Run mine setup -------------------------------------------------
# Delegate everything (banner, version check, agent detection, interactive
# selector, MCP+Skill install) to the binary itself.
$setupArgList = @("setup")
if ($SetupArgs) { $setupArgList += $SetupArgs }
& $destExe @setupArgList
if ($LASTEXITCODE -ne 0) { throw "mine setup failed with exit code $LASTEXITCODE" }
