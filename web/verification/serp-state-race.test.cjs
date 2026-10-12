'use strict';

// Executes the actual editor script against an isolated DOM and deferred API.
// This tests request ordering and draft preservation, not native text metrics.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const SCRIPT = fs.readFileSync(path.join(__dirname, '..', 'serp.js'), 'utf8');
const KEY = 'open-seo-spider.serp.v1';
const row = (title, description = 'Description', url = 'https://never-requested.invalid/') => ({ url, title, description });
const flush = () => new Promise(resolve => setImmediate(resolve));

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

class Element {
  constructor(tag = '') {
    this.tag = tag;
    this.value = '';
    this.textContent = '';
    this.files = [];
    this.children = [];
    this.attributes = new Map();
    this.handlers = new Map();
  }
  setAttribute(name, value) { this.attributes.set(name, String(value)); }
  append(child) { this.children.push(child); }
  replaceChildren() { this.children = []; }
  addEventListener(event, handler) { this.handlers.set(event, handler); }
  fire(event, payload = {}) { return this.handlers.get(event)?.(payload); }
  click() { return this.fire('click'); }
}

function harness(initialRows = []) {
  const elements = new Map(), storage = new Map(), requests = [], downloads = [];
  const objects = new Map();
  let objectId = 0;
  const get = selector => {
    if (!elements.has(selector)) elements.set(selector, new Element());
    return elements.get(selector);
  };
  if (initialRows.length) storage.set(KEY, JSON.stringify(initialRows));
  const context = {
    document: {
      querySelector: get,
      createElement(tag) {
        const element = new Element(tag);
        if (tag === 'a') element.click = () => downloads.push({ name: element.download, blob: objects.get(element.href) });
        return element;
      },
    },
    localStorage: {
      getItem: key => storage.get(key) ?? null,
      setItem: (key, value) => storage.set(key, value),
    },
    fetch(route, options) {
      assert.ok(['/api/serp/preview', '/api/serp/import', '/api/serp/export.csv'].includes(route), `Unexpected network request: ${route}`);
      const pending = deferred();
      requests.push({ route, input: JSON.parse(options.body), ...pending });
      return pending.promise;
    },
    Blob,
    URL: {
      createObjectURL(blob) { const id = `blob:owned-test-${++objectId}`; objects.set(id, blob); return id; },
      revokeObjectURL(id) { objects.delete(id); },
    },
    setTimeout(callback) { callback(); return 0; },
  };
  vm.createContext(context);
  vm.runInContext(SCRIPT, context, { filename: 'web/serp.js' });
  const h = {
    get, requests, downloads,
    saved: () => JSON.parse(storage.get(KEY) || '[]'),
    text: selector => get(selector).textContent,
    input(selector, value) { get(selector).value = value; get(selector).fire('input'); },
    edit(values) {
      for (const [field, value] of Object.entries(values)) h.input(`#${field}`, value);
      get('#editor').fire('submit', { preventDefault() {} });
      return requests.at(-1);
    },
    importFile(text, name = 'owned.csv') {
      const file = { size: 128, text: typeof text === 'function' ? text : async () => text };
      get('#csv-file').files = [file]; get('#csv-file').value = name;
      return get('#csv-file').fire('change');
    },
    preview(request, rows = request.input.rows) {
      request.resolve({ ok: true, json: async () => ({
        rows: rows.map((value, index) => ({
          ...value, source_row: index + 1,
          title_characters: Array.from(value.title).length,
          description_characters: Array.from(value.description).length,
          title_pixels: value.title === '😀' ? null : value.title.length * 10,
          description_pixels: value.description.length * 10,
        })),
        measurement_profile: 'test-response-order-only', website_requests: 0,
      }) });
    },
    error(request, message) { request.resolve({ ok: false, status: 400, json: async () => ({ error: message }) }); },
    select(index) { get('#rows-table tbody').children[index].children[0].children[0].click(); },
  };
  return h;
}

