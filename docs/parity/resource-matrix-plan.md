# B10: Ressourcenmatrix und nächste Abnahmen

**DESIGN_ONLY: geplant, noch nicht ausgeführt.** Dieser Plan baut auf dem eigenen B9-Vertrag von Commit `f107ce7cc5e610494f30df8a3a1d46d98d7870e4` auf. Die [strukturierte Matrix](resource-matrix-plan.json) enthält alle 108 benannten Zellen, erwartete V1-Entscheidungen und Kontrollfälle. Nachverfolgung: [GitHub-Issue 17](https://github.com/lia-xim/open-seo-spider/issues/17).

Die [B9-Beweise](../verification/resource-discovery-batch.json) decken 15 native Ressourcenfälle und die vier Speichern-/Abrufen-Kombinationen für Bilder ab. B10 hat **null ausgeführte Fälle**. Die Matrix ersetzt weder die [144 Feature-Abnahmen](../FEATURE_TODO.md) noch das [Register mit 416 Filtern und 32 Abläufen](granular-acceptance.json). Hersteller-Parität bleibt **NOT_PROVEN**.

## Umfang

| Bereich | Familien | Kombinationen | Gültig | Vor HTTP abzulehnen |
|---|---:|---:|---:|---:|
| Ressourcen: Speichern × Abrufen | 7 | 28 | 28 | 0 |
| Navigation: Speichern × Abrufen × Expandieren | 10 | 80 | 60 | 20 |
| Gesamte eigene Matrix | 17 | 108 | 88 | 20 |

Ressourcenfamilien: CSS, JavaScript, Bilder, Medien, PDF, SWF und andere. Navigationsfamilien: interne/externe Anchors, iframe, Refresh, Canonical, hreflang, Pagination, AMP, mobile Alternate und PDF-Annotationen. `expand=true` bei `fetch=false` muss vor jeder HTTP-Anfrage scheitern, einschließlich Quellseite und robots.txt.

Jede gültige Zelle verwendet eigene vollständige Quellen und eindeutige Ziele. Scope, Robots, Filter, URL-Policy, Tiefe und ursprüngliche Credential-Origins sind ausdrücklich festgelegt. Keine zusätzliche Quelle oder Seed darf einen abgeschalteten Abruf verdeckt erlauben. Ein erlaubtes Ziel ist jeweils die positive Kontrolle für ein gesperrtes Ziel. Kein Abruf bedeutet keine erfundene Antwort, Größe, MIME- oder Body-Hash-Angabe.

## Zuerst den Vertrag festlegen

Diese Quellcode-Beobachtungen sind **Vertragsfragen**, keine bereits ausgeführten Gegenbeweise. Bestehende V1-Konfigurationen und gespeicherte Grants dürfen ihre Bedeutung nicht still ändern.

| Reihenfolge | Frage | Geplanter Vergleich |
|---|---|---|
| 1 · CG-03 | Dürfen Ressourcen-Originlisten und externe Linkprüfung Redirects unabhängig von der ursprünglichen Rolle erlauben? | CSS darf zur freigegebenen Asset-Origin; Anchor zum gleichen Ziel bekommt dadurch keine Navigationserlaubnis. Drei Origins, jede Stufe, Robots/Credentials und native Policy getrennt prüfen. |
| 2 · CG-01 | Wie sollen `ugc`/`sponsored` sowie interne/externe Overrides behandelt werden? Aktuell setzt nur das exakte `nofollow`-Token das entsprechende eigene Flag. | Token einzeln/kombiniert, erlaubte und gesperrte Ziele; unabhängiger erlaubter Grant darf bestehen bleiben. |
| 3 · CG-02 | Welche Qualifikatoren und Beziehungen gelten für HTTP-`Link` gegenüber HTML? Der aktuelle Headerpfad übernimmt kein Rel-Nofollow. | `canonical nofollow`, Stylesheet und mobile Alternate als HTML/Header; Kontextabweichung und Headerbasis getrennt kontrollieren. |

Die [offizielle Herstellerkonfiguration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/#follow-internal-or-external-nofollow) beschreibt Nofollow-Schalter auch für `ugc` und `sponsored`. Das begründet eine konkrete Feature-Frage, keine Laufzeitparität. [Google-Linkqualifikation](https://developers.google.com/search/docs/crawling-indexing/qualify-outbound-links) und [HTTP-Web-Linking](https://www.rfc-editor.org/rfc/rfc8288.html#section-3) liefern öffentliche Begriffe für die eigenen Kontrollen.

Vier weitere Profilgrenzen sind ausdrücklich zu prüfen: Nofollow wirkt derzeit auf alle eigenen Referenzarten; Dokumentregeln verwenden `indexability_agent`, robots.txt den HTTP-`user_agent`; eine CSS-Antwort verleiht einer Navigationsrolle keine CSS-Expansion; Redirect-Headerdirektiven dürfen nicht als beobachtete finale Dokumentdirektiven erfunden werden. Die [öffentlichen Meta-/X-Robots-Regeln](https://developers.google.com/search/docs/crawling-indexing/robots-meta-tag) ersetzen keine Prüfung dieser eigenen Agenten- und Quellenbindung.

## Danach fehlende Beweise abarbeiten

- [ ] **P1:** `resource-products-every-type-and-navigation`, `relationship-discovery-each-positive-negative`: alle 108 Zellen mit tatsächlichem Requestlog, gespeicherten Vorkommen, Grants und Bericht/private-Metadaten-Abgrenzung ausführen.
- [ ] **P1:** `link-directive-four-way-traversal`: 49 geplante Direktivenkontrollen — 16 Overrides, 11 Rel-Tokenfälle, 14 Agent-/Quellenfälle und 8 HTML-/Header-Vergleiche.
- [ ] **P1:** `resource-cross-origin-redirect-hop-credentials`: 12 Kettentemplates je sieben Ressourcentypen plus vier Navigationsrollen, insgesamt 88 geplante Kettenkontrollen.
- [ ] **P2:** `resource-role-unions-reverse-and-concurrent`, `resource-selected-all-kind-targets`, `image-occurrence-alt-and-selected-source`: Reihenfolge/Parallelität, alle ausgewählten Rollen ohne neue Expansion, mehrere Bildquellseiten und fehlendes/leeres/Whitespace-Alt. Browser-currentSrc bleibt separat.
- [ ] **P2:** `css-encoding-environment-fallback`, `css-native-grammar-context-and-decoding`: 21 geplante Response-/MIME-Kontrollen, zusätzliche Grammatik und vererbte Encoding-Herkunft; unbekannte Quellen bleiben unbekannt.
- [ ] **P3:** `resource-interruption-each-stage`, `resource-budget-exact-plus-one-all-limits`: tatsächlicher Prozessabbruch an Transaktionsgrenzen sowie jede Grenze exakt/plus-eins; kein vollständiger gespeicherter Prefix bei Budgetfehlern.
- [ ] **P4:** `resource-native-renderer-enforcement`: die aktuelle Ablehnung vor HTTP erhalten. Browserunterstützung braucht erst einen eigenen Vertrag für jede Navigation und Subrequest.

Der erste überschaubare Gate umfasst **108 Matrixzellen plus vier vergleichende Canary-Gruppen**: HTML/Header-Nofollow, UGC/Sponsored, Ressource zu nicht freigegebener dritter Origin sowie Navigation zu ausschließlich für Ressourcen erlaubter Origin. Das sind 112 geplante Abnahmeeinheiten, keine 112 unabhängigen Crawl-Aufrufe. Jede Zelle erhält ein eigenes Ergebnis und erhaltene Fehlerbeweise; ein früher Abbruch oder ein leerer Export ist kein PASS für die übrige Matrix.
