// Authored-only B12 native durable sitemap V5 protocol URL preservation for Main ae77438. Execute only after Root review.
// Built-in Node modules; every process, listener, API endpoint and data directory is run-owned.
import assert from 'node:assert/strict';
import http from 'node:http';
import path from 'node:path';
import os from 'node:os';
import {createReadStream} from 'node:fs';
import {access,mkdir,mkdtemp,readFile,realpath,readdir,stat,lstat,writeFile} from 'node:fs/promises';
import {performance} from 'node:perf_hooks';
import {StringDecoder} from 'node:string_decoder';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {gzipSync} from 'node:zlib';
import {DatabaseSync} from 'node:sqlite';
import {fileURLToPath} from 'node:url';

const FORBIDDEN_PORTS=new Set([8787,8790]);
const SHA=/^[0-9a-f]{64}$/,COMMIT=/^[0-9a-f]{40}$/,UUID=/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const LIMITS=Object.freeze({runMs:180000,startMs:20000,scenarioMs:30000,requestMs:5000,
 stopMs:5000,drainMs:8000,quietMs:250,fixtureCloseMs:5000,apiBytes:8*1024*1024,
 apiCount:1024,apiCompanionBytes:64*1024*1024,appLogBytes:256*1024,fixtureRequests:4096});
