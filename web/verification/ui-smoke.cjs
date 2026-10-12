// Isolated UI fixture test. This does not claim Rust crawler or live-site proof.
// Set SPIDER_PLAYWRIGHT_MODULE and SPIDER_CHROME when using other installations.
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const {pathToFileURL}=require('node:url');
const assert = require('node:assert/strict');
const web = path.resolve(__dirname, '..');
const moduleCandidates = [process.env.SPIDER_PLAYWRIGHT_MODULE, path.resolve(web, '../renderer/node_modules/playwright-core'), 'playwright-core', path.join(process.env.APPDATA || '', 'npm/node_modules/rea-agents/node_modules/playwright-core')].filter(Boolean);
let playwright;
for(const candidate of moduleCandidates) { try { playwright=require(candidate); break; } catch (_) { /* Try the next installed test dependency. */ } }
if(!playwright) throw new Error('UI smoke test requires Playwright. Run npm ci in renderer or set SPIDER_PLAYWRIGHT_MODULE.');
const { chromium } = playwright;
const chrome = process.env.SPIDER_CHROME || path.join(process.env.ProgramFiles || 'C:/Program Files', 'Google/Chrome/Application/chrome.exe');
async function testBrowserPath(){if(process.env.SPIDER_CHROME)return process.env.SPIDER_CHROME;try{return await(await import(pathToFileURL(path.resolve(web,'../renderer/server.mjs')).href)).findBrowserExecutable()}catch(_){return fs.existsSync(chrome)?chrome:null}}
const pages = Array.from({ length: 115 }, (_, i) => {
  const url = `https://fixture.test/page-${String(i).padStart(3, '0')}`;
  return { url, final_url: url, internal: true, depth: i ? 1 : 0, status: i === 114 ? 404 : 200, final_status: i === 114 ? 404 : 200, content_type: 'text/html', response_ms: 20 + i,
    titles: i % 10 === 0 ? [] : [i === 1 ? '"><img src=x onerror=window.__xss=true>' : `Fixture title ${i % 15}`], meta_descriptions: [`Description ${i}`], h1: [`Heading ${i}`], h2: ['Subheading'], canonical: [url], meta_robots: i === 113 ? ['noindex'] : [], x_robots_tag: [], hreflang: [{ language: 'de', url }], language: 'de',
    images: [{ url: `https://fixture.test/image-${i}.png`, alt: i % 7 === 0 ? null : `Image ${i}`, width: 400, height: 300, loading: 'lazy' }],
    links: [{ url: `https://fixture.test/page-${String((i + 1) % 115).padStart(3, '0')}`, text: 'Next page', internal: true, kind: 'anchor', nofollow: false, rel: '' }, { url: 'https://external.test/', text: 'External', internal: false, kind: 'anchor', nofollow: true, rel: 'nofollow' }],
    word_count: 100 + i, indexable: i < 113, indexability_reasons: [], headers: { 'content-type': 'text/html' }, redirect_chain: [], size_bytes: 1024, structured_data: [], invalid_structured_data: 0, custom_extractions: i===1||i===2?[{name:'price',selector:'.price',attribute:null,values:i===1?['42.00']:[],source:'raw_html'}]:[], browser_version:i===1?'Fixture Chromium 154':null,rendered_viewport:i===1?{width:1280,height:800}:null,render_wait_ms:i===1?500:null,network_requests:i===1?[{url:'https://fixture.test/sample'}]:[],network_events_dropped:i===1?7:0 };
});
pages[3].html_canonical=[pages[3].url,pages[3].url];pages[3].title_pixel_widths=[700];
Object.assign(pages[10],{content_type:'application/pdf',titles:[],meta_descriptions:[],h1:[],h2:[],header_canonical:[pages[10].url],html_canonical:[]});
const issues = [{ url: pages[114].url, code: 'http_error', severity: 'error', message: 'HTTP 404', related_urls: [] }, { url: pages[0].url, code: 'title_missing', severity: 'warning', message: 'Title missing', related_urls: [] }];
const analysis = { graph: { nodes: pages.map((p, i) => ({ url: p.url, in_degree: 1, out_degree: 1, inlink_occurrences: 1, outlink_occurrences: 1, crawl_depth: p.depth, shortest_link_depth: p.depth, link_score: 100 - i / 2, indexable: p.indexable })), edges: pages.map((p) => ({ source: p.url, target: p.links[0].url, occurrences: 1, passes_score: true })), orphan_candidates: [] }, structured_data: [{ url: pages[1].url, parsed_blocks: 1, invalid_json_blocks: 0, types: { Article: 1 }, typed_nodes: [{ types: ['Article'] }] }], issues: [{ url: pages[1].url, code: 'security_csp_missing', severity: 'notice', message: 'No CSP observed', related_urls: [] }] };
let created = null, sequence = 0, imports = 0;
const status = (id = 'fixture') => ({ id, status: 'completed', seed_url: pages[0].url, config: { seed_urls: [pages[0].url] }, started_at: '2026-10-11T12:00:00Z', finished_at: '2026-10-11T12:00:10Z', stats: { fetched: 115, queued: 0, discovered: 115, failed: 1, issues: 2 }, warnings: ['Fixture warning <script>window.__xss=true</script>'] });
const report = { config: { seed_urls: [pages[0].url] }, summary: { started_at: status().started_at, finished_at: status().finished_at }, pages, issues };
const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, 'http://localhost');
  const send = (body, type = 'application/json') => { res.setHeader('content-type', type); res.end(type === 'application/json' ? JSON.stringify(body) : body); };
  if (url.pathname === '/api/crawls' && req.method === 'GET') return send({ crawls: [status(), ...(created ? [status(created.id)] : [])] });
  if (url.pathname === '/api/crawls' && req.method === 'POST') { let body = ''; for await (const chunk of req) body += chunk; created = { id: `created-${++sequence}`, payload: JSON.parse(body) }; return send({ id: created.id, status: 'running' }); }
  if (url.pathname === '/api/crawls/import') { imports++; return send({ id: 'fixture', status: 'completed' }); }
  if (url.pathname.endsWith('/pages')) { const offset = Number(url.searchParams.get('offset') || 0), limit = Number(url.searchParams.get('limit') || 1000); return send({ pages: pages.slice(offset, offset + limit), total: pages.length, offset, limit }); }
  if (url.pathname.endsWith('/issues')) return send({ issues, total: issues.length });
  if (url.pathname.endsWith('/analysis')) return send(analysis);
  if (url.pathname.endsWith('/project.json')) return send(report);
  if (url.pathname.endsWith('/sitemap.xml')) return send('<?xml version="1.0"?><urlset/>', 'application/xml');
  if (/^\/api\/crawls\//.test(url.pathname)) return send(status(url.pathname.split('/')[3]));
  const file = url.pathname === '/' ? 'index.html' : url.pathname.slice(1);
  if (!['index.html', 'app.js', 'styles.css'].includes(file)) { res.statusCode = 404; return res.end(); }
  send(fs.readFileSync(path.join(web, file), 'utf8'), file.endsWith('css') ? 'text/css' : file.endsWith('js') ? 'text/javascript' : 'text/html');
});

