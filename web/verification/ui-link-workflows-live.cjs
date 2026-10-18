'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),http=require('node:http'),crypto=require('node:crypto');
const {chromium}=require('../../renderer/node_modules/playwright-core');
const artifactPath=require('./artifact-path.cjs');
const csvRecords=require('./csv-records.cjs');
const app=process.env.SPIDER_APP_URL;
assert(app&&new URL(app).hostname==='127.0.0.1'&&!['8787','8790'].includes(new URL(app).port),'Use an isolated test app');
const checks=[],errors=[];let hits=0;
const website=http.createServer((req,res)=>{
  hits++;res.setHeader('content-type','text/html; charset=utf-8');
  if(req.url==='/old'){res.writeHead(301,{location:'/good'});return res.end();}
  if(req.url==='/missing')res.statusCode=404;
  res.end('<!doctype html><html lang="en"><head><title>Owned link workflow</title></head><body><main>'+
    (req.url==='/root'?Array.from({length:105},(_,i)=>`<a href="/missing">Missing occurrence ${i}</a>`).join('')+'<a href="/good">Good target</a><a href="/unobserved">Unknown target</a>':'Saved owned document')+'</main></body></html>');
});
const sha=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
async function api(route,body){const response=await fetch(app+route,{method:body===undefined?'GET':'POST',headers:body===undefined?{}:{'content-type':'application/json'},body:body===undefined?undefined:JSON.stringify(body)});assert.equal(response.status,200,await response.clone().text());return response.json();}
async function until(work,label){for(let i=0;i<300;i++){if(await work())return;await new Promise(r=>setTimeout(r,50));}throw Error('Timeout '+label);}
async function action(page,selector,suffix){const pending=page.waitForResponse(r=>r.url().endsWith(suffix)&&r.request().method()==='POST');await page.click(selector);const response=await pending;assert.equal(response.status(),200,await response.text());const value=await response.json();await until(()=>page.isEnabled(selector),'UI action completion');const cards=page.locator('#rows article');assert.equal(await cards.count(),value.rows.length);for(let i=0;i<value.rows.length;i++){const row=value.rows[i],card=cards.nth(i);assert.equal(await card.getAttribute('data-verdict'),row.verdict);if(suffix.includes('/broken-sources/')){assert.deepEqual(JSON.parse(await card.locator('pre').textContent()),row);assert((await card.innerText()).includes(row.link.text));}else{assert.deepEqual((await card.locator('dd').allTextContents()).slice(0,2),[row.old_value,row.expected_value]);assert((await card.locator('h3').innerText()).includes('CSV-Zeile '+row.source_row));}}return value;}
async function downloaded(page,selector,suffix){const event=page.waitForEvent('download'),pending=page.waitForResponse(r=>r.url().endsWith(suffix));await page.click(selector);const response=await pending;assert.equal(response.status(),200);const actual=fs.readFileSync(await(await event).path()),expected=await response.body();assert.deepEqual(actual,expected);assert.equal(actual.length,Number(response.headers()['content-length']));assert.equal(sha(actual),response.headers()['x-content-sha256']);return actual;}
async function responsive(page,name){for(const width of [1440,320]){await page.setViewportSize({width,height:900});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),name+' overflow '+width);await page.screenshot({path:artifactPath(name+'-'+width+'.png'),fullPage:true});}}
(async()=>{
  await new Promise(r=>website.listen(0,'127.0.0.1',r));const origin=`http://127.0.0.1:${website.address().port}`;
  const job=await api('/api/crawls',{seed_urls:[origin+'/root',origin+'/old'],crawl_links:true,obey_robots:false,discover_sitemaps:false,delay_ms:0,exclude_patterns:['/unobserved$']});
  await until(async()=>{const state=await api('/api/crawls/'+job.id);assert.notEqual(state.status,'failed',state.error);return state.status==='completed';},'native link crawl');
  const capturedHits=hits,original=await api('/api/crawls/'+job.id+'/project.json');
  assert(!original.pages.some(p=>p.url===origin+'/unobserved'));
  const executablePath=process.env.SPIDER_CHROME||process.env.OPEN_SEO_BROWSER_PATH||await(await import('../../renderer/server.mjs')).findBrowserExecutable();
  const browser=await chromium.launch({headless:true,executablePath}),page=await browser.newPage({acceptDownloads:true});
  page.on('pageerror',error=>errors.push(error.message));
  try{
    await page.goto(app+'/broken-sources');await page.selectOption('#saved-crawl',job.id);await page.selectOption('#outcome','all');
    const all=await action(page,'#preview','/broken-sources/preview');assert.equal(all.website_requests,0);assert.equal(all.total_retained_link_occurrences,107);assert.equal(all.rows.length,100);assert.equal(all.summary.observed_http_failure,105);assert.equal(all.summary.unknown,1);assert.equal(all.summary.observed_non_failure,1);
    assert.equal(await page.locator('#rows article').count(),100);const remaining=await action(page,'#next-page','/broken-sources/preview');assert.equal(remaining.rows.length,7);assert.equal(await page.locator('#rows article').count(),7);assert.equal(new Set([...all.rows,...remaining.rows].map(r=>r.source.page_ordinal+':'+r.link_ordinal)).size,107);
    const csv=await downloaded(page,'#export-csv','/broken-sources/csv');assert.match(csv.toString(),/Missing occurrence 0/);assert.match(csv.toString(),/Missing occurrence 104/);assert.match(csv.toString(),/Unknown target/);assert.match(csv.toString(),/observed_http_failure/);const csvLinks=csvRecords(csv);assert.equal(csvLinks.length,107);assert.deepEqual(csvLinks.map(r=>r.source_page_ordinal+':'+r.link_ordinal).sort(),[...all.rows,...remaining.rows].map(r=>r.source.page_ordinal+':'+r.link_ordinal).sort());
    checks.push('native saved link occurrences preserve all105 repeated404 links and missing observations across100+7 UI pages and full verified CSV');
    await page.selectOption('#outcome','broken');assert(await page.isDisabled('#export-csv'));const broken=await action(page,'#preview','/broken-sources/preview');assert.equal(broken.filtered_rows,105);assert(broken.rows.every(r=>r.verdict==='observed_http_failure'));
    await page.selectOption('#outcome','unknown');const unknown=await action(page,'#preview','/broken-sources/preview');assert.equal(unknown.rows.length,1);assert.equal(unknown.rows[0].observation_count,0);assert.match(await page.locator('#rows').innerText(),/Unbekannt/);
    await responsive(page,'broken-sources');checks.push('broken/unknown filter changes invalidate exports and keep unobserved targets unknown at desktop and320px');
    await page.goto(app+'/migration');await page.selectOption('#saved-crawl',job.id);
    const mapping=[...Array.from({length:105},()=>[origin+'/old',origin+'/good']),[origin+'/old',origin+'/wrong'],[origin+'/not-saved',origin+'/good'],[origin+'/missing',origin+'/missing'],['=SUM(1)',origin+'/good']];
    const input='old,expected\r\n'+mapping.map(row=>row.join(',')).join('\r\n')+'\r\n';
    await page.setInputFiles('#migration-file',{name:'owned-migration.csv',mimeType:'text/csv',buffer:Buffer.from(input)});await until(async()=>(await page.locator('#file-state').innerText()).includes('owned-migration.csv'),'CSV load');
    const first=await action(page,'#preview','/migration/preview');assert.equal(first.total_rows,109);assert.equal(first.rows.length,100);assert.equal(first.summary.destination_match,105);assert.equal(first.summary.destination_mismatch,1);assert.equal(first.summary.missing_observation,1);assert.equal(first.summary.final_http_failure,1);assert.equal(first.summary.invalid_mapping,1);assert.equal(first.rows[0].observation.redirect_chain[0].status,301);assert(first.rows[1].duplicate_of_row!=null);assert(first.rows[0].conflicting_expectations);
    const next=await action(page,'#next-page','/migration/preview');assert.equal(next.rows.length,9);assert.equal(new Set([...first.rows,...next.rows].map(r=>r.source_row)).size,109);
    const filteredResponse=page.waitForResponse(r=>r.url().endsWith('/migration/preview')&&r.request().postDataJSON().outcome==='destination_mismatch');await page.selectOption('#outcome','destination_mismatch');const filtered=await filteredResponse;assert.equal(filtered.status(),200);assert.equal((await filtered.json()).filtered_rows,1);await until(async()=>await page.locator('#rows article').count()===1&&await page.locator('#rows article[data-verdict="destination_mismatch"]').count()===1,'migration outcome filter');assert.match(await page.locator('#rows').innerText(),/\/wrong/);
    const envelope=JSON.parse((await downloaded(page,'#export-json','/migration/json')).toString()),report=envelope.report;assert.equal(report.rows.length,109);assert.equal(report.website_requests,0);assert.equal(report.rows.filter(r=>r.verdict==='destination_match').length,105);assert.equal(envelope.snapshot_token,first.snapshot_token);
    const migrationCSV=await downloaded(page,'#export-csv','/migration/csv');assert.match(migrationCSV.toString(),/invalid_mapping/);assert.match(migrationCSV.toString(),/SUM\(1\)/);const csvMappings=csvRecords(migrationCSV);assert.equal(csvMappings.length,109);for(let i=0;i<109;i++){assert.equal(Number(csvMappings[i].source_row),report.rows[i].source_row);assert.equal(csvMappings[i].outcome,report.rows[i].verdict);assert.equal(csvMappings[i].old_original,report.rows[i].old_value.startsWith('=')?"'"+report.rows[i].old_value:report.rows[i].old_value);assert.equal(csvMappings[i].expected_original,report.rows[i].expected_value);assert.equal(csvMappings[i].snapshot_token,first.snapshot_token);}
    checks.push('native migration retains duplicate/conflicting mapping rows and redirect/404/missing/invalid outcomes;100+9 paging and filtered UI export all109 original rows with verified byte hashes');
    await responsive(page,'migration');await page.fill('#old-column','2');await page.locator('#old-column').dispatchEvent('change');assert(await page.isDisabled('#export-json'));assert(await page.isDisabled('#export-csv'));assert(await page.locator('#migration-results').isHidden());
    checks.push('mapping option changes invalidate snapshot and exports; populated migration is contained at320px');
    assert.equal(hits,capturedHits);assert.deepEqual(await api('/api/crawls/'+job.id+'/project.json'),original);assert.deepEqual(errors,[]);
    fs.writeFileSync(artifactPath('link-workflows.json'),JSON.stringify({result:'PASS',checks,crawl_id:job.id,website_requests_after_crawl:hits-capturedHits,javascript_errors:errors,scope:'Actual browser/Rust/saved report/export for migration and retained page-link sources. Resource ledger sources and cold native restart are separate.'},null,2)+'\n');
    console.log(checks.length+' actual native link/migration workflows passed');
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;}).finally(()=>new Promise(resolve=>{website.close(resolve);website.closeAllConnections();}));