const SCENARIOS=Object.freeze([
 'ordinary-original-nested','policy-original-alias','robots-complete-originals',
 'robots-206-incomplete','file-gzip-corruption','known-empty',
 'pause-source-restart-resume','selected-ordinary-inheritance',
 'selected-policy-inheritance','legacy-none-import-selected-restart',
 'invalid-parent-catalog-rejected'
]);
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const stripCatalogSize=summary=>{const copy=structuredClone(summary);delete copy.catalog_json_bytes;return copy;};
const rows=(catalog,kind)=>catalog.rows.filter(item=>item.kind===kind).map(item=>item.row);
const count=(hits,group,route)=>hits.filter(hit=>hit.group===group && hit.path===route).length;
function parseBind(line){
 const match=/^Open SEO Spider: (http:\/\/127\.0\.0\.1:([1-9][0-9]*))\r?$/.exec(line);
 assert.ok(match,'Only a complete owned-child post-bind line supplies an API origin');
 const port=Number(match[2]);assert.ok(port<=65535 && !FORBIDDEN_PORTS.has(port));
 return match[1];
}
async function hashFile(file){const hash=createHash('sha256');for await(const chunk of createReadStream(file))hash.update(chunk);return hash.digest('hex');}
async function bounded(promise,ms,label){let timer;try{return await Promise.race([promise,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Timeout: '+label)),ms);})]);}finally{clearTimeout(timer);}}
async function absent(file){try{await access(file);}catch(error){if(error.code==='ENOENT')return;throw error;}throw Error('Refusing existing proof companion: '+file);}
function sameJson(actual,expected,label){assert.deepEqual(actual,expected,label);}
function diagnosticValue(value){
 if(value===undefined)return {type:'undefined'};
 if(value===null || ['number','boolean'].includes(typeof value))return value;
 if(typeof value==='string')return value.length<=4096?value:
  {type:'string',prefix:value.slice(0,4096),utf8_bytes:Buffer.byteLength(value),truncated:true};
 if(Array.isArray(value))return {type:'array',length:value.length,detail:'See source assertion and raw API/SQL companions'};
 return {type:typeof value,keys:Object.keys(value).slice(0,32),detail:'See source assertion and raw API/SQL companions'};
}

export async function main(){
 assert.ok(process.env.SPIDER_BINARY,'Explicit SPIDER_BINARY is required');
 assert.match(process.env.SPIDER_BINARY_SHA256 || '',SHA,'Expected binary SHA is required');
 assert.match(process.env.SPIDER_SOURCE_COMMIT || '',COMMIT,'Full declared source commit is required');
 assert.match(process.env.SPIDER_EXPECTED_SOURCE_COMMIT || '',COMMIT,'Full expected source commit is required');
 assert.equal(process.env.SPIDER_SOURCE_COMMIT,process.env.SPIDER_EXPECTED_SOURCE_COMMIT);
 const binary=await realpath(path.resolve(process.env.SPIDER_BINARY)),binaryInfo=await stat(binary);
 assert.ok(binaryInfo.isFile());const binarySha=await hashFile(binary);
 assert.equal(binarySha,process.env.SPIDER_BINARY_SHA256,'Executable differs before any launch');
 const harnessPath=fileURLToPath(import.meta.url),harnessSha=await hashFile(harnessPath);
 const runDir=await mkdtemp(path.join(process.env.RUNNER_TEMP || os.tmpdir(),'sitemap-ledger-e2e-'));
 const ownedRun=await realpath(runDir),data=path.join(ownedRun,'data');await mkdir(data);
 const ownedData=await realpath(data);assert.equal(path.dirname(ownedData),ownedRun);
 const proofPath=path.resolve(process.env.SPIDER_SITEMAP_LEDGER_PROOF || path.join(ownedRun,'proof.json'));
 const companionPaths={requests:proofPath+'.requests.json',api:proofPath+'.api.json',
  log:proofPath+'.app-log.json',sql:proofPath+'.sqlite-read-witness.json',fixtures:proofPath+'.fixtures.json'};
 await mkdir(path.dirname(proofPath),{recursive:true});
 for(const file of [proofPath,...Object.values(companionPaths)])await absent(file);
 const started=performance.now(),deadline=started+LIMITS.runMs,abort=new AbortController(),signalHandlers=[];
 const stamp=()=>performance.now()-started;
 const hits=[],active=new Set(),sockets=new Set(),routes=new Map(),recipes=[],apiRows=[],apps=[],checks=[],failures=[],sqlWitnesses=[];
 let apiCompanionBytes=0,totalRequests=0,acceptedConnections=0,lastActivity=performance.now(),fixtureOrigin;
 let witnessOverflow=false,fixtureClosed=false,witnessDrained=false,drainFacts=null,currentApp=null,phase='setup';
 const intervals=[],expectedHits=new Map(),reports=new Map(),jobReports=new Map(),parentBytes=new Map(),held=new Map();
 const fixtureErrors=[];let diagnosticScenario=null,diagnosticStage='setup';
 const fail=(where,error)=>{
  const stack=String(error?.stack || '').slice(0,16384);
  failures.push({phase:where,scenario_id:diagnosticScenario,scenario_phase:phase,stage:diagnosticStage,
   message:String(error?.message || error).slice(0,4000),name:String(error?.name || 'Error'),
   code:error?.code ?? null,operator:error?.operator ?? null,actual:diagnosticValue(error?.actual),
   expected:diagnosticValue(error?.expected),stack,
   failure_location:stack.split('\n').find(line=>line.includes(path.basename(harnessPath)+':'))?.trim() ?? null,
   owned_app_ordinal:currentApp?.ordinal ?? null,last_api_sequence:apiRows.at(-1)?.sequence ?? null,
   last_api_endpoint:apiRows.at(-1)?.endpoint ?? null,last_api_status:apiRows.at(-1)?.response_status ?? null});
 };
 for(const name of ['SIGINT','SIGTERM']){const handler=()=>abort.abort(Error('Interrupted: '+name));process.once(name,handler);signalHandlers.push([name,handler]);}
 function alive(){
  if(abort.signal.aborted)throw abort.signal.reason;if(performance.now()>=deadline)throw Error('Overall deadline exceeded');
  if(fixtureErrors.length)throw Error(fixtureErrors.join('; '));
  if(currentApp && (currentApp.spawnError || currentApp.bindError))throw currentApp.spawnError || currentApp.bindError;
  if(currentApp && !currentApp.plannedStop && currentApp.closed)throw Error('Owned App exited unexpectedly');
 }
 async function until(fn,ms,label){const end=Math.min(deadline,performance.now()+ms);while(performance.now()<end){alive();const value=await fn();if(value)return value;await delay(25);}throw Error('Timeout: '+label);}
 function expectHit(group,route,n){const key=group+'\0'+route;assert.ok(!expectedHits.has(key));expectedHits.set(key,{group,route,expected:n});}
 function route(group,url,body,headers={'content-type':'text/html'},status=200,hold=false){
  assert.ok(url.startsWith('/') && !url.includes('\n'));
  const recipe=Object.freeze({recipe_id:recipes.length+1,path:url,group,body:Buffer.from(body),
   headers:Object.freeze({...headers}),status,hold,registered_ms:stamp()});
  recipes.push(recipe);routes.set(url,recipe);
 }
 const site=http.createServer((request,response)=>{
  totalRequests++;lastActivity=performance.now();
  if(hits.length>=LIMITS.fixtureRequests){witnessOverflow=true;request.resume();response.writeHead(503);response.end('Witness budget exceeded');return;}
  const configured=routes.get(request.url);
  const hit={ordinal:totalRequests,group:configured?.group ?? null,path:request.url,method:request.method,
   started_ms:stamp(),finished_ms:null,state:'active',registered_route:Boolean(configured),phase_at_start:phase,
   recipe_id:configured?.recipe_id ?? null,response_status:configured?.status ?? 404,
   response_body_sha256:configured?sha(configured.body):null};
  hits.push(hit);active.add(hit);request.resume();
  const settle=state=>{if(!active.delete(hit))return;hit.state=state;hit.finished_ms=stamp();lastActivity=performance.now();};
  response.once('finish',()=>settle('finished'));response.once('close',()=>settle('aborted'));
  request.on('error',error=>{hit.request_error=error.code || error.message;});
  if(!configured){response.writeHead(404);response.end('Unregistered route');return;}
  const send=()=>{response.writeHead(configured.status,configured.headers);response.end(configured.body);};
  if(configured.hold){
   if(held.has(request.url)){fixtureErrors.push('Duplicate held source request');
    hit.response_status=409;hit.response_body_sha256=sha(Buffer.from('Duplicate held source request'));
    response.writeHead(409);response.end('Duplicate held source request');return;}
   held.set(request.url,{send,response});
  }
  else send();
 });
 site.on('error',error=>fixtureErrors.push(error.message));
 site.on('connection',socket=>{acceptedConnections++;sockets.add(socket);lastActivity=performance.now();socket.on('error',()=>{});
  socket.once('close',()=>{sockets.delete(socket);lastActivity=performance.now();});});
 const html=title=>'<html><head><title>'+title+'</title></head><body>Owned leaf</body></html>';
 const NS='http://www.sitemaps.org/schemas/sitemap/0.9';
 const urlset=entries=>'<urlset xmlns="'+NS+'">'+entries+'</urlset>';
 const index=entries=>'<sitemapindex xmlns="'+NS+'">'+entries+'</sitemapindex>';
 const loc=(kind,value)=>'<'+kind+'><loc>'+value+'</loc></'+kind+'>';
 async function call(endpoint,body,expectedStatus=200){
  alive();assert.ok(currentApp?.api && currentApp.pid && !currentApp.closed);
  assert.ok(endpoint==='/healthz' || endpoint==='/api/crawls' || endpoint==='/api/crawls/import' ||
   endpoint==='/api/lists/import' || /^\/api\/crawls\/[a-f0-9-]+(?:\/(?:project\.json|parent\.json|recrawl|pause|resume))?$/i.test(endpoint),'Unexpected API surface');
  assert.ok(apiRows.length<LIMITS.apiCount,'API witness row limit exceeded');
  const requestText=body===undefined?null:JSON.stringify(body),record={sequence:apiRows.length+1,phase,
   app_ordinal:currentApp.ordinal,origin:currentApp.api,endpoint,method:body===undefined?'GET':'POST',
   request_utf8:requestText,started_ms:stamp(),finished_ms:null,response_status:null,response_utf8:null};
  apiRows.push(record);
  const signal=AbortSignal.any([abort.signal,AbortSignal.timeout(Math.max(1,Math.min(LIMITS.requestMs,deadline-performance.now())))]);
  const response=await fetch(currentApp.api+endpoint,{redirect:'error',signal,...(body===undefined?{}:
   {method:'POST',headers:{'content-type':'application/json'},body:requestText})});
  const chunks=[];let bytes=0;
  if(response.body){const reader=response.body.getReader();try{while(true){const next=await reader.read();if(next.done)break;
   bytes+=next.value.byteLength;if(bytes>LIMITS.apiBytes){await reader.cancel();throw Error('API body limit');}chunks.push(Buffer.from(next.value));}
   }finally{reader.releaseLock();}}
  const raw=Buffer.concat(chunks);record.response_status=response.status;record.response_utf8=raw.toString('utf8');record.finished_ms=stamp();
  apiCompanionBytes+=raw.length+Buffer.byteLength(requestText || '');
  assert.ok(apiCompanionBytes<=LIMITS.apiCompanionBytes,'API companion budget');
  assert.equal(response.status,expectedStatus,endpoint+': '+record.response_utf8.slice(0,1500));alive();
  return {value:JSON.parse(record.response_utf8),bytes:raw,record};
 }
 async function launch(label){
  assert.ok(currentApp===null || currentApp.closed,'Previous owned App must close before restart');
  assert.equal(await hashFile(binary),binarySha,'Binary changed before spawn');
  assert.equal(await realpath(data),ownedData,'Owned data directory changed before native launch');
  assert.equal(path.dirname(ownedData),ownedRun);
  const app={ordinal:apps.length+1,label,api:null,pid:null,plannedStop:false,closed:false,close_meta:null,
   stdout:'',stderr:'',stdout_raw:[],stderr_raw:[],stdout_bytes:0,stderr_bytes:0,
   stdout_decoder:new StringDecoder('utf8'),stderr_decoder:new StringDecoder('utf8'),
   stdout_truncated:false,stderr_truncated:false,pending_line:'',bind_line:null,
   bindError:null,spawnError:null,started_ms:stamp(),stop_requested_ms:null};
  const child=spawn(binary,['serve','--listen','127.0.0.1:0','--data-dir',ownedData],
   {windowsHide:true,cwd:process.cwd(),stdio:['ignore','pipe','pipe']});app.child=child;app.pid=child.pid ?? null;
  apps.push(app);currentApp=app;
  app.closePromise=new Promise(resolve=>child.once('close',(code,signal)=>{
   app.stdout+=app.stdout_decoder.end();app.stderr+=app.stderr_decoder.end();
   app.closed=true;app.close_meta={code,signal,closed_ms:stamp()};resolve();}));
  child.once('error',error=>{app.spawnError=error;});
  function retainLog(channel,chunk){
   const remaining=Math.max(0,LIMITS.appLogBytes-app[channel+'_bytes']);
   const retained=Buffer.from(chunk.subarray(0,remaining));app[channel+'_raw'].push(retained);
   app[channel+'_bytes']+=retained.length;
   if(retained.length!==chunk.length){app[channel+'_truncated']=true;app.bindError=Error('App '+channel+' raw log budget exceeded');}
   const text=app[channel+'_decoder'].write(retained);app[channel]+=text;return text;
  }
  child.stdout.on('data',chunk=>{
   const text=retainLog('stdout',chunk);app.pending_line+=text;let end;
   while((end=app.pending_line.indexOf('\n'))!==-1){const line=app.pending_line.slice(0,end);app.pending_line=app.pending_line.slice(end+1);
    if(line.startsWith('Open SEO Spider:'))try{assert.equal(app.api,null);const found=parseBind(line);
     assert.notEqual(found,fixtureOrigin);app.api=found;app.bind_line=line;}catch(error){app.bindError=error;}}
   if(app.pending_line.length>8192){app.bindError=Error('Incomplete bind line budget exceeded');app.pending_line='';}
  });
  child.stderr.on('data',chunk=>{retainLog('stderr',chunk);});
  await until(()=>app.api,LIMITS.startMs,'owned post-bind '+label);
  const health=await call('/healthz');assert.equal(health.value.ok,true);assert.equal(health.value.name,'Open SEO Spider');
  return app;
 }
 async function stop(){
  if(!currentApp)return;
  const app=currentApp;if(app.closed){await app.closePromise;assert.ok(app.plannedStop,'Unexpected App close');return;}
  app.plannedStop=true;app.stop_requested_ms=stamp();app.child.kill('SIGTERM');
  try{await bounded(app.closePromise,LIMITS.stopMs,'owned App close');}
  catch{if(!app.closed)app.child.kill('SIGKILL');await bounded(app.closePromise,LIMITS.stopMs,'owned App force-close');}
  assert.ok(app.closed);
 }
 async function terminal(id,state='completed'){
  const done=await until(async()=>{const result=await call('/api/crawls/'+id);
   if(result.value.status==='failed')throw Error(result.value.error || 'Native job failed');
   return result.value.status===state && result.value;},LIMITS.scenarioMs,'job '+id+' '+state);
  assert.ok(done.checkpoint && done.generation>0,'Real native terminal checkpoint required');
  return done;
 }
 async function capture(id,state='completed'){
  const status=await terminal(id,state),report=await call('/api/crawls/'+id+'/project.json');
  const entry={id,status,report:report.value,project_sha256:sha(report.bytes),scenario_id:diagnosticScenario};
  jobReports.set(id,entry);return entry;
 }
 function catalogue(report,empty=false){
  const catalog=report.sitemap_catalog;assert.ok(catalog && Array.isArray(catalog.rows),'Known durable history required');
  assert.equal(catalog.schema_version,1);assert.equal(catalog.summary.engine,'open-seo-spider/sitemap-membership-ledger-v1');
  assert.equal(catalog.summary.rows_available,true);assert.equal(catalog.summary.declared_source_cohort_complete,false);
  assert.equal(catalog.summary.legacy_history_unknown,false);assert.match(catalog.summary.rolling_sha256,SHA);
  assert.equal(catalog.rows.length,catalog.summary.last_sequence);assert.equal(new Set(catalog.rows.map(row=>row.row.sequence)).size,catalog.rows.length);
  catalog.rows.forEach((row,i)=>assert.equal(row.row.sequence,i+1));
  if(empty){assert.equal(catalog.rows.length,0);assert.equal(catalog.summary.revision,0);}
  return catalog;
 }
 function base(seed){return {seed_urls:[fixtureOrigin+seed],discover_sitemaps:false,crawl_links:false,obey_robots:false,
  concurrency:1,delay_ms:0,request_timeout_secs:3,max_pages:20,max_body_bytes:1024*1024};}
 async function create(config){const created=await call('/api/crawls',config);assert.match(created.value.id,UUID);return created.value.id;}
 function checked(id,details){assert.ok(SCENARIOS.includes(id));assert.ok(!checks.some(item=>item.id===id));checks.push({id,assertions:details,recorded_ms:stamp()});}
 async function rootsAndNested(group,policy){
  phase=group;diagnosticScenario=group;diagnosticStage='scenario-start';const seed='/'+group+'/seed',map='/'+group+'/map.xml',child='/'+group+'/child.xml',page='/'+group+'/page';
  route(group,seed,html('Seed '+group));
  // SitemapDocument never applies query/rewrite transforms. Original fragment/
  // ASCII-space declarations normalize/coalesce without pretending otherwise.
  const rootValues=['  '+fixtureOrigin+map+'#first  ',fixtureOrigin+map+'#second'];
  const importedValues=[fixtureOrigin+map+'#third',fixtureOrigin+map+'#fourth'];
  const importedXml=index(importedValues.map(value=>loc('sitemap',value)).join(''));
  const imported=await call('/api/lists/import',{content_base64:Buffer.from(importedXml).toString('base64'),options:{format:{kind:'xml'}}});
  assert.equal(imported.value.rows.length,2);
  const childValue=fixtureOrigin+child+'#declared-child';
  route(group,map,index(loc('sitemap',childValue)+loc('sitemap',childValue)+loc('sitemap',fixtureOrigin+map)+( '<sitemap/>')));
  const listed=fixtureOrigin+(policy?'/'+group+'/listed?utm=1':page);
  const second=policy?fixtureOrigin+'/'+group+'/listed?utm=2':fixtureOrigin+page;
  const extraRejected=policy?
   loc('url','\u00a0'+fixtureOrigin+'/'+group+'/nbsp\u00a0')+
   '<url><loc>'+fixtureOrigin+'/'+group+'/a&#x9;b</loc></url>':'';
  route(group,child,urlset(loc('url',listed)+loc('url',second)+'<url/><url><loc>relative/path</loc></url>'+extraRejected),{'content-type':'application/xml'});
  if(policy){route(group,'/'+group+'/nbsp',html('Forbidden NBSP alias'));route(group,'/'+group+'/ab',html('Forbidden tab alias'));}
  route(group,page,html('Page '+group));
  const config={...base(seed),sitemap_urls:rootValues,input_imports:[imported.value]};
  if(policy){
   config.crawl_policy={scope:{roots:[{origin:fixtureOrigin,folders:['/']}],external:'deny',outside_folder:'deny'},
    initial_inputs:{rewrite:'apply'},query:{remove_keys:['tracking','utm']},
    rewrites:[{id:'listed-alias',pattern:'/'+group+'/listed',replacement:{kind:'literal',text:page}}]};
   diagnosticStage='policy-fixture-explicit-initial-rewrite-before-crawl-admission';
   assert.equal(config.crawl_policy.initial_inputs?.rewrite,'apply',
    'Initial page-input opt-in does not make SitemapDocument source kinds transformable');
  }
  const id=await create(config),entry=await capture(id),catalog=catalogue(entry.report);
  sameJson(entry.report.config.explicit_sitemap_inputs,rootValues,'Original explicit raw vector');
  if(policy)assert.equal(entry.report.config.crawl_policy.initial_inputs.rewrite,'apply',
   'Saved normalized policy must retain explicit alias-fixture intent');
  diagnosticStage='original-root-bindings';
  const roots=rows(catalog,'root');assert.equal(roots.length,4);
  sameJson(roots.filter(row=>row.kind==='explicit_config').map(row=>row.raw_value),rootValues);
  sameJson(roots.filter(row=>row.kind==='explicit_config').map(row=>row.source_ordinal),[0,1]);
  sameJson(roots.filter(row=>row.kind==='imported_sitemap_row').map(row=>row.raw_value),importedValues);
  sameJson(roots.filter(row=>row.kind==='imported_sitemap_row').map(row=>row.source_ordinal),[0,1]);
  for(const root of roots){assert.equal(root.admission.membership_url,root.normalized_document_url);
   assert.equal(root.admission.admitted_fetch_url,fixtureOrigin+map);
   assert.equal(Boolean(root.admission.policy_occurrence_id),policy);}
  assert.equal(roots.filter(row=>row.admission.decision==='scheduled').length,1);
  assert.equal(roots.filter(row=>row.admission.decision==='coalesced').length,3);
  const docs=rows(catalog,'document');assert.equal(docs.length,2);
  assert.ok(docs.every(doc=>doc.transport.body_complete && doc.transport.final_status===200 && doc.parser?.complete));
  diagnosticStage='xml-loc-shape-and-rejections';
  const locations=rows(catalog,'location');assert.equal(locations.length,policy?10:8);
  const missing=locations.filter(row=>row.admission.decision==='missing_location');
  const invalid=locations.filter(row=>row.admission.decision==='invalid_location');
  assert.equal(missing.length,2);assert.equal(invalid.length,policy?3:1);
  assert.ok(missing.every(row=>row.observed.loc_ordinal===null && row.admission.policy_occurrence_id===null));
  assert.ok(invalid.every(row=>row.observed.loc_ordinal!==null && row.observed.validation_error &&
   row.admission.admitted_fetch_url===null && row.admission.coalesced_policy_occurrence_id===null));
  assert.ok(invalid.every(row=>Boolean(row.admission.policy_occurrence_id)===policy),
   'Every invalid present policy Loc must retain its rejected actual policy input');
  if(policy)assert.ok(invalid.some(row=>row.observed.decoded_value.includes('\u00a0')) &&
   invalid.some(row=>row.observed.decoded_value.includes('\t')));
  diagnosticStage='valid-page-loc-scheduling-and-coalescing';
  const childLocs=locations.filter(row=>row.observed.role==='child_sitemap' &&
   row.observed.decoded_value===childValue);
  assert.equal(childLocs.length,2);
  assert.ok(childLocs.every(row=>row.admission.membership_url===fixtureOrigin+child &&
   row.admission.admitted_fetch_url===fixtureOrigin+child && row.referenced_sitemap_task===fixtureOrigin+child));
  const pageLocs=locations.filter(row=>row.observed.role==='page_location' && row.admission.admitted_fetch_url);
  assert.equal(pageLocs.length,2);assert.equal(pageLocs[0].admission.decision,'scheduled');assert.equal(pageLocs[1].admission.decision,'coalesced');
  assert.ok(pageLocs.every(row=>row.admission.membership_url===row.observed.normalized_fetch_url));
  if(policy){assert.ok(pageLocs.every(row=>row.admission.membership_url!==row.admission.admitted_fetch_url));
   assert.ok(pageLocs.every(row=>row.admission.admitted_fetch_url===fixtureOrigin+page && row.admission.policy_occurrence_id));}
  assert.equal(entry.report.pages.filter(row=>row.url===fixtureOrigin+page).length,1);
  assert.equal(entry.status.checkpoint.sitemap_ledger.rolling_sha256,catalog.summary.rolling_sha256);
  for(const target of [seed,map,child,page])expectHit(group,target,target===page?2:1);
  if(policy){for(const target of ['/'+group+'/listed?utm=1','/'+group+'/listed?utm=2',
   map+'?tracking=1',child+'?tracking=8','/'+group+'/nbsp','/'+group+'/ab'])expectHit(group,target,0);}
  reports.set(group,{entry,catalog,seed,map,child,page});
  if(policy)await protocolQueryControl(group,config.crawl_policy);
  checked(group,['4 fragment/ASCII-space original roots and raw/ordinal import overlap',
   'two primary XML tasks and exact ordinary/policy Loc counts; child fragments normalize and duplicates coalesce',
   policy?'invalid present policy Locs retain rejected SQL inputs; NBSP/tab aliases stay unrequested':
    'ordinary missing/invalid original Locs remain without invented policy references',
   'page Loc membership remains original while policy page aliases schedule/coalesce',
   policy?'separate queried root and queried child preserve protocol queries even with Apply; bare aliases stay unrequested':
    'ordinary protocol identities normalize only their original safe declarations',
   'full report/checkpoint catalogue binding']);
 }
 async function protocolQueryControl(group,policyConfig){
  diagnosticScenario=group;diagnosticStage='sitemap-document-query-preserve-control';
  const seed='/'+group+'/query-control-seed',map='/'+group+'/query-map.xml',child='/'+group+'/query-child.xml';
  const queriedMap=map+'?tracking=protocolkeep',queriedChild=child+'?tracking=childkeep';
  route(group,seed,html('Initial page query rewrite positive'));
  route(group,queriedMap,index(loc('sitemap',fixtureOrigin+queriedChild+'#child')+
   loc('sitemap',fixtureOrigin+queriedChild+'#duplicate')),{'content-type':'application/xml'});
  route(group,queriedChild,urlset(''),{'content-type':'application/xml'});
  route(group,map,urlset(''),{'content-type':'application/xml'});
  route(group,child,urlset(''),{'content-type':'application/xml'});
  const originalRoot='  '+fixtureOrigin+queriedMap+'#root  ';
  const config={...base(seed),seed_urls:[fixtureOrigin+seed+'?tracking=initialpage'],
   sitemap_urls:[originalRoot],crawl_policy:structuredClone(policyConfig)};
  assert.equal(config.crawl_policy.initial_inputs.rewrite,'apply');
  const entry=await capture(await create(config)),catalog=catalogue(entry.report);
  const rootRows=rows(catalog,'root');assert.equal(rootRows.length,1);
  assert.equal(rootRows[0].raw_value,originalRoot);assert.equal(rootRows[0].admission.decision,'scheduled');
  assert.equal(rootRows[0].normalized_document_url,fixtureOrigin+queriedMap);
  assert.equal(rootRows[0].admission.membership_url,fixtureOrigin+queriedMap);
  assert.equal(rootRows[0].admission.admitted_fetch_url,fixtureOrigin+queriedMap);
  assert.ok(rootRows[0].admission.policy_occurrence_id);
  const documents=rows(catalog,'document');assert.equal(documents.length,2);
  sameJson(documents.map(row=>row.task_url),[fixtureOrigin+queriedMap,fixtureOrigin+queriedChild]);
  assert.ok(documents.every(row=>row.transport.request_started && row.transport.body_complete &&
   row.transport.final_status===200 && row.parser.complete && row.failure_code===null));
  const declarations=rows(catalog,'location');assert.equal(declarations.length,2);
  sameJson(declarations.map(row=>row.admission.decision),['scheduled','coalesced']);
  assert.ok(declarations.every(row=>row.observed.role==='child_sitemap' &&
   row.admission.membership_url===fixtureOrigin+queriedChild &&
   row.admission.admitted_fetch_url===fixtureOrigin+queriedChild &&
   row.referenced_sitemap_task===fixtureOrigin+queriedChild && row.admission.policy_occurrence_id));
  assert.equal(entry.report.pages.length,1);
  assert.equal(entry.report.pages[0].url,fixtureOrigin+seed,
   'Initial page source transforms while SitemapDocument sources preserve their query URLs');
  for(const target of [seed,queriedMap,queriedChild])expectHit(group,target,1);
  for(const target of [seed+'?tracking=initialpage',map,child])expectHit(group,target,0);
  assert.equal(entry.status.checkpoint.sitemap_ledger.rolling_sha256,catalog.summary.rolling_sha256);
 }
 async function robots(group,incomplete){
  phase=group;diagnosticScenario=group;diagnosticStage='scenario-start';const seed='/'+group+'/seed',map='/'+group+'/map.xml',page='/'+group+'/page';
  route(group,seed,html(group));route(group,map,urlset(loc('url',fixtureOrigin+page)),{'content-type':'application/xml'});route(group,page,html('Robots page'));
  const raw='User-agent: *\r\nAllow: /\r\nSitemap:  '+fixtureOrigin+map+'#first \r\nSitemap: /relative.xml\r\nSitemap: '+fixtureOrigin+map+'#second\n';
  route(group,'/robots.txt',raw,{'content-type':'text/plain'},incomplete?206:200);
  const id=await create({...base(seed),crawl_links:true,obey_robots:true,discover_sitemaps:true});
  const entry=await capture(id),catalog=catalogue(entry.report),roots=rows(catalog,'root'),docs=rows(catalog,'document');
  assert.equal(roots.length,3);sameJson(roots.map(row=>row.source_ordinal),[0,1,2]);
  assert.ok(roots.every(row=>row.raw_line && row.robots_evidence && row.kind==='robots_directive'));
  assert.equal(roots[0].raw_value,'  '+fixtureOrigin+map+'#first ');assert.equal(roots[1].normalized_document_url,null);
  const observed=docs.find(row=>row.kind==='robots_directives');assert.ok(observed);assert.equal(observed.transport.final_status,incomplete?206:200);
  assert.equal(observed.transport.body_complete,true);
  if(incomplete){assert.equal(docs.length,1);assert.ok(roots.every(row=>row.admission.decision==='incomplete_source'));
   assert.ok(observed.failure_code);assert.equal(catalog.summary.incomplete_documents,1);}
  else {assert.equal(docs.length,2);sameJson(roots.map(row=>row.admission.decision),['scheduled','invalid_location','coalesced']);}
  expectHit(group,'/robots.txt',1);expectHit(group,seed,1);expectHit(group,map,incomplete?0:1);expectHit(group,page,incomplete?0:1);expectHit(group,'/relative.xml',0);
  checked(group,['3 original zero-based raw directives','true entity EOF distinct from HTTP source completeness',incomplete?'206 makes zero admitted sitemap/page tasks':'absolute duplicate roots coalesce and relative directive stays invalid']);
 }
 async function corruptedGzip(){
  const group='file-gzip-corruption';phase=group;diagnosticScenario=group;diagnosticStage='scenario-start';const seed='/'+group+'/seed',map='/'+group+'/map.xml',page='/'+group+'/page';
  const body=gzipSync(Buffer.from(urlset(loc('url',fixtureOrigin+page))));body[body.length-8]^=1;
  route(group,seed,html(group));route(group,map,body,{'content-type':'application/octet-stream'});route(group,page,html('Never admitted'));
  const entry=await capture(await create({...base(seed),sitemap_urls:[fixtureOrigin+map]})),catalog=catalogue(entry.report),docs=rows(catalog,'document');
  assert.equal(docs.length,1);assert.equal(docs[0].transport.body_complete,true);assert.equal(docs[0].transport.final_status,200);
  assert.equal(docs[0].entity_decode,null);assert.ok(docs[0].failure_code);assert.equal(catalog.summary.incomplete_documents,1);
  assert.equal(rows(catalog,'location').length,0);expectHit(group,seed,1);expectHit(group,map,1);expectHit(group,page,0);
  checked(group,['complete actual HTTP entity','corrupt file-gzip footer retained as source failure','no fabricated XML parser or page admission']);
 }
 async function knownEmpty(){
  const group='known-empty';phase=group;diagnosticScenario=group;diagnosticStage='scenario-start';const seed='/'+group+'/seed';route(group,seed,html(group));
  const entry=await capture(await create(base(seed)));catalogue(entry.report,true);
  sameJson(entry.report.config.explicit_sitemap_inputs,[]);expectHit(group,seed,2);
  reports.set(group,{entry,seed});checked(group,['known original empty vector','available empty catalogue','cohort completeness remains false']);
 }
 async function pauseRestart(){
  const group='pause-source-restart-resume';phase=group;diagnosticScenario=group;diagnosticStage='scenario-start';const map='/'+group+'/index.xml',children=['a','b'].map(letter=>'/'+group+'/'+letter+'.xml');
  const pages=['a','b'].map(letter=>'/'+group+'/'+letter);
  route(group,map,index(children.map(value=>loc('sitemap',fixtureOrigin+value)).join('')),{'content-type':'application/xml'},200,true);
  children.forEach((value,i)=>route(group,value,urlset(loc('url',fixtureOrigin+pages[i])),{'content-type':'application/xml'}));
  pages.forEach(value=>route(group,value,html(value)));
  const id=await create({...base('/unused-pause-seed'),seed_urls:[],sitemap_urls:[fixtureOrigin+map]});
  await until(()=>held.has(map),LIMITS.scenarioMs,'held original index');
  await call('/api/crawls/'+id+'/pause',{});const heldSource=held.get(map);held.delete(map);heldSource.send();
  const paused=await capture(id,'paused'),catalog=catalogue(paused.report);
  assert.equal(rows(catalog,'document').length,1);assert.equal(paused.report.pages.length,0);
  assert.equal(paused.status.checkpoint.discovery_pending,2);assert.equal(paused.status.checkpoint.discovery_complete,false);
  const before=await readFile(path.join(ownedData,id+'.json'));
  await stop();const interval={id:'paused-startup',started_ms:stamp(),finished_ms:null,
   after_request_ordinal:totalRequests,through_request_ordinal:null};intervals.push(interval);
  phase='restart-paused-read-only';await launch('paused-restart');
  const recovered=await call('/api/crawls/'+id);assert.equal(recovered.value.status,'paused');
  const old=await call('/api/crawls/'+id+'/project.json');sameJson(old.value.sitemap_catalog,catalog);
  assert.ok(before.equals(await readFile(path.join(ownedData,id+'.json'))));
  interval.finished_ms=stamp();interval.through_request_ordinal=totalRequests;phase=group;diagnosticScenario=group;diagnosticStage='scenario-start';
  await call('/api/crawls/'+id+'/resume',{concurrency:1,delay_ms:0});const completed=await capture(id),final=catalogue(completed.report);
  assert.equal(completed.status.generation,2);assert.equal(completed.status.checkpoint.revision,2);
  sameJson(rows(final,'root'),rows(catalog,'root'));assert.equal(final.summary.config_sha256,catalog.summary.config_sha256);
  assert.equal(rows(final,'document').length,3);assert.equal(completed.report.pages.length,2);
  assert.equal(rows(final,'document').find(row=>row.task_url===fixtureOrigin+map).generation,1);
  assert.ok(rows(final,'document').filter(row=>row.task_url!==fixtureOrigin+map).every(row=>row.generation===2));
  [map,...children,...pages].forEach(value=>expectHit(group,value,1));
  checked(group,['pause drains admitted source before checkpoint','restart preserves exact report bytes and does not issue fixture requests','resume preserves initial roots/config hash and fetches only saved pending children']);
 }
 async function selected(group,parentGroup){
  phase=group;diagnosticScenario=group;diagnosticStage='scenario-start';const parent=reports.get(parentGroup),target=fixtureOrigin+parent.page;
  const exact=await readFile(path.join(ownedData,parent.entry.id+'.json'));parentBytes.set(parent.entry.id,sha(exact));
  const created=await call('/api/crawls/'+parent.entry.id+'/recrawl',{urls:[target,target]});assert.match(created.value.id,UUID);
  assert.notEqual(created.value.id,parent.entry.id);assert.equal(created.value.parent_id,parent.entry.id);
  const child=await capture(created.value.id),catalog=catalogue(child.report);
  sameJson(catalog.rows,parent.catalog.rows);sameJson(stripCatalogSize(catalog.summary),stripCatalogSize(parent.catalog.summary));
  assert.equal(catalog.inherited.parent_snapshot_sha256,sha(exact));
  assert.equal(catalog.inherited.parent_catalog_sha256,sha(Buffer.from(JSON.stringify(parent.catalog))),
   'Controlled fixture typed catalog uses the same compact field order/UTF-8 JSON projection');
  assert.equal(catalog.inherited.current_discovery_executed,false);
  assert.equal(child.report.operation.selection.length,2);assert.equal(child.report.operation.refreshed_count,1);
  const immutable=await call('/api/crawls/'+created.value.id+'/parent.json');assert.ok(exact.equals(immutable.bytes));
  assert.ok(exact.equals(await readFile(path.join(ownedData,parent.entry.id+'.json'))));
  assert.equal(child.status.checkpoint.sitemap_ledger.rolling_sha256,catalog.summary.rolling_sha256);
  checked(group,['exact parent bytes and inherited unchanged sitemap rows','duplicate selection coalesces one refresh','no current sitemap discovery; final drained parent route counts establish absence']);
 }
 async function legacy(){
  const group='legacy-none-import-selected-restart';phase=group;diagnosticScenario=group;diagnosticStage='scenario-start';const parent=reports.get('known-empty'),legacy=structuredClone(parent.entry.report);
  for(const field of ['sitemap_catalog','sitemap_coverage_inventory','sitemap_source_measurements'])assert.ok(legacy[field],'Modern positive parent must actually contain '+field);
  // A modern dependent sidecar without its catalog is corruption, not legacy
  // absence. Keep both production guards exercised before constructing an old report.
  for(const [kind,message] of [['measurements','Parent measurements lack source catalog'],['inventory','Coverage inventory requires a sitemap catalog']]){
   const orphan=structuredClone(parent.entry.report);delete orphan.sitemap_catalog;if(kind==='inventory')delete orphan.sitemap_source_measurements;
   const importedOrphan=await call('/api/crawls/import',orphan),file=path.join(ownedData,importedOrphan.value.id+'.json'),original=await readFile(file);
   const before=(await readdir(ownedData)).sort(),interval={id:'orphan-'+kind+'-before-child-creation',started_ms:stamp(),finished_ms:null,after_request_ordinal:totalRequests,through_request_ordinal:null};intervals.push(interval);
   const rejected=await call('/api/crawls/'+importedOrphan.value.id+'/recrawl',{urls:[fixtureOrigin+parent.seed]},400);assert.equal(rejected.value.error,message);
   sameJson((await readdir(ownedData)).sort(),before);assert.ok(original.equals(await readFile(file)));interval.finished_ms=stamp();interval.through_request_ordinal=totalRequests;
  }
  delete legacy.config.explicit_sitemap_inputs;delete legacy.sitemap_catalog;
  delete legacy.sitemap_coverage_inventory;delete legacy.sitemap_source_measurements;
  const absentHistory=report=>{for(const field of ['sitemap_catalog','sitemap_coverage_inventory','sitemap_source_measurements'])assert.equal(report[field]??null,null,field+' must remain absent for a historical report');};absentHistory(legacy);
  const forbidden='/'+group+'/forbidden.xml';route(group,forbidden,index(''),{'content-type':'application/xml'});
  legacy.config.sitemap_urls=[fixtureOrigin+forbidden];legacy.config.discover_sitemaps=true;
  const imported=await call('/api/crawls/import',legacy),importedReport=await call('/api/crawls/'+imported.value.id+'/project.json');
  assert.equal(importedReport.value.sitemap_catalog ?? null,null);assert.equal(importedReport.value.config.explicit_sitemap_inputs ?? null,null);
  absentHistory(importedReport.value);
  const exact=await readFile(path.join(ownedData,imported.value.id+'.json'));
  const created=await call('/api/crawls/'+imported.value.id+'/recrawl',{urls:[fixtureOrigin+parent.seed]});
  const child=await capture(created.value.id);assert.equal(child.report.sitemap_catalog ?? null,null);
  assert.equal(child.report.config.explicit_sitemap_inputs ?? null,null);assert.equal(child.status.checkpoint.sitemap_ledger ?? null,null);
  absentHistory(child.report);
  assert.ok(exact.equals((await call('/api/crawls/'+created.value.id+'/parent.json')).bytes));
  const childBytes=await readFile(path.join(ownedData,created.value.id+'.json'));
  await stop();phase='restart-legacy-read-only';const interval={id:'legacy-startup',started_ms:stamp(),finished_ms:null,
   after_request_ordinal:totalRequests,through_request_ordinal:null};intervals.push(interval);
  await launch('legacy-restart');assert.equal((await call('/api/crawls/'+created.value.id)).value.status,'completed');
  const recovered=await call('/api/crawls/'+created.value.id+'/project.json');assert.equal(recovered.value.sitemap_catalog ?? null,null);
  absentHistory(recovered.value);
  assert.equal(recovered.value.config.explicit_sitemap_inputs ?? null,null);
  assert.ok(exact.equals(await readFile(path.join(ownedData,imported.value.id+'.json'))));
  assert.ok(childBytes.equals(await readFile(path.join(ownedData,created.value.id+'.json'))));interval.finished_ms=stamp();interval.through_request_ordinal=totalRequests;phase=group;diagnosticScenario=group;diagnosticStage='scenario-start';
  expectHit(group,forbidden,0);
  checked(group,['orphan measurements and orphan inventory each rejected400 before child creation or HTTP','publicly imported historical field absence preserved','selected native store and immutable parent remain catalogue-unavailable','restart neither upgrades history nor discovers declared forbidden source']);
 }
 async function corruptParent(){
  const group='invalid-parent-catalog-rejected';phase=group;diagnosticScenario=group;diagnosticStage='scenario-start';const parent=reports.get('ordinary-original-nested');
  const invalid=structuredClone(parent.entry.report);invalid.sitemap_catalog.summary.rolling_sha256='0'.repeat(64);
  const imported=await call('/api/crawls/import',invalid);
  const exact=await readFile(path.join(ownedData,imported.value.id+'.json'));
  const before=(await readdir(ownedData)).sort(),interval={id:'invalid-parent-before-child-creation',started_ms:stamp(),finished_ms:null,
   after_request_ordinal:totalRequests,through_request_ordinal:null};intervals.push(interval);
  const rejected=await call('/api/crawls/'+imported.value.id+'/recrawl',{urls:[fixtureOrigin+parent.page]},400);
  assert.ok(typeof rejected.value.error==='string' && rejected.value.error.length>0);
  sameJson((await readdir(ownedData)).sort(),before);assert.ok(exact.equals(await readFile(path.join(ownedData,imported.value.id+'.json'))));
  interval.finished_ms=stamp();interval.through_request_ordinal=totalRequests;
  checked(group,['bad immutable parent catalogue rejected by selected validation','no derived files created','original imported snapshot bytes preserved; final witness establishes no website requests']);
 }
 async function readSql(){
  assert.ok(apps.every(app=>app.closed),'Read-only SQLite witness runs after owned App closure');
  for(const [id,entry] of jobReports){
   diagnosticScenario=entry.scenario_id;diagnosticStage='readonly-sql-job:'+id;
   const file=path.join(ownedData,id+'.sqlite');assert.match(id,UUID);
   assert.equal(await realpath(ownedData),ownedData);assert.equal(path.dirname(await realpath(file)),ownedData);
   const info=await lstat(file);assert.ok(info.isFile() && !info.isSymbolicLink(),'SQLite witness requires a regular owned job file');
   const db=new DatabaseSync(file,{readOnly:true});
   try{
    assert.ok(db.prepare('SELECT count(*) AS n FROM policy_occurrences').get().n<=512,'Controlled policy SQL witness row budget');
    const policyRows=db.prepare('SELECT sequence,generation,data FROM policy_occurrences ORDER BY sequence').all();
    const metadata=new Map(db.prepare('SELECT key,value FROM metadata').all().map(row=>[row.key,row.value]));
    const saved=JSON.parse(metadata.get('report')),checkpoint=JSON.parse(metadata.get('checkpoint')),config=JSON.parse(metadata.get('config'));
    assert.equal(checkpoint.report_sha256,sha(Buffer.from(metadata.get('report'))));
    assert.equal(checkpoint.config_sha256,sha(Buffer.from(metadata.get('config'))));
    sameJson(saved.sitemap_catalog ?? null,entry.report.sitemap_catalog ?? null);
    sameJson(checkpoint.sitemap_ledger ?? null,entry.status.checkpoint.sitemap_ledger ?? null);
    sameJson(config.explicit_sitemap_inputs ?? null,entry.report.config.explicit_sitemap_inputs ?? null);
    const sqlRows=[];
    for(const table of ['sitemap_documents','sitemap_roots','sitemap_entries','sitemap_locations']){
     for(const row of db.prepare('SELECT sequence,row_id,data,data_sha256 FROM '+table+' ORDER BY sequence').all()){
      assert.equal(row.data_sha256,sha(Buffer.from(row.data)));const value=JSON.parse(row.data);assert.equal(value.row.sequence,row.sequence);
      sqlRows.push({...row,table});
     }
    }
    sqlRows.sort((a,b)=>a.sequence-b.sequence);
    sameJson(sqlRows.map(row=>JSON.parse(row.data)),entry.report.sitemap_catalog?.rows ?? []);
    const catalog=entry.report.sitemap_catalog;
    if(catalog){sameJson(JSON.parse(metadata.get('sitemap_ledger')),catalog.summary);
     if(!catalog.inherited){
      const documents=new Map(rows(catalog,'document').map(row=>[row.observation_id,row]));
      for(const item of catalog.rows){const reference=item.row.admission?.policy_occurrence_id;
       if(reference){const actual=db.prepare('SELECT sequence,generation,data FROM policy_occurrences WHERE occurrence_id=?').get(reference);
        assert.ok(actual);const bound=JSON.parse(actual.data),occurrence=bound.occurrence;
        assert.equal(bound.sequence,actual.sequence);assert.equal(bound.generation,actual.generation);
        assert.equal(occurrence.decision.occurrence_id,reference);
        assert.equal(occurrence.source_revision,item.row.admission.policy_source_revision);
        if(item.kind==='location'){
         const original=item.row.observed,document=documents.get(item.row.observation_id);assert.ok(document);
         assert.equal(bound.generation,document.generation);assert.equal(occurrence.source_revision,document.source_revision);
         assert.equal(occurrence.source_ordinal,original.ordinal);assert.equal(occurrence.input_basis,'parser_observed_xml');
         assert.equal(occurrence.raw_value_observed,true);assert.equal(occurrence.decision.raw_value,original.decoded_value);
         assert.equal(occurrence.decision.source_url,document.task_url);
         assert.equal(occurrence.task_role,original.role==='page_location'?'page_document':'sitemap_document');
         assert.equal(occurrence.decision.source_kind,original.role==='page_location'?'sitemap_location':'sitemap_document');
         if(original.validation_error || original.normalized_fetch_url===null){
          assert.equal(occurrence.input_error,original.validation_error || 'Parser supplied no accepted normalized URL');
          assert.ok(['policy_denied','input_row_rejected'].includes(occurrence.admission_reason));
          assert.equal(occurrence.scheduled_fetch_url,null);assert.equal(occurrence.coalesced_task_occurrence_id,null);
          assert.equal(item.row.admission.decision,'invalid_location');assert.equal(item.row.admission.admitted_fetch_url,null);
         }
        }else if(item.kind==='root'){
         assert.equal(occurrence.decision.source_kind,'sitemap_document');
         assert.equal(occurrence.input_basis,'declared_input');assert.equal(occurrence.raw_value_observed,true);
         assert.equal(occurrence.decision.raw_value,item.row.raw_value);
        }
       }
      }
     }
    }
    const pending=db.prepare('SELECT count(*) AS n FROM discovery_frontier WHERE done=0').get().n;
    assert.equal(pending,checkpoint.discovery_pending);
    sqlWitnesses.push({job_id:id,sqlite_path:file,connection_read_only:true,checkpoint,config_utf8:metadata.get('config'),
     report_utf8:metadata.get('report'),sitemap_summary_utf8:metadata.get('sitemap_ledger') ?? null,
     inherited_basis_utf8:metadata.get('sitemap_inherited_basis') ?? null,
     discovery_pending:pending,rows:sqlRows,policy_rows:policyRows});
   }finally{db.close();}
  }
 }
 async function drain(){
  assert.ok(apps.length>0 && apps.every(app=>app.closed));assert.ok(site.listening);
  const end=performance.now()+LIMITS.drainMs;
  while(performance.now()<end){if(active.size===0 && sockets.size===0 && performance.now()-lastActivity>=LIMITS.quietMs)break;await delay(25);}
  assert.equal(active.size,0);assert.equal(sockets.size,0);assert.ok(performance.now()-lastActivity>=LIMITS.quietMs);
  const before={recorded_at:new Date().toISOString(),all_owned_apps_closed:true,fixture_kept_listening_until_drained:site.listening,
   active_requests:active.size,connected_sockets:sockets.size,quiet_ms:LIMITS.quietMs,total_requests:totalRequests};
  await bounded(new Promise((resolve,reject)=>site.close(error=>error?reject(error):resolve())),LIMITS.fixtureCloseMs,'clean fixture close');
  fixtureClosed=true;assert.equal(active.size,0);assert.equal(sockets.size,0);assert.equal(hits.length,totalRequests);
  assert.equal(witnessOverflow,false);assert.ok(hits.every(hit=>hit.state==='finished' && !hit.request_error && hit.method==='GET' && hit.registered_route));
  drainFacts={...before,fixture_closed_cleanly:true,forced_socket_cleanup:false,accepted_connections:acceptedConnections,final_total_requests:totalRequests};
  witnessDrained=true;
 }
 try{
  await bounded(new Promise((resolve,reject)=>{site.once('error',reject);site.listen(0,'127.0.0.1',resolve);}),LIMITS.startMs,'owned fixture bind');
  const bound=site.address();assert.ok(bound && typeof bound==='object' && !FORBIDDEN_PORTS.has(bound.port));
  fixtureOrigin='http://127.0.0.1:'+bound.port;await launch('initial');
  await rootsAndNested('ordinary-original-nested',false);await rootsAndNested('policy-original-alias',true);
  await robots('robots-complete-originals',false);await robots('robots-206-incomplete',true);
  await corruptedGzip();await knownEmpty();await pauseRestart();
  await selected('selected-ordinary-inheritance','ordinary-original-nested');
  await selected('selected-policy-inheritance','policy-original-alias');await legacy();await corruptParent();
  assert.equal(checks.length,SCENARIOS.length);sameJson(checks.map(item=>item.id),SCENARIOS);
 }catch(error){fail('native_scenarios',error);}
 try{await stop();}catch(error){fail('owned_app_close',error);
  if(currentApp && !currentApp.closed){currentApp.child.stdout?.destroy();currentApp.child.stderr?.destroy();currentApp.child.unref();}}
 try{await drain();}catch(error){fail('final_fixture_drain',error);
  if(site.listening){const close=new Promise(resolve=>site.close(()=>resolve()));site.closeAllConnections();for(const socket of sockets)socket.destroy();
   try{await bounded(close,LIMITS.fixtureCloseMs,'failed fixture cleanup');fixtureClosed=true;}catch(cleanup){fail('fixture_cleanup',cleanup);}}}
 try{await readSql();}catch(error){fail('readonly_sql_witness',error);}
 for(const [name,handler] of signalHandlers)process.off(name,handler);
 for(const app of apps)if(app.spawnError || app.bindError || !app.closed || !app.plannedStop)fail('owned_app_integrity',app.spawnError || app.bindError || Error('Unproven owned App closure'));
 if(abort.signal.aborted)fail('interrupted',abort.signal.reason);
 let finalBinarySha=null;try{finalBinarySha=await hashFile(binary);assert.equal(finalBinarySha,binarySha);
  assert.equal(await hashFile(harnessPath),harnessSha);}catch(error){fail('final_source_file_binding',error);}
 const finalHits=[...expectedHits.values()].map(item=>({...item,actual:count(hits,item.group,item.route),
  witness_phase:witnessDrained?'after_all_owned_apps_close_and_clean_fixture_drain':'NOT_PROVEN'}));
 const zeroIntervals=intervals.map(item=>({...item,website_requests: item.finished_ms===null?null:
  hits.filter(hit=>hit.ordinal>item.after_request_ordinal && hit.ordinal<=item.through_request_ordinal).length,
  witness_basis:'final_drained_rows_between_exact_observed_request_ordinals'}));
 if(!witnessDrained || finalHits.some(item=>item.actual!==item.expected) || zeroIntervals.some(item=>item.website_requests!==0))fail('final_http_oracle',Error('Exact final expected counts and zero-request intervals require a drained witness'));
 if(checks.length!==SCENARIOS.length || new Set(checks.map(item=>item.id)).size!==SCENARIOS.length || !finalHits.some(item=>item.expected>0))fail('nonempty_matrix',Error('All 11 unique scenario groups and positive HTTP controls are mandatory'));
 if(sqlWitnesses.length!==jobReports.size || jobReports.size!==11)fail('sqlite_witness_scope',Error('All eleven tracked native jobs, including the protocol query-preserve subcase, require actual read-only SQLite witnesses'));
 const fixtures=recipes.map(value=>({recipe_id:value.recipe_id,path:value.path,group:value.group,
  registered_ms:value.registered_ms,status:value.status,headers:value.headers,
  body_base64:value.body.toString('base64'),body_sha256:sha(value.body),hold:value.hold}));
 for(const hit of hits){
  const recipe=fixtures.find(value=>value.recipe_id===hit.recipe_id);
  if(!recipe || recipe.path!==hit.path || recipe.group!==hit.group ||
   recipe.status!==hit.response_status || recipe.body_sha256!==hit.response_body_sha256)
   fail('final_recipe_binding',Error('Every HTTP witness must bind the retained recipe version actually selected by its handler'));
 }
 const logs=apps.map(app=>({ordinal:app.ordinal,label:app.label,pid:app.pid,origin:app.api,started_ms:app.started_ms,
  bind_line:app.bind_line,bind_line_sha256:app.bind_line?sha(Buffer.from(app.bind_line)):null,stop_requested_ms:app.stop_requested_ms,
  planned_stop:app.plannedStop,closed:app.closed,close_meta:app.close_meta,
  stdout:app.stdout,stderr:app.stderr,stdout_base64:Buffer.concat(app.stdout_raw).toString('base64'),
  stderr_base64:Buffer.concat(app.stderr_raw).toString('base64'),stdout_bytes:app.stdout_bytes,stderr_bytes:app.stderr_bytes,
  stdout_truncated:app.stdout_truncated,stderr_truncated:app.stderr_truncated}));
 const payloads={requests:hits,api:apiRows,log:logs,sql:sqlWitnesses,fixtures},companions={};
 for(const [name,value] of Object.entries(payloads)){const raw=Buffer.from(JSON.stringify(value,null,2)+'\n');
  await writeFile(companionPaths[name],raw,{flag:'wx'});companions[name]={file:companionPaths[name],bytes:raw.length,sha256:sha(raw)};}
 const proof={schema_version:1,result:failures.length?'FAIL':'PASS',scope_name:'native_sitemap_durable_11_drained_v5',
  recorded_at:new Date().toISOString(),source_commit:process.env.SPIDER_SOURCE_COMMIT,
  expected_source_commit:process.env.SPIDER_EXPECTED_SOURCE_COMMIT,source_basis:'caller_declared; independent CI build/manifest binding required',
  source_runtime_attested:false,binary_path:binary,binary_bytes:binaryInfo.size,binary_sha256:binarySha,
  expected_binary_sha256:process.env.SPIDER_BINARY_SHA256,final_binary_sha256:finalBinarySha,harness_sha256:harnessSha,
  scenario_ids:SCENARIOS,matrix_sha256:sha(Buffer.from(JSON.stringify(SCENARIOS))),platform:process.platform,node_version:process.version,limits:LIMITS,
  fixture_origin:fixtureOrigin ?? null,user_runtime_contacted:false,renderer_contacted:false,
  owned_api_origin_basis:'exact complete post-bind lines from run-owned child handles',owned_apps:logs.map(({stdout,stderr,stdout_base64,stderr_base64,...rest})=>rest),
  all_owned_apps_closed:apps.length>0 && apps.every(app=>app.closed && app.plannedStop),fixture_closed_cleanly:fixtureClosed && witnessDrained,
  final_target_http_witness:witnessDrained?'DRAINED':'NOT_PROVEN',drain:drainFacts,witness_complete:!witnessOverflow && hits.length===totalRequests,
  scenarios:checks,final_http_counts:finalHits,final_zero_request_intervals:zeroIntervals,companions,
  read_only_sqlite_witness_jobs:sqlWitnesses.length,tracked_native_jobs:jobReports.size,
  data_directory_preserved:ownedData,run_directory_preserved:ownedRun,failures,
  proof_limits:['11 owned native scenario groups only','imported legacy absence is an explicit API snapshot contract, not an old-version migration test',
   'invalid parent catalogue is rejected at selected preparation; report import is not asserted to reject it',
   'no SQL corruption writes or durable row/catalog-budget injection','no exactly-once remote request or power-loss guarantee',
   'pending dedicated sitemap API/MCP/CSV/UI are not covered','declared source-cohort completeness and full reference parity remain NOT_PROVEN'],
  declared_source_cohort_complete:'NOT_PROVEN',reference_parity:'NOT_PROVEN'};
 await writeFile(proofPath,JSON.stringify(proof,null,2)+'\n',{flag:'wx'});
 console.log(JSON.stringify({result:proof.result,scenarios:checks.length,sqlite_jobs:sqlWitnesses.length,
  final_target_http_witness:proof.final_target_http_witness,proof:proofPath,binary_sha256:binarySha}));
 if(failures.length){process.exitCode=1;console.error(failures.map(item=>item.phase+': '+item.message).join('\n'));}
 return proof;
}
if(process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url))await main();
