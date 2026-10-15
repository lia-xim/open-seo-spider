import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';

// Direct immutable Parent11 proof only. No former validator is imported or run.
const BASE='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const TASK='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
export const PARENT_SHA='8a55a0ba333742124b36a33fde8a889afcc594e052b09568910dccc3ea5e519c';
const PARENT_SUITE_SHA='9f19222aec9081ff361f3802523bd1d5a7f8a8da5a505442a2d7832aeb7946c0';
const PARENT_BUILD_SHA='1c421abd30f58ba5944c660a9776509e6c836cbc647d23ef03b474162bb35910';
const PARENT_BINARY_SHA='2d46366500887caeee33c45a972639dd379c02e678a9e07595a506244f9265f7';
const PARENT_BINARY_BYTES=43149824;
// Final immutable role/review refs are filled only after the actual Ngram closure.
const ROLE_PINS=[
  {
    "role": "static_limits",
    "proposal_manifest": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\crawl-static-limits-proposal-v3\\manifest.json",
      "sha256": "c33bd9315347d3f969ab89dd25482ceac5205f83bed776b93f1cb8caa82fcd67",
      "bytes": 76687
    },
    "review": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\crawl-static-limits-independent-review-v3\\receipt.json",
      "sha256": "a4ad5aab0270dfa11a6537278e4bce2128583c5acd0495b04c7d770ca54f0ab6",
      "bytes": 2672
    }
  },
  {
    "role": "sitemap_request_index",
    "proposal_manifest": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\sitemap-request-index-proposal-v8\\manifest.json",
      "sha256": "1c609a8ff3b352456f86145d4c97032fc7ec2f0df8fc16a5d0553828d6c5d5b8",
      "bytes": 5291
    },
    "review": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\sitemap-request-index-independent-review-v8\\receipt.json",
      "sha256": "510c8b1552ad368ae79d630b1e6ea37829e5bae8f6abe6d174d258842af37873",
      "bytes": 9250
    }
  },
  {
    "role": "ngram_html_channels",
    "proposal_manifest": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\saved-ngram-html-channels-proposal-v4\\manifest.json",
      "sha256": "2266ddc3a5372196c67b800955681122c5778d5435d9103a43e828b100bdad63",
      "bytes": 155218
    },
    "review": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\saved-ngram-html-channels-independent-review-v4\\receipt.json",
      "sha256": "03ff65b230b229d29f06e8b83a00b9e77e9e1ec270344662176758941b4c40a8",
      "bytes": 3281
    }
  }
];
const HASH=/^[a-f0-9]{64}$/,COMMIT=/^[a-f0-9]{40}$/;
const sha=b=>createHash('sha256').update(b).digest('hex');
const normalized=p=>path.resolve(p).toLowerCase();
const sort=(a,b)=>a<b?-1:a>b?1:0;
const profile=Object.freeze({opt_level:1,debug_assertions:true,overflow_checks:true,debug_info:0,environment:{CARGO_PROFILE_TEST_OPT_LEVEL:'1',CARGO_PROFILE_TEST_DEBUG_ASSERTIONS:'true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS:'true',CARGO_PROFILE_TEST_DEBUG:'0',CARGO_PROFILE_DEV_DEBUG:'0'}});
const gates=['live_browser_worker_executes_javascript_for_core_crawl','live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness','live_browser_worker_pause_drains_render_and_retains_rendered_discoveries','live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion'];
export function relativeFile(value){assert.equal(typeof value,'string');assert(value&&!value.includes('\\')&&!value.includes(':')&&!path.posix.isAbsolute(value));assert(value.split('/').every(p=>p&&p!=='.'&&p!=='..'));return value;}
function sameRef(a,b){assert(a&&b);assert.equal(normalized(a.path),normalized(b.path));assert.equal(a.sha256,b.sha256);if(b.bytes!==undefined)assert.equal(a.bytes,b.bytes);}
function sameRoles(actual,expected){assert(Array.isArray(actual));assert.equal(actual.length,expected.length);for(const [i,r]of expected.entries()){assert.deepEqual(Object.keys(actual[i]).sort(),['proposal_manifest','review','role']);assert.equal(actual[i].role,r.role);sameRef(actual[i].proposal_manifest,r.proposal_manifest);sameRef(actual[i].review,r.review);}}
function sameRoleReferences(actual,expected,field){assert(Array.isArray(actual));assert.equal(actual.length,expected.length);for(const [i,r]of expected.entries()){assert.equal(actual[i].role,r.role);sameRef(actual[i],r[field]);}}
function sameNodeCases(actual,expected){assert.equal(actual.length,expected.length);for(const [i,e]of expected.entries()){assert.equal(actual[i].role,e.role);assert.equal(actual[i].status,e.status);assert.deepEqual(actual[i].names,e.names);sameRef(actual[i].source,e.source);}}
function actualClose(v){assert.equal(v.actual_close_event,true);assert(Number.isSafeInteger(v.pid)&&v.pid>0);assert.deepEqual(v.exit,{code:0,signal:null});}
function actualPipes(v){assert.deepEqual(v.io_errors,[]);for(const side of ['stdout','stderr']){assert.equal(v.streams[side].end,true);assert.equal(v.streams[side].close,true);assert(Number.isSafeInteger(v.streams[side].bytes)&&v.streams[side].bytes>=0);}}
function rowsMap(rows){assert(Array.isArray(rows));const m=new Map();for(const f of rows){relativeFile(f.path);assert.deepEqual(Object.keys(f).sort(),['bytes','path','sha256']);assert.match(f.sha256,HASH);assert(Number.isSafeInteger(f.bytes)&&f.bytes>=0);assert(!m.has(f.path));m.set(f.path,f);}assert.deepEqual([...m.keys()],[...m.keys()].sort(sort));assert.equal(new Set([...m.keys()].map(p=>p.toLowerCase())).size,m.size);return m;}
function applyOps(before,ops,rel){let current=before;for(const o of ops){assert.equal(o.path,rel);assert(Number.isSafeInteger(o.offset)&&o.offset>=0&&o.offset<=current.length);assert.equal(sha(current),o.before_sha256);const old=Buffer.from(o.before_base64,'base64'),next=Buffer.from(o.after_base64,'base64');assert(current.subarray(o.offset,o.offset+old.length).equals(old));current=Buffer.concat([current.subarray(0,o.offset),next,current.subarray(o.offset+old.length)]);assert.equal(sha(current),o.after_sha256);}let inverse=current;for(const o of [...ops].reverse()){assert.equal(sha(inverse),o.after_sha256);const old=Buffer.from(o.before_base64,'base64'),next=Buffer.from(o.after_base64,'base64');assert(inverse.subarray(o.offset,o.offset+next.length).equals(next));inverse=Buffer.concat([inverse.subarray(0,o.offset),old,inverse.subarray(o.offset+next.length)]);assert.equal(sha(inverse),o.before_sha256);}assert(inverse.equals(before));return current;}
function fnNames(source){return [...source.matchAll(/#\[(?:tokio::)?test(?:\([^\]]*\))?\]\s*(?:async\s+)?fn\s+(\w+)\s*\(/g)].map(m=>m[1]);}
function noFindings(review){assert.equal(review.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(review.findings,[]);}

function verifyParentEvidence(manifest,pin){
 const task=TASK,source=path.join(BASE,'combined-source-v11'),repo=manifest.source_worktree;
 const suiteRoot=path.join(BASE,'results/combined-suite-v11'),buildRoot=path.join(BASE,'results/combined-optimized-build-v11');
 const driverPath="E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-v11-exact-length-driver-preparation-v2\\run-combined-suite-v11-r2.mjs",suiteDriverSha="27e0802434e8d83b2b212b4d5c96304ed9b5148b97c8b602b389a67101a08945";
 const builderPath="E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-v11-exact-length-driver-preparation-v2\\build-combined-optimized-v11-r2.mjs",builderSha="0f7c091addfd4d235db4497946c8d4cc97990e161c87ee3116f5f6a116592385";
 pin(driverPath,suiteDriverSha);pin(builderPath,builderSha);pin("E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-suite-v11\\owned-worker.json","a67636b84ea5f07b0429a63078ee4377efb8bbc6a97a86fd3b609d12d77f724e");
 const expectedManifestSha=PARENT_SHA,expectedSuiteProofSha=PARENT_SUITE_SHA;
 const authenticatedSuite=JSON.parse(pin(path.join(suiteRoot,'proof.json'),PARENT_SUITE_SHA));
 const lineage=authenticatedSuite.source_lineage,names=manifest.required_tests.flatMap(g=>g.names),minimumExpected=1384,expectedTestProfile=profile,rendererGates=gates;
 const expectedNewFullNames=manifest.all_required_full_test_names;
 const release={build_proof_sha256:PARENT_BUILD_SHA,candidate_binary_sha256:PARENT_BINARY_SHA};
 const suitePath=path.join(suiteRoot,'proof.json');
 const boundedRead=(file,max)=>{const st=fs.lstatSync(file);assert(st.isFile()&&!st.isSymbolicLink()&&st.size<=max);const bytes=fs.readFileSync(file);assert.equal(bytes.length,st.size);return pin(file,sha(bytes),max);};
 const parseSummaries=text=>[...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(m=>({state:m[1],passed:+m[2],failed:+m[3],ignored:+m[4]}));
const suiteCommands = [
  ['format', ['fmt', '--all', '--check']],
  ['workspace-live', ['test', '--workspace', '--locked', '--', '--include-ignored', '--test-threads=1']],
  ['clippy', ['clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings']],
];

function verifySuite() {
  assert.equal(sha(fs.readFileSync(driverPath)), suiteDriverSha, 'Exact actual suite producer required');
  const proofBytes = boundedRead(path.join(suiteRoot, 'proof.json'), 8 * 1024 * 1024);
  assert.equal(sha(proofBytes), expectedSuiteProofSha, 'Explicit actual suite proof hash required');
  const proof = JSON.parse(proofBytes);
  assert.equal(proof.kind, 'combined_full_suite_v11');
  assert.equal(proof.result, 'PASS_COMBINED_WORKSPACE_V11_REAL_RENDERER_FMT_CLIPPY');
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
    const bytes = boundedRead(path.join(source, file.path), 64 * 1024 * 1024);
    assert.equal(bytes.length, file.bytes); assert.equal(sha(bytes), file.sha256);
  }
  assert.equal(path.resolve(worker.chrome_path), path.resolve('C:/Program Files/Google/Chrome/Application/chrome.exe'));
  assert.match(worker.chrome_sha256, /^[a-f0-9]{64}$/);
  assert.equal(worker.chrome_sha256,"9ca2beaa7a9fd1109f01e562788c22e045a01cec0f6015ade689325af7f8a521");
  digests.push({path: 'owned-worker.json', sha256: sha(workerBytes), bytes: workerBytes.length});
  return {proof_sha256: sha(proofBytes), source_manifest_sha256: expectedManifestSha,
    suite_driver_sha256: suiteDriverSha, passed, failed, ignored, required_names: names.length, test_profile: expectedTestProfile, source_lineage: lineage,
    renderer_gates: 4, actual_three_closes: true, owned_resources_closed: true,
    new_authored_full_names: expectedNewFullNames, new_authored_full_names_verified: true,
    final_sources_verified: true, independently_reparsed_logs: true, digests};
}

 const admittedSuite=verifySuite();
function verifyBuild() {
  assert.equal(sha(boundedRead(builderPath, 1024 * 1024)), builderSha);
  const raw = boundedRead(path.join(buildRoot, 'proof.json'), 8 * 1024 * 1024), build = JSON.parse(raw);
  assert.equal(sha(raw), release.build_proof_sha256);
  assert.equal(build.kind, 'combined_optimized_build_v11');
  assert.equal(build.result, 'PASS_COMBINED_OPTIMIZED_BUILD_V11');
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

 const admittedBuild=verifyBuild();assert.equal(admittedBuild.binary_sha256,PARENT_BINARY_SHA);assert.equal(admittedBuild.binary_bytes,PARENT_BINARY_BYTES);
 return{...admittedSuite,full_names:expectedNewFullNames,build:admittedBuild,parent_renderer_files_scope:'immutable_frozen_source11_only',parent_chrome_identity_scope:'actual_authenticated_parent_worker_capture_only'};
}


function loadApproval({approvalPath,expectedApprovalSha,sourceDirectory,repo}){
 assert(Array.isArray(ROLE_PINS)&&ROLE_PINS.length===3,'Final actual role/review pins are required; WIP cannot admit execution');
 assert.equal(normalized(sourceDirectory),normalized(path.join(BASE,'combined-source-v12')));
 assert.equal(normalized(repo),normalized(path.join(BASE,'mcp-file-tools-v1')));
 const references=new Map();
 const pin=(p,expected,max=8*1024*1024)=>{assert(path.isAbsolute(p));p=path.resolve(p);assert.match(expected,HASH);const st=fs.lstatSync(p);assert(st.isFile()&&!st.isSymbolicLink()&&st.size<=max,'Bounded regular evidence required: '+p);const raw=fs.readFileSync(p);assert.equal(raw.length,st.size);assert.equal(sha(raw),expected,p);const r={path:p,sha256:expected,bytes:raw.length},key=normalized(p);if(references.has(key))assert.deepEqual(references.get(key),r);else references.set(key,r);return raw;};
 const data=r=>{assert(r&&path.isAbsolute(r.path));assert(Number.isSafeInteger(r.bytes)&&r.bytes>=0);const raw=pin(r.path,r.sha256);assert.equal(raw.length,r.bytes);return JSON.parse(raw);};
 const baselinePath=path.join(BASE,'combined-source-v11/manifest.json');const baseline=JSON.parse(pin(baselinePath,PARENT_SHA));assert.equal(baseline.kind,'combined_source_v11');assert.equal(baseline.source_commit,'61b0e3bca5386a17b449915b2915222e245084c5');assert.deepEqual(baseline.test_profile,profile);assert.equal(baseline.source_inputs,369);assert.equal(baseline.required_test_names,602);assert.equal(baseline.minimum_expected_workspace_tests,1384);assert.equal(baseline.all_required_full_test_names.length,606);
 const parentFiles=rowsMap(baseline.files);assert.equal(parentFiles.size,369);const parentRaw=new Map();for(const f of parentFiles.values()){const raw=pin(path.join(BASE,'combined-source-v11',f.path),f.sha256);assert.equal(raw.length,f.bytes);parentRaw.set(f.path,raw);}
 const parentSuiteVerified=verifyParentEvidence(baseline,pin);const parentSuiteReference={path:path.join(BASE,'results/combined-suite-v11/proof.json'),sha256:PARENT_SUITE_SHA,bytes:fs.statSync(path.join(BASE,'results/combined-suite-v11/proof.json')).size};const parentBuildReference={path:path.join(BASE,'results/combined-optimized-build-v11/proof.json'),sha256:PARENT_BUILD_SHA,bytes:fs.statSync(path.join(BASE,'results/combined-optimized-build-v11/proof.json')).size};
 assert.match(expectedApprovalSha,HASH);const rootApprovalReference={path:approvalPath,sha256:expectedApprovalSha,bytes:fs.statSync(approvalPath).size},a=data(rootApprovalReference);
 assert.equal(a.schema_version,1);assert.equal(a.kind,'root_approved_combined_source_v12');assert.equal(a.status,'REVIEWED_MERGED_FORMATTED_COMMITTED_AND_PRIVATELY_PUSHED');sameRef(a.parent_manifest,{path:baselinePath,sha256:PARENT_SHA,bytes:fs.statSync(baselinePath).size});sameRef(a.parent_suite,parentSuiteReference);sameRef(a.parent_build,parentBuildReference);assert.match(a.source_commit,COMMIT);assert.notEqual(a.source_commit,baseline.source_commit);assert.equal(typeof a.source_branch,'string');assert(a.source_branch.length>0&&a.source_branch.length<=256);assert.equal(normalized(a.source_worktree),normalized(repo));assert.equal(a.private_repository_verified,true);
 sameRoles(a.source_proposals,ROLE_PINS);
 const owned=new Map(),newFunctions=[],nodeCases=[];
 for(const role of ROLE_PINS){const m=data(role.proposal_manifest),review=data(role.review);noFindings(review);sameRef(review.proposal_manifest,role.proposal_manifest);sameRef(m.parent_manifest,{path:baselinePath,sha256:PARENT_SHA});assert.equal(m.parent_source_commit,baseline.source_commit);const dir=m.source_payload_directory??m.source_directory;assert(path.isAbsolute(dir));const d=data(m.derivation);const ops=d.operations??d.files.flatMap(f=>(f.operations??[]).map(o=>({...o,path:f.path})));assert(Array.isArray(ops));
  for(const f of m.files){relativeFile(f.path);assert(!owned.has(f.path),'Role paths overlap; separate merge approval would be required: '+f.path);assert(!/NOT.IN.CARGO|BENCHMARK|benchmark/i.test(f.path),'Benchmark is not a product input');const raw=pin(path.join(dir,f.path),f.sha256);assert.equal(raw.length,f.bytes);const before=parentFiles.get(f.path);if(before){if(f.before_sha256!==undefined)assert.equal(f.before_sha256,before.sha256);const steps=ops.filter(o=>o.path===f.path);assert(steps.length>0);assert(applyOps(parentRaw.get(f.path),steps,f.path).equals(raw));}else{assert(f.before_sha256===undefined||f.before_sha256==='ABSENT'||f.before_sha256===null);assert.equal(ops.filter(o=>o.path===f.path).length,0,'New files are reviewed payloads, not former Root state');}owned.set(f.path,{role:role.role,path:f.path,before_sha256:before?.sha256??null,before_bytes:before?.bytes??null,authored_sha256:f.sha256,authored_bytes:f.bytes,raw});}
  const declared=Array.isArray(m.new_authored_test_functions)?m.new_authored_test_functions:role.role==='sitemap_request_index'?m.new_authored_test_names.map(name=>({path:'crates/core/src/sitemap_membership_analysis/request_index_tests.rs',name,full_name:'sitemap_membership_analysis::request_index::tests::'+name,windows:true,unix:true})):null;assert(Array.isArray(declared)&&declared.length>0);
  for(const t of declared){assert.equal(t.windows,true);assert.equal(t.unix,true);assert.match(t.name,/^[A-Za-z_][A-Za-z0-9_]*$/);assert(t.full_name.endsWith('::'+t.name));const f=owned.get(t.path);assert(f&&f.role===role.role);assert.equal(fnNames(f.raw.toString()).filter(n=>n===t.name).length,1);newFunctions.push({path:t.path,name:t.name,full_name:t.full_name,windows:true,unix:true});}
  let nodeSource=m.node_regression_source??m.support_files?.find(r=>r.path.endsWith('saved-ngram-channel-ui.test.mjs'));let staticNodeNames=null;
  if(role.role==='static_limits'&&nodeSource){const current=pin(nodeSource.path,nodeSource.sha256);assert.equal(current.length,nodeSource.bytes);const inventory=data(m.test_inventory),declared=inventory.node_regression;assert.equal(inventory.kind,'crawl_static_limits_authored_test_inventory');assert(declared&&Array.isArray(declared.cases));assert.equal(declared.status,'AUTHORED_NOT_RUN');assert.equal(declared.cases.length,4);assert.equal(new Set(declared.cases).size,4);assert.equal(declared.sha256,nodeSource.sha256);assert.equal(declared.bytes,nodeSource.bytes);const original=pin(declared.path,declared.sha256);assert.equal(original.length,declared.bytes);assert(original.equals(current),'Current and inventoried static Node sources must be the same complete bytes');nodeSource={path:declared.path,sha256:declared.sha256,bytes:declared.bytes};staticNodeNames=declared.cases;}
  if(nodeSource){const bytes=pin(nodeSource.path,nodeSource.sha256);assert.equal(bytes.length,nodeSource.bytes);const text=bytes.toString();let names;if(role.role==='static_limits'){const match=text.match(/cases:\[([^\]]+)\]/);assert(match);names=[...match[1].matchAll(/'([^']+)'/g)].map(m=>m[1]);assert.equal(m.additional_non_cargo_ui_cases,4);assert.deepEqual(names,staticNodeNames);}else names=[...text.matchAll(/(?:^|\n)(?:test|it)\('([^']+)'/g)].map(m=>m[1]);assert.equal(names.length,4);assert.equal(new Set(names).size,4);nodeCases.push({role:role.role,source:nodeSource,names,status:'AUTHORED_NOT_RUN'});}
 }
 const counts={owned:owned.size,modified:[...owned.values()].filter(f=>f.before_sha256!==null).length,new:[...owned.values()].filter(f=>f.before_sha256===null).length};counts.inputs=parentFiles.size+counts.new;counts.unchanged=parentFiles.size-counts.modified;assert.equal(new Set(newFunctions.map(t=>t.name)).size,newFunctions.length);assert.equal(new Set(newFunctions.map(t=>t.full_name)).size,newFunctions.length);newFunctions.sort((a,b)=>sort(a.path,b.path)||sort(a.name,b.name));
 const authored=baseline.files.map(f=>owned.has(f.path)?{path:f.path,sha256:owned.get(f.path).authored_sha256,bytes:owned.get(f.path).authored_bytes}:f).concat([...owned.values()].filter(f=>f.before_sha256===null).map(f=>({path:f.path,sha256:f.authored_sha256,bytes:f.authored_bytes}))).sort((a,b)=>sort(a.path,b.path));
 const requiredTests=structuredClone(baseline.required_tests);for(const t of newFunctions){let group=requiredTests.find(g=>g.path===t.path);if(!group){group={path:t.path,names:[]};requiredTests.push(group);}assert(!group.names.includes(t.name));group.names.push(t.name);group.names.sort(sort);}for(const group of requiredTests)group.names.sort(sort);requiredTests.sort((a,b)=>sort(a.path,b.path));const required=requiredTests.flatMap(g=>g.names);assert.equal(required.length,602+newFunctions.length);assert.equal(new Set(required).size,required.length);const full=[...baseline.all_required_full_test_names,...newFunctions.map(t=>t.full_name)].sort(sort);assert.equal(full.length,606+newFunctions.length);assert.equal(new Set(full).size,full.length);const minimum=1384+newFunctions.length;
 const pre=data(a.preformat_manifest);assert.equal(pre.kind,'root_source12_complete_composition_before_format');sameRef(pre.parent_manifest,a.parent_manifest);assert.equal(pre.parent_source_commit,baseline.source_commit);sameRoles(pre.source_proposals,ROLE_PINS);assert.deepEqual(pre.files,authored);assert.deepEqual(pre.new_authored_test_functions,newFunctions);assert.deepEqual(pre.required_tests,requiredTests);assert.deepEqual(pre.all_required_full_test_names,full);assert.equal(pre.minimum_expected_workspace_tests,minimum);sameNodeCases(pre.additional_non_cargo_cases,nodeCases);assert(path.isAbsolute(pre.source_directory)&&path.isAbsolute(pre.preformat_directory));assert.notEqual(normalized(pre.source_directory),normalized(pre.preformat_directory));assert.notEqual(normalized(pre.source_directory),normalized(repo));
 const authoredBindings=[...owned.values()].map(({raw,...f})=>f).sort((a,b)=>sort(a.path,b.path));assert.deepEqual(pre.owned_changes,authoredBindings);
 const final=rowsMap(a.source_files);assert.equal(final.size,counts.inputs);assert.deepEqual([...final.keys()],authored.map(f=>f.path));const formatChanges=[],bindings=[];
 for(const f of authored){const before=pin(path.join(pre.preformat_directory,f.path),f.sha256);assert.equal(before.length,f.bytes);const after=final.get(f.path),formatted=pin(path.join(pre.source_directory,f.path),after.sha256);assert.equal(formatted.length,after.bytes);if(owned.has(f.path)){const o=owned.get(f.path);bindings.push({role:o.role,path:f.path,before_sha256:o.before_sha256,before_bytes:o.before_bytes,authored_sha256:f.sha256,authored_bytes:f.bytes,after_sha256:after.sha256,bytes:after.bytes});}else assert.deepEqual(after,parentFiles.get(f.path));if(!before.equals(formatted)){assert(owned.has(f.path)&&f.path.endsWith('.rs'),'Only owned Rust files may be formatted');formatChanges.push({path:f.path,before_sha256:f.sha256,after_sha256:after.sha256,bytes:after.bytes});}}
 assert.deepEqual(a.files,bindings);
 const composition=data(a.composition_review);noFindings(composition);sameRef(composition.proposal_manifest??composition.composition_preformat,a.preformat_manifest);sameRef(composition.parent_manifest,a.parent_manifest);sameRoles(composition.source_proposals,ROLE_PINS);
 const allFormatRows=authored.map(f=>({path:f.path,before_sha256:f.sha256,before_bytes:f.bytes,after_sha256:final.get(f.path).sha256,bytes:final.get(f.path).bytes}));const changedFormatRows=allFormatRows.filter(f=>f.before_sha256!==f.after_sha256);assert.deepEqual(changedFormatRows.map(({before_bytes,...r})=>r),formatChanges);
 const fmt=data(a.actual_format);assert.equal(fmt.kind,'actual_source12_preliminary_format');assert.equal(fmt.status,'PASS_ACTUAL_CARGO_FMT');sameRef(fmt.preformat_manifest,a.preformat_manifest);assert(Number.isSafeInteger(fmt.actual_process.pid)&&fmt.actual_process.pid>0);assert.equal(fmt.actual_process.status,0);assert.equal(fmt.actual_process.signal,null);assert.equal(fmt.actual_process.error,null);assert.equal(fmt.actual_process.observation,'spawnSync returned after child and pipe completion');assert.equal(normalized(fmt.command.cwd),normalized(pre.source_directory));assert.equal(normalized(fmt.command.executable),normalized('C:/Users/matth/.cargo/bin/cargo.exe'));assert.deepEqual(fmt.command.args.slice(0,3),['fmt','--all','--manifest-path']);assert.equal(fmt.command.args.length,4);assert.equal(normalized(fmt.command.args[3]),normalized(path.join(pre.source_directory,'Cargo.toml')));assert.deepEqual(fmt.source_files,allFormatRows);assert.deepEqual(fmt.changed_files,formatChanges.map(f=>f.path));for(const side of ['stdout','stderr'])dataLog(fmt[side]);assert.equal(fmt.root_repo_mutated,false);
 const formatter=data(a.formatter_review);noFindings(formatter);sameRef(formatter.preformat_manifest,a.preformat_manifest);sameRef(formatter.actual_format_process_receipt,a.actual_format);sameRef(formatter.composition_review,a.composition_review);assert.deepEqual(formatter.files,changedFormatRows);assert.deepEqual(formatter.formatted_files,a.source_files);
 const clippy=data(a.preliminary_clippy);assert.equal(clippy.kind,'actual_source12_preliminary_clippy');assert.equal(clippy.status,'PASS_ACTUAL_PRELIMINARY_CLIPPY');sameRef(clippy.format,a.actual_format);assert.equal(normalized(clippy.source_directory),normalized(pre.source_directory));assert.equal(normalized(clippy.command.cwd),normalized(pre.source_directory));assert.equal(normalized(clippy.command.executable),normalized('C:/Users/matth/.cargo/bin/cargo.exe'));assert.deepEqual(clippy.command.args,['clippy','--workspace','--all-targets','--locked','--','-D','warnings']);assert.deepEqual(clippy.profile,profile.environment);const cp=clippy.actual_process;assert(Number.isSafeInteger(cp.pid)&&cp.pid>0);assert.equal(cp.spawned,true);assert.deepEqual(cp.exit,{code:0,signal:null});assert.deepEqual(cp.close,cp.exit);assert.equal(cp.error,null);assert.equal(cp.timed_out,false);assert.equal(clippy.executed_tests,0);assert.equal(clippy.source_final_verified,true);assert.deepEqual(clippy.source_files,a.source_files);assert.equal(clippy.root_repo_mutated,false);for(const side of ['stdout','stderr']){const log=clippy.logs[side];assert.equal(log.end,true);assert.equal(log.error,null);assert.equal(normalized(log.path),normalized(log.reference.path));const raw=dataLog(log.reference);assert.equal(raw.length,log.bytes);assert(!/^error(?:\[|:)/m.test(raw.toString()));}
 function dataLog(r){const raw=pin(r.path,r.sha256);assert.equal(raw.length,r.bytes);return raw;}
 const preparation=data(a.preparation_manifest),preparationReview=data(a.preparation_review);noFindings(preparationReview);sameRef(preparationReview.proposal_manifest,a.preparation_manifest);assert.equal(preparation.kind,'combined_source_v12_direct_parent_driver_preparation');sameRoles(preparation.source_proposals,ROLE_PINS);assert.equal(preparation.files.length,4);const self=fileURLToPath(import.meta.url),selfRaw=pin(self,sha(fs.readFileSync(self)));assert.equal(normalized(self),normalized(path.join(TASK,'work/validate-combined-source-v12.mjs')));const producer=preparation.files.find(f=>f.path==='validate-combined-source-v12.mjs');assert(producer);assert.equal(producer.sha256,sha(selfRaw));assert.equal(producer.bytes,selfRaw.length);for(const f of preparation.files){relativeFile(f.path);const raw=pin(path.join(path.dirname(a.preparation_manifest.path),f.path),f.sha256);assert.equal(raw.length,f.bytes);}sameRef(preparationReview.binder,preparation.binder);dataLog(preparation.binder);
 const publication=data(a.private_publication);assert.equal(publication.kind,'actual_private_source12_code_publication');assert.equal(publication.source_commit,a.source_commit);assert.equal(publication.parent_code_commit,baseline.source_commit);assert.equal(publication.branch,a.source_branch);assert.equal(publication.repository,'lia-xim/open-seo-spider');assert.equal(publication.is_private,true);assert.equal(publication.source_inputs,counts.inputs);assert.equal(publication['all'+counts.inputs+'_root_source_hashes_verified'],true);assert.equal(publication['all'+counts.inputs+'_commit_blob_hashes_verified'],true);assert.equal(publication.remote_exact,true);assert.equal(publication.code_and_owned_raw_byte_attributes,true);assert.deepEqual(publication.owned_committed_paths,[...owned.keys()].sort(sort));assert.deepEqual(publication.additional_tracked_files,[]);sameRef(publication.composition,a.preformat_manifest);sameRef(publication.composition_review,a.composition_review);sameRef(publication.actual_format,a.actual_format);sameRef(publication.formatter_review,a.formatter_review);sameRef(publication.preliminary_clippy,a.preliminary_clippy);
 for(const f of final.values())for(const directory of[sourceDirectory,repo]){const raw=pin(path.join(directory,f.path),f.sha256);assert.equal(raw.length,f.bytes);}
 return{baselinePath,baselineSha:PARENT_SHA,baseline,parentFiles,parentRaw,parentSuiteVerified,parentSuiteReference,parentBuildReference,rootApproval:a,rootApprovalReference,sourceFiles:final,requiredTests,required,expectedNewFullNames:full,newFullNames:newFunctions.map(t=>t.full_name),newFunctions,newPaths:[...owned.values()].filter(f=>f.before_sha256===null).map(f=>f.path).sort(sort),counts,minimum,bindings,nodeCases,references,preparation};
}
function manifestFrom(c,repo){const validatorPath=fileURLToPath(import.meta.url);return{schema_version:1,kind:'combined_source_v12',result:'FROZEN_REVIEWED_SOURCE_FULL_SUITE_NOT_RUN',root_approval:c.rootApprovalReference,parent_source_manifest_sha256:PARENT_SHA,parent_source_inputs:369,source_inputs:c.counts.inputs,inherited_required_test_names:602,new_input_files:c.counts.new,changed_parent_inputs:c.counts.modified,unchanged_parent_inputs:c.counts.unchanged,minimum_expected_workspace_tests:c.minimum,required_test_names:c.required.length,new_authored_test_functions:c.newFunctions.length,test_profile:profile,files:[...c.sourceFiles.values()],required_tests:c.requiredTests,new_authored_tests:c.newFunctions,all_required_full_test_names:c.expectedNewFullNames,new_authored_full_test_names:c.newFullNames,source_proposals:ROLE_PINS,parent_suite:c.parentSuiteReference,parent_build:c.parentBuildReference,source_commit:c.rootApproval.source_commit,source_branch:c.rootApproval.source_branch,source_worktree:path.join(BASE,'mcp-file-tools-v1'),approved_product_bindings:c.bindings,additional_non_cargo_cases:c.nodeCases,validator:{path:validatorPath,sha256:sha(fs.readFileSync(validatorPath))},references:[...c.references.values()]};}
export function createManifest({approvalPath,expectedApprovalSha,sourceDirectory,repo=path.join(BASE,'mcp-file-tools-v1')}){const c=loadApproval({approvalPath,expectedApprovalSha,sourceDirectory,repo});return manifestFrom(c,repo);}
export function validateSource12Manifest({manifest,sourceDirectory,repo=path.join(BASE,'mcp-file-tools-v1')}){const c=loadApproval({approvalPath:manifest.root_approval.path,expectedApprovalSha:manifest.root_approval.sha256,sourceDirectory,repo});assert.deepEqual(manifest,manifestFrom(c,repo));const files=c.sourceFiles;const lineage=Object.freeze({parent_source_manifest_sha256:PARENT_SHA,parent_suite_proof_sha256:PARENT_SUITE_SHA,parent_build_proof_sha256:PARENT_BUILD_SHA,source_proposals:ROLE_PINS,root_approval_sha256:manifest.root_approval.sha256,parent_inputs:369,required_parent_names:602,unchanged_parent_inputs:c.counts.unchanged,changed_parent_inputs:c.counts.modified,new_input_files:c.counts.new,delta_bindings:c.counts.owned,new_named_windows_functions:c.newFullNames,actual_required_names:c.required.length,actual_source_inputs:c.counts.inputs,minimum_expected_workspace_tests:c.minimum});return{...c,files,rendererFiles:[...files.values()].filter(f=>f.path.startsWith('renderer/')),lineage,pinnedReferences:c.references};}
