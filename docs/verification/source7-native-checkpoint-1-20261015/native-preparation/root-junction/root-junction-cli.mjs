import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { symlinkSync } from 'node:fs';
import { readFile, writeFile, mkdir, mkdtemp, lstat, readdir, readlink, realpath, unlink, rm } from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const hashFile = async file => digest(await readFile(file));
async function verify(file, hash) {
  assert.match(hash, /^[0-9a-f]{64}$/);
  assert.equal(await hashFile(file), hash, `SHA-256 mismatch: ${file}`);
}
function below(root, relative) {
  assert.equal(typeof relative, 'string'); assert(!path.isAbsolute(relative));
  const value = path.resolve(root, relative), part = path.relative(root, value);
  assert(part && part !== '..' && !part.startsWith(`..${path.sep}`) && !path.isAbsolute(part));
  return value;
}
async function bounded(promise, label) {
  let timer;
  try { return await Promise.race([promise, new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`Timeout: ${label}`)), 20_000);
  })]); } finally { clearTimeout(timer); }
}
async function identity(file) {
  const info = await lstat(file, { bigint: true });
  return { dev: String(info.dev), ino: String(info.ino), directory: info.isDirectory(), symbolicLink: info.isSymbolicLink() };
}
async function witness(outside) {
  const bytes = await readFile(path.join(outside, 'witness.txt'));
  return {
    directoryIdentity: await identity(outside), fileIdentity: await identity(path.join(outside, 'witness.txt')),
    entries: (await readdir(outside, { withFileTypes: true })).map(item => ({ name: item.name, directory: item.isDirectory(), symbolicLink: item.isSymbolicLink() }))
      .sort((a, b) => Buffer.compare(Buffer.from(a.name), Buffer.from(b.name))),
    byteLength: bytes.length, bytesHex: bytes.toString('hex'), sha256: digest(bytes),
  };
}
async function linkIdentity(junction) {
  return { native: await identity(junction), target: await readlink(junction), realpath: await realpath(junction) };
}
assert.deepEqual(process.argv.slice(2, 3), ['--binding']);
assert.equal(process.argv.length, 4, 'Usage: node root-junction-cli.mjs --binding absolute.json');
assert(path.isAbsolute(process.argv[3]));
const binding = JSON.parse(await readFile(process.argv[3], 'utf8'));
for (const key of ['binary', 'sourceRoot', 'resultParent']) assert(path.isAbsolute(binding[key]), `${key}: absolute path required`);
await verify(path.join(here, 'manifest.json'), binding.preparationManifestSha256);
const preparation = JSON.parse(await readFile(path.join(here, 'manifest.json'), 'utf8'));
for (const item of preparation.payloads) await verify(below(here, item.path), item.sha256);
const contract = JSON.parse(await readFile(path.join(here, 'contract.json'), 'utf8'));
assert.equal(contract.sourceManifestSha256,'fd587ba252595d971e7557e5cca7f5ab90219ecda200a3f07fcd5c10c13b6397','Actual frozen Source7 authority');assert.equal(binding.sourceManifestSha256,contract.sourceManifestSha256);const expectedSourceSha=binding.sourceManifestSha256;assert.match(expectedSourceSha,/^[a-f0-9]{64}$/);
assert(path.isAbsolute(binding.sourceManifest.path));
assert.equal(binding.sourceManifest.sha256, expectedSourceSha);
await verify(binding.sourceManifest.path, expectedSourceSha);
const source = JSON.parse(await readFile(binding.sourceManifest.path, 'utf8'));
assert.equal(source.source_inputs, 326); assert.equal(source.files.length, 326);
assert.equal(new Set(source.files.map(item => item.path)).size, 326);
async function sourceGuard() {
  await verify(binding.sourceManifest.path, expectedSourceSha);
  for (const item of source.files) await verify(below(binding.sourceRoot, item.path), item.sha256);
  for (const item of contract.criticalSources) {
    assert.equal(binding.sourceFiles.filter(pin => pin.path === item.path).length, 1);
    assert.equal(binding.sourceFiles.find(pin => pin.path === item.path).sha256, item.sha256);
    assert.equal(source.files.find(pin => pin.path === item.path).sha256, item.sha256);
  }
  await verify(binding.binary, binding.binarySha256);
}
await sourceGuard();
if (process.platform === 'win32') assert(/^e:[\\/]/i.test(path.resolve(binding.resultParent)), 'result parent must be on E: before creating any fixture');
const run = await mkdtemp(path.join(binding.resultParent, 'mcp-root-junction-'));
const result = {
  status: 'RUNNING', run, platform: process.platform, node: process.version,
  preparationManifestSha256: binding.preparationManifestSha256,
  binary: { path: binding.binary, sha256: binding.binarySha256 },
  source: { root: binding.sourceRoot, manifest: binding.sourceManifest, inputs: 326,
    provenance: 'Explicit Root-supplied build/source identity; reproducible build provenance is a separate gate' },
  cases: [], processes: [], errors: [], websiteRequests: 0,
  boundary: 'Root-junction CLI rejection only; no ordinary-directory or MCP protocol acceptance, no global socket audit or HTTP shutdown-drain claim',
};
let junctionCreated = false, trap;
const children = [];
const outside = below(run, 'outside'), junction = below(run, 'grant-junction');
function ownChild(args, cwd, label) {
  const child = spawn(binding.binary, args, { cwd, windowsHide: true, shell: false, stdio: ['pipe', 'pipe', 'pipe'] });
  // Retain and observe immediately, before any await or failure callback.
  const receipt = { label, pid: child.pid ?? null, killSignals: [], closed: false, stdoutEnd: false, stderrEnd: false };
  const stdout = [], stderr = [];
  const originalKill = child.kill.bind(child);
  child.kill = signal => { receipt.killSignals.push(signal ?? 'SIGTERM'); return originalKill(signal); };
  child.once('spawn', () => { receipt.pid = child.pid; });
  const closed = new Promise(resolve => child.once('close', (code, signal) => { Object.assign(receipt, { closed: true, code, signal }); resolve(); }));
  const failed = new Promise((_, reject) => child.once('error', reject)); failed.catch(() => {});
  const item = { child, receipt, stdout, stderr, closed, failed };
  children.push(item); result.processes.push(receipt);
  assert(child.stdout && child.stderr && child.stdin, 'actual owned piped child required');
  child.stdout.on('data', data => stdout.push(Buffer.from(data)));
  child.stderr.on('data', data => stderr.push(Buffer.from(data)));
  child.stdout.once('end', () => { receipt.stdoutEnd = true; });
  child.stderr.once('end', () => { receipt.stderrEnd = true; });
  // Keep stdin OPEN. EOF must not let an accidentally accepted MCP process exit
  // cleanly and masquerade as configuration rejection.
  return item;
}
async function logChild(item) {
  await writeFile(path.join(run, `${item.receipt.label}.stdout.log`), Buffer.concat(item.stdout));
  await writeFile(path.join(run, `${item.receipt.label}.stderr.log`), Buffer.concat(item.stderr));
}
async function exists(file) {
  try { await lstat(file); return true; } catch (error) { if (error.code === 'ENOENT') return false; throw error; }
}
try {
  if (process.platform !== 'win32') {
    result.cases.push({ name: 'native Windows root-junction', status: 'NOT_RUN', reason: 'Windows required; not a PASS skip' });
    throw new Error('Native Windows root-junction gate not run on this platform');
  }
  assert(/^e:[\\/]/i.test(path.resolve(run)), 'fixture must stay in the approved own E: run');
  // Rule out a different reparse ancestor as the actual rejection cause.
  const absolute = path.resolve(run), anchor = path.parse(absolute).root;
  let ancestor = anchor; result.ancestorIdentities = [];
  for (const part of absolute.slice(anchor.length).split(path.sep).filter(Boolean)) {
    ancestor = path.join(ancestor, part);
    const native = await identity(ancestor);
    assert(native.directory && !native.symbolicLink, `fixture ancestor is not an ordinary directory: ${ancestor}`);
    result.ancestorIdentities.push({ path: ancestor, native });
  }
  await mkdir(outside);
  await writeFile(path.join(outside, 'witness.txt'), 'native root-junction outside witness\n', { flag: 'wx' });
  result.outsideBefore = await witness(outside);
  symlinkSync(outside, junction, 'junction'); // Failure is FAIL; no privilege fallback.
  junctionCreated = true;
  result.junctionBefore = await linkIdentity(junction);
  assert(result.junctionBefore.native.symbolicLink);
  assert.equal(result.junctionBefore.realpath.toLowerCase(), (await realpath(outside)).toLowerCase());
  trap = http.createServer((_, response) => { result.websiteRequests++; response.writeHead(500); response.end('must remain unused'); });
  trap.listen(0, '127.0.0.1'); await bounded(once(trap, 'listening'), 'own unused API trap');
  const api = `http://127.0.0.1:${trap.address().port}`;
  for (const command of ['serve', 'mcp']) {
    const cwd = below(run, `${command}-cwd`), dataDir = below(run, `${command}-data`);
    await mkdir(cwd); assert.equal(await exists(dataDir), false);
    const args = command === 'serve'
      ? ['serve', '--listen', '127.0.0.1:0', '--data-dir', dataDir, '--mcp-files-dir', junction]
      : ['mcp', '--server', api, '--mcp-files-dir', junction];
    const item = ownChild(args, cwd, command);
    await bounded(Promise.race([item.closed, item.failed]), `${command}: native rejection close`);
    assert(Number.isInteger(item.receipt.code) && item.receipt.code !== 0, 'configuration rejection must have an actual nonzero exit');
    assert.equal(item.receipt.signal, null); assert.deepEqual(item.receipt.killSignals, []);
    assert(item.receipt.closed && item.receipt.stdoutEnd && item.receipt.stderrEnd);
    const stdout = Buffer.concat(item.stdout).toString('utf8'), stderr = Buffer.concat(item.stderr).toString('utf8');
    assert(stderr.includes(contract.expectedErrorContext), 'must reject the configured nofollow directory, not another CLI/configuration error');
    assert(!stdout.includes('Open SEO Spider: http://'), 'no candidate socket advertisement');
    if (command === 'mcp') assert.equal(stdout, '', 'no MCP output before configuration validation');
    assert.equal(await exists(dataDir), false); assert.deepEqual(await readdir(cwd), []);
    const after = await witness(outside), linkAfter = await linkIdentity(junction);
    assert.deepEqual(after, result.outsideBefore); assert.deepEqual(linkAfter, result.junctionBefore);
    assert.equal(result.websiteRequests, 0);
    result.cases.push({ name: `${command}: root-junction rejected before data creation/advertisement`, status: 'PASS', args,
      dataDirectoryAbsent: true, cwdEntries: [], outsideAfter: after, junctionAfter: linkAfter });
    await logChild(item);
  }
  await unlink(junction); junctionCreated = false; result.junctionRemoved = true;
  result.outsideAfterUnlink = await witness(outside);
  assert.deepEqual(result.outsideAfterUnlink, result.outsideBefore);
  await sourceGuard();
  result.status = 'PASS_NATIVE_ROOT_JUNCTION_CLI_REJECTION';
} catch (error) {
  result.status = 'FAIL'; result.errors.push({ message: error.message, code: error.code, stack: error.stack }); process.exitCode = 1;
} finally {
  for (const item of children) {
    try {
      if (!item.receipt.closed && item.child.exitCode === null && item.child.signalCode === null) {
        item.receipt.forcedCleanup = true; item.child.kill('SIGKILL');
      }
      if (!item.receipt.closed) await bounded(item.closed, `${item.receipt.label}: own cleanup close`);
      assert(item.receipt.closed && item.receipt.stdoutEnd && item.receipt.stderrEnd);
      await logChild(item);
    } catch (error) { result.status = 'FAIL'; result.errors.push({ cleanup: item.receipt.label, message: error.message }); process.exitCode = 1; }
  }
  if (trap) {
    try { await bounded(new Promise((resolve, reject) => trap.close(error => error ? reject(error) : resolve())), 'own API trap close'); }
    catch (error) { result.status = 'FAIL'; result.errors.push({ cleanup: 'API trap', message: error.message }); process.exitCode = 1; }
  }
  result.websiteTrapClosed = trap ? !trap.listening : null;
  try {
    if (result.outsideBefore) result.outsideAtCleanup = await witness(outside);
    if (junctionCreated && children.every(item => item.receipt.closed)) {
      assert.deepEqual(await linkIdentity(junction), result.junctionBefore);
      await unlink(junction); junctionCreated = false; result.junctionRemoved = true;
    }
    if (children.every(item => item.receipt.closed) && !junctionCreated) {
      for (const name of ['outside', 'serve-cwd', 'mcp-cwd', 'serve-data', 'mcp-data']) {
        const file = below(run, name);
        if (await exists(file)) await rm(file, { recursive: true, force: false });
      }
      result.fixtureCleanup = 'own directories removed only after actual child close and own link removal';
    } else result.fixtureCleanup = 'retained: child or junction cleanup not confirmed';
  } catch (error) { result.status = 'FAIL'; result.errors.push({ cleanup: 'own fixture', message: error.message }); process.exitCode = 1; }
  if (result.websiteRequests !== 0) { result.status = 'FAIL'; result.errors.push({ websiteRequests: result.websiteRequests }); process.exitCode = 1; }
  await writeFile(path.join(run, 'result.json'), `${JSON.stringify(result, null, 2)}\n`, { flag: 'wx' });
  process.stdout.write(`${JSON.stringify({ status: result.status, result: path.join(run, 'result.json') })}\n`);
}
