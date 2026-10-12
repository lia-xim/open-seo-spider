# Erweiterte Extraktionen

`CrawlConfig.custom_extractions` enthält benannte Definitionen. Jede Definition
verwendet `selector` als Ausdruck; `method` wählt `css`, `xpath` oder `regex`.
Fehlendes `method` bleibt für ältere Projekte `css`; fehlendes `output` bleibt
`auto` und bewahrt das bisherige Text-/Attribut-/Skalarverhalten. Diese Funktionen verwenden
eigene Adapter über HTML5-Parsing, Rust Regex und sxd XPath. Gleiche Namen wie
Filter einer anderen Software belegen keine Verhaltensgleichheit.

```json
{
  "custom_extractions": [
    { "name": "Preis", "selector": ".price", "method": "css" },
    { "name": "Produktlinks", "selector": "a.product", "attribute": "href" },
    { "name": "Produkt-Markup", "selector": ".product", "output": "outer_html" },
    { "name": "Artikel-Markup", "method": "xpath", "selector": "//article", "output": "inner_html" },
    { "name": "Artikelzahl", "method": "xpath", "selector": "count(//article)", "output": "function_value" },
    { "name": "Artikeltext", "method": "xpath", "selector": "//article//text()", "max_results": 500 },
    { "name": "SKU", "method": "regex", "selector": "SKU-([0-9]+)", "regex_group": 1, "regex_flags": "i" }
  ]
}
```

## Quelle und Ergebnis

Ohne Browserrenderer stammt die Extraktion aus `raw_html`. Nach erfolgreichem
Rendering wird dieselbe Definition auf `rendered_html` angewandt; diese Ergebnisse
stehen in `page.custom_extractions`. `page.raw_metadata.custom_extractions` bewahrt
die vorherigen HTTP-Ergebnisse. Ein Rendererfehler bleibt in `renderer_error`
sichtbar und verwandelt den HTTP-Befund nicht in einen erfolgreichen Browserbefund.
CSS und XPath prüfen den durch HTML5-Parsing reparierten DOM. Regex prüft den
gewählten HTML-Quelltext; HTML-Entities bleiben dort als Quelltext erhalten.

Jedes Ergebnis enthält `name`, `selector`, `attribute`, `method`, `output`, `source`,
`engine`, `result_type`, `values`, `matched_count`, `truncated` und `error`.
`values` ist immer ein Array von Zeichenketten. XPath-Skalare ergeben ein Element
mit dem Typ `string`, `number` oder `boolean`; eine beobachtete Zahl `0`, ein
Boolean `false` und eine leere Zeichenkette sind Ergebnisse. Ein erfolgreicher
leerer Nodeset enthält keine Werte und `matched_count: 0`.

`matched_count` zählt extrahierbare Werte **vor** der Ergebnisbegrenzung.
`values.length` zählt zurückgegebene Werte. `truncated: true` bedeutet, dass die
Ergebnisbegrenzung weitere beobachtete Werte abgeschnitten hat. Fehlende CSS-
Attribute und nicht beteiligte optionale Regex-Gruppen liefern keine erfundenen
Leerwerte. Bei einem Extraktionsfehler bleiben `error` gesetzt und `matched_count`
unbekannt (`null`); ältere importierte Ergebnisse können ebenfalls eine unbekannte
Anzahl haben. Eine unvollständige HTTP-Antwort bleibt zusätzlich auf Seitenebene
`truncated`; ein niedriger Match Count beweist dann keine vollständige Abwesenheit.

## Unterstützte Syntax

