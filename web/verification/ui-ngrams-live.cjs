'use strict';
// Real browser -> native saved crawl -> offline corpus -> verified download.
const assert = require('node:assert/strict');
const http = require('node:http');
const fs = require('node:fs');
const crypto = require('node:crypto');
const {chromium} = require('../../renderer/node_modules/playwright-core');
const artifactPath = require('./artifact-path.cjs');
const app = process.env.SPIDER_APP_URL;
assert(app && new URL(app).hostname === '127.0.0.1' && !['8787', '8790'].includes(new URL(app).port), 'Use an isolated test app');
const checks = [], errors = [], hits = new Map(), readEvidence = [], downloadEvidence = [], previewBytes = new WeakMap();
const unique = Array.from({length: 105}, (_, i) => `word${i}`).join(' ');
const bodies = {
  '/a': `Alpha beta Alpha beta and gamma Alpha ${unique} ${'repeat '.repeat(20)}`,
  '/b': 'alpha beta and Gamma', '/empty': '', '/partial': 'ErrorPhrase must remain unknown',
  '/metadata': 'Body text capture is disabled for this fixture',
};
const website = http.createServer((req, res) => {
  hits.set(req.url, (hits.get(req.url) || 0) + 1);
  if (req.url === '/plain') { res.setHeader('content-type', 'text/plain'); return res.end('Unparsed plain text'); }
  if (!Object.hasOwn(bodies, req.url)) { res.statusCode = 404; return res.end('Unknown fixture path'); }
  if (req.url === '/partial') res.statusCode = 404;
  res.setHeader('content-type', 'text/html; charset=utf-8');
  res.end(`<html lang="en"><head><title>Saved ${req.url.slice(1)} Title</title></head><body><nav>ExcludedNavigation</nav><main>${bodies[req.url]}</main></body></html>`);
});
const count = () => Object.fromEntries(hits);
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const check = name => { checks.push(name); console.log('PASS ' + name); };
async function until(work, label) {
  for (let i = 0; i < 1000; i++) { const result = await work(); if (result) return result; await new Promise(r => setTimeout(r, 50)); }
  throw Error('Timeout: ' + label);
}
async function api(path, body) {
  const response = await fetch(app + path, {method: body === undefined ? 'GET' : 'POST',
    headers: body === undefined ? {} : {'content-type': 'application/json'}, body: body === undefined ? undefined : JSON.stringify(body)});
  const bytes = Buffer.from(await response.arrayBuffer());
  assert.equal(response.status, 200, bytes.toString('utf8')); return JSON.parse(bytes);
}
async function crawl(urls, extra = {}) {
  const started = await api('/api/crawls', {seed_urls: urls, crawl_links: false, discover_sitemaps: false,
    obey_robots: false, delay_ms: 0, content_capture: {sources: ['raw_http_html'], include_selectors: ['main']}, ...extra});
  await until(async () => { const job = await api('/api/crawls/' + started.id); assert.notEqual(job.status, 'failed', job.error); return ['completed', 'limited'].includes(job.status); }, 'native saved crawl');
  return started.id;
}
function csvRows(text) {
  const rows = []; let row = [], cell = '', quoted = false;
  for (let i = 0; i < text.length; i++) {
    const char = text[i];
    if (char === '"') { if (quoted && text[i + 1] === '"') { cell += '"'; i++; } else quoted = !quoted; }
    else if (char === ',' && !quoted) { row.push(cell); cell = ''; }
    else if (char === '\n' && !quoted) { row.push(cell.replace(/\r$/, '')); rows.push(row); row = []; cell = ''; }
    else cell += char;
  }
  assert.equal(quoted, false); if (cell || row.length) { row.push(cell); rows.push(row); } return rows;
}
async function nativeRead(response, expectedURL) {
  const request = response.request(), url = new URL(response.url()), body = request.postDataBuffer();
  assert.equal(response.url(), expectedURL); assert.equal(url.origin, new URL(app).origin);
  assert.match(url.pathname, /^\/api\/crawls\/[a-f0-9-]+\/ngrams\/(preview|csv)$/i);
  assert.equal(url.search, ''); assert.equal(request.method(), 'POST'); assert(body, 'Exact browser POST bytes required');
  // These two routes only read saved data. Always use the exact native POST as
  // oracle; CDP body availability is not a retry/fallback or a product assertion.
  const oracle = await fetch(response.url(), {method: 'POST', headers: {'content-type': request.headers()['content-type']}, body, cache: 'no-store', redirect: 'error'});
  const bytes = Buffer.from(await oracle.arrayBuffer()), headers = await response.allHeaders(), oracleHeaders = Object.fromEntries(oracle.headers);
  assert.equal(response.status(), oracle.status, bytes.toString('utf8'));
  for (const name of ['content-type', 'content-length', 'x-content-sha256', 'cache-control', 'x-ngram-snapshot']) assert.equal(headers[name] ?? null, oracleHeaders[name] ?? null, name);
  if (oracle.status === 200) { assert.equal(headers['content-type'], url.pathname.endsWith('/csv') ? 'text/csv; charset=utf-8' : 'application/json; charset=utf-8'); assert.equal(Number(headers['content-length']), bytes.length); assert.equal(headers['x-content-sha256'], sha(bytes)); assert.equal(headers['cache-control'], 'no-store'); }
  readEvidence.push({url: response.url(), method: 'POST', request_sha256: sha(body), status: oracle.status, bytes: bytes.length, sha256: sha(bytes), body_source: 'unconditional_exact_read_only_native_post'});
  return {bytes, headers};
}
async function preview(page, trigger = '#preview') {
  const expectedURL = `${app}/api/crawls/${await page.inputValue('#crawl')}/ngrams/preview`;
  const pending = page.waitForResponse(r => r.url() === expectedURL && r.request().method() === 'POST');
  await page.click(trigger); const response = await pending;
  const {bytes, headers} = await nativeRead(response, expectedURL); assert.equal(response.status(), 200, bytes.toString('utf8'));
  assert.equal(Number(headers['content-length']), bytes.length); assert.equal(headers['x-content-sha256'], sha(bytes));
  const value = JSON.parse(bytes); assert.equal(value.website_requests, 0); assert.equal(value.whole_website_coverage_attested, false);
  previewBytes.set(value, bytes);
  const visible = {
    summary: `${value.after + 1 > value.total_rows ? 0 : value.after + 1}–${value.after + value.rows.length} von ${value.total_rows} Zeilen in der Auswahl · ${value.total_tokens} Tokens · ${value.total_text_bytes} analysierte Textbytes.`,
    headings: value.rows.map(row => row.tokens ? row.tokens.join(' ') : row.source.requested_url),
    rows: value.rows,
  };
  await page.waitForFunction(expected => {
    if (document.getElementById('csv').disabled || document.getElementById('results').hidden || document.getElementById('summary').textContent !== expected.summary) return false;
    const articles = [...document.querySelectorAll('#rows > article')];
    if (articles.length !== expected.rows.length || JSON.stringify(articles.map(article => article.querySelector('h3')?.textContent)) !== JSON.stringify(expected.headings)) return false;
    try { return JSON.stringify(articles.map(article => JSON.parse(article.querySelector('details pre').textContent))) === JSON.stringify(expected.rows); }
    catch { return false; }
  }, visible);
  assert.equal(await page.locator('#rows > article').count(), value.rows.length);
  assert.deepEqual(await page.locator('#rows > article > h3').allTextContents(), visible.headings);
  assert.match(await page.locator('#status').innerText(), /0 Website-Anfragen/);
  return value;
}
async function download(page, button) {
  const pending = page.waitForEvent('download'); await page.click(button); const item = await pending;
  assert.equal(await item.failure(), null); const bytes = fs.readFileSync(await item.path());
  const filename = `ngrams-download-${downloadEvidence.length + 1}.${button === '#json' ? 'json' : 'csv'}`;
  fs.writeFileSync(artifactPath(filename), bytes); downloadEvidence.push({filename, bytes: bytes.length, sha256: sha(bytes)}); return bytes;
}
async function csv(page, expected) {
  const expectedURL = `${app}/api/crawls/${await page.inputValue('#crawl')}/ngrams/csv`;
  const pending = page.waitForResponse(r => r.url() === expectedURL && r.request().method() === 'POST');
  const bytes = await download(page, '#csv'), response = await pending, oracle = await nativeRead(response, expectedURL), headers = oracle.headers;
  assert.deepEqual(bytes, oracle.bytes, 'Actual browser CSV file must match the exact native read');
  assert.equal(response.status(), 200); assert.equal(headers['x-ngram-snapshot'], expected.snapshot.snapshot_token);
  assert.equal(Number(headers['content-length']), bytes.length); assert.equal(headers['x-content-sha256'], sha(bytes));
  assert.match(await page.locator('#status').innerText(), /SHA-256 bestätigt/); return csvRows(bytes.toString('utf8'));
}
async function selectCrawl(page, id) {
  await page.click('#reload'); await until(() => page.isEnabled('#crawl'), 'crawl list'); await page.selectOption('#crawl', id);
}
(async () => {
  await new Promise(r => website.listen(0, '127.0.0.1', r));
  const origin = `http://127.0.0.1:${website.address().port}`;
  const id = await crawl(['a', 'b', 'empty', 'partial', 'plain'].map(p => origin + '/' + p));
  const metadataId = await crawl([origin + '/metadata'], {content_capture: null});
  const limitedId = await crawl([origin + '/a', origin + '/b'], {max_pages: 1, concurrency: 1});
  assert.equal((await api('/api/crawls/' + limitedId)).status, 'limited');
  const saved = await api('/api/crawls/' + id + '/project.json');
  const expectedCaptured = count();
  const executablePath = process.env.SPIDER_CHROME || process.env.OPEN_SEO_BROWSER_PATH || await (await import('../../renderer/server.mjs')).findBrowserExecutable();
  const browser = await chromium.launch({headless: true, executablePath});
  const page = await browser.newPage({viewport: {width: 1440, height: 1000}, acceptDownloads: true});
  page.on('pageerror', e => errors.push(e.message));
  try {
    await page.goto(app + '/saved-ngrams'); await until(() => page.isEnabled('#crawl'), 'initial crawl list');
    await page.selectOption('#crawl', id); await page.selectOption('#n', '1');
    const first = await preview(page);
    assert.equal(first.saved_documents, 5); assert.equal(first.rows.length, 100); assert.equal(first.has_more, true);
    assert.equal(first.source_states.complete, 2); assert.equal(first.source_states.complete_empty, 1);
    assert.equal(first.source_states.incomplete_excluded, 1); assert.equal(first.source_states.missing_or_unavailable, 1);
    assert.equal(first.all_supplied_sources_complete, false); assert.match(await page.locator('#coverage').innerText(), /unvollständige Quellen ausgeschlossen/);
    assert(first.total_rows > 100); assert.equal(first.rows.find(r => r.tokens[0] === 'alpha').occurrences, 4);
    assert(!first.rows.some(r => r.tokens[0] === 'excludednavigation' || r.tokens[0] === 'errorphrase'));
    const firstJSON = await download(page, '#json'); assert.deepEqual(firstJSON, previewBytes.get(first)); assert.deepEqual(JSON.parse(firstJSON), first);
    const fullCSV = await csv(page, first); assert.equal(fullCSV.length, first.total_rows + 1);
    const tokensColumn = fullCSV[0].indexOf('tokens_json'); assert(tokensColumn >= 0);
    assert(fullCSV.slice(1).some(row => JSON.parse(row[tokensColumn]).includes('word104')));
    assert(fullCSV.slice(1).some(row => JSON.parse(row[tokensColumn]).includes('word0')));
    const second = await preview(page, '#next'); assert.equal(second.after, 100); assert.equal(second.has_more, false);
    assert.equal(second.snapshot.snapshot_token, first.snapshot.snapshot_token);
    const keys = [...first.rows, ...second.rows].map(r => JSON.stringify(r.tokens));
    assert.equal(keys.length, first.total_rows); assert.equal(new Set(keys).size, first.total_rows);
    const column = name => { const index = fullCSV[0].indexOf(name); assert(index >= 0, name); return index; };
    assert.deepEqual(fullCSV.slice(1).map(row => [JSON.parse(row[column('tokens_json')]), Number(row[column('occurrences')]), Number(row[column('document_frequency')])]),
      [...first.rows, ...second.rows].map(row => [row.tokens, row.occurrences, row.document_frequency]));
    assert(fullCSV.slice(1).every(row => row[column('snapshot_token')] === first.snapshot.snapshot_token && row[column('website_requests')] === '0'));
    assert.equal((await csv(page, second)).length, first.total_rows + 1);
    check('native saved sources retain complete/empty/incomplete/missing distinctions; bound 100-row pages have no gaps/duplicates and JSON/full CSV downloads match length, SHA and snapshot');

    await page.fill('#query', 'alpha beta'); await page.selectOption('#n', '2');
    assert.equal(await page.isDisabled('#csv'), true); assert.equal(await page.isVisible('#results'), false);
    const totals = await preview(page); assert.equal(totals.rows.length, 1); assert.deepEqual(totals.rows[0].tokens, ['alpha', 'beta']);
    assert.equal(totals.rows[0].occurrences, 3); assert.equal(totals.rows[0].document_frequency, 2);
    const occurrences = await preview(page, '#rows article button'); assert.equal(await page.inputValue('#view'), 'occurrences');
    assert.equal(occurrences.rows.length, 2);
    const sourceA = occurrences.rows.find(r => r.source.requested_url === origin + '/a');
    const sourceB = occurrences.rows.find(r => r.source.requested_url === origin + '/b');
    assert.deepEqual(sourceA.first_positions, [1, 3]); assert.equal(sourceA.occurrences, 2); assert.deepEqual(sourceB.first_positions, [1]);
    for (const row of occurrences.rows) {
      const capture = saved.pages.find(p => p.url === row.source.requested_url).content_captures.find(c => c.source_kind === 'raw_http_html');
      assert.equal(row.source.source_revision, capture.source_revision); assert.equal(row.source.text_sha256, capture.result.text_sha256);
      assert.equal(row.source.capture_config_sha256, capture.config_sha256); assert.equal(row.source.observed_generation, capture.observed_generation);
    }
    assert.match(await page.locator('#rows').innerText(), /Quell-URL/); assert.equal((await csv(page, occurrences)).length, 3);
    check('totals drill down through the actual browser into every exact source URL, overlapping token position and saved text/revision/generation binding');

    await page.click('#clear-phrase'); await page.selectOption('#view', 'totals'); await page.fill('#query', '');
    await page.selectOption('#n', '2'); await page.fill('#stopwords', 'and');
    const stopped = await preview(page); assert(stopped.excluded_stopword_windows[1] > 0);
    assert(!stopped.rows.some(r => r.tokens.includes('and') || r.tokens.join(' ') === 'beta gamma'));
    assert(stopped.rows.some(r => r.tokens.join(' ') === 'alpha beta'));
    await page.fill('#stopwords', ''); await page.selectOption('#case', 'preserve'); await page.selectOption('#n', '1'); await page.fill('#query', 'alpha');
    const preserved = await preview(page); assert.deepEqual(preserved.rows.map(r => [r.tokens[0], r.occurrences]).sort(), [['Alpha', 3], ['alpha', 1]]);
    await page.selectOption('#case', 'unicode_lowercase'); const folded = await preview(page); assert.equal(folded.rows.length, 1); assert.equal(folded.rows[0].occurrences, 4);
    await page.selectOption('#n', '6'); await page.fill('#query', ''); const six = await preview(page); assert(six.rows.length > 0); assert(six.rows.every(r => r.tokens.length === 6));
    check('n=1/2/6 and case options affect native counts; removed stopwords never bridge a new phrase and option changes invalidate the prior export');

    await page.selectOption('#n', '2'); await page.fill('#query', 'repeat repeat'); const repeats = await preview(page);
    assert.equal(repeats.rows[0].occurrences, 19); const sampled = await preview(page, '#rows article button');
    assert.equal(sampled.rows[0].occurrences, 19); assert.equal(sampled.rows[0].first_positions.length, 16); assert.equal(sampled.rows[0].positions_sampled, true);
    assert(sampled.rows[0].first_positions.every((position, index, positions) => index === 0 || position === positions[index - 1] + 1));
    await page.click('#clear-phrase'); await page.selectOption('#view', 'sources'); const sources = await preview(page);
    assert.equal(sources.rows.length, 5); assert.equal(sources.rows.find(r => r.source.requested_url === origin + '/empty').source.state, 'complete_empty');
    assert.equal(sources.rows.find(r => r.source.requested_url === origin + '/partial').source.counted, false);
    await page.selectOption('#source', 'rendered_dom_html'); const missingDOM = await preview(page);
    assert.equal(missingDOM.source_states.missing_or_unavailable, 5); assert(missingDOM.rows.every(r => r.source.counted === false));
    await page.selectOption('#view', 'totals'); assert.equal((await preview(page)).total_rows, 0);
    assert.match(await page.locator('#coverage').innerText(), /Fehlende oder unvollständige/);
    check('source inventory distinguishes excluded/empty rows, bounded position examples keep their full count, and absent DOM text never borrows raw data or pretends complete coverage');

    await selectCrawl(page, metadataId); await page.selectOption('#source', 'raw_http_html'); await page.fill('#query', '');
    const noCapture = await preview(page); assert.equal(noCapture.total_rows, 0); assert.equal(noCapture.source_states.missing_or_unavailable, 1);
    await page.selectOption('#channel', 'title'); const metadata = await preview(page);
    assert.equal(metadata.retained_metadata_channel.channel, 'title'); assert.equal(metadata.total_rows, 2); assert.equal(metadata.all_supplied_sources_complete, true);
    assert.deepEqual(metadata.rows.map(r => r.tokens).sort(), [['metadata', 'title'], ['saved', 'metadata']]);
    await page.locator('.channel-advanced summary').click(); await page.check('#channel-capture-only');
    const captureOnly = await preview(page); assert.equal(captureOnly.total_rows, 0); assert.equal(captureOnly.all_supplied_sources_complete, false);
    assert.deepEqual(count(), expectedCaptured);
    check('retained title metadata works without optional body capture; explicit capture-only restriction excludes unavailable text and every analysis/download makes zero website requests');

    await selectCrawl(page, limitedId); await page.selectOption('#channel', ''); await page.selectOption('#view', 'totals'); await page.fill('#query', '');
    const limited = await preview(page); const oldGeneration = limited.snapshot.caller.generation;
    const beforeResume = count(); await api('/api/crawls/' + limitedId + '/resume', {max_pages: 0});
    await until(async () => (await api('/api/crawls/' + limitedId)).status === 'completed', 'explicit fixture resume');
    const resumedCount = count(); assert.notDeepEqual(resumedCount, beforeResume);
    const staleURL = `${app}/api/crawls/${limitedId}/ngrams/csv`, denied = page.waitForResponse(r => r.url() === staleURL && r.request().method() === 'POST'); await page.click('#csv');
    const staleResponse = await denied; assert.equal(staleResponse.status(), 409); await nativeRead(staleResponse, staleURL); await until(() => page.isEnabled('#preview'), 'stale failure UI');
    assert.equal(await page.isDisabled('#csv'), true); assert.equal(await page.isVisible('#results'), false); assert.match(await page.locator('#status').innerText(), /generation|Stale/i);
    const refreshed = await preview(page); assert.notEqual(refreshed.snapshot.caller.generation, oldGeneration); assert.equal(refreshed.saved_documents, 2);
    assert.equal((await csv(page, refreshed)).length, refreshed.total_rows + 1); assert.deepEqual(count(), resumedCount);
    check('after an explicit native limited-crawl resume changes generation, the browser export gets real409, clears stale results and refreshes before a new verified export');

    await selectCrawl(page, id); await page.selectOption('#view', 'totals'); await page.selectOption('#n', '1'); await page.fill('#query', '');
    await preview(page); await page.selectOption('#cloud-mode', 'on'); await page.selectOption('#cloud-top', '10'); assert.equal(await page.locator('#cloud button').count(), 10);
    const cloud = await preview(page, '#cloud button:first-child'); assert.equal(cloud.view, 'occurrences'); assert(cloud.rows.length > 0);
    for (const width of [1440, 320]) {
      await page.setViewportSize({width, height: 900}); assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `populated ${width}px layout overflow`);
      await page.locator('#csv').scrollIntoViewIfNeeded(); const box = await page.locator('#csv').boundingBox(); assert(box.y >= 0 && box.y + box.height <= 900);
      await page.screenshot({path: artifactPath(`ngrams-${width}.png`), fullPage: true});
    }
    assert.deepEqual(count(), resumedCount); assert.deepEqual(await api('/api/crawls/' + id + '/project.json'), saved); assert.deepEqual(errors, []);
    check('bounded word cloud opens actual native source occurrences, populated 1440/320px layouts keep export reachable and the original saved report remains byte-equivalent as JSON');
    fs.writeFileSync(artifactPath('ngrams-live.json'), JSON.stringify({result: 'PASS', crawl_id: id, metadata_crawl_id: metadataId, resumed_crawl_id: limitedId,
      checks, website_requests: count(), analysis_website_requests: 0, errors, read_only_native_oracles: readEvidence, browser_downloads: downloadEvidence,
      scope: 'Actual redesigned browser/native Rust saved-crawl corpus, snapshot paging, generation conflict, JSON and hash/length verified CSV. Only owned synthetic HTML/native requests. No current whole-site coverage, reference tokenization parity, imported/tampered archive, cold restart or provider claim.'}, null, 2) + '\n');
    console.log(`${checks.length} native saved n-gram workflows passed`);
  } catch (error) {
    await page.screenshot({path: artifactPath('ngrams-failure.png'), fullPage: true});
    const layout = await page.evaluate(() => ({viewport: innerWidth, document: document.documentElement.scrollWidth, status: document.getElementById('status').textContent,
      overflow: [...document.querySelectorAll('body *')].filter(e => e.getBoundingClientRect().right > innerWidth + 1).slice(0, 25).map(e => ({tag: e.tagName, id: e.id, width: e.getBoundingClientRect().width, text: e.textContent.slice(0, 90)}))}));
    fs.writeFileSync(artifactPath('ngrams-failure.json'), JSON.stringify({error: error.stack, checks, hits: count(), layout, read_only_native_oracles: readEvidence, browser_downloads: downloadEvidence}, null, 2) + '\n'); throw error;
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; }).finally(async () => {
  website.closeAllConnections(); await new Promise(r => website.close(r));
});
