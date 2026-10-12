// Native metric comparison using already retained ordinary reference CSVs.
// This does not run the original application or request any listed website.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { csv } from './reference-normalize.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const app = process.env.SPIDER_APP_URL || 'http://127.0.0.1:8787';
const binary = resolve(process.env.SPIDER_BINARY || resolve(root, 'target/debug', process.platform === 'win32' ? 'open-seo-spider.exe' : 'open-seo-spider'));
const sha = value => createHash('sha256').update(value).digest('hex');
const reference = JSON.parse(await readFile(resolve(root, 'docs/parity/reference-fixture/latest-run.json'), 'utf8'));
assert.equal(reference.status, 'REFERENCE_EXPORT_OBSERVED');
const cases = [], files = [], placeholders = [];
for (const [name, field, prefix] of [['seitentitel_alle.csv', 'title', 'Titel'], ['meta_description_alle.csv', 'description', 'Meta Description']]) {
  const artifact = reference.exports.find(value => value.name === name);
  assert.ok(artifact);
  const bytes = await readFile(resolve(root, reference.output_directory, name));
  assert.equal(sha(bytes), artifact.sha256, `Retained ${name} must retain original bytes`);
  files.push(artifact);
  for (const row of csv(bytes.toString('utf8'))) {
    const occurrences = Number(row['Häufigkeit']);
    assert.ok(Number.isInteger(occurrences) && occurrences >= 0 && occurrences <= 2, 'Reference scope must explicitly support each available slot');
    if (!occurrences) placeholders.push({ url: row.Adresse, field, reference_display: row[`Pixelbreite von ${prefix} 1`], eligibility: 'absent metadata is not a present measured element' });
    for (let slot = 1; slot <= occurrences; slot++) {
      const expected = Number(row[`Pixelbreite von ${prefix} ${slot}`]);
      assert.ok(Number.isInteger(expected) && expected >= 0);
      cases.push({ url: row.Adresse, field, slot, text: row[`${prefix} ${slot}`], expected });
    }
  }
}
const response = await fetch(app + '/api/serp/preview', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ rows: cases.map(value => ({ url: value.url, title: value.field === 'title' ? value.text : '', description: value.field === 'description' ? value.text : '' })) }) });
assert.equal(response.status, 200);
const measured = await response.json();
assert.equal(measured.website_requests, 0);
assert.equal(measured.rows.length, cases.length);
const observations = cases.map((value, index) => ({ ...value, native: measured.rows[index][value.field + '_pixels'], status: measured.rows[index][value.field + '_pixels'] === value.expected ? 'MATCH' : measured.rows[index][value.field + '_pixels'] === null ? 'UNAVAILABLE' : 'DIFFERENT' }));
const counts = observations.reduce((result, value) => (result[value.status]++, result), { MATCH: 0, DIFFERENT: 0, UNAVAILABLE: 0 });
const proof = { schema_version: 1, tested_at: new Date().toISOString(), result: counts.DIFFERENT || counts.UNAVAILABLE ? 'DIFFERENCES_RETAINED' : 'PASS_BOUNDED_NATIVE_PROFILE', full_vendor_parity: 'NOT_PROVEN', app_url: app, binary_sha256: sha(await readFile(binary)), measurement_profile: measured.measurement_profile, reference_fixture_sha256: reference.fixture_sha256, reference_run_at: reference.observed_at, files, website_requests: measured.website_requests, counts, absent_placeholders_excluded: placeholders, observations, limitations: ['Only the retained ordinary Windows reference fixture and fixed 150-percent metric profile are compared.', 'Other DPI/font profiles, complex shaping and unavailable glyphs remain outside this proof.', 'This compares text measurements, not issue-filter membership or whole features.'] };
await writeFile(resolve(root, 'docs/parity/native-text-width-comparison.json'), JSON.stringify(proof, null, 2) + '\n');
console.log(JSON.stringify({ result: proof.result, counts, absent: placeholders.length, website_requests: measured.website_requests }));
assert.equal(counts.DIFFERENT, 0);
assert.equal(counts.UNAVAILABLE, 0);
