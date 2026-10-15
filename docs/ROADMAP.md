## Aktuell: Exportfix, Browser und kalte Wiederöffnung bestanden (15.10.2026)

Source11 besteht **1384 Tests, Format/Clippy und den optimierten Windows-Build**. Vier frische praktische Prüfbereiche sind abgeschlossen: native API/CLI, 13 UI-Mockgruppen, alle 27 Browsergruppen und der vollständige Resource-/Directory-/Ngram-Ablauf mit kalter Wiederöffnung ohne neue Website-Aufrufe. Die zuvor fehlerhafte Exportzählung ist jetzt korrekt: warm sechs, kalt zwei vollständig abgeschlossene Produzenten, keine Abbrüche oder Fehler. [Aktuelle Rohbelege](verification/source11-native-four-20261015/index.json), [Suite und Build](verification/source11-suite-build-20261015/index.json), [Exportfix und erste Fehler](verification/source11-directed-checkpoint-20261015/index.json). Sieben weitere praktische Prüfbereiche und die sichere Installation folgen. Installiert bleibt Source5 R3. Optionale Crawl-Grenzen, weitere Ngramkanäle und große Leseperformance werden parallel vorbereitet. Katalog unverändert: 31 eigene Abnahmen erfüllt, 69 teilweise geprüft, 44 offen; Herstellerparität `NOT_PROVEN`.

## Aktuell: Ngram-Warmprüfung besteht, echter Antwort-Abschlussfehler entdeckt (15.10.2026)

Source10 besteht1380Tests und optimiertenBuild. Sechs warmeResource-/Directory- und fünf neueNgramgruppen mit206gespeicherten Seiten,205Quellen/410Sechswortvorkommen,API/CLI/vollständigerCSV/UI1440/320 bestehen. Der Gesamtlauf bleibtFAIL: Die App zählt sechs vollständig gelesene Content-Length-Downloads beim regulären Beenden alsabgebrochen. Eine einzelne echteHTTP-Antwort reproduziert dies. [Rohbelege und Quellentrace](verification/source10-response-registry-20261015/index.json). Rust-Lebenszyklusfix mit Negativfällen und CSS-Umbruchfix werden gemeinsam neu geprüft; kalte Wiederöffnung und Installation bleiben offen. Die reineCSS-Source11-Vorbereitung ist unbenutzt und wird ersetzt. Katalog31/69/44; Herstellerparität NOT_PROVEN.

## Aktuell: Windows-Build bestanden, konkreter Browserfehler eingegrenzt (15.10.2026)

Source10 besteht1380 Tests und den optimierten Windows-Build.26 praktische Browsergruppen bestehen; Originaldaten laden jetzt bei1440/320. Ein langes JSON-Beispiel verursacht danach im Einstellungsdialog320px-Überlauf. Eine einzelne CSS-Umbruchregel besteht die tatsächliche Vorschau; neu kompilierte Prüfung folgt. [Build, Fehler und Diagnose](verification/source10-native-first-20261015/index.json). Weitere native Abnahmen und Installation bleiben offen; installiert istSource5R3. Herstellerparität NOT_PROVEN.

## Aktuell: vollständige Source10-Suite bestanden (15.10.2026)

Source10 besteht tatsächlich1380 Tests,598Pflichtnamen/602qualifizierte Namen, vier echte Renderer-Gates sowie Format und Clippy. Alle drei Cargo-Prozesse und die eigenen Dienste sind geschlossen, alle369 Quellinputs abschließend exakt. [Rohbelege](verification/combined-workspace-v10-20261015/index.json), [Ngram-Bedienung](saved-ngram-reports.md). Der optimierte Build läuft; frische native Abnahmen und sichere Installation folgen. Katalog unverändert31 umgesetzt/69teilweise/44geplant. Weitere Ngramkanäle/Wortwolke, große Leseperformance, nachvollziehbare Aufnahmegrenzen und Herstellerparität bleiben offen.

## Aktuell: N-Gramm-Corpus integriert, neue Tests bestanden (15.10.2026)

Source10 besteht alle21 neuen Rustfälle (15Ngram/6SQLite-Projektion), Format und Clippy. Der Originaldaten-Auswahlfix besteht zusätzlich3 separate Node-Regressionen. Der private Quellcommit istc6bd9c5. [Rohbelege inklusive erstem Compilerfehler](verification/source10-directed-checkpoint-20261015/index.json). Vollständige Suite, optimierter Build, frische praktische Prüfungen und sichere Installation folgen. Katalog unverändert31 umgesetzt/69teilweise/44geplant; zusätzliche Ngramkanäle/Wortwolke, große Leseperformance und Herstellerparität bleiben offen.

