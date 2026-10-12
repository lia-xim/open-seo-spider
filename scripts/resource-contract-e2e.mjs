// Native V2 behavior through the real API, CLI and official MCP SDK.
// All targets and data directories belong to this fixture, never to the user runtime.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import http from 'node:http';
import {mkdtemp,readFile,writeFile,mkdir,rm} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {DatabaseSync} from 'node:sqlite';
import {Client,StreamableHTTPClientTransport} from '@modelcontextprotocol/client';
import {StdioClientTransport} from '@modelcontextprotocol/client/stdio';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const binary=process.env.SPIDER_BINARY||path.join(root,'target/debug',`open-seo-spider${process.platform==='win32'?'.exe':''}`);
const data=await mkdtemp(path.join(os.tmpdir(),'open-seo-spider-contract-'));
const ENGINE='open-seo-spider/resource-contract-v2';
const hits=[],checks=[],jobs=[],tamperControls=[],ownedInstances=[];let api,siteOrigin,assetOrigin,thirdOrigin,server,stderr='',passed=false,foreignRequests=0;
const foreign=http.createServer((_req,res)=>{foreignRequests++;res.setHeader('content-type','application/json');res.end('{"ok":true}');});
const proof=process.env.SPIDER_EVIDENCE||path.join(root,'docs/verification/resource-contract-e2e.json');
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const qualifiers=['nofollow','ugc','sponsored'];
function response(kind,request,res){hits.push({kind,url:request.url,authorization:request.headers.authorization??null,cookie:request.headers.cookie??null});res.setHeader('content-type','text/html; charset=utf-8');}
const site=http.createServer((req,res)=>{
  response('site',req,res);
  const pathname=new URL(req.url,'http://owned.test').pathname;
  if(/^\/rel-/.test(pathname)&&!pathname.slice(1).includes('/')){
    res.end(`<html><body>${qualifiers.map(q=>`<a href="${pathname}/${q}" rel="${q}">internal ${q}</a><a href="${assetOrigin}${pathname}/${q}" rel="${q}">external ${q}</a>`).join('')}<a href="${pathname}/plain">independent unqualified control</a></body></html>`);return;
  }
  if(/^\/doc-/.test(pathname)&&!pathname.slice(1).includes('/')){
    if(pathname.includes('header'))res.setHeader('x-robots-tag','nofollow');
    res.end(`<html><head>${pathname.includes('meta')?'<meta name="robots" content="nofollow">':''}</head><body><a href="${pathname}/internal">internal</a><a href="${assetOrigin}${pathname}/external">external</a></body></html>`);return;
  }
  if(pathname==='/headers'){
    res.setHeader('link','</headers/http>; rel="canonical nofollow"');
    res.end('<html><head><link rel="canonical nofollow" href="/headers/html"></head><body>own equivalent declarations</body></html>');return;
  }
  if(pathname==='/roles'){
    res.end(`<html><body><a href="/roles/navigation-hop">navigation</a><img src="/roles/resource-hop"><img src="${assetOrigin}/roles/allowed.png"></body></html>`);return;
  }
  if(pathname==='/roles/navigation-hop'){res.statusCode=302;res.setHeader('location',assetOrigin+'/roles/navigation-final');res.end('owned nav redirect');return;}
  if(pathname==='/roles/resource-hop'){res.statusCode=302;res.setHeader('location',assetOrigin+'/roles/resource-middle');res.end('owned resource redirect');return;}
  if(pathname==='/coalesced'){res.end('<html><body><a href="/coalesced/hop">check only navigation</a><img src="/coalesced/hop"></body></html>');return;}
  if(pathname==='/coalesced/hop'){res.statusCode=302;res.setHeader('location',assetOrigin+'/coalesced/final');res.end('owned coalesced redirect');return;}
  res.end(`<html><title>${pathname}</title><body>Owned leaf without outward references</body></html>`);
});
const asset=http.createServer((req,res)=>{
  response('asset',req,res);
  if(req.url==='/roles/resource-middle'){res.statusCode=302;res.setHeader('location',thirdOrigin+'/roles/forbidden-final');res.end('owned second hop');return;}
  if(req.url==='/roles/navigation-final'){res.end(`<html><body><a href="${siteOrigin}/roles/forbidden-backlink">must not expand</a></body></html>`);return;}
  if(req.url==='/coalesced/final'){res.end(`<html><body><a href="${siteOrigin}/coalesced/forbidden-child">check-only role must not expand</a></body></html>`);return;}
  if(req.url.endsWith('.png')){res.setHeader('content-type','image/png');res.end(Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Y9Zl1QAAAAASUVORK5CYII=','base64'));return;}
  res.end('<html><body>Owned external leaf</body></html>');
});
const third=http.createServer((req,res)=>{response('third',req,res);res.end('forbidden controlled endpoint');});
const delay=ms=>new Promise(r=>setTimeout(r,ms));
const count=(kind,url)=>hits.filter(h=>h.kind===kind&&h.url===url).length;
const follow=(internal=false,external=false)=>({engine:ENGINE,follow_internal_qualified_links:internal,follow_external_qualified_links:external});
const config=(seed,resource={})=>({seed_urls:[siteOrigin+seed],obey_robots:false,discover_sitemaps:false,delay_ms:0,concurrency:1,check_external:true,resource_discovery:{navigation:{external_anchors:{store:true,fetch:true,expand:false}},...resource}});
function assertRunning(){if(server?.exitCode!==null&&server?.exitCode!==undefined)throw Error('Owned native server exited: '+stderr);}
async function until(fn){for(let i=0;i<1200;i++){assertRunning();const value=await fn();assertRunning();if(value)return value;await delay(25);}throw Error('Owned resource-contract acceptance timeout');}
async function request(route,body){const res=await fetch(api+route,body===undefined?{}:{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});return {res,value:await res.json()};}
async function json(route,body){const {res,value}=await request(route,body);assert.equal(res.ok,true,route+' '+JSON.stringify(value));return value;}
async function completed(id){const status=await until(async()=>{const value=await json('/api/crawls/'+id);if(value.status==='failed')throw Error(value.error);return !['running','pausing','cancelling'].includes(value.status)&&value;});return {id,status,project:await json(`/api/crawls/${id}/project.json`)};}
async function start(seed,resource={},overrides={}){const created=await json('/api/crawls',{...config(seed,resource),...overrides});const job=await completed(created.id);jobs.push(job);return job;}
async function ledger(id){const value=await json(`/api/crawls/${id}/resources?limit=1000`);assert.equal(value.ledger_available,true);assert.equal(value.has_more,false);return value;}
async function cli(args){return new Promise(resolve=>{const child=spawn(binary,args,{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});let stdout='',stderr='';child.stdout.on('data',c=>stdout+=c);child.stderr.on('data',c=>stderr+=c);child.on('error',e=>resolve({code:-1,stderr:e.message,stdout}));child.on('close',code=>resolve({code,stderr,stdout}));});}
async function runServer(listen='127.0.0.1:0'){
  stderr='';let stdout='',spawnError;
  server=spawn(binary,['serve','--listen',listen,'--data-dir',data],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});
  server.on('error',error=>{spawnError=error;});server.stderr.on('data',c=>stderr+=c.toString());server.stdout.on('data',c=>stdout+=c.toString());
  // Only this child can publish the post-bind address; never probe a configured foreign listener.
  const address=await until(()=>{if(spawnError)throw spawnError;return stdout.match(/^Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?$/m)?.[1];});
  assert.ok(Number(new URL(address).port)>0);api=address;ownedInstances.push({pid:server.pid,address,requested_listen:listen});
}
async function stopServer(){if(server&&server.exitCode===null){const closed=new Promise(r=>server.once('close',r));server.kill();await closed;}}
async function ready(){await until(async()=>{try{return(await fetch(api+'/healthz')).ok;}catch{return false;}});}
try{
  for(const fixture of [site,asset,third])await new Promise((resolve,reject)=>{fixture.once('error',reject);fixture.listen(0,'127.0.0.1',resolve);});
  siteOrigin=`http://127.0.0.1:${site.address().port}`;assetOrigin=`http://127.0.0.1:${asset.address().port}`;thirdOrigin=`http://127.0.0.1:${third.address().port}`;
  await new Promise((resolve,reject)=>{foreign.once('error',reject);foreign.listen(0,'127.0.0.1',resolve);});
  await assert.rejects(runServer('127.0.0.1:'+foreign.address().port),/Owned native server exited/);assert.equal(foreignRequests,0);await new Promise(resolve=>foreign.close(resolve));
  checks.push('a foreign listener with successful health response is rejected before every HTTP read or mutation; own dynamic address comes only from the child post-bind stdout');
  await runServer();await ready();
  const caps=await json('/api/capabilities');assert.ok(caps.native_resource_discovery.semantic_contracts.includes(ENGINE));
  const beforeInvalid=hits.length;
  for(const resource of [{contract:{...follow(),engine:'unsupported'}},{contract:follow(),follow_rel_nofollow:true},{contract:follow(),follow_document_nofollow:true},{contract:{...follow(),unknown:true}}]){const {res}=await request('/api/crawls',config('/invalid-owned',resource));assert.equal(res.status,400);}
  assert.equal(hits.length,beforeInvalid);checks.push('invalid contract engine, unknown fields and both contradictory legacy overrides fail before any target or robots HTTP');

  const legacy=await start('/rel-legacy');const old=await ledger(legacy.id);assert.equal(old.configured_semantic_contract,'open-seo-spider/resource-discovery-v1');assert.equal(old.snapshot.resource_contract_engine,undefined);
  for(const q of qualifiers)assert.equal(count('site',`/rel-legacy/${q}`),q==='nofollow'?0:1);
  assert.ok(old.observations.every(row=>row.decision.qualification===undefined&&row.decision.grant?.reference_kind===undefined));
  checks.push('absent contract preserves real V1 UGC/sponsored behavior, old serialization and unknown immutable grant role; historical compatibility is separate from V2 acceptance');
  const nullProfile=await start('/rel-null',{contract:null}),nullRows=await ledger(nullProfile.id);assert.equal(nullRows.configured_semantic_contract,'open-seo-spider/resource-discovery-v1');assert.equal(nullRows.snapshot.resource_contract_engine,undefined);assert.equal(nullProfile.project.config.resource_discovery.contract,undefined);assert.equal(nullRows.config_sha256,old.config_sha256);
  const defaultProfile=await start('/rel-default',{contract:{}}),defaultRows=await ledger(defaultProfile.id);assert.equal(defaultRows.snapshot.resource_contract_engine,ENGINE);assert.notEqual(defaultRows.config_sha256,old.config_sha256);for(const q of qualifiers){assert.equal(count('site','/rel-null/'+q),q==='nofollow'?0:1);assert.equal(count('site','/rel-default/'+q),0);assert.equal(count('asset','/rel-default/'+q),0);}assert.equal(count('site','/rel-default/plain'),1);
  checks.push('explicit null preserves V1 while explicit empty contract selects validated V2 defaults without suppressing the independent plain link');
  for(const internal of [false,true])for(const external of [false,true]){
    const seed=`/rel-v2-${Number(internal)}${Number(external)}`,job=await start(seed,{contract:follow(internal,external)}),saved=await ledger(job.id);
    assert.equal(saved.configured_semantic_contract,ENGINE);assert.equal(saved.semantic_contract_binding,'validated_full_config_and_generation');assert.equal(saved.snapshot.resource_contract_engine,ENGINE);
    for(const q of qualifiers){assert.equal(count('site',seed+'/'+q),Number(internal),seed+' internal '+q);assert.equal(count('asset',seed+'/'+q),Number(external),seed+' external '+q);}
    assert.equal(count('site',seed+'/plain'),1,'independent unqualified control');
    const qualified=saved.observations.filter(row=>qualifiers.some(q=>row.occurrence.raw_value.endsWith('/'+q)));
    assert.equal(qualified.length,6);for(const row of qualified){assert.equal(row.decision.qualification.engine,ENGINE);assert.equal(row.decision.qualification.document_nofollow,false);assert.equal(row.decision.qualification.follow_allowed,row.decision.qualification.scope==='internal'?internal:external);}
  }
  checks.push('all four independent internal/external overrides govern actual nofollow/ugc/sponsored requests while preserving every source and independently allowed target');
  for(const kind of ['meta','header'])for(const internal of [false,true])for(const external of [false,true]){
    const seed=`/doc-${kind}-${Number(internal)}${Number(external)}`,job=await start(seed,{contract:follow(internal,external)}),saved=await ledger(job.id);
    assert.equal(count('site',seed+'/internal'),Number(internal));assert.equal(count('asset',seed+'/external'),Number(external));
    const rows=saved.observations.filter(row=>row.occurrence.raw_value.endsWith('/internal')||row.occurrence.raw_value.endsWith('/external'));
    assert.equal(rows.length,2);assert.ok(rows.every(row=>row.decision.qualification?.document_nofollow===true));
  }
  checks.push('meta and X-Robots document nofollow share the V2 internal/external switches with actual allowed/denied target controls for every override combination');
  const headers=await start('/headers',{contract:follow(),navigation:{canonical:{store:true,fetch:true,expand:false}}});const headerRows=(await ledger(headers.id)).observations.filter(row=>row.occurrence.reference_kind==='canonical');assert.equal(headerRows.length,2);
  assert.equal(count('site','/headers/html'),0);assert.equal(count('site','/headers/http'),0);assert.ok(headerRows.every(row=>row.decision.qualification.rel_qualifiers.includes('nofollow')&&row.decision.qualification.follow_allowed===false));assert.deepEqual([...new Set(headerRows.map(row=>row.decision.qualification.source))].sort(),['html_rel','http_link_rel']);
  checks.push('equivalent HTML and HTTP Link canonical nofollow both retain source-specific qualification and suppress actual target HTTP');
  const roles=await start('/roles',{contract:follow(),resource_scope:{external_origins:[assetOrigin]}},{check_external:false});
  assert.equal(count('site','/roles/navigation-hop'),1);assert.equal(count('site','/roles/resource-hop'),1);assert.equal(count('asset','/roles/navigation-final'),0);assert.equal(count('site','/roles/forbidden-backlink'),0);assert.equal(count('asset','/roles/allowed.png'),1);assert.equal(count('asset','/roles/resource-middle'),1);assert.equal(count('third','/roles/forbidden-final'),0);
  const roleLedger=await ledger(roles.id);assert.ok(roleLedger.observations.some(row=>row.decision.grant?.reference_kind==='image'));assert.ok(roleLedger.observations.some(row=>row.decision.grant?.reference_kind==='internal_anchor'));
  const transports=await json(`/api/crawls/${roles.id}/resources/transports?limit=1000`);assert.equal(transports.snapshot.resource_contract_engine,ENGINE);
  const deniedNav=transports.transports.find(row=>row.requested_url===siteOrigin+'/roles/navigation-hop')?.projection?.resource_response?.redirect_decisions;assert.equal(deniedNav?.length,1);assert.equal(deniedNav[0].target_url,assetOrigin+'/roles/navigation-final');assert.equal(deniedNav[0].reason,'redirect_role_scope_denied');assert.equal(deniedNav[0].request_started,false);assert.equal(deniedNav[0].allowed,false);assert.deepEqual(deniedNav[0].original_reference_kinds,['internal_anchor']);
  checks.push('per-hop immutable resource/navigation role scope allows the positive asset but rejects navigation to asset-only origin and a resource redirect into an unlisted third origin');
  const coalesced=await start('/coalesced',{contract:follow(),navigation:{internal_anchors:{store:false,fetch:true,expand:false}}});assert.equal(count('site','/coalesced/hop'),1);assert.equal(count('asset','/coalesced/final'),1);assert.equal(count('site','/coalesced/forbidden-child'),0);assert.equal(coalesced.project.pages.length,1);
  const coalescedRows=await json(`/api/crawls/${coalesced.id}/resources/transports?limit=1000`),coalescedTransport=coalescedRows.transports.find(row=>row.requested_url===siteOrigin+'/coalesced/hop');assert.ok(coalescedTransport);assert.equal(coalescedTransport.retained_response,false);assert.equal(coalescedTransport.projection.resource_response.retained_response,false);assert.deepEqual(coalescedTransport.projection.resource_response.redirect_decisions[0].original_reference_kinds.slice().sort(),['image','internal_anchor']);assert.equal(coalescedTransport.projection.resource_response.redirect_decisions[0].authorizing_occurrence_ids.length,1);
  checks.push('a coalesced store=true image cannot borrow store=false navigation redirect authority to retain or expand the unlisted final response; one actual transport remains bodyless');

  const readsBefore=hits.length,exported=await fetch(api+`/api/crawls/${roles.id}/resources.json`);assert.equal(exported.status,200);assert.equal(exported.headers.get('x-resource-contract-engine'),ENGINE);const full=await exported.json();assert.equal(full.snapshot.resource_contract_engine,ENGINE);assert.equal(full.observations.length,roleLedger.observations.length);
  const csvExport=await fetch(api+`/api/crawls/${roles.id}/resources.csv`);assert.equal(csvExport.status,200);assert.equal(csvExport.headers.get('x-resource-contract-engine'),ENGINE);assert.ok((await csvExport.text()).includes('complete_observation_json'));
  const legacyExport=await fetch(api+`/api/crawls/${legacy.id}/resources.json`);assert.equal(legacyExport.status,200);assert.equal(legacyExport.headers.get('x-resource-contract-engine'),null);assert.equal((await legacyExport.json()).snapshot.resource_contract_engine,undefined);
  const output=path.join(data,'contract-full.json'),command=await cli(['resource-ledger',path.join(data,`${roles.id}.sqlite`),'--json',output]);assert.equal(command.code,0,command.stderr);assert.deepEqual(JSON.parse(await readFile(output,'utf8')),full);
  for(const kind of ['stdio','streamable_http']){
    const client=new Client({name:'resource-contract-acceptance',version:'0.1.0'});
    try{await client.connect(kind==='stdio'?new StdioClientTransport({command:binary,args:['mcp','--server',api],cwd:root,stderr:'pipe'}):new StreamableHTTPClientTransport(new URL(api+'/mcp')));assert.equal((await client.listTools()).tools.length,22);const result=await client.callTool({name:'spider_resources',arguments:{id:roles.id,limit:1000}});assert.equal(result.isError,false,JSON.stringify(result));assert.equal(result.structuredContent.snapshot.resource_contract_engine,ENGINE);assert.deepEqual(result.structuredContent.observations,roleLedger.observations);}finally{await client.close();}
  }
  assert.equal(hits.length,readsBefore);checks.push('full native API/CLI snapshot equality and official22-tool SDK over stdio/StreamableHTTP preserve V2 provenance without website requests');
  const dbPath=path.join(data,`${roles.id}.sqlite`);assert.equal(path.dirname(path.resolve(dbPath)),path.resolve(data));
  const database=new DatabaseSync(dbPath,{timeout:5000});
  async function mutated({name,select,update,key,change}){
    const reader=database.prepare(select),writer=database.prepare(update),original=reader.get(key)?.value;assert.equal(typeof original,'string',name+' original exists');
    const altered=JSON.stringify(change(JSON.parse(original)));assert.notEqual(altered,original);const observed=[];
    try{
      assert.equal(Number(writer.run(altered,key).changes),1);
      for(const route of ['/resources','/resources/transports','/resources.json','/resources.csv']){const response=await fetch(api+`/api/crawls/${roles.id}`+route),body=await response.text();assert.ok([409,500].includes(response.status),name+' '+route+' '+body);assert.ok(body.length>0);observed.push({route,status:response.status});}
    }finally{assert.equal(Number(writer.run(original,key).changes),1);assert.equal(reader.get(key)?.value,original);}
    assert.deepEqual((await ledger(roles.id)).observations,roleLedger.observations);assert.equal(hits.length,readsBefore);tamperControls.push({name,rejected:observed,exact_cell_restored:true,positive_read_after_restore:true,website_requests:0});
  }
  try{
    const grantId=database.prepare('SELECT occurrence_id FROM resource_grants ORDER BY occurrence_id LIMIT 1').get()?.occurrence_id;assert.equal(typeof grantId,'string');
    await mutated({name:'immutable_original_reference_role_changed',select:'SELECT data AS value FROM resource_grants WHERE occurrence_id=?',update:'UPDATE resource_grants SET data=? WHERE occurrence_id=?',key:grantId,change:grant=>({...grant,reference_kind:grant.reference_kind==='image'?'internal_anchor':'image'})});
    const redirect=database.prepare('SELECT url,data FROM resource_transports').all().find(row=>JSON.parse(row.data).projection?.resource_response?.redirect_decisions?.length);assert.ok(redirect);
    await mutated({name:'recorded_redirect_role_decision_changed_without_payload_hash',select:'SELECT data AS value FROM resource_transports WHERE url=?',update:'UPDATE resource_transports SET data=? WHERE url=?',key:redirect.url,change:row=>{row.projection.resource_response.redirect_decisions[0].scope_allowed=!row.projection.resource_response.redirect_decisions[0].scope_allowed;return row;}});
    await mutated({name:'declared_profile_changed_without_matching_native_generation',select:'SELECT value FROM metadata WHERE key=?',update:'UPDATE metadata SET value=? WHERE key=?',key:'config',change:cfg=>{cfg.resource_discovery.contract=null;return cfg;}});
  }finally{database.close();}
  checks.push('three owned SQLite profile/immutable-role/redirect-decision tamper controls reject every paged and streamed read, restore exact cells and make zero website requests');
  const parentFile=path.join(data,roles.id+'.json'),parentBytes=await readFile(parentFile),selectedStart=hits.length;
  const selectedCreated=await json(`/api/crawls/${roles.id}/recrawl`,{urls:[siteOrigin+'/roles']}),selected=await completed(selectedCreated.id);jobs.push(selected);
  assert.deepEqual(hits.slice(selectedStart).map(row=>({kind:row.kind,url:row.url})),[{kind:'site',url:'/roles'}]);assert.deepEqual(await readFile(parentFile),parentBytes);
  const selectedRows=await ledger(selected.id);assert.equal(selectedRows.snapshot.resource_contract_engine,ENGINE);assert.equal(selectedRows.semantic_contract_binding,'validated_full_config_and_generation');assert.equal(selectedRows.checkpoint.inherited_history.rows_available,false);assert.equal(selectedRows.checkpoint.complete,false);assert.equal(selectedRows.observations.length,3);assert.ok(selectedRows.observations.every(row=>row.decision.grant===null));
  checks.push('selected V2 source recrawl preserves the actual profile and exact parent bytes, records fresh references without outward HTTP or fabricated complete inherited ledger');
  const restartReadRequests=hits.length;
  const hashes=[];for(const job of jobs){const file=path.join(data,job.id+'.json');hashes.push({file,sha256:createHash('sha256').update(await readFile(file)).digest('hex')});}
  await stopServer();await runServer();await ready();assert.equal((await json('/api/crawls')).crawls.length,jobs.length);
  for(const item of hashes)assert.equal(createHash('sha256').update(await readFile(item.file)).digest('hex'),item.sha256);
  assert.deepEqual((await ledger(roles.id)).observations,roleLedger.observations);assert.deepEqual((await ledger(legacy.id)).observations,old.observations);assert.deepEqual((await ledger(selected.id)).observations,selectedRows.observations);assert.equal(hits.length,restartReadRequests);
  checks.push('cold native restart preserves completed V1 and V2 report bytes, immutable ledger rows and source bindings without target requests');
  const evidence={tested_at:new Date().toISOString(),result:'PASS',binary_sha256:hash(await readFile(binary)),engine:ENGINE,groups:checks.length,checks,tamper_controls:tamperControls,owned_jobs:jobs.length,owned_instances:ownedInstances,foreign_listener_requests:foreignRequests,request_log:hits,read_website_requests:0,source_file_hashes:{},scope:'Actual isolated localhost API/CLI/SDK, selected source recrawl, integrity and restart checks; full reference/type/redirect matrix remains separately tracked.'};
  for(const file of ['scripts/resource-contract-e2e.mjs','crates/app/src/server.rs','crates/app/src/server/resources.rs','crates/core/src/crawler.rs','crates/core/src/resource_policy.rs','crates/core/src/resource_html.rs','crates/core/src/resource_capture.rs','crates/core/src/resource_ledger.rs','crates/core/src/store.rs'])evidence.source_file_hashes[file]=hash(await readFile(path.join(root,file)));
  await mkdir(path.dirname(proof),{recursive:true});await writeFile(proof,JSON.stringify(evidence,null,2)+'\n');passed=true;console.log(JSON.stringify({result:evidence.result,groups:checks.length,jobs:jobs.length,binary_sha256:evidence.binary_sha256}));
}catch(error){
  await mkdir(path.dirname(proof),{recursive:true});await writeFile(proof,JSON.stringify({tested_at:new Date().toISOString(),result:'FAIL',engine:ENGINE,completed_groups:checks.length,checks,tamper_controls:tamperControls,error:String(error.stack||error),request_log:hits,native_stderr:stderr,owned_fixture:process.env.SPIDER_KEEP_FAILED_FIXTURE==='1'?data:null},null,2)+'\n');throw error;
}finally{
  await stopServer();for(const fixture of [site,asset,third,foreign]){fixture.closeAllConnections();if(fixture.listening)await new Promise(r=>fixture.close(r));}
  assert.equal(path.dirname(path.resolve(data)),path.resolve(os.tmpdir()));assert.ok(path.basename(data).startsWith('open-seo-spider-contract-'));
  if(!passed&&process.env.SPIDER_KEEP_FAILED_FIXTURE==='1')console.error('Retained owned failed fixture: '+data);else await rm(data,{recursive:true,force:true});
}
