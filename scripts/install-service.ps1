#Requires -RunAsAdministrator
#Requires -Version 5.1
<#
.SYNOPSIS
    Install pirouter as a Windows Service using NSSM.

.DESCRIPTION
    pirouter v0 is not yet Windows-Service-Control-Manager (SCM) aware,
    meaning it doesn't respond to SERVICE_STOP / SERVICE_PAUSE signals
    natively.  NSSM (Non-Sucking Service Manager) wraps any executable
    and handles the SCM protocol on its behalf — the correct v0 approach.

    What this script does:
      1. Checks for (or downloads) nssm.exe
      2. Registers pirouter.exe as service "pirouter"
      3. Configures stdout/stderr log rotation
      4. Sets the service to auto-start
      5. Starts the service immediately

.PARAMETER BinaryPath
    Full path to pirouter.exe.
    Defaults to .\dist\pirouter.exe relative to the repo root.

.PARAMETER ConfigPath
    Full path to the pirouter config.toml.
    Defaults to %APPDATA%\pirouter\config.toml  (the platform default).

.PARAMETER ServiceName
    Windows service name.  Default: pirouter

.PARAMETER LogDir
    Directory for stdout/stderr logs.
    Default: %ProgramData%\pirouter\logs

.PARAMETER NssmPath
    Path to nssm.exe if you have it already.
    If omitted and nssm.exe is not on PATH, the script downloads it.

.EXAMPLE
    # Minimal — uses dist\pirouter.exe and platform-default config path
    .\scripts\install-service.ps1

    # Explicit paths
    .\scripts\install-service.ps1 `
        -BinaryPath  C:\tools\pirouter.exe `
        -ConfigPath  C:\pirouter\config.toml
#>
param(
    [string]$BinaryPath  = "",
    [string]$ConfigPath  = "",
    [string]$ServiceName = "pirouter",
    [string]$LogDir      = "",
    [string]$NssmPath    = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── helpers ──────────────────────────────────────────────────────────────────

function Write-Step([string]$msg) { Write-Host "`n==> $msg" -ForegroundColor Cyan }
function Write-Ok([string]$msg)   { Write-Host "    OK  $msg" -ForegroundColor Green }
function Write-Fail([string]$msg) { Write-Host "    ERR $msg" -ForegroundColor Red; exit 1 }

# ── locate repo root ─────────────────────────────────────────────────────────

$RepoRoot = Split-Path -Parent $PSScriptRoot

# ── resolve paths ────────────────────────────────────────────────────────────

if ($BinaryPath -eq "") {
    $BinaryPath = Join-Path $RepoRoot "dist\pirouter.exe"
}
if (-not (Test-Path $BinaryPath)) {
    Write-Fail "pirouter.exe not found at $BinaryPath — run .\scripts\build.ps1 first"
}

if ($ConfigPath -eq "") {
    $ConfigPath = Join-Path $env:APPDATA "pirouter\config.toml"
}
if (-not (Test-Path $ConfigPath)) {
    Write-Host "  WARNING: config not found at $ConfigPath" -ForegroundColor Yellow
    Write-Host "           The service will fail to start until a valid config.toml is placed there." -ForegroundColor Yellow
    Write-Host "           Copy and edit config.example.toml:" -ForegroundColor Yellow
    Write-Host "             New-Item -Force -ItemType Directory `"$env:APPDATA\pirouter`"" -ForegroundColor DarkGray
    Write-Host "             Copy-Item config.example.toml `"$ConfigPath`"" -ForegroundColor DarkGray
}

if ($LogDir -eq "") {
    $LogDir = Join-Path $env:ProgramData "pirouter\logs"
}
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

# ── find or download nssm ────────────────────────────────────────────────────

Write-Step "Locating NSSM"

if ($NssmPath -ne "" -and (Test-Path $NssmPath)) {
    Write-Ok "Using supplied nssm: $NssmPath"
} else {
    $NssmPath = (Get-Command nssm.exe -ErrorAction SilentlyContinue)?.Source
    if ($NssmPath) {
        Write-Ok "Found nssm on PATH: $NssmPath"
    } else {
        Write-Host "    nssm not found — downloading portable nssm 2.24 ..." -ForegroundColor Yellow
        $NssmDir  = Join-Path $env:TEMP "nssm-2.24"
        $NssmZip  = Join-Path $env:TEMP "nssm-2.24.zip"
        $NssmUrl  = "https://nssm.cc/release/nssm-2.24.zip"
        try {
            Invoke-WebRequest -Uri $NssmUrl -OutFile $NssmZip -UseBasicParsing
            Expand-Archive -Path $NssmZip -DestinationPath $env:TEMP -Force
            # nssm ships win64 and win32 sub-dirs
            $NssmPath = Join-Path $NssmDir "win64\nssm.exe"
            if (-not (Test-Path $NssmPath)) {
                $NssmPath = Join-Path $NssmDir "win32\nssm.exe"
            }
        } catch {
            Write-Fail "Could not download nssm: $_`nInstall nssm manually (https://nssm.cc) and re-run with -NssmPath"
        }
        Write-Ok "Downloaded nssm: $NssmPath"
    }
}

# ── remove existing service if present ───────────────────────────────────────

Write-Step "Checking for existing '$ServiceName' service"
$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existing) {
    Write-Host "    Service exists (status: $($existing.Status)) — removing first ..." -ForegroundColor Yellow
    if ($existing.Status -eq "Running") {
        & $NssmPath stop $ServiceName confirm 2>&1 | Out-Null
    }
    & $NssmPath remove $ServiceName confirm 2>&1 | Out-Null
    Write-Ok "Removed existing service"
}

