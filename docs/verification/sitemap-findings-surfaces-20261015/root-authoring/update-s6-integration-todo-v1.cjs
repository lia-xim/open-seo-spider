const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),{createHash}=require('node:crypto'),{fileURLToPath,pathToFileURL}=require('node:url'),{execFileSync}=require('node:child_process');
const repo='E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-findings-surfaces-integration-v1';
const sha=b=>createHash('sha256').update(b).digest('hex'),insist=(c,m)=>{if(!c)throw new Error(m);};
const receiptPath=path.join(__dirname,'update-s6-integration-todo-v1.json');insist(!fs.existsSync(receiptPath),'receipt exists');
const ledgerPath=path.join(repo,'docs/feature-parity.json'),todoPath=path.join(repo,'docs/FEATURE_TODO.md'),granularPath=path.join(repo,'docs/parity/granular-acceptance.json'),generatorPath=path.join(repo,'scripts/update-feature-todo.mjs');
const ledgerBefore=fs.readFileSync(ledgerPath),todoBefore=fs.readFileSync(todoPath),granularBefore=fs.readFileSync(granularPath),generatorBefore=fs.readFileSync(generatorPath);
insist(sha(ledgerBefore)==='dc2837b745e91fdb7381e8426b8844288c49730300a6bc25663749b515a3034b','ledger changed');
insist(sha(todoBefore)==='54ff553fb7259d75b9fb7bdcfc3a41e2a16baf8a0e0a4fadc5575a7f0bbf9cd9','TODO changed');
insist(sha(granularBefore)==='e85b205fd6d061c96043c05ea2e6ffa3dbbe6a0f390c07bb8f1ffaf22f316962','granular ledger changed');
const catalog=JSON.parse(ledgerBefore),featureBefore=JSON.stringify(catalog.features),summaryBefore=JSON.stringify(catalog.status_summary),work=catalog.current_work.find(w=>w.id==='batch-b12-sitemap-membership');insist(work,'work entry missing');
insist(!work.pending_acceptance_de.some(s=>s.startsWith('Aktueller S6-Flächenschritt:')),'note already present');
work.pending_acceptance_de=work.pending_acceptance_de.map(s=>s.startsWith('Aktueller S6-Core-Quellschritt:')?s.replace('Aktueller S6-Core-Quellschritt:','Vorheriger S6-Core-Quellschritt (898c404):'):s);
work.pending_acceptance_de.push('Aktueller S6-Flächenschritt: gemeinsame strikte Auswahl-/Token-/JSON-/CSV-Adapter, versiegelte Session.page für denselben Lesestand, GET/POST-Befundrouten, kontrollierte Worker mit Abbruch/Frist/Shutdown, zwei vollständige CSV-Durchläufe und eigene Sitemap-Befundansicht im Quellcode integriert. CLI und eigenes MCP-Werkzeug sind angebunden; ihre unabhängige Quellprüfung und separaten Korrekturen laufen. Format-/Syntaxprüfungen und begrenzte Quellreviews ersetzen keine Kompilierung oder Abnahme. Compiler, Workspace-/SQLite-/HTTP-/CLI-/SDK-/Browserläufe und neue physische Größen-/Quellen-/Join-Grenzen NICHT AUSGEFÜHRT; koordinierter Compute-HOLD besteht weiter. Gemeinsamen korrigierten Stand einfrieren und nach tatsächlicher Freigabe vollständig prüfen. Keine neuen Häkchen, keine S6-Vollabnahme, keine Installation und keine Herstellerparität.');
work.gates.source_filter_s6_surfaces='WIP_SOURCE_CORE_SESSION_APP_HTTP_CLI_MCP_UI_FULL_CSV_INTEGRATED_REVIEW_POSTFIX_IN_PROGRESS_COMPILE_TEST_NATIVE_NOT_RUN';
if(!work.documentation.includes('docs/sitemap-findings-surfaces.md'))work.documentation.push('docs/sitemap-findings-surfaces.md');
insist(JSON.stringify(catalog.features)===featureBefore&&JSON.stringify(catalog.status_summary)===summaryBefore,'acceptance features/counts changed');
const ledgerBytes=Buffer.from(JSON.stringify(catalog,null,2)+'\n');
insist((generatorBefore.toString().match(/^import .+ from 'node:[^']+';\r?$/gm)||[]).length===4,'generator imports changed');
let code=generatorBefore.toString().replace(/^import .+ from 'node:[^']+';\r?\n/gm,'');insist((code.match(/import\.meta\.url/g)||[]).length===1,'generator URL changed');code=code.replace('import.meta.url',JSON.stringify(pathToFileURL(generatorPath).href));
let previewBytes=null;const messages=[];
function preview(check){
 const facade={readFileSync(p,encoding){const target=path.resolve(p);let b;if(target===path.resolve(ledgerPath))b=ledgerBytes;else if(target===path.resolve(granularPath))b=granularBefore;else if(target===path.resolve(todoPath)&&check&&previewBytes)b=previewBytes;else throw Error('Unexpected generator read '+target);return encoding?b.toString(encoding):b;},existsSync(p){insist(path.resolve(p)===path.resolve(todoPath),'unexpected exists');return true;},writeFileSync(p,value,encoding){insist(!check&&path.resolve(p)===path.resolve(todoPath)&&encoding==='utf8'&&previewBytes===null,'unexpected write');previewBytes=Buffer.from(value);}};
 vm.runInNewContext(code,{fs:facade,path,fileURLToPath,createHash,process:{argv:['node',generatorPath,...(check?['--check']:[])],stdout:{write(v){messages.push(v.trim());}}}},{timeout:5000});
}
preview(false);insist(previewBytes?.length>0,'empty TODO preview');preview(true);
for(const [p,b] of [[ledgerPath,ledgerBefore],[todoPath,todoBefore],[granularPath,granularBefore],[generatorPath,generatorBefore]])insist(sha(fs.readFileSync(p))===sha(b),'prerequisite changed');
// Complete real generator validation/render/check passed in memory first.
fs.writeFileSync(ledgerPath,ledgerBytes);fs.writeFileSync(todoPath,previewBytes);
const checked=execFileSync(process.execPath,[generatorPath,'--check'],{cwd:repo,encoding:'utf8'}).trim();
const receipt={schema_version:1,status:'INTEGRATION_TODO_UPDATED_NO_ACCEPTANCE_CHANGE',recorded_at:new Date().toISOString(),ledger_before_sha256:sha(ledgerBefore),ledger_after_sha256:sha(ledgerBytes),todo_before_sha256:sha(todoBefore),todo_after_sha256:sha(previewBytes),granular_unchanged_sha256:sha(granularBefore),generator_sha256:sha(generatorBefore),full_in_memory_preview:true,preview_messages:messages,checked,new_tests_executed:0,accepted_counts_unchanged:{implemented:31,partial:67,planned:46},installed_runtime_modified:false};fs.writeFileSync(receiptPath,JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});process.stdout.write(JSON.stringify(receipt)+'\n');
