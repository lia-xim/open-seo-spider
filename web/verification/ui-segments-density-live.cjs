'use strict';
// Owns an ephemeral native app, a fresh store and genuinely captured documents.
// No injected API results, user browser profile or installed app is used.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const {spawn} = require('node:child_process');
const {createHash, randomUUID} = require('node:crypto');
const {chromium} = require('../../renderer/node_modules/playwright-core');

const root = path.resolve(__dirname, '../..');
assert(process.env.SPIDER_BINARY, 'Explicit SPIDER_BINARY is required; never build or install from this test');
const binary = fs.realpathSync(path.resolve(process.env.SPIDER_BINARY));
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const binarySha = sha(fs.readFileSync(binary));
if (process.env.SPIDER_BINARY_SHA256) assert.equal(binarySha, process.env.SPIDER_BINARY_SHA256);
const output = path.resolve(process.env.SPIDER_SEGMENTS_DENSITY_OUTPUT || path.join(
  process.env.SPIDER_TEST_ARTIFACT_DIR || path.join(root, 'artifacts'), `segments-density-${Date.now()}-${randomUUID().slice(0, 8)}`));
assert(!fs.existsSync(output), 'Require a new owned artifact directory');
fs.mkdirSync(output, {recursive: true});
process.env.SPIDER_TEST_ARTIFACT_DIR = output;
const artifactPath = require('./artifact-path.cjs');
const pins = [__filename, path.join(__dirname, 'artifact-path.cjs'), ...['saved-segments.js', 'saved-segments.css'].map(file => path.join(root, 'web', file))]
  .map(file => ({path: file, sha256: sha(fs.readFileSync(file))}));
