# S4: konkreter CSV-Streaming-Integrationsentwurf

Historischer Entwurf vor der Implementierung. Der aktuelle
[Quellcodekandidat mit geschriebenen Tests](../sitemap-csv-streaming.md)
setzt diesen Plan teilweise um. Die folgenden DESIGN_ONLY-/PLANNED-Angaben
beschreiben den ursprünglichen Entwurfsstand, keine aktuelle Abnahme.

Status des ursprünglichen Entwurfs: DESIGN_ONLY. Damals kein Produktcode, keine Implementierung oder Laufzeitverifikation; alle unten benannten Regressionen waren PLANNED / NOT_AUTHORED / NOT_RUN. Basis waren ausschließlich die in source-pins.json festgehaltenen tatsächlichen Root-Dateien. Der aktuelle Implementierungs- und Testquellstand ist im oben verlinkten CSV-Vertrag getrennt dokumentiert.

## 1. Aktueller Pfad und erhaltene Verträge

`crates/app/src/server/sitemaps.rs:96-138` liest derzeit über `sitemap_read_snapshot_with_membership_analysis(Default::default())` den vollständigen Snapshot; `membership::csv` erzeugt danach ein komplettes Vec und Body::from(bytes). Die Analyse wird nur für ihren vollständigen Kontext gebunden; CSV enthält die ursprünglichen typisierten Rohzeilen und keine Lastmod- oder Seitenanalyse-Zusatzspalten.

Unverändert bleiben die zehn Spalten in `sitemap_membership.rs:922-939`, `csv_record` einschließlich Anführungszeichenverdopplung und CRLF, `write_csv_row` einschließlich typed_row_json, `csv_text`-Formelneutralisierung und unveränderte numerische sequence/source_ordinal-Zellen. Diese Funktionskörper werden geteilt oder lediglich zugänglich gemacht, nicht durch eine neue CSV-Bibliothek ersetzt. Kein BOM, kein Plattform-Zeilenumbruch, keine Nachnormalisierung, keine neue Spalte.

Unverändert bleiben der vorhandene Full-export-Token, kanonische Filter-Serialisierung/Hash, ursprüngliche Mitgliedschaftsidentität, Dateiname, Content-Type und die bestehenden x-sitemap-/x-crawl-Header. Ein frischer Inventar-Snapshot muss ohne neues Tokenformat exportierbar bleiben. Der Streamingpfad verwendet weiterhin SITEMAP_READ_WINDOW_ENGINE und exakt sitemap_read_state_sha256; ein eigener Exportengine darf den Snapshot-State nicht verändern.

Die private SQL-Visitor-Funktion ist bereits vorhanden: `sitemap_ledger/verified_read.rs:104-142` verarbeitet alle passenden gespeicherten Zeilen in sequence-Reihenfolge mit limit=-1. Sie verwendet keine erste Inventarseite, keinen nachgebauten Teilkatalog und keinen 8-MiB-Window-Loop. Die dortige vollständige Raw-Verifikation sowie Source-, Coverage- und Analysis-Kontextguards bleiben erforderlich, auch wenn der angefragte Filter die beschädigte Zeile ausschließen würde.

## 2. Kleinster öffentlicher Core-Vertrag

Vorschlag für einen gemeinsamen privaten Vorbereitungsbaustein der bestehenden Rich-Fassade und der neuen synchronen Callback-Fassade. Der Baustein führt exakt einmal die aktuelle Vorbereitung aus: tatsächlicher Store-Mutex + eine tatsächliche read-only Connection + eine tatsächliche Transaction; Metadata-Inputpreflights; SavedConfig/import/policy-Guard; tatsächlicher RunPlan; für Selected BEIDE alten höheren Raw-/Source-Presence- und exakten Parent-Guards; verify_raw; verify_source mit derselben Connection; kompakte Coverage; einmalige prepare_window_analysis; identischer vollständiger shared State.

Öffentliche konzeptionelle Signatur, noch kein kompiliertes API-Versprechen:

