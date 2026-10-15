$ErrorActionPreference='Stop'
$own=$PSScriptRoot
$basis='E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-findings-surfaces-integration-v1'
if(Test-Path -LiteralPath (Join-Path $own 'manifest.json')){throw 'Immutable manifest exists'}
function FilePin([string]$p){@{sha256=(Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant();bytes=(Get-Item -LiteralPath $p).Length}}
$files=@()
foreach($relative in @('web/app.js','web/index.html','web/styles.css','web/list-import.js','web/saved-comparison.js','web/verification/ui-sitemap-findings-smoke.cjs')){
  $before=Join-Path $basis $relative;$after=Join-Path $own $relative;$exists=Test-Path -LiteralPath $before
  $b=if($exists){FilePin $before}else{$null};$a=FilePin $after
  $role=if($relative -in @('web/list-import.js','web/saved-comparison.js')){'unchanged_browser_dependency'}elseif($exists){'guarded_product_replacement'}else{'new_authored_browser_fixture'}
  if($role -eq 'unchanged_browser_dependency' -and $b.sha256 -ne $a.sha256){throw 'Unchanged companion differs'}
  $files+=@{path=$relative;role=$role;before_sha256=if($exists){$b.sha256}else{$null};before_absent=(!$exists);after_sha256=$a.sha256;bytes=$a.bytes}
}
$companions=@();foreach($relative in @('HANDOFF.md','syntax-checks.json','findings-view.js','findings-view.html','findings-view.css','prepare-payloads.ps1','freeze-manifest.ps1')){$pin=FilePin (Join-Path $own $relative);$companions+=@{path=$relative;sha256=$pin.sha256;bytes=$pin.bytes}}
$inputs=@();foreach($p in @((Join-Path $basis 'crates/core/src/sitemap_findings.rs'),(Join-Path $basis 'crates/core/src/store/sitemap_findings_read.rs'),(Join-Path $basis 'crates/app/src/sitemap_findings.rs'),'E:/CodexWork/open-seo-spider-01a12ac3-20261013/s6-app-adapter-core-v1/manifest.json','E:/CodexWork/open-seo-spider-01a12ac3-20261013/s6-ui-surface-audit-v1/receipt.json')){$pin=FilePin $p;$inputs+=@{path=$p;sha256=$pin.sha256;bytes=$pin.bytes}}
$manifest=[ordered]@{schema_version=1;status='IMMUTABLE_SOURCE_ONLY';basis_root=$basis;basis_commit_supplied_by_root='898c404b596cdb812fd76c19a0a38de5d155371c';ownership='Three additive web payloads; one new authored UI fixture; unchanged static companions only';files=$files;authoring_companions=$companions;actual_inputs=$inputs;authored_browser_groups=13;executed_browser_groups=0;checks=@{node_syntax='PASS';compiler='NOT_RUN';browser='NOT_RUN';native='NOT_RUN';http='NOT_RUN';sqlite='NOT_RUN';tests='NOT_RUN'};accepted_feature_count_change=0;installed_runtime_modified=$false;root_product_modified=$false;scope_limits=@('UI fixture is mocked API only, not native predicate/CSV proof','Declared64MiB+1 short-body control is not physical64MiB stream proof','Responsive/keyboard behavior authored, not observed','GET available server-side; UI deliberately always POSTs literal selections')}
$manifest | ConvertTo-Json -Depth 14 | Set-Content -LiteralPath (Join-Path $own 'manifest.json') -Encoding utf8
Get-FileHash -LiteralPath (Join-Path $own 'manifest.json') -Algorithm SHA256
