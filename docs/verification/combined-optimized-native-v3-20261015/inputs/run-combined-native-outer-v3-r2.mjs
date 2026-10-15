import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

// One direct child only. Inner guardians retain their own deadlines/oracles.
const BASE = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const TASK = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const SOURCE_SHA = '01bfa8efcb6745c5521be5e533ae63f2f15679318b095a29eb3ab925fef95bdf';
const ISSUED_SHA = '1d2cd016e1cf4d201962d0b7da2965c1bfa0260f3eebf1dc54700b1461f2473d';
const BINDER_SHA = '69501b4ad1d2b0b7bef044ff8eafd6e047541f6e4088544d9f63a8f8d275a45a';
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
  native: ['v3-r2-native-release.json', work('run-combined-findings-acceptance-v3.mjs'), '849d605993eed0babcde52b7153d463c2a9b30202e566e819a2fe7f373a5474d', 'native-combined-rebind-v3', '7886162391eb6ad14fa98d0082ac4b3889352ba52deb963a8dbc0b0fa14013b5'],
  'ui-mock': ['v3-r2-ui-mock-release.json', work('run-combined-findings-acceptance-v3.mjs'), '849d605993eed0babcde52b7153d463c2a9b30202e566e819a2fe7f373a5474d', 'native-combined-rebind-v3', '7886162391eb6ad14fa98d0082ac4b3889352ba52deb963a8dbc0b0fa14013b5'],
  'browser-native': ['v3-r2-browser-native-release.json', path.join(BASE, 'browser-combined-rebind-v3/run-sitemap-findings-browser-combined-v3.mjs'), 'e18f493b07fc26d14c730971d52a29f46b60ada11b4bc834f6748cf94e071878', 'browser-combined-rebind-v3', 'fe9937b97ca1fe33608445f2f96388e372bb11a3a36885e9677e6c30237b5881'],
  'physical-findings': ['v3-r2-physical-release.json', path.join(BASE, 'physical-combined-rebind-v3/native-finding-physical-combined-v3.mjs'), '9375b8dcf0ccce0a27c73a0b3c0e51b13c3d295563e9d1fe01baf296b9dc24f2', 'physical-combined-rebind-v3', 'b36201be73c32e4d02acc5b27ba464db473d4619ddd4177f5a802f5ba3327e9c'],
  'file-sdk': ['v3-r2-mcp-file-sdk-binding.json', path.join(BASE, 'mcp-file-tools-sdk-preparation-v3/mcp-file-tools-sdk.mjs'), '96f66ce10d41dd2878b64e25cd2eaaebd6c3bf84393eba78ca0e86066db8d2da', 'mcp-file-tools-sdk-preparation-v3', 'daf416e95eadcdbb848dbaa2ca71561fb3939e1b104a9b6c0e3b2afc413b94bf'],
  'root-junction': ['v3-r2-mcp-root-junction-binding.json', path.join(BASE, 'mcp-root-junction-cli-preparation-v2/root-junction-cli.mjs'), 'e2e9924bda81d2b8242c8358261faf012b0d7c7c8b718171d896135dae76a505', 'mcp-root-junction-cli-preparation-v2', 'c171acdfb2eaa1c14ceb7cdacdbe9531eb516103479126ccc33dbc3d19907f5a'],
};