async function ready(rows = [row('Before')]) {
  const h = harness(rows);
  h.preview(h.requests[0]);
  await flush();
  return h;
}

test('a submitted edit invalidates an import while File.text is still pending', async () => {
  const h = await ready(), reading = deferred();
  const importing = h.importFile(() => reading.promise);
  const update = h.edit({ title: 'Newer user edit' });
  h.preview(update); await flush();
  reading.resolve('URL,Title,Description\nlocal,Imported,Old CSV\n');
  await importing; await flush();
  assert.equal(h.saved()[0].title, 'Newer user edit');
  assert.equal(h.requests.filter(request => request.route === '/api/serp/import').length, 0);
  assert.equal(h.get('#csv-file').value, '');
});

test('a late import response cannot overwrite a newer submitted edit or persisted draft', async () => {
  const h = await ready();
  const importing = h.importFile('owned CSV'); await flush();
  const importRequest = h.requests.at(-1);
  const editRequest = h.edit({ title: 'Newer user edit' });
  h.preview(editRequest); await flush();
  const currentStatus = h.text('#status');
  h.preview(importRequest, [row('Imported')]); await importing;
  assert.equal(h.saved()[0].title, 'Newer user edit');
  assert.equal(h.get('#title').value, 'Newer user edit');
  assert.equal(h.text('#status'), currentStatus);
});

test('a newer import wins when responses complete in reverse order', async () => {
  const h = await ready();
  const older = h.importFile('first CSV', 'older.csv'); await flush();
  const firstRequest = h.requests.at(-1);
  const newer = h.importFile('second CSV', 'newer.csv'); await flush();
  const secondRequest = h.requests.at(-1);
  h.preview(secondRequest, [row('New import')]); await newer;
  h.preview(firstRequest, [row('Old import')]); await older;
  assert.deepEqual(h.saved(), [row('New import')]);
  assert.equal(h.get('#title').value, 'New import');
});

test('an older import completion cannot clear the newer selected file or status', async () => {
  const h = await ready();
  const older = h.importFile('first CSV', 'older.csv'); await flush();
  const firstRequest = h.requests.at(-1);
  const newer = h.importFile('second CSV', 'newer.csv'); await flush();
  const secondRequest = h.requests.at(-1), status = h.text('#status');
  h.error(firstRequest, 'Stale import failure'); await older;
  assert.equal(h.get('#csv-file').value, 'newer.csv');
  assert.equal(h.text('#status'), status);
  h.preview(secondRequest, [row('Latest')]); await newer;
  assert.equal(h.get('#csv-file').value, '');
  assert.equal(h.saved()[0].title, 'Latest');
});

test('a newer import also wins when file reads finish in reverse order', async () => {
  const h = await ready(), firstRead = deferred();
  const older = h.importFile(() => firstRead.promise, 'older.csv');
  const newer = h.importFile('second CSV', 'newer.csv'); await flush();
  const secondRequest = h.requests.at(-1);
  h.preview(secondRequest, [row('Latest')]); await newer;
  firstRead.resolve('older CSV'); await older;
  assert.equal(h.saved()[0].title, 'Latest');
  assert.equal(h.requests.filter(request => request.route === '/api/serp/import').length, 1);
});

test('adding a row invalidates an in-flight import and selects the new blank editor row', async () => {
  const h = await ready();
  const importing = h.importFile('owned CSV'); await flush();
  const importRequest = h.requests.at(-1);
  h.get('#add-row').click();
  const addedRequest = h.requests.at(-1);
  assert.equal(h.get('#title').value, '');
  assert.equal(h.text('#measurements'), '');
  h.preview(addedRequest); await flush();
  h.preview(importRequest, [row('Imported')]); await importing;
  assert.deepEqual(h.saved(), [row('Before'), { url: '', title: '', description: '' }]);
  assert.equal(h.text('#editing-row'), 'Zeile 2 bearbeiten');
});