| Methode | Verhalten und Grenzen |
| --- | --- |
| CSS | `scraper-html5-css`; CSS-Selektoren auf dem HTML5-DOM. Ohne `attribute` wird der Text der Nachfahren mit normalisierten Leerzeichen ausgegeben. Mit `attribute` wird der vorhandene Attributwert ausgegeben. Werte wie relative `href` werden hier nicht zu absoluten URLs umgeschrieben. |
| XPath | `sxd-xpath-1.0-html-dom`; XPath-1.0-Ausdrücke, Achsen, Prädikate, Nodesets und Skalare. Nodesets werden in Dokumentreihenfolge als XPath-Stringwerte ausgegeben. Core-Funktionen sind unterstützt **außer `id()` und `lang()`**; Variablen, Erweiterungsfunktionen und XPath 2.0/3.0 sind nicht unterstützt. |
| Regex | `rust-regex`; Rust-Regex-Syntax mit Unicode. Lookaround und Backreferences sind nicht unterstützt. Alle nicht überlappenden Matches werden verarbeitet; ein JavaScript-Flag `g` ist daher kein gültiges Konfigurationsflag. |

## Ausgabemodi

| `output` | CSS | XPath | Regex |
| --- | --- | --- | --- |
| `auto` | Normalisierter Nachfahrentext oder vorhandenes `attribute` | Stringwerte in Dokumentreihenfolge; Skalare behalten ihren Typ | Match oder Capture-Gruppe |
| `text` | Normalisierter Nachfahrentext | XPath-Stringwerte; Skalare behalten ihren Typ | Nicht erlaubt |
| `inner_html` | Inhalt jedes ausgewählten Elements | Inhalt ausgewählter Elemente | Nicht erlaubt |
| `outer_html` | Jedes ausgewählte Element inklusive Tags | Ausgewählte Elemente inklusive Tags | Nicht erlaubt |
| `function_value` | Nicht erlaubt | Nur `string`, `number` oder `boolean`; Nodesets sind ein Fehler | Nicht erlaubt |

`attribute` lässt sich nur mit `output: "auto"` kombinieren. HTML-Ausgaben
verwenden den nativen HTML5-Serializer des bereits reparierten DOM: Entities,
Void-Elemente, Script-Rawtext und fremde Namespaces werden als HTML ausgegeben.
Die Ausgabe ist keine bytegenaue Kopie des ursprünglichen Quelltexts. Bei XPath
entsprechen Elementausgaben denselben DOM-Elementen wie CSS; die Wahl der
Dokumentwurzel `/` serialisiert das gesamte HTML-Dokument einschließlich eines
vorhandenen Doctype. Für die Dokumentwurzel haben InnerHTML und OuterHTML dasselbe
Ergebnis. Ein ausgewähltes leeres Element liefert bei InnerHTML einen erfassten
leeren Wert, keinen Null-Match.

XPath-HTML-Modi erlauben nur Elemente beziehungsweise die Dokumentwurzel.
Attribute, Text-, Kommentar-, Namespace- oder Processing-Instruction-Knoten
erzeugen einen expliziten Typfehler; ebenso erzeugt HTML-Ausgabe für einen Skalar
einen Fehler. Eine gemischte Auswahl wird vollständig auf diesen Fehler geprüft,
auch wenn `max_results` den problematischen Knoten später abgeschnitten hätte.
Mit `text` oder `auto` lassen sich diese Knoten als XPath-Stringwerte extrahieren.
`function_value` macht eine gewünschte Skalarausgabe ausdrücklich; `0`, `false`
und `""` bleiben gültige Werte. Ein leeres Nodeset ist bei erlaubtem Nodeset-Modus
eine erfolgreiche leere Auswahl.

Für XPath sind HTML-Elementnamen ohne Namespace zugänglich, beispielsweise
`//div/@data-id`. Fremde SVG-/MathML-Namespaces bleiben erhalten und werden über
`namespaces` gebunden:

```json
{
  "name": "SVG-Titel",
  "method": "xpath",
  "selector": "//svg:svg/svg:title/text()",
  "namespaces": { "svg": "http://www.w3.org/2000/svg" }
}
```

`xml` ist bereits gebunden. Eigene Bindungen dürfen `xml` oder `xmlns` nicht
überschreiben. `text()` wählt direkte Textknoten; `//text()` kann auch Text von
Nachfahren erfassen. HTML5-Reparaturen, etwa automatisch ergänzte Elemente oder
rekonstruierte Formatierung, können sich vom ursprünglichen Quelltext unterscheiden.

