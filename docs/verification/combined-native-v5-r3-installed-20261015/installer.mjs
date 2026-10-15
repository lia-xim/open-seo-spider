import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawn, execFileSync} from 'node:child_process';
import {createHash, randomUUID} from 'node:crypto';
import {fileURLToPath, pathToFileURL} from 'node:url';

// PREPARED ONLY: Root must issue the concrete, hash-pinned approval described in
// RELEASE-SCHEMA.md. This program accepts only an originally cold installation.
const HERE=path.dirname(fileURLToPath(import.meta.url));
const BASE='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const REPO='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https/outputs/open-seo-spider';
const DATA=path.join(REPO,'data'), TARGET=path.join(REPO,'target/release/open-seo-spider.exe');
const SOURCE='27a8b74c4cd0c394906f3fe3c0a6a0327569294b25056cf6c6f0aabcffc42788';
const OLD='ab1ac93c82905aa627d100d8622a50acf93dd7fe59c3f854eecdaac667110bcf';
const BINDER_SHA='6888c45c5cca4c1463f8d44fe2afea6ad9a02416980647b3ee30810a5a34d46f';
const sha=b=>createHash('sha256').update(b).digest('hex');
const read=p=>fs.readFileSync(p), json=p=>JSON.parse(read(p).toString('utf8').replace(/^\uFEFF/,''));
const hash=p=>sha(read(p));
const same=(a,b,label)=>assert.deepEqual(a,b,label);
const norm=p=>path.resolve(p).toLowerCase();
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
function verify(p,h){assert.match(h,/^[a-f0-9]{64}$/);same(hash(p),h,`Hash differs: ${p}`);}
function ordinary(p){
  p=path.resolve(p);
  for(;;){const s=fs.lstatSync(p);assert(!s.isSymbolicLink(),`Reparse path: ${p}`);const parent=path.dirname(p);if(parent===p)break;p=parent;}
}
function below(parent,p){const r=path.relative(path.resolve(parent),path.resolve(p));assert(r&&!r.startsWith('..')&&!path.isAbsolute(r));}
function save(p,v){const fd=fs.openSync(p,'wx');try{fs.writeFileSync(fd,JSON.stringify(v,null,2)+'\n');fs.fsyncSync(fd);}finally{fs.closeSync(fd);}}
function args(){const a=process.argv.slice(2);same(a.length,4);same(a[0],'--release');same(a[2],'--release-sha256');assert(path.isAbsolute(a[1]));verify(a[1],a[3]);return {path:a[1],sha256:a[3],value:json(a[1])};}
assert.equal(process.platform,'win32');assert.equal(process.env.NODE_OPTIONS,undefined);assert.equal(process.env.NODE_PATH,undefined);same(process.execArgv,[]);
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed');
const approval=args(), R=approval.value;
const NEW=R.candidate_sha256;assert.match(NEW,/^[a-f0-9]{64}$/);assert.notEqual(NEW,OLD);assert.notEqual(NEW,'c38f5743685353ab54331e074d2a20ac38543dfc4750c6896157218821b65348','Historical V4 candidate cannot install R3');assert(Number.isSafeInteger(R.candidate_bytes)&&R.candidate_bytes>0);
same(R.kind,'root_released_source_v5_r3_cold_install');same(R.status,'RELEASED');same(R.initial_state,'COLD_BOTH_PORTS_AND_PROCESSES');
same(R.user_data_integrity_contract_version,2,'Explicit runtime SHM LastWriteTime-only integrity contract required');
same(R.issued_by,'Root');assert(typeof R.user_authorization_reference==='string'&&R.user_authorization_reference.length>15);
same(R.source_manifest_sha256,SOURCE);same(R.candidate_sha256,NEW);same(R.installed_sha256,OLD);
const SUITE='f0d930348768fb3f033b978f223b62839f671c9da8148d08b995be3d24577bdb',BUILD=R.build_proof_sha256,BUILDER='3789801fca07630384d6237c9c6a7f746e69d8c5194f59b1cbf4a53c770cd845';assert.match(BUILD,/^[a-f0-9]{64}$/);
same(R.suite_proof_sha256,SUITE);same(R.build_proof_sha256,BUILD);same(R.build_driver_sha256,BUILDER);
verify(path.join(HERE,'manifest.json'),R.preparation_manifest_sha256);
const preparation=json(path.join(HERE,'manifest.json'));same(preparation.kind,'source_v5_r3_cold_installer_preparation');
for(const file of preparation.payloads)verify(path.join(HERE,file.path),file.sha256);
verify(R.independent_review.path,R.independent_review.sha256);const independent=json(R.independent_review.path);
same(independent.kind,'source_v5_r3_cold_installer_independent_review');same(independent.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');same(independent.findings,[]);same(independent.proposal_manifest.sha256,R.preparation_manifest_sha256);
const now=Date.now(), start=Date.parse(R.not_before), expiry=Date.parse(R.expires_at);
assert(Number.isFinite(start)&&Number.isFinite(expiry)&&start<=now&&now<expiry&&expiry-start<=2*3600_000);
verify(fileURLToPath(import.meta.url),R.installer_sha256);verify(path.join(HERE,'tree-ops.ps1'),R.tree_helper_sha256);
verify(process.execPath,R.node_sha256);ordinary(process.execPath);
const pwsh=R.powershell_path;assert(path.isAbsolute(pwsh));ordinary(pwsh);verify(pwsh,R.powershell_sha256);
function ps(mode,options={}){
  const a=['-NoLogo','-NoProfile','-NonInteractive','-File',path.join(HERE,'tree-ops.ps1'),'-Mode',mode];
  for(const [k,v] of Object.entries(options))a.push(`-${k}`,String(v));
  const raw=execFileSync(pwsh,a,{windowsHide:true,encoding:'utf8',maxBuffer:32*1024*1024});
  return JSON.parse(raw.replace(/^\uFEFF/,''));
}
function cold(){const v=ps('cold',{Root:REPO});same(v.listeners,[],'Original ports are occupied');same(v.processes,[],'Original app/renderer is running');return v;}
function snapshot(root,name){const out=path.join(OUTPUT,`${name}.json`);ps('snapshot',{Root:root,Out:out});return json(out);}
function copied(root,dest,name){freshDataCopyDestination(dest);const out=path.join(OUTPUT,`${name}.json`);ps('copy',{Root:root,Destination:dest,Out:out});const v=json(out);same(v.verified,true);return v;}
function canonical(v){if(Array.isArray(v))return v.map(canonical);if(v&&typeof v==='object')return Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])]));return v;}
const equalJSON=(a,b,label)=>same(canonical(a),canonical(b),label);
// Only these actual-runtime phases may report a baseline SQLite SHM LastWriteTime delta.
// No snapshot/array/row is rewritten, sorted, stripped or timestamp-restored.
const userDataIntegrityContract=Object.freeze({version:2,kind:'baseline_sqlite_shm_last_write_only',allowed_field:'modified',allowed_baseline_leaf_suffix:'.sqlite-shm',all_other_fields_and_arrays_exact:true});
let approvedRuntimeShmRows=null;
function baselineOrdinaryShmLeaf(row){return row&&row.kind==='file'&&typeof row.path==='string'&&/^[^\\/:]+\.sqlite-shm$/.test(row.path)&&typeof row.attributes==='string'&&!row.attributes.includes('ReparsePoint');}
function validModifiedIso(value){
  assert(typeof value==='string'&&/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{7}Z$/.test(value),'SHM modified must be original-format ISO UTC');
  const instant=Date.parse(value);assert(Number.isFinite(instant),'SHM modified date is invalid');
  same(new Date(instant).toISOString(),value.slice(0,23)+'Z','SHM modified date is not calendar-valid UTC');
}
function runtimeDataEqual(actual,baseline,phase,label){
  assert(['compat-old','compat-candidate','installed-original-live','rollback-original-after-live'].includes(phase),'Unapproved runtime metadata scope');
  assert(approvedRuntimeShmRows instanceof Map&&approvedRuntimeShmRows.size===4,'Four exact approved baseline SHM leaves required');
  assert(Array.isArray(actual)&&Array.isArray(baseline));same(actual.length,baseline.length,label+' row count');
  const deltas=[];
  for(let i=0;i<baseline.length;i++){
    const original=baseline[i],observed=actual[i];assert(original&&observed&&typeof original==='object'&&typeof observed==='object');
    same(observed.path,original.path,label+' exact row path/order');
    same(Object.keys(observed).sort(),Object.keys(original).sort(),label+' exact row keys');
    const allowed=baselineOrdinaryShmLeaf(original)&&approvedRuntimeShmRows.has(original.path);
    if(allowed)equalJSON(original,approvedRuntimeShmRows.get(original.path),'Caller changed approved SHM baseline');
    for(const field of Object.keys(original)){
      if(allowed&&field==='modified'){
        validModifiedIso(original.modified);validModifiedIso(observed.modified);
        if(observed.modified!==original.modified)deltas.push({phase,path:original.path,baseline_last_write_utc:original.modified,observed_last_write_utc:observed.modified});
      }else equalJSON(observed[field],original[field],label+' stable field '+field+' at row '+i);
    }
  }
  proof.volatile_sqlite_shm_last_write_deltas.push(...deltas);
  proof.runtime_data_comparisons.push({phase,rows:actual.length,allowed_last_write_deltas:deltas.length,all_bytes_streams_and_stable_metadata_exact:true});
}

