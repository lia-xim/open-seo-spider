'use strict';
// Pure extracted production-App functions with the real query/writer/controller.
// Fake DOM/download sinks do not establish browser, transport or native coverage.
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const root=path.resolve(__dirname,'../..'),app=fs.readFileSync(path.join(root,'web/app.js'),'utf8');
const nativeImports=JSON.parse(fs.readFileSync(path.join(root,'crates/core/tests/fixtures/list-import/serialized-contracts.json'),'utf8')).results;
function between(start,end){const from=app.indexOf(start),to=app.indexOf(end,from);assert(from>=0&&to>from,`App source boundary ${start}`);return app.slice(from,to);}
const helperStart=app.indexOf('  function tableCSVExportTitle()');
const actual=app.split(/\r?\n/).find(line=>line.trim().startsWith('const canonicalCount ='))+'\n'+(helperStart<0?'':app.slice(helperStart,app.indexOf('  function captureTableProjection(',helperStart)))+
  between('  function installDashboardSegmentView(','  function renderDashboardSegments()')+
  between('  function duplicateValues(','  function externalRows()')+
  between('  function renderControls()','  function renderMetrics()')+
  between('  function extractionSourceState(','  function filterOptions()')+
  between('  function matchesFilter(row)','  function tableOwner()')+
  between('  function tableOwner()','  function tableIdentity(')+
  between('  function projectColumnsAvailable()','  function bindNamedTableViews()')+
  between('  function filteredRows(','  function renderTable()')+
  between('  function captureTableProjection(','  async function serverDownload(');
