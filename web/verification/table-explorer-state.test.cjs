'use strict';
// Pure production helper controls. These do not claim browser/native acceptance.
const test = require('node:test'), assert = require('node:assert/strict'), fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
const explorer = require('../table-explorer.js');
const columns = [
  { key: 'url', type: 'text', value: row => row.url, mandatory: true },
  { key: 'title', type: 'text', value: row => row.title },
  { key: 'count', type: 'number', value: () => 'Nicht erfasst', rawValue: row => row.count },
  { key: 'status', type: 'number', value: row => row.status },
];
const view = changes => ({ search: '', rules: [], sort: { key: 'url', direction: 1 }, visibleKeys: columns.map(column => column.key), ...changes });
function plan(changes = {}, schema = columns, identity = {}) { const result = explorer.compile(schema, view(changes), identity); assert.equal(result.ok, true, JSON.stringify(result)); return result.value; }
function apply(rows, changes = {}, identity = {}) { const result = plan(changes, columns, identity).rows(rows); assert.equal(result.ok, true, JSON.stringify(result)); return result.value; }
function storage(initial = []) { const data = new Map(initial), calls = []; return { data, calls, getItem(key) { calls.push(['get', key]); return data.has(key) ? data.get(key) : null; }, setItem(key, value) { calls.push(['set', key]); data.set(key, value); }, removeItem(key) { calls.push(['remove', key]); data.delete(key); } }; }
const context = { projectId: 'owned-project', section: 'titles' };

