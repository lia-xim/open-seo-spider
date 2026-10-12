# Optional Chromium rendering worker

This is an original Open SEO Spider worker using the public Playwright and axe-core APIs. It renders a URL in an installed Chrome, Edge or Chromium browser and returns the resulting DOM plus separate browser observations. It does not use Screaming Frog binaries, source code, assets, profiles or licence data.

The HTTP crawler can use this worker as an optional service. Starting the worker alone does not demonstrate crawler integration or feature parity with another application.

## Run locally

Node.js 22 or newer and an installed Chromium browser are required. From this directory:

```powershell
npm ci --ignore-scripts
npm start
```

The service binds **127.0.0.1:8790**. It does not listen on the LAN or download a browser. Browser discovery checks installed Chrome/Edge locations on Windows and macOS and common Chrome/Edge/Chromium paths on Linux, then an already installed Playwright Chromium executable. To choose an executable explicitly:

```powershell
$env:OPEN_SEO_BROWSER_PATH = 'C:\Program Files\Google\Chrome\Application\chrome.exe'
npm start
```

Other settings:

| Environment variable | Default | Meaning |
| --- | --- | --- |
| `OPEN_SEO_RENDERER_PORT` | `8790` | Loopback port, 1–65535 |
| `OPEN_SEO_RENDERER_CONCURRENCY` | `4` | Concurrent rendering jobs, 1–4 |
| `OPEN_SEO_BROWSER_PATH` | autodetect | Absolute path to an existing browser executable |
| `OPEN_SEO_BROWSER_NO_SANDBOX` | unset | Setting `1` explicitly disables the Chromium sandbox for a container that requires this |

