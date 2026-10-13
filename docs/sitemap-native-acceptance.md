# Native Sitemap-Prüfungen

Die versionierten Skripte verwenden eigene App-Prozesse, dynamische Ports, temporäre Daten und einen eigenen Browser. Port 8787 und 8790 sind ausgeschlossen. `ledger` prüft elf Szenariogruppen und elf tatsächliche SQLite-Jobs; `membership` prüft elf API-/CSV-/SDK-/Browsergruppen. Wiederholungen werden nicht als zusätzliche Funktionsabdeckung gezählt.

```powershell
$env:SPIDER_BINARY = 'E:/eigener-build/open-seo-spider.exe'
$env:SPIDER_SOURCE_COMMIT = (git rev-parse HEAD).Trim()
$env:SPIDER_EXPECTED_SOURCE_COMMIT = $env:SPIDER_SOURCE_COMMIT
$env:SPIDER_SITEMAP_NATIVE_OUTPUT = 'E:/eigene-pruefungen/neuer-ledger-lauf'
node scripts/run-sitemap-native-checks.mjs ledger

$env:SPIDER_SITEMAP_NATIVE_OUTPUT = 'E:/eigene-pruefungen/neuer-membership-lauf'
$env:OPEN_SEO_BROWSER_PATH = 'C:/Program Files/Google/Chrome/Application/chrome.exe'
node scripts/run-sitemap-native-checks.mjs membership
```

Ausgabeordner müssen neu sein. `RUNNER_TEMP`, `TEMP` und `TMP` bestimmen zusätzlich den eigenen temporären Speicher. Der Membership-Lauf benötigt die festgelegten Abhängigkeiten aus `scripts/package-lock.json` und `renderer/package-lock.json`. Linux-CI übergibt den Pfad ihres zuvor installierten Playwright-Chromiums ausdrücklich.

Der Runner prüft saubere getrackte Cargo-/Fixture-/Asset-Dateien, hält ihre Bytes und SHA-256 fest und bindet den ausführbaren Testserver vor und nach dem Lauf. Ein optionales `SPIDER_BINARY_SHA256` verlangt einen zusätzlich vorgegebenen Binary-Hash. Ein Runtime-Hash allein beweist keine Herkunft: ein unabhängiger Compiler-Beleg oder der exakte CI-Build muss die Verbindung zwischen Quellstand und Binary bestätigen.

Ledger-Rohbelege enthalten Requests, API-Antworten, App-Logs, SQLite-Zeilen und konkrete Fixture-Rezepte. Membership erhält zusätzlich originale API-/Browserantworten, CSV-Download, MCP-Beobachtungen und Screenshot. Tatsächliche Prozessschließungen und vollständig geleerte Request-Fixtures sind Pflicht. SQLite-Startup-Arbeitsdateien werden vor dem strikten Dateibyte-Vergleich getrennt geprüft.

Die lokale Abnahme gilt für den unveränderten 197-Dateien-Quellstand von `ae77438` und den separat gebauten Windows-Dev-Binary. Die CI-Verdrahtung ist eine neue Anforderung; Windows-/Linux-CI, optimierter Release und Installation werden dadurch nicht als bestanden ausgegeben. Vollständige rekursive Quellen-Kohorten, Byte-Budget-Fehler, Power-Loss und Herstellerparität bleiben eigene offene Gates.