test('raw classification separates unknown, observed empty, zero, finite positive and wrong numeric types', () => {
  for (const raw of [null, undefined, NaN, Infinity, -Infinity, {}, [], Symbol('owned')]) assert.equal(explorer.classify(raw).kind, 'unknown');
  for (const raw of ['', ' \t\r\n', '\u00a0\u2003']) assert.deepEqual(explorer.classify(raw, 'number'), { kind: 'empty', value: '', search: '' });
  assert.deepEqual(explorer.classify(0, 'number'), { kind: 'number', value: 0, search: '0' });
  assert.equal(explorer.classify(42, 'number').value, 42);
  for (const raw of ['0', '42', false, true]) assert.equal(explorer.classify(raw, 'number').kind, 'unknown');
  assert.equal(explorer.classify(false).kind, 'boolean');
  assert.equal(explorer.classify(0, 'text').kind, 'text');
  assert.throws(() => explorer.classify(0, 'invented'), TypeError);
});
test('text normalization uses NFC and German lowercase without losing accents or merging internal whitespace', () => {
  assert.equal(explorer.normalizeText('  CAFE\u0301 Ä  '), 'café ä');
  assert.equal(explorer.normalizeText('A  B'), 'a  b');
  assert.notEqual(explorer.normalizeText('Cafe'), explorer.normalizeText('Café'));
  assert.throws(() => explorer.normalizeText(null), TypeError);
  const rows = [{ url: '1', title: 'Cafe\u0301 Ä' }, { url: '2', title: 'CAFÉ ä' }, { url: '3', title: 'Cafe Ä' }];
  assert.deepEqual(apply(rows, { rules: [{ key: 'title', op: 'equals', text: ' café Ä ' }] }), [rows[0], rows[1]]);
  assert.deepEqual(apply(rows, { search: 'Fe\u0301 ä' }), [rows[0], rows[1]]);
});
test('AND numeric bounds are inclusive and text rules retain every matching row beyond the first hundred', () => {
  const rows = Array.from({ length: 107 }, (_, index) => ({ url: `owned/${String(index).padStart(3, '0')}`, count: index, title: index >= 100 ? 'CAFÉ tail' : 'body', status: 200 }));
  const before = rows.slice();
  const selected = apply(rows, { visibleKeys: ['url'], rules: [{ key: 'count', op: 'min', min: 100 }, { key: 'count', op: 'max', max: 105 }, { key: 'title', op: 'contains', text: 'cafe\u0301' }], sort: { key: 'count', direction: -1 } });
  assert.deepEqual(selected, [rows[105], rows[104], rows[103], rows[102], rows[101], rows[100]]);
  assert.deepEqual(rows, before); assert.equal(selected[0], rows[105]);
  assert.deepEqual(apply(rows, { rules: [{ key: 'count', op: 'range', min: 104, max: 106 }] }), [rows[104], rows[105], rows[106]]);
  assert.equal(apply(rows).length, 107);
});
test('known, empty and unknown rules are disjoint and numeric zero is never fabricated from missing data', () => {
  const rows = [{ url: 'zero', count: 0 }, { url: 'positive', count: 9 }, { url: 'empty', count: '' }, { url: 'spaces', count: '\t ' }, { url: 'null', count: null }, { url: 'absent' }, { url: 'nonfinite', count: NaN }, { url: 'wrongtype', count: '0' }];
  assert.deepEqual(apply(rows, { rules: [{ key: 'count', op: 'range', min: 0, max: 0 }] }), [rows[0]]);
  assert.deepEqual(apply(rows, { rules: [{ key: 'count', op: 'known' }] }), [rows[1], rows[0]]);
  assert.deepEqual(apply(rows, { rules: [{ key: 'count', op: 'empty' }] }), [rows[2], rows[3]]);
  assert.deepEqual(apply(rows, { rules: [{ key: 'count', op: 'unknown' }] }), [rows[5], rows[6], rows[4], rows[7]]);
  assert.deepEqual(apply(rows, { rules: [{ key: 'count', op: 'known' }, { key: 'count', op: 'unknown' }] }), []);
});
test('hidden columns supply raw filter and sort values while global search also retains scalar display labels', () => {
  const rows = [{ url: 'b', title: 'hidden needle', count: 20 }, { url: 'a', title: 'hidden needle', count: 2 }, { url: 'c', title: 'other', count: 3 }];
  const compiled = plan({ visibleKeys: ['url'], search: 'NEEDLE', rules: [{ key: 'count', op: 'min', min: 2 }], sort: { key: 'count', direction: 1 } });
  assert.deepEqual(compiled.columns.map(column => column.key), ['url']);
  assert.deepEqual(compiled.rows(rows).value, [rows[1], rows[0]]);
  assert.deepEqual(apply(rows, { search: 'Nicht erfasst' }), [rows[1], rows[0], rows[2]]);
  const rawUnknown = [{ url: 'unknown', count: null }, { url: 'zero', count: 0 }];
  assert.deepEqual(apply(rawUnknown, { search: 'NICHT ERFASST', rules: [{ key: 'count', op: 'unknown' }], visibleKeys: ['url'] }), [rawUnknown[0]]);
  assert.deepEqual(apply(rawUnknown, { rules: [{ key: 'count', op: 'equals', text: 'Nicht erfasst' }] }), []);
  const labels = [{ key: 'url', value: row => row.url, mandatory: true }, { key: 'indexable', type: 'text', rawValue: row => row.indexable, value: row => row.indexable == null ? 'Unbekannt · Cafe\u0301' : 'Indexierbar' }];
  const compiledLabels = plan({ search: 'UNBEKANNT · CAFÉ', visibleKeys: ['url'], rules: [{ key: 'indexable', op: 'unknown' }] }, labels);
  assert.deepEqual(compiledLabels.rows([{ url: 'missing', indexable: null }, { url: 'known', indexable: true }]).value.map(row => row.url), ['missing']);
});
test('numeric ordering keeps empty before unknown at the end in both directions and distinguishes 2 from 10', () => {
  const rows = [{ url: 'unknown', count: null }, { url: 'ten', count: 10 }, { url: 'empty', count: '' }, { url: 'zero', count: 0 }, { url: 'two', count: 2 }, { url: 'negative', count: -1 }];
  assert.deepEqual(apply(rows, { sort: { key: 'count', direction: 1 } }), [rows[5], rows[3], rows[4], rows[1], rows[2], rows[0]]);
  assert.deepEqual(apply(rows, { sort: { key: 'count', direction: -1 } }), [rows[1], rows[4], rows[3], rows[5], rows[2], rows[0]]);
});
test('natural text sort has ascending identity and ordinal ties independent of primary direction, preserving duplicate observations', () => {
  const rows = [{ url: 'owned/10', title: 'Same', ordinal: 9 }, { url: 'owned/2', title: 'same', ordinal: 8 }, { url: 'owned/2', title: 'same', ordinal: 3 }, { url: 'owned/2', title: 'same', ordinal: 3 }, { url: 'owned/1', title: 'Zebra', ordinal: 0 }];
  const identity = { identity: row => row.url, ordinal: row => row.ordinal };
  assert.deepEqual(apply(rows, { sort: { key: 'title', direction: 1 } }, identity), [rows[2], rows[3], rows[1], rows[0], rows[4]]);
  assert.deepEqual(apply(rows, { sort: { key: 'title', direction: -1 } }, identity), [rows[4], rows[2], rows[3], rows[1], rows[0]]);
  assert.equal(apply(rows, { sort: { key: 'title', direction: -1 } }, identity).length, 5);
  const natural = [{ url: 'x', title: 'Seite 10' }, { url: 'y', title: 'Seite 2' }]; assert.deepEqual(apply(natural, { sort: { key: 'title', direction: 1 } }), [natural[1], natural[0]]);
});
test('projection preserves requested order, appends mandatory identity and rejects duplicate or unknown keys', () => {
  assert.deepEqual(explorer.projectColumns(columns, ['status', 'title']).value, [columns[3], columns[1], columns[0]]);
  assert.deepEqual(explorer.projectColumns(columns, []).value, [columns[0]]);
  for (const keys of [['url', 'url'], ['invented'], 'url']) assert.equal(explorer.projectColumns(columns, keys).ok, false);
  assert.equal(explorer.projectColumns(columns.map(column => ({ ...column, mandatory: false })), []).ok, false);
});
test('invalid rule/schema/sort limits refuse before evaluating rows or causing a partial result', () => {
  const invalidViews = [view({ rules: Array(17).fill({ key: 'count', op: 'known' }) }), view({ search: '雪'.repeat(1366) }), view({ sort: { key: 'count', direction: 0 } }), view({ sort: { key: 'invented', direction: 1 } }), view({ rules: [{ key: 'count', op: 'range', min: 2, max: 1 }] }), view({ rules: [{ key: 'count', op: 'min', min: '0' }] }), view({ rules: [{ key: 'count', op: 'max', max: Infinity }] }), view({ rules: [{ key: 'title', op: 'min', min: 1 }] }), view({ rules: [{ key: 'title', op: 'equals', text: 'x', extra: true }] }), view({ rules: [{ key: 'missing', op: 'known' }] }), view({ rules: [{ key: 'count', op: 'invented' }] }), { ...view(), invented: true }];
  for (const value of invalidViews) assert.equal(explorer.compile(columns, value).ok, false, JSON.stringify(value));
  for (const schema of [[], Array(129).fill(columns[0]), [...columns, columns[0]], [{ key: 'a', type: 'invented', value: row => row }], [{ key: 'a', value: true }]]) assert.equal(explorer.compile(schema, view()).ok, false);
  const throwing = plan({ visibleKeys: ['url'] }, [{ key: 'url', value: row => { if (row.bad) throw Error('Owned raw getter failure'); return row.url; } }]);
  const failed = throwing.rows([{ url: 'good' }, { bad: true }]); assert.equal(failed.ok, false); assert.match(failed.error.message, /Owned raw getter/); assert.equal(Object.hasOwn(failed, 'value'), false);
  for (const reason of [null, 'owned non-Error refusal', Object.create(null)]) { const rejected = plan({ visibleKeys: ['url'] }, [{ key: 'url', value() { throw reason; } }]).rows([{}]); assert.equal(rejected.ok, false); assert.equal(rejected.error.code, 'evaluation_failed'); assert.equal(typeof rejected.error.message, 'string'); }
  assert.equal(plan().rows(null).ok, false);
});
test('preferences save only visible columns with exact project/section identity, retaining other scopes and no view state', () => {
  const db = storage(), alternate = { projectId: 'owned-project', section: 'meta' }, otherProject = { projectId: 'second', section: 'titles' };
  assert.deepEqual(explorer.load(db, context, columns), { ok: true, value: null, missing: true });
  assert.deepEqual(explorer.save(db, context, columns, { visibleKeys: ['title', 'url'] }), { ok: true, value: { visibleKeys: ['title', 'url'] } });
  const document = JSON.parse(db.data.get(explorer.keyFor(context))); assert.deepEqual(document, { version: 1, projectId: 'owned-project', section: 'titles', visibleKeys: ['title', 'url'] });
  assert.deepEqual(explorer.load(db, context, columns), { ok: true, value: { visibleKeys: ['title', 'url'] }, missing: false });
  assert.equal(explorer.load(db, alternate, columns).missing, true); assert.equal(explorer.load(db, otherProject, columns).missing, true);
  const source = db.data.get(explorer.keyFor(context)); assert.equal(explorer.save(db, context, columns, { visibleKeys: ['url'], search: 'not persisted' }).ok, false); assert.equal(db.data.get(explorer.keyFor(context)), source);
  assert.notEqual(explorer.keyFor({ projectId: 'a:b', section: 'c' }), explorer.keyFor({ projectId: 'a', section: 'b:c' }));
});
test('corrupt, wrong-scope, stale-column and oversized preferences remain unchanged and visibly fail instead of resetting', () => {
  const key = explorer.keyFor(context), base = { version: 1, ...context, visibleKeys: ['url'] };
  const cases = ['{broken', 'null', JSON.stringify({ ...base, version: 2 }), JSON.stringify({ ...base, section: 'wrong' }), JSON.stringify({ ...base, projectId: 'wrong' }), JSON.stringify({ ...base, extra: true }), JSON.stringify({ ...base, visibleKeys: ['url', 'url'] }), JSON.stringify({ ...base, visibleKeys: ['removed'] }), JSON.stringify({ ...base, visibleKeys: null }), ' '.repeat(65537), '雪'.repeat(21846)];
  for (const raw of cases) { const db = storage([[key, raw]]); const loaded = explorer.load(db, context, columns); assert.equal(loaded.ok, false); assert(loaded.error.code); assert.equal(db.data.get(key), raw); assert.deepEqual(db.calls, [['get', key]]); }
});
test('unavailable or failed browser storage is an explicit Result, preserving bytes and never automatically writing', () => {
  for (const db of [null, {}, { getItem() { throw Error('Owned read refusal'); }, setItem() { throw Error('Owned quota refusal'); }, removeItem() { throw Error('Owned remove refusal'); } }]) {
    for (const outcome of [explorer.load(db, context, columns), explorer.save(db, context, columns, { visibleKeys: ['url'] }), explorer.reset(db, context)]) { assert.equal(outcome.ok, false); assert.equal(outcome.error.code, 'storage_unavailable'); }
  }
  const key = explorer.keyFor(context), db = storage([[key, 'prior bytes']]); db.setItem = () => { throw Error('Quota'); };
  assert.equal(explorer.save(db, context, columns, { visibleKeys: ['url'] }).ok, false); assert.equal(db.data.get(key), 'prior bytes');
  assert.equal(explorer.save(db, context, columns, { visibleKeys: ['url', 'url'] }).ok, false); assert.equal(db.data.get(key), 'prior bytes');
  assert.deepEqual(db.calls, []);
});
test('reset explicitly removes only its current scope, and invalid contexts do not access storage', () => {
  const key = explorer.keyFor(context), other = explorer.keyFor({ ...context, section: 'meta' }), db = storage([[key, 'owned'], [other, 'retain']]);
  assert.deepEqual(explorer.reset(db, context), { ok: true, value: null }); assert.equal(db.data.has(key), false); assert.equal(db.data.get(other), 'retain'); assert.deepEqual(db.calls, [['remove', key]]);
  for (const invalid of [{ projectId: '', section: 'titles' }, { ...context, section: '\u0000' }, { ...context, projectId: '雪'.repeat(86) }, { ...context, extra: true }]) { const untouched = storage(); assert.equal(explorer.load(untouched, invalid, columns).ok, false); assert.equal(explorer.save(untouched, invalid, columns, { visibleKeys: ['url'] }).ok, false); assert.equal(explorer.reset(untouched, invalid).ok, false); assert.deepEqual(untouched.calls, []); }
});
test('browser UMD installs only its namespace and constructing/compiling never accesses storage or network', () => {
  const source = fs.readFileSync(path.join(__dirname, '..', 'table-explorer.js'), 'utf8'), context = vm.createContext({ TextEncoder, Intl });
  let forbidden = 0; for (const name of ['localStorage', 'fetch']) Object.defineProperty(context, name, { get() { forbidden++; throw Error('Unexpected side effect'); } });
  vm.runInContext(source, context); assert.equal(typeof context.TableExplorer.compile, 'function');
  const compiled = context.TableExplorer.compile(columns, view()); assert.equal(compiled.ok, true); assert.equal(compiled.value.rows([{ url: 'owned', count: 0 }]).value.length, 1); assert.equal(forbidden, 0);
});
