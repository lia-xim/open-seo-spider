# Ressourcenvertrag V2: ausdrückliche Auswahl und erhaltene Herkunft

Stand: **IN_PROGRESS**. Die Implementierung wird mit nativen Fixtures und separaten API-/CLI-/MCP-/UI-Gates geprüft. Ein bestandener historischer V1-Kompatibilitätstest oder API-Mock ist kein nativer V2-, Release- oder Screaming-Frog-Paritätsbeleg. Der installierte B9-Release bleibt bis zum gesondert geprüften Update der Laufzeitstand. [Aktueller Arbeitsstand](FEATURE_TODO.md), [B10-Issue 17](https://github.com/lia-xim/open-seo-spider/issues/17), [eingefrorener Abnahmeplan](parity/resource-matrix-plan.json).

Die [lokale Abnahme](verification/resource-contract-checkpoint.json) belegt jetzt 387 Standard-Rust-Tests, 108 exakte Schalterfälle in beiden Profilen (216 Ausführungen), 44 Antwortkontrollen, 13 tatsächliche API-/CLI-/MCP-Gruppen mit 19 gespeicherten Jobs und zehn echte Browser-zu-Rust-Prüfungen am Debug `a5881b7c…`. Workspace-Clippy und Formatprüfung bestehen. Die Einzelbelege erhalten vollständige Fallnamen, Quellhashes, ursprüngliche Grants, HTTP-Kontrollen und frühere Fehlversuche. Der eingefrorene Designplan behält seinen historischen `DESIGN_ONLY`-Stand; ausgeführte Fälle stehen in den separaten Beweisen.

Die [zusätzliche native Redirect-Suite](verification/resource-contract-redirect-summary.json) besteht alle 88 geplanten IDs in beiden Profilen (176 Ausführungen) sowie zwölf getrennte Boundary-IDs in beiden Profilen (24 Ausführungen). Das ist ein zusätzlicher Rust-Test mit 200 tatsächlich ausgeführten Fällen. Originale Rollen und jeder tatsächliche Hop bleiben sichtbar; historische V1-Ausnahmen werden getrennt erwartet. Ein vertrauenswürdiger HTTPS-Positivfall bleibt offen.

Zusätzliche Encoding-/Unterbrechungs-/Budget-/Browser-Abnahmen, exakte Commit-CI, unveränderlicher Release und Installation bleiben gesonderte offene Schritte. Aus diesen begrenzten eigenen Abnahmen entsteht keine vollständige Hersteller-Parität.

## Auswahl

Eine abwesende oder `null` gesetzte Eigenschaft `resource_discovery.contract` behält den B9-V1-Vertrag. `resource_discovery:{}` wählt ebenfalls V1. Bestehende Konfigurationen, Grants, Berichtshashes und Reopen-/Resume-Vorgänge werden nicht automatisch umgestellt. Die Oberfläche bietet dafür ausdrücklich **Aktuelle Regeln verwenden** an.

```json
{
  "resource_discovery": {
    "contract": {
      "engine": "open-seo-spider/resource-contract-v2",
      "follow_internal_qualified_links": false,
      "follow_external_qualified_links": false
    }
  }
}
```

Ein ausdrücklich vorhandenes `contract:{}` wählt V2 mit diesen Standardwerten. Unbekannte Engines und Felder sowie `follow_rel_nofollow:true` oder `follow_document_nofollow:true` zusammen mit V2 werden vor HTTP abgelehnt. Diese bisherigen globalen Overrides sind mit den beiden neuen Scope-Schaltern nicht gleichzeitig zulässig.

## Link- und Dokumentregeln

V2 wertet die exakten, ohne Groß-/Kleinschreibung verglichenen Rel-Tokens `nofollow`, `ugc` und `sponsored` sowie tatsächlich beobachtetes Dokument-Nofollow gemeinsam aus. Die Erlaubnis richtet sich nach dem internen oder externen **Ziel**. Es gibt vier unabhängige Schalterkombinationen. Ohne Qualifikation werden gültige Ziele weiterhin durch ihre übrigen Scope-, Fetch-, Policy- und Robots-Regeln gesteuert.

`decision.qualification` erhält Engine, Scope, `rel_qualifiers`, Quelle, `document_nofollow` und `follow_allowed`. Die Quellen heißen `html_rel`, `http_link_rel` oder `not_applicable`. Die tatsächliche Dokumentdirektive bleibt auch bei einem erlaubten Override sichtbar. Diese eigene Entscheidung beweist keine Browserauswahl und keine vollständige Hersteller-Parität. Die öffentliche [Herstellerkonfiguration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/#follow-internal-or-external-nofollow) begründet den Vergleich; [Google-Linkqualifikation](https://developers.google.com/search/docs/crawling-indexing/qualify-outbound-links) und [HTTP-Web-Linking](https://www.rfc-editor.org/rfc/rfc8288.html#section-3) beschreiben öffentliche Begriffe.

## Ursprüngliche Rollen und Weiterleitungen

Ein V2-Grant erhält `reference_kind` aus seinem ursprünglichen Quellvorkommen. Ein späterer MIME-Typ, eine gemeinsam verwendete Fetch-URL oder ein anderer Grant schreibt diese Herkunft nicht um. Zusätzliche Ressourcen-Origins erteilen Ressourcenrollen eine exakt begrenzte Erlaubnis; sie erzeugen keine allgemeine Navigationserlaubnis. Ressourcenrollen benötigen diese Grenze auch bei aktivem `check_external` oder weiter gefasster nativer URL-Policy.

Jeder Redirect wird gegen dieselben ursprünglichen autorisierenden Grants und die bereits durchlaufene Kette geprüft. Eine Rolle darf nicht mitten in der Kette durch eine andere Rolle ersetzt werden, um Scope oder Expansion zu erweitern. Ein späterer unabhängiger Grant kann gespeicherte Quellen nur nach erneuter Prüfung der tatsächlich beobachteten Kette nutzen. Credentials bleiben an ihre ursprünglichen Origins gebunden. Native Policy und Robots bilden zusätzliche Grenzen; Check-only erlaubt keine neue Expansion.

`resource_response.redirect_decisions` erhält Quell-URL, rohe Location, aufgelöstes Ziel, ursprüngliche Rollen und Grant-IDs, Scope-/Policy-/Robots-Entscheidungen, Grund, Erlaubnis und `request_started`. Fehlende optionale Fakten bleiben unbekannt. `request_started:true` belegt die Einleitung eines Requests; es belegt keine Zustellung beim Zielserver. Die Fixtures prüfen Zustellung zusätzlich über ihre eigenen tatsächlichen Requestlogs.

Robots- und Sitemap-Protokolltransfers tragen derzeit keine Ressourcen-Grants. Ihre vollständige Rollen-/Redirect-Abnahme ist eine eigene offene Grenze; ihnen werden keine künstlichen InitialInput-Grants zugeschrieben. Rendering zusammen mit Ressourcen-Erkennung wird weiterhin vor HTTP abgelehnt, bis jeder Browser-Subrequest einen eigenen durchgängigen Regelvertrag besitzt.

## Gepinnte Leser und Exporte

Die Ressourcen-API zeigt `configured_semantic_contract` und `semantic_contract_binding`. Ohne gültige native Daten lautet die Bindung `declared_config_only` oder `not_configured`. Erst nach vollständiger Prüfung von gespeicherter Konfiguration, Generation und Ledger gilt `validated_full_config_and_generation`.

Bei V2 enthält `snapshot.resource_contract_engine` die Engine aus derselben geprüften SQLite-Lesetransaktion. Die API prüft sie zusätzlich gegen die Job-Konfiguration. API-/CLI-Voll-JSON und beide MCP-Transporte erhalten dieselben gespeicherten Fakten. HTTP-JSON-/CSV-Exporte erhalten bei V2 den Header `x-resource-contract-engine`; unveränderte V1-Exporte erhalten ihn nicht. CSV behält die vollständigen Zeilenfakten in `complete_observation_json`.

Ausgewählter Recrawl bleibt auf die ausgewählten Ziele begrenzt. Er darf aus neuen Quellvorkommen keine zusätzliche Ressourcen-/Navigationsexpansion starten. Wird ein vorher gespeichertes Ziel bei einem neuen Redirect nur durch eine Rolle ohne Speichererlaubnis erreicht, bleibt der ausdrücklich ausgewählte Versuch als begrenzte private Metadatenzeile sichtbar. `selected_materialization_reason:"selected_scope_limited_projection"` kennzeichnet diese Aufnahme; Body, Volltext und Capture-Daten werden daraus nicht erfunden. Die aktuelle Berichtaufnahme und die ursprünglichen privaten Transportfakten bleiben getrennt. Fehlende geerbte native Zeilen werden ausdrücklich als unvollständige Historie ausgewiesen. Vollständige Screaming-Frog-, Filter-, Workflow-, Encoding- und Browserparität bleibt **NOT_PROVEN**.
