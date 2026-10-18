'use strict';

// Executes the production script with deferred local API replies. These tests
// prove UI ownership and response ordering; provider/native persistence is
// covered separately by the live browser suite.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { createHash, webcrypto } = require('node:crypto');

const SCRIPT = fs.readFileSync(path.join(__dirname, '..', 'api-enrichment.js'), 'utf8');
const A = '11111111-1111-4111-8111-111111111111';
const B = '22222222-2222-4222-8222-222222222222';
const SNAPSHOT = 'a'.repeat(64);
const flush = () => new Promise(resolve => setImmediate(resolve));
const clone = value => JSON.parse(JSON.stringify(value));

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
    this.disabled = false;
    this.checked = false;
    this.children = [];
    this.handlers = new Map();
  }
  addEventListener(event, handler) { this.handlers.set(event, handler); }
  fire(event, payload = {}) { return this.handlers.get(event)?.(payload); }
  click() { if (!this.disabled) return this.fire('click'); }
  append(child) { this.children.push(child); }
  replaceChildren() { this.children = []; }
}

function harness() {
  const elements = new Map(), requests = [], downloads = [], timers = new Map();
  const objects = new Map(), storage = new Map();
  let timerId = 0, objectId = 0;
  const get = id => {
    if (!elements.has(id)) elements.set(id, new Element());
    return elements.get(id);
  };
  get('format').value = 'txt';
  get('strategy').value = 'mobile';
  get('profile').value = 'anonymous';
  const context = {
    TextEncoder, Uint8Array, structuredClone, Blob, crypto: webcrypto,
    document: {
      getElementById: get,
      createElement(tag) {
        const element = new Element(tag);
        if (tag === 'a') element.click = () => downloads.push({ name: element.download, blob: objects.get(element.href) });
        return element;
      },
    },
    window: { addEventListener() {} },
    localStorage: {
      getItem: key => storage.get(key) ?? null,
      setItem: (key, value) => storage.set(key, value),
    },
    fetch(route, options) {
      assert.match(route, /^\/api\/enrichments(?:\/|$)/, 'Only the local enrichment API is used');
      const pending = deferred();
      requests.push({ route, method: options.method, body: options.body ? JSON.parse(options.body) : undefined, ...pending });
      return pending.promise;
    },
    setTimeout(callback, delay) { const id = ++timerId; timers.set(id, { callback, delay }); return id; },
    clearTimeout(id) { timers.delete(id); },
    URL: {
      createObjectURL(blob) { const id = `blob:owned-enrichment-${++objectId}`; objects.set(id, blob); return id; },
      revokeObjectURL(id) { objects.delete(id); },
    },
  };
  vm.createContext(context);
  vm.runInContext(SCRIPT, context, { filename: 'web/api-enrichment.js' });
  return {
    get, requests, downloads, timers,
    request(route) {
      const request = requests.find(value => value.route === route && !value.responded);
      assert.ok(request, `Missing pending request: ${route}`);
      return request;
    },
    reply(request, value, status = 200) {
      request.responded = true;
      request.resolve(new Response(JSON.stringify(value), { status, headers: { 'content-type': 'application/json' } }));
    },
    fail(request, message) { request.responded = true; request.reject(Error(message)); },
    csv(request, bytes) {
      request.responded = true;
      request.resolve(new Response(bytes, { headers: {
        'content-length': String(bytes.length),
        'x-content-sha256': createHash('sha256').update(bytes).digest('hex'),
      } }));
    },
    reopen(id) { get('job').value = id; return get('reopen').click(); },
  };
}

function configuration(id) {
  return {
    acknowledge_remote_fetch: false,
    input_utf8: `https://owned.invalid/${id}`,
    import: { format: { kind: 'txt' } },
    options: { version: 1, strategy: 'mobile', credential_profile: 'anonymous' },
  };
}

function resultPage(id, { start = 1, count = 1, total = count, after = null } = {}) {
  return {
    id, generation: 1, snapshot: SNAPSHOT, status: 'complete', total, after,
    provider_request_attempts: total, application_website_requests: 0,
    rows: Array.from({ length: count }, (_, index) => ({
      input: { source_row: start + index, raw_value: `https://owned.invalid/${id}/${start + index}` },
      provider_observation: { outcome: 'success', api_http_status: 200, metrics: { performance_score: 0.9, largest_contentful_paint_ms: 1234 } },
    })),
  };
}

async function reopenThroughConfiguration(h, id) {
  const opening = h.reopen(id);
  h.reply(h.request(`/api/enrichments/${id}/configuration`), configuration(id));
  await flush();
  return { opening, status: h.request(`/api/enrichments/${id}`) };
}

async function completed(h, id = A, options) {
  const { opening, status } = await reopenThroughConfiguration(h, id);
  h.reply(status, { id, status: 'complete', snapshot: SNAPSHOT });
  await flush();
  h.reply(h.request(`/api/enrichments/${id}/results`), resultPage(id, options));
  await opening;
}

