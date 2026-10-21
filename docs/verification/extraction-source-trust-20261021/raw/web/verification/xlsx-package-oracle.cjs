'use strict';
// Independent acceptance reader. Never import TableXlsx, App, table compiler,
// projection builder, or a writer-generated expected matrix into this module.
const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { inflateRawSync } = require('node:zlib');
const PARTS = Object.freeze(['[Content_Types].xml', '_rels/.rels', 'xl/workbook.xml', 'xl/_rels/workbook.xml.rels', 'xl/worksheets/sheet1.xml']);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const utf8 = new TextDecoder('utf-8', { fatal: true });

// Deliberately tableless; no implementation shared with the writer's CRC table.
function crc32(bytes) {
  let current = 0xffffffff;
  for (const byte of bytes) {
    current ^= byte;
    for (let bit = 0; bit < 8; bit++) current = (current >>> 1) ^ ((current & 1) ? 0xedb88320 : 0);
  }
  return (current ^ 0xffffffff) >>> 0;
}
function packageParts(input, { maxBytes = 32 * 1024 * 1024, storedOnly = true } = {}) {
  const bytes = Buffer.from(input);
  assert(bytes.length >= 22 && bytes.length <= maxBytes, 'Complete package within explicit byte bound');
  const end = bytes.length - 22;
  assert.equal(bytes.readUInt32LE(end), 0x06054b50, 'EOCD is terminal; comments/trailing bytes refused');
  assert.equal(bytes.readUInt16LE(end + 4), 0); assert.equal(bytes.readUInt16LE(end + 6), 0);
  assert.equal(bytes.readUInt16LE(end + 8), PARTS.length); assert.equal(bytes.readUInt16LE(end + 10), PARTS.length);
  assert.equal(bytes.readUInt16LE(end + 20), 0);
  const centralSize = bytes.readUInt32LE(end + 12), centralOffset = bytes.readUInt32LE(end + 16);
  assert.equal(centralOffset + centralSize, end, 'No padding, ZIP64, or missing central bytes');
  assert(centralOffset > 0 && centralOffset < end);
  let at = centralOffset, nextLocal = 0, totalDecoded = 0;
  const entries = [], xml = {};
  for (let index = 0; index < PARTS.length; index++) {
    assert(at + 46 <= end, 'Complete central entry'); assert.equal(bytes.readUInt32LE(at), 0x02014b50);
    const made = bytes.readUInt16LE(at + 4), needed = bytes.readUInt16LE(at + 6), flags = bytes.readUInt16LE(at + 8), method = bytes.readUInt16LE(at + 10);
    const clock = bytes.readUInt16LE(at + 12), date = bytes.readUInt16LE(at + 14), crc = bytes.readUInt32LE(at + 16);
    const compressed = bytes.readUInt32LE(at + 20), uncompressed = bytes.readUInt32LE(at + 24);
    const nameLength = bytes.readUInt16LE(at + 28), extra = bytes.readUInt16LE(at + 30), comment = bytes.readUInt16LE(at + 32);
    assert.equal(made, 20); assert.equal(needed, 20); assert.equal(flags, 0x800, 'Only UTF-8 flag, no encryption or descriptors');
    assert(storedOnly ? method === 0 : [0, 8].includes(method), 'Reviewed compression method');
    assert.equal(clock, 0); assert.equal(date, 33, 'Deterministic ZIP date');
    assert.equal(extra, 0); assert.equal(comment, 0); assert.equal(bytes.readUInt16LE(at + 34), 0);
    assert.equal(bytes.readUInt16LE(at + 36), 0); assert.equal(bytes.readUInt32LE(at + 38), 0);
    assert(at + 46 + nameLength <= end, 'Central name inside bounds');
    const name = utf8.decode(bytes.subarray(at + 46, at + 46 + nameLength)); assert.equal(name, PARTS[index], 'Exact ordered allowlist of parts');
    const local = bytes.readUInt32LE(at + 42); assert.equal(local, nextLocal, 'No local-entry gaps, overlaps, or reordering');
    assert(local + 30 <= centralOffset); assert.equal(bytes.readUInt32LE(local), 0x04034b50);
    assert.equal(bytes.readUInt16LE(local + 4), needed); assert.equal(bytes.readUInt16LE(local + 6), flags);
    assert.equal(bytes.readUInt16LE(local + 8), method); assert.equal(bytes.readUInt16LE(local + 10), clock); assert.equal(bytes.readUInt16LE(local + 12), date);
    assert.equal(bytes.readUInt32LE(local + 14), crc); assert.equal(bytes.readUInt32LE(local + 18), compressed); assert.equal(bytes.readUInt32LE(local + 22), uncompressed);
    assert.equal(bytes.readUInt16LE(local + 26), nameLength); assert.equal(bytes.readUInt16LE(local + 28), 0);
    assert.equal(utf8.decode(bytes.subarray(local + 30, local + 30 + nameLength)), name);
    assert(uncompressed <= maxBytes && compressed <= maxBytes);
    const start = local + 30 + nameLength, finish = start + compressed; assert(finish <= centralOffset);
    const data = method === 0 ? bytes.subarray(start, finish) : inflateRawSync(bytes.subarray(start, finish), { maxOutputLength: maxBytes });
    assert.equal(data.length, uncompressed); assert.equal(crc32(data), crc, 'Independent decoded part CRC: ' + name);
    totalDecoded += data.length; assert(totalDecoded <= maxBytes, 'Decoded package byte bound');
    const source = utf8.decode(data); assert(!/<!DOCTYPE|<!ENTITY/i.test(source), 'No DTD or entity declarations');
    xml[name] = source; entries.push({ name, method, crc32: crc.toString(16).padStart(8, '0'), bytes: data.length, sha256: sha(data) });
    nextLocal = finish; at += 46 + nameLength;
  }
  assert.equal(nextLocal, centralOffset); assert.equal(at, end);
  return { xml, entries, bytes: bytes.length, sha256: sha(bytes) };
}