```rust
pub struct SitemapStoredRowsFilter {
    pub kind: SitemapAnalysisRowKind,
    pub source_observation_id: Option<String>,
    pub membership_url: Option<String>,
}

pub struct SitemapVerifiedReadHeader { /* echte kompakte Felder, siehe unten */ }
pub struct SitemapVerifiedStoredRows<'read> { /* ausschließlich private Felder */ }

impl CrawlStore {
    pub fn with_verified_sitemap_stored_rows<T>(
        &self,
        filter: &SitemapStoredRowsFilter,
        expected: Option<&SitemapReadWindowExpectedContext>,
        control: &SitemapReadControl,
        callback: impl for<'read> FnOnce(&SitemapVerifiedStoredRows<'read>)
            -> anyhow::Result<T>,
    ) -> anyhow::Result<T>;
}
impl SitemapVerifiedStoredRows<'_> {
    pub fn header(&self) -> &SitemapVerifiedReadHeader;
    pub fn visit_all_filtered(
        &self,
        visitor: impl FnMut(&SitemapLedgerRow) -> anyhow::Result<()>,
    ) -> anyhow::Result<()>;
}
```

Header: crawl_config_sha256, generation, ledger summary, inherited basis, coverage, optionale coverage_inventory_sha256, optionale SourceSummary, verifizierter SourceContext, tatsächlicher vorbereiteter AnalysisContext, reader_engine, optionale catalog_state_sha256 und materializing_selected_compatibility. Keine rows-/has_more-Ersatzfelder; Header ist ein DTO, kein Beweisconstructor. Kein öffentlicher Session-Constructor, Clone/Deserialize, SQL-Connectionzugriff oder beliebig eingesetzter Snapshot. Die genaue interne Lebenszeitanzahl ist Implementierungsdetail; HRTB/owned T müssen verhindern, dass ein SQL-Statement, Zeilenborrow oder Session-Beweis aus dem Callback entkommt.

Filter wird vor I/O validiert und in der Session festgehalten; wiederholte visits können ihn nicht ändern. Exakt bestehende All/Root/Document/Entry/Location-Semantik: Document matched seine observation_id, Entry/Location ihre source observation_id, Root seine optionale source observation_id. membership_url matched ausschließlich Root/Location admission.membership_url; kein Alias, keine final_url, keine kanonische URL und kein Substring. All stored bedeutet alle *gespeicherten* passenden Zeilen, keine Behauptung vollständiger entfernter Sitemap-Erfassung.

Vollständiger Expected-Vergleich geschieht nach Contextvorbereitung und vor Callback/Visitor/CSV-Bytezählung: Generation, aktueller SavedConfigSHA, initialer CatalogConfigSHA, Revision, RollingSHA, vollständiger shared State und inherited basis. Filter/JobUUID/Full-export-Rolle verifiziert die App vorher. Source-/Marker-/Report-/Inventory-/Pending-only-Änderungen müssen deshalb denselben Token wie im Inventar stalen. Der Header darf None für echte unbekannte Geschichte enthalten; die App verweigert CSV ohne tatsächlichen Raw-Katalog mit bestehendem HTTP409. Known-empty Some kann allein den unveränderten CSV-Header liefern.

Der Callback darf zwei Read-only-Visits durchführen. Core verifiziert nicht pro Zeile, pro Window oder pro Pass den ganzen Store erneut. Einzelzeilen-Decodierung und SQL-Spaltenabgleich bleiben erhalten. Kein zweites Öffnen des Stores, kein erneuter Rich-Window-Read zwischen Pass1 und Pass2, kein Readzeit-Schreiben. Der Callback darf den gleichen Store nicht reentern. Nach seinem Ergebnis werden alle Proofs/Statements verworfen und die Transaction abgeschlossen; erst danach darf die Methode das owned Completion-Ergebnis zurückgeben.

Core kennt weder CSV, Bytes, HTTP, Axum noch Content-Length. App übernimmt Formatter, Cap, SHA und Transport. Öffentliche Filter-/Header-/Opaque-Session-/Control-Typen werden normal über Core lib.rs exportiert. Das ist ein echter zukünftiger Integrationspunkt, keine public gemachte private Fake-Header-Funktion.

Selected bleibt ausdrücklich materialisierende Kompatibilität. Sowohl parentSome/currentNone als auch umgekehrte Presence und veränderte exakte Parent-BLOBs bleiben Fehler. Der Streamingtransport macht diese Parent-/RunPlan-Kosten nicht zu einem vollständig streamingfähigen Selected-Reader.

