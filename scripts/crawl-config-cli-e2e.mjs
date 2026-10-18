// Actual normal CLI: raw config remains supported; portable profile envelopes
// must fail before a client can create output/store files or fetch a website.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import {spawn} from 'node:child_process';
import {createHash, randomUUID} from 'node:crypto';
import {fileURLToPath} from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
assert(process.env.SPIDER_BINARY, 'Explicit SPIDER_BINARY is required; this runner never builds or discovers an installed application');
const binary = fs.realpathSync(path.resolve(process.env.SPIDER_BINARY));
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const binaryHash = sha256(fs.readFileSync(binary));
if (process.env.SPIDER_BINARY_SHA256) assert.equal(binaryHash, process.env.SPIDER_BINARY_SHA256);
const scriptHash = sha256(fs.readFileSync(fileURLToPath(import.meta.url)));
const output = path.resolve(process.env.SPIDER_CLI_CONFIG_OUTPUT || path.join(root, 'artifacts', 'cli-config-' + Date.now() + '-' + randomUUID().slice(0, 8)));
assert(!fs.existsSync(output), 'A fresh owned output directory is required');
fs.mkdirSync(output, {recursive: true});
const hits = [], cases = [];
let server, liveChild, failure;

function verifyPins() {
  assert.equal(sha256(fs.readFileSync(binary)), binaryHash, 'Normal CLI binary changed');
  assert.equal(sha256(fs.readFileSync(fileURLToPath(import.meta.url))), scriptHash, 'CLI verification script changed');
}
function tree(directory) {
  if (!fs.existsSync(directory)) return [];
  return fs.readdirSync(directory).sort().flatMap(name => {
    const file = path.join(directory, name), stat = fs.lstatSync(file);
    assert(!stat.isSymbolicLink());
    return stat.isDirectory() ? tree(file).map(entry => ({...entry, path: name + '/' + entry.path})) : [{path: name, bytes: stat.size, sha256: sha256(fs.readFileSync(file))}];
  });
}
async function runCli(args) {
  verifyPins();
  const env = {...process.env, NO_PROXY: '127.0.0.1,localhost', no_proxy: '127.0.0.1,localhost'};
  for (const key of ['HTTP_PROXY', 'HTTPS_PROXY', 'ALL_PROXY', 'http_proxy', 'https_proxy', 'all_proxy', 'OPEN_SEO_PAGESPEED_API_KEY']) delete env[key];
  const child = spawn(binary, args, {cwd: output, env, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe']});
  liveChild = child;
  let stdout = '', stderr = '', forced = false;
  child.stdout.on('data', bytes => { stdout += bytes; });
  child.stderr.on('data', bytes => { stderr += bytes; });
  const timer = setTimeout(() => { forced = true; child.kill(); }, 45000);
  let close;
  try { close = await new Promise((resolve, reject) => { child.once('error', reject); child.once('close', (code, signal) => resolve({code, signal})); }); }
  finally { clearTimeout(timer); liveChild = null; }
  assert.equal(forced, false, 'CLI must terminate naturally within the owned test budget');
  assert.equal(close.signal, null);
  verifyPins();
  return {args, close, stdout, stderr, forced};
}
async function run() {
  server = http.createServer((request, response) => {
    hits.push({method: request.method, path: request.url, user_agent: request.headers['user-agent']});
    if (request.url === '/raw-control' || request.url === '/profile-must-not-fetch') {
      response.writeHead(200, {'content-type': 'text/html;charset=utf-8'});
      response.end('<html lang="en"><head><title>Owned CLI configuration</title></head><body><h1>Local configuration control</h1></body></html>');
    } else { response.writeHead(404); response.end('Owned absent discovery document'); }
  });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  const port = server.address().port;
  assert(![8787, 8790].includes(port));
  const origin = 'http://127.0.0.1:' + port;
  const declared = {concurrency: 1, request_timeout_secs: 5, obey_robots: false, discover_sitemaps: false, crawl_links: false, delay_ms: 0, max_body_bytes: 4097, max_redirects: 0, user_agent: 'OwnedRawConfigCLI/1'};
  const rawInput = path.join(output, 'raw-config.json'), rawReport = path.join(output, 'raw-report.json');
  fs.writeFileSync(rawInput, JSON.stringify(declared));
  const raw = await runCli(['crawl', origin + '/raw-control', '--config', rawInput, '--list', '--max-pages', '1', '--output', rawReport]);
  assert.equal(raw.close.code, 0, raw.stderr);
  const report = JSON.parse(fs.readFileSync(rawReport, 'utf8'));
  for (const [key, value] of Object.entries(declared)) assert.deepEqual(report.config[key], value, 'Raw native setting ' + key);
  assert.deepEqual(report.pages.map(page => page.url), [origin + '/raw-control']);
  assert.deepEqual(hits.map(hit => hit.path), ['/raw-control']);
  assert.equal(hits[0].user_agent, declared.user_agent);
  cases.push({kind: 'raw_native_config', ...raw, report_sha256: sha256(fs.readFileSync(rawReport)), captured_pages: report.pages.length, owned_http_requests: hits.length});

  const profileInput = path.join(output, 'portable-profile.json'), rejectedOutput = path.join(output, 'profile-report-must-not-exist.json'), rejectedStore = path.join(output, 'profile-store-must-not-exist', 'crawl.sqlite');
  const profileBytes = Buffer.from(JSON.stringify({version: 1, config: declared, required_headers: ['X-Owned-Required']}));
  fs.writeFileSync(profileInput, profileBytes);
  const beforeFiles = tree(output), beforeHits = hits.length;
  const rejected = await runCli(['crawl', origin + '/profile-must-not-fetch', '--config', profileInput, '--list', '--max-pages', '1', '--store', rejectedStore, '--output', rejectedOutput]);
  assert.notEqual(rejected.close.code, 0, 'Portable profile must never silently use native defaults');
  assert.match(rejected.stderr, /configuration profiles are not supported by crawl --config/);
  assert.match(rejected.stderr, /browser Settings/);
  assert.match(rejected.stderr, /\/api\/crawls\/from-profile/);
  assert.equal(hits.length, beforeHits, 'Rejected profile must not make any HTTP request');
  assert.deepEqual(tree(output), beforeFiles, 'Rejected profile must not create, replace or mutate product files');
  assert(!fs.existsSync(rejectedOutput));
  assert(!fs.existsSync(path.dirname(rejectedStore)));
  assert.deepEqual(fs.readFileSync(profileInput), profileBytes);
  cases.push({kind: 'portable_profile_explicitly_refused', ...rejected, input_sha256: sha256(profileBytes), additional_http_requests: 0, product_files_unchanged: true, output_absent: true, store_directory_absent: true});
}

try { await run(); }
catch (error) { failure = error; }
finally {
  if (liveChild) liveChild.kill();
  if (server?.listening) { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
  const receipt = {result: failure ? 'FAIL' : 'PASS', binary, binary_sha256: binaryHash, script_sha256: scriptHash, output, scope: 'Two actual normal CLI processes and one owned loopback fixture; legacy raw config supported, portable profile explicitly refused without HTTP/output/store effects; no profile CLI binding capability', cases, website_hits: hits, ...(failure ? {error: failure.stack} : {})};
  fs.writeFileSync(path.join(output, 'receipt.json'), JSON.stringify(receipt, null, 2) + '\n');
}
if (failure) throw failure;
console.log('PASS actual raw CLI config and portable-profile refusal before HTTP/output/store effects');
console.log(output);
