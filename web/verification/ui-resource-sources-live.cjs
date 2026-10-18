'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),http=require('node:http'),crypto=require('node:crypto');
const {chromium}=require('../../renderer/node_modules/playwright-core');
const artifactPath=require('./artifact-path.cjs');
const csvRecords=require('./csv-records.cjs');
const app=process.env.SPIDER_APP_URL;
assert(app&&new URL(app).hostname==='127.0.0.1'&&!['8787','8790'].includes(new URL(app).port),'Use an isolated test app');
const checks=[],errors=[],hits={},readOracles=[],ownedIds=new Set();
const website=http.createServer((req,res)=>{
  hits[req.url]=(hits[req.url]||0)+1;
  if(req.url==='/root'){res.setHeader('content-type','text/html');return res.end('<html><head><title>Owned resources</title><link rel="stylesheet" href="/missing.css"></head><body>'+Array.from({length:105},(_,i)=>`<img alt="Owned occurrence ${i}" src="/missing.svg">`).join('')+'<img src="/good.svg"><script src="/deferred.js"></script></body></html>');}
  if(req.url==='/missing.css'){res.writeHead(404,{'content-type':'text/css'});return res.end('/* owned missing CSS */');}
  if(req.url==='/deferred.js')throw Error('Disabled resource fetch must never request deferred JavaScript');
  res.setHeader('content-type','image/svg+xml');if(req.url==='/missing.svg')res.statusCode=404;res.end('<svg xmlns="http://www.w3.org/2000/svg" width="2" height="3"/>');
});
async function api(route,body){const response=await fetch(app+route,{method:body===undefined?'GET':'POST',headers:body===undefined?{}:{'content-type':'application/json'},body:body===undefined?undefined:JSON.stringify(body)});assert.equal(response.status,200,await response.clone().text());return response.json();}
async function until(work,label){for(let i=0;i<300;i++){if(await work())return;await new Promise(r=>setTimeout(r,25));}throw Error('Timeout '+label);}
async function nativeReadOracle(response,action){
  const url=new URL(response.url()),match=/^\/api\/crawls\/([0-9a-f-]{36})\/resource-broken-sources\/(preview|csv)$/.exec(url.pathname),request=response.request();
  assert.equal(url.origin,app);assert.equal(url.search,'');assert(match&&ownedIds.has(match[1])&&match[2]===action);assert.equal(request.method(),'POST');
  const body=request.postData();assert.equal(typeof body,'string');assert(body.length>0);assert(JSON.parse(body)&&typeof JSON.parse(body)==='object');
  // Both whitelisted POSTs are pure saved-ledger reads; mutation requests are never replayed.
  const native=await fetch(url,{method:'POST',headers:{'content-type':'application/json'},body}),bytes=Buffer.from(await native.arrayBuffer());
  assert.equal(response.status(),200);assert.equal(native.status,200,bytes.toString());const digest=crypto.createHash('sha256').update(bytes).digest('hex');
  for(const headers of[response.headers(),Object.fromEntries(native.headers)]){assert.equal(headers['content-length'],String(bytes.length));assert.equal(headers['x-content-sha256'],digest);assert.equal(headers['cache-control'],'no-store');assert.equal(headers['content-type']?.split(';')[0],action==='csv'?'text/csv':'application/json');}
  readOracles.push({path:url.pathname,method:'POST',body,bytes:bytes.length,sha256:digest});return bytes;
}
async function preview(page,button='#preview'){const pending=page.waitForResponse(r=>r.request().method()==='POST'&&r.url().startsWith(app+'/api/crawls/')&&r.url().endsWith('/resource-broken-sources/preview'));await page.click(button);const response=await pending;const value=JSON.parse(await nativeReadOracle(response,'preview'));await until(()=>page.isEnabled('#preview'),'resource UI ready');assert(await page.locator('#results').isVisible(),await page.locator('#status').innerText());const cards=page.locator('#rows article');assert.equal(await cards.count(),value.page.rows.length);for(let i=0;i<value.page.rows.length;i++){assert.equal(await cards.nth(i).getAttribute('data-state'),value.page.rows[i].state);assert.deepEqual(JSON.parse(await cards.nth(i).locator('pre').textContent()),value.page.rows[i]);}return value;}
(async()=>{
  await new Promise(r=>website.listen(0,'127.0.0.1',r));const seed=`http://127.0.0.1:${website.address().port}/root`;
  const job=await api('/api/crawls',{seed_urls:[seed],crawl_links:false,obey_robots:false,discover_sitemaps:false,delay_ms:0,resource_discovery:{resources:{css:{store:true,fetch:true},images:{store:true,fetch:true},javascript:{store:true,fetch:false}}}});
  ownedIds.add(job.id);
  await until(async()=>{const state=await api('/api/crawls/'+job.id);assert.notEqual(state.status,'failed',state.error);return state.status==='completed';},'native resource fixture');
  const captured={...hits},original=await api('/api/crawls/'+job.id+'/project.json');assert.equal(hits['/missing.svg'],1);assert.equal(hits['/deferred.js'],undefined);
  const executablePath=process.env.SPIDER_CHROME||process.env.OPEN_SEO_BROWSER_PATH||await(await import('../../renderer/server.mjs')).findBrowserExecutable();
  const browser=await chromium.launch({headless:true,executablePath}),page=await browser.newPage({acceptDownloads:true});page.on('pageerror',e=>errors.push(e.message));
  try{
    await page.goto(app+'/resource-broken-sources');await page.selectOption('#crawl',job.id);const first=await preview(page);assert.equal(first.scope,'verified_resource_ledger_occurrences');assert.equal(first.page.available,true);assert.equal(first.page.rows.length,100);assert.equal(first.page.selected_distinct_occurrence_ids,106);assert(first.page.rows.every(r=>r.state==='observed_http_failure'));assert.equal(first.page.snapshot.website_requests,0);
    const last=await preview(page,'#next');assert.equal(last.page.rows.length,6);assert.equal(new Set([...first.page.rows,...last.page.rows].map(r=>r.observation.occurrence.occurrence_id)).size,106);
    const download=page.waitForEvent('download');await page.click('#json');const jsonBytes=fs.readFileSync(await(await download).path());assert.deepEqual(JSON.parse(jsonBytes),last);
    const csvEvent=page.waitForEvent('download'),csvResponse=page.waitForResponse(r=>r.url().endsWith('/resource-broken-sources/csv'));await page.click('#csv');const response=await csvResponse;assert.equal(response.status(),200);const csv=fs.readFileSync(await(await csvEvent).path());assert.deepEqual(csv,await nativeReadOracle(response,'csv'));assert.equal(csv.length,Number(response.headers()['content-length']));assert.equal(crypto.createHash('sha256').update(csv).digest('hex'),response.headers()['x-content-sha256']);assert.equal(response.headers()['x-resource-source-snapshot'],last.snapshot_token);assert.equal(response.headers()['x-resource-transport-set-sha256'],last.page.snapshot.transport_set_sha256);for(const row of [...first.page.rows,...last.page.rows])assert(csv.includes(Buffer.from(row.observation.occurrence.occurrence_id)));
    assert.equal(csvRecords(csv).length,106);checks.push('real saved resource ledger keeps106 distinct failing HTML/CSS occurrences despite shared transports;100+6 paging, exact pageJSON and complete snapshot/hash-bound CSV');
    await page.selectOption('#filter','unknown');assert(await page.isDisabled('#csv'));const unknown=await preview(page);assert.equal(unknown.page.rows.length,1);assert.equal(unknown.page.rows[0].observation.occurrence.raw_value,'/deferred.js');assert.equal(unknown.page.rows[0].target_transport,null);assert.equal(unknown.page.rows[0].state,'no_grant');assert.equal(await page.locator('#rows article[data-state="no_grant"]').count(),1);assert.match(await page.locator('#rows').innerText(),/\/deferred\.js/);
    checks.push('store-only JavaScript has unknown transport instead of an invented successful zero, with exports invalidated on filter change');
    await page.selectOption('#filter','all');const all=await preview(page);for(const width of [1440,320]){await page.setViewportSize({width,height:900});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'resource sources overflow '+width);await page.screenshot({path:artifactPath('resource-sources-'+width+'.png'),fullPage:true});}
    const bad=await fetch(app+'/api/crawls/'+job.id+'/resource-broken-sources/csv',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({filter:'unknown',limit:100,expected_generation:all.generation,expected_snapshot_token:all.snapshot_token})});assert.equal(bad.status,409);await bad.body.cancel();
    const imported=await api('/api/crawls/import',original);ownedIds.add(imported.id);await page.click('#reload');await page.selectOption('#crawl',imported.id);const unavailable=await preview(page);assert.equal(unavailable.page.available,false);assert(await page.isDisabled('#csv'));assert.match(await page.locator('#basis').innerText(),/Nicht verfügbar/);
    checks.push('changed query cannot reuse stale native export token; imported report without SQLite history remains unavailable at populated desktop/mobile');
    assert.deepEqual(hits,captured);assert.deepEqual(await api('/api/crawls/'+job.id+'/project.json'),original);assert.deepEqual(errors,[]);
    fs.writeFileSync(artifactPath('resource-sources-live.json'),JSON.stringify({result:'PASS',checks,read_only_native_oracles:readOracles,website_requests_after_crawl:0,errors,scope:'Actual native resource discovery, SQLite ledger UI, filtering, pagination and complete exports. CSS ancestor reconstruction and cold process restart are separate.'},null,2)+'\n');console.log(checks.length+' native resource-source workflows passed');
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;}).finally(()=>new Promise(resolve=>{website.close(resolve);website.closeAllConnections();}));
