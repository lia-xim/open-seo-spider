# CDN-Referenz V2: konkreter Getter, Matcher und verbleibende Grenzen

Der erfolgreiche Getter ist bis zum konkreten Klassifizierer und dessen Regexgenerator statisch aufgelöst. Das ist ein belastbarer Fortschritt gegenüber V1; eine vollständige Vendorparität oder ein Netzwerkgrant folgt daraus nicht. Es wurde kein Vendorprozess gestartet und kein Rustprodukt verändert.

## Herkunft und Werkzeuggrenze

Die bestehende REA-Sitzung war targetfrei. procedure_callers, procedure_references und xrefs meldeten tatsächlich target_required; REA-CAPABILITY-LIMIT.json hält diese Antwort fest. Daher wurde der erlaubte exakte Constant-Pool-Fallback auf demselben bereits inventarisierten 24.3-JAR genutzt. Je Suche: 3146 Klassen in den begrenzten Spiderpaketen, 11.415.671 Klassenbytes, 8.549.079 CP-Bytes; keine Methoden wurden während der Suche dekompiliert. Lizenz-/Credential-/Providerpfade blieben ausgeschlossen. Ausgewertet wurden anschließend nur der eindeutige Getter-Consumer und kleine, konkret aufgerufene URL-/Regexhelfer. Artefakt-/Occurrence-IDs und JAR-SHA sind in den Dateien gebunden.

Die offizielle CDN-Dokumentation bleibt die bereits in V1 gepinnte Primärquelle: https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/#cdns . Domain und Unterordner samt Inhalt, interne Darstellung und zusätzliche Extraktion sind dort beschrieben. Welche zusätzlichen Einzelwerte damit gemeint sind, ist weiter offen.

## Tatsächliche Kette

SpiderInternalURLConfig.id814175767()List → seo/spider/spider/id1768309875.constructor → List-map mit false,false → uk/co/screamingfrog/seospider/S/id281146822.id281146822(String,boolean,boolean) → Regexliste. Der tatsächliche CDN-Test ist candidate.matches(regex) über anyMatch. Der Lambda-Bootstrap ist als java.lang.String.matches bestätigt, nicht aus einem Methodennamen geraten.

Für normale geparste ASCII-Hosts bildet der false-Zweig einen gequoteten Host, entfernt davor ein einzelnes www. und erlaubt anschließend ein optionales literales www. Beliebige Unterdomains werden an diesem CDN-Pfad nicht zugeschaltet. Der erzeugte Ausdruck beginnt mit ^http(s)?://; er bindet sich somit nicht an die deklarierte HTTP(S)-Variante. Ein geparster expliziter Port wird als :port eingebaut. Der gequotete Verzeichnispfad wird durch (?s).* ergänzt. Bei normalen gültigen CDN-Zeilen mit abschließendem Slash bedeutet dies einen Verzeichnisprefix, keinen beliebigen Hostteil oder Regex aus Benutzereingaben.

Die Quellen zeigen getrennte URI-Komponenten: Fragment wird vor Query abgespalten, Path ist ein eigenes Feld. Die Vorverarbeitung des konfigurierten Authority-Texts kleinschreibt und vereinheitlicht Prozent-Hexzeichen. Das beweist nicht, dass jeder spätere Candidate identisch normalisiert ist. IDNA, Unicode, IPv6, Default-Port-Elision, fehlerhafte Escapes sowie Query-/Fragment-Sonderformen der Regel bleiben ausdrücklich Countercases; daraus wird noch kein eigener Matcher implementiert.

Der allgemeine Internal-Test nimmt zuerst den exakten Startstring mit equalsIgnoreCase, danach die Site-Regex und danach CDN-anyMatch. Die anschließende LinkScope-Methode liefert bei true INTERNAL, sonst EXTERNAL; die obfuskierten Enumfelder sind durch ihre Initialisierung aufgelöst. Die Liste ist hier eine OR-Menge und keine Prioritätsliste. Andere Methoden kombinieren CDN- bzw. Site-Matches zusätzlich mit der bestehenden Include-Regex-Erlaubnis. Eine Rückgabe für Berichtsscope darf daher nicht als universelle Fetchfreigabe behandelt werden.