const sectionTitle=between('  function renderSection()','  function selectResponse(').split(/\r?\n/).find(line=>line.trim().startsWith("$('export-csv').title ="));
assert(sectionTitle);
const ordinaryCSVTitle='Alle aktuell gefilterten Ergebnisse dieses Bereichs exportieren';
const xlsxTitle='Alle gefilterten Tabellenzeilen und gewählten Spalten als XLSX exportieren';
function harness(section,changes={}){
  const context=vm.createContext({Blob,TextEncoder,setTimeout,clearTimeout});
  for(const file of ['table-explorer.js','table-xlsx.js','table-export-ui.js'])vm.runInContext(fs.readFileSync(path.join(root,'web',file),'utf8'),context,{filename:file});
  vm.runInContext(`
  const window=globalThis;window.ListImportTools={};
  const state={section:${JSON.stringify(section)},crawl:{id:'declared-owner',status:'completed',config:{}},generation:1,pages:[{url:'https://owned.invalid/a',indexable:true}],pageMap:new Map(),issues:[],analysis:null,analysisError:'',responses:null,fragments:null,selected:null,recrawlURLs:new Set(),settings:{},cache:{},search:'',sort:'url',direction:1,filter:'all',polling:false};
  const nodes=new Map(),downloads=[],messages=[];
  const $=id=>{if(!nodes.has(id))nodes.set(id,{hidden:false,disabled:false,checked:false,textContent:'',title:id==='export-xlsx'?${JSON.stringify(xlsxTitle)}:${JSON.stringify(ordinaryCSVTitle)},value:'',dataset:{},isConnected:true,classList:{toggle(){}},listeners:new Map(),addEventListener(type,fn){this.listeners.set(type,fn);}});return nodes.get(id);};
  let tableExport=null,tableQueryRevision=0,tableRenderSequence=0,projectColumns=null,projectColumnsResetOwner=null;
  const screenshotView=null,dashboardSegments=null,configProfiles=null;
  const array=value=>Array.isArray(value)?value:value==null||value===''?[]:[value],first=value=>array(value).join(' | '),primary=value=>String(array(value)[0]??'');
  const number=value=>String(value??0),effectiveStatus=row=>row.final_status??row.status??null;
  const active=()=>false,actionBusy=()=>false,canResume=()=>false,canRecrawl=()=>false,policySection=()=>state.section==='policy',contentSection=()=>false,sitemapSection=()=>false,findingsSection=()=>false;
  const syncDirectiveReportingProfileSource=()=>{},renderDashboardSegments=()=>{},renderSitemapBundleControls=()=>{},renderQuotaSummary=()=>{},policyVisibleRows=()=>[{}];
  const separateSegmentSections=new Set(['inputs']),recrawlPage=row=>state.pageMap.get(row.url),internals=()=>state.pages,htmlPages=internals,pageIssues=()=>[];
  const normalPage=page=>page,reindexIssues=()=>{},render=()=>renderControls();
  const escape=value=>String(value),domPath=()=>'',domSource=()=>'',domSections=[];
  const downloadBlob=(blob,filename)=>downloads.push({blob,filename}),message=(text,error=false)=>messages.push({text,error});
  const rules=[];let visibleKeys=null,queryOkay=true;
  const tableExplorer={canExport:()=>queryOkay,columns:schema=>{const result=TableExplorer.projectColumns(schema,visibleKeys??schema.map(c=>c.key));if(!result.ok)throw Error(result.error.message);return result.value;},rows:(rows,schema,search,sort)=>{const plan=TableExplorer.compile(schema,{search,rules,sort,visibleKeys:visibleKeys??schema.map(c=>c.key)},{identity:row=>row.url});if(!plan.ok)throw Error(plan.error.message);const result=plan.value.rows(rows);if(!result.ok)throw Error(result.error.message);return result.value;}};
  `+actual,context,{filename:'extracted-app.js'});
  const run=code=>vm.runInContext(code,context);
  context.changes=changes;run('Object.assign(state,changes);state.pageMap=new Map(state.pages.map(page=>[page.url,page]));');
  run(`(()=>{const sourceAvailability=typeof tableSourceAvailability==='function'?tableSourceAvailability():{ok:true};${sectionTitle}})()`);run('bindTableExport();renderControls();');
  return{run,context,json:code=>JSON.parse(run(`JSON.stringify(${code})`)),downloads:run('downloads'),messages:run('messages')};
}
const loadedAnalysis={graph:{nodes:[]},structured_data:[]};
async function expectUnavailable(h,label){
  // Execute actual handlers first: old App falsely downloads header-only files.
  h.run('exportCSV()');assert.equal(h.downloads.length,0,`${label}: CSV must not download unavailable data`);
  assert.equal(h.run("$('export-csv').disabled"),true,`${label}: CSV disabled`);
  assert.equal(h.run("$('export-xlsx').disabled"),true,`${label}: XLSX disabled`);
  h.run("$('export-csv').disabled=false;"); // A stale/programmatically changed UI flag cannot authorize XLSX.
  assert.equal(h.run('tableExportAvailable()'),false,`${label}: independently unavailable`);
  assert.equal(h.run('captureTableProjection(true).error.code'),'table_source_unavailable');
  const result=await h.run('tableExport.run()');assert.equal(result.ok,false);assert.equal(result.error.code,'unavailable');assert.equal(h.downloads.length,0);
  assert.equal(h.run("$('table-export-status').dataset.code"),'unavailable');
  assert.match(h.run("$('export-csv').title"),/nicht (?:geladen|verfügbar|erfasst)|unvollständig/i);
  assert.equal(h.run("$('export-xlsx').title"),h.run("$('export-csv').title"));
  assert.equal(h.messages.at(-1).error,true);
}
for(const section of ['linkmetrics','structured']){
  test(`${section}: absent analysis refuses actual programmatic CSV/XLSX`,async()=>expectUnavailable(harness(section),section));
  test(`${section}: failed native source refuses cached arrays until recovery`,async()=>{
    const h=harness(section,{analysis:loadedAnalysis,analysisError:'controlled native read failure'});await expectUnavailable(h,section);
    h.run("state.analysisError='';renderControls();");
    assert.equal(h.run('tableExportAvailable()'),true);assert.equal(h.run("$('export-csv').title"),ordinaryCSVTitle);assert.equal(h.run("$('export-xlsx').title"),xlsxTitle);
    h.run('exportCSV()');assert.equal(h.downloads.length,1);assert.equal((await h.run('tableExport.run()')).ok,true);assert.equal(h.downloads.length,2);
  });
  for(const analysis of [[],{},section==='linkmetrics'?{graph:{nodes:{}}}:{structured_data:{}},section==='linkmetrics'?{graph:[]}:{structured_data:null}]){
    test(`${section}: malformed analysis ${JSON.stringify(analysis)} is unavailable`,async()=>expectUnavailable(harness(section,{analysis}),'malformed analysis'));
  }
  test(`${section}: genuine loaded empty array exports headers during unrelated polling`,async()=>{
    const h=harness(section,{analysis:loadedAnalysis,polling:true,pages:[]});assert.equal(h.run('tableExportAvailable()'),true);
    assert.equal(h.run('captureTableProjection(true).value.rows.length'),0);h.run('exportCSV()');assert.equal(h.downloads.length,1);
    assert.equal((await h.run('tableExport.run()')).ok,true);assert.equal(h.downloads.length,2);
    assert.equal((await h.downloads[0].blob.text()).split('\r\n').length,1);
  });
}
for(const inventory of [undefined,null,[],{},[null],[{}],[{rows:[]}], [{...nativeImports.txt,rows:[]}], [{...nativeImports.txt,rows:{}}], [{...nativeImports.txt,engine:'foreign'}], [{...nativeImports.txt,input_sha256:'bad'}], [{...nativeImports.txt,input_bytes:0}], [{...nativeImports.txt,input_bytes:1.1}], [{...nativeImports.txt,website_requests:1}]]){
  test(`inputs: unavailable captured envelope ${JSON.stringify(inventory)?.slice(0,100)}`,async()=>{
    const config=inventory===undefined?{}:{input_imports:inventory};await expectUnavailable(harness('inputs',{crawl:{id:'declared-owner',status:'completed',config}}),'uncaptured or malformed input source');
  });
}
test('inputs: actual serialized native fixture is available, query-empty remains a successful empty projection',async()=>{
  const h=harness('inputs',{crawl:{id:'declared-owner',status:'completed',config:{input_imports:[nativeImports.txt]}}});
  assert.equal(h.run('tableExportAvailable()'),true);assert.equal(h.run('captureTableProjection(true).value.rows.length'),nativeImports.txt.rows.length);
  h.run("state.search='no native source row matches this exact sentinel';renderControls();");
  assert.equal(h.run('captureTableProjection(true).value.rows.length'),0);h.run('exportCSV()');assert.equal(h.downloads.length,1);assert.equal((await h.run('tableExport.run()')).ok,true);
  assert.equal((await h.downloads[0].blob.text()).split('\r\n').length,1);
});
test('inputs: failed-load recovery removes stale reason and preserves raw imported values',async()=>{
  const h=harness('inputs');await expectUnavailable(h,'uncaptured inventory');
  h.context.fixture=JSON.parse(JSON.stringify(nativeImports.csv));h.run("state.crawl.config.input_imports=[fixture];state.sort='source_index';renderControls();");
  assert.equal(h.run('tableExportAvailable()'),true);assert.equal(h.run("$('export-csv').title"),ordinaryCSVTitle);assert.equal(h.run("$('export-xlsx').title"),xlsxTitle);
  const projection=h.json('captureTableProjection(true).value'),i=projection.columns.findIndex(column=>column.key==='raw_value');
  assert.deepEqual(projection.rows.map(row=>row[i]),nativeImports.csv.rows.map(row=>row.raw_value));
});
test('availability checks inspect import envelopes, not every native-validated row on each render',()=>{
  const h=harness('inputs');h.context.fixture=JSON.parse(JSON.stringify(nativeImports.txt));
  h.run("fixture.rows=new Proxy(new Array(1000000),{get(target,key){if(key==='length')return target.length;throw Error('availability iterated captured rows');}});state.crawl.config.input_imports=[fixture];renderControls();");
  assert.equal(h.run('tableExportAvailable()'),true);assert.equal(h.run("$('export-csv').disabled"),false);
});
test('typed indexable preserves booleans/null while CSV and existing text query labels remain unchanged',async()=>{
  const pages=[true,false,null,undefined,'true',0].map((indexable,i)=>({url:`https://owned.invalid/${i}`,indexable,meta_robots:[],x_robots_tag:[],indexability_reasons:[],status:200}));
  const h=harness('robots',{pages});
  const typed=h.json('captureTableProjection(true).value'),display=h.json('captureTableProjection(false).value'),i=typed.columns.findIndex(column=>column.key==='indexable');
  assert.deepEqual(typed.rows.map(row=>row[i]),[true,false,null,null,null,null]);
  assert.deepEqual(display.rows.map(row=>row[i]),['Indexierbar','Nicht indexierbar','Unbekannt','Unbekannt','Unbekannt','Unbekannt']);
  assert.equal(h.run("tableSchema().find(c=>c.key==='indexable').type"),'text');
  assert.equal(h.run("tableSchema().find(c=>c.key==='indexable').rawValue(state.pages[0])"),'Indexierbar');
  h.run("rules.push({key:'indexable',op:'equals',text:'Nicht indexierbar'});");
  assert.deepEqual(h.json('filteredRows().map(row=>row.url)'),['https://owned.invalid/1']);
  h.run('exportCSV()');assert.match(await h.downloads[0].blob.text(),/"Nicht indexierbar"/);
});
for(const [section,key,yes,no]of [
  ['external','nofollow','Ja','Nein'],['custom-search','evidence_complete','Ja','Nein'],
  ['responses','body_complete','Ja','Nein'],['responses','document_metadata_available','Ja','Nein'],
  ['headingorder','hidden_attribute','Ja','Nein'],['imagecandidates','eligible','Ja','Nein'],
  ['fragments','target_complete','Ja','Nein / unklar'],['imagefiles','body_complete','Ja','Nein'],
  ['pdf','text_complete','Ja','Nein'],['pdf','annotations_complete','Ja','Nein'],
  ['extractions','truncated','Ja','Nein'],['cookies','httpOnly','Ja','Nein'],['cookies','secure','Ja','Nein']
]){
  test(`${section}.${key}: typed boolean is independent of unchanged localized text predicates`,()=>{
    const h=harness(section);h.context.key=key;
    for(const value of [true,false,null,undefined,'true',0]){
      h.context.sample={[key]:value};const actual=h.run(`(()=>{const c=tableSchema().find(c=>c.key===key);return(c.exportValue||c.rawValue||c.value)(sample);})()`);
      assert.equal(actual,typeof value==='boolean'?value:null);
    }
    h.context.yes=yes;h.context.no=no;
    assert.equal(h.run('tableSchema().find(c=>c.key===key).value({[key]:true})'),yes);
    assert.equal(h.run('tableSchema().find(c=>c.key===key).value({[key]:false})'),no);
    assert.deepEqual(h.json(`(()=>{const c=tableSchema().find(c=>c.key===key);const p=TableExplorer.compile([c],{search:'',rules:[{key,op:'equals',text:no}],sort:{key,direction:1},visibleKeys:[key]});if(!p.ok)throw Error(p.error.message);return p.value.rows([{[key]:true},{[key]:false}]).value.map(row=>row[key]);})()`),[false]);
  });
}
test('derived header and canonical exports preserve native unknown/known distinctions',()=>{
  const h=harness('responses');
  for(const [headers,expected]of [[{},true],[{'content-type':'text/html'},true],[null,null],[undefined,null],[[],null],['bad',null]]){
    h.context.sample={headers};assert.equal(h.run("(()=>{const c=tableSchema().find(c=>c.key==='headers_known');return(c.exportValue||c.rawValue||c.value)(sample);})()"),expected);
  }
  h.run("state.section='canonical';");
  for(const [canonical,expected]of [[['https://owned.invalid/a'],true],[['https://owned.invalid/other'],false],[[],null],[null,null],['bad',null]]){
    h.context.sample={url:'https://owned.invalid/a',canonical};assert.equal(h.run("(()=>{const c=tableSchema().find(c=>c.key==='self_reference');return(c.exportValue||c.rawValue||c.value)(sample);})()"),expected);
  }
});
test('CSV section-specific policy and segment-issue titles survive control refresh',()=>{
  for(const [section,changes,title]of [['policy',{},'Nur gefilterte Vorkommen der aktuell geladenen Ledger-Seite exportieren'],['issues',{segmentView:{}},'Alle gefilterten Probleme mit den angezeigten Spalten; weitere URLs als Anzahl. Vollständige zugehörige URLs: native CSV-Art Probleme oder Segment-JSON.']]){
    const h=harness(section,changes);assert.equal(h.run("$('export-csv').title"),title);h.run('renderControls()');assert.equal(h.run("$('export-csv').title"),title);
  }
});
function emptySegmentResult(){
  return{context:{job_id:'declared-owner',app_generation:7,snapshot_token:'a'.repeat(64),rules_sha256:'b'.repeat(64),typed_report_sha256:'c'.repeat(64),reporting_view:null},cohort:{summary:{selected_pages:0,selected_issues:0,total_pages:3}},pages:[],issues:[],analysis:loadedAnalysis,responses:[],fragments:{enabled:true,observations:[]},target_lookup:[],selection:{kind:'segment',id:'s3'},rules:{version:1,definitions:[]}};
}
for(const section of ['titles','status','issues']){
  test(`${section}: actual installed, confirmed empty segment permits both complete header-only exports`,async()=>{
    const h=harness(section);h.run('state.crawl.generation=7;');h.context.segment=emptySegmentResult();
    h.run('installDashboardSegmentView(segment);');
    assert.equal(h.run("$('export-csv').disabled"),false,'A confirmed selected_pages0/selected_issues0 cohort is a captured source');
    assert.equal(h.run("$('export-xlsx').disabled"),false);assert.equal(h.run('tableExportAvailable()'),true);
    const projection=h.json('captureTableProjection(true)');assert.equal(projection.ok,true);assert.equal(projection.value.rows.length,0);
    assert(projection.value.columns.some(column=>column.key==='segment_token'));
    h.run('exportCSV()');assert.equal(h.downloads.length,1);assert.equal((await h.downloads[0].blob.text()).split('\r\n').length,1);
    const exported=await h.run('tableExport.run()');assert.equal(exported.ok,true);assert.equal(h.downloads.length,2);assert(h.downloads[1].blob.size>0);
  });
}
test('an empty unbound/malformed segment does not authorize empty page inventory',()=>{
  for(const edit of [view=>null,view=>({}),view=>({...view,context:{...view.context,job_id:'other-owner'}}),view=>({...view,context:{...view.context,app_generation:8}}),view=>({...view,pages:null}),view=>({...view,cohort:{summary:{selected_pages:1,selected_issues:0,total_pages:3}}})]){
    const h=harness('status',{pages:[],issues:[]});h.run('state.crawl.generation=7;');h.context.segment=edit(emptySegmentResult());h.run('state.segmentView=segment;renderControls();');
    assert.equal(h.run("$('export-csv').disabled"),true);assert.equal(h.run('tableExportAvailable()'),false);
  }
});
