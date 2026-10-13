# Open SEO Spider – Roadmap zur vollständigen Funktionstiefe

Diese Roadmap ordnet die nächsten Arbeitsschritte. Der vollständige laufende Status aller 144 Fähigkeiten steht in [FEATURE_TODO.md](FEATURE_TODO.md); die exakten Abnahmen und Belege stehen in [feature-parity.json](feature-parity.json).

Aktuell: **30 eigene Abnahmen erfüllt, 68 teilweise geprüft, 46 offen**. Zusätzlich bleiben [416 benannte Filter und 32 Ablaufverträge](parity/granular-acceptance.json) getrennt zu prüfen. Alle Hersteller-Parität bleibt **NOT_PROVEN**. Es wird kein Fertigstellungsprozentsatz berechnet.

## Aktueller nächster Baustein

**B9 ist als begrenzte Grundlage abgeschlossen; seine frühere lokale Installation ist historisch belegt.** Der [Aggregatbeleg](verification/resource-discovery-batch.json) bindet 362 Standard-Rust-Tests, API9, MCP22 und tatsächliche UI65. Alle vier [exakten CI-Jobs an Commit f107ce7](verification/resource-discovery-ci.json) sind grün. Der [unveränderliche Windows-Release](verification/resource-discovery-release.json) besteht zusätzlich API9, Capture-/Neustart8, MCP22 und UI12. Die [Installation auf localhost8787](verification/resource-discovery-installed.json) erhält sieben Jobs und elf JSON-Dateien byteidentisch. Die historische Stackoverflow-Reproduktion und ihre Reparatur bleiben getrennt gespeichert. Breitere Feature-Abnahmen bleiben PARTIAL.

**B10 wurde lokal installiert; die aktuelle Installation ist B11.** Der [eigene V2-Vertrag](resource-contract-v2.md) ergänzt ausdrückliche interne/externe Follow-Schalter und unveränderliche Rollen bei jedem Redirect. [216 Schalter-, 44 Antwort- und 200 Redirect-/Boundary-Ausführungen](verification/resource-contract-checkpoint.json), API13 und echte UI10 sind geprüft. [Alle vier exakten CI-Jobs](verification/resource-contract-ci.json), [optimierter Release](verification/resource-contract-release.json) und [Installation auf localhost8787](verification/resource-contract-installed.json) bestehen. Sieben vorhandene Jobs und elf JSON-Dateien bleiben byteidentisch. Weitere Encoding-/Unterbrechungs-/Budget-/Browser-/Master-Schalter-/TLS-Kontrollen bleiben offen. Der [historische Designplan](parity/resource-matrix-plan.md) bleibt unverändert; tatsächliche Beweise sind getrennt.

