# Speichergrenze für Sitemap-Mitgliedschaften

Stand: Der vollständige V4-Workspace besteht mit 1.216 Tests, Format und Clippy. Optimierter Build, native Größen-/Quellenabnahmen und Installation sind separat offen.

Die Crawl-Konfiguration kann die Kapazität des dauerhaft gespeicherten Sitemap-Katalogs ausdrücklich festlegen. In den Einstellungen heißt die Option „Eigene Speichergrenze für Sitemap-Mitgliedschaften“. Sie ist unabhängig von der Verarbeitung großer XML-Quellen.

```json
{
  "sitemap_membership_limits": {
    "max_catalog_json_bytes": 134217728,
    "max_row_json_bytes": 262144,
    "max_rows": 1000000
  }
}
```

Diese Option ist über die gespeicherte Crawl-Konfiguration, die vorhandene CLI-Konfigurationsdatei, die API und `spider_start_crawl.config` verfügbar. Eine ausgelassene Option oder `null` behält die bisherige Konfiguration ohne zusätzlich serialisiertes Feld. Ein leeres Objekt verwendet die Standardwerte.

| Feld | Standard | Zulässiger Bereich |
| --- | ---: | --- |
| `max_catalog_json_bytes` | 33.554.432 Bytes (32 MiB) | 1 bis 134.217.728 Bytes (128 MiB) |
| `max_row_json_bytes` | 262.144 Bytes (256 KiB) | 1 bis 1.048.576 Bytes (1 MiB), höchstens die Kataloggrenze |
| `max_rows` | 1.000.000 | 1 bis 1.000.000 |

Die Kataloggröße zählt serialisierte Katalogdaten. Dokumente, URL-Einträge und einzelne Loc-Vorkommen zählen als getrennte Zeilen. Das ist keine Begrenzung des gesamten Arbeitsspeichers. Eine größere Kataloggrenze erhöht weder XML-/HTTP-Verarbeitungsgrenzen noch API-/Exportgrenzen. Unbekannte Felder und ungültige Werte werden abgewiesen.

Die Einstellung gilt für einen neuen Katalog. Beim Fortsetzen eines gespeicherten Crawls bleiben dessen gespeicherte Grenzwerte erhalten; eine Kapazitätsänderung ist keine zulässige operative Resume-Anpassung. Ausgewählte Wiederholungen übernehmen die vollständige Elternkonfiguration und den dazugehörigen Katalog. Ein gespeichertes Objekt mit einer Kapazität oberhalb des bisherigen Standards muss eine exakt passende explizite Konfiguration enthalten.

Wenn das Speicherbudget nicht reicht, wird die Übernahme dieser Quelle vollständig zurückgerollt. Eine fehlgeschlagene Übernahme darf keine vollständige Mitgliedschaft vortäuschen. Ursprüngliche XML-Antworten werden dadurch nicht als vollständige Dateien dauerhaft gespeichert.

Der bestandene Test für 50.000 vollständige Einträge übernimmt echten Parseroutput in SQLite, liefert nach dem Öffnen denselben Katalog und enthält die letzte Seite. Das ersetzt weder die separate native Prüfung großer Quellen noch sämtliche zulässigen 50-MiB-Varianten. Die vollständige Herstellerparität bleibt offen.
