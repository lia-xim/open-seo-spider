import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';

// AUTHORED / NOT_RUN. No optional imports or effects before the operational guard.
assert(Number(process.versions.node.split('.')[0])>=22,'Node22+ required by pinned renderer package');
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed', 'Compute HOLD: Root must release this exact gate');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const task = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const root = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const repo = path.join(root, 'sitemap-findings-surfaces-integration-v1');
const source = path.join(root, 'sitemap-findings-surfaces-source-v1');
const handoff = path.join(root, 's6-native-acceptance-preparation-v1');
const runtime = path.join(task, 'outputs/open-seo-spider');
const expectedSourceSha = '69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751';
const self = fileURLToPath(import.meta.url);
assert.equal(path.resolve(path.dirname(self), '..'), path.resolve(task), 'Promote only to pinned Task/work before execution');
assert.equal(process.argv.length, 8, 'Expected --gate native|ui-mock --build-mode debug|optimized --release ABSJSON');
const args = {};
for (let i = 2; i < process.argv.length; i += 2) {
  const name = process.argv[i];
  assert(['--gate', '--build-mode', '--release'].includes(name) && !(name in args), 'Unknown/duplicate argument');
  args[name] = process.argv[i + 1];
}
const gate = args['--gate'], mode = args['--build-mode'];
assert(['native', 'ui-mock'].includes(gate)); assert(['debug', 'optimized'].includes(mode));
assert(path.isAbsolute(args['--release']));
const out = path.join(root, `results/sitemap-findings-${gate}-${mode}-acceptance-v1`);
assert(!fs.existsSync(out), 'Fresh evidence output required; preserve prior attempts');
const releaseRaw = fs.readFileSync(args['--release']);
assert(releaseRaw.length <= 65536, 'Bounded release receipt');
const release = JSON.parse(releaseRaw);
assert.equal(release.kind, 'root_released_sitemap_findings_acceptance_driver_v1');
assert.equal(release.gate, gate); assert.equal(release.build_mode, mode);
assert.equal(release.source_manifest_sha256, expectedSourceSha);
assert.equal(release.driver_sha256, sha(fs.readFileSync(self)));
assert.equal(path.resolve(release.output), path.resolve(out));
assert(Number.isFinite(Date.parse(release.expires_at)) && Date.parse(release.expires_at) > Date.now(), 'Operational release expired');
assert(!process.env.NODE_OPTIONS && process.execArgv.length===0,'No external Node preload/options accepted');
const sourceRaw = fs.readFileSync(path.join(source, 'manifest.json'));
assert.equal(sha(sourceRaw), expectedSourceSha);
const manifest = JSON.parse(sourceRaw);
assert.equal(manifest.files.length, 297); assert.equal(manifest.required_test_names, 388);
assert.equal(manifest.required_tests.reduce((n,x) => n+x.names.length, 0), 388);
assert.equal(manifest.minimum_expected_workspace_tests, 1170);
const ownedRendererGates = ['live_browser_worker_executes_javascript_for_core_crawl','live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness','live_browser_worker_pause_drains_render_and_retains_rendered_discoveries','live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion'];
const suitePath = path.join(root, 'results/sitemap-findings-surfaces-suite-v1/proof.json');
const suiteRaw = fs.readFileSync(suitePath), suite = JSON.parse(suiteRaw);
assert.equal(suite.result, 'PASS_LOCAL_CURRENT_WORKSPACE_REAL_RENDERER_CLIPPY_FMT');
assert.equal(suite.source_manifest_sha256, expectedSourceSha);
assert.equal(suite.files, 297); assert.equal(suite.required_test_names, 388);
assert.equal(suite.owned_worker_closed, true); assert.equal(suite.owned_renderer_gates_verified, true);
assert.equal(suite.owned_renderer_gate_count, 4); assert.deepEqual(suite.owned_renderer_gates, ownedRendererGates);
// Verify the actual three producer runs and captured logs, not a summary-only
// PASS string. All admission checks finish before any output/backup/child write.
assert(Array.isArray(suite.failures) && suite.failures.length === 0, 'Workspace proof must have no failures');
const expectedSuiteRuns = [
  {name: 'format', args: ['fmt', '--all', '--check']},
  {name: 'workspace-live', args: ['test', '--workspace', '--locked', '--', '--include-ignored', '--test-threads=1']},
  {name: 'clippy', args: ['clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings']},
];
assert(Array.isArray(suite.runs), 'Actual workspace run records required');
assert.deepEqual(suite.runs.map(run => run.name), expectedSuiteRuns.map(run => run.name), 'Exactly format/workspace-live/clippy in producer order required');
const verifiedSuiteRuns = [];
let actualWorkspace = null;
for (const [index, expected] of expectedSuiteRuns.entries()) {
  const run = suite.runs[index];
  assert.deepEqual(run.args, expected.args, expected.name + ' exact command');
  assert.equal(run.actual_close_event, true, expected.name + ' actual child close');
  assert.equal(run.spawn_error, null, expected.name + ' spawn error');
  assert.deepEqual(run.exit, {code: 0, signal: null}, expected.name + ' successful close');
  assert.equal(typeof run.log_sha256, 'string', expected.name + ' log SHA type');
  assert.match(run.log_sha256, /^[0-9a-f]{64}$/, expected.name + ' log SHA profile');
  // Fixed names from the expected command set, never untrusted path components.
  const logPath = path.join(path.dirname(suitePath), expected.name + '.log');
  const logBytes = fs.readFileSync(logPath);
  assert.equal(sha(logBytes), run.log_sha256, expected.name + ' actual log SHA');
  const logText = logBytes.toString('utf8');
  const summaries = [...logText.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)]
    .map(match => ({state: match[1], passed: +match[2], failed: +match[3], ignored: +match[4]}));
  assert(Array.isArray(run.summaries), expected.name + ' actual summary rows required');
  assert.deepEqual(run.summaries, summaries, expected.name + ' summary rows must match its actual log');
  for (const summary of summaries) {
    assert.equal(summary.state, 'ok', expected.name + ' summary state');
    for (const key of ['passed', 'failed', 'ignored']) assert(Number.isSafeInteger(summary[key]) && summary[key] >= 0, expected.name + ' bounded summary count');
    assert.equal(summary.failed, 0, expected.name + ' failed tests');
    assert.equal(summary.ignored, 0, expected.name + ' ignored tests');
  }
  if (expected.name === 'workspace-live') {
    assert(summaries.length > 0, 'Actual workspace test summaries required');
    const total = summaries.reduce((sum, row) => ({passed: sum.passed + row.passed, failed: sum.failed + row.failed, ignored: sum.ignored + row.ignored}), {passed: 0, failed: 0, ignored: 0});
    for (const value of Object.values(total)) assert(Number.isSafeInteger(value), 'Actual workspace aggregate must be exact');
    assert(total.passed >= 1170 && total.failed === 0 && total.ignored === 0, 'Actual workspace minimum/failed/ignored');
    assert.equal(suite.passed, total.passed, 'Proof passed total must equal actual workspace log');
    assert.equal(suite.ignored, total.ignored, 'Proof ignored total must equal actual workspace log');
    for (const group of manifest.required_tests) for (const name of group.names) assert(logText.includes('::' + name + ' ... ok'), 'Required actual source case not green: ' + group.path + '::' + name);
    for (const name of ownedRendererGates) assert(logText.includes('test ' + name + ' ... ok'), 'Required actual owned browser gate not green: ' + name);
    actualWorkspace = {...total, summary_rows: summaries.length, required_source_names: 388, required_renderer_gates: 4};
  }
  verifiedSuiteRuns.push({name: expected.name, args: expected.args, log_path: logPath, log_sha256: sha(logBytes), actual_close_event: run.actual_close_event, spawn_error: run.spawn_error, exit: run.exit, summaries});
}
assert(actualWorkspace !== null, 'Actual workspace run not verified');

