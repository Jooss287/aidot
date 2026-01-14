# aidot installer script for Windows
# Usage: irm https://raw.githubusercontent.com/Jooss287/aidot/main/scripts/install.ps1 | iex

$ErrorActionPreference = "Stop"

# Configuration
$Repo = "Jooss287/aidot"
$BinaryName = "aidot.exe"
$InstallDir = if ($env:AIDOT_INSTALL_DIR) { $env:AIDOT_INSTALL_DIR } else { "$env:USERPROFILE\.aidot\bin" }

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] " -ForegroundColor Red -NoNewline
    Write-Host $Message
    exit 1
}

function Check-GitInstalled {
    try {
        $null = & git --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            return $true
        }
    }
    catch {
        # Git not found
    }
    return $false
}

function Get-LatestVersion {
    param([switch]$Prerelease)

    try {
        if ($Prerelease) {
            $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases"
            return $response[0].tag_name
        } else {
            $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
            return $response.tag_name
        }
    }
    catch {
        Write-Error "Failed to get latest version: $_"
    }
}

function Install-Aidot {
    param([switch]$Prerelease)

    # Check if git is installed (required for remote repository features)
    if (-not (Check-GitInstalled)) {
        Write-Warn "Git is not installed. Some features (repo add, pull from remote) will not work."
        Write-Host ""
        Write-Host "To install Git for Windows, visit: https://git-scm.com/download/win" -ForegroundColor Cyan
        Write-Host ""
    }

    $platform = "x86_64-pc-windows-msvc"
    $version = Get-LatestVersion -Prerelease:$Prerelease

    Write-Info "Installing aidot $version for $platform..."

    $archiveName = "aidot-$version-$platform.zip"
    $downloadUrl = "https://github.com/$Repo/releases/download/$version/$archiveName"

    # Create temp directory
    $tempDir = New-Item -ItemType Directory -Path (Join-Path $env:TEMP "aidot-install-$(Get-Random)")

    try {
        # Download
        Write-Info "Downloading $downloadUrl..."
        $archivePath = Join-Path $tempDir $archiveName
        Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath

        # Extract
        Write-Info "Extracting..."
        Expand-Archive -Path $archivePath -DestinationPath $tempDir -Force

        # Install
        Write-Info "Installing to $InstallDir..."
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }

        $sourcePath = Join-Path $tempDir $BinaryName
        $destPath = Join-Path $InstallDir $BinaryName
        Move-Item -Path $sourcePath -Destination $destPath -Force

        Write-Info "Successfully installed aidot to $destPath"

        # Check if InstallDir is in PATH
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($userPath -notlike "*$InstallDir*") {
            Write-Warn "Note: $InstallDir is not in your PATH"
            Write-Host ""
            Write-Host "To add it to your PATH, run:"
            Write-Host ""
            Write-Host "  `$env:Path += `";$InstallDir`"" -ForegroundColor Cyan
            Write-Host "  [Environment]::SetEnvironmentVariable(`"Path`", `$env:Path + `";$InstallDir`", `"User`")" -ForegroundColor Cyan
            Write-Host ""

            # Ask if user wants to add to PATH automatically
            $addToPath = Read-Host "Would you like to add it to your PATH now? (y/N)"
            if ($addToPath -eq "y" -or $addToPath -eq "Y") {
                $newPath = $userPath + ";" + $InstallDir
                [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
                $env:Path += ";$InstallDir"
                Write-Info "Added $InstallDir to your PATH"
            }
        }

        # Print version
        Write-Host ""
        Write-Info "Installation complete!"
        & $destPath --version
    }
    catch {
        Write-Error "Installation failed: $_"
    }
    finally {
        # Cleanup
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Uninstall-Aidot {
    $binaryPath = Join-Path $InstallDir $BinaryName

    if (Test-Path $binaryPath) {
        Remove-Item -Path $binaryPath -Force
        Write-Info "Uninstalled aidot from $InstallDir"
    }
    else {
        Write-Warn "aidot is not installed in $InstallDir"
    }
}

# Main
$command = if ($args.Count -gt 0) { $args[0] } else { "install" }
$includePrerelease = $args -contains "-Prerelease" -or $args -contains "--prerelease"

switch ($command) {
    "install" { Install-Aidot -Prerelease:$includePrerelease }
    "uninstall" { Uninstall-Aidot }
    default {
        Write-Host "Usage: install.ps1 [install|uninstall] [--prerelease]"
        exit 1
    }
}