## 3. Zwei identische Formatterpässe in einer TX

App-pipeline läuft in EINEM spawn_blocking-Worker mit eigenem `CrawlStore::open_read_only(path)`; niemals auf der Connection des Crawl-Writers.

1. Query/Full-export-Rolle wie bisher vor dem Jobzugriff validieren; Job404-Priorität erhalten. Danach unter einem App-Job-Readguard UUID + exakten FilterSHA gegen den Token prüfen und Appgeneration/configSHA erfassen. DB fehlt: bisheriges catalog_unavailable409.
2. Worker startet den tatsächlichen Core-Callback. Actual Header gegen Appgeneration/configSHA prüfen; tatsächlichen Snapshot aus Header + JobUUID bilden und Token vollständig nachvalidieren. Header/Token/Katalogfehler erzeugen keinerlei CSV-Output.
3. Pass1: `write_csv_header` und dann *alle* visit_all_filtered-Zeilen durch denselben `write_csv_row` in einen CountingShaWriter schreiben. Er checked-addiert die tatsächlichen geschriebenen Bytes und hashed dieselben Bytes inkrementell. Keine CSV-Vec oder Spooldatei; keine zweite komplette SHA-Iteration. Einschließlich Header, Escaping, CRLF und typed_row_json gilt `length <= 67_108_864` inklusiv. Das erste zusätzliche Byte verweigert atomar mit bestehendem sitemap_csv_byte_budget413 VOR Ready/HTTP200.
4. Erst nach erfolgreichem vollständigem Pass1 und erneuter Control-Prüfung sendet der Worker Ready {Snapshot, exakt length, sha256, matched_row_count}. Handler wartet nur auf Ready, nicht auf das Ende des noch blockierbaren Produzenten. Job-Readguard danach freigeben; er darf nicht für die Dauer eines langsamen Downloads gehalten werden. Eine spätere Crawlmutation beeinflusst die gepinnte SQL-TX nicht.
5. Handler setzt bestehende Header + Content-Length=exakte Länge. Optional additiver `x-sitemap-csv-sha256`-Header mit Pass1SHA; kein veränderter Snapshot-State. Er baut Body::from_stream auf einem begrenzten eigenen Stream.
6. Pass2: gleicher Header und gleicher Session-Visitor/Filter, gleiche Formatterfunktionen in einen erneut zählenden/hashenden ChunkTailWriter. Vergleich von tatsächlicher Länge, SHA und Zeilenzahl mit Pass1. Ein fehlender/zusätzlicher oder anders formatierter Datensatz bleibt ein Fehler, selbst wenn die Länge zufällig gleich wäre. Zweitpass darf den bereits geprüften Cap nicht lockern.
7. Writer gibt den letzten positiven Datenblock NICHT an den Body aus, sondern als Teil des owned Callback-Ergebnisses zurück. Core schließt seine TX ab. Erst nach erfolgreichem Return, vollständigem Producer-Join und erneut geprüftem Completion-Receipt darf der Body diesen Block ausgeben.

Wichtig: ready ist ein interner Vorabnachweis, kein HTTP-Erfolg des fertigen Downloads. Netzwerkabbruch ist nach HTTP200 nicht zu JSON500 umdeutbar. Fehler nach Ready müssen den Body als Fehler/verkürzte Content-Length beenden. Niemals catch-error -> normales EOF, leere Erfolgsdatei, Reportfallback oder bloßes Logging. Auch ein Transaktionsabschlussfehler nach dem zweiten Pass darf keine vollständig gelieferte Datei erzeugen.

## 4. Tatsächlich begrenzter Rückstau und terminaler Block

Konkrete Startwerte: feste 64-KiB-Chunks, bounded mpsc(capacity=4), ein zurückgehaltener Tail von 1..64KiB. Der CSV-Header ist positiv lang, somit existiert auch bei 0 Treffern ein Tail. ChunkTailWriter sammelt kleine Formatterwrites; er sendet einen vollen Tail erst, wenn mindestens ein weiteres Byte existiert. flush darf den letzten positiven Tail nicht freigeben. Keine rowgroßen Bytes::copy_from_slice-Nachrichten und kein unbounded_channel. Größenbound der reinen Transportpuffer: 4 Queuedelemente + ein Tail + ein gerade zu sendender Chunk, jeweils höchstens64KiB; Formatter-Einzelzeilen- und verifizierte Header-/Proofkosten kommen ausdrücklich hinzu. Kein daraus abgeleiteter pauschaler RSS-Beweis.

