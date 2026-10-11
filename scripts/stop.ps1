$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$pidPath = Join-Path $projectRoot 'data\server.pid'
if (-not (Test-Path -LiteralPath $pidPath)) { Write-Host 'No recorded Open SEO Spider process.'; exit 0 }
$serverProcessId = [int](Get-Content -LiteralPath $pidPath -Raw).Trim()
$serverProcess = Get-Process -Id $serverProcessId -ErrorAction SilentlyContinue
if ($serverProcess) {
    $expectedBinary = Join-Path $projectRoot 'target\release\open-seo-spider.exe'
    if ($serverProcess.Path -ne $expectedBinary) { throw 'The recorded process belongs to another program; refusing to stop it.' }
    Stop-Process -Id $serverProcessId
}
Remove-Item -LiteralPath $pidPath
Write-Host 'Open SEO Spider stopped. Interrupted crawls can be resumed after restart.'
& (Join-Path $PSScriptRoot 'stop-renderer.ps1')
