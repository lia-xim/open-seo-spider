# Open SEO Spider – Feature-To-do-Liste

> Automatisch aus [feature-parity.json](feature-parity.json) erzeugt. Aktualisieren: `node scripts/update-feature-todo.mjs`. Unveränderten Stand prüfen: `node scripts/update-feature-todo.mjs --check`.

Die Liste hält alle 144 gruppierten Fähigkeiten fest. Ein Haken bedeutet, dass das **eigene gespeicherte Abnahmekriterium** mit Beweisen erfüllt ist. Er bedeutet keine vollständige Gleichheit mit Screaming Frog. Diese Referenzparität bleibt für alle Einträge **NOT_PROVEN**.

Aktueller Katalogstand: **36 eigene Abnahmen erfüllt**, **68 teilweise geprüft**, **40 offen**. Die Zahlen sind keine Aussage über den Anteil aller Herstellerfunktionen.

Die [416 benannten Filter und 32 Ablaufverträge](parity/granular-acceptance.json) bilden ein zusätzliches Register. Ihre positiven Fälle, Kontrollfälle, Parameter und Grenzfälle müssen jeweils geprüft werden. Ein implementierter Kern, ein leerer Export oder übereinstimmende CSV-Zellen ersetzen diese Einzelabnahme nicht.

## Aktuelle Arbeit und nächste Schritte

- [x] **Source15 mit dauerhaften Crawl-Limits vollständig abnehmen** — COMPLETED. Source15 Retry2 ist nach1535/0/0-Tests, vier frischen Rendererprüfungen, Release-Build und fünf tatsächlichen Nativrollen samt unabhängigen Rohdaten-Audits lokal installiert. Frische Ready-/Leerlaufprüfung, genaue Dienstbeendigung, drei exklusive Datenkopien und vollständige Berichtsvergleiche über alle acht gespeicherten Crawls bestehen. Dateibytes, Streams, ACLs, Attribute, Erstellzeiten und stabile Metadaten sind exakt; nur15 qualifizierte SHM-Schreibzeitdeltas bleiben. App8787 und Renderer8790 laufen mit bestätigten neuen Identitäten. Private Datenbelege bleiben lokal. Herstellerparität bleibt NOT_PROVEN.
  - [x] Historischer Erststand526bd13 geprüft; aktueller Retry2-Produktstand0bb0724 umfasst weiterhin429 exakt gebundene Quelldateien.
  - [x] Quoten-Produkt-/Kompositions-/Formatreviews und die tatsächlichen Vorprüfungen bestehen.
  - [x] Historischer Ausgangspunkt war Source14; anschließend wurde Source15 tatsächlich installiert und die gespeicherten Crawls wurden vollständig übernommen.
  - [x] Die zwei eng geprüften Testkorrekturen bestehen Formatter, unabhängigen Whitespace-Review, strengen Clippy und drei tatsächlich ausgeführte Inventartests. Privat veröffentlicht; alle 429 Dateien und Git-Blobs geprüft.
  - [x] Der vollständig eingefrorene Retry2-Quellstand besteht den tatsächlichen Gesamtlauf mit 1535 Rust-Tests einschließlich vier frischer Renderer-Prüfungen, Formatprüfung und Clippy. Alle 429 Quelldateien bleiben unverändert.
  - [x] Der tatsächliche optimierte Retry2-Build besteht; die neue Programmdatei ist mit SHA256 und Größe gebunden. Der fehlgeschlagene erste Nativlauf bleibt als fehlgeschlagener Beleg erhalten.
  - [x] Vier aktuelle tatsächliche Regressionsrollen bestehen; die unabhängige API-Prüfung bestätigt alle gespeicherten JSON-/CSV-Zellen, acht reguläre Prozessabschlüsse und die begrenzte synthetische Provider-Grenze.
  - [x] Alle fünf tatsächlichen Nativrollen sind unabhängig geprüft. Der vollständige Quota-Lauf besteht beide UI-Breiten, CSV/JSON/MCP/CLI, Cold-Neustart, fünf explizite Negativkopien und sechs begrenzte SQLite-Leseprüfungen; der Rohdaten-Audit prüft 10.628 CSV-Zellen.
  - [x] Finale V4-Installer-/Cold-Prüfung, tatsächliche Fünfrollen-Ready-Prüfung und Daten erhaltende lokale Installation bestanden; unabhängiger tatsächlicher Audit mit offengelegter Root-/Peer-Datenschutzgrenze.
  - Verträge: [installed-summary.json](verification/source15-installed-20261016/installed-summary.json), [index.json](verification/source15-five-native-audited-20261016/index.json), [index.json](verification/source15-four-native-20261016/index.json), [index.json](verification/source15-build-cli-native-failure-20261016/index.json), [index.json](verification/source15-retry2-suite-20261016/index.json), [index.json](verification/retry2-and-cli-progress-20261016/index.json), [index.json](verification/source15-suite-failure-20261016/index.json), [full-suite-failed.json](verification/source15-suite-failure-20261016/full-suite-failed.json), [index.json](verification/source15-code-integration-20261016/index.json), [frontdoor-source-audit.json](verification/source15-code-integration-20261016/frontdoor-source-audit.json), [installed-summary.json](verification/source14-installed-20261016/installed-summary.json).
  - Gate-Stand: private_code: PASS_ACTUAL_0BB0724_ALL429_BYTES_AND_GIT_BLOBS; source_review: NO_FINDINGS_BOUNDED_SOURCE_REVIEW; preliminary_clippy: PASS_ACTUAL_PREVIEW2; new_rust: PASS_29_FUNCTIONS_ONLY; inherited_node: PASS_9_CASES_ON_429_ONLY; full_suite: PASS_ACTUAL_1535_ZERO_FAILED_ZERO_IGNORED_FOUR_FRESH_RENDERER_FMT_CLIPPY; optimized_build: PASS_ACTUAL_OPTIMIZED_BUILD_CANDIDATE_214EB3CA_46790144_BYTES; native: PASS_ACTUAL_FIVE_ROLES; installation: PASS_ACTUAL_SOURCE15_RETRY2_COLD_COPY_COMPATIBILITY_AND_DATA_PRESERVATION; manufacturer_parity: NOT_PROVEN; retained_fixture_tests: PASS_ACTUAL_ALL_THREE_EXACTLY_ONCE; quota_native: PASS_ACTUAL_V5_17_GROUPS22_NATURAL_CHILDREN99_GET26_BOUNDARIES; native_regressions: PASS_ACTUAL_API_HEAD_MAIN_SITEMAP_FOUR_ROLES; actual_native_independent_audits: ALL_FIVE_NO_FINDINGS_BOUNDED_ACTUAL_NATIVE_AUDIT; actual_installation_independent_audit: NO_FINDINGS_BOUNDED_ACTUAL_INSTALLATION_AUDIT.

- [ ] **Gespeicherte Crawls per CLI sicher fortsetzen** — FULL_SUITE_AND_BUILD_PASS_NATIVE_INSTALL_OPEN. Source16 ist mit431 Produktdateien privat veröffentlicht und eingefroren. Vollständiger Gesamtlauf1546/0/0 einschließlich vier frischer Renderer-Prüfungen, Format und Clippy sowie optimierter Build bestehen. Neuer Kandidat e6a296bb/46850560Bytes. Tatsächliche CLI-Abnahme, fünf aktuelle native Regressionen und lokale Installation bleiben offen; installiert läuft Source15. Herstellerparität bleibtNOT_PROVEN.
  - [x] V1-Reviewbefund und V2-Prüfung mit vollständigen Dateipaaren belegt.
  - [x] Mechanisch geteiltes ResumeOptions-Datenformat; ein lokaler Resume-POST über die vorhandene App.
  - [x] Begrenztes Lesen am bereits offenen Handle, explizite Never-Retry-Policy und elf benannte Rust-Regressionstests vorhanden.
  - [x] Der CLI-Entwurf ist auf die tatsächlichen Retry2-Preimages übertragen und als 431-Dateien-Komposition unabhängig geprüft. Die damals noch unausgeführten elf neuen Tests sind im nachfolgenden tatsächlichen Source16-Gesamtlauf enthalten.
  - [x] Tatsächliche Formatierung mit unabhängiger mechanischer Prüfung, strenger Clippy und alle elf gerichteten Rust-Funktionen bestehen in der isolierten 431-Dateien-Komposition.
  - [x] Vier CLI-Produktpfade plus eigene Rohbyte-Attribute privat veröffentlicht; alle431 Arbeitsdateien, Index- und Commit-Blobs geprüft. Unabhängiger tatsächlicher Publikationsaudit ohne Befund.
  - [x] Vollständiger Source16-Gesamtlauf besteht mit1546/0/0, vier frischen Renderer-Gates, Format und Clippy; alle431 Eingaben unverändert und eigene Testprozesse/Fixture geschlossen.
  - [x] Tatsächlicher optimierter Source16-Build mit drei regulären Kindprozessabschlüssen, unverändertem Suite-Beleg und allen431 unveränderten Quelldateien; Programmdatei und exakt fünf Native-Core-SHA-Felder durch Root gebunden.
  - [ ] Aktuellen gebauten Source16-Kandidaten mit tatsächlichen CLI-Aufrufen, echtem Store, Resume, Quota-Erhöhung, Ablehnungen und Prozessschluss prüfen.
  - [ ] Fünf aktuelle native Regressionen auf der neuen Programmdatei ausführen; ältere Source15-Nachweise bleiben historisch.
  - [ ] Danach Source16 lokal installieren und bestehende Crawls an exklusiven Datenkopien und vollständigen Berichten prüfen.
  - Verträge: [index.json](verification/source16-build-resource-format-20261016/index.json), [index.json](verification/source16-cli-suite-20261016/index.json), [full-suite.json](verification/source16-cli-suite-20261016/full-suite.json), [index.json](verification/source15-build-cli-native-failure-20261016/index.json), [cli-parent-composition-source-review.json](verification/source15-retry2-suite-20261016/cli-parent-composition-source-review.json), [index.json](verification/retry2-and-cli-progress-20261016/index.json), [cli-v2-source-review.json](verification/retry2-and-cli-progress-20261016/cli-v2-source-review.json), [manifest.json](verification/retry2-and-cli-progress-20261016/cli-draft/manifest.json).
  - Gate-Stand: source_review: NO_FINDINGS_BOUNDED_SOURCE_REVIEW; format: PASS_ACTUAL_CURRENT_FULL_SUITE; clippy: PASS_ACTUAL_CURRENT_FULL_SUITE; new_rust: PASS_ALL_ELEVEN_IN_CURRENT_FULL_SUITE; integration: PASS_ACTUAL_PRIVATE_0620_ALL431_INDEX_AND_COMMIT_BYTES; native: NOT_RUN_ON_SOURCE16; installation: NOT_RUN_ON_SOURCE16; manufacturer_parity: NOT_PROVEN; parent_source_composition: NO_FINDINGS_BOUNDED_SOURCE_REVIEW_CURRENT_RETRY2_PREIMAGES; preview_format: PASS_ACTUAL_WITH_INDEPENDENT_MECHANICAL_REVIEW; preview_clippy: PASS_ACTUAL_STRICT_CLIPPY; preview_eleven_tests: PASS_ACTUAL_ELEVEN_ZERO_FAILED_ZERO_IGNORED; full_suite: PASS_ACTUAL_1546_ZERO_FAILED_ZERO_IGNORED_FOUR_FRESH_RENDERER_FMT_CLIPPY; optimized_build: PASS_ACTUAL_SOURCE16_E6A296BB_46850560_BYTES.

- [ ] **Native interne Ressourcen-Crawls mit dauerhaften Quoten verbinden** — ACTUAL_FORMAT_PASS_MECHANICAL_REVIEW_CLIPPY_RUST_NODE_OPEN. Source17 ist an den tatsächlichen vollständigen Source16-Gesamtlauf gebunden und als separate432-Dateien-Komposition unabhängig geprüft. Tatsächlicher Formatter besteht; nur die neue Core-Testdatei ist formatiert,431 andere Dateien bleiben bytegleich. Mechanischer Formatreview, Clippy,13 Core-Integrationstests, neun frische Node-Fälle und spätere vollständige Abnahme bleiben offen.
  - [x] Vorhandenen gemeinsamen Dispatcher, dauerhafte Reservierung vor Abruf, Ressourcenabschluss und Wiederaufnahme in der Quelle geprüft.
  - [x] Unabhängiger begrenzter Quellreview ohne Befund; ursprünglicher Source16-Preview und Source15-Vorgänger bleiben unverändert.
  - [x] Alle431 tatsächlichen Source16-Vorgängerdateien vor/nach und alle432 Kompositions-/Vorformat-/Vorschlagsdateien exakt geprüft; unabhängiger Kompositions- und Producer-Review ohne Befund.
  - [x] Tatsächliche Formatierung besteht; ausschließlich crates/core/tests/resource_quota_fixtures.rs geändert.
  - [ ] Mechanische Formatänderungen unabhängig prüfen, dann strengen Clippy und alle13 neuen Core-Integrationstests tatsächlich ausführen.
  - [ ] Alle vier betroffenen CDN-Frontendfälle frisch prüfen; neun historische Node-Fälle sind keine frischen Source17-Tests.
  - [ ] Eigener vollständiger Gesamtlauf, Build, Ressourcen-/Quoten-Nativmatrix mit Pause/Cold/Fehlerfällen und Installation.
  - [ ] Renderer-, Selected- und externe CDN-/Ressourcenziele bleiben gesonderte noch offene Kombinationen.
  - Verträge: [index.json](verification/source16-build-resource-format-20261016/index.json), [resource-quota-proposal-source-review.json](verification/source16-cli-suite-20261016/resource-quota-proposal-source-review.json).
  - Gate-Stand: source_review: NO_FINDINGS_BOUNDED_SOURCE_REVIEW; format: PASS_ACTUAL_FORMAT_COMMAND_ONLY; clippy: NOT_RUN; Rust13: NOT_RUN; Node_current: NOT_RUN; full_suite: NOT_RUN; build: NOT_RUN; native: NOT_RUN; installation: NOT_RUN; manufacturer_parity: NOT_PROVEN; formatter_review: PENDING_AT_THIS_CHECKPOINT.

- [ ] **Eigenständige API-Anreicherung ohne lokalen Websitecrawl** — INSTALLED_SOURCE14_BOUNDED_NATIVE_PASS_PROVIDER_BREADTH_OPEN. Source13 API-only-Modus ist installiert. Die vollständige1476/0/0-Suite sowie echte Frontdoors und ausdrücklich synthetische Saved-Consumer über API/CLI/MCP/Browser/Cold bestehen. Der eine produktive anonyme PageSpeed-Aufruf antwortete429; sein echtes Teilergebnis bleibt nach Neustart identisch. Erfolgreiche Metrics, Refresh, aktiver Provider-Cancel, erfolgreicher Enrichment-CLI und weitere Anbieter bleiben offen.
  - [x] Unabhängiger Vollreview plus Closure von CSV-Spalte1, expliziten Lighthouse-Laufzeitfehlern/fehlenden Metriken, normalisierter8192B-URLgrenze vor Jobaufnahme und Warnungsgate.
  - [x] Fünf Node-VM-Hilfsfälle tatsächlich mit exakten Namen und natürlichem Prozessabschluss bestanden; alle18 Vorschlags- und388 Elternquellen unverändert.
  - [x] Finaler Zwei-Dateien-Fixturestand: tatsächlicher Formatter, unabhängiger Byte-/Tokenreview, Clippy ohne Warnungen und alle 22 vorhandenen MCP-/Lifecycle-Tests mit regulärem Prozessabschluss bestanden. Keine neuen Testnamen oder gelockerten Produktgrenzen.
  - [x] Source13 privat veröffentlicht/eingefroren; tatsächliche1476/0/0-Suite und Releasebuild bestanden.
  - [x] Echte native Frontdoors und ausdrücklich synthetische gespeicherte Consumer über Browser/API/CLI/MCP sowie drei normale EOF-/Cold-Läufe bestanden; vollständige208 CSV-Zeilen einschließlich Header.
  - [x] Source13 lokal installiert; ein produktiver anonymer Provider-Aufruf mit genau einem Versuch erhielt429 und wurde über API/CSV/MCP warm/kalt konsistent gelesen.
  - [ ] Prove successful production-provider metrics and independent response observations; the one anonymous attempt returned HTTP429.
  - [ ] Prove explicit provider refresh, active in-flight cancellation and successful enrich-urls CLI JSON/CSV using the real accepted job path.
  - [ ] Implement and verify the remaining documented providers and separate CrUX/local Lighthouse behavior; one PageSpeed adapter does not complete provider breadth.
  - Verträge: [installed-summary.json](verification/source14-installed-20261016/installed-summary.json), [api-native.json](verification/source13-api-native-progress-20261016/api-native.json), [actual-format.json](verification/source12-thirteen-scopes-20261016/next-fixtures/actual-format.json), [index.json](verification/api-runtime-pass-20261016/index.json), [index.json](verification/api-only-rust-failure-20261016/index.json), [index.json](verification/next-feature-reviewed-20261016/index.json).
  - Gate-Stand: source_review: NO_FINDINGS_BOUNDED_SOURCE_REVIEW; node: PASS_FIVE_API_ONLY_FRONTEND_HELPER_CASES_ONLY; rust: PASS_TWENTY_NEW_RUST_FUNCTIONS_ONLY; provider: ONE_ACTUAL_HTTP429_ATTEMPT_NO_METRIC_SUCCESS; native: PASS_NATIVE_FRONTDOORS_AND_DECLARED_FIXTURE_READS_ONLY; installation: PASS_ACTUAL_SOURCE14; manufacturer_parity: NOT_PROVEN; format: PASS_FORMAT_COMMAND_ONLY; clippy: PASS_PRELIMINARY_CLIPPY_ONLY; inventory_fixture_format: PASS_FORMAT_COMMAND_ONLY; inventory_fixture_retained_tests: PASS_TWENTYTWO_RETAINED_MCP_LIFECYCLE_FUNCTIONS_ONLY; inventory_fixture_clippy: PASS_PRELIMINARY_CLIPPY_ONLY; full_suite: PASS_ACTUAL1506_WORKSPACE_FOUR_FRESH_RENDERER; optimized_build: PASS_ACTUAL_WINDOWS_RELEASE.

- [ ] **CDN-Verzeichnisse nachvollziehbar klassifizieren** — INSTALLED_SOURCE14_FOUR_NATIVE_ROLES_PASS_BOUNDED. Source14 ist nach erfolgreicher1506/0/0-Suite, vier frischenRendererprüfungen, optimiertem Build und vier praktischen Abnahmerollen lokal installiert. Segment-/CDN-Verzeichnisprüfung:18Gruppen,205Seiten,2435Issues, API/CLI/beideMCP-Zugänge, echteUI1440/320, Stale-UI undCold. Die Installation hat alte/neue Starts auf getrenntenDatenkopien,30Assets,28MCP-Werkzeuge und alle8gespeichertenCrawls geprüft. Dateibytes,Streams,ACLs und stabileMetadaten sind exakt erhalten; ausschließlich explizit qualifizierte SQLite-SHM-Schreibzeiten unterscheiden sich. Herstellerparität bleibtNOT_PROVEN.
  - [x] Die echte Ressourcen-fixture zweimal mit deaktivierter/aktivierter Klassifikation und identischen Fetchfreigaben geprüft; Klassifikation erteilt keine zusätzlichen Anfragen.
  - [x] Gemeinsame Source14 mit31 Produktpfaden privat veröffentlicht; aktuelle30 Rust-/9 Frontendfälle bestehen.
  - [x] Vollständige Source14-Suite: 1506 bestanden, 0 fehlgeschlagen, 0 ignoriert; alle aktuellen Pflichtnamen, vier frische Rendererfunktionen, Format/Clippy und eigene Prozess-/Fixtureabschlüsse geprüft.
  - [x] Optimierter Source14-Build und drei frische Nativrollen bestanden: elf Head/CDN-Fälle, qualifizierte API-only-Teilabnahme und acht Sitemap-Szenarien.
  - [x] Im ersten Segment-/CDN-Verzeichnislauf bestehen zehn Gruppen; der fehlgeschlagene MCP-Großexport und die vollständig geschlossenen eigenen Prozesse bleiben als Fehlernachweis erhalten.
  - [x] Korrigierter Test prüft alle MCP-Seiten und Issues, eigene kleine vollständige Auswahl sowie echte Größenrefusals; keine Produkt- oder Budgetänderung.
  - [x] Dritter tatsächlicher Lauf: Segment- und Links-CSV mit getrennten CRLF-/LF-Verträgen bestanden; zwölf Gruppen PASS. Negativimport scheitert vor IO an der eigenen64-KiB-Grenze. Alle eigenen Ressourcen geschlossen, keine Stoppsignale, Lock freigegeben.
  - [x] Aktueller V8-Lauf besteht alle18Gruppen einschließlich echterUI1440/320, sämtlicher gespeicherterZeilen/Exports, Stale-UI, Altformat-Negativkontrolle, Cold und genau bezeugtem Browser-/Prozess-/Pipeabschluss. Historische Fehlversuche bleiben erhalten.
  - [x] Finale Installer-/Cold-V4-Reviews ohne Befund, tatsächliche lesende Vierrollen-Bereitschaftsprüfung und lokale Source14-Installation mit acht gespeichertenCrawls erfolgreich. Private Datenbelege bleiben lokal.
  - [ ] Offene Herstellerdetails und noch nicht implementierte Funktionsgruppen weiter nach Katalog abarbeiten; erfolgreiche lokale Abnahme beweist keine vollständige Herstellerparität.
  - Verträge: [installed-summary.json](verification/source14-installed-20261016/installed-summary.json), [index.json](verification/source14-four-native-pass-20261016/index.json), [index.json](verification/main-negative-cap-and-reference-20261016/index.json), [index.json](verification/quotas-tested-main-retry-20261016/index.json), [main-first-native-failed.json](verification/quotas-tested-main-retry-20261016/main-first-native-failed.json), [index.json](verification/source14-three-native-progress-20261016/index.json), [index.json](verification/source14-full-suite-20261016/index.json), [full-suite.json](verification/source14-full-suite-20261016/full-suite.json), [index.json](verification/source14-code-sitemap-matrix-progress-20261016/index.json), [cdn-directory-rust12.json](verification/installed12-source13-progress-20261016/cdn-directory-rust12.json), [cdn-directory-node4.json](verification/installed12-source13-progress-20261016/cdn-directory-node4.json).
  - Gate-Stand: full_suite: PASS_ACTUAL_1506_0_0_FOUR_FRESH_RENDERER_FMT_CLIPPY; native: PASS_ACTUAL_FOUR_ROLES_QUALIFIED_SCOPES; manufacturer_parity: NOT_PROVEN; optimized_build: PASS_ACTUAL_SOURCE14; installation: PASS_ACTUAL_SOURCE14_COLD_COPY_COMPATIBILITY_AND_DATA_PRESERVATION.

- [ ] **Gespeicherte Segmente über Reports und Exporte anwenden** — INSTALLED_SOURCE14_FOUR_NATIVE_ROLES_PASS_BOUNDED. Source14 ist nach erfolgreicher1506/0/0-Suite, vier frischenRendererprüfungen, optimiertem Build und vier praktischen Abnahmerollen lokal installiert. Segment-/CDN-Verzeichnisprüfung:18Gruppen,205Seiten,2435Issues, API/CLI/beideMCP-Zugänge, echteUI1440/320, Stale-UI undCold. Die Installation hat alte/neue Starts auf getrenntenDatenkopien,30Assets,28MCP-Werkzeuge und alle8gespeichertenCrawls geprüft. Dateibytes,Streams,ACLs und stabileMetadaten sind exakt erhalten; ausschließlich explizit qualifizierte SQLite-SHM-Schreibzeiten unterscheiden sich. Herstellerparität bleibtNOT_PROVEN.
  - [x] SegmentV2 und gesamte418-Input-Komposition unabhängig geprüft.
  - [x] Tatsächlicher Formatter und unabhängiger Review aller418 Vorher-/Nachherdateien bestanden.
  - [x] Zwei reine Error-Unwrap-Testaufrufe eng korrigiert und unabhängig geprüft; alle417 übrigen Vorformatquellen exakt.
  - [x] Formatter, unabhängiger Formatreview, Clippy und alle neun Frontendfälle tatsächlich bestanden.
  - [x] Alle30 neuen Rustfunktionen tatsächlich ohne Fehler oder ignorierte Tests bestanden.
  - [x] Source14 mit31 Produktpfaden privat veröffentlicht; alle418 Rohdateien und Git-Blobs verifiziert.
  - [x] Vollständige Source14-Suite: 1506 bestanden, 0 fehlgeschlagen, 0 ignoriert; alle aktuellen Pflichtnamen, vier frische Rendererfunktionen, Format/Clippy und eigene Prozess-/Fixtureabschlüsse geprüft.
  - [x] Optimierter Source14-Build und drei frische Nativrollen bestanden: elf Head/CDN-Fälle, qualifizierte API-only-Teilabnahme und acht Sitemap-Szenarien.
  - [x] Im ersten Segment-/CDN-Verzeichnislauf bestehen zehn Gruppen; der fehlgeschlagene MCP-Großexport und die vollständig geschlossenen eigenen Prozesse bleiben als Fehlernachweis erhalten.
  - [x] Korrigierter Test prüft alle MCP-Seiten und Issues, eigene kleine vollständige Auswahl sowie echte Größenrefusals; keine Produkt- oder Budgetänderung.
  - [x] Dritter tatsächlicher Lauf: Segment- und Links-CSV mit getrennten CRLF-/LF-Verträgen bestanden; zwölf Gruppen PASS. Negativimport scheitert vor IO an der eigenen64-KiB-Grenze. Alle eigenen Ressourcen geschlossen, keine Stoppsignale, Lock freigegeben.
  - [x] Aktueller V8-Lauf besteht alle18Gruppen einschließlich echterUI1440/320, sämtlicher gespeicherterZeilen/Exports, Stale-UI, Altformat-Negativkontrolle, Cold und genau bezeugtem Browser-/Prozess-/Pipeabschluss. Historische Fehlversuche bleiben erhalten.
  - [x] Finale Installer-/Cold-V4-Reviews ohne Befund, tatsächliche lesende Vierrollen-Bereitschaftsprüfung und lokale Source14-Installation mit acht gespeichertenCrawls erfolgreich. Private Datenbelege bleiben lokal.
  - [ ] Offene Herstellerdetails und noch nicht implementierte Funktionsgruppen weiter nach Katalog abarbeiten; erfolgreiche lokale Abnahme beweist keine vollständige Herstellerparität.
  - Verträge: [installed-summary.json](verification/source14-installed-20261016/installed-summary.json), [index.json](verification/source14-four-native-pass-20261016/index.json), [index.json](verification/main-negative-cap-and-reference-20261016/index.json), [index.json](verification/quotas-tested-main-retry-20261016/index.json), [main-first-native-failed.json](verification/quotas-tested-main-retry-20261016/main-first-native-failed.json), [index.json](verification/source14-three-native-progress-20261016/index.json), [index.json](verification/source14-full-suite-20261016/index.json), [full-suite.json](verification/source14-full-suite-20261016/full-suite.json), [index.json](verification/source14-code-sitemap-matrix-progress-20261016/index.json), [segments-rust30.json](verification/source14-code-sitemap-matrix-progress-20261016/segments-rust30.json), [segments-clippy.json](verification/source13-installed-segments-progress-20261016/segments-clippy.json), [segments-node9.json](verification/source13-installed-segments-progress-20261016/segments-node9.json), [segments-first-clippy-failed.json](verification/source13-api-native-progress-20261016/segments-first-clippy-failed.json), [next-backlog-report.md](verification/installed12-source13-progress-20261016/next-backlog-report.md).
  - Gate-Stand: full_suite: PASS_ACTUAL_1506_0_0_FOUR_FRESH_RENDERER_FMT_CLIPPY; native: PASS_ACTUAL_FOUR_ROLES_QUALIFIED_SCOPES; manufacturer_parity: NOT_PROVEN; source_review: NO_FINDINGS_BOUNDED_SOURCE_REVIEW; format: PASS_FORMAT_COMMAND_ONLY; clippy: PASS_PRELIMINARY_CLIPPY_ONLY; new_rust: PASS_ALL_THIRTY_NEW_FUNCTIONS; node: PASS_ALL_NINE_CURRENT_FRONTEND_HELPERS; private_code: PASS_ACTUAL_D7B8508_ALL418_BYTES_AND_GIT_BLOBS; optimized_build: PASS_ACTUAL_SOURCE14; installation: PASS_ACTUAL_SOURCE14_COLD_COPY_COMPATIBILITY_AND_DATA_PRESERVATION.