Ein Coordinator besitzt und awaitet den spawn_blocking JoinHandle immer. Der Body besitzt Receiver, Cancel-on-drop-Guard und CompletionReceiver. Coordinator erfasst Workerpanic/JoinError als Fehler und übermittelt das owned Ergebnis erst nach tatsächlichem Join. Er darf vorzeitig nicht erfolgreiches EOF signalisieren.

Body-Zustände: Prefix -> AwaitJoinedCompletion -> FinalTail -> EOF oder Error. Bei CompletionSuccess vor Queueende Tail nur merken, vorhandene Prefixchunks geordnet ausgeben und erst danach Tail. Bei CompletionError keine weiteren Queuebytes als Erfolg ausgeben: Receiver schließen, höchstens die begrenzte Queue verwerfen, einmal Bodyfehler. Ein Content-Length-großer Prefix darf konstruktiv unmöglich sein; Tailfreigabe ist die einzige Stelle, die das letzte Byte ausliefern kann.

Vor Ready fehlgeschlagene ResponseBuilder-/Headerkonstruktion, abgebrochener Handler oder Timeout müssen ebenfalls Receiver schließen/canceln; kein Worker darf auf eine nie gepollte Bodyqueue warten. Auf Clientdisconnect schließt der Body-Drop synchron Receiver und signalisiert Cancellation; der Coordinator joinet anschließend unabhängig davon den Worker. Keine Handle::abort-Behauptung für spawn_blocking: ein bereits gestarteter blockierender Worker wird dadurch nicht beendet.

Die existierenden server/policy.rs und resources.rs sind nur Axum/mpsc-Vorbilder. Die Policyroute sendet Ready bei der ersten Seite und hält keinen JoinHandle im Body; diese Lebenszyklusmechanik erfüllt die obigen neuen Guarantees nicht und wird nicht blind kopiert.

## 5. Cancellation, Zeitgrenzen und Drain

Vorgeschlagene konkrete Servicegrenzen: Vorbereitungsdeadline120s ab Start; nach Ready absolute Streamingdeadline300s; höchstens30s ohne erfolgreiche Queueübergabe. Diese Werte sind neue betriebliche Grenzen, keine beobachteten Laufzeiten. Sie sind Konstanten/konfigurierbare interne Limits, keine versteckte Absenkung des64MiB-Caps.

Ein unabhängiger Watchdog muss auch wirken, wenn der HTTP-Body wegen eines langsamen Clients nicht gepollt wird. Ein bloßes Timeout *innerhalb* poll_next genügt nicht. Der bounded Sendweg benutzt einen abbrechbaren, deadlinebeachtenden Reserve/Send-Wait (beispielsweise RuntimeHandle::block_on(select! ...) ausschließlich im Blockingworker); kein unendlich blockierendes blocking_send ohne Cancel-/Deadlineweg. Geschlossener Receiver hat Vorrang und liefert BrokenPipe/Cancelled.

SitemapReadControl ist transportneutral: geteiltes Cancel-Flag + monotone Deadline und checkpoint()->Result. Core muss es vor teuren Stufen, zwischen tatsächlich gelesenen/verifizierten Raw-/Source-/Report-/Coverage-Zeilen und vor beiden visits/SQL-Schritten beachten. Bestehende Kontrollvarianten können ihre alte API mit never-cancel-Default behalten. Ohne diese Checks in den tatsächlichen Schleifen ist eine Behauptung begrenzter Abbruchlatenz nicht erfüllt. Die momentanen privaten verifier akzeptieren noch keinen solchen Controlparameter; dies ist ein erforderlicher Implementierungsschritt, kein bereits vorhandenes Feature.

