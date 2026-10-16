# Gespeicherte Crawls per CLI fortsetzen

Der Befehl `resume` gehört zum Source16-Kandidaten. Den aktuellen Installations- und Prüfstand hält [FEATURE_TODO.md](FEATURE_TODO.md) fest.

Die lokale Open-SEO-Spider-App muss laufen. Verwende die UUID eines gespeicherten, fortsetzbaren Crawls aus der Crawl-Liste:

```powershell
.\open-seo-spider.exe resume $crawlId
```

`$crawlId` steht hier für die vorhandene Crawl-UUID. Die CLI sendet den Auftrag an `http://127.0.0.1:8787`. Die laufende App übernimmt die gespeicherte Warteschlange und die Quotenbuchhaltung.

Bei einem anderen lokalen Port:

```powershell
.\open-seo-spider.exe resume $crawlId --server http://127.0.0.1:8788
```

`--server` akzeptiert eine HTTP-Adresse mit explizitem Port und `127.0.0.1` oder `[::1]`. Verwende hier die lokale App-Adresse.

Eine erfolgreiche Antwort sieht so aus:

```json
{"id":"UUID-des-Crawls","status":"running"}
```

Exitcode 0 bestätigt die Annahme der Wiederaufnahme. Der Crawl läuft anschließend in der App weiter; seinen Abschluss und die Ergebnisse siehst du dort. Bereits vollständig abgeschlossene Crawls und unzulässige Wiederaufnahmen werden abgelehnt.

## Optionen ändern

Mit `--options` liest die CLI eine UTF-8-JSON-Datei mit höchstens 65.536 Bytes:

```powershell
.\open-seo-spider.exe resume $crawlId --options .\resume-options.json
```

Beispiel für deren Inhalt:

```json
{
  "max_pages": 10000,
  "concurrency": 4,
  "delay_ms": 100
}
```

Unterstützte Felder sind `max_pages`, `max_attempts`, `concurrency`, `delay_ms`, `request_timeout_secs` und `quota_increases`. Ausgelassene Einstellungen behalten ihre gespeicherten Werte. `max_pages` gilt für reguläre Crawls, `max_attempts` für den gezielten Recrawl ausgewählter URLs. Eine ausdrückliche 0 entfernt das jeweilige Operatorlimit. Dauerhafte Quoten gelten zusätzlich weiter.

Quotenerhöhungen verwenden die aktuelle gespeicherte Generation und Profilrevision. Veraltete Angaben, Verringerungen und wirkungslose Erhöhungen werden abgelehnt. Für den gezielten Recrawl werden Quotenerhöhungen derzeit abgelehnt. Unbekannte oder doppelte JSON-Felder werden ebenfalls abgelehnt. Die App prüft den Auftrag, bevor sie den gespeicherten Crawl verändert.

Bei einem Fehler gibt die CLI eine Fehlermeldung auf stderr und einen Fehlercode zurück. Sie folgt keinen HTTP-Weiterleitungen und wiederholt den Auftrag nicht automatisch. Bei einem Verbindungsabbruch solltest du zunächst den Crawl-Status in der App prüfen, da der Auftrag dort bereits angekommen sein kann.
