# Security

Open SEO Spider is experimental local software. There is no supported public multi-user service or guaranteed security-response time. Use the latest reviewed source and inspect the limitations for the exact revision you run.

## Report a vulnerability

Use [GitHub private vulnerability reporting](https://github.com/lia-xim/open-seo-spider/security/advisories/new). Include the affected revision, a synthetic reproduction, impact and relevant sanitized logs. Do not send credentials, session cookies, private crawl exports or proprietary artifacts.

If private reporting is unavailable, open a minimal issue requesting a private reporting channel without publishing exploit steps or sensitive data. Maintainers must enable private reporting for the repository.

## Deployment boundary

The app and optional renderer bind to loopback by default. The API/MCP service has no built-in user authentication. Do not expose it directly to the internet or an untrusted LAN. A reverse proxy alone does not add an application authorization model.

A crawler makes requests on your machine's network authority. Crawl input, redirects, discovered resources and browser scripts may reach services your machine can access, including private-network services. The application is not an SSRF isolation boundary. Restrict target scope, outbound network access and OS permissions for your deployment, especially when someone else supplies crawl input.

Custom JavaScript runs in a browser context against the target page. Treat supplied scripts, page content, imported files and saved reports as untrusted. Browser-context isolation does not replace an OS or network sandbox; avoid sensitive browser profiles and unnecessary local permissions.

MCP clients can trigger the capabilities you enable. Optional file tools should receive the narrowest intended base directory; that setting is not an OS sandbox. Protect crawl stores and exports, which can contain sensitive URLs, page text and metadata.

TLS trust additions and local engine endpoints are operator configuration. Saved spelling text is sent only after explicit acknowledgement to the configured local LanguageTool endpoint. Install and manage that separate service yourself; do not assume the crawler starts it or authenticates its operator.

Native pilots, unit tests and static reviews cover particular fixtures. They do not prove safe operation on arbitrary hostile pages or an externally exposed deployment.
