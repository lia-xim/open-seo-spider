# CSV-Export über den vollständigen gespeicherten Sitemap-Bestand

Status: **Quellcodekandidat; Compiler und Laufzeitabnahme offen.** Der Zweig
`codex/sitemap-csv-streaming-20261014` baut auf `d105fb8` auf. Die installierte
Anwendung wird dadurch nicht geändert. Herstellerparität und S4-Vollabnahme
sind weiterhin nicht belegt.

35 zusätzliche Testfunktionen sind geschrieben: neun Store-/Session-, zehn
Control-/Reader-, drei Writer- und 13 App-/Download-Gruppen. Davon sind
**0 ausgeführt**. Die 13 App-Gruppen bilden 14 geplante Abnahmethemen ab;
die physische neue CSV-Grenze und einzelne gerichtete Varianten bleiben
ausdrücklich offen. Testanzahl ist kein Abdeckungsprozentsatz.

## Gemeinsame Datenbasis

Inventar und CSV bereiten innerhalb einer tatsächlichen SQLite-Lesetransaktion
dieselben Rohdaten-, Quellen-, Coverage- und Analysekontexte vor. Der neue
öffentliche Core-Aufruf `CrawlStore::with_verified_sitemap_stored_rows`
übergibt eine nur innerhalb des Callbacks gültige Session. Sie hat keinen
öffentlichen Konstruktor, SQL-Zugriff oder nachgebauten Snapshot.

Der Filter steht vor dem Read fest. Wiederholte Visits lesen alle passenden
gespeicherten Zeilen in derselben Transaktion. Sie durchlaufen weder nur die
ersten 200 Zeilen noch eine Schleife begrenzter Inventarseiten. Die vollständige
Integritätsprüfung bleibt auch für Zeilen außerhalb des Filters erforderlich.

Für vorhandene Kataloge wird der vollständige erwartete Zustand vor dem
Callback geprüft. Tatsächlich fehlende historische Kataloge behalten im
CSV-Handler den bestehenden Fehler `sitemap_catalog_unavailable` mit HTTP 409;
sie werden nicht zu erfolgreichen leeren Dateien. Ein tatsächlich vorhandener
leerer Bestand bleibt davon unterscheidbar. Der bestehende Rich-Inventarread
prüft seinen erwarteten Zustand weiterhin vor der Fortsetzungsposition.

Das bisherige Zustandsprofil, Tokenformat, Originalidentitäten, CSV-Spalten,
Quoting, CRLF und Formelschutz bleiben erhalten. Ein frischer vollständiger
Inventartoken soll auch den neuen CSV-Pfad öffnen. Nur die alten gepufferten
CSV-Funktionen dienen weiterhin als Testorakel.

## Zwei Durchläufe und begrenzte Puffer

Ein blockierender Worker öffnet einen eigenen Read-only-Store. Der erste
Formatterdurchlauf zählt und hasht die tatsächlichen CSV-Bytes. Die inklusive
Grenze bleibt 67.108.864 Bytes. Überschreitung oder Kontextfehler müssen vor
HTTP 200 enden; eine große CSV wird dabei nicht vollständig in einem Vec
gespeichert.

Nach erfolgreichem ersten Durchlauf erhält der Handler exakte Länge, SHA256,
Zeilenzahl und Snapshot. Der zweite Durchlauf verwendet dieselbe Session und
dieselben Formatter. Seine Länge, SHA256 und Zeilenzahl müssen übereinstimmen.
Die Übertragung verwendet höchstens vier gepufferte 64-KiB-Blöcke, einen
zurückgehaltenen Endblock und einen gegebenenfalls gerade versandten Block.
Zeilenformatierung, Parser, SQLite und Kontexte verursachen zusätzliche Kosten;
aus dieser Puffergrenze folgt kein gemessener Gesamt-RAM-Verbrauch.

