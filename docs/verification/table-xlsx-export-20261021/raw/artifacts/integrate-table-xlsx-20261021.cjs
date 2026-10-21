const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const root=path.resolve(__dirname,'..');
function edit(file,fn){const p=path.join(root,file),before=fs.readFileSync(p,'utf8'),next=fn(before,before.includes('\r\n')?'\r\n':'\n');assert.notEqual(next,before,file+' unchanged');fs.writeFileSync(p,next);}
function replace(s,a,b){assert.equal(s.split(a).length,2,'Expected one '+a.slice(0,90));return s.replace(a,b);}
edit('web/app.js',(s,nl)=>{
 s=replace(s,'  let tableRenderSequence = 0;','  let tableRenderSequence = 0;'+nl+'  let tableExport = null;');
 const start=s.indexOf('    const original = state.section;',s.indexOf('  function exportCSV() {'));
 const end=s.indexOf('    const csvCell = ',start);
 assert(start>0&&end>start);
 const old=s.slice(start,end);
 let shared=old.replace("    if(useExplorer&&tableExplorer&&!tableExplorer.canExport()){message('Die Tabellenansicht enthält einen Fehler. Vor dem Export die angezeigte Meldung beheben.',true);return;}","    if(useExplorer&&tableExplorer&&!tableExplorer.canExport())return{ok:false,error:{code:'query',message:'Die Tabellenansicht enthält einen Fehler. Vor dem Export die angezeigte Meldung beheben.'}};");
 shared=replace(shared,"    if (['overview', 'graph'].includes(original)) state.section = 'internal';","    try {"+nl+"    if (['overview', 'graph'].includes(original)) state.section = 'internal';");
 shared=replace(shared,'tableExplorer.columns(tableSchema()):columns();','tableExplorer.columns(tableSchema()).slice():columns();');
 shared=replace(shared,"    if(useExplorer&&tableExplorer&&!tableExplorer.canExport()){state.section=original;message('Die Tabellenansicht konnte nicht ausgewertet werden; kein CSV erzeugt.',true);return;}","    if(useExplorer&&tableExplorer&&!tableExplorer.canExport())return{ok:false,error:{code:'query',message:'Die Tabellenansicht konnte nicht vollständig ausgewertet werden; kein Export erzeugt.'}};");
 const suffix=[
 '    // Resolve getters while their original table context is still current.',
 '    // CSV retains its existing display values; XLSX preserves known raw scalars.',
 '    const values=rows.map(row=>cols.map(column=>{',
 '      const value=(typed&&column.rawValue?column.rawValue:column.value)(row);',
 '      return typed&&value==null?null:value;',
 '    }));',
 '    return{ok:true,value:{section:state.section,columns:cols.map(({key,label})=>({key,label})),rows:values}};',
 "    } catch { return{ok:false,error:{code:'snapshot',message:'Die vollständige Tabellenprojektion konnte nicht gelesen werden.'}}; }",
 '    finally { state.section=original; }',
 '  }',''
 ].join(nl);
 const functionStart=s.indexOf('  function exportCSV() {');
 s=s.slice(0,functionStart)+'  function captureTableProjection(typed=false) {'+nl+shared+suffix+nl+s.slice(functionStart);
 const newStart=s.indexOf(old,s.indexOf('  function exportCSV() {'));assert(newStart>0);
 s=s.slice(0,newStart)+[
 '    const captured=captureTableProjection();',
 '    if(!captured.ok){message(captured.error.message,true);return;}',
 '    const {columns:cols,rows,section}=captured.value;',
 ''
 ].join(nl)+s.slice(newStart+old.length);
 s=replace(s,'cols.map((column) => csvCell(column.value(row)))','row.map(csvCell)');
 s=replace(s,'    const section = state.section;'+nl+'    state.section = original;'+nl,'');
 const binding=[
 '  function tableExportAvailable() {',
 "    return Boolean(state.crawl && !$('table-view').hidden && !$('export-csv').disabled && !dashboardSegments?.state.busy && tableExplorer?.canExport());",
 '  }','',
 '  function renderTableExport(view=tableExport?.state()) {',
 "    const button=$('export-xlsx'),status=$('table-export-status'),cancel=$('table-export-cancel');",
 '    if(!button||!status||!cancel)return;',
 "    button.hidden=$('table-view').hidden;",
 '    button.disabled=!tableExport||Boolean(view?.busy)||!tableExportAvailable();',
 '    cancel.hidden=!view?.busy;',
 "    status.hidden=!view?.message;status.textContent=view?.message||'';",
 "    status.dataset.phase=view?.phase||'idle';status.dataset.code=view?.error?.code||'';",
 "    status.classList.toggle('error',Boolean(view?.error));",
 '  }','',
 '  function bindTableExport() {',
 "    const button=$('export-xlsx');",
 '    if(!window.TableXlsx||!window.TableExportUI){',
 "      renderTableExport({busy:false,phase:'refused',error:{code:'unavailable'},message:'XLSX-Export ist nicht geladen.'});return;",
 '    }',
 '    tableExport=window.TableExportUI.create({',
 '      writer:window.TableXlsx.create(),',
 '      available:tableExportAvailable,',
 "      connected:()=>button.isConnected&&$('export-xlsx')===button,",
 '      stamp:()=>JSON.stringify([tableOwner(),tableQueryRevision,tableRenderSequence,state.sort,state.direction,state.search,state.filter]),',
 '      snapshot:()=>{',
 '        const captured=captureTableProjection(true);if(!captured.ok)return captured;',
 '        const {section,columns,rows}=captured.value;',
 "        return{ok:true,value:{data:{columns,rows,context:[]},filename:`open-seo-spider-${section}-${state.crawl.id}.xlsx`}};",
 '      },',
 '      download:downloadBlob,changed:renderTableExport',
 '    });',
 "    button.addEventListener('click',()=>{void tableExport.run();});",
 "    $('table-export-cancel').addEventListener('click',()=>tableExport.cancel());",
 '    renderTableExport();',
 '  }',''
 ].join(nl);
 s=replace(s,'  async function serverDownload(endpoint, filename) {',binding+nl+'  async function serverDownload(endpoint, filename) {');
 s=replace(s,'    bindNamedTableViews();','    bindNamedTableViews();'+nl+'    bindTableExport();');
 s=replace(s,"    if(tableExplorer && !$('table-view').hidden && !tableExplorer.canExport())$('export-csv').disabled=true;","    if(tableExplorer && !$('table-view').hidden && !tableExplorer.canExport())$('export-csv').disabled=true;"+nl+'    renderTableExport();');
 s=replace(s,"    else renderTable();"+nl+"    if(dashboardSegments?.state.busy){$('export-csv').disabled=true;$('export-resources').disabled=true;$('export-visible-report').disabled=true;}"+nl+'  }',"    else renderTable();"+nl+"    if(dashboardSegments?.state.busy){$('export-csv').disabled=true;$('export-resources').disabled=true;$('export-visible-report').disabled=true;}"+nl+'    renderTableExport();'+nl+'  }');
 return s;
});
edit('web/index.html',(s,nl)=>{
 s=replace(s,'  <script src="/app.js" defer></script>','  <script src="/table-xlsx.js" defer></script>'+nl+'  <script src="/table-export-ui.js" defer></script>'+nl+'  <script src="/app.js" defer></script>');
 const needle='<button id="export-resources"';
 s=replace(s,needle,'<button id="export-xlsx" class="button small" type="button" disabled hidden title="Alle gefilterten Tabellenzeilen und gewählten Spalten als XLSX exportieren">XLSX</button><button id="table-export-cancel" class="button small" type="button" hidden>Export abbrechen</button><span id="table-export-status" class="table-export-status" role="status" aria-live="polite" hidden></span>'+needle);
 return s;
});
edit('web/styles.css',(s,nl)=>s+nl+[
 '/* XLSX uses the same table projection, with an explicit preparation state. */',
 '.table-export-status { flex-basis:100%; max-width:100%; font-size:var(--text-xs,12px); color:var(--text-secondary); overflow-wrap:anywhere; }',
 '.table-export-status.error { color:var(--danger,#f87171); }',
 '#table-export-cancel:focus-visible, #export-xlsx:focus-visible { outline:2px solid var(--accent,#78c9a3); outline-offset:3px; }',''
 ].join(nl));
