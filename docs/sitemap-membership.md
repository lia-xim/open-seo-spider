# Gespeicherte Sitemap-Mitgliedschaft

Die Ansicht „Sitemaps & Mitgliedschaft“ zeigt ursprüngliche Deklarationen, tatsächlich beobachtete Quelldokumente, XML-Einträge und Loc-Vorkommen. Ursprüngliche Mitgliedschaft und zugelassene Anfrage-URL stehen getrennt: Eine Policy darf eine Abrufadresse ändern, ohne die ursprüngliche Sitemap-Aussage zu ersetzen. Duplikate, ungültige Werte und fehlende Locs bleiben erhalten. Der HTTP-Status einer Sitemap-Quelle ist kein belegter Status ihrer gelisteten Seiten.

Die Ansicht liest einen vollständig validierten SQLite-Snapshot. Sie startet keine Crawls und wählt keine Sitemap-Vorkommen für einen Recrawl aus. Altbestände ohne erhaltene Herkunft bleiben unbekannt (`rows: null`). Ein vorhandener Katalog kann für eine Abfrage tatsächlich leer sein (`rows: []`). Geerbte Fakten werden als Elternhistorie gekennzeichnet. Fehlende Treffer belegen keine vollständige Website-Abdeckung; die rekursive Quellen-Kohorte ist weiterhin offen.

## Seiten und Export

Die sichtbare Zeilenauswahl bietet 200, 100, 50, 10 oder 1 Zeile. Vor/Zurück verwendet gebundene Cursor. Ein Wechsel des Projekts, der Generation oder der Seitengröße verwirft ältere laufende Antworten. Suche, Sortierung und Ansichtsfilter betreffen nur die geladene Seite. Bei einer zu großen JSON-Seite fordert die Ansicht eine kleinere Zeilenzahl an; sie kürzt Ergebnisse nicht heimlich.

„Geladene Seite“ exportiert nur die angezeigten Datensätze. Der vollständige native CSV-Export liest den gesamten passenden gespeicherten Katalog unter demselben Snapshot und Serverfilter. Spreadsheet-Formeln werden neutralisiert; der zusätzliche typisierte JSON-Wert hält ursprüngliche Daten einschließlich Integer-Lexemen fest.

## Lesende Schnittstellen

`GET /api/crawls/{id}/sitemaps` akzeptiert `kind` (`all`, `root`, `document`, `entry`, `location`), `source_observation_id`, die exakte kanonische `membership_url`, `limit` (1 bis 200, Standard 100) und einen optionalen Fortsetzungscursor. Der Mitgliedschaftsfilter vergleicht ursprüngliche Identitäten, keine Abrufaliase. Filter werden vor der Seitenauswahl auf den vollständigen validierten Katalog angewendet.

`GET /api/crawls/{id}/sitemaps/export.csv` benötigt den `snapshot_token` als `snapshot` und dieselben Serverfilter. Cursor und Exporttoken sind unterschiedliche Rollen. Der neue MCP-Lesebefehl heißt `spider_sitemaps`; Argumente werden vor API-Zugriffen geprüft. Das neue Inventar enthält 24 Tools. Der bisher installierte Runtime mit 23 Tools bleibt separat gebunden.

Tokens binden Job, Generation, tatsächliche gespeicherte Config-Bytes, ursprüngliche Katalog-Config, Revision, Rolling-Digest, Summary/Vererbung und Filter. Sie dienen der Snapshot-Identität; ihre ungeschlüsselte Prüfsumme ist keine Authentifizierung. Malformed-Anfragen werden mit 400, veraltete oder konfliktbehaftete Snapshots mit 409 abgelehnt. Eine vollständige JSON-Seite über 8 MiB oder CSV über 64 MiB scheitert mit 413 ohne Teilkörper.

## Prüfstand

Der [lokale Quellbeleg](verification/sitemap-membership-api-ui-local.json) hält den tatsächlichen vollständigen Cargo-Workspace fest: 736 bestanden, kein Fehler, vier separat ausgesparte Renderer-/Browser-Tests. Core510, App112 und der unveränderte externe Policy-Test bestehen. Zwölf neue App-Kontrollen und zehn Browser/API-Fixture-Gruppen sind enthalten; wiederholte Teilmengen werden nicht addiert.

SQLite kann beim ersten Read-only-Öffnen WAL-/Shared-Memory-Arbeitsdateien anlegen. Der Test prüft diese Startup-Phase ausdrücklich: originale Datenbankbytes unverändert, nur eigene 32768-Byte-SHM und leere WAL zulässig, null Website-Anfragen. Danach bleiben sämtliche Dateibytes während der gemessenen Handler-/CSV-Lesezugriffe unverändert; der Website-Zähler wird nach vollständigem Fixture-Drain geprüft.

Tatsächliche native Router-/SDK-/Browser- und Ledger-Recovery-Abnahmen, echte Größenkontrollen, vollständige Kohorten und große Datenmengen, Windows-/Linux-Quell-CI, optimierter Release und Installation bleiben eigene offene Gates. GitHub meldete beim vorherigen neuen Lauf eine Konto-/Billing-Sperre vor dem Jobstart. Vollständige Screaming-Frog-Parität bleibt `NOT_PROVEN`.
