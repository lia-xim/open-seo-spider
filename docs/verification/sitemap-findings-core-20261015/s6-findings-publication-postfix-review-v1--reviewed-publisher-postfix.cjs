const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),vm=require('node:vm');
const {fileURLToPath,pathToFileURL}=require('node:url');
const {createHash}=require('node:crypto'),{execFileSync}=require('node:child_process');const sha=b=>createHash('sha256').update(b).digest('hex');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',repo=path.join(base,'sitemap-findings-core-integration-v1');
const [prepPin,prepReviewPin]=process.argv.slice(2);for(const p of [prepPin,prepReviewPin])assert(/^[a-f0-9]{64}$/.test(p??''));
const prepRaw=fs.readFileSync(path.join(__dirname,'prepare-sitemap-findings-core-v1.json'));assert.equal(sha(prepRaw),prepPin);const prep=JSON.parse(prepRaw);
assert.equal(prep.files,275);assert.equal(prep.new_authored_tests,53);assert.equal(prep.required_names,341);assert.equal(prep.minimum_expected,1123);
const reviewRaw=fs.readFileSync(path.join(base,'s6-findings-preparation-review-v1/receipt.json'));assert.equal(sha(reviewRaw),prepReviewPin);
const sourceRaw=fs.readFileSync(path.join(prep.source,'manifest.json'));assert.equal(sha(sourceRaw),prep.manifest_sha256);const source=JSON.parse(sourceRaw);
for(const f of source.files)assert.equal(sha(fs.readFileSync(path.join(repo,f.path))),f.sha256,'Actual frozen source '+f.path);
for(const f of prep.staged_product_bindings)assert.equal(sha(execFileSync('git',['show',':'+f.path],{cwd:repo})),f.sha256);
const statusPaths=execFileSync('git',['status','--porcelain=v1','-uall'],{cwd:repo}).toString().trimEnd().split(/\r?\n/).filter(Boolean).map(s=>s.slice(3)).sort();
assert.deepEqual(statusPaths,[...prep.approved_product_bindings.map(f=>f.path),'docs/sitemap-findings-core.md'].sort(),'Only reviewed product plus new owned doc before publication');
const stagedPaths=execFileSync('git',['diff','--cached','--name-only'],{cwd:repo}).toString().trim().split(/\r?\n/).filter(Boolean).sort();
assert.deepEqual(stagedPaths,prep.staged_product_bindings.map(f=>f.path).sort(),'Exact staged product allowlist');
const docPath=path.join(repo,'docs/sitemap-findings-core.md'),docBefore=fs.readFileSync(docPath);
const folder='docs/verification/sitemap-findings-core-20261015',items=[];
function add(file,name){const bytes=fs.readFileSync(file);items.push({source:file,path:folder+'/'+name,bytes,sha256:sha(bytes)});}
add(path.join(prep.source,'manifest.json'),'source-manifest.json');
for(const dir of [
 's6-source-audit-v1','s6-pages-audit-v1','s6-acceptance-audit-v1','s6-findings-model-v1',
 's6-root-session-core-v1','s6-source-reader-core-v1','s6-page-reader-core-v1','s6-predicate-core-v1',
 's6-root-session-postfix-v1','s6-page-reader-postfix-v1','s6-root-session-review-v1',
 's6-predicate-review-v1','s6-source-reader-review-v1','s6-page-reader-review-v1',
 's6-root-session-postfix-review-v1','s6-findings-preparation-review-v1',
 's6-findings-publication-review-v1','s6-findings-publication-postfix-review-v1',
]){for(const f of fs.readdirSync(path.join(base,dir),{withFileTypes:true}).filter(f=>f.isFile()&&/\.(json|md|rs|cjs|mjs)$/i.test(f.name)))add(path.join(base,dir,f.name),dir+'--'+f.name);}
for(const f of ['freeze-s6-findings-model-v1.cjs','freeze-s6-root-session-v1.cjs','freeze-s6-root-postfix-v1.cjs',
 'promote-s6-handoff-v1.cjs','promote-s6-predicate-core-v1.json','promote-s6-source-reader-core-v1.json',
 'promote-s6-page-reader-core-v1.json','promote-s6-page-reader-postfix-v1.json',
 'prepare-sitemap-findings-core-v1.cjs','prepare-sitemap-findings-core-v1.json',
 'run-sitemap-findings-core-suite-v1.mjs','build-sitemap-findings-core-v1.mjs',
 'publish-sitemap-findings-core-docs-v1.cjs','stage-sitemap-findings-core-v1.cjs',
 'update-sitemap-findings-core-issue-v1.cjs'])add(path.join(__dirname,f),'preparation--'+f);