test('completed reopen, append pagination and exact-snapshot CSV keep the whole report', async () => {
  const h = harness();
  await completed(h, A, { count: 100, total: 105, after: 'owned-next-cursor' });
  assert.equal(h.get('input').value, configuration(A).input_utf8);
  assert.equal(h.get('ack').checked, false);
  assert.equal(h.get('start').disabled, true);
  assert.equal(h.get('rows').children.length, 100);
  const next = h.get('next').click();
  const continuation = h.request(`/api/enrichments/${A}/results`);
  assert.deepEqual(clone(continuation.body), { expected_snapshot: SNAPSHOT, after: 'owned-next-cursor', limit: 100, outcome: null });
  h.reply(continuation, resultPage(A, { start: 101, count: 5, total: 105 }));
  await next;
  assert.deepEqual(h.get('rows').children.map(row => Number(row.children[0].textContent)), Array.from({ length: 105 }, (_, index) => index + 1));
  assert.equal(h.get('next').disabled, true);
  const exporting = h.get('export').click(), request = h.request(`/api/enrichments/${A}/csv`);
  assert.deepEqual(clone(request.body), { expected_snapshot: SNAPSHOT, after: null, limit: 100, outcome: null });
  const bytes = Buffer.from('source_row,raw_value\r\n1,"owned 雪"\r\n');
  h.csv(request, bytes);
  await exporting;
  assert.equal(h.downloads.length, 1);
  assert.equal(h.downloads[0].name, `api-enrichment-${A}.csv`);
  assert.deepEqual(Buffer.from(await h.downloads[0].blob.arrayBuffer()), bytes);
  assert.equal(h.get('error').textContent, '');
});

test('terminal completed job cannot be presented as waiting for cancellation', async () => {
  const h = harness();
  await completed(h);
  const status = h.get('status').textContent;
  // Baseline erroneously enables the control. If enabled, exercise the real
  // backend's idempotent terminal-cancel reply rather than invent cancelling.
  if (!h.get('cancel').disabled) {
    const cancelling = h.get('cancel').click();
    h.reply(h.request(`/api/enrichments/${A}/cancel`), { id: A, status: 'complete', snapshot: SNAPSHOT });
    await cancelling;
  }
  assert.equal(h.get('status').textContent, status);
  assert.equal(h.get('cancel').disabled, true);
});

test('late status failure from A cannot overwrite successfully reopened B', async () => {
  const h = harness();
  const older = await reopenThroughConfiguration(h, A);
  await completed(h, B);
  const status = h.get('status').textContent;
  h.fail(older.status, 'old A status failed');
  await older.opening;
  assert.equal(h.get('error').textContent, '');
  assert.equal(h.get('status').textContent, status);
  assert.equal(h.get('rows').children[0].children[1].textContent, `https://owned.invalid/${B}/1`);
});

for (const outcome of ['success', 'failure']) {
  test(`late cancel ${outcome} from A cannot overwrite successfully reopened B`, async () => {
    const h = harness();
    const older = await reopenThroughConfiguration(h, A);
    h.reply(older.status, { id: A, status: 'running' });
    await older.opening;
    const cancelling = h.get('cancel').click(), request = h.request(`/api/enrichments/${A}/cancel`);
    await completed(h, B);
    const status = h.get('status').textContent;
    if (outcome === 'success') h.reply(request, { id: A, status: 'cancelling' });
    else h.fail(request, 'old A cancellation failed');
    await cancelling;
    assert.equal(h.get('error').textContent, '');
    assert.equal(h.get('status').textContent, status);
  });
}

test('older start status completion cannot unlock a newer pending reopen', async () => {
  const h = harness();
  h.get('input').value = 'https://owned.invalid/a';
  const previewing = h.get('configuration').fire('submit', { preventDefault() {} });
  h.reply(h.request('/api/enrichments/preview'), { fingerprint: 'owned-preview', supplied_rows: 1, unique_provider_urls: 1 });
  await previewing;
  h.get('ack').checked = true;
  h.get('ack').fire('change');
  const starting = h.get('start').click();
  h.reply(h.request('/api/enrichments'), { id: A, status: 'running' });
  await flush();
  const oldStatus = h.request(`/api/enrichments/${A}`);
  const opening = h.reopen(B);
  assert.equal(h.get('reopen').disabled, true);
  h.reply(oldStatus, { id: A, status: 'complete', snapshot: SNAPSHOT });
  await starting;
  const retainedBusy = h.get('reopen').disabled && h.get('preview').disabled && h.get('input').disabled;
  h.reply(h.request(`/api/enrichments/${B}/configuration`), configuration(B));
  await flush();
  h.reply(h.request(`/api/enrichments/${B}`), { id: B, status: 'complete', snapshot: SNAPSHOT });
  await flush();
  h.reply(h.request(`/api/enrichments/${B}/results`), resultPage(B));
  await opening;
  assert.equal(retainedBusy, true, 'Only the newer reopen owns its pending configuration busy state');
});

test('a stale-snapshot API refusal invalidates continuation and CSV permission', async () => {
  const h = harness();
  await completed(h, A, { total: 101, after: 'owned-next-cursor' });
  const next = h.get('next').click();
  h.reply(h.request(`/api/enrichments/${A}/results`), { error: { message: 'Stale API-only snapshot' } }, 409);
  await next;
  assert.match(h.get('error').textContent, /Stale API-only snapshot/);
  assert.equal(h.get('next').disabled, true);
  assert.equal(h.get('export').disabled, true);
});

test('a stale-snapshot CSV refusal cannot retain export permission or create a download', async () => {
  const h = harness();
  await completed(h);
  const exporting = h.get('export').click();
  h.reply(h.request(`/api/enrichments/${A}/csv`), { error: { message: 'CSV requires the exact current snapshot' } }, 409);
  await exporting;
  assert.match(h.get('error').textContent, /exact current snapshot/);
  assert.equal(h.get('export').disabled, true);
  assert.equal(h.downloads.length, 0);
});
