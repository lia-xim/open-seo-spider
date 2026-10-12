# Ressourcen-Protokoll: API, CLI und MCP

Stand B9: Die begrenzte native Integration, API/CLI, beide MCP-Transporte und tatsächliche UI sind [lokal geprüft](verification/resource-discovery-batch.json). Exakte Commit-CI, unveränderlicher Release und Installation werden separat nachgewiesen. Der installierte B8-Release erhält dadurch keine nachträgliche B9-Zusage. Die vollständige Typ-/Navigation-/Encoding-Matrix bleibt offen. [Feature-To-do](FEATURE_TODO.md), [Quellvertrag](resource-discovery.md), [UI-Vertrag](resource-discovery-ui.md).

## Native Quellvorkommen

`GET /api/crawls/{id}/resources?limit=200&after_sequence=123`

Das Ergebnis enthält `enabled`, `engine`, `config_sha256`, `crawl_config_sha256`, `crawl_generation`, `snapshot`, `ledger_available`, `checkpoint`, `observations`, `next_after_sequence`, `has_more`, `snapshot_scope`, `website_requests:0` und `entity_bodies_included:false`. `limit` liegt zwischen1 und1000. Der optionale nichtnegative Cursor darf die SQLite-Integergrenze nicht überschreiten. Unbekannte Queryfelder werden abgelehnt.

Ein Vorkommen bewahrt Reihenfolge, Generation, current/inherited-Basis, Auswertungsrevision sowie Originalwert, dekodierten Wert, Auflösung, Quelle, DOM-/Byteposition, Klassifizierung, deklarierte MIME-Hinweise und Fehler. `source_unit_revision` und `source_complete` unterscheiden unabhängige HTML-/CSS-Quelleinheiten. Eine Entscheidung enthält ihren Grund und gegebenenfalls einen unveränderlichen Grant. Zusammengeführte Zielabrufe verändern frühere Grants nicht.

SQLite-Seiten lesen Zähler und Zeilen innerhalb einer gemeinsamen Lese-Transaktion und prüfen die gespeicherten Zeilen-/Transportbindungen. `snapshot.crawl_config_sha256` bindet die vollständige typisierte gespeicherte Crawl-Konfiguration; `snapshot.generation` bindet die gespeicherte Generation aus derselben Transaktion. Die API vergleicht beides sowie Ressourcenconfig/Engine mit dem zugehörigen Job. Ein ausgetauschtes SQLite mit bloß gleicher Ressourcenconfig genügt nicht. Der nächste API-Aufruf kann einen inzwischen neueren Protokollstand sehen; Pagination allein ist daher kein vollständiger unveränderlicher Export. Diese gespeicherten Bindungen sind keine extern signierte Herkunftsbescheinigung. Eine künftige Optimierung großer wiederholter Leseprüfungen muss dieselben Integritätsgrenzen erhalten.

Ist die optionale Ressourcen-Erkennung deaktiviert oder fehlen native SQLite-Zeilen, ist `ledger_available:false` mit einem ausdrücklichen Grund. Ein im importierten Bericht erklärter Checkpoint kann als historische Information bestehen bleiben; daraus entstehen keine erfundenen nativen Zeilen oder erfolgreiche Vollständigkeitsbelege.

## Beobachtete Antwort-Metadaten

`GET /api/crawls/{id}/resources/transports?limit=200&after_url=<exakt URL-kodierter Cursor>`

Der erste Aufruf lässt `after_url` weg. Der optionale Cursor ist nicht leer und höchstens65536 UTF-8-Bytes lang. Die Antwort enthält `transports`, `next_after_url` und `has_more` sowie dieselben Protokollfelder. Jede Beobachtung benennt exakte requested URL, ursprüngliche beobachtete Generation, `retained_response` und eine begrenzte `projection` der extrahierten Metadaten. Die äußere `retained_response`-Angabe beschreibt die aktuelle Aufnahme in die Ergebniszeilen und kann durch einen späteren unabhängigen Speichern-Grant atomar wechseln. Die innere `projection.resource_response.retained_response:false` beschreibt weiterhin die ursprüngliche reduzierte private Metadatenprojektion. Fetched-Zeit/Generation, fehlender Volltext und fehlender Body werden dabei nicht nachträglich als neuer HTTP-Beobachtung behauptet. Vollständige HTML-/CSS-Entity-Bodies werden damit nicht ausgeliefert.

