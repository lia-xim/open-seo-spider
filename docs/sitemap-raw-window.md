# Verifizierte rohe Sitemap-Zeilen seitenweise lesen

Status: **Entwicklungskandidat; Kompilierung, Tests und native Abnahme offen.**
Zweig: `codex/sitemap-streaming-20261014`, aufbauend auf dem separaten
Quellenprofil-Kandidaten `380beb1`. Der installierte Release erhält dadurch
keine neue Funktion. Dieses Dokument beschreibt den neuen Quellcodevertrag.

36 zusätzliche Regressionen sind geschrieben: 15 Raw-Core-, 9 Source-Core-,
8 Store-/Zustands- und 4 CLI-Gruppen. Davon sind **0 ausgeführt**. Der
eingefrorene Stand bindet 246 Eingabedateien und 211 erforderliche Testnamen;
für den künftigen Gesamtlauf werden mindestens 993 bestandene Tests verlangt.
Die Zahlen sind Erwartungen. Die Workspace-Formatprüfung besteht; begrenzte
Quellreviews haben zwei Fehler gefunden und deren Quellkorrekturen geprüft.

Der Rust-Zugriff `CrawlStore::sitemap_raw_window` und der CLI-Befehl
`sitemap-rows` lesen originale Deklarationen, Dokumente, Einträge und
Loc-Vorkommen aus einer bestehenden SQLite-Datei. Der Leser prüft den gesamten
rohen Datenbestand und seine Quellmessungen in derselben Lesetransaktion,
bevor er eine begrenzte Seite zurückgibt. Ein beschädigter Datensatz außerhalb
der angeforderten Seite darf deshalb keinen erfolgreichen Präfix erzeugen.

## CLI-Vertrag nach Build und Abnahme

```powershell
open-seo-spider sitemap-rows crawl.sqlite --request page-request.json --output page.json
```

Ohne `--request` wird eine erste Seite mit höchstens 100 Zeilen gelesen.
Ohne `--output` wird dasselbe kompakte JSON auf stdout geschrieben. Eine
Ausgabedatei muss neu sein. Der Zugriff öffnet SQLite schreibgeschützt und
startet weder einen Crawl noch eine Schemamigration.

Beispiel für `page-request.json`:

```json
{
  "kind": "location",
  "membership_url": "https://example.org/page",
  "limit": 100
}
```

`kind` erlaubt `all`, `root`, `document`, `entry` und `location`.
`source_observation_id` ist optional und muss eine exakte kleingeschriebene SHA-256-ID
sein. `membership_url` ist eine exakte kanonische ursprüngliche HTTP(S)-URL;
Alias- und Redirect-Ziele werden dafür nicht eingesetzt. Der Filter wird vor
der Auswahl der Seite angewendet. `limit` liegt zwischen 1 und 1000; ungültige
Werte werden abgewiesen. Die vollständige kompakte JSON-Antwort bleibt auf
8 MiB begrenzt. Die CLI liest tatsächlich höchstens 128 KiB Requestdaten und
ein zusätzliches Byte zur Ablehnung, bevor JSON ausgewertet oder SQLite
geöffnet wird.

Die Antwort enthält `context`, `state_sha256`, `rows`,
`next_after_sequence` und `has_more`. Bei `has_more: true` übernimmt die nächste
Anfrage den zurückgegebenen `next_after_sequence` als `after_sequence` sowie
`state_sha256` als `expected_state_sha256`. Die Filter bleiben identisch.
Eine andere Seitengröße ist erlaubt. Der vorherige Ordinal muss tatsächlich
eine Zeile desselben Filters sein. Ein geänderter Zustand oder Filter verlangt
einen Neustart der Seitennavigation.

## Aussage und Grenzen

