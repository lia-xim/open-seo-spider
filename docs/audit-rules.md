# Auditregeln und Messgrenzen

Die Audits prüfen gespeicherte Crawl-Fakten. Die zusätzlichen Onpage-Regeln sind
unabhängige, konservative Prüfhilfen mit Schweregrad `notice`; sie sind keine
Entscheidungen einer Suchmaschine. Namen wurden gegen die erfasste Referenzliste
mit 416 Filtern geprüft. Eine Namensähnlichkeit oder ein beobachteter Defaultwert
belegt noch keine gleiche Filtermitgliedschaft oder vollständige Funktionsparität.

## Konfigurierte Grenzen

`CrawlConfig.audit_thresholds` verwendet folgende Defaults. Ein ausgelassenes
Feld erhält den Default; ein ausdrücklich gesetztes JSON-`null` deaktiviert
genau diese Grenze. `0` ist erlaubt. Werte müssen zwischen 0 und 1000000 liegen;
wenn Minimum und Maximum gesetzt sind, darf das Minimum das Maximum nicht
überschreiten. Grenzen werden strikt mit `< Minimum` beziehungsweise `> Maximum`
geprüft; ein Wert genau auf der Grenze löst diesen Hinweis nicht aus.

| Feld | Default | Beobachtung |
| --- | ---: | --- |
| `min_title_chars` / `max_title_chars` | 30 / 60 | Primärer nicht leerer Titel |
| `min_description_chars` / `max_description_chars` | 70 / 155 | Primäre nicht leere Meta Description |
| `max_h1_chars` / `max_h2_chars` | 70 / 70 | Jedes erfasste H1-/H2-Element |
| `max_alt_chars` | 100 | Vorhandener Alt-Text |
| `max_url_chars` | 115 | Gesamte normalisierte Anfrage-URL |
| `min_word_count` | 200 | Erfasster, normalisierter Text-Wortzähler |
| `max_internal_outlinks` / `max_external_outlinks` | 1000 / 10 | Erfasste Anchor-Vorkommen je Scope |
| `min_title_pixels` / `max_title_pixels` | 200 / 561 | Primärer bekannter Titel-Messwert |
| `min_description_pixels` / `max_description_pixels` | 400 / 985 | Primärer bekannter Description-Messwert |

```json
{
  "audit_thresholds": {
    "max_description_chars": 160,
    "min_word_count": null,
    "max_external_outlinks": 25,
    "min_title_pixels": null
  }
}
```

Dieses Beispiel bewahrt eine ausdrücklich gewählte Description-Grenze von 160;
es ersetzt keine übrigen Defaults. Zeichenlängen zählen Unicode-Skalarwerte,
keine UTF-8-Bytes oder Graphemcluster. Titel, Description und Headings werden
vor ihrer Längenprüfung getrimmt; Alt-Text wird so gezählt, wie er erfasst wurde.
Leere Titel/Descriptions werden von den bestehenden Missing-Regeln behandelt
und nicht zusätzlich als zu kurz gemeldet.

## Vollständige Beobachtung

Die neuen Body-, Heading-, Image- und Linkregeln prüfen nur interne HTML-Seiten
mit erfolgreichem vollständigem Capture: HTTP-Endstatus 200, kein Requestfehler,
keine Robots-Sperre, kein abgeschnittener Body, kein Rendererfehler und kein
bekannter Nicht-200-Browserstatus. Ein fehlender Endstatus kann bei älteren
Onpage-Datensätzen durch einen bekannten Anfrage-Status 200 ersetzt werden;
die historischen Basisaudits benötigen weiterhin einen erfassten Endstatus.
Ein fehlender Browserstatus wird nicht als beobachteter 200 ausgegeben.

Bei unvollständigen Antworten werden fehlende Titel, Headings, Alt-Attribute,
Links oder Sprachbeziehungen nicht aus einem leeren Teilbefund abgeleitet.
URL-Formmerkmale bleiben beobachtbare Fakten der Anfrage-URL und können auch
ohne vollständigen Body geprüft werden. HTTP-, Robots-, Body- und Rendererfehler
bleiben eigene bestehende Meldungen.