// This function is serialized into a fresh about:blank page, not the application
// realm. The browser's standards XML parser is independent of the product writer.
function parseInFreshRealm({ xml, expected }) {
  const fail = message => { throw new Error('Independent XLSX XML: ' + message); };
  const same = (actual, value, message) => { if (JSON.stringify(actual) !== JSON.stringify(value)) fail(message + ': ' + JSON.stringify(actual)); };
  const ns = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main', rel = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships';
  const packageRel = 'http://schemas.openxmlformats.org/package/2006/relationships', ct = 'http://schemas.openxmlformats.org/package/2006/content-types';
  const docs = {};
  for (const [name, source] of Object.entries(xml)) {
    const doc = new DOMParser().parseFromString(source, 'application/xml');
    if (doc.getElementsByTagName('parsererror').length || doc.doctype) fail('Malformed or DTD XML: ' + name);
    if (doc.getElementsByTagNameNS(ns, 'f').length || doc.getElementsByTagNameNS(ns, 'hyperlink').length) fail('Formula or hyperlink element: ' + name);
    docs[name] = doc;
  }
  function root(name, namespace, localName) {
    const element = docs[name].documentElement;
    if (element.namespaceURI !== namespace || element.localName !== localName) fail('Unexpected XML root: ' + name);
    return element;
  }
  function elements(parent, namespace, name) {
    const children = [...parent.children];
    if (children.some(node => node.namespaceURI !== namespace || node.localName !== name)) fail('Unexpected child of ' + parent.localName);
    return children;
  }
  function attrs(element, required) {
    same([...element.attributes].filter(a => a.namespaceURI !== 'http://www.w3.org/2000/xmlns/').map(a => [a.namespaceURI || '', a.localName, a.value]).sort(), required.map(([name, value, namespace = '']) => [namespace, name, value]).sort(), 'Exact ' + element.localName + ' attributes');
  }
  function only(parent, namespace, name) { const all = elements(parent, namespace, name); if (all.length !== 1) fail('Expected exactly one ' + name); return all[0]; }
  const types = root('[Content_Types].xml', ct, 'Types');
  same([...types.children].map(e => [e.localName, Object.fromEntries([...e.attributes].map(a => [a.name, a.value]))]), [
    ['Default', { Extension: 'rels', ContentType: 'application/vnd.openxmlformats-package.relationships+xml' }],
    ['Default', { Extension: 'xml', ContentType: 'application/xml' }],
    ['Override', { PartName: '/xl/workbook.xml', ContentType: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml' }],
    ['Override', { PartName: '/xl/worksheets/sheet1.xml', ContentType: 'application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml' }]
  ], 'Complete content type declarations');
  if ([...types.children].some(e => e.namespaceURI !== ct || e.children.length)) fail('Invalid content type node');
  function relationship(name, type, target) {
    const r = only(root(name, packageRel, 'Relationships'), packageRel, 'Relationship');
    attrs(r, [['Id', 'rId1'], ['Type', rel + '/' + type], ['Target', target]]);
    if (r.children.length) fail('Relationship child');
  }
  relationship('_rels/.rels', 'officeDocument', 'xl/workbook.xml');
  relationship('xl/_rels/workbook.xml.rels', 'worksheet', 'worksheets/sheet1.xml');
  const book = root('xl/workbook.xml', ns, 'workbook'), sheet = only(only(book, ns, 'sheets'), ns, 'sheet');
  attrs(sheet, [['name', 'Ergebnisse'], ['sheetId', '1'], ['id', 'rId1', rel]]); if (sheet.children.length) fail('Sheet children');
  const matrix = [expected.header, ...expected.rows];
  if (!matrix.length || !expected.header.length || matrix.some(row => row.length !== expected.header.length)) fail('Malformed independent expectation');
  const columnName = index => { let n = index + 1, result = ''; while (n > 0) { const digit = (n - 1) % 26; result = String.fromCharCode(65 + digit) + result; n = Math.floor((n - 1) / 26); } return result; };
  const ws = root('xl/worksheets/sheet1.xml', ns, 'worksheet');
  same([...ws.children].map(e => [e.namespaceURI, e.localName]), [[ns, 'dimension'], [ns, 'sheetData']], 'Worksheet child structure');
  const [dimension, data] = ws.children;
  attrs(dimension, [['ref', 'A1:' + columnName(expected.header.length - 1) + matrix.length]]); if (dimension.children.length) fail('Dimension children');
  const rows = elements(data, ns, 'row'); same(rows.length, matrix.length, 'Complete physical row count');
  const actual = [], typesActual = [], stats = { string: 0, number: 0, boolean: 0, null: 0, empty: 0 };
  const decodeXstringOnce = text => text.replace(/_x([0-9a-fA-F]{4})_/g, (_, value) => String.fromCharCode(parseInt(value, 16)));
  for (let rowIndex = 0; rowIndex < rows.length; rowIndex++) {
    const row = rows[rowIndex], wanted = matrix[rowIndex]; attrs(row, [['r', String(rowIndex + 1)]]);
    const cells = elements(row, ns, 'c'); same(cells.length, wanted.length, 'Dense cell count at row ' + (rowIndex + 1));
    const values = [], kinds = [];
    for (let column = 0; column < cells.length; column++) {
      const cell = cells[column], address = columnName(column) + (rowIndex + 1), expectedValue = wanted[column];
      const kind = expectedValue === null ? 'null' : typeof expectedValue;
      if (!['null', 'string', 'number', 'boolean'].includes(kind)) fail('Unsupported expected scalar ' + address);
      let value;
      if (kind === 'null') { attrs(cell, [['r', address]]); if (cell.children.length || cell.textContent) fail('Nonempty null ' + address); value = null; }
      else if (kind === 'string') {
        attrs(cell, [['r', address], ['t', 'inlineStr']]);
        const inline = only(cell, ns, 'is'); attrs(inline, []); const text = only(inline, ns, 't');
        attrs(text, [['space', 'preserve', 'http://www.w3.org/XML/1998/namespace']]); if (text.children.length) fail('Markup inside inline text ' + address);
        value = decodeXstringOnce(text.textContent); if (value === '') stats.empty++;
      } else {
        attrs(cell, [['r', address], ['t', kind === 'number' ? 'n' : 'b']]);
        const v = only(cell, ns, 'v'); attrs(v, []); if (v.children.length) fail('Markup inside scalar ' + address);
        if (kind === 'boolean') { if (!['0', '1'].includes(v.textContent)) fail('Boolean wire spelling ' + address); value = v.textContent === '1'; }
        else { value = Number(v.textContent); if (!Number.isFinite(value) || String(expectedValue) !== v.textContent) fail('Lossless numeric spelling ' + address); }
      }
      same(value, expectedValue, 'Exact independently expected scalar at ' + address); stats[kind]++; values.push(value); kinds.push(kind);
    }
    actual.push(values); typesActual.push(kinds);
  }
  return { sheet: 'Ergebnisse', header: actual[0], rows: actual.slice(1), types: typesActual, dimension: dimension.getAttribute('ref'), stats, reader: 'Fresh detached Chromium DOMParser + one-pass ST_Xstring decode after XML entities' };
}
async function inspectWorkbook(browser, bytes, expected) {
  const archive = packageParts(bytes), context = await browser.newContext();
  let page;
  try {
    page = await context.newPage();
    assert.equal(page.url(), 'about:blank', 'XML oracle has no application document or scripts');
    const decoded = await page.evaluate(parseInFreshRealm, { xml: archive.xml, expected });
    assert.deepEqual(decoded.header, expected.header); assert.deepEqual(decoded.rows, expected.rows);
    return { ...decoded, package: { bytes: archive.bytes, sha256: archive.sha256, entries: archive.entries }, matrix_sha256: sha(Buffer.from(JSON.stringify([decoded.header, ...decoded.rows]))) };
  } finally { await context.close(); }
}
module.exports = { PARTS, crc32, packageParts, parseInFreshRealm, inspectWorkbook };
