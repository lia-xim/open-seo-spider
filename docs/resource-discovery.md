# Ressourcen-Erkennung: eigener B9-Vertrag

Stand: **Die begrenzte B9-Grundlage ist geprüft und lokal installiert; breitere Feature-Abnahmen bleiben offen.** [Exakte Commit-CI](verification/resource-discovery-ci.json), [unveränderlicher Release](verification/resource-discovery-release.json) und [Installation mit erhaltenem Nutzerbestand](verification/resource-discovery-installed.json) bestehen. B10 wird anhand des [eingefrorenen Abnahmeplans](parity/resource-matrix-plan.md) implementiert; der [ausdrücklich wählbare V2-Vertrag](resource-contract-v2.md) und seine nativen Belege sind eigene Folgearbeit. Die [Feature-To-do-Liste](FEATURE_TODO.md) und [der Katalog](feature-parity.json) behalten 25 erfüllte eigene Abnahmen, 72 teilweise geprüfte und 47 offene Fähigkeiten. B8-native/API/UI-Beweise sowie [exakte B8-Commit-CI](verification/content-capture-ci.json) und [installierter Release](verification/content-capture-installed.json) bleiben historische B8-Belege. Hersteller-, Filter- und Workflow-Parität bleibt NOT_PROVEN.

## Behobene Neustartregression und historischer Fehler

Der [dreifach reproduzierte historische Fehler](verification/resource-discovery-restart-failure.json) am Build `1fc72a7772c2c8534fd5145feaf2e45384260b5fbefc61b1435417eea77a8c93` bleibt gespeichert: Synchrones Laden/SQLite-Recovery unter dem asynchronen Windows-Startup erschöpfte den Main-Thread-Stack. Die [lokale Reparatur](verification/resource-discovery-restart-repair.json) verlagert synchrones Parsing, SQLite und abgeleitete Analyse nach `spawn_blocking` und hält den Serve-Future auf dem Heap. Globale Stackgrenzen und historische Dateien wurden nicht verändert. Am neuen Build lädt die exakt fehlgeschlagene Fixture sechs fertige Jobs; die frische achtteilige Capture-/Restart-Prüfung und alle sieben bestehenden Regressionsgruppen bestehen. `legacy_restart_regression` ist jetzt **PASS**.

## Finaler lokal geprüfter Stand

Der eingefrorene Debug-Stand `142c0a1af3d420de5ccb1e182bdd04d701f8087a5dc72b3758464aa5e3d65724` besteht [362 Standard-Rust-Tests](verification/resource-discovery-core.json), darunter 15 native Ressourcen-Integrationsfälle und 15 CSS-Parser-Tests. Vier Live-Gates bleiben im Standardlauf ausdrücklich ignoriert; eine tatsächliche Chrome-Prüfung wurde separat bestanden. Formatierung und Clippy sind bestanden. Der [öffentliche B9-Aggregatbeleg](verification/resource-discovery-batch.json) bindet die aktuellen Core-/API-/MCP-/[UI65-Beweise](verification/resource-discovery-ui.json), sieben Regressionsgruppen sowie historischen Fehler und Reparatur per Hash. Der Abschlussbeleg ergänzt exakte f107-Commit-CI, Release und Installation mit ihren getrennten Artefakten.

Die [neun nativen API-/CLI-Gruppen](verification/resource-discovery-e2e.json) prüfen unter anderem 505 erhaltene Quellvorkommen, 506 unterschiedliche Ressourcen-URL-Transporte ohne feste 500-URL-Grenze, private Antwort-Metadaten, vollständige gepinnte Exporte und vier gezielte SQLite-Manipulationskontrollen. Vollständige typisierte Crawl-Konfiguration und Generation werden beim Lesen gebunden. Das [offizielle MCP-SDK](verification/mcp-e2e.json) prüft 22 Tools über stdio und Streamable HTTP.

Die [tatsächliche Ressourcen-UI](../web/verification/ui-resource-discovery-live-results.json) besteht 12 Prüfungen gegen Rust, eigene HTTP-Fixtures und SQLite. Weitere 53 tatsächliche Prüfungen der Content-/Policy-/Selected-/Control-/Renderer-Oberflächen bestehen am selben Debug-Stand. Die separate [Settings-Fixture](../web/verification/ui-resource-discovery-fixture-results.json) mit 10 und [Ledger-Fixture](../web/verification/ui-resource-ledger-fixture-results.json) mit 16 Prüfungen verwenden API-Mocks und ersetzen diesen nativen Beleg nicht.