SQLite wartet aktuell bis5s busy_timeout; reine Rust-Deserialisierung eines begrenzten Configs oder Selected-Parent-BLOBs ist ebenfalls nicht per tokio::abort hart unterbrechbar. Ein optionaler Core-intern gehaltener tatsächlicher SQLite-InterruptHandle kann lange SQL-Schritte abbrechen; er ersetzt keine Rust-Schleifenchecks. API-/Featureverfügbarkeit dafür bei Implementierung lokal verifizieren. Keine harte Echtzeit- oder maximal5s-Drainzusage aus den vorhandenen Dateien ableiten. Selected-Materialisierung bleibt besonders klar als grober kooperativer Abbruchpunkt dokumentiert.

Vor Ready Timeout: neue explizite JSON504 sitemap_export_timeout (nach unveränderter Query-/Job-Priorität), sofort Cancel; keine HTTP200. Nach Ready Timeout: Body-I/O-Fehler, keine Statusumschreibung. Coordinator behält den JoinHandle bis tatsächlichem Ende und verwirft restliche begrenzte Nachrichten. Requestende ist nicht dasselbe wie Workerende. Ein pro App besessener Exporttask-Tracker registriert diese Coordinators; Server-Shutdown cancelt und drain-joinet sie, statt sie zu detachen. Der aktuelle graceful_shutdown in server.rs:305-309 besitzt dieses Exporttask-Drain noch nicht. Falls eine Shutdown-Wartefrist gewählt wird, müssen verbleibende Worker ausdrücklich als nicht beendet gemeldet werden; kein angeblich erfolgreicher Drain nach bloßem Timeout.

Der Formatter wandelt aktuell JEDE std::io::Error in sitemap_csv_byte_budget um (`write_csv_fields:917-919`). Seine Byteerzeugung bleibt gleich, aber der transportierende Writer muss die erste konkrete Sinkursache separat behalten: BudgetExceeded, ClientDisconnected, Cancelled, Timeout, Transport. Das umhüllte Formatter-Failure darf keinen Clientdisconnect in413 und keinen Timeout in normale EOF umetikettieren. AppFailure ist derzeit kein std::error::Error: ein kleiner typed App-Adapter erhält Failure-Code/Message beim Durchreichen durch anyhow im Corevisitor, anstatt nur Stringfehler zu erfinden.

## 6. Status- und Tokenkompatibilität

| Fall vor Ready | Ergebnis |
| --- | --- |
| Ungültige Query/Filter/Rolle | vorhandenes400 |
| Unbekannter Job | vorhandenes404; Lookuppriorität wie bisher |
| Andere JobUUID/Filter/State, alter parsebarer Token | vorhandenes409 stale_sitemap_cursor |
| Appgeneration/config ungleich tatsächlicher Header | vorhandenes409 sitemap_job_snapshot_conflict |
| Echte RawNone/DB fehlt | vorhandenes409 catalog_unavailable |
| CSV tatsächlich >64MiB | vorhandenes413 sitemap_csv_byte_budget |
| Verifizierungs-/SQL-/Report-/Tailkorruption | vorhandenes500 read_failure, kein CSVprefix |
| Neue Vorbereitungsdeadline | explizites neues504 sitemap_export_timeout |

Core Expected-vor-Callback kann bei gleichzeitig veraltetem Token und Appjob/DB-Abweichung die bereits für Rich-Inventar dokumentierte409-Codepriorität stale_sitemap_cursor vor sitemap_job_snapshot_conflict übernehmen; HTTP409 bleibt gleich. Diese begrenzte Prioritätsabweichung muss ein Implementierungs-Handoff explizit nennen und testen. Ein zweiter SQL-Read zur künstlichen Rückkehr zur alten Priorität ist ausgeschlossen. Corruption hat weiterhin echte Readfehlerevidenz und darf nicht als stale verborgen werden.

Der eigentliche CSVformatter ist unabhängig von lastmod/page/requested-URL-Analyse; nur deren vollständiger gültiger Kontext bleibt Teil des shared State. Default(row_limit=0,page_limit=0)-Kontextvorbereitung darf deshalb keine first200-Samplingvariante oder einen vom Inventar abweichenden AnalysisContext erfinden.

## 7. Geplante Tests mit tatsächlichen Produzenten

