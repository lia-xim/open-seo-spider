# N-Gramme aus gespeicherten Crawls

Source10 ergänzt eine Auswertung von Wortfolgen über alle gespeicherten Seitenzeilen eines ausgewählten Crawl-Stands. Die Auswertung verwendet die damals gespeicherten Roh- oder DOM-Texte und startet keine Website-Abfragen. Der neue Stand wird erst nach seiner praktischen Abnahme installiert; die derzeit laufende ältere Installation enthält diese Ansicht noch nicht.

## Bedienung

Nach Installation von Source10 lässt sich die Ansicht über die N-Gramm-Verweise in der Oberfläche oder unter `/saved-ngrams` öffnen.

1. Einen gespeicherten Crawl und die Textquelle wählen. Der Crawl muss bereits mit Textspeicherung gelaufen sein. Fehlender DOM-Text wird als fehlende Quelle ausgewiesen.
2. Die gewünschte Wortlänge von eins bis sechs, Groß-/Kleinschreibung und gegebenenfalls Stopwörter einstellen. Ein Stopwort steht jeweils in einer eigenen Zeile. Wortfolgen werden über entfernte Stopwörter hinweg nicht neu verbunden.
3. **Gespeicherten Crawl auswerten** wählen. Die Gesamthäufigkeit zählt auch überlappende Vorkommen; die Anzahl verschiedener Quellseiten steht separat daneben.
4. **Alle Quell-URLs dieser Wortfolge** öffnet die vollständige Quellenzuordnung. **Nächste Seite** zeigt weitere Quellen. **Vollständige Auswahl CSV** exportiert die gesamte aktuelle Auswahl; **Diese Seite JSON** speichert die sichtbare Ergebnisseite.

Die Positionsangaben sind einbasige Tokenpositionen. Pro Wortfolge und Quelle werden höchstens 16 Beispiele ausgegeben. Bei weiteren Positionen erscheint ein Stichprobenflag; die Häufigkeit bleibt vollständig gezählt.

Eine leere Auswertung aus fehlenden oder unvollständigen Textquellen belegt keine tatsächliche Abwesenheit der Wörter. Die Quellenansicht zeigt, welche gespeicherten Seitenzeilen gezählt oder ausgeschlossen wurden.

## CLI

Die CLI liest eine ausdrücklich angegebene Projekt-JSON-Datei. Beispiel für `ngram-options.json`:

```json
{
  "source_kind": "raw_http_html",
  "options": { "max_n": 6 },
  "view": "totals",
  "n": 6,
  "limit": 200
}
```

```powershell
.\open-seo-spider.exe saved-ngrams .\project.json --options .\ngram-options.json --output .\ngrams-page.json --csv .\ngrams-full.csv
```

Beide Ausgabedateien müssen neu und verschieden sein. Die CLI verweigert das Überschreiben der Projektdatei, der Optionsdatei oder einer vorhandenen Ausgabe. Ohne Optionsdatei ist das bisherige Profil mit höchstens drei Wörtern voreingestellt; `max_n: 6` aktiviert zusätzlich Vier- bis Sechswortfolgen. Das bisherige Content-/Ähnlichkeitsprofil bleibt erhalten.

## Lokale API

`POST /api/crawls/{id}/ngrams/preview` nimmt dieselben Optionen entgegen. `view` kann `totals`, `occurrences` oder `sources` sein. `exact_tokens` wählt eine genaue Wortfolge; `query` ist ein enthaltener Textfilter. Eine Ergebnisseite umfasst höchstens 200 Zeilen.

Für die Fortsetzung sind `after` aus `next_after` und `expected_snapshot_token` aus `snapshot.snapshot_token` erforderlich. Optionen, Filter, Ansicht und Seitengröße müssen gleich bleiben. Eine bekannte Generation kann zusätzlich als `expected_generation` gebunden werden.

`POST /api/crawls/{id}/ngrams/csv` verlangt den Token der Vorschau und keine Fortsetzung. Der Export umfasst alle Zeilen der gebundenen Auswahl. Länge und SHA-256 stehen in den Antwortheadern; die Oberfläche prüft beide vor dem Download. Ein CLI-Aufruf hat einen eigenen Dateikontext: Ein API-Snapshot-Token lässt sich deshalb nicht als CLI-Token wiederverwenden.

## Grenzen

Das ist ein eigenes, dokumentiertes Tokenprofil für gespeicherten ausgewählten Text. Separate Titel-, Überschriften-, Linktext-, Bild-Alttext- und unverlinkte Textkanäle sowie eine Wortwolke stehen weiterhin im Backlog. Es gibt keinen Nachweis identischer Screaming-Frog-Algorithmen oder vollständiger Herstellerparität.

Die gespeicherte Projektdatei und CSV-Ausgabe sind jeweils auf 64 MiB begrenzt, JSON-Ausgaben auf 8 MiB. Weitere Dokument-, Text-, Token- und Aggregationsbudgets werden ausdrücklich geprüft; eine Überschreitung erzeugt einen Fehler statt still gekürzter Gesamtergebnisse. Die TX-lokale Sitemap-Projektion ist eine getrennte Änderung und belegt noch keinen gemessenen Geschwindigkeitsgewinn großer Crawls.

Die bisherigen Nachweise und verbleibenden Aufgaben stehen im [Feature-Backlog](FEATURE_TODO.md) und im [Source10-Testcheckpoint](verification/source10-directed-checkpoint-20261015/index.json).
