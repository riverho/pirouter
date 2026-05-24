#Requires -Version 5.1
<#
.SYNOPSIS
    Full build pipeline: CLI daemon to Tauri desktop app to Windows NSIS installer.

.DESCRIPTION
    1. cargo build --release          (pirouter CLI daemon)
    2. Copy binary into tray_app/src-tauri/binaries/
    3. npm install                    (tray_app frontend deps, if needed)
    4. npm run desktop:build          (bundles frontend + Rust shell to NSIS .exe)
    5. Report the installer path.

.PARAMETER SkipCliCheck
    Skip the post-build CLI dry-run that validates config and routes.

.EXAMPLE
    npm run desktop:build:win
    npm run desktop:build:win:skip-check
#>
param(
    [switch]$SkipCliCheck
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# helpers

function Write-Step([string]$msg) {
    Write-Host "`n==> $msg" -ForegroundColor Cyan
}

function Write-Ok([string]$msg) {
    Write-Host "    OK  $msg" -ForegroundColor Green
}

function Write-Fail([string]$msg) {
    Write-Host "    ERR $msg" -ForegroundColor Red
    exit 1
}

function Invoke-Checked([string]$desc, [scriptblock]$block) {
    Write-Host "    ... $desc" -ForegroundColor Gray
    try {
        & $block
        if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) { throw "exit code $LASTEXITCODE" }
        Write-Ok $desc
    } catch {
        Write-Fail "$desc - $_"
    }
}

# locate paths

$RepoRoot    = Split-Path -Parent $PSScriptRoot
$AppDir      = Join-Path $RepoRoot "tray_app"
$BinariesDir = Join-Path $AppDir "src-tauri\binaries"
$ReleaseBin  = Join-Path $RepoRoot "target\release\pirouter.exe"
$BundledBin  = Join-Path $BinariesDir "pirouter.exe"

Push-Location $RepoRoot
try {

# 1. Build CLI

Write-Step "Step 1 - cargo build --release (CLI daemon)"
Invoke-Checked "compile pirouter" {
    cargo build --release
}

if (-not (Test-Path $ReleaseBin)) {
    Write-Fail "Expected binary not found: $ReleaseBin"
}
$sizeMB = (Get-Item $ReleaseBin).Length / 1MB
Write-Ok ('binary: {0} ({1:F1} MB)' -f $ReleaseBin, $sizeMB)

# 2. Post-build CLI dry-run

if (-not $SkipCliCheck) {
    Write-Step "Step 2 - CLI dry-run verification"
    $ExampleCfg = Join-Path $RepoRoot "config.example.toml"
    if (Test-Path $ExampleCfg) {
        Invoke-Checked "check-config" {
            & $ReleaseBin --config $ExampleCfg check-config
        }
        Invoke-Checked "models list" {
            & $ReleaseBin --config $ExampleCfg models
        }
    } else {
        Write-Host "    (skipping dry-run - config.example.toml not found)" -ForegroundColor Yellow
    }
} else {
    Write-Step "Step 2 - CLI dry-run skipped (-SkipCliCheck)"
}

# 3. Copy binary for Tauri bundling

Write-Step "Step 3 - copy binary to tray_app/src-tauri/binaries/"
New-Item -ItemType Directory -Force $BinariesDir | Out-Null
Copy-Item -Force $ReleaseBin $BundledBin
Write-Ok "copied to $BundledBin"

# 4. npm install

Write-Step "Step 4 - npm install (tray_app)"
Push-Location $AppDir
try {
    Invoke-Checked "npm install" {
        npm install
    }
} finally {
    Pop-Location
}

# 5. tauri build

Write-Step "Step 5 - npm run desktop:build (NSIS installer)"
Push-Location $AppDir
try {
    Invoke-Checked "tauri build" {
        npm run desktop:build
    }
} finally {
    Pop-Location
}

# 6. Report installer path

Write-Step "Build complete"

$InstallerDir = Join-Path $AppDir "src-tauri\target\release\bundle\nsis"
$Installer = Get-ChildItem -Path $InstallerDir -Filter "*.exe" -ErrorAction SilentlyContinue |
             Sort-Object LastWriteTime -Descending |
             Select-Object -First 1

if ($Installer) {
    $installerMB = $Installer.Length / 1MB
    Write-Host ""
    Write-Host "  Installer : $($Installer.FullName)" -ForegroundColor White
    Write-Host ('  Size      : {0:F1} MB' -f $installerMB) -ForegroundColor White
    Write-Host ""
    Write-Host "  Distribute or double-click to install." -ForegroundColor Green
} else {
    Write-Host "  (installer .exe not found in $InstallerDir - check tauri build output above)" -ForegroundColor Yellow
}

} finally {
    Pop-Location
}
