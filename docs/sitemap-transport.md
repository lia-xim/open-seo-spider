# B12: Sitemap-Transport

Sitemap-Dokumente werden jetzt mit einem eigenen Fetch-Zweck im bestehenden Rust-Netzwerkpfad abgerufen. Aktivierte Ressourcenanalyse verlangt dafür keine Seiten-Ressourcenfreigabe. Eine Ressourcen-Origin-Freigabe erweitert zugleich keine Sitemap-Redirect-Berechtigung. Gewöhnliche Seiten behalten ihren bisherigen Pfad.

Der gemeinsame Request-Loop prüft weiterhin Scope, Policy-Rolle, Robots, Rate, Timeout sowie Redirect-Schleifen und -Grenzen. Zugangsdaten bleiben an den vorgesehenen Seed-Origin gebunden. Rollenfehler werden vor Robots- und HTTP-Anfragen abgewiesen. Die zwölf HTTP-basierten Core-Tests prüfen diese Grenzen mit tatsächlichen lokalen Requests.

Nach dem HTTP-Content-Decoding prüft die Sitemap-Verarbeitung die tatsächlichen Entitätsbytes, erkennt optionale Gzip-Dateikompression per Magic-Bytes und ruft den XML-Beobachtungsparser einmal auf. MIME-Typ und Dateiendung wählen keinen Decoder. PDF-, Bild-, Charset-, HTML-, Inhalts- und Browseranalyse werden für dieses Dokument nicht ausgeführt. Transport-, Inflation- und XML-Grenzen bleiben getrennte Zustände; exaktes Limit braucht echtes EOF oder einen zusätzlichen Sentinel.

Die interne Beobachtung enthält qualifizierte Request-/Final-URL, Status, Robots-/Redirect-Zustand, vollständige Transportbytes mit SHA, Decoder-Fakten sowie vollständige oder unvollständige Parser-Fakten. URL-Kontext über8192 UTF-8-Bytes wird als fehlender Kontext mit Fehler geführt. Dieser interne Zustand ist noch kein dauerhaftes SQLite-Ledger.

## Tatsächliche Prüfung

Der [Transport-Checkpoint](verification/sitemap-transport-checkpoint.json) bindet die integrierten Quellen, Cargo-Logs und Native-Fixtures. Zwölf gezielte HTTP-Tests und alle329 Core-Unit-Tests bestehen; Alltargets-Clippy und der tatsächliche Debug-App-Build bestehen ebenfalls. Diese Testzahlen beschreiben unterschiedliche Läufe und werden nicht addiert.

Die vollständige App wurde in16 isolierten Fällen über ihre tatsächliche API und den anschließenden Zielabruf geprüft: Ordinary/Policy mit Ressourcen an/aus, einfache XML- und Gzip-Dateien, irreführende `.gz`-Dateinamen, echtes HTTP-Gzip, beschädigte Gzip-Footer und gleichnamige fremde XML-Erweiterungen. Eine eingefrorene B11-Baseline hatte neun passende Kontrollen und sieben reproduzierte Fehlerfälle; die neue Debug-App besteht alle16 Fälle. Die ursprüngliche lokale Beobachtung wurde vor dem abschließenden Prozessstopp aufgezeichnet. Die dauerhafte CI-Fixture prüft Request-Zähler zusätzlich nach dem Stopp und dem Leeren des eigenen Fixture-Servers.

Reproduktion mit einem gebauten App-Binary:

```text
cargo test -p open-seo-spider-core --lib --locked crawler::sitemap_protocol_tests::
cargo clippy -p open-seo-spider-core --all-targets --locked -- -D warnings
cargo build -p open-seo-spider --locked
# SPIDER_BINARY auf das gebaute Binary setzen:
node scripts/sitemap-transport-e2e.mjs
```

## Weiter offen

Die Discovery-Aufrufer verwenden weiterhin eine reduzierte Kompatibilitätsprojektion. Sie erhält keine vollständige Zuordnung aller originalen XML-Vorkommen zu gespeicherten Admission-Entscheidungen. Vollständige Rohwerte und Originalordinalzahlen müssen direkt aus dem typisierten Parser in atomare Dokument-/Entry-/Loc-/Root-Zeilen übernommen werden. Task-Deduplizierung und einmalige Completion benötigen getrennte Nachweise.

Quell-Kohorten, Fehler-/Budgetzustände, Checkpoint/Recovery, unbekannte Altbestände, ausgewählte Vererbung sowie API-Cursor, CSV und Browseransicht bleiben offen. Ein leerer Discovery-Frontier beweist keine vollständige Mitgliedschaft. `sitemap.discovery` und `sitemap.coverage` bleiben deshalb teilweise geprüft. Vollständige Hersteller-Parität bleibt **NOT_PROVEN**. Die neueste Transport-CI, optimierter Release und Installation werden getrennt protokolliert; der installierte Nutzer-Release bleibt zunächst B11.
