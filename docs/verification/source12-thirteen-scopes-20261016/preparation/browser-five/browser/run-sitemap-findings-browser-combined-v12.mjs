// Authored future native acceptance driver. Do not execute during compute HOLD.
import assert from 'node:assert/strict';
import path from 'node:path';
import fs from 'node:fs/promises';
import {createWriteStream} from 'node:fs';
import {spawn} from 'node:child_process';
import http from 'node:http';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
import {preflight,sha} from './preflight.mjs';

assert.deepEqual(process.argv.slice(2,3),['--build-mode']);assert.equal(process.argv[4],'--release');assert.equal(process.argv[6],'--manifest-sha');assert.equal(process.argv.length,8);assert.match(process.argv[7],/^[a-f0-9]{64}$/);
const preparationRoot=path.dirname(fileURLToPath(import.meta.url));
const admission=await preflight({mode:process.argv[3],releasePath:process.argv[5],driverPath:fileURLToPath(import.meta.url),preparationRoot,expectedManifestSha:process.argv[7]});
// All active imports, output creation, listening, SQLite access and process spawn are below verified admission.
const executionStartedNs=process.hrtime.bigint(),EXECUTION_DEADLINE_MS=600000,EXECUTION_DEADLINE_NS=BigInt(EXECUTION_DEADLINE_MS)*1000000n;
const closeElapsed=()=>String(process.hrtime.bigint()-executionStartedNs);
const {runNativeFindingsBrowser}=await import('./browser-flow.mjs');
const {runSitemapSourceProfileForm}=await import('./profile-form-flow.mjs');
const {runMembershipCapacityForm}=await import('./browser-membership-flow.mjs');
const {runOriginalBodyFormResponsive}=await import('./browser-original-body-responsive.mjs');
const {DatabaseSync}=await import('node:sqlite');
const require=createRequire(path.join(admission.playwrightRoot,'package.json')),playwright=require(admission.playwrightRoot);
const {output}=admission;await fs.mkdir(output);await fs.mkdir(path.join(output,'data'));await fs.mkdir(path.join(output,'temporary'));await fs.mkdir(path.join(output,'chrome-profile'));await fs.mkdir(path.join(output,'downloads'));
const controller=new AbortController(),records=[],owned=[],siteRequests=[],failures=[];let site,siteOrigin,app,chrome,browser,api,job,browserResult,baseline,after,sqliteEvidence,profileResult,membershipResult,originalBodyResult,complete=false,siteClosed=false,completionClock=null;
const journal=(event)=>{const data={kind:'findings_native_ownership_v1',at:new Date().toISOString(),...event};if(process.connected)process.send(data);return data;};
journal({event:'directory',directory:output,fixture_directory_retained:true});
const abort=reason=>{if(!controller.signal.aborted)controller.abort(Error(String(reason)));};
const onMessage=message=>{if(message?.kind==='findings_native_abort_v1')abort('Outer owner requested abort');};const onSigint=()=>abort('SIGINT'),onSigterm=()=>abort('SIGTERM');process.on('message',onMessage);process.on('SIGINT',onSigint);process.on('SIGTERM',onSigterm);
const deadline=setTimeout(()=>abort('Native browser driver 600s cooperative deadline'),EXECUTION_DEADLINE_MS);
function checkpoint(){controller.signal.throwIfAborted();for(const item of owned)if(item.spawn_error||item.log_error)throw Error(item.spawn_error??item.log_error);}
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function until(fn,label,timeout=120000){const stop=Date.now()+timeout;while(Date.now()<stop){checkpoint();const result=await fn();if(result)return result;await delay(100);}throw Error(label+' timed out');}
function record(name,evidence){records.push({name,result:'PASS_ACTUAL_NATIVE',evidence});}
function child(role,executable,args){
  const stdout=createWriteStream(path.join(output,role+'.stdout.log'),{flags:'wx'}),stderr=createWriteStream(path.join(output,role+'.stderr.log'),{flags:'wx'});
  const processHandle=spawn(executable,args,{cwd:output,windowsHide:true,stdio:['ignore','pipe','pipe'],env:{...process.env,TEMP:path.join(output,'temporary'),TMP:path.join(output,'temporary')}});
  const item={role,pid:processHandle.pid??null,actual_close_event:false,spawn_error:null,code:null,signal:null,stdout_closed:false,stderr_closed:false,handle:processHandle,stdout_tail:''};owned.push(item);journal({event:'spawn',role,pid:item.pid});
    processHandle.stdout.on('data',chunk=>{item.stdout_tail=(item.stdout_tail+chunk.toString('utf8')).slice(-16384);});
  for(const [name,readable,writer] of [['stdout',processHandle.stdout,stdout],['stderr',processHandle.stderr,stderr]]){
    let observed=0;writer.on('error',error=>{item.log_error=name+': '+error.message;readable.unpipe(writer);readable.resume();abort(item.log_error);});
    readable.on('data',chunk=>{observed+=chunk.length;if(observed>16*1024*1024&&!item.log_error){item.log_error=name+' log exceeded16MiB';readable.unpipe(writer);writer.end();readable.resume();abort(item.log_error);}});readable.pipe(writer);
  }
  const streams=Promise.all([new Promise(resolve=>stdout.once('close',()=>{item.stdout_closed=true;item.stdout_close_elapsed_nanoseconds=closeElapsed();resolve();})),new Promise(resolve=>stderr.once('close',()=>{item.stderr_closed=true;item.stderr_close_elapsed_nanoseconds=closeElapsed();resolve();}))]);
  processHandle.once('error',error=>{item.spawn_error=error.message;});
  item.closed=new Promise(resolve=>processHandle.once('close',(code,signal)=>{Object.assign(item,{actual_close_event:true,code,signal,process_close_elapsed_nanoseconds:closeElapsed()});journal({event:'close',role,pid:item.pid,actual_close_event:true,code,signal});resolve();}));
  item.drained=item.closed.then(()=>streams);return item;
}
async function closeChild(item){
  if(!item)return;
  if(!item.actual_close_event){item.termination_requested=true;item.handle.kill();}
  const timeout=Symbol('timeout');let timer;const closed=await Promise.race([item.drained,new Promise(resolve=>{timer=setTimeout(()=>resolve(timeout),30000);})]);clearTimeout(timer);if(closed===timeout)throw Error('Actual child close/pipe drain unresolved for '+item.role+'; output retained');
}
async function httpRead(route,body,options={}){
  checkpoint();assert(route.startsWith('/')&&!route.startsWith('//'));
  const response=await fetch(api+route,{method:body===undefined?'GET':'POST',headers:body===undefined?{}:{'Content-Type':'application/json'},...(body===undefined?{}:{body:JSON.stringify(body)}),signal:AbortSignal.any([controller.signal,AbortSignal.timeout(150000)])});
  const limit=options.csv?64*1024*1024:8*1024*1024,length=response.headers.get('content-length');if(length!==null)assert(/^\d+$/.test(length)&&Number(length)<=limit);
  const chunks=[];let count=0;assert(response.body);for await(const chunk of response.body){checkpoint();count+=chunk.length;assert(count<=limit);chunks.push(chunk);}const data=Buffer.concat(chunks,count);if(length!==null)assert.equal(Number(length),count,'Actual complete EOF/declared length');const headers=Object.fromEntries(response.headers);
  const result={status:response.status,headers,bytes:data};if(!options.csv){assert.match(headers['content-type'],/application\/json|text\/html|text\/javascript|text\/css/);if(headers['content-type'].includes('application/json'))result.json=JSON.parse(data.toString('utf8'));}
  if(!options.allowError)assert.equal(response.status,200,data.toString('utf8').slice(0,1000));return result;
}
async function durableSnapshot(){
  const rows=[];let total=0;
  async function walk(dir,relative=''){for(const entry of (await fs.readdir(dir,{withFileTypes:true})).sort((a,b)=>a.name.localeCompare(b.name))){assert(!entry.isSymbolicLink());const file=path.join(dir,entry.name),name=(relative?relative+'/':'')+entry.name;if(entry.isDirectory()){await walk(file,name);continue;}assert(entry.isFile());if(name.endsWith('.sqlite-shm'))continue;const stat=await fs.stat(file);total+=stat.size;assert(total<=256*1024*1024&&rows.length<1024,'Fixture durable snapshot budget');const data=await fs.readFile(file);assert.equal(data.length,stat.size);rows.push({path:name,bytes:data.length,sha256:sha(data)});}}
  await walk(path.join(output,'data'));return rows;
}
try{
  const originalTinyBytes=Buffer.from('\uFEFF<?xml version="1.0" encoding="UTF-8"?>\r\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><!-- Original Ä/東京 --></urlset>\r\n');
  // Only this owned bounded website is crawled: baseline before, two explicit source-profile probes and three membership probes after the readonly interval.
  site=http.createServer((request,response)=>{
    const url=new URL(request.url,'http://127.0.0.1');siteRequests.push({method:request.method,path:url.pathname});
    const match=/^\/p(\d+)$/.exec(url.pathname);let body,status=200,type;
    if(match&&+match[1]<405){const n=+match[1];type='text/html; charset=utf-8';body=`<!doctype html><html lang="en"><head><title>Native saved page ${n}</title>${n===0?'<meta name="robots" content="noindex,follow">':''}</head><body><h1>Native page ${n}</h1><p>Actual fixture ${n}.</p>${n===2?'<a href="/p1">Actual incoming anchor to p1</a>':''}</body></html>`;}
    else if(url.pathname==='/a.xml'||url.pathname==='/b.xml'){type='application/xml; charset=utf-8';body=`<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>${siteOrigin}/p0</loc></url><url><loc>${siteOrigin}/p1</loc></url></urlset>`;}
    else if(url.pathname==='/body-original-tiny.xml'){type='application/xml; charset=utf-8';body=originalTinyBytes;}
    else{status=404;type='text/plain';body='Unexpected fixture path';}
    response.writeHead(status,{'Content-Type':type,'Content-Length':Buffer.byteLength(body),'Connection':'close'});response.end(body);
  });
  await new Promise((resolve,reject)=>{site.once('error',reject);site.listen(0,'127.0.0.1',resolve);});const sitePort=site.address().port;assert(![8787,8790].includes(sitePort));siteOrigin='http://127.0.0.1:'+sitePort;journal({event:'port',role:'fixture',port:sitePort});
  app=child('app',admission.binary,['serve','--listen','127.0.0.1:0','--data-dir',path.join(output,'data')]);
  api=await until(async()=>{assert(!app.actual_close_event,'App closed during startup');const match=/Open SEO Spider: (http:\/\/127\.0\.0\.1:(\d+))/.exec(app.stdout_tail);if(!match)return false;assert(![8787,8790].includes(+match[2]));return match[1];},'Owned App listener');journal({event:'port',role:'app',port:Number(new URL(api).port)});await until(async()=>{try{return (await httpRead('/healthz')).json.ok===true;}catch(error){checkpoint();return false;}},'Owned App health');
  const config={seed_urls:Array.from({length:405},(_,i)=>siteOrigin+'/p'+i),sitemap_urls:[siteOrigin+'/a.xml',siteOrigin+'/b.xml'],max_pages:1000,max_sitemap_documents:10,concurrency:1,delay_ms:0,request_timeout_secs:10,max_body_bytes:1048576,crawl_links:false,discover_sitemaps:false,obey_robots:false,check_external:false};
  const created=await httpRead('/api/crawls',config);assert.match(created.json.id,/^[a-f0-9-]{36}$/);job=await until(async()=>{const current=(await httpRead('/api/crawls/'+created.json.id)).json;assert(!['failed','cancelled'].includes(current.status),current.error??current.status);return current.status==='completed'?current:false;},'Actual 405-page crawl completion',300000);assert.equal(job.stats.fetched,405);assert.equal(job.stats.failed,0);assert.equal(job.stats.queued,0);assert.equal(job.stats.in_flight,0);assert.equal(job.stats.discovery_pending,0);assert.equal(job.generation,1);assert.equal(siteRequests.length,407);assert(siteRequests.every(x=>x.method==='GET'&&(/^\/p\d+$/.test(x.path)||['/a.xml','/b.xml'].includes(x.path))));
  await fs.writeFile(path.join(output,'crawl-config.json'),JSON.stringify(config,null,2)+'\n',{flag:'wx'});await fs.writeFile(path.join(output,'completed-job.json'),JSON.stringify(job,null,2)+'\n',{flag:'wx'});
  // Independent direct readonly SQL corroborates actual captured rows; no insertion/update/restoration fixture.
  const database=path.join(output,'data',job.id+'.sqlite');const header=Buffer.alloc(16),file=await fs.open(database,'r');try{await file.read(header,0,16,0);}finally{await file.close();}assert.equal(header.toString('binary'),'SQLite format 3\u0000');
  const sql=new DatabaseSync(database,{readOnly:true});try{
    const count=table=>Number(sql.prepare('SELECT count(*) AS n FROM '+table).get().n);const savedConfig=sql.prepare("SELECT value FROM metadata WHERE key='config'").get().value;const savedRun=JSON.parse(sql.prepare("SELECT value FROM metadata WHERE key='run_state'").get().value);
    assert.equal(JSON.parse(savedConfig).seed_urls.length,405);assert.equal(savedRun.generation,1);assert.equal(count('pages'),405);assert.equal(count('sitemap_documents'),2);assert.equal(count('sitemap_entries'),4);assert.equal(count('sitemap_locations'),4);assert.equal(count('sitemap_source_measurements'),2);assert.equal(count('sitemap_source_measurement_commits'),2);assert.equal(Number(sql.prepare('SELECT count(*) AS n FROM discovery_frontier WHERE done=0').get().n),0);
    sqliteEvidence={database,pages:405,documents:2,entries:4,locations:4,measurements:2,exact_associations:2,generation:savedRun.generation,actual_saved_config_sha256:sha(Buffer.from(savedConfig)),readonly:true};
  }finally{sql.close();}
  record('actual_public_crawl_sqlite_capture',sqliteEvidence);const siteBefore=siteRequests.length;baseline=await durableSnapshot();
  // Spawn Chrome directly: keep the actual ChildProcess before any CDP handshake can fail.
  chrome=child('chrome',admission.chrome,['--headless=new','--remote-debugging-address=127.0.0.1','--remote-debugging-port=0','--user-data-dir='+path.join(output,'chrome-profile'),'--no-first-run','--no-default-browser-check','--disable-background-networking','--disable-sync','--disable-extensions','--disable-component-update','--no-proxy-server','about:blank']);
  const devtools=await until(async()=>{assert(!chrome.actual_close_event,'Chrome closed during startup');try{const text=await fs.readFile(path.join(output,'chrome-profile','DevToolsActivePort'),'utf8');const [port,endpoint]=text.trim().split(/\r?\n/);assert(/^\d+$/.test(port)&&!['8787','8790'].includes(port));assert(/^\/devtools\/browser\/[A-Za-z0-9-]+$/.test(endpoint));return 'ws://127.0.0.1:'+port+endpoint;}catch(error){if(error.code==='ENOENT')return false;throw error;}},'Owned Chrome DevTools');journal({event:'port',role:'chrome',port:Number(new URL(devtools).port)});
  browser=await playwright.chromium.connectOverCDP(devtools,{timeout:30000});
  const assets={'/':'web/index.html','/app.js':'web/app.js','/styles.css':'web/styles.css','/list-import.js':'web/list-import.js','/saved-comparison.js':'web/saved-comparison.js'},expectedAssets={};for(const [route,sourcePath] of Object.entries(assets)){const actual=admission.sourceManifest.files.find(x=>x.path===sourcePath);assert(actual);expectedAssets[route]=actual.sha256;}
  const env={browser,api,job,siteOrigin,output,post:httpRead,read:httpRead,checkpoint,record,records,quiet:async()=>{await delay(150);checkpoint();},expectedAssets,configSha:sqliteEvidence.actual_saved_config_sha256,expectedSeedUrls:config.seed_urls};browserResult=await runNativeFindingsBrowser(env);browserResult.chrome_version=browser.version();assert.equal(env.contextClosed,true);assert.equal(siteRequests.length,siteBefore,'All findings/API/UI/export reads make zero new fixture website requests');
  after=await durableSnapshot();assert.deepEqual(after,baseline,'Every durable saved crawl/report/WAL byte remains unchanged');record('actual_readonly_interval',{website_requests_before:siteBefore,website_requests_after:siteRequests.length,new_website_requests:0,durable_files:baseline.length,durable_bytes:baseline.reduce((n,x)=>n+x.bytes,0),excluded_volatile_coordination:['*.sqlite-shm'],durable_file_sha256_unchanged:true});
  // Deliberate UI-started probe crawls occur AFTER the unchanged-read witness.
  const profileWebsiteBefore=siteRequests.length;
  const verifySavedConfig=async(id,profile)=>{
    const sql=new DatabaseSync(path.join(output,'data',id+'.sqlite'),{readOnly:true});try{
      const raw=sql.prepare("SELECT value FROM metadata WHERE key='config'").get().value,actual=JSON.parse(raw),run=JSON.parse(sql.prepare("SELECT value FROM metadata WHERE key='run_state'").get().value);
      assert.equal(actual.max_body_bytes,2097152);assert.deepEqual(actual.seed_urls,[siteOrigin+'/p404']);assert.equal(run.generation,1);
      if(profile===null)assert.equal(Object.hasOwn(actual,'sitemap_source_processing'),false,'Disabled optional native profile stays absent in saved serde config');else assert.deepEqual(actual.sitemap_source_processing,profile);
      return {id,readonly:true,saved_config_sha256:sha(Buffer.from(raw)),page_max_body_bytes:actual.max_body_bytes,sitemap_source_processing:actual.sitemap_source_processing??null,source_profile_field_present:Object.hasOwn(actual,'sitemap_source_processing'),generation:run.generation};
    }finally{sql.close();}
  };
  profileResult=await runSitemapSourceProfileForm({browser,api,siteOrigin,output,read:httpRead,checkpoint,quiet:env.quiet,record,verifySavedConfig});
  assert.equal(profileResult.context_closed,true);assert.equal(profileResult.authored_native_groups,5);assert.equal(profileResult.actual_profile_crawls,2);assert.equal(records.length,25);
  profileResult.intentional_website_requests=siteRequests.slice(profileWebsiteBefore);assert(siteRequests.length>profileWebsiteBefore,'Actual UI probes performed their own tiny crawl requests');
  const membershipWebsiteBefore=siteRequests.length;
  membershipResult=await runMembershipCapacityForm({browser,api,siteOrigin,output,read:httpRead,checkpoint,record});
  assert.equal(membershipResult.result,'PASS_ACTUAL_MEMBERSHIP_FORM');assert.equal(membershipResult.probes.length,3);assert.deepEqual(membershipResult.blocked,[]);assert.deepEqual(membershipResult.errors,[]);assert.equal(records.length,26);
  membershipResult.intentional_website_requests=siteRequests.slice(membershipWebsiteBefore);assert.deepEqual(membershipResult.intentional_website_requests,Array.from({length:3},()=>({method:'GET',path:'/p404'})),'Only three owned tiny membership crawls; no implicit sitemap/robots/external requests');
  const originalBodyWebsiteBefore=siteRequests.length;
  originalBodyResult=await runOriginalBodyFormResponsive({browser,api,sourceUrl:siteOrigin+'/body-original-tiny.xml',expectedBytes:originalTinyBytes,output,read:httpRead,checkpoint,record,sourceRequests:()=>siteRequests.map(r=>({...r}))});
  assert.equal(originalBodyResult.result,'PASS_ACTUAL_ORIGINAL_SOURCE_UI_1440_AND_320');assert.equal(originalBodyResult.probes.length,3);assert.deepEqual(originalBodyResult.blocked,[]);assert.deepEqual(originalBodyResult.errors,[]);assert.equal(records.length,27);assert.deepEqual(siteRequests.slice(originalBodyWebsiteBefore),Array.from({length:3},()=>({method:'GET',path:'/body-original-tiny.xml'})),'Only three deliberate owned BODY UI source crawls; actual backend read intervals remain unchanged');
  complete=true;
}catch(error){failures.push({phase:'native_browser',message:error.message,stack:error.stack});}
finally{
  let browserCloseError;
  try{if(browser)await browser.close({reason:'Owned native browser gate complete'});}catch(error){browserCloseError=error.message;}
  // Owned process handles, never a later public PID lookup. No cleanup directory deletion.
  for(const item of [chrome,app])try{await closeChild(item);}catch(error){failures.push({phase:'actual_child_close',role:item?.role,message:error.message});}
  if(site)try{site.closeAllConnections();await new Promise((resolve,reject)=>site.close(error=>error?reject(error):resolve()));siteClosed=true;}catch(error){failures.push({phase:'fixture_close',message:error.message});}
  // Freeze completion after held process closes AND actual writer-pipe drain.
  // A due-but-undelivered timer cannot make a >=600s completion pass.
  const closedClocks=owned.flatMap(item=>[item.process_close_elapsed_nanoseconds,item.stdout_close_elapsed_nanoseconds,item.stderr_close_elapsed_nanoseconds]);
  const allExpectedDrained=owned.length===2&&owned.every(item=>item.actual_close_event&&item.stdout_closed&&item.stderr_closed)&&closedClocks.every(value=>typeof value==='string');
  const lastActualCloseNs=closedClocks.filter(value=>typeof value==='string').reduce((latest,value)=>BigInt(value)>latest?BigInt(value):latest,0n),drainObservedNs=process.hrtime.bigint()-executionStartedNs;
  completionClock=Object.freeze({profile:'monotonic_actual_process_and_writer_drain_v1',deadline_nanoseconds:String(EXECUTION_DEADLINE_NS),last_actual_process_or_pipe_close_elapsed_nanoseconds:String(lastActualCloseNs),all_drains_observed_elapsed_nanoseconds:String(drainObservedNs),all_expected_processes_and_pipes_closed:allExpectedDrained,fixture_closed:siteClosed,within_deadline:allExpectedDrained&&siteClosed&&lastActualCloseNs<EXECUTION_DEADLINE_NS&&drainObservedNs<EXECUTION_DEADLINE_NS});
  clearTimeout(deadline);
  if(!completionClock.within_deadline)failures.push({phase:'monotonic_completion_deadline',message:'Actual process/writer drain incomplete or observed at/after600s',clock:completionClock});
  process.off('message',onMessage);process.off('SIGINT',onSigint);process.off('SIGTERM',onSigterm);
  try{admission.verify();assert.equal(sha(await fs.readFile(admission.binary)),admission.binarySha);assert.equal(sha(await fs.readFile(admission.chrome)),admission.chromeSha);for(const item of admission.sourceManifest.files)for(const base of [admission.sourceRoot,admission.compilerRoot]){const data=await fs.readFile(path.join(base,item.path));assert.equal(data.length,item.bytes);assert.equal(sha(data),item.sha256);}}catch(error){failures.push({phase:'final_input_identity',message:error.message});}
  if(controller.signal.aborted)failures.push({phase:'cooperative_abort',message:controller.signal.reason?.message??'Aborted'});
  if(browserCloseError)failures.push({phase:'browser_close',message:browserCloseError});
  for(const item of owned)if(item.spawn_error||item.log_error)failures.push({phase:'owned_process_io',role:item.role,message:item.spawn_error??item.log_error});
  const ownedProcesses=owned.map(({handle,closed,drained,stdout_tail,...item})=>item);const allClosed=ownedProcesses.every(x=>x.actual_close_event&&x.stdout_closed&&x.stderr_closed);if(!allClosed)failures.push({phase:'owned_lifecycle',message:'At least one actual close/pipe drain remains unobserved'});
  const proof={runtime_witness:admission.runtimeWitness,schema_version:1,result:complete&&allClosed&&siteClosed&&completionClock?.within_deadline===true&&failures.length===0?'PASS_COMBINED_V12_ACTUAL_NATIVE_FINDINGS_BROWSER':'FAIL_COMBINED_V12_ACTUAL_NATIVE_FINDINGS_BROWSER',recorded_at:new Date().toISOString(),execution_adapter:'separate_actual_browser_combined_v12_saved_renderer_none_ui_profiles_membership_405_saved_pages',source_publication:admission.publicationLineage,source_lineage:admission.sourceLineage,bindings_sha256:admission.bindingsSha,actual_close_clock:completionClock,source_manifest_sha256:admission.sourceSha,preparation_manifest_sha256:admission.preparationSha,suite_sha256:admission.suiteSha,build_proof_sha256:admission.buildSha,candidate_binary_sha256:admission.binarySha,chrome_sha256:admission.chromeSha,release_sha256:admission.releaseSha,api,site_origin:siteOrigin,job_id:job?.id,sqlite:sqliteEvidence,owned_processes:ownedProcesses,fixture_closed:siteClosed,fixture_directory_retained:true,browser:browserResult,source_profile_form:profileResult,membership_capacity_form:membershipResult,original_body_form:originalBodyResult,groups:records,failures,qualifications:['Actual 405-page source-limited native UI flow only; no full Screaming Frog parity claim','Small source threshold negative controls only; physical positive limits and Unknown-source matrix remain separate open gates','Durable files exclude SQLite shared-memory lock coordination; no byte-change claim for SHM','Actual top-level App and Chrome ChildProcess close+pipe drain observed; no OS-wide descendant enumeration or hard shutdown latency proof','13 mocked UI groups are separate and are not replaced by this gate','Source-profile and membership-form probes retain their five original tiny UI-started crawls; BODY group27 adds three deliberate tiny sources and1440/320 controls with backend read-phase counters. Full64MiB original public EOF/cold reopen remains a separate mandatory native gate, not this tiny browser flow']};
  await fs.writeFile(path.join(output,'durable-before.json'),JSON.stringify(baseline??null,null,2)+'\n',{flag:'wx'});await fs.writeFile(path.join(output,'durable-after.json'),JSON.stringify(after??null,null,2)+'\n',{flag:'wx'});await fs.writeFile(path.join(output,'site-requests.json'),JSON.stringify(siteRequests,null,2)+'\n',{flag:'wx'});await fs.writeFile(path.join(output,'proof.json'),JSON.stringify(proof,null,2)+'\n',{flag:'wx'});journal({event:'retained',directory:output,fixture_directory_retained:true,all_actual_children_closed:allClosed});
  console.log(JSON.stringify({result:proof.result,output,groups:records.length,owned_processes:ownedProcesses,failures}));if(proof.result!=='PASS_COMBINED_V12_ACTUAL_NATIVE_FINDINGS_BROWSER')process.exitCode=1;
}