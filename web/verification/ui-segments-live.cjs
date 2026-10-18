'use strict';
const assert = require('node:assert/strict');
const http = require('node:http');
const fs = require('node:fs');
const {createHash} = require('node:crypto');
const {chromium} = require('../../renderer/node_modules/playwright-core');
const artifactPath = require('./artifact-path.cjs');

const app = process.env.SPIDER_APP_URL;
assert(app && new URL(app).hostname === '127.0.0.1' && !['8787', '8790'].includes(new URL(app).port), 'Use an isolated owned native app');
const checks = [], errors = [];
let websiteHits = 0;
const documents = {
  '/index': {title: 'Owned home'},
  '/products/red': {title: 'Product Red'},
  '/products/blue': {title: 'Product Blue'},
  '/about': {title: 'About owned fixture'},
  '/untitled': {},
  '/hidden': {title: 'Hidden product', noindex: true},
  '/missing': {status: 404},
};
const website = http.createServer((request, response) => {
  websiteHits++;
  const document = documents[request.url] || (/^\/products\/batch-\d{3}$/.test(request.url) ? {title: 'Product Batch ' + request.url.slice(-3)} : undefined);
  response.writeHead(document?.status || (document ? 200 : 404), {'content-type': 'text/html; charset=utf-8'});
  response.end(`<html lang="en"><head>${document?.title ? `<title>${document.title}</title>` : ''}${document?.noindex ? '<meta name="robots" content="noindex">' : ''}</head><body><main>Owned saved-segments fixture.</main></body></html>`);
});
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
async function json(path, body) {
  const response = await fetch(app + path, {method: body === undefined ? 'GET' : 'POST', headers: body === undefined ? {} : {'content-type': 'application/json'}, body: body === undefined ? undefined : JSON.stringify(body)});
  assert.equal(response.status, 200, await response.clone().text());
  return response.json();
}
async function until(work, label) {
  for (let i = 0; i < 240; i++) {
    const value = await work();
    if (value) return value;
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  throw Error('Timeout: ' + label);
}
async function renderedRows(page, value) {
  const columns = value.view === 'pages'
    ? ['page_ordinal', 'url', 'document_url', 'status', 'depth', 'stored_title', 'matched_indices', 'unknown_indices']
    : ['code', 'severity', 'url', 'message', 'related_urls'];
  const expected = value.rows.map(row => columns.map(key => {
    if (key === 'matched_indices' || key === 'unknown_indices') return row[key].map(index => value.counts[index].name).join(', ');
    const cell = row[key];
    return cell === null ? 'Unbekannt' : typeof cell === 'object' ? JSON.stringify(cell) : String(cell ?? '');
  }));
  await page.waitForFunction(expectedRows => {
    const actual = [...document.querySelectorAll('#rows tr')].map(row => [...row.cells].map(cell => cell.textContent));
    return !document.getElementById('result').hidden && JSON.stringify(actual) === JSON.stringify(expectedRows);
  }, expected);
  assert.deepEqual(await page.locator('#rows tr').evaluateAll(rows => rows.map(row => [...row.cells].map(cell => cell.textContent))), expected,
    'Visible native result cells must match every field, row and ordering of the current page');
}
async function preview(page) {
  const responsePromise = page.waitForResponse(response => response.url().endsWith('/segments') && response.request().method() === 'POST');
  await page.getByRole('button', {name: 'Auswerten', exact: true}).click();
  const response = await responsePromise;
  assert.equal(response.status(), 200, await response.text());
  const bytes = await response.body();
  assert.equal(sha(bytes), response.headers()['x-content-sha256']);
  const value = JSON.parse(bytes);
  await page.waitForFunction(() => !document.getElementById('result').hidden);
  await renderedRows(page, value);
  assert.match(await page.locator('#summary').innerText(), new RegExp(`^${value.summary.selected_pages}/${value.summary.total_pages} Seiten`));
  return {value, request: response.request().postDataJSON()};
}
async function condition(fieldset, index, field, operator, value) {
  const row = fieldset.locator('.condition').nth(index);
  await row.locator('select').nth(0).selectOption(field);
  await row.locator('select').nth(1).selectOption(operator);
  await row.getByLabel('Wert', {exact: true}).fill(value);
}
async function download(page, button, suffix) {
  const responsePromise = page.waitForResponse(response => response.url().endsWith(suffix) && response.request().method() === 'POST');
  const downloadPromise = page.waitForEvent('download');
  await page.click(button);
  const response = await responsePromise;
  assert.equal(response.status(), 200, await response.text());
  const bytes = fs.readFileSync(await (await downloadPromise).path());
  assert.equal(sha(bytes), response.headers()['x-content-sha256']);
  assert.deepEqual(bytes, await response.body());
  return bytes;
}
// CSV parsing checks the exported records and JSON cells, including quoted commas/newlines.
function parseCsv(text) {
  const rows = []; let row = [], field = '', quoted = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === '"') {
      if (quoted && text[i + 1] === '"') { field += '"'; i++; } else quoted = !quoted;
    } else if (c === ',' && !quoted) { row.push(field); field = ''; }
    else if (c === '\n' && !quoted) { row.push(field.replace(/\r$/, '')); rows.push(row); row = []; field = ''; }
    else field += c;
  }
  assert.equal(quoted, false);
  if (field || row.length) { row.push(field); rows.push(row); }
  return rows;
}
async function refused(id, suffix, request, status) {
  const response = await fetch(`${app}/api/crawls/${id}/${suffix}`, {method: 'POST', headers: {'content-type': 'application/json'}, body: JSON.stringify(request)});
  assert.equal(response.status, status, await response.clone().text());
  return response.json();
}