const binderPath=path.join(BASE,'native-combined-rebind-v5-r3-v2/combined-bindings.mjs');verify(binderPath,BINDER_SHA);
const {bindCombined}=await import(pathToFileURL(binderPath));
const binding=bindCombined({source_manifest_sha256:SOURCE,suite_proof_sha256:SUITE,build_proof_sha256:BUILD,build_driver_sha256:BUILDER,candidate_binary_sha256:NEW},SOURCE);
same(sha(binding.suiteRaw),R.suite_proof_sha256);same(binding.binarySHA,NEW);same(fs.statSync(binding.binary).size,R.candidate_bytes);same(binding.build.binary.bytes,R.candidate_bytes);binding.verify();
const manifest=binding.manifest, sourceDirectory=path.join(BASE,'combined-source-v5-r3');
for(const row of manifest.files){ordinary(path.join(sourceDirectory,row.path));ordinary(path.join(manifest.source_worktree,row.path));}
ordinary(TARGET);ordinary(DATA);same(hash(TARGET),OLD);same(fs.statSync(TARGET).size,33010688);
verify(R.inventory.path,R.inventory.sha256);
same(norm(R.inventory.path),norm(path.join(HERE,'inventory-private.json')));same(R.inventory.sha256,preparation.private_inventory.sha256);const inventory=json(R.inventory.path);same(inventory.source_manifest_sha256,'45a07ef277cf1ffef5ff8d577fe05dc48ee1d84548ada2d8453aaf859a5d26b3','Unchanged historical private baseline provenance; fresh runtime snapshot must still match exactly');same(inventory.installed_binary.sha256,OLD);
for(const row of inventory.renderer_source){const p=path.join(REPO,row.path);ordinary(p);verify(p,row.expected_sha256);}
function rendererPins(){
  for(const row of manifest.files.filter(f=>f.path.startsWith('renderer/')))verify(path.join(REPO,row.path),row.sha256);
  const root=path.join(REPO,'renderer/node_modules'),actual=[],stack=[root];
  while(stack.length){const p=stack.pop();ordinary(p);const s=fs.lstatSync(p);const relative=path.relative(root,p)||'.';if(s.isDirectory()){actual.push({path:relative,kind:'directory'});for(const e of fs.readdirSync(p))stack.push(path.join(p,e));}else{assert(s.isFile());actual.push({path:relative,kind:'file',sha256:hash(p),bytes:s.size});}}
  const expected=inventory.renderer_dependency_tree.map(r=>r.kind==='directory'?{path:r.path,kind:r.kind}:{path:r.path,kind:r.kind,sha256:r.sha256,bytes:r.bytes});
  const sorted=rows=>rows.sort((a,b)=>a.path<b.path?-1:a.path>b.path?1:0);
  equalJSON(sorted(actual),sorted(expected),'Installed renderer dependency tree differs');
}
rendererPins();verify(R.chrome_path,R.chrome_sha256);ordinary(R.chrome_path);
const suiteWorker=json(path.join(BASE,'results/combined-suite-v5-r3/owned-worker.json'));
same(R.chrome_sha256,suiteWorker.chrome_sha256);same(suiteWorker.renderer_input_hashes.length,4);
for(const input of suiteWorker.renderer_input_hashes)verify(path.join(REPO,input.path),input.sha256);
same(inventory.renderer_dependency_tree.filter(r=>r.kind==='file').length,150);
same(binding.suite.owned_renderer_gates.length,4);same(binding.suite.owned_renderer_gates_verified,true);
const nativeHelper=path.join(HERE,'verify-current-acceptance.mjs');verify(nativeHelper,preparation.payloads.find(f=>f.path==='verify-current-acceptance.mjs').sha256);
const {admitCurrentNative}=await import(pathToFileURL(nativeHelper));verify(nativeHelper,preparation.payloads.find(f=>f.path==='verify-current-acceptance.mjs').sha256);
const nativeAcceptance=admitCurrentNative(R,{binding,HERE,SOURCE,SUITE,BUILD,BUILDER,NEW});
const {admitted,large,readyGuard,contract}=nativeAcceptance;
const OUTPUT=path.resolve(R.output);below(path.join(BASE,'results'),OUTPUT);same(path.dirname(OUTPUT),path.resolve(BASE,'results'));ordinary(path.dirname(OUTPUT));assert(!fs.existsSync(OUTPUT));cold();
// Full data copies remain direct siblings of DATA, preserving its inheritance parent.
// E OUTPUT holds proofs and the exact old EXE only. Root issues all concrete paths.
const DATA_PARENT=path.dirname(path.resolve(DATA)),OUTPUT_BASENAME=path.basename(OUTPUT),copies=R.data_copies;
assert.match(OUTPUT_BASENAME,/^cold-install-v5-r3(?:-[a-z0-9]{1,24}){0,6}-acceptance$/);
assert(copies&&typeof copies==='object'&&!Array.isArray(copies),'Root-bound same-parent data copies required');
same(copies.kind,'same_original_data_parent_v1');same(copies.output_basename,OUTPUT_BASENAME);
assert.match(copies.nonce,/^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/);
assert(path.isAbsolute(copies.parent));same(norm(copies.parent),norm(DATA_PARENT));ordinary(DATA_PARENT);
assert(copies.basenames&&typeof copies.basenames==='object'&&!Array.isArray(copies.basenames));
same(Object.keys(copies.basenames).sort(),['candidate_copy','data_backup','old_copy'],'Only three exact data-copy basenames are allowed');
const copyPaths={};
for(const [key,role]of [['data_backup','data-backup'],['old_copy','old-copy'],['candidate_copy','candidate-copy']]){
  const supplied=copies[key];assert(typeof supplied==='string'&&path.isAbsolute(supplied));
  const expectedName='.oss-'+OUTPUT_BASENAME+'-'+copies.nonce+'-'+role,actual=path.resolve(supplied);
  same(copies.basenames[key],expectedName,'Root release basename differs from deterministic copy name');
  same(path.basename(actual),expectedName,'Unexpected same-parent copy basename');
  same(norm(path.dirname(actual)),norm(DATA_PARENT),'Data copy is not a direct original-DATA sibling');
  assert.notEqual(norm(actual),norm(DATA));assert(!fs.existsSync(actual),'Same-parent data copy already exists');copyPaths[key]=actual;
}
same(new Set(Object.values(copyPaths).map(norm)).size,3);
const {data_backup:DATA_BACKUP,old_copy:OLD_COPY,candidate_copy:CANDIDATE_COPY}=copyPaths;
const git=R.git_path;assert(typeof git==='string'&&path.isAbsolute(git));ordinary(git);verify(git,R.git_sha256);
const gitExcludeLines=Object.values(copies.basenames).map(name=>'/'+name+'/');
function copyPrivacy(dest){
  ordinary(git);verify(git,R.git_sha256);
  execFileSync(git,['-C',REPO,'check-ignore','--no-index','--quiet','--',dest+path.sep],{windowsHide:true,encoding:'utf8',maxBuffer:1024*1024});
}
for(const dest of Object.values(copyPaths))copyPrivacy(dest);
function freshDataCopyDestination(dest){
  assert(Object.values(copyPaths).some(p=>norm(p)===norm(dest)),'Unapproved data copy destination');
  same(norm(path.dirname(path.resolve(dest))),norm(DATA_PARENT));ordinary(DATA_PARENT);
  assert(!fs.existsSync(dest),'Data copy destination exists before exclusive helper creation');
  copyPrivacy(dest);
}
const LOCK=path.join(BASE,'results/combined-native-outer-v4/active.lock');ordinary(path.dirname(LOCK));
const lockBytes=Buffer.from(JSON.stringify({kind:'source_v5_r3_cold_install_owned_lock',pid:process.pid,approval_sha256:approval.sha256,nonce:randomUUID()})+'\n');
let lockFD=null,lockIdentity=null,lockReleased=false,outputCreated=false;
const BACKUP=path.join(OUTPUT,'backup');
const proof={kind:'source_v5_r3_cold_install',result:'RUNNING_NOT_ACCEPTED',source_manifest_sha256:SOURCE,candidate_sha256:NEW,initial_state:'COLD_BOTH_PORTS_AND_PROCESSES',admitted,large,eof:nativeAcceptance.eof,ready_guard:R.ready_guard,independent_review:R.independent_review,backup:BACKUP,data_copies:{...copies,paths:copyPaths,expected_git_exclude_lines:gitExcludeLines,ownership:'exclusive_CreateDirectoryW_for_each_copied_root',data_backup_location:'same_original_DATA_parent',proof_and_old_exe_backup_location:BACKUP},children:[],requests:[],data_restored:false,user_data_integrity_contract:userDataIntegrityContract,user_data_all_bytes_and_recorded_metadata_equal:false,user_data_all_bytes_and_stable_metadata_equal:false,volatile_sqlite_shm_last_write_deltas:[],runtime_data_comparisons:[],graceful_windows_app_shutdown:'NOT_PROVEN',global_network_packet_trace:'NOT_RUN'};
const handles=[];let before=null,reportIds=[],replaced=false,keepRunning=false,newApp=null,newWorker=null;
function releaseLock(){assert(lockFD!==null&&lockIdentity,'No verified owned execution lock');const current=fs.lstatSync(LOCK);assert(!current.isSymbolicLink()&&current.isFile());same(current.dev,lockIdentity.dev);same(current.ino,lockIdentity.ino);same(current.birthtimeMs,lockIdentity.birthtimeMs);assert(read(LOCK).equals(lockBytes),'Execution lock bytes differ; retain');fs.closeSync(lockFD);lockFD=null;fs.unlinkSync(LOCK);lockReleased=true;}
// Every FD action is independently attempted. Event callbacks never throw.
function finalizeLogFile(log,errors,flush=true){
  if(!log||log.closed)return;
  if(flush){try{fs.fsyncSync(log.fd);}catch(error){log.sync_error=String(error);errors.push(log.path+' fsync: '+String(error));}}
  try{fs.closeSync(log.fd);log.closed=true;}catch(error){log.close_error=String(error);errors.push(log.path+' close: '+String(error));}
}
function assertOwnedClosed(h){
  assert(h.closed&&h.actual_close_event,'Owned process actual close missing');
  assert(h.fds_closed,'Owned process log descriptors not all closed');
  same(h.log_finalization_errors,[],'Owned log finalization failed');
}
function launch(exe,argv,label,cwd,env={}){
  ordinary(exe);const out=path.join(OUTPUT,label+'.stdout.log'),err=path.join(OUTPUT,label+'.stderr.log');
  const files=[];let cp;
  try{
    files.push({path:out,fd:fs.openSync(out,'wx'),closed:false,sync_error:null,close_error:null});
    files.push({path:err,fd:fs.openSync(err,'wx'),closed:false,sync_error:null,close_error:null});
    cp=spawn(exe,argv,{cwd,env:{...process.env,...env},windowsHide:true,detached:true,stdio:['ignore',files[0].fd,files[1].fd]});
  }catch(error){
    const cleanupErrors=[];for(const log of files)finalizeLogFile(log,cleanupErrors);
    (proof.launch_failures??=[]).push({role:label,error:String(error),log_files:files.map(({fd,...log})=>log),log_finalization_errors:cleanupErrors});
    throw new AggregateError([error,...cleanupErrors.map(message=>new Error(message))],'Owned child launch/log open failed: '+label);
  }
  const h={cp,role:label,executable:exe,args:argv,stdout:out,stderr:err,outFD:files[0].fd,errFD:files[1].fd,log_files:files,log_finalization_errors:[],spawned:false,closed:false,error:null,forced:false};
  h.close=new Promise(resolve=>{
    cp.once('spawn',()=>{h.spawned=true;});
    cp.once('error',error=>{h.error=String(error);});
    cp.once('close',(code,signal)=>{
      // No exception from an EventEmitter callback can bypass outer rollback.
      try{
        h.closed=true;h.code=code;h.signal=signal;h.actual_close_event=true;
        if(!h.parent_fds_closed_while_child_live){for(const log of h.log_files)finalizeLogFile(log,h.log_finalization_errors);}
      }catch(error){h.log_finalization_errors.push('Close callback bookkeeping: '+String(error));}
      finally{h.fds_closed=h.log_files.every(log=>log.closed);resolve();}
    });
  });
  handles.push(h);proof.children.push(h);return h;
}
async function bounded(p,ms,label){let timer;try{return await Promise.race([p,new Promise((_,reject)=>timer=setTimeout(()=>reject(new Error(label)),ms))]);}finally{clearTimeout(timer);}}
async function stop(h){
  if(!h)return;
  if(h.closed){assertOwnedClosed(h);return;}
  if(!h.spawned)await sleep(50);
  if(!h.closed){h.forced=true;assert(h.cp.kill('SIGKILL'),'Owned child termination request failed');}
  await bounded(h.close,10000,'Owned child actual close missing');assertOwnedClosed(h);
}
async function request(origin,route,body){
  const url=new URL(route,origin);same(url.origin,origin);same(url.hostname,'127.0.0.1');
  assert(route==='/mcp'||!body,'Only MCP inventory POST is permitted');
  if(body){same(body.method,'tools/list');same(body.params,{});}
  proof.requests.push({origin,route,method:body?'POST':'GET',purpose:'local read-only compatibility/install check'});
  const response=await fetch(url,{method:body?'POST':'GET',headers:body?{'content-type':'application/json'}:{},body:body?JSON.stringify(body):undefined,signal:AbortSignal.timeout(20000)});
  assert(response.ok,`${route}: ${response.status}`);return Buffer.from(await response.arrayBuffer());
}
async function ready(h,port=0){
  const deadline=Date.now()+30000;
  while(Date.now()<deadline){assert(!h.closed&&!h.error,`Owned startup closed: ${h.role}`);const text=read(h.stdout).toString('utf8');const found=text.match(/Open SEO Spider: http:\/\/127\.0\.0\.1:(\d+)/);
    if(found){const observed=Number(found[1]);if(port)same(observed,port);const origin=`http://127.0.0.1:${observed}`;const v=JSON.parse((await request(origin,'/healthz')).toString());same(v.name,'Open SEO Spider');const own=ps('owner',{ProcessId:h.cp.pid,Port:observed});same(norm(own.executable),norm(h.executable));assert(own.command_line.includes('--data-dir'));assert(own.listeners.length>0&&own.listeners.every(l=>l.OwningProcess===h.cp.pid&&l.LocalAddress==='127.0.0.1'));h.identity=own;h.origin=origin;return origin;}await sleep(100);
  }throw new Error(`Readiness timed out: ${h.role}`);
}
const assets=[['/','web/index.html'],['/app.js','web/app.js'],['/saved-comparison.js','web/saved-comparison.js'],['/styles.css','web/styles.css'],['/list-import.js','web/list-import.js'],['/serp','web/serp.html'],['/serp.js','web/serp.js'],['/robots','web/robots.html'],['/robots.js','web/robots.js']];
async function observe(h,origin,newVersion,label){
  const listing=JSON.parse((await request(origin,'/api/crawls')).toString());assert(Array.isArray(listing.crawls));const jobs=[...listing.crawls].sort((a,b)=>a.id.localeCompare(b.id));same(jobs.map(j=>j.id).sort(),reportIds,'Stored job set differs');
  for(const j of jobs){same(j.status,'completed');for(const field of ['queued','discovery_pending','in_flight'])same(j.stats[field],0);}
  const details=[];for(const job of jobs){const detail=JSON.parse((await request(origin,`/api/crawls/${job.id}`)).toString());equalJSON(detail,job,'List/detail disagree');const project=JSON.parse((await request(origin,`/api/crawls/${job.id}/project.json`)).toString());const history=JSON.parse((await request(origin,`/api/crawls/${job.id}/sitemaps?limit=1`)).toString());same(history.website_requests,0);if(!history.catalog_available){same(history.rows,null);same(history.coverage.derived.completion,'unknown');}details.push({id:job.id,detail,project,history});}
  const observations={jobs,details};
  if(newVersion){
    const capability=JSON.parse((await request(origin,'/api/capabilities')).toString());assert(capability.mcp&&!Object.hasOwn(capability.mcp,'file_tools'),'File tools enabled or capability shape differs without explicit directory');
    const inventory=JSON.parse((await request(origin,'/mcp',{jsonrpc:'2.0',id:1,method:'tools/list',params:{}})).toString());same(inventory.result.tools.map(t=>t.name),contract.legacyNames);same(inventory.result.tools.length,26);
    observations.assets=[];for(const [route,source]of assets){const expected=manifest.files.find(f=>f.path===source);assert(expected);const bytes=await request(origin,route);same(sha(bytes),expected.sha256);same(bytes.length,expected.bytes);observations.assets.push({route,source,sha256:expected.sha256,bytes:expected.bytes});}
    observations.capabilities=capability;observations.tool_names=inventory.result.tools.map(t=>t.name);
  }
  save(path.join(OUTPUT,`${label}-observations-private.json`),observations);return observations;
}
function compareHistory(old,newer){
  // Existing metadata and all stored report fields must match completely.
  // Newly available source views are separately checked; no generic defaults
  // removal, URL normalization, count substitution or projection is applied.
  equalJSON(newer.jobs,old.jobs,'Historical complete job metadata differs');
  equalJSON(newer.details.map(d=>({id:d.id,project:d.project})),old.details.map(d=>({id:d.id,project:d.project})),'Historical full report export differs');
}
function verifyMutationAuthority(){
  const instant=Date.now();assert(start<=instant&&instant<expiry,'Root install approval expired before File.Replace');
  verify(approval.path,approval.sha256);verify(path.join(HERE,'manifest.json'),R.preparation_manifest_sha256);
  for(const file of preparation.payloads)verify(path.join(HERE,file.path),file.sha256);
  verify(fileURLToPath(import.meta.url),R.installer_sha256);verify(path.join(HERE,'tree-ops.ps1'),R.tree_helper_sha256);
  ordinary(process.execPath);ordinary(pwsh);ordinary(R.chrome_path);verify(process.execPath,R.node_sha256);verify(pwsh,R.powershell_sha256);verify(R.chrome_path,R.chrome_sha256);
  verify(R.independent_review.path,R.independent_review.sha256);verify(R.ready_guard.path,R.ready_guard.sha256);verify(large.inner.path,large.inner.sha256);verify(large.outer.path,large.outer.sha256);
  for(const g of admitted){verify(g.inner.path,g.inner.sha256);verify(g.outer.path,g.outer.sha256);if(g.supplement)verify(g.supplement.path,g.supplement.sha256);}
  nativeAcceptance.verify();rendererPins();binding.verify();verify(TARGET,OLD);verify(binding.binary,NEW);cold();equalJSON(snapshot(DATA,'data-at-mutation-authority-check'),before,'Original user data changed at actual mutation boundary');
  assert(lockFD!==null&&lockIdentity);const current=fs.lstatSync(LOCK);same(current.dev,lockIdentity.dev);same(current.ino,lockIdentity.ino);assert(read(LOCK).equals(lockBytes),'Owned execution lock changed before mutation');
  assert(start<=Date.now()&&Date.now()<expiry,'Root install approval expired during final guards');verify(approval.path,approval.sha256);
}
function stageFile(from,to,expected){ordinary(path.dirname(to));assert(!fs.existsSync(to));const fd=fs.openSync(to,'wx');try{fs.writeFileSync(fd,read(from));fs.fsyncSync(fd);}finally{fs.closeSync(fd);}verify(to,expected);}
try{
  // The whole successful-lock-open lifetime, including every preflight write/read,
  // is covered by this catch/finally. Never release a foreign pre-existing lock.
  lockFD=fs.openSync(LOCK,'wx');lockIdentity=fs.fstatSync(lockFD);fs.writeFileSync(lockFD,lockBytes);fs.fsyncSync(lockFD);
  fs.mkdirSync(OUTPUT);outputCreated=true;fs.mkdirSync(BACKUP);save(path.join(OUTPUT,'approval.json'),{path:approval.path,sha256:approval.sha256,value:R});
before=snapshot(DATA,'data-before');
proof.old_executable_metadata=snapshot(TARGET,'old-executable-metadata');
const userDataBytes=before.filter(r=>r.kind==='file').reduce((n,r)=>n+r.bytes,0);
const freeE=fs.statfsSync(BASE),freeC=fs.statfsSync(path.dirname(TARGET));
assert(freeE.bavail*freeE.bsize>userDataBytes*4+R.candidate_bytes*2+64*1024*1024,'Insufficient E scratch space');
const namedStreamBytes=before.filter(r=>r.kind==='file').reduce((n,r)=>n+(r.streams??[]).reduce((sum,s)=>sum+s.bytes,0),0);
 assert(Number.isSafeInteger(namedStreamBytes)&&namedStreamBytes>=0);
 assert(freeC.bavail*freeC.bsize>(userDataBytes+namedStreamBytes)*3+R.candidate_bytes+33010688+16*1024*1024,'Insufficient C data-copy/staging/rollback space');
const contentRows=rows=>rows.map(r=>r.kind==='file'?{path:r.path,kind:r.kind,bytes:r.bytes,sha256:r.sha256,streams:r.streams}: {path:r.path,kind:r.kind});
const inventoryContent=inventory.data_entries.map(r=>r.kind==='file'?{path:r.path,kind:r.kind,bytes:r.bytes,sha256:r.sha256,streams:r.named_streams}:{path:r.path,kind:r.kind});
const byPath=(a,b)=>a.path<b.path?-1:a.path>b.path?1:0;
equalJSON(contentRows(before).sort(byPath),inventoryContent.sort(byPath),'User-data content differs from the Root-approved inventory');
const approvedMetadata=inventory.data_entries.map(r=>({path:r.path,kind:r.kind,attributes:r.attributes,created:r.creation_utc,modified:r.last_write_utc,sddl:r.sddl,...(r.kind==='file'?{bytes:r.bytes,sha256:r.sha256,streams:r.named_streams}:{})}));
equalJSON([...before].sort(byPath),approvedMetadata.sort(byPath),'Original user-data recorded NTFS metadata differs from the fresh Root-approved baseline');
 approvedRuntimeShmRows=new Map();for(const row of before){if(baselineOrdinaryShmLeaf(row)){assert(!approvedRuntimeShmRows.has(row.path));validModifiedIso(row.modified);approvedRuntimeShmRows.set(row.path,structuredClone(row));}}
 same(approvedRuntimeShmRows.size,4,'Fresh exact inventory must contain the four approved ordinary SHM leaves');
 proof.approved_volatile_sqlite_shm_baseline_leaves=[...approvedRuntimeShmRows.keys()];
for(const row of before.filter(r=>r.kind==='file'&&r.path.endsWith('.job.json'))){same(json(path.join(DATA,row.path)).status,'completed','Persisted non-completed job blocks cold install');}
reportIds=before.filter(r=>r.kind==='file'&&/^[a-f0-9-]{36}\.json$/i.test(r.path)).map(r=>path.basename(r.path,'.json')).sort();
assert(reportIds.length>0,'No stored reports admitted');

  copied(DATA,DATA_BACKUP,'backup-copy-verified');equalJSON(snapshot(DATA,'data-after-backup'),before,'Source changed after backup');
  stageFile(TARGET,path.join(BACKUP,'old-open-seo-spider.exe'),OLD);
  copied(DATA_BACKUP,OLD_COPY,'old-copy-verified');copied(DATA_BACKUP,CANDIDATE_COPY,'candidate-copy-verified');
  const old=launch(path.join(BACKUP,'old-open-seo-spider.exe'),['serve','--listen','127.0.0.1:0','--data-dir',OLD_COPY],'compat-old',REPO);
  const oldObserved=await observe(old,await ready(old),false,'compat-old');await stop(old);
  const candidate=launch(binding.binary,['serve','--listen','127.0.0.1:0','--data-dir',CANDIDATE_COPY],'compat-candidate',REPO);
  const candidateObserved=await observe(candidate,await ready(candidate),true,'compat-candidate');await stop(candidate);compareHistory(oldObserved,candidateObserved);
  runtimeDataEqual(snapshot(OLD_COPY,'old-copy-after'),before,'compat-old','Old copy startup changed stable data');runtimeDataEqual(snapshot(CANDIDATE_COPY,'candidate-copy-after'),before,'compat-candidate','Candidate copy startup changed stable data');
  verify(R.ready_guard.path,R.ready_guard.sha256);verify(large.inner.path,large.inner.sha256);verify(large.outer.path,large.outer.sha256);cold();equalJSON(snapshot(DATA,'data-before-replace'),before,'Original data changed before replacement');binding.verify();rendererPins();verify(TARGET,OLD);verify(binding.binary,NEW);
  const nonce=randomUUID(),stage=path.join(path.dirname(TARGET),`.open-seo-spider-v5-r3-${nonce}.stage`),oldSibling=path.join(path.dirname(TARGET),`.open-seo-spider-ab1-${nonce}.backup.exe`);
  stageFile(binding.binary,stage,NEW);proof.replacement_intent={target:TARGET,stage,oldSibling};save(path.join(OUTPUT,'replacement-intent.json'),proof.replacement_intent);
  // No long copy/compatibility phase may carry an expired or altered approval
  // across the actual executable mutation boundary.
  verifyMutationAuthority();
  const replacement=ps('replace',{Root:TARGET,Stage:stage,Destination:oldSibling});replaced=true;proof.replacement=replacement;proof.old_sibling_backup=oldSibling;verify(TARGET,NEW);verify(oldSibling,OLD);
  rendererPins();cold();
  newWorker=launch(process.execPath,[path.join(REPO,'renderer/server.mjs')],'installed-renderer',path.join(REPO,'renderer'),{OPEN_SEO_RENDERER_PORT:'8790',OPEN_SEO_RENDERER_CONCURRENCY:'4',OPEN_SEO_BROWSER_PATH:R.chrome_path});
  let workerHealth;
  for(let n=0;n<100;n++){assert(!newWorker.closed&&!newWorker.error,'Renderer exited');try{workerHealth=JSON.parse((await request('http://127.0.0.1:8790','/health')).toString());break;}catch{await sleep(100);}}
  same(workerHealth?.worker,'open-seo-spider-renderer');same(workerHealth.active,0);same(workerHealth.queued,0);same(workerHealth.browser.available,true);
  const workerOwner=ps('owner',{ProcessId:newWorker.cp.pid,Port:8790});same(norm(workerOwner.executable),norm(process.execPath));assert(workerOwner.command_line.includes('renderer'));assert(workerOwner.listeners.length>0&&workerOwner.listeners.every(l=>l.OwningProcess===newWorker.cp.pid&&l.LocalAddress==='127.0.0.1'));newWorker.identity=workerOwner;
  newApp=launch(TARGET,['serve','--listen','127.0.0.1:8787','--data-dir',DATA],'installed-app',REPO);
  const installed=await observe(newApp,await ready(newApp,8787),true,'installed');compareHistory(candidateObserved,installed);
  runtimeDataEqual(snapshot(DATA,'data-after-install-readonly'),before,'installed-original-live','Original data/stable metadata changed after readonly install');rendererPins();binding.verify();verify(TARGET,NEW);
  proof.result='PASS_SOURCE_V5_R3_COLD_LOCAL_INSTALL_WITH_COPY_COMPATIBILITY';proof.recorded_at=new Date().toISOString();proof.binary={path:TARGET,sha256:NEW,bytes:fs.statSync(TARGET).size};proof.dynamic={data_files:before.filter(r=>r.kind==='file').length,report_jobs:installed.jobs.length};same(proof.user_data_all_bytes_and_recorded_metadata_equal,false);proof.pid_files_updated=false;proof.assets=installed.assets;proof.tool_names=installed.tool_names;proof.renderer_health=workerHealth;proof.rollback_exercised=false;proof.website_boundary='Only read-only local API operations were issued; persisted jobs were completed and source startup does not auto-resume. Per-sitemap read receipts report zero website IO. No global network packet trace was performed.';
  keepRunning=true;
}catch(error){
  proof.result='FAIL_SOURCE_V5_R3_COLD_INSTALL';proof.failure=String(error.stack??error);proof.rollback='NOT_NEEDED_BEFORE_REPLACEMENT';
  // A replacement helper can fail after the namespace operation (for example,
  // while emitting its receipt). Recover from actual target bytes, not intent.
  if(proof.replacement_intent&&fs.existsSync(TARGET)&&hash(TARGET)===NEW)replaced=true;
  const closeErrors=[];for(const h of handles){try{await stop(h);}catch(closeError){closeErrors.push(String(closeError));}}
  if(closeErrors.length)proof.close_errors=closeErrors;
  if(replaced){try{assert(!handles.some(h=>!h.closed),'Owned process still live');cold();const rollbackStage=path.join(path.dirname(TARGET),`.open-seo-spider-rollback-${randomUUID()}.stage`);const failedSibling=path.join(path.dirname(TARGET),`.open-seo-spider-failed-v5-r3-${randomUUID()}.backup.exe`);verify(TARGET,NEW);stageFile(path.join(BACKUP,'old-open-seo-spider.exe'),rollbackStage,OLD);ps('replace',{Root:TARGET,Stage:rollbackStage,Destination:failedSibling});verify(TARGET,OLD);proof.rollback='OLD_EXACT_BYTES_RESTORED_INITIAL_COLD_STATE';proof.failed_candidate_preserved=failedSibling;proof.rollback_exercised=true;if(newApp?.spawned===true){runtimeDataEqual(snapshot(DATA,'data-after-rollback'),before,'rollback-original-after-live','User stable data changed after started App; no data restore was attempted');}else{equalJSON(snapshot(DATA,'data-after-rollback'),before,'User data changed without original App startup; no data restore was attempted');}}catch(rollbackError){proof.rollback='NOT_PROVEN';proof.rollback_error=String(rollbackError);}}
  process.exitCode=1;
}finally{
  const finalizationErrors=[];
  // On a finalization error, verified installed services may deliberately remain
  // running, but the terminal result must never remain PASS.
  if(keepRunning){for(const h of [newApp,newWorker]){
    try{
      assert(h&&!h.closed);h.parent_fds_closed_while_child_live=true;
      for(const log of h.log_files)finalizeLogFile(log,h.log_finalization_errors,false);
      h.fds_closed=h.log_files.every(log=>log.closed);
      assert(h.fds_closed,'Parent duplicate log FD remains open');same(h.log_finalization_errors,[],'Parent log handoff failed');
    }catch(error){finalizationErrors.push('Parent log handle handoff: '+String(error));}
    try{h?.cp.unref();}catch(error){finalizationErrors.push('Owned service unref: '+String(error));}
  }}
  proof.children=handles.map(({cp,close,outFD,errFD,...h})=>({...h,pid:cp.pid}));
  if(lockFD!==null){
    if(keepRunning||handles.every(h=>h.closed)){try{releaseLock();}catch(error){proof.lock_release_error=String(error);finalizationErrors.push('Owned lock completion: '+String(error));}}
    else{finalizationErrors.push('Owned children not actually closed; execution lock retained');}
  }
  proof.install_execution_lock_released=lockReleased;
  if(finalizationErrors.length){proof.previous_result=proof.result;proof.result='FAIL_SOURCE_V5_R3_COLD_INSTALL_FINALIZATION';proof.finalization_errors=finalizationErrors;process.exitCode=1;}
  proof.installed_services_left_running=keepRunning;proof.accepted_installation=keepRunning&&finalizationErrors.length===0;
  proof.user_data_all_bytes_and_stable_metadata_equal=proof.accepted_installation===true&&proof.result==='PASS_SOURCE_V5_R3_COLD_LOCAL_INSTALL_WITH_COPY_COMPATIBILITY';
  const proofPath=outputCreated?path.join(OUTPUT,'proof-private.json'):path.join(path.dirname(OUTPUT),path.basename(OUTPUT)+'.failure-'+randomUUID()+'.json');
  proof.recorded_at=new Date().toISOString();save(proofPath,proof);
  console.log(JSON.stringify({result:proof.result,proof:proofPath,binary_sha256:keepRunning?NEW:hash(TARGET),accepted_installation:proof.accepted_installation,installed_services_left_running:keepRunning,rollback:proof.rollback??null}));
}
