import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

// Editorial German labels. Acceptance/status/evidence remain catalog-owned.
const labels = `
modes.spider|Website rekursiv crawlen|Der rekursive Kern ist gegen eigene Fixtures geprüft.
modes.list|URL-Listen importieren und crawlen|Geordnete Listen mit Herkunft, Duplikaten und ungültigen Zeilen sind geprüft.
modes.serp|SERP-Listen offline verarbeiten|Der eigene Offline-Modus mit null Website-Anfragen ist geprüft.
modes.apis|URLs ausschließlich über APIs anreichern|API-Anreicherung, fehlende Daten und null Website-Anfragen prüfen.
modes.compare|Gespeicherte Crawls vergleichen|Neue, entfernte und veränderte Seiten gemeinsam in einer Fixture prüfen.
modes.control|Pausieren, stoppen und fortsetzen|Anfragen abarbeiten, persistieren und nach Neustart ohne fehlende Ergebnisse fortsetzen ist geprüft.
modes.recrawl|Ausgewählte URLs erneut crawlen|Erneute ausgewählte Anfragen mit unveränderten übrigen Ergebnissen und Elternhistorie sind geprüft.
crawl.unlimited|Crawlen ohne kommerzielle URL-Grenze|Mehr als 500 URLs sind im eigenen Crawl geprüft; Ressourcenbudgets bleiben ausdrücklich sichtbar.
crawl.scope|Host-, Subdomain- und Ordnergrenzen|Alle Scope-Varianten gemeinsam mit echten HTTP-/DNS-, Port- und Scheme-Fixtures prüfen.
crawl.resources|Ressourcentypen getrennt speichern und crawlen|Unabhängige Schalter und Fixtures für CSS, JavaScript, Medien, PDF und SWF ergänzen.
crawl.relationships|Beziehungs-URLs gezielt entdecken|Eigene Entdeckungsschalter für Canonical, Pagination, AMP, mobile Alternativen und Refresh ergänzen.
crawl.normalization|URL-Identitäten nachvollziehbar normalisieren|Unicode, Prozentkodierung und wiederholte Query-Parameter ohne unbelegte Zusammenführung prüfen.
crawl.rewrite|Explizite URL-Umschreibungen|Query-Regeln und begrenzte Regex-Regeln erhalten Original- und Fetch-Identitäten samt Ledger.
crawl.include_exclude|Ein- und Ausschlussregeln|Ressourcen und Beziehungsquellen vollständig prüfen; Browser-Navigation und Subressourcen bleiben offen.
crawl.cdns|Ausgewählte CDN-Hosts intern behandeln|Konfigurierte CDN-Grenzen ohne unbegrenzte externe Traversierung ergänzen.
crawl.nofollow|Nofollow- und verwandte Traversierungsregeln|Alle vier Direktiven mit konfigurierbarem Verhalten und erhaltenen Linkbeobachtungen prüfen.
crawl.limits|Vom Nutzer gewählte Crawl-Grenzen|Grenzen für Tiefe, Ordner, Query, Subdomains, URL-Länge und Linkanzahl ergänzen und einzeln prüfen.
crawl.speed|Rate und Parallelität begrenzen|Per-Origin-Rate und Parallelität mit Zeitstempeln und verzögerten Antworten prüfen.
crawl.timeouts_retries|Timeouts und begrenzte Wiederholungen|Backoff und getrennte DNS-, Netzwerk-, Timeout- und 5xx-Zustände ergänzen.
crawl.headers_proxy|Header, Authentifizierung und Proxy|Proxy-Konfiguration prüfen und Geheimnisse aus gemeinsam exportierten Einstellungen entfernen.
crawl.tls|TLS und HSTS nachvollziehbar behandeln|Vertrauenswürdige Testzertifikate und HSTS-Herkunft ohne globale Abschaltung der Prüfung testen.
robots.fetch|Robots-Dateien abrufen und cachen|Cache-Grenzen für Scheme, Host und Port mit unterschiedlicher Policy prüfen.
robots.matching|Robots-Agenten und Regeln auswerten|Vollständige Agentenauswahl und Endanker mit gerichteten Fixtures prüfen.
robots.test|Robots-Regeln offline simulieren|Der gemeinsame Entscheidungsmechanismus und null Website-Anfragen sind geprüft.
robots.override|Explizite Robots-Ausnahme|Gespeicherten Override und seine sichtbare Wirkung auf tatsächliche Anfragen prüfen.
robots.meta|Meta-Robots und X-Robots-Tag|Der eigene Direktiven- und Whitespace-Vertrag ist geprüft.
robots.indexability|Indexierbarkeit mit Gründen erklären|Jede Erklärung und Agenten-spezifische Direktive in einer vollständigen Fixture prüfen.
robots.traversal|Expansion nach Indexierungsregeln steuern|Noindex-, Canonical- und Pagination-Regeln anwenden, ohne Beobachtungen zu unterdrücken.
http.response|Initiale und finale HTTP-Antworten erfassen|Transferbytes, Timing-Phasen und mehrfach vorhandene Header vollständig erhalten.
http.failures|HTTP- und Netzwerkfehler unterscheiden|Maschinenlesbare 5xx-, DNS-, Timeout- und ungültige URL-Zustände vollständig prüfen.
http.redirects|Redirect-Ketten und Stop-Gründe|Weitere Endzustände und Kombinationen prüfen; fehlende Zwischenantworten ausdrücklich unbekannt lassen.
http.refresh|HTTP-, Meta- und JavaScript-Weiterleitungen|Die verschiedenen Mechanismen mit eigenen Fixtures getrennt erkennen und benennen.
http.migration|Migrationsziele überprüfen|Alte und erwartete neue URLs aus CSV mit tatsächlichen Endzielen vergleichen.
http.broken_sources|Quellen defekter Links exportieren|Alle Quellvorkommen mit Dokumentposition und Pfad exportieren.
metadata.titles|Seitentitel und Pixelbreiten|Weitere Schrift-/DPI-Profile und komplexe Sprachformung getrennt kalibrieren; unbekannte Glyphen nicht raten.
metadata.descriptions|Meta-Descriptions und Pixelbreiten|Weitere Schrift-/DPI-Profile und komplexe Sprachformung getrennt kalibrieren; unbekannte Glyphen nicht raten.
metadata.headings|Überschriften und Reihenfolge|Der eigene Vertrag einschließlich begrenzter H1-/H2-Referenz-Fixtures ist geprüft.
metadata.serp_preview|SERP-Vorschau offline bearbeiten|Die eigene Vorschau mit Versionierung, Import und Race-Prüfungen ist geprüft.
metadata.area|Inhaltsbereich per CSS auswählen|Native Roh-/DOM-Textbindung ist geprüft; Offline-Neuauswahl per geändertem CSS braucht noch erhaltenes HTML/DOM und eigene Artefaktbudgets.
metadata.wordcount|Wörter und Textanteil zählen|Versteckte Inhalte, Absätze und Text-/Code-Verhältnis explizit prüfen.
metadata.readability|Lesbarkeit bewerten|Gespeicherte Roh-/DOM-Quelle ist angebunden; breitere Sprach-/Silben-/Klassifikationsprofile und größere Corpora weiter prüfen.
metadata.exact|Exakt gleiche Inhalte gruppieren|Identische und um ein Zeichen veränderte Dokumente mit Digest-Herkunft gemeinsam prüfen.
metadata.near|Ähnliche Inhalte vergleichen|Ausgewählte gespeicherte Quellen und Aktualität sind angebunden; vollständige Website-/große Corpus-Analyse und Referenz-MinHash-Prädikate bleiben offen.
metadata.spelling|Rechtschreibung und Grammatik|Mehrsprachige Wörterbücher, Ausnahmen und genaue Fundstellen auf gespeichertem Text prüfen.
metadata.ngrams|Überlappende N-Gramme auswerten|Gespeicherte Crawl-Texte und Quellrevisionen sind angebunden; große Corpus-Aggregation und vollständige spezielle N-Gramm-Exporte weiter prüfen.
links.occurrences|Jedes Linkvorkommen erhalten|Linkvorkommen und Ressourcentypen werden im eigenen Vertrag getrennt erhalten.
links.positions|Linkpositionen und DOM-Pfade|Der eigene Positionsvertrag mit Dokumentreihenfolge ist geprüft.
links.depth|Minimale Klicktiefe bestimmen|Mehrere Wege und spät entdeckte kürzere Wege in einer vollständigen Graph-Fixture prüfen.
links.score|Nachvollziehbaren Link-Score berechnen|Algorithmus und Parameter mit Versionskennung in gespeicherter Graph-Evidenz ergänzen.
links.orphans|Orphan-Kandidaten aus Quellen abgleichen|Sitemap-, GA4- und GSC-Herkunft gemeinsam abgleichen; unvollständigen Crawl sichtbar begrenzen.
links.uncrawlable|Nicht regulär crawlbare Navigation beobachten|Onclick- und weitere Client-Muster einzeln prüfen, ohne Navigation zu erfinden.
links.fragments|Fragmentziele und Bookmarks prüfen|Der eigene opt-in Vertrag einschließlich leerer und unbekannter Ziele ist geprüft.
links.crawl_graph|Crawl-Graph interaktiv untersuchen|Kürzeste-Wege-Hierarchie, Fokus, Aufklappen und Status-/Indexierbarkeitsfarben prüfen.
links.directory_graph|Verzeichnis-Hierarchie anzeigen|Künstliche Gruppierungspfade von tatsächlich abgerufenen URLs unterscheiden.
links.graphs_3d|Große Graphen räumlich anzeigen|Begrenzung und Fokus großer Graphen sichtbar machen, statt Knoten still wegzulassen.
links.segments|Logische Segmente bilden|Geordnete Feld-/Regex-Segmente auf Zählungen, Issues, Diagramme und Exporte anwenden.
relationships.canonical|Canonical-Deklarationen erhalten|Der eigene HTML-/HTTP-Deklarationsvertrag ist geprüft.
relationships.canonical_checks|Canonical-Ketten und Zielzustände prüfen|Ungültige Deklarationen, Ketten, Schleifen und tatsächliche Zielzustände vollständig prüfen.
relationships.pagination|Pagination-Beziehungen prüfen|Lange Ketten, Schleifen und konfigurierbare Entdeckung ergänzen.
relationships.hreflang|Hreflang aus mehreren Quellen erfassen|XML-Sitemap-Alternativen mit HTML-/HTTP-Deklarationen zusammenführen.
relationships.hreflang_checks|Hreflang-Cluster und Rückverweise prüfen|Sprach-/Regioncodes und identische doppelte Deklarationen vollständig prüfen.
relationships.amp|AMP-Paare und Validierung|Rückverweise und einen fest versionierten öffentlichen AMP-Validator ergänzen.
relationships.mobile|Mobile Alternativen prüfen|Gegenseitige Verweise und tatsächliche Zielzustände in eigenen Fixtures prüfen.
images.sources|Bildquellen und responsive Kandidaten|Mehrseitige Vorkommen und tatsächlich gewähltes currentSrc mit Media-/Dichteauswahl prüfen.
images.alt|Alt-Attribute unterscheiden|Fehlendes, leeres und ausschließlich Whitespace enthaltendes Alt explizit prüfen.
images.size|Bildabmessungen und Größen|Gerenderte Maße, echte Transferbytes, Schwellen und zusätzliche Format-/Orientierungsfälle prüfen.
images.cookies|Cookies als Beobachtungen erfassen|Der eigene Cookie-Vertrag mit getrennten Browser-/HTTP-Beobachtungen ist geprüft.
images.pdf|PDF-Metadaten und Text erfassen|Schriftabdeckung, Form XObjects und gescannte PDFs/OCR getrennt unterstützen und prüfen.
images.pdf_links|Links und Anker in PDFs|Zuverlässigen Ankertext und weitere Link-Aktionen prüfen; Nicht-HTTP-Aktionen nicht zu Weblinks umdeuten.
rendering.text|Native HTML-Auswertung|Der eigene native HTML-Extraktionsvertrag ist geprüft.
rendering.chromium|JavaScript mit Chromium rendern|JS-erzeugte Links sowie Browser-, Viewport- und Wartezeit-Herkunft sind tatsächlich geprüft.
rendering.wait|Rendern, Warten und Ressourcen steuern|Viewport, Ressourcen-/Robots-Regeln und Timeout-Arten durchgehend konfigurieren und prüfen.
rendering.source_diff|Rohes HTML und DOM vergleichen|Beide vollständigen HTML-Repräsentationen erhalten und konkrete hinzugefügte/entfernte Elemente vergleichen.
rendering.screenshot|Screenshots nachvollziehbar speichern|Viewport im Crawl/UI auswählbar machen und Bild-/Artefakt-Prüfungen ergänzen.
rendering.console|Konsole und Netzwerk beobachten|Browser-Subressourcen nach Robots-Regeln steuern und blockierte Ressourcen mit Gründen prüfen.
rendering.flatten|Iframes und Shadow DOM behandeln|Zusätzliche DOM-Quellen mit Herkunft und Grenzen explizit erfassen.
rendering.archive|HTML-/DOM-Archive erhalten|Architektur und Artefakte für spätere Offline-Auswertung und Wiederöffnung prüfen.
rendering.custom_extract|Eigene CSS-, XPath- und Regex-Extraktion|Unterstützte Profile und Grenzen vollständig gegen Sonderfälle prüfen; keine umfassende XPath-Parität behaupten.
rendering.visual_extract|Extraktion im Browser auswählen|Visuelle Auswahl in reproduzierbare Regeln übersetzen und getrennt prüfen.
rendering.custom_search|Eigene Text- und Quellcodesuchen|Rohes HTML, DOM und ausgewählten Text mit Fundanzahlen und Match-Herkunft durchsuchen.
rendering.javascript|Eigene JavaScript-Auswertung|Der eigene opt-in Worker-Vertrag mit Grenzen und Herkunft ist geprüft.
rendering.interact|Klick-, Hover- und Scroll-Aktionen|Autorisierte Aktionen in erklärter Reihenfolge ausführen und resultierende Links prüfen.
rendering.snippet_library|Eigene Skriptbibliothek|Nutzer-Snippets ohne Geheimnisse importieren und exportieren.
quality.structured_formats|JSON-LD, Microdata und RDFa|Microdata/RDFa samt Rohknoten-Herkunft und verschachtelten Graph-Fixtures ergänzen.
quality.schema|Schema.org-Regeln prüfen|Typen und Eigenschaften gegen eine benannte Version prüfen, einschließlich unbekannter und veralteter Begriffe.
quality.richresults|Rich-Result-Regeln prüfen|Öffentliche versionierte Pflicht-/Empfehlungsregeln mit getrennten Fehlern und Warnungen prüfen.
quality.accessibility|Automatisierte Barrierefreiheit prüfen|Der eigene axe-basierte Browser-Vertrag ist tatsächlich geprüft.
quality.accessibility_limit|Grenzen automatisierter Accessibility zeigen|Needs-review und unvollständige Abdeckung durch UI und Spezialberichte erhalten.
quality.mobile|Mobile Nutzbarkeit der Zielseite prüfen|Tap-Ziele und Inhalte auf tatsächlich gerenderter Website prüfen.
quality.html_head|Ungültige Head-Struktur erkennen|Ungültige Head-Kinder und Auswirkungen auf Metadaten mit Rohmarkup-Herkunft prüfen.
quality.html_validation|HTML formal validieren|Versionierten Validator mit Zeile, Spalte und Regel auf eigenen Fehler-Fixtures prüfen.
quality.security|Transport- und Seiten-Sicherheitsprüfungen|Unsichere Formulare, MIME-Abweichungen, Header-Regeln und Transportfälle vervollständigen.
quality.carbon|CO₂-Schätzung nachvollziehbar berechnen|Datenbasis, Methode und Version angeben; fehlende Werte unbekannt lassen und Schätzung benennen.
sitemap.discovery|XML-Sitemaps entdecken|Gzip, Indexschleifen und fehlerhaftes XML über echte Requests prüfen.
sitemap.coverage|Sitemap-Mitgliedschaft abgleichen|Vollständige Mitgliedschaft speichern und Sitemap-only sowie nicht gelistete Crawl-URLs getrennt prüfen.
sitemap.validity|Sitemap-Felder und Zielzustände validieren|Lastmod erhalten/prüfen und Redirect-, Non-200- sowie nicht indexierbare Ziele getrennt melden.
sitemap.generate|Sitemaps aus gespeicherten URLs erzeugen|Lastmod, Bilder und Hreflang unterstützen und Ausgabe unabhängig parsen.
sitemap.split|Sitemaps teilen und indexieren|URL-/Byte-Grenzen, Indexdatei und exakte Segmentmitgliedschaft prüfen.
integrations.ga4|Google Analytics 4 anbinden|Autorisierte Property, URL-Abgleich und fehlende Daten getrennt von Nullwerten prüfen.
integrations.gsc|Google Search Console anbinden|Datum/Dimensionen auswählen und Kennzahlen samt Fehlern und Abdeckung erhalten.
integrations.inspection|URL-Inspection anbinden|Quoten einhalten und inspizierte von wartenden URLs samt Zeitpunkt unterscheiden.
integrations.psi|PageSpeed Insights anbinden|Mobil/Desktop, Feld/Labor, Einheiten und Providerfehler getrennt erhalten.
integrations.cwv|Core Web Vitals aus Felddaten|Pass/Fail aus vorhandenen Felddaten bestimmen; fehlende Felddaten nicht durch Labordaten ersetzen.
integrations.majestic|Majestic-Metriken anbinden|Separat autorisierte Credentials, Datensätze, Quoten und URL-/Domain-Ebene prüfen.
integrations.ahrefs|Ahrefs-Metriken anbinden|Eigene autorisierte API-Zugänge, Ablauf, Quoten und Metrik-Abgleich prüfen.
integrations.moz|Moz-Metriken anbinden|Autorisierte URL-/Domain-Abfragen samt fehlenden Daten und Quoten prüfen.
integrations.oauth|Provider-Konten sicher verbinden|Hinzufügen, Trennen und Widerrufen prüfen; Tokens aus Logs und Versionsverwaltung heraushalten.
integrations.provider_urls|Provider-only URLs mit Herkunft|Optionales Enqueue mit Scope-Prüfung und unverfälschter Orphan-Evidenz ergänzen.
ai.providers|Optionale KI-Provider anbinden|Mock-Provider und lokales Ollama mit getrennten Modell-/Providerfehlern prüfen.
ai.prompts|Eigene Prompts auf ausgewählten Daten|Exakte Eingabe vorzeigen und typisierte Ergebnisse mit Herkunft speichern.
ai.prompt_library|Prompt-Bibliothek|Nutzer-Prompts mit Modell/Einstellungen und ohne Geheimnisse roundtrippen.
ai.embeddings|Text-Embeddings versioniert speichern|Modell-/Input-Hash binden und unpassende Dimensionen ablehnen.
ai.similarity|Vektorähnlichkeit nachvollziehbar berechnen|Eigene bekannte Vektoren, Cosinus-Methode und Schwellen gemeinsam prüfen.
ai.outliers|Semantische Ausreißer bestimmen|Exakte Scores und erklärte Corpus-Grenzen zu jedem Ausreißer erhalten.
ai.semantic_search|Semantische Suche und Cluster|Gespeicherte Vektoren mit Modell-/Dimensions-Herkunft durchsuchen und darstellen.
ai.redirect_mapping|Semantische Redirect-Vorschläge|Zielkandidaten deterministisch ranken und Scores exportieren; Änderungen ausdrücklich getrennt halten.
ai.usage|KI-Nutzung und Budgets begrenzen|Rate, Kosten, Wiederholungen und lokale/entgeltliche Provider auseinanderhalten.
storage.database|Persistente SQLite-Crawls|Der eigene persistente Kern mit Wiederherstellung ist geprüft.
storage.memory|Speichermodus vergleichen|Gleiche Fixtures in RAM und SQLite mit gleichen Beobachtungen und erklärten Unterschieden prüfen.
storage.snapshots|Versionierte Crawl-Snapshots|Schema-Version und vollständige optionale Artefakte roundtrippen.
storage.projects|Projekte und Crawl-Historie|Benannte Projekte, Labels und Erstellungs-/Konfigurationsidentität prüfen.
storage.retention|Aufbewahrung und Sperren|Testzeitpunkte, gesperrte Snapshots und aktive Jobs bei automatischer Bereinigung prüfen.
storage.config|Konfiguration separat importieren/exportieren|Credentials, Pfade und unveränderte Einstellungen bei Roundtrips prüfen.
storage.portable|Portable Projekte und Archive|In anderem Verzeichnis öffnen und Manifest, Version sowie alle erklärten Artefakte prüfen.
storage.compare_changes|Veränderte Inhalte und Regeln vergleichen|Graph-/Schema-Deltas und Änderungen bei gleicher Issue-Anzahl prüfen.
storage.compare_mapping|URL-Mapping vor Vergleichen|Staging-/Produktions- oder Migrationspräfixe mit sichtbaren Kollisionen abgleichen.
storage.compare_export|Vergleiche vollständig exportieren|Gemeinsamen Export mit allen Delta-Arten und beiden Snapshot-Herkünften prüfen.
automation.tables|Tabellen und gefilterte Exporte|XLSX, Filterübereinstimmung, Formelinjektion, Unicode und Newlines vollständig prüfen.
automation.reports|Spezialisierte Auditberichte|Versionierten Überblick und eigene Redirect-, Canonical-, Hreflang-, Sitemap-, Security- und Content-Berichte prüfen.
automation.assets|HTML, Screenshots und Ressourcen exportieren|Rohes HTML, DOM, Ressourcen, Cookies und PDFs mit Artefaktmanifest exportieren.
automation.looker|Looker-/Cloud-Export|Autorisierte Ziele, Append/Replace und stabiles Spaltenschema über mehrere Läufe prüfen.
automation.cli|Lokale Kommandozeile|Der eigene CLI-Vertrag mit Offline- und Crawl-Funktionen ist geprüft.
automation.scheduler|Wiederkehrende lokale Crawls|Gespeicherte Aufgaben, Überlappung, Historie und Fehler mit Testuhr prüfen.
automation.auto_compare|Nach Crawls automatisch vergleichen|Zwei abgeschlossene Projekt-Snapshots nach gespeicherten Regeln automatisch vergleichen.
automation.notifications|Benachrichtigungen an konfigurierte Ziele|Eigene lokale SMTP-Testsenke samt Zusammenfassung, Deltas und Attachment-Fehlern prüfen.
automation.mcp|Lokale MCP-Werkzeuge|Der eigene MCP-Werkzeugvertrag ist gegen offizielle SDK-Clients geprüft.
automation.mcp_transport|MCP über stdio und HTTP|Beide eigenen Transportarten sind gegen offizielle SDK-Clients geprüft.
automation.stats|Nutzung über mehrere Crawls auswerten|Aggregate und standardmäßig abgeschaltete Telemetrie ausdrücklich prüfen.
ui.realtime|Live-Fortschritt und Steuerung|Verzögerte Updates, Pause und Konfigurationseinsicht im Browser vollständig prüfen.
ui.filter|Tabellen filtern und anpassen|Spaltenpräferenzen wiederöffnen und kombinierte Zahlen-/Textfilter prüfen.
ui.overview|Issue-Überblick und Drill-down|Schweregrad, Typ, Anzahl und Details aller beobachteten Issue-Familien konsistent prüfen.
ui.details|Quellen und Seiten-Details inspizieren|Vollständiges Roh-HTML/DOM, Cookies, Ressourcen und strukturierte Knoten im Detail zeigen.
ui.charts|Diagramme und Aggregate|Ordner-, Tiefen-, Status- und Timing-Aggregate mit gemeinsamen Filtermengen prüfen.
ui.localization|Sprache, Theme und Schrift wählen|UI-Einstellungen ändern, während gespeicherter Crawl-Text exakt unverändert bleibt.
`.trim().split('\n').map(line => line.split('|'));

