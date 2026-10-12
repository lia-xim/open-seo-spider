import http from 'node:http';
import { access, stat } from 'node:fs/promises';
import { constants } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium } from 'playwright-core';
import axe from 'axe-core';

const MAX_BODY_BYTES = 1024 * 1024;
const MAX_HTML_BYTES = 20 * 1024 * 1024;
const MAX_EVENTS = 10000;
const MAX_SCRIPT_RESULT_BYTES = 65536;
const SCRIPT_TIMEOUT_MS = 2000;
const DEFAULT_USER_AGENT = 'OpenSEOSpider/0.1 (+local Chromium SEO audit)';
const HEADER_NAME = /^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/;
const FORBIDDEN_HEADERS = new Set(['host', 'cookie', 'content-length', 'connection',
  'transfer-encoding', 'proxy-authorization', 'proxy-connection', 'upgrade']);

export class WorkerError extends Error {
  constructor(status, code, message) { super(message); this.status = status; this.code = code; }
}

function boundedInteger(value, fallback, minimum, maximum, name) {
  const result = value === undefined ? fallback : value;
  if (!Number.isSafeInteger(result) || result < minimum || result > maximum) {
    throw new WorkerError(400, 'invalid_request', `${name} must be an integer from ${minimum} to ${maximum}`);
  }
  return result;
}

export function validateRenderRequest(input) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    throw new WorkerError(400, 'invalid_request', 'Request must be a JSON object');
  }
  if (typeof input.url !== 'string' || input.url.length > 8192) {
    throw new WorkerError(400, 'invalid_url', 'url must be an HTTP(S) URL string of at most 8192 characters');
  }
  let url;
  try { url = new URL(input.url); } catch { throw new WorkerError(400, 'invalid_url', 'url is not a valid URL'); }
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) {
    throw new WorkerError(400, 'invalid_url', 'Only HTTP(S) URLs without embedded credentials are accepted');
  }
  const userAgent = input.user_agent ?? DEFAULT_USER_AGENT;
  if (typeof userAgent !== 'string' || !userAgent.trim() || userAgent.length > 1024 || /[\x00-\x1f\x7f]/.test(userAgent)) {
    throw new WorkerError(400, 'invalid_request', 'user_agent must be a nonempty string without control characters');
  }
  const timeout = boundedInteger(input.timeout_ms, 30000, 500, 120000, 'timeout_ms');
  const wait = boundedInteger(input.wait_ms, 1000, 0, 30000, 'wait_ms');
  if (wait >= timeout) throw new WorkerError(400, 'invalid_request', 'wait_ms must be less than timeout_ms');
  const custom = input.custom_headers ?? {};
  if (!custom || typeof custom !== 'object' || Array.isArray(custom) || Object.keys(custom).length > 100) {
    throw new WorkerError(400, 'invalid_headers', 'custom_headers must be an object with at most 100 entries');
  }
  const headers = Object.create(null);
  let headerBytes = 0;
  for (const [name, value] of Object.entries(custom)) {
    const lower = name.toLowerCase();
    if (!HEADER_NAME.test(name) || FORBIDDEN_HEADERS.has(lower) || lower.startsWith('sec-')) {
      throw new WorkerError(400, 'invalid_headers', `Unsupported header name: ${name}`);
    }
    if (typeof value !== 'string' || /[\x00-\x1f\x7f]/.test(value)) {
      throw new WorkerError(400, 'invalid_headers', 'Header values must be strings without control characters');
    }
    if (Object.hasOwn(headers, lower)) throw new WorkerError(400, 'invalid_headers', 'Header names must be unique ignoring case');
    headerBytes += Buffer.byteLength(name) + Buffer.byteLength(value);
    headers[lower] = value;
  }
  if (headerBytes > 32768) throw new WorkerError(400, 'invalid_headers', 'Custom headers exceed 32768 bytes');
  const flags = {};
  for (const name of ['screenshot', 'accessibility', 'collect_cookies']) {
    if (input[name] !== undefined && typeof input[name] !== 'boolean') {
      throw new WorkerError(400, 'invalid_request', `${name} must be a boolean`);
    }
    flags[name] = input[name] === true;
  }
  const customJavascript = input.custom_javascript ?? [];
  if (!Array.isArray(customJavascript) || customJavascript.length > 32) {
    throw new WorkerError(400, 'invalid_request', 'custom_javascript must be an array with at most 32 entries');
  }
  const scriptNames = new Set(); let scriptBytes = 0;
  const scripts = customJavascript.map((entry) => {
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)
        || typeof entry.name !== 'string' || !entry.name.trim()
        || Buffer.byteLength(entry.name.trim()) > 128 || /[\x00-\x1f\x7f]/.test(entry.name)
        || typeof entry.script !== 'string' || !entry.script.trim()
        || Buffer.byteLength(entry.script) > 65536) {
      throw new WorkerError(400, 'invalid_request', 'Each JavaScript entry needs a name up to 128 bytes and a nonempty script up to 65536 bytes');
    }
    const name = entry.name.trim();
    if (scriptNames.has(name)) throw new WorkerError(400, 'invalid_request', 'JavaScript names must be unique');
    scriptNames.add(name); scriptBytes += Buffer.byteLength(entry.script);
    if (scriptBytes > 262144) throw new WorkerError(400, 'invalid_request', 'JavaScript source exceeds 262144 bytes');
    return { name, script: entry.script };
  });
  const viewport = input.viewport ?? { width: 1280, height: 800 };
  if (!viewport || typeof viewport !== 'object' || Array.isArray(viewport)) {
    throw new WorkerError(400, 'invalid_request', 'viewport must contain integer width and height');
  }
  return { url: url.href, origin: url.origin, user_agent: userAgent,
    timeout_ms: timeout, wait_ms: wait, custom_headers: headers, custom_javascript: scripts, ...flags,
    viewport: { width: boundedInteger(viewport.width, 1280, 100, 4096, 'viewport.width'),
      height: boundedInteger(viewport.height, 800, 100, 4096, 'viewport.height') } };
}

