import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile, writeFile, mkdir, readdir, stat } from 'node:fs/promises';
import { resolve, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = resolve(root, 'docs/parity/reference-fixture');
const executable = process.env.OPEN_SEO_REFERENCE_CLI ?? 'C:/Program Files (x86)/Screaming Frog SEO Spider/ScreamingFrogSEOSpiderCli.exe';
const profile = resolve(process.env.USERPROFILE ?? '', '.ScreamingFrogSEOSpider');
const configPath = resolve(profile, 'spider.config');
const sha = data => createHash('sha256').update(data).digest('hex');
async function fileHash(path) { try { return sha(await readFile(path)); } catch (error) { if (error.code === 'ENOENT') return null; throw error; } }
async function exists(path) { try { await stat(path); return true; } catch (error) { if (error.code === 'ENOENT') return false; throw error; } }
export const pages = new Map();
const baseTitle = 'An independently authored local SEO reference fixture';
function page(path, { title = baseTitle + ' ' + path, description = 'A useful independently authored description for the local fixture ' + path, head = '', body = '', headers = {}, status = 200 } = {}) {
  pages.set(path, { status, headers, html: `<!doctype html><html lang="en"><head><meta charset="utf-8">${title === null ? '' : `<title>${title}</title>`}${description === null ? '' : `<meta name="description" content="${description}">`}${head}</head><body><h1>${path === '/' ? 'Fixture index' : path}</h1><p>${'This is controlled local fixture text for comparing crawler observations. '.repeat(8)}</p>${body}</body></html>` });
}
page('/baseline'); page('/missing-title', { title: null }); page('/empty-title', { title: '' }); page('/blank-title', { title: '   ' });
page('/duplicate-a', { title: 'A duplicate title for both controlled pages', description: 'The same description for two controlled pages.' });
page('/duplicate-b', { title: 'A duplicate title for both controlled pages', description: 'The same description for two controlled pages.' });
page('/long-title', { title: 'W'.repeat(75) }); page('/short-title', { title: 'Tiny' });
page('/multiple-title', { head: '<title>Second controlled title</title>' });
page('/missing-description', { description: null }); page('/empty-description', { description: '' });
page('/multiple-description', { head: '<meta name="description" content="Second controlled description">' });
page('/body-metadata', { title: null, description: null, body: '<title>Title in the body</title><meta name="description" content="Description in the body">' });
page('/noindex', { head: '<meta name="robots" content="noindex, follow">' });
page('/header-noindex', { headers: { 'X-Robots-Tag': 'noindex, nofollow' } });
page('/canonical-self', { head: '<link rel="canonical" href="/canonical-self">' });
page('/canonical-other', { head: '<link rel="canonical" href="/baseline">' });
page('/canonical-conflict', { head: '<link rel="canonical" href="/baseline"><link rel="canonical" href="/noindex">' });
page('/canonical-404', { head: '<link rel="canonical" href="/missing">' });
page('/lang-en', { head: '<link rel="alternate" hreflang="en" href="/lang-en"><link rel="alternate" hreflang="de" href="/lang-de">' });
page('/lang-de', { head: '<link rel="alternate" hreflang="de" href="/lang-de"><link rel="alternate" hreflang="en" href="/lang-en">' });
page('/links', { body: '<a href="/missing">Broken target</a><a href="/baseline">Repeated one</a><a href="/baseline" rel="nofollow">Repeated two</a><a href="/baseline"></a><a href="/redirect-one">Redirect</a><a href="/blocked">Blocked</a><a href="/baseline#absent">Absent fragment</a>' });
page('/images', { body: '<img src="/image.png"><img src="/image.png" alt=""><img src="/image.png" alt="A controlled accessible example" width="1" height="1">' });
page('/jsonld-invalid', { head: '<script type="application/ld+json">{"@type":"Article",}</script>' });
page('/jsonld-valid', { head: '<script type="application/ld+json">{"@context":"https://schema.org","@type":"Article","headline":"Independent fixture article"}</script>' });
page('/soft-error', { body: '<h2>404 Page Not Found</h2>' });
page('/Upper_Case', { body: '<a href="/baseline?a=1&amp;b=2">Parameters</a>' });
page('/blocked'); page('/orphan-only');
page('/', { body: [...pages.keys()].filter(path => path !== '/orphan-only').map(path => `<a href="${path}">${path}</a>`).join('\n') + '<a href="/redirect-one">Redirect chain</a><a href="/missing">Missing resource</a><a href="/server-error">Server error</a>' });
export async function startFixture({ port = 0 } = {}) {
const requests = [];
let origin;
const server = createServer((request, response) => {
  const target = new URL(request.url, origin);
  requests.push({ path: target.pathname + target.search, method: request.method, user_agent: request.headers['user-agent'] ?? null });
  response.setHeader('Content-Type', 'text/html; charset=utf-8');
  response.setHeader('X-Content-Type-Options', 'nosniff');
  if (target.pathname === '/robots.txt') { response.setHeader('Content-Type', 'text/plain'); return response.end(`User-agent: *\nDisallow: /blocked\nSitemap: ${origin}/sitemap.xml\n`); }
  if (target.pathname === '/sitemap.xml') { response.setHeader('Content-Type', 'application/xml'); return response.end(`<?xml version="1.0"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>${origin}/baseline</loc></url><url><loc>${origin}/orphan-only</loc></url></urlset>`); }
  if (target.pathname === '/image.png') { response.setHeader('Content-Type', 'image/png'); return response.end(Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jXnQAAAAASUVORK5CYII=', 'base64')); }
  if (target.pathname === '/redirect-one' || target.pathname === '/redirect-two') { response.statusCode = 301; response.setHeader('Location', target.pathname === '/redirect-one' ? '/redirect-two' : '/baseline'); return response.end(); }
  if (target.pathname === '/missing') { response.statusCode = 404; return response.end('<!doctype html><title>Real missing page</title>'); }
  if (target.pathname === '/server-error') { response.statusCode = 503; return response.end('<!doctype html><title>Controlled 503</title>'); }
  const fixture = pages.get(target.pathname);
  if (!fixture) { response.statusCode = 404; return response.end('Unknown controlled fixture URL'); }
  response.statusCode = fixture.status;
  for (const [key, value] of Object.entries(fixture.headers)) response.setHeader(key, value);
  response.end(fixture.html);
});
await new Promise(resolveListen => server.listen(port, '127.0.0.1', resolveListen));
origin = `http://127.0.0.1:${server.address().port}`;
return { server, origin, requests, pages };
}

export async function runReferenceCli(fixture) {
  const { origin, requests } = fixture;
  const runOutput = resolve(output, new Date().toISOString().replace(/[:.]/g, '-'));
  await mkdir(runOutput, { recursive: true });
  const before = await fileHash(configPath);
  const args = ['--headless', '--crawl', origin + '/', '--output-folder', runOutput, '--export-format', 'csv', '--export-tabs', 'Internal:All,Response Codes:All,Page Titles:All,Meta Description:All,H1:All,H2:All,Canonicals:All,Directives:All,Images:All,URL:All,Links:All'];
  const run = await new Promise((resolveRun, reject) => {
    const child = spawn(executable, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    const stdout = [], stderr = [];
    let timedOut = false;
    const timer = setTimeout(() => { timedOut = true; child.kill(); }, 90000);
    child.stdout.on('data', data => stdout.push(data)); child.stderr.on('data', data => stderr.push(data));
    child.on('error', error => { clearTimeout(timer); reject(error); });
    child.on('close', code => { clearTimeout(timer); resolveRun({ exit_code: code, timed_out: timedOut, stdout: new TextDecoder('windows-1252').decode(Buffer.concat(stdout)), stderr: new TextDecoder('windows-1252').decode(Buffer.concat(stderr)) }); });
  });
  const files = await readdir(runOutput);
  const exports = [];
  for (const name of files.filter(name => name.endsWith('.csv'))) {
    const contents = await readFile(resolve(runOutput, name));
    exports.push({ name, size_bytes: contents.length, sha256: sha(contents) });
  }
  const after = await fileHash(configPath);
  const diagnosticLines = (run.stdout + run.stderr).split(/\r?\n/).filter(line => /Licence Status:|Platform Info:|Java Info:|mConfigVersion=|Completed writing|Application Exited|Exception|licen[cs]e.*required|lizenz.*erforderlich/i.test(line));
  const result = { schema_version: 1, observed_at: new Date().toISOString(), fixture_origin: origin, fixture_pages: pages.size,
    upper_bound_distinct_paths: pages.size + 8, executable: basename(executable), arguments: args,
    existing_licence_file: await exists(resolve(profile, 'licence.txt')), licence_contents_read: false,
    configuration_sha256: { before, after, unchanged: before === after },
    output_directory: 'docs/parity/reference-fixture/' + basename(runOutput), exports,
    requested_tabs: args.at(-1).split(','), exit_code: run.exit_code, timed_out: run.timed_out, diagnostic_lines: diagnosticLines, requests,
    status: exports.length > 0 && run.exit_code === 0 ? 'REFERENCE_EXPORT_OBSERVED' : /licen[cs]|lizenz/i.test(run.stdout + run.stderr) ? 'LICENSE_BLOCKED' : 'REFERENCE_EXPORT_NOT_PROVEN',
    parity_status: 'NOT_PROVEN', notes: ['Original CLI was invoked normally under existing configuration without license/config/auth/save-crawl flags.', 'Local fixture URLs only; no owned or third-party website was supplied.', 'Original GUI session was not controlled or changed. CLI may create its own ordinary working files; its config hash is checked.', 'CSV exports are reference observations, not proof of implementation parity.'] };
  await writeFile(resolve(output, 'latest-run.json'), JSON.stringify(result, null, 2) + '\n');
  return result;
}

if (resolve(process.argv[1] ?? '') === fileURLToPath(import.meta.url)) {
  const fixture = await startFixture({ port: Number(process.env.OPEN_SEO_REFERENCE_PORT ?? 0) });
  if (!process.argv.includes('--run-reference')) {
    console.log(JSON.stringify({ mode: 'serve-only', origin: fixture.origin, fixture_pages: pages.size, upper_bound_distinct_paths: pages.size + 8 }));
    const close = () => fixture.server.close(); process.once('SIGINT', close); process.once('SIGTERM', close);
  } else {
    try {
      const result = await runReferenceCli(fixture);
      console.log(JSON.stringify({ status: result.status, exit_code: result.exit_code, exports: result.exports.map(item => item.name), requests: result.requests.length, configuration_unchanged: result.configuration_sha256.unchanged, diagnostic_lines: result.diagnostic_lines }));
    } finally { await new Promise(resolveClose => fixture.server.close(resolveClose)); }
  }
}
