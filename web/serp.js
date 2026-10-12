'use strict';
(() => {
  const $=selector=>document.querySelector(selector), KEY='open-seo-spider.serp.v1';
  let rows=[],preview=[],selected=0,page=0,revision=0,importRevision=0,draftRevision=0;
  const status=(text,error=false)=>{$('#status').textContent=text;$('#status').setAttribute('role',error?'alert':'status');};
  const inputRow=row=>({url:row.url,title:row.title,description:row.description});
  const width=value=>value===null||value===undefined?'unbekannt':`${value} px`;
  const cell=(tr,value)=>{const td=document.createElement('td');td.textContent=String(value??'');tr.append(td);return td;};
  function persist(){try{localStorage.setItem(KEY,JSON.stringify(rows));return true;}catch{status('Entwurf ist bearbeitbar, konnte aber nicht im Browserspeicher gespeichert werden. Bitte als JSON exportieren.',true);return false;}}
  function render(syncEditor=true){
    const current=rows[selected], measured=preview[selected];
    $('#editing-row').textContent=current?`Zeile ${selected+1} bearbeiten`:'Zeile hinzufügen';
    if(syncEditor)for(const key of ['url','title','description'])$(`#${key}`).value=current?.[key]??'';
    $('#delete-row').disabled=!current;
    $('#preview-url').textContent=current?.url??'';$('#preview-title').textContent=current?.title??'';$('#preview-description').textContent=current?.description??'';
    $('#measurements').textContent=measured?`Titel: ${measured.title_characters} Zeichen · ${width(measured.title_pixels)}. Beschreibung: ${measured.description_characters} Zeichen · ${width(measured.description_pixels)}.`:'';
    const tbody=$('#rows-table tbody');tbody.replaceChildren();
    for(let index=page*100;index<Math.min(rows.length,(page+1)*100);index++){
      const row=rows[index],m=preview[index],tr=document.createElement('tr');
      const first=cell(tr,'');const button=document.createElement('button');button.type='button';button.textContent=String(index+1);button.setAttribute('aria-label',`Zeile ${index+1} bearbeiten`);button.addEventListener('click',()=>{selected=index;render();});first.append(button);
      cell(tr,row.url);cell(tr,row.title);cell(tr,m?.title_characters??'');cell(tr,width(m?.title_pixels));cell(tr,row.description);tbody.append(tr);
    }
    $('#row-count').textContent=rows.length?`${page*100+1}–${Math.min(rows.length,(page+1)*100)} von ${rows.length} Zeilen`:'0 Zeilen';
    $('#previous').disabled=page===0;$('#next').disabled=(page+1)*100>=rows.length;
    $('#export-csv').disabled=rows.length===0;$('#export-json').disabled=rows.length===0;
  }
  async function api(route,body){const response=await fetch(route,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});if(!response.ok){let message=`HTTP ${response.status}`;try{message=(await response.json()).error??message;}catch{}throw Error(message);}return response;}
  async function measure(){const version=++revision;preview=[];render(false);try{const result=await(await api('/api/serp/preview',{rows})).json();if(version!==revision)return;preview=result.rows;$('#profile').textContent=result.measurement_profile;render(false);if(persist())status(`${rows.length} Zeilen gemessen und lokal gespeichert · ${result.website_requests} Website-Anfragen.`);}catch(error){if(version===revision)status(error.message,true);}}
  function saveBlob(blob,name){const url=URL.createObjectURL(blob),a=document.createElement('a');a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}
  $('#editor').addEventListener('submit',event=>{event.preventDefault();const row={url:$('#url').value,title:$('#title').value,description:$('#description').value};if(!rows.length)rows.push(row);else rows[selected]=row;void measure();});
  $('#add-row').addEventListener('click',()=>{if(rows.length>=10000){status('Maximal 10.000 Zeilen.',true);return;}rows.push({url:'',title:'',description:''});selected=rows.length-1;page=Math.floor(selected/100);render();void measure();});
  $('#delete-row').addEventListener('click',()=>{rows.splice(selected,1);selected=Math.max(0,Math.min(selected,rows.length-1));page=Math.min(page,Math.max(0,Math.ceil(rows.length/100)-1));render();void measure();});
  for(const key of ['url','title','description'])$(`#${key}`).addEventListener('input',()=>{draftRevision++;});
  $('#csv-file').addEventListener('change',async()=>{
    const file=$('#csv-file').files[0];if(!file)return;
    const version=++revision,importId=++importRevision,draft=draftRevision;
    const current=()=>version===revision&&draft===draftRevision;
    try{
      if(file.size>8*1024*1024)throw Error('CSV überschreitet 8 MiB.');
      const csv=await file.text();if(!current())return;
      const result=await(await api('/api/serp/import',{csv})).json();if(!current())return;
      rows=result.rows.map(inputRow);preview=result.rows;selected=0;page=0;$('#profile').textContent=result.measurement_profile;render();
      if(persist())status(`${rows.length} Zeilen importiert · Reihenfolge erhalten · ${result.website_requests} Website-Anfragen.`);
    }catch(error){if(current())status(error.message,true);}
    finally{if(importId===importRevision)$('#csv-file').value='';}
  });
  $('#export-csv').addEventListener('click',async()=>{try{saveBlob(await(await api('/api/serp/export.csv',{rows})).blob(),'serp.csv');}catch(error){status(error.message,true);}});
  $('#export-json').addEventListener('click',()=>saveBlob(new Blob([JSON.stringify({schema_version:1,rows},null,2)],{type:'application/json'}),'serp-draft.json'));
  $('#previous').addEventListener('click',()=>{page=Math.max(0,page-1);render(false);});$('#next').addEventListener('click',()=>{page++;render(false);});
  try{const stored=JSON.parse(localStorage.getItem(KEY)||'[]');if(Array.isArray(stored)&&stored.length<=10000&&stored.every(row=>row&&['url','title','description'].every(key=>typeof row[key]==='string'&&row[key].length<=65536)))rows=stored.map(inputRow);}catch{status('Gespeicherter Entwurf konnte nicht geladen werden.',true);}
  render();if(rows.length)void measure();
})();
