const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto'),{execFileSync}=require('node:child_process');
const sha=b=>createHash('sha256').update(b).digest('hex'),gh='C:/Program Files/GitHub CLI/gh.exe',repo='lia-xim/open-seo-spider',commit=process.argv[2];assert(/^[a-f0-9]{40}$/.test(commit??''));
const previous=fs.readFileSync(path.join(__dirname,'sitemap-issue-11-rich-read-v1.md'));assert.equal(sha(previous),'c7ac71f9849796371d4a2a750c7d3efab32dd514cd986c2a5d614f58c9459955');
const current=JSON.parse(execFileSync(gh,['issue','view','11','--repo',repo,'--json','body,updatedAt,url']));assert.equal(current.body.trim(),previous.toString().trim(),'Concurrent issue edit: merge before replacing');
const prep=JSON.parse(fs.readFileSync(path.join(__dirname,'prepare-sitemap-csv-streaming-v1.json')));assert.equal(prep.new_authored_tests,35);assert.equal(prep.required_names,288);assert.equal(prep.minimum_expected,1070);
const branch='codex/sitemap-csv-streaming-20261014',url='https://github.com/'+repo+'/blob/'+branch+'/',heading='### Neuer S4-Quellschritt: kompakter Rich-Kontext und App-Inventar';assert(previous.toString().includes(heading));
let tail=previous.toString().slice(previous.toString().indexOf(heading)).replace(heading,'### Vorheriger S4-Rich-Stand (d105fb8; separate ungetestete Quelle255)');
tail=tail.replace('Coverage/Analyse und der Inventarhandler sind jetzt als Quellcode verbunden; ausgewählte Eltern bleiben materialisierende Kompatibilität und CSV bleibt gepuffert.','Coverage/Analyse, Inventarhandler und begrenzter CSV-Transport sind jetzt als Quellcode verbunden; ausgewählte Eltern bleiben materialisierende Kompatibilität. Die neue CSV-Laufzeitabnahme ist offen.');
tail=tail.replace('https://github.com/lia-xim/open-seo-spider/blob/codex/sitemap-rich-read-20261014/docs/FEATURE_TODO.md',url+'docs/FEATURE_TODO.md');
const lines=[
 'Der aktuelle Entwicklungskandidat liegt im privaten Zweig `'+branch+'`, aufbauend auf dem separat erhaltenen Rich-Kandidaten `d105fb8`.',
 '',
 'Der Gesamtkatalog bleibt bei **31 eigenen Abnahmen erfüllt / 67 teilweise geprüft / 46 offen**. Das ist kein Hersteller-Abdeckungsgrad. Vollständige Screaming-Frog-Parität bleibt **NOT_PROVEN**.',
 '',
 'Quellcommit: [`'+commit.slice(0,7)+'`](https://github.com/'+repo+'/commit/'+commit+'), ausdrücklich WIP ohne Kompilierung/Testabnahme oder neue Installation. Frühere Quellen und Belege bleiben erhalten.',
 '',
 '### Neuer S4-Quellschritt: vollständiger gespeicherter CSV-Bestand mit begrenztem Transport',
 '',
 '- [x] Eine tatsächliche gemeinsame Read-TX mit vollständigen Raw-/Source-/Config-/Coverage-/Report-Guards und opaker wiederholt besuchbarer Allstored-Session implementiert. Kein Inventarseitenloop oder nachgebauter Teilkatalog; exakte Originalfilter und bestehender Zustand/Token bleiben.',
 '- [x] Zwei tatsächliche Formatterdurchläufe im Quellcode verbunden: vor Ready exakte Länge/SHA/Zeilenzahl unter inklusivem 64-MiB-Cap; danach gleiche Session/Formatter in Queue4 mit 64-KiB-Blöcken. Positiver Endblock wartet auf gleiche Receipts, Core-TX-Return und tatsächlichen Worker-Join.',
 '- [x] Geteilte Cancellation-/Deadline-Readerchecks, tatsächliche SQLite-Interrupt-Handles, Cancel-on-drop, unabhängigen Watchdog und registrierten Shutdown-Drain implementiert. Grobe Config-/Policy-/Selected-Arbeitsschritte bleiben ohne harte Latenzzusage.',
 '- [x] Drei konkrete Quellbefunde korrigiert und unabhängig nachgeprüft: Config-SHA-Borrow, irrtümlich test-only produktiver JSON-Writer und Leerlauf-Timer beim aktiven Verbrauch gepufferter Blöcke. Nur für Tests benötigte private Adapter entsprechend markiert.',
 '- [x] 35 neue Testfunktionen geschrieben: 9 Store-/Session /10 Control-/Reader /3 Writer /13 App-/HTTP-/Download-Gruppen. Die 13 App-Gruppen sind 14 geplanten Themen zugeordnet; nicht alle Untergates sind damit abgedeckt. Workspace-Formatprüfung und begrenzte Quellreviews dokumentiert.',
 '- [x] '+prep.files+' Compiler-Eingaben und 288 erforderliche Testnamen eingefroren; künftige vollständige Suite-/Buildrouten mit explizitem Rechenfenster-Guard, vorherigem Targetbackup und exakter Sourcebindung vorbereitet.',
 '- [ ] Kompilieren und ausführen: **0 der 35 neuen Tests ausgeführt**. Kumulativer künftiger Mindestwert **1070 Workspace-Passes**, ausdrücklich keine Ergebniszahl. Compiler, Clippy, echte HTTP-/TCP-/API-/CLI-/MCP-/Browser- und native Abnahme bleiben NOT_RUN.',
 '- [ ] Neue gültige physische CSV-Paarung exakt 67.108.864/67.108.865 Bytes abnehmen. Die frühere echte Installation hat eine solche Paarung, deren Producer dient nur als wiederzuprüfender Startpunkt; Writer-Sink-Test und historische Belege sind keine neue Route-Abnahme.',
 '- [ ] Inverse Selected-Präsenz und isolierten Selected-Kontext ergänzen; tatsächliche große Quellen, RAM/Latenz, Crash/Restart, alle sieben vollständigen Finding-Filter und Linux/Container prüfen. Keine S4-Vollabnahme und keine Installation.',
 '',
 'Details: [CSV-Vertrag]('+url+'docs/sitemap-csv-streaming.md), [exakter Vorbereitungsbeleg]('+url+'docs/verification/sitemap-csv-streaming-preparation.json).',
 '',tail,
 ];
const body=lines.join('\n'),file=path.join(__dirname,'sitemap-issue-11-csv-streaming-v1.md');assert(!fs.existsSync(file));fs.writeFileSync(file,body,{flag:'wx'});
execFileSync(gh,['issue','edit','11','--repo',repo,'--body-file',file]);
const verified=JSON.parse(execFileSync(gh,['issue','view','11','--repo',repo,'--json','body,updatedAt,url']));assert.equal(verified.body.trim(),body.trim());
const receipt={updated_at:verified.updatedAt,url:verified.url,commit,branch,body_file:file,body_sha256:sha(fs.readFileSync(file)),verified_remote_body:true,previous_body_sha256:sha(previous),new_tests_executed:0};fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(receipt));