Der letzte positive Block bleibt zurückgehalten, bis Core seine Transaktion
beendet hat und der Coordinator den tatsächlichen Worker-Join beobachtet hat.
Ein später Fehler muss den Body fehlerhaft oder kürzer als Content-Length
beenden. Ein schon versandtes HTTP 200 wird nicht nachträglich zu JSON 500
umgedeutet; ein Fehler darf auch kein normales erfolgreiches EOF erzeugen.

## Abbruch, Fristen und Shutdown

Die gemeinsamen Reader prüfen einen geteilten `SitemapReadControl` zwischen
tatsächlichen Arbeitsschritten. Ein intern besessener SQLite-InterruptHandle
ergänzt die Rust-Checkpoints. Frühere Signaturen bleiben dort, wo benötigt,
als Default-Adapter beziehungsweise Testorakel erhalten.

Standardfristen im Kandidaten: 120 Sekunden Vorbereitung, 300 Sekunden ab
Ready und 30 Sekunden ohne Transportfortschritt. Erfolgreiche Queueübergabe
und tatsächliches Entnehmen eines Prefixblocks aktualisieren den Leerlaufstand;
die absolute Frist wird dadurch nicht verlängert. Diese Werte sind gewählte
Betriebsgrenzen, keine beobachteten Laufzeiten oder Echtzeitzusagen.

Body-Drop und Verbindungsabbruch signalisieren Cancellation. Der Coordinator
behält den JoinHandle, bis der blockierende Worker tatsächlich beendet ist.
Ein bloßes `abort` eines blockierenden Tasks gilt nicht als Abbruchnachweis.
Der Server schließt beim Shutdown die Exportannahme, cancelt und drain-joinet
registrierte Exporte. Die App-Joblesesperre wird nach Ready freigegeben.

Ausgewählte Wiederholungs-Crawls behalten beide vollständigen höheren
Parent-/Plan-/Präsenzprüfungen und bleiben teilweise materialisierend.
Konfigurationsparser, globale Policy-Prüfung und ausgewählte Parent-BLOBs
besitzen weiterhin grobe kooperative Abbruchpunkte. Harte maximale
Abbruchlatenz, Speicher und SQL-Sortkosten sind nicht belegt.

## Prüfung und offene Abnahme

Die begrenzten Quellreviews fanden einen Konfigurations-Borrow-Konflikt,
eine versehentlich nur für Tests verfügbare produktive JSON-Hilfe und einen
Leerlauf-Timer, der aktiven Verbrauch bereits gepufferter Daten nicht erfasste.
Diese Stellen wurden korrigiert und unabhängig im Quellcode nachgeprüft.
Zusätzlich sind jetzt ausschließlich für Tests benötigte private Default-Adapter
entsprechend markiert. Das ersetzt keinen Compiler- oder Clippy-Lauf.

Die geschriebenen Tests verwenden tatsächliche Crawler-/Store-Fixtures,
unveränderte Formatter als vollständiges Orakel, echte Axum-/HTTP-/TCP-Pfade,
WAL-Schreiber und beobachtbare Worker-/Transaktionsbarrieren. Sie prüfen unter
anderem den vollständigen Bestand, Filter, Integrität, veraltete Kontexte,
Rückstau, Disconnect, Fristen, aktive langsame Downloads, späte Fehler,
Endblockfreigabe und Shutdown. Noch keine dieser neuen Gruppen ist ausgeführt.

Offen bleiben die kombinierte Kompilierung, vollständige Tests und Clippy,
native API-/CLI-/MCP-/Browserabnahme, eine gültige physische neue CSV-Paarung
mit exakt 64 MiB und einem Byte darüber, gerichtete inverse Selected-Präsenz
und Selected-Kontextvarianten, echte große Quellen, RAM/Latenz, Crash/Restart,
Linux/Container und die sieben vollständigen Finding-Filter. Die frühere
physische CSV-Abnahme der installierten Anwendung gilt nur für deren Stand.

Maßgeblich bleiben die [Feature-To-do-Liste](FEATURE_TODO.md), der
[ursprüngliche Integrationsentwurf](parity/sitemap-csv-streaming-plan.md)
und der [Filteroberflächenplan](parity/sitemap-filter-surfaces-plan.md).
