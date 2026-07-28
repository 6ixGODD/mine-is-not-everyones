param(
    [string]$Repository = $(if ($env:MINE_REPO) { $env:MINE_REPO } else { "https://github.com/6ixGODD/mine-is-not-everyones.git" }),
    [string]$Ref = $(if ($env:MINE_REF) { $env:MINE_REF } else { "latest" }),
    [string]$InstallDirectory = $(if ($env:MINE_HOME) { $env:MINE_HOME } elseif ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA "mine-is-not-everyones" } else { Join-Path $HOME ".local\share\mine-is-not-everyones" }),
    [string]$BinDirectory = $(if ($env:MINE_BIN_DIR) { $env:MINE_BIN_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\mine" }),
    [switch]$Force
)

# MINE bootstrap installer (Windows).
#
# Run from anywhere -- no clone, no Rust toolchain required:
#   Set-ExecutionPolicy -Scope Process Bypass
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.ps1)))
#
# Pin a published release tag:
#   $env:MINE_REF = 'v0.1.0'
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.ps1)))
#
# Responsibilities:
#   1. Clone or update a managed MINE source checkout (used only for Skills).
#   2. Link the five Skills into the discovering agent directories.
#   3. Download the prebuilt `mine.exe` from the matching GitHub Release and
#      install it on the user PATH.

$ErrorActionPreference = "Stop"
# Force TLS 1.2+ for GitHub downloads on Windows PowerShell 5.1.
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "git is required but was not found in PATH."
}

$ReleaseAccount = if ($env:MINE_RELEASE_ACCOUNT) { $env:MINE_RELEASE_ACCOUNT } else { "6ixGODD" }
$ReleaseRepo = if ($env:MINE_RELEASE_REPO) { $env:MINE_RELEASE_REPO } else { "mine-is-not-everyones" }

# --- 1. Managed source checkout (Skills only) ----------------------
$cloneArgs = @()
if ($Ref -ne "latest") { $cloneArgs = @("--branch", $Ref) }

if (Test-Path -LiteralPath (Join-Path $InstallDirectory ".git")) {
    git -C $InstallDirectory fetch --tags --prune
    if ($Ref -eq "latest") {
        $headRef = & git -C $InstallDirectory symbolic-ref refs/remotes/origin/HEAD 2>$null
        if ($LASTEXITCODE -eq 0 -and $headRef) {
            $branch = ($headRef -replace 'refs/remotes/origin/', '').Trim()
            git -C $InstallDirectory checkout $branch | Out-Null
        } else {
            git -C $InstallDirectory checkout master 2>$null | Out-Null
            if ($LASTEXITCODE -ne 0) { git -C $InstallDirectory checkout main | Out-Null }
        }
    } else {
        git -C $InstallDirectory checkout $Ref | Out-Null
    }
    git -C $InstallDirectory pull --ff-only | Out-Null
} elseif (Test-Path -LiteralPath $InstallDirectory) {
    throw "Install directory exists but is not a Git checkout: $InstallDirectory"
} else {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $InstallDirectory) | Out-Null
    git clone $cloneArgs $Repository $InstallDirectory
    if ($LASTEXITCODE -ne 0) { throw "git clone failed" }
}

# --- 2. Link Skills --------------------------------------------------
$installScript = Join-Path $InstallDirectory "scripts\install.ps1"
$invArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $installScript)
if ($Force) { $invArgs += "-Force" }
$pwsh = Get-Command pwsh -ErrorAction SilentlyContinue
if ($pwsh) {
    & $pwsh.Source @invArgs
} else {
    & powershell @invArgs
}
if ($LASTEXITCODE -ne 0) { throw "MINE installer failed with exit code $LASTEXITCODE" }
Write-Host "Skills linked."

# --- 3. Download the prebuilt binary ---------------------------------
if ($Ref -eq "latest") {
    $apiUrl = "https://api.github.com/repos/$ReleaseAccount/$ReleaseRepo/releases/latest"
    $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "mine-bootstrap" }
    $tag = $release.tag_name
} else {
    $tag = $Ref
}
if (-not $tag) { throw "Could not resolve release tag." }

$target = "x86_64-pc-windows-msvc"
$asset = "mine-$target.zip"
$assetUrl = "https://github.com/$ReleaseAccount/$ReleaseRepo/releases/download/$tag/$asset"
Write-Host "Downloading $assetUrl"

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $tmp | Out-Null
$zip = Join-Path $tmp $asset
Invoke-WebRequest -Uri $assetUrl -OutFile $zip -Headers @{ "User-Agent" = "mine-bootstrap" }
Expand-Archive -Path $zip -DestinationPath $tmp -Force

$srcExe = Join-Path $tmp "mine.exe"
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
    Write-Host "Open a new shell (or re-open your terminal) for 'mine' to be on PATH."
}

& $destExe --version
if ($LASTEXITCODE -ne 0) { throw "binary verification failed" }
Write-Host "MINE installed (release $tag)."