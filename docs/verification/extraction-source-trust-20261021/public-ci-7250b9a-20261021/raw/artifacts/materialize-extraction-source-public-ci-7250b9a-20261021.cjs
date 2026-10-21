'use strict';
// Finite offline supplement proposal. --validate is read-only; Root alone may
// --apply <exact printed plan SHA>. No Git, network, tests, browser or runtime.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const ROOT=path.resolve(__dirname,'..');
const SOURCE='artifacts/extraction-source-public-ci-7250b9a-20261021';
const DEST='docs/verification/extraction-source-trust-20261021/public-ci-7250b9a-20261021';
const SELF='artifacts/materialize-extraction-source-public-ci-7250b9a-20261021.cjs';
const COLLECTOR='artifacts/collect-extraction-source-public-ci-7250b9a-20261021.cjs';
const COMMIT='7250b9a0007d0f307321e3b4c7076fa95430dbc3',TREE='d5e2252877fc74c9de52872ecfec04f544856b5c',RUN=39638818285,RENDERER=119356860489;
const SHA=value=>createHash('sha256').update(value).digest('hex');
const PINS=new Map([
  [
    "run.json",
    "5adb495e9d050f60093a2e6a209ef37ad21fe7f5f7a1c7e6c86805434814301e"
  ],
  [
    "jobs.json",
    "7184c27b5c45a0ed1aebf44ea8a6bf527a2072b1ba5a48accc9e7fb512088f6b"
  ],
  [
    "renderer.log",
    "f38519afddc71e109f3351e88ec66c3733d1e1c5a72ef23d1b87766331c9ab28"
  ],
  [
    "windows.log",
    "bc8ca1541ce4880b6c5e12f2c4187aa4574fe327c91a91ce203f58d8e08cfa07"
  ],
  [
    "container.log",
    "0ca1f8124f9af9b23bd2c8bdc638498ab7e582598b499759044901a04a83544f"
  ],
  [
    "ubuntu.log",
    "d957f4d07e755713a863b41ac2cfcd8576e698754dfa70e811827a835dafae47"
  ],
  [
    "registered-ui-e2e.mjs",
    "766e8d128bf54d94263ab6bcd55789eacf248124e04c6fc49903e7ccfed88841"
  ],
  [
    "committed-ci.yml",
    "33999a2019e25edfdc8e785744511a71ea6c24695c5370e1627756f25ecc1840"
  ],
  [
    "receipt.json",
    "58c7cfe1239364c9b86dfe02e663319d1e0276d0c464bb6ba5c0fa4736f6f491"
  ]
]);
const JOBS=new Map([[119356860870,'test (windows-latest)'],[119356861256,'test (ubuntu-latest)'],[RENDERER,'renderer'],[119356861162,'container']]);
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
  noLinks('.gitattributes');const attributes=fs.readFileSync(absolute('.gitattributes'),'utf8');assert(attributes.includes('/docs/verification/extraction-source-trust-20261021/** -text'),'Existing raw-preserving destination policy required');
  const inputs=new Map(),outputs=new Map(),copies=[];
  function read(file,expected){
    noLinks(file);const full=absolute(file),stat=fs.lstatSync(full);assert(stat.isFile()&&stat.size<32*1024*1024,'Bounded regular source file required: '+file);
    const bytes=fs.readFileSync(full),digest=SHA(bytes);if(expected)assert.equal(digest,expected,'Frozen input changed: '+file);
    const prior=inputs.get(file);if(prior)assert.deepEqual(bytes,prior.bytes,'Input changed during preflight: '+file);else inputs.set(file,{bytes,sha256:digest});return bytes;
  }
  function put(name,bytes){relative(name);assert(!outputs.has(name),'Duplicate destination');outputs.set(name,Buffer.isBuffer(bytes)?bytes:Buffer.from(bytes));}
  const putJSON=(name,value)=>put(name,JSON.stringify(value,null,2)+'\n');
  read('docs/verification/extraction-source-trust-20261021/index.json','6acc698667e4306d60ed6e8c0fcc563bc86d65fa1926d6901ca26d81c25b4e0d');
  read('.gitattributes');
  function copy(file,category,expected){const bytes=read(file,expected),destination='raw/'+file;put(destination,bytes);copies.push({path:file,bytes:bytes.length,sha256:SHA(bytes),destination,category});return bytes;}
  const raw=new Map();for(const[name,digest]of PINS)raw.set(name,copy(SOURCE+'/'+name,'terminal-public-ci',digest));
  copy(COLLECTOR,'read-only-collector','9dc682b4c353afde88fce9fbcdb0eb2760feeb94892cd9e2058d09c3989e9f89');copy(SELF,'offline-materializer');
  const run=JSON.parse(raw.get('run.json')),jobResponse=JSON.parse(raw.get('jobs.json')),jobs=jobResponse.jobs,receipt=JSON.parse(raw.get('receipt.json'));
  assert.equal(run.id,RUN);assert.equal(run.head_sha,COMMIT);assert.equal(run.head_branch,'main');assert.equal(run.event,'push');assert.equal(run.run_attempt,1);assert.equal(run.status,'completed');assert.equal(run.conclusion,'success');
  assert.equal(jobResponse.total_count,4);assert.equal(jobs.length,4);assert.equal(new Set(jobs.map(job=>job.id)).size,4);
  for(const job of jobs){assert.equal(JOBS.get(job.id),job.name);assert.equal(job.run_id,RUN);assert.equal(job.head_sha,COMMIT);assert.equal(job.status,'completed');assert.equal(job.conclusion,'success');assert(job.steps.every(step=>step.conclusion!=='failure'));}
  assert.equal(receipt.result,'PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI49_INVOCATION');assert.equal(receipt.commit,COMMIT);assert.equal(receipt.tree,TREE);assert.equal(receipt.run,RUN);assert.equal(receipt.status,'completed');assert.equal(receipt.conclusion,'success');assert.equal(receipt.run_attempt,1);assert.equal(receipt.log_errors.length,0);
  assert.equal(receipt.raw.length,6);assert.deepEqual(receipt.raw.map(row=>row.path),[...PINS.keys()].slice(0,6));
  for(const item of receipt.raw){const bytes=raw.get(item.path);assert.equal(bytes.length,item.bytes);assert.equal(SHA(bytes),item.sha256);assert.equal(item.sha256,PINS.get(item.path));}
  const registry=raw.get('registered-ui-e2e.mjs'),workflow=raw.get('committed-ci.yml');
  assert.deepEqual(receipt.registered_suite_source,{commit:COMMIT,path:'scripts/ui-e2e.mjs',local_copy:'registered-ui-e2e.mjs',bytes:registry.length,sha256:SHA(registry)});
  assert.deepEqual(receipt.workflow_source,{commit:COMMIT,path:'.github/workflows/ci.yml',local_copy:'committed-ci.yml',bytes:workflow.length,sha256:SHA(workflow)});
  const expected=[...registry.toString('utf8').matchAll(/^\s+\['([a-z0-9-]+)',\s*'([^']+)',\s*'(fixture|live|owned-native)'/gm)].map(match=>({name:match[1],script:match[2],kind:match[3]}));
  assert.equal(expected.length,49);assert.equal(new Set(expected.map(row=>row.name)).size,49);assert.equal(expected.filter(row=>row.kind==='fixture').length,9);
  const log=raw.get('renderer.log').toString('utf8').replace(/\x1b\[[0-9;]*m/g,''),banners=[...log.matchAll(/^\d{4}-[^\r\n ]+ ##\[group\]Run node scripts\/ui-e2e\.mjs\r?$/gm)];
  assert.equal(banners.length,1,'One canonical unfiltered coordinator banner');
  const start=banners[0].index,after=log.slice(start),newline=after.indexOf('\n'),secondNewline=after.indexOf('\n',newline+1);
  assert.equal(after.slice(newline+1,secondNewline).replace(/^\d{4}-[^\r\n ]+ /,'').trim(),'node scripts/ui-e2e.mjs','Exact unfiltered command echo');
  const records=value=>[...value.matchAll(/^\d{4}-[^\r\n ]+ PASS ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].map(match=>({name:match[1],kind:match[2]}));
  const observed=records(log),nextStep=log.indexOf('##[group]Run ',start+banners[0][0].length);
  assert.equal(observed.length,49);assert.equal(new Set(observed.map(row=>row.name)).size,49);assert.deepEqual(observed,expected.map(({name,kind})=>({name,kind})));assert.deepEqual(records(log.slice(start,nextStep<0?undefined:nextStep)),observed,'All suite records belong to one invocation');
  assert.equal([...log.matchAll(/^\d{4}-[^\r\n ]+ FAIL ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].length,0);
  assert(observed.some(row=>row.name==='table-xlsx'&&row.kind==='owned-native'));assert(observed.some(row=>row.name==='table-xlsx-provenance'&&row.kind==='owned-native'));assert(observed.some(row=>row.name==='table-xlsx-availability'&&row.kind==='owned-native'));
  const renderer=jobs.find(job=>job.id===RENDERER),steps=renderer.steps.filter(step=>step.name==='Redesigned UI fixture and native end-to-end workflows');assert.equal(steps.length,1);assert.equal(steps[0].status,'completed');assert.equal(steps[0].conclusion,'success');
  assert.deepEqual(receipt.renderer.UI_step,steps[0]);assert.deepEqual(receipt.renderer.suites_in_committed_order,expected);assert.deepEqual(receipt.renderer.observed,observed);assert.equal(receipt.renderer.accepted,true);assert.equal(receipt.renderer.canonical_unfiltered_invocations,1);assert.equal(receipt.renderer.passed_suites,49);assert.equal(receipt.renderer.unique_passed_suites,49);assert.equal(receipt.renderer.fixture_suites,9);assert.equal(receipt.renderer.native_suites,40);
  const scope="Exact published main7250b9a only. All four public CI jobs and one complete unfiltered49-suite invocation passed:9 fixture and40 native. This closes the full registered49 public CI gate only for that immutable commit. A separate actual full local49 invocation is published in its own finite log-only child; this public CI result does not stand in for that local proof or independently replay its heavy child data corpora. Original2c573d9 public UI/dashboard failure remains FAIL with zero completed canonical suites and48 NOT_REACHED; its later overall cancellation is separately preserved. The selected1 local corrected dashboard fixture remains a separate invocation, with original legacy unknown rows and explicit mocked complete controls. Original Rust R1/R2 and native extraction R1/R2/R3 failures remain FAIL; native extraction R4 selected1 and the existing named-view/expression selected2 remain separately scoped in their original receipts. The earlier423 selected compiler/product inputs are an explicit set, not transitive toolchain/dependency or whole metadata closure. App enrichment cfg(test) opt-ins are not inferred from normal binary identity. Prior905a248 public48 and22fc522 public46 apply only to those predecessor commits. Duplicate branch39638818133 cancellation is separate. Subsequent native project-column implementation and any successor50-suite registry require fresh proof. No full backlog, all29-schema/operator, specialist/cursor/bulk/large streaming/multisheet exports, Excel/LibreOffice GUI, installed upgrade, arbitrary crash/corruption schedules, whole workflow completion or manufacturer parity is inferred.";
  const summary={schema_version:1,kind:'exact_published_extraction_source_trust_public_ci_supplement',result:'PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI49_INVOCATION',commit:COMMIT,tree:TREE,run:RUN,url:run.html_url,attempt:1,head_branch:'main',status:'completed',conclusion:'success',collected_at:receipt.collected_at,jobs:jobs.map(({id,name,status,conclusion,started_at,completed_at})=>({id,name,status,conclusion,started_at,completed_at})),renderer:{canonical_unfiltered_invocations:1,unique_passed_suites:49,fixture_suites:9,native_suites:40,suites_in_committed_order:expected,UI_step:steps[0]},raw_receipt:'raw/'+SOURCE+'/receipt.json',scope};
  putJSON('public-ci-result.json',summary);
  put('README.md',"# Extraction source trust: exact published public CI, 2026-10-21\n\nThe exact published main commit `7250b9a0007d0f307321e3b4c7076fa95430dbc3` (tree `d5e2252877fc74c9de52872ecfec04f544856b5c`) passed all four jobs in [run39638818285](https://github.com/lia-xim/open-seo-spider/actions/runs/39638818285): Windows, Ubuntu, Renderer and container. This supplement preserves their complete raw logs and original terminal run/jobs API responses.\n\nThe actual Renderer log contains **49 distinct suite PASS records in one unfiltered coordinator invocation**:9 API fixtures and40 native suites, including `extraction-source-trust` and the three existing XLSX suites. Names, kinds and order match the exact committed registry; the exact committed workflow is preserved separately. [Public result](public-ci-result.json), [original receipt](raw/artifacts/extraction-source-public-ci-7250b9a-20261021/receipt.json) and [copy index](index.json) bind all selected bytes. The collector and finite materializer are archived unchanged; relocation does not make them runnable from the archived path.\n\nThis closes the previously pending full registered49 public CI gate for immutable725 only. The [separate complete local49 invocation](../local-ui49-7250b9a-20261021/README.md) has its own finite log receipt; neither publication newly replays all heavy child data oracles. Original2c573d9 dashboard/UI failure remains FAIL: zero completed canonical suites, remaining48 NOT_REACHED, later cancellation of its other jobs qualified separately. The corrected selected1 local fixture and the existing native R4/selected2 observations remain distinct invocations. Original failed Rust and native attempts retain their original bytes and declarations.\n\nThe423 selected compiler/product inputs are an explicit source set, rather than transitive toolchain/dependency or whole metadata closure. Prior905a248 public48 and22fc522 public46 are limited to those commits. Duplicate branch39638818133 cancellation is separate. Any subsequent project-column implementation or successor50-suite registry needs its own proof. No all-table/operator, specialist/cursor/bulk/large streaming/multisheet export, Excel/LibreOffice GUI, installed upgrade, full backlog/workflow or manufacturer parity is inferred. This child creates no raw artifact ZIP/store/binary copies and does not rewrite its parent index or feature counts.\n");
  const index={schema_version:1,kind:'byte_exact_public_ci49_supplement_index',commit:COMMIT,tree:TREE,run:RUN,raw_files:copies,generated_files:[...outputs].filter(([name])=>!name.startsWith('raw/')).map(([name,bytes])=>({path:name,bytes:bytes.length,sha256:SHA(bytes)})),excluded:['public CI artifact ZIPs and their contents','job runtime workspaces/toolchains/dependencies','binary/store/browser runtime/user data','the separate local49 child corpus and successor project-column work'],scope:'Every raw_files entry is copied byte-for-byte from its finite explicit source. Generated files are derived summaries, not raw output. index.json excludes its own hash; the reviewed output plan binds index.json too.'};
  putJSON('index.json',index);
  const plan=[...outputs].map(([name,bytes])=>({path:DEST+'/'+name,bytes:bytes.length,sha256:SHA(bytes)})).sort((a,b)=>a.path.localeCompare(b.path));
  const planSHA=SHA(Buffer.from(JSON.stringify(plan))),result={result:apply?'MATERIALIZED_PUBLIC_CI49_SUPPLEMENT':'READY_VALIDATED_NO_WRITES',destination:DEST,commit:COMMIT,run:RUN,raw_files:copies.length,generated_files:3,output_files:outputs.size,total_bytes:plan.reduce((sum,row)=>sum+row.bytes,0),plan_sha256:planSHA,materializer_sha256:inputs.get(SELF).sha256,index_sha256:SHA(outputs.get('index.json'))};
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