# ── install ───────────────────────────────────────────────────────────────────

Write-Step "Installing service '$ServiceName'"

# Base install
& $NssmPath install $ServiceName $BinaryPath
if ($LASTEXITCODE -ne 0) { Write-Fail "nssm install failed" }

# Arguments: `run --config <path>`
& $NssmPath set $ServiceName AppParameters "--config `"$ConfigPath`" run"

# Restart on failure
& $NssmPath set $ServiceName AppExit Default Restart
& $NssmPath set $ServiceName AppRestartDelay 5000   # 5s back-off

# Auto-start
& $NssmPath set $ServiceName Start SERVICE_AUTO_START

# Log output rotation (64 MB cap, keep 10 files)
& $NssmPath set $ServiceName AppStdout         (Join-Path $LogDir "pirouter.log")
& $NssmPath set $ServiceName AppStderr         (Join-Path $LogDir "pirouter-err.log")
& $NssmPath set $ServiceName AppRotateFiles    1
& $NssmPath set $ServiceName AppRotateBytes    67108864   # 64 MB
& $NssmPath set $ServiceName AppRotateOnline   1

# Display name & description
& $NssmPath set $ServiceName DisplayName "pirouter LLM Router"
& $NssmPath set $ServiceName Description  "Lightweight OpenAI-compatible LLM routing daemon (pirouter)"

Write-Ok "Service registered"

# ── start ────────────────────────────────────────────────────────────────────

Write-Step "Starting service"
& $NssmPath start $ServiceName
Start-Sleep -Seconds 2
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($svc -and $svc.Status -eq "Running") {
    Write-Ok "Service is running"
} else {
    Write-Host "    Service did not start — check config and logs:" -ForegroundColor Yellow
    Write-Host "      $LogDir" -ForegroundColor DarkGray
}

# ── summary ───────────────────────────────────────────────────────────────────

Write-Step "Install complete"
Write-Host ""
Write-Host "  Service name : $ServiceName" -ForegroundColor White
Write-Host "  Binary       : $BinaryPath" -ForegroundColor White
Write-Host "  Config       : $ConfigPath" -ForegroundColor White
Write-Host "  Logs         : $LogDir" -ForegroundColor White
Write-Host ""
Write-Host "  Manage with:" -ForegroundColor Gray
Write-Host "    Start-Service $ServiceName" -ForegroundColor DarkGray
Write-Host "    Stop-Service  $ServiceName" -ForegroundColor DarkGray
Write-Host "    Get-Service   $ServiceName" -ForegroundColor DarkGray
Write-Host "    nssm edit     $ServiceName   (GUI)" -ForegroundColor DarkGray
Write-Host ""
Write-Host "  Health check (once running):" -ForegroundColor Gray
Write-Host "    Invoke-RestMethod http://127.0.0.1:11435/healthz" -ForegroundColor DarkGray
Write-Host ""
