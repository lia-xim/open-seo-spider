import json, hashlib
from pathlib import Path
BASE=Path('E:/CodexWork/open-seo-spider-01a12ac3-20261013')
OLD=BASE/'s6-native-browser-preparation-v1'
NEW=BASE/'s6-native-browser-postfix-v1'
sha=lambda b:hashlib.sha256(b).hexdigest()
assert sha((OLD/'manifest.json').read_bytes())=='71ed85b6baf20f9bb5f0c1103a4351d06cc44bd3cae49fe94db489bcf71e6e51'
manifest=json.loads((OLD/'manifest.json').read_bytes())
for f in manifest['files']:
    data=(OLD/f['path']).read_bytes()
    assert len(data)==f['bytes'] and sha(data)==f['sha256']
review=BASE/'s6-native-browser-review-v1/receipt.json'
assert sha(review.read_bytes())=='b8189980c0c49cf495924cb81f3216a99a6702ea7f3b0fbcccba388a56318fb8'
files=['preflight.mjs','browser-flow.mjs','run-sitemap-findings-browser-native-v1.mjs']
contents={f:(OLD/f).read_bytes().decode('utf-8') for f in files}
ops=[]
def change(f,before,after):
    nl='\r\n' if '\r\n' in contents[f] else '\n'
    before=before.strip('\n').replace('\n',nl)
    after=after.strip('\n').replace('\n',nl)
    assert contents[f].count(before)==1,(f,before[:120],contents[f].count(before))
    contents[f]=contents[f].replace(before,after)
    ops.append(dict(path=f,expected_occurrences=1,before=before,after=after))
