# Open SEO Spider – Feature-To-do-Liste

> Automatisch aus [feature-parity.json](feature-parity.json) erzeugt. Aktualisieren: `node scripts/update-feature-todo.mjs`. Unveränderten Stand prüfen: `node scripts/update-feature-todo.mjs --check`.

Die Liste hält alle 144 gruppierten Fähigkeiten fest. Ein Haken bedeutet, dass das **eigene gespeicherte Abnahmekriterium** mit Beweisen erfüllt ist. Er bedeutet keine vollständige Gleichheit mit Screaming Frog. Diese Referenzparität bleibt für alle Einträge **NOT_PROVEN**.

Aktueller Katalogstand: **31 eigene Abnahmen erfüllt**, **67 teilweise geprüft**, **46 offen**. Die Zahlen sind keine Aussage über den Anteil aller Herstellerfunktionen.

Die [416 benannten Filter und 32 Ablaufverträge](parity/granular-acceptance.json) bilden ein zusätzliches Register. Ihre positiven Fälle, Kontrollfälle, Parameter und Grenzfälle müssen jeweils geprüft werden. Ein implementierter Kern, ein leerer Export oder übereinstimmende CSV-Zellen ersetzen diese Einzelabnahme nicht.

## Aktuelle Arbeit und nächste Schritte

