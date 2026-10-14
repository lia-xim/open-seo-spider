# Kompakter Sitemap-Kontext und Inventarzugriff

Dieser Bericht beschreibt den vorherigen Rich-Kandidaten `d105fb8`.
Der darauf aufbauende [CSV-Streaming-Kandidat](sitemap-csv-streaming.md)
ersetzt dessen gepufferten Export im Quellcode; seine Laufzeitabnahme ist offen.

Status: **Quellcodekandidat, noch nicht kompiliert oder ausgeführt.** Der Zweig
`codex/sitemap-rich-read-20261014` baut auf `5091eb9` auf. Die installierte
Anwendung erhält dadurch noch keine Änderung. Die Abnahme von S4 und die
vollständige Herstellerparität bleiben offen.

42 zusätzliche Testgruppen sind geschrieben: 15 Coverage-, 10 Analyse-,
9 Store-/Zustands-/Parent- und 8 App-/Handler-Gruppen. Davon sind **0
ausgeführt**. Die früheren 246 eingefrorenen Eingaben und deren mindestens
993 erwartete Gesamttests bleiben als eigener Kandidat erhalten; der neue
Stand bindet 255 Eingabedateien und 253 erforderliche Testnamen. Mindestens
1035 bestandene Workspace-Tests werden für die eigene vollständige Abnahme
verlangt; das ist eine Erwartung, keine Ergebniszahl.

Der neue Core-Aufruf
`CrawlStore::sitemap_read_window_with_membership_analysis` verbindet das
begrenzte Rohdatenfenster mit der vollständigen Quellenabdeckung und dem
Kontext gespeicherter Seiten. Der bestehende HTTP-Inventarhandler verwendet
diesen Aufruf im Kandidaten. Der bestehende MCP-Zugriff leitet an denselben
HTTP-Endpunkt weiter; ein tatsächlicher neuer MCP- oder Browserlauf ist damit
noch nicht belegt.

## Daten und Zustand

Konfigurationsbytes, Generation, vollständige rohe Zeilen, Quellmessungen,
Discovery-Aufgaben, Coverage und gespeicherter Bericht werden innerhalb einer
tatsächlichen SQLite-Lesetransaktion geprüft. Gewöhnliche Reads halten keinen
vollständigen Rohkatalog, Quellmessungsvektor oder Task-Vektor in Rust vor.
Die vollständigen Zeilen werden trotzdem geprüft; ein beschädigtes Ende darf
keine erfolgreiche erste Seite erzeugen.

Der Coverage-Inventarhash entspricht weiterhin der bisherigen kanonischen
Serialisierung des vollständigen Inventars. Alle Tasks einschließlich noch
offener Aufgaben gehen ein. SQL-Sortierung und Identitätsvergleiche verwenden
explizit BINARY, damit importierte NOCASE-Spalten Groß-/Kleinschreibung nicht
zusammenfassen. Fehlende historische Daten bleiben unbekannt; ein tatsächlich
vorhandener leerer Bestand bleibt davon unterscheidbar.

Die Mitgliedschaft gespeicherter Seiten zählt passende Originalvorkommen und
Quellbeobachtungen über den gesamten verifizierten Bestand. Sichtbares
Zeilenfenster, Quellenfilter und unabhängig paginierte gespeicherte Seiten
begrenzen diese Zählung nicht. Originale Identitäten und zugelassene Aliase
bleiben getrennt. Der gespeicherte Source-Bericht wird zeilenweise mit dem
tatsächlichen SQL-Zustand verglichen; eine passende Zusammenfassung reicht
dafür nicht aus.

Vor dem JSON-Parser begrenzt eine zusätzliche geliehene Byteprüfung äußere
Berichtsfeldnamen auf 6144 kodierte Inhaltsbytes. Danach gilt weiterhin die
semantische Grenze von 1024 dekodierten UTF-8-Bytes. So kann auch ein früh
escaped, danach sehr großer Feldname keinen großen Parserpuffer aufbauen,
bevor die Grenze greift. Der zusätzliche vollständige Berichtsdurchlauf und
der verbleibende SQLite-/Parser-Speicher müssen praktisch gemessen werden.