- [ ] **Dauerhafte URL-Kontingente je Tiefe, Host und Muster ergänzen** — INSTALLED_CORE_QUOTAS_COMBINATIONS_OPEN. Source15 Retry2 ist nach1535/0/0-Tests, vier frischen Rendererprüfungen, Release-Build und fünf tatsächlichen Nativrollen samt unabhängigen Rohdaten-Audits lokal installiert. Frische Ready-/Leerlaufprüfung, genaue Dienstbeendigung, drei exklusive Datenkopien und vollständige Berichtsvergleiche über alle acht gespeicherten Crawls bestehen. Dateibytes, Streams, ACLs, Attribute, Erstellzeiten und stabile Metadaten sind exakt; nur15 qualifizierte SHM-Schreibzeitdeltas bleiben. App8787 und Renderer8790 laufen mit bestätigten neuen Identitäten. Private Datenbelege bleiben lokal. Herstellerparität bleibt NOT_PROVEN. Positive Kombinationen mit Renderer, Ressourcen und Selected sowie CLI-Wiederaufnahme und genuine Sibling-DNS-Beobachtung bleiben eigene Folgeaufgaben.
  - [x] Vorhandene Grenzwerte und verbleibende Quotenfamilien anhand tatsächlicher418 Quellen abgegrenzt.
  - [x] Alle 35 geänderten/neuen Pfade des V5-Entwurfs unabhängig geprüft; zwei Befunde im Code geschlossen.
  - [x] Separate 429-Dateien-Komposition unabhängig geprüft; tatsächlicher Formatter mit 18 Änderungen ausschließlich an eigenen Rustpfaden bestanden.
  - [x] V6 mit genau drei Clippy-Korrekturen unabhängig geprüft; 33 andere Payloads und 427 Quelldateien gegenüber V5 bytegenau.
  - [x] Tatsächlich gegen 429 ausgeführt: Format und Formatreview, Clippy, alle 29 neuen Rustfunktionen und neun geerbte Frontendfälle bestanden.
  - [x] Nach erfolgreicherSource14-Installation alle35geprüftenProduktpfade gezielt integriert und privatcommitted/gepusht;429Arbeitsdateien undGitBlobs exakt geprüft.
  - [x] Die zwei eng geprüften Testkorrekturen bestehen Formatter, unabhängigen Whitespace-Review, strengen Clippy und drei tatsächlich ausgeführte Inventartests. Privat veröffentlicht; alle 429 Dateien und Git-Blobs geprüft.
  - [x] Der vollständig eingefrorene Retry2-Quellstand besteht den tatsächlichen Gesamtlauf mit 1535 Rust-Tests einschließlich vier frischer Renderer-Prüfungen, Formatprüfung und Clippy. Alle 429 Quelldateien bleiben unverändert.
  - [x] Der tatsächliche optimierte Retry2-Build besteht; die neue Programmdatei ist mit SHA256 und Größe gebunden. Der fehlgeschlagene erste Nativlauf bleibt als fehlgeschlagener Beleg erhalten.
  - [x] Vier aktuelle tatsächliche Regressionsrollen bestehen; die unabhängige API-Prüfung bestätigt alle gespeicherten JSON-/CSV-Zellen, acht reguläre Prozessabschlüsse und die begrenzte synthetische Provider-Grenze.
  - [x] Alle fünf tatsächlichen Nativrollen sind unabhängig geprüft. Der vollständige Quota-Lauf besteht beide UI-Breiten, CSV/JSON/MCP/CLI, Cold-Neustart, fünf explizite Negativkopien und sechs begrenzte SQLite-Leseprüfungen; der Rohdaten-Audit prüft 10.628 CSV-Zellen.
  - [x] Finale V4-Installer-/Cold-Prüfung, tatsächliche Fünfrollen-Ready-Prüfung und Daten erhaltende lokale Installation bestanden; unabhängiger tatsächlicher Audit mit offengelegter Root-/Peer-Datenschutzgrenze.
  - [ ] Renderer-, Ressourcen- und Selected-Kombinationen mit aktivierten Quoten bleiben im ersten Entwurf ausgeschlossen; vollständige Herstellerparität bleibt offen.
  - [ ] CLI-Wiederaufnahme ist als Source16 veröffentlicht und besteht den tatsächlichen Gesamtlauf. Native CLI-Abnahme und Source16-Installation bleiben eigene Folgeaufgaben.
  - [ ] Sibling-DNS/Host-Nativprüfung bleibt ohne tatsächlich vorhandene Resolverbeobachtung offen; kein Portwechsel als Hostbeweis.
  - Verträge: [installed-summary.json](verification/source15-installed-20261016/installed-summary.json), [index.json](verification/source15-five-native-audited-20261016/index.json), [index.json](verification/source15-four-native-20261016/index.json), [index.json](verification/source15-build-cli-native-failure-20261016/index.json), [index.json](verification/source15-retry2-suite-20261016/index.json), [index.json](verification/retry2-and-cli-progress-20261016/index.json), [index.json](verification/source15-suite-failure-20261016/index.json), [full-suite-failed.json](verification/source15-suite-failure-20261016/full-suite-failed.json), [index.json](verification/source15-code-integration-20261016/index.json), [frontdoor-source-audit.json](verification/source15-code-integration-20261016/frontdoor-source-audit.json), [index.json](verification/quotas-tested-main-retry-20261016/index.json), [quotas-v2-rust29.json](verification/quotas-tested-main-retry-20261016/quotas-v2-rust29.json), [index.json](verification/source14-three-native-progress-20261016/index.json), [index.json](verification/source14-full-suite-20261016/index.json), [index.json](verification/source14-code-sitemap-matrix-progress-20261016/index.json).
  - Gate-Stand: source: NO_FINDINGS_BOUNDED_SOURCE_REVIEW; compile: PASS_PRELIMINARY_CLIPPY_ONLY; native: PASS_ACTUAL_FIVE_ROLES; manufacturer_parity: NOT_PROVEN; composition: NO_FINDINGS_BOUNDED_SOURCE_REVIEW; format: PASS_FORMAT_COMMAND_ONLY; formatter_review: NO_FINDINGS_BOUNDED_SOURCE_REVIEW; new_rust: PASS_ALL_29_NEW_FUNCTIONS; node: PASS_NINE_INHERITED_CASES_ON_429_ONLY; full_suite: PASS_ACTUAL_1535_ZERO_FAILED_ZERO_IGNORED_FOUR_FRESH_RENDERER_FMT_CLIPPY; installation: PASS_ACTUAL_SOURCE15_RETRY2_COLD_COPY_COMPATIBILITY_AND_DATA_PRESERVATION; private_code: PASS_ACTUAL_0BB0724_ALL429_BYTES_AND_GIT_BLOBS; retained_fixture_tests: PASS_ACTUAL_ALL_THREE_EXACTLY_ONCE; optimized_build: PASS_ACTUAL_OPTIMIZED_BUILD_CANDIDATE_214EB3CA_46790144_BYTES; quota_native: PASS_ACTUAL_V5_17_GROUPS22_NATURAL_CHILDREN99_GET26_BOUNDARIES; native_regressions: PASS_ACTUAL_API_HEAD_MAIN_SITEMAP_FOUR_ROLES; actual_native_independent_audits: ALL_FIVE_NO_FINDINGS_BOUNDED_ACTUAL_NATIVE_AUDIT; actual_installation_independent_audit: NO_FINDINGS_BOUNDED_ACTUAL_INSTALLATION_AUDIT.

Frühere Zwischenstände bleiben am Ende als eingeklappte Historie erhalten. Ihre damaligen offenen Schritte sind keine zusätzlichen aktuellen Aufgaben; maßgeblich sind die aktuelle Arbeit und die offenen Feature-Kriterien.

Das nächste Häkchen entsteht erst nach nachvollziehbaren Tests der offenen Kriterien und einem passenden Beweis im Katalog. Änderungen an Status oder Abnahme erfolgen dort; diese Datei wird danach neu erzeugt.

## Vollständige Liste nach Gruppen

Die Orientierungssätze sind eine deutsche Arbeitshilfe. Die eingeklappten Quellkriterien enthalten die unveränderte technische Abnahme, alle offenen Kriterien und alle implementierungsbezogenen Belege aus dem Katalog.

### Crawl-Modi und Steuerung

7 Einträge: 6 eigene Abnahmen erfüllt, 1 teilweise geprüft, 0 offen.

#### Teilweise vorhanden – Restarbeiten

- [ ] **URLs ausschließlich über APIs anreichern** — Teilweise geprüft; `modes.apis`, P1.

  Orientierung: API-Anreicherung, fehlende Daten und null Website-Anfragen prüfen.

  Beweisregister: [source13-installed-segments-progress-20261016](verification/source13-installed-segments-progress-20261016/index.json), [source13-api-native-progress-20261016](verification/source13-api-native-progress-20261016/index.json).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "modes.apis",
    "reference_name": "API-only enrichment",
    "status": "partial",
    "acceptance_test": "Enrich supplied URLs using a mock API; record zero website requests.",
    "remaining_acceptance": [
      "Prove successful production-provider metrics and independent response observations; the one anonymous attempt returned HTTP429.",
      "Prove explicit provider refresh, active in-flight cancellation and successful enrich-urls CLI JSON/CSV using the real accepted job path.",
      "Implement and verify the remaining documented providers and separate CrUX/local Lighthouse behavior; one PageSpeed adapter does not complete provider breadth."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "source13-installed-segments-progress-20261016",
          "source13-api-native-progress-20261016"
        ],
        "proves": "Installed Source13 includes API-only PageSpeed adapter, injectable transport success/error/budget/cancel Rust tests in the actual1476 suite, actual import/preview/refusal/API/CLI/MCP/browser consumers, complete207-row saved fixtures and cold reopen with zero dedicated website-trap requests/connections. One actual anonymous production attempt returned429 and its partial report was stored/reopened identically.",
        "limits": "Synthetic native saved fixtures contain unavailable values, not live provider metrics. Real metric success, explicit refresh, active provider cancellation, successful enrich-urls CLI and additional providers remain open."
      }
    ],
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

14 Einträge: 3 eigene Abnahmen erfüllt, 10 teilweise geprüft, 1 offen.

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

- [ ] **Ausgewählte CDN-Hosts intern behandeln** — Teilweise geprüft; `crawl.cdns`, P1.

  Orientierung: Konfigurierte CDN-Grenzen ohne unbegrenzte externe Traversierung ergänzen.

  Beweisregister: [resource-discovery-api-cli](verification/resource-discovery-e2e.json) (PASS), [ui-resource-discovery-live](../web/verification/ui-resource-discovery-live-results.json) (PASS), [resource-discovery-batch](verification/resource-discovery-batch.json) (PASS), [resource-contract-core](verification/resource-contract-core.json) (FOCUSED_CORE_PASS_FULL_MATRIX_PENDING), [resource-contract-api-cli-mcp](verification/resource-contract-e2e.json) (PASS), [resource-contract-ui](verification/resource-contract-ui.json) (PASS), [source13-tested-preview-20261016](verification/source13-tested-preview-20261016/index.json) (PASS_28_NEW_RUST_AND_SIX_NODE_PREVIEW_ONLY), [source13-installed-segments-progress-20261016](verification/source13-installed-segments-progress-20261016/index.json).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "crawl.cdns",
    "reference_name": "CDN classification",
    "status": "partial",
    "acceptance_test": "Classify configured CDN hosts as internal while preventing unrestricted external traversal.",
    "remaining_acceptance": [
      "Implement and prove the declared internal-CDN classification semantics while preventing unrestricted navigation; preserve the existing exact external-resource origin and credential boundaries.",
      "Implement and compare domain-plus-subfolder CDN rules, including descendant and sibling/encoded-path boundaries; the current exact-origin preview rejects paths and does not cover this documented vendor option.",
      "Determine and prove the additional extraction and crawl behavior for CDN-classified URLs against the reference. Reporting classification alone does not establish traversal or extraction parity; keep navigation, resource and credential grants explicit.",
      "Exakte12-Klassen-CDN-AnalysebelegtScopeweitergabe/UI-/Listenentscheidungen;zusätzlicheHTML-FeldnamenundkausaleCDN-ExtraktionsauditsbleibenUNKNOWN. GezieltParser-/Extractor-/Store-Sinksbelegen,bevoreinAbdeckungsclaim erfolgt.",
      "Vier gezielt gelesene Klassen belegen eine Klassifikationsflagge, weitere Guard-Prädikate und konfigurierte Extraktion bis zum typisierten Map-Feld. Konkrete Parsermodi, externe Guard-Semantik und zusätzliche feste SEO-Felder bleiben unbekannt."
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
      },
      {
        "verification_run_ids": [
          "source13-tested-preview-20261016"
        ],
        "proves": "Actual12 Rust functions prove strict optional exact-origin CDN reporting classification and retained original navigation/resource/credential boundaries. Three Node cases prove plain-text user configuration, exact declaration persistence and invalid origin refusal.",
        "limits": "Immutable preview only: active main sources remain Source12 and installed services Source11. Final full suite, native API/CLI/MCP/browser/cold acceptance, safe installation and broader vendor semantics remain open."
      },
      {
        "verification_run_ids": [
          "source13-installed-segments-progress-20261016"
        ],
        "proves": "Installed exact-origin reporting classification passed genuine declaration-only/external-anchor/resource-NoGrant/redirect controls, strict negatives, API/CLI/MCP/UI1440/320 and cold consumers in all11 native cases.",
        "limits": "Navigation/resource/credential grants remain separate. Domain-plus-subfolder reporting is only the next uninstalled418 preview; additional vendor CDN extraction and crawl equivalence remain unproven."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [ ] **Vom Nutzer gewählte Crawl-Grenzen** — Teilweise geprüft; `crawl.limits`, P1.

  Orientierung: Grenzen für Tiefe, Ordner, Query, Subdomains, URL-Länge und Linkanzahl ergänzen und einzeln prüfen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [renderer](../renderer/tests/worker.test.mjs) (PASS), [source12-suite-20261016](verification/source12-suite-20261016/index.json) (PASS_COMBINED_WORKSPACE_V12_REAL_RENDERER_FMT_CLIPPY).

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
      },
      {
        "verification_run_ids": [
          "source12-suite-20261016"
        ],
        "proves": "All21 new static-admission functions (16 core and5 CLI/API/MCP)  pass, including genuine501 controls, folder-depth/query-token/normalized-fetch-URL-byte boundaries, policy-vs-transport redirects and early denial. Native UI/CLI/API/Cold acceptance and other quota families remain open."
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

6 Einträge: 1 eigene Abnahmen erfüllt, 5 teilweise geprüft, 0 offen.

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

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS), [combined-workspace-v9-20261015](verification/combined-workspace-v9-20261015/index.json) (PASS_COMBINED_WORKSPACE_V9_REAL_RENDERER_FMT_CLIPPY).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "http.broken_sources",
    "reference_name": "Broken-link source export",
    "status": "partial",
    "acceptance_test": "Export every source occurrence for a broken destination with anchor, rel and location.",
    "remaining_acceptance": [
      "Run actual native saved-crawl separated PageLinks/ResourceLedger API/UI/fullCSV/CLI, real CSS404 sources, unknown/no-grant decisions, stale and cold reopen with zero read-phase website requests."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "api",
          "ui-live"
        ],
        "proves": "Broken-outlink findings identify source pages; all-occurrence links CSV contains source, target, anchor, kind and rel. API/browser exports are tested."
      },
      {
        "verification_run_ids": [
          "combined-workspace-v9-20261015"
        ],
        "proves": "Twenty actual PageLinks/ResourceLedger tests verify per-occurrence sources/positions, separated target scopes, grant revisions, complete transport-set snapshot, same-TX stale checks and full CSV; API/UI and explicit resource CLI are integrated.",
        "limits": "Actual optimized native public crawl/API/CLI/Browser/cold validation remains pending. Retained page links and private resource decisions have separate scopes; this is not complete vendor provenance parity.",
        "source_commit": "374229117c6fcc98d942976aa55444517d40524a"
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Geprüfter eigener Kern

- [x] **Migrationsziele überprüfen** — Eigene Abnahme erfüllt; `http.migration`, P1.

  Orientierung: Alte und erwartete neue URLs aus CSV mit tatsächlichen Endzielen vergleichen.

  Beweisregister: [combined-workspace-v8-20261015](verification/combined-workspace-v8-20261015/index.json) (PASS_COMBINED_WORKSPACE_V8_REAL_RENDERER_FMT_CLIPPY), [source8-native-checkpoint-1-20261015](verification/source8-native-checkpoint-1-20261015/index.json) (FIVE_SCOPES_PASS_BROWSER_AND_MOCK_LAYOUT_FAIL_JUNCTION_PRECHECK_NOT_RUN_PHYSICAL_RUNNING_LARGE_DEFERRED), [source11-native-ten-20261015](verification/source11-native-ten-20261015/index.json) (PASS_TEN_CURRENT_NATIVE_SCOPES_LARGE_RUNNING_NOT_YET_ACCEPTED).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "http.migration",
    "reference_name": "Migration destination validation",
    "status": "implemented",
    "acceptance_test": "Check a CSV of old/new URL expectations and identify mismatched final destinations.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "combined-workspace-v8-20261015"
        ],
        "proves": "Actual private Source8 commit and complete1322-test run verify offline CSV mapping, duplicates, ambiguous/missing observations, actual HTTP redirect/failure semantics, strict snapshot/cursor/limit handling and explicit new CLI output semantics. API/CLI/UI workflow code is integrated.",
        "limits": "Fresh public-crawl/API/CLI/browser workflow, held file/response regressions, cold reopen and installed same-candidate acceptance remain pending. Unit fixtures are not new website observations.",
        "source_commit": "e6b1fa2f87d0b39ac7f83aba6cdecc2110de5a50"
      },
      {
        "verification_run_ids": [
          "source8-native-checkpoint-1-20261015"
        ],
        "proves": "Actual same-source optimized migration workflow passes all8 groups: genuine redirect crawls,207 mapping rows, strict API pagination/filters/stale handling, complete JSON/CSV, explicit CLI header/index mappings, held real file/response UI races,1440/320 controls and cold reopen. All5 children close naturally; no website reads after crawl.",
        "limits": "Independent bounded migration implementation; no manufacturer internal logic or exhaustive vendor parity. Source8 main-page header layout remains a separate installation blocker.",
        "source_commit": "e6b1fa2f87d0b39ac7f83aba6cdecc2110de5a50"
      },
      {
        "verification_run_ids": [
          "source11-native-ten-20261015"
        ],
        "source_commit": "61b0e3bca5386a17b449915b2915222e245084c5",
        "proves": "Current Source11 actual optimized migration workflow passes all eight API, CLI, browser1440/320, genuine redirects, separately labelled synthetic duplicate, complete207-row export, first-app natural EOF and cold-reopen cases. Five actual children close without signals; reads and cold reopen cause zero website GETs.",
        "limits": "The independent own CSV migration contract is covered. Safe Source11 installation and full vendor equivalence remain unproven."
      }
    ],
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

  Beweisregister: [content-analysis-core](../crates/core/src/content_analysis.rs) (PASS), [content-analysis-api-cli](verification/content-analysis-e2e.json) (PASS), [native-policy-ci](verification/native-policy-ci.json) (PASS), [native-policy-installed](verification/native-policy-installed.json) (PASS), [content-capture-core](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-native-chrome](../crates/core/tests/content_capture_fixtures.rs) (PASS), [content-capture-api-cli](verification/content-capture-e2e.json) (PASS), [ui-content-fixture](../web/verification/ui-content-fixture-results.json) (PASS), [ui-content-live](../web/verification/ui-content-live-results.json) (PASS), [content-capture-batch](verification/content-capture-batch.json) (PASS), [content-capture-ci](verification/content-capture-ci.json) (PASS), [content-capture-installed](verification/content-capture-installed.json) (PASS), [source10-directed-checkpoint-20261015](verification/source10-directed-checkpoint-20261015/index.json) (PASS_21_NEW_FEATURE_NAMES_ONLY), [combined-workspace-v10-20261015](verification/combined-workspace-v10-20261015/index.json) (PASS_COMBINED_WORKSPACE_V10_REAL_RENDERER_FMT_CLIPPY), [source10-response-registry-20261015](verification/source10-response-registry-20261015/index.json) (FAIL_ACTUAL_SOURCE10_RESOURCE_DIRECTORY_WORKFLOW), [source11-native-four-20261015](verification/source11-native-four-20261015/index.json) (PASS_FOUR_CURRENT_NATIVE_SCOPES_SEVEN_PENDING), [source11-installed-generator-20261015](verification/source11-installed-generator-20261015/index.json) (PASS_LOCAL_INSTALL_AND_SIX_GENERATOR_GROUPS), [source12-suite-20261016](verification/source12-suite-20261016/index.json) (PASS_COMBINED_WORKSPACE_V12_REAL_RENDERER_FMT_CLIPPY), [source12-metadata-native-20261016](verification/source12-metadata-native-20261016/index.json) (PASS_ACTUAL_SOURCE12_NGRAM_METADATA_CHANNELS_WORDCLOUD_API_CLI_UI_COLD).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "metadata.ngrams",
    "reference_name": "N-gram frequency",
    "status": "partial",
    "acceptance_test": "Count overlapping one-, two- and three-word sequences with documented stopword and case policies.",
    "remaining_acceptance": [
      "Add separately documented title, heading and inlink text channels; capture needed alt/anchor and unlinked text provenance rather than infer it from flattened saved text.",
      "Add bounded word-cloud and specialized report interactions; verify large-corpus CPU/cancellation/performance and documented capture limits."
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
      },
      {
        "verification_run_ids": [
          "source10-directed-checkpoint-20261015"
        ],
        "proves": "Fifteen actual new Rust tests verify saved-row corpus1..6gram totals, complete source memberships, overlapping bounded position samples, Unicode/case/stopword boundaries, strict budgets/options/snapshots, genuine captured six-word and over200-row API paging/fullCSV and explicit CLI input/output protection.",
        "limits": "Source10 complete workspace/build/nativeUI/205genuine-source/cold/install gates pending. Saved selected raw/DOM text only; no whole-current-site assertion, separate title/heading/inlink/alt/unlinked channels or word cloud.",
        "source_commit": "c6bd9c50a0dfcfb434a7747a2052aab961d00ef7"
      },
      {
        "verification_run_ids": [
          "combined-workspace-v10-20261015"
        ],
        "proves": "Exact Source10 corpus implementation participates in actual1380/0/0 Windows workspace,15newNgramcases,all598required/602qualified names,4realRenderer gates,fmt/Clippy,actual closed worker/fixture and final369-source hash checks.",
        "limits": "Fresh compiled Ngram UI/205genuine sources/cold/install acceptance and complete manufacturer parity remain pending.",
        "source_commit": "c6bd9c50a0dfcfb434a7747a2052aab961d00ef7"
      },
      {
        "verification_run_ids": [
          "source10-response-registry-20261015"
        ],
        "source_commit": "c6bd9c50a0dfcfb434a7747a2052aab961d00ef7",
        "proves": "Actual compiled Source10 five warm Ngram cases on genuine206-page crawl: independent1..6 complete totals,205sources/410sixgram occurrences,API200+5/fullCSV,CLI/protected input and actual UI100+100+5/fullCSV/last-pageJSON/unknownDOM at1440/320.",
        "limits": "Overall native FAIL due exact Content-Length response completion misclassification; cold reopen not run, installation open. Other text channels/wordcloud/performance/full manufacturer parity remain open."
      },
      {
        "verification_run_ids": [
          "source11-native-four-20261015"
        ],
        "source_commit": "61b0e3bca5386a17b449915b2915222e245084c5",
        "proves": "Actual current optimized Source11 corpus API/CLI/fullCSV and compiled browser1440/320 pass on206genuine captured pages,205sources/410sixgram occurrences,independent complete1..6 totals,200+5API and100+100+5UI. Cold reopen preserves all sources/memberships/fullCSV with zero new website requests. Seven actual children close; response registry warm6/cold2 complete,0aborted/failed/retained.",
        "limits": "Safe Source11 installation is pending. Additional title, description, heading, inlink, alt and unlinked text channels, wordcloud and large-corpus performance are not covered by this runtime proof; full manufacturer parity NOT_PROVEN."
      },
      {
        "verification_run_ids": [
          "source11-installed-generator-20261015",
          "source11-native-four-20261015",
          "source12-suite-20261016"
        ],
        "proves": "Source11 saved-body corpus was safely installed with original user data preserved. Source12 adds tested title/description/H1-H6 metadata/capture channels and bounded wordcloud; all17 new channel Rustfunctions and existing corpus tests pass within1428. Source12 native/install and inlink/alt/unlinked-text channels remain open."
      },
      {
        "verification_run_ids": [
          "source12-metadata-native-20261016"
        ],
        "proves": "Actual eight genuine native metadata-channel/wordcloud groups pass with Source12 candidate: raw/DOM identity, None vs complete capture,48channel/N combinations, duplicate/element/Unicode/unknown semantics, full210rowCSV, CLI equivalence, safe UI at1440/320, busy-state barrier and cold reopen; seven direct children naturally closed and reads cause zero website requests.",
        "limits": "This does not complete missing inlink/alt/unlinked-text channels, large-corpus/RSS guarantees, vendor parity or Source12 installation."
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

11 Einträge: 3 eigene Abnahmen erfüllt, 6 teilweise geprüft, 2 offen.

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

- [ ] **Verzeichnis-Hierarchie anzeigen** — Teilweise geprüft; `links.directory_graph`, P2.

  Orientierung: Künstliche Gruppierungspfade von tatsächlich abgerufenen URLs unterscheiden.

  Beweisregister: [combined-workspace-v9-20261015](verification/combined-workspace-v9-20261015/index.json) (PASS_COMBINED_WORKSPACE_V9_REAL_RENDERER_FMT_CLIPPY).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "links.directory_graph",
    "reference_name": "Directory architecture views",
    "status": "partial",
    "acceptance_test": "Build directory hierarchy including synthetic grouping paths and label paths that are not fetched URLs.",
    "remaining_acceptance": [
      "Run actual compiled directory API/UI/CSV with over200 real saved children, synthetic-group separation, strict stale/cursor handling, multiple widths and cold reopen without new website requests."
    ],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "combined-workspace-v9-20261015"
        ],
        "proves": "Actual11 Core+6 App directory tests verify synthetic groups versus saved requested URLs, path/scheme/origin boundaries, status and renderer-unknown semantics, strict snapshot/cursor/filters and complete CSV. API and UI code is integrated.",
        "limits": "Fresh same-candidate native UI, complete over200 paging, Website-IO and cold reopen acceptance remain pending.",
        "source_commit": "374229117c6fcc98d942976aa55444517d40524a"
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