Die [API-, CLI- und MCP-Verträge](resource-discovery-api.md) und [UI-Verträge](resource-discovery-ui.md) beschreiben die geprüfte Integration. Die breitere Zehn-Gruppen-Abnahme bleibt teilweise erfüllt: insbesondere ist die komplette vierfache Speichern-/Abrufen-Matrix bislang für Bilder belegt, nicht für jeden Ressourcentyp und jede Navigation. B9-CI, Release und Installation bestehen als eigene getrennte Gates; die vollständige Feature-Matrix bleibt offen.

## Zweck und Grenzen

Der native Crawler steuert Quellvorkommen und erlaubte Abrufe gezielt: HTML-Navigation, CSS, JavaScript-Dateien, Bilder, Medien, PDF, ältere SWF-Referenzen und andere Ressourcen. Eine deklarierte Quelle ist noch kein Browser-Request, ausgewähltes Bild, ausgeführtes Script oder tatsächlich beobachteter MIME-Typ. JavaScript-Quelltext wird nicht nach vermeintlichen URLs durchsucht.

Die optionale Konfiguration bleibt standardmäßig abwesend. Alte Crawl-Konfigurationen, gespeicherte Berichte und ihre Hashes behalten den früheren Pfad. Explizite Ressourcenschalter werden vor HTTP streng geprüft. Native Decoder-/Parser-Grenzen und fehlende Quellen werden in den eigenen Zuständen ausgewiesen.

## Quell- und Berechtigungsvertrag

1. **Speichern und Abrufen sind getrennt.** Ein nicht abgerufenes Ziel hat keine erfundene Antwort, Größe oder MIME-Angabe. Das Abschalten gespeicherter Antwortzeilen löscht keine bereits beobachtete Link-/Bild-/Beziehungsquelle oder notwendige dauerhafte Entscheidung. Abgerufene, aber nicht gespeicherte Ressourcen zählen weiterhin zum ausdrücklich konfigurierten Transportbudget.
2. **Navigation hat eine eigene Expansionserlaubnis.** Canonical, hreflang, Pagination, AMP, mobile Alternate, iframe, Refresh und PDF-Verweise werden getrennt gesteuert. Expansion ohne Abruf ist ungültig. Eine Ressourcen-Antwort mit HTML erlaubt allein keine Anchor-Expansion.
3. **Jedes Vorkommen behält seine Erlaubnis.** Dasselbe Ziel kann als Bild, Stylesheet und Anchor vorkommen. Abrufe dürfen zusammengeführt werden; frühere unveränderliche Grants dürfen dadurch nicht nachträglich erweitert werden. Eine unabhängig erlaubte Anchor-Quelle kann ihre eigene Navigationserlaubnis beitragen.
4. **Scope und Credentials bleiben getrennt.** Zusätzliche exakte HTTP(S)-Origins für Ressourcen erlauben keine beliebige Navigation und machen externe Links nicht intern. URL-Policy/Robots bilden die obere Erlaubnisgrenze; ursprüngliche Credential-Origins werden weder durch CDN-Konfiguration noch Redirects erweitert.
5. **Native und Browsersteuerung sind getrennte Profile.** Die Kombination der neuen nativen Ressourcenschalter mit Rendering wird zunächst vor HTTP abgelehnt, solange Chromium-Subrequests nicht unter denselben Regeln kontrolliert sind. Bestehendes Rendering ohne die neue Konfiguration bleibt ein eigenes Verhalten. currentSrc, Media-/Codec-Auswahl und aktive Fallbacks bleiben nativ unbekannt.

## HTML-Quellen

| Tatsächliches Vorkommen | Quellrolle | Beobachtungsgrenze |
|---|---|---|
| img src/srcset, picture/source srcset | Bildkandidat | Descriptor/Alt/DOM-Pfad und Vorkommen erhalten; Auswahl unbekannt |
| video poster | Bildkandidat | vom Video-/Medien-src getrennt |
| link stylesheet, CSS Import | Stylesheet | tatsächliche CSS-Antwort erst nach MIME-/Body-/Decode-Prüfung |
| script src | JavaScript-Datei | keine Ausführung oder geratenen Modulimports |
| audio/video src und direkte source src | Medienkandidat | unmittelbarer geparster DOM-Elternknoten bestimmt Kontext |
| object data, embed src | Embedded-Kandidat | deklarierter Typ/Extension sind Hinweise, Antwort-MIME separat |
| iframe, Refresh und Dokumentbeziehungen | eigene Navigation | deklarierte Quelle und erlaubte Expansion getrennt |

