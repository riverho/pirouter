#Requires -Version 5.1
<#
.SYNOPSIS
    Build pirouter for Windows and run post-build dry-run verification.

.DESCRIPTION
    1. Runs `cargo build --release`
    2. Copies the binary to scripts/../dist/pirouter.exe
    3. Runs check-config and a set of representative `route` dry-runs
       against config.example.toml to confirm the binary is sane.

.PARAMETER Config
    Path to the config file used for dry-run validation.
    Defaults to config.example.toml in the repo root.

.PARAMETER SkipDryRun
    Build only; skip post-build dry-run verification.

.EXAMPLE
    .\scripts\build.ps1
    .\scripts\build.ps1 -Config C:\pirouter\config.toml
    .\scripts\build.ps1 -SkipDryRun
#>
param(
    [string]$Config   = "",
    [switch]$SkipDryRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── helpers ──────────────────────────────────────────────────────────────────

function Write-Step([string]$msg) {
    Write-Host "`n==> $msg" -ForegroundColor Cyan
}

function Write-Ok([string]$msg) {
    Write-Host "    OK  $msg" -ForegroundColor Green
}

function Write-Fail([string]$msg) {
    Write-Host "    ERR $msg" -ForegroundColor Red
}

function Invoke-Checked([string]$description, [scriptblock]$block) {
    Write-Host "    ... $description" -ForegroundColor Gray
    try {
        & $block
        if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) {
            throw "exit code $LASTEXITCODE"
        }
        Write-Ok $description
    } catch {
        Write-Fail "$description — $_"
        exit 1
    }
}

# ── locate repo root ─────────────────────────────────────────────────────────

$RepoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $RepoRoot
try {

# ── default config ────────────────────────────────────────────────────────────

if ($Config -eq "") {
    $Config = Join-Path $RepoRoot "config.example.toml"
}
if (-not (Test-Path $Config)) {
    Write-Fail "Config not found: $Config"
    exit 1
}

# ── 1. build ──────────────────────────────────────────────────────────────────

Write-Step "cargo build --release"
Invoke-Checked "compile" {
    cargo build --release 2>&1 | Write-Host
}

$Bin = Join-Path $RepoRoot "target\release\pirouter.exe"
if (-not (Test-Path $Bin)) {
    Write-Fail "Binary not found after build: $Bin"
    exit 1
}

$size = (Get-Item $Bin).Length / 1MB
Write-Ok ("binary: {0} ({1:F1} MB)" -f $Bin, $size)

# ── 2. copy to dist/ ──────────────────────────────────────────────────────────

$DistDir = Join-Path $RepoRoot "dist"
$DistBin = Join-Path $DistDir "pirouter.exe"
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
Copy-Item -Force $Bin $DistBin
Write-Ok "copied to dist\pirouter.exe"

# ── 3. dry-run verification ───────────────────────────────────────────────────

if ($SkipDryRun) {
    Write-Step "Skipping dry-run (--SkipDryRun)"
} else {
    Write-Step "Post-build dry-run verification (config: $Config)"

    Invoke-Checked "check-config" {
        & $DistBin --config $Config check-config 2>&1 | Write-Host
    }

    Invoke-Checked "models list" {
        & $DistBin --config $Config models 2>&1 | Write-Host
    }

    $routes = @(
        @{ label = "casual prompt → cheapest"; args = @("--prompt", "hello world") },
        @{ label = "code/debug → strong model"; args = @("--prompt", "debug this distributed SQL migration") },
        @{ label = "tool-use flag → tools-capable model"; args = @("--prompt", "search the web for me", "--tools") },
        @{ label = "explicit model alias"; args = @("--model", "sonnet", "--prompt", "translate this paragraph") },
        @{ label = "long context marker"; args = @("--prompt", ("x" * 5000)) }
    )

    foreach ($r in $routes) {
        $label = $r.label
        $extraArgs = $r.args
        Invoke-Checked "route: $label" {
            & $DistBin --config $Config route @extraArgs 2>&1 | Write-Host
        }
    }
}

# ── summary ───────────────────────────────────────────────────────────────────

Write-Step "Build complete"
Write-Host "`n  Binary : $DistBin" -ForegroundColor White
Write-Host "  Size   : $("{0:F1}" -f $size) MB" -ForegroundColor White
Write-Host "  Run it : .\dist\pirouter.exe --config config.toml run" -ForegroundColor White
Write-Host ""

} finally {
    Pop-Location
}
