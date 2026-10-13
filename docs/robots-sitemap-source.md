# B12: Robots-Sitemap-Quellfakten

Der Core erhält Sitemap-Deklarationen aus `robots.txt` als geordnete Originalvorkommen. Rohzeile, Rohwert, ursprüngliche Ordinalzahl, Zeilenende und Byte-Spans bleiben getrennt vom bisherigen URL-Kandidaten erhalten. LF, CRLF und CR werden erkannt; eine am unvollständigen Transportende abgeschnittene Zeile bekommt keinen Vollständigkeitsnachweis.

Das Netzwerk liest höchstens512KiB plus einen EOF-Sentinel. Vollständige Quelle bedeutet HTTP200, tatsächliches EOF, gültiges UTF8 und abgeschlossene Aufnahme. Eine206-Antwort, ungültiges UTF8, ein Readfehler oder ein überschrittenes Limit bleibt qualifiziert. Bei ungültigem UTF8 sind die Strings eine verlustbehaftete Projektion; der Digest bindet den aufgenommenen Byte-Präfix. Ein Readfehler am zusätzlichen EOF-Probe verändert die bisherige Robots-Regelentscheidung nicht. Ein früherer Readfehler bleibt ein Fehler im bisherigen Matcher.

Die bestehende Robots-Regelauswertung und historische Sitemap-Kandidatenprojektion bleiben erhalten. Der neue Scanner benennt Originalvorkommen zusätzlich. Er beweist keine RFC- oder Hersteller-Parität. Vollständige Originaldeklarationen werden noch nicht automatisch zu dauerhaften Root-Zeilen oder neuen Admission-Regeln.

Der [Core-Beleg](verification/robots-sitemap-source-core.json) bindet die vier integrierten Quelldateien:353 tatsächliche Core-Unit-Tests und Alltargets-Clippy bestehen. Darin sind18 neue reine Quelltests und sechs tatsächliche TCP-/Reqwest-Cachetests enthalten. Die Zahlen werden nicht zu früheren Läufen addiert.

Neue exakte Quell-CI, optimierter Release und Installation sind separat offen. Der aktuell installierte Transport-Release stammt von `d3d1e04` und enthält diese neueren Quellfakten noch nicht. Atomare Mitgliedschaft, Quell-Kohorten, Fehler-/Budgetzustände, Recovery/Vererbung sowie API/CSV/UI werden im [B12-Plan](parity/sitemap-b12-plan.md) weitergeführt. Ein leerer Frontier beweist keine vollständige Sitemap-Kohorte.
