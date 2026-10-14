const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto'),{execFileSync}=require('node:child_process');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',repo=path.join(base,'sitemap-csv-streaming-integration-v1'),sha=b=>createHash('sha256').update(b).digest('hex');
const prepRaw=fs.readFileSync(path.join(__dirname,'prepare-sitemap-csv-streaming-v1.json')),prep=JSON.parse(prepRaw);assert.equal(prep.files,265);assert.equal(prep.new_authored_tests,35);assert.equal(prep.required_names,288);assert.equal(prep.minimum_expected,1070);
const physicalPin=process.argv[2],physicalReviewPin=process.argv[3];assert(/^[a-f0-9]{64}$/.test(physicalPin??''));assert(/^[a-f0-9]{64}$/.test(physicalReviewPin??''));
for(const [p,pin] of [['s4-csv-final-preparation-review-v1/receipt.json','f84e45e68892c39178cd1ddedda0bb01d3f59c4a8df40bc4d732755b34ef3c80'],['s4-csv-physical-preparation-v1/manifest.json',physicalPin],['s4-csv-physical-preparation-review-v1/receipt.json',physicalReviewPin]])assert.equal(sha(fs.readFileSync(path.join(base,p))),pin,p);
const folder='docs/verification/sitemap-csv-streaming-20261015',items=[];
function add(source,name){const bytes=fs.readFileSync(source);items.push({source,path:folder+'/'+name,sha256:sha(bytes),bytes});}
add(path.join(prep.source,'manifest.json'),'source-manifest.json');assert.equal(items[0].sha256,prep.manifest_sha256);
for(const [dir,prefix] of [
 ['s4-csv-root-session-v1','root-session'],['s4-csv-streaming-app-v1','app-transport'],['s4-csv-control-core-v1','read-control'],
 ['s4-csv-streaming-tests-v1','wire-tests'],['s4-csv-root-session-review-v1','root-review'],['s4-csv-app-independent-review-v1','app-review'],
 ['s4-csv-control-independent-review-v1','control-review'],['s4-csv-reviewed-postfix-v1','reviewed-postfix'],['s4-csv-postfix-review-v1','postfix-review'],
 ['s4-csv-compat-wrappers-v1','compat-wrappers'],['s4-csv-compat-wrappers-review-v1','compat-review'],
 ['s4-csv-streaming-tests-review-v1','wire-review'],['s4-csv-preparation-review-v1','preparation-review'],
 ['s4-csv-final-preparation-review-v1','final-preparation-review'],['s4-csv-physical-preparation-v1','physical-preparation'],
 ['s4-csv-physical-preparation-review-v1','physical-preparation-review'],
 ['s4-csv-doc-publication-review-v1','doc-publication-review'],
]){const full=path.join(base,dir);assert(fs.statSync(full).isDirectory(),dir);for(const item of fs.readdirSync(full,{withFileTypes:true}).filter(x=>x.isFile()&&/\.(json|md|mjs|cjs|diff|patch)$/i.test(x.name)))add(path.join(full,item.name),prefix+'--'+item.name);}
for(const name of ['freeze-sitemap-csv-root-session-v1.cjs','refactor-sitemap-csv-context-v1.cjs','register-sitemap-stored-source-tests-v1.cjs','register-sitemap-stored-source-tests-v1.json',
 'promote-sitemap-csv-app-v1.cjs','promote-sitemap-csv-app-v1.json','promote-sitemap-csv-control-v1.cjs','promote-sitemap-csv-control-v1.json',
 'freeze-sitemap-csv-postfix-v1.cjs','freeze-sitemap-csv-compat-wrappers-v1.cjs','promote-sitemap-csv-wire-tests-v1.cjs','promote-sitemap-csv-wire-tests-v1.json',
 'normalize-sitemap-csv-product-v1.cjs','normalize-sitemap-csv-product-v1.json','prepare-sitemap-csv-streaming-v1.cjs','prepare-sitemap-csv-streaming-v1.json',
 'run-sitemap-csv-streaming-suite-v1.mjs','build-sitemap-csv-streaming-v1.mjs','sitemap-csv-preparation-invocation-correction-v1.json',
 'publish-sitemap-csv-streaming-docs-v1.cjs','stage-sitemap-csv-streaming-v1.cjs','update-sitemap-csv-streaming-issue-v1.cjs'])add(path.join(__dirname,name),'preparation--'+name);
