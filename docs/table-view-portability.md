# Ansichten als Datei austauschen

Der Import und Export erweitert die benannten Tabellenansichten um lokale JSON-Dateien. Einfache Ansichten (v1) und Gruppenansichten (v2) behalten getrennte Dateiformate, Browserkataloge und Schreibsperren. Der [begrenzte native R2-Beleg](verification/table-view-portability-20261019/README.md) umfasst die neue Suite mit vier Prüfgruppen, 22 JSON-Dateien und 27 CSV-Dateien mit 2.626 Datenzeilen. R1 bleibt als fehlgeschlagener Lauf mit 16 Suiten und 15 anderen erfolgreichen Suiten erhalten; R2 prüft ausschließlich die neue Suite. Der exakte veröffentlichte Stand `66dfe5c` besteht inzwischen alle vier öffentlichen CI-Jobs und die 45 registrierten UI-Suiten in einem einzigen ungefilterten Aufruf ([terminaler Beleg](verification/table-view-portability-20261019/public-ci-66dfe5c-20261021/receipt.json)). Ein zusätzlicher vollständiger lokaler 45-Suiten-Lauf wurde nicht ausgeführt. Der Fortschritt steht in [FEATURE_TODO.md](FEATURE_TODO.md).

## Export

„Ausgewählte Ansicht exportieren“ schreibt eine vollständige Definition. „Gesamten Katalog exportieren (alle Bereiche)“ enthält sämtliche Definitionen des aktuellen Filtermodus, einschließlich anderer Tabellenbereiche. Beide verwenden den vollständig validierten, aktuell gelesenen Browserkatalog. Ein beschädigter Katalog wird nicht teilweise exportiert. Der Export verändert weder die Tabelle noch gespeicherte Definitionen.

Die Dateien heißen `open-seo-spider-table-views-v1.json` beziehungsweise `open-seo-spider-table-views-v2.json`; Einzelexporte ergänzen `-selected`. Sie enthalten den bestehenden vollständigen Katalogvertrag mit Engine, Version und Einträgen. UTF-8-Ausgaben enthalten keinen BOM und keine zusätzliche Abschlusszeile. IDs, Filter, Suchtexte, Spalten, Sortierung und vollständige UND/ODER-Ausdrücke bleiben erhalten. Es werden keine Crawl-Berichte oder Ergebniszeilen eingefügt.

## Import und Konflikte

Eine gewählte Datei wird vollständig gelesen und geprüft. Die Vorschau zeigt alle enthaltenen Bereiche und Definitionen. Der Import schreibt erst nach ausdrücklicher Bestätigung. Er führt dieselbe Definition nicht automatisch auf der aktuellen Tabelle aus; dafür bleibt „Anwenden“ zuständig.

- Neue Einträge können mit unveränderter Kennung hinzugefügt oder ausdrücklich übersprungen werden.
- Bereits identische Definitionen benötigen keine Änderung.
- Bei Konflikten entscheiden Sie zwischen Überspringen, Ersetzen und einer Kopie mit eigenem Namen und neuer Kennung.
- Ersetzen ist nur bei einem eindeutig zugeordneten Zieleintrag möglich. Dessen Kennung und Anzeigename bleiben erhalten.
- Wenn Kennung und Name auf unterschiedliche Einträge zeigen oder eine Kennung einem anderen Bereich gehört, sind Überspringen oder eine neue Kopie erforderlich.

Die Entscheidung wird gegen den gesamten Zielkatalog geprüft. Ein Fehler bewirkt keinen Teilimport. Eine Bestätigung ohne Änderungen erhält die bisherigen Bytes, einschließlich eines noch nicht angelegten Katalogs. Der Import einer anderen Dateiversion wechselt den Filtermodus nicht automatisch.

## Grenzen und Erhaltung

Es gelten die bestehenden Konfigurationsgrenzen: höchstens 32 Definitionen pro Filtermodus, 2 MiB für die vollständige Datei beziehungsweise den Katalog, 512 KiB pro Eintrag und 128 UTF-8-Bytes pro Namen. Ein einzelner anfänglicher UTF-8-BOM wird beim Import akzeptiert und im physischen Dateibudget mitgezählt. Die v2-Ausdrucksgrenzen bleiben 16 Regeln, 31 Knoten, vier Gruppenebenen und 128 KiB. Dies sind Grenzen der Ansichtsdefinitionen; die Anzahl crawlbarer URLs wird dadurch nicht begrenzt.

Modus-, Crawl-, Query-, Auswahl- oder Namenswechsel, Abbrechen und fremde Katalogänderungen verwerfen eine ausstehende Dateivorbereitung. Der Schreibadapter prüft vor dem atomaren Speichern unter dem jeweiligen Web Lock die exakt zuvor gelesenen Katalogbytes erneut. Fehlende oder belegte Locks, Speicherfehler und fremde Änderungen führen zu einer sichtbaren Verweigerung. Der Import setzt beschädigte oder nicht unterstützte vorhandene Daten nicht zurück.

Die aktuelle Abfrage, Ergebniszeilen, Pagination und gespeicherten Crawl-Berichte bleiben beim Import erhalten. Wird die gespeicherte Definition einer aktiven Ansicht ersetzt, muss ihr Status den Unterschied zur aktuellen Arbeitsansicht weiterhin zeigen. Einfache und gruppierte Kataloge sowie getrennte Spaltenpräferenzen bleiben unabhängig.

Native Projektpräferenzen, XLSX, beliebige Browser- und Lastszenarien sowie vollständige Herstellerparität sind durch diesen Teilvertrag nicht nachgewiesen.
