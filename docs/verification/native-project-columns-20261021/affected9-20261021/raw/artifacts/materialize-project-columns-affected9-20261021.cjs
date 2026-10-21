'use strict';
// Offline finite log receipt proposal. No project tests, native/browser,
// network or Git calls. Root alone applies an exact reviewed output plan.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const ROOT=path.resolve(__dirname,'..'),SELF='artifacts/materialize-project-columns-affected9-20261021.cjs';
const LOCAL='artifacts/project-columns-affected-20261021-r3',RUNNER='artifacts/run-project-columns-affected-20261021-r3.cjs';
const DEST='docs/verification/native-project-columns-20261021/affected9-20261021';
const EXPECTED=['dashboard','table-explorer','table-named-views','table-expressions','table-view-portability','table-xlsx','table-xlsx-provenance','table-xlsx-availability','extraction-source-trust'];
const REGISTRY_SHA='bf2d662143290342f3e6cb62fb97e9453fd38d64f33b792e921e578e3748862a';
const FAILED='artifacts/project-columns-affected-20261021-r1',FAILED_NESTED='artifacts/project-columns-affected-20261021-r1/table-named-views/table-views-1792606680458-9bd9395f',REPAIR='artifacts/project-columns-named-view-readiness-20261021';
const FAILED_PINS=new Map([
  [
    "artifacts/project-columns-affected-20261021-r1/proof.json",
    "ec8ef6aea6be09d021ffd2641fb5258e51f51315cf9da145518a08d78d55e0ee"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1/app.log",
    "42e7625276a5bd8f2149f54225fa69549706531076d2123fb87fdc5bde7c20a2"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1.wrapper.json",
    "0acd702808ade4e2ac6a1a90e0a9d15abd01d4662be5a848aabbc9aec0d15afb"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1.stdout.log",
    "d77b521ee8ca044d041ad37c12fc7240110cf075b59d31d24ec902ffcae4c3be"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1.stderr.log",
    "58da25182057188674bc6e8b8875e35028d2ec96a9dd19ded98ced4bcb9fa66c"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1/dashboard/stdout.log",
    "08bd9386b671751d85c7675b9eced5d66844fa1e7d4f76c8feb1ff3c6dff598d"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1/dashboard/stderr.log",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1/table-explorer/stdout.log",
    "b469c2d1937c00dd992add323d3ecb92e363acde82f958b22f235b7892d2e51c"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1/table-explorer/stderr.log",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1/table-named-views/stdout.log",
    "151dc0eae11e7edc989604e86a97b40860caf9f840da870b54e7c9b2243e1ddb"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1/table-named-views/stderr.log",
    "78608a0fa333de0f3f74accfa15060a922e0de3607e0fae4d51a49f621515c8d"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1/table-named-views/table-views-1792606680458-9bd9395f/ui-table-views-live.json",
    "df83a4eaee57085905d9d08c37095c66961749b4dcd380b2243fe69389843377"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1/table-named-views/table-views-1792606680458-9bd9395f/table-views-failure.html",
    "b6b87e1e881fa3d93c621d778dc98fa8cb834c104959012f84eb98989311d0ee"
  ],
  [
    "artifacts/project-columns-affected-20261021-r1/table-named-views/table-views-1792606680458-9bd9395f/native-warm.log",
    "cce4a84c35d98e4199833821506878bcb3b1b6893e632725ab61c11a315f77ce"
  ],
  [
    "artifacts/project-columns-named-view-readiness-20261021/ui-table-views-live.r1.cjs",
    "907d197fc96bbc724115c63c43fbe8048cfd27e7ef11f19a674382de3bb6a5cb"
  ],
  [
    "artifacts/project-columns-named-view-readiness-20261021/repair.json",
    "3baa2d9e7c128fa3734c9bab40221f56fc1d0d75ad1a206e3257061e09b0b689"
  ],
  [
    "artifacts/repair-project-columns-named-view-readiness-20261021.cjs",
    "086b099ecb90dde651a9a0e1dd334491c164d2c8783abafa9bfa45e38934364c"
  ],
  [
    "artifacts/run-project-columns-affected-20261021-r1.cjs",
    "1d183b58f31b856c35d6a4659d24b3e14d2c80223670ffea780b132ebd9c0108"
  ]
]);
const FAILED2='artifacts/project-columns-affected-20261021-r2',FAILED2_NESTED='artifacts/project-columns-affected-20261021-r2/table-named-views/table-views-1792606906647-8b8b5314';
const FAILED2_PINS=new Map([
  [
    "artifacts/project-columns-affected-20261021-r2/proof.json",
    "1f2d7804f1576a911ff08b025fa383aaedd3800594214591a61668fd29a77073"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2/app.log",
    "c7f8323aefd112f70e9ebf77c559c55ea23bf09b05b7fedbecb6f24db6b30c40"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2.wrapper.json",
    "0c89ffeea5afe179783d9420de5d9dcb0b2bc8759322ee16ce4e69cbe9de67d2"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2.stdout.log",
    "175eb9f322b4accdc9f4247e12f70728f18414f83806526a8806c677ffb2926a"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2.stderr.log",
    "7627e931b2220ebc99c5f8b36dc04a8177c3b896a921242a235077a1b97dfb0a"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2/dashboard/stdout.log",
    "08bd9386b671751d85c7675b9eced5d66844fa1e7d4f76c8feb1ff3c6dff598d"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2/dashboard/stderr.log",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2/table-explorer/stdout.log",
    "07f28409323f7d3e679e87d5fbfc8d04557463c652801e866c3b1451f9f9eb24"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2/table-explorer/stderr.log",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2/table-named-views/stdout.log",
    "20307d8e701ff7192943fe334a98018f0c3f85d1b03eaa90cdba3ff6118d7948"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2/table-named-views/stderr.log",
    "19cf3a1344c3d687e309af602db8927d6ea8616708ea9295e8ebca0b10b06a24"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2/table-named-views/table-views-1792606906647-8b8b5314/ui-table-views-live.json",
    "3e187fd91a00565303ee1ca76b07e64d8b74a782081d43e828e77c05cceed3c9"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2/table-named-views/table-views-1792606906647-8b8b5314/table-views-failure.html",
    "86c8da31f5574a1070f906a2a14cc997647ecf9b31ad6346f7d7468c59c75e79"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2/table-named-views/table-views-1792606906647-8b8b5314/table-views-no-locks-failure.html",
    "cf6cfc7b6bd7a5816add2ad95b47d44359588b979c57d8ec82774b186cde7bd6"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2/table-named-views/table-views-1792606906647-8b8b5314/native-warm.log",
    "712b40d52009c48b72ec8a652f2d1826a76d6d59bf3f84bc52323046e8fc33bc"
  ],
  [
    "artifacts/project-columns-affected-20261021-r2/table-named-views/table-views-1792606906647-8b8b5314/table-views-no-locks-transport.json",
    "d1bee161b7bdf9f3c63969dc7cc872a4d12529460ae9284fa0774766ebc0a0cd"
  ],
  [
    "artifacts/project-columns-named-view-readiness-20261021/ui-table-views-live.r2.cjs",
    "ba1236f17844e181152d974161373241cdb70dc7ed3b10ae3ee6d2619396590c"
  ],
  [
    "artifacts/project-columns-named-view-readiness-20261021/alias-repair.json",
    "f41c19253267026b5a4adbc2ab874a74ab463cc8ce6a5218785e4b1af831dfd7"
  ],
  [
    "artifacts/repair-project-columns-named-view-alias-20261021.cjs",
    "9cfa54d3faa47f02249ba39d8db6d11eeee2f8beff6f0d34627dead72667b182"
  ],
  [
    "artifacts/run-project-columns-affected-20261021-r2.cjs",
    "c39cc9a24caa9972005672101a330cc5ca49de475aa00a180404f10e40a5d9a7"
  ]
]);
const BINARY_SHA='b245edcfa1b8f2a7b61155726fdd9c79c0b82c34f3835915d77bed3931798db9';
const sha=value=>createHash('sha256').update(value).digest('hex');
function rel(value){assert(typeof value==='string'&&value&&value===value.replaceAll('\\','/')&&!path.isAbsolute(value)&&!/[\u0000-\u001f\u007f:]/u.test(value));assert(value.split('/').every(part=>part&&part!=='.'&&part!=='..'));return value;}
function abs(value){const result=path.resolve(ROOT,rel(value));assert(result.startsWith(ROOT+path.sep));return result;}
function noLinks(value,missing=false){let current=ROOT;const parts=rel(value).split('/');for(let i=0;i<parts.length;i++){current=path.join(current,parts[i]);let stat;try{stat=fs.lstatSync(current);}catch(error){if(missing&&error.code==='ENOENT')return;throw error;}assert(!stat.isSymbolicLink(),'Link refused: '+value);assert(i===parts.length-1||stat.isDirectory());}}
class Pending extends Error{}
function main(){
 const args=process.argv.slice(2);let apply=false,reviewedPlan,rustRoot='artifacts/project-columns-rust-20261021-r2',seenRust=false,seenMode=false;
 for(let i=0;i<args.length;i++){
  if(args[i]==='--rust-root'){assert(!seenRust&&i+1<args.length);seenRust=true;rustRoot=args[++i];assert(/^artifacts\/project-columns-rust-20261021-r[1-9][0-9]*$/.test(rustRoot));}
  else if(args[i]==='--validate'){assert(!seenMode);seenMode=true;}
  else if(args[i]==='--apply'){assert(!seenMode&&i+1<args.length);seenMode=true;apply=true;reviewedPlan=args[++i];assert(/^[a-f0-9]{64}$/.test(reviewedPlan));}
  else assert.fail('Use [--rust-root artifacts/project-columns-rust-20261021-rN] --validate | --apply reviewed-plan-sha256');
 }
 assert.equal(path.resolve(process.cwd()),ROOT);assert(fs.lstatSync(ROOT).isDirectory()&&!fs.lstatSync(ROOT).isSymbolicLink());
 noLinks(DEST,true);assert(!fs.existsSync(abs(DEST)),'Fresh child destination only');
 if(!fs.existsSync(abs(LOCAL+'/proof.json'))||!fs.existsSync(abs(LOCAL+'.wrapper.json')))throw new Pending('Actual selected9 proof/wrapper is not terminal; no public writes');
 noLinks(path.posix.dirname(DEST));assert(fs.lstatSync(abs(path.posix.dirname(DEST))).isDirectory(),'Materialize the parent first');
 const inputs=new Map(),outputs=new Map(),copies=[];
 function read(file,pin){noLinks(file);const stat=fs.lstatSync(abs(file));assert(stat.isFile()&&stat.size<=128*1024*1024,'Bounded ordinary source required: '+file);const bytes=fs.readFileSync(abs(file));if(pin)assert.equal(sha(bytes),pin,'Source changed: '+file);if(inputs.has(file))assert.deepEqual(bytes,inputs.get(file));else inputs.set(file,bytes);return bytes;}
 const json=file=>JSON.parse(read(file).toString('utf8').replace(/^\uFEFF/u,''));
 function put(file,value){rel(file);assert(!outputs.has(file));outputs.set(file,Buffer.isBuffer(value)?value:Buffer.from(value));}
 function copy(file,category,pin){const bytes=read(file,pin),destination='raw/'+file;put(destination,bytes);copies.push({source:file,path:destination,bytes:bytes.length,sha256:sha(bytes),category});return bytes;}
 assert(read('.gitattributes').toString('utf8').includes('/docs/verification/native-project-columns-20261021/** -text'),'Explicit existing raw-preserving parent policy required');
 const registry=read('scripts/ui-e2e.mjs',REGISTRY_SHA),all=[...registry.toString('utf8').matchAll(/^\s+\['([a-z0-9-]+)',\s*'([^']+)',\s*'(fixture|live|owned-native)'(?:,\s*(\d+))?\]/gm)].map(m=>({name:m[1],script:m[2],kind:m[3],timeout_ms:m[4]?Number(m[4]):240000}));
 assert.equal(all.length,50);assert.equal(new Set(all.map(row=>row.name)).size,50);const expected=all.filter(row=>EXPECTED.includes(row.name));assert.deepEqual(expected.map(row=>row.name),EXPECTED);assert.equal(expected.filter(row=>row.kind==='fixture').length,1);assert.equal(expected.filter(row=>row.kind==='owned-native').length,8);
 const local=json(LOCAL+'/proof.json'),wrapper=json(LOCAL+'.wrapper.json'),rust=json(rustRoot+'/proof.json'),compiled=json(rustRoot+'/inputs.json');
 assert.equal(local.kind,'redesign_UI_E2E');assert.equal(local.complete,true);assert.equal(local.binary_unchanged,true);assert.equal(local.tracked_sources_unchanged,true);assert.deepEqual(local.changed,[]);assert(!local.failure);assert.deepEqual(local.shutdown,{code:0,signal:null});assert.equal(local.results.length,9);assert.deepEqual(local.results.map(({name,script,kind,timeout_ms})=>({name,script,kind,timeout_ms})),expected);for(const row of local.results){assert.equal(row.code,0,row.name);assert.equal(row.signal,null,row.name);assert.equal(row.timed_out,false,row.name);assert(!row.error,row.name);assert(Number.isSafeInteger(row.duration_ms)&&row.duration_ms>=0);}
 assert.equal(wrapper.code,0);assert.equal(wrapper.signal,null);assert(!wrapper.launchError);assert.equal(wrapper.binary_unchanged,true);assert(Number.isSafeInteger(wrapper.duration_ms)&&wrapper.duration_ms>=0);
 assert.equal(rust.complete,true,'The explicitly selected final Rust authority must PASS; earlier failed attempts remain separate');assert.deepEqual(rust.changed,[]);assert(!rust.failure);assert.deepEqual(rust.results.map(row=>row.name),['format','app-tests','build','clippy']);assert(rust.results.every(row=>row.code===0&&row.signal===null&&!row.launchError));assert(/^[a-f0-9]{64}$/.test(rust.binary?.sha256||''));assert(Number.isSafeInteger(rust.binary.bytes)&&rust.binary.bytes>0);assert.equal(local.binary_sha256,rust.binary.sha256);assert.equal(rust.binary.sha256,BINARY_SHA);
 const binary=read('target/debug/open-seo-spider.exe',rust.binary.sha256);assert.equal(binary.length,rust.binary.bytes);assert.equal(compiled.length,426);assert.equal(new Set(compiled.map(row=>row.file)).size,426);for(const row of compiled){const bytes=read(row.file,row.sha256);assert.equal(bytes.length,row.bytes);}
 const stdout=read(LOCAL+'.stdout.log').toString('utf8'),observed=[...stdout.matchAll(/^PASS ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].map(m=>({name:m[1],kind:m[2]}));assert.deepEqual(observed,expected.map(({name,kind})=>({name,kind})));assert.equal(new Set(observed.map(row=>row.name)).size,9);assert.equal([...stdout.matchAll(/^FAIL [a-z0-9-]+ \((fixture|live|owned-native)\)\r?$/gm)].length,0);assert(stdout.includes('Evidence: '+abs(LOCAL)));
 const runner=read(RUNNER).toString('utf8');assert(runner.includes("'"+rustRoot.slice('artifacts/'.length)+"/proof.json'"),'Runner must refer to the same explicit final Rust authority');assert(runner.includes("'--only','"+EXPECTED.join(',')+"'"),'Runner must select exactly this ordered9 list');assert(runner.includes("'project-columns-affected-20261021-r3'"));
 const sources=expected.map(row=>{const file='web/verification/'+row.script,bytes=read(file);return{file,bytes:bytes.length,sha256:sha(bytes)};});
 for(const name of ['proof.json','app.log'])copy(LOCAL+'/'+name,'actual-selected9-coordinator');for(const row of expected)for(const stream of ['stdout','stderr'])copy(LOCAL+'/'+row.name+'/'+stream+'.log','actual-selected9-'+row.kind+'-log');for(const suffix of ['.wrapper.json','.stdout.log','.stderr.log'])copy(LOCAL+suffix,'actual-selected9-wrapper-and-logs');copy(RUNNER,'original-selected9-runner');copy('scripts/ui-e2e.mjs','current50-registry-source',REGISTRY_SHA);copy(rustRoot+'/proof.json','explicit-final-Rust-authority');copy(rustRoot+'/inputs.json','explicit426-compiler-input-authority');copy(SELF,'offline-finite-publisher');assert.equal(copies.length,28);
 const failed=json(FAILED+'/proof.json'),failedWrapper=json(FAILED+'.wrapper.json'),failedNamed=json(FAILED_NESTED+'/ui-table-views-live.json'),repair=json(REPAIR+'/repair.json');
 assert.equal(failed.complete,false);assert.equal(failed.binary_sha256,BINARY_SHA);assert.equal(failed.binary_unchanged,true);assert.equal(failed.tracked_sources_unchanged,true);assert.deepEqual(failed.changed,[]);assert.deepEqual(failed.shutdown,{code:0,signal:null});assert.equal(failedWrapper.code,1);assert.equal(failedWrapper.signal,null);assert.equal(failedWrapper.binary_unchanged,true);
 assert.deepEqual(failed.results.map(({name,code,signal,timed_out})=>({name,code,signal,timed_out})),[{name:'dashboard',code:0,signal:null,timed_out:false},{name:'table-explorer',code:0,signal:null,timed_out:false},{name:'table-named-views',code:1,signal:null,timed_out:false}]);assert.equal(failedNamed.result,'FAIL');assert(failedNamed.failure.includes('named empty/zero view reused against source A first-page range is installed by Apply itself'));assert.deepEqual(failedNamed.errors,[]);assert.equal(failedNamed.binary_sha256,BINARY_SHA);
 const oldDriver=read(REPAIR+'/ui-table-views-live.r1.cjs',repair.before_sha256),r2Driver=read(REPAIR+'/ui-table-views-live.r2.cjs',repair.after_sha256),aliasRepair=json(REPAIR+'/alias-repair.json'),currentDriver=read('web/verification/ui-table-views-live.cjs',aliasRepair.after_sha256);assert.equal(repair.product_changed,false);assert.equal(repair.deadline_changed,false);assert.equal(repair.assertions_removed,0);
 const applyLine=text=>text.toString('utf8').split(/\r?\n/).find(line=>line.startsWith('async function applyView('));assert.equal(applyLine(r2Driver),applyLine(oldDriver));assert.equal(applyLine(currentDriver),applyLine(oldDriver),'Immediate post-Apply pagination/label/row assertions must remain byte-identical');assert(failedNamed.pins.some(pin=>pin.file.replaceAll('\\','/').endsWith('/web/verification/ui-table-views-live.cjs')&&pin.sha256===repair.before_sha256),'Failed original driver pin must match retained backup');
 for(const[file,pin]of FAILED_PINS)copy(file,'original-selected9-R1-FAIL-and-readiness-repair',pin);

 const failed2=json(FAILED2+'/proof.json'),failed2Wrapper=json(FAILED2+'.wrapper.json'),failed2Named=json(FAILED2_NESTED+'/ui-table-views-live.json'),failed2Transport=json(FAILED2_NESTED+'/table-views-no-locks-transport.json');
 assert.equal(failed2.complete,false);assert.equal(failed2.binary_sha256,BINARY_SHA);assert.equal(failed2.binary_unchanged,true);assert.equal(failed2.tracked_sources_unchanged,true);assert.deepEqual(failed2.changed,[]);assert.deepEqual(failed2.shutdown,{code:0,signal:null});assert.equal(failed2Wrapper.code,1);assert.equal(failed2Wrapper.signal,null);assert.equal(failed2Wrapper.binary_unchanged,true);assert.deepEqual(failed2.results.map(({name,code,signal,timed_out})=>({name,code,signal,timed_out})),failed.results.map(({name,code,signal,timed_out})=>({name,code,signal,timed_out})));
 assert.equal(failed2Named.result,'FAIL');assert(failed2Named.failure.includes('actualNoLocks')&&failed2Named.failure.includes('Timeout 30000ms'));assert.equal(failed2Named.binary_sha256,BINARY_SHA);assert.equal(failed2Transport.kind,'owned_loopback_passthrough');assert.notEqual(failed2Transport.origin,failed2Transport.upstream_origin);assert(failed2Transport.requests.some(row=>row.path.endsWith('/fragments')&&row.method==='GET'&&row.status===200&&row.complete===true));assert.equal(aliasRepair.before_sha256,repair.after_sha256);assert.equal(aliasRepair.normal_binary_sha256,BINARY_SHA);assert.equal(aliasRepair.product_changed,false);assert.equal(aliasRepair.deadline_changed,false);assert.equal(aliasRepair.assertions_removed,0);
 const oldOrigin='const origin=app.origin,phase=app.phase',newOrigin='const origin=new URL(page.url()).origin,phase=app.phase';assert.equal(r2Driver.toString('utf8').split(oldOrigin).length,2);assert.equal(currentDriver.toString('utf8'),r2Driver.toString('utf8').replace(oldOrigin,newOrigin),'Alias follow-up changes only the watched actual page origin');for(const[file,pin]of FAILED2_PINS)copy(file,'original-selected9-R2-FAIL-and-alias-repair',pin);
 const namedRoot=LOCAL+'/table-named-views';noLinks(namedRoot);const namedDirs=fs.readdirSync(abs(namedRoot),{withFileTypes:true}).filter(row=>row.isDirectory()&&/^table-views-[0-9]+-[a-f0-9]{8}$/.test(row.name));assert.equal(namedDirs.length,1,'Exactly one finite actual final named-view receipt directory');const FINAL_NAMED=namedRoot+'/'+namedDirs[0].name+'/ui-table-views-live.json',FINAL_TRANSPORT=namedRoot+'/'+namedDirs[0].name+'/table-views-no-locks-transport.json';noLinks(path.posix.dirname(FINAL_NAMED));
 const named=json(FINAL_NAMED);assert.equal(named.result,'PASS');assert(!named.failure);assert.equal(named.binary_sha256,BINARY_SHA);assert(Array.isArray(named.owner_readiness)&&named.owner_readiness.length>0);assert.equal(named.native_captures.length,2);const ownerIds=new Set(named.native_captures.map(row=>row.id));assert(named.owner_readiness.every(row=>ownerIds.has(row.id)&&row.method==='GET'&&row.status===200&&row.request_finished===true&&row.DOM_owner_completed===true&&row.export_enabled===true&&row.timeout_ms===30000&&row.source==='actual_browser_final_refresh_dependency_and_committed_data'&&row.route===row.origin+'/api/crawls/'+encodeURIComponent(row.id)+'/fragments'));assert.equal(new Set(named.owner_readiness.map(row=>row.id)).size,2);assert.deepEqual(named.errors,[]);assert(named.native.every(owner=>owner.natural_eof===true&&owner.forced===false&&owner.close.code===0&&owner.close.signal===null));const finalTransport=json(FINAL_TRANSPORT);assert.equal(finalTransport.kind,'owned_loopback_passthrough');assert.notEqual(finalTransport.origin,finalTransport.upstream_origin);assert.deepEqual(finalTransport.failures,[]);assert(finalTransport.requests.some(row=>row.path.endsWith('/fragments')&&row.method==='GET'&&row.status===200&&row.complete===true));assert(named.owner_readiness.some(row=>row.origin===finalTransport.origin),'Actual final NoLocks owner readiness must bind the browser alias origin');const allowedOrigins=new Set([...named.native.map(row=>row.origin),finalTransport.origin]);assert(named.owner_readiness.every(row=>allowedOrigins.has(row.origin)),'Passthrough browser origin is distinct from the normal native owner origin');copy(FINAL_NAMED,'actual-R3-named-owner-readiness-receipt');copy(FINAL_TRANSPORT,'actual-R3-insecure-alias-passthrough-witness');assert.equal(copies.length,68);

 put('source-bindings.json',JSON.stringify({kind:'selected9_unchanged_source_binding',binary:{bytes:rust.binary.bytes,sha256:rust.binary.sha256,copied:false},final_Rust_authority:rustRoot,selected_compiler_inputs:426,current_bytes_match_all_selected_inputs:true,named_readiness_driver:{before_sha256:repair.before_sha256,after_sha256:aliasRepair.after_sha256,intermediate_R2_sha256:repair.after_sha256,immediate_Apply_assertions_unchanged:true,actual_owner_readiness_observations:named.owner_readiness.length,receipt:'raw/'+FINAL_NAMED},manifest:'raw/'+rustRoot+'/inputs.json',scope:rust.scope,coordinator_tracked_sources_unchanged:true,selected_child_current_sources:sources,registry:{path:'raw/scripts/ui-e2e.mjs',sha256:REGISTRY_SHA,registered:50,selected:9},qualification:'Current selected child hashes are archival source bindings; the original coordinator independently reports its tracked execution window unchanged. No transitive toolchain/dependency, whole metadata closure or independent child-data replay is inferred.'},null,2)+'\n');
 put('result.json',JSON.stringify({kind:'actual_selected9_affected_UI_regression_log_receipt',result:'PASS_SELECTED9_AFFECTED_REGRESSIONS_NATURAL_COORDINATOR_EOF0_LOG_RECEIPT_ONLY',selected:9,fixture:1,native:8,registered:50,selected_in_registry_order:expected,wrapper_exit:{code:0,signal:null},coordinator_shutdown:local.shutdown,binary_sha256:rust.binary.sha256,binary_unchanged:true,selected_compiler_inputs_unchanged:426,tracked_sources_unchanged:true,duration_ms:wrapper.duration_ms,retained_R2_failure:{complete:false,passed:2,failed:1,failed_suite:'table-named-views',remaining_six:'NOT_REACHED_NO_LOGS_INVENTED',binary_unchanged:true,coordinator_EOF0:true,repair:'Watch the actual browser-page origin for the existing real insecure NoLocks passthrough, with unchanged GET/finished/UI guard, assertions and deadlines.'},retained_R1_failure:{complete:false,passed:2,failed:1,failed_suite:'table-named-views',remaining_six:'NOT_REACHED_NO_LOGS_INVENTED',binary_unchanged:true,coordinator_EOF0:true,repair:'Selected-owner final refresh readiness before Apply; immediate Apply assertions and product bytes unchanged.'},actual_R3_owner_readiness_observations:named.owner_readiness.length,explicit_Rust_authority:rustRoot,copied:'Finite coordinator proof/app log; exact18 child stdout/stderr logs; adjacent wrapper/stdout/stderr; original runner; current registry; final Rust proof/input manifest; publisher; finite original R1/R2 failures, drivers and repairs; actual R3 named owner-readiness receipt and real insecure-alias passthrough witness. No recursive data selection.',local_only:'Except for the explicitly indexed named-view receipts and witnesses, other child JSON/data receipts, downloaded files, native/SQLite stores, screenshots and relation corpus remain in the original ignored local run directory. This publication does not independently replay their data oracles.',not_proven:['This is selected9, not one complete local50 invocation or exact successor publicCI.','The new project-columns native suite is a separate selected1 proof; it is not among these9 affected suites.','Historical failed Rust/native/public CI attempts remain failures in their own receipts.','Full backlog, all-table/operator coverage, project workspace bundles and manufacturer parity remain open.']},null,2)+'\n');
 put('README.md','# Native project columns: nine affected regressions\n\nThis finite child accepts one actual explicitly selected local invocation of9 existing suites, in their unchanged registry order: `'+EXPECTED.join('`, `')+'`. It contains1 API fixture and8 native suites. All coordinator results require code0, null signal and no timeout; the coordinator must close naturally on EOF with0, and wrapper/source/binary checks must remain unchanged. [Result](result.json), [source bindings](source-bindings.json) and [index](index.json) bind the exact finite original logs and stated final Rust authority.\n\nAll426 selected compiler/product inputs must match that final Rust manifest, with the same normal App binary as this invocation. The current50-suite registry is archived as source; only these9 suites are accepted here. The new `project-columns` selected1 native acceptance, complete local50 and exact successor publicCI are separate gates. Prior failed attempts retain their own result; choosing an explicit final Rust root does not relabel them.\n\nOriginal selected9 R1 remains FAIL: dashboard and table-explorer passed; table-named-views failed its immediate empty/zero Apply range check; six later suites were NOT_REACHED. Its original proof, wrapper, reached-suite logs, named receipt/failure HTML/native log, original driver and narrow readiness repair are preserved. The repair strengthens selected-owner data readiness before Apply while preserving every immediate Apply assertion and the426 compiled product inputs. Original selected9 R2 also remains FAIL: the same two initial suites passed, named-view failed its30000ms response wait at the genuine insecure NoLocks alias; six later suites were NOT_REACHED. Its original reached logs/receipts, native log, both failure HTML files, real passthrough witness, intermediate driver and one-origin repair are retained. No timeout or assertion was weakened. The actual R3 named receipt records the genuine final GET/200/finished plus committed-owner UI boundary. This is a log/receipt supplement. Except for the explicitly indexed named-view receipts and witnesses, other child JSON/data receipts, heavy downloaded files, SQLite/native stores, screenshots and relation corpus remain LOCAL ONLY in the ignored original run directory. No new independent replay of those data oracles is claimed. The original logs preserve their own actual declarations. Full feature backlog, native workspace bundles and manufacturer parity remain open. The helper creates only this fresh child and leaves any parent index unchanged.\n');
 put('index.json',JSON.stringify({kind:'finite_selected9_affected_regression_original_log_index',raw_files:copies,generated_files:[...outputs].filter(([name])=>!name.startsWith('raw/')).map(([name,bytes])=>({path:name,bytes:bytes.length,sha256:sha(bytes)})),index_self_excluded:true,scope:'68 explicit raw copies and4 generated metadata files; no corpus/store/download/binary copies. The reviewed output plan binds this index too.'},null,2)+'\n');
 const plan=[...outputs].map(([name,bytes])=>({path:DEST+'/'+name,bytes:bytes.length,sha256:sha(bytes)})).sort((a,b)=>a.path<b.path?-1:a.path>b.path?1:0),planSHA=sha(Buffer.from(JSON.stringify(plan))),report={result:apply?'MATERIALIZED_SELECTED9_FINITE_LOG_RECEIPT':'READY_VALIDATED_NO_WRITES',destination:DEST,explicit_Rust_authority:rustRoot,raw_files:copies.length,generated_files:4,files:outputs.size,bytes:plan.reduce((total,row)=>total+row.bytes,0),plan_sha256:planSHA,index_sha256:sha(outputs.get('index.json')),publisher_sha256:sha(inputs.get(SELF))};
 if(!apply){console.log(JSON.stringify(report,null,2));return;}assert.equal(reviewedPlan,planSHA);for(const[file,bytes]of inputs){noLinks(file);assert.deepEqual(fs.readFileSync(abs(file)),bytes,'Input changed: '+file);}noLinks(DEST,true);assert(!fs.existsSync(abs(DEST)));fs.mkdirSync(abs(DEST));for(const[name,bytes]of outputs){const file=DEST+'/'+name,parent=path.posix.dirname(file);noLinks(parent,true);fs.mkdirSync(abs(parent),{recursive:true});noLinks(parent);fs.writeFileSync(abs(file),bytes,{flag:'wx'});assert.deepEqual(fs.readFileSync(abs(file)),bytes);}console.log(JSON.stringify(report,null,2));
}
try{main();}catch(error){console.error((error instanceof Pending?'PENDING_SELECTED9_TERMINAL_NO_WRITES: ':'')+error.stack);process.exitCode=error instanceof Pending?2:1;}