for(const [p,name] of [
 ['s4-csv-root-session-v1/product/crates/core/src/store/sitemap_read_context.rs','before-postfix--sitemap_read_context.rs'],
 ['s4-csv-streaming-app-v1/crates/app/src/sitemap_membership.rs','before-postfix--sitemap_membership.rs'],
 ['s4-csv-streaming-app-v1/crates/app/src/server/sitemap_csv_stream.rs','before-postfix--sitemap_csv_stream.rs'],
 ['s4-csv-control-core-v1/product/crates/core/src/sitemap_coverage/verified_read.rs','before-cfg--coverage_verified_read.rs'],
 ['s4-csv-control-core-v1/product/crates/core/src/sitemap_membership_analysis.rs','before-cfg--sitemap_membership_analysis.rs'],
 ['s4-csv-control-core-v1/product/crates/core/src/sitemap_membership_analysis/window.rs','before-cfg--membership_window.rs'],
])add(path.join(base,p),name);
assert.equal(new Set(items.map(x=>x.path)).size,items.length);for(const e of items)assert(!fs.existsSync(path.join(repo,e.path)),'Fresh archive '+e.path);
const proof={schema_version:1,result:'SOURCE_ONLY_35_AUTHORED_FROZEN265_MIN1070_NAMED288_FMT_STATIC_REVIEW_COMPILE_TEST_NATIVE_NOT_RUN',recorded_at:new Date().toISOString(),branch:'codex/sitemap-csv-streaming-20261014',baseline_commit:'d105fb8a0c527ffd99164e44ec644bd0e49f61a5',source_manifest_sha256:prep.manifest_sha256,frozen_compiler_inputs:265,owned_staged_product_files:26,new_authored_tests:35,new_tests_executed:0,required_unique_test_names:288,minimum_expected_workspace_tests:1070,
 format:'PASS actual cargo fmt --all -- --check after exact hook integration and LF normalization; actual git diff --check exit0',
 scope:['Same actual-TX full-context preparation shared by bounded inventory and opaque fixed-filter repeated allstored visits','Controlled Raw/Source/coverage/report/analysis readers with owned actual SQLite interrupts','Unchanged CSV formatter in two full passes; inclusive64MiB counting before Ready; bounded4x64KiB queue','Final positive tail withheld until receipt equality, Core TX return and actual blocking-worker join','Cancel/drop/deadline/supervisor registry and actual shutdown drain implemented as source'],
 source_findings_closed:['Config-SHA move while prepared analysis lifetime remains borrowed','Production-shared BoundedBytes incorrectly cfg(test)','Consumer prefix progress missing from idle watcher after producer joins','Private default compatibility wrappers and stats retained only under actual test callers','Preparer initially trusted arbitrary changes within owned paths; now derives every final approved payload/hook/LF byte before staging/freezing'],
 static_review:'BOUNDED_ONLY_NOT_COMPILER_OR_RUNTIME_PROOF',compiler:'NOT_RUN_COORDINATED_COMPUTE_HOLD',clippy:'NOT_RUN',workspace_tests:'NOT_RUN',native_cli_api_csv_sdk_browser:'NOT_RUN',
 physical_csv:'OPEN_NEW_ROUTE: prior genuine ab1 native exact64MiB/+1 producer is a reusable preparation seed, not current source proof; actual new run remains required',
 open_gates:['Compile, actual full workspace and all288 required names, all-target Clippy, owned actual browser gates','Real HTTP/TCP backpressure/disconnect/deadline/latefault/tail/TX/join/drain behavior and App token compatibility','Fresh valid physical67108864/67108865 Store+HTTP pair on the actual new binary','Selected inverse presence and isolated selected-operation context variants; selected full compatibility remains materializing','Actual large-source protocol boundary matrix, RSS/latency, SQL caches/sorts and coarse cancellation latency','Physical row256KiB/catalog32MiB, crash/restart and rollback failures','All seven full stored finding filters and API/CLI/MCP/UI surfaces','Optimized native release, installation, Linux/container and unavailable remote CI'],
 installed_binary_sha256:'ab1ac93c82905aa627d100d8622a50acf93dd7fe59c3f854eecdaac667110bcf',installed_runtime_modified:false,manufacturer_parity:'NOT_PROVEN',
 archive_note:'Raw exact evidence bytes. Archived host-specific preparers/runners are source-review evidence, not portable entrypoints. Actual heavy routes still require actual coordinated compute release.',companions:items.map(({source,path,sha256,bytes})=>({source,path,sha256,bytes:bytes.length}))};