Alle Namen unten sind neu vorzuschlagende Tests, nicht bereits authored/executed. Bestehende Formatter-Kleinkontrollen bleiben nützlich, beweisen aber weder den echten64MiB-Cap noch Wireverhalten. Positive gespeicherte Fixtures ausschließlich über tatsächliche Parser/Crawler/Store-APIs; SQL ist nur klar beschriftete Korruptions-/Concurrency-Störung, kein erfundener gültiger Header/Window/Report.

1. `actual_csv_stream_matches_buffered_oracle_all_stored_exact_filters`: tatsächliche Storefixture mit >200 Rohzeilen und mehreren Quellen/ursprünglichen Mitgliedschaften; alle Rowkinds/source/member-Kombinationen, leere Treffer, Aliasnegativkontrolle. Bytevergleich mit bisherigem gepuffertem csv-Oracle, identische typisierte Zeilen, Reihenfolge, Unicode, Komma, Quotes, CRLF, Formelpräfix. Keine JSONWindow-8MiB-Grenze für allstored einsetzen.
2. `actual_inventory_snapshot_token_opens_same_tx_two_pass_csv`: frischer Inventartoken über realen Apphandler, anschließend neuer Export; HeaderState identisch. Instrumentierter realer Reader belegt eine Vorbereitung/Verifikation und zwei vollständige Visits, keine zweite Connection/Snapshot, keine frei eingesetzten Sessionliterals.
3. `actual_csv_stream_rejects_full_tail_source_report_and_inventory_corruption_before_ready`: separate tatsächliche gültige Storefixtures, dann beschriftete Mutation am ungefilterten Tail, A-Record/Link, aktuellen gespeicherten Report/Context oder Inventory; HTTPnicht200, kein Ready-/CSVbyte. Nicht nur die erste passende Zeile testen.
4. `actual_csv_stream_stales_a_marker_pending_report_and_selected_context`: tatsächliche vorhandene A/B/Marker/Pending/Terminalreportfixtures übernehmen. Alte Tokens nach jeder isolierten gültigen Kontextänderung409; nicht bloß RawRevision ändern. Corruptionvariante getrennt500 statt stale.
5. `actual_csv_stream_selected_presence_and_exact_parent_guards`: echte initialize_selected_recrawl für parentSome und importiertes historicalNone; beide fehlenden Presencekombinationen und exakte Parent-BLOB-Störung negativ. HealthyNone409unavailable, knownemptyheaderpositiv, Selectedflag ausdrücklichmaterializing.
6. `actual_csv_stream_pins_original_tx_while_later_writer_changes_generation`: echte zweiter Store-WAL-Writer und Barrier nach Pass1Ready, vor Pass2; Writer committet tatsächliche neue Generation/Daten. Pass2 bleibt ursprünglicher kompletter CSVbytes/SHA/length; neuer Inventar-/Exportrequest sieht neue Frische. Keine SQLsameconnection-Mutation zum Umgehen der Sessiongrenze.
7. `actual_native_sixty_four_mib_csv_exact_cap_and_plus_one_before_http200`: tatsächlicher gültiger Store mit offizieller32MiBCataloggrenze/realen Rowcaps, Quoteamplifikation aus realen Ursprungsdaten. Zwei unabhängige Stellgrößen für kanonische Rawtexte mit Formatter-Inkrementen gcd=1 erlauben ein exakt67_108_864-Byte-CSV und ein exakt+1-Byte-CSV. Auf tatsächlichen writer-/parser-validen Erzeugerpfad aufbauen und dessen Zulässigkeit prüfen, keine Root/SQL-Literals als positiver Ersatz. Paarweise kalibrieren, endgültige Bytes mit unverändertem gepuffertem Oracle belegen, anschließend echte Streamingroute: exaktCap200+ContentLength+SHA+vollständig; +1413ohneBodyprefix. Keine abgesenkten Testcaps und kein riesiger Generatorlauf während HOLD. Wenn reale Erzeugerrestriktionen dieses Paar verhindern, Gate offen berichten und Fixture neu planen, nicht Capbeweis behaupten.
8. `actual_http_csv_large_body_uses_bounded_chunks_and_backpressure`: echtes Loopback-Axum + raw TCP/HTTP-Client, echte große Storefixture. Client liest zunächst nicht; instrumentierter echter Chunkwriter belegt maximal4 queued64KiB-Chunks plus feste lokale Puffer und ausbleibenden unbegrenzten Readfortschritt. Dann normal weiterlesen, vollständigen ByteSHA/Len gegen Oracle prüfen; weder kleiner Body noch nur mpsc-Unitprobe reicht.
9. `actual_http_csv_client_disconnect_cancels_and_joins_producer`: realer TCP-Disconnect nach Headers+Prefix bei voller Queue; tatsächlicher Blockingworker endet, Coordinatorjoint, aktive Reader/Tasktracker leer und readTXgeschlossen; DB-Dateien unverändert, Folgeexport funktioniert.
10. `actual_http_csv_idle_and_absolute_deadline_close_body_and_drain`: echte Netzwerk-/Producer-Barrieren mit injizierbarer Uhr/Deadline, unveränderter Daten64MiB-Cap. Idleclient ohne Bodypoll und arbeitender/langsamer Producer getrennt; vorReady504 und nachReadyBodyfehler; kein ignorierter blocking_send und kein detach. Tatsächliche Task-/TX-Freigabe belegen.
11. `actual_http_csv_tail_is_withheld_until_worker_join_and_tx_close`: realer Writer/Store mit Barrier unmittelbar nach letztem Formatterbyte, aber vor erfolgreichem CoreReturn; Client erhält höchstensL-1 Bytes. Successfreigabe liefert exaktRest. Kontrollierter Fehler/Panic/Digestmismatch an dieser Grenze bewirkt vorenthaltenenTail + Wirebodyfehler, niemals erfolgreiche vollständigeContentLength. Defektinjektion nur am Lifecyclehook; gültige Zeilen kommen vom echten Producer.
12. `actual_csv_stream_second_pass_failure_and_response_build_failure_drain`: typisierte Writerfehlerinjektion und Readyempfänger-/Responsebuilderdrop auf tatsächlicher Route; keine Budgetfehlerumetikettierung von Disconnect/Timeout, no successful EOF und alle Tasks gejoint.
13. `actual_csv_stream_shutdown_cancels_and_drains_active_exports`: echter Server mit mindestens einem backpressured Export, kontrollierter Shutdown; Registry bestätigt Cancel+Join, keine zugedeckte Resttask. Ausnahme einer absichtlich blockierten Testkontrolle muss Drainunvollständigkeit ehrlich anzeigen.
14. `actual_csv_stream_error_priority_preserves_existing_statuses_and_token_roles`: tatsächlicher fehlender Job, fremderJobtoken, geänderter Filter, full/page/rawcontinuation role, alteEngineToken, Appjob/DB-Mismatch und Doppelkonflikt. Exakte bestehenden Codes/HTTPstatus gemäßTabelle plus explizit dokumentierter409Prioritätsfall.