`store:false` schließt die Antwortzeile aus dem Bericht aus. Der tatsächlich abgeschlossene URL-Abruf und notwendige Metadaten für unabhängige spätere Grants bleiben im begrenzten Transportprotokoll. Diese Projektion enthält keinen gespeicherten Volltext, beliebige HTML-Extraktion, PDF-Volltext, Screenshot oder Browser-Netzwerkprotokoll. Nicht vorhandene Quelltexte bleiben ausdrücklich unavailable; spätere Materialisierung macht keine neue Netzwerkanfrage und erfindet keine vollständige ursprüngliche Textaufnahme.

`completed_transports` zählt die im vorliegenden Ressourcen-Protokoll bearbeiteten URL-/Frontier-Einheiten einschließlich durch Robots blockierter oder fehlgeschlagener Einheiten. Es zählt weder ausschließlich erfolgreiche beobachtete Antworten noch jeden Redirect-Hop oder Robots-/Sitemap-Request. Status, `blocked_by_robots` und Entity-Fakten zeigen, ob tatsächlich eine Antwort bzw. ein Body beobachtet wurde. `retained_responses` zählt gespeicherte Berichtzeilen; ausgewählte Recrawls können dabei geerbte Elternzeilen enthalten. Das bestehende `stats.fetched` behält diese Berichtzeilen-Semantik. Fehlende Elternhistorie bleibt über `inherited_history.rows_available:false` sichtbar.

## Vollständiger Protokollexport

`GET /api/crawls/{id}/resources.json` und `GET /api/crawls/{id}/resources.csv`

Diese Downloads lesen alle verfügbaren Entscheidungen aus genau einer validierten SQLite-Lese-Transaktion. Sie streamen in begrenzten Seiten und HTTP-Chunks; ein geschlossener Download bricht die Ausgabe ab. Snapshot-Revision, Zeilenzahl, Config-Hash, Protokoll-Hash und Discovery-Complete stehen auch in HTTP-Headern. Bei fehlendem nativen Protokoll wird kein leerer vollständiger Ersatzexport erzeugt.

JSON enthält die vollständige Config-/Generations-Snapshotbindung, alle strukturierten Vorkommen und `export.all_snapshot_rows_exported:true` erst nach erfolgreicher Ausgabe sämtlicher Snapshot-Zeilen. CSV enthält wesentliche Spalten und zusätzlich die vollständige JSON-Observation pro Zeile, damit null/leer, Attribute und Grants erhalten bleiben. Textspalten verwenden denselben Schutz vor Tabellenformeln wie bestehende Exporte; numerische/boolean Spalten und das exakte Original im JSON bleiben erhalten. Ein vollständiger Zeilenexport beweist weder abgeschlossene Entdeckung noch verfügbare geerbte Historie. `checkpoint.complete` und `inherited_history` müssen separat bewertet werden.

Die CLI nutzt dieselben gespeicherten Metadaten und denselben Snapshot-Writer:

```text
open-seo-spider resource-ledger crawl.sqlite --limit 200
open-seo-spider resource-ledger crawl.sqlite --after-sequence 123 --limit 200
open-seo-spider resource-ledger crawl.sqlite --transports --limit 200
open-seo-spider resource-ledger crawl.sqlite --transports --after-url "https://example.test/a?x=1&y=2"
open-seo-spider resource-ledger crawl.sqlite --json resources.json --csv resources.csv
```

Exportziele müssen neu sein und sich vom SQLite-Eingang sowie voneinander unterscheiden. Vollständige Occurrence-Exporte lassen sich nicht mit einem Cursor oder Transportmodus kombinieren.

## Geliefertes CSS offline untersuchen

`POST /api/css/inspect` nimmt `{ "source": CssSourceInput, "limits": CssInventoryLimits? }` entgegen. CLI: `open-seo-spider inspect-css input.json [--output new-output.json]`.

`source` enthält `css_text`, `base_url`, `source_url`, `source_revision`, `inline` und optional `syntax:stylesheet|declaration_list`. Das Ergebnis benennt Parserprofil, gelieferten Text-/Limits-Hash, ursprüngliche Byte-Spannen, dekodierte URL-Kandidaten, Kontext, Diagnosen und Vollständigkeit unter diesem Profil. Es macht keine Website-Anfrage und bestätigt weder Herkunft noch tatsächliches Browserladen, Bildauswahl oder Cascade-Anwendung.

MCP ergänzt drei lesende Werkzeuge: `spider_resources`, `spider_resource_transports`, `spider_inspect_css`. Beide vorhandenen Transporte verwenden dieselbe lokale API. Cursor werden typgeprüft und als URL-Querywerte kodiert; die Werkzeuge starten keine zusätzlichen Website-Abrufe. Der Quellstand enthält insgesamt22 Werkzeuge; der installierte B8-Stand enthält weiterhin19, bis ein geprüfter B9-Release separat installiert wird.
