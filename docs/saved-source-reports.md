# Gespeicherte Linkquellen und Verzeichnisstruktur

Diese Erweiterungen sind im privaten Entwicklungsstand integriert. Der aktuelle Prüf- und Installationsstand steht in [FEATURE_TODO.md](FEATURE_TODO.md). Ein erfolgreicher Compilerlauf ersetzt noch keine vollständige Funktionsabnahme; die bisher installierte Version kann diese Ansichten noch nicht enthalten.

## Fehlerhafte Seitenlinks

Die Ansicht `/broken-sources` zeigt, auf welchen gespeicherten Seiten ein Link zu einem fehlerhaften oder unbekannten Ziel vorkommt. Nach Auswahl eines gespeicherten Crawls lassen sich beobachtete HTTP-Fehler, unbekannte Ziele oder alle gespeicherten Linkvorkommen anzeigen.

Wiederholte Links bleiben einzelne Fundstellen. Gespeicherter Linktext, Attribute, DOM-Pfad und Position helfen bei der Korrektur. Fehlende Angaben werden als fehlend gezeigt. Eine gespeicherte Weiterleitung ist vom ursprünglichen angefragten Ziel zu unterscheiden.

„Unbekannt“ bedeutet beispielsweise: Das Ziel wurde nicht abgerufen, ist gesperrt, besitzt mehrere widersprüchliche Beobachtungen oder stammt aus einem älteren übernommenen Stand. Daraus wird kein erfundener 404-Befund. Das Öffnen, Filtern und Exportieren startet keinen neuen Crawl.

Der CSV-Export enthält die vollständige gefilterte Auswahl desselben gespeicherten Stands. Er ist unabhängig von der gerade sichtbaren Seite. Formeln in Tabellenzellen werden abgesichert; die vollständigen ursprünglichen Werte bleiben in den strukturierten Feldern erhalten.

## Ressourcen- und CSS-Fundstellen

Die eigenständige Ansicht `/resource-broken-sources` verbindet gespeicherte Ressourcenentscheidungen mit den tatsächlich gespeicherten HTTP-Antworten. Dafür müssen die nativen Ressourcenregeln beim ursprünglichen Crawl aktiviert gewesen sein. Ein alter Crawl ohne diese Historie wird als nicht verfügbar angezeigt.

Die Fundstellen können aus HTML und gespeicherten CSS-Quellen stammen. Angezeigt werden unter anderem Quell-URL, Originalreferenz, aufgelöstes Ziel, erlaubte Abruf-URL, tatsächliches Endziel und verfügbare Positionsangaben. Anchor-Texte werden im Ressourcen-Ledger nicht gespeichert und hier nicht nachträglich ergänzt.

Mehrere Entscheidungen derselben Fundstelle sind keine zusätzlichen physischen Links. Die Ansicht nennt beide Zahlen getrennt. Seitenlinks und Ressourcenentscheidungen besitzen unterschiedliche Grundlagen; ihre Zeilen sollten nicht blind addiert werden.

„Diese Seite JSON“ speichert nur das sichtbare Ergebnisfenster. „Vollständige Auswahl CSV“ exportiert alle passenden Entscheidungen im geprüften Stand. Ändert sich die gespeicherte Grundlage, muss die Vorschau neu geöffnet werden.

Die explizite Offline-CLI liest eine SQLite-Datei und schreibt neue Ausgabedateien:

```powershell
.\open-seo-spider.exe resource-broken-sources "C:\Crawls\crawl.sqlite" `
  --output "C:\Exports\resources-page.json" `
  --csv "C:\Exports\resources-all.csv"
```

Mit `--request "C:\Exports\selection.json"` kann beispielsweise diese Auswahl übergeben werden:

```json
{"filter":"failures","limit":200}
```

JSON enthält ein begrenztes Fenster; CSV enthält die vollständige gefilterte Auswahl. Vorhandene Ausgabedateien, Eingabedateien und SQLite-Sidecars dürfen nicht überschrieben werden. Die beiden neuen Ausgabedateien bilden keine gemeinsame Dateisystemtransaktion: Ein später Schreibfehler kann eine unvollständige neu angelegte Datei hinterlassen.

## Verzeichnisstruktur

Unter `/directory-tree` lassen sich gespeicherte URLs nach Protokoll, Host, Port und Pfad erkunden. Untergruppen, übergeordnete Gruppen und weitere Kinder sind navigierbar; der CSV-Export enthält den vollständigen Baum.

Eine Pfadgruppe wie `/shop/` beweist nicht, dass diese Adresse abgerufen wurde. Gruppierungen und tatsächlich gespeicherte URL-Beobachtungen haben ausdrücklich verschiedene Rollen. Angezeigte Statuszahlen beziehen sich auf die angefragte URL; eine Weiterleitung übernimmt nicht einfach den 200-Status ihrer Zielseite. Nicht sicher festgestellte Indexierbarkeit bleibt unbekannt.

Diese Ansicht beschreibt URL-Pfade. Linkbeziehungen und tatsächliche Crawltiefe sind eigene Auswertungen.

## Grenzen

Die Auswertungen beschreiben den konkret gespeicherten Stand. Sie bestätigen weder die aktuelle Website noch vollständige Screaming-Frog-Parität. Ergebnisfenster sind auf höchstens 200 Zeilen, öffentliche JSON-Ausgaben auf 8 MiB und vollständige CSV-Ausgaben auf 64 MiB begrenzt. Überschreitungen sollen einen Fehler liefern, keinen still abgeschnittenen Erfolg. Diese technischen Grenzen sind keine kostenpflichtige URL-Lizenzgrenze.
