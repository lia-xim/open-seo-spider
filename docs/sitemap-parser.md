# B12: begrenzter Sitemap-Parser

Der Core enthält jetzt den unabhängigen XML-Beobachtungsparser `open-seo-spider/sitemap-xml-occurrences-v1` und einen Gzip-Dateientitätsdecoder. [Issue11](https://github.com/lia-xim/open-seo-spider/issues/11) und [FEATURE_TODO.md](FEATURE_TODO.md) verfolgen die noch offene vollständige Anbindung. Die Feature-IDs `sitemap.discovery` und `sitemap.coverage` bleiben teilweise geprüft; vollständige Screaming-Frog-Parität ist **NOT_PROVEN**.

## Was der integrierte Parser erhält

`observe_sitemap_xml` prüft das gesamte begrenzte XML-Dokument. Ein `loc` gehört nur zum passenden Sitemap-Namespace und unmittelbaren `url`- beziehungsweise `sitemap`-Elternknoten. Gleichnamige Erweiterungselemente erzeugen keine Seiten- oder Child-Sitemap-Ziele. Das Namespace-Profil ist entweder Sitemap0.9 oder ausdrücklich erlaubtes unqualifiziertes Legacy-XML. Der aktuelle XML-Namensparser unterstützt ASCII-QNames; gesetzmäßige Unicode-Elementnamen bleiben ein ausdrücklich abgewiesenes Profil und sind keine vollständige XML-Standardabdeckung.

Die Beobachtung trennt rohe XML-Elemente, innere XML-Lexeme, dekodierte ungekürzte Werte und normalisierte Fetch-Identitäten. Doppelte Einträge bleiben eigene Vorkommen. Fehlende `loc`-Elemente und ungültige URLs bleiben Diagnosen; mehrere direkte `loc`-Elemente machen das Dokument ungültig. `lastmod` bleibt ein erhaltenes XML-Feld ohne behauptete Datumsvalidität oder Aktualität.

An URL-Rändern wird ausschließlich XML-S-Whitespace entfernt. Interne Steuerzeichen und anderer Whitespace am Rand werden vor der bestehenden URL-Normalisierung zurückgewiesen. XML-Syntax, Entities, Attribute und Namespaces werden auch in ignorierten Erweiterungen geprüft. DTDs und externe Entities sind nicht Teil dieses Profils.

Eine unvollständige oder fehlerhafte Beobachtung kann bereits geschlossene Präfix-Vorkommen als Fakten behalten. Ihre `admission_locations()` liefert keine Ziele. Parser-Vollständigkeit belegt die Dokumentprüfung; Crawl-Frontier, Quelle, Crawl-Kohorte und Zielabruf brauchen eigene Zustände.

## Gzip und Grenzen

`decode_sitemap_entity` erwartet tatsächliche Bytes **nach** dem bestehenden HTTP-Content-Decoding. Es erkennt Gzip-Dateientitäten anhand ihrer Magic-Bytes; Dateiname und MIME-Typ wählen den Decoder nicht. Gzip wird genau einmal dekodiert. CRC, ISIZE, abgeschnittene Footer, alle verketteten Member und nachfolgende Bytes werden bis zum tatsächlichen EOF geprüft. Ein zusätzliches Sentinel-Byte erkennt Überschreitung auch bei exaktem Limit. Gzip-in-Gzip wird ausdrücklich abgewiesen.

Die Standardgrenzen sind 8MiB Eingabe und XML, 500.000 XML-Ereignisse, Tiefe64, 50.000 Einträge, 100.000 Loc-Vorkommen, 8.192 UTF-8-Bytes pro Feld sowie 32MiB erhaltene Eingabe-Strings. Nutzerdefinierte kleinere positive Grenzen sind möglich; größere Grenzen als die Standardobergrenzen werden abgewiesen. JSON-Hüllen, Metadaten und ein späterer vollständiger Crawl-Katalog benötigen eigene Grenzen. Dekodierte XML-Bytes werden nicht als JSON-Zahlenarray serialisiert.

## Bestehende Aufrufer und offene Anbindung

Die vorhandene `parse_sitemap(&str)`-Signatur bleibt erhalten und benutzt den neuen XML-Parser. Sie liefert weiterhin eine absichtlich reduzierte Liste nichtleerer dekodierter Werte an die bisherigen Normalisierungs-/Policy-Aufrufer. Diese Kompatibilitätsprojektion erhält kein vollständiges Vorkommensledger und verwendet nicht die strengere `admission_locations()`-Projektion. Ihre bisherigen URI-Entscheidungen belegen daher keine vollständige native Validierung des neuen Beobachtungsmodells.

Der Dateientitätsdecoder ist noch nicht im Netzwerkpfad angebunden. Ebenfalls offen sind ein expliziter Sitemap-Fetch-Zweck, atomare vollständige SQLite-Vorkommen mit einmaliger Completion, Ordinary/Policy-Abgleich, Quellvollständigkeit bei Fehlern, Checkpoint/Recovery, unbekannte Altbestände, ausgewählte Vererbung und read-only API-/CSV-/UI-Abnahmen. Der installierte Nutzer-Release bleibt B11.

## Belege

Der [Parser-Checkpoint](verification/sitemap-parser-checkpoint.json) bindet die sieben tatsächlichen Main-Quellen einschließlich des unveränderten URL-Normalisierers an SHA-256. Ein gezielter **Core-Cargo-Lauf besteht54 Tests**: 53 neue Parser-/Decoder-Tests und den erhaltenen historischen Test. Alltargets-Clippy besteht. Ein separater Standalone-Lauf auf Main besteht56 Tests inklusive zweier tatsächlicher URL-Normalisierer-Tests; diese Zahlen werden nicht addiert.

Vier isolierte absichtlich beschädigte Work-Varianten kompilieren und scheitern jeweils an der erwarteten gerichteten Laufzeit-Assertion: falsche Namespace-Zuordnung, Admission aus unvollständigem XML, Unicode-Trim am URL-Rand und übersprungene Gzip-EOF-Prüfung. Sie prüfen die Testempfindlichkeit des eingefrorenen Entwurfs. Unabhängige Quellenprüfung fand keinen konkreten P1/P2-Fehler. Exakte neue vollständige Quell-CI sowie native Netzwerk-/Speicher-/Release-/Installationsbelege stehen aus. Hashes binden lokale Eingabebytes und behaupten keine Quellauthentizität.

Reproduzierbare Core-Prüfung:

```text
cargo test -p open-seo-spider-core --lib --locked sitemap::
cargo clippy -p open-seo-spider-core --all-targets --locked -- -D warnings
```
