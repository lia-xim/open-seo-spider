// Proposal-only regression source. The author does not execute this file.
// Root runs it against an explicitly selected current source tree. The DOM and
// request recorder below prove function-level serialization, not browser/network IO.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import vm from 'node:vm';

const args = process.argv.slice(2);
assert.ok(args.length === 0 || args.length === 2 && args[0] === '--source-root', 'Usage: node resource-quota-ui-regressions.mjs [--source-root ABSOLUTE_SOURCE_DIRECTORY]');
const sourceRoot = args.length ? args[1] : path.join(path.dirname(fileURLToPath(import.meta.url)), 'source');
assert.ok(path.isAbsolute(sourceRoot), 'An absolute source root is required');
const app = readFileSync(path.join(sourceRoot, 'web/app.js'), 'utf8');
const engine = 'open-seo-spider/resource-contract-v2';
const plain = value => JSON.parse(JSON.stringify(value));
function section(from, to) {
  const first = app.indexOf(from), last = app.indexOf(to, first + from.length);
  assert.ok(first >= 0 && last > first, `Actual source section missing: ${from}`);
  assert.equal(app.indexOf(from, first + from.length), -1, `Ambiguous actual source section: ${from}`);
  return app.slice(first, last);
}
const actualSections = [
  section('  async function startCrawl(event) {', '  async function controlCrawl('),
  section('  function openSettings() {', '  const crawlQuotaSettingKeys='),
  section('  const crawlQuotaSettingKeys=', '  function parseAdvanced(settings) {'),
  section('  function parseAdvanced(settings) {', '  // B12 draft:'),
  section('  function syncResourceContractSettings() {', '  function resourceSavedContractLabel() {'),
  section('  function parsePositionRules(text) {', '  function listOptions() {'),
];

class Element {
  constructor() { this.type = 'text'; this._value = ''; this.checked = false; this.disabled = false; this.hidden = false; this.open = false; this.closeCalls = 0; this.children = []; this.dataset = {}; this.attrs = new Set(); this.textContent = ''; }
  get value() { return this._value; }
  set value(value) { this._value = String(value ?? ''); }
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) { this.children = [...children]; }
  setAttribute(name) { this.attrs.add(name); }
  addEventListener() {}
  scrollIntoView() {}
  showModal() { this.open = true; }
  close() { this.open = false; this.closeCalls++; }
  querySelectorAll(selector) {
    const match = node => selector === '[data-quota-rule]' ? Object.hasOwn(node.dataset, 'quotaRule') : /^\[data-[\w-]+\]$/.test(selector) && node.attrs.has(selector.slice(1, -1));
    const result = [];
    const visit = node => { for (const child of node.children) { if (match(child)) result.push(child); visit(child); } };
    visit(this); return result;
  }
  querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
}

function resourceConfig() {
  return {
    contract: { engine, follow_internal_qualified_links: false, follow_external_qualified_links: false },
    resources: { css: { store: false, fetch: true }, javascript: { store: true, fetch: false }, images: { store: true, fetch: true }, media: { store: false, fetch: false }, pdf: { store: false, fetch: true }, swf: { store: true, fetch: false }, other: { store: true, fetch: true } },
    navigation: { internal_anchors: { store: true, fetch: true, expand: true }, external_anchors: { store: true, fetch: false, expand: false } },
    css: { stylesheets: true, style_elements: true, style_attributes: false }, resource_scope: { external_origins: [] },
    follow_rel_nofollow: false, follow_document_nofollow: false,
  };
}
const rules = [{ id: 'assets', pattern: '(?i)\\.(css|js)$', max_urls: '2' }];
function settings() {
  return { concurrency: 2, timeout_seconds: 20, max_pages: 12, max_depth: 3, user_agent: 'Synthetic UI fixture', indexability_agent: 'googlebot', obey_robots: true, follow_subdomains: false, delay_ms: 0, max_body_mb: 5, check_external: false,
    javascript_rendering: false, renderer_endpoint: 'http://127.0.0.1:8790/render', renderer_wait_ms: 50, renderer_accessibility: false, renderer_screenshot: false, renderer_collect_cookies: true,
    include_patterns_text: ' /public/\n/public/\n', exclude_patterns_text: ' /private/\r\n\\.(?:png|pdf)$\n/private/\n',
    custom_extractions_json: '[]', custom_javascript_json: '[]', audit_thresholds_json: '{}', pdf_analysis_json: '{}', link_position_rules_json: '[]', validate_fragments: false,
    crawl_policy_enabled: false, crawl_policy_json: '{}', live_policy_limits_json: '{}', content_capture_enabled: false, content_capture_json: '{}',
    resource_discovery_enabled: true, resource_discovery_json: JSON.stringify(resourceConfig()), resource_discovery_limits_json: '{"max_source_candidates":30,"max_ledger_bytes":null}',
    crawl_quotas_enabled: true, crawl_quota_per_depth: '0', crawl_quota_per_subdomain: '12', crawl_quota_patterns_json: JSON.stringify(rules),
    crawl_admission_limits_enabled: false, sitemap_source_processing_enabled: false, sitemap_membership_limits_enabled: false, sitemap_source_archive_enabled: false,
    cdn_classification_enabled: false, cdn_directory_reporting_enabled: false };
}

