import assert from 'node:assert/strict';
import path from 'node:path';
import fs from 'node:fs/promises';
import {createHash} from 'node:crypto';

const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const id=suffix=>'#sitemap-findings-'+suffix;
const filters=['urls-in-sitemap','urls-not-in-sitemap','orphan-urls','non-indexable-urls-in-sitemap','urls-in-multiple-sitemaps','xml-sitemap-with-over-50k-urls','xml-sitemap-over-50mb'];
export function parseCsv(text){
  const rows=[];let row=[],cell='',quoted=false,afterQuote=false;
  for(let i=0;i<text.length;i++){
    const c=text[i];
    if(quoted){if(c==='"'){if(text[i+1]==='"'){cell+='"';i++;}else{quoted=false;afterQuote=true;}}else cell+=c;continue;}
    if(c==='"'){assert(cell===''&&!afterQuote,'Unexpected CSV quote');quoted=true;continue;}
    if(c===','){row.push(cell);cell='';afterQuote=false;continue;}
    if(c==='\r'){assert.equal(text[++i],'\n');row.push(cell);rows.push(row);row=[];cell='';afterQuote=false;continue;}
    assert(!afterQuote&&c!=='\n','Unexpected CSV suffix');cell+=c;
  }
  assert(!quoted&&row.length===0&&cell==='','CSV must have complete CRLF EOF');return rows;
}
function selection(filter,assessment='all',requested_url=null,search=null){return {filter,assessment,requested_url,source_task_url:null,search};}
function identity(row){return row.evidence.domain==='saved_page'?row.evidence.page.finding.observation.requested_url:row.evidence.source.kind==='measured'?row.evidence.source.record.measurement.payload.binding.requested_task_url:row.evidence.source.kind==='legacy_document'?row.evidence.source.document.task_url:row.evidence.source.task.task_url;}

