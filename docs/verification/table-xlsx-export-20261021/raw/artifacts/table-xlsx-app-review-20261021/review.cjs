'use strict';
// Offline reviewer only. Execute extracted actual App functions with declared
// table-row providers; compare baseline bytes, not native website semantics.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict'),crypto=require('node:crypto'),{spawnSync}=require('node:child_process');
const root=path.resolve(__dirname,'../..'),hash=x=>crypto.createHash('sha256').update(x).digest('hex');
const before=spawnSync('git',['show','66dfe5c:web/app.js'],{cwd:root,encoding:'utf8',maxBuffer:4*1024*1024});assert.equal(before.status,0);
const after=fs.readFileSync(path.join(root,'web/app.js'),'utf8'),explorer=fs.readFileSync(path.join(root,'web/table-explorer.js'),'utf8'),xlsx=fs.readFileSync(path.join(root,'web/table-xlsx.js'),'utf8');
function between(source,start,end){const from=source.indexOf(start),to=source.indexOf(end,from);assert.ok(from>=0&&to>from,start);return source.slice(from,to);}
const sections=['overview','graph','internal','external','status','responses','inputs','titles','meta','headings','headingorder','keywords','linkdom','imagecandidates','navigation','fragments','canonical','robots','hreflang','images','imagefiles','pdf','issues','linkmetrics','structured','extractions','custom-search','javascript','cookies','accessibility','pagination'];
const dangerous=['plain','',null,'=SUM(A1:A2)','\t+1','\ufeff@A1','a,"b"\r\nnext','e\u0301 😀','_x005F_x0041_','-1','NFC é','last'];
const rows=dangerous.map((text,index)=>({url:'https://fixture.invalid/p/'+String(index).padStart(3,'0'),status:200,final_status:200,content_type:'text/html',indexable:index%3===0?null:index%2===0,
  titles:text===null?[]:[text],title_text:text??'',title_length:text?.length??0,title_pixel_widths:index===0?[]:[index===1?0:150.25],title_count:1,duplicates:2,
  meta_descriptions:text===null?[]:[text],description_text:text??'',description_length:text?.length??0,description_pixel_widths:[80],h1:text===null?[]:[text],h2:[],
  redirect_chain:[],final_url:'https://fixture.invalid/final',response_ms:index===0?null:index,depth:index,word_count:index,fetched_at:'2026-10-21T10:00:00Z',
  requested_url:'https://fixture.invalid/request/'+index,response_index:index,location:text,headers:index%2?{}:null,capture_kind:'terminal',body_bytes:0,body_complete:false,document_metadata_available:false,
  list_index:index,input_sha256:'declared-hash',source_index:index,source_row:index,source_line:index,sheet:'Input',cell:'A'+index,raw_value:text,normalized_url:'https://fixture.invalid/normalized',fragment:text,candidate_kind:'page_url',error:null,
  text,raw_text:text,text_with_image_alt:text,image_alt:[null,'',text],aria_level:'1',heading_offset:'0',hidden_attribute:false,dom_location:{index,path:'/html/body/x['+index+']'},dom_metadata:{source:'raw_html',complete:true,heading_text_engine:'declared'},tag_level:1,
  content:text,normalized:text,chars:text?.length??0,source_url:'https://fixture.invalid/source',kind:'anchor',raw_href:text,resolved_href:'https://fixture.invalid/target',position:null,
  image_src:'a.png',image_alt:text,source:'raw_html',raw_url:text,candidate_url:'https://fixture.invalid/a.png',descriptors:['1x'],width:0,height:0,density:1,media:text,type_hint:null,sizes:null,eligible:false,
  tag:'a',reason:text,onclick:text,router_link:text,potential_url:'https://fixture.invalid/potential',scheme:'https',source_document_url:'https://fixture.invalid/source',target_url:'https://fixture.invalid/target',decoded_fragment:text,target_document_url:null,target_complete:false,matched_target:null,
  canonical:[],html_canonical:[],header_canonical:[],blocked_by_robots:false,robots_warning:null,meta_robots:[],x_robots_tag:[],indexability_reasons:[],language:'de',hreflang:[],alt:text,loading:null,resource_status:'complete',intrinsic_width:0,intrinsic_height:null,
  format:'png',dimension_unit:'px',encoded_bytes:0,parse_error:null,pdf_title:text,author:text,page_count:0,declared_page_count:0,pdf_words:0,text_complete:false,annotation_count:0,annotations_complete:false,
  severity:'warning',code:'declared-code',message:text,related_urls:['https://fixture.invalid/related'],link_score:0.25,in_degree:0,out_degree:1,inlink_occurrences:0,outlink_occurrences:1,crawl_depth:0,shortest_link_depth:null,
  parsed_blocks:0,invalid_json_blocks:0,types_text:text,typed_nodes:[],name:text,method:'literal',output:null,result_type:'text',selector:text,attribute:null,values:[text??''],matched_count:0,truncated:false,
  rule_name:'declared-rule',count:0,matches_observed:0,matches:[],evidence_complete:false,reasons:[],value_type:'string',value_json:JSON.stringify(text),elapsed_ms:0,
  domain:'fixture.invalid',path:'/',sameSite:'Lax',httpOnly:false,secure:true,value_length:0,expires:-1,pagination_next:[],pagination_prev:[],amphtml:[],violations:null,incomplete:0,passes:0,inapplicable:0,axe_version:null,
  _segmentOrdinal:index,_unavailable:false}));
