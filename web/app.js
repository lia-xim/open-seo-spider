'use strict';

(() => {
  const $ = (id) => document.getElementById(id);
  const state = {
    resources: { result:null, token:null, revision:0, cursor:0, back:[], error:'', selected:null, sort:0, direction:1 },
    content: { inventory:null, inventoryToken:null, cursor:0, back:[], error:'', selected:new Set(), source:null, sourceToken:null, sourceError:'', result:null, analysisToken:null, analysisError:'', revision:0, resultPage:0, sort:0, direction:1, resultSort:1, resultDirection:-1 },
    recrawlURLs: new Set(), policy: null, policyError: '', policyToken: null, policyCursor: 0, policyBack: [], policySelected: null, policySort: 0, policyDirection: 1,
    crawl: null, crawls: [], pages: [], issues: [], section: 'overview',
    search: '', filter: 'all', sort: 'url', direction: 1, page: 0, pageSize: 50,
    selected: null, selectedResponse: null, selectedInput: null, responses: null, responsesError: '', fragments: null, fragmentsError: '', inspectorTab: 'details', polling: false, starting: false,
    generation: 0, pollToken: null, pollTimer: null, control: null, operationSequence: 0, startOperation: null, historySequence: 0, issueMap: new Map(), pageMap: new Map(), cache: {}, analysis: null, analysisError: '', settings: {
      concurrency: 8, timeout_seconds: 20, max_pages: 0, max_depth: 0,
      user_agent: 'Open SEO Spider/0.1', indexability_agent: 'googlebot', obey_robots: true, follow_subdomains: false,
      delay_ms: 100, max_body_mb: 5, check_external: false,
      javascript_rendering: false, renderer_endpoint: 'http://127.0.0.1:8790/render',
      renderer_wait_ms: 500, renderer_accessibility: false, renderer_screenshot: false,
      renderer_collect_cookies: false, custom_javascript_json: '[]',
      include_patterns_text: '', exclude_patterns_text: '', custom_extractions_json: '[]', audit_thresholds_json: '{}', pdf_analysis_json: '{}', validate_fragments: false, link_position_rules_json: '[]', crawl_policy_enabled: false, crawl_policy_json: '{"scope":{"roots":[{"origin":"https://example.test","folders":["/"],"subdomains":false,"exact_paths":[]}]}}', live_policy_limits_json: '{}', content_capture_enabled:false, content_capture_json:'{"sources":["raw_http_html"],"include_selectors":["body"],"exclude_selectors":["script","style","template","noscript","nav","footer"]}',resource_discovery_enabled:false,resource_discovery_json:'{}',resource_discovery_limits_json:'{}'
    }
  };
  const sections = {
    overview: ['CRAWL INTELLIGENCE', 'Übersicht', 'Ein Blick auf den technischen Zustand deiner Website.'],
    internal: ['URL INVENTAR', 'Interne URLs', 'Alle erfassten URLs im Crawl-Bereich mit ihrem tatsächlichen Abrufstatus.'],
    external: ['URL INVENTAR', 'Externe Links', 'Entdeckte Linkziele außerhalb des Crawl-Bereichs. Ungeprüfte Ziele haben keinen HTTP-Status.'],
    status: ['TECHNISCHE ANALYSE', 'HTTP & Indexierung', 'Antwortcodes, Weiterleitungen, Abrufdauer und Indexierbarkeit.'],
    responses: ['HTTP BEOBACHTUNGEN', 'Antworten', 'Jede erfasste native HTTP-Antwort einschließlich Zwischenredirects. Nicht gespeicherte Header und Body bleiben unbekannt; Dokumentdaten stehen separat.'],
    policy: ['NATIVE URL-POLICY', 'Policy-Entscheidungen', 'Gespeicherte Vorkommen und tatsächliche Zulassungsbefunde, begrenzt nach Ledger-Seite. Verweigerte Kandidaten sind keine abgerufenen Dokumente.'],
    policyaliases: ['NATIVE URL-POLICY', 'URL-Transformationen', 'Tatsächlich gespeicherte Transformationen von beobachteten Zielen zu Abrufkandidaten. Kein HTTP-Redirect und kein Erfolgsnachweis für das Originalziel.'],
    content: ['GESPEICHERTE INHALTE', 'Inhaltstexte & Analyse', 'Explizite Textquellen und ausgewählte Kohorten. Die Analyse verwendet gespeicherte Texte und startet keine Website-Anfragen.'],
    'resource-discovery': ['NATIVE RESSOURCEN', 'Ressourcenermittlung', 'Quellvorkommen und Abrufentscheidungen bleiben getrennt von Zielantworten und Browserauswahl.'],
    inputs: ['EINGABEHERKUNFT', 'Eingabezeilen', 'Gespeicherte, deklarierte Importherkunft mit ursprünglicher Reihenfolge, Duplikaten und ungültigen Zeilen. Der vollständige CSV-Export ergänzt tatsächliche Abrufbefunde.'],
    titles: ['ONPAGE ANALYSE', 'Seitentitel', 'Title-Tags, Zeichenlänge und mehrfach verwendete Titel. Längen sind Hinweise.'],
    meta: ['ONPAGE ANALYSE', 'Meta Descriptions', 'Descriptions, Zeichenlänge und Duplikate im abgerufenen HTML.'],
    headings: ['ONPAGE ANALYSE', 'Überschriften', 'H1- und H2-Struktur deiner Seiten, einschließlich mehrfacher Überschriften.'],
    headingorder: ['DOM BEOBACHTUNGEN', 'Überschriftenfolge', 'H1 bis H6 in erfasster DOM-Reihenfolge. Tag-Level, Bild-Alt und ARIA-Attribute bleiben getrennt; visuelle Ebenen werden nicht berechnet.'],
    keywords: ['DOM BEOBACHTUNGEN', 'Meta Keywords', 'Einzelne erfasste Keywords-Deklarationen, mit originalem content und normalisiertem Text. Keine Aussage über Rankingwirkung.'],
    linkdom: ['DOM BEOBACHTUNGEN', 'Linkvorkommen', 'Erfasste Linkziele, Fragmente, DOM-Pfade und Positionen. DOM-Index ist keine Quellzeile oder Bildschirmkoordinate.'],
    imagecandidates: ['DOM BEOBACHTUNGEN', 'Bildkandidaten', 'Deklarationen aus img src/srcset und picture source. Browserauswahl bleibt unbekannt; ein Kandidat ist kein tatsächlich angezeigtes Bild.'],
    navigation: ['DOM BEOBACHTUNGEN', 'Weitere Navigation', 'Nicht standardmäßige href-, Router- und Rollenbeobachtungen. Scripts werden hier nicht ausgeführt und daraus keine Graphlinks erfunden.'],
    fragments: ['ZIELBEZIEHUNGEN', 'Fragmentziele', 'Optionale Prüfung gegen erfasste IDs und benannte Anker. Fehlende, unbekannte und nicht unterstützte Ziele bleiben getrennte Befunde.'],
    canonical: ['ONPAGE ANALYSE', 'Canonicals', 'Deklarierte Canonical-Ziele und die Anzahl der Canonical-Tags.'],
    robots: ['TECHNISCHE ANALYSE', 'Robots & Directives', 'Robots.txt-Ausschlüsse, Meta-Robots und X-Robots-Tag aus der HTTP-Antwort.'],
    hreflang: ['INTERNATIONALES SEO', 'Hreflang', 'Sprachalternativen aus HTML und HTTP-Link-Headern. Beobachtete Ziel- und Rückverbindungen werden nach Crawl-Abschluss geprüft.'],
    images: ['ONPAGE ANALYSE', 'Bildvorkommen', 'Alt-Texte und deklarierte HTML-Dimensionen je Quellseite. Dateimaße stammen ausschließlich aus einer erfassten Bilddatei.'],
    imagefiles: ['RESSOURCEN', 'Bilddateien', 'Tatsächlich abgerufene Bildressourcen mit Dateiformat und intrinsischen Maßen. Fehlende Messwerte bleiben unbekannt.'],
    pdf: ['RESSOURCEN', 'PDF-Dokumente', 'Erfasste PDF-Metadaten, Textoperatoren und URI-Annotationen. Textauswertung ist keine OCR oder vollständige Layoutanalyse.'],
    issues: ['PRIORISIERUNG', 'Probleme', 'Erkannte Fehler, Warnungen und Hinweise aus der tatsächlichen Crawl-Analyse.'],
    graph: ['LINK ARCHITECTURE', 'Linkgraph', 'Interne Ankerverbindungen aus den abgerufenen Seiten. Der sichtbare Graph zeigt einen begrenzten Ausschnitt.'],
    linkmetrics: ['LINK ARCHITECTURE', 'Linkmetriken', 'Eigene PageRank-artige Berechnung auf dem beobachteten Linkgraph. Kein Suchmaschinen- oder Vendor-Score.'],
    structured: ['ONPAGE ANALYSE', 'Strukturierte Daten', 'JSON-LD-Blöcke und beobachtete Typen. Schema.org- und Google-Rich-Result-Validierung werden nicht ausgeführt.'],
    extractions: ['CUSTOM DATA', 'Eigene Extraktionen', 'Benannte Auswertungen aus dem tatsächlich analysierten HTML oder DOM.'],
    javascript: ['CUSTOM DATA', 'JavaScript-Auswertungen', 'Eigene Scripts im isolierten Seitenkontext nach der Render-Wartezeit. Werte und Fehler bleiben getrennt.'],
    cookies: ['BROWSER ANALYSE', 'Cookies', 'Cookie-Eigenschaften aus einzelnen isolierten Seitenaufrufen. Aufnahme muss aktiviert werden; Werte werden ausgelassen.'],
    pagination: ['TECHNISCHE ANALYSE', 'Pagination & AMP', 'Deklarierte next-, prev- und amphtml-Ziele aus HTML und HTTP-Link-Headern.']
  };
  const labels = {
    running: 'Crawl läuft', queued: 'Crawl wartet', pending: 'Crawl wartet',
    completed: 'Crawl abgeschlossen', complete: 'Crawl abgeschlossen', finished: 'Crawl abgeschlossen',
    cancelled: 'Crawl gestoppt', canceled: 'Crawl gestoppt', failed: 'Crawl fehlgeschlagen',
    pausing: 'Pause angefordert · laufende Anfragen werden beendet', paused: 'Crawl pausiert', limited: 'Crawl-Grenze erreicht', interrupted: 'Crawl unterbrochen', cancelling: 'Crawl wird gestoppt'
  };
  const listUI = { controller: null, page: 0, acceptedText: null, snapshot: null };
  let limitResumeContext = null;
  let selectedRecrawlContext = null;
  const policyPreview = { revision: 0, token: null, result: null };
  const policySection = () => ['policy', 'policyaliases'].includes(state.section);
  const nativePolicyEngine = 'open-seo-spider/native-crawl-policy-v1';
  const policyDecisionLabel = value => ({ fetch_and_traverse: 'Abruf & Traversierung', check_only: 'Nur prüfen', deny: 'Policy verweigert' }[value] || value || 'Nicht erfasst');
  const number = (value) => Number(value || 0).toLocaleString('de-DE');
  const array = (value) => Array.isArray(value) ? value : value == null || value === '' ? [] : [value];
  const first = (value) => array(value).join(' | ');
  const primary = (value) => String(array(value)[0] ?? '');
  const canonicalCount = (page) => array(page.html_canonical).length + array(page.header_canonical).length || page.canonical.length;
  const escape = (value) => String(value ?? '').replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[character]));
  const duration = (ms) => ms == null ? '—' : `${number(Math.round(Number(ms)))} ms`;
  const effectiveStatus = (page) => page.final_status ?? page.status ?? page.status_code ?? null;
  const resourceStatus = (status) => ({ complete: 'Erfasst', partial: 'Unvollständig', error: 'Fehler', encrypted: 'Verschlüsselt', disabled: 'Deaktiviert' }[status] || 'Nicht erfasst');
  const domSections = ['headingorder', 'keywords', 'imagecandidates', 'navigation'];
  const domSource = row => row._page?.dom_metadata?.source === 'raw_html' ? 'HTML-Antwort' : row._page?.dom_metadata?.source === 'rendered_html' ? 'Gerenderter DOM' : row._page?.dom_metadata?.source || 'Nicht erfasst';
  const domPath = row => row.dom_location?.path ?? (row.dom_location?.path_truncated ? '[Pfadbegrenzung]' : 'Nicht erfasst');
  const completeDOM = page => Boolean(page?.dom_metadata?.complete && page.final_status === 200 && !page.error && !page.truncated && !page.blocked_by_robots && !page.renderer_error && (page.dom_metadata.source !== 'rendered_html' || page.rendered_status === 200));
  const fragmentStatus = value => ({ matched_id: 'ID gefunden', matched_name: 'Ankername gefunden', top: 'Dokumentanfang', missing: 'Ziel fehlt', unknown: 'Unbekannt / nicht prüfbar', unsupported: 'Nicht unterstützt' }[value] || 'Nicht erfasst');
  const mediaType = page => String(page.content_type || '').split(';', 1)[0].trim().toLowerCase();
  const isPDF = page => Boolean(page.pdf) || mediaType(page) === 'application/pdf';
  const isImageFile = page => Boolean(page.image_metadata) || mediaType(page).startsWith('image/');
  const statusBadge = (status) => status == null ? '<span class="badge neutral">—</span>' : `<span class="badge ${Number(status) >= 400 ? 'danger' : Number(status) >= 300 ? 'warning' : 'good'}">${escape(status)}</span>`;
  const activeStatus = status => ['running', 'queued', 'pending', 'pausing', 'cancelling'].includes(status);
  const active = () => state.crawl && activeStatus(state.crawl.status);
  const canResume = () => state.crawl && state.crawl.resume_available !== false && ['paused', 'limited', 'cancelled', 'canceled', 'failed', 'interrupted'].includes(state.crawl.status);
  const actionBusy = () => Boolean(state.control || state.starting);
  const recrawlOperation = () => state.crawl?.operation?.coverage === 'mixed_age_selected_recrawl' ? state.crawl.operation : null;
  const pendingRecrawlURLs = crawl => new Set(crawl?.operation?.coverage === 'mixed_age_selected_recrawl' ? array(crawl.operation.targets).filter(target => target.state === 'pending').map(target => target.requested_url) : []);
  const canRecrawl = () => Boolean(state.crawl && state.crawl.recrawl_available !== false && !active() && !actionBusy() && state.recrawlURLs.size);
  // Resource/link occurrences can contain source facts beside a different target URL.
  // Only a row whose displayed URL is itself the stored document is selectable here.
  const recrawlPage = row => !row._page || row._page.url === row.url ? state.pageMap.get(row.url) : null;
  function observationAge(url) {
    const operation = recrawlOperation();
    if (!operation) return '';
    const target = array(operation.targets).find(entry => entry.requested_url === url);
    return target?.state === 'committed' ? 'Neu geprüft' : target?.state === 'pending' ? 'Ausstehend · kein aktueller Befund' : 'Beibehalten · vorheriger Abruf';
  }
  function setRecrawlSelected(url, selected) {
    if (!state.pageMap.has(url) || active() || actionBusy() || state.crawl?.recrawl_available === false) return;
    if (selected) state.recrawlURLs.add(url); else state.recrawlURLs.delete(url);
    const restoreFocus = document.activeElement?.dataset?.recrawlUrl === url;
    renderControls();
    if (!['overview', 'graph'].includes(state.section)) renderTable();
    if (restoreFocus) [...document.querySelectorAll('[data-recrawl-url]')].find(input => input.dataset.recrawlUrl === url)?.focus({ preventScroll: true });
  }
  function invalidatePoll() {
    invalidateContentRequests('Der Crawl-Stand wurde geändert.', true);
    resetResourceLedger();
    state.generation += 1;
    state.policyToken?.controller.abort();
    state.policyToken = null;
    clearTimeout(state.pollTimer);
    state.pollTimer = null;
    state.pollToken = null;
    state.polling = false;
    return state.generation;
  }
  const normalPage = (page) => ({
    ...page,
    url: page.url || page.address || '',
    internal: page.internal !== false,
    titles: array(page.titles ?? page.title),
    meta_descriptions: array(page.meta_descriptions ?? page.meta_description),
    h1: array(page.h1), h2: array(page.h2),
    canonical: array(page.canonical ?? page.canonicals),
    meta_robots: array(page.meta_robots ?? page.robots),
    x_robots_tag: array(page.x_robots_tag),
    hreflang: array(page.hreflang), images: array(page.images), custom_extractions: array(page.custom_extractions),
    custom_javascript_results: array(page.custom_javascript_results), browser_cookies: array(page.browser_cookies),
    links: array(page.links ?? page.outlinks),
    inlinks: array(page.inlinks),
    redirect_chain: array(page.redirect_chain),
    response_ms: page.response_ms ?? page.response_time_ms ?? null,
    content_type: page.content_type || '',
    indexable: page.indexable ?? null,
    indexability_reasons: array(page.indexability_reasons)
  });
  const isHTML = page => !page.pdf && !page.image_metadata && (['text/html', 'application/xhtml+xml'].includes(mediaType(page)) || (!mediaType(page) && (page.titles.length > 0 || page.h1.length > 0)));
  const internals = () => state.pages.filter((page) => page.internal);
  const htmlPages = () => internals().filter(isHTML);
  const stats = () => state.crawl?.stats || state.crawl?.statistics || {};
  const linkURL = (link) => typeof link === 'string' ? link : link.url || link.target || link.href || '';
  const pageIssues = (url) => state.issueMap.get(url) || [];
  const humanDate = (value) => { const date = new Date(value); return Number.isFinite(date.getTime()) ? date.toLocaleString('de-DE', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) : ''; };

  async function request(path, options = {}) {
    const response = await fetch(path, { ...options, headers: { ...(options.body ? { 'Content-Type': 'application/json' } : {}), ...options.headers } });
    const type = response.headers.get('content-type') || '';
    if (!response.ok) {
      const body = type.includes('json') ? await response.json().catch(() => ({})) : { error: await response.text() };
      throw new Error(body.error || body.message || `HTTP ${response.status}`);
    }
    if (response.status === 204) return null;
    return type.includes('json') ? response.json() : response.text();
  }

  function connection(online) {
    const element = $('connection-state');
    element.className = `connection ${online ? 'connected' : 'offline'}`;
    element.innerHTML = `<i></i>${online ? 'Lokaler Server verbunden' : 'Server nicht erreichbar'}`;
  }

  function message(text, error = false) {
    const element = $('message');
    element.hidden = !text;
    element.textContent = text || '';
    element.classList.toggle('error', error);
  }

  function reindexIssues() {
    state.issueMap = new Map();
    for (const issue of state.issues) {
      const entries = state.issueMap.get(issue.url) || [];
      entries.push(issue);
      state.issueMap.set(issue.url, entries);
    }
  }

  async function loadHistory() {
    const sequence = ++state.historySequence, generation = state.generation;
    const response = await request('/api/crawls');
    if (sequence !== state.historySequence || generation !== state.generation) return;
    state.crawls = Array.isArray(response) ? response : response.crawls || response.items || [];
    connection(true);
    renderHistory();
  }

  function renderHistory() {
    const select = $('crawl-history');
    const value = state.crawl?.id || select.value;
    select.innerHTML = '<option value="">Neuer Crawl</option>';
    for (const crawl of state.crawls) {
      const option = document.createElement('option');
      option.value = crawl.id;
      const seed = crawl.seed_url || crawl.config?.seed_urls?.[0] || crawl.seed_urls?.[0] || crawl.name || crawl.id;
      let host = seed;
      try { host = new URL(seed).host; } catch (_) { /* Stored identifiers are valid labels. */ }
      option.textContent = `${host} · ${humanDate(crawl.started_at || crawl.created_at)} · ${labels[crawl.status] || crawl.status || ''}`;
      select.append(option);
    }
    select.value = value;
  }

  async function loadPages(id, generation, from = 0) {
    let offset = from;
    const result = [];
    const batchSize = 1000;
    // A bounded DOM renders at most 250 rows. All stored records remain searchable.
    while (true) {
      const response = await request(`/api/crawls/${encodeURIComponent(id)}/pages?offset=${offset}&limit=${batchSize}`);
      if (generation !== state.generation) return null;
      const pages = Array.isArray(response) ? response : response.pages || response.items || [];
      result.push(...pages.map(normalPage));
      const total = Number(response.total ?? pages.length);
      if (Array.isArray(response) || pages.length === 0 || from + result.length >= total) break;
      offset += pages.length;
      // Yield so controls remain reachable during large completed crawl loads.
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
    return result;
  }

  async function refresh(generation = state.generation) {
    if (!state.crawl || state.pollToken || state.control || generation !== state.generation) return;
    const id = state.crawl.id, token = { id, generation };
    state.pollToken = token; state.polling = true;
    const current = () => state.pollToken === token && generation === state.generation && state.crawl?.id === id;
    try {
      const [crawlResult, issueResult] = await Promise.all([
        request('/api/crawls/' + encodeURIComponent(id)),
        request('/api/crawls/' + encodeURIComponent(id) + '/issues')
      ]);
      if (!current()) return;
      const previousStatus = state.crawl.status, nextCrawl = crawlResult.crawl || crawlResult;
      const terminal = !activeStatus(nextCrawl.status);
      const from = terminal || !['running', 'queued', 'pending', 'pausing'].includes(previousStatus) ? 0 : state.pages.length;
      const pages = await loadPages(id, generation, from);
      if (!current() || !pages) return;
      let analysis = state.analysis, analysisError = state.analysisError;
      let responses = state.responses, responsesError = state.responsesError;
      let fragments = state.fragments, fragmentsError = state.fragmentsError;
      if (terminal && !analysis) {
        try { analysis = await request('/api/crawls/' + encodeURIComponent(id) + '/analysis'); analysisError = ''; }
        catch (error) { analysisError = 'Weiterführende Analyse nicht verfügbar: ' + error.message; }
        if (!current()) return;
      }
      if (terminal && responses == null) {
        try {
          const result = await request('/api/crawls/' + encodeURIComponent(id) + '/responses');
          if (!Array.isArray(result.responses)) throw new Error('Ungültiges Antworten-Inventar');
          responses = result.responses; responsesError = '';
        } catch (error) { responsesError = 'Antworten-Inventar nicht verfügbar: ' + error.message; }
        if (!current()) return;
      }
      if (terminal && fragments == null) {
        try {
          const result = await request('/api/crawls/' + encodeURIComponent(id) + '/fragments');
          if (!Array.isArray(result.observations) || typeof result.enabled !== 'boolean') throw new Error('Ungültiges Fragment-Inventar');
          fragments = result; fragmentsError = '';
        } catch (error) { fragmentsError = 'Fragment-Inventar nicht verfügbar: ' + error.message; }
        if (!current()) return;
      }
      if ((nextCrawl.generation ?? null) !== (state.crawl.generation ?? null)) resetResourceLedger();
      state.crawl = nextCrawl;
      const pendingSelected = pendingRecrawlURLs(nextCrawl);
      state.pages = (from ? [...state.pages, ...pages] : pages).filter(page => !pendingSelected.has(page.url));
      state.pageMap = new Map(state.pages.map(page => [page.url, page])); state.cache = {};
      state.analysis = analysis; state.analysisError = analysisError;
      state.responses = responses; state.responsesError = responsesError;
      state.fragments = fragments; state.fragmentsError = fragmentsError;
      state.issues = Array.isArray(issueResult) ? issueResult : issueResult.issues || issueResult.items || [];
      if (analysis) state.issues.push(...array(analysis.issues));
      state.issues = state.issues.filter(issue => !pendingSelected.has(issue.url));
      reindexIssues(); connection(true); render();
      if (policySection() && !state.policyToken) loadPolicyPage(state.policyCursor);
      if (contentSection() && !active() && !state.content.inventory && !state.content.inventoryToken) loadContentInventory();
      if (resourceSection() && !state.resources.token) loadResourcePage();
      if (state.selectedResponse || state.selectedInput) renderInspector();
      else if (state.selected) { state.selected = state.pages.find(page => page.url === state.selected.url) || state.selected; renderInspector(); }
      if (previousStatus !== state.crawl.status && terminal) {
        await loadHistory();
        if (!current()) return;
        if (state.crawl.status === 'failed') message(state.crawl.error || 'Der Crawl ist fehlgeschlagen. Die bisherigen Daten bleiben sichtbar.', true);
      }
    } catch (error) {
      if (current()) { connection(false); message('Crawl-Daten konnten nicht geladen werden: ' + error.message, true); }
    } finally {
      if (state.pollToken === token) {
        state.pollToken = null; state.polling = false;
        if (generation === state.generation && state.crawl?.id === id && active()) state.pollTimer = setTimeout(() => refresh(generation), 2000);
      }
    }
  }

  async function selectCrawl(id) {
    invalidatePoll();
    resetContent();
    invalidatePolicyPreview();
    resetPolicyLedger();
    state.control = null; state.startOperation = null; state.starting = false;
    limitResumeContext = null;
    if ($('resume-limit-dialog').open) $('resume-limit-dialog').close();
    selectedRecrawlContext = null;
    if ($('selected-recrawl-dialog').open) $('selected-recrawl-dialog').close();
    state.recrawlURLs.clear();
    state.pages = [];
    state.issues = [];
    state.selected = null;
    state.selectedResponse = null;
    state.selectedInput = null;
    state.responses = null;
    state.responsesError = '';
    state.fragments = null;
    state.fragmentsError = '';
    state.page = 0;
    state.crawl = id ? state.crawls.find((crawl) => crawl.id === id) || { id, status: 'running' } : null;
    state.issueMap = new Map();
    state.pageMap = new Map();
    state.cache = {};
    state.analysis = null;
    state.analysisError = '';
    $('inspector').hidden = true;
    message('');
    render();
    if (id) {
      const generation = state.generation;
      if (generation === state.generation) await refresh(generation);
    }
  }

  async function startCrawl(event) {
    event.preventDefault();
    if (actionBusy() || active()) return;
    const mode = $('crawl-mode').value;
    let urls = mode === 'list' ? [] : [$('seed-url').value.trim()], sitemapUrls = [], inputImports = [];
    if (mode !== 'list' && !urls[0]) { message('Bitte gib mindestens eine URL ein.', true); return; }
    for (const url of urls) {
      try { const parsed = new URL(url); if (!['http:', 'https:'].includes(parsed.protocol)) throw new Error('scheme'); }
      catch (_) { message(`Keine gültige HTTP(S)-URL: ${url}`, true); return; }
    }
    let advanced;
    try { advanced = parseAdvanced(state.settings); validateRendererSettings(state.settings); }
    catch (error) { message(`Ungültige Crawl-Einstellung: ${error.message}`, true); return; }
    const operation = { sequence: ++state.operationSequence, generation: state.generation, id: state.crawl?.id || null };
    const current = () => state.startOperation === operation && state.generation === operation.generation && (state.crawl?.id || null) === operation.id;
    state.startOperation = operation; state.starting = true;
    $('start-button').disabled = true;
    message('');
    try {
      if (mode === 'list') {
        if (!listUI.controller) throw new Error('Der Listenimport wurde nicht geladen. Seite neu laden.');
        let imported = listUI.controller.state.accepted;
        if (!imported || listUI.acceptedText !== $('url-list').value) {
          listUI.controller.setEditedText($('url-list').value);
          imported = await listUI.controller.preview();
          if (!current()) return;
          if (!imported) throw new Error(listUI.controller.state.error || 'Die Liste wurde während der Prüfung geändert. Bitte erneut starten.');
        }
        const summary = window.ListImportTools.summarizeRows(imported.rows);
        if (!summary.accepted) { if (!$('list-import-dialog').open) $('list-import-dialog').showModal(); throw new Error('Keine gültigen HTTP(S)-Kandidaten. Die Vorschau enthält die ungültigen Quellzeilen.'); }
        urls = summary.page_urls; sitemapUrls = summary.sitemap_urls; inputImports = [imported];
      }
      if ($('crawl-mode').value !== mode) throw new Error('Der Crawl-Modus wurde während der Prüfung geändert. Bitte erneut starten.');
      if (!current()) return;
      const payload = { ...state.settings, ...advanced, mode, seed_url: mode === 'sitemap' ? '' : urls[0] || '', urls: mode === 'sitemap' ? [] : urls, seed_urls: mode === 'sitemap' ? [] : urls, sitemap_urls: mode === 'sitemap' ? urls : sitemapUrls, input_imports: inputImports, crawl_links: mode !== 'list', include_subdomains: state.settings.follow_subdomains, request_timeout_secs: state.settings.timeout_seconds, max_body_bytes: state.settings.max_body_mb * 1024 * 1024 };
      for (const key of ['crawl_policy_enabled', 'crawl_policy_json', 'live_policy_limits_json', 'content_capture_enabled', 'content_capture_json','resource_discovery_enabled','resource_discovery_json','resource_discovery_limits_json']) delete payload[key];
      payload.renderer = state.settings.javascript_rendering ? { endpoint: state.settings.renderer_endpoint, wait_ms: state.settings.renderer_wait_ms, accessibility: state.settings.renderer_accessibility, screenshot: state.settings.renderer_screenshot, collect_cookies: state.settings.renderer_collect_cookies, custom_javascript: parseCustomJavascript(state.settings.custom_javascript_json) } : null;
      const result = await request('/api/crawls', { method: 'POST', body: JSON.stringify(payload) });
      const crawl = result.crawl || result;
      if (!crawl.id) throw new Error('Der Server hat keine Crawl-ID zurückgegeben.');
      if (!current()) { await loadHistory(); return; }
      state.crawls.unshift(crawl);
      renderHistory();
      $('crawl-history').value = crawl.id;
      await selectCrawl(crawl.id);
    } catch (error) { if (current()) message(`Der Crawl konnte nicht gestartet werden: ${error.message}`, true); }
    finally { if (state.startOperation === operation) { state.startOperation = null; state.starting = false; renderControls(); } }
  }

  async function controlCrawl(action, body, expected) {
    if (!state.crawl || actionBusy()) return;
    if (expected && (expected.id !== state.crawl.id || expected.generation !== state.generation)) return;
    const status = state.crawl.status;
    if (action === 'pause' && !['running', 'queued', 'pending'].includes(status)) return;
    if (action === 'cancel' && !active()) return;
    if (action === 'resume' && !canResume()) return;
    const id = state.crawl.id, generation = invalidatePoll();
    const operation = { id, generation, action, sequence: ++state.operationSequence };
    state.control = operation; renderControls();
    const current = () => state.control === operation && state.generation === generation && state.crawl?.id === id;
    try {
      const response = await request('/api/crawls/' + encodeURIComponent(id) + '/' + action, { method: 'POST', ...(body ? { body: JSON.stringify(body) } : {}) });
      if (!current()) return;
      const observed = response?.crawl || response;
      if (observed && typeof observed.status === 'string') state.crawl = { ...state.crawl, ...observed };
      if (action === 'resume') {
        state.analysis = null; state.analysisError = ''; state.responses = null; state.responsesError = '';
        state.fragments = null; state.fragmentsError = ''; state.selectedResponse = null;
      }
      message(action === 'pause' ? 'Pause angefordert. Laufende Anfragen werden beendet, danach wird der Stand gespeichert.' : action === 'resume' ? 'Crawl wird aus dem validierten lokal gespeicherten Stand fortgesetzt.' : 'Stopp angefordert. Laufende Anfragen werden beendet; erfasste Daten bleiben erhalten.');
      state.control = null; render();
      await refresh(generation);
    } catch (error) {
      if (!current()) return;
      const verb = action === 'pause' ? 'pausiert' : action === 'resume' ? 'fortgesetzt' : 'gestoppt';
      message('Der Crawl konnte nicht ' + verb + ' werden: ' + error.message, true);
      state.control = null; renderControls();
      await refresh(generation);
    } finally {
      if (state.control === operation) { state.control = null; renderControls(); }
    }
  }

  const pauseCrawl = () => controlCrawl('pause');
  const cancelCrawl = () => controlCrawl('cancel');
  function resumeCrawl() {
    if (!canResume() || actionBusy()) return;
    if (state.crawl.status !== 'limited') { controlCrawl('resume'); return; }
    const selected = Boolean(recrawlOperation());
    limitResumeContext = { id: state.crawl.id, generation: state.generation, budgetKey: selected ? 'max_attempts' : 'max_pages' };
    const limit = selected ? recrawlOperation().max_attempts ?? 0 : state.crawl.config?.max_pages ?? 0;
    $('resume-limit-heading').textContent = selected ? 'Abrufbudget festlegen' : 'URL-Grenze festlegen';
    $('resume-limit-current').textContent = `Gespeichertes ${selected ? 'Abrufbudget' : 'URL-Limit'}: ${number(limit)}${limit === 0 ? ' (unbegrenzt)' : ''}.`;
    $('resume-limit-explanation').textContent = selected ? 'Das Abrufbudget gilt für die nächste Ausführung der ausgewählten Wiederprüfung. 0 entfernt diese Grenze; die ursprüngliche URL-Grenze des Eltern-Crawls wird nicht verändert. Laufzeit und Speicherbedarf hängen von deinem Rechner und der Website ab.' : 'Die Grenze bleibt unverändert, bis du hier eine andere Zahl bestätigst. Bei unveränderter Grenze kann der Crawl sofort wieder anhalten. 0 entfernt die konfigurierte URL-Grenze; Laufzeit und Speicherbedarf hängen von deinem Rechner und der Website ab.';
    $('resume-limit-label').textContent = selected ? 'Maximal gestartete Abrufe in der nächsten Ausführung' : 'Maximal erfasste URLs insgesamt';
    $('resume-max-pages').value = limit;
    $('resume-limit-error').hidden = true;
    $('resume-limit-dialog').showModal();
  }

  function openSelectedRecrawl() {
    if (!canRecrawl()) return;
    selectedRecrawlContext = { id: state.crawl.id, generation: state.generation, urls: [...state.recrawlURLs] };
    $('selected-recrawl-urls').replaceChildren(...selectedRecrawlContext.urls.map(url => { const item = document.createElement('li'); item.textContent = url; return item; }));
    $('selected-recrawl-count').textContent = `${number(selectedRecrawlContext.urls.length)} angeforderte URLs aus dem gespeicherten Bericht.`;
    $('recrawl-max-attempts').value = '';
    $('selected-recrawl-error').hidden = true;
    $('selected-recrawl-dialog').showModal();
  }

  async function selectedRecrawl(event) {
    event.preventDefault();
    const expected = selectedRecrawlContext;
    if (!expected || expected.id !== state.crawl?.id || expected.generation !== state.generation || !canRecrawl()) { $('selected-recrawl-dialog').close(); selectedRecrawlContext = null; return; }
    if (!expected.urls.every(url => state.pageMap.has(url))) { $('selected-recrawl-error').textContent = 'Mindestens eine ausgewählte URL gehört nicht mehr zum angezeigten Inventar. Schließe die Bestätigung und aktualisiere die Auswahl.'; $('selected-recrawl-error').hidden = false; return; }
    const text = $('recrawl-max-attempts').value;
    const maxAttempts = text === '' ? null : Number(text);
    if (maxAttempts != null && (!Number.isSafeInteger(maxAttempts) || maxAttempts < 0)) { $('selected-recrawl-error').textContent = 'Das Abrufbudget muss eine nicht negative ganze Zahl sein. Leer oder 0 bedeutet unbegrenzt.'; $('selected-recrawl-error').hidden = false; return; }
    const body = { urls: expected.urls, ...(maxAttempts == null ? {} : { max_attempts: maxAttempts }) };
    selectedRecrawlContext = null; $('selected-recrawl-dialog').close();
    const operation = { id: expected.id, generation: invalidatePoll(), action: 'recrawl', sequence: ++state.operationSequence };
    state.control = operation; renderControls(); message('');
    const current = () => state.control === operation && state.crawl?.id === operation.id && state.generation === operation.generation;
    try {
      const result = await request('/api/crawls/' + encodeURIComponent(operation.id) + '/recrawl', { method: 'POST', body: JSON.stringify(body) });
      if (!result || typeof result.id !== 'string' || !result.id || typeof result.status !== 'string') throw new Error('Der Server hat keine gültige Kennung des neuen Laufs geliefert. Prüfe die Laufhistorie vor einem erneuten Start.');
      if (!current()) { await loadHistory(); return; }
      state.crawls = [...state.crawls.filter(crawl => crawl.id !== result.id), { id: result.id, status: result.status, parent_id: operation.id }];
      state.control = null;
      $('crawl-history').value = result.id;
      await selectCrawl(result.id);
      await loadHistory();
    } catch (error) {
      if (current()) { state.control = null; message('Die Wiederprüfung konnte nicht gestartet werden: ' + error.message, true); renderControls(); await refresh(operation.generation); }
    } finally {
      if (state.control === operation) { state.control = null; renderControls(); }
    }
  }

  function renderRecrawlOperation() {
    const operation = recrawlOperation(), element = $('recrawl-operation');
    element.hidden = !operation;
    if (!operation) return;
    const count = key => operation[key] == null ? 'nicht erfasst' : number(operation[key]);
    $('recrawl-operation-summary').textContent = `Neu geprüft: ${count('refreshed_count')} · Beibehalten: ${count('retained_count')} · Ausstehend: ${count('pending_count')}`;
    $('recrawl-operation-budget').textContent = `Eigenes Abrufbudget: ${operation.max_attempts == null || operation.max_attempts === 0 ? 'unbegrenzt' : number(operation.max_attempts)} · Gestartete Abrufe in dieser Ausführung: ${count('admitted_this_run')} · Gespeicherte Befunde: ${count('committed_this_run')}. Die ursprüngliche URL-Grenze steuert dieses Budget nicht.`;
    $('recrawl-operation-targets').replaceChildren(...array(operation.targets).map(target => { const item = document.createElement('li'); item.textContent = `${target.requested_url} · ${target.state === 'committed' ? 'Neu geprüft' : target.state === 'pending' ? 'Ausstehend · kein aktueller Befund' : 'Zustand nicht erfasst'}`; return item; }));
    $('recrawl-parent').hidden = !operation.parent?.job_id;
    $('recrawl-parent').textContent = 'Elternlauf in Historie öffnen';
    $('recrawl-parent-snapshot').hidden = state.crawl.parent_snapshot_available !== true;
    $('recrawl-parent-evidence').textContent = `Originalquelle: ${operation.parent?.source_kind || 'nicht erfasst'} · Originalbericht SHA-256: ${operation.parent?.snapshot_sha256 || 'nicht erfasst'}${state.crawl.parent_snapshot_available === true ? '' : ' · Exakter Originalbericht nicht zum Download verfügbar.'}`;
  }

  function renderControls() {
    const busy = actionBusy();
    $('start-button').disabled = Boolean(busy || active());
    $('pause-button').hidden = !state.crawl || !['running', 'queued', 'pending', 'pausing'].includes(state.crawl.status);
    $('pause-button').disabled = busy || state.crawl?.status === 'pausing';
    $('cancel-button').hidden = !active();
    $('cancel-button').disabled = busy || state.crawl?.status === 'cancelling';
    $('resume-button').hidden = !canResume();
    $('resume-button').disabled = busy;
    $('crawl-mode').disabled = Boolean(busy || active());
    $('seed-url').disabled = Boolean(busy || active());
    $('url-list').disabled = Boolean(busy || active());
    $('list-import-open').disabled = !window.ListImportTools || Boolean(busy || active());
    const hasData = Boolean(state.crawl && state.pages.length);
    $('export-csv').disabled = policySection() ? !policyVisibleRows().length || Boolean(state.policyToken || state.policyError) : (!hasData && !state.issues.length) || state.section === 'responses' && state.responses == null || state.section === 'fragments' && (!state.fragments || !state.fragments.enabled);
    $('export-sitemap').disabled = !hasData || Boolean(busy || active());
    $('export-project').disabled = !state.crawl || Boolean(busy || active());
    $('export-resources').disabled = !state.crawl || Boolean(busy || active());
    $('recrawl-selected-count').textContent = `${number(state.recrawlURLs.size)} URLs ausgewählt`;
    $('recrawl-selected').disabled = !canRecrawl();
    $('recrawl-clear').disabled = !state.recrawlURLs.size || busy;
    $('recrawl-selection-hint').textContent = active() ? 'Die Auswahl ist nach dem Anhalten oder Abschluss verfügbar.' : state.crawl?.recrawl_available === false ? 'Für diesen Bericht ist keine Wiederprüfung verfügbar.' : 'Auswahl gilt nur für vorhandene Anfrage-URLs in diesem Bericht; Filter und Pagination ändern sie nicht.';
    if (contentSection()) $('export-csv').disabled = active() || Boolean(state.content.inventoryToken || state.content.error) || !contentVisibleRows().length;
    if (state.section==='resource-discovery') $('export-csv').disabled=true;
    $('inspector-recrawl-selection').hidden = !state.selected || !state.pageMap.has(state.selected.url) || Boolean(state.selectedResponse || state.selectedInput);
    $('inspector-recrawl-checkbox').checked = Boolean(state.selected && state.recrawlURLs.has(state.selected.url));
    $('inspector-recrawl-checkbox').disabled = Boolean(busy || active() || state.crawl?.recrawl_available === false);
    $('settings-summary').textContent = `${state.settings.concurrency} parallele Requests · ${state.settings.obey_robots ? 'Robots.txt beachten' : 'Robots.txt ignorieren'}${state.settings.max_pages ? ` · Grenze ${number(state.settings.max_pages)}` : ''}${state.settings.javascript_rendering ? ' · JavaScript-Rendering' : ''}${state.settings.crawl_policy_enabled ? ' · Native URL-Policy' : ''}${state.settings.resource_discovery_enabled ? ' · Native Ressourcenregeln' : ''}`;
  }

  function renderMetrics() {
    renderRecrawlOperation();
    const current = stats();
    const fetched = current.fetched ?? state.pages.filter((page) => page.status != null || page.error || page.blocked_by_robots).length;
    const discovered = current.discovered ?? state.pages.length;
    const queued = current.queued ?? Math.max(0, discovered - fetched);
    const failed = current.failed ?? state.pages.filter((page) => page.error).length;
    const html = htmlPages();
    $('metric-fetched').textContent = number(fetched);
    $('metric-queued').textContent = number(queued);
    $('metric-html').textContent = number(html.length);
    $('metric-issues').textContent = number(Math.max(current.issues || 0, state.issues.length));
    $('metric-discovered').textContent = `${number(discovered)} URLs entdeckt`;
    $('metric-inflight').textContent = current.in_flight == null ? 'Laufende Anfragen: nicht erfasst' : `${number(current.in_flight)} laufende Anfragen`;
    $('metric-discovery').textContent = current.discovery_pending == null ? 'Sitemap-Discovery: nicht erfasst' : `${number(current.discovery_pending)} ausstehende Sitemap-Discovery-Einheiten`;
    const checkpoint = state.crawl?.checkpoint;
    const checkpointParts = state.crawl ? [`Generation ${state.crawl.generation == null ? 'nicht erfasst' : number(state.crawl.generation)}`] : [];
    if (checkpoint) checkpointParts.push(`Gespeicherte Revision ${checkpoint.revision == null ? 'nicht erfasst' : number(checkpoint.revision)}`, `Checkpoint-Generation ${checkpoint.generation == null ? 'nicht erfasst' : number(checkpoint.generation)}`, `Befund: ${checkpoint.outcome || 'nicht erfasst'}`, `Discovery ${checkpoint.discovery_complete === true ? 'abgeschlossen' : checkpoint.discovery_complete === false ? 'unvollständig' : 'nicht erfasst'}`);
    else if (state.crawl) checkpointParts.push('Checkpoint nicht erfasst');
    $('crawl-checkpoint').hidden = !state.crawl;
    $('crawl-checkpoint').textContent = checkpointParts.join(' · ');
    $('metric-failed').textContent = `${number(failed)} fehlgeschlagene Abrufe`;
    $('metric-indexable').textContent = `${number(html.filter((page) => page.indexable === true).length)} indexierbar`;
    const started = state.crawl?.started_at || state.crawl?.created_at;
    const finished = state.crawl?.finished_at;
    const elapsed = started ? Math.max(0, Math.floor(((finished ? new Date(finished).getTime() : Date.now()) - new Date(started).getTime()) / 1000)) : 0;
    $('metric-progress').textContent = state.crawl ? `${number(state.pages.length)} Datensätze · ${Number.isFinite(elapsed) ? `${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, '0')} min` : '—'}` : 'Noch keine URLs';
    const status = state.crawl?.status || 'idle';
    const notices = [...array(state.crawl?.warnings), ...(state.crawl?.error ? [state.crawl.error] : []), ...(state.analysisError ? [state.analysisError] : []), ...(status === 'limited' || state.crawl?.summary?.limited ? ['Die konfigurierte Crawl-Grenze wurde erreicht. Die Website wurde nicht vollständig erfasst. Fortsetzen verwendet die gespeicherten Crawl-Einstellungen.'] : []), ...(state.crawl?.resume_available === false && ['paused', 'limited', 'cancelled', 'failed', 'interrupted'].includes(status) ? ['Fortsetzen ist für diesen gespeicherten Stand nicht verfügbar. Die bisherigen Befunde bleiben einsehbar.'] : []), ...(state.crawl?.summary?.pending ? [`${number(state.crawl.summary.pending)} entdeckte URLs wurden in diesem Crawl noch nicht verarbeitet.`] : [])];
    $('crawl-notices').hidden = !notices.length;
    $('crawl-notices').querySelector('ul').innerHTML = notices.map((notice) => `<li>${escape(notice)}</li>`).join('');
    const statusClass = ['completed', 'complete', 'finished'].includes(status) ? 'completed' : status;
    $('crawl-state').innerHTML = `<span class="status-dot ${escape(statusClass)}"></span>${escape(labels[status] || (state.crawl ? status : 'Bereit für deinen ersten Crawl'))}`;
    const fraction = discovered ? Math.min(1, fetched / discovered) : 0;
    $('progress-bar').style.width = `${state.crawl && ['completed', 'complete', 'finished'].includes(status) ? 100 : fraction * 100}%`;
    $('progress-bar').style.backgroundColor = status === 'failed' ? 'var(--danger)' : ['cancelled', 'canceled', 'cancelling', 'pausing', 'paused', 'limited', 'interrupted'].includes(status) ? 'var(--warning)' : '';
    if (status === 'pausing') $('crawl-state').insertAdjacentHTML('beforeend', '<span>· Noch kein fertiger Pause-Checkpoint</span>');
    else if (active()) $('crawl-state').insertAdjacentHTML('beforeend', '<span>· Warteschlange kann während des Crawls wachsen</span>');
    const counts = { overview: state.crawl ? number(fetched) : '—', internal: number(internals().length), external: number(externalRows().length), responses: state.responses == null ? '—' : number(state.responses.length), images: number(imageRows().length), imagefiles: number(imageFileRows().length), pdf: number(pdfRows().length), issues: number(state.issues.length) };
    document.querySelectorAll('[data-count]').forEach((element) => { element.textContent = counts[element.dataset.count]; });
    renderControls();
  }

  function renderOverview() {
    const element = $('overview-view');
    if (!state.pages.length && !state.issues.length) {
      element.innerHTML = `<div class="empty-overview"><div class="empty-icon" aria-hidden="true">⌘</div><h3>${state.crawl ? 'Die ersten Antworten werden ausgewertet' : 'Dein erster Crawl beginnt hier'}</h3><p>${state.crawl ? 'Sobald URLs abgerufen wurden, erscheinen HTTP-Status, Seitenmetadaten und erkannte Probleme. Ein leerer Bereich ist noch keine Aussage über die Website.' : 'Gib eine Website ein oder importiere eine URL-Liste. Open SEO Spider entdeckt Seiten und zeigt dir die technischen Details, die Suchmaschinen sehen.'}</p><div class="feature-pills"><span>HTTP & Redirects</span><span>Titles & Descriptions</span><span>Canonical & Robots</span><span>Links & Hreflang</span><span>CSV & Sitemap</span></div></div>`;
      return;
    }
    const pages = internals();
    const buckets = [
      ['2xx · Erfolgreich', pages.filter((p) => effectiveStatus(p) >= 200 && effectiveStatus(p) < 300).length, ''],
      ['3xx · Weiterleitung', pages.filter((p) => (p.status >= 300 && p.status < 400) || p.redirect_chain.length).length, 'warning'],
      ['4xx · Client-Fehler', pages.filter((p) => effectiveStatus(p) >= 400 && effectiveStatus(p) < 500).length, 'danger'],
      ['5xx · Server-Fehler', pages.filter((p) => effectiveStatus(p) >= 500).length, 'danger'],
      ['Nicht abgerufen', pages.filter((p) => p.blocked_by_robots || p.error || effectiveStatus(p) == null).length, 'warning']
    ];
    const grouped = new Map();
    for (const issue of state.issues) {
      const group = grouped.get(issue.code) || { ...issue, count: 0 };
      group.count += 1;
      grouped.set(issue.code, group);
    }
    const severityOrder = { error: 0, warning: 1, notice: 2 };
    const groups = [...grouped.values()].sort((a, b) => (severityOrder[a.severity] ?? 3) - (severityOrder[b.severity] ?? 3) || b.count - a.count).slice(0, 6);
    element.innerHTML = `<div class="overview-grid"><div class="overview-card"><h3>Antworten & Abrufstatus</h3>${buckets.map(([label, count, kind]) => `<div class="status-summary-row"><span>${escape(label)}</span><div class="bar ${kind}"><span style="width:${Math.min(100, pages.length ? count / pages.length * 100 : 0)}%"></span></div><strong>${number(count)}</strong></div>`).join('')}<p class="graph-note">Weiterleitungen können zusätzlich in der finalen 2xx-Antwort enthalten sein.</p></div><div class="overview-card"><h3>Erkannte Probleme</h3>${groups.length ? groups.map((group) => `<button class="issue-summary-row text-button" style="width:100%;text-align:left" data-issue-filter="${escape(group.code)}"><i class="severity-dot ${escape(group.severity)}"></i><span>${escape(issueTitle(group.code))}</span><strong>${number(group.count)}</strong></button>`).join('') : '<p class="detail-empty">Bislang wurden keine Probleme gemeldet. Das ist kein Nachweis für vollständige SEO-Fehlerfreiheit.</p>'}</div></div><div class="overview-footnote"><span aria-hidden="true">ⓘ</span><span>${state.crawl?.config?.renderer ? 'JavaScript-Rendering ist konfiguriert. Prüfe den tatsächlichen Renderstatus je Seite; Worker-Fehler erscheinen in den Seitendetails. Externe APIs sind nicht Teil dieses Crawls.' : 'Die Auswertung beruht auf gelieferten HTTP-Antworten und HTML. JavaScript-Inhalte und externe APIs sind nicht Teil dieses Crawls.'}</span></div>`;
    element.querySelectorAll('[data-issue-filter]').forEach((button) => button.addEventListener('click', () => {
      selectSection('issues');
      state.search = button.dataset.issueFilter;
      $('table-search').value = state.search;
      renderTable();
    }));
  }

  function issueTitle(code) {
    const names = { missing_title: 'Seitentitel fehlt', title_missing: 'Seitentitel fehlt', duplicate_title: 'Doppelter Seitentitel', title_duplicate: 'Doppelter Seitentitel', missing_meta_description: 'Meta Description fehlt', meta_description_missing: 'Meta Description fehlt', missing_h1: 'H1 fehlt', h1_missing: 'H1 fehlt', multiple_h1: 'Mehrere H1-Überschriften', h1_multiple: 'Mehrere H1-Überschriften', broken_link: 'Defektes Linkziel', http_error: 'HTTP-Fehler', missing_alt: 'Bild ohne Alt-Attribut', image_missing_alt: 'Bild ohne Alt-Attribut', noindex: 'Noindex-Direktive', robots_blocked: 'Durch Robots.txt ausgeschlossen', redirect: 'Weiterleitung', canonical_missing: 'Canonical fehlt', canonical_multiple: 'Mehrere Canonicals' };
    return names[code] || String(code || 'Unbekannter Hinweis').replace(/[_-]/g, ' ');
  }

  function duplicateValues(field) {
    const counts = new Map();
    for (const page of htmlPages()) {
      const value = primary(page[field]).trim();
      if (value) counts.set(value, (counts.get(value) || 0) + 1);
    }
    return counts;
  }

  function externalRows() {
    if (state.cache.external) return state.cache.external;
    const result = new Map();
    const records = state.pageMap;
    for (const page of internals()) for (const link of page.links) {
      const url = linkURL(link);
      if (!url || typeof link === 'object' && link.internal !== false) continue;
      if (typeof link === 'string') {
        try { if (new URL(url).host === new URL(page.url).host) continue; } catch (_) { continue; }
      }
      const row = result.get(url) || { url, text: typeof link === 'object' ? link.text || '' : '', sources: new Set(), nofollow: typeof link === 'object' && Boolean(link.nofollow), record: records.get(url), _page: page };
      row.sources.add(page.url);
      result.set(url, row);
    }
    state.cache.external = [...result.values()].map((row) => ({ ...row, source_count: row.sources.size, status: row.record ? effectiveStatus(row.record) : null }));
    return state.cache.external;
  }

  function imageRows() {
    if (state.cache.images) return state.cache.images;
    const rows = [];
    for (const page of internals()) for (const image of page.images) {
      if (!image) continue;
      const url = typeof image === 'string' ? image : image.url || image.src || '';
      const resource = resourceAliases().get(url);
      rows.push({ ...(typeof image === 'object' ? image : {}), url, alt: typeof image === 'string' ? null : image.alt, source: page.url, _page: page, _resourcePage: resource, intrinsic_width: resource?.image_metadata?.width, intrinsic_height: resource?.image_metadata?.height, resource_status: resource?.image_metadata?.status });
    }
    state.cache.images = rows;
    return rows;
  }

  function resourceAliases() {
    if (state.cache.resourceAliases) return state.cache.resourceAliases;
    const aliases = new Map(state.pages.map(page => [page.url, page]));
    for (const page of state.pages) if (page.final_url && !aliases.has(page.final_url)) aliases.set(page.final_url, page);
    state.cache.resourceAliases = aliases;
    return aliases;
  }

  function imageFileRows() {
    return state.pages.filter(isImageFile).map(page => ({ ...page, _page: page, resource_status: page.image_metadata?.status, format: page.image_metadata?.format, intrinsic_width: page.image_metadata?.width, intrinsic_height: page.image_metadata?.height, dimension_unit: page.image_metadata?.dimension_unit, encoded_bytes: page.image_metadata?.encoded_bytes, body_complete: page.image_metadata?.body_complete, parse_error: page.image_metadata?.parse_error }));
  }

  function pdfRows() {
    return state.pages.filter(isPDF).map(page => ({ ...page, _page: page, resource_status: page.pdf?.status, pdf_title: page.pdf?.info_metadata?.Title, author: page.pdf?.info_metadata?.Author, page_count: page.pdf?.page_count, declared_page_count: page.pdf?.declared_page_count, pdf_words: page.pdf?.word_count, text_complete: page.pdf?.text_complete, annotation_count: page.pdf ? array(page.pdf.annotation_links).length : null, annotations_complete: page.pdf?.annotations_complete, parse_error: page.pdf?.parse_error || array(page.pdf?.errors).join(' | ') }));
  }

  function domRows(key) {
    if (state.cache[key]) return state.cache[key];
    state.cache[key] = htmlPages().flatMap(page => {
      if (!page.dom_metadata) return [{ url: page.url, _page: page, _unavailable: true }];
      if (key === 'responsive_images') return array(page.dom_metadata.responsive_images).flatMap(image => array(image.candidates).map(candidate => ({ ...candidate, url: page.url, candidate_url: candidate.url, image_src: image.src, image_alt: image.alt, image_dom_location: image.dom_location, _page: page })));
      return array(page.dom_metadata[key]).map(record => ({ ...record, url: page.url, _page: page }));
    });
    return state.cache[key];
  }

  function linkDomRows() {
    return internals().flatMap(page => page.links.map(link => ({ ...(typeof link === 'object' ? link : { url: link }), source_url: page.url, _page: page, _unavailable: typeof link !== 'object' || !link.dom_location })));
  }

  function getRows() {
    switch (state.section) {
      case 'inputs': return array(state.crawl?.config?.input_imports).flatMap((result, index) => array(result.rows).map((row, rowIndex) => ({ ...row, list_index: index + 1, occurrence_index: rowIndex, input_sha256: result.input_sha256, _import: result, url: row.normalized_url || '' })));
      case 'external': return externalRows();
      case 'images': return imageRows();
      case 'imagefiles': return imageFileRows();
      case 'pdf': return pdfRows();
      case 'responses': return array(state.responses).map(response => ({ ...response, url: response.response_url }));
      case 'headingorder': return domRows('headings');
      case 'keywords': return domRows('meta_keywords');
      case 'imagecandidates': return domRows('responsive_images');
      case 'navigation': return domRows('navigation_candidates');
      case 'linkdom': return linkDomRows();
      case 'fragments': return array(state.fragments?.observations).map(record => ({ ...record, url: record.source_url, dom_location: record.source_dom_location, _page: state.pageMap.get(record.source_url) }));
      case 'issues': return state.issues.map((issue) => ({ ...issue, _page: state.pageMap.get(issue.url) }));
      case 'linkmetrics': return array(state.analysis?.graph?.nodes).map((node) => ({ ...node, _page: state.pageMap.get(node.url) }));
      case 'structured': return array(state.analysis?.structured_data).map((entry) => ({ ...entry, types_text: Object.entries(entry.types || {}).map(([type, count]) => `${type} (${count})`).join(' | '), _page: state.pageMap.get(entry.url) }));
      case 'extractions': return internals().flatMap((page) => page.custom_extractions.map((entry) => ({ ...entry, url: page.url, name: entry.name || entry.selector, values: array(entry.values), _page: page })));
      case 'javascript': return internals().flatMap(page => page.custom_javascript_results.map(entry => ({ ...entry, url: page.url, value_json: entry.error ? '' : JSON.stringify(entry.value), _page: page })));
      case 'cookies': return internals().flatMap(page => page.browser_cookies.map(entry => ({ ...entry, url: page.url, _page: page })));
      case 'titles': { if (state.cache.titles) return state.cache.titles; const duplicates = duplicateValues('titles'); state.cache.titles = htmlPages().map((page) => ({ ...page, title_text: primary(page.titles), title_length: Array.from(primary(page.titles)).length, duplicates: duplicates.get(primary(page.titles).trim()) || 0 })); return state.cache.titles; }
      case 'meta': { if (state.cache.meta) return state.cache.meta; const duplicates = duplicateValues('meta_descriptions'); state.cache.meta = htmlPages().map((page) => ({ ...page, description_text: primary(page.meta_descriptions), description_length: Array.from(primary(page.meta_descriptions)).length, duplicates: duplicates.get(primary(page.meta_descriptions).trim()) || 0 })); return state.cache.meta; }
      case 'headings': return htmlPages();
      default: return internals();
    }
  }

  const textColumn = (key, label, getter = (row) => row[key], klass = '') => ({ key, label, value: getter, render: (row) => `<span class="${klass || 'truncate'}" title="${escape(getter(row) ?? '')}">${escape(getter(row) ?? '—')}</span>` });
  const urlColumn = textColumn('url', 'URL', (row) => row.url, 'url-cell');
  const numberColumn = (key, label, getter = (row) => row[key]) => ({ key, label, value: getter, render: (row) => getter(row) == null ? '—' : escape(number(getter(row))) });
  const statusColumn = { key: 'status', label: 'HTTP final', value: effectiveStatus, render: (row) => statusBadge(effectiveStatus(row)) };
  const indexColumn = { key: 'indexable', label: 'Indexierbarkeit', value: (row) => row.indexable === true ? 'Indexierbar' : row.indexable === false ? 'Nicht indexierbar' : 'Unbekannt', render: (row) => `<span class="badge ${row.indexable === false ? 'warning' : row.indexable == null ? 'neutral' : ''}">${row.indexable === true ? 'Indexierbar' : row.indexable === false ? 'Nicht indexierbar' : 'Unbekannt'}</span>` };
  const issueColumn = numberColumn('issue_count', 'Probleme', (row) => pageIssues(row.url).length);
  const domColumns = () => [numberColumn('dom_index', 'DOM-Index', row => row.dom_location?.index), textColumn('dom_path', 'DOM-Pfad', domPath), textColumn('dom_source', 'DOM-Quelle', domSource), textColumn('dom_complete', 'Vollständiger Prüfbefund', row => !row._page?.dom_metadata ? 'Nicht erfasst' : completeDOM(row._page) ? 'Ja' : 'Nein / unklar')];

  function columns() {
    switch (state.section) {
      case 'inputs': return [numberColumn('list_index', 'Liste'), textColumn('input_sha256', 'Quellhash'), numberColumn('source_index', 'Quellindex'), numberColumn('source_row', 'Quellzeile'), numberColumn('source_line', 'Physische Zeile'), textColumn('sheet', 'Worksheet'), textColumn('cell', 'Zelle'), textColumn('raw_value', 'Originalwert', row => row.raw_value == null ? 'Nicht erfasst' : row.raw_value === '' ? '[leer]' : row.raw_value), textColumn('normalized_url', 'Normalisierte URL'), textColumn('fragment', 'Fragment'), textColumn('candidate_kind', 'Kandidat', row => row.candidate_kind === 'page_url' ? 'Seiten-URL' : row.candidate_kind === 'sitemap_document' ? 'Sitemap-Dokument' : 'Nicht erfasst'), textColumn('error', 'Importfehler')];
      case 'external': return [urlColumn, { key: 'status', label: 'HTTP geprüft', value: (row) => row.status, render: (row) => statusBadge(row.status) }, textColumn('text', 'Linktext'), numberColumn('source_count', 'Verlinkende Seiten'), textColumn('nofollow', 'Nofollow', (row) => row.nofollow ? 'Ja' : 'Nein')];
      case 'status': return [urlColumn, { key: 'initial_status', label: 'HTTP initial', value: (row) => row.status, render: (row) => statusBadge(row.status) }, statusColumn, textColumn('final_url', 'Finale URL'), numberColumn('redirects', 'Redirects', (row) => row.redirect_chain.length), { key: 'response_ms', label: 'Antwortzeit', value: (row) => row.response_ms, render: (row) => duration(row.response_ms) }, indexColumn, textColumn('error', 'Abruffehler')];
      case 'responses': return [urlColumn, textColumn('requested_url', 'Anfrage-URL'), numberColumn('response_index', 'Antwortposition'), { key: 'status', label: 'HTTP beobachtet', value: row => row.status, render: row => statusBadge(row.status) }, textColumn('location', 'Location'), textColumn('capture_kind', 'Befund', row => row.capture_kind === 'redirect_hop' ? 'Zwischenredirect' : 'Terminale Antwort'), textColumn('headers_known', 'Header erfasst', row => row.headers == null ? 'Unbekannt' : 'Ja'), textColumn('content_type', 'Content-Type'), numberColumn('body_bytes', 'Erfasste Bodybytes'), textColumn('body_complete', 'Body vollständig', row => row.body_complete == null ? 'Unbekannt' : row.body_complete ? 'Ja' : 'Nein'), textColumn('document_metadata_available', 'HTML-Metadaten verfügbar', row => row.document_metadata_available ? 'Ja' : 'Nein')];
      case 'titles': return [urlColumn, textColumn('title_text', 'Seitentitel'), numberColumn('title_length', 'Zeichen'), numberColumn('title_pixels', 'Pixelprofil 20px', row => array(row.title_pixel_widths)[0]), numberColumn('title_count', 'Title-Tags', (row) => row.titles.length), numberColumn('duplicates', 'Vorkommen'), statusColumn];
      case 'meta': return [urlColumn, textColumn('description_text', 'Meta Description'), numberColumn('description_length', 'Zeichen'), numberColumn('description_pixels', 'Pixelprofil 14px', row => array(row.description_pixel_widths)[0]), numberColumn('meta_count', 'Meta-Tags', (row) => row.meta_descriptions.length), numberColumn('duplicates', 'Vorkommen'), statusColumn];
      case 'headings': return [urlColumn, textColumn('h1', 'H1', (row) => first(row.h1)), numberColumn('h1_count', 'H1-Anzahl', (row) => row.h1.length), textColumn('h2', 'H2', (row) => first(row.h2)), numberColumn('h2_count', 'H2-Anzahl', (row) => row.h2.length), numberColumn('word_count', 'Wörter')];
      case 'headingorder': return [urlColumn, numberColumn('tag_level', 'Tag-Level'), textColumn('text', 'Heading-Text'), textColumn('raw_text', 'Originaler Text'), textColumn('text_with_image_alt', 'Text mit Inline-Bild-Alt', row => row.text_with_image_alt == null ? 'Nicht erfasst' : row.text_with_image_alt === '' ? '[leer]' : row.text_with_image_alt), textColumn('heading_text_engine', 'Text-/Alt-Profil', row => row._page?.dom_metadata?.heading_text_engine || 'Nicht erfasst'), textColumn('image_alt', 'Bild-Alt im Heading', row => row._unavailable ? 'Nicht erfasst' : array(row.image_alt).map(value => value == null ? '[Attribut fehlt]' : value === '' ? '[leer]' : value).join(' | ')), textColumn('aria_level', 'aria-level'), textColumn('heading_offset', 'headingoffset'), textColumn('hidden_attribute', 'hidden-Attribut', row => row.hidden_attribute == null ? 'Nicht erfasst' : row.hidden_attribute ? 'Ja' : 'Nein'), ...domColumns()];
      case 'keywords': return [urlColumn, textColumn('content', 'Originaler content', row => row._unavailable ? 'Nicht erfasst' : row.content == null ? '[content-Attribut fehlt]' : row.content === '' ? '[leer]' : row.content), textColumn('normalized', 'Normalisierter Text'), numberColumn('chars', 'Originale Zeichen'), ...domColumns()];
      case 'linkdom': return [urlColumn, textColumn('source_url', 'Quellseite'), textColumn('kind', 'Linkart'), textColumn('text', 'Linktext'), textColumn('raw_href', 'Originales Attribut'), textColumn('resolved_href', 'Aufgelöst inkl. Fragment'), textColumn('fragment', 'Fragment'), textColumn('position', 'Position', row => row.position ?? 'Nicht erfasst'), ...domColumns()];
      case 'imagecandidates': return [urlColumn, textColumn('image_src', 'img src'), textColumn('image_alt', 'img Alt'), textColumn('source', 'Kandidatenquelle'), textColumn('raw_url', 'Deklarierter Kandidat'), textColumn('candidate_url', 'Aufgelöste HTTP(S)-URL'), textColumn('descriptors', 'Deskriptoren', row => first(row.descriptors)), numberColumn('width', 'w-Deskriptor'), numberColumn('density', 'x-Deskriptor'), textColumn('media', 'Media'), textColumn('type_hint', 'Typdeklaration'), textColumn('sizes', 'Sizes'), textColumn('eligible', 'Deklaration akzeptiert', row => row.eligible == null ? 'Nicht erfasst' : row.eligible ? 'Ja' : 'Nein'), textColumn('selection', 'Browserauswahl', () => 'Nicht erfasst'), textColumn('error', 'Kandidatenfehler'), ...domColumns()];
      case 'navigation': return [urlColumn, textColumn('tag', 'Element'), textColumn('reason', 'Beobachtung'), textColumn('text', 'Elementtext'), textColumn('raw_href', 'Originales href / Routerattribut'), textColumn('resolved_href', 'Aufgelöste Adresse'), textColumn('potential_url', 'Mögliche HTTP(S)-URL'), textColumn('scheme', 'Scheme'), textColumn('onclick', 'onclick-Attribut'), textColumn('router_link', 'Router-/Data-Attribut'), ...domColumns()];
      case 'fragments': return [urlColumn, textColumn('source_document_url', 'Quelldokument'), textColumn('target_url', 'Ziel-URL'), textColumn('resolved_href', 'Aufgelöstes href'), textColumn('fragment', 'Originalfragment'), textColumn('decoded_fragment', 'Dekodiertes Fragment'), textColumn('status', 'Prüfergebnis', row => fragmentStatus(row.status)), textColumn('reason', 'Begründung'), textColumn('target_document_url', 'Erfasstes Zieldokument'), textColumn('target_complete', 'Ziel vollständig', row => row.target_complete ? 'Ja' : 'Nein / unklar'), textColumn('matched_target', 'Gefundener Zieltyp', row => row.matched_target?.kind), textColumn('target_dom_path', 'Ziel-DOM-Pfad', row => row.matched_target?.dom_location?.path), ...domColumns().slice(0, 2)];
      case 'canonical': return [urlColumn, textColumn('canonical', 'Canonical-Ziel', (row) => first(row.canonical)), numberColumn('canonical_count', 'Deklarationen', canonicalCount), textColumn('self_reference', 'Selbstreferenz', (row) => row.canonical.length ? row.canonical.includes(row.rendered && !row.renderer_error ? row.rendered_url || row.final_url || row.url : row.final_url || row.url) ? 'Ja' : 'Nein' : '—'), indexColumn];
      case 'robots': return [urlColumn, textColumn('blocked', 'Robots.txt', (row) => row.blocked_by_robots ? 'Ausgeschlossen' : row.robots_warning ? `Hinweis: ${row.robots_warning}` : effectiveStatus(row) == null ? 'Unbekannt' : 'Nicht ausgeschlossen'), textColumn('meta_robots', 'Meta Robots', (row) => first(row.meta_robots)), textColumn('x_robots_tag', 'X-Robots-Tag', (row) => first(row.x_robots_tag)), indexColumn, textColumn('indexability_reasons', 'Begründung', (row) => first(row.indexability_reasons))];
      case 'hreflang': return [urlColumn, textColumn('language', 'HTML-Sprache'), numberColumn('alternates', 'Sprachalternativen', (row) => row.hreflang.length), textColumn('hreflang', 'Deklarierte Alternativen', (row) => row.hreflang.map((entry) => `${entry.language || entry.lang || '?'}: ${entry.url || entry.href || ''}`).join(' | ')), statusColumn];
      case 'images': return [urlColumn, textColumn('alt', 'Alt-Text', (row) => row.alt == null ? '[Attribut fehlt]' : String(row.alt).trim() === '' ? '[leer]' : row.alt), textColumn('source', 'Quellseite'), textColumn('width', 'Breite (HTML)'), textColumn('height', 'Höhe (HTML)'), textColumn('loading', 'Loading'), textColumn('resource_status', 'Dateianalyse', row => resourceStatus(row.resource_status)), numberColumn('intrinsic_width', 'Dateibreite'), numberColumn('intrinsic_height', 'Dateihöhe')];
      case 'imagefiles': return [urlColumn, statusColumn, textColumn('resource_status', 'Dateianalyse', row => resourceStatus(row.resource_status)), textColumn('format', 'Dateiformat'), numberColumn('intrinsic_width', 'Intrinsische Breite'), numberColumn('intrinsic_height', 'Intrinsische Höhe'), textColumn('dimension_unit', 'Maßeinheit'), numberColumn('encoded_bytes', 'Erfasste Dateibytes'), textColumn('body_complete', 'Body vollständig', row => row.body_complete == null ? 'Unbekannt' : row.body_complete ? 'Ja' : 'Nein'), textColumn('parse_error', 'Parserfehler')];
      case 'pdf': return [urlColumn, statusColumn, textColumn('resource_status', 'PDF-Analyse', row => resourceStatus(row.resource_status)), textColumn('pdf_title', 'PDF-Metadatentitel'), textColumn('author', 'Autor'), numberColumn('page_count', 'Geprüfte Seitenzahl'), numberColumn('declared_page_count', 'Deklarierte Seitenzahl'), numberColumn('pdf_words', 'Erfasste Wörter'), textColumn('text_complete', 'Text vollständig', row => row.text_complete == null ? 'Unbekannt' : row.text_complete ? 'Ja' : 'Nein'), numberColumn('annotation_count', 'URI-Annotationen'), textColumn('annotations_complete', 'Annotationen vollständig', row => row.annotations_complete == null ? 'Unbekannt' : row.annotations_complete ? 'Ja' : 'Nein'), textColumn('parse_error', 'Parserfehler')];
      case 'issues': return [urlColumn, { key: 'severity', label: 'Priorität', value: (row) => row.severity, render: (row) => `<span class="badge ${row.severity === 'error' ? 'danger' : row.severity === 'warning' ? 'warning' : 'neutral'}">${row.severity === 'error' ? 'Fehler' : row.severity === 'warning' ? 'Warnung' : 'Hinweis'}</span>` }, textColumn('code', 'Problem', (row) => issueTitle(row.code)), textColumn('message', 'Details'), numberColumn('related', 'Weitere URLs', (row) => array(row.related_urls).length)];
      case 'linkmetrics': return [urlColumn, { key: 'link_score', label: 'Eigener Linkscore', value: (row) => row.link_score, render: (row) => Number(row.link_score || 0).toLocaleString('de-DE', { maximumFractionDigits: 2 }) }, numberColumn('in_degree', 'Verlinkende URLs'), numberColumn('out_degree', 'Verlinkte URLs'), numberColumn('inlink_occurrences', 'Eingehende Anker'), numberColumn('outlink_occurrences', 'Ausgehende Anker'), numberColumn('crawl_depth', 'Crawl-Tiefe'), numberColumn('shortest_link_depth', 'Kürzester Linkweg'), indexColumn];
      case 'structured': return [urlColumn, numberColumn('parsed_blocks', 'JSON-LD-Blöcke'), numberColumn('invalid_json_blocks', 'Ungültiges JSON'), textColumn('types_text', 'Beobachtete @type-Werte'), numberColumn('typed_nodes', 'Knoten mit @type', (row) => array(row.typed_nodes).length), textColumn('validation', 'Schema-Validierung', () => 'Nicht ausgeführt')];
      case 'extractions': return [urlColumn, textColumn('name', 'Feldname'), textColumn('method', 'Methode', row => row.method || 'css'), textColumn('output', 'Ausgabe', row => row.output || 'auto'), textColumn('result_type', 'Ergebnistyp'), textColumn('selector', 'Ausdruck'), textColumn('attribute', 'Attribut', (row) => row.attribute || '—'), textColumn('source', 'Quelle', (row) => row.source === 'raw_html' ? 'HTML-Antwort' : row.source === 'rendered_html' ? 'Gerenderter DOM' : row.source || 'Unbekannt'), numberColumn('matches', 'Erfasste Werte', (row) => row.values.length), numberColumn('matched_count', 'Beobachtete Treffer'), textColumn('truncated', 'Abgeschnitten', row => row.truncated ? 'Ja' : 'Nein'), textColumn('values', 'Werte', (row) => row.values.join(' | ')), textColumn('error', 'Fehler')];
      case 'javascript': return [urlColumn, textColumn('name', 'Feldname'), textColumn('value_type', 'Datentyp'), textColumn('value_json', 'JSON-Rückgabe'), textColumn('error', 'Auswertungsfehler'), { key: 'elapsed_ms', label: 'Ausführungszeit', value: row => row.elapsed_ms, render: row => duration(row.elapsed_ms) }];
      case 'cookies': return [urlColumn, textColumn('name', 'Cookie-Name'), textColumn('domain', 'Domain'), textColumn('path', 'Pfad'), textColumn('sameSite', 'SameSite'), textColumn('httpOnly', 'HttpOnly', row => row.httpOnly ? 'Ja' : 'Nein'), textColumn('secure', 'Secure', row => row.secure ? 'Ja' : 'Nein'), numberColumn('value_length', 'Wertlänge in Bytes'), textColumn('expires', 'Ablauf', row => row.expires < 0 ? 'Session' : new Date(row.expires * 1000).toISOString())];
      case 'pagination': return [urlColumn, textColumn('pagination_next', 'Next', row => first(row.pagination_next)), textColumn('pagination_prev', 'Prev', row => first(row.pagination_prev)), textColumn('amphtml', 'AMP-Ziel', row => first(row.amphtml)), statusColumn, indexColumn];
      default: return [urlColumn, statusColumn, textColumn('content_type', 'Content-Type'), textColumn('titles', 'Seitentitel', (row) => first(row.titles)), indexColumn, numberColumn('depth', 'Tiefe'), { key: 'response_ms', label: 'Antwortzeit', value: (row) => row.response_ms, render: (row) => duration(row.response_ms) }, numberColumn('word_count', 'Wörter'), issueColumn];
    }
  }

  function filterOptions() {
    const base = [['all', 'Alle Ergebnisse']];
    switch (state.section) {
      case 'inputs': return [...base, ['invalid', 'Ungültige Zeilen'], ['page_url', 'Seiten-URLs'], ['sitemap_document', 'Sitemap-Dokumente']];
      case 'fragments': return [...base, ['matched', 'ID / Name / Dokumentanfang gefunden'], ['missing', 'Ziel fehlt'], ['unknown', 'Unbekannt / nicht prüfbar'], ['unsupported', 'Nicht unterstützt']];
      case 'headingorder': return [...base, ['unavailable', 'Nicht erfasst'], ['partial', 'Unvollständiger Prüfbefund'], ['raw', 'HTML-Antwort'], ['rendered', 'Gerenderter DOM'], ...[1, 2, 3, 4, 5, 6].map(level => [`h${level}`, `H${level}`])];
      case 'keywords': return [...base, ['unavailable', 'Nicht erfasst'], ['partial', 'Unvollständiger Prüfbefund'], ['raw', 'HTML-Antwort'], ['rendered', 'Gerenderter DOM'], ['content_missing', 'content-Attribut fehlt'], ['empty', 'Vorhandener content leer']];
      case 'imagecandidates': return [...base, ['unavailable', 'Nicht erfasst'], ['partial', 'Unvollständiger Prüfbefund'], ['raw', 'HTML-Antwort'], ['rendered', 'Gerenderter DOM'], ['error', 'Kandidatenfehler'], ['picture_source', 'picture source'], ['img_srcset', 'img srcset'], ['img_src', 'img src']];
      case 'navigation': return [...base, ['unavailable', 'Nicht erfasst'], ['partial', 'Unvollständiger Prüfbefund'], ['raw', 'HTML-Antwort'], ['rendered', 'Gerenderter DOM'], ['javascript_href', 'javascript href'], ['non_http_scheme', 'Anderes Scheme'], ['custom_navigation_attribute', 'Router-/Data-Attribut']];
      case 'linkdom': return [...base, ['unavailable', 'DOM-Position nicht erfasst'], ['anchor', 'Ankerlinks'], ['fragment', 'Mit Fragment'], ...['nav', 'header', 'footer', 'aside', 'head', 'content'].map(position => [`position:${position}`, `Position: ${position}`]), ...array(state.crawl?.config?.link_position_rules).map(rule => [`position:${rule.name}`, `Eigene Position: ${rule.name}`])];
      case 'responses': return [...base, ['2xx', 'HTTP 2xx'], ['3xx', 'HTTP 3xx'], ['4xx', 'HTTP 4xx'], ['5xx', 'HTTP 5xx'], ['redirect_hop', 'Zwischenredirects'], ['headers_unknown', 'Header unbekannt'], ['body_unknown', 'Body unbekannt'], ['body_partial', 'Body unvollständig']];
      case 'issues': return [...base, ['error', 'Fehler'], ['warning', 'Warnungen'], ['notice', 'Hinweise']];
      case 'titles': case 'meta': return [...base, ['missing', 'Fehlend'], ['multiple', 'Mehrere Tags'], ['duplicate', 'Duplikate'], ['short', 'Unter konfigurierter Zeichengrenze'], ['long', 'Über konfigurierter Zeichengrenze'], ['pixels', 'Über konfigurierter Pixelgrenze']];
      case 'headings': return [...base, ['missing', 'H1 fehlt'], ['multiple', 'Mehrere H1']];
      case 'canonical': return [...base, ['missing', 'Canonical fehlt'], ['multiple', 'Mehrere Canonicals'], ['other', 'Nicht selbstreferenziert']];
      case 'robots': return [...base, ['blocked', 'Robots.txt-Ausschluss'], ['nonindexable', 'Nicht indexierbar'], ['noindex', 'Noindex-Direktive']];
      case 'hreflang': return [...base, ['present', 'Mit Hreflang'], ['missing', 'Ohne Hreflang']];
      case 'images': return [...base, ['missing', 'Alt-Attribut fehlt'], ['empty', 'Alt-Text leer'], ['lazy', 'Lazy Loading']];
      case 'imagefiles': return [...base, ['complete', 'Erfasst'], ['partial', 'Unvollständig'], ['error', 'Parserfehler'], ['unavailable', 'Nicht erfasst'], ['dimensions_unknown', 'Maße unbekannt']];
      case 'pdf': return [...base, ['complete', 'Erfasst'], ['partial', 'Unvollständig'], ['error', 'Parserfehler'], ['encrypted', 'Verschlüsselt'], ['disabled', 'Analyse deaktiviert'], ['unavailable', 'Nicht erfasst']];
      case 'external': return [...base, ['unverified', 'Nicht geprüft'], ['error', 'HTTP-Fehler'], ['nofollow', 'Nofollow']];
      case 'linkmetrics': return [...base, ['zero', 'Keine beobachteten Eingangslinks'], ['unreachable', 'Kein beobachteter Weg ab Start-URL']];
      case 'structured': return [...base, ['invalid', 'Mit ungültigem JSON'], ['types', 'Mit @type-Werten']];
      case 'extractions': return [...base, ['present', 'Mit Werten'], ['empty', 'Ohne Werte'], ['error', 'Mit Fehler'], ['truncated', 'Erfassung abgeschnitten']];
      case 'javascript': return [...base, ['success', 'Mit Rückgabe'], ['error', 'Mit Auswertungsfehler']];
      case 'cookies': return [...base, ['insecure', 'Ohne Secure'], ['scriptable', 'Ohne HttpOnly'], ['session', 'Session-Cookies']];
      case 'pagination': return [...base, ['present', 'Mit Pagination'], ['amp', 'Mit AMP-Verweis']];
      default: return [...base, ['html', 'HTML'], ['2xx', 'HTTP 2xx'], ['3xx', 'Weiterleitungen'], ['4xx', 'HTTP 4xx'], ['5xx', 'HTTP 5xx'], ['nonindexable', 'Nicht indexierbar'], ['issues', 'Mit Problemen'], ['failed', 'Abruffehler']];
    }
  }

  function matchesFilter(row) {
    const filter = state.filter;
    if (filter === 'all') return true;
    if (state.section === 'inputs') return filter === 'invalid' ? row.error != null : row.candidate_kind === filter;
    if (state.section === 'fragments') return filter === 'matched' ? ['matched_id', 'matched_name', 'top'].includes(row.status) : row.status === filter;
    if (state.section === 'linkdom') return filter === 'unavailable' ? row._unavailable : filter === 'fragment' ? row.fragment != null : filter.startsWith('position:') ? row.position === filter.slice(9) : row.kind === filter;
    if (domSections.includes(state.section)) {
      if (filter === 'unavailable') return row._unavailable;
      if (filter === 'partial') return Boolean(row._page?.dom_metadata) && !completeDOM(row._page);
      if (filter === 'raw' || filter === 'rendered') return row._page?.dom_metadata?.source === `${filter}_html`;
      if (row._unavailable) return false;
      if (state.section === 'headingorder') return row.tag_level === Number(filter.slice(1));
      if (state.section === 'keywords') return filter === 'content_missing' ? row.content == null : row.content != null && String(row.content).trim() === '';
      if (state.section === 'imagecandidates') return filter === 'error' ? Boolean(row.error) : row.source === filter;
      return row.reason === filter;
    }
    if (state.section === 'responses') return filter === 'redirect_hop' ? row.capture_kind === 'redirect_hop' : filter === 'headers_unknown' ? row.headers == null : filter === 'body_unknown' ? row.body_bytes == null : filter === 'body_partial' ? row.body_complete === false : Math.floor(row.status / 100) === Number(filter[0]);
    if (['pdf', 'imagefiles'].includes(state.section)) return filter === 'unavailable' ? !row.resource_status : filter === 'dimensions_unknown' ? row.intrinsic_width == null || row.intrinsic_height == null : row.resource_status === filter;
    if (state.section === 'issues') return row.severity === filter;
    if (state.section === 'linkmetrics') return filter === 'zero' ? row.in_degree === 0 : row.shortest_link_depth == null;
    if (state.section === 'structured') return filter === 'invalid' ? row.invalid_json_blocks > 0 : Boolean(row.types_text);
    if (state.section === 'extractions') return filter === 'present' ? row.values.length > 0 : filter === 'error' ? Boolean(row.error) : filter === 'truncated' ? row.truncated : !row.error && row.values.length === 0;
    if (state.section === 'javascript') return filter === 'error' ? Boolean(row.error) : !row.error;
    if (state.section === 'cookies') return filter === 'insecure' ? !row.secure : filter === 'scriptable' ? !row.httpOnly : row.expires < 0;
    if (state.section === 'pagination') return filter === 'amp' ? array(row.amphtml).length > 0 : array(row.pagination_next).length + array(row.pagination_prev).length > 0;
    if (state.section === 'external') return filter === 'unverified' ? row.status == null : filter === 'error' ? row.status >= 400 : row.nofollow;
    if (state.section === 'images') return filter === 'missing' ? row.alt == null : filter === 'empty' ? String(row.alt ?? '').trim() === '' : row.loading === 'lazy';
    if (['titles', 'meta'].includes(state.section)) {
      const values = state.section === 'titles' ? row.titles : row.meta_descriptions;
      const kind = state.section === 'titles' ? 'title' : 'description', thresholds = state.crawl?.config?.audit_thresholds || {};
      const threshold = (key, fallback) => Object.hasOwn(thresholds, key) ? thresholds[key] : fallback;
      const max = threshold(`max_${kind}_chars`, kind === 'title' ? 60 : 155), min = threshold(`min_${kind}_chars`, kind === 'title' ? 30 : 70), length = Array.from(primary(values)).length;
      if (filter === 'pixels') { const width = array(row[kind === 'title' ? 'title_pixel_widths' : 'description_pixel_widths'])[0], limit = threshold(`max_${kind}_pixels`, kind === 'title' ? 561 : 985); return width != null && limit != null && width > limit; }
      return filter === 'missing' ? !values.length || !primary(values).trim() : filter === 'multiple' ? values.length > 1 : filter === 'duplicate' ? row.duplicates > 1 : filter === 'short' ? min != null && length > 0 && length < min : max != null && length > max;
    }
    if (state.section === 'headings') return filter === 'missing' ? !row.h1.length : row.h1.length > 1;
    if (state.section === 'canonical') return filter === 'missing' ? !row.canonical.length : filter === 'multiple' ? canonicalCount(row) > 1 : row.canonical.length && !row.canonical.includes(row.rendered && !row.renderer_error ? row.rendered_url || row.final_url || row.url : row.final_url || row.url);
    if (state.section === 'hreflang') return filter === 'present' ? row.hreflang.length > 0 : row.hreflang.length === 0;
    if (filter === 'blocked') return Boolean(row.blocked_by_robots);
    if (filter === 'noindex') return array(row.robots_directives).length ? array(row.effective_robots).includes('noindex') : /(?:^|[\s,])noindex(?:$|[\s,])/i.test([...row.meta_robots, ...row.x_robots_tag].join(' '));
    if (filter === 'nonindexable') return row.indexable === false;
    if (filter === 'html') return isHTML(row);
    if (filter === 'issues') return pageIssues(row.url).length > 0;
    if (filter === 'failed') return Boolean(row.error);
    if (filter === '3xx') return row.status >= 300 && row.status < 400 || row.redirect_chain.length > 0;
    const status = effectiveStatus(row);
    return filter === '2xx' ? status >= 200 && status < 300 : filter === '4xx' ? status >= 400 && status < 500 : filter === '5xx' ? status >= 500 : true;
  }

  function filteredRows() {
    const search = state.search.toLocaleLowerCase('de-DE');
    const currentColumns = columns();
    const rows = getRows().filter((row) => matchesFilter(row) && (!search || currentColumns.map((column) => column.value(row) ?? '').join(' ').toLocaleLowerCase('de-DE').includes(search) || String(row.code || '').toLowerCase().includes(search)));
    const column = currentColumns.find((entry) => entry.key === state.sort) || currentColumns[0];
    rows.sort((a, b) => {
      const one = column.value(a), two = column.value(b);
      if (typeof one === 'number' && typeof two === 'number') return (one - two) * state.direction;
      return String(one ?? '').localeCompare(String(two ?? ''), 'de', { numeric: true, sensitivity: 'base' }) * state.direction;
    });
    return rows;
  }

  function renderTable() {
    const currentColumns = columns();
    const rows = filteredRows();
    const pages = Math.ceil(rows.length / state.pageSize);
    state.page = Math.max(0, Math.min(state.page, pages - 1));
    const offset = state.page * state.pageSize;
    const visible = rows.slice(offset, offset + state.pageSize);
    const selectionCell = row => {
      if (['inputs', 'responses'].includes(state.section)) return '';
      const page = recrawlPage(row);
      if (!page) return '';
      return `<input class="recrawl-row-checkbox" type="checkbox" data-recrawl-url="${escape(page.url)}" aria-label="Für Wiederprüfung auswählen: ${escape(page.url)}" ${state.recrawlURLs.has(page.url) ? 'checked' : ''} ${active() || actionBusy() || state.crawl?.recrawl_available === false ? 'disabled' : ''}>`;
    };
    const cell = (column, row, index) => `${index === 0 ? selectionCell(row) : ''}${column.render(row)}${index === 0 && !['inputs', 'responses'].includes(state.section) && recrawlPage(row) && observationAge(recrawlPage(row).url) ? `<small class="observation-age">${escape(observationAge(recrawlPage(row).url))}</small>` : ''}`;
    const isSelected = row => state.section === 'inputs' ? state.selectedInput && row.list_index === state.selectedInput.list_index && row.occurrence_index === state.selectedInput.occurrence_index : state.section === 'responses' ? state.selectedResponse && state.selectedResponse.requested_url === row.requested_url && state.selectedResponse.response_index === row.response_index : state.selected && (row._page?.url || row.url) === state.selected.url;
    const emptyNote = state.section === 'responses' ? state.responsesError || (state.responses == null ? 'Diese Beobachtungen werden nach Abschluss oder Stopp des Crawls geladen. Nicht erfasste Antworten bleiben unbekannt.' : 'Es wurden keine nativen HTTP-Antworten erfasst, beispielsweise bei DNS-Fehlern oder Robots-Sperren.') : state.section === 'fragments' ? state.fragmentsError || (!state.fragments ? 'Fragmentbeobachtungen werden nach Abschluss oder Stopp geladen.' : !state.fragments.enabled ? 'Fragmentprüfung ist für diesen Crawl deaktiviert. Aktiviere sie in den Einstellungen für einen neuen Crawl.' : 'Keine Fragmentreferenzen im erfassten Inventar. Crawl- und Capture-Grenzen gelten weiterhin.') : null;
    const table = $('results-table');
    const inputEmptyNote = state.section === 'inputs' ? array(state.crawl?.config?.input_imports).length ? 'Die gespeicherten Importergebnisse enthalten keine Eingabezeilen.' : 'Eingabeherkunft nicht erfasst. Bei älteren Projekten bleibt sie unbekannt; dies ist kein Nachweis einer leeren Eingabe.' : null;
    table.querySelector('thead').innerHTML = `<tr>${currentColumns.map((column) => `<th scope="col" aria-sort="${state.sort === column.key ? state.direction > 0 ? 'ascending' : 'descending' : 'none'}"><button type="button" data-sort="${escape(column.key)}">${escape(column.label)}<span aria-hidden="true">${state.sort === column.key ? state.direction > 0 ? '↑' : '↓' : '↕'}</span></button></th>`).join('')}</tr>`;
    table.querySelector('tbody').innerHTML = visible.length ? visible.map((row, index) => `<tr class="click-row${isSelected(row) ? ' selected' : ''}" data-row="${index}" tabindex="0" aria-label="Details zu ${escape(row.url)} öffnen">${currentColumns.map((column, columnIndex) => `<td>${cell(column, row, columnIndex)}</td>`).join('')}</tr>`).join('') : `<tr><td class="empty-cell" colspan="${currentColumns.length}"><strong>${state.search || state.filter !== 'all' ? 'Keine passenden Ergebnisse' : state.crawl ? 'Noch keine Daten in diesem Bereich' : 'Starte einen Crawl, um Ergebnisse zu sehen'}</strong>${escape(state.search || state.filter !== 'all' ? 'Ändere den Filter oder den Suchbegriff.' : inputEmptyNote || emptyNote || (['linkmetrics', 'structured'].includes(state.section) ? state.analysisError || 'Diese Auswertung wird nach Abschluss oder Stopp des Crawls berechnet.' : state.section === 'images' ? 'Bildreferenzen erscheinen nach der Auswertung von HTML-Seiten.' : state.section === 'external' ? 'Externe Links erscheinen, sobald sie im HTML entdeckt werden.' : 'Nur tatsächlich erfasste Daten werden angezeigt.'))}</td></tr>`;
    table.querySelectorAll('[data-recrawl-url]').forEach(input => {
      input.addEventListener('click', event => event.stopPropagation());
      input.addEventListener('keydown', event => event.stopPropagation());
      input.addEventListener('change', () => setRecrawlSelected(input.dataset.recrawlUrl, input.checked));
    });
    table.querySelectorAll('[data-sort]').forEach((button) => button.addEventListener('click', () => {
      state.direction = state.sort === button.dataset.sort ? -state.direction : 1;
      state.sort = button.dataset.sort;
      state.page = 0;
      renderTable();
    }));
    table.querySelectorAll('[data-row]').forEach((rowElement) => {
      const open = () => {
        const row = visible[Number(rowElement.dataset.row)];
        if (state.section === 'inputs') { state.selected = null; state.selectedResponse = null; state.selectedInput = row; renderInspector(); renderTable(); $('inspector').scrollIntoView({ behavior: 'smooth', block: 'nearest' }); return; }
        if (state.section === 'responses') { selectResponse(row); return; }
        if (state.section === 'fragments' && !row._page) { message('Kein gespeicherter Dokumentdatensatz zur Quell-URL verfügbar.', true); return; }
        const page = row._page || state.pageMap.get(row.url) || row;
        selectPage(page);
        if (domSections.includes(state.section) || ['linkdom', 'fragments'].includes(state.section)) { state.inspectorTab = 'dom'; renderInspector(); }
      };
      rowElement.addEventListener('click', open);
      rowElement.addEventListener('keydown', (event) => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); open(); } });
    });
    $('result-count').textContent = `${number(rows.length)} Ergebnisse`;
    $('page-label').textContent = rows.length ? `${number(offset + 1)}–${number(Math.min(offset + state.pageSize, rows.length))} von ${number(rows.length)}` : 'Keine Ergebnisse';
    $('page-prev').disabled = state.page === 0;
    $('page-next').disabled = state.page + 1 >= pages;
    renderControls();
  }

  function selectSection(section) {
    if (!sections[section]) return;
    if (resourceSection() && section !== 'resource-discovery') invalidateResourceRequest();
    state.section = section;
    state.page = 0;
    state.filter = 'all';
    state.search = '';
    state.sort = section === 'inputs' ? 'list_index' : 'url';
    state.direction = 1;
    $('table-search').value = '';
    $('table-filter').innerHTML = filterOptions().map(([value, label]) => `<option value="${escape(value)}">${escape(label)}</option>`).join('');
    document.querySelectorAll('[data-section]').forEach((button) => {
      button.classList.toggle('active', button.dataset.section === section);
      if (button.dataset.section === section) button.setAttribute('aria-current', 'page'); else button.removeAttribute('aria-current');
    });
    renderSection();
    if (policySection() && state.crawl && !state.policy && !state.policyToken) loadPolicyPage();
    if (contentSection() && state.crawl && !active() && !state.content.inventory && !state.content.inventoryToken) loadContentInventory();
    if (resourceSection() && state.crawl && !state.resources.result && !state.resources.token) loadResourcePage();
  }

  function renderSection() {
    const [eyebrow, title, description] = sections[state.section];
    $('section-eyebrow').textContent = eyebrow;
    $('section-heading').textContent = title;
    $('section-description').textContent = description;
    $('overview-view').hidden = state.section !== 'overview';
    $('graph-view').hidden = state.section !== 'graph';
    $('policy-view').hidden = !policySection();
    $('table-view').hidden = ['overview', 'graph', 'content','resource-discovery'].includes(state.section) || policySection();
    $('content-view').hidden = state.section !== 'content';
    $('resource-discovery-view').hidden = state.section !== 'resource-discovery';
    $('export-csv').disabled = policySection() ? !policyVisibleRows().length || Boolean(state.policyToken || state.policyError) : (state.section === 'inputs' ? !getRows().length : !state.pages.length && !state.issues.length) || state.section === 'responses' && state.responses == null || state.section === 'fragments' && (!state.fragments || !state.fragments.enabled);
    $('export-csv').title = policySection() ? 'Nur gefilterte Vorkommen der aktuell geladenen Ledger-Seite exportieren' : 'Alle aktuell gefilterten Ergebnisse dieses Bereichs exportieren';
    if (state.section==='resource-discovery') $('export-csv').disabled=true;
    $('export-resources').hidden = !['inputs', 'pdf', 'imagefiles', 'responses', 'fragments', 'linkdom', ...domSections].includes(state.section);
    $('export-resources').disabled = !state.crawl || Boolean(actionBusy() || active());
    $('export-resources').textContent = state.section === 'inputs' ? '↓ Alle Eingabezeilen CSV' : domSections.includes(state.section) ? '↓ Alle DOM-Beobachtungen CSV' : state.section === 'pdf' ? '↓ PDF-Details CSV' : state.section === 'responses' ? '↓ Alle Antworten CSV' : state.section === 'fragments' ? '↓ Alle Fragmentziele CSV' : state.section === 'linkdom' ? '↓ Alle Linkvorkommen CSV' : '↓ Bilddateien CSV';
    if (state.section === 'overview') renderOverview();
    else if (state.section === 'graph') renderGraph();
    else if (policySection()) renderPolicyLedger();
    else if (state.section === 'content') renderContent();
    else if (state.section === 'resource-discovery') renderResourceLedger();
    else renderTable();
  }

  function selectResponse(response) {
    state.selectedInput = null;
    state.selected = null;
    state.selectedResponse = response;
    renderInspector();
    renderTable();
    $('inspector').scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }

  function renderResponseInspector(response) {
    $('inspector').hidden = false;
    $('inspector-tabs').hidden = true;
    $('inspector').querySelector('.eyebrow').textContent = 'HTTP RESPONSE OBSERVATION';
    $('inspector-url').textContent = response.response_url;
    const fields = [
      ['Anfrage-URL', response.requested_url], ['Antwort-URL', response.response_url],
      ['Antwortposition', response.response_index], ['HTTP beobachtet', response.status],
      ['Befund', response.capture_kind === 'redirect_hop' ? 'Zwischenredirect' : 'Terminale native Antwort'],
      ['Location', response.location ?? 'Keine erfasst'],
      ['Content-Type', response.content_type ?? 'Unbekannt (Header nicht gespeichert)'],
      ['Bodybytes', response.body_bytes == null ? 'Unbekannt (Body nicht gespeichert)' : `${number(response.body_bytes)} Bytes`],
      ['Body vollständig', response.body_complete == null ? 'Unbekannt' : response.body_complete ? 'Ja' : 'Nein'],
      ['HTTP-Header', response.headers == null ? 'Unbekannt (Header nicht gespeichert)' : Object.entries(response.headers).map(([name, value]) => `${name}: ${value}`).join('\n') || 'Keine Header erfasst'],
      ['HTML-Metadaten verfügbar', response.document_metadata_available ? 'Dokumentbefund zur Anfrage separat verfügbar' : 'Keine HTML-Dokumentmetadaten für diese Antwort angegeben']
    ];
    const page = state.pageMap.get(response.requested_url);
    const element = $('inspector-content');
    element.innerHTML = `<p class="detail-empty">Diese Ansicht enthält ausschließlich den Antwortbefund. Metadaten des finalen Dokuments werden keinem Zwischenredirect zugerechnet.</p><dl class="details-grid">${fields.map(([name, value]) => `<dt>${escape(name)}</dt><dd>${escape(value ?? 'Unbekannt')}</dd>`).join('')}</dl>${page ? '<button type="button" class="button small" data-response-document>Dokumentdatensatz zur Anfrage öffnen</button>' : ''}`;
    element.querySelector('[data-response-document]')?.addEventListener('click', () => selectPage(page));
  }

  function selectPage(page) {
    state.selectedInput = null;
    state.selectedResponse = null;
    state.selected = normalPage(page);
    state.inspectorTab = 'details';
    renderInspector();
    if (!['overview', 'graph'].includes(state.section)) renderTable();
    $('inspector').scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }

  function renderInputInspector(row) {
    $('inspector').hidden = false;
    $('inspector-tabs').hidden = true;
    $('inspector').querySelector('.eyebrow').textContent = 'DEKLARIERTE EINGABEHERKUNFT';
    $('inspector-url').textContent = row.normalized_url ?? row.raw_value ?? 'Eingabezeile';
    const fields = [['Liste', row.list_index], ['Parserprofil', row._import.engine], ['Quellhash', row.input_sha256], ['Quellbytes', row._import.input_bytes], ['Quellindex', row.source_index], ['Quellzeile', row.source_row], ['Physische Zeile', row.source_line], ['Worksheet', row.sheet], ['Zelle', row.cell], ['Originalwert', row.raw_value === '' ? '[leer]' : row.raw_value], ['Normalisierte URL', row.normalized_url], ['Fragment', row.fragment], ['Kandidatenart', row.candidate_kind], ['Zelltyp', row.cell_type], ['Formelmetadaten', row.formula_metadata == null ? null : JSON.stringify(row.formula_metadata)], ['Importfehler', row.error ?? 'Keiner deklariert']];
    $('inspector-content').innerHTML = `<p class="detail-empty">Deklarierte Herkunft aus dem gespeicherten Importergebnis, keine signierte Bestätigung. Index 0 ist erhalten und bedeutet eine nicht erfasste Quellposition. Abrufbefunde sind ausschließlich im vollständigen Eingabezeilen-CSV ergänzt.</p><dl class="details-grid">${fields.map(([label, value]) => `<dt>${escape(label)}</dt><dd>${escape(value ?? 'Nicht erfasst')}</dd>`).join('')}</dl><details><summary>Importoptionen &amp; Grenzen</summary><pre class="list-import-details">${escape(JSON.stringify({ options: row._import.options, website_requests: row._import.website_requests, warnings: row._import.warnings, limitations: row._import.limitations }, null, 2))}</pre></details>`;
  }

  function renderInspector() {
    renderControls();
    if (state.selectedInput) { renderInputInspector(state.selectedInput); return; }
    if (state.selectedResponse) { renderResponseInspector(state.selectedResponse); return; }
    const page = state.selected;
    if (!page) { $('inspector').hidden = true; return; }
    $('inspector').hidden = false;
    $('inspector-tabs').hidden = false;
    $('inspector').querySelector('.eyebrow').textContent = 'URL INSPECTOR';
    $('inspector-url').textContent = page.url;
    document.querySelectorAll('[data-tab]').forEach((button) => button.setAttribute('aria-selected', String(button.dataset.tab === state.inspectorTab)));
    const element = $('inspector-content');
    if (state.inspectorTab === 'details') {
      const fields = [
        ...(observationAge(page.url) ? [['Alter des Befunds', observationAge(page.url)]] : []),
        ['HTTP initial / final', `${page.status ?? '—'} / ${effectiveStatus(page) ?? '—'}`],
        ['Finale URL', page.final_url || '—'], ['Content-Type', page.content_type || '—'],
        ['Antwortzeit', duration(page.response_ms)], ['Seitentitel', first(page.titles) || '—'],
        ['Meta Description', first(page.meta_descriptions) || '—'], ['H1', first(page.h1) || '—'],
        ['Pixelbreiten Titel / Description', `${JSON.stringify(page.title_pixel_widths || [])}\n${JSON.stringify(page.description_pixel_widths || [])}`],
        ['Text-Messprofil', page.text_metrics_engine || 'Keine Messung erfasst'],
        ['H2', first(page.h2) || '—'], ['Canonical', first(page.canonical) || '—'],
        ['Canonical aus HTML / HTTP', `${first(page.html_canonical) || '—'}\n${first(page.header_canonical) || '—'}`],
        ['Dokumentbasis', page.document_base_url || page.final_url || page.url],
        ['Pagination next / prev', `${first(page.pagination_next) || '—'}\n${first(page.pagination_prev) || '—'}`],
        ['AMP-Verweise', first(page.amphtml) || '—'],
        ['Meta Robots', first(page.meta_robots) || '—'], ['X-Robots-Tag', first(page.x_robots_tag) || '—'],
        ['Indexierbarkeit', page.indexable === true ? 'Indexierbar nach der lokalen Prüfung' : page.indexable === false ? 'Nicht indexierbar' : 'Noch nicht bestimmt'],
        ['Begründung', first(page.indexability_reasons) || '—'],
        ['Indexierungs-Bot', page.indexability_agent || 'Aus älterem Bericht nicht bekannt'],
        ['Wirksame Robots-Anweisungen', first(page.effective_robots) || 'Keine erfasst'],
        ['Robots-Quellen', array(page.robots_directives).map(entry => `${entry.source} · ${entry.agent || 'alle Bots'}: ${entry.raw}`).join('\n') || '—'],
        ['HTML-Sprache', page.language || '—'], ['Hreflang', page.hreflang.map((entry) => `${entry.language || entry.lang}: ${entry.url || entry.href}`).join('\n') || '—'],
        ['Wörter / Tiefe', `${page.word_count ?? '—'} / ${page.depth ?? '—'}`],
        ['Antwortgröße', page.size_bytes == null ? '—' : `${number(page.size_bytes)} Bytes${page.truncated ? ' · Antwort abgeschnitten' : ''}`],
        ['Strukturierte Daten', `${array(page.structured_data).length} JSON-LD-Blöcke · ${page.invalid_structured_data ?? 0} ungültig`],
        ['Weiterleitungen', page.redirect_chain.map((entry) => `${entry.status} ${entry.url} → ${entry.target}`).join('\n') || '—'],
        ['Robots.txt', page.blocked_by_robots ? 'Abruf ausgeschlossen' : page.robots_warning || 'Kein gemeldeter Ausschluss'],
        ['Abruffehler', page.error || '—'], ['Entdeckt auf', page.discovered_from || '—'],
        ['Renderstatus', page.rendered ? 'JavaScript-DOM erfasst' : page.renderer_error ? 'Rendering fehlgeschlagen' : 'Kein Rendering erfasst'],
        ['Browser-Version', page.browser_version || '—'],
        ['Render-Viewport', page.rendered_viewport ? JSON.stringify(page.rendered_viewport) : '—'],
        ['Render-Wartezeit', duration(page.render_wait_ms)],
        ['Renderer-Fehler', page.renderer_error || '—'], ['Seitenwarnungen', array(page.warnings).join('\n') || '—'],
        ['Gerenderte URL', page.rendered_url || '—'], ['Gerenderter HTTP-Status', page.rendered_status ?? '—'],
        ['Titel vor Rendering', page.raw_metadata ? first(page.raw_metadata.titles) || '[leer]' : '—'],
        ['H1 vor Rendering', page.raw_metadata ? first(page.raw_metadata.h1) || '[leer]' : '—'],
        ['Browser-Konsole', array(page.console_errors).length ? JSON.stringify(page.console_errors, null, 2) : 'Keine erfassten Fehler'],
        ['Browser-Requests', page.network_requests == null ? '—' : `${number(array(page.network_requests).length)} erfasst${page.network_events_dropped > 0 ? ` · ${number(page.network_events_dropped)} weitere Netzwerkereignisse nicht gespeichert (Aufzeichnung abgeschnitten)` : ''}`],
        ['Axe-Ergebnis', page.accessibility ? JSON.stringify(page.accessibility, null, 2) : 'Keine Prüfung erfasst'],
        ['Eigene Extraktionen', page.custom_extractions.length ? page.custom_extractions.map((entry) => `${entry.name || entry.selector} (${entry.method || 'css'}; ${entry.output || 'auto'}; ${entry.result_type || 'Typ unbekannt'}; ${entry.selector}${entry.attribute ? ` @${entry.attribute}` : ''}; ${entry.source || 'Quelle unbekannt'}; beobachtete Treffer ${entry.matched_count ?? 'unbekannt'}${entry.truncated ? '; Ergebnis abgeschnitten' : ''}): ${entry.error ? `Fehler: ${entry.error}` : array(entry.values).length ? array(entry.values).join(' | ') : '[keine Werte]'}`).join('\n') : 'Keine Extraktionen konfiguriert oder erfasst'],
        ['Extraktionen vor Rendering', page.raw_metadata ? array(page.raw_metadata.custom_extractions).map((entry) => `${entry.name || entry.selector}: ${array(entry.values).join(' | ') || '[keine Werte]'}`).join('\n') || '[keine Extraktionen]' : '—'],
        ['JavaScript-Auswertungen', page.custom_javascript_results.length ? page.custom_javascript_results.map(entry => `${entry.name} (${entry.value_type || 'Fehler'}; ${duration(entry.elapsed_ms)}): ${entry.error || JSON.stringify(entry.value)}`).join('\n') : 'Keine Auswertungen erfasst'],
        ['Browser-Cookies', page.browser_cookies.length ? JSON.stringify(page.browser_cookies, null, 2) : 'Keine erfasst; Aufnahme ist optional und pro Seitenaufruf isoliert'],
        ['HTTP-Header', Object.entries(page.headers || {}).map(([key, value]) => `${key}: ${value}`).join('\n') || '—']
      ];
      fields.push(['Native Policy-Aufzeichnung', page.policy ? page.policy.admission?.engine === nativePolicyEngine ? 'Gespeicherte native Task-Zulassung' : 'Unbekanntes gespeichertes Policy-Profil' : 'Nicht erfasst; keine nachträgliche Zulassung abgeleitet']);
      if (page.policy) {
        const policy = page.policy, admission = policy.admission || {};
        const permission = value => typeof value === 'boolean' ? value ? 'Ja' : 'Nein' : 'Nicht erfasst';
        fields.push(['Policy-Engine / SHA256', `${admission.engine || 'Nicht erfasst'}\n${admission.policy_sha256 || 'Nicht erfasst'}`],['Policy-Anfrage-URL / Rolle', `${admission.fetch_url || 'Nicht erfasst'}\n${admission.task_role || 'Nicht erfasst'}`],['Policy-Vorkommen / Quelle', `${admission.occurrence_id || 'Nicht erfasst'}\n${admission.source_kind || 'Nicht erfasst'}`],['Policy-Aufzeichnung geerbt', permission(admission.inherited)],['Policy-Seitentraversierung initial / effektiv', `${permission(admission.can_expand)} / ${permission(policy.effective_can_expand)}`],['Policy-XML-Discovery erlaubt', permission(admission.can_discover_xml)],['Policy-Quellkandidaten', `${policy.candidate_count ?? 'Nicht erfasst'} · Erfassung vollständig: ${permission(policy.candidates_complete)}\nQuellrevision ${policy.source_revision || 'Nicht erfasst'}`],['Policy-Redirect-Entscheidungen', JSON.stringify(policy.redirect_decisions || [],null,2)]);
      }
      if (isPDF(page)) {
        const pdf = page.pdf;
        fields.push(['PDF-Analyse', resourceStatus(pdf?.status)]);
        if (pdf) {
          const preview = value => value == null ? 'Unbekannt' : String(value).length > 4000 ? `${String(value).slice(0, 4000)}\n[UI-Vorschau auf 4000 Zeichen begrenzt; JSON-Export enthält den gespeicherten Befund]` : value;
          const observedBoolean = value => value == null ? 'Unbekannt' : value ? 'Ja' : 'Nein';
          fields.push(
            ['PDF-Engine / Version', `${pdf.engine || 'Unbekannt'} / ${pdf.version || 'Unbekannt'}`],
            ['PDF-Seitenzahl geprüft / deklariert', `${pdf.page_count ?? 'Unbekannt'} / ${pdf.declared_page_count ?? 'Unbekannt'}`],
            ['PDF-Verschlüsselung beobachtet', observedBoolean(pdf.encrypted)],
            ['PDF-Info-Metadaten', Object.entries(pdf.info_metadata || {}).map(([name, value]) => `${name}: ${preview(value)}`).join('\n') || 'Keine Info-Metadaten erfasst'],
            ['PDF-Text vollständig', observedBoolean(pdf.text_complete)],
            ['PDF-Textseiten verarbeitet', pdf.text_pages_processed ?? 'Unbekannt'],
            ['PDF-Textzeichen / Wörter', `${pdf.text_chars ?? 'Unbekannt'} / ${pdf.word_count ?? 'Unbekannt'}`],
            ['PDF-Texthash', pdf.text_hash || 'Nicht erfasst'],
            ['PDF-Textvorschau', preview(pdf.text_preview)],
            ['PDF-Textseiten', array(pdf.text_pages).slice(0, 50).map(entry => `Seite ${entry.page}: ${entry.chars ?? 'unbekannte'} Zeichen, ${entry.word_count ?? 'unbekannte'} Wörter${entry.truncated ? ' · Vorschau abgeschnitten' : ''}${entry.error ? ` · Fehler: ${entry.error}` : ''}\n${preview(entry.preview)}`).join('\n\n') + (array(pdf.text_pages).length > 50 ? '\n[UI zeigt die ersten 50 Textseiten; gespeicherte Daten im JSON-Export]' : '') || 'Keine Textseiten erfasst'],
            ['PDF-URI-Annotationen', array(pdf.annotation_links).slice(0, 100).map(entry => `Seite ${entry.page}: ${entry.raw_uri}\nNormalisierte HTTP(S)-URL: ${entry.url || '[nicht verfügbar]'}`).join('\n') + (array(pdf.annotation_links).length > 100 ? '\n[UI zeigt die ersten 100 Annotationen; gespeicherte Daten im JSON-Export]' : '') || 'Keine URI-Annotationen erfasst'],
            ['PDF-Annotationen vollständig', observedBoolean(pdf.annotations_complete)],
            ['PDF-Erreichte Grenzen', array(pdf.limits_reached).join('\n') || 'Keine gemeldet'],
            ['PDF-Warnungen', array(pdf.warnings).join('\n') || 'Keine gemeldet'],
            ['PDF-Fehler', [pdf.parse_error, ...array(pdf.errors)].filter(Boolean).join('\n') || 'Keine gemeldet']
          );
        }
      }
      if (isImageFile(page)) {
        const image = page.image_metadata;
        fields.push(['Bilddatei-Analyse', resourceStatus(image?.status)]);
        if (image) fields.push(
          ['Bilddatei-Engine / Format', `${image.engine || 'Unbekannt'} / ${image.format || 'Unbekannt'}`],
          ['Intrinsische Bildmaße', `${image.width ?? 'Unbekannt'} × ${image.height ?? 'Unbekannt'} ${image.dimension_unit || '[Einheit unbekannt]'}`],
          ['Bilddatei-Bytes erfasst', image.encoded_bytes ?? 'Unbekannt'],
          ['Bilddatei-Body vollständig', image.body_complete == null ? 'Unbekannt' : image.body_complete ? 'Ja' : 'Nein'],
          ['SVG-ViewBox', image.svg_view_box ? JSON.stringify(image.svg_view_box) : 'Nicht erfasst; keine Größenableitung'],
          ['Bilddatei-Warnungen', array(image.warnings).join('\n') || 'Keine gemeldet'],
          ['Bilddatei-Parserfehler', image.parse_error || 'Keine gemeldet']
        );
      }
      element.innerHTML = `<dl class="details-grid">${fields.map(([label, value]) => `<dt>${escape(label)}</dt><dd>${escape(value)}</dd>`).join('')}</dl>`;
      if (typeof page.screenshot_base64 === 'string' && /^[A-Za-z0-9+/=\r\n]+$/.test(page.screenshot_base64)) {
        const details = document.createElement('details');
        details.className = 'rendered-screenshot';
        const summary = document.createElement('summary'); summary.textContent = 'Erfassten Browser-Screenshot anzeigen';
        details.append(summary);
        details.addEventListener('toggle', () => {
          if (details.open && !details.querySelector('img')) {
            const image = document.createElement('img');
            image.src = `data:image/png;base64,${page.screenshot_base64}`;
            image.alt = `Lokal erfasster Browser-Screenshot von ${page.url}`;
            details.append(image);
          }
        });
        element.append(details);
      }
    } else if (state.inspectorTab === 'dom') {
      const facts = page.dom_metadata;
      const fields = [
        ['DOM-Engine', facts?.engine || 'Nicht erfasst'], ['Heading-Text-/Alt-Profil', facts?.heading_text_engine || 'Nicht erfasst'], ['DOM-Quelle', facts?.source || 'Nicht erfasst'],
        ['DOM-Sammlung vollständig', facts == null ? 'Nicht erfasst' : facts.complete ? 'Ja' : 'Nein'],
        ['Vollständiger Prüfbefund', facts == null ? 'Nicht erfasst' : completeDOM(page) ? 'Ja' : 'Nein / unklar; Capture-, HTTP- und Rendererstatus prüfen'],
        ['DOM-Grenzen erreicht', facts == null ? 'Nicht erfasst' : first(facts.limits_reached) || 'Keine gemeldet'],
        ['Überschriften / Keywords / Ziel-IDs', facts == null ? 'Nicht erfasst' : `${array(facts.headings).length} / ${array(facts.meta_keywords).length} / ${array(facts.fragment_targets).length}`]
      ];
      const preview = (name, values) => {
        if (values == null) return `<p class="detail-empty">${escape(name)}: Nicht erfasst.</p>`;
        const rows = array(values), text = JSON.stringify(rows.slice(0, 100), null, 2);
        return `<details class="dom-preview"><summary>${escape(name)} · ${number(rows.length)} erfasst</summary><pre>${escape(text.slice(0, 30000))}</pre>${rows.length > 100 || text.length > 30000 ? '<p class="detail-empty">UI-Vorschau auf 100 Einträge / 30000 Zeichen begrenzt; der JSON-Export enthält die gespeicherten Beobachtungen.</p>' : ''}</details>`;
      };
      element.innerHTML = `<p class="detail-empty">Beobachtungen aus dem reparierten HTML5-DOM. DOM-Pfad und Reihenfolge sind keine Quellzeilen oder Bildschirmkoordinaten. Kandidaten-Syntax belegt keine Browserauswahl.</p><dl class="details-grid">${fields.map(([name, value]) => `<dt>${escape(name)}</dt><dd>${escape(value)}</dd>`).join('')}</dl>${preview('Überschriftenfolge', facts?.headings)}${preview('Meta Keywords', facts?.meta_keywords)}${preview('Fragmentziele im Dokument', facts?.fragment_targets)}${preview('Weitere Navigation', facts?.navigation_candidates)}${preview('Responsive Bildvorkommen', facts?.responsive_images)}${preview('Linkvorkommen und Position', page.links)}${preview('DOM vor Rendering', page.raw_metadata?.dom_metadata ? [page.raw_metadata.dom_metadata] : null)}${preview('Fragmentreferenzen dieser Quell-URL', state.fragments?.enabled ? array(state.fragments.observations).filter(record => record.source_url === page.url) : null)}`;
    } else if (state.inspectorTab === 'outlinks') {
      element.innerHTML = page.links.length ? `<ul class="detail-links">${page.links.map((link) => `<li>${safeLink(linkURL(link))}${typeof link === 'object' ? `<small>${escape(link.text || '(ohne Linktext)')} · ${link.internal ? 'intern' : 'extern'}${link.nofollow ? ' · nofollow' : ''}${link.kind ? ` · ${escape(link.kind)}` : ''}</small>` : ''}</li>`).join('')}</ul>` : '<p class="detail-empty">Keine ausgehenden Links erfasst.</p>';
    } else if (state.inspectorTab === 'inlinks') {
      const sources = internals().flatMap((source) => source.links.filter((link) => linkURL(link) === page.url || linkURL(link) === page.final_url).map((link) => ({ url: source.url, text: typeof link === 'object' ? link.text : '' })));
      element.innerHTML = sources.length ? `<ul class="detail-links">${sources.map((source) => `<li>${safeLink(source.url)}<small>${escape(source.text || '(ohne Linktext)')}</small></li>`).join('')}</ul>` : '<p class="detail-empty">Keine eingehenden Links aus den bislang erfassten Seiten. Die Start-URL kann ohne interne Eingangslinks erscheinen.</p>';
    } else {
      const issues = pageIssues(page.url);
      element.innerHTML = issues.length ? `<ul class="detail-links">${issues.map((issue) => `<li><span class="badge ${issue.severity === 'error' ? 'danger' : issue.severity === 'warning' ? 'warning' : 'neutral'}">${escape(issue.severity)}</span> ${escape(issueTitle(issue.code))}<small>${escape(issue.message || '')}</small>${array(issue.related_urls).length ? `<small>${array(issue.related_urls).map(safeLink).join('<br>')}</small>` : ''}</li>`).join('')}</ul>` : '<p class="detail-empty">Für diese URL wurden bislang keine Probleme gemeldet.</p>';
    }
  }

  function safeLink(url) {
    try {
      const parsed = new URL(url);
      if (['http:', 'https:'].includes(parsed.protocol)) return `<a href="${escape(parsed.href)}" target="_blank" rel="noopener noreferrer">${escape(url)}</a>`;
    } catch (_) { /* Display invalid or unsupported schemes as plain text. */ }
    return escape(url);
  }

  function renderGraph() {
    const element = $('graph-view');
    const all = internals();
    if (!all.length) { element.innerHTML = '<div class="graph-empty">Der Linkgraph erscheint nach den ersten abgerufenen Seiten.</div>'; return; }
    const pages = [...all].sort((a, b) => (a.depth || 0) - (b.depth || 0)).slice(0, 70);
    const analysisGraph = state.analysis?.graph;
    element.innerHTML = `<div class="graph-container"><p class="graph-note">${number(pages.length)} von ${number(all.length)} internen URLs · Ausschnitt mit maximal 70 Knoten · Knoten anklicken für Seitendetails${analysisGraph ? `<br>${number(array(analysisGraph.edges).length)} beobachtete Ankerverbindungen · ${number(array(analysisGraph.orphan_candidates).length)} URLs ohne beobachtete Eingangslinks · Vollständigkeit der gesamten Website nicht nachgewiesen` : ''}</p><svg viewBox="0 0 950 460" role="img" aria-label="Ausschnitt der internen Linkstruktur"></svg><div class="graph-legend"><span><i></i>URL ohne gemeldeten HTTP-Fehler</span><span class="bad"><i></i>HTTP- oder Abruffehler</span></div></div>`;
    const svg = element.querySelector('svg');
    const NS = 'http://www.w3.org/2000/svg';
    const make = (tag, attributes) => { const node = document.createElementNS(NS, tag); Object.entries(attributes).forEach(([key, value]) => node.setAttribute(key, String(value))); return node; };
    const groups = new Map();
    pages.forEach((page) => { const depth = Math.min(4, Number(page.depth) || 0); const group = groups.get(depth) || []; group.push(page); groups.set(depth, group); });
    const positions = new Map();
    const orderedGroups = [...groups.entries()].sort((a, b) => a[0] - b[0]);
    orderedGroups.forEach(([depth, group], layer) => {
      const x = orderedGroups.length === 1 ? 475 : 80 + layer * 790 / Math.max(1, orderedGroups.length - 1);
      group.forEach((page, index) => positions.set(page.url, { x, y: 50 + (index + .5) * 350 / group.length, depth }));
      const label = make('text', { x, y: 435, 'text-anchor': 'middle', fill: '#95a68d', 'font-size': 11, 'font-family': 'Segoe UI, sans-serif' });
      label.textContent = `Tiefe ${depth}${depth === 4 ? '+' : ''}`;
      svg.append(label);
    });
    let edges = 0;
    const graphEdges = analysisGraph ? array(analysisGraph.edges) : pages.flatMap((page) => page.links.filter((link) => typeof link === 'string' || !link.kind || link.kind === 'anchor').map((link) => ({ source: page.url, target: linkURL(link) })));
    for (const edge of graphEdges) {
      const from = positions.get(edge.source), to = positions.get(edge.target);
      if (!from || !to || edges > 600) continue;
      svg.append(make('path', { d: `M ${from.x} ${from.y} C ${(from.x + to.x) / 2} ${from.y}, ${(from.x + to.x) / 2} ${to.y}, ${to.x} ${to.y}`, stroke: '#d3dec8', 'stroke-width': 1, fill: 'none', opacity: .6 }));
      edges += 1;
    }
    for (const page of pages) {
      const position = positions.get(page.url);
      const group = make('g', { class: 'graph-node', tabindex: 0, role: 'button', 'aria-label': `Details zu ${page.url}` });
      const title = make('title', {}); title.textContent = page.url; group.append(title);
      group.append(make('circle', { cx: position.x, cy: position.y, r: page.depth === 0 ? 7 : 4.5, fill: effectiveStatus(page) >= 400 || page.error ? '#bd7568' : '#658e64', stroke: '#fff', 'stroke-width': 1.5 }));
      if (pages.length < 22) {
        const label = make('text', { x: position.x + 10, y: position.y + 3, fill: '#6d8063', 'font-size': 9, 'font-family': 'Consolas, monospace' });
        let path = page.url; try { path = new URL(page.url).pathname; } catch (_) { /* URL label already usable. */ }
        label.textContent = path.length > 26 ? `${path.slice(0, 23)}…` : path;
        group.append(label);
      }
      group.addEventListener('click', () => selectPage(page));
      group.addEventListener('keydown', (event) => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); selectPage(page); } });
      svg.append(group);
    }
  }

  function render() { renderMetrics(); renderSection(); }

  function downloadBlob(blob, filename) {
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    document.body.append(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  function exportCSV() {
    if (policySection()) { exportPolicyPage(); return; }
    if (contentSection()) { exportContentRows(); return; }
    if (state.section==='resource-discovery') return;
    const original = state.section;
    if (['overview', 'graph'].includes(original)) state.section = 'internal';
    const cols = columns();
    const rows = filteredRows();
    const operation = state.crawl?.operation;
    const selected = operation?.coverage === 'mixed_age_selected_recrawl';
    const targets = new Map(array(operation?.targets).map(target => [target.requested_url, target]));
    const metadata = row => {
      const page = ['inputs', 'responses'].includes(state.section) ? null : recrawlPage(row);
      const target = page ? targets.get(page.url) : null;
      const selectedState = selected && target ? target.state || '' : '';
      const basis = !page ? 'unknown_or_mixed_row_identity' : ['issues', 'linkmetrics', 'structured'].includes(state.section) ? 'derived_report_observation' : !operation ? 'captured' : !selected ? 'unknown_operation' : selectedState === 'committed' ? 'refreshed_in_selected_job' : selectedState === 'pending' ? 'selected_pending_not_refreshed' : 'inherited_from_parent';
      return { coverage: operation?.coverage ?? 'crawl_snapshot', basis, selectedState, parentHash: operation?.parent?.snapshot_sha256 ?? '', fetchedAt: page?.fetched_at ?? '', requestedURL: page?.url ?? '' };
    };
    for (const [key, label] of [['coverage', 'Crawl Coverage'], ['basis', 'Observation Basis'], ['selectedState', 'Selected State'], ['parentHash', 'Parent Snapshot SHA256'], ['fetchedAt', 'Fetched At'], ['requestedURL', 'Observation Requested URL']]) cols.push(textColumn('observation_' + key, label, row => metadata(row)[key]));
    const csvCell = (input) => { let value = String(input ?? ''); if (typeof input !== 'number' && typeof input !== 'boolean' && /^[\s\uFEFF]*[=+@-]/u.test(value)) value = `'${value}`; return `"${value.replace(/"/g, '""')}"`; };
    const data = [cols.map((column) => csvCell(column.label)).join(','), ...rows.map((row) => cols.map((column) => csvCell(column.value(row))).join(','))].join('\r\n');
    const section = state.section;
    state.section = original;
    downloadBlob(new Blob(['\uFEFF', data], { type: 'text/csv;charset=utf-8' }), `open-seo-spider-${section}-${state.crawl?.id || 'export'}.csv`);
    message(`${number(rows.length)} gefilterte Datensätze als CSV exportiert.`);
  }

  async function serverDownload(endpoint, filename) {
    if (!state.crawl) return;
    try {
      const response = await fetch(`/api/crawls/${encodeURIComponent(state.crawl.id)}/${endpoint}`);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      downloadBlob(await response.blob(), filename);
    } catch (error) { message(`Export fehlgeschlagen: ${error.message}`, true); }
  }

  async function importProject(file) {
    if (!file) return;
    $('import-project').disabled = true;
    try {
      const report = JSON.parse(await file.text());
      if (!report || !Array.isArray(report.pages) || !report.config || !report.summary) throw new Error('Die Datei ist kein vollständiger Open SEO Spider-Bericht.');
      const result = await request('/api/crawls/import', { method: 'POST', body: JSON.stringify(report) });
      await loadHistory();
      await selectCrawl(result.id);
      $('crawl-history').value = result.id;
      message('Crawl-Bericht lokal importiert.');
    } catch (error) { message(`Der Bericht konnte nicht importiert werden: ${error.message}`, true); }
    finally { $('import-project').disabled = false; $('import-file').value = ''; }
  }

  function openSettings() {
    invalidatePolicyPreview();
    const form = $('settings-form');
    for (const [key, value] of Object.entries(state.settings)) {
      const element = form.elements.namedItem(key);
      if (!element) continue;
      if (element.type === 'checkbox') element.checked = Boolean(value); else element.value = value;
    }
    toggleRendererSettings(state.settings.javascript_rendering);
    $('settings-error').hidden = true;
    $('settings-dialog').showModal();
  }

  function saveSettings(event) {
    event.preventDefault();
    const form = $('settings-form');
    const advancedText = { include_patterns_text: form.elements.include_patterns_text.value, exclude_patterns_text: form.elements.exclude_patterns_text.value, custom_extractions_json: form.elements.custom_extractions_json.value, custom_javascript_json: form.elements.custom_javascript_json.value, audit_thresholds_json: form.elements.audit_thresholds_json.value, pdf_analysis_json: form.elements.pdf_analysis_json.value, link_position_rules_json: form.elements.link_position_rules_json.value, validate_fragments: form.elements.validate_fragments.checked, crawl_policy_enabled: form.elements.crawl_policy_enabled.checked, crawl_policy_json: form.elements.crawl_policy_json.value, live_policy_limits_json: form.elements.live_policy_limits_json.value, javascript_rendering: form.elements.javascript_rendering.checked, content_capture_enabled:form.elements.content_capture_enabled.checked, content_capture_json:form.elements.content_capture_json.value,resource_discovery_enabled:form.elements.resource_discovery_enabled.checked,resource_discovery_json:form.elements.resource_discovery_json.value,resource_discovery_limits_json:form.elements.resource_discovery_limits_json.value };
    try { parseAdvanced(advancedText); validateRendererSettings({ javascript_rendering: form.elements.javascript_rendering.checked, renderer_endpoint: form.elements.renderer_endpoint.value.trim(), renderer_wait_ms: Number(form.elements.renderer_wait_ms.value), timeout_seconds: Number(form.elements.timeout_seconds.value) }); }
    catch (error) {
      $('settings-error').textContent = error.message;
      $('settings-error').hidden = false;
      $('advanced-config').open = true;
      $('settings-error').scrollIntoView({block:'nearest'});
      return;
    }
    state.settings = { concurrency: Number(form.elements.concurrency.value), timeout_seconds: Number(form.elements.timeout_seconds.value), max_pages: Number(form.elements.max_pages.value), max_depth: Number(form.elements.max_depth.value), user_agent: form.elements.user_agent.value.trim(), indexability_agent: form.elements.indexability_agent.value.trim(), obey_robots: form.elements.obey_robots.checked, follow_subdomains: form.elements.follow_subdomains.checked, delay_ms: Number(form.elements.delay_ms.value), max_body_mb: Number(form.elements.max_body_mb.value), check_external: form.elements.check_external.checked, javascript_rendering: form.elements.javascript_rendering.checked, renderer_endpoint: form.elements.renderer_endpoint.value.trim(), renderer_wait_ms: Number(form.elements.renderer_wait_ms.value), renderer_accessibility: form.elements.renderer_accessibility.checked, renderer_screenshot: form.elements.renderer_screenshot.checked, renderer_collect_cookies: form.elements.renderer_collect_cookies.checked };
    Object.assign(state.settings, advancedText);
    invalidateContentRequests('Die Konfiguration wurde geändert. Gespeicherte Quellen bleiben unverändert.');
    try { localStorage.setItem('open-seo-spider-settings', JSON.stringify(state.settings)); } catch (_) { /* Crawl settings still apply when storage is unavailable. */ }
    $('settings-dialog').close();
    invalidatePolicyPreview();
    renderControls();
    if (contentSection()) renderContent();
  }

  function restoreSettings() {
    try {
      const saved = JSON.parse(localStorage.getItem('open-seo-spider-settings') || 'null');
      if (saved && typeof saved === 'object') {
        const positive = (value, fallback, max) => Number.isInteger(Number(value)) && Number(value) > 0 && Number(value) <= max ? Number(value) : fallback;
        const nonnegative = (value) => Number.isInteger(Number(value)) && Number(value) >= 0 ? Number(value) : 0;
        state.settings = { concurrency: positive(saved.concurrency, 8, 64), timeout_seconds: positive(saved.timeout_seconds, 20, 300), max_pages: nonnegative(saved.max_pages), max_depth: nonnegative(saved.max_depth), user_agent: String(saved.user_agent || 'Open SEO Spider/0.1').slice(0, 256), obey_robots: saved.obey_robots !== false, follow_subdomains: saved.follow_subdomains === true, delay_ms: nonnegative(saved.delay_ms ?? 100), max_body_mb: positive(saved.max_body_mb, 5, 100), check_external: saved.check_external === true, javascript_rendering: saved.javascript_rendering === true, renderer_endpoint: String(saved.renderer_endpoint || 'http://127.0.0.1:8790/render'), renderer_wait_ms: nonnegative(saved.renderer_wait_ms ?? 500), renderer_accessibility: saved.renderer_accessibility === true, renderer_screenshot: saved.renderer_screenshot === true };
        Object.assign(state.settings, { indexability_agent: String(saved.indexability_agent || 'googlebot'), renderer_collect_cookies: saved.renderer_collect_cookies === true, custom_javascript_json: String(saved.custom_javascript_json || '[]'), audit_thresholds_json: String(saved.audit_thresholds_json || '{}'), pdf_analysis_json: String(saved.pdf_analysis_json || '{}'), include_patterns_text: String(saved.include_patterns_text || ''), exclude_patterns_text: String(saved.exclude_patterns_text || ''), custom_extractions_json: String(saved.custom_extractions_json || '[]'), validate_fragments: saved.validate_fragments === true, link_position_rules_json: String(saved.link_position_rules_json || '[]') });
        Object.assign(state.settings, { crawl_policy_enabled: saved.crawl_policy_enabled === true, crawl_policy_json: String(saved.crawl_policy_json || '{"scope":{"roots":[{"origin":"https://example.test","folders":["/"]}]}}'), live_policy_limits_json: String(saved.live_policy_limits_json || '{}') });
        Object.assign(state.settings, { content_capture_enabled:saved.content_capture_enabled === true, content_capture_json:String(saved.content_capture_json || '{"sources":["raw_http_html"],"include_selectors":["body"],"exclude_selectors":["script","style","template","noscript","nav","footer"]}') });
        Object.assign(state.settings,{resource_discovery_enabled:saved.resource_discovery_enabled===true,resource_discovery_json:String(saved.resource_discovery_json || '{}'),resource_discovery_limits_json:String(saved.resource_discovery_limits_json || '{}')});
      }
    } catch (_) { /* Invalid saved settings fall back to safe defaults. */ }
  }

  function parseAdvanced(settings) {
    parseCustomJavascript(settings.custom_javascript_json);
    let definitions;
    try { definitions = JSON.parse(settings.custom_extractions_json || '[]'); }
    catch (_) { throw new Error('Eigene Extraktionen müssen ein gültiges JSON-Array sein.'); }
    if (!Array.isArray(definitions) || definitions.length > 128) throw new Error('Eigene Extraktionen: Ein JSON-Array mit höchstens 128 Definitionen ist erforderlich.');
    const names = new Set();
    const custom_extractions = definitions.map((definition, index) => {
      if (!definition || typeof definition !== 'object' || Array.isArray(definition)) throw new Error(`Extraktion ${index + 1}: Ein Objekt mit name, selector und optional attribute ist erforderlich.`);
      if (typeof definition.name !== 'string' || !definition.name.trim()) throw new Error(`Extraktion ${index + 1}: Ein Feldname ist erforderlich.`);
      const name = definition.name.trim();
      if (Array.from(name).length > 128 || /[\x00-\x1f\x7f]/.test(name)) throw new Error(`Extraktion „${name}“: Name zu lang oder mit Steuerzeichen.`);
      const key = name.replace(/[A-Z]/g, character => character.toLowerCase());
      if (names.has(key)) throw new Error(`Der Extraktionsname „${name}“ ist mehrfach vorhanden.`);
      names.add(key);
      const method = definition.method || 'css';
      if (!['css', 'xpath', 'regex'].includes(method)) throw new Error(`Extraktion „${name}“: method muss css, xpath oder regex sein.`);
      const output = definition.output || 'auto';
      if (!['auto', 'text', 'inner_html', 'outer_html', 'function_value'].includes(output)) throw new Error(`Extraktion „${name}“: output muss auto, text, inner_html, outer_html oder function_value sein.`);
      if (method === 'regex' && output !== 'auto') throw new Error(`Extraktion „${name}“: Regex unterstützt nur output auto.`);
      if (method === 'css' && output === 'function_value') throw new Error(`Extraktion „${name}“: function_value benötigt XPath.`);
      if (typeof definition.selector !== 'string' || !definition.selector.trim() || new TextEncoder().encode(definition.selector).length > 8192) throw new Error(`Extraktion „${name}“: Ein Ausdruck mit höchstens 8192 UTF-8-Bytes ist erforderlich.`);
      const selector = definition.selector.trim();
      if (method === 'css') {
        try { document.createDocumentFragment().querySelector(selector); }
        catch (_) { throw new Error(`Extraktion „${name}“: Der Browser erkennt „${selector}“ nicht als gültigen CSS-Selektor. Der Crawler prüft die unterstützte CSS-Syntax zusätzlich.`); }
      }
      if (definition.attribute != null && (typeof definition.attribute !== 'string' || !definition.attribute.trim())) throw new Error(`Extraktion „${name}“: attribute muss null oder ein nicht leerer Attributname sein.`);
      if (method !== 'css' && definition.attribute != null) throw new Error(`Extraktion „${name}“: attribute ist nur für CSS verfügbar. XPath-Attribute mit /@name auswählen.`);
      if (definition.attribute != null && output !== 'auto') throw new Error(`Extraktion „${name}“: attribute kann nur mit output auto kombiniert werden.`);
      const max_results = definition.max_results ?? 10000, regex_group = definition.regex_group ?? null, regex_flags = definition.regex_flags ?? '', namespaces = definition.namespaces ?? {};
      if (!Number.isSafeInteger(max_results) || max_results < 1 || max_results > 100000) throw new Error(`Extraktion „${name}“: max_results muss zwischen 1 und 100000 liegen.`);
      if (regex_group != null && (!Number.isSafeInteger(regex_group) || regex_group < 0) || typeof regex_flags !== 'string' || /[^imsxU]/.test(regex_flags)) throw new Error(`Extraktion „${name}“: Ungültige Regex-Gruppe oder Flags.`);
      if (method !== 'regex' && (regex_group != null || regex_flags)) throw new Error(`Extraktion „${name}“: Regex-Optionen benötigen method regex.`);
      if (!namespaces || typeof namespaces !== 'object' || Array.isArray(namespaces) || Object.entries(namespaces).some(([prefix, uri]) => !prefix || typeof uri !== 'string' || !uri.trim()) || Object.keys(namespaces).length > 32) throw new Error(`Extraktion „${name}“: namespaces muss ein Objekt mit höchstens 32 Präfix-/URI-Paaren sein.`);
      if (method !== 'xpath' && Object.keys(namespaces).length) throw new Error(`Extraktion „${name}“: namespaces benötigt method xpath.`);
      return { name, selector, attribute: definition.attribute == null ? null : definition.attribute.trim(), method, output, max_results, regex_group, regex_flags, namespaces };
    });
    const lines = (value) => [...new Set(String(value || '').split(/\r?\n/).map((line) => line.trim()).filter(Boolean))];
    return { custom_extractions, audit_thresholds: parseAuditThresholds(settings.audit_thresholds_json), pdf_analysis: parsePdfAnalysis(settings.pdf_analysis_json), link_position_rules: parsePositionRules(settings.link_position_rules_json), validate_fragments: settings.validate_fragments === true, include_patterns: lines(settings.include_patterns_text), exclude_patterns: lines(settings.exclude_patterns_text), ...parseNativePolicy(settings), ...parseContentCapture(settings),...parseResourceDiscovery(settings) };
  }

  function policyObject(value, allowed, label) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error(`${label}: Ein JSON-Objekt ist erforderlich.`);
    for (const key of Object.keys(value)) if (!allowed.includes(key)) throw new Error(`${label}: Unbekanntes Feld ${key}.`);
    return value;
  }

  function parsePolicyConfig(text) {
    let config;
    try { config = JSON.parse(text || '{}'); } catch (_) { throw new Error('URL-Policy: Gültiges JSON ist erforderlich.'); }
    policyObject(config, ['engine','scope','query','rewrites','include_patterns','exclude_patterns','filter_identity','initial_inputs','limits'], 'URL-Policy');
    if (config.engine != null && config.engine !== 'open-seo-spider/crawl-policy-v1') throw new Error('URL-Policy: Unbekanntes Engine-Profil.');
    const strings = (value, label, max) => { if (!Array.isArray(value) || value.length > max || value.some(item => typeof item !== 'string')) throw new Error(`${label}: Ein String-Array mit höchstens ${max} Einträgen ist erforderlich.`); };
    const choice = (value, choices, label) => { if (value != null && !choices.includes(value)) throw new Error(`${label}: ${choices.join(', ')} erforderlich.`); };
    policyObject(config.scope, ['roots','outside_folder','external'], 'Policy-Scope');
    if (!Array.isArray(config.scope.roots) || !config.scope.roots.length || config.scope.roots.length > 32) throw new Error('Policy-Scope: 1 bis 32 explizite Origins sind erforderlich.');
    choice(config.scope.outside_folder, ['deny','check_once'], 'Außerhalb Ordner'); choice(config.scope.external, ['deny','check_once'], 'Externe Origins');
    for (const root of config.scope.roots) {
      policyObject(root, ['origin','subdomains','folders','exact_paths'], 'Policy-Root');
      let url; try { url = new URL(root.origin); } catch (_) { throw new Error('Policy-Root: Eine vollständige HTTP(S)-Origin ist erforderlich.'); }
      if (typeof root.origin !== 'string' || !['http:','https:'].includes(url.protocol) || url.username || url.password || url.pathname !== '/' || url.search || url.hash) throw new Error('Policy-Root: Origin ohne Inhaltspfad, Zugangsdaten, Query oder Fragment eingeben.');
      if (root.subdomains != null && typeof root.subdomains !== 'boolean') throw new Error('Policy-Root: subdomains muss true oder false sein.');
      strings(root.folders, 'Policy-Ordner', 128); if (root.exact_paths != null) strings(root.exact_paths, 'Exakte Pfade', 128);
      if (!root.folders.length && !root.exact_paths?.length) throw new Error('Policy-Root: Mindestens ein Ordner oder exakter Pfad ist erforderlich.');
      for (const path of [...root.folders, ...(root.exact_paths || [])]) if (!path.startsWith('/') || path.includes('?') || path.includes('#') || /[\u0000-\u0020\u007f]/.test(path)) throw new Error('Policy-Pfad: Absoluter kodierter Pfad ohne Query, Fragment oder Leerraum erforderlich.');
      if (root.folders.some(path => !path.endsWith('/'))) throw new Error('Policy-Ordner beginnen und enden mit /. exact_paths erlaubt einzelne Pfade.');
    }
    if (config.query != null) { policyObject(config.query, ['remove_keys','key_match','order'], 'Policy-Query'); if (config.query.remove_keys != null) strings(config.query.remove_keys, 'Query-Schlüssel', 128); choice(config.query.key_match, ['encoded_bytes','percent_decoded_utf8'], 'Query-Key-Matching'); choice(config.query.order, ['preserve','stable_encoded_key'], 'Query-Reihenfolge'); }
    for (const key of ['include_patterns','exclude_patterns']) if (config[key] != null) strings(config[key], key, 128);
    choice(config.filter_identity, ['fetch','discovered','both'], 'Filter-Identität');
    if (config.initial_inputs != null) { policyObject(config.initial_inputs, ['rewrite','filters'], 'Initiale Eingaben'); choice(config.initial_inputs.rewrite, ['preserve','apply'], 'Initiale Transformation'); choice(config.initial_inputs.filters, ['exempt','apply'], 'Initiale Filter'); }
    if (config.rewrites != null) {
      if (!Array.isArray(config.rewrites) || config.rewrites.length > 64) throw new Error('Policy-Rewrites: Höchstens 64 geordnete Regeln.');
      const ids = new Set();
      for (const rule of config.rewrites) {
        policyObject(rule, ['id','pattern','replacement'], 'Policy-Rewrite');
        if (typeof rule.id !== 'string' || !rule.id || ids.has(rule.id) || typeof rule.pattern !== 'string') throw new Error('Policy-Rewrite: Eindeutige id und Regex-String pattern erforderlich.');
        ids.add(rule.id); const replacement = rule.replacement;
        policyObject(replacement, ['kind', replacement?.kind === 'literal' ? 'text' : 'template'], 'Policy-Replacement');
        if (!['literal','captures'].includes(replacement.kind) || typeof replacement[replacement.kind === 'literal' ? 'text' : 'template'] !== 'string') throw new Error('Policy-Replacement: literal mit text oder captures mit template erforderlich.');
      }
    }
    if (config.limits != null) {
      const ceilings = { max_inputs:10000,max_input_bytes:8388608,max_url_bytes:16384,max_roots:32,max_scope_paths:128,max_query_keys:128,max_query_tokens:1024,max_rewrite_rules:64,max_filters:128,max_pattern_bytes:8192,max_total_pattern_bytes:65536,max_replacement_bytes:4096,max_capture_groups:32,max_trace_steps:256,max_output_bytes:16777216,regex_size_limit:1048576,dfa_size_limit:1048576 };
      policyObject(config.limits, Object.keys(ceilings), 'Policy-Preview-Grenzen');
      for (const [key, value] of Object.entries(config.limits)) if (!Number.isSafeInteger(value) || value < 1 || value > ceilings[key]) throw new Error(`Policy-Grenze ${key}: positive Ganzzahl bis ${ceilings[key]} erforderlich.`);
    }
    return config;
  }

  function parseNativePolicy(settings) {
    if (!settings.crawl_policy_enabled) return {};
    if (settings.javascript_rendering) throw new Error('Native URL-Policy und JavaScript-Rendering können nicht gemeinsam verwendet werden. Regeln für Browser-Requests sind hier noch nicht implementiert.');
    if (String(settings.include_patterns_text || '').trim() || String(settings.exclude_patterns_text || '').trim()) throw new Error('Mit nativer URL-Policy müssen die bisherigen URL-Include/Exclude-Felder leer sein. Regeln in der Policy konfigurieren; bestehende Werte werden nicht automatisch gelöscht.');
    const crawl_policy = parsePolicyConfig(settings.crawl_policy_json);
    let limits; try { limits = JSON.parse(settings.live_policy_limits_json || '{}'); } catch (_) { throw new Error('Native Ledger-Grenzen benötigen gültiges JSON.'); }
    policyObject(limits, ['max_row_bytes','max_batch_bytes','max_batch_occurrences','max_ledger_bytes','max_page_bytes'], 'Native Ledger-Grenzen');
    const effective = { max_row_bytes:262144,max_batch_bytes:33554432,max_batch_occurrences:100000,max_ledger_bytes:null,max_page_bytes:1048576,...limits };
    for (const [key, value] of Object.entries(effective)) if (!(key === 'max_ledger_bytes' && value === null) && (!Number.isSafeInteger(value) || value < 1)) throw new Error(`Ledger-Grenze ${key}: positive ganze Zahl${key === 'max_ledger_bytes' ? ' oder null' : ''} erforderlich.`);
    if (effective.max_row_bytes < 1024 || effective.max_row_bytes > 4194304 || effective.max_batch_bytes < effective.max_row_bytes || effective.max_batch_bytes > 268435456 || effective.max_batch_occurrences > 1000000 || effective.max_page_bytes < effective.max_row_bytes || effective.max_page_bytes > 16777216) throw new Error('Native Ledger-Grenzen: Zeile 1024–4194304 Bytes, Batch bis 256 MiB, maximal 1000000 Vorkommen, API-Seite bis 16 MiB; Batch/API-Seite müssen eine Zeile aufnehmen können.');
    return { crawl_policy, ...(Object.keys(limits).length ? { live_policy_limits: limits } : {}) };
  }

  function contentJSON(text, keys, label) {
    let value; try { value = JSON.parse(text || '{}'); } catch (_) { throw new Error(`${label}: Gültiges JSON ist erforderlich.`); }
    return policyObject(value, keys, label);
  }
  function contentLimits(value, ceilings, label) {
    policyObject(value, Object.keys(ceilings), label);
    for (const [key, limit] of Object.entries(value)) if (!Number.isSafeInteger(limit) || limit < 1 || limit > ceilings[key]) throw new Error(`${label} ${key}: positive ganze Zahl bis ${ceilings[key]} erforderlich.`);
  }
  function parseContentCapture(settings) {
    if (!settings.content_capture_enabled) return {};
    const config = contentJSON(settings.content_capture_json, ['sources','include_selectors','exclude_selectors','limits'], 'Text-Erfassung');
    if (config.sources != null && (!Array.isArray(config.sources) || !config.sources.length || config.sources.length > 2 || new Set(config.sources).size !== config.sources.length || config.sources.some(source => !['raw_http_html','rendered_dom_html'].includes(source)))) throw new Error('Text-Erfassung: sources muss eine oder zwei verschiedene Quellen raw_http_html/rendered_dom_html enthalten.');
    const limits = { max_html_utf8_bytes:8388608,max_text_utf8_bytes:524288,max_visited_dom_nodes:250000,max_dom_depth:1024,max_selectors:64,max_selector_utf8_bytes:4096,max_selector_checks:2000000,max_config_json_bytes:65536,max_output_json_bytes:4194304 };
    if (config.limits != null) contentLimits(config.limits, limits, 'Text-Erfassungsgrenze');
    const maxSelectors = config.limits?.max_selectors ?? 64, maxSelectorBytes = config.limits?.max_selector_utf8_bytes ?? 4096;
    for (const key of ['include_selectors','exclude_selectors']) if (config[key] != null && (!Array.isArray(config[key]) || config[key].some(selector => typeof selector !== 'string' || !selector.trim() || selector.includes('\0') || new TextEncoder().encode(selector).length > maxSelectorBytes))) throw new Error(`Text-Erfassung ${key}: Ein Array nicht leerer CSS-Strings innerhalb der UTF-8-Grenze ist erforderlich.`);
    if (config.include_selectors?.length === 0) throw new Error('Text-Erfassung: Mindestens ein Include-Selektor ist erforderlich.');
    if ((config.include_selectors?.length ?? 1) + (config.exclude_selectors?.length ?? 6) > maxSelectors) throw new Error('Text-Erfassung: Die gesamte Selektoranzahl überschreitet max_selectors.');
    if (new TextEncoder().encode(JSON.stringify(config)).length > (config.limits?.max_config_json_bytes ?? 65536)) throw new Error('Text-Erfassung: Konfiguration überschreitet max_config_json_bytes.');
    return { content_capture:config };
  }
  function parseContentAnalysisConfig() {
    const config = contentJSON($('content-analysis-config').value, ['case_behavior','stopwords','similarity','readability','limits'], 'Inhaltsanalyse');
    if (config.case_behavior != null && !['preserve','unicode_lowercase'].includes(config.case_behavior)) throw new Error('Inhaltsanalyse: case_behavior muss preserve oder unicode_lowercase sein.');
    if (config.stopwords != null && (!Array.isArray(config.stopwords) || config.stopwords.length > 1024 || config.stopwords.some(word => typeof word !== 'string' || !word || word.trim() !== word))) throw new Error('Inhaltsanalyse: Höchstens 1024 nicht leere, ungepolsterte Stopword-Strings. Rust prüft das genaue Tokenprofil.');
    const ceilings = { max_documents:128,max_input_bytes:8388608,max_text_bytes:524288,max_tokens_per_document:50000,max_token_bytes:4096,max_unique_ngrams_per_document:100000,max_ngram_key_bytes:16777216,max_pairs:8192,max_pair_feature_visits:20000000,max_position_samples:16,max_shared_shingle_samples:16,max_output_bytes:16777216 };
    if (config.limits != null) contentLimits(config.limits, ceilings, 'Analysegrenze');
    const maxTokens = config.limits?.max_tokens_per_document ?? 50000;
    if (config.similarity != null) {
      policyObject(config.similarity,['shingle_size','threshold','min_tokens'],'Textähnlichkeit');
      const n = config.similarity.shingle_size ?? 3, threshold = config.similarity.threshold ?? 0.9, minTokens = config.similarity.min_tokens ?? 10;
      if (![1,2,3].includes(n) || typeof threshold !== 'number' || !Number.isFinite(threshold) || threshold < 0 || threshold > 1 || !Number.isSafeInteger(minTokens) || minTokens < n || minTokens > maxTokens) throw new Error('Textähnlichkeit: shingle_size 1–3, threshold 0–1 und min_tokens mindestens shingle_size innerhalb des Tokenbudgets.');
    }
    if (config.readability != null) { policyObject(config.readability,['min_words'],'Lesbarkeit'); const minimum = config.readability.min_words ?? 30; if (!Number.isSafeInteger(minimum) || minimum < 1 || minimum > maxTokens) throw new Error('Lesbarkeit: min_words muss positiv und innerhalb des Tokenbudgets sein.'); }
    return config;
  }

  function parseResourceDiscovery(settings) {
    if (!settings.resource_discovery_enabled) return {};
    if (settings.javascript_rendering) throw new Error('Native Ressourcenermittlung und JavaScript-Rendering können nicht gemeinsam verwendet werden. Keine Einstellung wird automatisch ausgeschaltet.');
    const config=contentJSON(settings.resource_discovery_json,['resources','navigation','css','resource_scope','follow_rel_nofollow','follow_document_nofollow'],'Ressourcenermittlung');
    const booleanFields=(value,keys,label)=>{policyObject(value,keys,label);for(const [key,flag] of Object.entries(value))if(typeof flag!=='boolean')throw new Error(`${label}.${key}: true oder false erforderlich.`);};
    if(Object.hasOwn(config,'resources')) {
      policyObject(config.resources,['css','javascript','images','media','pdf','swf','other'],'Ressourcentypen');
      for(const [kind,rule] of Object.entries(config.resources)) booleanFields(rule,['store','fetch'],`Ressourcen.${kind}`);
    }
    if(Object.hasOwn(config,'navigation')) {
      policyObject(config.navigation,['internal_anchors','external_anchors','iframes','refresh','canonical','hreflang','pagination','amp','mobile_alternate','pdf_annotations'],'Navigation');
      for(const [kind,rule] of Object.entries(config.navigation)) {
        booleanFields(rule,['store','fetch','expand'],`Navigation.${kind}`);
        if(rule.expand===true && rule.fetch!==true)throw new Error(`Navigation.${kind}: expand true benötigt fetch true; fehlendes fetch in einer angegebenen Regel ist false.`);
      }
    }
    if(Object.hasOwn(config,'css')) booleanFields(config.css,['stylesheets','style_elements','style_attributes'],'CSS-Quellen');
    for(const key of ['follow_rel_nofollow','follow_document_nofollow']) if(Object.hasOwn(config,key) && typeof config[key]!=='boolean') throw new Error(`Ressourcenermittlung.${key}: true oder false erforderlich.`);
    if(Object.hasOwn(config,'resource_scope')) {
      policyObject(config.resource_scope,['external_origins'],'Ressourcen-Origin-Scope');
      if(Object.hasOwn(config.resource_scope,'external_origins')) {
        const origins=config.resource_scope.external_origins;
        if(!Array.isArray(origins) || origins.length>32)throw new Error('Ressourcen-Origin-Scope: Höchstens 32 vollständige HTTP(S)-Origins.');
        const seen=new Set();
        for(const origin of origins) {
          let url;try{if(typeof origin!=='string')throw new Error('type');url=new URL(origin);}catch(_){throw new Error('Ressourcen-Origin-Scope: Vollständige HTTP(S)-Origin erforderlich.');}
          if(!['http:','https:'].includes(url.protocol) || !url.hostname || origin.includes('*') || /\*|%2a/i.test(url.hostname) || url.username || url.password || url.pathname!=='/' || /[?#]/.test(url.href))throw new Error('Ressourcen-Origin-Scope: Nur exakte HTTP(S)-Origins ohne Wildcard, Inhaltspfad, Zugangsdaten, Query oder Fragment.');
          if(seen.has(url.origin))throw new Error('Ressourcen-Origin-Scope: Doppelte normalisierte Origin (Scheme, Host und Port).');seen.add(url.origin);
        }
      }
    }
    const limits=contentJSON(settings.resource_discovery_limits_json,['max_source_candidates','max_occurrence_utf8_bytes','max_source_batch_bytes','max_css_utf8_bytes','max_css_tokens','max_css_nesting','max_diagnostics','max_ledger_bytes'],'Ressourcenermittlungsgrenzen');
    const ceilings={max_source_candidates:1000000,max_occurrence_utf8_bytes:65536,max_source_batch_bytes:134217728,max_css_utf8_bytes:67108864,max_css_tokens:8000000,max_css_nesting:128,max_diagnostics:10000,max_ledger_bytes:Number.MAX_SAFE_INTEGER};
    for(const [key,value] of Object.entries(limits))if(!(key==='max_ledger_bytes' && value===null) && (!Number.isSafeInteger(value) || value<1 || value>ceilings[key]))throw new Error(`Ressourcengrenze ${key}: positive ganze Zahl bis ${ceilings[key]}${key==='max_ledger_bytes'?' oder null':''} erforderlich.`);
    return {resource_discovery:config,...(Object.keys(limits).length?{resource_discovery_limits:limits}:{})};
  }

  const nativeResourceEngine = 'open-seo-spider/resource-discovery-v1';
  const resourceSection = () => state.section === 'resource-discovery';
  const resourceView = () => $('resource-discovery-kind').value;
  const resourceBasis = value => value === 'current' ? 'Aktuelle Ledger-Beobachtung' : value === 'inherited' ? 'Geerbte Ledger-Historie' : 'Basis unbekannt';
  const resourceValue = value => value == null ? 'Unbekannt / nicht erfasst' : value === '' ? '[leer]' : typeof value === 'boolean' ? value ? 'Ja' : 'Nein' : String(value);
  function invalidateResourceRequest() {
    state.resources.revision += 1; state.resources.token?.controller.abort(); state.resources.token = null;
  }
  function resetResourceLedger() {
    invalidateResourceRequest(); Object.assign(state.resources,{ result:null, cursor:resourceView()==='transports'?'':0, back:[], error:'', selected:null, sort:0, direction:1 });
    if ($('resource-discovery-search')) { $('resource-discovery-search').value=''; $('resource-discovery-inspector').hidden=true; syncResourceFilter(); }
    if(resourceSection())renderResourceLedger();
  }
  function syncResourceFilter() {
    const options=resourceView()==='transports' ? [['all','Alle geladenen Transporte'],['retained','Antwort im Seitenbericht gespeichert'],['not_retained','Antwort nicht im Seitenbericht gespeichert'],['error','Transportfehler / Robots'],['metadata_unknown','Metadatenvollständigkeit unbekannt']] : [['all','Alle geladenen Vorkommen'],['granted','Eigenständiger Grant vorhanden'],['no_grant','Kein Grant für dieses Vorkommen'],['coalesced','Mit anderer Task zusammengeführt'],['source_incomplete','Quelleinheit unvollständig'],['current','Aktuelle Ledger-Beobachtung'],['inherited','Geerbte Ledger-Historie']];
    $('resource-discovery-filter').innerHTML=options.map(([value,label])=>`<option value="${escape(value)}">${escape(label)}</option>`).join('');
  }
  function validateResourceEnvelope(result,cursor,kind,expectedGeneration) {
    if(!result || typeof result.enabled!=='boolean' || typeof result.ledger_available!=='boolean' || typeof result.has_more!=='boolean' || result.website_requests!==0 || result.entity_bodies_included!==false || result.snapshot_scope!=='sqlite_read_snapshot')throw new Error('Nicht unterstützte Ressourcen-Snapshot-Bindung; keine Zielbefunde werden abgeleitet.');
    const rows=kind==='transports'?result.transports:result.observations;
    if(!Array.isArray(rows) || rows.length>200)throw new Error('Ungültige oder unbeschränkte Ressourcen-Seite.');
    if(result.enabled && result.engine!==nativeResourceEngine)throw new Error('Unbekanntes Ressourcen-Profil.');
    if(!result.ledger_available){if(result.snapshot!==null)throw new Error('Nicht verfügbares Ledger darf keinen verifizierten nativen Snapshot behaupten.');return;}
    const snapshot=result.snapshot;
    if(!snapshot || !Number.isSafeInteger(snapshot.generation) || snapshot.generation<1 || snapshot.generation!==result.crawl_generation || expectedGeneration!=null && snapshot.generation!==expectedGeneration || typeof snapshot.crawl_config_sha256!=='string' || !/^[a-fA-F0-9]{64}$/.test(snapshot.crawl_config_sha256) || snapshot.crawl_config_sha256!==result.crawl_config_sha256)throw new Error('Ressourcen-Snapshot passt nicht zur aktuellen Crawl-Generation oder vollständigen Job-Konfiguration.');
    const summary=result.checkpoint;
    if(!result.enabled || summary?.engine!==nativeResourceEngine || summary.config_sha256!==result.config_sha256 || typeof summary.complete!=='boolean' || !['config_sha256','rolling_sha256'].every(key=>typeof summary[key]==='string' && /^[a-fA-F0-9]{64}$/.test(summary[key])) || ['revision','decision_rows','committed_bytes','completed_transports','retained_responses','pending_transports','pending_expansions','incomplete_sources'].some(key=>!Number.isSafeInteger(summary[key]) || summary[key]<0))throw new Error('Ressourcen-Ledger ohne unterstützte Zusammenfassung oder erwartete Job-Config-Bindung.');
    if(summary.inherited_history!=null && (typeof summary.inherited_history.rows_available!=='boolean' || typeof summary.inherited_history.parent_snapshot_sha256!=='string'))throw new Error('Unbekannte Verfügbarkeit der geerbten Ledger-Historie.');
    if(kind==='transports') {
      let previous=cursor;
      for(const row of rows) {
        if(row.engine!==nativeResourceEngine || typeof row.requested_url!=='string' || !row.requested_url || row.requested_url<=previous || !Number.isSafeInteger(row.observed_generation) || row.observed_generation<1 || typeof row.retained_response!=='boolean' || !row.projection || row.projection.url!==row.requested_url || row.projection.resource_response!=null && row.projection.resource_response.engine!==nativeResourceEngine)throw new Error('Transport ohne unterstützte Anfrage-Identität, geordneten Cursor oder Metadatenprojektion.');
        previous=row.requested_url;
      }
      if(result.next_after_url!== (rows.length?rows.at(-1).requested_url:null) || result.has_more && !rows.length)throw new Error('Ungültiger Transport-Cursor.');
    } else {
      let previous=cursor;
      for(const row of rows) {
        const observed=row.occurrence, grant=row.decision?.grant;
        if(!Number.isSafeInteger(row.sequence) || row.sequence<=previous || !Number.isSafeInteger(row.generation) || row.generation<1 || !['current','inherited'].includes(row.basis) || !observed || ['occurrence_id','requested_source_url','source_document_url','raw_value','reference_kind','classification_basis','source_revision'].some(key=>typeof observed[key]!=='string') || !observed.occurrence_id || typeof observed.nofollow!=='boolean' || !Number.isSafeInteger(observed.source_ordinal) || observed.source_ordinal<0 || typeof row.evaluation_revision!=='string' || !row.decision || typeof row.decision.reason!=='string')throw new Error('Ressourcenvorkommen ohne gespeicherte Sequenz, Quelle oder Entscheidung.');
        if(grant!=null && (grant.engine!==nativeResourceEngine || grant.config_sha256!==summary.config_sha256 || grant.occurrence_id!==observed.occurrence_id || typeof grant.fetch_url!=='string' || ['store','can_expand_navigation','can_expand_resources','can_expand_css','can_expand_pdf'].some(key=>typeof grant[key]!=='boolean')))throw new Error('Nicht unterstützter eigenständiger Ressourcen-Grant.');
        previous=row.sequence;
      }
      if(result.next_after_sequence!== (rows.length?rows.at(-1).sequence:null) || result.has_more && !rows.length)throw new Error('Ungültiger oder nicht fortschreitender Vorkommens-Cursor.');
    }
  }
  async function loadResourcePage(cursor=state.resources.cursor,navigation=null) {
    if(!state.crawl || !resourceSection() || state.resources.token)return;
    const resources=state.resources, kind=resourceView();
    const token={id:state.crawl.id,generation:state.generation,serverGeneration:state.crawl.generation??null,revision:resources.revision,kind,cursor,controller:new AbortController()};
    const current=()=>resources.token===token && token.id===state.crawl?.id && token.generation===state.generation && token.serverGeneration===(state.crawl?.generation??null) && token.revision===resources.revision && token.kind===resourceView() && resourceSection();
    resources.token=token; resources.error=''; resources.selected=null; renderResourceLedger();
    try {
      const suffix=kind==='transports'?`resources/transports?${cursor===''?'':`after_url=${encodeURIComponent(cursor)}&`}`:`resources?after_sequence=${encodeURIComponent(cursor)}&`;
      const result=await request(`/api/crawls/${encodeURIComponent(token.id)}/${suffix}limit=200`,{signal:token.controller.signal});
      if(!current())return;
      validateResourceEnvelope(result,cursor,kind,token.serverGeneration);
      if(navigation==='next')resources.back.push(resources.cursor); else if(navigation==='back')resources.back.pop();
      resources.cursor=cursor; resources.result=result;
    }catch(error){if(current())resources.error=`Ressourcenbeobachtungen nicht verfügbar: ${error.message}`;}
    finally{if(resources.token===token){resources.token=null;if(token.id===state.crawl?.id && token.generation===state.generation && token.revision===resources.revision && resourceSection())renderResourceLedger();}}
  }
  const resourceOccurrenceColumns=[
    ['Sequenz',row=>row.sequence],['Generation',row=>row.generation],['Ledger-Basis',row=>resourceBasis(row.basis)],['Referenzrolle',row=>row.occurrence.reference_kind],
    ['Anfrage-Identität der Quelle',row=>row.occurrence.requested_source_url],['Quelldokument-URL aus Vorkommen',row=>row.occurrence.source_document_url],['Originalwert',row=>row.occurrence.raw_value],['Dekodierter Wert',row=>row.occurrence.decoded_value],['Aufgelöste Referenz',row=>row.occurrence.resolved_url],
    ['Entscheidungsgrund',row=>row.decision.reason],['Eigenständige Grant-Abruf-URL',row=>row.decision.grant?.fetch_url],['Antwort speichern · Grant',row=>row.decision.grant?.store],['Navigation expandieren · Grant',row=>row.decision.grant?.can_expand_navigation],['Ressourcen expandieren · Grant',row=>row.decision.grant?.can_expand_resources],['CSS expandieren · Grant',row=>row.decision.grant?.can_expand_css],['PDF expandieren · Grant',row=>row.decision.grant?.can_expand_pdf],
    ['Zusammengeführte Grant-ID',row=>row.decision.coalesced_into],['Quellordinal',row=>row.occurrence.source_ordinal],['Quelleinheit vollständig',row=>row.occurrence.source_complete],['Quelleinheit-Revision',row=>row.occurrence.source_unit_revision],['DOM-Pfad',row=>row.occurrence.dom_location],['UTF-8-Bytebeginn',row=>row.occurrence.byte_start],['UTF-8-Byteende',row=>row.occurrence.byte_end],['Deklarierter MIME-Hinweis',row=>row.occurrence.declared_mime],['Klassifikationsbasis',row=>row.occurrence.classification_basis],['Nofollow beobachtet',row=>row.occurrence.nofollow],['Beobachtungsfehler',row=>row.occurrence.observation_error],['Quellrevision',row=>row.occurrence.source_revision],['Vorkommen-ID',row=>row.occurrence.occurrence_id],['Auswertungsrevision',row=>row.evaluation_revision]
  ];
  const resourceTransportColumns=[
    ['Anfrage-Identität',row=>row.requested_url],['Gespeicherte finale URL',row=>row.projection.final_url],['Beobachtete Generation',row=>row.observed_generation],['Antwort im Seitenbericht gespeichert',row=>row.retained_response],['Erster HTTP-Status',row=>row.projection.status],['Finaler HTTP-Status',row=>row.projection.final_status],['Beobachteter Content-Type',row=>row.projection.content_type],['Antwortzeit ms',row=>row.projection.response_ms],['Erfasste Größe Bytes',row=>row.projection.size_bytes],['Transportfehler',row=>row.projection.error],['Robots blockiert',row=>row.projection.blocked_by_robots],['Response abgeschnitten',row=>row.projection.truncated],
    ['Metadatenprojektion vollständig',row=>row.projection.resource_response?.metadata_projection_complete],['Nicht erhaltene Projektionsfelder',row=>row.projection.resource_response?.unretained_fields==null?null:JSON.stringify(row.projection.resource_response.unretained_fields)],['Entity-Body in Projektion erhalten',row=>row.projection.resource_response?.entity_body_retained],['Erfasste Entity-Bytes',row=>row.projection.resource_response?.captured_entity_bytes],['Entity-Erfassung vollständig',row=>row.projection.resource_response?.captured_entity_complete],['Dekodierungsstatus',row=>row.projection.resource_response?.decode_status],['Dekodierte Quelle SHA256',row=>row.projection.resource_response?.decoded_source_sha256],['Entity-SHA256',row=>row.projection.resource_response?.captured_entity_sha256],['Erfasst am',row=>row.projection.fetched_at]
  ];
  const resourceColumns=()=>resourceView()==='transports'?resourceTransportColumns:resourceOccurrenceColumns;
  const resourceUsable=()=>Boolean(state.resources.result?.enabled && state.resources.result.ledger_available && state.resources.result.engine===nativeResourceEngine && !state.resources.error && !state.resources.token);
  function resourceVisibleRows() {
    if(!resourceUsable())return [];
    const kind=resourceView(), search=$('resource-discovery-search').value.toLocaleLowerCase(), filter=$('resource-discovery-filter').value;
    const rows=kind==='transports'?state.resources.result.transports:state.resources.result.observations;
    return rows.filter(row=>{
      if(kind==='transports'){
        if(filter==='retained' && !row.retained_response || filter==='not_retained' && row.retained_response || filter==='error' && !row.projection.error && !row.projection.blocked_by_robots || filter==='metadata_unknown' && row.projection.resource_response?.metadata_projection_complete!=null)return false;
      }else if(filter==='granted' && !row.decision.grant || filter==='no_grant' && row.decision.grant || filter==='coalesced' && row.decision.coalesced_into==null || filter==='source_incomplete' && row.occurrence.source_complete!==false || ['current','inherited'].includes(filter) && row.basis!==filter)return false;
      return !search || JSON.stringify(row).toLocaleLowerCase().includes(search);
    }).sort((left,right)=>{const value=resourceColumns()[state.resources.sort][1],a=value(left),b=value(right);if(a==null && b==null)return 0;if(a==null)return 1;if(b==null)return -1;return (typeof a==='number' && typeof b==='number'?a-b:String(a).localeCompare(String(b),'de',{numeric:true}))*state.resources.direction;});
  }
  function renderResourceLedger() {
    if(!resourceSection())return;
    const resources=state.resources,result=resources.result,summary=result?.checkpoint,busy=Boolean(resources.token),usable=resourceUsable(),kind=resourceView(),rows=resourceVisibleRows(),columns=resourceColumns();
    $('resource-discovery-error').hidden=!resources.error; $('resource-discovery-error').textContent=resources.error;
    $('resource-discovery-summary').textContent=!state.crawl?'Kein Crawl ausgewählt.':busy?'Gespeicherte Ressourcen-Seite wird geladen …':resources.error?'Snapshot nicht verfügbar; kein vollständiges leeres Ergebnis.':!result?'Noch keine gespeicherte Ledger-Seite geladen.':!result.enabled?'Native Ressourcenermittlung war für diesen Bericht nicht aktiviert.':!result.ledger_available?'Echtes SQLite-Ledger nicht verfügbar. Eine deklarierte Zusammenfassung rekonstruiert keine gespeicherten Vorkommen.':`${number(summary.decision_rows)} gespeicherte Entscheidungen · ${number(summary.completed_transports)} Abgeschlossene URL-Abrufe (vorliegendes Protokoll) · ${number(summary.retained_responses)} Gespeicherte Ergebniszeilen · ${number(summary.pending_transports)} Transporte ausstehend · ${number(summary.pending_expansions)} Expansionen ausstehend · ${number(summary.incomplete_sources)} unvollständige Quellen · Entdeckung ${summary.complete?'vollständig laut Ledger':'unvollständig'}`;
    const inherited=summary?.inherited_history;
    $('resource-discovery-scope').textContent=result?[`API-Snapshot: ${result.snapshot_scope}; Website-Anfragen: ${result.website_requests}; Entity-Bodies enthalten: ${resourceValue(result.entity_bodies_included)}.`,result.reason?`Verfügbarkeitsgrund: ${result.reason}.`:'',summary?`${result.ledger_available?'SQLite-Zusammenfassung':'Nur deklarierte Zusammenfassung'} · Revision ${resourceValue(summary.revision)} · Config-SHA256 ${resourceValue(summary.config_sha256)}.`:'',inherited?`Elternsnapshot ${inherited.parent_snapshot_sha256}; geerbte Historienzeilen ${inherited.rows_available?'verfügbar':'NICHT verfügbar; Herkunftsabdeckung unvollständig'}. Eine deklarierte Elternzusammenfassung bestätigt keine vorhandenen Zeilen.`:''].filter(Boolean).join(' '):'Metadaten werden ausschließlich aus gespeicherten API-Beobachtungen gelesen; keine Website-Anfrage wird gestartet.';
    $('resource-discovery-checkpoint').textContent=JSON.stringify({enabled:result?.enabled??null,ledger_available:result?.ledger_available??null,engine:result?.engine??null,checkpoint:summary??null,snapshot:result?.snapshot??null,crawl_config_sha256:result?.crawl_config_sha256??null,crawl_generation:result?.crawl_generation??null,reason:result?.reason??null,snapshot_scope:result?.snapshot_scope??null,entity_bodies_included:result?.entity_bodies_included??null},null,2);
    const table=$('resource-discovery-table');
    table.querySelector('thead').innerHTML=`<tr>${columns.map(([label],index)=>`<th scope="col" aria-sort="${resources.sort===index?resources.direction===1?'ascending':'descending':'none'}"><button type="button" data-resource-sort="${index}">${escape(label)} ${resources.sort===index?resources.direction===1?'↑':'↓':'↕'}</button></th>`).join('')}</tr>`;
    const empty=busy?'Seite wird geladen; bisherige Zeilen werden nicht als neuer Snapshot angezeigt.':resources.error?'Lesefehler; dies ist kein leeres vollständiges Ergebnis.':!result?.ledger_available?'Keine verifizierten Ledger-Zeilen verfügbar.':!rows.length?'Keine passenden Beobachtungen auf der geladenen Seite. Weitere Seiten können andere Vorkommen enthalten.':'';
    table.querySelector('tbody').innerHTML=rows.length?rows.map((row,index)=>`<tr data-resource-row="${index}" tabindex="0" aria-label="Details zur gespeicherten Ressourcenbeobachtung ${escape(kind==='transports'?row.requested_url:row.occurrence.occurrence_id)}">${columns.map(([,value])=>`<td>${escape(resourceValue(value(row)))}</td>`).join('')}</tr>`).join(''):`<tr><td class="empty-cell" colspan="${columns.length}">${escape(empty)}</td></tr>`;
    table.querySelectorAll('[data-resource-sort]').forEach(button=>button.addEventListener('click',()=>{const sort=Number(button.dataset.resourceSort);resources.direction=resources.sort===sort?-resources.direction:1;resources.sort=sort;renderResourceLedger();}));
    table.querySelectorAll('[data-resource-row]').forEach(element=>{const open=()=>{resources.selected=rows[Number(element.dataset.resourceRow)];renderResourceInspector();$('resource-discovery-inspector').scrollIntoView({block:'nearest'});};element.addEventListener('click',open);element.addEventListener('keydown',event=>{if(['Enter',' '].includes(event.key)){event.preventDefault();open();}});});
    $('resource-discovery-prev').disabled=busy || !resources.back.length; $('resource-discovery-next').disabled=!usable || !result.has_more; $('resource-discovery-refresh').disabled=busy || !state.crawl;
    for(const suffix of ['csv','json'])$('resource-discovery-export-'+suffix).disabled=!usable;
    $('resource-discovery-export-page').disabled=!usable || !rows.length;
    const loaded=kind==='transports'?result?.transports?.length:result?.observations?.length;
    $('resource-discovery-page-label').textContent=`${number(rows.length)} gefilterte/sortierte von ${loaded==null?'unbekannt':number(loaded)} geladenen Zeilen · nach ${kind==='transports'?'Anfrage-URL':'Sequenz'} ${resources.cursor===''?'[Start]':resources.cursor} · maximal 200`;
    renderResourceInspector();
  }
  function renderResourceInspector() {
    const row=state.resources.selected; $('resource-discovery-inspector').hidden=!row;if(!row)return;
    const transport=resourceView()==='transports';
    $('resource-discovery-inspector-heading').textContent=transport?'Abgeschlossene native Transportbeobachtung':'Gespeichertes Quellvorkommen und eigenständiger Grant';
    $('resource-discovery-inspector-basis').textContent=transport?`Tatsächliche Anfrage ${row.requested_url}; beobachtete Generation ${row.observed_generation}; Antwort ${row.retained_response?'im Seitenbericht gespeichert':'nicht im Seitenbericht gespeichert'}. Diese Projektion enthält keine Entity-HTML/CSS-Bodies; fehlende Quellerfassung oder Metadatenvollständigkeit bleiben unbekannt. Vereinigte Task-Erlaubnisse erweitern keinen früheren einzelnen Grant.`:`${resourceBasis(row.basis)}; Generation ${row.generation}; Sequenz ${row.sequence}. Quelle und Originalwert sind beobachtet. Der eigenständige Grant ${row.decision.grant?'erteilt Erlaubnisse für dieses Vorkommen':'fehlt für dieses Vorkommen'}; Zusammenführung beweist keinen gestarteten oder erfolgreichen Transport und verändert seine bisherigen Erlaubnisse nicht.`;
    $('resource-discovery-inspector-data').textContent=JSON.stringify(row,null,2);
  }
  function exportResourcePage() {
    const rows=resourceVisibleRows();if(!rows.length)return;
    const result=state.resources.result,summary=result.checkpoint;
    const meta=[['API Snapshot-Scope',()=>result.snapshot_scope],['Vollständige Crawl-Config SHA256',()=>result.snapshot.crawl_config_sha256],['SQL-Snapshot Generation',()=>result.snapshot.generation],['Ledger-Engine',()=>result.engine],['Ledger-Config SHA256',()=>summary.config_sha256],['Ledger-Snapshot Revision',()=>summary.revision],['Ledger-Rolling SHA256',()=>summary.rolling_sha256],['Ledger-Entdeckung vollständig',()=>summary.complete],['Geerbte Historienzeilen verfügbar',()=>summary.inherited_history?.rows_available],['Geladener Cursor',()=>state.resources.cursor],['Exportumfang',()=> 'filtered_loaded_page_only']];
    const columns=[...resourceColumns(),...meta],cell=input=>{let value=String(input??'');if(typeof input!=='number' && typeof input!=='boolean' && /^[\s\uFEFF]*[=+@-]/u.test(value))value=`'${value}`;return `"${value.replace(/"/g,'""')}"`;};
    const data=[columns.map(([label])=>cell(label)).join(','),...rows.map(row=>columns.map(([,value])=>cell(value(row))).join(','))].join('\r\n');
    downloadBlob(new Blob(['\uFEFF',data],{type:'text/csv;charset=utf-8'}),`open-seo-spider-${state.crawl.id}-resources-${resourceView()}-loaded-page.csv`);
    message(`${number(rows.length)} gefilterte Zeilen der geladenen Ressourcen-Seite exportiert; weitere gespeicherte Zeilen sind darin nicht enthalten.`);
  }
  function downloadResourceSnapshot(extension) {
    if(!resourceUsable() || !state.crawl)return;
    const link=document.createElement('a');link.href=`/api/crawls/${encodeURIComponent(state.crawl.id)}/resources.${extension}`;link.download=`open-seo-spider-${state.crawl.id}-resources.${extension}`;document.body.append(link);link.click();link.remove();
    message('Export aller gespeicherten Vorkommen angefordert. Er exportiert einen SQLite-Snapshot; vollständige Zeilen sind keine vollständige Entdeckung.');
  }
  function bindResourceEvents() {
    syncResourceFilter();
    $('resource-discovery-kind').addEventListener('change',()=>{resetResourceLedger();renderResourceLedger();loadResourcePage();});
    $('resource-discovery-search').addEventListener('input',renderResourceLedger); $('resource-discovery-filter').addEventListener('change',renderResourceLedger);
    $('resource-discovery-refresh').addEventListener('click',()=>loadResourcePage());
    $('resource-discovery-next').addEventListener('click',()=>{if(resourceUsable() && state.resources.result.has_more)loadResourcePage(resourceView()==='transports'?state.resources.result.next_after_url:state.resources.result.next_after_sequence,'next');});
    $('resource-discovery-prev').addEventListener('click',()=>{if(state.resources.back.length)loadResourcePage(state.resources.back.at(-1),'back');});
    $('resource-discovery-inspector-close').addEventListener('click',()=>{state.resources.selected=null;renderResourceInspector();});
    $('resource-discovery-export-page').addEventListener('click',exportResourcePage);
    $('resource-discovery-export-csv').addEventListener('click',()=>downloadResourceSnapshot('csv')); $('resource-discovery-export-json').addEventListener('click',()=>downloadResourceSnapshot('json'));
  }

  function invalidatePolicyPreview() {
    policyPreview.revision += 1; policyPreview.token?.controller.abort(); policyPreview.token = null; policyPreview.result = null;
    if (!$('policy-preview-results')) return;
    $('policy-preview-results').hidden = true; $('policy-preview-error').hidden = true; $('policy-preview-button').disabled = false;
    $('policy-preview-status').textContent = 'Noch keine gültige Offline-Probe für diese Eingabe.';
  }

  const savedContentEngine = 'open-seo-spider/saved-content-cohort-v1';
  const contentProfiles = { engine:'open-seo-spider/content-analysis-v1',token:'unicode-alphanumeric-mark-runs-no-normalization-v1',similarity:'exact-token-shingle-set-jaccard-v1',readability:'declared-en-de-vowel-groups-terminal-e-v1',capture:'open-seo-spider/native-content-binding-v1',captureEngine:'open-seo-spider/html-content-capture-v1',selection:'css-ancestor-membership-document-order-v1',whitespace:'unicode-whitespace-and-nonempty-node-boundaries-ascii-space-v1',parser:'scraper-0.27.0-html5ever-document-recovery-v1',htmlHash:'supplied-html-string-utf8-v1' };
  const contentSection = () => state.section === 'content';
  const contentSource = () => $('content-source').value;
  const contentBasis = basis => ({captured_in_crawl:'Erfasst in diesem Crawl',refreshed_selected:'Neu geprüft im ausgewählten Lauf',inherited_parent:'Beibehalten aus Elternbericht',pending_selected:'Ausstehend · kein aktueller Text',unknown_operation:'Befundalter unbekannt'}[basis] || 'Befundbasis unbekannt');
  const contentParent = row => !row ? 'Elternquelle unbekannt' : row.parent_source_kind == null ? 'Elternquelle unbekannt' : `${row.parent_source_kind}${row.parent_source_kind==='imported_snapshot' ? ' · deklarierter Import; keine authentifizierte Netzwerkquelle' : ''}`;
  const contentStatus = status => ({complete:'Vollständig',empty:'Beobachtet leer',incomplete:'Unvollständig',unavailable:'Quelle nicht verfügbar',error:'Erfassungsfehler',missing:'Nicht erfasst / Legacy'}[status] || 'Nicht unterstützter Zustand');
  const contentOrigin = origin => origin === 'native_saved_report' ? 'Nativ gespeicherter Bericht' : origin === 'declared_imported_report' ? 'Deklarierter importierter Bericht; keine authentifizierte Netzwerkquelle' : 'Herkunft nicht unterstützt';
  const contentReason = row => [row.reason,...array(row.capture_reasons).map(reason => [reason.code,reason.detail].filter(value => value != null).join(': ')),row.capture_failure && `${row.capture_failure.code}: ${row.capture_failure.detail}`].filter(Boolean).join(' · ');
  function invalidateContentRequests(reason = 'Eingabe geändert; vorherige Analyse ungültig.', inventory = false) {
    const content = state.content; content.revision += 1;
    for (const key of ['analysisToken','sourceToken','inventoryToken']) { content[key]?.controller.abort(); content[key] = null; }
    content.result = null; content.analysisError = ''; content.source = null; content.sourceError = ''; content.resultPage = 0;
    if (inventory) { content.inventory = null; content.error = ''; content.cursor = 0; content.back = []; }
    if ($('content-analysis-status')) { $('content-analysis-status').textContent = reason; $('content-analysis-results').hidden = true; $('content-source-inspector').hidden = true; $('content-analysis-json').disabled = true; $('content-source-json').disabled = true; $('content-analysis-cancel').hidden = true; }
  }
  function resetContent() {
    state.content.inventoryToken?.controller.abort(); state.content.inventoryToken = null;
    state.content.inventory = null; state.content.selected.clear(); state.content.cursor = 0; state.content.back = []; state.content.error = '';
    state.content.sort = 0; state.content.direction = 1;
    if ($('content-search')) { $('content-search').value = ''; $('content-filter').value = 'all'; $('content-result-search').value = ''; }
  }
  function contentToken(kind) {
    const token = { kind,id:state.crawl.id,generation:state.generation,revision:state.content.revision,source:contentSource(),controller:new AbortController() };
    state.content[kind] = token; return token;
  }
  function contentCurrent(token) { return state.content[token.kind] === token && token.id === state.crawl?.id && token.generation === state.generation && token.revision === state.content.revision && token.source === contentSource(); }
  function validateContentEnvelope(result, source) {
    if (!result || result.engine !== savedContentEngine || result.source_kind !== source || result.website_requests !== 0 || result.site_wide_coverage !== false || result.snapshot?.sha_profile !== 'typed-crawl-report-serde-json-v1' || typeof result.snapshot.sha256 !== 'string' || !/^[a-fA-F0-9]{64}$/.test(result.snapshot.sha256) || !['native_saved_report','declared_imported_report'].includes(result.snapshot.origin) || !(result.snapshot.generation == null || Number.isSafeInteger(result.snapshot.generation) && result.snapshot.generation >= 0)) throw new Error('Nicht unterstützte gespeicherte Inhaltsbindung, Quelle oder Snapshot-Profil.');
  }
  function validateContentRow(row, source) {
    if (!row || typeof row.requested_url !== 'string' || !Number.isSafeInteger(row.ordinal) || row.ordinal < 0 || row.source_kind !== source || row.text_included !== false) throw new Error('Ungültige Metadaten einer gespeicherten Textquelle.');
  }
  async function loadContentInventory(cursor = state.content.cursor, navigation = null) {
    const content = state.content;
    if (!state.crawl || active() || content.inventoryToken || !contentSection()) return;
    const token = contentToken('inventoryToken'); content.error = ''; renderContent();
    try {
      const result = await request(`/api/crawls/${encodeURIComponent(token.id)}/content?after_ordinal=${cursor}&limit=200&source_kind=${token.source}`,{signal:token.controller.signal});
      if (!contentCurrent(token)) return;
      validateContentEnvelope(result,token.source);
      if (!Array.isArray(result.rows) || result.rows.length > 200 || result.limit !== 200 || result.text_included !== false || result.after_ordinal !== cursor || typeof result.has_more !== 'boolean' || !Number.isSafeInteger(result.total_saved_pages) || result.total_saved_pages < 0 || result.has_more && (!Number.isSafeInteger(result.next_after_ordinal) || result.next_after_ordinal <= cursor)) throw new Error('Ungültige oder unbeschränkte gespeicherte Quellseite.');
      result.rows.forEach(row => validateContentRow(row,token.source));
      if (content.inventory && content.inventory.snapshot.sha256 !== result.snapshot.sha256) {
        invalidateContentRequests('Der gespeicherte Snapshot wurde geändert. Kohorte erneut auswählen.'); content.selected.clear();
        // This accepted inventory is the new basis; older document/analysis tokens remain invalid.
        token.revision=content.revision; content.inventoryToken=token;
      }
      if (navigation === 'next') content.back.push(content.cursor); else if (navigation === 'back') content.back.pop();
      content.cursor = cursor; content.inventory = result;
    } catch (error) { if (contentCurrent(token)) content.error = `Gespeicherte Textquellen nicht verfügbar: ${error.message}`; }
    finally { if (content.inventoryToken === token) { content.inventoryToken = null; if (token.id === state.crawl?.id && token.generation === state.generation) renderContent(); } }
  }
  const contentColumns = [
    ['Ordinal',row => row.ordinal],['Exakte Anfrage-URL',row => row.requested_url],['Befundbasis',row => contentBasis(row.basis)],['Quelle',row => row.source_kind],['HTTP-Dokumentstatus',row => row.page_http_status],['Textzustand',row => contentStatus(row.status)],['Textbytes',row => row.text_bytes],['Quellsprache',row => row.html_language],['Sprachgrund',row => row.html_language_reason],['Vollständiges Textsample',row => row.sample_complete],['Quellgeneration',row => row.observed_generation],['Quelle/Dokument-URL',row => row.source_document_url],['Quellrevision',row => row.source_revision],['Erfassungsprofil',row => row.capture_engine],['Auswahlprofil',row => row.selection_profile],['Erfassungskonfiguration SHA256',row => row.capture_config_sha256],['Text SHA256',row => row.text_sha256],['Grund / Fehler',contentReason],['Elternquelltyp',contentParent],['Eltern-Snapshot SHA256',row=>row.parent_snapshot_sha256]
  ];
  function contentVisibleRows() {
    const content = state.content, search = $('content-search').value.toLocaleLowerCase(), filter = $('content-filter').value;
    return array(content.inventory?.rows).filter(row => (filter === 'all' || row.status === filter) && (!search || JSON.stringify(row).toLocaleLowerCase().includes(search))).sort((left,right) => compareContentValues(contentColumns[content.sort][1](left),contentColumns[content.sort][1](right),content.direction));
  }
  function compareContentValues(a,b,direction) { if (a == null && b == null) return 0; if (a == null) return 1; if (b == null) return -1; return (typeof a === 'number' && typeof b === 'number' ? a-b : String(a).localeCompare(String(b),'de',{numeric:true})) * direction; }
  function setContentSelected(url, selected) {
    const content = state.content;
    if (active() || actionBusy() || content.inventoryToken || !array(content.inventory?.rows).some(row => row.requested_url === url)) return;
    if (selected && !content.selected.has(url) && content.selected.size >= 128) { content.analysisError = 'Maximal 128 exakte Anfrage-URLs. Keine erste Teilmenge wird automatisch gewählt.'; renderContent(); return; }
    if (selected) content.selected.add(url); else content.selected.delete(url);
    invalidateContentRequests('Kohorte geändert. Vorherige Berechnung ist nicht mehr gültig.'); renderContent();
    [...document.querySelectorAll('[data-content-select]')].find(input => input.dataset.contentSelect === url)?.focus({preventScroll:true});
  }
  function renderContent() {
    if (!contentSection()) return;
    const content = state.content, inventory = content.inventory, snapshot = inventory?.snapshot;
    $('content-summary').textContent = !state.crawl ? 'Einen gespeicherten Crawl auswählen.' : active() ? 'Der Crawl ist aktiv. Textquellen werden erst aus einem ruhenden gespeicherten Stand gelesen.' : content.inventoryToken ? 'Gespeicherte Quellseite wird geladen …' : !inventory ? 'Noch keine gespeicherten Textquellen geladen.' : `${contentOrigin(snapshot.origin)} · Snapshot ${snapshot.sha256} · Generation ${snapshot.generation ?? 'unbekannt'} · ${number(inventory.total_saved_pages)} gespeicherte Dokumente; diese API-Seite enthält keinen Text.`;
    $('content-scope').textContent = 'Quelle: ' + contentSource() + ' · Snapshot-Profil: ' + (snapshot?.sha_profile || 'noch nicht erfasst') + ' · Keine Website-Vollständigkeit abgeleitet. Befundalter, Textvollständigkeit und Herkunft bleiben getrennt.';
    $('content-error').hidden = !content.error; $('content-error').textContent = content.error;
    const rows = contentVisibleRows(), usable = Boolean(inventory && !content.error && !active()), busy = Boolean(content.inventoryToken);
    const table = $('content-table');
    table.querySelector('thead').innerHTML = `<tr><th scope="col">Kohorte</th>${contentColumns.map(([label],index) => `<th scope="col" aria-sort="${content.sort===index ? content.direction===1?'ascending':'descending':'none'}"><button type="button" data-content-sort="${index}">${escape(label)} ${content.sort===index ? content.direction===1?'↑':'↓':'↕'}</button></th>`).join('')}</tr>`;
    table.querySelector('tbody').innerHTML = rows.length && !content.error ? rows.map(row => `<tr data-content-row="${row.ordinal}"><td><input class="content-row-checkbox" type="checkbox" data-content-select="${escape(row.requested_url)}" aria-label="${escape(row.requested_url)} zur Textkohorte auswählen" ${content.selected.has(row.requested_url)?'checked':''} ${!usable || busy || actionBusy()?'disabled':''}></td>${contentColumns.map(([,value],index) => `<td>${index===1 ? `<button class="text-button source-open" type="button" data-content-open="${escape(row.requested_url)}" ${!usable?'disabled':''}>${escape(row.requested_url)}</button>` : escape(value(row) == null ? 'Unbekannt' : typeof value(row)==='boolean' ? value(row)?'Ja':'Nein' : value(row))}</td>`).join('')}</tr>`).join('') : `<tr><td class="empty-cell" colspan="${contentColumns.length+1}">${escape(content.error ? 'Quelle konnte nicht geladen werden; kein leeres vollständiges Ergebnis.' : active() ? 'Ein ruhender gespeicherter Stand ist erforderlich.' : inventory ? 'Keine passenden Befunde auf der geladenen Quellseite.' : 'Noch keine gespeicherten Textquellen geladen.')}</td></tr>`;
    table.querySelectorAll('[data-content-sort]').forEach(button => button.addEventListener('click',()=>{ const sort=Number(button.dataset.contentSort); content.direction=content.sort===sort?-content.direction:1; content.sort=sort; renderContent(); }));
    table.querySelectorAll('[data-content-select]').forEach(input => input.addEventListener('change',()=>setContentSelected(input.dataset.contentSelect,input.checked)));
    table.querySelectorAll('[data-content-open]').forEach(button => button.addEventListener('click',()=>loadContentDocument(button.dataset.contentOpen)));
    $('content-prev').disabled = busy || !usable || !content.back.length; $('content-next').disabled = busy || !usable || !inventory.has_more;
    $('content-refresh').disabled = !state.crawl || active() || busy; $('content-page-label').textContent = `${number(rows.length)} passende von ${number(inventory?.rows?.length || 0)} geladenen Befunden · Ordinal ab ${content.cursor} · maximal 200 pro API-Seite`;
    $('content-selection-summary').textContent = `${number(content.selected.size)} exakte Anfrage-URLs ausgewählt · Quelle ${contentSource()} · höchstens 128, keine automatische Teilmenge`;
    $('content-selection-list').replaceChildren(...[...content.selected].map(url=>{const element=document.createElement('li');element.textContent=url;return element;}));
    $('content-clear').disabled = !content.selected.size || Boolean(content.analysisToken);
    $('content-analyze').disabled = !usable || busy || Boolean(content.analysisToken) || content.selected.size<1 || content.selected.size>128 || actionBusy();
    $('content-analysis-cancel').hidden = !content.analysisToken;
    $('content-analysis-error').hidden = !content.analysisError; $('content-analysis-error').textContent = content.analysisError;
    $('export-csv').disabled = !usable || busy || !rows.length;
    renderContentSource(); renderContentAnalysis();
  }
  async function loadContentDocument(url) {
    const content = state.content;
    if (active() || !content.inventory || !array(content.inventory.rows).some(row=>row.requested_url===url)) return;
    content.sourceToken?.controller.abort(); const token=contentToken('sourceToken'); content.source=null; content.sourceError=''; $('content-source-inspector').hidden=false; $('content-source-text').textContent='Gespeicherte Quelle wird geladen …'; $('content-source-json').disabled=true; $('content-source-inspector').scrollIntoView({block:'nearest'});
    try {
      const result=await request(`/api/crawls/${encodeURIComponent(token.id)}/content/document?url=${encodeURIComponent(url)}&source_kind=${token.source}`,{signal:token.controller.signal});
      if (!contentCurrent(token)) return;
      validateContentEnvelope(result,token.source); validateContentRow(result.row,token.source);
      if (result.row.requested_url!==url || result.snapshot.sha256!==content.inventory.snapshot.sha256 || result.snapshot.generation!==content.inventory.snapshot.generation) throw new Error('Textquelle gehört nicht zum ausgewählten gespeicherten Snapshot. Quellseite aktualisieren.');
      content.source=result;
    } catch(error) { if(contentCurrent(token)) content.sourceError='Textquelle nicht verfügbar: '+error.message; }
    finally { if(content.sourceToken===token) { content.sourceToken=null; if(token.id===state.crawl?.id && token.generation===state.generation) renderContentSource(); } }
  }
  function renderContentSource() {
    const content=state.content, source=content.source;
    $('content-source-inspector').hidden=!source && !content.sourceToken && !content.sourceError;
    if(content.sourceToken) return;
    if(!source) { if(content.sourceError) $('content-source-text').textContent=content.sourceError; $('content-source-json').disabled=true; return; }
    const row=source.row,result=source.capture?.result;
    $('content-source-heading').textContent=row.requested_url;
    $('content-source-basis').textContent=`${contentBasis(row.basis)} · ${contentOrigin(source.snapshot.origin)}${['inherited_parent','pending_selected'].includes(row.basis) ? ' · Elternquelle '+contentParent(row)+' · Eltern-Snapshot '+(row.parent_snapshot_sha256 ?? 'unbekannt') : ''} · Quelle ${source.source_kind} · ${contentStatus(row.status)} · Quellsprache ${row.html_language ?? 'unbekannt'}${row.html_language_reason ? ' ('+row.html_language_reason+')' : ''}. Textvollständigkeit belegt keinen neuen Abruf.`;
    const supported = !source.capture || source.capture.engine===contentProfiles.capture && (!result || result.engine===contentProfiles.captureEngine && result.selection_profile===contentProfiles.selection && result.whitespace_profile===contentProfiles.whitespace && result.parser_profile===contentProfiles.parser && result.html_hash_representation===contentProfiles.htmlHash);
    $('content-source-text').textContent=!supported ? 'Nicht unterstütztes Erfassungsprofil. Exakte Daten bleiben im JSON sichtbar; kein Text wird als geprüfte Quelle umgedeutet.' : result?.text == null ? `Kein gespeicherter Text. ${contentReason(row) || 'Quelle nicht verfügbar.'}` : result.text==='' ? '[Beobachtet leere Textauswahl: exakter Text ist eine leere Zeichenfolge.]' : result.text;
    $('content-source-evidence').textContent=JSON.stringify(source,null,2); $('content-source-json').disabled=false;
  }
  async function analyzeSavedContent() {
    const content=state.content;
    if(active() || actionBusy() || content.analysisToken || !content.inventory || content.error) return;
    let config; try { config=parseContentAnalysisConfig(); if(content.selected.size<1 || content.selected.size>128 || content.selected.size>(config.limits?.max_documents ?? 128)) throw new Error('1 bis höchstens max_documents/128 konkrete Anfrage-URLs auswählen; keine automatische Teilmenge.'); }
    catch(error) { content.analysisError=error.message; renderContent(); return; }
    invalidateContentRequests('Gespeicherte Texte werden offline berechnet …');
    const token=contentToken('analysisToken'),snapshot=content.inventory.snapshot;
    const body={urls:[...content.selected],source_kind:token.source,config,indexable_only:$('content-indexable-only').checked,complete_only:$('content-complete-only').checked,...(snapshot.generation==null ? {} : {expected_generation:snapshot.generation})};
    renderContent();
    try {
      const result=await request(`/api/crawls/${encodeURIComponent(token.id)}/content/analyze`,{method:'POST',body:JSON.stringify(body),signal:token.controller.signal});
      if(!contentCurrent(token)) return;
      validateContentEnvelope(result,token.source);
      if(result.snapshot.sha256!==snapshot.sha256 || result.snapshot.generation!==snapshot.generation || !Array.isArray(result.requested_urls) || JSON.stringify(result.requested_urls)!==JSON.stringify(body.urls) || result.selected_count!==body.urls.length || !Array.isArray(result.documents) || !Array.isArray(result.excluded) || result.included_count!==result.documents.length || result.included_count+result.excluded.length!==result.selected_count || !['ready','empty_filtered_cohort'].includes(result.status) || result.status==='ready' && !result.analysis || result.status==='empty_filtered_cohort' && (result.analysis!==null || result.included_count!==0)) throw new Error('Analyse ist nicht eindeutig an den ausgewählten Snapshot und die vollständige explizite Kohorte gebunden.');
      if(result.indexable_only!==body.indexable_only || result.complete_only!==body.complete_only) throw new Error('Analysefilter stimmen nicht mit der ausgewählten Kohorte überein.');
      const observed=new Set(),ids=new Set();
      for(const entry of [...result.documents,...result.excluded]) { validateContentRow(entry.row,token.source); if(!body.urls.includes(entry.row.requested_url) || observed.has(entry.row.requested_url)) throw new Error('Analysedokumente/Ausschlüsse verändern die exakte Kohortenidentität.'); observed.add(entry.row.requested_url); }
      for(const entry of result.documents) { if(typeof entry.analysis_document_id!=='string' || ids.has(entry.analysis_document_id)) throw new Error('Analysedokumente haben keine eindeutige Quellbindung.'); ids.add(entry.analysis_document_id); }
      if(result.analysis && (!Array.isArray(result.analysis.documents) || result.analysis.documents.length!==result.documents.length || new Set(result.analysis.documents.map(document=>document.id)).size!==ids.size || result.analysis.documents.some(document=>!ids.has(document.id)) || !Array.isArray(result.analysis.pairs) || result.analysis.pairs.some(pair=>!ids.has(pair.left_id) || !ids.has(pair.right_id)))) throw new Error('Analysewerte sind nicht an alle enthaltenen Dokumente gebunden.');
      content.result=result; content.resultPage=0; $('content-result-search').value=''; $('content-analysis-status').textContent=result.status==='empty_filtered_cohort' ? 'Alle ausgewählten Quellen wurden ausdrücklich ausgeschlossen; keine Analyse erfunden.' : 'Ausgewählte gespeicherte Texte berechnet · 0 Website-Anfragen.';
    } catch(error) { if(contentCurrent(token)) { content.analysisError='Inhaltsanalyse fehlgeschlagen: '+error.message; $('content-analysis-status').textContent='Keine gültige Analyse für diese Kohorte.'; } }
    finally { if(content.analysisToken===token) { content.analysisToken=null; if(token.id===state.crawl?.id && token.generation===state.generation) renderContent(); } }
  }
  function supportedContentAnalysis(result) { return result?.engine===contentProfiles.engine && result.token_profile===contentProfiles.token && result.similarity_profile===contentProfiles.similarity; }
  function contentResultData() {
    const result=state.content.result, analysis=result?.analysis, kind=$('content-result-kind').value;
    if(!analysis || !supportedContentAnalysis(analysis)) return {columns:[],rows:[],evidence:analysis?'Nicht unterstütztes Analyse-/Token-/Ähnlichkeitsprofil; keine abgeleiteten Werte angezeigt.':'Keine enthaltenen Texte nach ausdrücklichen Filtern.'};
    const provenance=new Map(array(result.documents).map(document=>[document.analysis_document_id,document.row]));
    const label=id=>provenance.get(id)?.requested_url || id;
    if(kind==='similarity') return {columns:[['Linke Anfrage-URL',row=>label(row.left_id)],['Rechte Anfrage-URL',row=>label(row.right_id)],['Linke Befundbasis',row=>contentBasis(provenance.get(row.left_id)?.basis)],['Rechte Befundbasis',row=>contentBasis(provenance.get(row.right_id)?.basis)],['Shingle-Länge',row=>row.shingle_size],['Schwelle',row=>row.threshold],['Schnittmenge',row=>row.intersection],['Vereinigung',row=>row.union],['Beobachteter Jaccard',row=>row.observed_jaccard],['Near-Duplicate',row=>row.near_duplicate],['Exakter Text identisch',row=>row.identical_supplied_text],['Vollständige Samples',row=>row.sample_complete],['Grund',row=>row.reason],['Gemeinsame Beispiele',row=>JSON.stringify(row.shared_shingle_samples)],['Beispiele gekürzt',row=>row.shared_samples_truncated]],rows:array(analysis.pairs),evidence:`Alle ausgegebenen Paare dieser Kohorte. Jaccard 1 ist keine Byte-Identität. Unvollständige Samples können einen beobachteten Wert mit unbekannter Klassifikation haben. Paarprofil ${analysis.similarity_profile}.`};
    if(kind==='readability') return {columns:[['Anfrage-URL',row=>label(row.id)],['Befundbasis',row=>contentBasis(provenance.get(row.id)?.basis)],['Textzustand',row=>row.status],['Deklarierte Quellsprache',row=>row.readability?.declared_language],['Profil',row=>row.readability?.profile],['Formel',row=>row.readability?.formula],['Wörter',row=>row.readability?.words],['Sätze',row=>row.readability?.sentences],['Silbenheuristik',row=>row.readability?.syllables],['Score',row=>row.readability?.profile===contentProfiles.readability ? row.readability.score : null],['Grund',row=>row.readability?.profile===contentProfiles.readability ? row.readability.reason : 'unsupported_readability_profile'],['Nicht unterstützte Tokens',row=>row.readability?.unsupported_tokens],['Tokenbeispiele',row=>JSON.stringify(row.readability?.unsupported_token_samples)],['Sample vollständig',row=>row.readability?.sample_complete]],rows:array(analysis.documents),evidence:'Nur deklarierte EN/DE-Quellsprache; unabhängige Vokalgruppen- und Satzzeichenheuristik. Scores bleiben unbeschnitten, auch unter 0/über 100. Keine Sprachdetektion, Schulstufe, Verständnis- oder Rankingprognose.'};
    const document=array(analysis.documents).find(row=>row.id===$('content-result-document').value),n=Number($('content-result-n').value),group=array(document?.ngrams).find(row=>row.n===n);
    return {columns:[['Tokenfolge',row=>row.tokens.join(' ')],['Vorkommen',row=>row.occurrences],['Erste Positionen (1-basiert)',row=>JSON.stringify(row.first_positions)],['Positionen nur Stichprobe',row=>row.positions_sampled]],rows:array(group?.rows),evidence:document ? `${label(document.id)} · ${document.status} · ${group?.candidate_windows ?? 'unbekannt'} Kandidatenfenster, ${group?.excluded_stopword_windows ?? 'unbekannt'} mit Stopwords ausgeschlossen, ${group?.included_windows ?? 'unbekannt'} eingeschlossen. Exakte Anzahl und begrenzte Positionsbeispiele bleiben getrennt. ${analysis.token_profile}; ${analysis.config?.case_behavior || 'unbekannt'}; Unicode ${array(analysis.lowercase_unicode_version).join('.')}.` : 'Ein enthaltenes Dokument wählen. Fehlender Text ist kein beobachtet leeres Sample.'};
  }
  function renderContentAnalysis() {
    const content=state.content,result=content.result,analysis=result?.analysis;
    $('content-analysis-results').hidden=!result; $('content-analysis-json').disabled=!result;
    if(!result) return;
    $('content-analysis-summary').textContent=`${result.selected_count} ausgewählt · ${result.included_count} enthalten · ${result.excluded.length} ausdrücklich ausgeschlossen · ${contentOrigin(result.snapshot.origin)} · Snapshot ${result.snapshot.sha256}, Generation ${result.snapshot.generation ?? 'unbekannt'} · Quelle ${result.source_kind} · 0 Website-Anfragen. Keine Website-Vollständigkeit behauptet.`;
    $('content-analysis-profile').textContent=!analysis ? 'Leere Kohorte nach ausdrücklichen Filtern; Ausschlüsse sind vollständig in der Evidenz aufgeführt.' : !supportedContentAnalysis(analysis) ? 'Nicht unterstütztes Analyseprofil. Keine N-Grams, Ähnlichkeits- oder Lesbarkeitswerte werden daraus abgeleitet.' : `Engine ${analysis.engine} · Tokenprofil ${analysis.token_profile} · Paarprofil ${analysis.similarity_profile} · Input SHA256 ${analysis.input_sha256} · Config SHA256 ${analysis.config_sha256} · ${analysis.corpus?.pairs_emitted ?? 'unbekannt'} ausgegebene / ${analysis.corpus?.possible_pairs ?? 'unbekannt'} mögliche Paare; ${analysis.corpus?.pairs_scored ?? 'unbekannt'} bewertet.`;
    const selector=$('content-result-document'), previous=selector.value;
    selector.replaceChildren(...array(analysis?.documents).map(analysisDocument=>{const option=document.createElement('option');option.value=analysisDocument.id;option.textContent=array(result.documents).find(row=>row.analysis_document_id===analysisDocument.id)?.row.requested_url || analysisDocument.id;return option;}));
    if([...selector.options].some(option=>option.value===previous)) selector.value=previous;
    const ngram=$('content-result-kind').value==='ngrams'; $('content-result-document-label').hidden=!ngram; $('content-result-n-label').hidden=!ngram;
    $('content-analysis-evidence').textContent=JSON.stringify(result,null,2);
    const data=contentResultData(),search=$('content-result-search').value.toLocaleLowerCase(),sort=Math.min(content.resultSort,Math.max(0,data.columns.length-1));
    const rows=data.rows.filter(row=>!search || JSON.stringify(row).toLocaleLowerCase().includes(search)).sort((left,right)=>compareContentValues(data.columns[sort]?.[1](left),data.columns[sort]?.[1](right),content.resultDirection));
    const size=200,pages=Math.max(1,Math.ceil(rows.length/size)); content.resultPage=Math.min(content.resultPage,pages-1);
    const visible=rows.slice(content.resultPage*size,(content.resultPage+1)*size),table=$('content-result-table');
    $('content-result-evidence').textContent=data.evidence;
    table.querySelector('thead').innerHTML=`<tr>${data.columns.map(([label],index)=>`<th scope="col" aria-sort="${sort===index?content.resultDirection===1?'ascending':'descending':'none'}"><button type="button" data-content-result-sort="${index}">${escape(label)}</button></th>`).join('')}</tr>`;
    table.querySelector('tbody').innerHTML=visible.length ? visible.map(row=>`<tr data-content-result-row>${data.columns.map(([,value])=>{const cell=value(row);return `<td>${escape(cell==null?'Unbekannt':typeof cell==='boolean'?cell?'Ja':'Nein':cell)}</td>`;}).join('')}</tr>`).join('') : `<tr><td class="empty-cell" colspan="${Math.max(1,data.columns.length)}">${escape(data.rows.length ? 'Keine passenden Zeilen in dieser Ergebnisansicht.' : data.evidence)}</td></tr>`;
    table.querySelectorAll('[data-content-result-sort]').forEach(button=>button.addEventListener('click',()=>{const index=Number(button.dataset.contentResultSort);content.resultDirection=content.resultSort===index?-content.resultDirection:1;content.resultSort=index;content.resultPage=0;renderContentAnalysis();}));
    $('content-result-prev').disabled=content.resultPage===0; $('content-result-next').disabled=content.resultPage>=pages-1; $('content-result-page').textContent=`${number(visible.length)} von ${number(rows.length)} passenden Ergebniszeilen · Seite ${content.resultPage+1}/${pages} · maximal 200 gerenderte Zeilen`;
  }
  function exportContentRows() {
    const inventory=state.content.inventory,rows=contentVisibleRows(); if(active() || !inventory || state.content.inventoryToken || state.content.error || !rows.length) return;
    const cell=value=>{let text=String(value??'');if(typeof value!=='number' && typeof value!=='boolean' && /^[\s\uFEFF]*[=+@-]/u.test(text))text="'"+text;return '"'+text.replace(/"/g,'""')+'"';};
    const columns=[['Snapshot SHA256',()=>inventory.snapshot.sha256],['Snapshot Generation',()=>inventory.snapshot.generation],['Snapshot Origin',()=>inventory.snapshot.origin],...contentColumns];
    const csv=[columns.map(([label])=>cell(label)).join(','),...rows.map(row=>columns.map(([,value])=>cell(value(row))).join(','))].join('\r\n'); downloadBlob(new Blob(['\uFEFF',csv],{type:'text/csv;charset=utf-8'}),`open-seo-spider-${state.crawl.id}-content-page-${state.content.cursor}.csv`);
  }
  function bindContentEvents() {
    $('content-source').addEventListener('change',()=>{invalidateContentRequests('Textquelle geändert; vorherige Berechnung ungültig.',true);renderContent();loadContentInventory();});
    $('content-refresh').addEventListener('click',()=>loadContentInventory());
    $('content-prev').addEventListener('click',()=>{if(state.content.back.length)loadContentInventory(state.content.back.at(-1),'back');});
    $('content-next').addEventListener('click',()=>{if(state.content.inventory?.has_more)loadContentInventory(state.content.inventory.next_after_ordinal,'next');});
    for(const id of ['content-search','content-filter']) $(id).addEventListener(id==='content-search'?'input':'change',renderContent);
    $('content-clear').addEventListener('click',()=>{state.content.selected.clear();invalidateContentRequests('Auswahl aufgehoben.');renderContent();});
    for(const id of ['content-analysis-config','content-indexable-only','content-complete-only']) $(id).addEventListener(id==='content-analysis-config'?'input':'change',()=>{invalidateContentRequests('Analyse-Eingaben geändert; vorheriges Ergebnis ungültig.');renderContent();});
    $('content-analyze').addEventListener('click',analyzeSavedContent);
    $('content-analysis-cancel').addEventListener('click',()=>{invalidateContentRequests('Berechnung im Browser abgebrochen; kein neues Ergebnis übernommen.');renderContent();});
    $('content-source-close').addEventListener('click',()=>{state.content.sourceToken?.controller.abort();state.content.sourceToken=null;state.content.source=null;state.content.sourceError='';renderContentSource();});
    $('content-source-json').addEventListener('click',()=>{if(state.content.source)downloadBlob(new Blob([JSON.stringify(state.content.source,null,2)+'\n'],{type:'application/json'}),`open-seo-spider-${state.crawl.id}-content-source.json`);});
    $('content-analysis-json').addEventListener('click',()=>{if(state.content.result)downloadBlob(new Blob([JSON.stringify(state.content.result,null,2)+'\n'],{type:'application/json'}),`open-seo-spider-${state.crawl.id}-content-analysis.json`);});
    for(const id of ['content-result-kind','content-result-document','content-result-n','content-result-search']) $(id).addEventListener(id==='content-result-search'?'input':'change',()=>{state.content.resultPage=0;renderContentAnalysis();});
    $('content-result-prev').addEventListener('click',()=>{state.content.resultPage=Math.max(0,state.content.resultPage-1);renderContentAnalysis();});
    $('content-result-next').addEventListener('click',()=>{state.content.resultPage+=1;renderContentAnalysis();});
  }

  async function previewPolicy() {
    invalidatePolicyPreview();
    let body;
    try {
      const config = parsePolicyConfig($('settings-form').elements.crawl_policy_json.value), raw = $('policy-preview-raw').value;
      if (!raw.length) throw new Error('Mindestens einen Offline-Kandidaten eingeben.');
      const base = $('policy-preview-base').value.trim();
      if (base) { const url = new URL(base); if (!['http:','https:'].includes(url.protocol) || url.username || url.password) throw new Error('Basis-URL muss eine HTTP(S)-URL ohne Zugangsdaten sein.'); }
      const inputs = raw.split(/\r?\n/).map((raw_value, index) => ({ occurrence_id:`ui-probe-${index + 1}`,source_kind:$('policy-preview-kind').value,raw_value,base_url:base || null }));
      if (inputs.length > (config.limits?.max_inputs ?? 10000)) throw new Error('Zu viele Kandidaten für den konfigurierten Preview-Batch.');
      body = { config, inputs };
    } catch (error) { $('policy-preview-error').textContent = error.message; $('policy-preview-error').hidden = false; return; }
    const token = { revision:policyPreview.revision,generation:state.generation,controller:new AbortController() };
    policyPreview.token = token; $('policy-preview-button').disabled = true; $('policy-preview-status').textContent = 'Offline-Probe wird lokal ausgewertet …';
    const current = () => policyPreview.token === token && policyPreview.revision === token.revision && state.generation === token.generation && $('settings-dialog').open;
    try {
      const result = await request('/api/policy/preview', { method:'POST',body:JSON.stringify(body),signal:token.controller.signal });
      if (!current()) return;
      if (result.engine !== 'open-seo-spider/crawl-policy-v1' || !Array.isArray(result.decisions) || result.website_requests !== 0) throw new Error('Unbekanntes oder unvollständiges Offline-Preview-Profil.');
      policyPreview.result = result;
      $('policy-preview-summary').textContent = `${number(result.decisions.length)} Offline-Vorkommen · Website-Requests: 0 · Profil ${result.engine} · Policy-SHA256 ${result.policy_sha256 || 'nicht erfasst'}`;
      $('policy-preview-details').textContent = JSON.stringify({ config:result.config,replacement_profile:result.replacement_profile,limitations:result.limitations,decisions:result.decisions }, null, 2);
      $('policy-preview-table').querySelector('tbody').innerHTML = result.decisions.map(row => `<tr>${[row.occurrence_id,row.raw_value,row.normalized_discovered_url ?? row.resolved_discovered_url,row.fetch_candidate_url,row.fragment,policyDecisionLabel(row.decision),[row.reason_code,row.error,...array(row.warnings)].filter(Boolean).join(' · '),row.coalesced_into].map(value => `<td>${escape(value ?? 'Nicht erfasst')}</td>`).join('')}</tr>`).join('');
      $('policy-preview-results').hidden = false; $('policy-preview-status').textContent = 'Offline-Probe abgeschlossen. Kein Crawl gestartet; native Zulassung und Robots bleiben getrennte Prüfungen.';
    } catch (error) { if (current()) { $('policy-preview-error').textContent = error.message; $('policy-preview-error').hidden = false; $('policy-preview-status').textContent = 'Offline-Probe fehlgeschlagen; kein gültiges Ergebnis.'; } }
    finally { if (policyPreview.token === token) { policyPreview.token = null; $('policy-preview-button').disabled = false; } }
  }

  function resetPolicyLedger() {
    state.policyToken?.controller.abort(); state.policyToken = null; state.policy = null; state.policyError = ''; state.policyCursor = 0; state.policyBack = []; state.policySelected = null; state.policySort = 0; state.policyDirection = 1;
    if ($('policy-search')) { $('policy-search').value = ''; $('policy-filter').value = 'all'; $('policy-inspector').hidden = true; }
  }

  async function loadPolicyPage(cursor = state.policyCursor, navigation = null) {
    if (!state.crawl || state.policyToken || !policySection()) return;
    const token = { id:state.crawl.id,generation:state.generation,cursor,controller:new AbortController() };
    state.policyToken = token; state.policyError = ''; renderPolicyLedger(); renderControls();
    const current = () => state.policyToken === token && token.id === state.crawl?.id && token.generation === state.generation;
    try {
      const result = await request(`/api/crawls/${encodeURIComponent(token.id)}/policy?after_sequence=${encodeURIComponent(cursor)}&limit=200`, { signal:token.controller.signal });
      if (!current()) return;
      if (typeof result.enabled !== 'boolean' || !Array.isArray(result.observations) || typeof result.has_more !== 'boolean' || result.observations.length > 200) throw new Error('Ungültige oder unbeschränkte Policy-Ledger-Seite.');
      if (result.enabled && result.engine !== nativePolicyEngine) throw new Error('Unbekanntes natives Policy-Ledger-Profil. Keine Zulassungen werden daraus abgeleitet.');
      if (result.has_more && (!Number.isSafeInteger(result.next_after_sequence) || result.next_after_sequence <= cursor)) throw new Error('Ungültiger oder nicht fortschreitender Ledger-Cursor.');
      if (result.observations.some(row => !Number.isSafeInteger(row.sequence) || row.sequence <= cursor || !row.occurrence || !row.occurrence.decision)) throw new Error('Policy-Vorkommen ohne gespeicherte Sequenz/Entscheidung.');
      if (navigation === 'next') state.policyBack.push(state.policyCursor);
      else if (navigation === 'back') state.policyBack.pop();
      state.policyCursor = cursor; state.policy = result;
      const selected = state.policySelected && result.observations.find(row => row.sequence === state.policySelected.sequence && row.occurrence.decision.occurrence_id === state.policySelected.occurrence.decision.occurrence_id);
      state.policySelected = selected || null;
      if (!selected) $('policy-inspector').hidden = true;
    } catch (error) { if (current()) state.policyError = `Policy-Ledger nicht verfügbar: ${error.message}`; }
    finally { if (state.policyToken === token) { state.policyToken = null; if (current() || token.id === state.crawl?.id && token.generation === state.generation) { renderPolicyLedger(); renderControls(); } } }
  }

  function policyVisibleRows() {
    if (!state.policy?.enabled || state.policy.ledger_available === false || state.policy.engine !== nativePolicyEngine) return [];
    const search = $('policy-search').value.toLocaleLowerCase(), filter = $('policy-filter').value;
    return state.policy.observations.filter(row => {
      const decision = row.occurrence.decision;
      if (state.section === 'policyaliases' && !(typeof decision.normalized_discovered_url === 'string' && typeof decision.fetch_candidate_url === 'string' && decision.normalized_discovered_url !== decision.fetch_candidate_url)) return false;
      if (filter !== 'all' && (['current','inherited'].includes(filter) ? row.basis !== filter : decision.decision !== filter)) return false;
      return !search || JSON.stringify(row).toLocaleLowerCase().includes(search);
    }).sort((left,right) => { const value = policyColumns[state.policySort][1], a = value(left), b = value(right); if (a == null && b == null) return 0; if (a == null) return 1; if (b == null) return -1; return (typeof a === 'number' && typeof b === 'number' ? a - b : String(a).localeCompare(String(b),'de',{numeric:true})) * state.policyDirection; });
  }

  const policyColumns = [
    ['Sequenz', row => row.sequence],['Generation',row => row.generation],['Ledger-Basis',row => row.basis === 'current' ? 'Aktuelle Beobachtung' : row.basis === 'inherited' ? 'Geerbte Historie' : 'Nicht erfasst'],
    ['Vorkommen-ID',row => row.occurrence.decision.occurrence_id],['Quelle / Rolle',row => `${row.occurrence.decision.source_kind} / ${row.occurrence.task_role}`],['Originalwert',row => row.occurrence.decision.raw_value],
    ['Beobachtete URL',row => row.occurrence.decision.normalized_discovered_url],['Abrufkandidat',row => row.occurrence.decision.fetch_candidate_url],['Fragment',row => row.occurrence.decision.fragment],
    ['Policy-Entscheidung',row => policyDecisionLabel(row.occurrence.decision.decision)],['Task-Zulassung',row => row.occurrence.admission_reason],['Eingeplante Anfrage-URL',row => row.occurrence.scheduled_fetch_url],
    ['Zusammengeführte Task-ID',row => row.occurrence.coalesced_task_occurrence_id],['Grund / Fehler',row => [row.occurrence.input_error,row.occurrence.decision.reason_code,row.occurrence.decision.error].filter(Boolean).join(' · ')],
    ['Abrufkandidat: Origin intern',row => row.occurrence.decision.host_internal],['Abrufkandidat: Pfad intern',row => row.occurrence.decision.folder_internal],['Eingabebasis',row => row.occurrence.input_basis],
    ['Originalwert beobachtet',row => row.occurrence.raw_value_observed],['Quellordinal',row => row.occurrence.source_ordinal],['Quellrevision',row => row.occurrence.source_revision],['Policy-SHA256',row => row.occurrence.decision.policy_sha256]
  ];

  function renderPolicyLedger() {
    if (!policySection()) return;
    const result = state.policy, checkpoint = result?.checkpoint;
    $('policy-error').hidden = !state.policyError; $('policy-error').textContent = state.policyError;
    $('policy-summary').textContent = !state.crawl ? 'Einen gespeicherten Crawl auswählen.' : state.policyToken ? 'Ledger-Seite wird geladen …' : !result ? 'Noch keine Policy-Ledger-Beobachtungen geladen.' : !result.enabled ? 'Keine Policy-Aufzeichnung für diesen Lauf. Historische Befunde bleiben ohne nachträgliche Policy-Zulassung.' : result.ledger_available === false ? `Policy konfiguriert; gespeicherte SQLite-Vorkommen nicht verfügbar. Grund: ${result.reason || 'nicht erfasst'}. Eine importierte Zusammenfassung ist nur deklarierte Metadaten und kein verfügbares leeres oder vollständiges Ledger.` : `Profil ${result.engine} · Policy-SHA256 ${result.policy_sha256 || 'nicht erfasst'} · Revision ${checkpoint?.revision ?? 'nicht erfasst'} · ${checkpoint?.occurrences == null ? 'Anzahl nicht erfasst' : `${number(checkpoint.occurrences)} gespeicherte Vorkommen`} · ${checkpoint?.committed_bytes == null ? 'Speicher nicht erfasst' : `${number(checkpoint.committed_bytes)} Bytes`} · Ledger ${checkpoint?.complete === true ? 'vollständig' : checkpoint?.complete === false ? 'unvollständig' : 'Vollständigkeit unbekannt'}`;
    const history = checkpoint?.inherited_history;
    $('policy-scope').textContent = [result?.scope == null ? 'Ledger-Abdeckung nicht erfasst.' : typeof result.scope === 'string' ? result.scope : JSON.stringify(result.scope), ...(history ? [`Geerbte Historie: ${history.available_occurrences ?? 'unbekannt'} von ${history.expected_occurrences ?? 'unbekannt'} Vorkommen verfügbar; ${history.complete === true ? 'vollständig' : history.complete === false ? 'unvollständig' : 'Vollständigkeit unbekannt'}. Parent-Ledger-SHA256 ${history.parent_ledger_sha256 || 'nicht erfasst'}`] : []), ...(checkpoint ? [`Unvollständige Quellen: ${checkpoint.incomplete_sources ?? 'nicht erfasst'}. Rolling-SHA256 ${checkpoint.rolling_sha256 || 'nicht erfasst'}. Vollständigkeit betrifft das gespeicherte Ledger, nicht alle URLs der Website.`] : [])].join(' ');
    const rows = policyVisibleRows(), table = $('policy-table');
    table.querySelector('thead').innerHTML = `<tr>${policyColumns.map(([label], index) => `<th scope="col" aria-sort="${state.policySort === index ? state.policyDirection === 1 ? 'ascending' : 'descending' : 'none'}"><button type="button" data-policy-sort="${index}">${escape(label)} ${state.policySort === index ? state.policyDirection === 1 ? '↑' : '↓' : '↕'}</button></th>`).join('')}</tr>`;
    const empty = state.policyError ? 'Ledger konnte nicht geladen werden; dies ist kein leeres vollständiges Ergebnis.' : !state.crawl ? 'Kein Crawl gewählt.' : result?.ledger_available === false && result.enabled ? 'Gespeicherte SQLite-Vorkommen sind nicht verfügbar. Vollständige Exporte sind gesperrt; eine importierte Zusammenfassung rekonstruiert keine Ledger-Zeilen.' : !result?.enabled ? 'Keine Policy-Vorkommen erfasst.' : !rows.length ? 'Keine passenden Vorkommen auf der geladenen Ledger-Seite. Weitere Ledger-Seiten können andere Entscheidungen enthalten.' : '';
    table.querySelector('tbody').innerHTML = rows.length && !state.policyError ? rows.map((row,index) => `<tr data-policy-row="${index}" tabindex="0" aria-label="Details zum Policy-Vorkommen ${escape(row.occurrence.decision.occurrence_id)}">${policyColumns.map(([,value]) => `<td>${escape(value(row) ?? 'Nicht erfasst')}</td>`).join('')}</tr>`).join('') : `<tr><td class="empty-cell" colspan="${policyColumns.length}">${escape(empty)}</td></tr>`;
    table.querySelectorAll('[data-policy-sort]').forEach(button => button.addEventListener('click',() => { const sort = Number(button.dataset.policySort); state.policyDirection = state.policySort === sort ? -state.policyDirection : 1; state.policySort = sort; renderPolicyLedger(); }));
    table.querySelectorAll('[data-policy-row]').forEach(element => { const open = () => { state.policySelected = rows[Number(element.dataset.policyRow)]; renderPolicyInspector(); $('policy-inspector').scrollIntoView({block:'nearest'}); }; element.addEventListener('click',open); element.addEventListener('keydown',event => { if (['Enter',' '].includes(event.key)) { event.preventDefault(); open(); } }); });
    renderPolicyInspector();
    const busy = Boolean(state.policyToken), usable = Boolean(result?.enabled && result.ledger_available !== false && !state.policyError && result.engine === nativePolicyEngine);
    $('policy-prev').disabled = busy || !state.policyBack.length; $('policy-next').disabled = busy || !usable || !result.has_more; $('policy-refresh').disabled = busy || !state.crawl;
    $('policy-export-csv').disabled = !usable || busy; $('policy-export-json').disabled = !usable || busy;
    $('policy-page-label').textContent = `${number(rows.length)} gefilterte/sortierte von ${number(result?.observations?.length || 0)} geladenen Vorkommen · nach Sequenz ${state.policyCursor} · maximal 200 pro API-Seite`;
  }

  function renderPolicyInspector() {
    const row = state.policySelected; $('policy-inspector').hidden = !row; if (!row) return;
    $('policy-inspector-basis').textContent = row.basis === 'inherited' ? 'Geerbte Ledger-Historie; kein neuer Abruf in diesem Lauf.' : row.basis === 'current' ? 'Aktuell gespeichertes Policy-Vorkommen. Eine Entscheidung allein belegt keinen gestarteten oder erfolgreichen Abruf.' : 'Ledger-Basis nicht erfasst.';
    $('policy-inspector-data').textContent = JSON.stringify(row,null,2);
    const url = row.occurrence.scheduled_fetch_url; $('policy-open-document').hidden = !url || !state.pageMap.has(url);
  }

  function exportPolicyPage() {
    if (state.policyToken || state.policyError) return;
    const rows = policyVisibleRows(); if (!rows.length) return;
    const cell = input => { let value = String(input ?? ''); if (typeof input !== 'number' && typeof input !== 'boolean' && /^[\s\uFEFF]*[=+@-]/u.test(value)) value = `'${value}`; return `"${value.replace(/"/g,'""')}"`; };
    const data = [policyColumns.map(([label]) => cell(label)).join(','),...rows.map(row => policyColumns.map(([,value]) => cell(value(row))).join(','))].join('\r\n');
    downloadBlob(new Blob(['\uFEFF',data],{type:'text/csv;charset=utf-8'}),`open-seo-spider-${state.crawl.id}-policy-page-after-${state.policyCursor}.csv`);
    message(`${number(rows.length)} gefilterte Vorkommen der geladenen Ledger-Seite exportiert. Vollständige Exporte stehen separat bereit.`);
  }

  function downloadPolicySnapshot(extension) {
    if (!state.crawl || state.policyToken || state.policyError || !state.policy?.enabled || state.policy.ledger_available === false || state.policy.engine !== nativePolicyEngine) return;
    const link = document.createElement('a');
    link.href = `/api/crawls/${encodeURIComponent(state.crawl.id)}/policy.${extension}`;
    link.download = `open-seo-spider-${state.crawl.id}-policy.${extension}`;
    document.body.append(link); link.click(); link.remove();
  }

  function parsePositionRules(text) {
    let rules;
    try { rules = JSON.parse(text || '[]'); } catch (_) { throw new Error('Linkpositionen müssen ein gültiges JSON-Array sein.'); }
    if (!Array.isArray(rules) || rules.length > 32) throw new Error('Linkpositionen: Höchstens 32 geordnete Regeln sind erlaubt.');
    const names = new Set(), bytes = value => new TextEncoder().encode(value).length;
    return rules.map((rule, index) => {
      if (!rule || typeof rule !== 'object' || Array.isArray(rule) || typeof rule.name !== 'string' || typeof rule.selector !== 'string') throw new Error(`Linkposition ${index + 1}: name und selector sind erforderlich.`);
      const name = rule.name.trim(), selector = rule.selector.trim();
      if (!name || bytes(name) > 128 || /[\u0000-\u001f\u007f-\u009f]/.test(name) || names.has(name)) throw new Error(`Linkposition ${index + 1}: Namen müssen eindeutig sein, ohne Steuerzeichen, maximal 128 UTF-8-Bytes.`);
      if (!selector || bytes(selector) > 8192) throw new Error(`Linkposition „${name}“: CSS-Selektor muss 1 bis 8192 UTF-8-Bytes enthalten.`);
      try { document.createDocumentFragment().querySelector(selector); } catch (_) { throw new Error(`Linkposition „${name}“: Ungültiger CSS-Selektor.`); }
      names.add(name); return { name, selector };
    });
  }

  function parsePdfAnalysis(text) {
    let config;
    try { config = JSON.parse(text || '{}'); } catch (_) { throw new Error('PDF-Einstellungen müssen ein gültiges JSON-Objekt sein.'); }
    if (!config || typeof config !== 'object' || Array.isArray(config)) throw new Error('PDF-Einstellungen: Ein Objekt ist erforderlich.');
    const limits = { max_pages: 100000, max_objects: 1000000, max_decompressed_bytes: 128 * 1024 * 1024, max_text_bytes: 64 * 1024 * 1024, max_text_bytes_per_page: 64 * 1024 * 1024, max_annotations: 1000000 };
    for (const [key, value] of Object.entries(config)) {
      if (key === 'enabled') { if (typeof value !== 'boolean') throw new Error('PDF enabled muss true oder false sein.'); }
      else if (!Object.prototype.hasOwnProperty.call(limits, key) || !Number.isSafeInteger(value) || value < 1 || value > limits[key]) throw new Error(`Ungültige PDF-Grenze ${key}; positive Ganzzahl innerhalb der unterstützten Grenze erforderlich.`);
    }
    return config;
  }

  function parseAuditThresholds(text) {
    let thresholds;
    try { thresholds = JSON.parse(text || '{}'); } catch (_) { throw new Error('Prüfgrenzen müssen ein gültiges JSON-Objekt sein.'); }
    const allowed = new Set(['min_title_chars','max_title_chars','min_description_chars','max_description_chars','max_h1_chars','max_h2_chars','max_alt_chars','max_url_chars','min_word_count','max_internal_outlinks','max_external_outlinks','min_title_pixels','max_title_pixels','min_description_pixels','max_description_pixels']);
    if (!thresholds || typeof thresholds !== 'object' || Array.isArray(thresholds)) throw new Error('Prüfgrenzen müssen ein JSON-Objekt sein.');
    for (const [key, value] of Object.entries(thresholds)) {
      if (!allowed.has(key)) throw new Error(`Unbekannte Prüfgrenze: ${key}`);
      if (value !== null && (!Number.isSafeInteger(value) || value < 0 || value > 1000000)) throw new Error(`Prüfgrenze „${key}“ muss null oder eine ganze Zahl von 0 bis 1000000 sein.`);
    }
    for (const kind of ['title_chars','description_chars','title_pixels','description_pixels']) {
      const min = thresholds[`min_${kind}`], max = thresholds[`max_${kind}`];
      if (min != null && max != null && min > max) throw new Error(`Prüfgrenze „${kind}“: Minimum ist größer als Maximum.`);
    }
    return thresholds;
  }

  function parseCustomJavascript(text) {
    let definitions;
    try { definitions = JSON.parse(text || '[]'); }
    catch (_) { throw new Error('JavaScript-Auswertungen müssen ein gültiges JSON-Array sein.'); }
    if (!Array.isArray(definitions) || definitions.length > 32) throw new Error('JavaScript-Auswertungen: Ein Array mit höchstens 32 Einträgen ist erforderlich.');
    const names = new Set(), encoder = new TextEncoder();
    let bytes = 0;
    return definitions.map((definition, index) => {
      if (!definition || typeof definition !== 'object' || Array.isArray(definition) || typeof definition.name !== 'string' || typeof definition.script !== 'string') throw new Error(`JavaScript ${index + 1}: name und script müssen Strings sein.`);
      const name = definition.name.trim();
      if (!name || encoder.encode(name).length > 128 || /[\x00-\x1f\x7f]/.test(name)) throw new Error(`JavaScript ${index + 1}: Ein Name mit höchstens 128 UTF-8-Bytes ohne Steuerzeichen ist erforderlich.`);
      if (names.has(name)) throw new Error(`Der JavaScript-Name „${name}“ ist mehrfach vorhanden.`);
      names.add(name);
      const script = definition.script;
      const size = encoder.encode(script).length;
      if (!script.trim() || size > 65536) throw new Error(`JavaScript „${name}“: Ein nicht leeres Script mit höchstens 65536 UTF-8-Bytes ist erforderlich.`);
      bytes += size;
      if (bytes > 262144) throw new Error('JavaScript-Auswertungen dürfen insgesamt höchstens 262144 UTF-8-Bytes enthalten.');
      return { name, script };
    });
  }

  function validateRendererSettings(settings) {
    if (!settings.javascript_rendering) return;
    let endpoint;
    try { endpoint = new URL(settings.renderer_endpoint); } catch (_) { throw new Error('Der Browser-Worker benötigt einen gültigen lokalen HTTP-Endpunkt.'); }
    if (endpoint.protocol !== 'http:' || !['127.0.0.1', 'localhost', '[::1]', '::1'].includes(endpoint.hostname)) throw new Error('Der Browser-Worker muss per HTTP auf 127.0.0.1, localhost oder ::1 erreichbar sein.');
    if (!Number.isSafeInteger(settings.renderer_wait_ms) || settings.renderer_wait_ms < 0 || settings.renderer_wait_ms > 30000) throw new Error('Die Render-Wartezeit muss zwischen 0 und 30000 ms liegen.');
    if (settings.renderer_wait_ms >= Math.min(settings.timeout_seconds * 1000, 120000)) throw new Error('Die Render-Wartezeit muss kürzer sein als der konfigurierte Timeout.');
  }

  function toggleRendererSettings(enabled) {
    $('renderer-settings').hidden = !enabled;
    $('renderer-settings').querySelectorAll('input, textarea').forEach((input) => { input.disabled = !enabled; });
  }

  function listOptions() {
    return window.ListImportTools.buildListOptions({ kind: $('list-format').value, delimiter: $('list-delimiter').value,
      column_kind: $('list-column-kind').value, column_index: $('list-column-index').value,
      header_name: $('list-header-name').value, header_row: $('list-header-row').value, start_row: $('list-start-row').value,
      sheet_kind: 'index', sheet_index: $('list-sheet').value, formula_policy: $('list-formula-policy').value });
  }

  function syncListControls() {
    const kind = $('list-format').value, columns = ['csv', 'xlsx'].includes(kind), header = columns && $('list-column-kind').value === 'header';
    for (const [id, show] of [['list-delimiter-control', kind === 'csv'], ['list-sheet-control', kind === 'xlsx'], ['list-column-kind-control', columns], ['list-column-index-control', columns && !header], ['list-header-name-control', header], ['list-header-row-control', header], ['list-start-row-control', columns], ['list-formula-control', kind === 'xlsx']]) $(id).hidden = !show;
    $('list-inspect').hidden = kind !== 'xlsx';
  }

  function updateListOptions() {
    if (!listUI.controller) return;
    listUI.page = 0; syncListControls(); listUI.controller.setOptions(null);
    try { listUI.controller.setOptions(listOptions()); }
    catch (error) { $('list-import-error').textContent = error.message; $('list-import-error').hidden = false; }
  }

  function renderListPreview(snapshot = listUI.snapshot) {
    if (!snapshot) return;
    listUI.snapshot = snapshot;
    const busy = ['reading', 'inspecting', 'parsing'].includes(snapshot.phase);
    $('list-inspect').disabled = busy || !snapshot.source;
    $('list-preview').disabled = busy || !snapshot.source || !snapshot.options;
    $('list-accept').disabled = snapshot.phase !== 'preview' || !snapshot.result || !window.ListImportTools.summarizeRows(snapshot.result.rows).accepted;
    $('list-import-error').hidden = !snapshot.error;
    $('list-import-error').textContent = snapshot.error;
    $('list-source-info').textContent = snapshot.source ? `${snapshot.source.name} · ${number(snapshot.source.bytes)} Byte` : '';
    $('list-import-status').textContent = ({ empty: 'Noch keine Datei ausgewählt.', reading: 'Lokale Datei wird gelesen …', selected: 'Quelle ausgewählt; noch keine aktuelle Vorschau übernommen.', inspecting: 'Workbook wird lokal geprüft …', parsing: 'Quelle wird lokal gelesen und validiert …', preview: 'Vorschau bereit. Noch kein Crawl gestartet.', accepted: 'Geprüfte Kandidaten übernommen. Crawl separat starten.', error: 'Import nicht erfolgreich; kein Ergebnis übernommen.' })[snapshot.phase] || 'Befund nicht erfasst.';
    if (snapshot.inspection) {
      const selected = $('list-sheet').value;
      $('list-sheet').innerHTML = '<option value="">Worksheet ausdrücklich auswählen</option>' + snapshot.inspection.sheets.map(sheet => `<option value="${escape(sheet.index)}"${sheet.sheet_type === 'WorkSheet' ? '' : ' disabled'}>${escape(sheet.name)} · ${escape(sheet.visibility)} · ${escape(sheet.sheet_type)}</option>`).join('');
      $('list-sheet').value = selected;
    }
    const result = snapshot.result, rows = array(result?.rows);
    $('list-preview-panel').hidden = !result;
    $('list-origin-state').textContent = snapshot.accepted ? `Übernommener ${snapshot.accepted.options?.format?.kind?.toUpperCase() || 'Import'} · ${number(rows.length)} Quellzeilen erhalten` : 'Manueller Text · wird vor dem Crawl lokal geprüft';
    if (!result) return;
    const summary = window.ListImportTools.summarizeRows(rows);
    $('list-preview-summary').textContent = `${number(summary.rows)} Quellzeilen · ${number(summary.accepted)} akzeptierte Kandidaten · ${number(summary.invalid)} ungültig · ${number(summary.duplicates)} Wiederholungen · ${number(summary.page_urls.length)} eindeutige Seiten / ${number(summary.sitemap_urls.length)} Sitemap-Dokumente`;
    $('list-preview-details').textContent = JSON.stringify({ engine: result.engine, input_sha256: result.input_sha256, input_bytes: result.input_bytes, options: result.options, selected_sheet: result.selected_sheet, selected_column: result.selected_column, website_requests: result.website_requests, warnings: result.warnings, limitations: result.limitations }, null, 2);
    const pages = Math.max(1, Math.ceil(rows.length / 50)); listUI.page = Math.max(0, Math.min(listUI.page, pages - 1));
    const seen = new Set(), duplicate = rows.map(row => { if (row.error != null || !row.normalized_url) return false; const repeated = seen.has(row.normalized_url); seen.add(row.normalized_url); return repeated; });
    const offset = listUI.page * 50, known = value => Number.isSafeInteger(value) && value >= 0 ? number(value) : 'Nicht erfasst';
    $('list-preview-table').querySelector('tbody').innerHTML = rows.slice(offset, offset + 50).map((row, index) => `<tr>${[known(row.source_index), known(row.source_row), known(row.source_line), [row.sheet, row.cell].filter(value => value != null).join(' / ') || 'Nicht erfasst', row.raw_value == null ? 'Nicht erfasst' : row.raw_value === '' ? '[leer]' : row.raw_value, row.candidate_kind === 'page_url' ? 'Seiten-URL' : row.candidate_kind === 'sitemap_document' ? 'Sitemap-Dokument' : 'Nicht erfasst', row.normalized_url ?? 'Nicht gültig / erfasst', row.fragment ?? 'Nicht erfasst', [row.cell_type || 'Nicht erfasst', row.formula_metadata == null ? '' : JSON.stringify(row.formula_metadata)].filter(Boolean).join(' · '), duplicate[offset + index] ? 'Wiederholung' : '—', row.error ?? '—'].map((value, cell) => `<td${cell === 10 && row.error != null ? ' class="import-error"' : ''}>${escape(value)}</td>`).join('')}</tr>`).join('') || '<tr><td colspan="11">Keine Quellzeilen erfasst.</td></tr>';
    $('list-page-label').textContent = rows.length ? `${number(offset + 1)}–${number(Math.min(offset + 50, rows.length))} von ${number(rows.length)}` : 'Keine Ergebnisse';
    $('list-page-prev').disabled = listUI.page === 0; $('list-page-next').disabled = listUI.page + 1 >= pages;
  }

  function bindListImporter() {
    if (!window.ListImportTools) { $('list-import-open').disabled = true; return; }
    listUI.controller = new window.ListImportTools.ImportPreviewState({ transport: (path, body) => request(path, { method: 'POST', body: JSON.stringify(body) }), emit: renderListPreview });
    $('list-import-open').addEventListener('click', () => { syncListControls(); renderListPreview(listUI.controller.state); $('list-import-dialog').showModal(); });
    $('list-import-close').addEventListener('click', () => { listUI.controller.cancelPending(); $('list-import-dialog').close(); });
    $('list-import-dialog').addEventListener('cancel', () => listUI.controller.cancelPending());
    $('list-file').addEventListener('change', async event => {
      const file = event.target.files[0]; if (!file) return;
      const extension = file.name.split('.').pop().toLowerCase();
      if (['xls', 'xlsm', 'xlsb', 'ods'].includes(extension)) { listUI.controller.setEditedText(''); $('list-import-error').textContent = 'Dieses Workbook-Format wird nicht unterstützt. Bitte als XLSX speichern.'; $('list-import-error').hidden = false; return; }
      if (['txt', 'csv', 'xml', 'xlsx'].includes(extension)) $('list-format').value = extension;
      $('list-sheet').innerHTML = '<option value="">Zuerst Workbook prüfen</option>'; listUI.page = 0; listUI.acceptedText = null;
      const source = await listUI.controller.setFile(file); if (source) updateListOptions();
    });
    for (const id of ['list-format', 'list-delimiter', 'list-sheet', 'list-column-kind', 'list-column-index', 'list-header-name', 'list-header-row', 'list-start-row', 'list-formula-policy']) $(id).addEventListener('change', updateListOptions);
    $('list-inspect').addEventListener('click', async () => { try { await listUI.controller.inspect(); } catch (error) { message(error.message, true); } });
    $('list-import-form').addEventListener('submit', async event => { event.preventDefault(); updateListOptions(); if (!listUI.controller.state.options) return; await listUI.controller.preview(); });
    $('list-accept').addEventListener('click', () => {
      try { const handoff = listUI.controller.accept(); const imported = handoff.input_imports[0]; $('url-list').value = imported.rows.filter(row => row.error == null && row.normalized_url).map(row => row.normalized_url).join('\n'); listUI.acceptedText = $('url-list').value; $('list-import-dialog').close(); }
      catch (error) { $('list-import-error').textContent = error.message; $('list-import-error').hidden = false; }
    });
    $('url-list').addEventListener('input', () => { listUI.acceptedText = null; $('list-format').value = 'txt'; $('list-file').value = ''; listUI.page = 0; listUI.controller.setEditedText($('url-list').value); syncListControls(); });
    $('list-page-prev').addEventListener('click', () => { listUI.page--; renderListPreview(); }); $('list-page-next').addEventListener('click', () => { listUI.page++; renderListPreview(); });
    renderListPreview(listUI.controller.state);
  }

  function bindEvents() {
    bindResourceEvents();
    bindContentEvents();
    $('policy-settings').addEventListener('input', invalidatePolicyPreview);
    $('policy-settings').addEventListener('change', invalidatePolicyPreview);
    $('policy-preview-button').addEventListener('click', previewPolicy);
    $('settings-dialog').addEventListener('close', invalidatePolicyPreview);
    $('settings-dialog').addEventListener('cancel', invalidatePolicyPreview);
    $('policy-search').addEventListener('input', () => { renderPolicyLedger(); renderControls(); });
    $('policy-filter').addEventListener('change', () => { renderPolicyLedger(); renderControls(); });
    $('policy-refresh').addEventListener('click', () => loadPolicyPage());
    $('policy-next').addEventListener('click', () => { if (state.policy?.has_more) loadPolicyPage(state.policy.next_after_sequence, 'next'); });
    $('policy-prev').addEventListener('click', () => { if (state.policyBack.length) loadPolicyPage(state.policyBack[state.policyBack.length - 1], 'back'); });
    $('policy-inspector-close').addEventListener('click', () => { state.policySelected = null; $('policy-inspector').hidden = true; });
    $('policy-open-document').addEventListener('click', () => { const url = state.policySelected?.occurrence.scheduled_fetch_url; if (url && state.pageMap.has(url)) selectPage(state.pageMap.get(url)); });
    $('policy-export-csv').addEventListener('click', () => downloadPolicySnapshot('csv'));
    $('policy-export-json').addEventListener('click', () => downloadPolicySnapshot('json'));
    $('recrawl-selected').addEventListener('click', openSelectedRecrawl);
    $('recrawl-clear').addEventListener('click', () => { if (actionBusy()) return; state.recrawlURLs.clear(); renderControls(); if (!['overview', 'graph'].includes(state.section)) renderTable(); });
    $('inspector-recrawl-checkbox').addEventListener('change', event => { if (state.selected) setRecrawlSelected(state.selected.url, event.target.checked); });
    $('selected-recrawl-form').addEventListener('submit', selectedRecrawl);
    const closeRecrawl = () => { selectedRecrawlContext = null; $('selected-recrawl-dialog').close(); };
    $('selected-recrawl-close').addEventListener('click', closeRecrawl);
    $('selected-recrawl-cancel').addEventListener('click', closeRecrawl);
    $('selected-recrawl-dialog').addEventListener('cancel', () => { selectedRecrawlContext = null; });
    $('recrawl-parent').addEventListener('click', () => { const id = recrawlOperation()?.parent?.job_id; if (id) selectCrawl(id).then(() => { $('crawl-history').value = state.crawl?.id || ''; }); });
    $('recrawl-parent-snapshot').addEventListener('click', () => serverDownload('parent.json', `open-seo-spider-${state.crawl.id}-originalbericht.json`));
    bindListImporter();
    $('crawl-form').addEventListener('submit', startCrawl);
    $('pause-button').addEventListener('click', pauseCrawl);
    $('cancel-button').addEventListener('click', cancelCrawl);
    $('resume-button').addEventListener('click', resumeCrawl);
    $('resume-limit-close').addEventListener('click', () => { limitResumeContext = null; $('resume-limit-dialog').close(); });
    $('resume-limit-dialog').addEventListener('cancel', () => { limitResumeContext = null; });
    $('resume-limit-form').addEventListener('submit', event => {
      event.preventDefault();
      const maxPages = Number($('resume-max-pages').value), expected = limitResumeContext;
      if (!$('resume-max-pages').value.trim() || !Number.isSafeInteger(maxPages) || maxPages < 0) { $('resume-limit-error').textContent = 'Eine nichtnegative ganze URL-Grenze eingeben. 0 hebt die konfigurierte Grenze auf.'; $('resume-limit-error').hidden = false; return; }
      if (!expected || state.crawl?.id !== expected.id || state.generation !== expected.generation) { $('resume-limit-dialog').close(); return; }
      limitResumeContext = null; $('resume-limit-dialog').close(); controlCrawl('resume', { [expected.budgetKey || 'max_pages']: maxPages }, expected);
    });
    $('crawl-mode').addEventListener('change', () => {
      const list = $('crawl-mode').value === 'list';
      $('list-control').hidden = !list;
      $('seed-url').required = !list;
      $('seed-url').closest('.seed-control').hidden = list;
      $('seed-url').labels[0].textContent = $('crawl-mode').value === 'sitemap' ? 'Sitemap-URL' : 'Start-URL';
      $('seed-url').placeholder = $('crawl-mode').value === 'sitemap' ? 'https://example.com/sitemap.xml' : 'https://example.com';
    });
    $('crawl-history').addEventListener('change', (event) => selectCrawl(event.target.value));
    document.querySelectorAll('[data-section]').forEach((button) => button.addEventListener('click', () => selectSection(button.dataset.section)));
    let searchTimer;
    $('table-search').addEventListener('input', (event) => {
      state.search = event.target.value;
      state.page = 0;
      clearTimeout(searchTimer);
      searchTimer = setTimeout(renderTable, 100);
    });
    $('table-filter').addEventListener('change', (event) => { state.filter = event.target.value; state.page = 0; renderTable(); });
    $('page-size').addEventListener('change', (event) => { state.pageSize = Number(event.target.value); state.page = 0; renderTable(); });
    $('page-prev').addEventListener('click', () => { state.page -= 1; renderTable(); });
    $('page-next').addEventListener('click', () => { state.page += 1; renderTable(); });
    $('inspector-close').addEventListener('click', () => { state.selected = null; state.selectedResponse = null; state.selectedInput = null; $('inspector').hidden = true; if (!['overview', 'graph'].includes(state.section)) renderTable(); });
    const tabs = [...document.querySelectorAll('[data-tab]')];
    tabs.forEach((button, index) => {
      button.addEventListener('click', () => { state.inspectorTab = button.dataset.tab; renderInspector(); });
      button.addEventListener('keydown', (event) => {
        if (!['ArrowRight', 'ArrowLeft', 'Home', 'End'].includes(event.key)) return;
        event.preventDefault();
        const target = event.key === 'Home' ? 0 : event.key === 'End' ? tabs.length - 1 : (index + (event.key === 'ArrowRight' ? 1 : -1) + tabs.length) % tabs.length;
        tabs[target].focus();
        state.inspectorTab = tabs[target].dataset.tab;
        renderInspector();
      });
    });
    $('settings-open').addEventListener('click', openSettings);
    $('settings-inline').addEventListener('click', openSettings);
    $('settings-close').addEventListener('click', () => $('settings-dialog').close());
    $('settings-form').addEventListener('submit', saveSettings);
    $('settings-form').elements.javascript_rendering.addEventListener('change', (event) => toggleRendererSettings(event.target.checked));
    $('export-csv').addEventListener('click', exportCSV);
    $('export-resources').addEventListener('click', () => serverDownload(domSections.includes(state.section) ? 'dom.csv' : ({ inputs: 'inputs.csv', pdf: 'pdf.csv', responses: 'responses.csv', fragments: 'fragments.csv', linkdom: 'links.csv', imagefiles: 'images.csv' })[state.section], `open-seo-spider-${state.crawl.id}-${state.section}.csv`));
    $('import-project').addEventListener('click', () => $('import-file').click());
    $('import-file').addEventListener('change', (event) => importProject(event.target.files[0]));
    $('export-sitemap').addEventListener('click', () => serverDownload('sitemap.xml', `open-seo-spider-${state.crawl.id}-sitemap.xml`));
    $('export-project').addEventListener('click', () => serverDownload('project.json', `open-seo-spider-${state.crawl.id}.json`));
    document.addEventListener('visibilitychange', () => { if (!document.hidden && active() && !state.polling) refresh(); });
  }

  async function initialize() {
    $('advanced-config').append($('policy-settings-template').content.cloneNode(true));
    $('advanced-config').append($('content-settings-template').content.cloneNode(true));
    $('advanced-config').append($('resource-discovery-settings-template').content.cloneNode(true));
    restoreSettings();
    bindEvents();
    selectSection('overview');
    render();
    try {
      await loadHistory();
      const running = state.crawls.find((crawl) => ['running', 'queued', 'pending'].includes(crawl.status));
      if (running) { await selectCrawl(running.id); $('crawl-history').value = running.id; }
    } catch (error) { connection(false); message(`Der lokale Crawler-Server ist nicht erreichbar: ${error.message}`, true); }
  }

  initialize();
})();
