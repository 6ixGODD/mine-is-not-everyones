param(
    [ValidateSet("all", "pi", "claude", "codex", "opencode")]
    [string[]]$Target = @("all"),
    [switch]$Force
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$SkillsRoot = Join-Path $RepoRoot "skills"
$SkillNames = Get-ChildItem -Path $SkillsRoot -Directory | ForEach-Object { $_.Name }

if ($Target -contains "all") {
    # OpenCode discovers Claude-compatible global skills, so installing both would duplicate names.
    $Target = @("pi", "claude", "codex")
}

$Destinations = @{
    pi       = Join-Path $HOME ".pi\agent\skills"
    claude   = Join-Path $HOME ".claude\skills"
    codex    = Join-Path $HOME ".codex\skills"
    opencode = Join-Path $HOME ".config\opencode\skills"
}

function Install-SkillLink([string]$Source, [string]$Destination) {
    $Parent = Split-Path -Parent $Destination
    New-Item -ItemType Directory -Force -Path $Parent | Out-Null

    if (Test-Path -LiteralPath $Destination) {
        $Item = Get-Item -LiteralPath $Destination -Force
        $CurrentTarget = @($Item.Target) -join ""
        if ($Item.LinkType -and $CurrentTarget -eq $Source) {
            Write-Host "ok    $Destination"
            return
        }
        if (-not $Force) {
            throw "Destination already exists: $Destination. Re-run with -Force only after verifying it contains no unique work."
        }
        Remove-Item -LiteralPath $Destination -Recurse -Force
    }

    if ($IsWindows -or $env:OS -eq "Windows_NT") {
        New-Item -ItemType Junction -Path $Destination -Target $Source | Out-Null
    } else {
        New-Item -ItemType SymbolicLink -Path $Destination -Target $Source | Out-Null
    }
    Write-Host "link  $Destination -> $Source"
}

foreach ($Name in $Target) {
    $Root = $Destinations[$Name]
    if (-not $Root) { throw "Unsupported target: $Name" }
    foreach ($SkillName in $SkillNames) {
        Install-SkillLink (Join-Path $SkillsRoot $SkillName) (Join-Path $Root $SkillName)
    }
}

Write-Host ""
Write-Host "Installed MINE skills for: $($Target -join ', ')"
if ($Target -contains "claude") {
    Write-Host "OpenCode will also discover the Claude-compatible installation."
}
Write-Host "Restart Codex; Pi and Claude can usually reload or discover changes directly."