P='preflight.mjs';F='browser-flow.mjs';D='run-sitemap-findings-browser-native-v1.mjs'
change(P,"  const suitePath=path.join(BASE,'results/sitemap-findings-surfaces-suite-v1/proof.json')",r"""
  // Historical private source publication; not current GitHEAD or installed runtime acceptance.
  const publicationPath=path.join(TASK,'work/sitemap-findings-surfaces-published-v1.json');
  const publicationSha='b71188d145a8e720bd9d6c9aaf4720c03c20cced7f406839264e5432e85e84ff';
  const publicationRaw=hash(publicationPath,publicationSha,1024*1024),publication=JSON.parse(publicationRaw);
  assert.equal(publication.result,'PUSHED_PRIVATE_SOURCE_ONLY_GOAL_ACTIVE');assert.equal(publication.commit,'f43efc17d1393645a9eb8c8b8510743f3dfedb5c');assert.equal(publication.source_manifest_sha256,SOURCE_SHA);
  assert.equal(publication.repository.isPrivate,true);assert.equal(publication.repository.nameWithOwner,'lia-xim/open-seo-spider');assert.equal(publication.frozen_inputs,297);assert.equal(publication.approved_product_bindings,38);assert.equal(publication.new_authored_rust_tests,47);assert.equal(publication.executed_new_rust_tests,0);assert.equal(publication.ui_mock_groups_authored,13);assert.equal(publication.executed_ui_groups,0);assert.equal(publication.compiler_clippy_native_browser_build,'NOT_RUN_COORDINATED_COMPUTE_HOLD');
  const publicationLineage=Object.freeze({path:publicationPath,sha256:publicationSha,commit:publication.commit,source_manifest_sha256:SOURCE_SHA,repository_private:true,checkpoint_result:publication.result,checkpoint_execution:publication.compiler_clippy_native_browser_build});
  const suitePath=path.join(BASE,'results/sitemap-findings-surfaces-suite-v1/proof.json')
""")
change(P,'return {mode,output,binary,','return {publicationPath,publicationSha,publicationLineage,mode,output,binary,')
change(D,'// All active imports, output creation, listening, SQLite access and process spawn are below verified admission.',r"""
// All active imports, output creation, listening, SQLite access and process spawn are below verified admission.
const executionStartedNs=process.hrtime.bigint(),EXECUTION_DEADLINE_MS=600000,EXECUTION_DEADLINE_NS=BigInt(EXECUTION_DEADLINE_MS)*1000000n;
const closeElapsed=()=>String(process.hrtime.bigint()-executionStartedNs);
""")
change(D,'complete=false,siteClosed=false;','complete=false,siteClosed=false,completionClock=null;')
change(D,"const deadline=setTimeout(()=>abort('Native browser driver 600s cooperative deadline'),600000);","const deadline=setTimeout(()=>abort('Native browser driver 600s cooperative deadline'),EXECUTION_DEADLINE_MS);")
change(D,'item.stdout_closed=true;resolve();','item.stdout_closed=true;item.stdout_close_elapsed_nanoseconds=closeElapsed();resolve();')
change(D,'item.stderr_closed=true;resolve();','item.stderr_closed=true;item.stderr_close_elapsed_nanoseconds=closeElapsed();resolve();')
change(D,'Object.assign(item,{actual_close_event:true,code,signal});','Object.assign(item,{actual_close_event:true,code,signal,process_close_elapsed_nanoseconds:closeElapsed()});')
change(D,'  clearTimeout(deadline);let browserCloseError;','  let browserCloseError;')
change(D,"  process.off('message',onMessage);process.off('SIGINT',onSigint);process.off('SIGTERM',onSigterm);",r"""
  // Freeze completion after held process closes AND actual writer-pipe drain.
  // A due-but-undelivered timer cannot make a >=600s completion pass.
  const closedClocks=owned.flatMap(item=>[item.process_close_elapsed_nanoseconds,item.stdout_close_elapsed_nanoseconds,item.stderr_close_elapsed_nanoseconds]);
  const allExpectedDrained=owned.length===2&&owned.every(item=>item.actual_close_event&&item.stdout_closed&&item.stderr_closed)&&closedClocks.every(value=>typeof value==='string');
  const lastActualCloseNs=closedClocks.filter(value=>typeof value==='string').reduce((latest,value)=>BigInt(value)>latest?BigInt(value):latest,0n),drainObservedNs=process.hrtime.bigint()-executionStartedNs;
  completionClock=Object.freeze({profile:'monotonic_actual_process_and_writer_drain_v1',deadline_nanoseconds:String(EXECUTION_DEADLINE_NS),last_actual_process_or_pipe_close_elapsed_nanoseconds:String(lastActualCloseNs),all_drains_observed_elapsed_nanoseconds:String(drainObservedNs),all_expected_processes_and_pipes_closed:allExpectedDrained,fixture_closed:siteClosed,within_deadline:allExpectedDrained&&siteClosed&&lastActualCloseNs<EXECUTION_DEADLINE_NS&&drainObservedNs<EXECUTION_DEADLINE_NS});
  clearTimeout(deadline);
  if(!completionClock.within_deadline)failures.push({phase:'monotonic_completion_deadline',message:'Actual process/writer drain incomplete or observed at/after600s',clock:completionClock});
  process.off('message',onMessage);process.off('SIGINT',onSigint);process.off('SIGTERM',onSigterm);
""")
change(D,'try{assert.equal(sha(await fs.readFile(admission.binary)),admission.binarySha);','try{assert.equal(sha(await fs.readFile(admission.publicationPath)),admission.publicationSha);assert.equal(sha(await fs.readFile(admission.binary)),admission.binarySha);')
change(D,'result:complete&&allClosed&&siteClosed&&failures.length===0?','result:complete&&allClosed&&siteClosed&&completionClock?.within_deadline===true&&failures.length===0?')
change(D,'source_manifest_sha256:admission.sourceSha,preparation_manifest_sha256:','source_publication:admission.publicationLineage,actual_close_clock:completionClock,source_manifest_sha256:admission.sourceSha,preparation_manifest_sha256:')
change(D,'expectedAssets,configSha:sqliteEvidence.actual_saved_config_sha256','expectedAssets,configSha:sqliteEvidence.actual_saved_config_sha256,expectedSeedUrls:config.seed_urls')
change(F,'quiet,expectedAssets,configSha}=env;','quiet,expectedAssets,configSha,expectedSeedUrls}=env;')
change(F,r"""
    async function collect(query){let cursor=null,context=null,rows=[],steps=0;do{const response=(await post(findingPath,{...query,limit:200,candidate_limit:5000,...(cursor?{cursor}:{})})).json;assert(response.findings&&Array.isArray(response.findings.rows));const current=response.findings.context.context_sha256;if(context)assert.equal(current,context);context=current;rows.push(...response.findings.rows);cursor=response.next_cursor;assert(++steps<=10);}while(cursor);return rows;}
""",r"""
    const collectedContexts=new Map();
    async function collect(query){let cursor=null,context=null,rows=[],steps=0;const cursors=new Set();do{const response=(await post(findingPath,{...query,limit:200,candidate_limit:5000,...(cursor?{cursor}:{})})).json;assert.equal(response.access_target.job_id,job.id);assert.equal(response.access_target.generation,job.generation);assert.equal(response.access_target.config_sha256,configSha);assert.equal(response.website_requests,0);assert(response.findings&&Array.isArray(response.findings.rows));if(context)assert.deepEqual(response.findings.context,context);context=response.findings.context;rows.push(...response.findings.rows);cursor=response.next_cursor;if(cursor){assert(!cursors.has(cursor),'Repeated API continuation');cursors.add(cursor);}else{assert.equal(response.findings.scan.exhausted,true);assert.equal(response.findings.scan.stop,'exhausted');}assert(++steps<=10);}while(cursor);collectedContexts.set(JSON.stringify(query),context);return rows;}
    assert.equal(expectedSeedUrls.length,405);assert.equal(new Set(expectedSeedUrls).size,405);
    function assertCompleteMatrix(filter,rows){
      const source=filter.startsWith('xml-'),expected=source?[siteOrigin+'/a.xml',siteOrigin+'/b.xml']:expectedSeedUrls;
      assert.equal(rows.length,expected.length);assert.equal(new Set(rows.map(identity)).size,expected.length,'Every original occurs once in actual API matrix');assert.deepEqual(rows.map(identity).slice().sort(),expected.slice().sort(),'Exact actual fixture original inventory');
      assert.equal(new Set(rows.map(row=>JSON.stringify(row.position))).size,expected.length,'Every candidate position unique');
      if(!source){for(const [ordinal,row] of rows.entries()){assert.equal(row.evidence.domain,'saved_page');assert.deepEqual(row.position,{kind:'saved_page',ordinal});}if(matrix['urls-in-sitemap'])assert.deepEqual(rows.map(identity),matrix['urls-in-sitemap'].map(identity),'Same report ordinal identifies same original across filters');}
      else for(const row of rows){assert.equal(row.evidence.domain,'source_attempt');assert.equal(row.position.kind,'source_measurement');assert(Number.isSafeInteger(row.position.sequence)&&row.position.sequence>0);}
    }
""")
change(F,"const query=selection(filter),all=await collect(query);assert.equal(all.length,filter.startsWith('xml-')?2:405);","const query=selection(filter),all=await collect(query);assertCompleteMatrix(filter,all);")
change(F,r"""
    const csvSelection=selection('urls-not-in-sitemap','match'),shown=await configure(csvSelection);assert.equal(shown.body.findings.rows.length,1);const exportRequest={...csvSelection,snapshot:shown.body.export_token};
""",r"""
    const csvSelection=selection('urls-not-in-sitemap','match'),matchingApi=await collect(csvSelection);assert.deepEqual(matchingApi,matrix['urls-not-in-sitemap'].filter(row=>row.assessment.verdict==='match'),'Actual match API traversal equals complete all-assessment matrix');assert.equal(matchingApi.length,403);assert.equal(new Set(matchingApi.map(identity)).size,403);assert.equal(new Set(matchingApi.map(row=>JSON.stringify(row.position))).size,403);
    const shown=await configure(csvSelection);assert.equal(shown.body.findings.rows.length,1);assert.deepEqual(shown.body.findings.context,collectedContexts.get(JSON.stringify(csvSelection)));const exportRequest={...csvSelection,snapshot:shown.body.export_token};
""")
change(F,r"""
const csvMetadata=JSON.parse(parsed[1][9]);assert.equal(csvMetadata.context.basis.crawl_config_sha256,configSha);assert.equal(csvMetadata.context.context_sha256,shown.body.findings.context.context_sha256);assert.deepEqual(csvMetadata.selection,csvSelection);const data=parsed.slice(2);assert.equal(data.length,403);assert(data.every(row=>row.length===10&&row[0]==='finding'));for(const row of data){const finding=JSON.parse(row[8]);assert.equal(finding.assessment.verdict,'match');assert.equal(finding.evidence.domain,'saved_page');assert.deepEqual(JSON.parse(row[6]),finding.position);}assert(data.some(row=>identity(JSON.parse(row[8]))===farUrl));
""",r"""
const csvMetadata=JSON.parse(parsed[1][9]);assert.equal(parsed[1].length,10);assert.deepEqual(parsed[1].slice(0,9),['context','1','saved_page',csvSelection.filter,'match','','','','']);assert.deepEqual(csvMetadata,{engine:'open-seo-spider/app-sitemap-findings-v1',selection:csvSelection,context:shown.body.findings.context},'Full actual CSV metadata/context with no missing/extra facts');const data=parsed.slice(2);assert.equal(data.length,matchingApi.length);for(const [index,row] of data.entries()){const expected=matchingApi[index];assert.equal(row.length,10);assert.deepEqual(row.slice(0,6),['finding','1','saved_page',csvSelection.filter,'match',identity(expected)],'Every human cell and exact original in real API order');assert.deepEqual(JSON.parse(row[6]),expected.position);assert.deepEqual(JSON.parse(row[7]),expected.assessment.reasons);assert.deepEqual(JSON.parse(row[8]),expected,'Every full typed CSV row equals corresponding genuine API row');assert.equal(row[9],'','No repeated context on data rows');}assert(data.some(row=>identity(JSON.parse(row[8]))===farUrl));
""")
# Every replacement checked before any payload is written. Preserve all other bytes.
guards=[]
for f in files:
    data=contents[f].encode('utf-8')
    with (NEW/f).open('xb') as out: out.write(data)
    guards.append(dict(path=f,before_path=str(OLD/f),before_sha256=sha((OLD/f).read_bytes()),after_sha256=sha(data),after_bytes=len(data)))
for f in files:
    inverse=contents[f]
    for op in reversed(ops):
        if op['path']==f:
            assert inverse.count(op['after'])==1
            inverse=inverse.replace(op['after'],op['before'])
    assert inverse.encode('utf-8')==(OLD/f).read_bytes()
derivation=dict(schema_version=1,basis_manifest_sha256='71ed85b6baf20f9bb5f0c1103a4351d06cc44bd3cae49fe94db489bcf71e6e51',review_receipt_sha256='b8189980c0c49cf495924cb81f3216a99a6702ea7f3b0fbcccba388a56318fb8',operations=ops,payload_guards=guards,forward_replay='EXACT',inverse_reconstruction='EXACT_ORIGINAL_BYTES',scope='Three reviewed browser-source issues only; original694/f43; no UIe9 or runtime acceptance')
with (NEW/'DERIVATION.json').open('x',encoding='utf-8',newline='\n') as out:json.dump(derivation,out,indent=2);out.write('\n')
print(json.dumps(dict(operation_count=len(ops),payloads=guards)))