#### Noch offen

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

10 Einträge: 2 eigene Abnahmen erfüllt, 4 teilweise geprüft, 4 offen.

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

- [x] **Ungültige Head-Struktur erkennen** — Eigene Abnahme erfüllt; `quality.html_head`, P3.

  Orientierung: Ungültige Head-Kinder und Auswirkungen auf Metadaten mit Rohmarkup-Herkunft prüfen.

  Beweisregister: [source13-tested-preview-20261016](verification/source13-tested-preview-20261016/index.json) (PASS_28_NEW_RUST_AND_SIX_NODE_PREVIEW_ONLY), [source13-installed-segments-progress-20261016](verification/source13-installed-segments-progress-20261016/index.json).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "quality.html_head",
    "reference_name": "Invalid head elements",
    "status": "implemented",
    "acceptance_test": "Seed invalid head children that affect subsequent metadata; point to observed raw markup.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "source13-tested-preview-20261016"
        ],
        "proves": "Actual16 Rust functions prove bounded explicit raw-HTTP head diagnostics, source positions/excerpts, invalid children and following metadata observations; UI helpers preserve unknown legacy/incomplete state and render stored markup as text.",
        "limits": "Immutable preview only: active main sources remain Source12 and installed services Source11. Final full suite, native API/CLI/MCP/browser/cold acceptance, safe installation and broader vendor semantics remain open."
      },
      {
        "verification_run_ids": [
          "source13-installed-segments-progress-20261016"
        ],
        "proves": "Own invalid-head criterion completed: eight genuine HTTP literal cases with independently fixed byte positions, raw excerpts and following metadata, separately changed rendered DOM, complete API/CLI/MCP/UI/cold consumers and installed Source13. Full1476 suite and actual11-case native proof retained.",
        "limits": "Bounded captured explicit-head source diagnostics only. Incomplete/non200/lossy/legacy states remain Unknown. No Google processing or full manufacturer runtime equality is asserted."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

### Sitemaps

5 Einträge: 5 eigene Abnahmen erfüllt, 0 teilweise geprüft, 0 offen.

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

- [x] **Sitemap-Mitgliedschaft abgleichen** — Eigene Abnahme erfüllt; `sitemap.coverage`, P2.

  Orientierung: Vollständige Mitgliedschaft speichern und Sitemap-only sowie nicht gelistete Crawl-URLs getrennt prüfen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [analysis](analysis-acceptance.md) (PASS), [sitemap-derived-assessment-local](verification/sitemap-assessment-local.json) (PASS_LOCAL_WINDOWS), [combined-optimized-native-v3-20261015](verification/combined-optimized-native-v3-20261015/index.json) (FIVE_NATIVE_SCOPES_PASS_PHYSICAL_FAILED_EXTERNAL_ROOT_ABORT_NOT_INSTALL_ELIGIBLE), [source14-code-sitemap-matrix-progress-20261016](verification/source14-code-sitemap-matrix-progress-20261016/index.json).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "sitemap.coverage",
    "reference_name": "Sitemap coverage and orphan checks",
    "status": "implemented",
    "acceptance_test": "Compare fetched pages and sitemap locations; distinguish not listed, sitemap-only and incomplete crawl.",
    "remaining_acceptance": [],
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
      },
      {
        "scope": "Actual same-source small native and405-page browser findings/complete CSV scope; physical export limits and50000URL/50MiB positives remain open.",
        "verification_run_ids": [
          "combined-optimized-native-v3-20261015"
        ],
        "artifact": "docs/verification/combined-optimized-native-v3-20261015/index.json",
        "reference_parity": "NOT_PROVEN"
      },
      {
        "verification_run_ids": [
          "source14-code-sitemap-matrix-progress-20261016"
        ],
        "proves": "Actual Source13 eight-scenario matrix joins original persisted memberships with fetched pages and observed inlinks; distinguishes not listed, sitemap-only, pending, unavailable, incomplete, genuine empty, historical and active-resume generation states. Complete API/CLI/HTTP+serial stdio MCP/UI1440+320/CSV and cold reopen preserve the results with zero read-triggered website GETs.",
        "limits": "Finite own acceptance only. No whole-website completeness, website-wide orphan proof, arbitrary Internet/protocol exhaustiveness or manufacturer parity. No new Source14 runtime claim."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Sitemap-Felder und Zielzustände validieren** — Eigene Abnahme erfüllt; `sitemap.validity`, P2.

  Orientierung: Lastmod erhalten/prüfen und Redirect-, Non-200- sowie nicht indexierbare Ziele getrennt melden.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [sitemap-derived-assessment-local](verification/sitemap-assessment-local.json) (PASS_LOCAL_WINDOWS), [combined-optimized-native-v3-20261015](verification/combined-optimized-native-v3-20261015/index.json) (FIVE_NATIVE_SCOPES_PASS_PHYSICAL_FAILED_EXTERNAL_ROOT_ABORT_NOT_INSTALL_ELIGIBLE), [source14-code-sitemap-matrix-progress-20261016](verification/source14-code-sitemap-matrix-progress-20261016/index.json).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "sitemap.validity",
    "reference_name": "Sitemap URL validation",
    "status": "implemented",
    "acceptance_test": "Find redirect/non-200/non-indexable URLs and invalid lastmod values in fixture XML.",
    "remaining_acceptance": [],
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
      },
      {
        "scope": "Actual same-source small native and405-page browser findings/complete CSV scope; physical export limits and50000URL/50MiB positives remain open.",
        "verification_run_ids": [
          "combined-optimized-native-v3-20261015"
        ],
        "artifact": "docs/verification/combined-optimized-native-v3-20261015/index.json",
        "reference_parity": "NOT_PROVEN"
      },
      {
        "verification_run_ids": [
          "source14-code-sitemap-matrix-progress-20261016"
        ],
        "proves": "Actual Source13 matrix retains original redirect301 despite final200, actual404/nonindexable/robots observations, original old sitemap membership beside rewritten new fetch identity, malformed XML and retained valid/invalid calendar lastmods. Full consumers and cold reopen passed; independent audit separates raw XML evidence and admitted assessment assertions from response bodies not separately retained.",
        "limits": "Finite own acceptance only. No whole-website completeness, website-wide orphan proof, arbitrary Internet/protocol exhaustiveness or manufacturer parity. No new Source14 runtime claim."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Sitemaps aus gespeicherten URLs erzeugen** — Eigene Abnahme erfüllt; `sitemap.generate`, P2.

  Orientierung: Lastmod, Bilder und Hreflang unterstützen und Ausgabe unabhängig parsen.

  Beweisregister: [core-fixtures](../crates/core/tests/crawl_fixtures.rs) (PASS), [api](verification/api-e2e.json) (PASS), [ui-live](../web/verification/ui-live-results.json) (PASS), [source11-installed-generator-20261015](verification/source11-installed-generator-20261015/index.json) (PASS_LOCAL_INSTALL_AND_SIX_GENERATOR_GROUPS), [source11-suite-build-20261015](verification/source11-suite-build-20261015/index.json) (PASS_ACTUAL1384_WORKSPACE_AND_OPTIMIZED_WINDOWS_BUILD_NATIVE_PENDING).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "sitemap.generate",
    "reference_name": "XML sitemap generation",
    "status": "implemented",
    "acceptance_test": "Export valid XML with optional lastmod, image entries and hreflang; parse output with an independent parser.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "core-fixtures",
          "api",
          "ui-live"
        ],
        "proves": "Basic indexable-page loc XML export escapes URL text, passes core content assertions and downloads through API/live browser."
      },
      {
        "verification_run_ids": [
          "source11-installed-generator-20261015",
          "source11-suite-build-20261015"
        ],
        "proves": "Actual independently parsed native XML/ZIP supports captured lastmod, images and hreflang; API, five CLI children, full UI and cold reopen agree. Separate32 actual full-suite functions cover additional option/boundary/error and MCP cases."
      }
    ],
    "parity_status": "NOT_PROVEN"
  }
  ```

  </details>

- [x] **Sitemaps teilen und indexieren** — Eigene Abnahme erfüllt; `sitemap.split`, P2.

  Orientierung: URL-/Byte-Grenzen, Indexdatei und exakte Segmentmitgliedschaft prüfen.

  Beweisregister: [source11-installed-generator-20261015](verification/source11-installed-generator-20261015/index.json) (PASS_LOCAL_INSTALL_AND_SIX_GENERATOR_GROUPS), [source11-suite-build-20261015](verification/source11-suite-build-20261015/index.json) (PASS_ACTUAL1384_WORKSPACE_AND_OPTIMIZED_WINDOWS_BUILD_NATIVE_PENDING).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "sitemap.split",
    "reference_name": "Split and segmented sitemaps",
    "status": "implemented",
    "acceptance_test": "Split at configured URL/byte bounds, create index and prove segment URL membership.",
    "remaining_acceptance": [],
    "implementation_evidence": [
      {
        "verification_run_ids": [
          "source11-installed-generator-20261015",
          "source11-suite-build-20261015"
        ],
        "proves": "Actual configured URL and byte splits produce independently parsed indexes with exact segment membership; exact/one-byte-under caps and structured refusals are checked. Separate actual full-suite50001-URL split case is preserved."
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
      "Using an authorized property, join page metrics with declared URL matching and preserve absent data as absent.",
      "Own authorized client; preserve raw URL and match reason. Verify dimensional compatibility, pagination, missing rows, no-data vs zero, orphan discovery separately from optional crawl admission."
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
      "Fetch selected dates/dimensions and join clicks/impressions/CTR/position; report API errors and row coverage.",
      "Independent OAuth client, explicit property/date/filter scope; row-coverage metadata, null vs zero, canonical/URL matching and distinct orphan/import/crawl decisions."
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
      "Honor daily/property quotas; retain inspection timestamp and distinguish inspected from queued URLs.",
      "Per-property quota accounting, inspected vs deferred status, inspection timestamp, raw verdicts, structured enhancement issues; no live inspection until authorized property access."
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
      "Preserve mobile/desktop, field/lab distinction, metric units and provider error details.",
      "Current first-provider proposal is remote lab subset only. Add bounded local Lighthouse worker with explicit fetch/auth/profile lifecycle, full audit selection/details and source/resource exports. Add separate CrUX URL/origin/form-factor/period/distribution outcomes; do not infer field INP from lab TBT or treat absent data as zero."
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
      "Use separately authorized credentials; mock fresh/historic datasets, quota exhaustion and URL/domain scopes.",
      "Own authorized integration, bounded typed results, dataset/scope provenance, units/quotas, unavailable and retry/error states."
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
      "Use independent authorized app/API access; mock expiry, quota and metric joins without reusing vendor tokens.",
      "Own app/OAuth or independently entitled API access. No vendor token/app reuse. Explicit endpoint/metric scope, token expiry, quotas, pagination and joins.",
      "Ahrefs country and volume basis: preserve and validate country separately from monthly/average volume, bind both to metric/query provenance, and test invalid values and roundtrips with owned mocks before an independently authorized live provider check."
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
      "Mock authorized URL/domain metric queries with unavailable/quota states.",
      "Independent token profile, selected metrics/scopes, expiry/quota/error handling, typed unavailable outcomes and durable source provenance."
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
      "Use mock OpenAI-compatible/Gemini/Anthropic and a local Ollama endpoint; record model/provider errors separately.",
      "Validate selected models against a live-model-list fixture, offer available models, and distinguish unavailable/retired models from provider failures.",
      "Prove local Ollama image-generation and adjustable request-timeout behavior independently of text-generation coverage."
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
      "Run an authored prompt on selected page text/HTML/extraction; preview exact input and persist typed output.",
      "Round-trip provider-wide system prompts and expose the exact combined system/user input in test preview."
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
      "Query saved vectors and visualize clusters while retaining model/dimensionality provenance.",
      "Content cluster legend: enable and disable displayed clusters through visible legend controls, preserve stored cluster assignments and model provenance, and verify return to the complete graph and exports."
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
      "Enforce explicit rate/spend bounds for an API fixture, record retries and distinguish local from paid providers.",
      "Attribute actual provider token-usage fields to the connected account/model; retain absent usage as unknown."
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

11 Einträge: 2 eigene Abnahmen erfüllt, 5 teilweise geprüft, 4 offen.

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
      "Add XLSX, filtered-export parity, formula-injection escaping and explicit Unicode/newline acceptance.",
      "Prove optional skip-empty behavior for selected multi-report/bulk exports, including mixed empty and nonempty reports."
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

- [ ] **Lokale MCP-Werkzeuge** — Teilweise geprüft; `automation.mcp`, P4.

  Orientierung: Historischer Grundablauf geprüft; neue Dateifunktionen noch abnehmen und Katalog-/Export-/Node-Funktionen ergänzen.

  Beweisregister: [mcp](verification/mcp-e2e.json) (PASS), [native-policy-ci](verification/native-policy-ci.json) (PASS), [native-policy-installed](verification/native-policy-installed.json) (PASS), [combined-optimized-native-v3-20261015](verification/combined-optimized-native-v3-20261015/index.json) (FIVE_NATIVE_SCOPES_PASS_PHYSICAL_FAILED_EXTERNAL_ROOT_ABORT_NOT_INSTALL_ELIGIBLE), [source11-native-ten-20261015](verification/source11-native-ten-20261015/index.json) (PASS_TEN_CURRENT_NATIVE_SCOPES_LARGE_RUNNING_NOT_YET_ACCEPTED).

  <details>
  <summary>Exakte Abnahme, offene Kriterien und Belege</summary>

  ```json
  {
    "id": "automation.mcp",
    "reference_name": "MCP agent interface",
    "status": "partial",
    "acceptance_test": "Perform the basic official-client workflow and cover discoverable reports/bulk exports/field-filter projections, stored content outputs, configured base-directory file tools and explicitly enabled Node/npm processing across supported transports; track each sub-contract separately.",
    "remaining_acceptance": [
      "Expose and prove report, bulk-export, filter and data-field discovery with typed selected-field/format/segment/full-selection exports; existing snapshot and sitemap exports prove narrower contracts.",
      "Implement configured base-directory text read/write, directory discovery/list/create and explicit disabled-by-default Node script/npm capabilities. Prove exact target containment, unsupported paths, defaults, output/error/deadline/actual child drain and retained outputs with independent fixtures.",
      "Prove stored URL content/image/screenshot outputs, stable selected-crawl workflow and same source/config/error behavior using real official SDK clients on both transports. The static26-tool registry and historical11-tool basic workflow are not full reference MCP coverage.",
      "Prove exported documentation of the actual offered API, explicit opt-in server autostart, all multi-file exports and API/analysis progress phases; a static registry is insufficient."
    ],
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
      },
      {
        "scope": "Actual optional file-tool SDK and Windows rejection scope; additional MCP catalog/export/Node workflows remain open.",
        "verification_run_ids": [
          "combined-optimized-native-v3-20261015"
        ],
        "artifact": "docs/verification/combined-optimized-native-v3-20261015/index.json",
        "reference_parity": "NOT_PROVEN"
      },
      {
        "verification_run_ids": [
          "source11-native-ten-20261015"
        ],
        "source_commit": "61b0e3bca5386a17b449915b2915222e245084c5",
        "proves": "Current Source11 official SDK file-tool acceptance and both root-junction CLI refusals pass against the actual optimized candidate. Existing source/hash/ownership guards and no original user-data operations remain in force.",
        "limits": "This file-tool gate does not prove graceful Windows in-flight HTTP shutdown or additional catalog, specialized export and node capabilities."
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
      "Add explicit aggregate usage statistics over multiple snapshots and demonstrate default no-telemetry behavior.",
      "Prove both cumulative and non-cumulative usage views from retained local measurements."
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
      "Send only to an explicitly configured local test SMTP sink; verify summary, change table and attachment error handling.",
      "Prove selected zipped export attachments, oversize/provider refusal, and skipped-empty reports through an explicitly configured local SMTP sink."
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

## Frühere Arbeitsstände

<details>
<summary>36 historische Zwischenstände mit ihren damaligen Ergebnissen und offenen Schritten</summary>

Diese unveränderten Einträge dokumentieren den damaligen Stand. Aktuelle Restarbeiten stehen oben und in den jeweiligen Feature-Kriterien.

- [ ] **Export-Abschlüsse und schmale Einstellungsdialoge korrigieren** — ACTUAL1384_SUITE_BUILD_ELEVEN_NATIVE_AND_GENERATOR_PASS_LOCALLY_INSTALLED. Source11 ist lokal installiert und läuft mit Renderer und allen acht gespeicherten Crawls. Die vollständige1384-Test-Suite, optimierter Windows-Build, elf praktische Prüfbereiche und sechs zusätzliche Sitemap-Generator-Gruppen bestehen. Alle Nutzerdatenbytes und stabilen Metadaten blieben erhalten; ausschließlich Änderungszeiten der fünf exakt inventarisierten SQLite-SHM-Dateien durften sich ändern.
  - [x] Fünf bestehende Dateien aus zwei unabhängig geprüften Vorschlägen integriert; 364 weitere Quellinputs, Cargo-Konfiguration und Lockdatei unverändert.
  - [x] Tatsächlich 20 Exporttests bestanden: 16 bestehende und vier neue Namen, null Fehler und null ignorierte Fälle. Die Regressionen decken den letzten Frame ohne weitere EOF-Abfrage, echte HTTP-Antworten, leere/verkürzte/zu lange Ausgaben sowie späte Fehler, Panik und Abbruch ab.
  - [x] Tatsächlicher All-Targets-Clippy-Abschluss mit Code 0 und abschließend unveränderten 369 Quellhashes. Unabhängige Kompositions- und Formatprüfung ohne offene Befunde.
  - [x] Private Veröffentlichung mit exakten 369 Git-Blob- und Arbeitsdatei-Hashes. Erster Startfehler wegen Windows-Pfadschreibweise ist vor der Suite erhalten und im neuen Freeze ohne Produktänderung korrigiert.
  - [x] Tatsächliche vollständige 1384/0/0-Suite mit vier echten Renderergates, 602 Pflichtnamen/606 vollständigen Namen, regulär beendeten eigenen Diensten und abschließend identischen 369 Eingaben.
  - [x] Optimierter Windows-Kandidat aus genau diesem Stand erstellt; alle drei Buildprozesse schließen mit Code 0, Quell- und Suitebelege sind unverändert und die separate PE-Kopie ist per Header/Bytezahl/SHA geprüft.
  - [x] Frische native API/CLI-Gruppe und 13 UI-Mockgruppen bestehen; alle 27 echten Browsergruppen bestehen einschließlich 320px-Einstellungen und gespeicherter Originaldaten.
  - [x] Vollständiger Resource-/Directory-Ablauf mit allen acht bestehenden Fällen und sechs Ngramfällen besteht. Sieben Prozesse tatsächlich beendet; kalte Wiederöffnung benötigt null neue Website-Aufrufe. Registry zählt warm6/kalt2 vollständig, null Abbruch/Fehler/Rest.
  - [x] Alle zwölf tatsächlichen Größenfälle bestehen: CSV exakt64MiB und JSON exakt8MiB werden vollständig geladen; jeweils ein Byte darüber wird ohne erfolgreiche Teilausgabe abgelehnt. MCP-Datei-SDK und beide Root-Junction-Ablehnungen bestehen.
  - [x] Alle dreizehn Originaldateifälle bestehen einschließlich echter64MiB-Crawl-/Archivquelle, vollständigerDownloads und kalter Wiederöffnung. Alle acht Migrationsfälle und beide regulären EOF-Fälle bestehen mit tatsächlichen Prozess-/Pipeabschlüssen und freigegebenen eigenen Locks.
  - [x] Der vollständige 13-Fälle-Großquellenlauf besteht warm und nach Neustart: 50.000 echte XML-Mitgliedschaften, 250 Fenster à 200 und vollständige CSV pro Durchlauf; tatsächliche Prozess-/Pipeabschlüsse und unveränderte eigene Daten sind belegt.
  - [x] Source11 tatsächlich lokal installiert; vollständige Sicherungen vorhanden, acht gespeicherte Crawls und24 Assets/26 MCP-Werkzeuge geprüft. Alle Datenbytes und stabilen Metadaten identisch; die15 beobachteten Änderungszeit-Abweichungen betreffen ausschließlich fünf vorher exakt inventarisierte SQLite-SHM-Dateien.
  - [x] Sechs zusätzliche Sitemap-Generator-Gruppen bestanden: unabhängig geparste XML-/ZIP-Ausgabe mit lastmod/Bildern/hreflang, konfigurierte Aufteilung/Bytegrenzen, API/CLI-Bytegleichheit, atomarer Exportwettlauf, Browser1440/320 und kalte Wiederöffnung mit null neuen Website-Abrufen.
  - Verträge: [index.json](verification/source11-installed-generator-20261015/index.json), [index.json](verification/source11-native-eleven-20261015/index.json), [index.json](verification/source11-native-ten-20261015/index.json), [index.json](verification/source11-native-four-20261015/index.json), [index.json](verification/source11-suite-build-20261015/index.json), [index.json](verification/source11-directed-checkpoint-20261015/index.json), [index.json](verification/source10-response-registry-20261015/index.json).
  - Gate-Stand: source: PRIVATELY_COMMITTED_61B0E3B_FIVE_PATHS; directed: PASS_20_RESPONSE_STREAM_TEST_NAMES_ONLY; preliminary_clippy: PASS_PRELIMINARY_CLIPPY_ONLY; full_suite: PASS_COMBINED_WORKSPACE_V11_REAL_RENDERER_FMT_CLIPPY; optimized_build: PASS_COMBINED_OPTIMIZED_BUILD_V11; native: PASS_ALL_ELEVEN_REQUIRED_SOURCE11_NATIVE_SCOPES; installation: PASS_SOURCE_V11_COLD_LOCAL_INSTALL_WITH_COPY_COMPATIBILITY; manufacturer_parity: NOT_PROVEN; native_first_four: PASS_FOUR_CURRENT_NATIVE_SCOPES_SEVEN_PENDING; sitemap_generator: PASS_ACTUAL_SOURCE11_SITEMAP_GENERATION_SIX_PAGE_API_CLI_UI_COLD.

- [ ] **Bereinigten gemeinsamen Stand vollständig abnehmen** — ACTUAL_1256_FULL_SUITE_EIGHT_NATIVE_SCOPES_AND_LOCAL_INSTALLATION_PASS. Source5 R3 besteht die tatsächliche1256-Test-Suite, Format/Clippy, optimiertenBuild und alleacht nativen Prüfbereiche. Der vollständige50.000-Einträge-Lauf besteht vor und nachWiederöffnung. Kandidat9bed8a6 ist jetzt lokal installiert; App8787 undRenderer8790 laufen. Alle32Dateiinhalte/ADS/Berechtigungen/stabilenMetadaten sind gleich; ausschließlich dieSchreibzeiten vonvier vorhandenenSQLiteSHM-Dateien ändern sich beimLesen. DieseAusnahme ist ausdrücklich dokumentiert, vollständigeMetadatenidentität wird nicht behauptet.
  - [x] Alle bisherigen Assertions und Testfunktionen erhalten; eine tatsächliche Dateidifferenz gegenüber R2 unabhängig geprüft.
  - [x] Endgültige BODY4/Lifecycle4/Stateless2, drei Zusammenführungen, Formatänderungen und R3-Prüftreiber gebunden.
  - [x] Tatsächliche vorgezogene Clippy-Prüfung ohne Warnungen bestanden.
  - [x] Vollständige frische Suite tatsächlich bestanden; alle drei Cargo-Prozesse mit Exit0 geschlossen und Rohlogs/Renderer-/Quellenbelege archiviert.
  - [x] Tatsächlichen optimierten Build mit Exit0, exklusiver PE-Kandidatenkopie und erneuter vollständiger Quellen-/Suite-Prüfung abgeschlossen.
  - [x] Sechs getrennte Native-Scopes tatsächlich bestanden; jeweilige innere Abnahmen und äußere Prozessende-Belege samt Rohlogs archiviert.
  - [x] Beide frischen EOF-Fälle mit natürlichem Exit0, tatsächlicher Listener-Schließung und unveränderter geprüfter Persistenz bestanden; erster Fehllauf bleibt archiviert.
  - [x] Vollständigen13-Fälle-Großquellenlauf tatsächlich bestanden: beidePhasen/50000Zeilen/250Fenster/vollständigerCSV/alleProzess- undPipe-Enden; finaleDaten-/Quellguards bestehen.
  - [x] Private Gegenprobe: direkte Kopie als Daten-Geschwister im selben C-Elternverzeichnis bewahrt alle33 Einträge/32 Dateien samtBytes, ADS, Zeiten, Attributen und vollständigerSDDL; Quelldaten unverändert. Die vier E-ACL-Scratchversuche und der vorherige E-Sicherungsversuch bleiben fehlgeschlagene Vorprüfungen, kein Programmaustausch.
  - [x] AktuelleKaltinstallation tatsächlich bestanden: vollständigeSicherung undzweiKompatibilitätskopien mitallen33Einträgen/32Dateien exakt; alte/neueVersion lesengleicheHistorie. Alle9Assets/26MCPTools/aktuelleProzessidentitäten/Listener undRenderer geprüft, eigenerLock freigegeben. Originaldateiinhalte gleich,4SHM-Schreibzeiten alsengeRuntimeausnahme protokolliert.
  - [x] Drei frühere fehlgeschlagene Installationsversuche bleiben alsprivateRohbelege erhalten; keiner ersetzte dasProgramm.26synthetischeIntegritätsfälle undaktuellerDrei-Kopien-/Ablehnungstest tatsächlich bestanden.
  - [ ] Source6Originaldatei-API/UI undResponseOwner integrieren, mindestens1283Tests/501Pflichtnamen/fourRealRenderer/Build/allefrischenNativfälle bestehen.
  - [ ] Source7Verbindungsbesitz undübrige144erFeature-/416Filter-/32Workflowliste weiter implementieren; Herstellerparität bleibtNOT_PROVEN.
  - Verträge: [index.json](verification/combined-native-v5-r3-installed-20261015/index.json), [index.json](verification/combined-native-v5-r3-large-pass-20261015/index.json), [index.json](verification/combined-native-v5-r3-six-scopes-20261015/index.json), [index.json](verification/combined-native-v5-r3-eof-r2-pass-20261015/index.json), [index.json](verification/combined-optimized-v5-r3-20261015/index.json), [index.json](verification/combined-native-v5-r3-eof-failed-20261015/index.json), [index.json](verification/combined-workspace-v5-r3-20261015/index.json), [index.json](verification/combined-source-v5-r3-source-20261015/index.json), [index.json](verification/combined-source-v5-r2-clippy-failed-20261015/index.json).
  - Gate-Stand: source_review: PASS_BOUNDED_SOURCE_ONLY; preliminary_clippy: PASS_ACTUAL_29_59_SECONDS; full_workspace: PASS_ACTUAL_1256_ALL_474_ALL_40_NEW_FOUR_REAL_RENDERERS_FMT_CLIPPY; optimized_candidate: PASS_ACTUAL_OPTIMIZED_WINDOWS_9BED_39906816_BYTES; native: PASS_ALL_EIGHT_CURRENT_SCOPES_SIX_SMALL_EOF2_LARGE13_FULL_50000_INITIAL_AND_REOPEN; installation: ACTUAL_PASS_LOCAL_9BED_SERVICES_RUNNING_BYTES_AND_STABLE_METADATA_EXACT_SHM_LASTWRITE_ONLY; manufacturer_parity: NOT_PROVEN.

- [ ] **Korrigierten gemeinsamen Stand vollständig ausführen** — ACTUAL_1256_TESTS_PASS_CLIPPY_FAIL. Der tatsächliche vollständige Testlauf besteht 1256 Tests ohne Fehler oder ignorierte Fälle, einschließlich aller 40 neuen Fälle und vier echter Renderer-Prüfungen. Clippy scheitert an einer einzelnen redundanten Bindung im Testhelfer. Daher ist die gesamte R2-Abnahme fehlgeschlagen; für diesen Stand gibt es keinen Build und keine Installation.
  - [x] Endgültige Vorschläge BODY3, Lifecycle4 und zustandslose Leser V2 unabhängig geprüft; alle 48 Vorschlagspfade und die drei Zusammenführungen exakt gebunden.
  - [x] 36 Formatdifferenzen separat geprüft; gegenüber dem fehlgeschlagenen Source5 sind genau fünf Testdateien geändert, alle bisherigen Assertions und Testfunktionen erhalten.
  - [x] Tatsächlicher vollständiger All-Target-Compilercheck besteht; beide früheren Compilerfehlerstände bleiben erhalten.
  - [ ] Einzelnen Testhelfer korrigieren und frische vollständige Suite auf R3 bestehen.
  - [ ] Danach optimierten Kandidaten bauen und echte Native-/EOF-/Großquellenprüfungen ausführen.
  - [ ] Originaldateien über API/Oberfläche freigeben, blockierte Downloads sauber abbrechen und übrige Featureliste abarbeiten.
  - Verträge: [index.json](verification/combined-source-v5-r2-clippy-failed-20261015/index.json), [index.json](verification/combined-source-v5-r2-source-20261015/index.json), [index.json](verification/combined-source-v5-compile-failed-20261015/index.json).
  - Gate-Stand: source_review: PASS_BOUNDED_SOURCE_ONLY; all_target_check: PASS_ACTUAL_20_44_SECONDS_NO_WARNINGS; full_workspace: TESTS_PASS_1256_CLIPPY_FAIL_OVERALL_FAIL; optimized_candidate: NOT_RUN; native: NOT_RUN_ON_SOURCE5_R2; installation: NOT_RUN_INSTALLED_AB1_UNCHANGED; manufacturer_parity: NOT_PROVEN.

- [ ] **Originaldateien, Aufgabenbesitz und effizientere Sitemap-Leser gemeinsam prüfen** — ACTUAL_CORE_TEST_COMPILE_FAILURE_REPAIR_IN_PROGRESS. Source5 ist privat gespeichert und eingefroren. Der tatsächliche vollständige Lauf endet nach 65 Sekunden Compilerarbeit mit sieben Fehlern in Testhilfen; kein Test wurde ausgeführt. Eigene Renderer-/Fixture-Server sind geschlossen und alle Quellen unverändert. Die eng begrenzte Reparatur erhält sämtliche bisherigen Assertions und erfordert einen neuen vollständigen Lauf.
  - [x] 45 eigene geänderte/neue Dateien exakt übernommen, privat committed und gepusht; alle 304 bisherigen Eingabepfade und 434 bisherigen Pflichtnamen erhalten.
  - [x] 40 neue Windows-Testfunktionen aus den tatsächlichen endgültigen Vorschlägen abgeleitet.
  - [x] Tatsächlichen fehlgeschlagenen Compilerlauf einschließlich sieben konkreter Fehler, Exit101, geschlossenen eigenen Servern und finalen Quellenprüfungen unverändert archiviert.
  - [ ] Tatsächliche vollständige Rust-/Renderer-/Format-/Clippy-Suite bestehen.
  - [ ] Danach optimierten Kandidaten bauen und echte Native-/EOF-/Großquellenprüfungen auf genau diesen Kandidaten ausführen.
  - [ ] Öffentliche Archiv-API und Oberfläche vervollständigen; Installation, weitere Funktionslücken und vollständige Herstellerparität bleiben offen.
  - Verträge: [index.json](verification/combined-source-v5-compile-failed-20261015/index.json), [index.json](verification/combined-native-v4-large-failed-20261015/index.json), [FEATURE_TODO.md](FEATURE_TODO.md).
  - Gate-Stand: source_review: PASS_BOUNDED_SOURCE_ONLY; full_workspace: FAIL_CORE_TEST_COMPILE_101_ZERO_TESTS; optimized_candidate: NOT_RUN_AFTER_COMPILE_FAILURE; native: NOT_RUN_ON_SOURCE5; installation: NOT_RUN_INSTALLED_AB1_UNCHANGED; manufacturer_parity: NOT_PROVEN.

- [ ] **SQL-Skalierung und explizite Sitemap-Kapazität gemeinsam prüfen** — SIX_NATIVE_SCOPES_PASS_LARGE_SOURCE_FAILED_DEADLINE_NOT_INSTALLED. V4 besteht 1216 Rust-Tests, Format/Clippy, den optimierten Build und sechs begrenzte Laufzeitbereiche. Alle 13 großen Quellenfälle bestehen initial; 50.000 URLs wurden über 250 Seiten gelesen und vollständig exportiert. Die vollständige Prüfung nach Neustart scheitert am tatsächlichen 45-Minuten-Budget. Diese Fassung ist nicht installiert.
  - [x] Beide konkreten Quellvorschläge unabhängig geprüft; sechs Rust-Dateien durch cargo fmt formatiert, JavaScript-Syntaxprüfung bestanden.
  - [x] Alle13 zusätzlichen Testfunktionen bestehen im tatsächlichen vollständigen1216-Test-Lauf, darunter50.000 Parser-/SQLite-Mitgliedschaften, Wiederöffnung, Rollback und Konfigurationskontrollen.
  - [x] Vollständiger tatsächlicher V4-Lauf:254 App-,844 Core- und118 Integrationstests bestanden,434 Pflichtnamen und vier echte Renderer-Fälle ausgeführt; Format und Clippy-Dwarnings0.
  - [x] Eigene Renderer-/Fixture-Server tatsächlich geschlossen; alle304 Eingaben und unveränderlichen Review-/Freigabebelege anschließend erneut exakt.
  - [x] Optimierter tatsächlicher V4-Build bestanden; Candidate c38f5743,38.117.376 Bytes, alle drei Cargo-/Compiler-Abschlüsse sowie Quellen-/Suite-Endprüfungen exakt.
  - [x] Native API/CLI/SDK,175 Datei-SDK-Operationsprüfungen, zwei Root-Junction-Ablehnungen und13 ausdrücklich gemockte UI-Gruppen bestehen.
  - [x] Alle zwölf physischen Größenfälle bestehen: CSV exakt67.108.864 Bytes und JSON exakt8.388.608 Bytes vollständig; jeweils+1 atomar mit413 abgelehnt.14 eigene Apps und Fixture tatsächlich geschlossen.
  - [x] Echter Browser-R2 besteht26 Gruppen: alle25 bisherigen sowie Standardkapazität, ausdrücklich128MiB und erneutes Deaktivieren. Request-Schalter und gespeichertes renderer:null getrennt geprüft; Quellstand und Kandidat unverändert.
  - [x] Großer echter Quellenlauf: alle 13 Initialfälle, 50.000 Entry-/Loc-Zeilen, 59.613.930 kanonische Katalogbytes, 250 Ergebnisseiten und vollständiges 57.574.930-Byte-CSV tatsächlich geprüft.
  - [ ] Nach der Leseoptimierung dieselben 13 Fälle einschließlich aller Ergebnisseiten und vollständigem Export nach Neustart erfolgreich wiederholen; initialer Erfolg ersetzt diese Abnahme nicht.
  - [ ] Dauerhafte Original-Sitemap-Dateien BODY01–04 und HTTP-/MCP-Lifecycle-Ownership werden als getrennte Quellvorschläge umgesetzt; weitere öffentliche Schnittstellen und Abnahmen bleiben offen.
  - [ ] Windows-Installation aus verifiziertem Backup und Datenkopien separat prüfen; Linux/Container und vollständige Herstellerparität sind weiterhin nicht bewiesen.
  - Verträge: [index.json](verification/combined-native-v4-large-failed-20261015/index.json), [index.json](verification/combined-native-v4-browser-r2-20261015/index.json), [index.json](verification/combined-native-v4-checkpoint-1-20261015/index.json), [index.json](verification/combined-workspace-v4-20261015/index.json), [index.json](verification/combined-source-v4-draft-20261015/index.json), [index.json](verification/combined-optimized-native-v3-20261015/index.json).
  - Gate-Stand: source_review: PASS_BOUNDED_SOURCE_ONLY; formatter: PASS_CARGO_FMT; javascript_syntax: PASS_NODE_CHECK; full_workspace: PASS_COMBINED_WORKSPACE_V4_REAL_RENDERER_FMT_CLIPPY; optimized_candidate: PASS_COMBINED_OPTIMIZED_BUILD_V4_C38F5743; native: SIX_BOUNDED_NATIVE_SCOPES_PASS; physical: PASS_COMBINED_V4_NATIVE_CONTROLLED_FINDING_CSV64M_APPJSON8M_EXACT_PLUS1; installation: NOT_RUN_INSTALLED_AB1_UNCHANGED; manufacturer_parity: NOT_PROVEN; required_named_cases: PASS_434; actual_renderer: PASS_4; large_source_native: FAIL_ACTUAL_45_MINUTE_DEADLINE_REOPEN_PAGING; native_api_cli_sdk: PASS_SMALL_DIRECTED_FIXTURE; file_sdk: PASS_175_OPERATION_ASSERTIONS; windows_root_junction: PASS_2_NATURAL_REJECTIONS; ui_mock: PASS_13_MOCKED_GROUPS; native_browser: PASS_R2_26_REAL_GROUPS_ORIGINAL_R1_FAILURE_PRESERVED.

- [ ] **Große Sitemap-Ausgaben korrigieren und denselben Kandidaten vollständig abnehmen** — FIVE_NATIVE_SCOPES_PASS_PHYSICAL_FAILED_INSTALLATION_OPEN. Der optimierte V3-Kandidat besteht fünf getrennte Laufzeitbereiche. Der große physische Test ist nicht bestanden: Nach etwa524 Sekunden waren140 von405 Zeilen gesammelt; Root hat nur die eigene Testinstanz beendet. Eine pro Seite erneut ausgeführte SQLite-JSON-Auswertung des ganzen Berichts ist quellseitig bestätigt. Originalinstallation und Nutzerdaten bleiben unverändert.
  - [x] Optimierter Build aus denselben301 Eingaben wie der vollständige1203/0/0-Lauf: tatsächliche Cargo-Abschlüsse, Candidate22f8c602 und endgültige Quell-/Suite-Hashes verifiziert.
  - [x] Native API-/CLI-/offizielle SDK-Prüfung mit kleiner echter Fixture besteht; Datei-SDK besteht175 Operationsprüfungen mit26/31 Inventaren, beiden Transporten und erwarteten Fehlerfällen.
  - [x] Zwei native Root-Junction-Ablehnungen,13 ausdrücklich gemockte UI-Gruppen und25 echte Browsergruppen mit405 gespeicherten Seiten und vollständigem403-Zeilen-CSV bestehen.
  - [x] Fehlgeschlagenen physischen Lauf samt externer Root-Abbruchquittung archiviert. Beide eigenen Apps, der Fixture-Server und der äußere Prozess sind tatsächlich geschlossen; Teilzeilen bleiben lokal erhalten.
  - [ ] Quadratische pro-Seite-Auswertung des gesamten JSON-Berichts reparieren; Originalauswahl, Aktualität, SQL-Fakten und alle Größenorakel erhalten. Neuen Quellstand passend vollständig prüfen und bauen.
  - [ ] Alle zwölf physischen CSV64MiB-/AppJSON8MiB-Fälle einschließlich exakter Grenze und+1 mit dem neuen akzeptierten Kandidaten ausführen. Keine kleineren Ersatz-Fixtures.
  - [ ] 50.000 vollständige dauerhafte Sitemap-Mitgliedschaften und50MiB-Quellen tatsächlich positiv prüfen. Das derzeitige32MiB-Ledger reicht für das serialisierte50.000er-Format nicht aus.
  - [ ] HTTP-In-flight-Drain, Unix/Linux/Container, weitere Ressourcen-/Recovery-/Unknown-Matrizen und Daten erhaltende Installation bleiben separat offen.
  - Verträge: [index.json](verification/combined-optimized-native-v3-20261015/index.json), [index.json](verification/combined-workspace-v3-20261015/index.json).
  - Gate-Stand: optimized_candidate: PASS_SOURCE_V3_22F8C602; native_api_cli_sdk: PASS_SMALL_DIRECTED_FIXTURE; file_sdk: PASS_175_OPERATION_ASSERTIONS; windows_root_junction: PASS_2_NATURAL_REJECTIONS; ui_mock: PASS_13_MOCKED_GROUPS; native_browser: PASS_25_REAL_GROUPS; physical: NOT_PASSED_ROOT_ABORT_140_OF_405_ROWS_0_OF_12_CASES; installation: NOT_RUN_CANDIDATE_NOT_RELEASE_ELIGIBLE; installed_runtime: UNCHANGED_AB1; manufacturer_parity: NOT_PROVEN.

- [ ] **Kombinierten Quellstand vollständig prüfen und als Kandidaten abnehmen** — FULL_WINDOWS_WORKSPACE_PASS_OPTIMIZED_NATIVE_PHYSICAL_FAILED. Der tatsächliche vollständige V3-Lauf besteht mit1203/0/0,421 Pflichtnamen, vier echten Chromium-Gates, Format und Clippy. Der spätere optimierte Build und fünf Laufzeitbereiche bestehen; die physische Größenprüfung ist nicht bestanden und verhindert die Installation.
  - [x] Alle301 Snapshot-/Checkout-Eingaben sind nach tatsächlichem Schließen der eigenen Server bytegleich; Rohprotokolle und Child-Close-Belege sind archiviert.
  - [x] Alle bisherigen419 Pflichtnamen plus zwei neue Typed-Kontrollen tatsächlich ausgeführt; mindestens1.203 Gesamtprüfungen mit null Fehlern und null ignorierten Fällen.
  - [x] Alle vier echten Chromium-Renderer-Fälle und anschließendes Clippy für alle Workspace-Ziele mit-Dwarnings bestehen.
  - [ ] SQL-Skalierungsfehler korrigieren und neuen Quellstand passend vollständig abnehmen; den großen physischen Test unverändert wiederholen.
  - [ ] Linux/Container und Daten erhaltende Installation bleiben getrennt offen.
  - Verträge: [index.json](verification/combined-workspace-v3-20261015/index.json), [proof.json](verification/combined-workspace-v3-20261015/actual-suite/proof.json), [index.json](verification/combined-optimized-native-v3-20261015/index.json).
  - Gate-Stand: full_workspace: PASS_COMBINED_WORKSPACE_V3_REAL_RENDERER_FMT_CLIPPY; required_named_cases: PASS_421; actual_renderer: PASS_4; optimized_candidate: PASS_SOURCE_V3_22F8C602; installed_runtime: UNCHANGED_AB1; manufacturer_parity: NOT_PROVEN; native_physical: NOT_PASSED_EXTERNAL_ROOT_ABORT.

- [ ] **33 Clippy-Meldungen ohne Test- oder Prüfungsabbau korrigieren** — FULL_COMBINED_WORKSPACE_PASS_NATIVE_ACCEPTANCE_OPEN. 16 Core- und17 App-Diagnosen sind in23 Dateien korrigiert. Der nachfolgende tatsächliche V3-Gesamtlauf besteht mit1203/0/0, allen421 Pflichtnamen, vier echten Renderer-Gates, Format und Clippy-Dwarnings. Native Größenabnahme und Installation bleiben offen.
  - [x] Core-Korrektur unabhängig geprüft: geänderte Enum-Payloads, transparente Serde-Boxen, identische externe Profilnamen und erhaltene Fehler-/SQL-/Manipulationsprüfungen.
  - [x] Ersten tatsächlichen Wiederholungslauf archiviert: Core-Meldungen behoben,17 App-Meldungen konkret sichtbar; Format0/Clippy101, tatsächliche Closes und301 Eingaben danach exakt.
  - [x] App-Korrektur unabhängig geprüft: gemeinsame Testhilfe ohne verlorene Testfunktionen, explizites Ende der Mutex-Sperre vor Benachrichtigung, erhaltene CSV-Prefix-/Hash-/Join-/Fehlerprüfungen.
  - [x] Zweite tatsächliche Ausführung besteht Format0 und vollständiges Clippy-Dwarnings0; keine Warnungsunterdrückung oder Testentfernung, beide ursprünglichen Diagnoseprotokolle bleiben erhalten.
  - [x] Nachfolgender tatsächlicher kombinierter V3-Lauf besteht mit 1203/0/0, allen421 Pflichtnamen, vier echten Renderer-Gates, Format und Clippy; siehe separaten V3-Beleg.
  - [ ] Optimierten Kandidaten bauen und separate native API-/CLI-/SDK-/Browser-/physische sowie Plattform- und Installationsprüfungen ausführen.
  - Verträge: [index.json](verification/clippy-corrections-20261015/index.json), [proof.json](verification/clippy-corrections-20261015/app-clippy-correction-check-v1/proof.json), [index.json](verification/combined-workspace-v3-20261015/index.json).
  - Gate-Stand: source_reviews: NO_CONFIRMED_FINDINGS_BOUNDED; draft_fmt_clippy: PASS_ACTUAL; full_new_source_suite: PASS_COMBINED_V3_1203_0_0; installed_runtime: UNCHANGED_AB1; manufacturer_parity: NOT_PROVEN; combined_v3_workspace: PASS_1203_INCLUDING_4_RENDERER_FMT_CLIPPY.

- [ ] **Kombinierten Quellstand vollständig prüfen und als Kandidaten abnehmen** — FULL_WORKSPACE_GATE_FAILED. Alle1.203 tatsächlichen Rust-Tests bestehen mit0 Fehlern und0 ignorierten Fällen, darunter alle421 benannten Pflichtfälle und vier echte Chromium-Renderer-Gates. Anschließendes Clippy endet mit Exit101 und16 Core-Diagnosen; deshalb ist der Gesamtstand noch nicht buildberechtigt. Vollständige Protokolle und alle tatsächlichen Prozess-/Serverabschlüsse sind archiviert.
  - [x] Alle301 Snapshot-/Checkout-Eingaben sind nach tatsächlichem Schließen der eigenen Server bytegleich; Rohprotokolle und Child-Close-Belege sind archiviert.
  - [x] Alle1.203 Rust-Tests,421 Pflichtnamen und vier tatsächlichen Renderer-Gates bestehen; anschließend16 Core-Clippy-Diagnosen konkret archiviert.
  - [ ] Fehlgeschlagenes Gate gezielt reparieren und den neuen Quellstand erneut passend prüfen.
  - [ ] Optimierten Kandidaten aus demselben Quellstand bauen; native API-/CLI-/offizielle MCP-SDK-/echte Browser-/physische Grenzprüfungen ausführen.
  - [ ] Linux-/Container-Prüfung nach Reparatur der lokal nicht registrierten WSL-Installation ergänzen; Docker ist im Windows-Pfad nicht verfügbar.
  - [ ] Daten erhaltende Installation und bestehende Nutzer-Crawls getrennt prüfen; Hersteller-Parität bleibt offen.
  - Verträge: [index.json](verification/combined-workspace-v2-20261015/index.json), [proof.json](verification/combined-workspace-v2-20261015/actual-suite/proof.json).
  - Gate-Stand: full_workspace: FAIL_COMBINED_WORKSPACE_V2; required_named_cases: PASS_421; actual_renderer: PASS_4; optimized_candidate: NOT_RUN; installed_runtime: UNCHANGED_AB1; manufacturer_parity: NOT_PROVEN; rust_workspace: PASS_ACTUAL_1203_0_0; clippy: FAIL_101_16_CORE_DIAGNOSTICS.

- [ ] **Vier Fehler aus dem kombinierten Gesamtlauf beheben** — FULL_COMBINED_WORKSPACE_PASS_NATIVE_ACCEPTANCE_OPEN. Die vier Fehler aus dem historischen1079/4/0-Lauf sind korrigiert;18 gerichtete Funktionen und der spätere vollständige V3-Lauf1203/0/0 einschließlich Renderer/Format/Clippy bestehen. Historische Fehlerbelege bleiben erhalten; native Größenabnahme und Installation sind offen.
  - [x] Vollständigen fehlgeschlagenen kombinierten Lauf archiviert: App251/0/0 und Core828/4/0, tatsächlicher Cargo-Exit101, beide eigenen Server geschlossen und alle301 Eingaben danach bytegleich.
  - [x] Bekannten HTTP-Status nach einer zu langen finalen Redirect-Adresse erhalten; XML-Decoding, Zählung und daraus abgeleitete Aussagen bleiben ausdrücklich unbekannt. Echte Redirect-Regression und zwei neue positive/negative Typed-Kontrollgruppen bestehen.
  - [x] Konfigurationsprofil verlangt ein Objekt. Standardwerte, Teilkonfigurationen, äußeres null, Serialisierung und vorhandene Budgets bleiben erhalten; zehn Core-Profiltests und drei App-DTO-Gruppen bestehen.
  - [x] Zwei vorhandene Tests präzisiert: tatsächliche XML-Reader-Fehlerklasse für numerische Null und gültige Record-/Gesamtkap-Konfiguration vor exakten Bytegrenzen. Alle ursprünglichen Fehler-/Persistenz-/Cleanup-Prüfungen bleiben erhalten.
  - [x] Nachfolgender tatsächlicher kombinierter V3-Lauf besteht mit 1203/0/0, allen421 Pflichtnamen, vier echten Renderer-Gates, Format und Clippy; siehe separaten V3-Beleg.
  - [ ] Optimierten Kandidaten aus demselben Quellstand bauen und native API-/CLI-/SDK-/Browser-/physische sowie Plattformabnahmen ausführen.
  - [ ] Daten erhaltende Installation separat prüfen; die aktuell laufende ab1-Version bleibt unverändert.
  - Verträge: [index.json](verification/combined-core-repairs-20261015/index.json), [proof.json](verification/combined-core-repairs-20261015/failed-full-suite/proof.json), [proof.json](verification/combined-core-repairs-20261015/focused-three-repairs/proof.json), [proof.json](verification/combined-core-repairs-20261015/focused-profile/proof.json), [index.json](verification/combined-workspace-v3-20261015/index.json).
  - Gate-Stand: historical_full_workspace: FAIL_1079_PASS_4_FAIL_0_IGNORE; directed_core_and_app: PASS_18_ACTUAL_FUNCTIONS; full_workspace_after_repairs: PASS_COMBINED_V3_1203_0_0; installed_runtime: UNCHANGED_AB1; manufacturer_parity: NOT_PROVEN; combined_v3_workspace: PASS_1203_INCLUDING_4_RENDERER_FMT_CLIPPY.

- [ ] **CSV-Worker bei abgebrochenen und parallelen Shutdowns vollständig halten** — FULL_COMBINED_WORKSPACE_PASS_NATIVE_ACCEPTANCE_OPEN. Registry, Worker und Supervisor behalten gemeinsame Join-Slots auch bei abgebrochenen oder gleichzeitigen Drains. Zwei echte Crawler-/SQLite-/CSV-Regressionen und der spätere vollständige V3-Lauf bestehen. Die gesonderte große physische Ausgabe und Installation sind noch nicht abgenommen.
  - [x] Unabhängiger begrenzter Source-Review ohne bestätigte Befunde; beide kritischen Originalregionen mit2171 und20684 Bytes unabhängig bytegleich geprüft.
  - [x] Zwei tatsächliche neue Regressionen bestehen: Supervisorabbruch samt Body-/Callerabbruch und abgebrochene/gleichzeitige Drains. Reale Blocking-Rückkehr wird kontrolliert gehalten; vor Freigabe bleibt Drain nachweislich wartend, danach sind beide tatsächlichen Joins beobachtet, Registry leer und Abrechnung vollständig.
  - [x] Ersten irreführenden Cargo-Exit0 mit null ausgewählten Tests durch Namens-/Anzahlguard als FAIL verworfen; ursprüngliche Protokolle erhalten. Bytegleicher Mtime-Refresh nach Windows-Copy erzwingt tatsächliche Kompilierung und grüne zweite Ausführung.
  - [x] Nachfolgender tatsächlicher kombinierter V3-Lauf besteht mit 1203/0/0, allen421 Pflichtnamen, vier echten Renderer-Gates, Format und Clippy; siehe separaten V3-Beleg.
  - [ ] Native API-/CLI-/SDK-/Browser-/physische und Plattformabnahmen auf den neuen Quellstand binden; installierte Version bleibt unverändert.
  - Verträge: [source-handoff.json](verification/csv-registry-lifecycle-20261015/source-handoff.json), [source-review.json](verification/csv-registry-lifecycle-20261015/source-review.json), [index.json](verification/csv-registry-lifecycle-20261015/focused-repaired/index.json), [index.json](verification/combined-workspace-v3-20261015/index.json).
  - Gate-Stand: source_review: BOUNDED_SOURCE_NO_CONFIRMED_FINDINGS; directed_two_groups: PASS_ACTUAL_2_0_0; critical_regions: EXACT_2171_PLUS_20684_BYTES; full_workspace: PASS_COMBINED_V3_1203_0_0; installed_runtime: UNCHANGED_AB1; manufacturer_parity: NOT_PROVEN; combined_v3_workspace: PASS_1203_INCLUDING_4_RENDERER_FMT_CLIPPY.

- [ ] **Sitemap-Compilerfehler beheben und den vollständigen Stand erneut prüfen** — FULL_COMBINED_WORKSPACE_PASS_NATIVE_ACCEPTANCE_OPEN. Der reparierte v3-Stand kompiliert ohne Warnungen. Sein vollständiger Lauf endete mit217 bestandenen und3 fehlgeschlagenen App-Tests; Core, die vier echten Renderer-Gates und Clippy wurden danach nicht ausgeführt. Die drei Fehler sind durch genau4 Testdatei-Korrekturen behoben und alle3 gerichteten Wiederholungen bestehen tatsächlich. Der neue gesamte Quellstand muss weiterhin vollständig geprüft werden.
  - [x] Tatsächlichen fehlgeschlagenen e9-Lauf mit vollständigem Compilerlog, Exit101 und geschlossenem eigenen Browser-Worker archiviert; installierter Release bleibt unverändert.
  - [x] Vier Typfehler und die mehrdeutigen Testimporte korrigiert, ohne Testfunktionen oder Assertions zu entfernen. Unabhängiger Review bestätigt die erhaltenen SQL-/SHA-/Größen-/Gleichheitsprüfungen.
  - [x] Tatsächliches cargo check --workspace --all-targets --locked des fünfdateiligen Zwischenstands besteht; drei gemeldete App-Warnungen danach gezielt bereinigt.
  - [x] Unabhängiger Review entdeckte eine fehlerhafte Entfernung eines geteilten Fixturefelds vor Ausführung; Feld und bestehende MCP-Assertion sind erhalten, gemeinsamer Fehlertext nutzt die Herkunft. Korrigierter v3-Quellabzug ersetzt den nicht ausgeführten v2-Abzug.
  - [x] Korrigierter endgültiger v3-Stand besteht tatsächlich cargo check --workspace --all-targets --locked in23,26 Sekunden ohne Warnungen; alle297 Snapshot-/Checkout-Eingaben danach bytegleich geprüft.
  - [x] Vollständigen v3-Lauf mit217 bestandenen,3 fehlgeschlagenen und0 ignorierten App-Tests samt vollständigem Log und geschlossenem eigenen Browser-Worker archiviert. Core-/Renderer-/Clippy-Abnahme wurde dadurch nicht erreicht.
  - [x] Genau4 Testdateien repariert: eindeutiges26er-MCP-Inventar, echte nichtleere Abbruchschreibprobe und wohldefinierter SQLite-Read-only-Start vor den6 unveränderten Korruptionsfällen. Alle3 gezielten Testwiederholungen bestehen mit jeweils1 bestanden,0 fehlgeschlagen,0 ignoriert; alle297 Eingaben danach bytegleich.
  - [x] Nachfolgender tatsächlicher kombinierter V3-Lauf besteht mit 1203/0/0, allen421 Pflichtnamen, vier echten Renderer-Gates, Format und Clippy; siehe separaten V3-Beleg.
  - [ ] Optimierten Kandidaten bauen und native API-/CLI-/SDK-/Browser-/physische Abnahmen ausdrücklich auf den reparierten Quellstand binden; alte e9-Treiber bestätigen diese geänderten Bytes nicht.
  - [ ] Plattform- und installierte Laufzeitabnahmen getrennt abschließen; keine vollständige Hersteller-Parität behaupten.
  - Verträge: [index.json](verification/sitemap-source-ui-suite-failed-20261015/index.json), [check.json](verification/sitemap-compile-repair-20261015/check.json), [index.json](verification/sitemap-compile-repair-20261015/focused-app-regressions/index.json), [index.json](verification/combined-workspace-v3-20261015/index.json).
  - Gate-Stand: historical_e9_full_suite: ACTUAL_COMPILE_FAIL_101_ZERO_TESTS_WORKER_CLOSED; intermediate_five_file_check: PASS_WITH_THREE_APP_WARNINGS; final_eight_file_full_suite: FAIL_APP_217_PASS_3_FAIL_0_IGNORE_CORE_RENDERER_CLIPPY_NOT_RUN; final_source_manifest_sha256: 0ab151e4fd06f4603d8429bed4c784f7045e5c838bec378706d6c01bfaefb7bf; installed_runtime: UNCHANGED_AB1; manufacturer_parity: NOT_PROVEN; final_eight_file_check: PASS_NO_WARNINGS_23_26_SECONDS; directed_three_regressions: PASS_3_ACTUAL_TESTS_4_TEST_SOURCE_REPAIRS; new_full_workspace: NOT_RUN_AFTER_TEST_REPAIRS; combined_v3_workspace: PASS_1203_INCLUDING_4_RENDERER_FMT_CLIPPY.

- [ ] **Source14 vollständig abnehmen und installieren** — INSTALLED_SOURCE14_FOUR_NATIVE_ROLES_PASS_BOUNDED. Source14 ist nach erfolgreicher1506/0/0-Suite, vier frischenRendererprüfungen, optimiertem Build und vier praktischen Abnahmerollen lokal installiert. Segment-/CDN-Verzeichnisprüfung:18Gruppen,205Seiten,2435Issues, API/CLI/beideMCP-Zugänge, echteUI1440/320, Stale-UI undCold. Die Installation hat alte/neue Starts auf getrenntenDatenkopien,30Assets,28MCP-Werkzeuge und alle8gespeichertenCrawls geprüft. Dateibytes,Streams,ACLs und stabileMetadaten sind exakt erhalten; ausschließlich explizit qualifizierte SQLite-SHM-Schreibzeiten unterscheiden sich. Herstellerparität bleibtNOT_PROVEN.
  - [x] Alle418 Quelldateien und Git-Blobs des privatenCodes stimmen.
  - [x] UnabhängigeProdukt-/Format-/Produzenten-/Binderreviews;Source14 unveränderlich eingefroren.
  - [x] Vollständige1506/0/0-Suite mit allen aktuellen Pflichtnamen und vier frischenRendererfunktionen bestanden.
  - [x] Optimierter Source14-Build und drei frische Nativrollen bestanden: elf Head/CDN-Fälle, qualifizierte API-only-Teilabnahme und acht Sitemap-Szenarien.
  - [x] Im ersten Segment-/CDN-Verzeichnislauf bestehen zehn Gruppen; der fehlgeschlagene MCP-Großexport und die vollständig geschlossenen eigenen Prozesse bleiben als Fehlernachweis erhalten.
  - [x] Korrigierter Test prüft alle MCP-Seiten und Issues, eigene kleine vollständige Auswahl sowie echte Größenrefusals; keine Produkt- oder Budgetänderung.
  - [x] Dritter tatsächlicher Lauf: Segment- und Links-CSV mit getrennten CRLF-/LF-Verträgen bestanden; zwölf Gruppen PASS. Negativimport scheitert vor IO an der eigenen64-KiB-Grenze. Alle eigenen Ressourcen geschlossen, keine Stoppsignale, Lock freigegeben.
  - [x] Aktueller V8-Lauf besteht alle18Gruppen einschließlich echterUI1440/320, sämtlicher gespeicherterZeilen/Exports, Stale-UI, Altformat-Negativkontrolle, Cold und genau bezeugtem Browser-/Prozess-/Pipeabschluss. Historische Fehlversuche bleiben erhalten.
  - [x] Finale Installer-/Cold-V4-Reviews ohne Befund, tatsächliche lesende Vierrollen-Bereitschaftsprüfung und lokale Source14-Installation mit acht gespeichertenCrawls erfolgreich. Private Datenbelege bleiben lokal.
  - [ ] Offene Herstellerdetails und noch nicht implementierte Funktionsgruppen weiter nach Katalog abarbeiten; erfolgreiche lokale Abnahme beweist keine vollständige Herstellerparität.
  - Verträge: [installed-summary.json](verification/source14-installed-20261016/installed-summary.json), [index.json](verification/source14-four-native-pass-20261016/index.json), [index.json](verification/main-negative-cap-and-reference-20261016/index.json), [index.json](verification/quotas-tested-main-retry-20261016/index.json), [main-first-native-failed.json](verification/quotas-tested-main-retry-20261016/main-first-native-failed.json), [index.json](verification/source14-three-native-progress-20261016/index.json), [index.json](verification/source14-full-suite-20261016/index.json).
  - Gate-Stand: full_suite: PASS_ACTUAL_1506_0_0_FOUR_FRESH_RENDERER_FMT_CLIPPY; optimized_build: PASS_ACTUAL_SOURCE14; native: PASS_ACTUAL_FOUR_ROLES_QUALIFIED_SCOPES; installation: PASS_ACTUAL_SOURCE14_COLD_COPY_COMPATIBILITY_AND_DATA_PRESERVATION; manufacturer_parity: NOT_PROVEN.

- [ ] **N-Gramm-Corpus, Quellenberichte und Verzeichnisansicht prüfen** — ACTUAL_WARM_RESOURCE6_NGRAM5_PASS_EXACT_LENGTH_LIFECYCLE_BUG. Source10 besteht1380Tests und optimierten Build sowie native API-/CLI-Gruppe/13UI-Mockgruppen. Sechs warme Resource-/Directory- und fünf neue Ngramgruppen bestehen mit206echten gespeicherten Seiten,205Quellen/410Sechswortvorkommen, unabhängigen1–6-Summen,API/CLI/vollständigerCSV/UI1440/320. Reguläre Appbeendigung klassifiziert jedoch sechs nachweislich vollständig gelesene Content-Length-Downloads alsabgebrochen. Ein einzelner echter205Zeilen-HTTP-Download reproduziert das. Gesamtlauf bleibtFAIL; kalte Wiederöffnung und Installation sind offen. Neben dem CSS-Umbruch ist ein begrenzter Rust-Lebenszyklusfix erforderlich.
  - [x] 369 Inputs zusammengesetzt,18eigene Pfade (7geänderte/11neue),351 unverändert; unabhängige Kompositions- und Formatprüfung.
  - [x] 21 neue Rustnamen tatsächlich ausgeführt:15Ngramfälle inklusive echter Capture->API/CLI/CSV und6SQLite-Projektionsfälle.
  - [x] Ursprünglicher Clippy-Stilfehler mit Code101 erhalten und eng korrigiert; neuer Clippyprozess schließt tatsächlich0.
  - [x] Vollständige tatsächliche1380/0/0-Workspace-Abnahme mit vier echten Renderergates,598Pflichtnamen/602qualifizierten Namen, drei erfolgreichen Cargoabschlüssen, geschlossenen eigenen Diensten und abschließenden369Quellhashes.
  - [ ] Known-Length-Antworten mit tatsächlichem Worker-/Supervisorabschluss vor dem letzten Frame korrekt abschließen; Zero/short/extra/lateError/cancel/realHyper testen. Unknown-Length-Verhalten erhalten.
  - [ ] Rustfix und einzelne CSS-Umbruchregel als neue geprüfte Quellenfassung integrieren; frische Suite, Windows-Build und native Abnahmen einschließlich kalter Wiederöffnung bestehen.
  - [ ] Sicher mit erhaltenen Nutzerdaten installieren; weitere Textkanäle/Wortwolke, große Sitemap-Abfragen und tatsächliche Crawl-Aufnahmegrenzen weiter abarbeiten.
  - Verträge: [index.json](verification/source10-response-registry-20261015/index.json), [index.json](verification/source10-native-first-20261015/index.json), [index.json](verification/combined-workspace-v10-20261015/index.json), [saved-ngram-reports.md](saved-ngram-reports.md), [index.json](verification/source10-directed-checkpoint-20261015/index.json).
  - Gate-Stand: directed_new_names: PASS_21_NEW_FEATURE_NAMES_ONLY; preliminary_clippy: PASS_PRELIMINARY_CLIPPY_ONLY; full_suite: PASS_COMBINED_WORKSPACE_V10_REAL_RENDERER_FMT_CLIPPY; optimized_build: PASS_COMBINED_OPTIMIZED_BUILD_V10; native: NOT_RUN; installation: UNCHANGED_SOURCE5_R3; browser: FAIL_INVALID_REPRESENTATIONS_SETTINGS_OVERFLOW_320; resource_directory: FAIL_EXACT_LENGTH_COMPLETE_BYTES_COUNTED_ABORTED; ngram: FIVE_ACTUAL_WARM_CASES_PASS_COLD_NOT_RUN.

- [ ] **Defekte Linkquellen, Ressourcen und Verzeichnisansicht ausbauen** — ACTUAL1359_SUITE_AND_BUILD_PASS_NATIVE_BROWSER_PRODUCT_BUG_SOURCE10_FIX_INTEGRATING. Source9 besteht1359 Tests, Format, Clippy und optimierten Build. Browser27 besteht26 Gruppen, scheitert dann am echten deaktivierten Originaldaten-Ladebutton nach erster Crawl-Auswahl. Einzeiliger Fix besteht tatsächlich3 separate Node-Regressionen nach1pass/2fail im alten Stand. Resource-/Directory-Workflow besteht fünf funktionale Gruppen, stoppt dann an zu breiter Sidebar-Bounds-Prüfung; alle35 Navigationen separat bei320Pixel per Tastatur/Pointer erreichbar. Source9 wird nicht installiert. Source10 integriert den UI-Fix,1–6-Gramm-Corpus und kleine TX-lokale Reportprojektion mit vollständiger neuer Abnahme.
  - [x] Kopfzeilenfehler im tatsächlichen Source8-Browser gefunden; CSS-Umbruch unabhängig geprüft und als Browser-CSS-Vorschau bei1440/320 gemessen.
  - [x] ResourceWorkflow-Quellenreviewbefund und zwei Verzeichnis-Sonderfälle vor Integration geschlossen; vollständige30-Pfade-Komposition unabhängig quellengeprüft.
  - [x] Header-CSS mit vollständig komponierten statischen Assets bei320/560/561/600/768/850/1440 im Browser ohneÜberlauf geprüft; ausdrücklich kein kompilierter Produktnachweis.
  - [x] Neuer Stand mit30 eigenen Quellpfaden einschließlich getrennter Link-/Ressourcenberichte, API/UI/CSV, Resource-CLI, Verzeichnisansicht und globalem Headerumbruch privat versioniert.
  - [x] Tatsächliche Clippy-Compilerprüfung aller Workspace-Ziele ohne Warnungen bestanden; alle358 Eingaben nach Abschluss exakt geprüft.
  - [x] Tatsächliche gerichtete Tests:17 Verzeichnisfälle bestanden,19 Link-/Ressourcenfälle bestanden und ein enger Fixtureselektor fehlgeschlagen. Vollständige Rohlogs bleiben erhalten.
  - [x] Tatsächliche vollständige1359/0/0-Abnahme des finalen358-Dateienstands mit allen577 Pflichtnamen,581 vollständigen Namen, vier echten Renderer-Gates, drei erfolgreichen Cargo-Abschlüssen, geschlossenem eigenen Renderer/Testserver und abschließend identischen Quellen.
  - [ ] Source10 aus drei unabhängig geprüften Komponenten integrieren; die21 neuen Rust-Fälle und3 separaten Node-Regressionsfälle tatsächlich ausführen.
  - [ ] Vollständige Suite, optimierten Build und alle erforderlichen nativen Bereiche am neuen Kandidaten frisch prüfen; keine Übertragung historischer PASS-Werte.
  - [ ] Sichere Installation mit erhaltenen Nutzerdaten erst nach erfolgreicher praktischer Abnahme. Herstellerparität, weitere N-Gramm-Kanäle/Wortwolke und amortisierte große Sitemap-Abfragen bleiben offen.
  - Verträge: [index.json](verification/source9-native-checkpoint-20261015/index.json), [index.json](verification/combined-workspace-v9-20261015/index.json), [index.json](verification/source9-directed-checkpoint-1-20261015/index.json), [index.json](verification/source9-preliminary-failed-20261015/index.json), [index.json](verification/source8-native-checkpoint-1-20261015/index.json).
  - Gate-Stand: implementation: PRIVATELY_COMMITTED_30_SOURCE_PATHS; compile: PASS_ALL_TARGET_CLIPPY_NO_WARNINGS; directed_tests: PASS37_AFTER_EXACT_TEST_IDENTITY_REPAIR; full_suite: PASS_COMBINED_WORKSPACE_V9_REAL_RENDERER_FMT_CLIPPY; optimized_build: PASS_COMBINED_OPTIMIZED_BUILD_V9; native: NOT_RUN; installation: UNCHANGED_SOURCE5_R3; manufacturer_parity: NOT_PROVEN; native_browser: FAIL_COMBINED_V9_ACTUAL_NATIVE_FINDINGS_BROWSER; resource_directory: FAIL_ACTUAL_SOURCE9_RESOURCE_DIRECTORY_WORKFLOW; ui_regressions: PASS3_NODE_CASES_AFTER_BASELINE_1PASS_2FAIL.

- [ ] **Migrationsprüfung, Schreibvertrag und responsive Originaldateien prüfen** — SUITE_BUILD_SIX_NATIVE_SCOPES_PASS_UI_LAYOUT_FAIL_NOT_INSTALLABLE. Source8 besteht die vollständige1322-Test-Suite und den optimierten Build. Sechs native Bereiche bestehen: API/CLI/SDK-Grundablauf,175 Dateioperationen, acht Migrationsgruppen, zwei EOF-Fälle, zwölf physische CSV-/JSON-Grenzfälle und13 Originaldatei-Gruppen mit echten64MiB. Der neue Migrationslink verbreitert jedoch die Kopfzeile bei320Pixeln; Browser und Mock scheitern daran. Der Junction-Harness bricht vor der Funktion an einer veralteten Quellenanzahl ab. Source8 wird nicht installiert. Ein eng geprüfter Headerumbruch und neue Link-/Ressourcenberichte werden als Source9 vorbereitet.
  - [x] 335 aktuelle Quellen privat committed, gepusht und eingefroren; drei Produktanteile und alle Formatänderungen unabhängig geprüft.
  - [x] 25 neue Testfunktionen sowie alle bisherigen 1297 Tests im tatsächlichen gemeinsamen Lauf bestanden; alle drei Cargo-Prozesse erfolgreich beendet, eigene Testserver und Renderer geschlossen und Quellen abschließend identisch.
  - [x] Tatsächlicher optimierter Build mit41.269.248Bytes, unveränderten335Quellen und final erneut bestätigter vollständiger Suite bestanden.
  - [x] Migrationsworkflow: echte Redirect-Crawls,207 CSV-Zeilen, vollständige API-/CLI-/Browser-Exporte, gehaltene Datei-/Antwortregressionen,1440/320Pixel, fünf natürliche Kindprozessabschlüsse und Wiederöffnung mit null zusätzlichen Websiteabrufen bestanden.
  - [x] Originaldateien: alle13Gruppen einschließlich vollständiger64MiB-EOF-/Hashprüfungen, beschädigter/fehlender Dateien, Quoten, Clientabbruch und kalter Wiederöffnung bestanden. Reparierter Testserver hat elf abgeschlossene Transfers, keine Fehler und einen erfolgreichen Abschluss.
  - [x] Zwölf physische CSV-/JSON-Grenzfälle tatsächlich bestanden:64MiB beziehungsweise8MiB vollständig mit normalem EOF, je einByte darüber abgewiesen; alle14 eigenen App-Kindprozesse geschlossen und Testserver beendet.
  - [ ] Kopfzeile umbrechen und Browser27 sowie Mock13 am nächsten optimierten Kandidaten erneut vollständig prüfen. Die CSS-Vorschau passt tatsächlich bei1440/320, ist aber kein Produkt-Browser-PASS.
  - [ ] Junction-Harness an die tatsächliche Quellenanzahl binden; der fehlgeschlagene Vorcheck hat keine Junctionfunktion ausgeführt.
  - [ ] Vollständigen50.000-Einträge-Lauf und alle erforderlichen nativen Bereiche am endgültigen korrigierten Kandidaten ausführen.
  - [ ] Getrennte Seitenlink-/Ressourcenquellen-Berichte und Verzeichnisansicht in Source9 integrieren, vollständig kompilieren und praktisch prüfen.
  - [ ] Sichere Installation unter Erhalt aller Nutzerdaten erst nach allen erforderlichen Abnahmen. Übrige Fähigkeiten, Filter und Abläufe weiter abarbeiten.
  - Verträge: [index.json](verification/source8-physical-pass-20261015/index.json), [index.json](verification/source8-native-checkpoint-1-20261015/index.json), [index.json](verification/combined-workspace-v8-20261015/index.json).
  - Gate-Stand: full_workspace: PASS_COMBINED_WORKSPACE_V8_REAL_RENDERER_FMT_CLIPPY; optimized_build: PASS_COMBINED_OPTIMIZED_BUILD_V8; native: SIX_SCOPES_PASS_BROWSER_MOCK_LAYOUT_FAIL_JUNCTION_PRECHECK_NOT_RUN_LARGE_DEFERRED; installation: NOT_INSTALLED_ACTUAL_HEADER_LAYOUT_DEFECT; manufacturer_parity: NOT_PROVEN.

- [ ] **Gemeinsame Originaldatei- und HTTP-Verbindungsfunktionen prüfen** — SEVEN_NATIVE_SCOPES_PASS_BROWSER_AND_BODY_OVERALL_FAIL_NOT_INSTALLED_SUPERSEDED_BY_SOURCE8. Source7 besteht 1297 Tests und den optimierten Build. Sieben native Bereiche bestehen, einschließlich aller 50000 Sitemap-Einträge vor und nach kalter Wiederöffnung sowie des korrigierten Mocks. Browser27 scheitert am Originaldatei-Panel bei 320 Pixeln. BODY13 besteht die Funktionsgruppen und zwei natürliche App-Abschlüsse, scheitert insgesamt jedoch am Testserver-Abschluss. Beide Fehler bleiben erhalten; Source7 wurde nicht installiert. Der korrigierte Source8-Stand ist jetzt integriert und vollständig getestet.
  - [x] Neun geprüfte Transportdateien auf den tatsächlich erfolgreichen Source6R2-Stand übertragen; alle Abbruchkorrekturen und die gesamte Regression erhalten. Unabhängige Quellen-, Formatierungs- und Runnerprüfungen sowie Vorab-Clippy bestanden.
  - [x] 326 Eingaben privat committed, gepusht und eingefroren. Die tatsächliche vollständige Suite besteht mit 1297/0/0, 515 Pflichtnamen, 41 Modulnamen, 13 neuen Transportfällen, vier Renderern sowie Format und Clippy. Alle drei Cargo-Prozesse erfolgreich beendet; eigene Testserver und Renderer geschlossen und Quellen abschließend identisch.
  - [x] Optimierter Kandidat mit 40.585.216 Bytes und SHA 5e7c8fbb bestanden; Build und Quellen abschließend identisch. Fünf getrennte native Bereiche bestehen, darunter CSV genau 64 MiB und JSON genau 8 MiB sowie jeweils eine korrekt abgewiesene Überschreitung um ein Byte.
  - [ ] Originaldatei-Panel bei 320 Pixeln korrigieren und echte Browser27-Abnahme am korrigierten gemeinsamen Kandidaten bestehen.
  - [ ] Veraltete Mockadapter-Verweise eng korrigieren und Mock13 erneut in frischen Ausgaben ausführen.
  - [ ] Große Sitemaps und Originaldateien nativ prüfen; sämtliche erforderlichen Prüfungen am endgültigen Kandidaten wiederholen.
  - [ ] Migration und Teil-Schreibvertrag im nächsten gemeinsamen Stand integrieren; danach sichere Installation unter Erhalt aller Nutzerdaten.
  - [ ] Übrige 144 Fähigkeiten, 416 Filter und 32 Abläufe weiter abarbeiten; vollständige Herstellerparität bleibt NOT_PROVEN.
  - Verträge: [index.json](verification/combined-workspace-v8-20261015/index.json), [index.json](verification/source7-native-checkpoint-1-20261015/index.json), [index.json](verification/combined-workspace-v7-20261015/index.json).
  - Gate-Stand: full_workspace: PASS_COMBINED_WORKSPACE_V7_REAL_RENDERER_FMT_CLIPPY; required_named_cases: PASS_515_AND_41_FULL_MODULE_NAMES; actual_renderer: PASS_4; optimized_build: PASS_COMBINED_OPTIMIZED_BUILD_V7; native: SEVEN_SCOPES_PASS_BROWSER_AND_BODY_OVERALL_FAIL; installation: NOT_INSTALLED_SUPERSEDED_BY_SOURCE8; manufacturer_parity: NOT_PROVEN.

- [ ] **Originaldatei-API und Response-Owner gemeinsam integrieren** — ACTUAL_REPAIRED_FULL_WORKSPACE_PASS_SOURCE7_COMBINATION_PENDING. Der frische reparierte Source6R2-Stand besteht tatsächlich1284 Tests,502 Pflichtnamen,28 vollständige neue Modulnamen, vier echte Renderer-Fälle sowie Format und Clippy. Der frühere Source6-Hänger bleibt als Fehlbeleg erhalten. Zwei Reparaturdateien sind privat unter3be3fec integriert; der gemeinsame Source7-Kandidat und seine vollständige native Abnahme folgen.
  - [x] Body-App4 und Response-Owner2 mit17 Änderungsdateien zusammengeführt;322 unveränderte Eingabedateien und beide unveränderten gemeinsamen Zusammenführungen gegenüberV2 exakt geprüft.
  - [x] Fehlgeschlagene vorgezogene Compilerläufe samt Rohlogs erhalten: zuerst eine Core-Clippy-Meldung, danach zwei App-Clippy-Meldungen und ein testlokaler ApiError-Diagnosefehler. Keine Tests ausgeführt und keine Produktfreigabe daraus abgeleitet.
  - [x] Drei neue Diagnosekorrekturen geprüft; alleAssertions und27 neuen Windows-Testnamen plus einUnix-Fall erhalten. Isolierten drittenEntwurf mit tatsächlichemFormat-Exit0 erstellt.
  - [x] DrittenVorab-Clippy mit31Sekunden/Exit101/0Tests unverändert erhalten. Genau drei .err().expect()-Testauswertungen durch exhaustiveMatches mitgleichemPaniktext ersetzt; alleAssertions/Timeouts/Testnamen erhalten. Beide Vorschläge undKomposition4 separat unabhängig geprüft.
  - [x] Body5, Response4, Komposition4, Format4 sowie Ausführungstreiber4 und Binder2 unabhängig geprüft. Tatsächlicher Vorab-Clippy-Lauf beendet mit Exit0, geschlossenen Streams und final bestätigten324 Eingaben; 0 Tests ausgeführt.
  - [x] 17eigeneDateien privat committed/pushed15cb424,324 Eingaben endgültig eingefroren. VollständigeSuite tatsächlich bis zum echtenQueue-/Abbruchtest ausgeführt; Rohlog,506Sekunden undabnormalerExit0xffffffff erhalten. Es existierenkeinevollständigenTestsummaries, aber163 einzelneok-Zeilen.
  - [x] Ursache gegen tatsächlichenCode/Standardbibliotheksvertrag belegt: permanenterInterrupted-Abbruch hältwrite_all inWiederholung. Nur eigenerTestprozess nachPfad/Startzeit/Elternkette beendet; ursprünglicheApp/Renderer nichtsignalisiert. AlleQuellen finalidentisch undeigeneTestserver geschlossen.
  - [x] ReparaturV2: genauvierpermanenteErrorKinds undein zusätzlicher begrenzterWriteAll-/Flush-Regressionsfall unabhängig geprüft. AllealtenTestbytes erhalten; tatsächlicherFormatlauf undvorgezogenerAll-Targets-Clippy schließen0,0Tests ausgeführt.
  - [x] Source6R2: genauzwei Reparaturdateien privat committed/pushed3be3fec und324 Eingaben eingefroren. Tatsächlicher vollständiger Lauf1284/0/0,502 Pflichtnamen und28 vollständige Modulnamen; vier echte Renderer-Fälle, Format/Clippy undalle drei Cargo-close-Ereignisse erfolgreich.
  - [x] Der ursprüngliche echte8-enqueues/9th-full-reservation/Bodydrop-Fall endet jetzt erfolgreich; der zusätzliche begrenztewrite_all/flush-Regressionsfall besteht. EigeneRenderer/Fixture geschlossen undalle Quellen finalidentisch.
  - [ ] Source7-Verbindungsbesitz aufden tatsächlichen repariertenR2-Stand übertragen undgemeinsameSuite bestehen.
  - [ ] Gemeinsamen optimiertenKandidaten bauen undallefrischenNative-/BODY13-/Browser27-/Verbindungsprüfungen bestehen.
  - [ ] Migrations-CSV, Teil-Schreibvertrag undübrigeFeature-/Filter-/Ablaufliste weiterabarbeiten; vollständigeHerstellerparität bleibtNOT_PROVEN.
  - Verträge: [index.json](verification/combined-workspace-v6-r2-20261015/index.json), [index.json](verification/source6-r2-reviewed-repair-20261015/index.json), [index.json](verification/combined-workspace-v6-cancel-hang-failed-20261015/index.json), [index.json](verification/source6-draft4-preliminary-pass-20261015/index.json), [index.json](verification/source6-7-reviewed-drafts-20261015/index.json), [sitemap-source-archive-plan.md](parity/sitemap-source-archive-plan.md), [http-response-body-drain-plan.md](parity/http-response-body-drain-plan.md).
  - Gate-Stand: source_review: PASS_BOUNDED_COMPOSITION4_BODY5_RESPONSE4; format: ACTUAL_DRAFT4_FORMAT_CLOSE0_INDEPENDENT_REVIEW_PASS; preliminary_clippy: ACTUAL_DRAFT4_ALL_TARGETS_CLOSE0_ZERO_EXECUTED_TESTS; full_workspace: PASS_COMBINED_WORKSPACE_V6_R2_REAL_RENDERER_FMT_CLIPPY; source_promotion: ACTUAL_17_ORIGINAL_PLUS_2_REPAIR_FILES_PRIVATELY_PUSHED; native: NOT_RUN_ON_SOURCE6_R2; installation: NOT_RUN; manufacturer_parity: NOT_PROVEN; optimized_build: NOT_RUN_COMBINED_SOURCE7_CANDIDATE_PLANNED; terminal_cancellation_repair: ACTUAL_BOUNDED_REGRESSION_AND_ORIGINAL_HANG_CASE_PASS; required_named_cases: PASS_502_AND_28_FULL_MODULE_NAMES; actual_renderer: PASS_4.

- [ ] **Ursprüngliche Sitemap-Dateien dauerhaft speichern** — ACTUAL_R3_RUST_TESTS_PASS_NATIVE_PENDING. Die optionale Core-/Store-Grundlage ist im tatsächlich vollständig geprüften Source5-R3-Kandidaten enthalten: alle16 neuen Windows-Fälle bestehen in der1256-Test-Suite. Originalbytes, getrennte Budgets, unveränderliche Dateien und Wiederöffnungsprüfungen sind umgesetzt. Echte große Archiv-/Recovery-Fälle fehlen. API, Oberfläche und verifizierter Download sind als Source6-Quellvorschlag vorbereitet und noch nicht im Kandidaten integriert.
  - [x] BODY01–04: getrennte Konfiguration und Budgets, eigene unveränderliche Dateien, SQLite-Zuordnung zur tatsächlich gespeicherten Quellenmessung und Wiederöffnungsprüfung als Quellcode vorbereitet.
  - [x] Beide konkreten Erstbefunde korrigiert: eindeutiger Archivpfad je Datenbankdateiname und SQL-Feldgrenzen vor Allokation. V2-Review 702d4337 ohne weitere Befunde im begrenzten Umfang.
  - [x] 14 neue Testfunktionen geschrieben, davon13 für Windows; noch nicht ausgeführt.
  - [x] Finale V3 mit früher ausdrücklicher UTF-16-Ablehnung und drei weiteren Tests unabhängig geprüft; insgesamt 17 verfasste Tests, davon 16 unter Windows. 35 tatsächliche Formatänderungen separat geprüft.
  - [x] Aktuell: alle16 neuen Windows-Fälle tatsächlich aufR3 bestanden; frühere Angaben zu nicht ausgeführten Entwürfen sind historische Entwicklungsstände.
  - [ ] Echte Dateierhalt-, Wiederöffnungs-, Quota- und Korruptionsfälle einschließlich großer Originaldateien nativ bestehen; die vollständige1256-Test-Suite besteht bereits.
  - [ ] BODY05–08: vorbereitete API-/UI-Konfiguration und verifizierten Originaldownload integrieren, vollständig kompilieren und im echten Browser gegen die Bytequellen prüfen.
  - [ ] MCP-Fenster, eigene Kopien für abgeleitete Crawls, portable Bundles, Wiederherstellung und kontrollierte Bereinigung bleiben getrennte offene Schritte.
  - Verträge: [index.json](verification/source6-7-reviewed-drafts-20261015/index.json), [index.json](verification/combined-workspace-v5-r3-20261015/index.json), [index.json](verification/combined-source-v5-r3-source-20261015/index.json), [index.json](verification/combined-source-v5-r2-clippy-failed-20261015/index.json), [index.json](verification/combined-source-v5-r2-source-20261015/index.json), [sitemap-source-archive-plan.md](parity/sitemap-source-archive-plan.md), [sitemap-source-archive-tasks.json](parity/sitemap-source-archive-tasks.json).
  - Gate-Stand: source_review: PASS_BOUNDED_SOURCE_ONLY_V3_AND_FORMAT_MERGE; rust_tests: PASS_ALL_NEW_CASES_ON_ACTUAL_R3_FULL_1256_SUITE; public_api_ui: SOURCE6_REVIEWED_PROPOSAL_ONLY_NOT_PROMOTED_OR_EXECUTED; native_body_recovery: NOT_RUN; installation: NOT_RUN.

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

- [ ] **Große Sitemap-Berichte beim Blättern beschleunigen** — TX_LOCAL_PROJECTION_ACTUAL1380_WORKSPACE_PASS_SPEED_NOT_PROVEN. Optionale2MiB-TX-lokale Reportprojektion besteht6 echte SQLite-Tests inklusive exakter Budgetgrenze, ungültiger UTF8-Rückfall, Hash-/Stale-Semantik und Cancel. Original256MiB-Reportgrenze und vollständige Integritätsprüfungen bleiben erhalten. Konstante Verbesserung ohne gemessenen Leistungsnachweis; amortisierte große Lesesitzungen bleiben offen.
  - [x] Tatsächliche Laufzeitbelege und acht unveränderte Source7/8-Lesepfade geprüft; langsamer Read-only-Abschnitt von Erfassung, Netzwerk und Dateischreibphase getrennt.
  - [x] Enger3-Dateien-Entwurf mit optionaler2MiB-TX-Projektion für zwei interne JSON-Prüfungen unabhängig quellengeprüft. UTF8-Unsupported fällt auf alte SQL-Pfade zurück; sechs echte SQLite-Testquellen sind geschrieben.
  - [ ] Einzelne echte Requestzeiten und deterministische JSON-/Hash-/Decode-Arbeitszähler erheben.
  - [ ] Zuerst pro bestehendem Read-TX die geprüfte Reportprojektion wiederverwenden; bisherige Fallbacks und Hash-/Stale-/Tail-/Unknown-/Größenprüfungen vollständig erhalten.
  - [ ] Für Amortisierung über Fenster eine ausdrücklich begrenzte Read-Snapshot-Session mit Datei-/Connectionidentität und Mutations-/Idle-/Stop-/Join-Vertrag entwerfen; kein ungesicherter Hashcache auf veränderlicher Datenbank.
  - [ ] Identische50.000-Einträge-/CSV-/Tail-/Wiederöffnungsfälle tatsächlich vor/nach vergleichen; ohne Messung keinen Geschwindigkeitsgewinn behaupten.
  - [ ] Wiederholte vollständige Integritäts-/Report-/Katalogprüfung und :memory:-Indexerzeugung je öffentlicher200er-Seite instrumentieren. Eine explizite, begrenzte Lesesitzung mit eigenem Snapshot-/Invalidierungs-/Cancel-/EOF-Vertrag prüfen; Generation-only-Cache darf Tail-/Quellkorruption nicht verdecken.
  - Verträge: [PLAN.md](verification/next-feature-reviewed-20261016/plans/source12-sitemap-pagination-performance-next-step-v1/PLAN.md), [index.json](verification/sitemap-read-performance-20261015/index.json), [index.json](verification/next-batch-source-proposals-20261015/index.json).
  - Gate-Stand: evidence: ACTUAL_PRIOR_TIMESTAMPS_AND_CURRENT_BYTE_EQUAL_READ_SOURCE; implementation: SOURCE_ONLY_REVIEWED_NOT_COMPILED; performance_gain: NOT_PROVEN; manufacturer_parity: NOT_PROVEN.

- [ ] **N-Gramm-Corpus mit vollständigen Quellen und Exporten ausbauen** — FIVE_ACTUAL_WARM_NGRAM_CASES_PASS_RESPONSE_LIFECYCLE_FIX_COLD_PENDING. Gespeicherter1–6-Gramm-Corpus ist in Source10 integriert und besteht15 neue Rust-Fälle mit Häufigkeiten, vollständigen Quellenmitgliedschaften, begrenzten Positionsbeispielen, expliziten fehlenden Quellen, Snapshot-/Filterbindung, API/CLI und vollständiger CSV. Vollständige Suite, echter UI-/205Quellseiten-/Cold-Nachweis und Installation folgen; weitere Kanäle/Wortwolke bleiben offen.
  - [x] Entwurf und Quellenreview ohne offene Befunde abgeschlossen; Quellen-/Filter-/Snapshotbindung und vollständige versus begrenzte Positionsdaten sind getrennt.
  - [x] Offizielle N-Gramm-Anleitung gegen tatsächlich gespeicherte Text-, Titel-, Überschriften-, Link- und Bilddaten abgeglichen; fehlende Kanalbeweise einzeln erfasst.
  - [ ] Auf endgültigen Source9-Parent rebasen, alle neun neuen Fälle kompilieren/ausführen und vollständige native Export-/Crawl-/Wiederöffnungsabnahme durchführen.
  - [ ] 4-,5-,6-Gramme mit getrennter Fensterkonfiguration und Überlappungs-/Stopwort-/Sprach-/Budgetregressionen ergänzen, bestehende Similarity-Konfiguration erhalten.
  - [ ] Titel-, Heading- und eingehende Ankerkanäle mit eigener Quellen-/Mehrdeutigkeits-/Vollständigkeitspolitik ergänzen. Zielgebundene Bildlink-Altbeobachtungen brauchen zusätzliche Erfassung.
  - [ ] Unverlinkten Körpertext erst mit belegter Segmentzugehörigkeit oder aufbewahrtem HTML/DOM rekonstruieren; flache Textnachbarschaften sind kein Beweis.
  - [ ] Begrenzte nachvollziehbare Wortwolke mit Listen-/Tastaturalternative sowie320/1440-Browserkontrollen ergänzen.
  - [ ] Große Corpora mit tatsächlichen Arbeitszählern prüfen; vollständige Neuaggregation je Fenster und ausgewählteIdentität O(S*T) gezielt reduzieren.
  - Verträge: [index.json](verification/source10-response-registry-20261015/index.json), [index.json](verification/combined-workspace-v10-20261015/index.json), [index.json](verification/source10-directed-checkpoint-20261015/index.json), [index.json](verification/next-batch-source-proposals-20261015/index.json), [ADDENDUM.md](verification/next-batch-source-proposals-20261015/ngram/gap/ADDENDUM.md).
  - Gate-Stand: source_review: NO_FINDINGS_BOUNDED_SOURCE_REVIEW; rust_tests: 9_AUTHORED_NOT_RUN; native: NOT_RUN; manufacturer_parity: NOT_PROVEN.

- [x] **Crawl-Grenzen, weitere Ngramkanäle und große Sitemap-Daten ausbauen** — COMPLETED. Source12 ist nach der vollständigen1428/0/0-Suite, dem optimierten Windows-Build und14 tatsächlich bestandenen nativen Prüfbereichen lokal installiert. Static25 besteht mit501 echten WebsiteGETs. Alle acht gespeicherten Crawls und38 Daten-Dateien sind erhalten; Datenbytes und vereinbarte stabile Metadaten sind gleich. Das vollständige rohe Metadatenprotokoll bleibt wegen ausdrücklich inventarisierter SQLite-SHM-Zeitänderungen separat.
  - [x] 381 Eingaben aus drei unabhängig geprüften Rollen ohne Pfadüberschneidung komponiert und formatiert. Alle 606 bisherigen Vollnamen bleiben zusätzlich zu 44 neuen Funktionen erhalten.
  - [x] Strenger Compilerlauf erfolgreich. Beide echten 501-URL-Kontrollcrawls, alle sechs Indexfälle einschließlich 50.000 Mitgliedschaften und 59-MiB-Bericht sowie die neuen Metadatenkanäle ohne Pflicht-Textcapture bestehen in den jeweiligen Funktionen.
  - [x] Beide Fixture-Korrekturen in finaler Komposition unabhängig geprüft und formatiert; tatsächlicher finaler Clippy-Abschluss mit Code0. Gegen die vorherige Komposition sind379 von381 Eingaben bytegleich und ausschließlich zwei Testdateien geändert.
  - [x] Finale Quellen als5d50a10 privat veröffentlicht; alle381 Arbeits- und Git-Blob-Hashes geprüft. Beide vorherigen Fixturefehler bestehen jetzt in der vollständigen1428/0/0-Suite.
  - [x] Vier tatsächliche Renderergates, alle650 Vollnamen/646 Pflichtnamen, Format/Clippy und reguläre eigene Worker-/Fixtureabschlüsse sind belegt. Alle381 Quellen bleiben am Ende identisch.
  - [x] Optimierter Windows-Build tatsächlich bestanden: AMD64 PE43650560Bytes, SHA256f7ce325cf73d16f1c4943ebf98dc8a2e888e93ee25c999bc7f2c10d1b1a1a65c.
  - [x] Sitemap-Erzeugung: sechs tatsächliche API-/CLI-/Browser1440/320-/Cold-Gruppen bestanden; alle acht direkten Kinder natürlich geschlossen, eigener Lock freigegeben.
  - [x] Acht Metadaten-Ngram-API-/CLI-/Browser-/Cold-Gruppen tatsächlich bestanden: acht HTML-URLs/elf WebsiteGETs; alle sieben direkt gestarteten Kinder natürlich geschlossen, Renderer/Fixture geleert, null WebsiteGETs bei späteren Lese-/Exportoperationen.
  - [x] Sechs bestehende native Bereiche tatsächlich bestanden und unabhängig an2039 Assertions/478 Dateien abgeglichen: API/CLI3, UI-Mock13, Browser27, physische Exportgrenzen12, MCP-Dateitools175 und Root-Junction2. Prozessabschlussarten sind einzeln dokumentiert; erzwungene Stops belegen keinen Graceful Drain.
  - [x] Large13 tatsächlich vollständig bestanden:50.000 gespeicherte Sitemap-Einträge,250×200 öffentliche Seiten warm und kalt, vollständiger57.574.930-Byte-CSV-Export, eigene Prozesse/Pipes/Fixture geschlossen. Kein50.000-Seiten-Websitecrawl oder Geschwindigkeitsnachweis.
  - [x] Aktuelle Chrome-Programmdateien vollständig und exklusiv nach E kopiert: 265 Dateien/512.064.902 Bytes; Original und Zielbaum unverändert. Vier echte Rendererfunktionen und die aktuellen UI-/Browser-/Generator-/Metadatenläufe bestehen. Die historische Gesamtsuite bleibt separat mit ihrer ursprünglichen Browseridentität erhalten.
  - [x] EOF2, Originaldaten13, Migration8 und Resource8 mit sechs zusätzlichen Ngramgruppen bestehen und sind unabhängig am tatsächlichen Nachweis geprüft. Acht aktuelle und fünf erhaltene historische native Bereiche ergeben 13 von 14; Static25 bleibt fehlgeschlagen.
  - [x] Static25 mit unveränderten25 Orakeln erneut tatsächlich bestanden:501 echte GETs,7 natürlich geschlossene Kinder; frühere Fehlerläufe unverändert erhalten.
  - [x] Alle14 nativen Bereiche einschließlich der historischen Browser-freien Nachweise unabhängig an ihren tatsächlichen Dateien geprüft.
  - [x] Source12 mit drei geprüften Datenkopien, enger EXE-Ersetzung, erhaltenen acht Jobs und bytegleichen Daten installiert; App und Renderer sind gesund.
  - Verträge: [index.json](verification/installed12-source13-progress-20261016/index.json), [installed-summary.json](verification/installed12-source13-progress-20261016/installed-summary.json), [index.json](verification/source12-thirteen-scopes-20261016/index.json), [index.json](verification/source12-selected-four-20261016/index.json), [renderer-four-proof.json](verification/api-runtime-pass-20261016/runtime/renderer-four-proof.json), [current-runtime-review.json](verification/api-runtime-pass-20261016/runtime/current-runtime-review.json), [runtime-scope-correction.json](verification/api-only-rust-failure-20261016/plans/runtime-scope-correction.json), [index.json](verification/source12-runtime-drift-20261016/index.json), [index.json](verification/source12-small6-20261016/index.json), [index.json](verification/source12-metadata-native-20261016/index.json), [index.json](verification/source12-build-native-20261016/index.json), [index.json](verification/source12-suite-20261016/index.json), [index.json](verification/source12-preliminary-tests-20261015/index.json).
  - Gate-Stand: preliminary_clippy: PASS_ACTUAL_FINAL_DRAFT4_PRELIMINARY_CLIPPY; node: PASS_EIGHT_LOGICAL_SOURCE_CASES; new_rust_tests: PASS_ALL44_WITHIN_ACTUAL1428_FULL_SUITE; full_suite: PASS_COMBINED_WORKSPACE_V12_REAL_RENDERER_FMT_CLIPPY; optimized_build: PASS_COMBINED_OPTIMIZED_BUILD_V12; native: FOURTEEN_ACTUAL_PRODUCT_SCOPES_PASS; installation: PASS_ACTUAL_SOURCE12_COLD_LOCAL_INSTALL_WITH_COPY_COMPATIBILITY; manufacturer_parity: NOT_PROVEN; generator: PASS_ACTUAL_SOURCE12_SITEMAP_GENERATION_SIX_PAGE_API_CLI_UI_COLD; ngram_metadata: PASS_ACTUAL_SOURCE12_NGRAM_METADATA_CHANNELS_WORDCLOUD_API_CLI_UI_COLD; static_limits: PASS_ACTUAL25_WITH501_GENUINE_GETS; large_source_50000: PASS_GENUINE_LARGE_SOURCE_NATIVE_WITH_DECLARED_BODY_RETENTION_GAP; static_limits_r4: FAIL_FINAL_RUNTIME_HASH_AFTER_25_PASS_CASES; current_chrome: PASS_FOUR_CURRENT_SOURCE12_RENDERER_FUNCTIONS_ONLY; owned_http_eof: PASS_SOURCE12_IDLE_AND_ACTUAL_INFLIGHT_CRAWL_EOF_ONLY; original_response_body: PASS_SOURCE12_ACTUAL_CRAWL_ORIGINAL64M_PUBLIC_EOF_COLD_REOPEN_AND_BOUNDARIES; migration: PASS_ACTUAL_SOURCE12_MIGRATION_WORKFLOW; resource_directory: PASS_ACTUAL_SOURCE12_RESOURCE_DIRECTORY_WORKFLOW_WITH_NGRAM6; independent_selected_four: NO_FINDINGS_BOUNDED_ACTUAL_EVIDENCE_AUDIT.

- [ ] **Sitemap-Ziele und Mitgliedschaft öffentlich nachvollziehbar auswerten** — COMPLETED_OWN_CRITERIA_REFERENCE_PARITY_NOT_PROVEN. Acht echte Source13-Nativszenarien bestehen über vollständige API-/CLI-/MCP-/Browser-/CSV-/Cold-Schnittstellen. Unabhängiger Audit bestätigt die beiden präzisen eigenen Kriterien für Abdeckung und Gültigkeit. Gesamte Websiteabdeckung und Hersteller-Parität bleiben unbewiesen.
  - [x] Originalmitgliedschaft mit tatsächlichen Seiten und Inlinks verknüpft; fehlend, nur Sitemap, wartend, unvollständig, unbekannt, historisch und echte leere Quelle getrennt.
  - [x] Redirect301,404,Noindex,Robots und Rewrite-Identitäten mit tatsächlichen Requests geprüft; Originalmitgliedschaft erhalten.
  - [x] 17 natürliche eigene Prozessabschlüsse, vollständige CSVs, reale UI bei1440/320, Warm-/Coldgleichheit und null zusätzliche WebsiteGETs beim Lesen unabhängig geprüft.
  - Verträge: [index.json](verification/source14-code-sitemap-matrix-progress-20261016/index.json), [sitemap-independent-audit.json](verification/source14-code-sitemap-matrix-progress-20261016/sitemap-independent-audit.json).
  - Gate-Stand: native: PASS_ACTUAL_SOURCE13_FINITE_SITEMAP_PUBLIC_UNKNOWN_REWRITE_HISTORY_COLD_MATRIX; independent_audit: NO_FINDINGS_BOUNDED_ACTUAL_EVIDENCE_AUDIT; manufacturer_parity: NOT_PROVEN.

- [ ] **Teilweise geschriebene Downloadblöcke korrekt melden** — ACTUAL_SOURCE8_WORKSPACE_AND_BODY_TRANSPORT_NATIVE_PASS. Source8 korrigiert den Write-Vertrag: Eine erfolgreiche Schreiboperation meldet die tatsächlich angenommene Präfixlänge; spätere Fehler werden nicht als Nullfortschritt ausgegeben. Drei neue Regressionen und alle bisherigen HTTP-/Abbruchtests bestehen im tatsächlichen Gesamtlauf. Frische native Abnahmen folgen.
  - [x] Produktionscode und tatsächliche vier privaten Exportaufrufer gegen Rusts Write-Vertrag unabhängig gelesen; Bewertung bleibt vom belegten Interrupted-Endlosfehler getrennt.
  - [ ] Künftige Produktänderungen mit vollständiger Exportregression am jeweiligen Kandidaten prüfen.
  - Verträge: [index.json](verification/source8-physical-pass-20261015/index.json), [index.json](verification/source8-native-checkpoint-1-20261015/index.json), [index.json](verification/combined-workspace-v8-20261015/index.json), [partial-write-contract-assessment.json](verification/source6-r2-reviewed-repair-20261015/partial-write-contract-assessment.json), [index.json](verification/combined-workspace-v6-cancel-hang-failed-20261015/index.json).
  - Gate-Stand: source_assessment: CONFIRMED_TRAIT_CONTRACT_GAP_CURRENT_SUCCESSFUL_CORRUPTION_NOT_PROVEN; implementation: INTEGRATED_SOURCE8; rust_tests: PASS_ACTUAL_THREE_NEW_REGRESSIONS_AND1322_WORKSPACE; native: BODY13_EOF2_PHYSICAL12_ACTUAL_PASS.

- [ ] **Blockierte Downloadantworten und Verbindungen vollständig beenden** — ACTUAL_SOURCE7_FULL_WORKSPACE_PASS_NATIVE_PENDING. Response-Owner ist als Source6-Vorschlag unabhängig geprüft. Eigener Accept-/Connection-Besitz und gemeinsame Schonfrist sind als Source7 vorbereitet; alle drei konkreten Testorakel-Lücken der ersten Version sind in der zweiten Version eng korrigiert und unabhängig geprüft. Die acht übrigen Payloads sind unverändert. Integration in den endgültigen Source6-Stand sowie alle Compiler-/Rust-/TCP-Abnahmen fehlen.
  - [x] Tatsächlichen bestehenden HTTP/1-Stack und unbesessene Verbindungsaufgaben analysiert; fünf Sekunden Schonfrist für eingehende Verbindungen als Entwurfsentscheidung festgelegt.
  - [x] Source6-Response-Owner separat quellengeprüft: zwölf neue Windows-Tests verfasst, jedoch noch nicht ausgeführt.
  - [x] Source7-V1 unabhängig gelesen; dreiP2-Testorakel offen dokumentiert: tatsächlicher wartenderDrain, noch unvollständiger positiverDownload beiT0 und isolierterSocketDrop mit lebendemLease.
  - [x] Source7-V2 quellengeprüft: tatsächlichesPending vorDrain-Waiter-Abbruch, beiT0 gehaltene positiveProducer/Originalübertragung und isolierterSocketDrop bei lebendemLease ohneGlobalcancel. 13Testfunktionen verfasst,0ausgeführt; alle bisherigenAssertions erhalten.
  - [x] Source6R2-Abbruchfix und 13 Source7-Verbindungsfälle im tatsächlichen Gesamtlauf mit 1297 Tests bestanden. Zusätzliche Native- und Originaldateiprüfungen bleiben offen.
  - [ ] Source6-Owner gemeinsam mit Originaldatei-API integrieren und vollständige Compiler-/Rust-/Export-Abnahmen tatsächlich bestehen.
  - [ ] Source7-V2 an tatsächlichen endgültigenSource6-Stand anpassen, Zusammenführung unabhängig prüfen und Compiler-/Rust-Abnahmen ausführen.
  - [ ] Echte TCP-Nichtleser, natürliche Prozess-/Pipe-Enden und vollständige positive Downloads prüfen; keine allgemeine Fünf-Sekunden-Shutdown-Garantie behaupten.
  - Verträge: [index.json](verification/combined-workspace-v7-20261015/index.json), [index.json](verification/source6-7-reviewed-drafts-20261015/index.json), [http-response-body-drain-plan.md](parity/http-response-body-drain-plan.md), [http-response-body-drain-tasks.md](parity/http-response-body-drain-tasks.md), [http-response-transport-plan.md](parity/http-response-transport-plan.md).
  - Gate-Stand: source_review: PASS_BOUNDED_RESPONSE_OWNER_AND_TRANSPORT_V2_THREE_ORACLE_SOURCE_CLOSURES; rust_tests: NOT_RUN; native_tcp_nonreader: NOT_RUN.

- [ ] **Laufende HTTP- und MCP-Arbeiten sauber beenden** — ACTUAL_R3_RUST_TESTS_AND_TWO_NATIVE_EOF_PASS_REMAINING_TRANSPORT_OPEN. Gehaltene Request-, Blocking-, Crawl- und PDF-Aufgaben und frühes Schließen der Arbeitszulassung bestehen mit allen16 neuen Fällen in der tatsächlichen Source5-R3-Suite. Beide frischen eigenen EOF-Fälle schließen natürlich mitExit0; der aktive Fall belegt Listener-Ablehnung vor Freigabe der gehaltenen Antwort sowie dauerhaft1abgeschlossen/2offen. Langsam lesende HTTP-Clients, vollständige MCP-Sessions und Rendererbesitz bleiben offen.
  - [x] Tatsächliche Starts von Request-, Blocking-, Crawl- und PDF-Arbeiten im vorhandenen Code erfasst; konkrete Shutdown-Reihenfolge dokumentiert.
  - [x] Regressionen für getrennten Client, abgebrochenen Drain-Waiter, Start vor Jobinsert, bestehendes Cancel während PDF-Arbeit und Signalfehler nach Persistenz geschrieben; noch nicht ausgeführt.
  - [x] Zwei konkrete Erstbefunde korrigiert und V2 unabhängig geprüft: Export-Worker starten tatsächlich; Core-Joinfehler bleiben als Lifecycle-Fehler erkennbar.
  - [x] Aktuell: alle16 neuen Windows-Tests und beide frischen nativen EOF-Fälle tatsächlich bestanden; vollständige Logs, natürliche Prozess-/Pipe-Enden und Persistenz vor/nach tatsächlichem Ende geprüft.
  - [ ] Stateful MCP-Sessions, requests/cancelled und die Lebensdauer externer Rendererprozesse bleiben zusätzliche offene Verträge.
  - [ ] Response-Body-/Connection-Besitz und Transportabbruch für nicht lesende Clients separat implementieren und mit echtem TCP prüfen; Source5 beweist keine begrenzte Shutdown-Latenz hierfür.
  - Verträge: [index.json](verification/combined-native-v5-r3-eof-r2-pass-20261015/index.json), [index.json](verification/combined-workspace-v5-r3-20261015/index.json), [index.json](verification/combined-source-v5-r3-source-20261015/index.json), [index.json](verification/combined-source-v5-r2-clippy-failed-20261015/index.json), [index.json](verification/combined-source-v5-r2-source-20261015/index.json), [http-lifecycle-plan.md](parity/http-lifecycle-plan.md).
  - Gate-Stand: source_review: PASS_BOUNDED_SOURCE_ONLY_V2_AND_FORMAT_MERGE; rust_tests: PASS_ALL_NEW_CASES_ON_ACTUAL_R3_FULL_1256_SUITE; native_eof_inflight: PASS_TWO_FRESH_NATURAL_CLOSE0_IDLE_AND_ONE_ACTUAL_INFLIGHT_CRAWL_ONLY; installation: NOT_RUN.

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

- [x] **Roh-HTML-Head-Diagnostik und explizite CDN-Klassifikation** — COMPLETED. Source13 ist nach vollständiger1476/0/0-Suite, optimiertem Windows-Build, elf Head/CDN-Nativgruppen und der qualifizierten API-only-Teilabnahme lokal installiert. Acht gespeicherte Crawls und38 Dateien bleiben erhalten; Datenbytes und vereinbarte stabile Metadaten sind gleich.27 Webassets und27 normale MCP-Tools sind verifiziert. Der einzelne anonyme Liveprovider-Aufruf erhielt429; erfolgreiche Messwerte sind offen.
  - [x] Fehlerhafte explizite Head-Inhalte mit Rohquellpositionen und getrennten unbekannten Alt-/Budgetzuständen in16 Rustfunktionen geprüft.
  - [x] Exakte konfigurierte CDN-Origins mit Schema, IDNA-Host und effektivem Port in12 Rustfunktionen geprüft; Berichtsklassifikation erweitert weder Navigation noch Ressourcen-/Credentialfreigaben.
  - [x] Alle28 neuen Rustfunktionen tatsächlich bestanden; sechs neue Node-Quellfälle, Format und Clippy bestanden. Drei alte Fixtureinitialisierungen eng korrigiert; alle35 ursprünglichen Testkörper bleiben bytegleich.
  - [x] Source12 vollständig lokal abgenommen und installiert.
  - [x] Finalen400-Input-Produktstand privat veröffentlicht; genau36 Produktpfade, alle Rohbytes und derselbe entfernte Commit geprüft.
  - [x] Separaten Node-Verifikationscode vollständig rekonstruiert und an alle fünf tatsächlichen Testnamen gebunden; vier neue Prüftreiber unabhängig geprüft. Alle400 finalen Quellen mit tatsächlicher Rootfreigabe eingefroren.
  - [x] Vollständige1476/0/0-Suite, Format, strenger Clippy und vier frische Rendererfunktionen tatsächlich bestanden; alle400 Quellen am Ende unverändert.
  - [x] Optimierten Windows-Kandidaten45168128Bytes tatsächlich gebaut.
  - [x] API-native Teilabnahme mit drei regulären App-EOFs, Browser.close-Ack/echtem Remote-Abschluss und vollständigen207-Zeilen-JSON-/CSV-Lesewegen bestanden.
  - [x] Alle elf Head/CDN-Gruppen über echte API-/CLI-/MCP-/UI-/Cold-Schnittstellen bestanden; beide früheren Fehler bleiben erhalten.
  - [x] Source13 mit drei geprüften Datenkopien, strenger Datenintegrität und enger EXE-Ersetzung installiert; beide Dienste laufen.
  - Verträge: [index.json](verification/source13-installed-segments-progress-20261016/index.json), [installed-summary.json](verification/source13-installed-segments-progress-20261016/installed-summary.json), [index.json](verification/source13-api-native-progress-20261016/index.json), [source13-private-publication.json](verification/installed12-source13-progress-20261016/source13-private-publication.json), [index.json](verification/source13-tested-preview-20261016/index.json).
  - Gate-Stand: new_rust: PASS_ALL28_ONLY; node: PASS_SIX_SOURCE_CASES_ONLY; clippy: PASS_ACTUAL_PRELIMINARY_CLIPPY; full_suite: PASS_COMBINED_WORKSPACE_V13_REAL_RENDERER_FMT_CLIPPY; optimized_build: PASS_COMBINED_OPTIMIZED_BUILD_V13; native: PASS_HEAD11_AND_HONEST_API_SUBSET; installation: PASS_ACTUAL_SOURCE13_COLD_LOCAL_INSTALL_WITH_COPY_COMPATIBILITY; manufacturer_parity: NOT_PROVEN; private_publication: PASS_ACTUAL400_PRIVATE_EXACT_REMOTE; manifest: PASS_ACTUAL_FROZEN400; api_native_subset: PASS_SOURCE13_NATIVE_FRONTDOORS_AND_DECLARED_FIXTURE_READS_ONLY.

- [ ] **Erwartete und tatsächliche Migrationsziele prüfen** — ACTUAL_WORKSPACE_BUILD_AND_EIGHT_NATIVE_WORKFLOW_GROUPS_PASS. CSV-Zuordnungen gegen gespeicherte HTTP-Redirects sind vollständig im eigenen Workflow integriert und praktisch geprüft:16Core-Fälle, API/CLI-Servicefälle und alleacht nativen Gruppen bestehen. Browser bei1440/320, vollständige Exporte, Held-Datei-/Antwortregressionen und Wiederöffnung bestanden ohne weitere Websiteabrufe. Herstellerparität bleibt separat offen.
  - [x] NeuerCore-Code undalle16Testquellen unabhängig gelesen; zweiLib-Einfügehunks undzweiTestkorrekturen exaktzurückgeführt.
  - [x] Migrationsworkflow: echte Redirect-Crawls,207 CSV-Zeilen, vollständige API-/CLI-/Browser-Exporte, gehaltene Datei-/Antwortregressionen,1440/320Pixel, fünf natürliche Kindprozessabschlüsse und Wiederöffnung mit null zusätzlichen Websiteabrufen bestanden.
  - [ ] Beim nächsten gemeinsamen Kandidaten den unveränderten Migrationsworkflow erneut mitführen; sichere Installation bleibt vom separaten Hauptseiten-Layoutfehler und den übrigen Abnahmen abhängig.
  - Verträge: [index.json](verification/source8-native-checkpoint-1-20261015/index.json), [index.json](verification/combined-workspace-v8-20261015/index.json), [migration-INTEGRATION-TODO.md](verification/source6-r2-reviewed-repair-20261015/migration-INTEGRATION-TODO.md), [migration-core-review.json](verification/source6-r2-reviewed-repair-20261015/migration-core-review.json).
  - Gate-Stand: source_review: NO_FINDINGS_BOUNDED_WORKFLOW; rust_tests: PASS_ACTUAL1322_WORKSPACE_ALL22_MIGRATION_CASES; api_cli_ui: PASS_ACTUAL_EIGHT_GROUPS_OPTIMIZED_1440_320_COLD_REOPEN; installed_runtime: NOT_INSTALLED_SOURCE8_MAIN_HEADER_DEFECT; manufacturer_parity: NOT_PROVEN.

- [ ] **Große Sitemap-Kataloge effizient durchblättern** — ACTUAL_R3_RUST_AND_FULL_50000_NATIVE_INITIAL_REOPEN_PASS. Source5 entfernt SQL-Einzelabfragen pro Eintrag und erneutes vollständiges Serialisieren, während Integritäts-, Quellen-, Reihenfolge-, Cursor- und Größenprüfungen erhalten bleiben. Acht neue Tests prüfen die exakte bisherige Serialisierung und gültige UTF-16-Ledger. Der tatsächliche Laufzeitgewinn und die vollständige 50.000-Zeilen-Wiederöffnung sind noch offen.
  - [x] Konkrete Codepfade und die wiederholten vollständigen Durchläufe anhand des eingefrorenen V4-Quellstands bestätigt.
  - [x] Festgehaltene Sitzungen als mögliche spätere Architektur dokumentiert; Dateiidentität und Abbruch-/Shutdown-Verhalten bleiben dafür ausdrücklich offene Voraussetzungen.
  - [x] Aktuell: vollständiger50.000-Zeilen-Durchlauf mit250x200Fenstern/vollständigemCSV sowohl frisch alsauch nachWiederöffnung nativ bestanden; alleeigenenProzesse geschlossen undDaten unverändert.
  - [ ] Vollständige Source5-Suite einschließlich aller bisherigen und acht neuen Ledger-/Coverage-Orakel bestehen.
  - [ ] Exakte kanonische Bytezählung gegen den unveränderten Serializer und alle bisherigen Cursor-/Mutation-/Corruption-Orakel prüfen.
  - [ ] Echte50.000 Zeilen vollständig paginieren, exportieren und nach Neustart prüfen; konkrete Arbeitszähler, Zeit und Speicher getrennt messen.
  - Verträge: [index.json](verification/combined-native-v5-r3-large-pass-20261015/index.json), [index.json](verification/combined-workspace-v5-r3-20261015/index.json), [index.json](verification/combined-source-v5-r3-source-20261015/index.json), [index.json](verification/combined-source-v5-r2-clippy-failed-20261015/index.json), [index.json](verification/combined-source-v5-r2-source-20261015/index.json), [sitemap-location-paging-plan.md](parity/sitemap-location-paging-plan.md).
  - Gate-Stand: source_analysis: COMPLETE_BOUNDED_SOURCE_ONLY; implementation: IN_PROGRESS; rust_tests: PASS_ALL_NEW_CASES_ON_ACTUAL_R3_FULL_1256_SUITE; native_large_sources: NOT_RUN_ON_NEW_SOURCE; source_review: PASS_BOUNDED_SOURCE_ONLY_V2_AND_FORMAT_MERGE; actual_50000_reopen: NOT_RUN_ON_SOURCE5; performance: NOT_MEASURED; native_50000: PASS_FULL_INITIAL_AND_REOPEN_ACTUAL_WINDOWS.

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
  - [x] Physische native JSON8MiB-/CSV64MiB-Exaktgrenzen am installierten Release ab1 bestanden: tatsächliche vollständige HTTP200-Körper, atomare413-Gegenproben, kleinere/anschließende Snapshot-Lesezugriffe unverändert, echte Original-Captures und SQLite-Rows geprüft, null Website-Anfragen bei Lesezugriffen und alle eigenen Produzenten geschlossen/Verbindungen geleert. Abgewiesene +1-Körper werden nicht ausgeliefert: JSON-Größe durch tatsächlichen Rust-serde-Fakten-Witness, CSV durch gemessene Original-Quote-/ASCII-Schritte belegt.
  - [ ] Vorheriger S4-Entwicklungskandidat: ausdrücklich konfigurierbare Quellenverarbeitung bis 64 MiB/2 Mio. XML-Ereignisse mit unveränderten bisherigen Standards und Retentionsbudgets im Quellcode verbunden. 17 zusätzliche Rust-Regressionen geschrieben, 238 Eingabedateien eingefroren, künftig mindestens 957 Gesamttests und 175 erforderliche Namen. Workspace-Formatprüfung bestanden; Kompilierung, Tests, echte 9-/50-MiB-Fixtures und native/API-/SDK-/Browser-Abnahmen NICHT AUSGEFÜHRT. Praktische verifizierte SQL-Paginierung/Streaming und zugehörige Speicher-/Latenzabnahmen bleiben ausdrücklich offen. Keine S4-Vollabnahme oder Installation.
  - [ ] Vorherige S6-Flächenplanung: damalige Lücken für alle sieben Sitemap-Filter festgehalten. Gemeinsamer Abfrage-/Exportkern, getrennte Seiten-/Quellversuch-Domänen und API-/CLI-/MCP-/Browserflächen sind inzwischen als ungetesteter Quellcode verbunden; die aktuelle Flächenabnahme bleibt offen. Filter auf den ersten 200 geladenen Zeilen ersetzen diese Abnahmen nicht.
  - [ ] Vorheriger S3-Quellstand 2a01db3: S3-Quellprotokoll-Zweig: Messdatenmodell, fehlerbewahrende Dekodierung, XML-Zählung und tatsächlicher Fetch sind jetzt mit dauerhafter A-Beobachtung, frischer atomarer B-Zuordnung, Report/Checkpoint/Elternsnapshot und App-Cursorbindung als Quellcode integriert. 94 neue S3-Testfunktionen sind geschrieben; 234 Eingabedateien, mindestens 940 Gesamttests und 158 erforderliche Testnamen sind eingefroren. Workspace-Formatprüfung bestanden; Kompilierung, Rust-Tests, native/SDK-/Browser- und Crash/Restart-Abnahmen NICHT AUSGEFÜHRT. Statisch gefundene Elternhistorien-Lücke und Windows-Starterpfad korrigiert; der tatsächliche isolierte JS-Pfad-Control besteht. Öffentliche Filter und echte 50-MiB-Quellenabnahmen bleiben offen. Keine Hersteller-Parität oder Installation daraus abgeleitet.
  - [ ] Entwicklungskandidat c35b929: Sitemap-ZIP-Erzeugung, gespeicherte Ziel-/Inlink-/Mehrfachquellen-Auswertung und begrenzte MCP-stdio-Eingaben sind im privaten Zweig erfasst.218 Eingabedateien,52 zusätzliche Rust-Fälle und64 konkret erforderliche Testnamen sind eingefroren; mindestens846 erwartete Tests sowie native API-/CLI-/SDK-/Browser-/Renderer-Abnahmen sind vorbereitet, gemeinsam aber noch NICHT AUSGEFÜHRT. Installierter Stand bleibt ab1; Hersteller-Parität unverändert unbewiesen.
  - [ ] Budget4: Native einzelne Ledger-Zeile exakt256KiB und ein Byte darüber aus echter XML-Parserbeobachtung; neue Rows/Tasks müssen bei Überlauf gemeinsam zurückrollen.
  - [ ] Budget5: Nativer vollständiger Katalog exakt32MiB und ein Byte darüber über zwei reale Quellen; zuvor committed Quelle erhalten, zweite Quelle atomar ablehnen.
  - [ ] Budget6: Gezielte SQL-Fehler-Injektion ausschließlich offline in eigener bereits echt erzeugter Fixture-Datenbank; Ordinary/Policy-Rollback und Wiederaufnahme ohne erfundene Herkunft prüfen.
  - [ ] Budget7: Nur tatsächlich bestandene physische Grenzen veröffentlichen, gescheiterte Kandidaten erhalten und TODO/GitHub aktualisieren; Quell-CI/Release/Installation bleiben getrennt.
  - [ ] Live-Generationswechsel im nativen API-/Browserpfad prüfen.
  - [ ] GitHub-/Linux-/Container-CI nach Konto-Verfügbarkeit getrennt ausführen. Lokale794/0ausgespart, optimierter Release/native13 und datenbewahrende Installation sind bestanden; Remote-CI bleibt NOT_PROVEN.
  - [ ] Originale Mitgliedschaften an tatsächliche Fetch-/Inlink-Zustände anbinden und konkrete Sitemap-Zielfehler ausgeben; vollständige Website-Abdeckung bleibt unbewiesen.
  - [ ] Sitemap-Filter S1–S7 einzeln abnehmen: Quellen-/Ziel-/Inlink-/Mehrfachdokumente, tatsächliche 49.999/50.000/50.001 Einträge und 52.428.799/52.428.800/52.428.801 dekodierte XML-Bytes; aktuelle 8MiB-/Ledger-Ressourcenstopps ersetzen diese Protokollprüfung nicht. Generierte Segmentgrenzen sind keine Prüfung großer gefetchter Quellen.
  - [ ] Vorheriger S4-Raw-Quellstand (5091eb9): verifizierte Raw-/Source-SQL-Durchläufe, Same-TX-Store-Fenster und separater sitemap-rows-CLI-Leser integriert. 36 zusätzliche Regressionen geschrieben, 246 Eingabedateien und 211 erforderliche Namen eingefroren; künftig mindestens 993 Gesamttests. Workspace-Formatprüfung und begrenzte Quellreviews einschließlich zwei Quellkorrekturen bestehen. Kompilierung, alle neuen Tests und native Läufe NICHT AUSGEFÜHRT. Rich-Coverage-/Analyse-/Selected-Kontexte, echte Browser/API/MCP-Umstellung, CSV-Streaming, physische Größen/RAM/Latenz und vollständige sieben Filter bleiben offen. Keine S4-Vollabnahme oder Installation.
  - [ ] Vorheriger S4-Rich-Quellstand (d105fb8): vollständige Coverage-/Inventarhash-Projektion, gespeicherte Quellen-/Seitenanalyse und produktiven App-Inventarhandler mit kompaktem Same-TX-Fenster verbunden. Inventar und gepufferter CSV-Export teilen den neuen Zustandshash. 42 zusätzliche Regressionen geschrieben, 255 Eingabedateien und 253 erforderliche Namen eingefroren; künftig mindestens 1035 Gesamttests. Workspace-Formatprüfung und begrenzte Quellreviews einschließlich früher Feldnamen-Speichergrenze bestehen. Kompilierung, Tests und native/API-/MCP-/Browserabnahme NICHT AUSGEFÜHRT. Selected-Parent-Kompatibilität bleibt materialisierend; CSV-Streaming, tatsächliche Größen/RAM/Latenz, Recovery und sieben vollständige Filter bleiben offen. Keine S4-Vollabnahme oder Installation.
  - [ ] Vorheriger S4-CSV-Quellstand (c8f539b): gemeinsamer tatsächlicher Same-TX-Kontext, opake wiederholbare Allstored-Session, kontrollierte Reader und zwei Formatterdurchläufe mit begrenzter Queue4/64KiB sowie Endblock erst nach Core-Return und tatsächlichem Worker-Join integriert. Cancellation/Fristen/Drop/Shutdown-Drain als Quellcode vorhanden. 35 zusätzliche Testfunktionen geschrieben (9 Store/10 Control/3 Writer/13 App-Download); 265 Compiler-Eingaben und 288 Pflichtnamen eingefroren, künftig mindestens 1070 Gesamttests. Tatsächliche Workspace-Formatprüfung und begrenzte unabhängige Quellreviews bestehen; Compiler, Clippy, Tests und native/API-/CLI-/MCP-/Browserläufe NICHT AUSGEFÜHRT. Physische neue64MiB/+1-Grenze, Selected-Untervarianten, große Quellen/RAM/Latenz/Recovery und sieben vollständige Filter bleiben offen. Keine S4-Vollabnahme oder Installation.
  - [ ] Vorheriger S6-Core-Quellschritt (898c404): sieben getrennte Finding-Prädikate auf vollständigen gespeicherten SavedPage-/SourceAttempt-Domänen, typisierte PDF-/Robots-Qualifikation, tatsächliche A-only/exakteB/Legacy-/Task-Nachweise, vollständiger Taskhash auch ohne Raw-Katalog, gemeinsame verifizierte Read-TX, wiederholbare Allstored-Session und kontextgebundene dünne Trefferfortsetzungen implementiert. Globale Zusatzfeldprüfung vor Callback schließt ausgeschlossene beschädigte Berichtsteile ein. 53 zusätzliche Testfunktionen geschrieben (18 Root/11 Source/6 Seitenleser/18 Modell); 275 Compiler-Eingaben und341 Pflichtnamen eingefroren, künftig mindestens1123 Gesamttests. Tatsächliche Formatprüfung und begrenzte unabhängige Quellreviews bestehen; Compiler, Clippy, Tests und native Läufe NICHT AUSGEFÜHRT. API-/CLI-/MCP-/UI-Anbindung und eigener vollständiger Finding-Export bleiben zu implementieren, Größen/RAM/Latenz/Recovery/Selected/Linux/Container abzunehmen. Keine S6-Vollabnahme, keine Herstellerparität und keine Installation.
  - [ ] Aktueller S6-Flächenschritt: API, CLI, MCP und eigene Sitemap-Befundansicht über denselben vollständigen gespeicherten Finding-Kern integriert; getrennte Rollen für Seiten-/Quellenabfragen und vollständige Exporte. Kontrollierte Reader und zwei CSV-Durchläufe erhalten tatsächliche TX-/Worker-Ende-Grenzen, Originalfakten und Unknown-Qualifikation. Sieben bestätigte Produkt-/Harness-Quellbefunde separat korrigiert und begrenzt nachgeprüft. 47 zusätzliche Rust-Testfunktionen sowie13 Mock-Browsergruppen geschrieben, davon0ausgeführt. 297 Compiler-Eingaben und388 Pflichtnamen gemeinsam eingefroren; künftiger Workspace mindestens1170 Tests. Tatsächliche Format-/Syntaxprüfung, Hash-/Lineageprüfung, Source-Freeze und begrenzte Quellreviews bestehen; Compiler, Clippy, Tests, SQL-/HTTP-/CLI-/SDK-/Browserläufe NICHT AUSGEFÜHRT. Neue Whole-App8MiB-/FindingCSV64MiB-Grenzen, Quellmatrix, >200/400-Kandidaten, Abbruch/Join/Recovery/Selected/RAM/Latenz und native Installation noch abzunehmen. Historische ab1-Rohdatenbelege werden nicht übertragen. Keine S6-Vollabnahme, keine Installation und keine Herstellerparität.
  - [ ] Aktueller S4-Oberflächenschritt: ausdrücklich aktivierbares Sitemap-Quellenprofil in den Crawl-Einstellungen als Quellcode ergänzt. Drei native Grenzen werden getrennt geprüft und ohne Rundung übermittelt: Eingangsdaten/entpacktes XML je 1–67.108.864 Bytes, XML-Ereignisse 1–2.000.000. Ausgeschaltet fehlt das native Profil vollständig; Seitengröße und gespeicherte Mitgliedschaftsbudgets bleiben getrennt. Speichern/Wiederherstellen und UI-Metadatenbereinigung integriert. Separater UI-Quellstand e9bbc545… mit 297 Eingaben eingefroren: 295 unverändert, zwei UI-Dateien geändert, keine zusätzlichen Rust-Tests geschrieben oder ausgeführt. Die bisherigen 388 Pflichtnamen und mindestens 1170 künftig erwarteten Tests bleiben erhalten. Syntax-/Hash-/Diffprüfung und begrenztes unabhängiges Quellreview bestehen. Browser/Startpayload/größere echte XML-Quelle/320px/Compiler/Release/Installation bleiben offen; die Native-Prüfvorbereitung für den vorherigen 69492d0c…-Stand gilt nicht automatisch für diese UI-Änderung. Keine Feature-Abnahme oder Hersteller-Parität.
  - [ ] Vorherige native S6-Prüfvorbereitung für den älteren Quellstand: offizielle SDK-/API-/CLI-Prüfung und getrennte 13 Mock-Browsergruppen, 20 echte native Browsergruppen mit 405 gespeicherten Seiten sowie zwölf exakt/+1-Größenfälle für vollständiges App-JSON und Finding-CSV als Skripte vorbereitet. Bestätigte Prüfharness-Befunde in Quellkorrekturen geschlossen: feste Veröffentlichung, unveränderliche tatsächliche Prozessende-Frist, vollständiger API/CSV-Abgleich und gültige leere Formatlogs. Syntax-/Hash-/begrenzte unabhängige Quellreviews bestehen, sämtliche nativen Gruppen und physischen Fälle jedoch 0 ausgeführt. Größentests verwenden ausdrücklich synthetische Offline-Abzweige echter kleinerer Crawl-Baselines, keine angeblich beobachteten Website-Warnungen. Diese Prüfrouten gelten nur für den früheren 69492d0c…/f43efc1-Stand; die neue UIe9bbc545… braucht eigene Bindung und Ausführung. Kein Compiler-, Laufzeit-, Ressourcen-, Installations- oder Herstellerparitätsbeleg.
  - [ ] Aktuelle native Prüfrouten auf den neuen UI-Quellstand e9bbc545… gebunden: SDK/API/CLI, sechs aktuelle UI-Assets mit getrennten13 Mock-Gruppen,25 echte Browsergruppen einschließlich fünf neuer Formulargruppen sowie zwölf physische JSON-/CSV-Fälle. Aktuelle Suiteb4c0 und Builder1c72 samt exakter Quellherkunft und tatsächlichen Logs sind künftig zwingend; Belege des früheren694-Standes werden nicht übertragen. Zwei kleine UI-Starts und gespeicherte Konfiguration werden erst nach der unveränderten405/403-Lesephase geprüft und sind keine angeblich anfragefreien Befundleser. Quellreviews/Syntax/Hashprüfung bestehen, alle nativen Läufe und physischen Fälle0ausgeführt. Größere XML-Wirkung, vollständige Compiler-/Laufzeit-/Ressourcen-/Installationsabnahmen bleiben offen.
  - Verträge: [sitemap-b12-plan.md](parity/sitemap-b12-plan.md), [sitemap-parser.md](sitemap-parser.md), [sitemap-transport.md](sitemap-transport.md), [robots-sitemap-source.md](robots-sitemap-source.md), [sitemap-ledger.md](sitemap-ledger.md), [sitemap-membership.md](sitemap-membership.md), [sitemap-native-acceptance.md](sitemap-native-acceptance.md), [sitemap-budget-plan.md](parity/sitemap-budget-plan.md), [sitemap-budget-plan-source-pins.json](parity/sitemap-budget-plan-source-pins.json), [sitemap-assessment.md](sitemap-assessment.md), [sitemap-filter-depth-plan.md](parity/sitemap-filter-depth-plan.md), [sitemap-generation.md](sitemap-generation.md), [sitemap-generation-workflows.md](sitemap-generation-workflows.md), [sitemap-target-analysis.md](sitemap-target-analysis.md), [sitemap-output-target-preparation-v2.json](verification/sitemap-output-target-preparation-v2.json), [sitemap-source-measurement.md](parity/sitemap-source-measurement.md), [sitemap-source-protocol-preparation.json](verification/sitemap-source-protocol-preparation.json), [sitemap-source-fetch-preparation.json](verification/sitemap-source-fetch-preparation.json), [sitemap-source-durable-preparation.json](verification/sitemap-source-durable-preparation.json), [sitemap-source-processing.md](sitemap-source-processing.md), [sitemap-large-processing-preparation.json](verification/sitemap-large-processing-preparation.json), [sitemap-filter-surfaces-plan.md](parity/sitemap-filter-surfaces-plan.md), [sitemap-filter-surface-preparation.json](verification/sitemap-filter-surface-preparation.json), [sitemap-streaming-read-preparation.json](verification/sitemap-streaming-read-preparation.json), [sitemap-raw-window.md](sitemap-raw-window.md), [sitemap-rich-window.md](sitemap-rich-window.md), [sitemap-rich-read-preparation.json](verification/sitemap-rich-read-preparation.json), [sitemap-csv-streaming-plan.md](parity/sitemap-csv-streaming-plan.md), [sitemap-csv-streaming.md](sitemap-csv-streaming.md), [sitemap-csv-streaming-preparation.json](verification/sitemap-csv-streaming-preparation.json), [sitemap-findings-core.md](sitemap-findings-core.md), [sitemap-findings-core-preparation.json](verification/sitemap-findings-core-preparation.json), [sitemap-findings-surfaces.md](sitemap-findings-surfaces.md), [sitemap-findings-surfaces-preparation.json](verification/sitemap-findings-surfaces-preparation.json), [sitemap-source-processing-ui.md](sitemap-source-processing-ui.md), [sitemap-source-processing-ui-preparation.json](verification/sitemap-source-processing-ui-preparation.json), [sitemap-findings-native-verification.md](sitemap-findings-native-verification.md), [sitemap-findings-native-preparation.json](verification/sitemap-findings-native-preparation.json), [sitemap-native-ui-rebind-preparation.json](verification/sitemap-native-ui-rebind-preparation.json).
  - Gate-Stand: source_review: PARSER_AND_FETCH_NO_CONCRETE_P1_P2; implementation: MAIN_DURABLE_WRITER_AND_READONLY_API_CSV_MCP_UI; native_acceptance: PASS_LOCAL_TRANSPORT16_LEDGER11_MEMBERSHIP11_SEPARATE_SCOPES; work_parser: PASS_53_PLUS_EXISTING_NORMALIZER2; main_core_parser: PASS_54; main_core_clippy: PASS_ALL_TARGETS; main_standalone: PASS_56_SEPARATE; work_mutations: PASS_4_EXPECTED_RUNTIME_ASSERTION_FAILURES; exact_commit_ci: PASS_6597C9C_FOUR_JOBS_565_EACH; main_core_fetch_http: PASS_12; main_core_all_unit: PASS_CURRENT_CARGO_CORE510; main_portable_transport: PASS_16_FINAL_DRAIN; parser_exact_commit_ci: PASS_FOUR_JOBS_6171a32; optimized_transport_release: PASS_SAME_OPTIMIZED_EXE_16_NATIVE_SDK_BROWSER; installation: PASS_LOCAL_OPTIMIZED_EA314045_AB1AC93C_7JOBS_19FILES; robots_source_core: PASS_353_AND_ALLTARGETS_CLIPPY; transport_proof_validator: PASS_SYNTHETIC_1POSITIVE_24NEGATIVE_AND_ACTUAL_DEBUG_RELEASE_COMPANIONS; new_robots_validator_ci: PASS_6597C9C_FOUR_JOBS_FULL_RAW_ARCHIVE; durable_source_ledger: MAIN_PASS_NATIVE_LEDGER11_SQL11_DRAINED; durable_work_core: HISTORICAL_437_V4_GAP_FIXED_BY_V5; durable_original_binding: PASS_V4_17_EXPECTED_FAILURES_V5_ALL_21_CONTROLS; durable_full_crawler_bootstrap: PASS_8_INCLUDED_IN_CORE_508; durable_workspace_clippy: PASS_LOCAL_CARGO_ALLTARGETS_SAME_SOURCE; durable_source_cohort: PASS_25_CORE_AND_NATIVE13_QUALIFIED_DECLARED_SCOPE; durable_exact_commit_ci: NOT_PROVEN_GITHUB_ACCOUNT_UNAVAILABLE_LOCAL_SCOPE_SEPARATE; durable_api_csv_ui: PASS_ROUTED_NATIVE_API_CSV_SDK_BROWSER11; durable_installation: PASS_LOCAL_USER_AUTHORIZED_740_NATIVE_RELEASE_24TOOLS; durable_policy_invalid_locs: PASS_UNCHANGED_EXTERNAL_POLICY13_AND_CORE_DISCOVERY12; durable_account_gate: DEFERRED_REMOTE_CI_USER_AUTHORIZED_LOCAL_ROUTE_NO_SETTINGS_CHANGE; durable_workspace_tests: PASS_LOCAL794_0FAIL_0IGNORED_REAL_BROWSER; durable_actual_browser: PASS_NATIVE_CHROME_SAME_ASSETS_320PX; durable_official_mcp_sdk: PASS_BOTH_OFFICIAL_TRANSPORTS24_TOOLS; durable_native_matrix: PASS_WINDOWS_DEV11_SCENARIOS11_SQL_REQUEST_DRAIN; durable_native_byte_budgets: PASS_ACCEPTED_AB1_JSON8M_CSV64M_EXACT_PLUS1_ROW_CATALOG_STILL_PENDING; durable_native_generation_race: PENDING_LIVE_ACTIVE_GENERATION; durable_native_rollback_budget: PENDING_FAILURE_INJECTION; durable_budget_plan: TASKS1_2_3_AND_THEIR_PHYSICAL_PUBLICATION_PASS_4_5_6_PENDING; durable_live_renderer4: PASS_OWNED_EPHEMERAL_CURRENT_SOURCE; durable_optimized_release: PASS_LOCAL_WINDOWS_THIN_LTO_AB1AC93C; durable_lastmod: PASS_22_CORE_4_REAL_STORE_APP_AND_NATIVE_BROWSER; durable_assessment_installation: PASS_LOCAL_794_NATIVE13_INSTALLED_7JOBS_19FILES; durable_json_physical_budget: PASS_NATIVE_HTTP200_8388608_413_PLUS1_RUST_SERDE_WITNESS_RECOVERY; durable_csv_physical_budget: PASS_NATIVE_HTTP200_67108864_413_PLUS1_REAL_ORIGINAL_CAPTURE; generation_target_candidate: SOURCE_C35B929_FROZEN218_MIN846_AND64_NAMED_TESTS_AUTHORED_COMPILE_NATIVE_INSTALL_NOT_RUN; generation_target_native_preparation: TRACKED10_MODULES_GEN10_TARGET15_BROWSER16_RENDER8_PLANNED_ZERO_NATIVE_EXECUTIONS; source_protocol_s3_source: WIP94_AUTHORED_FROZEN234_MIN940_NAMED158_DURABLE_REPORT_PARENT_CURSOR_FMT_PASS_COMPILE_TEST_NATIVE_NOT_RUN; source_protocol_s3_completion: SOURCE_INTEGRATED_RUNTIME_ACCEPTANCE_PUBLIC_FILTERS_PHYSICAL50M_CRASH_RESTART_STILL_OPEN; source_large_processing_s4_source: WIP17_AUTHORED_FROZEN238_MIN957_NAMED175_CFG64M_FETCH_STORE_FMT_PASS_COMPILE_TEST_NATIVE_NOT_RUN; source_large_processing_s4_completion: COMPACT_RICH_CONTEXT_APP_CSV_TRANSPORT_SOURCE_INTEGRATED_SELECTED_MATERIALIZING_PHYSICAL_NATIVE_UI_MEMORY_PLATFORM_NOT_PROVEN; source_filter_s6_surfaces: WIP47_AUTHORED_FROZEN297_MIN1170_NAMED388_CORE_API_CLI_MCP_UI_FULL_CSV_FMT_SYNTAX_BOUNDED_REVIEWS_COMPILE_TEST_NATIVE_NOT_RUN; source_large_processing_s4_streaming: WIP36_AUTHORED_FROZEN246_MIN993_NAMED211_RAW_SOURCE_SQL_STORE_CLI_FMT_PASS_COMPILE_TEST_NATIVE_NOT_RUN; source_large_processing_s4_rich_read: WIP42_AUTHORED_FROZEN255_MIN1035_NAMED253_COVERAGE_ANALYSIS_STORE_APP_SHAREDCSVSTATE_FMT_STATICREVIEW_PASS_COMPILE_TEST_NATIVE_NOT_RUN; source_large_processing_s4_csv_transport: WIP35_AUTHORED_FROZEN265_MIN1070_NAMED288_SAME_TX_TWOPASS_QUEUE4_64K_POSITIVE_TAIL_CORE_RETURN_JOIN_CANCEL_DRAIN_FMT_STATIC_REVIEW_COMPILE_TEST_NATIVE_NOT_RUN; source_large_processing_s4_csv_physical: OPEN_NEW_ROUTE_67108864_67108865_ACTUAL_PREVIOUS_AB1_PRODUCER_SEED_ONLY; source_filter_s6_core: WIP53_AUTHORED_FROZEN275_MIN1123_NAMED341_TYPED_PREDICATES_FULL_STORED_SAME_TX_SPARSE_CURSOR_GLOBAL_TAIL_PREFLIGHT_FMT_STATIC_REVIEW_COMPILE_TEST_NATIVE_NOT_RUN; source_processing_ui: WIP_E9_BOUND_SUITE_BUILD_NATIVE_AND_FIVE_PROFILE_FORM_PROBES_AUTHORED_ZERO_RUNS_LARGE_XML_OPEN; findings_native_preparation: CURRENT_UI_E9_BOUND_SDK_API_CLI_MOCK13_NATIVE_BROWSER25_PHYSICAL12_AUTHORED_ZERO_RUNS.

- [ ] **MCP um die weiteren dokumentierten Abläufe ergänzen** — REFERENCE_SCOPE_GAP_CONFIRMED_NOT_IMPLEMENTED. Der geprüfte Grundablauf bleibt erhalten. Für den vollständigen Umfang fehlen zusätzliche Katalog-/Export-, Datei- und optionale Node-Funktionen. Deshalb wurde automation.mcp von erfüllt auf teilweise geprüft präzisiert; keine Produktfunktion wurde entfernt.
  - [x] Aktuelle offizielle Dokumentation mit dem tatsächlichen statischen26-Werkzeugregister und den erhaltenen historischen Basic-Belegen abgeglichen.
  - [ ] Expose and prove report, bulk-export, filter and data-field discovery with typed selected-field/format/segment/full-selection exports; existing snapshot and sitemap exports prove narrower contracts.
  - [ ] Implement configured base-directory text read/write, directory discovery/list/create and explicit disabled-by-default Node script/npm capabilities. Prove exact target containment, unsupported paths, defaults, output/error/deadline/actual child drain and retained outputs with independent fixtures.
  - [ ] Prove stored URL content/image/screenshot outputs, stable selected-crawl workflow and same source/config/error behavior using real official SDK clients on both transports. The static26-tool registry and historical11-tool basic workflow are not full reference MCP coverage.
  - Verträge: [mcp-reference-scope-audit-20261015.json](verification/mcp-reference-scope-audit-20261015.json), [feature-catalog.md](feature-catalog.md), [granular-acceptance.json](parity/granular-acceptance.json).
  - Gate-Stand: reference_docs: CHECKED_PUBLIC_PRIMARY_20261015; current_tool_inventory: STATIC26_NOT_RUNTIME; expanded_workflow: NOT_IMPLEMENTED_NOT_EXECUTED; manufacturer_parity: NOT_PROVEN.

- [ ] **Optionale lokale Dateifunktionen für MCP abnehmen** — WINDOWS_MODULE_TRANSPORT_AND_OFFICIAL_SDK_PASS_REMAINING_SCOPE_OPEN. Fünf optionale lokale Dateifunktionen bestehen29 Windows-Modul-/Transportgruppen, den vollständigen V3-Workspace und175 tatsächliche offizielle SDK-Operationsprüfungen auf STDIO und HTTP.26 Standard-Tools und31 aktivierte Tools sind tatsächlich geprüft. Windows-Junctions werden abgewiesen. HTTP-In-flight-Drain, Unix, weitere MCP-Funktionen und Installation bleiben offen; automation.mcp bleibt partial.
  - [x] Eigenes capability-relatives Dateimodul sowie CLI-/Bridge-/HTTP-/STDIO-Anbindung geschrieben; Typen und Pfade werden vor Dateioperationen geprüft, globale Pfadfreigaben entstehen nicht.
  - [x] Quellreviews einschließlich Rootkorrekturen für Response-Workerbesitz, tatsächliche Loopbackadresse, eigenen HTTP-Startupkanal und beobachtete Drain-Wartezustände abgeschlossen. Das sind keine Laufzeitbeweise.
  - [x] 31 neue Testgruppen geschrieben:29 gemeinsame Windows-Fälle und2 Unix-Fälle. Alle15 Windows-Modulgruppen bestehen nach der eng begrenzten Windows-Testkorrektur tatsächlich; alle300 Eingaben danach exakt und Cargo vollständig geschlossen.
  - [x] Windows-Root-Handle-Test besteht mit OS5/32-Fehlerprüfung und zwingend erfolgreicher gleicher Umbenennung samt identischen Dateibytes nach tatsächlichem Drain und Schließen des Grants.
  - [x] Alle14 Transportgruppen bestehen tatsächlich, darunter echte STDIO-Frames, EOF-/Outputfehler-Drain und eigenes HTTP-Bind/Startup für deaktiviert127.0.0.1 sowie aktiviert127.0.0.1 und127.0.0.2. Bestehender Crawl-Listenobjektvertrag ist erhalten; Testshape ausschließlich um das vorhandene crawls-Feld korrigiert.
  - [x] Nachfolgender tatsächlicher kombinierter V3-Lauf besteht mit 1203/0/0, allen421 Pflichtnamen, vier echten Renderer-Gates, Format und Clippy; siehe separaten V3-Beleg.
  - [x] Offizielle Clients bestehen175 Operationsprüfungen:60 erfolgreiche Operationen und115 erwartete Ablehnungen; beide aktivierten Transporte weisen je fünf innere Junction-Zugriffe mit unveränderten Außendaten ab. Beide Root-Junction-CLI-Starts werden natürlich mit Exit1 abgewiesen.
  - [ ] Tatsächliches kontrolliertes Leeren laufender HTTP-Dateiarbeit separat prüfen. Die erzwungene Windows-Testbereinigung ist dafür kein Beleg.
  - [ ] Zwei Unix-Gruppen, weitere Symlink-/Spezialdateifälle und passende Plattformabnahme ausführen.
  - [ ] Report-/Bulkexport-/Feld-/Filterkataloge, gespeicherte Inhaltsausgaben und ausdrücklich optionale Node/npm-Funktionen vervollständigen; Installation separat prüfen.
  - Verträge: [mcp-file-tools.md](mcp-file-tools.md), [preparation.json](verification/mcp-file-tools-source-20261015/preparation.json), [index.json](verification/mcp-file-tools-source-20261015/focused-module-first/index.json), [windows-root-test-repair.json](verification/mcp-file-tools-source-20261015/windows-root-test-repair.json), [index.json](verification/mcp-file-tools-source-20261015/focused-module-repaired/index.json), [index.json](verification/mcp-file-tools-source-20261015/focused-wiring-repaired/index.json), [index.json](verification/combined-workspace-v3-20261015/index.json), [index.json](verification/combined-optimized-native-v3-20261015/index.json).
  - Gate-Stand: source_review: BOUNDED_SOURCE_ONLY_NO_CONFIRMED_FINDINGS; authored_tests: 31_WRITTEN_29_WINDOWS_EXECUTED_AND_PASSED_2_UNIX_NOT_RUN; compile: PASS_ACTUAL_TEST_PROFILE_51_82_SECONDS; official_sdk: PASS_175_ACTUAL_OPERATION_ASSERTIONS; native_http_stdio: PASS_BOTH_OFFICIAL_SDK_TRANSPORTS; installed_runtime: UNCHANGED_AB1; manufacturer_parity: NOT_PROVEN; windows_root_test_repair: PASS_ACTUAL_MODULE_GROUP; transport_tests: PASS_14_ACTUAL_GROUPS_OWNED_HTTP_THREE_MODES; unix_tests: 2_AUTHORED_NOT_RUN; combined_v3_workspace: PASS_1203_INCLUDING_4_RENDERER_FMT_CLIPPY; windows_junction: PASS_INNER_AND_ROOT_CONTROLS; http_inflight_graceful_drain: NOT_PROVEN.

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

</details>

## Pflege und Beweisgrenzen

1. Die offene Abnahme im JSON als konkrete Fixture oder Runtime-Prüfung ausarbeiten.
2. Positive Fälle, Kontrollfälle, unbekannte Daten und Ressourcen-/Corpus-Grenzen zusammen prüfen.
3. Prüfergebnis, Artefakt, Engine-/Profilversion und den tatsächlichen Geltungsbereich im Katalog festhalten.
4. Erst bei vollständig erfüllter eigener Abnahme den Status auf `implemented` setzen; Referenzparität separat nachweisen.
5. Den Generator ausführen und mit `--check` prüfen. Historische Beweise bleiben erhalten.

Offline-Auswertung mit geändertem CSS-Inhaltsbereich braucht gespeichertes HTML beziehungsweise DOM. Ein bereits ausgewählter Text mit HTML-Hash reicht zur erneuten Auswahl nicht aus. Dieser zusätzliche Workflow bleibt offen, solange keine vollständigen Quellartefakte und entsprechende Offline-Prüfung existieren.

Reproduktionsbindung: SHA-256 des verwendeten Katalogs `fa5020eaebcc29a4a08a3807a18097fe88b41cd23bb594c9e2d8c58c64e5d84f`. Letzter im Katalog erfasster Beweisstand: `2026-10-16T09:44:20.980Z`. Laufende neue Arbeit aktualisiert frühere Beweise nicht.