const checks = [], errors = [], external = [], metrics = [], reads = [], exportsProof = [], served = [];
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const longTitle = 'Gespeicherter Titel 雪 – ' + 'vollständige unveränderte Wörter '.repeat(24) + '<img src=x onerror="window.segmentInjection=true"> SELECTABLE-END';
const titleFor = index => index < 10 ? `Owned ${index}: ${longTitle}` : 'Owned product ' + index;
const segmentName = 'Lange Segmentbezeichnung 雪 · ' + 'Gruppe-'.repeat(20) + ' ENDE';
const suffixes = Array.from({length: 105}, (_, index) => '/products/' + (index === 0 ? 'unbroken-address-'.repeat(14) : '') + String(index).padStart(3, '0'));
const hits = new Map();
let app, appClose, appLog = '', appOrigin, siteOrigin, browser, page, failure, shutdown, capturedHits, nativePages;
const website = http.createServer((request, response) => {
  hits.set(request.url, (hits.get(request.url) || 0) + 1);
  const index = suffixes.indexOf(request.url);
  const title = titleFor(index);
  response.writeHead(index === 104 || index < 0 ? 404 : 200, {'content-type': 'text/html; charset=utf-8'});
  const escape = value => value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
  response.end('<html lang="de"><head>' + (index === 103 ? '' : '<title>' + escape(title) + '</title>') + '</head><body><main>Owned density fixture.</main></body></html>');
});
function verifyPins() {
  assert.equal(sha(fs.readFileSync(binary)), binarySha);
  for (const pin of pins) assert.equal(sha(fs.readFileSync(pin.path)), pin.sha256, 'Changed owned input: ' + pin.path);
}
async function until(work, label) {
  for (let attempt = 0; attempt < 1200; attempt++) {
    if (app) assert.equal(app.exitCode, null, 'Owned app exited: ' + appLog);
    const value = await work(); if (value) return value;
    await sleep(25);
  }
  throw Error('Timeout: ' + label);
}
function deadline(promise, label) {
  let timer;
  return Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(Error('Timeout: ' + label)), 20000); })])
    .finally(() => clearTimeout(timer));
}
async function api(route, body) {
  const response = await fetch(appOrigin + route, {method: body === undefined ? 'GET' : 'POST',
    headers: body === undefined ? {} : {'content-type': 'application/json'}, body: body === undefined ? undefined : JSON.stringify(body), redirect: 'error'});
  const bytes = Buffer.from(await response.arrayBuffer());
  assert.equal(response.status, 200, bytes.toString());
  return JSON.parse(bytes);
}
function expectRows(value) {
  const columns = value.view === 'pages'
    ? ['page_ordinal', 'url', 'document_url', 'status', 'depth', 'stored_title', 'matched_indices', 'unknown_indices']
    : ['code', 'severity', 'url', 'message', 'related_urls'];
  return value.rows.map(row => columns.map(key => {
    let cell = row[key];
    if (key === 'matched_indices' || key === 'unknown_indices') cell = cell.map(index => value.counts[index].name).join(', ');
    return cell === null ? 'Unbekannt' : typeof cell === 'object' ? JSON.stringify(cell) : String(cell ?? '');
  }));
}
async function exactDOM(value) {
  const expected = expectRows(value);
  await page.waitForFunction(rows => !document.getElementById('result').hidden && JSON.stringify(
    [...document.querySelectorAll('#rows tr')].map(row => [...row.cells].map(cell => cell.textContent))) === JSON.stringify(rows), expected);
  assert.deepEqual(await page.locator('#rows tr').evaluateAll(rows => rows.map(row => [...row.cells].map(cell => cell.textContent))), expected);
  assert.deepEqual(await page.locator('#counts tr').evaluateAll(rows => rows.map(row => [...row.cells].map(cell => cell.textContent))),
    value.counts.map(count => [count.name, count.matched, count.unknown, count.selected_matched, count.selected_unknown].map(String)));
  assert.equal(await page.evaluate(() => window.segmentInjection), undefined, 'Saved title markup remains inert text');
}
function captureRead(id, suffix = '') {
  return page.waitForResponse(response => response.url() === `${appOrigin}/api/crawls/${id}/segments${suffix}` && response.request().method() === 'POST')
    .then(async response => {
      // Pure saved-data reads always use an independent exact native POST oracle.
      // Mutation requests are never replayed and CDP body storage is not used.
      const request = response.request(), body = request.postDataBuffer(); assert(body);
      const oracle = await fetch(response.url(), {method: 'POST', headers: {'content-type': 'application/json'}, body, redirect: 'error', cache: 'no-store'});
      const bytes = Buffer.from(await oracle.arrayBuffer()), headers = await response.allHeaders();
      assert.equal(response.status(), 200, bytes.toString()); assert.equal(oracle.status, 200);
      for (const name of ['content-type', 'content-length', 'x-content-sha256', 'cache-control']) assert.equal(headers[name], oracle.headers.get(name), name);
      assert.equal(Number(headers['content-length']), bytes.length); assert.equal(headers['x-content-sha256'], sha(bytes)); assert.equal(headers['cache-control'], 'no-store');
      reads.push({url: response.url(), request_sha256: sha(body), bytes: bytes.length, sha256: sha(bytes), body_source: 'unconditional_exact_read_only_native_post', browser_content_binding: 'product_SHA_validation_and_exact_DOM_or_actual_download'});
      return {bytes, value: suffix === '.csv' ? null : JSON.parse(bytes)};
    });
}
async function preview(id, next = false) {
  const pending = captureRead(id);
  await page.click(next ? '#next' : '#segments-form button[type=submit]');
  const result = await pending; await exactDOM(result.value); return result.value;
}
async function download(id, format) {
  const pending = captureRead(id, '.' + format), attachment = page.waitForEvent('download');
  await page.click('#export-' + format);
  const expected = await pending, item = await attachment;
  assert.equal(await item.failure(), null);
  const bytes = fs.readFileSync(await item.path()); assert.deepEqual(bytes, expected.bytes);
  fs.writeFileSync(artifactPath('complete.' + format), bytes);
  exportsProof.push({format, bytes: bytes.length, sha256: sha(bytes)}); return bytes;
}
function parseCSV(text) {
  const rows = []; let row = [], field = '', quoted = false;
  for (let index = 0; index < text.length; index++) {
    const char = text[index];
    if (char === '"' && quoted && text[index + 1] === '"') { field += '"'; index++; }
    else if (char === '"') quoted = !quoted;
    else if (char === ',' && !quoted) { row.push(field); field = ''; }
    else if (char === '\r' && !quoted) { assert.equal(text[++index], '\n'); row.push(field); rows.push(row); row = []; field = ''; }
    else field += char;
  }
  assert(!quoted); assert.equal(field, ''); assert.deepEqual(row, []);
  assert.deepEqual(rows.shift(), ['row_type', 'page_ordinal', 'url', 'segment_indices_json', 'unknown_indices_json', 'data_json']);
  assert(rows.every(row => row.length === 6)); return rows;
}
function validateCSV(bytes, full) {
  const rows = parseCSV(bytes.toString('utf8')), projection = full.projection;
  assert.deepEqual(rows.map(row => row[0]), ['context', 'summary', ...(projection.reporting ? ['reporting'] : []),
    ...projection.pages.map(() => 'page'), ...projection.issues.map(() => 'issue')]);
  const expectedSummary = Object.fromEntries(Object.entries(projection).filter(([key]) => !['pages', 'issues', 'reporting'].includes(key)));
  const expected = {context: full.context, summary: expectedSummary, reporting: projection.reporting};
  let pageIndex = 0, issueIndex = 0;
  for (const row of rows) {
    const kind = row[0], saved = kind === 'page' ? projection.pages[pageIndex++] : kind === 'issue' ? projection.issues[issueIndex++] : expected[kind];
    assert.deepEqual(JSON.parse(row[5]), saved);
    assert.deepEqual(row.slice(0, 5), kind === 'page' ? ['page', String(saved.page_ordinal), saved.url, JSON.stringify(saved.matched_indices), JSON.stringify(saved.unknown_indices)]
      : kind === 'issue' ? ['issue', '', saved.url, '', ''] : [kind, '', '', '', '']);
  }
}
async function compactMetrics(label) {
  const result = await page.evaluate(() => ({viewport: innerWidth, document: document.documentElement.scrollWidth,
    rows: [...document.querySelectorAll('#rows tr')].map(row => row.getBoundingClientRect().height),
    counts: [...document.querySelectorAll('#counts tr')].map(row => row.getBoundingClientRect().height),
    previews: document.querySelectorAll('#rows details').length,
    tableWidth: document.querySelector('.segment-results-table').getBoundingClientRect().width,
    scrollerWidth: document.querySelector('.segment-results-table').parentElement.clientWidth}));
  assert.equal(result.viewport, 320); assert(result.document <= 320, 'No document overflow');
  assert(result.rows.length > 0 && result.previews > 0); assert(result.tableWidth > result.scrollerWidth, 'Only the labelled table scroller overflows horizontally');
  assert(Math.max(...result.rows) <= 104, 'Every closed mobile result row must remain bounded to104px');
  assert(Math.max(...result.counts) <= 104, 'Long segment names must not inflate count rows');
  metrics.push({label, ...result}); return result;
}