Ungültige, leere, nicht-HTTP und lokale Fragmentreferenzen bleiben beobachtbar, ohne daraus ein künstliches HTTP-Ziel zu machen. Native Parsing-Reparaturen sind der DOM-Kontext; sie beweisen keine aktive Browserauswahl. [HTML source](https://html.spec.whatwg.org/multipage/embedded-content.html#the-source-element), [video poster](https://html.spec.whatwg.org/multipage/media.html#attr-video-poster), [object/embed](https://html.spec.whatwg.org/multipage/iframe-embed-object.html#the-object-element).

## CSS-Inventar

Das eigene pure Modul `css_discovery` nimmt gelieferten dekodierten UTF-8-CSS-Text, tatsächliche Basis-/Quell-URL, Quellrevision und Inline-Kennzeichen entgegen. Es macht keine HTTP-Anfrage und erteilt keine Abrufberechtigung. Der aufrufende Crawler muss tatsächlichen MIME-Typ, Dekodierung, Body-Vollständigkeit und Response-Herkunft binden.

Jede Abhängigkeit behält Originalspanne in UTF-8-Bytes, dekodierten URL-Wert, getrennte Auflösung/Fehler, Property-/At-rule-Kontext, Vorkommensordinal und Kategoriehinweis. Imports sind Stylesheet-Quellen; image-set/image-Strings zählen nur an ihren URL-Operandpositionen. Typstrings, lokale Fontnamen, Namespaces, Selector- und gewöhnliche Content-Strings sind keine Abrufziele. Image-set-Auflösung/Typ werden erhalten, ohne einen Gewinner auszuwählen. Eigene globale Byte-/Token-/Tiefe-/Kandidaten-/Diagnostik-Grenzen werden positiv validiert; Überschreiten scheitert atomar statt einen vollständigen Prefix vorzutäuschen. [cssparser Parser](https://docs.rs/cssparser/0.37.0/cssparser/struct.Parser.html), [CSS Syntax](https://www.w3.org/TR/css-syntax-3/#consume-ident-like-token), [CSS Images](https://www.w3.org/TR/css-images-4/#image-set-notation), [Namespaces](https://www.w3.org/TR/css-namespaces-3/#declaration).

Externe CSS-Referenzen verwenden die finale Stylesheet-URL; eingebettete Styles verwenden die Dokumentbasis. Bedingte Regeln, Cascade, Custom-Property-Anwendung und tatsächliches Laden werden nicht aus dem Inventar abgeleitet. Syntaxfehler, unbekannte Grammatik und unvollständige Quellen brauchen eigene Diagnose-/Coverage-Zustände. [CSS relative URLs](https://www.w3.org/TR/css-values-4/#relative-urls).

### Begrenztes Encoding-Profil

Der native B9-Decoder verwendet laut aktuellem Core-Vertrag BOM, dann gültiges HTTP-Charset, dann die exakt am Anfang stehende `@charset`-Byte-Deklaration und schließlich UTF-8 als Fallback. Die Environment-Encoding-Herkunft aus dem verweisenden Dokument oder importierenden Stylesheet wird bislang nicht mitgeführt. Deshalb bleiben mehrdeutige nicht-ASCII-Quellen ohne solche expliziten Angaben unbekannt; reine ASCII-Quellen können im eigenen Profil eindeutig verarbeitet werden. Das reine CSS-Modul erhält bereits dekodierten Text und beweist diese Byte-Dekodierung nicht.

Die öffentliche CSS-Spezifikation berücksichtigt vor dem UTF-8-Fallback auch ein geliefertes Environment-Encoding. Diese verbleibende Lücke verhindert eine Aussage über universelle CSS-Encoding- oder Browser-Parität. [CSS Syntax: Eingabebytes und Environment-Encoding](https://www.w3.org/TR/css-syntax-3/#input-byte-stream).

- [ ] **`css-encoding-environment-fallback`:** Eigene Legacy-HTML- und importierte Stylesheet-Quellen mit nicht-ASCII-URL-Bytes, ohne BOM/HTTP-Charset/`@charset`, gegen ASCII-, explizite UTF-8- und widersprüchliche BOM-/HTTP-Kontrollen prüfen. Zunächst den unbekannten Herkunftszustand und null daraus unberechtigt abgeleitete Zielanfragen belegen; Environment-Encoding-Vererbung bleibt ein separater offener Vertrag.

## Wiederaufnahme und gespeicherte Entscheidungen

Quellvorkommen, Grants, Transport-/Parser-Beobachtung, offene Expansion, Policy-Ledger und Frontier müssen zusammen mit der Seite atomar speichern. Offene Parser-/Expansionseinheiten verhindern falsche Completion. Pause stoppt neue Admission und lässt bereits zugelassene Einheiten auslaufen. SQLite-Recovery verwendet gespeicherte tatsächliche Fetch-Identitäten, gültige Grants und unveränderte Config-/Quellrevisionen; URL-Rewrites laufen nicht erneut. Ausgewählter Recrawl lädt nur ausgewählte gespeicherte Ziele und schaltet neue Navigation/CSS/XML-Expansion ab. Nicht ausgewählte Elternbeobachtungen bleiben unverändert.

## Zehn native Abnahmegruppen: geprüft und verbleibend

| Gruppe | Begrenzter eigener Beleg | Verbleibende benannte Kontrollen |
|---|---|---|
| HTML-Inventar | Native Quellfamilien, Poster/Source-Kontext, responsive Duplikate und lokale Fragmente geprüft | `image-occurrence-alt-and-selected-source`: mehrere Quellseiten, fehlendes/leeres/Whitespace-Alt, zusätzliche DOM-Reparaturen; Browser-Auswahl bleibt separat |
| Speichern/Abrufen | Alle vier Kombinationen für Bilder geprüft, private Metadaten und optionaler Bericht erhalten | `resource-products-every-type-and-navigation`: vollständige Matrix für CSS/JS/Medien/PDF/SWF und Navigation |
| Rollen-Grants | Bild/Anchor, spätere Navigation/Store und einmaliger Transport geprüft | `resource-role-unions-reverse-and-concurrent`: umgekehrte/parallele Rollen anderer Typen |
| MIME/Basis/Decode | Antwort-MIME, finale CSS-Basis und echte Antwort-/Keine-Antwort-Hashes geprüft | `css-encoding-environment-fallback`: vererbte Encoding-Herkunft; zusätzliche fehlende/widersprüchliche Angaben und Body-Caps |
| CSS-Tokens | 15 deklarierte Parserfälle einschließlich Escapes, EOF, indirektem image-set und Originalspannen bestanden | Unterstütztes eigenes Profil ist geprüft; keine vollständige CSS-Grammatik- oder Browser-Ladeabnahme |
| CSS-Kontext | Native Imports/Zyklen und unabhängige fehlerhafte CSS-Einheiten geprüft | `css-native-grammar-context-and-decoding`: weitere Fonts, moderne Kontexte, Bedingungen und Decode-Revisionen |
| Scope/Robots/Credentials | Zwei Origins, erlaubtes/gesperrtes Ziel, Credential-Grenze und Policy/check-only geprüft | `resource-cross-origin-redirect-hop-credentials`: mehrgliedrige Redirects je Ressourcentyp und jede verbotene Stufe |
| Pause/Recovery/Integrität | Pause, Wiederaufnahme, atomare Rollbacks, Tamper-Reject und gepinnte gleichzeitige Leser geprüft | `resource-interruption-each-stage`: Prozessabbruch an jedem Parser-/Grant-/Expansion-Punkt |
| Budgets | Quell-/Gesamtbudget-Rollback, private Transportgrenze und mehr als 500 Ziele geprüft | `resource-budget-exact-plus-one-all-limits`: jede einzelne Grenze exakt/plus-eins und größere erlaubte Limits |
| Ausgewählte Ressourcen | Keine neue Expansion, unveränderte Eltern-/Restdaten und immutable Eltern-Grants geprüft | `resource-selected-all-kind-targets`: CSS/Bild/PDF/Navigation sowie Fehler-/Pending-Matrix |

Zusätzlich offen sind `relationship-discovery-each-positive-negative`, `link-directive-four-way-traversal` und `resource-native-renderer-enforcement`. Konfigurationsfelder oder native Kandidaten ersetzen ihre tatsächlichen positiven und negativen Requestkontrollen nicht.

- [x] Begrenztes natives Ressourcen-Subset, API/CLI/MCP und tatsächliche Browser-UI geprüft.
- [x] Eigene exakte B9-Commit-CI: vier Jobs am unveränderlichen f107-Source-Commit bestanden.
- [x] Eigener unveränderlicher B9-Release und installierte Runtime: zusätzliche Release-API9, Capture-/Neustart8, MCP22 und UI12 bestanden; sieben Jobs/elf JSON-Dateien byteidentisch erhalten.

Private Transport-Projektionen enthalten begrenzte Metadaten und keine Body-/Custom-Extraction-/PDF-/Capture-Texte. `retained_response` beschreibt die aktuelle Aufnahme in den Bericht; ursprüngliche Projektionsfakten bleiben getrennt. `completed_transports` zählt Frontier-Fetch-Einheiten, nicht jede Redirect-Stufe oder Discovery-Protokollanfrage. Integritätsprüfung pro Seite priorisiert Korrektheit; große Ledger benötigen zusätzliche Performance-Gates.

Diese Abnahme ist ein eigener nachvollziehbarer Vertrag; sie beweist keine vollständige Screaming-Frog-Gleichheit.
