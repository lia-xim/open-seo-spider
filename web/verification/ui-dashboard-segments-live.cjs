'use strict';
// Actual dashboard/editor -> two normal native processes -> one fresh store.
// Synthetic website content is captured by Rust; imports below are explicitly
// declared duplicate-observation controls, never positive native provenance.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const {spawn} = require('node:child_process');
const {createHash, randomUUID} = require('node:crypto');
const {chromium} = require('../../renderer/node_modules/playwright-core');
const root = path.resolve(__dirname, '../..');
assert(process.env.SPIDER_BINARY, 'Explicit normal SPIDER_BINARY required; no test builds or installs');
const binary = fs.realpathSync(process.env.SPIDER_BINARY), sha = bytes => createHash('sha256').update(bytes).digest('hex');
const binarySha = sha(fs.readFileSync(binary));
if (process.env.SPIDER_BINARY_SHA256) assert.equal(binarySha, process.env.SPIDER_BINARY_SHA256);
const output = path.resolve(process.env.SPIDER_DASHBOARD_SEGMENTS_OUTPUT || path.join(
  process.env.SPIDER_TEST_ARTIFACT_DIR || path.join(root, 'artifacts'), `dashboard-segments-${Date.now()}-${randomUUID().slice(0,8)}`));
assert(!fs.existsSync(output)); fs.mkdirSync(output, {recursive: true}); process.env.SPIDER_TEST_ARTIFACT_DIR = output;
const artifactPath = require('./artifact-path.cjs'), store = path.join(output, 'native-store');
const pins = [__filename, path.join(__dirname, 'artifact-path.cjs')].map(file => ({path: file, sha256: sha(fs.readFileSync(file))}));
const checks = [], owners = [], errors = [], external = [], reads = [], downloads = [], assets = [], projections = [], guards = [];
const readPromises = [], websiteHits = new Map(), witnessHits = [], witnessControls = [], nonce = randomUUID();
const productPaths = Array.from({length:1005},(_, index) => '/products/' + String(index).padStart(4,'0'));
const websitePaths = [...productPaths, '/products/hidden', '/outside', '/untitled', '/unassigned', '/missing'];
const reportingColumns = ['Captured Pages','Visible Pages','Hidden Pages','Captured Issues','Visible Issues','Hidden Issues'];
let website, siteOrigin, origin, app, browser, browserContext, page, editor, report, rawReportBytes, rawAnalysis, rules, nativeId, tailURL, offline = false, capturedHits, failure;
const sleep = ms => new Promise(resolve => setTimeout(resolve,ms));
const localeNumber = value => Number(value).toLocaleString('de-DE');
const collator = new Intl.Collator('de', {numeric:true,sensitivity:'base'});
function deadline(promise,label) { let timer; return Promise.race([promise,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Timeout: '+label)),30000);})]).finally(()=>clearTimeout(timer)); }
async function until(work,label) {
  for(let attempt=0;attempt<2400;attempt++){if(app)assert.equal(app.child.exitCode,null,app.log);const value=await work();if(value)return value;await sleep(25);}
  throw Error('Timeout: '+label);
}
function verifyPins(){assert.equal(sha(fs.readFileSync(binary)),binarySha);for(const pin of pins)assert.equal(sha(fs.readFileSync(pin.path)),pin.sha256);}
async function launch(phase,listen='127.0.0.1:0') {
  verifyPins();const env={...process.env,OPEN_SEO_COORDINATED_EXECUTION_WINDOW:'confirmed'};delete env.OPEN_SEO_PAGESPEED_API_KEY;
  const child=spawn(binary,['serve','--listen',listen,'--data-dir',store,'--shutdown-on-stdin-eof'],{cwd:root,windowsHide:true,stdio:['pipe','pipe','pipe'],env});
  const owner={phase,child,log:''};owner.closed=new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));
  child.once('error',error=>{owner.spawnError=error;});for(const stream of[child.stdout,child.stderr])stream.on('data',bytes=>{owner.log+=bytes;});
  owners.push(owner);app=owner;owner.origin=await until(()=>{if(owner.spawnError)throw owner.spawnError;return owner.log.match(/^Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?$/m)?.[1];},'native '+phase);
  assert(!['8787','8790'].includes(new URL(owner.origin).port));origin=owner.origin;assert.equal((await fetch(origin+'/healthz')).status,200);return owner;
}
async function stop(owner) { owner.child.stdin.end();owner.shutdown=await deadline(owner.closed,'native EOF '+owner.phase);assert.deepEqual(owner.shutdown,{code:0,signal:null},owner.log);if(app===owner)app=null;verifyPins(); }
async function raw(route,body,status=200) {
  const response=await fetch(origin+route,{method:body===undefined?'GET':'POST',headers:body===undefined?{}:{'content-type':'application/json'},body:body===undefined?undefined:JSON.stringify(body),redirect:'error',cache:'no-store'});
  const bytes=Buffer.from(await response.arrayBuffer());assert.equal(response.status,status,bytes.toString());return{response,bytes};
}
async function api(route,body,status=200){const value=await raw(route,body,status);return{...value,value:JSON.parse(value.bytes)};}
async function assetPins(phase) {
  for(const name of['app.js','dashboard-segments.js','index.html','styles.css','saved-segments.js','saved-segments.css']){
    const {bytes}=await raw(name==='index.html'?'/':'/'+name);const digest=sha(bytes);assert.equal(digest,sha(fs.readFileSync(path.join(root,'web',name))),'Normal build must serve current '+name);assets.push({phase,path:name,sha256:digest});
  }
}
async function observedRead(response) {
  const request=response.request(),body=request.postDataBuffer(),url=new URL(response.url());assert(body);assert.equal(url.origin,origin);
  assert.match(url.pathname,/^\/api\/crawls\/[a-f0-9-]+\/segments\/dashboard(?:-analysis|\.json|\/[a-z-]+\.csv)?$/i);
  // Repeat exact observed POST REQUEST bytes only for these pure reads. Browser
  // metadata is compared to the oracle; installed DOM is bound by the product's
  // own SHA check plus exact cell assertions. Downloads compare actual bytes.
  // The oracle is never presented as a captured CDP/browser response body.
  const oracle=await fetch(response.url(),{method:'POST',headers:{'content-type':'application/json'},body,redirect:'error',cache:'no-store'});
  const bytes=Buffer.from(await oracle.arrayBuffer()),headers=await response.allHeaders();assert.equal(response.status(),oracle.status,bytes.toString());assert.equal(oracle.status,200,bytes.toString());
  for(const key of['content-type','content-length','x-content-sha256','cache-control','x-segment-snapshot-token','x-segment-generation'])assert.equal(headers[key],oracle.headers.get(key),key);
  assert.equal(Number(headers['content-length']),bytes.length);assert.equal(headers['x-content-sha256'],sha(bytes));assert.equal(headers['cache-control'],'no-store');
  const value=url.pathname.endsWith('.csv')?null:JSON.parse(bytes);if(value){assert.equal(headers['x-segment-snapshot-token'],value.context.snapshot_token);assert.equal(headers['x-segment-generation'],String(value.context.app_generation));}
  const entry={url:response.url(),path:url.pathname,request:JSON.parse(body),request_sha256:sha(body),bytes:bytes.length,sha256:sha(bytes),body_source:'unconditional_exact_read_only_native_post',browser_content_binding:'product_SHA_validation_and_exact_DOM_or_actual_download',headers,value,buffer:bytes};reads.push(entry);return entry;
}
function observe(target) {
  target.on('pageerror',error=>errors.push(error.message));
  target.on('response',response=>{if(response.request().method()==='POST'&&/\/segments\/dashboard(?:-analysis|\.json|\/[a-z-]+\.csv)?$/.test(new URL(response.url()).pathname)){
    const pending=observedRead(response);readPromises.push(pending);pending.catch(()=>{});
  }});
}
async function drainReads(){await Promise.all(readPromises);}
async function selectCrawl(id) {
  await page.locator(`#crawl-history option[value="${id}"]`).waitFor({state:'attached'});await page.selectOption('#crawl-history',id);
  await page.waitForFunction(()=>!document.getElementById('dashboard-segment-apply').disabled);await page.waitForFunction(()=>document.getElementById('crawl-state').textContent.toLowerCase().includes('abgeschlossen'));
}
async function openMain(id) {await page.goto(origin);await page.locator('#connection-state.connected').waitFor();await selectCrawl(id);}
async function authorRules(definitions) {
  if(!editor||editor.isClosed()){editor=await browserContext.newPage();observe(editor);}
  await editor.goto(origin+'/saved-segments');
  for(const definition of definitions){await editor.click('#add');const fieldset=editor.locator('#definitions fieldset').last();await fieldset.getByLabel('Name',{exact:true}).fill(definition.name);
    const condition=definition.conditions[0];await fieldset.locator('.condition select').nth(0).selectOption(condition.field);await fieldset.locator('.condition select').nth(1).selectOption(condition.operator);await fieldset.getByLabel('Wert',{exact:true}).fill(condition.value);
    if(condition.case_insensitive)await fieldset.locator('.condition input[type=checkbox]').check();
  }
  await editor.click('#save');assert.match(await editor.locator('#message').textContent(),/gespeichert/);
  assert.deepEqual(await editor.evaluate(()=>JSON.parse(localStorage.getItem('open-seo-saved-segments-v1'))),{version:1,definitions});
}
async function loadedDefinitions(){await page.click('#dashboard-segment-load');assert.equal(await page.locator('#dashboard-segment-json').isDisabled(),true);}
function selectedOrdinals(source,selection,mode='') {
  const visible=source.pages.map((value,index)=>({value,index})).filter(({value})=>mode==='raw'||!value.meta_robots?.some(text=>/noindex/i.test(text)));
  return visible.filter(({value})=>selection==='all'||selection==='segment:s1'&&productPaths.some(suffix=>value.url===siteOrigin+suffix)||selection==='segment:s2'&&value.titles?.[0]?.includes('Owned product')||selection==='segment:s3'&&value.url===tailURL||selection==='unknown'&&value.url===siteOrigin+'/untitled'||selection==='unassigned'&&['/outside','/unassigned','/missing'].some(suffix=>value.url===siteOrigin+suffix)).map(({index})=>index);
}
function sourceMembers(source,ordinals){return ordinals.map(index=>({page_ordinal:index,page:source.pages[index]}));}
function sourceIssues(source,ordinals){const urls=new Set(ordinals.map(index=>source.pages[index].url));return source.issues.filter(issue=>urls.has(issue.url));}
async function apply(selection,mode='',source=report,expectedOrdinals=selectedOrdinals(source,selection,mode)) {
  await page.selectOption('#dashboard-segment-selection',selection);await page.selectOption('#dashboard-segment-mode',mode);
  assert.equal(await page.locator('#dashboard-segment-json').isDisabled(),true,'Changing selection invalidates previous exports immediately');
  const begin=reads.length,analysisPending=page.waitForResponse(response=>response.url()===`${origin}/api/crawls/${sourceId(source)}/segments/dashboard-analysis`&&response.request().method()==='POST',{timeout:120000});
  await page.click('#dashboard-segment-apply');await analysisPending;
  await page.waitForFunction(()=>!document.getElementById('dashboard-segment-json').disabled);await drainReads();
  const current=reads.slice(begin),pageReads=current.filter(entry=>entry.path.endsWith('/dashboard')&&entry.request.view==='pages'),issueReads=current.filter(entry=>entry.path.endsWith('/dashboard')&&entry.request.view==='issues');
  assert(pageReads.length>0&&issueReads.length>0);assert.deepEqual(pageReads.flatMap(entry=>entry.value.rows).map(({page_ordinal,page})=>({page_ordinal,page})),sourceMembers(source,expectedOrdinals));
  const expectedIssues=sourceIssues(source,expectedOrdinals);assert.deepEqual(issueReads.flatMap(entry=>entry.value.rows),expectedIssues);
  const context=pageReads[0].value.context,cohort=pageReads[0].value.cohort;
  for(const entry of current){assert.deepEqual(entry.value?.context,context);assert.equal(entry.request.limit,200);if(entry.request.cursor)assert.equal(entry.request.expected_snapshot_token,context.snapshot_token);}
  for(const pages of[pageReads,issueReads]){for(let index=0;index<pages.length;index++){assert.deepEqual(pages[index].request.cursor,index?pages[index-1].value.next:null);assert.equal(pages[index].value.next?.offset??pages[index].value.total,pages.slice(0,index+1).reduce((count,item)=>count+item.value.rows.length,0));}}
  const analysis=current.find(entry=>entry.path.endsWith('/dashboard-analysis')).value;
  assert.equal(cohort.summary.selected_pages,expectedOrdinals.length);assert.equal(cohort.summary.selected_issues,expectedIssues.length);assert.deepEqual(analysis.cohort,cohort);assert.deepEqual(analysis.captured_crawl_summary,source.summary);
  const records=expectedOrdinals.map(index=>source.pages[index]);
  assert.equal(cohort.summary.internal_pages,records.filter(value=>value.internal).length);assert.equal(cohort.summary.html_pages,records.filter(value=>value.internal&&value.content_type.startsWith('text/html')).length);
  assert.equal(cohort.summary.indexable_pages,records.filter(value=>value.indexable).length);assert.equal(cohort.summary.failed_pages,records.filter(value=>value.error!=null).length);
  assert.deepEqual(cohort.summary.issue_counts,Object.fromEntries(['error','warning','notice'].map(kind=>[kind,expectedIssues.filter(issue=>issue.severity===kind).length])));
  assert.equal(await page.locator('#metric-fetched').textContent(),localeNumber(expectedOrdinals.length));assert.match(await page.locator('#dashboard-segment-state').textContent(),new RegExp('^'+localeNumber(expectedOrdinals.length)+' von '));
  assert.equal(await page.locator('#metric-html').textContent(),localeNumber(cohort.summary.html_pages));assert.equal(await page.locator('#metric-issues').textContent(),localeNumber(expectedIssues.length));
  projections.push({selection,mode,source:sourceId(source),pages:expectedOrdinals.length,issues:expectedIssues.length,page_sizes:pageReads.map(entry=>entry.value.rows.length),snapshot:context.snapshot_token});
  return{context,cohort,analysis,ordinals:expectedOrdinals,pages:sourceMembers(source,expectedOrdinals),issues:expectedIssues,pageReads,request:pageReads[0].request};
}
const sourceIds=new WeakMap();function sourceId(source){const id=sourceIds.get(source);assert(id);return id;}
function titleRows(pages) {
  const titles=new Map();for(const{page}of pages){const title=String(page.titles[0]??'').trim();if(title)titles.set(title,(titles.get(title)||0)+1);}
  return [...pages].sort((a,b)=>collator.compare(a.page.url,b.page.url)).map(({page})=>{const title=String(page.titles[0]??'');return[page.url,title,localeNumber(Array.from(title).length),page.title_pixel_widths?.[0]==null?'—':localeNumber(page.title_pixel_widths[0]),localeNumber(page.titles.length),localeNumber(titles.get(title.trim())||0),String(page.final_status??page.status??'—')];});
}
async function exactTitles(view,all=true) {
  await page.click('[data-section="titles"]');await page.selectOption('#page-size','100');
  const expected=titleRows(view.pages),seen=[];let offset=0;
  do{const slice=expected.slice(offset,offset+100);await page.waitForFunction(rows=>JSON.stringify([...document.querySelectorAll('#results-table tbody tr[data-row]')].map(row=>[...row.cells].map(cell=>cell.textContent)))===JSON.stringify(rows),slice);
    const actual=await page.locator('#results-table tbody tr[data-row]').evaluateAll(rows=>rows.map(row=>[...row.cells].map(cell=>cell.textContent)));assert.deepEqual(actual,slice);seen.push(...actual);
    if(!all||await page.locator('#page-next').isDisabled())break;await page.click('#page-next');offset+=100;
  }while(true);
  if(all)assert.deepEqual(seen,expected);return expected;
}
function csvRows(bytes){
  const text=bytes.toString('utf8').replace(/^\uFEFF/,''),rows=[];let row=[],field='',quoted=false;
  for(let index=0;index<text.length;index++){const char=text[index];if(char==='"'&&quoted&&text[index+1]==='"'){field+='"';index++;}else if(char==='"')quoted=!quoted;else if(char===','&&!quoted){row.push(field);field='';}else if(char==='\n'&&!quoted){row.push(field.replace(/\r$/,''));rows.push(row);row=[];field='';}else field+=char;}
  assert.equal(quoted,false);if(field||row.length){row.push(field);rows.push(row);}assert(rows.length>0);assert(rows.every(row=>row.length===rows[0].length));return rows;
}
async function attachment(button,label,native=true){const pending=page.waitForEvent('download'),start=reads.length;await page.click(button);const file=await pending;assert.equal(await file.failure(),null);const bytes=fs.readFileSync(await file.path());await drainReads();const oracle=native?reads.slice(start).find(entry=>entry.path.endsWith('.json')||entry.path.endsWith('.csv')):null;if(native){assert(oracle);assert.deepEqual(bytes,oracle.buffer);}downloads.push({label,bytes:bytes.length,sha256:sha(bytes),native_oracle:!!oracle});fs.writeFileSync(artifactPath(label),bytes);return bytes;}
function csvContext(view,source){return[
 ['Segment Snapshot Token',view.context.snapshot_token],['Segment Native Generation',String(view.context.app_generation??'')],['Segment Typed Report SHA256',view.context.typed_report_sha256],['Segment Rules SHA256',view.context.rules_sha256],['Segment Selection JSON',JSON.stringify(view.cohort.selection)],['Segment Reporting View JSON',JSON.stringify(view.context.reporting_view??null)],['Segment Captured Pages',String(source.pages.length)],['Segment Eligible Pages',String(view.cohort.summary.total_pages)],['Segment Selected Pages',String(view.pages.length)],['Segment Captured Issues',String(source.issues.length)],['Segment Selected Issues',String(view.issues.length)],['Segment Orphan Issues',String(view.cohort.summary.orphan_issues)],['Segment Scope','selected saved observations; crawl activity and relationship targets unchanged; no SQL or origin attestation']];}
async function nativeExports(view,source,label) {
  const full=JSON.parse(await attachment('#dashboard-segment-json',label+'.json'));
  assert.equal(full.kind,'segment_dashboard_cohort_not_raw_crawl');assert.deepEqual(full.context,view.context);assert.deepEqual(full.cohort,view.cohort);assert.deepEqual(full.pages.map(({page_ordinal,page})=>({page_ordinal,page})),view.pages);assert.deepEqual(full.issues,view.issues);assert.deepEqual(full.captured_config,source.config);assert.deepEqual(full.captured_crawl_summary,source.summary);
  const csv=csvRows(await attachment('#dashboard-segment-csv',label+'.csv')),baseline=csvRows((await raw(`/api/crawls/${sourceId(source)}/export.csv`)).bytes),baseHeader=baseline[0].filter(name=>!reportingColumns.includes(name));
  const extras=csvContext(view,source);assert.deepEqual(csv[0],[...baseHeader,...extras.map(pair=>pair[0])]);
  const eligible=source.pages.map((page,index)=>({page,index})).filter(({page})=>!page.meta_robots?.some(text=>/noindex/i.test(text)));
  assert.equal(baseline.length-1,eligible.length);
  const byOrdinal=new Map(eligible.map((item,index)=>[item.index,baseline[index+1].filter((_,column)=>!reportingColumns.includes(baseline[0][column]))]));
  assert.deepEqual(csv.slice(1),view.pages.map(row=>{const cells=byOrdinal.get(row.page_ordinal);assert(cells,'This CSV case uses profile-visible source observations');return[...cells,...extras.map(pair=>pair[1])];}));
  return full;
}
async function nativeOwnedCSV(view,source,kind,ownerColumn,label){
  const csv=csvRows(await attachment('#export-resources',label)),baseline=csvRows((await raw(`/api/crawls/${sourceId(source)}/${kind}.csv`)).bytes);
  const baseHeader=baseline[0].filter(name=>!reportingColumns.includes(name)),extras=csvContext(view,source),owner=baseline[0].indexOf(ownerColumn),selected=new Set(view.pages.map(({page})=>page.url));assert(owner>=0);
  assert.deepEqual(csv[0],[...baseHeader,...extras.map(pair=>pair[0])]);
  const expected=baseline.slice(1).filter(row=>selected.has(row[owner])).map(row=>[...row.filter((_,column)=>!reportingColumns.includes(baseline[0][column])),...extras.map(pair=>pair[1])]);
  assert.deepEqual(csv.slice(1),expected);return csv;
}
async function genericTitleCSV(view,source,label,query=''){
  await page.locator('#table-search').fill(query);
  const sorted=[...view.pages].sort((a,b)=>collator.compare(a.page.url,b.page.url)).filter(({page})=>!query||page.url===query);
  const csv=csvRows(await attachment('#export-csv',label,false));
  const headers=['URL','Seitentitel','Zeichen','Pixelprofil 20px','Title-Tags','Vorkommen','HTTP final','Crawl Coverage','Observation Basis','Selected State','Parent Snapshot SHA256','Fetched At','Observation Requested URL','Segment Snapshot Token','Segment Rules SHA256','Segment Typed Report SHA256','Segment Generation','Segment Selection','Segment Reporting Identity','Original Page Ordinal','Captured Pages','Visible Pages','Hidden Pages'];
  assert.deepEqual(csv[0],headers);
  const counts=new Map();for(const{page}of view.pages){const title=String(page.titles[0]??'').trim();if(title)counts.set(title,(counts.get(title)||0)+1);}
  const visible=source.pages.filter(page=>!page.meta_robots?.some(text=>/noindex/i.test(text))).length;
  assert.deepEqual(csv.slice(1),sorted.map(({page,page_ordinal})=>{const title=String(page.titles[0]??'');return[
    page.url,title,String(Array.from(title).length),String(page.title_pixel_widths?.[0]??''),String(page.titles.length),String(counts.get(title.trim())||0),String(page.final_status??page.status??''),
    'crawl_snapshot','captured','','',page.fetched_at,page.url,view.context.snapshot_token,view.context.rules_sha256,view.context.typed_report_sha256,String(view.context.app_generation),JSON.stringify(view.cohort.selection),JSON.stringify(view.context.reporting_view??null),String(page_ordinal),String(source.pages.length),String(visible),String(source.pages.length-visible)];}));
  await page.locator('#table-search').fill('');
}
async function noApplied(){await page.waitForFunction(()=>document.getElementById('dashboard-segment-json').disabled&&!document.getElementById('dashboard-segments').getAttribute('aria-busy').includes('true'));assert.equal(await page.locator('#dashboard-segment-csv').isDisabled(),true);assert.equal(await page.locator('#dashboard-segment-scope').isHidden(),true);}
async function probeWitness(){const response=await fetch(siteOrigin+'/owned-witness-probe',{headers:{'x-owned-witness-control':nonce},redirect:'error'});assert.equal(response.status,410);assert.equal(await response.text(),'Owned offline witness');}

(async()=>{
  try{
    website=http.createServer((request,response)=>{if(offline){(request.url==='/owned-witness-probe'&&request.headers['x-owned-witness-control']===nonce?witnessControls:witnessHits).push(request.url);response.writeHead(410,{'content-type':'text/plain'});response.end('Owned offline witness');return;}
      websiteHits.set(request.url,(websiteHits.get(request.url)||0)+1);const valid=websitePaths.includes(request.url),product=productPaths.includes(request.url),title=product||request.url==='/products/hidden'?'Owned product '+request.url.slice(-4)+' – independent captured document':'Outside selected cohort '+request.url;
      response.writeHead(request.url==='/missing'||!valid?404:200,{'content-type':'text/html; charset=utf-8'});response.end(`<html lang="en"><head>${request.url==='/untitled'?'':`<title>${title}</title>`}<meta name="description" content="Independent owned fixture ${request.url} with captured source facts and enough distinct document text to inspect saved membership accurately.">${request.url==='/products/hidden'?'<meta name="robots" content="noindex">':''}</head><body><main><h1>${title}</h1><p>Genuinely captured owned native dashboard segmentation fixture. No provider or public website.</p>${product?'<a href="/outside">Captured target outside selected products</a>':request.url==='/outside'?'<a href="/products/0000">Genuine captured incoming link</a>':''}</main></body></html>`);});
    await new Promise((resolve,reject)=>{website.once('error',reject);website.listen(0,'127.0.0.1',resolve);});siteOrigin='http://127.0.0.1:'+website.address().port;
    const first=await launch('first');await assetPins('first');
    nativeId=(await api('/api/crawls',{seed_urls:websitePaths.map(suffix=>siteOrigin+suffix),crawl_links:false,discover_sitemaps:false,obey_robots:false,delay_ms:0,concurrency:16,directive_reporting:{version:1,respect_noindex:true,respect_canonical:false,respect_next_prev:false}})).value.id;
    await until(async()=>{const job=(await api('/api/crawls/'+nativeId)).value;assert.notEqual(job.status,'failed',job.error);return job.status==='completed';},'1010 genuinely captured pages');
    const saved=await api(`/api/crawls/${nativeId}/project.json`);report=saved.value;rawReportBytes=saved.bytes;sourceIds.set(report,nativeId);
    assert.equal(report.pages.length,1010);assert.deepEqual(new Set(report.pages.map(page=>page.url)),new Set(websitePaths.map(suffix=>siteOrigin+suffix)));capturedHits=[...websiteHits.values()].reduce((sum,value)=>sum+value,0);assert.equal(capturedHits,1010);assert([...websiteHits.values()].every(value=>value===1));
    const tail=report.pages.map((page,index)=>({page,index})).filter(({page,index})=>index>1000&&productPaths.includes(new URL(page.url).pathname)).at(-1);assert(tail);tailURL=tail.page.url;
    rawAnalysis=(await api(`/api/crawls/${nativeId}/analysis`)).value;
    rules={version:1,definitions:[
      {id:'s1',name:'Produkte über1000',join:'all',conditions:[{field:'requested_url',operator:'regex',value:'/products/[0-9]{4}$',case_insensitive:false}]},
      {id:'s2',name:'Titelüberlappung',join:'all',conditions:[{field:'stored_title',operator:'contains',value:'Owned product',case_insensitive:false}]},
      {id:'s3',name:'Nur hinter Originalzeile1000',join:'all',conditions:[{field:'requested_url',operator:'equals',value:tailURL,case_insensitive:false}]},
      {id:'s4',name:'Explizit bekannte leere Auswahl',join:'all',conditions:[{field:'requested_url',operator:'equals',value:siteOrigin+'/never-captured',case_insensitive:false}]}]};
    const executablePath=process.env.OPEN_SEO_BROWSER_PATH||process.env.SPIDER_CHROME;assert(executablePath);browser=await chromium.launch({headless:true,executablePath});browserContext=await browser.newContext({acceptDownloads:true,viewport:{width:1440,height:1000}});
    await browserContext.route('**/*',route=>{const url=route.request().url();if(url.startsWith(origin+'/')||url.startsWith('blob:')||url.startsWith('data:'))return route.continue();external.push(url);return route.abort();});
    page=await browserContext.newPage();observe(page);await openMain(nativeId);await noApplied();await authorRules(rules.definitions);await noApplied();await loadedDefinitions();
    const products=await apply('segment:s1');assert.deepEqual(products.pageReads.map(entry=>entry.value.rows.length),[200,200,200,200,200,5]);assert.equal(products.cohort.summary.total_pages,1009);assert.equal(products.cohort.summary.multiple_matches,1005);assert.equal(products.cohort.summary.unknown_pages,1);assert.equal(products.cohort.summary.unassigned_pages,3);
    assert.deepEqual(products.cohort.counts.map(count=>[count.matched,count.unknown]),[[1005,0],[1005,1],[1,0],[0,0]]);
    await exactTitles(products);await nativeExports(products,report,'products-complete');await genericTitleCSV(products,report,'products-title-all1005.csv');await genericTitleCSV(products,report,'products-title-filtered-tail.csv',tailURL);
    assert.deepEqual(await attachment('#export-project','original-raw-project.json',false),rawReportBytes,'Raw-labelled export remains the complete original crawl');
    const alias=JSON.parse(await attachment('#export-visible-report','products-visible-json-alias.json'));assert.equal(alias.kind,'segment_dashboard_cohort_not_raw_crawl');assert.deepEqual(alias.pages.map(({page_ordinal,page})=>({page_ordinal,page})),products.pages);
    await page.click('[data-section="external"]');assert.equal(await page.locator('#result-count').textContent(),'0 Ergebnisse','A captured internal target outside the cohort is never relabelled external');
    await page.click('[data-section="linkdom"]');const productLinks=await nativeOwnedCSV(products,report,'links','Source','products-original-source-links.csv');assert.equal(productLinks.length,1006);assert(productLinks.slice(1).every(row=>row[1]===siteOrigin+'/outside'));
    assert.deepEqual(products.analysis.analysis.graph.edges,rawAnalysis.graph.edges.filter(edge=>products.pages.some(({page})=>page.url===edge.source)));
    assert.equal(products.analysis.analysis.graph.edges.filter(edge=>edge.target===siteOrigin+'/outside').length,1005);assert(products.analysis.target_lookup.some(target=>target.url===siteOrigin+'/outside'));
    assert.deepEqual(products.analysis.analysis.graph.orphan_candidates,rawAnalysis.graph.orphan_candidates.filter(candidate=>products.pages.some(({page})=>page.url===candidate.url)));
    assert(!products.analysis.analysis.graph.orphan_candidates.some(candidate=>candidate.url===siteOrigin+productPaths[0]),'A genuine incoming link from an outside captured owner must not become a false orphan');
    assert.deepEqual(products.analysis.analysis.graph.nodes,rawAnalysis.graph.nodes.filter(node=>products.pages.some(({page})=>page.url===node.url)),'Link metrics preserve the original full-capture graph basis');
    await page.click('[data-section="graph"]');assert.match(await page.locator('#graph-view .graph-note').textContent(),/70 von 1\.005 internen URLs/);assert.match(await page.locator('#graph-view .graph-note').textContent(),/1\.005 beobachtete Ankerverbindungen/);assert.equal(await page.locator('#graph-view .graph-node').count(),70);
    assert.match(await page.locator('#dashboard-segment-scope').textContent(),/vollständigen Original-Captures/);
    await page.click('[data-section="linkmetrics"]');assert.equal(await page.locator('#result-count').textContent(),'1.005 Ergebnisse');
    await page.click('[data-section="overview"]');assert.deepEqual(await page.locator('#overview-view .status-summary-row strong').allTextContents(),['1.005','0','0','0','0']);
    assert.match(await page.locator('#metric-discovered').textContent(),/1\.010/);assert.equal(await page.locator('#metric-queued').textContent(),'0');
    assert.match(await page.locator('#metric-queued').locator('..').textContent(),/gesamter Crawl/);assert.match(await page.locator('#metric-fetched').locator('..').textContent(),/Seiten im Segment/);
    checks.push('1010 genuine native pages feed complete200-row native cursors and exact1005-row dashboard/title paging; counts, overlap/unknowns, original graph targets and full native JSON/CSV retain all source observations');
    for(const selection of['segment:s2','unknown','unassigned','segment:s4']){const view=await apply(selection);await exactTitles(view);await nativeExports(view,report,selection.replace(':','-'));if(selection==='segment:s4'){await page.click('[data-section="overview"]');assert.match(await page.locator('#overview-view').textContent(),/Keine Seiten in dieser Segmentauswahl/);}}
    const rawAll=await apply('all','raw');assert.equal(rawAll.pages.length,1010);assert.equal(rawAll.cohort.counts[1].matched,1006);await exactTitles(rawAll,false);
    const reportedAll=await apply('all','reported');assert.equal(reportedAll.pages.length,1009);assert(!reportedAll.pages.some(({page})=>page.url===siteOrigin+'/products/hidden'));await nativeExports(reportedAll,report,'reported-all');
    const tailView=await apply('segment:s3');assert.equal(tailView.pages.length,1);assert.equal(tailView.pages[0].page_ordinal,tail.index);assert(tail.index>1000);await exactTitles(tailView);
    await page.locator('#results-table tbody tr[data-row]').click();assert.equal(await page.locator('#inspector-url').textContent(),tailURL);
    assert.equal(await page.locator('#inspector-content dt').filter({hasText:/^Originale Seitenbeobachtung \(ab 0\)$/}).evaluate(element=>element.nextElementSibling.textContent),String(tail.index));
    await page.click('#inspector-tabs [data-tab="outlinks"]');assert(await page.locator('#inspector-content a').evaluateAll((links,target)=>links.some(link=>link.href===target),siteOrigin+'/outside'));
    await nativeExports(tailView,report,'tail-after1000');
    for(const[field,value]of[['expected_snapshot_token','0'.repeat(64)],['expected_generation',tailView.context.app_generation+1]]){const body={...tailView.request,cursor:null,expected_snapshot_token:tailView.context.snapshot_token,expected_generation:tailView.context.app_generation,[field]:value};const refused=await api(`/api/crawls/${nativeId}/segments/dashboard.json`,body,409);assert.equal(refused.response.headers.get('content-disposition'),null);assert.equal(refused.response.headers.get('x-content-sha256'),null);guards.push({field,status:409,code:refused.value.code});}
    checks.push('Unknown, unassigned, overlapping, explicitly empty and raw/reported cohorts stay distinct; tail-only originalordinal>1000 reaches inspector with genuine outside-target link; stale token/generation exports refuse409');
    await page.click('#dashboard-segment-clear');await noApplied();assert.equal(await page.locator('#metric-fetched').textContent(),'1.010');
    await loadedDefinitions();await apply('segment:s3');await editor.getByLabel('Name',{exact:true}).first().fill('Bearbeitete Definition');await editor.click('#save');await noApplied();
    await authorRules(rules.definitions);await loadedDefinitions();await apply('segment:s3');await page.reload();await selectCrawl(nativeId);await noApplied();await loadedDefinitions();
    // The duplicate URL control is deliberately imported and status-relabelled.
    const declared=structuredClone(report),base=structuredClone(report.pages.find(value=>value.url===siteOrigin+productPaths[0])),outside=structuredClone(report.pages.find(value=>value.url===siteOrigin+'/outside'));
    declared.pages=[{...base,status:200,final_status:200,titles:['Declared selected observation200']},{...base,status:404,final_status:404,titles:['Declared omitted observation404']},{...outside,status:500,final_status:500}];
    declared.issues=[{severity:'warning',code:'declared_duplicate_control',url:base.url,message:'Declared duplicate URL issue',related_urls:[]},{severity:'notice',code:'declared_outside_control',url:outside.url,message:'Declared outside issue',related_urls:[]}];
    const importPending=page.waitForResponse(response=>response.url()===origin+'/api/crawls/import'&&response.request().method()==='POST').then(async response=>({status:response.status(),bytes:await response.body()}));
    await page.setInputFiles('#import-file',{name:'declared-duplicate-observations.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(declared))});const imported=await importPending;assert.equal(imported.status,200,imported.bytes.toString());const declaredId=JSON.parse(imported.bytes).id;
    const declaredSaved=(await api(`/api/crawls/${declaredId}/project.json`)).value;sourceIds.set(declaredSaved,declaredId);await page.waitForFunction(id=>document.getElementById('crawl-history').value===id,declaredId);await noApplied();
    const statusRules=[{id:'s1',name:'Declared status200 only',join:'all',conditions:[{field:'status',operator:'equals',value:'200',case_insensitive:false}]}];await authorRules(statusRules);await loadedDefinitions();
    const duplicate=await apply('segment:s1','',declaredSaved,[0]);assert.equal(duplicate.pages[0].page.titles[0],'Declared selected observation200');assert.deepEqual(duplicate.issues.map(issue=>issue.code),['declared_duplicate_control','meta_keywords_missing']);assert(duplicate.issues.every(issue=>issue.url===base.url));await exactTitles(duplicate);await nativeExports(duplicate,declaredSaved,'declared-single-duplicate-ordinal');
    await page.locator('#results-table tbody tr[data-row]').click();assert.match(await page.locator('#inspector-content').textContent(),/Declared selected observation200/);assert(!/Declared omitted observation404/.test(await page.locator('#inspector-content').textContent()));
    await selectCrawl(nativeId);await noApplied();await authorRules(rules.definitions);await loadedDefinitions();const beforeCold=await apply('segment:s3');const coldBytes=await attachment('#dashboard-segment-json','before-cold.json');
    checks.push('Explicit Clear, cross-tab rule save, reload and crawl switch invalidate cohort/export ownership; declared duplicate URLs retain selected originalordinal and source-owned issues without substituting the omitted observation');
    await editor.close();editor=null;offline=true;await probeWitness();await stop(first);
    const second=await launch('cold',new URL(origin).host);assert.equal(second.origin,first.origin);assert.notEqual(first.child.pid,second.child.pid);await assetPins('cold');
    await page.reload();await selectCrawl(nativeId);await noApplied();await loadedDefinitions();const cold=await apply('segment:s3');assert.deepEqual(cold.context,beforeCold.context);assert.deepEqual(cold.pages,beforeCold.pages);assert.deepEqual(await attachment('#dashboard-segment-json','after-cold.json'),coldBytes);await exactTitles(cold);
    await page.setViewportSize({width:320,height:900});for(const selector of['#dashboard-segment-selection','#dashboard-segment-apply','#dashboard-segment-json','#dashboard-segment-csv']){const element=page.locator(selector);await element.scrollIntoViewIfNeeded();const box=await element.boundingBox();assert(box&&box.x>=0&&box.x+box.width<=320,selector+' reachable at320px');}assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
    await page.locator('#dashboard-segment-state').scrollIntoViewIfNeeded();await page.screenshot({path:artifactPath('dashboard-segments-320.png')});await page.setViewportSize({width:1440,height:1000});await page.screenshot({path:artifactPath('dashboard-segments-1440.png')});
    assert.deepEqual((await raw(`/api/crawls/${nativeId}/project.json`)).bytes,rawReportBytes);await drainReads();assert.deepEqual(errors,[]);assert.deepEqual(external,[]);assert.deepEqual(witnessHits,[]);
    await browser.close();browser=null;await stop(second);await probeWitness();assert.deepEqual(witnessHits,[]);assert.equal(witnessControls.length,2);assert.equal([...websiteHits.values()].reduce((sum,value)=>sum+value,0),capturedHits);
    checks.push('Two naturally closed native processes reuse only the owned store/origin and retained browser rules; explicit cold apply reproduces identical cohort/export with zero website refetches through both EOF0/browser closes; original raw report bytes stay unchanged and320px controls remain reachable');
  }catch(error){failure=error;}
  finally{
    if(page&&failure)try{await page.screenshot({path:artifactPath('dashboard-segments-failure.png')});}catch{}
    if(browser)await browser.close();for(const owner of owners)if(!owner.shutdown&&owner.child.exitCode===null){try{await stop(owner);}catch(error){failure||=error;if(owner.child.exitCode===null)owner.child.kill();}}
    if(website?.listening){website.closeAllConnections();await new Promise(resolve=>website.close(resolve));}
    for(const owner of owners)fs.writeFileSync(artifactPath('native-'+owner.phase+'.log'),owner.log);try{verifyPins();}catch(error){failure||=error;}
    fs.writeFileSync(artifactPath('dashboard-segments-live.json'),JSON.stringify({result:failure?'FAIL':'PASS',checks,errors,failure:failure?.stack,binary_sha256:binarySha,pins,served_assets:assets,
      native:owners.map(owner=>({phase:owner.phase,pid:owner.child.pid,origin:owner.origin,shutdown:owner.shutdown})),native_pages:report?.pages.length,native_crawl_id:nativeId,tail_original_ordinal:report?.pages.findIndex(value=>value.url===tailURL),
      website_capture_requests:capturedHits,offline_product_requests:witnessHits,witness_controls:witnessControls,projections,source_guards:guards,exports:downloads,
      read_only_native_oracles:reads.map(({value,buffer,...entry})=>entry),external_requests:external,
      scope:'Bounded real dashboard/editor/native integration over >1000 genuinely captured source observations, separate declared duplicate controls, same-origin retained browser rules and normal two-process cold restart. No server-side rule storage, crash recovery, all operators, installation, real providers or reference parity claimed.'},null,2)+'\n');
  }
  if(failure)throw failure;for(const check of checks)console.log('PASS '+check);console.log('Evidence: '+output);
})().catch(error=>{console.error(error.stack);process.exitCode=1;});