// This code is an explicitly configured browser function body. It never runs
// in Node or shares a filesystem/runtime with the worker process.
async function evaluateCustomJavascript(page, definitions) {
  if (!definitions.length) return [];
  const session = await page.context().newCDPSession(page);
  const results = [];
  try {
    for (const definition of definitions) {
      const started = Date.now(); let timer;
      try {
        const expression = `(async()=>{const value=await(async()=>{\n${definition.script}\n})();
          const value_type=value===null?'null':typeof value;
          if(value_type==='function'||value_type==='symbol'||value_type==='bigint'
             ||(value_type==='number'&&!Number.isFinite(value)))throw new Error('Result must be JSON-compatible');
          return {serialized:JSON.stringify(value)??'null',value_type};})()`;
        const output = await Promise.race([
          page.evaluate(expression),
          new Promise((_, reject) => { timer = setTimeout(() => reject(new WorkerError(504, 'custom_javascript_timeout', `JavaScript ${definition.name} exceeded ${SCRIPT_TIMEOUT_MS} ms`)), SCRIPT_TIMEOUT_MS); })
        ]);
        if (Buffer.byteLength(output.serialized) > MAX_SCRIPT_RESULT_BYTES) throw new Error('JavaScript result exceeds 65536 bytes');
        results.push({ name: definition.name, value: JSON.parse(output.serialized), value_type: output.value_type,
          error: null, elapsed_ms: Date.now() - started });
      } catch (error) {
        if (error instanceof WorkerError && error.code === 'custom_javascript_timeout') {
          // A CPU loop must be terminated before context cleanup; a timeout
          // invalidates the rendered observation instead of claiming success.
          await session.send('Runtime.terminateExecution').catch(() => {});
          throw error;
        }
        results.push({ name: definition.name, value: null, value_type: null,
          error: String(error.message).slice(0, 8192), elapsed_ms: Date.now() - started });
      } finally { clearTimeout(timer); }
    }
  } finally { await session.detach().catch(() => {}); }
  return results;
}

