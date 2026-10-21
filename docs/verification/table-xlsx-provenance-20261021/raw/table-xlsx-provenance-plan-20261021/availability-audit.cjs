'use strict';
// Read-only extracted-App audit. No DOM/browser/native/Cargo/network or product edits.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const root=path.resolve(__dirname,'../..'),source=fs.readFileSync(path.join(root,'web/app.js'),'utf8');
const xlsx=fs.readFileSync(path.join(root,'web/table-xlsx.js'),'utf8'),hash=bytes=>createHash('sha256').update(bytes).digest('hex');
function between(start,end){const from=source.indexOf(start),to=source.indexOf(end,from);assert(from>=0&&to>from);return source.slice(from,to);}
const actual=between('  function renderControls()','  function renderMetrics()')+between('  function getRows()','  const textColumn =')+
  between('  function captureTableProjection(','  function exportCSV()')+between('  function tableExportAvailable()','  function renderTableExport(');
const sectionDisabled=between('  function renderSection()','  function selectResponse(').split(/\r?\n/).find(line=>line.trim().startsWith("$('export-csv').disabled ="));assert(sectionDisabled);
const cases=[
  {section:'linkmetrics',analysis:null,analysisError:'Weiterführende Analyse nicht verfügbar: controlled non-success read',label:'unavailable analysis'},
  {section:'structured',analysis:null,analysisError:'Weiterführende Analyse nicht verfügbar: controlled non-success read',label:'unavailable analysis'},
  {section:'linkmetrics',analysis:{graph:{nodes:[]},structured_data:[]},analysisError:'',label:'known loaded empty analysis'},
  {section:'inputs',analysis:{},analysisError:'',label:'missing historical input inventory',config:{}},
  {section:'inputs',analysis:{},analysisError:'',label:'declared complete empty input inventory',config:{input_imports:[{rows:[]}]}}
];
const observed=[];
for(const input of cases){
  const realm=vm.createContext({TextEncoder});vm.runInContext(xlsx,realm);
  const setup=`const state={section:${JSON.stringify(input.section)},crawl:{id:'declared-source',status:'completed',config:${JSON.stringify(input.config??{})}},analysis:${JSON.stringify(input.analysis)},analysisError:${JSON.stringify(input.analysisError)},pages:[{url:'https://owned.invalid/a'}],issues:[],responses:null,fragments:null,selected:null,recrawlURLs:new Set(),settings:{}};
  const nodes=new Map(),$=id=>{if(!nodes.has(id))nodes.set(id,{hidden:false,disabled:false,checked:false,textContent:'',title:'',value:''});return nodes.get(id);};
  const window={ListImportTools:{}},screenshotView=null,dashboardSegments=null,configProfiles=null,tableExplorer={canExport:()=>true,columns:schema=>schema};
  const array=value=>Array.isArray(value)?value:value==null||value===''?[]:[value],columns=()=>[{key:'url',label:'URL',value:row=>row.url}],tableSchema=columns;
  const filteredRows=()=>getRows(),recrawlPage=()=>null,separateSegmentSections=new Set(['inputs']),textColumn=(key,label,value)=>({key,label,value});
  const active=()=>false,actionBusy=()=>false,canResume=()=>false,canRecrawl=()=>false,policySection=()=>false,contentSection=()=>false,sitemapSection=()=>false,findingsSection=()=>false;
  const syncDirectiveReportingProfileSource=()=>{},renderTableExport=()=>{},renderDashboardSegments=()=>{},renderSitemapBundleControls=()=>{},renderQuotaSummary=()=>{},number=value=>String(value??0);
  `;
  vm.runInContext(setup+actual,realm);vm.runInContext(sectionDisabled,realm);const sectionDisabledBefore=vm.runInContext("$('export-csv').disabled",realm);
  const observation=JSON.parse(vm.runInContext(`JSON.stringify((()=>{renderControls();const available=tableExportAvailable(),capture=captureTableProjection(true),writer=TableXlsx.create(),prepared=writer.prepareTable({columns:capture.value.columns,rows:capture.value.rows,context:[]}),encoded=writer.encode(prepared.value);return{section:state.section,rows:getRows().length,CSV_enabled:!$('export-csv').disabled,XLSX_available:available,projection_ok:capture.ok,writer_ok:encoded.ok,workbook_rows:encoded.value?.rows,workbook_columns:encoded.value?.columns,workbook_bytes:encoded.value?.bytes.length};})())`,realm));
  observed.push({...input,section_disabled_before_renderControls:sectionDisabledBefore,...observation});
}
assert(observed.every(item=>item.CSV_enabled&&item.XLSX_available&&item.projection_ok&&item.writer_ok&&item.workbook_rows===0));
assert.equal(observed.find(item=>item.label==='missing historical input inventory').section_disabled_before_renderControls,true);
const proof={kind:'read_only_actual_App_availability_distinction_audit',result:'OBSERVED_GAP_PURE_EXTRACTED_APP',source:{path:'web/app.js',bytes:Buffer.byteLength(source),sha256:hash(Buffer.from(source))},writer:{path:'web/table-xlsx.js',sha256:hash(Buffer.from(xlsx))},observed,
  finding:'Absent/failed analysis and loaded-empty analysis both reach a successful header-only export; missing historical input data is re-enabled by renderControls after section-specific disabling. This does not establish a native failure occurrence; it proves the current source path fails to distinguish these declared states.',
  required_next:'Use genuine unsuccessful analysis delivery/read controls before claiming broad native schemas. Preserve known-empty export success while refusing unavailable/incomplete sources. No product edit was made.'};
fs.writeFileSync(path.join(__dirname,'availability-proof.json'),JSON.stringify(proof,null,2)+'\n');console.log(JSON.stringify(proof,null,2));
