import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { symlinkSync } from 'node:fs';
import { readFile, writeFile, mkdtemp, mkdir, rm, readdir, lstat, realpath, readlink, unlink } from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

// Prepared source only. Root supplies hashes of the actual future build and
// source snapshot; a matching hash does not itself prove build provenance.
const here = path.dirname(fileURLToPath(import.meta.url));
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const hashFile = async file => sha(await readFile(file));
const pending = new Set();
const TIMEOUT = 20_000;
async function bounded(promise, label, ms = TIMEOUT) {
  let timer;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`Timeout: ${label}`)), ms);
      pending.add(timer);
    })]);
  } finally { clearTimeout(timer); pending.delete(timer); }
}
function below(root, relative) {
  assert.equal(typeof relative, 'string');
  assert(!path.isAbsolute(relative));
  const resolved = path.resolve(root, relative);
  const rel = path.relative(root, resolved);
  assert(rel && !rel.startsWith(`..${path.sep}`) && rel !== '..' && !path.isAbsolute(rel));
  return resolved;
}
async function verify(file, expected) {
  assert.match(expected, /^[0-9a-f]{64}$/);
  assert.equal(await hashFile(file), expected, `SHA-256 mismatch: ${file}`);
}
assert.deepEqual(process.argv.slice(2, 3), ['--binding']);
assert.equal(process.argv.length, 4, 'Usage: node mcp-file-tools-sdk.mjs --binding absolute.json');
assert(path.isAbsolute(process.argv[3]));
const binding = JSON.parse(await readFile(process.argv[3], 'utf8'));
for (const key of ['binary', 'sourceRoot', 'sdkInstallRoot', 'resultParent']) {
  assert(path.isAbsolute(binding[key]), `${key} must be an absolute path`);
}
const preparation = path.join(here, 'manifest.json');
await verify(preparation, binding.preparationManifestSha256);
const manifest = JSON.parse(await readFile(preparation, 'utf8'));
for (const item of manifest.payloads) await verify(below(here, item.path), item.sha256);
const contract = JSON.parse(await readFile(path.join(here, 'contract.json'), 'utf8'));
await verify(binding.binary, binding.binarySha256);
assert(path.isAbsolute(binding.sourceManifest.path));
await verify(binding.sourceManifest.path, binding.sourceManifest.sha256);
assert(Array.isArray(binding.sourceFiles));
assert.equal(contract.sourceManifestSha256,'fd587ba252595d971e7557e5cca7f5ab90219ecda200a3f07fcd5c10c13b6397','Actual frozen Source7 authority');assert.equal(binding.sourceManifestSha256,contract.sourceManifestSha256);assert.match(binding.sourceManifestSha256,/^[a-f0-9]{64}$/);assert.equal(binding.sourceManifest.sha256,binding.sourceManifestSha256);
const sourceManifest=JSON.parse(await readFile(binding.sourceManifest.path,'utf8'));assert.equal(sourceManifest.kind,'combined_source_v7');assert.equal(sourceManifest.source_inputs,contract.sourceInputs);assert.equal(sourceManifest.files.length,contract.sourceInputs);assert.equal(new Set(sourceManifest.files.map(f=>f.path)).size,contract.sourceInputs);assert.deepEqual(binding.sourceFiles,sourceManifest.files.map(f=>({path:f.path,sha256:f.sha256,bytes:f.bytes})));
for (const required of contract.requiredSourceFiles) {
  assert.equal(binding.sourceFiles.filter(item => item.path === required).length, 1);
}
for (const item of binding.sourceFiles) await verify(below(binding.sourceRoot, item.path), item.sha256);
assert.equal(binding.sourceFiles.find(item => item.path === contract.fileTools.path).sha256, contract.fileTools.sha256);
for (const item of contract.sdkPins) await verify(below(binding.sdkInstallRoot, item.path), item.sha256);
const lock = JSON.parse(await readFile(path.join(binding.sdkInstallRoot, 'package-lock.json'), 'utf8'));
for (const name of ['client', 'core']) {
  const packagePath = `node_modules/@modelcontextprotocol/${name}`;
  const installed = JSON.parse(await readFile(path.join(binding.sdkInstallRoot, packagePath, 'package.json'), 'utf8'));
  assert.equal(installed.version, '2.3.1');
  assert.equal(lock.packages[packagePath].version, installed.version);
}
const { Client, StreamableHTTPClientTransport } = await import(pathToFileURL(path.join(binding.sdkInstallRoot, 'node_modules/@modelcontextprotocol/client/dist/index.mjs')));
const { StdioClientTransport } = await import(pathToFileURL(path.join(binding.sdkInstallRoot, 'node_modules/@modelcontextprotocol/client/dist/stdio.mjs')));
const runRoot = await mkdtemp(path.join(binding.resultParent, 'mcp-file-tools-sdk-'));
const result = {
  status: 'RUNNING', runRoot, startedAt: new Date().toISOString(),
  node: process.version, platform: process.platform,
  preparationManifestSha256: binding.preparationManifestSha256,
  binary: { path: binding.binary, sha256: binding.binarySha256 },
  source: { root: binding.sourceRoot, manifest: binding.sourceManifest, files: binding.sourceFiles,
    provenance: 'Root-supplied source/build binding; rebuild provenance is a separate gate' },
  sdk: { package: '@modelcontextprotocol/client', version: '2.3.1', pins: contract.sdkPins,
    negotiation: 'legacy: official 2025 initialize, no disposable probe sibling' },
  cases: [], operations: [], processes: [], errors: [], websiteRequests: 0,
  httpGracefulDrain: 'NOT PROVEN: no in-flight HTTP worker-drain proof; Windows owned force cleanup is not a drain result',
};
let trap;
const owned = [];
let baseline;
let previousScopeWitness;
let activeSession;
function observeChild(child, label, { stdoutIsProtocol = false } = {}) {
  assert(child && typeof child.kill === 'function', `${label}: actual owned ChildProcess required`);
  const receipt = { label, pid: child.pid ?? null, killSignals: [], closed: false, stdoutEnd: false, stderrEnd: false };
  child.once('spawn', () => { receipt.pid = child.pid; });
  const kill = child.kill.bind(child);
  child.kill = signal => { receipt.killSignals.push(signal ?? 'SIGTERM'); return kill(signal); };
  const stdout = [], stderr = [];
  // Protocol stdout is already consumed by the SDK. A second listener observes
  // bytes without replacing or implementing framing or message dispatch.
  child.stdout.on('data', bytes => stdout.push(Buffer.from(bytes)));
  child.stderr.on('data', bytes => stderr.push(Buffer.from(bytes)));
  child.stdout.once('end', () => { receipt.stdoutEnd = true; });
  child.stderr.once('end', () => { receipt.stderrEnd = true; });
  const closed = new Promise(resolve => child.once('close', (code, signal) => {
    Object.assign(receipt, { closed: true, code, signal }); resolve(receipt);
  }));
  const failed = new Promise((_, reject) => child.once('error', reject));
  // An error is also awaited in startup; attach a catch now to avoid a delayed
  // unhandled rejection after successful startup.
  failed.catch(() => {});
  const item = { child, label, receipt, stdout, stderr, closed, failed, stdoutIsProtocol };
  owned.push(item); result.processes.push(receipt);
  return item;
}
async function flushLogs(item) {
  await writeFile(path.join(runRoot, `${item.label}.stdout.log`), Buffer.concat(item.stdout));
  await writeFile(path.join(runRoot, `${item.label}.stderr.log`), Buffer.concat(item.stderr));
}
async function cleanupChild(item, gracefulRequested = false) {
  const running = () => item.child.exitCode === null && item.child.signalCode === null;
  if (!item.receipt.closed && running() && gracefulRequested && process.platform !== 'win32') item.child.kill('SIGINT');
  if (!item.receipt.closed && gracefulRequested && process.platform !== 'win32') {
    try { await bounded(item.closed, `${item.label} SIGINT close`, 5_000); } catch {}
  }
  if (!item.receipt.closed && running()) {
    item.receipt.forceCleanup = true;
    item.child.kill('SIGKILL'); // Only this exact retained, still-open owned child.
  }
  if (!item.receipt.closed) await bounded(item.closed, `${item.label} owned actual close`);
  assert(item.receipt.closed && item.receipt.stdoutEnd && item.receipt.stderrEnd,
    `${item.label}: actual child close and both output pipes must drain`);
  await flushLogs(item);
}
async function startHttp(enabled, grant, label) {
  const data = path.join(runRoot, `${label}-crawl-data`);
  const args = ['serve', '--listen', '127.0.0.1:0', '--data-dir', data];
  if (enabled) args.push('--mcp-files-dir', grant);
  const child = spawn(binding.binary, args, {
    cwd: binding.sourceRoot, windowsHide: true, shell: false, stdio: ['pipe', 'pipe', 'pipe'],
  });
  const item = observeChild(child, label);
  child.stdin.end();
  let buffered = '';
  const announced = new Promise((resolve, reject) => {
    const onData = bytes => {
      buffered += bytes.toString('utf8');
      if (buffered.length > 1024 * 1024) { reject(new Error('Startup output exceeds bound')); return; }
      for (;;) {
        const newline = buffered.indexOf('\n');
        if (newline < 0) break;
        const line = buffered.slice(0, newline).replace(/\r$/, '');
        buffered = buffered.slice(newline + 1);
        const match = /^Open SEO Spider: http:\/\/127\.0\.0\.1:([0-9]+)$/.exec(line);
        if (match) {
          const port = Number(match[1]); assert(port > 0 && port < 65536);
          child.stdout.off('data', onData);
          item.receipt.startupLine = line; item.receipt.port = port;
          resolve(`http://127.0.0.1:${port}`);
        }
      }
    };
    child.stdout.on('data', onData);
  });
  const url = await bounded(Promise.race([announced, item.failed,
    item.closed.then(() => { throw new Error(`${label} closed before actual address announcement`); })]), `${label} stdout address`);
  return { item, url };
}
function note(label, detail) { result.cases.push({ label, status: 'PASS', ...detail }); }
function describeArguments(args) {
  return Object.fromEntries(Object.entries(args).map(([key, value]) => [key,
    typeof value === 'string' && Buffer.byteLength(value) > 256
      ? { utf8Bytes: Buffer.byteLength(value), sha256: sha(Buffer.from(value)) } : value]));
}
async function call(client, name, args = {}) {
  const operation = { session: activeSession, name, arguments: describeArguments(args), expected: 'success', status: 'PENDING' };
  result.operations.push(operation);
  const value = await bounded(client.callTool({ name, arguments: args }), name);
  assert.equal(value.isError, false, `${name}: ${JSON.stringify(value)}`);
  assert(value.structuredContent && typeof value.structuredContent === 'object');
  operation.status = 'PASS';
  return value.structuredContent;
}
async function fileCall(client, name, args = {}) {
  const value = await call(client, name, args);
  assert.equal(value.website_requests, 0);
  return value;
}
async function denied(client, name, args) {
  const operation = { session: activeSession, name, arguments: describeArguments(args), expected: 'rejection', status: 'PENDING' };
  result.operations.push(operation);
  let failure;
  try {
    const value = await bounded(client.callTool({ name, arguments: args }), `expected rejection ${name}`);
    assert.equal(value.isError, true, `unexpected successful operation ${name}`);
    failure = { kind: 'tool_error', error: value };
  } catch (error) {
    // Connection errors, timeouts, SDK validators, and assertion failures must
    // never turn a broken test into an expected rejection.
    if (![-32602, -32603].includes(error.code)) throw error;
    failure = { kind: 'jsonrpc_error', code: error.code, message: error.message };
  }
  assert(failure);
  Object.assign(operation, { status: 'PASS', failure });
  return failure;
}
async function nativeIdentity(target) {
  const info = await lstat(target, { bigint: true });
  return {
    observed_dev: String(info.dev), observed_ino: String(info.ino),
    directory: info.isDirectory(), symbolic_link: info.isSymbolicLink(),
  };
}
async function outsideWitness(outside) {
  const bytes = await readFile(path.join(outside, 'witness.txt'));
  const entries = (await readdir(outside, { withFileTypes: true }))
    .map(entry => ({ name: entry.name, directory: entry.isDirectory(), symbolic_link: entry.isSymbolicLink() }))
    .sort((left, right) => Buffer.compare(Buffer.from(left.name), Buffer.from(right.name)));
  return {
    directory_identity: await nativeIdentity(outside), entries,
    file_identity: await nativeIdentity(path.join(outside, 'witness.txt')),
    byte_length: bytes.length, bytes_hex: bytes.toString('hex'), sha256: sha(bytes),
  };
}
async function windowsJunctionCases(client, grant, label, trapUrl) {
  const receipt = {
    label: `${label}: native Windows inner-junction containment`, status: 'PENDING',
    source_path: contract.fileTools.path, source_sha256: contract.fileTools.sha256,
    grant_root: grant, paths: ['junction', 'junction/witness.txt', 'junction/new.txt', 'junction/new-directory'],
  };
  result.cases.push(receipt);
  if (process.platform !== 'win32') {
    receipt.status = 'NOT_RUN'; receipt.reason = 'Native Windows junction case requires Windows; no PASS skip';
    throw new Error(receipt.reason);
  }
  let created = false;
  let outside;
  let junction;
  try {
    // Every fixture pathname is owned by this single mkdtemp run on E:.
    // The target is a sibling of the grant, never an arbitrary host directory.
    assert(/^e:[\\/]/i.test(path.resolve(runRoot)), 'native junction fixture must stay on the approved own E: run');
    outside = below(runRoot, `${label}-junction-outside`);
    junction = below(grant, 'junction');
    assert(path.relative(grant, outside).startsWith(`..${path.sep}`));
    await mkdir(outside);
    await writeFile(path.join(outside, 'witness.txt'), `outside witness ${label}\n${trapUrl}`, { flag: 'wx' });
    receipt.outside_root = outside;
    receipt.grant_identity_before = await nativeIdentity(grant);
    assert(receipt.grant_identity_before.directory && !receipt.grant_identity_before.symbolic_link);
    receipt.outside_before = await outsideWitness(outside);
    // This is a directory junction, not a privileged file-symlink fallback.
    symlinkSync(outside, junction, 'junction');
    created = true;
    receipt.junction_identity = await nativeIdentity(junction);
    receipt.junction_target = await readlink(junction);
    receipt.junction_realpath = await realpath(junction);
    receipt.outside_realpath = await realpath(outside);
    assert(receipt.junction_identity.symbolic_link, 'fixture must be an actual reparse link');
    assert.equal(receipt.junction_realpath.toLowerCase(), receipt.outside_realpath.toLowerCase());
    await denied(client, 'spider_list_directory', { path: 'junction' });
    await denied(client, 'spider_read_text_file', { path: 'junction/witness.txt' });
    await denied(client, 'spider_write_text_file', { path: 'junction/witness.txt', text: 'must not replace outside witness' });
    await denied(client, 'spider_write_text_file', { path: 'junction/new.txt', text: 'must not create outside file' });
    await denied(client, 'spider_create_directory', { path: 'junction/new-directory' });
    receipt.outside_after = await outsideWitness(outside);
    receipt.grant_identity_after = await nativeIdentity(grant);
    assert.deepEqual(receipt.outside_after, receipt.outside_before, 'outside native identity, exact bytes and complete entry names/types remain unchanged');
    assert.deepEqual(receipt.grant_identity_after, receipt.grant_identity_before);
    await unlink(junction); // Remove only the own reparse entry, never recurse through it.
    created = false;
    receipt.junction_removed = true;
    receipt.outside_after_unlink = await outsideWitness(outside);
    assert.deepEqual(receipt.outside_after_unlink, receipt.outside_before);
    receipt.status = 'PASS';
  } catch (error) {
    receipt.status = 'FAIL'; receipt.error = { code: error.code, message: error.message };
    throw error;
  } finally {
    // Preserve outside observations even when an unexpectedly successful SDK
    // operation aborts the case. Creation failures also stay explicit FAIL.
    if (outside && receipt.outside_before && !receipt.outside_after) {
      try { receipt.outside_after = await outsideWitness(outside); }
      catch (error) { receipt.outside_observation_error = error.message; }
    }
    if (created) {
      try { await unlink(junction); receipt.junction_removed = true; }
      catch (error) { receipt.junction_cleanup_error = error.message; }
    }
  }
}
async function fileCases(client, grant, label, trapUrl) {
  const allowed = await fileCall(client, 'spider_list_allowed_directory');
  assert.deepEqual(Object.keys(allowed).sort(), ['allowed_directory', 'authority', 'limits', 'website_requests']);
  assert.equal(allowed.allowed_directory, '.');
  assert.equal(allowed.authority, 'configured_directory_handle');
  assert.deepEqual(allowed.limits, {
    write_text_max_bytes: 1048576, read_chunk_max_bytes: 262144,
    read_file_max_bytes: 33554432, directory_max_entries: 10000,
    directory_name_max_bytes: 2097152, concurrent_file_operations: 4,
  });
  const created = await fileCall(client, 'spider_create_directory', { path: 'nested/child' });
  assert.deepEqual(created.created_components, ['nested', 'child']);
  assert.equal(created.directory_available, true);
  assert.deepEqual((await fileCall(client, 'spider_create_directory', { path: 'nested/child' })).created_components, []);
  const text = `Aä🙂Z\n${trapUrl}`;
  const bytes = Buffer.from(text);
  const target = 'nested/child/unicode.txt';
  const written = await fileCall(client, 'spider_write_text_file', { path: target, text });
  assert.equal(written.bytes_written, bytes.length);
  assert.equal(written.sha256, sha(bytes));
  assert.equal(written.atomic_replace, true); assert.equal(written.file_synced, true);
  assert.equal(written.existing_regular_file_observed_before_write, false);
  assert.deepEqual(await readFile(path.join(grant, target)), bytes);
  const first = await fileCall(client, 'spider_read_text_file', { path: target, max_bytes: 2 });
  assert.equal(first.text, 'A'); assert.equal(first.byte_length, 1); assert.equal(first.next_offset, 1);
  assert.equal(first.total_bytes, bytes.length); assert.equal(first.sha256, sha(bytes));
  assert.match(first.snapshot, /^[0-9a-f]{64}$/);
  const second = await fileCall(client, 'spider_read_text_file', { path: target, offset: 1, max_bytes: 2, snapshot: first.snapshot });
  assert.equal(second.text, 'ä'); assert.equal(second.next_offset, 3);
  const rest = await fileCall(client, 'spider_read_text_file', { path: target, offset: 3, snapshot: first.snapshot });
  assert.equal(rest.text, text.slice(2)); assert.equal(rest.eof, true); assert.equal(rest.next_offset, null);
  const eof = await fileCall(client, 'spider_read_text_file', { path: target, offset: bytes.length, max_bytes: 1, snapshot: first.snapshot });
  assert.equal(eof.text, ''); assert.equal(eof.byte_length, 0); assert.equal(eof.eof, true);
  await denied(client, 'spider_read_text_file', { path: target, offset: 1 });
  await denied(client, 'spider_read_text_file', { path: target, offset: 2, snapshot: first.snapshot });
  await denied(client, 'spider_read_text_file', { path: target, offset: 3, max_bytes: 3, snapshot: first.snapshot });
  await denied(client, 'spider_read_text_file', { path: target, offset: bytes.length + 1, snapshot: first.snapshot });
  const overwrite = await fileCall(client, 'spider_write_text_file', { path: target, text: 'replacement' });
  assert.equal(overwrite.existing_regular_file_observed_before_write, true);
  assert.deepEqual(await readFile(path.join(grant, target)), Buffer.from('replacement'));
  await denied(client, 'spider_read_text_file', { path: target, offset: 1, snapshot: first.snapshot });
  await fileCall(client, 'spider_write_text_file', { path: 'empty.txt', text: '' });
  const empty = await fileCall(client, 'spider_read_text_file', { path: 'empty.txt', max_bytes: 1 });
  assert.equal(empty.text, ''); assert.equal(empty.total_bytes, 0); assert.equal(empty.eof, true);
  await fileCall(client, 'spider_create_directory', { path: 'paging' });
  for (const name of ['c.txt', 'a.txt', 'b.txt']) await fileCall(client, 'spider_write_text_file', { path: `paging/${name}`, text: name });
  const page = await fileCall(client, 'spider_list_directory', { path: 'paging', limit: 1 });
  assert.equal(page.total_entries, 3); assert.equal(page.entries[0].name, 'a.txt'); assert(page.next_cursor);
  assert.equal(page.entries[0].kind, 'file'); assert.equal(page.entries[0].addressable_name, true);
  const continuation = await fileCall(client, 'spider_list_directory', { path: 'paging', limit: 200, cursor: page.next_cursor });
  assert.deepEqual(continuation.entries.map(entry => entry.name), ['b.txt', 'c.txt']); assert.equal(continuation.next_cursor, null);
  await fileCall(client, 'spider_write_text_file', { path: 'paging/a.txt', text: 'changed bytes only' });
  assert.equal((await fileCall(client, 'spider_list_directory', { path: 'paging', cursor: page.next_cursor })).total_entries, 3);
  await denied(client, 'spider_list_directory', { path: '.', cursor: page.next_cursor });
  await fileCall(client, 'spider_write_text_file', { path: 'paging/d.txt', text: 'new name' });
  await denied(client, 'spider_list_directory', { path: 'paging', cursor: page.next_cursor });
  const badPaths = ['', '..', '../escape.txt', '/absolute.txt', './x', 'a//b', 'x/../y', 'x/', 'x\0y', '.__open_seo_spider_mcp_tmp_forged'];
  if (process.platform === 'win32') badPaths.push('C:/escape.txt', '\\server\\share', 'a\\b', 'CON', 'file:stream', 'trailing.', 'trailing ');
  for (const target of badPaths) await denied(client, 'spider_write_text_file', { path: target, text: 'must not exist' });
  for (const args of [{ path: '.', limit: 0 }, { path: '.', limit: 201 }, { path: '.', cursor: 'bad' }, { path: '.', cursor: 'x'.repeat(1048577) }]) await denied(client, 'spider_list_directory', args);
  for (const args of [{ path: target, max_bytes: 0 }, { path: target, max_bytes: 262145 }, { path: target, offset: -1 }, { path: target, max_bytes: '1' }, { path: target, snapshot: 'A'.repeat(64) }, { path: target, unexpected: true }]) await denied(client, 'spider_read_text_file', args);
  for (const args of [{ path: target, text: 1 }, { path: target }, { path: target, text: 'x', overwrite: false }, { path: 'nested', text: 'x' }, { path: 'missing/leaf.txt', text: 'x' }]) await denied(client, 'spider_write_text_file', args);
  await denied(client, 'spider_create_directory', { path: target });
  await denied(client, 'spider_list_allowed_directory', { path: '.' });
  const maximum = 'ä'.repeat(524288); // Exactly 1 MiB of UTF-8, not characters.
  const maxWrite = await fileCall(client, 'spider_write_text_file', { path: 'maximum.txt', text: maximum });
  assert.equal(maxWrite.bytes_written, 1048576);
  assert.deepEqual(await readFile(path.join(grant, 'maximum.txt')), Buffer.from(maximum));
  await denied(client, 'spider_write_text_file', { path: 'maximum.txt', text: `${maximum}x` });
  assert.deepEqual(await readFile(path.join(grant, 'maximum.txt')), Buffer.from(maximum));
  const maxRead = await fileCall(client, 'spider_read_text_file', { path: 'maximum.txt', max_bytes: 262144 });
  assert.equal(maxRead.byte_length, 262144); assert.equal(Buffer.byteLength(maxRead.text), 262144);
  await writeFile(path.join(grant, 'invalid-utf8.bin'), Buffer.from([0x61, 0xff]));
  await denied(client, 'spider_read_text_file', { path: 'invalid-utf8.bin' });
  await writeFile(path.join(grant, 'read-boundary.txt'), Buffer.alloc(33554432, 0x61));
  const boundary = await fileCall(client, 'spider_read_text_file', { path: 'read-boundary.txt', max_bytes: 1 });
  assert.equal(boundary.total_bytes, 33554432); assert.equal(boundary.text, 'a');
  const boundaryEof = await fileCall(client, 'spider_read_text_file', { path: 'read-boundary.txt', offset: 33554432, max_bytes: 1, snapshot: boundary.snapshot });
  assert.equal(boundaryEof.eof, true); assert.equal(boundaryEof.text, '');
  await writeFile(path.join(grant, 'read-boundary.txt'), Buffer.alloc(33554433, 0x61));
  await denied(client, 'spider_read_text_file', { path: 'read-boundary.txt' });
  const finalListing = await fileCall(client, 'spider_list_directory');
  assert(!finalListing.entries.some(entry => entry.name.startsWith('.__open_seo_spider_mcp_tmp_')));
  assert.deepEqual(await readFile(path.join(grant, target)), Buffer.from('replacement'));
  const stableRead = await fileCall(client, 'spider_read_text_file', { path: target });
  const stablePage = await fileCall(client, 'spider_list_directory', { path: 'paging', limit: 1 });
  assert(stablePage.next_cursor);
  if (previousScopeWitness) {
    // Same relative path, bytes, sorted names, and types in separate grants:
    // failure demonstrates capability-instance binding rather than mutation.
    assert.equal(stableRead.sha256, previousScopeWitness.sha256);
    await denied(client, 'spider_read_text_file', { path: target, snapshot: previousScopeWitness.snapshot });
    await denied(client, 'spider_list_directory', { path: 'paging', cursor: previousScopeWitness.cursor });
  }
  previousScopeWitness = { snapshot: stableRead.snapshot, sha256: stableRead.sha256, cursor: stablePage.next_cursor };
  note(`${label}: five file tools and bounded acceptance cases`, { cases: contract.fileCases });
}
async function session(kind, enabled, server, grant, label, trapUrl, exerciseFiles = true) {
  activeSession = label;
  const client = new Client({ name: 'open-seo-spider-file-tools-acceptance', version: '1.0.0' }, { versionNegotiation: { mode: 'legacy' } });
  let item;
  const sessionChildren = [];
  const transport = kind === 'stdio'
    ? new StdioClientTransport({ command: binding.binary, args: ['mcp', '--server', server.url, ...(enabled ? ['--mcp-files-dir', grant] : [])], cwd: binding.sourceRoot, stderr: 'pipe' })
    : new StreamableHTTPClientTransport(new URL(`${server.url}/mcp`));
  if (kind === 'stdio') {
    // Pinned SDK start() synchronously assigns _process inside its Promise
    // executor before returning its promise. Capture here, before Client's
    // first await/initialize, including children whose handshake later fails.
    // The unmodified SDK still starts the process and owns the MCP protocol.
    const originalStart = transport.start.bind(transport);
    transport.start = (...args) => {
      const started = originalStart(...args);
      const child = transport._process;
      if (child && !sessionChildren.some(record => record.child === child)) {
        item = observeChild(child, sessionChildren.length ? `${label}-unexpected-${sessionChildren.length}` : label,
          { stdoutIsProtocol: true });
        sessionChildren.push(item);
      }
      return started;
    };
    // Attach stderr before start, per the official transport contract.
    transport.stderr.on('data', () => {});
  }
  try {
    await bounded(client.connect(transport), `${label} official SDK connect`);
    if (kind === 'stdio') {
      assert.equal(sessionChildren.length, 1, 'legacy initialize must own exactly one captured SDK child');
      assert.equal(item.child, transport._process);
      assert.equal(transport.pid, item.child.pid);
    }
    const inventory = (await bounded(client.listTools(), `${label} inventory`)).tools;
    assert.equal(inventory.length, enabled ? 31 : 26);
    assert.equal(new Set(inventory.map(tool => tool.name)).size, inventory.length);
    assert.deepEqual(inventory.slice(0, 26).map(tool => tool.name), contract.legacyNames);
    if (!baseline) baseline = inventory.slice(0, 26);
    else assert.deepEqual(inventory.slice(0, 26), baseline, 'exact legacy descriptors remain unchanged');
    assert.deepEqual(await call(client, 'spider_list_crawls'), { crawls: [] });
    if (enabled) {
      assert.deepEqual(inventory.slice(26).map(tool => tool.name), contract.fileNames);
      for (const [index, tool] of inventory.slice(26).entries()) {
        assert.equal(tool.inputSchema.additionalProperties, false);
        assert.equal(tool.annotations.openWorldHint, false);
        assert.equal(tool.annotations.readOnlyHint, [true, true, false, true, false][index]);
      }
      if (exerciseFiles) {
        await fileCases(client, grant, label, trapUrl);
        await windowsJunctionCases(client, grant, label, trapUrl);
      }
    } else {
      for (const name of contract.fileNames) await denied(client, name, {});
    }
    note(`${label}: official initialize, inventory, exact legacy descriptor prefix and empty crawl list`, { transport: kind, enabled, tools: inventory.length });
  } finally {
    await bounded(client.close(), `${label} SDK close`);
    for (const captured of sessionChildren) {
      await bounded(captured.closed, `${captured.label} actual STDIO child close`);
      assert.equal(captured.receipt.code, 0, 'STDIO EOF must exit cleanly');
      assert.equal(captured.receipt.signal, null);
      assert.deepEqual(captured.receipt.killSignals, [], 'SDK force fallback is not graceful drain');
      assert(captured.receipt.stdoutEnd && captured.receipt.stderrEnd);
      captured.receipt.eofDrain = 'PASS: EOF, actual code0 child close and both output pipes drained';
      await flushLogs(captured);
    }
  }
}
try {
  trap = http.createServer((_, response) => { result.websiteRequests++; response.writeHead(500); response.end('website trap must remain unused'); });
  trap.listen(0, '127.0.0.1'); await once(trap, 'listening');
  const trapUrl = `http://127.0.0.1:${trap.address().port}/must-not-fetch`;
  for (const enabled of [false, true]) {
    const httpGrant = path.join(runRoot, `${enabled ? 'enabled' : 'disabled'}-http-grant`);
    const stdioGrant = path.join(runRoot, `${enabled ? 'enabled' : 'disabled'}-stdio-grant`);
    await mkdir(httpGrant); await mkdir(stdioGrant);
    const server = await startHttp(enabled, httpGrant, `${enabled ? 'enabled' : 'disabled'}-http-server`);
    try {
      await session('stdio', enabled, server, stdioGrant, `${enabled ? 'enabled' : 'disabled'}-stdio`, trapUrl);
      await session('streamable_http', enabled, server, httpGrant, `${enabled ? 'enabled' : 'disabled'}-http`, trapUrl);
      await session('stdio', !enabled, server, stdioGrant,
        `${enabled ? 'http-enabled-stdio-disabled' : 'http-disabled-stdio-enabled'}`, trapUrl, false);
    } finally { await cleanupChild(server.item, true); }
  }
  assert.equal(result.websiteRequests, 0);
  assert.equal(await hashFile(binding.binary), binding.binarySha256, 'binary unchanged after acceptance');
  await verify(binding.sourceManifest.path, binding.sourceManifest.sha256);
  for (const item of binding.sourceFiles) await verify(below(binding.sourceRoot, item.path), item.sha256);
  for (const item of contract.sdkPins) await verify(below(binding.sdkInstallRoot, item.path), item.sha256);
  result.status = 'PASS_FILE_TOOL_ACCEPTANCE';
} catch (error) {
  result.status = 'FAIL'; result.errors.push({ message: error.message, stack: error.stack }); process.exitCode = 1;
} finally {
  for (const item of owned) {
    try { if (!item.receipt.closed) await cleanupChild(item); else await flushLogs(item); }
    catch (error) { result.status = 'FAIL'; result.errors.push({ cleanup: item.label, message: error.message }); process.exitCode = 1; }
  }
  if (trap) {
    try { await bounded(new Promise((resolve, reject) => trap.close(error => error ? reject(error) : resolve())), 'owned trap server close'); }
    catch (error) { result.status = 'FAIL'; result.errors.push({ cleanup: 'website trap', message: error.message }); process.exitCode = 1; }
  }
  result.websiteTrapClosed = trap ? !trap.listening : null;
  if (result.websiteRequests !== 0) {
    result.status = 'FAIL'; result.errors.push({ websiteRequests: result.websiteRequests }); process.exitCode = 1;
  }
  result.completedAt = new Date().toISOString();
  // Remove only grants and crawl data under this mkdtemp-owned directory,
  // after every retained child is confirmed closed. Preserve logs/receipt.
  try {
    if (owned.every(item => item.receipt.closed)) {
      for (const name of await readdir(runRoot)) {
        if (/^(enabled|disabled)-(http-grant|stdio-grant|http-server-crawl-data)$/.test(name)
          || /^enabled-(stdio|http)-junction-outside$/.test(name)) await rm(below(runRoot, name), { recursive: true, force: false });
      }
      result.fixtureCleanup = 'owned fixture directories removed after actual child close';
    } else result.fixtureCleanup = 'retained: an owned child did not close';
  } catch (error) {
    result.status = 'FAIL'; result.errors.push({ cleanup: 'owned fixture directory', message: error.message }); process.exitCode = 1;
  }
  await writeFile(path.join(runRoot, 'result.json'), `${JSON.stringify(result, null, 2)}\n`, { flag: 'wx' });
  for (const timer of pending) clearTimeout(timer);
  process.stdout.write(`${JSON.stringify({ status: result.status, result: path.join(runRoot, 'result.json'), httpGracefulDrain: result.httpGracefulDrain })}\n`);
}