**Abgeschlossen und installiert: B11.** [Issue18](https://github.com/lia-xim/open-seo-spider/issues/18) verfolgt den eigenen Vergleich gespeicherter Crawls mit konkreten Vorher/Nachher-Werten, vollständigem optionalem Originalbericht-JSON und explizitem URL-Mapping. [Alle vier exakten CI-Jobs an Quellcommit2a81036](verification/saved-comparison-ci.json) sind grün; Windows und Linux bestehen jeweils476 Standard-Rust-Tests. Der [optimierte Windows-Release](verification/saved-comparison-release.json) besteht mit demselben Executable native11/SDK12, vier CLI-Grenzgruppen und13 lokale Browsergruppen. Die [Installation auf localhost8787](verification/saved-comparison-installed.json) erhält sieben Jobs und19 dauerhafte Dateien byteidentisch, fünf Git-Blob-Assets und23 read-only Inventar-Tools. Fehlende Quellen bleiben unbekannt; Hersteller-Parität bleibt NOT_PROVEN.

**In Arbeit: B12 – Sitemaps.** Der [konkrete Plan](parity/sitemap-b12-plan.md) erfasst namespace-/elternbewusstes XML-Parsing, begrenzte Gzip-Dateidekodierung und persistierte Sitemap-Mitgliedschaft. Ein separater Work-Entwurf besteht53 neue Parser-/Decoder-Tests plus zwei tatsächliche vorhandene URL-Tests; Main-Integration und native Discovery-/Speicher-Abnahmen stehen aus. Duplikate, fehlende/ungültige Locs und vollständige Quellen müssen nachvollziehbar erhalten bleiben. Positive XML-Mitgliedschaft bleibt von tatsächlich abgerufenen, ausgeschlossenen und blockierten Zielen getrennt. Ein leerer Discovery-Frontier beweist keine vollständige Sitemap-Abdeckung. Beide Feature-IDs bleiben teilweise geprüft.

## Was bereits als Grundlage dient

Native Crawls, geordnete Listenimporte, SQLite-Pause/Recovery, ausgewähltes erneutes Crawlen, eigene URL-Regeln, Chromium, Extraktion und lokale CLI/MCP sind geprüft. B8 ergänzt opt-in gespeicherte Roh-/DOM-Texte und ausgewählte Offline-Inhaltsanalyse: [native API/CLI](verification/content-capture-e2e.json), [tatsächliche Content-UI](../web/verification/ui-content-live-results.json). Exakte [B8-Commit-CI](verification/content-capture-ci.json) und [unveränderlicher installierter Release](verification/content-capture-installed.json) sind ebenfalls geprüft; frühere B7-Beweise attestieren diesen späteren Code nicht.

Diese Grundlagen ersetzen keine vollständige Abnahme aller Filter, Source-Profile, Exporte, Anbieter und Grenzfälle. Die folgende Reihenfolge priorisiert praktische Lücken und ihre Abhängigkeiten; jeder neue Baustein bekommt eigene positive Fälle, Kontrollfälle, unbekannte Daten und gespeicherte Beweise.

## Ausbau in konkreten Slices

### 1. Entdeckungsquellen und Ressourcenschalter vervollständigen

Native Restarbeiten: Unabhängige store/fetch/expand-Schalter und dauerhafte Grants sind integriert. B10 prüft die vollständige eigene Matrix für sieben Ressourcentypen und zehn Navigationsquellen in beiden Profilen. Vererbtes CSS-Environment-Encoding, zusätzliche Redirect-/Unterbrechungsfälle und Browser-Policy bleiben offen.

Referenzgrenze: Anchor-, IMG-, Script-, Stylesheet- und Relationship-Zahlen sind unterschiedliche Messkonzepte. Kein Graph-Link darf nur für übereinstimmende CSV-Zahlen umklassifiziert werden.

- [x] Begrenzte native HTML-/CSS-Quellfamilien, Bild-Schaltermatrix, unveränderliche Bild/Anchor-Grants, finaler Stylesheet-Bezug, private Transporte und gepinnte API/CLI-Exporte gegen eigene Fixtures prüfen.
- [x] Eigene `resource-products-every-type-and-navigation`-Matrix: sieben Ressourcenrollen mal vier store/fetch-Kombinationen und zehn Navigationsquellen mal acht store/fetch/expand-Produkte, beide Profile und genaue HTTP-/MIME-/Grant-Kontrollen.
- [x] Eigene direkte Beziehungsquellen-Schaltermatrix und V2-Linkdirektiven: nofollow/ugc/sponsored sowie Meta-/X-Robots-Dokument-Nofollow einzeln erlauben/sperren und Vorkommen erhalten. Globale crawl_links:false-Direktivenkontrollen sind zusätzliche offene Fälle.
- [ ] `resource-role-unions-reverse-and-concurrent`: umgekehrte/parallele Rollen anderer Typen; `resource-cross-origin-redirect-hop-credentials`: jede Redirect-Stufe und Credential-/Robots-Grenze prüfen.
- [ ] `css-encoding-environment-fallback`, `css-native-grammar-context-and-decoding`, `resource-interruption-each-stage` und `resource-budget-exact-plus-one-all-limits` mit positiven und unbekannten Kontrollfällen ergänzen.
- [ ] Responsive Bilder: deklarierte Kandidaten von tatsächlich browsergewähltem currentSrc und gerenderten Maßen trennen; PDF unsupported fonts/Form XObjects/OCR nicht als vollständiger Text.

Zugeordnete Katalog-IDs: `crawl.resources`, `crawl.relationships`, `crawl.cdns`, `crawl.nofollow`, `robots.traversal`, `images.sources`, `images.alt`, `images.size`, `images.cookies`, `images.pdf`, `images.pdf_links`.

Abhängigkeiten: `B8-content-capture`.

Primärquellen: [configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/), [how-to-find-missing-image-alt-text](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-find-missing-image-alt-text/), [how-to-perform-a-cookie-audit](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-perform-a-cookie-audit/), [how-to-audit-pdfs](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-pdfs/).

### 2. Gespeicherte Inhaltsanalyse vervollständigen

Native Restarbeiten: Native Roh-/DOM-Textbindung und ausgewählte Analyse sind inzwischen geprüft. Weitere Arbeit betrifft Offline-Neuauswahl, große Corpora, vollständige Gruppierung und Spezialberichte.

Referenzgrenze: 22 eigene Original-Fixtures liefern Wort-/Satz-/Readability-Beobachtungen; near checking war ausgeschaltet. Jaccard und EN/DE-Silbenheuristik sind eigene Profile.

- [ ] Volles Roh-HTML/DOM optional unter eigenen Artefakt-/Retention-Budgets speichern; geänderte CSS-Auswahl auf diesen Artefakten ohne Website-Anfrage neu berechnen und Quell-/Config-Revision erhalten.
- [ ] Große gespeicherte Corpora über wiederaufnehmbare Index-/Analyseaufgaben bearbeiten. Fehlende/unvollständige Quellen, Pair-/Output-Budgets und tatsächlich verarbeitete Teilmengen sichtbar machen; keine stillen ersten128 Seiten als gesamte Website.
- [ ] Persistierte exakte Textgruppen mit identisch/ein-Zeichen-verändert/leer/incomplete und wechselndem Source-Profil prüfen; vollständige Gruppenmitglieder und1/2/3-Gramm-Berichte exportieren.
- [ ] EN/DE-Heuristik sowie zusätzliche Sprach-/Silben-/Satzprofile mit benannten positiven und negativen Beispielen prüfen. Original22 Content-Beobachtungen bleiben Kalibrierungshinweise, keine Algorithmus-Parität.

Zugeordnete Katalog-IDs: `metadata.area`, `metadata.wordcount`, `metadata.readability`, `metadata.exact`, `metadata.near`, `metadata.ngrams`.

Abhängigkeiten: keine zusätzliche Slice-Abhängigkeit.

Primärquellen: [how-to-check-for-duplicate-content](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-check-for-duplicate-content/), [how-to-test-readability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-test-readability/), [how-to-use-n-grams](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-n-grams/).

### 3. Request-, Robots-, Scope- und Budget-Verträge abrunden

Native Restarbeiten: Eigene native Policy ist integriert; All-subdomain-HTTP, Rate-Beweise, Retry/Backoff, Proxy/TLS und zusätzliche Limits bleiben offen.

Referenzgrenze: Initiale Antwort, finale Dokumentantwort, nicht gefolgtes Ziel und Intermediate-Row-Entdeckung benötigen explizite Tabellenprofile statt Datenverlust.

- [ ] Lokale DNS/HTTP(S)-Origins prüfen exact host/all-subdomains/folder, Schema/Port, Unicode/IDNA, Prozentkodierung, wiederholte Queries und Fragmentidentität.
- [ ] Robots-Agentengruppen, Allow/Disallow-Gleichstand, Dollar-Endanker, kodierte Pfade, Cache-Origin, Redirect, 404/503 und expliziten Override mit negativen Requestkontrollen prüfen.
- [ ] Verspätete Antworten und monotones Requestlog prüfen per-origin Rate/Concurrency; 429/5xx, Retry-After, begrenzte Backoff-Versuche und Pause ohne neue Admission.
- [ ] Lokales Proxy-/TLS-Testnetz prüft Proxywahl, vertrauenswürdige Zertifikate/HSTS und unveränderte Credential-Grants; Geheimnisse aus exportierter Konfiguration entfernen.
- [ ] Redirect-Profile speichern initial/final getrennt; Loop/Limit/invalid/missing Location und unobserved final body erhalten; Transferbytes und doppelte Header korrekt benennen.

Zugeordnete Katalog-IDs: `modes.spider`, `modes.list`, `modes.serp`, `modes.control`, `modes.recrawl`, `crawl.unlimited`, `crawl.scope`, `crawl.normalization`, `crawl.rewrite`, `crawl.include_exclude`, `crawl.limits`, `crawl.speed`, `crawl.timeouts_retries`, `crawl.headers_proxy`, `crawl.tls`, `robots.fetch`, `robots.matching`, `robots.test`, `robots.override`, `robots.meta`, `robots.indexability`, `http.response`, `http.failures`, `http.redirects`, `http.refresh`, `http.migration`, `http.broken_sources`.

Abhängigkeiten: keine zusätzliche Slice-Abhängigkeit.

Primärquellen: [configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/), [how-to-use-list-mode](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-list-mode/), [general](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/), [seo-spider](https://www.screamingfrog.co.uk/seo-spider/), [tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/), [robots-txt-tester](https://www.screamingfrog.co.uk/seo-spider/tutorials/robots-txt-tester/), [audit-redirects](https://www.screamingfrog.co.uk/seo-spider/tutorials/audit-redirects/), [internal-linking-audit-with-the-seo-spider](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/).

### 4. Vollständige Sitemap-Mitgliedschaft und Beziehungsprüfungen

Native Restarbeiten: Sitemap-Entdeckung besteht, jedoch vollständige Mitgliedschaft/Felder, XML-Hreflang, Gzip, Validierung und Generator-Splitting sind nicht vollständig geprüft.

Referenzgrenze: Sitemap-only, nicht gelistet, nicht indexierbar, Redirect und nicht beobachtetes Ziel sind getrennte Prädikate; HTML-/HTTP-/XML-Konflikte gezielt vergleichen.

- [ ] Owned sitemap index mit gzip-child, cycle, malformed XML, übergroßer Inflation, duplicate loc, relative/invalid loc und lastmod-Grenzen; jede XML-Occurrence und Herkunft erhalten.
- [ ] Sitemap membership gegen vollständig erklärte Crawl-Kohorte abgleichen; excluded/blocked/unfetched Ziele nicht als fehlende Webseite bezeichnen.
- [ ] HTML-/HTTP-/XML-Hreflang Cluster, duplicate-identical/conflict, canonical chain/cycle, pagination long-chain sowie AMP/mobile return target state prüfen.
- [ ] Unabhängiger XML-Parser validiert generierte lastmod/image/hreflang-Ausgabe, Byte-/URL-Splits, Index und exakte URL-Multimenge.

Zugeordnete Katalog-IDs: `relationships.canonical`, `relationships.canonical_checks`, `relationships.pagination`, `relationships.hreflang`, `relationships.hreflang_checks`, `relationships.amp`, `relationships.mobile`, `sitemap.discovery`, `sitemap.coverage`, `sitemap.validity`, `sitemap.generate`, `sitemap.split`.

Abhängigkeiten: `resource-discovery`, `transport-governance`.

Primärquellen: [how-to-audit-canonicals](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-canonicals/), [how-to-audit-pagination](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-pagination/), [how-to-audit-hreflang](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-hreflang/), [how-to-audit-validate-amp](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-validate-amp/), [how-to-audit-mobile-usability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-mobile-usability/), [how-to-audit-xml-sitemaps](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-xml-sitemaps/), [xml-sitemap-generator](https://www.screamingfrog.co.uk/seo-spider/tutorials/xml-sitemap-generator/).

### 5. Konkrete Filtermitgliedschaft statt All-Tab-Näherung

Native Restarbeiten: 144 Fähigkeiten ersetzen keine 416 Filterabnahmen. Metadaten-Grenzen, Indexability-Profile, genaue Source-Reports und unbekannte Messungen einzeln spezifizieren.

Referenzgrenze: 78 Original-Named-Filter-Beobachtungen besitzen 64 positive/control pairs, vier refined controls und zehn partielle Beobachtungen. Der native 1:1-Vergleich dieser Mitglieder ist noch NOT_PROVEN.

- [ ] Retained CSVs zuerst gegen tatsächliche native Issue-/Feld-Prädikate auf denselben authored HTML-Cases normalisieren; exakte path sets, zusätzliche/fehlende Mitglieder pro Filter ausgeben.
- [ ] Jeder Schwellenfilter besitzt threshold-1/threshold/threshold+1 und missing/empty/whitespace/multiple-Fälle; Indexability-Gating pro Dokument-/Ressourcenprofil explizit.
- [ ] Initial/final Displayprofile, anchor-only Outlinks, sämtliche resource refs und Zwischenredirect-Discovery getrennt vergleichen; keine Messkonzepte herunterrechnen.
- [ ] 24 eigene H1/H2-Referenzfälle und 58 vorhandene native Pixelwerte als begrenzte Goldens erhalten; andere Fonts/DPI/Sprachen oder vollständige Predicate-Parität nicht ableiten.
- [ ] Spelling/Grammar erst mit eigenem versioniertem Wörterbuch-/Rule-Profil und exakten Textpositionen testen; fehlende Original-Lizenzfunktionen explizit blocked statt empty=PASS.

Zugeordnete Katalog-IDs: `metadata.titles`, `metadata.descriptions`, `metadata.headings`, `metadata.serp_preview`, `metadata.spelling`.

Abhängigkeiten: `B8-content-capture`, `transport-governance`.

Primärquellen: [tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/), [general](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/), [spelling-grammar-checker](https://www.screamingfrog.co.uk/seo-spider/tutorials/spelling-grammar-checker/).

### 6. Quellartefakte, portable Projekte und vollständige Vergleiche

Native Restarbeiten: JSON-/SQLite-Persistenz und eigene Historie sind vorhanden. Volles HTML/DOM, Artefaktmanifest, portable Roundtrips und vollständige Comparison-Felder fehlen.

Referenzgrenze: Herstellereigene Projektformate werden nicht als kompatibel behauptet. Offline-Auswahl und Delta-Semantik können über die unabhängigen eigenen Formate geprüft werden.

- [ ] Opt-in HTML/DOM/resource artifacts mit Representation-Hash, Bytebudget, Version, Source-Revision und Referenzen speichern; unabhängige Auswahl nach CSS-Änderung ohne HTTP wiederholen.
- [ ] Archive in zweitem Verzeichnis öffnen; fehlende/hash-veränderte Artefakte, relative Pfade, schema upgrades, locked snapshots und retention mit Testuhr prüfen.
- [ ] RAM/SQLite gleiche Fixture-Beobachtungen; Compare new/removed/changed, same issue count but changed facts, Graph/Schema-Deltas, URL-map collisions und vollständigen Export prüfen.
- [ ] Konfigurationsimport erhält erlaubte Profile, validiert Pfade und behandelt Credentials lokal getrennt von portablen Exporten.

Zugeordnete Katalog-IDs: `modes.compare`, `rendering.source_diff`, `rendering.archive`, `storage.database`, `storage.memory`, `storage.snapshots`, `storage.projects`, `storage.retention`, `storage.config`, `storage.portable`, `storage.compare_changes`, `storage.compare_mapping`, `storage.compare_export`, `automation.assets`.

Abhängigkeiten: `B8-content-capture`.

Primärquellen: [how-to-compare-crawls](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-compare-crawls/), [crawl-javascript-seo](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/), [configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/), [general](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/), [seo-spider-working-in-teams](https://www.screamingfrog.co.uk/seo-spider/tutorials/seo-spider-working-in-teams/).

### 7. Browser-Policy, zusätzliche DOM-Quellen und Interaktionen

Native Restarbeiten: Echter Chromium-Kern, custom JavaScript und axe sind geprüft. Live URL-Policy+renderer ist bewusst rejected; Subresource-Robots, Aktionen, iframe/shadow Quellen und Artefakt-Diffs bleiben offen.

Referenzgrenze: Ordentlicher Original-Free-CLI-Referenzvergleich für lizenzgebundenes Rendering ist nicht verfügbar. Native Browser-Akzeptanz bleibt unabhängig beweisbar.

- [ ] Worker-Browserkontext prüft fetch/XHR/navigation/css/image/iframe über zwei Origins mit Scope/Robots/Secrets und expliziter Resource-Reason-Evidenz.
- [ ] Same-origin iframe, cross-origin frame denial, open/closed shadow roots, templates und computed visibility als getrennte Quellprofile prüfen; fremde DOMs nicht zusammenraten.
- [ ] Autorisierte click/hover/scroll-Aktionsliste mit Reihenfolge, Timeout, stale elements, fail/pause und nachträglichen Links prüfen.
- [ ] Raw-/DOM-full-artifact-Diffs, viewport/currentSrc/screenshot-Provenienz, Console/Network dropping und axe needs-review durch UI/Export erhalten.
- [ ] Eigene visuelle Selector-/Snippet-Bibliothek erzeugt reproduzierbare bounded Regeln ohne Credentials; Browser-UI-Selektion gegen zweite Fixture prüfen.

Zugeordnete Katalog-IDs: `rendering.text`, `rendering.chromium`, `rendering.wait`, `rendering.screenshot`, `rendering.console`, `rendering.flatten`, `rendering.custom_extract`, `rendering.visual_extract`, `rendering.custom_search`, `rendering.javascript`, `rendering.interact`, `rendering.snippet_library`.

Abhängigkeiten: `resource-discovery`, `transport-governance`, `source-artifacts`.

Primärquellen: [crawl-javascript-seo](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/), [web-scraping](https://www.screamingfrog.co.uk/seo-spider/tutorials/web-scraping/), [how-to-use-custom-search](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-custom-search/), [how-to-debug-custom-javascript-snippets](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-debug-custom-javascript-snippets/), [simulating-user-interactions-with-custom-javascript](https://www.screamingfrog.co.uk/seo-spider/tutorials/simulating-user-interactions-with-custom-javascript/).

### 8. Versionierte Schema-, HTML-, AMP- und Sicherheitsregeln

Native Restarbeiten: JSON-LD-Fakten und Sicherheitsheader sind vorhandene Teilbeobachtungen. Microdata/RDFa, schema/rich-results, HTML/head und komplette Sicherheits-/Mobile-Regeln fehlen.

Referenzgrenze: Axe-Ergebnis, Schema.org-Gültigkeit und Google-Rich-Result-Berechtigung sind verschiedene Verträge; verwendete öffentliche Rule-Version jeweils binden.

- [ ] JSON-LD @graph/@id, Microdata nested itemref und RDFa mehrere scopes erzeugen denselben kontrollierten Fact-Graph mit source node provenance und parse errors.
- [ ] Versionierte erlaubte/deprecated/unknown Types und erforderliche/empfohlene Properties; kein positives Eligibility-Urteil bei unbekannter Rule-Version oder unvollständigem Capture.
- [ ] Offizieller versionierter AMP-/HTML-Validator mit gültiger Control, minimal broken markup, line/column/rule sowie unavailable states.
- [ ] HTTPS-/HTTP-Formular, MIME mismatch, CSP/HSTS/Header-Varianten und Mobile target-site tap/content fixtures; Carbon-Ausgabe nur mit benannter Schätzmethodik.

Zugeordnete Katalog-IDs: `quality.structured_formats`, `quality.schema`, `quality.richresults`, `quality.accessibility`, `quality.accessibility_limit`, `quality.mobile`, `quality.html_head`, `quality.html_validation`, `quality.security`, `quality.carbon`.

Abhängigkeiten: `source-artifacts`, `browser-depth`.

Primärquellen: [structured-data-testing-validation](https://www.screamingfrog.co.uk/seo-spider/tutorials/structured-data-testing-validation/), [how-to-perform-a-web-accessibility-audit](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-perform-a-web-accessibility-audit/), [how-to-audit-mobile-usability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-mobile-usability/), [how-to-debug-invalid-html-elements-in-the-head](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-debug-invalid-html-elements-in-the-head/), [configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/), [tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/).

### 9. Segmente, Graphen, vollständige Tabellen und Berichte

Native Restarbeiten: Eigene Tabellen/Graphen und CSV existieren; logische Segmente, Directory/3D, gespeicherte Preferences, komplette Specialist-Reports/XLSX und Combined Filter fehlen.

Referenzgrenze: Exact positive/control membership muss Drill-down, counts, export und alle source occurrences durchgehend belegen; vorhandene UI-Screenshot-Anzahl ist keine Algorithmus-Parität.

- [ ] Ordered segments mit overlap/default/unknown Fields filtern Issues, Charts, Pages und CSV/XLSX über identische exakte Ordinalmengen.
- [ ] Shortest-path graph, directory virtual paths, bounded 3D/focus und klar ausgewiesene dropped nodes; PageRank version/parameter hash persistent.
- [ ] Specialist canonical/hreflang/redirect/sitemap/content/security reports samt source location, full occurrence and snapshot provenance.
- [ ] Numeric+text combined filter, persistent columns, sort/pagination, export formula/newline/Unicode, mobile bottom reachability und localization without data rewrite.

Zugeordnete Katalog-IDs: `links.occurrences`, `links.positions`, `links.depth`, `links.score`, `links.uncrawlable`, `links.fragments`, `links.crawl_graph`, `links.directory_graph`, `links.graphs_3d`, `links.segments`, `automation.tables`, `automation.reports`, `ui.realtime`, `ui.filter`, `ui.overview`, `ui.details`, `ui.charts`, `ui.localization`.

Abhängigkeiten: `filter-calibration`, `sitemap-relationships`, `source-artifacts`.

Primärquellen: [internal-linking-audit-with-the-seo-spider](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/), [seo-spider-24](https://www.screamingfrog.co.uk/blog/seo-spider-24/), [general](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/), [site-architecture-crawl-visualisations](https://www.screamingfrog.co.uk/seo-spider/tutorials/site-architecture-crawl-visualisations/), [configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/), [tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/).

### 10. Autorisierte Provider-APIs und URL-Provenienz

Native Restarbeiten: GA4/GSC/Inspection/PSI/CrUX/Backlinks und OAuth fehlen. Diese Fähigkeiten können mit eigenen Mock-Providern entwickelt und erst separat mit autorisierten Konten belegt werden.

Referenzgrenze: Fehlende Provider-Zeilen sind weder Nullwerte noch fehlende Nachfrage. Website requests0 im API-only Modus separat protokollieren.

- [ ] Provider mock OAuth add/revoke/refresh/expired/scope/429; Tokens bleiben außerhalb JSON/CSV/Logs/Repo.
- [ ] Datums-/Property-/URL-match Profile, paged row coverage, duplicates, missing versus zero, API errors und provider-only URLs mit provenance/scope vergleichen.
- [ ] PSI mobile/desktop und lab/field mit Einheiten/Zeitraum; CWV nur aus vorhandenen Feldmetriken, Inspection quota queue and observed timestamp.
- [ ] Source-cohort orphan reconciliation mit Sitemap und kontrollierter GA/GSC-URL-Liste; nicht beobachtete Webseiten bleiben unknown.

Zugeordnete Katalog-IDs: `modes.apis`, `links.orphans`, `integrations.ga4`, `integrations.gsc`, `integrations.inspection`, `integrations.psi`, `integrations.cwv`, `integrations.majestic`, `integrations.ahrefs`, `integrations.moz`, `integrations.oauth`, `integrations.provider_urls`.

Abhängigkeiten: `sitemap-relationships`, `transport-governance`.

Primärquellen: [configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/), [find-orphan-pages](https://www.screamingfrog.co.uk/seo-spider/tutorials/find-orphan-pages/), [how-to-automate-the-url-inspection-api](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-automate-the-url-inspection-api/), [how-to-audit-core-web-vitals](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-core-web-vitals/).

### 11. Optionale lokale KI und reproduzierbare semantische Daten

Native Restarbeiten: Alle neun AI-Katalogfähigkeiten sind geplant. Lokaler Mock/Ollama-Kern ist zuerst testbar; kostenpflichtige Provider bleiben gesondert autorisiert.

Referenzgrenze: Embedding-/Prompt-Ausgaben benötigen eigene Modell-/Input-Bindungen; Hersteller-Provider-Tokens und proprietäre Prompt-Bibliotheken werden nicht übernommen.

- [ ] Eigene Prompts mit sichtbarer exakter selected text/HTML input, unsupported/missing states, typed output und versioned input/model hashes.
- [ ] Deterministische Vektoren prüfen cosine, similarity/outlier/semantic search, dimensions/model mismatches, cache/invalidation und redirect candidate ranking.
- [ ] Bounded rate/spend/retry, local versus paid usage, token failures, user-owned library import/export and no automatic redirect mutation.

Zugeordnete Katalog-IDs: `ai.providers`, `ai.prompts`, `ai.prompt_library`, `ai.embeddings`, `ai.similarity`, `ai.outliers`, `ai.semantic_search`, `ai.redirect_mapping`, `ai.usage`.

Abhängigkeiten: `B8-content-capture`, `provider-joins`.

Primärquellen: [how-to-crawl-with-ai-prompts](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-crawl-with-ai-prompts/), [how-to-identify-semantically-similar-pages-outliers](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-identify-semantically-similar-pages-outliers/), [how-to-use-vector-embeddings-for-redirect-mapping](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-vector-embeddings-for-redirect-mapping/).

### 12. Lokale Planung, MCP, Berichte und Plattformbetrieb

Native Restarbeiten: CLI und MCP sind geprüft. Scheduler/auto-compare/notifications/cloud export/usage aggregates sowie umfassende native/WSL/portable Installakzeptanz bleiben offen.

Referenzgrenze: Ein grüner Linux-Docker-CI-Lauf ist kein Nachweis der lokalen Windows-Docker-Installation. Nachrichten/Providerziele benötigen tatsächlich konfigurierte Ziele.

- [ ] Fake-clock scheduler prüft timezone/DST/overlap/cancel/restart, saved configurations, successful/failed runs and exact automatic comparison basis.
- [ ] Lokale SMTP-Testsenke und mock export destination prüfen retries, attachments, append/replace sowie no automatic unsolicited delivery.
- [ ] Official MCP SDK stdio/HTTP sweeps across owned commands, errors, content/policy pagination and exact declared tool schema; no website fetch in offline tools.
- [ ] Windows/Linux native portable and unprivileged Docker persistent volume; optional local WSL/Docker install separately proven, preserving current user snapshots.

Zugeordnete Katalog-IDs: `automation.looker`, `automation.cli`, `automation.scheduler`, `automation.auto_compare`, `automation.notifications`, `automation.mcp`, `automation.mcp_transport`, `automation.stats`.

Abhängigkeiten: `source-artifacts`, `analyst-workflows`.

Primärquellen: [how-to-automate-crawl-reports-in-data-studio](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-automate-crawl-reports-in-data-studio/), [general](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/), [seo-spider-24](https://www.screamingfrog.co.uk/blog/seo-spider-24/), [how-to-set-up-crawl-email-notifications-export-attachments](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-set-up-crawl-email-notifications-export-attachments/), [configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/).

## Referenzmessungen als eigene Arbeitsspur

1. Die retained Original-CSVs zuerst gegen tatsächliche native Filter-Sets auf denselben selbst erstellten Fällen vergleichen: zusätzliche/fehlende Mitglieder und bekannte Feldprofile je Filter ausgeben.
2. Jede relevante Schwelle mit darunter/genau/darüber sowie fehlend/leer/Whitespace/mehrfach prüfen. Indexierbarkeit und kombinierte Fehler müssen als explizite Prädikate erhalten bleiben.
3. Initiale Antwort, finales Dokument, Zwischenredirect, Anchor-Outlinks und alle Ressourcenbezüge getrennt halten. PNG-/PDF-/HTML-Indexierbarkeit verlangt jeweils ein benanntes Profil.
4. Neue Original-Free-CLI-Läufe nur für konkrete noch offene Fälle auf eigenen lokalen Fixtures ausführen. Lizenzabhängige Funktionen bleiben unter ihren tatsächlichen Grenzen; ein leerer Filter ist kein PASS.
5. Eigene Abnahme, begrenzten Originalvergleich und vollständige Herstellerparität getrennt dokumentieren. Status und TODO erst nach passenden endgültigen Gates aktualisieren.

Bisherige Grenzen: 887 passende Vergleichszellen, 58 Pixelmessungen im festen Schrift-/DPI-Profil und48 H1/H2-Mitgliedschaften aus24 Fällen sind begrenzte Beobachtungen. Sie zählen weder vollständige Fähigkeiten noch alle416 Filter. Die78 Named-Filter-Proben benötigen weiterhin ihren tatsächlichen unabhängigen Vergleich.

## Wie die Liste fortgeführt wird

Für jeden begonnenen Slice die aktuelle Restabnahme aus dem Katalog übernehmen, konkrete Fixtures ausführen und Ergebnis/Profil/Artefakt speichern. Erst eine vollständig erfüllte eigene Abnahme bekommt einen Feature-Haken. Danach `node scripts/update-feature-todo.mjs` ausführen und mit `--check` prüfen. Historische Beweise bleiben erhalten.

Diese Roadmap ist ein Arbeitsplan. Noch offene Checkboxen sind keine fehlgeschlagenen Tests; bereits vorhandene Kernbeweise stehen im Katalog. Provider, kostenpflichtige APIs und externe Benachrichtigungsziele brauchen ihre jeweils konkret autorisierten Zugänge. Native/WSL/Docker-Plattformbeweise bleiben separat.
