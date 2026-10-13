// Pure red controls: no native executable, child process, HTTP server or website is used.
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp, writeFile, readFile, mkdir, rm, realpath} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath} from 'node:url';
import {parseCsv, assertComparisonCsv, verifyBinarySha, assertFinalWitnessState} from './saved-comparison-mapping-oracles.mjs';

const passed = [], failed = [];
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const headers = ['page_kind', 'comparison_key', 'before_requested_url', 'after_requested_url', 'before_ordinal', 'after_ordinal', 'field', 'outcome', 'before_state', 'after_state', 'before_value_json', 'after_value_json', 'availability_changed', 'comparable_complete_values', 'reason', 'before_profile', 'after_profile', 'before_provenance_json', 'after_provenance_json', 'before_snapshot_sha256', 'after_snapshot_sha256'];
const beforeHash = 'a'.repeat(64), afterHash = 'b'.repeat(64);
const beforeValue = '=original "quoted", text\r\nsecond line';
const afterValue = {value: 'After, value', list: [2, 1]};
const api = {comparison: {
  before_snapshot: {typed_report_sha256: beforeHash}, after_snapshot: {typed_report_sha256: afterHash},
  pages: [{kind: 'changed', comparison_key: 'http://production.test/site/item?view=full',
    before: {ordinal: 7, original_requested_url: 'http://stage.test/stage/item?view=full'},
    after: {ordinal: 9, original_requested_url: 'http://production.test/site/item?view=full'},
    fields: [
      {field: 'titles', outcome: 'changed', availability_changed: false, comparable_complete_values: true,
        reason: '\t\uFEFF=SUM(1,2)\r\n+next',
        before: {state: 'observed', value: beforeValue, profile: '\u0085@SUM(1)', provenance: {generation: 7, label: 'before'}},
        after: {state: 'observed', value: afterValue, profile: 'profile-after', provenance: {generation: 9, label: 'after'}}},
      {field: 'canonical', outcome: 'unknown', availability_changed: true, comparable_complete_values: false, reason: null,
        before: {state: 'missing', value: null, profile: null, provenance: null},
        after: {state: 'observed', value: ['http://production.test/canonical'], profile: 'profile-after', provenance: {generation: 9}}}
    ]}]
}};
// Independent literal contract rows: no oracle projection builder creates this CSV fixture.
const baselineRows = [headers,
  ['changed', 'http://production.test/site/item?view=full', 'http://stage.test/stage/item?view=full', 'http://production.test/site/item?view=full', '7', '9', 'titles', 'changed', 'observed', 'observed', JSON.stringify(beforeValue), JSON.stringify(afterValue), 'false', 'true', "'\t\uFEFF=SUM(1,2)\r\n+next", "'\u0085@SUM(1)", 'profile-after', '{"generation":7,"label":"before"}', '{"generation":9,"label":"after"}', beforeHash, afterHash],
  ['changed', 'http://production.test/site/item?view=full', 'http://stage.test/stage/item?view=full', 'http://production.test/site/item?view=full', '7', '9', 'canonical', 'unknown', 'missing', 'observed', 'null', '["http://production.test/canonical"]', 'true', 'false', '', '', 'profile-after', 'null', '{"generation":9}', beforeHash, afterHash]
];
const csv = rows => rows.map(row => row.map(cell => '"' + cell.replaceAll('"', '""') + '"').join(',')).join('\r\n') + '\r\n';
const changed = mutate => { const rows = structuredClone(baselineRows); mutate(rows); return csv(rows); };
const rejectCsv = mutate => assert.throws(() => assertComparisonCsv(changed(mutate), api));
const cases = [
  ['RFC quoted parser retains commas, escaped quotes, embedded CR/LF, empty cells and UTF-8', () => {
    assert.deepEqual(parseCsv('"a,b","two ""quotes""","line\r\nnext",,"ä"\r\n"last",x,,,\r\n'), [['a,b', 'two "quotes"', 'line\r\nnext', '', 'ä'], ['last', 'x', '', '', '']]);
  }],
  ['Malformed quote and line boundaries reject instead of changing columns', () => {
    for (const malformed of ['"unterminated', 'un"quoted', '"closed"junk', 'a\rb']) assert.throws(() => parseCsv(malformed));
  }],
  ['Literal CSV checks every ordered row and all 42 data cells against API, including JSON/null/provenance/formula protection', () => {
    assert.deepEqual(assertComparisonCsv(csv(baselineRows), api), {data_rows: 2, columns: 21, checked_cells: 42});
  }],
  ['Red: swapped before/after concrete JSON values reject', () => rejectCsv(rows => { [rows[1][10], rows[1][11]] = [rows[1][11], rows[1][10]]; })],
  ['Red: swapped before/after requested URL cells reject', () => rejectCsv(rows => { [rows[1][2], rows[1][3]] = [rows[1][3], rows[1][2]]; })],
  ['Red: changed comparison key rejects', () => rejectCsv(rows => { rows[1][1] += '&wrong=1'; })],
  ['Red: changed page classification rejects', () => rejectCsv(rows => { rows[1][0] = 'removed'; })],
  ['Red: swapped before/after typed snapshot hash cells reject', () => rejectCsv(rows => { [rows[1][19], rows[1][20]] = [rows[1][20], rows[1][19]]; })],
  ['Red: changed field outcome rejects', () => rejectCsv(rows => { rows[1][7] = 'unchanged'; })],
  ['Red: changed fact availability state rejects', () => rejectCsv(rows => { rows[1][8] = 'partial'; })],
  ['Red: changed source ordinal rejects', () => rejectCsv(rows => { rows[1][4] = '9'; })],
  ['Red: swapped before/after concrete provenance rejects', () => rejectCsv(rows => { [rows[1][17], rows[1][18]] = [rows[1][18], rows[1][17]]; })],
  ['Red: unprotected whitespace/BOM-hidden reason formula rejects', () => rejectCsv(rows => { rows[1][14] = rows[1][14].slice(1); })],
  ['Red: unprotected Unicode whitespace-hidden profile formula rejects', () => rejectCsv(rows => { rows[1][15] = rows[1][15].slice(1); })],
  ['Red: wrong ordered field rows reject', () => rejectCsv(rows => { [rows[1], rows[2]] = [rows[2], rows[1]]; })],
  ['Red: missing/truncated CSV row rejects', () => rejectCsv(rows => { rows.pop(); })],
  ['Red: extra duplicated CSV row rejects', () => rejectCsv(rows => { rows.push(rows[1]); })],
  ['Red: changed ordered CSV header rejects', () => rejectCsv(rows => { [rows[0][10], rows[0][11]] = [rows[0][11], rows[0][10]]; })],
  ['Exact binary bytes verify before a hypothetical start without launching them', async () => {
    const dir = await mkdtemp(path.join(os.tmpdir(), 'spider-mapping-oracle-'));
    try {
      const filename = path.join(dir, 'not-an-executable.bin');
      const bytes = Buffer.from('pure fixture bytes; never executed'); await writeFile(filename, bytes);
      assert.equal(await verifyBinarySha(filename, sha(bytes)), sha(bytes));
    } finally { assert.ok(path.basename(await realpath(dir)).startsWith('spider-mapping-oracle-')); await rm(dir, {recursive: true, force: true}); }
  }],
  ['Red: changed binary bytes reject; restored bound bytes pass again', async () => {
    const dir = await mkdtemp(path.join(os.tmpdir(), 'spider-mapping-oracle-'));
    try {
      const filename = path.join(dir, 'not-an-executable.bin'), original = Buffer.from('original fixture bytes');
      await writeFile(filename, original); const expected = sha(original);
      await writeFile(filename, 'changed fixture bytes'); await assert.rejects(verifyBinarySha(filename, expected), /Native binary changed/);
      await writeFile(filename, original); assert.equal(await verifyBinarySha(filename, expected), expected);
    } finally { assert.ok(path.basename(await realpath(dir)).startsWith('spider-mapping-oracle-')); await rm(dir, {recursive: true, force: true}); }
  }],
  ['Zero witnesses after child close pass; a late request after an earlier zero sample rejects', () => {
    const state = {liveChildCount: 0, fixtureHitCount: 10, fixtureHitBaseline: 10, witnesses: [{origin: 'stage', attempts: []}, {origin: 'production', attempts: []}]};
    assert.deepEqual(assertFinalWitnessState(state), [{origin: 'stage', attempts: 0}, {origin: 'production', attempts: 0}]);
    state.witnesses[1].attempts.push({path: '/late'});
    assert.throws(() => assertFinalWitnessState(state), /Offline request observed after child shutdown/);
  }],
  ['Red: a still-owned live child rejects final witness acceptance', () => {
    assert.throws(() => assertFinalWitnessState({liveChildCount: 1, fixtureHitCount: 10, fixtureHitBaseline: 10, witnesses: [{origin: 'stage', attempts: []}, {origin: 'production', attempts: []}]}), /emitted close/);
  }],
  ['Red: a late live-fixture request rejects final acceptance', () => {
    assert.throws(() => assertFinalWitnessState({liveChildCount: 0, fixtureHitCount: 11, fixtureHitBaseline: 10, witnesses: [{origin: 'stage', attempts: []}, {origin: 'production', attempts: []}]}), /live website request/);
  }],
  ['Red: missing one offline origin witness rejects', () => {
    assert.throws(() => assertFinalWitnessState({liveChildCount: 0, fixtureHitCount: 10, fixtureHitBaseline: 10, witnesses: [{origin: 'stage', attempts: []}]}), /Both former origins/);
  }]
];
for (const [name, action] of cases) {
  await test(name, async () => { try { await action(); passed.push(name); } catch (error) { failed.push(name); throw error; } });
}
if (failed.length === 0 && passed.length === cases.length) {
  const sourcePath = fileURLToPath(import.meta.url), helperPath = fileURLToPath(new URL('./saved-comparison-mapping-oracles.mjs', import.meta.url));
  const root = path.resolve(process.env.SPIDER_ROOT || path.join(path.dirname(sourcePath), '..'));
  const proofPath = path.resolve(process.env.SPIDER_COMPARISON_MAPPING_CONTROL_PROOF || path.join(root, 'docs/verification/saved-comparison-mapping-oracle-controls.json'));
  const proof = {tested_at: new Date().toISOString(), result: 'PASS', pure_tests: passed.length, passed, test_sha256: sha(await readFile(sourcePath)), oracle_helper_sha256: sha(await readFile(helperPath)), native_executable_launched: false, app_launched: false, website_or_http_server_used: false, original_and_tampered_controls: ['every CSV column and row', 'swapped before/after values and requested URLs', 'comparison key', 'page class', 'swapped snapshot hashes', 'formula defence', 'mutated binary file', 'late request after initial zero sample', 'children still running']};
  await mkdir(path.dirname(proofPath), {recursive: true}); await writeFile(proofPath, JSON.stringify(proof, null, 2) + '\n');
  process.stdout.write(JSON.stringify({oracle_controls: 'PASS', tests: passed.length, proof: proofPath}) + '\n');
}