## Aktuell: Browsertest findet Auswahlfehler; nächster Batch in Integration (15.10.2026)

Source9 besteht1359 Tests und optimierten Build. Der echte Browserlauf besteht26 Gruppen und findet danach einen deaktivierten Ladebutton für gespeicherte Originaldateien. Die enge Korrektur besteht3 separate Regressionen, während der alte Stand1pass/2fail zeigt. Der Resource-/Directory-Lauf besteht5 Gruppen; seine Sidebar-Prüfung wird nach separatem Nachweis aller35 erreichbaren Navigationen korrigiert. [Rohbelege und offene Grenzen](verification/source9-native-checkpoint-20261015/index.json). Source9 wird nicht installiert. Source10 integriert UI-Fix,1–6-Gramm-Corpus und eine kleine Sitemap-Abfrageverbesserung; vollständige neue Prüfungen folgen. Katalog unverändert31 umgesetzt,69 teilweise,44 geplant; Herstellerparität offen.

## Aktuell: Quellenberichte und Verzeichnisansicht vollständig im Workspace geprüft (15.10.2026)

Source9 besteht tatsächlich1359 Tests, alle577 Pflichtnamen/581 vollständigen Namen, vier echte Renderer-Gates sowie Format und Clippy. [Rohbelege](verification/combined-workspace-v9-20261015/index.json) und [Bedienung](saved-source-reports.md). Der optimierte Build läuft; praktische native Abnahmen und Installation folgen. Verzeichnishierarchie ist jetzt teilweise belegt. Katalog:31 umgesetzt,69 teilweise,44 geplant. N-Gramm-Corpus und Leseoptimierung sind quellengeprüft für den nächsten Batch; Herstellerparität bleibt offen.

## Aktuell: Migration praktisch abgenommen, nächste Funktionslücken in Arbeit (15.10.2026)

Source8 besteht1322 Tests und den optimierten Build. Fünf native Bereiche bestehen, darunter der vollständige achtteilige Migrationsworkflow und Originaldateien bis64MiB. Ein echter320Pixel-Überlauf der Hauptkopfzeile verhindert die Installation; der eng geprüfte Umbruch kommt in den nächsten gemeinsamen Stand. Link-/Ressourcenquellen und die Verzeichnisansicht werden parallel vorbereitet. [Rohbelege einschließlich Fehlern](verification/source8-native-checkpoint-1-20261015/index.json). Katalog:31 umgesetzt,68 teilweise,45 geplant; Herstellerparität offen.

## Aktuell: Migrationsworkflow integriert, 1322 Tests bestanden (15.10.2026)

Der private Source8-Stand unter `e6b1fa2` besteht tatsächlich die vollständige Suite mit 1322 Tests, allen 540 Pflichtnamen und 66 Modulnamen, vier echten Renderern sowie Format und Clippy. [Rohbelege und unveränderte Fehlerhistorie](verification/combined-workspace-v8-20261015/index.json). Der optimierte Build läuft; frische native Browser-, Migrations-, Originaldatei- und Großprüfungen sowie sichere Installation folgen. Migrationsvalidierung ist jetzt teilweise belegt. Katalog: 30 umgesetzt, 69 teilweise, 45 geplant. Herstellerparität bleibt offen.

## Aktuell: Build bestanden, Browserfehler gefunden (15.10.2026)

Source7 besteht die vollständige Suite mit 1297 Tests und den optimierten Build. Fünf native Bereiche bestehen. Die echte Browserprüfung findet nach 26 Gruppen einen Überlauf des Originaldatei-Panels bei 320 Pixeln; eine separate Mockprüfung scheitert vorher an veralteten Manifestverweisen im Testadapter. Beide [Fehlbelege und Erfolge](verification/source7-native-checkpoint-1-20261015/index.json) bleiben erhalten. Der Kandidat wird mit diesem bekannten UI-Fehler nicht installiert; R3 bleibt verfügbar. Der nächste gemeinsame Stand enthält die Produktkorrektur, den Migrationsworkflow und den Teil-Schreibvertrag. Große Sitemaps und Originaldateien werden separat weiter geprüft.

## Aktuell: Source7 besteht die vollständige Suite (15.10.2026)

Der gemeinsame Stand unter `e96d1d1` besteht tatsächlich 1297 Tests ohne Fehler oder ignorierte Fälle, alle 515 Pflichtnamen und 41 vollständigen Modulnamen, vier echte Renderer-Prüfungen sowie Format und Clippy. [Rohbelege](verification/combined-workspace-v7-20261015/index.json). Der optimierte Kandidat und alle frischen Native-/BODY13-/Browser27-Abnahmen sind separate nächste Schritte. Lokal bleibt der geprüfte R3-Stand installiert. Vollständige Herstellerparität bleibt offen.

