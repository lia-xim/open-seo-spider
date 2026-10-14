// Native membership acceptance: owned processes, data and browser; no shared runtime operations.
import assert from 'node:assert/strict';
import http from 'node:http';
import path from 'node:path';
import {createReadStream} from 'node:fs';
import * as fs from 'node:fs/promises';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {runNativeTargetBrowser} from './sitemap-target-browser-v1.mjs';
import {assertTargetPage,assertSavedFinding,assertVisibleLocationFinding} from './sitemap-target-assertions-v1.mjs';

const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const HEX=/^[a-f0-9]{64}$/,UUID=/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i,forbiddenPorts=new Set([8787,8790]);
const budgets={run:240000,start:20000,case:30000,request:10000,stop:5000,drain:10000,quiet:250,api:8388608,csv:67108864,requests:256,log:16777216};
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function hashFile(filename){const hash=createHash('sha256');for await(const bytes of createReadStream(filename))hash.update(bytes);return hash.digest('hex');}
async function bounded(promise,ms,label){let timer;try{return await Promise.race([promise,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Timeout '+label)),ms);})]);}finally{clearTimeout(timer);}}
async function writeNew(filename,bytes){await fs.writeFile(filename,bytes,{flag:'wx'});}
async function jsonNew(filename,value){await writeNew(filename,JSON.stringify(value,null,2)+'\n');}
function parseArgs(){const allowed=new Set(['binary','binary-sha256','source-root','source-manifest','source-manifest-sha256','runtime-root','out','chrome']),args={};for(let index=2;index<process.argv.length;index+=2){const name=process.argv[index];assert(name?.startsWith('--') && allowed.has(name.slice(2)) && process.argv[index+1] && args[name.slice(2)]===undefined,'Unknown/duplicate/missing harness argument');args[name.slice(2)]=process.argv[index+1];}for(const name of ['binary','binary-sha256','source-root','source-manifest','source-manifest-sha256','runtime-root','out'])assert(args[name],'Missing --'+name);return args;}
function csvRows(text){const records=[],fields=[];let field='',quoted=false;for(let index=0;index<text.length;index++){const c=text[index];if(quoted){if(c==='"' && text[index+1]==='"'){field+='"';index++;}else if(c==='"')quoted=false;else field+=c;}else if(c==='"'){assert.equal(field,'');quoted=true;}else if(c===','){fields.push(field);field='';}else if(c==='\r' || c==='\n'){if(c==='\r' && text[index+1]==='\n')index++;fields.push(field);records.push(fields.splice(0));field='';}else field+=c;}assert.equal(quoted,false);if(field || fields.length){fields.push(field);records.push(fields);}if(records[0]?.[0]?.startsWith('\uFEFF'))records[0][0]=records[0][0].slice(1);return records;}
function member(row){return ['root','location'].includes(row.kind)?row.row.admission.membership_url:null;}
function sourceId(row){return row.kind==='root'?row.row.source_observation_id:row.row.observation_id;}
function coverage(value){
  const derived=value.coverage.derived;
  assert(derived && derived.engine==='open-seo-spider/declared-sitemap-source-coverage-v1');
  assert(['complete','known_empty','unknown'].includes(derived.completion));
  assert(['current_declared_sources','inherited_historical_declared_sources','unavailable'].includes(derived.scope));
  assert.equal(derived.current_valid_original_location_negative_permitted, derived.scope==='current_declared_sources' && derived.completion!=='unknown');
  assert.equal(derived.historical_valid_original_location_negative_permitted, derived.scope==='inherited_historical_declared_sources' && derived.completion!=='unknown');
  assert.equal(value.lastmod_assessment_engine,'sitemap-lastmod/xsd-1.0-second-edition+w3c-note-1998/v1');
  if(value.catalog_available){assert(Array.isArray(value.lastmod_assessments));assert.deepEqual(value.lastmod_assessments.map(x=>x.entry_id), value.rows.filter(x=>x.kind==='entry').map(x=>x.row.entry_id));}
  else assert.equal(value.lastmod_assessments,null);
  for(const assessment of value.lastmod_assessments||[]){assert.equal(assessment.freshness_assessed,false);assert.equal(assessment.http_status_assessed,false);}
assert.equal(value.website_requests,0);assert.equal(value.coverage.declared_source_cohort_complete,false);assert.equal(value.coverage.negative_membership,'unknown');assert.equal(value.coverage.fetch_status_join,'not_performed');assert.equal(value.snapshot_scope,'validated_sqlite_read_snapshot');}

const args=parseArgs();
for(const key of ['binary','source-root','source-manifest','runtime-root','out'])assert(path.isAbsolute(args[key]),'Absolute path required for '+key);
assert.match(args['binary-sha256'],HEX);assert.match(args['source-manifest-sha256'],HEX);
const binary=await fs.realpath(args.binary),sourceRoot=await fs.realpath(args['source-root']),runtimeRoot=await fs.realpath(args['runtime-root']);
assert((await fs.lstat(binary)).isFile());assert.equal(await hashFile(binary),args['binary-sha256'],'Explicit Root executable hash before any launch');
const sourceBytes=await fs.readFile(args['source-manifest']);assert.equal(sha(sourceBytes),args['source-manifest-sha256']);
const sourceManifest=JSON.parse(sourceBytes);assert(Array.isArray(sourceManifest.files));
const sourceFiles=new Map();
async function verifySources(){for(const info of sourceManifest.files){assert(info && typeof info.path==='string' && !path.isAbsolute(info.path));const filename=path.resolve(sourceRoot,info.path),relative=path.relative(sourceRoot,filename);assert(relative && !relative.startsWith('..'+path.sep) && relative!=='..' && !path.isAbsolute(relative));assert.match(info.sha256,HEX);const stat=await fs.lstat(filename);assert(stat.isFile() && !stat.isSymbolicLink());assert.equal(stat.size,info.bytes);assert.equal(await hashFile(filename),info.sha256,'Frozen source '+info.path);sourceFiles.set(info.path,info);}}
await verifySources();
const assetRoutes={'/':'web/index.html','/app.js':'web/app.js','/styles.css':'web/styles.css','/list-import.js':'web/list-import.js','/saved-comparison.js':'web/saved-comparison.js'};for(const file of Object.values(assetRoutes))assert(sourceFiles.has(file),'Source closure lacks '+file);
const harnessShaBefore=await hashFile(fileURLToPath(import.meta.url));
const browserModuleShaBefore=await hashFile(fileURLToPath(new URL('./sitemap-target-browser-v1.mjs',import.meta.url)));
const assertionsModuleShaBefore=await hashFile(fileURLToPath(new URL('./sitemap-target-assertions-v1.mjs',import.meta.url)));
const runtimeRequire=createRequire(path.join(runtimeRoot,'scripts/package.json'));
let sdkRoot=path.dirname(runtimeRequire.resolve('@modelcontextprotocol/client')),sdkPackage;
while(true){try{const candidate=JSON.parse(await fs.readFile(path.join(sdkRoot,'package.json'),'utf8'));if(candidate.name==='@modelcontextprotocol/client'){sdkPackage=candidate;break;}}catch(error){if(error.code!=='ENOENT')throw error;}const parent=path.dirname(sdkRoot);assert.notEqual(parent,sdkRoot);sdkRoot=parent;}
assert.equal(sdkPackage.version,'2.3.1');
function sdkEntry(key){const entry=sdkPackage.exports[key]?.import?.default;assert.equal(typeof entry,'string');assert(entry.startsWith('./'));const filename=path.resolve(sdkRoot,entry);assert(filename.startsWith(sdkRoot+path.sep));return pathToFileURL(filename).href;}
const {Client,StreamableHTTPClientTransport}=await import(sdkEntry('.')),{StdioClientTransport}=await import(sdkEntry('./stdio'));
const chrome=args.chrome || 'C:/Program Files/Google/Chrome/Application/chrome.exe';assert(path.isAbsolute(chrome));assert((await fs.stat(chrome)).isFile());
const output=path.resolve(args.out);try{await fs.lstat(output);throw Error('Refusing existing output directory');}catch(error){if(error.code!=='ENOENT')throw error;}
await fs.realpath(path.dirname(output));await fs.mkdir(output);const data=path.join(output,'data');await fs.mkdir(data);await fs.mkdir(path.join(output,'api'));await fs.mkdir(path.join(output,'sdk'));await fs.mkdir(path.join(output,'browser'));await fs.mkdir(path.join(output,'downloads'));
await writeNew(path.join(output,'source-manifest.json'),sourceBytes);
await jsonNew(path.join(output,'inputs.json'),{scope:'Root supplied native executable; source-to-binary compilation provenance is declared, not inferred',binary,binary_sha256:args['binary-sha256'],source_root:sourceRoot,source_manifest:args['source-manifest'],source_manifest_sha256:args['source-manifest-sha256'],source_files:sourceManifest.files.length,harness_sha256:await hashFile(fileURLToPath(import.meta.url)),frozen_handoff_sha256:null,runtime_root:runtimeRoot,sdk_version:sdkPackage.version,sdk_package_sha256:await hashFile(path.join(sdkRoot,'package.json')),chrome,data,output});

const started=Date.now(),deadline=started+budgets.run,abort=new AbortController(),checks=[],failures=[],apiRecords=[],sdkRecords=[],sdkWire=[],intervals=[],uiRequests=[],uiResponses=[],external=[],pageErrors=[],oracleControls=[],stdioProducers=[],stdioRegistry=new WeakMap(),sdkProducers=[],sdkRegistry=new WeakMap();
const signalHandlers=['SIGINT','SIGTERM'].map(name=>{const handler=()=>abort.abort(Error('Native acceptance interrupted by '+name));process.once(name,handler);return [name,handler];});
const fixtures=[],routes=new Map(),jobs={};let child,api,appRuns=[],browser,page,plannedStop=false,spawnError,apiSerial=0,sdkSerial=0,browserSerial=0;
let appClose,appClosed=true,bindFailure,siteOrigin,offlineOrigin,oraclePhase=false;
const stamp=()=>Date.now()-started;
function alive(){if(abort.signal.aborted)throw abort.signal.reason;if(Date.now()>=deadline)throw Error('Native harness deadline exceeded');if(spawnError)throw spawnError;if(bindFailure)throw bindFailure;if(child && !plannedStop && (child.exitCode!==null || child.signalCode!==null))throw Error('Owned App exited unexpectedly');}
async function until(fn,label,timeout=budgets.case){const end=Math.min(deadline,Date.now()+timeout);while(Date.now()<end){alive();const result=await fn();if(result)return result;await delay(25);}throw Error('Timeout '+label);}
const check=(name,evidence={})=>{checks.push({name,...evidence});console.log('PASS '+name);};
async function fixture(name,handler){const hits=[],active=new Set(),sockets=new Set();let ordinal=0,lastActivity=Date.now(),overflow=false,errors=[];const server=http.createServer((req,res)=>{ordinal++;lastActivity=Date.now();if(ordinal>budgets.requests){overflow=true;res.writeHead(503);res.end('Witness overflow');return;}const row={ordinal,method:req.method,path:req.url,started_ms:stamp(),finished_ms:null,response_state:'active'};hits.push(row);active.add(row);const settle=state=>{if(active.delete(row)){row.finished_ms=stamp();row.response_state=state;lastActivity=Date.now();}};res.once('finish',()=>settle('finished'));res.once('close',()=>settle('aborted'));req.once('error',error=>row.request_error=error.code || error.message);req.resume();handler(req,res,row);});server.on('error',error=>errors.push(error.message));server.on('connection',socket=>{sockets.add(socket);lastActivity=Date.now();socket.on('error',()=>{});socket.once('close',()=>{sockets.delete(socket);lastActivity=Date.now();});});await bounded(new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);}),budgets.start,'fixture bind');const address=server.address();assert.equal(address.address,'127.0.0.1');assert(!forbiddenPorts.has(address.port));const value={name,server,hits,active,sockets,errors,get ordinal(){return ordinal;},get lastActivity(){return lastActivity;},get overflow(){return overflow;},origin:'http://127.0.0.1:'+address.port,drained:false};fixtures.push(value);return value;}
async function launch(){assert(appClosed);assert.equal(await hashFile(binary),args['binary-sha256']);spawnError=null;bindFailure=null;plannedStop=false;api=null;appClosed=false;const run={ordinal:appRuns.length+1,args:['serve','--listen','127.0.0.1:0','--data-dir',data],pid:null,bind_line:null,origin:null,closed:false,exit:null,stdout_bytes:0,stderr_bytes:0};appRuns.push(run);const stdout=await fs.open(path.join(output,`app-${run.ordinal}.stdout.log`),'wx'),stderr=await fs.open(path.join(output,`app-${run.ordinal}.stderr.log`),'wx');let pending='',stdoutWrites=Promise.resolve(),stderrWrites=Promise.resolve();child=spawn(binary,run.args,{cwd:sourceRoot,windowsHide:true,stdio:['ignore','pipe','pipe'],env:{...process.env,TMP:output,TEMP:output}});run.pid=child.pid;appClose=new Promise(resolve=>child.once('close',(code,signal)=>{run.closed=true;run.exit={code,signal,at_ms:stamp()};resolve();}));child.once('error',error=>spawnError=error);child.stdout.on('data',bytes=>{run.stdout_bytes+=bytes.length;if(run.stdout_bytes>budgets.log)bindFailure=Error('Stdout log budget exceeded');stdoutWrites=stdoutWrites.then(()=>stdout.write(bytes));pending+=bytes.toString('utf8');let end;while((end=pending.indexOf('\n'))>=0){const line=pending.slice(0,end);pending=pending.slice(end+1);if(line.startsWith('Open SEO Spider:')){const match=/^Open SEO Spider: (http:\/\/127\.0\.0\.1:([1-9][0-9]*))\r?$/.exec(line);if(!match || api || Number(match[2])>65535 || forbiddenPorts.has(Number(match[2]))){bindFailure=Error('Invalid/repeated/forbidden owned post-bind line');continue;}api=match[1];run.origin=api;run.bind_line=line;assert.notEqual(api,siteOrigin);assert.notEqual(api,offlineOrigin);}}if(pending.length>8192)bindFailure=Error('Incomplete stdout line exceeded bind limit');});child.stderr.on('data',bytes=>{run.stderr_bytes+=bytes.length;if(run.stderr_bytes>budgets.log)bindFailure=Error('Stderr log budget exceeded');stderrWrites=stderrWrites.then(()=>stderr.write(bytes));});run.closeLogs=async()=>{await Promise.all([stdoutWrites,stderrWrites]);await stdout.close();await stderr.close();const outPath=path.join(output,"app-"+run.ordinal+".stdout.log"),errPath=path.join(output,"app-"+run.ordinal+".stderr.log");assert.equal((await fs.stat(outPath)).size,run.stdout_bytes);assert.equal((await fs.stat(errPath)).size,run.stderr_bytes);run.stdout_sha256=await hashFile(outPath);run.stderr_sha256=await hashFile(errPath);};await until(()=>api,'exact App post-bind stdout',budgets.start);await until(async()=>{try{const health=await raw('/healthz');return health.status===200 && JSON.parse(health.bytes).ok===true;}catch{alive();return false;}},'owned App health',budgets.start);}
async function stop(){if(!child)return;const owned=child,run=appRuns.at(-1);if(owned.exitCode!==null || owned.signalCode!==null){await appClose;await run.closeLogs();appClosed=true;child=null;throw Error('App ended before planned close');}plannedStop=true;owned.kill('SIGTERM');try{await bounded(appClose,budgets.stop,'owned App close');}catch{owned.kill('SIGKILL');await bounded(appClose,budgets.stop,'owned App force close');run.forced=true;throw Error('Owned App required forced close');}await run.closeLogs();appClosed=true;child=null;}
async function raw(route,body,status=200,max=budgets.api){alive();assert(api && child?.pid);assert(route==='/healthz' || route==='/' || route.startsWith('/api/') || Object.hasOwn(assetRoutes,route),'Owned route required');const uri=new URL(route,api);assert.equal(uri.origin,api);assert(!forbiddenPorts.has(Number(uri.port)));const serial=++apiSerial,prefix=String(serial).padStart(4,'0'),requestBody=body===undefined?null:JSON.stringify(body);await jsonNew(path.join(output,'api',prefix+'.request.json'),{url:uri.href,method:body===undefined?'GET':'POST',body:requestBody,started_ms:stamp()});const response=await fetch(uri,{redirect:'error',signal:AbortSignal.any([abort.signal,AbortSignal.timeout(budgets.request)]),...(body===undefined?{}:{method:'POST',headers:{'content-type':'application/json'},body:requestBody})});const chunks=[];let length=0;for await(const chunk of response.body || []){length+=chunk.byteLength;if(length>max)throw Error('Native API response byte budget exceeded');chunks.push(Buffer.from(chunk));}const bytes=Buffer.concat(chunks),record={serial,url:uri.href,status:response.status,headers:Object.fromEntries(response.headers),response_sha256:sha(bytes),response_bytes:bytes.length,finished_ms:stamp()};apiRecords.push(record);await writeNew(path.join(output,'api',prefix+'.response.bin'),bytes);await jsonNew(path.join(output,'api',prefix+'.response.json'),record);assert.equal(response.status,status,uri.pathname+' '+bytes.toString('utf8').slice(0,2000));return {...record,bytes};}
async function json(route,body,status=200){return JSON.parse((await raw(route,body,status)).bytes);}
async function dataSnapshot(){const files=[];async function visit(relative=''){for(const name of (await fs.readdir(path.join(data,relative))).sort()){const item=path.join(relative,name),filename=path.resolve(data,item);assert(filename.startsWith(data+path.sep));const stat=await fs.lstat(filename);assert(!stat.isSymbolicLink());if(stat.isDirectory())await visit(item);else{assert(stat.isFile());files.push({name:item.replaceAll('\\','/'),bytes:stat.size,sha256:await hashFile(filename)});}}}await visit();return files;}
async function readInterval(name,fn){const before=fixtures.map(value=>({name:value.name,ordinal:value.ordinal})),beforePrime=await dataSnapshot(),primed=[];for(const job of Object.values(jobs)){if(!job.report)continue;await pageOf(job,{limit:'1'});primed.push(job.id);}const afterPrime=await dataSnapshot(),byName=new Map(afterPrime.map(file=>[file.name,file]));for(const file of beforePrime)assert.deepEqual(byName.get(file.name),file,'Original native file changed during read-only primer');const priorNames=new Set(beforePrime.map(file=>file.name)),created=afterPrime.filter(file=>!priorNames.has(file.name));assert(created.every(file=>file.name.endsWith('.sqlite-shm') && file.bytes===32768 || file.name.endsWith('.sqlite-wal') && file.bytes===0),'Primer created unrelated native files');const result=await fn(),after=fixtures.map(value=>({name:value.name,ordinal:value.ordinal})),finalFiles=await dataSnapshot();assert.deepEqual(after,before,'Read-only interval initiated fixture HTTP: '+name);assert.deepEqual(finalFiles,afterPrime,'Primed native data changed during read-only interval: '+name);intervals.push({name,before,after,primer:{jobs:primed,created_auxiliary_files:created,original_files_unchanged:true},primed_data_files_sha256:sha(Buffer.from(JSON.stringify(afterPrime))),all_primed_native_file_bytes_unchanged:true,at_ms:stamp()});return result;}
async function pageOf(job,query={}){return json('/api/crawls/'+job.id+'/sitemaps?'+new URLSearchParams({kind:'all',limit:'200',...query}));}
async function collect(job,query={},limit=3){let cursor=null,rows=[],token=null,snapshot=null,summary=null,guard=0;do{assert(++guard<100);const value=await pageOf(job,{...query,limit:String(limit),...(cursor?{cursor}:{})});coverage(value);assert(value.catalog_available);assert(value.rows.length<=limit);if(token===null){token=value.snapshot_token;snapshot=value.snapshot;summary=value.summary;}else{assert.equal(value.snapshot_token,token);assert.deepEqual(value.snapshot,snapshot);assert.deepEqual(value.summary,summary);}rows.push(...value.rows);assert(!value.has_more || value.rows.length && typeof value.next_cursor==='string' && value.next_cursor!==cursor);cursor=value.next_cursor;if(!value.has_more){assert.equal(cursor,null);break;}}while(true);assert(rows.every((row,index)=>index===0 || row.row.sequence>rows[index-1].row.sequence));return {rows,token,snapshot,summary};}
async function error(route,status,code){const value=await json(route,undefined,status);assert.equal(value.code,code);assert.equal(value.website_requests,0);return value;}
async function recordSdk(label,action){try{const result=await bounded(action(),budgets.request,"SDK action "+label);const record={label,result,at_ms:stamp()};sdkRecords.push(record);await jsonNew(path.join(output,'sdk',String(++sdkSerial).padStart(3,'0')+'.json'),record);return result;}catch(caught){const record={label,error:{message:caught.message,code:caught.code,data:caught.data},at_ms:stamp()};sdkRecords.push(record);await jsonNew(path.join(output,'sdk',String(++sdkSerial).padStart(3,'0')+'.json'),record);throw caught;}}
function noteWire(label,transport){const send=transport.send.bind(transport);transport.send=async(message,...options)=>{const record={label,message,at_ms:stamp(),scope:'actual SDK outgoing message before native transport delivery'};sdkWire.push(record);await jsonNew(path.join(output,'sdk','wire-'+String(sdkWire.length).padStart(3,'0')+'.json'),record);return send(message,...options);};}
function assertInventory(value){assert.equal(value.tools.length,25);const tool=value.tools.find(item=>item.name==='spider_sitemaps');assert(tool);assert.equal(tool.annotations.readOnlyHint,true);assert.equal(tool.annotations.openWorldHint,false);assert.equal(tool.inputSchema.additionalProperties,false);assert.equal(tool.inputSchema.properties.limit.maximum,200);assert.equal(tool.inputSchema.properties.page_limit.minimum,0);assert.equal(tool.inputSchema.properties.page_limit.maximum,200);assert.equal(tool.inputSchema.properties.page_cursor.maxLength,4096);return value.tools.map(item=>item.name).sort();}
async function registerSdk(label,client,transport){
  assert(!sdkRegistry.has(transport),'Owned SDK transport registered once before connect');
  const producer={label,client,transport,closed:false,close_attempts:[]};sdkProducers.push(producer);sdkRegistry.set(transport,producer);
  if(!(transport instanceof StdioClientTransport))return producer;
  let resolveClose;const record={label,pid:null,binary_sha256:args['binary-sha256'],start_attempted:false,started:false,actual_close_event:false};
  record.closed=new Promise(resolve=>resolveClose=resolve);stdioRegistry.set(transport,record);stdioProducers.push(record);
  const logPath=path.join(output,'sdk','stdio-'+(stdioProducers.length-1)+'.stderr.log'),log=await fs.open(logPath,'wx');let writes=Promise.resolve(),bytes=0,logClose;
  transport.stderr?.on('data',chunk=>{bytes+=chunk.length;if(bytes>budgets.log)abort.abort(Error('SDK stderr log budget exceeded'));writes=writes.then(()=>log.write(chunk));});
  record.closeLogs=()=>logClose??=(async()=>{await writes;await log.close();assert.equal((await fs.stat(logPath)).size,bytes);record.stderr_bytes=bytes;record.stderr_sha256=await hashFile(logPath);})();
  // Client.connect installs its public onclose handler before calling public start.
  // Observe the actual child close while preserving that installed SDK handler.
  const start=transport.start.bind(transport);
  transport.start=async(...options)=>{record.start_attempted=true;const previous=transport.onclose;transport.onclose=()=>{record.actual_close_event=true;record.closed_ms=stamp();resolveClose();previous?.();};try{const result=await start(...options);const pid=transport.pid;assert(Number.isSafeInteger(pid) && pid>0,'Actual owned SDK stdio PID required');record.pid=pid;record.started=true;return result;}finally{if(record.pid===null && Number.isSafeInteger(transport.pid) && transport.pid>0)record.pid=transport.pid;}};
  return producer;
}
async function closeSdk(client,transport,{retry=false}={}){
  const producer=sdkRegistry.get(transport);assert(producer && producer.client===client,'SDK producer must be registered before connect');if(producer.closed)return;
  const attempt={retry,started_ms:stamp(),completed:false};producer.close_attempts.push(attempt);
  try{await bounded(retry?transport.close():client.close(),budgets.stop,'owned SDK close');const record=stdioRegistry.get(transport);if(record){if(record.start_attempted)await bounded(record.closed,budgets.stop,'actual SDK stdio child close event');await record.closeLogs();assert(!record.start_attempted || record.actual_close_event);}producer.closed=true;attempt.completed=true;attempt.finished_ms=stamp();}
  catch(error){attempt.error=error.message;attempt.finished_ms=stamp();throw error;}
}
async function drain(value){assert(appClosed && child===null);assert(sdkProducers.every(producer=>producer.closed) && stdioProducers.every(record=>record.actual_close_event),'All owned SDK transports and actual stdio child close events are required before final fixture drain');const end=Date.now()+budgets.drain;while(Date.now()<end){if(value.active.size===0 && value.sockets.size===0 && Date.now()-value.lastActivity>=budgets.quiet)break;await delay(25);}assert.equal(value.active.size,0,value.name+' active requests');assert.equal(value.sockets.size,0,value.name+' connected sockets');assert(Date.now()-value.lastActivity>=budgets.quiet);assert.equal(value.errors.length,0);assert.equal(value.overflow,false);assert.equal(value.ordinal,value.hits.length);assert(value.hits.every(row=>row.response_state==='finished' && !row.request_error));await bounded(new Promise((resolve,reject)=>value.server.close(error=>error?reject(error):resolve())),budgets.stop,'clean fixture close');value.drained=true;}

const browserProducers=[],browserChecks=[];
let browserProof=null,site,offline;
const gates=new Map(),expectedRequests=new Map();
const expectedBrowserLabels=['native-asset-and-draft-binding','original-redirect-final-and-target-filters','same-document-versus-multiple-documents','observed-self-nonself-and-sitemap-only','native-original-and-lastmod-inspector','native-csv-exact-catalog','independent-raw-and-page-pagers','policy-original-and-alias','limited-pending-source-target','current-unknown-source-membership','inherited-historical-mixed-age','historical-unknown-membership','known-empty-source-catalog','imported-unavailable-catalog','responsive320-and-generator-reachability','mutation-and-website-fence-oracles'];
function expectRequest(route,count=1){expectedRequests.set(route,(expectedRequests.get(route)||0)+count);}
function sendRoute(res,route){const body=route.body;res.writeHead(route.status||200,{'content-type':route.type,'content-length':Buffer.byteLength(body),'connection':'close',...(route.headers||{})});res.end(body);}
async function waitTerminal(id,status='completed'){
  return until(async()=>{const value=await json('/api/crawls/'+id);if(value.status==='failed')throw Error(value.error);if(['completed','paused','limited','cancelled'].includes(value.status)){assert.equal(value.status,status);return value;}return null;},'actual terminal '+status+' '+id);
}
async function captureJob(name,id,done){
  const response=await raw('/api/crawls/'+id+'/project.json'),report=JSON.parse(response.bytes);
  assert(Array.isArray(report.sitemap_catalog?.rows));assert(done.checkpoint&&HEX.test(done.checkpoint.report_sha256));
  const handle=await fs.open(path.join(data,id+'.sqlite'),'r');try{const head=Buffer.alloc(16);await handle.read(head,0,16,0);assert.equal(head.toString('binary'),'SQLite format 3\u0000');}finally{await handle.close();}
  const value={id,generation:done.generation,status:done.status,checkpoint:done.checkpoint,report,report_sha256:sha(response.bytes)};jobs[name]=value;return value;
}
async function create(name,config,status='completed'){const value=await json('/api/crawls',config);assert.match(value.id,UUID);return captureJob(name,value.id,await waitTerminal(value.id,status));}
async function targetPage(job,query={}){const value=await pageOf(job,query);coverage(value);assertTargetPage(value,job);return value;}
async function savedPages(job,limit=2){
  const values=[],seen=new Set();let cursor=null,snapshot=null,rawSequence=null,context=null;
  for(let ordinal=0;ordinal<100;ordinal++){
    const value=await targetPage(job,{limit:'1',page_limit:String(limit),...(cursor?{page_cursor:cursor}:{})});
    if(snapshot===null){snapshot=value.snapshot_token;context=value.membership_analysis.context;rawSequence=value.rows[0]?.row.sequence??null;}
    else {assert.equal(value.snapshot_token,snapshot);assert.deepEqual(value.membership_analysis.context,context);assert.equal(value.rows[0]?.row.sequence??null,rawSequence);}
    for(const finding of value.membership_analysis.saved_pages){assert(!seen.has(finding.observation.requested_url));seen.add(finding.observation.requested_url);values.push(finding);}
    if(!value.membership_analysis.saved_pages_have_more){assert.equal(value.next_page_cursor,null);assert.deepEqual(values.map(row=>row.observation.requested_url),job.report.pages.map(row=>row.url));return values;}
    assert(value.next_page_cursor&&value.next_page_cursor!==cursor);cursor=value.next_page_cursor;
  }
  throw Error('Saved-page cursor failed to terminate');
}
async function recrawl(name,parent,urls){
  const bytes=await fs.readFile(path.join(data,parent.id+'.json'));
  const created=await json('/api/crawls/'+parent.id+'/recrawl',{urls,max_attempts:urls.length});assert.match(created.id,UUID);
  const result=await captureJob(name,created.id,await waitTerminal(created.id));
  assert.deepEqual(result.report.sitemap_catalog.rows,parent.report.sitemap_catalog.rows);
  assert.equal(result.report.sitemap_catalog.inherited.current_discovery_executed,false);
  assert.equal(result.report.sitemap_catalog.inherited.parent_snapshot_sha256,sha(bytes));return result;
}
async function sdkCall(client,label,argumentsValue){const value=await recordSdk(label,()=>client.callTool({name:'spider_sitemaps',arguments:argumentsValue}));assert.equal(value.isError,false);assert(value.structuredContent);return value.structuredContent;}
async function sdkPagers(kind,client,job){
  let rawCursor=null,snapshot=null,rawRows=[],guard=0;
  do{
    assert(++guard<100);const value=await sdkCall(client,kind+' raw page '+guard,{id:job.id,kind:'all',limit:3,page_limit:0,...(rawCursor?{cursor:rawCursor}:{})});coverage(value);assertTargetPage(value,job);
    if(snapshot===null)snapshot=value.snapshot_token;else assert.equal(value.snapshot_token,snapshot);
    assert.deepEqual(value.membership_analysis.saved_pages,[]);rawRows.push(...value.rows);rawCursor=value.next_cursor;
  }while(rawCursor);
  assert.deepEqual(rawRows,job.report.sitemap_catalog.rows);
  let pageCursor=null,firstRaw=null;const pageRows=[];guard=0;
  do{
    assert(++guard<100);const value=await sdkCall(client,kind+' saved page '+guard,{id:job.id,kind:'all',limit:1,page_limit:2,...(pageCursor?{page_cursor:pageCursor}:{})});coverage(value);assertTargetPage(value,job);
    assert.equal(value.snapshot_token,snapshot);if(firstRaw===null)firstRaw=value.rows;else assert.deepEqual(value.rows,firstRaw);
    pageRows.push(...value.membership_analysis.saved_pages);pageCursor=value.next_page_cursor;
  }while(pageCursor);
  assert.deepEqual(pageRows.map(value=>value.observation.requested_url),job.report.pages.map(value=>value.url));
  const first=await sdkCall(client,kind+' first independent cursors',{id:job.id,limit:1,page_limit:1});assert(first.next_cursor&&first.next_page_cursor);
  const both=await sdkCall(client,kind+' both independent cursors',{id:job.id,limit:1,cursor:first.next_cursor,page_limit:1,page_cursor:first.next_page_cursor});
  assert.equal(both.rows[0].row.sequence,job.report.sitemap_catalog.rows[1].row.sequence);assert.equal(both.membership_analysis.saved_pages[0].observation.report_page_ordinal,1);
  assert.deepEqual(both,await targetPage(job,{limit:'1',cursor:first.next_cursor,page_limit:'1',page_cursor:first.next_page_cursor}));
  for(const argumentsValue of [{id:job.id,cursor:first.next_page_cursor},{id:job.id,page_cursor:first.next_cursor}]){
    const rejected=await recordSdk(kind+' wrong cursor role',()=>client.callTool({name:'spider_sitemaps',arguments:argumentsValue}));assert.equal(rejected.isError,true);assert(rejected.content.some(block=>block.text?.includes('invalid_sitemap_cursor')));
  }
}
async function sdkStale(kind,client,job,witness){
  assert(job.generation>witness.snapshot.crawl_generation,'Actual same-job resumed generation changed');
  for(const argumentsValue of [{id:job.id,limit:1,cursor:witness.next_cursor},{id:job.id,page_limit:1,page_cursor:witness.next_page_cursor}]){
    const rejected=await recordSdk(kind+' actual stale generation cursor',()=>client.callTool({name:'spider_sitemaps',arguments:argumentsValue}));
    assert.equal(rejected.isError,true);assert.equal(rejected.structuredContent,undefined,'Stale response contains no partial findings');assert(rejected.content.some(block=>block.text?.includes('stale_sitemap_cursor')));
  }
}

try{
  site=await fixture('website',(req,res,row)=>{
    const route=routes.get(req.url);row.registered_route=Boolean(route);
    if(!route){res.writeHead(404,{'connection':'close'});res.end('Unregistered owned route');return;}
    const gate=gates.get(req.url);if(gate&&!gate.released){row.held_for_actual_resume=true;gate.requests.push(()=>sendRoute(res,route));gate.entered=true;return;}
    sendRoute(res,route);
  });siteOrigin=site.origin;
  offline=await fixture('offline_mcp_api',(req,res,row)=>{row.registered_route=false;res.writeHead(410,{'content-type':'application/json','connection':'close'});res.end('{"error":"pre-I/O probe must never reach this owned API witness"}');});offlineOrigin=offline.origin;
  const html='<html><head><title>Owned target acceptance</title></head><body>Owned page. No anchors.</body></html>';
  const xml=value=>({type:'application/xml',body:value}),leaf=value=>`<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${value}</urlset>`,loc=(name,date='')=>`<url><loc>${siteOrigin}${name}</loc>${date}</url>`;
  for(const name of ['/seed','/listed','/final','/not-found','/noindex','/blocked','/sitemap-only','/same-doc','/two-docs','/not-listed','/policy-seed','/policy-alias','/broken-seed','/broken-not-listed','/broken-member','/empty-seed','/limit-seed','/limit-held','/limit-other','/limit-extra','/ui-limit-seed','/ui-limit-a','/ui-limit-b'])routes.set(name,{type:'text/html',body:html});
  routes.set('/robots.txt',{type:'text/plain',body:'User-agent: *\nDisallow: /blocked\n'});
  routes.set('/seed',{type:'text/html',body:'<html><head><title>Owned seed</title></head><body><a href="/listed">One</a><a href="/listed" rel="nofollow">Two</a><a href="/not-listed">Other</a><a href="/final">Final</a></body></html>'});
  routes.set('/listed',{type:'text/html',body:'<html><head><title>Owned listed</title></head><body><a href="/listed">Self</a><img src="/sitemap-only" alt="Image is not an anchor"></body></html>'});
  routes.set('/redirect',{status:301,type:'text/html',headers:{location:siteOrigin+'/final'},body:''});
  routes.set('/not-found',{status:404,type:'text/html',body:html});
  routes.set('/noindex',{type:'text/html',body:'<html><head><meta name="robots" content="noindex"><title>Noindex</title></head><body>Noindex.</body></html>'});
  routes.set('/index.xml',xml(`<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc>${siteOrigin}/index.xml</loc></sitemap><sitemap><loc>${siteOrigin}/leaf-one.xml</loc></sitemap><sitemap><loc>${siteOrigin}/leaf-one.xml</loc></sitemap><sitemap><loc>${siteOrigin}/leaf-two.xml</loc></sitemap></sitemapindex>`));
  routes.set('/leaf-one.xml',xml(leaf(loc('/listed','<lastmod>2023-02-29</lastmod>')+loc('/listed')+loc('/redirect')+loc('/not-found')+loc('/noindex')+loc('/blocked')+loc('/sitemap-only')+loc('/same-doc')+loc('/same-doc')+loc('/two-docs')+'<url/><url><loc>=SUM(1,1)</loc></url><url><loc>&lt;img src=x onerror=&quot;window.__nativeTargetInjected=1&quot;&gt;</loc></url>')));
  routes.set('/leaf-two.xml',xml(leaf(loc('/two-docs'))));
  routes.set('/policy.xml',xml(leaf(loc('/policy-original')+loc('/policy-original'))));
  routes.set('/broken.xml',xml('<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">'+loc('/broken-member')+'<url><loc>'));
  routes.set('/broken-seed',{type:'text/html',body:'<html><body><a href="/broken-not-listed">Observed outside incomplete sitemap</a></body></html>'});
  routes.set('/limited.xml',xml(leaf(loc('/limit-held')+loc('/limit-other')+loc('/limit-extra'))));
  routes.set('/ui-limited.xml',xml(leaf(loc('/ui-limit-a')+loc('/ui-limit-b'))));
  await launch();
  const served=[];for(const [route,file] of Object.entries(assetRoutes)){const value=await raw(route,undefined,200,4*1024*1024);assert.equal(value.response_sha256,sourceFiles.get(file).sha256);served.push({route,file,sha256:value.response_sha256});}
  check('01 exact Root binary and source manifest five served native assets owned ephemeral API',{served});
  const base={crawl_links:false,discover_sitemaps:false,obey_robots:false,concurrency:1,delay_ms:0,max_pages:100,request_timeout_secs:10,max_body_bytes:1048576};
  const complete=await create('complete',{...base,crawl_links:true,obey_robots:true,seed_urls:[siteOrigin+'/seed'],sitemap_urls:[siteOrigin+'/index.xml',siteOrigin+'/index.xml']});
  for(const name of ['/index.xml','/leaf-one.xml','/leaf-two.xml','/seed','/listed','/redirect','/not-found','/noindex','/sitemap-only','/same-doc','/two-docs','/not-listed'])expectRequest(name);expectRequest('/final',2);expectRequest('/robots.txt');
  await readInterval('native terminal source target and anchor facts',async()=>{
    const value=await targetPage(complete,{limit:'200',page_limit:'200'}),analysis=value.membership_analysis;
    assert.equal(value.coverage.derived.completion,'complete');assert.equal(value.coverage.derived.current_valid_original_location_negative_permitted,true);
    const finding=name=>analysis.location_findings.find(row=>row.original_membership_url===siteOrigin+name),saved=name=>analysis.saved_pages.find(row=>row.observation.requested_url===siteOrigin+name);
    assert.equal(finding('/redirect').original_state,'redirected');assert.equal(finding('/redirect').original_observation.status,301);assert.equal(finding('/redirect').original_observation.final_status,200);assert.equal(finding('/redirect').original_observation.final_url,siteOrigin+'/final');
    for(const [name,state] of [['/not-found','non200'],['/noindex','non_indexable'],['/blocked','blocked_by_robots']])assert.equal(finding(name).original_state,state);
    assert.equal(saved('/final').membership,'not_listed_current_declared_sources');assert.equal(saved('/not-listed').membership,'not_listed_current_declared_sources');
    assert.equal(saved('/listed').original_membership_occurrences,2);assert.equal(saved('/listed').original_source_document_count,1);assert.equal(saved('/same-doc').original_membership_occurrences,2);assert.equal(saved('/same-doc').original_source_document_count,1);
    assert.equal(saved('/two-docs').original_membership_occurrences,2);assert.equal(saved('/two-docs').original_source_document_count,2);
    assert.deepEqual(saved('/two-docs').original_source_document_samples.map(row=>row.task_url),[siteOrigin+'/leaf-one.xml',siteOrigin+'/leaf-two.xml']);
    const links=saved('/listed').observed_inlinks;assert.equal(links.anchor_occurrences,3);assert.equal(links.nonself_anchor_occurrences,2);assert.equal(links.distinct_nonself_requested_sources,1);assert(links.samples.some(row=>row.nofollow));
    assert.equal(saved('/sitemap-only').observed_inlinks.anchor_occurrences,0);assert.equal(saved('/sitemap-only').listed_without_observed_nonself_inlinks,true);
    assert.equal(site.hits.filter(row=>row.path==='/blocked').length,0);assert.equal(analysis.context.website_wide_orphan_inference,false);
  });check('02 real recursive duplicate and sibling sources HTTP301-200404 noindex robots exact anchors and qualified sitemap-only findings',{job:complete.id});
  const policy=await create('policy',{...base,seed_urls:[siteOrigin+'/policy-seed'],sitemap_urls:[siteOrigin+'/policy.xml'],crawl_policy:{scope:{roots:[{origin:siteOrigin,subdomains:false,folders:['/'],exact_paths:[]}],external:'deny',outside_folder:'deny'},rewrites:[{id:'native-original-alias',pattern:'/policy-original$',replacement:{kind:'literal',text:'/policy-alias'}}]}});
  for(const name of ['/policy-seed','/policy.xml','/policy-alias'])expectRequest(name);
  await readInterval('actual native rewrite identity separation',async()=>{const value=await targetPage(policy),findings=value.membership_analysis.location_findings;assert.equal(findings.length,2);for(const finding of findings){assert.equal(finding.original_membership_url,siteOrigin+'/policy-original');assert.equal(finding.admitted_fetch_url,siteOrigin+'/policy-alias');assert.equal(finding.original_observation,null);assert.equal(finding.original_state,'no_stored_requested_observation');assert.equal(finding.admitted_alias_observation.requested_url,siteOrigin+'/policy-alias');}const alias=value.membership_analysis.saved_pages.find(row=>row.observation.requested_url===siteOrigin+'/policy-alias');assert.equal(alias.original_membership_occurrences,0);assert.equal(alias.admitted_alias_occurrences,2);assert.equal(alias.membership,'not_listed_current_declared_sources');assert.equal(site.hits.filter(row=>row.path==='/policy-original').length,0);});check('03 actual native policy original URL remains separate from admitted alias and membership');
  let cursorWitness;
  await readInterval('independent native JSON raw and saved-page cursors',async()=>{
    const rawRows=await collect(complete,{page_limit:'0'},3);assert.deepEqual(rawRows.rows,complete.report.sitemap_catalog.rows);
    const allSaved=await savedPages(complete,2);assert.equal(allSaved.length,complete.report.pages.length);
    const first=await targetPage(complete,{limit:'1',page_limit:'1'});assert(first.next_cursor&&first.next_page_cursor);
    const pageOnly=await targetPage(complete,{limit:'1',page_limit:'1',page_cursor:first.next_page_cursor});assert.deepEqual(pageOnly.rows,first.rows);assert.equal(pageOnly.membership_analysis.saved_pages[0].observation.report_page_ordinal,1);
    const rawOnly=await targetPage(complete,{limit:'1',page_limit:'1',cursor:first.next_cursor});assert.equal(rawOnly.rows[0].row.sequence,complete.report.sitemap_catalog.rows[1].row.sequence);assert.deepEqual(rawOnly.membership_analysis.saved_pages,first.membership_analysis.saved_pages);
    assert.equal(first.snapshot_token,pageOnly.snapshot_token);assert.equal(first.snapshot_token,rawOnly.snapshot_token);cursorWitness=first;
  });check('04 real independent pagers enumerate exact original raw rows and every requested report page without gaps');
  await readInterval('actual cursor role and missing-job pre-I/O query controls',async()=>{
    const id=complete.id,rawToken=encodeURIComponent(cursorWitness.next_cursor),pageToken=encodeURIComponent(cursorWitness.next_page_cursor),snapshot=encodeURIComponent(cursorWitness.snapshot_token),absent='00000000-0000-4000-8000-000000000000';
    for(const route of [`/api/crawls/${id}/sitemaps?cursor=${pageToken}`,`/api/crawls/${id}/sitemaps?page_cursor=${rawToken}`,`/api/crawls/${id}/sitemaps/export.csv?snapshot=${pageToken}`,`/api/crawls/${id}/sitemaps?cursor=${snapshot}`])await error(route,400,'invalid_sitemap_cursor');
    await error(`/api/crawls/${id}/sitemaps?page_limit=0&page_cursor=${pageToken}`,400,'invalid_sitemap_query');
    await error(`/api/crawls/${policy.id}/sitemaps?page_limit=1&page_cursor=${pageToken}`,409,'stale_sitemap_cursor');
    await error(`/api/crawls/${id}/sitemaps?kind=location&page_cursor=${pageToken}`,409,'stale_sitemap_cursor');
    await error(`/api/crawls/${absent}/sitemaps?page_limit=201`,400,'invalid_sitemap_query');
    await error(`/api/crawls/${absent}/sitemaps?page_cursor=${rawToken}`,400,'invalid_sitemap_cursor');
    await error(`/api/crawls/${absent}/sitemaps`,404,'job_not_found');
  });check('05 both cursor roles export roles cross-job/filter mismatch and typed pre-job validation reject atomically');
  await readInterval('original raw CSV remains exact native catalog',async()=>{for(const job of [complete,policy]){const value=await targetPage(job,{limit:'1',page_limit:'1'}),result=await raw(`/api/crawls/${job.id}/sitemaps/export.csv?kind=all&snapshot=${encodeURIComponent(value.snapshot_token)}`,undefined,200,budgets.csv);const records=csvRows(result.bytes.toString('utf8'));assert.deepEqual(records[0],['kind','sequence','observation_id','occurrence_id','source_ordinal','raw_value','membership_url','admitted_fetch_url','admission_decision','typed_row_json']);assert.deepEqual(records.slice(1).map(row=>JSON.parse(row[9])),job.report.sitemap_catalog.rows);assert.equal(result.headers['x-sitemap-catalog-state-sha256'],value.snapshot.catalog_state_sha256);assert.equal(result.headers['x-crawl-generation'],String(value.snapshot.crawl_generation));}});check('06 original raw CSV typed rows and headers remain exact while target findings are separate');
  const limited=await create('limited',{...base,seed_urls:[siteOrigin+'/limit-seed'],sitemap_urls:[siteOrigin+'/limited.xml'],max_pages:2},'limited');expectRequest('/limited.xml');
  let oldLimited;
  await readInterval('genuine Limited checkpoint has pending target evidence',async()=>{oldLimited=await targetPage(limited,{limit:'1',page_limit:'1'});assert(oldLimited.next_cursor&&oldLimited.next_page_cursor);const all=await targetPage(limited);assert.equal(all.membership_analysis.context.observation_scope,'partial_terminal_saved_report_observations');assert(all.membership_analysis.location_findings.some(row=>row.original_state==='pending'));});
  const pendingTarget=(await targetPage(limited)).membership_analysis.location_findings.find(row=>row.original_state==='pending').original_membership_url;
  const gatePath=new URL(pendingTarget).pathname,gate={released:false,entered:false,requests:[]};gates.set(gatePath,gate);
  await json('/api/crawls/'+limited.id+'/resume',{max_pages:0,request_timeout_secs:30});await until(()=>gate.entered,'actual resumed native request held by own fixture');
  const liveStatus=await json('/api/crawls/'+limited.id);assert.equal(liveStatus.status,'running');assert(liveStatus.generation>limited.generation);
  const beforeLive=fixtures.map(row=>row.ordinal),live=await pageOf({...limited,generation:liveStatus.generation},{limit:'200',page_limit:'1'});coverage(live);assertTargetPage(live,limited,{available:false});assert.equal(live.membership_analysis.context.reason,'live_crawl_target_analysis_unavailable');
  for(const suffix of [`cursor=${encodeURIComponent(oldLimited.next_cursor)}`,`page_cursor=${encodeURIComponent(oldLimited.next_page_cursor)}&page_limit=1`])await error(`/api/crawls/${limited.id}/sitemaps?limit=1&${suffix}`,409,'stale_sitemap_cursor');
  await error(`/api/crawls/${limited.id}/sitemaps/export.csv?snapshot=${encodeURIComponent(oldLimited.snapshot_token)}`,409,'stale_sitemap_cursor');assert.deepEqual(fixtures.map(row=>row.ordinal),beforeLive,'Held live reads did not cause new website attempts');
  gate.released=true;for(const finish of gate.requests.splice(0))finish();const resumed=await captureJob('limited',limited.id,await waitTerminal(limited.id));assert.equal(resumed.report.pages.length,4);for(const name of ['/limit-seed','/limit-held','/limit-other','/limit-extra'])expectRequest(name);
  const uiStart=site.ordinal,limitedUi=await create('limitedUi',{...base,seed_urls:[siteOrigin+'/ui-limit-seed'],sitemap_urls:[siteOrigin+'/ui-limited.xml'],max_pages:1},'limited');
  const uiHits=site.hits.slice(uiStart);assert.equal(uiHits.filter(row=>row.path==='/ui-limited.xml').length,1);const uiPageHits=uiHits.filter(row=>['/ui-limit-seed','/ui-limit-a','/ui-limit-b'].includes(row.path));assert.equal(uiPageHits.length,1);assert.equal(limitedUi.report.pages.length,1);assert.equal(new URL(limitedUi.report.pages[0].url).pathname,uiPageHits[0].path);expectRequest('/ui-limited.xml');expectRequest(uiPageHits[0].path);
  check('07 real Limited and resumed-running snapshots separate pending from unavailable and invalidate actual old cursors',{resumed_job:resumed.id,gated_route:gatePath,live_generation:liveStatus.generation,limited_ui_job:limitedUi.id});
  const broken=await create('broken',{...base,crawl_links:true,seed_urls:[siteOrigin+'/broken-seed'],sitemap_urls:[siteOrigin+'/broken.xml']});for(const name of ['/broken-seed','/broken-not-listed','/broken.xml'])expectRequest(name);
  const empty=await create('empty',{...base,seed_urls:[siteOrigin+'/empty-seed'],sitemap_urls:[]});expectRequest('/empty-seed');
  const imported=await json('/api/crawls/import',complete.report);assert.match(imported.id,UUID);jobs.imported={id:imported.id};
  await readInterval('unknown source graph known-empty source cohort and missing SQLite authority',async()=>{
    const incomplete=await targetPage(broken);assert.equal(incomplete.coverage.derived.completion,'unknown');assert.equal(incomplete.coverage.derived.current_valid_original_location_negative_permitted,false);const unlisted=incomplete.membership_analysis.saved_pages.find(row=>row.observation.requested_url===siteOrigin+'/broken-not-listed');assert.equal(unlisted.membership,'unknown');
    const knownEmpty=await targetPage(empty);assert.equal(knownEmpty.coverage.derived.completion,'known_empty');assert.deepEqual(knownEmpty.rows,[]);assert.equal(knownEmpty.membership_analysis.saved_pages[0].membership,'not_listed_current_declared_sources');
    const unavailable=await pageOf(jobs.imported);coverage(unavailable);assertTargetPage(unavailable,jobs.imported);assert.equal(unavailable.rows,null);assert.equal(unavailable.catalog_available,false);try{await fs.stat(path.join(data,imported.id+'.sqlite'));throw Error('Imported report acquired new SQLite authority');}catch(error){if(error.code!=='ENOENT')throw error;}
  });check('08 malformed retained prefix stays unknown known-empty is qualified and real report import acquires no SQLite authority');
  const inherited=await recrawl('inherited',complete,[siteOrigin+'/listed']);expectRequest('/listed');expectRequest('/robots.txt');
  const historicalUnknown=await recrawl('historicalUnknown',broken,[siteOrigin+'/broken-seed']);expectRequest('/broken-seed');
  await readInterval('actual selected historical complete and unknown source states remain mixed age',async()=>{
    const value=await targetPage(inherited),pages=value.membership_analysis.saved_pages;assert.equal(value.coverage.derived.scope,'inherited_historical_declared_sources');assert.equal(value.coverage.derived.current_valid_original_location_negative_permitted,false);assert.equal(value.coverage.derived.historical_valid_original_location_negative_permitted,true);assert.equal(value.membership_analysis.context.observation_scope,'mixed_age_selected_saved_report');
    assert.equal(pages.find(row=>row.observation.requested_url===siteOrigin+'/listed').observation.age_scope,'selected_current_generation');const retained=pages.find(row=>row.observation.requested_url===siteOrigin+'/not-listed');assert.equal(retained.observation.age_scope,'inherited_parent_observation');assert.equal(retained.membership,'not_listed_historical_declared_sources');
    const unknown=await targetPage(historicalUnknown);assert.equal(unknown.coverage.derived.completion,'unknown');assert.equal(unknown.coverage.derived.historical_valid_original_location_negative_permitted,false);assert.equal(unknown.membership_analysis.context.observation_scope,'mixed_age_selected_saved_report');assert.equal(unknown.membership_analysis.saved_pages.find(row=>row.observation.requested_url===siteOrigin+'/broken-not-listed').membership,'unknown');
  });check('09 true selected recrawls retain exact parent rows historical-only membership and inherited page age');
  let expectedNames;
  for(const kind of ['stdio','streamable_http']){
    const client=new Client({name:'native-target-inlink-acceptance',version:'1.0.0'}),transport=kind==='stdio'?new StdioClientTransport({command:binary,args:['mcp','--server',api],cwd:sourceRoot,stderr:'pipe',env:{TMP:output,TEMP:output}}):new StreamableHTTPClientTransport(new URL(api+'/mcp'));
    await registerSdk(kind,client,transport);noteWire(kind,transport);
    try{await bounded(client.connect(transport),budgets.request,'actual SDK connect '+kind);await readInterval('official SDK '+kind+'25 tools two pagers and strict typed roles',async()=>{const names=assertInventory(await recordSdk(kind+' inventory',()=>client.listTools()));if(expectedNames)assert.deepEqual(names,expectedNames);else expectedNames=names;await sdkPagers(kind,client,complete);await sdkStale(kind,client,resumed,oldLimited);try{await recordSdk(kind+' unknown argument',()=>client.callTool({name:'spider_sitemaps',arguments:{id:complete.id,unexpected_argument:true}}));throw Error('Unknown argument accepted');}catch(error){assert.equal(error.code,-32602);}assert(sdkWire.some(row=>row.label===kind&&row.message.method==='tools/call'&&row.message.params?.arguments?.unexpected_argument===true));});}
    finally{await closeSdk(client,transport);}
    check(kind==='stdio'?'10 actual official stdio SDK25 both pagers roles and actual child close':'11 actual official HTTP SDK25 both pagers and typed roles');
  }
  await readInterval('official SDK stdio typed pre-I/O probes against owned offline API witness',async()=>{
    const client=new Client({name:'native-target-preio',version:'1.0.0'}),transport=new StdioClientTransport({command:binary,args:['mcp','--server',offlineOrigin],cwd:sourceRoot,stderr:'pipe',env:{TMP:output,TEMP:output}});await registerSdk('offline',client,transport);noteWire('offline',transport);
    const probes=[{id:complete.id,page_limit:'1'},{id:complete.id,page_limit:201},{id:complete.id,page_cursor:cursorWitness.next_cursor},{id:complete.id,cursor:cursorWitness.next_page_cursor},{id:complete.id,page_limit:0,page_cursor:cursorWitness.next_page_cursor}];
    try{await bounded(client.connect(transport),budgets.request,'offline actual SDK connect');assertInventory(await recordSdk('offline inventory',()=>client.listTools()));for(const argumentsValue of probes){const value=await recordSdk('offline typed '+JSON.stringify(argumentsValue).slice(0,100),()=>client.callTool({name:'spider_sitemaps',arguments:argumentsValue}));assert.equal(value.isError,true);assert.equal(value.structuredContent,undefined);assert(value.content.some(row=>row.text?.includes('page_limit must')||row.text?.includes('invalid_sitemap_query')||row.text?.includes('invalid_sitemap_cursor')));}assert.equal(sdkWire.filter(row=>row.label==='offline'&&row.message.method==='tools/call'&&row.message.params?.name==='spider_sitemaps').length,probes.length);}
    finally{await closeSdk(client,transport);}assert.equal(offline.ordinal,0);
  });check('12 five actual native stdio type and cursor-role probes reach SDK transport but never owned API witness');
  await stop();await launch();
  await readInterval('cold restarted native target analysis and catalog authority',async()=>{for(const job of [complete,policy,broken,empty,inherited,historicalUnknown,resumed,limitedUi]){await savedPages(job,2);assert.deepEqual((await collect(job,{page_limit:'0'},3)).rows,job.report.sitemap_catalog.rows);}const missing=await pageOf(jobs.imported);assert.equal(missing.rows,null);assert.equal(missing.membership_analysis,null);});check('13 cold restart reconstructs exact checkpoint target source and page provenance without website requests');
  browserProof=await runNativeTargetBrowser({api,sourceRoot,runtimeRoot,output,jobs,siteOrigin,offlineOrigin,assetRoutes,budgets,stamp,raw,json,readInterval,bounded,writeNew,jsonNew,hashFile,check:(name,evidence={})=>{browserChecks.push({name,...evidence});console.log('PASS_BROWSER '+name);},onProducer:record=>browserProducers.push(record),chrome});
  assert.deepEqual(browserProof.cases.map(row=>row.label),expectedBrowserLabels);assert.equal(browserProof.javascript_errors.length,0);assert.equal(browserProducers.length,1);assert(browserProducers.every(row=>row.actual_close_event&&row.server_close_event&&row.context_closed&&!row.forced&&!row.unexpected_close));check('14 actual native Chrome sixteen source target inspector pagination CSV320px and mutation-fence controls',{cases:browserProof.cases.length,pixel_review:'Root must inspect captured native screenshot pixels after execution'});
  await stop();for(const value of fixtures)await drain(value);
  const counts=Object.fromEntries([...new Set(site.hits.map(row=>row.path))].sort().map(route=>[route,site.hits.filter(row=>row.path===route).length])),expected=Object.fromEntries([...expectedRequests].sort(([left],[right])=>left<right?-1:left>right?1:0));
  assert.deepEqual(counts,expected);assert.equal(site.hits.filter(row=>row.path==='/blocked'||row.path==='/policy-original'||row.path==='/broken-member').length,0);assert(site.hits.every(row=>row.registered_route&&row.method==='GET'));assert.equal(offline.ordinal,0);assert.equal(appRuns.length,2);assert(appRuns.every(row=>row.closed&&!row.forced));assert.equal(stdioProducers.length,2);assert(stdioProducers.every(row=>row.actual_close_event));
  assert.equal(await hashFile(binary),args['binary-sha256']);await verifySources();assert.equal(await hashFile(args['source-manifest']),args['source-manifest-sha256']);assert.equal(await hashFile(fileURLToPath(import.meta.url)),harnessShaBefore);assert.equal(await hashFile(fileURLToPath(new URL('./sitemap-target-browser-v1.mjs',import.meta.url))),browserModuleShaBefore);assert.equal(await hashFile(fileURLToPath(new URL('./sitemap-target-assertions-v1.mjs',import.meta.url))),assertionsModuleShaBefore);
  check('15 exact final request counts after actual App SDK Chrome close and clean fixture TCP drains',{counts,offline_requests:offline.ordinal});
}catch(error){failures.push({message:error.message,stack:error.stack,at_ms:stamp()});process.exitCode=1;console.error(error.stack);}
finally{
  for(const gate of gates.values()){if(!gate.released){gate.released=true;for(const finish of gate.requests.splice(0)){try{finish();}catch(error){failures.push({phase:'owned gate cleanup',message:error.message});}}}}
  if(child){try{await stop();}catch(error){failures.push({phase:'owned App close',message:error.message});if(child&&!appClosed){child.stdout?.destroy();child.stderr?.destroy();child.unref();}}}
  for(const producer of sdkProducers){if(producer.closed)continue;try{await closeSdk(producer.client,producer.transport,{retry:true});}catch(error){failures.push({phase:'owned SDK cleanup',label:producer.label,message:error.message});}}
  for(const value of fixtures){if(value.drained)continue;try{if(appClosed)await drain(value);else throw Error('Owned App close unproven');}catch(error){failures.push({phase:'fixture drain',name:value.name,message:error.message});if(value.server.listening){const closure=new Promise(resolve=>value.server.close(resolve));value.server.closeAllConnections();for(const socket of value.sockets)socket.destroy();await bounded(closure,budgets.stop,'failed fixture cleanup').catch(()=>{});}value.forced_cleanup=true;}}
  const witnessDrained=fixtures.length===2&&fixtures.every(value=>value.drained&&!value.forced_cleanup)&&appClosed&&sdkProducers.every(value=>value.closed)&&stdioProducers.every(value=>value.actual_close_event)&&browserProducers.length===1&&browserProducers.every(value=>value.actual_close_event&&value.server_close_event&&value.context_closed&&!value.forced&&!value.unexpected_close);
  await jsonNew(path.join(output,'requests.json'),{phase:witnessDrained?'AFTER_ACTUAL_OWNED_APP_SDK_CHROME_CLOSE_AND_CLEAN_FIXTURE_DRAIN':'INCOMPLETE_WITNESS',fixtures:fixtures.map(value=>({name:value.name,origin:value.origin,drained:value.drained,forced_cleanup:!!value.forced_cleanup,active_requests:value.active.size,connected_sockets:value.sockets.size,total_requests:value.ordinal,rows:value.hits,errors:value.errors}))});
  const result=failures.length===0&&checks.length===15&&witnessDrained?'PASS_NATIVE_SITEMAP_TARGET_INLINK_API_SDK_BROWSER':'FAIL_NATIVE_SITEMAP_TARGET_INLINK_API_SDK_BROWSER';
  await jsonNew(path.join(output,'proof.json'),{schema_version:1,result,scope:'Actual future Root-supplied binary and frozen source: owned native crawler SQLite/API source-target/inlink projections original versus alias/final identities qualified current/historical sources independent raw/saved pagination raw CSV official SDK25 both transports and actual Chrome; no full vendor parity or fresh/universal orphan inference',source_linkage:'Root source and exact binary inputs; served five assets verified; compilation provenance requires separate Root build receipt',binary_sha256:args['binary-sha256'],source_manifest_sha256:args['source-manifest-sha256'],harness_sha256:await hashFile(fileURLToPath(import.meta.url)),browser_module_sha256:await hashFile(fileURLToPath(new URL('./sitemap-target-browser-v1.mjs',import.meta.url))),assertions_module_sha256:await hashFile(fileURLToPath(new URL('./sitemap-target-assertions-v1.mjs',import.meta.url))),output,checks,failures,browser_checks:browserChecks,browser:browserProof,owned_apps:appRuns.map(({closeLogs,...run})=>run),owned_sdk_transports:sdkProducers.map(({client,transport,...producer})=>producer),owned_sdk_stdio:stdioProducers.map(({closed,closeLogs,...record})=>record),owned_browsers:browserProducers,jobs:Object.fromEntries(Object.entries(jobs).map(([name,job])=>[name,{id:job.id,generation:job.generation,status:job.status,checkpoint:job.checkpoint,report_sha256:job.report_sha256}])),website_counts_phase:witnessDrained?'AFTER_ACTUAL_OWNED_APP_SDK_CHROME_CLOSE_AND_CLEAN_FIXTURE_DRAIN':'NOT_PROVEN',read_only_intervals:intervals,api_responses:apiRecords,sdk_observations:sdkRecords,sdk_outgoing_messages:sdkWire,retained_native_data_files:appClosed?await dataSnapshot():[],renderer_navigation_native:'NOT_EXERCISED_BY_THIS_NATIVE_HTTP_FIXTURE; directed stored-field and authored UI mock proofs remain separately labelled',screenshot_pixel_review:'Root inspection required after successful native capture',declared_source_cohort_complete:false,website_wide_orphan_inference:false});
  if(result!=='PASS_NATIVE_SITEMAP_TARGET_INLINK_API_SDK_BROWSER')process.exitCode=1;console.log(JSON.stringify({result,proof:path.join(output,'proof.json'),checks:checks.length}));
  for(const [name,handler] of signalHandlers)process.off(name,handler);
}
