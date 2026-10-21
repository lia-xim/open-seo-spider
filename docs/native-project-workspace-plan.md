# Offener Vertrag für portable native Projektansichten

Status: **geplant, nicht implementiert**. Browserlokale Spalten und benannte
Ansichten sind keine portable native Projekteinstellung. Dieser Punkt bleibt
Teil des gesamten Feature-Backlogs.

Ein vollständiger Ausbau benötigt einen projektlokalen Workspace-Sidecar und
ein versioniertes natives Projektpaket. Der ursprüngliche Crawl-Bericht muss
als exakte Bytes mit SHA-256 enthalten sein, damit Parent-, Generation- und
Snapshot-Bindungen erhalten bleiben. Das bisherige Projekt-JSON bleibt ein
Berichtsaustauschformat. Ein Paket darf kein SQLite-Resume versprechen.

Der Workspace enthält Spaltenpräferenzen und projektlokale v1-/v2-Ansichten
mit den vorhandenen Eintragsfeldern `id`, `name`, `section`, `query_semantics`,
`schema`, `section_filters`, `view`. V1 erhält Regeln; V2 den Ausdrucksbaum.
Browserglobale Ansichtskataloge werden durch einen Import nicht überschrieben.
Ein ausdrücklicher Import zeigt Vorschau sowie Überspringen-, Ersetzen- und
Kopieren-Konfliktentscheidungen.

Native Lese-/Schreibzugriffe benötigen projektbezogene Sperren, Revision-UUID
und Raw-SHA-CAS. Fehlende, beschädigte und leere Daten bleiben verschieden.
Ein Reset erzeugt eine neue Revision. Paketimport erstellt eine neue
Crawl-UUID und veröffentlicht Bericht plus Workspace atomar über einen
Directory-Commit. Startup ignoriert unvollständige Staging-Verzeichnisse.
Import-Fehler oder Disconnect dürfen keine halben Projekte registrieren.

Der ausgewählte Recrawl benötigt einen Adapter für den exakten gespeicherten
Berichtspfad; ein Paketbericht darf durch erneute Serialisierung oder
`extend_report` nicht unbeabsichtigt zur CurrentProjection werden.

Die Abnahme muss echte Browserbedienung, v1/v2-Filter und Spalten, Konflikte,
CAS, Quota-/Dateifehler, Disconnect, EOF, kalten Neustart und einen Portwechsel
umfassen. Dabei sind ursprüngliche Berichtsbytes und globale Browserkataloge
vorher/nachher unabhängig zu vergleichen. Eine UI-Vorschau allein erfüllt
diesen Vertrag nicht.
