from pathlib import Path
p=Path('web/app.js')
t=p.read_bytes().decode('utf-8')
def line(prefix,new):
    global t
    found=[x for x in t.splitlines(keepends=True) if x.startswith(prefix)]
    assert len(found)==1,(prefix,len(found))
    old=found[0]; end='\r\n' if old.endswith('\r\n') else '\n'
    t=t.replace(old,new+end,1)
helpers='''  // Trust describes the captured extraction input, independently of max_results.
  // Imported observations are validated natively; these checks also fail closed
  // for stale/malformed browser data. A hash is an input binding, not website authenticity.
  function extractionSourceState(row) {
    const unknown=reason=>({status:'unknown',evaluation:'unknown',decode:'unknown',exactCount:null,scope:'unknown',reasons:[reason]});
    const o=row?.source_observation;
    if(o==null)return unknown('missing_saved_observation');
    const hash=value=>typeof value==='string'&&/^[a-f0-9]{64}$/.test(value);
    const natural=value=>Number.isSafeInteger(value)&&value>=0;
    const values=Array.isArray(row.values)?row.values:[];
    if(typeof o!=='object'||Array.isArray(o)||o.engine!=='open-seo-spider/extraction-source-v1'||!['raw_html','rendered_html'].includes(o.source)||o.source!==row.source||!['complete','incomplete','unavailable'].includes(o.status)||!['evaluated','error','not_run'].includes(o.evaluation)||!['exact','lossy','unknown'].includes(o.decode_status)||!Array.isArray(o.reasons)||o.reasons.some(reason=>typeof reason!=='string'||!reason||reason.length>256)||o.reasons.length>16||!hash(o.source_revision)||(o.observed_generation!=null&&!natural(o.observed_generation)))return unknown('invalid_source_observation');
    const hasInput=hash(o.input_sha256)&&natural(o.input_utf8_bytes);
    const noInput=o.input_sha256==null&&o.input_utf8_bytes==null;
    const evaluated=o.evaluation==='evaluated'&&!row.error&&natural(row.matched_count)&&row.matched_count>=values.length&&typeof row.truncated==='boolean'&&row.truncated===(row.matched_count>values.length);
    const failed=o.evaluation==='error'&&typeof row.error==='string'&&Boolean(row.error)&&row.matched_count==null&&!values.length&&row.truncated===false;
    const notRun=o.evaluation==='not_run'&&!row.error&&row.matched_count==null&&!values.length&&row.truncated===false;
    if(o.status==='unavailable'?(!noInput||!notRun||!o.reasons.length):(!hasInput||!(evaluated||failed)))return unknown('invalid_source_observation');
    if(o.status==='complete'&&(o.decode_status!=='exact'||o.reasons.length||typeof o.document_url!=='string'||!o.document_url||typeof o.observed_at!=='string'||!Number.isFinite(Date.parse(o.observed_at))))return unknown('invalid_source_observation');
    if(o.status==='incomplete'&&!o.reasons.length)return unknown('invalid_source_observation');
    return {status:o.status,evaluation:o.evaluation,decode:o.decode_status,exactCount:o.status==='complete'&&evaluated?row.matched_count:null,scope:evaluated?(o.status==='complete'?'complete_source':'evaluated_input_only'):failed?'evaluation_failed':'not_evaluated',reasons:o.reasons};
  }
  const extractionSourceLabel=status=>({complete:'Vollständig erfasst',incomplete:'Quelle unvollständig',unavailable:'Nicht ausgewertet',unknown:'Quellvertrauen unbekannt'}[status]||'Unbekannt');
  function extractionRows() {
    if(state.cache.extractions)return state.cache.extractions;
    const configs=array(state.crawl?.config?.custom_extractions);
    const identity=entry=>JSON.stringify([entry?.name||'',entry?.selector||'',entry?.attribute??null,entry?.method||'css',entry?.output||'auto']);
    const rows=[];
    for(const page of internals()){
      const saved=array(page.custom_extractions),remaining=new Map();
      for(const entry of saved){const key=identity(entry);remaining.set(key,(remaining.get(key)||0)+1);rows.push({...entry,url:page.url,name:entry?.name||entry?.selector||'',values:array(entry?.values),_page:page});}
      for(const config of configs){const key=identity(config),count=remaining.get(key)||0;if(count){remaining.set(key,count-1);continue;}rows.push({...config,url:page.url,name:config.name||config.selector,source:'',values:[],matched_count:null,truncated:null,error:null,_page:page,_missingObservation:true});}
    }
    state.cache.extractions=rows;
    return rows;
  }

'''
marker='  function getRows() {'
assert t.count(marker)==1
t=t.replace(marker,helpers.replace('\n','\r\n')+marker,1)
line("      case 'extractions': return internals()", "      case 'extractions': return extractionRows();")
line("      case 'extractions': return [urlColumn", "      case 'extractions': return [urlColumn, textColumn('name', 'Feldname'), textColumn('method', 'Methode', row => row.method || 'css'), textColumn('output', 'Ausgabe', row => row.output || 'auto'), textColumn('result_type', 'Ergebnistyp'), textColumn('selector', 'Ausdruck'), textColumn('attribute', 'Attribut', row => row.attribute || '—'), textColumn('source', 'Quelle', row => row.source === 'raw_html' ? 'HTML-Antwort' : row.source === 'rendered_html' ? 'Gerenderter DOM' : row.source || 'Unbekannt'), numberColumn('matches', 'Erfasste Werte', row => row.values.length), numberColumn('matched_count', 'Treffer im ausgewerteten Input'), booleanColumn('truncated', 'Werte begrenzt', row => row.truncated == null ? 'Unbekannt' : row.truncated ? 'Ja' : 'Nein'), textColumn('values', 'Werte', row => row.values.join(' | ')), textColumn('error', 'Fehler'), {key:'source_status',label:'Quellzustand',type:'text',value:row=>extractionSourceState(row).status,render:row=>escape(extractionSourceLabel(extractionSourceState(row).status))}, textColumn('evaluation_status', 'Auswertung', row => extractionSourceState(row).evaluation), textColumn('decode_status', 'Quelldekodierung', row => extractionSourceState(row).decode), numberColumn('exact_count', 'Exakte Gesamtanzahl', row => extractionSourceState(row).exactCount), textColumn('count_scope', 'Zählbereich', row => extractionSourceState(row).scope), textColumn('source_reasons', 'Quellhinweise', row => extractionSourceState(row).reasons.join(' | ')), textColumn('document_url', 'Dokument-URL', row => row.source_observation?.document_url), numberColumn('observed_generation', 'Erfasste Generation', row => row.source_observation?.observed_generation), textColumn('observed_at', 'Erfasst am', row => row.source_observation?.observed_at), textColumn('input_sha256', 'Eingabe-SHA256', row => row.source_observation?.input_sha256), numberColumn('input_utf8_bytes', 'Eingabe-UTF8-Bytes', row => row.source_observation?.input_utf8_bytes), textColumn('source_revision', 'Quellrevision', row => row.source_observation?.source_revision), textColumn('source_engine', 'Quellvertrag', row => row.source_observation?.engine)];")
line("      case 'extractions': return [...base", "      case 'extractions': return [...base, ['present', 'Mit erfassten Werten'], ['empty', 'Bestätigt keine Werte'], ['error', 'Mit Auswertungsfehler'], ['truncated', 'Ergebniswerte begrenzt'], ['incomplete', 'Quelle unvollständig'], ['unknown', 'Quellvertrauen unbekannt'], ['unavailable', 'Nicht ausgewertet']];")
line("    if (state.section === 'extractions') return", "    if (state.section === 'extractions') { const source=extractionSourceState(row);return filter==='present'?row.values.length>0:filter==='empty'?source.exactCount===0:filter==='error'?Boolean(row.error):filter==='truncated'?row.truncated===true:['incomplete','unknown','unavailable'].includes(filter)?source.status===filter:false; }")
line('    return {section:state.section,query_semantics:', "    const semantics=version===2?window.TableViewLibrary.EXPRESSION_QUERY_SEMANTICS:window.TableViewLibrary.QUERY_SEMANTICS;\r\n    return {section:state.section,query_semantics:state.section==='extractions'?`${semantics}/extraction-source-v1`:semantics,columns:tableSchema(),sectionFilters:filterOptions().map(option=>option[0])};")
old="    if(state.section==='inputs'){"
assert t.count(old)==1
t=t.replace(old,"    if(state.section==='extractions'&&!array(state.crawl?.config?.custom_extractions).length&&!extractionRows().length)return unavailable('Eigene Extraktionen nicht erfasst: Für diesen Lauf sind keine Regeln oder gespeicherten Beobachtungen verfügbar.');\r\n"+old,1)
line("        ['Eigene Extraktionen',", "        ['Eigene Extraktionen', page.custom_extractions.length ? page.custom_extractions.map(entry => {const s=extractionSourceState(entry),o=entry.source_observation;return `${entry.name || entry.selector} (${entry.method || 'css'}; ${entry.output || 'auto'}; ${entry.source || 'Quelle unbekannt'}): ${entry.error ? `Fehler: ${entry.error}` : array(entry.values).length ? array(entry.values).join(' | ') : '[keine erfassten Werte]'}\\n${extractionSourceLabel(s.status)}; Auswertung ${s.evaluation}; Dekodierung ${s.decode}; Treffer im Input ${entry.matched_count ?? 'unbekannt'}; exakte Gesamtanzahl ${s.exactCount ?? 'unbekannt'}; Zählbereich ${s.scope}; Werte begrenzt ${entry.truncated ?? 'unbekannt'}\\n${s.reasons.join(' | ')}\\n${o ? JSON.stringify(o) : 'Keine gespeicherte Quellbeobachtung'}`;}).join('\\n\\n') : 'Keine Extraktionen konfiguriert oder erfasst'],")
line("        ['Extraktionen vor Rendering',", "        ['Extraktionen vor Rendering', page.raw_metadata ? array(page.raw_metadata.custom_extractions).map(entry => `${entry.name || entry.selector}: ${array(entry.values).join(' | ') || '[keine erfassten Werte]'}; ${extractionSourceLabel(extractionSourceState(entry).status)}; exakte Gesamtanzahl ${extractionSourceState(entry).exactCount ?? 'unbekannt'}; ${entry.source_observation ? JSON.stringify(entry.source_observation) : 'Keine gespeicherte Quellbeobachtung'}`).join('\\n') || '[keine Extraktionen]' : '—'],")
p.write_bytes(t.encode('utf-8'))
print('App extraction source contract installed without normalizing existing line endings')
