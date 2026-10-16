"use strict";
// SOURCE28_PURE_BEGIN
function spellingWords(value) {
  const rows=value.split(/\r?\n/).map(v=>v.trim()).filter(Boolean);
  return rows.map(row=>{const p=row.split("|").map(v=>v.trim());if(p.length!==2||!p[0]||!p[1])throw Error("Wörter benötigen Sprache | Wort");return {language:p[0],word:p[1]};});
}
function spellingDictionaryText(words) {return words.map(v=>v.language+" | "+v.word).join("\n");}
function spellingValidateReply(value,request,id) {
  const r=value?.report;
  if(value?.crawl_id!==id||r?.engine!=="open-seo-spider/saved-spelling-v1"||r.website_requests!==0||r.offset_unit!=="utf16_code_units")throw Error("Unerwartete Antwortidentität");
  if(JSON.stringify(r.request)!==JSON.stringify(request)) {
    // Object key order is not an identity. Compare canonical recursively.
    const canon=v=>Array.isArray(v)?v.map(canon):v&&typeof v==="object"?Object.fromEntries(Object.keys(v).sort().map(k=>[k,canon(v[k])])):v;
    if(JSON.stringify(canon(r.request))!==JSON.stringify(canon(request)))throw Error("Antwort gehört zu einer anderen Anfrage");
  }
  const s=request.selection,snapshot=r.saved_capture?.snapshot;
  if(r.crawl_id!==id||JSON.stringify(r.saved_capture?.requested_urls)!==JSON.stringify(s.urls)||r.saved_capture?.source_kind!==s.source_kind)throw Error("Gespeicherte Auswahl gehört zu einer anderen Anfrage");
  if(s.reporting_mode&&r.saved_capture?.reporting?.identity?.mode!==s.reporting_mode)throw Error("Berichtsmodus fehlt oder weicht ab");
  if(s.expected_snapshot_sha256&&snapshot?.sha256!==s.expected_snapshot_sha256)throw Error("Erfassung geändert");
  if(s.expected_generation!=null&&snapshot?.generation!==s.expected_generation)throw Error("Generation geändert");
  if(s.expected_reporting_identity&&r.saved_capture?.reporting?.identity_sha256!==s.expected_reporting_identity)throw Error("Ansichtsprofil geändert");
  if(r.dictionary_sha256!==request.expected_dictionary_sha256||!Array.isArray(r.documents)||r.documents.length>32||!Number.isInteger(r.engine_requests)||r.engine_requests<0||r.engine_requests>32)throw Error("Wörterbuch oder Antwortgrenze abweichend");
  if(r.complete&&(!r.binding_current||!Number.isSafeInteger(r.accepted_count)||r.accepted_count<0))throw Error("Vollständige Antwort ohne gültige Bindung/Fehlerzahl");
  if(!r.complete&&r.accepted_count!==null)throw Error("Unvollständige Antwort beansprucht Fehlerzahl");
  return r;
}
// SOURCE28_PURE_END
(() => {
  const $=id=>document.getElementById(id);
  let serial=0,controller=null,inventory=null,next=null,selected=new Set(),dictionarySha=null,dictionaryDirty=false,profileSha=null,result=null;
  function status(text){$("status").textContent=text;}
  function invalidate(message="Auswahl/Einstellungen geändert. Eine neue Prüfung ist erforderlich.") {
    serial++;controller?.abort();controller=null;result=null;$("csv").disabled=true;$("results").hidden=true;status(message);
  }
  function resetSelection(){invalidate();inventory=null;next=null;selected=new Set();$("captures").replaceChildren();$("more").disabled=true;$("capture-info").textContent="Inventar erneut laden.";profileSha=null;$("save-profile").disabled=true;}
  function begin(){invalidate("Vorgang läuft …");controller=new AbortController();return {token:serial,signal:controller.signal};}
  function current(op){if(op.token!==serial)throw Error("Veraltete Antwort verworfen");}
  async function api(path,op,body) {
    const response=await fetch(path,{method:body===undefined?"GET":"POST",headers:body===undefined?{}:{"Content-Type":"application/json"},body:body===undefined?undefined:JSON.stringify(body),signal:AbortSignal.any([op.signal,AbortSignal.timeout(100000)]),cache:"no-store"});
    const reader=response.body?.getReader();if(!reader)throw Error("Antwortkörper fehlt");let bytes=0,chunks=[];
    while(true){const {done,value}=await reader.read();if(done)break;bytes+=value.length;if(bytes>8*1024*1024){await reader.cancel();throw Error("Antwort überschreitet8MiB");}chunks.push(value);}
    const raw=new Uint8Array(bytes);let at=0;for(const c of chunks){raw.set(c,at);at+=c.length;}
    const value=JSON.parse(new TextDecoder("utf-8",{fatal:true}).decode(raw));current(op);
    if(!response.ok)throw Error(value.error||"HTTP"+response.status);return value;
  }
  const id=()=>{if(!$("crawl").value)throw Error("Crawl auswählen");return $("crawl").value;};
  const path=tail=>"/api/crawls/"+encodeURIComponent(id())+"/spelling/"+tail;
  function config(){
    return {version:1,enabled:$("enabled").checked,endpoint:$("endpoint").value.trim()||null,
      acknowledge_local_text_transfer:$("ack").checked,expected_engine_version:$("engine-version").value.trim()||null,
      spelling:$("spelling").checked,grammar:$("grammar").checked,manual_language:$("manual").value.trim()||null,
      use_html_language:$("html-lang").checked,fallback_language:$("fallback").value.trim(),
      regional_defaults:JSON.parse($("regions").value),ignored_words:spellingWords($("ignored-words").value),
      ignored_rules:$("ignored-rules").value.split(/\r?\n/).map(v=>v.trim()).filter(Boolean)};
  }
  function applyConfig(c) {
    for(const [key,name]of [["enabled","enabled"],["spelling","spelling"],["grammar","grammar"],["ack","acknowledge_local_text_transfer"],["html-lang","use_html_language"]])$(key).checked=c[name];
    for(const [key,name]of [["endpoint","endpoint"],["engine-version","expected_engine_version"],["manual","manual_language"],["fallback","fallback_language"]])$(key).value=c[name]||"";
    $("regions").value=JSON.stringify(c.regional_defaults);$("ignored-words").value=spellingDictionaryText(c.ignored_words);$("ignored-rules").value=c.ignored_rules.join("\n");
  }
  function action(button,work){$(button).addEventListener("click",()=>{const op=begin();Promise.resolve().then(()=>work(op)).catch(e=>{if(op.token===serial)status(e.name==="AbortError"?"Anzeige abgebrochen. Ein serverseitig begonnener Prüflauf kann bereits gespeichert sein.":e.message);});});}
  function addRows(rows) {for(const row of rows){
    const tr=document.createElement("tr"),choose=document.createElement("input");choose.type="checkbox";choose.checked=selected.has(row.requested_url);
    choose.addEventListener("change",()=>{invalidate();if(choose.checked){if(selected.size>=32){choose.checked=false;status("Maximal32 URLs");return;}selected.add(row.requested_url);}else selected.delete(row.requested_url);});
    const first=document.createElement("td");first.append(choose);tr.append(first);
    for(const text of [row.requested_url+" · #"+row.ordinal,JSON.stringify({status:row.status,reason:row.reason,html_language:row.html_language,complete:row.sample_complete,text_bytes:row.text_bytes})]){const td=document.createElement("td");td.textContent=text;tr.append(td);}
    $("captures").append(tr);
  }}
  async function loadInventory(op,more) {
    const params=new URLSearchParams({source_kind:$("source").value,limit:"200",after_ordinal:String(more?next:0)});
    if($("mode").value)params.set("reporting_mode",$("mode").value);
    if(more&&inventory){params.set("expected_snapshot_sha256",inventory.snapshot.sha256);if(inventory.snapshot.generation!=null)params.set("expected_generation",inventory.snapshot.generation);if(inventory.reporting)params.set("expected_reporting_identity",inventory.reporting.identity_sha256);}
    const value=await api("/api/crawls/"+encodeURIComponent(id())+"/content?"+params,op);
    if(!more){selected=new Set();$("captures").replaceChildren();inventory=value;}else if(value.snapshot.sha256!==inventory.snapshot.sha256)throw Error("Inventar geändert");
    addRows(value.rows);next=value.next_after_ordinal;$("more").disabled=!value.has_more;
    $("capture-info").textContent="Snapshot "+value.snapshot.sha256+" · "+value.snapshot.origin+" · "+value.total_saved_pages+" rohe Beobachtungen. Sichtbarkeit wird vor Prüfung angewendet; Auswahl maximal32.";status("Inventar geladen.");
  }
  function render(r){
    $("results").hidden=false;$("summary").textContent=(r.complete&&r.binding_current?"Vollständige Prüfung der gewählten erfassten Texte: "+r.accepted_count+" Befunde.":"Unvollständig, deaktiviert oder historisch: Gesamtzahl unbekannt.")+" "+r.binding_qualification;
    $("rules").textContent=r.rules.map(v=>v.rule_id+": "+v.issues+" beobachtete Befunde / "+v.affected_urls+" URLs").join("\n")||"Keine beobachteten akzeptierten Regeln; siehe Prüfstatus.";
    $("documents").replaceChildren();
    for(const d of r.documents){const detail=document.createElement("details"),summary=document.createElement("summary");summary.textContent=d.url+" · "+d.status+" · "+(d.accepted_count===null?"unbekannt":d.accepted_count)+" · "+(d.reason||"");detail.append(summary);
      const lang=document.createElement("p");lang.textContent=JSON.stringify({language:d.language,engine:d.checked?.software,source:d.source,http_status:d.provider?.http_status,response_complete:d.provider?.body_complete});detail.append(lang);
      const checked=document.createElement("pre");checked.textContent=d.text;detail.append(checked);
      for(const i of d.checked?.issues||[]){const p=document.createElement("p");p.textContent=i.rule_id+" ["+i.disposition+"] "+i.message+" · UTF16 "+i.offset_utf16+"+"+i.length_utf16+" · "+JSON.stringify(i.matched_text)+" → "+i.replacements.join(" / ");detail.append(p);const c=document.createElement("pre");c.textContent=i.context;detail.append(c);}
      if(d.provider?.error){const p=document.createElement("p");p.textContent="Dienstfehler: "+d.provider.error;detail.append(p);}$("documents").append(detail);}
    $("provenance").textContent=JSON.stringify({run_id:r.run_id,saved_capture:r.saved_capture,config_sha256:r.config_sha256,dictionary_sha256:r.dictionary_sha256,website_requests:r.website_requests,engine_requests:r.engine_requests,limitations:r.limitations},null,2);
    $("csv").disabled=false;status("Prüflauf gespeichert. Einstellungen und Wörterbuch können geändert und gespeicherte Texte erneut geprüft werden.");
  }
  action("refresh",async op=>{const value=await api("/api/crawls",op);$("crawl").replaceChildren(new Option("Crawl auswählen",""));for(const c of value.crawls)$("crawl").append(new Option(c.id+" · "+c.status,c.id));resetSelection();});
  $("crawl").addEventListener("change",resetSelection);$("source").addEventListener("change",resetSelection);$("mode").addEventListener("change",resetSelection);
  for(const field of ["enabled","spelling","grammar","endpoint","engine-version","ack","manual","html-lang","fallback","regions","ignored-words","ignored-rules","dictionary"])$(field).addEventListener("input",()=>{if(field==="dictionary")dictionaryDirty=true;invalidate();});
  action("inventory",op=>loadInventory(op,false));action("more",op=>loadInventory(op,true));
  action("load-profile",async op=>{const v=await api(path("profile"),op);applyConfig(v.profile.config);profileSha=v.sha256;$("save-profile").disabled=false;status("Gespeicherte Einstellungen ausdrücklich in den Entwurf geladen.");});
  action("save-profile",async op=>{if(!profileSha)throw Error("Zuerst gespeicherte Einstellungen laden");const v=await api(path("profile"),op,{version:1,expected_sha256:profileSha,profile:{config:config()}});profileSha=v.sha256;status("Crawlbezogene Einstellungen gespeichert.");});
  action("load-dictionary",async op=>{const v=await api("/api/spelling/dictionary",op);$("dictionary").value=spellingDictionaryText(v.dictionary.words);dictionarySha=v.sha256;dictionaryDirty=false;$("save-dictionary").disabled=false;$("dictionary-info").textContent="Geladene Anwendungsrevision "+dictionarySha;status("Dauerhaftes Wörterbuch geladen.");});
  action("save-dictionary",async op=>{if(!dictionarySha)throw Error("Wörterbuch zuerst laden");const v=await api("/api/spelling/dictionary",op,{version:1,expected_sha256:dictionarySha,dictionary:{version:1,words:spellingWords($("dictionary").value)}});dictionarySha=v.sha256;dictionaryDirty=false;$("dictionary-info").textContent="Gespeicherte Anwendungsrevision "+dictionarySha;status("Wörterbuch gespeichert. Erneut prüfen für neue Ergebnisse.");});
  action("check",async op=>{if(dictionaryDirty)throw Error("Wörterbuchentwurf zuerst speichern oder ausdrücklich neu laden");if(!inventory||!selected.size||!dictionarySha)throw Error("Inventar, URL-Auswahl und Wörterbuchrevision laden");
    const request={version:1,selection:{urls:[...selected],source_kind:$("source").value,reporting_mode:$("mode").value||null,expected_generation:inventory.snapshot.generation,expected_snapshot_sha256:inventory.snapshot.sha256,expected_reporting_identity:inventory.reporting?.identity_sha256||null},config:config(),expected_dictionary_sha256:dictionarySha};
    const crawl=id(),value=await api(path("analyze"),op,request);const r=spellingValidateReply(value,request,crawl);current(op);result={report:r,crawl};render(r);});
  $("cancel").addEventListener("click",()=>invalidate("Anzeige abgebrochen. Begonnene serverseitige Prüfläufe können bereits gespeichert sein."));
  $("csv").addEventListener("click",async()=>{const snapshot=result;if(!snapshot)return;const token=serial;try{
    const r=await fetch("/api/crawls/"+encodeURIComponent(snapshot.crawl)+"/spelling/runs/"+encodeURIComponent(snapshot.report.run_id)+"/csv",{cache:"no-store",signal:AbortSignal.timeout(15000)});
    if(!r.ok)throw Error("CSV HTTP"+r.status);const reader=r.body?.getReader();if(!reader)throw Error("CSV-Körper fehlt");let length=0,chunks=[];while(true){const {done,value}=await reader.read();if(done)break;length+=value.length;if(length>8*1024*1024){await reader.cancel();throw Error("CSV-Grenze");}chunks.push(value);}const bytes=new Uint8Array(length);let at=0;for(const chunk of chunks){bytes.set(chunk,at);at+=chunk.length;}if(!bytes.length)throw Error("CSV leer");
    const hash=Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256",bytes)),v=>v.toString(16).padStart(2,"0")).join("");
    if(hash!==r.headers.get("x-content-sha256")||Number(r.headers.get("content-length"))!==bytes.length)throw Error("CSV-Länge/Hash abweichend");
    if(token!==serial||result!==snapshot)throw Error("CSV gehört zur vorherigen Auswahl");const url=URL.createObjectURL(new Blob([bytes],{type:"text/csv;charset=utf-8"})),a=document.createElement("a");a.href=url;a.download="spelling-"+snapshot.report.run_id+".csv";a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
  }catch(e){status(e.message);}});
})();