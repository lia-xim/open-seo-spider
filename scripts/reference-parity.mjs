import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { startFixture, runReferenceCli } from './reference-fixture.mjs';
import { normalizeReference } from './reference-normalize.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = resolve(root, 'docs/parity');
const binary = resolve(process.env.SPIDER_BINARY ?? resolve(root, 'target/release', `open-seo-spider${process.platform === 'win32' ? '.exe' : ''}`));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const delay = ms => new Promise(resolveDelay => setTimeout(resolveDelay, ms));
const fixture = await startFixture();
let child;
let openLogs = '';
let data;
async function freePort() { const probe = createServer(); await new Promise(resolveListen => probe.listen(0, '127.0.0.1', resolveListen)); const port = probe.address().port; await new Promise(resolveClose => probe.close(resolveClose)); return port; }
async function stop() {
  if (!child || child.exitCode !== null) return;
  const exited = new Promise(resolveExit => child.once('exit', resolveExit)); child.kill(); await exited;
}
function fieldReasons(field) {
  const reasons = { title_pixels: 'Recorded font-based metrics need calibration for glyph advances, kerning and rounding; an absent native metric remains unavailable.',
    inlinks: 'Reference source/link occurrence and resource accounting can differ; require lower-link exports to calibrate.',
    outlinks: 'Comparable anchor occurrences only; total non-anchor references remain separate independent facts.',
    unique_inlinks: 'Reference graph identity and resource inclusion need calibration.', unique_outlinks: 'Reference graph identity and resource inclusion need calibration.',
    word_count: 'Content-area/tokenization must be calibrated against reference; metadata-only proxies are insufficient.',
    indexability_reason: 'Different reason precedence/coverage may explain mismatches; both raw reason lists are retained.',
    title_occurrences: 'Empty-element preservation and initial versus final redirect document extraction can differ.',
    canonical_occurrences: 'Source-aware canonical count can differ from merged target count.' };
  return reasons[field] ?? 'Observed field mismatch requires an implementation or reference-scope decision; not waived.';
}
try {
  await mkdir(output, { recursive: true });
  const fixtureCases = [...fixture.pages].map(([path, value]) => ({ id: 'local' + path.replace(/[^a-z0-9]+/gi, '-'), path,
    response: { status: value.status, headers: value.headers, html: value.html }, html_sha256: hash(value.html) }));
  const manifest = { schema_version: 1, authored_by: 'Open SEO Spider independent fixture', html_cases: fixtureCases,
    transport_cases: [
      {path:'/robots.txt',status:200,content_type:'text/plain',body:'User-agent: *\nDisallow: /blocked\nSitemap: {origin}/sitemap.xml\n'},
      {path:'/sitemap.xml',status:200,content_type:'application/xml',urls:['/baseline','/orphan-only']},
      {path:'/image.png',status:200,content_type:'image/png',width:1,height:1},
      {path:'/redirect-one',status:301,location:'/redirect-two'}, {path:'/redirect-two',status:301,location:'/baseline'},
      {path:'/missing',status:404,html:'<!doctype html><title>Real missing page</title>'},
      {path:'/server-error',status:503,html:'<!doctype html><title>Controlled 503</title>'}
    ], maximum_distinct_paths: 38, source_script: 'scripts/reference-fixture.mjs' };
  manifest.fixture_sha256 = hash(JSON.stringify({ html_cases: manifest.html_cases, transport_cases: manifest.transport_cases }));
  await writeFile(resolve(output, 'fixture-cases.json'), JSON.stringify(manifest, null, 2) + '\n');
  let referenceRun;
  if (process.argv.includes('--reuse-reference')) referenceRun = JSON.parse(await readFile(resolve(output, 'reference-fixture/latest-run.json'), 'utf8'));
  else referenceRun = await runReferenceCli(fixture);
  assert.equal(referenceRun.status, 'REFERENCE_EXPORT_OBSERVED', 'Reference export must succeed; license errors stay blockers.');
  if (process.argv.includes('--reuse-reference')) assert.equal(referenceRun.fixture_sha256, manifest.fixture_sha256, 'Retained reference must be bound to the identical fixture signature.');
  else { referenceRun.fixture_sha256 = manifest.fixture_sha256; await writeFile(resolve(output, 'reference-fixture/latest-run.json'), JSON.stringify(referenceRun,null,2)+'\n'); }
  const reference = await normalizeReference(referenceRun);
  const originalRequests = [...referenceRun.requests];
  fixture.requests.length = 0;
  data = await mkdtemp(resolve(tmpdir(), 'open-seo-spider-reference-'));
  const port = await freePort();
  const api = `http://127.0.0.1:${port}`;
  child = spawn(binary, ['serve', '--listen', `127.0.0.1:${port}`, '--data-dir', data], { cwd: root, windowsHide: true, stdio: ['ignore','pipe','pipe'] });
  child.stdout.on('data', value => { openLogs += value; }); child.stderr.on('data', value => { openLogs += value; });
  child.on('error', error => { openLogs += error.message; });
  async function apiJson(route, options = {}) {
    const response = await fetch(api + route, { ...options, headers: {'content-type':'application/json', ...options.headers} });
    const body = await response.text();
    if (!response.ok) throw new Error(`API ${route}: ${response.status} ${body}`);
    return body ? JSON.parse(body) : null;
  }
  async function until(check, label) {
    const deadline = Date.now() + 30000;
    while (Date.now() < deadline) { if (child.exitCode !== null) throw new Error(`Native server exited ${child.exitCode}: ${openLogs.slice(-1000)}`); const result = await check(); if (result) return result; await delay(50); }
    throw new Error(`Timed out ${label}: ${openLogs.slice(-1000)}`);
  }
  await until(async () => { try { return (await apiJson('/healthz')).ok; } catch { return false; } }, 'native health');
  const config = { seed_urls:[fixture.origin + '/'], discover_sitemaps:false, obey_robots:true, check_external:false,
    crawl_links:true, concurrency:4, delay_ms:0, max_pages:null, renderer:null };
  const job = await apiJson('/api/crawls', {method:'POST',body:JSON.stringify(config)});
  const finished = await until(async () => { const value = await apiJson('/api/crawls/' + job.id); return ['completed','failed','cancelled'].includes(value.status) && value; }, 'native crawl');
  assert.equal(finished.status, 'completed');
  const native = await apiJson(`/api/crawls/${job.id}/project.json`);
  const nativeAnalysis = await apiJson(`/api/crawls/${job.id}/analysis`);
  const urlPath = url => { try { const parsed = new URL(url); return parsed.origin === fixture.origin ? parsed.pathname + parsed.search + parsed.hash : url; } catch { return url; } };
  const nativePages = native.pages.map(page => ({path:urlPath(page.url),http_status:page.status ?? null,
    content_type:page.content_type.split(';')[0].trim().toLowerCase(),indexability:page.indexable ? 'indexable':'non-indexable',
    indexability_reason:page.indexability_reasons.join(', '),indexability_reasons:page.indexability_reasons,
    title:page.titles[0]??'',title_occurrences:page.titles.length,title_length:[...(page.titles[0]??'')].length,title_pixels:page.title_pixel_widths?.[0]??null,second_title:page.titles[1]??'',
    description:page.meta_descriptions[0]??'',description_length:[...(page.meta_descriptions[0]??'')].length,
    description_occurrences:page.meta_descriptions.length,second_description:page.meta_descriptions[1]??'',
    h1:page.h1[0]??'',h2:page.h2[0]??'',robots_meta:page.meta_robots[0]??'',robots_header:page.x_robots_tag[0]??'',
    canonical:urlPath(page.canonical[0]??''),second_canonical:urlPath(page.canonical[1]??''),canonical_occurrences:page.canonical.length,
    depth:page.depth,outlinks:page.links.filter(link=>!link.kind||link.kind==='anchor').length,unique_outlinks:new Set(page.links.filter(link=>!link.kind||link.kind==='anchor').map(link=>link.url)).size,
    total_reference_occurrences:page.links.length,reference_kind_counts:Object.fromEntries([...new Set(page.links.map(link=>link.kind||'legacy_anchor'))].map(kind=>[kind,page.links.filter(link=>(link.kind||'legacy_anchor')===kind).length])),
    redirect_to:urlPath(page.redirect_chain[0]?.target??''),word_count:page.word_count,
    inlinks:null,unique_inlinks:null,blocked_by_robots:page.blocked_by_robots})).sort((a,b)=>a.path.localeCompare(b.path));
  for (const page of nativePages) {
    const node = nativeAnalysis.graph.nodes.find(node => urlPath(node.url) === page.path);
    page.inlinks = node?.inlink_occurrences ?? null; page.unique_inlinks = node?.in_degree ?? null;
  }
  const referencePages = reference.observations.map(page => ({...page, content_type:page.content_type.split(';')[0].trim().toLowerCase(),
    indexability:page.indexability === 'Indexierbar' ? 'indexable' : 'non-indexable',
    indexability_reason:page.indexability_reason.toLowerCase().replace('canonicalised','canonicalized').replace('blocked by robots.txt','blocked by robots'),
    robots_meta:page.robots_meta.toLowerCase(),robots_header:page.robots_header.toLowerCase()}));
  const nativeByPath = new Map(nativePages.map(page=>[page.path,page])); const referenceByPath = new Map(referencePages.map(page=>[page.path,page]));
  const fields = ['http_status','content_type','indexability','indexability_reason','title','title_occurrences','title_length','title_pixels','second_title','description','description_occurrences','description_length','second_description','h1','h2','robots_meta','robots_header','canonical','second_canonical','canonical_occurrences','depth','inlinks','unique_inlinks','outlinks','unique_outlinks','redirect_to','word_count'];
  const comparisons = [];
  for (const path of [...new Set([...nativeByPath.keys(),...referenceByPath.keys()])].sort()) {
    const independent = nativeByPath.get(path), original = referenceByPath.get(path);
    if (!independent || !original) { comparisons.push({path,field:'coverage',status:'DIFFERENT',reference:!!original,independent:!!independent,reason:'Discovery/resource policy differs; all missing and additional paths remain visible.'}); continue; }
    for (const field of fields) {
      let actual = independent[field];
      if (field === 'indexability_reason') actual = actual.toLowerCase().replaceAll('canonicalised','canonicalized').replaceAll('blocked_by_robots','blocked by robots');
      const expected = original[field];
      const status = actual === null && expected !== null ? 'UNAVAILABLE' : JSON.stringify(actual) === JSON.stringify(expected) ? 'MATCH' : 'DIFFERENT';
      comparisons.push({path,field,status,reference:expected,independent:actual,...(status==='MATCH'?{}:{reason:fieldReasons(field)})});
    }
  }
  const counts = Object.fromEntries(['MATCH','DIFFERENT','UNAVAILABLE'].map(status=>[status,comparisons.filter(value=>value.status===status).length]));
  const result = {schema_version:1,tested_at:new Date().toISOString(),status:'COMPARISON_EXECUTED',full_parity:'NOT_PROVEN',fixture_sha256:manifest.fixture_sha256,
    reference:{version:'24.3',run_at:referenceRun.observed_at,licence_state:'Missing',executable:referenceRun.executable,normal_profile_write:!referenceRun.configuration_sha256.unchanged,rows:referencePages.length},
    independent:{binary:basename(binary),binary_sha256:hash(await readFile(binary)),config:native.config??finished.config,rows:nativePages.length},
    normalization:['Both crawls use the same live local fixture unless --reuse-reference explicitly requests the retained identical case definition.','Fixture origin becomes path; Content-Type parameters omitted; equivalent localized indexability names mapped.','Crawl timestamps, wall-clock latency, job IDs and cache-related headers excluded.','Outlinks comparisons count actual anchor/legacy-anchor records; total references by kind remain separate independent facts.','Recorded title_pixel_widths[0] is compared when available; absent metric stays unavailable.','No semantic mismatches are suppressed. Pixel/word/link-model metrics remain visible as unavailable or different.'],
    reused_reference:process.argv.includes('--reuse-reference'),counts,comparisons,
    requests:{reference:originalRequests.map(request=>({path:request.path,method:request.method})),independent:fixture.requests.map(request=>({path:request.path,method:request.method}))},
    limitations:['These compare exported fields, not vendor issue-filter membership.','A native release hash binds findings to this build; rerun after implementation changes.','No paid rendering/auth/provider configuration or >500 URL reference behavior was tested.']};
  await writeFile(resolve(output,'reference-comparison.json'),JSON.stringify(result,null,2)+'\n');
  await writeFile(resolve(output,'independent-observations.json'),JSON.stringify({schema_version:1,fixture_sha256:manifest.fixture_sha256,pages:nativePages,issues:native.issues??[],config:native.config??finished.config},null,2)+'\n');
  const differences = comparisons.filter(value=>value.status!=='MATCH');
  const lines = ['# Local reference comparison','',`Run: ${result.tested_at}. Full product parity: **NOT PROVEN**.`,``,`${referencePages.length} reference rows / ${nativePages.length} independent rows; ${counts.MATCH} matching fields, ${counts.DIFFERENT} differing fields, ${counts.UNAVAILABLE} unavailable fields.`,
    '',`Fixture SHA-256: \`${manifest.fixture_sha256}\`. Native binary SHA-256: \`${result.independent.binary_sha256}\`.`,
    '', 'The original normal CLI reported a missing licence and successfully exported eleven CSV tabs for this controlled fixture. The original CLI rewrites its ordinary profile on shutdown; no licence/config/auth flag or setting change was supplied.',
    '', 'Comparisons are field-level observations. They do not prove issue-filter membership, paid features or complete behavior equivalence.',
    '', 'Metric-profile correction: the first run compared all native references, including three IMG occurrences, to reference anchor outlinks. This run counts anchors for the comparable field and retains total references/kinds separately. Initial-response and final-document metadata differences remain visible.',
    '', '| Path | Field | Result | Reference | Independent |','| --- | --- | --- | --- | --- |',
    ...differences.map(value=>`| ${value.path} | ${value.field} | ${value.status} | ${JSON.stringify(value.reference).replaceAll('|','\\|')} | ${JSON.stringify(value.independent).replaceAll('|','\\|')} |`),
    '', 'Machine-readable explanations are in `reference-comparison.json`; exact independently authored cases are in `fixture-cases.json`.',
    '', 'Run `node scripts/reference-parity.mjs` to compare both engines on the same server. Set `SPIDER_BINARY` for a freshly built executable. `--reuse-reference` compares against retained reference CSVs without executing the original again.'];
  await writeFile(resolve(output,'reference-comparison.md'),lines.join('\n')+'\n');
  console.log(JSON.stringify({status:result.status,counts,reference_rows:referencePages.length,independent_rows:nativePages.length,full_parity:'NOT_PROVEN',outputs:['docs/parity/reference-comparison.json','docs/parity/reference-comparison.md','docs/parity/fixture-cases.json']}));
} finally {
  await stop(); await new Promise(resolveClose=>fixture.server.close(resolveClose));
  if (data) { const target=resolve(data); assert.equal(dirname(target),resolve(tmpdir())); assert(basename(target).startsWith('open-seo-spider-reference-')); await rm(target,{recursive:true,force:true}); }
}
