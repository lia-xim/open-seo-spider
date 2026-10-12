// Native fixtures and local-only API/CLI controls; never uses the user's data/runtime.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import http from 'node:http';
import { mkdtemp, readFile, writeFile, mkdir, rm, stat } from 'node:fs/promises';
import { DatabaseSync } from 'node:sqlite';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const binary=process.env.SPIDER_BINARY||path.join(root,'target','debug',`open-seo-spider${process.platform==='win32'?'.exe':''}`);
const data=await mkdtemp(path.join(os.tmpdir(),'open-seo-spider-resources-'));
const api='http://127.0.0.1:18803',origin='http://127.0.0.1:19203',externalOrigin='http://127.0.0.1:19204';
const hits=new Map(),externalHits=[];
const fixture=http.createServer((request,response)=>{
  const url=new URL(request.url,origin);hits.set(url.pathname,(hits.get(url.pathname)||0)+1);
  response.setHeader('content-type','text/html; charset=utf-8');
  if(url.pathname==='/styles/first'){response.setHeader('content-type','text/css');response.end('@import "second"; .x{background:url("/image-no-ext")}');return;}
  if(url.pathname==='/styles/second'){response.setHeader('content-type','text/css');response.end('@import "first"; .x{background:url("/tiny.png")}');return;}
  if(url.pathname==='/app.js'){response.setHeader('content-type','application/javascript');response.end('console.log("owned static fixture");');return;}
  if(url.pathname==='/image-no-ext'||url.pathname==='/tiny.png'){response.setHeader('content-type','image/png');response.end(Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Y9Zl1QAAAAASUVORK5CYII=','base64'));return;}
  if(url.pathname==='/root'){response.end('<html><head><link rel="stylesheet" href="/styles/first"><script src="/app.js"></script></head><body><a href="/navigation">Navigation</a><img src="/image-no-ext" srcset="/tiny.png 2x, /image-no-ext 1x"></body></html>');return;}
  if(url.pathname==='/many'){response.end(`<html><body>${Array.from({length:505},(_,i)=>`<img src="/tiny.png" alt="repeat ${i}">`).join('')}</body></html>`);return;}
  if(url.pathname==='/unique'){response.end(`<html><body>${Array.from({length:505},(_,i)=>`<script src="/resource-${i}.js"></script>`).join('')}</body></html>`);return;}
  if(/^\/resource-\d+\.js$/.test(url.pathname)){response.setHeader('content-type','application/javascript');response.end('/* independently fetched owned resource */');return;}
  if(url.pathname==='/private'){response.end('<html><body><img src="/hidden"></body></html>');return;}
  if(url.pathname==='/hidden'){response.end('<html lang="de"><title>Known metadata</title><body><h1>Secret retained-source control</h1><a href="/outward-hidden">Outward</a></body></html>');return;}
  if(url.pathname==='/origins'){response.end(`<html><body><img src="${externalOrigin}/image"><a href="${externalOrigin}/navigation">External link</a></body></html>`);return;}
  response.end(`<html><title>${url.pathname}</title><body>Owned fixture</body></html>`);
});
const external=http.createServer((request,response)=>{externalHits.push(request.url);response.setHeader('content-type','image/png');response.end(Buffer.from('89504e47','hex'));});
await Promise.all([new Promise(r=>fixture.listen(19203,'127.0.0.1',r)),new Promise(r=>external.listen(19204,'127.0.0.1',r))]);
let stderr='';
const server=spawn(binary,['serve','--listen','127.0.0.1:18803','--data-dir',data],{cwd:root,windowsHide:true,stdio:['ignore','ignore','pipe']});
server.stderr.on('data',chunk=>{stderr+=chunk.toString();});
const delay=ms=>new Promise(r=>setTimeout(r,ms));
async function until(fn){for(let i=0;i<600;i++){const result=await fn();if(result)return result;if(server.exitCode!==null)throw Error(`server exited: ${stderr}`);await delay(25);}throw Error('Native resource acceptance timeout');}
async function json(route,body){const response=await fetch(api+route,body===undefined?{}:{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});const value=await response.json();assert.equal(response.ok,true,JSON.stringify(value));return value;}
async function start(name,config={}){const job=await json('/api/crawls',{seed_urls:[origin+name],obey_robots:false,discover_sitemaps:false,delay_ms:0,concurrency:1,...config});const status=await until(async()=>{const value=await json(`/api/crawls/${job.id}`);if(value.status==='failed')throw Error(value.error);return !['running','pausing','cancelling'].includes(value.status)&&value;});return {id:job.id,status,project:await json(`/api/crawls/${job.id}/project.json`)};}
async function cli(args){return await new Promise(resolve=>{const child=spawn(binary,args,{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});let stdout='',stderr='';child.stdout.on('data',c=>{stdout+=c;});child.stderr.on('data',c=>{stderr+=c;});child.on('error',error=>resolve({code:-1,stdout,stderr:error.message}));child.on('close',code=>resolve({code,stdout,stderr}));});}
async function allOccurrences(id,limit=200){const rows=[];let after;do{const page=await json(`/api/crawls/${id}/resources?limit=${limit}${after===undefined?'':`&after_sequence=${after}`}`);assert.equal(page.ledger_available,true);rows.push(...page.observations);if(!page.has_more)return {rows,checkpoint:page.checkpoint};assert.ok(page.next_after_sequence> (after||0));after=page.next_after_sequence;}while(true);}
try{
  await until(async()=>{try{return(await fetch(api+'/healthz')).ok;}catch{return false;}});
  const checks=[];
  const caps=await json('/api/capabilities');assert.equal(caps.native_resource_discovery.renderer_supported,false);
  const job=await start('/root',{crawl_links:false,indexability_agent:'  GOOGLEBOT ',resource_discovery:{}});assert.equal(job.status.config.indexability_agent,'googlebot');
  for(const name of ['/root','/styles/first','/styles/second','/app.js','/image-no-ext','/tiny.png'])assert.equal(hits.get(name),1,name);
  assert.equal(hits.get('/navigation'),undefined,'navigation master does not disable opted-in resources');
  const ledger=await allOccurrences(job.id,1);assert.equal(new Set(ledger.rows.map(r=>r.sequence)).size,ledger.rows.length);assert.equal(ledger.rows.length,ledger.checkpoint.decision_rows);
  assert.ok(ledger.rows.some(r=>r.occurrence.raw_value==='/styles/first'&&r.decision.grant?.can_expand_css));
  assert.ok(ledger.rows.some(r=>r.occurrence.classification_basis==='css_parser_declared_dependency'&&r.occurrence.resolved_url===origin+'/tiny.png'));
  assert.ok(ledger.rows.some(r=>r.occurrence.resolved_url===origin+'/navigation'&&!r.decision.grant));
  const snapshotHits=JSON.stringify([...hits]);await json(`/api/crawls/${job.id}/resources`);await json(`/api/crawls/${job.id}/resources/transports`);assert.equal(JSON.stringify([...hits]),snapshotHits);
  checks.push('native HTML resources + response-MIME CSS imports/cycle + responsive duplicates, navigation-disabled resource fetches, one transport per URL, bounded immutable occurrence cursor and read-only endpoints');

  const hidden=await start('/private',{crawl_links:false,content_capture:{},resource_discovery:{resources:{images:{store:false,fetch:true}}}});
  assert.equal(hits.get('/hidden'),1);assert.equal(hits.get('/outward-hidden'),undefined);assert.equal(hidden.project.pages.length,1);
  const transports=await json(`/api/crawls/${hidden.id}/resources/transports?limit=1`);assert.equal(transports.transports.length,1);assert.equal(transports.has_more,true);
  const next=await json(`/api/crawls/${hidden.id}/resources/transports?limit=100&after_url=${encodeURIComponent(transports.next_after_url)}`);
  const hiddenRow=[...transports.transports,...next.transports].find(row=>row.requested_url===origin+'/hidden');assert.ok(hiddenRow);assert.equal(hiddenRow.retained_response,false);
  assert.equal(hiddenRow.projection.resource_response.entity_body_retained,false);const hiddenCapture=hiddenRow.projection.content_captures.find(c=>c.source_kind==='raw_http_html');assert.ok(hiddenCapture);assert.equal(hiddenCapture.result.text,null);assert.equal(hiddenCapture.source_html_utf8_sha256,null);
  assert.equal(hiddenCapture.status,'unavailable');
  assert.equal(hidden.status.resource_ledger.completed_transports,2);assert.equal(hidden.status.stats.fetched,1);assert.equal(hidden.status.resource_ledger.retained_responses,1);
  checks.push('store=false actual HTTP observation remains bounded private metadata, body/source text unavailable, private responses counted separately from retained report rows');

  const beforeExternal=externalHits.length;
  await start('/origins',{crawl_links:true,resource_discovery:{resource_scope:{external_origins:[externalOrigin]}}});
  assert.deepEqual(externalHits.slice(beforeExternal),['/image']);
  checks.push('explicit external resource origin grants a resource transport without enabling external navigation');

  const many=await start('/many',{crawl_links:false,resource_discovery:{}});const inventory=await allOccurrences(many.id,200);assert.ok(inventory.rows.length>500);
  const response=await fetch(`${api}/api/crawls/${many.id}/resources.json`);assert.equal(response.status,200);const full=await response.json();assert.equal(full.observations.length,inventory.checkpoint.decision_rows);assert.equal(full.export.all_snapshot_rows_exported,true);
  assert.equal(response.headers.get('x-resource-snapshot-observations'),String(full.observations.length));assert.equal(response.headers.get('x-resource-ledger-sha256'),full.checkpoint.rolling_sha256);
  const jsonOutput=path.join(data,'full-ledger.json'),csvOutput=path.join(data,'full-ledger.csv');const exported=await cli(['resource-ledger',path.join(data,`${many.id}.sqlite`),'--json',jsonOutput,'--csv',csvOutput]);assert.equal(exported.code,0,exported.stderr);
  assert.deepEqual(JSON.parse(await readFile(jsonOutput,'utf8')),full);const csv=await readFile(csvOutput,'utf8');assert.equal(csv.split('\r\n').length-2,full.observations.length);assert.ok(csv.includes('complete_observation_json'));
  const beforeFile=await readFile(jsonOutput);const refused=await cli(['resource-ledger',path.join(data,`${many.id}.sqlite`),'--json',jsonOutput]);assert.notEqual(refused.code,0);assert.deepEqual(await readFile(jsonOutput),beforeFile);
  const transportCli=await cli(['resource-ledger',path.join(data,`${hidden.id}.sqlite`),'--transports','--limit','1']);assert.equal(transportCli.code,0,transportCli.stderr);assert.equal(JSON.parse(transportCli.stdout).rows.length,1);
  checks.push('505 repeated source candidates retained while URLs coalesce; complete pinned API/CLI JSON and CSV export beyond first page; existing CLI destinations preserved; transport CLI cursor');

  const unique=await start('/unique',{crawl_links:false,resource_discovery:{}});assert.equal(unique.project.pages.length,506);assert.equal(unique.status.resource_ledger.completed_transports,506);assert.equal(unique.status.status,'completed');
  for(let i=0;i<505;i++)assert.equal(hits.get(`/resource-${i}.js`),1);
  const limited=await start('/root',{crawl_links:false,max_pages:3,resource_discovery:{resources:{css:{store:false,fetch:true},javascript:{store:false,fetch:true},images:{store:false,fetch:true}}}});
  assert.equal(limited.status.status,'limited');assert.equal(limited.status.resource_ledger.completed_transports,3);assert.equal(limited.status.stats.fetched,1);assert.ok(limited.status.resource_ledger.pending_transports>0);assert.equal(limited.status.resource_ledger.complete,false);
  const beforeInvalid=JSON.stringify([...hits]);
  for(const invalid of [{resource_discovery:{},resource_discovery_limits:{max_source_candidates:0}},{resource_discovery:{},renderer:{endpoint:origin+'/never-render'}}]){
    const response=await fetch(api+'/api/crawls',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({seed_urls:[origin+'/never-invalid'],...invalid})});assert.equal(response.status,400);
  }
  assert.equal(JSON.stringify([...hits]),beforeInvalid);
  checks.push('506 distinct native URL transports without a 500-URL cap, all privately retained metadata responses count toward explicit max_pages, invalid limits and ungoverned browser combination rejected before HTTP');

  const disabled=await start('/legacy',{crawl_links:false});const unavailable=await json(`/api/crawls/${disabled.id}/resources`);assert.equal(unavailable.enabled,false);assert.equal(unavailable.ledger_available,false);assert.deepEqual(unavailable.observations,[]);
  assert.equal((await fetch(`${api}/api/crawls/${disabled.id}/resources.json`)).status,409);
  for(const suffix of ['/resources?limit=0','/resources?after_sequence=9223372036854775808','/resources/transports?after_url=','/resources?unsupported=1'])assert.equal((await fetch(`${api}/api/crawls/${job.id}${suffix}`)).status,400);
  checks.push('legacy mode remains opt-out; missing native ledger never becomes a fabricated empty full export; strict invalid cursor/limit/unknown query rejection');

  const cssInput={source:{css_text:'@import "theme.css"; a{background:image-set("first.png" 1x, url("second.png") 2x)}',base_url:origin+'/css/main',source_url:origin+'/css/main',source_revision:'supplied-own-css-v1',inline:false}};
  const before=JSON.stringify([...hits]);const cssApi=await json('/api/css/inspect',cssInput);assert.equal(cssApi.website_requests,0);assert.equal(cssApi.dependencies.length,3);assert.equal(cssApi.dependencies[0].resolved_url,origin+'/css/theme.css');
  const cssFile=path.join(data,'css-input.json');await writeFile(cssFile,JSON.stringify(cssInput));const cssCli=await cli(['inspect-css',cssFile]);assert.equal(cssCli.code,0,cssCli.stderr);assert.deepEqual(JSON.parse(cssCli.stdout),cssApi);assert.equal(JSON.stringify([...hits]),before);
  assert.equal((await fetch(api+'/api/css/inspect',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({...cssInput,limits:{max_nesting:0}})})).status,400);
  checks.push('declared-source bounded CSS parser API/CLI equality, image-set candidates and @import resolution without HTTP, invalid parser limit rejection');

  const beforeSelected=new Map(hits);const selected=await json(`/api/crawls/${job.id}/recrawl`,{urls:[origin+'/root']});await until(async()=>{const s=await json(`/api/crawls/${selected.id}`);if(s.status==='failed')throw Error(s.error);return s.status==='completed';});
  assert.equal(hits.get('/root'),beforeSelected.get('/root')+1);for(const name of ['/styles/first','/styles/second','/app.js','/image-no-ext','/tiny.png'])assert.equal(hits.get(name),beforeSelected.get(name),name);
  const childLedger=await json(`/api/crawls/${selected.id}/resources`);assert.equal(childLedger.checkpoint.inherited_history.rows_available,false);assert.equal(childLedger.checkpoint.complete,false);
  checks.push('selected recrawl emits fresh source evidence with no outward resource/navigation HTTP and declares unavailable inherited ledger history');

  // Only this script's completed temporary fixture store is mutated. Each
  // mutation is restored to the exact original cell even if an assertion fails.
  // No hash/checkpoint is recomputed to make altered evidence appear valid.
  const fixtureStorePath=path.join(data,`${job.id}.sqlite`);
  assert.equal(path.dirname(path.resolve(fixtureStorePath)),path.resolve(data));
  assert.equal((await stat(fixtureStorePath)).isFile(),true);
  const fixtureDatabase=new DatabaseSync(fixtureStorePath,{timeout:5000});
  const tamperControls=[];
  const beforeTamperRequests=JSON.stringify({native:[...hits],external:externalHits});
  async function mutatedCell({name,select,update,key,change,expectedStatus}) {
    const reader=fixtureDatabase.prepare(select),writer=fixtureDatabase.prepare(update);
    const original=reader.get(key)?.value;assert.equal(typeof original,'string',`${name} original cell exists`);
    const altered=change(JSON.parse(original));assert.notEqual(altered,original,`${name} changes the owned cell`);
    const observed=[];
    try {
      assert.equal(Number(writer.run(altered,key).changes),1);
      for(const route of ['/resources','/resources/transports','/resources.json','/resources.csv']) {
        const response=await fetch(`${api}/api/crawls/${job.id}${route}`);
        const body=await response.text();const status=expectedStatus(route);
        assert.equal(response.status,status,`${name} ${route}: ${body}`);
        assert.ok(body.length>0,`${name} ${route} explicit error`);
        observed.push({route,status:response.status});
      }
    } finally {
      assert.equal(Number(writer.run(original,key).changes),1);
      assert.equal(reader.get(key)?.value,original,`${name} restores exact original cell`);
    }
    const restored=await json(`/api/crawls/${job.id}/resources?limit=1`);
    assert.equal(restored.ledger_available,true);assert.equal(restored.observations.length,1);
    const restoredTransports=await json(`/api/crawls/${job.id}/resources/transports?limit=1`);
    assert.equal(restoredTransports.ledger_available,true);assert.equal(restoredTransports.transports.length,1);
    assert.equal(JSON.stringify({native:[...hits],external:externalHits}),beforeTamperRequests,`${name} reads cause zero fixture requests`);
    tamperControls.push({name,rejected:observed,exact_cell_restored:true,positive_read_after_restore:true,fixture_requests_added:0});
  }
  try {
    const sequence=fixtureDatabase.prepare('SELECT sequence FROM resource_occurrences ORDER BY sequence DESC LIMIT 1').get()?.sequence;
    assert.ok(Number.isSafeInteger(sequence)&&sequence>0,'positive native occurrence control');
    await mutatedCell({name:'occurrence_payload_without_rolling_hash',key:sequence,
      select:'SELECT data AS value FROM resource_occurrences WHERE sequence=?',
      update:'UPDATE resource_occurrences SET data=? WHERE sequence=?',
      change:row=>{row.occurrence.raw_value+=' altered-owned-control';return JSON.stringify(row);},
      expectedStatus:route=>route.endsWith('.json')||route.endsWith('.csv')?409:500});
    const transportUrl=fixtureDatabase.prepare('SELECT url FROM resource_transports ORDER BY url LIMIT 1').get()?.url;
    assert.equal(typeof transportUrl,'string','observed native transport control');
    await mutatedCell({name:'transport_projection_without_payload_hash',key:transportUrl,
      select:'SELECT data AS value FROM resource_transports WHERE url=?',
      update:'UPDATE resource_transports SET data=? WHERE url=?',
      change:row=>{row.projection.url=origin+'/altered-owned-projection';return JSON.stringify(row);},
      expectedStatus:route=>route.endsWith('.json')||route.endsWith('.csv')?409:500});
    await mutatedCell({name:'different_full_config_with_same_resource_config',key:'config',
      select:'SELECT value FROM metadata WHERE key=?',update:'UPDATE metadata SET value=? WHERE key=?',
      change:config=>{config.user_agent+=' changed-owned-control';return JSON.stringify(config);},expectedStatus:()=>409});
    await mutatedCell({name:'different_sqlite_generation',key:'run_state',
      select:'SELECT value FROM metadata WHERE key=?',update:'UPDATE metadata SET value=? WHERE key=?',
      change:state=>{assert.ok(Number.isSafeInteger(state.generation)&&state.generation>0);state.generation+=1;return JSON.stringify(state);},expectedStatus:()=>409});
  } finally {fixtureDatabase.close();}
  assert.deepEqual(await json(`/api/crawls/${job.id}/project.json`),job.project,'read-only integrity probes preserve cached completed project');
  checks.push('four directed owned-SQLite tamper controls reject altered payload hashes, full-config mismatch and generation mismatch across paged readers and streamed exports; exact restoration plus positive read controls and zero fixture requests');

  const evidence={tested_at:new Date().toISOString(),result:'PASS',platform:process.platform,binary_sha256:createHash('sha256').update(await readFile(binary)).digest('hex'),checks,groups:checks.length,tamper_controls:tamperControls,fixture_website_requests:[...hits],external_fixture_requests:externalHits,scope:'native local HTML/CSS resource and persisted API/CLI fixtures; full commercial parity NOT_PROVEN'};
  console.log(JSON.stringify(evidence,null,2));
  if(process.env.SPIDER_EVIDENCE){await mkdir(path.dirname(process.env.SPIDER_EVIDENCE),{recursive:true});await writeFile(process.env.SPIDER_EVIDENCE,JSON.stringify(evidence,null,2)+'\n');}
}finally{
  if(server.exitCode===null){const stopped=new Promise(r=>server.once('exit',r));server.kill();await stopped;}
  await Promise.all([new Promise(r=>fixture.close(r)),new Promise(r=>external.close(r))]);
  assert.equal(path.dirname(path.resolve(data)),path.resolve(os.tmpdir()));assert.ok(path.basename(data).startsWith('open-seo-spider-resources-'));await rm(data,{recursive:true,force:true});
}
