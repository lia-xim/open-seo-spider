param([int]$Port = 8790)
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$pidPath = Join-Path $projectRoot 'data\renderer.pid'
if (-not (Test-Path -LiteralPath $pidPath)) { exit 0 }
$workerProcessId = [int](Get-Content -LiteralPath $pidPath -Raw).Trim()
$workerProcess = Get-CimInstance Win32_Process -Filter "ProcessId = $workerProcessId"
if ($workerProcess) {
    $workerScript = Join-Path $projectRoot 'renderer\server.mjs'
    if (-not $workerProcess.CommandLine.Contains($workerScript)) { throw 'Recorded PID belongs to another program; refusing to stop it.' }
    $health = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/health" -TimeoutSec 2
    if ($health.worker -ne 'open-seo-spider-renderer') { throw 'The worker port belongs to another application.' }
    Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$Port/shutdown" -TimeoutSec 10 | Out-Null
    Wait-Process -Id $workerProcessId -Timeout 15 -ErrorAction SilentlyContinue
    if (Get-Process -Id $workerProcessId -ErrorAction SilentlyContinue) { throw 'Worker still running; inspect renderer logs.' }
}
Remove-Item -LiteralPath $pidPath
Write-Host 'Browser worker stopped.'
