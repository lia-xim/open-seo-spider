import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { spawn } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

// One direct child only. Inner guardians retain their own deadlines/oracles.
const BASE = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const TASK = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
// Actual Source8 SHA is provided only by the Root-CLI-pinned issued receipt.
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
const gates = {
  "native": [
    "v8-native-release.json",
    "C:\\Users\\matth\\Documents\\Codex\\2026-10-11\\https-github-com-morluto-rea-https\\work\\run-combined-findings-acceptance-v8.mjs",
    "8075ea60abfcb15117254da6b62d485060e007fec5860d4e5a94cc07f95bdec5",
    "source8-native-small-preparation-v1/adapters",
    "65f6022ce6a8edc5d68773b21ab3257b08bfa945f1909a0a594c8a2d48f9d13d"
  ],
  "ui-mock": [
    "v8-ui-mock-release.json",
    "C:\\Users\\matth\\Documents\\Codex\\2026-10-11\\https-github-com-morluto-rea-https\\work\\run-combined-findings-ui-mock-acceptance-v8.mjs",
    "0c6942c893d375cac6c9a356e05741ac2dd2d38d7bb043d18a605d4be3e0e3ab",
    "source8-native-small-preparation-v1/adapters",
    "65f6022ce6a8edc5d68773b21ab3257b08bfa945f1909a0a594c8a2d48f9d13d"
  ],
  "browser-native": [
    "v8-browser-native-release.json",
    "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\browser\\run-sitemap-findings-browser-combined-v8.mjs",
    "254e443f5b69af4a811913b2840681cd7645bd013981d966b025ef1247b58269",
    "source8-native-small-preparation-v1/browser",
    "a8329c77ad797396cc628a8c1e19cda6d9906ee8fba1caae002419c2f7f70974"
  ],
  "physical-findings": [
    "v8-physical-release.json",
    "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\physical\\native-finding-physical-combined-v8.mjs",
    "e1225460134ab5d84098ab5c3e775c368aff2935e5013d928cfff2e0f29d4139",
    "source8-native-small-preparation-v1/physical",
    "3f0f670dc0cda65b8cf68b0174fa7efdf21740a842ebf6a3a46637cb3aaae044"
  ],
  "file-sdk": [
    "v8-mcp-file-sdk-binding.json",
    "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\file-sdk\\mcp-file-tools-sdk.mjs",
    "963db114159fb5c9263f675ea89ed2ec6aa0fc5b936de1a9ee2068e6834f278d",
    "source8-native-small-preparation-v1/file-sdk",
    "c796a9af1cb4285c6f69e55325f0a92a935c100d22120d075648355f41f310e8"
  ],
  "root-junction": [
    "v8-mcp-root-junction-binding.json",
    "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\root-junction\\root-junction-cli.mjs",
    "d68cc16fcdb9e94508f34ab83de551e4a4769bbc36dfc6dab798b4b9b22f2c01",
    "source8-native-small-preparation-v1/root-junction",
    "7dc9271382a7ae4dc584f099669742b0b15178d0ad112f64bddfa910483b9f85"
  ]
};

