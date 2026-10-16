import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
export const CAP = 64 * 1024 * 1024, JSON_CAP = 8 * 1024 * 1024;
export const sha = bytes => createHash('sha256').update(bytes).digest('hex');
export function ordinary(p) {
  for (p = path.resolve(p); ; p = path.dirname(p)) {
    assert(!fs.lstatSync(p).isSymbolicLink(), p);
    if (path.dirname(p) === p) break;
  }
}
export function fileHash(p) { ordinary(p); const b = fs.readFileSync(p); return { sha256: sha(b), bytes: b.length }; }
export function save(p, value) {
  const fd = fs.openSync(p, 'wx');
  try { fs.writeFileSync(fd, JSON.stringify(value, null, 2) + '\n'); fs.fsyncSync(fd); }
  finally { fs.closeSync(fd); }
}
function verify(r) {
  assert(r && path.isAbsolute(r.path)); assert.match(r.sha256, /^[a-f0-9]{64}$/);
  const actual = fileHash(r.path); assert.equal(actual.sha256, r.sha256, r.path);
  assert.equal(actual.bytes, r.bytes, r.path);
}
export async function admit(HERE) {
  const args = process.argv.slice(2);
  assert.deepEqual([args[0], args[2]], ['--release', '--release-sha256']); assert.equal(args.length, 4);
  assert.equal(process.platform, 'win32'); assert.deepEqual(process.execArgv, []);
  assert.equal(process.env.NODE_OPTIONS, undefined); assert.equal(process.env.NODE_PATH, undefined);
  assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed');
  assert(path.isAbsolute(args[1])); assert.match(args[3], /^[a-f0-9]{64}$/);
  const task = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
  assert.equal(path.dirname(path.resolve(args[1])), path.resolve(task, 'work'));
  assert.match(path.basename(args[1]), /^v12-ngram-metadata-native-runtime2-release\.json$/);
  ordinary(args[1]); assert(fs.statSync(args[1]).size <= 128 * 1024);
  const releaseRaw = fs.readFileSync(args[1]); assert.equal(sha(releaseRaw), args[3]);
  const R = JSON.parse(releaseRaw), BASE = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
  assert.equal(R.kind, 'root_released_source12_ngram_metadata_native_runtime2');
  assert.equal(R.status, 'RELEASED'); assert.equal(R.issued_by, 'Root');
  assert.equal(R.scope, 'eight_genuine_html_metadata_channels_wordcloud_api_cli_ui_cold');
  assert.equal(R.execution_lock_mode, 'harness_owns_shared_native_lock');
  assert.match(R.lock_token, /^[a-f0-9]{32,128}$/);
  assert(typeof R.user_authorization_reference === 'string' && R.user_authorization_reference.includes('Continue until all matches'));
  const start = Date.parse(R.not_before), end = Date.parse(R.expires_at);
  function live() {
    assert(Number.isFinite(start) && Number.isFinite(end) && start <= Date.now() && Date.now() < end && end - start <= 7200000);
    assert(fs.readFileSync(args[1]).equals(releaseRaw));
  }
  live();
  for (const key of ['source_manifest_sha256', 'suite_proof_sha256', 'build_proof_sha256', 'candidate_binary_sha256', 'build_driver_sha256', 'preparation_manifest_sha256']) assert.match(R[key], /^[a-f0-9]{64}$/);
  const manifestPath = path.join(HERE, 'manifest.json');
  assert.equal(fileHash(manifestPath).sha256, R.preparation_manifest_sha256);
  const manifest = JSON.parse(fs.readFileSync(manifestPath));
  assert.equal(manifest.kind, 'source12_ngram_metadata_native_runtime2_source_preparation');
  assert.equal(manifest.status, 'AUTHORED_NOT_RUN');
  for (const f of manifest.payloads) {
    assert(typeof f.path === 'string' && !path.isAbsolute(f.path) && !f.path.includes('..') && !f.path.includes('\\'));
    verify({ ...f, path: path.join(HERE, f.path) });
  }
  verify(R.independent_review);
  const review = JSON.parse(fs.readFileSync(R.independent_review.path));
  assert.equal(review.result, 'NO_FINDINGS_BOUNDED_SOURCE_REVIEW'); assert.deepEqual(review.findings, []);
  assert.equal(review.kind, 'source12_ngram_metadata_native_independent_review');
  assert.equal(path.resolve(review.proposal_manifest.path), path.resolve(manifestPath));
  assert.equal(review.proposal_manifest.sha256, R.preparation_manifest_sha256);
  assert.equal(review.harness.sha256, manifest.payloads.find(f => f.path === 'native-ngram-channels.mjs').sha256);
  assert.equal(path.resolve(review.harness.path), path.join(HERE, 'native-ngram-channels.mjs'));
  const P = JSON.parse(fs.readFileSync(path.join(HERE, 'PINNED.json')));
  for (const r of Object.values(P.refs)) verify(r);
  assert.equal(R.source_manifest_sha256, P.refs.source_manifest.sha256);
  assert.equal(R.build_driver_sha256, P.refs.builder.sha256);
  const runtimeAdmissionPath="E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source12-browser-five-runtime2-preparation-v4\\runtime2-admission.mjs",runtimeAdmissionSHA='664162e5637bfb8df9e24484639dd8b61ddb697518f24746cbd34e7eaa5b5583';assert.equal(sha(fs.readFileSync(runtimeAdmissionPath)),runtimeAdmissionSHA);const{admitBrowserRuntime}=await import(pathToFileURL(runtimeAdmissionPath));assert.equal(sha(fs.readFileSync(runtimeAdmissionPath)),runtimeAdmissionSHA);const runtime2=await admitBrowserRuntime(R);
  verify(R.node); assert.equal(path.resolve(R.node.path),path.resolve(process.execPath));verify(R.chrome);
  assert(path.isAbsolute(R.playwright_root)); ordinary(R.playwright_root);
  assert.deepEqual(Object.keys(R.playwright_package).sort(), ['bytes', 'sha256']);
  verify({ ...R.playwright_package, path: path.join(R.playwright_root, 'package.json') });
  const OUTPUT = path.resolve(R.output);
  assert.equal(path.dirname(OUTPUT), path.resolve(BASE, 'results'));
  assert.equal(path.basename(OUTPUT),'ngram-metadata-native-v12-runtime2');
  assert(!fs.existsSync(OUTPUT)); ordinary(path.dirname(OUTPUT));
  const lockPath = path.join(BASE, 'results/combined-native-outer-v4/active.lock');
  assert(!fs.existsSync(lockPath), 'Actual shared native lock must be free'); ordinary(path.dirname(lockPath));
  const binding=runtime2.binding;
  assert.equal(binding.manifest.source_commit, P.source_commit);
  assert.equal(binding.manifest.kind, 'combined_source_v12'); assert.equal(binding.manifest.source_inputs, 381);
  assert.equal(binding.manifest.required_test_names, 646); assert.equal(binding.manifest.all_required_full_test_names.length, 650);
  assert(binding.actualWorkspace.passed >= 1428); assert.equal(binding.binarySHA, R.candidate_binary_sha256);
  assert(Number.isSafeInteger(R.candidate_bytes) && R.candidate_bytes > 0); assert.equal(fileHash(binding.binary).bytes, R.candidate_bytes);
  const runtimeRoot = path.resolve(R.renderer_runtime_root);
  assert.equal(runtimeRoot, path.resolve('C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https/outputs/open-seo-spider'));
  assert.equal(path.resolve(R.playwright_root), path.join(runtimeRoot, 'renderer/node_modules/playwright-core'));
  assert.deepEqual(Object.keys(R.axe_package).sort(), ['bytes', 'sha256']);
  verify({ ...R.axe_package, path: path.join(runtimeRoot, 'renderer/node_modules/axe-core/package.json') });
  const rendererFiles = binding.manifest.files.filter(f => f.path.startsWith('renderer/'));
  assert.equal(rendererFiles.length, 4);
  function check() {
    live(); runtime2.verify();
    for (const r of Object.values(P.refs)) verify(r);
    for (const f of manifest.payloads) verify({ ...f, path: path.join(HERE, f.path) });
    for (const f of rendererFiles) verify({ ...f, path: path.join(runtimeRoot, f.path) });
    verify(R.node); verify(R.chrome);
    verify({ ...R.playwright_package, path: path.join(R.playwright_root, 'package.json') });
    verify({ ...R.axe_package, path: path.join(runtimeRoot, 'renderer/node_modules/axe-core/package.json') });
  }
  check();
  return { R, P, OUTPUT, BASE, binding, runtimeWitness:runtime2.witness, runtimeRoot, rendererFiles, verify: check, manifest, release: { path: args[1], sha256: args[3] } };
}
