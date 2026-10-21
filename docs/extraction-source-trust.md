# Quellvertrauen eigener Extraktionen

Native CSS-, XPath- und Regex-Ergebnisse erhalten einen optionalen
`source_observation`-Beleg mit Vertrag
`open-seo-spider/extraction-source-v1`. Er beschreibt den tatsächlich
ausgewerteten, dekodierten Input. Er bescheinigt keine Website-Authentizität
und keine Gleichheit mit einem anderen Crawler.

`status` trennt `complete`, `incomplete` und `unavailable`.
`evaluation` trennt `evaluated`, `error` und `not_run`.
`decode_status` trennt `exact`, `lossy` und `unknown`. Zusätzlich werden
Dokument-URL, Beobachtungszeit, optionale native Generation, UTF-8-Länge,
SHA-256 des ausgewerteten Strings, Quellrevision und Gründe gespeichert.
Die Revision ist SHA-256 des kompakten JSON-Arrays
`[engine,source,document_url,observed_at,observed_generation,input_sha256,input_utf8_bytes]`;
fehlende optionale Werte stehen darin als `null`.

Ein vollständig beobachteter leerer String hat Länge `0` und den SHA-256 des
leeren Strings. Eine nicht verfügbare Quelle hat keinen Inputhash und keine
Inputlänge. Diese Fälle sind verschieden.

`matched_count` zählt weiterhin extrahierbare Werte im ausgewerteten Input.
`truncated` betrifft ausschließlich `max_results`. Die abgeleitete exakte
Gesamtanzahl ist nur bei gültigem, vollständigem Quellbeleg und erfolgreicher
Auswertung bekannt. Bei einem Fehler oder unvollständigen Input bleibt sie
unbekannt. Ein vollständiger Input mit drei Treffern und zwei gespeicherten
Werten hat deshalb die exakte Anzahl drei, obwohl `truncated` wahr ist.

Eine Anzahl aus einem Präfix ist keine allgemeine Untergrenze: Ein Regex mit
Endanker kann nur am abgeschnittenen Ende treffen; HTML-Reparatur und
XPath-Positionen oder skalare Funktionen können sich mit dem Rest des Dokuments
ändern. Gefundene Werte werden erhalten und als Auswertung dieses Inputs
gekennzeichnet. XPath-Zahl `0`, Boolean `false` und String `""` sind jeweils ein
extrahierter Wert; ein leerer Nodeset hat null Werte.

Vollständiges, exakt dekodiertes HTTP-HTML kann auch bei Status 404 vollständig
beobachtet sein. Bodylimit, Transport-/Lesefehler, Bereichsantworten,
No-Content-Status, unsichere Dekodierung und unbekannter Status verhindern
eine vollständige Einstufung. Frühe Abbrüche und nicht ausgewertete Inhalte
erhalten pro konfigurierter Regel einen `unavailable`-/`not_run`-Beleg.

Nach erfolgreichem Rendering besitzt das aktuelle Ergebnis einen eigenen
DOM-Beleg. Die rohe Beobachtung bleibt in `raw_metadata.custom_extractions`.
Ein optionaler Rendererfehler lässt einen vollständigen rohen Befund gültig.
Fehlende DOM-MIME-, Zeit- oder Statusbeobachtungen verhindern eine vollständige
DOM-Einstufung. Der DOM-Hash beschreibt die Browser-Serialisierung.

Historische Ergebnisse ohne dieses Feld bleiben lesbar und unbekannt.
Reine Bibliotheksaufrufe mit einem vom Aufrufer gelieferten String erhalten
keinen nativen Capture-Beleg. Vorhandene Belege werden beim Import,
Wiederöffnen, ausgewählten Recrawl und in nativen Speicherpfaden auf Struktur,
Revision und Ergebniskonsistenz geprüft. Das ersetzt keine unabhängige
Authentifizierung importierter Daten.

Die Tabelle und CSV/XLSX führen Quellstatus, Auswertung, Dekodierung,
exakte Gesamtanzahl, Zählbereich und Bindungsdaten. `Bestätigt keine Werte`
verlangt eine bekannte exakte Anzahl null. `Ergebniswerte begrenzt` betrifft
weiterhin nur die Ergebnisgrenze. Eigene Filter zeigen unvollständige,
unbekannte und nicht ausgewertete Quellen. Fehlende konfigurierte Ergebnisse
erscheinen als explizite unbekannte Zeilen ohne erfundene Matches.
Ohne Regeln und gespeicherte Beobachtungen wird der Tabellenexport verweigert.
Eine vorhandene Quelle mit null Abfragetreffern darf Kopfzeilen exportieren.

Native Extraktions-CSV behält ihre ersten 15 Spalten und ergänzt die
Quellinformationen. Bestehende benannte Extraktionsansichten müssen wegen
geänderter Nullbefund-Semantik und Spalten ausdrücklich neu angelegt werden;
andere Tabellen behalten ihre Abfrageverträge.

Die [lokale Abnahme](verification/extraction-source-trust-20261021/README.md)
prüft 19 tatsächliche CSV-/XLSX-Paare mit 712 Zeilen, vier native API-CSV-Dateien
mit 265 Zeilen, beide Ansichtsformate und kalten Neustart. 535 Node- und 1843
reguläre Rust-Tests sowie vier separat ausgeführte Core-Renderer-Tests bestehen.
Die [zusätzliche Ansichtsregression](verification/extraction-source-trust-20261021/existing-views-20261021/README.md)
ist ein eigener Zwei-Suiten-Aufruf. Die vollständigen 49 Suiten und die neue
öffentliche CI bleiben separate Gates. Der aktuelle Prüfstatus und offene
Grenzen stehen in [FEATURE_TODO.md](FEATURE_TODO.md). Dieser Vertrag schließt
den übrigen Extraktions-, Tabellen- und Herstellervergleich nicht ab.
