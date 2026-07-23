param(
    [string]$Repository = $(if ($env:MINE_REPO) { $env:MINE_REPO } else { "https://github.com/6ixGODD/mine-is-not-everyones.git" }),
    [string]$Ref = "main",
    [string]$InstallDirectory = $(if ($env:MINE_HOME) { $env:MINE_HOME } elseif ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA "mine-is-not-everyones" } else { Join-Path $HOME ".local\share\mine-is-not-everyones" }),
    [switch]$Force
)

$ErrorActionPreference = "Stop"
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "git is required but was not found in PATH."
}

if (Test-Path -LiteralPath (Join-Path $InstallDirectory ".git")) {
    git -C $InstallDirectory fetch --tags --prune
    git -C $InstallDirectory checkout $Ref
    git -C $InstallDirectory pull --ff-only
} elseif (Test-Path -LiteralPath $InstallDirectory) {
    throw "Install directory exists but is not a Git checkout: $InstallDirectory"
} else {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $InstallDirectory) | Out-Null
    git clone --branch $Ref $Repository $InstallDirectory
}

$InvocationArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Join-Path $InstallDirectory "scripts\install.ps1"))
if ($Force) { $InvocationArgs += "-Force" }
$Pwsh = Get-Command pwsh -ErrorAction SilentlyContinue
if ($Pwsh) {
    & $Pwsh.Source @InvocationArgs
} else {
    & powershell @InvocationArgs
}
if ($LASTEXITCODE -ne 0) { throw "MINE installer failed with exit code $LASTEXITCODE" }

Write-Host "MINE installed from $InstallDirectory"
