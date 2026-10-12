# DOM-Beobachtungen und Fragmentziele

Open SEO Spider speichert eigene strukturierte Beobachtungen aus dem reparierten
HTML5-DOM. Die Felder dokumentieren erfasste Deklarationen und die Herkunft ihres
Befunds. Ein ähnlich benannter Filter anderer Software belegt keine identische
Regel, und eine unvollständige Erfassung belegt keine Abwesenheit.

## Quelle, Reihenfolge und Grenzen

`PageRecord.dom_metadata` ist optional. `null` oder ein fehlendes Feld bedeutet
„nicht erfasst“, beispielsweise in älteren Projekten. Ein vorhandenes Objekt mit
einer leeren Sammlung bedeutet eine beobachtete leere Sammlung. Das Profil ist
`independent_html5_dom_v1`; `source` ist `raw_html` oder `rendered_html`. Nach
erfolgreichem Browserrendering bewahrt `raw_metadata.dom_metadata` den vorherigen
HTTP-Befund. Der effektive Befund bleibt in `page.dom_metadata`.

`complete` gilt ausschließlich für die Sammlung der DOM-Beobachtungen. Ein
vollständiger Prüfbefund setzt zusätzlich eine erfolgreiche HTTP-200-Antwort,
geeigneten Medientyp, vollständigen Body sowie fehlende Request- und Renderfehler
voraus. Beim gerenderten DOM muss dessen Dokumentantwort ebenfalls erfolgreich
sein. Der Medientyp wird vor dem Semikolon getrennt und ohne Beachtung der
Groß-/Kleinschreibung verglichen; der originale `content_type` bleibt erhalten.

Ein `DomLocation` enthält `index`, `path`, `path_truncated` und
`semantic_ancestors`. Index und Pfad beziehen sich auf den reparierten DOM-Baum,
einschließlich vom HTML5-Parser ergänzter Elemente. Sie sind keine Quellzeilen,
Bytepositionen, CSS-Layoutkoordinaten oder gemessene Sichtbarkeit. Die Sammlung
begrenzt DOM-Knoten und Beobachtungen, Baumtiefe, Pfadlänge, Kandidaten und
erfassten Text. Erreichte Grenzen stehen explizit in `limits_reached`; der aktuelle
Parser verwendet unter anderem 100000 Knoten, Tiefe 256 und 8192 Byte pro Pfad.
Inhalte von `template` sind inert und werden nicht als Dokumentziele behandelt.

## Beobachtungsfamilien

| Sammlung | Gespeicherte Fakten | Grenze der Aussage |
| --- | --- | --- |
| `headings` | H1–H6 in Baumreihenfolge, normalisierter und ursprünglicher Text, separate Bild-Alt-Attribute, optionaler `text_with_image_alt`, `aria_level`, `heading_offset`, `hidden_attribute`, DOM-Position | ARIA und headingoffset werden gespeichert, ohne eine visuelle Ebene zu berechnen. `hidden_attribute` ist keine vollständige CSS-Sichtbarkeitsprüfung. |
| `meta_keywords` | Jedes Keywords-Metaelement, optionaler originaler `content`, normalisierter Text und originale Unicode-Zeichenanzahl | Fehlendes Attribut unterscheidet sich von `content=""` mit Zeichenanzahl 0. Keine Aussage über Rankingwirkung. |
| `fragment_targets` | Originale `id`-Werte sowie Namen von HTML-Ankern, jeweils mit Zieltyp und DOM-Position | Ein Ziel kann mehrfach deklariert sein. PDF-, XML-, Medien- und Clientrouter-Semantik wird nicht aus diesen HTML-Fakten abgeleitet. |
| `navigation_candidates` | Nicht standardmäßige href-, Router-/Data- und Rollenbeobachtungen einschließlich Originalattributen, möglicher Adresse, Scheme, Begründung und `onclick` | Ein möglicher HTTP-Link ist kein ausgeführter Klick oder automatisch erzeugter Crawl-/Graphlink. Scripts werden für diese Sammlung nicht ausgeführt. |
| `responsive_images` | Attribute jedes img-Vorkommens sowie Kandidaten aus img src, img srcset und picture source mit Deskriptoren, Media, Typ, Größenhinweis, Parse-/Anwendbarkeitsfehlern und Position | `eligible` dokumentiert Deklarationssyntax und erfasste strukturelle Anwendbarkeit. Es belegt keine Medienabfrageauswertung oder tatsächliche Browserauswahl. w/x-Deskriptoren sind keine gemessenen Dateimaße. |

Navigation enthält die begründeten Klassen `anchor_without_href`,
`javascript_href`, `non_http_scheme`, `invalid_http_href`,
`href_on_non_anchor`, `custom_navigation_attribute` und
`role_link_without_anchor_href`. Ein beliebiges onclick-Element ohne diese
Merkmale gehört nicht automatisch zu der Sammlung. Reguläre Anker bleiben
`LinkRecord`-Vorkommen. Bild-, Relationship- und sonstige Ressourcenverweise
behalten ihre tatsächliche `kind`; sie werden nicht als Anker gezählt.

`headings[].text_with_image_alt` ist eine zusätzliche beobachtete Textvariante:
das Profil `heading_text_engine = independent_dom_text_with_inline_image_alt_v1`
fügt img-Alt-Text an dessen Position zwischen Textknoten ein und normalisiert
anschließend Leerraum. Ursprünglicher `raw_text`, bisheriger `text`, einzelne
`image_alt`-Werte sowie `page.h1`/`page.h2` bleiben getrennt erhalten. `null`
beziehungsweise ein fehlendes Feld bedeutet unbekannt; `""` bedeutet beobachtet
leer. Das Profil berechnet weder CSS-Sichtbarkeit noch eine universelle
Verhaltensgleichheit mit anderen Programmen.