function harness(initial = settings(), stored = null) {
  const nodes = new Map(), fields = new Map(), writes = [], requests = [], messages = [], storage = new Map();
  if (stored != null) storage.set('open-seo-spider-settings', stored);
  const node = id => { if (!nodes.has(id)) nodes.set(id, new Element()); return nodes.get(id); };
  const field = name => { if (!fields.has(name)) { const e = new Element(); if (typeof initial[name] === 'boolean' || name.endsWith('_enabled')) e.type = 'checkbox'; fields.set(name, e); } return fields.get(name); };
  node('settings-form').elements = new Proxy({}, { get: (_, name) => name === 'namedItem' ? field : field(String(name)) });
  node('crawl-mode').value = 'spider'; node('seed-url').value = 'https://synthetic.test/public/';
  const state = { settings: plain(initial), generation: 1, operationSequence: 0, crawl: null, crawls: [], resources: {}, section: 'internal' };
  const noOp = () => {};
  const context = vm.createContext({ URL, TextEncoder, AbortController, console, state, $: node,
    document: { createElement: () => new Element(), createDocumentFragment: () => ({ querySelector: () => null }), getElementById: node },
    localStorage: { getItem: key => storage.get(key) ?? null, setItem: (key, value) => { storage.set(key, value); writes.push({ key, value }); } },
    window: {}, resourceContractEngine: engine,
    invalidatePolicyPreview: noOp, invalidateContentRequests: noOp, renderControls: noOp, renderContent: noOp, contentSection: () => false,
    // Unrelated CDN controls are deliberately disabled. Resource/quota/scope,
    // persistence, renderer validation and request serialization are actual code.
    readCdnDirectoryReportingSettings: () => ({}), readCdnClassificationSettings: () => ({}),
    restoreCdnDirectoryReportingSettings: () => ({}), restoreCdnClassificationSettings: () => ({}),
    active: () => false, actionBusy: () => false, message: (text, error) => messages.push({ text, error }),
    loadHistory: async () => {}, renderHistory: noOp, selectCrawl: async () => {},
    request: async (url, options) => { requests.push({ url, method: options.method, body: JSON.parse(options.body) }); return { id: 'synthetic-recorded-request' }; },
  });
  vm.runInContext(actualSections.join('\n') + '\nglobalThis.api={open:openSettings,save:saveSettings,restore:restoreSettings,parse:parseAdvanced,quotas:parseCrawlQuotas,start:startCrawl,current:useCurrentResourceContract};', context, { timeout: 1000, filename: 'actual-app-settings-sections.js' });
  context.api.open();
  return { api: context.api, state, field, node, writes, requests, messages, storage, setResource: config => { field('resource_discovery_json').value = JSON.stringify(config); } };
}
const event = { preventDefault() {} };
function rejectedSave(mutator) {
  const h = harness(), before = plain(h.state.settings);
  mutator(h); h.api.save(event);
  assert.equal(h.node('settings-dialog').open, true); assert.equal(h.node('settings-dialog').closeCalls, 0);
  assert.equal(h.node('settings-error').hidden, false); assert.ok(h.node('settings-error').textContent.length > 0);
  assert.equal(h.writes.length, 0); assert.equal(h.requests.length, 0); assert.deepEqual(h.state.settings, before);
}