const groupNames = {
  modes: 'Crawl-Modi und Steuerung', crawl: 'Entdeckung, Scope und URL-Regeln',
  robots: 'Robots und Indexierbarkeit', http: 'HTTP, Fehler und Weiterleitungen',
  metadata: 'Metadaten und Inhalte', links: 'Links und Website-Architektur',
  relationships: 'Canonical, Sprachen und alternative Seiten', images: 'Bilder, Cookies und PDFs',
  rendering: 'Browser-Rendering und eigene Extraktion', quality: 'Validierung und Qualitätsprüfungen',
  sitemap: 'Sitemaps', integrations: 'Autorisierte externe APIs', ai: 'Optionale KI und semantische Analyse',
  storage: 'Speicherung, Projekte und Vergleiche', automation: 'Automation, Exporte und MCP',
  ui: 'Oberfläche für die Analyse',
};

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const args = new Set(process.argv.slice(2));
if ([...args].some(arg => arg !== '--check')) throw new Error('Usage: node scripts/update-feature-todo.mjs [--check]');
const read = name => JSON.parse(fs.readFileSync(path.join(root, name), 'utf8').replace(/^\uFEFF/, ''));
const catalog = read('docs/feature-parity.json');
const ledger = read(catalog.granular_acceptance.artifact);
const labelMap = new Map(labels.map(([id, name, next]) => [id, { name, next }]));
const statuses = { implemented: 'Eigene Abnahme erfüllt', partial: 'Teilweise geprüft', planned: 'Offen' };
const ids = new Set();
const counts = { implemented: 0, partial: 0, planned: 0, total: 0 };
for (const feature of catalog.features) {
  if (ids.has(feature.id)) throw new Error(`Duplicate feature ID: ${feature.id}`);
  ids.add(feature.id);
  if (!labelMap.has(feature.id)) throw new Error(`Missing German label: ${feature.id}`);
  if (!groupNames[feature.group] || !(feature.status in statuses)) throw new Error(`Invalid feature shape: ${feature.id}`);
  if (!Array.isArray(feature.remaining_acceptance) || !Array.isArray(feature.implementation_evidence)) throw new Error(`Missing acceptance/evidence: ${feature.id}`);
  if (feature.status === 'implemented' && feature.remaining_acceptance.length) throw new Error(`Implemented feature has remaining acceptance: ${feature.id}`);
  if (feature.parity_status !== 'NOT_PROVEN') throw new Error(`Review parity wording before generation: ${feature.id}`);
  counts[feature.status]++; counts.total++;
}
if (counts.total !== 144 || labelMap.size !== ids.size) throw new Error('Review the complete grouped inventory before generating a new scope.');
if (JSON.stringify(counts) !== JSON.stringify(catalog.status_summary.overall)) throw new Error('Catalog status summary does not agree with its feature records.');
if (ledger.filters.length !== 416 || ledger.workflow_contracts.length !== 32) throw new Error('Review granular inventory count changes explicitly.');
const catalogSha = createHash('sha256').update(fs.readFileSync(path.join(root, 'docs/feature-parity.json'))).digest('hex');
const runMap = new Map(catalog.verification_runs.map(run => [run.id, run]));
const lines = [
  '# Open SEO Spider – Feature-To-do-Liste', '',
  '> Automatisch aus [feature-parity.json](feature-parity.json) erzeugt. Aktualisieren: `node scripts/update-feature-todo.mjs`. Unveränderten Stand prüfen: `node scripts/update-feature-todo.mjs --check`.', '',
  'Die Liste hält alle 144 gruppierten Fähigkeiten fest. Ein Haken bedeutet, dass das **eigene gespeicherte Abnahmekriterium** mit Beweisen erfüllt ist. Er bedeutet keine vollständige Gleichheit mit Screaming Frog. Diese Referenzparität bleibt für alle Einträge **NOT_PROVEN**.', '',
  `Aktueller Katalogstand: **${counts.implemented} eigene Abnahmen erfüllt**, **${counts.partial} teilweise geprüft**, **${counts.planned} offen**. Die Zahlen sind keine Aussage über den Anteil aller Herstellerfunktionen.`, '',
  'Die [416 benannten Filter und 32 Ablaufverträge](parity/granular-acceptance.json) bilden ein zusätzliches Register. Ihre positiven Fälle, Kontrollfälle, Parameter und Grenzfälle müssen jeweils geprüft werden. Ein implementierter Kern, ein leerer Export oder übereinstimmende CSV-Zellen ersetzen diese Einzelabnahme nicht.', '',
  '## Aktuelle Arbeit und nächste Schritte', '',
];
for (const work of catalog.current_work ?? []) {
  lines.push(`- [${work.status === 'COMPLETED' ? 'x' : ' '}] **${work.title_de}** — ${work.status}. ${work.summary_de}`);
  if (work.completed_acceptance_de?.length) lines.push(...work.completed_acceptance_de.map(item => `  - [x] ${item}`));
  if (work.pending_acceptance_de?.length) lines.push(...work.pending_acceptance_de.map(item => `  - ${item}`));
  if (work.documentation?.length) lines.push(`  - Verträge: ${work.documentation.map(file => `[${path.basename(file)}](${path.relative('docs', file).replaceAll('\\', '/')})`).join(', ')}.`);
  if (work.gates) lines.push(`  - Gate-Stand: ${Object.entries(work.gates).map(([id, result]) => `${id}: ${result}`).join('; ')}.`);
  lines.push('');
}
lines.push(
  'Das nächste Häkchen entsteht erst nach nachvollziehbaren Tests der offenen Kriterien und einem passenden Beweis im Katalog. Änderungen an Status oder Abnahme erfolgen dort; diese Datei wird danach neu erzeugt.', '',
  '## Vollständige Liste nach Gruppen', '',
  'Die Orientierungssätze sind eine deutsche Arbeitshilfe. Die eingeklappten Quellkriterien enthalten die unveränderte technische Abnahme, alle offenen Kriterien und alle implementierungsbezogenen Belege aus dem Katalog.', '',
);
for (const group of catalog.groups) {
  const features = catalog.features.filter(feature => feature.group === group.id);
  const groupCounts = { implemented: 0, partial: 0, planned: 0 };
  features.forEach(feature => groupCounts[feature.status]++);
  lines.push(`### ${groupNames[group.id]}`, '', `${features.length} Einträge: ${groupCounts.implemented} eigene Abnahmen erfüllt, ${groupCounts.partial} teilweise geprüft, ${groupCounts.planned} offen.`, '');
  for (const state of ['partial', 'planned', 'implemented']) {
    const selected = features.filter(feature => feature.status === state);
    if (!selected.length) continue;
    lines.push(`#### ${state === 'implemented' ? 'Geprüfter eigener Kern' : state === 'partial' ? 'Teilweise vorhanden – Restarbeiten' : 'Noch offen'}`, '');
    for (const feature of selected) {
      const copy = labelMap.get(feature.id);
      lines.push(`- [${state === 'implemented' ? 'x' : ' '}] **${copy.name}** — ${statuses[state]}; \`${feature.id}\`, ${feature.phase}.`, '', `  Orientierung: ${copy.next}`, '');
      const evidenceIds = [...new Set(feature.implementation_evidence.flatMap(evidence => typeof evidence === 'object' ? evidence.verification_run_ids ?? [] : []))];
      if (evidenceIds.length) {
        const links = evidenceIds.map(id => {
          const run = runMap.get(id);
          if (!run) throw new Error(`Unknown verification run ${id} on ${feature.id}`);
          const suffix = run.result ? ` (${run.result})` : '';
          return run.artifact ? `[${id}](${path.relative('docs', run.artifact).replaceAll('\\', '/')})${suffix}` : `\`${id}\`${suffix}`;
        });
        lines.push(`  Beweisregister: ${links.join(', ')}.`, '');
      } else lines.push('  Beweisregister: keine zugeordnete ausgeführte Prüfung.', '');
      lines.push('  <details>', '  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>', '', '  ```json');
      const proof = { id: feature.id, reference_name: feature.name, status: feature.status, acceptance_test: feature.acceptance_test, remaining_acceptance: feature.remaining_acceptance, implementation_evidence: feature.implementation_evidence, parity_status: feature.parity_status };
      lines.push(...JSON.stringify(proof, null, 2).split('\n').map(line => `  ${line}`), '  ```', '', '  </details>', '');
    }
  }
}
lines.push('## Pflege und Beweisgrenzen', '',
  '1. Die offene Abnahme im JSON als konkrete Fixture oder Runtime-Prüfung ausarbeiten.',
  '2. Positive Fälle, Kontrollfälle, unbekannte Daten und Ressourcen-/Corpus-Grenzen zusammen prüfen.',
  '3. Prüfergebnis, Artefakt, Engine-/Profilversion und den tatsächlichen Geltungsbereich im Katalog festhalten.',
  '4. Erst bei vollständig erfüllter eigener Abnahme den Status auf `implemented` setzen; Referenzparität separat nachweisen.',
  '5. Den Generator ausführen und mit `--check` prüfen. Historische Beweise bleiben erhalten.', '',
  'Offline-Auswertung mit geändertem CSS-Inhaltsbereich braucht gespeichertes HTML beziehungsweise DOM. Ein bereits ausgewählter Text mit HTML-Hash reicht zur erneuten Auswahl nicht aus. Dieser zusätzliche Workflow bleibt offen, solange keine vollständigen Quellartefakte und entsprechende Offline-Prüfung existieren.', '',
  `Reproduktionsbindung: SHA-256 des verwendeten Katalogs \`${catalogSha}\`. Letzter im Katalog erfasster Beweisstand: \`${catalog.last_verified_at ?? catalog.implementation_review_date ?? catalog.catalog_date}\`. Ein laufender B8-Arbeitsstand aktualisiert frühere Beweise nicht.`, '',
);
const output = `${lines.join('\n').trimEnd()}\n`;
const target = path.join(root, 'docs/FEATURE_TODO.md');
if (args.has('--check')) {
  if (!fs.existsSync(target) || fs.readFileSync(target, 'utf8') !== output) throw new Error('FEATURE_TODO.md is stale. Run node scripts/update-feature-todo.mjs.');
  process.stdout.write(`PASS: ${counts.total} unique grouped capabilities; ${counts.implemented}/${counts.partial}/${counts.planned}; 416 filters; 32 workflows.\n`);
} else {
  fs.writeFileSync(target, output, 'utf8');
  process.stdout.write(`Written docs/FEATURE_TODO.md: ${counts.total} grouped capabilities, with exact catalog acceptance/evidence.\n`);
}