## Konsumenten und offene Extraktionsfrage

Der konkrete Consumer implementiert seo/spider/spider/id487602255. Die exakte Interface-Suche findet zwölf Klassen; CLASSIFIER-INTERFACE-XREF-SEARCH.json enthält deren Namen, Methodendeskriptoren und CP-Indizes. Der eindeutige Getter-Consumer ist damit wirklich in der Spider-Klassifikation angekommen. Diese begrenzte Untersuchung beweist noch nicht alle konkreten Titel-/Head-/Link-/Audit-Feldzuweisungen, die von ihm abhängen. Die Interface-Callsites wurden nicht pauschal als Laufzeitdispatch oder zusätzliche Extraktion ausgegeben. Diese Grenze ist der nächste gezielte Untersuchungsauftrag, falls weitere statische Auflösung gewünscht wird.

## Nächster begrenzter Rustvorschlag

Noch keine Implementierung einfrieren. Zuerst die erlaubten Ruleformen und den eigenen versionierten Regelvertrag festlegen: HTTP(S)-Domains plus Slash-begrenzte Unterpfade, klare Scheme-/www.-/Portentscheidung und ein getrenntes Ergebnis für Berichtsklassifikation. Den bestehenden Exact-Origin-Vertrag unverändert erhalten; None/null/Alt-Konfiguration und alte Reportbytes bleiben kompatibel. Regelrevision und tatsächlich getroffene Regel sollen getrennt nachprüfbar werden. Vendorabweichungen offen benennen, insbesondere wenn die eigene Sicherheitssemantik enger bleibt.

Der Slice darf Navigation, Redirect-/Resource-Transport, Authentifizierung, Robots oder Credentialweitergabe nicht automatisch erweitern. Im aktuellen OpenSEO extrahiert der bereits gelesene Crawler vorhandenes HTML grundsätzlich aus tatsächlich empfangenen HTML-Seiten; ein neues mehr-extrahiert-Flag ohne konkrete fehlende Felder wäre unbegründet. Erst nach Feldinventur darf ein tatsächlicher kleiner Extraktions-/Audit-Slice vorgeschlagen werden. Fehlender Crawl oder fehlende gespeicherte Quelle bleibt unknown.

## Konkrete spätere Nachweise

Die zehn Beispiele in EVIDENCE.json sind aus dem ausgewählten Predicate abgeleitete Erwartungen für bereits normalisierte ASCII-Eingaben, keine ausgeführten Tests: HTTP↔HTTPS, optionales www., keine beliebige Unterdomain, /assets/ gegenüber /assets2/, Query-/Fragment-Suffix, expliziter Port und Path-Großschreibung. Weitere bewusst offene Gegenfälle: explizite :80/:443 gegen ausgelassenen Port, Uppercase-Candidate, IDN/Punycode, IPv6, Prozent-Hexschreibung, encoded slash, doppelte Slashes, Dotsegmente, Query-/Fragmentzeichen in der Regel, leere/duplizierte Zeilen und überlappende Regeln. None/Legacy sowie aktuelle gespeicherte Config-/Berichtbytes müssen separat erhalten bleiben.

Eine spätere eigene Fixture soll A/B-Origin und echte GET-Ledger, initiale/finale URL, Redirect-/Resource-Denials und berichteten Internal/External-Scope gleichzeitig prüfen. Daraus darf erst nach tatsächlicher Ausführung ein eigener Produktbeleg entstehen. Ein Vendor-Blackboxtest oder weitere Vendorstarts sind hier nicht autorisiert und nicht erfolgt.