test('resource_quota_visible_settings_current_action_saves_reopens_and_submits_exact_native_payload', async () => {
  const h = harness(), legacy = resourceConfig(); delete legacy.contract;
  h.setResource(legacy); h.api.current();
  assert.deepEqual(JSON.parse(h.field('resource_discovery_json').value), resourceConfig());
  h.api.save(event);
  assert.equal(h.node('settings-dialog').open, false); assert.equal(h.node('settings-dialog').closeCalls, 1); assert.equal(h.writes.length, 1);
  const saved = h.storage.get('open-seo-spider-settings');
  const reopened = harness(settings(), saved); reopened.api.restore(); reopened.api.open();
  assert.equal(reopened.field('exclude_patterns_text').value, settings().exclude_patterns_text);
  assert.equal(reopened.field('include_patterns_text').value, settings().include_patterns_text);
  assert.deepEqual(JSON.parse(reopened.field('resource_discovery_json').value), resourceConfig());
  assert.equal(reopened.field('renderer_collect_cookies').checked, true);
  await reopened.api.start(event);
  assert.equal(reopened.requests.length, 1, JSON.stringify(reopened.messages));
  const capture = reopened.requests[0], payload = capture.body;
  assert.equal(capture.url, '/api/crawls'); assert.equal(capture.method, 'POST');
  assert.deepEqual(payload.resource_discovery, resourceConfig());
  assert.deepEqual(payload.resource_discovery_limits, { max_source_candidates: 30, max_ledger_bytes: null });
  assert.deepEqual(payload.crawl_quotas, { version: 1, url_pattern_quotas: [{ ...rules[0], max_urls: 2 }], max_urls_per_depth: 0, max_urls_per_subdomain: 12 });
  assert.deepEqual(payload.include_patterns, ['/public/']); assert.deepEqual(payload.exclude_patterns, ['/private/', '\\.(?:png|pdf)$']);
  assert.equal(payload.javascript_rendering, false); assert.equal(payload.renderer, null); assert.equal(payload.renderer_collect_cookies, true);
  assert.equal(payload.include_subdomains, false); assert.equal(payload.check_external, false);
  for (const key of ['resource_discovery_enabled', 'resource_discovery_json', 'resource_discovery_limits_json', 'crawl_quotas_enabled', 'crawl_quota_per_depth', 'crawl_quota_per_subdomain', 'crawl_quota_patterns_json']) assert.equal(Object.hasOwn(payload, key), false, key);
  // This form has no HTTP credential editor. The repair must not synthesize one.
  for (const key of ['basic_auth', 'authorization', 'request_headers', 'cookies']) assert.equal(Object.hasOwn(payload, key), false, key);
});

test('resource_quota_default_current_engine_and_each_store_fetch_product_remain_literal', () => {
  for (const store of [false, true]) for (const fetch of [false, true]) {
    const h = harness(), config = resourceConfig(); config.contract = {};
    for (const rule of Object.values(config.resources)) Object.assign(rule, { store, fetch });
    h.setResource(config); h.api.save(event);
    assert.equal(h.node('settings-dialog').open, false, h.node('settings-error').textContent);
    const parsed = plain(h.api.parse(h.state.settings));
    assert.deepEqual(parsed.resource_discovery, config); assert.equal(parsed.crawl_quotas.max_urls_per_depth, 0);
    assert.deepEqual(parsed.exclude_patterns, ['/private/', '\\.(?:png|pdf)$']);
  }
});

test('resource_quota_save_refuses_renderer_legacy_unknown_subdomain_external_and_origin_profiles', () => {
  for (const mutate of [
    h => { h.field('javascript_rendering').checked = true; },
    h => { const c = resourceConfig(); delete c.contract; h.setResource(c); },
    h => { const c = resourceConfig(); c.contract.engine = 'unsupported'; h.setResource(c); },
    h => { h.field('follow_subdomains').checked = true; },
    h => { h.field('check_external').checked = true; },
    h => { const c = resourceConfig(); c.resource_scope.external_origins = ['https://external.synthetic.test']; h.setResource(c); },
  ]) rejectedSave(mutate);
});

