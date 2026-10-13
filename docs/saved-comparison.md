# Gespeicherte Crawls vergleichen

B11 ergänzt einen eigenen, versionierten Vergleich gespeicherter Beobachtungen. Er führt keine Website-Abfrage aus. Der bisherige einfache `GET /api/compare` bleibt verfügbar. Die aktuellen Abnahmen stehen in [FEATURE_TODO.md](FEATURE_TODO.md) und im [Backend-Nachweis](verification/saved-comparison-backend-checkpoint.json). Hersteller-Parität bleibt `NOT_PROVEN`.

## Schnittstellen und Identitäten

`POST /api/compare/saved` erhält `before` und `after` als gespeicherte Job-UUIDs. Optional sind `config`, `expected_before_generation` und `expected_after_generation`. Unbekannte Felder und ungültige verschachtelte Typen werden abgewiesen. Erfolg liefert `{comparison, website_requests: 0}`; Fehler erhalten Code, Nachricht, alle Kollisionsvorkommen und bei Konflikten die tatsächliche Job-Generation und den Status.

Der Engine-Vertrag heißt `open-seo-spider/saved-comparison-v1`. Exakte ursprüngliche angefragte URLs bilden die Join-Schlüssel. Query-Reihenfolge und Prozentkodierung bleiben erhalten. Ein ausdrücklich konfiguriertes Mapping enthält `side`, `from_origin`, `from_path_prefix`, `to_origin` und `to_path_prefix`. Es verändert ausschließlich den Join-Schlüssel. Ursprüngliche URLs, finale Ziele, Linkvorkommen und JSON-LD-Werte werden nicht umgeschrieben. Überschneidungen und doppelte Identitäten werden vor dem Join abgewiesen.

Der Core vergleicht konkrete Vorher/Nachher-Werte mit Zustand, Quellenprofil, Gründen und Herkunft. Dazu gehören Titel, Description, Überschriften, Direktiven, Linkvorkommen, JSON-LD und gespeicherte Textquellen. Fehlende alte Felder beweisen keine leere Beobachtung; gleiche unvollständige Textpräfixe beweisen keine Inhaltsgleichheit. Roh-HTTP und gerendertes DOM bleiben eigene Quellen. Ausgewählte Texte sind mit `include_selected_text: true` ausdrücklich opt-in.

## Snapshot-Bindung

Beide vollständigen typisierten Reports werden vor der Feldprojektion gehasht (`typed-crawl-report-serde-json-v1`). Dieser Hash ist keine Aussage über Originaldateibytes oder SQL-Authentizität. Die API klont beide stabilen Reports samt App-Deklarationen unter Lesesperren; laufende, pausierende oder abbrechende Jobs werden mit `409` abgewiesen. Unbekannte Import-Generation `0` wird bei erwarteter Generation geprüft, aber im Core als unbekannte Herkunft weitergegeben. Die App erfindet keine native SQL-Attestierung.

Ausgewählte Recrawls behalten die tatsächliche Herkunft alter und neuer Zeilen. Pending-Zeilen liefern keine frischen Texte aus dem Elternbericht. Zeitstempel und Laufzeitmetadaten verändern nicht automatisch die Inhaltsklasse. Die vier Klassen beschreiben ausschließlich die gespeicherten verglichenen Felder; sie beweisen keine vollständige Website-Gleichheit.

## CLI, MCP und Exporte

```text
open-seo-spider compare-saved before.json after.json --config compare.json --output comparison.json --csv comparison.csv
```

Ohne `--output` schreibt die CLI dasselbe JSON-Format nach stdout. Dateien und Konfiguration werden begrenzt eingelesen. Ausgabedateien müssen neu sein; vorhandene Ziele bleiben unverändert. CSV erhält Feldwerte, Zustände, Profile, Herkunft und beide typisierten Snapshot-Hashes mit Formula-Schutz und korrektem Quoting. Die zwei Ausgabedateien bilden keine gemeinsame Dateisystem-Transaktion: Ein IO-Fehler kann neue teilweise geschriebene Dateien hinterlassen; vorhandene Dateien werden nicht überschrieben.