const proofPath='docs/verification/sitemap-csv-streaming-preparation.json';assert(!fs.existsSync(path.join(repo,proofPath)));
const attrPath=path.join(repo,'.gitattributes'),attrBefore=fs.readFileSync(attrPath),attr=attrBefore.toString('utf8'),rule=folder+'/** -text whitespace=-blank-at-eol,-blank-at-eof,-space-before-tab,cr-at-eol';assert(!attr.includes(rule));
const ledgerPath=path.join(repo,'docs/feature-parity.json'),ledgerBefore=fs.readFileSync(ledgerPath),ledger=JSON.parse(ledgerBefore),work=ledger.current_work.find(x=>x.id==='batch-b12-sitemap-membership');assert(work);
const old=work.pending_acceptance_de.findIndex(x=>x.startsWith('Aktueller S4-Rich-Quellschritt:'));assert(old>=0);work.pending_acceptance_de[old]=work.pending_acceptance_de[old].replace('Aktueller S4-Rich-Quellschritt:','Vorheriger S4-Rich-Quellstand (d105fb8):');
work.pending_acceptance_de.push('Aktueller S4-CSV-Quellschritt: gemeinsamer tatsächlicher Same-TX-Kontext, opake wiederholbare Allstored-Session, kontrollierte Reader und zwei Formatterdurchläufe mit begrenzter Queue4/64KiB sowie Endblock erst nach Core-Return und tatsächlichem Worker-Join integriert. Cancellation/Fristen/Drop/Shutdown-Drain als Quellcode vorhanden. 35 zusätzliche Testfunktionen geschrieben (9 Store/10 Control/3 Writer/13 App-Download); 265 Compiler-Eingaben und 288 Pflichtnamen eingefroren, künftig mindestens 1070 Gesamttests. Tatsächliche Workspace-Formatprüfung und begrenzte unabhängige Quellreviews bestehen; Compiler, Clippy, Tests und native/API-/CLI-/MCP-/Browserläufe NICHT AUSGEFÜHRT. Physische neue64MiB/+1-Grenze, Selected-Untervarianten, große Quellen/RAM/Latenz/Recovery und sieben vollständige Filter bleiben offen. Keine S4-Vollabnahme oder Installation.');
for(const p of ['docs/sitemap-csv-streaming.md',proofPath]){assert(!work.documentation.includes(p));work.documentation.push(p);}
work.gates.source_large_processing_s4_csv_transport='WIP35_AUTHORED_FROZEN265_MIN1070_NAMED288_SAME_TX_TWOPASS_QUEUE4_64K_POSITIVE_TAIL_CORE_RETURN_JOIN_CANCEL_DRAIN_FMT_STATIC_REVIEW_COMPILE_TEST_NATIVE_NOT_RUN';
work.gates.source_large_processing_s4_completion='COMPACT_RICH_CONTEXT_APP_CSV_TRANSPORT_SOURCE_INTEGRATED_SELECTED_MATERIALIZING_PHYSICAL_NATIVE_UI_MEMORY_PLATFORM_NOT_PROVEN';
work.gates.source_large_processing_s4_csv_physical='OPEN_NEW_ROUTE_67108864_67108865_ACTUAL_PREVIOUS_AB1_PRODUCER_SEED_ONLY';
// Complete every deterministic prerequisite and final byte derivation before
// the first archive/proof/attributes/ledger mutation.
const receiptPath=__filename.replace(/\.cjs$/,'.json');assert(!fs.existsSync(receiptPath));
const proofBytes=Buffer.from(JSON.stringify(proof,null,2)+'\n'),attrBytes=Buffer.from(attr.trimEnd()+'\n'+rule+'\n'),ledgerBytes=Buffer.from(JSON.stringify(ledger,null,2)+'\n');
assert.equal(sha(fs.readFileSync(attrPath)),sha(attrBefore),'Attributes changed since preflight');assert.equal(sha(fs.readFileSync(ledgerPath)),sha(ledgerBefore),'Ledger changed since preflight');
assert(!fs.existsSync(path.join(repo,proofPath)));for(const e of items)assert(!fs.existsSync(path.join(repo,e.path)),'Fresh archive '+e.path);
for(const e of items){const dest=path.join(repo,e.path);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.writeFileSync(dest,e.bytes,{flag:'wx'});assert.equal(sha(fs.readFileSync(dest)),e.sha256);}
fs.writeFileSync(path.join(repo,proofPath),proofBytes,{flag:'wx'});
assert.equal(sha(fs.readFileSync(attrPath)),sha(attrBefore));fs.writeFileSync(attrPath,attrBytes);
assert.equal(sha(fs.readFileSync(ledgerPath)),sha(ledgerBefore));fs.writeFileSync(ledgerPath,ledgerBytes);
const generated=execFileSync(process.execPath,[path.join(repo,'scripts/update-feature-todo.mjs')],{cwd:repo}).toString(),checked=execFileSync(process.execPath,[path.join(repo,'scripts/update-feature-todo.mjs'),'--check'],{cwd:repo}).toString();
const receipt={created_at:new Date().toISOString(),proof_path:proofPath,proof_sha256:sha(fs.readFileSync(path.join(repo,proofPath))),archived_companions:items.length,files:265,new_authored_tests:35,new_tests_executed:0,minimum_expected_tests:1070,required_test_names:288,catalog_generated:generated.trim(),catalog_checked:checked.trim(),manufacturer_parity:'NOT_PROVEN',installed_runtime_modified:false};
fs.writeFileSync(receiptPath,JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(receipt));
