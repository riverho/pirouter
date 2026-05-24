#Requires -RunAsAdministrator
#Requires -Version 5.1
<#
.SYNOPSIS
    Stop and remove the pirouter Windows Service.

.PARAMETER ServiceName
    Service name to remove.  Default: pirouter

.PARAMETER NssmPath
    Path to nssm.exe.  Auto-detected from PATH if omitted.

.PARAMETER RemoveLogs
    Also delete the log directory under ProgramData.
#>
param(
    [string]$ServiceName = "pirouter",
    [string]$NssmPath    = "",
    [switch]$RemoveLogs
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Step([string]$msg) { Write-Host "`n==> $msg" -ForegroundColor Cyan }
function Write-Ok([string]$msg)   { Write-Host "    OK  $msg" -ForegroundColor Green }
function Write-Fail([string]$msg) { Write-Host "    ERR $msg" -ForegroundColor Red; exit 1 }

if ($NssmPath -eq "") {
    $NssmPath = (Get-Command nssm.exe -ErrorAction SilentlyContinue)?.Source
}
if (-not $NssmPath -or -not (Test-Path $NssmPath)) {
    Write-Fail "nssm.exe not found. Pass -NssmPath or put nssm on PATH."
}

Write-Step "Stopping '$ServiceName'"
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if (-not $svc) {
    Write-Host "    Service '$ServiceName' not found — nothing to do." -ForegroundColor Yellow
    exit 0
}
if ($svc.Status -eq "Running") {
    & $NssmPath stop $ServiceName confirm 2>&1 | Out-Null
    Write-Ok "Stopped"
}

Write-Step "Removing '$ServiceName'"
& $NssmPath remove $ServiceName confirm 2>&1 | Out-Null
Write-Ok "Service removed"

if ($RemoveLogs) {
    $LogDir = Join-Path $env:ProgramData "pirouter\logs"
    if (Test-Path $LogDir) {
        Remove-Item -Recurse -Force $LogDir
        Write-Ok "Logs removed: $LogDir"
    }
}

Write-Host "`n  Done. Config and binary are untouched.`n" -ForegroundColor White