assert.equal(sha(fs.readFileSync(path.join(task,'work/run-sitemap-findings-surfaces-suite-v1.mjs'))),'5de37908b795947c8db1cab459f1a45a2edda823499747b162d13a6bef3fcc66');
assert.equal(sha(fs.readFileSync(path.join(task,'work/build-sitemap-findings-surfaces-v2.mjs'))),'5478157ccc2e3511285a175e0b559168dc66f268bc79f45c631f98609be137d6');
const buildRoot = path.join(root, `results/sitemap-findings-surfaces-${mode}-build-v2`);
const buildRaw = fs.readFileSync(path.join(buildRoot, 'proof.json')), build = JSON.parse(buildRaw);
assert.equal(sha(buildRaw), release.build_proof_sha256);
assert.equal(build.result, 'PASS_LOCAL_CURRENT_' + mode.toUpperCase() + '_BUILD');
assert.equal(build.mode, mode); assert.equal(build.source_manifest_sha256, expectedSourceSha);
assert.equal(build.input_files, 297); assert.equal(build.source_and_checkout_identity_verified, true);
assert(Array.isArray(build.failures) && build.failures.length === 0);
assert.equal(build.required_workspace.path, suitePath); assert.equal(build.required_workspace.sha256, sha(suiteRaw));
assert.deepEqual(build.required_workspace.verified_runs, verifiedSuiteRuns);
assert.deepEqual(build.required_workspace.actual_workspace, actualWorkspace);
const expectedBuildRuns = [
  {name:'cargo-version', args:['--version','--verbose']},
  {name:'rustc-version', args:['--version','--verbose']},
  {name:'build', args:['build','--locked','-p','open-seo-spider', ...(mode === 'optimized' ? ['--release'] : [])]},
];
assert.deepEqual(build.runs.map(run => run.name), expectedBuildRuns.map(run => run.name));
for (const [i, expected] of expectedBuildRuns.entries()) {
  const run = build.runs[i];
  assert.deepEqual(run.args, expected.args); assert.equal(run.actual_close_event, true);
  assert.equal(run.spawn_error, null); assert.deepEqual(run.exit, {code:0,signal:null});
  assert.equal(sha(fs.readFileSync(path.join(buildRoot, expected.name + '.log'))), run.log_sha256);
}
const binary = path.join(buildRoot, 'open-seo-spider.exe');
assert.equal(path.resolve(build.binary.path), path.resolve(binary));
const binaryBytes = fs.readFileSync(binary), binarySHA = sha(binaryBytes);
assert.equal(binarySHA, build.binary.sha256); assert.equal(binarySHA, release.candidate_binary_sha256);
assert.equal(binaryBytes.length, build.binary.bytes);
assert(binaryBytes.length > 0 && binaryBytes[0] === 0x4d && binaryBytes[1] === 0x5a);
const pinFile = (file, expected) => {const bytes=fs.readFileSync(file); assert.equal(sha(bytes),expected,file); return bytes;};
const adapterPath = path.join(handoff, 'findings-native-owned-adapter.mjs');
const adapterSHA = '30381b2d523fac41ab4a41f5e2517628c5f8c7a43c115c92b5ed28da441bff5c';
const adapterBytes = pinFile(adapterPath, adapterSHA);
const uiAdapterPath = path.join(handoff,'findings-ui-mock-owned-adapter.cjs');
const uiAdapterSHA = 'dd51478d6be132a29d2c7a19317a9ffa02e207094c3afa10b82e273bf97f4a4d';
const uiAdapterBytes = pinFile(uiAdapterPath,uiAdapterSHA);
const nativeOriginalSHA = '7e7dc7c973eba232f4f8c9a3f6f4ebe34904d6d85d04b81cabefe8b1946b4d3b';
pinFile(path.join(source,'scripts/sitemap-findings-mcp-e2e.mjs'), nativeOriginalSHA);
const packageBytes = pinFile(path.join(repo,'scripts/package.json'),'9df2a89604b02cc8112205fc2cf1824372b38f0a7b03a2cd857d00a12f7331d1');
const lockBytes = pinFile(path.join(repo,'scripts/package-lock.json'),'0cb0fb5e20c84375aaa2c99642d07305cd8acf38b16ad52cbb1f47cf83eb5f7c');
assert.equal(JSON.parse(packageBytes).devDependencies['@modelcontextprotocol/client'],'2.3.1');
assert.equal(JSON.parse(lockBytes).packages['node_modules/@modelcontextprotocol/client'].version,'2.3.1');
const uiManifestPath = path.join(root,'s6-ui-surface-core-v1/manifest.json');
const uiManifestSHA = 'c9e4cb2821182d8a357ce8f8082668a6892d96cf7929e959187e31321c0ee707';
const uiManifest = JSON.parse(pinFile(uiManifestPath,uiManifestSHA));
assert.equal(uiManifest.files.length,6);
for (const file of uiManifest.files) {
  const compiler = manifest.files.find(row => row.path === file.path);
  assert(compiler); assert.equal(file.after_sha256,compiler.sha256);
  pinFile(path.join(source,file.path),file.after_sha256);
}
function verifySource() {
  assert.equal(sha(fs.readFileSync(path.join(source,'manifest.json'))),expectedSourceSha);
  for (const file of manifest.files) for (const base of [source,repo]) {
    const bytes=fs.readFileSync(path.join(base,file.path));
    assert.equal(bytes.length,file.bytes,file.path); assert.equal(sha(bytes),file.sha256,file.path);
  }
  pinFile(path.join(repo,'scripts/package.json'),sha(packageBytes));
  pinFile(path.join(repo,'scripts/package-lock.json'),sha(lockBytes));
  pinFile(binary,binarySHA); pinFile(adapterPath,adapterSHA); pinFile(uiAdapterPath,uiAdapterSHA);
}
function treeIdentity(base) {
  const rows=[];
  function visit(relative) {
    const current=path.join(base,relative),stat=fs.lstatSync(current);
    assert(!stat.isSymbolicLink(),'Dependency links not accepted inside pinned tree');
    if (stat.isDirectory()) for (const name of fs.readdirSync(current).sort()) visit(path.join(relative,name));
    else {assert(stat.isFile()); const bytes=fs.readFileSync(current); rows.push({path:relative.replaceAll('\\','/'),bytes:bytes.length,sha256:sha(bytes)});}
  }
  visit(''); return {root:base,files:rows.length,sha256:sha(Buffer.from(JSON.stringify(rows)))};
}
const sdkRoot=path.join(runtime,'scripts/node_modules');
const playwrightRoot=path.join(runtime,'renderer/node_modules/playwright-core');
const sdkPackage=JSON.parse(pinFile(path.join(sdkRoot,'@modelcontextprotocol/client/package.json'),'4a886cbee0d611f822618e4ee94ae4589655c282fa0dbafc17e80da9697f2d91'));
assert.equal(sdkPackage.version,'2.3.1');
pinFile(path.join(sdkRoot,'@modelcontextprotocol/client/dist/stdio.mjs'),'af887c8972a37b8c3fcc78192d8f1d56c099bf8ca365f4843ec51ced6f5650da');
const playwrightPackage=JSON.parse(pinFile(path.join(playwrightRoot,'package.json'),'1d6b6f80ffb0c52865af0f9fe01abf2140df019e49ae6ac134605e462d583f3a'));
assert.equal(playwrightPackage.version,'1.64.0');
const dependenciesBefore=treeIdentity(gate==='native'?sdkRoot:path.join(runtime,'renderer/node_modules'));
assert.equal(dependenciesBefore.files,gate==='native'?1111:150);
assert.equal(dependenciesBefore.sha256,gate==='native'?'ccc97b0249386b580c556b0f9c66a92618d55605fb795f338418f00e1a7fdbb1':'9bf4b4ff3378e538764bff2580ad0fc8797ee613ecbad538383e7ed442e0ccc3');
const installed=path.join(runtime,'open-seo-spider.exe');
const installedBefore=fs.existsSync(installed)?sha(fs.readFileSync(installed)):null;
let chromeSHA=null;
if (gate==='ui-mock') {
  assert(path.isAbsolute(release.chrome_executable)); assert.match(release.chrome_sha256,/^[0-9a-f]{64}$/);
  chromeSHA=sha(fs.readFileSync(release.chrome_executable)); assert.equal(chromeSHA,release.chrome_sha256);
}
verifySource();
// ALL prerequisite checks above precede output creation, dependency linking,
// optional module imports (only occur inside child harness) or any child spawn.
fs.mkdirSync(out);
const write=(name,value)=>fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2)+'\n',{flag:'wx'});
write('prerequisites.json',{source_manifest_sha256:expectedSourceSha,suite_sha256:sha(suiteRaw),build_proof_sha256:sha(buildRaw),binary:{path:binary,sha256:binarySHA},source_inputs:297,workspace:actualWorkspace,release_sha256:sha(releaseRaw),dependencies:dependenciesBefore,installed_before:installedBefore,driver_sha256:sha(fs.readFileSync(self)),native_frozen_harness_sha256:nativeOriginalSHA,native_execution_adapter_sha256:adapterSHA,ui_execution_adapter_sha256:uiAdapterSHA,ui_manifest_sha256:uiManifestSHA,qualification:'UI gate is mocked API only; native directed fixture is small; this is not whole S6 acceptance.'});
const temporary=path.join(out,'temporary');fs.mkdirSync(temporary);
const env={...process.env,TEMP:temporary,TMP:temporary,NO_PROXY:'localhost,127.0.0.1,::1',no_proxy:'localhost,127.0.0.1,::1'};
delete env.OPEN_SEO_SPIDER_RENDERER;
let commandArgs;
if (gate==='native') {
  const isolated=path.join(out,'native-root'),scripts=path.join(isolated,'scripts');
  fs.mkdirSync(isolated);fs.mkdirSync(scripts);
  fs.writeFileSync(path.join(scripts,'package.json'),packageBytes,{flag:'wx'});
  fs.writeFileSync(path.join(scripts,'package-lock.json'),lockBytes,{flag:'wx'});
  fs.writeFileSync(path.join(scripts,'sitemap-findings-mcp-e2e.mjs'),adapterBytes,{flag:'wx'});
  fs.symlinkSync(sdkRoot,path.join(scripts,'node_modules'),'junction');
  env.SPIDER_COMPUTE_RELEASED='s6-cli-mcp';env.SPIDER_BINARY=binary;env.SPIDER_BINARY_SHA256=binarySHA;
  commandArgs=[path.join(scripts,'sitemap-findings-mcp-e2e.mjs')];
} else {
  const uiOutput=path.join(out,'ui-mock');
  const uiRelease=path.join(out,'ui-release.json');
  fs.writeFileSync(uiRelease,JSON.stringify({kind:'root_released_sitemap_findings_ui_fixture',source_manifest_sha256:uiManifestSHA,output:uiOutput,expires_at:release.expires_at,compiler_source_manifest_sha256:expectedSourceSha,root_release_sha256:sha(releaseRaw)})+'\n',{flag:'wx'});
  env.SPIDER_CHROME=release.chrome_executable;env.SPIDER_PLAYWRIGHT_MODULE=playwrightRoot;
  const isolatedUI=path.join(out,'ui-execution-adapter.cjs');fs.writeFileSync(isolatedUI,uiAdapterBytes,{flag:'wx'});
  commandArgs=[isolatedUI,'--source-root',source,'--runtime-root',runtime,'--manifest',uiManifestPath,'--manifest-sha256',uiManifestSHA,'--release',uiRelease,'--out',uiOutput];
}
const runs=[],failures=[],ownership=[],owners=new Map();
let child,actualClose=false,spawnError=null,deadlineExceeded=false,externalCancellation=false;
const cooperativeAbort=reason=>{
  if (reason==='deadline') deadlineExceeded=true; else externalCancellation=true;
  if (child?.connected) child.send({kind:'findings_native_abort_v1'},()=>{});
  write(reason==='deadline'?'deadline-exceeded.json':'external-cancellation.json',{reason,pid:child?.pid??null,at:new Date().toISOString(),owned_processes:[...owners.values()],note:'Cooperative cancellation requested; retain all evidence. No nested PID kill or inferred close. Await actual harness close; no hard drain-latency guarantee.'});
};
const onSIGINT=()=>{if(!externalCancellation)cooperativeAbort('external');},onSIGTERM=onSIGINT;
process.once('SIGINT',onSIGINT);process.once('SIGTERM',onSIGTERM);
try {
  verifySource();assert(Date.parse(release.expires_at)>Date.now(),'Release expired before child');
  const stdoutPath=path.join(out,'harness.stdout.log'),stderrPath=path.join(out,'harness.stderr.log');
  const stdoutFD=fs.openSync(stdoutPath,'wx'),stderrFD=fs.openSync(stderrPath,'wx');
  let exit,timer;
  try {
    child=spawn(process.execPath,commandArgs,{cwd:out,env,windowsHide:true,stdio:['ignore',stdoutFD,stderrFD,'ipc']});
    child.once('error',error=>{spawnError=error.message;});
    child.on('message',event=>{
      try {
        assert(event?.kind==='findings_native_ownership_v1' && gate==='native');
        const raw=JSON.stringify(event);assert(Buffer.byteLength(raw)<=16384,'Bounded ownership event');
        ownership.push(event);fs.appendFileSync(path.join(out,'ownership.ndjson'),raw+'\n');
        if (event.event==='spawn') {
          assert(/^(app|cli|sdk-stdio-\d+)$/.test(event.role));
          assert(event.pid===null||(Number.isSafeInteger(event.pid)&&event.pid>0));
          assert(!owners.has(event.role),'Duplicate ownership role');owners.set(event.role,{role:event.role,pid:event.pid,actual_close_event:false});
        } else if (event.event==='close') {
          assert.equal(event.actual_close_event,true);
          const owned=owners.get(event.role);assert(owned,'Close without tracked spawn');assert.equal(owned.pid,event.pid);
          assert.equal(owned.actual_close_event,false,'Duplicate close');owned.actual_close_event=true;
        } else assert(['directory','directory_retained','close_error'].includes(event.event));
      } catch(error) {failures.push({phase:'ownership journal',message:error.message});}
    });
    timer=setTimeout(()=>cooperativeAbort('deadline'),600000);timer.unref();
    exit=await new Promise(resolve=>child.once('close',(code,signal)=>{actualClose=true;resolve({code,signal});}));
  } finally {clearTimeout(timer);fs.closeSync(stdoutFD);fs.closeSync(stderrFD);}
  const stdout=fs.readFileSync(stdoutPath),stderr=fs.readFileSync(stderrPath);
  runs.push({name:gate,node:process.execPath,args:commandArgs,pid:child?.pid??null,actual_close_event:actualClose,spawn_error:spawnError,exit,stdout_sha256:sha(stdout),stderr_sha256:sha(stderr),deadline_exceeded:deadlineExceeded,external_cancellation:externalCancellation});
  assert(actualClose);assert.equal(spawnError,null);assert.deepEqual(exit,{code:0,signal:null});
  assert(!deadlineExceeded&&!externalCancellation,'Cancelled/over-deadline run cannot PASS');
  assert(stdout.length<=1024*1024,'Bounded harness receipt');
  if (gate==='native') {
    const receipt=JSON.parse(stdout);
    assert.equal(receipt.result,'PASS');assert.equal(receipt.binary_sha256,binarySHA);
    assert.equal(receipt.execution_adapter,'isolated_findings_native_ownership_v1');
    assert.equal(receipt.website_requests_during_read,0);assert.equal(receipt.checks.length,3);
    assert(receipt.checks[0].startsWith('stdio:'));assert(receipt.checks[1].startsWith('streamable_http:'));assert(receipt.checks[2].startsWith('actual CLI'));
    assert.equal(owners.size,3);assert.deepEqual([...owners.keys()].sort(),['app','cli','sdk-stdio-0']);
    for (const owned of owners.values()) {assert(Number.isSafeInteger(owned.pid)&&owned.pid>0);assert.equal(owned.actual_close_event,true);}
    assert.deepEqual(receipt.owned_processes.slice().sort((a,b)=>a.role.localeCompare(b.role)),[...owners.values()].sort((a,b)=>a.role.localeCompare(b.role)));
    for(const port of Object.values(receipt.owned_ports)) assert(Number.isInteger(port)&&port>0&&![8787,8790].includes(port));
    assert(path.resolve(receipt.fixture_directory_retained).startsWith(path.resolve(temporary)+path.sep));
    write('native-receipt.json',receipt);
  } else {
    const receipt=JSON.parse(fs.readFileSync(path.join(out,'ui-mock/proof.json')));
    assert.equal(receipt.status,'PASS_UI_FIXTURE_ONLY');assert.equal(receipt.native_findings_acceptance,'NOT_RUN');
    assert.deepEqual(receipt.groups.map(group=>group.name),["01 seven filters and tri-state server selections","02 sparse zero-row continuation and independent back","03 source precision integrity retention association and unknown history","04 original literal case spaces and long POST selection","05 full matching server CSV uses fresh export role and rejects partial or oversized files","06 late old page bodies and finally never repaint or unlock new query job or view","07 late downloads after edit or explicit cancellation create no file","08 unsupported envelopes and explicit HTTP errors fail closed","09 unavailable DB and Core analysis never become empty success","10 markup formula and large original evidence remain inert","11 keyboard row activation close and focus return","12 narrow and wide controls details bottom and footer reachable","13 fixture wire scope and no website or crawl mutation"]);
    assert.deepEqual(receipt.source_pins,uiManifest.files.map(file=>({file:file.path,sha256:file.after_sha256})));
    assert.equal(receipt.groups.length,13);assert(receipt.groups.every(group=>group.result==='PASS_UI_FIXTURE_ONLY'));
    assert.equal(new Set(receipt.groups.map(group=>group.name)).size,13);
    assert.equal(receipt.website_reads,0);assert.deepEqual(receipt.external,[]);assert.deepEqual(receipt.page_errors,[]);
    write('ui-mock-receipt.json',receipt);
  }
} catch(error) {failures.push({phase:'acceptance',message:error.message,stack:error.stack});process.exitCode=1;}
finally {
  process.removeListener('SIGINT',onSIGINT);process.removeListener('SIGTERM',onSIGTERM);
  try {
    verifySource();assert.deepEqual(treeIdentity(dependenciesBefore.root),dependenciesBefore);
    assert.equal(fs.existsSync(installed)?sha(fs.readFileSync(installed)):null,installedBefore,'Installed user binary changed');
    if(chromeSHA!==null)assert.equal(sha(fs.readFileSync(release.chrome_executable)),chromeSHA);
  } catch(error) {failures.push({phase:'final source/dependency/binary identity',message:error.message});process.exitCode=1;}
  const pass=failures.length===0&&actualClose;
  write('proof.json',{schema_version:1,result:pass?(gate==='native'?'PASS_DIRECTED_NATIVE_SDK_API_CLI_ONLY':'PASS_13_UI_MOCK_GROUPS_ONLY'):'FAIL_PREPARED_ACCEPTANCE',recorded_at:new Date().toISOString(),gate,mode,source_manifest_sha256:expectedSourceSha,source_inputs:297,build_proof_sha256:sha(buildRaw),binary:{path:binary,sha256:binarySHA},frozen_native_harness_sha256:nativeOriginalSHA,derived_execution_adapter_sha256:adapterSHA,ui_frozen_harness_sha256:'889275f564d254512348a0196ee04b84317d39a45b7d0a2d4ffb3c5067eba8be',ui_execution_adapter_sha256:uiAdapterSHA,ui_manifest_sha256:uiManifestSHA,dependencies:dependenciesBefore,runs,ownership,failures,installed_binary_preserved:!failures.some(f=>f.phase==='final source/dependency/binary identity'),qualification:'Native SDK/API/CLI small5-page fixture and13 mocked-API UI groups are separate results. Native browser actual Core/API/UI journey, threshold/large-window matrix, physical8MiB/64MiB boundaries, cancellation latency, RSS/recovery/selected/platform/install remain OPEN. No fullS6/manufacturer parity or hard deadline drain guarantee. Every artifact and private fixture is retained; no cleanup of prior/user/shared data.'});
  console.log(JSON.stringify({result:pass?'PASS_SCOPED_ONLY':'FAIL',gate,out,failures:failures.map(f=>f.message)}));
}
