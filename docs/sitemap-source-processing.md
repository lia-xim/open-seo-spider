# Konfigurierbare Verarbeitung großer Sitemap-Quellen

Status: **Quellcode in Entwicklung; Kompilierung und Laufzeitabnahme offen.**
Dieses Profil gehört zum Arbeitszweig `codex/sitemap-large-source-20261014`.
Der installierte Release wird dadurch nicht aktualisiert. Die Beispiele gelten
erst für einen gebauten und abgenommenen Kandidaten dieses Zweigs.

## Konfiguration

`CrawlConfig.sitemap_source_processing` ist optional. Ohne dieses Feld oder mit
`null` gilt das bisherige Verhalten: Das HTTP-Limit für Sitemap-Quellen ist das
kleinere von `max_body_bytes` und 8 MiB. Der Standard für Seiten bleibt 5 MiB;
der Standard für dekodiertes Sitemap-XML bleibt 8 MiB.

Ein ausdrücklich gesetztes Profil legt die Arbeitsgrenzen für Sitemap-Quellen
fest. Es vergrößert weder die Antwortkörper gewöhnlicher Seiten noch die
Grenzen für gespeicherte Mitgliedschaften.

```json
{
  "seed_urls": ["https://example.org/"],
  "sitemap_urls": ["https://example.org/sitemap.xml"],
  "max_body_bytes": 5242880,
  "sitemap_source_processing": {
    "max_post_http_entity_bytes": 67108864,
    "max_decoded_xml_bytes": 67108864,
    "max_xml_events": 2000000
  }
}
```

| Einstellung | Standard im ausdrücklich gesetzten Profil | Zulässiger Bereich |
| --- | --- | --- |
| `max_post_http_entity_bytes` | 8.388.608 | 1 bis 67.108.864 Bytes |
| `max_decoded_xml_bytes` | 8.388.608 | 1 bis 67.108.864 Bytes |
| `max_xml_events` | 500.000 | 1 bis 2.000.000 Ereignisse |

Ein leeres Profil `{}` ist ein ausdrücklicher Wechsel auf diese Profilstandards.
Ungültige Werte werden abgewiesen; sie werden nicht auf einen erlaubten Wert
gekürzt. Unbekannte Profilfelder werden ebenfalls abgewiesen.

Die Konfiguration kann nach Abnahme über die vorhandene CLI-Konfigurationsdatei,
den Crawl-Create-API-Körper oder das `config`-Objekt von `spider_start_crawl`
übergeben werden. Eigene Formularelemente für dieses Profil im Browser sind
noch offen. Die bisherigen erweiterten Formularfelder sind kein allgemeiner
Editor für beliebige `CrawlConfig`-Felder.

## Bedeutung der Grenzen

`max_post_http_entity_bytes` begrenzt die tatsächlich nach HTTP-Dekodierung
behaltene Quelle. Diese Größe ist keine Messung komprimierter Netzwerkbytes.
Bei HTTP-Gzip liefert der HTTP-Client bereits dekodierte Bytes. Bei einer
Gzip-Datei folgt anschließend die getrennte Dateidekodierung.

`max_decoded_xml_bytes` begrenzt das Ergebnis dieser Dateidekodierung. Messungen
erhalten tatsächliche ausgegebene Bytes, Hashumfang, Vollständigkeit und Fehler
getrennt. Ein abgeschnittener Abruf erhält keine erfundene XML-Größe. Ein
Datei-Gzip-Abbruch kann einen tatsächlich ausgegebenen zusätzlichen Sentinel-Byte
belegen, ohne damit vollständige Integrität zu belegen.

Das 64-MiB-Profil lässt die echte Verarbeitung von 52.428.801 XML-Bytes zu.
Ein Profil, das bereits bei 52.428.800 Bytes stoppt, kann eine größere Plain-
oder HTTP-Gzip-Quelle nicht allein anhand empfangener Bytes als vollständig
dekodiertes XML einstufen. Die vorhandene Messprojektion unterscheidet deshalb
weiterhin exakt bekannte Werte, beobachtete Untergrenzen und unbekannte Werte.

Die bisherigen Standardgrenzen für Tiefe, einzelne Felder, behaltene Einträge,
Loc-Vorkommen und behaltenen Text bleiben bestehen. Das gilt auch für Zeilen-
und Katalogbudgets sowie API- und Exportgrenzen. Die unabhängige Zählung kann
mehr Quellvorkommen beobachten, als als Mitgliedschaften behalten werden.
Eine Messung ist daher kein Nachweis vollständiger gespeicherter Mitgliedschaft.

## Noch erforderliche Abnahmen

- Tatsächlichen Abruf, Dekodierung, Zählung und dauerhafte Zuordnung mit
  demselben validierten Profil über alle Pfade prüfen.
- Große Plain-, Datei-Gzip-, HTTP-Gzip- und Index-Quellen einschließlich der
  physischen Grenzwerte, CRC-/Truncation- und Ressourcenstopps ausführen.
- API-/CLI-/MCP-Konfiguration und gespeicherte Wiederaufnahme nativ prüfen.
- Praktische SQL-Paginierung und Exporte ohne vollständiges Katalogladen auf
  jeder Seite entwickeln und abnehmen. Ein größeres Eingabelimit erledigt
  diese offene S4-Anforderung nicht.
- Öffentliche Quellenbefunde und die sieben Filter über alle gespeicherten
  Kandidaten anbinden und abnehmen; eine Suche in den ersten 200 Zeilen reicht
  dafür nicht. Der [konkrete Flächenplan](parity/sitemap-filter-surfaces-plan.md)
  hält die tatsächlichen Lücken, den vorgeschlagenen gemeinsamen Abfragekern
  und die einzelnen Abnahmen fest; er ist noch keine Implementierung.

Der [Filter-Tiefenplan](parity/sitemap-filter-depth-plan.md) und die
[Feature-To-do-Liste](FEATURE_TODO.md) behalten diese Gates offen. Dieses
Dokument belegt weder bestandene neue Tests noch vollständige Herstellerparität.
