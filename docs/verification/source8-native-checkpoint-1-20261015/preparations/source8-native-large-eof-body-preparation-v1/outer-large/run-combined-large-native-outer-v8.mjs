import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { spawn } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

// One direct child only. Inner guardians retain their own deadlines/oracles.
const BASE = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const TASK = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const BINDER_SHA = 'c58f1984c6b6593d384915157afdcdae206acd1bb8af72279a9ee82702e1b7e7';
const work = name => path.join(TASK, 'work', name);
const ownPath = fileURLToPath(import.meta.url);
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const normalized = value => path.resolve(value).replaceAll('\\', '/').toLowerCase();
const samePath = (actual, expected) => assert.equal(normalized(actual), normalized(expected));
function hashFile(file) {
  const fd = fs.openSync(file, 'r'), hash = createHash('sha256'), chunk = Buffer.alloc(65536);
  try { let count; while ((count = fs.readSync(fd, chunk, 0, chunk.length, null)) !== 0) hash.update(chunk.subarray(0, count)); }
  finally { fs.closeSync(fd); }
  return hash.digest('hex');
}
function readPinned(file, expected, max = 2 * 1024 * 1024) {
  const info = fs.lstatSync(file); assert(info.isFile() && !info.isSymbolicLink() && info.size <= max, file);
  const bytes = fs.readFileSync(file); assert.equal(digest(bytes), expected, file); return bytes;
}
async function main() {
  assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed');
  assert.equal(process.platform,'win32');assert.match(process.versions.node,/^24\./);
  assert(!process.env.NODE_OPTIONS&&!process.env.NODE_PATH);assert.equal(process.execArgv.length,0);
  samePath(ownPath,work('run-combined-large-native-outer-v8.mjs'));
  assert.equal(process.argv.length,6,'Use --release-sha ACTUAL64 --preparation-sha ACTUAL64');assert.equal(process.argv[2],'--release-sha');assert.equal(process.argv[4],'--preparation-sha');
  const releaseSha=process.argv[3],preparationSha=process.argv[5];for(const s of [releaseSha,preparationSha])assert.match(s,/^[a-f0-9]{64}$/);
  const preparationPath=path.join(BASE,'source8-native-large-eof-body-preparation-v1/outer-large/manifest.json'),preparation=JSON.parse(readPinned(preparationPath,preparationSha));
  const selfRow=preparation.files.find(f=>f.path==='run-combined-large-native-outer-v8.mjs');assert(selfRow);assert.equal(driverSHA,selfRow.sha256);
  const helperPath=path.join(path.dirname(preparationPath),'large-outer-preflight.mjs'),helperRow=preparation.files.find(f=>f.path==='large-outer-preflight.mjs');assert(helperRow);readPinned(helperPath,helperRow.sha256);
  const grantRow={path:work('v8-large-sitemap-release.json'),sha256:releaseSha},grant=JSON.parse(readPinned(grantRow.path,grantRow.sha256));
  assert.equal(grant.kind,'root_released_combined_large_sitemap_native_v8');assert.equal(grant.scope,'native_combined_s4_genuine_large_sources_v8');assert.equal(grant.status,'RELEASED');assert.equal(grant.build_mode,'optimized');
  assert.equal(grant.outer_driver_sha256,driverSHA);samePath(grant.large_outer_preparation.path,preparationPath);assert.equal(grant.large_outer_preparation.sha256,preparationSha);
  const {preflight}=await import(pathToFileURL(helperPath).href);readPinned(helperPath,helperRow.sha256);
  const admitted=await preflight({proofPins:grant.proof_inputs,preparationPath,preparationSha,largeReviewPin:grant.large_review});
  for(const [key,value]of Object.entries({...admitted.core,driver_sha256:admitted.harnessSha,bindings_sha256:BINDER_SHA,preparation_manifest_sha256:admitted.largeManifestSha}))assert.equal(grant[key],value,key);
  samePath(grant.output,admitted.largeOutput);samePath(grant.lease_writer.path,work('prepare-v8-large-native-release.mjs'));assert.equal(grant.lease_writer.sha256,preparation.files.find(f=>f.path==='prepare-v8-large-native-release.mjs').sha256);readPinned(grant.lease_writer.path,grant.lease_writer.sha256);assert.deepEqual(grant.source_publication,admitted.binding.publicationLineage);
  const SOURCE_SHA=admitted.core.source_manifest_sha256;assert.equal(grant.source_manifest_sha256,SOURCE_SHA);const gate='large-source-v8',harnessPath=admitted.harnessPath,harnessSHA=admitted.harnessSha;
  const issuedPath=admitted.buildPath,ISSUED_SHA=admitted.core.build_proof_sha256,issued={binary:{path:admitted.binding.binary,sha256:admitted.binding.binarySHA,bytes:admitted.binding.build.binary.bytes}};
  const sourcePath=admitted.sourcePath,suitePath=admitted.suitePath,buildPath=admitted.buildPath;
  const checkExpiry=()=>{assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed');const start=Date.parse(grant.not_before),end=Date.parse(grant.expires_at);assert(Number.isFinite(start)&&Number.isFinite(end)&&start<=Date.now()&&Date.now()<end&&end-start<=2*3600_000,'Fresh two-hour Large13 Root lease required');assert.equal(hashFile(grantRow.path),releaseSha);};
  checkExpiry();admitted.verify();
  const args=['--release',grantRow.path,'--release-sha',releaseSha,'--preparation-sha',admitted.largeManifestSha,'--manifest-sha',SOURCE_SHA,'--bindings',admitted.binderPath,'--bindings-sha',BINDER_SHA,'--suite-proof-sha',admitted.core.suite_proof_sha256,'--build-proof-sha',admitted.core.build_proof_sha256,'--build-driver-sha',admitted.core.build_driver_sha256,'--candidate-binary-sha',issued.binary.sha256,'--out',grant.output];
  // All admission above precedes output/lock creation. No harness import/eval.
  const outputRoot = path.join(BASE, 'results/combined-native-outer-v8');
  const output = path.join(outputRoot, gate), lockPath = path.join(BASE,'results/combined-native-outer-v4/active.lock');
  assert(!fs.existsSync(output), 'Gate already has output; no automatic rerun');
  fs.mkdirSync(outputRoot, { recursive: true });
  const lockFd = fs.openSync(lockPath, 'wx');
  const lockBytes = Buffer.from(JSON.stringify({ token: randomUUID(), owner_pid: process.pid, gate }) + '\n');
  const lockInfo = fs.fstatSync(lockFd); fs.writeFileSync(lockFd, lockBytes); fs.fsyncSync(lockFd);
  const failures = [], streams = {}, signals = [];
  let child = null, actualClose = false, spawnEvent = false, spawnError = null, exitEvent = false;
  let exit = { code: null, signal: null }, closeExit = { code: null, signal: null };
  const errorText = error => String(error?.stack ?? error);
  const signalHandler = signal => { signals.push(signal); failures.push(`Outer received ${signal}; waiting for owned child without killing`); };
  const onInt = () => signalHandler('SIGINT'), onTerm = () => signalHandler('SIGTERM');
  process.on('SIGINT', onInt); process.on('SIGTERM', onTerm);
  const setupStream = name => {
    const file = path.join(output, `${name}.log`), fd = fs.openSync(file, 'wx');
    const state = { path: file, fd, end: false, closed: false, observed_bytes: 0, error: null, write_error: null };
    streams[name] = state;
  };
  const observe = (name, readable) => {
    const state = streams[name];
    if (!readable) { state.error = 'Missing piped stream'; return; }
    readable.on('data', chunk => {
      state.observed_bytes += chunk.length;
      if (state.write_error) return; // Continue consuming actual pipe even after disk failure.
      try { let offset = 0; while (offset < chunk.length) { const n = fs.writeSync(state.fd, chunk, offset, chunk.length - offset); assert(n > 0); offset += n; } }
      catch (error) { state.write_error = errorText(error); }
    });
    readable.once('end', () => { state.end = true; });
    readable.once('close', () => { state.closed = true; });
    readable.once('error', error => { state.error = errorText(error); readable.resume(); });
  };
  try {
    fs.mkdirSync(output); setupStream('stdout'); setupStream('stderr'); checkExpiry();
    await new Promise(resolve => {
      try { child = spawn(process.execPath, [harnessPath, ...args], { cwd: TASK, env: { ...process.env }, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }); }
      catch (error) { spawnError = errorText(error); resolve(); return; }
      child.once('spawn', () => { spawnEvent = true; });
      child.once('error', error => { spawnError = errorText(error); });
      child.once('exit', (code, signal) => { exitEvent = true; exit = { code, signal }; });
      child.once('close', (code, signal) => { actualClose = true; closeExit = { code, signal }; resolve(); });
      observe('stdout', child.stdout); observe('stderr', child.stderr);
    });
  } catch (error) { failures.push(errorText(error)); }
  // No asynchronous work that can drop the sole child handle occurs above close.
  for (const [name, state] of Object.entries(streams)) {
    try { fs.fsyncSync(state.fd); } catch (error) { failures.push(`${name} fsync: ${errorText(error)}`); }
    try { fs.closeSync(state.fd); state.fd_closed = true; } catch (error) { failures.push(`${name} close: ${errorText(error)}`); }
    try { state.sha256 = hashFile(state.path); state.bytes = fs.statSync(state.path).size; }
    catch (error) { state.sha256 = null; state.bytes = null; failures.push(`${name} final log read: ${errorText(error)}`); }
    state.drained = state.end && state.closed && !state.error && !state.write_error && state.bytes === state.observed_bytes && state.fd_closed === true;
    delete state.fd;
    if (!state.drained) failures.push(`${name}: actual complete pipe drain/log not proven`);
  }
  if (!actualClose || !spawnEvent || !exitEvent || !Number.isSafeInteger(child?.pid) || spawnError || exit.code !== 0 || exit.signal !== null || closeExit.code !== exit.code || closeExit.signal !== exit.signal) failures.push('Owned harness did not spawn, exit and actually close with exit 0/signal null');
  try {
    readPinned(issuedPath,ISSUED_SHA);readPinned(grantRow.path,grantRow.sha256);readPinned(preparationPath,preparationSha);
    assert.equal(hashFile(harnessPath),harnessSHA);assert.equal(hashFile(ownPath),driverSHA);assert.equal(hashFile(issued.binary.path),issued.binary.sha256);
    readPinned(grant.lease_writer.path,grant.lease_writer.sha256);admitted.verify();checkExpiry();
  } catch(error){failures.push('After-close actual Source8 source/suite/build/proof/lease guard: '+errorText(error));}
  const complete = actualClose && Object.values(streams).length === 2 && Object.values(streams).every(s => s.drained);
  let lockReleased = false;
  try {
    if (!child || complete) {
      const current = fs.lstatSync(lockPath);
      assert.equal(current.dev, lockInfo.dev); assert.equal(current.ino, lockInfo.ino);
      assert(fs.readFileSync(lockPath).equals(lockBytes)); fs.unlinkSync(lockPath); lockReleased = true;
    } else failures.push('Retained active.lock: actual child/pipe completion not proven; manual investigation required');
  } catch (error) { failures.push(`Lock retained: ${errorText(error)}`); }
  fs.closeSync(lockFd); process.off('SIGINT', onInt); process.off('SIGTERM', onTerm);
  const proof = {
    kind: 'combined_large_native_outer_v8', result: failures.length ? 'FAIL_OUTER_CHILD' : 'PASS_OUTER_CHILD_EXIT_NOT_INNER_ACCEPTANCE', recorded_at: new Date().toISOString(),
    gate, source_manifest_sha256: SOURCE_SHA, driver_sha256: driverSHA,
    harness: { path: harnessPath, sha256: harnessSHA, args }, node: { executable: process.execPath, version: process.versions.node, no_preloads: true },
    actual_build_proof: { path: issuedPath, sha256: ISSUED_SHA }, proof_inputs: admitted.core, grant: { path: grantRow.path, sha256: grantRow.sha256 }, binary: issued.binary,
    pid: child?.pid ?? null, spawn_event: spawnEvent, actual_exit_event: exitEvent, actual_close_event: actualClose, spawn_error: spawnError, exit, close_exit: closeExit,
    stdout: streams.stdout ?? null, stderr: streams.stderr ?? null, signals, lock_released: lockReleased, failures,
    qualification: 'Only the direct harness process and captured pipes are observed here. Inner proofs independently determine acceptance, descendant cleanup and scope. No outer timeout, kill, retry, installation or source mutation.',
  };
  const proofPath = path.join(output, 'proof.json'); fs.writeFileSync(proofPath, JSON.stringify(proof, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify({ gate, proof: proofPath, sha256: hashFile(proofPath), result: proof.result }));
  if (failures.length) process.exitCode = 1;
}
const driverSHA = hashFile(ownPath);
await main().catch(error => { console.error(error?.stack ?? error); process.exitCode = 1; });
