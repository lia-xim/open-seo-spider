param([int]$Port = 8790)
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$rendererPath = Join-Path $projectRoot 'renderer'
$dataPath = Join-Path $projectRoot 'data'
try { $health = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/health" -TimeoutSec 2 } catch { $health = $null }
if ($health) { if ($health.worker -ne 'open-seo-spider-renderer') { throw 'This port is used by another program.' }; Write-Host "Browser worker already runs at 127.0.0.1:$Port"; exit 0 }
if (-not (Test-Path -LiteralPath (Join-Path $rendererPath 'node_modules\playwright-core'))) { throw 'Run npm ci inside renderer/ before starting the optional browser worker.' }
$nodePath = (Get-Command node -ErrorAction Stop).Source
New-Item -ItemType Directory -Path $dataPath -Force | Out-Null
$env:OPEN_SEO_RENDERER_PORT = "$Port"
$workerProcess = Start-Process -FilePath $nodePath -ArgumentList @((Join-Path $rendererPath 'server.mjs')) -WorkingDirectory $rendererPath -WindowStyle Hidden -RedirectStandardOutput (Join-Path $dataPath 'renderer.log') -RedirectStandardError (Join-Path $dataPath 'renderer-error.log') -PassThru
Set-Content -LiteralPath (Join-Path $dataPath 'renderer.pid') -Value $workerProcess.Id
for ($attempt = 0; $attempt -lt 40; $attempt++) {
    if ($workerProcess.HasExited) { throw 'Browser worker exited; read data/renderer-error.log.' }
    try { $health = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/health" -TimeoutSec 1; if ($health.status -eq 'ok') { break } } catch { }
    Start-Sleep -Milliseconds 250
}
if ($health.status -ne 'ok') { throw 'Browser worker did not become ready.' }
Write-Host "Browser worker: http://127.0.0.1:$Port"
