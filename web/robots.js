'use strict';
(() => {
  const $=selector=>document.querySelector(selector), bytes=value=>new TextEncoder().encode(value).length;
  const limits={max_robots_bytes:['#max-robots-bytes',524288],max_urls:['#max-urls',10000],max_url_bytes:['#max-url-bytes',8192],max_input_bytes:['#max-input-bytes',8388608],max_evaluation_bytes:['#max-evaluation-bytes',67108864]};
  let revision=0,result=null,submittedInput=null,page=0;
  const status=(text,error=false)=>{$('#status').textContent=text;$('#status').setAttribute('role',error?'alert':'status');};
  const cell=(tr,value)=>{const td=document.createElement('td');td.textContent=String(value??'');tr.append(td);return td;};
  function invalidate(){revision++;result=null;submittedInput=null;$('#result-card').hidden=true;$('#export-csv').disabled=true;$('#export-json').disabled=true;status('Eingabe geändert. Erneut prüfen, um ein aktuelles Ergebnis zu erhalten.');}
  function input(){
    const request={robots_text:$('#robots-text').value,user_agent:$('#user-agent').value,urls:[],limits:{}};
    if(!request.user_agent.trim()||bytes(request.user_agent)>256||/[^\x20-\x7e]/.test(request.user_agent))throw Error('Agent: 1 bis 256 ASCII-Bytes ohne Steuerzeichen eingeben.');
    for(const [key,[selector,ceiling]] of Object.entries(limits)){
      const raw=$(selector).value,n=Number(raw);
      if(!raw.trim()||!Number.isSafeInteger(n)||n<1||n>ceiling)throw Error(`${key}: ganze Zahl zwischen 1 und ${ceiling} eingeben.`);
      request.limits[key]=n;
    }
    if(!$('#urls').value)throw Error('Mindestens eine URL-Zeile eingeben.');
    request.urls=$('#urls').value.split(/\r\n|\r|\n/);
    if(request.urls.at(-1)==='')request.urls.pop();
    if(!request.urls.length||request.urls.length>request.limits.max_urls)throw Error(`Zwischen 1 und ${request.limits.max_urls} URL-Zeilen eingeben.`);
    const ruleBytes=bytes(request.robots_text);
    if(ruleBytes>request.limits.max_robots_bytes)throw Error('Regeltext überschreitet die eingestellte UTF-8-Bytegrenze.');
    if(request.urls.some(url=>bytes(url)>request.limits.max_url_bytes))throw Error('Eine URL überschreitet die eingestellte Bytegrenze.');
    if(ruleBytes+bytes(request.user_agent)+request.urls.reduce((n,url)=>n+bytes(url),0)>request.limits.max_input_bytes)throw Error('Gesamteingabe überschreitet die eingestellte Bytegrenze.');
    if(Math.max(1,ruleBytes)*request.urls.length>request.limits.max_evaluation_bytes)throw Error('Regelbytes × URL-Zeilen überschreiten die Prüfgrenze. Kleinere Listen verwenden.');
    return request;
  }
  async function api(path,request,text=false){
    const response=await fetch(path,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(request)});
    if(!response.ok){let message=`Lokale Prüfung fehlgeschlagen (${response.status}).`;try{const error=await response.json();message=error.error||error.message||message;}catch{}throw Error(message);}
    return text?response.text():response.json();
  }
  function render(){
    if(!result)return;
    $('#result-card').hidden=false;$('#export-csv').disabled=false;$('#export-json').disabled=false;
    $('#facts').textContent=`Agent: ${result.user_agent} · Website-Anfragen: ${result.website_requests} · Regeltext: ${result.rules_bytes} Bytes · SHA256: ${result.rules_sha256} · Engine: ${result.engine}`;
    $('#ignored').textContent=result.ignored_directives.length?`Für diese Zugriffsentscheidung ignorierte Felder: ${result.ignored_directives.join(', ')}.`:'Keine zusätzlichen ignorierten Felder erfasst.';
    const type=$('#decision-filter').value;
    const selected=result.rows.filter(row=>type==='all'||(type==='allowed'&&row.allowed===true)||(type==='blocked'&&row.allowed===false)||(type==='unknown'&&row.allowed===null));
    page=Math.min(page,Math.max(0,Math.ceil(selected.length/100)-1));
    const tbody=$('#results-table tbody');tbody.replaceChildren();
    for(const row of selected.slice(page*100,(page+1)*100)){
      const tr=document.createElement('tr');tr.dataset.sourceRow=String(row.source_row);
      cell(tr,row.source_row);cell(tr,row.input_url);cell(tr,row.normalized_url);cell(tr,row.path_query);cell(tr,row.fragment);
      cell(tr,row.allowed===true?'Erlaubt':row.allowed===false?'Gesperrt':'Unbekannt').className='decision';cell(tr,row.error);tbody.append(tr);
    }
    $('#row-count').textContent=`${selected.length} von ${result.rows.length} Zeilen · Seite ${page+1}`;
    $('#previous').disabled=page===0;$('#next').disabled=(page+1)*100>=selected.length;
    const list=$('#limitations');list.replaceChildren();for(const text of result.limitations){const li=document.createElement('li');li.textContent=text;list.append(li);}
  }
  function download(name,value,type){const url=URL.createObjectURL(new Blob([value],{type}));const a=document.createElement('a');a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),0);}
  $('#agent-preset').addEventListener('change',()=>{if($('#agent-preset').value!=='custom')$('#user-agent').value=$('#agent-preset').value;invalidate();});
  for(const selector of ['#robots-text','#urls','#user-agent',...Object.values(limits).map(([selector])=>selector)])$(selector).addEventListener('input',()=>{if(selector==='#user-agent')$('#agent-preset').value='custom';invalidate();});
  $('#robots-form').addEventListener('submit',async event=>{
    event.preventDefault();const current=++revision;result=null;submittedInput=null;$('#result-card').hidden=true;$('#export-csv').disabled=true;$('#export-json').disabled=true;
    try{
      const request=input();status('Lokale Regeln werden geprüft …');const response=await api('/api/robots/simulate',request);
      if(current!==revision)return;
      result=response;submittedInput=request;page=0;$('#decision-filter').value='all';render();
      const invalid=result.rows.filter(row=>row.allowed===null).length;status(`${result.rows.length} Zeilen geprüft · ${invalid} unbekannt · ${result.website_requests} Website-Anfragen.`);
    }catch(error){if(current===revision)status(error.message,true);}
  });
  $('#decision-filter').addEventListener('change',()=>{page=0;render();});
  $('#previous').addEventListener('click',()=>{page=Math.max(0,page-1);render();});$('#next').addEventListener('click',()=>{page++;render();});
  $('#export-csv').addEventListener('click',async()=>{
    if(!result||!submittedInput)return;const current=revision,request=submittedInput;
    try{const csv=await api('/api/robots/export.csv',request,true);if(current===revision&&submittedInput===request)download('open-seo-spider-robots.csv',csv,'text/csv;charset=utf-8');}
    catch(error){if(current===revision)status(error.message,true);}
  });
  $('#export-json').addEventListener('click',()=>{if(result&&submittedInput)download('open-seo-spider-robots.json',JSON.stringify({schema_version:1,input:submittedInput,result},null,2),'application/json');});
})();
