'use strict';
(function(host,factory){const api=factory();if(typeof module==='object'&&module.exports)module.exports=api;else host.TableViewsUI=api;})(typeof globalThis==='object'?globalThis:this,function(){
  const ok=value=>({ok:true,value}),failure=(code,message)=>({ok:false,error:{code,message}});
  const message=result=>result?.error?.message||'Die benannte Ansicht konnte nicht verarbeitet werden.';
  function createController(hooks){
    const {library,store}=hooks;
    const state={raw:null,entries:[],loaded:false,catalogValid:false,selectedId:'',name:'',busy:false,activity:'',error:'',notice:'',active:null,deleteConfirmation:null,resetConfirmation:null};
    let context=null,revision=0,operation=null,resetRaw,imported=null;
    const available=()=>{try{return hooks.available?hooks.available()===true:true;}catch(_){return false;}};
    const unavailable=()=>failure('inactive_query_mode','Diese Ansichten gehören zum anderen Filtermodus. Zuerst den passenden Modus auswählen.');
    const notify=()=>hooks.notify?.();
    const contractStamp=()=>{const c=hooks.contract();return JSON.stringify([c.section,c.query_semantics,c.columns.map(x=>[x.key,x.type??'auto',Boolean(x.mandatory)]),c.sectionFilters]);};
    const ownerStamp=()=>JSON.stringify([hooks.owner(),contractStamp(),available()]);
    const stamp=()=>JSON.stringify([ownerStamp(),hooks.queryRevision(),revision]);
    function invalidate(queryChanged=true){revision++;operation=null;state.busy=false;state.activity='';imported=null;state.deleteConfirmation=null;state.resetConfirmation=null;if(queryChanged&&state.active)state.active={...state.active,modified:true};}
    function synchronize(){let next;try{next=ownerStamp();}catch(_){next=null;}if(next!==context){invalidate(false);context=next;resetRaw=undefined;state.selectedId='';state.name='';state.active=null;state.error='';state.notice='';}}
    const selected=()=>state.entries.find(entry=>entry.id===state.selectedId&&entry.section===hooks.contract().section);
    function showError(result){state.error=message(result);notify();return result;}
    function refresh(){synchronize();invalidate(false);let result;try{result=store.read();}catch(_){result=failure('storage_unavailable','Der Browserkatalog ist nicht lesbar.');}if(!result.ok){resetRaw=Object.hasOwn(result,'observedRaw')?result.observedRaw:undefined;state.catalogValid=false;return showError(result);}resetRaw=result.value.raw;state.raw=result.value.raw;state.entries=result.value.entries;state.loaded=true;state.catalogValid=true;state.error='';const entry=selected();if(!entry){state.selectedId='';state.name='';}else state.name=entry.name;state.notice='Katalog gelesen. Die aktuelle Tabellenansicht bleibt unverändert.';notify();return result;}
    function select(id){synchronize();invalidate(false);if(id&& !state.entries.some(entry=>entry.id===id&&entry.section===hooks.contract().section))return showError(failure('unknown_view','Die gewählte Ansicht ist in diesem Bereich nicht verfügbar.'));state.selectedId=id;state.name=selected()?.name||'';state.error='';notify();return ok(null);}
    function name(value){invalidate(false);state.name=value;state.error='';notify();}
    function begin(reset=false,activity='catalog',keepImport=false){synchronize();if(!available())return showError(unavailable());if(!reset&&(!state.loaded||!state.catalogValid))return showError(failure('catalog_unavailable','Den gültigen Katalog zuerst ausdrücklich neu laden.'));if(!keepImport)imported=null;const ticket={stamp:stamp()};operation=ticket;state.busy=true;state.activity=activity;state.error='';state.deleteConfirmation=null;state.resetConfirmation=null;const guard=()=>available()&&operation===ticket&&ticket.stamp===stamp();notify();return{ticket,guard};}
    function stale(started,message){
      const result=failure('stale_operation',message);
      // A completion may arrive after a newer explicit operation has started.
      // Release only its own ticket, including a revision-only invalidation that
      // has not reached the render/invalidate callback yet.
      if(operation===started.ticket){operation=null;state.busy=false;state.activity='';if(imported&&imported.stamp!==stamp())imported=null;state.error=message;notify();}
      return result;
    }
    function requireCrawl(){return hooks.owner()?.projectId?ok(null):failure('no_crawl','Zuerst einen Crawl und eine Ergebnistabelle auswählen.');}
    async function mutate(kind){
      synchronize();if(!available())return showError(unavailable());if(state.busy)return failure('busy','Eine Katalogaktion läuft bereits.');
      if(!state.loaded||!state.catalogValid)return showError(failure('catalog_unavailable','Den gültigen Katalog zuerst ausdrücklich neu laden.'));
      if(kind==='create'||kind==='replace'){const crawl=requireCrawl();if(!crawl.ok)return showError(crawl);}
      const entry=selected();if(['replace','rename','delete'].includes(kind)&&!entry)return showError(failure('no_selection','Zuerst eine gespeicherte Ansicht auswählen.'));
      let next=[...state.entries],created;
      if(kind==='create'||kind==='replace'){
        let snapshot;try{snapshot=hooks.snapshot();}catch(_){snapshot=failure('invalid_working_view','Die vollständige aktuelle Ansicht ist nicht verfügbar.');}if(!snapshot.ok)return showError(snapshot);
        let id;try{id=kind==='replace'?entry.id:hooks.uuid();}catch(_){return showError(failure('uuid_unavailable','Eine eindeutige Ansichtskennung konnte nicht erzeugt werden.'));}
        const result=library.makeEntry({id,name:kind==='replace'?entry.name:state.name},hooks.contract(),snapshot.value);if(!result.ok)return showError(result);created=result.value;next=kind==='create'?[...next,created]:next.map(row=>row.id===entry.id?created:row);
      }else if(kind==='rename')next=next.map(row=>row.id===entry.id?{...row,name:state.name}:row);
      else if(kind==='delete')next=next.filter(row=>row.id!==entry.id);
      else return showError(failure('invalid_action','Unbekannte Katalogaktion.'));
      const encoded=library.encodeCatalog(next);if(!encoded.ok)return showError(encoded);
      const started=begin();if(!started.guard)return started;
      let result;try{result=await store.mutate({expectedRaw:state.raw,entries:next,guard:started.guard});}catch(_){result=failure('storage_unavailable','Der Katalog konnte nicht gespeichert werden.');}
      if(!started.guard())return stale(started,'Die Ansicht wurde zwischenzeitlich geändert; die alte Aktion wurde verworfen.');
      state.busy=false;operation=null;
      if(!result.ok)return showError(result);
      state.raw=result.value.raw;resetRaw=result.value.raw;state.entries=result.value.entries;state.catalogValid=true;state.error='';
      // Saving catalog metadata does not apply rows or rebind another selected
      // view. Keep the badge truthful only for the already active definition.
      if(state.active&&entry&&state.active.id===entry.id){
        if(kind==='replace')state.active={id:created.id,name:created.name,modified:false};
        else if(kind==='rename')state.active={...state.active,name:state.entries.find(row=>row.id===entry.id).name};
        else if(kind==='delete')state.active=null;
      }
      if(kind==='create'){state.selectedId=created.id;state.name=created.name;}
      if(kind==='rename')state.name=selected()?.name||'';
      if(kind==='delete'){state.selectedId='';state.name='';}
      state.notice=kind==='create'?'Neue Ansicht im Browserkatalog gespeichert. Die Tabelle wurde nicht angewendet.':kind==='replace'?'Gespeicherte Ansicht überschrieben. Die aktuelle Tabelle bleibt unverändert.':kind==='rename'?'Katalogeintrag umbenannt. Die aktuelle Tabelle bleibt unverändert.':'Katalogeintrag gelöscht. Die aktuelle Tabelle bleibt unverändert.';
      notify();return result;
    }
    function apply(){
      synchronize();if(!available())return showError(unavailable());if(state.busy)return failure('busy','Eine Katalogaktion läuft bereits.');if(!state.catalogValid)return showError(failure('catalog_unavailable','Den gültigen Katalog zuerst neu laden.'));const crawl=requireCrawl();if(!crawl.ok)return showError(crawl);const entry=selected();if(!entry)return showError(failure('no_selection','Zuerst eine gespeicherte Ansicht auswählen.'));
      const prepared=library.prepareApply(entry,hooks.contract());if(!prepared.ok)return showError(prepared);
      const started=begin();if(!started.guard)return started;
      let result;try{result=hooks.apply(prepared.value,started.guard);}catch(_){result=failure('apply_failed','Die Ansicht konnte nicht atomar angewendet werden.');}
      // A successful synchronous App commit deliberately advances its revision
      // and calls invalidate(). Do not mistake that accepted commit for stale work.
      if(result?.ok===true){state.busy=false;operation=null;state.active={id:entry.id,name:entry.name,modified:false};state.error='';state.notice='Vollständige Ansicht auf den aktuellen Crawl angewendet.';notify();return result;}
      if(!started.guard())return stale(started,'Die alte Anwendung wurde verworfen.');state.busy=false;operation=null;return showError(result||failure('apply_failed','Keine atomare Anwendung bestätigt.'));
    }
    function requestDelete(){synchronize();if(!available())return showError(unavailable());if(!state.catalogValid)return showError(failure('catalog_unavailable','Den gültigen Katalog zuerst neu laden.'));const entry=selected();if(!entry)return showError(failure('no_selection','Zuerst eine gespeicherte Ansicht auswählen.'));invalidate(false);state.deleteConfirmation={id:entry.id,name:entry.name,stamp:stamp()};state.error='';notify();return ok(null);}
    function confirmDelete(){const confirm=state.deleteConfirmation;if(!confirm||confirm.id!==state.selectedId||confirm.stamp!==stamp())return failure('stale_confirmation','Die Löschbestätigung ist nicht mehr aktuell.');return mutate('delete');}
    function cancelDelete(){invalidate(false);notify();}
    function requestReset(){synchronize();if(!available())return showError(unavailable());if(resetRaw===undefined)return showError(failure('unobserved_catalog','Katalog zuerst neu laden. Nicht gelesene Daten können nicht zurückgesetzt werden.'));invalidate(false);state.resetConfirmation={stamp:stamp()};notify();return ok(null);}
    async function confirmReset(){const confirm=state.resetConfirmation;if(!confirm||confirm.stamp!==stamp()||resetRaw===undefined)return failure('stale_confirmation','Die Rücksetzbestätigung ist nicht mehr aktuell.');const expectedRaw=resetRaw,started=begin(true);if(!started.guard)return started;let result;try{result=await store.reset({expectedRaw,guard:started.guard});}catch(_){result=failure('storage_unavailable','Der benannte Katalog konnte nicht zurückgesetzt werden.');}if(!started.guard())return stale(started,'Die alte Rücksetzung wurde verworfen.');state.busy=false;operation=null;if(!result.ok)return showError(result);state.raw=null;resetRaw=null;state.entries=[];state.loaded=true;state.catalogValid=true;state.selectedId='';state.name='';state.active=null;state.error='';state.notice='Der ausgewählte Ansichtskatalog wurde entfernt. Andere Ansichtskataloge, die aktuelle Tabelle und gespeicherte Spalten bleiben erhalten.';notify();return result;}
    function externalChange(){invalidate(false);state.catalogValid=false;resetRaw=undefined;state.notice='Der Katalog wurde in einem anderen Fenster geändert. Die Tabelle bleibt erhalten; Katalog ausdrücklich neu laden.';notify();}
    const portability=hooks.portability;
    function portable(method,...args){try{return typeof portability?.[method]==='function'?portability[method](...args):failure('portability_unavailable','Ansichtsdateien sind in dieser Umgebung nicht verfügbar.');}catch(_){return failure('invalid_import','Die vollständigen Ansichtsdefinitionen konnten nicht geprüft werden.');}}
    function portableEnd(started,result,drop=false){
      if(!started.guard())return stale(started,'Die Ansicht oder Datei wurde inzwischen geändert; die alte Dateiaktion wurde verworfen.');
      operation=null;state.busy=false;state.activity='';if(drop)imported=null;
      if(!result.ok)return showError(result);state.error='';notify();return result;
    }
    function observedCatalog(guard,expectedRaw=state.raw){
      let result;try{result=store.read();}catch(_){result=failure('storage_unavailable','Der Browserkatalog ist nicht lesbar.');}
      if(!guard())return failure('stale_operation','Die Dateiaktion gehört nicht mehr zur aktuellen Ansicht.');
      if(!result.ok){state.catalogValid=false;resetRaw=undefined;imported=null;return result;}
      if(result.value.raw!==expectedRaw){state.catalogValid=false;resetRaw=undefined;imported=null;return failure('stale_base','Der Ansichtskatalog wurde geändert. Vor einer Dateiaktion ausdrücklich neu laden.');}
      return result;
    }
    function exportDefinitions(kind){
      synchronize();if(!available())return showError(unavailable());
      if(kind!=='all'&&kind!=='selected')return showError(failure('invalid_export','Alle Ansichten oder genau eine gespeicherte Ansicht auswählen.'));
      if(!portability||typeof hooks.download!=='function')return showError(failure('portability_unavailable','Ansichtsdateien sind in dieser Umgebung nicht verfügbar.'));
      if(kind==='selected'&&!selected())return showError(failure('no_selection','Zuerst eine gespeicherte Ansicht auswählen.'));
      const scope=kind==='all'?{kind}:{kind,id:state.selectedId},started=begin(false,'export');if(!started.guard)return started;
      const loaded=observedCatalog(started.guard);if(!loaded.ok)return portableEnd(started,loaded);
      const prepared=portable('exportDefinitions',loaded.value.raw,scope);if(!prepared.ok)return portableEnd(started,prepared);
      let blob;try{blob=hooks.blob?hooks.blob(prepared.value.raw,prepared.value.mime):new Blob([prepared.value.raw],{type:prepared.value.mime});}catch(_){return portableEnd(started,failure('download_unavailable','Die Ansichtsdatei konnte nicht erstellt werden.'));}
      // Planning a large catalog is synchronous, but a different tab may write
      // without delivering its event yet. Recheck exact bytes before download.
      const current=observedCatalog(started.guard,loaded.value.raw);if(!current.ok)return portableEnd(started,current);
      if(!started.guard())return stale(started,'Die alte Exportaktion wurde verworfen.');
      try{hooks.download(blob,prepared.value.filename);}catch(_){return portableEnd(started,failure('download_failed','Die Ansichtsdatei konnte nicht heruntergeladen werden.'));}
      state.notice='Ansichtsdefinitionen als JSON heruntergeladen. Die aktuelle Tabelle bleibt erhalten.';
      return portableEnd(started,prepared);
    }
    async function stageImportFile(file){
      synchronize();if(!available())return showError(unavailable());
      // A newer explicit file selection also supersedes an unfinished read.
      invalidate(false);
      if(!portability)return showError(failure('portability_unavailable','Ansichtsdateien sind in dieser Umgebung nicht verfügbar.'));
      const started=begin(false,'file');if(!started.guard)return started;
      const loaded=observedCatalog(started.guard);if(!loaded.ok)return portableEnd(started,loaded,true);
      let size,read;try{size=file?.size;read=hooks.readFile?()=>hooks.readFile(file):typeof file?.arrayBuffer==='function'?()=>file.arrayBuffer():null;}catch(_){return portableEnd(started,failure('file_read_failed','Die ausgewählte Datei konnte nicht gelesen werden.'),true);}
      if(!Number.isSafeInteger(size)||size<1||size>library.limits.maxCatalogBytes)return portableEnd(started,failure('file_byte_limit','Genau eine nichtleere JSON-Datei innerhalb der Katalog-Bytegrenze auswählen.'),true);
      if(!read)return portableEnd(started,failure('file_read_failed','Die ausgewählte Datei konnte nicht gelesen werden.'),true);
      if(!started.guard())return stale(started,'Die alte Dateiauswahl wurde verworfen.');
      let buffer;try{buffer=await read();}catch(_){if(!started.guard())return stale(started,'Die alte Dateiauswahl wurde verworfen.');return portableEnd(started,failure('file_read_failed','Die ausgewählte Datei konnte nicht vollständig gelesen werden.'),true);}
      if(!started.guard())return stale(started,'Die alte Dateiauswahl wurde verworfen.');
      let length;try{length=Reflect.apply(Object.getOwnPropertyDescriptor(ArrayBuffer.prototype,'byteLength').get,buffer,[]);}catch(_){return portableEnd(started,failure('file_read_failed','Die Datei lieferte keinen vollständigen Bytepuffer.'),true);}
      if(length!==size)return portableEnd(started,failure('file_size_mismatch','Dateigröße und tatsächlich gelesene Bytes stimmen nicht überein.'),true);
      const current=observedCatalog(started.guard,loaded.value.raw);if(!current.ok)return portableEnd(started,current,true);
      const checked=portable('previewImport',loaded.value.raw,new Uint8Array(buffer));if(!checked.ok)return portableEnd(started,checked,true);
      if(!started.guard())return stale(started,'Die alte Dateivorschau wurde verworfen.');
      const choices=Object.create(null),prepared=portable('prepareImport',checked.value,choices);
      imported={token:Object.freeze({}),stamp:started.ticket.stamp,preview:checked.value,expectedRaw:loaded.value.raw,choices,prepared};
      state.notice='Datei vollständig geprüft. Vorschau und Konflikte prüfen; erst die Bestätigung speichert den Katalog.';
      return portableEnd(started,ok(null));
    }
    function currentImport(){
      synchronize();if(imported&&(!available()||imported.stamp!==stamp()||imported.expectedRaw!==state.raw))invalidate(false);
      return imported;
    }
    function resolveImport(id,action,copyName){
      const pending=currentImport();if(!pending)return showError(failure('no_import_preview','Zuerst eine aktuelle Datei vollständig prüfen.'));
      if(state.busy)return failure('busy','Die Importbestätigung läuft bereits.');
      const item=pending.preview.items.find(row=>row.id===id);if(!item)return showError(failure('unknown_resolution','Die Auswahl gehört nicht zu dieser Dateivorschau.'));
      let choice;
      if(action==='skip')choice={action};
      else if(action==='replace'&&item.replaceTargetId!==null)choice={action,targetId:item.replaceTargetId};
      else if(action==='copy'){
        if(copyName!==undefined&&typeof copyName!=='string')return showError(failure('invalid_name','Einen ausdrücklichen Namen für die Kopie eingeben.'));
        let copyId;try{copyId=pending.choices[id]?.action==='copy'?pending.choices[id].id:hooks.uuid();}catch(_){return showError(failure('uuid_unavailable','Eine frische Kennung für die Kopie konnte nicht erstellt werden.'));}
        choice={action,id:copyId,name:copyName??''};
      }else if(action!=='default'&&action!=='')return showError(failure('invalid_resolution','Die Konfliktentscheidung ist nicht verfügbar.'));
      else if(action==='default'&&item.disposition==='conflict')return showError(failure('unresolved_conflict','Für diesen Konflikt ausdrücklich Überspringen, Ersetzen oder Kopieren wählen.'));
      if(choice)pending.choices[id]=choice;else delete pending.choices[id];
      revision++;pending.stamp=stamp();pending.prepared=portable('prepareImport',pending.preview,pending.choices);state.error='';notify();
      return pending.prepared.ok?ok(null):showError(pending.prepared);
    }
    async function confirmImport(){
      const pending=currentImport();if(!pending)return showError(failure('no_import_preview','Zuerst eine aktuelle Datei vollständig prüfen.'));
      if(state.busy)return failure('busy','Eine Dateiaktion läuft bereits.');
      const planned=portable('prepareImport',pending.preview,pending.choices);pending.prepared=planned;if(!planned.ok)return showError(planned);
      const started=begin(false,'import',true);if(!started.guard)return started;
      const fresh=observedCatalog(started.guard,planned.value.expectedRaw);if(!fresh.ok)return portableEnd(started,fresh,true);
      if(!planned.value.changed){state.notice='Keine Katalogänderung erforderlich. Die vorhandenen Bytes und die aktuelle Tabelle bleiben erhalten.';return portableEnd(started,planned,true);}
      let saved;try{saved=await store.mutate({expectedRaw:planned.value.expectedRaw,entries:planned.value.entries,guard:started.guard});}catch(_){saved=failure('storage_unavailable','Der vollständige importierte Katalog konnte nicht gespeichert werden.');}
      if(!started.guard())return stale(started,'Die Ansicht oder Datei wurde inzwischen geändert; der alte Import wurde verworfen.');
      if(!saved.ok){if(saved.error?.code==='stale_base'){state.catalogValid=false;resetRaw=undefined;imported=null;}return portableEnd(started,saved);}
      const before=state.active&&state.entries.find(row=>row.id===state.active.id),after=state.active&&saved.value.entries.find(row=>row.id===state.active.id);
      if(after&&(!before||JSON.stringify(before)!==JSON.stringify(after))){
        let matches=false;
        try{const snapshot=hooks.snapshot();if(snapshot.ok){const working=library.makeEntry({id:after.id,name:after.name},hooks.contract(),snapshot.value);matches=working.ok&&JSON.stringify(working.value)===JSON.stringify(after);}}catch(_){}
        state.active={...state.active,name:after.name,modified:!matches};
      }
      state.raw=saved.value.raw;resetRaw=saved.value.raw;state.entries=saved.value.entries;state.catalogValid=true;state.loaded=true;
      state.notice='Import im Browserkatalog gespeichert: '+planned.value.addedIds.length+' hinzugefügt, '+planned.value.replacedIds.length+' ersetzt. Die aktuelle Tabelle wurde nicht angewendet.';
      return portableEnd(started,planned,true);
    }
    function cancelImport(){invalidate(false);state.error='';state.notice='Dateiauswahl und Importvorschau verworfen. Der Katalog und die aktuelle Tabelle bleiben erhalten.';notify();return ok(null);}
    function status(){synchronize();currentImport();const c=hooks.contract(),entries=state.entries.filter(entry=>entry.section===c.section);const {raw,...publicState}=state;const preview=imported?{token:imported.token,stamp:imported.stamp,entries:imported.preview.incomingEntries,currentEntries:imported.preview.currentEntries,items:imported.preview.items,choices:Object.fromEntries(Object.entries(imported.choices).map(([key,value])=>[key,{...value}])),valid:imported.prepared.ok,validationError:imported.prepared.ok?'':message(imported.prepared),counts:imported.prepared.ok?{added:imported.prepared.value.addedIds.length,replaced:imported.prepared.value.replacedIds.length,skipped:imported.prepared.value.skippedIds.length,identical:imported.prepared.value.identicalIds.length}:null}:null;return{...publicState,entries:[...entries],importPreview:preview,resetAvailable:resetRaw!==undefined,active:state.active?{...state.active}:null,deleteConfirmation:state.deleteConfirmation?{...state.deleteConfirmation}:null,resetConfirmation:state.resetConfirmation?{...state.resetConfirmation}:null};}
    return Object.freeze({status,refresh,select,name,create:()=>mutate('create'),replace:()=>mutate('replace'),rename:()=>mutate('rename'),apply,requestDelete,confirmDelete,cancelDelete,requestReset,confirmReset,cancelReset:cancelDelete,exportDefinitions,stageImportFile,resolveImport,confirmImport,cancelImport,externalChange,invalidate,resetOwner:()=>{context=null;invalidate(false);resetRaw=undefined;},synchronize});
  }
  function create(hooks){
    const env=hooks.environment||globalThis,document=hooks.document||env.document;
    const prefix=hooks.idPrefix||'table-view',$=id=>document.getElementById(prefix+id.slice('table-view'.length));
    const library=hooks.library||env.TableViewLibrary.create(env.TableExplorer);
    let storage=null,locks=null;try{storage=env.localStorage;}catch(_){}try{locks=env.navigator?.locks;}catch(_){}
    const store=hooks.store||env.TableViewStorage.create({library,storage,locks});
    const portability=hooks.portability||env.TableViewPortability?.create(hooks.catalogVersion??1);
    let controller,rendering=false;
    function renderImport(s){
      const preview=$('table-view-import-preview');if(!preview)return;
      const reading=s.busy&&s.activity==='file',value=s.importPreview,active=!hooks.available||hooks.available()===true;
      preview.hidden=!reading&&!value;
      $('table-view-import-confirm').disabled=!active||s.busy||!value?.valid;
      $('table-view-import-cancel').disabled=!active||!reading&&!value;
      const summary=$('table-view-import-summary'),list=$('table-view-import-items');
      summary.textContent=reading?'Datei wird vollständig gelesen und geprüft. Abbrechen verwirft diese Dateiauswahl.':value?value.entries.length+' vollständige Ansichten aus der Datei. '+(value.valid?value.counts.added+' hinzufügen, '+value.counts.replaced+' ersetzen, '+value.counts.skipped+' überspringen, '+value.counts.identical+' unverändert.':value.validationError)+' Erst „Import bestätigen“ speichert diesen Katalog. Die aktuelle Tabelle bleibt erhalten.':'';
      list.replaceChildren();if(!value)return;
      const token=value.token,revision=value.stamp;
      const current=node=>node.isConnected&&active&&(!hooks.available||hooks.available()===true)&&controller.status().importPreview?.token===token&&controller.status().importPreview?.stamp===revision;
      function focus(id,start,end){const target=document.getElementById(id);target?.focus?.({preventScroll:true});if(Number.isInteger(start))try{target?.setSelectionRange?.(start,end);}catch(_){} }
      for(const item of value.items){
        const entry=value.entries.find(row=>row.id===item.id),choice=value.choices[item.id],row=document.createElement('li');row.dataset.tableImportId=item.id;
        const title=document.createElement('p');title.textContent=entry.name+' · Bereich: '+entry.section+' · Kennung: '+entry.id;row.append(title);
        const detail=document.createElement('p');detail.textContent=item.disposition==='add'?'Neue Ansicht; ohne zusätzliche Auswahl wird sie hinzugefügt.':item.disposition==='identical'?'Vollständig identische Ansicht; ohne zusätzliche Auswahl bleiben die bestehenden Bytes erhalten.':item.reason==='ambiguous_targets'?'Kennung und Name verweisen auf verschiedene gespeicherte Ansichten. Nur ausdrücklich überspringen oder als frische Kopie übernehmen.':item.reason==='foreign_section_id'?'Diese Kennung gehört bereits zu einem anderen Bereich. Nur ausdrücklich überspringen oder als frische Kopie übernehmen.':'Konflikt mit einer gespeicherten Kennung oder einem normalisierten Namen. Entscheidung ausdrücklich wählen.';row.append(detail);
        for(const targetId of new Set([item.idTargetId,item.nameTargetId].filter(Boolean))){const target=value.currentEntries.find(row=>row.id===targetId),targetText=document.createElement('p');targetText.textContent='Betroffene gespeicherte Ansicht: '+target.name+' · Bereich: '+target.section+' · Kennung: '+target.id;row.append(targetText);}
        const label=document.createElement('label');label.textContent='Importentscheidung';const select=document.createElement('select');select.id=prefix+'-import-choice-'+item.id;select.dataset.tableImportChoice=item.id;
        const choices=item.disposition==='conflict'?[['','Entscheidung wählen …']]:[['default',item.disposition==='add'?'Hinzufügen':'Unverändert lassen']];choices.push(['skip','Überspringen']);if(item.disposition==='conflict'&&item.replaceTargetId!==null)choices.push(['replace','Gespeicherte Ansicht ersetzen (Name und Kennung bleiben)']);choices.push(['copy','Als frische Kopie speichern']);
        for(const [key,text]of choices){const option=document.createElement('option');option.value=key;option.textContent=text;select.append(option);}select.value=choice?.action||(item.disposition==='conflict'?'':'default');select.disabled=s.busy||!active;label.append(select);row.append(label);
        select.addEventListener('change',()=>{if(!current(select))return;const action=select.value;controller.resolveImport(item.id,action,action==='copy'?choice?.name??'':undefined);focus(select.id);});
        if(choice?.action==='copy'){
          const copyLabel=document.createElement('label');copyLabel.textContent='Name der frischen Kopie';const input=document.createElement('input');input.type='text';input.id=prefix+'-import-name-'+item.id;input.dataset.tableImportName=item.id;input.value=choice.name;input.disabled=s.busy||!active;input.autocomplete='off';copyLabel.append(input);row.append(copyLabel);const copyIdentity=document.createElement('p');copyIdentity.textContent='Frische Kennung der Kopie: '+choice.id;row.append(copyIdentity);
          input.addEventListener('input',()=>{if(!current(input))return;const start=input.selectionStart,end=input.selectionEnd;controller.resolveImport(item.id,'copy',input.value);focus(input.id,start,end);});
        }
        const complete=document.createElement('details'),legend=document.createElement('summary'),json=document.createElement('pre');legend.textContent='Vollständige Definition aus der Datei';json.tabIndex=0;json.setAttribute('aria-label','Vollständige Ansichtsdefinition: '+entry.name+' · Bereich: '+entry.section);json.textContent=JSON.stringify(entry,null,2);complete.append(legend,json);row.append(complete);list.append(row);
      }
    }
    function render(){
      if(rendering||!controller)return;rendering=true;
      try{
        const s=controller.status(),select=$('table-view-select'),previous=select.value;
        select.replaceChildren();const empty=document.createElement('option');empty.value='';empty.textContent='Gespeicherte Ansicht auswählen';select.append(empty);
        for(const entry of s.entries){const option=document.createElement('option');option.value=entry.id;const compatible=library.prepareApply(entry,hooks.contract());option.textContent=entry.name+(compatible.ok?'':' (nicht kompatibel)');select.append(option);}select.value=s.selectedId||'';
        if(select.value!==s.selectedId&&previous===s.selectedId)select.value=previous;
        $('table-view-name').value=s.name;$('table-view-status').textContent=s.busy?s.activity==='file'?'Datei wird gelesen …':s.activity==='export'?'Ansichtsdatei wird geprüft …':s.activity==='import'?'Vollständiger Import wird gespeichert …':'Katalogaktion läuft …':s.notice;
        if($('table-view-status').dataset)$('table-view-status').dataset.portabilityPhase=s.busy?s.activity:'';
        $('table-view-error').hidden=!s.error;$('table-view-error').textContent=s.error;
        $('table-view-active').textContent=s.active?'Angewendet: '+s.active.name+(s.active.modified?' · geändert, nicht gespeichert':''):'';
        const owner=hooks.owner(),entry=s.entries.find(row=>row.id===s.selectedId),compatible=entry?library.prepareApply(entry,hooks.contract()).ok:false;
        $('table-view-create').disabled=s.busy||!owner.projectId||!s.catalogValid;
        for(const id of['table-view-replace','table-view-rename','table-view-delete'])$(id).disabled=s.busy||!entry||!s.catalogValid;
        $('table-view-replace').disabled=s.busy||!owner.projectId||!entry||!s.catalogValid;
        $('table-view-apply').disabled=s.busy||!owner.projectId||!entry||!compatible||!s.catalogValid;
        $('table-view-delete-confirmation').hidden=!s.deleteConfirmation;
        $('table-view-delete-prompt').textContent=s.deleteConfirmation?'Gespeicherte Ansicht „'+s.deleteConfirmation.name+'“ aus diesem Browserkatalog löschen? Die aktuelle Tabelle bleibt erhalten.':'';
        $('table-view-reset').disabled=s.busy||!s.resetAvailable;$('table-view-reset-confirmation').hidden=!s.resetConfirmation;
        if(hooks.available&&hooks.available()!==true)for(const suffix of['select','name','refresh','create','replace','rename','delete','apply','reset','delete-confirm','delete-cancel','reset-confirm','reset-cancel'])$('table-view-'+suffix).disabled=true;
        else for(const suffix of['select','name','refresh','delete-confirm','delete-cancel','reset-confirm','reset-cancel'])$('table-view-'+suffix).disabled=false;
        const active=!hooks.available||hooks.available()===true;
        for(const suffix of['export-selected','export-all','import-file']){const control=$('table-view-'+suffix);if(control)control.disabled=!active||!portability||!s.catalogValid||(suffix==='import-file'?s.busy&&s.activity!=='file':s.busy||suffix==='export-selected'&&!entry);}
        renderImport(s);
      }finally{rendering=false;}
    }
    controller=createController({...hooks,library,store,portability,blob:hooks.blob||((raw,mime)=>new (env.Blob||globalThis.Blob)([raw],{type:mime})),uuid:hooks.uuid||(()=>env.crypto.randomUUID()),notify:render});
    function bind(id,event,action){const element=$(id);if(!element)return;element.addEventListener(event,e=>{if(!element.isConnected||$(id)!==element||hooks.available&&hooks.available()!==true)return;if(event==='click')e.preventDefault();action(e);});}
    bind('table-view-select','change',e=>controller.select(e.target.value));bind('table-view-name','input',e=>{const value=e.target.value;controller.name(value);});
    for(const[id,method]of[['refresh','refresh'],['create','create'],['apply','apply'],['replace','replace'],['rename','rename'],['delete','requestDelete'],['delete-confirm','confirmDelete'],['delete-cancel','cancelDelete'],['reset','requestReset'],['reset-confirm','confirmReset'],['reset-cancel','cancelReset']])bind('table-view-'+id,'click',()=>{Promise.resolve(controller[method]()).catch(()=>{ /* Controller reports owned failures as Results. */ });});
    bind('table-view-export-selected','click',()=>controller.exportDefinitions('selected'));bind('table-view-export-all','click',()=>controller.exportDefinitions('all'));
    bind('table-view-import-file','change',event=>{const input=event.target,file=input.files?.length===1?input.files[0]:null;input.value='';Promise.resolve(controller.stageImportFile(file)).catch(()=>{});});
    bind('table-view-import-confirm','click',()=>{Promise.resolve(controller.confirmImport()).catch(()=>{});});bind('table-view-import-cancel','click',()=>controller.cancelImport());
    const storageKey=hooks.storageKey||env.TableViewStorage?.KEY;
    env.addEventListener?.('storage',event=>{if(event.key===storageKey||event.key===null)controller.externalChange();});
    controller.refresh();render();
    return Object.freeze({render,invalidate:()=>{controller.invalidate();render();},edited:()=>{controller.invalidate();render();},resetOwner:()=>{controller.resetOwner();render();}});
  }
  return Object.freeze({createController,create});
});