// Preserve the exact six superseded review payloads, not merely summaries of
// bytes no longer present in the final product. Other final payloads are Git files.
for(const [dir,paths] of [
 ['s6-root-session-core-v1',['crates/core/src/store/sitemap_findings_read.rs','crates/core/src/store/sitemap_findings_read_tests.rs','crates/core/src/sitemap_membership_analysis/findings_store_tests.rs']],
 ['s6-page-reader-core-v1',['crates/core/src/sitemap_membership_analysis.rs','crates/core/src/sitemap_membership_analysis/findings.rs','crates/core/src/sitemap_membership_analysis/findings_tests.rs']],
])for(const p of paths)add(path.join(base,dir,p),'before-postfix--'+dir+'--'+path.basename(p));
assert.equal(new Set(items.map(i=>i.path)).size,items.length);
const proofPath='docs/verification/sitemap-findings-core-preparation.json',receiptPath=__filename.replace(/\.cjs$/,'.json');
assert(!fs.existsSync(receiptPath));assert(!fs.existsSync(path.join(repo,proofPath)));
for(const i of items)assert(!fs.existsSync(path.join(repo,i.path)),'Fresh evidence companion '+i.path);
const attrPath=path.join(repo,'.gitattributes'),attrBefore=fs.readFileSync(attrPath),rule=folder+'/** -text whitespace=-blank-at-eol,-blank-at-eof,-space-before-tab,cr-at-eol';assert(!attrBefore.toString().includes(rule));
const ledgerPath=path.join(repo,'docs/feature-parity.json'),ledgerBefore=fs.readFileSync(ledgerPath),ledger=JSON.parse(ledgerBefore),entry=ledger.current_work.find(x=>x.id==='batch-b12-sitemap-membership');assert(entry);
const beforeCsv=entry.pending_acceptance_de.findIndex(x=>x.startsWith('Aktueller S4-CSV-Quellschritt:'));assert(beforeCsv>=0);
entry.pending_acceptance_de[beforeCsv]=entry.pending_acceptance_de[beforeCsv].replace('Aktueller S4-CSV-Quellschritt:','Vorheriger S4-CSV-Quellstand (c8f539b):');
entry.pending_acceptance_de.push('Aktueller S6-Core-Quellschritt: sieben getrennte Finding-Prädikate auf vollständigen gespeicherten SavedPage-/SourceAttempt-Domänen, typisierte PDF-/Robots-Qualifikation, tatsächliche A-only/exakteB/Legacy-/Task-Nachweise, vollständiger Taskhash auch ohne Raw-Katalog, gemeinsame verifizierte Read-TX, wiederholbare Allstored-Session und kontextgebundene dünne Trefferfortsetzungen implementiert. Globale Zusatzfeldprüfung vor Callback schließt ausgeschlossene beschädigte Berichtsteile ein. 53 zusätzliche Testfunktionen geschrieben (18 Root/11 Source/6 Seitenleser/18 Modell); 275 Compiler-Eingaben und341 Pflichtnamen eingefroren, künftig mindestens1123 Gesamttests. Tatsächliche Formatprüfung und begrenzte unabhängige Quellreviews bestehen; Compiler, Clippy, Tests und native Läufe NICHT AUSGEFÜHRT. API-/CLI-/MCP-/UI-Anbindung und eigener vollständiger Finding-Export bleiben zu implementieren, Größen/RAM/Latenz/Recovery/Selected/Linux/Container abzunehmen. Keine S6-Vollabnahme, keine Herstellerparität und keine Installation.');
for(const p of ['docs/sitemap-findings-core.md',proofPath]){assert(!entry.documentation.includes(p));entry.documentation.push(p);}
entry.gates.source_filter_s6_surfaces='CORE_SOURCE_IMPLEMENTED_ADAPTERS_API_CLI_MCP_UI_AND_DISTINCT_FULL_EXPORT_NOT_IMPLEMENTED_RUNTIME_NOT_RUN';
entry.gates.source_filter_s6_core='WIP53_AUTHORED_FROZEN275_MIN1123_NAMED341_TYPED_PREDICATES_FULL_STORED_SAME_TX_SPARSE_CURSOR_GLOBAL_TAIL_PREFLIGHT_FMT_STATIC_REVIEW_COMPILE_TEST_NATIVE_NOT_RUN';
const proof={schema_version:1,result:'SOURCE_ONLY_53_AUTHORED_FROZEN275_MIN1123_NAMED341_FMT_STATIC_REVIEW_COMPILE_TEST_NATIVE_NOT_RUN',recorded_at:new Date().toISOString(),branch:'codex/sitemap-findings-core-20261015',baseline_commit:'c8f539be979ac5436dcc962c5ce7677aab9f3b0d',source_manifest_sha256:prep.manifest_sha256,frozen_compiler_inputs:275,owned_product_files:prep.approved_product_bindings.length,new_authored_tests:53,new_tests_executed:0,required_unique_test_names:341,minimum_expected_workspace_tests:1123,
 format:'PASS actual cargo fmt --all --check and git diff --check on assembled Core source; not compilation',
 scope:['Independent seven findings predicates with Match/NotMatched/Unknown and exact original requested/task identities','Full actual saved-report traversal in <=200 batches sharing old target/membership/inlink projection','Actual source measurements plus exact commit association, A-only, explicit legacy documents and unmeasured current tasks','Full current task-state fingerprint even RawNone, distinct versioned findings context, stale check before cursor and callback','Opaque same-TX repeated complete-stored visitor and limited sparse page with honest work continuation','Global typed target/PDF/robots preflight includes excluded prefix/tail before public callback'],
 source_findings_closed:['New typed PDF/robots fields were initially checked only in selected batches; now full actual report preflight before public callback','Candidate budget checked before finding predicate; verification, protocol projection and page batching remain separate costs','Preparer originally staged index before runner preflight; now all runner plan validation precedes stage/output','Publisher originally ran required generator validation after archive/ledger writes; now the exact generator fully validates/renders/checks proposed bytes in memory before any target document mutation'],
 withdrawn_hypothesis:'Robots measurement cursor is unreachable through globally verified XML-only Source records; existing schema/model/row validators prove the domain, no defensive code change counted',
 static_review:'BOUNDED_ONLY_NOT_COMPILER_OR_RUNTIME_PROOF',compiler:'NOT_RUN_COORDINATED_COMPUTE_HOLD',clippy:'NOT_RUN',workspace_tests:'NOT_RUN',native_cli_api_csv_sdk_browser:'NOT_RUN',
 open_gates:['Actual compile, entire workspace >=1123 passes with all341 required names, all-target Clippy and owned real renderer','Implement shared API/CLI/MCP/UI adapters and separate full finding export','Actual native/source boundaries, current route physical CSV64MiB/+1, JSON/row/catalog budgets','Selected-context presence variants, crash/restart/rollback, measured RSS/latency and SQL working storage','Optimized release, data-preserving installation, Linux/container and unavailable remote CI'],
 accepted_catalog_counts_unchanged:{implemented:31,partial:67,planned:46},installed_runtime_modified:false,manufacturer_parity:'NOT_PROVEN',archive_note:'Exact raw review bytes and superseded payloads. Host-bound preparation references are not portable launch entrypoints. Every new authored test is unexecuted; minimum counts are expectations, not results.',companions:items.map(({source,path,bytes,sha256})=>({source,path,bytes:bytes.length,sha256}))};
