# Ressourcen-Erkennung: eigener B9-Vertrag

Stand: B9 in Design/Implementierung, noch keine bestandene B9-Abnahme. Die [Feature-To-do-Liste](FEATURE_TODO.md) und [der Katalog](feature-parity.json) behalten 25 erfüllte eigene Abnahmen, 72 teilweise geprüfte und 47 offene Fähigkeiten. B8-native/API/UI-Beweise sowie [exakte B8-Commit-CI](verification/content-capture-ci.json) und [installierter Release](verification/content-capture-installed.json) bleiben bestehen; B9-Commit-CI, Release und Installation werden separat nachgewiesen. Hersteller-, Filter- und Workflow-Parität bleibt NOT_PROVEN.

## Zweck und Grenzen

Der native Crawler soll Quellvorkommen und erlaubte Abrufe gezielt steuern: HTML-Navigation, CSS, JavaScript-Dateien, Bilder, Medien, PDF, ältere SWF-Referenzen und andere Ressourcen. Eine deklarierte Quelle ist noch kein Browser-Request, ausgewähltes Bild, ausgeführtes Script oder tatsächlich beobachteter MIME-Typ. JavaScript-Quelltext wird nicht nach vermeintlichen URLs durchsucht.

Die neue optionale Konfiguration bleibt standardmäßig abwesend. Alte Crawl-Konfigurationen, gespeicherte Berichte und ihre Hashes behalten den früheren Pfad. Explizite Ressourcenschalter werden vor HTTP streng geprüft. Der genaue öffentlich exponierte Transport-/API-Schema-Stand wird erst mit der Integration festgeschrieben; dieser Vertrag kündigt keine bereits verfügbare API an.

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

## Wiederaufnahme und gespeicherte Entscheidungen

Quellvorkommen, Grants, Transport-/Parser-Beobachtung, offene Expansion, Policy-Ledger und Frontier müssen zusammen mit der Seite atomar speichern. Offene Parser-/Expansionseinheiten verhindern falsche Completion. Pause stoppt neue Admission und lässt bereits zugelassene Einheiten auslaufen. SQLite-Recovery verwendet gespeicherte tatsächliche Fetch-Identitäten, gültige Grants und unveränderte Config-/Quellrevisionen; URL-Rewrites laufen nicht erneut. Ausgewählter Recrawl lädt nur ausgewählte gespeicherte Ziele und schaltet neue Navigation/CSS/XML-Expansion ab. Nicht ausgewählte Elternbeobachtungen bleiben unverändert.

## Zehn noch offene native Gate-Gruppen

- [ ] HTML-Inventar: alle Quellfamilien, Base, DOM-Reparaturen, Alt/Vorkommen, Poster, Source-Eltern und ungültige/nicht-HTTP Kontrollen.
- [ ] Speichern/Abrufen: alle vier Kombinationen pro Typ und Navigation, exakte Zielanfragen und fehlende Ziel-Fakten bei Nichtabruf.
- [ ] Rollen-Zusammenführung: Anchor/Bild-Reihenfolge, unabhängige Check-/Navigation-Grants, spätere Expansion höchstens einmal, kein Doppeltransport.
- [ ] MIME/Basis/Dekodierung: widersprüchliche Typ-/Extension-Hinweise, Redirect-finale CSS-Basis, fehlender MIME, Body-Cap und Unicode/Charset.
- [ ] CSS-Tokens: Escapes, quoted/unquoted, Import, image-set/image, Comments, Daten-/Fragmentwerte, Original-UTF-8-Spannen und echte Syntaxfehler.
- [ ] CSS-Kontext: verschachtelte Regeln/Imports, Zyklen, Font-Quellen, Custom-Properties, Bedingungen und unterschiedliche Decode-Revisionen.
- [ ] Scope/Robots/Credentials: zwei eigene Origins, CDN-Freigabe, deny/check-only, Redirect und null verbotene Zielanfragen.
- [ ] Pause/Crash/Recovery: aktive HTTP-/Parser-Barrieren, offene späte Grants, atomare Rollbacks, Tamper-Reject vor HTTP und Legacy-None.
- [ ] Budgets: exakt/plus-eins Grenzen, kein gespeicherter Prefix, kontrollierte größere Limits und 501 Ziele ohne feste URL-Sperre.
- [ ] Ausgewählte Ressourcen: gespeicherte CSS/Bild/PDF-/Navigation-Ziele, Fehler/Pending ohne Altdaten-Fallback, Restdaten unverändert, keine neue Expansion.

API/CLI/MCP, tatsächliche Browser-UI, exakte Commit-CI, unveränderlicher Release und installierter Runtime sind nach diesen Gruppen eigene Gates. Bis ihre Beweise existieren bleibt jede Checkbox offen. Diese Abnahme ist ein eigener nachvollziehbarer Vertrag; sie beweist keine vollständige Screaming-Frog-Gleichheit.
