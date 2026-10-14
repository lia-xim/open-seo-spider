const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {execFileSync}=require('node:child_process'),{createHash}=require('node:crypto');const sha=b=>createHash('sha256').update(b).digest('hex');
const repo='E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-findings-core-integration-v1';
const gh='C:/Program Files/GitHub CLI/gh.exe';
const git=args=>execFileSync('git',args,{cwd:repo}).toString().trim();
const branch='codex/sitemap-findings-core-20261015',head=git(['rev-parse','HEAD']);
assert.equal(git(['branch','--show-current']),branch);assert.notEqual(head,'c8f539be979ac5436dcc962c5ce7677aab9f3b0d');
assert.equal(git(['status','--porcelain=v1','-uall']),'');assert(git(['ls-remote','origin','refs/heads/'+branch]).startsWith(head+'\t'));
const privacy=JSON.parse(execFileSync(gh,['repo','view','lia-xim/open-seo-spider','--json','isPrivate,nameWithOwner']));assert.equal(privacy.isPrivate,true);assert.equal(privacy.nameWithOwner,'lia-xim/open-seo-spider');
const oldRaw=fs.readFileSync(path.join(__dirname,'sitemap-issue-11-csv-streaming-v1.md'));
assert.equal(sha(oldRaw),'15d094382b57291a598328dfc820948f3ecc1e067a8be1b18fa438d964838189');
const before=JSON.parse(execFileSync(gh,['issue','view','11','--repo','lia-xim/open-seo-spider','--json','body,url,updatedAt']));
assert.equal(before.body.trim(),oldRaw.toString().trim(),'Preserve any concurrent issue edits');
const old=oldRaw.toString();const historyStart=old.indexOf('### Neuer S4-Quellschritt:');assert(historyStart>0);
let history=old.slice(historyStart).replace('### Neuer S4-Quellschritt: vollständiger gespeicherter CSV-Bestand mit begrenztem Transport','### Vorheriger S4-CSV-Stand (c8f539b; separate ungetestete Quelle265)');
history=history.replace('Gemeinsamen serverseitigen Abfrage-/Exportkern mit getrennten SavedPage-/SourceAttempt-Domänen und API-/CLI-/MCP-/Browserflächen implementieren und abnehmen.','Gemeinsame API-/CLI-/MCP-/Browserflächen und getrennten vollständigen Finding-Export an den jetzt als Quelle implementierten SavedPage-/SourceAttempt-Core anbinden und abnehmen.');
history=history.replace('ist ein konkreter Entwurf, keine Implementierung.','bleibt der übergreifende Vertrag; der neue Core ist implementierter, noch unkompilierter Quellcode.');
history=history.replace('blob/codex/sitemap-csv-streaming-20261014/docs/FEATURE_TODO.md','blob/'+branch+'/docs/FEATURE_TODO.md');
const header=`Der aktuelle Entwicklungskandidat liegt im privaten Zweig \`${branch}\`, aufbauend auf dem separat erhaltenen CSV-Kandidaten \`c8f539b\`.

Der Gesamtkatalog bleibt bei **31 eigenen Abnahmen erfüllt / 67 teilweise geprüft / 46 offen**. Das ist kein Hersteller-Abdeckungsgrad. Vollständige Screaming-Frog-Parität bleibt **NOT_PROVEN**.

Quellcommit: [\`${head.slice(0,7)}\`](https://github.com/lia-xim/open-seo-spider/commit/${head}), ausdrücklich WIP ohne Kompilierung/Testabnahme oder neue Installation. Frühere Quellen und Belege bleiben erhalten.

### Neuer S6-Core-Quellschritt: sieben vollständige gespeicherte Finding-Prädikate

- [x] Die sieben benannten Filter als gemeinsame unabhängige Core-Prädikate implementiert: ursprüngliche Mitgliedschaft, qualifizierte Nichtmitgliedschaft, beobachtete Orphan-Kandidaten, qualifizierte Nichtindexierbarkeit, mehrere ursprüngliche XML-Quellen und zwei gemessene Protokollgrenzen. Match / NotMatched / Unknown bleiben getrennt.
- [x] Tatsächliche vollständige SavedPage-/SourceAttempt-Leser angebunden. Originale statt Aliase, alle beobachteten Nicht-Selbst-Anker, typisierte PDF-/Robots-Zusatzfelder, A-only und ausschließlich exakte B-Zuordnung berücksichtigt. Legacy-Dokumente und ungemessene aktuelle Tasks sind explizit unbekannt; globale Auslassung ist keine erfundene Messung.
- [x] Gemeinsame verifizierte Read-TX, opake wiederholbare Allstored-Session und eigener kontextgebundener Query-/Cursorvertrag implementiert. Vollständiger aktueller Taskhash gilt auch ohne Raw-Katalog. Dünne Nulltrefferfenster liefern ehrliche Fortsetzung; ein ausgewertetes leeres Ergebnis ersetzt keine unbekannte Historie.
- [x] In der Quellprüfung gefundene Lücke geschlossen: neue typisierte Ziel-/PDF-/Robots-Felder werden global vor Kontext/Callback geprüft, einschließlich ausgeschlossener Zeilen vor/nach dem Cursor. Arbeitsbudget wird vor dem Finding-Prädikat geprüft; Vorbereitung, Protokollprojektion und SQL-/Batchkosten sind keine gemessene Latenz-/RAM-Grenze.
- [x] 53 neue Testfunktionen geschrieben (18 Root / 11 Source / 6 Seitenleser / 18 Modell). Tatsächliche Workspace-Formatprüfung und begrenzte unabhängige Quellreviews dokumentiert. 275 Compiler-Eingaben und 341 Pflichtnamen eingefroren; neue vollständige Suite-/Buildrouten vorbereitet.
- [ ] Kompilieren und ausführen: **0 der 53 neuen Tests ausgeführt**. Kumulativer künftiger Mindestwert **1123 Workspace-Passes**, ausdrücklich keine Ergebniszahl. Compiler, Clippy und tatsächliche Laufzeit-/Native-Abnahme bleiben NOT_RUN.
- [ ] API-, CLI-, MCP- und Browserflächen an diesen Core anbinden sowie den getrennten vollständigen Finding-Export implementieren und tatsächlich prüfen. Gegenwärtige Raw-Inventare/CSV ersetzen diese Flächen nicht.
- [ ] Tatsächliche Protokoll-/Größenmatrizen, native API/CLI/SDK/Browser, Selected-Untervarianten, Recovery, RAM/Latenz, optimierten Release/Installation und Linux/Container abnehmen. Keine S6-Vollabnahme und keine hundertprozentige Funktionsabdeckung.

Details: [Core-Vertrag](https://github.com/lia-xim/open-seo-spider/blob/${branch}/docs/sitemap-findings-core.md), [exakter Vorbereitungsbeleg](https://github.com/lia-xim/open-seo-spider/blob/${branch}/docs/verification/sitemap-findings-core-preparation.json).

`;
const body=header+history,dest=path.join(__dirname,'sitemap-issue-11-findings-core-v1.md'),receipt=__filename.replace(/\.cjs$/,'.json');assert(!fs.existsSync(dest));assert(!fs.existsSync(receipt));
fs.writeFileSync(dest,body,{flag:'wx'});execFileSync(gh,['issue','edit','11','--repo','lia-xim/open-seo-spider','--body-file',dest]);
const after=JSON.parse(execFileSync(gh,['issue','view','11','--repo','lia-xim/open-seo-spider','--json','body,url,updatedAt']));assert.equal(after.body.trim(),body.trim());
fs.writeFileSync(receipt,JSON.stringify({schema_version:1,result:'PRIVATE_ISSUE_UPDATED_EXACT_BODY_VERIFIED',recorded_at:new Date().toISOString(),commit:head,branch,url:after.url,updated_at:after.updatedAt,body_sha256:sha(Buffer.from(body)),previous_body_sha256:sha(oldRaw),runtime_tests_executed:0},null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({url:after.url,commit:head,updated_at:after.updatedAt,body_sha256:sha(Buffer.from(body))}));