export async function findBrowserExecutable(env = process.env, platform = process.platform) {
  if (env.OPEN_SEO_BROWSER_PATH) {
    const candidate = env.OPEN_SEO_BROWSER_PATH;
    if (!path.isAbsolute(candidate) || !(await exists(candidate, platform))) {
      throw new WorkerError(503, 'browser_not_found', 'OPEN_SEO_BROWSER_PATH must name an existing absolute browser executable');
    }
    return candidate;
  }
  let candidates;
  if (platform === 'win32') {
    candidates = [env.ProgramFiles, env['ProgramFiles(x86)'], env.LOCALAPPDATA].filter(Boolean)
      .flatMap(root => [path.join(root, 'Google', 'Chrome', 'Application', 'chrome.exe'),
        path.join(root, 'Microsoft', 'Edge', 'Application', 'msedge.exe')]);
  } else if (platform === 'darwin') {
    candidates = ['/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
      '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge'];
  } else {
    candidates = ['/usr/bin/google-chrome', '/usr/bin/google-chrome-stable', '/opt/google/chrome/chrome',
      '/usr/bin/microsoft-edge', '/usr/bin/microsoft-edge-stable', '/opt/microsoft/msedge/msedge',
      '/usr/bin/chromium', '/usr/bin/chromium-browser'];
  }
  candidates.push(chromium.executablePath());
  for (const candidate of candidates) if (await exists(candidate, platform)) return candidate;
  throw new WorkerError(503, 'browser_not_found', 'Install Chrome/Edge/Chromium or set OPEN_SEO_BROWSER_PATH; no browser is downloaded automatically');
}

async function exists(candidate, platform) {
  try {
    await access(candidate, platform === 'win32' ? constants.F_OK : constants.X_OK);
    return (await stat(candidate)).isFile();
  } catch { return false; }
}

export class JobPool {
  constructor(concurrency = 4, maxQueued = 64) {
    this.concurrency = boundedInteger(concurrency, 4, 1, 4, 'concurrency');
    this.maxQueued = boundedInteger(maxQueued, 64, 0, 64, 'max_queued');
    this.active = 0; this.queue = []; this.closed = false; this.idleWaiters = [];
  }
  run(job) {
    if (this.closed) return Promise.reject(new WorkerError(503, 'shutting_down', 'Renderer is shutting down'));
    if (this.active >= this.concurrency && this.queue.length >= this.maxQueued) {
      return Promise.reject(new WorkerError(429, 'queue_full', 'Renderer queue is full'));
    }
    return new Promise((resolve, reject) => { this.queue.push({ job, resolve, reject }); this.drain(); });
  }
  drain() {
    while (!this.closed && this.active < this.concurrency && this.queue.length) {
      const task = this.queue.shift(); this.active += 1;
      Promise.resolve().then(task.job).then(task.resolve, task.reject).finally(() => {
        this.active -= 1; this.drain(); this.notifyIdle();
      });
    }
  }
  notifyIdle() {
    if (!this.active && !this.queue.length) {
      for (const resolve of this.idleWaiters.splice(0)) resolve();
    }
  }
  waitForIdle() {
    if (!this.active && !this.queue.length) return Promise.resolve();
    return new Promise(resolve => this.idleWaiters.push(resolve));
  }
  close() {
    this.closed = true;
    for (const task of this.queue.splice(0)) task.reject(new WorkerError(503, 'shutting_down', 'Renderer is shutting down'));
    this.notifyIdle();
  }
}

