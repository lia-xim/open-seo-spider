'use strict';
// Finite offline supplement proposal. --validate is read-only; Root alone may
// --apply <exact printed plan SHA>. No Git, network, tests, browser or runtime.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const ROOT=path.resolve(__dirname,'..');
const SOURCE='artifacts/table-xlsx-public-ci-22fc522-20261021';
const DEST='docs/verification/table-xlsx-export-20261021/public-ci-22fc522-20261021';
const SELF='artifacts/materialize-table-xlsx-public-ci-22fc522-20261021.cjs';
const COLLECTOR='artifacts/collect-table-xlsx-public-ci-22fc522-20261021.cjs';
const COMMIT='22fc5220333c7a1e5a1204ad11692a8d1b62ac76',TREE='32ccac14f1716e50a1115bf0312d3bec6903a1d0',RUN=39599316343,RENDERER=119219455951;
const SHA=value=>createHash('sha256').update(value).digest('hex');
const PINS=new Map([
  ['run.json','87c47bbf229e46e2a64c78a324c062a56599adafb26cab10a08ed5634a65d925'],
  ['jobs.json','55c80baf6d0c5b8e0a657ce80a59e0bb3798ee23ec8db7c7a6b0a562e4f71def'],
  ['windows.log','dbba0752d30881fbd357e8e332d9076cf52a2a785820f4ca39eddf16c564dbee'],
  ['ubuntu.log','37f258696c3a28db0d8ab85a772ff2fed5ea8c7b103a9a0d4143c127eda80929'],
  ['renderer.log','27213d347f011032cad23051548ea81624b0b50413fddc880d668f8b3df11fb0'],
  ['container.log','949a35df179653c4714266bb666d7e598c5f6cdb3fc998be0a9f88029ddf3ea3'],
  ['registered-ui-e2e.mjs','c086354cbef8d656872155961da8310e8c22a101a5602a51bf48f8b06aef0b86'],
  ['committed-ci.yml','4e7e23c410d1e1b61602427231ca9883d86a01600c7a0245c1dd5dee46a68667'],
  ['receipt.json','35fcaa778d4fb81b83a7956601c9968c60c55e1a1936acf0638baba71388c032']
]);
const JOBS=new Map([[119219455758,'test (windows-latest)'],[119219455948,'test (ubuntu-latest)'],[RENDERER,'renderer'],[119219456437,'container']]);
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
  noLinks('.gitattributes');const attributes=fs.readFileSync(absolute('.gitattributes'),'utf8');assert(attributes.includes('/docs/verification/table-xlsx-export-20261021/** -text'),'Existing raw-preserving destination policy required');
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
  copy(COLLECTOR,'read-only-collector','3610e58baf9194bf368b60260aa00baebd082681c385b3a4e847797a22a420f4');copy(SELF,'offline-materializer');
  const run=JSON.parse(raw.get('run.json')),jobResponse=JSON.parse(raw.get('jobs.json')),jobs=jobResponse.jobs,receipt=JSON.parse(raw.get('receipt.json'));
  assert.equal(run.id,RUN);assert.equal(run.head_sha,COMMIT);assert.equal(run.head_branch,'main');assert.equal(run.event,'push');assert.equal(run.run_attempt,1);assert.equal(run.status,'completed');assert.equal(run.conclusion,'success');
  assert.equal(jobResponse.total_count,4);assert.equal(jobs.length,4);assert.equal(new Set(jobs.map(job=>job.id)).size,4);
  for(const job of jobs){assert.equal(JOBS.get(job.id),job.name);assert.equal(job.run_id,RUN);assert.equal(job.head_sha,COMMIT);assert.equal(job.status,'completed');assert.equal(job.conclusion,'success');assert(job.steps.every(step=>step.conclusion!=='failure'));}
  assert.equal(receipt.result,'PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI46_INVOCATION');assert.equal(receipt.commit,COMMIT);assert.equal(receipt.tree,TREE);assert.equal(receipt.run,RUN);assert.equal(receipt.status,'completed');assert.equal(receipt.conclusion,'success');assert.equal(receipt.run_attempt,1);assert.equal(receipt.log_errors.length,0);
  assert.equal(receipt.raw.length,6);assert.deepEqual(receipt.raw.map(row=>row.path),[...PINS.keys()].slice(0,6));
  for(const item of receipt.raw){const bytes=raw.get(item.path);assert.equal(bytes.length,item.bytes);assert.equal(SHA(bytes),item.sha256);assert.equal(item.sha256,PINS.get(item.path));}
  const registry=raw.get('registered-ui-e2e.mjs'),workflow=raw.get('committed-ci.yml');
  assert.deepEqual(receipt.registered_suite_source,{commit:COMMIT,path:'scripts/ui-e2e.mjs',local_copy:'registered-ui-e2e.mjs',bytes:registry.length,sha256:SHA(registry)});
  assert.deepEqual(receipt.workflow_source,{commit:COMMIT,path:'.github/workflows/ci.yml',local_copy:'committed-ci.yml',bytes:workflow.length,sha256:SHA(workflow)});
  const expected=[...registry.toString('utf8').matchAll(/^\s+\['([a-z0-9-]+)',\s*'([^']+)',\s*'(fixture|live|owned-native)'/gm)].map(match=>({name:match[1],script:match[2],kind:match[3]}));
  assert.equal(expected.length,46);assert.equal(new Set(expected.map(row=>row.name)).size,46);assert.equal(expected.filter(row=>row.kind==='fixture').length,9);
  const log=raw.get('renderer.log').toString('utf8').replace(/\x1b\[[0-9;]*m/g,''),banners=[...log.matchAll(/^\d{4}-[^\r\n ]+ ##\[group\]Run node scripts\/ui-e2e\.mjs\r?$/gm)];
  assert.equal(banners.length,1,'One canonical unfiltered coordinator banner');
  const start=banners[0].index,after=log.slice(start),newline=after.indexOf('\n'),secondNewline=after.indexOf('\n',newline+1);
  assert.equal(after.slice(newline+1,secondNewline).replace(/^\d{4}-[^\r\n ]+ /,'').trim(),'node scripts/ui-e2e.mjs','Exact unfiltered command echo');
  const records=value=>[...value.matchAll(/^\d{4}-[^\r\n ]+ PASS ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].map(match=>({name:match[1],kind:match[2]}));
  const observed=records(log),nextStep=log.indexOf('##[group]Run ',start+banners[0][0].length);
  assert.equal(observed.length,46);assert.equal(new Set(observed.map(row=>row.name)).size,46);assert.deepEqual(observed,expected.map(({name,kind})=>({name,kind})));assert.deepEqual(records(log.slice(start,nextStep<0?undefined:nextStep)),observed,'All suite records belong to one invocation');
  assert.equal([...log.matchAll(/^\d{4}-[^\r\n ]+ FAIL ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].length,0);
  assert(observed.some(row=>row.name==='table-xlsx'&&row.kind==='owned-native'));
  const renderer=jobs.find(job=>job.id===RENDERER),steps=renderer.steps.filter(step=>step.name==='Redesigned UI fixture and native end-to-end workflows');assert.equal(steps.length,1);assert.equal(steps[0].status,'completed');assert.equal(steps[0].conclusion,'success');
  assert.deepEqual(receipt.renderer.UI_step,steps[0]);assert.deepEqual(receipt.renderer.suites_in_committed_order,expected);assert.deepEqual(receipt.renderer.observed,observed);assert.equal(receipt.renderer.accepted,true);assert.equal(receipt.renderer.canonical_unfiltered_invocations,1);assert.equal(receipt.renderer.passed_suites,46);assert.equal(receipt.renderer.unique_passed_suites,46);assert.equal(receipt.renderer.fixture_suites,9);assert.equal(receipt.renderer.native_suites,37);
  const scope='Exact published main commit22fc522 only. All four public CI jobs and one unfiltered46-suite invocation passed. This closes the previously pending public CI/full registered46 gate for that commit; it does not claim a fresh full local46 run. The earlier selected605-input local manifest is not a transitive dependency/toolchain or whole metadata closure. Earlier focused R1/R2 failures remain FAIL and their original evidence is unchanged. Duplicate branch39599316540 was cancelled separately and is not a main failure. Subsequent raw Boolean/source-availability/provenance changes are outside22fc522. No all-table/operator, selected-recrawl/segment/specialist/cursor/bulk XLSX acceptance, arbitrary corruption/crash/resource behavior, large-corpus streaming, Excel GUI interoperability, installed upgrade/full workflow completion or manufacturer parity is inferred.';
  const summary={schema_version:1,kind:'exact_published_main_table_xlsx_public_ci_supplement',result:'PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI46_INVOCATION',commit:COMMIT,tree:TREE,run:RUN,url:run.html_url,attempt:1,head_branch:'main',status:'completed',conclusion:'success',collected_at:receipt.collected_at,jobs:jobs.map(({id,name,status,conclusion,started_at,completed_at})=>({id,name,status,conclusion,started_at,completed_at})),renderer:{canonical_unfiltered_invocations:1,unique_passed_suites:46,fixture_suites:9,native_suites:37,suites_in_committed_order:expected,UI_step:steps[0]},raw_receipt:'raw/'+SOURCE+'/receipt.json',scope};
  putJSON('public-ci-result.json',summary);
  put('README.md',`# Main-table XLSX: exact published public CI, 2026-10-21\n\nThe exact published main commit \`${COMMIT}\` (tree \`${TREE}\`) passed all four jobs in [run ${RUN}](${run.html_url}): Windows, Ubuntu, Renderer and container. This supplement preserves their complete raw logs plus the original run/jobs API responses.\n\nThe raw Renderer log proves **46 distinct suite PASS records in one unfiltered coordinator invocation**: 9 fixture suites and 37 native suites, including \`table-xlsx\`. Their names, kinds and order match the exact committed [registry](raw/${SOURCE}/registered-ui-e2e.mjs). The exact committed workflow is also preserved. [Public result](public-ci-result.json), [original receipt](raw/${SOURCE}/receipt.json) and [copy index](index.json) bind the conclusions and all selected bytes. The collection helper and this materializer are archived, unchanged; archival paths are not a promise that the helpers run from their relocated copies.\n\nThis closes the earlier pending full registered 46-suite/public-CI check for this published XLSX revision. The earlier local focused R1/R2 failures remain unchanged and separately scoped; their later focused R3 PASS is not relabelled as a full local 46-suite run. The selected 605-input local manifest remains an explicit input set rather than a transitive toolchain/dependency or whole metadata closure. Duplicate branch run 39599316540 was cancelled separately; this supplement concerns the successful main run only.\n\nSubsequent Boolean fidelity, source-availability and provenance changes are outside this commit and require their own proof. No all-table/operator, segment/selected-recrawl/specialist/cursor/bulk XLSX acceptance, large-corpus streaming, Excel GUI interoperability, installed application upgrade, full workflow completion or manufacturer parity is inferred. Existing feature/filter/workflow totals and status counts are unchanged. No parent evidence, tracked document or feature status is rewritten by this helper.\n`);
  const index={schema_version:1,kind:'byte_exact_public_ci46_supplement_index',commit:COMMIT,tree:TREE,run:RUN,raw_files:copies,generated_files:[...outputs].filter(([name])=>!name.startsWith('raw/')).map(([name,bytes])=>({path:name,bytes:bytes.length,sha256:SHA(bytes)})),excluded:['public CI artifact ZIPs and their contents','job runtime workspaces/toolchains/dependencies','binary/store/browser runtime/user data','later successor local or public results'],scope:'Every raw_files entry is copied byte-for-byte from its finite explicit source. Generated files are derived summaries, not raw output. index.json excludes its own hash; the reviewed output plan binds index.json too.'};
  putJSON('index.json',index);
  const plan=[...outputs].map(([name,bytes])=>({path:DEST+'/'+name,bytes:bytes.length,sha256:SHA(bytes)})).sort((a,b)=>a.path.localeCompare(b.path));
  const planSHA=SHA(Buffer.from(JSON.stringify(plan))),result={result:apply?'MATERIALIZED_PUBLIC_CI46_SUPPLEMENT':'READY_VALIDATED_NO_WRITES',destination:DEST,commit:COMMIT,run:RUN,raw_files:copies.length,generated_files:3,output_files:outputs.size,total_bytes:plan.reduce((sum,row)=>sum+row.bytes,0),plan_sha256:planSHA,materializer_sha256:inputs.get(SELF).sha256,index_sha256:SHA(outputs.get('index.json'))};
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