## Aktuell: Source5 R3 besteht die vollständige Suite (15.10.2026)

Der private Commit `9fa103e` besteht tatsächlich 1256 Tests ohne Fehler oder ignorierte Fälle, alle 474 Pflichtnamen, alle 40 neuen Windows-Fälle und vier echte Renderer-Prüfungen. Format und Clippy sind ebenfalls grün; eigene Server sind geschlossen und Quellen unverändert. Rohbelege: [vollständige Suite](verification/combined-workspace-v5-r3-20261015/index.json).

Der optimierte Windows-Kandidat `9bed8a6` besteht alle acht nativen Prüfbereiche, einschließlich vollständiger 50.000 Einträge vor/nach Wiederöffnung, und ist jetzt lokal installiert. App8787 und Renderer8790 laufen; gespeicherte Historie, neun Assets und26 MCP-Tools sind geprüft. Alle32 Dateiinhalte, ADS, Rechte und stabilen Metadaten bleiben exakt; nur die Schreibzeitpunkte von vier bestehenden SQLite-SHM-Dateien ändern sich durch das Lesen. Vollständige Metadatenidentität und allgemeines sanftes Windows-Prozessende werden nicht behauptet. [Installationsbelege](verification/combined-native-v5-r3-installed-20261015/index.json).

Der reparierte Source6R2-Stand unter `3be3fec` besteht tatsächlich1284 Tests ohne Fehler oder ignorierte Fälle, alle502 Pflichtnamen und28 vollständige Modulnamen, vier echteRenderer-Fälle sowie Format und Clippy. Der ursprüngliche Hänger im echtenQueue-/Abbruchfall undder neuebegrenzteWriteAll-/Flush-Fall bestehen jetzt. [VollständigeR2-Rohbelege](verification/combined-workspace-v6-r2-20261015/index.json). Der frühere [Source6-Fehllauf](verification/combined-workspace-v6-cancel-hang-failed-20261015/index.json) bleibt unverändert erhalten. AlsNächstes wirdSource7 aufdiesen tatsächlichenParent übertragen, vollständig getestet undder gemeinsameoptimierteKandidat mitallenfrischenNativeroutinen geprüft. DieR3-Installation bleibtverfügbar; Herstellerparität bleibt offen.

## Historisch: Source5 gemeinsam prüfen (15.10.2026)

Source5 ist als privater Commit `95cc8ee` mit 317 eingefrorenen Eingaben veröffentlicht. Der tatsächliche vollständige Lauf scheitert mit sieben Compilerfehlern in Testhilfen; noch kein Test ist ausgeführt. Die Reparatur muss anschließend mindestens 1256 Tests, 474 Pflichtnamen und vier echte Renderer-Fälle bestehen. Optimierter Build, Native-Abnahmen und Installation stehen für diesen Stand noch aus.

Der große V4-Lauf ist nach dem tatsächlichen 45-Minuten-Limit fehlgeschlagen. Alle 13 Initialfälle und der vollständige 50.000-Zeilen-Export bestehen; die vollständige Wiederöffnungsprüfung fehlt. Das unveränderte Ergebnis ist unter `verification/combined-native-v4-large-failed-20261015` archiviert. Die neue Leseoptimierung muss denselben Test erneut bestehen.

Nächste Schritte: Original-Sitemap-Dateien über API und Oberfläche zugänglich machen, sauberes Prozessende nativ prüfen, Response-/Connection-Besitz für nicht lesende Clients vervollständigen und danach die übrige Featureliste weiter abarbeiten. Vollständige Herstellerparität ist nicht nachgewiesen.

# Open SEO Spider – Roadmap zur vollständigen Funktionstiefe

Diese Roadmap ordnet die nächsten Arbeitsschritte. Der vollständige laufende Status aller 144 Fähigkeiten steht in [FEATURE_TODO.md](FEATURE_TODO.md); die exakten Abnahmen und Belege stehen in [feature-parity.json](feature-parity.json).

Aktuell: **30 eigene Abnahmen erfüllt, 68 teilweise geprüft, 46 offen**. Zusätzlich bleiben [416 benannte Filter und 32 Ablaufverträge](parity/granular-acceptance.json) getrennt zu prüfen. Alle Hersteller-Parität bleibt **NOT_PROVEN**. Es wird kein Fertigstellungsprozentsatz berechnet.

## Frühere Bausteine und offene Folgearbeiten