class BrowserRenderer {
  constructor(env) { this.env = env; this.browserPromise = null; this.contexts = new Set(); this.closed = false; }
  async browser() {
    if (this.closed) throw new WorkerError(503, 'shutting_down', 'Renderer is shutting down');
    if (!this.browserPromise) {
      const launching = (async () => {
        const executablePath = await findBrowserExecutable(this.env);
        const browser = await chromium.launch({ executablePath, headless: true, timeout: 15000,
          chromiumSandbox: this.env.OPEN_SEO_BROWSER_NO_SANDBOX !== '1',
          handleSIGINT: false, handleSIGTERM: false, handleSIGHUP: false });
        browser.on('disconnected', () => { if (this.browserPromise === launching) this.browserPromise = null; });
        if (this.closed) { await browser.close(); throw new WorkerError(503, 'shutting_down', 'Renderer is shutting down'); }
        return browser;
      })();
      this.browserPromise = launching;
      launching.catch(() => { if (this.browserPromise === launching) this.browserPromise = null; });
    }
    return this.browserPromise;
  }
  async render(options, clientSignal) {
    let context;
    let deadline;
    let rejectDeadline;
    const waitController = new AbortController();
    const expired = new Promise((_, reject) => { rejectDeadline = reject; });
    const cancel = (error) => {
      waitController.abort();
      rejectDeadline(error);
      if (context) void context.close().catch(() => {});
    };
    let stopped = false;
    const onClientAbort = () => { stopped = true; cancel(new WorkerError(499, 'client_closed', 'Render client disconnected')); };
    if (clientSignal?.aborted) throw new WorkerError(499, 'client_closed', 'Render client disconnected');
    clientSignal?.addEventListener('abort', onClientAbort, { once: true });
    deadline = setTimeout(() => {
      stopped = true; cancel(new WorkerError(504, 'render_timeout', 'Rendering exceeded timeout_ms'));
    }, options.timeout_ms);
    const operation = (async () => {
      const browser = await this.browser();
      if (stopped) throw new WorkerError(504, 'render_timeout', 'Rendering stopped before context creation');
      context = await browser.newContext({ userAgent: options.user_agent, viewport: options.viewport,
        acceptDownloads: false, serviceWorkers: 'block' });
      if (stopped) { await context.close(); throw new WorkerError(504, 'render_timeout', 'Rendering stopped during context creation'); }
      this.contexts.add(context);
      context.on('close', () => waitController.abort());
      context.setDefaultTimeout(options.timeout_ms);
      context.setDefaultNavigationTimeout(options.timeout_ms);
      const page = await context.newPage();
      // One inspected page per job. Popups do not inherit audit credentials.
      context.on('page', other => { if (other !== page) void other.close().catch(() => {}); });
      if (Object.keys(options.custom_headers).length) {
        const session = await context.newCDPSession(page);
        // CDP Fetch header overrides apply only to this request; Chromium
        // pauses each redirect hop again. Playwright route.continue overrides
        // would propagate to redirects, while route.fetch changes networking.
        session.on('Fetch.requestPaused', async event => {
          try {
            const params = { requestId: event.requestId };
            if (new URL(event.request.url).origin === options.origin) {
              const headers = Object.create(null);
              for (const [name, value] of Object.entries(event.request.headers)) headers[name.toLowerCase()] = String(value);
              Object.assign(headers, options.custom_headers);
              params.headers = Object.entries(headers).map(([name, value]) => ({ name, value }));
            }
            await session.send('Fetch.continueRequest', params);
          } catch { await session.send('Fetch.failRequest', { requestId: event.requestId, errorReason: 'Failed' }).catch(() => {}); }
        });
        await session.send('Fetch.enable', { patterns: [{ urlPattern: '*', requestStage: 'Request' }] });
      }
      const consoleErrors = [];
      const requests = new Map();
      let droppedEvents = 0;
      let documentResponse = null;
      page.on('pageerror', error => {
        if (consoleErrors.length < 1000) consoleErrors.push({ type: 'pageerror', message: error.message });
      });
      page.on('console', message => {
        if (message.type() === 'error' && consoleErrors.length < 1000) {
          consoleErrors.push({ type: 'console', message: message.text(), location: message.location() });
        }
      });
      page.on('request', request => {
        if (requests.size >= MAX_EVENTS) { droppedEvents += 1; return; }
        requests.set(request, { url: request.url(), method: request.method(), resource_type: request.resourceType(), status: null, error: null });
      });
      page.on('response', response => {
        const request = response.request();
        const record = requests.get(request); if (record) record.status = response.status();
        if (request.isNavigationRequest() && response.frame() === page.mainFrame()) documentResponse = response;
      });
      page.on('requestfailed', request => { const record = requests.get(request); if (record) record.error = request.failure()?.errorText ?? 'Request failed'; });
      await page.goto(options.url, { waitUntil: 'domcontentloaded', timeout: options.timeout_ms });
      if (options.wait_ms) await delay(options.wait_ms, undefined, { signal: waitController.signal });
      const customJavascriptResults = await evaluateCustomJavascript(page, options.custom_javascript);
      const finalUrl = page.url();
      if (!['http:', 'https:'].includes(new URL(finalUrl).protocol)) {
        throw new WorkerError(502, 'unsupported_navigation', 'Rendered page left HTTP(S) URL space');
      }
      const html = await page.content();
      if (Buffer.byteLength(html) > MAX_HTML_BYTES) throw new WorkerError(413, 'html_too_large', 'Rendered HTML exceeds 20 MiB');
      const result = { html, final_url: finalUrl,
        status: documentResponse?.status() ?? null,
        headers: documentResponse ? await documentResponse.allHeaders() : {},
        title: await page.title(), console_errors: consoleErrors,
        network_requests: [...requests.values()], network_events_dropped: droppedEvents,
        rendered_at: new Date().toISOString(), browser_version: browser.version(),
        viewport: options.viewport, render_wait_ms: options.wait_ms,
        custom_javascript_results: customJavascriptResults,
        browser_cookies: options.collect_cookies ? (await context.cookies()).map(({ value, ...cookie }) => ({ ...cookie, value_length: Buffer.byteLength(value) })) : [] };
      if (options.screenshot) {
        result.screenshot_base64 = (await page.screenshot({ type: 'png', fullPage: false, timeout: options.timeout_ms })).toString('base64');
        result.screenshot_mime_type = 'image/png';
      }
      if (options.accessibility) {
        try {
          await page.evaluate(axe.source);
          const audited = await page.evaluate(async () => {
            const results = await window.axe.run(document);
            return { engine: 'axe-core', version: results.testEngine.version,
              violations: results.violations, incomplete: results.incomplete,
              passes_count: results.passes.length, inapplicable_count: results.inapplicable.length,
              automated_only: true };
          });
          result.accessibility = audited;
        } catch (error) {
          result.accessibility = { engine: 'axe-core', version: axe.version, automated_only: true,
            violations: null, incomplete: null, error: String(error.message) };
        }
      }
      return result;
    })();
    try { return await Promise.race([operation, expired]); }
    finally {
      clearTimeout(deadline); clientSignal?.removeEventListener('abort', onClientAbort);
      waitController.abort();
      if (context) { this.contexts.delete(context); await context.close().catch(() => {}); }
    }
  }
  async close() {
    this.closed = true;
    await Promise.allSettled([...this.contexts].map(context => context.close()));
    const browser = await this.browserPromise?.catch(() => null);
    if (browser) await browser.close().catch(() => {});
    this.browserPromise = null;
  }
}