test('resource_quota_existing_pattern_resource_rule_and_limit_refusals_keep_state_unsaved', () => {
  for (const mutate of [
    h => { h.api.open(); const rows = h.node('crawl-quota-patterns').querySelectorAll('[data-quota-rule]'); rows[0].querySelector('[data-quota-max]').value = '-1'; },
    h => { const c = resourceConfig(); c.resources.css.fetch = 'true'; h.setResource(c); },
    h => { const c = resourceConfig(); c.css.unknown = true; h.setResource(c); },
    h => { const c = resourceConfig(); c.contract.follow_internal_qualified_links = 'true'; h.setResource(c); },
    h => { const c = resourceConfig(); c.follow_rel_nofollow = true; h.setResource(c); },
    h => { const c = resourceConfig(); c.resource_scope.external_origins = ['https://external.synthetic.test/path']; h.setResource(c); },
    h => { h.field('resource_discovery_limits_json').value = '{"max_source_candidates":0}'; },
    h => { h.node('crawl-quota-patterns').querySelector('[data-quota-pattern]').value = 'x'.repeat(4097); },
    h => { h.field('crawl_quota_per_depth').value = ''; h.field('crawl_quota_per_subdomain').value = ''; h.node('crawl-quota-patterns').replaceChildren(); },
  ]) rejectedSave(mutate);
  const h = harness(), bad = settings(); bad.crawl_quota_patterns_json = JSON.stringify([...rules, rules[0]]);
  assert.throws(() => h.api.quotas(bad));
});

test('resource_quota_disabled_features_preserve_resource_only_and_page_quota_profiles', () => {
  const resourceOnly = harness(), legacy = resourceConfig(); delete legacy.contract; legacy.resource_scope.external_origins = ['https://external.synthetic.test'];
  resourceOnly.setResource(legacy); resourceOnly.field('crawl_quotas_enabled').checked = false;
  resourceOnly.field('follow_subdomains').checked = true; resourceOnly.field('check_external').checked = true; resourceOnly.api.save(event);
  assert.equal(resourceOnly.node('settings-dialog').open, false); assert.deepEqual(plain(resourceOnly.api.parse(resourceOnly.state.settings)).resource_discovery, legacy);
  assert.equal(Object.hasOwn(resourceOnly.api.parse(resourceOnly.state.settings), 'crawl_quotas'), false);
  const pageOnly = harness(); pageOnly.field('resource_discovery_enabled').checked = false; pageOnly.field('resource_discovery_json').value = 'malformed';
  pageOnly.field('follow_subdomains').checked = true; pageOnly.field('check_external').checked = true; pageOnly.api.save(event);
  assert.equal(pageOnly.node('settings-dialog').open, false); assert.equal(Object.hasOwn(pageOnly.api.parse(pageOnly.state.settings), 'resource_discovery'), false);
  assert.equal(pageOnly.api.parse(pageOnly.state.settings).crawl_quotas.max_urls_per_depth, 0);
  const off = harness(); off.field('resource_discovery_enabled').checked = false; off.field('resource_discovery_json').value = 'malformed';
  off.field('crawl_quotas_enabled').checked = false; off.field('crawl_quota_per_depth').value = '-1'; off.api.save(event);
  assert.equal(off.node('settings-dialog').open, false); assert.equal(Object.hasOwn(off.api.parse(off.state.settings), 'crawl_quotas'), false);
});

test('resource_quota_restored_unsupported_scope_refuses_start_before_creation_request', async () => {
  for (const key of ['follow_subdomains', 'check_external', 'javascript_rendering']) {
    const saved = settings(); saved[key] = true;
    const h = harness(settings(), JSON.stringify(saved)); h.api.restore(); await h.api.start(event);
    assert.equal(h.requests.length, 0, key); assert.ok(h.messages.some(row => row.error && row.text.includes('Ungültige Crawl-Einstellung')), key);
  }
  const saved = settings(), legacy = resourceConfig(); delete legacy.contract; saved.resource_discovery_json = JSON.stringify(legacy);
  const h = harness(settings(), JSON.stringify(saved)); h.api.restore(); await h.api.start(event); assert.equal(h.requests.length, 0);
});

test('resource_quota_current_action_keeps_explicit_mixed_nofollow_migration_semantics', () => {
  const h = harness(), legacy = resourceConfig(); delete legacy.contract; legacy.follow_rel_nofollow = true; legacy.follow_document_nofollow = false;
  h.setResource(legacy); h.api.current(); const converted = JSON.parse(h.field('resource_discovery_json').value);
  assert.deepEqual(converted.contract, { engine, follow_internal_qualified_links: false, follow_external_qualified_links: false });
  assert.equal(converted.follow_rel_nofollow, false); assert.equal(converted.follow_document_nofollow, false);
  assert.deepEqual(converted.resources, legacy.resources); assert.deepEqual(converted.css, legacy.css);
  h.api.save(event); assert.equal(h.node('settings-dialog').open, false, h.node('settings-error').textContent);
});