Das zusätzliche, lesende MCP-Werkzeug `spider_compare_saved` bietet denselben gespeicherten Vergleich über stdio und Streamable HTTP. Die bisherigen 22 Werkzeuge bleiben erhalten; der aktuelle Quellstand enthält insgesamt 23. Strukturierte Fehler bleiben in `structuredContent` nachvollziehbar.

Vergleichs-JSON enthält Snapshotbeschreibungen und Feldprojektionen. Ein gemeinsamer Download mit beiden vollständigen unveränderten Originalberichten ist noch offen. Volles HTML/DOM, semantische Ähnlichkeit und automatisch geplante Vergleiche sind separate Anforderungen.

## Abnahmegrenzen

Die unabhängige Prüfung umfasst 34 gerichtete Core-Fälle, 62 App-Tests und insgesamt 440 Standard-Workspace-Tests. Vier explizite Chromium-Live-Tests bleiben in diesem Standardlauf ignoriert und gehören zur separaten Browser-CI. Neun tatsächliche [native/API/CLI/SDK-Gruppen](verification/saved-comparison-e2e.json) prüfen alle vier Klassen gemeinsam, konkrete Feldänderungen trotz gleicher Issue-Zahl, Quelle fehlend/partiell, Mapping, strukturierten Fehler und Kaltstart. Ein eigener HTTP-Witness am abgeschalteten Website-Endpunkt zählt dabei null Anfragen. Die [bestehende SDK-Prüfung](verification/saved-comparison-mcp-sdk.json) besteht mit 23 Werkzeugen über beide Transporte.

Die [tatsächliche Browserprüfung](verification/saved-comparison-ui-native.json) besteht acht Gruppen nach App-Kaltstart: vier Klassen, konkrete Werte, JSON-/CSV-Downloads, fehlende und partielle Quellen, strukturierte Fehler und 320px-Erreichbarkeit. Alle fünf ausgelieferten UI-Dateien stimmen bytegenau mit den geprüften Quellen überein. Der vollständige App-Schreibzugriffsschutz erfasst auch MCP; eine separat protokollierte Gegenprobe wird vor Zustellung blockiert. Browser und App enden bei aktivem HTTP-Witness; erst nach dessen vollständigem Abschluss wird der Zähler null übernommen. Dazu bestehen 16 UI-Vertrags-, 23 Browser-Fixture- und vier Mutation-Fence-Prüfungen.

Zehn [native Mappinggruppen](verification/saved-comparison-mapping-e2e.json) prüfen vier tatsächliche Crawls auf zwei verschiedenen Origins. Staging-/Produktions-Präfixe und Reverse-Mapping verändern nur Join-Schlüssel; finale Redirect-URLs, Linkvorkommen, JSON-LD, Text und Herkunft bleiben erhalten. Kollisionen vor und nach dem Mapping erhalten beide Seiten und alle ursprünglichen Vorkommen. CLI-CSV stimmt in jeder Spalte jeder Feldzeile mit der API überein. Nach Kaltstart bleiben alle Report-/Revisions-/Exportbytes gleich; zwei abgeschlossene Offline-Witnesses zählen null. 24 [Gegenproben](verification/saved-comparison-mapping-oracle-controls.json) erkennen unter anderem vertauschte CSV-Spalten, einen veränderten Programmstand und verspätete Anfragen. Der [Gesamtbeleg](verification/saved-comparison-checkpoint.json) bindet die getrennten Abnahmen.

Vollständiges Originalbericht-Bundle und exakte Source-CI/Release/Installation bleiben getrennte offene Gates. Der installierte Nutzer-Release auf Port 8787 bleibt B10, bis B11 diese Gates durchlaufen hat.
