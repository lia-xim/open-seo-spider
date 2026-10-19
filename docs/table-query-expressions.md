# Verschachtelte Tabellenfilter

Der Gruppenmodus ergänzt die einfachen UND-Regeln um verschachtelte UND/ODER-Gruppen. Bereichsfilter und globale Suche gelten zusätzlich zum gesamten Ausdruck. Ausgeblendete Spalten bleiben durchsuchbar und filterbar. Fehlende Messwerte, leerer Text, null, false und numerische 0 behalten die vorhandene typisierte Bedeutung.

Die [begrenzte native R6-Abnahme](verification/table-query-readiness-20261019-r6/README.md) besteht in drei vorab festgelegten Durchläufen mit je45 Tabellenbeobachtungen und9 CSV-Dateien/927 Datenzeilen. Der ursprüngliche R4-Gesamtlauf bleibt FAIL mit43 erfolgreichen Suiten und einer fehlgeschlagenen Bereitschaftsprüfung im Driver. R6 wartet auf den tatsächlichen Owner-Datencommit; ein neuer vollständiger44-Suiten-Lauf ist separat erforderlich. Herstellerparität bleibt NOT_PROVEN. Fortschritt: `feature-parity.json` und `FEATURE_TODO.md`.

## Bedienung

Unter „Ansicht und Filter“ wechselt „Filtermodus“ zwischen einfachen Regeln und Gruppen. Der Wechsel in den Gruppenmodus übernimmt vorhandene Regeln verlustlos als UND-Ausdruck. Eine neue Regel wird aus dem vorhandenen Formular in die gewählte Gruppe eingefügt. Vorhandene Knoten können mit UND oder ODER umschlossen, innerhalb einer Gruppe verschoben oder entfernt werden. Leere Gruppen sind unzulässig; das Entfernen des letzten Kindes entfernt die Gruppe. Ein leerer Gesamtausdruck bedeutet keinen zusätzlichen Regelfilter.

Der ausdrückliche Wechsel zurück zu einfachen Regeln ist nur bei verlustlos darstellbarem UND möglich: verschachtelte UND-Gruppen und Gruppen mit einem Kind werden in ihrer ursprünglichen Reihenfolge aufgelöst. Ein ODER mit mehreren Kindern wird sichtbar zurückgewiesen. „Filter aufheben“ löscht den Ausdruck, danach ist der Wechsel möglich. Ein Owner- oder Schemawechsel setzt die Regeln und den Filtermodus zurück.

Sortierung, sichtbare Spalten, CSV und Pagination verwenden dieselbe vollständig validierte Abfrage. ODER gibt jede ursprüngliche Zeilenbeobachtung höchstens einmal aus. Zwei unterschiedliche Beobachtungen derselben URL bleiben zwei Zeilen; die Ausgabe wird nicht nach URL zusammengeführt.

## Ausdrucksvertrag

```json
{"kind":"any","children":[{"kind":"rule","rule":{"key":"count","op":"min","min":2}},{"kind":"all","children":[{"kind":"rule","rule":{"key":"title","op":"empty"}},{"kind":"rule","rule":{"key":"url","op":"contains","text":"/help/"}}]}]}
```

`null` ist der ungefilterte Ausdruck. Blätter sind `{kind:"rule",rule:…}`; Gruppen sind `{kind:"all"|"any",children:[…]}` mit mindestens einem Kind. Der vollständige Baum wird vor jeder Zeilenauswertung oder Übernahme geprüft. Grenzen: 16 Regeln, 31 Knoten, vier Gruppenebenen und 128 KiB UTF-8-Ausdruck. Bisherige Grenzen einzelner Regeln bleiben erhalten. Diese Grenzen betreffen Abfragekonfiguration, nicht die Anzahl crawlbarer URLs. Bestehende einfache Abfragen erhalten keine zusätzlichen Baumgrenzen.

## Gespeicherte Ansichten

Einfache Ansichten behalten ihren unveränderten v1-Katalog `open-seo-spider.table-views.v1`. Gruppenansichten werden ausdrücklich im getrennten v2-Katalog `open-seo-spider.table-views.v2` gespeichert, mit Engine `open-seo-spider/table-views-v2` und Abfragesemantik `open-seo-spider/table-query-v2`. Es gibt keine automatische Migration oder Speicherung beim Wechsel des Modus. Jede Liste erlaubt 32 Einträge und besitzt ihren eigenen exklusiven Web Lock sowie eine Prüfung der zuvor gelesenen Bytes.

Eine Gruppenansicht enthält exakt `sectionFilter`, `search`, `expression`, `sort` und `visibleKeys`. Sie enthält keine erfassten Ergebniszeilen, Crawl-ID oder Zugangsdaten. Auswahl und Speichern ändern keine aktuelle Tabelle. Erst „Anwenden“ übernimmt die vollständig geprüfte Definition atomar für den aktuellen Crawl. Modus-, Query-, Owner- oder Schemawechsel verwerfen ausstehende Aktionen. Die versteckte Liste des anderen Modus nimmt keine UI-Aktionen an.

Löschen und Zurücksetzen benötigen eine getrennte Bestätigung. Der Reset der Gruppenliste betrifft ausschließlich den v2-Katalog; einfache Ansichten, gespeicherte Crawls und getrennte Spaltenpräferenzen bleiben erhalten. Bei fremden Änderungen ist ausdrückliches Neuladen erforderlich. Ohne Web Locks bleiben Lesen und Anwenden möglich; Schreibaktionen werden sichtbar verweigert. Portable Definitionen, native Projektpräferenzen und XLSX sind weiterhin offen.