test('deleting a row invalidates an in-flight import and clears measurements for shifted rows', async () => {
  const h = await ready([row('First'), row('Second')]);
  const importing = h.importFile('owned CSV'); await flush();
  const importRequest = h.requests.at(-1);
  h.get('#delete-row').click();
  const deletedRequest = h.requests.at(-1);
  assert.equal(h.get('#title').value, 'Second');
  assert.equal(h.text('#measurements'), '');
  h.preview(deletedRequest); await flush();
  h.preview(importRequest, [row('Imported')]); await importing;
  assert.deepEqual(h.saved(), [row('Second')]);
  assert.match(h.text('#measurements'), /6 Zeichen/);
});

test('an import replaces an older in-flight preview without the old metrics returning', async () => {
  const h = await ready();
  const oldPreview = h.edit({ title: 'Older preview' });
  const importing = h.importFile('owned CSV'); await flush();
  const importRequest = h.requests.at(-1);
  h.preview(importRequest, [row('New')]); await importing;
  h.preview(oldPreview); await flush();
  assert.deepEqual(h.saved(), [row('New')]);
  assert.match(h.text('#measurements'), /3 Zeichen/);
});

test('a stale preview error cannot replace the status of a completed import', async () => {
  const h = await ready();
  const oldPreview = h.edit({ title: 'Older preview' });
  const importing = h.importFile('owned CSV'); await flush();
  h.preview(h.requests.at(-1), [row('New')]); await importing;
  const status = h.text('#status');
  h.error(oldPreview, 'Stale preview failure'); await flush();
  assert.equal(h.text('#status'), status);
  assert.equal(h.saved()[0].title, 'New');
});

test('preview completion preserves text typed after the last submitted edit', async () => {
  const h = await ready();
  const measuring = h.edit({ title: 'Committed' });
  h.input('#title', 'Unsubmitted draft');
  h.preview(measuring); await flush();
  assert.equal(h.get('#title').value, 'Unsubmitted draft');
  assert.equal(h.saved()[0].title, 'Committed');
  assert.equal(h.text('#preview-title'), 'Committed');
});

test('an import cannot overwrite an unsubmitted newer form draft', async () => {
  const h = await ready();
  const importing = h.importFile('owned CSV'); await flush();
  const importingRequest = h.requests.at(-1);
  h.input('#title', 'Unsubmitted draft');
  h.preview(importingRequest, [row('Imported')]); await importing;
  assert.equal(h.get('#title').value, 'Unsubmitted draft');
  assert.equal(h.saved()[0].title, 'Before');
});

test('an older preview cannot restore a deleted row after the newer preview completes', async () => {
  const h = await ready([row('First'), row('Second')]);
  const older = h.edit({ title: 'Edited first' });
  h.select(1); h.get('#delete-row').click();
  const newer = h.requests.at(-1);
  h.preview(newer); await flush();
  h.preview(older); await flush();
  assert.deepEqual(h.saved(), [row('Edited first')]);
  assert.equal(h.get('#rows-table tbody').children.length, 1);
  assert.match(h.text('#measurements'), /12 Zeichen/);
});

test('JSON snapshots and CSV API inputs keep exact ordered duplicate rows and only local API requests', async () => {
  const values = [row('Über "uns"', 'two\nlines'), row('😀', ' \t=FORMULA()')];
  const h = await ready(values);
  h.get('#export-json').click();
  const json = JSON.parse(await h.downloads[0].blob.text());
  assert.deepEqual(json, { schema_version: 1, rows: values });
  const downloading = h.get('#export-csv').click();
  const request = h.requests.at(-1);
  assert.equal(request.route, '/api/serp/export.csv');
  assert.deepEqual(request.input.rows, values);
  request.resolve({ ok: true, blob: async () => new Blob(['owned CSV response']) });
  await downloading;
  assert.equal(h.downloads[1].name, 'serp.csv');
  assert.ok(h.requests.every(request => request.route.startsWith('/api/serp/')));
});