Für Regex ist `regex_group: null` beziehungsweise fehlend der vollständige Match;
`0` bezeichnet ebenfalls den vollständigen Match. Positive Ganzzahlen wählen eine
vorhandene Capture-Gruppe. `regex_flags` erlaubt die unterschiedlichen Zeichen
`i` (Groß-/Kleinschreibung ignorieren), `m` (Zeilengrenzen), `s` (Punkt erfasst
Zeilenumbrüche), `x` (Pattern-Leerzeichen ignorieren) und `U` (Greediness umkehren).
Inline-Rust-Regex-Flags funktionieren ebenfalls. Gruppen und Regex-Flags sind nur
bei `method: "regex"`, Namespace-Bindungen nur bei XPath erlaubt. `attribute` ist
nur bei CSS erlaubt; XPath-Attribute werden mit `/@name` gewählt.

## Validierung und Ressourcen

- Höchstens 128 Definitionen; Namen mit 1–128 Zeichen ohne Steuerzeichen,
  nach Trim und ASCII-Kleinschreibung eindeutig.
- Ausdrücke mit 1–8192 UTF-8-Bytes und nicht leerem Inhalt.
- `max_results` standardmäßig 10000, erlaubt 1–100000 pro Definition.
- Höchstens 32 XPath-Namespace-Bindungen; Präfixe maximal 128 Bytes und URIs
  maximal 2048 Bytes. XPath-Ausdrücke haben zusätzlich eine Grenze von 64
  Verschachtelungsebenen und 1024 gezählten Syntaxoperationen.
- Die XPath-DOM-Konvertierung begrenzt die Verarbeitung auf 100000 Knoten und
  256 Ebenen. Überschreitungen liefern einen expliziten Fehler und keine
  scheinbar erfolgreiche Teilauswahl. Regex hat interne Compile-/DFA-Grenzen
  von jeweils 1 MiB.

Ungültige Crawl-Konfigurationen werden vor dem Crawl abgelehnt. Seitenspezifische
Auswertungsfehler erscheinen im Ergebnis und in den Seitenwarnungen; sie sind
keine Null-Matches.

## CSV und unverändertes JSON

Der Extraktions-CSV enthält pro Wert eine Zeile; auch eine leere oder fehlerhafte
Extraktion behält eine Zeile mit ihren Evidenzfeldern. Die bestehenden Spalten
`URL`, `Name`, `Selector`, `Attribute`, `Source`, `Match Count`, `Match Index`,
`Value` bleiben an ihrer Position. `Match Count` ist die Zahl zurückgegebener Werte.
Angehängt sind `Method`, `Engine`, `Result Type`, `Observed Match Count`,
`Truncated`, `Error`, `Output Mode`. Eine unbekannte beobachtete Anzahl bleibt leer.

Rust-CSV-Exporte und der Browserexport der sichtbaren Tabelle schützen Textzellen, deren erstes bedeutendes Zeichen
`=`, `+`, `-` oder `@` ist, mit einem führenden Apostroph. Die Prüfung erkennt
auch vorgeschaltete Leerzeichen, Tabs, Zeilenumbrüche und BOM. Das betrifft auch
Website-Texte, Konfigurationsnamen und Fehlermeldungen. Echte typisierte Zahlen
und Booleans bleiben erhalten; XPath-Werte mit `result_type: "number"` und
gültiger numerischer Darstellung werden als Zahlen exportiert. JSON behält die
exakten gespeicherten Daten. Wenn unveränderte Textwerte benötigt werden, ist
der JSON-Projektexport die geeignete Grundlage.

Der JavaScript-CSV bewahrt Ergebniswerte als JSON und führt Scriptname, Typ,
Fehler und Laufzeit mit. Der Cookie-CSV exportiert ausschließlich Attribute
einschließlich `value_length`, niemals den Cookie-Wert. Ein fehlender beobachteter
Längenwert bleibt leer und wird nicht aus importierten Cookie-Werten rekonstruiert.

Die deterministischen Regressionen stehen in
`crates/core/src/extraction_engine.rs` und `crates/core/src/export.rs`.