## Onpage-Hinweise

| Bereich | Issue-Codes und Bedeutung |
| --- | --- |
| Titel / Description | `title_short`, `title_long`, `description_short`, `description_long`: primäre Zeichenlänge außerhalb der konfigurierten Grenzen. |
| Pixel | `title_pixels_short`, `title_pixels_long`, `description_pixels_short`, `description_pixels_long`: bekannter primärer Messwert außerhalb der Pixelgrenzen; Meldung enthält das tatsächliche Messprofil. |
| Headings | `h1_long`, `h2_long`, `h2_missing`, `h2_multiple`; bestehende `h1_missing`/`h1_multiple` bleiben erhalten. Mehrere H2 können eine normale Gliederung darstellen. |
| Heading-Duplikate | `h1_duplicate`, `h2_duplicate`: gleicher, whitespace-normalisierter und Unicode-kleingeschriebener Text auf mehreren vollständigen indexierbaren URLs. Wiederholungen nur innerhalb derselben URL erzeugen kein Seitenduplikat; gemeinsame Abschnittsnamen können sinnvoll sein. |
| Bilder | `image_alt_empty` unterscheidet ein vorhandenes leeres Attribut von bestehendem `image_alt_missing`. `image_alt_long` verwendet die konfigurierte Zeichengrenze. `image_size_attributes_missing` prüft fehlende/leere HTML-Attribute für width oder height; es misst keine intrinsischen oder gerenderten Abmessungen. Leeres Alt kann für dekorative Bilder beabsichtigt sein. |
| Content | `content_low_word_count`: erfasster Wortzähler unter der konfigurierten Grenze; Wortzahl allein belegt keine schlechte Inhaltsqualität. |
| Linkmenge | `high_internal_outlinks`, `high_external_outlinks`: Zahl der Anchor-Vorkommen, inklusive wiederholter Ziele. Ressourcenlinks zählen hier nicht. `internal_outlinks_missing` beschreibt den vollständigen erfassten DOM, nicht sämtliche möglichen Navigationen. |
| Linkannotation | `internal_nofollow_outlinks`, `external_nofollow_outlinks` berücksichtigen Linkannotation und die anwendbare Seitenpolicy des gewählten `indexability_agent`. `internal_sponsored_outlinks`, `external_sponsored_outlinks`, `internal_ugc_outlinks`, `external_ugc_outlinks` beschreiben beobachtete rel-Tokens. Die zusätzlichen sponsored/ugc-Regeln sind eigene Regeln, keine nachgewiesenen Referenzfilter. |
| Anchor-Text | `internal_outlinks_empty_anchor` prüft erfassten Anchor-Text. Bild-Alt, ARIA und berechnete zugängliche Namen werden daraus nicht abgeleitet. |

## URL-Formmerkmale

`url_uppercase`, `url_underscores`, `url_non_ascii`, `url_contains_space` prüfen
Pfad und Query nach gültiger UTF-8-Percent-Decodierung. Großgeschriebene
Hex-Ziffern in einem Escape sind kein Großbuchstabenbefund. Ein ungültig
UTF-8-codierter Teil bleibt für diese Zeichenmerkmale unbekannt. Host und Scheme
werden nicht als Großbuchstaben-/Non-ASCII-Pfadbefund bewertet.

`url_multiple_slashes` prüft wiederholte Slashes im tatsächlichen Pfad und
ignoriert das erforderliche `https://`. `url_parameters` zählt geparste
Query-Parameter; `+` wird für den Space-Hinweis nicht pauschal als Leerzeichen
interpretiert. `url_long` verwendet die vollständige normalisierte Anfrage-URL.
Lokalisierte Zeichen, Groß-/Kleinschreibung und Parameter können absichtlich
verwendet werden; diese Hinweise stellen kein Sicherheits- oder Indexierungsrisiko
fest. Es gibt hier keine erfundene Prüfung fehlender Fragmentziele, solange
die dafür notwendigen DOM-ID-Fakten nicht erfasst werden.

## Pixelprofil

