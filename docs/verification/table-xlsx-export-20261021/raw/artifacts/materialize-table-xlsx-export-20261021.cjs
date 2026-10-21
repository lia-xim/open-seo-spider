'use strict';
// ROOT-REVIEWED EVIDENCE COPY ONLY. Default validates in memory, with no writes.
// No child processes, Git, tests, native server, browser, network or source edits.
// R1/R2 focused FAILs remain FAILs; terminal focused R3 is a separate gate.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const ROOT=path.resolve(__dirname,'..'),DEST='docs/verification/table-xlsx-export-20261021';
const FINAL='artifacts/table-xlsx-final-inputs-20261021-r3.json';
const FINAL_SHA='1e9d5a289784fe1bdc018dfc611e421a782810d87e751ce211ba3c33a5a9eeb5';
const R1_FINAL='artifacts/table-xlsx-final-inputs-20261021.json';
const R1_FINAL_SHA='fd3960bd74c9d9335c65352e5347424aa149df3a8282c43004bbd1cd63a4a6d2';
const R2_FINAL='artifacts/table-xlsx-final-inputs-20261021-r2.json';
const R2_FINAL_SHA='f51aec60687110c0a5905cbd40379bb11d99d54abb83d45134e7132856b9dd71';
const BINARY_SHA='c2df6f1ce6699b8c91af9488b8c71a77f91b5991a1df9b7c1f6f76cb2a4aa72c';
const NATIVE='artifacts/table-xlsx-native-run-20261021-r1',FOCUSED='artifacts/table-xlsx-focused-e2e-20261021-';
const SUITES=['dashboard','pause-controls','recrawl-controls','policy-controls','content-controls','resource-controls','resource-ledger','resource-contract','comparison-controls','selected-recrawl','link-workflows','directory-tree','saved-segments','table-explorer','table-named-views','table-expressions','table-view-portability','table-xlsx'];
const sha=value=>createHash('sha256').update(value).digest('hex');
class Pending extends Error{}
function relative(value){
  assert.equal(typeof value,'string');assert(value.length&&value===value.replaceAll('\\','/')&&!path.isAbsolute(value)&&!/[\u0000-\u001f\u007f:]/u.test(value),'Unsafe relative path: '+value);
  assert(value.split('/').every(part=>part&&part!=='.'&&part!=='..'),'Unsafe relative segment: '+value);return value;
}
function absolute(value){const result=path.resolve(ROOT,relative(value));assert(result.startsWith(ROOT+path.sep),'Outside owned repository');return result;}
function noLinks(value,mustExist=true){
  const segments=relative(value).split('/');let current=ROOT;
  for(let i=0;i<segments.length;i++){
    current=path.join(current,segments[i]);
    if(!fs.existsSync(current)){assert(!mustExist,'Missing: '+value);return;}
    const stat=fs.lstatSync(current);assert(!stat.isSymbolicLink(),'No symlink/junction: '+value);
    assert(i===segments.length-1||stat.isDirectory(),'Non-directory parent: '+value);
  }
}
function repositoryRelative(value){
  if(!path.isAbsolute(value))return relative(value);
  const result=path.relative(ROOT,path.resolve(value)).replaceAll('\\','/');return relative(result);
}
function terminal(file){if(!fs.existsSync(absolute(file)))throw new Pending('Terminal focused R3 record is not present: '+file);}
function prepare(){
  assert.equal(path.resolve(process.cwd()),ROOT,'Run from the owned repository root');
  assert(fs.lstatSync(ROOT).isDirectory()&&!fs.lstatSync(ROOT).isSymbolicLink());
  assert(!fs.existsSync(absolute(DEST)),'Fresh destination only; existing directory will never be replaced');noLinks(DEST,false);
  // Static source/older immutable proofs can be checked now. The current R3
  // coordinator is never traversed until its terminal records both exist.
  const observed=new Map(),prepared=new Map(),copies=[],skipped=[];
  function read(file){
    noLinks(file);const full=absolute(file),stat=fs.lstatSync(full);assert(stat.isFile(),'Regular file required: '+file);
    assert(stat.size<=160*1024*1024,'Unexpected individual source size: '+file);
    const bytes=fs.readFileSync(full),digest=sha(bytes),prior=observed.get(file);
    if(prior)assert.equal(digest,prior.sha256,'Source changed during validation: '+file);
    else observed.set(file,{path:file,bytes:bytes.length,sha256:digest});return bytes;
  }
  function pin(file){read(file);return observed.get(file);}
  const json=file=>JSON.parse(read(file));
  function fixedJSON(file,digest){assert.equal(pin(file).sha256,digest,'Reviewed evidence SHA changed: '+file);return json(file);}
  function put(name,bytes){relative(name);assert(!prepared.has(name),'Duplicate output: '+name);prepared.set(name,Buffer.isBuffer(bytes)?bytes:Buffer.from(bytes));}
  const putJSON=(name,value)=>put(name,JSON.stringify(value,null,2)+'\n');
  function copy(file,category){const bytes=read(file),name='raw/'+file;put(name,bytes);copies.push({...pin(file),destination:name,category});}
  function flat(directory,names,category){for(const name of names)copy(directory+'/'+relative(name),category);}
  function sourceGuard(){
    const current=fixedJSON(FINAL,FINAL_SHA),prior=fixedJSON(R1_FINAL,R1_FINAL_SHA),r2=fixedJSON(R2_FINAL,R2_FINAL_SHA);
    assert.equal(current.selected_input_count,605);assert.equal(current.files.length,605);assert.equal(prior.files.length,605);assert.equal(r2.files.length,605);
    assert.deepEqual(current.files.map(row=>row.path),prior.files.map(row=>row.path),'Reviewed R3 selected path set');assert.deepEqual(current.files.map(row=>row.path),r2.files.map(row=>row.path));
    assert.equal(new Set(current.files.map(row=>row.path)).size,605);
    const changes=[];
    for(let i=0;i<current.files.length;i++){
      const item=current.files[i];assert.deepEqual(pin(item.path),item,'Current selected source drift: '+item.path);
      if(item.sha256!==r2.files[i].sha256)changes.push(item.path);
    }
    assert.deepEqual(changes,['.gitattributes','web/verification/ui-table-view-portability-live.cjs']);
    const oldDriver='artifacts/table-xlsx-focused-r1-original-portability-driver.cjs';
    assert.equal(pin(oldDriver).sha256,'2e23eb9c29813f79e968eae28df0c2ab48f19db4770aa025cd83f45271c44f2b');
    const r2Driver='artifacts/table-xlsx-focused-r2-portability-driver.cjs';assert.equal(pin(r2Driver).sha256,'b6801096d40739e94078209cb2d6f360eba7de1954a4e3ca58fa3d55f9b22822');
    const old=read(oldDriver).toString('utf8'),r2Source=read(r2Driver).toString('utf8');
    assert.equal(old.replace("assert.equal(assets.length,15,'Exactly the reviewed15 actual served assets');","assert.equal(assets.length,17,'Exactly the reviewed17 actual served assets');\nassert(assets.includes('table-xlsx.js')&&assets.includes('table-export-ui.js'));"),r2Source,'Exact R1 to R2 asset-only transform');
    const source=read('web/verification/ui-table-view-portability-live.cjs').toString('utf8');
    assert.equal(source,r2Source.replace('assert.equal(served.length,30)','assert.equal(served.length,assets.length*2)').replace('served15asset closure','served17asset closure'),'Exact R2 to R3 sum/diagnostic transform; data/download assertions unchanged');
    const attributes=read('.gitattributes').toString('utf8'),whitespace="\n# Preserve the reviewed native XLSX driver terminal blank lines.\n/web/verification/ui-table-xlsx-live.cjs whitespace=-blank-at-eof\n";
    assert.equal(attributes.split(whitespace).length,2,'Only exact new driver whitespace policy');assert.equal(sha(Buffer.from(attributes.replace(whitespace,''))),r2.files.find(row=>row.path==='.gitattributes').sha256);
    return{current,prior,r2,changes};
  }
  const source=sourceGuard();
  const binary=pin('target/debug/open-seo-spider.exe');assert.equal(binary.sha256,BINARY_SHA);assert.equal(binary.bytes,90249216);
  const build=json('artifacts/table-xlsx-build-gates-20261021-r1/proof.json');assert.equal(build.complete,true);assert.deepEqual(build.changed,[]);assert.equal(build.binary.sha256,BINARY_SHA);
  assert.deepEqual(build.results.map(result=>result.name),['format','build','app-tests','clippy']);assert(build.results.every(result=>result.code===0&&result.signal===null));
  const buildInputs=json('artifacts/table-xlsx-build-gates-20261021-r1/inputs.json'),currentByPath=new Map(source.current.files.map(row=>[row.path,row]));
  const buildDeltas=buildInputs.files.filter(row=>currentByPath.get(row.path)?.sha256!==row.sha256).map(row=>row.path);
  assert.deepEqual(buildDeltas,['.gitattributes','scripts/ui-e2e.mjs','web/verification/ui-table-view-portability-live.cjs'],'Only reviewed post-Cargo harness/whitespace corrections');
  const additions=source.current.files.filter(row=>!buildInputs.files.some(old=>old.path===row.path)).map(row=>row.path);
  assert.deepEqual(additions,['web/verification/ui-table-xlsx-live.cjs','web/verification/xlsx-package-oracle.cjs']);
  const pure=json('artifacts/table-xlsx-pure-gates-20261021-r1/proof.json');assert.equal(pure.complete,true);assert.equal(pure.passed,459);assert.deepEqual(pure.changed,[]);assert(pure.results.every(result=>result.code===0&&result.signal===null&&result.failed===0));
  for(const item of pure.sources)assert.equal(pin(item.file).sha256,item.sha256);
  const semantic=json('artifacts/table-xlsx-semantic-20261021-r1/proof.json');assert.equal(semantic.passed,true);assert.equal(semantic.groups,40);assert.equal(semantic.actual_browser,false);assert.equal(semantic.actual_engine,false);
  const controller=json('artifacts/table-export-controller-20261021/proof.json');assert.equal(controller.exitCode,0);assert.equal(controller.tests,35);assert.equal(controller.pass,35);assert.equal(controller.fail,0);
  for(const[file,digest]of Object.entries(controller.sourceHashes))assert.equal(pin(file).sha256,digest);
  const prototype=fixedJSON('artifacts/table-xlsx-export-draft-20261019/r1.proof.json','e1dc001728a3647b97c4d0b35d3411f349074585dbe52dd5b0ee6467598682a6');assert.equal(prototype.result,'FAIL');assert.equal(prototype.counts.pass,23);assert.equal(prototype.counts.fail,1);
  const firstCore=fixedJSON('artifacts/table-xlsx-product-pure-20261021-r1/proof.json','4422960b47173ac0baa71ecca75905d646ebb07d2e03588eb37b78c852766448');assert.equal(firstCore.result,'FAIL');assert.match(firstCore.error,/UnicodeEncodeError/);
  const core=fixedJSON('artifacts/table-xlsx-product-pure-20261021-r1/r2/proof.json','ddfcd85188bb0c6c20ea73c9210dda5b6a3ba4fccd2acfb6242ba105922ea594');assert.equal(core.result,'PASS_PURE_FORMAT_CONTROLS');assert.equal(core.tests.pass,27);assert.equal(core.tests.fail,0);
  for(const[file,digest]of [['review-proof-r1-harness-helper-fail.json','76a75e14a7f3bddf394f81781d96a105a3d3599fab0d144ffdd7e44bcb91d32a'],['review-proof-r2-harness-input-identity-fail.json','f0540212a554ea71678a37dd28e87942c999cce023bc86f4cc0ba578f3f95b38']])assert.equal(fixedJSON('artifacts/table-xlsx-app-review-20261021/'+file,digest).result,'FAIL');
  const appReview=json('artifacts/table-xlsx-app-review-20261021/review-proof.json');assert.equal(appReview.result,'PASS_PURE_APP_EXTRACTION_CONTROLS');assert.equal(appReview.csv_cases,124);assert.equal(appReview.typed_sections,29);assert.equal(appReview.current_sha256,pin('web/app.js').sha256);

  function focusedRecord(attempt,expectPass){
    if(expectPass){terminal(FOCUSED+attempt+'/proof.json');terminal('artifacts/table-xlsx-focused-invocation-20261021-'+attempt+'/invocation.json');}
    const directory=FOCUSED+attempt,record=json(directory+'/proof.json'),invocationDirectory='artifacts/table-xlsx-focused-invocation-20261021-'+attempt,invocation=json(invocationDirectory+'/invocation.json');
    assert.equal(record.complete,expectPass);assert.equal(invocation.complete,expectPass);assert.equal(invocation.code,expectPass?0:1);assert.equal(invocation.signal,null);assert.equal(invocation.timedOut,false);assert.deepEqual(invocation.selected,SUITES);
    assert.equal(record.binary_sha256,BINARY_SHA);assert.equal(record.binary_unchanged,true);assert.equal(record.tracked_sources_unchanged,true);assert.deepEqual(record.changed,[]);assert.deepEqual(record.shutdown,{code:0,signal:null});
    assert.deepEqual(record.results.map(row=>row.name),expectPass?SUITES:SUITES.slice(0,17));
    assert(record.results.slice(0,expectPass?18:16).every(row=>row.code===0&&row.signal===null&&!row.error&&!row.timed_out));
    if(!expectPass){const failed=record.results.at(-1);assert.equal(failed.name,'table-view-portability');assert.equal(failed.code,1);assert.equal(failed.signal,null);assert.equal(failed.timed_out,false);if(attempt==='r1'){assert.match(record.failure,/Exactly the reviewed15 actual served assets/);assert.match(record.failure,/17 !== 15/);}else assert.match(record.failure,/34 !== 30/);}
    copy(directory+'/proof.json','focused-'+attempt+'-coordinator');copy(directory+'/app.log','focused-'+attempt+'-coordinator');
    flat(invocationDirectory,['invocation.json','stdout.log','stderr.log'],'focused-'+attempt+'-invocation');return record;
  }
  focusedRecord('r1',false);focusedRecord('r2',false);
  const proofNames={dashboard:'ui-fixture-results.json','pause-controls':'ui-crawl-control-fixture-results.json','recrawl-controls':'ui-selected-recrawl-fixture-results.json','policy-controls':'ui-policy-fixture-results.json','content-controls':'ui-content-fixture-results.json','resource-controls':'ui-resource-discovery-fixture-results.json','resource-ledger':'ui-resource-ledger-fixture-results.json','resource-contract':'ui-resource-contract-fixture-results.json','comparison-controls':'native-proof.json','selected-recrawl':'ui-selected-recrawl-live-results.json','link-workflows':'link-workflows.json','directory-tree':'directory-tree-live.json','saved-segments':'segments-live.json'};
  const nested={
    'table-explorer':['table-explorer-','ui-table-explorer-live.json'],
    'table-named-views':['table-views-','ui-table-views-live.json'],
    'table-expressions':['table-expressions-','ui-table-expressions-live.json'],
    'table-view-portability':['table-view-portability-','ui-table-view-portability-live.json'],
    'table-xlsx':['table-xlsx-','ui-table-xlsx-live.json']
  };
  function oneChild(directory,prefix){
    noLinks(directory);const entries=fs.readdirSync(absolute(directory),{withFileTypes:true});
    const matches=entries.filter(item=>item.name.startsWith(prefix)&&item.isDirectory());assert.equal(matches.length,1,'One owned run directory: '+directory);
    const selected=directory+'/'+matches[0].name;noLinks(selected);return selected;
  }
  function olderSuiteCopies(attempt){for(const suite of SUITES.slice(0,17)){
    const directory=FOCUSED+attempt+'/'+suite;flat(directory,['stdout.log','stderr.log'],'focused-'+attempt+'-old-suite-logs');
    if(proofNames[suite])copy(directory+'/'+proofNames[suite],'focused-'+attempt+'-old-suite-JSON-only');
    else if(attempt==='r1'&&suite==='table-view-portability'){
      assert.match(read(directory+'/stderr.log').toString(),/Exactly the reviewed15 actual served assets/);
      const child=oneChild(directory,nested[suite][0]);assert(!fs.existsSync(absolute(child+'/'+nested[suite][1])),'R1 precondition failed before native proof');
      skipped.push({root:child,scope:'R1 portability failed asset precondition before native capture; no successful native receipt exists.'});
    }else{
      const child=oneChild(directory,nested[suite][0]);copy(child+'/'+nested[suite][1],'focused-'+attempt+'-old-suite-JSON-only');
      for(const phase of ['warm','cold']){const file=child+'/native-'+phase+'.log';if(fs.existsSync(absolute(file)))copy(file,'focused-'+attempt+'-old-suite-native-log');}
    }
    skipped.push({root:directory,scope:'Supplement retains child stdout/stderr and selected JSON proof, plus available native warm/cold logs. It omits legacy CSVs/images/project datasets and all stores; original proof references outside this supplement are not copied implicitly.'});
  }}
  olderSuiteCopies('r1');olderSuiteCopies('r2');

  function nativeCopy(directory,label,expectedReceiptHash){
    const receiptFile=directory+'/ui-table-xlsx-live.json';if(expectedReceiptHash)assert.equal(pin(receiptFile).sha256,expectedReceiptHash);
    const record=json(receiptFile);assert.equal(record.result,'PASS');assert.equal(record.binary_sha256,BINARY_SHA);assert.equal(record.workbooks.length,30);assert.equal(record.csv_pairs.length,30);
    assert.equal(record.downloads.filter(item=>item.source==='browser-event').length,60);assert.equal(record.prepare_races.length,8);assert.equal(record.geometry.length,12);
    assert.equal(record.native.length,2);assert(record.native.every(owner=>owner.natural_eof&&!owner.forced&&owner.close.code===0&&owner.close.signal===null));
    assert.equal(record.native[0].origin,record.native[1].origin);assert.notEqual(record.native[0].pid,record.native[1].pid);
    assert.deepEqual(record.offline_product_requests,[]);assert.equal(record.positive_410_controls.length,2);assert(record.positive_410_controls.every(item=>item.status===410));assert.deepEqual(record.errors,[]);assert.deepEqual(record.external_requests,[]);
    assert.equal(record.served_assets.length,34);assert.deepEqual(record.native_captures.map(item=>item.pages),[153,153,1]);
    const stats={workbooks:30,csv:30,rows:0,data_cells:0,headers:0};
    const csv=record.downloads.filter(item=>item.source==='actual-file');assert.equal(csv.length,30);
    const expectedFiles=new Set(['ui-table-xlsx-live.json','native-warm.log','native-cold.log','xlsx-responsive-1440.png','xlsx-responsive-320.png','declared-negative-corrupt-crc.xlsx']);
    copy(receiptFile,label+'-complete-native-receipt');
    for(const item of record.workbooks){
      const workbook=directory+'/'+item.label+'.xlsx';assert.equal(repositoryRelative(item.path),workbook);assert.equal(pin(workbook).sha256,item.sha256);assert.equal(pin(workbook).bytes,item.bytes);
      const expectedFile=directory+'/'+item.label+'.independent-expected.json',readerFile=directory+'/'+item.label+'.reader.json';
      const expectation=json(expectedFile),reader=json(readerFile);assert.deepEqual(reader.header,expectation.header);assert.deepEqual(reader.rows,expectation.rows);assert.equal(reader.matrix_sha256,item.matrix_sha256);assert.equal(reader.rows.length,item.rows);
      for(const suffix of ['.xlsx','.csv','.independent-expected.json','.reader.json']){const file=directory+'/'+item.label+suffix;copy(file,label+'-complete-native-raw');expectedFiles.add(item.label+suffix);}
      stats.rows+=item.rows;stats.headers+=item.columns.length;stats.data_cells+=item.rows*item.columns.length;
    }
    for(const item of csv){const file=repositoryRelative(item.path);assert.equal(path.posix.dirname(file),directory);assert.equal(pin(file).sha256,item.sha256);assert.equal(pin(file).bytes,item.bytes);}
    assert.deepEqual(stats,{workbooks:30,csv:30,rows:3020,data_cells:35652,headers:342});
    for(const capture of record.native_captures){assert.match(capture.id,/^[0-9a-f-]{36}$/u);const name=capture.id+'-original-project.json',file=directory+'/'+name;assert.equal(pin(file).sha256,capture.project_sha256);assert.equal(json(file).pages.length,capture.pages);copy(file,label+'-complete-original-owned-report');expectedFiles.add(name);}
    for(const phase of ['warm','cold'])copy(directory+'/native-'+phase+'.log',label+'-native-process-log');
    for(const size of [1440,320])copy(directory+'/xlsx-responsive-'+size+'.png',label+'-responsive-capture');
    copy(directory+'/declared-negative-corrupt-crc.xlsx',label+'-declared-negative-only');
    const children=fs.readdirSync(absolute(directory),{withFileTypes:true});
    assert.deepEqual(children.filter(item=>item.isFile()).map(item=>item.name).sort(),[...expectedFiles].sort(),'Explicit new XLSX artifact closure');
    assert(children.filter(item=>item.isDirectory()).every(item=>item.name==='store'),'Only declared owned store may be omitted');
    skipped.push({root:directory+'/store',scope:'All live/closed native storage internals, lock files and derived stores omitted. Original owned API project reports above are preserved byte-exactly.'});
    return{receipt:pin(receiptFile),stats};
  }
  const firstNative=nativeCopy(NATIVE+'/native','direct-r1','b74be7ac1178460c802c4553751146775e16b563375e9ff530cf17ea1d0165d0');
  const invocation=json(NATIVE+'/invocation.json');assert.equal(invocation.complete,true);assert.equal(invocation.code,0);assert.equal(invocation.timedOut,false);assert.equal(invocation.binary.sha256,BINARY_SHA);flat(NATIVE,['invocation.json','stdout.log','stderr.log'],'direct-r1-invocation');
  for(const attempt of ['r1','r2','r3'])skipped.push({root:FOCUSED+attempt+'/data',scope:'Coordinator app store/locks omitted; owned fixture JSON proofs and coordinator logs retained.'});
  const actualAudit=fixedJSON('artifacts/table-xlsx-actual-audit-20261021-r1/audit.json','4ac3bae740ddc146652ae7fdf71f7ba77c562f5f0267e9d95186fb6021922b7b');
  assert.equal(actualAudit.result,'PASS');assert.equal(actualAudit.audited_workbooks,30);assert.equal(actualAudit.reconstructed_data_rows,3020);assert.equal(actualAudit.physical_cells_checked,35994);assert.equal(actualAudit.original_evidence_sha256,firstNative.receipt.sha256);assert.equal(actualAudit.original_inputs_unchanged,true);

  // Fixed, finite authoring/evidence roots; no recursive filesystem discovery.
  flat('artifacts/table-xlsx-build-gates-20261021-r1',['inputs.json','progress.json','proof.json',...['format','build','app-tests','clippy'].flatMap(name=>[name+'.stdout.log',name+'.stderr.log'])],'fresh-Rust-build-gates');
  flat('artifacts/table-xlsx-pure-gates-20261021-r1',['proof.json',...Array.from({length:15},(_,i)=>[i+'.stdout.log',i+'.stderr.log']).flat()],'fresh-459-Node-tests');
  flat('artifacts/table-xlsx-semantic-20261021-r1',['proof.json','stdout.log','stderr.log'],'fresh-40-semantic-groups');
  flat('artifacts/table-export-controller-20261021',['proof.json','verify.cjs','node-test.stdout.txt','node-test.stderr.txt'],'controller-pure35');
  flat('artifacts/table-xlsx-actual-audit-20261021-r1',['audit.json','audit.py'],'independent-original-30-native-workbook-audit');
  flat('artifacts/table-xlsx-independent-review-20261021',['expected.json','generate.cjs','independent-cases.xlsx','python.stdout.txt','python.stderr.txt','review.json','verify.py'],'independent-core-review');
  flat('artifacts/table-xlsx-app-review-20261021',['native-candidate-static-review.json','review-proof-r1-harness-helper-fail.json','review-proof-r2-harness-input-identity-fail.json','review-proof.json','review-summary.json','review.cjs'],'App-and-native-candidate-review-with-failures');
  flat('artifacts/table-xlsx-native-draft-20261021',['check-oracle-offline.cjs','manifest.json','offline-package-check.json','plan.json','README.md','ui-table-xlsx-live.cjs','xlsx-package-oracle.cjs'],'native-preparation-original');
  flat('artifacts/table-xlsx-product-pure-20261021-r1',['edge-scalars-reader.stderr.log','edge-scalars-reader.stdout.log','edge-scalars.expected.json','edge-scalars.xlsx','first-node-run-expected-count-error.log','node-tests.stderr.log','node-tests.stdout.log','proof.json','run-proof.cjs','run-proof-r2.cjs'],'core-first-FAIL-retained');
  flat('artifacts/table-xlsx-product-pure-20261021-r1/r2',['proof.json','node-tests.stdout.log','node-tests.stderr.log','bad-crc.xlsx','bad-crc-reader.stdout.log','bad-crc-reader.stderr.log',...['edge-scalars','headers-only','wide-ordered'].flatMap(name=>[name+'.xlsx',name+'.expected.json',name+'.reader.json',name+'-reader.stdout.log',name+'-reader.stderr.log'])],'core-r2-pure27-actual-byte-PASS');
  flat('artifacts/table-xlsx-export-draft-20261019',['inspect_xlsx.py','r1.proof.json','r1.stderr.log','r1.stdout.log','regression-first.json','regression-first.stderr.log','regression-first.stdout.log','run-pure-proof.cjs','table-xlsx.cjs','table-xlsx.test.cjs'],'prototype-FAIL-original');
  flat('artifacts/table-xlsx-export-draft-20261019/actual-files',['corrupt-negative.xlsx',...['column-coordinates','empty-result','ordered-occurrences'].flatMap(name=>[name+'.xlsx',name+'.expected.json',name+'.reader.json']),...['scalars-and-escapes.xlsx','scalars-and-escapes.expected.json']],'prototype-original-raw-files');
  const helpers=['integrate-table-xlsx-20261021.cjs','integrate-table-xlsx-20261021-resume.cjs','promote-and-run-table-xlsx-20261021.cjs','record-table-xlsx-final-inputs-r2-20261021.cjs','record-table-xlsx-final-inputs-r3-20261021.cjs','run-table-xlsx-build-gates-20261021.cjs','run-table-xlsx-focused-20261021.cjs','run-table-xlsx-focused-20261021-r2.cjs','run-table-xlsx-focused-20261021-r3.cjs','run-table-xlsx-pure-20261021.cjs','table-xlsx-export-next-plan-20261019.json','table-xlsx-focused-r1-original-portability-driver.cjs','table-xlsx-focused-r2-portability-driver.cjs'];
  flat('artifacts',helpers,'original-source-plans-and-helpers');copy(FINAL,'selected-final-R3-source-manifest');copy(R1_FINAL,'historical-selected-R1-source-manifest');copy(R2_FINAL,'historical-selected-R2-source-manifest');
  copy(repositoryRelative(__filename),'materializer-original-helper');
  // Only now inspect R3. This lets default validation exercise all static source,
  // original direct XLSX, historical failures and finite preparation file paths
  // while a pending R3 returns PENDING without reading its mutable child trees.
  focusedRecord('r3',true);olderSuiteCopies('r3');
  const focusedXlsx=FOCUSED+'r3/table-xlsx';flat(focusedXlsx,['stdout.log','stderr.log'],'focused-r3-XLSX-child-log');
  const secondNative=nativeCopy(oneChild(focusedXlsx,nested['table-xlsx'][0]),'focused-r3');

  const summary={result:'PASS_LOCAL_MAIN_TABLE_XLSX_BOUNDED_ACCEPTANCE_WITH_RETAINED_R1_R2_FOCUSED_FAILURES',source_manifest:pin(FINAL),historical_R1_source_manifest:pin(R1_FINAL),historical_R2_source_manifest:pin(R2_FINAL),selected_inputs:605,normal_binary:binary,
    Rust:build.results,ordinary_Core_suite:'NOT_RUN_IN_THIS_SLICE',renderer_opt_ins:'NOT_RUN_IN_THIS_SLICE',Node_tests:459,semantic_groups:40,core_pure_tests:27,controller_pure_tests:35,App_CSV_exact_cases:124,App_typed_projection_sections:29,
    direct_native_R1:firstNative,focused_R1:{complete:false,passed:16,failed:1,not_reached:'table-xlsx',reason:'Old portability driver asserted15 assets although actualHTML has17. Failure occurred before portability native runtime; direct XLSX run is separate PASS.'},
    focused_R2:{complete:false,passed:16,failed:1,not_reached:'table-xlsx',reason:'Portability final two-start served-asset sum still expected30, actual34. Completed groups remain partial evidence; the final native and coordinator receipts stay FAIL.'},focused_R3:{complete:true,selected:18,passed:18,full_registered46:false,XLSX:secondNative},independent_original_actual_audit:{raw:pin('artifacts/table-xlsx-actual-audit-20261021-r1/audit.json'),workbooks:30,rows:3020,physical_cells:35994,openpyxl_version:actualAudit.openpyxl_version},
    post_Cargo_harness_changes:buildDeltas,post_Cargo_new_reviewed_harness_files:additions,
    qualification:'605 explicit selected source/harness inputs, not a transitive toolchain/dependency or whole metadata closure. Compiled Rust and embedded UI remain byte-identical to fresh Cargo gates. Later changes are one suite registration, exact reviewed portability asset count/presence then aggregate sum/diagnostic correction, and one driver-specific trailing-whitespace policy.',
    installed_application_updated:false,full_current_UI46:'NOT_RUN',exact_successor_public_CI:'NOT_PROVEN_BY_THIS_LOCAL_SUPPLEMENT',manufacturer_parity:'NOT_PROVEN'};
  const retained=[{path:'artifacts/table-xlsx-export-draft-20261019/r1.proof.json',result:'FAIL',reason:'Original overlapping Xstring escape writer defect;23/24 tests.'},{path:'artifacts/table-xlsx-product-pure-20261021-r1/proof.json',result:'FAIL',reason:'Python Unicode stdout cp1252 failure; portable Node27 passed; later explicit UTF8 R2 is separate PASS.'},{path:'artifacts/table-xlsx-product-pure-20261021-r1/first-node-run-expected-count-error.log',result:'FAIL',reason:'Original hand-counted expected scalar byte totals corrected in test; writer unchanged.'},{path:'artifacts/table-xlsx-app-review-20261021/review-proof-r1-harness-helper-fail.json',result:'FAIL',reason:'Reviewer extraction omitted accessibility helper.'},{path:'artifacts/table-xlsx-app-review-20261021/review-proof-r2-harness-input-identity-fail.json',result:'FAIL',reason:'Reviewer input identity lookup expected absent URL column.'},{path:FOCUSED+'r1/proof.json',result:'FAIL',reason:'Original focused18 attempt16PASS, portability asset preconditionFAIL, XLSX not reached.'},{path:FOCUSED+'r2/proof.json',result:'FAIL',reason:'Original focused18 R2 attempt16PASS, portability final served sum34!==30FAIL, XLSX not reached.'}].map(item=>({...item,raw:pin(item.path)}));
  putJSON('local-result.json',summary);putJSON('retained-failures.json',{retained,qualification:'Original byte-exact failure artifacts remain failures. Later successful receipts neither rewrite nor retroactively accept those attempts.'});
  putJSON('bounded-raw-scope.json',{complete_raw_new_XLSX_runs:[firstNative.receipt,secondNative.receipt],older_17_suites:'All three focused attempts retain coordinator/child logs and selected JSON proofs only; legacy datasets/raw downloads/images are not complete in this supplement. R2 failed native portability receipt remains FAIL.',omitted:skipped,not_proven:['All46 registered UI suites','Ordinary fullCore suite or renderer opt-ins in this slice','Other main-table native schemas','Selected-recrawl/segment native XLSX provenance','Specialist/resource/policy/content/sitemap/bulk or server-cursor export','Large-corpus performance/crash behavior','Excel/LibreOffice GUI and all browsers','Installed app upgrade','Exact future public CI','Manufacturer feature parity']});
  put('README.md',[
    '# Main-table XLSX: local evidence, 2026-10-21',
    'This is a byte-exact, indexed supplement for a bounded local main-table XLSX export slice. The 605-file selected source manifest is explicit, not a transitive dependency/toolchain or whole metadata closure. Fresh Cargo format/build/App tests/Clippy, 459 selected Node tests and 40 semantic groups are separate gates. Ordinary Core tests and renderer opt-ins were not rerun for this slice.',
    'The first direct native run passed 30 actual XLSX and 30 CSV downloads: 3,020 data rows, 35,652 data cells plus 342 header cells, 8 genuine DOM prepare races, 12 geometry observations, 2 natural native EOF0 processes, 2 positive HTTP410 witnesses and 0 offline product requests. An independent original-file Python/XML/openpyxl 3.1.5 audit checks all 35,994 physical cells; required ST_Xstring adaptation is explicitly reported and is not an Excel GUI roundtrip claim. The separate successful focused R3 suite has its complete new XLSX raw files too. See [local result](local-result.json).',
    '**Focused R1 and R2 remain FAIL: each 16 PASS, one portability failure, new XLSX not reached.** R1 expected 15 assets although reviewed HTML has 17 and failed before native capture. R2 required the correct 17 assets but its final two-start aggregate still expected 30; all 34 served bindings were present. R2 completed groups remain partial evidence, and its native/coordinator receipts stay FAIL. Original driver versions and all failure logs/proofs are preserved. R3 changes exactly the aggregate asset sum and diagnostic text; a separate driver-specific trailing-blank-line policy affects whitespace checking only. Data/download assertions, deadlines and compiled product bytes remain unchanged. R3 is accepted here only if its actual terminal 18-suite coordinator and invocation prove success. This is a selected 18-suite regression, not the full registered 46-suite run. See [retained failures](retained-failures.json).',
    'The two new XLSX native runs include complete actual CSV/XLSX files, expected matrices, independent readers, original owned API project reports, screenshots and native logs. For the older 17 focused suites this supplement keeps coordinator/child logs and selected JSON proofs, plus available native logs across all three attempts; their legacy raw datasets/downloads/screenshots are not copied. No store internals, locks, user data, binaries or browser runtime are included. [Exact raw scope](bounded-raw-scope.json) and [copy index](index.json) distinguish these boundaries. Original absolute paths in raw receipts are archival; external references are not implicitly included. Helpers are archived unchanged in their original form, not promised to run from this relocated raw folder.',
    'Prototype failures, the initial Unicode-stdout failure and reviewer harness failures are preserved byte-for-byte alongside later passes. No full manufacturer parity, all-table/operator acceptance, specialist/bulk export, large-corpus performance, all-browser or Excel GUI interoperability, installed application upgrade or exact future public CI is inferred.'
  ].join('\n\n')+'\n');
  const total=copies.reduce((sum,item)=>sum+item.bytes,0);assert(total<=128*1024*1024,'Explicit total public raw-byte budget');
  const generated=[...prepared].filter(([name])=>!name.startsWith('raw/')).map(([name,bytes])=>({path:name,bytes:bytes.length,sha256:sha(bytes)}));
  putJSON('index.json',{kind:'byte_exact_selected_XLSX_local_evidence_index',destination:DEST,materializer:pin(repositoryRelative(__filename)),source_manifest:pin(FINAL),copy_count:copies.length,total_raw_bytes:total,copies:[...copies].sort((a,b)=>a.destination.localeCompare(b.destination)),generated,scope:'Exact listed copies only. index.json and index.sha256 exclude themselves to avoid recursive hashes; index.sha256 independently hashes the index bytes.'});
  put('index.sha256',sha(prepared.get('index.json'))+'  index.json\n');
  // All source bytes have been read/checked before any possible destination write.
  function unchanged(){sourceGuard();for(const[file,item]of observed)assert.equal(sha(fs.readFileSync(absolute(file))),item.sha256,'Observed input drift: '+file);}
  unchanged();const files=[...prepared].map(([name,bytes])=>({path:name,bytes:bytes.length,sha256:sha(bytes)})).sort((a,b)=>a.path.localeCompare(b.path));
  const planSha=sha(Buffer.from(JSON.stringify({destination:DEST,files})));return{prepared,files,planSha,copies,summary,total,unchanged};
}
function main(){
  const args=process.argv.slice(2);
  if(args.length===1&&args[0]==='--help'){console.log('Usage: node artifacts/materialize-table-xlsx-export-20261021.cjs [--validate | --apply EXACT_VALIDATED_PLAN_SHA256]\nDefault --validate is read-only. Requires terminal focused R3 and pinned605-source manifest. --apply creates only the fresh docs/verification/table-xlsx-export-20261021 destination, never replaces/removes it.');return;}
  const apply=args[0]==='--apply';assert(args.length===0||args.length===1&&args[0]==='--validate'||apply&&args.length===2&&/^[a-f0-9]{64}$/u.test(args[1]),'Use --validate or --apply exact-plan-sha256');
  const plan=prepare();if(apply){
    assert.equal(args[1],plan.planSha,'Run --validate and review its exact SHA/list before --apply');plan.unchanged();noLinks(DEST,false);assert(!fs.existsSync(absolute(DEST)),'Fresh destination required');
    fs.mkdirSync(absolute(DEST));
    // Index/hash are written last. A failed partial copy has no success output.
    const order=[...plan.prepared.keys()].filter(name=>!['index.json','index.sha256'].includes(name)).concat(['index.json','index.sha256']);
    for(const name of order){const target=absolute(DEST+'/'+name);noLinks(DEST+'/'+name,false);fs.mkdirSync(path.dirname(target),{recursive:true});const parent=path.posix.dirname(name);noLinks(parent==='.'?DEST:DEST+'/'+parent);fs.writeFileSync(target,plan.prepared.get(name),{flag:'wx'});}
    for(const item of plan.files){noLinks(DEST+'/'+item.path);const bytes=fs.readFileSync(absolute(DEST+'/'+item.path));assert.equal(bytes.length,item.bytes);assert.equal(sha(bytes),item.sha256,'Destination mismatch: '+item.path);}plan.unchanged();
  }
  console.log(JSON.stringify({result:apply?'MATERIALIZED_BYTE_EXACT_NEW_DESTINATION':'VALIDATED_READ_ONLY_NO_WRITES',destination:DEST,plan_sha256:plan.planSha,source_manifest:{path:FINAL,sha256:FINAL_SHA,selected_inputs:605},copied_files:plan.copies.length,raw_bytes:plan.total,output_files:plan.files.length,focused_R1:'FAIL_RETAINED_16_PASS_PORTABILITY_PRECONDITION_FAIL_XLSX_NOT_REACHED',focused_R2:'FAIL_RETAINED_16_PASS_PORTABILITY_FINAL_SUM_FAIL_XLSX_NOT_REACHED',focused_R3:'PASS_SELECTED18_NOT_FULL46',docs_written:apply,files:plan.files},null,2));
}
if(require.main===module){try{main();}catch(error){console.error(JSON.stringify({result:error instanceof Pending?'PENDING_FOCUSED_R3_TERMINAL_NO_WRITES':'REFUSED_NO_SUCCESS_CLAIM',message:error.message,stack:error instanceof Pending?undefined:error.stack}));process.exitCode=error instanceof Pending?2:1;}}
module.exports=Object.freeze({prepare,DEST,FINAL,FINAL_SHA});
