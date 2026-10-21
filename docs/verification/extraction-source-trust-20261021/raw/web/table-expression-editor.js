'use strict';
(function(host,factory){const api=factory();if(typeof module==='object'&&module.exports)module.exports=api;else host.TableExpressionEditor=api;})(typeof globalThis==='object'?globalThis:this,function(){
  const ok=value=>({ok:true,value}),bad=message=>({ok:false,error:{code:'invalid_expression_edit',message}});
  const own=(value,key)=>Object.prototype.hasOwnProperty.call(value,key);
  function data(object,key){const descriptor=Object.getOwnPropertyDescriptor(object,key);if(!descriptor||!own(descriptor,'value'))throw Error('Filterdefinitionen benötigen eigene Datenfelder.');return descriptor.value;}
  function cloneRule(rule){if(!rule||typeof rule!=='object'||Array.isArray(rule))throw Error('Eine echte Filterregel fehlt.');const result={};for(const key of Object.keys(rule)){const value=data(rule,key);if(!['string','number','boolean'].includes(typeof value)||typeof value==='number'&&!Number.isFinite(value))throw Error('Eine Filterregel enthält ungültige Daten.');Object.defineProperty(result,key,{value,enumerable:true,writable:true,configurable:true});}return result;}
  function copy(expression){let count=0;const seen=new Set();function node(value,depth){if(value===null)return null;if(!value||typeof value!=='object'||Array.isArray(value)||depth>32||++count>64||seen.has(value))throw Error('Der Filterbaum ist ungültig oder zu groß.');seen.add(value);const kind=data(value,'kind');if(kind==='rule'){if(Object.keys(value).length!==2)throw Error('Ungültiger Regelknoten.');return{kind,rule:cloneRule(data(value,'rule'))};}if(!['all','any'].includes(kind)||Object.keys(value).length!==2)throw Error('Unbekannte Filtergruppe.');const children=data(value,'children');if(!Array.isArray(children)||!children.length||children.length>32)throw Error('Eine Gruppe braucht mindestens eine echte Regel.');const cloned=[];for(let index=0;index<children.length;index++){const child=node(data(children,String(index)),depth+1);if(child===null)throw Error('Leere Zweige sind nicht erlaubt.');cloned.push(child);}return{kind,children:cloned};}return node(expression,0);}
  function indices(path){if(path==='root')return[];if(typeof path!=='string'||!/^(?:0|[1-9]\d*)(?:\.(?:0|[1-9]\d*))*$/.test(path))throw Error('Das gewählte Gruppenziel ist veraltet.');const values=path.split('.').map(Number);if(values.length>32||values.some(value=>!Number.isSafeInteger(value)||value>31))throw Error('Das gewählte Gruppenziel ist ungültig.');return values;}
  const pathFor=parts=>parts.length?parts.join('.'):'root';
  function at(root,parts){let node=root;for(const index of parts){if(!node||node.kind==='rule'||!node.children[index])throw Error('Der gewählte Filterzweig ist nicht mehr vorhanden.');node=node.children[index];}return node;}
  function replace(root,parts,value){if(!parts.length)return value;const parent=at(root,parts.slice(0,-1));if(parent.kind==='rule'||!parent.children[parts.at(-1)])throw Error('Der gewählte Filterzweig ist veraltet.');parent.children[parts.at(-1)]=value;return root;}
  function remove(root,parts){if(!parts.length)return null;const parentParts=parts.slice(0,-1),parent=at(root,parentParts);parent.children.splice(parts.at(-1),1);return parent.children.length?root:remove(root,parentParts);}
  function update(expression,action){try{
    let root=copy(expression);if(!action||typeof action!=='object')throw Error('Die Filteraktion fehlt.');const parts=indices(action.path??'root'),node=at(root,parts);let selection=pathFor(parts);
    if(action.type==='addRule'){
      const leaf={kind:'rule',rule:cloneRule(action.rule)};
      if(root===null){if(parts.length)throw Error('Das Gruppenziel ist nicht mehr vorhanden.');root=leaf;selection='root';}
      else if(node.kind==='rule'){root=replace(root,parts,{kind:'all',children:[node,leaf]});selection=pathFor(parts);}
      else node.children.push(leaf);
    }else if(action.type==='wrap'){
      if(!node)throw Error('Zuerst eine echte Formularregel hinzufügen; eine leere Gruppe wird nicht angelegt.');if(!['all','any'].includes(action.kind))throw Error('Unbekannte Gruppenart.');root=replace(root,parts,{kind:action.kind,children:[node]});
    }else if(action.type==='kind'){
      if(!node||node.kind==='rule'||!['all','any'].includes(action.kind))throw Error('Das Ziel ist keine gültige Gruppe.');node.kind=action.kind;
    }else if(action.type==='remove'){
      if(!node)throw Error('Der Filterzweig fehlt.');root=remove(root,parts);selection='root';
    }else if(action.type==='move'){
      if(!parts.length||![-1,1].includes(action.delta))throw Error('Dieser Zweig kann nicht verschoben werden.');const parent=at(root,parts.slice(0,-1)),index=parts.at(-1),next=index+action.delta;if(!parent||parent.kind==='rule'||next<0||next>=parent.children.length)throw Error('Die Zielposition ist nicht vorhanden.');[parent.children[index],parent.children[next]]=[parent.children[next],parent.children[index]];selection=pathFor([...parts.slice(0,-1),next]);
    }else throw Error('Unbekannte Filteraktion.');
    return ok({expression:root,selection});
  }catch(error){return bad(error.message);}}
  function flatRules(expression){try{const root=copy(expression),rules=[];function collect(node){if(node===null)return;if(node.kind==='rule'){rules.push({...node.rule});return;}if(node.kind==='any'&&node.children.length>1)throw Error('Diese ODER-Gruppe lässt sich nicht verlustfrei in einfache UND-Regeln umwandeln. Zuerst die Gruppenfilter aufheben.');node.children.forEach(collect);}collect(root);return ok(rules);}catch(error){return bad(error.message);}}
  function nodes(expression){const result=[];function visit(node,parts){if(!node)return;result.push({path:pathFor(parts),node,depth:parts.length});if(node.kind!=='rule')node.children.forEach((child,index)=>visit(child,[...parts,index]));}visit(expression,[]);return result;}
  function create(hooks){
    const document=hooks.document,container=hooks.container,target=hooks.target;let selected='root',focus=null;
    const current=stamp=>hooks.current(stamp)&&container.isConnected&&target.isConnected;
    function select(path){selected=path;}
    function render(){
      const expression=hooks.expression(),items=nodes(expression),stamp=hooks.stamp();if(!items.some(item=>item.path===selected))selected='root';
      const active=document.activeElement;if(active?.dataset?.expressionFocus)focus=active.dataset.expressionFocus;
      target.replaceChildren();for(const item of items){const option=document.createElement('option');option.value=item.path;const name=item.node.kind==='rule'?'Regel · neue Regel wird mit UND verbunden':item.node.kind==='all'?'UND · alle Bedingungen':'ODER · mindestens eine Bedingung';option.textContent=(item.depth?'— '.repeat(item.depth):'')+(item.path==='root'?'Gesamter Filter':item.path)+' · '+name;target.append(option);}if(!items.length){const option=document.createElement('option');option.value='root';option.textContent='Erste echte Regel hinzufügen';target.append(option);}target.value=selected;container.replaceChildren();
      const toolbar=document.createElement('div');toolbar.className='table-expression-toolbar';
      const action=(button,edit,key)=>{button.type='button';button.dataset.expressionFocus=key;button.addEventListener('click',()=>{if(!current(stamp)||!button.isConnected)return;focus=key;hooks.action(edit);});return button;};
      for(const[kind,label]of[['all','Auswahl in UND-Gruppe fassen'],['any','Auswahl in ODER-Gruppe fassen']]){const button=document.createElement('button');button.id='table-expression-wrap-'+kind;button.className='button small';button.textContent=label;action(button,{type:'wrap',path:selected,kind},'toolbar-'+kind);toolbar.append(button);}container.append(toolbar);
      const list=document.createElement('ol');list.className='table-expression-tree';
      const ruleLabel=rule=>{const column=hooks.columns().find(column=>column.key===rule.key),value=own(rule,'text')?' „'+rule.text+'“':own(rule,'min')?' '+rule.min+(own(rule,'max')?' bis '+rule.max:''):own(rule,'max')?' '+rule.max:'';return(column?.label||rule.key)+' · '+({contains:'enthält',equals:'ist gleich',min:'mindestens',max:'höchstens',range:'im Zahlenbereich',known:'Wert vorhanden',unknown:'Kein Messwert',empty:'Leerer Text'}[rule.op]||rule.op)+value;};
      function tree(node,parts,parentList){
        const path=pathFor(parts),item=document.createElement('li');item.className='table-expression-node';item.dataset.expressionPath=path;item.dataset.expressionKind=node.kind;
        const label=document.createElement('strong');label.className='table-expression-label';label.textContent=node.kind==='rule'?ruleLabel(node.rule):node.kind==='all'?'UND – alle folgenden Bedingungen':'ODER – mindestens eine folgende Bedingung';item.append(label);
        const actions=document.createElement('div');actions.className='table-expression-actions';
        const choose=document.createElement('button');choose.type='button';choose.className='button small';choose.dataset.expressionSelect=path;choose.dataset.expressionFocus='select-'+path;choose.textContent=selected===path?'Gewähltes Ziel':'Als Ziel wählen';choose.setAttribute('aria-pressed',String(selected===path));choose.addEventListener('click',()=>{if(!current(stamp)||!choose.isConnected)return;selected=path;focus='select-'+path;hooks.selectionChanged();});actions.append(choose);
        if(node.kind!=='rule'){const kind=document.createElement('select');kind.dataset.expressionGroupKind=path;kind.dataset.expressionFocus='kind-'+path;kind.setAttribute('aria-label','Gruppenart '+path);for(const[value,text]of[['all','UND – alle'],['any','ODER – mindestens eine']]){const option=document.createElement('option');option.value=value;option.textContent=text;kind.append(option);}kind.value=node.kind;kind.addEventListener('change',()=>{if(!current(stamp)||!kind.isConnected)return;focus='kind-'+path;hooks.action({type:'kind',path,kind:kind.value});});actions.append(kind);}
        for(const[kind,text]of[['all','In UND fassen'],['any','In ODER fassen']]){const button=document.createElement('button');button.className='button small';button.textContent=text;button.dataset.expressionWrap=kind;button.dataset.expressionPath=path;action(button,{type:'wrap',path,kind},'wrap-'+kind+'-'+path);actions.append(button);}
        for(const[direction,delta,text]of[['up',-1,'Nach oben'],['down',1,'Nach unten']]){const button=document.createElement('button');button.className='button small';button.textContent=text;button.dataset.expressionMove=direction;button.dataset.expressionPath=path;const parent=parts.length?at(expression,parts.slice(0,-1)):null;button.disabled=!parts.length||parts.at(-1)+delta<0||parts.at(-1)+delta>=parent.children.length;action(button,{type:'move',path,delta},'move-'+direction+'-'+path);actions.append(button);}
        const removeButton=document.createElement('button');removeButton.className='button small';removeButton.textContent=node.kind==='rule'?'Regel entfernen':'Gruppe samt Zweigen entfernen';removeButton.dataset.expressionRemove=path;action(removeButton,{type:'remove',path},'remove-'+path);actions.append(removeButton);item.append(actions);
        if(node.kind!=='rule'){const children=document.createElement('ol');children.className='table-expression-tree';node.children.forEach((child,index)=>tree(child,[...parts,index],children));item.append(children);}parentList.append(item);
      }
      if(expression)tree(expression,[],list);else{const empty=document.createElement('li');empty.textContent='Keine Gruppenfilter. Die nächste Formularregel wird die erste echte Bedingung.';list.append(empty);}container.append(list);
      if(focus){const wanted=[...container.querySelectorAll('[data-expression-focus]')].find(element=>element.dataset.expressionFocus===focus&&!element.disabled);(wanted||container.querySelector('[data-expression-select]')||target).focus();focus=null;}
    }
    target.addEventListener('change',()=>{if(!target.isConnected)return;selected=target.value;hooks.selectionChanged();});
    return Object.freeze({render,select,target:()=>selected,reset:()=>{selected='root';focus=null;}});
  }
  return Object.freeze({copy,update,flatRules,nodes,create});
});