`title_pixel_widths` und `description_pixel_widths` bewahren die Position jeder
erfassten Deklaration. `Some(0)` beziehungsweise JSON `0` ist ein beobachteter
leerer Text; `None` beziehungsweise JSON `null` bedeutet unbekannt. Eine leere
Liste bedeutet, dass keine positionsbezogenen Messungen vorhanden sind.

Das aktuelle lokale Profil ist
`roboto-2.138-simple-rounded-advances-v1; title20px; description14px; latin-greek-cyrillic`.
Es verwendet den offen lizenzierten eingebetteten Roboto-Regular-Font und
aufsummierte, pro Glyph auf Pixel gerundete Font-Advances bei 20 px für Titel
und 14 px für Descriptions. Nicht unterstützte Schriften, kombinierende Zeichen,
Emoji, fehlende Glyphen und Überläufe bleiben unbekannt. Unbekannte Messungen
lösen keinen Pixelhinweis aus und werden nicht mit Schätzwerten aufgefüllt.
Die Messung ist keine Browser-Layoutmessung und keine Vorhersage der Schrift,
Umbruch-/Kürzungsregeln oder Darstellung einer Suchmaschine.

Der Seiten-CSV hängt `Title Pixels`, `Description Pixels`,
`Title Pixel Measurements`, `Description Pixel Measurements` und
`Text Metrics Engine` an die bestehenden Spalten an. Die ersten zwei Felder
enthalten den primären numerischen Messwert oder bleiben bei unbekanntem Wert
leer. Die Measurement-Felder enthalten die positionsgetreuen JSON-Arrays,
beispielsweise `[561,null,0]`; ohne vorhandene Messliste bleiben sie leer.
Ein beobachteter Null-Pixelwert bleibt `0`. Das Engine-Feld bewahrt das Profil;
alte Projekte erhalten keine erfundenen Messwerte oder Engine-Angaben.

## Beziehungen und Crawl-Grenzen

Die Analyse berücksichtigt HTML- und HTTP-Link-Deklarationen mit ihrer Herkunft,
anwendbare Robots-Direktiven und die tatsächliche erfolgreiche Dokumentadresse.
Nicht beobachtete Canonical-, Hreflang- oder Pagination-Ziele bleiben unbekannt;
sie werden nicht als kaputte Ziele ausgegeben. Bekannte Redirects, Fehlerstatus,
Noindex und Canonical-Weiterverweise sind getrennte Befunde. Unvollständige
Zielcaptures erzeugen keine erfundenen fehlenden Rückverweise.

Hreflang verwendet ein dokumentiertes Google-Sprachcodeprofil und explizite
Sprach-/Regions-/Script-Code-Tabellen. Sprachkonflikte werden aus Deklarationen
abgeleitet, nicht aus einer vermuteten Inhaltssprache. `x-default` ist optional;
ein Hinweis auf fehlendes x-default ist keine Pflichtverletzung. Pagination-
Reziprozität und Anchor-Konsistenz sind Navigationsheuristiken. Google nutzt
rel-next/prev-Metadaten nicht mehr als Indexierungssignal.

`crawl_orphan_candidate` bedeutet, dass keine Anchor-Inlinks **von anderen
erfassten Seiten** beobachtet wurden. Seeds sind ausgenommen; Selbstlinks
beseitigen diesen Kandidaten nicht. Crawl-Limits, Include-/Exclude-Muster,
deaktiviertes Link-Folgen, ausstehende Captures und Fehler begrenzen die Aussage.
Ein solcher Kandidat ist kein Nachweis einer websiteweiten verwaisten Seite.
Die Graph-Coverage führt diese Grenzen mit.

CSV schützt formelartige Textzellen mit einem Apostroph; JSON bewahrt exakte
gespeicherte Daten. Details zur Ergebnis- und Exportprovenienz stehen in
[Erweiterte Extraktionen](advanced-extraction.md). Deterministische
Grenzfalltests stehen in `crates/core/src/onpage.rs`, `audit.rs`, `export.rs`
und `crates/app/src/analysis.rs`.
