'use strict';
(function(host,factory){const api=factory();if(typeof module==='object'&&module.exports)module.exports=api;else host.TableViewsUI=api;})(typeof globalThis==='object'?globalThis:this,function(){
  const ok=value=>({ok:true,value}),failure=(code,message)=>({ok:false,error:{code,message}});
  const message=result=>result?.error?.message||'Die benannte Ansicht konnte nicht verarbeitet werden.';
  function createController(hooks){
    const {library,store}=hooks;
    const state={raw:null,entries:[],loaded:false,catalogValid:false,selectedId:'',name:'',busy:false,error:'',notice:'',active:null,deleteConfirmation:null,resetConfirmation:null};
    let context=null,revision=0,operation=null,resetRaw;
    const notify=()=>hooks.notify?.();
    const contractStamp=()=>{const c=hooks.contract();return JSON.stringify([c.section,c.query_semantics,c.columns.map(x=>[x.key,x.type??'auto',Boolean(x.mandatory)]),c.sectionFilters]);};
    const ownerStamp=()=>JSON.stringify([hooks.owner(),contractStamp()]);
    const stamp=()=>JSON.stringify([ownerStamp(),hooks.queryRevision(),revision]);
    function invalidate(queryChanged=true){revision++;operation=null;state.busy=false;state.deleteConfirmation=null;state.resetConfirmation=null;if(queryChanged&&state.active)state.active={...state.active,modified:true};}
    function synchronize(){let next;try{next=ownerStamp();}catch(_){next=null;}if(next!==context){invalidate(false);context=next;resetRaw=undefined;state.selectedId='';state.name='';state.active=null;state.error='';state.notice='';}}
    const selected=()=>state.entries.find(entry=>entry.id===state.selectedId&&entry.section===hooks.contract().section);
    function showError(result){state.error=message(result);notify();return result;}
    function refresh(){synchronize();invalidate(false);let result;try{result=store.read();}catch(_){result=failure('storage_unavailable','Der Browserkatalog ist nicht lesbar.');}if(!result.ok){resetRaw=Object.hasOwn(result,'observedRaw')?result.observedRaw:undefined;state.catalogValid=false;return showError(result);}resetRaw=result.value.raw;state.raw=result.value.raw;state.entries=result.value.entries;state.loaded=true;state.catalogValid=true;state.error='';const entry=selected();if(!entry){state.selectedId='';state.name='';}else state.name=entry.name;state.notice='Katalog gelesen. Die aktuelle Tabellenansicht bleibt unverändert.';notify();return result;}
    function select(id){synchronize();invalidate(false);if(id&& !state.entries.some(entry=>entry.id===id&&entry.section===hooks.contract().section))return showError(failure('unknown_view','Die gewählte Ansicht ist in diesem Bereich nicht verfügbar.'));state.selectedId=id;state.name=selected()?.name||'';state.error='';notify();return ok(null);}
    function name(value){invalidate(false);state.name=value;state.error='';notify();}
    function begin(reset=false){synchronize();if(!reset&&(!state.loaded||!state.catalogValid))return showError(failure('catalog_unavailable','Den gültigen Katalog zuerst ausdrücklich neu laden.'));const ticket={stamp:stamp()};operation=ticket;state.busy=true;state.error='';state.deleteConfirmation=null;state.resetConfirmation=null;const guard=()=>operation===ticket&&ticket.stamp===stamp();notify();return{ticket,guard};}
    function stale(started,message){
      const result=failure('stale_operation',message);
      // A completion may arrive after a newer explicit operation has started.
      // Release only its own ticket, including a revision-only invalidation that
      // has not reached the render/invalidate callback yet.
      if(operation===started.ticket){operation=null;state.busy=false;state.error=message;notify();}
      return result;
    }
    function requireCrawl(){return hooks.owner()?.projectId?ok(null):failure('no_crawl','Zuerst einen Crawl und eine Ergebnistabelle auswählen.');}
    async function mutate(kind){
      synchronize();if(state.busy)return failure('busy','Eine Katalogaktion läuft bereits.');
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
      synchronize();if(state.busy)return failure('busy','Eine Katalogaktion läuft bereits.');if(!state.catalogValid)return showError(failure('catalog_unavailable','Den gültigen Katalog zuerst neu laden.'));const crawl=requireCrawl();if(!crawl.ok)return showError(crawl);const entry=selected();if(!entry)return showError(failure('no_selection','Zuerst eine gespeicherte Ansicht auswählen.'));
      const prepared=library.prepareApply(entry,hooks.contract());if(!prepared.ok)return showError(prepared);
      const started=begin();if(!started.guard)return started;
      let result;try{result=hooks.apply(prepared.value,started.guard);}catch(_){result=failure('apply_failed','Die Ansicht konnte nicht atomar angewendet werden.');}
      // A successful synchronous App commit deliberately advances its revision
      // and calls invalidate(). Do not mistake that accepted commit for stale work.
      if(result?.ok===true){state.busy=false;operation=null;state.active={id:entry.id,name:entry.name,modified:false};state.error='';state.notice='Vollständige Ansicht auf den aktuellen Crawl angewendet.';notify();return result;}
      if(!started.guard())return stale(started,'Die alte Anwendung wurde verworfen.');state.busy=false;operation=null;return showError(result||failure('apply_failed','Keine atomare Anwendung bestätigt.'));
    }
    function requestDelete(){synchronize();if(!state.catalogValid)return showError(failure('catalog_unavailable','Den gültigen Katalog zuerst neu laden.'));const entry=selected();if(!entry)return showError(failure('no_selection','Zuerst eine gespeicherte Ansicht auswählen.'));invalidate(false);state.deleteConfirmation={id:entry.id,name:entry.name,stamp:stamp()};state.error='';notify();return ok(null);}
    function confirmDelete(){const confirm=state.deleteConfirmation;if(!confirm||confirm.id!==state.selectedId||confirm.stamp!==stamp())return failure('stale_confirmation','Die Löschbestätigung ist nicht mehr aktuell.');return mutate('delete');}
    function cancelDelete(){invalidate(false);notify();}
    function requestReset(){synchronize();if(resetRaw===undefined)return showError(failure('unobserved_catalog','Katalog zuerst neu laden. Nicht gelesene Daten können nicht zurückgesetzt werden.'));invalidate(false);state.resetConfirmation={stamp:stamp()};notify();return ok(null);}
    async function confirmReset(){const confirm=state.resetConfirmation;if(!confirm||confirm.stamp!==stamp()||resetRaw===undefined)return failure('stale_confirmation','Die Rücksetzbestätigung ist nicht mehr aktuell.');const expectedRaw=resetRaw,started=begin(true);let result;try{result=await store.reset({expectedRaw,guard:started.guard});}catch(_){result=failure('storage_unavailable','Der benannte Katalog konnte nicht zurückgesetzt werden.');}if(!started.guard())return stale(started,'Die alte Rücksetzung wurde verworfen.');state.busy=false;operation=null;if(!result.ok)return showError(result);state.raw=null;resetRaw=null;state.entries=[];state.loaded=true;state.catalogValid=true;state.selectedId='';state.name='';state.active=null;state.error='';state.notice='Alle benannten Ansichten dieses Browsers entfernt. Die aktuelle Tabelle und gespeicherte Spalten bleiben erhalten.';notify();return result;}
    function externalChange(){invalidate(false);state.catalogValid=false;resetRaw=undefined;state.notice='Der Katalog wurde in einem anderen Fenster geändert. Die Tabelle bleibt erhalten; Katalog ausdrücklich neu laden.';notify();}
    function status(){synchronize();const c=hooks.contract(),entries=state.entries.filter(entry=>entry.section===c.section);const {raw,...publicState}=state;return{...publicState,entries:[...entries],resetAvailable:resetRaw!==undefined,active:state.active?{...state.active}:null,deleteConfirmation:state.deleteConfirmation?{...state.deleteConfirmation}:null,resetConfirmation:state.resetConfirmation?{...state.resetConfirmation}:null};}
    return Object.freeze({status,refresh,select,name,create:()=>mutate('create'),replace:()=>mutate('replace'),rename:()=>mutate('rename'),apply,requestDelete,confirmDelete,cancelDelete,requestReset,confirmReset,cancelReset:cancelDelete,externalChange,invalidate,resetOwner:()=>{context=null;invalidate(false);resetRaw=undefined;},synchronize});
  }
  function create(hooks){
    const env=hooks.environment||globalThis,document=hooks.document||env.document,$=id=>document.getElementById(id);
    const library=hooks.library||env.TableViewLibrary.create(env.TableExplorer);
    let storage=null,locks=null;try{storage=env.localStorage;}catch(_){}try{locks=env.navigator?.locks;}catch(_){}
    const store=hooks.store||env.TableViewStorage.create({library,storage,locks});
    let controller,rendering=false;
    function render(){
      if(rendering||!controller)return;rendering=true;
      try{
        const s=controller.status(),select=$('table-view-select'),previous=select.value;
        select.replaceChildren();const empty=document.createElement('option');empty.value='';empty.textContent='Gespeicherte Ansicht auswählen';select.append(empty);
        for(const entry of s.entries){const option=document.createElement('option');option.value=entry.id;const compatible=library.prepareApply(entry,hooks.contract());option.textContent=entry.name+(compatible.ok?'':' (nicht kompatibel)');select.append(option);}select.value=s.selectedId||'';
        if(select.value!==s.selectedId&&previous===s.selectedId)select.value=previous;
        $('table-view-name').value=s.name;$('table-view-status').textContent=s.busy?'Katalogaktion läuft …':s.notice;
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
      }finally{rendering=false;}
    }
    controller=createController({...hooks,library,store,uuid:hooks.uuid||(()=>env.crypto.randomUUID()),notify:render});
    function bind(id,event,action){const element=$(id);element.addEventListener(event,e=>{if(!element.isConnected||$(id)!==element)return;if(event==='click')e.preventDefault();action(e);});}
    bind('table-view-select','change',e=>controller.select(e.target.value));bind('table-view-name','input',e=>{const value=e.target.value;controller.name(value);});
    for(const[id,method]of[['refresh','refresh'],['create','create'],['apply','apply'],['replace','replace'],['rename','rename'],['delete','requestDelete'],['delete-confirm','confirmDelete'],['delete-cancel','cancelDelete'],['reset','requestReset'],['reset-confirm','confirmReset'],['reset-cancel','cancelReset']])bind('table-view-'+id,'click',()=>{Promise.resolve(controller[method]()).catch(()=>{ /* Controller reports owned failures as Results. */ });});
    env.addEventListener?.('storage',event=>{if(event.key===env.TableViewStorage?.KEY||event.key===null)controller.externalChange();});
    controller.refresh();render();
    return Object.freeze({render,invalidate:()=>{controller.invalidate();render();},edited:()=>{controller.invalidate();render();},resetOwner:()=>{controller.resetOwner();render();}});
  }
  return Object.freeze({createController,create});
});
