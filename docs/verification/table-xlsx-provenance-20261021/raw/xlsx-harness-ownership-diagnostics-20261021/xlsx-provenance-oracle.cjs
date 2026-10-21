'use strict';
// Independently authored fixture/source-record expectations. No product schema,
// projection, compiler/evaluator, CSV writer or XLSX writer is imported here.
const assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const PROFILE=Object.freeze({version:1,respect_noindex:true,respect_canonical:false,respect_next_prev:false});
const LABELS=Object.freeze({url:'URL',title_text:'Seitentitel',title_length:'Zeichen',title_pixels:'Pixelprofil 20px',title_count:'Title-Tags',duplicates:'Vorkommen',status:'HTTP final',initial_status:'HTTP initial',final_url:'Finale URL',redirects:'Redirects',response_ms:'Antwortzeit',indexable:'Indexierbarkeit',error:'Abruffehler'});
const PROVENANCE=Object.freeze(['Crawl Coverage','Observation Basis','Selected State','Parent Snapshot SHA256','Fetched At','Observation Requested URL']);
const SEGMENT=Object.freeze(['Segment Snapshot Token','Segment Rules SHA256','Segment Typed Report SHA256','Segment Generation','Segment Selection','Segment Reporting Identity','Original Page Ordinal']);
const REPORTING=Object.freeze(['Captured Pages','Visible Pages','Hidden Pages']);
const TITLE_KEYS=Object.freeze(['url','title_text','title_length','title_pixels','title_count','duplicates','status']);
const STATUS_KEYS=Object.freeze(['url','initial_status','status','final_url','redirects','response_ms','indexable','error']);
const PROJECTION=Object.freeze(['url','status','title_text','title_pixels','title_count','duplicates','title_length']);
const collator=new Intl.Collator('de-DE',{numeric:true,sensitivity:'base'});
const pathname=page=>new URL(page.url).pathname;
function profileCounts(source){
  assert.deepEqual(source.config.directive_reporting,PROFILE,'Fixture uses exactly one saved reporting profile');
  const hidden=source.pages.filter(page=>pathname(page)==='/products/hidden');assert.equal(hidden.length,1,'Exactly the captured declared noindex observation');
  for(const page of source.pages){const noindex=page.meta_robots.some(value=>value.toLowerCase().split(/[\s,]+/u).includes('noindex'));assert.equal(noindex,pathname(page)==='/products/hidden','Actual captured directive agrees with declared hidden fixture');}
  return{captured:source.pages.length,visible:source.pages.length-1,hidden:1};
}
function selectedOrdinals(source,selection,mode='reported'){
  assert(['reported','raw'].includes(mode));profileCounts(source);
  return source.pages.map((page,ordinal)=>({page,ordinal})).filter(({page})=>mode==='raw'||pathname(page)!=='/products/hidden').filter(({page})=>{
    const p=pathname(page),title=page.titles[0];
    if(selection.kind==='all')return true;
    if(selection.kind==='unknown')return title===undefined;
    if(selection.kind==='unassigned')return !/^\/products\/(?:\d{4}|hidden)$/u.test(p)&&!/^\/redirect-[ab]$/u.test(p)&&title!==undefined&&!title.includes('Product');
    assert.equal(selection.kind,'segment');
    if(selection.id==='s1')return /^\/products\/(?:\d{4}|hidden)$/u.test(p)||/^\/redirect-[ab]$/u.test(p);
    if(selection.id==='s2')return title!==undefined&&title.includes('Product');
    if(selection.id==='s3')return false;
    throw Error('Undeclared fixture segment '+selection.id);
  }).map(({ordinal})=>ordinal);
}
function declaration(value){if(value==='all'||value==='unknown'||value==='unassigned')return{kind:value};assert(value.startsWith('segment:'));return{kind:'segment',id:value.slice(8)};}
function contextExpected(capture,rules,selection,mode){
  assert(capture.canonicalBytes&&capture.detail.generation>0,'Genuine immutable native generation bytes required');
  assert.deepEqual(JSON.parse(capture.canonicalBytes),capture.report,'Canonical native generation content equals original API report');
  const typed=sha(capture.canonicalBytes),rulesHash=sha(Buffer.from(JSON.stringify(rules))),origin=capture.detail.checkpoint?'app_declared_job_report_with_cached_checkpoint':'app_declared_job_report_without_cached_checkpoint';
  const identity={engine:'open-seo-spider/saved-segments-reporting-v1',mode,directive_reporting:{...PROFILE}};
  const tuple=['saved_segments_v1',capture.id,capture.detail.generation,origin,typed,rulesHash,selection,identity];
  return{engine:'saved_segments_v1',job_id:capture.id,app_generation:capture.detail.generation,origin,typed_report_sha256:typed,rules_sha256:rulesHash,reporting_view:identity,snapshot_token:sha(Buffer.from(JSON.stringify(tuple))),sql_attested:false,report_generation_attested:false};
}
function checkSegment(capture,rules,selection,mode,pageReads,issueReads,analysis){
  assert(pageReads.length>0&&issueReads.length>0);const context=contextExpected(capture,rules,selection,mode),ordinals=selectedOrdinals(capture.report,selection,mode);
  const pages=pageReads.flatMap(value=>value.rows),cohort=pageReads[0].cohort;
  assert.deepEqual(pages.map(row=>row.page_ordinal),ordinals,'Native complete segment membership uses declared fixture truth, including original ordinals');
  assert.deepEqual(pages.map(row=>row.page),ordinals.map(ordinal=>capture.report.pages[ordinal]),'Every native segment PageRecord equals its immutable original observation');
  for(const read of[...pageReads,...issueReads,analysis]){assert.deepEqual(read.context,context,'Independent native generation/rules/profile/selection tuple binds every context cell');assert.deepEqual(read.cohort,cohort);}
  assert.deepEqual(cohort.selection,selection);assert.deepEqual(cohort.rules,rules);
  const counts=profileCounts(capture.report),eligible=mode==='raw'?counts.captured:counts.visible;
  assert.equal(cohort.reporting.captured_page_observations,counts.captured);assert.equal(cohort.reporting.profile_visible_page_observations,counts.visible);assert.equal(cohort.reporting.profile_excluded_page_observations,counts.hidden);
  assert.equal(cohort.reporting.eligible_page_observations,eligible);assert.equal(cohort.reporting.selected_page_observations,ordinals.length);assert.deepEqual(cohort.reporting.identity,context.reporting_view);
  assert.equal(cohort.summary.total_pages,eligible);assert.equal(cohort.summary.selected_pages,ordinals.length);
  const urls=new Set(ordinals.map(index=>capture.report.pages[index].url)),expectedIssues=capture.report.issues.filter(issue=>urls.has(issue.url));assert.deepEqual(issueReads.flatMap(value=>value.rows),expectedIssues);
  assert.equal(cohort.summary.selected_issues,expectedIssues.length);assert.deepEqual(analysis.captured_crawl_summary,capture.report.summary);
  return{context,selection,mode,ordinals,cohort,counts};
}
function rows(capture,section,view=null){
  assert(['titles','status'].includes(section));
  const ordinals=view?view.ordinals:selectedOrdinals(capture.report,{kind:'all'},'reported');
  const source=ordinals.map(ordinal=>({page:capture.report.pages[ordinal],ordinal})).filter(({page})=>page.internal&&(section!=='titles'||page.content_type.split(';')[0]==='text/html'));
  const duplicate=new Map();for(const{page}of source){const value=(page.titles[0]||'').trim();if(value)duplicate.set(value,(duplicate.get(value)||0)+1);}
  return source.map(({page,ordinal})=>section==='titles'?{url:page.url,title_text:page.titles.length?page.titles[0]:null,title_length:Array.from(page.titles[0]||'').length,title_pixels:page.title_pixel_widths[0]??null,title_count:page.titles.length,duplicates:duplicate.get((page.titles[0]||'').trim())||0,status:page.final_status??page.status,page,ordinal}:{url:page.url,initial_status:page.status,status:page.final_status??page.status,final_url:page.final_url,redirects:page.redirect_chain.length,response_ms:page.response_ms,indexable:page.indexable??null,error:page.error,page,ordinal});
}
function order(rows,key='url',direction=1){return[...rows].sort((a,b)=>{
  const rank=value=>value==null?2:typeof value==='string'&&!value.trim()?1:0,one=a[key],two=b[key],placement=rank(one)-rank(two);if(placement)return placement;
  const compare=rank(one)?0:typeof one==='number'&&typeof two==='number'?(one<two?-1:one>two?1:0)*direction:collator.compare(String(one).normalize('NFC'),String(two).normalize('NFC'))*direction;
  return compare||(a.url<b.url?-1:a.url>b.url?1:0)||a.ordinal-b.ordinal;
});}
function provenance(capture,row,section){
  const op=capture.report.operation,target=op?.targets.find(value=>value.requested_url===row.page.url),selected=op?.coverage==='mixed_age_selected_recrawl',state=selected&&target?target.state:'';
  assert.notEqual(state,'pending','Actual pending selected PageRecord must remain absent; historical target cannot masquerade as current');
  const basis=!op?'captured':!selected?'unknown_operation':state==='committed'?'refreshed_in_selected_job':'inherited_from_parent';
  return[op?.coverage??'crawl_snapshot',basis,state||'',op?.parent.snapshot_sha256??'',row.page.fetched_at,row.page.url];
}
function matrix(capture,orderedRows,keys,section,view=null){
  const counts=profileCounts(capture.report),header=[...keys.map(key=>LABELS[key]),...PROVENANCE,...view?SEGMENT:[],...REPORTING];
  const data=orderedRows.map(row=>[...keys.map(key=>row[key]??null),...provenance(capture,row,section),...view?[view.context.snapshot_token,view.context.rules_sha256,view.context.typed_report_sha256,view.context.app_generation,JSON.stringify(view.selection),JSON.stringify(view.context.reporting_view),row.ordinal]:[],counts.captured,counts.visible,counts.hidden]);
  return{header,rows:data};
}
function csvBytes(expected){
  const scalar=value=>{let text=String(value??'');if(typeof value!=='number'&&typeof value!=='boolean'&&/^[\s\uFEFF]*[=+@-]/u.test(text))text="'"+text;return'"'+text.replace(/"/g,'""')+'"';};
  return Buffer.from('\uFEFF'+[expected.header,...expected.rows].map((row,rowIndex)=>row.map((value,column)=>scalar(rowIndex&&expected.header[column]==='Indexierbarkeit'&&typeof value==='boolean'?(value?'Indexierbar':'Nicht indexierbar'):value)).join(',')).join('\r\n'));
}
function age(capture,row){const op=capture.report.operation;if(!op)return'';const target=op.targets.find(value=>value.requested_url===row.page.url);assert.notEqual(target?.state,'pending');return target?.state==='committed'?'Neu geprüft':'Beibehalten · vorheriger Abruf';}
function display(row,key){const value=row[key];if(key==='indexable')return value==null?'Unbekannt':value?'Indexierbar':'Nicht indexierbar';if(key==='title_text')return value??'';if(key==='response_ms')return Math.round(value).toLocaleString('de-DE')+' ms';if(key==='status'||key==='initial_status')return value==null?'—':String(value);return value==null?'—':typeof value==='number'?Number(value).toLocaleString('de-DE'):String(value);}
module.exports={PROFILE,LABELS,PROVENANCE,SEGMENT,REPORTING,TITLE_KEYS,STATUS_KEYS,PROJECTION,profileCounts,selectedOrdinals,declaration,contextExpected,checkSegment,rows,order,provenance,matrix,csvBytes,age,display};
