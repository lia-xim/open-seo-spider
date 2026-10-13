'use strict';

// The dialog reads saved reports through the comparison API. It never starts a crawl.
(() => {
  const ENGINE = 'open-seo-spider/saved-comparison-v1';
  const KINDS = { added: 'Hinzugefügt', removed: 'Entfernt', changed: 'Geändert', unchanged: 'Unverändert' };
  const STATES = { observed: 'Beobachtet', observed_empty: 'Beobachtet leer', partial: 'Teilweise', missing: 'Fehlt', unknown: 'Unbekannt', unavailable: 'Nicht verfügbar', error: 'Fehler' };
  const OUTCOMES = { changed: 'Geändert', unchanged: 'Unverändert', unknown: 'Unbekannt' };
  const PAGE_SIZE = 50, MAX_RESPONSE = 64 * 1024 * 1024, MAX_CSV = 256 * 1024 * 1024;
  const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
  const safeInteger = value => Number.isSafeInteger(value) && value >= 0;
  const strings = value => Array.isArray(value) && value.every(item => typeof item === 'string');
  const escape = value => String(value ?? '').replace(/[&<>"']/g, char => ({ '&':'&amp;', '<':'&lt;', '>':'&gt;', '"':'&quot;', "'":'&#39;' }[char]));
  const json = value => JSON.stringify(value, null, 2);
  const numeric = value => value.toLocaleString('de-DE');
  const unsafeInteger = value => { const pending=[value]; while(pending.length) { const item=pending.pop(); if(typeof item==='number' && Number.isInteger(item) && !Number.isSafeInteger(item)) return true; if(item && typeof item==='object') for(const child of Object.values(item)) pending.push(child); } return false; };
  const safeJSON = value => !unsafeInteger(value) ? json(value) : 'Hinweis: Numerischer Wert außerhalb der präzisen Browser-Darstellung. Exakte Originalzahlen sind nur in der unveränderten API-JSON-Antwort erhalten.\n' + JSON.stringify(value, (_key, item) => typeof item === 'number' && Number.isInteger(item) && !Number.isSafeInteger(item) ? '[Zahl nicht präzise darstellbar; Original in API-JSON]' : item, 2);
  const factValue = fact => unsafeInteger(fact.value) ? 'Numerischer Wert außerhalb der präzisen Browser-Darstellung. Exakten Wert im gesamten JSON-Export prüfen.' : fact.value === null ? (fact.state === 'observed_empty' ? 'Beobachtet leer · null' : fact.state === 'observed' ? 'Beobachteter JSON-Wert · null' : 'Kein gespeicherter Wert') : json(fact.value);
  const validFact = fact => object(fact) && Object.hasOwn(STATES, fact.state) && Object.hasOwn(fact, 'value') && (fact.profile === null || typeof fact.profile === 'string') && strings(fact.reasons) && Object.hasOwn(fact, 'provenance');
  const validReference = reference => reference === null || object(reference) && safeInteger(reference.ordinal) && ['original_requested_url', 'document_url', 'fetched_at', 'basis'].every(key => typeof reference[key] === 'string');
  const validSnapshot = snapshot => object(snapshot) && /^[a-f0-9]{64}$/.test(snapshot.typed_report_sha256) && typeof snapshot.typed_report_sha_profile === 'string' && object(snapshot.coverage) && safeInteger(snapshot.coverage.saved_page_count) && safeInteger(snapshot.coverage.pending) && ['limited','cancelled','mixed_age_selected_recrawl','website_completeness_proven'].every(key => typeof snapshot.coverage[key] === 'boolean') && (snapshot.caller_binding === null || object(snapshot.caller_binding));

  function validateEnvelope(envelope, includeOriginalReports = false) {
    const result = envelope?.comparison;
    if (!object(envelope) || envelope.website_requests !== 0 || !object(result) || result.schema_version !== 1 || result.engine !== ENGINE || result.website_requests !== 0 || !Array.isArray(result.pages) || !object(result.counts) || !object(result.config) || !validSnapshot(result.before_snapshot) || !validSnapshot(result.after_snapshot) || !strings(result.limitations)) throw Error('Unbekanntes oder unvollständiges Vergleichsformat. Es wird kein Ergebnis als vollständig angezeigt.');
    const counts = { added:0, removed:0, changed:0, unchanged:0, pages_with_unknown_fields:0 }, keys = new Set();
    for (const page of result.pages) {
      if (!object(page) || !Object.hasOwn(KINDS, page.kind) || typeof page.comparison_key !== 'string' || keys.has(page.comparison_key) || !validReference(page.before) || !validReference(page.after) || !Array.isArray(page.fields) || !strings(page.unknown_fields) || typeof page.equality_proven_for_compared_fields !== 'boolean') throw Error('Ungültige Vergleichszeile oder doppelte Vergleichsidentität.');
      if (page.kind === 'added' ? page.before !== null || page.after === null : page.kind === 'removed' ? page.before === null || page.after !== null : page.before === null || page.after === null) throw Error('URL-Zuordnung und Änderungsklasse passen nicht zusammen.');
      keys.add(page.comparison_key); counts[page.kind] += 1;
      if (page.unknown_fields.length) counts.pages_with_unknown_fields += 1;
      const fieldNames = new Set();
      for (const field of page.fields) {
        if (!object(field) || typeof field.field !== 'string' || fieldNames.has(field.field) || !validFact(field.before) || !validFact(field.after) || !Object.hasOwn(OUTCOMES, field.outcome) || typeof field.availability_changed !== 'boolean' || typeof field.comparable_complete_values !== 'boolean' || !(field.reason === null || typeof field.reason === 'string')) throw Error('Ungültiger Feldwert oder fehlende Quellenangabe.');
        fieldNames.add(field.field);
      }
    }
    if (Object.keys(counts).some(key => !safeInteger(result.counts[key]) || counts[key] !== result.counts[key])) throw Error('Die Zähler stimmen nicht mit den gespeicherten Vergleichszeilen überein.');
    validateOriginalReports(envelope, includeOriginalReports);
    return envelope;
  }

  function validateOriginalReports(envelope, expected) {
    const present = Object.hasOwn(envelope, 'original_reports');
    if (!expected) { if (present) throw Error('Unerwartetes Berichtsbundle ohne gewählte Option.'); return; }
    const bundle = envelope.original_reports, result = envelope.comparison;
    const exactKeys = (value, keys) => object(value) && Object.keys(value).length === keys.length && keys.every(key => Object.hasOwn(value, key));
    if (!present || !exactKeys(bundle, ['schema_version','representation','binding','before','after']) || bundle.schema_version !== 1 || bundle.representation !== 'typed-crawl-report-serde-json-v1' || bundle.binding !== 'same-typed-reports-as-comparison') throw Error('Die gewählten gespeicherten Berichte fehlen oder haben ein unbekanntes Bundleformat.');
    for (const side of ['before','after']) {
      const original = bundle[side], snapshot = result[side + '_snapshot'], report = original?.report;
      if (!exactKeys(original, ['report','typed_report_sha256','typed_report_json_bytes']) || !safeInteger(original.typed_report_json_bytes) || original.typed_report_json_bytes > MAX_RESPONSE || original.typed_report_sha256 !== snapshot.typed_report_sha256 || original.typed_report_json_bytes !== snapshot.typed_report_json_bytes || snapshot.typed_report_sha_profile !== bundle.representation || !object(report) || !object(report.config) || !object(report.summary) || !Array.isArray(report.pages) || !report.pages.every(page => object(page) && typeof page.url === 'string') || report.pages.length !== snapshot.coverage.saved_page_count || !Array.isArray(report.issues) || !report.issues.every(object) || !strings(report.warnings)) throw Error('Gespeicherter Bericht und Vergleichsbezug passen nicht zusammen.');
      for (const page of result.pages) {
        const reference = page[side];
        if (reference && report.pages[reference.ordinal]?.url !== reference.original_requested_url) throw Error('Original-URL im Bericht und Vergleichsreferenz passen nicht zusammen.');
      }
    }
    // Checks declared profile/hash/count/reference consistency only. Browser
    // parsed numbers cannot reconstruct Rust's typed report hash. Exact report
    // bytes are retained exclusively in resultRaw for the JSON download.
  }

  // Same formula defence as the existing product CSV export. JSON keeps originals.
  const csvCell = input => { let value = String(input ?? ''); if (/^[\s\uFEFF]*[=+@-]/u.test(value)) value = "'" + value; return '"' + value.replace(/"/g, '""') + '"'; };
  function csvExport(envelope, pages) {
    const comparison = envelope.comparison;
    const columns = ['Art','Vergleichsidentität','Vorher Original-URL','Nachher Original-URL','Feld','Befund','Vorher Zustand','Vorher Wert JSON','Vorher Quellenprofil','Vorher Gründe JSON','Vorher Herkunft JSON','Nachher Zustand','Nachher Wert JSON','Nachher Quellenprofil','Nachher Gründe JSON','Nachher Herkunft JSON','Verfügbarkeit geändert','Vollständige Werte vergleichbar','Vergleichsgrund','Vorher Bericht SHA256','Nachher Bericht SHA256'];
    const lines = ['\uFEFF' + columns.map(csvCell).join(',')]; let bytes = new TextEncoder().encode(lines[0]).length + 2;
    for (const page of pages) for (const field of page.fields) {
      if (unsafeInteger(field.before.value) || unsafeInteger(field.after.value) || unsafeInteger(field.before.provenance) || unsafeInteger(field.after.provenance)) throw Error('Ein numerischer Wert überschreitet die präzise CSV-Darstellung im Browser. Exakte Originalwerte stehen im gesamten JSON-Export; es wurde keine CSV-Teildatei exportiert.');
      const line = [page.kind,page.comparison_key,page.before?.original_requested_url ?? '',page.after?.original_requested_url ?? '',field.field,field.outcome,field.before.state,JSON.stringify(field.before.value),field.before.profile ?? '',JSON.stringify(field.before.reasons),JSON.stringify(field.before.provenance),field.after.state,JSON.stringify(field.after.value),field.after.profile ?? '',JSON.stringify(field.after.reasons),JSON.stringify(field.after.provenance),field.availability_changed,field.comparable_complete_values,field.reason ?? '',comparison.before_snapshot.typed_report_sha256,comparison.after_snapshot.typed_report_sha256].map(csvCell).join(',');
      bytes += new TextEncoder().encode(line).length + 2;
      if (bytes > MAX_CSV) throw Error('Der CSV-Export überschreitet 256 MiB. Es wurde keine Teildatei exportiert.');
      lines.push(line);
    }
    return lines.join('\r\n') + '\r\n';
  }

  async function boundedJSON(response) {
    if (!response.body || !response.headers.get('content-type')?.includes('json')) throw Error('Die Vergleichs-API liefert kein JSON.');
    const declared = Number(response.headers.get('content-length'));
    if (declared > MAX_RESPONSE) throw Error('Die API-Antwort überschreitet 64 MiB.');
    const reader = response.body.getReader(), decoder = new TextDecoder('utf-8', { fatal:true }); let bytes = 0, text = '';
    try {
      while (true) { const chunk = await reader.read(); if (chunk.done) break; bytes += chunk.value.byteLength; if (bytes > MAX_RESPONSE) throw Error('Die API-Antwort überschreitet 64 MiB.'); text += decoder.decode(chunk.value, { stream:true }); }
      text += decoder.decode(); return {body:JSON.parse(text),text};
    } finally { await reader.cancel().catch(() => {}); reader.releaseLock(); }
  }

  function bind({ downloadBlob, selectedJob = () => null }) {
    const $ = id => document.getElementById(id), dialog = $('saved-comparison-dialog');
    if (!dialog || dialog.dataset.bound) return;
    dialog.dataset.bound = 'true';
    const state = { jobs:[], result:null, resultRaw:null, error:null, page:0, selected:null, fieldPage:0, revision:0, token:null, historyToken:null, exportBusy:false };
    const filtered = () => { const kind = $('saved-comparison-filter').value, query = $('saved-comparison-search').value.trim().toLocaleLowerCase('de-DE'); return (state.result?.comparison.pages || []).filter(page => (kind === 'all' || kind === page.kind) && (!query || [page.comparison_key,page.before?.original_requested_url,page.after?.original_requested_url].some(value => String(value ?? '').toLocaleLowerCase('de-DE').includes(query)))); };
    const busy = () => Boolean(state.token || state.historyToken || state.exportBusy);
    const eligible = job => job && !['running','pausing','cancelling','queued','pending'].includes(job.status) && job.recrawl_available !== false;
    const announce = text => { $('saved-comparison-status').textContent = text; };
    function controls() {
      const usable = Boolean(state.result) && !busy();
      $('saved-comparison-run').disabled = busy() || !eligible(state.jobs.find(job => job.id === $('saved-comparison-before').value)) || !eligible(state.jobs.find(job => job.id === $('saved-comparison-after').value));
      $('saved-comparison-refresh').disabled = busy();
      $('saved-comparison-export-json').disabled = !usable;
      $('saved-comparison-export-json').textContent = $('saved-comparison-original-reports').checked ? 'Vergleich mit gespeicherten Berichten' : 'Vergleichs-JSON';
      $('saved-comparison-export-scope').textContent = $('saved-comparison-original-reports').checked ? 'JSON enthält den Vergleich und beide geladenen gespeicherten Berichte mit ihren vorhandenen Daten. Unbekannte und teilweise Befunde bleiben erhalten. Die Berichtsinhalte werden mit der unveränderten Antwort exportiert. CSV enthält die verglichenen Felder der gewählten URLs. Exporte verwenden die bereits geladenen Daten.' : 'Exporte verwenden das bereits geladene Vergleichsergebnis. Das Vergleichs-JSON enthält die verglichenen Befunde und Berichtsbezüge. Mit der Option oben kann JSON auch beide geladenen gespeicherten Berichte enthalten. CSV enthält die verglichenen Felder der gewählten URLs, unabhängig von der sichtbaren Tabellen- oder Feldseite. JSON bewahrt die erhaltene Antwort; CSV schützt Textzellen vor Formelausführung.';
      $('saved-comparison-export-csv').disabled = !usable;
      $('saved-comparison-export-filtered').disabled = !usable || !filtered().length;
      $('saved-comparison-form').setAttribute('aria-busy', String(Boolean(state.token)));
    }
    function abortComparison() { state.revision += 1; state.token?.controller.abort(); state.token = null; controls(); }
    function invalidate() { abortComparison(); state.result = null; state.resultRaw = null; state.error = null; state.selected = null; state.page = 0; $('saved-comparison-results').hidden = true; $('saved-comparison-detail').hidden = true; showError(null); announce('Auswahl oder Optionen geändert. Gespeicherten Vergleich erneut berechnen.'); controls(); }
    function showError(error) {
      state.error = error; $('saved-comparison-error').hidden = !error;
      if (!error) return;
      $('saved-comparison-error-message').textContent = typeof error.error?.message === 'string' ? `${error.error.code}: ${error.error.message}` : error.message || 'Vergleich fehlgeschlagen.';
      $('saved-comparison-error-data').textContent = safeJSON(error);
    }
    function fillJobs() {
      const preferred = selectedJob();
      for (const side of ['before','after']) {
        const element = $('saved-comparison-' + side), previous = element.value;
        element.replaceChildren(new Option('Gespeicherten Crawl auswählen', ''));
        for (const job of state.jobs) {
          const seed = job.seed_url || job.config?.seed_urls?.[0] || job.id;
          const date = job.started_at ? new Date(job.started_at).toLocaleString('de-DE') : 'Zeit nicht erfasst';
          const status = ({completed:'Abgeschlossen',paused:'Pausiert',limited:'Grenze erreicht',cancelled:'Gestoppt',interrupted:'Unterbrochen',failed:'Fehlgeschlagen',running:'Läuft',pausing:'Wird pausiert',cancelling:'Wird gestoppt'}[job.status] || job.status || 'Status unbekannt');
          const option = new Option(`${seed} · ${date} · ${status}${eligible(job) ? '' : ' · derzeit nicht vergleichbar'}`, job.id); option.disabled = !eligible(job); element.append(option);
        }
        element.value = state.jobs.some(job => job.id === previous) ? previous : '';
        if (!previous && side === 'after' && eligible(state.jobs.find(job => job.id === preferred))) element.value = preferred;
        if (!element.value && !previous) element.value = state.jobs.find(job => eligible(job) && (side === 'after' || job.id !== $('saved-comparison-after').value))?.id || '';
      }
      // Prefer two distinct jobs initially, while explicitly allowing self-comparison.
      if (!$('saved-comparison-before').value || $('saved-comparison-before').value === $('saved-comparison-after').value) {
        if (!dialog.dataset.hasSelection) $('saved-comparison-before').value = state.jobs.find(job => eligible(job) && job.id !== $('saved-comparison-after').value)?.id || $('saved-comparison-before').value;
      }
      dialog.dataset.hasSelection = 'true'; controls();
    }
    async function loadJobs() {
      const token = { controller:new AbortController() }; state.historyToken?.controller.abort(); state.historyToken = token; controls(); announce('Gespeicherte Crawls werden geladen …');
      try {
        const response = await fetch('/api/crawls', { signal:token.controller.signal }); const {body} = await boundedJSON(response);
        if (state.historyToken !== token) return;
        if (!response.ok) throw Error(`Crawl-Historie nicht verfügbar: HTTP ${response.status}`);
        const jobs = Array.isArray(body) ? body : body.crawls || body.items;
        if (!Array.isArray(jobs) || jobs.some(job => !object(job) || typeof job.id !== 'string')) throw Error('Unbekanntes Format der Crawl-Historie.');
        state.jobs = jobs; fillJobs(); announce(state.result ? 'Gespeichertes Vergleichsergebnis bleibt erhalten. Neue Berechnung übernimmt den aktuellen Jobstand.' : 'Zwei gespeicherte Crawls auswählen. Gleicher Crawl auf beiden Seiten ist erlaubt.');
      } catch (error) { if (state.historyToken === token && error.name !== 'AbortError') { showError({message:error.message}); announce('Crawl-Auswahl konnte nicht aktualisiert werden.'); } }
      finally { if (state.historyToken === token) { state.historyToken = null; controls(); } }
    }
    const snapshotDescription = (snapshot, label) => {
      const coverage = snapshot.coverage;
      return `${label}: ${numeric(coverage.saved_page_count)} gespeicherte Seiten · ${numeric(coverage.pending)} ausstehend${coverage.limited ? ' · Grenze erreicht' : ''}${coverage.cancelled ? ' · gestoppt' : ''}${coverage.mixed_age_selected_recrawl ? ' · Befunde unterschiedlichen Alters' : ''}. ${coverage.website_completeness_proven ? 'Vollständigkeit laut Ergebnisprofil belegt.' : 'Vollständigkeit der Website nicht belegt.'}`;
    };
    function render() {
      const result = state.result?.comparison; if (!result) return;
      $('saved-comparison-results').hidden = false;
      $('saved-comparison-counts').innerHTML = Object.entries(KINDS).map(([kind,label]) => `<span>${escape(label)} <strong>${numeric(result.counts[kind])}</strong></span>`).join('') + `<span>Mit unbekannten Feldern <strong>${numeric(result.counts.pages_with_unknown_fields)}</strong></span>`;
      $('saved-comparison-coverage').textContent = snapshotDescription(result.before_snapshot, 'Vorher') + '\n' + snapshotDescription(result.after_snapshot, 'Nachher');
      const bundle = state.result.original_reports;
      $('saved-comparison-bundle-status').hidden = !bundle;
      $('saved-comparison-bundle-status').textContent = bundle ? `JSON enthält beide geladenen gespeicherten Berichte · Vorher ${numeric(bundle.before.typed_report_json_bytes)} Bytes · Nachher ${numeric(bundle.after.typed_report_json_bytes)} Bytes. Unbekannte und teilweise Befunde bleiben gekennzeichnet.` : '';
      $('saved-comparison-profile').textContent = safeJSON({engine:result.engine,identity_profile:result.identity_profile,mapping_profile:result.mapping_profile,config:result.config,config_sha256:result.config_sha256,before_snapshot:result.before_snapshot,after_snapshot:result.after_snapshot,ignored_runtime_fields:result.ignored_runtime_fields,limitations:result.limitations,website_requests:result.website_requests,vendor_parity:result.vendor_parity});
      const rows = filtered(); state.page = Math.max(0,Math.min(state.page,Math.max(0,Math.ceil(rows.length / PAGE_SIZE)-1))); const offset = state.page * PAGE_SIZE;
      $('saved-comparison-table').querySelector('tbody').innerHTML = rows.slice(offset,offset+PAGE_SIZE).map(page => `<tr><td><span class="badge ${page.kind === 'changed' ? 'warning' : page.kind === 'removed' ? 'danger' : 'neutral'}">${escape(KINDS[page.kind])}</span></td><td><button class="saved-comparison-row" type="button" data-comparison-key="${escape(page.comparison_key)}">${escape(page.comparison_key)}</button><small>Vorher: ${escape(page.before?.original_requested_url ?? 'URL fehlt in diesem Bericht')}<br>Nachher: ${escape(page.after?.original_requested_url ?? 'URL fehlt in diesem Bericht')}</small></td><td>${numeric(page.fields.filter(field => field.outcome === 'changed').length)} geändert · ${numeric(page.unknown_fields.length)} unbekannt<small>${page.equality_proven_for_compared_fields ? 'Gleichheit für die verglichenen Felder belegt' : 'Gleichheit für die verglichenen Felder nicht belegt'}</small></td></tr>`).join('') || '<tr><td colspan="3" class="empty-cell">Keine Vergleichszeilen für diesen Filter.</td></tr>';
      $('saved-comparison-page-label').textContent = rows.length ? `${numeric(offset+1)}–${numeric(Math.min(rows.length,offset+PAGE_SIZE))} von ${numeric(rows.length)} gefilterten URLs; ${numeric(result.pages.length)} URLs im gesamten Ergebnis.` : `0 gefilterte URLs; ${numeric(result.pages.length)} URLs im gesamten Ergebnis.`;
      $('saved-comparison-prev').disabled = state.page === 0; $('saved-comparison-next').disabled = offset+PAGE_SIZE >= rows.length; controls();
    }
    function factHTML(fact, label) {
      return `<section class="saved-comparison-fact"><h5>${escape(label)} · <span class="badge ${['unknown','partial','missing','error','unavailable'].includes(fact.state) ? 'warning' : 'neutral'}">${escape(STATES[fact.state])}</span></h5><pre tabindex="0">${escape(factValue(fact))}</pre><p>Quellenprofil: <code>${escape(fact.profile ?? 'Nicht erfasst')}</code></p>${fact.reasons.length ? `<p>Gründe: ${escape(fact.reasons.join(' · '))}</p>` : ''}<details><summary>Gespeicherte Herkunft</summary><pre tabindex="0">${escape(fact.provenance === null ? 'Herkunft nicht erfasst.' : safeJSON(fact.provenance))}</pre></details></section>`;
    }
    function detail() {
      const page = state.result?.comparison.pages.find(page => page.comparison_key === state.selected); $('saved-comparison-detail').hidden = !page; if (!page) return;
      $('saved-comparison-detail-heading').textContent = `${KINDS[page.kind]} · ${page.comparison_key}`;
      $('saved-comparison-page-data').textContent = safeJSON({before:page.before,after:page.after,unknown_fields:page.unknown_fields,equality_proven_for_compared_fields:page.equality_proven_for_compared_fields});
      const kind = $('saved-comparison-field-filter').value, fields = page.fields.filter(field => kind === 'all' || kind === field.outcome); state.fieldPage = Math.max(0,Math.min(state.fieldPage,Math.max(0,Math.ceil(fields.length/PAGE_SIZE)-1))); const offset = state.fieldPage*PAGE_SIZE;
      $('saved-comparison-fields').innerHTML = fields.slice(offset,offset+PAGE_SIZE).map(field => `<article class="saved-comparison-field"><h4>${escape(field.field)} <span class="badge ${field.outcome === 'unknown' ? 'warning' : 'neutral'}">${escape(OUTCOMES[field.outcome])}</span></h4><p>${field.comparable_complete_values ? 'Vollständige Werte unter passenden Quellenprofilen vergleichbar.' : 'Vollständiger Wertevergleich nicht belegt.'}${field.availability_changed ? ' Verfügbarkeit geändert.' : ''}${field.reason ? ' ' + escape(field.reason) : ''}</p><div class="saved-comparison-facts">${factHTML(field.before,'Vorher')}${factHTML(field.after,'Nachher')}</div></article>`).join('') || '<p>Keine Felder für diesen Filter.</p>';
      $('saved-comparison-field-label').textContent = `${numeric(Math.min(fields.length,offset+1))}–${numeric(Math.min(fields.length,offset+PAGE_SIZE))} von ${numeric(fields.length)} gefilterten Feldern.`;
      $('saved-comparison-field-prev').disabled = state.fieldPage === 0; $('saved-comparison-field-next').disabled = offset+PAGE_SIZE >= fields.length;
    }
    async function compare(event) {
      event.preventDefault(); if (busy()) return;
      const before = state.jobs.find(job => job.id === $('saved-comparison-before').value), after = state.jobs.find(job => job.id === $('saved-comparison-after').value);
      if (!eligible(before) || !eligible(after)) { showError({message:'Zwei derzeit vergleichbare gespeicherte Crawls auswählen.'}); return; }
      let config;
      try { config = JSON.parse($('saved-comparison-config').value); if (!object(config)) throw Error('Vergleichsoptionen müssen ein JSON-Objekt sein.'); }
      catch (error) { showError({message:error.message}); return; }
      const body = {before:before.id,after:after.id,config}; const omitted = [];
      const includeOriginalReports = $('saved-comparison-original-reports').checked;
      if (includeOriginalReports) body.include_original_reports = true;
      for (const [side,job] of [['before',before],['after',after]]) { if (safeInteger(job.generation)) body['expected_' + side + '_generation'] = job.generation; else omitted.push(side === 'before' ? 'Vorher' : 'Nachher'); }
      const payload = JSON.stringify(body);
      if (new TextEncoder().encode(payload).length > 1024*1024) { showError({message:'Vergleichsoptionen überschreiten das Request-Budget von 1 MiB.'}); return; }
      abortComparison(); const token = {controller:new AbortController(),revision:state.revision}; state.token = token; state.result = null; state.resultRaw = null; state.selected = null; $('saved-comparison-results').hidden = true; $('saved-comparison-detail').hidden = true; showError(null); controls(); announce('Gespeicherte Berichte werden verglichen …');
      try {
        const response = await fetch('/api/compare/saved', {method:'POST',headers:{'Content-Type':'application/json'},body:payload,signal:token.controller.signal}); const {body:envelope,text:raw} = await boundedJSON(response);
        if (state.token !== token || state.revision !== token.revision || !dialog.open) return;
        if (!response.ok) { if (!object(envelope.error) || typeof envelope.error.code !== 'string' || typeof envelope.error.message !== 'string' || !Array.isArray(envelope.error.collisions)) throw Error(`Ungültiger strukturierter API-Fehler: HTTP ${response.status}`); showError(envelope); announce(response.status === 409 ? 'Gespeicherter Jobstand geändert oder nicht stabil. Historie aktualisieren und erneut vergleichen.' : `Vergleich abgewiesen: HTTP ${response.status}`); return; }
        state.result = validateEnvelope(envelope, includeOriginalReports); state.resultRaw = raw; state.page = 0; render(); announce(`Vergleich gespeichert geladener Berichte abgeschlossen · ${numeric(envelope.comparison.pages.length)} URLs · keine Website-Anfragen.${omitted.length ? ` Keine präzise erwartete Jobgeneration für ${omitted.join(', ')} verfügbar; ohne diese Erwartung verglichen.` : ''}`);
      } catch (error) { if (state.token === token && error.name !== 'AbortError') { showError({message:error.message}); announce('Vergleich nicht verfügbar. Kein Teilergebnis wird exportiert.'); } }
      finally { if (state.token === token) { state.token = null; controls(); } }
    }
    async function download(kind, filteredOnly = false) {
      if (!state.result || busy()) return; state.exportBusy = true; controls();
      try {
        const envelope = state.result, rows = filteredOnly ? filtered() : envelope.comparison.pages;
        const text = kind === 'json' ? state.resultRaw : csvExport(envelope,rows);
        const bundled = kind === 'json' && Boolean(envelope.original_reports);
        await downloadBlob(new Blob([text], {type:kind === 'json' ? 'application/json;charset=utf-8' : 'text/csv;charset=utf-8'}), `open-seo-spider-gespeicherter-vergleich-${envelope.comparison.before_snapshot.typed_report_sha256.slice(0,12)}-${envelope.comparison.after_snapshot.typed_report_sha256.slice(0,12)}${bundled ? '-mit-gespeicherten-berichten' : ''}${filteredOnly ? '-gefiltert' : ''}.${kind}`);
        announce(kind === 'json' ? (bundled ? 'Vergleich mit beiden geladenen gespeicherten Berichten als unverändertes JSON exportiert.' : 'Vergleichs-JSON mit den verglichenen Befunden exportiert.') : `${numeric(rows.length)} ${filteredOnly ? 'gefilterte' : 'gespeicherte Vergleichs-'}URLs als CSV exportiert, einschließlich aller zugehörigen Felder und Herkunft. Keine weiteren Berichte abgerufen.`);
      } catch (error) { showError({message:error.message}); }
      finally { state.exportBusy = false; controls(); }
    }
    $('saved-comparison-open').addEventListener('click', () => { dialog.showModal(); loadJobs(); });
    $('saved-comparison-close').addEventListener('click', () => dialog.close());
    dialog.addEventListener('close', () => { abortComparison(); state.historyToken?.controller.abort(); state.historyToken = null; controls(); if (!state.result) announce('Dialog geschlossen. Auswahl und Optionen bleiben erhalten.'); });
    dialog.addEventListener('cancel', abortComparison);
    $('saved-comparison-refresh').addEventListener('click', loadJobs);
    $('saved-comparison-form').addEventListener('submit', compare);
    for (const id of ['saved-comparison-before','saved-comparison-after']) $(id).addEventListener('change', invalidate);
    $('saved-comparison-config').addEventListener('input', invalidate);
    $('saved-comparison-original-reports').addEventListener('change', invalidate);
    for (const id of ['saved-comparison-filter','saved-comparison-search']) $(id).addEventListener(id.endsWith('search') ? 'input' : 'change', () => {state.page=0;render();});
    $('saved-comparison-prev').addEventListener('click', () => {state.page-=1;render();}); $('saved-comparison-next').addEventListener('click', () => {state.page+=1;render();});
    $('saved-comparison-table').addEventListener('click', event => { const button = event.target.closest('[data-comparison-key]'); if (!button) return; state.selected = button.dataset.comparisonKey; state.fieldPage=0; detail(); $('saved-comparison-detail-heading').focus(); });
    $('saved-comparison-detail-close').addEventListener('click', () => {state.selected=null;detail();});
    $('saved-comparison-field-filter').addEventListener('change', () => {state.fieldPage=0;detail();});
    $('saved-comparison-field-prev').addEventListener('click', () => {state.fieldPage-=1;detail();}); $('saved-comparison-field-next').addEventListener('click', () => {state.fieldPage+=1;detail();});
    $('saved-comparison-export-json').addEventListener('click', () => download('json')); $('saved-comparison-export-csv').addEventListener('click', () => download('csv')); $('saved-comparison-export-filtered').addEventListener('click', () => download('csv',true));
    controls();
  }
  const api = { bind, validateEnvelope, csvExport, csvCell, safeJSON };
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (typeof window === 'object') window.SavedComparisonUI = api;
})();