- [ ] **Aktuelle Arbeit – B10: vollständige Ressourcenmatrix und gerichtete Sonderfälle** — IN_PROGRESS. Der ausdrücklich wählbare V2-Vertrag ist an Commit 8fab997 mit vier grünen CI-Jobs geprüft und lokal auf Port 8787 installiert. Eigene native Matrix: 216 Schalter-, 44 Antwort- und 200 Redirect-/Boundary-Ausführungen; API/CLI/MCP13 und echte UI10. Der optimierte Release besteht separat API13, Legacy-Ressourcen9, Capture-/Neustart8, beide SDK22-Transporte und UI10. Sieben vorhandene Jobs und elf JSON-Dateien bleiben byteidentisch. Weitere Encoding-/Browser-/Budget-/Master-Schalter-/TLS-Abnahmen bleiben offen; vollständige Hersteller-Parität ist NOT_PROVEN. GitHub: [B10-Issue #17](https://github.com/lia-xim/open-seo-spider/issues/17).
  - [x] Ausdrückliche V2-Auswahl mit getrennten internen/externen nofollow-/ugc-/sponsored-/Dokument-Schaltern, unveränderlichen Rollen und Scope-Witnesses implementiert; 22 fokussierte Contract-Tests und historische B9-JSON-/Hash-Pins bestehen.
  - [x] Exakte 108 Schalterfälle in V1 und V2: 216 Ausführungen bestehen, darunter 40 ungültige Konfigurationen vor jedem HTTP-Request. Beobachtung, Reportaufnahme, Fetch, Expansion, MIME und Grants sind getrennt geprüft.
  - [x] 21 eingefrorene MIME-/Antwortkontrollen in beiden Profilen plus PDF-Annotationsvarianten: 44 tatsächliche Ausführungen bestehen. Ursprüngliche Transport-Permissions und spätere unabhängige Report-Witnesses bleiben getrennt.
  - [x] 13 echte API-/CLI-/MCP-Gruppen mit 19 eigenen Jobs prüfen Profile, Rollen, Exporte, manipulierte SQLite-Zellen, ausgewählten Recrawl und Kaltstart. Read-only-Prüfungen verursachen null Website-Anfragen; ein fremder Health-Server wird vor allen HTTP-Aufrufen abgelehnt.
  - [x] 387 Standard-Rust-Tests, Formatprüfung und Workspace-Alltargets-Clippy bestehen am lokalen Debug a5881b7c; vier explizite Chromium-Live-Gates bleiben getrennt.
  - [x] Zehn echte Browser-zu-Rust-Prüfungen bestehen am isolierten Debug: erfolgreiche V1/V2-Jobs, keine automatische Umstellung, exakte gesperrte CSV-/JSON-Werte, unveränderliche Rollen, drei Origins, Read-only-Exporte und 320px-Breite. Elf gemockte UI-Fälle inklusive verzögerter Hintergrund-Lesezugriffe und Mutation-/Netzwerk-Negativkontrollen bleiben separate Belege.
  - [x] Eigenes Kriterium crawl.nofollow erfüllt: nofollow/ugc/sponsored und Meta-/X-Robots-Dokument-Nofollow sind mit Follow aus/an, tatsächlichem GET0/1 und erhaltenen Quellvorkommen geprüft. Dieses Häkchen gilt für die ausdrücklich gewählten V2-Follow-Schalter; globale crawl_links:false-Kontrollen bleiben zusätzlich offen.
  - [x] Alle 88 eingefrorenen Redirect-IDs in beiden Profilen plus zwölf zusätzliche Boundary-IDs: 200 tatsächliche Ausführungen bestehen. Originale Rollen, jedes Location-/Status-/Scope-/Policy-/Robots-Faktum, Credential-Herkunft und erlaubte/verbotene Zielrequests sind separat geprüft; der zusätzliche Rust-Test und Core-Clippy bestehen.
  - [x] Exakte 8fab997-Source-CI: Windows, Linux, Chromium und Container erfolgreich. Separater unveränderlicher optimierter Windows-Release mit API13/Legacy9/Capture8/SDK22/UI10 geprüft und lokal installiert; sieben Nutzer-Jobs und elf JSON-Dateien byteidentisch erhalten.
  - [ ] CSS-Encoding-Herkunft, unabhängige parallele Rollen, alle ausgewählten Ressourcentypen, Abbruchphasen und jede Budgetgrenze exakt/plus-eins mit zusätzlichen nativen Fixtures ergänzen.
  - [ ] Zusätzliche Vier-Direktiven-Kontrollen unter globalem crawl_links:false als eigenen Master-Schalter-Vertrag prüfen; die belegten V2-Follow-Schalter nicht damit gleichsetzen.
  - [ ] Einen vertrauenswürdigen eigenen HTTPS-Positivfall ergänzen; die belegte Scheme-Mismatch-Sperre gegenüber einem HTTP-Ziel ersetzt keine tatsächliche TLS-Verbindung.
  - Verträge: [resource-matrix-plan.md](parity/resource-matrix-plan.md), [resource-matrix-plan.json](parity/resource-matrix-plan.json), [resource-discovery.md](resource-discovery.md), [resource-contract-v2.md](resource-contract-v2.md), [ROADMAP.md](ROADMAP.md).
  - Gate-Stand: contract_regressions: PASS_V2_WITH_PRESERVED_V1_COMPATIBILITY; resource_products_28: PASS_BOTH_PROFILES; navigation_products_80: PASS_BOTH_PROFILES; response_controls_21: PASS_44_EXECUTIONS; native_request_logs: PASS_BOUNDED_SLICE; api_cli_mcp: PASS_13_GROUPS; redirect_controls_88: PASS_176_PROFILE_EXECUTIONS; workspace_tests_clippy: PASS; exact_commit_ci: PASS; immutable_release: PASS; installed_runtime: PASS_B10; full_feature_acceptance: PARTIAL; actual_browser_ui: PASS_10_NATIVE_CHECKS; ui_mock: PASS_11_SEPARATE_FIXTURES; additional_redirect_boundaries: PASS_24_PROFILE_EXECUTIONS.

- [x] **B9: Ressourcen und CSS-Abhängigkeiten – geprüfte Grundlage** — COMPLETED. Die begrenzte native B9-Grundlage ist implementiert, an Commit f107ce7 geprüft und lokal installiert. 362 Standard-Rust-Tests, API9, MCP22 und tatsächliche UI65 bestehen; alle vier exakten CI-Jobs sind grün. Der unveränderliche Release besteht zusätzlich API9, Capture-/Neustart8, MCP22 und UI12. Sieben vorhandene Jobs und elf JSON-Dateien bleiben byteidentisch. Die vollständige Ressourcen-/Navigations-/Encoding-Abnahme bleibt PARTIAL und wird als B10 weitergeführt.
  - [x] Finaler reparierter Debug 142c: 362 Standard-Rust-Tests, vier ignorierte Live-Gates; separat echte Chrome-Prüfung, Formatierung/Clippy und 512KiB-Core-Recovery bestanden. Alle sieben bestehenden Regressionstestsuiten sind am reparierten Build geprüft.
  - [x] 15 native Ressourcenfälle und 15 CSS-Tests prüfen Quellrollen, Bild-Speichern/Abrufen in vier Kombinationen, späte unabhängige Grants, MIME/Basis, fehlerhafte getrennte CSS-Einheiten und atomare Budgets.
  - [x] Pause/Recovery und ausgewählter Recrawl erhalten private Transportbeobachtungen, Eltern-/Restdaten und unveränderliche Grants; paged/streamed Leser prüfen vollständige Config, Generation und Datenintegrität in einem SQLite-Snapshot.
  - [x] Neun echte API-/CLI-Gruppen prüfen unter anderem 506 unterschiedliche Ressourcen-URL-Transporte ohne feste 500-URL-Grenze, vollständige Exporte, null Website-Anfragen bei CSS-Inspektion und vier gezielte SQLite-Manipulationskontrollen.
  - [x] Offizieller MCP-SDK: 22 Tools über stdio und Streamable HTTP geprüft. Die tatsächliche Browser-UI besteht 12 Ressourcenprüfungen plus 53 zugehörige bestehende Prüfungen; separate Settings10/Ledger16-Mocks bleiben als solche ausgewiesen.
  - [x] Lokale Neustartreparatur an 142c0a…: exakt fehlgeschlagene Fixture lädt sechs fertige Jobs; frische achtteilige Capture-/Restart-Prüfung, API9 und MCP22 bestehen. Historische Dateien werden nicht umgeschrieben, die globale Stackgrenze nicht erhöht.
  - [x] Exakte B9-Commit-CI: Windows, Linux, Chromium und Docker bestehen. Release bfcbbb… ist auf localhost8787 installiert; API9, Capture-/Neustart8, MCP22 und tatsächliche UI12 am Release bestanden. Sieben vorhandene Jobs und elf JSON-Dateien bleiben byteidentisch.
  - [ ] resource-products-every-type-and-navigation: Die vier Speichern/Abrufen-Kombinationen sind nativ für Bilder geprüft; CSS, JavaScript, Medien, PDF, SWF und jede Navigation benötigen ihre eigene vollständige Matrix.
  - [ ] relationship-discovery-each-positive-negative und link-directive-four-way-traversal: Für Canonical, hreflang, Pagination, AMP, mobile Alternate, iframe/Refresh sowie nofollow/ugc/sponsored/page-nofollow je aktivierte und gesperrte Ziele mit exaktem Requestlog prüfen.
  - [ ] resource-role-unions-reverse-and-concurrent: Anchor/Bild und späte Store-/Navigations-Grants sind geprüft. Umgekehrte parallele Rollen anderer Typen und tatsächliche Media-/Source-Kontexte gezielt ergänzen.
  - [ ] css-encoding-environment-fallback: BOM/HTTP-Charset/exakte führende @charset-Deklaration sind das eigene Profil. Verweisendes HTML und importierende CSS-Umgebungen liefern noch keine vererbte Encoding-Herkunft; mehrdeutige nicht-ASCII-Quellen bleiben unbekannt.
  - [ ] css-native-grammar-context-and-decoding: Die 15 eigenen Parserfälle und native Imports/Zyklen sind geprüft; zusätzliche Fonts, moderne URL-Grammatik, bedingte Regeln und Decoding-Revisionen benötigen gerichtete native Kontrollen. Kandidaten beweisen kein tatsächliches Browserladen.
  - [ ] resource-cross-origin-redirect-hop-credentials: Zwei Origins und direkte Robots-/Credential-Grenzen sind geprüft; mehrgliedrige Redirects je Ressourcentyp mit jedem verbotenen Ziel und unveränderten Credential-Origins ergänzen.
  - [ ] resource-interruption-each-stage und resource-budget-exact-plus-one-all-limits: Atomare Rollbacks, Pause, Wiederaufnahme und gepinnte parallele Leser sind geprüft; Prozessabbruch an jedem Parser-/Grant-/Expansion-Punkt und jede einzelne Grenze exakt/plus-eins weiter prüfen.
  - [ ] resource-selected-all-kind-targets und image-occurrence-alt-and-selected-source: Selektiver Recrawl ohne Expansion ist geprüft; CSS/Bild/PDF/Navigations-Matrix, mehrere Quellseiten sowie fehlendes/leeres/Whitespace-Alt und Browser-currentSrc bleiben zusätzliche Abnahmen.
  - [ ] resource-native-renderer-enforcement: Native Ressourcenschalter plus Rendering werden weiterhin vor HTTP abgelehnt. Browser-Navigation und Subrequests müssen erst unter denselben Regeln tatsächlich kontrolliert und geprüft werden.
  - Verträge: [resource-discovery.md](resource-discovery.md), [ROADMAP.md](ROADMAP.md), [resource-discovery-api.md](resource-discovery-api.md), [resource-discovery-ui.md](resource-discovery-ui.md).
  - Gate-Stand: pure_css_inventory: PASS; resource_policy_bounded: PASS; native_fixture_subset: PASS; native_fixture_groups_10: PARTIAL; api_cli_mcp: PASS; actual_ui: PASS; related_actual_browser_53: PASS; ui_settings_fixture: PASS; ui_ledger_fixture: PASS; exact_commit_ci: PASS; immutable_release: PASS; installed_runtime: PASS; legacy_restart_regression: PASS; full_core_and_ui_on_repaired_source: PASS; legacy_regression_groups_7: PASS.

- [x] **Abgeschlossen – B11: gespeicherte Crawls vergleichen** — COMPLETED. Vergleich gespeicherter Crawls mit konkreten Änderungen, URL-Mapping und optionalem vollständigem Originalbericht-JSON ist als eigener Baustein abgeschlossen und auf Port8787 installiert. Alle vier exakten CI-Jobs sind grün; Windows/Linux bestehen jeweils476 Standard-Rust-Tests. Derselbe optimierte Windows-Release besteht native11/SDK12, CLI-Grenzgruppen4 und lokale Browsergruppen13. Sieben vorhandene Jobs und19 dauerhaft gespeicherte Dateien bleiben unverändert. Hersteller-Parität bleibt NOT_PROVEN.
  - [x] Versionierter exakter URL-Vergleich mit added/removed/changed/unchanged und konkreten Feldwerten in zwei tatsächlichen gespeicherten nativen Crawls geprüft.
  - [x] Titel, Description, H1/H2, Direktiven, Linkvorkommen, JSON-LD und ausgewählte Texte ändern sich nachvollziehbar trotz gleicher Issue-Zahl; fehlende, partielle und nicht verfügbare Quellen bleiben getrennt.
  - [x] API/CLI/MCP liefern identische Deltas und typisierte Report-Hashes; nach Abschaltung der Fixture-Website und App-Kaltstart beobachten ein eigener HTTP-Witness und beide SDK-Transporte null Website-Anfragen.
  - [x] Zwei App/Core-Vertragsfehler rot reproduziert und repariert; 440 Standard-Rust-Tests sowie Alltargets-Clippy bestehen.
  - [x] Vergleichsoberfläche, alle vier Klassen, Details, fehlende/partielle Quellen, genaue JSON-/CSV-Downloads und 320px-Erreichbarkeit an tatsächlichen gespeicherten nativen Crawls geprüft; eingebettete Assets und Mutation-Fence sind gebunden.
  - [x] Vier tatsächliche Crawls auf zwei Origins prüfen Staging/Production-Prefix-Joins und beidseitige Mapping-Kollisionen; alle Originalfakten und Report-/Revisions-/Exportbytes bleiben erhalten, zwei Offline-Witnesses bleiben nach finalem Prozessende bei null.
  - [x] Gemeinsamer optionaler JSON-Export erhält beide vollständigen geladenen typisierten Reports mit Hash-/Bytebindung und allen Delta-Klassen; native SDK-, Browser-, Kaltstart- und CLI-Grenzprüfungen bestehen.
  - [x] Exakter Quellcommit2a81036 mit vier grünen CI-Jobs, optimierter Windows-Release und tatsächliche lokale Browser-Abnahme13; datenbewahrend auf Nutzer-Port8787 installiert, sieben Jobs und19 dauerhafte Dateien byteidentisch.
  - Verträge: [FEATURE_TODO.md](FEATURE_TODO.md), [ROADMAP.md](ROADMAP.md), [saved-comparison.md](saved-comparison.md).
  - Gate-Stand: core_contract: PASS_34; api_cli_mcp: PASS_NATIVE_9_AND_SDK23; actual_browser_ui: PASS_NATIVE_8_WITH_SERVED_ASSETS_AND_MUTATION_CONTROL; snapshot_and_mapping_integrity: PASS_NATIVE_10_TWO_ORIGINS_BOTH_COLLISION_SIDES; offline_restart_zero_requests: PASS_FINAL_DRAINED_HTTP_WITNESSES_0; work_draft_core: PASS_34_ISOLATED_WORK_ONLY; workspace_tests: PASS_FINAL_SOURCE_CI_476_EACH_PLATFORM_4_IGNORED_SEPARATE_RENDERER; workspace_clippy: PASS_ALL_TARGETS; combined_full_original_bundle: PASS_SAME_OPTIMIZED_EXE_NATIVE_SDK_CLI_BROWSER; exact_commit_ci: PASS_2A81036_ALL4; immutable_release: PASS_NATIVE11_SDK12_CLI_BUDGET4_UI13; installed_runtime: PASS_B11_7JOBS_19DURABLE_FILES; ui_contract: PASS_28; ui_browser_fixture: PASS_34; mapping_oracle_controls: PASS; final_app_tests: PASS_98; worker_maximum_depth: PASS_256_PLUS1_REFUSED_ON_1MIB_CALLER; cli_bundle_fixed_ceiling: PASS_NATIVE_64MIB_EXACT_PLUS1; baseline_exact_commit_ci: PASS_B8D6908_ALL4.

- [ ] **Nächster Baustein – B12: Sitemaps und Mitgliedschaft** — LOCAL_WINDOWS794_NATIVE13_OPTIMIZED_INSTALLED_CI_UNAVAILABLE_USER_AUTHORIZED. Qualifizierte Sitemap-Quellenabdeckung und Lastmod-Auswertung sind auf8787 installiert. Der vollständige lokale Workspace794 ohne Fehler oder ausgesparte Tests, Format/Clippy und13 native API-/CSV-/SDK-/Browsergruppen mit24Tools/320px bestehen am optimierten Release ab1ac93c. Sieben historische Jobs und19 dauerhafte Dateien bleiben unverändert; eigener Renderer8790 und verifiziertes Backup bereit. Lokale Prüfungen ersetzen auf Nutzerwunsch vorerst die gesperrten GitHub Actions. Ziel-/Inlink-Auswertung, Sitemap-ZIP-Erzeugung und übrige Abnahmen bleiben in Arbeit; GitHub/Linux/Container und Hersteller-Parität bleiben NOT_PROVEN.
  - [x] Namespace-/Elternstruktur, Rohwerte, Duplikate, fehlende/ungültige Locs, vollständige XML-Prüfung und tatsächliche Gzip-Grenzen im integrierten Core-Parser geprüft: 53 neue Tests und ein erhaltener historischer Test.
  - [x] Vier absichtlich beschädigte Work-Varianten werden durch gerichtete Laufzeit-Assertions erkannt; integrierte Quellen unabhängig geprüft, Alltargets-Clippy grün.
  - [x] Expliziter Sitemap-Fetch-Zweck, Datei-Gzip nach tatsächlichem HTTP-Decoding und unabhängige Ressourcenrechte im realen Netzwerkpfad angebunden; zwölf gerichtete HTTP-Core-Tests und alle329 Core-Unit-Tests bestehen.
  - [x] Ordinary/Policy × Ressourcen an/aus sowie Dateiname-/HTTP-Gzip-/CRC-/Namespace-Kontrollen über vollständige App/API und tatsächliche Zielrequests geprüft:16 grün; portable CI-Fixture prüft finale Request-Zähler nach eigenem Prozessstopp.
  - [x] Vorheriger Parser-Commit6171a32 besteht alle vier Quell-CI-Jobs; jeweils529 Standardtests bestehen auf Windows/Linux bei vier separat geprüften Browser-Tests.
  - [x] Transport d3d1e04: vier exakte CI-Jobs grün, Windows/Linux jeweils541 Standardtests; optimierter unveränderlicher Windows-Release lokal mit16 Sitemap-Fällen und46 vollständig geleerten Request-Zeilen geprüft.
  - [x] Derselbe optimierte Release besteht native11/SDK12 und Browser13; datenbewahrend auf Port8787 installiert. Sieben bestehende Jobs und19 dauerhafte Dateien sind byteidentisch, fünf Assets und23 read-only Inventar-Tools geprüft.
  - [x] Originale Robots-Sitemap-Deklarationen mit LF/CRLF/CR, Byte-Spans, Rohwerten, echten EOF-/512KiB-Grenzen und qualifizierten206-/UTF8-/Readfehlern im Core erhalten; alle353 Core-Unit-Tests und Alltargets-Clippy bestehen.
  - [x] Strikter gespeicherter Transport-Beweisprüfer besteht eine synthetische Positivkontrolle und24 Negativkontrollen sowie tatsächliche Debug-/Release-Companion-Prüfungen. Exakte Robots-CI6597c9c archiviert vollständige Requests, App-Logs und Originalpfad-Validierung mit festen Namen und vollständigen Hashes/Bytes.
  - [x] Robots-Quellcommit6597c9c: vier exakte CI-Jobs grün, Windows/Linux jeweils565 Standardtests; derselbe optimierte Release besteht lokal16 Sitemap-Fälle/46 geleerte Requests, native11/SDK12 und Browser13 und ist datenbewahrend installiert.
  - [x] Historischer V4-Work-Kandidat:437 Tests bestanden, die anschließend gefundene Ordinary-Root-Bindungslücke war damals offen. Neuer Main-Beleg dokumentiert V5-Reparatur und508 bestandene Core-Tests; der historische Beleg bleibt unverändert.
  - [x] Ordinary-Root-Rohwert, ursprünglicher Ordinal und normalisierte Originalidentität werden auch ohne Policy-ID an gespeicherte Originaldaten gebunden. Dieselben21 Kontrollen zeigen17 erwartete negative Assertions gegenV4 und alle21 erfolgreich gegenV5;63 V5-Modultests bestehen.
  - [x] Dauerhafte typisierte Dokument-/Root-/Entry-/Loc-Vorkommen, originale Config-/Import-Bindung und atomare Ordinary-/Policy-Admission im Hauptcode integriert. Formatierter Main-Core-Snapshot besteht508 Tests, einschließlich8 vollständiger Crawler-Bootstrap-Fälle; Workspace-Alltargets-Clippy besteht.
  - [x] Bericht/Checkpoint lesen vollständig validierte Kataloge; bekannte leere Daten und unbekannte Altbestände bleiben getrennt. Ausgewählte Vererbung bindet den exakten Parent-Snapshot. Diese Core-Abnahme beweist noch keine vollständige Quellenabdeckung oder native Recovery-Matrix.
  - [x] Zusätzliche Quell-CI265062f meldete eine echte Regression im unveränderten externen Policy-Sitemap-Test. Main-Reparatur erhält ungültige vorhandene Locs im Policy-Protokoll mit ursprünglichem Wert/Ordinal/Parserfehler; leniente Normalisierung erzeugt keine Admission. Alltargets-Clippy besteht. Neue vollständige Quell-CI/Laufzeitabnahme stehen aus.
  - [x] Aktuelle reparierte Main-Core-Quellen59e8453 bestehen510 lokale Unit-Tests, darunter12 gerichtete Discovery-Fixtures mit den zwei neuen Kontrollen. Die unveränderte externe Policy-Integrationsfixture bleibt eine getrennte offene Laufzeitabnahme. Neuer GitHub-Lauf38355115138 startet aufgrund Konto-/Billing-Sperre null Tests; Rohantwort/Annotation archiviert.
  - [x] Lesende vollständig validierte SQLite-Snapshot-API mit Filter-vor-Pagination, gebundenen Cursor-/Exporttokens, JSON8MiB/CSV64MiB-Fehlergrenzen und MCPspider_sitemaps integriert. Zwölf neue App-Kontrollen bestehen: fünf tatsächliche Crawler/Store/Handler-, sechs pure Cursor/Filter/Writer- und eine MCP-Vor-I/O-Prüfung. Startup-SQLite-Arbeitsdateien sind ausdrücklich getrennt; gemessene Handler-/CSV-Lesezugriffe erhalten alle Dateibytes und verursachen final nach Drain null Website-Anfragen.
  - [x] Vollständiger lokaler Cargo-Workspace am festen197-Dateien-Quellstand besteht736 Tests,0 Fehler und4 separat ausgesparte Browser-/Renderer-Fälle. Core510/App112 sowie unveränderte Policy13 bestehen; Format und Workspace-Alltargets-Clippy grün.
  - [x] Sitemap-Browseransicht an exakt integrierten Assets besteht zehn eigene Headless-Chrome/API-Fixture-Gruppen: begrenzte Seiten, Originale/Aliase, unbekannt/leer/vererbt, stale Antworten/Generationen, inerte XML-Rohwerte, CSV und320px. Sichtbare200/100/50/10/1-Zeilenauswahl ermöglicht explizite Erholung von413 ohne heimliche Kürzung. Dies ist Mock-UI-Beleg, keine native Browserabnahme.
  - [x] Tatsächlicher gespeicherter Ledger: elf native Szenariogruppen und elf SQLite-Jobs bestehen an demselben separat gebauten Windows-Dev-Binary. Originale/Policy-Aliase, Roots/Child-Queries, vollständige/206-Robots, beschädigtes Gzip, bekannte leere Daten, Pause/Neustart/Resume, unveränderte Elternvererbung, Legacy-unknown und ungültiger Parent geprüft. Drei eigene Apps tatsächlich geschlossen;40 finale Request-Oracles und28 vollständig geleerte Zielanfragen. Keine Kohorten-/Budget-/Power-Loss-Parität daraus ableiten.
  - [x] Tatsächliche geroutete SQLite-Snapshot-API, voller gespeicherter CSV-Katalog, beide offiziellen MCP-SDK-Transporte mit24 Tools und native Chrome-Ansicht bestehen elf Gruppen einschließlich Kaltstart und320px. Zwei eigene Apps und zwei SDK-stdio-Produzenten tatsächlich geschlossen; alle gemessenen Dateibytes/Zielzähler unverändert. Vollständige Rohantworten, Download und kompakter Screenshot archiviert.
  - [x] Native Skripte im Repository versioniert und über gemeinsamen Runner tatsächlich erneut ausgeführt. Ledger byteidentisch zum eingefrorenen V5; Membership entfernt Work-Pfadabhängigkeit und bindet den Harness vor/nach Ausführung. CI-Verdrahtung für Windows/Linux, optimierten Windows-Ledger und Linux-Browser ergänzt; tatsächlicher CI-Lauf bleibt offen. Wiederholungen zählen nicht als zusätzliche Feature-Abdeckung.
  - [x] Nutzerfreigabe für lokalen Ersatz der GitHub-Actions-Sperre: optimierter Windows-Release tatsächlich gebaut und geprüft; Runtime-Rust/Lock/Assets entsprechen197 Compiler-Dateien, nur drei Integrationstest-Endpunkte wurden für eigenen Renderer anpassbar gemacht. GitHub/Linux/Container ausdrücklich NOT_PROVEN, keine Billing-Einstellungen geändert.
  - [x] Vier bisher ausgesparte Browser-Tests an eigenem aufgewärmtem dynamischem Renderer ausgeführt: vollständiger lokaler Workspace740 besteht ohne Fehler oder Ignorierung; Workspace-Alltargets-Clippy und Format bestehen. Separate echte Renderer17 bestehen. Nutzer-Renderer8790 blieb unangetastet.
  - [x] Derselbe optimierte Release besteht Ledger11/SQLite11, geroutete API-/CSV-/SDK-/Browser11 mit24Tools/320px, Transport16 und vorhandene Originalbericht-/CLI-Bundle-Grenzen. Erfolgreich auf8787 installiert: sieben vollständige Job-Metadaten und19 dauerhafte Dateien byteidentisch, fünf Git-Assets und24MCP-Tools geprüft; Legacy-Sitemap-Historie bleibt unbekannt. Backup aufE verifiziert, Rollback nicht benötigt/ausgeführt.
  - [x] Aktueller Erweiterungsstand: 54 zusätzliche Rust-Tests, gesamter Workspace794/0Fehler/0ausgespart mit eigenem echtem Renderer, Clippy/Format bestanden. Derselbe optimierte Release ab1ac93c besteht13 native API-/CSV-/SDK-/Browsergruppen mit24Tools/320px, Gzip/Index-Zyklus, unvollständigem XML, aktuellen und geerbten qualifizierten Quellen-Kohorten sowie Lastmod-Profilen. Datenbewahrend auf8787 mit Renderer8790 installiert: sieben Job-Metadaten/19 dauerhafte Dateien unverändert, fünf eingebettete Assets und24Tools geprüft, historische Sitemap-Daten bleiben unbekannt; Backup verifiziert.
  - [ ] Budget2: Native JSON-Seite exakt8MiB und ein Byte darüber; kleinere Seite muss am selben gespeicherten Snapshot tatsächlich funktionieren.
  - [ ] Budget3: Nativer CSV-Export exakt64MiB und ein Byte darüber mit echten ursprünglichen Sitemap-Deklarationen; Überlauf ohne Teil-Download oder Mutation.
  - [ ] Budget4: Native einzelne Ledger-Zeile exakt256KiB und ein Byte darüber aus echter XML-Parserbeobachtung; neue Rows/Tasks müssen bei Überlauf gemeinsam zurückrollen.
  - [ ] Budget5: Nativer vollständiger Katalog exakt32MiB und ein Byte darüber über zwei reale Quellen; zuvor committed Quelle erhalten, zweite Quelle atomar ablehnen.
  - [ ] Budget6: Gezielte SQL-Fehler-Injektion ausschließlich offline in eigener bereits echt erzeugter Fixture-Datenbank; Ordinary/Policy-Rollback und Wiederaufnahme ohne erfundene Herkunft prüfen.
  - [ ] Budget7: Nur tatsächlich bestandene physische Grenzen veröffentlichen, gescheiterte Kandidaten erhalten und TODO/GitHub aktualisieren; Quell-CI/Release/Installation bleiben getrennt.
  - [ ] Live-Generationswechsel im nativen API-/Browserpfad prüfen.
  - [ ] GitHub-/Linux-/Container-CI nach Konto-Verfügbarkeit getrennt ausführen. Lokale794/0ausgespart, optimierter Release/native13 und datenbewahrende Installation sind bestanden; Remote-CI bleibt NOT_PROVEN.
  - [ ] Originale Mitgliedschaften an tatsächliche Fetch-/Inlink-Zustände anbinden und konkrete Sitemap-Zielfehler ausgeben; vollständige Website-Abdeckung bleibt unbewiesen.
  - Verträge: [sitemap-b12-plan.md](parity/sitemap-b12-plan.md), [sitemap-parser.md](sitemap-parser.md), [sitemap-transport.md](sitemap-transport.md), [robots-sitemap-source.md](robots-sitemap-source.md), [sitemap-ledger.md](sitemap-ledger.md), [sitemap-membership.md](sitemap-membership.md), [sitemap-native-acceptance.md](sitemap-native-acceptance.md), [sitemap-budget-plan.md](parity/sitemap-budget-plan.md), [sitemap-budget-plan-source-pins.json](parity/sitemap-budget-plan-source-pins.json), [sitemap-assessment.md](sitemap-assessment.md).
  - Gate-Stand: source_review: PARSER_AND_FETCH_NO_CONCRETE_P1_P2; implementation: MAIN_DURABLE_WRITER_AND_READONLY_API_CSV_MCP_UI; native_acceptance: PASS_LOCAL_TRANSPORT16_LEDGER11_MEMBERSHIP11_SEPARATE_SCOPES; work_parser: PASS_53_PLUS_EXISTING_NORMALIZER2; main_core_parser: PASS_54; main_core_clippy: PASS_ALL_TARGETS; main_standalone: PASS_56_SEPARATE; work_mutations: PASS_4_EXPECTED_RUNTIME_ASSERTION_FAILURES; exact_commit_ci: PASS_6597C9C_FOUR_JOBS_565_EACH; main_core_fetch_http: PASS_12; main_core_all_unit: PASS_CURRENT_CARGO_CORE510; main_portable_transport: PASS_16_FINAL_DRAIN; parser_exact_commit_ci: PASS_FOUR_JOBS_6171a32; optimized_transport_release: PASS_SAME_OPTIMIZED_EXE_16_NATIVE_SDK_BROWSER; installation: PASS_LOCAL_OPTIMIZED_EA314045_AB1AC93C_7JOBS_19FILES; robots_source_core: PASS_353_AND_ALLTARGETS_CLIPPY; transport_proof_validator: PASS_SYNTHETIC_1POSITIVE_24NEGATIVE_AND_ACTUAL_DEBUG_RELEASE_COMPANIONS; new_robots_validator_ci: PASS_6597C9C_FOUR_JOBS_FULL_RAW_ARCHIVE; durable_source_ledger: MAIN_PASS_NATIVE_LEDGER11_SQL11_DRAINED; durable_work_core: HISTORICAL_437_V4_GAP_FIXED_BY_V5; durable_original_binding: PASS_V4_17_EXPECTED_FAILURES_V5_ALL_21_CONTROLS; durable_full_crawler_bootstrap: PASS_8_INCLUDED_IN_CORE_508; durable_workspace_clippy: PASS_LOCAL_CARGO_ALLTARGETS_SAME_SOURCE; durable_source_cohort: PASS_25_CORE_AND_NATIVE13_QUALIFIED_DECLARED_SCOPE; durable_exact_commit_ci: NOT_PROVEN_GITHUB_ACCOUNT_UNAVAILABLE_LOCAL_SCOPE_SEPARATE; durable_api_csv_ui: PASS_ROUTED_NATIVE_API_CSV_SDK_BROWSER11; durable_installation: PASS_LOCAL_USER_AUTHORIZED_740_NATIVE_RELEASE_24TOOLS; durable_policy_invalid_locs: PASS_UNCHANGED_EXTERNAL_POLICY13_AND_CORE_DISCOVERY12; durable_account_gate: DEFERRED_REMOTE_CI_USER_AUTHORIZED_LOCAL_ROUTE_NO_SETTINGS_CHANGE; durable_workspace_tests: PASS_LOCAL794_0FAIL_0IGNORED_REAL_BROWSER; durable_actual_browser: PASS_NATIVE_CHROME_SAME_ASSETS_320PX; durable_official_mcp_sdk: PASS_BOTH_OFFICIAL_TRANSPORTS24_TOOLS; durable_native_matrix: PASS_WINDOWS_DEV11_SCENARIOS11_SQL_REQUEST_DRAIN; durable_native_byte_budgets: PENDING_FULL_SIZE_EXACT_PLUS1_JSON_CSV; durable_native_generation_race: PENDING_LIVE_ACTIVE_GENERATION; durable_native_rollback_budget: PENDING_FAILURE_INJECTION; durable_budget_plan: TASK1_PASS_3_INCLUDED_IN_WORKSPACE794_PHYSICAL_GATES_SEPARATE; durable_live_renderer4: PASS_OWNED_EPHEMERAL_CURRENT_SOURCE; durable_optimized_release: PASS_LOCAL_WINDOWS_THIN_LTO_AB1AC93C; durable_lastmod: PASS_22_CORE_4_REAL_STORE_APP_AND_NATIVE_BROWSER; durable_assessment_installation: PASS_LOCAL_794_NATIVE13_INSTALLED_7JOBS_19FILES.

- [x] **B8: optionale Textaufnahme und gespeicherte Inhaltsanalyse** — COMPLETED. Der begrenzte B8-Baustein für optionale Roh-/DOM-Textaufnahme und gespeicherte ausgewählte Inhaltsanalyse ist mit eigenen nativen/API/UI-Gates, exakter Commit-CI und unveränderlichem installiertem Release geprüft. Vollständige Website-Analyse und CSS-Neuauswahl ohne erhaltenes HTML/DOM bleiben zusätzliche offene Feature-Abnahmen.
  - [x] Pure CSS-/Unicode-Auswahl20 und Binding4 sowie16 echte native Fixtures geprüft; ungültige Konfiguration scheitert vor HTTP.
  - [x] Rohe und tatsächliche Chromium-DOM-Quellen behalten getrennte Texte, Sprachen, Hashes und Herkunft; fehlender Renderer ersetzt keine Quelle.
  - [x] Capture, Seite, Ledger und Frontier werden zusammen gespeichert; Pause/Crash/Recrawl erhalten die tatsächliche Generation und unveränderte Eltern-/Restdaten.
  - [x] Acht native API-/CLI-Gruppen prüfen gespeicherte ausgewählte Analyse ohne Website-Anfragen; fehlend, leer, unvollständig und deklarativ bleiben getrennt.
  - [x] Tatsächliche gespeicherte Content-UI mit 12 Prüfungen und nativer/Chromium-Quelle bestanden; getrennte Fixture18 geprüft.
  - [x] Exakte B8-Commit-CI mit vier Jobs und unveränderlicher nativer Release installiert; sieben bestehende Jobs und elf JSON-Dateien nach Neustart byteidentisch erhalten.
  - [ ] Vollständige Website-Analyse und skalierbare große Corpora benötigen weitere Scheduler-/Index-/Budget-Fixtures; aktuell ist die Auswahl ausdrücklich auf1..128 gespeicherte Quellen begrenzt.
  - [ ] Geänderte CSS-Auswahl ohne HTTP braucht gespeichertes volles HTML/DOM. Ausgewählter Text und HTML-Hash ersetzen dieses Artefakt nicht.
  - [ ] Eigene Text-/Readability-/Jaccard-Profile und begrenzte Originalbeobachtungen sind keine vollständige Hersteller- oder Filter-Parität.
  - Verträge: [content-capture.md](content-capture.md), [content-analysis.md](content-analysis.md), [reference-content-capture.md](reference-content-capture.md), [ROADMAP.md](ROADMAP.md).
  - Gate-Stand: pure_capture: PASS; native_capture_and_persistence: PASS; actual_chromium_capture: PASS; saved_corpus_api_cli: PASS; content_ui_fixture: PASS; content_ui_actual: PASS; aggregate: PASS; batch_ci_release_install: PASS; related_actual_browser_53: PASS; exact_commit_ci: PASS; immutable_release: PASS; installed_runtime: PASS.

Das nächste Häkchen entsteht erst nach nachvollziehbaren Tests der offenen Kriterien und einem passenden Beweis im Katalog. Änderungen an Status oder Abnahme erfolgen dort; diese Datei wird danach neu erzeugt.

## Vollständige Liste nach Gruppen

Die Orientierungssätze sind eine deutsche Arbeitshilfe. Die eingeklappten Quellkriterien enthalten die unveränderte technische Abnahme, alle offenen Kriterien und alle implementierungsbezogenen Belege aus dem Katalog.

### Crawl-Modi und Steuerung

7 Einträge: 6 eigene Abnahmen erfüllt, 0 teilweise geprüft, 1 offen.

#### Noch offen

- [ ] **URLs ausschließlich über APIs anreichern** — Offen; `modes.apis`, P1.

  Orientierung: API-Anreicherung, fehlende Daten und null Website-Anfragen prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "modes.apis",
    "reference_name": "API-only enrichment",
    "status": "planned",
    "acceptance_test": "Enrich supplied URLs using a mock API; record zero website requests.",
    "remaining_acceptance": [
      "Enrich supplied URLs using a mock API; record zero website requests."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **Website rekursiv crawlen** — Eigene Abnahme erfüllt; `modes.spider`, P1.

  Orientierung: Der rekursive Kern ist gegen eigene Fixtures geprüft.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [analysis](analysis-acceptance.md) (PASS), [api](verification/api-e2e.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "modes.spider",
    "reference_name": "Website spider",
    "status": "implemented",
    "acceptance_test": "Crawl a synthetic linked graph; fetch each normalized URL once and preserve every link occurrence.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "analysis",
          "api"
        ],
        "proves": "The 601-URL core fixture fetches the fragment-duplicate destination once, and the 604-URL API fixture completes. Raw links retain occurrences; analysis separately tests repeated anchors and unique degrees."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **URL-Listen importieren und crawlen** — Eigene Abnahme erfüllt; `modes.list`, P1.

  Orientierung: Geordnete Listen mit Herkunft, Duplikaten und ungültigen Zeilen sind geprüft.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [list-import-batch](verification/list-import-batch.json) (PASS), [list-import-api-cli](verification/list-import-e2e.json) (PASS), [ui-list-import](../web/verification/ui-list-import-live-results.json) (PASS), [list-import-state](../web/verification/list-import-state.test.mjs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "modes.list",
    "reference_name": "List import",
    "status": "implemented",
    "acceptance_test": "Import TXT, CSV, XLSX and XML; keep original row order and duplicate inputs in exports while deduplicating network work.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "ui-fixture"
        ],
        "proves": "Seed URL arrays with crawl_links=false fetch only listed URLs. UI list-mode payload and bounded body/noindex behavior are tested."
      },
      {
        "verification_run_ids": [
          "list-import-batch",
          "list-import-api-cli",
          "ui-list-import",
          "list-import-state"
        ],
        "proves": "Pure native bounded TXT/CSV/XML/XLSX parsing retains row order, blanks/errors, duplicate values and explicit cell/formula facts. Actual API/CLI/browser crawl fetches unique page identities once; inputs.csv repeats original occurrences joined to captured observations. XML index candidates remain sitemap documents. Import/inspection performs zero website requests."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **SERP-Listen offline verarbeiten** — Eigene Abnahme erfüllt; `modes.serp`, P1.

  Orientierung: Der eigene Offline-Modus mit null Website-Anfragen ist geprüft.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [api](verification/api-e2e.json) (PASS), [offline-serp](../web/verification/ui-serp-live-results.json) (PASS), [serp-state-races](../web/verification/serp-state-race.test.cjs) (PASS), [resource-offline-batch](verification/resource-offline-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "modes.serp",
    "reference_name": "Offline SERP editing",
    "status": "implemented",
    "acceptance_test": "Import URL/title/description rows, calculate length and approximate width, and export without making network requests.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "api",
          "offline-serp",
          "serp-state-races",
          "resource-offline-batch"
        ],
        "proves": "Offline import/measurement/export uses native Rust without fetching supplied website URLs. The actual browser fixture observes zero website requests; quoted newline/Unicode values, ordered duplicate URLs, malformed CSV errors, unknown versus zero widths, reload persistence and formula-safe CSV versus exact JSON values are verified. Thirteen focused editor regressions cover file-read/import/preview races and concurrent edits/add/delete."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Gespeicherte Crawls vergleichen** — Eigene Abnahme erfüllt; `modes.compare`, P1.

  Orientierung: Neue, entfernte und veränderte Seiten gemeinsam in einer Fixture prüfen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS), [saved-comparison-native](verification/saved-comparison-e2e.json) (PASS), [saved-comparison-backend-checkpoint](verification/saved-comparison-backend-checkpoint.json) (PASS), [saved-comparison-ci](verification/saved-comparison-ci.json) (PASS), [saved-comparison-release](verification/saved-comparison-release.json) (PASS), [saved-comparison-installed](verification/saved-comparison-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "modes.compare",
    "reference_name": "Snapshot comparison",
    "status": "implemented",
    "acceptance_test": "Compare two saved fixture snapshots and classify new, missing and changed URLs.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "api"
        ],
        "proves": "Snapshot comparison classifies added/removed/changed URL keys in code. Core recovery checks added URLs; API JSON reimport comparison proves an unchanged snapshot has zero changes."
      },
      {
        "verification_run_ids": [
          "saved-comparison-native",
          "saved-comparison-backend-checkpoint"
        ],
        "proves": "Two actual saved native cohorts jointly classify added, removed, changed and unchanged; API/CLI/SDK share identical deltas and typed hashes after website shutdown and App cold restart.",
        "limits": "Own versioned stored-fact comparison acceptance only; browser/release and full vendor parity remain separate."
      },
      {
        "verification_run_ids": [
          "saved-comparison-ci",
          "saved-comparison-release",
          "saved-comparison-installed"
        ],
        "proves": "Exact-source CI, immutable optimized native release and data-preserving user installation now pass the saved-fact comparison and optional complete typed-report export gates.",
        "limits": "Own contracts and stored observations; reference parity remains NOT_PROVEN."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Pausieren, stoppen und fortsetzen** — Eigene Abnahme erfüllt; `modes.control`, P1.

  Orientierung: Anfragen abarbeiten, persistieren und nach Neustart ohne fehlende Ergebnisse fortsetzen ist geprüft.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS), [crawl-control-core](../crates/core/tests/control_fixtures.rs) (PASS), [crawl-control](verification/crawl-control-e2e.json) (PASS), [ui-control-fixture](../web/verification/ui-crawl-control-fixture-results.json) (PASS), [ui-control-live](../web/verification/ui-crawl-control-live-results.json) (PASS), [native-policy-ci](verification/native-policy-ci.json) (PASS), [native-policy-installed](verification/native-policy-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "modes.control",
    "reference_name": "Pause, stop, resume",
    "status": "implemented",
    "acceptance_test": "Pause with requests in flight, persist frontier, restart, and complete with no missing or duplicated results.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "api",
          "ui-live"
        ],
        "proves": "Cancellation preserves a resumable report/frontier. API cancel/resume and process termination/restart finish 725 observations without re-fetching the completed seed."
      },
      {
        "verification_run_ids": [
          "crawl-control-core",
          "crawl-control",
          "ui-control-fixture",
          "ui-control-live"
        ],
        "proves": "Actual admitted HTTP/browser/PDF work drains; pending page/discovery work persists, SQLite-only restart makes zero HTTP, generation resume completes exact unique results with unchanged committed payloads. UI actions and delayed history/polls are generation-bound. Missing storage errors remain explicit."
      },
      {
        "verification_run_ids": [
          "native-policy-ci",
          "native-policy-installed"
        ],
        "proves": "Exact B7 source915e175 has four successful Windows/Linux/Chromium/Linux Docker CI jobs and an installed native Windows immutable release. Existing seven jobs and eleven original JSON files remain byte-identical after restart; no user-data fixture requests. This confirms the already documented B7 slice only; ongoing B8 capture/corpus changes and full reference parity are not attested."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Ausgewählte URLs erneut crawlen** — Eigene Abnahme erfüllt; `modes.recrawl`, P1.

  Orientierung: Erneute ausgewählte Anfragen mit unveränderten übrigen Ergebnissen und Elternhistorie sind geprüft.

  Beweisregister: [selected-recrawl-native](../crates/core/tests/recrawl_fixtures.rs) (PASS), [selected-recrawl-api-cli](verification/selected-recrawl-e2e.json) (PASS), [ui-selected-recrawl-fixture](../web/verification/ui-selected-recrawl-fixture-results.json) (PASS), [ui-selected-recrawl-live](../web/verification/ui-selected-recrawl-live-results.json) (PASS), [selected-recrawl-ci](verification/selected-recrawl-ci.json) (PASS), [native-policy-ci](verification/native-policy-ci.json) (PASS), [native-policy-installed](verification/native-policy-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "modes.recrawl",
    "reference_name": "Selective re-crawl",
    "status": "implemented",
    "acceptance_test": "Re-fetch a chosen failed URL after the fixture changes without clearing unrelated saved results.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "selected-recrawl-native",
          "selected-recrawl-api-cli",
          "ui-selected-recrawl-fixture",
          "ui-selected-recrawl-live",
          "selected-recrawl-ci"
        ],
        "proves": "The recorded failed404 changes to newly captured200 after an owned fixture changes; all four unrelated saved rows and exact immutable parent bytes remain unchanged. Native/API/CLI/browser selection preserves exact requested identities, ordered duplicate occurrences, independent attempt budgets, pending exclusion, retained/refreshed basis and durable selected-only pause/crash recovery. The own chosen-URL acceptance is fulfilled."
      },
      {
        "verification_run_ids": [
          "native-policy-ci",
          "native-policy-installed"
        ],
        "proves": "Exact B7 source915e175 has four successful Windows/Linux/Chromium/Linux Docker CI jobs and an installed native Windows immutable release. Existing seven jobs and eleven original JSON files remain byte-identical after restart; no user-data fixture requests. This confirms the already documented B7 slice only; ongoing B8 capture/corpus changes and full reference parity are not attested."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Entdeckung, Scope und URL-Regeln

14 Einträge: 3 eigene Abnahmen erfüllt, 9 teilweise geprüft, 2 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **Host-, Subdomain- und Ordnergrenzen** — Teilweise geprüft; `crawl.scope`, P1.

  Orientierung: Alle Scope-Varianten gemeinsam mit echten HTTP-/DNS-, Port- und Scheme-Fixtures prüfen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [policy-preview](../crates/core/tests/fixtures/crawl-policy/manifest.json) (PASS), [native-policy-core](../crates/core/tests/policy_fixtures.rs) (PASS), [native-policy-api-cli](verification/crawl-policy-e2e.json) (PASS), [ui-policy-live](../web/verification/ui-policy-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.scope",
    "reference_name": "Host, subdomain and folder scope",
    "status": "partial",
    "acceptance_test": "Verify exact host, all-subdomains and start-folder policies against an HTTP fixture with cross-scope links.",
    "remaining_acceptance": [
      "Prove the complete stated exact-host/all-subdomains/start-folder acceptance together on controlled HTTP/DNS fixtures, including scheme/port cross-scope observations; current subdomain edge cases are pure-policy rather than actual subdomain HTTP evidence."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "core-fixtures"
        ],
        "proves": "Exact host/port and optional subdomain boundaries are implemented. Tests reject suffix lookalikes and accept multiple explicitly seeded origins."
      },
      {
        "verification_run_ids": [
          "policy-preview",
          "native-policy-core",
          "native-policy-api-cli",
          "ui-policy-live"
        ],
        "proves": "Native exact origin and slash-bounded folder/exact-path scope, deny/check-once permissions, two-origin cross-scope links/redirects and credential isolation are proved on owned HTTP fixtures. Pure authored candidates separately cover scheme/port/subdomain and suffix-lookalike decisions. Optional policy defaults/seed exceptions remain explicit; browser policy combinations are rejected."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Ressourcentypen getrennt speichern und crawlen** — Teilweise geprüft; `crawl.resources`, P1.

  Orientierung: Unabhängige Schalter und Fixtures für CSS, JavaScript, Medien, PDF und SWF ergänzen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [core-unit](../crates/core/src) (PASS), [resource-discovery-api-cli](verification/resource-discovery-e2e.json) (PASS), [ui-resource-discovery-live](../web/verification/ui-resource-discovery-live-results.json) (PASS), [resource-discovery-batch](verification/resource-discovery-batch.json) (PASS), [resource-contract-matrix](verification/resource-contract-matrix.json) (PASS), [resource-contract-responses](verification/resource-contract-responses.json) (PASS), [resource-contract-api-cli-mcp](verification/resource-contract-e2e.json) (PASS), [resource-contract-ui](verification/resource-contract-ui.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.resources",
    "reference_name": "Per-type discovery and storage",
    "status": "partial",
    "acceptance_test": "Independently store and fetch HTML, CSS, JavaScript, images, media, PDF and legacy SWF references.",
    "remaining_acceptance": [
      "Prove the full independent four-product store/fetch matrix for CSS, JavaScript, media, PDF, legacy SWF and navigation with positive/negative native fixtures; the completed matrix is currently for images.",
      "Extend native CSS encoding/context and cross-origin redirect/interruption/budget fixtures; browser resource enforcement remains a separate unsupported combination."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "core-unit"
        ],
        "proves": "HTML extraction records anchors, stylesheet, script, image, iframe and selected media references; enabled link discovery fetches observed destinations and records response evidence."
      },
      {
        "batch": "B9-resource-discovery",
        "verification_run_ids": [
          "resource-discovery-api-cli",
          "ui-resource-discovery-live",
          "resource-discovery-batch"
        ],
        "proves": "Optional typed store/fetch controls and durable occurrences/grants/transports are integrated. Native/image four-product controls, observed HTML/CSS/JS/media/PDF/SWF families, private store=false responses, late role unions, pinned full exports and 506 distinct resource units are bounded-tested.",
        "limits": "All resource-type/navigation four-product combinations are not yet tested; native renderer combination rejected, browser cascade/selection unknown.",
        "source_debug_sha256_by_run": {
          "resource-discovery-api-cli": "142c0a1af3d420de5ccb1e182bdd04d701f8087a5dc72b3758464aa5e3d65724",
          "ui-resource-discovery-live": "142c0a1af3d420de5ccb1e182bdd04d701f8087a5dc72b3758464aa5e3d65724"
        }
      },
      {
        "batch": "B10-resource-contract-v2",
        "verification_run_ids": [
          "resource-contract-matrix",
          "resource-contract-responses",
          "resource-contract-api-cli-mcp",
          "resource-contract-ui"
        ],
        "proves": "Seven declared resource kinds in all four store/fetch products across both profiles, actual MIME and immutable role facts; explicit profile selection, persisted exports and UI checked.",
        "limits": "Bounded own native/local acceptance. Existing remaining acceptance and reference parity remain separately tracked; CI/release/installation pending."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Beziehungs-URLs gezielt entdecken** — Teilweise geprüft; `crawl.relationships`, P1.

  Orientierung: Eigene Entdeckungsschalter für Canonical, Pagination, AMP, mobile Alternativen und Refresh ergänzen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [analysis](analysis-acceptance.md) (PASS), [resource-discovery-api-cli](verification/resource-discovery-e2e.json) (PASS), [ui-resource-discovery-live](../web/verification/ui-resource-discovery-live-results.json) (PASS), [resource-discovery-batch](verification/resource-discovery-batch.json) (PASS), [resource-contract-matrix](verification/resource-contract-matrix.json) (PASS), [resource-contract-core](verification/resource-contract-core.json) (FOCUSED_CORE_PASS_FULL_MATRIX_PENDING).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.relationships",
    "reference_name": "Relationship discovery",
    "status": "partial",
    "acceptance_test": "Enable discovery separately for canonicals, hreflang, next/prev, AMP, iframe, mobile alternate and meta refresh.",
    "remaining_acceptance": [
      "Prove separate enabled/disabled native request paths for canonical, hreflang, next/prev, AMP, iframe, mobile alternate and meta refresh, including source evidence, scope and selected-recrawl ceilings."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "analysis"
        ],
        "proves": "HTML canonical/hreflang values are resolved and audited against observed targets; iframe references are recorded."
      },
      {
        "batch": "B9-resource-discovery",
        "verification_run_ids": [
          "resource-discovery-api-cli",
          "ui-resource-discovery-live",
          "resource-discovery-batch"
        ],
        "proves": "Typed per-kind navigation grants preserve observed relationship sources; record-only canonical/frame references and navigation-disabled/list/selected controls remain unfetched in native/API/UI fixtures.",
        "limits": "Each relationship kind still needs independent enabled and disabled native paths; enum/config availability is not a complete behavior gate.",
        "source_debug_sha256_by_run": {
          "resource-discovery-api-cli": "142c0a1af3d420de5ccb1e182bdd04d701f8087a5dc72b3758464aa5e3d65724",
          "ui-resource-discovery-live": "142c0a1af3d420de5ccb1e182bdd04d701f8087a5dc72b3758464aa5e3d65724"
        }
      },
      {
        "batch": "B10-resource-contract-v2",
        "verification_run_ids": [
          "resource-contract-matrix",
          "resource-contract-core"
        ],
        "proves": "All ten navigation source roles in eight store/fetch/expand products, positive child controls and invalid combinations before HTTP; HTML/HTTP qualification retains source provenance.",
        "limits": "Bounded own native/local acceptance. Existing remaining acceptance and reference parity remain separately tracked; CI/release/installation pending."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **URL-Identitäten nachvollziehbar normalisieren** — Teilweise geprüft; `crawl.normalization`, P1.

  Orientierung: Unicode, Prozentkodierung und wiederholte Query-Parameter ohne unbelegte Zusammenführung prüfen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.normalization",
    "reference_name": "URL identity and encoding",
    "status": "partial",
    "acceptance_test": "Cover relative URLs, base href, Unicode host/path, percent encoding, fragments and duplicate query strings without merging distinct resources.",
    "remaining_acceptance": [
      "Add explicit Unicode host/path, percent-encoding and repeated-query fixture cases without speculative identity merging."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "core-fixtures"
        ],
        "proves": "Relative links/base href, lowercase host/default port, fragment removal and query-order preservation are tested. Duplicate fragments do not duplicate network work."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Ein- und Ausschlussregeln** — Teilweise geprüft; `crawl.include_exclude`, P1.

  Orientierung: Ressourcen und Beziehungsquellen vollständig prüfen; Browser-Navigation und Subressourcen bleiben offen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [ui-renderer](../web/verification/ui-renderer-live-results.json) (PASS), [native-policy-core](../crates/core/tests/policy_fixtures.rs) (PASS), [native-policy-api-cli](verification/crawl-policy-e2e.json) (PASS), [ui-policy-fixture](../web/verification/ui-policy-fixture-results.json) (PASS), [ui-policy-live](../web/verification/ui-policy-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.include_exclude",
    "reference_name": "Include/exclude expressions",
    "status": "partial",
    "acceptance_test": "Exercise allow and deny regexes against links and resources with an explained exclusion reason.",
    "remaining_acceptance": [
      "Complete dedicated resource-type allow/deny and discovery-switch fixtures for CSS/JavaScript/media/PDF/SWF and supported relationship sources; renderer/navigation/subresource policy is explicitly unsupported."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "ui-fixture",
          "ui-renderer"
        ],
        "proves": "Validated Rust URL regexes apply before link/sitemap enqueue; tests preserve explicit seed exemptions and reject invalid definitions before HTTP. UI regex configuration payload is checked."
      },
      {
        "verification_run_ids": [
          "native-policy-core",
          "native-policy-api-cli",
          "ui-policy-fixture",
          "ui-policy-live"
        ],
        "proves": "Explicit native policy records include/exclude decisions with per-occurrence reasons and raw/fetch identities instead of silently deleting observed links; denied redirects are never fetched and outside-scope check-once responses never expand. Legacy top-level regex and native policy cannot be silently combined; explicit seeds/import errors stay differentiated."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Vom Nutzer gewählte Crawl-Grenzen** — Teilweise geprüft; `crawl.limits`, P1.

  Orientierung: Grenzen für Tiefe, Ordner, Query, Subdomains, URL-Länge und Linkanzahl ergänzen und einzeln prüfen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [renderer](../renderer/tests/worker.test.mjs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.limits",
    "reference_name": "Operator crawl controls",
    "status": "partial",
    "acceptance_test": "Test total, depth, URLs-per-depth, folder, query, subdomain, redirect, URL length, link-count and response-size bounds.",
    "remaining_acceptance": [
      "Implement or explicitly specify URLs-per-depth, folder/query/subdomain counts, URL length and link-count controls; exercise every configured bound."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "renderer"
        ],
        "proves": "Optional total/depth, concurrency, HTTP body, redirect and sitemap-document limits exist; body truncation and cancellation are tested. Renderer contexts/queue/deadline/HTML are bounded."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Rate und Parallelität begrenzen** — Teilweise geprüft; `crawl.speed`, P1.

  Orientierung: Per-Origin-Rate und Parallelität mit Zeitstempeln und verzögerten Antworten prüfen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [renderer](../renderer/tests/worker.test.mjs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.speed",
    "reference_name": "Concurrency and request speed",
    "status": "partial",
    "acceptance_test": "Use a timestamped fixture to prove request-rate bounds and bounded concurrency under latency.",
    "remaining_acceptance": [
      "Add timestamped HTTP fixtures asserting the core per-origin request-rate and concurrency bounds under latency."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "renderer"
        ],
        "proves": "Core scheduling caps concurrent tasks and spaces per-origin starts; renderer pool tests prove max four concurrent jobs with bounded queuing."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Timeouts und begrenzte Wiederholungen** — Teilweise geprüft; `crawl.timeouts_retries`, P1.

  Orientierung: Backoff und getrennte DNS-, Netzwerk-, Timeout- und 5xx-Zustände ergänzen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [renderer](../renderer/tests/worker.test.mjs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.timeouts_retries",
    "reference_name": "Timeout and retry policy",
    "status": "partial",
    "acceptance_test": "Distinguish timeout, DNS/network and HTTP errors; retry selected 5xx responses with bounded attempts.",
    "remaining_acceptance": [
      "Introduce bounded retry rules/backoff and dedicated DNS, network, native timeout and selected-5xx distinctions."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "renderer"
        ],
        "proves": "HTTP failures are retained with error messages; slow rendering returns 504 and releases capacity. Native timeout configuration and resumable cancellation exist."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Header, Authentifizierung und Proxy** — Teilweise geprüft; `crawl.headers_proxy`, P1.

  Orientierung: Proxy-Konfiguration prüfen und Geheimnisse aus gemeinsam exportierten Einstellungen entfernen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [renderer](../renderer/tests/worker.test.mjs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.headers_proxy",
    "reference_name": "Agents, headers and proxies",
    "status": "partial",
    "acceptance_test": "Verify custom user agent/header and proxy behavior using captured fixture requests; redact secrets from export.",
    "remaining_acceptance": [
      "Add explicit proxy configuration/proof and secret-redacted config/report exports."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "renderer"
        ],
        "proves": "Custom UA/header handling and malformed header rejection are tested. Both native HTTP and Chromium two-origin tests keep test credentials away from robots/external requests and cross-origin redirects."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Noch offen

- [ ] **Ausgewählte CDN-Hosts intern behandeln** — Offen; `crawl.cdns`, P1.

  Orientierung: Konfigurierte CDN-Grenzen ohne unbegrenzte externe Traversierung ergänzen.

  Beweisregister: [resource-discovery-api-cli](verification/resource-discovery-e2e.json) (PASS), [ui-resource-discovery-live](../web/verification/ui-resource-discovery-live-results.json) (PASS), [resource-discovery-batch](verification/resource-discovery-batch.json) (PASS), [resource-contract-core](verification/resource-contract-core.json) (FOCUSED_CORE_PASS_FULL_MATRIX_PENDING), [resource-contract-api-cli-mcp](verification/resource-contract-e2e.json) (PASS), [resource-contract-ui](verification/resource-contract-ui.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.cdns",
    "reference_name": "CDN classification",
    "status": "planned",
    "acceptance_test": "Classify configured CDN hosts as internal while preventing unrestricted external traversal.",
    "remaining_acceptance": [
      "Implement and prove the declared internal-CDN classification semantics while preventing unrestricted navigation; preserve the existing exact external-resource origin and credential boundaries."
    ],
    "implementation_evidence": [
      {
        "batch": "B9-resource-discovery",
        "verification_run_ids": [
          "resource-discovery-api-cli",
          "ui-resource-discovery-live",
          "resource-discovery-batch"
        ],
        "proves": "Exact configured external resource origin is fetched without external navigation or expanded credential origins; two-origin robots and allowed/blocked request controls passed.",
        "limits": "This is an explicit external resource grant. Classifying CDN hosts as internal under the original grouped acceptance is not implemented or claimed.",
        "source_debug_sha256_by_run": {
          "resource-discovery-api-cli": "142c0a1af3d420de5ccb1e182bdd04d701f8087a5dc72b3758464aa5e3d65724",
          "ui-resource-discovery-live": "142c0a1af3d420de5ccb1e182bdd04d701f8087a5dc72b3758464aa5e3d65724"
        }
      },
      {
        "batch": "B10-resource-contract-v2",
        "verification_run_ids": [
          "resource-contract-core",
          "resource-contract-api-cli-mcp",
          "resource-contract-ui"
        ],
        "proves": "Exact resource origins stay separate from navigation scope at each directed hop; unlisted third origin and navigation to resource-only origin denied with actual first-hop positive controls.",
        "limits": "Bounded own native/local acceptance. Existing remaining acceptance and reference parity remain separately tracked; CI/release/installation pending."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **TLS und HSTS nachvollziehbar behandeln** — Offen; `crawl.tls`, P1.

  Orientierung: Vertrauenswürdige Testzertifikate und HSTS-Herkunft ohne globale Abschaltung der Prüfung testen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.tls",
    "reference_name": "Trusted certificates and HSTS",
    "status": "planned",
    "acceptance_test": "Crawl a fixture with a user-trusted certificate and observe HSTS upgrade provenance without disabling validation globally.",
    "remaining_acceptance": [
      "Crawl a fixture with a user-trusted certificate and observe HSTS upgrade provenance without disabling validation globally."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **Crawlen ohne kommerzielle URL-Grenze** — Eigene Abnahme erfüllt; `crawl.unlimited`, P1.

  Orientierung: Mehr als 500 URLs sind im eigenen Crawl geprüft; Ressourcenbudgets bleiben ausdrücklich sichtbar.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.unlimited",
    "reference_name": "No commercial URL ceiling",
    "status": "implemented",
    "acceptance_test": "Complete a deterministic 501-URL fixture; configurable operator limits remain independent of licenses.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "api",
          "ui-fixture"
        ],
        "proves": "Default max_pages=None has no commercial ceiling. A 601-URL core fixture and 604-URL HTTP/API fixture complete; UI aliases do not introduce a cap. Explicit operator limits remain available."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Explizite URL-Umschreibungen** — Eigene Abnahme erfüllt; `crawl.rewrite`, P1.

  Orientierung: Query-Regeln und begrenzte Regex-Regeln erhalten Original- und Fetch-Identitäten samt Ledger.

  Beweisregister: [policy-preview](../crates/core/tests/fixtures/crawl-policy/manifest.json) (PASS), [crawl-control](verification/crawl-control-e2e.json) (PASS), [native-policy-core](../crates/core/tests/policy_fixtures.rs) (PASS), [native-policy-api-cli](verification/crawl-policy-e2e.json) (PASS), [ui-policy-live](../web/verification/ui-policy-live-results.json) (PASS), [native-policy-ci](verification/native-policy-ci.json) (PASS), [native-policy-installed](verification/native-policy-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.rewrite",
    "reference_name": "URL rewriting rules",
    "status": "implemented",
    "acceptance_test": "Apply saved query removal, regex replacement and rewrite rules; keep original and requested addresses for evidence.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "policy-preview",
          "crawl-control"
        ],
        "proves": "Pure explicit query removal and bounded regex replacement preserve original/resolved/normalized/fetch identities, duplicate source occurrences and explained denies. API/CLI preview agree with zero HTTP; that earlier preview gate did not prove live integration."
      },
      {
        "verification_run_ids": [
          "policy-preview",
          "native-policy-core",
          "native-policy-api-cli",
          "ui-policy-live"
        ],
        "proves": "Saved explicit query-key removal/order and bounded ordered Rust-regex replacements now drive native discovery admissions. Observed raw/resolved/fragment links remain intact while transformed fetch targets coalesce. Immutable admission SHA/permissions, ordered persisted occurrences/aliases, actual-target robots, original-origin credential grants, unrewritten redirect evaluation, atomic page/ledger/frontier commits and saved pause/resume/selected permissions are tested. Complete506-row API/CLI JSON/CSV snapshots retain every original occurrence. Existing own rewriting acceptance is fulfilled for explicit native policy; renderer+policy remains unsupported and fail-closed."
      },
      {
        "verification_run_ids": [
          "native-policy-ci",
          "native-policy-installed"
        ],
        "proves": "Exact B7 source915e175 has four successful Windows/Linux/Chromium/Linux Docker CI jobs and an installed native Windows immutable release. Existing seven jobs and eleven original JSON files remain byte-identical after restart; no user-data fixture requests. This confirms the already documented B7 slice only; ongoing B8 capture/corpus changes and full reference parity are not attested."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Nofollow- und verwandte Traversierungsregeln** — Eigene Abnahme erfüllt; `crawl.nofollow`, P1.

  Orientierung: Alle vier Direktiven mit konfigurierbarem Verhalten und erhaltenen Linkbeobachtungen prüfen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [analysis](analysis-acceptance.md) (PASS), [resource-discovery-api-cli](verification/resource-discovery-e2e.json) (PASS), [ui-resource-discovery-live](../web/verification/ui-resource-discovery-live-results.json) (PASS), [resource-discovery-batch](verification/resource-discovery-batch.json) (PASS), [resource-contract-core](verification/resource-contract-core.json) (FOCUSED_CORE_PASS_FULL_MATRIX_PENDING), [resource-contract-api-cli-mcp](verification/resource-contract-e2e.json) (PASS), [resource-contract-ui](verification/resource-contract-ui.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.nofollow",
    "reference_name": "Link directive policy",
    "status": "implemented",
    "acceptance_test": "Exercise nofollow, ugc, sponsored and page nofollow; store links regardless of whether discovery is enabled.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "analysis"
        ],
        "proves": "Anchor rel/nofollow values remain observable. Independent graph scoring excludes nofollow/ugc/sponsored/page nofollow; discovery currently follows stored references."
      },
      {
        "batch": "B9-resource-discovery",
        "verification_run_ids": [
          "resource-discovery-api-cli",
          "ui-resource-discovery-live",
          "resource-discovery-batch"
        ],
        "proves": "Actual native/UI nofollow target is retained as a source occurrence and remains unfetched under saved traversal settings.",
        "limits": "The observed nofollow control does not prove ugc, sponsored and page-nofollow in all enabled/disabled combinations.",
        "source_debug_sha256_by_run": {
          "resource-discovery-api-cli": "142c0a1af3d420de5ccb1e182bdd04d701f8087a5dc72b3758464aa5e3d65724",
          "ui-resource-discovery-live": "142c0a1af3d420de5ccb1e182bdd04d701f8087a5dc72b3758464aa5e3d65724"
        }
      },
      {
        "batch": "B10-resource-contract-v2",
        "verification_run_ids": [
          "resource-contract-core",
          "resource-contract-api-cli-mcp",
          "resource-contract-ui"
        ],
        "proves": "Explicit V2 internal/external qualifier switches cover nofollow/ugc/sponsored and actual document directives; original source occurrences and HTML/HTTP provenance remain visible, V1 compatibility remains separate. The exact stored native enabled/disabled traversal and source-preservation acceptance is fulfilled for the explicit V2 follow switches, confirmed by separate read-only acceptance review. This does not redefine global crawl_links:false or convert the preserved V1 behavior.",
        "limits": "Own explicitly selected V2 follow-switch contract only. A separate four-directive matrix under global crawl_links:false remains an additional B10 control; existing V1 behavior is intentionally preserved. Exact-source CI, optimized release and installation are independently passed in the B10 release checkpoint; full reference parity NOT_PROVEN."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Robots und Indexierbarkeit

7 Einträge: 2 eigene Abnahmen erfüllt, 4 teilweise geprüft, 1 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **Robots-Dateien abrufen und cachen** — Teilweise geprüft; `robots.fetch`, P1.

  Orientierung: Cache-Grenzen für Scheme, Host und Port mit unterschiedlicher Policy prüfen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [core-unit](../crates/core/src) (PASS), [api](verification/api-e2e.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "robots.fetch",
    "reference_name": "robots.txt retrieval and caching",
    "status": "partial",
    "acceptance_test": "Cache per origin and test distinct policy on scheme/host/port; record blocked URLs without fabricated status codes.",
    "remaining_acceptance": [
      "Add scheme/host/port cache separation fixtures with contrasting policy and declared cache lifetime."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "core-unit",
          "api"
        ],
        "proves": "Per-origin robots cache retrieves once in the 601-page fixture. Blocked pages have no fabricated HTTP status; robots 503/outage denies crawling with explicit evidence."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Robots-Agenten und Regeln auswerten** — Teilweise geprüft; `robots.matching`, P1.

  Orientierung: Vollständige Agentenauswahl und Endanker mit gerichteten Fixtures prüfen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [native-policy-core](../crates/core/tests/policy_fixtures.rs) (PASS), [native-policy-api-cli](verification/crawl-policy-e2e.json) (PASS), [ui-policy-live](../web/verification/ui-policy-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "robots.matching",
    "reference_name": "Agent matching and directives",
    "status": "partial",
    "acceptance_test": "Run fixtures for multiple user-agent groups, allow/disallow wildcards, end anchors and sitemap declarations.",
    "remaining_acceptance": [
      "Add end-anchor directive fixtures and the full declared agent-selection acceptance matrix."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "core-fixtures"
        ],
        "proves": "Google robotstxt parser tests multiple user-agent groups, wildcard Disallow and Allow precedence. Sitemap declarations drive the core sitemap fixture."
      },
      {
        "verification_run_ids": [
          "native-policy-core",
          "native-policy-api-cli",
          "ui-policy-live"
        ],
        "proves": "Native rewritten fetch URLs are checked against the same robots engine on actual target paths; owned denied targets receive zero requests and robots/protocol requests carry no seed credentials."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Explizite Robots-Ausnahme** — Teilweise geprüft; `robots.override`, P1.

  Orientierung: Gespeicherten Override und seine sichtbare Wirkung auf tatsächliche Anfragen prüfen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "robots.override",
    "reference_name": "Explicit robots override",
    "status": "partial",
    "acceptance_test": "Keep respect as the default; require a saved explicit override and show its state in crawl metadata.",
    "remaining_acceptance": [
      "Add a saved explicit-override fixture proving the changed request behavior and visible crawl metadata state."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "ui-fixture"
        ],
        "proves": "obey_robots defaults true and is stored in CrawlConfig; settings expose an explicit option. Respect behavior is proved by blocked target fixtures."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Indexierbarkeit mit Gründen erklären** — Teilweise geprüft; `robots.indexability`, P1.

  Orientierung: Jede Erklärung und Agenten-spezifische Direktive in einer vollständigen Fixture prüfen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [analysis](analysis-acceptance.md) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "robots.indexability",
    "reference_name": "Indexability explanations",
    "status": "partial",
    "acceptance_test": "Report concrete reasons for redirect, robots blocking, noindex, non-HTML and canonicalization with supporting evidence.",
    "remaining_acceptance": [
      "Add a dedicated fixture asserting every explanation and correct agent-specific directive interpretation."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "analysis",
          "ui-live"
        ],
        "proves": "Records explain non-200 requested status, robots blocking, errors, non-HTML, noindex, canonicalization, truncated body and rendering failure. Inspector exposes stored observations."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Noch offen

- [ ] **Expansion nach Indexierungsregeln steuern** — Offen; `robots.traversal`, P1.

  Orientierung: Noindex-, Canonical- und Pagination-Regeln anwenden, ohne Beobachtungen zu unterdrücken.

  Beweisregister: [resource-contract-core](verification/resource-contract-core.json) (FOCUSED_CORE_PASS_FULL_MATRIX_PENDING), [resource-contract-api-cli-mcp](verification/resource-contract-e2e.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "robots.traversal",
    "reference_name": "Directive-dependent discovery",
    "status": "planned",
    "acceptance_test": "Verify saved noindex/canonical/pagination policies influence expansion without suppressing observed evidence.",
    "remaining_acceptance": [
      "Verify saved noindex/canonical/pagination policies influence expansion without suppressing observed evidence."
    ],
    "implementation_evidence": [
      {
        "batch": "B10-resource-contract-v2",
        "verification_run_ids": [
          "resource-contract-core",
          "resource-contract-api-cli-mcp"
        ],
        "proves": "Actual meta and X-Robots nofollow share independent internal/external target overrides; document observations persist, redirects cannot borrow unrelated role scope.",
        "limits": "Bounded own native/local acceptance. Existing remaining acceptance and reference parity remain separately tracked; CI/release/installation pending."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **Robots-Regeln offline simulieren** — Eigene Abnahme erfüllt; `robots.test`, P1.

  Orientierung: Der gemeinsame Entscheidungsmechanismus und null Website-Anfragen sind geprüft.

  Beweisregister: [dom-robots-batch](verification/dom-robots-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "robots.test",
    "reference_name": "Custom policy simulation",
    "status": "implemented",
    "acceptance_test": "Evaluate supplied rules against a URL list without editing the remote site's robots.txt.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "dom-robots-batch"
        ],
        "proves": "Pure local shared crawler robots matcher accepts supplied rules and ordered URL occurrences, exposes invalid rows and limits, exports safe CSV and makes zero website or sitemap requests; actual browser and input-race fixtures pass."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Meta-Robots und X-Robots-Tag** — Eigene Abnahme erfüllt; `robots.meta`, P1.

  Orientierung: Der eigene Direktiven- und Whitespace-Vertrag ist geprüft.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [core-unit](../crates/core/src) (PASS), [advanced-batch](verification/advanced-batch.json) (PASS), [resource-fixtures](../crates/core/tests/resource_fixtures.rs) (PASS), [resource-offline-batch](verification/resource-offline-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "robots.meta",
    "reference_name": "Meta and HTTP robots",
    "status": "implemented",
    "acceptance_test": "Extract multiple meta robots and X-Robots-Tag fields; distinguish general and agent-specific directives.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "core-unit"
        ],
        "proves": "Multiple extracted meta directives and X-Robots-Tag values remain evidence. Noindex affects indexability in the list-mode fixture."
      },
      {
        "verification_run_ids": [
          "advanced-batch"
        ],
        "proves": "Multiple meta/X-Robots directives retain agent, source and raw evidence. Configured indexability agent is independent of HTTP UA; restrictive directives win and renderer response sources remain separate."
      },
      {
        "verification_run_ids": [
          "core-unit",
          "resource-fixtures",
          "resource-offline-batch"
        ],
        "proves": "Valued robots directives tolerate whitespace while preserving date-valued unavailable_after content. Multiple native X-Robots-Tag/meta values retain general versus configured-agent scope; a real fixture proves googlebot noindex is applied while bingbot-only nofollow is excluded. Previously pending whitespace handling is covered by the current unit/integration gates."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### HTTP, Fehler und Weiterleitungen

6 Einträge: 0 eigene Abnahmen erfüllt, 5 teilweise geprüft, 1 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **Initiale und finale HTTP-Antworten erfassen** — Teilweise geprüft; `http.response`, P1.

  Orientierung: Transferbytes, Timing-Phasen und mehrfach vorhandene Header vollständig erhalten.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS), [renderer](../renderer/tests/worker.test.mjs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "http.response",
    "reference_name": "Response evidence",
    "status": "partial",
    "acceptance_test": "Record status, final content type, measured bytes, transferred bytes, timings and complete header evidence.",
    "remaining_acceptance": [
      "Retain measured transfer bytes/timing phases and complete duplicate header provenance, not only a single-value map."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "api",
          "renderer"
        ],
        "proves": "Initial/final status, content type, response milliseconds, bounded decoded bytes and header maps are retained. Rendered status/headers remain separate from native evidence."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **HTTP- und Netzwerkfehler unterscheiden** — Teilweise geprüft; `http.failures`, P1.

  Orientierung: Maschinenlesbare 5xx-, DNS-, Timeout- und ungültige URL-Zustände vollständig prüfen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS), [renderer](../renderer/tests/worker.test.mjs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "http.failures",
    "reference_name": "Status and network failure groups",
    "status": "partial",
    "acceptance_test": "Seed 2xx, 3xx, 4xx, 5xx, timeout and malformed links; expose distinct machine-readable results.",
    "remaining_acceptance": [
      "Add the full native 5xx/DNS/timeout/malformed-link classification matrix with stable machine-readable error classes."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "api",
          "renderer"
        ],
        "proves": "2xx, redirects, 404, robots outage, redirect-loop and renderer timeout are tested and produce separate observations/issues."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Redirect-Ketten und Stop-Gründe** — Teilweise geprüft; `http.redirects`, P1.

  Orientierung: Weitere Endzustände und Kombinationen prüfen; fehlende Zwischenantworten ausdrücklich unbekannt lassen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS), [renderer](../renderer/tests/worker.test.mjs) (PASS), [core-unit](../crates/core/src) (PASS), [ui-resources](../web/verification/ui-resource-live-results.json) (PASS), [resource-offline-batch](verification/resource-offline-batch.json) (PASS), [native-policy-core](../crates/core/tests/policy_fixtures.rs) (PASS), [native-policy-api-cli](verification/crawl-policy-e2e.json) (PASS), [ui-policy-live](../web/verification/ui-policy-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "http.redirects",
    "reference_name": "Redirect chains and loops",
    "status": "partial",
    "acceptance_test": "Persist every hop including terminal URL; find chains, circular loops, off-scope destinations and non-200 endings.",
    "remaining_acceptance": [
      "Extend explicit chain-report acceptance across non-200 endings, malformed Location and traversal-limit combinations; intermediate bodies and most intermediate headers remain unretained and unknown."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "api",
          "renderer"
        ],
        "proves": "Redirect hop records, initial/final status and loop detection are tested. Worker same-origin and foreign-origin redirect behavior is verified without header leakage."
      },
      {
        "verification_run_ids": [
          "core-unit",
          "api",
          "ui-resources",
          "resource-offline-batch"
        ],
        "proves": "A separate native-response inventory and CSV retain each captured redirect-hop status/location/identity and terminal response without manufacturing intermediate document records. Actual 302 -> 307 -> 200 flow is verified. Real stopped-off-scope and loop regressions bind retained headers only to the last captured response by index, keep unrequested targets unobserved and preserve unknown intermediate headers/body facts."
      },
      {
        "verification_run_ids": [
          "native-policy-core",
          "native-policy-api-cli",
          "ui-policy-live"
        ],
        "proves": "Native policy records literal invalid/denied/loop/limit redirect locations before stopping, with distinct stop reasons; missing Location invents no occurrence. Unknown unrequested final body/status and protocol-source incompleteness remain explicit."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **HTTP-, Meta- und JavaScript-Weiterleitungen** — Teilweise geprüft; `http.refresh`, P1.

  Orientierung: Die verschiedenen Mechanismen mit eigenen Fixtures getrennt erkennen und benennen.

  Beweisregister: [core-browser](../crates/core/tests/crawl_fixtures.rs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "http.refresh",
    "reference_name": "Refresh and script redirects",
    "status": "partial",
    "acceptance_test": "Detect HTTP refresh, meta refresh and rendered JavaScript navigation; label each mechanism separately.",
    "remaining_acceptance": [
      "Detect and label HTTP Refresh, meta refresh and JavaScript navigation mechanisms in a dedicated fixture."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-browser"
        ],
        "proves": "Chromium final_url can observe a page after JavaScript navigation; rendered_url remains separate from native final_url."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Quellen defekter Links exportieren** — Teilweise geprüft; `http.broken_sources`, P1.

  Orientierung: Alle Quellvorkommen mit Dokumentposition und Pfad exportieren.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "http.broken_sources",
    "reference_name": "Broken-link source export",
    "status": "partial",
    "acceptance_test": "Export every source occurrence for a broken destination with anchor, rel and location.",
    "remaining_acceptance": [
      "Add an explicit broken-destination source export including per-occurrence document location/path."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "api",
          "ui-live"
        ],
        "proves": "Broken-outlink findings identify source pages; all-occurrence links CSV contains source, target, anchor, kind and rel. API/browser exports are tested."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Noch offen

- [ ] **Migrationsziele überprüfen** — Offen; `http.migration`, P1.

  Orientierung: Alte und erwartete neue URLs aus CSV mit tatsächlichen Endzielen vergleichen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "http.migration",
    "reference_name": "Migration destination validation",
    "status": "planned",
    "acceptance_test": "Check a CSV of old/new URL expectations and identify mismatched final destinations.",
    "remaining_acceptance": [
      "Check a CSV of old/new URL expectations and identify mismatched final destinations."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Metadaten und Inhalte

11 Einträge: 2 eigene Abnahmen erfüllt, 8 teilweise geprüft, 1 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **Seitentitel und Pixelbreiten** — Teilweise geprüft; `metadata.titles`, P1.

  Orientierung: Weitere Schrift-/DPI-Profile und komplexe Sprachformung getrennt kalibrieren; unbekannte Glyphen nicht raten.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [core-unit](../crates/core/src) (PASS), [api](verification/api-e2e.json) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [advanced-batch](verification/advanced-batch.json) (PASS), [native-text-width](parity/native-text-width-comparison.json) (PASS_BOUNDED_NATIVE_PROFILE), [reference-fields](parity/reference-comparison.json) (COMPARISON_EXECUTED), [resource-offline-batch](verification/resource-offline-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "metadata.titles",
    "reference_name": "Title extraction and audits",
    "status": "partial",
    "acceptance_test": "Test absent, empty, multiple, duplicate and configurable character/width thresholds with a known HTML fixture.",
    "remaining_acceptance": [
      "Representative other font/DPI display profiles and complex-language shaping/fallback remain outside the calibrated fixed 150-percent profile; unsupported glyphs/shaping must continue to report unknown."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "core-unit",
          "api",
          "ui-fixture"
        ],
        "proves": "Title extraction, missing/multiple/duplicate and fixed character checks exist; duplicate fixture and UI title filter/inspector are tested."
      },
      {
        "verification_run_ids": [
          "advanced-batch"
        ],
        "proves": "Absent/empty/multiple/duplicate and configurable character/pixel thresholds are tested. An openly licensed font profile returns unknown for unsupported shaping/glyphs; reference comparison records measured differences."
      },
      {
        "verification_run_ids": [
          "core-unit",
          "native-text-width",
          "reference-fields",
          "resource-offline-batch"
        ],
        "proves": "The fixed public-font 150-percent glyph-quantization profile is tested through the actual offline Rust API against ordinary original CLI title20px/description14px CSV observations: all 58 present measurements match, including empty elements and multiple positions; two absent-element zero placeholders are excluded. This is a bounded measurement comparison, not issue-filter membership or whole-feature parity."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Meta-Descriptions und Pixelbreiten** — Teilweise geprüft; `metadata.descriptions`, P1.

  Orientierung: Weitere Schrift-/DPI-Profile und komplexe Sprachformung getrennt kalibrieren; unbekannte Glyphen nicht raten.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [advanced-batch](verification/advanced-batch.json) (PASS), [native-text-width](parity/native-text-width-comparison.json) (PASS_BOUNDED_NATIVE_PROFILE), [reference-fields](parity/reference-comparison.json) (COMPARISON_EXECUTED), [resource-offline-batch](verification/resource-offline-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "metadata.descriptions",
    "reference_name": "Description extraction and audits",
    "status": "partial",
    "acceptance_test": "Test absent, duplicate, multiple and length/width limits without assuming a fixed search-engine display width.",
    "remaining_acceptance": [
      "Representative other font/DPI display profiles and complex-language shaping/fallback remain outside the calibrated fixed 150-percent profile; unsupported glyphs/shaping must continue to report unknown."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "core-fixtures"
        ],
        "proves": "Description extraction and missing/multiple/duplicate/fixed-length issue code are implemented; extraction and duplicate audit source are exercised."
      },
      {
        "verification_run_ids": [
          "advanced-batch"
        ],
        "proves": "Configurable length and public-profile width thresholds are tested; unknown widths are excluded from filters and disabled thresholds produce no invented finding."
      },
      {
        "verification_run_ids": [
          "core-unit",
          "native-text-width",
          "reference-fields",
          "resource-offline-batch"
        ],
        "proves": "The fixed public-font 150-percent glyph-quantization profile is tested through the actual offline Rust API against ordinary original CLI title20px/description14px CSV observations: all 58 present measurements match, including empty elements and multiple positions; two absent-element zero placeholders are excluded. This is a bounded measurement comparison, not issue-filter membership or whole-feature parity."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Inhaltsbereich per CSS auswählen** — Teilweise geprüft; `metadata.area`, P1.

  Orientierung: Native Roh-/DOM-Textbindung ist geprüft; Offline-Neuauswahl per geändertem CSS braucht noch erhaltenes HTML/DOM und eigene Artefaktbudgets.

  Beweisregister: [content-capture-core](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-native-chrome](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-api-cli](verification/content-capture-e2e.json) (PASS), [ui-content-fixture](../web/verification/ui-content-fixture-results.json) (PASS), [ui-content-live](../web/verification/ui-content-live-results.json) (PASS), [content-capture-batch](verification/content-capture-batch.json) (PASS), [content-capture-ci](verification/content-capture-ci.json) (PASS), [content-capture-installed](verification/content-capture-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "metadata.area",
    "reference_name": "Configurable content area",
    "status": "partial",
    "acceptance_test": "Exclude shared navigation/footer and include a chosen selector; prove selected text feeds analysis.",
    "remaining_acceptance": [
      "Re-select changed CSS include/exclude content areas after a crawl with zero website requests using retained full raw HTML/DOM artifacts and explicit artifact/retention budgets. Saved selected text and an HTML hash alone cannot replay selection; B8 does not retain full HTML/DOM.",
      "Broader source/layout profiles (computed visible text, iframe/shadow DOM, PDF/article selection) and complete-site/large-corpus analysis require separate versioned acceptance; own ancestor/node-boundary profile is not reference content-area equivalence."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [],
        "proves": "No tracked/exposed native text-capture integration is part of B7. The separately authored B8 pure capture source and fixtures are intentionally excluded from this batch and are not implementation proof."
      },
      {
        "verification_run_ids": [
          "content-capture-core",
          "content-capture-native-chrome",
          "content-capture-api-cli",
          "ui-content-fixture",
          "ui-content-live",
          "content-capture-batch"
        ],
        "proves": "Opt-in native CSS-selected text excludes shared nav/footer and respects selected/nested/excluded/template/document-order/Unicode boundaries. Capture disabled by default keeps legacy JSON absent; strict invalid CSS/limits reject before HTTP. Real raw and actual Chromium DOM sources bind independent text/language/UTF-8-representation/entity hashes, requested/final URL, timestamp/config/profile, completeness and committed generation. SQLite pause/crash/rollback and selected recrawl retain inherited captures while failed fresh/pending text is never old-success fallback. Eight actual API/CLI groups feed saved selected text to offline analysis with zero website requests. UI fixture evidence is isolated; actual UI evidence is included only when its fresh file exists. Full HTML/DOM and offline CSS re-selection are not implemented."
      },
      {
        "verification_run_ids": [
          "content-capture-ci",
          "content-capture-installed"
        ],
        "proves": "Exact-source B8 commit667bc97 passes four CI jobs (Windows/Linux, renderer, Linux container) and its immutable native Windows release runs the bounded raw/DOM saved-text contracts with19 MCP tools while preserving seven jobs/eleven original JSON bytes after restart. This does not prove whole-site analysis, saved full-HTML CSS reselection or subsequent B9 source changes."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Wörter und Textanteil zählen** — Teilweise geprüft; `metadata.wordcount`, P1.

  Orientierung: Versteckte Inhalte, Absätze und Text-/Code-Verhältnis explizit prüfen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [content-capture-core](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-native-chrome](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-api-cli](verification/content-capture-e2e.json) (PASS), [ui-content-fixture](../web/verification/ui-content-fixture-results.json) (PASS), [ui-content-live](../web/verification/ui-content-live-results.json) (PASS), [content-capture-batch](verification/content-capture-batch.json) (PASS), [content-capture-ci](verification/content-capture-ci.json) (PASS), [content-capture-installed](verification/content-capture-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "metadata.wordcount",
    "reference_name": "Text count and ratio",
    "status": "partial",
    "acceptance_test": "Use fixture paragraphs and hidden/script content to prove documented tokenization and text/code calculations.",
    "remaining_acceptance": [
      "Add versioned text/code ratio with HTML-representation denominator, dedicated paragraph fixtures and display/filter provenance distinguishing legacy whitespace word_count from selected Unicode analysis tokens.",
      "Computed CSS visibility/hidden-content exclusion is not observed by native source selection; support it only under a separate actual browser profile with controlled visibility fixtures."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit"
        ],
        "proves": "Normalized body whitespace tokens exclude script/style/noscript/template ancestors; the extraction fixture proves a five-word result despite script text."
      },
      {
        "verification_run_ids": [
          "content-capture-core",
          "content-capture-native-chrome",
          "content-capture-api-cli",
          "ui-content-fixture",
          "ui-content-live",
          "content-capture-batch"
        ],
        "proves": "B8 persists exact selected text and versioned whitespace/Unicode profile, with source-specific language, hash and completeness. Hidden/CSS-invisible text remains source DOM text unless explicitly excluded; no computed visibility is inferred. Downstream own Unicode token counts are reproducible over this selected saved sample; legacy page.word_count remains a separately labelled extraction metric."
      },
      {
        "verification_run_ids": [
          "content-capture-ci",
          "content-capture-installed"
        ],
        "proves": "Exact-source B8 commit667bc97 passes four CI jobs (Windows/Linux, renderer, Linux container) and its immutable native Windows release runs the bounded raw/DOM saved-text contracts with19 MCP tools while preserving seven jobs/eleven original JSON bytes after restart. This does not prove whole-site analysis, saved full-HTML CSS reselection or subsequent B9 source changes."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Lesbarkeit bewerten** — Teilweise geprüft; `metadata.readability`, P1.

  Orientierung: Gespeicherte Roh-/DOM-Quelle ist angebunden; breitere Sprach-/Silben-/Klassifikationsprofile und größere Corpora weiter prüfen.

  Beweisregister: [content-analysis-core](../crates/core/src/content_analysis.rs) (PASS), [content-analysis-api-cli](verification/content-analysis-e2e.json) (PASS), [original-content-observations](reference-content-observations.json) (BOUNDED_ORIGINAL_REFERENCE_OBSERVED), [native-policy-ci](verification/native-policy-ci.json) (PASS), [native-policy-installed](verification/native-policy-installed.json) (PASS), [content-capture-core](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-native-chrome](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-api-cli](verification/content-capture-e2e.json) (PASS), [ui-content-fixture](../web/verification/ui-content-fixture-results.json) (PASS), [ui-content-live](../web/verification/ui-content-live-results.json) (PASS), [content-capture-batch](verification/content-capture-batch.json) (PASS), [content-capture-ci](verification/content-capture-ci.json) (PASS), [content-capture-installed](verification/content-capture-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "metadata.readability",
    "reference_name": "Readability scoring",
    "status": "partial",
    "acceptance_test": "Return score and formula/language version; classify unsupported language explicitly.",
    "remaining_acceptance": [
      "Calibrate broader sentence/syllable/language and classification profiles against positive/control cases. The own declared EN/DE heuristic does not prove detected-language or original readability equivalence.",
      "Finite selected saved corpus only; complete-site/bounded large-corpus scheduling and additional language/shaping/source profiles need separate acceptance."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "content-analysis-core",
          "content-analysis-api-cli",
          "original-content-observations"
        ],
        "proves": "Offline supplied text returns exact documented EN/DE Flesch formulas under an own declared-language/vowel-group/terminal-e heuristic profile, words/sentences/syllables and nullable scores/reasons for unsupported language/script, short or incomplete text. Config/input/token hashes and API/CLI zero-HTTP agreement are tested. Original22-case scores/counts are separate bounded observations and differ from this declared heuristic."
      },
      {
        "verification_run_ids": [
          "native-policy-ci",
          "native-policy-installed"
        ],
        "proves": "Exact B7 source915e175 has four successful Windows/Linux/Chromium/Linux Docker CI jobs and an installed native Windows immutable release. Existing seven jobs and eleven original JSON files remain byte-identical after restart; no user-data fixture requests. This confirms the already documented B7 slice only; ongoing B8 capture/corpus changes and full reference parity are not attested."
      },
      {
        "verification_run_ids": [
          "content-capture-core",
          "content-capture-native-chrome",
          "content-capture-api-cli",
          "ui-content-fixture",
          "ui-content-live",
          "content-capture-batch"
        ],
        "proves": "Actual saved raw/DOM text carries its own root-declared language and completeness through finite exact-requested-URL cohort analysis. Supported own EN/DE heuristic formulas retain nullable short/incomplete/unsupported results, source revisions and mixed retained/refreshed basis. Raw text never borrows effective rendered page language. Parameter changes and saved analysis make zero website requests. This does not calibrate the original sentence/syllable/classification profile."
      },
      {
        "verification_run_ids": [
          "content-capture-ci",
          "content-capture-installed"
        ],
        "proves": "Exact-source B8 commit667bc97 passes four CI jobs (Windows/Linux, renderer, Linux container) and its immutable native Windows release runs the bounded raw/DOM saved-text contracts with19 MCP tools while preserving seven jobs/eleven original JSON bytes after restart. This does not prove whole-site analysis, saved full-HTML CSS reselection or subsequent B9 source changes."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Exakt gleiche Inhalte gruppieren** — Teilweise geprüft; `metadata.exact`, P1.

  Orientierung: Identische und um ein Zeichen veränderte Dokumente mit Digest-Herkunft gemeinsam prüfen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [content-capture-core](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-native-chrome](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-api-cli](verification/content-capture-e2e.json) (PASS), [ui-content-fixture](../web/verification/ui-content-fixture-results.json) (PASS), [ui-content-live](../web/verification/ui-content-live-results.json) (PASS), [content-capture-batch](verification/content-capture-batch.json) (PASS), [content-capture-ci](verification/content-capture-ci.json) (PASS), [content-capture-installed](verification/content-capture-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "metadata.exact",
    "reference_name": "Exact duplicate detection",
    "status": "partial",
    "acceptance_test": "Group identical content but separate a one-character change; record the digest method.",
    "remaining_acceptance": [
      "Add a dedicated persisted selected-text exact-duplicate grouping/export fixture covering identical versus one-character change, empty/incomplete eligibility, source profile changes and all group members; current pair byte identity and separate legacy digest grouping are not that full workflow."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "core-fixtures"
        ],
        "proves": "SHA-256 hashes normalized body text; audit code groups equal captured hashes and records related URLs."
      },
      {
        "verification_run_ids": [
          "content-capture-core",
          "content-capture-native-chrome",
          "content-capture-api-cli",
          "ui-content-fixture",
          "ui-content-live",
          "content-capture-batch"
        ],
        "proves": "Selected text SHA-256 and separate supplied UTF-8 HTML/entity hashes persist with exact source/revision/config identity; raw and DOM representations never share a fabricated digest. Controlled identical selected samples remain distinct requested identities and byte identity is separate from token-shingle similarity. Legacy content-hash grouping remains separate from saved-corpus analysis."
      },
      {
        "verification_run_ids": [
          "content-capture-ci",
          "content-capture-installed"
        ],
        "proves": "Exact-source B8 commit667bc97 passes four CI jobs (Windows/Linux, renderer, Linux container) and its immutable native Windows release runs the bounded raw/DOM saved-text contracts with19 MCP tools while preserving seven jobs/eleven original JSON bytes after restart. This does not prove whole-site analysis, saved full-HTML CSS reselection or subsequent B9 source changes."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Ähnliche Inhalte vergleichen** — Teilweise geprüft; `metadata.near`, P1.

  Orientierung: Ausgewählte gespeicherte Quellen und Aktualität sind angebunden; vollständige Website-/große Corpus-Analyse und Referenz-MinHash-Prädikate bleiben offen.

  Beweisregister: [content-analysis-core](../crates/core/src/content_analysis.rs) (PASS), [content-analysis-api-cli](verification/content-analysis-e2e.json) (PASS), [original-content-observations](reference-content-observations.json) (BOUNDED_ORIGINAL_REFERENCE_OBSERVED), [native-policy-ci](verification/native-policy-ci.json) (PASS), [native-policy-installed](verification/native-policy-installed.json) (PASS), [content-capture-core](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-native-chrome](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-api-cli](verification/content-capture-e2e.json) (PASS), [ui-content-fixture](../web/verification/ui-content-fixture-results.json) (PASS), [ui-content-live](../web/verification/ui-content-live-results.json) (PASS), [content-capture-batch](verification/content-capture-batch.json) (PASS), [content-capture-ci](verification/content-capture-ci.json) (PASS), [content-capture-installed](verification/content-capture-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "metadata.near",
    "reference_name": "Near duplicate detection",
    "status": "partial",
    "acceptance_test": "Score a controlled changed paragraph above/below a configured threshold; preserve matched URL and comparison evidence.",
    "remaining_acceptance": [
      "Complete-site and large-corpus duplicate scheduling/indexing remain outside the explicit1..128 selected-source cohort. Preserve corpus, pair/sample budgets, unobserved pages and mixed freshness without silent whole-site claims.",
      "Original MinHash/candidate selection/content normalization/threshold and combined filter membership remain NOT_PROVEN; retained original near checker was disabled."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "content-analysis-core",
          "content-analysis-api-cli",
          "original-content-observations"
        ],
        "proves": "Offline exact unique-token-shingle Jaccard exposes controlled changed-paragraph pairs, intersections/unions/shared examples, versioned parameters and inclusive thresholds. Missing/short/incomplete samples retain nullable classification and explicit corpus scope/truncation. Byte identity remains separate. Original near checking was disabled and contributes no near-algorithm parity proof."
      },
      {
        "verification_run_ids": [
          "native-policy-ci",
          "native-policy-installed"
        ],
        "proves": "Exact B7 source915e175 has four successful Windows/Linux/Chromium/Linux Docker CI jobs and an installed native Windows immutable release. Existing seven jobs and eleven original JSON files remain byte-identical after restart; no user-data fixture requests. This confirms the already documented B7 slice only; ongoing B8 capture/corpus changes and full reference parity are not attested."
      },
      {
        "verification_run_ids": [
          "content-capture-core",
          "content-capture-native-chrome",
          "content-capture-api-cli",
          "ui-content-fixture",
          "ui-content-live",
          "content-capture-batch"
        ],
        "proves": "Opt-in persisted text feeds exact finite selected requested-URL/source cohorts with URL→analysis-id provenance, source revision, complete/empty/missing/incomplete/error states, eligibility exclusions and inherited/refreshed/pending basis. Controlled duplicate pair score1 remains known while failed selected500 replacement is incomplete and near_duplicate stays null; unselected/parent captures remain unchanged. Own exact shingle-set Jaccard is not a reference MinHash/threshold proof."
      },
      {
        "verification_run_ids": [
          "content-capture-ci",
          "content-capture-installed"
        ],
        "proves": "Exact-source B8 commit667bc97 passes four CI jobs (Windows/Linux, renderer, Linux container) and its immutable native Windows release runs the bounded raw/DOM saved-text contracts with19 MCP tools while preserving seven jobs/eleven original JSON bytes after restart. This does not prove whole-site analysis, saved full-HTML CSS reselection or subsequent B9 source changes."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Überlappende N-Gramme auswerten** — Teilweise geprüft; `metadata.ngrams`, P1.

  Orientierung: Gespeicherte Crawl-Texte und Quellrevisionen sind angebunden; große Corpus-Aggregation und vollständige spezielle N-Gramm-Exporte weiter prüfen.

  Beweisregister: [content-analysis-core](../crates/core/src/content_analysis.rs) (PASS), [content-analysis-api-cli](verification/content-analysis-e2e.json) (PASS), [native-policy-ci](verification/native-policy-ci.json) (PASS), [native-policy-installed](verification/native-policy-installed.json) (PASS), [content-capture-core](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-native-chrome](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-api-cli](verification/content-capture-e2e.json) (PASS), [ui-content-fixture](../web/verification/ui-content-fixture-results.json) (PASS), [ui-content-live](../web/verification/ui-content-live-results.json) (PASS), [content-capture-batch](verification/content-capture-batch.json) (PASS), [content-capture-ci](verification/content-capture-ci.json) (PASS), [content-capture-installed](verification/content-capture-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "metadata.ngrams",
    "reference_name": "N-gram frequency",
    "status": "partial",
    "acceptance_test": "Count overlapping one-, two- and three-word sequences with documented stopword and case policies.",
    "remaining_acceptance": [
      "Prove complete-site/large-corpus aggregation and downloadable specialized1/2/3-gram reports with source revisions and full-versus-sampled position evidence; B8 analysis remains a bounded explicit saved cohort."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "content-analysis-core",
          "content-analysis-api-cli"
        ],
        "proves": "Offline explicitly supplied text has deterministic overlapping1/2/3-gram counts/positions with versioned Unicode/case rules, stopword-containing windows excluded without false adjacency, exact count/hash evidence and bounded sampling/resource errors. Native API and CLI agree with zero website requests."
      },
      {
        "verification_run_ids": [
          "native-policy-ci",
          "native-policy-installed"
        ],
        "proves": "Exact B7 source915e175 has four successful Windows/Linux/Chromium/Linux Docker CI jobs and an installed native Windows immutable release. Existing seven jobs and eleven original JSON files remain byte-identical after restart; no user-data fixture requests. This confirms the already documented B7 slice only; ongoing B8 capture/corpus changes and full reference parity are not attested."
      },
      {
        "verification_run_ids": [
          "content-capture-core",
          "content-capture-native-chrome",
          "content-capture-api-cli",
          "ui-content-fixture",
          "ui-content-live",
          "content-capture-batch"
        ],
        "proves": "Persisted selected raw/DOM text now feeds the own overlapping1/2/3-gram engine through offline saved-source API/CLI and explicit cohort provenance. Case/stopword/config edits bind hashes and current source revision; duplicate source identities are rejected, 1..128 selection is explicit, and max input/pair/output budgets fail without silent first-N site coverage. Browser fixture controls show ngram paging/display; actual UI is a separately retained gate."
      },
      {
        "verification_run_ids": [
          "content-capture-ci",
          "content-capture-installed"
        ],
        "proves": "Exact-source B8 commit667bc97 passes four CI jobs (Windows/Linux, renderer, Linux container) and its immutable native Windows release runs the bounded raw/DOM saved-text contracts with19 MCP tools while preserving seven jobs/eleven original JSON bytes after restart. This does not prove whole-site analysis, saved full-HTML CSS reselection or subsequent B9 source changes."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Noch offen

- [ ] **Rechtschreibung und Grammatik** — Offen; `metadata.spelling`, P1.

  Orientierung: Mehrsprachige Wörterbücher, Ausnahmen und genaue Fundstellen auf gespeichertem Text prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "metadata.spelling",
    "reference_name": "Spelling and grammar",
    "status": "planned",
    "acceptance_test": "Test multilingual dictionaries, ignored words/rules and exact issue locations against saved text.",
    "remaining_acceptance": [
      "Test multilingual dictionaries, ignored words/rules and exact issue locations against saved text."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **Überschriften und Reihenfolge** — Eigene Abnahme erfüllt; `metadata.headings`, P1.

  Orientierung: Der eigene Vertrag einschließlich begrenzter H1-/H2-Referenz-Fixtures ist geprüft.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [core-browser](../crates/core/tests/crawl_fixtures.rs) (PASS), [advanced-batch](verification/advanced-batch.json) (PASS), [dom-robots-batch](verification/dom-robots-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "metadata.headings",
    "reference_name": "H1/H2 and meta keyword evidence",
    "status": "implemented",
    "acceptance_test": "Record heading order and duplicate/missing/multiple conditions plus meta keyword values as observational fields.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "core-browser"
        ],
        "proves": "H1/H2 lists and H1 missing/multiple findings exist. Raw and JavaScript-changed H1 are separately retained in the live browser fixture."
      },
      {
        "verification_run_ids": [
          "advanced-batch"
        ],
        "proves": "Missing/multiple/duplicate/long H1/H2 cases use configurable bounds and captured HTML evidence."
      },
      {
        "verification_run_ids": [
          "dom-robots-batch"
        ],
        "proves": "Bounded raw/rendered DOM facts preserve literal heading order, raw text, inline image-alt display profile, locations and meta keywords; duplicate/missing/multiple/nonsequential/alt rules have fixtures. Corrected native H1/H2 memberships match 48 retained reference observations across 24 authored HTML cases."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **SERP-Vorschau offline bearbeiten** — Eigene Abnahme erfüllt; `metadata.serp_preview`, P1.

  Orientierung: Die eigene Vorschau mit Versionierung, Import und Race-Prüfungen ist geprüft.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [offline-serp](../web/verification/ui-serp-live-results.json) (PASS), [serp-state-races](../web/verification/serp-state-race.test.cjs) (PASS), [resource-offline-batch](verification/resource-offline-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "metadata.serp_preview",
    "reference_name": "SERP preview",
    "status": "implemented",
    "acceptance_test": "Show editable title/description preview; identify it as an approximation and export edited values.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "offline-serp",
          "serp-state-races",
          "resource-offline-batch"
        ],
        "proves": "Editable title/description previews display character counts and supported-profile approximate widths, label the preview as an approximation, retain unknown widths explicitly, and export edited input as safe CSV or exact JSON. Actual desktop/mobile browser checks and stale-response protection are verified; this does not predict a live search-engine result."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Links und Website-Architektur

11 Einträge: 3 eigene Abnahmen erfüllt, 5 teilweise geprüft, 3 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **Minimale Klicktiefe bestimmen** — Teilweise geprüft; `links.depth`, P2.

  Orientierung: Mehrere Wege und spät entdeckte kürzere Wege in einer vollständigen Graph-Fixture prüfen.

  Beweisregister: [analysis](analysis-acceptance.md) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "links.depth",
    "reference_name": "Shortest crawl depth",
    "status": "partial",
    "acceptance_test": "On a graph with multiple routes, compute minimum click depth deterministically after complete discovery.",
    "remaining_acceptance": [
      "Add a dedicated multi-route/long-first discovery graph asserting all minimum click depths."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "analysis"
        ],
        "proves": "Graph analysis deterministically computes shortest observed anchor distances by BFS, separately from stored crawl depth; fixture checks depth and input-order invariance."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Nachvollziehbaren Link-Score berechnen** — Teilweise geprüft; `links.score`, P2.

  Orientierung: Algorithmus und Parameter mit Versionskennung in gespeicherter Graph-Evidenz ergänzen.

  Beweisregister: [analysis](analysis-acceptance.md) (PASS), [api](verification/api-e2e.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "links.score",
    "reference_name": "Internal authority scoring",
    "status": "partial",
    "acceptance_test": "Run a documented independent PageRank-style method on a known graph; save algorithm and parameter version.",
    "remaining_acceptance": [
      "Add an explicit algorithm/parameter version identifier to persisted graph evidence."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "analysis",
          "api"
        ],
        "proves": "Independent PageRank uses damping .85, L1 tolerance 1e-12, max200 iterations, uniform dangling redistribution and max-rank normalization100. Tests prove conserved rank, dangling/nofollow behavior and deterministic order."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Orphan-Kandidaten aus Quellen abgleichen** — Teilweise geprüft; `links.orphans`, P2.

  Orientierung: Sitemap-, GA4- und GSC-Herkunft gemeinsam abgleichen; unvollständigen Crawl sichtbar begrenzen.

  Beweisregister: [analysis](analysis-acceptance.md) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "links.orphans",
    "reference_name": "Orphan URL discovery",
    "status": "partial",
    "acceptance_test": "Merge sitemap/analytics/search data; distinguish absent internal links from incomplete crawl coverage.",
    "remaining_acceptance": [
      "Join sitemap/GA4/GSC URL provenance and prove whole supplied-source reconciliation while preserving incomplete-crawl boundaries."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "analysis",
          "core-fixtures"
        ],
        "proves": "Zero observed inlinks identifies usable non-seed HTML candidates only; pending/limited/cancelled/truncated coverage is explicit. Sitemap-only pages are discoverable."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Nicht regulär crawlbare Navigation beobachten** — Teilweise geprüft; `links.uncrawlable`, P2.

  Orientierung: Onclick- und weitere Client-Muster einzeln prüfen, ohne Navigation zu erfinden.

  Beweisregister: [dom-robots-batch](verification/dom-robots-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "links.uncrawlable",
    "reference_name": "Nonstandard link evidence",
    "status": "partial",
    "acceptance_test": "Record onclick, javascript URLs and href on non-anchor elements without assuming a valid navigable target.",
    "remaining_acceptance": [
      "Plain onclick-only elements and the broader client navigation patterns need dedicated observation/discovery acceptance; no JavaScript navigation inference is claimed."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "dom-robots-batch"
        ],
        "proves": "DOM navigation candidates preserve non-anchor href, javascript schemes, router/onclick attributes and reason without guessing a navigable target."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Crawl-Graph interaktiv untersuchen** — Teilweise geprüft; `links.crawl_graph`, P2.

  Orientierung: Kürzeste-Wege-Hierarchie, Fokus, Aufklappen und Status-/Indexierbarkeitsfarben prüfen.

  Beweisregister: [analysis](analysis-acceptance.md) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "links.crawl_graph",
    "reference_name": "Crawl tree and graph views",
    "status": "partial",
    "acceptance_test": "Render shortest-path hierarchy, focus/expand and status/indexability coloring from fixture data.",
    "remaining_acceptance": [
      "Add shortest-path tree hierarchy, focus/expand controls and full status/indexability color acceptance."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "analysis",
          "ui-fixture",
          "ui-live"
        ],
        "proves": "Observed anchor nodes/edges/degrees are API-proved; browser displays a bounded graph and real local-crawl graph with explicit scope."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Noch offen

- [ ] **Verzeichnis-Hierarchie anzeigen** — Offen; `links.directory_graph`, P2.

  Orientierung: Künstliche Gruppierungspfade von tatsächlich abgerufenen URLs unterscheiden.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "links.directory_graph",
    "reference_name": "Directory architecture views",
    "status": "planned",
    "acceptance_test": "Build directory hierarchy including synthetic grouping paths and label paths that are not fetched URLs.",
    "remaining_acceptance": [
      "Build directory hierarchy including synthetic grouping paths and label paths that are not fetched URLs."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Große Graphen räumlich anzeigen** — Offen; `links.graphs_3d`, P2.

  Orientierung: Begrenzung und Fokus großer Graphen sichtbar machen, statt Knoten still wegzulassen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "links.graphs_3d",
    "reference_name": "3D graph views",
    "status": "planned",
    "acceptance_test": "Display a bounded large graph and expose truncation/focus rather than silently omitting nodes.",
    "remaining_acceptance": [
      "Display a bounded large graph and expose truncation/focus rather than silently omitting nodes."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Logische Segmente bilden** — Offen; `links.segments`, P2.

  Orientierung: Geordnete Feld-/Regex-Segmente auf Zählungen, Issues, Diagramme und Exporte anwenden.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "links.segments",
    "reference_name": "Configurable segmentation",
    "status": "planned",
    "acceptance_test": "Apply ordered field/regex segments and filter counts, issues, charts and exports by segment.",
    "remaining_acceptance": [
      "Apply ordered field/regex segments and filter counts, issues, charts and exports by segment."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **Jedes Linkvorkommen erhalten** — Eigene Abnahme erfüllt; `links.occurrences`, P2.

  Orientierung: Linkvorkommen und Ressourcentypen werden im eigenen Vertrag getrennt erhalten.

  Beweisregister: [analysis](analysis-acceptance.md) (PASS), [core-unit](../crates/core/src) (PASS), [api](verification/api-e2e.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "links.occurrences",
    "reference_name": "Inlinks and outlinks",
    "status": "implemented",
    "acceptance_test": "For repeated A-to-B anchors, expose total occurrences and unique sources separately with anchor text and rel.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "analysis",
          "core-unit",
          "api"
        ],
        "proves": "Repeated A-to-B anchors retain occurrence counts separately from unique source/target degrees. Original LinkRecord values preserve anchor text/rel and all-occurrence links export is API-tested."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Linkpositionen und DOM-Pfade** — Eigene Abnahme erfüllt; `links.positions`, P2.

  Orientierung: Der eigene Positionsvertrag mit Dokumentreihenfolge ist geprüft.

  Beweisregister: [dom-robots-batch](verification/dom-robots-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "links.positions",
    "reference_name": "Link position and path",
    "status": "implemented",
    "acceptance_test": "Classify nav/header/footer/content links and custom positions; retain selector/path per occurrence.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "dom-robots-batch"
        ],
        "proves": "Every retained link occurrence carries raw/resolved href, fragment, bounded DOM path/index/ancestors and first-match custom CSS position or nearest semantic default. Fixture and browser exports retain provenance and duplicates."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Fragmentziele und Bookmarks prüfen** — Eigene Abnahme erfüllt; `links.fragments`, P2.

  Orientierung: Der eigene opt-in Vertrag einschließlich leerer und unbekannter Ziele ist geprüft.

  Beweisregister: [dom-robots-batch](verification/dom-robots-batch.json) (PASS), [list-import-batch](verification/list-import-batch.json) (PASS), [core-unit](../crates/core/src) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "links.fragments",
    "reference_name": "Bookmark validation",
    "status": "implemented",
    "acceptance_test": "Verify decoded fragment destinations against IDs/names; distinguish a missing fragment from a failed page.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "dom-robots-batch"
        ],
        "proves": "Opt-in captured-document validation checks raw fragment then UTF-8 decoded fallback against ID and legacy anchor name; raw/decoded/top/missing/unknown/unsupported outcomes are distinct. No validation request is made; failed, incomplete, legacy and status-unknown rendered targets remain unknown."
      },
      {
        "verification_run_ids": [
          "list-import-batch",
          "core-unit"
        ],
        "proves": "Lazy per-complete-document borrowed ID/name indexes replace repeated whole-DOM scans; min DOM index wins imported ties and raw/decoded/ID/name/unknown semantics remain tested. 20,000 targets and 20,000 links are exercised; this is not whole-crawl capacity proof."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Canonical, Sprachen und alternative Seiten

7 Einträge: 1 eigene Abnahmen erfüllt, 6 teilweise geprüft, 0 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **Canonical-Ketten und Zielzustände prüfen** — Teilweise geprüft; `relationships.canonical_checks`, P2.

  Orientierung: Ungültige Deklarationen, Ketten, Schleifen und tatsächliche Zielzustände vollständig prüfen.

  Beweisregister: [analysis](analysis-acceptance.md) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "relationships.canonical_checks",
    "reference_name": "Canonical validation",
    "status": "partial",
    "acceptance_test": "Flag malformed, conflicting, redirecting, blocked and non-indexable targets and canonical chains.",
    "remaining_acceptance": [
      "Preserve malformed declaration evidence and test full chain/cycle/target-state acceptance through actual captured HTML."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "analysis",
          "core-fixtures"
        ],
        "proves": "Observed redirect/non200/blocked/noindex/canonicalized/unobserved/conflicting targets and canonical cycles have distinct findings. Unknown targets never become fabricated HTTP failures."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Pagination-Beziehungen prüfen** — Teilweise geprüft; `relationships.pagination`, P2.

  Orientierung: Lange Ketten, Schleifen und konfigurierbare Entdeckung ergänzen.

  Beweisregister: [advanced-batch](verification/advanced-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "relationships.pagination",
    "reference_name": "Pagination relationship validation",
    "status": "partial",
    "acceptance_test": "Extract HTML/HTTP next and prev; test broken links, absent returns and inconsistent chains.",
    "remaining_acceptance": [
      "Dedicated long-chain/cycle boundary fixtures and configurable relationship-discovery policies remain."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "advanced-batch"
        ],
        "proves": "HTML/HTTP next/prev extraction, source preservation, malformed/conflicting/self declarations, target status/robots/canonicalization and return/anchor diagnostics are tested and displayed."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Hreflang aus mehreren Quellen erfassen** — Teilweise geprüft; `relationships.hreflang`, P2.

  Orientierung: XML-Sitemap-Alternativen mit HTML-/HTTP-Deklarationen zusammenführen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [analysis](analysis-acceptance.md) (PASS), [advanced-batch](verification/advanced-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "relationships.hreflang",
    "reference_name": "Hreflang extraction",
    "status": "partial",
    "acceptance_test": "Read HTML, HTTP Link and XML sitemap alternates with language/region and x-default.",
    "remaining_acceptance": [
      "XML sitemap alternate extraction and source joining remain."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "analysis"
        ],
        "proves": "HTML alternate hreflang language strings and resolved URLs are retained; language labels normalize for conflict checks."
      },
      {
        "verification_run_ids": [
          "advanced-batch"
        ],
        "proves": "HTML and HTTP declarations preserve language/region/script/x-default and sources. Tests cover registered profile, self/return/cluster/duplicate conflicts, bad captured targets and unknown/incomplete evidence."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Hreflang-Cluster und Rückverweise prüfen** — Teilweise geprüft; `relationships.hreflang_checks`, P2.

  Orientierung: Sprach-/Regioncodes und identische doppelte Deklarationen vollständig prüfen.

  Beweisregister: [analysis](analysis-acceptance.md) (PASS), [core-unit](../crates/core/src) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "relationships.hreflang_checks",
    "reference_name": "Hreflang reciprocity and validity",
    "status": "partial",
    "acceptance_test": "Test unsupported codes, missing returns/self-reference, duplicates and non-indexable targets.",
    "remaining_acceptance": [
      "Validate supported language/region codes and preserve duplicate-identical declaration findings."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "analysis",
          "core-unit"
        ],
        "proves": "Self-reference, language-target conflicts, fetched complete target reciprocity and noindex/canonicalized targets are tested. Core tests avoid false return absence on 500/truncated targets."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **AMP-Paare und Validierung** — Teilweise geprüft; `relationships.amp`, P2.

  Orientierung: Rückverweise und einen fest versionierten öffentlichen AMP-Validator ergänzen.

  Beweisregister: [advanced-batch](verification/advanced-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "relationships.amp",
    "reference_name": "AMP extraction and validation",
    "status": "partial",
    "acceptance_test": "Detect amphtml/canonical reciprocity and report versioned AMP validation findings.",
    "remaining_acceptance": [
      "AMP/canonical return validation and a version-pinned AMP validator remain."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "advanced-batch"
        ],
        "proves": "HTML/HTTP amphtml relationships retain target and declaration provenance."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Mobile Alternativen prüfen** — Teilweise geprüft; `relationships.mobile`, P2.

  Orientierung: Gegenseitige Verweise und tatsächliche Zielzustände in eigenen Fixtures prüfen.

  Beweisregister: [advanced-batch](verification/advanced-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "relationships.mobile",
    "reference_name": "Mobile alternate relationships",
    "status": "partial",
    "acceptance_test": "Resolve alternate media relationships and surface missing or non-indexable target evidence.",
    "remaining_acceptance": [
      "Dedicated alternate-mobile reciprocity and target-state fixture coverage remains."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "advanced-batch"
        ],
        "proves": "HTML/HTTP media alternates preserve media and target provenance."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **Canonical-Deklarationen erhalten** — Eigene Abnahme erfüllt; `relationships.canonical`, P2.

  Orientierung: Der eigene HTML-/HTTP-Deklarationsvertrag ist geprüft.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [analysis](analysis-acceptance.md) (PASS), [advanced-batch](verification/advanced-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "relationships.canonical",
    "reference_name": "Canonical extraction",
    "status": "implemented",
    "acceptance_test": "Extract HTML and HTTP Link canonical entries including relative, multiple and self-referencing values.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "analysis"
        ],
        "proves": "Relative HTML canonical links resolve against base URL; multiple/conflicting/self/target-cycle facts are analyzed."
      },
      {
        "verification_run_ids": [
          "advanced-batch"
        ],
        "proves": "HTML and multi-line HTTP Link canonical declarations retain distinct source/context and resolve relative/multiple/self values. Header targets use the response URL independently of HTML base. Complete unit and HTTP fixtures test context/quoting and mixed-source cases."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Bilder, Cookies und PDFs

6 Einträge: 1 eigene Abnahmen erfüllt, 5 teilweise geprüft, 0 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **Bildquellen und responsive Kandidaten** — Teilweise geprüft; `images.sources`, P2.

  Orientierung: Mehrseitige Vorkommen und tatsächlich gewähltes currentSrc mit Media-/Dichteauswahl prüfen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [dom-robots-batch](verification/dom-robots-batch.json) (PASS), [resource-discovery-api-cli](verification/resource-discovery-e2e.json) (PASS), [ui-resource-discovery-live](../web/verification/ui-resource-discovery-live-results.json) (PASS), [resource-discovery-batch](verification/resource-discovery-batch.json) (PASS), [resource-contract-matrix](verification/resource-contract-matrix.json) (PASS), [resource-contract-responses](verification/resource-contract-responses.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "images.sources",
    "reference_name": "Image candidates and occurrences",
    "status": "partial",
    "acceptance_test": "Record src/srcset candidates and every source page independently; preserve per-occurrence alt.",
    "remaining_acceptance": [
      "Explicit multi-page occurrence/alt acceptance remains. Actual browser-selected currentSrc, media and density selection remains a separate unproven observation profile."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "ui-fixture"
        ],
        "proves": "Each img src occurrence retains its source PageRecord, alt and declared width/height/loading; browser inspector exposes image evidence."
      },
      {
        "verification_run_ids": [
          "dom-robots-batch"
        ],
        "proves": "Raw/rendered responsive image observations retain img src/srcset/sizes and preceding picture sources, width/density/height descriptors, media/type eligibility, malformed candidates, per-occurrence alt and locations; eligible HTTP candidates use normal crawl policy."
      },
      {
        "batch": "B9-resource-discovery",
        "verification_run_ids": [
          "resource-discovery-api-cli",
          "ui-resource-discovery-live",
          "resource-discovery-batch"
        ],
        "proves": "Responsive image/source duplicates and local-fragment controls are preserved in the native occurrence ledger; same URL image/anchor grants remain independent while network transport coalesces.",
        "limits": "Cross-source-page alt/occurrence acceptance and actual browser currentSrc/media/density selection are not proven by native candidates.",
        "source_debug_sha256_by_run": {
          "resource-discovery-api-cli": "142c0a1af3d420de5ccb1e182bdd04d701f8087a5dc72b3758464aa5e3d65724",
          "ui-resource-discovery-live": "142c0a1af3d420de5ccb1e182bdd04d701f8087a5dc72b3758464aa5e3d65724"
        }
      },
      {
        "batch": "B10-resource-contract-v2",
        "verification_run_ids": [
          "resource-contract-matrix",
          "resource-contract-responses"
        ],
        "proves": "Independent declared resource occurrences, requested identity, actual response MIME and coalesced fetch behavior checked in both profiles; browser-selected currentSrc remains unproven.",
        "limits": "Bounded own native/local acceptance. Existing remaining acceptance and reference parity remain separately tracked; CI/release/installation pending."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Alt-Attribute unterscheiden** — Teilweise geprüft; `images.alt`, P2.

  Orientierung: Fehlendes, leeres und ausschließlich Whitespace enthaltendes Alt explizit prüfen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "images.alt",
    "reference_name": "Missing alt distinctions",
    "status": "partial",
    "acceptance_test": "Separate missing attribute from empty/whitespace alt using three deliberately different image elements.",
    "remaining_acceptance": [
      "Add the three-element absent/empty/whitespace fixture and report these distinctions explicitly."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "core-fixtures",
          "ui-fixture"
        ],
        "proves": "Option<String> distinguishes absent alt from present strings; missing-alt fixture findings and UI image-alt filter are tested."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Bildabmessungen und Größen** — Teilweise geprüft; `images.size`, P2.

  Orientierung: Gerenderte Maße, echte Transferbytes, Schwellen und zusätzliche Format-/Orientierungsfälle prüfen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [resource-fixtures](../crates/core/tests/resource_fixtures.rs) (PASS), [ui-resources](../web/verification/ui-resource-live-results.json) (PASS), [resource-offline-batch](verification/resource-offline-batch.json) (PASS), [resource-contract-matrix](verification/resource-contract-matrix.json) (PASS), [resource-contract-responses](verification/resource-contract-responses.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "images.size",
    "reference_name": "Image dimensions and size audits",
    "status": "partial",
    "acceptance_test": "Record intrinsic/rendered dimensions, transfer size and configurable thresholds without mislabeling unavailable data.",
    "remaining_acceptance": [
      "Rendered image dimensions and wire-transfer byte measurements remain unproven; encoded captured bytes are not wire size. Add configurable dimension/size threshold acceptance and broader raster-format/animation orientation fixtures."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit"
        ],
        "proves": "Declared HTML image dimensions are captured as observational strings; independently fetched resources have native byte evidence."
      },
      {
        "verification_run_ids": [
          "core-unit",
          "resource-fixtures",
          "ui-resources",
          "resource-offline-batch"
        ],
        "proves": "Fetched image resources expose dimensions from encoded raster headers and bounded SVG absolute lengths, encoded captured bytes, completeness/parse status and units. Actual PNG and fractional SVG measurements persist through SQLite/JSON, tables and CSV; viewBox-only SVG, malformed/truncated input and legacy missing observations stay explicit unknown/error states. HTML width/height and each alt occurrence remain separate from file dimensions; this is header inspection, not full pixel decoding."
      },
      {
        "batch": "B10-resource-contract-v2",
        "verification_run_ids": [
          "resource-contract-matrix",
          "resource-contract-responses"
        ],
        "proves": "Real valid PNG responses provide independently asserted dimensions/entity hashes under all resource switch products; no font/audio/SWF execution claim.",
        "limits": "Bounded own native/local acceptance. Existing remaining acceptance and reference parity remain separately tracked; CI/release/installation pending."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **PDF-Metadaten und Text erfassen** — Teilweise geprüft; `images.pdf`, P2.

  Orientierung: Schriftabdeckung, Form XObjects und gescannte PDFs/OCR getrennt unterstützen und prüfen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [resource-fixtures](../crates/core/tests/resource_fixtures.rs) (PASS), [ui-resources](../web/verification/ui-resource-live-results.json) (PASS), [resource-offline-batch](verification/resource-offline-batch.json) (PASS), [resource-contract-responses](verification/resource-contract-responses.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "images.pdf",
    "reference_name": "PDF metadata and text",
    "status": "partial",
    "acceptance_test": "Extract title, author, dates, pages and text from an owned fixture PDF; show unsupported encrypted files explicitly.",
    "remaining_acceptance": [
      "PDF text coverage remains bounded to supported font mappings and native text operators; unsupported fonts, Form XObjects, rendering/OCR and scanned-page readable text require separate support and acceptance."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "resource-fixtures",
          "ui-resources",
          "resource-offline-batch"
        ],
        "proves": "Owned original PDFs prove Info title/author/date metadata, validated page-tree count separate from declared count, compressed native text, per-page source facts, text/count/hash and configurable object/page/decompression/text limits. Encrypted, malformed, partial, disabled and legitimate blank cases remain distinct; incomplete input has no invented counts. Actual UI/CSV/project export and SQLite/legacy round trips preserve resource facts without inventing HTML metadata."
      },
      {
        "batch": "B10-resource-contract-v2",
        "verification_run_ids": [
          "resource-contract-responses"
        ],
        "proves": "Actual valid and password-encrypted authored PDFs distinguish stored/fetched/parser outcomes and completeness without treating MIME hints as successful parsing.",
        "limits": "Bounded own native/local acceptance. Existing remaining acceptance and reference parity remain separately tracked; CI/release/installation pending."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Links und Anker in PDFs** — Teilweise geprüft; `images.pdf_links`, P2.

  Orientierung: Zuverlässigen Ankertext und weitere Link-Aktionen prüfen; Nicht-HTTP-Aktionen nicht zu Weblinks umdeuten.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [resource-fixtures](../crates/core/tests/resource_fixtures.rs) (PASS), [ui-resources](../web/verification/ui-resource-live-results.json) (PASS), [resource-offline-batch](verification/resource-offline-batch.json) (PASS), [resource-contract-responses](verification/resource-contract-responses.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "images.pdf_links",
    "reference_name": "PDF links and anchor evidence",
    "status": "partial",
    "acceptance_test": "Resolve link annotations, record source document/page and extract text where reliable.",
    "remaining_acceptance": [
      "Associate reliable annotation anchor text with extracted page text and extend accepted PDF link action/annotation cases; unsupported non-HTTP actions remain explicit rather than fabricated web links."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "resource-fixtures",
          "ui-resources",
          "resource-offline-batch"
        ],
        "proves": "Direct/indirect HTTP URI link annotations resolve supported document URI bases, retain source document/page/rectangle/action evidence and enqueue eligible targets. Actual PDF-only local target discovery, per-page inspector facts, JSON/SQLite and dedicated CSV are verified; malformed bases/actions and annotation limits remain visible. Anchor text is not inferred from a nearby text stream."
      },
      {
        "batch": "B10-resource-contract-v2",
        "verification_run_ids": [
          "resource-contract-responses"
        ],
        "proves": "Valid PDF annotations have separate fetch-disabled and fetch-enabled native request controls in both profiles, with preserved original PDF role witnesses.",
        "limits": "Bounded own native/local acceptance. Existing remaining acceptance and reference parity remain separately tracked; CI/release/installation pending."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **Cookies als Beobachtungen erfassen** — Eigene Abnahme erfüllt; `images.cookies`, P2.

  Orientierung: Der eigene Cookie-Vertrag mit getrennten Browser-/HTTP-Beobachtungen ist geprüft.

  Beweisregister: [advanced-batch](verification/advanced-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "images.cookies",
    "reference_name": "Cookie audit",
    "status": "implemented",
    "acceptance_test": "Collect response and script cookies with domain/path/security attributes and distinguish session contexts.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "advanced-batch"
        ],
        "proves": "Opt-in browser captures response/script cookies with domain/path/expiry/httpOnly/secure/sameSite and value byte length. Fresh contexts per URL are explicit and fixture-tested; cookie values are omitted from persist/export. Shared authenticated sessions are a separate unfinished feature."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Browser-Rendering und eigene Extraktion

14 Einträge: 4 eigene Abnahmen erfüllt, 4 teilweise geprüft, 6 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **Rendern, Warten und Ressourcen steuern** — Teilweise geprüft; `rendering.wait`, P3.

  Orientierung: Viewport, Ressourcen-/Robots-Regeln und Timeout-Arten durchgehend konfigurieren und prüfen.

  Beweisregister: [renderer](../renderer/tests/worker.test.mjs) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [core-browser](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-renderer](../web/verification/ui-renderer-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.wait",
    "reference_name": "Render completion policy",
    "status": "partial",
    "acceptance_test": "Control viewport, wait/timeout and resource loading; distinguish timed-out render from a successful raw fetch.",
    "remaining_acceptance": [
      "Expose viewport/resource-loading policy through crawler config and prove selected resource/robots controls plus full timeout distinctions."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "renderer",
          "core-fixtures",
          "core-browser",
          "ui-renderer"
        ],
        "proves": "Worker controls viewport/wait/deadline; core distinguishes raw success from renderer failure and disallowed native pages never trigger rendering."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Rohes HTML und DOM vergleichen** — Teilweise geprüft; `rendering.source_diff`, P3.

  Orientierung: Beide vollständigen HTML-Repräsentationen erhalten und konkrete hinzugefügte/entfernte Elemente vergleichen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [core-browser](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [ui-renderer](../web/verification/ui-renderer-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.source_diff",
    "reference_name": "Raw/rendered parity analysis",
    "status": "partial",
    "acceptance_test": "Surface elements and links added/removed by rendering while retaining both representations.",
    "remaining_acceptance": [
      "Retain both full HTML representations and compute explicit added/removed element/link differences."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "core-browser",
          "ui-fixture",
          "ui-renderer"
        ],
        "proves": "RawMetadata preserves original titles/headings/links/extraction values while current fields reflect rendered DOM. Native status/headers remain separate."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Screenshots nachvollziehbar speichern** — Teilweise geprüft; `rendering.screenshot`, P3.

  Orientierung: Viewport im Crawl/UI auswählbar machen und Bild-/Artefakt-Prüfungen ergänzen.

  Beweisregister: [renderer](../renderer/tests/worker.test.mjs) (PASS), [core-browser](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-renderer](../web/verification/ui-renderer-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.screenshot",
    "reference_name": "Rendered screenshots",
    "status": "partial",
    "acceptance_test": "Capture a deterministic owned page at selected viewport and link the image to URL/snapshot.",
    "remaining_acceptance": [
      "Expose selected capture viewport through crawler/UI, add deterministic image/viewport assertions and artifact-file links."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "renderer",
          "core-browser",
          "ui-renderer"
        ],
        "proves": "Worker returns a valid viewport PNG; real core crawl stores screenshot_base64 on the URL record and JSON snapshot can serialize it. Actual UI lazily previews the PNG and project export retains render evidence."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Konsole und Netzwerk beobachten** — Teilweise geprüft; `rendering.console`, P3.

  Orientierung: Browser-Subressourcen nach Robots-Regeln steuern und blockierte Ressourcen mit Gründen prüfen.

  Beweisregister: [renderer](../renderer/tests/worker.test.mjs) (PASS), [core-browser](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-renderer](../web/verification/ui-renderer-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.console",
    "reference_name": "Console and blocked-resource evidence",
    "status": "partial",
    "acceptance_test": "Save browser JS errors and failed/robots-blocked resource loads with URLs and error types.",
    "remaining_acceptance": [
      "Enforce browser subresource robots policy and prove blocked-resource reason evidence; native main-page robots gating alone does not establish this."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "renderer",
          "core-browser",
          "ui-renderer"
        ],
        "proves": "Console/page errors and network request URL/type/status/failure objects are captured and stored; fixture asserts deliberate console error and HTTP observation."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Noch offen

- [ ] **Iframes und Shadow DOM behandeln** — Offen; `rendering.flatten`, P3.

  Orientierung: Zusätzliche DOM-Quellen mit Herkunft und Grenzen explizit erfassen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.flatten",
    "reference_name": "Shadow DOM and iframe flattening",
    "status": "planned",
    "acceptance_test": "Extract owned fixtures with shadow roots and same-origin iframe content; preserve source context.",
    "remaining_acceptance": [
      "Extract owned fixtures with shadow roots and same-origin iframe content; preserve source context."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **HTML-/DOM-Archive erhalten** — Offen; `rendering.archive`, P3.

  Orientierung: Architektur und Artefakte für spätere Offline-Auswertung und Wiederöffnung prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.archive",
    "reference_name": "Website archiving",
    "status": "planned",
    "acceptance_test": "Save configured page/resource artifacts and verify a manifest can locate each stored file.",
    "remaining_acceptance": [
      "Save configured page/resource artifacts and verify a manifest can locate each stored file."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Extraktion im Browser auswählen** — Offen; `rendering.visual_extract`, P3.

  Orientierung: Visuelle Auswahl in reproduzierbare Regeln übersetzen und getrennt prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.visual_extract",
    "reference_name": "Visual selector authoring",
    "status": "planned",
    "acceptance_test": "Choose an element, preview raw/rendered extraction, save selector and verify it on a second fixture.",
    "remaining_acceptance": [
      "Choose an element, preview raw/rendered extraction, save selector and verify it on a second fixture."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Eigene Text- und Quellcodesuchen** — Offen; `rendering.custom_search`, P3.

  Orientierung: Rohes HTML, DOM und ausgewählten Text mit Fundanzahlen und Match-Herkunft durchsuchen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.custom_search",
    "reference_name": "Literal and regex search",
    "status": "planned",
    "acceptance_test": "Search source, rendered content and selected text with occurrence counts and match evidence.",
    "remaining_acceptance": [
      "Search source, rendered content and selected text with occurrence counts and match evidence."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Klick-, Hover- und Scroll-Aktionen** — Offen; `rendering.interact`, P3.

  Orientierung: Autorisierte Aktionen in erklärter Reihenfolge ausführen und resultierende Links prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.interact",
    "reference_name": "Scripted interactions",
    "status": "planned",
    "acceptance_test": "Execute authored hover/click/scroll actions in declared order and discover resulting links.",
    "remaining_acceptance": [
      "Execute authored hover/click/scroll actions in declared order and discover resulting links."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Eigene Skriptbibliothek** — Offen; `rendering.snippet_library`, P3.

  Orientierung: Nutzer-Snippets ohne Geheimnisse importieren und exportieren.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.snippet_library",
    "reference_name": "User snippet library",
    "status": "planned",
    "acceptance_test": "Import/export user-owned snippets without storing provider secrets in shared library files.",
    "remaining_acceptance": [
      "Import/export user-owned snippets without storing provider secrets in shared library files."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **Native HTML-Auswertung** — Eigene Abnahme erfüllt; `rendering.text`, P3.

  Orientierung: Der eigene native HTML-Extraktionsvertrag ist geprüft.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [core-browser](../crates/core/tests/crawl_fixtures.rs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.text",
    "reference_name": "Raw HTML mode",
    "status": "implemented",
    "acceptance_test": "Fetch and parse a script-dependent fixture while proving no JavaScript execution.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "core-browser"
        ],
        "proves": "Native HTTP plus HTML parsing retains Raw title/Raw heading before optional browser execution. The extraction fixture ignores script words and the live core/browser fixture separately retains the unexecuted raw metadata."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **JavaScript mit Chromium rendern** — Eigene Abnahme erfüllt; `rendering.chromium`, P3.

  Orientierung: JS-erzeugte Links sowie Browser-, Viewport- und Wartezeit-Herkunft sind tatsächlich geprüft.

  Beweisregister: [renderer](../renderer/tests/worker.test.mjs) (PASS), [core-browser](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [ui-renderer](../web/verification/ui-renderer-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.chromium",
    "reference_name": "Chromium rendering mode",
    "status": "implemented",
    "acceptance_test": "Discover links created after script execution; save browser/version/config used for the crawl.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "renderer",
          "core-browser",
          "ui-fixture",
          "ui-renderer"
        ],
        "proves": "Real Rust→worker→installed Chrome execution changes title/H1 and extracts a JavaScript-only link. Optional renderer settings and worker startup are independently tested. PageRecord browser_version/rendered_viewport/render_wait_ms/network_events_dropped preserve worker provenance; Rust fixture and explicit live worker test verify it. Saved CrawlConfig retains optional renderer settings. The fresh real core-to-Chrome gate also asserts browser_version, rendered_viewport, render_wait_ms=100 and network_events_dropped=0."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Eigene CSS-, XPath- und Regex-Extraktion** — Eigene Abnahme erfüllt; `rendering.custom_extract`, P3.

  Orientierung: Unterstützte Profile und Grenzen vollständig gegen Sonderfälle prüfen; keine umfassende XPath-Parität behaupten.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [core-browser](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [ui-renderer](../web/verification/ui-renderer-live-results.json) (PASS), [advanced-batch](verification/advanced-batch.json) (PASS), [resource-offline-batch](verification/resource-offline-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.custom_extract",
    "reference_name": "CSS/XPath/regex extraction",
    "status": "implemented",
    "acceptance_test": "Return text, attributes, HTML, inner HTML and function values from a known fixture with multi-value results.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "core-fixtures",
          "core-browser",
          "ui-fixture",
          "ui-renderer"
        ],
        "proves": "Named CSS text/attribute extraction is validated before HTTP, applies to raw/rendered DOM, keeps all matches and empty results, and exports exact values. UI config and inspector are fixture-tested. The actual final app/browser→Rust→Chrome fixture additionally proves raw price10 versus rendered price20, saved extraction evidence and rendered-DOM table values."
      },
      {
        "verification_run_ids": [
          "advanced-batch"
        ],
        "proves": "CSS, namespace-aware supported XPath1.0 and Rust regex run on repaired raw and rendered HTML. Real browser fixture proves price10→20 and count scalar; source/type/engine/count/truncation/error survive snapshots and CSV."
      },
      {
        "verification_run_ids": [
          "core-unit",
          "core-fixtures",
          "core-browser",
          "ui-renderer",
          "resource-offline-batch"
        ],
        "proves": "CSS and supported XPath 1.0 return text, attributes, serialized outer/inner HTML, multi-value results and typed scalar/function values; Rust regex keeps captures. Actual raw/rendered nested markup is escaped in the UI and survives dedicated CSV and project JSON. XPath id/lang/extension functions and PCRE-only syntax are rejected upfront; the supported repaired-HTML/XPath 1.0/Rust regex contract is bounded and is not arbitrary XML or full vendor-language equivalence."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Eigene JavaScript-Auswertung** — Eigene Abnahme erfüllt; `rendering.javascript`, P3.

  Orientierung: Der eigene opt-in Worker-Vertrag mit Grenzen und Herkunft ist geprüft.

  Beweisregister: [advanced-batch](verification/advanced-batch.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "rendering.javascript",
    "reference_name": "Custom JavaScript extraction",
    "status": "implemented",
    "acceptance_test": "Run a user-authored script against a fixture with timeout/error handling and typed returned columns.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "advanced-batch"
        ],
        "proves": "Up to 32 named async scripts execute in captured page context with bounded time/output, typed JSON and per-script errors. Real UI→Rust→Chrome verifies error continuation, typed values, persistence, table filters and CSV. Worker verifies nonterminating execution ends and next URL remains usable."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Validierung und Qualitätsprüfungen

10 Einträge: 1 eigene Abnahmen erfüllt, 4 teilweise geprüft, 5 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **JSON-LD, Microdata und RDFa** — Teilweise geprüft; `quality.structured_formats`, P3.

  Orientierung: Microdata/RDFa samt Rohknoten-Herkunft und verschachtelten Graph-Fixtures ergänzen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [analysis](analysis-acceptance.md) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "quality.structured_formats",
    "reference_name": "JSON-LD, Microdata and RDFa",
    "status": "partial",
    "acceptance_test": "Extract nested graphs from each format; keep parse errors and raw node references.",
    "remaining_acceptance": [
      "Implement Microdata/RDFa with raw-node provenance and nested-graph fixtures for each supported format."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "analysis"
        ],
        "proves": "JSON-LD parsed blocks/error count and recursively observed @type facts are tested; context type-coercion definitions are excluded from content type facts."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Grenzen automatisierter Accessibility zeigen** — Teilweise geprüft; `quality.accessibility_limit`, P3.

  Orientierung: Needs-review und unvollständige Abdeckung durch UI und Spezialberichte erhalten.

  Beweisregister: [renderer](../renderer/tests/worker.test.mjs) (PASS), [core-browser](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-renderer](../web/verification/ui-renderer-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "quality.accessibility_limit",
    "reference_name": "Manual accessibility boundaries",
    "status": "partial",
    "acceptance_test": "Display automated-only coverage and incomplete/needs-review findings in report exports.",
    "remaining_acceptance": [
      "Display/export incomplete needs-review details and automated coverage throughout UI/specialist reports, with end-to-end assertions."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "renderer",
          "core-browser",
          "ui-renderer"
        ],
        "proves": "Worker accessibility JSON marks automated_only:true and keeps incomplete results; errors use null findings plus error instead of an empty pass report."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Mobile Nutzbarkeit der Zielseite prüfen** — Teilweise geprüft; `quality.mobile`, P3.

  Orientierung: Tap-Ziele und Inhalte auf tatsächlich gerenderter Website prüfen.

  Beweisregister: [core-unit](../crates/core/src) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "quality.mobile",
    "reference_name": "Mobile usability checks",
    "status": "partial",
    "acceptance_test": "Test viewport, sizing, tap targets and incompatible content under defined mobile rendering.",
    "remaining_acceptance": [
      "Add rendered target-site tap sizing/content usability rules and dedicated mobile-site fixtures; dashboard responsiveness is not website mobile usability proof."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-unit",
          "ui-fixture"
        ],
        "proves": "Viewport meta evidence/missing-viewport findings exist; dashboard itself passes narrow viewport layout checks."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Transport- und Seiten-Sicherheitsprüfungen** — Teilweise geprüft; `quality.security`, P3.

  Orientierung: Unsichere Formulare, MIME-Abweichungen, Header-Regeln und Transportfälle vervollständigen.

  Beweisregister: [analysis](analysis-acceptance.md) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "quality.security",
    "reference_name": "Transport and header audits",
    "status": "partial",
    "acceptance_test": "Test mixed content, insecure forms, MIME mismatch and configurable security-header presence with concrete evidence.",
    "remaining_acceptance": [
      "Add insecure-form/MIME-mismatch checks, configurable header rules and complete transport fixture coverage."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "analysis",
          "core-fixtures"
        ],
        "proves": "Observed HTTPS header presence/selected values and HTTP resource references are independently tested. HTTP anchor navigation remains a separate finding."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Noch offen

- [ ] **Schema.org-Regeln prüfen** — Offen; `quality.schema`, P3.

  Orientierung: Typen und Eigenschaften gegen eine benannte Version prüfen, einschließlich unbekannter und veralteter Begriffe.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "quality.schema",
    "reference_name": "Schema vocabulary validation",
    "status": "planned",
    "acceptance_test": "Check types/properties against an identified schema version; test unknown and deprecated terms.",
    "remaining_acceptance": [
      "Check types/properties against an identified schema version; test unknown and deprecated terms."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Rich-Result-Regeln prüfen** — Offen; `quality.richresults`, P3.

  Orientierung: Öffentliche versionierte Pflicht-/Empfehlungsregeln mit getrennten Fehlern und Warnungen prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "quality.richresults",
    "reference_name": "Rich result requirements",
    "status": "planned",
    "acceptance_test": "Validate required/recommended fields using versioned public rules; distinguish errors from warnings.",
    "remaining_acceptance": [
      "Validate required/recommended fields using versioned public rules; distinguish errors from warnings."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Ungültige Head-Struktur erkennen** — Offen; `quality.html_head`, P3.

  Orientierung: Ungültige Head-Kinder und Auswirkungen auf Metadaten mit Rohmarkup-Herkunft prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "quality.html_head",
    "reference_name": "Invalid head elements",
    "status": "planned",
    "acceptance_test": "Seed invalid head children that affect subsequent metadata; point to observed raw markup.",
    "remaining_acceptance": [
      "Seed invalid head children that affect subsequent metadata; point to observed raw markup."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **HTML formal validieren** — Offen; `quality.html_validation`, P3.

  Orientierung: Versionierten Validator mit Zeile, Spalte und Regel auf eigenen Fehler-Fixtures prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "quality.html_validation",
    "reference_name": "HTML conformance checks",
    "status": "planned",
    "acceptance_test": "Run a versioned validator against owned malformed HTML and retain line/column/rule where available.",
    "remaining_acceptance": [
      "Run a versioned validator against owned malformed HTML and retain line/column/rule where available."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **CO₂-Schätzung nachvollziehbar berechnen** — Offen; `quality.carbon`, P3.

  Orientierung: Datenbasis, Methode und Version angeben; fehlende Werte unbekannt lassen und Schätzung benennen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "quality.carbon",
    "reference_name": "Green hosting and carbon estimate",
    "status": "planned",
    "acceptance_test": "Expose dataset/method/version and unknown states; label output as an estimate.",
    "remaining_acceptance": [
      "Expose dataset/method/version and unknown states; label output as an estimate."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **Automatisierte Barrierefreiheit prüfen** — Eigene Abnahme erfüllt; `quality.accessibility`, P3.

  Orientierung: Der eigene axe-basierte Browser-Vertrag ist tatsächlich geprüft.

  Beweisregister: [renderer](../renderer/tests/worker.test.mjs) (PASS), [core-browser](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-renderer](../web/verification/ui-renderer-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "quality.accessibility",
    "reference_name": "axe accessibility findings",
    "status": "implemented",
    "acceptance_test": "Run version-pinned axe on a deliberately failing rendered fixture; retain rule, impact, node and WCAG tags.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "renderer",
          "core-browser",
          "ui-renderer"
        ],
        "proves": "Pinned axe-core4.14.0 runs on owned failing rendered HTML. Tests assert button-name/image-alt plus engine version, impact, node HTML/target and WCAG tags; the live Rust/browser test retains the accessibility JSON."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Sitemaps

5 Einträge: 1 eigene Abnahmen erfüllt, 3 teilweise geprüft, 1 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **Sitemap-Mitgliedschaft abgleichen** — Teilweise geprüft; `sitemap.coverage`, P2.

  Orientierung: Vollständige Mitgliedschaft speichern und Sitemap-only sowie nicht gelistete Crawl-URLs getrennt prüfen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [analysis](analysis-acceptance.md) (PASS), [sitemap-derived-assessment-local](verification/sitemap-assessment-local.json) (PASS_LOCAL_WINDOWS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "sitemap.coverage",
    "reference_name": "Sitemap coverage and orphan checks",
    "status": "partial",
    "acceptance_test": "Compare fetched pages and sitemap locations; distinguish not listed, sitemap-only and incomplete crawl.",
    "remaining_acceptance": [
      "Join persisted original memberships to fetched page and inlink observations, distinguishing not-listed, sitemap-only, pending, unavailable and historical evidence without inventing whole-website completeness."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "analysis"
        ],
        "proves": "Sitemap-only locations are fetched with discovery source; observed graph orphan candidates explicitly retain incomplete/limited coverage boundaries."
      },
      {
        "verification_run_ids": [
          "sitemap-derived-assessment-local"
        ],
        "proves": "Actual same-transaction inventory permits qualified Complete/KnownEmpty/Unknown for current or historical declared sources; missing/denied/incomplete sources remain unknown, invalid Page Locs separate. Target/inlink joins and full website coverage remain open."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Sitemap-Felder und Zielzustände validieren** — Teilweise geprüft; `sitemap.validity`, P2.

  Orientierung: Lastmod erhalten/prüfen und Redirect-, Non-200- sowie nicht indexierbare Ziele getrennt melden.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [sitemap-derived-assessment-local](verification/sitemap-assessment-local.json) (PASS_LOCAL_WINDOWS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "sitemap.validity",
    "reference_name": "Sitemap URL validation",
    "status": "partial",
    "acceptance_test": "Find redirect/non-200/non-indexable URLs and invalid lastmod values in fixture XML.",
    "remaining_acceptance": [
      "Produce explicit sitemap target redirect/non200/nonindexable findings from actual page observations; preserve original membership beside rewritten fetch identities."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures"
        ],
        "proves": "Discovered sitemap locations receive ordinary status/indexability observations and errors."
      },
      {
        "verification_run_ids": [
          "sitemap-derived-assessment-local"
        ],
        "proves": "Raw lastmod fields retained and assessed against named XSD1.0 date|dateTime and W3C precision profiles; actual API/SDK/browser handle calendar errors, duplicate elements, incomplete prefixes and undetermined leap seconds. Target redirect/non200/nonindexable findings remain open."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Sitemaps aus gespeicherten URLs erzeugen** — Teilweise geprüft; `sitemap.generate`, P2.

  Orientierung: Lastmod, Bilder und Hreflang unterstützen und Ausgabe unabhängig parsen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "sitemap.generate",
    "reference_name": "XML sitemap generation",
    "status": "partial",
    "acceptance_test": "Export valid XML with optional lastmod, image entries and hreflang; parse output with an independent parser.",
    "remaining_acceptance": [
      "Support optional lastmod/image/hreflang entries and independent-parser output validation."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "api",
          "ui-live"
        ],
        "proves": "Basic indexable-page loc XML export escapes URL text, passes core content assertions and downloads through API/live browser."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Noch offen

- [ ] **Sitemaps teilen und indexieren** — Offen; `sitemap.split`, P2.

  Orientierung: URL-/Byte-Grenzen, Indexdatei und exakte Segmentmitgliedschaft prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "sitemap.split",
    "reference_name": "Split and segmented sitemaps",
    "status": "planned",
    "acceptance_test": "Split at configured URL/byte bounds, create index and prove segment URL membership.",
    "remaining_acceptance": [
      "Split at configured URL/byte bounds, create index and prove segment URL membership."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **XML-Sitemaps entdecken** — Eigene Abnahme erfüllt; `sitemap.discovery`, P2.

  Orientierung: Gzip, Indexschleifen und fehlerhaftes XML über echte Requests prüfen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [core-unit](../crates/core/src) (PASS), [sitemap-derived-assessment-local](verification/sitemap-assessment-local.json) (PASS_LOCAL_WINDOWS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "sitemap.discovery",
    "reference_name": "Robots and explicit sitemaps",
    "status": "implemented",
    "acceptance_test": "Load a sitemap index, gzipped child sitemap and explicit input; handle loops/malformed XML safely.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "core-unit"
        ],
        "proves": "Robots/explicit sitemap inputs, nested sitemap indexes, multiple explicit origins, XML escaping, malformed-root rejection and bounded/seen discovery exist and are fixture-tested."
      },
      {
        "verification_run_ids": [
          "sitemap-derived-assessment-local"
        ],
        "proves": "Actual native explicit duplicate roots, recursive index/self-cycle, duplicate compressed child and malformed XML retain original occurrences, complete-source facts and bounded deduplication. Current workspace includes parser/transport/ledger cases. Independent discovery acceptance is met; vendor parity is unproven."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Autorisierte externe APIs

10 Einträge: 0 eigene Abnahmen erfüllt, 0 teilweise geprüft, 10 offen.

#### Noch offen

- [ ] **Google Analytics 4 anbinden** — Offen; `integrations.ga4`, P4.

  Orientierung: Autorisierte Property, URL-Abgleich und fehlende Daten getrennt von Nullwerten prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "integrations.ga4",
    "reference_name": "Google Analytics 4",
    "status": "planned",
    "acceptance_test": "Using an authorized property, join page metrics with declared URL matching and preserve absent data as absent.",
    "remaining_acceptance": [
      "Using an authorized property, join page metrics with declared URL matching and preserve absent data as absent."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Google Search Console anbinden** — Offen; `integrations.gsc`, P4.

  Orientierung: Datum/Dimensionen auswählen und Kennzahlen samt Fehlern und Abdeckung erhalten.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "integrations.gsc",
    "reference_name": "Search Console analytics",
    "status": "planned",
    "acceptance_test": "Fetch selected dates/dimensions and join clicks/impressions/CTR/position; report API errors and row coverage.",
    "remaining_acceptance": [
      "Fetch selected dates/dimensions and join clicks/impressions/CTR/position; report API errors and row coverage."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **URL-Inspection anbinden** — Offen; `integrations.inspection`, P4.

  Orientierung: Quoten einhalten und inspizierte von wartenden URLs samt Zeitpunkt unterscheiden.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "integrations.inspection",
    "reference_name": "Search Console URL inspection",
    "status": "planned",
    "acceptance_test": "Honor daily/property quotas; retain inspection timestamp and distinguish inspected from queued URLs.",
    "remaining_acceptance": [
      "Honor daily/property quotas; retain inspection timestamp and distinguish inspected from queued URLs."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **PageSpeed Insights anbinden** — Offen; `integrations.psi`, P4.

  Orientierung: Mobil/Desktop, Feld/Labor, Einheiten und Providerfehler getrennt erhalten.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "integrations.psi",
    "reference_name": "PageSpeed Insights and Lighthouse",
    "status": "planned",
    "acceptance_test": "Preserve mobile/desktop, field/lab distinction, metric units and provider error details.",
    "remaining_acceptance": [
      "Preserve mobile/desktop, field/lab distinction, metric units and provider error details."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Core Web Vitals aus Felddaten** — Offen; `integrations.cwv`, P4.

  Orientierung: Pass/Fail aus vorhandenen Felddaten bestimmen; fehlende Felddaten nicht durch Labordaten ersetzen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "integrations.cwv",
    "reference_name": "Core Web Vitals aggregation",
    "status": "planned",
    "acceptance_test": "Compute pass/fail from available field metrics; never substitute lab results for unavailable field data.",
    "remaining_acceptance": [
      "Compute pass/fail from available field metrics; never substitute lab results for unavailable field data."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Majestic-Metriken anbinden** — Offen; `integrations.majestic`, P4.

  Orientierung: Separat autorisierte Credentials, Datensätze, Quoten und URL-/Domain-Ebene prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "integrations.majestic",
    "reference_name": "Majestic metrics",
    "status": "planned",
    "acceptance_test": "Use separately authorized credentials; mock fresh/historic datasets, quota exhaustion and URL/domain scopes.",
    "remaining_acceptance": [
      "Use separately authorized credentials; mock fresh/historic datasets, quota exhaustion and URL/domain scopes."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Ahrefs-Metriken anbinden** — Offen; `integrations.ahrefs`, P4.

  Orientierung: Eigene autorisierte API-Zugänge, Ablauf, Quoten und Metrik-Abgleich prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "integrations.ahrefs",
    "reference_name": "Ahrefs metrics",
    "status": "planned",
    "acceptance_test": "Use independent authorized app/API access; mock expiry, quota and metric joins without reusing vendor tokens.",
    "remaining_acceptance": [
      "Use independent authorized app/API access; mock expiry, quota and metric joins without reusing vendor tokens."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Moz-Metriken anbinden** — Offen; `integrations.moz`, P4.

  Orientierung: Autorisierte URL-/Domain-Abfragen samt fehlenden Daten und Quoten prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "integrations.moz",
    "reference_name": "Moz metrics",
    "status": "planned",
    "acceptance_test": "Mock authorized URL/domain metric queries with unavailable/quota states.",
    "remaining_acceptance": [
      "Mock authorized URL/domain metric queries with unavailable/quota states."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Provider-Konten sicher verbinden** — Offen; `integrations.oauth`, P4.

  Orientierung: Hinzufügen, Trennen und Widerrufen prüfen; Tokens aus Logs und Versionsverwaltung heraushalten.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "integrations.oauth",
    "reference_name": "Account connection management",
    "status": "planned",
    "acceptance_test": "Add, disconnect and revoke an authorized provider account; verify tokens stay out of logs and version control.",
    "remaining_acceptance": [
      "Add, disconnect and revoke an authorized provider account; verify tokens stay out of logs and version control."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Provider-only URLs mit Herkunft** — Offen; `integrations.provider_urls`, P4.

  Orientierung: Optionales Enqueue mit Scope-Prüfung und unverfälschter Orphan-Evidenz ergänzen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "integrations.provider_urls",
    "reference_name": "API-discovered crawl candidates",
    "status": "planned",
    "acceptance_test": "Optionally enqueue provider-only URLs with provenance and scope checking; preserve orphan evidence.",
    "remaining_acceptance": [
      "Optionally enqueue provider-only URLs with provenance and scope checking; preserve orphan evidence."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Optionale KI und semantische Analyse

9 Einträge: 0 eigene Abnahmen erfüllt, 0 teilweise geprüft, 9 offen.

#### Noch offen

- [ ] **Optionale KI-Provider anbinden** — Offen; `ai.providers`, P4.

  Orientierung: Mock-Provider und lokales Ollama mit getrennten Modell-/Providerfehlern prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ai.providers",
    "reference_name": "Cloud and local model adapters",
    "status": "planned",
    "acceptance_test": "Use mock OpenAI-compatible/Gemini/Anthropic and a local Ollama endpoint; record model/provider errors separately.",
    "remaining_acceptance": [
      "Use mock OpenAI-compatible/Gemini/Anthropic and a local Ollama endpoint; record model/provider errors separately."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Eigene Prompts auf ausgewählten Daten** — Offen; `ai.prompts`, P4.

  Orientierung: Exakte Eingabe vorzeigen und typisierte Ergebnisse mit Herkunft speichern.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ai.prompts",
    "reference_name": "Custom prompts and test preview",
    "status": "planned",
    "acceptance_test": "Run an authored prompt on selected page text/HTML/extraction; preview exact input and persist typed output.",
    "remaining_acceptance": [
      "Run an authored prompt on selected page text/HTML/extraction; preview exact input and persist typed output."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Prompt-Bibliothek** — Offen; `ai.prompt_library`, P4.

  Orientierung: Nutzer-Prompts mit Modell/Einstellungen und ohne Geheimnisse roundtrippen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ai.prompt_library",
    "reference_name": "Prompt import/export",
    "status": "planned",
    "acceptance_test": "Round-trip user-authored prompts with model/settings and no secrets.",
    "remaining_acceptance": [
      "Round-trip user-authored prompts with model/settings and no secrets."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Text-Embeddings versioniert speichern** — Offen; `ai.embeddings`, P4.

  Orientierung: Modell-/Input-Hash binden und unpassende Dimensionen ablehnen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ai.embeddings",
    "reference_name": "Embedding generation",
    "status": "planned",
    "acceptance_test": "Embed selected text, cache with model/input hash and reject incompatible dimensionality.",
    "remaining_acceptance": [
      "Embed selected text, cache with model/input hash and reject incompatible dimensionality."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Vektorähnlichkeit nachvollziehbar berechnen** — Offen; `ai.similarity`, P4.

  Orientierung: Eigene bekannte Vektoren, Cosinus-Methode und Schwellen gemeinsam prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ai.similarity",
    "reference_name": "Semantic similarity",
    "status": "planned",
    "acceptance_test": "Rank a known vector fixture using a documented cosine method and configured thresholds.",
    "remaining_acceptance": [
      "Rank a known vector fixture using a documented cosine method and configured thresholds."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Semantische Ausreißer bestimmen** — Offen; `ai.outliers`, P4.

  Orientierung: Exakte Scores und erklärte Corpus-Grenzen zu jedem Ausreißer erhalten.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ai.outliers",
    "reference_name": "Semantic relevance and outliers",
    "status": "planned",
    "acceptance_test": "Report low-centroid-similarity fixture vectors with exact scores and declared corpus coverage.",
    "remaining_acceptance": [
      "Report low-centroid-similarity fixture vectors with exact scores and declared corpus coverage."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Semantische Suche und Cluster** — Offen; `ai.semantic_search`, P4.

  Orientierung: Gespeicherte Vektoren mit Modell-/Dimensions-Herkunft durchsuchen und darstellen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ai.semantic_search",
    "reference_name": "Semantic search and clusters",
    "status": "planned",
    "acceptance_test": "Query saved vectors and visualize clusters while retaining model/dimensionality provenance.",
    "remaining_acceptance": [
      "Query saved vectors and visualize clusters while retaining model/dimensionality provenance."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Semantische Redirect-Vorschläge** — Offen; `ai.redirect_mapping`, P4.

  Orientierung: Zielkandidaten deterministisch ranken und Scores exportieren; Änderungen ausdrücklich getrennt halten.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ai.redirect_mapping",
    "reference_name": "Semantic redirect suggestions",
    "status": "planned",
    "acceptance_test": "Rank candidate target pages from a deterministic vector fixture and export suggestions with score; never apply redirects automatically.",
    "remaining_acceptance": [
      "Rank candidate target pages from a deterministic vector fixture and export suggestions with score; never apply redirects automatically."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **KI-Nutzung und Budgets begrenzen** — Offen; `ai.usage`, P4.

  Orientierung: Rate, Kosten, Wiederholungen und lokale/entgeltliche Provider auseinanderhalten.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ai.usage",
    "reference_name": "Provider cost and rate controls",
    "status": "planned",
    "acceptance_test": "Enforce explicit rate/spend bounds for an API fixture, record retries and distinguish local from paid providers.",
    "remaining_acceptance": [
      "Enforce explicit rate/spend bounds for an API fixture, record retries and distinguish local from paid providers."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Speicherung, Projekte und Vergleiche

10 Einträge: 4 eigene Abnahmen erfüllt, 5 teilweise geprüft, 1 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **Speichermodus vergleichen** — Teilweise geprüft; `storage.memory`, P2.

  Orientierung: Gleiche Fixtures in RAM und SQLite mit gleichen Beobachtungen und erklärten Unterschieden prüfen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "storage.memory",
    "reference_name": "Optional memory storage",
    "status": "partial",
    "acceptance_test": "Run the same fixture in memory and disk modes; compare results while documenting mode differences.",
    "remaining_acceptance": [
      "Run the identical fixture in both modes and assert equal captured observations/graph with documented mode differences."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures"
        ],
        "proves": "Crawler can run with no SQLite store; default in-memory fixtures and durable-store recovery fixtures pass."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Versionierte Crawl-Snapshots** — Teilweise geprüft; `storage.snapshots`, P2.

  Orientierung: Schema-Version und vollständige optionale Artefakte roundtrippen.

  Beweisregister: [api](verification/api-e2e.json) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS), [ui-renderer](../web/verification/ui-renderer-live-results.json) (PASS), [crawl-control](verification/crawl-control-e2e.json) (PASS), [ui-control-live](../web/verification/ui-crawl-control-live-results.json) (PASS), [content-capture-core](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-native-chrome](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-api-cli](verification/content-capture-e2e.json) (PASS), [ui-content-fixture](../web/verification/ui-content-fixture-results.json) (PASS), [ui-content-live](../web/verification/ui-content-live-results.json) (PASS), [content-capture-batch](verification/content-capture-batch.json) (PASS), [content-capture-ci](verification/content-capture-ci.json) (PASS), [content-capture-installed](verification/content-capture-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "storage.snapshots",
    "reference_name": "Save/open/import/export snapshots",
    "status": "partial",
    "acceptance_test": "Round-trip an Open SEO Spider snapshot including config, graph, issues and optional artifacts; verify schema version.",
    "remaining_acceptance": [
      "Add an explicit versioned snapshot schema and complete optional-artifact round-trip assertions."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "api",
          "ui-fixture",
          "ui-live",
          "ui-renderer"
        ],
        "proves": "Complete CrawlReport JSON exports/imports config/pages/links/issues and optional inline browser fields. API round-trip compare has zero changes; live browser report download works. Actual browser render evidence survives project export and a legacy pre-extraction JSON report remains importable."
      },
      {
        "verification_run_ids": [
          "crawl-control",
          "ui-control-live"
        ],
        "proves": "Explicit resume archives exact current JSON bytes including unknown extensions/formatting, immutable earlier generation remains identical, and read-only startup never rewrites historical reports."
      },
      {
        "verification_run_ids": [
          "content-capture-core",
          "content-capture-native-chrome",
          "content-capture-api-cli",
          "ui-content-fixture",
          "ui-content-live",
          "content-capture-batch"
        ],
        "proves": "Optional saved PageRecord content_captures and source/config/profile/revision facts roundtrip in native project JSON without changing capture-disabled legacy serialization. Typed snapshot SHA is explicitly distinct from original archived file-byte hashes; selected parent bytes and unrelated capture bytes remain unchanged."
      },
      {
        "verification_run_ids": [
          "content-capture-ci",
          "content-capture-installed"
        ],
        "proves": "Exact-source B8 commit667bc97 passes four CI jobs (Windows/Linux, renderer, Linux container) and its immutable native Windows release runs the bounded raw/DOM saved-text contracts with19 MCP tools while preserving seven jobs/eleven original JSON bytes after restart. This does not prove whole-site analysis, saved full-HTML CSS reselection or subsequent B9 source changes."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Projekte und Crawl-Historie** — Teilweise geprüft; `storage.projects`, P2.

  Orientierung: Benannte Projekte, Labels und Erstellungs-/Konfigurationsidentität prüfen.

  Beweisregister: [api](verification/api-e2e.json) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "storage.projects",
    "reference_name": "Projects and crawl history",
    "status": "partial",
    "acceptance_test": "Group snapshots by project, label them, reopen them and record their creation/config identity.",
    "remaining_acceptance": [
      "Add named project grouping, labels and explicit creation/config identity acceptance."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "api",
          "ui-fixture"
        ],
        "proves": "Local crawl history reopens persisted snapshots and active/interrupted job identity survives restart."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Konfiguration separat importieren/exportieren** — Teilweise geprüft; `storage.config`, P2.

  Orientierung: Credentials, Pfade und unveränderte Einstellungen bei Roundtrips prüfen.

  Beweisregister: [api](verification/api-e2e.json) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [list-import-api-cli](verification/list-import-e2e.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "storage.config",
    "reference_name": "Reusable crawl configuration",
    "status": "partial",
    "acceptance_test": "Export/import typed config and validate missing credentials/paths without silently changing settings.",
    "remaining_acceptance": [
      "Add dedicated config import/export with credential/path validation and unchanged setting assertions."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "api",
          "core-fixtures",
          "ui-fixture"
        ],
        "proves": "Typed CrawlConfig is serialized with snapshots; defaults/aliases, bad headers/selectors/regexes and persisted UI settings are tested."
      },
      {
        "verification_run_ids": [
          "list-import-api-cli"
        ],
        "proves": "CLI --config preserves saved concurrency, fragment and named extraction settings when flags are unset. Dedicated configuration UI and credential/path validation remain incomplete."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Portable Projekte und Archive** — Teilweise geprüft; `storage.portable`, P2.

  Orientierung: In anderem Verzeichnis öffnen und Manifest, Version sowie alle erklärten Artefakte prüfen.

  Beweisregister: [api](verification/api-e2e.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "storage.portable",
    "reference_name": "Shared crawl portability",
    "status": "partial",
    "acceptance_test": "Copy an Open SEO Spider archive to a second local directory and open all declared portable artifacts.",
    "remaining_acceptance": [
      "Copy/open an archive in a second directory and validate all declared portable artifacts plus manifest/schema version."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "api"
        ],
        "proves": "Self-contained report JSON imports under a new local crawl ID, keeping source observations; inline optional screenshot data has no external image path requirement."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Noch offen

- [ ] **Aufbewahrung und Sperren** — Offen; `storage.retention`, P2.

  Orientierung: Testzeitpunkte, gesperrte Snapshots und aktive Jobs bei automatischer Bereinigung prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "storage.retention",
    "reference_name": "Retention and locking",
    "status": "planned",
    "acceptance_test": "Use test timestamps to expire unlocked fixture snapshots while preserving locked snapshots and active jobs.",
    "remaining_acceptance": [
      "Use test timestamps to expire unlocked fixture snapshots while preserving locked snapshots and active jobs."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **Persistente SQLite-Crawls** — Eigene Abnahme erfüllt; `storage.database`, P2.

  Orientierung: Der eigene persistente Kern mit Wiederherstellung ist geprüft.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS), [native-policy-core](../crates/core/tests/policy_fixtures.rs) (PASS), [native-policy-api-cli](verification/crawl-policy-e2e.json) (PASS), [ui-policy-live](../web/verification/ui-policy-live-results.json) (PASS), [native-policy-ci](verification/native-policy-ci.json) (PASS), [native-policy-installed](verification/native-policy-installed.json) (PASS), [content-capture-core](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-native-chrome](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-api-cli](verification/content-capture-e2e.json) (PASS), [ui-content-fixture](../web/verification/ui-content-fixture-results.json) (PASS), [ui-content-live](../web/verification/ui-content-live-results.json) (PASS), [content-capture-batch](verification/content-capture-batch.json) (PASS), [content-capture-ci](verification/content-capture-ci.json) (PASS), [content-capture-installed](verification/content-capture-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "storage.database",
    "reference_name": "Durable local database",
    "status": "implemented",
    "acceptance_test": "Crash after committed requests; recover completed results and pending queue with consistent graph state.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "api"
        ],
        "proves": "SQLite stores result plus discovered frontier transactionally. Cancel/resume and actual process termination/restart retain completed pages and finish the pending graph without re-fetching the completed seed."
      },
      {
        "verification_run_ids": [
          "native-policy-core",
          "native-policy-api-cli",
          "ui-policy-live"
        ],
        "proves": "Native policy admission and page/protocol/occurrence/frontier updates are one guarded SQLite transaction; rollback/pause/crash/resume fixtures preserve original decisions, aliases and immutable selected permissions. Rolling ledger SHA/bytes and SQL identity/generation are verified before resume."
      },
      {
        "verification_run_ids": [
          "native-policy-ci",
          "native-policy-installed"
        ],
        "proves": "Exact B7 source915e175 has four successful Windows/Linux/Chromium/Linux Docker CI jobs and an installed native Windows immutable release. Existing seven jobs and eleven original JSON files remain byte-identical after restart; no user-data fixture requests. This confirms the already documented B7 slice only; ongoing B8 capture/corpus changes and full reference parity are not attested."
      },
      {
        "verification_run_ids": [
          "content-capture-core",
          "content-capture-native-chrome",
          "content-capture-api-cli",
          "ui-content-fixture",
          "ui-content-live",
          "content-capture-batch"
        ],
        "proves": "Configured content observations validate source/config/generation and commit with page/frontier/native-policy ledger in one transaction. Native capture fixtures verify rollback, SQLite-only pause/crash recovery, immutable source config, tamper rejection and selected inherited/fresh capture isolation. Read-only restart retains valid checkpoint binding; JSON beside malformed SQLite remains declarative with unknown generation."
      },
      {
        "verification_run_ids": [
          "content-capture-ci",
          "content-capture-installed"
        ],
        "proves": "Exact-source B8 commit667bc97 passes four CI jobs (Windows/Linux, renderer, Linux container) and its immutable native Windows release runs the bounded raw/DOM saved-text contracts with19 MCP tools while preserving seven jobs/eleven original JSON bytes after restart. This does not prove whole-site analysis, saved full-HTML CSS reselection or subsequent B9 source changes."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Veränderte Inhalte und Regeln vergleichen** — Eigene Abnahme erfüllt; `storage.compare_changes`, P2.

  Orientierung: Graph-/Schema-Deltas und Änderungen bei gleicher Issue-Anzahl prüfen.

  Beweisregister: [api](verification/api-e2e.json) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [saved-comparison-native](verification/saved-comparison-e2e.json) (PASS), [saved-comparison-backend-checkpoint](verification/saved-comparison-backend-checkpoint.json) (PASS), [saved-comparison-ci](verification/saved-comparison-ci.json) (PASS), [saved-comparison-release](verification/saved-comparison-release.json) (PASS), [saved-comparison-installed](verification/saved-comparison-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "storage.compare_changes",
    "reference_name": "Field and content changes",
    "status": "implemented",
    "acceptance_test": "Detect title, description, headings, graph, directives, structured data and content changes even if issue counts are unchanged.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "api",
          "core-fixtures"
        ],
        "proves": "Comparison joins stable URL keys and checks status/finalURL/title/description/headings/canonical/robots/hash/count/render fields; imported-identical report and added URLs are tested."
      },
      {
        "verification_run_ids": [
          "saved-comparison-native",
          "saved-comparison-backend-checkpoint"
        ],
        "proves": "Actual native title, description, H1, H2, robots-directive, link graph occurrence, JSON-LD and selected raw-text changes are detected together while issue counts are equal. Source absence/partiality and unavailable rendering remain explicit; original field values are retained.",
        "limits": "Own versioned stored-fact comparison acceptance only; browser/release and full vendor parity remain separate."
      },
      {
        "verification_run_ids": [
          "saved-comparison-ci",
          "saved-comparison-release",
          "saved-comparison-installed"
        ],
        "proves": "Exact-source CI, immutable optimized native release and data-preserving user installation now pass the saved-fact comparison and optional complete typed-report export gates.",
        "limits": "Own contracts and stored observations; reference parity remains NOT_PROVEN."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **URL-Mapping vor Vergleichen** — Eigene Abnahme erfüllt; `storage.compare_mapping`, P2.

  Orientierung: Staging-/Produktions- oder Migrationspräfixe mit sichtbaren Kollisionen abgleichen.

  Beweisregister: [saved-comparison-native](verification/saved-comparison-e2e.json) (PASS), [saved-comparison-backend-checkpoint](verification/saved-comparison-backend-checkpoint.json) (PASS), [saved-comparison-native-mapping](verification/saved-comparison-mapping-e2e.json) (PASS), [saved-comparison-mapping-oracles](verification/saved-comparison-mapping-oracle-controls.json) (PASS), [saved-comparison-native-ui](verification/saved-comparison-ui-native.json) (PASS), [saved-comparison-ci](verification/saved-comparison-ci.json) (PASS), [saved-comparison-release](verification/saved-comparison-release.json) (PASS), [saved-comparison-installed](verification/saved-comparison-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "storage.compare_mapping",
    "reference_name": "Cross-host comparison mapping",
    "status": "implemented",
    "acceptance_test": "Map staging/production or migrated URL prefixes before joining snapshots and expose collisions.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "saved-comparison-native",
          "saved-comparison-backend-checkpoint"
        ],
        "proves": "Explicit origin/segment-prefix mappings transform only join keys. Native controls retain original requested/final URLs and reject overlap; directed Core tests cover collisions on both sides, percent-encoded boundaries, malformed authorities and query identity.",
        "limits": "Full native cross-origin/mapping-induced collision and browser workflow remain open."
      },
      {
        "verification_run_ids": [
          "saved-comparison-native-mapping",
          "saved-comparison-mapping-oracles",
          "saved-comparison-native-ui"
        ],
        "proves": "Four actual native crawls on two distinct origins preserve every original URL/link/JSON-LD/text fact while explicit origin/segment-prefix mappings join stage and production identities, support reverse mapping and reject before/after/two-sided mapped collisions with all source ordinals. CLI all-column CSV and JSON agree; original report/revision/export bytes and offline results survive cold restart with two drained HTTP witnesses at zero.",
        "limits": "Own recorded-identity mapping acceptance. Raw historical bytes, SQL authenticity, full website equality and vendor parity are not established."
      },
      {
        "verification_run_ids": [
          "saved-comparison-ci",
          "saved-comparison-release",
          "saved-comparison-installed"
        ],
        "proves": "Exact-source CI, immutable optimized native release and data-preserving user installation now pass the saved-fact comparison and optional complete typed-report export gates.",
        "limits": "Own contracts and stored observations; reference parity remains NOT_PROVEN."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Vergleiche vollständig exportieren** — Eigene Abnahme erfüllt; `storage.compare_export`, P2.

  Orientierung: Gemeinsamen Export mit allen Delta-Arten und beiden Snapshot-Herkünften prüfen.

  Beweisregister: [api](verification/api-e2e.json) (PASS), [saved-comparison-native](verification/saved-comparison-e2e.json) (PASS), [saved-comparison-backend-checkpoint](verification/saved-comparison-backend-checkpoint.json) (PASS), [saved-comparison-native-ui](verification/saved-comparison-ui-native.json) (PASS), [saved-comparison-native-mapping](verification/saved-comparison-mapping-e2e.json) (PASS), [saved-comparison-ui-fixture](verification/saved-comparison-ui-fixture.json) (PASS), [saved-comparison-original-bundle-native](verification/saved-comparison-original-bundle-e2e.json) (PASS), [saved-comparison-original-bundle-oracles](verification/saved-comparison-original-bundle-oracle-controls.json) (PASS), [saved-comparison-original-bundle-cli-budget](verification/saved-comparison-original-bundle-cli-budget.json) (PASS), [saved-comparison-original-bundle-ui-contract](verification/saved-comparison-original-bundle-ui-contract.json) (PASS), [saved-comparison-original-bundle-ui-fixture](verification/saved-comparison-original-bundle-ui-fixture.json) (PASS), [saved-comparison-original-bundle-ui-native](verification/saved-comparison-original-bundle-ui-native.json) (PASS), [saved-comparison-original-bundle-mutation-fence](verification/saved-comparison-original-bundle-native-fence.json) (PASS), [saved-comparison-original-bundle-checkpoint](verification/saved-comparison-original-bundle-checkpoint.json) (PASS), [saved-comparison-ci](verification/saved-comparison-ci.json) (PASS), [saved-comparison-release](verification/saved-comparison-release.json) (PASS), [saved-comparison-installed](verification/saved-comparison-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "storage.compare_export",
    "reference_name": "Comparison export",
    "status": "implemented",
    "acceptance_test": "Export both snapshots and deltas with new/missing/changed evidence and stable URL keys.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "api"
        ],
        "proves": "Snapshot exports and JSON comparison endpoint expose before/after IDs and added/removed/changed URL field deltas."
      },
      {
        "verification_run_ids": [
          "saved-comparison-native",
          "saved-comparison-backend-checkpoint"
        ],
        "proves": "Local-file CLI writes comparison JSON and formula-protected CSV with concrete field values/states/profiles/provenance and both typed report hashes; existing destinations retain their exact bytes.",
        "limits": "Historical projection-only checkpoint; full original typed reports are covered by the separately registered bundle proofs."
      },
      {
        "verification_run_ids": [
          "saved-comparison-native-ui",
          "saved-comparison-native-mapping",
          "saved-comparison-ui-fixture"
        ],
        "proves": "Native/browser comparison JSON preserves exact received response bytes; CSV carries every compared field rather than only visible rows, with filter, profile, provenance and formula protection. Actual cross-origin CLI output agrees column-for-column with API.",
        "limits": "Historical projection-only checkpoint; full original typed reports are covered by the separately registered bundle proofs."
      },
      {
        "verification_run_ids": [
          "saved-comparison-original-bundle-native",
          "saved-comparison-original-bundle-oracles",
          "saved-comparison-original-bundle-cli-budget",
          "saved-comparison-original-bundle-ui-contract",
          "saved-comparison-original-bundle-ui-fixture",
          "saved-comparison-original-bundle-ui-native",
          "saved-comparison-original-bundle-mutation-fence",
          "saved-comparison-original-bundle-checkpoint"
        ],
        "proves": "Optional complete typed original-report bundle retains both source reports and all four delta classes in API/CLI/MCP and native browser JSON downloads; ordered original values, exact numeric lexemes, report hashes/counts and all compared CSV fields are independently checked. Default projection stays unchanged; over-budget or invalid requested bundles never silently fall back.",
        "limits": "Typed loaded reports, not historical input-file bytes or SQL authenticity. Browser exports exact received JSON and does not reconstruct Rust hashes. Native CLI fixed-ceiling control is synthetic; API/MCP fixed64MiB boundary and vendor parity remain separately unproven."
      },
      {
        "verification_run_ids": [
          "saved-comparison-ci",
          "saved-comparison-release",
          "saved-comparison-installed"
        ],
        "proves": "Exact-source CI, immutable optimized native release and data-preserving user installation now pass the saved-fact comparison and optional complete typed-report export gates.",
        "limits": "Own contracts and stored observations; reference parity remains NOT_PROVEN."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Automation, Exporte und MCP

11 Einträge: 3 eigene Abnahmen erfüllt, 4 teilweise geprüft, 4 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **Tabellen und gefilterte Exporte** — Teilweise geprüft; `automation.tables`, P4.

  Orientierung: XLSX, Filterübereinstimmung, Formelinjektion, Unicode und Newlines vollständig prüfen.

  Beweisregister: [api](verification/api-e2e.json) (PASS), [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS), [native-policy-core](../crates/core/tests/policy_fixtures.rs) (PASS), [native-policy-api-cli](verification/crawl-policy-e2e.json) (PASS), [ui-policy-live](../web/verification/ui-policy-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "automation.tables",
    "reference_name": "Table and bulk exports",
    "status": "partial",
    "acceptance_test": "Export filtered observations and all-occurrence evidence as CSV/XLSX; escape formulas and preserve Unicode/newlines.",
    "remaining_acceptance": [
      "Add XLSX, filtered-export parity, formula-injection escaping and explicit Unicode/newline acceptance."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "api",
          "core-fixtures",
          "ui-live"
        ],
        "proves": "CSV pages/issues/all-occurrence links/extraction exports and basic XML/JSON exports exist; exact quoted extraction values and browser downloads are tested."
      },
      {
        "verification_run_ids": [
          "native-policy-core",
          "native-policy-api-cli",
          "ui-policy-live"
        ],
        "proves": "Native policy cursor exports label visible subsets while complete streamed JSON/CSV expose all506 ordered saved occurrences with snapshot counts/hashes and formula-safe CSV."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Spezialisierte Auditberichte** — Teilweise geprüft; `automation.reports`, P4.

  Orientierung: Versionierten Überblick und eigene Redirect-, Canonical-, Hreflang-, Sitemap-, Security- und Content-Berichte prüfen.

  Beweisregister: [api](verification/api-e2e.json) (PASS), [analysis](analysis-acceptance.md) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "automation.reports",
    "reference_name": "Issue and specialist reports",
    "status": "partial",
    "acceptance_test": "Export versioned overview plus redirect/canonical/hreflang/sitemap/security/content report fixtures.",
    "remaining_acceptance": [
      "Add versioned overview and dedicated redirect/canonical/hreflang/sitemap/security/content specialist report acceptance."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "api",
          "analysis",
          "ui-live"
        ],
        "proves": "Issues CSV and advanced graph/relationship/security analysis plus basic sitemap exports are accessible and include independently observed issue codes."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **HTML, Screenshots und Ressourcen exportieren** — Teilweise geprüft; `automation.assets`, P4.

  Orientierung: Rohes HTML, DOM, Ressourcen, Cookies und PDFs mit Artefaktmanifest exportieren.

  Beweisregister: [core-browser](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "automation.assets",
    "reference_name": "Artifact exports",
    "status": "partial",
    "acceptance_test": "Export source, rendered HTML, screenshots, headers, cookies and PDF artifacts with a manifest.",
    "remaining_acceptance": [
      "Export raw/rendered HTML and resource/cookie/PDF files with an artifact manifest."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-browser",
          "api"
        ],
        "proves": "CrawlReport JSON carries headers, raw metadata and optional screenshot/accessibility/network observations."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Nutzung über mehrere Crawls auswerten** — Teilweise geprüft; `automation.stats`, P4.

  Orientierung: Aggregate und standardmäßig abgeschaltete Telemetrie ausdrücklich prüfen.

  Beweisregister: [api](verification/api-e2e.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "automation.stats",
    "reference_name": "Local usage statistics",
    "status": "partial",
    "acceptance_test": "Aggregate locally saved crawl counts/duration without sending telemetry by default.",
    "remaining_acceptance": [
      "Add explicit aggregate usage statistics over multiple snapshots and demonstrate default no-telemetry behavior."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "api",
          "ui-live"
        ],
        "proves": "Saved CrawlSummary and local job status expose crawl counts/duration and current history in the local application."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Noch offen

- [ ] **Looker-/Cloud-Export** — Offen; `automation.looker`, P4.

  Orientierung: Autorisierte Ziele, Append/Replace und stabiles Spaltenschema über mehrere Läufe prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "automation.looker",
    "reference_name": "Google Sheets/Looker output",
    "status": "planned",
    "acceptance_test": "Use an authorized export destination; retain append/replace setting and stable column schema across two runs.",
    "remaining_acceptance": [
      "Use an authorized export destination; retain append/replace setting and stable column schema across two runs."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Wiederkehrende lokale Crawls** — Offen; `automation.scheduler`, P4.

  Orientierung: Gespeicherte Aufgaben, Überlappung, Historie und Fehler mit Testuhr prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "automation.scheduler",
    "reference_name": "Recurring scheduling",
    "status": "planned",
    "acceptance_test": "Use a fake clock to execute saved crawl tasks, record history/errors and define overlap policy.",
    "remaining_acceptance": [
      "Use a fake clock to execute saved crawl tasks, record history/errors and define overlap policy."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Nach Crawls automatisch vergleichen** — Offen; `automation.auto_compare`, P4.

  Orientierung: Zwei abgeschlossene Projekt-Snapshots nach gespeicherten Regeln automatisch vergleichen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "automation.auto_compare",
    "reference_name": "Automatic last-two comparison",
    "status": "planned",
    "acceptance_test": "After two completed project snapshots, create a comparison automatically using saved comparison settings.",
    "remaining_acceptance": [
      "After two completed project snapshots, create a comparison automatically using saved comparison settings."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Benachrichtigungen an konfigurierte Ziele** — Offen; `automation.notifications`, P4.

  Orientierung: Eigene lokale SMTP-Testsenke samt Zusammenfassung, Deltas und Attachment-Fehlern prüfen.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "automation.notifications",
    "reference_name": "Completion and change notifications",
    "status": "planned",
    "acceptance_test": "Send only to an explicitly configured local test SMTP sink; verify summary, change table and attachment error handling.",
    "remaining_acceptance": [
      "Send only to an explicitly configured local test SMTP sink; verify summary, change table and attachment error handling."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **Lokale Kommandozeile** — Eigene Abnahme erfüllt; `automation.cli`, P4.

  Orientierung: Der eigene CLI-Vertrag mit Offline- und Crawl-Funktionen ist geprüft.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS), [list-import-batch](verification/list-import-batch.json) (PASS), [list-import-api-cli](verification/list-import-e2e.json) (PASS), [native-policy-ci](verification/native-policy-ci.json) (PASS), [native-policy-installed](verification/native-policy-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "automation.cli",
    "reference_name": "Headless command line",
    "status": "implemented",
    "acceptance_test": "Run a crawl/config/import/analysis/export sequence from CLI and verify deterministic exit codes.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "api"
        ],
        "proves": "Original CLI exposes serve/crawl/mcp and crawl JSON/CSV/XML output paths; underlying crawler/export behavior is tested through core/API."
      },
      {
        "verification_run_ids": [
          "list-import-batch",
          "list-import-api-cli"
        ],
        "proves": "Actual CLI reads typed configuration, imports lists, crawls, analyzes and exports JSON/input CSV. Unset flags preserve saved settings. Explicit success/error exits are 0/1/2; invalid selection/config/scheme/options produce no requests or output."
      },
      {
        "verification_run_ids": [
          "native-policy-ci",
          "native-policy-installed"
        ],
        "proves": "Exact B7 source915e175 has four successful Windows/Linux/Chromium/Linux Docker CI jobs and an installed native Windows immutable release. Existing seven jobs and eleven original JSON files remain byte-identical after restart; no user-data fixture requests. This confirms the already documented B7 slice only; ongoing B8 capture/corpus changes and full reference parity are not attested."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Lokale MCP-Werkzeuge** — Eigene Abnahme erfüllt; `automation.mcp`, P4.

  Orientierung: Der eigene MCP-Werkzeugvertrag ist gegen offizielle SDK-Clients geprüft.

  Beweisregister: [mcp](verification/mcp-e2e.json) (PASS), [native-policy-ci](verification/native-policy-ci.json) (PASS), [native-policy-installed](verification/native-policy-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "automation.mcp",
    "reference_name": "MCP agent interface",
    "status": "implemented",
    "acceptance_test": "Perform MCP initialize/list-tools/crawl/read/export using a local client; keep script execution an explicit capability.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "mcp"
        ],
        "proves": "Official local MCP SDK clients initialize/list 11 tools, start a crawl, read status/pages/issues/analysis/project JSON, compare/list history and receive tool errors. Project JSON provides local snapshot export; arbitrary script execution is not exposed as an implicit tool."
      },
      {
        "verification_run_ids": [
          "native-policy-ci",
          "native-policy-installed"
        ],
        "proves": "Exact B7 source915e175 has four successful Windows/Linux/Chromium/Linux Docker CI jobs and an installed native Windows immutable release. Existing seven jobs and eleven original JSON files remain byte-identical after restart; no user-data fixture requests. This confirms the already documented B7 slice only; ongoing B8 capture/corpus changes and full reference parity are not attested."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **MCP über stdio und HTTP** — Eigene Abnahme erfüllt; `automation.mcp_transport`, P4.

  Orientierung: Beide eigenen Transportarten sind gegen offizielle SDK-Clients geprüft.

  Beweisregister: [mcp](verification/mcp-e2e.json) (PASS), [native-policy-ci](verification/native-policy-ci.json) (PASS), [native-policy-installed](verification/native-policy-installed.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "automation.mcp_transport",
    "reference_name": "STDIO and streamable HTTP",
    "status": "implemented",
    "acceptance_test": "Exercise both transports on loopback and prove JSON protocol framing and job identifiers remain stable.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "mcp"
        ],
        "proves": "Both stdio and Streamable HTTP on loopback complete the official SDK initialize/tool workflow. Typed JSON framing and the returned job IDs remain usable for status/pages/project/analysis throughout each transport."
      },
      {
        "verification_run_ids": [
          "native-policy-ci",
          "native-policy-installed"
        ],
        "proves": "Exact B7 source915e175 has four successful Windows/Linux/Chromium/Linux Docker CI jobs and an installed native Windows immutable release. Existing seven jobs and eleven original JSON files remain byte-identical after restart; no user-data fixture requests. This confirms the already documented B7 slice only; ongoing B8 capture/corpus changes and full reference parity are not attested."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Oberfläche für die Analyse

6 Einträge: 0 eigene Abnahmen erfüllt, 5 teilweise geprüft, 1 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **Live-Fortschritt und Steuerung** — Teilweise geprüft; `ui.realtime`, P2.

  Orientierung: Verzögerte Updates, Pause und Konfigurationseinsicht im Browser vollständig prüfen.

  Beweisregister: [api](verification/api-e2e.json) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ui.realtime",
    "reference_name": "Live crawl progress and tables",
    "status": "partial",
    "acceptance_test": "During a delayed fixture crawl, update progress/results and permit safe pause/config inspection.",
    "remaining_acceptance": [
      "Exercise delayed live updates and explicit pause/config-inspection acceptance in browser."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "api",
          "ui-fixture",
          "ui-live"
        ],
        "proves": "Polling job progress/results, cancellation/resume and actual form→Rust→fixture→inspector/export flow are tested; safe bounded pagination exists."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Tabellen filtern und anpassen** — Teilweise geprüft; `ui.filter`, P2.

  Orientierung: Spaltenpräferenzen wiederöffnen und kombinierte Zahlen-/Textfilter prüfen.

  Beweisregister: [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ui.filter",
    "reference_name": "Sort, search and saved columns",
    "status": "partial",
    "acceptance_test": "Combine numeric/text filters and sort; keep column preferences when reopening a project.",
    "remaining_acceptance": [
      "Save/reopen column preferences and test combined numeric/text filtering rather than separate controls alone."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "ui-fixture",
          "ui-live"
        ],
        "proves": "Text search, numeric sorting, title/image/issue filters, bounded pagination and persisted settings are fixture-tested; live404 filter/inspector is proved."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Issue-Überblick und Drill-down** — Teilweise geprüft; `ui.overview`, P2.

  Orientierung: Schweregrad, Typ, Anzahl und Details aller beobachteten Issue-Familien konsistent prüfen.

  Beweisregister: [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ui.overview",
    "reference_name": "Overview and issue taxonomy",
    "status": "partial",
    "acceptance_test": "Show severity/type/count and drill-down to every affected URL; distinguish observation from advice.",
    "remaining_acceptance": [
      "Assert severity/type/count consistency and drill-down for every observed issue family with clear observation/advice distinction."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "ui-fixture",
          "ui-live"
        ],
        "proves": "Overview and independently computed issue counters/filtering drill into URL observations; live404 and robots findings are tested."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Quellen und Seiten-Details inspizieren** — Teilweise geprüft; `ui.details`, P2.

  Orientierung: Vollständiges Roh-HTML/DOM, Cookies, Ressourcen und strukturierte Knoten im Detail zeigen.

  Beweisregister: [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS), [ui-renderer](../web/verification/ui-renderer-live-results.json) (PASS), [native-policy-core](../crates/core/tests/policy_fixtures.rs) (PASS), [native-policy-api-cli](verification/crawl-policy-e2e.json) (PASS), [ui-policy-live](../web/verification/ui-policy-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ui.details",
    "reference_name": "URL evidence inspector",
    "status": "partial",
    "acceptance_test": "Select a URL and inspect links, resources, raw/rendered source, headers, cookies and structured nodes.",
    "remaining_acceptance": [
      "Add full raw/rendered HTML, cookie details and complete resource/structured-node inspection acceptance."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "ui-fixture",
          "ui-live",
          "ui-renderer"
        ],
        "proves": "Inspector shows URL status, headers, anchor/image evidence, link metrics, JSON-LD and CSS extraction source; rendered/raw metadata is present in supported fields."
      },
      {
        "verification_run_ids": [
          "native-policy-core",
          "native-policy-api-cli",
          "ui-policy-live"
        ],
        "proves": "Native policy UI exposes bounded saved-ledger pages, exact raw/query/fragment versus fetch aliases, deny/check-only/inherited/unknown history facts and explicit actual-document actions without silently transferring source metadata to alias targets."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Diagramme und Aggregate** — Teilweise geprüft; `ui.charts`, P2.

  Orientierung: Ordner-, Tiefen-, Status- und Timing-Aggregate mit gemeinsamen Filtermengen prüfen.

  Beweisregister: [analysis](analysis-acceptance.md) (PASS), [ui-fixture](../web/verification/ui-fixture-results.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ui.charts",
    "reference_name": "Site structure and timing charts",
    "status": "partial",
    "acceptance_test": "Aggregate folder/depth/status/timing over a fixture dataset with consistent filtered totals.",
    "remaining_acceptance": [
      "Add folder/depth/status/timing aggregation charts with consistent combined filtered totals."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "analysis",
          "ui-fixture",
          "ui-live"
        ],
        "proves": "Graph/depth distribution and page status/timing observations are available with bounded visual graph fixture/live proof."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Noch offen

- [ ] **Sprache, Theme und Schrift wählen** — Offen; `ui.localization`, P2.

  Orientierung: UI-Einstellungen ändern, während gespeicherter Crawl-Text exakt unverändert bleibt.

  Beweisregister: keine zugeordnete ausgeführte Prüfung.

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "ui.localization",
    "reference_name": "Language and UI preferences",
    "status": "planned",
    "acceptance_test": "Switch supported interface locale/theme/font settings while preserving stored crawl text exactly.",
    "remaining_acceptance": [
      "Switch supported interface locale/theme/font settings while preserving stored crawl text exactly."
    ],
    "implementation_evidence": [],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

## Pflege und Beweisgrenzen

1. Die offene Abnahme im JSON als konkrete Fixture oder Runtime-Prüfung ausarbeiten.
2. Positive Fälle, Kontrollfälle, unbekannte Daten und Ressourcen-/Corpus-Grenzen zusammen prüfen.
3. Prüfergebnis, Artefakt, Engine-/Profilversion und den tatsächlichen Geltungsbereich im Katalog festhalten.
4. Erst bei vollständig erfüllter eigener Abnahme den Status auf `implemented` setzen; Referenzparität separat nachweisen.
5. Den Generator ausführen und mit `--check` prüfen. Historische Beweise bleiben erhalten.

Offline-Auswertung mit geändertem CSS-Inhaltsbereich braucht gespeichertes HTML beziehungsweise DOM. Ein bereits ausgewählter Text mit HTML-Hash reicht zur erneuten Auswahl nicht aus. Dieser zusätzliche Workflow bleibt offen, solange keine vollständigen Quellartefakte und entsprechende Offline-Prüfung existieren.

Reproduktionsbindung: SHA-256 des verwendeten Katalogs `7421b88fec45e123e63e6331016ff98506c885d3a9a43c88cedf29ba98319467`. Letzter im Katalog erfasster Beweisstand: `2026-10-14T17:30:09.4173068Z`. Laufende neue Arbeit aktualisiert frühere Beweise nicht.