Der neue Zustands-Hash besitzt das eigene Profil
`sitemap-verified-raw-window-v1`. Er bindet den Filter, die tatsächlichen
gespeicherten Konfigurationsbytes, die Generation, den vollständigen rohen
Ledger-Zustand und den verifizierten Quellmessungszustand. Auch eine neue
Quellmessung ohne bereits zugeordnete Mitgliedschaft ändert diesen Zustand.
Er ist kein Authentifizierungstoken und übernimmt keine Bedeutung bestehender
Browser-Cursor.

Fehlende historische Rohdaten bleiben über `context.ledger: null`
unbekannt. Ein vorhandener Ledger mit null Zeilen belegt dagegen den bekannten
leeren Rohbestand. Messungen ohne Commit dürfen keine gespeicherten
Mitgliedschaften erfinden. Der Quellmessungs-Kontext behält sein bestehendes
Hashprofil.

Der normale neue Roh-/Messungsleser arbeitet mit Zeilendurchläufen und einem
begrenzten Seitenpuffer. Konfigurationen und Importe haben weiterhin eigene
begrenzte Materialisierungskosten. Der neue Leser prüft vor den bestehenden
Kontextvalidierungen außerdem die Metadatenlängen: höchstens 64 KiB für
`run_state`/`operation_kind` und 256 MiB für `run_plan`. Das ist eine eigene
Eingabegrenze dieses neuen Lesers; ein kleiner Parent begrenzt nicht automatisch
eine veränderte Plan-Metadatenzeile. Bei ausgewählten Wiederholungs-Crawls
bleiben die vorhandenen vollständigen Parent-/Plan- und Präsenzprüfungen
aktiv; `materializing_selected_compatibility: true` kennzeichnet diese Kosten.
Dieser Pfad ist noch kein vollständig streamender ausgewählter Kontext.
Die bestehenden Prüfhelfer für originale Deklarationskoordinaten können
Konfigurationsdaten weiterhin je Root erneut auswerten. Die praktische
Latenzabnahme muss auch diese Kosten erfassen.

Die bestehenden Browser-/API-/MCP-Inventare, deren Coverage und gespeicherte
Seiten-/Inlink-Analysen nutzen weiterhin ihren bisherigen Leser. Der neue
Rohleser ersetzt diese Aussagen nicht. Änderungen allein an noch offenen
Discovery-Aufgaben oder gespeicherten Berichtsanalysen sind daher keine
zusätzlich zugesicherte Frischebedingung dieses Rohzustands. Vollständige
CSV-Streams und die sieben Filter über alle gespeicherten Kandidaten bleiben
gesonderte Aufgaben. Der bisherige CSV-Formatter wurde für gemeinsame
Ausgabesenken herausgezogen; der aktuelle HTTP-Export bleibt gepuffert.

## Offene Abnahmen

- Neue Validatoren gegen die bestehenden vollständigen Validatoren prüfen:
  kanonische Bytes, Hashes, SQL-Spalten, Reihenfolgen, Querverweise und exakte
  Eintrag-/Loc-Zuordnung einschließlich doppelter deklarierter Ordinale.
- Echte Store-, CLI-, schreibgeschützte Legacy-, Zustandswechsel-, Parent- und
  Beschädigungsfälle ausführen. Geschriebene Tests sind noch kein Testbeleg.
- Tatsächliche Speicher- und Laufzeitmessungen einschließlich SQLite-Sorts,
  großer Konfigurationen und ausgewählter Elternkontexte erheben.
- Den vollständigen Kontextleser und reale Browser-/API-/MCP-Pfade umstellen,
  anschließend CSV mit begrenzter Rückstaukontrolle, Abbruch und Producer-Ende
  verbinden und abnehmen.
- Große Quellen, physische Grenzen, Recovery und Plattformen aus dem
  [Sitemap-Tiefenplan](parity/sitemap-filter-depth-plan.md) weiter abarbeiten.

S4 bleibt offen. Die [Feature-To-do-Liste](FEATURE_TODO.md) und der
[Filter-Flächenplan](parity/sitemap-filter-surfaces-plan.md) verfolgen die
übrigen Anforderungen; vollständige Herstellerparität ist nicht belegt.
