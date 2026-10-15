  // Findings has its own query, opaque cursor roles and download ownership.
  const sitemapFindings = { result:null, token:null, download:null, revision:0, cursor:null, back:[], selected:null, error:'', downloadError:'', downloadBlocked:false, queryDirty:false };
  const findingsFilters = {
    'urls-in-sitemap':['URLs in Sitemap','saved_page'],
    'urls-not-in-sitemap':['URLs nicht in Sitemap','saved_page'],
    'orphan-urls':['Verwaiste URLs im gespeicherten Linkgraph','saved_page'],
    'non-indexable-urls-in-sitemap':['Nicht indexierbare URLs in Sitemap','saved_page'],
    'urls-in-multiple-sitemaps':['URLs in mehreren Sitemaps','saved_page'],
    'xml-sitemap-with-over-50k-urls':['XML-Sitemap mit über 50.000 URLs','source_attempt'],
    'xml-sitemap-over-50mb':['XML-Sitemap über 50 MiB','source_attempt']
  };
  const findingsAppEngine='open-seo-spider/app-sitemap-findings-v1';
  const findingsCoreEngine='open-seo-spider/full-stored-sitemap-findings-v1';
  const findingsSection=()=>state.section==='sitemap-findings';
  const findingsObject=v=>Boolean(v && typeof v==='object' && !Array.isArray(v));
  const findingsHash=v=>typeof v==='string' && /^[a-f0-9]{64}$/.test(v);
  const findingsInteger=v=>Number.isSafeInteger(v) && v>=0;
  const findingsOpaque=v=>typeof v==='string' && v.length>0 && v.length<=4096 && /^[A-Za-z0-9_-]+$/.test(v);
  const findingsVerdict=v=>({match:'Erfüllt',not_matched:'Nicht erfüllt',unknown:'Unbekannt'}[v] || v);
  const findingsValue=v=>v==null?'Unbekannt':typeof v==='number'&&!Number.isSafeInteger(v)?'Außerhalb sicherer Browser-Zahlengenauigkeit':String(v);
  function findingsQuantity(v) {
    if (!v || !['exact','at_least'].includes(v.precision) || !findingsInteger(v.value)) return 'Unbekannt';
    return `${v.precision==='at_least'?'≥ ':''}${findingsValue(v.value)}${v.precision==='exact'?' · exakt':' · Untergrenze'}`;
  }
  function findingsQuery() {
    const filter=$('sitemap-findings-filter').value, source=findingsFilters[filter]?.[1]==='source_attempt';
    const exact=$('sitemap-findings-exact').value, search=$('sitemap-findings-search').value;
    // Keep field order identical to Rust SitemapFindingSelection serialization.
    return {filter,assessment:$('sitemap-findings-assessment').value,requested_url:!source&&exact!==''?exact:null,source_task_url:source&&exact!==''?exact:null,search:search!==''?search:null};
  }
  function findingsSelectionText(query) {
    const text=JSON.stringify(query);
    // Rust strings cannot contain lone UTF-16 surrogates. Do not replace them.
    for (const value of Object.values(query)) if(typeof value==='string') for(let i=0;i<value.length;i++) {
      const code=value.charCodeAt(i);
      if(code>=0xd800&&code<=0xdbff){if(i+1>=value.length || value.charCodeAt(++i)<0xdc00 || value.charCodeAt(i)>0xdfff)throw Error('Ungültiges Unicode-Zeichen in der Auswahl.');}
      else if(code>=0xdc00&&code<=0xdfff)throw Error('Ungültiges Unicode-Zeichen in der Auswahl.');
    }
    return text;
  }
  async function findingsSelectionHash(text) {
    const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(text));
    return [...new Uint8Array(digest)].map(byte=>byte.toString(16).padStart(2,'0')).join('');
  }
  function invalidateFindingsRequest(clear=true) {
    sitemapFindings.revision++;
    sitemapFindings.token?.controller.abort();sitemapFindings.download?.controller.abort();
    sitemapFindings.token=null;sitemapFindings.download=null;
    if(clear)Object.assign(sitemapFindings,{result:null,cursor:null,back:[],selected:null,error:'',downloadError:'',downloadBlocked:false,queryDirty:false});
  }
  function resetSitemapFindings() { invalidateFindingsRequest();if(findingsSection())renderSitemapFindings(); }
  function findingsOwns(token,kind='token') {
    return sitemapFindings[kind]===token && token.revision===sitemapFindings.revision && token.id===state.crawl?.id && token.generation===state.generation && token.serverGeneration===(state.crawl?.generation??null) && findingsSection() && token.selectionText===JSON.stringify(findingsQuery()) && token.limit===Number($('sitemap-findings-limit').value) && token.candidateLimit===Number($('sitemap-findings-candidates').value);
  }
  function findingsToken() {
    const query=findingsQuery();
    return {id:state.crawl.id,generation:state.generation,serverGeneration:state.crawl.generation??null,revision:sitemapFindings.revision,query,selectionText:findingsSelectionText(query),limit:Number($('sitemap-findings-limit').value),candidateLimit:Number($('sitemap-findings-candidates').value),controller:new AbortController()};
  }
  async function findingsBytes(response,max,signal) {
    const declared=response.headers.get('content-length');
    if(declared!==null && (!/^\d+$/.test(declared) || Number(declared)>max)){await response.body?.cancel();throw Error('Antwort überschreitet die zulässige Byte-Grenze.');}
    if(!response.body)throw Error('Antwort enthält keinen lesbaren Body.');
    const reader=response.body.getReader(),chunks=[];let size=0;
    try {
      while(true){if(signal.aborted)throw new DOMException('Abgebrochen','AbortError');const part=await reader.read();if(part.done)break;size+=part.value.byteLength;if(size>max)throw Error('Antwort überschreitet die zulässige Byte-Grenze.');chunks.push(part.value);}
      if(declared!==null && Number(declared)!==size)throw Error('Unvollständige Antwort: Byte-Länge stimmt nicht.');
      return chunks;
    } catch(error) { await reader.cancel().catch(()=>{});throw error; }
    finally {reader.releaseLock();}
  }
  function findingsText(chunks) {const decoder=new TextDecoder('utf-8',{fatal:true});return chunks.map(chunk=>decoder.decode(chunk,{stream:true})).join('')+decoder.decode();}
  function findingsHTTPError(body,status) {const error=Error(typeof body?.error==='string'?body.error:`HTTP ${status}`);error.status=status;error.code=body?.code;return error;}
  async function findingsJSON(response,signal) {
    if(!(response.headers.get('content-type')||'').includes('application/json'))throw Error('Befund-API liefert keine unterstützte JSON-Antwort.');
    const body=JSON.parse(findingsText(await findingsBytes(response,8*1024*1024,signal)));
    if(!response.ok)throw findingsHTTPError(body,response.status);
    return body;
  }
  function validateFindingsEnvelope(body,token,expected=null) {
    const reject=message=>{throw Error(message);}, target=body?.access_target;
    if(!findingsObject(body)||body.engine!==findingsAppEngine||body.website_requests!==0||!findingsObject(target)||target.kind!=='job'||target.job_id!==token.id||!findingsInteger(target.generation)||!findingsHash(target.config_sha256))reject('Nicht unterstützter Befundstand oder falscher Crawl.');
    if(token.serverGeneration!==null && target.generation!==token.serverGeneration)reject('Befunde gehören zu einer anderen Crawl-Generation.');
    if(body.availability_reason!==null && typeof body.availability_reason!=='string')reject('Ungültige Verfügbarkeitsangabe.');
    if(body.findings===null){if(body.next_cursor!==null||body.export_token!==null||!body.availability_reason)reject('Nicht verfügbare Daten dürfen keine Cursor oder Exporte behaupten.');return;}
    if(token.serverGeneration===null)reject('Die Crawl-Generation ist noch nicht verfügbar. Stand neu laden.');
    const page=body.findings,ctx=page?.context,scan=page?.scan,basis=ctx?.basis,source=token.query.source_task_url!==null||findingsFilters[token.query.filter][1]==='source_attempt';
    if(!findingsObject(page)||!findingsObject(ctx)||ctx.engine!==findingsCoreEngine||ctx.selection_sha256!==token.selectionHash||!findingsHash(ctx.context_sha256)||!['available','available_with_unknown_history','unavailable'].includes(ctx.availability)||!Array.isArray(ctx.reasons)||!ctx.reasons.every(v=>typeof v==='string')||!findingsObject(basis)||basis.crawl_config_sha256!==target.config_sha256||basis.generation!==(target.generation===0?null:target.generation))reject('Befundkontext passt nicht zur tatsächlichen Auswahl oder Generation.');
    if(expected && (ctx.context_sha256!==expected.context || body.export_token!==expected.exportToken))reject('Die gespeicherte Auswahl wurde während der Seitennavigation geändert.');
    if(!findingsObject(scan)||!findingsInteger(scan.evaluated_candidates)||scan.evaluated_candidates>token.candidateLimit||typeof scan.exhausted!=='boolean'||!['exhausted','more_matches','candidate_budget','unavailable'].includes(scan.stop)||![null,true,false].includes(scan.more_matches))reject('Ungültiger Suchfortschritt.');
    if(source && (!findingsObject(ctx.source_scope)||typeof ctx.source_scope.unknown_history!=='boolean'||typeof ctx.source_scope.measurement_history_available!=='boolean'||typeof ctx.source_scope.current_task_inventory_available!=='boolean'||typeof ctx.source_scope.inherited!=='boolean'))reject('Quellenhistorie ist nicht unterstützt.');
    if(!source && ctx.source_scope!==null)reject('Quelldomäne passt nicht zur Seitenauswahl.');
    if(ctx.availability==='unavailable') {
      if(page.rows!==null||page.continuation!==null||body.next_cursor!==null||body.export_token!==null||scan.stop!=='unavailable'||scan.exhausted||scan.more_matches!==null)reject('Nicht verfügbare Befunde dürfen kein leeres Gesamtergebnis behaupten.');return;
    }
    if(!Array.isArray(page.rows)||page.rows.length>token.limit||!findingsOpaque(body.export_token)||body.availability_reason!==null)reject('Ungültige Befundseite oder Exportbindung.');
    if(scan.stop==='unavailable')reject('Verfügbarer Kontext hat einen widersprüchlichen Suchstand.');
    if(page.continuation===null) {
      if(body.next_cursor!==null||!scan.exhausted||scan.stop!=='exhausted'||scan.more_matches!==false)reject('Fehlender Fortsetzungscursor bei unvollständiger Suche.');
    } else {
      const continuation=page.continuation;
      if(!findingsObject(continuation)||!findingsOpaque(body.next_cursor)||body.next_cursor===token.cursor||scan.exhausted||continuation.context_sha256!==ctx.context_sha256||continuation.reason!==scan.stop||!['candidate_budget','more_matches'].includes(scan.stop)||(scan.stop==='candidate_budget'?scan.more_matches!==null:scan.more_matches!==true))reject('Ungültige Befundfortsetzung.');
      validateFindingsPosition(continuation.after,source,reject);
    }
    const positions=new Set();
    for(const row of page.rows) {
      if(!findingsObject(row)||!findingsObject(row.assessment)||!['match','not_matched','unknown'].includes(row.assessment.verdict)||token.query.assessment!=='all'&&row.assessment.verdict!==token.query.assessment||!Array.isArray(row.assessment.reasons)||!row.assessment.reasons.every(v=>typeof v==='string')||!findingsObject(row.evidence)||row.evidence.domain!==(source?'source_attempt':'saved_page'))reject('Befundzeile passt nicht zur Auswahl.');
      validateFindingsPosition(row.position,source,reject);
      const position=JSON.stringify(row.position);if(positions.has(position))reject('Doppelte Befundposition.');positions.add(position);
      if(source)validateFindingsSource(row.evidence.source,reject);
      else {const candidate=row.evidence.page,finding=candidate?.finding;if(!findingsObject(candidate)||!findingsObject(finding)||!findingsObject(finding.observation)||typeof finding.observation.requested_url!=='string'||!findingsObject(finding.observed_inlinks))reject('Gespeicherter Seitenbefund ist beschädigt.');}
    }
  }
  function validateFindingsPosition(position,source,reject) {
    if(!findingsObject(position)||!['saved_page','source_measurement','legacy_source_document','unmeasured_source_task'].includes(position.kind)||(position.kind!=='saved_page')!==source)reject('Falsche Befundcursor-Rolle.');
    const value=position.ordinal??position.sequence;if(!findingsInteger(value)||position.kind!=='saved_page'&&value===0)reject('Ungültige Befundposition.');
  }
  function validateFindingsSource(source,reject) {
    if(!findingsObject(source)||!['measured','legacy_document','unmeasured_task'].includes(source.kind))reject('Nicht unterstützte Quellenbeobachtung.');
    if(source.kind==='measured') {
      const record=source.record,protocol=source.protocol;
      if(!findingsObject(record?.measurement?.payload?.binding)||typeof record.measurement.payload.binding.requested_task_url!=='string'||!findingsObject(protocol)||protocol.measurement_id!==record.measurement.measurement_id)reject('Messung und Protokollprüfung sind nicht gebunden.');
      for(const field of ['entry_limit','decoded_xml_byte_limit']) {
        const check=protocol[field],value=check?.proof_quantity;
        if(!findingsObject(check)||!['within_limit','observed_exceeded','unknown'].includes(check.verdict)||typeof check.basis!=='string'||!findingsInteger(check.threshold)||!findingsObject(value)||!['exact','at_least','unavailable'].includes(value.precision)||(value.precision==='unavailable'?value.value!==null:!findingsInteger(value.value)))reject('Nicht unterstützte Protokollmessung oder Genauigkeit.');
      }
    } else if(typeof(source.kind==='legacy_document'?source.document?.task_url:source.task?.task_url)!=='string')reject('Originale Quellenidentität fehlt.');
  }
  function findingsReasonText(values) {return values.length?values.join(' · '):'Keine weiteren Gründe';}
  function findingsSourceIdentity(source) {return source.kind==='measured'?source.record.measurement.payload.binding.requested_task_url:source.kind==='legacy_document'?source.document.task_url:source.task.task_url;}
  function findingsCells(row) {
    const assessment=row.assessment, prefix=[findingsVerdict(assessment.verdict),findingsReasonText(assessment.reasons)];
    if(row.evidence.domain==='saved_page') {
      const finding=row.evidence.page.finding,observation=finding.observation;
      return [observation.requested_url,...prefix,finding.membership,findingsValue(finding.original_membership_occurrences),findingsValue(finding.original_source_document_count),observation.state,findingsValue(observation.final_status),findingsValue(finding.observed_inlinks.nonself_anchor_occurrences),observation.age_scope];
    }
    const source=row.evidence.source,p=source.protocol;
    return [findingsSourceIdentity(source),...prefix,({measured:'Messung',legacy_document:'Historisches Dokument',unmeasured_task:'Aufgabe ohne Messung'}[source.kind]),p?findingsQuantity(p.entry_limit.proof_quantity):'Unbekannt',p?findingsQuantity(p.decoded_xml_byte_limit.proof_quantity):'Unbekannt',p?.decode_integrity??'Unbekannt',p?.membership_retention??'Unbekannt',source.kind==='measured'?(source.association?'Exakte Commit-Zuordnung':'A-Messung ohne B-Commit'):'Keine Messzuordnung'];
  }
  function renderSitemapFindings() {
    const current=sitemapFindings,body=current.result,page=body?.findings,ctx=page?.context,rows=page?.rows??[],busy=Boolean(current.token),ready=Boolean(body&&page&&ctx.availability!=='unavailable'&&!busy&&!current.error),source=findingsFilters[$('sitemap-findings-filter').value][1]==='source_attempt';
    $('sitemap-findings-view').setAttribute('aria-busy',String(busy));
    $('sitemap-findings-error').hidden=!current.error;$('sitemap-findings-error').textContent=current.error;
    $('sitemap-findings-download-error').hidden=!current.downloadError;$('sitemap-findings-download-error').textContent=current.downloadError;
    $('sitemap-findings-summary').textContent=!state.crawl?'Einen gespeicherten Crawl auswählen.':busy?'Gespeicherte Befunde werden geprüft …':current.error?'Befunde konnten nicht sicher geladen werden.':current.queryDirty?'Auswahl geändert. Mit „Auswahl anwenden“ laden.':!body?'Noch keine Befunde geladen.':!page?`Befunde nicht verfügbar: ${body.availability_reason}`:ctx.availability==='unavailable'?`Gespeicherte Analyse nicht verfügbar: ${findingsReasonText(ctx.reasons)}`:`${rows.length} Zeilen in diesem Suchfenster · ${page.scan.evaluated_candidates} gespeicherte Kandidaten geprüft · ${page.scan.exhausted?'gespeicherte Auswahl durchlaufen':'weitere gespeicherte Kandidaten vorhanden'}`;
    let scope='Filter und CSV gelten für alle passenden gespeicherten Kandidaten. Keine vollständige Website- oder Sitemap-Entdeckung wird behauptet. Suche: wörtlich und mit Groß-/Kleinschreibung auf Original-URL bzw. Original-Aufgabe.';
    if(ctx){scope+=` Stand: ${ctx.availability}. ${ctx.reasons.length?findingsReasonText(ctx.reasons)+'. ':''}`;if(ctx.basis.inherited)scope+='Geerbte Beobachtungen aus dem Elternlauf; kein aktueller Discoverylauf. ';if(ctx.source_scope){const s=ctx.source_scope;scope+=`Messhistorie: ${s.measurement_history_available?'vorhanden':'unbekannt'}; unbekannte Vorgeschichte: ${s.unknown_history?'ja':'nein'}; Auslassung: ${s.omission??'keine aufgezeichnet'}; aktuelles Aufgabeninventar: ${s.current_task_inventory_available?'verfügbar':'unbekannt'}. `;}scope+='Neue Website-Anfragen beim Lesen: 0.';}
    $('sitemap-findings-scope').textContent=scope;
    $('sitemap-findings-exact-label').textContent=source?'Exakte Original-Aufgaben-URL (optional)':'Exakte ursprüngliche Anfrage-URL (optional)';
    $('sitemap-findings-next').disabled=!ready||!body.next_cursor;$('sitemap-findings-prev').disabled=!ready||!current.back.length;
    $('sitemap-findings-refresh').disabled=!state.crawl;
    $('sitemap-findings-export').disabled=!ready||!body.export_token||Boolean(current.download)||current.downloadBlocked;
    $('sitemap-findings-export-cancel').hidden=!current.download;
    $('sitemap-findings-export').textContent=current.download?'CSV wird vollständig übertragen …':'↓ Alle gefilterten gespeicherten Befunde CSV';
    $('sitemap-findings-page-label').textContent=page?(page.scan.stop==='candidate_budget'?'Arbeitsfenster erreicht – auch ohne Treffer mit „Weiter“ fortsetzen.':page.scan.stop==='more_matches'?'Weitere passende Befunde vorhanden.':page.scan.stop==='exhausted'?'Gespeicherte Auswahl durchlaufen.':'Analyse nicht verfügbar.'):'Keine geladene Auswahl';
    const headers=source?['Original-Aufgabe','Befund','Gründe','Quellenevidenz','Protokoll-Einträge','Dekodierte XML-Bytes','Integrität','Mitgliedschafts-Retention','Commit-Zuordnung']:['Ursprüngliche Anfrage-URL','Befund','Gründe','Mitgliedschaft','Original-Vorkommen','Verschiedene Original-Aufgaben','Zielbefund','HTTP final','Beobachtete Nicht-Selbst-Inlinks','Beobachtungsalter'];
    const table=$('sitemap-findings-table');table.querySelector('thead').innerHTML=`<tr>${headers.map(h=>`<th scope="col">${escape(h)}</th>`).join('')}</tr>`;
    table.querySelector('tbody').innerHTML=rows.map((row,index)=>`<tr tabindex="0" data-finding-row="${index}" aria-label="Befund ${index+1} öffnen">${findingsCells(row).map(cell=>`<td>${escape(cell??'Unbekannt')}</td>`).join('')}</tr>`).join('');
    table.querySelectorAll('[data-finding-row]').forEach(element=>{const open=()=>{current.selected=Number(element.dataset.findingRow);renderFindingsInspector();$('sitemap-findings-inspector-heading').focus({preventScroll:true});$('sitemap-findings-inspector').scrollIntoView({block:'nearest'});};element.addEventListener('click',open);element.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();open();}});});
    renderFindingsInspector();
  }
  function renderFindingsInspector() {
    const row=sitemapFindings.result?.findings?.rows?.[sitemapFindings.selected];$('sitemap-findings-inspector').hidden=!row;if(!row)return;
    const details=[['Ergebnis',findingsVerdict(row.assessment.verdict)],['Gründe',findingsReasonText(row.assessment.reasons)]];
    if(row.evidence.domain==='source_attempt') {const source=row.evidence.source,p=source.protocol;details.push(['Original-Aufgabe',findingsSourceIdentity(source)],['Evidenztyp',source.kind]);if(p)for(const [label,key] of [['URL-/Eintragsgrenze','entry_limit'],['Dekodierte XML-Byte-Grenze','decoded_xml_byte_limit']])details.push([label,`${p[key].verdict} · ${findingsQuantity(p[key].proof_quantity)} · Grenze ${p[key].threshold} · ${p[key].basis}`]);if(p)details.push(['Decode-Integrität',p.decode_integrity],['XML-Scan',p.xml_scan_status],['Mitgliedschafts-Retention',p.membership_retention],['Commit-Zuordnung',source.association?'Exakte B-Zuordnung gespeichert':'Keine B-Zuordnung; Messung beweist keine Mitgliedschaft']);}
    if(row.evidence.domain==='source_attempt' && row.evidence.source.kind==='measured') {
      const payload=row.evidence.source.record.measurement.payload,t=payload.transport,decode=payload.decode,scan=payload.scan;
      details.push(['HTTP-Anfrage gestartet',findingsValue(t?.request_started)],['HTTP final',findingsValue(t?.final_status)],['Transport-Body vollständig',findingsValue(t?.body_complete)],['Nach HTTP-Dekodierung empfangene Bytes',findingsQuantity(t?.post_http_received_bytes)],['Vollständige dekodierte XML-Bytes',findingsQuantity(decode?.decoded_xml_bytes)],['Decode-Abbruch',decode?.stop_code??'Keiner aufgezeichnet'],['Scan-Abbruch',scan?.stop_code??'Keiner aufgezeichnet']);
      for(const [label,key] of [['Begonnene unmittelbare Einträge','immediate_entry_starts'],['Abgeschlossene unmittelbare Einträge','completed_immediate_entries'],['Protokollgeeignete abgeschlossene Einträge','protocol_eligible_completed_entries'],['Tatsächliche loc-Vorkommen im Dokumentnamensraum','actual_document_namespace_loc_occurrences']])details.push([label,findingsQuantity(scan?.counts?.[key])]);
    }
    $('sitemap-findings-inspector-details').innerHTML=details.map(([key,value])=>`<dt>${escape(key)}</dt><dd>${escape(value)}</dd>`).join('');
    $('sitemap-findings-inspector-data').textContent=JSON.stringify(row,null,2);
  }
  async function loadSitemapFindings(cursor=null,navigation=null) {
    if(!state.crawl||!findingsSection()||sitemapFindings.token)return;
    const previous=sitemapFindings.result,oldCursor=sitemapFindings.cursor,expected=navigation?{context:previous?.findings?.context.context_sha256,exportToken:previous?.export_token}:null;
    sitemapFindings.download?.controller.abort();sitemapFindings.download=null;sitemapFindings.downloadError='';sitemapFindings.downloadBlocked=false;sitemapFindings.queryDirty=false;
    let token;try{token=findingsToken();}catch(error){sitemapFindings.error=error.message;renderSitemapFindings();return;}
    token.cursor=cursor;sitemapFindings.token=token;sitemapFindings.result=null;sitemapFindings.selected=null;sitemapFindings.error='';renderSitemapFindings();
    try {
      token.selectionHash=await findingsSelectionHash(token.selectionText);if(!findingsOwns(token))return;
      const payload={...token.query,limit:token.limit,candidate_limit:token.candidateLimit,...(cursor?{cursor}:{})};
      const result=await findingsJSON(await fetch(`/api/crawls/${encodeURIComponent(token.id)}/sitemaps/findings`,{method:'POST',headers:{'Content-Type':'application/json',Accept:'application/json'},body:JSON.stringify(payload),signal:token.controller.signal}),token.controller.signal);
      if(!findingsOwns(token))return;validateFindingsEnvelope(result,token,expected);
      if(navigation==='next')sitemapFindings.back.push(oldCursor);else if(navigation==='back')sitemapFindings.back.pop();
      sitemapFindings.result=result;sitemapFindings.cursor=cursor;
    } catch(error) {if(findingsOwns(token)){sitemapFindings.result=null;sitemapFindings.cursor=null;sitemapFindings.back=[];sitemapFindings.error=error.name==='AbortError'?'Laden abgebrochen.':`${error.status?`HTTP ${error.status} · `:''}${error.code?error.code+' · ':''}${error.message}`;}}
    finally {if(sitemapFindings.token===token){sitemapFindings.token=null;if(findingsSection())renderSitemapFindings();}}
  }
  async function exportSitemapFindings() {
    const body=sitemapFindings.result;if(!body?.export_token||sitemapFindings.token||sitemapFindings.download||sitemapFindings.downloadBlocked||sitemapFindings.error||!findingsSection()||!state.crawl)return;
    let token;try{token=findingsToken();}catch(error){sitemapFindings.downloadError=error.message;renderSitemapFindings();return;}
    token.result=body;sitemapFindings.download=token;sitemapFindings.downloadError='';renderSitemapFindings();
    try {
      const response=await fetch(`/api/crawls/${encodeURIComponent(token.id)}/sitemaps/findings/export.csv`,{method:'POST',headers:{'Content-Type':'application/json',Accept:'text/csv'},body:JSON.stringify({...token.query,snapshot:body.export_token}),signal:token.controller.signal});
      if(!response.ok){await findingsJSON(response,token.controller.signal);throw Error(`HTTP ${response.status}`);}
      if(!(response.headers.get('content-type')||'').toLowerCase().startsWith('text/csv'))throw Error('CSV-API liefert keinen unterstützten CSV-Body.');
      const chunks=await findingsBytes(response,64*1024*1024,token.controller.signal);
      if(!findingsOwns(token,'download')||sitemapFindings.result!==token.result)return;
      downloadBlob(new Blob(chunks,{type:'text/csv;charset=utf-8'}),`open-seo-spider-${token.id}-sitemap-findings.csv`);
    } catch(error) {if(findingsOwns(token,'download')){sitemapFindings.downloadBlocked=true;sitemapFindings.downloadError=error.name==='AbortError'?'CSV-Übertragung abgebrochen.':`${error.status?`HTTP ${error.status} · `:''}${error.code?error.code+' · ':''}${error.message} · Vor erneutem Export den Stand neu laden.`;}}
    finally {if(sitemapFindings.download===token){sitemapFindings.download=null;if(findingsSection())renderSitemapFindings();}}
  }
  function bindSitemapFindingsEvents() {
    const reset=()=>{resetSitemapFindings();};
    $('sitemap-findings-filter').addEventListener('change',()=>{$('sitemap-findings-exact').value='';reset();loadSitemapFindings();});
    for(const id of ['assessment','limit','candidates'])$('sitemap-findings-'+id).addEventListener('change',()=>{reset();loadSitemapFindings();});
    // Invalidate synchronously while editing; submit performs one exact query.
    for(const id of ['search','exact'])$('sitemap-findings-'+id).addEventListener('input',()=>{reset();sitemapFindings.queryDirty=true;renderSitemapFindings();});
    $('sitemap-findings-form').addEventListener('submit',event=>{event.preventDefault();reset();loadSitemapFindings();});
    $('sitemap-findings-refresh').addEventListener('click',()=>{reset();loadSitemapFindings();});
    $('sitemap-findings-next').addEventListener('click',()=>{if(sitemapFindings.result?.next_cursor)loadSitemapFindings(sitemapFindings.result.next_cursor,'next');});
    $('sitemap-findings-prev').addEventListener('click',()=>{if(sitemapFindings.back.length)loadSitemapFindings(sitemapFindings.back.at(-1),'back');});
    $('sitemap-findings-export').addEventListener('click',exportSitemapFindings);
    $('sitemap-findings-export-cancel').addEventListener('click',()=>{const token=sitemapFindings.download;if(token){token.controller.abort();sitemapFindings.download=null;sitemapFindings.downloadError='CSV-Übertragung abgebrochen.';renderSitemapFindings();}});
    $('sitemap-findings-inspector-close').addEventListener('click',()=>{const index=sitemapFindings.selected;sitemapFindings.selected=null;renderFindingsInspector();$('sitemap-findings-table').querySelector(`[data-finding-row="${index}"]`)?.focus({preventScroll:true});});
  }
