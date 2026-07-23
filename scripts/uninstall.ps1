param(
    [ValidateSet("all", "pi", "claude", "codex", "opencode")]
    [string[]]$Target = @("all")
)
$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$SkillNames = Get-ChildItem -Path (Join-Path $RepoRoot "skills") -Directory | ForEach-Object { $_.Name }
if ($Target -contains "all") { $Target = @("pi", "claude", "codex", "opencode") }
$Destinations = @{
    pi       = Join-Path $HOME ".pi\agent\skills"
    claude   = Join-Path $HOME ".claude\skills"
    codex    = Join-Path $HOME ".codex\skills"
    opencode = Join-Path $HOME ".config\opencode\skills"
}
foreach ($Name in $Target) {
    foreach ($SkillName in $SkillNames) {
        $Path = Join-Path $Destinations[$Name] $SkillName
        if (-not (Test-Path -LiteralPath $Path)) { continue }
        $Item = Get-Item -LiteralPath $Path -Force
        if (-not $Item.LinkType) {
            Write-Warning "skip non-link path: $Path"
            continue
        }
        Remove-Item -LiteralPath $Path -Force
        Write-Host "removed $Path"
    }
}
