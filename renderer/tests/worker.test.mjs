import { test } from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import { setTimeout as delay } from 'node:timers/promises';
import { createRendererServer, findBrowserExecutable, JobPool, validateRenderRequest } from '../server.mjs';

const listen = server => new Promise(resolve => server.listen(0, '127.0.0.1', () => resolve(`http://127.0.0.1:${server.address().port}`)));
const close = server => new Promise(resolve => { server.close(resolve); server.closeAllConnections(); });
const html = (title, body = '', script = '') => `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>${title}</title></head><body>${body}<script>${script}</script></body></html>`;

test('validation rejects non-HTTP URLs, header injection and unbounded options', () => {
  for (const input of [{ url: 'file:///C:/secret' }, { url: 'http://user:password@fixture.test/' },
    { url: 'https://fixture.test/', custom_headers: { 'X-Test': 'a\r\nb' } },
    { url: 'https://fixture.test/', custom_headers: { Host: 'other.test' } },
    { url: 'https://fixture.test/', custom_headers: { Test: 'a', test: 'b' } },
    { url: 'https://fixture.test/', timeout_ms: 0 },
    { url: 'https://fixture.test/', screenshot: 'yes' }]) {
    assert.throws(() => validateRenderRequest(input), error => error.status === 400);
  }
  assert.equal(validateRenderRequest({url: 'https://fixture.test/'}).wait_ms, 1000);
});

test('job pool caps concurrency, queue growth and shutdown', async () => {
  const pool = new JobPool(4, 3); let active = 0; let maximum = 0;
  const tasks = Array.from({length: 7}, () => pool.run(async () => {
    active += 1; maximum = Math.max(maximum, active); await delay(15); active -= 1;
  }));
  await assert.rejects(pool.run(async () => {}), error => error.code === 'queue_full');
  await Promise.all(tasks); await pool.waitForIdle(); assert.equal(maximum, 4);
  assert.equal(pool.active, 0); pool.close();
  await assert.rejects(pool.run(async () => {}), error => error.code === 'shutting_down');
  const draining = new JobPool(1, 1);
  const first = draining.run(() => delay(10)); const queued = draining.run(() => delay(10));
  draining.close(); await assert.rejects(queued, error => error.code === 'shutting_down');
  await first; await draining.waitForIdle();
});

