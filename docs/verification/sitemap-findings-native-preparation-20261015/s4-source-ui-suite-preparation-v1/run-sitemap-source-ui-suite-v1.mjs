import assert from 'node:assert/strict';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {fileURLToPath,pathToFileURL} from 'node:url';

const task = "C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https";
const runtimeRoot=path.join(task,'outputs/open-seo-spider');
const repo = "E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-source-processing-ui-v1";
const root='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed','New large runs remain held until the coordinated execution window is released');
assert.equal(path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),path.resolve(task),'Promote this guarded driver into the pinned task/work directory before execution');
for (const name of ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER']) { assert(!process.env[name], 'Unexpected externally configured compiler setting: ' + name); }
const source=path.join(root,'sitemap-source-processing-ui-source-v1');
const sourceRaw=fs.readFileSync(path.join(source,'manifest.json'));
const manifest=JSON.parse(sourceRaw);
assert.equal(createHash('sha256').update(sourceRaw).digest('hex'),'e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5');
assert.equal(manifest.files.length,297);
assert.equal(manifest.required_test_names,388);
assert.equal(manifest.minimum_expected_workspace_tests,1170);
assert.equal(manifest.new_authored_test_functions,0);
assert.equal(manifest.approved_product_bindings.length,2);
const ownedRendererGates=["live_browser_worker_executes_javascript_for_core_crawl","live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness","live_browser_worker_pause_drains_render_and_retains_rendered_discoveries","live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion"];
assert.equal(manifest.required_tests.reduce((n,x)=>n+x.names.length,0),388);
const out=path.join(root,'results/sitemap-source-ui-suite-v1');
const sha=b=>createHash('sha256').update(b).digest('hex');
// Fixed historical publication lineage; later metadata commits may change HEAD.
// Actual current compiler bytes in repo/source, not live Git metadata, admit IO.
function verifyUiLineage() {
  const baseSha = '69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751';
  const uiSha = 'e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5';
  const baseCommit = 'f43efc17d1393645a9eb8c8b8510743f3dfedb5c';
  const uiCommit = 'f4ed4b9bb7c760755ec7e448aac660cfb6063ade';
  const basePublicationSha = 'b71188d145a8e720bd9d6c9aaf4720c03c20cced7f406839264e5432e85e84ff';
  const uiPublicationSha = '4316704b11e47d7f49bb541e9b1db9b1c31ac9c53d5b68dd2c47ab339b4bc24b';
  const baseDirectory = path.join(root, 'sitemap-findings-surfaces-source-v1');
  const baseRaw = fs.readFileSync(path.join(baseDirectory, 'manifest.json'));
  assert.equal(sha(baseRaw), baseSha, 'Genuine base694 manifest required');
  assert.equal(sha(fs.readFileSync(path.join(source, 'manifest.json'))), uiSha, 'Current UI frozen manifest');
  const base = JSON.parse(baseRaw);
  const basePublicationRaw = fs.readFileSync(path.join(task, 'work/sitemap-findings-surfaces-published-v1.json'));
  const uiPublicationRaw = fs.readFileSync(path.join(task, 'work/sitemap-source-processing-ui-published-v1.json'));
  assert.equal(sha(basePublicationRaw), basePublicationSha, 'Fixed base publication receipt');
  assert.equal(sha(uiPublicationRaw), uiPublicationSha, 'Fixed UI publication receipt');
  const basePublication = JSON.parse(basePublicationRaw), uiPublication = JSON.parse(uiPublicationRaw);
  assert.equal(basePublication.commit, baseCommit); assert.equal(basePublication.source_manifest_sha256, baseSha); assert.equal(basePublication.repository.isPrivate, true);
  assert.equal(uiPublication.commit, uiCommit); assert.equal(uiPublication.source_manifest_sha256, uiSha); assert.equal(uiPublication.repository.isPrivate, true);
  assert.equal(uiPublication.base_source_commit, baseCommit); assert.equal(uiPublication.base_source_manifest_sha256, baseSha);
  assert.equal(uiPublication.source_inputs, 297); assert.equal(uiPublication.unchanged_base_inputs, 295); assert.equal(uiPublication.changed_ui_inputs, 2);
  assert.equal(uiPublication.new_authored_rust_tests, 0); assert.equal(uiPublication.inherited_source_authored_test_expectations, 376);
  assert.equal(uiPublication.required_test_names, 388); assert.equal(uiPublication.minimum_expected_workspace_tests, 1170);
  assert.equal(uiPublication.source_preparation_proof_sha256, 'b71c0b855945f41958a68bf168f7a2a00fd51cb15d17b5c7157045fcefe2f458');
  assert.equal(base.files.length, 297); assert.equal(base.new_authored_test_functions, 47); assert.equal(base.approved_product_bindings.length, 38);
  assert.equal(base.required_test_names, 388); assert.equal(base.minimum_expected_workspace_tests, 1170);
  assert.equal(manifest.base_source_commit, baseCommit); assert.equal(manifest.base_source_manifest_sha256, baseSha);
  assert.equal(manifest.base_inputs, 297); assert.equal(manifest.source_inputs, 297); assert.equal(manifest.unchanged_inputs, 295);
  assert.equal(manifest.new_authored_test_functions, 0); assert.equal(manifest.inherited_source_authored_test_expectations, 376);
  assert.equal(path.resolve(manifest.source_worktree), path.resolve(repo));
  assert.deepEqual(manifest.required_tests, base.required_tests, 'All inherited grouped names/order remain exact');
  assert.equal(manifest.review.sha256, '3e67b838a3f90a0381b274b85d9fd51cc41fcc435b71f15377b5e53d40fbbb5f');
  assert.equal(sha(fs.readFileSync(path.join(root, 's4-source-processing-ui-review-v1/receipt.json'))), manifest.review.sha256);
  const expectedUI = [
    {path: 'web/app.js', before_sha256: '84df9b724802d86d69262d98dcee10e22d465c7a6583fc034b438df452bb0bb9', after_sha256: 'b1ec57080fec11529e89b21ce07471ea3ddc2141d4f64ae79ffc236c9b5af3f0', bytes: 379587},
    {path: 'web/index.html', before_sha256: 'f5f494781f91029f1b64ace3c0f1ddcba742014690e0d4ee0dbbef53ebf79cff', after_sha256: 'ed8b3d0a624d02b428a3a7430845b417eca73a34bd3ecd6b0d5899d46bd15e77', bytes: 82970},
  ];
  assert.deepEqual(manifest.approved_product_bindings, expectedUI, 'Only the exact two reviewed UI payloads');
  const baseFiles = new Map(base.files.map(file => [file.path, file]));
  const uiFiles = new Map(manifest.files.map(file => [file.path, file]));
  assert.equal(baseFiles.size, 297); assert.equal(uiFiles.size, 297);
  assert.deepEqual([...baseFiles.keys()].sort(), [...uiFiles.keys()].sort(), 'Identical297 input path set');
  let unchanged = 0, changed = 0;
  const verifyFile = (directory, file) => {
    const current = fs.readFileSync(path.join(directory, file.path));
    assert.equal(current.length, file.bytes, file.path + ' exact bytes');
    assert.equal(sha(current), file.sha256, file.path + ' exact SHA');
  };
  for (const [filePath, previous] of baseFiles) {
    const current = uiFiles.get(filePath), binding = expectedUI.find(item => item.path === filePath);
    if (binding) {
      assert.equal(previous.sha256, binding.before_sha256); assert.equal(current.sha256, binding.after_sha256); assert.equal(current.bytes, binding.bytes);
      assert.notEqual(current.sha256, previous.sha256); changed++;
    } else { assert.deepEqual(current, previous, filePath + ' inherited input unchanged'); unchanged++; }
    verifyFile(baseDirectory, previous); verifyFile(source, current); verifyFile(repo, current);
  }
  assert.equal(unchanged, 295); assert.equal(changed, 2);
  return {base_source_manifest_sha256: baseSha, base_source_commit: baseCommit, base_publication_receipt_sha256: basePublicationSha, ui_source_manifest_sha256: uiSha, ui_source_commit: uiCommit, ui_publication_receipt_sha256: uiPublicationSha, base_inputs: 297, unchanged_inputs: unchanged, changed_ui_inputs: changed, inherited_base_new_rust_tests: 47, inherited_base_approved_bindings: 38, new_ui_rust_tests: 0, approved_ui_bindings: 2, inherited_source_authored_expectations: 376, inherited_required_names: 388, minimum_expected_workspace_tests: 1170};
}
const sourceUILineage = verifyUiLineage();
assert(!fs.existsSync(out));fs.mkdirSync(out);
const temp=path.join(out,'temp');fs.mkdirSync(temp);
const env={...process.env,CARGO_TARGET_DIR:path.join(root,'cargo-target'),CARGO_INCREMENTAL:'0',CARGO_PROFILE_DEV_DEBUG:'0',CARGO_PROFILE_TEST_DEBUG:'0',CARGO_TERM_COLOR:'never',OPEN_SEO_BROWSER_PATH:'C:/Program Files/Google/Chrome/Application/chrome.exe',TEMP:temp,TMP:temp};
const runs=[],failures=[];
let worker,fixture,workerAddress,fixtureAddress,closed=false,rendererGatesVerified=false;
function verify(){assert.deepEqual(verifyUiLineage(),sourceUILineage);for(const f of manifest.files){assert.equal(sha(fs.readFileSync(path.join(source,f.path))),f.sha256);assert.equal(sha(fs.readFileSync(path.join(repo,f.path))),f.sha256);}}
function verifyRuntime(){for(const f of manifest.files.filter(x=>x.path.startsWith('renderer/')))assert.equal(sha(fs.readFileSync(path.join(runtimeRoot,f.path))),f.sha256);}
function write(name,value){fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2)+'\n',{flag:'wx'});}
async function run(name,args){
  const log=fs.openSync(path.join(out,name+'.log'),'wx');
  const started=Date.now();
  let child,spawnError=null,exit;
  try{
    child=spawn('C:/Users/matth/.cargo/bin/cargo.exe',args,{cwd:source,env,windowsHide:true,stdio:['ignore',log,log]});
    child.once('error',error=>{spawnError=error.message;});
    exit=await new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));
  }finally{fs.closeSync(log);}
  const bytes=fs.readFileSync(path.join(out,name+'.log'));const text=bytes.toString('utf8');
  const summaries=[...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(m=>({state:m[1],passed:+m[2],failed:+m[3],ignored:+m[4]}));
  const record={name,args,pid:child.pid,actual_close_event:true,exit,spawn_error:spawnError,elapsed_ms:Date.now()-started,log_sha256:sha(bytes),summaries};
  runs.push(record);write(name+'.json',record);
  console.log(JSON.stringify({name,exit,passed:summaries.reduce((n,x)=>n+x.passed,0),ignored:summaries.reduce((n,x)=>n+x.ignored,0)}));
  assert.equal(spawnError,null,spawnError??"spawn");
  assert.equal(exit.code,0,text.slice(-3500));
  assert.equal(exit.signal,null);
}
try{
  verify();verifyRuntime();
  const {createRendererServer}=await import(pathToFileURL(path.join(runtimeRoot,'renderer/server.mjs')).href);
  worker=createRendererServer({env,concurrency:4});workerAddress=await worker.listen(0);
  assert(![8787,8790].includes(workerAddress.port));
  fixture=http.createServer((req,res)=>{res.setHeader('content-type','text/html');res.end('<html><head><title>Warmup</title></head><body><script>document.title="Owned ready";</script></body></html>');});
  await new Promise(resolve=>fixture.listen(0,'127.0.0.1',resolve));fixtureAddress=fixture.address();
  assert(![8787,8790].includes(fixtureAddress.port));
  const origin='http://127.0.0.1:'+workerAddress.port;
  const health=await(await fetch(origin+'/health',{signal:AbortSignal.timeout(10000)})).json();assert.equal(health.browser.available,true);
  const response=await fetch(origin+'/render',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({url:'http://127.0.0.1:'+fixtureAddress.port+'/',wait_ms:20}),signal:AbortSignal.timeout(35000)});
  assert.equal(response.status,200);
  write('owned-worker.json',{origin,fixture_origin:'http://127.0.0.1:'+fixtureAddress.port,health,warmup:await response.json(),shared_worker_contacted:false});
  env.OPEN_SEO_SPIDER_RENDERER=origin+'/render';
  await run('format',['fmt','--all','--check']);
  await run('workspace-live',['test','--workspace','--locked','--','--include-ignored','--test-threads=1']);
  const workspaceText=fs.readFileSync(path.join(out,'workspace-live.log'),'utf8');
  for(const name of ownedRendererGates)assert(workspaceText.includes('test '+name+' ... ok'),'Required actual owned browser gate not green: '+name);
  rendererGatesVerified=true;
  for(const group of manifest.required_tests)for(const name of group.names)assert(workspaceText.includes('::'+name+' ... ok'),'Required actual test not green: '+group.path+'::'+name);
  await run('clippy',['clippy','--workspace','--all-targets','--locked','--','-D','warnings']);
  verify();verifyRuntime();
}catch(error){failures.push({message:error.message,stack:error.stack});process.exitCode=1;console.error(error.message.slice(-3500));}
finally{
  try{if(worker)await worker.close();if(fixture)await new Promise(resolve=>fixture.close(resolve));closed=(!worker||!worker.server.listening)&&(!fixture||!fixture.listening);}catch(error){failures.push({phase:'owned close',message:error.message});process.exitCode=1;}
  const tests=runs.find(x=>x.name==='workspace-live');const passed=tests?.summaries.reduce((n,x)=>n+x.passed,0)??0;const ignored=tests?.summaries.reduce((n,x)=>n+x.ignored,0)??0;
  const result=failures.length===0&&closed&&rendererGatesVerified&&passed>=1170&&ignored===0?'PASS_LOCAL_CURRENT_WORKSPACE_REAL_RENDERER_CLIPPY_FMT':'FAIL_LOCAL_CURRENT_WORKSPACE';
  const proof={schema_version:1,result,recorded_at:new Date().toISOString(),source_manifest_sha256:sha(sourceRaw),files:manifest.files.length,source_authored_delta:0,source_authored_total:376,inherited_source_authored_delta:47,inherited_approved_product_bindings:38,source_ui_bindings:2,source_lineage:sourceUILineage,accepted_baseline:794,passed,ignored,minimum_expected:1170,required_test_names:manifest.required_tests.reduce((n,x)=>n+x.names.length,0),runs,owned_worker_port:workerAddress?.port,owned_worker_closed:closed,owned_renderer_gates:ownedRendererGates,owned_renderer_gate_count:ownedRendererGates.length,owned_renderer_gates_verified:rendererGatesVerified,user_renderer_contacted:false,failures,qualification:"Actual Windows full workspace over297 UI-profile frozen inputs with388 inherited named source cases and4 real owned renderer gates; all4 are already included in minimum1170. Genuine base694 lineage retains295 identical inputs and admits only2 exact reviewed UI changes. Base47 newly authored Rust functions and38 product bindings are historical; this layer adds0 Rust tests and2 UI bindings. Cumulative source-authored376 expectations over accepted baseline794 are inherited, not new UI acceptance. Only this actual full workspace/Clippy/fmt run is covered. Browser form save/reload/disabled absence/enabled native profile/independent page budget, actual larger-source UI fetch,13 UI mocks, SDK/native/physical/source matrix, RSS/latency/recovery/selected/platform/install remain separate OPEN gates. Old694 acceptance drivers are not automatically rebound to these UI bytes."};
  write('proof.json',proof);console.log(JSON.stringify({result,passed,ignored,closed,out}));
  if(result.startsWith('FAIL'))process.exitCode=1;
}
