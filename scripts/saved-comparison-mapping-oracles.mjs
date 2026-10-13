// Independent native harness oracles. Importing this module launches no process.
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';

export const COMPARISON_CSV_COLUMNS = [
  'page_kind', 'comparison_key', 'before_requested_url', 'after_requested_url',
  'before_ordinal', 'after_ordinal', 'field', 'outcome', 'before_state', 'after_state',
  'before_value_json', 'after_value_json', 'availability_changed', 'comparable_complete_values',
  'reason', 'before_profile', 'after_profile', 'before_provenance_json', 'after_provenance_json',
  'before_snapshot_sha256', 'after_snapshot_sha256'
];

export function parseCsv(text) {
  assert.equal(typeof text, 'string');
  const rows = [];
  let row = [], cell = '', state = 'start', rowOpen = false;
  function endCell() { row.push(cell); cell = ''; state = 'start'; }
  function endRow() { endCell(); rows.push(row); row = []; rowOpen = false; }
  for (let index = 0; index < text.length; index++) {
    const character = text[index];
    rowOpen = true;
    if (state === 'quoted') {
      if (character === '"') {
        if (text[index + 1] === '"') { cell += '"'; index++; }
        else state = 'closed';
      } else cell += character;
      continue;
    }
    if (character === ',') { endCell(); continue; }
    if (character === '\r' || character === '\n') {
      if (character === '\r') { assert.equal(text[index + 1], '\n', 'Bare CR outside a quoted CSV field'); index++; }
      endRow(); continue;
    }
    assert.notEqual(state, 'closed', 'Characters follow a closing CSV quote');
    if (character === '"') {
      assert.equal(state, 'start', 'A CSV quote starts inside an unquoted field'); state = 'quoted';
    } else { cell += character; state = 'unquoted'; }
  }
  assert.notEqual(state, 'quoted', 'Unterminated quoted CSV field');
  if (rowOpen) endRow();
  return rows;
}

function protectedText(value) {
  const significant = value.replace(/^[\p{White_Space}\uFEFF]*/u, '');
  return /^[=+@-]/.test(significant) ? "'" + value : value;
}
function textCell(actual, expected, label) {
  assert.equal(typeof expected, 'string', 'Expected text projection: ' + label);
  assert.equal(actual, protectedText(expected), 'CSV cell mismatch/formula defence: ' + label);
}
function jsonCell(actual, expected, label) {
  // JSON-string cells have their own quoting; leading formula text must not become a CSV formula.
  assert.ok(!/^[\p{White_Space}\uFEFF]*[=+@-]/u.test(actual), 'Unsafe unprotected CSV JSON cell: ' + label);
  let decoded;
  try { decoded = JSON.parse(actual); } catch { assert.fail('Invalid CSV JSON cell: ' + label); }
  assert.deepEqual(decoded, expected, 'CSV JSON projection mismatch: ' + label);
}
export function assertComparisonCsv(text, apiResponse) {
  const rows = parseCsv(text);
  assert.ok(rows.length > 0, 'CSV header is absent');
  assert.deepEqual(rows[0], COMPARISON_CSV_COLUMNS, 'CSV must retain the complete ordered public schema');
  const comparison = apiResponse.comparison;
  const expected = comparison.pages.flatMap(page => page.fields.map(field => ({page, field})));
  assert.equal(rows.length - 1, expected.length, 'CSV has missing, extra or truncated field rows');
  for (let index = 0; index < expected.length; index++) {
    const row = rows[index + 1], {page, field} = expected[index];
    assert.equal(row.length, COMPARISON_CSV_COLUMNS.length, 'CSV row column count: ' + index);
    const label = name => 'row ' + index + ' / ' + name;
    const expectedText = [
      page.kind, page.comparison_key, page.before?.original_requested_url || '', page.after?.original_requested_url || '',
      page.before ? String(page.before.ordinal) : '', page.after ? String(page.after.ordinal) : '', field.field,
      field.outcome, field.before.state, field.after.state
    ];
    for (let column = 0; column < expectedText.length; column++) textCell(row[column], expectedText[column], label(COMPARISON_CSV_COLUMNS[column]));
    jsonCell(row[10], field.before.value, label('before_value_json'));
    jsonCell(row[11], field.after.value, label('after_value_json'));
    textCell(row[12], String(field.availability_changed), label('availability_changed'));
    textCell(row[13], String(field.comparable_complete_values), label('comparable_complete_values'));
    textCell(row[14], field.reason || '', label('reason'));
    textCell(row[15], field.before.profile || '', label('before_profile'));
    textCell(row[16], field.after.profile || '', label('after_profile'));
    jsonCell(row[17], field.before.provenance, label('before_provenance_json'));
    jsonCell(row[18], field.after.provenance, label('after_provenance_json'));
    textCell(row[19], comparison.before_snapshot.typed_report_sha256, label('before_snapshot_sha256'));
    textCell(row[20], comparison.after_snapshot.typed_report_sha256, label('after_snapshot_sha256'));
  }
  return {data_rows: expected.length, columns: COMPARISON_CSV_COLUMNS.length, checked_cells: expected.length * COMPARISON_CSV_COLUMNS.length};
}

export async function verifyBinarySha(filename, expectedSha256) {
  assert.match(expectedSha256, /^[a-f0-9]{64}$/);
  const observedSha256 = createHash('sha256').update(await readFile(filename)).digest('hex');
  assert.equal(observedSha256, expectedSha256, 'Native binary changed before spawn or final verification');
  return observedSha256;
}

export function assertFinalWitnessState({liveChildCount, fixtureHitCount, fixtureHitBaseline, witnesses}) {
  assert.equal(liveChildCount, 0, 'All owned children must have emitted close before final witness acceptance');
  assert.equal(fixtureHitCount, fixtureHitBaseline, 'Read-only phase made a live website request');
  assert.equal(witnesses.length, 2, 'Both former origins must retain an owned request witness');
  for (const witness of witnesses) assert.equal(witness.attempts.length, 0, 'Offline request observed after child shutdown: ' + witness.origin);
  return witnesses.map(witness => ({origin: witness.origin, attempts: witness.attempts.length}));
}
