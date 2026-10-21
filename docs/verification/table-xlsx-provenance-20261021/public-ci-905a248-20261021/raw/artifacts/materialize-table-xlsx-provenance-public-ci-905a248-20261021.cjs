'use strict';
// Finite offline supplement proposal. --validate is read-only; Root alone may
// --apply <exact printed plan SHA>. No Git, network, tests, browser or runtime.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const ROOT=path.resolve(__dirname,'..');
const SOURCE='artifacts/table-xlsx-provenance-public-ci-905a248-20261021';
const DEST='docs/verification/table-xlsx-provenance-20261021/public-ci-905a248-20261021';
const SELF='artifacts/materialize-table-xlsx-provenance-public-ci-905a248-20261021.cjs';
const COLLECTOR='artifacts/collect-table-xlsx-provenance-public-ci-905a248-20261021.cjs';
const COMMIT='905a2480c54d431a42eae1ed93e43f094a3e7a76',TREE='6afaa4c322c9d467aa1dcf7d2951284bc6bc589a',RUN=39609409363,RENDERER=119254001915;
const SHA=value=>createHash('sha256').update(value).digest('hex');
const PINS=new Map([
  [
    "run.json",
    "73563f056c280fba388ac5ba4b2f24849bc480389ebeb6f4cc54d28ee79344f9"
  ],
  [
    "jobs.json",
    "830aea0cd473ddc788babf67a99de14733ca257361629e581b98d6af251558fd"
  ],
  [
    "renderer.log",
    "8dd15c19f0c7b3f1dab9faddd50a49781334be2d000bc473bef4fc7cd5dfdc61"
  ],
  [
    "container.log",
    "0a1e56b4f094445ee0a3b608c10e24553808d5b6f1d3647c7fd39d4bdb373f9f"
  ],
  [
    "ubuntu.log",
    "39f3cf2c9ccccff384dd7fe5caca5d9215a90ba875c09c7c0dd7c3789be4b4c1"
  ],
  [
    "windows.log",
    "43a2bd26a59af707fa0ce5e0bfb758ad6a4ecd2c9e374e2f3f746f30f48044bb"
  ],
  [
    "registered-ui-e2e.mjs",
    "329c6e82a3f9f601ac15dfa479c45374099fe9e871fed35fc013ed98491abc92"
  ],
  [
    "committed-ci.yml",
    "274bcddb4563878639a4df5d173592a63ab62c310676f36c49413cd8128ea75f"
  ],
  [
    "receipt.json",
    "3b5d4c837a560609833358526ea87752e30f76e7f78c0103e42f0a9cdfa4f576"
  ]
]);
const JOBS=new Map([[119254002447,'test (windows-latest)'],[119254002116,'test (ubuntu-latest)'],[RENDERER,'renderer'],[119254002100,'container']]);
function relative(value){
  assert.equal(typeof value,'string');assert(value&&value===value.replaceAll('\\','/')&&!path.isAbsolute(value)&&!/[\u0000-\u001f\u007f:]/u.test(value),'Unsafe relative path');
  assert(value.split('/').every(part=>part&&part!=='.'&&part!=='..'),'Unsafe path segment');return value;
}
function absolute(value){const result=path.resolve(ROOT,relative(value));assert(result.startsWith(ROOT+path.sep),'Outside repository');return result;}
function noLinks(value,missing=false){
  let current=ROOT;const parts=relative(value).split('/');
  for(let i=0;i<parts.length;i++){
    current=path.join(current,parts[i]);
    let stat;try{stat=fs.lstatSync(current);}catch(error){if(error.code==='ENOENT'&&missing)return;throw error;}
    assert(!stat.isSymbolicLink(),'No link/junction allowed: '+value);assert(i===parts.length-1||stat.isDirectory(),'Non-directory parent: '+value);
  }
}
function main(){
  const args=process.argv.slice(2),apply=args[0]==='--apply';
  assert(args.length===0||(args.length===1&&args[0]==='--validate')||(args.length===2&&apply&&/^[a-f0-9]{64}$/.test(args[1])),'Use --validate or --apply <reviewed plan SHA256>');
  assert.equal(path.resolve(process.cwd()),ROOT,'Run from owned repository');assert(fs.lstatSync(ROOT).isDirectory()&&!fs.lstatSync(ROOT).isSymbolicLink());
  noLinks(DEST,true);assert(!fs.existsSync(absolute(DEST)),'Fresh supplement only; nothing is overwritten');
  noLinks(path.posix.dirname(DEST));assert(fs.lstatSync(absolute(path.posix.dirname(DEST))).isDirectory());
  noLinks('.gitattributes');const attributes=fs.readFileSync(absolute('.gitattributes'),'utf8');assert(attributes.includes('/docs/verification/table-xlsx-provenance-20261021/** -text'),'Existing raw-preserving destination policy required');
  const inputs=new Map(),outputs=new Map(),copies=[];
  function read(file,expected){
    noLinks(file);const full=absolute(file),stat=fs.lstatSync(full);assert(stat.isFile()&&stat.size<32*1024*1024,'Bounded regular source file required: '+file);
    const bytes=fs.readFileSync(full),digest=SHA(bytes);if(expected)assert.equal(digest,expected,'Frozen input changed: '+file);
    const prior=inputs.get(file);if(prior)assert.deepEqual(bytes,prior.bytes,'Input changed during preflight: '+file);else inputs.set(file,{bytes,sha256:digest});return bytes;
  }
  function put(name,bytes){relative(name);assert(!outputs.has(name),'Duplicate destination');outputs.set(name,Buffer.isBuffer(bytes)?bytes:Buffer.from(bytes));}
  const putJSON=(name,value)=>put(name,JSON.stringify(value,null,2)+'\n');
  function copy(file,category,expected){const bytes=read(file,expected),destination='raw/'+file;put(destination,bytes);copies.push({path:file,bytes:bytes.length,sha256:SHA(bytes),destination,category});return bytes;}
  const raw=new Map();for(const[name,digest]of PINS)raw.set(name,copy(SOURCE+'/'+name,'terminal-public-ci',digest));
  copy(COLLECTOR,'read-only-collector','0ffe5af6b8a9625ce1033cf0b33efa50bc82c526f0792d1164961d8e97e8312c');copy(SELF,'offline-materializer');
  const run=JSON.parse(raw.get('run.json')),jobResponse=JSON.parse(raw.get('jobs.json')),jobs=jobResponse.jobs,receipt=JSON.parse(raw.get('receipt.json'));
  assert.equal(run.id,RUN);assert.equal(run.head_sha,COMMIT);assert.equal(run.head_branch,'main');assert.equal(run.event,'push');assert.equal(run.run_attempt,1);assert.equal(run.status,'completed');assert.equal(run.conclusion,'success');
  assert.equal(jobResponse.total_count,4);assert.equal(jobs.length,4);assert.equal(new Set(jobs.map(job=>job.id)).size,4);
  for(const job of jobs){assert.equal(JOBS.get(job.id),job.name);assert.equal(job.run_id,RUN);assert.equal(job.head_sha,COMMIT);assert.equal(job.status,'completed');assert.equal(job.conclusion,'success');assert(job.steps.every(step=>step.conclusion!=='failure'));}
  assert.equal(receipt.result,'PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI48_INVOCATION');assert.equal(receipt.commit,COMMIT);assert.equal(receipt.tree,TREE);assert.equal(receipt.run,RUN);assert.equal(receipt.status,'completed');assert.equal(receipt.conclusion,'success');assert.equal(receipt.run_attempt,1);assert.equal(receipt.log_errors.length,0);
  assert.equal(receipt.raw.length,6);assert.deepEqual(receipt.raw.map(row=>row.path),[...PINS.keys()].slice(0,6));
  for(const item of receipt.raw){const bytes=raw.get(item.path);assert.equal(bytes.length,item.bytes);assert.equal(SHA(bytes),item.sha256);assert.equal(item.sha256,PINS.get(item.path));}
  const registry=raw.get('registered-ui-e2e.mjs'),workflow=raw.get('committed-ci.yml');
  assert.deepEqual(receipt.registered_suite_source,{commit:COMMIT,path:'scripts/ui-e2e.mjs',local_copy:'registered-ui-e2e.mjs',bytes:registry.length,sha256:SHA(registry)});
  assert.deepEqual(receipt.workflow_source,{commit:COMMIT,path:'.github/workflows/ci.yml',local_copy:'committed-ci.yml',bytes:workflow.length,sha256:SHA(workflow)});
  const expected=[...registry.toString('utf8').matchAll(/^\s+\['([a-z0-9-]+)',\s*'([^']+)',\s*'(fixture|live|owned-native)'/gm)].map(match=>({name:match[1],script:match[2],kind:match[3]}));
  assert.equal(expected.length,48);assert.equal(new Set(expected.map(row=>row.name)).size,48);assert.equal(expected.filter(row=>row.kind==='fixture').length,9);
  const log=raw.get('renderer.log').toString('utf8').replace(/\x1b\[[0-9;]*m/g,''),banners=[...log.matchAll(/^\d{4}-[^\r\n ]+ ##\[group\]Run node scripts\/ui-e2e\.mjs\r?$/gm)];
  assert.equal(banners.length,1,'One canonical unfiltered coordinator banner');
  const start=banners[0].index,after=log.slice(start),newline=after.indexOf('\n'),secondNewline=after.indexOf('\n',newline+1);
  assert.equal(after.slice(newline+1,secondNewline).replace(/^\d{4}-[^\r\n ]+ /,'').trim(),'node scripts/ui-e2e.mjs','Exact unfiltered command echo');
  const records=value=>[...value.matchAll(/^\d{4}-[^\r\n ]+ PASS ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].map(match=>({name:match[1],kind:match[2]}));
  const observed=records(log),nextStep=log.indexOf('##[group]Run ',start+banners[0][0].length);
  assert.equal(observed.length,48);assert.equal(new Set(observed.map(row=>row.name)).size,48);assert.deepEqual(observed,expected.map(({name,kind})=>({name,kind})));assert.deepEqual(records(log.slice(start,nextStep<0?undefined:nextStep)),observed,'All suite records belong to one invocation');
  assert.equal([...log.matchAll(/^\d{4}-[^\r\n ]+ FAIL ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].length,0);
  assert(observed.some(row=>row.name==='table-xlsx'&&row.kind==='owned-native'));assert(observed.some(row=>row.name==='table-xlsx-provenance'&&row.kind==='owned-native'));assert(observed.some(row=>row.name==='table-xlsx-availability'&&row.kind==='owned-native'));
  const renderer=jobs.find(job=>job.id===RENDERER),steps=renderer.steps.filter(step=>step.name==='Redesigned UI fixture and native end-to-end workflows');assert.equal(steps.length,1);assert.equal(steps[0].status,'completed');assert.equal(steps[0].conclusion,'success');
  assert.deepEqual(receipt.renderer.UI_step,steps[0]);assert.deepEqual(receipt.renderer.suites_in_committed_order,expected);assert.deepEqual(receipt.renderer.observed,observed);assert.equal(receipt.renderer.accepted,true);assert.equal(receipt.renderer.canonical_unfiltered_invocations,1);assert.equal(receipt.renderer.passed_suites,48);assert.equal(receipt.renderer.unique_passed_suites,48);assert.equal(receipt.renderer.fixture_suites,9);assert.equal(receipt.renderer.native_suites,39);
  const scope="Exact published main905a248 only. All four public CI jobs and one unfiltered48-suite invocation passed. This closes the previously pending public CI/full registered48 gate for that commit; no fresh full local48 run is inferred. The earlier609 selected input set has608 unchanged files and one separately qualified post-gate whitespace policy; it is not a transitive dependency/toolchain or whole metadata closure. Original native R1/R2/R3 failures remain FAIL. The original R3 provenance35 PASS belongs to its failed aggregate, original R4 availability8 PASS is selected-only, and local focused20 PASS remains separate. Duplicate branch39609409247 cancellation is separate from this main run. Subsequent extraction-source-trust work and its current49-suite registry are outside905a248. No all29-schema/operator, specialist/cursor/bulk/streaming/multisheet XLSX acceptance, arbitrary corruption/crash/resource behavior, Excel/LibreOffice GUI interoperability, installed upgrade, full workflow completion or manufacturer parity is inferred.";
  const summary={schema_version:1,kind:'exact_published_source_availability_provenance_public_ci_supplement',result:'PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI48_INVOCATION',commit:COMMIT,tree:TREE,run:RUN,url:run.html_url,attempt:1,head_branch:'main',status:'completed',conclusion:'success',collected_at:receipt.collected_at,jobs:jobs.map(({id,name,status,conclusion,started_at,completed_at})=>({id,name,status,conclusion,started_at,completed_at})),renderer:{canonical_unfiltered_invocations:1,unique_passed_suites:48,fixture_suites:9,native_suites:39,suites_in_committed_order:expected,UI_step:steps[0]},raw_receipt:'raw/'+SOURCE+'/receipt.json',scope};
  putJSON('public-ci-result.json',summary);
  put('README.md',"# XLSX availability and provenance: exact published public CI, 2026-10-21\n\nThe exact published main commit `905a2480c54d431a42eae1ed93e43f094a3e7a76` (tree `6afaa4c322c9d467aa1dcf7d2951284bc6bc589a`) passed all four jobs in [run 39609409363](https://github.com/lia-xim/open-seo-spider/actions/runs/39609409363): Windows, Ubuntu, Renderer and container. This supplement preserves their complete raw logs plus original run/jobs API responses.\n\nThe raw Renderer log proves **48 distinct suite PASS records in one unfiltered coordinator invocation**: 9 fixture suites and 39 native suites, including `table-xlsx`, `table-xlsx-provenance` and `table-xlsx-availability`. Their names, kinds and order match the exact committed [registry](raw/artifacts/table-xlsx-provenance-public-ci-905a248-20261021/registered-ui-e2e.mjs); the exact committed workflow is preserved separately. [Public result](public-ci-result.json), [original receipt](raw/artifacts/table-xlsx-provenance-public-ci-905a248-20261021/receipt.json) and [copy index](index.json) bind the conclusions and all selected bytes. The read-only collector and this offline materializer are archived unchanged; relocation is not a promise that those helpers run from their archived paths.\n\nThis closes the previously pending public CI/full registered48 gate for the published availability/provenance revision. It does not turn the separate local focused20 invocation into a full local48 run. Original native R1/R2/R3 failures remain failures: the original R3 provenance35 PASS belongs to its failed aggregate, while original R4 availability8 PASS is selected-only. The earlier609-input local manifest has608 unchanged files and one separately qualified post-gate whitespace policy; it is an explicit selected input set rather than a transitive dependency/toolchain or whole metadata closure. Duplicate branch run39609409247 was cancelled separately and is not this successful main run.\n\nSubsequent extraction-source-trust changes and their current49-suite registry are outside this commit and need their own proof. No all29-schema/operator acceptance, specialist/cursor/bulk/large streaming or multisheet exports, Excel/LibreOffice GUI interoperability, installed application upgrade, full workflow completion or manufacturer parity is inferred. Existing144/416/32 backlog totals and39/68/37 feature states remain unchanged. This helper creates only this fresh child supplement; it does not rewrite the parent evidence index, tracked documents or feature statuses.\n");
  const index={schema_version:1,kind:'byte_exact_public_ci48_supplement_index',commit:COMMIT,tree:TREE,run:RUN,raw_files:copies,generated_files:[...outputs].filter(([name])=>!name.startsWith('raw/')).map(([name,bytes])=>({path:name,bytes:bytes.length,sha256:SHA(bytes)})),excluded:['public CI artifact ZIPs and their contents','job runtime workspaces/toolchains/dependencies','binary/store/browser runtime/user data','later successor local or public results'],scope:'Every raw_files entry is copied byte-for-byte from its finite explicit source. Generated files are derived summaries, not raw output. index.json excludes its own hash; the reviewed output plan binds index.json too.'};
  putJSON('index.json',index);
  const plan=[...outputs].map(([name,bytes])=>({path:DEST+'/'+name,bytes:bytes.length,sha256:SHA(bytes)})).sort((a,b)=>a.path.localeCompare(b.path));
  const planSHA=SHA(Buffer.from(JSON.stringify(plan))),result={result:apply?'MATERIALIZED_PUBLIC_CI48_SUPPLEMENT':'READY_VALIDATED_NO_WRITES',destination:DEST,commit:COMMIT,run:RUN,raw_files:copies.length,generated_files:3,output_files:outputs.size,total_bytes:plan.reduce((sum,row)=>sum+row.bytes,0),plan_sha256:planSHA,materializer_sha256:inputs.get(SELF).sha256,index_sha256:SHA(outputs.get('index.json'))};
  if(!apply){console.log(JSON.stringify(result,null,2));return;}
  assert.equal(args[1],planSHA,'Exact reviewed output plan SHA required');
  // Recheck all finite inputs before the first public write. Never overwrite,
  // delete, traverse a link, or update an existing parent evidence index.
  for(const[file,item]of inputs){noLinks(file);assert.deepEqual(fs.readFileSync(absolute(file)),item.bytes,'Input changed after plan: '+file);}
  noLinks(DEST,true);assert(!fs.existsSync(absolute(DEST)));fs.mkdirSync(absolute(DEST));
  for(const[name,bytes]of outputs){const output=DEST+'/'+name,parent=path.posix.dirname(output);noLinks(parent,true);fs.mkdirSync(absolute(parent),{recursive:true});noLinks(parent);assert(!fs.existsSync(absolute(output)));fs.writeFileSync(absolute(output),bytes,{flag:'wx'});assert.equal(SHA(fs.readFileSync(absolute(output))),SHA(bytes));}
  console.log(JSON.stringify(result,null,2));
}
try{main();}catch(error){console.error(error.stack);process.exitCode=1;}
