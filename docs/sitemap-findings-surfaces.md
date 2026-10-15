# Gespeicherte Sitemap-Befunde über alle Zugänge

Arbeitsstand: **Quellcode in Integration, NOT_COMPILED / NOT_RUN / NOT_INSTALLED**.
Installiert ist weiterhin der zuvor geprüfte Windows-Release.
Diese Erweiterung erhält noch keine neue Feature-Abnahme.
Vollständige Gleichheit mit Screaming Frog bleibt **NOT_PROVEN**.

## Gemeinsamer Datenweg

Die sieben Filter verwenden den [Core-Befundkern](sitemap-findings-core.md).
Fünf bewerten ursprünglich angefragte gespeicherte Seiten; zwei bewerten
tatsächlich gespeicherte XML-Quellenversuche. Jede Bewertung erhält `match`,
`not_matched` oder `unknown` und konkrete Gründe. Ein unbekannter Messwert ist
kein negativer Befund. Verwaiste URLs beziehen sich auf den gespeicherten
Linkgraphen; sie beweisen keine vollständige Entdeckung der Website.

Eine versiegelte Core-Lesesitzung prüft den gespeicherten Bestand innerhalb
einer tatsächlichen SQLite-Transaktion. Seitenabfrage und vollständiger Export
können denselben geprüften Stand verwenden. Filter gelten für alle passenden
gespeicherten Kandidaten. Eine JSON-Seite enthält ein begrenztes Suchfenster;
der CSV-Export durchläuft die gesamte feste Auswahl.

Die Auswahl besteht aus Filter, Bewertung, optionaler exakter Originalidentität
und optionaler wörtlicher Suche. Suche und Identitäten behalten Leerzeichen,
Groß-/Kleinschreibung und Originaltext. Seiten- und Quellenidentitäten sind
getrennte Domänen. Seiten- und Exporttoken haben unterschiedliche Rollen und
binden Auswahl, Core-Kontext und den tatsächlichen App-Crawl samt Generation
und Konfigurationshash. Sie sind Kontextprüfsummen, keine Zugriffsberechtigung.

## Zugänge

| Zugang | Quellstand dieses Schritts | Noch erforderlicher Beleg |
| --- | --- | --- |
| Core | `session.page()` und wiederholbarer vollständiger Besuch innerhalb derselben Lesesitzung | Compiler und tatsächliche SQLite-/Snapshot-Tests |
| HTTP | GET/POST `.../sitemaps/findings` und `.../sitemaps/findings/export.csv` | Tatsächlicher Server, Abbruch, Neustart und Grenzfälle |
| Oberfläche | Eigene Ansicht „Sitemap-Befunde“, sieben Filter, Gründe, Quelleninspektor, Fortsetzung und vollständiger Download | Browser gegen Mock und gegen denselben nativen Kandidaten |
| CLI | Gemeinsamer Abfrage-/Exportadapter integriert und statisch geprüft | Tatsächliche Offline-Dateien, Fortsetzungen, Export und Fehlerpfade |
| MCP | Eigenes Werkzeug und begrenzte Antwort integriert und statisch geprüft | Offizielles SDK über beide Transporte, einschließlich Abbruch |

Die CLI-Anbindung lautet im neuen Quellstand:

```text
open-seo-spider sitemap-findings saved.sqlite --request selection.json --output page.json --csv findings.csv
```

Mindestens ein neues Ausgabeziel ist erforderlich. Bestehende Ziele werden
zurückgewiesen. Beide Ausgaben werden vor der Veröffentlichung vollständig
geschrieben und synchronisiert. Gewinnt ein anderer Prozess erst danach den
zweiten Dateinamen, bleibt die bereits vollständig veröffentlichte erste Datei
erhalten und der Befehl meldet ausdrücklich eine teilweise veröffentlichte
Paar-Ausgabe. Eine atomare Veröffentlichung beider Dateinamen wird nicht behauptet.

Das MCP-Werkzeug heißt `spider_sitemap_findings`; seine Argumente stehen flach
neben der Crawl-`id`. `include_export_descriptor` ergänzt auf Wunsch eine gebundene
POST-Exportbeschreibung. Die CSV selbst wird nicht in die MCP-Antwort kopiert.
Die gesamte Antwort einschließlich RPC-ID und Stdio-Zeilenende hat eine eigene
8-MiB-Grenze. Nur diese Befundabfrage erhält eine HTTP-Frist von 150 Sekunden;
bestehende Werkzeuge behalten ihre bisherige Frist.

Eine erste Abfrage ohne gespeicherte SQLite-Daten liefert ausdrücklich fehlende
Daten und keinen erfundenen Core-Nachweis. Eine Fortsetzung, deren erforderlicher
SQLite-Kontext inzwischen fehlt, wird als veraltet abgelehnt. Verfügbare Quellen
mit unbekannter Vorgeschichte können eine echte Kontextzeile und null ausgewählte
Befunde exportieren; daraus wird keine Zahl tatsächlicher Remote-Versuche abgeleitet.