edit('crates/app/src/server.rs',(s,nl)=>{
 const start=s.indexOf('        .route('+nl+'            "/table-view-portability.js",'),end=s.indexOf('        .route(',start+15);assert(start>0&&end>start);
 const original=s.slice(start,end);
 return s.slice(0,end)+original.replaceAll('table-view-portability','table-xlsx')+original.replaceAll('table-view-portability','table-export-ui')+s.slice(end);
});
const fixtures=['ui-smoke.cjs','ui-crawl-control-smoke.cjs','ui-selected-recrawl-smoke.cjs','ui-policy-smoke.cjs','ui-content-smoke.cjs','ui-resource-discovery-smoke.cjs','ui-resource-ledger-smoke.cjs','ui-resource-contract-smoke.cjs','ui-saved-comparison-smoke.cjs'];
for(const file of fixtures)edit('web/verification/'+file,s=>replace(s,"'/table-view-portability.js':", "'/table-xlsx.js':'table-xlsx.js','/table-export-ui.js':'table-export-ui.js','/table-view-portability.js':"));
for(const file of ['sitemap-membership-e2e.mjs','sitemap-assessment-e2e.mjs'])edit('scripts/'+file,s=>replace(s,"'/table-view-portability.js':'web/table-view-portability.js'","'/table-view-portability.js':'web/table-view-portability.js','/table-xlsx.js':'web/table-xlsx.js','/table-export-ui.js':'web/table-export-ui.js'").replace('fifteen actual embedded assets','seventeen actual embedded assets'));
edit('.github/workflows/ci.yml',(s,nl)=>replace(s,'      - run: node --test web/verification/table-view-portability.test.cjs','      - run: node --test web/verification/table-view-portability.test.cjs'+nl+'      - run: node --test web/verification/table-xlsx.test.cjs web/verification/table-export-ui.test.cjs'));
edit('.gitattributes',(s,nl)=>s+nl+'# Preserve exact XLSX ZIPs, CSV downloads and original verification logs.'+nl+'/docs/verification/table-xlsx-export-20261021/** -text whitespace=-blank-at-eol,-blank-at-eof,-space-before-tab,cr-at-eol'+nl);
edit('scripts/check-ui-fixtures.mjs',s=>{
 const start=s.indexOf('const reviewedCurrentTargets={'),end=s.indexOf('\n};',start);assert(start>=0&&end>start);let block=s.slice(start,end);
 for(const file of ['web/app.js','web/index.html']){const raw=fs.readFileSync(path.join(root,file)),entry="  '"+file+"':{bytes:"+raw.length+",sha256:'"+crypto.createHash('sha256').update(raw).digest('hex')+"'},";block=block.replace(new RegExp("  '"+file.replaceAll('.','\\.')+"':\\{[^\\n]+"),entry);}
 return s.slice(0,start)+block+s.slice(end);
});
console.log('Integrated shared projection, UI hooks, HTML/CSS, static assets and CI tests.');
