import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export function csv(text) {
  const rows = []; let row = [], value = '', quoted = false;
  text = text.replace(/^\uFEFF/, '');
  for (let i = 0; i < text.length; i++) {
    const char = text[i];
    if (char === '"') { if (quoted && text[i + 1] === '"') { value += '"'; i++; } else quoted = !quoted; }
    else if (char === ',' && !quoted) { row.push(value); value = ''; }
    else if (char === '\n' && !quoted) { row.push(value.replace(/\r$/, '')); rows.push(row); row = []; value = ''; }
    else value += char;
  }
  if (value || row.length) { row.push(value); rows.push(row); }
  const headers = rows.shift();
  return rows.filter(row => row.some(Boolean)).map(row => Object.fromEntries(headers.map((header, index) => [header, row[index] ?? ''])));
}
export async function normalizeReference(run) {
const tables = {};
for (const artifact of run.exports) tables[artifact.name] = csv(await readFile(resolve(root, run.output_directory, artifact.name), 'utf8'));
const path = url => { try { const parsed = new URL(url); return parsed.origin === run.fixture_origin ? parsed.pathname + parsed.search + parsed.hash : url; } catch { return url; } };
const table = name => new Map(tables[name].map(row => [path(row.Adresse), row]));
const pages = table('intern_alle.csv'), titles = table('seitentitel_alle.csv'), canonicals = table('canonicals_alle.csv'), descriptions = table('meta_description_alle.csv');
const observed = [...pages].map(([address, row]) => ({ path: address, http_status: Number(row['Status-Code']) || null,
  content_type: row.Inhaltstyp, indexability: row.Indexierbarkeit, indexability_reason: row.Indexierbarkeitsstatus,
  title: row['Titel 1'], title_length: Number(row['Länge von Titel 1']) || 0, title_pixels: Number(row['Pixelbreite von Titel 1']) || 0,
  title_occurrences: Number(titles.get(address)?.Häufigkeit) || 0, second_title: row['Titel 2'],
  description: row['Meta Description 1'], description_length: Number(row['Länge von Meta Description 1']) || 0,
  description_occurrences: Number(descriptions.get(address)?.Häufigkeit) || 0, second_description: row['Meta Description 2'],
  h1: row['H1-1'], h2: row['H2-1'], robots_meta: row['Meta Robots 1'], robots_header: row['X-Robots-Tag 1'],
  canonical: path(row['Canonical-Linkelement 1']), second_canonical: path(row['Canonical-Linkelement 2']),
  canonical_occurrences: Number(canonicals.get(address)?.Häufigkeit) || 0, depth: Number(row.Crawltiefe) || 0,
  inlinks: Number(row.Inlinks) || 0, unique_inlinks: Number(row['Einzigartige Inlinks']) || 0,
  outlinks: Number(row.Outlinks) || 0, unique_outlinks: Number(row['Einzigartige Outlinks']) || 0,
  redirect_to: path(row['Umleitungs-URL']), word_count: Number(row.Wortanzahl) || 0, hash: row.Hash || null })).sort((a,b) => a.path.localeCompare(b.path));
const byPath = new Map(observed.map(row => [row.path, row]));
const checks = [];
function check(id, paths, test) { test(); checks.push({ id, paths, status: 'PASS', scope: 'Reference export observation only; no Open SEO Spider comparison yet.' }); }
check('reference.missing-empty-blank-title', ['/missing-title','/empty-title','/blank-title'], () => {
  assert.equal(byPath.get('/missing-title').title_occurrences, 0);
  assert.equal(byPath.get('/empty-title').title, ''); assert.equal(byPath.get('/blank-title').title, '');
});
check('reference.two-title-elements', ['/multiple-title'], () => { assert.equal(byPath.get('/multiple-title').title_occurrences, 2); assert.equal(byPath.get('/multiple-title').second_title, 'Second controlled title'); });
check('reference.same-title-description-values', ['/duplicate-a','/duplicate-b'], () => { assert.equal(byPath.get('/duplicate-a').title, byPath.get('/duplicate-b').title); assert.equal(byPath.get('/duplicate-a').description, byPath.get('/duplicate-b').description); });
check('reference.title-width-vs-length', ['/long-title'], () => { assert.equal(byPath.get('/long-title').title_length, 75); assert.equal(byPath.get('/long-title').title_pixels, 1350); });
check('reference.body-metadata-extracted', ['/body-metadata'], () => { assert.equal(byPath.get('/body-metadata').title, 'Title in the body'); assert.equal(byPath.get('/body-metadata').description, 'Description in the body'); });
check('reference.multiple-description-elements', ['/multiple-description'], () => { assert.equal(byPath.get('/multiple-description').description_occurrences, 2); assert.equal(byPath.get('/multiple-description').second_description, 'Second controlled description'); });
check('reference.meta-header-noindex', ['/noindex','/header-noindex'], () => { assert.equal(byPath.get('/noindex').indexability_reason, 'noindex'); assert.equal(byPath.get('/header-noindex').indexability_reason, 'noindex'); });
check('reference.relative-canonical-resolved', ['/canonical-self','/canonical-other'], () => { assert.equal(byPath.get('/canonical-self').canonical, '/canonical-self'); assert.equal(byPath.get('/canonical-other').canonical, '/baseline'); assert.equal(byPath.get('/canonical-other').indexability_reason, 'Canonicalised'); });
check('reference.two-conflicting-canonicals-retained', ['/canonical-conflict'], () => { assert.equal(byPath.get('/canonical-conflict').canonical_occurrences, 2); assert.equal(byPath.get('/canonical-conflict').canonical, '/baseline'); assert.equal(byPath.get('/canonical-conflict').second_canonical, '/noindex'); });
check('reference.initial-redirect-status', ['/redirect-one','/redirect-two'], () => { assert.equal(byPath.get('/redirect-one').http_status, 301); assert.equal(byPath.get('/redirect-one').redirect_to, '/redirect-two'); assert.equal(byPath.get('/redirect-two').redirect_to, '/baseline'); });
check('reference.real-404-503', ['/missing','/server-error'], () => { assert.equal(byPath.get('/missing').http_status, 404); assert.equal(byPath.get('/server-error').http_status, 503); });
check('reference.robots-no-fetch', ['/blocked'], () => { assert.equal(byPath.get('/blocked').http_status, null); assert(!run.requests.some(request => request.path === '/blocked')); });
check('reference.sitemap-orphan-not-auto-fetched-default', ['/orphan-only'], () => { assert(!byPath.has('/orphan-only')); assert(!run.requests.some(request => request.path === '/orphan-only')); });
const result = { schema_version: 1, generated_at: new Date().toISOString(), reference_run_at: run.observed_at,
  reference_version: '24.3', reference_licence_state: 'Missing; normal CLI export succeeded for this small fixture.',
  source: 'docs/parity/reference-fixture/latest-run.json', normalization: 'Fixture-origin URLs become paths; exported localized values remain observations.',
  status: 'PASS', checks, observations: observed, export_row_counts: Object.fromEntries(Object.entries(tables).map(([name, rows]) => [name, rows.length])),
  comparison_status: 'NOT_PROVEN', limitations: ['All-tab columns were observed; issue-filter membership was not exported in this run.', 'Pixel values depend on reference version/font measurement; no independent metric equivalence is claimed.', 'No browser rendering, paid configuration, API provider or >500 URL vendor behavior was exercised.'] };
await writeFile(resolve(root, 'docs/parity/reference-fixture/normalized-observations.json'), JSON.stringify(result, null, 2) + '\n');
return result;
}

if (resolve(process.argv[1] ?? '') === fileURLToPath(import.meta.url)) {
  const run = JSON.parse(await readFile(resolve(root, 'docs/parity/reference-fixture/latest-run.json'), 'utf8'));
  const result = await normalizeReference(run);
  console.log(JSON.stringify({ status: result.status, reference_checks: result.checks.length, observed_rows: result.observations.length, export_row_counts: result.export_row_counts }));
}
