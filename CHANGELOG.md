# Changelog

## Unreleased — initial public alpha

Independent Rust implementation with an original local web interface, CLI and MCP workflows.

- Native crawl/discovery, robots and SEO observations; list imports and configurable extraction.
- Durable crawl state, pause/resume, selected recrawl, saved comparison and exports.
- Sitemap workflows, resource observations, saved content cohorts, segments and n-gram reports.
- Native custom-CA/HSTS controls; active combinations with browser rendering remain refused.
- Optional Chromium rendering, screenshots, accessibility observations and custom JavaScript.
- 32 standard MCP tools and five explicitly enabled local file tools.
- Opt-in local LanguageTool analysis of selected saved HTML text, with dictionary/provenance handling and explicit unknown states.

Focused saved-spelling evidence includes 34 default Rust tests, 40 extracted UI VM groups across related workflows, and two real app-consumer local-engine tests. These are bounded fixtures, not full spelling/grammar or product parity.

### Verification still open

The final Windows-local source run passed 1,722 workspace tests with zero failures and zero ignored tests, including four real-renderer integration tests. All-target/all-feature strict Clippy passed. Seven updated legacy CI scripts have source review and still await full CI execution. Fresh Source27/28 native front-door checks remain pending. Fresh public Linux CI, Docker runtime and broader platform acceptance are not proven. A prior Windows native pilot remains separate from the newer repository source; publishing source does not upgrade that installation.

Complete authenticated sessions, OCR, scheduling, broad provider integrations and full feature/performance parity remain incomplete or unproven. Track progress in the [feature ledger](docs/feature-parity.json) and [to-do list](docs/FEATURE_TODO.md).
