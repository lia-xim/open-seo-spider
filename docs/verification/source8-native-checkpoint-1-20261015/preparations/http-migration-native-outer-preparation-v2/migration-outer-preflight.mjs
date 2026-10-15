// Future Root-only admission for exactly one migration harness. No generic dispatch.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
const BASE='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const TASK='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const PREP=path.join(BASE,'http-migration-native-outer-preparation-v2');
const HARNESS_PREP=path.join(BASE,'http-migration-native-preparation-v3');
const HARNESS_MANIFEST='52f828b753b2316b34cd9f0f5e3ca79dc246028e730e2529e27af95ff6210a95';
const HARNESS='c0af21f5fc5f66b3d82b3902f64a41ced69a56b7cf31f14dd08f5e333690f647';
const HARNESS_REVIEW='3ffbd29e36d3d68eed2ed4e78e9e660f2a53f75f19880f0241602c981f67b79a';
const BINDER_PREP=path.join(BASE,'combined-v8-runner-preparation-v4');
const BINDER_MANIFEST='d4adece778ec4a2b3ef713be7319840457bbf5119b48809fe07493af65ca1c80';
const BINDER='c58f1984c6b6593d384915157afdcdae206acd1bb8af72279a9ee82702e1b7e7';
const BINDER_REVIEW='3b6240aca4bec56afb058d83b8552ef1c36b28ba465169a31052c5df0e28ae78';
const BUILDER='e48e2cadc361a100eecbbed5465bf4436d28f7fa19068682550932f320e1c547';
const SUITE='921a8023a3354c42ef28583888734bbe6c4784794161ec5e5f5c8f53dd39f62b';
const GATE='migration-workflow-v8';
const sha=b=>createHash('sha256').update(b).digest('hex');
const normalized=p=>path.resolve(p).toLowerCase();
const samePath=(a,b)=>assert.equal(normalized(a),normalized(b));
function hashFile(file,max=256*1024*1024){
 const stat=fs.lstatSync(file);assert(stat.isFile()&&!stat.isSymbolicLink()&&stat.size<=max,file);
 const fd=fs.openSync(file,'r'),hash=createHash('sha256'),chunk=Buffer.alloc(65536);
 try{let n;while((n=fs.readSync(fd,chunk,0,chunk.length,null))!==0)hash.update(chunk.subarray(0,n));}
 finally{fs.closeSync(fd);}return hash.digest('hex');
}
export async function preflight({grant,grantRow,preparationPath,preparationSha,actual}){
 assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed');
 assert.equal(process.platform,'win32');assert.match(process.versions.node,/^24\./);
 assert(!process.env.NODE_OPTIONS&&!process.env.NODE_PATH);assert.equal(process.execArgv.length,0);
 const refs=new Map();
 function read(file,expected,max=8*1024*1024){
  assert(path.isAbsolute(file));assert.match(expected,/^[a-f0-9]{64}$/);
  const stat=fs.lstatSync(file);assert(stat.isFile()&&!stat.isSymbolicLink()&&stat.size<=max,file);
  const bytes=fs.readFileSync(file);assert.equal(bytes.length,stat.size);assert.equal(sha(bytes),expected,file);
  const key=normalized(file),old=refs.get(key);if(old)assert.equal(old.sha256,expected);else refs.set(key,{path:file,sha256:expected,bytes:bytes.length});
  return bytes;
 }
 const data=(file,hash)=>JSON.parse(read(file,hash));
 function payloads(directory,rows){
  assert(Array.isArray(rows)&&rows.length>0);const seen=new Set();
  for(const row of rows){
   assert(typeof row.path==='string'&&row.path&&!row.path.includes('\\')&&!row.path.includes(':')&&!path.isAbsolute(row.path));
   assert(row.path.split('/').every(p=>p&&p!=='.'&&p!=='..'));assert(!seen.has(row.path));seen.add(row.path);
   assert.equal(read(path.join(directory,row.path),row.sha256).length,row.bytes);
  }
 }
 const keys=['source_manifest_sha256','suite_proof_sha256','build_proof_sha256','candidate_binary_sha256'];
 for(const key of keys){assert.match(actual[key],/^[a-f0-9]{64}$/);assert.equal(grant[key],actual[key]);}
 assert.match(actual.lock_token,/^[a-f0-9]{32,128}$/);assert.equal(grant.lock_token,actual.lock_token);
 const core=Object.freeze({...Object.fromEntries(keys.map(k=>[k,grant[k]])),build_driver_sha256:grant.build_driver_sha256});
 assert.equal(core.build_driver_sha256,BUILDER);const sourceSHA=core.source_manifest_sha256;
 assert.equal(grant.kind,'root_released_migration_workflow_native_v8');assert.equal(grant.status,'RELEASED');assert.equal(grant.issued_by,'Root');
 assert.equal(grant.scope,'genuine_saved_crawl_migration_api_cli_browser_cold_reopen');
 assert.equal(grant.source8_admission_status,'ACTUAL_REVIEWED_BINDER_AND_PROOFS_AVAILABLE');
 assert.equal(grant.execution_lock_mode,'outer_owns_shared_native_lock');assert.equal(grant.deadline_ms,600000);
 const start=Date.parse(grant.not_before),end=Date.parse(grant.expires_at);
 assert(Number.isFinite(start)&&Number.isFinite(end)&&start<=Date.now()&&Date.now()<end&&end-start<=7200000);
 samePath(grantRow.path,path.join(TASK,'work/v8-migration-native-release.json'));read(grantRow.path,grantRow.sha256);
 samePath(preparationPath,path.join(PREP,'manifest.json'));const preparation=data(preparationPath,preparationSha);
 assert.equal(preparation.kind,'http_migration_native_outer_preparation_v2');assert.equal(preparation.status,'AUTHORED_NOT_RUN');payloads(PREP,preparation.files);
 const outerRow=preparation.files.find(f=>f.path==='run-migration-native-outer-v8.mjs');assert(outerRow);read(path.join(TASK,'work',outerRow.path),outerRow.sha256);
 samePath(grant.migration_outer_preparation.path,preparationPath);assert.equal(grant.migration_outer_preparation.sha256,preparationSha);assert.equal(grant.outer_driver_sha256,outerRow.sha256);
 const outerReview=data(grant.outer_independent_review.path,grant.outer_independent_review.sha256);
 assert.equal(outerReview.kind,'http_migration_native_outer_independent_review');assert.equal(outerReview.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(outerReview.findings,[]);
 samePath(outerReview.proposal_manifest.path,preparationPath);assert.equal(outerReview.proposal_manifest.sha256,preparationSha);
 samePath(outerReview.outer.path,path.join(PREP,outerRow.path));assert.equal(outerReview.outer.sha256,outerRow.sha256);
 const harnessManifestPath=path.join(HARNESS_PREP,'manifest.json'),harnessManifest=data(harnessManifestPath,HARNESS_MANIFEST);
 assert.equal(harnessManifest.kind,'http_migration_native_preparation_v3');assert.equal(harnessManifest.mandatory_named_runtime_groups,8);payloads(HARNESS_PREP,harnessManifest.payloads);
 const harnessPath=path.join(HARNESS_PREP,'native-migration-workflow.mjs');read(harnessPath,HARNESS);
 assert.equal(grant.preparation_manifest_sha256,HARNESS_MANIFEST);assert.equal(grant.harness_sha256,HARNESS);
 const helper=harnessManifest.payloads.find(f=>f.path==='migration-browser-flow.mjs');assert(helper);assert.equal(helper.sha256,'85586c0849d6c384a4c0bb4579356d06960d95854a070708868bf8784dca1232');
 samePath(grant.independent_review.path,path.join(BASE,'http-migration-native-independent-review-v3/receipt.json'));assert.equal(grant.independent_review.sha256,HARNESS_REVIEW);
 const harnessReview=data(grant.independent_review.path,HARNESS_REVIEW);assert.equal(harnessReview.kind,'http_migration_native_independent_review');assert.equal(harnessReview.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(harnessReview.findings,[]);
 samePath(harnessReview.proposal_manifest.path,harnessManifestPath);assert.equal(harnessReview.proposal_manifest.sha256,HARNESS_MANIFEST);samePath(harnessReview.harness.path,harnessPath);assert.equal(harnessReview.harness.sha256,HARNESS);
 const binderPath=path.join(BINDER_PREP,'combined-bindings-v8.mjs'),binderManifestPath=path.join(BINDER_PREP,'manifest.json'),binderManifest=data(binderManifestPath,BINDER_MANIFEST);payloads(BINDER_PREP,binderManifest.files);
 samePath(binderManifest.binder.path,binderPath);assert.equal(binderManifest.binder.sha256,BINDER);read(binderPath,BINDER);
 const binderReviewPath=path.join(BASE,'combined-v8-runner-independent-review-v4/receipt.json'),binderReview=data(binderReviewPath,BINDER_REVIEW);
 assert.equal(binderReview.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(binderReview.findings,[]);
 samePath(binderReview.proposal_manifest.path,binderManifestPath);assert.equal(binderReview.proposal_manifest.sha256,BINDER_MANIFEST);samePath(binderReview.binder.path,binderPath);assert.equal(binderReview.binder.sha256,BINDER);
 samePath(grant.binder.path,binderPath);assert.equal(grant.binder.sha256,BINDER);samePath(grant.binder.manifest.path,binderManifestPath);assert.equal(grant.binder.manifest.sha256,BINDER_MANIFEST);
 samePath(grant.binder.independent_review.path,binderReviewPath);assert.equal(grant.binder.independent_review.sha256,BINDER_REVIEW);
 const sourcePath=path.join(BASE,'combined-source-v8/manifest.json'),suitePath=path.join(BASE,'results/combined-suite-v8/proof.json'),buildPath=path.join(BASE,'results/combined-optimized-build-v8/proof.json');
 read(sourcePath,sourceSHA);read(suitePath,core.suite_proof_sha256);read(buildPath,core.build_proof_sha256);
 read(path.join(TASK,'work/run-combined-suite-v8.mjs'),SUITE);read(path.join(TASK,'work/build-combined-optimized-v8.mjs'),BUILDER);
 const {bindCombined}=await import(pathToFileURL(binderPath).href);read(binderPath,BINDER);const binding=bindCombined(core,sourceSHA);
 assert.equal(binding.manifest.kind,'combined_source_v8');assert.equal(binding.manifest.source_inputs,335);assert.equal(binding.manifest.required_test_names,540);assert.equal(binding.manifest.minimum_expected_workspace_tests,1322);
 assert.equal(binding.suite.kind,'combined_full_suite_v8');assert.equal(binding.suite.result,'PASS_COMBINED_WORKSPACE_V8_REAL_RENDERER_FMT_CLIPPY');assert(binding.suite.passed>=1322);assert.equal(binding.suite.failed,0);assert.equal(binding.suite.ignored,0);
 assert.equal(binding.build.kind,'combined_optimized_build_v8');assert.equal(binding.build.result,'PASS_COMBINED_OPTIMIZED_BUILD_V8');assert.equal(typeof binding.binary,'string');assert.equal(binding.binarySHA,core.candidate_binary_sha256);
 assert.equal(sha(binding.sourceRaw),sourceSHA);assert.equal(sha(binding.suiteRaw),core.suite_proof_sha256);assert.equal(sha(binding.buildRaw),core.build_proof_sha256);samePath(binding.suitePath,suitePath);samePath(path.join(binding.buildRoot,'proof.json'),buildPath);binding.verify();
 assert.match(grant.node_sha256,/^[a-f0-9]{64}$/);assert.equal(hashFile(process.execPath),grant.node_sha256);
 samePath(grant.chrome.path,'C:/Program Files/Google/Chrome/Application/chrome.exe');assert.match(grant.chrome.sha256,/^[a-f0-9]{64}$/);assert.equal(hashFile(grant.chrome.path),grant.chrome.sha256);
 samePath(grant.playwright_root,path.join(TASK,'outputs/open-seo-spider/renderer/node_modules/playwright-core'));assert.match(grant.playwright_package_sha256,/^[a-f0-9]{64}$/);read(path.join(grant.playwright_root,'package.json'),grant.playwright_package_sha256);
 assert.equal(typeof grant.output,'string');samePath(grant.output,path.join(BASE,'results/migration-workflow-native-v8'));assert(!fs.existsSync(grant.output),'Fresh migration inner output required');
 const outerOutput=path.join(BASE,'results/combined-native-outer-v4',GATE),lockPath=path.join(BASE,'results/combined-native-outer-v4/active.lock');
 assert(!fs.existsSync(outerOutput),'Fresh migration outer output required');assert(!fs.existsSync(lockPath),'Never steal the shared native lock');
 function verify(){for(const r of refs.values())read(r.path,r.sha256);assert.equal(hashFile(process.execPath),grant.node_sha256);assert.equal(hashFile(grant.chrome.path),grant.chrome.sha256);binding.verify();}
 verify();return{core,sourceSHA,binding,sourcePath,suitePath,buildPath,binderPath,harnessPath,harnessSha:HARNESS,harnessManifestPath,innerOutput:path.resolve(grant.output),outerOutput,lockPath,references:[...refs.values()],verify};
}
