// Native bounded sitemap transport acceptance with final drained HTTP witnesses.
// Node built-ins only; all HTTP endpoints and process handles are created by this run.
import assert from 'node:assert/strict';
import http from 'node:http';
import path from 'node:path';
import os from 'node:os';
import {createReadStream} from 'node:fs';
import {access, mkdir, mkdtemp, readFile, realpath, rm, stat, writeFile} from 'node:fs/promises';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {gzipSync} from 'node:zlib';
import {fileURLToPath} from 'node:url';

const FORBIDDEN_PORTS = new Set([8787, 8790]);
const LIMITS = Object.freeze({runMs:120_000, startupMs:20_000, caseMs:30_000,
  requestMs:5_000, stopMs:5_000, drainMs:8_000, quietMs:250, fixtureCloseMs:5_000,
  apiBytes:8*1024*1024, outputTailChars:65536, bindLineChars:8192, witnessRequests:1024});
const SHA256 = /^[0-9a-f]{64}$/;
const COMMIT = /^[0-9a-f]{40}$/;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
export const sha = bytes => createHash('sha256').update(bytes).digest('hex');

export function matrix() {
  const cases=[];
  for (const policy of [false,true]) for (const resources of [false,true])
    cases.push({id:`plain-${Number(policy)}${Number(resources)}`,policy,resources,format:'plain',extension:false,target:1});
  for (const policy of [false,true]) for (const resources of [false,true])
    cases.push({id:`file-gzip-${Number(policy)}${Number(resources)}`,policy,resources,format:'file-gzip',extension:false,target:1});
  for (const format of ['misleading-gz','http-gzip','corrupt-gzip']) for (const policy of [false,true])
    cases.push({id:`${format}-${Number(policy)}`,policy,resources:false,format,extension:false,target:format==='corrupt-gzip'?0:1});
  for (const policy of [false,true])
    cases.push({id:`extension-${Number(policy)}`,policy,resources:false,format:'plain',extension:true,target:1});
  return cases;
}

export function parsePostBindLine(line) {
  const match=/^Open SEO Spider: (http:\/\/127\.0\.0\.1:([1-9][0-9]*))\r?$/.exec(line);
  assert.ok(match,'Only exact complete post-bind App stdout is an endpoint source');
  const port=Number(match[2]);
  assert.ok(Number.isInteger(port) && port<=65535,'Invalid bound App port');
  assert.ok(!FORBIDDEN_PORTS.has(port),'User runtime ports must never be contacted');
  const url=new URL(match[1]);assert.equal(url.port,String(port));assert.equal(url.hostname,'127.0.0.1');
  return url.origin;
}

export function finalCaseResult(observed,hits,witnessDrained) {
  const count=route=>hits.filter(hit=>hit.path===route).length;
  const actual={seed_requests:count(observed.seed),sitemap_requests:count(observed.map),
    target_requests:count(observed.target),target_pages:observed.target_pages,
    foreign_requests:count(observed.foreign),foreign_pages:observed.foreign_pages};
  const expected={seed_requests:1,sitemap_requests:1,target_requests:observed.definition.target,
    target_pages:observed.definition.target,foreign_requests:0,foreign_pages:0};
  const passed=Object.keys(expected).every(key=>actual[key]===expected[key]);
  return {id:observed.definition.id,policy:observed.definition.policy,resources:observed.definition.resources,
    format:observed.definition.format,extension:observed.definition.extension,
    job_id:observed.job_id,generation:observed.generation,terminal_status:observed.terminal_status,
    report_sha256:observed.report_sha256,config_sha256:observed.config_sha256,
    warning_count:observed.warning_count,warnings:observed.warnings,expected,actual,
    http_counts_phase:witnessDrained?'after_owned_app_close_and_clean_fixture_drain':'incomplete_witness',
    result:!witnessDrained?'NOT_PROVEN':passed?'EXPECTED_BEHAVIOR_PASS':'EXPECTED_BEHAVIOR_FAILURE'};
}