Das gemeinsame Zustandsprofil
`open-seo-spider/sitemap-rich-sql-window-v1` bindet diese Kontexte sowie die
Analyse- und Lastmod-Engines. Ein erwarteter Cursor-Zustand wird vor der
Fortsetzungsposition geprüft. Veraltet, ungültige Position und überschrittenes
Antwortbudget besitzen getrennte Core-Fehlertypen und HTTP-Status 409, 400 und
413. Sonstige beschädigte Daten werden dadurch nicht zu Cursorfehlern.

## Bestehende Oberflächen

Die bisherige Inventar-JSON-Struktur, Filter, unabhängigen Cursorrollen,
Lastmod-Projektion und vollständige 8-MiB-Antwortgrenze bleiben erhalten.
Der zusätzliche interne Core-Datentransfer mit Headern hat eine getrennte
16-MiB-Grenze. Ein Fenster ist kein portabler, abgeschnittener Rohkatalog.

Inventar und bisheriger CSV-Export verwenden denselben neuen Zustandshash.
Neue Inventar-Tokens sollen dadurch unmittelbar für CSV funktionieren.
Strukturell gültige Tokens des alten Hashprofils bleiben parsebar und werden
als veraltet abgewiesen. Nur bei gleichzeitig veraltetem Token und abweichendem
App-/Datenbankzustand kann jetzt `stale_sitemap_cursor` vor
`sitemap_job_snapshot_conflict` zurückkommen; beide haben HTTP 409. Erste
Anfragen und die Prüfung nach erfolgreichem Read behalten ihren bisherigen
Konfliktcode. Fehlende Jobs behalten HTTP 404.

CSV bleibt vollständig materialisiert und bis 64 MiB gepuffert. Spalten,
Quoting, Formelschutz und ursprüngliche typisierte Zeilen bleiben im
Kandidaten unverändert. Rückstaukontrolle, Abbruch, Producer-Ende und begrenzter
Streaming-Transport sind ein eigener offener Arbeitsschritt.
Der [konkrete CSV-Streaming-Entwurf](parity/sitemap-csv-streaming-plan.md)
beschreibt diesen nächsten Schritt einschließlich 14 erst geplanter
Testgruppen; er ist noch keine Implementierung oder Abnahme.

## Kosten und offene Nachweise

Ausgewählte Wiederholungs-Crawls behalten sämtliche vollständigen Parent-,
Plan- und Präsenzprüfungen. `materializing_selected_compatibility` kennzeichnet
diesen Pfad auch bei historisch fehlendem Rohkatalog. Konfigurationsdaten,
Importe, globale Policy-Prüfung, Graphidentitäten sowie SQLite-Sorts und Caches
haben weiterhin eigene Kosten. Eine bereits notwendige Policy-Prüfung wird
derzeit teilweise wiederholt. Logische Größenlimits sind kein RSS-Nachweis.

Die neuen Tests vergleichen tatsächliche Store-/Crawler-Fixtures mit dem
bisherigen vollständigen Leser und prüfen unter anderem WAL, unbekannt/leer,
Parent-Präsenz, Quellmessungen vor und nach Commit, vollständige Mitgliedschaft,
alte/neue Tokens, CSV-Konvergenz und Antwortgrenzen. Sie sind geschrieben,
aber noch nicht ausgeführt. Format- und begrenzte Quellprüfungen ersetzen
keinen Compiler- oder Laufzeitbeleg.

Offen bleiben die kombinierte Kompilierung und Testabnahme, native API-/CLI-/
MCP-/Browserläufe, echte große Quellen, physische Grenzen, Speicher und Latenz,
Recovery, Linux/Container sowie die sieben Filter über alle gespeicherten
Kandidaten. Die [Feature-To-do-Liste](FEATURE_TODO.md) und der
[Tiefenplan](parity/sitemap-filter-depth-plan.md) bleiben maßgeblich.
