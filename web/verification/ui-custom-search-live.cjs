'use strict';
const assert=require('node:assert/strict'),http=require('node:http'),fs=require('node:fs'),crypto=require('node:crypto');
const path=require('node:path'),{spawn}=require('node:child_process');
const {chromium}=require('../../renderer/node_modules/playwright-core');
const artifactPath=require('./artifact-path.cjs');
const app=process.env.SPIDER_APP_URL,renderer=process.env.SPIDER_RENDERER_URL;
assert(app&&new URL(app).hostname==='127.0.0.1'&&!['8787','8790'].includes(new URL(app).port),'Use an isolated native test app');
assert(renderer&&new URL(renderer).hostname==='127.0.0.1'&&!['8787','8790'].includes(new URL(renderer).port),'Use an isolated renderer');
const checks=[],errors=[],attachmentChecks=[],hits=new Map(),sources=['raw_html','rendered_html','raw_selected_text','rendered_selected_text'];
const sourceLabels={raw_html:'Rohes HTML',rendered_html:'Gerendertes HTML',raw_selected_text:'Ausgewählter Roh-Text',rendered_selected_text:'Ausgewählter DOM-Text'};
const statusLabels={complete:'Vollständig',incomplete:'Unvollständig',unavailable:'Nicht verfügbar'};
const profile={version:1,rules:[{name:'Tokens <b>literal</b>',method:'literal',pattern:'TOKEN',sources},
  {name:'Regex ignore case',method:'regex',pattern:'token',regex_flags:'i',sources},{name:'Confirmed zero',method:'literal',pattern:'NEVER_PRESENT_OWNED',sources:['raw_selected_text']}],
  limits:{max_input_bytes:65536,max_matches_scanned:100,max_saved_matches:1,max_excerpt_bytes:80,max_output_json_bytes:65536}};
