/** AUTHORED / NOT_RUN. Actual owned HTTP -> product Crawler -> SQLite -> reopen.
 * Source8 retains explicit128 MiB membership capacity; actual50k initial/reopened positive remains mandatory.
 * Fixture files are harness witnesses, never asserted to be product-retained XML.
 */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createGzip, createGunzip} from 'node:zlib';
import {pipeline} from 'node:stream/promises';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {performance} from 'node:perf_hooks';

const SELF=fileURLToPath(import.meta.url), OWN=path.dirname(SELF);
const BASE='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const SOURCE_SHA=argumentsOnly()['manifest-sha'];
const XML_LIMIT=52_428_800, WORK_CAP=67_108_864, MEMBERSHIP_CAP=134_217_728, DEFAULT_MEMBERSHIP_CAP=33_554_432, PUBLIC_JSON_CAP=8_388_608, CSV_CAP=67_108_864;
const MEMBERSHIP_LIMITS=Object.freeze({max_catalog_json_bytes:MEMBERSHIP_CAP,max_row_json_bytes:262_144,max_rows:1_000_000});
const SOURCE_ROOT=path.join(BASE,'combined-source-v8');
const SHA=value=>createHash('sha256').update(value).digest('hex');
const normalized=value=>path.resolve(value).replaceAll('\\','/').toLowerCase();
const equalPath=(a,b)=>assert.equal(normalized(a),normalized(b));
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const actualStreamClose=stream=>new Promise(resolve=>stream.once('close',resolve));
function childOf(parent,child){const relative=path.relative(parent,child);return relative!==''&&relative!=='..'&&!relative.startsWith('..'+path.sep)&&!path.isAbsolute(relative);}
function read(file,max=8*1024*1024){const stat=fs.lstatSync(file);assert(stat.isFile()&&!stat.isSymbolicLink()&&stat.size>0&&stat.size<=max,file);const bytes=fs.readFileSync(file);assert.equal(bytes.length,stat.size);return bytes;}
async function hashFile(file){const digest=createHash('sha256');let bytes=0;for await(const chunk of fs.createReadStream(file)){bytes+=chunk.length;digest.update(chunk);}return {bytes,sha256:digest.digest('hex')};}
function argumentsOnly(){
  const allowed=['release','release-sha','preparation-sha','manifest-sha','bindings','bindings-sha','suite-proof-sha','build-proof-sha','build-driver-sha','candidate-binary-sha','out'],args={};
  for(let i=2;i<process.argv.length;i+=2){assert(process.argv[i]?.startsWith('--'));const name=process.argv[i].slice(2);assert(allowed.includes(name)&&!Object.hasOwn(args,name)&&process.argv[i+1]);args[name]=process.argv[i+1];}
  assert.equal(Object.keys(args).length,allowed.length);
  for(const name of allowed.filter(name=>name.endsWith('-sha')))assert.match(args[name],/^[a-f0-9]{64}$/);
  for(const name of ['release','bindings','out']){assert(path.isAbsolute(args[name]));args[name]=path.resolve(args[name]);}
  return args;
}

// Admission precedes ALL fixture generation, mkdir, SQLite import, listen or spawn.
const args=argumentsOnly();
assert.equal(process.platform,'win32');assert.match(process.versions.node,/^24\./);
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed');
assert(!process.env.NODE_OPTIONS&&!process.env.NODE_PATH);assert.equal(process.execArgv.length,0);
assert.equal(args['manifest-sha'],SOURCE_SHA);assert.equal(SOURCE_SHA,'58ab1ff000b9ec5ffa005eeab573a0cde3f4af4bb7c9c132b4d85251ea4de720');
const preparationPath=path.join(OWN,'manifest.json'),preparationRaw=read(preparationPath);
assert.equal(SHA(preparationRaw),args['preparation-sha']);const preparation=JSON.parse(preparationRaw);
assert.equal(preparation.kind,'genuine_large_sitemap_native_preparation_v8');
assert.equal(preparation.status,'AUTHORED_NOT_RUN');assert.equal(preparation.source_manifest_sha256,'58ab1ff000b9ec5ffa005eeab573a0cde3f4af4bb7c9c132b4d85251ea4de720');
assert.equal(preparation.cases.length,13);
for(const row of preparation.files){assert(typeof row.path==='string'&&/^[a-zA-Z0-9_.-]+$/.test(row.path));const bytes=read(path.join(OWN,row.path),4*1024*1024);assert.equal(bytes.length,row.bytes);assert.equal(SHA(bytes),row.sha256);}
const selfRow=preparation.files.find(row=>row.path===path.basename(SELF));assert(selfRow);assert.equal(SHA(read(SELF)),selfRow.sha256);
const binderPath=args.bindings;equalPath(binderPath,path.join(BASE,'combined-v8-runner-preparation-v4/combined-bindings-v8.mjs'));
assert.equal(SHA(read(binderPath)),args['bindings-sha']);
assert.equal(args['bindings-sha'],preparation.binding_requirements.actual_binder_sha256);
const releaseRaw=read(args.release);assert.equal(SHA(releaseRaw),args['release-sha']);const release=JSON.parse(releaseRaw);
assert.equal(release.kind,'root_released_combined_large_sitemap_native_v8');
assert.equal(release.scope,'native_combined_s4_genuine_large_sources_v8');assert.equal(release.status,'RELEASED');
assert.equal(release.source_manifest_sha256,SOURCE_SHA);assert.equal(release.build_mode,'optimized');
assert.equal(release.driver_sha256,selfRow.sha256);assert.equal(release.bindings_sha256,args['bindings-sha']);
for(const [field,argument] of [['suite_proof_sha256','suite-proof-sha'],['build_proof_sha256','build-proof-sha'],['build_driver_sha256','build-driver-sha'],['candidate_binary_sha256','candidate-binary-sha']])assert.equal(release[field],args[argument]);
assert.equal(release.preparation_manifest_sha256,args['preparation-sha']);equalPath(release.output,args.out);
const beginning=Date.parse(release.not_before),expiry=Date.parse(release.expires_at);
assert(Number.isFinite(beginning)&&Number.isFinite(expiry)&&beginning<=Date.now()&&Date.now()<expiry&&expiry-beginning<=4*3600_000);
assert(childOf(path.join(OWN,'runs'),args.out)&&!fs.existsSync(args.out),'Fresh owned output child only');
equalPath(path.dirname(args.out),path.join(OWN,'runs'));
// The serial Root outer owns this exact existing global lock through actual
// child close and both drained pipes. The child only reads it, never reacquires,
// creates, removes, closes, steals, or replaces a lock.
const outerLockPath=path.join(BASE,'results/combined-native-outer-v4/active.lock');
let outerLockIdentity=null;
function verifyOuterLock(){
  const stat=fs.lstatSync(outerLockPath);assert(stat.isFile()&&!stat.isSymbolicLink());
  const bytes=read(outerLockPath,64*1024),value=JSON.parse(bytes);
  assert.equal(value.owner_pid,process.ppid,'Direct retained Root outer must own the global lock');
  assert.equal(value.gate,'large-source-v8');assert.match(value.token,/^[a-f0-9-]{36}$/);
  if(outerLockIdentity){assert.equal(stat.dev,outerLockIdentity.dev);assert.equal(stat.ino,outerLockIdentity.ino);assert(bytes.equals(outerLockIdentity.bytes),'Global native lock identity/bytes changed');}
  else outerLockIdentity={dev:stat.dev,ino:stat.ino,bytes};
}
verifyOuterLock();
const binding=(await import(pathToFileURL(binderPath).href)).bindCombined(release,SOURCE_SHA);
assert.equal(binding.manifest.kind,'combined_source_v8');assert.equal(binding.manifest.source_commit,'e6b1fa2f87d0b39ac7f83aba6cdecc2110de5a50');assert.equal(binding.manifest.approved_product_bindings.length,17);assert.equal(binding.manifest.changed_parent_inputs,8);assert.equal(binding.manifest.new_input_files,9);assert.equal(binding.manifest.unchanged_parent_inputs,318);assert.match(binding.manifest.source_commit,/^[a-f0-9]{40}$/);
assert.equal(binding.manifest.files.length,335);assert.equal(binding.manifest.required_test_names,540);
assert.equal(binding.manifest.minimum_expected_workspace_tests,1322);assert(binding.actualWorkspace.passed>=1322);binding.verify();
for(const expected of preparation.source_contract_files){const actual=binding.manifest.files.find(row=>row.path===expected.path);assert(actual,expected.path);assert.equal(actual.sha256,expected.sha256,expected.path);assert.equal(actual.bytes,expected.bytes,expected.path);}
assert.equal(SHA(binding.suiteRaw),args['suite-proof-sha']);assert.equal(SHA(binding.buildRaw),args['build-proof-sha']);
assert.equal(binding.build.binary.sha256,args['candidate-binary-sha']);assert.equal(binding.binarySHA,args['candidate-binary-sha']);
assert(Number.isSafeInteger(binding.build.binary.bytes)&&binding.build.binary.bytes>0);
const space=fs.statfsSync(OWN);assert(space.bavail*space.bsize>=4*1024**3,'4 GiB owned scratch required for actual source/SQLite/CSV witnesses');
const {DatabaseSync}=await import('node:sqlite');
fs.mkdirSync(args.out,{recursive:true});
const FIXTURES=path.join(args.out,'fixture-witnesses'),DATA=path.join(args.out,'product-data');
fs.mkdirSync(FIXTURES);fs.mkdirSync(DATA);
function save(name,value){const full=path.join(args.out,name);assert(childOf(args.out,full));fs.writeFileSync(full,JSON.stringify(value,null,2)+'\n',{flag:'wx'});return full;}
save('inputs.json',{kind:'actual_genuine_large_sources_started',recorded_at:new Date().toISOString(),args,release,binary:binding.build.binary,source_manifest_sha256:SOURCE_SHA,suite_proof_sha256:SHA(binding.suiteRaw),build_proof_sha256:SHA(binding.buildRaw),limits:{xml_protocol_bytes:XML_LIMIT,entity_xml_work:WORK_CAP,xml_events:2_000_000,retained_entries:50_000,retained_text:33_554_432,membership_catalog:MEMBERSHIP_CAP,membership_catalog_default_unchanged:DEFAULT_MEMBERSHIP_CAP,public_json:PUBLIC_JSON_CAP,csv:CSV_CAP},scope:'Full-body XML files below fixture-witnesses are harness-owned wire witnesses, not product-retained source bodies.'});