## 8. Umsetzungsscheiben und Abnahmegrenzen

A. Core: gemeinsamer vorbereiteter Sessionbaustein, publicOpaqueCallback/Header/Filter und kooperative Controlhooks, ohne CSV-/HTTP-Code; existingRichWindow muss byteidentischen State behalten. Tatsächliche Coreoracle-/selected-/cancellationtests.
B. App: unveränderte Formatter teilen; typedCountingShaWriter + boundedChunkTailWriter; Token/Headerbinder gegen echten Coreheader; kleiner Callbackfehleradapter. Gepuffertes csv nur als cfg(test)-Oracle erhalten, keine Produkt-Reportfallbackroute.
C. HTTP: dedizierte Ready/Body/Coordinatorzustände statt read_context bisWorkerende zu awaiten; boundedTransport und unabhängiger Watchdog; trackedJoin/ShutdownDrain. Bestehendes read_context ist für abgeschlossene owned Ergebnisse gebaut und kann nicht einfach unverändert einen Worker awaiten, der auf einen noch nicht ausgelieferten Body wartet.
D. Nach aufgehobenem HOLD erst tatsächliche Compiler-/Unit-/SQLite-/Loopback-Wiregates ausführen und separat ausweisen. Statische API- und Formattergleichheit ist kein positiver Transport- oder64MiB-Beweis. Dieser Entwurf beendet keines dieser Gates und ändert keine Featureparitätsbehauptung.
