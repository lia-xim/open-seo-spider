'use strict';
// Finite original failed-CI publication proposal. Offline; Root alone may apply
// the exact reviewed plan. No tests, native/browser, Git or network execution.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const ROOT=path.resolve(__dirname,'..'),SELF='artifacts/materialize-project-columns-ci9816-failure-20261021.cjs';
const EARLY='artifacts/project-columns-renderer-failure-9816fe9-20261021';
const DEST='docs/verification/native-project-columns-20261021/ci-9816fe9-failure-20261021',COLLECTOR='artifacts/collect-project-columns-public-ci-9816fe9-20261021.cjs';
const COMMIT='9816fe9c9f77ea93ec6346eb46cbc8ff91458848',RUN=39654844791;
const EARLY_PINS=new Map([
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/run.json",
    "43dfc88a292c19db24f742698b6563ea793ac12e944324171457f1c21e6d6153"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/jobs.json",
    "00adaa0cb00a22e8179a11111beb2a639a992625b682fd976aa4b3e3d8ed8067"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/renderer.log",
    "4ddeb7d151fcf596b83ce070abc779807577e23abc293db2b8900f10c78d0b73"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/artifacts.json",
    "7eb0f5890573dccfa344b7d90e38094c357d26624c32ad72171a45eb2363c9d1"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/receipt.json",
    "d0b45305b3f055133e2b5d8c91266dbbe6d33d35728aefd2eec055869e1bba4c"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/registered-ui-e2e.mjs",
    "bf2d662143290342f3e6cb62fb97e9453fd38d64f33b792e921e578e3748862a"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/committed-ci.yml",
    "df0a6b71ec43bd3a649502cf332f0826b2ee4011499d0f9dbcd79721dc045f1f"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/redesign-ui-e2e.zip",
    "daa3aac32d666c901a2558cdf00495f900dc3545c5a6232291be1a18bd5dde60"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/unpacked/proof.json",
    "0e88c0bc342ac594fdf462ff63a128c3fe639cd4a4504156355be3f6b78064ce"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/unpacked/app.log",
    "b10d80ff0c18e6be312cd389db958e70bd03319f2b2c9ebaae463c4b06d4d970"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/unpacked/comparison-controls/stdout.log",
    "20377c1b2f3ed521208374eb541c4e5f065f33f8156107f028c2eb68ea820b18"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/unpacked/comparison-controls/stderr.log",
    "54936524c71f554433cebe4b70af6a721aa4f71793cb4ffdbf41edd86deec1fd"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/unpacked/comparison-controls/native-proof.json",
    "677fe4993a3b74630f83dcb5ad8fc9b58fe3c4a480db372e3ca305800b599aa9"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/diagnosis.json",
    "910cd5cecd6aa96f4898a24c6c3f1110c449b6407c8922e48f2e72aba376e914"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/committed-ui-saved-comparison-smoke.cjs",
    "5e37e8cf0e6069113590c8ef4fe123a169f520919c551c1e3e72379186fb9d49"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/ubuntu.log",
    "09004d331988865e5adae07d26c235fb87d916c4b0531467085cf0a4748f5dfb"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/jobs-ubuntu-terminal.json",
    "e7e64b58744011059e3e3c74e6830780a99a89e616e60beba467ed2efcf17cbc"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/committed-table_preferences_tests.rs",
    "81cca1ce4c54af8e8d51e73f28b24091ab6a576b940b2f96fcd572ddbc7519fa"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/ubuntu-diagnosis.json",
    "4a679c4c2375ddce7a69d3f704f982f6427d433502e5cc84c89c0be73ea659fb"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/bounded-run-snapshot.json",
    "43dfc88a292c19db24f742698b6563ea793ac12e944324171457f1c21e6d6153"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/bounded-jobs-snapshot.json",
    "ac60eedc3530f248a689777f770a85bb0be17e225349119141cff335003ed4c0"
  ],
  [
    "artifacts/project-columns-renderer-failure-9816fe9-20261021/container.log",
    "6ba1b91912413ec30565dab652fc3d2f3fa1853c9a95f335b7f9c75f102d296c"
  ]
]);
const JOBS=new Map([[119412266665,'renderer'],[119412267203,'container'],[119412267330,'test (ubuntu-latest)'],[119412267503,'test (windows-latest)']]);
const sha=b=>createHash('sha256').update(b).digest('hex');
function rel(v){assert(typeof v==='string'&&v&&v===v.replaceAll('\\','/')&&!path.isAbsolute(v)&&!/[\u0000-\u001f\u007f:]/u.test(v));assert(v.split('/').every(x=>x&&x!=='.'&&x!=='..'));return v;}
function abs(v){const p=path.resolve(ROOT,rel(v));assert(p.startsWith(ROOT+path.sep));return p;}
function links(v,missing=false){let p=ROOT;const a=rel(v).split('/');for(let i=0;i<a.length;i++){p=path.join(p,a[i]);let s;try{s=fs.lstatSync(p);}catch(e){if(missing&&e.code==='ENOENT')return;throw e;}assert(!s.isSymbolicLink(),'Link refused: '+v);assert(i===a.length-1||s.isDirectory());}}
class Pending extends Error{}
function main(){
 const args=process.argv.slice(2),apply=args[0]==='--apply';assert(!args.length||(args.length===1&&['--validate','--check-plan'].includes(args[0]))||(apply&&args.length===2&&/^[a-f0-9]{64}$/.test(args[1])));
 assert.equal(path.resolve(process.cwd()),ROOT);assert(fs.lstatSync(ROOT).isDirectory()&&!fs.lstatSync(ROOT).isSymbolicLink());links(DEST,true);assert(!fs.existsSync(abs(DEST)),'Fresh child required');
 links(path.posix.dirname(DEST));
 const inputs=new Map(),outputs=new Map(),copies=[];
 function read(file,pin){links(file);const s=fs.lstatSync(abs(file));assert(s.isFile()&&s.size<32*1024*1024);const b=fs.readFileSync(abs(file));if(pin)assert.equal(sha(b),pin,'Original changed: '+file);if(inputs.has(file))assert.deepEqual(b,inputs.get(file));else inputs.set(file,b);return b;}
 const json=file=>JSON.parse(read(file).toString('utf8'));
 function put(name,b){rel(name);assert(!outputs.has(name));outputs.set(name,Buffer.isBuffer(b)?b:Buffer.from(b));}
 function copy(file,category,pin){const b=read(file,pin),destination='raw/'+file;put(destination,b);copies.push({source:file,path:destination,bytes:b.length,sha256:sha(b),category});return b;}
 assert(read('.gitattributes').toString('utf8').includes('/docs/verification/native-project-columns-20261021/** -text'));
 for(const[file,pin]of EARLY_PINS)copy(file,'original-early-failure-snapshot-and-diagnosis',pin);
 const early=json(EARLY+'/run.json'),earlyJobs=json(EARLY+'/jobs.json').jobs,ui=json(EARLY+'/unpacked/proof.json'),fixture=json(EARLY+'/unpacked/comparison-controls/native-proof.json'),diag=json(EARLY+'/diagnosis.json'),unix=json(EARLY+'/ubuntu-diagnosis.json');
 assert.equal(early.id,RUN);assert.equal(early.head_sha,COMMIT);assert.equal(early.head_branch,'main');assert.equal(early.status,'in_progress','Original snapshot must retain its actual early status');assert.equal(earlyJobs.find(x=>x.id===119412266665).conclusion,'failure');assert.equal(ui.complete,false);assert.equal(ui.binary_unchanged,true);assert.equal(ui.tracked_sources_unchanged,true);assert.deepEqual(ui.shutdown,{code:0,signal:null});assert.equal(ui.results.length,9);assert.deepEqual(ui.results.map(x=>x.code),[0,0,0,0,0,0,0,0,1]);assert.equal(ui.results[8].name,'comparison-controls');assert.equal(ui.results[8].timed_out,false);assert.deepEqual(fixture.fixture_server_errors,['Unexpected GET /project-columns.js']);assert.deepEqual(fixture.javascript_runtime_errors,[]);assert.equal(diag.passed,8);assert.equal(diag.failed,1);assert.equal(diag.native_not_reached,41);
 const rawRegistry=read(EARLY+'/registered-ui-e2e.mjs','bf2d662143290342f3e6cb62fb97e9453fd38d64f33b792e921e578e3748862a'),registry=[...rawRegistry.toString().matchAll(/^\s+\['([a-z0-9-]+)',\s*'([^']+)',\s*'(fixture|live|owned-native)'/gm)].map(m=>({name:m[1],script:m[2],kind:m[3]}));assert.equal(registry.length,50);assert.deepEqual(ui.results.map(({name,script,kind})=>({name,script,kind})),registry.slice(0,9));assert.equal(registry.slice(9).filter(x=>x.kind!=='fixture').length,41);
 assert.equal(unix.result,'ORIGINAL_UBUNTU_APP_TEST_FAIL_RETAINED');assert.deepEqual(unix.App,{passed:551,failed:1,ignored:1});assert.equal(unix.source_line,751);const ubuntu=read(EARLY+'/ubuntu.log').toString();assert(ubuntu.includes('DirectoryNotEmpty')&&ubuntu.includes('table_preferences_tests.rs:751:59')&&ubuntu.includes('551 passed; 1 failed; 1 ignored'));assert.equal(json(EARLY+'/jobs-ubuntu-terminal.json').jobs.find(x=>x.id===119412267330).conclusion,'failure');
 const run=json(EARLY+'/bounded-run-snapshot.json'),response=json(EARLY+'/bounded-jobs-snapshot.json'),jobs=response.jobs;assert.equal(run.id,RUN);assert.equal(run.head_sha,COMMIT);assert.equal(run.head_branch,'main');assert.equal(run.status,'in_progress','This publication is explicitly the nonterminal bounded snapshot');assert.equal(run.conclusion,null);assert.equal(run.run_attempt,1);assert.equal(response.total_count,4);assert.equal(jobs.length,4);assert.equal(new Set(jobs.map(x=>x.id)).size,4);for(const job of jobs){assert.equal(JOBS.get(job.id),job.name);assert.equal(job.run_id,RUN);assert.equal(job.head_sha,COMMIT);}for(const id of[119412266665,119412267330]){assert.equal(jobs.find(x=>x.id===id).status,'completed');assert.equal(jobs.find(x=>x.id===id).conclusion,'failure');}assert.equal(jobs.find(x=>x.id===119412267203).status,'completed');assert.equal(jobs.find(x=>x.id===119412267203).conclusion,'success');assert.equal(jobs.find(x=>x.id===119412267503).status,'in_progress');assert.equal(jobs.find(x=>x.id===119412267503).conclusion,null);
 copy(COLLECTOR,'future-separate-read-only-terminal-collector-source-not-executed-here','cfcd81a9736bd04c27a4834c8d14b3911d2f6cb854c63e89f3bd4bc7f7db9e5b');copy(SELF,'offline-finite-early-failure-publisher');assert.equal(copies.length,24);
 const summary={kind:'bounded_early_exact9816_public_ci_failure_snapshots',result:'EARLY_RENDERER_AND_UBUNTU_FAIL_OVERALL_RUN_NONTERMINAL',commit:COMMIT,run:RUN,url:run.html_url,bounded_snapshot:{status:run.status,conclusion:run.conclusion,whole_run_terminal:false,jobs:jobs.map(({id,name,status,conclusion,started_at,completed_at,steps})=>({id,name,status,conclusion,started_at,completed_at,current_step:steps.find(x=>x.status==='in_progress')?.name}))},original_earlier_snapshot:{status:early.status,scope:'Preserved original metadata collected immediately after Renderer failure; it is not rewritten to the later bounded snapshot.'},UI:{passed_fixture_suites:8,failed_fixture_suite:'comparison-controls',failure:'Exact new /project-columns.js asset absent from the fixture allowlist; unexpected GET is retained, not ignored.',native_suites_not_reached:41,coordinator_EOF0:true},Ubuntu:{App:unix.App,test:unix.test,actual_error:unix.actual_error,source_contract_inference:unix.source_contract_inference},remaining_Windows:'IN_PROGRESS_AT_SNAPSHOT_NOT_CANCELLED_NOT_SUCCESS_NOT_TERMINAL_HERE',later_terminal_addendum:'NOT_PROVEN_HERE. A later separate collection may record terminal job/run states and any configured new-push cancellation; this early snapshot will remain unchanged.',scope:'Immutable main9816 has two actual failed jobs at this bounded snapshot. Container is terminalSUCCESS; Windows and the overall run remain in_progress with null conclusion. No combined whole-run terminal result, Windows cancellation, repaired local fixture/Unix test, full50 or successor publicCI PASS is inferred. Earlier bounded local R2/selected9R3 and immutable725 public49 remain separate. The original ZIP has fixture logs/receipts/PNGs/CSV and no runtime-store/executable members. Full144/416/32 backlog,39/68/37 states and manufacturer parity remain open.'};
 put('receipt.json',JSON.stringify(summary,null,2)+'\n');put('README.md','# Bounded early public9816 CI failure snapshots\n\nThis is an explicitly NONTERMINAL snapshot of immutable main `9816fe9c9f77ea93ec6346eb46cbc8ff91458848`, [run39654844791](https://github.com/lia-xim/open-seo-spider/actions/runs/39654844791). Renderer and Ubuntu have actual terminal failures. Container is terminal SUCCESS. Windows and the overall run remain IN_PROGRESS with no conclusion at the preserved bounded snapshot. No later cancellation or final whole-run result is invented.\n\nRenderer accepted the first8 API fixtures, then `comparison-controls` failed because its fixture allowlist did not serve the new `/project-columns.js` asset. All41 native suites were NOT_REACHED. Its original coordinator nevertheless closed naturally with EOF0; that does not make the failed invocation successful. The complete original667381-byte artifact ZIP, five selected original members and immutable fixture source are preserved.\n\nUbuntu independently failed App tests:551 passed,1 failed,1 ignored. The Unix nofollow negative fixture attempted to remove its directory while the intentionally persistent test-owned lock remained. Original raw logs, immutable test source and qualified source-level diagnosis are retained. No local Unix execution or repair acceptance is inferred.\n\nThe original early Renderer snapshot, later Ubuntu-failure snapshot and latest bounded run/jobs snapshot remain distinct, with their original declarations. [Receipt](receipt.json) and [index](index.json) bind the exact finite sources. A later terminal addendum must use a new collection and preserve these early bytes, including any actual configured new-push cancellation; the archived terminal collector is source only and has not supplied a terminal result here. No manual main cancellation is performed by this helper.\n\nCorrected local controls, fresh Rust gates, complete50 and successor publicCI require separate proof. Local bounded native-column R2, selected-nine R3 and predecessor725 public49 remain separately scoped. Full backlog and manufacturer parity remain open.\n');put('index.json',JSON.stringify({kind:'finite_bounded_early_failed_CI9816_evidence_index',raw_files:copies,generated_files:[...outputs].filter(([name])=>!name.startsWith('raw/')).map(([name,b])=>({path:name,bytes:b.length,sha256:sha(b)})),index_self_excluded:true,scope:'24 finite original raw copies. All snapshot timing/status declarations and source-level diagnoses remain separately labelled. No whole-run terminal claim, Windows cancellation inference, repaired-success inference or external/user runtime-store/executable copy.'},null,2)+'\n');
 const plan=[...outputs].map(([name,b])=>({path:DEST+'/'+name,bytes:b.length,sha256:sha(b)})).sort((a,b)=>a.path<b.path?-1:a.path>b.path?1:0),planSHA=sha(Buffer.from(JSON.stringify(plan))),report={result:apply?'MATERIALIZED_BOUNDED_EARLY_CI9816_FAILURE_SNAPSHOTS':'READY_VALIDATED_NO_WRITES',destination:DEST,raw_files:copies.length,generated_files:3,files:outputs.size,bytes:plan.reduce((n,x)=>n+x.bytes,0),plan_sha256:planSHA,index_sha256:sha(outputs.get('index.json')),publisher_sha256:sha(inputs.get(SELF))};if(!apply){console.log(JSON.stringify(report,null,2));return;}assert.equal(args[1],planSHA);for(const[file,b]of inputs){links(file);assert.deepEqual(fs.readFileSync(abs(file)),b);}links(DEST,true);assert(!fs.existsSync(abs(DEST)));fs.mkdirSync(abs(DEST));for(const[name,b]of outputs){const target=DEST+'/'+name,parent=path.posix.dirname(target);links(parent,true);fs.mkdirSync(abs(parent),{recursive:true});links(parent);fs.writeFileSync(abs(target),b,{flag:'wx'});assert.deepEqual(fs.readFileSync(abs(target)),b);}console.log(JSON.stringify(report,null,2));
}
try{main();}catch(e){console.error((e instanceof Pending?'PENDING_TERMINAL_MAIN_METADATA_NO_WRITES: ':'')+e.stack);process.exitCode=e instanceof Pending?2:1;}
