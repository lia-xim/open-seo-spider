# Native Prüfung der gespeicherten Sitemap-Befunde

Stand: **Prüfskripte geschrieben und im begrenzten Quellumfang geprüft;
NOT_RUN / NOT_COMPILED / NOT_INSTALLED.** Es liegt kein neuer nativer Test-Pass vor.

Diese Vorbereitung gilt für den Quellcommit `f43efc1` und das eingefrorene
Inventar `69492d0c…` mit 297 Eingaben. Die separate Bedienerweiterung
`f4ed4b9` / `e9bbc545…` ändert zwei UI-Dateien. Sie benötigt ihre eigene
Prüfbindung und tatsächliche Ausführung; die hier vorbereiteten nativen
Ergebnisse werden nicht auf diese geänderte Oberfläche übertragen.

Für den neuen UI-Stand `e9bbc545…` sind eine eigene vollständige Suite und ein
eigener Builder vorbereitet. Sie prüfen die feste Herkunft beider Snapshots,
295 unveränderte und zwei exakt geänderte UI-Eingaben, dieselben 388 Pflichtnamen
und vier Browser-Worker-Fälle. Auch diese getrennten Treiber sind **NOT_RUN**.
Die Prüfung des neuen Formulars und die nativen Abnahmerouten bleiben separat offen.

| Getrennte Prüfroute | Vorbereiteter Umfang | Tatsächlich ausgeführt |
| --- | --- | --- |
| API, Offline-CLI und offizielles MCP-SDK 2.3.1 | Derselbe gebaute Kandidat, beide SDK-Transporte, 26 Werkzeuge, sieben Filter, Fortsetzung, Exportdescriptor, vollständiger kleiner CSV-Abgleich und unveränderte gespeicherte Website-Daten | 0 Läufe |
| Browser mit kontrollierten API-Antworten | Die bestehenden 13 Mock-Gruppen; keine Behauptung eines tatsächlichen Core-/SQLite-Durchlaufs | 0 Gruppen |
| Browser mit tatsächlichem nativen Server und Crawl | 20 Gruppen mit 405 echten Fixture-Seiten, sieben Filter gegen gespeicherte Fakten, mehr als 400 exportierte Treffer, vollständiger API-/CSV-Abgleich, dünnes leeres Fenster mit Fortsetzung, Tastatur und 320/1440 Pixel | 0 Gruppen |
| Physische Antwortgrößen | Zwei echte kleinere Crawl-Baselines; zwölf ausdrücklich synthetische Offline-Abzweige zur Kalibrierung und exakten Grenze/+1 von vollständigem App-JSON8MiB und Finding-CSV64MiB | 0 Fälle |

Bei den physischen Abzweigen werden ausschließlich erlaubte Warnungstexte
in erhaltenen gespeicherten Berichten verlängert und die zugehörigen
Integritätshashes angepasst. Das sind Testdaten, keine angeblich von Websites
gemessenen Warnungen. Unveränderte Berichtsteile und Datenbankinhalte bleiben
Teil des künftigen Orakels. Der neue Finding-CSV-Vertrag enthält zehn Spalten,
den tatsächlichen Kontext und vollständige typisierte Befunde. Frühere
Rohdaten-CSV-Messungen ersetzen diesen Vertrag nicht.

Die ausgewählten Treiber verlangen vor ihrem aktiven Einsatz die feste private
Quellveröffentlichung, tatsächliche vollständige Suite-Logs, 388 Pflichtnamen,
vier reale Browser-Worker-Fälle und denselben gebauten Kandidaten. Erwartete
mindestens 1170 Workspace-Passes sind weiterhin eine Erwartung. Ein gültiges
leeres `cargo fmt`-Log darf als tatsächlicher erfolgreicher Lauf zugelassen
werden; leere Manifeste, Freigaben oder Binärdateien bleiben ungültig.

Die Quellkorrekturen binden SDK-Transportenden, tatsächliche Prozessenden und
Pipe-Drain, eine unveränderliche monotone Frist sowie den vollständigen
gespeicherten API-/CSV-Zeilenbestand. Ein erfolgreicher HTTP-Dateiabschluss
allein beweist keinen internen TX-/Worker-Join. Unaufgelöste Prozesse und
Fixture-Daten bleiben erhalten; ein Fristablauf darf kein positives Ergebnis
erzeugen. Harte Abbruchlatenz und RAM-Grenzen sind damit noch nicht gemessen.

Die Archivkopien im Repository dienen der Nachvollziehbarkeit. Die Treiber
enthalten ausdrücklich gebundene lokale Pfade und sind keine portablen
Startskripte. Nur der ausgewählte SDK-/Mock-Guardian wurde nach `Task/work`
kopiert; Browser- und Größenprüfer behalten ihre eigenen unveränderlichen
E-Verzeichnisse. Frühere Versionen mit bestätigten Befunden bleiben als
**SUPERSEDED_NOEXEC** erhalten. Es wurde keine operative Ausführungsfreigabe
erzeugt und kein Treiber importiert oder ausgeführt.

Offen bleiben die tatsächlichen Läufe selbst, die geänderte Einstellungs-UI,
positive 50k/50MiB-Quellen, unvollständige und unbekannte Quellzustände,
stale-Kontext-/Antwort-Races, Abbruch/Disconnect/späte Fehler/Join/Shutdown,
Selected-Varianten, Recovery, gemessene RAM-/Latenzgrenzen, optimierter Release,
datenbewahrende Installation und Linux/Container. Der installierte vorherige
Release wurde nicht geändert. Dieser Vorbereitungsstand änderte keine Abnahme.
Nach dem gesonderten MCP-Umfangsabgleich liegt der aktuelle Katalog bei
30 erfüllt / 68 teilweise geprüft / 46 offen; Herstellerparität ist **NOT_PROVEN**.