(async () => {
  await new Promise(resolve => website.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${website.address().port}`;
  const start = await json('/api/crawls', {seed_urls: Object.keys(documents).map(path => origin + path), crawl_links: false, discover_sitemaps: false, obey_robots: false, delay_ms: 0, directive_reporting: {version: 1, respect_noindex: true, respect_canonical: false, respect_next_prev: false}});
  await until(async () => { const job = await json('/api/crawls/' + start.id); assert.notEqual(job.status, 'failed', job.error); return job.status === 'completed'; }, 'owned native seven-page crawl');
  const capturedHits = websiteHits;
  assert.equal(capturedHits, 7, 'Exactly the owned seven seeded documents must be captured');
  const executablePath = process.env.SPIDER_CHROME || process.env.OPEN_SEO_BROWSER_PATH || await (await import('../../renderer/server.mjs')).findBrowserExecutable();
  const browser = await chromium.launch({headless: true, executablePath});
  const page = await browser.newPage();
  page.on('pageerror', error => errors.push(error.message));
  try {
    await page.goto(app + '/saved-segments');
    await page.locator(`#job option[value="${start.id}"]`).waitFor({state: 'attached'});
    await page.selectOption('#job', start.id);
    await page.click('#add');
    let segment = page.locator('#definitions fieldset').nth(0);
    await segment.getByLabel('Name', {exact: true}).fill('Products · AND');
    await condition(segment, 0, 'requested_url', 'regex', '/products/(red|blue)$');
    await segment.getByRole('button', {name: 'Bedingung hinzufügen', exact: true}).click();
    await condition(segment, 1, 'status', 'equals', '200');
    await page.click('#add');
    segment = page.locator('#definitions fieldset').nth(1);
    await segment.getByLabel('Name', {exact: true}).fill('Named products');
    await condition(segment, 0, 'stored_title', 'contains', 'Product');
    await page.click('#add');
    segment = page.locator('#definitions fieldset').nth(2);
    await segment.getByLabel('Name', {exact: true}).fill('About or red · OR');
    await segment.locator(':scope > label select').selectOption('any');
    await condition(segment, 0, 'requested_url', 'equals', origin + '/about');
    await segment.getByRole('button', {name: 'Bedingung hinzufügen', exact: true}).click();
    await condition(segment, 1, 'requested_url', 'equals', origin + '/products/red');
    await page.click('#save');
    assert.match(await page.locator('#message').innerText(), /gespeichert/);
    const savedRules = await page.evaluate(() => JSON.parse(localStorage.getItem('open-seo-saved-segments-v1')));
    assert.deepEqual(savedRules.definitions.map(rule => rule.id), ['s1', 's2', 's3']);
    assert.equal(savedRules.definitions[0].conditions.length, 2);
    await page.reload();
    await page.click('#restore');
    assert.equal(await page.locator('#definitions fieldset').count(), 3);
    assert.equal(await page.locator('#definitions fieldset').nth(2).getByLabel('Name', {exact: true}).inputValue(), 'About or red · OR');
    await page.locator(`#job option[value="${start.id}"]`).waitFor({state: 'attached'});
    await page.selectOption('#job', start.id);
    checks.push('Actual builder creates ordered AND/OR/regex definitions; explicit save and reload restore all browser-persisted rules');

    const first = await preview(page), initialHash = first.value.context.typed_report_sha256;
    assert.equal(first.value.summary.total_pages, 6);
    assert.equal(first.value.summary.selected_pages, 6);
    assert.equal(first.value.summary.multiple_matches, 2);
    assert.equal(first.value.summary.unassigned_pages, 1);
    assert.equal(first.value.summary.unknown_pages, 2);
    assert.deepEqual(first.value.counts.map(count => [count.matched, count.unknown]), [[2, 0], [2, 2], [2, 0]]);
    assert.equal(first.value.reporting.captured_page_observations, 7);
    assert.equal(first.value.reporting.profile_excluded_page_observations, 1);
    assert.equal(first.value.context.sql_attested, false);
    const expectedAssignments = {
      '/index': [[], []], '/products/red': [[0, 1, 2], []], '/products/blue': [[0, 1], []],
      '/about': [[2], []], '/untitled': [[], [1]], '/missing': [[], [1]],
    };
    assert.deepEqual(Object.fromEntries(first.value.rows.map(row => [new URL(row.url).pathname, [row.matched_indices, row.unknown_indices]])), expectedAssignments);
    checks.push('Native saved report separates true/false/unknown, overlapping matches, AND/OR and noindex profile exclusions');

    const fullBytes = await download(page, '#export-json', '/segments.json'), full = JSON.parse(fullBytes);
    assert.equal(full.context.snapshot_token, first.value.context.snapshot_token);
    assert.deepEqual(full.projection.pages, first.value.rows);
    const csvBytes = await download(page, '#export-csv', '/segments.csv'), csv = parseCsv(csvBytes.toString('utf8'));
    assert.deepEqual(csv.shift(), ['row_type', 'page_ordinal', 'url', 'segment_indices_json', 'unknown_indices_json', 'data_json']);
    const csvContext = JSON.parse(csv.find(row => row[0] === 'context')[5]);
    assert.equal(csvContext.snapshot_token, full.context.snapshot_token);
    assert.deepEqual(csv.filter(row => row[0] === 'page').map(row => JSON.parse(row[5])), full.projection.pages);
    assert.deepEqual(csv.filter(row => row[0] === 'issue').map(row => JSON.parse(row[5])), full.projection.issues);
    fs.writeFileSync(artifactPath('segments.csv'), csvBytes);
    fs.writeFileSync(artifactPath('segments.json'), fullBytes);
    checks.push('Browser downloads full JSON and CSV; actual response SHA256, preview token, page rows and issue rows all agree');

    const selected = {};
    for (const [selection, expected] of [['segment:s1', 2], ['unassigned', 1], ['unknown', 2]]) {
      await page.selectOption('#selection', selection);
      assert.equal(await page.locator('#result').isHidden(), true, 'Selection change must invalidate old result and export context');
      const result = await preview(page);
      assert.equal(result.value.summary.selected_pages, expected);
      selected[selection] = result.value.rows.map(row => row.url);
    }
    assert.deepEqual(selected['segment:s1'].sort(), [origin + '/products/red', origin + '/products/blue'].sort());
    assert.deepEqual(selected.unassigned, [origin + '/index']);
    assert.deepEqual(selected.unknown.sort(), [origin + '/missing', origin + '/untitled'].sort());
    await page.selectOption('#view', 'issues');
    const issues = await preview(page);
    assert(issues.value.rows.length > 0, 'Missing-title/404 pages must have genuine native issues');
    assert(issues.value.rows.every(issue => selected.unknown.includes(issue.url)), 'Issues must follow the exact selected page URL');
    const issueExport = JSON.parse(await download(page, '#export-json', '/segments.json'));
    assert.deepEqual(issueExport.projection.issues, issues.value.rows);
    checks.push('Confirmed segment, unassigned and unknown selections retain exact native membership; issue view and full export agree');

    await page.selectOption('#selection', 'all');
    await page.selectOption('#view', 'pages');
    await page.selectOption('#reporting-mode', 'raw');
    assert.equal(await page.locator('#result').isHidden(), true);
    const raw = await preview(page);
    assert.equal(raw.value.summary.selected_pages, 7);
    assert.equal(raw.value.context.typed_report_sha256, initialHash);
    assert.notEqual(raw.value.context.snapshot_token, first.value.context.snapshot_token);
    const bound = {...first.request, expected_snapshot_token: first.value.context.snapshot_token, expected_generation: first.value.context.app_generation};
    for (const suffix of ['segments', 'segments.json', 'segments.csv']) {
      await refused(start.id, suffix, {...bound, reporting_mode: 'raw'}, 409);
      await refused(start.id, suffix, {...bound, expected_generation: first.value.context.app_generation + 1}, 409);
      const changedRules = structuredClone(bound); changedRules.rules.definitions[0].conditions[0].value = '/never-matches$';
      await refused(start.id, suffix, changedRules, 409);
    }
    await refused(start.id, 'segments.csv', first.request, 400);
    checks.push('Reported/raw UI revision invalidates old output; native preview/JSON/CSV reject crossed profile, stale generation and changed rules; unbound export is refused');

    segment = page.locator('#definitions fieldset').nth(0);
    await condition(segment, 0, 'requested_url', 'regex', '(?=unsupported-lookaround)');
    const invalidResponse = page.waitForResponse(response => response.url().endsWith('/segments') && response.request().method() === 'POST');
    await page.getByRole('button', {name: 'Auswerten', exact: true}).click();
    assert.equal((await invalidResponse).status(), 400);
    await until(() => page.locator('#result').isHidden(), 'invalid Rust regex hides previous result');
    await condition(segment, 0, 'requested_url', 'regex', '/products/(red|blue)$');
    await preview(page);
    checks.push('Rust-only regex validation surfaces a real native refusal and clears the prior preview before recovery');

    for (const width of [1440, 320]) {
      await page.setViewportSize({width, height: 900});
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, 'Populated saved-segments page must fit ' + width + 'px');
      await page.screenshot({path: artifactPath(`segments-${width}.png`), fullPage: true});
    }
    assert.equal(websiteHits, capturedHits, 'Saved segmentation must never refetch the website');
    const final = await json(`/api/crawls/${start.id}/segments`, {...raw.request, reporting_mode: 'raw'});
    assert.equal(final.context.typed_report_sha256, initialHash, 'All evaluations and exports preserve the captured native report');

    const batchUrls = Array.from({length: 105}, (_, i) => origin + '/products/batch-' + String(i).padStart(3, '0'));
    const batch = await json('/api/crawls', {seed_urls: batchUrls, crawl_links: false, discover_sitemaps: false, obey_robots: false, delay_ms: 0});
    await until(async () => { const job = await json('/api/crawls/' + batch.id); assert.notEqual(job.status, 'failed', job.error); return job.status === 'completed'; }, 'owned native pagination crawl');
    const batchCapturedHits = websiteHits;
    assert.equal(batchCapturedHits - capturedHits, 105);
    await page.reload();
    await page.click('#restore');
    await page.locator(`#job option[value="${batch.id}"]`).waitFor({state: 'attached'});
    await page.selectOption('#job', batch.id);
    await page.selectOption('#selection', 'segment:s2');
    const batchFirst = await preview(page);
    assert.equal(batchFirst.value.summary.selected_pages, 105);
    assert.equal(batchFirst.value.rows.length, 100);
    assert.equal(batchFirst.value.next.offset, 100);
    const continuationResponse = page.waitForResponse(response => response.url().endsWith('/segments') && response.request().method() === 'POST');
    await page.click('#next');
    const continuation = await continuationResponse;
    assert.equal(continuation.status(), 200, await continuation.text());
    const batchLast = await continuation.json();
    await renderedRows(page, batchLast);
    assert.equal(batchLast.rows.length, 5);
    assert.equal(batchLast.context.snapshot_token, batchFirst.value.context.snapshot_token);
    assert.equal(continuation.request().postDataJSON().cursor.offset, 100);
    await until(() => page.isDisabled('#next'), 'last native segment page');
    const allBatchUrls = [...batchFirst.value.rows, ...batchLast.rows].map(row => row.url).sort();
    assert.deepEqual(allBatchUrls, [...batchUrls].sort());
    assert.equal(new Set(allBatchUrls).size, 105);
    const batchCsv = await download(page, '#export-csv', '/segments.csv');
    assert.deepEqual(parseCsv(batchCsv.toString('utf8')).filter(row => row[0] === 'page').map(row => row[2]).sort(), [...batchUrls].sort());
    fs.writeFileSync(artifactPath('segments-pagination.csv'), batchCsv);
    assert.equal(websiteHits, batchCapturedHits, 'Segment continuation and full export must never refetch the batch documents');
    checks.push('105 real saved pages span 100+5 snapshot-bound browser pages without gaps/duplicates; verified full CSV exports all 105 from the last preview');
    assert.deepEqual(errors, []);
    fs.writeFileSync(artifactPath('segments-live.json'), JSON.stringify({result: 'PASS', crawl_id: start.id, pagination_crawl_id: batch.id, checks, captured_pages: batchCapturedHits, website_refetches: websiteHits - batchCapturedHits, typed_report_sha256: initialHash, errors, scope: 'Actual redesigned browser -> native Rust saved crawls -> deterministic predicates -> preview, snapshot-bound 100+5 pagination and integrity-verified full exports. Browser reload persistence covers definitions only; no cold native process restart, imported missing status or database attestation is claimed.'}, null, 2) + '\n');
    console.log(`${checks.length} native saved-segments workflows passed`);
  } catch (error) {
    await page.screenshot({path: artifactPath('segments-failure.png'), fullPage: true});
    const layout = await page.evaluate(() => ({viewport: innerWidth, document: document.documentElement.scrollWidth, overflow: [...document.querySelectorAll('body *')].filter(element => element.getBoundingClientRect().right > innerWidth + 1).slice(0, 25).map(element => ({tag: element.tagName, id: element.id, width: element.getBoundingClientRect().width, text: element.textContent.slice(0, 90)}))}));
    fs.writeFileSync(artifactPath('segments-failure.json'), JSON.stringify({error: error.stack, checks, layout}, null, 2) + '\n');
    throw error;
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; }).finally(async () => {
  website.closeAllConnections(); await new Promise(resolve => website.close(resolve));
});
