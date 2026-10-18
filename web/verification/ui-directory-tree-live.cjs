// Actual redesigned browser -> native saved report -> offline hierarchy -> verified export.
'use strict';
const assert = require('node:assert/strict');
const http = require('node:http');
const fs = require('node:fs');
const {createHash, randomUUID} = require('node:crypto');
const {chromium} = require('../../renderer/node_modules/playwright-core');
const artifactPath = require('./artifact-path.cjs');
const app = process.env.SPIDER_APP_URL;
assert(app && new URL(app).hostname === '127.0.0.1' && !['8787','8790'].includes(new URL(app).port), 'Use an isolated native E2E app');
const checks = [], errors = [], hits = new Map();
const browserEvidence = {main_frame_navigations:[],request_failures:[],response_captures:[]};
const count = () => [...hits.values()].reduce((a,b) => a+b, 0);
const fixture = http.createServer((req,res) => {
  hits.set(req.url, (hits.get(req.url) || 0)+1);
  if (req.url === '/old') { res.writeHead(301,{location:'/docs/p000.html'}); return res.end(); }
  if (req.url === '/missing') res.statusCode = 404;
  res.setHeader('content-type','text/html; charset=utf-8');
  res.end(`<!doctype html><html lang="en"><head><title>Owned directory fixture ${req.url}</title>${req.url === '/secret/hidden.html' ? '<meta name="robots" content="noindex">' : ''}</head><body><main>Saved directory fixture.</main></body></html>`);
});
async function api(path, body, expected = 200) {
  const response = await fetch(app+path,{method:body === undefined ? 'GET' : 'POST',headers:body === undefined ? {} : {'content-type':'application/json'},body:body === undefined ? undefined : JSON.stringify(body)});
  assert.equal(response.status,expected,await response.clone().text());
  return response.json();
}
async function until(work,label) {
  for (let i=0; i<600; i++) { const value=await work(); if(value)return value; await new Promise(r=>setTimeout(r,50)); }
  throw Error('Timeout: '+label);
}
function csvRows(source) {
  const rows=[];let row=[],cell='',quoted=false;
  for(let i=0;i<source.length;i++) {
    const c=source[i];
    if(c==='"'&&quoted&&source[i+1]==='"'){cell+='"';i++;}
    else if(c==='"')quoted=!quoted;
    else if(c===','&&!quoted){row.push(cell);cell='';}
    else if(c==='\r'&&!quoted){assert.equal(source[++i],'\n');row.push(cell);rows.push(row);row=[];cell='';}
    else cell+=c;
  }
  assert.equal(quoted,false);assert.equal(cell,'');assert.equal(row.length,0);
  const header=rows.shift();return rows.map(cells=>{assert.equal(cells.length,header.length);return Object.fromEntries(header.map((h,i)=>[h,cells[i]]));});
}
function verified(bytes, headers) {
  assert.equal(Number(headers['content-length']),bytes.length);
  assert.equal(headers['x-content-sha256'],createHash('sha256').update(bytes).digest('hex'));
  assert.equal(headers['cache-control'],'no-store');
}
async function assertRenderedPage(page,value) {
  const rendered=await page.locator('#directory-children article').evaluateAll(nodes=>nodes.map(node=>({
    label:node.querySelector('h3').textContent,
    address:node.querySelector('.address').textContent,
    observed:node.classList.contains('observed'),
    navigable:node.querySelector('h3 button')!==null,
    metrics:[...node.querySelectorAll('.metrics')].map(item=>item.textContent)
  })));
  assert.deepEqual(rendered,value.children.map(node=>({
    label:node.label,address:node.address||'Alle gespeicherten URL-Pfade',
    observed:!node.synthetic_group,navigable:node.synthetic_group,
    metrics:[
      `${node.counts.unique_urls} URLs · ${node.counts.observations} Beobachtungen · ${node.child_count} direkte Kinder`,
      `HTTP2xx: ${node.counts.status_2xx} · 3xx: ${node.counts.status_3xx} · 4xx: ${node.counts.status_4xx} · 5xx: ${node.counts.status_5xx} · sonstige: ${node.counts.status_other} · unbeobachtet: ${node.counts.status_unobserved}`,
      `Gespeicherte Indexierbarkeit: ${node.counts.stored_indexable} ja / ${node.counts.stored_non_indexable} nein / ${node.counts.indexability_unobserved} unbekannt`
    ]
  })),'Every displayed card must represent the current native page in order');
  assert.deepEqual(await page.locator('#breadcrumbs button').allTextContents(),value.ancestors.map(node=>node.label),'Breadcrumbs must match the current hierarchy');
  assert.equal(await page.locator('#current-label').innerText(),value.current.label);
  assert.equal(await page.locator('#page-state').innerText(),`Kinder ${value.total_children?value.offset+1:0}–${value.offset+value.children.length} von ${value.total_children}.`);
  assert.equal(await page.isVisible('#empty-node'),value.children.length===0);
  assert.equal(await page.isDisabled('#previous-page'),value.offset===0);
  assert.equal(await page.isDisabled('#next-page'),value.next_offset===null);
}
function captureResponse(page,predicate,label) {
  // Start CDP body retrieval at the response event, before Playwright waits for
  // a click's post-action work. Retain one buffer for diagnostics, parsing and
  // integrity checks; a successful status must not eagerly call text() again.
  return page.waitForResponse(predicate).then(async response=>{
    const record={label,url:response.url(),method:response.request().method(),resource_type:response.request().resourceType(),status:response.status(),page_url_at_headers:page.url(),body_bytes:null,body_sha256:null,error:null};
    browserEvidence.response_captures.push(record);
    try {
      const bytes=await response.body(),headers=await response.allHeaders();
      record.body_bytes=bytes.length;record.body_sha256=createHash('sha256').update(bytes).digest('hex');record.page_url_after_body=page.url();
      return {response,bytes,headers};
    } catch(error) {record.error=String(error);record.page_url_after_body=page.url();throw error;}
  });
}
function assertNoNavigation(page,before,label) {
  assert.equal(page.url(),before.url,label+' must preserve the current document URL');
  assert.equal(browserEvidence.main_frame_navigations.length,before.navigations,label+' must not navigate or reload the document');
}
async function preview(page,click,expected=200) {
  const before={url:page.url(),navigations:browserEvidence.main_frame_navigations.length};
  const pending=captureResponse(page,r=>r.url().endsWith('/directory-tree/preview')&&r.request().method()==='POST','directory-preview');
  const [{response,bytes,headers}]=await Promise.all([pending,Promise.resolve().then(click)]);
  assert.equal(response.status(),expected,bytes.toString('utf8'));
  assertNoNavigation(page,before,'Directory preview');
  if(expected!==200)return JSON.parse(bytes);
  verified(bytes,headers);const value=JSON.parse(bytes);
  await page.waitForFunction(label=>document.getElementById('current-label').textContent===label&&!document.getElementById('open-tree').disabled,value.current.label);
  assert.equal(value.website_requests,0);await assertRenderedPage(page,value);return value;
}
async function open(page,id,mode='') {
  await page.goto(app+'/directory-tree');
  await until(()=>page.locator(`#saved-crawl option[value="${id}"]`).count(),'saved crawl option');
  await page.selectOption('#saved-crawl',id);await page.selectOption('#directory-mode',mode);
  return preview(page,()=>page.click('#open-tree'));
}
async function descend(page,current,find) {
  const node=current.children.find(find);assert(node,'Expected child node');assert.equal(node.synthetic_group,true);
  return preview(page,()=>page.locator('#directory-children article').filter({has:page.locator('.address',{hasText:node.address})}).locator('h3 button').click());
}
async function download(page) {
  const before={url:page.url(),navigations:browserEvidence.main_frame_navigations.length};
  const pending=captureResponse(page,r=>r.url().endsWith('/directory-tree/csv')&&r.request().method()==='POST','directory-csv'),saving=page.waitForEvent('download');
  const [{response,bytes,headers},file]=await Promise.all([pending,saving,page.click('#export-csv')]);
  assert.equal(response.status(),200,bytes.toString('utf8'));verified(bytes,headers);assertNoNavigation(page,before,'Directory CSV');
  assert.equal(file.suggestedFilename(),'directory-tree.csv');assert.deepEqual(fs.readFileSync(await file.path()),bytes);
  await page.waitForFunction(()=>document.getElementById('directory-status').textContent.includes('Prüfsumme bestätigt'));
  return {bytes,rows:csvRows(bytes.toString('utf8')),request:response.request().postDataJSON()};
}
async function importReport(page,report,name) {
  await page.goto(app);await page.locator('#connection-state.connected').waitFor();
  const before={url:page.url(),navigations:browserEvidence.main_frame_navigations.length};
  const pending=captureResponse(page,r=>r.url()===app+'/api/crawls/import'&&r.request().method()==='POST','project-import');
  const [{response,bytes}]=await Promise.all([pending,page.setInputFiles('#import-file',{name,mimeType:'application/json',buffer:Buffer.from(JSON.stringify(report))})]);
  assert.equal(response.status(),200,bytes.toString('utf8'));assertNoNavigation(page,before,'Project import');return JSON.parse(bytes).id;
}
(async()=>{
  await new Promise(resolve=>fixture.listen(0,'127.0.0.1',resolve));
  const origin=`http://127.0.0.1:${fixture.address().port}`;
  const documents=Array.from({length:105},(_,i)=>`${origin}/docs/p${String(i).padStart(3,'0')}.html`);
  const started=await api('/api/crawls',{seed_urls:[...documents,origin+'/old',origin+'/missing',origin+'/secret/hidden.html'],crawl_links:false,discover_sitemaps:false,check_external:false,obey_robots:false,delay_ms:0,max_pages:200,concurrency:12,directive_reporting:{version:1,respect_noindex:true,respect_canonical:false,respect_next_prev:false}});
  await until(async()=>{const job=await api('/api/crawls/'+started.id);assert.notEqual(job.status,'failed',job.error);return job.status==='completed';},'owned native directory crawl');
  const savedResponse=await fetch(app+`/api/crawls/${started.id}/project.json`),savedBytes=await savedResponse.text(),report=JSON.parse(savedBytes);
  assert.equal(report.pages.length,108);assert.equal(report.pages.filter(p=>p.status===301).length,1);assert.equal(report.pages.filter(p=>p.status===404).length,1);
  const capturedHits=count();
  const executablePath=process.env.SPIDER_CHROME||process.env.OPEN_SEO_BROWSER_PATH||await(await import('../../renderer/server.mjs')).findBrowserExecutable();
  const browser=await chromium.launch({headless:true,executablePath}),page=await browser.newPage({viewport:{width:1440,height:900}});
  page.on('pageerror',error=>errors.push(error.message));
  page.on('framenavigated',frame=>{if(frame===page.mainFrame())browserEvidence.main_frame_navigations.push({url:frame.url(),at:new Date().toISOString()});});
  page.on('requestfailed',request=>browserEvidence.request_failures.push({url:request.url(),method:request.method(),resource_type:request.resourceType(),navigation:request.isNavigationRequest(),failure:request.failure()?.errorText||null,page_url:page.url()}));
  let emptyId,sparseId;
  try {
    const root=await open(page,started.id);
    assert.equal(root.reporting_view.identity.mode,'reported');assert.equal(root.reporting_view.captured_page_observations,108);assert.equal(root.reporting_view.profile_excluded_page_observations,1);
    assert.equal(root.current.kind,'root');assert.equal(root.current.synthetic_group,true);assert.equal(root.current.counts.observations,107);
    assert.equal(root.current.counts.status_2xx,105);assert.equal(root.current.counts.status_3xx,1);assert.equal(root.current.counts.status_4xx,1);assert.equal(root.current.counts.status_unobserved,0);
    assert.equal(root.current.counts.stored_indexable,105);assert.equal(root.current.counts.indexability_unobserved,2);
    assert.match(await page.locator('#counts-note').innerText(),/Eine Weiterleitung übernimmt nicht den Status des Zieles/);
    checks.push('Actual 108-page native crawl: automatic saved noindex profile excludes exactly one captured row; requested redirect/404 statuses remain distinct and their indexability remains unknown');
    let group=await descend(page,root,n=>n.kind==='protocol');
    group=await descend(page,group,n=>n.kind==='origin');
    group=await descend(page,group,n=>n.kind==='directory'&&n.address===origin+'/docs/');
    assert.equal(group.total_children,105);assert.equal(group.children.length,100);assert.equal(group.next_offset,100);assert.equal(group.current.counts.unique_urls,105);
    assert.equal(await page.locator('#directory-children article').count(),100);assert.equal(await page.isDisabled('#previous-page'),true);assert.equal(await page.isDisabled('#next-page'),false);
    const firstIds=group.children.map(n=>n.id);assert(group.children.every(n=>n.kind==='observed_url'&&!n.synthetic_group));
    const second=await preview(page,()=>page.click('#next-page'));
    assert.equal(second.offset,100);assert.equal(second.children.length,5);assert.equal(second.next_offset,null);assert.equal(await page.isDisabled('#next-page'),true);assert.equal(await page.isDisabled('#previous-page'),false);
    assert.equal(new Set([...firstIds,...second.children.map(n=>n.id)]).size,105);assert.equal(second.children.at(-1).address,documents.at(-1));
    const back=await preview(page,()=>page.click('#previous-page'));assert.deepEqual(back.children.map(n=>n.id),firstIds);
    const ancestor=await preview(page,()=>page.locator('#breadcrumbs button').first().click());assert.equal(ancestor.current.id,root.root_id);
    checks.push('Every actual preview card has the native page address/label/status/indexability counts and grouping action; hierarchy breadcrumbs and105 siblings page100+5 retain stable disjoint displayed ordering, previous/next boundaries and return to root');
    const exported=await download(page);assert.equal(exported.rows.length,root.total_nodes);assert.equal(exported.request.parent_id,null);assert.equal(exported.request.offset,0);
    const leafRows=exported.rows.filter(r=>r.kind==='ObservedUrl');assert.equal(leafRows.length,107);assert(documents.every(url=>leafRows.some(r=>r.address===url)));
    const redirect=leafRows.find(r=>r.address===origin+'/old');assert.equal(redirect.status_3xx,'1');assert.equal(redirect.status_2xx,'0');assert.equal(redirect.indexability_unobserved,'1');
    const rootRow=exported.rows.find(r=>r.id===root.root_id);assert.equal(rootRow.observations,'107');assert.equal(rootRow.view_mode,'reported');assert.equal(rootRow.profile_excluded_page_observations,'1');
    fs.writeFileSync(artifactPath('directory-tree-reported.csv'),exported.bytes);
    checks.push('Actual browser CSV download independently verifies bytes/Content-Length/SHA256 and exports every node beyond the visible page with matching root/profile counts and requested-URL redirect semantics');
    await page.selectOption('#directory-mode','raw');assert.equal(await page.isVisible('#directory-results'),false);assert.equal(await page.isDisabled('#export-csv'),true);
    const raw=await preview(page,()=>page.click('#open-tree'));assert.equal(raw.current.counts.observations,108);assert.equal(raw.reporting_view.identity.mode,'raw');assert.equal(raw.reporting_view.excluded_from_view_page_observations,0);assert.notEqual(raw.snapshot_token,root.snapshot_token);
    const rawCSV=await download(page);assert.equal(rawCSV.rows.filter(r=>r.kind==='ObservedUrl').length,108);assert(rawCSV.rows.some(r=>r.address===origin+'/secret/hidden.html'));
    const stale=await page.evaluate(async({id,token,generation})=>{const results=[];for(const suffix of ['preview','csv']){const r=await fetch(`/api/crawls/${id}/directory-tree/${suffix}`,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({mode:'reported',expected_generation:generation,expected_snapshot_token:token})});results.push({status:r.status,body:await r.json()});}return results;},{id:started.id,token:raw.snapshot_token,generation:raw.snapshot.app_generation});
    for(const result of stale){assert.equal(result.status,409);assert.equal(result.body.code,'stale_directory_snapshot');}
    checks.push('Changing the visible mode invalidates old UI results/export; raw includes the hidden captured page, and native preview plus CSV reject stale cross-view tokens with HTTP409');
    let rawGroup=await descend(page,raw,n=>n.kind==='protocol');
    rawGroup=await descend(page,rawGroup,n=>n.kind==='origin');
    rawGroup=await descend(page,rawGroup,n=>n.address===origin+'/docs/');
    const rawLastPage=await preview(page,()=>page.click('#next-page'));assert.equal(rawLastPage.children.length,5);
    for(const width of [1440,320]) {
      await page.setViewportSize({width,height:900});
      const layout=await page.evaluate(()=>({viewport:innerWidth,document:document.documentElement.scrollWidth}));assert(layout.document<=layout.viewport,JSON.stringify(layout));
      await page.locator('#export-csv').scrollIntoViewIfNeeded();assert(await page.locator('#export-csv').isVisible());
      await page.screenshot({path:artifactPath(`directory-tree-${width}.png`),fullPage:true});
    }
    checks.push('Populated raw hierarchy, controls and export remain reachable without document overflow at 1440px and 320px');
    const empty=structuredClone(report);empty.pages=[];empty.issues=[];empty.config.directive_reporting=null;
    emptyId=await importReport(page,empty,'owned-directory-empty.json');const emptyTree=await open(page,emptyId);
    assert.equal(emptyTree.current.counts.observations,0);assert.equal(emptyTree.current.counts.status_unobserved,0);assert.equal(emptyTree.total_nodes,1);assert.equal(emptyTree.total_children,0);
    assert.equal(await page.isVisible('#empty-node'),true);assert.equal(await page.isDisabled('#previous-page'),true);assert.equal(await page.isDisabled('#next-page'),true);
    assert.equal(emptyTree.snapshot.caller_binding.origin,'app_declared_imported_or_detached_report');assert.equal(emptyTree.snapshot.sql_attested,false);
    const emptyCSV=await download(page);assert.equal(emptyCSV.rows.length,1);assert.equal(emptyCSV.rows[0].observations,'0');
    checks.push('Actual UI project import of an explicit empty report produces a known-empty root and one-row CSV, disabled paging and honest imported/unattested snapshot scope');
    const sparse=structuredClone(empty),base=structuredClone(report.pages.find(p=>p.status===200));
    const unknown={...base,url:origin+'/unknown/page',final_url:origin+'/unknown/page',status:null,final_status:null,fetched_at:'',indexable:true};
    const conflicting={...unknown,status:404,final_status:404};
    sparse.pages=[unknown,conflicting,{...base,url:'not an absolute URL',final_url:''},{...base,url:'ftp://owned.invalid/file',final_url:'ftp://owned.invalid/file'},{...base,url:'http://user:pass@owned.invalid/private',final_url:'http://user:pass@owned.invalid/private'}];
    sparseId=await importReport(page,sparse,'owned-directory-unobserved.json');const sparseTree=await open(page,sparseId);
    assert.equal(sparseTree.input_observations,5);assert.equal(sparseTree.current.counts.observations,2);assert.equal(sparseTree.current.counts.unique_urls,1);assert.equal(sparseTree.current.counts.duplicate_observations,1);assert.equal(sparseTree.current.counts.conflicting_urls,1);
    assert.equal(sparseTree.current.counts.status_unobserved,1);assert.equal(sparseTree.current.counts.status_4xx,1);assert.equal(sparseTree.current.counts.indexability_unobserved,2);assert.equal(sparseTree.current.counts.stored_indexable,0);
    assert.equal(sparseTree.invalid_url_observations,1);assert.equal(sparseTree.unsupported_url_observations,1);assert.equal(sparseTree.credential_url_observations,1);
    const sparseCSV=await download(page),sparseLeaf=sparseCSV.rows.find(r=>r.kind==='ObservedUrl');assert.equal(sparseLeaf.status_unobserved,'1');assert.equal(sparseLeaf.conflicting_urls,'1');assert.equal(sparseLeaf.stored_indexable,'0');
    assert.match(await page.locator('#directory-children').innerText(),/Mehrere widersprüchliche gespeicherte Beobachtungen/);assert.match(await page.locator('#exclusions').innerText(),/1 ungültige, 1 andere Protokolle, 1 URLs mit Zugangsdaten/);
    checks.push('Declared imported missing status stays unknown, duplicate conflicting captures remain two observations/one URL, and invalid/unsupported/credential URL rows are explicitly excluded rather than counted as HTTP0 or indexable');
    const missing=await page.evaluate(async id=>{const r=await fetch(`/api/crawls/${id}/directory-tree/preview`,{method:'POST',headers:{'content-type':'application/json'},body:'{}'});return {status:r.status,body:await r.json()};},randomUUID());
    assert.equal(missing.status,404);assert.equal(missing.body.code,'directory_job_not_found');
    assert.equal(await (await fetch(app+`/api/crawls/${started.id}/project.json`)).text(),savedBytes);
    assert.equal(count(),capturedHits,'Directory views/import/export must not refetch any website');assert.equal(hits.get('/docs/')||0,0,'Synthetic directory address must never be fetched');assert.deepEqual(errors,[]);
    checks.push('Missing saved job is a native HTTP404 rather than an empty tree; all analysis/navigation/import/export preserve original saved report bytes and make zero website requests');
    fs.writeFileSync(artifactPath('directory-tree-live.json'),JSON.stringify({result:'PASS',crawl_id:started.id,empty_import_id:emptyId,sparse_import_id:sparseId,checks,website_requests_during_crawl:capturedHits,website_refetches:count()-capturedHits,errors,browser_evidence:browserEvidence,scope:'Actual redesigned browser/native crawler/saved report/profile/tree navigation/CSV. Unknown and known-empty edge fixtures are honestly declared imports. Cross-view and missing-job guards are actual browser fetches to native API. No vendor semantic parity or cold-process persistence claim.'},null,2)+'\n');
    console.log(`${checks.length} actual native directory-tree workflows passed`);
  } catch(error) {
    await page.screenshot({path:artifactPath('directory-tree-failure.png'),fullPage:true});
    fs.writeFileSync(artifactPath('directory-tree-failure.json'),JSON.stringify({error:error.stack,checks,crawl_id:started.id,page_url:page.url(),browser_evidence:browserEvidence,layout:await page.evaluate(()=>({viewport:innerWidth,document:document.documentElement.scrollWidth,overflow:[...document.querySelectorAll('body *')].filter(e=>e.getBoundingClientRect().right>innerWidth+1).slice(0,20).map(e=>({tag:e.tagName,id:e.id,width:e.getBoundingClientRect().width,text:e.textContent.slice(0,100)}))}))},null,2)+'\n');throw error;
  } finally {await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;}).finally(async()=>{fixture.closeAllConnections();await new Promise(resolve=>fixture.close(resolve));});
