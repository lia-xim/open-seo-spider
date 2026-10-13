// Pure lossless/slice oracle controls. This file launches no native executable, child or HTTP server.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile, writeFile, mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {parseLossless, at, scalar, compactJson, replaceToken, assertLosslessEqual, unsafeIntegerLexemes, assertOriginalBundle, assertDefaultEnvelope, assertComparableResponse, assertBundleCsv} from './saved-comparison-bundle-oracles.mjs';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const beforeRaw = String.raw`{"config":{"source":"pure fixture"},"pages":[{"url":"http://stage.test/item","response_ms":9007199254740993,"links":[{"url":"http://stage.test/target"},{"url":"http://stage.test/target"}],"structured_data":[{"counter":9007199254740993}]}],"issues":[],"summary":{"pending":0},"warnings":["literal original_reports report marker; ä and \"quotes\""]}`;
const afterRaw = '{"config":{"source":"pure fixture"},"pages":[{"url":"http://production.test/item","response_ms":9007199254740995,"links":[{"url":"http://production.test/target"},{"url":"http://production.test/target"}],"structured_data":[{"counter":9007199254740995}]}],"issues":[],"summary":{"pending":0},"warnings":[]}';
const before = {document: parseLossless(beforeRaw), compactBytes: Buffer.from(beforeRaw)}, after = {document: parseLossless(afterRaw), compactBytes: Buffer.from(afterRaw)};
const hashes = {before: digest(before.compactBytes), after: digest(after.compactBytes)};
const count = {before: before.compactBytes.length, after: after.compactBytes.length};
const descriptor = side => `{"typed_report_sha256":"${hashes[side]}","typed_report_sha_profile":"typed-crawl-report-serde-json-v1","typed_report_json_bytes":${count[side]},"caller_binding_attested_by_core":false,"caller_binding":{"job_id":"${side}-owned-job","generation":1,"origin":"app_declared_native_persisted_job","basis":"pure-control"}}`;
const fieldRaw = '{"field":"structured_data","outcome":"changed","availability_changed":false,"comparable_complete_values":true,"reason":null,"before":{"state":"observed","value":[{"counter":9007199254740993}],"profile":null,"provenance":null},"after":{"state":"observed","value":[{"counter":9007199254740995}],"profile":null,"provenance":null}}';
const comparisonRaw = `{"before_snapshot":${descriptor('before')},"after_snapshot":${descriptor('after')},"pages":[{"kind":"changed","comparison_key":"http://production.test/item","before":{"original_requested_url":"http://stage.test/item","ordinal":0},"after":{"original_requested_url":"http://production.test/item","ordinal":1},"fields":[${fieldRaw}]}]}`;
function response(beforeSource = beforeRaw, afterSource = afterRaw) { return `{"comparison":${comparisonRaw},"website_requests":0,"original_reports":{"schema_version":1,"representation":"typed-crawl-report-serde-json-v1","binding":"same-typed-reports-as-comparison","before":{"report":${beforeSource},"typed_report_sha256":"${hashes.before}","typed_report_json_bytes":${count.before}},"after":{"report":${afterSource},"typed_report_sha256":"${hashes.after}","typed_report_json_bytes":${count.after}}}}`; }
const responseRaw = response(), responseDoc = parseLossless(responseRaw);
const csvHeaders = ['page_kind', 'comparison_key', 'before_requested_url', 'after_requested_url', 'before_ordinal', 'after_ordinal', 'field', 'outcome', 'before_state', 'after_state', 'before_value_json', 'after_value_json', 'availability_changed', 'comparable_complete_values', 'reason', 'before_profile', 'after_profile', 'before_provenance_json', 'after_provenance_json', 'before_snapshot_sha256', 'after_snapshot_sha256'];
const csvRow = ['changed', 'http://production.test/item', 'http://stage.test/item', 'http://production.test/item', '0', '1', 'structured_data', 'changed', 'observed', 'observed', '[{"counter":9007199254740993}]', '[{"counter":9007199254740995}]', 'false', 'true', '', '', '', 'null', 'null', hashes.before, hashes.after];
const csv = row => [csvHeaders, row].map(record => record.map(cell => '"' + cell.replaceAll('"', '""') + '"').join(',')).join('\r\n') + '\r\n';
function reorderedRaw(document) { return '{' + [...document.root.properties].sort(([a], [b]) => a.localeCompare(b)).map(([key, value]) => JSON.stringify(key) + ':' + document.source.slice(value.start, value.end)).join(',') + '}'; }
const cases = [
  ['Lossless scan retains u64/max-u64, negative zero, exponent and escaped strings', () => {
    const doc = parseLossless(String.raw`{"large":9007199254740993,"max":18446744073709551615,"negative":-0,"float":1.25e+3,"escaped":"comma, \"quote\" and ä"}`);
    assert.equal(scalar(at(doc, ['large'])), '9007199254740993'); assert.equal(scalar(at(doc, ['max'])), '18446744073709551615'); assert.equal(scalar(at(doc, ['negative'])), '-0'); assert.equal(scalar(at(doc, ['float'])), '1.25e+3');
  }],
  ['Duplicate JSON keys refuse ambiguous slice extraction', () => assert.throws(() => parseLossless('{"report":1,"report":2}'), /Duplicate/)],
  ['Malformed JSON number/value/string controls reject', () => { for (const text of ['01', '1.', '+1', 'NaN', 'truex', '[1,]', '{"a":}', '"unterminated']) assert.throws(() => parseLossless(text)); }],
  ['Invalid UTF-8 rejects instead of replacing original bytes', () => assert.throws(() => parseLossless(Buffer.from([0x22, 0xff, 0x22])))],
  ['Compaction removes only outside whitespace and retains numeric/string UTF-8 lexemes', () => {
    const input = String.raw` {
 "large":9007199254740993, "text":"ä \"q\" \n whitespace", "list": [ 1, 2 ]
 } `;
    assert.equal(compactJson(input).toString(), String.raw`{"large":9007199254740993,"text":"ä \"q\" \n whitespace","list":[1,2]}`);
  }],
  ['Red: values that JS Number rounds equal remain distinct numeric tokens', () => {
    const good = '{"u":9007199254740993}', rounded = '{"u":9007199254740992}';
    assert.deepEqual(JSON.parse(good), JSON.parse(rounded)); assert.throws(() => assertLosslessEqual(parseLossless(good).root, parseLossless(rounded).root), /numeric lexeme differs/);
  }],
  ['Object key ordering is semantic; array order and duplicate occurrence count are retained', () => {
    assertLosslessEqual(parseLossless('{"a":1,"b":2}').root, parseLossless('{"b":2,"a":1}').root);
    assert.throws(() => assertLosslessEqual(parseLossless('[1,2]').root, parseLossless('[2,1]').root)); assert.throws(() => assertLosslessEqual(parseLossless('[1,1]').root, parseLossless('[1]').root));
  }],
  ['Oracle byte ceiling exact fit succeeds and one byte over rejects', () => { parseLossless('{"x":1}', 7); assert.throws(() => parseLossless('{"x":1}', 6), /byte ceiling/); }],
  ['Oracle nesting ceiling exact256 succeeds and257 rejects', () => { parseLossless('['.repeat(256) + '0' + ']'.repeat(256)); assert.throws(() => parseLossless('['.repeat(257) + '0' + ']'.repeat(257)), /structural budget/); }],
  ['Depth123 viewport plus bundle envelope retains exact u64 and quote/backslash escaping', () => {
    const viewport = '['.repeat(123) + String.raw`{"counter":9007199254740993,"escaped":"quotes \" and backslash \\"}` + ']'.repeat(123);
    const standalone = parseLossless(viewport), wrapped = parseLossless(`{"original_reports":{"before":{"report":{"pages":[{"rendered_viewport":${viewport}}]}}}}`);
    assertLosslessEqual(at(wrapped, ['original_reports', 'before', 'report', 'pages', 0, 'rendered_viewport']), standalone.root);
    assert.equal(unsafeIntegerLexemes(wrapped.root)[0].lexeme, '9007199254740993');
  }],
  ['Exact f64 lexeme is retained without reparsing and near decimal differs', () => {
    const precise = parseLossless('{"width":51.248178375505404}'); assert.equal(scalar(at(precise, ['width'])), '51.248178375505404');
    assert.throws(() => assertLosslessEqual(precise.root, parseLossless('{"width":51.2481783755054}').root));
  }],
  ['Complete bundle oracle rejects changed f64 lexical value against independent typed source', () => {
    const widthBeforeRaw = beforeRaw.replace('"response_ms":9007199254740993', '"response_ms":9007199254740993,"width":51.248178375505404');
    const widthSource = {document: parseLossless(widthBeforeRaw), compactBytes: Buffer.from(widthBeforeRaw)};
    const raw = response(widthBeforeRaw).replaceAll(hashes.before, digest(widthSource.compactBytes)).replaceAll('"typed_report_json_bytes":' + count.before, '"typed_report_json_bytes":' + widthSource.compactBytes.length);
    assertOriginalBundle(parseLossless(raw), widthSource, after);
    assert.throws(() => assertOriginalBundle(parseLossless(raw.replace('"width":51.248178375505404', '"width":51.2481783755054')), widthSource, after));
  }],
  ['Raw token edit changes u64 without serializing other original numeric values', () => {
    const doc = parseLossless('{"one":1,"other":9007199254740995}'), edited = replaceToken(doc, ['one'], '9007199254740993');
    assert.equal(edited.toString(), '{"one":9007199254740993,"other":9007199254740995}');
  }],
  ['Complete compact typed source fixtures match both report slices/SHA/counts and all values', () => {
    const evidence = assertOriginalBundle(responseDoc, before, after); assert.equal(evidence.length, 2); assert.equal(evidence[0].typed_report_sha256, hashes.before); assert.ok(evidence[0].unsafe_integer_lexemes.some(value => value.lexeme === '9007199254740993'));
  }],
  ['Red: rounded embedded report numeric lexeme rejects despite JS parsed equality', () => assert.throws(() => assertOriginalBundle(parseLossless(responseRaw.replace('"response_ms":9007199254740993', '"response_ms":9007199254740992')), before, after))],
  ['Red: swapped before/after full original reports reject', () => assert.throws(() => assertOriginalBundle(parseLossless(response(afterRaw, beforeRaw)), before, after))],
  ['Red: removed original field or array occurrence rejects full report retention', () => {
    assert.throws(() => assertOriginalBundle(parseLossless(response(beforeRaw.replace(String.raw`,"warnings":["literal original_reports report marker; ä and \"quotes\""]`, ''))), before, after));
    const missingOccurrence = beforeRaw.replace(',{"url":"http://stage.test/target"}', ''); assert.throws(() => assertOriginalBundle(parseLossless(response(missingOccurrence)), before, after));
  }],
  ['Red: wrapper hash/count differs from independent source/Core descriptors', () => {
    const wrongHash = responseRaw.replace('"typed_report_sha256":"' + hashes.before + '"', '"typed_report_sha256":"' + 'c'.repeat(64) + '"'); assert.throws(() => assertOriginalBundle(parseLossless(wrongHash), before, after));
    assert.throws(() => assertOriginalBundle(parseLossless(responseRaw.replace('"typed_report_json_bytes":' + count.before, '"typed_report_json_bytes":' + (count.before + 1))), before, after));
  }],
  ['MCP sorted object keys retain full semantics; raw sorted slices are explicitly not typed hash profile', () => {
    const sorted = parseLossless(response(reorderedRaw(before.document), reorderedRaw(after.document)));
    assertOriginalBundle(sorted, before, after, {directTypedSlices: false}); assert.throws(() => assertOriginalBundle(sorted, before, after, {directTypedSlices: true}));
  }],
  ['Red: pretty embedded report cannot claim direct compact typed slice profile', () => assert.throws(() => assertOriginalBundle(parseLossless(response(beforeRaw.replace('{"config"', '{\n"config"'))), before, after))],
  ['Default envelope requires field omission; null originals and unrecognized fields reject', () => {
    assertDefaultEnvelope(parseLossless(`{"comparison":${comparisonRaw},"website_requests":0}`));
    assert.throws(() => assertDefaultEnvelope(parseLossless(`{"comparison":${comparisonRaw},"website_requests":0,"original_reports":null}`)));
    assert.throws(() => assertOriginalBundle(parseLossless(responseRaw.replace('"schema_version":1', '"extra":true,"schema_version":1')), before, after));
  }],
  ['Red: API/MCP caller generation change rejects with exact default binding oracle', () => {
    const changed = parseLossless(responseRaw.replace('"generation":1', '"generation":2'));
    assert.throws(() => assertComparableResponse(changed, responseDoc), /generation/);
  }],
  ['Red: API/MCP job ID swap rejects with exact default binding oracle', () => {
    const changed = parseLossless(responseRaw.replace('"job_id":"before-owned-job"', '"job_id":"after-owned-job"'));
    assert.throws(() => assertComparableResponse(changed, responseDoc), /job_id/);
  }],
  ['CLI caller binding exception requires explicit option; all Core values still bind', () => {
    const changed = parseLossless(responseRaw.replace('"generation":1', '"generation":2'));
    assertComparableResponse(changed, responseDoc, {ignoreCallerBinding: true});
    const corrupted = parseLossless(changed.source.replace(hashes.before, 'e'.repeat(64)));
    assert.throws(() => assertComparableResponse(corrupted, responseDoc, {ignoreCallerBinding: true}));
  }],
  ['Every CSV cell matches lossless ordered comparison projection', () => assert.deepEqual(assertBundleCsv(csv(csvRow), responseDoc), {data_rows: 1, columns: 21, checked_cells: 21, numeric_json_cells_lossless: true})],
  ['Red: CSV numeric token rounded to same JS Number still rejects', () => {
    const row = [...csvRow]; row[10] = '[{"counter":9007199254740992}]'; assert.deepEqual(JSON.parse(row[10]), JSON.parse(csvRow[10])); assert.throws(() => assertBundleCsv(csv(row), responseDoc));
  }],
  ['Red: swapped CSV originals and hashes reject', () => {
    const values = [...csvRow]; [values[10], values[11]] = [values[11], values[10]]; assert.throws(() => assertBundleCsv(csv(values), responseDoc));
    const hashesSwapped = [...csvRow]; [hashesSwapped[19], hashesSwapped[20]] = [hashesSwapped[20], hashesSwapped[19]]; assert.throws(() => assertBundleCsv(csv(hashesSwapped), responseDoc));
  }],
  ['Unsafe integer evidence labels preserve exact lexeme strings', () => {
    const rows = unsafeIntegerLexemes(before.document.root); assert.deepEqual(rows.map(value => value.lexeme), ['9007199254740993', '9007199254740993']); assert.ok(rows.every(value => typeof value.lexeme === 'string'));
  }]
];
const passed = [], failed = [];
for (const [name, run] of cases) await test(name, async () => { try { await run(); passed.push(name); } catch (error) { failed.push(name); throw error; } });
if (!failed.length && passed.length === cases.length) {
  const sourcePath = fileURLToPath(import.meta.url), helperPath = fileURLToPath(new URL('./saved-comparison-bundle-oracles.mjs', import.meta.url));
  const root = path.resolve(process.env.SPIDER_ROOT || path.join(path.dirname(sourcePath), '..'));
  const proofPath = path.resolve(process.env.SPIDER_BUNDLE_CONTROL_PROOF || path.join(root, 'docs/verification/saved-comparison-original-bundle-oracle-controls.json'));
  const proof = {result: 'PASS', tested_at: new Date().toISOString(), pure_tests: passed.length, passed, helper_sha256: digest(await readFile(helperPath)), tests_sha256: digest(await readFile(sourcePath)), numeric_hashing_strategy: 'Exact raw compact UTF-8 report slices; numeric tokens retained as strings, never JSON.stringify of parsed reports', native_executable_launched: false, app_or_child_or_http_server_launched: false, original_fixture_kind: 'Synthetic pure oracle controls; not native report evidence'};
  await mkdir(path.dirname(proofPath), {recursive: true}); await writeFile(proofPath, JSON.stringify(proof, null, 2) + '\n'); process.stdout.write(JSON.stringify({pure_oracle_controls: 'PASS', tests: passed.length, proof: proofPath}) + '\n');
}