const proofBytes=Buffer.from(JSON.stringify(proof,null,2)+'\n'),attrBytes=Buffer.from(attrBefore.toString().trimEnd()+'\n'+rule+'\n'),ledgerBytes=Buffer.from(JSON.stringify(ledger,null,2)+'\n');
// Execute the actual generator with an in-memory fs facade. Proposed catalog
// and TODO bytes never touch the project during this full validation/render.
const generatorPath=path.join(repo,'scripts/update-feature-todo.mjs'),generatorBefore=fs.readFileSync(generatorPath);
const todoPath=path.join(repo,'docs/FEATURE_TODO.md'),todoBefore=fs.readFileSync(todoPath);
assert.equal(ledger.granular_acceptance.artifact,'docs/parity/granular-acceptance.json');
const granularPath=path.join(repo,ledger.granular_acceptance.artifact),granularBefore=fs.readFileSync(granularPath);
const importLines=generatorBefore.toString().match(/^import .+ from 'node:[^']+';\r?$/gm);assert.equal(importLines.length,4);
let generatorCode=generatorBefore.toString().replace(/^import .+ from 'node:[^']+';\r?\n/gm,'');
assert.equal(generatorCode.match(/import\.meta\.url/g)?.length,1);generatorCode=generatorCode.replace('import.meta.url',JSON.stringify(pathToFileURL(generatorPath).href));
let previewBytes=null;const previewMessages=[];
function preview(check){
 const facade={
  readFileSync(p,encoding){const target=path.resolve(p);let b;
   if(target===path.resolve(ledgerPath))b=ledgerBytes;
   else if(target===path.resolve(granularPath))b=granularBefore;
   else if(target===path.resolve(todoPath)&&check&&previewBytes)b=previewBytes;
   else throw new Error('Unexpected generator read: '+target);
   return encoding?b.toString(encoding):b;
  },
  existsSync(p){assert.equal(path.resolve(p),path.resolve(todoPath));return true;},
  writeFileSync(p,value,encoding){assert(!check);assert.equal(path.resolve(p),path.resolve(todoPath));assert.equal(encoding,'utf8');assert.equal(previewBytes,null);previewBytes=Buffer.from(value);},
 };
 vm.runInNewContext(generatorCode,{fs:facade,path,fileURLToPath,createHash,process:{argv:['node',generatorPath,...(check?['--check']:[])],stdout:{write(v){previewMessages.push(v.trim());}}}},{timeout:5000});
}
preview(false);assert(previewBytes&&previewBytes.length>0);preview(true);
assert.equal(sha(fs.readFileSync(attrPath)),sha(attrBefore));assert.equal(sha(fs.readFileSync(ledgerPath)),sha(ledgerBefore));
for(const [p,b] of [[docPath,docBefore],[todoPath,todoBefore],[generatorPath,generatorBefore],[granularPath,granularBefore]])assert.equal(sha(fs.readFileSync(p)),sha(b),'Prerequisite changed '+p);
// All fresh paths, exact source/index inputs, raw archive and ledger/attr bytes
// are decided before any archive/document mutation.
for(const i of items){const dest=path.join(repo,i.path);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.writeFileSync(dest,i.bytes,{flag:'wx'});assert.equal(sha(fs.readFileSync(dest)),i.sha256);}
fs.writeFileSync(path.join(repo,proofPath),proofBytes,{flag:'wx'});fs.writeFileSync(attrPath,attrBytes);fs.writeFileSync(ledgerPath,ledgerBytes);fs.writeFileSync(todoPath,previewBytes);
const generated=previewMessages[0];
const checked=execFileSync(process.execPath,[path.join(repo,'scripts/update-feature-todo.mjs'),'--check'],{cwd:repo}).toString();
const receipt={schema_version:1,result:'EXACT_SOURCE_ONLY_EVIDENCE_AND_TODO_PUBLISHED_NO_EXECUTION',recorded_at:new Date().toISOString(),proof_path:proofPath,proof_sha256:sha(proofBytes),archived_companions:items.length,source_manifest_sha256:prep.manifest_sha256,frozen_inputs:275,new_authored_tests:53,executed_tests:0,required_names:341,minimum_expected:1123,generator_full_in_memory_preview:true,prerequisite_sha256:{doc:sha(docBefore),todo:sha(todoBefore),generator:sha(generatorBefore),granular:sha(granularBefore),ledger:sha(ledgerBefore)},generated:generated.trim(),checked:checked.trim(),installed_runtime_modified:false};
fs.writeFileSync(receiptPath,JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(receipt));