**V4-Gesamtlauf bestanden:** [Tatsächlicher vollständiger Workspace](verification/combined-workspace-v4-20261015/index.json):1216/0/0,434 Pflichtnamen, vier echte Renderer-Fälle, Format und Clippy. Der optimierte Build und alle sechs bisherigen nativen Prüfbereiche bestehen, einschließlich zwölf physischer Größenfälle und [26 tatsächlicher Browsergruppen im korrigierten R2](verification/combined-native-v4-browser-r2-20261015/index.json). Der erste Browser-Testfehler bleibt als eigener historischer Beleg erhalten. Die13 großen echten Quellenfälle laufen separat; Installation und weitere Funktionslücken bleiben offen. Die Standardkapazität bleibt32MiB; höhere Mitgliedschaftskapazität bis128MiB muss ausdrücklich konfiguriert werden.

**B9 ist als begrenzte Grundlage abgeschlossen; seine frühere lokale Installation ist historisch belegt.** Der [Aggregatbeleg](verification/resource-discovery-batch.json) bindet 362 Standard-Rust-Tests, API9, MCP22 und tatsächliche UI65. Alle vier [exakten CI-Jobs an Commit f107ce7](verification/resource-discovery-ci.json) sind grün. Der [unveränderliche Windows-Release](verification/resource-discovery-release.json) besteht zusätzlich API9, Capture-/Neustart8, MCP22 und UI12. Die [Installation auf localhost8787](verification/resource-discovery-installed.json) erhält sieben Jobs und elf JSON-Dateien byteidentisch. Die historische Stackoverflow-Reproduktion und ihre Reparatur bleiben getrennt gespeichert. Breitere Feature-Abnahmen bleiben PARTIAL.

**B10 wurde lokal installiert; die aktuelle Installation enthält zusätzlich den B12-Sitemap-Transport.** Der [eigene V2-Vertrag](resource-contract-v2.md) ergänzt ausdrückliche interne/externe Follow-Schalter und unveränderliche Rollen bei jedem Redirect. [216 Schalter-, 44 Antwort- und 200 Redirect-/Boundary-Ausführungen](verification/resource-contract-checkpoint.json), API13 und echte UI10 sind geprüft. [Alle vier exakten CI-Jobs](verification/resource-contract-ci.json), [optimierter Release](verification/resource-contract-release.json) und [Installation auf localhost8787](verification/resource-contract-installed.json) bestehen. Sieben vorhandene Jobs und elf JSON-Dateien bleiben byteidentisch. Weitere Encoding-/Unterbrechungs-/Budget-/Browser-/Master-Schalter-/TLS-Kontrollen bleiben offen. Der [historische Designplan](parity/resource-matrix-plan.md) bleibt unverändert; tatsächliche Beweise sind getrennt.