async function readJson(request) {
  const type = (request.headers['content-type'] ?? '').split(';')[0].trim().toLowerCase();
  if (type !== 'application/json') throw new WorkerError(415, 'json_required', 'Use Content-Type: application/json');
  let size = 0; const chunks = [];
  for await (const chunk of request) {
    size += chunk.length;
    if (size > MAX_BODY_BYTES) throw new WorkerError(413, 'request_too_large', 'Request JSON exceeds 1 MiB');
    chunks.push(chunk);
  }
  try { return JSON.parse(Buffer.concat(chunks).toString('utf8')); }
  catch { throw new WorkerError(400, 'invalid_json', 'Request body is not valid JSON'); }
}

function send(response, status, value) {
  if (response.destroyed || response.writableEnded) return;
  const body = JSON.stringify(value);
  response.writeHead(status, { 'content-type': 'application/json; charset=utf-8',
    'content-length': Buffer.byteLength(body), 'cache-control': 'no-store', 'x-content-type-options': 'nosniff' });
  response.end(body);
}

function allowedHost(request) {
  try {
    const host = new URL(`http://${request.headers.host}`);
    return ['127.0.0.1', 'localhost', '[::1]'].includes(host.hostname)
      && Number(host.port || 80) === request.socket.localPort && !host.username && !host.password;
  } catch { return false; }
}

