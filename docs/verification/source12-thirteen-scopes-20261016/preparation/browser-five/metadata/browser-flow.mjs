import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { bound } from './owned-runtime.mjs';
import { expected, assertCsv, assertSources, metadataRequest, LONG_WORD } from './fixture-oracle.mjs';

export async function runChannelBrowser({ browser, api, jobs, origin, output, unchanged, retained }) {
  function keep(p) { retained.add(p); p.then(() => retained.delete(p), () => retained.delete(p)); return p; }
  const context = await keep(browser.newContext({ acceptDownloads: true, viewport: { width: 1440, height: 1000 } }));
  const page = await keep(context.newPage()), errors = [], posts = [], external = [], widths = [], gates = [], handlers = [], downloads = [];
  page.setDefaultTimeout(30000);
  page.on('pageerror', error => errors.push(String(error)));
  page.on('request', request => {
    if (!request.url().startsWith(api + '/')) external.push(request.url());
    if (request.method() === 'POST') posts.push({ url: request.url(), body: request.postDataJSON() });
  });
  const ready = () => page.waitForFunction(() => !document.querySelector('#preview').disabled && !document.querySelector('#results').hidden);
  const rows = async () => (await page.locator('#rows article details pre').allTextContents()).map(JSON.parse);
  async function submit() { await page.locator('#preview').click(); await ready(); }
  async function bounds() {
    const result = await page.evaluate(() => {
      const content = document.documentElement.clientWidth;
      return { width: innerWidth, content, document: document.documentElement.scrollWidth, body: document.body.scrollWidth,
        bad: [...document.querySelectorAll('main button,main input,main select,main textarea,main article,main h1,main dl')]
          .filter(e => e.getClientRects().length).map(e => ({ id: e.id, tag: e.tagName, rect: e.getBoundingClientRect().toJSON() }))
          .filter(e => e.rect.left < -1 || e.rect.right > content + 1) };
    });
    assert(result.content > 0 && result.content <= result.width);
    assert(result.document > 0 && result.document <= result.content + 1);
    assert(result.body > 0 && result.body <= result.content + 1); assert.deepEqual(result.bad, []);
    return result;
  }
  async function download(id, basename) {
    const promised = keep(page.waitForEvent('download', { timeout: 30000 })); downloads.push(promised);
    const task = keep((async () => {
      await page.locator('#' + id).click(); const d = await promised;
      assert.equal(await bound(keep(d.failure()), 15000, 'Actual owned download failure status'), null);
      const target = path.join(output, basename); await bound(keep(d.saveAs(target)), 30000, 'Actual owned download save close');
      await page.waitForFunction(() => !document.querySelector('#preview').disabled); return fs.readFileSync(target);
    })()); downloads.push(task); return await task;
  }
  try {
    const want = expected(origin, jobs.raw.paths, 'title', 1);
    assert.equal(want.rows.length, 210);
    for (const width of [1440, 320]) {
      await page.setViewportSize({ width, height: 1000 });
      await page.goto(api + '/saved-ngrams', { waitUntil: 'networkidle' });
      await page.waitForFunction(id => [...document.querySelector('#crawl').options].some(o => o.value === id) && !document.querySelector('#preview').disabled, jobs.raw.id);
      await page.locator('#crawl').selectOption(jobs.raw.id);
      assert.equal(await page.locator('#channel').inputValue(), '');
      assert.equal(await page.locator('#channel-capture-only').isChecked(), false);
      const beforeBody = posts.length; await submit();
      const body = posts.slice(beforeBody).at(-1).body;
      assert(!Object.hasOwn(body, 'html_text_channel') && !Object.hasOwn(body, 'retained_metadata_channel'));
      assert.equal((await rows()).length, 0);
      await page.locator('#channel').selectOption('title'); await page.locator('#n').selectOption('1');

      // Hold a real fetched API response without changing its status/headers/body.
      let enter, release;
      const entered = new Promise(resolve => { enter = resolve; });
      const released = new Promise(resolve => { release = resolve; }); gates.push(release);
      const routePattern = api + '/api/crawls/' + jobs.raw.id + '/ngrams/preview';
      const routeHandler = route => {
        const task = (async () => {
          const response = await route.fetch({ timeout: 30000 }); assert.equal(response.status(), 200);
          const actual = await response.body(); assert(actual.length > 0);
          assert.equal(Number(response.headers()['content-length']), actual.length); enter();
          await bound(released, 30000, 'Release retained genuine preview response');
          await route.fulfill({ response });
        })();
        keep(task); handlers.push(task); return task;
      };
      await page.route(routePattern, routeHandler);
      const beforeHeld = posts.length; await page.locator('#preview').click();
      await bound(entered, 30000, 'Real metadata API response entered');
      for (const id of ['crawl', 'source', 'channel', 'channel-capture-only', 'view', 'n', 'query', 'case', 'stopwords', 'cloud-mode', 'cloud-top', 'preview', 'reload', 'clear-phrase', 'next', 'json', 'csv']) assert(await page.locator('#' + id).isDisabled(), id);
      await page.locator('#ngram-form').evaluate(form => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
      await page.evaluate(() => new Promise(resolve => setTimeout(resolve, 0)));
      assert.equal(posts.length - beforeHeld, 1, 'Busy submit must not start another API request');
      release(); await bound(Promise.all([...handlers]), 30000, 'Actual retained preview handlers finish');
      await ready(); await page.unroute(routePattern, routeHandler);
      assert.deepEqual(posts.at(-1).body.retained_metadata_channel, { version: 1, channel: 'title' });
      assert(!Object.hasOwn(posts.at(-1).body, 'html_text_channel'));
      // The real held title preview above opens #results after channel/n reset.
      assert(await page.locator('#results').isVisible());
      await page.locator('#cloud-mode').scrollIntoViewIfNeeded();
      assert(await page.locator('#cloud-mode').isVisible());
      assert(await page.locator('#cloud-mode').isEnabled());
      await page.locator('#cloud-mode').selectOption('on');

      for (const top of [10, 25, 50, 100]) {
        await page.locator('#cloud-top').selectOption(String(top));
        assert.deepEqual(await page.locator('#cloud .cloud-word').allTextContents(), want.rows.slice(0, top).map(r => r.tokens.join(' ')));
        const nodes = await page.locator('#cloud .cloud-word').evaluateAll(buttons => buttons.map(b => ({ onlyText: [...b.childNodes].every(n => n.nodeType === Node.TEXT_NODE), font: parseFloat(b.style.fontSize), title: b.title })));
        assert(nodes.every(n => n.onlyText && n.font >= 0.9 && n.font <= 2.201));
        for (const [i, node] of nodes.entries()) assert.equal(node.title, want.rows[i].occurrences + ' Vorkommen · ' + want.rows[i].document_frequency + ' gespeicherte Quellseiten');
        await bounds();
      }
      await page.locator('#cloud .cloud-word').first().scrollIntoViewIfNeeded();
      await page.locator('#cloud .cloud-word').first().click({ trial: true }); await page.locator('#cloud .cloud-word').first().click(); await ready();
      const sourceRows = await rows(), phrase = want.rows[0].tokens;
      const memberships = want.sources.filter(s => s.own.has(JSON.stringify(phrase)));
      assert.equal(sourceRows.length, memberships.length);
      for (const row of sourceRows) {
        const s = memberships.find(s => s.url === row.source.requested_url); assert(s);
        assert.deepEqual(row.tokens, phrase); assert.equal(row.occurrences, s.own.get(JSON.stringify(phrase)).occurrences);
        assert.deepEqual(row.first_positions, s.own.get(JSON.stringify(phrase)).positions);
      }
      assert.equal(await page.locator('#cloud .cloud-word').count(), 0);
      await page.locator('#view').selectOption('totals'); await submit();
      const seen = [], pages = [];
      for (const count of [100, 100, 10]) {
        const actual = await rows(); assert.equal(actual.length, count); pages.push(count); seen.push(...actual);
        await bounds();
        if (pages.length < 3) { await page.locator('#next').click(); await ready(); assert.equal(await page.locator('#cloud .cloud-word').count(), 0); }
      }
      assert.deepEqual(seen.map(r => [r.tokens, r.occurrences, r.document_frequency]), want.rows.map(r => [r.tokens, r.occurrences, r.document_frequency]));
      assert(await page.locator('#next').isHidden());
      const csv = await download('csv', 'metadata-ui-' + width + '.csv'); assertCsv(csv, want, { channel: 'title' });
      const json = JSON.parse((await download('json', 'metadata-ui-' + width + '.json')).toString());
      assert.equal(json.after, 200); assert.equal(json.total_rows, 210); assert.equal(json.rows.length, 10);

      await page.locator('#channel').selectOption('h6'); await page.locator('#view').selectOption('totals'); await submit();
      assert((await page.locator('#cloud .cloud-word').allTextContents()).includes(LONG_WORD));
      assert.equal(await page.locator('#cloud img,#rows img').count(), 0);
      assert.equal(await page.evaluate(() => window.NGRAM_INJECTED), undefined); await bounds();
      await page.locator('#channel').selectOption('h4'); await page.locator('#view').selectOption('sources'); await submit();
      assertSources(await rows(), expected(origin, jobs.raw.paths, 'h4', 1), { report: jobs.raw.report });
      await page.locator('.channel-advanced > summary').click();
      await page.locator('#channel-capture-only').check(); await submit(); assert((await rows()).every(r => r.source.counted === false));
      assert.deepEqual(posts.at(-1).body.html_text_channel, { version: 1, channel: 'h4' });
      assert(!Object.hasOwn(posts.at(-1).body, 'retained_metadata_channel'));
      await page.locator('#crawl').selectOption(jobs.captured.id); await page.locator('#channel').selectOption('title'); await submit();
      const completeRows = await rows(); assert.equal(completeRows.length, 1); assert.equal(completeRows[0].source.counted, true);
      assert.equal(completeRows[0].source.source_revision, jobs.captured.report.pages[0].content_captures[0].source_revision);
      assert.equal(completeRows[0].source.channel.capture_html_sha256, jobs.captured.report.pages[0].content_captures[0].source_html_utf8_sha256);
      assert(completeRows[0].source.channel.metadata_source == null);
      await page.locator('#channel-capture-only').uncheck();
      await page.locator('#crawl').selectOption(jobs.budget.id); await page.locator('#channel').selectOption('title'); await submit();
      assertSources(await rows(), expected(origin, ['/budget'], 'title', 1), { captured: true, report: jobs.budget.report });
      await page.locator('#crawl').selectOption(jobs.dom.id); await page.locator('#source').selectOption('rendered_dom_html'); await submit();
      assertSources(await rows(), expected(origin, jobs.dom.paths, 'title', 1, { source: 'rendered_dom_html' }), { kind: 'rendered_dom_html', report: jobs.dom.report });
      await bounds(); await page.screenshot({ path: path.join(output, 'metadata-' + width + '.png'), fullPage: false });
      await unchanged('metadata browser ' + width); widths.push({ width, positive_content_bounds: await bounds(), paging: pages, csv_rows: 210, source_memberships: sourceRows.length, genuine_busy_barrier: true, text_nodes_only: true });
    }
    assert.deepEqual(errors, []); assert.deepEqual(external, []);
    assert(posts.every(p => /\/api\/crawls\/[^/]+\/ngrams\/(?:preview|csv)$/.test(p.url)));
    return { widths, page_errors: errors, external_requests: external, actual_api_posts: posts, crawl_start_posts: 0, positive_csv_downloads: 2 };
  } finally {
    for (const release of gates) release();
    const settled = await Promise.allSettled([
      bound(keep(context.close()), 15000, 'Actual metadata browser context close'),
      bound(Promise.allSettled([...handlers, ...downloads]).then(results => { const failed = results.filter(r => r.status === 'rejected'); if (failed.length) throw new AggregateError(failed.map(r => r.reason), 'Retained metadata response/download failure'); }), 35000, 'All retained metadata response and download promises settle'),
    ]);
    const failures = settled.filter(r => r.status === 'rejected'); if (failures.length) throw new AggregateError(failures.map(r => r.reason), 'Owned metadata browser cleanup failed');
  }
}