(async () => {
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const executable=await testBrowserPath();
  const browser = await chromium.launch({ ...(executable ? { executablePath: executable } : {}), headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, acceptDownloads: true });
  const errors = [], checks = [];
  page.on('pageerror', (error) => errors.push(error.message));
  const checked = (name) => { checks.push(name); console.log(`PASS ${name}`); };
  try {
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    await page.locator('#connection-state.connected').waitFor();
    assert.equal(await page.locator('#metric-fetched').innerText(), '0'); checked('empty state');
    await page.selectOption('#crawl-history', 'fixture');
    await page.waitForFunction(() => document.getElementById('metric-fetched').textContent === '115' && document.getElementById('metric-issues').textContent === '3');
    assert.equal(await page.locator('#metric-issues').innerText(), '3'); checked('snapshot and advanced analysis');
    await page.click('[data-section=internal]');
    assert.equal(await page.locator('#results-table tbody tr.click-row').count(), 50);
    await page.click('#page-next'); assert.match(await page.locator('#page-label').innerText(), /^51.*100 von 115$/); checked('bounded pagination');
    await page.fill('#table-search', 'page-012'); await page.waitForFunction(() => document.querySelectorAll('#results-table tbody tr.click-row').length === 1);
    await page.locator('#results-table tbody tr.click-row').click(); assert.match(await page.locator('#inspector-content').innerText(), /Fixture title 12/);
    await page.click('[data-tab=outlinks]'); assert.equal(await page.locator('#inspector-content li').count(), 2);
    await page.click('[data-tab=inlinks]'); assert.equal(await page.locator('#inspector-content li').count(), 1); checked('search and URL inspector links');
    await page.click('[data-section=titles]'); await page.selectOption('#table-filter', 'missing'); assert.equal(await page.locator('#result-count').innerText(), '11 Ergebnisse');
    await page.selectOption('#table-filter', 'all'); await page.click('[data-sort=title_length]'); assert.equal(await page.locator('th[aria-sort=ascending]').count(), 1); checked('title filter and numeric sorting');
    await page.selectOption('#table-filter','pixels');assert.equal(await page.locator('#result-count').innerText(),'1 Ergebnisse');assert.match(await page.locator('#results-table tbody').innerText(),/700/);checked('pixel filters require observed widths and preserve unknown older reports');
    await page.click('[data-section=canonical]');await page.selectOption('#table-filter','multiple');assert.equal(await page.locator('#result-count').innerText(),'1 Ergebnisse');assert.match(await page.locator('#results-table tbody').innerText(),/page-003/);await page.selectOption('#table-filter','all');await page.fill('#table-search','page-010');assert.match(await page.locator('#results-table tbody').innerText(),/page-010/);await page.click('[data-section=hreflang]');await page.fill('#table-search','page-010');assert.match(await page.locator('#results-table tbody').innerText(),/page-010/);checked('duplicate canonical declarations and HTTP relationships on non-HTML remain visible');
    assert.equal(await page.evaluate(() => Boolean(window.__xss)), false); assert.equal(await page.locator('#results-table img').count(), 0); checked('untrusted metadata escaped');
    await page.click('[data-section=images]'); await page.selectOption('#table-filter', 'missing'); assert.equal(await page.locator('#result-count').innerText(), '17 Ergebnisse'); checked('image alt filter');
    await page.click('[data-section=issues]'); await page.selectOption('#table-filter', 'notice'); assert.equal(await page.locator('#result-count').innerText(), '1 Ergebnisse'); checked('advanced issue filter');
    await page.click('[data-section=linkmetrics]'); assert.equal(await page.locator('#result-count').innerText(), '115 Ergebnisse');
    await page.click('[data-section=structured]'); assert.equal(await page.locator('#result-count').innerText(), '1 Ergebnisse'); checked('link metrics and JSON-LD');
    await page.click('[data-section=extractions]'); assert.equal(await page.locator('#result-count').innerText(),'2 Ergebnisse'); await page.selectOption('#table-filter','empty'); assert.equal(await page.locator('#result-count').innerText(),'1 Ergebnisse'); await page.selectOption('#table-filter','present'); await page.locator('#results-table tbody tr.click-row').click(); assert.match(await page.locator('#inspector-content').innerText(),/42\.00/); assert.match(await page.locator('#inspector-content').innerText(),/raw_html/); checked('CSS extraction results and source in table and inspector');
    const provenance=await page.locator('#inspector-content').innerText();assert.match(provenance,/Fixture Chromium 154/);assert.match(provenance,/"width":1280,"height":800/);assert.match(provenance,/500 ms/);assert.match(provenance,/7 weitere Netzwerkereignisse nicht gespeichert \(Aufzeichnung abgeschnitten\)/);checked('renderer provenance and explicit dropped network observations');
    await page.click('[data-section=graph]'); assert.equal(await page.locator('.graph-node').count(), 70); assert.equal(await page.locator('#graph-view svg path').count(), 69); checked('bounded real anchor graph');
    for (const [id, pattern] of [['export-csv', /internal/], ['export-sitemap', /sitemap.xml$/], ['export-project', /fixture.json$/]]) { const pending = page.waitForEvent('download'); await page.click(`#${id}`); assert.match((await pending).suggestedFilename(), pattern); } checked('CSV sitemap project downloads');
    await page.click('#settings-open'); await page.fill('[name=max_pages]', '520'); await page.fill('[name=delay_ms]', '0'); await page.check('[name=check_external]'); await page.check('[name=javascript_rendering]'); await page.locator('#renderer-settings').waitFor({state:'visible'}); await page.fill('[name=renderer_wait_ms]','750'); await page.check('[name=renderer_accessibility]'); await page.check('[name=renderer_screenshot]');
    await page.locator('#advanced-config summary').click(); await page.fill('[name=custom_extractions_json]','not-json'); await page.click('#settings-form button[type=submit]'); await page.locator('#settings-error').waitFor({state:'visible'}); assert.equal(await page.locator('#settings-dialog').isVisible(),true); checked('invalid extraction JSON blocks settings save');
    await page.fill('[name=custom_extractions_json]',JSON.stringify([{name:'bad',selector:'???',attribute:null}])); await page.click('#settings-form button[type=submit]'); assert.match(await page.locator('#settings-error').innerText(),/gültigen CSS-Selektor/); checked('invalid CSS rejected before starting');
    await page.fill('[name=custom_extractions_json]','[]');await page.fill('[name=custom_javascript_json]',JSON.stringify([{name:'same',script:'return 1;'},{name:'same',script:'return 2;'}]));await page.click('#settings-form button[type=submit]');assert.match(await page.locator('#settings-error').innerText(),/mehrfach/);await page.fill('[name=custom_javascript_json]','[]');checked('duplicate JavaScript output names rejected in settings');
    await page.fill('[name=audit_thresholds_json]','{"min_title_chars":100,"max_title_chars":60}');await page.click('#settings-form button[type=submit]');assert.match(await page.locator('#settings-error').innerText(),/Minimum/);await page.fill('[name=audit_thresholds_json]','{"max_title_chars":65,"min_word_count":null}');checked('inconsistent thresholds rejected and individual checks can be disabled');
    await page.fill('[name=custom_extractions_json]',JSON.stringify([{name:'price',selector:'.price',attribute:null},{name:'link',selector:'a',attribute:'href'}])); await page.fill('[name=include_patterns_text]','/page\n(?i)price\n/page'); await page.fill('[name=exclude_patterns_text]','/logout\n/checkout'); await page.click('#settings-form button[type=submit]'); assert.match(await page.locator('#settings-summary').innerText(), /520/); checked('persisted settings');
    await page.selectOption('#crawl-mode', 'list'); await page.fill('#url-list', 'https://fixture.test/a\nhttps://fixture.test/b\nhttps://fixture.test/a'); await page.click('#start-button'); await page.waitForFunction(() => document.getElementById('crawl-state').textContent.includes('abgeschlossen'));
    assert.equal(created.payload.seed_urls.length, 2); assert.equal(created.payload.crawl_links, false); assert.equal(created.payload.max_pages, 520); assert.equal(created.payload.check_external, true); checked('list mode POST contract');
    assert.deepEqual(created.payload.renderer,{endpoint:'http://127.0.0.1:8790/render',wait_ms:750,accessibility:true,screenshot:true,collect_cookies:false,custom_javascript:[]}); checked('opt-in renderer POST contract, no runtime claim');
    const extractionDefaults={method:'css',max_results:10000,regex_group:null,regex_flags:'',namespaces:{}};assert.deepEqual(created.payload.custom_extractions,[{name:'price',selector:'.price',attribute:null,...extractionDefaults},{name:'link',selector:'a',attribute:'href',...extractionDefaults}]); assert.deepEqual(created.payload.include_patterns,['/page','(?i)price']); assert.deepEqual(created.payload.exclude_patterns,['/logout','/checkout']);assert.deepEqual(created.payload.audit_thresholds,{max_title_chars:65,min_word_count:null});checked('named CSS Rust regex and custom audit thresholds POST contract');
    await page.selectOption('#crawl-mode', 'sitemap'); await page.fill('#seed-url', 'https://fixture.test/sitemap.xml'); await page.click('#start-button'); await page.waitForFunction(() => document.getElementById('crawl-state').textContent.includes('abgeschlossen'));
    assert.deepEqual(created.payload.seed_urls, []); assert.deepEqual(created.payload.sitemap_urls, ['https://fixture.test/sitemap.xml']); checked('sitemap mode POST contract');
    await page.setInputFiles('#import-file', { name: 'report.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(report)) }); await page.waitForFunction(() => document.getElementById('message').textContent.includes('importiert')); assert.equal(imports, 1); checked('report import');
    await page.click('[data-section=internal]');
    if (await page.locator('#inspector-close').isVisible()) await page.click('#inspector-close');
    for (const [width, height] of [[1920, 1080], [1440, 900], [390, 844], [320, 740]]) {
      await page.setViewportSize({ width, height });
      const dimensions = await page.evaluate(() => ({ viewport: innerWidth, width: document.documentElement.scrollWidth }));
      assert.ok(dimensions.width <= dimensions.viewport, `Document overflow: ${JSON.stringify(dimensions)}`);
      await page.locator('.main-footer').scrollIntoViewIfNeeded();
      const box = await page.locator('.main-footer').boundingBox(); assert.ok(box.y < height && box.y + box.height > 0);
      checked(`viewport ${width}: no document overflow, footer reachable`);
    }
    await page.setViewportSize({width:390,height:844}); await page.click('#settings-open'); await page.locator('.dialog-footer button').scrollIntoViewIfNeeded(); const saveBox=await page.locator('.dialog-footer button').boundingBox(); assert.ok(saveBox.y>=0&&saveBox.y+saveBox.height<=844); assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)); await page.click('#settings-close'); checked('expanded renderer and extraction settings bottom reachable on mobile');
    assert.deepEqual(errors, []);
    await page.setViewportSize({ width: 1440, height: 1000 }); await page.evaluate(() => scrollTo(0, 0)); await page.screenshot({ path: path.join(__dirname, 'browser-wide.png'), fullPage: true });
    await page.setViewportSize({ width: 390, height: 844 }); await page.screenshot({ path: path.join(__dirname, 'browser-mobile.png'), fullPage: true });
    const proof = { tested_at: new Date().toISOString(), scope: 'Isolated browser fixture only; not live Rust backend proof', browser_version: await browser.version(), browser_executable: executable || chromium.executablePath(), checks, javascript_runtime_errors: errors };
    fs.writeFileSync(path.join(__dirname, 'ui-fixture-results.json'), JSON.stringify(proof, null, 2));
    console.log(`${checks.length} UI checks passed.`);
  } finally { await browser.close(); await new Promise((resolve) => server.close(resolve)); }
})().catch((error) => { console.error(error); server.close(); process.exitCode = 1; });