function realm(source,config){
  const context=vm.createContext({TextEncoder,TextDecoder,Blob,console});
  vm.runInContext(explorer+'\n'+xlsx,context);
  const glue=`
  const fixtureRows=${JSON.stringify(rows)};fixtureRows.forEach((row,index)=>{row._page=index===2?{url:'https://fixture.invalid/other-source'}:row;});
  const state={section:${JSON.stringify(config.section)},crawl:{id:'declared-owner',operation:${JSON.stringify(config.context?{coverage:'mixed_age_selected_recrawl',targets:[{requested_url:rows[0].url,state:'committed'},{requested_url:rows[1].url,state:'pending'}],parent:{snapshot_sha256:'parent-hash'}}:null)},reporting_visibility:${JSON.stringify(config.context?{captured_pages:24,visible_pages:12,hidden_pages:12}:null)}},sort:'url',direction:-1,search:'',pageMap:new Map(fixtureRows.map(row=>[row.url,row])),segmentView:${JSON.stringify(config.context?{context:{snapshot_token:'segment-token',rules_sha256:'rules-hash',typed_report_sha256:'typed-hash',app_generation:42,reporting_view:{kind:'declared'}},selection:{kind:'all'}}:null)}};
  const separateSegmentSections=new Set(['inputs','policy','policyaliases','content','resource-discovery','sitemap-membership','sitemap-findings']);
  const dashboardSegments=null,downloads=[],messages=[];
  const policySection=()=>false,contentSection=()=>false,sitemapSection=()=>false,findingsSection=()=>false;
  const pageIssues=url=>[],issueTitle=value=>'Label '+value;
  const getRows=()=>fixtureRows,matchesFilter=()=>true;
  const recrawlPage=row=>!row._page||row._page.url===row.url?(state.segmentView&&Number.isSafeInteger((row._page||row)._segmentOrdinal)?row._page||row:state.pageMap.get(row.url)):null;
  const downloadBlob=(blob,filename)=>downloads.push({blob,filename}),message=(...value)=>messages.push(value);
  const tableExplorer={canExport:()=>true,columns:schema=>{let keys=schema.map(c=>c.key);if(${JSON.stringify(config.projected)})keys=[...schema.filter(c=>c.mandatory).map(c=>c.key),...schema.filter(c=>!c.mandatory).reverse().filter((_,i)=>i%2===0).map(c=>c.key)];const out=TableExplorer.projectColumns(schema,keys);if(!out.ok)throw Error(JSON.stringify(out.error));return out.value;},rows:(rows,schema,search,sort)=>{const query=TableExplorer.compile(schema,{search,rules:[],sort,visibleKeys:schema.map(c=>c.key)},{identity:row=>row.url});if(!query.ok)throw Error(JSON.stringify(query.error));const out=query.value.rows(rows);if(!out.ok)throw Error(JSON.stringify(out.error));return out.value;}};
  `;
  const actual=between(source,'  const number =','  function customSearchRows()')+
    between(source,'  const accessibilityLabel=','  function getRows()')+
    between(source,'  const textColumn =','  function filterOptions()')+
    between(source,'  function tableSchema()','  function tableOwner()')+
    between(source,'  function filteredRows(','  function renderTable()')+
    (source.includes('  function captureTableProjection(')?between(source,'  function captureTableProjection(','  function tableExportAvailable()'):between(source,'  function exportCSV()','  async function serverDownload('));
  vm.runInContext(glue+'\n'+actual,context);return context;
}
(async()=>{
const proof={kind:'read_only_extracted_actual_App_CSV_regression',result:'RUNNING',baseline:'66dfe5c',baseline_sha256:hash(before.stdout),current_sha256:hash(after),node:process.version,
  boundary:'Extracted actual columns/tableSchema/filteredRows/exportCSV/capture functions using declared fixture row provider and actual TableExplorer compiler, no DOM/native/UI or claim about real native membership.',cases:[],typed:[]};
try{
  for(const section of sections)for(const context of [false,true])for(const projected of [false,true]){
    const config={section,context,projected},old=realm(before.stdout,config),now=realm(after,config);
    vm.runInContext('exportCSV()',old);vm.runInContext('exportCSV()',now);
    const oldDownloads=vm.runInContext('downloads',old),newDownloads=vm.runInContext('downloads',now);assert.equal(oldDownloads.length,1,section+' baseline');assert.equal(newDownloads.length,1,section+' current');
    const a=Buffer.from(await oldDownloads[0].blob.arrayBuffer()),b=Buffer.from(await newDownloads[0].blob.arrayBuffer());assert.deepEqual(a,b,JSON.stringify(config));assert.equal(oldDownloads[0].filename,newDownloads[0].filename);assert.equal(vm.runInContext('state.section',now),section);
    proof.cases.push({...config,bytes:b.length,sha256:hash(b),filename:newDownloads[0].filename});
  }
  for(const section of sections.filter(s=>!['overview','graph'].includes(s))){
    const context=realm(after,{section,context:true,projected:false});
    const projected=JSON.parse(vm.runInContext('JSON.stringify(captureTableProjection(true))',context));assert.equal(projected.ok,true,section);
    const prepared=vm.runInContext('(()=>{const captured=captureTableProjection(true),writer=TableXlsx.create();return writer.prepareTable({columns:captured.value.columns,rows:captured.value.rows,context:[]});})()',context);assert.equal(prepared.ok,true,section+': '+JSON.stringify(prepared.error));
    const keys=projected.value.columns.map(column=>column.key),row=key=>projected.value.rows.find(row=>keys.includes('url')?row[keys.indexOf('url')]===key:row[keys.indexOf('source_index')]===rows.findIndex(source=>source.url===key));
    if(section==='titles'){assert.equal(row(rows[0].url)[keys.indexOf('title_pixels')],null);assert.equal(row(rows[1].url)[keys.indexOf('title_pixels')],0);assert.equal(row(rows[1].url)[keys.indexOf('title_text')],'');assert.equal(row(rows[2].url)[keys.indexOf('title_text')],null);assert.equal(row(rows[3].url)[keys.indexOf('title_text')],'=SUM(A1:A2)');}
    if(section==='meta'){assert.equal(row(rows[1].url)[keys.indexOf('description_text')],'');assert.equal(row(rows[2].url)[keys.indexOf('description_text')],null);}
    if(section==='inputs'){assert.equal(row(rows[1].url)[keys.indexOf('raw_value')],'');assert.equal(row(rows[2].url)[keys.indexOf('raw_value')],null);}
    proof.typed.push({section,rows:projected.value.rows.length,columns:keys.length,prepared:true});
  }
  proof.result='PASS_PURE_APP_EXTRACTION_CONTROLS';proof.csv_cases=proof.cases.length;proof.typed_sections=proof.typed.length;
}catch(error){proof.result='FAIL';proof.error=error.stack;process.exitCode=1;}
proof.finished_at=new Date().toISOString();fs.writeFileSync(path.join(__dirname,'review-proof.json'),JSON.stringify(proof,null,2)+'\n');console.log(JSON.stringify({result:proof.result,csv_cases:proof.csv_cases,typed_sections:proof.typed_sections,error:proof.error}));
})();
