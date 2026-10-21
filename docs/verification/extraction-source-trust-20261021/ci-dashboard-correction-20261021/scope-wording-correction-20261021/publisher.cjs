'use strict';
// Offline editorial correction only. Root alone may apply after both exact49
// gates. No test/runtime/network/Git execution and no raw ZIP/helper rewriting.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash,randomUUID}=require('node:crypto');
const ROOT=path.resolve(__dirname,'..'),SELF='artifacts/correct-ci-dashboard-scope-20261021.cjs';
const BASE='docs/verification/extraction-source-trust-20261021/ci-dashboard-correction-20261021';
const CHILD=BASE+'/scope-wording-correction-20261021';
const AUDIT='artifacts/ci-dashboard-correction-independent-audit-20261021/receipt.json';
const LOCAL='artifacts/extraction-source-full-ui49-20261021-r1';
const PUBLIC='artifacts/extraction-source-public-ci-7250b9a-20261021/receipt.json';
const COMMIT='7250b9a0007d0f307321e3b4c7076fa95430dbc3';
const PINS={
 'result.json':'dd155b633b7762276460a4990325fe566084716441ca86f4fd55eec7a7857d51',
 'README.md':'5465c7e4a33ffc254b3de099f41806b5400d89cfdf0c7961360d3c6e0205d6f2',
 'index.json':'44fa5155d493df62afde796d3795cf4de9a96dd4b2240d95a813285df5a293ce',
};
const AUDIT_SHA='641b3321a42da898f8b5e4bbb417c0901e213ee433b6ac19627f87cf290462e3';
const ZIP='raw/artifacts/extraction-source-renderer-failure-2c573d9-20261021/enrichment-browser-e2e.zip';
const HELPER='raw/artifacts/materialize-ci-dashboard-correction-20261021.cjs';
const sha=b=>createHash('sha256').update(b).digest('hex');
function relative(value){assert(typeof value==='string'&&value&&value===value.replaceAll('\\','/')&&!path.isAbsolute(value)&&!/[\u0000-\u001f\u007f:]/u.test(value));assert(value.split('/').every(x=>x&&x!=='.'&&x!=='..'));return value;}
function absolute(value){const result=path.resolve(ROOT,relative(value));assert(result.startsWith(ROOT+path.sep));return result;}
function noLinks(value,missing=false){let current=ROOT;const parts=relative(value).split('/');for(let i=0;i<parts.length;i++){current=path.join(current,parts[i]);let stat;try{stat=fs.lstatSync(current);}catch(error){if(missing&&error.code==='ENOENT')return;throw error;}assert(!stat.isSymbolicLink(),'Link refused: '+value);assert(i===parts.length-1||stat.isDirectory(),'Parent is not a directory: '+value);}}
function main(){
 const args=process.argv.slice(2),apply=args[0]==='--apply';assert(!args.length||(args.length===1&&args[0]==='--validate')||(apply&&args.length===2&&/^[a-f0-9]{64}$/.test(args[1])),'Use --validate or --apply reviewed-plan-sha256');
 assert.equal(path.resolve(process.cwd()),ROOT);assert(fs.lstatSync(ROOT).isDirectory()&&!fs.lstatSync(ROOT).isSymbolicLink());noLinks(BASE);noLinks(CHILD,true);assert(!fs.existsSync(absolute(CHILD)),'Fresh correction child required; no automatic retry/overwrite');
 const inputs=new Map(),outputs=new Map();
 function read(file,pin){noLinks(file);const stat=fs.lstatSync(absolute(file));assert(stat.isFile()&&stat.size<=16*1024*1024,'Bounded regular file required: '+file);const bytes=fs.readFileSync(absolute(file));if(pin)assert.equal(sha(bytes),pin,'Frozen file changed: '+file);if(inputs.has(file))assert.deepEqual(bytes,inputs.get(file));else inputs.set(file,bytes);return bytes;}
 const json=file=>JSON.parse(read(file).toString('utf8'));
 function put(file,value){relative(file);assert(!outputs.has(file));outputs.set(file,Buffer.isBuffer(value)?value:Buffer.from(value));}
 function record(file,bytes){return{path:file,bytes:bytes.length,sha256:sha(bytes)};}
 const originals=new Map(Object.entries(PINS).map(([name,pin])=>[name,read(BASE+'/'+name,pin)]));
 const auditBytes=read(AUDIT,AUDIT_SHA),audit=JSON.parse(auditBytes);
 assert.equal(audit.result,'PASS_FINITE_BYTE_BINDINGS_WITH_GENERATED_SCOPE_WORDING_FINDING');assert.equal(audit.index_sha256,PINS['index.json']);assert.equal(audit.raw_copies,31);assert.equal(audit.runtime_failure,false);
 const index=JSON.parse(originals.get('index.json'));assert.equal(index.raw_files.length,31);assert.equal(new Set(index.raw_files.map(row=>row.path)).size,31);assert.equal(index.generated_files.length,2);
 for(const row of index.raw_files){assert(row.path.startsWith('raw/'));const bytes=read(BASE+'/'+relative(row.path),row.sha256);assert.equal(bytes.length,row.bytes);}
 for(const row of index.generated_files){assert(['result.json','README.md'].includes(row.path));assert.equal(row.sha256,PINS[row.path]);assert.equal(row.bytes,originals.get(row.path).length);}
 const zip=index.raw_files.find(row=>row.path===ZIP),helper=index.raw_files.find(row=>row.path===HELPER);assert(zip&&helper);assert.equal(zip.sha256,'a6f0744b270bf99693402cc0a37ba78404e457072bbf34bf8b4944f6dfdc3688');assert.equal(helper.sha256,'208071f08e4d1bfdbeb72893c3370ab899d3a1a768be6e63024932a594387518');
 const result=JSON.parse(originals.get('result.json')),oldSentence='No runtime stores or binaries are copied.';
 assert.equal(result.result,'ORIGINAL_CI_UI_FAIL_RETAINED_SELECTED_LOCAL_DASHBOARD_PASS_NOT_FULL49');assert.equal(result.scope.split(oldSentence).length,2);
 result.scope=result.scope.replace(oldSentence,'The unchanged enrichment artifact ZIP retains12 owned synthetic native-store member files. Installed/user stores and native executables are excluded; original raw ZIP and archived helper bytes remain unchanged.');
 put(BASE+'/result.json',JSON.stringify(result,null,2)+'\n');
 const note='\nEditorial scope correction: the unchanged complete enrichment artifact ZIP includes12 owned synthetic native-store member files from its isolated fixture. These are owned synthetic test records; installed/user stores and native executables are excluded. The earlier generated wording “No runtime stores or binaries are copied” was too broad. [Transition](scope-wording-correction-20261021/transition.json) preserves the original result, README and index plus the independent audit. The ZIP and archived materializer retain their exact original bytes. This wording change adds no test result and does not relabel the historical failed CI or selected dashboard invocation.\n';
 put(BASE+'/README.md',Buffer.concat([originals.get('README.md'),Buffer.from(note)]));
 for(const[name,bytes]of originals)put(CHILD+'/original/'+name,bytes);
 put(CHILD+'/audit-receipt.json',auditBytes);put(CHILD+'/publisher.cjs',read(SELF));
 const transition={kind:'editorial_scope_correction_originals_preserved',result:'GENERATED_METADATA_WORDING_ONLY_NO_NEW_RUNTIME_OR_ACCEPTANCE',audit_source:AUDIT,audit_sha256:AUDIT_SHA,original_metadata:[...originals].map(([name,bytes])=>({...record('original/'+name,bytes),original_path:BASE+'/'+name})),replacement_metadata:['result.json','README.md'].map(name=>record(BASE+'/'+name,outputs.get(BASE+'/'+name))),unchanged_raw_files:index.raw_files.length,unchanged_enrichment_zip:zip,unchanged_archived_helper:helper,correction:'The retained complete artifact ZIP includes12 owned synthetic native-store member files. The exclusions concern installed/user stores and native executables. No original raw declaration or archived helper is rewritten.',sequencing_gates:{local:LOCAL+'.wrapper.json',public:PUBLIC,commit:COMMIT,scope:'Apply requires both exact49 receipts to be terminal PASS. This is sequencing only; the historical failed CI and selected-one evidence are not promoted to full49.'},publication_limit:'Each of the three generated metadata replacements uses a staged adjacent file and rename, after all original metadata is copied and byte-verified in the fresh child. The three renames are not a single filesystem transaction or a power-loss durability guarantee. If interrupted, originals remain in the child and automatic reapplication is refused for explicit review.'};
 put(CHILD+'/transition.json',JSON.stringify(transition,null,2)+'\n');
 index.generated_files=['result.json','README.md'].map(name=>record(name,outputs.get(BASE+'/'+name)));
 index.editorial_correction_files=[...outputs].filter(([file])=>file.startsWith(CHILD+'/')).map(([file,bytes])=>record(file.slice(BASE.length+1),bytes));
 index.scope+=' Editorial generated-metadata correction retains the original index/result/README in the declared fresh child and leaves all31 raw files unchanged; see scope-wording-correction-20261021/transition.json.';
 put(BASE+'/index.json',JSON.stringify(index,null,2)+'\n');assert.equal(outputs.size,9);assert.equal(index.editorial_correction_files.length,6);
 const plan=[...outputs].map(([file,bytes])=>record(file,bytes)).sort((a,b)=>a.path<b.path?-1:a.path>b.path?1:0),planSHA=sha(Buffer.from(JSON.stringify(plan)));
 let gatesReady=false,gateReason;
 try{
  const wrapper=json(LOCAL+'.wrapper.json'),local=json(LOCAL+'/proof.json'),pub=json(PUBLIC);
  assert.equal(wrapper.source_commit,COMMIT);assert.equal(wrapper.canonical49_accepted,true);assert.equal(wrapper.code,0);assert.equal(wrapper.signal,null);assert.equal(wrapper.binary_unchanged,true);assert.equal(wrapper.compiler_inputs_unchanged,true);
  assert.equal(local.complete,true);assert.equal(local.results.length,49);assert.equal(new Set(local.results.map(row=>row.name)).size,49);assert(local.results.every(row=>row.code===0&&row.signal===null&&!row.timed_out&&!row.error));assert.deepEqual(local.shutdown,{code:0,signal:null});assert.deepEqual(local.changed,[]);
  assert.equal(pub.result,'PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI49_INVOCATION');assert.equal(pub.commit,COMMIT);assert.equal(pub.run,39638818285);assert.equal(pub.status,'completed');assert.equal(pub.conclusion,'success');assert.equal(pub.jobs.length,4);assert.equal(new Set(pub.jobs.map(row=>row.id)).size,4);assert(pub.jobs.every(row=>row.status==='completed'&&row.conclusion==='success'));assert.equal(pub.renderer.accepted,true);assert.equal(pub.renderer.passed_suites,49);assert.equal(pub.renderer.unique_passed_suites,49);assert.equal(pub.renderer.canonical_unfiltered_invocations,1);assert.deepEqual(pub.log_errors,[]);
  gatesReady=true;
 }catch(error){gateReason=error.message;}
 const report={result:apply?'APPLIED_EDITORIAL_SCOPE_CORRECTION':gatesReady?'READY_VALIDATED_NO_WRITES':'PLAN_READY_AWAITING_EXACT49_GATES_NO_WRITES',gates_ready:gatesReady,...gateReason?{gate_reason:gateReason}:{},plan_sha256:planSHA,files:plan.length,metadata_replacements:3,preserved_and_transition_files:6,unchanged_raw_files:31,publisher_sha256:sha(inputs.get(SELF)),plan};
 if(!apply){console.log(JSON.stringify(report,null,2));return;}
 assert(gatesReady,'Exact49 local/public sequencing gates are required before any write');assert.equal(args[1],planSHA,'Reviewed finite plan SHA required');
 for(const[file,bytes]of inputs){noLinks(file);assert.deepEqual(fs.readFileSync(absolute(file)),bytes,'Input changed: '+file);}noLinks(CHILD,true);assert(!fs.existsSync(absolute(CHILD)));
 fs.mkdirSync(absolute(CHILD));fs.mkdirSync(absolute(CHILD+'/original'));
 function writeNew(file,bytes){noLinks(path.posix.dirname(file));fs.writeFileSync(absolute(file),bytes,{flag:'wx'});assert.deepEqual(fs.readFileSync(absolute(file)),bytes);}
 for(const[file,bytes]of outputs)if(file.startsWith(CHILD+'/'))writeNew(file,bytes);
 const staged=[];try{
  for(const name of['result.json','README.md','index.json']){const target=BASE+'/'+name,temp=BASE+'/.scope-wording-'+randomUUID()+'.tmp',bytes=outputs.get(target);noLinks(BASE);const fd=fs.openSync(absolute(temp),'wx');staged.push({target,temp,bytes});try{fs.writeFileSync(fd,bytes);}finally{fs.closeSync(fd);}assert.deepEqual(fs.readFileSync(absolute(temp)),bytes);}
  for(const entry of staged){noLinks(entry.target);assert.deepEqual(fs.readFileSync(absolute(entry.target)),originals.get(path.posix.basename(entry.target)));fs.renameSync(absolute(entry.temp),absolute(entry.target));assert.deepEqual(fs.readFileSync(absolute(entry.target)),entry.bytes);}
 }finally{for(const entry of staged)if(fs.existsSync(absolute(entry.temp))){noLinks(entry.temp);assert(fs.lstatSync(absolute(entry.temp)).isFile());fs.unlinkSync(absolute(entry.temp));}}
 for(const row of index.raw_files){noLinks(BASE+'/'+row.path);assert.equal(sha(fs.readFileSync(absolute(BASE+'/'+row.path))),row.sha256);}
 console.log(JSON.stringify(report,null,2));
}
try{main();}catch(error){console.error(error.stack);process.exitCode=1;}
