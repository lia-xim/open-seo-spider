# Sitemap-Quellen in den Crawl-Einstellungen

Stand: **Quellcode, NOT_COMPILED / NOT_RUN / NOT_INSTALLED**.
Diese Bedienerweiterung baut auf `f43efc1` auf. Die installierte Anwendung wurde
nicht verändert. Der vorherige eingefrorene Stand mit 297 Eingaben bleibt erhalten.

Unter „URL-Filter, Extraktionen & Prüfgrenzen“ gibt es einen eigenen Abschnitt
„Verarbeitung von XML-Sitemap-Quellen“. Seine Checkbox aktiviert ausdrücklich
ein Profil für den nächsten Crawl. Ohne Aktivierung enthält der Startauftrag
kein `sitemap_source_processing`; die bisherigen Backend-Defaults gelten weiter.

| Einstellung | Native Eigenschaft | Anfangswert bei Aktivierung | Erlaubter Bereich |
| --- | --- | --- | --- |
| Eingangsdaten nach HTTP-Dekomprimierung | `max_post_http_entity_bytes` | 8.388.608 Bytes | 1–67.108.864 Bytes |
| XML nach zusätzlicher Gzip-Dateidekomprimierung | `max_decoded_xml_bytes` | 8.388.608 Bytes | 1–67.108.864 Bytes |
| XML-Ereignisse einschließlich Dateiende | `max_xml_events` | 500.000 | 1–2.000.000 |

Alle drei Werte werden ohne Rundung als positive ganze Zahlen geprüft. Die
Bytegrenzen sind unabhängig voneinander; ein Gzip-Dokument darf sich innerhalb
des gewählten XML-Limits weiter entfalten. Die bestehende Antwortgröße für
Seiten und die gespeicherten Mitgliedschaftsbudgets bleiben getrennte Optionen.
Ein größeres Arbeitsbudget ist kein Beleg für vollständige Messung oder Speicherung.

Gespeicherte Einstellungen werden beim erneuten Öffnen wieder angezeigt.
Ein gespeichertes aktiviertes Profil mit ungültigen Werten verhindert den Start,
bis es korrigiert oder ausdrücklich deaktiviert wird. Werte werden nicht still
auf eine größere Grenze geklemmt. Bei deaktiviertem Profil sind die drei Felder
ausgeblendet und deaktiviert. Die vier reinen UI-Eigenschaften werden vor dem
API-Aufruf entfernt; das aktivierte native Profil enthält nur die drei Eigenschaften.

Die Quelländerung betrifft `web/app.js` und `web/index.html`. Rust-Code und
die bisherigen 388 erforderlichen Rust-Testnamen bleiben unverändert; für
diesen Bedienerschritt wurden keine weiteren Rust-Tests geschrieben.

Offene Abnahmen:

- Browserbedienung, gespeicherte Werte nach Neuladen und die genaue Startanfrage
  bei ausgeschaltetem und eingeschaltetem Profil prüfen.
- Ein aktiviertes Profil am tatsächlichen neuen Rust-Kandidaten mit einer
  realen größeren XML-Quelle prüfen; ein Formularwert allein beweist keinen Fetch.
- Fehlerdarstellung, Tastaturbedienung und vollständige Erreichbarkeit auf
  schmalen Bildschirmen prüfen.
- Diesen veränderten UI-Quellstand separat an die vollständigen Suite-, Build-
  und nativen Prüfrouten binden. Die Vorbereitung für den vorherigen Stand
  `69492d0c…` gilt nicht automatisch als Beleg für diese UI-Änderung.
- Optimierten Release und datenbewahrende Installation getrennt abnehmen.

Die Feature-Anzahl und sämtliche Hersteller-Paritätsabnahmen bleiben unverändert.
