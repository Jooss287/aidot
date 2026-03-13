# aidot run script for Windows - npx-style one-off preset apply
#
# Installs aidot to a temporary directory, runs quick-pull with the given
# arguments, then removes everything. The host system is left unchanged.
#
# Usage (pipe into PowerShell):
#   $env:AIDOT_RUN_ARGS="<URL> [OPTIONS]"; irm .../run.ps1 | iex
#
# Or save and run directly:
#   .\run.ps1 <URL> [OPTIONS]
#
# Examples:
#   .\run.ps1 https://github.com/myteam/llm-config
#   .\run.ps1 https://github.com/myteam/llm-config --tools claude,cursor
#   .\run.ps1 .\my-local-preset --dry-run

param(
    [Parameter(Position = 0)]
    [string]$PresetSource,

    [Parameter(ValueFromRemainingArguments)]
    [string[]]$QuickPullArgs
)

$ErrorActionPreference = "Stop"

$Repo       = "Jooss287/aidot"
$BinaryName = "aidot.exe"
$Platform   = "x86_64-pc-windows-msvc"

function Write-RunInfo  { param([string]$m); Write-Host "[aidot-run] " -ForegroundColor Green  -NoNewline; Write-Host $m }
function Write-RunWarn  { param([string]$m); Write-Host "[aidot-run] " -ForegroundColor Yellow -NoNewline; Write-Host $m }
function Write-RunError { param([string]$m); Write-Host "[aidot-run] " -ForegroundColor Red    -NoNewline; Write-Host $m; throw $m }

# --------------------------------------------------------------------------- #
# Support pipe-in mode via env var (irm ... | iex)
# --------------------------------------------------------------------------- #
if (-not $PresetSource -and $env:AIDOT_RUN_ARGS) {
    $parsed    = $env:AIDOT_RUN_ARGS -split '\s+'
    $PresetSource   = $parsed[0]
    $QuickPullArgs  = $parsed[1..($parsed.Length - 1)]
}

if (-not $PresetSource) {
    Write-Host "Usage:" -ForegroundColor Cyan
    Write-Host "  .\run.ps1 <PRESET_URL_OR_PATH> [OPTIONS]"
    Write-Host ""
    Write-Host "Options (passed directly to aidot quick-pull):"
    Write-Host "  --tools <list>   Apply to specific tools (comma-separated: claude,cursor,copilot,codex)"
    Write-Host "  --dry-run        Preview changes without applying"
    Write-Host "  --force          Overwrite existing files without asking"
    Write-Host "  --skip           Skip existing files without asking"
    throw "No preset source specified. See usage above."
}

# --------------------------------------------------------------------------- #
# Fetch latest version
# --------------------------------------------------------------------------- #
function Get-LatestVersion {
    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
        return $response.tag_name
    } catch {
        Write-RunError "Failed to fetch latest version: $_"
    }
}

# --------------------------------------------------------------------------- #
# Isolated install to temp directory
# --------------------------------------------------------------------------- #
$TempDir = Join-Path $env:TEMP "aidot-run-$(Get-Random)"
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

# Always clean up on script exit
try {
    $version     = Get-LatestVersion
    $archiveName = "aidot-$version-$Platform.zip"
    $downloadUrl = "https://github.com/$Repo/releases/download/$version/$archiveName"
    $archivePath = Join-Path $TempDir $archiveName

    Write-RunInfo "Creating isolated environment in $TempDir..."
    Write-RunInfo "Fetching aidot $version ($Platform)..."

    try {
        Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath
    } catch {
        Write-RunError "Download failed ($downloadUrl): $_"
    }

    Expand-Archive -Path $archivePath -DestinationPath $TempDir -Force

    $AidotBin = Join-Path $TempDir $BinaryName
    if (-not (Test-Path $AidotBin)) {
        Write-RunError "Binary not found after extraction: $AidotBin"
    }

    # --------------------------------------------------------------------------- #
    # Run quick-pull
    # --------------------------------------------------------------------------- #
    $displayArgs = ($QuickPullArgs -join " ")
    Write-RunInfo "Running: aidot quick-pull $PresetSource $displayArgs"
    Write-Host ""

    & $AidotBin quick-pull $PresetSource @QuickPullArgs

    Write-Host ""
    Write-RunInfo "Done. Temporary aidot installation removed."
} finally {
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}
