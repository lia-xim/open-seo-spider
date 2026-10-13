import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {existsSync, lstatSync, mkdirSync, readFileSync, writeFileSync} from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawnSync, execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const mode = process.argv[2];
assert(['ledger', 'membership'].includes(mode), 'Select ledger or membership');
assert.equal(process.argv.length, 3, 'No additional runner arguments');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const git = args => execFileSync('git', ['-C', repo, ...args], {encoding: 'utf8'});
const commit = git(['rev-parse', 'HEAD']).trim();
assert.match(process.env.SPIDER_SOURCE_COMMIT || '', /^[a-f0-9]{40}$/);
assert.equal(process.env.SPIDER_SOURCE_COMMIT, commit);
assert.equal(process.env.SPIDER_EXPECTED_SOURCE_COMMIT, commit);
assert(process.env.SPIDER_BINARY, 'Explicit SPIDER_BINARY required');
const binary = path.resolve(repo, process.env.SPIDER_BINARY);
assert(lstatSync(binary).isFile() && !lstatSync(binary).isSymbolicLink());
const binarySha = sha(readFileSync(binary));
if (process.env.SPIDER_BINARY_SHA256) assert.equal(binarySha, process.env.SPIDER_BINARY_SHA256);
const exact = new Set(['Cargo.toml', 'Cargo.lock', 'crates/core/Cargo.toml',
  'crates/app/Cargo.toml', 'docs/parity/reference-headings/fixture-cases.json',
  'docs/parity/resource-matrix-plan.json']);
const prefixes = ['crates/core/src/', 'crates/core/tests/', 'crates/app/src/', 'assets/fonts/', 'web/'];
const sourcePaths = git(['ls-files', '-z']).split('\0').filter(Boolean)
  .filter(name => exact.has(name) || prefixes.some(prefix => name.startsWith(prefix))).sort();
for (const filename of exact) assert(sourcePaths.includes(filename), 'Missing source closure: ' + filename);
git(['diff', '--quiet', 'HEAD', '--', ...sourcePaths]);
const sourceFiles = sourcePaths.map(filename => {
  const full = path.join(repo, filename), info = lstatSync(full);
  assert(info.isFile() && !info.isSymbolicLink());
  const bytes = readFileSync(full);
  return {path: filename, bytes: bytes.length, sha256: sha(bytes)};
});
const harness = path.join(repo, 'scripts', `sitemap-${mode}-e2e.mjs`);
const harnessSha = sha(readFileSync(harness));
if (process.env.CI === 'true') {
  assert(git(['ls-files', '--', path.relative(repo, harness)]).trim());
  git(['diff', '--quiet', 'HEAD', '--', path.relative(repo, harness), 'scripts/run-sitemap-native-checks.mjs']);
}
const out = path.resolve(process.env.SPIDER_SITEMAP_NATIVE_OUTPUT ||
  path.join(process.env.RUNNER_TEMP || os.tmpdir(), `sitemap-${mode}-${Date.now()}-${process.pid}`));
assert(!existsSync(out), 'Native output must be new: ' + out);
mkdirSync(out, {recursive: true});
const manifest = {schema_version: 1, source_commit: commit, files: sourceFiles,
  qualification: 'Actual clean tracked Cargo/fixture/asset inputs. Binary hash pins this run; independent compiler or CI build receipts establish source-to-binary provenance.'};
const manifestPath = path.join(out, 'source-manifest.json');
const write = (filename, value) => writeFileSync(path.join(out, filename), JSON.stringify(value, null, 2) + '\n', {flag: 'wx'});
write('source-manifest.json', manifest);
const manifestSha = sha(readFileSync(manifestPath));
const env = {...process.env, SPIDER_BINARY: binary, SPIDER_BINARY_SHA256: binarySha,
  SPIDER_SITEMAP_LEDGER_PROOF: path.join(out, 'ledger.json')};
const args = [harness];
if (mode === 'membership') {
  assert(process.env.OPEN_SEO_BROWSER_PATH, 'Explicit installed Chromium executable required');
  args.push('--binary', binary, '--binary-sha256', binarySha, '--source-root', repo,
    '--source-manifest', manifestPath, '--source-manifest-sha256', manifestSha,
    '--runtime-root', repo, '--out', path.join(out, 'membership'),
    '--chrome', path.resolve(process.env.OPEN_SEO_BROWSER_PATH));
}
write('inputs.json', {mode, source_commit: commit, binary, binary_sha256: binarySha,
  binary_bytes: lstatSync(binary).size, harness_sha256: harnessSha,
  source_manifest_sha256: manifestSha, args, started_at: new Date().toISOString(),
  source_to_binary_provenance: 'Separate build receipt required; never inferred from this runtime test'});
const started = Date.now();
const run = spawnSync(process.execPath, args, {cwd: repo, env, windowsHide: true,
  encoding: 'utf8', timeout: 300000, maxBuffer: 8 * 1024 * 1024});
writeFileSync(path.join(out, 'stdout.log'), run.stdout || '', {flag: 'wx'});
writeFileSync(path.join(out, 'stderr.log'), run.stderr || '', {flag: 'wx'});
assert.equal(sha(readFileSync(binary)), binarySha, 'Binary changed during native run');
assert.equal(sha(readFileSync(harness)), harnessSha, 'Harness changed during native run');
for (const source of sourceFiles) assert.equal(sha(readFileSync(path.join(repo, source.path))), source.sha256, source.path);
const proofPath = mode === 'ledger' ? path.join(out, 'ledger.json') : path.join(out, 'membership/proof.json');
const proof = existsSync(proofPath) ? JSON.parse(readFileSync(proofPath)) : null;
const expected = mode === 'ledger' ? 'PASS' : 'PASS_NATIVE_SITEMAP_API_SDK_BROWSER';
write('result.json', {mode, exit_code: run.status, signal: run.signal, error: run.error?.message,
  result: proof?.result || 'NO_NATIVE_PROOF', elapsed_ms: Date.now() - started,
  binary_sha256: binarySha, harness_sha256: harnessSha, source_manifest_sha256: manifestSha,
  proof_sha256: proof ? sha(readFileSync(proofPath)) : null,
  stdout_sha256: sha(readFileSync(path.join(out, 'stdout.log'))),
  stderr_sha256: sha(readFileSync(path.join(out, 'stderr.log'))),
  qualification: 'Inspect raw companions and final drain; exit code alone is not acceptance.'});
console.log(JSON.stringify({mode, result: proof?.result, output: out}));
assert.equal(run.status, 0, (run.stderr || run.stdout || run.error?.message || '').slice(-4000));
assert.equal(proof?.result, expected);
assert.equal(proof?.binary_sha256, binarySha);
assert.equal(proof?.harness_sha256, harnessSha);
assert.equal(mode === 'ledger' ? proof.scenarios.length : proof.checks.length, 11);
