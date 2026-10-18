'use strict';
// UI for the main in-memory result table. Specialist inventories keep their own
// pagination and exports. Only column keys are saved, in this browser origin.
(() => {
  const labels = {contains:'enthält Text',equals:'Text ist gleich',min:'mindestens',max:'höchstens',range:'im Zahlenbereich',known:'Wert vorhanden (auch 0)',unknown:'Kein Messwert',empty:'Leerer Text'};
  const decimal = value => {
    const text = String(value).trim();
    if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(text) || !Number.isFinite(Number(text))) throw Error('Eine endliche Zahl eingeben; Dezimaltrennzeichen ist ein Punkt.');
    return Number(text);
  };
  function create(hooks) {
    const tools=window.TableExplorer,$=id=>document.getElementById(id);
    if (!tools) throw Error('Die Tabellenfunktionen sind nicht geladen.');
    const state={scope:null,schema:[],visibleKeys:[],rules:[],error:'',storageError:'',evaluationError:'',notice:''};
    let revision=0;
    const context=()=>{const owner=hooks.owner();return {projectId:owner.projectId,section:owner.section};};
    const stamp=()=>JSON.stringify([hooks.owner(),revision]);
    const same=owner=>owner===stamp();
    const storage=()=>{try{return window.localStorage;}catch(_){return null;}};
    const canonical=keys=>[...state.schema.filter(column=>column.mandatory).map(column=>column.key),...keys.filter(key=>!state.schema.find(column=>column.key===key)?.mandatory)];
    function changed() { state.evaluationError='';hooks.changed(); }
    function synchronize(schema) {
      const owner=hooks.owner(),scope=JSON.stringify([owner.projectId,owner.section,schema.map(column=>[column.key,column.type,column.mandatory])]);
      state.schema=schema;
      if (scope===state.scope) return;
      state.scope=scope;state.rules=[];state.error='';state.evaluationError='';state.storageError='';
      state.visibleKeys=canonical(schema.map(column=>column.key));
      state.notice=owner.projectId?'Standardspalten · Änderungen mit „Spalten speichern“ sichern.':'Spalten lassen sich nach Auswahl eines Crawls speichern.';
      if(owner.projectId) {
        const result=tools.load(storage(),context(),schema);
        if (!result.ok) state.storageError='Gespeicherte Spaltenansicht nicht geladen: '+result.error.message+' Standardspalten werden angezeigt; gespeicherte Daten wurden nicht verändert.';
        else if (result.value) {state.visibleKeys=canonical(result.value.visibleKeys);state.notice='Gespeicherte Spaltenansicht für diesen Crawl und Bereich geladen.';}
      }
      fillColumns($('table-rule-column'),schema,null);fillColumns($('table-sort-column'),schema,hooks.sort().key);
      $('table-rule-text').value='';$('table-rule-min').value='';$('table-rule-max').value='';
      renderRuleInputs();
    }
    function fillColumns(select,schema,selected) {
      select.replaceChildren();
      for(const column of schema){const option=document.createElement('option');option.value=column.key;option.textContent=column.label;select.append(option);}
      if(schema.some(column=>column.key===selected))select.value=selected;
    }
    function compile(search,sort) {return tools.compile(state.schema,{search,rules:state.rules,sort,visibleKeys:canonical(state.visibleKeys)},hooks.identity);}
    function columns(schema) {
      synchronize(schema);const result=tools.projectColumns(schema,canonical(state.visibleKeys));
      if(!result.ok){state.evaluationError=result.error.message;return schema.filter(column=>column.mandatory);}
      return result.value;
    }
    function rows(input,schema,search,sort) {
      synchronize(schema);state.evaluationError='';
      const compiled=compile(search,sort),result=compiled.ok?compiled.value.rows(input):compiled;
      if(!result.ok){state.evaluationError='Tabelle nicht ausgewertet: '+result.error.message;return [];}
      return result.value;
    }
    function renderRuleInputs() {
      const column=state.schema.find(column=>column.key===$('table-rule-column').value),select=$('table-rule-op'),previous=select.value;
      const operators=column?.type==='number'?['min','max','range','known','unknown','empty']:['contains','equals','known','unknown','empty'];
      if([...select.options].map(option=>option.value).join('|')!==operators.join('|')){
        select.replaceChildren();for(const op of operators){const option=document.createElement('option');option.value=op;option.textContent=labels[op];select.append(option);}if(operators.includes(previous))select.value=previous;
      }
      const op=select.value;
      $('table-rule-text-label').hidden=!['contains','equals'].includes(op);
      $('table-rule-min-label').hidden=!['min','range'].includes(op);
      $('table-rule-max-label').hidden=!['max','range'].includes(op);
    }
    function readRule() {
      const rule={key:$('table-rule-column').value,op:$('table-rule-op').value};
      if(['contains','equals'].includes(rule.op))rule.text=$('table-rule-text').value;
      if(['min','range'].includes(rule.op))rule.min=decimal($('table-rule-min').value);
      if(['max','range'].includes(rule.op))rule.max=decimal($('table-rule-max').value);
      return rule;
    }
    function render() {
      revision++;const owner=stamp(),list=$('table-rules'),focused=document.activeElement;
      const columnFocus=focused?.dataset?.tableColumn!==undefined?{key:focused.dataset.tableColumn}:focused?.dataset?.tableColumnMove?{key:focused.dataset.columnKey,move:focused.dataset.tableColumnMove}:null;
      const ruleFocus=focused?.dataset?.tableRuleRemove;list.replaceChildren();
      for(const [index,rule]of state.rules.entries()){
        const item=document.createElement('li'),column=state.schema.find(column=>column.key===rule.key),button=document.createElement('button');
        item.dataset.tableRuleIndex=String(index);const text=[column?.label||rule.key,labels[rule.op]];
        if('text'in rule)text.push('„'+rule.text+'“');if('min'in rule)text.push(String(rule.min));if('max'in rule)text.push(String(rule.max));
        item.append(document.createTextNode(text.join(' ')+' '));button.type='button';button.className='button small';button.dataset.tableRuleRemove=String(index);button.textContent='Entfernen';button.setAttribute('aria-label','Filter entfernen: '+text.join(' '));
        button.addEventListener('click',()=>{if(!same(owner))return;state.rules.splice(index,1);state.error='';changed();});item.append(button);list.append(item);
      }
      $('table-rules-clear').disabled=!state.rules.length&&!state.error;
      const columnList=$('table-column-list');columnList.replaceChildren();
      const ordered=[...canonical(state.visibleKeys),...state.schema.map(column=>column.key).filter(key=>!state.visibleKeys.includes(key))];
      for(const key of ordered){
        const column=state.schema.find(entry=>entry.key===key),item=document.createElement('li'),label=document.createElement('label'),input=document.createElement('input');
        item.dataset.columnKey=key;input.type='checkbox';input.value=key;input.dataset.tableColumn=key;input.checked=state.visibleKeys.includes(key);input.disabled=Boolean(column.mandatory);label.append(input,document.createTextNode(' '+column.label+(column.mandatory?' (Identität)':'')));item.append(label);
        input.addEventListener('change',()=>{if(!same(owner))return;state.visibleKeys=input.checked?[...state.visibleKeys,key]:state.visibleKeys.filter(value=>value!==key);state.visibleKeys=canonical(state.visibleKeys);state.notice='Spalten geändert · noch nicht gespeichert.';changed();});
        const position=state.visibleKeys.indexOf(key),fixed=state.schema.filter(entry=>entry.mandatory).length;
        for(const[direction,offset,caption]of [['up',-1,'Nach oben'],['down',1,'Nach unten']]){
          const button=document.createElement('button');button.type='button';button.className='button small';button.dataset.tableColumnMove=direction;button.dataset.columnKey=key;button.textContent=offset<0?'↑':'↓';button.setAttribute('aria-label',column.label+': '+caption);
          button.disabled=Boolean(column.mandatory)||position<0||(offset<0?position<=fixed:position>=state.visibleKeys.length-1);
          button.addEventListener('click',()=>{if(!same(owner))return;const next=[...state.visibleKeys];[next[position],next[position+offset]]=[next[position+offset],next[position]];state.visibleKeys=next;state.notice='Spaltenreihenfolge geändert · noch nicht gespeichert.';changed();});item.append(button);
        }
        columnList.append(item);
      }
      const sort=hooks.sort();$('table-sort-column').value=state.schema.some(column=>column.key===sort.key)?sort.key:state.schema[0]?.key||'';$('table-sort-direction').value=String(sort.direction);
      const error=[state.error,state.storageError,state.evaluationError].filter(Boolean).join(' ');$('table-explorer-error').hidden=!error;$('table-explorer-error').textContent=error;
      if(error)$('table-options').open=true;
      $('table-column-storage-status').textContent=state.notice;$('table-columns-save').disabled=!hooks.owner().projectId||Boolean(state.evaluationError);$('table-columns-reset').disabled=!hooks.owner().projectId;
      if(columnFocus){const controls=[...columnList.querySelectorAll('input,button')];const target=controls.find(control=>!control.disabled&&(columnFocus.move?control.dataset.columnKey===columnFocus.key&&control.dataset.tableColumnMove===columnFocus.move:control.dataset.tableColumn===columnFocus.key))||controls.find(control=>!control.disabled&&control.dataset.tableColumn===columnFocus.key);target?.focus();}
      if(ruleFocus!==undefined){const buttons=[...list.querySelectorAll('[data-table-rule-remove]')];(buttons[Math.min(Number(ruleFocus),buttons.length-1)]||$('table-rule-add')).focus();}
    }
    $('table-rule-form').addEventListener('submit',event=>{
      event.preventDefault();
      try{
        const candidate=[...state.rules,readRule()],result=tools.compile(state.schema,{search:'',rules:candidate,sort:hooks.sort(),visibleKeys:canonical(state.visibleKeys)},hooks.identity);
        if(!result.ok)throw Error(result.error.message);state.rules=candidate;state.error='';changed();
      }catch(error){state.error='Filter nicht übernommen: '+error.message+' Die bisherige Auswahl bleibt bestehen.';hooks.changed();}
    });
    $('table-rule-column').addEventListener('change',renderRuleInputs);$('table-rule-op').addEventListener('change',renderRuleInputs);
    $('table-rules-clear').addEventListener('click',()=>{state.rules=[];state.error='';changed();});
    for(const id of ['table-sort-column','table-sort-direction'])$(id).addEventListener('change',()=>{hooks.setSort({key:$('table-sort-column').value,direction:Number($('table-sort-direction').value)});changed();});
    $('table-columns-save').addEventListener('click',()=>{
      const result=tools.save(storage(),context(),state.schema,{visibleKeys:canonical(state.visibleKeys)});
      if(result.ok){state.visibleKeys=canonical(result.value.visibleKeys);state.storageError='';state.notice='Spaltenansicht für diesen Crawl und Bereich gespeichert.';}else state.storageError='Spalten nicht gespeichert: '+result.error.message;
      hooks.changed();
    });
    $('table-columns-reset').addEventListener('click',()=>{
      const result=tools.reset(storage(),context());
      if(result.ok){state.visibleKeys=canonical(state.schema.map(column=>column.key));state.storageError='';state.notice='Gespeicherte Spaltenansicht dieses Crawls und Bereichs entfernt; Standardspalten aktiv.';}else state.storageError='Spaltenansicht nicht zurückgesetzt: '+result.error.message;
      hooks.changed();
    });
    window.addEventListener('storage',event=>{
      if(!hooks.owner().projectId)return;let key;try{key=tools.keyFor(context());}catch(_){return;}
      if(event.key!==key&&event.key!==null)return;
      state.storageError='Die gespeicherte Spaltenansicht wurde in einem anderen Fenster geändert. Die aktuelle Ansicht bleibt erhalten; zum Laden den Crawl/Bereich erneut öffnen oder die aktuelle Ansicht ausdrücklich speichern.';hooks.changed();
    });
    return Object.freeze({columns,rows,render,synchronize,hasRules:()=>Boolean(state.rules.length),canExport:()=>!state.error&&!state.evaluationError,resetOwner:()=>{state.scope=null;}});
  }
  window.TableExplorerUI=Object.freeze({create});
})();
