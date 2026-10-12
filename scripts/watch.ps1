<#
.SYNOPSIS
    Aura Hot Reload Watcher for Windows / PowerShell.
.DESCRIPTION
    Monitors `src/` and `Cargo.toml` for file modifications, recompiling
    and restarting Aura on every save.
    Uses `cargo-watch` if installed, otherwise seamlessly falls back to
    a native .NET FileSystemWatcher loop.
#>

[CmdletBinding()]
param(
    [switch]$ForceNativeFallback
)

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

Write-Host "=================================================" -ForegroundColor Cyan
Write-Host "   AURA REST CLIENT - HOT RELOAD WATCHER         " -ForegroundColor Cyan
Write-Host "=================================================" -ForegroundColor Cyan

# Check if cargo-watch is available
$hasCargoWatch = $false
if (-not $ForceNativeFallback) {
    try {
        $null = Get-Command "cargo-watch" -ErrorAction Stop
        $hasCargoWatch = $true
    } catch {
        # Check through cargo invocation
        $cargoList = cargo --list 2>$null
        if ($cargoList -match '\bwatch\b') {
            $hasCargoWatch = $true
        }
    }
}

if ($hasCargoWatch) {
    Write-Host "[Aura Hot Reload] Detected 'cargo-watch'. Launching watch daemon..." -ForegroundColor Green
    cargo watch -c -w src -w Cargo.toml -x "run --bin aura"
    exit $LASTEXITCODE
}

Write-Host "[Aura Hot Reload] 'cargo-watch' not found in PATH." -ForegroundColor Yellow
Write-Host "[Aura Hot Reload] Starting built-in PowerShell FileSystemWatcher engine..." -ForegroundColor Yellow
Write-Host "[Tip] You can install cargo-watch anytime via: cargo install cargo-watch`n" -ForegroundColor DarkGray

$WatchPaths = @(
    (Join-Path $RepoRoot "src"),
    (Join-Path $RepoRoot "Cargo.toml")
)

$script:currentProcess = $null

function Stop-AuraProcess {
    if ($script:currentProcess -and -not $script:currentProcess.HasExited) {
        Write-Host "[Aura Hot Reload] Stopping running instance (PID: $($script:currentProcess.Id))..." -ForegroundColor DarkYellow
        try {
            # Kill process tree on Windows
            Start-Process -FilePath "taskkill.exe" -ArgumentList "/PID $($script:currentProcess.Id) /T /F" -NoNewWindow -Wait -ErrorAction SilentlyContinue | Out-Null
        } catch {}
        try {
            $script:currentProcess.Kill()
        } catch {}
        $script:currentProcess = $null
    }
}

function Start-AuraProcess {
    Stop-AuraProcess
    Write-Host "[Aura Hot Reload] [$(Get-Date -Format 'HH:mm:ss')] Building & starting Aura..." -ForegroundColor Cyan
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "cargo"
    $psi.Arguments = "run --bin aura"
    $psi.WorkingDirectory = $RepoRoot.Path
    $psi.UseShellExecute = $false

    $script:currentProcess = [System.Diagnostics.Process]::Start($psi)
}

# Start initial run
Start-AuraProcess

# Setup FileSystemWatcher
$fsw = New-Object System.IO.FileSystemWatcher
$fsw.Path = (Join-Path $RepoRoot "src")
$fsw.IncludeSubdirectories = $true
$fsw.EnableRaisingEvents = $true
$fsw.NotifyFilter = [System.IO.NotifyFilters]'FileName, LastWrite, Size'

$lastChange = [DateTime]::MinValue
$debounceMs = 400

try {
    Write-Host "[Aura Hot Reload] Watching for changes in '$($fsw.Path)' (Press Ctrl+C to stop)...`n" -ForegroundColor Green

    while ($true) {
        $change = $fsw.WaitForChanged([System.IO.WatcherChangeTypes]::All, 500)
        if (-not $change.TimedOut) {
            $now = [DateTime]::Now
            if (($now - $lastChange).TotalMilliseconds -gt $debounceMs) {
                $lastChange = $now
                Write-Host "`n[Aura Hot Reload] File changed: $($change.Name) ($($change.ChangeType))" -ForegroundColor Magenta
                Start-AuraProcess
            }
        }
    }
} finally {
    Write-Host "`n[Aura Hot Reload] Shutting down watcher..." -ForegroundColor Yellow
    $fsw.EnableRaisingEvents = $false
    $fsw.Dispose()
    Stop-AuraProcess
}