async function main() {
  assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed');
  assert.equal(process.platform, 'win32'); assert.match(process.versions.node, /^24\./);
  assert(!process.env.NODE_OPTIONS && !process.env.NODE_PATH); assert.equal(process.execArgv.length, 0);
  samePath(ownPath, work('run-combined-native-outer-v8.mjs'));
  assert.equal(process.argv.length, 6, 'Use --gate NAME --bindings-sha ROOT_ISSUED_SHA');
  assert.equal(process.argv[2], '--gate'); assert.equal(process.argv[4], '--bindings-sha');
  const gate = process.argv[3]; assert(Object.hasOwn(gates, gate));
  const ISSUED_SHA=process.argv[5];assert.match(ISSUED_SHA,/^[a-f0-9]{64}$/);
  const [grantName, harnessPath, harnessSHA, prepDir, prepSHA] = gates[gate];
  const issuedPath = work('v8-native-bindings-issued.json');
  const issued = JSON.parse(readPinned(issuedPath, ISSUED_SHA));const SOURCE_SHA=issued.source_manifest_sha256;assert.equal(SOURCE_SHA,'58ab1ff000b9ec5ffa005eeab573a0cde3f4af4bb7c9c132b4d85251ea4de720');assert.match(SOURCE_SHA,/^[a-f0-9]{64}$/);
  assert.equal(issued.kind, 'root_concrete_v8_native_bindings_from_actual_optimized_build');
  assert.equal(issued.source_manifest_sha256, SOURCE_SHA);assert.equal(issued.bindings_sha256,BINDER_SHA);assert.equal(issued.build_driver_sha256,'e48e2cadc361a100eecbbed5465bf4436d28f7fa19068682550932f320e1c547');samePath(issued.writer.path,work('prepare-v8-native-bindings.mjs'));readPinned(issued.writer.path,issued.writer.sha256);
  assert.match(issued.compiled_source_commit,/^[a-f0-9]{40}$/);
  assert(Array.isArray(issued.rows) && issued.rows.length === 6);
  assert.equal(new Set(issued.rows.map(row => normalized(row.path))).size, 6);
  for (const [name] of Object.values(gates)) {
    const matches = issued.rows.filter(row => normalized(row.path) === normalized(work(name)));
    assert.equal(matches.length, 1); const row = matches[0];
    const bytes = readPinned(row.path, row.sha256); assert.equal(bytes.length, row.bytes);
  }
  const grantRow = issued.rows.find(row => normalized(row.path) === normalized(work(grantName)));
  const grant = JSON.parse(readPinned(grantRow.path, grantRow.sha256));
  const sourcePath = path.join(BASE, 'combined-source-v8/manifest.json');
  readPinned(sourcePath, SOURCE_SHA);
  const suitePath = path.join(BASE, 'results/combined-suite-v8/proof.json');
  const buildPath = path.join(BASE, 'results/combined-optimized-build-v8/proof.json');
  const suite = JSON.parse(readPinned(suitePath, issued.suite_proof_sha256));
  const build = JSON.parse(readPinned(buildPath, issued.build_proof_sha256));
  assert.equal(suite.result, 'PASS_COMBINED_WORKSPACE_V8_REAL_RENDERER_FMT_CLIPPY');
  assert.equal(build.result, 'PASS_COMBINED_OPTIMIZED_BUILD_V8');
  assert.equal(build.source_manifest_sha256, SOURCE_SHA); assert.equal(suite.source_manifest_sha256, SOURCE_SHA);
  assert.equal(build.required_suite.proof_sha256, issued.suite_proof_sha256);
  samePath(issued.binary.path, path.join(BASE, 'results/combined-optimized-build-v8/candidate/open-seo-spider.exe'));
  assert.equal(fs.statSync(issued.binary.path).size, issued.binary.bytes);
  assert.equal(hashFile(issued.binary.path), issued.binary.sha256);
  assert.equal(hashFile(harnessPath), harnessSHA);
  readPinned(path.join(BASE, prepDir, 'manifest.json'), prepSHA);
  const binderPath=path.join(BASE,'combined-v8-runner-preparation-v4/combined-bindings-v8.mjs');readPinned(binderPath,BINDER_SHA);const {bindCombined}=await import(pathToFileURL(binderPath).href);readPinned(binderPath,BINDER_SHA);const deepBinding=bindCombined(issued,SOURCE_SHA);deepBinding.verify();samePath(deepBinding.binary,issued.binary.path);assert.equal(deepBinding.binarySHA,issued.binary.sha256);assert.equal(deepBinding.manifest.kind,'combined_source_v8');assert.equal(issued.compiled_source_commit,deepBinding.manifest.source_commit);
  const checkExpiry = () => assert(Number.isFinite(Date.parse(issued.expires_at)) && Date.now() < Date.parse(issued.expires_at), 'Issued bindings expired');
  let args;
  if (gate === 'file-sdk' || gate === 'root-junction') {
    assert.equal(grant.preparationManifestSha256, prepSHA);
    samePath(grant.binary, issued.binary.path); assert.equal(grant.binarySha256, issued.binary.sha256);
    samePath(grant.sourceManifest.path, sourcePath); assert.equal(grant.sourceManifest.sha256, SOURCE_SHA);assert.equal(grant.sourceManifestSha256,SOURCE_SHA);
    samePath(grant.buildProof.path, buildPath); assert.equal(grant.buildProof.sha256, issued.build_proof_sha256);
    args = ['--binding', grantRow.path];
  } else {
    assert.equal(grant.status, 'RELEASED'); assert.equal(grant.build_mode, 'optimized');
    assert.equal(grant.source_manifest_sha256, SOURCE_SHA); assert.equal(grant.bindings_sha256, BINDER_SHA);
    assert.equal(grant.candidate_binary_sha256, issued.binary.sha256); assert.equal(grant.build_proof_sha256, issued.build_proof_sha256);assert.equal(grant.suite_proof_sha256,issued.suite_proof_sha256);assert.equal(grant.build_driver_sha256,issued.build_driver_sha256);
    assert(Date.now() >= Date.parse(grant.not_before)); assert.equal(grant.expires_at, issued.expires_at);
    assert.equal(grant.driver_sha256 ?? grant.harness_sha256, harnessSHA);
    if (gate === 'native' || gate === 'ui-mock') args = ['--manifest-sha', SOURCE_SHA, '--gate', gate, '--release', grantRow.path];
    else if (gate === 'browser-native') args = ['--build-mode', 'optimized', '--release', grantRow.path, '--manifest-sha', SOURCE_SHA];
    else args = ['--binary', issued.binary.path, '--binary-sha256', issued.binary.sha256, '--suite-proof', suitePath, '--suite-sha256', issued.suite_proof_sha256, '--build-proof', buildPath, '--build-sha256', issued.build_proof_sha256, '--compute-release', grantRow.path, '--release-sha256', grantRow.sha256, '--out', grant.output, '--manifest-sha', SOURCE_SHA];
  }
  checkExpiry();
  // All admission above precedes output/lock creation. No harness import/eval.
  const outputRoot = path.join(BASE, 'results/combined-native-outer-v8');
  const output = path.join(outputRoot, gate), lockPath = path.join(BASE, 'results/combined-native-outer-v4/active.lock');
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
    readPinned(issuedPath, ISSUED_SHA); readPinned(grantRow.path, grantRow.sha256);
    readPinned(sourcePath, SOURCE_SHA); readPinned(suitePath, issued.suite_proof_sha256); readPinned(buildPath, issued.build_proof_sha256);
    assert.equal(hashFile(harnessPath), harnessSHA); assert.equal(hashFile(ownPath), driverSHA);
    assert.equal(hashFile(issued.binary.path), issued.binary.sha256);deepBinding.verify();readPinned(issued.writer.path,issued.writer.sha256);
  } catch (error) { failures.push(`After-close input guard: ${errorText(error)}`); }
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
    kind: 'combined_native_outer_v8', result: failures.length ? 'FAIL_OUTER_CHILD' : 'PASS_OUTER_CHILD_EXIT_NOT_INNER_ACCEPTANCE', recorded_at: new Date().toISOString(),
    gate, source_manifest_sha256: SOURCE_SHA, driver_sha256: driverSHA,
    harness: { path: harnessPath, sha256: harnessSHA, args }, node: { executable: process.execPath, version: process.versions.node, no_preloads: true },
    binding_receipt: { path: issuedPath, sha256: ISSUED_SHA }, grant: { path: grantRow.path, sha256: grantRow.sha256 }, binary: issued.binary,
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