async function hashFile(file) {
  const hash=createHash('sha256');for await(const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest('hex');
}
async function bounded(promise,ms,label) {
  let timer;
  try {return await Promise.race([promise,new Promise((_,reject)=>{
    timer=setTimeout(()=>reject(Error(`Timeout: ${label}`)),ms);
  })]);} finally {clearTimeout(timer);}
}
async function absent(file) {
  try {await access(file);} catch(error) {if(error.code==='ENOENT')return;throw error;}
  throw Error(`Refusing to overwrite an existing proof companion: ${file}`);
}

export async function main() {
  // Validate external inputs before creating either listener or launching anything.
  assert.ok(process.env.SPIDER_BINARY,'Explicit SPIDER_BINARY is required');
  const binary=await realpath(path.resolve(process.env.SPIDER_BINARY));
  const binaryStat=await stat(binary);assert.ok(binaryStat.isFile(),'SPIDER_BINARY must be a regular file');
  const expectedBinary=process.env.SPIDER_BINARY_SHA256 || null;
  if(expectedBinary!==null)assert.match(expectedBinary,SHA256,'Invalid optional expected binary SHA');
  const sourceCommit=process.env.SPIDER_SOURCE_COMMIT || process.env.GITHUB_SHA || null;
  const expectedSource=process.env.SPIDER_EXPECTED_SOURCE_COMMIT || null;
  if(sourceCommit!==null)assert.match(sourceCommit,COMMIT,'Declared source must be a full lowercase commit');
  if(expectedSource!==null){assert.match(expectedSource,COMMIT);assert.equal(sourceCommit,expectedSource,'Declared source differs from expected source');}
  const binarySha=await hashFile(binary);
  if(expectedBinary!==null)assert.equal(binarySha,expectedBinary,'Executable differs from expected SHA before any launch');
  const harnessPath=fileURLToPath(import.meta.url),harnessSha=await hashFile(harnessPath);
  const runDir=await mkdtemp(path.join(process.env.RUNNER_TEMP || os.tmpdir(),'sitemap-transport-e2e-'));
  const ownedRun=await realpath(runDir),data=path.join(ownedRun,'data');await mkdir(data);
  const ownedData=await realpath(data);
  const proofPath=path.resolve(process.env.SPIDER_SITEMAP_TRANSPORT_PROOF || path.join(ownedRun,'proof.json'));
  const logPath=proofPath+'.app.log',requestsPath=proofPath+'.requests.json';
  try{
    await mkdir(path.dirname(proofPath),{recursive:true});
    for(const file of [proofPath,logPath,requestsPath])await absent(file);
  }catch(error){
    // No listeners/processes exist yet; clean only our freshly created data directory.
    assert.equal(await realpath(data),ownedData);assert.equal(path.dirname(ownedData),ownedRun);
    await rm(ownedData,{recursive:true,force:false});throw error;
  }

  const started=Date.now(),deadline=started+LIMITS.runMs;
  const abort=new AbortController(),signalHandlers=[];
  for(const name of ['SIGINT','SIGTERM']){
    const handler=()=>abort.abort(Error(`Harness interrupted by ${name}`));
    process.once(name,handler);signalHandlers.push([name,handler]);
  }
  const routes=new Map(),hits=[],active=new Set(),sockets=new Set(),observed=[],failures=[];
  const fixtureErrors=[];let requestOverflow=false,totalRequests=0,acceptedConnections=0,lastActivity=Date.now();
  let fixtureOrigin,api,child,closed,spawnFailure,closedMeta,stdout='',stderr='',pendingLine='',bindLine,bindFailure;
  let plannedStop=false,appClosed=false,fixtureClosed=false,witnessDrained=false,drainFacts=null,dataRemoved=false,stopRequestedMs=null;
  const stamp=()=>Date.now()-started;
  const fail=(phase,error)=>failures.push({phase,message:String(error?.message || error).slice(0,4000)});
  const alive=()=>{
    if(abort.signal.aborted)throw abort.signal.reason;
    if(Date.now()>=deadline)throw Error('Harness overall deadline exceeded');
    if(spawnFailure)throw spawnFailure;
    if(bindFailure)throw bindFailure;
    if(fixtureErrors.length)throw Error(`Owned fixture error: ${fixtureErrors.join('; ')}`);
    if(child && !plannedStop && (child.exitCode!==null || child.signalCode!==null))
      throw Error('Owned App exited before completion');
  };
  async function until(fn,timeout,label){
    const end=Math.min(deadline,Date.now()+timeout);
    while(Date.now()<end){alive();const value=await fn();if(value)return value;await delay(25);}
    throw Error(`Timeout: ${label}`);
  }
  const site=http.createServer((req,res)=>{
    totalRequests++;lastActivity=Date.now();
    if(hits.length>=LIMITS.witnessRequests){requestOverflow=true;req.resume();res.writeHead(503);res.end('Witness budget exceeded');return;}
    const record={ordinal:totalRequests,method:req.method,path:req.url,started_ms:stamp(),finished_ms:null,response_state:'active'};
    hits.push(record);active.add(record);
    const settle=state=>{if(!active.delete(record))return;record.response_state=state;record.finished_ms=stamp();lastActivity=Date.now();};
    res.once('finish',()=>settle('finished'));res.once('close',()=>settle('aborted'));
    req.on('error',error=>{record.request_error=error.code || error.message;});req.resume();
    const route=routes.get(req.url);
    if(!route){record.registered_route=false;res.writeHead(404);res.end('No fixture route');return;}
    record.registered_route=true;res.writeHead(200,route.headers);res.end(route.body);
  });
  site.on('error',error=>fixtureErrors.push(String(error.message)));
  site.on('connection',socket=>{
    acceptedConnections++;sockets.add(socket);lastActivity=Date.now();
    socket.on('error',()=>{});socket.once('close',()=>{sockets.delete(socket);lastActivity=Date.now();});
  });
  async function json(route,body){
    alive();assert.ok(api && child?.pid,'API requires owned post-bind child endpoint');
    assert.ok(route==='/healthz' || route==='/api/crawls' || /^\/api\/crawls\/[a-f0-9-]+(?:\/project\.json)?$/i.test(route));
    const signal=AbortSignal.any([abort.signal,AbortSignal.timeout(Math.max(1,Math.min(LIMITS.requestMs,deadline-Date.now())))]);
    const response=await fetch(api+route,{redirect:'error',signal,...(body===undefined?{}:
      {method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)})});
    const chunks=[];let bytes=0;
    if(response.body){const reader=response.body.getReader();try{
      while(true){const next=await reader.read();if(next.done)break;bytes+=next.value.byteLength;
        if(bytes>LIMITS.apiBytes){await reader.cancel();throw Error('Owned API JSON exceeded bounded response budget');}
        chunks.push(Buffer.from(next.value));}
    }finally{reader.releaseLock();}}
    const buffer=Buffer.concat(chunks);alive();
    assert.ok(response.ok,`${route}: HTTP ${response.status}: ${buffer.toString('utf8').slice(0,2000)}`);
    return {value:JSON.parse(buffer.toString('utf8')),bytes:buffer};
  }
  function appendStdout(chunk){
    const text=chunk.toString('utf8');stdout=(stdout+text).slice(-LIMITS.outputTailChars);
    pendingLine+=text;
    let end;
    while((end=pendingLine.indexOf('\n'))!==-1){const line=pendingLine.slice(0,end);pendingLine=pendingLine.slice(end+1);
      if(line.startsWith('Open SEO Spider:'))try{
        const found=parsePostBindLine(line);assert.equal(api,undefined,'App emitted more than one bind announcement');
        assert.notEqual(found,fixtureOrigin,'App endpoint cannot be fixture endpoint');api=found;bindLine=line;
      }catch(error){bindFailure=error;}}
    if(pendingLine.length>LIMITS.bindLineChars){pendingLine='';bindFailure=Error('Incomplete child stdout line exceeded bounded bind profile');}
  }
  async function stopOwnedApp(){
    if(!child)return;
    if(child.exitCode!==null || child.signalCode!==null){await closed;appClosed=true;
      throw Error('Owned App closed before planned terminal shutdown');}
    plannedStop=true;stopRequestedMs=stamp();
    if(child.exitCode===null && child.signalCode===null)child.kill('SIGTERM');
    try{await bounded(closed,LIMITS.stopMs,'owned App close');}
    catch{if(child.exitCode===null && child.signalCode===null)child.kill('SIGKILL');
      await bounded(closed,LIMITS.stopMs,'owned App force-close');}
    appClosed=true;
  }
  async function drainOwnedFixture(){
    if(!site.listening){assert.ok(!fixtureOrigin,'Owned fixture closed before drain');return;}
    assert.ok(appClosed,'Final witness requires owned App close before listener drain');
    const end=Date.now()+LIMITS.drainMs;
    while(Date.now()<end){
      assert.ok(site.listening,'Fixture must remain listening while requests drain');
      if(active.size===0 && sockets.size===0 && Date.now()-lastActivity>=LIMITS.quietMs)break;
      await delay(25);
    }
    assert.equal(active.size,0,'Owned fixture requests did not drain');
    assert.equal(sockets.size,0,'Owned fixture sockets did not drain');
    assert.ok(Date.now()-lastActivity>=LIMITS.quietMs,'Owned fixture did not become quiescent');
    const beforeClose={recorded_at:new Date().toISOString(),owned_app_closed:true,
      fixture_kept_listening_until_requests_drained:site.listening,quiet_ms:LIMITS.quietMs,
      active_requests:active.size,connected_sockets:sockets.size,total_requests:totalRequests};
    await bounded(new Promise((resolve,reject)=>site.close(error=>error?reject(error):resolve())),LIMITS.fixtureCloseMs,'clean owned fixture close');
    fixtureClosed=true;
    assert.equal(active.size,0);assert.equal(sockets.size,0);assert.equal(totalRequests,hits.length,'No witness rows may be dropped');
    assert.ok(!requestOverflow,'Witness request limit exceeded');
    assert.ok(hits.every(hit=>hit.response_state==='finished' && !hit.request_error),'Every observed response must finish before final counts');
    assert.ok(hits.every(hit=>hit.method==='GET' && hit.registered_route),'Unexpected fixture requests invalidate controlled matrix');
    drainFacts={...beforeClose,fixture_closed_cleanly:true,forced_socket_cleanup:false,final_total_requests:totalRequests};
    witnessDrained=true;
  }
  async function cleanupOwnedFixture(){
    if(!site.listening && fixtureClosed)return;
    if(!site.listening && !fixtureOrigin)return;
    // Failing drain is a test failure, never a successful negative-request witness.
    const closure=new Promise(resolve=>site.close(()=>resolve()));
    site.closeAllConnections();for(const socket of sockets)socket.destroy();
    await bounded(closure,LIMITS.fixtureCloseMs,'failed-run owned fixture cleanup');fixtureClosed=true;
  }
  async function removeOwnedData(){
    if(child && !appClosed)throw Error('Cannot remove data while owned App close is unproven');
    const actual=await realpath(data);
    assert.equal(actual,ownedData,'Owned data path changed');
    assert.equal(path.dirname(actual),ownedRun,'Data deletion must remain in this mkdtemp run');
    assert.equal(path.basename(actual),'data');
    await rm(actual,{recursive:true,force:false});dataRemoved=true;
  }
  async function caseRun(definition){
    const seed='/seed-'+definition.id,map='/map-'+definition.id+(definition.format==='misleading-gz'?'.xml.gz':'.xml');
    const target='/target-'+definition.id,foreign='/foreign-'+definition.id;
    const xml=`<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:x="urn:owned-extension"><url><loc>${fixtureOrigin+target}</loc>${definition.extension?`<x:wrapper><x:loc>${fixtureOrigin+foreign}</x:loc></x:wrapper>`:''}</url></urlset>`;
    let body=Buffer.from(xml);const headers={'content-type':'application/xml; charset=utf-8'};
    if(['file-gzip','http-gzip','corrupt-gzip'].includes(definition.format))body=gzipSync(body);
    if(definition.format==='corrupt-gzip')body[body.length-8]^=1;
    if(definition.format==='http-gzip')headers['content-encoding']='gzip';
    routes.set(seed,{body:Buffer.from('<html><title>Owned seed</title><body>No references</body></html>'),headers:{'content-type':'text/html'}});
    routes.set(map,{body,headers});for(const route of [target,foreign])routes.set(route,
      {body:Buffer.from('<html><title>Owned target</title><body>Leaf</body></html>'),headers:{'content-type':'text/html'}});
    const config={seed_urls:[fixtureOrigin+seed],sitemap_urls:[fixtureOrigin+map],discover_sitemaps:false,
      crawl_links:false,obey_robots:false,concurrency:1,delay_ms:0,max_pages:10,max_body_bytes:1024*1024};
    if(definition.resources)config.resource_discovery={};
    if(definition.policy)config.crawl_policy={scope:{roots:[{origin:fixtureOrigin,folders:['/']}],external:'deny',outside_folder:'deny'}};
    const {value:job}=await json('/api/crawls',config);assert.match(job.id,UUID);
    const done=await until(async()=>{const {value}=await json('/api/crawls/'+job.id);
      return ['completed','failed','cancelled'].includes(value.status)&&value;},LIMITS.caseMs,'terminal case '+definition.id);
    assert.equal(done.status,'completed','Case must complete before process shutdown');
    const exported=await json('/api/crawls/'+job.id+'/project.json'),report=exported.value;
    assert.ok(Array.isArray(report.pages));assert.ok(Array.isArray(report.warnings));
    assert.ok(report.warnings.length<=100 && report.warnings.every(value=>typeof value==='string' && value.length<=8192),'Warnings exceed bounded proof profile');
    assert.equal(report.pages.filter(page=>page.url===fixtureOrigin+seed).length,1,'Independent seed page control');
    observed.push({definition,seed,map,target,foreign,job_id:job.id,generation:done.generation ?? null,
      terminal_status:done.status,config_sha256:sha(Buffer.from(JSON.stringify(config))),report_sha256:sha(exported.bytes),
      target_pages:report.pages.filter(page=>page.url===fixtureOrigin+target).length,
      foreign_pages:report.pages.filter(page=>page.url===fixtureOrigin+foreign).length,
      warning_count:report.warnings.length,warnings:report.warnings});
  }

  try{
    await bounded(new Promise((resolve,reject)=>{site.once('error',reject);site.listen(0,'127.0.0.1',resolve);}),LIMITS.startupMs,'owned fixture listen');
    const address=site.address();assert.ok(address && typeof address==='object');
    assert.ok(address.port>0 && !FORBIDDEN_PORTS.has(address.port));fixtureOrigin='http://127.0.0.1:'+address.port;
    assert.equal(await hashFile(binary),binarySha,'Executable changed before native spawn');
    child=spawn(binary,['serve','--listen','127.0.0.1:0','--data-dir',ownedData],
      {cwd:process.cwd(),windowsHide:true,stdio:['ignore','pipe','pipe']});
    closed=new Promise(resolve=>child.once('close',(code,signal)=>{closedMeta={code,signal,closed_ms:stamp()};resolve();}));
    child.once('error',error=>{spawnFailure=error;});child.stdout.on('data',appendStdout);
    child.stderr.on('data',chunk=>{stderr=(stderr+chunk.toString('utf8')).slice(-LIMITS.outputTailChars);});
    await until(()=>api,LIMITS.startupMs,'complete owned post-bind stdout');
    await until(async()=>{try{const health=await json('/healthz');return health.value.ok===true && health.value.name==='Open SEO Spider';}
      catch(error){alive();return false;}},LIMITS.startupMs,'owned App health');
    for(const definition of matrix())await caseRun(definition);
    assert.equal(observed.length,16);
  }catch(error){fail('run',error);}
  // Final request counts are deliberately evaluated AFTER these owned-only closures.
  try{await stopOwnedApp();}catch(error){fail('owned_app_close',error);
    if(child && !appClosed){
      // A failed kill/close remains FAIL/NOT_PROVEN. Do not hang indefinitely or delete its data.
      child.stdout?.destroy();child.stderr?.destroy();child.unref();
    }}
  try{await drainOwnedFixture();}catch(error){fail('final_fixture_drain',error);
    try{await cleanupOwnedFixture();}catch(cleanupError){fail('fixture_cleanup',cleanupError);}}
  try{await removeOwnedData();}catch(error){fail('owned_data_cleanup',error);}
  for(const [name,handler] of signalHandlers)process.off(name,handler);
  if(abort.signal.aborted)fail('interrupted',abort.signal.reason);
  if(bindFailure)fail('final_bind_integrity',bindFailure);
  if(spawnFailure)fail('final_child_integrity',spawnFailure);
  if(fixtureErrors.length)fail('final_fixture_integrity',Error(fixtureErrors.join('; ')));
  let finalBinarySha=null;
  try{finalBinarySha=await hashFile(binary);assert.equal(finalBinarySha,binarySha,'Executable changed during native fixture');}
  catch(error){fail('final_binary_binding',error);}
  const results=observed.map(item=>finalCaseResult(item,hits,witnessDrained));
  if(results.length!==16 || results.some(item=>item.result!=='EXPECTED_BEHAVIOR_PASS'))
    fail('final_matrix',Error('All 16 exact expected cases require a drained final HTTP witness'));
  if(!appClosed || !fixtureClosed || !witnessDrained || !dataRemoved)
    fail('final_ownership',Error('Owned App/fixture/data cleanup was incomplete'));
  const logBytes=Buffer.from('STDOUT TAIL\n'+stdout+'\nSTDERR TAIL\n'+stderr+'\n');
  const requestBytes=Buffer.from(JSON.stringify(hits,null,2)+'\n');
  try{assert.equal(await hashFile(harnessPath),harnessSha,'Harness file changed while it ran');}
  catch(error){fail('final_harness_binding',error);}
  const proof={schema_version:1,result:failures.length===0?'PASS':'FAIL',scope_name:'native_sitemap_transport_16_drained_v1',recorded_at:new Date().toISOString(),
    scope:'16 existing owned native sitemap transport cases; final HTTP counts only after owned App close and clean fixture drain. No commercial parity or complete occurrence ledger claim.',
    source_commit:sourceCommit,expected_source_commit:expectedSource,
    source_basis:'caller_declared; CI checkout/build/manifest must independently bind commit to executable',
    source_runtime_attested:false,binary_path:binary,binary_bytes:binaryStat.size,binary_sha256:binarySha,
    final_binary_sha256:finalBinarySha,expected_binary_sha256:expectedBinary,expected_binary_sha_checked:expectedBinary!==null,
    harness_sha256:harnessSha,matrix_sha256:sha(Buffer.from(JSON.stringify(matrix()))),
    platform:process.platform,node_version:process.version,limits:LIMITS,
    fixture_origin:fixtureOrigin ?? null,owned_native_origin:api ?? null,owned_native_pid:child?.pid ?? null,
    bind_origin_basis:'complete post-bind owned child stdout only',bind_line_sha256:bindLine?sha(Buffer.from(bindLine)):null,
    listen_argument:'127.0.0.1:0',user_runtime_contacted:false,renderer_contacted:false,
    foreign_listener_or_process_controls_used:false,owned_app_stop_requested_ms:stopRequestedMs,
    owned_app_close:closedMeta ?? null,owned_app_closed:appClosed,
    final_target_http_witness:witnessDrained?'DRAINED':'NOT_PROVEN',drain:drainFacts,
    witness_complete:!requestOverflow && hits.length===totalRequests,requests_sha256:sha(requestBytes),
    requests_file:requestsPath,app_log_sha256:sha(logBytes),app_log_file:logPath,
    fixture_routes:[...routes].map(([route,value])=>({route,body_bytes:value.body.length,body_sha256:sha(value.body),headers:value.headers})),
    cases:results,summary:{cases:results.length,passes:results.filter(item=>item.result==='EXPECTED_BEHAVIOR_PASS').length,
      failures:results.filter(item=>item.result==='EXPECTED_BEHAVIOR_FAILURE').length,
      unproven:results.filter(item=>item.result==='NOT_PROVEN').length},
    cleanup:{owned_data_removed:dataRemoved,owned_fixture_closed:fixtureClosed,owned_app_closed:appClosed,
      run_directory_preserved_for_proof:ownedRun},failures,reference_parity:'NOT_PROVEN'};
  await writeFile(logPath,logBytes,{flag:'wx'});await writeFile(requestsPath,requestBytes,{flag:'wx'});
  await writeFile(proofPath,JSON.stringify(proof,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({result:proof.result,summary:proof.summary,proof:proofPath,
    binary_sha256:binarySha,final_target_http_witness:proof.final_target_http_witness}));
  if(failures.length){process.exitCode=1;console.error(failures.map(item=>item.phase+': '+item.message).join('\n'));}
  return proof;
}

// Importing pure oracles below does not create a listener or launch a process.
if(process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url))
  await main();