export function createRendererServer({ env = process.env, concurrency = 4, maxQueued = 64 } = {}) {
  const pool = new JobPool(concurrency, maxQueued);
  const renderer = new BrowserRenderer(env);
  let closing = false;
  const server = http.createServer(async (request, response) => {
    const controller = new AbortController();
    response.on('close', () => { if (!response.writableEnded) controller.abort(); });
    try {
      if (closing) throw new WorkerError(503, 'shutting_down', 'Renderer is shutting down');
      if (!allowedHost(request)) throw new WorkerError(403, 'invalid_host', 'Only loopback Host headers are accepted');
      if (request.headers.origin) {
        const ownOrigin = `http://${request.headers.host}`;
        if (request.headers.origin !== ownOrigin) throw new WorkerError(403, 'foreign_origin', 'Cross-origin browser requests are not accepted; use the local crawl server');
      }
      if (request.method === 'GET' && request.url === '/health') {
        let browser;
        try { browser = { available: true, executable: await findBrowserExecutable(env) }; }
        catch (error) { browser = { available: false, error: error.code ?? 'browser_not_found' }; }
        send(response, 200, { status: 'ok', worker: 'open-seo-spider-renderer',
          active: pool.active, queued: pool.queue.length, max_concurrency: pool.concurrency, browser }); return;
      }
      if (request.method === 'POST' && request.url === '/shutdown') {
        send(response, 202, { status: 'shutting_down' });
        setImmediate(() => { void close().catch(error => console.error(`Renderer shutdown failed: ${error.message}`)); });
        return;
      }
      if (request.method !== 'POST' || request.url !== '/render') {
        throw new WorkerError(404, 'not_found', 'Use POST /render, GET /health or POST /shutdown');
      }
      const options = validateRenderRequest(await readJson(request));
      const result = await pool.run(() => renderer.render(options, controller.signal));
      send(response, 200, result);
    } catch (error) {
      const known = error instanceof WorkerError;
      send(response, known ? error.status : 502, { error: { code: known ? error.code : 'render_failed',
        message: known ? error.message : String(error.message) } });
    }
  });
  server.requestTimeout = 10000;
  server.headersTimeout = 10000;
  server.keepAliveTimeout = 1000;
  const listen = (port = 8790) => new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', () => { server.removeListener('error', reject); resolve(server.address()); });
  });
  let closePromise;
  const close = () => closePromise ??= (async () => {
    closing = true; pool.close();
    const closedServer = new Promise(resolve => server.close(resolve));
    await Promise.race([pool.waitForIdle(), delay(5000, undefined, { ref: false })]);
    await renderer.close(); server.closeAllConnections(); await closedServer;
  })();
  return { server, pool, listen, close };
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const port = boundedInteger(Number(process.env.OPEN_SEO_RENDERER_PORT ?? 8790), 8790, 1, 65535, 'OPEN_SEO_RENDERER_PORT');
  const concurrency = boundedInteger(Number(process.env.OPEN_SEO_RENDERER_CONCURRENCY ?? 4), 4, 1, 4, 'OPEN_SEO_RENDERER_CONCURRENCY');
  const worker = createRendererServer({ concurrency });
  await worker.listen(port);
  console.log(`Open SEO Spider renderer listening on http://127.0.0.1:${port}`);
  for (const signal of ['SIGINT', 'SIGTERM']) {
    process.once(signal, () => { void worker.close().then(() => { process.exitCode = 0; }); });
  }
}
