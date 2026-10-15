import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';

// Five direct reviewed CSS/exact-length deltas over immutable actual Parent10.
// This module reads evidence only; it does not import a former validator, run
// Git, copy product files, create processes, or accept a former PASS as current.
const BASE='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const TASK='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
export const PARENT_SHA='12b238c43bd504b71e4948a8e11d4622f34589a35290ca8ef923b833464206c7';
const PARENT_SUITE_SHA='3bb753190aaf64aea45367c3afa9507772fc765d19e31bbe93d7cba9c63440f5';
const PARENT_BUILD_SHA='abfb7db3576ae51239fadc43c213a4027c698280ed5e6fd733806bebbb6c7c52';
const PARENT_BINARY_SHA='a36757ef457b1c740ba4982d17ef78071799bc27bb59a859375d539633d2e76c';
const PARENT_BINARY_BYTES=43135488;
const CSS={path:'web/styles.css',before_sha256:'72cb03daecb5754d295e801be78c328a37f5e2d856aabde62f606975035859f5',before_bytes:42268,after_sha256:'fc69c805d5d6c71cef31047c3d9139c7b88bed0e7b3f6b61c324d0421be8aba3',bytes:42399};
const CSS_PROPOSAL={path:BASE+'/settings-hint-wrap-proposal-v2/manifest.json',sha256:'d29f3dfabd3a1fef20f9b877d2358ccf97d624f2360f9ebcc88e1a9a88a27b97'};
const CSS_REVIEW={path:BASE+'/settings-hint-wrap-independent-review-v2/receipt.json',sha256:'dedb0e420992f60d506252f9678355950c75605deada834250704745eca785a8'};
const CSS_DIAGNOSTIC={path:BASE+'/results/source10-invalid-settings-diagnostic-v1/proof.json',sha256:'1a289c0bb74ead3747702a86530a3b2f95157b003784150e5d95fc2ddd9557d0'};
const CSS_PREVIEW={path:BASE+'/results/source10-invalid-settings-diagnostic-v2/proof.json',sha256:'595740ed6c24ec026ddaee78b525183b0d7e2e6c8890a863665592dc3cecab55'};
const COMPOSITION_REVIEW={"path":"E:/CodexWork/open-seo-spider-01a12ac3-20261013/source11-composition-independent-review-v1/receipt.json","sha256":"54775a6d6b8d29cb87b76ce1ae0af417148ac72fe15e0b84b57275082fe6243b","bytes":70983};
const RESPONSE_PROPOSAL={"path":"E:/CodexWork/open-seo-spider-01a12ac3-20261013/http-response-exact-length-proposal-v3/manifest.json","sha256":"9f8048eadf8ff548892b2d634f6245280fee718e88d25317ba8ac7527d504d27","bytes":4781};
const RESPONSE_REVIEW={"path":"E:/CodexWork/open-seo-spider-01a12ac3-20261013/http-response-exact-length-independent-review-v3/receipt.json","sha256":"0757991e2ec4661566267d52e8142147a90cdf14f862e276f241f496bb5ffa86","bytes":3094};
const DIRECTED_DRIVER={"path":"C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https/work/run-source11-response-stream-tests-v1.mjs","sha256":"8b37b5a4e912b41cb09c1979b8345cf3c3e370b60c9a072dda276943407d87b7","bytes":5213};
const RESPONSE_TEST_PATH='crates/app/src/server/response_stream_tests.rs';
const RESPONSE_NAMES=[
'response_exact_length_commits_joined_final_frame_without_a_following_eof_poll',
'response_exact_length_holds_only_final_frame_and_rejects_late_error_panic_and_cancel',
'response_exact_length_zero_waits_before_headers_and_short_extra_producers_never_deadlock',
'response_exact_length_real_hyper_http1_commits_nonzero_and_unpolled_zero_bodies'].sort();
const RESPONSE_FULL=RESPONSE_NAMES.map(n=>'server::response_stream::tests::'+n).sort();
const RESPONSE_PATHS=['crates/app/src/server/ngram_corpus.rs','crates/app/src/server/resource_broken_sources.rs','crates/app/src/server/response_stream.rs',RESPONSE_TEST_PATH].sort();
const OWNED=[...RESPONSE_PATHS,CSS.path].sort();
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const normalized=p=>path.resolve(p).toLowerCase();
const HASH=/^[a-f0-9]{64}$/;
const profile=Object.freeze({opt_level:1,debug_assertions:true,overflow_checks:true,debug_info:0,environment:{CARGO_PROFILE_TEST_OPT_LEVEL:'1',CARGO_PROFILE_TEST_DEBUG_ASSERTIONS:'true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS:'true',CARGO_PROFILE_TEST_DEBUG:'0',CARGO_PROFILE_DEV_DEBUG:'0'}});
const gates=['live_browser_worker_executes_javascript_for_core_crawl','live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness','live_browser_worker_pause_drains_render_and_retains_rendered_discoveries','live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion'];
export function relativeFile(value){assert.equal(typeof value,'string');assert(value&&!value.includes('\\')&&!value.includes(':')&&!path.posix.isAbsolute(value));assert(value.split('/').every(p=>p&&p!=='.'&&p!=='..'));return value;}
function rowsMap(rows,count){assert(Array.isArray(rows));assert.equal(rows.length,count);const map=new Map();for(const f of rows){relativeFile(f.path);assert.deepEqual(Object.keys(f).sort(),['bytes','path','sha256']);assert.match(f.sha256,HASH);assert(Number.isSafeInteger(f.bytes)&&f.bytes>=0);assert(!map.has(f.path));map.set(f.path,f);}assert.deepEqual([...map.keys()],[...map.keys()].sort());assert.equal(new Set([...map.keys()].map(p=>p.toLowerCase())).size,count);return map;}
function sameRef(a,b){assert.equal(normalized(a.path),normalized(b.path));assert.equal(a.sha256,b.sha256);}
function actualClose(v){assert.equal(v.actual_close_event,true);assert(Number.isSafeInteger(v.pid)&&v.pid>0);assert.deepEqual(v.exit,{code:0,signal:null});}
function actualPipes(v){for(const side of ['stdout','stderr']){assert.equal(v.streams[side].end,true);assert.equal(v.streams[side].close,true);assert(Number.isSafeInteger(v.streams[side].bytes)&&v.streams[side].bytes>=0);}assert.deepEqual(v.io_errors,[]);}
function summaries(raw){return [...raw.toString().matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(m=>({state:m[1],passed:+m[2],failed:+m[3],ignored:+m[4]}));}
function jsonLines(raw){return [...raw.toString().matchAll(/^test ([^\r\n]+?) \.\.\. (ok|FAILED|ignored[^\r\n]*)\r?$/gm)].map(m=>({name:m[1],state:m[2]}));}
function fileRow(bytes,p){return {path:p,sha256:sha(bytes),bytes:bytes.length};}
function applyOps(before,operations,p){let current=before;for(const op of operations){assert.equal(op.path,p);assert(Number.isSafeInteger(op.offset)&&op.offset>=0);assert.equal(sha(current),op.before_sha256);const old=Buffer.from(op.before_base64,'base64'),next=Buffer.from(op.after_base64,'base64');assert(current.subarray(op.offset,op.offset+old.length).equals(old));current=Buffer.concat([current.subarray(0,op.offset),next,current.subarray(op.offset+old.length)]);assert.equal(sha(current),op.after_sha256);}let inverse=current;for(const op of [...operations].reverse()){const old=Buffer.from(op.before_base64,'base64'),next=Buffer.from(op.after_base64,'base64');assert.equal(sha(inverse),op.after_sha256);assert(inverse.subarray(op.offset,op.offset+next.length).equals(next));inverse=Buffer.concat([inverse.subarray(0,op.offset),old,inverse.subarray(op.offset+next.length)]);assert.equal(sha(inverse),op.before_sha256);}assert(inverse.equals(before));return current;}

function verifyParentSuite(parent,pin){
 const task=TASK,source=path.join(BASE,'combined-source-v10'),repo=path.join(BASE,'mcp-file-tools-v1');
 const suiteRoot=path.join(BASE,'results/combined-suite-v10');
 const driverPath=path.join(BASE,'combined-v10-driver-preparation-v2/run-combined-suite-v10.mjs');
 const suiteDriverSha='399c294fd9ea6ac0b78b012f430de566b1d2ecabb531896c375a8abc7d0b7d61';
 pin(driverPath,suiteDriverSha);
 const parentProof=JSON.parse(pin(path.join(suiteRoot,'proof.json'),PARENT_SUITE_SHA));
 const resultFile=parentProof.required_test_names_verified;
 assert.equal(resultFile,true);
 const actualNamesPath=path.join(suiteRoot,'required-test-results.json');const actualNamesStat=fs.lstatSync(actualNamesPath);assert(actualNamesStat.isFile()&&!actualNamesStat.isSymbolicLink()&&actualNamesStat.size<=8*1024*1024);const actualNamesRaw=fs.readFileSync(actualNamesPath);assert.equal(actualNamesRaw.length,actualNamesStat.size);
 const expectedNewFullNames=JSON.parse(actualNamesRaw).results.map(r=>r.actual).sort();
 assert.equal(expectedNewFullNames.length,602);assert.equal(new Set(expectedNewFullNames).size,602);
 const expectedSuiteProofSha=PARENT_SUITE_SHA,expectedManifestSha=PARENT_SHA;
 const manifest=parent,lineage=parentProof.source_lineage,names=parent.required_tests.flatMap(g=>g.names),minimumExpected=1380;
 const expectedTestProfile=profile,rendererGates=gates;
 const boundedRead=(file,max)=>{const st=fs.lstatSync(file);assert(st.isFile()&&!st.isSymbolicLink()&&st.size<=max);const raw=fs.readFileSync(file);assert.equal(raw.length,st.size);pin(file,sha(raw));return raw;};
 const parseSummaries=text=>[...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(m=>({state:m[1],passed:+m[2],failed:+m[3],ignored:+m[4]}));
 const suiteCommands=[['format',['fmt','--all','--check']],['workspace-live',['test','--workspace','--locked','--','--include-ignored','--test-threads=1']],['clippy',['clippy','--workspace','--all-targets','--locked','--','-D','warnings']]];
function verifySuite() {
  assert.equal(sha(fs.readFileSync(driverPath)), suiteDriverSha, 'Exact actual suite producer required');
  const proofBytes = boundedRead(path.join(suiteRoot, 'proof.json'), 8 * 1024 * 1024);
  assert.equal(sha(proofBytes), expectedSuiteProofSha, 'Explicit actual suite proof hash required');
  const proof = JSON.parse(proofBytes);
  assert.equal(proof.kind, 'combined_full_suite_v10');
  assert.equal(proof.result, 'PASS_COMBINED_WORKSPACE_V10_REAL_RENDERER_FMT_CLIPPY');
  assert.equal(proof.driver_sha256, suiteDriverSha);
  assert.equal(proof.source_manifest_sha256, expectedManifestSha);
  assert.equal(path.resolve(proof.source_directory), path.resolve(source));
  assert.equal(path.resolve(proof.source_worktree), path.resolve(repo));
  assert.equal(proof.source_inputs, manifest.files.length);
  assert.deepEqual(proof.source_manifest_metadata, Object.fromEntries(Object.entries(manifest)
    .filter(([key]) => !['files', 'required_tests'].includes(key))));
  assert.deepEqual(proof.source_lineage, lineage);
  assert.deepEqual(proof.test_profile, expectedTestProfile);
  assert.equal(proof.required_test_names, names.length);
  assert.equal(proof.required_test_names_verified, true);
  assert.equal(proof.minimum_expected, minimumExpected);
  assert.equal(proof.owned_renderer_gates_verified, true);
  assert.deepEqual(proof.owned_renderer_gates, rendererGates);
  assert.equal(proof.owned_worker_closed, true);
  assert.equal(proof.owned_fixture_closed, true);
  assert.equal(proof.source_final_verified, true);
  assert.equal(proof.user_renderer_contacted, false);
  assert.deepEqual(proof.failures, []);
  for (const port of [proof.owned_worker_port, proof.owned_fixture_port]) {
    assert(Number.isInteger(port) && port >= 1 && port <= 65535 && ![8787, 8790].includes(port));
  }
  assert.equal(proof.runs.length, 3);
  const digests = [{path: 'proof.json', sha256: sha(proofBytes), bytes: proofBytes.length}];
  let workspaceText, workspaceSummary;
  for (const [index, [name, args]] of suiteCommands.entries()) {
    const run = proof.runs[index];
    assert.equal(run.name, name);
    assert.deepEqual(run.args, args);
    assert(Number.isInteger(run.pid) && run.pid > 0);
    assert.equal(run.actual_close_event, true);
    assert.equal(run.spawn_error, null);
    assert.deepEqual(run.exit, {code: 0, signal: null});
    const log = boundedRead(path.join(suiteRoot, name + '.log'), 64 * 1024 * 1024);
    assert.equal(sha(log), run.log_sha256, 'Raw suite log SHA mismatch: ' + name);
    assert.equal(log.length, run.log_bytes);
    const summaries = parseSummaries(log.toString('utf8'));
    assert.deepEqual(summaries, run.summaries, 'Reparse actual suite summaries: ' + name);
    const json = boundedRead(path.join(suiteRoot, name + '.json'), 8 * 1024 * 1024);
    assert.deepEqual(JSON.parse(json), run, 'Actual individual child-close receipt differs');
    digests.push({path: name + '.log', sha256: sha(log), bytes: log.length},
      {path: name + '.json', sha256: sha(json), bytes: json.length});
    if (name === 'workspace-live') { workspaceText = log.toString('utf8'); workspaceSummary = summaries; }
  }
  assert(workspaceSummary.length > 0, 'A PASS label with no actual tests is rejected');
  assert(workspaceSummary.every(summary => summary.state === 'ok'));
  const passed = workspaceSummary.reduce((n, x) => n + x.passed, 0);
  const failed = workspaceSummary.reduce((n, x) => n + x.failed, 0);
  const ignored = workspaceSummary.reduce((n, x) => n + x.ignored, 0);
  assert(passed >= minimumExpected);
  assert.equal(failed, 0); assert.equal(ignored, 0);
  assert.equal(proof.passed, passed); assert.equal(proof.failed, failed); assert.equal(proof.ignored, ignored);
  const seen = new Map();
  for (const match of workspaceText.matchAll(/^test ([^\r\n]+?) \.\.\. (ok|FAILED|ignored[^\r\n]*)\r?$/gm)) {
    const leaf = match[1].split('::').at(-1);
    if (!seen.has(leaf)) seen.set(leaf, []);
    seen.get(leaf).push({full: match[1], status: match[2]});
  }
  const rows = [...names, ...rendererGates].map(name => {
    const found = seen.get(name) ?? [];
    assert.equal(found.length, 1, 'Missing/duplicate actual suite test: ' + name);
    assert.equal(found[0].status, 'ok', 'Actual suite test not green: ' + name);
    return {required: name, actual: found[0].full, status: found[0].status};
  });
  for (const expected of expectedNewFullNames) {
    const found = seen.get(expected.split('::').at(-1)) ?? [];
    assert.equal(found.length, 1, 'New authored full-module result missing/duplicate: ' + expected);
    assert.equal(found[0].full, expected, 'New authored function executed in wrong Cargo module');
    assert.equal(found[0].status, 'ok');
  }
  const nameBytes = boundedRead(path.join(suiteRoot, 'required-test-results.json'), 8 * 1024 * 1024);
  assert.deepEqual(JSON.parse(nameBytes), {required_names: names.length, renderer_names: 4,
    results: rows, passed, failed, ignored});
  digests.push({path: 'required-test-results.json', sha256: sha(nameBytes), bytes: nameBytes.length});
  const workerBytes = boundedRead(path.join(suiteRoot, 'owned-worker.json'), 8 * 1024 * 1024);
  const worker = JSON.parse(workerBytes);
  assert.equal(worker.origin, 'http://127.0.0.1:' + proof.owned_worker_port);
  assert.equal(worker.fixture_origin, 'http://127.0.0.1:' + proof.owned_fixture_port);
  assert.equal(worker.health.browser.available, true);
  assert.equal(worker.warmup.title, 'Owned ready');
  assert.equal(worker.warmup.status, 200);
  assert.equal(worker.warmup.final_url, worker.fixture_origin + '/');
  assert.equal(worker.user_renderer_contacted, false);
  assert.equal(path.resolve(worker.renderer_runtime_root), path.resolve(task, 'outputs/open-seo-spider'));
  assert.deepEqual(worker.renderer_input_hashes, manifest.files.filter(file => file.path.startsWith('renderer/')));
  assert.equal(worker.renderer_input_hashes.length, 4);
  for (const file of worker.renderer_input_hashes) {
    const bytes = boundedRead(path.join(worker.renderer_runtime_root, file.path), 64 * 1024 * 1024);
    assert.equal(bytes.length, file.bytes); assert.equal(sha(bytes), file.sha256);
  }
  assert.equal(path.resolve(worker.chrome_path), path.resolve('C:/Program Files/Google/Chrome/Application/chrome.exe'));
  assert.match(worker.chrome_sha256, /^[a-f0-9]{64}$/);
  assert.equal(sha(boundedRead(worker.chrome_path, 256 * 1024 * 1024)), worker.chrome_sha256);
  digests.push({path: 'owned-worker.json', sha256: sha(workerBytes), bytes: workerBytes.length});
  return {proof_sha256: sha(proofBytes), source_manifest_sha256: expectedManifestSha,
    suite_driver_sha256: suiteDriverSha, passed, failed, ignored, required_names: names.length, test_profile: expectedTestProfile, source_lineage: lineage,
    renderer_gates: 4, actual_three_closes: true, owned_resources_closed: true,
    new_authored_full_names: expectedNewFullNames, new_authored_full_names_verified: true,
    final_sources_verified: true, independently_reparsed_logs: true, digests};
}

 const verified=verifySuite();
 assert.deepEqual(verified.new_authored_full_names,expectedNewFullNames);
 return {verified,full_names:expectedNewFullNames};
}
function verifyParentBuild(parent,admittedSuite,pin){
 const manifest=parent,lineage=admittedSuite.source_lineage,source=path.join(BASE,'combined-source-v10'),repo=path.join(BASE,'mcp-file-tools-v1');
 const buildRoot=path.join(BASE,'results/combined-optimized-build-v10'),suitePath=path.join(BASE,'results/combined-suite-v10/proof.json');
 const builderPath=path.join(BASE,'combined-v10-driver-preparation-v2/build-combined-optimized-v10.mjs'),builderSha='38e93a174d69dbce759bfd8eb0ff39068d928d69028cc82bb356db52ab208002';
 pin(builderPath,builderSha);
 const expectedSuiteProofSha=PARENT_SUITE_SHA,expectedManifestSha=PARENT_SHA;
 const release={build_proof_sha256:PARENT_BUILD_SHA,candidate_binary_sha256:PARENT_BINARY_SHA};
 const boundedRead=(file,max)=>{const st=fs.lstatSync(file);assert(st.isFile()&&!st.isSymbolicLink()&&st.size<=max);const raw=fs.readFileSync(file);assert.equal(raw.length,st.size);pin(file,sha(raw));return raw;};
function verifyBuild() {
  assert.equal(sha(boundedRead(builderPath, 1024 * 1024)), builderSha);
  const raw = boundedRead(path.join(buildRoot, 'proof.json'), 8 * 1024 * 1024), build = JSON.parse(raw);
  assert.equal(sha(raw), release.build_proof_sha256);
  assert.equal(build.kind, 'combined_optimized_build_v10');
  assert.equal(build.result, 'PASS_COMBINED_OPTIMIZED_BUILD_V10');
  assert.equal(build.builder_sha256, builderSha);
  assert.equal(build.explicit_suite_proof_sha256,expectedSuiteProofSha);
  assert.equal(build.source_manifest_sha256, expectedManifestSha);
  assert.equal(build.source_commit, manifest.source_commit);
  assert.equal(path.resolve(build.source_directory), path.resolve(source));
  assert.equal(path.resolve(build.source_worktree), path.resolve(repo));
  assert.equal(build.source_inputs, manifest.files.length);
  assert.deepEqual(build.source_lineage, lineage);
  assert.equal(build.source_final_verified, true); assert.equal(build.suite_final_verified, true);
  assert.deepEqual(build.failures, []);
  assert.deepEqual(build.required_suite, {path: suitePath, ...admittedSuite});
  assert.deepEqual(build.compiler_profile, {mode: 'optimized', lto: 'thin', codegen_units: 1, strip: true,
    declared_in: 'Cargo.toml', external_profile_overrides_rejected: true});
  const expected = [
    ['cargo-version', 'C:/Users/matth/.cargo/bin/cargo.exe', ['--version', '--verbose']],
    ['rustc-version', 'C:/Users/matth/.cargo/bin/rustc.exe', ['--version', '--verbose']],
    ['build', 'C:/Users/matth/.cargo/bin/cargo.exe', ['build', '--release', '-p', 'open-seo-spider', '--locked']],
  ];
  assert.equal(build.runs.length, 3);
  const digests = [{path: 'proof.json', sha256: sha(raw), bytes: raw.length}];
  for (const [i, [name, executable, args]] of expected.entries()) {
    const run = build.runs[i]; assert.equal(run.name, name); assert.deepEqual(run.args, args);
    assert.equal(run.executable, executable); assert(Number.isSafeInteger(run.pid) && run.pid > 0);
    assert.equal(run.actual_close_event, true); assert.equal(run.spawn_error, null);
    assert.deepEqual(run.exit, {code: 0, signal: null});
    const log = boundedRead(path.join(buildRoot, name + '.log'), 64 * 1024 * 1024);
    assert.equal(sha(log), run.log_sha256); assert.equal(log.length, run.log_bytes);
    const json = boundedRead(path.join(buildRoot, name + '.json'), 8 * 1024 * 1024);
    assert.deepEqual(JSON.parse(json), run);
    if (name === 'build') assert(/Finished\s+`release`\s+profile\s+\[optimized\]/.test(log.toString('utf8')));
    digests.push({path: name + '.log', sha256: sha(log), bytes: log.length}, {path: name + '.json', sha256: sha(json), bytes: json.length});
  }
  const binary = path.join(buildRoot, 'candidate/open-seo-spider.exe');
  assert.equal(path.resolve(build.binary.path), path.resolve(binary));
  const bytes = boundedRead(binary, 256 * 1024 * 1024), hash = sha(bytes);
  assert.equal(hash, build.binary.sha256); assert.equal(hash, release.candidate_binary_sha256);
  assert.equal(bytes.length, build.binary.bytes);
  assert(bytes.length >= 64 && bytes[0] === 0x4d && bytes[1] === 0x5a);
  const pe = bytes.readUInt32LE(60);
  assert(pe <= bytes.length - 4 && bytes.subarray(pe, pe + 4).equals(Buffer.from([0x50, 0x45, 0, 0])));
  return {proof_sha256: sha(raw), binary_path: binary, binary_sha256: hash, binary_bytes: bytes.length, digests};
}

 const verified=verifyBuild();assert.equal(verified.binary_bytes,PARENT_BINARY_BYTES);return verified;
}

export function loadSource11Approval({approvalPath,approvalSha,expectedCommit,repo=path.join(BASE,'mcp-file-tools-v1')}){
 assert.equal(normalized(repo),normalized(path.join(BASE,'mcp-file-tools-v1')));
 const references=new Map();
 function pin(file,hash){assert(path.isAbsolute(file));assert.match(hash,HASH);const st=fs.lstatSync(file);assert(st.isFile()&&!st.isSymbolicLink()&&st.size<=64*1024*1024,file);const raw=fs.readFileSync(file);assert.equal(raw.length,st.size);assert.equal(sha(raw),hash,file);const key=normalized(file),old=references.get(key);if(old)assert.deepEqual(old,{path:old.path,sha256:hash,bytes:raw.length});else references.set(key,{path:file,sha256:hash,bytes:raw.length});return raw;}
 const data=r=>{const raw=pin(r.path,r.sha256);if(r.bytes!==undefined)assert.equal(raw.length,r.bytes);return JSON.parse(raw);};
 const baselinePath=path.join(BASE,'combined-source-v10/manifest.json');
 const baseline=data({path:baselinePath,sha256:PARENT_SHA});
 assert.equal(baseline.kind,'combined_source_v10');assert.equal(baseline.source_commit,'c6bd9c50a0dfcfb434a7747a2052aab961d00ef7');assert.equal(baseline.source_inputs,369);assert.equal(baseline.required_test_names,598);assert.equal(baseline.minimum_expected_workspace_tests,1380);assert.deepEqual(baseline.test_profile,profile);
 const parentFiles=rowsMap(baseline.files,369),parentRaw=new Map();
 // Only immutable Parent10's direct369 files are read, never its789 references.
 // No comparison of current Root to old Parent10 bytes occurs here.
 for(const f of baseline.files){const raw=pin(path.join(path.dirname(baselinePath),f.path),f.sha256);assert.equal(raw.length,f.bytes);parentRaw.set(f.path,raw);}
 const parentSuiteVerified=verifyParentSuite(baseline,pin),parentBuildVerified=verifyParentBuild(baseline,parentSuiteVerified.verified,pin);
 const parentSuiteReference={path:BASE+'/results/combined-suite-v10/proof.json',sha256:PARENT_SUITE_SHA};
 const parentBuildReference={path:BASE+'/results/combined-optimized-build-v10/proof.json',sha256:PARENT_BUILD_SHA};
 const proposal=data(CSS_PROPOSAL),review=data(CSS_REVIEW);
 assert.equal(proposal.kind,'settings_hint_wrap_proposal');sameRef(proposal.parent_source_manifest,{path:baselinePath,sha256:PARENT_SHA});assert.deepEqual(proposal.owned_paths,[CSS.path]);assert.deepEqual(proposal.source_files,[{path:CSS.path,sha256:CSS.after_sha256,bytes:CSS.bytes}]);assert.deepEqual(proposal.new_cargo_test_names,[]);
 assert.equal(review.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(review.findings,[]);sameRef(review.proposal_manifest,CSS_PROPOSAL);assert.deepEqual(review.files,[CSS]);
 const authored=pin(path.join(proposal.source_payload_directory,CSS.path),CSS.after_sha256);assert.equal(authored.length,CSS.bytes);assert.equal(parentFiles.get(CSS.path).sha256,CSS.before_sha256);assert.equal(parentRaw.get(CSS.path).length,CSS.before_bytes);
 const derivation=data(proposal.derivation);assert.equal(derivation.kind,'settings_hint_wrap_exact_append');assert.equal(derivation.operations.length,1);const op=derivation.operations[0];assert.equal(op.path,CSS.path);assert.equal(op.offset,CSS.before_bytes);assert.equal(op.before_base64,'');assert.equal(op.before_sha256,CSS.before_sha256);assert.equal(op.after_sha256,CSS.after_sha256);const addition=Buffer.from(op.after_base64,'base64');assert.equal(addition.length,131);assert(Buffer.concat([parentRaw.get(CSS.path),addition]).equals(authored));assert(authored.subarray(0,CSS.before_bytes).equals(parentRaw.get(CSS.path)));assert.equal(addition.toString(),'\n/* Keep saved settings JSON examples readable within the dialog. */\n#settings-dialog .settings-grid small{overflow-wrap:anywhere}\n');
 sameRef(proposal.actual_diagnostic,CSS_DIAGNOSTIC);sameRef(proposal.runtime_verification,CSS_PREVIEW);data(CSS_DIAGNOSTIC);
 const preview=data(CSS_PREVIEW);assert.equal(preview.kind,'source10_invalid_settings_css_preview');assert.equal(preview.result,'REPRODUCED_320_OVERFLOW_AND_NARROW_CSS_PREVIEW_PASSES_1440_AND_320');assert.equal(preview.source_manifest_sha256,PARENT_SHA);assert.equal(preview.candidate_binary_sha256,PARENT_BINARY_SHA);assert.equal(preview.preview_css,'#settings-dialog .settings-grid small{overflow-wrap:anywhere}');assert.deepEqual(preview.preview_layouts.map(r=>r.viewport),[1440,320]);for(const r of preview.preview_layouts){assert(r.scroll<=r.client+1);assert(r.document_scroll<=r.viewport+1);}assert.equal(preview.browser_close_awaited,true);assert.deepEqual(preview.app_actual_close,{code:0,signal:null});
 // The optional authored form-width helper is deliberately not a gate.
 const rootApprovalReference={path:approvalPath,sha256:approvalSha},rootApproval=data(rootApprovalReference);
 assert.equal(rootApproval.schema_version,1);assert.equal(rootApproval.kind,'root_approved_combined_source_v11');assert.equal(rootApproval.status,'REVIEWED_MERGED_FORMATTED_COMMITTED_AND_PRIVATELY_PUSHED');assert.equal(rootApproval.parent_source_manifest_sha256,PARENT_SHA);sameRef(rootApproval.parent_suite,parentSuiteReference);sameRef(rootApproval.parent_build,parentBuildReference);sameRef(rootApproval.css_proposal,CSS_PROPOSAL);sameRef(rootApproval.css_review,CSS_REVIEW);sameRef(rootApproval.css_diagnostic,CSS_DIAGNOSTIC);sameRef(rootApproval.css_preview,CSS_PREVIEW);sameRef(rootApproval.response_proposal,RESPONSE_PROPOSAL);sameRef(rootApproval.response_review,RESPONSE_REVIEW);
 assert.match(rootApproval.source_commit,/^[a-f0-9]{40}$/);if(expectedCommit)assert.equal(rootApproval.source_commit,expectedCommit);assert.notEqual(rootApproval.source_commit,baseline.source_commit);assert.equal(rootApproval.source_branch,baseline.source_branch);assert.equal(normalized(rootApproval.source_worktree),normalized(repo));assert.equal(rootApproval.private_repository_verified,true);assert.equal(rootApproval.product_scope,'CSS_AND_EXACT_LENGTH_RESPONSE_COMPLETION_FIVE_EXISTING_PATHS');assert.deepEqual(rootApproval.test_profile,profile);
 const response=data(RESPONSE_PROPOSAL),responseReview=data(RESPONSE_REVIEW);
 assert.equal(response.kind,'http_response_exact_length_source_proposal_v3');sameRef(response.parent_source_manifest,{path:baselinePath,sha256:PARENT_SHA});assert.equal(response.parent_source_commit,baseline.source_commit);assert.deepEqual(response.owned_paths,RESPONSE_PATHS);assert.equal(response.new_input_files,0);assert.equal(response.new_windows_test_count,4);assert.deepEqual([...response.new_authored_full_test_names].sort(),RESPONSE_FULL);assert.equal(response.all_old_test_bytes_exact,true);
 assert.equal(responseReview.kind,'http_response_exact_length_independent_review');assert.equal(responseReview.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(responseReview.findings,[]);sameRef(responseReview.proposal_manifest,RESPONSE_PROPOSAL);
 assert.equal(response.files.length,4);assert.deepEqual(response.files.map(f=>f.path),RESPONSE_PATHS);
 const responseDerivation=data(response.derivation);sameRef(responseDerivation.parent_manifest,{path:baselinePath,sha256:PARENT_SHA});assert.equal(responseDerivation.kind,'http_response_exact_length_narrow_source_patch');assert.equal(responseDerivation.operations.length,5);assert.deepEqual([...new Set(responseDerivation.operations.map(o=>o.path))].sort(),RESPONSE_PATHS);
 const rawOwned=new Map([[CSS.path,authored]]);
 for(const f of response.files){const old=parentFiles.get(f.path);assert.equal(f.before_sha256,old.sha256);assert.equal(f.before_bytes,old.bytes);const bytes=pin(path.join(response.source_payload_directory,f.path),f.after_sha256);assert.equal(bytes.length,f.bytes);const derived=applyOps(parentRaw.get(f.path),responseDerivation.operations.filter(o=>o.path===f.path),f.path);assert(derived.equals(bytes));rawOwned.set(f.path,bytes);}
 const oldTests=parentRaw.get(RESPONSE_TEST_PATH),newTests=rawOwned.get(RESPONSE_TEST_PATH);assert(newTests.subarray(0,oldTests.length).equals(oldTests));const addedSource=newTests.subarray(oldTests.length).toString();const actualNew=[...addedSource.matchAll(/(?:async\s+)?fn\s+(response_exact_length_[a-z0-9_]+)\s*\(/g)].map(m=>m[1]).sort();assert.deepEqual(actualNew,RESPONSE_NAMES);assert.equal((addedSource.match(/#\[tokio::test\(/g)||[]).length,4);const oldNames=[...oldTests.toString().matchAll(/(?:async\s+)?fn\s+(response_[a-z0-9_]+)\s*\(/g)].map(m=>m[1]).sort();assert.equal(oldNames.length,16);assert.equal(new Set(oldNames).size,16);
 const authoredFiles=baseline.files.map(f=>rawOwned.has(f.path)?fileRow(rawOwned.get(f.path),f.path):f);
 const preformat=data(rootApproval.preformat_manifest);assert.equal(preformat.kind,'combined_source_v11_draft_composition');sameRef(preformat.parent_manifest,{path:baselinePath,sha256:PARENT_SHA});assert.equal(preformat.parent_source_manifest_sha256,PARENT_SHA);assert.equal(preformat.parent_source_commit,baseline.source_commit);sameRef(preformat.parent_suite,parentSuiteReference);assert.equal(preformat.source_inputs,369);assert.equal(preformat.parent_source_inputs,369);assert.equal(preformat.owned_paths,5);assert.equal(preformat.changed_parent_inputs,5);assert.equal(preformat.new_input_files,0);assert.equal(preformat.unchanged_parent_inputs,364);assert.deepEqual(preformat.owned_source_paths,OWNED);assert.deepEqual(preformat.files,authoredFiles);sameRef(preformat.response_proposal,RESPONSE_PROPOSAL);sameRef(preformat.response_review,RESPONSE_REVIEW);sameRef(preformat.css_proposal,CSS_PROPOSAL);sameRef(preformat.css_review,CSS_REVIEW);
 const addedFunctions=RESPONSE_NAMES.map(name=>({path:RESPONSE_TEST_PATH,name,full_name:'server::response_stream::tests::'+name}));assert.equal(preformat.new_authored_test_functions.length,4);assert.deepEqual([...preformat.new_authored_test_functions].sort((a,b)=>a.name<b.name?-1:a.name>b.name?1:0),addedFunctions);
 assert(path.isAbsolute(preformat.preformat_directory));assert(path.isAbsolute(preformat.source_directory));assert.notEqual(normalized(preformat.preformat_directory),normalized(preformat.source_directory));assert.notEqual(normalized(preformat.source_directory),normalized(repo));
 const sourceFiles=rowsMap(rootApproval.source_files,369),formatChanges=[],bindings=[];
 for(const f of authoredFiles){const before=pin(path.join(preformat.preformat_directory,f.path),f.sha256);assert.equal(before.length,f.bytes);const after=sourceFiles.get(f.path);assert(after);const final=pin(path.join(preformat.source_directory,f.path),after.sha256);assert.equal(final.length,after.bytes);if(!before.equals(final)){assert(RESPONSE_PATHS.includes(f.path),'Only the four reviewed Rust paths may be formatted');formatChanges.push({path:f.path,before_sha256:f.sha256,after_sha256:after.sha256,bytes:after.bytes});}if(!OWNED.includes(f.path))assert.deepEqual(after,parentFiles.get(f.path));else{const old=parentFiles.get(f.path);assert.notEqual(after.sha256,old.sha256);bindings.push({path:f.path,before_sha256:old.sha256,before_bytes:old.bytes,after_sha256:after.sha256,bytes:after.bytes});}}
 assert.equal(bindings.length,5);assert.deepEqual(rootApproval.files,bindings);assert.equal(sourceFiles.get(CSS.path).sha256,CSS.after_sha256);assert.equal(sourceFiles.get(CSS.path).bytes,CSS.bytes);
 sameRef(rootApproval.composition_review,COMPOSITION_REVIEW);const composition=data(COMPOSITION_REVIEW);assert.equal(composition.kind,'source11_composition_independent_review');assert.equal(composition.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(composition.findings,[]);sameRef(composition.proposal_manifest,rootApproval.preformat_manifest);sameRef(composition.preformat,rootApproval.preformat_manifest);sameRef(composition.actual_format,rootApproval.actual_format);sameRef(composition.parent_manifest,{path:baselinePath,sha256:PARENT_SHA});assert.deepEqual(composition.formatted_files,rootApproval.source_files);assert.deepEqual(composition.approved_product_bindings,preformat.approved_product_bindings);
 const fmt=data(rootApproval.actual_format);assert.equal(fmt.kind,'actual_source11_format_only');assert.equal(fmt.result,'PASS_FORMAT_COMMAND_ONLY');actualClose(fmt);actualPipes(fmt);assert.equal(fmt.preformat_sha256,rootApproval.preformat_manifest.sha256);assert.equal(fmt.source_inputs,369);assert.deepEqual(fmt.changed_files,formatChanges);assert.equal(fmt.log_sha256,rootApproval.format_log.sha256);pin(rootApproval.format_log.path,rootApproval.format_log.sha256);
 const formatter=data(rootApproval.formatter_review);assert.equal(formatter.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(formatter.findings,[]);assert.deepEqual(formatter.files,formatChanges);sameRef(formatter.preformat_manifest,rootApproval.preformat_manifest);sameRef(formatter.actual_format_process_receipt,rootApproval.actual_format);sameRef(formatter.composition_review,COMPOSITION_REVIEW);assert.deepEqual(formatter.formatted_files,rootApproval.source_files);
 const clippy=data(rootApproval.preliminary_clippy);assert.equal(clippy.kind,'source11_actual_preliminary_clippy');assert.equal(clippy.result,'PASS_PRELIMINARY_CLIPPY_ONLY');actualClose(clippy);assert.equal(clippy.executed_tests,0);assert.equal(clippy.source_final_verified,true);assert.equal(clippy.source_inputs,369);assert.deepEqual(clippy.files,rootApproval.source_files);assert.equal(clippy.preformat_sha256,rootApproval.preformat_manifest.sha256);assert.equal(clippy.format_sha256,rootApproval.actual_format.sha256);assert.equal(clippy.log_sha256,rootApproval.preliminary_clippy_log.sha256);const clippyRaw=pin(rootApproval.preliminary_clippy_log.path,rootApproval.preliminary_clippy_log.sha256);assert(!/^error(?:\[|:)/m.test(clippyRaw.toString()));
 sameRef(rootApproval.directed_driver,DIRECTED_DRIVER);pin(DIRECTED_DRIVER.path,DIRECTED_DRIVER.sha256);
 const directed=data(rootApproval.directed_tests);assert.equal(directed.kind,'source11_actual_response_stream_tests_only');assert.equal(directed.result,'PASS_20_RESPONSE_STREAM_TEST_NAMES_ONLY');assert.equal(directed.source_final_verified,true);assert.equal(directed.old_names_verified,16);assert.equal(directed.new_names_verified,4);assert.equal(directed.preformat_sha256,rootApproval.preformat_manifest.sha256);assert.equal(directed.preliminary_clippy_sha256,rootApproval.preliminary_clippy.sha256);assert.equal(normalized(directed.source_directory),normalized(preformat.source_directory));assert.equal(directed.source_inputs,369);assert.deepEqual(directed.failures,[]);assert.equal(directed.runs.length,1);const directedRun=directed.runs[0];actualClose(directedRun);assert.equal(directedRun.spawn_error,null);assert.equal(directedRun.filter,'server::response_stream::tests');assert.deepEqual(directedRun.args,['test','-p','open-seo-spider','--locked','server::response_stream::tests','--','--include-ignored','--test-threads=1']);assert.equal(directedRun.log_sha256,rootApproval.directed_tests_log.sha256);
 const allResponseNames=[...oldNames,...RESPONSE_NAMES].sort().map(n=>'server::response_stream::tests::'+n);const resultRows=names=>names.map(actual=>({required:actual.split('::').at(-1),actual,status:'ok'}));assert.deepEqual(data(rootApproval.new_test_names),resultRows(RESPONSE_FULL));assert.deepEqual(data(rootApproval.response_stream_test_names),resultRows(allResponseNames));const directedRaw=pin(rootApproval.directed_tests_log.path,rootApproval.directed_tests_log.sha256),directedRows=jsonLines(directedRaw),runNames=directedRows.map(r=>r.name).sort();assert.deepEqual(runNames,allResponseNames);assert(directedRows.every(r=>r.state==='ok'));const summary=summaries(directedRaw);assert(summary.length>0);assert(summary.every(r=>r.state==='ok'&&r.failed===0&&r.ignored===0));assert.equal(summary.reduce((n,r)=>n+r.passed,0),20);assert.deepEqual(directedRun.summaries,summary);assert.equal(directedRun.log_bytes,directedRaw.length);
 assert.deepEqual(rootApproval.new_authored_tests,[{path:RESPONSE_TEST_PATH,names:RESPONSE_NAMES}]);assert.deepEqual(rootApproval.new_authored_full_test_names,RESPONSE_FULL);
 const publication=data(rootApproval.private_publication);assert.equal(publication.kind,'actual_private_source11_code_publication');assert.equal(publication.source_commit,rootApproval.source_commit);assert.equal(publication.branch,rootApproval.source_branch);assert.equal(publication.repository,'lia-xim/open-seo-spider');assert.equal(publication.is_private,true);assert.equal(publication.source_inputs,369);assert.equal(publication.all369_root_source_hashes_verified,true);assert.equal(publication.all369_commit_blob_hashes_verified,true);assert.equal(publication.remote_exact,true);assert.equal(publication.code_and_owned_raw_byte_attributes,true);assert.deepEqual(publication.owned_committed_paths,OWNED);assert.deepEqual(publication.additional_tracked_files,[]);
 for(const f of sourceFiles.values()){const st=fs.lstatSync(path.join(repo,f.path));assert(st.isFile()&&!st.isSymbolicLink());const raw=fs.readFileSync(path.join(repo,f.path));assert.equal(raw.length,f.bytes);assert.equal(sha(raw),f.sha256);}
 const requiredTests=structuredClone(baseline.required_tests),testGroup=requiredTests.find(g=>g.path===RESPONSE_TEST_PATH);assert(testGroup);testGroup.names.push(...RESPONSE_NAMES);testGroup.names.sort();const required=requiredTests.flatMap(g=>g.names);assert.equal(required.length,602);assert.equal(new Set(required).size,602);
 return {baselinePath,baselineSha:PARENT_SHA,baseline,parentFiles,parentRaw,rootApproval,rootApprovalReference,sourceFiles,requiredTests,required,references,pin,parentSuiteVerified,parentBuildVerified,parentSuiteReference,parentBuildReference,addedTests:[{path:RESPONSE_TEST_PATH,names:RESPONSE_NAMES}],newFullNames:RESPONSE_FULL,minimum:1384,paths:[...sourceFiles.keys()],bindings};
}

// Pure manifest construction after Root has already reviewed, privately pushed,
// and hash-pinned the actual five-path promotion. It writes nothing itself.
function manifestFromContext(c,repo){
 const validatorPath=path.join(TASK,'work/validate-combined-source-v11-r2.mjs');
 assert.equal(normalized(fileURLToPath(import.meta.url)),normalized(validatorPath));
 const validatorRaw=fs.readFileSync(validatorPath),validatorSha=sha(validatorRaw);c.pin(validatorPath,validatorSha);
 return {schema_version:1,kind:'combined_source_v11',result:'FROZEN_REVIEWED_SOURCE_FULL_SUITE_NOT_RUN',root_approval:c.rootApprovalReference,parent_source_manifest_sha256:PARENT_SHA,parent_source_inputs:369,source_inputs:369,inherited_required_test_names:598,new_input_files:0,changed_parent_inputs:5,unchanged_parent_inputs:364,minimum_expected_workspace_tests:1384,required_test_names:602,new_authored_test_functions:4,test_profile:profile,files:[...c.sourceFiles.values()],required_tests:c.requiredTests,new_authored_tests:c.addedTests,new_authored_full_test_names:c.newFullNames,all_required_full_test_names:[...c.parentSuiteVerified.full_names,...c.newFullNames].sort(),source_proposals:[{role:'settings_hint_wrap',...CSS_PROPOSAL,review_path:CSS_REVIEW.path,review_sha256:CSS_REVIEW.sha256},{role:'response_exact_length',...RESPONSE_PROPOSAL,review_path:RESPONSE_REVIEW.path,review_sha256:RESPONSE_REVIEW.sha256}],parent_suite:c.parentSuiteReference,parent_build:c.parentBuildReference,source_commit:c.rootApproval.source_commit,source_branch:c.rootApproval.source_branch,source_worktree:repo,approved_product_bindings:c.bindings,validator:{path:validatorPath,sha256:validatorSha},references:[...c.references.values()]};
}
export function createSource11Manifest({approvalPath,approvalSha,expectedCommit,repo=path.join(BASE,'mcp-file-tools-v1')}){
 const c=loadSource11Approval({approvalPath,approvalSha,expectedCommit,repo});return manifestFromContext(c,repo);
}
export function validateSource11Manifest({manifest,sourceDirectory,repo=path.join(BASE,'mcp-file-tools-v1')}){
 assert.equal(normalized(sourceDirectory),normalized(path.join(BASE,'combined-source-v11')));
 const c=loadSource11Approval({approvalPath:manifest.root_approval.path,approvalSha:manifest.root_approval.sha256,expectedCommit:manifest.source_commit,repo});
 const expected=manifestFromContext(c,repo);assert.deepEqual(manifest,expected);
 const validatorPath=fileURLToPath(import.meta.url);assert.equal(normalized(manifest.validator.path),normalized(validatorPath));c.pin(manifest.validator.path,manifest.validator.sha256);
 const published=new Map();for(const r of manifest.references){assert(!published.has(normalized(r.path)));published.set(normalized(r.path),r);}assert.equal(published.size,c.references.size);for(const [key,r]of c.references)assert.deepEqual(published.get(key),r);
 for(const f of c.sourceFiles.values()){const st=fs.lstatSync(path.join(sourceDirectory,f.path));assert(st.isFile()&&!st.isSymbolicLink());const raw=fs.readFileSync(path.join(sourceDirectory,f.path));assert.equal(raw.length,f.bytes);assert.equal(sha(raw),f.sha256);}
 const files=c.sourceFiles,rendererFiles=[...files.values()].filter(f=>f.path.startsWith('renderer/'));assert.equal(rendererFiles.length,4);
 const lineage=Object.freeze({parent_source_manifest_sha256:PARENT_SHA,parent_suite_proof_sha256:PARENT_SUITE_SHA,parent_build_proof_sha256:PARENT_BUILD_SHA,css_proposal_sha256:CSS_PROPOSAL.sha256,css_review_sha256:CSS_REVIEW.sha256,response_proposal_sha256:RESPONSE_PROPOSAL.sha256,response_review_sha256:RESPONSE_REVIEW.sha256,root_approval_sha256:manifest.root_approval.sha256,parent_inputs:369,required_parent_names:598,unchanged_parent_inputs:364,changed_parent_inputs:5,new_input_files:0,delta_bindings:5,new_named_windows_functions:c.newFullNames,actual_required_names:602,actual_source_inputs:369,minimum_expected_workspace_tests:1384});
 const expectedNewFullNames=[...c.parentSuiteVerified.full_names,...c.newFullNames].sort();assert.equal(expectedNewFullNames.length,606);assert.equal(new Set(expectedNewFullNames).size,606);
 return {...c,files,rendererFiles,lineage,pinnedReferences:published,expectedNewFullNames};
}