`LinkRecord.raw_href` bewahrt das Originalattribut. `resolved_href` bewahrt die
aufgelöste Adresse inklusive Fragment, `fragment` das ursprüngliche Fragment.
`url` ist weiterhin die normalisierte Crawl-Identität ohne Fragment. Dadurch
bleiben mehrere Fragmentvorkommen sichtbar, ohne Netzwerkabrufe zu duplizieren.

## Geordnete Linkpositionen

`CrawlConfig.link_position_rules` ist standardmäßig eine leere Liste. Die erste
konfigurierte CSS-Regel, die den Link oder einen Vorfahren trifft, gewinnt.

```json
{
  "link_position_rules": [
    { "name": "Produktbereich", "selector": "article.product" },
    { "name": "Artikel", "selector": "article" }
  ]
}
```

Es sind maximal 32 Regeln erlaubt. Namen müssen nach dem Trimmen eindeutig,
nicht leer, frei von Steuerzeichen und höchstens 128 UTF-8-Byte lang sein;
Namensvergleich ist case-sensitive. Selektoren müssen gültige CSS-Selektoren mit
1 bis 8192 UTF-8-Byte sein. Browser und Server prüfen die Eingabe vor dem Crawl.
Ohne eigenen Treffer gelten nächster nav/header/footer/aside/head-Bereich oder
content für main/body. Diese Position ist eine DOM-Klassifikation, keine
Bildschirmposition. Unbekannte historische Positionen bleiben unbekannt.

## Optionale Fragmentprüfung

`CrawlConfig.validate_fragments` ist standardmäßig `false`. Aktivieren in den
Crawl-Einstellungen erzeugt Ergebnisse für erfasste Ankerfragmentvorkommen gegen
bereits erfasste Dokumente. Die Prüfung führt keine zusätzlichen Requests aus.
`GET /api/crawls/{id}/fragments` liefert `{observations, enabled, scope}`.
Ein leeres Inventar bei `enabled:false` bedeutet eine deaktivierte Prüfung.

| Status | Bedeutung |
| --- | --- |
| `matched_id` | Das erfasste HTML-Dokument enthält die passende ID. |
| `matched_name` | Kein passender ID-Treffer; ein passender HTML-Ankername wurde erfasst. |
| `top` | Leeres Fragment oder der HTML-Dokumentanfang über top ohne vorherigen Zieltreffer. |
| `missing` | Keine passende ID und kein Ankername in einem vollständig erfassten geeigneten HTML-200-Dokument. |
| `unknown` | Zum Beispiel unerfasstes, mehrdeutiges, fehlgeschlagenes, abgeschnittenes oder historisches Ziel; unbekannte Redirect-/JS-Navigation-Fragmentweitergabe. `reason` benennt die Ursache. |
| `unsupported` | Zum Beispiel andere MIME-Semantik, XHTML, Textdirektiven oder mögliche Clientrouter-Fragmente ohne tatsächlichen Zieltreffer. |

Die Suche prüft zunächst das Originalfragment, dann genau einmal
percent-dekodiertes UTF-8. Plus bleibt ein Plus; ungültige UTF-8-Sequenzen erhalten
Ersatzzeichen. IDs haben Vorrang vor Namen; bei Duplikaten gewinnt die erste
erfasste passende ID in Baumreihenfolge. Originale und dekodierte Werte, tatsächliche
Quell-/Zieldokumentadresse, Quellposition, Zielposition und `target_complete`
bleiben im Ergebnis sichtbar. Ein fehlgeschlagener Request ist `unknown` mit
seiner Begründung und erzeugt keinen vermeintlich fehlenden HTML-Anker.

## Oberfläche und Exporte

Die Tabellen Überschriftenfolge, Meta Keywords, Bildkandidaten und Weitere
Navigation zeigen einzelne gespeicherte Beobachtungen; ältere fehlende Sammlungen
erhalten ausdrücklich „Nicht erfasst“. Linkvorkommen zeigt tatsächliche Links und
ihre Positionen. Fragmentziele trennt gefunden, fehlt, unbekannt und nicht
unterstützt. Der DOM-Inspector bewahrt den effektiven und vorherigen Befund als
sicher dargestellte JSON-Vorschau; dessen 100-Einträge-/30000-Zeichenbegrenzung
ändert den gespeicherten Bericht nicht.

Der gefilterte sichtbare CSV-Export enthält die aktuelle Tabelle.
`dom.csv` enthält alle DOM-Familien samt Verfügbarkeit und Snapshot-Rolle,
`links.csv` alle tatsächlichen Linkvorkommen samt Herkunft und Position und
`fragments.csv` alle aktivierten Prüfergebnisse mit Begründung. Website- und
konfigurationsgesteuerte CSV-Textzellen werden mit einem führenden Apostroph gegen
Tabellenformeln geschützt, falls erforderlich. `project.json` bewahrt die exakten
gespeicherten Daten ohne diesen CSV-Präfix. Die eigenständigen Tag-Reihenfolge-
Hinweise sind keine nachgewiesene vollständige Verhaltensgleichheit mit einem
anderen SEO-Programm.
