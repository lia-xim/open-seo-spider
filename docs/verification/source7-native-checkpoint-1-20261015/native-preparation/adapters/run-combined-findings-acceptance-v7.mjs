import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
assert(Number(process.versions.node.split('.')[0])>=22);
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed','Root must release this exact combined gate');
assert(!process.env.NODE_OPTIONS&&process.execArgv.length===0,'No external Node preloads/options');
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const task='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https',root='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const repo=path.join(root,'mcp-file-tools-v1'),source=path.join(root,'combined-source-v7');
const handoff=path.join(root,'s6-native-acceptance-preparation-v1'),combinedHandoff=path.join(root,'source7-native-preparation-v2/adapters');
const runtime=path.join(task,'outputs/open-seo-spider');
const self=fileURLToPath(import.meta.url),selfSha=sha(fs.readFileSync(self));
assert.equal(path.resolve(self),path.join(task,'work/run-combined-findings-acceptance-v7.mjs'),'Promote exact new Task/work name');
assert.equal(process.argv.length,8,'Expected --manifest-sha ROOT64 --gate native|ui-mock --release ABSJSON');
const args={};for(let i=2;i<process.argv.length;i+=2){const name=process.argv[i];assert(['--manifest-sha','--gate','--release'].includes(name)&&!(name in args));args[name]=process.argv[i+1];}
const expectedSourceSha=args['--manifest-sha'];assert.equal(expectedSourceSha,'fd587ba252595d971e7557e5cca7f5ab90219ecda200a3f07fcd5c10c13b6397');assert.match(expectedSourceSha,/^[a-f0-9]{64}$/);
const gate=args['--gate'],mode='optimized';assert(['native','ui-mock'].includes(gate));assert(path.isAbsolute(args['--release']));
const out=path.join(root,'results/combined-findings-'+gate+'-optimized-acceptance-v7');assert(!fs.existsSync(out),'Fresh result output required');
const releaseStat=fs.statSync(args['--release']);assert(releaseStat.isFile()&&releaseStat.size<=65536);
const releaseRaw=fs.readFileSync(args['--release']);assert.equal(releaseRaw.length,releaseStat.size);const release=JSON.parse(releaseRaw);
assert.equal(release.kind,'root_released_combined_findings_acceptance_v7');assert.equal(release.gate,gate);assert.equal(release.build_mode,mode);
assert.equal(release.source_manifest_sha256,expectedSourceSha);assert.equal(release.driver_sha256,selfSha);assert.equal(path.resolve(release.output),path.resolve(out));
assert(Number.isFinite(Date.parse(release.expires_at))&&Date.parse(release.expires_at)>Date.now(),'Operational release expired');
for(const field of ['source_manifest_sha256','suite_proof_sha256','build_proof_sha256','candidate_binary_sha256','build_driver_sha256'])assert.match(release[field],/^[a-f0-9]{64}$/);
const bindingsPath=path.join(root,'combined-v7-runner-preparation-v2/combined-bindings-v7.mjs'),bindingsSha='c2bf39d4e6c177374a1f9331f1bcb06b04993bb1000777dff80319532b5e9978';
assert.equal(release.bindings_sha256,bindingsSha);assert.equal(sha(fs.readFileSync(bindingsPath)),bindingsSha);
// The imported binding module only reads/hashes local prerequisites. All native
// or browser adapters remain child-only, after the completed prerequisite block.
const {bindCombined}=await import(pathToFileURL(bindingsPath).href);const binding=bindCombined(release,expectedSourceSha);
const {manifest,sourceRaw,sourceLineage,publicationLineage,suitePath,suiteRaw,suite,actualWorkspace,buildRoot,buildRaw,build,binary,binarySHA}=binding;
const pinFile = (file, expected) => {const bytes=fs.readFileSync(file); assert.equal(sha(bytes),expected,file); return bytes;};
const adapterPath = path.join(handoff, 'findings-native-owned-adapter.mjs');
const adapterSHA = '30381b2d523fac41ab4a41f5e2517628c5f8c7a43c115c92b5ed28da441bff5c';
const adapterBytes = pinFile(adapterPath, adapterSHA);
const uiAdapterPath = path.join(combinedHandoff,'findings-ui-mock-combined-owned-adapter.cjs');
const uiAdapterSHA = 'a0daa9d26251e3e87177344bcd79164571ac0d968cf21778e6c42c8563707a5e';
const uiAdapterBytes = pinFile(uiAdapterPath,uiAdapterSHA);
const nativeOriginalSHA = '7e7dc7c973eba232f4f8c9a3f6f4ebe34904d6d85d04b81cabefe8b1946b4d3b';
pinFile(path.join(source,'scripts/sitemap-findings-mcp-e2e.mjs'), nativeOriginalSHA);
const packageBytes = pinFile(path.join(repo,'scripts/package.json'),'9df2a89604b02cc8112205fc2cf1824372b38f0a7b03a2cd857d00a12f7331d1');
const lockBytes = pinFile(path.join(repo,'scripts/package-lock.json'),'0cb0fb5e20c84375aaa2c99642d07305cd8acf38b16ad52cbb1f47cf83eb5f7c');
assert.equal(JSON.parse(packageBytes).devDependencies['@modelcontextprotocol/client'],'2.3.1');
assert.equal(JSON.parse(lockBytes).packages['node_modules/@modelcontextprotocol/client'].version,'2.3.1');
const uiManifestPath = path.join(combinedHandoff,'UI-ASSETS.json');
const uiManifestSHA = '631bf3ce6dcb9505ffdc3054c4358f654f4278981d09b4885b73e76b861ac886';
const uiManifest = JSON.parse(pinFile(uiManifestPath,uiManifestSHA));
assert.equal(uiManifest.compiler_source_parent_manifest_sha256,'27a8b74c4cd0c394906f3fe3c0a6a0327569294b25056cf6c6f0aabcffc42788');assert.equal(uiManifest.compiler_source_manifest_sha256,'fd587ba252595d971e7557e5cca7f5ab90219ecda200a3f07fcd5c10c13b6397','Actual Source7 SHA is bound by reviewed binder/current Root release');assert.equal(uiManifest.files.length,6);assert.equal(uiManifest.supported_source_kind,'combined_source_v7');assert.equal(uiManifest.kind,'ui_combined_v7_mock_asset_manifest_v1');
for (const file of uiManifest.files) {
  const compiler = manifest.files.find(row => row.path === file.path);
  assert(compiler); assert.equal(file.after_sha256,compiler.sha256);
  pinFile(path.join(source,file.path),file.after_sha256);
}
function verifySource() {
  assert.equal(sha(fs.readFileSync(self)),selfSha,'Driver changed');assert.equal(sha(fs.readFileSync(bindingsPath)),bindingsSha,'Combined binding module changed');binding.verify();
  pinFile(path.join(repo,'scripts/package.json'),sha(packageBytes));pinFile(path.join(repo,'scripts/package-lock.json'),sha(lockBytes));
  pinFile(binary,binarySHA);pinFile(adapterPath,adapterSHA);pinFile(uiAdapterPath,uiAdapterSHA);pinFile(uiManifestPath,uiManifestSHA);
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
write('prerequisites.json',{source_publication:publicationLineage,source_manifest_sha256:expectedSourceSha,suite_sha256:sha(suiteRaw),build_proof_sha256:sha(buildRaw),binary:{path:binary,sha256:binarySHA},source_inputs:manifest.files.length,workspace:actualWorkspace,release_sha256:sha(releaseRaw),dependencies:dependenciesBefore,installed_before:installedBefore,driver_sha256:sha(fs.readFileSync(self)),native_frozen_harness_sha256:nativeOriginalSHA,native_execution_adapter_sha256:adapterSHA,ui_execution_adapter_sha256:uiAdapterSHA,ui_manifest_sha256:uiManifestSHA,qualification:'UI gate is mocked API only; native directed fixture is small; this is not whole S6 acceptance.'});
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
  fs.writeFileSync(uiRelease,JSON.stringify({kind:'root_released_combined_findings_ui_fixture_v7',source_manifest_sha256:uiManifestSHA,output:uiOutput,expires_at:release.expires_at,compiler_source_manifest_sha256:expectedSourceSha,root_release_sha256:sha(releaseRaw)})+'\n',{flag:'wx'});
  env.SPIDER_CHROME=release.chrome_executable;env.SPIDER_PLAYWRIGHT_MODULE=playwrightRoot;
  const isolatedUI=path.join(out,'ui-execution-adapter.cjs');fs.writeFileSync(isolatedUI,uiAdapterBytes,{flag:'wx'});
  commandArgs=[isolatedUI,'--source-root',source,'--runtime-root',runtime,'--manifest',uiManifestPath,'--manifest-sha256',uiManifestSHA,'--compiler-manifest-sha256',expectedSourceSha,'--release',uiRelease,'--out',uiOutput];
}
const runs=[],failures=[],ownership=[],owners=new Map();
const HARNESS_DEADLINE_MS=600000, HARNESS_DEADLINE_NS=BigInt(HARNESS_DEADLINE_MS)*1000000n;
let child,actualClose=false,spawnError=null,deadlineExceeded=false,externalCancellation=false,completionClock=null;
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
    const childStartedNs=process.hrtime.bigint();
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
    timer=setTimeout(()=>cooperativeAbort('deadline'),HARNESS_DEADLINE_MS);timer.unref();
    const closed=await new Promise(resolve=>child.once('close',(code,signal)=>{
      actualClose=true;
      resolve(Object.freeze({code,signal,elapsed_nanoseconds:(process.hrtime.bigint()-childStartedNs).toString()}));
    }));
    exit={code:closed.code,signal:closed.signal};
    // Immutable monotonic actual-close observation: a due timer may not have
    // run yet, and clearing it cannot turn an over600s completion into PASS.
    completionClock=Object.freeze({elapsed_nanoseconds:closed.elapsed_nanoseconds,deadline_nanoseconds:HARNESS_DEADLINE_NS.toString(),within_deadline:BigInt(closed.elapsed_nanoseconds)<HARNESS_DEADLINE_NS});
  } finally {clearTimeout(timer);fs.closeSync(stdoutFD);fs.closeSync(stderrFD);}
  const stdout=fs.readFileSync(stdoutPath),stderr=fs.readFileSync(stderrPath);
  runs.push({name:gate,node:process.execPath,args:commandArgs,pid:child?.pid??null,actual_close_event:actualClose,spawn_error:spawnError,exit,stdout_sha256:sha(stdout),stderr_sha256:sha(stderr),deadline_exceeded:deadlineExceeded||completionClock?.within_deadline===false,actual_close_clock:completionClock,external_cancellation:externalCancellation});
  assert(actualClose);assert.equal(spawnError,null);assert.deepEqual(exit,{code:0,signal:null});
  assert(completionClock?.within_deadline===true,'Actual close observation exceeded monotonic600s deadline');
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
  write('proof.json',{schema_version:1,result:pass?(gate==='native'?'PASS_DIRECTED_NATIVE_SDK_API_CLI_ONLY':'PASS_13_UI_MOCK_GROUPS_ONLY'):'FAIL_PREPARED_ACCEPTANCE',recorded_at:new Date().toISOString(),gate,mode,source_publication:publicationLineage,source_lineage:sourceLineage,actual_close_clock:completionClock,source_manifest_sha256:expectedSourceSha,source_inputs:manifest.files.length,build_proof_sha256:sha(buildRaw),binary:{path:binary,sha256:binarySHA},frozen_native_harness_sha256:nativeOriginalSHA,derived_execution_adapter_sha256:adapterSHA,ui_frozen_harness_sha256:'889275f564d254512348a0196ee04b84317d39a45b7d0a2d4ffb3c5067eba8be',ui_execution_adapter_sha256:uiAdapterSHA,ui_manifest_sha256:uiManifestSHA,dependencies:dependenciesBefore,runs,ownership,failures,installed_binary_preserved:!failures.some(f=>f.phase==='final source/dependency/binary identity'),qualification:'Native SDK/API/CLI small5-page fixture and13 mocked-API UI groups are separate results. Native browser actual Core/API/UI journey, threshold/large-window matrix, physical8MiB/64MiB boundaries, cancellation latency, RSS/recovery/selected/platform/install remain OPEN. No fullS6/manufacturer parity or hard deadline drain guarantee. Every artifact and private fixture is retained; no cleanup of prior/user/shared data.'});
  console.log(JSON.stringify({result:pass?'PASS_SCOPED_ONLY':'FAIL',gate,out,failures:failures.map(f=>f.message)}));
}
