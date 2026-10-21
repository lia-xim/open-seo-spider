'use strict';

(() => {
  const $ = (id) => document.getElementById(id);
  const state = {
    resources: { result:null, token:null, revision:0, cursor:0, back:[], error:'', selected:null, sort:0, direction:1 },
    content: { inventory:null, inventoryToken:null, cursor:0, back:[], error:'', selected:new Set(), source:null, sourceToken:null, sourceError:'', result:null, analysisToken:null, analysisError:'', revision:0, resultPage:0, sort:0, direction:1, resultSort:1, resultDirection:-1 },
    recrawlURLs: new Set(), policy: null, policyError: '', policyToken: null, policyCursor: 0, policyBack: [], policySelected: null, policySort: 0, policyDirection: 1,
    segmentView:null,segmentBase:null,
    crawl: null, crawls: [], pages: [], issues: [], section: 'overview',
    search: '', filter: 'all', sort: 'url', direction: 1, page: 0, pageSize: 50,
    selected: null, selectedResponse: null, selectedInput: null, responses: null, responsesError: '', fragments: null, fragmentsError: '', inspectorTab: 'details', polling: false, starting: false,
    generation: 0, pollToken: null, pollTimer: null, control: null, operationSequence: 0, startOperation: null, historySequence: 0, issueMap: new Map(), pageMap: new Map(), cache: {}, analysis: null, analysisError: '', settings: {
      crawl_quotas_enabled:false,crawl_quota_per_depth:"",crawl_quota_per_subdomain:"",crawl_quota_patterns_json:"[]",
      crawl_admission_limits_enabled:false,crawl_max_folder_depth:"",crawl_max_query_tokens:"",crawl_max_fetch_url_utf8_bytes:"",
      concurrency: 8, timeout_seconds: 20, max_pages: 0, max_depth: 0,
      user_agent: 'Open SEO Spider/0.1', indexability_agent: 'googlebot', obey_robots: true, follow_subdomains: false,
      delay_ms: 100, max_body_mb: 5, check_external: false,
      sitemap_source_archive_enabled:false, sitemap_source_archive_post_http:true, sitemap_source_archive_decoded_xml:true, sitemap_source_archive_max_body_bytes:'67108864', sitemap_source_archive_max_total_bytes:'536870912', sitemap_source_archive_max_records:'4096',
      sitemap_source_processing_enabled: false, sitemap_source_max_post_http_entity_bytes: '8388608',
      sitemap_source_max_decoded_xml_bytes: '8388608', sitemap_source_max_xml_events: '500000',
      sitemap_membership_limits_enabled:false, sitemap_membership_max_catalog_json_bytes:'33554432', sitemap_membership_max_row_json_bytes:'262144', sitemap_membership_max_rows:'1000000',
      javascript_rendering: false, renderer_endpoint: 'http://127.0.0.1:8790/render',
      renderer_wait_ms: 500, renderer_accessibility: false, renderer_screenshot: false,
      renderer_viewport_enabled: false, renderer_viewport_width: 1280, renderer_viewport_height: 800,
      renderer_collect_cookies: false, custom_javascript_json: '[]',
      custom_search_enabled:false, custom_search_json:'{"version":1,"rules":[{"name":"Beispiel","method":"literal","pattern":"Suchbegriff","sources":["raw_html"]}]}',
      include_patterns_text: '', exclude_patterns_text: '', custom_extractions_json: '[]', audit_thresholds_json: '{}', pdf_analysis_json: '{}', validate_fragments: false, link_position_rules_json: '[]', crawl_policy_enabled: false, crawl_policy_json: '{"scope":{"roots":[{"origin":"https://example.test","folders":["/"],"subdomains":false,"exact_paths":[]}]}}', live_policy_limits_json: '{}', content_capture_enabled:false, content_capture_json:'{"sources":["raw_http_html"],"include_selectors":["body"],"exclude_selectors":["script","style","template","noscript","nav","footer"]}',resource_discovery_enabled:false,resource_discovery_json:'{}',resource_discovery_limits_json:'{}',tls_enabled:false,tls_respect_hsts:false,tls_certificates_json:'[]',cdn_classification_enabled:false,cdn_origins_text:'',cdn_directory_reporting_enabled:false,cdn_directory_rules_text:'',respect_noindex:false,respect_canonical:false,respect_next_prev:false
    }
  };

  let configProfiles = null, profileFormBaseline = null, profileToolbarBaseline = null, profileHeaderSignature = '', profileQuotaBaseline = null;
  let configProfileCatalog = null, profileCatalogNameOwner = '', profileCatalogDelete = null;
  let screenshotController = null, screenshotView = null;
  let tableExplorer = null;
  let projectColumns = null, projectColumnsResetOwner = null;
  let tableRenderSequence = 0;
  let tableExport = null;
  let tableViews = null, tableExpressionViews = null, tableQueryRevision = 0, tableSearchTimer = null;
  function tableQueryEdited() {
    tableQueryRevision++;
    invalidateProjectColumns();
    clearTimeout(tableSearchTimer);tableSearchTimer=null;
    tableViews?.invalidate();
    tableExpressionViews?.invalidate();
  }
  const legacySettingsDefaults = {...state.settings};
  let dashboardSegments = null;
  const separateSegmentSections = new Set(['inputs','policy','policyaliases','content','resource-discovery','sitemap-membership','sitemap-findings']);
  const relationshipPages = () => state.segmentView ? state.segmentView.target_lookup : state.pages;
  function clearDashboardSegmentView() {
    if (!state.segmentBase) return;
    Object.assign(state,state.segmentBase);state.segmentBase=null;state.segmentView=null;state.cache={};
    state.selected=null;state.selectedResponse=null;state.selectedInput=null;state.recrawlURLs.clear();state.page=0;
    reindexIssues();$('inspector').hidden=true;render();
  }
  function installDashboardSegmentView(result) {
    state.segmentBase={pages:state.pages,issues:state.issues,analysis:state.analysis,analysisError:state.analysisError,pageMap:state.pageMap,responses:state.responses,responsesError:state.responsesError,fragments:state.fragments,fragmentsError:state.fragmentsError};
    state.segmentView=result;
    state.pages=result.pages.map(row=>({...normalPage(row.page),_segmentOrdinal:row.page_ordinal,_segmentMembership:row.membership}));
    state.pageMap=new Map(state.pages.map(page=>[page.url,page]));state.cache={};
    state.issues=result.issues;state.analysis=result.analysis;state.analysisError='';
    state.responses=result.responses;state.responsesError='';state.fragments=result.fragments;state.fragmentsError='';
    state.selected=null;state.selectedResponse=null;state.selectedInput=null;state.recrawlURLs.clear();state.page=0;
    reindexIssues();$('inspector').hidden=true;render();
  }
  function renderDashboardSegments() {
    const panel=$('dashboard-segments');if(!panel)return;
    const controller=dashboardSegments,view=state.segmentView,s=controller?.state;
    panel.setAttribute('aria-busy',String(Boolean(s?.busy)));
    $('dashboard-segment-apply').disabled=!controller || Boolean(s?.busy || s?.draftValid===false || state.polling || active() || actionBusy() || !state.crawl);
    $('dashboard-segment-clear').disabled=!controller || !Boolean(s?.busy || s?.applied || s?.error);
    for(const id of ['dashboard-segment-json','dashboard-segment-csv','dashboard-segment-export-kind'])$(id).disabled=!view || Boolean(s?.busy || active() || actionBusy());
    $('dashboard-segment-state').textContent=!controller?'Segmentfunktionen werden mit dem nativen Dashboard geladen.':s.busy?'Vollständigen gespeicherten Segmentstand prüfen …':s.error?s.error:view?`${number(view.cohort.summary.selected_pages)} von ${number(view.cohort.summary.total_pages)} Seitenbeobachtungen · ${number(view.cohort.summary.selected_issues)} gespeicherte Issues · Stand ${view.context.snapshot_token.slice(0,12)} · keine Websiteabrufe`:'Keine Segmentauswahl angewandt. Definitionen in diesem Browser laden, auswählen und ausdrücklich anwenden.';
    $('dashboard-segment-state').classList.toggle('error',Boolean(s?.error));
    const separate=separateSegmentSections.has(state.section);
    $('dashboard-segment-scope').hidden=!view;
    $('dashboard-segment-scope').textContent=!view?'':state.section==='issues'?'Probleme der bestätigten Segmentauswahl. Der Tabellen-CSV enthält die angezeigten Spalten einschließlich der Anzahl weiterer URLs. Für sämtliche zugehörigen URLs im Originalumfang oben die native CSV-Art „Probleme – alle zugehörigen URLs“ oder Segment-JSON wählen.':separate?'Eigenständiges Originalinventar: Das Dashboardsegment wird auf diesen Bereich und dessen separate Exporte nicht angewandt. Die Segmentkennzahlen oben bleiben auf die ausgewählten Seiten begrenzt.':['graph','linkmetrics'].includes(state.section)?'Ausgewählte Seiten, Linkmetriken des vollständigen Original-Captures. Zielbeziehungen außerhalb des Segments bleiben erhalten; keine neue Berechnung eines isolierten Teilgraphen.':'Seiten und Quellvorkommen aus der bestätigten Segmentauswahl. Beziehungspartner außerhalb der Auswahl bleiben als gespeicherter Kontext erhalten.';
    $('export-visible-report').textContent=view?'Segment-JSON':'JSON';
    $('export-sitemap').title=view?'Sitemap verwendet den Originalbericht und ist bei aktiver Segmentansicht gesperrt.':'';
  }
  function loadDashboardSegmentDefinitions() {
    if(!dashboardSegments)return;
    dashboardSegments.invalidate();
    try{
      const raw=localStorage.getItem(window.DashboardSegments.KEY);if(raw && new TextEncoder().encode(raw).length>65536)throw Error('Gespeicherte Segmentdefinitionen sind zu groß.');
      const rules=window.DashboardSegments.validateRules(raw?JSON.parse(raw):{version:1,definitions:[]});
      const select=$('dashboard-segment-selection'),chosen=select.value;
      select.replaceChildren();
      for(const[value,label]of [['all','Alle Seiten dieser Berichtsansicht'],...rules.definitions.map(definition=>['segment:'+definition.id,definition.name]),['unassigned','Unzugeordnet'],['unknown','Unbekannt']]){const option=document.createElement('option');option.value=value;option.textContent=label;select.append(option);}
      select.value=[...select.options].some(option=>option.value===chosen)?chosen:'all';
      dashboardSegments.configure(rules,select.value,$('dashboard-segment-mode').value);
    }catch(error){dashboardSegments.invalidate(error.message);}
  }
  function downloadDashboardSegmentCSV() {
    return dashboardSegments?.download($('dashboard-segment-export-kind').value);
  }
  function bindDashboardSegments() {
    if(!window.DashboardSegments)return;
    dashboardSegments=window.DashboardSegments.createController({
      owner:()=>({id:state.crawl?.id??null,nativeGeneration:state.crawl?.generation??null,uiGeneration:state.generation,stable:Boolean(state.crawl&&!active()&&!actionBusy()&&!state.pollToken)}),
      fetch:(...args)=>fetch(...args),crypto:window.crypto,onClear:clearDashboardSegmentView,onInstall:installDashboardSegmentView,
      onChange:()=>{renderDashboardSegments();renderControls();},
      onDownload:(bytes,kind)=>{downloadBlob(new Blob([bytes],{type:kind==='json'?'application/json':'text/csv;charset=utf-8'}),`open-seo-spider-${state.crawl.id}-segment-${kind}.${kind==='json'?'json':'csv'}`);message('Vollständiger Segmentexport · Länge, SHA-256 und Snapshot bestätigt.');}
    });
    const configure=()=>{try{dashboardSegments.configure(dashboardSegments.state.rules,$('dashboard-segment-selection').value,$('dashboard-segment-mode').value);}catch(_){}};
    $('dashboard-segment-load').addEventListener('click',loadDashboardSegmentDefinitions);
    $('dashboard-segment-selection').addEventListener('change',configure);$('dashboard-segment-mode').addEventListener('change',configure);
    $('dashboard-segment-apply').addEventListener('click',()=>dashboardSegments.apply());
    $('dashboard-segment-clear').addEventListener('click',()=>dashboardSegments.invalidate());
    $('dashboard-segment-json').addEventListener('click',()=>dashboardSegments.download('json'));
    $('dashboard-segment-csv').addEventListener('click',downloadDashboardSegmentCSV);
    window.addEventListener('storage',event=>{if(event.key===window.DashboardSegments.KEY || event.key===null)dashboardSegments.invalidate('Segmentdefinitionen wurden in einem anderen Fenster geändert. Definitionen erneut laden und anwenden.');});
  }

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
    'sitemap-membership': ['SITEMAP-QUELLEN', 'Sitemaps & URL-Mitgliedschaft', 'Originale Deklarationen, Quellen und XML-URLs aus dem gespeicherten Crawl. Mitgliedschaft und Abrufzulassung bleiben getrennt.'],
    'sitemap-findings': ['GESPEICHERTE SITEMAP-BEFUNDE', 'Sitemap-Befunde', 'Alle gespeicherten Kandidaten nach Originalidentität prüfen. Messgrenzen, unbekannte Historie und Zielbefunde bleiben getrennt.'],
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
    'custom-search': ['CUSTOM DATA', 'Eigene Suche', 'Gespeicherte Literal- und Regex-Suche je Regel und Quelle. Exakte Anzahl, beobachtete Untergrenze und begrenzte Beispiele bleiben getrennt; fehlende Aufzeichnungen sind unbekannt.'],
    javascript: ['CUSTOM DATA', 'JavaScript-Auswertungen', 'Eigene Scripts im isolierten Seitenkontext nach der Render-Wartezeit. Werte und Fehler bleiben getrennt.'],
    cookies: ['BROWSER ANALYSE', 'Cookies', 'Cookie-Eigenschaften aus einzelnen isolierten Seitenaufrufen. Aufnahme muss aktiviert werden; Werte werden ausgelassen.'],
    accessibility: ['AUTOMATISIERTE PRÜFUNG', 'Barrierefreiheit', 'Gespeicherte axe-Beobachtungen der geladenen Seiten: Verstöße und manuell zu prüfende Befunde bleiben getrennt. Kein Nachweis vollständiger WCAG-Konformität. Der vollständige CSV liest alle durch das gespeicherte Berichtsprofil sichtbaren Seiten.'],
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
  const searchSource = value => ({raw_html:'Rohes HTML',rendered_html:'Gerendertes HTML',raw_selected_text:'Ausgewählter Roh-Text',rendered_selected_text:'Ausgewählter DOM-Text'}[value] || value || 'Nicht erfasst');
  const searchStatus = value => ({complete:'Vollständig',incomplete:'Unvollständig',unavailable:'Nicht verfügbar'}[value] || 'Nicht erfasst / unbekannt');
  function customSearchRows() {
    return state.pages.flatMap(page => array(page.custom_search).map(entry => ({...entry,url:page.url,_page:page,_search:entry})));
  }
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
  const recrawlPage = row => !row._page || row._page.url === row.url ? (state.segmentView && Number.isSafeInteger((row._page || row)._segmentOrdinal) ? row._page || row : state.pageMap.get(row.url)) : null;
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
    screenshotController?.invalidate();
    dashboardSegments?.invalidate();
    invalidateContentRequests('Der Crawl-Stand wurde geändert.', true);
    resetResourceLedger();
    resetQuotaSummary();
    resetSitemapFindings();
    resetSitemapMembership();
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
      if((nextCrawl.generation??null)!==(state.crawl.generation??null))dashboardSegments?.invalidate();
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
      if ((nextCrawl.generation ?? null) !== (state.crawl.generation ?? null)) { resetResourceLedger(); resetSitemapMembership(); resetSitemapBundle(); resetSitemapFindings(); }
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
      loadQuotaSummary();
      if (policySection() && !state.policyToken) loadPolicyPage(state.policyCursor);
      if (contentSection() && !active() && !state.content.inventory && !state.content.inventoryToken) loadContentInventory();
      if (resourceSection() && !state.resources.token) loadResourcePage();
      if (sitemapSection() && !sitemapMembership.result && !sitemapMembership.token) loadSitemapPage();
      if (findingsSection() && !sitemapFindings.result && !sitemapFindings.token && !sitemapFindings.queryDirty && !sitemapFindings.error) loadSitemapFindings();
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
        renderControls();
        if (generation === state.generation && state.crawl?.id === id && active()) state.pollTimer = setTimeout(() => refresh(generation), 2000);
      }
    }
  }

  async function selectCrawl(id) {
    if (typeof configProfiles !== 'undefined' && configProfiles) configProfiles.edited();
    invalidatePoll();
    resetSitemapBundle();
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
    tableQueryEdited();
    tableExplorer?.resetOwner();
    state.search='';state.filter='all';state.sort=state.section==='inputs'?'list_index':'url';state.direction=1;
    $('table-search').value='';$('table-filter').value='all';
    state.crawl = id ? state.crawls.find((crawl) => crawl.id === id) || { id, status: 'running' } : null;
    invalidateProjectColumns();
    renderSourceBodies();
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

  function configProfileStartRefusal() {
    if(configProfiles?.state.restoreError)return 'Gespeicherter Profilentwurf konnte nicht wiederhergestellt werden: '+configProfiles.state.restoreError+' Der Crawlstart bleibt gesperrt. In den Einstellungen ein gültiges Profil prüfen und ausdrücklich speichern.';
    if(configProfiles?.state.restorePending)return 'Der gespeicherte Profilentwurf wurde noch nicht erfolgreich wiederhergestellt. Der Crawlstart bleibt gesperrt. In den Einstellungen ein gültiges Profil prüfen und ausdrücklich speichern.';
    return configProfiles?.state.busy||configProfileCatalog?.state.busy?'Die Profilprüfung oder Bibliotheksaktion läuft noch. Bitte vor dem Crawlstart abschließen.':'';
  }
  async function startCrawl(event) {
    event.preventDefault();
    const refusal=configProfileStartRefusal();if(refusal){message(refusal,true);return;}
    if (actionBusy() || active()) return;
    if (typeof configProfiles !== 'undefined' && configProfiles?.state.active) { await startProfileCrawl(); return; }
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
      for (const key of ['tls_enabled','tls_respect_hsts','tls_certificates_json','respect_noindex','respect_canonical','respect_next_prev','cdn_directory_reporting_enabled','cdn_directory_rules_text','cdn_classification_enabled','cdn_origins_text','crawl_policy_enabled', 'crawl_policy_json', 'live_policy_limits_json', 'content_capture_enabled', 'content_capture_json','resource_discovery_enabled','resource_discovery_json','resource_discovery_limits_json']) delete payload[key];
      for (const key of [...crawlQuotaSettingKeys, ...crawlAdmissionSettingKeys, ...sitemapArchiveSettingKeys, 'sitemap_source_processing_enabled', ...sitemapSourceFields.map(field => field.setting), 'sitemap_membership_limits_enabled', ...sitemapMembershipFields.map(field => field.setting)]) delete payload[key];
      delete payload.custom_search_enabled; delete payload.custom_search_json;
      for(const key of ['renderer_viewport_enabled','renderer_viewport_width','renderer_viewport_height'])delete payload[key];
      payload.renderer = state.settings.javascript_rendering ? { endpoint: state.settings.renderer_endpoint, wait_ms: state.settings.renderer_wait_ms, accessibility: state.settings.renderer_accessibility, screenshot: state.settings.renderer_screenshot, ...(rendererViewport(state.settings) ? {viewport:rendererViewport(state.settings)} : {}), collect_cookies: state.settings.renderer_collect_cookies, custom_javascript: parseCustomJavascript(state.settings.custom_javascript_json) } : null;
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
    if (quotaUI.result?.summary?.deferred_targets > 0 && state.crawl.config?.crawl_quotas) { openQuotaIncrease(); return; }
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
    if(screenshotView?.previewOwner&&!window.SpiderScreenshots?.sameOwner(screenshotView.previewOwner,screenshotOwner()))clearScreenshotPreview();
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
    const hasData = Boolean(state.crawl && state.pages.length),sourceAvailability=tableSourceAvailability();
    $('export-csv').disabled = policySection() ? !policyVisibleRows().length || Boolean(state.policyToken || state.policyError) : !sourceAvailability.ok || (!hasData && !state.issues.length && !sourceAvailability.captured) || state.section === 'responses' && state.responses == null || state.section === 'fragments' && (!state.fragments || !state.fragments.enabled);
    $('export-csv').title=sourceAvailability.ok?tableCSVExportTitle():sourceAvailability.error.message;
    $('export-sitemap').disabled = !hasData || Boolean(busy || active());
    $('export-project').disabled = !state.crawl || Boolean(busy || active());
    $('export-visible-report').disabled = $('export-project').disabled;
    syncDirectiveReportingProfileSource();
    $('export-resources').disabled = !state.crawl || Boolean(busy || active());
    $('recrawl-selected-count').textContent = `${number(state.recrawlURLs.size)} URLs ausgewählt`;
    $('recrawl-selected').disabled = !canRecrawl();
    $('recrawl-clear').disabled = !state.recrawlURLs.size || busy;
    $('recrawl-selection-hint').textContent = active() ? 'Die Auswahl ist nach dem Anhalten oder Abschluss verfügbar.' : state.crawl?.recrawl_available === false ? 'Für diesen Bericht ist keine Wiederprüfung verfügbar.' : '';
    if (contentSection()) $('export-csv').disabled = active() || Boolean(state.content.inventoryToken || state.content.error) || !contentVisibleRows().length;
    if (state.section==='resource-discovery' || sitemapSection() || findingsSection()) $('export-csv').disabled=true;
    $('inspector-recrawl-selection').hidden = !state.selected || !state.pageMap.has(state.selected.url) || Boolean(state.selectedResponse || state.selectedInput);
    $('inspector-recrawl-checkbox').checked = Boolean(state.selected && state.recrawlURLs.has(state.selected.url));
    $('inspector-recrawl-checkbox').disabled = Boolean(busy || active() || state.crawl?.recrawl_available === false);
    if(state.segmentView)$('export-sitemap').disabled=true;
    if(dashboardSegments?.state.busy){$('export-csv').disabled=true;$('export-resources').disabled=true;$('export-visible-report').disabled=true;}
    if(tableExplorer && !$('table-view').hidden && !tableExplorer.canExport())$('export-csv').disabled=true;
    renderTableExport();
    renderProjectColumns();
    renderDashboardSegments();
    renderSitemapBundleControls();
    renderQuotaSummary();
    $('settings-summary').textContent = `${state.settings.concurrency} parallele Requests · ${state.settings.obey_robots ? 'Robots.txt beachten' : 'Robots.txt ignorieren'}${state.settings.max_pages ? ` · Grenze ${number(state.settings.max_pages)}` : ''}${state.settings.javascript_rendering ? ' · JavaScript-Rendering' : ''}${state.settings.crawl_policy_enabled ? ' · Native URL-Policy' : ''}${state.settings.resource_discovery_enabled ? ' · Native Ressourcenregeln' : ''}${state.settings.sitemap_source_processing_enabled ? ' · Eigene Sitemap-Grenzen' : ''}`;
    if(typeof configProfiles!=='undefined'&&configProfiles){if(configProfiles.state.active)$('settings-summary').textContent='Vollständiges natives Profil · '+configProfiles.unresolved().length+' erforderliche Headerwerte offen';if(configProfiles.state.restoreError||configProfiles.state.restorePending)$('settings-summary').textContent='Gespeichertes Profil nicht wiederhergestellt · Crawlstart gesperrt · Einstellungen prüfen';if(configProfileStartRefusal())$('start-button').disabled=true;}
  }

  function renderMetrics() {
    renderRecrawlOperation();
    const current = stats(),cohort=state.segmentView?.cohort.summary;
    const fetched = cohort?.selected_pages ?? current.fetched ?? state.pages.filter((page) => page.status != null || page.error || page.blocked_by_robots).length;
    const globalPages=state.segmentBase?.pages??state.pages;
    const globalFetched=current.fetched??globalPages.filter(page=>page.status!=null||page.error||page.blocked_by_robots).length;
    const discovered = current.discovered ?? globalPages.length;
    const queued = current.queued ?? Math.max(0, discovered - globalFetched);
    const failed = cohort?.failed_pages ?? current.failed ?? state.pages.filter((page) => page.error).length;
    const html = htmlPages();
    $('metric-fetched').textContent = number(fetched);
    $('metric-queued').textContent = number(queued);
    $('metric-html').textContent = number(cohort?.html_pages ?? html.length);
    $('metric-issues').textContent = number(cohort?.selected_issues ?? Math.max(current.issues || 0, state.issues.length));
    $('metric-url-label').textContent=cohort?'Seiten im Segment':'URLs';
    $('metric-queue-label').textContent=cohort?'Warteschlange · gesamter Crawl':'Warteschlange';
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
    $('metric-indexable').textContent = `${number(cohort?.indexable_pages ?? html.filter((page) => page.indexable === true).length)} indexierbar`;
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
    const fraction = discovered ? Math.min(1, globalFetched / discovered) : 0;
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
    if (state.segmentView && !state.pages.length) { element.innerHTML='<div class="empty-overview"><h3>Keine Seiten in dieser Segmentauswahl</h3><p>Bestätigte leere Auswahl des gespeicherten Berichts. Unbekannte Prüfungen und ausgeblendete Beobachtungen bleiben getrennt.</p></div>';return; }
    if (!state.pages.length && !state.issues.length && state.crawl?.reporting_visibility?.captured_pages > 0 && state.crawl.reporting_visibility.visible_pages === 0) {
      element.innerHTML = directiveReportingNote() + '<div class="empty-overview"><h3>Alle erfassten Seiten sind ausgeblendet</h3><p>Die gespeicherten Berichtsfilter blenden die Seiten aus. Der vollständige Roh-Snapshot bleibt verfügbar.</p></div>';
      return;
    }
    if (!state.pages.length && !state.issues.length) {
      element.innerHTML = directiveReportingNote() + `<div class="empty-overview"><div class="empty-icon" aria-hidden="true">⌘</div><h3>${state.crawl ? 'Warte auf die ersten Antworten' : 'Noch keine Daten'}</h3><p>${state.crawl ? 'Ergebnisse erscheinen, sobald URLs abgerufen wurden.' : 'Gib oben eine URL ein und starte einen Crawl oder öffne einen gespeicherten.'}</p><div class="feature-pills"><span>HTTP & Redirects</span><span>Titles & Descriptions</span><span>Canonical & Robots</span><span>Links & Hreflang</span><span>CSV & Sitemap</span></div></div>`;
      return;
    }
    const pages = internals();
    const nativeOverview=state.segmentView?.cohort.charts.overview;
    const buckets = nativeOverview ? [['2xx · Erfolgreich',nativeOverview.success,''],['3xx · Weiterleitung',nativeOverview.redirect,'warning'],['4xx · Client-Fehler',nativeOverview.client_error,'danger'],['5xx · Server-Fehler',nativeOverview.server_error,'danger'],['Nicht abgerufen',nativeOverview.unfetched,'warning']] : [
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
    element.innerHTML = directiveReportingNote() + `<div class="overview-grid"><div class="overview-card"><h3>Antworten & Abrufstatus</h3>${buckets.map(([label, count, kind]) => `<div class="status-summary-row"><span>${escape(label)}</span><div class="bar ${kind}"><span style="width:${Math.min(100, pages.length ? count / pages.length * 100 : 0)}%"></span></div><strong>${number(count)}</strong></div>`).join('')}</div><div class="overview-card"><h3>Erkannte Probleme</h3>${groups.length ? groups.map((group) => `<button class="issue-summary-row text-button" style="width:100%;text-align:left" data-issue-filter="${escape(group.code)}"><i class="severity-dot ${escape(group.severity)}"></i><span>${escape(issueTitle(group.code))}</span><strong>${number(group.count)}</strong></button>`).join('') : '<p class="detail-empty">Keine Probleme gemeldet.</p>'}</div></div><div class="overview-footnote"><span aria-hidden="true">ⓘ</span><span>${state.crawl?.config?.renderer ? 'Mit JavaScript-Rendering. Renderstatus je Seite in den Details.' : 'Basis: HTTP-Antworten und HTML, ohne JavaScript-Rendering.'}</span></div>`;
    element.querySelectorAll('[data-issue-filter]').forEach((button) => button.addEventListener('click', () => {
      selectSection('issues');
      state.search = button.dataset.issueFilter;
      $('table-search').value = state.search;
      renderTable();
    }));
  }

  function htmlHeadDetailFields(page) {
    const facts = page?.html_head_observation;
    const unknown = [['Raw-HTTP-Head', 'Unbekannt – keine passende gespeicherte Raw-Beobachtung; nicht nachträglich aus dem DOM abgeleitet.']];
    if (!facts || facts.engine !== 'open-seo-spider/raw-http-head-v1' || facts.parser_profile !== 'html5ever-0.39-tokenizer-explicit-head-scripting-enabled-v1' || facts.source !== 'raw_http_decoded_utf8' || facts.document_url !== page.final_url || facts.fetched_at !== page.fetched_at) return unknown;
    const span = value => value ? '<' + value.element + '> · decodierte UTF-8-Bytes ' + value.start_utf8_byte + '..' + value.end_utf8_byte + ' · Zeile ' + value.line + ', Unicode-Spalte ' + value.unicode_column + '\n' + value.raw_excerpt + (value.excerpt_truncated ? '\n[Quellvorschau gekürzt; Endposition gehört zum vollständigen Tag.]' : '') : 'Nicht beobachtet';
    return [
      ['Raw-HTTP-Head', facts.status === 'complete' ? facts.first_invalid_child ? 'Ungültiges Kind im gespeicherten expliziten Raw-Head; nachfolgende Metadaten prüfen.' : 'Begrenztes explizites Raw-Head geprüft; kein ungültiges Element beobachtet.' : 'Unbekannt / nicht vollständig beurteilt: ' + (facts.reason || 'Grund nicht gespeichert')],
      ['Raw-Head-Quelle', facts.document_url + '\n' + facts.fetched_at + '\nNur gelieferte HTTP-Quelle, kein Nachweis der Google-Verarbeitung oder des gerenderten DOM.'],
      ['Raw-Head-Quellbereich', facts.scanned_prefix_utf8_bytes + ' von ' + facts.decoded_source_utf8_bytes + ' decodierten UTF-8-Bytes geprüft · Präfix-SHA256 ' + facts.scanned_prefix_sha256],
      ['Erstes ungültiges Head-Kind', span(facts.first_invalid_child)],
      ['Metadatenelemente danach', Array.isArray(facts.following_metadata) && facts.following_metadata.length ? facts.following_metadata.slice(0, 8).map(span).join('\n\n') : 'Keine in diesem Quellbereich beobachtet; unbekannte Prüfung ist kein Nachweis für Abwesenheit.']
    ];
  }

  function issueTitle(code) {
    const names = { missing_title: 'Seitentitel fehlt', title_missing: 'Seitentitel fehlt', duplicate_title: 'Doppelter Seitentitel', title_duplicate: 'Doppelter Seitentitel', missing_meta_description: 'Meta Description fehlt', meta_description_missing: 'Meta Description fehlt', missing_h1: 'H1 fehlt', h1_missing: 'H1 fehlt', multiple_h1: 'Mehrere H1-Überschriften', h1_multiple: 'Mehrere H1-Überschriften', broken_link: 'Defektes Linkziel', http_error: 'HTTP-Fehler', missing_alt: 'Bild ohne Alt-Attribut', image_missing_alt: 'Bild ohne Alt-Attribut', noindex: 'Noindex-Direktive', robots_blocked: 'Durch Robots.txt ausgeschlossen', redirect: 'Weiterleitung', canonical_missing: 'Canonical fehlt', canonical_multiple: 'Mehrere Canonicals', html_head_invalid_child: 'Ungültiges Element im Raw-Head', html_head_metadata_after_invalid_child: 'Metadaten nach ungültigem Raw-Head-Element' };
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
    const records = state.segmentView ? new Map(relationshipPages().map(page=>[page.url,page])) : state.pageMap;
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
    const aliases = new Map(relationshipPages().map(page => [page.url, page]));
    for (const page of relationshipPages()) if (page.final_url && !aliases.has(page.final_url)) aliases.set(page.final_url, page);
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

  function accessibilityObservation(page) {
    const raw=page.accessibility, unknown=(status,reason)=>({status,reason,violations:null,incomplete:null,passes:null,inapplicable:null});
    if(raw==null)return unknown(page.renderer_error?'renderer_error':'missing','Keine gespeicherte axe-Beobachtung');
    if(typeof raw!=='object'||Array.isArray(raw))return unknown('malformed','Ungültige gespeicherte Beobachtung');
    if(raw.error!=null)return unknown('error','Gespeicherter Fehler der axe-Prüfung');
    const impact=value=>value===null||['minor','moderate','serious','critical'].includes(value);
    const target=(value,depth=0)=>depth<=8&&(typeof value==='string'||Array.isArray(value)&&value.length>0&&value.every(v=>target(v,depth+1)));
    const rule=value=>value&&typeof value==='object'&&['id','description','help','helpUrl'].every(key=>typeof value[key]==='string')&&value.id.length>0&&impact(value.impact)&&Array.isArray(value.tags)&&value.tags.every(v=>typeof v==='string')&&Array.isArray(value.nodes)&&value.nodes.length>0&&value.nodes.every(node=>node&&typeof node==='object'&&typeof node.html==='string'&&Array.isArray(node.target)&&target(node.target)&&(node.failureSummary==null||typeof node.failureSummary==='string')&&['any','all','none'].every(key=>Array.isArray(node[key])&&node[key].every(check=>check&&typeof check==='object'&&typeof check.id==='string'&&impact(check.impact)&&(check.message==null||typeof check.message==='string'))));
    if(raw.engine!=='axe-core'||typeof raw.version!=='string'||!raw.version||raw.automated_only!==true||![raw.passes_count,raw.inapplicable_count].every(n=>Number.isSafeInteger(n)&&n>=0)||![raw.violations,raw.incomplete].every(rows=>Array.isArray(rows)&&rows.every(rule)))return unknown('malformed','Unvollständige oder ungültige axe-Daten; keine bestätigten Nullwerte');
    return {status:raw.incomplete.length?'needs_review':raw.violations.length?'violations':'observed_zero',reason:'',violations:raw.violations.length,incomplete:raw.incomplete.length,passes:raw.passes_count,inapplicable:raw.inapplicable_count};
  }
  const accessibilityLabel=value=>({needs_review:'Manuell prüfen',violations:'Automatisierte Verstöße',observed_zero:'Keine automatisierten Befunde',missing:'Nicht erfasst',renderer_error:'Rendererfehler',error:'Prüfung fehlgeschlagen',malformed:'Unbekannt: ungültige Daten'}[value]||'Unbekannt');

  // Trust describes the captured extraction input, independently of max_results.
  // Imported observations are validated natively; these checks also fail closed
  // for stale/malformed browser data. A hash is an input binding, not website authenticity.
  function extractionSourceState(row) {
    const unknown=reason=>({status:'unknown',evaluation:'unknown',decode:'unknown',exactCount:null,scope:'unknown',reasons:[reason]});
    const o=row?.source_observation;
    if(o==null)return unknown('missing_saved_observation');
    const hash=value=>typeof value==='string'&&/^[a-f0-9]{64}$/.test(value);
    const natural=value=>Number.isSafeInteger(value)&&value>=0;
    const values=Array.isArray(row.values)?row.values:[];
    if(typeof o!=='object'||Array.isArray(o)||o.engine!=='open-seo-spider/extraction-source-v1'||!['raw_html','rendered_html'].includes(o.source)||o.source!==row.source||!['complete','incomplete','unavailable'].includes(o.status)||!['evaluated','error','not_run'].includes(o.evaluation)||!['exact','lossy','unknown'].includes(o.decode_status)||!Array.isArray(o.reasons)||o.reasons.some(reason=>typeof reason!=='string'||!reason||reason.length>256)||o.reasons.length>16||!hash(o.source_revision)||(o.observed_generation!=null&&!natural(o.observed_generation)))return unknown('invalid_source_observation');
    const hasInput=hash(o.input_sha256)&&natural(o.input_utf8_bytes);
    const noInput=o.input_sha256==null&&o.input_utf8_bytes==null;
    const evaluated=o.evaluation==='evaluated'&&!row.error&&natural(row.matched_count)&&row.matched_count>=values.length&&typeof row.truncated==='boolean'&&row.truncated===(row.matched_count>values.length);
    const failed=o.evaluation==='error'&&typeof row.error==='string'&&Boolean(row.error)&&row.matched_count==null&&!values.length&&row.truncated===false;
    const notRun=o.evaluation==='not_run'&&!row.error&&row.matched_count==null&&!values.length&&row.truncated===false;
    if(o.status==='unavailable'?(!noInput||!notRun||!o.reasons.length):(!hasInput||!(evaluated||failed)))return unknown('invalid_source_observation');
    if(o.status==='complete'&&(o.decode_status!=='exact'||o.reasons.length||typeof o.document_url!=='string'||!o.document_url||typeof o.observed_at!=='string'||!Number.isFinite(Date.parse(o.observed_at))))return unknown('invalid_source_observation');
    if(o.status==='incomplete'&&!o.reasons.length)return unknown('invalid_source_observation');
    return {status:o.status,evaluation:o.evaluation,decode:o.decode_status,exactCount:o.status==='complete'&&evaluated?row.matched_count:null,scope:evaluated?(o.status==='complete'?'complete_source':'evaluated_input_only'):failed?'evaluation_failed':'not_evaluated',reasons:o.reasons};
  }
  const extractionSourceLabel=status=>({complete:'Vollständig erfasst',incomplete:'Quelle unvollständig',unavailable:'Nicht ausgewertet',unknown:'Quellvertrauen unbekannt'}[status]||'Unbekannt');
  function extractionRows() {
    if(state.cache.extractions)return state.cache.extractions;
    const configs=array(state.crawl?.config?.custom_extractions);
    const identity=entry=>JSON.stringify([entry?.name||'',entry?.selector||'',entry?.attribute??null,entry?.method||'css',entry?.output||'auto']);
    const rows=[];
    for(const page of internals()){
      const saved=array(page.custom_extractions),remaining=new Map();
      for(const entry of saved){const key=identity(entry);remaining.set(key,(remaining.get(key)||0)+1);rows.push({...entry,url:page.url,name:entry?.name||entry?.selector||'',values:array(entry?.values),_page:page});}
      for(const config of configs){const key=identity(config),count=remaining.get(key)||0;if(count){remaining.set(key,count-1);continue;}rows.push({...config,url:page.url,name:config.name||config.selector,source:'',values:[],matched_count:null,truncated:null,error:null,_page:page,_missingObservation:true});}
    }
    state.cache.extractions=rows;
    return rows;
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
      case 'extractions': return extractionRows();
      case 'custom-search': return customSearchRows();
      case 'javascript': return internals().flatMap(page => page.custom_javascript_results.map(entry => ({ ...entry, url: page.url, value_json: entry.error ? '' : JSON.stringify(entry.value), _page: page })));
      case 'cookies': return internals().flatMap(page => page.browser_cookies.map(entry => ({ ...entry, url: page.url, _page: page })));
      case 'accessibility': return state.pages.map(page=>({...accessibilityObservation(page),url:page.url,_page:page,axe_version:typeof page.accessibility?.version==='string'?page.accessibility.version:null}));
      case 'titles': { if (state.cache.titles) return state.cache.titles; const duplicates = duplicateValues('titles'); state.cache.titles = htmlPages().map((page) => ({ ...page, title_text: primary(page.titles), title_length: Array.from(primary(page.titles)).length, duplicates: duplicates.get(primary(page.titles).trim()) || 0 })); return state.cache.titles; }
      case 'meta': { if (state.cache.meta) return state.cache.meta; const duplicates = duplicateValues('meta_descriptions'); state.cache.meta = htmlPages().map((page) => ({ ...page, description_text: primary(page.meta_descriptions), description_length: Array.from(primary(page.meta_descriptions)).length, duplicates: duplicates.get(primary(page.meta_descriptions).trim()) || 0 })); return state.cache.meta; }
      case 'headings': return htmlPages();
      default: return internals();
    }
  }

  const textColumn = (key, label, getter = (row) => row[key], klass = '') => ({ key, label, type:'text', value: getter, render: (row) => `<span class="${klass || 'truncate'}" title="${escape(getter(row) ?? '')}">${escape(getter(row) ?? '—')}</span>` });
  const booleanExportValue = (row,key) => !row._unavailable&&typeof row[key]==='boolean'?row[key]:null;
  const booleanColumn = (key,label,getter) => ({...textColumn(key,label,getter),exportValue:row=>booleanExportValue(row,key)});
  const urlColumn = textColumn('url', 'URL', (row) => row.url, 'url-cell');
  const numberColumn = (key, label, getter = (row) => row[key]) => ({ key, label, type:'number', value: getter, render: (row) => getter(row) == null ? '—' : escape(number(getter(row))) });
  const statusColumn = { key: 'status', label: 'HTTP final', value: effectiveStatus, render: (row) => statusBadge(effectiveStatus(row)) + (row.tls?.hsts_upgrades?.length ? ' <span class="badge" title="Lokale Transportregel, keine HTTP-Weiterleitung">HSTS 307</span>' : '') };
  const indexColumn = { key: 'indexable', label: 'Indexierbarkeit', value: (row) => row.indexable === true ? 'Indexierbar' : row.indexable === false ? 'Nicht indexierbar' : 'Unbekannt', exportValue: row => booleanExportValue(row,'indexable'), render: (row) => `<span class="badge ${row.indexable === false ? 'warning' : row.indexable == null ? 'neutral' : ''}">${row.indexable === true ? 'Indexierbar' : row.indexable === false ? 'Nicht indexierbar' : 'Unbekannt'}</span>` };
  const issueColumn = numberColumn('issue_count', 'Probleme', (row) => pageIssues(row.url).length);
  const domColumns = () => [numberColumn('dom_index', 'DOM-Index', row => row.dom_location?.index), textColumn('dom_path', 'DOM-Pfad', domPath), textColumn('dom_source', 'DOM-Quelle', domSource), textColumn('dom_complete', 'Vollständiger Prüfbefund', row => !row._page?.dom_metadata ? 'Nicht erfasst' : completeDOM(row._page) ? 'Ja' : 'Nein / unklar')];

  function columns() {
    switch (state.section) {
      case 'custom-search': return [urlColumn,textColumn('rule_name','Suchregel'),textColumn('method','Methode'),textColumn('source','Quelle',row=>searchSource(row.source)),textColumn('status','Suchzustand',row=>searchStatus(row.status)),numberColumn('count','Exakte Gesamtanzahl',row=>row.status==='complete'?row.count:null),numberColumn('matches_observed','Beobachtete Untergrenze'),numberColumn('saved_matches','Gespeicherte Beispiele',row=>array(row.matches).length),booleanColumn('evidence_complete','Beispiele vollständig',row=>row.evidence_complete===true?'Ja':row.evidence_complete===false?'Nein':'Unbekannt'),textColumn('reasons','Gründe',row=>array(row.reasons).join(' | '))];
      case 'accessibility': return [urlColumn,textColumn('status','Beobachtung',row=>accessibilityLabel(row.status)),numberColumn('violations','Verstoßregeln'),numberColumn('incomplete','Manuell zu prüfende Regeln'),numberColumn('passes','Bestandene Regeln'),numberColumn('inapplicable','Nicht anwendbare Regeln'),textColumn('axe_version','axe-Version'),textColumn('reason','Hinweis')];
      case 'inputs': return [numberColumn('list_index', 'Liste'), textColumn('input_sha256', 'Quellhash'), numberColumn('source_index', 'Quellindex'), numberColumn('source_row', 'Quellzeile'), numberColumn('source_line', 'Physische Zeile'), textColumn('sheet', 'Worksheet'), textColumn('cell', 'Zelle'), textColumn('raw_value', 'Originalwert', row => row.raw_value == null ? 'Nicht erfasst' : row.raw_value === '' ? '[leer]' : row.raw_value), textColumn('normalized_url', 'Normalisierte URL'), textColumn('fragment', 'Fragment'), textColumn('candidate_kind', 'Kandidat', row => row.candidate_kind === 'page_url' ? 'Seiten-URL' : row.candidate_kind === 'sitemap_document' ? 'Sitemap-Dokument' : 'Nicht erfasst'), textColumn('error', 'Importfehler')];
      case 'external': return [urlColumn, { key: 'status', label: 'HTTP geprüft', value: (row) => row.status, render: (row) => statusBadge(row.status) }, textColumn('text', 'Linktext'), numberColumn('source_count', 'Verlinkende Seiten'), booleanColumn('nofollow', 'Nofollow', (row) => row.nofollow ? 'Ja' : 'Nein')];
      case 'status': return [urlColumn, { key: 'initial_status', label: 'HTTP initial', value: (row) => row.status, render: (row) => statusBadge(row.status) }, statusColumn, textColumn('final_url', 'Finale URL'), numberColumn('redirects', 'Redirects', (row) => row.redirect_chain.length), { key: 'response_ms', label: 'Antwortzeit', value: (row) => row.response_ms, render: (row) => duration(row.response_ms) }, indexColumn, textColumn('error', 'Abruffehler')];
      case 'responses': return [urlColumn, textColumn('requested_url', 'Anfrage-URL'), numberColumn('response_index', 'Antwortposition'), { key: 'status', label: 'HTTP beobachtet', value: row => row.status, render: row => statusBadge(row.status) }, textColumn('location', 'Location'), textColumn('capture_kind', 'Befund', row => row.capture_kind === 'redirect_hop' ? 'Zwischenredirect' : 'Terminale Antwort'), {...textColumn('headers_known', 'Header erfasst', row => row.headers == null ? 'Unbekannt' : 'Ja'),exportValue:row=>!row._unavailable&&row.headers&&typeof row.headers==='object'&&!Array.isArray(row.headers)?true:null}, textColumn('content_type', 'Content-Type'), numberColumn('body_bytes', 'Erfasste Bodybytes'), booleanColumn('body_complete', 'Body vollständig', row => row.body_complete == null ? 'Unbekannt' : row.body_complete ? 'Ja' : 'Nein'), booleanColumn('document_metadata_available', 'HTML-Metadaten verfügbar', row => row.document_metadata_available ? 'Ja' : 'Nein')];
      case 'titles': return [urlColumn, textColumn('title_text', 'Seitentitel'), numberColumn('title_length', 'Zeichen'), numberColumn('title_pixels', 'Pixelprofil 20px', row => array(row.title_pixel_widths)[0]), numberColumn('title_count', 'Title-Tags', (row) => row.titles.length), numberColumn('duplicates', 'Vorkommen'), statusColumn];
      case 'meta': return [urlColumn, textColumn('description_text', 'Meta Description'), numberColumn('description_length', 'Zeichen'), numberColumn('description_pixels', 'Pixelprofil 14px', row => array(row.description_pixel_widths)[0]), numberColumn('meta_count', 'Meta-Tags', (row) => row.meta_descriptions.length), numberColumn('duplicates', 'Vorkommen'), statusColumn];
      case 'headings': return [urlColumn, textColumn('h1', 'H1', (row) => first(row.h1)), numberColumn('h1_count', 'H1-Anzahl', (row) => row.h1.length), textColumn('h2', 'H2', (row) => first(row.h2)), numberColumn('h2_count', 'H2-Anzahl', (row) => row.h2.length), numberColumn('word_count', 'Wörter')];
      case 'headingorder': return [urlColumn, numberColumn('tag_level', 'Tag-Level'), textColumn('text', 'Heading-Text'), textColumn('raw_text', 'Originaler Text'), textColumn('text_with_image_alt', 'Text mit Inline-Bild-Alt', row => row.text_with_image_alt == null ? 'Nicht erfasst' : row.text_with_image_alt === '' ? '[leer]' : row.text_with_image_alt), textColumn('heading_text_engine', 'Text-/Alt-Profil', row => row._page?.dom_metadata?.heading_text_engine || 'Nicht erfasst'), textColumn('image_alt', 'Bild-Alt im Heading', row => row._unavailable ? 'Nicht erfasst' : array(row.image_alt).map(value => value == null ? '[Attribut fehlt]' : value === '' ? '[leer]' : value).join(' | ')), textColumn('aria_level', 'aria-level'), textColumn('heading_offset', 'headingoffset'), booleanColumn('hidden_attribute', 'hidden-Attribut', row => row.hidden_attribute == null ? 'Nicht erfasst' : row.hidden_attribute ? 'Ja' : 'Nein'), ...domColumns()];
      case 'keywords': return [urlColumn, textColumn('content', 'Originaler content', row => row._unavailable ? 'Nicht erfasst' : row.content == null ? '[content-Attribut fehlt]' : row.content === '' ? '[leer]' : row.content), textColumn('normalized', 'Normalisierter Text'), numberColumn('chars', 'Originale Zeichen'), ...domColumns()];
      case 'linkdom': return [urlColumn, textColumn('source_url', 'Quellseite'), textColumn('kind', 'Linkart'), textColumn('text', 'Linktext'), textColumn('raw_href', 'Originales Attribut'), textColumn('resolved_href', 'Aufgelöst inkl. Fragment'), textColumn('fragment', 'Fragment'), textColumn('position', 'Position', row => row.position ?? 'Nicht erfasst'), ...domColumns()];
      case 'imagecandidates': return [urlColumn, textColumn('image_src', 'img src'), textColumn('image_alt', 'img Alt'), textColumn('source', 'Kandidatenquelle'), textColumn('raw_url', 'Deklarierter Kandidat'), textColumn('candidate_url', 'Aufgelöste HTTP(S)-URL'), textColumn('descriptors', 'Deskriptoren', row => first(row.descriptors)), numberColumn('width', 'w-Deskriptor'), numberColumn('density', 'x-Deskriptor'), textColumn('media', 'Media'), textColumn('type_hint', 'Typdeklaration'), textColumn('sizes', 'Sizes'), booleanColumn('eligible', 'Deklaration akzeptiert', row => row.eligible == null ? 'Nicht erfasst' : row.eligible ? 'Ja' : 'Nein'), textColumn('selection', 'Browserauswahl', () => 'Nicht erfasst'), textColumn('error', 'Kandidatenfehler'), ...domColumns()];
      case 'navigation': return [urlColumn, textColumn('tag', 'Element'), textColumn('reason', 'Beobachtung'), textColumn('text', 'Elementtext'), textColumn('raw_href', 'Originales href / Routerattribut'), textColumn('resolved_href', 'Aufgelöste Adresse'), textColumn('potential_url', 'Mögliche HTTP(S)-URL'), textColumn('scheme', 'Scheme'), textColumn('onclick', 'onclick-Attribut'), textColumn('router_link', 'Router-/Data-Attribut'), ...domColumns()];
      case 'fragments': return [urlColumn, textColumn('source_document_url', 'Quelldokument'), textColumn('target_url', 'Ziel-URL'), textColumn('resolved_href', 'Aufgelöstes href'), textColumn('fragment', 'Originalfragment'), textColumn('decoded_fragment', 'Dekodiertes Fragment'), textColumn('status', 'Prüfergebnis', row => fragmentStatus(row.status)), textColumn('reason', 'Begründung'), textColumn('target_document_url', 'Erfasstes Zieldokument'), booleanColumn('target_complete', 'Ziel vollständig', row => row.target_complete ? 'Ja' : 'Nein / unklar'), textColumn('matched_target', 'Gefundener Zieltyp', row => row.matched_target?.kind), textColumn('target_dom_path', 'Ziel-DOM-Pfad', row => row.matched_target?.dom_location?.path), ...domColumns().slice(0, 2)];
      case 'canonical': return [urlColumn, textColumn('canonical', 'Canonical-Ziel', (row) => first(row.canonical)), numberColumn('canonical_count', 'Deklarationen', canonicalCount), {...textColumn('self_reference', 'Selbstreferenz', (row) => row.canonical.length ? row.canonical.includes(row.rendered && !row.renderer_error ? row.rendered_url || row.final_url || row.url : row.final_url || row.url) ? 'Ja' : 'Nein' : '—'),exportValue:row=>!row._unavailable&&Array.isArray(row.canonical)&&row.canonical.length&&row.canonical.every(value=>typeof value==='string')?row.canonical.includes(row.rendered&&!row.renderer_error?row.rendered_url||row.final_url||row.url:row.final_url||row.url):null}, indexColumn];
      case 'robots': return [urlColumn, textColumn('blocked', 'Robots.txt', (row) => row.blocked_by_robots ? 'Ausgeschlossen' : row.robots_warning ? `Hinweis: ${row.robots_warning}` : effectiveStatus(row) == null ? 'Unbekannt' : 'Nicht ausgeschlossen'), textColumn('meta_robots', 'Meta Robots', (row) => first(row.meta_robots)), textColumn('x_robots_tag', 'X-Robots-Tag', (row) => first(row.x_robots_tag)), indexColumn, textColumn('indexability_reasons', 'Begründung', (row) => first(row.indexability_reasons))];
      case 'hreflang': return [urlColumn, textColumn('language', 'HTML-Sprache'), numberColumn('alternates', 'Sprachalternativen', (row) => row.hreflang.length), textColumn('hreflang', 'Deklarierte Alternativen', (row) => row.hreflang.map((entry) => `${entry.language || entry.lang || '?'}: ${entry.url || entry.href || ''}`).join(' | ')), statusColumn];
      case 'images': return [urlColumn, textColumn('alt', 'Alt-Text', (row) => row.alt == null ? '[Attribut fehlt]' : String(row.alt).trim() === '' ? '[leer]' : row.alt), textColumn('source', 'Quellseite'), textColumn('width', 'Breite (HTML)'), textColumn('height', 'Höhe (HTML)'), textColumn('loading', 'Loading'), textColumn('resource_status', 'Dateianalyse', row => resourceStatus(row.resource_status)), numberColumn('intrinsic_width', 'Dateibreite'), numberColumn('intrinsic_height', 'Dateihöhe')];
      case 'imagefiles': return [urlColumn, statusColumn, textColumn('resource_status', 'Dateianalyse', row => resourceStatus(row.resource_status)), textColumn('format', 'Dateiformat'), numberColumn('intrinsic_width', 'Intrinsische Breite'), numberColumn('intrinsic_height', 'Intrinsische Höhe'), textColumn('dimension_unit', 'Maßeinheit'), numberColumn('encoded_bytes', 'Erfasste Dateibytes'), booleanColumn('body_complete', 'Body vollständig', row => row.body_complete == null ? 'Unbekannt' : row.body_complete ? 'Ja' : 'Nein'), textColumn('parse_error', 'Parserfehler')];
      case 'pdf': return [urlColumn, statusColumn, textColumn('resource_status', 'PDF-Analyse', row => resourceStatus(row.resource_status)), textColumn('pdf_title', 'PDF-Metadatentitel'), textColumn('author', 'Autor'), numberColumn('page_count', 'Geprüfte Seitenzahl'), numberColumn('declared_page_count', 'Deklarierte Seitenzahl'), numberColumn('pdf_words', 'Erfasste Wörter'), booleanColumn('text_complete', 'Text vollständig', row => row.text_complete == null ? 'Unbekannt' : row.text_complete ? 'Ja' : 'Nein'), numberColumn('annotation_count', 'URI-Annotationen'), booleanColumn('annotations_complete', 'Annotationen vollständig', row => row.annotations_complete == null ? 'Unbekannt' : row.annotations_complete ? 'Ja' : 'Nein'), textColumn('parse_error', 'Parserfehler')];
      case 'issues': return [urlColumn, { key: 'severity', label: 'Priorität', value: (row) => row.severity, render: (row) => `<span class="badge ${row.severity === 'error' ? 'danger' : row.severity === 'warning' ? 'warning' : 'neutral'}">${row.severity === 'error' ? 'Fehler' : row.severity === 'warning' ? 'Warnung' : 'Hinweis'}</span>` }, textColumn('code', 'Problem', (row) => issueTitle(row.code)), textColumn('message', 'Details'), numberColumn('related', 'Weitere URLs', (row) => array(row.related_urls).length)];
      case 'linkmetrics': return [urlColumn, { key: 'link_score', label: 'Eigener Linkscore', value: (row) => row.link_score, render: (row) => Number(row.link_score || 0).toLocaleString('de-DE', { maximumFractionDigits: 2 }) }, numberColumn('in_degree', 'Verlinkende URLs'), numberColumn('out_degree', 'Verlinkte URLs'), numberColumn('inlink_occurrences', 'Eingehende Anker'), numberColumn('outlink_occurrences', 'Ausgehende Anker'), numberColumn('crawl_depth', 'Crawl-Tiefe'), numberColumn('shortest_link_depth', 'Kürzester Linkweg'), indexColumn];
      case 'structured': return [urlColumn, numberColumn('parsed_blocks', 'JSON-LD-Blöcke'), numberColumn('invalid_json_blocks', 'Ungültiges JSON'), textColumn('types_text', 'Beobachtete @type-Werte'), numberColumn('typed_nodes', 'Knoten mit @type', (row) => array(row.typed_nodes).length), textColumn('validation', 'Schema-Validierung', () => 'Nicht ausgeführt')];
      case 'extractions': return [urlColumn, textColumn('name', 'Feldname'), textColumn('method', 'Methode', row => row.method || 'css'), textColumn('output', 'Ausgabe', row => row.output || 'auto'), textColumn('result_type', 'Ergebnistyp'), textColumn('selector', 'Ausdruck'), textColumn('attribute', 'Attribut', row => row.attribute || '—'), textColumn('source', 'Quelle', row => row.source === 'raw_html' ? 'HTML-Antwort' : row.source === 'rendered_html' ? 'Gerenderter DOM' : row.source || 'Unbekannt'), numberColumn('matches', 'Erfasste Werte', row => row.values.length), numberColumn('matched_count', 'Treffer im ausgewerteten Input'), booleanColumn('truncated', 'Werte begrenzt', row => row.truncated == null ? 'Unbekannt' : row.truncated ? 'Ja' : 'Nein'), textColumn('values', 'Werte', row => row.values.join(' | ')), textColumn('error', 'Fehler'), {key:'source_status',label:'Quellzustand',type:'text',value:row=>extractionSourceState(row).status,render:row=>escape(extractionSourceLabel(extractionSourceState(row).status))}, textColumn('evaluation_status', 'Auswertung', row => extractionSourceState(row).evaluation), textColumn('decode_status', 'Quelldekodierung', row => extractionSourceState(row).decode), numberColumn('exact_count', 'Exakte Gesamtanzahl', row => extractionSourceState(row).exactCount), textColumn('count_scope', 'Zählbereich', row => extractionSourceState(row).scope), textColumn('source_reasons', 'Quellhinweise', row => extractionSourceState(row).reasons.join(' | ')), textColumn('document_url', 'Dokument-URL', row => row.source_observation?.document_url), numberColumn('observed_generation', 'Erfasste Generation', row => row.source_observation?.observed_generation), textColumn('observed_at', 'Erfasst am', row => row.source_observation?.observed_at), textColumn('input_sha256', 'Eingabe-SHA256', row => row.source_observation?.input_sha256), numberColumn('input_utf8_bytes', 'Eingabe-UTF8-Bytes', row => row.source_observation?.input_utf8_bytes), textColumn('source_revision', 'Quellrevision', row => row.source_observation?.source_revision), textColumn('source_engine', 'Quellvertrag', row => row.source_observation?.engine)];
      case 'javascript': return [urlColumn, textColumn('name', 'Feldname'), textColumn('value_type', 'Datentyp'), textColumn('value_json', 'JSON-Rückgabe'), textColumn('error', 'Auswertungsfehler'), { key: 'elapsed_ms', label: 'Ausführungszeit', value: row => row.elapsed_ms, render: row => duration(row.elapsed_ms) }];
      case 'cookies': return [urlColumn, textColumn('name', 'Cookie-Name'), textColumn('domain', 'Domain'), textColumn('path', 'Pfad'), textColumn('sameSite', 'SameSite'), booleanColumn('httpOnly', 'HttpOnly', row => row.httpOnly ? 'Ja' : 'Nein'), booleanColumn('secure', 'Secure', row => row.secure ? 'Ja' : 'Nein'), numberColumn('value_length', 'Wertlänge in Bytes'), textColumn('expires', 'Ablauf', row => row.expires < 0 ? 'Session' : new Date(row.expires * 1000).toISOString())];
      case 'pagination': return [urlColumn, textColumn('pagination_next', 'Next', row => first(row.pagination_next)), textColumn('pagination_prev', 'Prev', row => first(row.pagination_prev)), textColumn('amphtml', 'AMP-Ziel', row => first(row.amphtml)), statusColumn, indexColumn];
      default: return [urlColumn, statusColumn, textColumn('content_type', 'Content-Type'), textColumn('titles', 'Seitentitel', (row) => first(row.titles)), indexColumn, numberColumn('depth', 'Tiefe'), { key: 'response_ms', label: 'Antwortzeit', value: (row) => row.response_ms, render: (row) => duration(row.response_ms) }, numberColumn('word_count', 'Wörter'), issueColumn];
    }
  }

  function filterOptions() {
    const base = [['all', 'Alle Ergebnisse']];
    switch (state.section) {
      case 'custom-search': return [...base,['complete','Vollständige Suche'],['incomplete','Unvollständige Suche'],['unavailable','Quelle nicht verfügbar'],['matched','Beobachtete Treffer'],['zero','Bestätigt keine Treffer'],['evidence-limited','Begrenzte Beispiele']];
      case 'accessibility': return [...base,['violations','Mit automatisierten Verstößen'],['needs_review','Manuell prüfen'],['observed_zero','Bestätigt keine automatisierten Befunde'],['unknown','Nicht erfasst / Fehler / ungültig']];
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
      case 'extractions': return [...base, ['present', 'Mit erfassten Werten'], ['empty', 'Bestätigt keine Werte'], ['error', 'Mit Auswertungsfehler'], ['truncated', 'Ergebniswerte begrenzt'], ['incomplete', 'Quelle unvollständig'], ['unknown', 'Quellvertrauen unbekannt'], ['unavailable', 'Nicht ausgewertet']];
      case 'javascript': return [...base, ['success', 'Mit Rückgabe'], ['error', 'Mit Auswertungsfehler']];
      case 'cookies': return [...base, ['insecure', 'Ohne Secure'], ['scriptable', 'Ohne HttpOnly'], ['session', 'Session-Cookies']];
      case 'pagination': return [...base, ['present', 'Mit Pagination'], ['amp', 'Mit AMP-Verweis']];
      default: return [...base, ['html', 'HTML'], ['2xx', 'HTTP 2xx'], ['3xx', 'Weiterleitungen'], ['4xx', 'HTTP 4xx'], ['5xx', 'HTTP 5xx'], ['nonindexable', 'Nicht indexierbar'], ['issues', 'Mit Problemen'], ['failed', 'Abruffehler']];
    }
  }

  function matchesFilter(row) {
    const filter = state.filter;
    if (filter === 'all') return true;
    if (state.section === 'custom-search') return filter === 'matched' ? row.matches_observed > 0 : filter === 'zero' ? row.status === 'complete' && row.count === 0 : filter === 'evidence-limited' ? row.evidence_complete === false : row.status === filter;
    if (state.section === 'accessibility') return filter==='violations'?row.violations>0:filter==='needs_review'?row.incomplete>0:filter==='unknown'?row.violations===null:row.status===filter;
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
    if (state.section === 'extractions') { const source=extractionSourceState(row);return filter==='present'?row.values.length>0:filter==='empty'?source.exactCount===0:filter==='error'?Boolean(row.error):filter==='truncated'?row.truncated===true:['incomplete','unknown','unavailable'].includes(filter)?source.status===filter:false; }
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

  function tableSchema() {
    const numeric=new Set(['status','initial_status','response_ms','elapsed_ms','link_score']);
    const required=state.section==='inputs'?['list_index','input_sha256','source_index']:state.section==='responses'?['url','requested_url','response_index']:state.section==='custom-search'?['url','rule_name','method','source']:['url'];
    return columns().map((column,index)=>{
      const type=column.type||(numeric.has(column.key)?'number':'text');
      const rawValue=row=>{
        if(row._unavailable && !required.includes(column.key))return null;
        if(type==='number')return column.value(row);
        if(['titles','title_text'].includes(column.key))return row.titles?.length?row.titles[0]:null;
        if(column.key==='description_text')return row.meta_descriptions?.length?row.meta_descriptions[0]:null;
        if(column.key==='indexable'&&row.indexable==null)return null;
        if(column.key==='raw_value')return row.raw_value;
        if(state.section==='images'&&column.key==='alt')return row.alt;
        if(state.section==='keywords'&&column.key==='content')return row.content;
        if(state.section==='headingorder'&&column.key==='text_with_image_alt')return row.text_with_image_alt;
        if(column.key==='dom_source')return row._page?.dom_metadata?.source??null;
        if(column.key==='dom_path')return row.dom_location?.path??null;
        if(state.section==='issues'&&column.key==='code')return row.code;
        if(column.key==='headers_known'&&row.headers==null)return null;
        if(column.key==='selection'||column.key==='validation')return null;
        if(column.key==='blocked'&&!row.blocked_by_robots&&!row.robots_warning&&effectiveStatus(row)==null)return null;
        if(column.key==='self_reference'&&!row.canonical?.length)return null;
        if(column.key==='dom_complete'&&!row._page?.dom_metadata)return null;
        if(column.key==='heading_text_engine'&&!row._page?.dom_metadata?.heading_text_engine)return null;
        if(Object.hasOwn(row,column.key)&&row[column.key]==null)return null;
        return column.value(row);
      };
      return {...column,type,rawValue,mandatory:index===0||required.includes(column.key)};
    });
  }
  function tableOwner() {
    return {projectId:state.crawl?.id??null,section:state.section,generation:state.generation,segment:state.segmentView?.context?.snapshot_token??null};
  }
  function tableIdentity(row) {
    return JSON.stringify([row.requested_url??row._page?.url??row.url??row.normalized_url??'',row.url??'',row.list_index??null,row.occurrence_index??null,row.source_index??null,row.response_index??null,row.dom_location?.index??null,row.rule_name??null,row.source??null,row._segmentOrdinal??row._page?._segmentOrdinal??null]);
  }
  function tableViewContract(version=1) {
    const semantics=version===2?window.TableViewLibrary.EXPRESSION_QUERY_SEMANTICS:window.TableViewLibrary.QUERY_SEMANTICS;
    return {section:state.section,query_semantics:state.section==='extractions'?`${semantics}/extraction-source-v1`:semantics,columns:tableSchema(),sectionFilters:filterOptions().map(option=>option[0])};
  }
  function tableViewSnapshot(version=1) {
    if(!state.crawl?.id||!tableExplorer)return {ok:false,error:{code:'no_crawl',message:'Zuerst einen Crawl und eine Ergebnistabelle auswählen.'}};
    if(tableExplorer.queryVersion()!==version)return {ok:false,error:{code:'inactive_query_mode',message:'Zuerst den passenden Filtermodus auswählen.'}};
    const draft=tableExplorer.draft();if(!draft.ok)return draft;
    return {ok:true,value:{sectionFilter:state.filter,search:state.search,...(version===2?{expression:draft.value.expression}:{rules:draft.value.rules}),sort:{key:state.sort,direction:state.direction},visibleKeys:draft.value.visibleKeys}};
  }
  function applyTableView(prepared,guard,version=1) {
    const stale=()=>({ok:false,error:{code:'stale_view',message:'Crawl, Suche oder Ansicht wurden inzwischen geändert. Die Ansicht erneut ausdrücklich anwenden.'}});
    if(typeof guard!=='function'||!guard()||!state.crawl?.id||!tableExplorer)return stale();
    if(tableExplorer.queryVersion()!==version)return stale();
    const contract=tableViewContract(version);
    if(prepared?.section!==contract.section||prepared?.query_semantics!==contract.query_semantics)return {ok:false,error:{code:'incompatible_view',message:'Die Ansicht passt nicht zur aktuellen Tabellenbedeutung.'}};
    const library=version===2?window.TableViewLibrary.createExpression(window.TableExplorer):window.TableViewLibrary.create(window.TableExplorer);
    const checked=library.prepareView(contract,{sectionFilter:prepared.sectionFilter,search:prepared.search,...(version===2?{expression:prepared.expression}:{rules:prepared.rules}),sort:prepared.sort,visibleKeys:prepared.visibleKeys});
    if(!checked.ok)return checked;if(!guard())return stale();
    // No await or externally visible partial render occurs between validation and commit.
    const installed=tableExplorer.installPrepared(checked.value);if(!installed.ok)return installed;
    state.search=checked.value.search;state.filter=checked.value.sectionFilter;state.sort=checked.value.sort.key;state.direction=checked.value.sort.direction;state.page=0;
    $('table-search').value=state.search;$('table-filter').value=state.filter;
    tableQueryEdited();
    renderTable();return {ok:true,value:checked.value};
  }
  function projectColumnsAvailable() {
    return Boolean(state.crawl && tableExplorer && !$('table-view').hidden && !active() && !actionBusy() && !state.pollToken && !dashboardSegments?.state.busy && tableExplorer.canExport());
  }
  function renderProjectColumns() {
    const controls=['project-columns-load','project-columns-save','project-columns-reset'];
    if(!projectColumns){for(const id of controls)$(id).disabled=true;return;}
    const current=projectColumns.state(),available=projectColumnsAvailable();
    $('project-columns-load').disabled=!available||current.busy;
    $('project-columns-save').disabled=!available||!projectColumns.canSave();
    $('project-columns-reset').disabled=!available||!projectColumns.canReset();
    $('project-columns-state').textContent=current.message;
    $('project-columns-state').dataset.phase=current.phase;
    $('project-columns-state').dataset.status=current.status;
    $('project-columns-state').dataset.projectId=current.projectId||'';
    $('project-columns-error').hidden=!current.error;
    $('project-columns-error').textContent=current.error?.message||'';
    if(current.busy){$('project-columns-reset-question').hidden=true;projectColumnsResetOwner=null;}
  }
  function invalidateProjectColumns() {
    $('project-columns-reset-question').hidden=true;projectColumnsResetOwner=null;
    projectColumns?.invalidate();
  }
  function bindProjectColumns() {
    if(!window.ProjectColumns){$('project-columns-error').hidden=false;$('project-columns-error').textContent='Projektspalten sind nicht geladen.';renderProjectColumns();return;}
    projectColumns=window.ProjectColumns.create({
      owner:()=>({...tableOwner(),nativeGeneration:state.crawl?.generation??null}),
      queryRevision:()=>tableQueryRevision,available:projectColumnsAvailable,
      schema:()=>tableSchema().map(({key,type,mandatory})=>({key,type,mandatory:Boolean(mandatory)})),
      visibleKeys:()=>tableExplorer.columns(tableSchema()).map(column=>column.key),
      fetch:(...args)=>fetch(...args),changed:renderProjectColumns,
      apply:(keys,guard)=>{
        if(!guard()||!projectColumnsAvailable())return{ok:false,error:{code:'stale_preferences',message:'Crawl oder Tabellenansicht wurden inzwischen geändert.'}};
        return tableExplorer.installColumns(keys,guard);
      }
    });
    $('project-columns-load').addEventListener('click',()=>projectColumns.load());
    $('project-columns-save').addEventListener('click',()=>projectColumns.save());
    $('project-columns-reset').addEventListener('click',()=>{
      if(!projectColumnsAvailable()||!projectColumns.canReset())return;
      projectColumnsResetOwner=JSON.stringify([tableOwner(),tableQueryRevision]);
      $('project-columns-reset-question').hidden=false;
    });
    $('project-columns-reset-cancel').addEventListener('click',()=>{$('project-columns-reset-question').hidden=true;projectColumnsResetOwner=null;});
    $('project-columns-reset-confirm').addEventListener('click',()=>{
      const current=projectColumnsResetOwner===JSON.stringify([tableOwner(),tableQueryRevision]);
      $('project-columns-reset-question').hidden=true;projectColumnsResetOwner=null;
      if(current&&projectColumnsAvailable()&&projectColumns.canReset())projectColumns.reset();
    });
    renderProjectColumns();
  }

  function bindNamedTableViews() {
    if(!window.TableViewsUI||!window.TableViewLibrary||!window.TableViewStorage||!window.TableViewPortability||!tableExplorer){$('table-view-error').hidden=false;$('table-view-error').textContent='Benannte Tabellenansichten sind nicht geladen.';return;}
    const common={owner:tableOwner,queryRevision:()=>tableQueryRevision,download:downloadBlob};
    tableViews=window.TableViewsUI.create({...common,catalogVersion:1,portability:window.TableViewPortability.create(1),available:()=>tableExplorer.queryVersion()===1,contract:()=>tableViewContract(1),snapshot:()=>tableViewSnapshot(1),apply:(prepared,guard)=>applyTableView(prepared,guard,1)});
    const library=window.TableViewLibrary.createExpression(window.TableExplorer);
    let storage=null,locks=null;try{storage=window.localStorage;}catch(_){}try{locks=window.navigator?.locks;}catch(_){}
    tableExpressionViews=window.TableViewsUI.create({...common,catalogVersion:2,portability:window.TableViewPortability.create(2),idPrefix:'table-expression-view',storageKey:window.TableViewStorage.EXPRESSION_KEY,library,store:window.TableViewStorage.createExpression({library,storage,locks}),available:()=>tableExplorer.queryVersion()===2,contract:()=>tableViewContract(2),snapshot:()=>tableViewSnapshot(2),apply:(prepared,guard)=>applyTableView(prepared,guard,2)});
  }
  function bindTableExplorer() {
    if(!window.TableExplorer||!window.TableExplorerUI){$('table-options').open=true;$('table-explorer-error').hidden=false;$('table-explorer-error').textContent='Erweiterte Tabellenfunktionen sind nicht geladen.';return;}
    tableExplorer=window.TableExplorerUI.create({owner:tableOwner,identity:{identity:tableIdentity},sort:()=>({key:state.sort,direction:state.direction}),setSort:sort=>{state.sort=sort.key;state.direction=sort.direction;},queryEdited:tableQueryEdited,changed:()=>{state.page=0;renderTable();}});
  }
  function filteredRows(useExplorer=true) {
    if(tableExplorer&&useExplorer){
      const schema=tableSchema();if(!schema.some(column=>column.key===state.sort))state.sort=schema[0].key;
      return tableExplorer.rows(getRows().filter(matchesFilter),schema,state.search,{key:state.sort,direction:state.direction});
    }
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
    const tableSequence=++tableRenderSequence,owner=JSON.stringify(tableOwner());
    const currentTable=()=>tableSequence===tableRenderSequence&&owner===JSON.stringify(tableOwner());
    const currentColumns = tableExplorer?tableExplorer.columns(tableSchema()):columns();
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
    const isSelected = row => state.segmentView && Number.isSafeInteger((row._page||row)._segmentOrdinal) && state.selected ? (row._page||row)._segmentOrdinal===state.selected._segmentOrdinal : state.section === 'inputs' ? state.selectedInput && row.list_index === state.selectedInput.list_index && row.occurrence_index === state.selectedInput.occurrence_index : state.section === 'responses' ? state.selectedResponse && state.selectedResponse.requested_url === row.requested_url && state.selectedResponse.response_index === row.response_index : state.selected && (row._page?.url || row.url) === state.selected.url;
    const emptyNote = state.section === 'responses' ? state.responsesError || (state.responses == null ? 'Diese Beobachtungen werden nach Abschluss oder Stopp des Crawls geladen. Nicht erfasste Antworten bleiben unbekannt.' : 'Es wurden keine nativen HTTP-Antworten erfasst, beispielsweise bei DNS-Fehlern oder Robots-Sperren.') : state.section === 'fragments' ? state.fragmentsError || (!state.fragments ? 'Fragmentbeobachtungen werden nach Abschluss oder Stopp geladen.' : !state.fragments.enabled ? 'Fragmentprüfung ist für diesen Crawl deaktiviert. Aktiviere sie in den Einstellungen für einen neuen Crawl.' : 'Keine Fragmentreferenzen im erfassten Inventar. Crawl- und Capture-Grenzen gelten weiterhin.') : null;
    const table = $('results-table');
    const inputEmptyNote = state.section === 'custom-search' ? state.crawl?.config?.custom_search ? 'Keine gespeicherten Suchbeobachtungen für diese Seiten. Die fehlende Aufzeichnung bleibt unbekannt, ohne bestätigte Null-Treffer.' : 'Eigene Suche war nicht aktiviert oder ist historisch nicht erfasst. Regeln in den Einstellungen für einen neuen Crawl aktivieren; keine nachträglichen Null-Treffer abgeleitet.' : state.section === 'inputs' ? array(state.crawl?.config?.input_imports).length ? 'Die gespeicherten Importergebnisse enthalten keine Eingabezeilen.' : 'Eingabeherkunft nicht erfasst. Bei älteren Projekten bleibt sie unbekannt; dies ist kein Nachweis einer leeren Eingabe.' : null;
    table.querySelector('thead').innerHTML = `<tr>${currentColumns.map((column) => `<th scope="col" aria-sort="${state.sort === column.key ? state.direction > 0 ? 'ascending' : 'descending' : 'none'}"><button type="button" data-sort="${escape(column.key)}">${escape(column.label)}<span aria-hidden="true">${state.sort === column.key ? state.direction > 0 ? '↑' : '↓' : '↕'}</span></button></th>`).join('')}</tr>`;
    table.querySelector('tbody').innerHTML = visible.length ? visible.map((row, index) => `<tr class="click-row${isSelected(row) ? ' selected' : ''}" data-row="${index}" tabindex="0" aria-label="Details zu ${escape(row.url)} öffnen">${currentColumns.map((column, columnIndex) => `<td>${cell(column, row, columnIndex)}</td>`).join('')}</tr>`).join('') : `<tr><td class="empty-cell" colspan="${currentColumns.length}"><strong>${state.search || state.filter !== 'all' || tableExplorer?.hasRules() ? 'Keine passenden Ergebnisse' : state.crawl ? 'Noch keine Daten in diesem Bereich' : 'Starte einen Crawl, um Ergebnisse zu sehen'}</strong>${escape(state.search || state.filter !== 'all' || tableExplorer?.hasRules() ? 'Ändere den Filter oder den Suchbegriff.' : inputEmptyNote || emptyNote || (['linkmetrics', 'structured'].includes(state.section) ? state.analysisError || 'Diese Auswertung wird nach Abschluss oder Stopp des Crawls berechnet.' : state.section === 'images' ? 'Bildreferenzen erscheinen nach der Auswertung von HTML-Seiten.' : state.section === 'external' ? 'Externe Links erscheinen, sobald sie im HTML entdeckt werden.' : 'Nur tatsächlich erfasste Daten werden angezeigt.'))}</td></tr>`;
    table.querySelectorAll('[data-recrawl-url]').forEach(input => {
      input.addEventListener('click', event => event.stopPropagation());
      input.addEventListener('keydown', event => event.stopPropagation());
      input.addEventListener('change', () => {if(currentTable())setRecrawlSelected(input.dataset.recrawlUrl, input.checked);});
    });
    table.querySelectorAll('[data-sort]').forEach((button) => button.addEventListener('click', () => {
      if(!currentTable())return;
      tableQueryEdited();
      state.direction = state.sort === button.dataset.sort ? -state.direction : 1;
      state.sort = button.dataset.sort;
      state.page = 0;
      renderTable();
    }));
    table.querySelectorAll('[data-row]').forEach((rowElement) => {
      const open = () => {
        if(!currentTable())return;
        const row = visible[Number(rowElement.dataset.row)];
        if (state.section === 'inputs') { state.selected = null; state.selectedResponse = null; state.selectedInput = row; renderInspector(); renderTable(); $('inspector').scrollIntoView({ behavior: 'smooth', block: 'nearest' }); return; }
        if (state.section === 'responses') { selectResponse(row); return; }
        if (state.section === 'fragments' && !row._page) { message('Kein gespeicherter Dokumentdatensatz zur Quell-URL verfügbar.', true); return; }
        const page = row._page || (Number.isSafeInteger(row._segmentOrdinal)?row:state.pageMap.get(row.url)) || row;
        selectPage(page);
        if (state.section === 'custom-search') { state.inspectorTab = 'custom-search'; renderInspector(); }
        if (state.section === 'accessibility') { state.inspectorTab = 'accessibility'; renderInspector(); }
        if (domSections.includes(state.section) || ['linkdom', 'fragments'].includes(state.section)) { state.inspectorTab = 'dom'; renderInspector(); }
      };
      rowElement.addEventListener('click', open);
      rowElement.addEventListener('keydown', (event) => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); open(); } });
    });
    $('result-count').textContent = `${number(rows.length)} Ergebnisse`;
    $('page-label').textContent = rows.length ? `${number(offset + 1)}–${number(Math.min(offset + state.pageSize, rows.length))} von ${number(rows.length)}` : 'Keine Ergebnisse';
    $('page-prev').disabled = state.page === 0;
    $('page-next').disabled = state.page + 1 >= pages;
    tableExplorer?.render();
    $('table-views').hidden=tableExplorer?.queryVersion()===2;
    $('table-expression-views').hidden=tableExplorer?.queryVersion()!==2;
    tableViews?.render();
    tableExpressionViews?.render();
    renderControls();
  }

  function selectSection(section) {
    if (!sections[section]) return;
    if (resourceSection() && section !== 'resource-discovery') invalidateResourceRequest();
    if (sitemapSection() && section !== 'sitemap-membership') invalidateSitemapRequest();
    if (findingsSection() && section !== 'sitemap-findings') resetSitemapFindings();
    state.section = section;
    state.page = 0;
    state.filter = 'all';
    state.search = '';
    tableQueryEdited();
    state.sort = section === 'inputs' ? 'list_index' : 'url';
    state.direction = 1;
    tableExplorer?.resetOwner();
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
    if (sitemapSection() && state.crawl && !sitemapMembership.result && !sitemapMembership.token) loadSitemapPage();
    if (findingsSection() && state.crawl && !sitemapFindings.result && !sitemapFindings.token && !sitemapFindings.queryDirty) loadSitemapFindings();
  }

  function renderSection() {
    const [eyebrow, title, description] = sections[state.section];
    renderDashboardSegments();
    $('section-eyebrow').textContent = eyebrow;
    $('section-heading').textContent = title;
    $('section-description').textContent = description;
    $('overview-view').hidden = state.section !== 'overview';
    $('graph-view').hidden = state.section !== 'graph';
    $('policy-view').hidden = !policySection();
    $('table-view').hidden = ['overview', 'graph', 'content','resource-discovery','sitemap-membership','sitemap-findings'].includes(state.section) || policySection();
    $('content-view').hidden = state.section !== 'content';
    $('resource-discovery-view').hidden = state.section !== 'resource-discovery';
    $('sitemap-membership-view').hidden = !sitemapSection();
    $('sitemap-findings-view').hidden = !findingsSection();
    const sourceAvailability=tableSourceAvailability();
    $('export-csv').disabled = policySection() ? !policyVisibleRows().length || Boolean(state.policyToken || state.policyError) : !sourceAvailability.ok || (!state.pages.length && !state.issues.length && !sourceAvailability.captured) || state.section === 'responses' && state.responses == null || state.section === 'fragments' && (!state.fragments || !state.fragments.enabled);
    $('export-csv').title = sourceAvailability.ok?tableCSVExportTitle():sourceAvailability.error.message;
    if (state.section==='resource-discovery' || sitemapSection() || findingsSection()) $('export-csv').disabled=true;
    $('export-resources').hidden = !['accessibility', 'custom-search', 'inputs', 'pdf', 'imagefiles', 'responses', 'fragments', 'linkdom', ...domSections].includes(state.section);
    $('export-resources').disabled = !state.crawl || Boolean(actionBusy() || active());
    $('export-resources').textContent = state.section === 'accessibility' ? '↓ Alle gespeicherten Accessibility-Befunde CSV' : state.section === 'custom-search' ? '↓ Alle gespeicherten Suchbefunde CSV' : state.section === 'inputs' ? '↓ Alle Eingabezeilen CSV' : domSections.includes(state.section) ? '↓ Alle DOM-Beobachtungen CSV' : state.section === 'pdf' ? '↓ PDF-Details CSV' : state.section === 'responses' ? '↓ Alle Antworten CSV' : state.section === 'fragments' ? '↓ Alle Fragmentziele CSV' : state.section === 'linkdom' ? '↓ Alle Linkvorkommen CSV' : '↓ Bilddateien CSV';
    if (state.section === 'overview') renderOverview();
    else if (state.section === 'graph') renderGraph();
    else if (policySection()) renderPolicyLedger();
    else if (state.section === 'content') renderContent();
    else if (state.section === 'resource-discovery') renderResourceLedger();
    else if (sitemapSection()) renderSitemapMembership();
    else if (findingsSection()) renderSitemapFindings();
    else renderTable();
    if(dashboardSegments?.state.busy){$('export-csv').disabled=true;$('export-resources').disabled=true;$('export-visible-report').disabled=true;}
    renderTableExport();
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

  function screenshotOwner() {
    const view=screenshotView;
    if(!view||!view.details.open||view.details.isConnected===false||$('inspector').hidden||state.selected!==view.page||state.selectedInput||state.selectedResponse||state.inspectorTab!=='details'||!state.crawl)return null;
    return {id:state.crawl.id,url:view.page.url,ordinal:Number.isSafeInteger(view.page._segmentOrdinal)?view.page._segmentOrdinal:null,nativeGeneration:state.crawl.generation??null,uiGeneration:state.generation,selection:view.page,view,segmentRevision:dashboardSegments?.state.revision??null,tab:state.inspectorTab,section:state.section};
  }
  function clearScreenshotPreview(view=screenshotView) {
    if(!view)return;
    view.previewToken=null;view.previewKey=null;view.previewOwner=null;view.previewError='';
    view.image?.remove();view.image=null;
    if(view.previewURL){URL.revokeObjectURL(view.previewURL);view.previewURL=null;}
    view.details.dataset.previewStatus='idle';
  }
  function renderScreenshotPreview(view,metadata) {
    const tools=window.SpiderScreenshots,owner=screenshotOwner();
    if(!tools||!owner||!metadata?.available){clearScreenshotPreview(view);view.details.dataset.previewStatus=metadata?'unavailable':'idle';return;}
    const key=JSON.stringify(metadata);
    if(view.previewKey===key&&tools.sameOwner(view.previewOwner,owner))return;
    clearScreenshotPreview(view);
    const token={owner};view.previewKey=key;view.previewOwner=owner;view.previewToken=token;view.details.dataset.previewStatus='checking';
    const current=()=>screenshotView===view&&view.previewToken===token&&tools.sameOwner(owner,screenshotOwner())&&screenshotController?.state.metadata?.available&&JSON.stringify(screenshotController.state.metadata)===key;
    tools.checkedPreview(view.page.screenshot_base64,metadata).then(bytes=>{
      if(!current())return;
      const url=URL.createObjectURL(new Blob([bytes],{type:'image/png'})),image=document.createElement('img');
      view.previewURL=url;view.image=image;image.src=url;image.alt=`Lokal erfasster, geprüfter Browser-Screenshot von ${view.page.url}`;image.dataset.screenshotPreview='';view.details.append(image);view.details.dataset.previewStatus='available';
    }).catch(error=>{
      if(!current())return;
      clearScreenshotPreview(view);view.previewKey=key;view.previewOwner=owner;view.previewError=typeof error?.message==='string'?error.message:'Die gespeicherte Vorschau konnte nicht geprüft werden.';view.details.dataset.previewStatus='error';renderScreenshotState();
    });
  }
  function renderScreenshotState() {
    const view=screenshotView;if(!view)return;
    const value=screenshotController?.state,metadata=value?.metadata;
    view.status.textContent=(value?.busy?(value.phase==='download'?'Gespeicherten PNG-Screenshot vollständig prüfen …':'Gespeicherte Screenshot-Metadaten prüfen …'):value?.error?value.error:metadata?metadata.available?(value.phase==='downloaded'?'PNG geprüft und heruntergeladen.':'PNG gespeichert; der Download prüft die Originaldatei vollständig.'):({disabled:'Screenshot war deaktiviert.',render_failed:'Browser-Rendering ist fehlgeschlagen.',missing:'Kein Screenshot aufgezeichnet; Verfügbarkeit unbekannt.',corrupt:'Gespeicherter Screenshot ist beschädigt oder widersprüchlich.'}[metadata.status]+' Befund: '+metadata.reason):'Noch keine nativen Screenshot-Metadaten geladen.')+(view.previewError?' Vorschau nicht verfügbar: '+view.previewError:'');
    view.status.dataset.status=metadata?.status||(value?.error?'error':'idle');
    view.status.classList.toggle('error',Boolean(value?.error||metadata?.status==='corrupt'));
    view.button.disabled=!screenshotController||Boolean(value?.busy)||Boolean(metadata&&!metadata.available);
    view.metadata.replaceChildren();
    if(metadata){
      const fields=[['Crawl-ID',metadata.id],['Originale Seitenbeobachtung (ab 0)',metadata.ordinal],['Anfrage-URL',metadata.url],['Finale URL',metadata.final_url],['Native Generation',metadata.generation??'Unbekannt'],['Snapshot SHA-256',metadata.snapshot_sha256],['Dateistatus',metadata.status],['Befund',metadata.reason??'Keine Abweichung'],['Gespeicherte Browser-Ansicht',metadata.viewport?`${metadata.viewport.width} × ${metadata.viewport.height}`:'Nicht gespeichert; historischer Worker-Standard nicht abgeleitet'],['PNG-Abmessungen',metadata.available?`${metadata.width} × ${metadata.height}`:'Nicht bestätigt'],['PNG-Bytes',metadata.bytes??'Nicht bestätigt'],['PNG SHA-256',metadata.png_sha256??'Nicht bestätigt']];
      for(const[label,value]of fields){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=label;dd.textContent=String(value);view.metadata.append(dt,dd);}
    }
    renderScreenshotPreview(view,metadata);
  }
  function renderScreenshotInspector(page,element) {
    if(!screenshotController&&window.SpiderScreenshots)screenshotController=window.SpiderScreenshots.createController({owner:screenshotOwner,changed:renderScreenshotState,download:downloadBlob});
    const details=document.createElement('details'),summary=document.createElement('summary'),status=document.createElement('p'),metadata=document.createElement('dl'),button=document.createElement('button');
    details.id='screenshot-details';details.className='rendered-screenshot';summary.textContent='Screenshot prüfen, anzeigen und herunterladen';status.id='screenshot-state';status.setAttribute('role','status');status.setAttribute('aria-live','polite');metadata.id='screenshot-metadata';metadata.className='details-grid';button.id='screenshot-export';button.type='button';button.className='button small';button.textContent='PNG prüfen und herunterladen';
    details.append(summary,status,metadata,button);
    screenshotView={page,details,status,metadata,button};
    details.addEventListener('toggle',()=>{
      if(screenshotView?.details!==details)return;
      if(!details.open){screenshotController?.invalidate();return;}
      screenshotController?.inspect();
    });
    button.addEventListener('click',()=>screenshotController?.download());element.append(details);renderScreenshotState();
  }

  function renderInspector() {
    screenshotController?.invalidate();
    screenshotView = null;
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
    if (state.inspectorTab === 'custom-search') { renderCustomSearchInspector(page, element); return; }
    if (state.inspectorTab === 'accessibility') { renderAccessibilityInspector(page, element); return; }
    if (state.inspectorTab === 'details') {
      const fields = [
        ...(Number.isSafeInteger(page._segmentOrdinal)?[['Originale Seitenbeobachtung (ab 0)',page._segmentOrdinal]]:[]),
        ...(observationAge(page.url) ? [['Alter des Befunds', observationAge(page.url)]] : []),
        ['HTTP initial / final', `${page.status ?? '—'} / ${effectiveStatus(page) ?? '—'}`],
        ['Finale URL', page.final_url || '—'], ['Content-Type', page.content_type || '—'],
        ['Antwortzeit', duration(page.response_ms)], ['Seitentitel', first(page.titles) || '—'],
        ['Meta Description', first(page.meta_descriptions) || '—'], ['H1', first(page.h1) || '—'],
        ['Pixelbreiten Titel / Description', `${JSON.stringify(page.title_pixel_widths || [])}\n${JSON.stringify(page.description_pixel_widths || [])}`],
        ['Text-Messprofil', page.text_metrics_engine || 'Keine Messung erfasst'],
        ['CDN-Verzeichniszuordnung', cdnDirectoryReportText(page.cdn_directory_reporting)],
        ['CDN-Klassifikation', page.cdn_classification ? page.cdn_classification.configured_origin + '\nDeklarierte Konfiguration · ' + page.cdn_classification.engine + '\nKonfigurations-SHA256 ' + page.cdn_classification.config_sha256 + '\nUrsprünglicher Seed-Bereich: ' + (page.cdn_classification.seed_scope_internal ? 'intern' : 'extern') + ' · keine zusätzliche Abruf-/Traversierungs-/Headerfreigabe' : 'Kein gespeicherter CDN-Zuordnungsbefund; alte Beobachtungen werden nicht nachträglich klassifiziert.'],
        ...htmlHeadDetailFields(page),
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
        ['HSTS-Transportregeln', page.tls?.hsts_upgrades?.length ? JSON.stringify(page.tls.hsts_upgrades, null, 2) : 'Keine gespeicherte lokale HSTS-Weiterleitung. HTTP-Status und Server-Weiterleitungen bleiben separat.'],
        ['Weiterleitungen', page.redirect_chain.map((entry) => `${entry.status} ${entry.url} → ${entry.target}`).join('\n') || '—'],
        ['Robots.txt', page.blocked_by_robots ? 'Abruf ausgeschlossen' : page.robots_warning || 'Kein gemeldeter Ausschluss'],
        ['Abruffehler', page.error || '—'], ['Statische URL-Zulassung', page.crawl_admission ? (page.crawl_admission.last.denied ? 'Verweigert: '+page.crawl_admission.last.denied : 'Geprüft') : 'Nicht aktiviert / Altbestand'], ['URL-Grenzen (Tiefe / Query / Bytes)', page.crawl_admission ? [page.crawl_admission.last.folder_depth,page.crawl_admission.last.query_tokens,page.crawl_admission.last.fetch_url_utf8_bytes].join(' / ') : '—'], ['Entdeckt auf', page.discovered_from || '—'],
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
        ['Eigene Extraktionen', page.custom_extractions.length ? page.custom_extractions.map(entry => {const s=extractionSourceState(entry),o=entry.source_observation;return `${entry.name || entry.selector} (${entry.method || 'css'}; ${entry.output || 'auto'}; ${entry.source || 'Quelle unbekannt'}): ${entry.error ? `Fehler: ${entry.error}` : array(entry.values).length ? array(entry.values).join(' | ') : '[keine erfassten Werte]'}\n${extractionSourceLabel(s.status)}; Auswertung ${s.evaluation}; Dekodierung ${s.decode}; Treffer im Input ${entry.matched_count ?? 'unbekannt'}; exakte Gesamtanzahl ${s.exactCount ?? 'unbekannt'}; Zählbereich ${s.scope}; Werte begrenzt ${entry.truncated ?? 'unbekannt'}\n${s.reasons.join(' | ')}\n${o ? JSON.stringify(o) : 'Keine gespeicherte Quellbeobachtung'}`;}).join('\n\n') : 'Keine Extraktionen konfiguriert oder erfasst'],
        ['Extraktionen vor Rendering', page.raw_metadata ? array(page.raw_metadata.custom_extractions).map(entry => `${entry.name || entry.selector}: ${array(entry.values).join(' | ') || '[keine erfassten Werte]'}; ${extractionSourceLabel(extractionSourceState(entry).status)}; exakte Gesamtanzahl ${extractionSourceState(entry).exactCount ?? 'unbekannt'}; ${entry.source_observation ? JSON.stringify(entry.source_observation) : 'Keine gespeicherte Quellbeobachtung'}`).join('\n') || '[keine Extraktionen]' : '—'],
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
      renderScreenshotInspector(page, element);
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
      element.innerHTML = page.links.length ? `<ul class="detail-links">${page.links.map((link) => `<li>${safeLink(linkURL(link))}${typeof link === 'object' ? `<small>${escape(link.text || '(ohne Linktext)')} · ${link.internal ? 'intern' : 'extern'}${link.nofollow ? ' · nofollow' : ''}${link.kind ? ` · ${escape(link.kind)}` : ''}${link.cdn_directory_reporting ? ` · ${escape(cdnDirectoryReportText(link.cdn_directory_reporting))}` : ''}</small>` : ''}</li>`).join('')}</ul>` : '<p class="detail-empty">Keine ausgehenden Links erfasst.</p>';
    } else if (state.inspectorTab === 'inlinks') {
      const sources = (state.segmentView && state.segmentBase ? state.segmentBase.pages.filter(page=>page.internal) : internals()).flatMap((source) => source.links.filter((link) => linkURL(link) === page.url || linkURL(link) === page.final_url).map((link) => ({ url: source.url, text: typeof link === 'object' ? link.text : '' })));
      element.innerHTML = (state.segmentView?'<p class="detail-empty">Eingehende Linkvorkommen aus dem profilsichtbaren Originalbestand, einschließlich Quellen außerhalb des Segments. Durch das Berichtsprofil ausgeblendete Quellen sind hier nicht enthalten; native Graphmetriken verwenden den vollständigen Capture.</p>':'') + (sources.length ? `<ul class="detail-links">${sources.map((source) => `<li>${safeLink(source.url)}<small>${escape(source.text || '(ohne Linktext)')}</small></li>`).join('')}</ul>` : '<p class="detail-empty">Keine eingehenden Links aus den bislang erfassten Seiten. Die Start-URL kann ohne interne Eingangslinks erscheinen.</p>');
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

  function renderAccessibilityInspector(page, element) {
    const observed=accessibilityObservation(page),raw=page.accessibility;
    element.replaceChildren(); element.dataset.accessibilityStatus=observed.status;
    const display=value=>value==null?'Unbekannt':typeof value==='string'?value:typeof value==='number'||typeof value==='boolean'?String(value):JSON.stringify(value).slice(0,8192);
    const make=(tag,text,klass)=>{const node=document.createElement(tag);if(text!=null)node.textContent=display(text);if(klass)node.className=klass;return node;};
    element.append(make('p','Nur automatisierte axe-Beobachtungen. Manuelle Prüfungen und vollständige WCAG-Konformität sind nicht nachgewiesen. Gespeicherte Daten sind keine unabhängig attestierte Prüfung.','accessibility-scope'));
    const fields=[['Beobachtung',accessibilityLabel(observed.status)],['Automated only',raw?.automated_only===true?'true':raw?.automated_only===false?'false · keine bestätigte automatisierte Beobachtung':'Unbekannt'],['Engine / Version',`${display(raw?.engine)} / ${display(raw?.version)}`],['Verstoßregeln',observed.violations],['Manuell zu prüfende Regeln',observed.incomplete],['Bestandene Regeln',observed.passes],['Nicht anwendbare Regeln',observed.inapplicable],['Browser / Zeitpunkt',`${page.browser_version??'Unbekannt'} / ${page.rendered_at??'Unbekannt'}`],['Gerenderte URL / Status',`${page.rendered_url??'Unbekannt'} / ${page.rendered_status??'Unbekannt'}`],['Fehler / Hinweis',raw?.error??page.renderer_error??observed.reason]];
    const dl=make('dl',null,'details-grid');for(const[label,value]of fields){dl.append(make('dt',label),make('dd',value??'Unbekannt'));}element.append(dl);
    if(observed.violations!==null){
      element.append(make('p','Verstöße und Prüfpunkte zählen Regeln, nicht Elemente. Die Ansicht zeigt bis zu 100 Regeln und 50 Elemente je Regel; vollständige gespeicherte Details stehen im nativen CSV und Projekt-JSON.','detail-empty'));
      for(const[kind,title]of [['violations','Automatisierte Verstöße'],['incomplete','Manuell prüfen / needs-review']]){
        const group=make('section',null,'accessibility-group');group.dataset.accessibilityKind=kind;group.append(make('h3',`${title} (${raw[kind].length})`));
        if(!raw[kind].length)group.append(make('p','0 beobachtete Regeln. Dies ist kein Nachweis vollständiger Barrierefreiheit.'));
        for(const rule of raw[kind].slice(0,100)){
          const card=make('article',null,'accessibility-rule');card.dataset.accessibilityRule=rule.id;
          card.append(make('h4',`${rule.id} · ${rule.impact??'Impact unbekannt'}`),make('p',rule.help),make('p',rule.description),make('p',`Tags: ${rule.tags.join(' · ')}`),make('p',`Regelhilfe: ${rule.helpUrl}`));
          for(const node of rule.nodes.slice(0,50)){
            const details=make('details',null,'accessibility-node');details.append(make('summary',`Element: ${JSON.stringify(node.target).slice(0,2048)}`));
            details.addEventListener('toggle',()=>{if(!details.open||details.dataset.loaded)return;details.dataset.loaded='true';details.append(make('h5','Gespeichertes HTML'),make('pre',node.html.slice(0,8192),'accessibility-evidence'));if(node.failureSummary)details.append(make('p',node.failureSummary.slice(0,8192)));
              for(const key of ['any','all','none']){details.append(make('h5',`Prüfungen: ${key}`));const list=make('ul');for(const check of node[key].slice(0,100))list.append(make('li',`${check.id} · ${check.impact??'Impact unbekannt'} · ${check.message??'Keine Nachricht gespeichert'}`));details.append(list);}
              const source=make('details'),summary=make('summary','Gespeicherte Element- und Checkdaten als JSON');source.append(summary,make('pre',JSON.stringify(node,null,2).slice(0,32768),'accessibility-evidence'));details.append(source);
            });card.append(details);
          }group.append(card);
        }element.append(group);
      }
    }else element.append(make('p','Alle abgeleiteten Zähler bleiben unbekannt. Fehlende oder fehlerhafte Daten sind kein bestätigter Nullbefund.','detail-empty'));
    const source=make('details');source.id='accessibility-json';source.append(make('summary','Gespeicherte axe-Beobachtung als JSON'));
    source.addEventListener('toggle',()=>{if(!source.open||source.dataset.loaded)return;source.dataset.loaded='true';source.append(make('pre',JSON.stringify(raw??null,null,2).slice(0,131072),'accessibility-evidence'));source.append(make('p','Gekürzte JSON-Ansicht; vollständige gespeicherte Daten im nativen Export.','detail-empty'));});element.append(source);
  }

  function renderCustomSearchInspector(page, element) {
    const results = array(page.custom_search);
    element.replaceChildren();
    const note = document.createElement('p'); note.className = 'detail-empty';
    note.textContent = results.length ? 'Gespeicherte Suchbefunde je Quelle. Bytepositionen beziehen sich auf die jeweilige UTF-8-Quelle, nicht auf Quellzeilen. Unvollständige Suche: Gesamtzahl unbekannt. Exakte Gesamtzahl und begrenzte Beispiele werden unabhängig ausgewiesen.' : 'Keine Suchaufzeichnung: deaktiviert, historisch unbekannt oder nicht erfasst. Dies ist kein bestätigtes Null-Ergebnis.';
    element.append(note);
    for (const result of results) {
      const card = document.createElement('article'); card.className = 'custom-search-result';
      card.dataset.searchRule = result.rule_name; card.dataset.searchSource = result.source;
      const title = document.createElement('h3'); title.textContent = `${result.rule_name} · ${searchSource(result.source)}`;
      const fields = [['Zustand',searchStatus(result.status)],['Exakte Gesamtanzahl',result.status==='complete'&&result.count!=null?result.count:'Unbekannt'],['Beobachtete Untergrenze',result.matches_observed],['Gespeicherte Beispiele',array(result.matches).length],['Beispiele vollständig',result.evidence_complete===true?'Ja':result.evidence_complete===false?'Nein':'Unbekannt'],['Methode',result.method],['Muster SHA-256',result.pattern_sha256],['Konfiguration SHA-256',result.config_sha256],['Eingabe-UTF-8-Bytes',result.input_utf8_bytes],['Durchsuchte UTF-8-Bytes',result.searched_utf8_bytes],['Eingabe SHA-256',result.input_sha256],['Dokument-URL',result.document_url],['Beobachtete Generation',result.observed_generation],['Quellrevision',result.source_revision],['Gründe',array(result.reasons).join(' | ')||'Keine gemeldet']];
      const dl = document.createElement('dl'); dl.className = 'details-grid';
      for (const [label,value] of fields) { const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=label;dd.textContent=String(value??'Nicht erfasst');dl.append(dt,dd); }
      card.append(title,dl);
      const evidence = document.createElement('details'),summary=document.createElement('summary');summary.textContent='Gespeicherte Treffer und Textausschnitte';evidence.append(summary);
      evidence.addEventListener('toggle',()=>{if(!evidence.open||evidence.dataset.loaded)return;evidence.dataset.loaded='true';const pre=document.createElement('pre');pre.className='custom-search-evidence';pre.textContent=JSON.stringify(array(result.matches).slice(0,200),null,2);evidence.append(pre);if(array(result.matches).length>200){const hint=document.createElement('p');hint.textContent='Oberfläche zeigt die ersten 200 gespeicherten Beispiele. Vollständige gespeicherte Daten stehen im JSON-/CSV-Export.';evidence.append(hint);}});
      card.append(evidence); element.append(card);
    }
    if (results.length) {
      const details=document.createElement('details'),summary=document.createElement('summary');details.id='custom-search-json';summary.textContent='Exakte gespeicherte Suchdaten als JSON';details.append(summary);
      details.addEventListener('toggle',()=>{if(details.open&&!details.querySelector('pre')){const pre=document.createElement('pre');pre.textContent=JSON.stringify(results,null,2);details.append(pre);}});element.append(details);
    }
  }

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

  function tableCSVExportTitle() {
    return policySection() ? 'Nur gefilterte Vorkommen der aktuell geladenen Ledger-Seite exportieren' : state.segmentView && state.section==='issues' ? 'Alle gefilterten Probleme mit den angezeigten Spalten; weitere URLs als Anzahl. Vollständige zugehörige URLs: native CSV-Art Probleme oder Segment-JSON.' : 'Alle aktuell gefilterten Ergebnisse dieses Bereichs exportieren';
  }

  function tableSourceAvailability() {
    const object=value=>Boolean(value&&typeof value==='object'&&!Array.isArray(value));
    const unavailable=message=>({ok:false,error:{code:'table_source_unavailable',message}});
    if(state.section==='linkmetrics'||state.section==='structured'){
      const label=state.section==='linkmetrics'?'Linkmetriken':'Strukturierte Daten';
      if(state.analysisError)return unavailable(`${label}: Die native Analyse ist nicht verfügbar. Analyse erneut laden, bevor diese Tabelle exportiert wird.`);
      const loaded=object(state.analysis)&&(state.section==='linkmetrics'?object(state.analysis.graph)&&Array.isArray(state.analysis.graph.nodes):Array.isArray(state.analysis.structured_data));
      return loaded?{ok:true,captured:true}:unavailable(`${label}: Die native Analyse ist nicht geladen oder unvollständig. Ein leerer Export wäre kein bestätigter Befund.`);
    }
    if(state.section==='extractions'&&!array(state.crawl?.config?.custom_extractions).length&&!extractionRows().length)return unavailable('Eigene Extraktionen nicht erfasst: Für diesen Lauf sind keine Regeln oder gespeicherten Beobachtungen verfügbar.');
    if(state.section==='inputs'){
      const imports=state.crawl?.config?.input_imports;
      if(!Array.isArray(imports)||!imports.length)return unavailable('Eingabeherkunft nicht erfasst: Für diesen Lauf ist keine erfasste Importquelle verfügbar.');
      // Native validate_import_result owns row integrity. Check captured envelopes
      // in O(imports), without scanning potentially large row inventories per render.
      for(const result of imports){
        if(!object(result)||result.engine!=='open-seo-spider/offline-list-v1/calamine-0.36.1'||typeof result.input_sha256!=='string'||!/^[a-f0-9]{64}$/i.test(result.input_sha256)||!Number.isSafeInteger(result.input_bytes)||result.input_bytes<=0||result.website_requests!==0||!Array.isArray(result.rows)||!result.rows.length)return unavailable('Eingabeherkunft unvollständig: Die erfasste Importquelle ist nicht verfügbar oder hat kein gültiges natives Herkunftsformat.');
      }
      return{ok:true,captured:true};
    }
    // A fully installed native cohort can genuinely contain no selected pages
    // or issues. Its bound inventory, rather than row count, permits export.
    const view=state.segmentView,context=view?.context,summary=view?.cohort?.summary;
    const boundSegment=!separateSegmentSections.has(state.section)&&object(view)&&object(context)&&object(summary)&&Boolean(state.crawl)&&context.job_id===state.crawl.id&&Number.isSafeInteger(context.app_generation)&&context.app_generation>=0&&context.app_generation===state.crawl.generation&&['snapshot_token','rules_sha256','typed_report_sha256'].every(key=>typeof context[key]==='string'&&/^[a-f0-9]{64}$/.test(context[key]))&&Array.isArray(view.pages)&&Array.isArray(view.issues)&&summary.selected_pages===view.pages.length&&summary.selected_issues===view.issues.length&&Number.isSafeInteger(summary.total_pages)&&summary.total_pages>=summary.selected_pages&&state.pages.length===view.pages.length&&state.issues.length===view.issues.length;
    return{ok:true,captured:boundSegment};
  }

  function captureTableProjection(typed=false) {
    const sourceAvailability=tableSourceAvailability();
    if(!sourceAvailability.ok)return sourceAvailability;
    const original = state.section;
    const useExplorer=!['overview','graph'].includes(original);
    if(useExplorer&&tableExplorer&&!tableExplorer.canExport())return{ok:false,error:{code:'query',message:'Die Tabellenansicht enthält einen Fehler. Vor dem Export die angezeigte Meldung beheben.'}};
    try {
    if (['overview', 'graph'].includes(original)) state.section = 'internal';
    const cols = useExplorer&&tableExplorer?tableExplorer.columns(tableSchema()).slice():columns();
    const rows = filteredRows(useExplorer);
    if(useExplorer&&tableExplorer&&!tableExplorer.canExport())return{ok:false,error:{code:'query',message:'Die Tabellenansicht konnte nicht vollständig ausgewertet werden; kein Export erzeugt.'}};
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
    const segment=separateSegmentSections.has(state.section)?null:state.segmentView;
    if(segment)for(const[key,label,value]of [['token','Segment Snapshot Token',()=>segment.context.snapshot_token],['rules','Segment Rules SHA256',()=>segment.context.rules_sha256],['report','Segment Typed Report SHA256',()=>segment.context.typed_report_sha256],['generation','Segment Generation',()=>segment.context.app_generation],['selection','Segment Selection',()=>JSON.stringify(segment.selection)],['reporting','Segment Reporting Identity',()=>JSON.stringify(segment.context.reporting_view??null)],['ordinal','Original Page Ordinal',row=>['issues','linkmetrics','structured'].includes(state.section)?'':(row._page||row)._segmentOrdinal??'']])cols.push(textColumn('segment_'+key,label,value));
    const visibility=state.crawl?.reporting_visibility;
    if(visibility)for(const [key,label] of [['captured_pages','Captured Pages'],['visible_pages','Visible Pages'],['hidden_pages','Hidden Pages']])cols.push(textColumn('reporting_'+key,label,()=>visibility[key]));
    // Resolve getters while their original table context is still current.
    // CSV retains its existing display values; XLSX preserves known raw scalars.
    const values=rows.map(row=>cols.map(column=>{
      const value=(typed?(column.exportValue||column.rawValue||column.value):column.value)(row);
      return typed&&value==null?null:value;
    }));
    return{ok:true,value:{section:state.section,columns:cols.map(({key,label})=>({key,label})),rows:values}};
    } catch { return{ok:false,error:{code:'snapshot',message:'Die vollständige Tabellenprojektion konnte nicht gelesen werden.'}}; }
    finally { state.section=original; }
  }

  function exportCSV() {
    if(dashboardSegments?.state.busy)return;
    if (policySection()) { exportPolicyPage(); return; }
    if (contentSection()) { exportContentRows(); return; }
    if (state.section==='resource-discovery' || sitemapSection() || findingsSection()) return;
    const captured=captureTableProjection();
    if(!captured.ok){message(captured.error.message,true);return;}
    const {columns:cols,rows,section}=captured.value;
    const csvCell = (input) => { let value = String(input ?? ''); if (typeof input !== 'number' && typeof input !== 'boolean' && /^[\s\uFEFF]*[=+@-]/u.test(value)) value = `'${value}`; return `"${value.replace(/"/g, '""')}"`; };
    const data = [cols.map((column) => csvCell(column.label)).join(','), ...rows.map((row) => row.map(csvCell).join(','))].join('\r\n');
    downloadBlob(new Blob(['\uFEFF', data], { type: 'text/csv;charset=utf-8' }), `open-seo-spider-${section}-${state.crawl?.id || 'export'}.csv`);
    message(`${number(rows.length)} gefilterte Datensätze als CSV exportiert.`);
  }

  function tableExportAvailable() {
    return Boolean(state.crawl && tableSourceAvailability().ok && !$('table-view').hidden && !$('export-csv').disabled && !dashboardSegments?.state.busy && tableExplorer?.canExport());
  }

  function renderTableExport(view=tableExport?.state()) {
    const button=$('export-xlsx'),status=$('table-export-status'),cancel=$('table-export-cancel');
    if(!button||!status||!cancel)return;
    button.hidden=$('table-view').hidden;
    button.disabled=!tableExport||Boolean(view?.busy)||!tableExportAvailable();
    if(button.dataset.defaultTitle===undefined)button.dataset.defaultTitle=button.title;
    const sourceAvailability=tableSourceAvailability();
    button.title=sourceAvailability.ok?button.dataset.defaultTitle:sourceAvailability.error.message;
    cancel.hidden=!view?.busy;
    status.hidden=!view?.message;status.textContent=view?.message||'';
    status.dataset.phase=view?.phase||'idle';status.dataset.code=view?.error?.code||'';
    status.classList.toggle('error',Boolean(view?.error));
  }

  function bindTableExport() {
    const button=$('export-xlsx');
    if(!window.TableXlsx||!window.TableExportUI){
      renderTableExport({busy:false,phase:'refused',error:{code:'unavailable'},message:'XLSX-Export ist nicht geladen.'});return;
    }
    tableExport=window.TableExportUI.create({
      writer:window.TableXlsx.create(),
      available:tableExportAvailable,
      connected:()=>button.isConnected&&$('export-xlsx')===button,
      stamp:()=>JSON.stringify([tableOwner(),tableQueryRevision,tableRenderSequence,state.sort,state.direction,state.search,state.filter]),
      snapshot:()=>{
        const captured=captureTableProjection(true);if(!captured.ok)return captured;
        const {section,columns,rows}=captured.value;
        return{ok:true,value:{data:{columns,rows,context:[]},filename:`open-seo-spider-${section}-${state.crawl.id}.xlsx`}};
      },
      download:downloadBlob,changed:renderTableExport
    });
    button.addEventListener('click',()=>{void tableExport.run();});
    $('table-export-cancel').addEventListener('click',()=>tableExport.cancel());
    renderTableExport();
  }

  async function serverDownload(endpoint, filename) {
    if (!state.crawl) return;
    if(state.segmentView){
      if(endpoint==='report.json'){await dashboardSegments.download('json');return;}
      if(endpoint==='sitemap.xml'){message('Sitemap-Export benötigt die Originalansicht.',true);return;}
      const kind=({'dom.csv':'dom','custom-search.csv':'custom-search','pdf.csv':'pdf','responses.csv':'responses','fragments.csv':'fragments','links.csv':'links','images.csv':'images'})[endpoint] || (endpoint.startsWith('accessibility.csv')?'accessibility':null);
      if(kind){await dashboardSegments.download(kind);return;}
    }
    const owner={id:state.crawl.id,generation:state.generation,nativeGeneration:state.crawl.generation,segmentRevision:dashboardSegments?.state.revision};
    try {
      const response = await fetch(`/api/crawls/${encodeURIComponent(owner.id)}/${endpoint}`);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      if(endpoint==='custom-search.csv'||endpoint.startsWith('accessibility.csv')){
        const length=response.headers.get('content-length'),digest=response.headers.get('x-content-sha256');
        if(!/^(0|[1-9][0-9]*)$/.test(length??'')||!Number.isSafeInteger(Number(length))||Number(length)>67108864||!/^[a-f0-9]{64}$/.test(digest??'')||response.headers.get('cache-control')!=='no-store'||response.headers.get('content-type')?.split(';',1)[0].trim()!=='text/csv')throw new Error('Suchbefund-CSV ohne bestätigte Länge, Prüfsumme oder Ausgabeformat.');
        const file=await readSitemapBundleStream(response,67108864,Number(length)),bytes=new Uint8Array(await file.blob.arrayBuffer());
        const actual=[...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(byte=>byte.toString(16).padStart(2,'0')).join('');
        if(actual!==digest)throw new Error('CSV: SHA-256 stimmt nicht überein.');
        if(endpoint.startsWith('accessibility.csv')&&response.headers.get('x-accessibility-generation')!==String(owner.nativeGeneration))throw new Error('Accessibility-CSV: Generation stimmt nicht überein.');
        if(owner.id!==state.crawl?.id||owner.generation!==state.generation||owner.segmentRevision!==dashboardSegments?.state.revision)return;
        downloadBlob(new Blob([bytes],{type:'text/csv;charset=utf-8'}),filename);message('Gespeicherte Befunde CSV · Länge und SHA-256 bestätigt.');return;
      }
      const blob=await response.blob();
      if(owner.id!==state.crawl?.id||owner.generation!==state.generation||owner.segmentRevision!==dashboardSegments?.state.revision)return;
      downloadBlob(blob, filename);
    } catch (error) { if(owner.id!==state.crawl?.id||owner.generation!==state.generation||owner.segmentRevision!==dashboardSegments?.state.revision)return;message(`Export fehlgeschlagen: ${error.message}`, true); }
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
    tlsFileRevision++;
    $('tls-file-status').textContent='';
    $('tls-certificate-files').value='';
    invalidatePolicyPreview();
    const form = $('settings-form');
    for (const [key, value] of Object.entries(state.settings)) {
      const element = form.elements.namedItem(key);
      if (!element) continue;
      if (element.type === 'checkbox') element.checked = Boolean(value); else element.value = value;
    }
    toggleRendererSettings(state.settings.javascript_rendering);
    syncResourceContractSettings();
    syncSitemapSourceProcessingSettings();
    syncSitemapSourceArchiveSettings();
    syncCrawlAdmissionSettings();
    renderQuotaRuleControls();syncQuotaSettings();
    $('settings-error').hidden = true;
    syncDirectiveReportingProfileSource();
    $('directive-reporting-load-status').textContent = '';
    $('directive-reporting-load-status').setAttribute('role', 'status');
    $('settings-dialog').showModal();
    if (typeof configProfiles !== 'undefined' && configProfiles) {
      configProfiles.begin();
      if (configProfiles.state.working) projectConfigProfile(configProfiles.state.working);
      else { profileFormBaseline = profileFormSnapshot(); profileConstraints(false); }
      renderConfigProfile();
    }
  }

  async function saveSettings(event) {
    event.preventDefault();
    if (typeof configProfiles !== 'undefined' && configProfiles && (configProfiles.state.restoreError || configProfiles.state.restorePending || configProfiles.state.working || configProfiles.state.text.trim())) { await saveConfigProfile(); return; }
    const form = $('settings-form');
    const advancedText = { ...readTlsSettings(form), ...readDirectiveReportingSettings(form), ...readCdnDirectoryReportingSettings(form), ...readCdnClassificationSettings(form), ...readCrawlQuotaSettings(form), ...readCrawlAdmissionSettings(form), ...readSitemapSourceProcessingSettings(form), ...readSitemapSourceArchiveSettings(form), include_patterns_text: form.elements.include_patterns_text.value, exclude_patterns_text: form.elements.exclude_patterns_text.value, custom_extractions_json: form.elements.custom_extractions_json.value, custom_javascript_json: form.elements.custom_javascript_json.value, audit_thresholds_json: form.elements.audit_thresholds_json.value, pdf_analysis_json: form.elements.pdf_analysis_json.value, link_position_rules_json: form.elements.link_position_rules_json.value, validate_fragments: form.elements.validate_fragments.checked, crawl_policy_enabled: form.elements.crawl_policy_enabled.checked, crawl_policy_json: form.elements.crawl_policy_json.value, live_policy_limits_json: form.elements.live_policy_limits_json.value, follow_subdomains: form.elements.follow_subdomains.checked, check_external: form.elements.check_external.checked, javascript_rendering: form.elements.javascript_rendering.checked, content_capture_enabled:form.elements.content_capture_enabled.checked, content_capture_json:form.elements.content_capture_json.value,resource_discovery_enabled:form.elements.resource_discovery_enabled.checked,resource_discovery_json:form.elements.resource_discovery_json.value,resource_discovery_limits_json:form.elements.resource_discovery_limits_json.value };
    Object.assign(advancedText,{custom_search_enabled:form.elements.custom_search_enabled.checked,custom_search_json:form.elements.custom_search_json.value});
    Object.assign(advancedText, {renderer_viewport_enabled:form.elements.renderer_viewport_enabled.checked,renderer_viewport_width:Number(form.elements.renderer_viewport_width.value),renderer_viewport_height:Number(form.elements.renderer_viewport_height.value)});
    try { parseAdvanced(advancedText); validateRendererSettings({ javascript_rendering: form.elements.javascript_rendering.checked, renderer_endpoint: form.elements.renderer_endpoint.value.trim(), renderer_wait_ms: Number(form.elements.renderer_wait_ms.value), timeout_seconds: Number(form.elements.timeout_seconds.value), ...advancedText }); }
    catch (error) {
      if (form.elements.tls_enabled.checked) $('tls-settings').open = true;
      if (form.elements.sitemap_source_processing_enabled.checked || form.elements.sitemap_membership_limits_enabled.checked) $('sitemap-source-processing-settings').open = true;
      if (form.elements.crawl_admission_limits_enabled.checked) $('crawl-admission-settings').open = true;
      if (form.elements.crawl_quotas_enabled.checked) $('crawl-quota-settings').open = true;
      if (form.elements.sitemap_source_archive_enabled.checked) $('sitemap-source-archive-settings').open = true;
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
        Object.assign(state.settings,{renderer_viewport_enabled:saved.renderer_viewport_enabled===true,renderer_viewport_width:positive(saved.renderer_viewport_width,1280,4096),renderer_viewport_height:positive(saved.renderer_viewport_height,800,4096)});
        Object.assign(state.settings, { crawl_policy_enabled: saved.crawl_policy_enabled === true, crawl_policy_json: String(saved.crawl_policy_json || '{"scope":{"roots":[{"origin":"https://example.test","folders":["/"]}]}}'), live_policy_limits_json: String(saved.live_policy_limits_json || '{}') });
        Object.assign(state.settings, { content_capture_enabled:saved.content_capture_enabled === true, content_capture_json:String(saved.content_capture_json || '{"sources":["raw_http_html"],"include_selectors":["body"],"exclude_selectors":["script","style","template","noscript","nav","footer"]}') });
        Object.assign(state.settings,{resource_discovery_enabled:saved.resource_discovery_enabled===true,resource_discovery_json:String(saved.resource_discovery_json || '{}'),resource_discovery_limits_json:String(saved.resource_discovery_limits_json || '{}')});
        Object.assign(state.settings,{custom_search_enabled:saved.custom_search_enabled===true,custom_search_json:String(saved.custom_search_json||'{"version":1,"rules":[{"name":"Beispiel","method":"literal","pattern":"Suchbegriff","sources":["raw_html"]}]}')});
        Object.assign(state.settings,restoreTlsSettings(saved));
        Object.assign(state.settings,restoreCdnClassificationSettings(saved));
        Object.assign(state.settings,restoreCdnDirectoryReportingSettings(saved));
        Object.assign(state.settings,restoreDirectiveReportingSettings(saved));
        state.settings.crawl_quotas_enabled=saved.crawl_quotas_enabled===true;
        state.settings.crawl_quota_per_depth=String(saved.crawl_quota_per_depth??'');
        state.settings.crawl_quota_per_subdomain=String(saved.crawl_quota_per_subdomain??'');
        state.settings.crawl_quota_patterns_json=String(saved.crawl_quota_patterns_json??'[]');
        state.settings.crawl_admission_limits_enabled = saved.crawl_admission_limits_enabled === true;
        for(const field of crawlAdmissionFields)state.settings[field.setting]=String(saved[field.setting]??'');
        state.settings.sitemap_source_archive_enabled = saved.sitemap_source_archive_enabled === true;
        state.settings.sitemap_source_archive_post_http = saved.sitemap_source_archive_post_http !== false;
        state.settings.sitemap_source_archive_decoded_xml = saved.sitemap_source_archive_decoded_xml !== false;
        for(const field of sitemapArchiveFields)state.settings[field.setting]=String(saved[field.setting]??field.initial);
        state.settings.sitemap_source_processing_enabled = saved.sitemap_source_processing_enabled === true;
        state.settings.sitemap_membership_limits_enabled = saved.sitemap_membership_limits_enabled === true;
        for (const field of sitemapMembershipFields) state.settings[field.setting] = String(saved[field.setting] ?? field.initial);
        for (const field of sitemapSourceFields) state.settings[field.setting] = String(saved[field.setting] ?? field.initial);
      }
    } catch (_) { /* Invalid saved settings fall back to safe defaults. */ }
  }

  function profileFormSnapshot() {
    const result = Object.create(null);
    for (const element of $('settings-form').elements) if (element.name && element.type !== 'file' && !['submit','button'].includes(element.type)) result[element.name] = element.type === 'checkbox' ? element.checked : element.value;
    return result;
  }
  function profileToolbarSnapshot() { return JSON.stringify({mode:$('crawl-mode').value,seed:$('seed-url').value,list:$('url-list').value}); }
  function profileSourceSnapshot(envelope) {
    const fields=window.ConfigProfiles.fields(envelope.config_json);
    return JSON.stringify(['seed_urls','sitemap_urls','input_imports','explicit_sitemap_inputs'].map(key=>[key,fields.has(key),fields.get(key)]));
  }
  function profileConstraints(enabled) {
    const form = $('settings-form');
    const bounds = {concurrency:['1',enabled?'1024':'64'],timeout_seconds:['1',enabled?null:'300'],delay_ms:['0',enabled?null:'60000'],max_body_mb:[enabled?'0.00000095367431640625':'1',enabled?null:'100']};
    for (const [key,[min,max]] of Object.entries(bounds)) { const input=form.elements[key];input.min=min;if(max===null)input.removeAttribute('max');else input.max=max; }
    form.elements.max_body_mb.step = enabled ? 'any' : '1';
  }
  function projectConfigProfile(envelope, commit = false) {
    const tools=window.ConfigProfiles, effective=tools.fields(envelope.effective_config_json), settings={...legacySettingsDefaults};
    const raw=(key,fallback)=>effective.get(key)??fallback;
    const string=(key,fallback='')=>JSON.parse(raw(key,JSON.stringify(fallback)));
    for(const [ui,key] of [['concurrency','concurrency'],['timeout_seconds','request_timeout_secs'],['delay_ms','delay_ms']])settings[ui]=raw(key,'0');
    settings.max_pages=raw('max_pages','null')==='null'?'0':raw('max_pages','0');settings.max_depth=raw('max_depth','null')==='null'?'0':raw('max_depth','0');
    settings.max_body_mb=tools.toMiB(raw('max_body_bytes','5242880'));
    for(const key of ['user_agent','indexability_agent'])settings[key]=string(key,legacySettingsDefaults[key]);
    for(const key of ['obey_robots','check_external','validate_fragments'])settings[key]=raw(key,'false')==='true';
    settings.follow_subdomains=raw('include_subdomains','false')==='true';
    for(const [ui,key]of [['include_patterns_text','include_patterns'],['exclude_patterns_text','exclude_patterns']])settings[ui]=JSON.parse(raw(key,'[]')).join('\n');
    for(const [ui,key]of [['custom_extractions_json','custom_extractions'],['audit_thresholds_json','audit_thresholds'],['pdf_analysis_json','pdf_analysis'],['link_position_rules_json','link_position_rules']])settings[ui]=raw(key,legacySettingsDefaults[ui]);
    const renderer=raw('renderer','null');settings.javascript_rendering=renderer!=='null';
    if(settings.javascript_rendering){const r=tools.fields(renderer);settings.renderer_endpoint=JSON.parse(r.get('endpoint'));settings.renderer_wait_ms=r.get('wait_ms');settings.renderer_accessibility=r.get('accessibility')==='true';settings.renderer_screenshot=r.get('screenshot')==='true';settings.renderer_collect_cookies=r.get('collect_cookies')==='true';settings.custom_javascript_json=r.get('custom_javascript')||'[]';const viewport=r.get('viewport');settings.renderer_viewport_enabled=viewport!==undefined&&viewport!=='null';if(settings.renderer_viewport_enabled){const v=tools.fields(viewport);settings.renderer_viewport_width=v.get('width');settings.renderer_viewport_height=v.get('height');}}
    for(const [enabled,json,key,limits,limitsKey]of [['crawl_policy_enabled','crawl_policy_json','crawl_policy','live_policy_limits_json','live_policy_limits'],['content_capture_enabled','content_capture_json','content_capture'],['custom_search_enabled','custom_search_json','custom_search'],['resource_discovery_enabled','resource_discovery_json','resource_discovery','resource_discovery_limits_json','resource_discovery_limits']]){settings[enabled]=effective.has(key)&&raw(key)!=='null';if(settings[enabled])settings[json]=raw(key);if(limits)settings[limits]=raw(limitsKey,'{}');}
    for(const [enabled,key,definitions]of [['sitemap_source_processing_enabled','sitemap_source_processing',sitemapSourceFields],['sitemap_membership_limits_enabled','sitemap_membership_limits',sitemapMembershipFields],['crawl_admission_limits_enabled','crawl_admission_limits',crawlAdmissionFields],['sitemap_source_archive_enabled','sitemap_source_archive',sitemapArchiveFields]]){
      settings[enabled]=effective.has(key)&&raw(key)!=='null';
      if(settings[enabled]){const values=tools.fields(raw(key));for(const field of definitions)settings[field.setting]=values.get(field.native)??String(field.initial??'');if(key==='sitemap_source_archive'){settings.sitemap_source_archive_post_http=values.get('retain_post_http_entity')==='true';settings.sitemap_source_archive_decoded_xml=values.get('retain_decoded_xml')==='true';}}
    }
    settings.crawl_quotas_enabled=effective.has('crawl_quotas')&&raw('crawl_quotas')!=='null';
    if(settings.crawl_quotas_enabled){const q=tools.fields(raw('crawl_quotas'));settings.crawl_quota_per_depth=q.get('max_urls_per_depth')==='null'?'':q.get('max_urls_per_depth')??'';settings.crawl_quota_per_subdomain=q.get('max_urls_per_subdomain')==='null'?'':q.get('max_urls_per_subdomain')??'';settings.crawl_quota_patterns_json=q.get('url_pattern_quotas')||'[]';}
    settings.tls_enabled=effective.has('tls')&&raw('tls')!=='null';if(settings.tls_enabled){const t=tools.fields(raw('tls'));settings.tls_respect_hsts=t.get('respect_hsts')==='true';settings.tls_certificates_json=t.get('trusted_certificates')||'[]';}
    settings.cdn_classification_enabled=effective.has('cdn_classification')&&raw('cdn_classification')!=='null';if(settings.cdn_classification_enabled)settings.cdn_origins_text=JSON.parse(tools.fields(raw('cdn_classification')).get('origins')||'[]').join('\n');
    settings.cdn_directory_reporting_enabled=effective.has('cdn_directory_reporting')&&raw('cdn_directory_reporting')!=='null';if(settings.cdn_directory_reporting_enabled)settings.cdn_directory_rules_text=JSON.parse(tools.fields(raw('cdn_directory_reporting')).get('rules')||'[]').map(rule=>typeof rule==='string'?rule:rule.rule??rule.directory??'').join('\n');
    const directive=effective.has('directive_reporting')&&raw('directive_reporting')!=='null'?tools.fields(raw('directive_reporting')):new Map();for(const key of directiveReportingKeys)settings[key]=directive.get(key)==='true';
    const form=$('settings-form');for(const[key,value]of Object.entries(settings)){const element=form.elements.namedItem(key);if(!element)continue;if(element.type==='checkbox')element.checked=Boolean(value);else element.value=value;}
    profileConstraints(true);toggleRendererSettings(settings.javascript_rendering);syncResourceContractSettings();syncSitemapSourceProcessingSettings();syncSitemapSourceArchiveSettings();syncCrawlAdmissionSettings();renderQuotaRuleControls(JSON.parse(settings.crawl_quota_patterns_json));syncQuotaSettings();
    profileFormBaseline=profileFormSnapshot();
    profileQuotaBaseline=JSON.stringify(readQuotaRuleControls());
    if(commit)state.settings=settings;
    return settings;
  }
  function configProfilePatches() {
    if(!profileFormBaseline)return [];
    const form=$('settings-form'), current=profileFormSnapshot(), baseline=profileFormBaseline, patches=[];
    if(profileQuotaBaseline!==null){const rows=JSON.stringify(readQuotaRuleControls());if(rows!==profileQuotaBaseline)current.crawl_quota_patterns_json=rows;}
    const changed=key=>current[key]!==baseline[key], set=(path,value_json)=>patches.push({path,value_json});
    const declared=configProfiles.state.working?.config_json||'{}';
    const remove=path=>{if(window.ConfigProfiles.at(declared,path)!==undefined)patches.push({path,remove:true});};
    const integer=value=>{if(!/^\d+$/.test(String(value)))throw Error('Eine ganze nichtnegative Zahl ist erforderlich.');return String(value).replace(/^0+(?=\d)/,'');};
    for(const[ui,key]of [['concurrency','concurrency'],['timeout_seconds','request_timeout_secs'],['delay_ms','delay_ms'],['max_pages','max_pages'],['max_depth','max_depth']])if(changed(ui))set([key],['max_pages','max_depth'].includes(ui)&&integer(current[ui])==='0'?'null':integer(current[ui]));
    if(changed('max_body_mb'))set(['max_body_bytes'],window.ConfigProfiles.fromMiB(current.max_body_mb));
    for(const key of ['user_agent','indexability_agent'])if(changed(key))set([key],JSON.stringify(current[key].trim()));
    for(const[ui,key]of [['follow_subdomains','include_subdomains'],['obey_robots','obey_robots'],['check_external','check_external'],['validate_fragments','validate_fragments']])if(changed(ui))set([key],JSON.stringify(current[ui]));
    for(const[ui,key]of [['include_patterns_text','include_patterns'],['exclude_patterns_text','exclude_patterns']])if(changed(ui))set([key],JSON.stringify([...new Set(current[ui].split(/\r?\n/).map(value=>value.trim()).filter(Boolean))]));
    for(const[ui,key]of [['custom_extractions_json','custom_extractions'],['audit_thresholds_json','audit_thresholds'],['pdf_analysis_json','pdf_analysis'],['link_position_rules_json','link_position_rules']])if(changed(ui))set([key],current[ui]);
    const base=window.ConfigProfiles.fields(configProfiles.state.working?.effective_config_json||'{}');
    const exists=key=>base.has(key)&&base.get(key)!=='null';
    for(const[enabled,json,key,limits,limitsKey]of [['crawl_policy_enabled','crawl_policy_json','crawl_policy','live_policy_limits_json','live_policy_limits'],['content_capture_enabled','content_capture_json','content_capture'],['custom_search_enabled','custom_search_json','custom_search'],['resource_discovery_enabled','resource_discovery_json','resource_discovery','resource_discovery_limits_json','resource_discovery_limits']]){
      if(changed(enabled)){if(!current[enabled]){remove([key]);if(limitsKey)remove([limitsKey]);}else{set([key],current[json]);if(limitsKey)set([limitsKey],current[limits]);}}
      else if(current[enabled]){if(changed(json))set([key],current[json]);if(limits&&changed(limits))set([limitsKey],current[limits]);}
    }
    if(changed('javascript_rendering')){
      if(!current.javascript_rendering)set(['renderer'],'null');
      else set(['renderer'],'{"endpoint":'+JSON.stringify(current.renderer_endpoint.trim())+',"wait_ms":'+integer(current.renderer_wait_ms)+',"accessibility":'+JSON.stringify(current.renderer_accessibility)+',"screenshot":'+JSON.stringify(current.renderer_screenshot)+',"collect_cookies":'+JSON.stringify(current.renderer_collect_cookies)+',"custom_javascript":'+current.custom_javascript_json+(current.renderer_viewport_enabled?',"viewport":'+JSON.stringify(rendererViewport(current)):'')+'}');
    }else if(current.javascript_rendering){
      for(const[ui,key]of [['renderer_endpoint','endpoint'],['renderer_wait_ms','wait_ms'],['renderer_accessibility','accessibility'],['renderer_screenshot','screenshot'],['renderer_collect_cookies','collect_cookies'],['custom_javascript_json','custom_javascript']])if(changed(ui))set(['renderer',key],ui==='custom_javascript_json'?current[ui]:ui==='renderer_wait_ms'?integer(current[ui]):JSON.stringify(current[ui]));
      if(changed('renderer_viewport_enabled')){if(current.renderer_viewport_enabled)set(['renderer','viewport'],JSON.stringify(rendererViewport(current)));else remove(['renderer','viewport']);}
      else if(current.renderer_viewport_enabled){const viewport=rendererViewport(current);for(const axis of ['width','height'])if(changed('renderer_viewport_'+axis))set(['renderer','viewport',axis],String(viewport[axis]));}
    }
    const objectGroup=(enabled,key,keys,parser)=>{
      if(!keys.some(changed)&&!changed(enabled))return;
      if(!current[enabled]){remove([key]);return;}
      const parsed=parser(current)[key];
      if(!parsed){remove([key]);return;}
      if(changed(enabled)||!exists(key))set([key],JSON.stringify(parsed));
      return parsed;
    };
    for(const[enabled,key,definitions,parser]of [['sitemap_source_processing_enabled','sitemap_source_processing',sitemapSourceFields,parseSitemapSourceProcessing],['sitemap_membership_limits_enabled','sitemap_membership_limits',sitemapMembershipFields,parseSitemapMembershipLimits],['crawl_admission_limits_enabled','crawl_admission_limits',crawlAdmissionFields,parseCrawlAdmissionLimits],['sitemap_source_archive_enabled','sitemap_source_archive',sitemapArchiveFields,parseSitemapSourceArchive]]){
      const keys=definitions.map(field=>field.setting);if(key==='sitemap_source_archive')keys.push('sitemap_source_archive_post_http','sitemap_source_archive_decoded_xml');
      const parsed=objectGroup(enabled,key,keys,parser);if(!parsed||changed(enabled)||!exists(key))continue;
      for(const field of definitions)if(changed(field.setting)){if(Object.hasOwn(parsed,field.native))set([key,field.native],JSON.stringify(parsed[field.native]));else remove([key,field.native]);}
      if(key==='sitemap_source_archive')for(const[ui,native]of [['sitemap_source_archive_post_http','retain_post_http_entity'],['sitemap_source_archive_decoded_xml','retain_decoded_xml']])if(changed(ui))set([key,native],JSON.stringify(current[ui]));
    }
    if(['crawl_quotas_enabled',...crawlQuotaSettingKeys].some(changed)){const keys=crawlQuotaSettingKeys.filter(key=>key!=='crawl_quotas_enabled');const parsed=objectGroup('crawl_quotas_enabled','crawl_quotas',keys,parseCrawlQuotas);if(parsed&&!changed('crawl_quotas_enabled')&&exists('crawl_quotas'))for(const[ui,key]of [['crawl_quota_per_depth','max_urls_per_depth'],['crawl_quota_per_subdomain','max_urls_per_subdomain'],['crawl_quota_patterns_json','url_pattern_quotas']])if(changed(ui))set(['crawl_quotas',key],JSON.stringify(parsed[key]??null));}
    if(['tls_enabled','tls_respect_hsts','tls_certificates_json'].some(changed)){
      if(!current.tls_enabled)remove(['tls']);else if(changed('tls_enabled')||!exists('tls'))set(['tls'],'{"version":1,"respect_hsts":'+JSON.stringify(current.tls_respect_hsts)+',"trusted_certificates":'+current.tls_certificates_json+'}');else{if(changed('tls_respect_hsts'))set(['tls','respect_hsts'],JSON.stringify(current.tls_respect_hsts));if(changed('tls_certificates_json'))set(['tls','trusted_certificates'],current.tls_certificates_json);}
    }
    for(const[enabled,text,key,leaf,parser]of [['cdn_classification_enabled','cdn_origins_text','cdn_classification','origins',parseCdnClassification],['cdn_directory_reporting_enabled','cdn_directory_rules_text','cdn_directory_reporting','rules',parseCdnDirectoryReporting]])if(changed(enabled)||changed(text)){
      if(!current[enabled])remove([key]);else{const parsed=parser(current)[key];if(changed(enabled)||!exists(key))set([key],JSON.stringify(parsed));else set([key,leaf],JSON.stringify(parsed[leaf]));}
    }
    if(directiveReportingKeys.some(changed)){
      if(!exists('directive_reporting'))set(['directive_reporting'],JSON.stringify({version:1,...Object.fromEntries(directiveReportingKeys.map(key=>[key,current[key]]))}));
      else for(const key of directiveReportingKeys)if(changed(key))set(['directive_reporting',key],JSON.stringify(current[key]));
    }
    return patches;
  }
  function renderConfigProfile() {
    if(!configProfiles)return;
    const profile=configProfiles.state, envelope=profile.working||profile.active;
    const text=$('config-profile-json');if(text.value!==profile.text&&profile.open)text.value=profile.text;
    $('config-profile-effective').textContent=profile.working?.effective_config_json||'';
    const restoreError=profile.restoreError||profile.restorePending?configProfileStartRefusal():'',error=[restoreError,profile.error!==profile.restoreError?profile.error:''].filter(Boolean).join('\n');
    $('config-profile-restore-error').hidden=!restoreError;$('config-profile-restore-error').textContent=restoreError;
    $('config-profile-error').hidden=!error;$('config-profile-error').textContent=error;
    $('config-profile-status').textContent=profile.busy?'Profil wird ohne Crawl oder Website-Anfragen geprüft.':envelope?'Vollständiger nativer Entwurf. Formularänderungen werden gezielt auf den JSON-Entwurf angewendet. Erst Speichern übernimmt ihn.':'Noch kein vollständiges Profil geladen; bisherige Einstellungen bleiben aktiv.';
    for(const id of ['config-profile-export','config-profile-load-selected','config-profile-default','config-profile-inspect'])$(id).disabled=profile.busy||(id==='config-profile-load-selected'&&!state.crawl);
    $('settings-form').querySelector('button[type="submit"]').disabled=profile.busy||Boolean(configProfileCatalog?.state.busy);
    const names=envelope?.required_headers||[],signature=JSON.stringify(names);
    if(signature!==profileHeaderSignature){
      profileHeaderSignature=signature;const container=$('config-profile-required-headers');container.replaceChildren();
      for(const name of names){const row=document.createElement('div'),label=document.createElement('label'),caption=document.createElement('span'),input=document.createElement('input'),resolve=document.createElement('input'),resolved=document.createElement('label');caption.textContent=name+' – explizite Eingabe für den nächsten Start';input.type='password';input.autocomplete='off';input.dataset.profileHeader=name;input.setAttribute('aria-label','Headerwert '+name);resolve.type='checkbox';resolve.dataset.profileHeaderResolve=name;resolved.append(resolve,document.createTextNode('Diesen Wert verwenden (auch leer)'));label.append(caption,input);row.append(label,resolved);container.append(row);input.addEventListener('input',()=>{const value=input.value;configProfiles.invalidate();resolve.checked=true;configProfiles.header(name,value,true);});resolve.addEventListener('change',()=>{const value=input.value,present=resolve.checked;configProfiles.invalidate();configProfiles.header(name,value,present);});}
    }
    for(const input of $('config-profile-required-headers').querySelectorAll('[data-profile-header]')){const name=input.dataset.profileHeader;if(input.value!==configProfiles.headerValue(name))input.value=configProfiles.headerValue(name);const resolve=[...$('config-profile-required-headers').querySelectorAll('[data-profile-header-resolve]')].find(element=>element.dataset.profileHeaderResolve===name);resolve.checked=configProfiles.hasHeader(name);}
    $('config-profile-header-state').textContent=names.length?configProfiles.unresolved().length+' von '+names.length+' Headerwerten fehlen. Profilentwurf und Profilexport enthalten keine Headerwerte. Beim Start werden sie Teil der lokal gespeicherten Laufkonfiguration.':'Keine Headerwerte erforderlich.';
    if(profile.active){const values=window.ConfigProfiles.fields(profile.active.effective_config_json),seeds=JSON.parse(values.get('seed_urls')||'[]'),maps=JSON.parse(values.get('sitemap_urls')||'[]');$('config-profile-active').hidden=false;$('config-profile-active').textContent='Vollprofil aktiv: '+seeds.length+' Start-URLs, '+maps.length+' Sitemap-URLs. Unveränderte Startfelder behalten alle gespeicherten Quellen und Eingabeherkünfte. Änderungen der Startfelder ersetzen diese Quellen ausdrücklich; Linkverfolgung bleibt im Profil festgelegt.';}
    renderConfigProfileCatalog();
  }
  function renderConfigProfileCatalog() {
    if(!configProfileCatalog)return;
    const catalog=configProfileCatalog.state,entry=configProfileCatalog.selected(),busy=catalog.busy||configProfiles.state.busy,select=$('config-catalog-select');
    const options=[document.createElement('option')];options[0].value='';options[0].textContent=catalog.loaded?'Eintrag auswählen':'Zuerst gespeicherte Profile laden';
    for(const item of catalog.entries){const option=document.createElement('option');option.value=item.id;option.textContent=item.name+' · Revision '+item.revision;options.push(option);}select.replaceChildren(...options);select.value=catalog.selectedId;select.disabled=!catalog.entries.length;
    const owner=entry?entry.id+':'+entry.revision:'';if(owner!==profileCatalogNameOwner){profileCatalogNameOwner=owner;if(entry)$('config-catalog-name').value=entry.name;}
    $('config-catalog-status').textContent=catalog.busy?'Native Bibliotheksaktion läuft; der aktive Crawlentwurf bleibt unverändert.':catalog.status;
    $('config-catalog-error').hidden=!catalog.error;$('config-catalog-error').textContent=catalog.error;
    $('config-catalog-meta').textContent=entry?'Revision '+entry.revision+' · '+entry.profile_bytes+' UTF-8-Bytes · gespeichert '+entry.updated_at+' · SHA-256 '+entry.profile_sha256:'Keine gespeicherte Profilkopie ausgewählt.';
    $('config-catalog-list').disabled=busy;$('config-catalog-create').disabled=busy;
    for(const id of ['load','replace','rename','delete'])$('config-catalog-'+id).disabled=busy||!entry;
    const token=entry?entry.id+':'+entry.revision+':'+entry.profile_sha256:null,confirm=Boolean(token&&token===profileCatalogDelete);
    $('config-catalog-delete-question').hidden=!confirm;$('config-catalog-delete-question-text').textContent=confirm?'„'+entry.name+'“ aus der nativen Bibliothek löschen? Bereits geladene Entwürfe und Crawl-Konfigurationen bleiben erhalten.':'';
    $('config-catalog-delete-confirm').disabled=busy||!confirm;
  }
  function projectProfileToolbar(envelope) {
    const values=window.ConfigProfiles.fields(envelope.effective_config_json),seeds=JSON.parse(values.get('seed_urls')||'[]'),maps=JSON.parse(values.get('sitemap_urls')||'[]');
    $('crawl-mode').value=!seeds.length&&maps.length?'sitemap':'spider';$('seed-url').value=seeds[0]||maps[0]||'';
    $('list-control').hidden=true;$('seed-url').closest('.seed-control').hidden=false;$('seed-url').required=true;
    $('seed-url').labels[0].textContent=$('crawl-mode').value==='sitemap'?'Sitemap-URL':'Start-URL';
    profileToolbarBaseline=profileToolbarSnapshot();renderConfigProfile();
  }
  async function stageConfigProfile(work) {
    tlsFileRevision++;
    const result=await work;if(result){projectConfigProfile(result);renderConfigProfile();}return result;
  }
  function configProfileFromFormText() {
    const settings=profileFormSnapshot(), advanced=parseAdvanced(settings);
    const config={...advanced,concurrency:Number(settings.concurrency),request_timeout_secs:Number(settings.timeout_seconds),max_pages:Number(settings.max_pages)||null,max_depth:Number(settings.max_depth)||null,delay_ms:Number(settings.delay_ms),max_body_bytes:Number(settings.max_body_mb)*1048576,user_agent:settings.user_agent,indexability_agent:settings.indexability_agent,obey_robots:settings.obey_robots,include_subdomains:settings.follow_subdomains,check_external:settings.check_external};
    config.renderer=settings.javascript_rendering?{endpoint:settings.renderer_endpoint,wait_ms:Number(settings.renderer_wait_ms),accessibility:settings.renderer_accessibility,screenshot:settings.renderer_screenshot,...(rendererViewport(settings)?{viewport:rendererViewport(settings)}:{}),collect_cookies:settings.renderer_collect_cookies,custom_javascript:parseCustomJavascript(settings.custom_javascript_json)}:null;
    return JSON.stringify(config);
  }
  async function ensureConfigProfile() {
    if(configProfiles.state.working||configProfiles.state.text.trim())return true;
    return Boolean(await stageConfigProfile(configProfiles.fromConfig(configProfileFromFormText())));
  }
  function configProfileCatalogPreparation() { return configProfiles.state.working||configProfiles.state.text.trim()?{profile_text:configProfiles.state.text,patches:configProfilePatches()}:{config_text:configProfileFromFormText()}; }
  async function writeConfigProfileCatalog(replace) {
    profileCatalogDelete=null;
    try{const name=$('config-catalog-name').value,preparation=configProfileCatalogPreparation();await configProfileCatalog[replace?'overwrite':'create'](name,preparation);}
    catch(error){configProfileCatalog.state.error=error.message;renderConfigProfileCatalog();}
  }
  async function saveConfigProfile() {
    tlsFileRevision++;
    try{
      if(!await ensureConfigProfile())return;
      const previous=configProfiles.state.active,pendingToolbar=previous&&profileToolbarSnapshot()!==profileToolbarBaseline;
      const patches=configProfilePatches(),saved=await configProfiles.save(patches);if(!saved)return;
      projectConfigProfile(saved,true);
      if(!pendingToolbar||profileSourceSnapshot(previous)!==profileSourceSnapshot(saved))projectProfileToolbar(saved);
      $('settings-dialog').close();invalidateContentRequests('Vollständiges Konfigurationsprofil gespeichert.');renderControls();
    }catch(error){configProfiles.state.error=error.message;renderConfigProfile();}
  }
  async function startProfileCrawl() {
    const refusal=configProfileStartRefusal();if(refusal){message(refusal,true);return;}
    const operation={sequence:++state.operationSequence,generation:state.generation,id:state.crawl?.id||null};
    const current=()=>state.startOperation===operation&&state.generation===operation.generation&&(state.crawl?.id||null)===operation.id;
    state.startOperation=operation;state.starting=true;renderControls();
    try{
      const header_values=configProfiles.headerValues(),patches=[];
      if(profileToolbarSnapshot()!==profileToolbarBaseline){
        const mode=$('crawl-mode').value;let seeds=[],maps=[],imports=[];
        if(mode==='list'){
          let imported=listUI.controller.state.accepted;if(!imported||listUI.acceptedText!==$('url-list').value){listUI.controller.setEditedText($('url-list').value);imported=await listUI.controller.preview();if(!current())return;}
          if(!imported)throw Error('Die neue URL-Liste muss vollständig geprüft werden.');const summary=window.ListImportTools.summarizeRows(imported.rows);if(!summary.accepted)throw Error('Keine gültigen neuen Ziele.');seeds=summary.page_urls;maps=summary.sitemap_urls;imports=[imported];
        }else{const url=$('seed-url').value.trim();const parsed=new URL(url);if(!['http:','https:'].includes(parsed.protocol))throw Error('Eine HTTP(S)-Zieladresse ist erforderlich.');if(mode==='sitemap')maps=[url];else seeds=[url];}
        patches.push({path:['seed_urls'],value_json:JSON.stringify(seeds)},{path:['sitemap_urls'],value_json:JSON.stringify(maps)},{path:['input_imports'],value_json:JSON.stringify(imports)});
        if(window.ConfigProfiles.at(configProfiles.state.active.config_json,['explicit_sitemap_inputs'])!==undefined)patches.push({path:['explicit_sitemap_inputs'],remove:true});
      }
      if(!current())return;
      const result=await request('/api/crawls/from-profile',{method:'POST',body:JSON.stringify({profile_text:configProfiles.state.active.profile_json,patches,header_values})});const crawl=result.crawl||result;if(!crawl.id)throw Error('Keine Crawl-ID empfangen.');
      if(!current()){await loadHistory();return;}state.crawls.unshift(crawl);renderHistory();$('crawl-history').value=crawl.id;await selectCrawl(crawl.id);
    }catch(error){if(current())message('Der Profil-Crawl konnte nicht gestartet werden: '+error.message,true);}
    finally{if(state.startOperation===operation){state.startOperation=null;state.starting=false;renderControls();}}
  }
  function bindConfigProfileCatalog() {
    $('config-catalog-list').addEventListener('click',()=>{profileCatalogDelete=null;configProfileCatalog.list();});
    $('config-catalog-select').addEventListener('change',()=>{profileCatalogDelete=null;configProfileCatalog.select($('config-catalog-select').value);});
    $('config-catalog-name').addEventListener('input',()=>{profileCatalogDelete=null;configProfileCatalog.edited();});
    $('config-catalog-load').addEventListener('click',()=>{tlsFileRevision++;profileCatalogDelete=null;stageConfigProfile(configProfileCatalog.load());});
    $('config-catalog-create').addEventListener('click',()=>writeConfigProfileCatalog(false));$('config-catalog-replace').addEventListener('click',()=>writeConfigProfileCatalog(true));
    $('config-catalog-rename').addEventListener('click',()=>{profileCatalogDelete=null;configProfileCatalog.rename($('config-catalog-name').value);});
    $('config-catalog-delete').addEventListener('click',()=>{const entry=configProfileCatalog.selected();if(entry){configProfileCatalog.edited();profileCatalogDelete=entry.id+':'+entry.revision+':'+entry.profile_sha256;renderConfigProfileCatalog();}});
    $('config-catalog-delete-cancel').addEventListener('click',()=>{profileCatalogDelete=null;renderConfigProfileCatalog();});
    $('config-catalog-delete-confirm').addEventListener('click',()=>{const entry=configProfileCatalog.selected();if(entry&&profileCatalogDelete===entry.id+':'+entry.revision+':'+entry.profile_sha256){profileCatalogDelete=null;return configProfileCatalog.remove();}});
  }
  async function initializeConfigProfiles() {
    if(!window.ConfigProfiles)return;
    configProfiles=window.ConfigProfiles.createController({invalidated:()=>{profileCatalogDelete=null;configProfileCatalog?.invalidate();},changed:()=>{renderConfigProfile();renderControls();}});
    configProfileCatalog=window.ConfigProfiles.createCatalogController({profiles:configProfiles,changed:()=>{renderConfigProfile();renderControls();}});
    bindConfigProfileCatalog();
    $('config-profile-json').addEventListener('input',()=>{tlsFileRevision++;configProfiles.edited($('config-profile-json').value);});
    $('config-profile-import').addEventListener('click',()=>{configProfiles.edited();$('config-profile-file').value='';$('config-profile-file').click();});
    $('config-profile-file').addEventListener('change',()=>stageConfigProfile(configProfiles.importFile($('config-profile-file').files[0])));
    $('config-profile-inspect').addEventListener('click',async()=>{try{if(!await ensureConfigProfile())return;await stageConfigProfile(configProfiles.inspect(configProfilePatches()));}catch(error){configProfiles.state.error=error.message;renderConfigProfile();}});
    $('config-profile-default').addEventListener('click',()=>stageConfigProfile(configProfiles.defaults()));
    $('config-profile-load-selected').addEventListener('click',()=>{const id=state.crawl?.id,generation=state.generation,nativeGeneration=state.crawl?.generation;if(id)stageConfigProfile(configProfiles.selected(id,()=>$('settings-dialog').open&&state.crawl?.id===id&&state.generation===generation&&state.crawl?.generation===nativeGeneration));});
    $('config-profile-export').addEventListener('click',async()=>{try{if(!await ensureConfigProfile())return;const result=await configProfiles.exportProfile(configProfilePatches());if(result)downloadBlob(new Blob([result.bytes],{type:'application/json'}),'open-seo-spider-config-profile-v1.json');}catch(error){configProfiles.state.error=error.message;renderConfigProfile();}});
    $('settings-form').addEventListener('input',event=>{if(event.target.name)configProfiles.edited();});
    $('settings-form').addEventListener('change',event=>{if(event.target.name)configProfiles.edited();});
    $('crawl-quota-patterns').addEventListener('input',()=>configProfiles.edited());$('crawl-quota-patterns').addEventListener('click',event=>{if(event.target.closest('button'))configProfiles.edited();});$('crawl-quota-add').addEventListener('click',()=>configProfiles.edited());
    $('tls-certificate-files').addEventListener('change',()=>configProfiles.edited());
    for(const id of ['seed-url','url-list','crawl-mode'])for(const event of ['input','change'])$(id).addEventListener(event,()=>{configProfiles.edited();configProfiles.clearHeaders();});
    $('settings-dialog').addEventListener('close',()=>configProfiles.close());$('settings-dialog').addEventListener('cancel',()=>configProfiles.close());
    const restored=await configProfiles.restore();if(restored){projectConfigProfile(restored,true);projectProfileToolbar(restored);}renderConfigProfile();
  }

  const crawlQuotaSettingKeys=['crawl_quotas_enabled','crawl_quota_per_depth','crawl_quota_per_subdomain','crawl_quota_patterns_json'];
  const quotaUI={result:null,error:'',token:null};
  let quotaIncreaseContext=null;
  const quotaInteger=(raw,label)=>{if(!/^[0-9]+$/.test(String(raw))||(!Number.isSafeInteger(Number(raw))||Number(raw)>1000000000))throw new Error(label+': Eine ganze Zahl von 0 bis 1000000000 eingeben.');return Number(raw);};
  const quotaCount=value=>Number.isSafeInteger(value)&&value>=0?number(value):'Unbekannt / nicht sicher darstellbar';
  function readQuotaRuleControls(){return [...$('crawl-quota-patterns').querySelectorAll('[data-quota-rule]')].map(row=>({id:row.querySelector('[data-quota-id]').value,pattern:row.querySelector('[data-quota-pattern]').value,max_urls:row.querySelector('[data-quota-max]').value}));}
  function renderQuotaRuleControls(rows){
    if(!rows){try{rows=JSON.parse(state.settings.crawl_quota_patterns_json);}catch(_){rows=[];}}
    if(!Array.isArray(rows)||rows.length>32)rows=[];
    const host=$('crawl-quota-patterns');host.replaceChildren();
    rows.forEach((rule,index)=>{
      const row=document.createElement('fieldset');row.dataset.quotaRule='';const legend=document.createElement('legend');legend.textContent='Muster '+(index+1);row.append(legend);
      for(const [key,label,type] of [['id','Eindeutige ID','text'],['pattern','Rust-Regex','text'],['max','Maximale URLs','number']]){const wrap=document.createElement('label');wrap.textContent=label;const input=document.createElement('input');input.type=type;input.setAttribute('data-quota-'+key,'');input.value=rule[key==='max'?'max_urls':key]??'';if(type==='number'){input.min='0';input.step='1';}wrap.append(input);row.append(wrap);}
      for(const [label,delta] of [['Nach oben',-1],['Nach unten',1],['Entfernen',0]]){const button=document.createElement('button');button.type='button';button.className='button small';button.textContent=label;button.disabled=delta===-1&&index===0||delta===1&&index===rows.length-1;button.addEventListener('click',()=>{const current=readQuotaRuleControls();if(delta)[current[index],current[index+delta]]=[current[index+delta],current[index]];else current.splice(index,1);renderQuotaRuleControls(current);});row.append(button);}host.append(row);
    });
    $('crawl-quota-add').disabled=rows.length>=32;
  }
  function syncQuotaSettings(){const fields=$('crawl-quota-fields');fields.disabled=!$('settings-form').elements.crawl_quotas_enabled.checked;fields.hidden=fields.disabled;}
  function readCrawlQuotaSettings(form){return {crawl_quotas_enabled:form.elements.crawl_quotas_enabled.checked,crawl_quota_per_depth:form.elements.crawl_quota_per_depth.value,crawl_quota_per_subdomain:form.elements.crawl_quota_per_subdomain.value,crawl_quota_patterns_json:JSON.stringify(readQuotaRuleControls())};}
  function parseCrawlQuotas(settings){
    if(settings.crawl_quotas_enabled!==true)return {};
    if(settings.javascript_rendering===true)throw new Error('Crawl-Quoten unterstützen derzeit kein JavaScript-Rendering.');
    if(settings.resource_discovery_enabled===true){
      const resource=parseResourceDiscovery(settings).resource_discovery;
      if(resource.contract==null)throw new Error('Crawl-Quoten mit Ressourcenermittlung benötigen aktuelle resource-contract-v2-Regeln.');
      if(settings.follow_subdomains===true||settings.check_external===true||(resource.resource_scope?.external_origins?.length??0)!==0)throw new Error('Crawl-Quoten mit Ressourcen unterstützen nur den internen Bereich ohne Subdomains, externe Linkprüfung oder externe Ressourcen-Origins.');
    }
    const config={version:1,url_pattern_quotas:[]};
    for(const [setting,key] of [['crawl_quota_per_depth','max_urls_per_depth'],['crawl_quota_per_subdomain','max_urls_per_subdomain']]){const raw=settings[setting];if(raw!==''&&raw!=null)config[key]=quotaInteger(raw,key);}
    const rows=JSON.parse(settings.crawl_quota_patterns_json||'[]'),ids=new Set();
    if(!Array.isArray(rows)||rows.length>32)throw new Error('Höchstens 32 Quoten-Muster sind möglich.');
    let patternBytes=0;const encoder=new TextEncoder();
    for(const row of rows){if(!row||typeof row.id!=='string'||!/^[A-Za-z0-9_-]{1,64}$/.test(row.id)||ids.has(row.id)||typeof row.pattern!=='string'||!row.pattern.length||encoder.encode(row.pattern).length>4096)throw new Error('Jedes Muster benötigt eine eindeutige ASCII-ID (1 bis 64 Buchstaben, Ziffern, _ oder -) und eine Rust-Regex von 1 bis 4096 UTF-8-Bytes.');patternBytes+=encoder.encode(row.pattern).length;if(patternBytes>32768)throw new Error('Alle Quoten-Muster zusammen dürfen höchstens 32768 UTF-8-Bytes belegen.');ids.add(row.id);config.url_pattern_quotas.push({id:row.id,pattern:row.pattern,max_urls:quotaInteger(row.max_urls,'Muster '+row.id)});}
    if(config.max_urls_per_depth==null&&config.max_urls_per_subdomain==null&&!rows.length)throw new Error('Mindestens eine Quote eingeben oder die Funktion ausschalten.');
    return {crawl_quotas:config};
  }
  function resetQuotaSummary(){quotaUI.token?.controller.abort();quotaUI.token=null;quotaUI.result=null;quotaUI.error='';quotaIncreaseContext=null;if($('quota-increase-dialog').open)$('quota-increase-dialog').close();}
  async function loadQuotaSummary(force=false){
    if(!state.crawl||quotaUI.token)return;
    if(!state.crawl.config?.crawl_quotas){quotaUI.result={enabled:false,summary:null,availability:'legacy_unavailable'};quotaUI.error='';renderQuotaSummary();return;}
    if(!force&&quotaUI.result&&quotaUI.result.id!==state.crawl.id)quotaUI.result=null;
    const token={id:state.crawl.id,generation:state.generation,controller:new AbortController()};quotaUI.token=token;renderQuotaSummary();
    try{const result=await request('/api/crawls/'+encodeURIComponent(token.id)+'/quotas',{signal:token.controller.signal});if(quotaUI.token!==token||state.crawl?.id!==token.id||state.generation!==token.generation)return;if(result.id!==token.id||result.availability!=='available'||!result.summary)throw new Error('Keine bestätigte Quoten-Zusammenfassung erhalten.');quotaUI.result=result;quotaUI.error='';}
    catch(error){if(quotaUI.token===token){quotaUI.result=null;if(error.name!=='AbortError')quotaUI.error=error.message;}}
    finally{if(quotaUI.token===token){quotaUI.token=null;renderQuotaSummary();}}
  }
  function renderQuotaSummary(){
    const panel=$('crawl-quota-summary-panel');panel.hidden=!state.crawl;if(!state.crawl)return;
    const summary=quotaUI.result?.summary;
    $('crawl-quota-error').hidden=!quotaUI.error;$('crawl-quota-error').textContent=quotaUI.error;
    $('crawl-quota-state').textContent=quotaUI.token?'Gespeicherte Quoten werden geladen …':quotaUI.error?'Quoten nicht verfügbar; Zähler bleiben unbekannt.':summary?'Crawl-Durchlauf '+quotaCount(summary.generation)+' · Quotenstand '+quotaCount(summary.profile_revision):state.crawl.config?.crawl_quotas?'Quoten sind konfiguriert; noch kein bestätigter gespeicherter Stand geladen.':'Keine gespeicherte Quotenmessung für diesen Crawl. Historische Werte bleiben unbekannt.';
    $('crawl-quota-counts').textContent=summary?'Reservierte Ziele: '+quotaCount(summary.reserved_distinct_targets)+' · Gestartete Versuche: '+quotaCount(summary.actual_started_attempts)+' · Beendete Versuche: '+quotaCount(summary.settled_attempts)+' · Aktive Versuche: '+quotaCount(summary.active_attempts)+' · Unterbrochen / unbekannt: '+quotaCount(summary.interrupted_or_unknown_attempts)+' · Wegen Quoten zurückgestellt: '+quotaCount(summary.deferred_targets):'Reservierte Ziele, Versuche und Zurückstellungen: unbekannt.';
    $('crawl-quota-causes').textContent=summary?'Gespeicherte Quotenursachen: '+(Array.isArray(summary.causes)&&summary.causes.length?summary.causes.join(' · '):'Keine in dieser Zusammenfassung erfasst'):'Quotenursachen: unbekannt.';
    $('crawl-quota-buckets').innerHTML=(Array.isArray(summary?.bucket_rows)?summary.bucket_rows:[]).map(row=>'<tr><td>'+escape(row.dimension)+'</td><td>'+escape(row.key)+'</td><td>'+escape(quotaCount(row.used))+'</td><td>'+escape(quotaCount(row.limit))+'</td></tr>').join('');
    $('crawl-quota-omitted').textContent=summary?'Zusätzliche hier nicht angezeigte Gruppen: '+quotaCount(summary.bucket_rows_omitted):'';
    $('crawl-quota-refresh').disabled=Boolean(quotaUI.token||actionBusy());
    $('crawl-quota-increase').disabled=!summary||!canResume()||actionBusy()||Boolean(quotaUI.token)||!Number.isSafeInteger(summary.generation)||!Number.isSafeInteger(summary.profile_revision)||!/^[0-9a-f]{64}$/.test(summary.profile_sha256);
  }
  function openQuotaIncrease(){
    const summary=quotaUI.result?.summary;if(!summary||!canResume()||actionBusy()||quotaUI.token)return;
    if(!Number.isSafeInteger(summary.generation)||!Number.isSafeInteger(summary.profile_revision)||!/^[0-9a-f]{64}$/.test(summary.profile_sha256)){message('Der gespeicherte Quotenstand kann nicht sicher gebunden werden.',true);return;}
    const fields=$('quota-increase-fields');fields.replaceChildren();const entries=[];
    if(summary.config.max_urls_per_depth!=null)entries.push(['max_urls_per_depth','Je Crawl-Tiefe',summary.config.max_urls_per_depth]);
    if(summary.config.max_urls_per_subdomain!=null)entries.push(['max_urls_per_subdomain','Je Host / Subdomain',summary.config.max_urls_per_subdomain]);
    for(const rule of summary.config.url_pattern_quotas||[])entries.push(['pattern:'+rule.id,'Muster '+rule.id+' · '+rule.pattern,rule.max_urls]);
    if(entries.some(([, ,cap])=>!Number.isSafeInteger(cap)||cap<0)){message('Mindestens eine gespeicherte Grenze ist nicht sicher darstellbar.',true);return;}
    for(const [key,label,cap] of entries){const wrap=document.createElement('label');wrap.textContent=label+' (aktuell '+quotaCount(cap)+')';const input=document.createElement('input');input.type='number';input.min=String(cap);input.step='1';input.value=String(cap);input.dataset.quotaIncrease=key;wrap.append(input);fields.append(wrap);}
    quotaIncreaseContext={id:state.crawl.id,generation:state.generation,summary,entries};$('quota-increase-error').hidden=true;$('quota-increase-dialog').showModal();
  }
  function submitQuotaIncrease(event){
    event.preventDefault();const context=quotaIncreaseContext;if(!context||context.id!==state.crawl?.id||context.generation!==state.generation||!canResume()){$('quota-increase-dialog').close();return;}
    try{const summary=context.summary,increases={expected_generation:summary.generation,expected_profile_revision:summary.profile_revision,expected_profile_sha256:summary.profile_sha256,url_pattern_maxima:{}};let changed=false;
      for(const [index,input] of [...$('quota-increase-fields').querySelectorAll('[data-quota-increase]')].entries()){const [key,,cap]=context.entries[index],value=quotaInteger(input.value,key);if(value<cap)throw new Error('Quoten dürfen nicht abgesenkt werden.');if(value===cap)continue;changed=true;if(key.startsWith('pattern:'))Object.defineProperty(increases.url_pattern_maxima,key.slice(8),{value,enumerable:true});else increases[key]=value;}
      if(!changed)throw new Error('Mindestens eine bestehende Quote erhöhen.');quotaIncreaseContext=null;$('quota-increase-dialog').close();controlCrawl('resume',{quota_increases:increases},context);
    }catch(error){$('quota-increase-error').textContent=error.message;$('quota-increase-error').hidden=false;}
  }

  const crawlAdmissionFields = [
    {setting:'crawl_max_folder_depth',native:'max_folder_depth',minimum:0,maximum:65535,label:'Ordnertiefe'},
    {setting:'crawl_max_query_tokens',native:'max_query_tokens',minimum:0,maximum:65535,label:'Query-Tokens'},
    {setting:'crawl_max_fetch_url_utf8_bytes',native:'max_fetch_url_utf8_bytes',minimum:1,maximum:1048576,label:'Abruf-URL in UTF-8-Bytes'}
  ];
  const crawlAdmissionSettingKeys=['crawl_admission_limits_enabled',...crawlAdmissionFields.map(field=>field.setting)];
  function readCrawlAdmissionSettings(form){const values={crawl_admission_limits_enabled:form.elements.crawl_admission_limits_enabled.checked};for(const field of crawlAdmissionFields)values[field.setting]=form.elements[field.setting].value;return values;}
  function syncCrawlAdmissionSettings(){const fields=$('crawl-admission-fields');fields.disabled=!$('settings-form').elements.crawl_admission_limits_enabled.checked;fields.hidden=fields.disabled;}
  function parseCrawlAdmissionLimits(settings){
    if(settings.crawl_admission_limits_enabled!==true)return {};
    if(settings.javascript_rendering===true)throw new Error('Statische URL-Grenzen unterstützen derzeit keinen JavaScript-Renderer.');
    const limits={version:1};let count=0;
    for(const field of crawlAdmissionFields){const raw=settings[field.setting];if(raw==null||raw==='')continue;const value=Number(raw);if(!(typeof raw==='number'||typeof raw==='string'&&/^[0-9]+$/.test(raw))||!Number.isSafeInteger(value)||value<field.minimum||value>field.maximum)throw new Error(field.label+': Eine ganze Zahl von '+field.minimum+' bis '+field.maximum+' ist erforderlich.');limits[field.native]=value;count++;}
    if(count===0)throw new Error('Mindestens eine statische URL-Grenze eingeben oder die Funktion ausschalten.');
    return {crawl_admission_limits:limits};
  }

  const sitemapSourceFields = [
    { setting:'sitemap_source_max_post_http_entity_bytes', native:'max_post_http_entity_bytes', initial:8388608, maximum:67108864, label:'Eingangsdaten der Sitemap' },
    { setting:'sitemap_source_max_decoded_xml_bytes', native:'max_decoded_xml_bytes', initial:8388608, maximum:67108864, label:'Entpackte XML-Daten' },
    { setting:'sitemap_source_max_xml_events', native:'max_xml_events', initial:500000, maximum:2000000, label:'XML-Ereignisse' }
  ];
  const sitemapMembershipFields = [
    {setting:'sitemap_membership_max_catalog_json_bytes',native:'max_catalog_json_bytes',initial:33554432,maximum:134217728,label:'Sitemap-Katalog'},
    {setting:'sitemap_membership_max_row_json_bytes',native:'max_row_json_bytes',initial:262144,maximum:1048576,label:'Sitemap-Zeile'},
    {setting:'sitemap_membership_max_rows',native:'max_rows',initial:1000000,maximum:1000000,label:'Sitemap-Katalogzeilen'}
  ];
  function readSitemapSourceProcessingSettings(form) {
    const result = { sitemap_source_processing_enabled:form.elements.sitemap_source_processing_enabled.checked, sitemap_membership_limits_enabled:form.elements.sitemap_membership_limits_enabled.checked };
    for (const field of sitemapMembershipFields) result[field.setting] = form.elements.namedItem(field.setting).value;
    for (const field of sitemapSourceFields) result[field.setting] = form.elements.namedItem(field.setting).value;
    return result;
  }
  function syncSitemapSourceProcessingSettings() {
    const enabled = $('settings-form').elements.sitemap_source_processing_enabled.checked;
    const fields = $('sitemap-source-processing-fields');
    fields.disabled = !enabled;
    fields.hidden = !enabled;
    const membershipEnabled = $('settings-form').elements.sitemap_membership_limits_enabled.checked;
    const membershipFields = $('sitemap-membership-limits-fields');
    membershipFields.disabled = !membershipEnabled; membershipFields.hidden = !membershipEnabled;
  }
  function parseSitemapSourceProcessing(settings) {
    if (settings.sitemap_source_processing_enabled !== true) return {};
    const profile = {};
    for (const field of sitemapSourceFields) {
      const raw = settings[field.setting], value = Number(raw);
      if (!(typeof raw === 'number' || typeof raw === 'string' && /^[0-9]+$/.test(raw)) || !Number.isSafeInteger(value) || value < 1 || value > field.maximum) {
        throw new Error(field.label + ': Eine ganze Zahl von 1 bis ' + field.maximum + ' ist erforderlich.');
      }
      profile[field.native] = value;
    }
    return { sitemap_source_processing:profile };
  }

  function parseSitemapMembershipLimits(settings) {
    if (settings.sitemap_membership_limits_enabled !== true) return {};
    const limits = {};
    for (const field of sitemapMembershipFields) {
      const raw=settings[field.setting],value=Number(raw);
      if (!(typeof raw==='number'||typeof raw==='string'&&/^[0-9]+$/.test(raw))||!Number.isSafeInteger(value)||value<1||value>field.maximum) throw new Error(field.label+': Eine ganze Zahl von 1 bis '+field.maximum+' ist erforderlich.');
      limits[field.native]=value;
    }
    if (limits.max_row_json_bytes>limits.max_catalog_json_bytes) throw new Error('Die Sitemap-Zeilengrenze darf die Kataloggrenze nicht überschreiten.');
    return {sitemap_membership_limits:limits};
  }

  const sitemapArchiveFields = [
    {setting:'sitemap_source_archive_max_body_bytes',native:'max_bytes_per_body',initial:67108864,maximum:67108864,label:'Dateigröße der Sitemap-Quelle'},
    {setting:'sitemap_source_archive_max_total_bytes',native:'max_total_body_bytes',initial:536870912,maximum:8589934592,label:'Originaldaten je Crawl'},
    {setting:'sitemap_source_archive_max_records',native:'max_capture_records',initial:4096,maximum:100000,label:'Quellenaufzeichnungen'}
  ];
  const sitemapArchiveSettingKeys = ['sitemap_source_archive_enabled','sitemap_source_archive_post_http','sitemap_source_archive_decoded_xml',...sitemapArchiveFields.map(field=>field.setting)];
  function readSitemapSourceArchiveSettings(form) {
    const result={sitemap_source_archive_enabled:form.elements.sitemap_source_archive_enabled.checked,sitemap_source_archive_post_http:form.elements.sitemap_source_archive_post_http.checked,sitemap_source_archive_decoded_xml:form.elements.sitemap_source_archive_decoded_xml.checked};
    for(const field of sitemapArchiveFields)result[field.setting]=form.elements.namedItem(field.setting).value;
    return result;
  }
  function syncSitemapSourceArchiveSettings(){const enabled=$('settings-form').elements.sitemap_source_archive_enabled.checked,fields=$('sitemap-source-archive-fields');fields.disabled=!enabled;fields.hidden=!enabled;}
  function parseSitemapSourceArchive(settings){
    if(settings.sitemap_source_archive_enabled!==true)return {};
    const profile={retain_post_http_entity:settings.sitemap_source_archive_post_http===true,retain_decoded_xml:settings.sitemap_source_archive_decoded_xml===true};
    if(!profile.retain_post_http_entity&&!profile.retain_decoded_xml)throw new Error('Für Originaldaten mindestens eine Darstellung auswählen.');
    for(const field of sitemapArchiveFields){const raw=settings[field.setting],value=Number(raw);if(!(typeof raw==='number'||typeof raw==='string'&&/^[0-9]+$/.test(raw))||!Number.isSafeInteger(value)||value<1||value>field.maximum)throw new Error(field.label+': Eine ganze Zahl von 1 bis '+field.maximum+' ist erforderlich.');profile[field.native]=value;}
    return {sitemap_source_archive:profile};
  }
  const sourceBodies={sequence:0,controller:null,result:null,cursor:null,back:[],busy:false,error:'',bodies:[],documentId:null};
  function selectedSourceBodyDocument(){const row=sitemapMembership.selected?.row;return typeof row?.observation_id==='string'&&/^[a-f0-9]{64}$/.test(row.observation_id)?row.observation_id:null;}
  function resetSourceBodies(){sourceBodies.sequence++;sourceBodies.controller?.abort();Object.assign(sourceBodies,{controller:null,result:null,cursor:null,back:[],busy:false,error:'',bodies:[],documentId:selectedSourceBodyDocument()});renderSourceBodies();}
  function renderSourceBodies(){
    if(!$('sitemap-source-bodies-status'))return;
    const window=sourceBodies.result?.window;
    $('sitemap-source-bodies-status').textContent=sourceBodies.busy?'Originaldaten werden geprüft …':window?.state==='recorded_metadata'?`${number(window.summary?.captures||0)} gespeicherte Quellenaufzeichnungen · Dateiverfügbarkeit noch nicht geprüft`:window?.state==='inherited_archive_not_copied'?'Geerbter Crawl: keine eigene verifizierte Dateikopie.':sourceBodies.result?'Originaldaten nicht aufbewahrt oder historische Verfügbarkeit unbekannt.':'Originaldaten wurden noch nicht geladen.';
    $('sitemap-source-bodies-error').hidden=!sourceBodies.error;$('sitemap-source-bodies-error').textContent=sourceBodies.error;
    const bodies=[];const rows=[];
    for(const capture of window?.captures||[]){if(capture.omission)rows.push('<li>Quelle '+escape(capture.measurement_id)+' · Nicht aufbewahrt: '+escape(capture.omission)+'</li>');for(const body of capture.bodies||[]){const index=bodies.push(body)-1;const representation=body.representation==='decoded_xml'?'Dekodierte XML':'Entity nach HTTP-Dekodierung',extent=body.extent==='complete'?'vollständig beobachtet':'beobachteter Präfix';rows.push('<li><p>'+escape(representation)+' · '+escape(extent)+' · '+escape(number(body.bytes))+' Bytes</p><p class="detail-empty">SHA-256: '+escape(body.sha256)+'</p><button type="button" class="button small" data-source-body="'+index+'"'+(sourceBodies.busy?' disabled':'')+'>Prüfen und herunterladen</button></li>');}}
    sourceBodies.bodies=bodies;$('sitemap-source-bodies-list').innerHTML=rows.join('');$('sitemap-source-bodies-list').querySelectorAll('[data-source-body]').forEach(button=>button.addEventListener('click',()=>downloadSourceBody(Number(button.dataset.sourceBody))));
    $('sitemap-source-bodies-refresh').disabled=sourceBodies.busy||!state.crawl?.id;$('sitemap-source-bodies-prev').disabled=sourceBodies.busy||!sourceBodies.back.length;$('sitemap-source-bodies-next').disabled=sourceBodies.busy||!sourceBodies.result?.cursor;
  }
  async function loadSourceBodies(cursor=null,navigation=null){
    const id=state.crawl?.id;if(!id)return;const documentId=selectedSourceBodyDocument();sourceBodies.documentId=documentId;sourceBodies.controller?.abort();const controller=new AbortController(),sequence=++sourceBodies.sequence,generation=state.generation;sourceBodies.controller=controller;sourceBodies.busy=true;sourceBodies.error='';renderSourceBodies();
    const current=()=>sequence===sourceBodies.sequence&&generation===state.generation&&state.crawl?.id===id;
    try{const result=await request(`/api/crawls/${encodeURIComponent(id)}/sitemaps/source-bodies?limit=50${documentId?'&document_id='+documentId:''}${cursor?'&cursor='+encodeURIComponent(cursor):''}`,{signal:controller.signal});if(!current())return;if(result.job!==id||result.byte_availability!=='not_checked'||result.download_available!==null)throw new Error('Unerwartete Originaldaten-Metadaten.');if(navigation==='next')sourceBodies.back.push(sourceBodies.cursor);else if(navigation==='prev')sourceBodies.back.pop();else sourceBodies.back=[];sourceBodies.cursor=cursor;sourceBodies.result=result;}
    catch(error){if(current()&&error.name!=='AbortError')sourceBodies.error=error.message;}finally{if(current()){sourceBodies.busy=false;sourceBodies.controller=null;renderSourceBodies();}}
  }
  async function downloadSourceBody(index){
    const body=sourceBodies.bodies[index],snapshot=sourceBodies.result?.snapshot,id=state.crawl?.id;if(!body||!snapshot||!id||sourceBodies.busy)return;
    if(sourceBodies.documentId!==selectedSourceBodyDocument()){resetSourceBodies();sourceBodies.error='Andere Sitemap-Quelle ausgewählt. Originaldaten erneut laden.';renderSourceBodies();return;}
    if(!/^[a-f0-9]{64}$/.test(body.body_id)||!Number.isSafeInteger(body.bytes)||body.bytes<0||body.bytes>67108864){sourceBodies.error='Ungültige Dateibeschreibung.';renderSourceBodies();return;}
    const sequence=++sourceBodies.sequence,generation=state.generation,controller=new AbortController();sourceBodies.controller=controller;sourceBodies.busy=true;sourceBodies.error='';renderSourceBodies();const current=()=>sequence===sourceBodies.sequence&&generation===state.generation&&state.crawl?.id===id;let reader;
    try{const response=await fetch(`/api/crawls/${encodeURIComponent(id)}/sitemaps/source-bodies/${body.body_id}?snapshot=${encodeURIComponent(snapshot)}`,{signal:controller.signal});if(!response.ok){const failure=await response.json().catch(()=>({}));throw new Error(failure.message||failure.error||`HTTP ${response.status}`);}if(response.headers.get('content-type')!=='application/octet-stream'||!/^(0|[1-9][0-9]*)$/.test(response.headers.get('content-length')||'')||Number(response.headers.get('content-length'))!==body.bytes||response.headers.get('etag')!==`"sha256:${body.sha256}"`)throw new Error('Die geprüfte Dateibeschreibung hat sich geändert. Originaldaten erneut laden.');if(!response.body)throw new Error('Kein binärer Download-Stream verfügbar.');reader=response.body.getReader();const chunks=[];let count=0;for(;;){const item=await reader.read();if(item.done)break;count+=item.value.byteLength;if(count>body.bytes||count>67108864)throw new Error('Der Download überschreitet die Dateigrenze.');chunks.push(item.value);}if(count!==body.bytes)throw new Error('Der Download wurde vorzeitig beendet.');if(!current())return;const blob=new Blob(chunks,{type:'application/octet-stream'}),url=URL.createObjectURL(blob),anchor=document.createElement('a');anchor.href=url;anchor.download=`sitemap-${body.body_id}-${body.representation}-${body.extent}.bin`;document.body.append(anchor);anchor.click();anchor.remove();setTimeout(()=>URL.revokeObjectURL(url),1000);}
    catch(error){if(current()&&error.name!=='AbortError')sourceBodies.error=error.message;}finally{if(reader)await reader.cancel().catch(()=>{});if(current()){sourceBodies.busy=false;sourceBodies.controller=null;renderSourceBodies();}}
  }
  function bindSourceBodyEvents(){
    $('sitemap-source-bodies-refresh').addEventListener('click',()=>loadSourceBodies());$('sitemap-source-bodies-next').addEventListener('click',()=>{if(sourceBodies.result?.cursor)loadSourceBodies(sourceBodies.result.cursor,'next');});$('sitemap-source-bodies-prev').addEventListener('click',()=>{if(sourceBodies.back.length)loadSourceBodies(sourceBodies.back.at(-1),'prev');});
  }

  function parseCustomSearch(settings) {
    if (settings.custom_search_enabled !== true) return {};
    let profile; try { profile=JSON.parse(settings.custom_search_json||'{}'); } catch { throw new Error('Eigene Suche: Gültiges JSON ist erforderlich.'); }
    if(!profile||typeof profile!=='object'||Array.isArray(profile)||profile.version!==1||!Array.isArray(profile.rules)||!profile.rules.length)throw new Error('Eigene Suche: version 1 und mindestens eine Regel sind erforderlich.');
    for(const rule of profile.rules){if(!rule||typeof rule!=='object'||Array.isArray(rule)||typeof rule.name!=='string'||!rule.name.trim()||!['literal','regex'].includes(rule.method)||typeof rule.pattern!=='string'||rule.method==='literal'&&!rule.pattern||!Array.isArray(rule.sources)||!rule.sources.length||rule.sources.some(source=>!['raw_html','rendered_html','raw_selected_text','rendered_selected_text'].includes(source)))throw new Error('Eigene Suche: Jede Regel benötigt name, method, pattern und ausdrücklich gewählte sources.');}
    return {custom_search:profile};
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
    return { custom_extractions, audit_thresholds: parseAuditThresholds(settings.audit_thresholds_json), pdf_analysis: parsePdfAnalysis(settings.pdf_analysis_json), link_position_rules: parsePositionRules(settings.link_position_rules_json), validate_fragments: settings.validate_fragments === true, include_patterns: lines(settings.include_patterns_text), exclude_patterns: lines(settings.exclude_patterns_text), ...parseCustomSearch(settings), ...parseNativePolicy(settings), ...parseContentCapture(settings),...parseResourceDiscovery(settings), ...parseSitemapSourceProcessing(settings), ...parseSitemapMembershipLimits(settings), ...parseSitemapSourceArchive(settings), ...parseCrawlQuotas(settings), ...parseCrawlAdmissionLimits(settings), ...parseTlsSettings(settings), ...parseCdnClassification(settings), ...parseCdnDirectoryReporting(settings), ...parseDirectiveReporting(settings) };
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

  const directiveReportingKeys=["respect_noindex","respect_canonical","respect_next_prev"];
  function readDirectiveReportingSettings(form){return Object.fromEntries(directiveReportingKeys.map(key=>[key,form.elements[key].checked]));}
  function restoreDirectiveReportingSettings(saved){return Object.fromEntries(directiveReportingKeys.map(key=>[key,saved[key]===true]));}
  function parseDirectiveReporting(settings){
    const flags=Object.fromEntries(directiveReportingKeys.map(key=>{const value=settings[key]??false;if(typeof value!=="boolean")throw new Error("Berichtsfilter: true oder false erforderlich.");return[key,value];}));
    return Object.values(flags).some(Boolean)?{directive_reporting:{version:1,...flags}}:{};
  }
  function directiveReportingSettingsFromCrawlConfig(config) {
    if (!config || typeof config !== 'object' || Array.isArray(config)) throw new Error('Die Konfiguration des ausgewählten Crawls ist noch nicht geladen.');
    const profile = config.directive_reporting;
    if (profile == null) return Object.fromEntries(directiveReportingKeys.map(key => [key, false]));
    const allowed = ['version', ...directiveReportingKeys];
    if (typeof profile !== 'object' || Array.isArray(profile) || Object.keys(profile).length !== allowed.length || allowed.some(key => !Object.hasOwn(profile, key)) || profile.version !== 1 || directiveReportingKeys.some(key => typeof profile[key] !== 'boolean')) throw new Error('Die gespeicherten Berichtsfilter haben kein unterstütztes Version-1-Profil mit drei booleschen Schaltern.');
    return Object.fromEntries(directiveReportingKeys.map(key => [key, profile[key]]));
  }
  function syncDirectiveReportingProfileSource() {
    const crawl = state.crawl, config = crawl?.config;
    const available = Boolean(crawl && config && typeof config === 'object' && !Array.isArray(config));
    $('directive-reporting-load-selected').disabled = !available;
    $('directive-reporting-profile-source').textContent = available ? 'Quelle: ausgewählter Crawl ' + crawl.id + '. Es werden nur die drei Berichtsfilter geladen.' : crawl ? 'Die Konfiguration des ausgewählten Crawls ist noch nicht geladen.' : 'Zuerst einen gespeicherten Crawl auswählen, um dessen Berichtsfilter zu laden.';
  }
  function loadSelectedDirectiveReportingProfile() {
    const status = $('directive-reporting-load-status');
    try {
      if (!state.crawl) throw new Error('Zuerst einen gespeicherten Crawl auswählen.');
      const flags = directiveReportingSettingsFromCrawlConfig(state.crawl.config);
      const form = $('settings-form');
      for (const key of directiveReportingKeys) form.elements[key].checked = flags[key];
      status.setAttribute('role', 'status');
      status.textContent = 'Berichtsfilter aus Crawl ' + state.crawl.id + ' in den Entwurf übernommen. Mit Speichern gelten sie für den nächsten Crawl.';
    } catch (error) {
      status.setAttribute('role', 'alert');
      status.textContent = 'Berichtsfilter konnten nicht geladen werden: ' + error.message;
    }
  }
  function directiveReportingNote(){if(state.segmentView){const r=state.segmentView.cohort.reporting;return `<p class="graph-note">Bestätigte Segmentauswahl: ${number(state.segmentView.cohort.summary.selected_pages)} von ${number(state.segmentView.cohort.summary.total_pages)} Seitenbeobachtungen${r?` · ${number(r.captured_page_observations)} erfasst · ${number(r.profile_excluded_page_observations)} durch Berichtsprofil ausgeblendet`:''}. Globale Crawl-Zähler und Beziehungspartner bleiben getrennt.</p>`;}const counts=state.crawl?.reporting_visibility;return counts?`<p class="graph-note">Berichtsfilter: ${number(counts.visible_pages)} sichtbar, ${number(counts.hidden_pages)} ausgeblendet, ${number(counts.captured_pages)} erfasst. Abrufe und ausgehende Linkentdeckung bleiben erhalten. Roh-Snapshots und getrennte Analysebereiche sind ungefiltert.</p>`:"";}

  function restoreCdnDirectoryReportingSettings(saved){return {cdn_directory_reporting_enabled:saved.cdn_directory_reporting_enabled===true,cdn_directory_rules_text:typeof saved.cdn_directory_rules_text==='string'?saved.cdn_directory_rules_text:''};}
  function readCdnDirectoryReportingSettings(form){return {cdn_directory_reporting_enabled:form.elements.cdn_directory_reporting_enabled.checked,cdn_directory_rules_text:form.elements.cdn_directory_rules_text.value};}
  function parseCdnDirectoryReporting(settings){
    if(!settings.cdn_directory_reporting_enabled)return {};
    const text=settings.cdn_directory_rules_text,encoder=new TextEncoder();
    const invalid=()=>new Error('Eine CDN-Verzeichnisadresse ist ungültig. Verwende einen ASCII-Domainnamen mit http:// oder https:// und einem abschließenden /. Zugangsdaten, Platzhalter, Zusätze, Punkte-Verzeichnisse und die Ports 80/443 sind hier nicht erlaubt.');
    if(typeof text!=='string'||encoder.encode(text).length>131072)throw new Error('Die CDN-Verzeichnisliste ist zu lang. Gib höchstens 32 Adressen ein.');
    const rules=text.split(/\r?\n/).filter(value=>value.trim()!=='');
    if(rules.length<1||rules.length>32)throw new Error('Gib 1 bis 32 CDN-Verzeichnisadressen ein, jeweils eine pro Zeile.');
    const seen=new Set();
    for(const value of rules){
      if(encoder.encode(value).length>2048||/[^\x21-\x7e]|[\\?#@*]/.test(value))throw invalid();
      const parts=/^(https?):\/\/([^/]+)(\/.*)?$/i.exec(value);if(!parts)throw invalid();
      const authority=parts[2].split(':'),host=authority[0].toLowerCase(),directory=parts[3]||'/';
      if(authority.length>2||host.length>253||!host.split('.').every(label=>label.length>=1&&label.length<=63&&/^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/.test(label)&&!label.startsWith('xn--')))throw invalid();
      const port=authority.length===2?Number(authority[1]):null;
      if(authority.length===2&&(!/^\d+$/.test(authority[1])||!Number.isInteger(port)||port<1||port>65535||port===80||port===443))throw invalid();
      if(!directory.endsWith('/')||/%(?![a-f0-9]{2})/i.test(directory)||directory.split('/').some(segment=>['.','..'].includes(segment.replace(/%2e/ig,'.'))))throw invalid();
      let url;try{url=new URL(value);}catch(_){throw invalid();}
      if(url.hostname!==host||url.pathname!==directory||(url.port?Number(url.port):null)!==port)throw invalid();
      // Numeric/IP hosts are outside this own DNS profile, including URL's IPv4 aliases.
      if(/^[\d.]+$/.test(url.hostname))throw invalid();
      const alias=host.replace(/^www\./,'');let base;try{base=new URL(`http://${alias}/`);}catch(_){throw invalid();}
      if(base.hostname!==alias||/^[\d.]+$/.test(base.hostname))throw invalid();
      const key=JSON.stringify([alias,port,directory]);
      if(seen.has(key))throw new Error('Eine CDN-Verzeichnisregel ist mehrfach eingetragen. Auch http/https und ein vorangestelltes www zählen hier zur selben Regel.');
      seen.add(key);
    }
    return {cdn_directory_reporting:{version:1,rules}};
  }
  function cdnDirectoryReportText(observation){
    if(!observation||!Array.isArray(observation.matching_rules)||!observation.matching_rules.length)return 'Kein gespeicherter CDN-Verzeichnisbefund; alte Beobachtungen werden nicht nachträglich zugeordnet.';
    return observation.matching_rules.slice(0,32).map(rule=>`Regel ${Number(rule.rule_index)+1}: ${String(rule.declared_directory)}`).join('\n')+'\nDeklarierte Berichtszuordnung · '+String(observation.engine)+' · '+String(observation.normalization)+'\nKonfigurations-SHA256 '+String(observation.config_sha256)+'\nUrsprünglicher Seed-Bereich: '+(observation.seed_scope_internal?'intern':'extern')+' · keine zusätzliche Abruf-/Traversierungs-/Headerfreigabe';
  }

  let tlsFileRevision = 0;
  function restoreTlsSettings(saved) {
    return {tls_enabled:saved.tls_enabled===true,tls_respect_hsts:saved.tls_respect_hsts===true,tls_certificates_json:typeof saved.tls_certificates_json==='string'?saved.tls_certificates_json:'[]'};
  }
  function readTlsSettings(form) {
    return {tls_enabled:form.elements.tls_enabled.checked,tls_respect_hsts:form.elements.tls_respect_hsts.checked,tls_certificates_json:form.elements.tls_certificates_json.value};
  }
  function parseTlsSettings(settings) {
    if(!settings.tls_enabled)return {};
    const text=settings.tls_certificates_json, encoder=new TextEncoder();
    if(typeof text!=='string'||encoder.encode(text).length>600000)throw new Error('Die Zertifikatsliste ist zu groß.');
    let files;try{files=JSON.parse(text);}catch(_){throw new Error('Zertifikate: gültiges JSON erforderlich.');}
    if(!Array.isArray(files)||files.length>8)throw new Error('Höchstens 8 öffentliche Zertifikatsdateien sind erlaubt.');
    let total=0;const names=new Set();
    for(const file of files){
      if(!file||typeof file!=='object'||Array.isArray(file)||Object.keys(file).sort().join(',')!=='data,format,name'||!['pem','der'].includes(file.format)||typeof file.name!=='string'||!file.name||encoder.encode(file.name).length>256||/[\x00-\x1f\x7f/\\]/.test(file.name)||names.has(file.name)||typeof file.data!=='string')throw new Error('Zertifikat: format, eindeutiger name und data sind erforderlich.');
      names.add(file.name);
      let size;
      if(file.format==='der'){
        if(!/^(?:[0-9a-f]{2})+$/.test(file.data))throw new Error('DER muss als klein geschriebene Hex-Zeichenfolge vorliegen.');
        size=file.data.length/2;
      }else{
        const rest=file.data.trim().replace(/-----BEGIN CERTIFICATE-----[\t\r\n A-Za-z0-9+/=]+-----END CERTIFICATE-----/g,'').trim();
        if(rest||!file.data.includes('-----BEGIN CERTIFICATE-----'))throw new Error('PEM darf nur öffentliche CERTIFICATE-Blöcke enthalten, keine privaten Schlüssel.');
        size=encoder.encode(file.data).length;
      }
      total+=size;
      if(!size||size>65536||total>262144)throw new Error('Zertifikate: höchstens 65536 Bytes je Datei und 262144 Bytes insgesamt.');
    }
    const respect_hsts=settings.tls_respect_hsts===true;
    if(!respect_hsts&&!files.length)return {};
    if(settings.javascript_rendering)throw new Error('Crawl-eigene Zertifikate und HSTS sind mit dem externen JavaScript-Renderer noch nicht unterstützt. Keine Option wird automatisch ausgeschaltet.');
    return {tls:{version:1,respect_hsts,trusted_certificates:files}};
  }
  async function stageTlsCertificateFiles(event) {
    const input=event.target, files=Array.from(input.files||[]), form=$('settings-form');
    const revision=++tlsFileRevision, prior=form.elements.tls_certificates_json.value;
    try{
      if(!files.length)return;
      if(files.length>8||files.some(file=>!/\.(crt|pem|cer|der)$/i.test(file.name)||file.size<1||file.size>65536)||files.reduce((n,file)=>n+file.size,0)>262144)throw new Error('Wähle bis zu 8 .crt/.pem/.cer/.der-Dateien im angegebenen Byte-Limit.');
      const selected=[];
      for(const file of files){
        const bytes=new Uint8Array(await file.arrayBuffer());
        let pem=null;try{const decoded=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes);if(decoded.replace(/^[\t\n\v\f\r ]+/, '').startsWith('-----BEGIN'))pem=decoded;}catch(_){}
        selected.push({format:pem===null?'der':'pem',name:file.name,data:pem===null?Array.from(bytes,b=>b.toString(16).padStart(2,'0')).join(''):pem});
      }
      parseTlsSettings({tls_enabled:true,tls_respect_hsts:form.elements.tls_respect_hsts.checked,tls_certificates_json:JSON.stringify(selected),javascript_rendering:false});
      if(revision!==tlsFileRevision||!$('settings-dialog').open||form.elements.tls_certificates_json.value!==prior)return;
      form.elements.tls_certificates_json.value=JSON.stringify(selected,null,2);
      if(typeof configProfiles!=='undefined'&&configProfiles)configProfiles.edited();
      $('tls-file-status').textContent='Öffentliche Zertifikatsdaten vorgemerkt. Erst Speichern übernimmt sie für den nächsten Crawl.';
    }catch(error){
      if(revision===tlsFileRevision&&$('settings-dialog').open)$('tls-file-status').textContent=error.message;
    }finally{input.value='';}
  }
  function restoreCdnClassificationSettings(saved){return {cdn_classification_enabled:saved.cdn_classification_enabled===true,cdn_origins_text:typeof saved.cdn_origins_text==='string'?saved.cdn_origins_text:''};}
  function readCdnClassificationSettings(form){return {cdn_classification_enabled:form.elements.cdn_classification_enabled.checked,cdn_origins_text:form.elements.cdn_origins_text.value};}
  function parseCdnClassification(settings){
    if(!settings.cdn_classification_enabled)return {};
    const text=settings.cdn_origins_text,encoder=new TextEncoder();
    if(typeof text!=='string'||encoder.encode(text).length>131072)throw new Error('Die Liste der CDN-Adressen ist zu lang. Gib höchstens 32 Adressen ein.');
    const origins=text.split(/\r?\n/).filter(value=>value.trim()!=='');
    if(origins.length<1||origins.length>32)throw new Error('Gib 1 bis 32 CDN-Adressen ein, jeweils eine pro Zeile.');
    const seen=new Set();
    for(const value of origins){
      if(value.trim()!==value||encoder.encode(value).length>2048||/[\x00-\x1f\x7f\\?#@*]/.test(value))throw new Error('Eine CDN-Adresse ist ungültig oder zu lang. Entferne Leerzeichen, Platzhalter und Zusätze nach der Adresse.');
      const parts=value.split('://');if(parts.length!==2)throw new Error('Jede CDN-Adresse muss mit http:// oder https:// beginnen.');
      const slash=parts[1].indexOf('/'),suffix=slash<0?'':parts[1].slice(slash);if(suffix!==''&&suffix!=='/')throw new Error('Gib nur die CDN-Adresse mit optionalem Port ein, ohne Unterverzeichnis oder Dateipfad.');
      let url;try{url=new URL(value);}catch(_){throw new Error('Eine CDN-Adresse ist keine gültige Webadresse.');}
      if(!['http:','https:'].includes(url.protocol)||!url.hostname||url.username||url.password||url.pathname!=='/'||url.search||url.hash)throw new Error('Erlaubt sind nur http://- oder https://-Adressen ohne Zugangsdaten oder Zusätze.');
      if(seen.has(url.origin))throw new Error('Eine CDN-Adresse ist mehrfach eingetragen. Entferne den doppelten Eintrag.');seen.add(url.origin);
    }
    return {cdn_classification:{version:1,origins}};
  }

  function parseResourceDiscovery(settings) {
    if (!settings.resource_discovery_enabled) return {};
    if (settings.javascript_rendering) throw new Error('Native Ressourcenermittlung und JavaScript-Rendering können nicht gemeinsam verwendet werden. Keine Einstellung wird automatisch ausgeschaltet.');
    const config=contentJSON(settings.resource_discovery_json,['resources','navigation','css','resource_scope','follow_rel_nofollow','follow_document_nofollow','contract'],'Ressourcenermittlung');
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
    if(config.contract!=null) {
      policyObject(config.contract,['engine','follow_internal_qualified_links','follow_external_qualified_links'],'Aktuelle Ressourcenregeln');
      if(Object.hasOwn(config.contract,'engine') && config.contract.engine!==resourceContractEngine)throw new Error('Nicht unterstützte aktuelle Ressourcenregeln. Vorhandene Einstellungen wurden nicht umgestellt.');
      for(const key of ['follow_internal_qualified_links','follow_external_qualified_links'])if(Object.hasOwn(config.contract,key) && typeof config.contract[key]!=='boolean')throw new Error(`Aktuelle Ressourcenregeln.${key}: true oder false erforderlich.`);
      if(config.follow_rel_nofollow===true || config.follow_document_nofollow===true)throw new Error('Aktuelle Ressourcenregeln und die bisherigen Rel-/Dokument-Nofollow-Schalter widersprechen sich. Unter den aktuellen Regeln müssen beide bisherigen Schalter false sein.');
    }
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

  // B12 draft: tagged stored facts. No target document status is inferred here.
  // Findings has its own query, opaque cursor roles and download ownership.
  const sitemapFindings = { result:null, token:null, download:null, revision:0, cursor:null, back:[], selected:null, error:'', downloadError:'', downloadBlocked:false, queryDirty:false };
  const findingsFilters = {
    'urls-in-sitemap':['URLs in Sitemap','saved_page'],
    'urls-not-in-sitemap':['URLs nicht in Sitemap','saved_page'],
    'orphan-urls':['Verwaiste URLs im gespeicherten Linkgraph','saved_page'],
    'non-indexable-urls-in-sitemap':['Nicht indexierbare URLs in Sitemap','saved_page'],
    'urls-in-multiple-sitemaps':['URLs in mehreren Sitemaps','saved_page'],
    'xml-sitemap-with-over-50k-urls':['XML-Sitemap mit über 50.000 URLs','source_attempt'],
    'xml-sitemap-over-50mb':['XML-Sitemap über 50 MiB','source_attempt']
  };
  const findingsAppEngine='open-seo-spider/app-sitemap-findings-v1';
  const findingsCoreEngine='open-seo-spider/full-stored-sitemap-findings-v1';
  const findingsSection=()=>state.section==='sitemap-findings';
  const findingsObject=v=>Boolean(v && typeof v==='object' && !Array.isArray(v));
  const findingsHash=v=>typeof v==='string' && /^[a-f0-9]{64}$/.test(v);
  const findingsInteger=v=>Number.isSafeInteger(v) && v>=0;
  const findingsOpaque=v=>typeof v==='string' && v.length>0 && v.length<=4096 && /^[A-Za-z0-9_-]+$/.test(v);
  const findingsVerdict=v=>({match:'Erfüllt',not_matched:'Nicht erfüllt',unknown:'Unbekannt'}[v] || v);
  const findingsValue=v=>v==null?'Unbekannt':typeof v==='number'&&!Number.isSafeInteger(v)?'Außerhalb sicherer Browser-Zahlengenauigkeit':String(v);
  function findingsQuantity(v) {
    if (!v || !['exact','at_least'].includes(v.precision) || !findingsInteger(v.value)) return 'Unbekannt';
    return `${v.precision==='at_least'?'≥ ':''}${findingsValue(v.value)}${v.precision==='exact'?' · exakt':' · Untergrenze'}`;
  }
  function findingsQuery() {
    const filter=$('sitemap-findings-filter').value, source=findingsFilters[filter]?.[1]==='source_attempt';
    const exact=$('sitemap-findings-exact').value, search=$('sitemap-findings-search').value;
    // Keep field order identical to Rust SitemapFindingSelection serialization.
    return {filter,assessment:$('sitemap-findings-assessment').value,requested_url:!source&&exact!==''?exact:null,source_task_url:source&&exact!==''?exact:null,search:search!==''?search:null};
  }
  function findingsSelectionText(query) {
    const text=JSON.stringify(query);
    // Rust strings cannot contain lone UTF-16 surrogates. Do not replace them.
    for (const value of Object.values(query)) if(typeof value==='string') for(let i=0;i<value.length;i++) {
      const code=value.charCodeAt(i);
      if(code>=0xd800&&code<=0xdbff){if(i+1>=value.length || value.charCodeAt(++i)<0xdc00 || value.charCodeAt(i)>0xdfff)throw Error('Ungültiges Unicode-Zeichen in der Auswahl.');}
      else if(code>=0xdc00&&code<=0xdfff)throw Error('Ungültiges Unicode-Zeichen in der Auswahl.');
    }
    return text;
  }
  async function findingsSelectionHash(text) {
    const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(text));
    return [...new Uint8Array(digest)].map(byte=>byte.toString(16).padStart(2,'0')).join('');
  }
  function invalidateFindingsRequest(clear=true) {
    sitemapFindings.revision++;
    sitemapFindings.token?.controller.abort();sitemapFindings.download?.controller.abort();
    sitemapFindings.token=null;sitemapFindings.download=null;
    if(clear)Object.assign(sitemapFindings,{result:null,cursor:null,back:[],selected:null,error:'',downloadError:'',downloadBlocked:false,queryDirty:false});
  }
  function resetSitemapFindings() { invalidateFindingsRequest();if(findingsSection())renderSitemapFindings(); }
  function findingsOwns(token,kind='token') {
    return sitemapFindings[kind]===token && token.revision===sitemapFindings.revision && token.id===state.crawl?.id && token.generation===state.generation && token.serverGeneration===(state.crawl?.generation??null) && findingsSection() && token.selectionText===JSON.stringify(findingsQuery()) && token.limit===Number($('sitemap-findings-limit').value) && token.candidateLimit===Number($('sitemap-findings-candidates').value);
  }
  function findingsToken() {
    const query=findingsQuery();
    return {id:state.crawl.id,generation:state.generation,serverGeneration:state.crawl.generation??null,revision:sitemapFindings.revision,query,selectionText:findingsSelectionText(query),limit:Number($('sitemap-findings-limit').value),candidateLimit:Number($('sitemap-findings-candidates').value),controller:new AbortController()};
  }
  async function findingsBytes(response,max,signal) {
    const declared=response.headers.get('content-length');
    if(declared!==null && (!/^\d+$/.test(declared) || Number(declared)>max)){await response.body?.cancel();throw Error('Antwort überschreitet die zulässige Byte-Grenze.');}
    if(!response.body)throw Error('Antwort enthält keinen lesbaren Body.');
    const reader=response.body.getReader(),chunks=[];let size=0;
    try {
      while(true){if(signal.aborted)throw new DOMException('Abgebrochen','AbortError');const part=await reader.read();if(part.done)break;size+=part.value.byteLength;if(size>max)throw Error('Antwort überschreitet die zulässige Byte-Grenze.');chunks.push(part.value);}
      if(declared!==null && Number(declared)!==size)throw Error('Unvollständige Antwort: Byte-Länge stimmt nicht.');
      return chunks;
    } catch(error) { await reader.cancel().catch(()=>{});throw error; }
    finally {reader.releaseLock();}
  }
  function findingsText(chunks) {const decoder=new TextDecoder('utf-8',{fatal:true});return chunks.map(chunk=>decoder.decode(chunk,{stream:true})).join('')+decoder.decode();}
  function findingsHTTPError(body,status) {const error=Error(typeof body?.error==='string'?body.error:`HTTP ${status}`);error.status=status;error.code=body?.code;return error;}
  async function findingsJSON(response,signal) {
    if(!(response.headers.get('content-type')||'').includes('application/json'))throw Error('Befund-API liefert keine unterstützte JSON-Antwort.');
    const body=JSON.parse(findingsText(await findingsBytes(response,8*1024*1024,signal)));
    if(!response.ok)throw findingsHTTPError(body,response.status);
    return body;
  }
  function validateFindingsEnvelope(body,token,expected=null) {
    const reject=message=>{throw Error(message);}, target=body?.access_target;
    if(!findingsObject(body)||body.engine!==findingsAppEngine||body.website_requests!==0||!findingsObject(target)||target.kind!=='job'||target.job_id!==token.id||!findingsInteger(target.generation)||!findingsHash(target.config_sha256))reject('Nicht unterstützter Befundstand oder falscher Crawl.');
    if(token.serverGeneration!==null && target.generation!==token.serverGeneration)reject('Befunde gehören zu einer anderen Crawl-Generation.');
    if(body.availability_reason!==null && typeof body.availability_reason!=='string')reject('Ungültige Verfügbarkeitsangabe.');
    if(body.findings===null){if(body.next_cursor!==null||body.export_token!==null||!body.availability_reason)reject('Nicht verfügbare Daten dürfen keine Cursor oder Exporte behaupten.');return;}
    if(token.serverGeneration===null)reject('Die Crawl-Generation ist noch nicht verfügbar. Stand neu laden.');
    const page=body.findings,ctx=page?.context,scan=page?.scan,basis=ctx?.basis,source=token.query.source_task_url!==null||findingsFilters[token.query.filter][1]==='source_attempt';
    if(!findingsObject(page)||!findingsObject(ctx)||ctx.engine!==findingsCoreEngine||ctx.selection_sha256!==token.selectionHash||!findingsHash(ctx.context_sha256)||!['available','available_with_unknown_history','unavailable'].includes(ctx.availability)||!Array.isArray(ctx.reasons)||!ctx.reasons.every(v=>typeof v==='string')||!findingsObject(basis)||basis.crawl_config_sha256!==target.config_sha256||basis.generation!==(target.generation===0?null:target.generation))reject('Befundkontext passt nicht zur tatsächlichen Auswahl oder Generation.');
    if(expected && (ctx.context_sha256!==expected.context || body.export_token!==expected.exportToken))reject('Die gespeicherte Auswahl wurde während der Seitennavigation geändert.');
    if(!findingsObject(scan)||!findingsInteger(scan.evaluated_candidates)||scan.evaluated_candidates>token.candidateLimit||typeof scan.exhausted!=='boolean'||!['exhausted','more_matches','candidate_budget','unavailable'].includes(scan.stop)||![null,true,false].includes(scan.more_matches))reject('Ungültiger Suchfortschritt.');
    if(source && (!findingsObject(ctx.source_scope)||typeof ctx.source_scope.unknown_history!=='boolean'||typeof ctx.source_scope.measurement_history_available!=='boolean'||typeof ctx.source_scope.current_task_inventory_available!=='boolean'||typeof ctx.source_scope.inherited!=='boolean'))reject('Quellenhistorie ist nicht unterstützt.');
    if(!source && ctx.source_scope!==null)reject('Quelldomäne passt nicht zur Seitenauswahl.');
    if(ctx.availability==='unavailable') {
      if(page.rows!==null||page.continuation!==null||body.next_cursor!==null||body.export_token!==null||scan.stop!=='unavailable'||scan.exhausted||scan.more_matches!==null)reject('Nicht verfügbare Befunde dürfen kein leeres Gesamtergebnis behaupten.');return;
    }
    if(!Array.isArray(page.rows)||page.rows.length>token.limit||!findingsOpaque(body.export_token)||body.availability_reason!==null)reject('Ungültige Befundseite oder Exportbindung.');
    if(scan.stop==='unavailable')reject('Verfügbarer Kontext hat einen widersprüchlichen Suchstand.');
    if(page.continuation===null) {
      if(body.next_cursor!==null||!scan.exhausted||scan.stop!=='exhausted'||scan.more_matches!==false)reject('Fehlender Fortsetzungscursor bei unvollständiger Suche.');
    } else {
      const continuation=page.continuation;
      if(!findingsObject(continuation)||!findingsOpaque(body.next_cursor)||body.next_cursor===token.cursor||scan.exhausted||continuation.context_sha256!==ctx.context_sha256||continuation.reason!==scan.stop||!['candidate_budget','more_matches'].includes(scan.stop)||(scan.stop==='candidate_budget'?scan.more_matches!==null:scan.more_matches!==true))reject('Ungültige Befundfortsetzung.');
      validateFindingsPosition(continuation.after,source,reject);
    }
    const positions=new Set();
    for(const row of page.rows) {
      if(!findingsObject(row)||!findingsObject(row.assessment)||!['match','not_matched','unknown'].includes(row.assessment.verdict)||token.query.assessment!=='all'&&row.assessment.verdict!==token.query.assessment||!Array.isArray(row.assessment.reasons)||!row.assessment.reasons.every(v=>typeof v==='string')||!findingsObject(row.evidence)||row.evidence.domain!==(source?'source_attempt':'saved_page'))reject('Befundzeile passt nicht zur Auswahl.');
      validateFindingsPosition(row.position,source,reject);
      const position=JSON.stringify(row.position);if(positions.has(position))reject('Doppelte Befundposition.');positions.add(position);
      if(source)validateFindingsSource(row.evidence.source,reject);
      else {const candidate=row.evidence.page,finding=candidate?.finding;if(!findingsObject(candidate)||!findingsObject(finding)||!findingsObject(finding.observation)||typeof finding.observation.requested_url!=='string'||!findingsObject(finding.observed_inlinks))reject('Gespeicherter Seitenbefund ist beschädigt.');}
    }
  }
  function validateFindingsPosition(position,source,reject) {
    if(!findingsObject(position)||!['saved_page','source_measurement','legacy_source_document','unmeasured_source_task'].includes(position.kind)||(position.kind!=='saved_page')!==source)reject('Falsche Befundcursor-Rolle.');
    const value=position.ordinal??position.sequence;if(!findingsInteger(value)||position.kind!=='saved_page'&&value===0)reject('Ungültige Befundposition.');
  }
  function validateFindingsSource(source,reject) {
    if(!findingsObject(source)||!['measured','legacy_document','unmeasured_task'].includes(source.kind))reject('Nicht unterstützte Quellenbeobachtung.');
    if(source.kind==='measured') {
      const record=source.record,protocol=source.protocol;
      if(!findingsObject(record?.measurement?.payload?.binding)||typeof record.measurement.payload.binding.requested_task_url!=='string'||!findingsObject(protocol)||protocol.measurement_id!==record.measurement.measurement_id)reject('Messung und Protokollprüfung sind nicht gebunden.');
      for(const field of ['entry_limit','decoded_xml_byte_limit']) {
        const check=protocol[field],value=check?.proof_quantity;
        if(!findingsObject(check)||!['within_limit','observed_exceeded','unknown'].includes(check.verdict)||typeof check.basis!=='string'||!findingsInteger(check.threshold)||!findingsObject(value)||!['exact','at_least','unavailable'].includes(value.precision)||(value.precision==='unavailable'?value.value!==null:!findingsInteger(value.value)))reject('Nicht unterstützte Protokollmessung oder Genauigkeit.');
      }
    } else if(typeof(source.kind==='legacy_document'?source.document?.task_url:source.task?.task_url)!=='string')reject('Originale Quellenidentität fehlt.');
  }
  function findingsReasonText(values) {return values.length?values.join(' · '):'Keine weiteren Gründe';}
  function findingsSourceIdentity(source) {return source.kind==='measured'?source.record.measurement.payload.binding.requested_task_url:source.kind==='legacy_document'?source.document.task_url:source.task.task_url;}
  function findingsCells(row) {
    const assessment=row.assessment, prefix=[findingsVerdict(assessment.verdict),findingsReasonText(assessment.reasons)];
    if(row.evidence.domain==='saved_page') {
      const finding=row.evidence.page.finding,observation=finding.observation;
      return [observation.requested_url,...prefix,finding.membership,findingsValue(finding.original_membership_occurrences),findingsValue(finding.original_source_document_count),observation.state,findingsValue(observation.final_status),findingsValue(finding.observed_inlinks.nonself_anchor_occurrences),observation.age_scope];
    }
    const source=row.evidence.source,p=source.protocol;
    return [findingsSourceIdentity(source),...prefix,({measured:'Messung',legacy_document:'Historisches Dokument',unmeasured_task:'Aufgabe ohne Messung'}[source.kind]),p?findingsQuantity(p.entry_limit.proof_quantity):'Unbekannt',p?findingsQuantity(p.decoded_xml_byte_limit.proof_quantity):'Unbekannt',p?.decode_integrity??'Unbekannt',p?.membership_retention??'Unbekannt',source.kind==='measured'?(source.association?'Exakte Commit-Zuordnung':'A-Messung ohne B-Commit'):'Keine Messzuordnung'];
  }
  function renderSitemapFindings() {
    const current=sitemapFindings,body=current.result,page=body?.findings,ctx=page?.context,rows=page?.rows??[],busy=Boolean(current.token),ready=Boolean(body&&page&&ctx.availability!=='unavailable'&&!busy&&!current.error),source=findingsFilters[$('sitemap-findings-filter').value][1]==='source_attempt';
    $('sitemap-findings-view').setAttribute('aria-busy',String(busy));
    $('sitemap-findings-error').hidden=!current.error;$('sitemap-findings-error').textContent=current.error;
    $('sitemap-findings-download-error').hidden=!current.downloadError;$('sitemap-findings-download-error').textContent=current.downloadError;
    $('sitemap-findings-summary').textContent=!state.crawl?'Einen gespeicherten Crawl auswählen.':busy?'Gespeicherte Befunde werden geprüft …':current.error?'Befunde konnten nicht sicher geladen werden.':current.queryDirty?'Auswahl geändert. Mit „Auswahl anwenden“ laden.':!body?'Noch keine Befunde geladen.':!page?`Befunde nicht verfügbar: ${body.availability_reason}`:ctx.availability==='unavailable'?`Gespeicherte Analyse nicht verfügbar: ${findingsReasonText(ctx.reasons)}`:`${rows.length} Zeilen in diesem Suchfenster · ${page.scan.evaluated_candidates} gespeicherte Kandidaten geprüft · ${page.scan.exhausted?'gespeicherte Auswahl durchlaufen':'weitere gespeicherte Kandidaten vorhanden'}`;
    let scope='Filter und CSV gelten für alle passenden gespeicherten Kandidaten. Keine vollständige Website- oder Sitemap-Entdeckung wird behauptet. Suche: wörtlich und mit Groß-/Kleinschreibung auf Original-URL bzw. Original-Aufgabe.';
    if(ctx){scope+=` Stand: ${ctx.availability}. ${ctx.reasons.length?findingsReasonText(ctx.reasons)+'. ':''}`;if(ctx.basis.inherited)scope+='Geerbte Beobachtungen aus dem Elternlauf; kein aktueller Discoverylauf. ';if(ctx.source_scope){const s=ctx.source_scope;scope+=`Messhistorie: ${s.measurement_history_available?'vorhanden':'unbekannt'}; unbekannte Vorgeschichte: ${s.unknown_history?'ja':'nein'}; Auslassung: ${s.omission??'keine aufgezeichnet'}; aktuelles Aufgabeninventar: ${s.current_task_inventory_available?'verfügbar':'unbekannt'}. `;}scope+='Neue Website-Anfragen beim Lesen: 0.';}
    $('sitemap-findings-scope').textContent=scope;
    $('sitemap-findings-exact-label').textContent=source?'Exakte Original-Aufgaben-URL (optional)':'Exakte ursprüngliche Anfrage-URL (optional)';
    $('sitemap-findings-next').disabled=!ready||!body.next_cursor;$('sitemap-findings-prev').disabled=!ready||!current.back.length;
    $('sitemap-findings-refresh').disabled=!state.crawl;
    $('sitemap-findings-export').disabled=!ready||!body.export_token||Boolean(current.download)||current.downloadBlocked;
    $('sitemap-findings-export-cancel').hidden=!current.download;
    $('sitemap-findings-export').textContent=current.download?'CSV wird vollständig übertragen …':'↓ Alle gefilterten gespeicherten Befunde CSV';
    $('sitemap-findings-page-label').textContent=page?(page.scan.stop==='candidate_budget'?'Arbeitsfenster erreicht – auch ohne Treffer mit „Weiter“ fortsetzen.':page.scan.stop==='more_matches'?'Weitere passende Befunde vorhanden.':page.scan.stop==='exhausted'?'Gespeicherte Auswahl durchlaufen.':'Analyse nicht verfügbar.'):'Keine geladene Auswahl';
    const headers=source?['Original-Aufgabe','Befund','Gründe','Quellenevidenz','Protokoll-Einträge','Dekodierte XML-Bytes','Integrität','Mitgliedschafts-Retention','Commit-Zuordnung']:['Ursprüngliche Anfrage-URL','Befund','Gründe','Mitgliedschaft','Original-Vorkommen','Verschiedene Original-Aufgaben','Zielbefund','HTTP final','Beobachtete Nicht-Selbst-Inlinks','Beobachtungsalter'];
    const table=$('sitemap-findings-table');table.querySelector('thead').innerHTML=`<tr>${headers.map(h=>`<th scope="col">${escape(h)}</th>`).join('')}</tr>`;
    table.querySelector('tbody').innerHTML=rows.map((row,index)=>`<tr tabindex="0" data-finding-row="${index}" aria-label="Befund ${index+1} öffnen">${findingsCells(row).map(cell=>`<td>${escape(cell??'Unbekannt')}</td>`).join('')}</tr>`).join('');
    table.querySelectorAll('[data-finding-row]').forEach(element=>{const open=()=>{current.selected=Number(element.dataset.findingRow);renderFindingsInspector();$('sitemap-findings-inspector-heading').focus({preventScroll:true});$('sitemap-findings-inspector').scrollIntoView({block:'nearest'});};element.addEventListener('click',open);element.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();open();}});});
    renderFindingsInspector();
  }
  function renderFindingsInspector() {
    const row=sitemapFindings.result?.findings?.rows?.[sitemapFindings.selected];$('sitemap-findings-inspector').hidden=!row;if(!row)return;
    const details=[['Ergebnis',findingsVerdict(row.assessment.verdict)],['Gründe',findingsReasonText(row.assessment.reasons)]];
    if(row.evidence.domain==='source_attempt') {const source=row.evidence.source,p=source.protocol;details.push(['Original-Aufgabe',findingsSourceIdentity(source)],['Evidenztyp',source.kind]);if(p)for(const [label,key] of [['URL-/Eintragsgrenze','entry_limit'],['Dekodierte XML-Byte-Grenze','decoded_xml_byte_limit']])details.push([label,`${p[key].verdict} · ${findingsQuantity(p[key].proof_quantity)} · Grenze ${p[key].threshold} · ${p[key].basis}`]);if(p)details.push(['Decode-Integrität',p.decode_integrity],['XML-Scan',p.xml_scan_status],['Mitgliedschafts-Retention',p.membership_retention],['Commit-Zuordnung',source.association?'Exakte B-Zuordnung gespeichert':'Keine B-Zuordnung; Messung beweist keine Mitgliedschaft']);}
    if(row.evidence.domain==='source_attempt' && row.evidence.source.kind==='measured') {
      const payload=row.evidence.source.record.measurement.payload,t=payload.transport,decode=payload.decode,scan=payload.scan;
      details.push(['HTTP-Anfrage gestartet',findingsValue(t?.request_started)],['HTTP final',findingsValue(t?.final_status)],['Transport-Body vollständig',findingsValue(t?.body_complete)],['Nach HTTP-Dekodierung empfangene Bytes',findingsQuantity(t?.post_http_received_bytes)],['Vollständige dekodierte XML-Bytes',findingsQuantity(decode?.decoded_xml_bytes)],['Decode-Abbruch',decode?.stop_code??'Keiner aufgezeichnet'],['Scan-Abbruch',scan?.stop_code??'Keiner aufgezeichnet']);
      for(const [label,key] of [['Begonnene unmittelbare Einträge','immediate_entry_starts'],['Abgeschlossene unmittelbare Einträge','completed_immediate_entries'],['Protokollgeeignete abgeschlossene Einträge','protocol_eligible_completed_entries'],['Tatsächliche loc-Vorkommen im Dokumentnamensraum','actual_document_namespace_loc_occurrences']])details.push([label,findingsQuantity(scan?.counts?.[key])]);
    }
    $('sitemap-findings-inspector-details').innerHTML=details.map(([key,value])=>`<dt>${escape(key)}</dt><dd>${escape(value)}</dd>`).join('');
    $('sitemap-findings-inspector-data').textContent=JSON.stringify(row,null,2);
  }
  async function loadSitemapFindings(cursor=null,navigation=null) {
    if(!state.crawl||!findingsSection()||sitemapFindings.token)return;
    const previous=sitemapFindings.result,oldCursor=sitemapFindings.cursor,expected=navigation?{context:previous?.findings?.context.context_sha256,exportToken:previous?.export_token}:null;
    sitemapFindings.download?.controller.abort();sitemapFindings.download=null;sitemapFindings.downloadError='';sitemapFindings.downloadBlocked=false;sitemapFindings.queryDirty=false;
    let token;try{token=findingsToken();}catch(error){sitemapFindings.error=error.message;renderSitemapFindings();return;}
    token.cursor=cursor;sitemapFindings.token=token;sitemapFindings.result=null;sitemapFindings.selected=null;sitemapFindings.error='';renderSitemapFindings();
    try {
      token.selectionHash=await findingsSelectionHash(token.selectionText);if(!findingsOwns(token))return;
      const payload={...token.query,limit:token.limit,candidate_limit:token.candidateLimit,...(cursor?{cursor}:{})};
      const result=await findingsJSON(await fetch(`/api/crawls/${encodeURIComponent(token.id)}/sitemaps/findings`,{method:'POST',headers:{'Content-Type':'application/json',Accept:'application/json'},body:JSON.stringify(payload),signal:token.controller.signal}),token.controller.signal);
      if(!findingsOwns(token))return;validateFindingsEnvelope(result,token,expected);
      if(navigation==='next')sitemapFindings.back.push(oldCursor);else if(navigation==='back')sitemapFindings.back.pop();
      sitemapFindings.result=result;sitemapFindings.cursor=cursor;
    } catch(error) {if(findingsOwns(token)){sitemapFindings.result=null;sitemapFindings.cursor=null;sitemapFindings.back=[];sitemapFindings.error=error.name==='AbortError'?'Laden abgebrochen.':`${error.status?`HTTP ${error.status} · `:''}${error.code?error.code+' · ':''}${error.message}`;}}
    finally {if(sitemapFindings.token===token){sitemapFindings.token=null;if(findingsSection())renderSitemapFindings();}}
  }
  async function exportSitemapFindings() {
    const body=sitemapFindings.result;if(!body?.export_token||sitemapFindings.token||sitemapFindings.download||sitemapFindings.downloadBlocked||sitemapFindings.error||!findingsSection()||!state.crawl)return;
    let token;try{token=findingsToken();}catch(error){sitemapFindings.downloadError=error.message;renderSitemapFindings();return;}
    token.result=body;sitemapFindings.download=token;sitemapFindings.downloadError='';renderSitemapFindings();
    try {
      const response=await fetch(`/api/crawls/${encodeURIComponent(token.id)}/sitemaps/findings/export.csv`,{method:'POST',headers:{'Content-Type':'application/json',Accept:'text/csv'},body:JSON.stringify({...token.query,snapshot:body.export_token}),signal:token.controller.signal});
      if(!response.ok){await findingsJSON(response,token.controller.signal);throw Error(`HTTP ${response.status}`);}
      if(!(response.headers.get('content-type')||'').toLowerCase().startsWith('text/csv'))throw Error('CSV-API liefert keinen unterstützten CSV-Body.');
      const chunks=await findingsBytes(response,64*1024*1024,token.controller.signal);
      if(!findingsOwns(token,'download')||sitemapFindings.result!==token.result)return;
      downloadBlob(new Blob(chunks,{type:'text/csv;charset=utf-8'}),`open-seo-spider-${token.id}-sitemap-findings.csv`);
    } catch(error) {if(findingsOwns(token,'download')){sitemapFindings.downloadBlocked=true;sitemapFindings.downloadError=error.name==='AbortError'?'CSV-Übertragung abgebrochen.':`${error.status?`HTTP ${error.status} · `:''}${error.code?error.code+' · ':''}${error.message} · Vor erneutem Export den Stand neu laden.`;}}
    finally {if(sitemapFindings.download===token){sitemapFindings.download=null;if(findingsSection())renderSitemapFindings();}}
  }
  function bindSitemapFindingsEvents() {
    const reset=()=>{resetSitemapFindings();};
    $('sitemap-findings-filter').addEventListener('change',()=>{$('sitemap-findings-exact').value='';reset();loadSitemapFindings();});
    for(const id of ['assessment','limit','candidates'])$('sitemap-findings-'+id).addEventListener('change',()=>{reset();loadSitemapFindings();});
    // Invalidate synchronously while editing; submit performs one exact query.
    for(const id of ['search','exact'])$('sitemap-findings-'+id).addEventListener('input',()=>{reset();sitemapFindings.queryDirty=true;renderSitemapFindings();});
    $('sitemap-findings-form').addEventListener('submit',event=>{event.preventDefault();reset();loadSitemapFindings();});
    $('sitemap-findings-refresh').addEventListener('click',()=>{reset();loadSitemapFindings();});
    $('sitemap-findings-next').addEventListener('click',()=>{if(sitemapFindings.result?.next_cursor)loadSitemapFindings(sitemapFindings.result.next_cursor,'next');});
    $('sitemap-findings-prev').addEventListener('click',()=>{if(sitemapFindings.back.length)loadSitemapFindings(sitemapFindings.back.at(-1),'back');});
    $('sitemap-findings-export').addEventListener('click',exportSitemapFindings);
    $('sitemap-findings-export-cancel').addEventListener('click',()=>{const token=sitemapFindings.download;if(token){token.controller.abort();sitemapFindings.download=null;sitemapFindings.downloadError='CSV-Übertragung abgebrochen.';renderSitemapFindings();}});
    $('sitemap-findings-inspector-close').addEventListener('click',()=>{const index=sitemapFindings.selected;sitemapFindings.selected=null;renderFindingsInspector();$('sitemap-findings-table').querySelector(`[data-finding-row="${index}"]`)?.focus({preventScroll:true});});
  }

  const sitemapMembership = { result:null, token:null, revision:0, cursor:null, back:[], pageCursor:null,pageBack:[],selectedPage:null,error:'', stale:false, selected:null, sort:0, direction:1 };
  const sitemapEngine = 'open-seo-spider/sitemap-membership-ledger-v1';
  const sitemapSection = () => state.section === 'sitemap-membership';
  const sitemapKind = () => $('sitemap-membership-kind').value;
  const sitemapLimit = () => Number($('sitemap-membership-limit').value);
  const sitemapValue = value => value == null ? 'Nicht erfasst' : value === '' ? '[leer]' : typeof value === 'boolean' ? value ? 'Ja' : 'Nein' : String(value);
  const sitemapDecisionLabel = value => ({ unbound:'Noch nicht zugeordnet', invalid_location:'Ungültige URL', missing_location:'URL fehlt', incomplete_source:'Quelle unvollständig', outside_scope:'Außerhalb Crawl-Bereich', filter_denied:'Filter verweigert', source_check_only:'Quelle nur geprüft', policy_denied:'Policy verweigert', coalesced:'Zusammengeführt', scheduled:'Eingeplant' }[value] || value || 'Nicht erfasst');
  const sitemapKindLabel = value => ({ root:'Deklaration', document:'Quelle', entry:'XML-Eintrag', location:'XML-URL' }[value] || value);
  const sitemapRootLabel = value => ({ explicit_config:'Explizite Eingabe', imported_sitemap_row:'Importzeile', robots_directive:'robots.txt' }[value] || value || 'Nicht erfasst');
  const sitemapLastmod = fields => fields.length ? fields.map(value=>value.decoded_value===''?'[leere Deklaration]':value.decoded_value).join(' | ') : 'Keine lastmod-Deklaration';

  const sitemapLastmodEngine = 'sitemap-lastmod/xsd-1.0-second-edition+w3c-note-1998/v1';
  const sitemapXsdProfile = 'sitemap-0.9/tLastmod/xsd-1.0-second-edition/date|dateTime';
  const sitemapW3cProfile = 'W3C-NOTE-datetime-19980827/six-precision-levels';
  const sitemapCoverageEngine = 'open-seo-spider/declared-sitemap-source-coverage-v1';
  const sitemapLastmodPresence = value => ({ absent_optional:'Optional, nicht deklariert', unobserved_incomplete_entry:'Unbekannt: Eintrag unvollständig', observed_one:'Eine Deklaration', observed_multiple:'Mehrere Deklarationen' }[value] || 'Prüfung nicht verfügbar');
  const sitemapLastmodReason = value => ({ empty:'Leerer Wert', lexical_grammar:'Format nicht zulässig', year_zero:'Jahr 0 nicht zulässig', invalid_calendar_date:'Kalenderdatum ungültig', invalid_time:'Uhrzeit ungültig', invalid_timezone:'Zeitzone ungültig', leap_second_needs_external_evidence:'Schaltsekunde benötigt externe Belege', w3c_year_zero_not_specified:'Jahr 0 im W3C-Profil nicht festgelegt' }[value] || value);
  const sitemapLastmodPrecision = value => ({ year:'Jahr',month:'Monat',date:'Datum',minute:'Minute',second:'Sekunde',fractional_second:'Sekundenbruchteil' }[value] || value);
  const sitemapCoverageReasons = {
    catalog_unavailable:'Katalog nicht verfügbar',catalog_rows_unavailable:'Katalogzeilen nicht verfügbar',legacy_history_unknown:'Frühere Quellenhistorie unbekannt',source_inventory_unavailable:'Quelleninventar nicht verfügbar',historical_inventory_unavailable:'Historisches Quelleninventar nicht verfügbar',discovery_not_initialized:'Quellenermittlung nicht initialisiert',initial_root_inventory_unknown:'Ursprüngliche Deklarationen unbekannt',missing_initial_root_declaration:'Ursprüngliche Deklaration fehlt',unexpected_initial_root_declaration:'Unerwartete ursprüngliche Deklaration',initial_root_coordinates_differ:'Koordinaten ursprünglicher Deklarationen weichen ab',source_task_missing:'Quellenaufgabe fehlt',pending_source:'Quellenaufgabe noch offen',done_source_without_observation:'Abgeschlossene Quellenaufgabe ohne Beobachtung',observation_without_task:'Quellenbeobachtung ohne Aufgabe',source_generation_after_inventory:'Quellengeneration nach Inventarstand',source_not_complete:'Quelle nicht vollständig',source_request_not_started:'Quellenanfrage nicht begonnen',source_transport_unavailable:'Transportdaten nicht verfügbar',source_http_not200:'Quellenstatus ist nicht HTTP 200',source_body_incomplete:'Quellenantwort unvollständig',source_blocked_by_robots:'Quelle durch robots.txt blockiert',source_transport_failed:'Quellentransport fehlgeschlagen',source_decode_unavailable:'Quellendekodierung nicht verfügbar',source_parser_incomplete:'Quellenparser unvollständig',source_declarations_incomplete:'Quellendeklarationen unvollständig',source_rows_incomplete:'Gespeicherte Quellenzeilen unvollständig',source_failed:'Quelle fehlgeschlagen',root_invalid:'Sitemap-Deklaration ungültig',root_denied:'Sitemap-Deklaration nicht zugelassen',root_unbound:'Sitemap-Deklaration nicht zugeordnet',child_invalid:'Kind-Sitemap ungültig',child_denied:'Kind-Sitemap nicht zugelassen',child_unbound:'Kind-Sitemap nicht zugeordnet',source_graph_unbound_task:'Nicht zugeordnete Aufgabe im Quellenverbund',required_robots_inventory_unknown:'Erforderliche robots.txt-Quellen unbekannt',required_robots_task_missing:'Erforderliche robots.txt-Aufgabe fehlt'
  };
  function sitemapAssessment(tagged) {
    if(tagged.kind!=='entry')return null;
    return sitemapMembership.result?.lastmod_assessments?.find(value=>value.entry_id===tagged.row.entry_id && value.observation_id===tagged.row.observation_id && value.entry_sequence===tagged.row.sequence) || null;
  }
  function sitemapProfileSummary(assessment,profile) {
    if(!assessment)return null;
    if(!assessment.fields.length)return assessment.presence==='absent_optional'?'Nicht deklariert (optional)':'Unbekannt: Eintrag unvollständig';
    return assessment.fields.map(field=>{const value=field[profile];return value.status==='valid'?'Gültig · '+sitemapLastmodPrecision(value.value.precision):value.status==='invalid'?'Ungültig · '+sitemapLastmodReason(value.reason):'Unbestimmt · '+sitemapLastmodReason(value.reason);}).join(' | ');
  }
  function sitemapParsedDetails(value) {
    return [sitemapLastmodPrecision(value.precision),'Jahr: '+value.year,value.month==null?'':'Monat: '+value.month,value.day==null?'':'Tag: '+value.day,value.hour==null?'':'Stunde: '+value.hour,value.minute==null?'':'Minute: '+value.minute,value.second==null?'':'Sekunde: '+value.second,value.fraction==null?'':'Exakte Bruchstellen: '+value.fraction,value.timezone_lexeme==null?'Zeitzone nicht deklariert':'Originale Zeitzone: '+value.timezone_lexeme+' (Offset '+value.timezone_offset_minutes+' Minuten)',value.end_of_day?'24:00:00 bezeichnet den Beginn des Folgetags':''].filter(Boolean).join(' · ');
  }
  function sitemapValueAssessmentDetails(value) {
    return value.status==='valid'?'Gültig · '+sitemapParsedDetails(value.value):value.status==='invalid'?'Ungültig · '+sitemapLastmodReason(value.reason):'Unbestimmt · '+sitemapLastmodReason(value.reason)+' · '+sitemapParsedDetails(value.lexical_value);
  }
  function renderSitemapDerivedCoverage(result,busy,error) {
    const derived=!busy && !error?result?.coverage?.derived:null;
    const label=derived?.scope==='inherited_historical_declared_sources'?'Historisch erfasste deklarierte Sitemap-Quellen':derived?.scope==='current_declared_sources'?'Aktuell erfasste deklarierte Sitemap-Quellen':'Deklarierte Sitemap-Quellen';
    $('sitemap-membership-coverage-summary').textContent=busy?'Quellenabdeckung wird geladen …':error?'Quellenabdeckung nicht verfügbar.':!derived?'Noch keine abgeleitete Quellenprüfung verfügbar.':label+' · '+({complete:'rekursiver Quellenverbund vollständig erfasst',known_empty:'bekannt leerer Quellenverbund',unknown:'Vollständigkeit unbekannt'}[derived.completion]);
    $('sitemap-membership-coverage-detail').textContent=!derived?'Diese Prüfung verwendet ausschließlich gespeicherte Quellen.':derived.scope==='unavailable'?'Die ursprünglichen Quellen sind nicht verfügbar; das ist kein bekannter leerer Quellenverbund.':[
      'Die Prüfung gilt für den erfassten Verbund deklarierter Sitemap-Quellen. Die vollständige Website-Abdeckung bleibt unbekannt.',
      derived.scope==='inherited_historical_declared_sources'?'Elternhistorie; die Quellen wurden in diesem Lauf nicht neu ermittelt.':'',
      derived.current_valid_original_location_negative_permitted?'Für diesen aktuellen Quellenverbund kann das Fehlen einer gültigen ursprünglichen Seiten-Loc geprüft werden.':derived.historical_valid_original_location_negative_permitted?'Für diesen historischen Quellenverbund kann das Fehlen einer gültigen ursprünglichen Seiten-Loc geprüft werden.':'Eine fehlende ursprüngliche Seiten-Loc bleibt unbekannt.',
      'Ungültige Locs und zugelassene Abrufaliase sind keine gültigen ursprünglichen Seiten-Locs. lastmod prüft deklarierte Datumswerte; Aktualität und Ziel-Antwortstatus werden nicht geprüft.'
    ].filter(Boolean).join(' ');
    const counts=derived?.counts,items=counts?[['Sitemap-Deklarationen',counts.root_declarations],['Kind-Sitemap-Deklarationen',counts.child_declarations],['Erfasste Quellenaufgaben',counts.discovery_tasks],['Beobachtete Quellen',counts.observed_documents],['Vollständig erfasste Quelleinheiten',counts.complete_documents],['Erreichte XML-Sitemap-Aufgaben',counts.reached_xml_tasks],['Ungültige / fehlende Seiten-Locs',counts.invalid_or_missing_page_locations]]:[];
    $('sitemap-membership-coverage-counts').innerHTML=items.map(([name,count])=>'<dt>'+escape(name)+'</dt><dd>'+escape(number(count))+'</dd>').join('');
    $('sitemap-membership-coverage-reasons').textContent=derived?Object.entries(derived.reason_counts).map(([reason,count])=>sitemapCoverageReasons[reason]+': '+number(count)).join(' · ') || 'Keine Lücken im erfassten Quellenverbund festgestellt.':'';
  }
  function invalidateSitemapRequest() {
    sitemapMembership.revision += 1;
    sitemapMembership.token?.controller.abort();
    sitemapMembership.token = null;
  }
  function resetSitemapMembership() {
    resetSourceBodies();
    invalidateSitemapRequest();
    Object.assign(sitemapMembership,{result:null,cursor:null,back:[],pageCursor:null,pageBack:[],selectedPage:null,error:'',stale:false,selected:null,sort:0,direction:1});
    if ($('sitemap-membership-search')) { $('sitemap-membership-search').value=''; $('sitemap-membership-filter').value='all'; }
    if (sitemapSection()) renderSitemapMembership();
  }
  function sitemapDocumentComplete(row) {
    const transport=row.transport;
    if(row.failure_code!=null || row.retained_rows_complete!==true || !transport || transport.request_started!==true || transport.body_complete!==true || transport.final_status!==200 || transport.blocked_by_robots!==false || transport.failure_code!=null || !transport.requested_url || !transport.final_url)return false;
    let requested=row.task_url;
    if(row.kind==='robots_directives'){try{requested=new URL(row.task_url).origin+'/robots.txt';}catch(_){return false;}}
    if(transport.requested_url!==requested)return false;
    return row.kind==='robots_directives' ? row.declaration_scan_complete===true && row.declaration_utf8_lossy===false : row.parser?.complete===true && row.parser.failure==null && row.entity_decode!=null;
  }
  function sitemapRowFacts(tagged) {
    const row=tagged.row;
    if(tagged.kind==='root')return {kind:sitemapRootLabel(row.kind),original:row.raw_value,membership:row.admission.membership_url,admitted:row.admission.admitted_fetch_url,decision:row.admission.decision,ordinal:row.source_ordinal,source:row.source_observation_id,diagnostic:null};
    if(tagged.kind==='location')return {kind:row.observed.role==='child_sitemap'?'XML-Kind-Sitemap':'XML-Seiten-URL',original:row.observed.decoded_value,membership:row.admission.membership_url,admitted:row.admission.admitted_fetch_url,decision:row.admission.decision,ordinal:row.observed.ordinal,source:row.observation_id,diagnostic:row.observed.validation_error};
    if(tagged.kind==='document')return {kind:row.kind==='robots_directives'?'robots.txt-Quelle':'XML-Sitemap-Quelle',original:row.task_url,membership:null,admitted:null,decision:null,ordinal:null,source:row.observation_id,diagnostic:row.failure_code || row.transport?.failure_code || row.parser?.failure?.code || null,complete:sitemapDocumentComplete(row)};
    return {kind:'XML-Eintrag',original:sitemapLastmod(row.observed.lastmod),membership:null,admitted:null,decision:null,ordinal:row.observed.ordinal,source:row.observation_id,diagnostic:row.observed.diagnostics.join(' | ') || null,complete:row.observed.complete};
  }
  function sitemapEntryLocations(tagged) {
    if(tagged.kind!=='entry')return null;
    const row=tagged.row,expected=array(row.observed.loc_occurrence_ordinals),ordinals=new Set(expected);
    const observed=array(sitemapMembership.result?.rows).filter(candidate=>candidate.kind==='location' && candidate.row.observation_id===row.observation_id && candidate.row.observed.entry_ordinal===row.observed.ordinal && ordinals.has(candidate.row.observed.ordinal)).sort((a,b)=>a.row.observed.ordinal-b.row.observed.ordinal);
    const values=observed.map(candidate=>candidate.row.admission.membership_url??candidate.row.observed.decoded_value??'[Loc nicht deklariert]');
    const scope=observed.length?`${number(observed.length)} von ${number(expected.length)} gespeicherten Loc-Vorkommen auf dieser geladenen Seite${row.observed.complete?'':' · XML-Eintrag unvollständig'}.`:expected.length?'Loc-Zuordnung nicht auf dieser geladenen Seite verfügbar.':row.observed.complete?'Kein zugeordnetes Loc-Vorkommen im gespeicherten XML-Eintrag.':'Kein zugeordnetes Loc-Vorkommen beobachtet; XML-Eintrag unvollständig.';
    return {values,scope};
  }
  const sitemapRawColumns = [
    ['Position',tagged=>tagged.row.sequence],['Vorkommen',tagged=>sitemapRowFacts(tagged).kind],['Originalwert',tagged=>sitemapRowFacts(tagged).original],['Ursprüngliche Mitgliedschaft',tagged=>sitemapRowFacts(tagged).membership],['Zugelassene Anfrage-URL',tagged=>sitemapRowFacts(tagged).admitted],['Zulassung / Quelle',tagged=>{const facts=sitemapRowFacts(tagged);return facts.decision?sitemapDecisionLabel(facts.decision):facts.complete===true?'Quelleinheit vollständig':facts.complete===false?'Quelleinheit unvollständig':'Nicht erfasst';}],['Quellordinal',tagged=>sitemapRowFacts(tagged).ordinal],['Quellenhinweis',tagged=>sitemapRowFacts(tagged).diagnostic]
  ];
  const sitemapTargetLabels={blocked_by_robots:'Durch robots.txt blockiert',redirected:'Weiterleitung',non200:'Antwort ungleich 200',non_indexable:'Nicht indexierbar',fetch_failed:'Abruf fehlgeschlagen',incomplete_observation:'Beobachtung unvollständig',indexable:'Indexierbar',no_response:'Keine HTTP-Antwort erfasst',pending:'Im gespeicherten Stand ausstehend',no_stored_requested_observation:'Kein gespeicherter Abruf dieser Original-URL',invalid_or_missing_original_location:'Original-URL ungültig / fehlend',rendered_navigation_changed:'Browser zu anderer URL navigiert',rendered_non200:'Browserantwort ungleich 200'};
  const sitemapRenderingLabels={not_configured:'Browserdarstellung nicht konfiguriert',not_applicable:'Browserdarstellung für diese Antwort nicht vorgesehen',required_but_unobserved:'Konfigurierte Browserdarstellung nicht beobachtet',failed:'Browserdarstellung fehlgeschlagen',url_unobserved:'Browser-End-URL nicht beobachtet',url_unusable:'Browser-End-URL nicht als HTTP-Identität auswertbar',response_unobserved:'Browserantwortstatus nicht beobachtet',non200:'Browserantwort ungleich 200',navigation_changed:'Browser zu anderer URL navigiert',observed_200:'Browserantwort 200 für dieselbe native End-URL beobachtet'};
  const sitemapPageMembershipLabels={listed_original:'Als Original-URL gelistet',not_listed_current_declared_sources:'Nicht in erfassten deklarierten Quellen gelistet',not_listed_historical_declared_sources:'Nicht in historischen Elternquellen gelistet',unknown:'Mitgliedschaft unbekannt'};
  function sitemapTargetFinding(tagged){return tagged.kind==='location'?sitemapMembership.result?.membership_analysis?.location_findings?.find(value=>value.sequence===tagged.row.sequence && value.occurrence_id===tagged.row.occurrence_id && value.observation_id===tagged.row.observation_id)??null:null;}
  const sitemapColumns = [...sitemapRawColumns,['lastmod-Vorkommen',tagged=>sitemapAssessment(tagged)?sitemapLastmodPresence(sitemapAssessment(tagged).presence):null],['lastmod · XSD-Union',tagged=>sitemapProfileSummary(sitemapAssessment(tagged),'xsd')],['lastmod · W3C-Profil',tagged=>sitemapProfileSummary(sitemapAssessment(tagged),'w3c_note')],['Originale Locs · nur geladene Seite',tagged=>{const locations=sitemapEntryLocations(tagged);return locations?locations.values.length?locations.values.join(' | '):locations.scope:null;}],['Original-Ziel · gespeicherter Stand',tagged=>sitemapTargetLabels[sitemapTargetFinding(tagged)?.original_state]??null],['HTTP zuerst / zuletzt',tagged=>{const page=sitemapTargetFinding(tagged)?.original_observation;return page?`${page.status??'—'} / ${page.final_status??'—'}`:null;}],['Beobachtete Inlinks ohne Selbstlinks',tagged=>sitemapTargetFinding(tagged)?.observed_original_inlinks?.nonself_anchor_occurrences??null]];
  function sitemapUsable() { return Boolean(sitemapMembership.result?.catalog_available && !sitemapMembership.token && !sitemapMembership.error); }
  function sitemapVisibleRows() {
    if(!sitemapUsable())return [];
    const search=$('sitemap-membership-search').value.toLocaleLowerCase(), filter=$('sitemap-membership-filter').value;
    return sitemapMembership.result.rows.filter(tagged=>{
      const facts=sitemapRowFacts(tagged);
      if(filter==='alias' && !(facts.membership && facts.admitted && facts.membership!==facts.admitted))return false;
      if(filter==='denied' && !(facts.decision && !['scheduled','coalesced'].includes(facts.decision)))return false;
      if(filter==='invalid' && !['invalid_location','missing_location'].includes(facts.decision))return false;
      if(filter==='incomplete' && !(facts.complete===false || facts.decision==='incomplete_source'))return false;
      if(filter.startsWith('target-') && sitemapTargetFinding(tagged)?.original_state!==filter.slice(7))return false;
      if(filter==='listed-no-observed-inlinks' && sitemapTargetFinding(tagged)?.observed_original_inlinks?.nonself_anchor_occurrences!==0)return false;
      const assessment=sitemapAssessment(tagged);
      if(filter.startsWith('lastmod-')) {
        if(!assessment)return false;
        if(filter==='lastmod-invalid' && !assessment.fields.some(field=>field.xsd.status==='invalid'))return false;
        if(filter==='lastmod-disagreement' && !assessment.findings.some(finding=>finding.code==='schema_profile_disagreement'))return false;
        if(filter==='lastmod-undetermined' && !assessment.fields.some(field=>field.xsd.status==='undetermined' || field.w3c_note.status==='undetermined'))return false;
        if(filter==='lastmod-multiple' && assessment.presence!=='observed_multiple')return false;
        if(filter==='lastmod-absent' && assessment.presence!=='absent_optional')return false;
        if(filter==='lastmod-incomplete' && assessment.all_entry_fields_observed!==false)return false;
      }
      return !search || [facts.kind,facts.original,facts.membership,facts.admitted,facts.diagnostic,assessment?sitemapLastmodPresence(assessment.presence):null,sitemapProfileSummary(assessment,'xsd'),sitemapProfileSummary(assessment,'w3c_note')].some(value=>String(value??'').toLocaleLowerCase().includes(search));
    }).sort((left,right)=>{const value=sitemapColumns[sitemapMembership.sort][1],a=value(left),b=value(right);if(a==null && b==null)return 0;if(a==null)return 1;if(b==null)return -1;return (typeof a==='number' && typeof b==='number'?a-b:String(a).localeCompare(String(b),'de',{numeric:true}))*sitemapMembership.direction;});
  }
  function renderSitemapMembership() {
    if(!sitemapSection())return;
    const current=sitemapMembership, result=current.result, summary=result?.summary, busy=Boolean(current.token), usable=sitemapUsable(), rows=sitemapVisibleRows();
    $('sitemap-membership-error').hidden=!current.error; $('sitemap-membership-error').textContent=current.error;
    $('sitemap-membership-summary').textContent=!state.crawl?'Einen gespeicherten Crawl auswählen.':busy?'Gespeicherte Sitemap-Seite wird geladen …':current.error?'Sitemap-Stand nicht verfügbar. Dies ist kein vollständiges leeres Ergebnis.':!result?'Noch kein gespeicherter Sitemap-Stand geladen.':!result.catalog_available?'Keine verfügbare ursprüngliche Sitemap-Aufzeichnung. Historische oder importierte Berichte werden nicht nachträglich als leer rekonstruiert.':`${number(summary.root_rows)} Deklarationen · ${number(summary.document_rows)} Quellen · ${number(summary.entry_rows)} XML-Einträge · ${number(summary.location_rows)} XML-URL-Vorkommen · ${number(summary.incomplete_documents)} unvollständige Quellen`;
    $('sitemap-membership-scope').textContent=result?.catalog_available ? ['Gespeicherte Vorkommen verfügbar. Vollständige Website-Abdeckung nicht bestätigt.',summary.legacy_history_unknown?'Frühere Quellenhistorie unbekannt.':'',result.inherited?'Geerbter Stand des Elternlaufs; Sitemap-Ermittlung wurde in diesem Lauf nicht neu ausgeführt.':'',summary.incomplete_documents?`${number(summary.incomplete_documents)} Quelleinheiten sind unvollständig; ihre URLs beweisen keine vollständige Mitgliedschaft.`:'',result.membership_analysis?.context.available?'Zielstatus und Inlinks stammen aus dem gebundenen gespeicherten Bericht. Original-, Rewrite- und Redirect-Identitäten bleiben getrennt.':'Zielstatus und Inlinks sind für diesen Stand nicht verfügbar.'].filter(Boolean).join(' '):'Diese Ansicht liest gespeicherte Beobachtungen und startet keine Website-Anfrage.';
    renderSitemapDerivedCoverage(result,busy,current.error);
    renderSitemapTargetAnalysis();
    const table=$('sitemap-membership-table');
    table.querySelector('thead').innerHTML=`<tr>${sitemapColumns.map(([label],index)=>`<th scope="col" aria-sort="${current.sort===index?current.direction===1?'ascending':'descending':'none'}"><button type="button" data-sitemap-sort="${index}">${escape(label)} ${current.sort===index?current.direction===1?'↑':'↓':'↕'}</button></th>`).join('')}</tr>`;
    const empty=busy?'Sitemap-Seite wird geladen …':current.error?'Gespeicherte Beobachtungen konnten nicht geladen werden.':!state.crawl?'Kein Crawl ausgewählt.':!result?.catalog_available?'Ursprüngliche Quellen nicht verfügbar; keine Aussage über leere Sitemaps.':!result.rows.length?'Dieser gespeicherte Stand enthält hier keine Vorkommen.':!rows.length?'Keine passenden Vorkommen auf dieser geladenen Seite. Weitere gespeicherte Seiten können andere Befunde enthalten.':'';
    table.querySelector('tbody').innerHTML=rows.length?rows.map((tagged,index)=>`<tr data-sitemap-row="${index}" tabindex="0" aria-label="Details zu ${escape(sitemapRowFacts(tagged).kind)}, Position ${tagged.row.sequence}">${sitemapColumns.map(([,value],column)=>`<td${column===2?' class="sitemap-raw"':''}>${escape(sitemapValue(value(tagged)))}</td>`).join('')}</tr>`).join(''):`<tr><td class="empty-cell" colspan="${sitemapColumns.length}">${escape(empty)}</td></tr>`;
    table.querySelectorAll('[data-sitemap-sort]').forEach(button=>button.addEventListener('click',()=>{const column=Number(button.dataset.sitemapSort);current.direction=current.sort===column?-current.direction:1;current.sort=column;renderSitemapMembership();}));
    table.querySelectorAll('[data-sitemap-row]').forEach(element=>{const open=()=>{current.selected=rows[Number(element.dataset.sitemapRow)];renderSitemapInspector();$('sitemap-membership-inspector').scrollIntoView({block:'nearest'});};element.addEventListener('click',open);element.addEventListener('keydown',event=>{if(['Enter',' '].includes(event.key)){event.preventDefault();open();}});});
    $('sitemap-membership-prev').disabled=busy || current.stale || !current.back.length;
    $('sitemap-membership-next').disabled=!usable || !result.has_more;
    $('sitemap-membership-refresh').disabled=busy || !state.crawl;
    $('sitemap-membership-limit').disabled=!state.crawl;
    $('sitemap-membership-export-page').disabled=!usable || !rows.length;
    $('sitemap-membership-export-csv').disabled=!usable;
    $('sitemap-membership-page-label').textContent=`${number(rows.length)} gefilterte von ${result?.rows==null?'unbekannt':number(result.rows.length)} geladenen Zeilen · maximal ${sitemapLimit()}${current.stale?' · Stand geändert, aktuellen Stand neu laden':''}`;
    $('sitemap-membership-checkpoint').textContent=JSON.stringify({catalog_available:result?.catalog_available??null,snapshot:result?.snapshot??null,summary:summary??null,inherited:result?.inherited??null,availability_reason:result?.availability_reason??null,coverage:result?.coverage??null,lastmod_assessment_engine:result?.lastmod_assessment_engine??null},null,2);
    renderSitemapInspector();
  }
  function sitemapPageAgeLabel(value){return {captured_checkpoint_history:'Gespeicherte Checkpoint-Historie',selected_current_generation:'Ausgewählte URL in dieser Generation erfasst',selected_prior_generation:'Ausgewählte URL in früherer Generation erfasst',inherited_parent_observation:'Geerbte Elternbeobachtung'}[value]??'Alter nicht bestimmt';}
  function renderSitemapTargetAnalysis(){
    const current=sitemapMembership,analysis=current.result?.membership_analysis,context=analysis?.context,usable=sitemapUsable() && context?.available,rows=usable?analysis.saved_pages:[],search=$('sitemap-analysis-search').value.toLocaleLowerCase(),filter=$('sitemap-analysis-filter').value;
    const matches=value=>{if(filter==='all')return true;if(filter==='multiple-documents')return value.original_source_document_count>1;if(filter==='listed-no-links')return value.listed_without_observed_nonself_inlinks;if(filter==='not-listed')return ['not_listed_current_declared_sources','not_listed_historical_declared_sources'].includes(value.membership);if(filter==='unknown')return value.membership==='unknown';return value.observation.state===filter;};
    const visible=rows.filter(value=>(!search || [value.observation.requested_url,value.observation.final_url,...value.observation.indexability_reasons].some(v=>v.toLocaleLowerCase().includes(search))) && matches(value));
    const reasons={terminal_saved_report_unavailable:'Kein gebundener abgeschlossener Bericht verfügbar.',validated_source_catalog_unavailable:'Ursprünglicher Quellenkatalog nicht verfügbar.',live_crawl_target_analysis_unavailable:'Der Crawl läuft. Zielstatus und Inlinks werden nach dem nächsten gespeicherten Abschluss verfügbar.',saved_checkpoint_context_mismatch:'Der Bericht passt nicht mehr zum gespeicherten Checkpoint. Keine Zielauswertung für diesen Stand.'};
    $('sitemap-analysis-summary').textContent=current.token?'Gespeicherte Ziele werden geladen …':current.error?'Zielauswertung nicht verfügbar.':!context?'Für diesen API-Stand ist keine gespeicherte Zielauswertung verfügbar.':!context.available?reasons[context.reason]??'Zielauswertung für diesen Stand nicht verfügbar.':`${number(context.saved_page_count)} gespeicherte Seiten · ${number(context.pending_page_count)} ausstehende Seiten · ${context.observation_scope==='mixed_age_selected_saved_report'?'Ausgewählter Wiederholungslauf mit verschieden alten Beobachtungen':context.observation_scope==='partial_terminal_saved_report_observations'?'Unvollständiger gespeicherter Lauf':'Abgeschlossener gespeicherter Lauf'}`;
    $('sitemap-analysis-scope').textContent=context?.available?[context.source_scope==='inherited_historical_declared_sources'?'Die Sitemap-Zuordnung stammt aus den historischen Quellen des Elternlaufs.':'Die Sitemap-Zuordnung bezieht sich auf die erfassten deklarierten Quellen.','„Nicht gelistet“ ist nur bei vollständig erfasstem Quellenverbund bestimmbar. Inlinks zählen gespeicherte Anchor-Vorkommen auf die exakte ursprüngliche Anfrage-URL; Selbstlinks sind gesondert gezählt.','Keine beobachteten Inlinks beweisen keine verwaiste Seite der gesamten Website. Geerbte Beobachtungen bestätigen keine heutige Aktualität. Suche und Filter gelten für diese geladene Seitenauswahl.'].join(' '):'Diese Auswertung liest ausschließlich gespeicherte Fakten und startet keine Website-Anfrage.';
    $('sitemap-analysis-table').querySelector('tbody').innerHTML=visible.length?visible.map((value,index)=>{const page=value.observation;return `<tr data-sitemap-page="${index}" tabindex="0"><td>${escape(page.requested_url)}</td><td>${escape(sitemapPageMembershipLabels[value.membership])}</td><td>${number(value.original_membership_occurrences)}</td><td>${number(value.original_source_document_count)}</td><td>${escape(sitemapTargetLabels[page.state])}</td><td>${page.status??'—'} / ${page.final_status??'—'}</td><td>${escape(page.final_url||'Nicht erfasst')}</td><td>${number(value.observed_inlinks.nonself_anchor_occurrences)}</td><td>${escape(value.listed_without_observed_nonself_inlinks?'Keine im Bericht beobachtet':'—')}</td><td>${escape(sitemapPageAgeLabel(page.age_scope))}</td></tr>`;}).join(''):`<tr><td colspan="10" class="empty-cell">${escape(!usable?'Keine verfügbare gespeicherte Zielauswertung.':rows.length?'Keine passenden Seiten in dieser geladenen Auswahl.':'Keine gespeicherten Seiten in dieser Auswahl.')}</td></tr>`;
    $('sitemap-analysis-table').querySelectorAll('[data-sitemap-page]').forEach(element=>{const open=()=>{current.selectedPage=visible[Number(element.dataset.sitemapPage)];renderSitemapSavedPageInspector();};element.addEventListener('click',open);element.addEventListener('keydown',event=>{if(['Enter',' '].includes(event.key)){event.preventDefault();open();}});});
    $('sitemap-analysis-prev').disabled=!usable || !current.pageBack.length;
    $('sitemap-analysis-next').disabled=!usable || !analysis.saved_pages_have_more;
    $('sitemap-analysis-limit').disabled=Boolean(current.token) || !state.crawl;
    $('sitemap-analysis-page-label').textContent=`${number(visible.length)} gefilterte von ${number(rows.length)} geladenen gespeicherten Seiten`;
    renderSitemapSavedPageInspector();
  }
  function renderSitemapSavedPageInspector(){
    const value=sitemapMembership.selectedPage,section=$('sitemap-analysis-inspector');section.hidden=!value || !sitemapUsable();if(section.hidden)return;
    const page=value.observation,links=value.observed_inlinks;
    $('sitemap-analysis-inspector-heading').textContent=page.requested_url;
    $('sitemap-analysis-inspector-fields').innerHTML=[['Sitemap-Zuordnung',sitemapPageMembershipLabels[value.membership]],['Ursprüngliche Loc-Vorkommen',value.original_membership_occurrences],['Verschiedene erfasste Sitemap-Dokumente',value.original_source_document_count],['Getrennte Quellbeobachtungen',value.original_source_observation_count],['Sitemap-Quellbeispiele',value.original_source_document_samples.map(source=>source.task_url).join(' · ')],['Gesonderte zugelassene Alias-Verweise',value.admitted_alias_occurrences],['Gespeicherter Zielzustand',sitemapTargetLabels[page.state]],['Indexierbarkeitshinweise',page.indexability_reasons.join(' · ')],['Browserdarstellung',sitemapRenderingLabels[page.rendering_state]],['Gesonderte Browser-End-URL',page.rendered_url],['Gesonderter Browserstatus',page.rendered_status],['Beobachtete Anchor-Inlinks inkl. Selbstlinks',links.anchor_occurrences],['Beobachtete Inlinks ohne Selbstlinks',links.nonself_anchor_occurrences],['Davon interne Anchor-Vorkommen',links.internal_nonself_anchor_occurrences],['Verschiedene beobachtete Quellen ohne Selbstlinks',links.distinct_nonself_requested_sources],['Inlink-Beispiele',`${links.samples.length}${links.samples_have_more?' · weitere Vorkommen gezählt':''}`],['Zeitliche Herkunft',sitemapPageAgeLabel(page.age_scope)],['Gespeicherte Abrufzeit',page.fetched_at]].map(([label,content])=>`<dt>${escape(label)}</dt><dd>${escape(sitemapValue(content))}</dd>`).join('');
    $('sitemap-analysis-inspector-data').textContent=JSON.stringify(value,null,2);
  }
  function renderSitemapInspector() {
    if(sourceBodies.documentId!==selectedSourceBodyDocument())resetSourceBodies();
    const tagged=sitemapMembership.selected, inspector=$('sitemap-membership-inspector');
    inspector.hidden=!tagged || !sitemapUsable(); if(inspector.hidden)return;
    const row=tagged.row, facts=sitemapRowFacts(tagged);
    $('sitemap-membership-inspector-heading').textContent=`${facts.kind} · Position ${row.sequence}`;
    $('sitemap-membership-inspector-basis').textContent=tagged.kind==='root'?'Originale Deklaration und ihre eigene Abrufzulassung. Explizite Eingaben und robots.txt-Zeilen zählen ab 0.':tagged.kind==='location'?'XML-Original und dekodierter Wert bleiben getrennt. XML-Vorkommen zählen ab 1. Eine zugelassene Alias-URL ersetzt die ursprüngliche Mitgliedschaft nicht.':tagged.kind==='entry'?'Originale XML-Felder bleiben unverändert. lastmod wird getrennt gegen die Sitemap-XSD-Union und das W3C-Datumsprofil geprüft. Die Prüfung bestätigt keine Aktualität und keinen Ziel-Antwortstatus.':'Gespeicherte Quellen- und Transportfakten. Fehlende Werte bleiben nicht erfasst; die eigene Quelleinheit bestimmt keine vollständige rekursive Abdeckung.';
    const fields=[['Originalwert',facts.original],['Ursprüngliche Mitgliedschaft',facts.membership],['Zugelassene Anfrage-URL',facts.admitted],['Zulassungsgrund',facts.decision?sitemapDecisionLabel(facts.decision):null],['Quellordinal',facts.ordinal],['Quellhinweis',facts.diagnostic]];
    if(tagged.kind==='root')fields.push(['Originalzeile',row.raw_line],['Robots-Zeilen vollständig',row.robots_evidence?.line_complete],['Ursprüngliche Quellzeile',row.robots_evidence?.line_ordinal]);
    if(tagged.kind==='location')fields.push(['Originales XML-Element',row.observed.raw_element_xml],['Originaler XML-Inhalt',row.observed.raw_inner_xml],['XML-Eintragsordinal',row.observed.entry_ordinal],['Loc-Ordinal',row.observed.loc_ordinal],['Rolle',row.observed.role]);
    if(tagged.kind==='document')fields.push(['Anfrage begonnen',row.transport?.request_started],['Tatsächliche Anfrage-URL',row.transport?.requested_url],['Beobachtete finale URL',row.transport?.final_url],['Erster HTTP-Status',row.transport?.initial_status],['Finaler HTTP-Status',row.transport?.final_status],['Antwort vollständig',row.transport?.body_complete],['Parser vollständig',row.parser?.complete],['Quelleinheit vollständig',facts.complete]);
    if(tagged.kind==='entry'){
      const locations=sitemapEntryLocations(tagged);
      fields.push(['XML-Eintrag vollständig',row.observed.complete],['lastmod-Deklarationen',sitemapLastmod(row.observed.lastmod)],['XML-Diagnosen',row.observed.diagnostics.join(' | ')],['Originale Locs · nur geladene Seite',locations.values.length?locations.values.join(' | '):'Nicht auf dieser geladenen Seite verfügbar'],['Loc-Zuordnung · nur geladene Seite',locations.scope]);
    }
    const assessment=sitemapAssessment(tagged);
    if(assessment) {
      fields.push(['lastmod-Vorkommen',sitemapLastmodPresence(assessment.presence)],['Alle lastmod-Felder des Eintrags beobachtet',assessment.all_entry_fields_observed],['Schema-Prüfung','Sitemap 0.9 · XSD 1.0 date | dateTime'],['Separate Profilprüfung','W3C-Datumsprofil mit sechs Präzisionsstufen'],['lastmod · XSD-Union',sitemapProfileSummary(assessment,'xsd')],['lastmod · W3C-Profil',sitemapProfileSummary(assessment,'w3c_note')],['Aktualität geprüft',assessment.freshness_assessed],['Ziel-Antwortstatus geprüft',assessment.http_status_assessed]);
      for(const field of assessment.fields)fields.push(['lastmod '+field.observed.ordinal+' · Originalwert',field.observed.decoded_value],['lastmod '+field.observed.ordinal+' · Ausgewerteter Wert (XML-Whitespace)',field.evaluated_value],['lastmod '+field.observed.ordinal+' · XSD-Union',sitemapValueAssessmentDetails(field.xsd)],['lastmod '+field.observed.ordinal+' · W3C-Profil',sitemapValueAssessmentDetails(field.w3c_note)]);
    }
    $('sitemap-membership-inspector-assessment-section').hidden=!assessment;
    $('sitemap-membership-inspector-assessment').textContent=assessment?JSON.stringify(assessment,null,2):'';
    const target=sitemapTargetFinding(tagged);
    $('sitemap-membership-inspector-target-section').hidden=!target;
    $('sitemap-membership-inspector-target').textContent=target?JSON.stringify(target,null,2):'';
    if(target)fields.push(['Original-Ziel · gespeicherter Zustand',sitemapTargetLabels[target.original_state]],['Beobachtete Original-Inlinks ohne Selbstlinks',target.observed_original_inlinks?.nonself_anchor_occurrences??null],['Original-Ziel nach Weiterleitung',target.original_observation?.final_url??null],['Gesonderter Browserbefund',sitemapRenderingLabels[target.original_observation?.rendering_state]??null],['Gesonderte Browser-End-URL',target.original_observation?.rendered_url??null],['Gesonderter Browserstatus',target.original_observation?.rendered_status??null],['Gesonderter zugelassener Alias-Abruf',target.admitted_alias_observation?.requested_url??(target.admitted_alias_pending?'Im gespeicherten Stand ausstehend':null)]);
    $('sitemap-membership-inspector-fields').innerHTML=fields.map(([label,value])=>`<dt>${escape(label)}</dt><dd>${escape(sitemapValue(value))}</dd>`).join('');
    $('sitemap-membership-inspector-data').textContent=JSON.stringify(tagged,null,2);
  }
  function exportSitemapLoadedPage() {
    if(!sitemapUsable())return;
    const rows=sitemapVisibleRows();if(!rows.length)return;
    const result=sitemapMembership.result, columns=[...sitemapRawColumns,['Exportumfang',()=> 'filtered_loaded_page_only'],['Quellenabdeckung vollständig',()=>result.summary.declared_source_cohort_complete],['Frühere Historie unbekannt',()=>result.summary.legacy_history_unknown]];
    const cell=input=>{let value=String(input??'');if(typeof input!=='number' && typeof input!=='boolean' && /^[\s\uFEFF]*[=+@-]/u.test(value))value=`'${value}`;return `"${value.replace(/"/g,'""')}"`;};
    const data=[columns.map(([label])=>cell(label)).join(','),...rows.map(row=>columns.map(([,value])=>cell(value(row))).join(','))].join('\r\n');
    downloadBlob(new Blob(['\uFEFF',data],{type:'text/csv;charset=utf-8'}),`open-seo-spider-${state.crawl.id}-sitemaps-loaded-page.csv`);
    message(`${number(rows.length)} gefilterte Vorkommen der geladenen Sitemap-Seite exportiert.`);
  }

  const sitemapBundleEngine='open-seo-spider/segmented-sitemap-generation-v1';
  const sitemapBundle={ownerId:null,token:null,error:'',status:'',result:null};
  function resetSitemapBundle() {
    sitemapBundle.token?.controller.abort();
    Object.assign(sitemapBundle,{ownerId:null,token:null,error:'',status:'',result:null});
    $('sitemap-bundle-form')?.reset();
  }
  function sitemapBundlePublicBase() {
    const candidates=[...array(state.crawl?.config?.seed_urls),...array(state.crawl?.config?.sitemap_urls),...state.pages.map(page=>page.url)];
    for(const candidate of candidates){try{const url=new URL(candidate);if(['http:','https:'].includes(url.protocol) && !url.username && !url.password)return url.origin+'/';}catch(_){}}
    return '';
  }
  function renderSitemapBundleControls() {
    if(!$('sitemap-bundle-form'))return;
    if(sitemapBundle.ownerId!==state.crawl?.id){
      $('sitemap-bundle-public-base').value=sitemapBundlePublicBase();
      sitemapBundle.ownerId=state.crawl?.id??null;
    }
    const running=Boolean(sitemapBundle.token),ready=Boolean(state.crawl && !active() && !actionBusy());
    $('sitemap-bundle-generate').disabled=!ready || running;
    $('sitemap-bundle-cancel').hidden=!running;
    $('sitemap-bundle-provided-fields').hidden=$('sitemap-bundle-lastmod-source').value!=='provided';
    $('sitemap-bundle-external-images').disabled=!$('sitemap-bundle-images').checked || running;
    $('sitemap-bundle-form').querySelectorAll('input,select,textarea').forEach(input=>{
      if(input.id!=='sitemap-bundle-external-images')input.disabled=running;
    });
    $('sitemap-bundle-selected-crawl').textContent=state.crawl?`Ausgewählter Crawl: ${state.crawl.id}${active()?' · Erzeugung nach dem Anhalten oder Abschluss verfügbar.':''}`:'Einen gespeicherten Crawl auswählen.';
    $('sitemap-bundle-error').hidden=!sitemapBundle.error;
    $('sitemap-bundle-error').textContent=sitemapBundle.error;
    $('sitemap-bundle-status').textContent=sitemapBundle.status || (ready?'Die Datei wird erst nach „Sitemap-ZIP erzeugen“ vorbereitet.':'Einen gespeicherten, angehaltenen oder abgeschlossenen Crawl auswählen.');
    $('sitemap-bundle-result').hidden=!sitemapBundle.result;
    if(sitemapBundle.result){
      const result=sitemapBundle.result,mode={omit:'Ohne Änderungsdatum',captured_sitemap:'Gespeicherte Sitemap-Deklarationen',provided:'Eigene Datumswerte mit Quellenangabe'}[result.options.lastmod.source];
      const fields=[['Gespeicherte URLs im Archiv',number(result.generated)],['Ausgeschlossene Seiten',number(result.excluded)],['Archivgröße',number(result.bytes)+' Bytes'],['Geplantes öffentliches Verzeichnis',result.options.public_base_url],['Änderungsdatum',mode],['Bilddeklarationen',result.options.include_images?(result.options.allow_external_images?'Auch andere Hosts':'Gleicher Host'):'Nicht aufgenommen'],['Sprachalternativen',result.options.include_hreflang?'Aufgenommen, soweit gespeicherte Deklarationen verwendbar sind':'Nicht aufgenommen'],['SHA-256 des Archivs',result.bundleSha],['SHA-256 des gespeicherten Berichts',result.reportSha]];
      if(result.reporting){const view=result.reporting;fields.push(['Berichtsansicht',view.identity.mode],['Erfasste Seiten',number(view.captured_page_observations)],['Nach Berichtsansicht berechtigte Seiten',number(view.eligible_page_observations)],['Durch Berichtsprofil ausgeblendet',number(view.excluded_reporting_page_observations)],['Berichtsidentität SHA-256',view.identity_sha256],['Berechtigte Rohordinal-Identität SHA-256',view.eligible_raw_page_ordinals_sha256]);}
      $('sitemap-bundle-result-fields').innerHTML=fields.map(([label,value])=>`<dt>${escape(label)}</dt><dd>${escape(value)}</dd>`).join('');
    }else{$('sitemap-bundle-result-fields').replaceChildren();}
  }
  function sitemapBundleOptions() {
    const reportingMode=$('sitemap-bundle-reporting-mode').value;
    if(!['','raw','reported'].includes(reportingMode))throw new Error('Eine unterstützte Berichtsansicht auswählen.');
    const integer=(id,label,max)=>{const input=$(id).value;if(!/^[1-9][0-9]*$/.test(input) || !Number.isSafeInteger(Number(input)) || Number(input)>max)throw new Error(`${label}: eine ganze Zahl zwischen 1 und ${number(max)} eingeben.`);return Number(input);};
    const publicBase=$('sitemap-bundle-public-base').value;
    let url;try{url=new URL(publicBase);}catch(_){throw new Error('Ein vollständiges öffentliches HTTP(S)-Verzeichnis eingeben.');}
    if(!['http:','https:'].includes(url.protocol) || url.username || url.password || url.hash || url.search || !url.pathname.endsWith('/') || url.href!==publicBase)throw new Error('Das öffentliche Verzeichnis muss eine unveränderte vollständige HTTP(S)-URL ohne Zugangsdaten, Query oder Fragment sein und mit / enden.');
    const source=$('sitemap-bundle-lastmod-source').value;
    if(!['omit','captured_sitemap','provided'].includes(source))throw new Error('Eine unterstützte Quelle für das Änderungsdatum auswählen.');
    const lastmod={source};
    if(source==='provided'){
      const raw=$('sitemap-bundle-provided-values').value;
      if(!raw.trim())throw new Error('Eigene Datumswerte als drei Tab-getrennte Spalten eingeben: gespeicherte URL, Datum, Quelle.');
      lastmod.values=raw.split(/\r\n|\n|\r/u).filter(line=>line!=='').map((line,index)=>{
        const fields=line.split('\t');
        if(fields.length!==3)throw new Error(`Datumszeile ${index+1}: genau drei Tab-getrennte Spalten verwenden.`);
        const [page_url,value,reference]=fields;
        let page;try{page=new URL(page_url);}catch(_){throw new Error(`Datumszeile ${index+1}: eine vollständige gespeicherte Seiten-URL eingeben.`);}
        if(!['http:','https:'].includes(page.protocol) || page.username || page.password || page.hash || page.href!==page_url || !value.trim() || !reference.trim())throw new Error(`Datumszeile ${index+1}: eine unveränderte gespeicherte HTTP(S)-URL sowie Datum und Quellenangabe eingeben.`);
        return {page_url,value,reference};
      });
      if(!lastmod.values.length)throw new Error('Mindestens eine eigene Datumszeile eingeben.');
    }
    const includeImages=$('sitemap-bundle-images').checked;
    return {
      public_base_url:publicBase,
      ...(reportingMode?{reporting_mode:reportingMode}:{}),
      max_urls_per_file:integer('sitemap-bundle-max-urls','URLs je Datei',50000),
      max_urlset_uncompressed_bytes:integer('sitemap-bundle-max-file-bytes','URL-Sitemap-Größe',52428800),
      max_index_uncompressed_bytes:integer('sitemap-bundle-max-index-bytes','Indexgröße',52428800),
      max_total_output_bytes:integer('sitemap-bundle-max-total-bytes','Gesamte Ausgabe',268435456),
      include_images:includeImages,
      allow_external_images:includeImages && $('sitemap-bundle-external-images').checked,
      include_hreflang:$('sitemap-bundle-hreflang').checked,
      lastmod,
      changefreq:$('sitemap-bundle-changefreq').value || null,
      priority:$('sitemap-bundle-priority').value || null,
    };
  }
  async function validateSitemapBundleReporting(rawReporting,requestedMode,generated,excluded) {
    if(requestedMode!=null && !['raw','reported'].includes(requestedMode))throw new Error('Eine unterstützte Berichtsansicht auswählen.');
    if(rawReporting===null){if(requestedMode!=null)throw new Error('Die ausdrücklich gewählten Berichtsmetadaten fehlen.');return null;}
    if(typeof rawReporting!=='string'||new TextEncoder().encode(rawReporting).byteLength>8192)throw new Error('Die Berichtsmetadaten überschreiten ihre Grenze oder sind ungültig.');
    let value;try{value=JSON.parse(rawReporting);}catch(_){throw new Error('Ungültige Berichtsmetadaten.');}
    const object=value=>value!==null&&typeof value==='object'&&!Array.isArray(value);
    const exact=(value,keys)=>object(value)&&Object.keys(value).length===keys.length&&keys.every(key=>Object.hasOwn(value,key));
    const identity=value?.identity;
    if(!object(value)||!exact(identity,['engine','mode','directive_reporting'])||identity.engine!=='open-seo-spider/sitemap-generation-reporting-v1'||!['raw','reported'].includes(identity.mode)||requestedMode!=null&&identity.mode!==requestedMode)throw new Error('Ungültige Berichtsidentität oder Berichtsansicht.');
    const profile=identity.directive_reporting;
    if(profile!==null && (!exact(profile,['version','respect_noindex','respect_canonical','respect_next_prev'])||profile.version!==1||['respect_noindex','respect_canonical','respect_next_prev'].some(key=>typeof profile[key]!=='boolean')||![profile.respect_noindex,profile.respect_canonical,profile.respect_next_prev].some(flag=>flag===true)))throw new Error('Ungültiges aktives Berichtsprofil.');
    const counts=[generated,excluded,value.captured_page_observations,value.profile_visible_page_observations,value.profile_excluded_page_observations,value.eligible_page_observations,value.excluded_reporting_page_observations,value.generated_pages];
    if(counts.some(n=>!Number.isSafeInteger(n)||n<0||n>1000000)||generated<1||value.captured_page_observations!==generated+excluded||value.profile_visible_page_observations+value.profile_excluded_page_observations!==value.captured_page_observations||value.eligible_page_observations!==(identity.mode==='raw'?value.captured_page_observations:value.profile_visible_page_observations)||value.excluded_reporting_page_observations+value.eligible_page_observations!==value.captured_page_observations||value.generated_pages!==generated||generated>value.eligible_page_observations)throw new Error('Ungültige Berichts-Zählung.');
    if([value.identity_sha256,value.eligible_raw_page_ordinals_sha256,value.generated_raw_page_ordinals_sha256].some(hash=>typeof hash!=='string'||!/^[a-f0-9]{64}$/.test(hash)))throw new Error('Ungültige Berichts-Prüfsumme.');
    if(!globalThis.crypto?.subtle)throw new Error('Dieser Browser kann die Berichtsidentität nicht verifizieren.');
    const digest=[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(JSON.stringify(identity))))].map(byte=>byte.toString(16).padStart(2,'0')).join('');
    if(digest!==value.identity_sha256)throw new Error('Die Berichtsidentität stimmt nicht mit der Prüfsumme überein.');
    return value;
  }
  async function readSitemapBundleStream(response,limit,expectedLength=null) {
    if(!response.body?.getReader)throw new Error('Dieser Browser unterstützt den begrenzten Datei-Download nicht.');
    const reader=response.body.getReader(),chunks=[];let bytes=0;
    try{
      while(true){const part=await reader.read();if(part.done)break;bytes+=part.value.byteLength;if(bytes>limit || expectedLength!==null && bytes>expectedLength)throw new Error('Die Antwort überschreitet die bestätigte Dateigröße; es wird kein Download bereitgestellt.');chunks.push(part.value);}
    }catch(error){await reader.cancel().catch(()=>{});throw error;}finally{reader.releaseLock();}
    if(expectedLength!==null && bytes!==expectedLength)throw new Error('Das Archiv wurde nicht vollständig übertragen; es wird kein Download bereitgestellt.');
    return {blob:new Blob(chunks),bytes};
  }
  async function generateSitemapBundle(event) {
    event.preventDefault();
    if(!state.crawl || active() || actionBusy() || sitemapBundle.token)return;
    sitemapBundle.error='';sitemapBundle.result=null;
    let options;try{options=sitemapBundleOptions();}catch(error){sitemapBundle.error=error.message;renderSitemapBundleControls();return;}
    const token={id:state.crawl.id,generation:state.generation,serverGeneration:state.crawl.generation??null,reportingMode:$('sitemap-bundle-reporting-mode').value,controller:new AbortController()};
    const owns=()=>sitemapBundle.token===token && state.crawl?.id===token.id && state.generation===token.generation && (state.crawl?.generation??null)===token.serverGeneration && $('sitemap-bundle-reporting-mode').value===token.reportingMode && !token.controller.signal.aborted;
    sitemapBundle.token=token;sitemapBundle.status='Sitemap-Dateien werden aus dem gespeicherten Crawl vorbereitet …';renderSitemapBundleControls();
    try{
      const response=await fetch(`/api/crawls/${encodeURIComponent(token.id)}/sitemap-bundle`,{method:'POST',headers:{'Content-Type':'application/json',Accept:'application/zip'},body:JSON.stringify(options),signal:token.controller.signal});
      if(!owns())return;
      if(!response.ok){
        const failure=await readSitemapBundleStream(response,65536),raw=await failure.blob.text();
        let details;try{details=JSON.parse(raw);}catch(_){}
        const message=typeof details?.message==='string'?details.message:typeof details?.error==='string'?details.error:`Sitemap-Dateien konnten nicht erzeugt werden (HTTP ${response.status}).`;
        const reasonLabels={excluded_reporting:'Durch Berichtsprofil ausgeblendet',outside_public_base:'Außerhalb des öffentlichen Verzeichnisses',invalid_page_url:'Ungültige Seiten-URL',invalid_url:'Ungültige Seiten-URL',non_indexable:'Nicht indexierbare Seite',not_indexable:'Nicht indexierbare Seite',non_200:'Antwortstatus ungleich 200',non_200_status:'Antwortstatus ungleich 200',redirected:'Weitergeleitete Seite',duplicate_url:'Doppelte URL',incomplete_page:'Unvollständiger Seitenbefund'};
        const diagnostic=details?.diagnostic_counts && typeof details.diagnostic_counts==='object' && !Array.isArray(details.diagnostic_counts)?Object.entries(details.diagnostic_counts).filter(([name,count])=>/^[a-z0-9_]+$/.test(name) && Number.isSafeInteger(count) && count>0).map(([name,count])=>`${reasonLabels[name] || 'Weitere Auslassung'}: ${number(count)}`).join(' · '):'';
        throw new Error(message+(diagnostic?` Ausgelassene Befunde: ${diagnostic}.`:''));
      }
      const hash=name=>{const value=response.headers.get(name);if(!/^[a-f0-9]{64}$/.test(value??''))throw new Error('Dem Archiv fehlt eine unterstützte Prüfsumme; es wird kein Download bereitgestellt.');return value;};
      const count=(name,positive=false)=>{const raw=response.headers.get(name);if(!/^(0|[1-9][0-9]*)$/.test(raw??'') || !Number.isSafeInteger(Number(raw)) || positive && Number(raw)<1)throw new Error('Die Archivantwort enthält keine unterstützte Dateigröße oder URL-Zählung.');return Number(raw);};
      if(response.headers.get('content-type')?.split(';',1)[0].trim().toLowerCase()!=='application/zip' || response.headers.get('x-sitemap-generation-engine')!==sitemapBundleEngine)throw new Error('Der Server hat kein unterstütztes Sitemap-Archiv geliefert.');
      const reportSha=hash('x-sitemap-report-sha256'),bundleSha=hash('x-sitemap-bundle-sha256'),generated=count('x-sitemap-generated-urls',true),excluded=count('x-sitemap-excluded-pages'),expected=count('content-length',true);
      const reporting=await validateSitemapBundleReporting(response.headers.get('x-sitemap-reporting'),options.reporting_mode??null,generated,excluded);
      if(expected>options.max_total_output_bytes)throw new Error('Das Archiv überschreitet die angegebene Gesamtgröße; es wird kein Download bereitgestellt.');
      const file=await readSitemapBundleStream(response,options.max_total_output_bytes,expected);
      if(!owns())return;
      const bytes=new Uint8Array(await file.blob.arrayBuffer());
      if(bytes.length<4 || bytes[0]!==80 || bytes[1]!==75 || bytes[2]!==3 || bytes[3]!==4)throw new Error('Die Antwort hat keinen unterstützten ZIP-Dateikopf.');
      if(!globalThis.crypto?.subtle)throw new Error('Dieser Browser kann die Archiv-Prüfsumme nicht verifizieren; es wird kein Download bereitgestellt.');
      const actual=[...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(byte=>byte.toString(16).padStart(2,'0')).join('');
      if(actual!==bundleSha)throw new Error('Die Archiv-Prüfsumme stimmt nicht überein; es wird kein Download bereitgestellt.');
      if(!owns())return;
      downloadBlob(new Blob([bytes],{type:'application/zip'}),`open-seo-spider-${token.id}-sitemap-bundle.zip`);
      sitemapBundle.result={options,generated,excluded,bytes:file.bytes,reportSha,bundleSha,...(reporting?{reporting}:{})};
      sitemapBundle.status=`ZIP zum lokalen Download bereitgestellt: ${number(generated)} gespeicherte URLs, ${number(excluded)} ausgeschlossene Seiten. Die Dateien wurden nicht veröffentlicht.`;
    }catch(error){if(owns()){sitemapBundle.error=error.message;sitemapBundle.status='Kein neues Archiv zum Download bereitgestellt.';}}
    finally{if(sitemapBundle.token===token){sitemapBundle.token=null;renderSitemapBundleControls();}}
  }
  function bindSitemapBundleEvents() {
    $('sitemap-bundle-form').addEventListener('submit',generateSitemapBundle);
    $('sitemap-bundle-reporting-mode').addEventListener('change',()=>{sitemapBundle.token?.controller.abort();sitemapBundle.token=null;sitemapBundle.result=null;sitemapBundle.error='';sitemapBundle.status='Berichtsansicht geändert; das nächste Archiv wird neu aus dem gespeicherten Stand erzeugt.';renderSitemapBundleControls();});
    $('sitemap-bundle-lastmod-source').addEventListener('change',renderSitemapBundleControls);
    $('sitemap-bundle-images').addEventListener('change',()=>{if(!$('sitemap-bundle-images').checked)$('sitemap-bundle-external-images').checked=false;renderSitemapBundleControls();});
    $('sitemap-bundle-cancel').addEventListener('click',()=>{if(!sitemapBundle.token)return;sitemapBundle.token.controller.abort();sitemapBundle.token=null;sitemapBundle.result=null;sitemapBundle.error='';sitemapBundle.status='Download abgebrochen. Kein neues Archiv bereitgestellt.';renderSitemapBundleControls();});
  }

  function validateSitemapDerived(result,reject) {
    const object=value=>Boolean(value && typeof value==='object' && !Array.isArray(value)),text=value=>typeof value==='string',integer=value=>Number.isSafeInteger(value) && value>=0,hash=value=>text(value) && /^[a-f0-9]{64}$/.test(value),nullableHash=value=>value===null || hash(value);
    if(result.lastmod_assessment_engine!==sitemapLastmodEngine || !object(result.coverage.derived))reject('Abgeleitete Sitemap-Prüfung ist nicht unterstützt.');
    const derived=result.coverage.derived,counts=derived.counts,basis=derived.basis;
    if(derived.engine!==sitemapCoverageEngine || !['current_declared_sources','inherited_historical_declared_sources','unavailable'].includes(derived.scope) || !['complete','known_empty','unknown'].includes(derived.completion) || typeof derived.current_valid_original_location_negative_permitted!=='boolean' || typeof derived.historical_valid_original_location_negative_permitted!=='boolean' || !object(derived.reason_counts) || Object.entries(derived.reason_counts).some(([name,count])=>!Object.hasOwn(sitemapCoverageReasons,name) || !integer(count) || count<1) || !object(counts) || ['root_declarations','child_declarations','discovery_tasks','observed_documents','complete_documents','reached_xml_tasks','invalid_or_missing_page_locations'].some(name=>!integer(counts[name])) || counts.complete_documents>counts.observed_documents)reject('Abgeleitete Sitemap-Quellenabdeckung ist beschädigt.');
    const permits=derived.completion==='complete' || derived.completion==='known_empty';
    if(derived.current_valid_original_location_negative_permitted!==(permits && derived.scope==='current_declared_sources') || derived.historical_valid_original_location_negative_permitted!==(permits && derived.scope==='inherited_historical_declared_sources') || permits && (Object.keys(derived.reason_counts).length || counts.reached_xml_tasks>counts.discovery_tasks) || !permits && !Object.keys(derived.reason_counts).length || derived.completion==='known_empty' && (counts.root_declarations!==0 || counts.child_declarations!==0 || counts.reached_xml_tasks!==0))reject('Sitemap-Quellenprüfung behauptet eine unbestätigte Negativaussage.');
    if(!result.catalog_available) {
      if(result.lastmod_assessments!==null || derived.scope!=='unavailable' || derived.completion!=='unknown' || basis!==null || Object.values(counts).some(count=>count!==0))reject('Nicht verfügbare Sitemap-Quellen dürfen keine abgeleiteten Einträge liefern.');
      return;
    }
    if(!Array.isArray(result.lastmod_assessments) || !object(basis) || derived.scope!==(result.inherited?'inherited_historical_declared_sources':'current_declared_sources') || basis.initial_config_sha256!==result.summary.config_sha256 || basis.catalog_revision!==result.summary.revision || basis.catalog_rolling_sha256!==result.summary.rolling_sha256 || !nullableHash(basis.captured_inventory_sha256) || !nullableHash(basis.inventory_saved_config_sha256) || basis.inventory_generation!==null && (!integer(basis.inventory_generation) || basis.inventory_generation<1) || (result.inherited?(basis.parent_snapshot_sha256!==result.inherited.parent_snapshot_sha256 || basis.parent_catalog_sha256!==result.inherited.parent_catalog_sha256):(basis.parent_snapshot_sha256!==null || basis.parent_catalog_sha256!==null)))reject('Sitemap-Quellenprüfung ist nicht an den gespeicherten Stand gebunden.');
    if(counts.root_declarations!==result.summary.root_rows || counts.observed_documents!==result.summary.document_rows || counts.child_declarations>result.summary.location_rows || counts.invalid_or_missing_page_locations>result.summary.location_rows || permits && (!hash(basis.captured_inventory_sha256) || !hash(basis.inventory_saved_config_sha256) || basis.inventory_generation===null))reject('Sitemap-Quellenzählung oder vollständiges Inventar passt nicht zum gespeicherten Katalog.');
    const entries=result.rows.filter(tagged=>tagged.kind==='entry'),seen=new Set(),invalidReasons=['empty','lexical_grammar','year_zero','invalid_calendar_date','invalid_time','invalid_timezone'],undeterminedReasons=['leap_second_needs_external_evidence','w3c_year_zero_not_specified'];
    if(result.lastmod_assessments.length!==entries.length)reject('lastmod-Prüfung ist für die geladenen XML-Einträge unvollständig.');
    const parsed=value=>object(value) && ['year','month','date','minute','second','fractional_second'].includes(value.precision) && text(value.year) && /^-?[0-9]+$/.test(value.year) && ['month','day','hour','minute','second'].every(name=>value[name]===null || integer(value[name])) && (value.fraction===null || text(value.fraction) && /^[0-9]+$/.test(value.fraction)) && (value.timezone_lexeme===null?value.timezone_offset_minutes===null:text(value.timezone_lexeme) && Number.isInteger(value.timezone_offset_minutes) && Math.abs(value.timezone_offset_minutes)<=1439) && typeof value.end_of_day==='boolean';
    const assessed=value=>object(value) && (value.status==='valid'?parsed(value.value):value.status==='invalid'?invalidReasons.includes(value.reason):value.status==='undetermined' && undeterminedReasons.includes(value.reason) && parsed(value.lexical_value));
    for(const assessment of result.lastmod_assessments) {
      if(!object(assessment) || !hash(assessment.entry_id) || !hash(assessment.observation_id) || !integer(assessment.entry_sequence) || seen.has(assessment.entry_sequence))reject('lastmod-Prüfung ohne eindeutige Eintragsidentität.');
      seen.add(assessment.entry_sequence);
      const entry=entries.find(tagged=>tagged.row.entry_id===assessment.entry_id && tagged.row.observation_id===assessment.observation_id && tagged.row.sequence===assessment.entry_sequence)?.row;
      if(!entry || assessment.engine!==sitemapLastmodEngine || assessment.xsd_profile!==sitemapXsdProfile || assessment.w3c_note_profile!==sitemapW3cProfile || assessment.entry_ordinal!==entry.observed.ordinal || assessment.entry_xml_byte_start!==entry.observed.xml_byte_start || assessment.entry_xml_byte_end!==entry.observed.xml_byte_end || assessment.entry_complete!==entry.observed.complete || assessment.all_entry_fields_observed!==entry.observed.complete || assessment.freshness_assessed!==false || assessment.http_status_assessed!==false || !Array.isArray(assessment.fields) || assessment.fields.length!==entry.observed.lastmod.length || !Array.isArray(assessment.findings))reject('lastmod-Prüfung passt nicht zum geladenen ursprünglichen XML-Eintrag.');
      const presence=assessment.fields.length>1?'observed_multiple':assessment.fields.length===1?'observed_one':entry.observed.complete?'absent_optional':'unobserved_incomplete_entry';
      if(assessment.presence!==presence)reject('lastmod-Vorkommen passen nicht zum ursprünglichen XML-Eintrag.');
      for(let index=0;index<assessment.fields.length;index++) {
        const field=assessment.fields[index],original=entry.observed.lastmod[index];
        if(!object(field) || !object(field.observed) || ['ordinal','xml_byte_start','xml_byte_end','raw_element_xml','raw_inner_xml','decoded_value'].some(name=>field.observed[name]!==original[name]) || !text(field.evaluated_value) || !assessed(field.xsd) || !assessed(field.w3c_note))reject('lastmod-Feldprüfung passt nicht zum ursprünglichen XML-Feld.');
      }
      for(const finding of assessment.findings) {
        if(!object(finding) || (finding.code==='multiple_lastmod_elements'?finding.observed_count!==assessment.fields.length || finding.observed_count<2:!assessment.fields.some(field=>field.observed.ordinal===finding.field_ordinal) || (finding.code==='invalid_schema_value'?!invalidReasons.includes(finding.reason):finding.code==='schema_value_undetermined'?!undeterminedReasons.includes(finding.reason):finding.code!=='schema_profile_disagreement')))reject('lastmod-Prüfhinweis ist nicht unterstützt.');
      }
    }
  }
  function validateSitemapTargetAnalysis(result,token,reject){
    const analysis=result.membership_analysis;
    if(analysis==null){if(result.next_page_cursor!=null)reject('Seitencursor ohne gespeicherte Zielauswertung.');return;}
    const object=v=>Boolean(v && typeof v==='object' && !Array.isArray(v)),integer=v=>Number.isSafeInteger(v) && v>=0,text=v=>typeof v==='string',hash=v=>text(v) && /^[a-f0-9]{64}$/.test(v),nullableText=v=>v===null || text(v),opaque=v=>text(v) && v.length>0 && v.length<=4096 && /^[A-Za-z0-9_-]+$/.test(v),url=v=>text(v) && v.length>0 && v.length<=65536 && !/[\u0000-\u001f\u007f]/.test(v);
    const context=analysis.context;
    if(!object(analysis) || !object(context) || context.engine!=='open-seo-spider/sitemap-target-inlink-analysis-v1' || typeof context.available!=='boolean' || !text(context.reason) || context.website_wide_orphan_inference!==false || !['current_declared_sources','inherited_historical_declared_sources','unavailable'].includes(context.source_scope) || context.source_scope!==result.coverage.derived?.scope || !nullableText(context.report_sha256) || context.report_sha256!==null && !hash(context.report_sha256) || context.report_bytes!==null && (!integer(context.report_bytes) || context.report_bytes>268435456) || typeof analysis.saved_pages_have_more!=='boolean')reject('Gespeicherte Zielauswertung hat keinen unterstützten gebundenen Kontext.');
    if(!context.available){if(analysis.location_findings!==null || analysis.saved_pages!==null || analysis.saved_pages_have_more || analysis.last_saved_page_url!==null || analysis.last_saved_page_ordinal!==null || result.next_page_cursor!==null)reject('Nicht verfügbare Zielauswertung darf keine Befunde behaupten.');return;}
    if(context.reason!=='validated_terminal_saved_report' || context.generation!==result.snapshot?.crawl_generation || !integer(context.checkpoint_revision) || context.checkpoint_revision<1 || !['completed','paused','cancelled','limited'].includes(context.outcome) || !hash(context.report_sha256) || !integer(context.saved_page_count) || !integer(context.pending_page_count) || !['completed_saved_report_observations','partial_terminal_saved_report_observations','mixed_age_selected_saved_report'].includes(context.observation_scope))reject('Zielauswertung passt nicht zu einem gespeicherten terminalen Bericht.');
    const observation=(v,requested)=>object(v) && v.requested_url===requested && url(v.requested_url) && (v.final_url==='' || url(v.final_url)) && integer(v.report_page_ordinal) && v.report_page_ordinal<context.saved_page_count && [v.status,v.final_status,v.rendered_status].every(s=>s===null || integer(s) && s<=65535) && typeof v.indexable==='boolean' && Array.isArray(v.indexability_reasons) && v.indexability_reasons.every(text) && ['blocked_by_robots','truncated','redirect_chain_present','rendered','rendering_expected','rendered_navigation_changed'].every(k=>typeof v[k]==='boolean') && (v.rendered_url===null || url(v.rendered_url)) && (v.rendered_fetch_url===null || url(v.rendered_fetch_url)) && Object.hasOwn(sitemapRenderingLabels,v.rendering_state) && nullableText(v.error) && nullableText(v.renderer_error) && text(v.fetched_at) && Object.hasOwn(sitemapTargetLabels,v.state) && ['captured_checkpoint_history','selected_current_generation','selected_prior_generation','inherited_parent_observation'].includes(v.age_scope);
    const inlinks=(v,target)=>object(v) && ['anchor_occurrences','nonself_anchor_occurrences','internal_nonself_anchor_occurrences','distinct_nonself_requested_sources'].every(k=>integer(v[k])) && v.nonself_anchor_occurrences<=v.anchor_occurrences && v.internal_nonself_anchor_occurrences<=v.nonself_anchor_occurrences && v.distinct_nonself_requested_sources<=v.nonself_anchor_occurrences && Array.isArray(v.samples) && v.samples.length===Math.min(v.anchor_occurrences,5) && v.samples_have_more===(v.anchor_occurrences>5) && v.samples.every(s=>object(s) && url(s.source_requested_url) && s.target_requested_url===target && integer(s.source_page_ordinal) && s.source_page_ordinal<context.saved_page_count && integer(s.link_ordinal) && text(s.text) && text(s.rel) && typeof s.internal==='boolean' && typeof s.nofollow==='boolean');
    const originalRows=array(result.rows).filter(row=>row.kind==='location' && row.row.observed.role==='page_location');
    if(!Array.isArray(analysis.location_findings) || analysis.location_findings.length!==originalRows.length || !Array.isArray(analysis.saved_pages) || analysis.saved_pages.length>(token.pageLimit??100))reject('Zielauswertung passt nicht zur begrenzten sichtbaren Auswahl.');
    const seen=new Set();
    for(const finding of analysis.location_findings){const row=originalRows.find(v=>v.row.sequence===finding.sequence)?.row;
      if(!object(finding) || !row || seen.has(finding.sequence) || finding.observation_id!==row.observation_id || finding.occurrence_id!==row.occurrence_id || finding.entry_ordinal!==row.observed.entry_ordinal || finding.admitted_fetch_url!==row.admission.admitted_fetch_url || finding.admission_decision!==row.admission.decision || finding.original_membership_url!==(row.observed.validation_error===null && row.observed.loc_ordinal!==null?row.observed.normalized_fetch_url:null) || !Object.hasOwn(sitemapTargetLabels,finding.original_state) || typeof finding.admitted_alias_pending!=='boolean')reject('Originales Sitemap-Ziel wurde unzulässig umgebunden.');
      seen.add(finding.sequence);
      if(finding.original_observation!==null && (!observation(finding.original_observation,finding.original_membership_url) || finding.original_state!==finding.original_observation.state) || finding.admitted_alias_observation!==null && (finding.admitted_fetch_url===finding.original_membership_url || !observation(finding.admitted_alias_observation,finding.admitted_fetch_url)) || (finding.original_membership_url===null?finding.observed_original_inlinks!==null:!inlinks(finding.observed_original_inlinks,finding.original_membership_url)))reject('Original-, Alias- oder Inlink-Fakten passen nicht zusammen.');
    }
    let previous=-1;const pages=new Set();
    const sourceEvidence=v=>integer(v.original_source_document_count) && integer(v.original_source_observation_count) && v.original_source_document_count<=v.original_source_observation_count && v.original_source_observation_count<=v.original_membership_occurrences && (v.original_source_document_count>0)===(v.original_membership_occurrences>0) && Array.isArray(v.original_source_document_samples) && v.original_source_document_samples.length===Math.min(v.original_source_document_count,5) && v.source_document_samples_have_more===(v.original_source_document_count>5) && new Set(v.original_source_document_samples.map(s=>s.task_url)).size===v.original_source_document_samples.length && v.original_source_document_samples.every(s=>object(s) && hash(s.observation_id) && url(s.task_url) && (s.source_final_url===null || url(s.source_final_url)));
    for(const value of analysis.saved_pages){const page=value?.observation;
      if(!object(value) || !observation(page,page?.requested_url) || page.report_page_ordinal<=previous || pages.has(page.requested_url) || !integer(value.original_membership_occurrences) || !sourceEvidence(value) || !integer(value.admitted_alias_occurrences) || !Object.hasOwn(sitemapPageMembershipLabels,value.membership) || !inlinks(value.observed_inlinks,page.requested_url) || value.listed_without_observed_nonself_inlinks!==(value.original_membership_occurrences>0 && value.observed_inlinks.nonself_anchor_occurrences===0) || (value.original_membership_occurrences>0)!==(value.membership==='listed_original') || value.membership==='not_listed_current_declared_sources' && result.coverage.derived.current_valid_original_location_negative_permitted!==true || value.membership==='not_listed_historical_declared_sources' && result.coverage.derived.historical_valid_original_location_negative_permitted!==true)reject('Gespeicherte Seitenzuordnung oder Inlink-Abdeckung ist nicht unterstützt.');
      previous=page.report_page_ordinal;pages.add(page.requested_url);
    }
    const last=analysis.saved_pages.at(-1)?.observation;
    if(analysis.last_saved_page_url!==(last?.requested_url??null) || analysis.last_saved_page_ordinal!==(last?.report_page_ordinal??null) || analysis.saved_pages_have_more && (!last || !opaque(result.next_page_cursor) || result.next_page_cursor===token.pageCursor) || !analysis.saved_pages_have_more && result.next_page_cursor!==null)reject('Gespeicherte Seitenauswahl hat keinen gültigen Fortsetzungscursor.');
  }
  function validateSitemapEnvelope(result,token,expectedSnapshotToken) {
    const object=value=>Boolean(value && typeof value==='object' && !Array.isArray(value)),hash=value=>typeof value==='string' && /^[a-f0-9]{64}$/.test(value),integer=value=>Number.isSafeInteger(value) && value>=0,text=value=>typeof value==='string',nullableText=value=>value===null || text(value),opaque=value=>text(value) && value.length>0 && value.length<=4096 && /^[A-Za-z0-9_-]+$/.test(value);
    const reject=message=>{throw new Error(message);};
    if(!object(result) || result.schema_version!==1 || result.job_id!==token.id || typeof result.catalog_available!=='boolean' || typeof result.has_more!=='boolean' || result.website_requests!==0 || result.snapshot_scope!=='validated_sqlite_read_snapshot' || !object(result.coverage) || result.coverage.declared_source_cohort_complete!==false || result.coverage.negative_membership!=='unknown' || result.coverage.fetch_status_join!=='not_performed' || result.coverage.scope!=='retained_original_declarations_and_memberships')reject('Nicht unterstützter Sitemap-Snapshot oder unbestätigte Abdeckung.');
    if(!object(result.filter) || result.filter.kind!==token.kind || result.filter.source_observation_id!==null || result.filter.membership_url!==null)reject('Sitemap-Seite passt nicht zur angeforderten Auswahl.');
    if(!result.catalog_available){
      if(!['sitemap_catalog_unavailable','sqlite_history_unavailable'].includes(result.availability_reason) || result.rows!==null || result.summary!==null || result.snapshot!==null || result.snapshot_token!==null || result.inherited!==null || result.next_cursor!==null || result.has_more)reject('Nicht verfügbare Quellen dürfen keinen vollständigen leeren Stand behaupten.');
      validateSitemapDerived(result,reject);
      validateSitemapTargetAnalysis(result,token,reject);
      return;
    }
    const summary=result.summary,snapshot=result.snapshot;
    if(result.availability_reason!=='saved_validated_sqlite_catalog' || !object(summary) || summary.engine!==sitemapEngine || summary.rows_available!==true || summary.declared_source_cohort_complete!==false || typeof summary.legacy_history_unknown!=='boolean' || ['revision','last_sequence','root_rows','document_rows','entry_rows','location_rows','incomplete_documents','catalog_json_bytes'].some(key=>!integer(summary[key])) || !hash(summary.config_sha256) || !hash(summary.rolling_sha256) || summary.hash_profile!=='sha256-tagged-canonical-rust-serde-rows-v1')reject('Sitemap-Stand hat keine unterstützte gespeicherte Zusammenfassung.');
    if(!object(snapshot) || snapshot.job_id!==token.id || !integer(snapshot.crawl_generation) || snapshot.crawl_generation<1 || token.serverGeneration!==null && snapshot.crawl_generation!==token.serverGeneration || !hash(snapshot.crawl_config_sha256) || !hash(snapshot.catalog_config_sha256) || !hash(snapshot.catalog_rolling_sha256) || !hash(snapshot.catalog_state_sha256) || snapshot.catalog_config_sha256!==summary.config_sha256 || snapshot.catalog_revision!==summary.revision || snapshot.catalog_rolling_sha256!==summary.rolling_sha256 || !opaque(result.snapshot_token) || expectedSnapshotToken!==null && result.snapshot_token!==expectedSnapshotToken)reject('Sitemap-Seite gehört nicht zum aktuellen gebundenen Stand.');
    if(result.inherited!==null && (!object(result.inherited) || !hash(result.inherited.parent_snapshot_sha256) || !hash(result.inherited.parent_catalog_sha256) || result.inherited.current_discovery_executed!==false))reject('Geerbte Herkunft ist nicht unterstützt.');
    if(JSON.stringify(snapshot.inherited_basis)!==JSON.stringify(result.inherited))reject('Geerbte Herkunft passt nicht zum Snapshot.');
    if(!Array.isArray(result.rows) || result.rows.length>token.limit || result.has_more && (!result.rows.length || !opaque(result.next_cursor) || result.next_cursor===token.cursor) || !result.has_more && result.next_cursor!==null)reject('Ungültige begrenzte Sitemap-Seite oder Cursor.');
    let previous=0;
    for(const tagged of result.rows){
      if(!object(tagged) || !['root','document','entry','location'].includes(tagged.kind) || token.kind!=='all' && tagged.kind!==token.kind || !object(tagged.row) || !integer(tagged.row.sequence) || tagged.row.sequence<=previous || tagged.row.sequence>summary.last_sequence)reject('Sitemap-Vorkommen ohne geordnete gespeicherte Identität.');
      previous=tagged.row.sequence;const row=tagged.row;
      if(tagged.kind==='root' || tagged.kind==='location'){
        const binding=row.admission;
        if(!object(binding) || !['unbound','invalid_location','missing_location','incomplete_source','outside_scope','filter_denied','source_check_only','policy_denied','coalesced','scheduled'].includes(binding.decision) || ['membership_url','admitted_fetch_url','coalesced_policy_occurrence_id','policy_occurrence_id','policy_source_revision'].some(key=>!nullableText(binding[key])))reject('Sitemap-Zulassungsbindung ist beschädigt.');
        if(tagged.kind==='root' && (!['explicit_config','imported_sitemap_row','robots_directive'].includes(row.kind) || !text(row.raw_value) || !nullableText(row.raw_line) || !integer(row.source_ordinal)))reject('Originale Sitemap-Deklaration ist beschädigt.');
        if(tagged.kind==='location' && (!object(row.observed) || !['page_location','child_sitemap'].includes(row.observed.role) || !integer(row.observed.ordinal) || row.observed.ordinal<1 || !integer(row.observed.entry_ordinal) || row.observed.entry_ordinal<1 || row.observed.loc_ordinal!==null && (!integer(row.observed.loc_ordinal) || row.observed.loc_ordinal<1) || ['decoded_value','validation_error','raw_element_xml','raw_inner_xml'].some(key=>!nullableText(row.observed[key]))))reject('Originales XML-URL-Vorkommen ist beschädigt.');
      }
      if(tagged.kind==='document' && (!['robots_directives','sitemap_xml'].includes(row.kind) || !text(row.task_url) || typeof row.retained_rows_complete!=='boolean' || typeof row.declaration_scan_complete!=='boolean' || !nullableText(row.failure_code)))reject('Sitemap-Quelleinheit ist beschädigt.');
      if(tagged.kind==='entry' && (!object(row.observed) || !integer(row.observed.ordinal) || row.observed.ordinal<1 || typeof row.observed.complete!=='boolean' || !Array.isArray(row.observed.lastmod) || row.observed.lastmod.some(value=>!object(value) || !text(value.decoded_value)) || !Array.isArray(row.observed.diagnostics) || row.observed.diagnostics.some(value=>!text(value))))reject('XML-Eintragsbeobachtung ist beschädigt.');
    }
    validateSitemapDerived(result,reject);
    validateSitemapTargetAnalysis(result,token,reject);
  }
  async function readSitemapPage(path,signal) {
    const response=await fetch(path,{signal,headers:{Accept:'application/json'}}),type=response.headers.get('content-type') || '';
    if(!type.includes('application/json'))throw new Error('Sitemap-API hat keine unterstützte JSON-Antwort geliefert.');
    const raw=await response.text();
    if(new TextEncoder().encode(raw).length>8388608)throw new Error('Sitemap-Antwort überschreitet die begrenzte Seitengröße.');
    const body=JSON.parse(raw);
    if(!response.ok){const error=new Error(typeof body.error==='string'?body.error:`HTTP ${response.status}`);error.code=body.code;error.status=response.status;throw error;}
    return body;
  }
  async function loadSitemapPage(cursor=sitemapMembership.cursor,navigation=null,pageCursor=sitemapMembership.pageCursor,pageNavigation=null) {
    if(!state.crawl || !sitemapSection() || sitemapMembership.token)return;
    const current=sitemapMembership,token={id:state.crawl.id,generation:state.generation,serverGeneration:state.crawl.generation??null,revision:current.revision,kind:sitemapKind(),limit:sitemapLimit(),pageLimit:Number($('sitemap-analysis-limit').value),cursor,pageCursor,controller:new AbortController()};
    const expectedSnapshotToken=navigation || pageNavigation?current.result?.snapshot_token??null:null;
    const owns=()=>current.token===token && token.id===state.crawl?.id && token.generation===state.generation && token.serverGeneration===(state.crawl?.generation??null) && token.revision===current.revision && token.kind===sitemapKind() && token.limit===sitemapLimit() && token.pageLimit===Number($('sitemap-analysis-limit').value) && sitemapSection();
    current.token=token;current.error='';current.stale=false;current.selected=null;current.selectedPage=null;renderSitemapMembership();
    try{
      if(![200,100,50,10,1].includes(token.limit) || ![200,100,50,10,1].includes(token.pageLimit))throw new Error('Eine unterstützte Anzahl Zeilen und gespeicherter Seiten auswählen.');
      const query=new URLSearchParams({kind:token.kind,limit:String(token.limit),page_limit:String(token.pageLimit)});if(cursor!==null)query.set('cursor',cursor);if(pageCursor!==null)query.set('page_cursor',pageCursor);
      const result=await readSitemapPage(`/api/crawls/${encodeURIComponent(token.id)}/sitemaps?${query}`,token.controller.signal);
      if(!owns())return;
      validateSitemapEnvelope(result,token,expectedSnapshotToken);
      if(navigation==='next')current.back.push(current.cursor);else if(navigation==='back')current.back.pop();
      if(pageNavigation==='next')current.pageBack.push(current.pageCursor);else if(pageNavigation==='back')current.pageBack.pop();
      current.cursor=cursor;current.pageCursor=pageCursor;current.result=result;
    }catch(error){
      if(owns()){
        current.stale=['stale_sitemap_cursor','invalid_sitemap_cursor','sitemap_job_snapshot_conflict'].includes(error.code);
        if(current.stale){current.cursor=null;current.back=[];current.pageCursor=null;current.pageBack=[];}
        current.error=error.status===413 && error.code==='sitemap_response_byte_budget'?'Diese Sitemap-Seite enthält zu umfangreiche Originaldaten. Unter „Zeilen je Seite“ weniger Zeilen wählen, um den gespeicherten Stand erneut zu laden. Es wurden keine Vorkommen abgeschnitten.':current.stale?'Der gespeicherte Sitemap-Stand wurde geändert oder der Cursor ist ungültig. „Aktuellen Stand laden“ lädt die erste Seite des aktuellen Stands.':`Sitemap-Beobachtungen nicht verfügbar: ${error.message}`;
      }
    }finally{
      if(current.token===token){current.token=null;if(token.id===state.crawl?.id && token.generation===state.generation && token.revision===current.revision && token.kind===sitemapKind() && token.limit===sitemapLimit() && sitemapSection())renderSitemapMembership();}
    }
  }
  function downloadSitemapSnapshot() {
    if(!sitemapUsable() || !state.crawl)return;
    const result=sitemapMembership.result,query=new URLSearchParams({kind:result.filter.kind,snapshot:result.snapshot_token}),link=document.createElement('a');
    link.href=`/api/crawls/${encodeURIComponent(state.crawl.id)}/sitemaps/export.csv?${query}`;
    link.download=`open-seo-spider-${state.crawl.id}-sitemaps.csv`;document.body.append(link);link.click();link.remove();
    message('CSV des gebundenen gespeicherten Sitemap-Stands angefordert. Dies bestätigt keine vollständige Quellenabdeckung.');
  }
  function bindSitemapEvents() {
    $('sitemap-analysis-limit').addEventListener('change',()=>{sitemapMembership.pageCursor=null;sitemapMembership.pageBack=[];loadSitemapPage(sitemapMembership.cursor,null,null);});
    $('sitemap-analysis-search').addEventListener('input',renderSitemapTargetAnalysis);
    $('sitemap-analysis-filter').addEventListener('change',renderSitemapTargetAnalysis);
    $('sitemap-analysis-next').addEventListener('click',()=>{if(sitemapUsable() && sitemapMembership.result?.membership_analysis?.saved_pages_have_more)loadSitemapPage(sitemapMembership.cursor,null,sitemapMembership.result.next_page_cursor,'next');});
    $('sitemap-analysis-prev').addEventListener('click',()=>{if(sitemapUsable() && sitemapMembership.pageBack.length)loadSitemapPage(sitemapMembership.cursor,null,sitemapMembership.pageBack.at(-1),'back');});
    $('sitemap-analysis-inspector-close').addEventListener('click',()=>{sitemapMembership.selectedPage=null;renderSitemapSavedPageInspector();});
    $('sitemap-membership-kind').addEventListener('change',()=>{resetSitemapMembership();renderSitemapMembership();loadSitemapPage();});
    $('sitemap-membership-limit').addEventListener('change',()=>{resetSitemapMembership();loadSitemapPage(null);});
    $('sitemap-membership-search').addEventListener('input',renderSitemapMembership);
    $('sitemap-membership-filter').addEventListener('change',renderSitemapMembership);
    $('sitemap-membership-refresh').addEventListener('click',()=>{resetSitemapMembership();loadSitemapPage(null);});
    $('sitemap-membership-next').addEventListener('click',()=>{if(sitemapUsable() && sitemapMembership.result.has_more)loadSitemapPage(sitemapMembership.result.next_cursor,'next');});
    $('sitemap-membership-prev').addEventListener('click',()=>{if(sitemapUsable() && sitemapMembership.back.length)loadSitemapPage(sitemapMembership.back.at(-1),'back');});
    $('sitemap-membership-inspector-close').addEventListener('click',()=>{sitemapMembership.selected=null;renderSitemapInspector();});
    $('sitemap-membership-export-page').addEventListener('click',exportSitemapLoadedPage);
    $('sitemap-membership-export-csv').addEventListener('click',downloadSitemapSnapshot);
  }

  const nativeResourceEngine = 'open-seo-spider/resource-discovery-v1';
  const resourceContractEngine = 'open-seo-spider/resource-contract-v2';
  const resourceReferenceKinds = ['initial_input','internal_anchor','external_anchor','css','java_script','image','media','pdf','swf','other','iframe','refresh','canonical','hreflang','pagination','amp','mobile_alternate','pdf_annotation'];
  const resourceContractLabel=engine=>engine==null?null:engine===resourceContractEngine?'Aktuelle Regeln':engine===nativeResourceEngine?'Bisherige Regeln':'Nicht unterstützt';
  const qualificationValue=(row,key)=>row.decision.qualification?.engine===resourceContractEngine?row.decision.qualification[key]:null;
  function syncResourceContractSettings() {
    const form=$('settings-form'),enabled=form.elements.resource_discovery_enabled.checked;
    let config,error;try{config=JSON.parse(form.elements.resource_discovery_json.value);}catch(_){error='Regel-JSON ist ungültig; kein Profil wird automatisch eingesetzt.';}
    const object=config && typeof config==='object' && !Array.isArray(config),contract=object?config.contract:null,current=contract && typeof contract==='object' && !Array.isArray(contract) && (!Object.hasOwn(contract,'engine') || contract.engine===resourceContractEngine);
    const mixed=object && (config.follow_rel_nofollow===true)!==(config.follow_document_nofollow===true);
    $('resource-contract-status').textContent=!enabled?'Ressourcenregeln sind aus; diese Konfiguration wird nicht an den Crawl gesendet.':error?error:!object?'Regel-JSON muss ein Objekt sein.':contract==null?`Bisherige Regeln: Die gespeicherte Konfiguration bleibt unverändert. Aktuelle Regeln fassen qualifizierte Rel-Links und Dokument-Nofollow gemeinsam je intern/extern zusammen.${mixed?' Die bisherigen Rel-/Dokumentschalter unterscheiden sich; beim ausdrücklich gewählten Wechsel starten beide neuen Schalter deshalb aus.':' Beim ausdrücklich gewählten Wechsel wird der gemeinsame bisherige Wert in beide neuen Schalter übernommen.'}`:current?'Aktuelle Regeln: nofollow, ugc und sponsored sowie Dokument-Nofollow werden gemeinsam berücksichtigt; Overrides gelten getrennt für interne und externe Links.':'Dieses Semantikprofil wird nicht unterstützt; die vorhandenen Werte bleiben erhalten.';
    $('resource-contract-use-current').disabled=!enabled || !object || contract!=null && !current;
    for(const [id,key] of [['resource-follow-internal-qualified','follow_internal_qualified_links'],['resource-follow-external-qualified','follow_external_qualified_links']]){const input=$(id);input.disabled=!enabled || !current;input.checked=current && contract[key]===true;}
  }
  function useCurrentResourceContract() {
    const form=$('settings-form');
    try {
      const parsed=parseResourceDiscovery({resource_discovery_enabled:true,javascript_rendering:false,resource_discovery_json:form.elements.resource_discovery_json.value,resource_discovery_limits_json:form.elements.resource_discovery_limits_json.value}),config=parsed.resource_discovery;
      if(config.contract==null){const rel=config.follow_rel_nofollow===true,document=config.follow_document_nofollow===true,follow=rel===document?rel:false;config.contract={engine:resourceContractEngine,follow_internal_qualified_links:follow,follow_external_qualified_links:follow};config.follow_rel_nofollow=false;config.follow_document_nofollow=false;form.elements.resource_discovery_json.value=JSON.stringify(config,null,2);}
      $('settings-error').hidden=true;syncResourceContractSettings();
    }catch(error){$('settings-error').textContent=error.message;$('settings-error').hidden=false;}
  }
  function changeResourceQualifiedLinks() {
    const form=$('settings-form');
    try {
      const parsed=parseResourceDiscovery({resource_discovery_enabled:true,javascript_rendering:false,resource_discovery_json:form.elements.resource_discovery_json.value,resource_discovery_limits_json:form.elements.resource_discovery_limits_json.value}),config=parsed.resource_discovery;
      if(config.contract==null || Object.hasOwn(config.contract,'engine') && config.contract.engine!==resourceContractEngine)throw new Error('Zuerst die aktuellen Regeln ausdrücklich verwenden.');
      config.contract.follow_internal_qualified_links=$('resource-follow-internal-qualified').checked;config.contract.follow_external_qualified_links=$('resource-follow-external-qualified').checked;
      form.elements.resource_discovery_json.value=JSON.stringify(config,null,2);$('settings-error').hidden=true;
    }catch(error){$('settings-error').textContent=error.message;$('settings-error').hidden=false;}
    syncResourceContractSettings();
  }
  function resourceSavedContractLabel() {
    const result=state.resources.result,profile=result?.configured_semantic_contract,binding=result?.semantic_contract_binding;
    if(!result)return 'Semantikprofil des gespeicherten Snapshots noch nicht erfasst.';
    if(!result.enabled)return 'Für diesen Bericht wurden keine Ressourcenregeln konfiguriert.';
    if(profile==null || binding==null)return 'Semantikprofil des gespeicherten Snapshots nicht erfasst. Fehlende Grant-Rollen bleiben unbekannt.';
    if(![nativeResourceEngine,resourceContractEngine].includes(profile))return 'Gespeichertes Semantikprofil nicht unterstützt; genaue Rohdaten bleiben separat sichtbar.';
    const label=profile===resourceContractEngine?'aktuelle Ressourcenregeln':'bisherige Ressourcenregeln';
    if(binding!=='validated_full_config_and_generation')return `Nur deklarierte Konfiguration: ${label}. Kein durch SQLite-Config und Generation gebundenes Semantikprofil; historische Grants werden nicht umgestellt.`;
    return `Validierter SQLite-Snapshot: ${label}. Die Rolle jedes Grants stammt ausschließlich aus dessen eigener Beobachtung; fehlende Rollen bleiben unbekannt.`;
  }
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
    if(result.enabled && result.engine!==nativeResourceEngine)throw new Error('Nicht unterstützte Ressourcen-Ledger-Engine; dies ist keine fehlende Quellbeobachtung.');
    if(!result.ledger_available){if(result.snapshot!==null)throw new Error('Nicht verfügbares Ledger darf keinen verifizierten nativen Snapshot behaupten.');return;}
    const snapshot=result.snapshot;
    if(!snapshot || !Number.isSafeInteger(snapshot.generation) || snapshot.generation<1 || snapshot.generation!==result.crawl_generation || expectedGeneration!=null && snapshot.generation!==expectedGeneration || typeof snapshot.crawl_config_sha256!=='string' || !/^[a-fA-F0-9]{64}$/.test(snapshot.crawl_config_sha256) || snapshot.crawl_config_sha256!==result.crawl_config_sha256)throw new Error('Ressourcen-Snapshot passt nicht zur aktuellen Crawl-Generation oder vollständigen Job-Konfiguration.');
    if(result.semantic_contract_binding!=null) {
      if(result.semantic_contract_binding!=='validated_full_config_and_generation')throw new Error('Native Semantikprofil-Bindung ist nur deklariert; kein Profil wird daraus als verifiziert angezeigt.');
      const configured=result.configured_semantic_contract,recorded=snapshot.resource_contract_engine;
      if(configured===resourceContractEngine && recorded!==resourceContractEngine || configured===nativeResourceEngine && recorded!=null)throw new Error('Semantikprofil des SQL-Snapshots stimmt nicht mit der gebundenen Konfiguration überein.');
    }
    const summary=result.checkpoint;
    if(!result.enabled || summary?.engine!==nativeResourceEngine || summary.config_sha256!==result.config_sha256 || typeof summary.complete!=='boolean' || !['config_sha256','rolling_sha256'].every(key=>typeof summary[key]==='string' && /^[a-fA-F0-9]{64}$/.test(summary[key])) || ['revision','decision_rows','committed_bytes','completed_transports','retained_responses','pending_transports','pending_expansions','incomplete_sources'].some(key=>!Number.isSafeInteger(summary[key]) || summary[key]<0))throw new Error('Ressourcen-Ledger ohne unterstützte Zusammenfassung oder erwartete Job-Config-Bindung.');
    if(summary.inherited_history!=null && (typeof summary.inherited_history.rows_available!=='boolean' || typeof summary.inherited_history.parent_snapshot_sha256!=='string'))throw new Error('Unbekannte Verfügbarkeit der geerbten Ledger-Historie.');
    if(kind==='transports') {
      let previous=cursor;
      for(const row of rows) {
        if(row.engine!==nativeResourceEngine || typeof row.requested_url!=='string' || !row.requested_url || row.requested_url<=previous || !Number.isSafeInteger(row.observed_generation) || row.observed_generation<1 || typeof row.retained_response!=='boolean' || !row.projection || row.projection.url!==row.requested_url || row.projection.resource_response!=null && row.projection.resource_response.engine!==nativeResourceEngine)throw new Error('Transport ohne unterstützte Anfrage-Identität, geordneten Cursor oder Metadatenprojektion.');
        const redirects=row.projection.resource_response?.redirect_decisions;
        if(redirects!=null && (!Array.isArray(redirects) || redirects.some(hop=>!hop || ['source_url','raw_location','reason'].some(key=>typeof hop[key]!=='string') || hop.target_url!=null && typeof hop.target_url!=='string' || !Array.isArray(hop.original_reference_kinds) || hop.original_reference_kinds.some(kind=>!resourceReferenceKinds.includes(kind)) || !Array.isArray(hop.authorizing_occurrence_ids) || hop.authorizing_occurrence_ids.some(id=>typeof id!=='string') || ['scope_allowed','allowed','request_started'].some(key=>typeof hop[key]!=='boolean') || ['policy_check','policy_traverse','robots_allowed'].some(key=>hop[key]!=null && typeof hop[key]!=='boolean'))))throw new Error('Redirect-Rollenbeobachtung ist beschädigt; fehlende Zulassungs- oder Quellenfakten werden nicht ergänzt.');
        previous=row.requested_url;
      }
      if(result.next_after_url!== (rows.length?rows.at(-1).requested_url:null) || result.has_more && !rows.length)throw new Error('Ungültiger Transport-Cursor.');
    } else {
      let previous=cursor;
      for(const row of rows) {
        const observed=row.occurrence, grant=row.decision?.grant;
        if(!Number.isSafeInteger(row.sequence) || row.sequence<=previous || !Number.isSafeInteger(row.generation) || row.generation<1 || !['current','inherited'].includes(row.basis) || !observed || ['occurrence_id','requested_source_url','source_document_url','raw_value','reference_kind','classification_basis','source_revision'].some(key=>typeof observed[key]!=='string') || !observed.occurrence_id || typeof observed.nofollow!=='boolean' || !Number.isSafeInteger(observed.source_ordinal) || observed.source_ordinal<0 || typeof row.evaluation_revision!=='string' || !row.decision || typeof row.decision.reason!=='string')throw new Error('Ressourcenvorkommen ohne gespeicherte Sequenz, Quelle oder Entscheidung.');
        if(grant!=null && (grant.engine!==nativeResourceEngine || grant.config_sha256!==summary.config_sha256 || grant.occurrence_id!==observed.occurrence_id || typeof grant.fetch_url!=='string' || ['store','can_expand_navigation','can_expand_resources','can_expand_css','can_expand_pdf'].some(key=>typeof grant[key]!=='boolean')))throw new Error('Nicht unterstützter eigenständiger Ressourcen-Grant.');
        if(grant?.reference_kind!=null && !resourceReferenceKinds.includes(grant.reference_kind))throw new Error('Nicht unterstützte beobachtete Grant-Rolle; keine Rolle wird aus URL, MIME oder Quellreferenz abgeleitet.');
        const qualification=row.decision.qualification;
        if(qualification!=null && (typeof qualification!=='object' || Array.isArray(qualification) || typeof qualification.engine!=='string'))throw new Error('Qualifizierungsprofil ohne beobachtete Profilkennung.');
        if(qualification?.engine===resourceContractEngine && (!['internal','external','unknown'].includes(qualification.scope) || !['html_rel','http_link_rel','not_applicable'].includes(qualification.source) || !Array.isArray(qualification.rel_qualifiers) || qualification.rel_qualifiers.some(value=>!['nofollow','ugc','sponsored'].includes(value)) || ['document_nofollow','follow_allowed'].some(key=>typeof qualification[key]!=='boolean')))throw new Error('Qualifizierungsbeobachtung enthält keine unterstützten Bereichs-, Quellen- oder Direktivenfakten.');
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
    ['Zusammengeführte Grant-ID',row=>row.decision.coalesced_into],['Quellordinal',row=>row.occurrence.source_ordinal],['Quelleinheit vollständig',row=>row.occurrence.source_complete],['Quelleinheit-Revision',row=>row.occurrence.source_unit_revision],['DOM-Pfad',row=>row.occurrence.dom_location],['UTF-8-Bytebeginn',row=>row.occurrence.byte_start],['UTF-8-Byteende',row=>row.occurrence.byte_end],['Deklarierter MIME-Hinweis',row=>row.occurrence.declared_mime],['Klassifikationsbasis',row=>row.occurrence.classification_basis],['Nofollow beobachtet',row=>row.occurrence.nofollow],['Beobachtungsfehler',row=>row.occurrence.observation_error],['Quellrevision',row=>row.occurrence.source_revision],['Vorkommen-ID',row=>row.occurrence.occurrence_id],['Auswertungsrevision',row=>row.evaluation_revision],['Tatsächlich beobachtete Grant-Rolle',row=>row.decision.grant?.reference_kind],['Beobachtetes Qualifizierungsprofil',row=>resourceContractLabel(row.decision.qualification?.engine)],['Beobachteter Link-Bereich',row=>qualificationValue(row,'scope')],['Beobachtete Rel-Qualifikatoren',row=>qualificationValue(row,'rel_qualifiers')==null?null:JSON.stringify(qualificationValue(row,'rel_qualifiers'))],['Qualifizierungsquelle',row=>qualificationValue(row,'source')],['Dokument-Nofollow · beobachtet',row=>qualificationValue(row,'document_nofollow')],['Folgen laut gespeicherter Entscheidung',row=>qualificationValue(row,'follow_allowed')]
  ];
  const resourceTransportColumns=[
    ['Anfrage-Identität',row=>row.requested_url],['Gespeicherte finale URL',row=>row.projection.final_url],['Beobachtete Generation',row=>row.observed_generation],['Antwort im Seitenbericht gespeichert',row=>row.retained_response],['Erster HTTP-Status',row=>row.projection.status],['Finaler HTTP-Status',row=>row.projection.final_status],['Beobachteter Content-Type',row=>row.projection.content_type],['Antwortzeit ms',row=>row.projection.response_ms],['Erfasste Größe Bytes',row=>row.projection.size_bytes],['Transportfehler',row=>row.projection.error],['Robots blockiert',row=>row.projection.blocked_by_robots],['Response abgeschnitten',row=>row.projection.truncated],
    ['Metadatenprojektion vollständig',row=>row.projection.resource_response?.metadata_projection_complete],['Nicht erhaltene Projektionsfelder',row=>row.projection.resource_response?.unretained_fields==null?null:JSON.stringify(row.projection.resource_response.unretained_fields)],['Entity-Body in Projektion erhalten',row=>row.projection.resource_response?.entity_body_retained],['Erfasste Entity-Bytes',row=>row.projection.resource_response?.captured_entity_bytes],['Entity-Erfassung vollständig',row=>row.projection.resource_response?.captured_entity_complete],['Dekodierungsstatus',row=>row.projection.resource_response?.decode_status],['Dekodierte Quelle SHA256',row=>row.projection.resource_response?.decoded_source_sha256],['Entity-SHA256',row=>row.projection.resource_response?.captured_entity_sha256],['Erfasst am',row=>row.projection.fetched_at],['Beobachtetes Antwort-Semantikprofil',row=>resourceContractLabel(row.projection.resource_response?.contract_engine)],['Gespeicherte Redirect-Entscheidungen',row=>row.projection.resource_response?.redirect_decisions?.length]
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
    $('resource-discovery-profile').textContent=resourceSavedContractLabel();
    $('resource-discovery-error').hidden=!resources.error; $('resource-discovery-error').textContent=resources.error;
    $('resource-discovery-summary').textContent=!state.crawl?'Kein Crawl ausgewählt.':busy?'Gespeicherte Ressourcen-Seite wird geladen …':resources.error?'Snapshot nicht verfügbar; kein vollständiges leeres Ergebnis.':!result?'Noch keine gespeicherte Ledger-Seite geladen.':!result.enabled?'Native Ressourcenermittlung war für diesen Bericht nicht aktiviert.':!result.ledger_available?'Echtes SQLite-Ledger nicht verfügbar. Eine deklarierte Zusammenfassung rekonstruiert keine gespeicherten Vorkommen.':`${number(summary.decision_rows)} gespeicherte Entscheidungen · ${number(summary.completed_transports)} Abgeschlossene URL-Abrufe (vorliegendes Protokoll) · ${number(summary.retained_responses)} Gespeicherte Ergebniszeilen · ${number(summary.pending_transports)} Transporte ausstehend · ${number(summary.pending_expansions)} Expansionen ausstehend · ${number(summary.incomplete_sources)} unvollständige Quellen · Entdeckung ${summary.complete?'vollständig laut Ledger':'unvollständig'}`;
    const inherited=summary?.inherited_history;
    $('resource-discovery-scope').textContent=result?[`API-Snapshot: ${result.snapshot_scope}; Website-Anfragen: ${result.website_requests}; Entity-Bodies enthalten: ${resourceValue(result.entity_bodies_included)}.`,result.reason?`Verfügbarkeitsgrund: ${result.reason}.`:'',summary?`${result.ledger_available?'SQLite-Zusammenfassung':'Nur deklarierte Zusammenfassung'} · Revision ${resourceValue(summary.revision)} · Config-SHA256 ${resourceValue(summary.config_sha256)}.`:'',inherited?`Elternsnapshot ${inherited.parent_snapshot_sha256}; geerbte Historienzeilen ${inherited.rows_available?'verfügbar':'NICHT verfügbar; Herkunftsabdeckung unvollständig'}. Eine deklarierte Elternzusammenfassung bestätigt keine vorhandenen Zeilen.`:''].filter(Boolean).join(' '):'Metadaten werden ausschließlich aus gespeicherten API-Beobachtungen gelesen; keine Website-Anfrage wird gestartet.';
    $('resource-discovery-checkpoint').textContent=JSON.stringify({enabled:result?.enabled??null,ledger_available:result?.ledger_available??null,engine:result?.engine??null,configured_semantic_contract:result?.configured_semantic_contract??null,semantic_contract_binding:result?.semantic_contract_binding??null,checkpoint:summary??null,snapshot:result?.snapshot??null,crawl_config_sha256:result?.crawl_config_sha256??null,crawl_generation:result?.crawl_generation??null,reason:result?.reason??null,snapshot_scope:result?.snapshot_scope??null,entity_bodies_included:result?.entity_bodies_included??null},null,2);
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
    $('resource-redirect-details').hidden=!transport;
    if(transport){
      const decisions=row.projection.resource_response?.redirect_decisions,known=Array.isArray(decisions),rows=known?decisions.slice(0,200):[],table=$('resource-redirect-table');
      const columns=[['Position',(_,index)=>index+1],['Quell-URL',hop=>hop.source_url],['Original-Location',hop=>hop.raw_location],['Beobachtetes Redirect-Ziel',hop=>hop.target_url],['Ursprüngliche Grant-Rollen',hop=>JSON.stringify(hop.original_reference_kinds)],['Erlaubende Vorkommen',hop=>JSON.stringify(hop.authorizing_occurrence_ids)],['Rollen-Scope erlaubt',hop=>hop.scope_allowed],['Policy-Prüfung',hop=>hop.policy_check],['Policy-Traversierung',hop=>hop.policy_traverse],['Robots-Zulassung',hop=>hop.robots_allowed],['Zugelassen',hop=>hop.allowed],['Gespeicherter Grund',hop=>hop.reason],['Anfrage begonnen',hop=>hop.request_started]];
      $('resource-redirect-scope').textContent=!known?'Redirect-Rollenfakten nicht erfasst. Aus finaler URL oder Status werden keine früheren Zulassungen, Header oder Anfragen abgeleitet.':`${decisions.length} gespeicherte Entscheidungen; ${rows.length} hier angezeigt. Anfrage begonnen bedeutet Initiierung, keinen bestätigten Empfang oder Erfolg am Ziel. Fehlende Policy-/Robots-Fakten bleiben unbekannt; frühere Direktiven werden nicht zu finalen Dokumentdirektiven umgedeutet.`;
      table.querySelector('thead').innerHTML=`<tr>${columns.map(([label])=>`<th scope="col">${escape(label)}</th>`).join('')}</tr>`;
      table.querySelector('tbody').innerHTML=rows.length?rows.map((hop,index)=>`<tr>${columns.map(([,value])=>`<td>${escape(resourceValue(value(hop,index)))}</td>`).join('')}</tr>`).join(''):`<tr><td class="empty-cell" colspan="${columns.length}">${known?'Beobachtet ohne gespeicherte Redirect-Entscheidungen.':'Nicht erfasst; kein Beweis für eine leere Redirectkette.'}</td></tr>`;
    }
  }
  function exportResourcePage() {
    const rows=resourceVisibleRows();if(!rows.length)return;
    const result=state.resources.result,summary=result.checkpoint;
    const meta=[['API Snapshot-Scope',()=>result.snapshot_scope],['Vollständige Crawl-Config SHA256',()=>result.snapshot.crawl_config_sha256],['SQL-Snapshot Generation',()=>result.snapshot.generation],['Ledger-Engine',()=>result.engine],['Ledger-Config SHA256',()=>summary.config_sha256],['Ledger-Snapshot Revision',()=>summary.revision],['Ledger-Rolling SHA256',()=>summary.rolling_sha256],['Ledger-Entdeckung vollständig',()=>summary.complete],['Geerbte Historienzeilen verfügbar',()=>summary.inherited_history?.rows_available],['Geladener Cursor',()=>state.resources.cursor],['Exportumfang',()=> 'filtered_loaded_page_only'],['Konfiguriertes Semantikprofil',()=>result.configured_semantic_contract],['Semantikprofil-Bindung',()=>result.semantic_contract_binding],['SQL-Snapshot Semantikprofil',()=>result.snapshot.resource_contract_engine]];
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
    $('resource-contract-use-current').addEventListener('click',useCurrentResourceContract);
    for(const id of ['resource-follow-internal-qualified','resource-follow-external-qualified'])$(id).addEventListener('change',changeResourceQualifiedLinks);
    $('settings-form').elements.resource_discovery_json.addEventListener('input',syncResourceContractSettings);
    $('settings-form').elements.resource_discovery_enabled.addEventListener('change',syncResourceContractSettings);
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
  const contentReportingMode = () => $('content-reporting-mode').value;
  function contentReportingQuery(token,inventory=null) {
    const query=new URLSearchParams();
    if(token.reportingMode)query.set('reporting_mode',token.reportingMode);
    if(inventory){query.set('expected_snapshot_sha256',inventory.snapshot.sha256);if(inventory.snapshot.generation!=null)query.set('expected_generation',String(inventory.snapshot.generation));if(inventory.reporting)query.set('expected_reporting_identity',inventory.reporting.identity_sha256);}
    return query.toString() ? '&'+query : '';
  }
  function validateContentReporting(result,token,inventory=null) {
    const reporting=result.reporting;
    if(reporting){const identity=reporting.identity;if(identity?.engine!=='open-seo-spider/saved-content-reporting-v1'||!['raw','reported'].includes(identity.mode)||token.reportingMode&&identity.mode!==token.reportingMode||!/^[a-f0-9]{64}$/.test(reporting.identity_sha256)||!/^[a-f0-9]{64}$/.test(reporting.eligible_raw_page_ordinals_sha256)||!Number.isSafeInteger(reporting.captured_page_observations)||!Number.isSafeInteger(reporting.profile_visible_page_observations)||!Number.isSafeInteger(reporting.profile_excluded_page_observations)||!Number.isSafeInteger(reporting.eligible_page_observations)||reporting.profile_visible_page_observations<0||reporting.profile_excluded_page_observations<0||reporting.profile_visible_page_observations+reporting.profile_excluded_page_observations!==reporting.captured_page_observations||reporting.eligible_page_observations!==(identity.mode==='raw'?reporting.captured_page_observations:reporting.profile_visible_page_observations))throw new Error('Ungültige Berichtsansicht oder Befundzählung.');}
    else if(token.reportingMode)throw new Error('Die ausdrücklich gewählte Berichtsansicht fehlt.');
    if(inventory && (inventory.reporting?.identity_sha256!==reporting?.identity_sha256 || inventory.reporting?.eligible_raw_page_ordinals_sha256!==reporting?.eligible_raw_page_ordinals_sha256))throw new Error('Gespeichertes Berichtsprofil oder sichtbare Befundidentität geändert. Quellseite neu laden.');
  }
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
    if ($('content-reporting-mode')) $('content-reporting-mode').value = '';
    if ($('content-search')) { $('content-search').value = ''; $('content-filter').value = 'all'; $('content-result-search').value = ''; }
  }
  function contentToken(kind) {
    const token = { kind,id:state.crawl.id,generation:state.generation,revision:state.content.revision,source:contentSource(),reportingMode:contentReportingMode(),controller:new AbortController() };
    state.content[kind] = token; return token;
  }
  function contentCurrent(token) { return state.content[token.kind] === token && token.id === state.crawl?.id && token.generation === state.generation && token.revision === state.content.revision && token.source === contentSource() && token.reportingMode === contentReportingMode(); }
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
      const result = await request(`/api/crawls/${encodeURIComponent(token.id)}/content?after_ordinal=${cursor}&limit=200&source_kind=${token.source}${contentReportingQuery(token,navigation ? content.inventory : null)}`,{signal:token.controller.signal});
      if (!contentCurrent(token)) return;
      validateContentEnvelope(result,token.source); validateContentReporting(result,token,navigation ? content.inventory : null);
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
    $('content-scope').textContent = (inventory?.reporting ? 'Ansicht '+inventory.reporting.identity.mode+' · '+number(inventory.reporting.eligible_page_observations)+' berechtigte / '+number(inventory.reporting.captured_page_observations)+' erfasste Befunde · '+number(inventory.reporting.profile_excluded_page_observations)+' durch gespeichertes Profil ausgeblendet. ' : '') + 'Quelle: ' + contentSource() + ' · Snapshot-Profil: ' + (snapshot?.sha_profile || 'noch nicht erfasst') + ' · Keine Website-Vollständigkeit abgeleitet. Befundalter, Textvollständigkeit und Herkunft bleiben getrennt.';
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
      const result=await request(`/api/crawls/${encodeURIComponent(token.id)}/content/document?url=${encodeURIComponent(url)}&source_kind=${token.source}${contentReportingQuery(token,content.inventory)}`,{signal:token.controller.signal});
      if (!contentCurrent(token)) return;
      validateContentEnvelope(result,token.source); validateContentReporting(result,token,content.inventory); validateContentRow(result.row,token.source);
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
    $('content-source-text').textContent=source.status==='excluded_reporting' ? 'Bekannte gespeicherte URL, durch das Berichtsprofil ausgeblendet. In dieser Ansicht werden keine Textquelle oder Analysewerte ausgegeben; Raw zeigt die gespeicherte Quelle.' : !supported ? 'Nicht unterstütztes Erfassungsprofil. Exakte Daten bleiben im JSON sichtbar; kein Text wird als geprüfte Quelle umgedeutet.' : result?.text == null ? `Kein gespeicherter Text. ${contentReason(row) || 'Quelle nicht verfügbar.'}` : result.text==='' ? '[Beobachtet leere Textauswahl: exakter Text ist eine leere Zeichenfolge.]' : result.text;
    $('content-source-evidence').textContent=JSON.stringify(source,null,2); $('content-source-json').disabled=false;
  }
  async function analyzeSavedContent() {
    const content=state.content;
    if(active() || actionBusy() || content.analysisToken || !content.inventory || content.error) return;
    let config; try { config=parseContentAnalysisConfig(); if(content.selected.size<1 || content.selected.size>128 || content.selected.size>(config.limits?.max_documents ?? 128)) throw new Error('1 bis höchstens max_documents/128 konkrete Anfrage-URLs auswählen; keine automatische Teilmenge.'); }
    catch(error) { content.analysisError=error.message; renderContent(); return; }
    invalidateContentRequests('Gespeicherte Texte werden offline berechnet …');
    const token=contentToken('analysisToken'),snapshot=content.inventory.snapshot;
    const body={urls:[...content.selected],source_kind:token.source,config,...(token.reportingMode?{reporting_mode:token.reportingMode}:{}),expected_snapshot_sha256:snapshot.sha256,...(content.inventory.reporting?{expected_reporting_identity:content.inventory.reporting.identity_sha256}:{}),indexable_only:$('content-indexable-only').checked,complete_only:$('content-complete-only').checked,...(snapshot.generation==null ? {} : {expected_generation:snapshot.generation})};
    renderContent();
    try {
      const result=await request(`/api/crawls/${encodeURIComponent(token.id)}/content/analyze`,{method:'POST',body:JSON.stringify(body),signal:token.controller.signal});
      if(!contentCurrent(token)) return;
      validateContentEnvelope(result,token.source); validateContentReporting(result,token,content.inventory);
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
    $('content-analysis-summary').textContent=(result.reporting ? 'Ansicht '+result.reporting.identity.mode+' · '+result.excluded_reporting_count+' durch Berichtsprofil ausgeschlossen · ' : '')+`${result.selected_count} ausgewählt · ${result.included_count} enthalten · ${result.excluded.length} ausdrücklich ausgeschlossen · ${contentOrigin(result.snapshot.origin)} · Snapshot ${result.snapshot.sha256}, Generation ${result.snapshot.generation ?? 'unbekannt'} · Quelle ${result.source_kind} · 0 Website-Anfragen. Keine Website-Vollständigkeit behauptet.`;
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
    const columns=[['Snapshot SHA256',()=>inventory.snapshot.sha256],['Snapshot Generation',()=>inventory.snapshot.generation],['Snapshot Origin',()=>inventory.snapshot.origin],...(inventory.reporting ? [['Berichtsmodus',()=>inventory.reporting.identity.mode],['Berichtsidentität SHA256',()=>inventory.reporting.identity_sha256],['Berechtigte Rohordinal-Identität SHA256',()=>inventory.reporting.eligible_raw_page_ordinals_sha256],['Erfasste Befunde',()=>inventory.reporting.captured_page_observations],['Berechtigte Befunde',()=>inventory.reporting.eligible_page_observations]] : []),...contentColumns];
    const csv=[columns.map(([label])=>cell(label)).join(','),...rows.map(row=>columns.map(([,value])=>cell(value(row))).join(','))].join('\r\n'); downloadBlob(new Blob(['\uFEFF',csv],{type:'text/csv;charset=utf-8'}),`open-seo-spider-${state.crawl.id}-content-page-${state.content.cursor}.csv`);
  }
  function bindContentEvents() {
    $('content-reporting-mode').addEventListener('change',()=>{state.content.selected.clear();invalidateContentRequests('Berichtsansicht geändert; Kohorte und Ergebnis neu wählen.',true);renderContent();loadContentInventory();});
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
    rendererViewport(settings);
    let endpoint;
    try { endpoint = new URL(settings.renderer_endpoint); } catch (_) { throw new Error('Der Browser-Worker benötigt einen gültigen lokalen HTTP-Endpunkt.'); }
    if (endpoint.protocol !== 'http:' || !['127.0.0.1', 'localhost', '[::1]', '::1'].includes(endpoint.hostname)) throw new Error('Der Browser-Worker muss per HTTP auf 127.0.0.1, localhost oder ::1 erreichbar sein.');
    if (!Number.isSafeInteger(settings.renderer_wait_ms) || settings.renderer_wait_ms < 0 || settings.renderer_wait_ms > 30000) throw new Error('Die Render-Wartezeit muss zwischen 0 und 30000 ms liegen.');
    if (settings.renderer_wait_ms >= Math.min(settings.timeout_seconds * 1000, 120000)) throw new Error('Die Render-Wartezeit muss kürzer sein als der konfigurierte Timeout.');
  }

  function toggleRendererSettings(enabled) {
    $('renderer-settings').hidden = !enabled;
    $('renderer-settings').querySelectorAll('input, textarea').forEach((input) => { input.disabled = !enabled; });
    const form=$('settings-form');
    for(const axis of ['width','height'])if(form.elements['renderer_viewport_'+axis])form.elements['renderer_viewport_'+axis].disabled=!enabled||!form.elements.renderer_viewport_enabled.checked;
  }

  function rendererViewport(settings) {
    if(!settings.renderer_viewport_enabled)return null;
    const viewport={width:Number(settings.renderer_viewport_width),height:Number(settings.renderer_viewport_height)};
    if(Object.values(viewport).some(value=>!Number.isInteger(value)||value<100||value>4096))throw Error('Die Browser-Ansicht benötigt Breite und Höhe als ganze Zahlen von100 bis4096 Pixeln.');
    return viewport;
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
    bindDashboardSegments();
    window.SavedComparisonUI?.bind({ downloadBlob, selectedJob: () => state.crawl?.id });
    bindResourceEvents();
    bindSitemapEvents();
    bindSourceBodyEvents();
    bindSitemapFindingsEvents();
    bindSitemapBundleEvents();
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
    // Search revision changes before the deferred render so late view operations cannot win.
    $('table-search').addEventListener('input', (event) => {
      tableQueryEdited();
      state.search = event.target.value;
      state.page = 0;
      clearTimeout(tableSearchTimer);
      const revision=tableQueryRevision,owner=JSON.stringify(tableOwner());
      tableSearchTimer=setTimeout(()=>{if(revision===tableQueryRevision&&owner===JSON.stringify(tableOwner())){tableSearchTimer=null;renderTable();}},100);
    });
    $('table-filter').addEventListener('change', (event) => { tableQueryEdited();state.filter = event.target.value; state.page = 0; renderTable(); });
    $('page-size').addEventListener('change', (event) => { state.pageSize = Number(event.target.value); state.page = 0; renderTable(); });
    $('page-prev').addEventListener('click', () => { state.page -= 1; renderTable(); });
    $('page-next').addEventListener('click', () => { state.page += 1; renderTable(); });
    $('inspector-close').addEventListener('click', () => { screenshotController?.invalidate();screenshotView=null;state.selected = null; state.selectedResponse = null; state.selectedInput = null; $('inspector').hidden = true; if (!['overview', 'graph'].includes(state.section)) renderTable(); });
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
    $('tls-certificate-files').addEventListener('change', stageTlsCertificateFiles);
    $('directive-reporting-load-selected').addEventListener('click', loadSelectedDirectiveReportingProfile);
    $('settings-form').elements.javascript_rendering.addEventListener('change', (event) => toggleRendererSettings(event.target.checked));
    $('settings-form').elements.renderer_viewport_enabled.addEventListener('change',()=>toggleRendererSettings($('settings-form').elements.javascript_rendering.checked));
    $('settings-form').elements.crawl_admission_limits_enabled.addEventListener('change', syncCrawlAdmissionSettings);
    $('settings-form').elements.crawl_quotas_enabled.addEventListener('change',syncQuotaSettings);
    $('crawl-quota-add').addEventListener('click',()=>{const rows=readQuotaRuleControls();if(rows.length>=32)return;rows.push({id:'',pattern:'',max_urls:''});renderQuotaRuleControls(rows);});
    $('crawl-quota-refresh').addEventListener('click',()=>loadQuotaSummary(true));
    $('crawl-quota-increase').addEventListener('click',openQuotaIncrease);
    $('quota-increase-close').addEventListener('click',()=>{quotaIncreaseContext=null;$('quota-increase-dialog').close();});
    $('quota-increase-dialog').addEventListener('cancel',()=>{quotaIncreaseContext=null;});
    $('quota-increase-form').addEventListener('submit',submitQuotaIncrease);
    $('settings-form').elements.sitemap_source_archive_enabled.addEventListener('change', syncSitemapSourceArchiveSettings);
    $('settings-form').elements.sitemap_source_processing_enabled.addEventListener('change', syncSitemapSourceProcessingSettings);
    $('settings-form').elements.sitemap_membership_limits_enabled.addEventListener('change', syncSitemapSourceProcessingSettings);
    $('export-csv').addEventListener('click', exportCSV);
    $('export-resources').addEventListener('click', () => serverDownload(domSections.includes(state.section) ? 'dom.csv' : ({ accessibility:'accessibility.csv?generation='+encodeURIComponent(state.crawl.generation), 'custom-search':'custom-search.csv',inputs: 'inputs.csv', pdf: 'pdf.csv', responses: 'responses.csv', fragments: 'fragments.csv', linkdom: 'links.csv', imagefiles: 'images.csv' })[state.section], `open-seo-spider-${state.crawl.id}-${state.section}.csv`));
    $('import-project').addEventListener('click', () => $('import-file').click());
    $('import-file').addEventListener('change', (event) => importProject(event.target.files[0]));
    $('export-sitemap').addEventListener('click', () => serverDownload('sitemap.xml', `open-seo-spider-${state.crawl.id}-sitemap.xml`));
    $('export-project').addEventListener('click', () => serverDownload('project.json', `open-seo-spider-${state.crawl.id}.json`));
    $('export-visible-report').addEventListener('click', () => serverDownload('report.json', `open-seo-spider-${state.crawl.id}-sichtbar.json`));
    document.addEventListener('visibilitychange', () => { if (!document.hidden && active() && !state.polling) refresh(); });
  }

  async function initialize() {
    $('advanced-config').append($('crawl-admission-settings-template').content.cloneNode(true));
    $('advanced-config').append($('crawl-quota-settings-template').content.cloneNode(true));
    $('advanced-config').append($('sitemap-source-archive-settings-template').content.cloneNode(true));
    $('advanced-config').append($('sitemap-source-processing-settings-template').content.cloneNode(true));
    $('advanced-config').append($('policy-settings-template').content.cloneNode(true));
    $('advanced-config').append($('content-settings-template').content.cloneNode(true));
    $('advanced-config').append($('custom-search-settings-template').content.cloneNode(true));
    $('advanced-config').append($('resource-discovery-settings-template').content.cloneNode(true));
    const searchTab=document.createElement('button');searchTab.type='button';searchTab.dataset.tab='custom-search';searchTab.setAttribute('role','tab');searchTab.setAttribute('aria-selected','false');searchTab.textContent='Eigene Suche';$('inspector-tabs').append(searchTab);
    restoreSettings();
    bindEvents();
    bindTableExplorer();
    bindNamedTableViews();
    bindProjectColumns();
    bindTableExport();
    selectSection('overview');
    render();
    await initializeConfigProfiles();
    try {
      await loadHistory();
      const running = state.crawls.find((crawl) => ['running', 'queued', 'pending'].includes(crawl.status));
      if (running) { await selectCrawl(running.id); $('crawl-history').value = running.id; }
    } catch (error) { connection(false); message(`Der lokale Crawler-Server ist nicht erreichbar: ${error.message}`, true); }
  }

  initialize();
})();