const started=performance.now(),deadline=started+45*60_000;
// At most64 tiny observations from responses/operations already in this run.
// These stdout lines are captured by Root's retained outer pipe; they never
// constitute acceptance, extend a deadline, or request additional HTTP/SQL.
let progressRecords=0;
function progress(event,details={}){if(progressRecords>=64)return;progressRecords++;console.log(JSON.stringify({kind:'large_source_progress_observation_only',event,phase,...details,acceptance:false}));}
const failures=[],cases=[],runs=[],hits=[],apiReceipts=[],signals=[];
let app=null,site=null,origin=null,filterPattern=null,response503Info=null,siteClosed=false,phase='fixture preparation';
let finalBindingVerified=false,databaseBytesUnchanged=false;
const sockets=new Set(),active=new Set(),fixtureTasks=new Set();
const routes=new Map();
function gate(condition,message,details=null){if(!condition)failures.push({phase,message,details});return condition;}
function check(){assert(!signals.length,'Owned harness interrupted');assert(performance.now()<deadline,'Harness 45-minute observation budget exceeded');}
function lease(){check();assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed');assert(Date.now()<expiry,'Root execution lease expired');assert.equal(SHA(read(args.release)),args['release-sha']);verifyOuterLock();}
const onSignal=signal=>{signals.push(signal);};const onInt=()=>onSignal('SIGINT'),onTerm=()=>onSignal('SIGTERM');
process.on('SIGINT',onInt);process.on('SIGTERM',onTerm);

// An independently constrained ASCII fixture recognizer. It consumes actual file
// bytes, skips the actual complete comment, then validates every literal entry.
// It is deliberately NOT a general XML/XSD parser or a copy of the Rust recognizer.
async function inspectFixture(file){
  assert(origin&&filterPattern);const hash=createHash('sha256'),multiset=createHash('sha256'),retainedMultiset=createHash('sha256');let bytes=0,count=0,pending='',state='header',last=null;
  const header='<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n<!--';
  const close='</urlset>\n';
  for await(const chunk of fs.createReadStream(file,{highWaterMark:64*1024})){
    bytes+=chunk.length;hash.update(chunk);assert(chunk.every(byte=>byte<128),'ASCII fixture oracle profile');pending+=chunk.toString('ascii');
    for(;;){
      if(state==='header'){if(pending.length<header.length)break;assert(pending.startsWith(header));pending=pending.slice(header.length);state='comment';}
      else if(state==='comment'){const end=pending.indexOf('-->\n');if(end<0){let retained=0;for(let length=1;length<=3;length++)if(pending.endsWith('-->\n'.slice(0,length)))retained=length;assert(/^x*$/.test(pending.slice(0,pending.length-retained)));pending=retained?pending.slice(-retained):'';break;}assert(/^x*$/.test(pending.slice(0,end)));pending=pending.slice(end+4);state='entries';}
      else if(state==='entries'){const newline=pending.indexOf('\n');if(newline<0)break;const line=pending.slice(0,newline+1);pending=pending.slice(newline+1);if(line===close){state='done';continue;}const match=/^<url><loc>(http:\/\/127\.0\.0\.1:[1-9][0-9]*\/p\/[0-9]{5})<\/loc><\/url>\n$/.exec(line);assert(match,'Exact actual owned-origin fixture entry shape');assert.equal(match[1],origin+'/p/'+String(count).padStart(5,'0'));assert(new RegExp(filterPattern).test(match[1]));last=match[1];multiset.update(match[1]+'\n');if(count<50_000)retainedMultiset.update(match[1]+'\n');count++;}
      else{assert.equal(pending,'');break;}
    }
    assert(pending.length<8192,'Bounded fixture oracle carry');
  }
  assert.equal(state,'done');assert.equal(pending,'');return {bytes,sha256:hash.digest('hex'),eligible_entries:count,loc_multiset_sha256:multiset.digest('hex'),first_50000_loc_multiset_sha256:retainedMultiset.digest('hex'),last_loc:last,oracle:'independent constrained ASCII stream recognition of delivered source file'};
}
function writeAll(fd,bytes){let offset=0;while(offset<bytes.length){const amount=fs.writeSync(fd,bytes,offset,bytes.length-offset);assert(amount>0);offset+=amount;}}
async function generate(label,count,exactBytes=null){
  lease();const file=path.join(FIXTURES,label+'.xml'),fd=fs.openSync(file,'wx');
  const header=Buffer.from('<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n<!--');
  const tail=Buffer.from('-->\n'+Array.from({length:count},(_,index)=>'<url><loc>'+origin+'/p/'+String(index).padStart(5,'0')+'</loc></url>\n').join('')+'</urlset>\n');
  const filler=exactBytes===null?0:exactBytes-header.length-tail.length;assert(filler>=0);
  try{writeAll(fd,header);let remaining=filler;const block=Buffer.alloc(64*1024,0x78);while(remaining){const amount=Math.min(remaining,block.length);writeAll(fd,block.subarray(0,amount));remaining-=amount;}writeAll(fd,tail);fs.fsyncSync(fd);}finally{fs.closeSync(fd);}
  const witness=await inspectFixture(file);assert.equal(witness.eligible_entries,count);if(exactBytes!==null)assert.equal(witness.bytes,exactBytes);
  return {file,...witness};
}
async function gzipFile(source,label){lease();const file=path.join(FIXTURES,label+'.gz');await pipeline(fs.createReadStream(source.file),createGzip({level:1}),fs.createWriteStream(file,{flags:'wx'}));const compressed=await hashFile(file),hash=createHash('sha256');let bytes=0;const reader=fs.createReadStream(file),unzip=createGunzip(),readerClose=actualStreamClose(reader),unzipClose=actualStreamClose(unzip);reader.pipe(unzip);try{for await(const chunk of unzip){bytes+=chunk.length;hash.update(chunk);}}finally{reader.destroy();unzip.destroy();await Promise.all([readerClose,unzipClose]);}assert.equal(bytes,source.bytes);assert.equal(hash.digest('hex'),source.sha256);return {file,...compressed,decoded:source};}
async function corruptCrc(gzip){const file=path.join(FIXTURES,'bad-crc.gz');fs.copyFileSync(gzip.file,file,fs.constants.COPYFILE_EXCL);const fd=fs.openSync(file,'r+');try{const byte=Buffer.alloc(1);assert.equal(fs.readSync(fd,byte,0,1,gzip.bytes-8),1);byte[0]^=1;assert.equal(fs.writeSync(fd,byte,0,1,gzip.bytes-8),1);fs.fsyncSync(fd);}finally{fs.closeSync(fd);}const reader=fs.createReadStream(file),unzip=createGunzip(),readerClose=actualStreamClose(reader),unzipClose=actualStreamClose(unzip);reader.pipe(unzip);let outputBytes=0,failure=null;try{for await(const chunk of unzip)outputBytes+=chunk.length;}catch(error){failure={code:error.code,message:error.message};}finally{reader.destroy();unzip.destroy();await Promise.all([readerClose,unzipClose]);}assert.equal(failure?.code,'Z_DATA_ERROR','Independent native gzip CRC negative must actually reject');return {file,...await hashFile(file),decoded:gzip.decoded,deliberate_crc_corruption:true,independent_decode:{outputBytes,failure}};}
async function request(route,{body,max=PUBLIC_JSON_CAP,saveAs=null,timeout=180_000}={}){
  check();assert(app?.origin&&!app.run.actual_close_event);const url=new URL(route,app.origin);assert.equal(url.origin,app.origin);assert(![8787,8790].includes(Number(url.port)));
  const payload=body===undefined?undefined:Buffer.from(JSON.stringify(body));assert(!payload||payload.length<=64*1024*1024);
  const response=await fetch(url,{redirect:'error',signal:AbortSignal.timeout(timeout),...(payload?{method:'POST',headers:{'content-type':'application/json'},body:payload}:{})});
  const hash=createHash('sha256'),parts=[];let bytes=0;const file=saveAs?path.join(args.out,saveAs):null;const fd=file?fs.openSync(file,'wx'):null;
  try{for await(const chunk of response.body){check();bytes+=chunk.length;assert(bytes<=max,'Bounded native response: '+route);hash.update(chunk);if(fd!==null)writeAll(fd,chunk);else parts.push(Buffer.from(chunk));}if(fd!==null)fs.fsyncSync(fd);}finally{if(fd!==null)fs.closeSync(fd);}
  const result={route,status:response.status,headers:Object.fromEntries(response.headers),bytes,sha256:hash.digest('hex'),normal_eof:true,file};apiReceipts.push(result);return {...result,body:fd===null?Buffer.concat(parts):null};
}
function decode(response){assert.equal(response.status,200,JSON.stringify({route:response.route,status:response.status,body:response.body?.toString().slice(0,2000)}));return JSON.parse(response.body);}
function observePipe(stream,fd,record,name,onChunk){const state={end:false,close:false,error:null,bytes:0};record[name]=state;stream.on('data',chunk=>{state.bytes+=chunk.length;if(state.bytes>16*1024*1024&&!state.error)state.error='Log cap exceeded';try{writeAll(fd,chunk);onChunk?.(chunk);}catch(error){state.error=String(error.stack??error);}});stream.once('end',()=>{state.end=true;});stream.once('close',()=>{state.close=true;});stream.once('error',error=>{state.error=String(error);stream.resume();});}
async function start(label){
  assert.equal(app,null);lease();binding.verify();const stdoutPath=path.join(args.out,label+'.stdout.log'),stderrPath=path.join(args.out,label+'.stderr.log');
  const stdout=fs.openSync(stdoutPath,'wx'),stderr=fs.openSync(stderrPath,'wx');
  const run={label,executable:binding.binary,args:['serve','--listen','127.0.0.1:0','--data-dir',DATA,'--shutdown-on-stdin-eof'],pid:null,spawn_event:false,actual_exit_event:false,actual_close_event:false,spawn_error:null,exit:null,close_exit:null,termination_requested:[],forced_escalation:false,stdin:{eof_sent:false,finished:false,closed:false,error:null},shutdown_receipts:[],stdout:{end:false,close:false,error:null,bytes:0},stderr:{end:false,close:false,error:null,bytes:0}};runs.push(run);
  let child;try{child=spawn(binding.binary,run.args,{cwd:SOURCE_ROOT,windowsHide:true,stdio:['pipe','pipe','pipe'],env:{...process.env,TEMP:args.out,TMP:args.out}});}catch(error){run.spawn_error=String(error.stack??error);fs.closeSync(stdout);fs.closeSync(stderr);throw error;}
  const current={child,run,origin:null,closed:null};app=current;run.pid=child.pid;
  child.stdin.once('finish',()=>{run.stdin.finished=true;});child.stdin.once('close',()=>{run.stdin.closed=true;});child.stdin.once('error',error=>{run.stdin.error=String(error);});
  current.closed=new Promise(resolve=>child.once('close',(code,signal)=>{run.actual_close_event=true;run.close_exit={code,signal};resolve();}));
  child.once('spawn',()=>{run.spawn_event=true;});child.once('error',error=>{run.spawn_error=String(error.stack??error);});child.once('exit',(code,signal)=>{run.actual_exit_event=true;run.exit={code,signal};});
  let lineBuffer='';observePipe(child.stdout,stdout,run,'stdout',chunk=>{lineBuffer+=chunk.toString('utf8');assert(lineBuffer.length<=256*1024,'Bounded bind line buffer');let newline;while((newline=lineBuffer.indexOf('\n'))>=0){const line=lineBuffer.slice(0,newline);lineBuffer=lineBuffer.slice(newline+1);const match=/^Open SEO Spider: (http:\/\/127\.0\.0\.1:([1-9][0-9]*))\r?$/.exec(line);if(match){assert(!current.origin);assert(![8787,8790].includes(Number(match[2])));current.origin=match[1];run.origin=match[1];}}});let stderrPending='';observePipe(child.stderr,stderr,run,'stderr',chunk=>{stderrPending+=chunk.toString('utf8');assert(stderrPending.length<=256*1024,'Bounded final receipt line');let newline;while((newline=stderrPending.indexOf('\n'))>=0){const line=stderrPending.slice(0,newline).replace(/\r$/,'');stderrPending=stderrPending.slice(newline+1);if(!line.startsWith('{'))continue;let value;try{value=JSON.parse(line);}catch{continue;}if(value.kind==='owned_application_shutdown_v1')run.shutdown_receipts.push({value,before_actual_close:!run.actual_close_event});}});
  current.finishLogs=async()=>{for(const [name,fd,file] of [['stdout',stdout,stdoutPath],['stderr',stderr,stderrPath]]){try{fs.fsyncSync(fd);}finally{fs.closeSync(fd);}Object.assign(run[name],await hashFile(file));run[name].file=file;}};
  const until=performance.now()+60_000;while(!current.origin&&performance.now()<until){check();assert(!run.spawn_error&&!run.actual_close_event,'Owned app failed before bind');await delay(50);}assert(current.origin,'Owned app bind receipt missing');assert.equal((await request('/healthz',{max:1024})).status,200);
}
async function stop(){
  if(!app)return;const current=app;const run=current.run;
  if(!run.actual_close_event&&!run.actual_exit_event){run.termination_requested.push('OWNED_STDIN_EOF');run.stdin.eof_sent=true;current.child.stdin.end();}
  let waited=0;while(!run.actual_close_event){await Promise.race([current.closed,delay(1000)]);waited++;if(waited===15&&!run.actual_close_event){run.forced_escalation=true;run.termination_requested.push('SIGKILL');current.child.kill('SIGKILL');}}
  await current.closed;await current.finishLogs();app=null;
  assert(run.spawn_event&&run.actual_exit_event&&run.actual_close_event&&!run.spawn_error&&run.pid>0);
  assert.deepEqual(run.close_exit,run.exit);for(const name of ['stdout','stderr'])assert(run[name].end&&run[name].close&&!run[name].error,'Actual owned pipe drain '+name);
  assert(!run.forced_escalation,'Escalated termination is not accepted');
  assert.deepEqual(run.exit,{code:0,signal:null});assert.deepEqual(run.stdin,{eof_sent:true,finished:true,closed:true,error:null});assert.equal(run.shutdown_receipts.length,1);assert.equal(run.shutdown_receipts[0].before_actual_close,true);assertLargeReceipt(run.shutdown_receipts[0].value,run.label==='capture-app'?13:0);
}
function assertIncomingTransport(receipt){
  assert.equal(receipt.response_transport_owned,true);
  const t=receipt.incoming_transports;assert(t&&typeof t==='object','actual incoming transport receipt required');
  assert.equal(t.incoming_protocol,'http1_plain_tcp_no_application_upgrades');assert.equal(t.grace_seconds,5);assert.equal(t.registered,true);assert.equal(t.listener_dropped,true);
  assert.equal(t.first_failure,null);assert.equal(t.whole_application_five_second_bound,false);
  for(const k of ['accepted','rejected','retained','complete','peer_errors','shutdown_aborted','actual_io_dropped','tcp_write_pending','accept_admitted','accept_joined','accept_retained','connection_admitted','connection_joined','connection_retained'])assert(Number.isSafeInteger(t[k])&&t[k]>=0,'actual nonnegative transport counter '+k);
  assert(t.accepted>0);assert.equal(t.accept_admitted,1);assert.equal(t.accept_joined,1);
  for(const k of ['retained','accept_retained','connection_retained'])assert.equal(t[k],0);
  assert.equal(t.connection_admitted,t.connection_joined);assert.equal(t.accepted,t.connection_admitted+t.rejected);assert.equal(t.actual_io_dropped,t.accepted);
  assert.equal(t.complete+t.peer_errors+t.shutdown_aborted,t.connection_admitted);
  // Historical tcp_write_pending and expected client PeerError are not live retained work.
}
function assertLargeReceipt(receipt,expectedCrawls){
  assert.equal(receipt.clean,true);assert.equal(receipt.cause,'owned_stdin_eof');assertIncomingTransport(receipt);
  assert.deepEqual(receipt.owners.map(row=>row.owner).sort(),['http_requests','http_blocking','crawl_operations'].sort());
  for(const owner of receipt.owners){assert.equal(owner.closing,true);assert.equal(owner.retained,0);assert.equal(owner.first_failure,null);assert.equal(owner.admitted,owner.started);assert.equal(owner.started,owner.joined);}assert.equal(receipt.owners.find(row=>row.owner==='crawl_operations').joined,expectedCrawls);assert(receipt.owners.find(row=>row.owner==='http_requests').joined>0);assert(receipt.owners.find(row=>row.owner==='http_blocking').joined>0);
  // These unchanged thirteen configurations omit archive, policy and resources.
  // Their CSV routes use state.exports, not the new response_streams registry.
  const bodies=receipt.source_body_transports;assert(bodies&&bodies.owner==='source_body_transports');for(const key of ['admitted','started','joined','retained','spool_leases']){assert(Object.hasOwn(bodies,key),key);assert.equal(bodies[key],0);}assert.equal(bodies.logical_spool_byte_limit,134217728);assert.equal(bodies.first_failure,null);assert.equal(bodies.unresponsive_transport_shutdown,'not_proven');
  const responses=receipt.response_streams;assert(responses&&typeof responses==='object');for(const key of ['admitted','started','joined','retained','complete','aborted','failed','enqueues','full_reservations','max_chunk','peak_queued','response_supervisors_joined']){assert(Object.hasOwn(responses,key),key);assert.equal(responses[key],0);}assert.equal(responses.first_failure,null);
}
async function terminal(id){const until=performance.now()+15*60_000;let value;while(performance.now()<until){check();value=decode(await request('/api/crawls/'+id));if(['completed','failed','cancelled','limited','interrupted'].includes(value.status))return value;await delay(500);}assert.fail('Actual crawl did not reach a terminal observation state');}
const CSV_HEADER=['kind','sequence','observation_id','occurrence_id','source_ordinal','raw_value','membership_url','admitted_fetch_url','admission_decision','typed_row_json'];
function csvSafe(value){const trimmed=value.replace(/^[\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000\ufeff]*/u,'');return /^[=+@-]/u.test(trimmed)?"'"+value:value;}
function rawFields(tagged,raw){const row=tagged.row,kind=tagged.kind;let id='',source='',order='',value='',member='',admitted='',decision='';if(kind==='root'){id=row.declaration_id;source=row.source_observation_id??'';order=String(row.source_ordinal);value=row.raw_value;member=row.admission.membership_url??'';admitted=row.admission.admitted_fetch_url??'';decision=row.admission.decision;}else if(kind==='document'){id=row.observation_id;source=id;}else if(kind==='entry'){id=row.entry_id;source=row.observation_id;order=String(row.observed.ordinal);}else if(kind==='location'){id=row.occurrence_id;source=row.observation_id;order=String(row.observed.ordinal);value=row.observed.decoded_value??'';member=row.admission.membership_url??'';admitted=row.admission.admitted_fetch_url??'';decision=row.admission.decision;}else assert.fail('Raw row kind');return [kind,String(row.sequence),source,id,order,value,member,admitted,decision,raw].map((value,index)=>index===1||index===4?value:csvSafe(value));}
function readDatabase(database,kind='all'){
  const db=new DatabaseSync(database,{readOnly:true});try{
    const metadata=Object.fromEntries(db.prepare('SELECT key,value FROM metadata ORDER BY key').all().map(row=>[row.key,row.value]));
    const sourceRows=db.prepare('SELECT sequence,measurement_id,task_url,data,data_sha256 FROM sitemap_source_measurements ORDER BY sequence').all();
    for(const record of sourceRows){assert.equal(SHA(record.data),record.data_sha256);const envelope=JSON.parse(record.data);assert.equal(JSON.stringify(envelope),record.data);assert.equal(envelope.sequence,record.sequence);assert.equal(envelope.measurement.measurement_id,record.measurement_id);assert.equal(envelope.measurement.payload.binding.requested_task_url,record.task_url);}
    const commits=db.prepare('SELECT document_id,measurement_id,data,data_sha256 FROM sitemap_source_measurement_commits ORDER BY document_sequence').all();for(const record of commits)assert.equal(SHA(record.data),record.data_sha256);
    const documents=db.prepare('SELECT row_id,task_url,data,data_sha256 FROM sitemap_documents ORDER BY sequence').all();for(const record of documents){assert.equal(SHA(record.data),record.data_sha256);const tagged=JSON.parse(record.data);assert.equal(tagged.kind,'document');assert.equal(tagged.row.observation_id,record.row_id);assert.equal(tagged.row.task_url,record.task_url);}
    for(const record of commits){const association=JSON.parse(record.data),source=sourceRows.find(row=>row.measurement_id===record.measurement_id),document=documents.find(row=>row.row_id===record.document_id);assert(source&&document);assert.equal(association.measurement_id,source.measurement_id);assert.equal(association.document_id,document.row_id);assert.equal(association.measurement_envelope_sha256,source.data_sha256);assert.equal(association.document_payload_sha256,document.data_sha256);}
    const counts=Object.fromEntries(['roots','documents','entries','locations'].map(table=>[table,db.prepare('SELECT count(*) n FROM sitemap_'+table).get().n]));
    const sql="SELECT 'root' kind,sequence,data,data_sha256 FROM sitemap_roots UNION ALL SELECT 'document',sequence,data,data_sha256 FROM sitemap_documents UNION ALL SELECT 'entry',sequence,data,data_sha256 FROM sitemap_entries UNION ALL SELECT 'location',sequence,data,data_sha256 FROM sitemap_locations ORDER BY sequence";
    const catalogPrefix='{"schema_version":1,"summary":'+metadata.sitemap_ledger+',"inherited":null,"rows":[';
    assert(!Object.hasOwn(metadata,'sitemap_inherited_basis'),'Fresh capture must not claim inherited rows');
    const catalogHash=createHash('sha256');catalogHash.update(catalogPrefix);
    const decisions={};let catalogSerializedBytes=Buffer.byteLength(catalogPrefix),catalogRows=0;
    const hash=createHash('sha256'),locationsHash=createHash('sha256');let csvBytes=0,rows=0,locationRows=0,lastLocation=null,previous=0,catalogRowBytes=0;
    const feed=fields=>{const text=fields.map(value=>'"'+value.replaceAll('"','""')+'"').join(',')+'\r\n';csvBytes+=Buffer.byteLength(text);hash.update(text);};feed(CSV_HEADER);
    for(const record of db.prepare(sql).iterate()){
      if(catalogRows){catalogHash.update(',');catalogSerializedBytes++;}catalogHash.update(record.data);catalogSerializedBytes+=Buffer.byteLength(record.data);catalogRows++;
      const tagged=JSON.parse(record.data);assert.equal(JSON.stringify(tagged),record.data);assert.equal(SHA(record.data),record.data_sha256);assert.equal(tagged.kind,record.kind);assert.equal(tagged.row.sequence,record.sequence);assert(record.sequence>previous);previous=record.sequence;catalogRowBytes+=Buffer.byteLength(record.data);
      if(record.kind==='location'){locationRows++;lastLocation=tagged.row.observed.decoded_value;assert.equal(lastLocation,origin+'/p/'+String(locationRows-1).padStart(5,'0'));assert.equal(tagged.row.admission.membership_url,lastLocation);assert.equal(tagged.row.admission.admitted_fetch_url,null);decisions[tagged.row.admission.decision]=(decisions[tagged.row.admission.decision]??0)+1;locationsHash.update(lastLocation+'\n');}
      if(kind==='all'||kind===record.kind){feed(rawFields(tagged,record.data));rows++;}
    }
    const config=JSON.parse(metadata.config),summary=JSON.parse(metadata.sitemap_ledger),sourceSummary=JSON.parse(metadata.sitemap_source_store);
    catalogHash.update(']}');catalogSerializedBytes+=2;
    assert.deepEqual(config.sitemap_membership_limits,MEMBERSHIP_LIMITS);assert.deepEqual(summary.limits,MEMBERSHIP_LIMITS);
    assert(summary.catalog_json_bytes<=MEMBERSHIP_CAP);assert.equal(summary.catalog_json_bytes,catalogSerializedBytes,'Actual stored canonical envelope/rows byte count must match production accounting');
    const catalogSerialized={bytes:catalogSerializedBytes,sha256:catalogHash.digest('hex'),actual_row_json_bytes:catalogRowBytes,actual_rows:catalogRows,declared_capacity:MEMBERSHIP_CAP,above_unchanged_default:catalogSerializedBytes>DEFAULT_MEMBERSHIP_CAP,method:'Actual SQL canonical row TEXT + actual saved summary + exact fresh SitemapCatalog Serde field order; not a symbolic floor or fabricated fixture'};
    return {metadata,config,summary,sourceSummary,sourceRows,commits,documents,counts,csv:{kind,bytes:csvBytes,sha256:hash.digest('hex'),rows},locations:{count:locationRows,last:lastLocation,multiset_sha256:locationsHash.digest('hex')},catalog_row_json_bytes:catalogRowBytes,catalog_serialized:catalogSerialized,admission_decisions:decisions,measurement:sourceRows[0]?JSON.parse(sourceRows[0].data):null};
  }finally{db.close();}
}
function parseQuotedCsv(bytes){const text=bytes.toString('utf8'),rows=[];let row=[],field='',quoted=false,after=false;for(let index=0;index<text.length;index++){const character=text[index];if(quoted){if(character==='"'){if(text[index+1]==='"'){field+='"';index++;}else{quoted=false;after=true;}}else field+=character;}else if(character==='"'){assert(!field.length&&!after);quoted=true;}else if(character===','){row.push(field);field='';after=false;}else if(character==='\r'){assert.equal(text[++index],'\n');row.push(field);rows.push(row);row=[];field='';after=false;}else assert.fail('CSV fixture parser requires all fields quoted');}assert(!quoted&&!field.length&&!row.length);return rows;}
async function findings(record,label){
  const results={};for(const filter of ['xml-sitemap-with-over-50k-urls','xml-sitemap-over-50mb']){
    const query={filter,assessment:'all',source_task_url:record.task,limit:200,candidate_limit:5000};
    const response=await request('/api/crawls/'+record.id+'/sitemaps/findings',{body:query});const value=decode(response);
    assert.equal(value.website_requests,0);assert.equal(value.findings?.scan.exhausted,true);assert.equal(value.next_cursor,null);assert.equal(value.findings.rows.length,1);
    const row=value.findings.rows[0];assert.equal(row.evidence.domain,'source_attempt');assert.equal(row.evidence.source.kind,'measured');
    assert.deepEqual(row.evidence.source.record,record.sql.measurement);assert.equal(row.evidence.source.record.measurement.payload.binding.requested_task_url,record.task);
    assert.equal(row.evidence.source.association!==null,record.sql.commits.length===1);assert.equal(row.evidence.source.committed_document!==null,record.sql.documents.length===1);
    const protocol=row.evidence.source.protocol,entry=protocol.entry_limit,byte=protocol.decoded_xml_byte_limit;
    assert.equal(entry.threshold,50_000);assert.equal(byte.threshold,XML_LIMIT);
    assert.equal(row.assessment.verdict,filter==='xml-sitemap-with-over-50k-urls'?record.expectedEntry:record.expectedByte);
    assert.equal(entry.verdict,{match:'observed_exceeded',not_matched:'within_limit',unknown:'unknown'}[record.expectedEntry]);
    assert.equal(byte.verdict,{match:'observed_exceeded',not_matched:'within_limit',unknown:'unknown'}[record.expectedByte]);
    const csv=await request('/api/crawls/'+record.id+'/sitemaps/findings/export.csv',{body:{...query,snapshot:value.export_token,limit:undefined,candidate_limit:undefined},max:CSV_CAP});assert.equal(csv.status,200);
    const parsed=parseQuotedCsv(csv.body);assert.equal(parsed.length,3);assert.equal(parsed[0].length,10);assert.equal(parsed[1][0],'context');assert.equal(parsed[2][0],'finding');assert.deepEqual(JSON.parse(parsed[2][8]),row);assert.deepEqual(JSON.parse(parsed[1][9]).context,value.findings.context);
    const prefix=label+'.'+filter;save(prefix+'.api.json',value);fs.writeFileSync(path.join(args.out,prefix+'.csv'),csv.body,{flag:'wx'});
    results[filter]={json_sha256:response.sha256,csv_sha256:csv.sha256,csv_bytes:csv.bytes,measurement_id:row.evidence.source.record.measurement.measurement_id,verdict:row.assessment.verdict,protocol};
  }return results;
}
async function rawRead(record,label,{requiredCount=null}={}){
  let cursor=null,page=0,total=0,first=null,last=null;const multiset=createHash('sha256');const pages=[];
  do{assert(page<1000);const query=new URLSearchParams({kind:'location',limit:'200',page_limit:'0'});if(cursor)query.set('cursor',cursor);const response=await request('/api/crawls/'+record.id+'/sitemaps?'+query);const value=decode(response);assert.equal(value.website_requests,0);assert(value.catalog_available);assert(Array.isArray(value.rows)&&value.rows.length<=200);assert.equal(value.summary.location_rows,record.sql.counts.locations);if(!first)first=value;else assert.deepEqual(value.snapshot,first.snapshot);for(const row of value.rows){assert.equal(row.kind,'location');total++;last=row.row.observed.decoded_value;multiset.update(last+'\n');}pages.push({page,rows:value.rows.length,sha256:response.sha256,has_more:value.has_more});cursor=value.next_cursor;assert.equal(Boolean(cursor),value.has_more);page++;if(requiredCount===50_000&&page%25===0)progress('verified_50k_page_windows',{label,pages:page,rows:total});}while(cursor);
  if(requiredCount===50_000){assert.equal(pages.length,250,'Full50000 positive needs250 actual200-row windows');assert(pages.every(window=>window.rows===200));}
  const actualHash=multiset.digest('hex');assert.equal(total,record.sql.locations.count);assert.equal(actualHash,record.sql.locations.multiset_sha256);assert.equal(last,record.sql.locations.last);if(requiredCount!==null)gate(total===requiredCount,'Required complete actual source membership not retained',{label,requiredCount,actual:total,terminal_status:record.terminal.status});
  const decodeComplete=record.sql.measurement?.measurement.payload.decode.integrity==='complete';
  if(decodeComplete&&record.sql.commits.length===1&&record.sql.measurement.measurement.payload.membership_retention==='complete')assert.equal(first.coverage.derived.completion,'complete','Complete retained source needs qualified complete declared-source coverage');
  else assert.equal(first.coverage.derived.completion,'unknown','Failed/unobserved source must not manufacture known-empty coverage');
  const expected=readDatabase(record.database,'location').csv;
  const query=new URLSearchParams({kind:'location',snapshot:first.snapshot_token});const csv=await request('/api/crawls/'+record.id+'/sitemaps/export.csv?'+query,{max:CSV_CAP,saveAs:label+'.all-stored-location.csv'});
  assert.equal(csv.status,200,'Complete location CSV required below unchanged cap');assert.equal(csv.bytes,expected.bytes);assert.equal(csv.sha256,expected.sha256);assert.equal(csv.headers['x-sitemap-csv-sha256'],expected.sha256);assert.equal(Number(csv.headers['content-length']),expected.bytes);assert(csv.normal_eof);
  if(total>200){assert(pages.length>1);assert(pages.at(-1).rows>0);}
  const result={pages,total,last,multiset_sha256:actualHash,csv:expected,normal_eof:csv.normal_eof,actual_final_tail_after_200:total>200&&pages.length>1,coverage:first.coverage.derived};save(label+'.paging.json',result);return result;
}
async function inspectCase(spec,label){
  const record=cases.find(row=>row.label===spec.label);assert(record);record.sql=readDatabase(record.database);
  const saved=record.sql;assert.deepEqual(saved.config.sitemap_source_processing??null,spec.legacy?null:{max_post_http_entity_bytes:WORK_CAP,max_decoded_xml_bytes:WORK_CAP,max_xml_events:2_000_000});
  assert.deepEqual(saved.config.sitemap_membership_limits,MEMBERSHIP_LIMITS);assert.deepEqual(saved.config.exclude_patterns,[filterPattern]);assert.equal(saved.config.max_pages,1);assert.equal(saved.config.check_external,false);assert.equal(saved.config.crawl_links,false);
  const denied=new RegExp(filterPattern);assert(denied.test(origin+'/p/00000')&&denied.test(origin+'/p/50000'));assert(!denied.test(record.task)&&!denied.test(origin+'/seed'));
  assert.equal(saved.sourceRows.length,1,'Actual source A must persist');assert.equal(saved.sourceSummary.record_count,1);assert.equal(saved.sourceSummary.commit_count,saved.commits.length);assert.equal(saved.sourceSummary.omission,null);
  assert(saved.measurement);const measurement=saved.measurement.measurement.payload;assert.equal(measurement.transport.request_started,true);assert.equal(measurement.transport.requested_url,record.task);assert.equal(measurement.transport.final_url,record.task);assert.equal(measurement.transport.final_status,spec.status??200);
  assert.equal(measurement.bounds.max_retained_post_http_bytes,spec.legacy?PUBLIC_JSON_CAP:WORK_CAP);assert.equal(measurement.bounds.max_retained_xml_bytes,spec.legacy?PUBLIC_JSON_CAP:WORK_CAP);assert.equal(measurement.bounds.max_xml_events,spec.legacy?500_000:2_000_000);assert.equal(measurement.bounds.max_retained_entries,50_000);assert.equal(measurement.bounds.max_retained_location_occurrences,100_000);assert.equal(measurement.bounds.max_retained_text_utf8_bytes,33_554_432);assert.equal(measurement.bounds.max_field_utf8_bytes,8192);assert.equal(measurement.bounds.max_xml_depth,64);
  if(spec.completeDecode){assert.equal(measurement.transport.body_complete,true);assert.equal(measurement.decode.integrity,'complete');assert.deepEqual(measurement.decode.decoded_xml_bytes,{value:spec.xml.bytes,precision:'exact'});assert.equal(measurement.decode.decoded_complete_sha256,spec.xml.sha256);assert.equal(measurement.scan.status,'complete_profile');assert.deepEqual(measurement.scan.counts.protocol_eligible_completed_entries,{value:spec.xml.eligible_entries,precision:'exact'});assert.deepEqual(measurement.scan.counts.completed_immediate_entries,{value:spec.xml.eligible_entries,precision:'exact'});assert.equal(measurement.decode.encoding,spec.fileGzip?'gzip_file_entity':'identity');}
  else{assert.equal(measurement.scan.status,'not_observed');assert.deepEqual(measurement.scan.counts.protocol_eligible_completed_entries,{value:null,precision:'unavailable'});assert.equal(measurement.decode.decoded_complete_sha256,null);if(spec.legacy){assert.equal(measurement.transport.body_complete,false);assert.equal(measurement.transport.retained_post_http_prefix_bytes,PUBLIC_JSON_CAP);assert.equal(measurement.transport.post_http_received_bytes.precision,'at_least');assert(measurement.transport.post_http_received_bytes.value>PUBLIC_JSON_CAP);assert.deepEqual(measurement.decode.decoded_xml_bytes,{value:null,precision:'unavailable'});}else if(spec.badCrc){assert.equal(measurement.transport.body_complete,true);assert.equal(measurement.decode.integrity,'failed');assert.equal(measurement.decode.stop_code,'invalid_gzip');assert(measurement.decode.decoded_xml_bytes.precision!=='exact');}else{assert.equal(measurement.decode.integrity,'not_observed');assert.deepEqual(measurement.decode.decoded_xml_bytes,{value:null,precision:'unavailable'});}}
  if(spec.completeDecode){const expectedPost=spec.fileGzip?spec.wire.bytes:spec.xml.bytes;assert.deepEqual(measurement.transport.post_http_received_bytes,{value:expectedPost,precision:'exact'});assert.equal(measurement.transport.retained_post_http_prefix_bytes,expectedPost);assert.equal(measurement.transport.complete_post_http_entity_sha256,spec.fileGzip?spec.wire.sha256:spec.xml.sha256);assert.equal(measurement.membership_retention,spec.xml.eligible_entries>50_000?'incomplete':'complete');if(saved.documents.length===1){const document=JSON.parse(saved.documents[0].data).row;assert.equal(document.entity_decode.decoded_xml_bytes,spec.xml.bytes);assert.equal(document.entity_decode.decoded_xml_sha256,spec.xml.sha256);assert.equal(document.transport.body_complete,true);}}
  record.findings??={};record.findings[label]=await findings(record,label);
  if(!spec.isCountBoundary){assert.equal(record.terminal.status,'completed');assert.equal(saved.commits.length,1);assert.equal(saved.documents.length,1);record.paging??={};record.paging[label]=await rawRead(record,label,{requiredCount:spec.completeDecode?spec.xml.eligible_entries:0});if(spec.completeDecode&&spec.xml.eligible_entries===257)assert(record.paging[label].actual_final_tail_after_200);}
  else if(spec.xml.eligible_entries!==50_000){
    assert.equal(record.terminal.status,'completed');assert.equal(saved.commits.length,1);assert.equal(saved.documents.length,1);
    const expectedRetained=Math.min(50_000,spec.xml.eligible_entries);assert.equal(saved.counts.entries,expectedRetained);assert.equal(saved.counts.locations,expectedRetained);
    assert.equal(saved.locations.multiset_sha256,spec.xml.first_50000_loc_multiset_sha256);
    const expectedDecision=spec.xml.eligible_entries>50_000?'incomplete_source':'filter_denied';assert.deepEqual(saved.admission_decisions,{[expectedDecision]:expectedRetained});
  }
  else if(spec.xml.eligible_entries===50_000){
    gate(record.terminal.status==='completed'&&saved.commits.length===1&&saved.documents.length===1&&saved.counts.entries===50_000&&saved.counts.locations===50_000,'Genuine 50,000-URL durable positive is required; count-only success or budget rollback does not satisfy it',{terminal:record.terminal.status,error:record.terminal.error,counts:saved.counts,source_commits:saved.commits.length,source_captured_count:measurement.scan.counts.protocol_eligible_completed_entries});
    if(saved.counts.locations===50_000){assert.equal(saved.locations.multiset_sha256,spec.xml.loc_multiset_sha256);assert.deepEqual(saved.admission_decisions,{filter_denied:50_000});assert(saved.catalog_serialized.above_unchanged_default);record.paging??={};record.paging[label]=await rawRead(record,label,{requiredCount:50_000});}
    else assert.equal(saved.commits.length,0,'Failed B must remain unassociated, not inferred from A');
  }
  save(label+'.sql-inspection.json',{counts:saved.counts,summary:saved.summary,sourceSummary:saved.sourceSummary,measurement:saved.measurement,commits:saved.commits,documents:saved.documents,locations:saved.locations,catalog_row_json_bytes:saved.catalog_row_json_bytes,catalog_serialized:saved.catalog_serialized,admission_decisions:saved.admission_decisions,config_sha256:SHA(saved.metadata.config)});
  return record;
}
async function snapshotData(){const result=[];for(const name of fs.readdirSync(DATA).sort()){const file=path.join(DATA,name),stat=fs.lstatSync(file);assert(!stat.isSymbolicLink());if(stat.isDirectory()){const walk=async(directory,prefix)=>{for(const child of fs.readdirSync(directory).sort()){const full=path.join(directory,child),info=fs.lstatSync(full);assert(!info.isSymbolicLink());if(info.isDirectory())await walk(full,prefix+'/'+child);else{assert(info.isFile());result.push({path:prefix+'/'+child,...await hashFile(full)});}}};await walk(file,name);}else{assert(stat.isFile());result.push({path:name,...await hashFile(file)});}}return result;}
async function closeFixture(){
  if(!site)return;site.closeIdleConnections();const closed=new Promise(resolve=>site.close(()=>{siteClosed=true;resolve();}));
  const until=performance.now()+10_000;while((active.size||sockets.size||fixtureTasks.size)&&performance.now()<until){site.closeIdleConnections();await delay(25);}
  if(active.size||sockets.size||fixtureTasks.size){failures.push({phase:'fixture cleanup',message:'Forced owned connection cleanup was necessary'});site.closeAllConnections();}
  await Promise.allSettled([...fixtureTasks]);await closed;assert.equal(active.size,0);assert.equal(sockets.size,0);assert(hits.every(hit=>hit.closed&&hit.stream_closed),'Actual response and fixture file-stream closes');
}

async function startFixture(){
  lease();
  site=http.createServer((req,res)=>{
    const hit={ordinal:hits.length+1,method:req.method,path:req.url,finished:false,closed:false,stream_closed:false,bytes_emitted:0};hits.push(hit);active.add(hit);
    const spec=routes.get(req.url);let stream=null;
    res.once('finish',()=>{hit.finished=true;});res.once('close',()=>{hit.closed=true;active.delete(hit);stream?.destroy();});
    if(req.method!=='GET'||(!spec&&req.url!=='/seed')||hits.length>50){res.statusCode=404;res.end('unexpected fixture request');hit.stream_closed=true;return;}
    if(req.url==='/seed'){const body='<html><title>Owned source acceptance seed</title></html>';res.writeHead(200,{'content-type':'text/html','content-length':Buffer.byteLength(body)});res.end(body);hit.stream_closed=true;return;}
    const wire=spec.wire??response503Info;hit.expected_file_sha256=wire.sha256;hit.expected_file_bytes=wire.bytes;hit.expected_status=spec.status??200;
    res.writeHead(spec.status??200,{'content-type':'application/xml','content-length':wire.bytes,...(spec.httpGzip?{'content-encoding':'gzip'}:{})});
    stream=fs.createReadStream(wire.file);stream.on('data',chunk=>{hit.bytes_emitted+=chunk.length;});
    const task=new Promise(resolve=>stream.once('close',()=>{hit.stream_closed=true;fixtureTasks.delete(task);resolve();}));fixtureTasks.add(task);
    stream.once('error',error=>{hit.stream_error=String(error);res.destroy(error);});stream.pipe(res);
  });site.on('connection',socket=>{sockets.add(socket);socket.on('error',()=>{});socket.once('close',()=>sockets.delete(socket));});
  await new Promise((resolve,reject)=>{site.once('error',reject);site.listen(0,'127.0.0.1',resolve);});const port=site.address().port;assert(![8787,8790].includes(port));origin='http://127.0.0.1:'+port;
  filterPattern='^http://127[.]0[.]0[.]1:'+port+'/p/[0-9]{5}$';
  assert(new RegExp(filterPattern).test(origin+'/p/00000'));assert(!new RegExp(filterPattern).test(origin+'/seed'));
}

try{
  await startFixture();
  const count49999=await generate('count-49999',49_999),count50000=await generate('count-50000',50_000),count50001=await generate('count-50001',50_001);
  const exact=await generate('bytes-exact',257,XML_LIMIT),plus=await generate('bytes-plus-one',257,XML_LIMIT+1),empty=await generate('empty-known-zero',0);
  const exactGzip=await gzipFile(exact,'bytes-exact'),plusGzip=await gzipFile(plus,'bytes-plus-one'),badCrc=await corruptCrc(exactGzip);
  const specs=[
    {label:'01-count-49999',xml:count49999,wire:count49999,isCountBoundary:true,completeDecode:true,expectedEntry:'not_matched',expectedByte:'not_matched'},
    {label:'02-count-50000-required-positive',xml:count50000,wire:count50000,isCountBoundary:true,completeDecode:true,expectedEntry:'not_matched',expectedByte:'not_matched'},
    {label:'03-count-50001-exceeded',xml:count50001,wire:count50001,isCountBoundary:true,completeDecode:true,expectedEntry:'match',expectedByte:'not_matched'},
    {label:'04-bytes-plain-exact',xml:exact,wire:exact,completeDecode:true,expectedEntry:'not_matched',expectedByte:'not_matched'},
    {label:'05-bytes-plain-plus-one',xml:plus,wire:plus,completeDecode:true,expectedEntry:'not_matched',expectedByte:'match'},
    {label:'06-bytes-file-gzip-exact',xml:exact,wire:exactGzip,fileGzip:true,completeDecode:true,expectedEntry:'not_matched',expectedByte:'not_matched'},
    {label:'07-bytes-file-gzip-plus-one',xml:plus,wire:plusGzip,fileGzip:true,completeDecode:true,expectedEntry:'not_matched',expectedByte:'match'},
    {label:'08-bytes-http-gzip-exact',xml:exact,wire:exactGzip,httpGzip:true,completeDecode:true,expectedEntry:'not_matched',expectedByte:'not_matched'},
    {label:'09-bytes-http-gzip-plus-one',xml:plus,wire:plusGzip,httpGzip:true,completeDecode:true,expectedEntry:'not_matched',expectedByte:'match'},
    {label:'10-legacy-cap-unknown',xml:exact,wire:exact,legacy:true,expectedEntry:'unknown',expectedByte:'unknown'},
    {label:'11-file-gzip-crc-failed-unknown',xml:exact,wire:badCrc,fileGzip:true,badCrc:true,expectedEntry:'unknown',expectedByte:'unknown'},
    {label:'12-empty-known-zero',xml:empty,wire:empty,completeDecode:true,expectedEntry:'not_matched',expectedByte:'not_matched'},
    {label:'13-http503-unknown',xml:null,wire:null,status:503,expectedEntry:'unknown',expectedByte:'unknown'},
  ];assert.deepEqual(specs.map(spec=>spec.label),preparation.cases.map(spec=>spec.label));
  const response503=path.join(FIXTURES,'503.txt');fs.writeFileSync(response503,'owned unavailable source\n',{flag:'wx'});response503Info={file:response503,...await hashFile(response503)};
  for(const spec of specs){spec.route='/'+spec.label+(spec.fileGzip?'.xml.gz':'.xml');routes.set(spec.route,spec);}

  progress('fixture_preparation_complete',{required_cases:specs.length});
  phase='actual source capture';await start('capture-app');
  for(const spec of specs){
    phase=spec.label;lease();const task=origin+spec.route;
    // Every Loc is an original owned-origin identity. The explicit URL filter
    // blocks only page admissions, preserving all source capture/retained rows.
    assert(!new RegExp(filterPattern).test(task));
    const config={seed_urls:[origin+'/seed'],sitemap_urls:[task],exclude_patterns:[filterPattern],sitemap_membership_limits:MEMBERSHIP_LIMITS,crawl_links:false,discover_sitemaps:false,obey_robots:false,check_external:false,concurrency:1,delay_ms:0,max_pages:1,max_sitemap_documents:1,max_body_bytes:spec.legacy?WORK_CAP:1024,request_timeout_secs:180,...(!spec.legacy?{sitemap_source_processing:{max_post_http_entity_bytes:WORK_CAP,max_decoded_xml_bytes:WORK_CAP,max_xml_events:2_000_000}}:{})};
    try{const created=decode(await request('/api/crawls',{body:config}));assert.match(created.id,/^[a-f0-9-]{36}$/);const observed=await terminal(created.id);const record={label:spec.label,id:created.id,task,database:path.join(DATA,created.id+'.sqlite'),terminal:observed,expectedEntry:spec.expectedEntry,expectedByte:spec.expectedByte,wire_witness:spec.wire,xml_witness:spec.xml};cases.push(record);save(spec.label+'.terminal.json',observed);await inspectCase(spec,spec.label+'.initial');}
    catch(error){failures.push({phase,message:String(error.stack??error)});}
    progress('case_phase_complete',{label:spec.label,observation_phase:'initial',failures_observed:failures.length});
  }
  progress('capture_observation_phase_complete',{cases_observed:cases.length,failures_observed:failures.length});
  phase='actual captured-app close';await stop();progress('capture_app_actual_close_complete',{owned_runs:runs.length});
  const before=await snapshotData();save('product-data-before-reopen.json',before);const requestsBefore=hits.length;
  phase='saved SQLite reopen';await start('reopen-app');
  for(const spec of specs){const record=cases.find(row=>row.label===spec.label);if(!record)continue;phase=spec.label+' reopen';try{const oldFindings=record.findings?.[spec.label+'.initial'];await inspectCase(spec,spec.label+'.reopened');assert.deepEqual(record.findings?.[spec.label+'.reopened'],oldFindings,'Actual reopened API and CSV exact source evidence');}catch(error){failures.push({phase,message:String(error.stack??error)});}progress('case_phase_complete',{label:spec.label,observation_phase:'reopened',failures_observed:failures.length});}
  assert.equal(hits.length,requestsBefore,'Offline saved reads/reopen must make zero website requests');
  progress('reopen_observation_phase_complete',{cases_observed:cases.length,failures_observed:failures.length});
  phase='actual reopened-app close';await stop();const after=await snapshotData();save('product-data-after-reopen.json',after);assert.deepEqual(after,before,'All actual product data bytes unchanged across reopen and read-only API/CSV');databaseBytesUnchanged=true;progress('reopen_actual_close_and_unchanged_data_complete',{owned_runs:runs.length});
  for(const spec of specs){const sourceHits=hits.filter(hit=>hit.path===spec.route);assert.equal(sourceHits.length,1,'Exactly one actual source request '+spec.label);}
  assert(hits.every(hit=>hit.method==='GET'&&(routes.has(hit.path)||hit.path==='/seed')),'No other website fetches');
}catch(error){failures.push({phase,message:String(error.stack??error)});}
finally{
  phase='retained owned cleanup';try{await stop();}catch(error){failures.push({phase,message:String(error.stack??error)});}
  try{await closeFixture();}catch(error){failures.push({phase:'fixture cleanup',message:String(error.stack??error)});}
  try{verifyOuterLock();binding.verify();assert.equal(SHA(read(SELF)),selfRow.sha256);assert.equal(SHA(read(preparationPath)),args['preparation-sha']);assert.equal(SHA(read(args.release)),args['release-sha']);assert.equal(SHA(read(binderPath)),args['bindings-sha']);finalBindingVerified=true;}catch(error){failures.push({phase:'final immutable guards',message:String(error.stack??error)});}
  process.off('SIGINT',onInt);process.off('SIGTERM',onTerm);
  const genuine=cases.find(row=>row.label==='02-count-50000-required-positive');
  const genuine50000Durable=Boolean(genuine?.sql?.counts.locations===50_000&&genuine.sql.counts.entries===50_000&&genuine.sql.commits.length===1&&genuine.terminal.status==='completed'&&genuine.paging?.['02-count-50000-required-positive.initial']?.total===50_000&&genuine.paging?.['02-count-50000-required-positive.reopened']?.total===50_000&&genuine.sql.catalog_serialized.above_unchanged_default);
  const complete=failures.length===0&&cases.length===13&&runs.length===2&&runs.every(run=>run.actual_close_event&&run.stdout.end&&run.stdout.close&&run.stderr.end&&run.stderr.close)&&siteClosed&&finalBindingVerified&&databaseBytesUnchanged&&genuine50000Durable;
  const proof={kind:'genuine_large_sitemap_native_v8',result:complete?'PASS_GENUINE_LARGE_SOURCE_NATIVE_WITH_DECLARED_BODY_RETENTION_GAP':'FAIL_REQUIRED_LARGE_SOURCE_ACCEPTANCE',recorded_at:new Date().toISOString(),source_manifest_sha256:SOURCE_SHA,preparation_manifest_sha256:args['preparation-sha'],harness_sha256:selfRow.sha256,binary:binding.build.binary,build_proof_sha256:SHA(binding.buildRaw),suite_proof_sha256:SHA(binding.suiteRaw),release_sha256:args['release-sha'],cases:cases.map(record=>({...record,sql:record.sql?{counts:record.sql.counts,summary:record.sql.summary,sourceSummary:record.sql.sourceSummary,locations:record.sql.locations,catalog_serialized:record.sql.catalog_serialized,admission_decisions:record.sql.admission_decisions}:null})),actual_case_count:cases.length,required_case_count:13,genuine_50000_complete_durable_positive:genuine50000Durable,all_owned_app_processes_actually_closed:runs.every(run=>run.actual_close_event),all_owned_app_pipes_actually_drained:runs.every(run=>run.stdout.end&&run.stdout.close&&run.stderr.end&&run.stderr.close),fixture_actual_close_callback:siteClosed,fixture_sockets_remaining:sockets.size,fixture_requests_remaining:active.size,fixture_file_stream_tasks_remaining:fixtureTasks.size,all_product_data_bytes_unchanged_across_reopen:databaseBytesUnchanged,final_source_suite_build_binary_guards_verified:finalBindingVerified,api_receipts:apiReceipts,runs,fixture_hits:hits,signals,failures,progress_observations_only:{emitted:progressRecords,maximum:64,acceptance:false,extra_http_or_sql:0},known_source_gaps:{original_xml_body_durable_recovery:'NOT_ASSERTED_BY_THIS_GATE; Source8 includes opt-in original-body public APIs, independently tested by a separate BODY gate, but these13 unchanged configs omit sitemap_source_archive. Harness fixture files are not product persistence; no original-body durable recovery follows from this gate.',genuine_50000_membership_capacity:'Public explicit128MiB capacity is declared independently of64MiB source work. Actual serialized footprint is counted from real SQLite canonical rows/envelope in this run; the symbolic51,700,000-byte entry/Loc floor is historical schema reasoning only. Missing complete50000 rows remains a failing positive, never a passing Unknown substitute.'},qualification:'Actual Source8/Windows optimized candidate only. No installation, vendor parity, full index matrix, full XML/XSD validity, source-body retention, 50k crawled website pages, platform portability, RSS, throughput or hard/graceful in-flight drain claim. Each public JSON response retains actual8MiB bound; count/decode facts, XML work64MiB, explicit membership catalog128MiB (default32MiB unchanged), retained text32MiB and CSV64MiB remain independent. Normal app cleanup requests owned stdin EOF only after all complete CSV reads. Actual clean Source8 receipt, exit0/null signal and drained pipes are required; fallback SIGKILL is failure-only. The sitemap CSV routes use state.exports, whose shutdown is one of nine actual source-pinned drain branches; their full normal EOF is separately observed, not counted as new policy/resource response_streams work. New registries must emit genuine zero work in these unchanged13 configs. No active-export/unpolled transport drain or hard shutdown deadline is asserted.'};
  const proofPath=save('proof.json',proof);console.log(JSON.stringify({result:proof.result,proof:proofPath,sha256:(await hashFile(proofPath)).sha256,genuine_50000_complete_durable_positive:genuine50000Durable,failures:failures.length}));if(!complete)process.exitCode=1;
}
