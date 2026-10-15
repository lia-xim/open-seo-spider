$ErrorActionPreference='Stop'
$own=$PSScriptRoot
$basis='E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-findings-surfaces-integration-v1'
function Patch([string]$old,[string]$new) {
  if(($script:source.Split(@($old),[StringSplitOptions]::None).Count-1) -ne 1){throw ('Expected one insertion: '+$old)}
  $script:source=$script:source.Replace($old,$new)
}
$source=[IO.File]::ReadAllText((Join-Path $basis 'web/app.js'))
Patch '    resetResourceLedger();' "    resetResourceLedger();`r`n    resetSitemapFindings();"
Patch 'resetResourceLedger(); resetSitemapMembership(); resetSitemapBundle();' 'resetResourceLedger(); resetSitemapMembership(); resetSitemapBundle(); resetSitemapFindings();'
Patch '      if (sitemapSection() && !sitemapMembership.result && !sitemapMembership.token) loadSitemapPage();' @'
      if (sitemapSection() && !sitemapMembership.result && !sitemapMembership.token) loadSitemapPage();
      if (findingsSection() && !sitemapFindings.result && !sitemapFindings.token && !sitemapFindings.queryDirty && !sitemapFindings.error) loadSitemapFindings();
'@
Patch "    inputs: ['EINGABEHERKUNFT'" "    'sitemap-findings': ['GESPEICHERTE SITEMAP-BEFUNDE', 'Sitemap-Befunde', 'Alle gespeicherten Kandidaten nach Originalidentität prüfen. Messgrenzen, unbekannte Historie und Zielbefunde bleiben getrennt.'],`r`n    inputs: ['EINGABEHERKUNFT'"
Patch '    state.section = section;' "    if (findingsSection() && section !== 'sitemap-findings') resetSitemapFindings();`r`n    state.section = section;"
Patch '    if (sitemapSection() && state.crawl && !sitemapMembership.result && !sitemapMembership.token) loadSitemapPage();' @'
    if (sitemapSection() && state.crawl && !sitemapMembership.result && !sitemapMembership.token) loadSitemapPage();
    if (findingsSection() && state.crawl && !sitemapFindings.result && !sitemapFindings.token && !sitemapFindings.queryDirty) loadSitemapFindings();
'@
Patch "['overview', 'graph', 'content','resource-discovery','sitemap-membership'].includes(state.section)" "['overview', 'graph', 'content','resource-discovery','sitemap-membership','sitemap-findings'].includes(state.section)"
Patch '    $(''sitemap-membership-view'').hidden = !sitemapSection();' @'
    $('sitemap-membership-view').hidden = !sitemapSection();
    $('sitemap-findings-view').hidden = !findingsSection();
'@
Patch '    else if (sitemapSection()) renderSitemapMembership();' "    else if (sitemapSection()) renderSitemapMembership();`r`n    else if (findingsSection()) renderSitemapFindings();"
$needle="state.section==='resource-discovery' || sitemapSection()"
if(($source.Split(@($needle),[StringSplitOptions]::None).Count-1) -ne 3){throw 'Expected3 generic export fences'}
$source=$source.Replace($needle,$needle+' || findingsSection()')
Patch '  const sitemapMembership = {' (([IO.File]::ReadAllText((Join-Path $own 'findings-view.js')))+"`r`n  const sitemapMembership = {")
Patch '    bindSitemapEvents();' "    bindSitemapEvents();`r`n    bindSitemapFindingsEvents();"
[IO.File]::WriteAllText((Join-Path $own 'web/app.js'),$source,[Text.UTF8Encoding]::new($false))
$source=[IO.File]::ReadAllText((Join-Path $basis 'web/index.html'))
Patch '        <section id="sitemap-membership-view"' (([IO.File]::ReadAllText((Join-Path $own 'findings-view.html')))+"`r`n        <section id=`"sitemap-membership-view`"")
Patch '        <button data-section="sitemap-membership"' @'
        <button data-section="sitemap-findings" class="nav-item"><span class="nav-symbol" aria-hidden="true">✓</span>Sitemap-Befunde</button>
        <button data-section="sitemap-membership"
'@
[IO.File]::WriteAllText((Join-Path $own 'web/index.html'),$source,[Text.UTF8Encoding]::new($false))
$source=[IO.File]::ReadAllText((Join-Path $basis 'web/styles.css'))+[IO.File]::ReadAllText((Join-Path $own 'findings-view.css'))
[IO.File]::WriteAllText((Join-Path $own 'web/styles.css'),$source,[Text.UTF8Encoding]::new($false))