async function main() {
  assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed');
  assert.equal(process.platform, 'win32'); assert.match(process.versions.node, /^24\./);
  assert(!process.env.NODE_OPTIONS && !process.env.NODE_PATH); assert.equal(process.execArgv.length, 0);
  samePath(ownPath, work('run-combined-native-outer-v3-r2.mjs'));
  assert.equal(process.argv.length, 6, 'Use --gate NAME --bindings-sha ROOT_ISSUED_SHA');
  assert.equal(process.argv[2], '--gate'); assert.equal(process.argv[4], '--bindings-sha');
  const gate = process.argv[3]; assert(Object.hasOwn(gates, gate));
  assert.equal(process.argv[5], ISSUED_SHA);
  const [grantName, harnessPath, harnessSHA, prepDir, prepSHA] = gates[gate];
  const issuedPath = work('v3-r2-native-bindings-issued.json');
  const issued = JSON.parse(readPinned(issuedPath, ISSUED_SHA));
  assert.equal(issued.kind, 'root_concrete_v3_native_bindings_from_actual_optimized_build');
  assert.equal(issued.source_manifest_sha256, SOURCE_SHA);
  assert.equal(issued.compiled_source_commit, 'f3307413110a62dc4db00afac1b14862eee5ff3e');
  assert(Array.isArray(issued.rows) && issued.rows.length === 6);
  assert.equal(new Set(issued.rows.map(row => normalized(row.path))).size, 6);
  for (const [name] of Object.values(gates)) {
    const matches = issued.rows.filter(row => normalized(row.path) === normalized(work(name)));
    assert.equal(matches.length, 1); const row = matches[0];
    const bytes = readPinned(row.path, row.sha256); assert.equal(bytes.length, row.bytes);
  }
  const grantRow = issued.rows.find(row => normalized(row.path) === normalized(work(grantName)));
  const grant = JSON.parse(readPinned(grantRow.path, grantRow.sha256));
  const sourcePath = path.join(BASE, 'combined-source-v3/manifest.json');
  readPinned(sourcePath, SOURCE_SHA);
  const suitePath = path.join(BASE, 'results/combined-suite-v3/proof.json');
  const buildPath = path.join(BASE, 'results/combined-optimized-build-v3/proof.json');
  const suite = JSON.parse(readPinned(suitePath, issued.suite_proof_sha256));
  const build = JSON.parse(readPinned(buildPath, issued.build_proof_sha256));
  assert.equal(suite.result, 'PASS_COMBINED_WORKSPACE_V3_REAL_RENDERER_FMT_CLIPPY');
  assert.equal(build.result, 'PASS_COMBINED_OPTIMIZED_BUILD_V3');
  assert.equal(build.source_manifest_sha256, SOURCE_SHA); assert.equal(suite.source_manifest_sha256, SOURCE_SHA);
  assert.equal(build.required_suite.proof_sha256, issued.suite_proof_sha256);
  samePath(issued.binary.path, path.join(BASE, 'results/combined-optimized-build-v3/candidate/open-seo-spider.exe'));
  assert.equal(fs.statSync(issued.binary.path).size, issued.binary.bytes);
  assert.equal(hashFile(issued.binary.path), issued.binary.sha256);
  assert.equal(hashFile(harnessPath), harnessSHA);
  readPinned(path.join(BASE, prepDir, 'manifest.json'), prepSHA);
  readPinned(path.join(BASE, 'native-combined-rebind-v3/combined-bindings.mjs'), BINDER_SHA);
  const checkExpiry = () => assert(Number.isFinite(Date.parse(issued.expires_at)) && Date.now() < Date.parse(issued.expires_at), 'Issued bindings expired');
  let args;
  if (gate === 'file-sdk' || gate === 'root-junction') {
    assert.equal(grant.preparationManifestSha256, prepSHA);
    samePath(grant.binary, issued.binary.path); assert.equal(grant.binarySha256, issued.binary.sha256);
    samePath(grant.sourceManifest.path, sourcePath); assert.equal(grant.sourceManifest.sha256, SOURCE_SHA);
    samePath(grant.buildProof.path, buildPath); assert.equal(grant.buildProof.sha256, issued.build_proof_sha256);
    args = ['--binding', grantRow.path];
  } else {
    assert.equal(grant.status, 'RELEASED'); assert.equal(grant.build_mode, 'optimized');
    assert.equal(grant.source_manifest_sha256, SOURCE_SHA); assert.equal(grant.bindings_sha256, BINDER_SHA);
    assert.equal(grant.candidate_binary_sha256, issued.binary.sha256); assert.equal(grant.build_proof_sha256, issued.build_proof_sha256);
    assert(Date.now() >= Date.parse(grant.not_before)); assert.equal(grant.expires_at, issued.expires_at);
    assert.equal(grant.driver_sha256 ?? grant.harness_sha256, harnessSHA);
    if (gate === 'native' || gate === 'ui-mock') args = ['--manifest-sha', SOURCE_SHA, '--gate', gate, '--release', grantRow.path];
    else if (gate === 'browser-native') args = ['--build-mode', 'optimized', '--release', grantRow.path, '--manifest-sha', SOURCE_SHA];
    else args = ['--binary', issued.binary.path, '--binary-sha256', issued.binary.sha256, '--suite-proof', suitePath, '--suite-sha256', issued.suite_proof_sha256, '--build-proof', buildPath, '--build-sha256', issued.build_proof_sha256, '--compute-release', grantRow.path, '--release-sha256', grantRow.sha256, '--out', grant.output, '--manifest-sha', SOURCE_SHA];
  }
  checkExpiry();
  // All admission above precedes output/lock creation. No harness import/eval.
  const outputRoot = path.join(BASE, 'results/combined-native-outer-v3-r2');
  const output = path.join(outputRoot, gate), lockPath = path.join(outputRoot, 'active.lock');
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
    assert.equal(hashFile(issued.binary.path), issued.binary.sha256);
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
    kind: 'combined_native_outer_v3', result: failures.length ? 'FAIL_OUTER_CHILD' : 'PASS_OUTER_CHILD_EXIT_NOT_INNER_ACCEPTANCE', recorded_at: new Date().toISOString(),
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