The browser starts lazily on the first render request and is reused. Every job creates a separate nonpersistent context, closes it afterwards, and blocks service workers. Cookies and local storage do not carry between jobs. Popup pages are closed; the returned document is the inspected page. The worker never attaches to a user's daily browser profile. These contexts use Playwright's [documented isolation API](https://playwright.dev/docs/api/class-browser#browser-new-context).

## HTTP interface

`GET /health` returns `{status:"ok", worker, active, queued, max_concurrency, browser}`. `browser.available` only confirms that an executable was found; it does not launch the browser or prove successful rendering.

`POST /render` requires `Content-Type: application/json`. Example body:

```json
{
  "url": "https://example.com/",
  "user_agent": "OpenSEOSpider/0.1",
  "timeout_ms": 30000,
  "wait_ms": 1000,
  "custom_headers": {},
  "screenshot": false,
  "accessibility": false,
  "collect_cookies": false,
  "custom_javascript": [{"name":"document_title","script":"return document.title;"}],
  "viewport": { "width": 1280, "height": 800 }
}
```

Only HTTP(S) URLs without embedded username/password are accepted. `timeout_ms` is 500–120000; `wait_ms` is 0–30000 and must be less than the timeout. The deadline covers the rendering job, including browser/context startup, navigation, waiting, DOM collection, optional screenshot and axe run. Queued time is separate. Viewport dimensions are 100–4096 pixels. Request JSON is limited to 1 MiB; rendered HTML to 20 MiB. At most 4 jobs run simultaneously, with at most 64 waiting jobs.

The result has the following fields:

| Field | Type / meaning |
| --- | --- |
| `html` | Serialized DOM after navigation and configured wait |
| `final_url` | Final browser URL after HTTP or JavaScript navigation |
| `status` | Observed final main-document HTTP status, or `null` when unavailable |
| `headers` | String-to-string map of observed main-document response headers |
| `title` | Browser document title |
| `console_errors` | Console error / uncaught page-error objects |
| `network_requests` | Objects with `url`, `method`, `resource_type`, `status` and `error` |
| `network_events_dropped` | Count beyond the 10000-request observation cap |
| `rendered_at` | Worker UTC ISO timestamp |
| `browser_version` | Actual launched browser version |
| `viewport`, `render_wait_ms` | Applied capture settings |
| `screenshot_base64` | Optional viewport PNG encoded in base64 |
| `screenshot_mime_type` | `image/png` when screenshot requested |
| `accessibility` | Optional axe results described below |
| `custom_javascript_results` | Per-script name, JSON value, value_type, error and elapsed_ms |
| `browser_cookies` | Optional redacted cookie attributes, including value_length; never cookie values |

An HTTP 404 or 500 from the website remains a successful worker response with that website status in `status`. Navigation waits for `DOMContentLoaded` and then the requested delay; it does not claim all delayed or user-triggered content has loaded. This follows the [documented navigation behavior](https://playwright.dev/docs/api/class-page#page-goto). Network entries are observations during that interval, not a complete HAR or resource-body archive.

`POST /shutdown` returns HTTP 202 with `{status:"shutting_down"}`, stops accepting jobs, rejects queued work, allows active work up to five seconds, then closes job contexts and Chromium. The standalone Node process exits naturally after its resources close. SIGINT and SIGTERM use the same shutdown path. The shutdown endpoint applies the same Host/Origin checks as render requests.

Errors use `{error:{code,message}}`: invalid requests return 400, unsupported media type 415, size limits 413, queue capacity 429, missing browser/shutdown 503, deadline 504, and other browser failures 502. Browser callers with a foreign `Origin` and non-loopback `Host` are rejected with 403. The Rust service should call this endpoint locally; no broad browser CORS policy is enabled.

## Headers and credentials

`custom_headers` accepts at most 100 case-insensitively unique header names and 32768 combined bytes. Names must be HTTP tokens and values cannot contain control characters. Transport-controlled headers such as Host, Cookie, Content-Length, connection/proxy headers and `Sec-*` are rejected.

Custom headers are applied to main-page requests only when their origin exactly matches the initial requested URL's **scheme, host and port**. They are never configured as global context headers. Chromium DevTools `Fetch.continueRequest` checks each redirect hop separately; its overrides do not carry to a redirected request. This preserves native Chromium networking while keeping credentials off a second origin. The behavior is specified in the [official CDP protocol](https://github.com/ChromeDevTools/devtools-protocol/blob/master/json/browser_protocol.json), and the two-origin fixture proves subresource and redirect isolation. Dedicated workers, popups and independent browser targets are not authenticated with these custom headers.

Browser certificate, CSP, cross-origin and local-network protections remain enabled. The default Chromium sandbox remains enabled. No HTTP authentication-session persistence, cookie import, interactive login or browser extension loading is implemented here.

## Custom JavaScript and cookie observations

`custom_javascript` accepts up to32 uniquely named function bodies. Scripts run in the inspected page after the configured wait, before DOM collection, screenshots and axe. `await` is supported. Each return value must be JSON-serializable; individual script exceptions, circular values and oversized results produce an error row while subsequent scripts continue. Names are limited to128 UTF-8 bytes, each body to65536 bytes and all bodies to262144 bytes. Each JSON result is limited to65536 bytes.

Each script has a two-second deadline. The worker terminates browser execution if a script hangs, including a synchronous infinite loop. The rendered observation then fails explicitly; the Rust crawler retains its native HTTP observation. Scripts can change the inspected DOM or navigate and their effects are captured afterwards. This is a separate original execution contract, not a claim of Screaming Frog's script helper compatibility.

`collect_cookies:true` captures the current isolated context's cookie name, domain, path, expiry, Secure, HttpOnly and SameSite attributes plus the byte length of its value. The value itself is omitted. Capture is disabled by default. Each URL has a fresh context, so these observations do not represent a shared crawl login session or cookies from the user's browser.

## Accessibility

With `accessibility:true`, the worker injects the installed open-source axe-core engine and calls `axe.run(document)`. `accessibility` contains `engine`, `version`, `violations`, `incomplete`, `passes_count`, `inapplicable_count` and `automated_only:true`. Failures include `error` and null results rather than an empty pass report. These are automated page observations; they are not a complete accessibility compliance assessment. See the [axe-core API](https://github.com/dequelabs/axe-core/blob/develop/doc/API.md).

## Verified acceptance

```powershell
npm test
```

On 2026-10-12, Node 24.15.0 with the installed Windows Chrome passed **15 tests, 0 failures, 0 skips**. Tests use owned temporary loopback HTTP fixtures and a real isolated headless browser:

- JavaScript changes the raw title and inserts a previously absent crawlable link; the worker observes both.
- Console errors, HTTP request status, custom headers, user agent and valid PNG bytes are captured.
- A second job has fresh cookies and local storage.
- Custom async JavaScript captures values, retains individual exceptions and bounds circular or oversized results.
- Opt-in cookie properties omit values and do not carry into the next context.
- A synchronous infinite script loop is terminated and a subsequent job succeeds.
- A second origin receives external scripts and redirects without Authorization or the private test header; same-origin redirect requests retain the headers.
- axe identifies an unnamed button and image with missing alt text.
- A website 404 remains a recorded observation.
- A slow navigation returns a timeout, frees capacity and allows the next job.
- Bounded concurrency/queue and invalid-input/foreign-origin checks hold.
- HTTP shutdown responds, closes the worker/browser, and rejects new jobs.

These checks prove this worker's acceptance behavior on the test machine. Cross-platform execution, large-crawl performance and integration with the Rust crawler require their own evidence.

## Dependencies

The lockfile pins `playwright-core` 1.64.0 and `axe-core` 4.14.0. Installations use `--ignore-scripts`; `playwright-core` does not provision a browser. Upstream dependencies retain their respective Apache-2.0 and MPL-2.0 licences. Open SEO Spider's worker implementation is original project code.