**Abgeschlossen und installiert: B11.** [Issue18](https://github.com/lia-xim/open-seo-spider/issues/18) verfolgt den eigenen Vergleich gespeicherter Crawls mit konkreten Vorher/Nachher-Werten, vollständigem optionalem Originalbericht-JSON und explizitem URL-Mapping. [Alle vier exakten CI-Jobs an Quellcommit2a81036](verification/saved-comparison-ci.json) sind grün; Windows und Linux bestehen jeweils476 Standard-Rust-Tests. Der [optimierte Windows-Release](verification/saved-comparison-release.json) besteht mit demselben Executable native11/SDK12, vier CLI-Grenzgruppen und13 lokale Browsergruppen. Die [Installation auf localhost8787](verification/saved-comparison-installed.json) erhält sieben Jobs und19 dauerhafte Dateien byteidentisch, fünf Git-Blob-Assets und23 read-only Inventar-Tools. Fehlende Quellen bleiben unbekannt; Hersteller-Parität bleibt NOT_PROVEN.

**In Arbeit: B12 – Sitemaps.** Die [neue Quellen-/Lastmod-Prüfung](sitemap-assessment.md) besteht den gesamten lokalen Workspace794/0 Fehler/0 ausgespart, Format/Clippy und13 tatsächliche API-/CSV-/SDK-/Browsergruppen am optimierten Windows-Release. [Belege](verification/sitemap-assessment-local.json) halten206 Quellinputs,24 MCP-Tools und320px fest; [Lokal auf8787 installiert](verification/sitemap-assessment-installed.json): sieben bestehende Jobs und19 dauerhafte Dateien unverändert, Renderer8790 verfügbar und Backup verifiziert. Rekursive Quellen-Kohorten sind qualifiziert vollständig/leer/unbekannt; Lastmod behält Originale und getrennte XSD-/W3C-Profile. Ziel-/Inlink-Projektionen, weitere physische Budget-Gates und Sitemap-ZIP-Erzeugung bleiben in Arbeit. Der Nutzer erlaubt die lokale Alternative zur GitHub-Actions-Sperre; Remote-/Linux-/Container-CI und vollständige Herstellerparität bleiben NOT_PROVEN.

**Neuer V3-Kandidat noch nicht installiert:** [Optimierter Build und fünf begrenzte native Bereiche](verification/combined-optimized-native-v3-20261015/index.json) bestehen zusätzlich zum vollständigen1203-Test-Lauf. Die große physische Prüfung ist nicht bestanden; nach etwa524 Sekunden wurden140/405 Zeilen erhalten und die eigene Testinstanz ausdrücklich beendet. Die quadratische SQLite-JSON-Projektion wird korrigiert.50.000 dauerhafte Mitgliedschaften,50MiB-Quellen und die restlichen Abnahmen bleiben offen. Die bisherige ab1-Installation und sämtliche Nutzerdaten sind unverändert.

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

Aktualisierung59e8453: Der [lokale formatierte Main-Core-Snapshot](verification/sitemap-ledger-local-510-account-gate.json) besteht510 Unit-Tests, darunter12 gerichtete Discovery-Fälle. Der unveränderte externe Policy-Integrationstest ist noch gesondert auszuführen. Die neue GitHub-CI38355115138 startet wegen einer Konto-/Billing-Sperre keine Job-Schritte und führt null Tests aus. Dieser Kontozustand ist kein Code-Testergebnis; Windows-/Linux-CI, App/native API/UI und neuer Release/Installation bleiben offen. Die installierten Quellen bleiben6597c9c.

Aktueller API/UI-Checkpoint: Die [integrierten Quellen](verification/sitemap-membership-api-ui-local.json) bestehen den vollständigen lokalen Cargo-Workspace736/0 Fehler/4 separat ausgesparte Renderer-Fälle, einschließlich Core510/App112 und unveränderter Policy13. Format und Alltargets-Clippy bestehen. Die [lesende Sitemap-Ansicht](sitemap-membership.md) ist angebunden und besteht zehn isolierte UI/API-Fixture-Gruppen. Geroutete native API/CSV/SDK/Browser- und Ledger-Recovery-Abnahmen, echte Größenkontrollen, Quell-CI und Release/Installation sind separate offene Gates. Installiert bleibt6597c9c.

Aktueller nativer Sitemap-Checkpoint: Die [vollständigen Rohbelege](verification/sitemap-native-checkpoint.json) bestätigen elf Ledger-Gruppen mit elf SQLite-Jobs und elf native API-/CSV-/SDK-/Browsergruppen am selben separat gebauten Windows-Dev-Binary. Pause/Neustart/Resume, Elternvererbung, unbekannte Altbestände, beide MCP-Transporte mit24 Tools und320px sind geprüft. [Versionierte Skripte](sitemap-native-acceptance.md) bestehen ihre tatsächlichen erneuten Läufe; Wiederholungen erhöhen keine Abdeckungszählung. Echte Größen-/Rollback-/Kohortenprüfungen, neue Quell-CI, optimierter Release und Installation bleiben offen. Installierter Stand6597c9c unverändert.

Nächster B12-Abschnitt: Der [Budget-Plan](parity/sitemap-budget-plan.md) führt sieben konkrete offene Aufgaben auf: tatsächliche Kalibrierung, JSON8MiB, CSV64MiB, Zeile256KiB, Gesamtkatalog32MiB, eigene Fehler-Injektion und qualifizierte Veröffentlichung. Der Plan ist an21 tatsächliche Quell-/Lockdateien gebunden, aber noch nicht ausgeführt. Die voreingestellte Million-Zeilen-Grenze kann unter dem32MiB-Katalogcap physisch nicht zuerst erreicht werden; kleinere konfigurierte Core-/Store-Grenzen bleiben eigene Integrationsprüfungen.

Lokale Installation nach ausdrücklicher Nutzerfreigabe: [optimierter Windows-Release/Installation](verification/sitemap-local-release-installed.json) besteht Workspace740/0 Fehler/0 ignoriert, echte Renderer17, Format/Clippy und separate native Release-Abnahmen. Auf8787 laufen24MCP-Tools; sieben vollständige historische Jobs und19 dauerhafte Dateien bleiben unverändert, fünf Git-Assets geprüft, vorhandener Renderer erhalten und Backup verifiziert. GitHub/Linux/Container sind kein bestandener Teil dieser lokalen Ausnahme. Physische Budget-/Kohorten-/übrige Feature-Abnahmen bleiben offen.
