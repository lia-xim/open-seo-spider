// Raw report slices and complete JSON semantics; numeric tokens never pass through JS Number.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {parseCsv, COMPARISON_CSV_COLUMNS} from './saved-comparison-mapping-oracles.mjs';
export const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const WHITE = /[\u0020\t\r\n]/;
export function parseLossless(input, maximumBytes = 64 * 1024 * 1024) {
  const bytes = Buffer.isBuffer(input) ? input : Buffer.from(input, 'utf8');
  assert.ok(bytes.length <= maximumBytes, 'Lossless oracle input exceeds its explicit byte ceiling');
  const source = new TextDecoder('utf-8', {fatal: true}).decode(bytes);
  let index = 0, nodes = 0;
  function whitespace() { while (index < source.length && WHITE.test(source[index])) index++; }
  function string() {
    const start = index++; let closed = false;
    while (index < source.length) {
      const character = source[index++];
      if (character === '\\') { index++; continue; }
      if (character === '"') { closed = true; break; }
    }
    assert.ok(closed, 'Unterminated JSON string');
    const value = JSON.parse(source.slice(start, index));
    return {type: 'string', value, start, end: index};
  }
  function value(depth) {
    assert.ok(depth <= 256 && ++nodes <= 500_000, 'Lossless JSON structural budget');
    whitespace(); const start = index, character = source[index];
    if (character === '"') return string();
    if (character === '{') {
      index++; whitespace(); const properties = new Map();
      if (source[index] !== '}') while (true) {
        assert.equal(source[index], '"', 'JSON object key must be a string');
        const key = string().value; assert.ok(!properties.has(key), 'Duplicate JSON object key is ambiguous: ' + key);
        whitespace(); assert.equal(source[index++], ':'); properties.set(key, value(depth + 1)); whitespace();
        if (source[index] !== ',') break; index++; whitespace();
      }
      assert.equal(source[index++], '}'); return {type: 'object', properties, start, end: index};
    }
    if (character === '[') {
      index++; whitespace(); const elements = [];
      if (source[index] !== ']') while (true) {
        elements.push(value(depth + 1)); whitespace(); if (source[index] !== ',') break; index++;
      }
      assert.equal(source[index++], ']'); return {type: 'array', elements, start, end: index};
    }
    for (const [token, type, decoded] of [['true', 'boolean', true], ['false', 'boolean', false], ['null', 'null', null]]) {
      if (source.startsWith(token, index)) { index += token.length; return {type, value: decoded, start, end: index}; }
    }
    const number = /-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/y;
    number.lastIndex = index; const match = number.exec(source); assert.ok(match, 'Invalid JSON value at ' + index);
    index = number.lastIndex; return {type: 'number', lexeme: match[0], start, end: index};
  }
  const root = value(0); whitespace(); assert.equal(index, source.length, 'Trailing JSON material');
  return {source, bytes, root, node_count: nodes};
}
export function at(documentOrNode, segments) {
  let node = documentOrNode.root || documentOrNode;
  for (const segment of segments) {
    if (node.type === 'object') { assert.ok(node.properties.has(segment), 'Missing JSON path member: ' + segment); node = node.properties.get(segment); }
    else { assert.equal(node.type, 'array'); assert.ok(Number.isSafeInteger(segment) && segment >= 0 && segment < node.elements.length); node = node.elements[segment]; }
  }
  return node;
}
export function scalar(node) {
  assert.ok(['string', 'number', 'boolean', 'null'].includes(node.type), 'Expected scalar JSON node');
  return node.type === 'number' ? node.lexeme : node.value;
}
export function assertLosslessEqual(left, right, path = '$', ignored = new Set()) {
  if (ignored.has(path)) return;
  assert.equal(left.type, right.type, 'JSON kind differs: ' + path);
  if (left.type === 'object') {
    assert.deepEqual([...left.properties.keys()].sort(), [...right.properties.keys()].sort(), 'Complete JSON field set differs: ' + path);
    for (const [key, node] of left.properties) assertLosslessEqual(node, right.properties.get(key), path + '.' + key, ignored);
  } else if (left.type === 'array') {
    assert.equal(left.elements.length, right.elements.length, 'JSON array occurrence count differs: ' + path);
    left.elements.forEach((node, ordinal) => assertLosslessEqual(node, right.elements[ordinal], path + '[' + ordinal + ']', ignored));
  } else assert.equal(scalar(left), scalar(right), 'Exact JSON scalar/numeric lexeme differs: ' + path);
}
export function compactJson(input) {
  const text = Buffer.isBuffer(input) ? new TextDecoder('utf-8', {fatal: true}).decode(input) : input;
  const chunks = []; let segment = 0, quoted = false, escaped = false;
  for (let index = 0; index < text.length; index++) {
    const character = text[index];
    if (quoted) { if (escaped) escaped = false; else if (character === '\\') escaped = true; else if (character === '"') quoted = false; }
    else if (character === '"') quoted = true;
    else if (WHITE.test(character)) { if (segment < index) chunks.push(text.slice(segment, index)); segment = index + 1; }
  }
  assert.equal(quoted, false, 'Unterminated string in JSON compaction');
  chunks.push(text.slice(segment)); const bytes = Buffer.from(chunks.join(''), 'utf8'); parseLossless(bytes);
  return bytes;
}
export function replaceToken(document, segments, replacement) {
  parseLossless(replacement); const node = at(document, segments);
  return Buffer.from(document.source.slice(0, node.start) + replacement + document.source.slice(node.end), 'utf8');
}
export function unsafeIntegerLexemes(node, prefix = '$') {
  const result = [];
  function visit(value, path) {
    if (value.type === 'number' && /^-?\d+$/.test(value.lexeme) && (BigInt(value.lexeme) > BigInt(Number.MAX_SAFE_INTEGER) || BigInt(value.lexeme) < -BigInt(Number.MAX_SAFE_INTEGER))) result.push({path, lexeme: value.lexeme});
    else if (value.type === 'object') for (const [key, child] of value.properties) visit(child, path + '.' + key);
    else if (value.type === 'array') value.elements.forEach((child, ordinal) => visit(child, path + '[' + ordinal + ']'));
  }
  visit(node, prefix); return result;
}
export function assertOriginalBundle(document, before, after, {directTypedSlices = true} = {}) {
  assert.deepEqual([...document.root.properties.keys()].sort(), ['comparison', 'original_reports', 'website_requests']);
  assert.equal(scalar(at(document, ['website_requests'])), '0');
  const bundle = at(document, ['original_reports']);
  assert.deepEqual([...bundle.properties.keys()].sort(), ['after', 'before', 'binding', 'representation', 'schema_version']);
  assert.equal(scalar(at(bundle, ['schema_version'])), '1');
  assert.equal(scalar(at(bundle, ['representation'])), 'typed-crawl-report-serde-json-v1');
  assert.equal(scalar(at(bundle, ['binding'])), 'same-typed-reports-as-comparison');
  const bindings = [];
  for (const [side, original] of [['before', before], ['after', after]]) {
    const wrapped = at(bundle, [side]), report = at(wrapped, ['report']);
    assert.equal(report.type, 'object', 'Original report must be a full object');
    assert.deepEqual([...wrapped.properties.keys()].sort(), ['report', 'typed_report_json_bytes', 'typed_report_sha256']);
    assertLosslessEqual(report, original.document.root);
    const descriptor = at(document, ['comparison', side + '_snapshot']);
    const hash = scalar(at(wrapped, ['typed_report_sha256'])), count = scalar(at(wrapped, ['typed_report_json_bytes']));
    assert.match(hash, /^[a-f0-9]{64}$/); assert.match(count, /^\d+$/);
    assert.equal(hash, scalar(at(descriptor, ['typed_report_sha256'])));
    assert.equal(count, scalar(at(descriptor, ['typed_report_json_bytes'])));
    assert.equal(scalar(at(descriptor, ['typed_report_sha_profile'])), 'typed-crawl-report-serde-json-v1');
    assert.equal(scalar(at(descriptor, ['caller_binding_attested_by_core'])), false);
    assert.equal(hash, sha(original.compactBytes), 'Descriptor must also match independent compact Rust source export');
    assert.equal(BigInt(count), BigInt(original.compactBytes.length), 'Descriptor count differs from independent compact Rust source export');
    if (directTypedSlices) {
      const slice = Buffer.from(document.source.slice(report.start, report.end), 'utf8');
      assert.deepEqual(slice, compactJson(slice), 'Embedded API/CLI typed report slice must be compact');
      assert.equal(sha(slice), hash, 'Exact compact report slice hash differs from Core descriptor');
      assert.equal(BigInt(slice.length), BigInt(count), 'Exact compact report slice count differs from Core descriptor');
      assert.deepEqual(slice, original.compactBytes, 'All ordered typed report bytes must match exported source');
    }
    bindings.push({side, typed_report_sha256: hash, typed_report_json_bytes: Number(count), direct_embedded_slice_verified: directTypedSlices, unsafe_integer_lexemes: unsafeIntegerLexemes(report)});
  }
  return bindings;
}
export function assertDefaultEnvelope(document) {
  assert.equal(document.root.type, 'object'); assert.deepEqual([...document.root.properties.keys()].sort(), ['comparison', 'website_requests']);
  assert.equal(scalar(at(document, ['website_requests'])), '0');
}
export function assertComparableResponse(left, right, {ignoreCallerBinding = false} = {}) {
  const ignored = ignoreCallerBinding ? new Set(['$.comparison.before_snapshot.caller_binding', '$.comparison.after_snapshot.caller_binding']) : new Set();
  assertLosslessEqual(at(left, ['comparison']), at(right, ['comparison']), '$.comparison', ignored);
  assertLosslessEqual(at(left, ['website_requests']), at(right, ['website_requests']));
}
export function assertBundleCsv(text, document) {
  const rows = parseCsv(text); assert.deepEqual(rows[0], COMPARISON_CSV_COLUMNS);
  const comparison = at(document, ['comparison']), expected = [];
  for (const page of at(comparison, ['pages']).elements) for (const field of at(page, ['fields']).elements) expected.push({page, field});
  assert.equal(rows.length - 1, expected.length);
  const optional = (node, member) => { const value = at(node, [member]); return value.type === 'null' ? '' : String(scalar(value)); };
  const reference = (page, side, key) => { const value = at(page, [side]); return value.type === 'null' ? '' : String(scalar(at(value, [key]))); };
  const protect = text => /^[=+@-]/.test(text.replace(/^[\p{White_Space}\uFEFF]*/u, '')) ? "'" + text : text;
  expected.forEach(({page, field}, ordinal) => {
    const row = rows[ordinal + 1]; assert.equal(row.length, 21);
    const plain = [scalar(at(page, ['kind'])), scalar(at(page, ['comparison_key'])), reference(page, 'before', 'original_requested_url'), reference(page, 'after', 'original_requested_url'), reference(page, 'before', 'ordinal'), reference(page, 'after', 'ordinal'), scalar(at(field, ['field'])), scalar(at(field, ['outcome'])), scalar(at(field, ['before', 'state'])), scalar(at(field, ['after', 'state']))];
    plain.forEach((value, column) => assert.equal(row[column], protect(String(value)), 'CSV text ' + ordinal + '/' + column));
    for (const [column, side, name] of [[10, 'before', 'value'], [11, 'after', 'value'], [17, 'before', 'provenance'], [18, 'after', 'provenance']]) assertLosslessEqual(parseLossless(row[column]).root, at(field, [side, name]), 'CSV row ' + ordinal + '/' + COMPARISON_CSV_COLUMNS[column]);
    const tails = [[12, String(scalar(at(field, ['availability_changed'])))], [13, String(scalar(at(field, ['comparable_complete_values'])))], [14, optional(field, 'reason')], [15, optional(at(field, ['before']), 'profile')], [16, optional(at(field, ['after']), 'profile')], [19, scalar(at(comparison, ['before_snapshot', 'typed_report_sha256']))], [20, scalar(at(comparison, ['after_snapshot', 'typed_report_sha256']))]];
    for (const [column, value] of tails) assert.equal(row[column], protect(value), 'CSV text ' + ordinal + '/' + column);
  });
  return {data_rows: expected.length, columns: 21, checked_cells: expected.length * 21, numeric_json_cells_lossless: true};
}