const html='<!doctype html><html lang="en"><head><title>Owned search fixture</title></head><body><nav>Ä TOKEN OUTSIDE_SELECTION</nav><main>Ä TOKEN raw <em>TOKEN</em></main><script>const word=String.fromCharCode(84,79,75,69,78);document.querySelector("main").textContent=["Ä",word,"dom",word,word].join(" ");</script></body></html>';
let externalURL;
const external=http.createServer((req,res)=>{hits.set('external:'+req.url,(hits.get('external:'+req.url)||0)+1);res.setHeader('content-type','text/html; charset=utf-8');res.end('<html><head><title>Owned external</title></head><body>TOKEN external TOKEN</body></html>');});
const website=http.createServer((req,res)=>{
  hits.set(req.url,(hits.get(req.url)||0)+1);
  if(req.url==='/robots.txt'){res.setHeader('content-type','text/plain');return res.end('User-agent: *\nAllow: /\n');}
  if(req.url==='/visibility-root'){res.setHeader('content-type','text/html; charset=utf-8');return res.end(`<html><head><title>Owned visibility root</title></head><body>TOKEN root <a href="/hidden">Hidden owned page</a><a href="${externalURL}">Owned external page</a></body></html>`);}
  if(req.url==='/hidden'){res.setHeader('content-type','text/html; charset=utf-8');return res.end('<html><head><title>Owned hidden</title><meta name="robots" content="noindex"></head><body>TOKEN TOKEN TOKEN TOKEN</body></html>');}
  if(req.url!=='/dynamic'){res.statusCode=404;res.setHeader('content-type','text/html');return res.end('<html><body>Owned missing fixture</body></html>');}
  res.setHeader('content-type','text/html; charset=utf-8');
  res.end(html);
});
const count=()=>Object.fromEntries(hits),sha=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
const check=name=>{checks.push(name);console.log('PASS '+name);};
async function until(work,label){for(let i=0;i<1200;i++){const value=await work();if(value)return value;await new Promise(r=>setTimeout(r,50));}throw Error('Timeout: '+label);}
async function api(path,origin=app){const response=await fetch(origin+path);assert.equal(response.status,200,await response.clone().text());return response.json();}
function captureResponse(page,predicate){return page.waitForResponse(predicate).then(response=>{const body=response.body();return body.then(bytes=>({response,bytes}));});}
async function ownedNative(data,phase){
  const root=path.resolve(__dirname,'../..'),binary=process.env.SPIDER_BINARY||path.join(root,'target/debug/open-seo-spider'+(process.platform==='win32'?'.exe':''));
  const binarySha=sha(fs.readFileSync(binary));let origin='',log='',pending='',startupError;
  const child=spawn(binary,['serve','--listen','127.0.0.1:0','--data-dir',data,'--shutdown-on-stdin-eof'],{cwd:root,windowsHide:true,stdio:['pipe','pipe','pipe'],env:{...process.env,OPEN_SEO_COORDINATED_EXECUTION_WINDOW:'confirmed'}});
  const closed=new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));child.once('error',error=>startupError=error);
  for(const stream of [child.stdout,child.stderr])stream.on('data',bytes=>log+=bytes);
  child.stdout.on('data',bytes=>{pending+=bytes.toString('utf8');let end;while((end=pending.indexOf('\n'))>=0){const line=pending.slice(0,end).replace(/\r$/,'');pending=pending.slice(end+1);const bound=/^Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)$/.exec(line);if(bound){assert.equal(origin,'');origin=bound[1];}}});
  let stopped=false;
  const stop=async()=>{if(stopped)return;stopped=true;child.stdin.end();const timer=setTimeout(()=>child.kill(),20000);const shutdown=await closed;clearTimeout(timer);fs.writeFileSync(artifactPath(`custom-search-cold-${phase}.log`),log);assert.equal(shutdown.code,0,'Owned native process must close naturally');assert.equal(sha(fs.readFileSync(binary)),binarySha);return shutdown;};
  try{await until(()=>{if(startupError)throw startupError;assert.equal(child.exitCode,null,log);return origin;},'owned cold native bind');assert(!['8787','8790'].includes(new URL(origin).port));assert.equal((await fetch(origin+'/healthz')).status,200);return {origin,stop,binarySha};}catch(error){await stop();throw error;}
}
async function settings(page,config,render=true,capture=true,extra={}){
  await page.click('#settings-open');await page.locator('#advanced-config').evaluate(e=>e.open=true);await page.locator('#custom-search-settings').evaluate(e=>e.open=true);
  await page.check('[name=custom_search_enabled]');await page.fill('[name=custom_search_json]',JSON.stringify(config));
  await page.setChecked('[name=javascript_rendering]',render);
  if(render){await page.fill('[name=renderer_endpoint]',renderer);await page.fill('[name=renderer_wait_ms]','30');}
  await page.locator('#content-capture-settings').evaluate(e=>e.open=true);await page.setChecked('[name=content_capture_enabled]',capture);
  if(capture)await page.fill('[name=content_capture_json]',JSON.stringify({sources:['raw_http_html','rendered_dom_html'],include_selectors:['main']}));
  await page.uncheck('[name=obey_robots]');await page.fill('[name=delay_ms]','0');
  await page.setChecked('[name=check_external]',extra.external===true);await page.locator('[name=respect_noindex]').evaluate(input=>input.closest('details').open=true);await page.setChecked('[name=respect_noindex]',extra.noindex===true);
  await page.click('#settings-form button[type=submit]');await page.locator('#settings-dialog').waitFor({state:'hidden'});
}
async function start(page,seed,expected=200){
  await page.fill('#seed-url',seed);const pending=captureResponse(page,r=>r.url()===app+'/api/crawls'&&r.request().method()==='POST');await page.click('#start-button');const {response,bytes}=await pending;
  assert.equal(response.status(),expected,bytes.toString('utf8'));
  if(expected!==200){await page.waitForFunction(()=>!document.getElementById('start-button').disabled);return response;}
  const id=JSON.parse(bytes).id;
  await until(async()=>{const job=await api('/api/crawls/'+id);assert.notEqual(job.status,'failed',job.error);return job.status==='completed';},'native custom-search crawl');
  await page.waitForFunction(id=>document.getElementById('crawl-history').value===id&&document.querySelector('#crawl-state .status-dot')?.classList.contains('completed'),id);
  return {id,report:await api('/api/crawls/'+id+'/project.json'),request:response.request().postDataJSON()};
}
function parseCsv(text){const rows=[];let row=[],cell='',quoted=false;for(let i=0;i<text.length;i++){const c=text[i];if(c==='"'){if(quoted&&text[i+1]==='"'){cell+='"';i++;}else quoted=!quoted;}else if(c===','&&!quoted){row.push(cell);cell='';}else if(c==='\n'&&!quoted){row.push(cell.replace(/\r$/,''));rows.push(row);row=[];cell='';}else cell+=c;}assert.equal(quoted,false);if(cell||row.length){row.push(cell);rows.push(row);}const header=rows.shift();return rows.map(row=>Object.fromEntries(header.map((key,index)=>[key,row[index]])));}
async function download(page,button,path){
  // Chrome may omit attachment response bodies from CDP. Always verify the
  // actual browser file against an independent GET to the exact observed URL.
  const pending=page.waitForEvent('download'),responding=page.waitForResponse(r=>new URL(r.url()).pathname.endsWith(path));
  await page.click(button);const response=await responding;assert.equal(response.status(),200);
  const bytes=fs.readFileSync(await(await pending).path()),url=new URL(response.url());
  assert.equal(url.origin,new URL(page.url()).origin,'Download must belong to this owned native process');
  const native=await fetch(url);assert.equal(native.status,200);const expected=Buffer.from(await native.arrayBuffer());
  assert.equal(bytes.length,expected.length,'Browser attachment/native HTTP length mismatch');assert.equal(sha(bytes),sha(expected),'Browser attachment/native HTTP hash mismatch');
  if(path==='/custom-search.csv'){const headers=response.headers();assert.equal(Number(headers['content-length']),bytes.length);assert.equal(headers['x-content-sha256'],sha(bytes));assert.equal(headers['cache-control'],'no-store');}
  attachmentChecks.push({url:String(url),download_bytes:bytes.length,native_sha256:sha(expected),verification:'Actual browser file independently matched exact same owned native GET URL; no CDP attachment body assumed.'});
  return {bytes,response};
}
async function table(page,observations){
  await page.click('[data-section="custom-search"]');
  const expected=observations.map(row=>[row.url||row.document_url||observations[0]?.document_url,row.rule_name,row.method,sourceLabels[row.source],statusLabels[row.status],row.status==='complete'&&row.count!=null?String(row.count):'—',String(row.matches_observed),String(row.matches.length),row.evidence_complete?'Ja':'Nein']).sort((a,b)=>a[0].localeCompare(b[0],'de',{numeric:true,sensitivity:'base'}));
  await page.waitForFunction(expected=>{const rows=[...document.querySelectorAll('#results-table tr[data-row]')];return rows.length===expected.length&&JSON.stringify(rows.map(row=>[...row.querySelectorAll('td')].slice(0,9).map(cell=>cell.textContent.trim())))===JSON.stringify(expected);},expected);
  assert.equal(await page.locator('#results-table tr[data-row]').count(),expected.length);
}
(async()=>{
  await Promise.all([new Promise(r=>website.listen(0,'127.0.0.1',r)),new Promise(r=>external.listen(0,'127.0.0.1',r))]);externalURL=`http://localhost:${external.address().port}/external`;const seed=`http://127.0.0.1:${website.address().port}/dynamic`;
  const executablePath=process.env.SPIDER_CHROME||process.env.OPEN_SEO_BROWSER_PATH||await(await import('../../renderer/server.mjs')).findBrowserExecutable();
  const browser=await chromium.launch({headless:true,executablePath});const page=await browser.newPage({viewport:{width:1440,height:1000},acceptDownloads:true});page.on('pageerror',e=>errors.push(e.message));
  let disabled,enabled,visibility,unavailable,scanLimited,inputLimited,regexWorkLimited,importedId,coldId,coldOwner,coldProof;
  try{
    await page.goto(app);await page.locator('#connection-state.connected').waitFor();
    disabled=await start(page,seed);assert.equal(Object.hasOwn(disabled.request,'custom_search'),false);assert.equal(disabled.report.config.custom_search??null,null);
    assert(disabled.report.pages.every(row=>row.custom_search==null));await page.click('[data-section="custom-search"]');assert.equal(await page.locator('#results-table tr[data-row]').count(),0);assert.match(await page.locator('#results-table').innerText(),/nicht aktiviert|historisch nicht erfasst/);
    const beforeDisabledExport=count(),disabledCSV=await download(page,'#export-resources','/custom-search.csv'),disabledRows=parseCsv(disabledCSV.bytes.toString('utf8'));
    assert.equal(disabledCSV.response.headers()['x-content-sha256'],sha(disabledCSV.bytes));assert.equal(disabledRows.length,1);assert.equal(disabledRows[0].Status,'unavailable');assert.equal(disabledRows[0].Count,'');assert.equal(disabledRows[0]['Matches Observed'],'');assert.equal(disabledRows[0]['Rule Name'],'');assert.deepEqual(JSON.parse(disabledRows[0]['Reasons JSON']),['missing_saved_observation']);assert.deepEqual(count(),beforeDisabledExport);
    check('default-disabled native configuration stores no custom-search results; redesigned empty UI and actual native CSV keep missing/historical search unknown with blank count rather than zero');

    const beforeInvalid=count(),beforeJobs=(await api('/api/crawls')).crawls.map(row=>row.id).sort();
    const invalid=structuredClone(profile);invalid.rules[1].pattern='(?=unsupported-lookaround)';await settings(page,invalid);
    assert.deepEqual(count(),beforeInvalid);await start(page,seed,400);assert.deepEqual(count(),beforeInvalid);assert.deepEqual((await api('/api/crawls')).crawls.map(row=>row.id).sort(),beforeJobs);
    assert.deepEqual(await api('/api/crawls/'+disabled.id+'/project.json'),disabled.report);
    check('browser settings defer Rust regex semantics; native invalid-regex preflight rejects HTTP400 before any website request or new job and preserves prior report');

    await settings(page,profile);const savedSettings=await page.evaluate(()=>JSON.parse(localStorage.getItem('open-seo-spider-settings')));assert.equal(savedSettings.custom_search_enabled,true);assert.deepEqual(JSON.parse(savedSettings.custom_search_json),profile);
    await page.reload();await page.locator('#connection-state.connected').waitFor();await page.click('#settings-open');await page.locator('#advanced-config').evaluate(e=>e.open=true);await page.locator('#custom-search-settings').evaluate(e=>e.open=true);
    assert.equal(await page.isChecked('[name=custom_search_enabled]'),true);assert.deepEqual(JSON.parse(await page.inputValue('[name=custom_search_json]')),profile);await page.click('#settings-close');
    assert.deepEqual(count(),beforeInvalid,'Changing/restoring settings must not fetch the website');
    enabled=await start(page,seed);assert.deepEqual(enabled.request.custom_search,profile);assert.equal(Object.hasOwn(enabled.request,'custom_search_json'),false);assert.equal(Object.hasOwn(enabled.request,'custom_search_enabled'),false);
    const results=enabled.report.pages[0].custom_search;assert.equal(results.length,9);
    const expectedCounts={raw_html:3,rendered_html:4,raw_selected_text:2,rendered_selected_text:3};
    for(const name of [profile.rules[0].name,profile.rules[1].name])for(const source of sources){const result=results.find(row=>row.rule_name===name&&row.source===source);assert(result);assert.equal(result.status,'complete');assert.equal(result.count,expectedCounts[source]);assert.equal(result.matches_observed,result.count);assert.equal(result.matches.length,1);assert.equal(result.evidence_complete,false);assert.match(result.pattern_sha256,/^[a-f0-9]{64}$/);assert.match(result.config_sha256,/^[a-f0-9]{64}$/);assert.match(result.input_sha256,/^[a-f0-9]{64}$/);assert(result.observed_generation>0);assert(result.source_revision);assert.equal(result.document_url,seed);const match=result.matches[0];assert.equal(match.text,'TOKEN');assert.equal(match.end_byte-match.start_byte,5);assert.equal(match.match_utf8_bytes,5);assert.equal(match.text_truncated,false);if(source==='raw_html'){assert.equal(result.input_sha256,sha(Buffer.from(html)));assert.equal(match.start_byte,Buffer.from(html).indexOf('TOKEN'));}if(source.endsWith('_selected_text')){const capture=enabled.report.pages[0].content_captures.find(row=>row.source_kind===(source.startsWith('rendered')?'rendered_dom_html':'raw_http_html'));assert.equal(result.source_revision,capture.source_revision);assert.equal(result.input_sha256,capture.result.text_sha256);assert.equal(match.start_byte,Buffer.from(capture.result.text).indexOf('TOKEN'));}}
    const zero=results.find(row=>row.rule_name==='Confirmed zero');assert.equal(zero.status,'complete');assert.equal(zero.count,0);assert.deepEqual(zero.matches,[]);
    await table(page,results);assert.equal(await page.locator('#results-table b').count(),0);
    const captured=count();await page.locator('#results-table tr[data-row]').first().click();await page.locator('#custom-search-json').waitFor();assert.equal(await page.locator('#custom-search-json pre').count(),0);
    assert.equal(await page.locator('.custom-search-result').count(),9);assert.equal(await page.locator('.custom-search-result b').count(),0);
    await page.locator('#custom-search-json summary').click();await page.locator('#custom-search-json pre').waitFor();assert.deepEqual(JSON.parse(await page.locator('#custom-search-json pre').innerText()),results);
    const firstCard=page.locator('.custom-search-result').first();assert.equal(await firstCard.locator('.custom-search-evidence').count(),0);await firstCard.locator('details summary').click();await firstCard.locator('.custom-search-evidence').waitFor();assert.deepEqual(JSON.parse(await firstCard.locator('.custom-search-evidence').innerText()),results[0].matches);
    assert.equal(await page.locator('#inspector-content em, #inspector-content script').count(),0);assert.equal(await page.evaluate(()=>window.__searchXss??null),null);
    check('settings survive browser reload; actual Chrome raw/rendered HTML and selected-text counts3/4/2/3 prove excluded navigation and separate capture hashes, saved-example limit1 preserves exact totals, and lazy evidence stays safe text');

    const raw=await download(page,'#export-project','/project.json');assert.deepEqual(JSON.parse(raw.bytes),enabled.report);
    const full=await download(page,'#export-resources','/custom-search.csv');const exported=parseCsv(full.bytes.toString('utf8')),headers=full.response.headers();
    assert.equal(headers['x-content-sha256'],sha(full.bytes));assert.equal(Number(headers['content-length']),full.bytes.length);assert.equal(headers['cache-control'],'no-store');
    assert.equal(exported.length,results.length);assert(full.bytes.includes(Buffer.from(profile.rules[0].name)));
    const fields={'Document URL':'document_url','Rule Name':'rule_name','Method':'method','Pattern SHA256':'pattern_sha256','Configuration SHA256':'config_sha256','Source':'source','Status':'status','Count':'count','Matches Observed':'matches_observed','Evidence Complete':'evidence_complete','Input UTF8 Bytes':'input_utf8_bytes','Searched UTF8 Bytes':'searched_utf8_bytes','Input SHA256':'input_sha256','Observed Generation':'observed_generation','Source Revision':'source_revision','Engine':'engine'};
    for(let index=0;index<results.length;index++){const csv=exported[index],result=results[index];assert.equal(csv.URL,seed);for(const [column,key]of Object.entries(fields))assert.equal(csv[column],result[key]==null?'':String(result[key]),column);assert.equal(Number(csv['Saved Match Evidence']),result.matches.length);assert.deepEqual(JSON.parse(csv['Matches JSON']),result.matches);assert.deepEqual(JSON.parse(csv['Reasons JSON']),result.reasons);}
    fs.writeFileSync(artifactPath('custom-search-native.csv'),full.bytes);
    await page.selectOption('#table-filter','zero');await until(async()=>(await page.locator('#results-table tr[data-row]').count())===1,'known-zero filter');assert.match(await page.locator('#results-table tr[data-row]').innerText(),/Confirmed zero/);
    const filteredPending=page.waitForEvent('download');await page.click('#export-csv');const filteredBytes=fs.readFileSync(await(await filteredPending).path());
    const filteredRows=parseCsv(filteredBytes.toString('utf8').replace(/^\uFEFF/,''));assert.equal(filteredRows.length,1);assert.equal(filteredRows[0]['Suchregel'],'Confirmed zero');assert.equal(filteredRows[0]['Exakte Gesamtanzahl'],'0');
    check('saved project JSON and actual native full-search CSV download preserve captured results, while the UI zero filter selects only confirmed complete zero');
    assert.deepEqual(count(),captured);

    await page.selectOption('#table-filter','all');await table(page,results);
    const importing=captureResponse(page,r=>r.url()===app+'/api/crawls/import'&&r.request().method()==='POST');
    await page.setInputFiles('#import-file',{name:'owned-custom-search.json',mimeType:'application/json',buffer:raw.bytes});
    const {response:imported,bytes:importedBytes}=await importing;assert.equal(imported.status(),200,importedBytes.toString('utf8'));importedId=JSON.parse(importedBytes).id;assert.notEqual(importedId,enabled.id);
    await page.waitForFunction(id=>document.getElementById('crawl-history').value===id,importedId);await table(page,results);
    const importedReport=await api('/api/crawls/'+importedId+'/project.json');assert.deepEqual(importedReport.config.custom_search,enabled.report.config.custom_search);assert.deepEqual(importedReport.pages[0].custom_search,results);
    const importedCSV=await download(page,'#export-resources','/custom-search.csv');assert.deepEqual(importedCSV.bytes,full.bytes);assert.deepEqual(count(),captured);
    check('actual browser project import preserves the complete saved custom-search configuration and observations; declared imported native CSV stays byte-identical with zero website requests');
    await page.locator('#results-table tr[data-row]').first().click();await page.locator('#custom-search-json summary').click();await page.locator('#custom-search-json pre').waitFor();
    for(const width of [1440,320]){await page.setViewportSize({width,height:900});await page.locator('#export-resources').scrollIntoViewIfNeeded();const box=await page.locator('#export-resources').boundingBox();assert(box.y>=0&&box.y+box.height<=900);assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:artifactPath(`custom-search-${width}.png`),fullPage:true});}
    await page.setViewportSize({width:1440,height:1000});

    const visibleConfig={version:1,rules:[{name:'Visible and external',method:'literal',pattern:'TOKEN',sources:['raw_html']}],limits:profile.limits};
    const beforeVisibility=count();await settings(page,visibleConfig,false,false,{external:true,noindex:true});assert.deepEqual(count(),beforeVisibility);
    const rootURL=seed.replace('/dynamic','/visibility-root'),hiddenURL=seed.replace('/dynamic','/hidden');visibility=await start(page,rootURL);assert.equal(visibility.report.pages.length,3);
    const externalPage=visibility.report.pages.find(row=>row.url===externalURL);assert(externalPage);assert.equal(externalPage.internal,false);assert.equal(externalPage.status,200);assert.equal(externalPage.custom_search[0].count,2);assert((hits.get('external:/external')||0)>0);
    const hiddenPage=visibility.report.pages.find(row=>row.url===hiddenURL);assert(hiddenPage);assert.equal(hiddenPage.custom_search[0].count,4);assert.equal(visibility.report.config.directive_reporting.respect_noindex,true);
    const visibilityCaptured=count(),visibleRows=visibility.report.pages.filter(row=>row.url!==hiddenURL).flatMap(page=>page.custom_search.map(result=>({...result,url:page.url})));
    await table(page,visibleRows);const visibleCSV=await download(page,'#export-resources','/custom-search.csv'),visibleRecords=parseCsv(visibleCSV.bytes.toString('utf8'));
    assert.deepEqual(visibleRecords.map(row=>row.URL).sort(),[rootURL,externalURL].sort());assert.equal(visibleCSV.response.headers()['x-content-sha256'],sha(visibleCSV.bytes));assert(!visibleRecords.some(row=>row.URL===hiddenURL));
    const rawVisibility=await download(page,'#export-project','/project.json');assert.deepEqual(JSON.parse(rawVisibility.bytes).pages,visibility.report.pages);assert.deepEqual(count(),visibilityCaptured);
    check('genuinely checked external page remains visible beside internal search rows; active noindex profile hides exactly its captured page from UI/full CSV while raw JSON preserves all three native observations');

    const beforeUnavailable=count();await settings(page,profile,false,false);assert.deepEqual(count(),beforeUnavailable);unavailable=await start(page,seed);const absent=unavailable.report.pages[0].custom_search,unavailableCaptured=count();
    for(const row of absent){if(row.source==='raw_html'){assert.equal(row.status,'complete');assert.equal(row.count,3);}else{assert.equal(row.status,'unavailable');assert.equal(row.count,null);assert.equal(row.matches_observed,0);assert.deepEqual(row.matches,[]);}}
    await table(page,absent);await page.selectOption('#table-filter','zero');assert.equal(await page.locator('#results-table tr[data-row]').count(),0);
    await page.selectOption('#table-filter','unavailable');await until(async()=>(await page.locator('#results-table tr[data-row]').count())===7,'unavailable filter');
    assert.deepEqual(count(),unavailableCaptured);
    check('populated desktop/320px controls remain reachable; unavailable renderer/selected-text sources never fall back to raw HTML or become a successful zero');

    const bounded={version:1,rules:[{name:'Bounded scan',method:'literal',pattern:'TOKEN',sources:['raw_html']}],limits:{...profile.limits,max_matches_scanned:1}};
    await settings(page,bounded,false,false);assert.deepEqual(count(),unavailableCaptured);scanLimited=await start(page,seed);const scan=scanLimited.report.pages[0].custom_search[0],scanCaptured=count();assert.equal(scan.status,'incomplete');assert.equal(scan.count,null);assert.equal(scan.matches_observed,1);assert.equal(scan.evidence_complete,false);
    await page.selectOption('#table-filter','all');await table(page,[scan]);assert.deepEqual(count(),scanCaptured);
    const prefix=structuredClone(bounded);prefix.limits.max_matches_scanned=100;prefix.limits.max_input_bytes=16;
    prefix.rules.push({name:'Context-bound regex',method:'regex',pattern:'^.*$',regex_flags:'s',sources:['raw_html']});
    await settings(page,prefix,false,false);assert.deepEqual(count(),scanCaptured);inputLimited=await start(page,seed);const input=inputLimited.report.pages[0].custom_search[0],inputCaptured=count();assert.equal(input.status,'incomplete');assert.equal(input.count,null);assert.equal(input.matches_observed,0);assert.equal(input.searched_utf8_bytes,16);assert(input.input_utf8_bytes>16);await table(page,inputLimited.report.pages[0].custom_search);
    await page.selectOption('#table-filter','zero');assert.equal(await page.locator('#results-table tr[data-row]').count(),0);assert.deepEqual(count(),inputCaptured);
    const regexPartial=inputLimited.report.pages[0].custom_search[1];assert.equal(regexPartial.status,'incomplete');assert.equal(regexPartial.count,null);assert.equal(regexPartial.matches_observed,0);assert.equal(regexPartial.searched_utf8_bytes,0);assert.deepEqual(regexPartial.matches,[]);assert(regexPartial.reasons.includes('regex_partial_context_unknown'));
    check('real native scan/input budgets preserve unknown totals; truncated literal zero never enters confirmed-zero, and anchored regex refuses partial context without fabricated evidence');

    const work={version:1,rules:[{name:'Regex work budget',method:'regex',pattern:'TOKEN',sources:['raw_html']}],limits:{...profile.limits,max_regex_scan_work_bytes:1}};
    await settings(page,work,false,false);assert.deepEqual(count(),inputCaptured);regexWorkLimited=await start(page,seed);const workResult=regexWorkLimited.report.pages[0].custom_search[0],workCaptured=count();assert.equal(workResult.status,'incomplete');assert.equal(workResult.count,null);assert.equal(workResult.matches_observed,0);assert.deepEqual(workResult.matches,[]);assert(workResult.reasons.includes('regex_scan_work_limit'));await page.selectOption('#table-filter','all');await table(page,[workResult]);assert.deepEqual(count(),workCaptured);
    check('explicit JSON regex work budget reaches native engine and retains an incomplete unknown total with zero invented matches');

    const coldData=artifactPath('custom-search-cold-data');assert.equal(fs.existsSync(coldData),false,'Cold restart proof requires a fresh owned store');
    coldOwner=await ownedNative(coldData,'first');let coldPage=await browser.newPage();coldPage.on('pageerror',e=>errors.push(e.message));await coldPage.goto(coldOwner.origin);await coldPage.locator('#connection-state.connected').waitFor();
    const coldConfig={seed_urls:[seed],crawl_links:false,discover_sitemaps:false,obey_robots:false,delay_ms:0,custom_search:{version:1,rules:[{...profile.rules[0],sources:['raw_html']}],limits:profile.limits}};
    const coldStart=await coldPage.evaluate(async config=>{const response=await fetch('/api/crawls',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(config)});return {status:response.status,body:await response.json()};},coldConfig);
    assert.equal(coldStart.status,200);coldId=coldStart.body.id;
    await until(async()=>{const job=await api('/api/crawls/'+coldId,coldOwner.origin);assert.notEqual(job.status,'failed',job.error);return job.status==='completed';},'owned cold-proof native crawl');
    const coldBefore=await(await fetch(coldOwner.origin+'/api/crawls/'+coldId+'/project.json')).text(),firstOrigin=coldOwner.origin,binarySha=coldOwner.binarySha;
    const coldResults=JSON.parse(coldBefore).pages[0].custom_search,coldCaptured=count();assert.equal(coldResults[0].count,3);await coldPage.close();const firstShutdown=await coldOwner.stop();coldOwner=null;
    coldOwner=await ownedNative(coldData,'reopened');assert.equal(coldOwner.binarySha,binarySha,'Cold reopen must use exactly the first process binary');coldPage=await browser.newPage();coldPage.on('pageerror',e=>errors.push(e.message));await coldPage.goto(coldOwner.origin);await coldPage.locator('#connection-state.connected').waitFor();
    assert.equal(await(await fetch(coldOwner.origin+'/api/crawls/'+coldId+'/project.json')).text(),coldBefore);assert.deepEqual(count(),coldCaptured,'Cold native reopen must not refetch a completed crawl');
    await coldPage.selectOption('#crawl-history',coldId);await coldPage.waitForFunction(id=>document.getElementById('crawl-history').value===id&&document.querySelector('#crawl-state .status-dot')?.classList.contains('completed'),coldId);
    await table(coldPage,coldResults);const coldCSV=await download(coldPage,'#export-resources','/custom-search.csv');assert.equal(coldCSV.response.headers()['x-content-sha256'],sha(coldCSV.bytes));assert.equal(parseCsv(coldCSV.bytes.toString('utf8'))[0].Count,'3');assert.deepEqual(count(),coldCaptured);
    await coldPage.close();coldProof={crawl_id:coldId,first_origin:firstOrigin,reopened_origin:coldOwner.origin,binary_sha256:binarySha,first_shutdown:firstShutdown,reopened_shutdown:await coldOwner.stop()};coldOwner=null;
    check('two owned native processes share one fresh store: actual crawl results survive natural shutdown and cold reopen byte-for-byte, then redesigned browser results and verified CSV work without website refetch');
    assert.deepEqual(await api('/api/crawls/'+enabled.id+'/project.json'),enabled.report);assert.deepEqual(errors,[]);
    fs.writeFileSync(artifactPath('custom-search-live.json'),JSON.stringify({result:'PASS',checks,crawl_ids:{disabled:disabled.id,enabled:enabled.id,declared_import:importedId,external_and_reporting:visibility.id,unavailable:unavailable.id,scan_limited:scanLimited.id,input_limited:inputLimited.id,regex_work_limited:regexWorkLimited.id,cold_reopened:coldId},cold_proof:coldProof,website_requests:count(),analysis_refetches:0,attachment_checks:attachmentChecks,errors,
      scope:'Actual redesigned browser settings/native Rust preflight/crawler/owned Chrome renderer/saved JSON/native full CSV and rendered result cells. Own native cold restart verifies an additional real raw-source crawl in a fresh isolated store. Only owned synthetic pages. No reference regex/vendor parity, crash recovery or remote-process claim.'},null,2)+'\n');
    console.log(`${checks.length} actual native custom-search workflows passed`);
  }catch(error){await page.screenshot({path:artifactPath('custom-search-failure.png'),fullPage:true});fs.writeFileSync(artifactPath('custom-search-failure.json'),JSON.stringify({error:error.stack,checks,errors,hits:count(),message:await page.locator('#message').innerText(),settings_error:await page.locator('#settings-error').innerText(),layout:await page.evaluate(()=>({viewport:innerWidth,document:document.documentElement.scrollWidth}))},null,2)+'\n');throw error;}
  finally{await browser.close();if(coldOwner)await coldOwner.stop();}
})().catch(error=>{console.error(error);process.exitCode=1;}).finally(async()=>{for(const server of [website,external]){server.closeAllConnections();await new Promise(r=>server.close(r));}});