(async () => {
  try {
    await new Promise((resolve, reject) => { website.once('error', reject); website.listen(0, '127.0.0.1', resolve); });
    siteOrigin = 'http://127.0.0.1:' + website.address().port;
    const env = {...process.env, OPEN_SEO_COORDINATED_EXECUTION_WINDOW: 'confirmed'};
    delete env.OPEN_SEO_PAGESPEED_API_KEY;
    app = spawn(binary, ['serve', '--listen', '127.0.0.1:0', '--data-dir', path.join(output, 'store'), '--shutdown-on-stdin-eof'], {cwd: root, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'], env});
    appClose = new Promise(resolve => app.once('close', (code, signal) => resolve({code, signal})));
    let spawnError; app.once('error', error => { spawnError = error; });
    for (const stream of [app.stdout, app.stderr]) stream.on('data', bytes => { appLog += bytes; });
    appOrigin = await until(() => { if (spawnError) throw spawnError; return appLog.match(/^Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?$/m)?.[1]; }, 'owned native bind');
    assert(!['8787', '8790'].includes(new URL(appOrigin).port));
    for (const name of ['saved-segments.js', 'saved-segments.css', 'saved-segments.html', 'styles.css']) {
      const route = name === 'saved-segments.html' ? '/saved-segments' : '/' + name;
      const response = await fetch(appOrigin + route); assert.equal(response.status, 200);
      const bytes = Buffer.from(await response.arrayBuffer()); assert.equal(sha(bytes), sha(fs.readFileSync(path.join(root, 'web', name))), 'Build must embed current product asset: ' + name);
      served.push({path: name, sha256: sha(bytes)});
    }
    const started = await api('/api/crawls', {seed_urls: suffixes.map(suffix => siteOrigin + suffix), crawl_links: false, discover_sitemaps: false, obey_robots: false, delay_ms: 0});
    await until(async () => { const job = await api('/api/crawls/' + started.id); assert.notEqual(job.status, 'failed', job.error); return job.status === 'completed'; }, '105 genuine native pages');
    const report = await api(`/api/crawls/${started.id}/project.json`);
    assert.equal(report.pages.length, 105); assert.deepEqual(new Set(report.pages.map(row => row.url)), new Set(suffixes.map(suffix => siteOrigin + suffix)));
    nativePages = report.pages.length;
    assert.equal(report.pages.find(row => row.url === siteOrigin + suffixes[0]).titles[0], titleFor(0));
    assert.equal(report.pages.find(row => row.url === siteOrigin + suffixes[103]).titles.length, 0);
    assert.equal(report.pages.find(row => row.url === siteOrigin + suffixes[104]).status, 404);
    capturedHits = [...hits.values()].reduce((sum, count) => sum + count, 0); assert.equal(capturedHits, 105); assert([...hits.values()].every(count => count === 1));
    const executablePath = process.env.OPEN_SEO_BROWSER_PATH || process.env.SPIDER_CHROME;
    assert(executablePath, 'Explicit owned browser executable is required');
    browser = await chromium.launch({headless: true, executablePath});
    const context = await browser.newContext({viewport: {width: 320, height: 900}, hasTouch: true, acceptDownloads: true});
    page = await context.newPage(); page.on('pageerror', error => errors.push(error.message));
    await page.route('**/*', route => { const url = route.request().url(); if (url.startsWith(appOrigin + '/') || url.startsWith('blob:') || url.startsWith('data:')) return route.continue(); external.push(url); return route.abort(); });
    await page.goto(appOrigin + '/saved-segments'); await page.locator(`#job option[value="${started.id}"]`).waitFor({state: 'attached'}); await page.selectOption('#job', started.id);
    await page.click('#add'); const fieldset = page.locator('#definitions fieldset');
    await fieldset.getByLabel('Name', {exact: true}).fill(segmentName);
    await fieldset.locator('.condition select').nth(0).selectOption('requested_url'); await fieldset.locator('.condition select').nth(1).selectOption('contains');
    await fieldset.getByLabel('Wert', {exact: true}).fill('/products/');
    const first = await preview(started.id); assert.equal(first.rows.length, 100); assert.equal(first.total, 105); assert.equal(first.counts[0].matched, 105);
    assert(await page.locator('#segment-preview-help').isVisible()); assert.match(await page.locator('#segment-preview-help').textContent(), /Eingabe\/Leertaste/);
    const closed = await compactMetrics('first100_closed');
    const longIndex = first.rows.findIndex(row => row.stored_title?.endsWith('SELECTABLE-END')); assert(longIndex >= 0, 'At least five of ten long native titles belong to the first100 rows');
    const fullTitle = titleFor(suffixes.indexOf(new URL(first.rows[longIndex].url).pathname));
    const titleCell = page.locator('#rows tr').nth(longIndex).locator('td').nth(5), details = titleCell.locator('details'), summary = details.locator('summary'), value = summary.locator('span');
    assert.equal(await titleCell.textContent(), fullTitle); assert.equal(await titleCell.locator('.segment-cell-text').count(), 1);
    await page.locator('#rows tr').nth(longIndex).locator('td').nth(2).locator('summary').focus(); await page.keyboard.press('Tab');
    assert.equal(await summary.evaluate(element => document.activeElement === element), true, 'Native Tab order reaches the disclosure');
    await page.keyboard.press('Enter'); await page.waitForFunction(index => document.querySelectorAll('#rows tr')[index].cells[5].querySelector('details').open, longIndex);
    assert.equal(await titleCell.textContent(), fullTitle); assert.equal(await value.evaluate(element => element.scrollHeight <= element.clientHeight + 1), true, 'Expanded full value must have no internal vertical clipping');
    assert.notEqual(await value.evaluate(element => getComputedStyle(element).userSelect), 'none');
    assert((await page.locator('#rows tr').nth(longIndex).boundingBox()).height > closed.rows[longIndex]);
    await value.evaluate(element => element.scrollIntoView({block: 'start', inline: 'nearest'}));
    const firstLine = await value.evaluate(element => { const range = document.createRange(); range.selectNodeContents(element); const rect = range.getClientRects()[0]; return {x: rect.x, y: rect.y, width: rect.width, height: rect.height}; });
    assert(firstLine.x >= 0 && firstLine.x + firstLine.width <= 320 && firstLine.y >= 0 && firstLine.y + firstLine.height <= 900);
    await page.mouse.move(firstLine.x + 1, firstLine.y + firstLine.height / 2); await page.mouse.down();
    await page.mouse.move(firstLine.x + firstLine.width - 1, firstLine.y + firstLine.height / 2, {steps: 8}); await page.mouse.up();
    const selectedFragment = await page.evaluate(() => window.getSelection().toString());
    assert(selectedFragment.length > 2 && fullTitle.includes(selectedFragment), 'Real pointer drag selects original expanded text');
    assert.equal(await details.getAttribute('open'), '', 'Selecting text must not collapse its disclosure');
    await value.evaluate(element => { const range = document.createRange(); range.selectNodeContents(element); const selection = window.getSelection(); selection.removeAllRanges(); selection.addRange(range); });
    assert.equal(await page.evaluate(() => window.getSelection().toString()), fullTitle, 'Every original character remains selectable once');
    await summary.evaluate(element => element.scrollIntoView({block: 'start', inline: 'center'}));
    const marker = await summary.evaluate(element => {
      const rect = element.getBoundingClientRect(), text = element.querySelector('span'), style = getComputedStyle(element);
      const lineHeight = parseFloat(style.lineHeight), fontSize = parseFloat(style.fontSize), paddingTop = parseFloat(style.paddingTop);
      const points = [];
      for (let y = Math.max(0, Math.floor(rect.y + paddingTop)); y < Math.min(innerHeight, Math.ceil(rect.y + paddingTop + lineHeight)); y++) {
        for (let x = Math.max(0, Math.floor(rect.x - Math.max(24, fontSize * 2))); x < Math.min(innerWidth, Math.ceil(rect.x)); x++) {
          const target = document.elementFromPoint(x, y);
          if (target === element && !text.contains(target)) points.push({x, y});
        }
      }
      const point = points[Math.floor(points.length / 2)];
      return {point, candidates: points.length, summary: {x: rect.x, y: rect.y, width: rect.width, height: rect.height}, line_height: lineHeight,
        owner_is_summary: !!point && document.elementFromPoint(point.x, point.y) === element,
        outside_text: !!point && !text.contains(document.elementFromPoint(point.x, point.y))};
    });
    assert(marker.candidates > 0 && marker.owner_is_summary && marker.outside_text, 'Native first-line marker must remain a reachable summary target outside the text span');
    metrics.push({stage: 'native_marker_hit_test', ...marker});
    const {x: markerX, y: markerY} = marker.point;
    await page.touchscreen.tap(markerX, markerY); await page.waitForFunction(index => !document.querySelectorAll('#rows tr')[index].cells[5].querySelector('details').open, longIndex);
    await page.touchscreen.tap(markerX, markerY); await page.waitForFunction(index => document.querySelectorAll('#rows tr')[index].cells[5].querySelector('details').open, longIndex);
    await page.evaluate(() => window.getSelection().removeAllRanges());
    await summary.focus(); await page.keyboard.press('Space'); await page.waitForFunction(index => !document.querySelectorAll('#rows tr')[index].cells[5].querySelector('details').open, longIndex);
    await exactDOM(first); await compactMetrics('keyboard_recollapsed');
    await summary.tap(); await page.waitForFunction(index => document.querySelectorAll('#rows tr')[index].cells[5].querySelector('details').open, longIndex);
    assert.equal(await titleCell.textContent(), fullTitle); await page.screenshot({path: artifactPath('segments-density-expanded-320.png')});
    const expandedJSON = await download(started.id, 'json');
    await summary.tap(); await page.waitForFunction(index => !document.querySelectorAll('#rows tr')[index].cells[5].querySelector('details').open, longIndex);
    await exactDOM(first); await compactMetrics('touch_recollapsed');
    await summary.scrollIntoViewIfNeeded(); await page.screenshot({path: artifactPath('segments-density-collapsed-320.png')});
    const last = await preview(started.id, true); assert.equal(last.rows.length, 5); assert.equal(last.next, null); await compactMetrics('last5_closed');
    const json = await download(started.id, 'json'), csv = await download(started.id, 'csv'), full = JSON.parse(json);
    assert.deepEqual(json, expandedJSON, 'Disclosure and current table page never change complete export bytes');
    assert.deepEqual(full.projection.pages, [...first.rows, ...last.rows]); assert.equal(full.projection.pages.length, 105);
    assert.deepEqual(full.projection.pages.map(row => row.url), report.pages.map(row => row.url));
    assert.equal(full.projection.pages.find(row => row.url === siteOrigin + suffixes[0]).stored_title, titleFor(0));
    assert.equal(full.projection.rules.definitions[0].name, segmentName); validateCSV(csv, full);
    await page.locator('#export-csv').scrollIntoViewIfNeeded(); const exportBox = await page.locator('#export-csv').boundingBox(); assert(exportBox.x >= 0 && exportBox.x + exportBox.width <= 320);
    checks.push('105 genuine saved pages retain exact100+5 row/cell order, unknown title and native404; closed320px result/count rows stay bounded, with horizontal table scrolling and no document overflow');
    checks.push('Native disclosure supports Enter open, Space close and touch open/close; exact original Unicode/markup text appears once and is completely selectable, with no executable stored markup');
    await page.selectOption('#view', 'issues'); const issues = await preview(started.id);
    assert(issues.rows.length > 0, 'Native capture must supply real issue rows');
    assert.deepEqual(issues.rows, report.issues.slice(0, 100)); await compactMetrics('issues_closed');
    const issueDetails = page.locator('#rows details').first(), issueSummary = issueDetails.locator('summary'), issueText = await issueSummary.textContent();
    await issueSummary.tap(); assert.equal(await issueDetails.getAttribute('open'), ''); assert.equal(await issueSummary.textContent(), issueText);
    await issueSummary.tap(); assert.equal(await issueDetails.getAttribute('open'), null); await exactDOM(issues); await compactMetrics('issues_recollapsed');
    await page.selectOption('#view', 'pages');
    await page.setViewportSize({width: 1440, height: 1000}); await preview(started.id);
    assert.equal(await page.locator('#segment-preview-help').isVisible(), false);
    const desktopTitle = page.locator('#rows tr').nth(longIndex).locator('td').nth(5).locator('span');
    assert.equal(await desktopTitle.isVisible(), true);
    assert.equal(await desktopTitle.evaluate(element => getComputedStyle(element).webkitLineClamp), 'none');
    const desktopLayout = await desktopTitle.evaluate(element => { const range = document.createRange(); range.selectNodeContents(element); const rects = [...range.getClientRects()], cell = element.closest('td').getBoundingClientRect(); return {lines: rects.length, all_text_inside_cell: rects.every(rect => rect.top >= cell.top - 1 && rect.bottom <= cell.bottom + 1), height: cell.height}; });
    assert(desktopLayout.lines > 3 && desktopLayout.all_text_inside_cell, 'Every desktop text line remains laid out without mobile clipping');
    metrics.push({label: 'desktop_unclamped', ...desktopLayout});
    assert.equal(await desktopTitle.locator('..').getAttribute('tabindex'), '-1'); await exactDOM(first);
    await desktopTitle.scrollIntoViewIfNeeded(); await page.screenshot({path: artifactPath('segments-density-1440.png')});
    checks.push('Desktop values remain unclamped; complete actual browser JSON/CSV downloads preserve every native page, issue, rule, count and metadata cell despite collapsed mobile previews and paging');
    assert.equal([...hits.values()].reduce((sum, count) => sum + count, 0), capturedHits, 'Saved analysis, disclosure and exports never refetch the website');
    assert.deepEqual(errors, []); assert.deepEqual(external, []); verifyPins();
  } catch (error) { failure = error; }
  finally {
    if (page && failure) { try { await page.screenshot({path: artifactPath('segments-density-failure.png')}); } catch {} }
    if (browser) await browser.close();
    if (app && !app.stdin.destroyed) app.stdin.end();
    if (appClose) { try { shutdown = await deadline(appClose, 'natural native EOF close'); assert.deepEqual(shutdown, {code: 0, signal: null}); } catch (error) { failure ||= error; if (app?.exitCode === null) app.kill(); } }
    if (website.listening) { website.closeAllConnections(); await new Promise(resolve => website.close(resolve)); }
    if (capturedHits !== undefined) { try { assert.equal([...hits.values()].reduce((sum, count) => sum + count, 0), capturedHits, 'No website reads through final producer shutdown'); } catch (error) { failure ||= error; } }
    try { verifyPins(); } catch (error) { failure ||= error; }
    fs.writeFileSync(artifactPath('native.log'), appLog);
    fs.writeFileSync(artifactPath('segments-density-live.json'), JSON.stringify({result: failure ? 'FAIL' : 'PASS', checks, errors, failure: failure?.stack,
      binary_sha256: binarySha, pins, served_assets: served, native: {pid: app?.pid, origin: appOrigin, shutdown}, native_pages: nativePages ?? null,
      website_capture_requests: capturedHits, website_requests_after_capture: capturedHits === undefined ? null : [...hits.values()].reduce((sum, count) => sum + count, 0) - capturedHits,
      metrics, read_oracles: reads, exports: exportsProof, external_requests: external,
      scope: 'Owned normal native executable and fresh actual105-page crawl. Responsive disclosure and exact DOM/export semantics only; no installed app, crash recovery, broad browser matrix or manufacturer parity claim.'}, null, 2) + '\n');
  }
  if (failure) throw failure;
  for (const check of checks) console.log('PASS ' + check);
  console.log('Evidence: ' + output);
})().catch(error => { console.error(error.stack); process.exitCode = 1; });