// Actual browser only: never route.fulfill, evaluate product state, or manufacture an API row.
export async function runNativeFindingsBrowser(env){
  const {browser,api,job,siteOrigin,output,post,read,checkpoint,record,quiet,expectedAssets,configSha}=env;
  const context=await browser.newContext({acceptDownloads:true,serviceWorkers:'block',viewport:{width:1440,height:1000}});context.setDefaultTimeout(30000);
  const blocked=[],requests=[],downloadEvents=[],errors=[];let page,closed=false;
  const findingPath=`/api/crawls/${job.id}/sitemaps/findings`,csvPath=findingPath+'/export.csv';
  await context.route('**/*',async route=>{
    const request=route.request(),url=new URL(request.url()),method=request.method();
    const asset=Object.hasOwn(expectedAssets,url.pathname),jobRead=url.pathname===`/api/crawls/${job.id}`||url.pathname.startsWith(`/api/crawls/${job.id}/`);
    const allowed=url.origin===api&&((method==='GET'&&(asset||url.pathname==='/favicon.ico'||url.pathname==='/healthz'||url.pathname==='/api/crawls'||jobRead))||(method==='POST'&&[findingPath,csvPath].includes(url.pathname)));
    const item={url:url.href,method,allowed};requests.push(item);
    if(!allowed){blocked.push(item);return route.abort('blockedbyclient');}return route.continue();
  });
  try{
    page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));page.on('download',download=>downloadEvents.push(download));
    for(const [route,expected] of Object.entries(expectedAssets)){const response=await read(route);assert.equal(sha(response.bytes),expected,route+' served binary-embedded asset must equal frozen source');}
    await page.goto(api+'/',{waitUntil:'networkidle'});await page.locator('#crawl-history option[value="'+job.id+'"]').waitFor({state:'attached'});await page.selectOption('#crawl-history',job.id);
    await page.locator('[data-section="sitemap-findings"]').click();await page.locator(id('view')).waitFor({state:'visible'});
    async function settled(){await page.waitForFunction(()=>document.getElementById('sitemap-findings-view').getAttribute('aria-busy')==='false');await quiet();assert.equal(await page.locator(id('error')).isVisible(),false,await page.locator(id('error')).textContent());checkpoint();}
    async function capture(trigger){
      const pending=page.waitForResponse(r=>r.url()===api+findingPath&&r.request().method()==='POST');
      await trigger();const response=await pending;assert.equal(response.status(),200);const request=JSON.parse(response.request().postData()),body=await response.json();await settled();
      const oracle=(await post(findingPath,request)).json;assert.equal(body.access_target.config_sha256,configSha);assert.equal(body.access_target.generation,job.generation);assert.equal(body.access_target.job_id,job.id);assert.deepEqual(body,oracle,'UI HTTP body must equal independent actual App read under same immutable context');assert(body.findings&&Array.isArray(body.findings.rows));
      assert.equal(await page.locator(id('table')+' [data-finding-row]').count(),body.findings.rows.length);return {request,body};
    }
    async function configure(query,limit=1,candidates=5000){
      // Only normal DOM control values change; one genuine form submit invokes the production handler.
      await page.locator(id('form')).evaluate((form,{query,limit,candidates})=>{
        for(const [key,value] of Object.entries({filter:query.filter,assessment:query.assessment,exact:query.requested_url??query.source_task_url??'',search:query.search??'',limit:String(limit),candidates:String(candidates)})){const element=form.querySelector('#sitemap-findings-'+key);element.value=value;assertDom(element.value===value,key);}
        function assertDom(ok,name){if(!ok)throw Error('Unsupported actual DOM option '+name);}
      },{query,limit,candidates});
      return capture(()=>page.locator(id('form')+' button[type="submit"]').click());
    }
    async function inspect(row,keyboard='Enter'){
      const item=page.locator(id('table')+' [data-finding-row="0"]');await item.focus();await item.press(keyboard);await page.locator(id('inspector')).waitFor({state:'visible'});
      const actual=JSON.parse(await page.locator(id('inspector-data')).textContent());assert.deepEqual(actual,row);assert.equal(await page.locator(id('inspector-heading')).evaluate(el=>el===document.activeElement),true);
      await page.locator(id('inspector-close')).click();assert.equal(await item.evaluate(el=>el===document.activeElement),true);return actual;
    }
    await settled();assert.equal(await page.locator('#export-csv').isDisabled(),true,'Generic visible-row CSV is disabled for findings');
    // Query all candidates through real paginated App reads, not a seeded UI/DTO oracle.
    async function collect(query){let cursor=null,context=null,rows=[],steps=0;do{const response=(await post(findingPath,{...query,limit:200,candidate_limit:5000,...(cursor?{cursor}:{})})).json;assert(response.findings&&Array.isArray(response.findings.rows));const current=response.findings.context.context_sha256;if(context)assert.equal(current,context);context=current;rows.push(...response.findings.rows);cursor=response.next_cursor;assert(++steps<=10);}while(cursor);return rows;}
    const matrix={};
    for(const filter of filters){
      checkpoint();const query=selection(filter),all=await collect(query);assert.equal(all.length,filter.startsWith('xml-')?2:405);
      const current=await configure(query);assert.equal(current.body.findings.rows.length,1);assert.deepEqual(current.body.findings.rows[0],all[0]);await inspect(all[0],filter.startsWith('xml-')?'Space':'Enter');
      matrix[filter]=all;record('actual_filter_ui_api_'+filter,{rows:all.length,verdicts:Object.fromEntries(['match','not_matched','unknown'].map(v=>[v,all.filter(r=>r.assessment.verdict===v).length])),first_position:all[0].position});
    }
    const verdict=(filter,url)=>matrix[filter].find(row=>identity(row)===url)?.assessment.verdict;
    assert.equal(verdict('urls-in-sitemap',siteOrigin+'/p0'),'match');assert.equal(verdict('urls-in-sitemap',siteOrigin+'/p1'),'match');assert.equal(verdict('urls-in-sitemap',siteOrigin+'/p404'),'not_matched');
    assert.equal(verdict('urls-not-in-sitemap',siteOrigin+'/p404'),'match');assert.equal(verdict('orphan-urls',siteOrigin+'/p0'),'match');assert.equal(verdict('orphan-urls',siteOrigin+'/p1'),'not_matched');
    assert.equal(verdict('non-indexable-urls-in-sitemap',siteOrigin+'/p0'),'match');assert.equal(verdict('non-indexable-urls-in-sitemap',siteOrigin+'/p1'),'not_matched');
    for(const p of ['p0','p1'])assert.equal(verdict('urls-in-multiple-sitemaps',siteOrigin+'/'+p),'match');
    for(const filter of filters.filter(x=>x.startsWith('xml-')))for(const row of matrix[filter]){assert.equal(row.assessment.verdict,'not_matched');assert.equal(row.evidence.source.kind,'measured');assert(row.evidence.source.association,'Actual exact durable B association');}
    record('actual_fixture_truth_controls',{saved_pages:405,listed_pages:2,source_documents:2,p0_noindex:true,p1_anchor_non_orphan:true,source_threshold_cases:'negative_controls_only'});
    // One-row pagination must resume and return to the exact preceding real row.
    const normal=await configure(selection('urls-in-sitemap'));assert(normal.body.next_cursor);assert.equal(await page.locator(id('next')).isEnabled(),true);
    const second=await capture(()=>page.locator(id('next')).click());assert.notDeepEqual(second.body.findings.rows[0].position,normal.body.findings.rows[0].position);
    const back=await capture(()=>page.locator(id('prev')).click());assert.deepEqual(back.body,normal.body);record('actual_one_row_next_back',{first:normal.body.findings.rows[0].position,second:second.body.findings.rows[0].position});
    // An actual last page identity lies beyond candidate #400. Choose it from the verified oracle.
    const far=matrix['urls-in-sitemap'].slice(400).reverse().find(row=>row.assessment.verdict==='not_matched');assert(far,'A real unlisted original exists beyond ordinal400');const farUrl=identity(far);assert(farUrl.startsWith(siteOrigin+'/p'));const exact=await configure(selection('urls-in-sitemap','all',farUrl));assert.equal(exact.body.findings.rows.length,1);assert.deepEqual(exact.body.findings.rows[0],far);await inspect(far);
    record('actual_original_identity_beyond_400',{ordinal:far.position,url:farUrl});
    const sparse=await configure(selection('urls-in-sitemap','all',farUrl),1,1);assert.deepEqual(sparse.body.findings.rows,[]);assert.equal(sparse.body.findings.scan.stop,'candidate_budget');assert(sparse.body.next_cursor);assert.equal(await page.locator(id('next')).isEnabled(),true);
    const sparseNext=await capture(()=>page.locator(id('next')).click());assert.deepEqual(sparseNext.body.findings.rows,[]);const sparseBack=await capture(()=>page.locator(id('prev')).click());assert.deepEqual(sparseBack.body,sparse.body);record('actual_sparse_candidate_one_next_back',{first_scan:sparse.body.findings.scan,second_scan:sparseNext.body.findings.scan});
    // Every assessment selector uses the genuine HTTP producer, including a genuine evaluated empty unknown slice.
    for(const assessment of ['match','not_matched','unknown']){const item=await configure(selection('urls-in-sitemap',assessment));for(const row of item.body.findings.rows)assert.equal(row.assessment.verdict,assessment);record('actual_assessment_selector_'+assessment,{rows:item.body.findings.rows.length,scan:item.body.findings.scan});}
    // Full saved-selection CSV (403 rows) from an on-screen one-row window.
    const csvSelection=selection('urls-not-in-sitemap','match'),shown=await configure(csvSelection);assert.equal(shown.body.findings.rows.length,1);const exportRequest={...csvSelection,snapshot:shown.body.export_token};
    const direct=await post(csvPath,exportRequest,{csv:true});assert.equal(direct.status,200);assert.match(direct.headers['content-type'],/^text\/csv/);assert.equal(Number(direct.headers['content-length']),direct.bytes.length);assert.equal(direct.headers['x-sitemap-csv-sha256'],sha(direct.bytes));
    const parsed=parseCsv(direct.bytes.toString('utf8'));assert.deepEqual(parsed[0],['record_type','schema_version','domain','filter','assessment','original_identity','position_json','reasons_json','typed_finding_json','context_json']);assert.equal(parsed[1][0],'context');const csvMetadata=JSON.parse(parsed[1][9]);assert.equal(csvMetadata.context.basis.crawl_config_sha256,configSha);assert.equal(csvMetadata.context.context_sha256,shown.body.findings.context.context_sha256);assert.deepEqual(csvMetadata.selection,csvSelection);const data=parsed.slice(2);assert.equal(data.length,403);assert(data.every(row=>row.length===10&&row[0]==='finding'));for(const row of data){const finding=JSON.parse(row[8]);assert.equal(finding.assessment.verdict,'match');assert.equal(finding.evidence.domain,'saved_page');assert.deepEqual(JSON.parse(row[6]),finding.position);}assert(data.some(row=>identity(JSON.parse(row[8]))===farUrl));
    const csvResponse=page.waitForResponse(r=>r.url()===api+csvPath&&r.request().method()==='POST'),downloadPromise=page.waitForEvent('download');await page.locator(id('export')).click();const [response,download]=await Promise.all([csvResponse,downloadPromise]);assert.equal(response.status(),200);assert.deepEqual(JSON.parse(response.request().postData()),exportRequest);assert.equal(await download.failure(),null);assert.equal(download.suggestedFilename(),`open-seo-spider-${job.id}-sitemap-findings.csv`);assert(download.url().startsWith('blob:'+api+'/'));
    const downloadPath=path.join(output,'ui-full-findings.csv');await download.saveAs(downloadPath);const downloaded=await fs.readFile(downloadPath);assert.deepEqual(downloaded,direct.bytes);const uiHeaders=await response.allHeaders();assert.equal(Number(uiHeaders['content-length']),downloaded.length);assert.equal(uiHeaders['x-sitemap-csv-sha256'],sha(downloaded));assert.equal(uiHeaders['x-sitemap-findings-context-sha256'],shown.body.findings.context.context_sha256);assert.equal(uiHeaders['x-sitemap-findings-selection-sha256'],shown.body.findings.context.selection_sha256);assert.equal(downloadEvents.length,1);record('actual_full_csv_browser_download',{data_rows:data.length,visible_rows:1,bytes:downloaded.length,sha256:sha(downloaded),api_sha256:sha(direct.bytes),header_sha256:direct.headers['x-sitemap-csv-sha256'],suggested_filename:download.suggestedFilename(),body_eof_and_api_bytes_equal:true});
    // Native real 400 no-file path: malformed control selection reaches actual server validation.
    const beforeDownloads=downloadEvents.length;await page.locator(id('exact')).fill('x'.repeat(65537));const bad=page.waitForResponse(r=>r.url()===api+findingPath&&r.request().method()==='POST');await page.locator(id('form')+' button[type="submit"]').click();const badResponse=await bad;assert.equal(badResponse.status(),400);await page.locator(id('error')).waitFor({state:'visible'});assert.equal(await page.locator(id('export')).isDisabled(),true);await quiet();assert.equal(downloadEvents.length,beforeDownloads);record('actual_invalid_selector_400_no_download',{status:400,downloads_unchanged:true});
    await configure(selection('urls-in-sitemap'));
    for(const width of [1440,320]){await page.setViewportSize({width,height:900});await page.locator(id('export')).scrollIntoViewIfNeeded();const dimensions=await page.evaluate(()=>({viewport:innerWidth,document:document.documentElement.scrollWidth,body:document.body.scrollWidth}));assert(dimensions.document<=width+1&&dimensions.body<=width+1,'Page must not overflow viewport');await page.locator('.main-footer').scrollIntoViewIfNeeded();assert(await page.locator('.main-footer').isVisible());await page.screenshot({path:path.join(output,`browser-${width}-bottom.png`),fullPage:true});await page.locator(id('table')+' [data-finding-row="0"]').scrollIntoViewIfNeeded();await inspect((await post(findingPath,{...selection('urls-in-sitemap'),limit:1,candidate_limit:5000})).json.findings.rows[0]);record('actual_responsive_'+width,{...dimensions,footer_reached:true,keyboard_inspector:true});}
    assert.deepEqual(blocked,[],'No unintended browser mutation or external route');assert.deepEqual(errors,[],'No browser pageerror');await fs.writeFile(path.join(output,'browser-requests.json'),JSON.stringify(requests,null,2)+'\n',{flag:'wx'});
    return {actual_ui:true,mocked_api:false,groups:env.records.length,requests:requests.length,downloads:downloadEvents.length,blocked,errors,physical_positive_source_limits:'NOT_RUN_SEPARATE_GATE',unknown_source_matrix:'NOT_RUN_SEPARATE_GATE'};
  }finally{await context.close();closed=true;env.contextClosed=closed;}
}