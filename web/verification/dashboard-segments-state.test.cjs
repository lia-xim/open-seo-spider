'use strict';
// Actual production controller with deferred native-shaped responses. Native browser proof is separate.
const test=require('node:test'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const {createController,validateRules}=require('../dashboard-segments.js');
const digest=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
const rules={version:1,definitions:[{id:'owned',name:'Owned',join:'all',conditions:[{field:'requested_url',operator:'contains',value:'/owned/',case_insensitive:false}]}]};
const copy=value=>JSON.parse(JSON.stringify(value));
const context={engine:'saved_segments_v1',job_id:'A',app_generation:7,origin:'app_declared_saved_report',typed_report_sha256:'a'.repeat(64),rules_sha256:'b'.repeat(64),snapshot_token:'c'.repeat(64),sql_attested:false,report_generation_attested:false};
const page=index=>({page_ordinal:index+500,membership:{matched_indices:[0],unknown_indices:[]},page:{url:`https://owned.invalid/owned/${String(index).padStart(4,'0')}`,final_url:'https://owned.invalid/document',status:200,internal:true,titles:[`Title ${index}`]}});
const issue={url:page(200).page.url,code:'missing_description',severity:'warning',message:'Stored issue',related_urls:['https://outside.invalid/']};
const cohort={engine:'saved_segments_v1',rules,selection:{kind:'segment',id:'owned'},summary:{total_pages:250,selected_pages:201,selected_issues:1,internal_pages:201,html_pages:201,indexable_pages:201,failed_pages:0,multiple_matches:0,unassigned_pages:49,unknown_pages:0,orphan_issues:0,issue_counts:{error:0,warning:1,notice:0}},counts:[{id:'owned',matched:201,unknown:0}],charts:{status:{200:201},depth:{0:201},effective_status:{200:201},overview:{success:201,redirect:0,client_error:0,server_error:0,unfetched:0}}};
function payload(view,offset=0){const rows=view==='pages'?Array.from({length:201},(_,index)=>page(index)):[issue];const end=Math.min(rows.length,offset+200);return{context:copy(context),cohort:copy(cohort),captured_crawl_summary:{pages_fetched:250},view,total:rows.length,rows:rows.slice(offset,end),next:end<rows.length?{offset:end,view,snapshot_token:context.snapshot_token}:null};}
function analysis(){return{context:copy(context),cohort:copy(cohort),captured_crawl_summary:{pages_fetched:250},analysis:{graph:{nodes:[],edges:[]},issues:[]},target_lookup:[{page_ordinal:249,url:'https://outside.invalid/',image_metadata:{width:200}}],responses:[],fragments:{enabled:false,observations:[],unknown_is_not_missing:true}};}
function response(data,options={}){const bytes=Buffer.from(typeof data==='string'?data:JSON.stringify(data));return new Response(bytes,{status:options.status??200,headers:{'content-type':options.csv?'text/csv; charset=utf-8':'application/json; charset=utf-8','content-length':String(bytes.length),'x-content-sha256':digest(bytes),'cache-control':'no-store','x-segment-snapshot-token':context.snapshot_token,'x-segment-generation':'7',...options.headers}});}
function harness(){let owner={id:'A',nativeGeneration:7,uiGeneration:9,stable:true};const pending=[],installs=[],downloads=[],events=[];let clears=0;const controller=createController({owner:()=>owner,crypto:crypto.webcrypto,fetch:(url,options)=>new Promise((resolve,reject)=>pending.push({url,options,body:JSON.parse(options.body),resolve,reject})),onInstall:value=>installs.push(value),onClear:()=>clears++,onDownload:(...args)=>downloads.push(args),onChange:s=>events.push({busy:s.busy,applied:!!s.applied,error:s.error})});controller.configure(rules,'segment:owned','reported');return{controller,pending,installs,downloads,events,setOwner:value=>owner=value,get clears(){return clears;}};}
async function tick(){await new Promise(resolve=>setImmediate(resolve));}
async function deliver(h,index,data,options){const until=Date.now()+3000;while(!h.pending[index]&&Date.now()<until){await tick();}assert(h.pending[index],`request ${index}: ${h.controller.state.error}`);h.pending[index].resolve(response(data,options));await tick();}
async function complete(h){const operation=h.controller.apply();await deliver(h,0,payload('pages'));assert.equal(h.installs.length,0);await deliver(h,1,payload('pages',200));assert.equal(h.installs.length,0);await deliver(h,2,payload('issues'));assert.equal(h.installs.length,0);await deliver(h,3,analysis());await operation;return h.installs[0];}

test('complete 200+1 pages and issues install atomically with raw ordinals, original facts, target context and exact frozen requests',async()=>{
 const h=harness(),result=await complete(h);assert(result);assert.equal(h.installs.length,1);assert.deepEqual(result.pages.map(row=>row.page_ordinal),Array.from({length:201},(_,i)=>i+500));assert.deepEqual(result.issues,[issue]);assert.equal(result.target_lookup[0].image_metadata.width,200);assert.equal(result.cohort.summary.selected_pages,201);assert.equal(h.controller.state.busy,false);
 assert.equal(h.pending.length,4);for(const request of h.pending){assert.equal(request.options.method,'POST');assert.equal(request.body.reporting_mode,'reported');assert.equal(request.body.expected_generation,7);assert.deepEqual(request.body.rules,rules);}assert.equal(h.pending[0].body.expected_snapshot_token,null);assert.equal(h.pending[1].body.cursor.offset,200);assert.equal(h.pending[2].body.view,'issues');assert(h.pending[3].url.endsWith('/segments/dashboard-analysis'));assert.equal(h.pending[3].body.expected_snapshot_token,context.snapshot_token);
});
test('invalid saved definition cannot leave an old valid draft actionable',async()=>{
 const h=harness();assert.throws(()=>h.controller.configure({...rules,definitions:[...rules.definitions,...rules.definitions]},'segment:owned',''));assert.equal(h.controller.state.draftValid,false);await h.controller.apply();assert.equal(h.pending.length,0);assert.match(h.controller.state.error,/Ungültige/);assert.equal(validateRules(rules).definitions.length,1);
});
test('draft edit invalidates pending A and late A success cannot install or clear busy B',async()=>{
 const h=harness(),a=h.controller.apply(),old=h.pending[0];h.controller.configure(rules,'all','raw');assert.equal(old.options.signal.aborted,true);const b=h.controller.apply();old.resolve(response(payload('pages')));await a;assert.equal(h.installs.length,0);assert.equal(h.controller.state.busy,true);assert.equal(h.controller.state.pending.controller.signal,h.pending[1].options.signal);h.controller.invalidate();h.pending[1].reject(Object.assign(Error('aborted'),{name:'AbortError'}));await b;
});
test('old crawl failure cannot overwrite newer crawl error or busy ownership',async()=>{
 const h=harness(),a=h.controller.apply();h.setOwner({id:'B',nativeGeneration:8,uiGeneration:10,stable:true});h.controller.invalidate();h.controller.configure(rules,'all','');const b=h.controller.apply();h.pending[0].reject(Error('PRIVATE A ERROR'));await a;assert.equal(h.controller.state.error,'');assert.equal(h.controller.state.busy,true);assert.equal(h.controller.state.pending.owner.id,'B');h.controller.invalidate('Current B refusal');h.pending[1].reject(Error('Late B'));await b;assert.equal(h.controller.state.error,'Current B refusal');
});
test('same-job generation change without callback invalidation still discards pending snapshot',async()=>{
 const h=harness(),operation=h.controller.apply();h.setOwner({id:'A',nativeGeneration:8,uiGeneration:9,stable:true});await deliver(h,0,payload('pages'));await operation;assert.equal(h.installs.length,0);assert.equal(h.pending.length,1);
});
for(const [name,mutate]of[
 ['repeated page ordinal',data=>data.rows[1].page_ordinal=data.rows[0].page_ordinal],
 ['omitted tail page',data=>{data.rows=[];data.next=null;}],
 ['wrong page cursor',data=>data.next.offset=199],
 ['changed summary between pages',data=>data.cohort.summary.html_pages=1],
 ['changed snapshot between pages',data=>data.context.snapshot_token='d'.repeat(64)]
])test(`malformed population refuses ${name} with no partial installation`,async()=>{
 const h=harness(),operation=h.controller.apply();if(name==='changed summary between pages'||name==='changed snapshot between pages'||name==='omitted tail page'){await deliver(h,0,payload('pages'));const changed=payload('pages',200);mutate(changed);await deliver(h,1,changed);}else{const changed=payload('pages');mutate(changed);await deliver(h,0,changed);}await operation;assert.equal(h.installs.length,0);assert.equal(h.controller.state.applied,null);assert.equal(h.controller.state.busy,false);assert(h.controller.state.error);
});
test('page read failure clears the full usable snapshot and leaves no firstpage success',async()=>{
 const h=harness(),operation=h.controller.apply();await deliver(h,0,payload('pages'));await deliver(h,1,{message:'Snapshot changed'},{status:409});await operation;assert.equal(h.installs.length,0);assert.equal(h.controller.state.applied,null);assert.match(h.controller.state.error,/Snapshot changed/);assert.equal(h.controller.state.busy,false);
});
test('actual complete CSV bytes require exact snapshot, generation, length and digest',async()=>{
 const h=harness();await complete(h);const csv='row_type,url\r\npage,https://owned.invalid/owned/0200\r\n';const operation=h.controller.download('pages');await deliver(h,4,csv,{csv:true});await operation;assert.equal(h.downloads.length,1);assert.equal(Buffer.from(h.downloads[0][0]).toString(),csv);assert.equal(h.pending[4].body.expected_snapshot_token,context.snapshot_token);assert.equal(h.pending[4].body.cursor,null);assert(h.pending[4].url.endsWith('/segments/dashboard/pages.csv'));
});
test('edit while download waits prevents old bytes and errors from creating downloads',async()=>{
 const h=harness();await complete(h);const download=h.controller.download('json');h.controller.configure(rules,'unknown','');const newer=h.controller.apply();await deliver(h,4,{kind:'segment_dashboard_cohort_not_raw_crawl'});await download;assert.equal(h.downloads.length,0);assert.equal(h.controller.state.busy,true);h.controller.invalidate();h.pending[5].reject(Object.assign(Error('aborted'),{name:'AbortError'}));await newer;
});
for(const headers of [{'x-segment-snapshot-token':'d'.repeat(64)},{'x-segment-generation':'8'},{'x-content-sha256':'0'.repeat(64)},{'content-length':'500'}])test(`export corruption clears actionable context ${Object.keys(headers)[0]}`,async()=>{
 const h=harness();await complete(h);const operation=h.controller.download('pages');await deliver(h,4,'URL\r\n',{csv:true,headers});await operation;assert.equal(h.downloads.length,0);assert.equal(h.controller.state.applied,null);assert.equal(h.controller.state.busy,false);assert(h.controller.state.error);
});
test('duplicate requested URLs retain individually selected raw observations',async()=>{
 const h=harness(),operation=h.controller.apply(),first=payload('pages');first.rows[1].page.url=first.rows[0].page.url;first.rows[1].page.titles=['Different observation'];await deliver(h,0,first);await deliver(h,1,payload('pages',200));await deliver(h,2,payload('issues'));await deliver(h,3,analysis());await operation;assert.equal(h.installs.length,1);assert.equal(h.installs[0].pages.length,201);assert.equal(h.installs[0].pages[0].page.url,h.installs[0].pages[1].page.url);assert.equal(h.installs[0].pages[1].page.titles[0],'Different observation');
});
test('full JSON export verifies the named cohort envelope and complete original observation order',async()=>{
 const h=harness(),result=await complete(h);const operation=h.controller.download('json');await deliver(h,4,{kind:'segment_dashboard_cohort_not_raw_crawl',context:result.context,cohort:result.cohort,captured_config:{},captured_crawl_summary:result.captured_crawl_summary,pages:result.pages,issues:result.issues});await operation;assert.equal(h.downloads.length,1);assert.equal(h.downloads[0][1],'json');
});
test('full JSON cannot claim completeness with a repeated observation replacing the tail',async()=>{
 const h=harness(),result=await complete(h),pages=copy(result.pages);pages[200]=pages[199];const operation=h.controller.download('json');await deliver(h,4,{kind:'segment_dashboard_cohort_not_raw_crawl',context:result.context,cohort:result.cohort,pages,issues:result.issues});await operation;assert.equal(h.downloads.length,0);assert.equal(h.controller.state.applied,null);
});
test('browser storage invalidation requires definitions reload before another application',async()=>{
 const h=harness();await complete(h);h.controller.invalidate('Definitionen geändert; neu laden.');assert.equal(h.controller.state.applied,null);assert.equal(h.controller.state.draftValid,false);await h.controller.apply();assert.equal(h.pending.length,4);h.controller.configure(rules,'segment:owned','reported');assert.equal(h.controller.state.draftValid,true);
});

// Exercise actual main-dashboard projection/export functions, without automatic startup.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
function appHarness(){
 const elements=new Map(),get=id=>{if(!elements.has(id))elements.set(id,{hidden:false,textContent:'',classList:{toggle(){}},events:new Map(),addEventListener(name,listener){this.events.set(name,listener);},setAttribute(name,value){this[name]=value;},scrollIntoView(){}});return elements.get(id);};
 const source=fs.readFileSync(path.join(__dirname,'../app.js'),'utf8'),ending=/  initialize\(\);\s*\}\)\(\);\s*$/;assert(ending.test(source));
 const sandbox={console,Intl,URL,Set,Map,Number,JSON,Blob,TextEncoder,Uint8Array,document:{getElementById:get},downloaded:[]};
 vm.runInNewContext(source.replace(ending,"  renderInspector=()=>{};renderTable=()=>{};downloadBlob=(blob,name)=>globalThis.downloaded.push({blob,name});globalThis.actual={state,normalPage,getRows,columns,filteredRows,imageRows,externalRows,selectPage,exportCSV,relationshipPages,downloadDashboardSegmentCSV,renderDashboardSegments,bindDashboardSegments,setSegmentController:controller=>dashboardSegments=controller};\n})();"),sandbox);
 return {...sandbox.actual,get,downloaded:sandbox.downloaded,bindNative:controller=>{sandbox.window={DashboardSegments:{createController:()=>controller},crypto:crypto.webcrypto,addEventListener(){}};sandbox.actual.bindDashboardSegments();}};
}
test('actual page projections retain duplicate URL records and inspector selection uses the selected observation',()=>{
 const app=appHarness();app.state.section='titles';app.state.segmentView={context};app.state.pages=[0,1].map(index=>({...app.normalPage({url:'https://owned.invalid/same',status:200,content_type:'text/html',titles:[`Different ${index}`]}),_segmentOrdinal:500+index}));app.state.pageMap=new Map([[app.state.pages[0].url,app.state.pages[0]]]);
 const rows=app.getRows();assert.deepEqual([...rows].map(row=>row.title_text),['Different 0','Different 1']);app.selectPage(rows[1]);assert.equal(app.state.selected._segmentOrdinal,501);assert.equal(app.state.selected.titles[0],'Different 1');
});
test('actual source-owned image/external projections retain captured target facts outside the selected cohort',()=>{
 const app=appHarness(),target='https://outside.invalid/file.png',source=app.normalPage({url:'https://owned.invalid/source',status:200,internal:true,content_type:'text/html',images:[{url:target,alt:'Known'}],links:[{url:target,internal:false,text:'Outside'}]});app.state.pages=[source];app.state.pageMap=new Map([[source.url,source]]);app.state.segmentView={target_lookup:[{url:target,final_url:target,status:200,final_status:200,internal:false,image_metadata:{width:640,height:320,status:'complete'}}]};
 assert.equal(app.imageRows()[0].intrinsic_width,640);assert.equal(app.imageRows()[0]._page.url,source.url);assert.equal(app.externalRows()[0].status,200);assert.equal(app.externalRows()[0].source_count,1);
});
test('actual current-section CSV includes all filtered rows beyond the DOM page and context/ordinal for every row',async()=>{
 const app=appHarness();app.state.section='internal';app.state.crawl={id:'A',generation:7};app.state.segmentView={context,selection:{kind:'segment',id:'owned'}};app.state.pages=Array.from({length:201},(_,index)=>({...app.normalPage(page(index).page),_segmentOrdinal:500+index}));app.state.pageMap=new Map(app.state.pages.map(row=>[row.url,row]));app.state.pageSize=25;app.exportCSV();assert.equal(app.downloaded.length,1);const csv=await app.downloaded[0].blob.text(),lines=csv.replace(/^\uFEFF/,'').split('\r\n');assert.equal(lines.length,202);assert.match(lines[0],/Segment Snapshot Token/);assert.match(lines[0],/Original Page Ordinal/);assert(lines[201].includes(page(200).page.url));for(const line of lines.slice(1)){assert(line.includes(context.snapshot_token));assert(line.includes(context.rules_sha256));}assert.match(lines[201],/"700"$/);
});
test('actual HTML exposes exactly thirteen native CSV kinds with pages as the labelled default',()=>{
 const html=fs.readFileSync(path.join(__dirname,'../index.html'),'utf8'),match=html.match(/<label for="dashboard-segment-export-kind">Native CSV-Art<select id="dashboard-segment-export-kind" disabled>([\s\S]*?)<\/select><\/label>/);assert(match,'Native export selector has a real visible label');
 const options=[...match[1].matchAll(/<option value="([^"]+)"( selected)?>([^<]+)<\/option>/g)];assert.deepEqual(options.map(option=>option[1]),['pages','issues','links','dom','extractions','javascript','cookies','pdf','images','custom-search','accessibility','responses','fragments']);assert.deepEqual(options.filter(option=>option[2]).map(option=>option[1]),['pages']);assert.match(options.find(option=>option[1]==='issues')[3],/alle zugehörigen URLs/);
});
test('actual output-choice handler changes neither cohort nor requests until explicit export and preserves native issues CSV bytes',async()=>{
 const h=harness(),result=await complete(h),app=appHarness();app.bindNative(h.controller);app.state.segmentView=result;app.get('dashboard-segment-export-kind').value='issues';assert.equal(app.get('dashboard-segment-export-kind').events.size,0,'Changing only output kind binds no evaluation or invalidation action');assert.equal(h.pending.length,4);assert.equal(h.controller.state.applied,result);const originalIssues=copy(result.issues);
 const csv='Severity,Code,URL,Message,Related URLs\r\nWarning,missing_description,https://owned.invalid/owned/0200,Stored issue,https://outside.invalid/\r\n';const operation=app.get('dashboard-segment-csv').events.get('click')();assert.equal(h.pending.length,5);assert.equal(h.pending[4].url,'/api/crawls/A/segments/dashboard/issues.csv');assert.equal(h.pending[4].body.expected_snapshot_token,result.context.snapshot_token);assert.equal(h.pending[4].body.cursor,null);await deliver(h,4,csv,{csv:true});await operation;assert.equal(h.downloads.length,1);assert.equal(h.downloads[0][1],'issues');assert.equal(Buffer.from(h.downloads[0][0]).toString(),csv);assert.equal(h.controller.state.applied,result);assert.deepEqual(result.issues,originalIssues);
});
test('actual render disables the native kind selector with its CSV button and explains issue-count versus complete relationships',async()=>{
 const h=harness(),result=await complete(h),app=appHarness();app.setSegmentController(h.controller);app.state.crawl={id:'A',status:'completed'};app.state.section='issues';app.state.segmentView=result;app.renderDashboardSegments();assert.equal(app.get('dashboard-segment-export-kind').disabled,false);assert.equal(app.get('dashboard-segment-csv').disabled,false);assert.match(app.get('dashboard-segment-scope').textContent,/Anzahl weiterer URLs/);assert.match(app.get('dashboard-segment-scope').textContent,/sämtliche zugehörigen URLs/);
 h.controller.state.busy=true;app.renderDashboardSegments();assert.equal(app.get('dashboard-segment-export-kind').disabled,true);assert.equal(app.get('dashboard-segment-csv').disabled,true);h.controller.state.busy=false;app.state.segmentView=null;app.renderDashboardSegments();assert.equal(app.get('dashboard-segment-export-kind').disabled,true);assert.equal(app.get('dashboard-segment-csv').disabled,true);
});
