'use strict';

(() => {
  const $ = (id) => document.getElementById(id);
  const state = {
    crawl: null, crawls: [], pages: [], issues: [], section: 'overview',
    search: '', filter: 'all', sort: 'url', direction: 1, page: 0, pageSize: 50,
    selected: null, selectedResponse: null, responses: null, responsesError: '', inspectorTab: 'details', polling: false, starting: false,
    generation: 0, issueMap: new Map(), pageMap: new Map(), cache: {}, analysis: null, analysisError: '', settings: {
      concurrency: 8, timeout_seconds: 20, max_pages: 0, max_depth: 0,
      user_agent: 'Open SEO Spider/0.1', indexability_agent: 'googlebot', obey_robots: true, follow_subdomains: false,
      delay_ms: 100, max_body_mb: 5, check_external: false,
      javascript_rendering: false, renderer_endpoint: 'http://127.0.0.1:8790/render',
      renderer_wait_ms: 500, renderer_accessibility: false, renderer_screenshot: false,
      renderer_collect_cookies: false, custom_javascript_json: '[]',
      include_patterns_text: '', exclude_patterns_text: '', custom_extractions_json: '[]', audit_thresholds_json: '{}', pdf_analysis_json: '{}'
    }
  };
  const sections = {
    overview: ['CRAWL INTELLIGENCE', 'Übersicht', 'Ein Blick auf den technischen Zustand deiner Website.'],
    internal: ['URL INVENTAR', 'Interne URLs', 'Alle erfassten URLs im Crawl-Bereich mit ihrem tatsächlichen Abrufstatus.'],
    external: ['URL INVENTAR', 'Externe Links', 'Entdeckte Linkziele außerhalb des Crawl-Bereichs. Ungeprüfte Ziele haben keinen HTTP-Status.'],
    status: ['TECHNISCHE ANALYSE', 'HTTP & Indexierung', 'Antwortcodes, Weiterleitungen, Abrufdauer und Indexierbarkeit.'],
    responses: ['HTTP BEOBACHTUNGEN', 'Antworten', 'Jede erfasste native HTTP-Antwort einschließlich Zwischenredirects. Nicht gespeicherte Header und Body bleiben unbekannt; Dokumentdaten stehen separat.'],
    titles: ['ONPAGE ANALYSE', 'Seitentitel', 'Title-Tags, Zeichenlänge und mehrfach verwendete Titel. Längen sind Hinweise.'],
    meta: ['ONPAGE ANALYSE', 'Meta Descriptions', 'Descriptions, Zeichenlänge und Duplikate im abgerufenen HTML.'],
    headings: ['ONPAGE ANALYSE', 'Überschriften', 'H1- und H2-Struktur deiner Seiten, einschließlich mehrfacher Überschriften.'],
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
    paused: 'Crawl pausiert', interrupted: 'Crawl unterbrochen', cancelling: 'Crawl wird gestoppt'
  };
  const number = (value) => Number(value || 0).toLocaleString('de-DE');
  const array = (value) => Array.isArray(value) ? value : value == null || value === '' ? [] : [value];
  const first = (value) => array(value).join(' | ');
  const primary = (value) => String(array(value)[0] ?? '');
  const canonicalCount = (page) => array(page.html_canonical).length + array(page.header_canonical).length || page.canonical.length;
  const escape = (value) => String(value ?? '').replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[character]));
  const duration = (ms) => ms == null ? '—' : `${number(Math.round(Number(ms)))} ms`;
  const effectiveStatus = (page) => page.final_status ?? page.status ?? page.status_code ?? null;
  const resourceStatus = (status) => ({ complete: 'Erfasst', partial: 'Unvollständig', error: 'Fehler', encrypted: 'Verschlüsselt', disabled: 'Deaktiviert' }[status] || 'Nicht erfasst');
  const isPDF = (page) => Boolean(page.pdf) || /\bapplication\/pdf\b/i.test(page.content_type);
  const isImageFile = (page) => Boolean(page.image_metadata) || /^image\//i.test(page.content_type);
  const statusBadge = (status) => status == null ? '<span class="badge neutral">—</span>' : `<span class="badge ${Number(status) >= 400 ? 'danger' : Number(status) >= 300 ? 'warning' : 'good'}">${escape(status)}</span>`;
  const active = () => state.crawl && ['running', 'queued', 'pending', 'cancelling'].includes(state.crawl.status);
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
  const isHTML = (page) => /(?:text\/html|application\/xhtml)/i.test(page.content_type) || (!page.content_type && (page.titles.length > 0 || page.h1.length > 0));
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
    const response = await request('/api/crawls');
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
    if (!state.crawl || state.polling) return;
    state.polling = true;
    const id = state.crawl.id;
    try {
      const [crawlResult, issueResult] = await Promise.all([
        request(`/api/crawls/${encodeURIComponent(id)}`),
        request(`/api/crawls/${encodeURIComponent(id)}/issues`)
      ]);
      if (generation !== state.generation) return;
      const previousStatus = state.crawl.status;
      const nextCrawl = crawlResult.crawl || crawlResult;
      const terminal = !['running', 'queued', 'pending', 'cancelling'].includes(nextCrawl.status);
      const from = terminal || !['running', 'queued', 'pending'].includes(previousStatus) ? 0 : state.pages.length;
      const pages = await loadPages(id, generation, from);
      if (generation !== state.generation || !pages) return;
      state.crawl = nextCrawl;
      state.pages = from ? [...state.pages, ...pages] : pages;
      state.pageMap = new Map(state.pages.map((page) => [page.url, page]));
      state.cache = {};
      state.issues = Array.isArray(issueResult) ? issueResult : issueResult.issues || issueResult.items || [];
      if (terminal && !state.analysis) {
        try { state.analysis = await request(`/api/crawls/${encodeURIComponent(id)}/analysis`); state.analysisError = ''; }
        catch (error) { state.analysisError = `Weiterführende Analyse nicht verfügbar: ${error.message}`; }
        if (generation !== state.generation) return;
      }
      if (terminal && state.responses == null) {
        try {
          const responseResult = await request(`/api/crawls/${encodeURIComponent(id)}/responses`);
          if (generation !== state.generation) return;
          if (!Array.isArray(responseResult.responses)) throw new Error('Ungültiges Antworten-Inventar');
          state.responses = responseResult.responses;
          state.responsesError = '';
        } catch (error) { if (generation === state.generation) state.responsesError = `Antworten-Inventar nicht verfügbar: ${error.message}`; }
      }
      if (state.analysis) state.issues.push(...array(state.analysis.issues));
      reindexIssues();
      connection(true);
      render();
      if (state.selectedResponse) renderInspector();
      else if (state.selected) {
        state.selected = state.pages.find((page) => page.url === state.selected.url) || state.selected;
        renderInspector();
      }
      if (previousStatus !== state.crawl.status && !active()) {
        await loadHistory();
        if (state.crawl.status === 'failed') message(state.crawl.error || 'Der Crawl ist fehlgeschlagen. Die bisherigen Daten bleiben sichtbar.', true);
      }
    } catch (error) {
      if (generation === state.generation) { connection(false); message(`Crawl-Daten konnten nicht geladen werden: ${error.message}`, true); }
    } finally {
      state.polling = false;
      if (generation === state.generation && active()) setTimeout(() => refresh(generation), 2000);
    }
  }

  async function selectCrawl(id) {
    state.generation += 1;
    state.pages = [];
    state.issues = [];
    state.selected = null;
    state.selectedResponse = null;
    state.responses = null;
    state.responsesError = '';
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
      // An older polling request may still be resolving; allow its generation check to discard it.
      while (state.polling) await new Promise((resolve) => setTimeout(resolve, 25));
      if (generation === state.generation) await refresh(generation);
    }
  }

  async function startCrawl(event) {
    event.preventDefault();
    if (state.starting || active()) return;
    const mode = $('crawl-mode').value;
    const urls = mode === 'list' ? [...new Set($('url-list').value.split(/\r?\n/).map((url) => url.trim()).filter(Boolean))] : [$('seed-url').value.trim()];
    if (!urls.length) { message('Bitte gib mindestens eine URL ein.', true); return; }
    for (const url of urls) {
      try { const parsed = new URL(url); if (!['http:', 'https:'].includes(parsed.protocol)) throw new Error('scheme'); }
      catch (_) { message(`Keine gültige HTTP(S)-URL: ${url}`, true); return; }
    }
    let advanced;
    try { advanced = parseAdvanced(state.settings); validateRendererSettings(state.settings); }
    catch (error) { message(`Ungültige Crawl-Einstellung: ${error.message}`, true); return; }
    state.starting = true;
    $('start-button').disabled = true;
    message('');
    try {
      const payload = { ...state.settings, ...advanced, mode, seed_url: mode === 'sitemap' ? '' : urls[0], urls: mode === 'sitemap' ? [] : urls, seed_urls: mode === 'sitemap' ? [] : urls, sitemap_urls: mode === 'sitemap' ? urls : [], crawl_links: mode !== 'list', include_subdomains: state.settings.follow_subdomains, request_timeout_secs: state.settings.timeout_seconds, max_body_bytes: state.settings.max_body_mb * 1024 * 1024 };
      payload.renderer = state.settings.javascript_rendering ? { endpoint: state.settings.renderer_endpoint, wait_ms: state.settings.renderer_wait_ms, accessibility: state.settings.renderer_accessibility, screenshot: state.settings.renderer_screenshot, collect_cookies: state.settings.renderer_collect_cookies, custom_javascript: parseCustomJavascript(state.settings.custom_javascript_json) } : null;
      const result = await request('/api/crawls', { method: 'POST', body: JSON.stringify(payload) });
      const crawl = result.crawl || result;
      if (!crawl.id) throw new Error('Der Server hat keine Crawl-ID zurückgegeben.');
      state.crawls.unshift(crawl);
      renderHistory();
      await selectCrawl(crawl.id);
      $('crawl-history').value = crawl.id;
    } catch (error) { message(`Der Crawl konnte nicht gestartet werden: ${error.message}`, true); }
    finally { state.starting = false; renderControls(); }
  }

  async function cancelCrawl() {
    if (!state.crawl || !active()) return;
    $('cancel-button').disabled = true;
    try {
      await request(`/api/crawls/${encodeURIComponent(state.crawl.id)}/cancel`, { method: 'POST' });
      message('Stopp angefordert. Bereits laufende Requests werden beendet; die erfassten Daten bleiben erhalten.');
      await refresh();
    } catch (error) { message(`Der Crawl konnte nicht gestoppt werden: ${error.message}`, true); }
    finally { $('cancel-button').disabled = false; }
  }

  async function resumeCrawl() {
    if (!state.crawl || active()) return;
    $('resume-button').disabled = true;
    try {
      await request(`/api/crawls/${encodeURIComponent(state.crawl.id)}/resume`, { method: 'POST' });
      message('Crawl wird aus dem lokal gespeicherten Stand fortgesetzt.');
      state.generation += 1;
      state.analysis = null;
      state.analysisError = '';
      state.responses = null;
      state.responsesError = '';
      state.selectedResponse = null;
      const generation = state.generation;
      while (state.polling) await new Promise((resolve) => setTimeout(resolve, 25));
      if (generation === state.generation) await refresh(generation);
    } catch (error) { message(`Der Crawl konnte nicht fortgesetzt werden: ${error.message}`, true); }
    finally { $('resume-button').disabled = false; }
  }

  function renderControls() {
    $('start-button').disabled = Boolean(state.starting || active());
    $('cancel-button').hidden = !active();
    $('resume-button').hidden = !state.crawl || !['cancelled', 'canceled', 'failed', 'interrupted'].includes(state.crawl.status);
    $('crawl-mode').disabled = Boolean(active());
    $('seed-url').disabled = Boolean(active());
    $('url-list').disabled = Boolean(active());
    const hasData = Boolean(state.crawl && state.pages.length);
    $('export-csv').disabled = (!hasData && !state.issues.length) || state.section === 'responses' && state.responses == null;
    $('export-sitemap').disabled = !hasData || Boolean(active());
    $('export-project').disabled = !state.crawl || Boolean(active());
    $('settings-summary').textContent = `${state.settings.concurrency} parallele Requests · ${state.settings.obey_robots ? 'Robots.txt beachten' : 'Robots.txt ignorieren'}${state.settings.max_pages ? ` · Grenze ${number(state.settings.max_pages)}` : ''}${state.settings.javascript_rendering ? ' · JavaScript-Rendering' : ''}`;
  }

  function renderMetrics() {
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
    $('metric-failed').textContent = `${number(failed)} fehlgeschlagene Abrufe`;
    $('metric-indexable').textContent = `${number(html.filter((page) => page.indexable === true).length)} indexierbar`;
    const started = state.crawl?.started_at || state.crawl?.created_at;
    const finished = state.crawl?.finished_at;
    const elapsed = started ? Math.max(0, Math.floor(((finished ? new Date(finished).getTime() : Date.now()) - new Date(started).getTime()) / 1000)) : 0;
    $('metric-progress').textContent = state.crawl ? `${number(state.pages.length)} Datensätze · ${Number.isFinite(elapsed) ? `${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, '0')} min` : '—'}` : 'Noch keine URLs';
    const status = state.crawl?.status || 'idle';
    const notices = [...array(state.crawl?.warnings), ...(state.crawl?.error ? [state.crawl.error] : []), ...(state.analysisError ? [state.analysisError] : []), ...(state.crawl?.summary?.limited ? ['Die konfigurierte Crawl-Grenze wurde erreicht. Die Website wurde nicht vollständig erfasst.'] : []), ...(state.crawl?.summary?.pending ? [`${number(state.crawl.summary.pending)} entdeckte URLs wurden in diesem Crawl noch nicht verarbeitet.`] : [])];
    $('crawl-notices').hidden = !notices.length;
    $('crawl-notices').querySelector('ul').innerHTML = notices.map((notice) => `<li>${escape(notice)}</li>`).join('');
    const statusClass = ['completed', 'complete', 'finished'].includes(status) ? 'completed' : status;
    $('crawl-state').innerHTML = `<span class="status-dot ${escape(statusClass)}"></span>${escape(labels[status] || (state.crawl ? status : 'Bereit für deinen ersten Crawl'))}`;
    const fraction = discovered ? Math.min(1, fetched / discovered) : 0;
    $('progress-bar').style.width = `${state.crawl && ['completed', 'complete', 'finished'].includes(status) ? 100 : fraction * 100}%`;
    $('progress-bar').style.backgroundColor = status === 'failed' ? 'var(--danger)' : ['cancelled', 'canceled', 'cancelling'].includes(status) ? 'var(--warning)' : '';
    if (active()) $('crawl-state').insertAdjacentHTML('beforeend', '<span>· Warteschlange kann während des Crawls wachsen</span>');
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

  function getRows() {
    switch (state.section) {
      case 'external': return externalRows();
      case 'images': return imageRows();
      case 'imagefiles': return imageFileRows();
      case 'pdf': return pdfRows();
      case 'responses': return array(state.responses).map(response => ({ ...response, url: response.response_url }));
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

  function columns() {
    switch (state.section) {
      case 'external': return [urlColumn, { key: 'status', label: 'HTTP geprüft', value: (row) => row.status, render: (row) => statusBadge(row.status) }, textColumn('text', 'Linktext'), numberColumn('source_count', 'Verlinkende Seiten'), textColumn('nofollow', 'Nofollow', (row) => row.nofollow ? 'Ja' : 'Nein')];
      case 'status': return [urlColumn, { key: 'initial_status', label: 'HTTP initial', value: (row) => row.status, render: (row) => statusBadge(row.status) }, statusColumn, textColumn('final_url', 'Finale URL'), numberColumn('redirects', 'Redirects', (row) => row.redirect_chain.length), { key: 'response_ms', label: 'Antwortzeit', value: (row) => row.response_ms, render: (row) => duration(row.response_ms) }, indexColumn, textColumn('error', 'Abruffehler')];
      case 'responses': return [urlColumn, textColumn('requested_url', 'Anfrage-URL'), numberColumn('response_index', 'Antwortposition'), { key: 'status', label: 'HTTP beobachtet', value: row => row.status, render: row => statusBadge(row.status) }, textColumn('location', 'Location'), textColumn('capture_kind', 'Befund', row => row.capture_kind === 'redirect_hop' ? 'Zwischenredirect' : 'Terminale Antwort'), textColumn('headers_known', 'Header erfasst', row => row.headers == null ? 'Unbekannt' : 'Ja'), textColumn('content_type', 'Content-Type'), numberColumn('body_bytes', 'Erfasste Bodybytes'), textColumn('body_complete', 'Body vollständig', row => row.body_complete == null ? 'Unbekannt' : row.body_complete ? 'Ja' : 'Nein'), textColumn('document_metadata_available', 'HTML-Metadaten verfügbar', row => row.document_metadata_available ? 'Ja' : 'Nein')];
      case 'titles': return [urlColumn, textColumn('title_text', 'Seitentitel'), numberColumn('title_length', 'Zeichen'), numberColumn('title_pixels', 'Pixelprofil 20px', row => array(row.title_pixel_widths)[0]), numberColumn('title_count', 'Title-Tags', (row) => row.titles.length), numberColumn('duplicates', 'Vorkommen'), statusColumn];
      case 'meta': return [urlColumn, textColumn('description_text', 'Meta Description'), numberColumn('description_length', 'Zeichen'), numberColumn('description_pixels', 'Pixelprofil 14px', row => array(row.description_pixel_widths)[0]), numberColumn('meta_count', 'Meta-Tags', (row) => row.meta_descriptions.length), numberColumn('duplicates', 'Vorkommen'), statusColumn];
      case 'headings': return [urlColumn, textColumn('h1', 'H1', (row) => first(row.h1)), numberColumn('h1_count', 'H1-Anzahl', (row) => row.h1.length), textColumn('h2', 'H2', (row) => first(row.h2)), numberColumn('h2_count', 'H2-Anzahl', (row) => row.h2.length), numberColumn('word_count', 'Wörter')];
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
    const isSelected = row => state.section === 'responses' ? state.selectedResponse && state.selectedResponse.requested_url === row.requested_url && state.selectedResponse.response_index === row.response_index : state.selected && (row._page?.url || row.url) === state.selected.url;
    const emptyNote = state.section === 'responses' ? state.responsesError || (state.responses == null ? 'Diese Beobachtungen werden nach Abschluss oder Stopp des Crawls geladen. Nicht erfasste Antworten bleiben unbekannt.' : 'Es wurden keine nativen HTTP-Antworten erfasst, beispielsweise bei DNS-Fehlern oder Robots-Sperren.') : null;
    const table = $('results-table');
    table.querySelector('thead').innerHTML = `<tr>${currentColumns.map((column) => `<th scope="col" aria-sort="${state.sort === column.key ? state.direction > 0 ? 'ascending' : 'descending' : 'none'}"><button type="button" data-sort="${escape(column.key)}">${escape(column.label)}<span aria-hidden="true">${state.sort === column.key ? state.direction > 0 ? '↑' : '↓' : '↕'}</span></button></th>`).join('')}</tr>`;
    table.querySelector('tbody').innerHTML = visible.length ? visible.map((row, index) => `<tr class="click-row${isSelected(row) ? ' selected' : ''}" data-row="${index}" tabindex="0" aria-label="Details zu ${escape(row.url)} öffnen">${currentColumns.map((column) => `<td>${column.render(row)}</td>`).join('')}</tr>`).join('') : `<tr><td class="empty-cell" colspan="${currentColumns.length}"><strong>${state.search || state.filter !== 'all' ? 'Keine passenden Ergebnisse' : state.crawl ? 'Noch keine Daten in diesem Bereich' : 'Starte einen Crawl, um Ergebnisse zu sehen'}</strong>${escape(state.search || state.filter !== 'all' ? 'Ändere den Filter oder den Suchbegriff.' : emptyNote || (['linkmetrics', 'structured'].includes(state.section) ? state.analysisError || 'Diese Auswertung wird nach Abschluss oder Stopp des Crawls berechnet.' : state.section === 'images' ? 'Bildreferenzen erscheinen nach der Auswertung von HTML-Seiten.' : state.section === 'external' ? 'Externe Links erscheinen, sobald sie im HTML entdeckt werden.' : 'Nur tatsächlich erfasste Daten werden angezeigt.'))}</td></tr>`;
    table.querySelectorAll('[data-sort]').forEach((button) => button.addEventListener('click', () => {
      state.direction = state.sort === button.dataset.sort ? -state.direction : 1;
      state.sort = button.dataset.sort;
      state.page = 0;
      renderTable();
    }));
    table.querySelectorAll('[data-row]').forEach((rowElement) => {
      const open = () => {
        const row = visible[Number(rowElement.dataset.row)];
        if (state.section === 'responses') { selectResponse(row); return; }
        const page = row._page || state.pageMap.get(row.url) || row;
        selectPage(page);
      };
      rowElement.addEventListener('click', open);
      rowElement.addEventListener('keydown', (event) => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); open(); } });
    });
    $('result-count').textContent = `${number(rows.length)} Ergebnisse`;
    $('page-label').textContent = rows.length ? `${number(offset + 1)}–${number(Math.min(offset + state.pageSize, rows.length))} von ${number(rows.length)}` : 'Keine Ergebnisse';
    $('page-prev').disabled = state.page === 0;
    $('page-next').disabled = state.page + 1 >= pages;
  }

  function selectSection(section) {
    if (!sections[section]) return;
    state.section = section;
    state.page = 0;
    state.filter = 'all';
    state.search = '';
    state.sort = 'url';
    state.direction = 1;
    $('table-search').value = '';
    $('table-filter').innerHTML = filterOptions().map(([value, label]) => `<option value="${value}">${escape(label)}</option>`).join('');
    document.querySelectorAll('[data-section]').forEach((button) => {
      button.classList.toggle('active', button.dataset.section === section);
      if (button.dataset.section === section) button.setAttribute('aria-current', 'page'); else button.removeAttribute('aria-current');
    });
    renderSection();
  }

  function renderSection() {
    const [eyebrow, title, description] = sections[state.section];
    $('section-eyebrow').textContent = eyebrow;
    $('section-heading').textContent = title;
    $('section-description').textContent = description;
    $('overview-view').hidden = state.section !== 'overview';
    $('graph-view').hidden = state.section !== 'graph';
    $('table-view').hidden = ['overview', 'graph'].includes(state.section);
    $('export-csv').disabled = (!state.pages.length && !state.issues.length) || state.section === 'responses' && state.responses == null;
    $('export-resources').hidden = !['pdf', 'imagefiles', 'responses'].includes(state.section);
    $('export-resources').disabled = !state.crawl || Boolean(active());
    $('export-resources').textContent = state.section === 'pdf' ? '↓ PDF-Details CSV' : state.section === 'responses' ? '↓ Alle Antworten CSV' : '↓ Bilddateien CSV';
    if (state.section === 'overview') renderOverview();
    else if (state.section === 'graph') renderGraph();
    else renderTable();
  }

  function selectResponse(response) {
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
    state.selectedResponse = null;
    state.selected = normalPage(page);
    state.inspectorTab = 'details';
    renderInspector();
    if (!['overview', 'graph'].includes(state.section)) renderTable();
    $('inspector').scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }

  function renderInspector() {
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
    const original = state.section;
    if (['overview', 'graph'].includes(original)) state.section = 'internal';
    const cols = columns();
    const rows = filteredRows();
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
    const form = $('settings-form');
    for (const [key, value] of Object.entries(state.settings)) {
      const element = form.elements.namedItem(key);
      if (element.type === 'checkbox') element.checked = Boolean(value); else element.value = value;
    }
    toggleRendererSettings(state.settings.javascript_rendering);
    $('settings-error').hidden = true;
    $('settings-dialog').showModal();
  }

  function saveSettings(event) {
    event.preventDefault();
    const form = $('settings-form');
    const advancedText = { include_patterns_text: form.elements.include_patterns_text.value, exclude_patterns_text: form.elements.exclude_patterns_text.value, custom_extractions_json: form.elements.custom_extractions_json.value, custom_javascript_json: form.elements.custom_javascript_json.value, audit_thresholds_json: form.elements.audit_thresholds_json.value, pdf_analysis_json: form.elements.pdf_analysis_json.value };
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
    try { localStorage.setItem('open-seo-spider-settings', JSON.stringify(state.settings)); } catch (_) { /* Crawl settings still apply when storage is unavailable. */ }
    $('settings-dialog').close();
    renderControls();
  }

  function restoreSettings() {
    try {
      const saved = JSON.parse(localStorage.getItem('open-seo-spider-settings') || 'null');
      if (saved && typeof saved === 'object') {
        const positive = (value, fallback, max) => Number.isInteger(Number(value)) && Number(value) > 0 && Number(value) <= max ? Number(value) : fallback;
        const nonnegative = (value) => Number.isInteger(Number(value)) && Number(value) >= 0 ? Number(value) : 0;
        state.settings = { concurrency: positive(saved.concurrency, 8, 64), timeout_seconds: positive(saved.timeout_seconds, 20, 300), max_pages: nonnegative(saved.max_pages), max_depth: nonnegative(saved.max_depth), user_agent: String(saved.user_agent || 'Open SEO Spider/0.1').slice(0, 256), obey_robots: saved.obey_robots !== false, follow_subdomains: saved.follow_subdomains === true, delay_ms: nonnegative(saved.delay_ms ?? 100), max_body_mb: positive(saved.max_body_mb, 5, 100), check_external: saved.check_external === true, javascript_rendering: saved.javascript_rendering === true, renderer_endpoint: String(saved.renderer_endpoint || 'http://127.0.0.1:8790/render'), renderer_wait_ms: nonnegative(saved.renderer_wait_ms ?? 500), renderer_accessibility: saved.renderer_accessibility === true, renderer_screenshot: saved.renderer_screenshot === true };
        Object.assign(state.settings, { indexability_agent: String(saved.indexability_agent || 'googlebot'), renderer_collect_cookies: saved.renderer_collect_cookies === true, custom_javascript_json: String(saved.custom_javascript_json || '[]'), audit_thresholds_json: String(saved.audit_thresholds_json || '{}'), pdf_analysis_json: String(saved.pdf_analysis_json || '{}'), include_patterns_text: String(saved.include_patterns_text || ''), exclude_patterns_text: String(saved.exclude_patterns_text || ''), custom_extractions_json: String(saved.custom_extractions_json || '[]') });
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
    return { custom_extractions, audit_thresholds: parseAuditThresholds(settings.audit_thresholds_json), pdf_analysis: parsePdfAnalysis(settings.pdf_analysis_json), include_patterns: lines(settings.include_patterns_text), exclude_patterns: lines(settings.exclude_patterns_text) };
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

  function bindEvents() {
    $('crawl-form').addEventListener('submit', startCrawl);
    $('cancel-button').addEventListener('click', cancelCrawl);
    $('resume-button').addEventListener('click', resumeCrawl);
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
    $('inspector-close').addEventListener('click', () => { state.selected = null; state.selectedResponse = null; $('inspector').hidden = true; if (!['overview', 'graph'].includes(state.section)) renderTable(); });
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
    $('export-resources').addEventListener('click', () => serverDownload(state.section === 'pdf' ? 'pdf.csv' : state.section === 'responses' ? 'responses.csv' : 'images.csv', `open-seo-spider-${state.crawl.id}-${state.section}.csv`));
    $('import-project').addEventListener('click', () => $('import-file').click());
    $('import-file').addEventListener('change', (event) => importProject(event.target.files[0]));
    $('export-sitemap').addEventListener('click', () => serverDownload('sitemap.xml', `open-seo-spider-${state.crawl.id}-sitemap.xml`));
    $('export-project').addEventListener('click', () => serverDownload('project.json', `open-seo-spider-${state.crawl.id}.json`));
    document.addEventListener('visibilitychange', () => { if (!document.hidden && active() && !state.polling) refresh(); });
  }

  async function initialize() {
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