test('real installed browser renders JavaScript, isolates jobs and audits owned fixtures', { timeout: 60000 }, async t => {
  const executable = await findBrowserExecutable(); assert.ok(executable);
  const observedForeign = []; const observedMain = [];
  const foreign = http.createServer((request, response) => {
    observedForeign.push({url: request.url, headers: request.headers});
    if (request.url === '/resource.js') {
      response.writeHead(200, {'content-type': 'text/javascript'}); response.end("window.foreignLoaded=true;");
    } else {
      response.writeHead(200, {'content-type': 'text/html'}); response.end(html('Foreign target'));
    }
  });
  const foreignOrigin = await listen(foreign);
  const fixture = http.createServer((request, response) => {
    observedMain.push({url: request.url, headers: request.headers});
    if (request.url === '/redirect-external') {
      response.writeHead(302, {location: `${foreignOrigin}/target`}); response.end(); return;
    }
    if (request.url === '/redirect-local') {
      response.writeHead(302, {location: '/dynamic'}); response.end(); return;
    }
    if (request.url === '/slow') {
      const pending = setTimeout(() => { if (!response.destroyed) { response.writeHead(200, {'content-type': 'text/html'}); response.end(html('Too late')); } }, 3000);
      response.on('close', () => clearTimeout(pending)); return;
    }
    response.writeHead(request.url === '/missing' ? 404 : 200,
      {'content-type': 'text/html; charset=utf-8', 'x-fixture': 'rendered-evidence'});
    if (request.url === '/dynamic') {
      response.end(html('Raw title', '<main><h1>Owned rendering fixture</h1></main>', `setTimeout(() => {
        document.title='Rendered SEO title'; const link=document.createElement('a'); link.id='late';
        link.href='/js-only'; link.textContent='Discovered after JavaScript'; document.querySelector('main').appendChild(link);
        console.error('owned-fixture-console'); }, 30);`));
    } else if (request.url === '/external') {
      response.end(html('External resource', `<script src="${foreignOrigin}/resource.js"></script>`));
    } else if (request.url === '/cookie-set') {
      response.end(html('Setter', '', "localStorage.setItem('isolation','private-job');document.cookie='isolated=yes;path=/';"));
    } else if (request.url === '/cookie-check') {
      response.end(html('Checker', '', "document.title=(localStorage.getItem('isolation')||'fresh')+'|'+(document.cookie||'empty');"));
    } else if (request.url === '/accessibility') {
      response.end(html('Accessibility fixture', '<main><h1>Fixture</h1><button></button><img src="data:image/svg+xml,%3Csvg xmlns=%22http://www.w3.org/2000/svg%22/%3E"></main>'));
    } else { response.end(html(request.url === '/missing' ? 'Not found' : 'Fixture page')); }
  });
  const fixtureOrigin = await listen(fixture);
  const worker = createRendererServer();
  const address = await worker.listen(0); const origin = `http://127.0.0.1:${address.port}`;
  t.after(async () => { await worker.close(); await close(fixture); await close(foreign); });
  const render = async (route, extra = {}) => {
    const response = await fetch(`${origin}/render`, { method: 'POST', headers: {'content-type': 'application/json'},
      body: JSON.stringify({url: `${fixtureOrigin}${route}`, wait_ms: 150, timeout_ms: 15000, ...extra}) });
    return {status: response.status, body: await response.json()};
  };

  await t.test('health is available before launching a browser', async () => {
    const response = await fetch(`${origin}/health`); const value = await response.json();
    assert.equal(response.status, 200); assert.equal(value.browser.available, true);
    assert.equal(value.max_concurrency, 4); assert.equal(value.active, 0);
  });
  await t.test('dynamic title and link, console errors, network and PNG are captured', async () => {
    const raw = await (await fetch(`${fixtureOrigin}/dynamic`)).text();
    assert.ok(raw.includes('<title>Raw title</title>')); assert.ok(!raw.includes('<a id="late"'));
    const result = await render('/dynamic', {screenshot: true, user_agent: 'OpenSEOFixture/1', custom_headers: {'X-Job': 'owned-fixture'}});
    assert.equal(result.status, 200); assert.equal(result.body.status, 200);
    assert.equal(result.body.title, 'Rendered SEO title'); assert.ok(result.body.html.includes('<a id="late"'));
    assert.ok(result.body.console_errors.some(error => error.message === 'owned-fixture-console'));
    assert.ok(result.body.network_requests.some(request => request.url.endsWith('/dynamic') && request.status === 200));
    assert.equal(result.body.headers['x-fixture'], 'rendered-evidence');
    assert.equal(Buffer.from(result.body.screenshot_base64, 'base64').subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
    assert.ok(observedMain.some(request => request.url === '/dynamic' && request.headers['user-agent'] === 'OpenSEOFixture/1'
      && request.headers['x-job'] === 'owned-fixture'));
  });
  await t.test('cookies and local storage do not leak across jobs', async () => {
    assert.equal((await render('/cookie-set')).status, 200);
    assert.equal((await render('/cookie-check')).body.title, 'fresh|empty');
    assert.ok(observedMain.filter(request => request.url === '/cookie-check').every(request => !request.headers.cookie));
  });
  await t.test('custom credentials stay on their origin for resources and redirects', async () => {
    const headers = {Authorization: 'Bearer OWNED_TEST_ONLY', 'X-Private-Fixture': 'owned-value'};
    const external = await render('/external', {custom_headers: headers});
    assert.equal(external.status, 200);
    const redirected = await render('/redirect-external', {custom_headers: headers});
    assert.equal(redirected.status, 200); assert.equal(redirected.body.final_url, `${foreignOrigin}/target`);
    assert.equal(redirected.body.title, 'Foreign target');
    assert.ok(observedMain.some(request => request.url === '/redirect-external' && request.headers.authorization === headers.Authorization));
    assert.ok(observedForeign.some(request => request.url === '/resource.js'), JSON.stringify({network: external.body.network_requests, errors: external.body.console_errors}));
    assert.ok(observedForeign.some(request => request.url === '/target'));
    assert.ok(observedForeign.every(request => !request.headers.authorization && !request.headers['x-private-fixture']));
    const local = await render('/redirect-local', {custom_headers: headers});
    assert.equal(local.body.final_url, `${fixtureOrigin}/dynamic`);
    assert.ok(observedMain.some(request => request.url === '/dynamic' && request.headers.authorization === headers.Authorization));
  });
  await t.test('axe violations are reported without claiming full accessibility compliance', async () => {
    const result = await render('/accessibility', {accessibility: true});
    assert.equal(result.status, 200); assert.equal(result.body.accessibility.engine, 'axe-core');
    assert.equal(result.body.accessibility.version, '4.14.0');
    assert.equal(result.body.accessibility.automated_only, true);
    const unnamedButton = result.body.accessibility.violations.find(violation => violation.id === 'button-name');
    assert.ok(unnamedButton); assert.equal(typeof unnamedButton.impact, 'string');
    assert.ok(unnamedButton.tags.some(tag => tag.startsWith('wcag')));
    assert.ok(unnamedButton.nodes.some(node => node.target.length > 0 && node.html.includes('<button')));
    assert.ok(result.body.accessibility.violations.some(violation => violation.id === 'image-alt'));
  });
  await t.test('HTTP error status remains an observation rather than a worker failure', async () => {
    const result = await render('/missing'); assert.equal(result.status, 200); assert.equal(result.body.status, 404);
  });
  await t.test('navigation timeout closes its context and releases capacity', async () => {
    const result = await render('/slow', {timeout_ms: 500, wait_ms: 0});
    assert.equal(result.status, 504); assert.equal(result.body.error.code, 'render_timeout');
    await worker.pool.waitForIdle(); assert.equal(worker.pool.active, 0);
    assert.equal((await render('/dynamic')).status, 200);
  });
  await t.test('HTTP boundary rejects malformed inputs and foreign browser origins', async () => {
    const nonJson = await fetch(`${origin}/render`, {method: 'POST', body: 'hello'}); assert.equal(nonJson.status, 415);
    const invalid = await fetch(`${origin}/render`, {method: 'POST', headers: {'content-type': 'application/json'}, body: '{'});
    assert.equal(invalid.status, 400);
    const foreignRequest = await fetch(`${origin}/render`, {method: 'POST', headers: {'content-type': 'application/json', origin: 'https://foreign.example'},
      body: JSON.stringify({url: `${fixtureOrigin}/dynamic`})}); assert.equal(foreignRequest.status, 403);
    const hostileHost = await new Promise((resolve, reject) => {
      const request = http.get(`${origin}/health`, {headers: {host: 'foreign.invalid'}}, response => {
        response.resume(); response.on('end', () => resolve(response.statusCode));
      }); request.on('error', reject);
    });
    assert.equal(hostileHost, 403);
    const foreignShutdown = await fetch(`${origin}/shutdown`, {method: 'POST', headers: {origin: 'https://foreign.example'}});
    assert.equal(foreignShutdown.status, 403);
  });
  await t.test('loopback shutdown responds then closes the browser and rejects new jobs', async () => {
    const response = await fetch(`${origin}/shutdown`, {method: 'POST'});
    assert.equal(response.status, 202); assert.equal((await response.json()).status, 'shutting_down');
    await worker.close(); assert.equal(worker.server.listening, false); assert.equal(worker.pool.closed, true);
    assert.equal(worker.pool.active, 0);
    await assert.rejects(worker.pool.run(async () => {}), error => error.code === 'shutting_down');
  });
});
