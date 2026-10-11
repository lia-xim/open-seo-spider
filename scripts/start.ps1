param([int]$Port = 8787, [switch]$NoBrowser)
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$dataPath = Join-Path $projectRoot 'data'
$binaryPath = Join-Path $projectRoot 'target\release\open-seo-spider.exe'
$healthUrl = "http://127.0.0.1:$Port/healthz"
try {
    $health = Invoke-RestMethod -Uri $healthUrl -TimeoutSec 2
} catch { $health = $null }
if ($health) {
    if ($health.name -ne 'Open SEO Spider') { throw 'This port is used by another application.' }
    if (Test-Path -LiteralPath (Join-Path $projectRoot 'renderer\node_modules\playwright-core')) { & (Join-Path $PSScriptRoot 'start-renderer.ps1') }
    Write-Host "Open SEO Spider already runs at http://127.0.0.1:$Port"
    if (-not $NoBrowser) { Start-Process "http://127.0.0.1:$Port" }
    exit 0
}
if (-not (Test-Path -LiteralPath $binaryPath)) {
    $cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
    $cargoPath = if ($cargoCommand) { $cargoCommand.Source } else { Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe' }
    if (-not (Test-Path -LiteralPath $cargoPath)) { throw 'Rust Cargo is required for the first build.' }
    Push-Location $projectRoot
    try { & $cargoPath build --release --locked -p open-seo-spider; if ($LASTEXITCODE -ne 0) { throw 'Build failed.' } }
    finally { Pop-Location }
}
New-Item -ItemType Directory -Path $dataPath -Force | Out-Null
if (Test-Path -LiteralPath (Join-Path $projectRoot 'renderer\node_modules\playwright-core')) { & (Join-Path $PSScriptRoot 'start-renderer.ps1') }
$serverProcess = Start-Process -FilePath $binaryPath -ArgumentList @('serve', '--listen', "127.0.0.1:$Port", '--data-dir', 'data') -WorkingDirectory $projectRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $dataPath 'server.log') -RedirectStandardError (Join-Path $dataPath 'server-error.log') -PassThru
Set-Content -LiteralPath (Join-Path $dataPath 'server.pid') -Value $serverProcess.Id
for ($attempt = 0; $attempt -lt 40; $attempt++) {
    if ($serverProcess.HasExited) { throw 'Server exited. Read data/server-error.log.' }
    try { $health = Invoke-RestMethod -Uri $healthUrl -TimeoutSec 1; if ($health.ok) { break } } catch { }
    Start-Sleep -Milliseconds 250
}
if (-not $health.ok) { throw 'Server did not become ready. Read data/server-error.log.' }
Write-Host "Open SEO Spider: http://127.0.0.1:$Port"
if (-not $NoBrowser) { Start-Process "http://127.0.0.1:$Port" }