## Größen, Konsistenz und Abbruch

POST akzeptiert bis zu 64 MiB Anfragebytes. Die gesamte JSON-Antwort einschließlich
App-Metadaten und Token ist auf 8 MiB begrenzt. Ein kleineres Seitenlimit kann
erforderlich sein, obwohl die Core-Seite allein noch in das Limit passt. Die
gesamte CSV einschließlich Kopf und Kontext ist auf 64 MiB begrenzt. Überschreitungen
werden ausdrücklich zurückgewiesen; Ergebnisse werden nicht still gekürzt.

Die CSV hat zehn versionierte, zitierte RFC-4180-Spalten. Genau eine Kontextzeile
enthält die tatsächliche Auswahl und den geprüften Kontext. Datenzeilen behalten
den vollständigen typisierten Befund. Die lesbare Originalidentität erhält den
bestehenden Schutz vor Tabellenformeln; die maschinenlesbare JSON-Identität bleibt
unverändert. JSON-Felder werden direkt durch die CSV-Zeichenmaskierung geschrieben.

Der Server zählt und hasht zuerst die komplette CSV und erzeugt sie anschließend
in derselben Lesesitzung erneut. Eine begrenzte Warteschlange transportiert den
Body. Der letzte positive Abschnitt bleibt bis zum tatsächlichen Core-Rücklauf
und Worker-Join zurückgehalten. Fehler dürfen keinen erfolgreichen vollständigen
Download erzeugen. JSON-Lesearbeiten behalten ebenfalls einen registrierten
Blocking-Worker; Abbruch und Zeitlimit signalisieren dessen tatsächliche Kontrolle.
Der Shutdown wartet auf diese Worker. Ununterbrechbare Betriebssystem-I/O kann
die Wartezeit verlängern; eine harte Abbruchlatenz ist damit nicht bewiesen.

## Offene Abnahmen

- Gemeinsamen eingefrorenen Workspace kompilieren, sämtliche Tests mit eigenem
  Renderer ausführen und Alltargets-Clippy sowie Formatprüfung bestehen.
- Die sieben Filter mit positiven, negativen und unbekannten echten gespeicherten
  Fällen über API, CLI, MCP und Browser prüfen; dünne Suchfenster dürfen auch ohne
  Treffer fortsetzbar sein.
- Vollständige Exporte jenseits der ersten 200/400 Kandidaten, unveränderte
  Originaldaten und null zusätzliche Website-Anfragen belegen.
- Tatsächliche 8-MiB-/64-MiB-Grenzen einschließlich Plus-eins, beschädigte Quellen,
  Schreibfehler, späte Fehler, Disconnect, Zeitlimit, Worker-Join und Shutdown prüfen.
- Native Browsernavigation, veraltete Antworten/Downloads, Tastatur und schmale
  Darstellung am selben geprüften Kandidaten verifizieren.
- Optimierten Release und datenbewahrende lokale Installation separat prüfen;
  GitHub Actions, Linux und Container bleiben eigene offene Gates.

Syntaxprüfungen und Quellreviews ersetzen diese Laufzeitbelege nicht. Die
Feature-To-do-Liste wird erst nach tatsächlicher Abnahme abgehakt.

## Eingefrorener Quellstand

297 Eingaben sind gemeinsam hashgebunden. Gegenüber dem vorherigen Core-Schritt
wurden 47 neue Rust-Testfunktionen geschrieben; ausgeführt wurden davon null.
Alle 341 bisherigen Pflichtnamen bleiben erhalten, zusammen sind es 388. Der
künftige vollständige Workspace muss mindestens 1.170 Tests bestehen. Diese
Zahlen sind Abnahmeerwartungen und keine Testergebnisse. Zusätzlich liegen
13 Browsergruppen mit gemockter API vor, ebenfalls ohne Ausführung.

Der Quellstand enthält getrennte Fixes für die sieben im begrenzten Quellreview
bestätigten Produkt-/Harness-Befunde. Die Fixes sind quellseitig nachgeprüft;
der Compiler und die Laufzeit müssen sie noch bestätigen. Die physischen
Grenzbelege des älteren installierten Releases gelten für dessen damalige
Rohdatenrouten und werden nicht auf diese neuen Befundrouten übertragen.

Die [Prüfvorbereitung](verification/sitemap-findings-surfaces-runners-preparation.json)
bindet die künftige vollständige Suite und den Build an genau diesen Quellstand.
Sie dokumentiert vorbereitete Aufrufe, keine ausgeführten Prüfungen.
