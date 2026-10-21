/* Native saved-cohort controller. Definitions are browser-local; observations remain native. */
((root, factory) => {
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  else root.DashboardSegments = api;
})(typeof globalThis === 'object' ? globalThis : this, () => {
  'use strict';
  const KEY = 'open-seo-saved-segments-v1', CAP = 64 * 1024 * 1024;
  const clone = value => JSON.parse(JSON.stringify(value));
  const textBytes = value => new TextEncoder().encode(value).length;
  const integer = value => Number.isSafeInteger(value) && value >= 0;
  const hex = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
  function validateRules(rules) {
    if (!rules || rules.version !== 1 || !Array.isArray(rules.definitions) || rules.definitions.length > 32) throw Error('Ungültige gespeicherte Segmentdefinitionen.');
    const ids = new Set(); let total = 0;
    for (const definition of rules.definitions) {
      if (!definition || !/^[A-Za-z0-9_-]{1,64}$/.test(definition.id) || ids.has(definition.id) || typeof definition.name !== 'string' || !definition.name.trim() || textBytes(definition.name) > 256 || !['all','any'].includes(definition.join) || !Array.isArray(definition.conditions) || !definition.conditions.length || definition.conditions.length > 8) throw Error('Ungültige gespeicherte Segmentdefinition.');
      ids.add(definition.id);
      for (const condition of definition.conditions) {
        if (!condition || !['requested_url','final_url','status','media_type','stored_title'].includes(condition.field) || !['equals','contains','regex'].includes(condition.operator) || typeof condition.value !== 'string' || typeof condition.case_insensitive !== 'boolean' || textBytes(condition.value) > 2048) throw Error('Ungültige Segmentbedingung.');
        total += textBytes(condition.value);
      }
    }
    if (total > 32768 || textBytes(JSON.stringify(rules)) > 60000) throw Error('Segmentdefinitionen sind zu groß.');
    return clone(rules);
  }
  function selection(value, rules) {
    if (['all','unassigned','unknown'].includes(value)) return {kind:value};
    const id = typeof value === 'string' && value.startsWith('segment:') ? value.slice(8) : '';
    if (!rules.definitions.some(definition => definition.id === id)) throw Error('Das ausgewählte Segment ist nicht mehr gespeichert.');
    return {kind:'segment',id};
  }
  function createController(deps) {
    const state = {revision:0,busy:false,applied:null,error:'',draftValid:true,rules:{version:1,definitions:[]},selection:'all',reportingMode:'',pending:null};
    const notify = () => deps.onChange?.(state);
    const owner = () => deps.owner();
    const key = value => JSON.stringify(value, (_name,item) => item && typeof item==='object' && !Array.isArray(item) ? Object.fromEntries(Object.keys(item).sort().map(name=>[name,item[name]])) : item);
    const owns = token => state.pending === token && state.revision === token.revision;
    const current = token => owns(token) && key(owner()) === token.ownerKey;
    function invalidate(reason = '') {
      state.revision++; state.pending?.controller.abort(); state.pending = null;
      state.busy = false; state.applied = null; state.error = reason;
      if(reason)state.draftValid=false;
      deps.onClear?.(); notify();
    }
    function configure(rules, chosen = state.selection, mode = state.reportingMode) {
      invalidate();
      state.draftValid=false;
      try {
        const checked = validateRules(rules); selection(chosen, checked);
        if (!['','raw','reported'].includes(mode)) throw Error('Ungültige Berichtsansicht.');
        state.rules = checked; state.selection = chosen; state.reportingMode = mode;state.draftValid=true;
      } catch (error) { state.error = error.message; notify(); throw error; }
      notify();
    }
    function requestBody(token, view = 'pages', cursor = null, context = null) {
      return {rules:token.rules,selection:token.selection,view,limit:200,cursor,expected_generation:token.owner.nativeGeneration,expected_snapshot_token:context?.snapshot_token ?? null,...(token.mode ? {reporting_mode:token.mode} : {})};
    }
    async function post(token, suffix, body, context = null, csv = false) {
      const response = await deps.fetch(`/api/crawls/${encodeURIComponent(token.owner.id)}/segments/${suffix}`, {method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body),signal:token.controller.signal});
      const rawLength = response.headers.get('content-length');
      if (!response.ok) {
        const bytes = await response.arrayBuffer();
        if (bytes.byteLength > CAP) throw Error('Segmentantwort überschreitet die Ausgabegrenze.');
        let message; try { const value=JSON.parse(new TextDecoder().decode(bytes)); message=value.message || value.error?.message || value.error; } catch (_) {}
        throw Error(typeof message === 'string' ? message : `Segmentauswertung abgelehnt (${response.status}).`);
      }
      const length = Number(rawLength), digest = response.headers.get('x-content-sha256');
      if (!/^(0|[1-9][0-9]*)$/.test(rawLength ?? '') || !integer(length) || length > CAP || !hex(digest) || response.headers.get('cache-control') !== 'no-store' || response.headers.get('content-type')?.split(';',1)[0].trim() !== (csv ? 'text/csv' : 'application/json')) throw Error('Segmentantwort ohne bestätigte Länge, Prüfsumme oder Ausgabeformat.');
      const chunks=[]; let size=0;
      if (response.body?.getReader) {
        const reader=response.body.getReader();
        try { while (true) { const part=await reader.read(); if (part.done) break; size+=part.value.byteLength; if(size>length || size>CAP) throw Error('Segmentantwort überschreitet die bestätigte Länge.'); chunks.push(part.value); } }
        finally { reader.releaseLock(); }
      } else { const chunk=new Uint8Array(await response.arrayBuffer());size=chunk.byteLength;chunks.push(chunk); }
      if(size!==length) throw Error('Segmentantwort ist unvollständig.');
      const bytes=new Uint8Array(size);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.byteLength;}
      const actual=[...new Uint8Array(await deps.crypto.subtle.digest('SHA-256',bytes))].map(byte=>byte.toString(16).padStart(2,'0')).join('');
      if(actual!==digest) throw Error('Segmentantwort: SHA-256 stimmt nicht überein.');
      if(context && (response.headers.get('x-segment-snapshot-token')!==context.snapshot_token || response.headers.get('x-segment-generation')!==String(token.owner.nativeGeneration))) throw Error('Segmentexport gehört zu einem anderen gespeicherten Stand.');
      return {bytes,data:csv?null:JSON.parse(new TextDecoder().decode(bytes))};
    }
    function validateContext(data, token, context) {
      if (!data || !data.context || data.context.job_id !== token.owner.id || data.context.app_generation !== token.owner.nativeGeneration || !hex(data.context.snapshot_token) || !hex(data.context.rules_sha256) || !hex(data.context.typed_report_sha256)) throw Error('Segmentantwort ohne passende Snapshot-Bindung.');
      if(context && key(context)!==key(data.context)) throw Error('Segmentseiten gehören zu unterschiedlichen Snapshots.');
      if (!data.cohort || !data.cohort.summary || !integer(data.cohort.summary.selected_pages) || !integer(data.cohort.summary.selected_issues) || !integer(data.cohort.summary.total_pages) || data.cohort.summary.selected_pages > data.cohort.summary.total_pages || !Array.isArray(data.cohort.counts) || !data.cohort.charts) throw Error('Segmentantwort ohne vollständige Populationszahlen.');
      if(key(data.cohort.rules)!==key(token.rules) || key(data.cohort.selection)!==key(token.selection))throw Error('Segmentantwort gehört zu anderen Regeln oder einer anderen Auswahl.');
      const summary=data.cohort.summary;
      if(['internal_pages','html_pages','indexable_pages','failed_pages','multiple_matches','unassigned_pages','unknown_pages','orphan_issues'].some(name=>!integer(summary[name])) || summary.internal_pages>summary.selected_pages || summary.html_pages>summary.selected_pages || summary.indexable_pages>summary.selected_pages || summary.failed_pages>summary.selected_pages || !summary.issue_counts || ['error','warning','notice'].some(name=>!integer(summary.issue_counts[name])) || !data.cohort.charts.overview || ['success','redirect','client_error','server_error','unfetched'].some(name=>!integer(data.cohort.charts.overview[name])))throw Error('Segmentantwort enthält unbestätigte Kennzahlen.');
    }
    async function apply() {
      if(!state.draftValid){notify();return;}
      invalidate();
      const observed=owner();
      if(!observed?.id || !integer(observed.nativeGeneration) || !observed.stable) { state.error='Segmente erst auf einen gespeicherten, angehaltenen oder abgeschlossenen Crawl anwenden.';notify();return; }
      const token={owner:clone(observed),ownerKey:key(observed),revision:state.revision,controller:new AbortController(),rules:clone(state.rules),selection:selection(state.selection,state.rules),mode:state.reportingMode};
      state.pending=token;state.busy=true;notify();
      try {
        let context=null,cohort=null,captured=null;const pages=[],issues=[];
        for(const view of ['pages','issues']){
          let cursor=null,total=null;
          do {
            const {data}=await post(token,'dashboard',requestBody(token,view,cursor,context));
            if(!current(token))return;
            validateContext(data,token,context);
            if(context && (key(cohort)!==key(data.cohort) || key(captured)!==key(data.captured_crawl_summary)))throw Error('Segmentpopulation änderte sich während des Ladens.');
            context??=data.context;cohort??=data.cohort;captured??=data.captured_crawl_summary;
            const collected=view==='pages'?pages:issues;
            if(data.view!==view || !integer(data.total) || !Array.isArray(data.rows) || data.rows.length>200 || (total!==null && total!==data.total) || data.total!==(view==='pages'?cohort.summary.selected_pages:cohort.summary.selected_issues))throw Error('Ungültige Segmentseitengrenze.');
            total=data.total;
            for(const row of data.rows){
              if(view==='pages'){
                if(!integer(row.page_ordinal) || (pages.length && row.page_ordinal<=pages[pages.length-1].page_ordinal) || !row.page || typeof row.page.url!=='string' || !row.membership || !Array.isArray(row.membership.matched_indices) || !Array.isArray(row.membership.unknown_indices))throw Error('Segmentseiten enthalten fehlende, doppelte oder ungeordnete Beobachtungen.');
              }else if(!row || typeof row.url!=='string' || typeof row.code!=='string')throw Error('Ungültiger gespeicherter Issue.');
              collected.push(row);
            }
            if(collected.length>total)throw Error('Zu viele Segmentzeilen.');
            cursor=data.next;
            if(cursor && (cursor.view!==view || cursor.offset!==collected.length || cursor.snapshot_token!==context.snapshot_token || !data.rows.length || collected.length>=total))throw Error('Ungültiger Segmentcursor.');
            if(!cursor && collected.length!==total)throw Error('Segmentpopulation wurde nicht vollständig geliefert.');
          }while(cursor);
        }
        const {data}=await post(token,'dashboard-analysis',requestBody(token,'pages',null,context));
        if(!current(token))return;
        validateContext(data,token,context);
        if(key(data.cohort)!==key(cohort) || !data.analysis || !Array.isArray(data.target_lookup) || !Array.isArray(data.responses) || !data.fragments || typeof data.fragments.enabled!=='boolean' || !Array.isArray(data.fragments.observations))throw Error('Segmentanalyse ohne passende vollständige Quelle.');
        const result={context,cohort,captured_crawl_summary:captured,pages,issues,analysis:data.analysis,target_lookup:data.target_lookup,responses:data.responses,fragments:data.fragments,rules:token.rules,selection:token.selection,reporting_mode:token.mode};
        state.applied=result;state.error='';deps.onInstall?.(result);notify();
      }catch(error){if(current(token)&&error.name!=='AbortError'){invalidate(error.message);}}
      finally{if(owns(token)){state.pending=null;state.busy=false;notify();}}
    }
    async function download(kind) {
      if(!state.applied || state.busy)return;
      if(!['json','pages','issues','links','dom','extractions','javascript','cookies','pdf','images','custom-search','accessibility','responses','fragments'].includes(kind))throw Error('Unbekannter Segmentexport.');
      const applied=state.applied,observed=owner();
      if(!observed?.stable || observed.id!==applied.context.job_id || observed.nativeGeneration!==applied.context.app_generation){invalidate('Der gespeicherte Crawlstand hat sich geändert. Segment erneut anwenden.');return;}
      const token={owner:clone(observed),ownerKey:key(observed),revision:state.revision,controller:new AbortController(),rules:applied.rules,selection:applied.selection,mode:applied.reporting_mode};
      state.pending=token;state.busy=true;notify();
      try{
        const {bytes,data}=await post(token,kind==='json'?'dashboard.json':`dashboard/${kind}.csv`,requestBody(token,'pages',null,applied.context),applied.context,kind!=='json');
        if(kind==='json' && (data.kind!=='segment_dashboard_cohort_not_raw_crawl' || key(data.context)!==key(applied.context) || key(data.cohort)!==key(applied.cohort) || !Array.isArray(data.pages) || !Array.isArray(data.issues) || data.pages.length!==applied.pages.length || data.issues.length!==applied.issues.length || data.pages.some((row,index)=>row.page_ordinal!==applied.pages[index].page_ordinal)))throw Error('Segment-JSON enthält nicht die bestätigte vollständige Population.');
        if(current(token)&&state.applied===applied)deps.onDownload?.(bytes,kind,applied);
      }catch(error){if(current(token)&&error.name!=='AbortError')invalidate(error.message);}
      finally{if(owns(token)){state.pending=null;state.busy=false;notify();}}
    }
    return {state,configure,invalidate,apply,download};
  }
  return {KEY,validateRules,selection,createController};
});
