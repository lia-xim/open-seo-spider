'use strict';
// Pure tree edits and minimal-control UI boundary. Real browser/native proof is separate.
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const editor=require('../table-expression-editor.js'),engine=require('../table-explorer.js');
const clone=value=>JSON.parse(JSON.stringify(value)),leaf=(key,op,more={})=>({kind:'rule',rule:{key,op,...more}}),A=leaf('count','min',{min:0}),B=leaf('title','contains',{text:' Café '}),C=leaf('count','max',{max:4});
function accepted(result){assert.equal(result.ok,true,JSON.stringify(result));return result.value;}
const columns=[{key:'url',label:'URL',type:'text',mandatory:true,value:row=>row.url},{key:'title',label:'Titel',type:'text',value:row=>row.title},{key:'count',label:'Anzahl',type:'number',value:row=>row.count}];
test('pure structural append/wrap/kind/move edits preserve the original tree and real leaf data',()=>{
  const source=clone(A),group=accepted(editor.update(source,{type:'wrap',path:'root',kind:'any'}));assert.deepEqual(group.expression,{kind:'any',children:[A]});assert.deepEqual(source,A);
  const added=accepted(editor.update(group.expression,{type:'addRule',path:'root',rule:B.rule}));assert.deepEqual(added.expression,{kind:'any',children:[A,B]});assert.deepEqual(group.expression,{kind:'any',children:[A]});
  const changed=accepted(editor.update(added.expression,{type:'kind',path:'root',kind:'all'}));const moved=accepted(editor.update(changed.expression,{type:'move',path:'1',delta:-1}));assert.deepEqual(moved,{expression:{kind:'all',children:[B,A]},selection:'0'});assert.deepEqual(changed.expression,{kind:'all',children:[A,B]});
});
test('pure insertion requires actual rules, wraps a leaf with AND and never invents an empty group',()=>{
  assert.equal(editor.update(null,{type:'wrap',path:'root',kind:'all'}).ok,false);assert.equal(editor.update(null,{type:'wrap',path:'root',kind:'any'}).ok,false);
  assert.deepEqual(accepted(editor.update(null,{type:'addRule',path:'root',rule:A.rule})),{expression:A,selection:'root'});
  assert.deepEqual(accepted(editor.update(A,{type:'addRule',path:'root',rule:B.rule})).expression,{kind:'all',children:[A,B]});
});
test('pure removal recursively drops empty ancestors but preserves genuine unary grouping',()=>{
  const tree={kind:'all',children:[A,{kind:'any',children:[B]}]},before=clone(tree);
  assert.deepEqual(accepted(editor.update(tree,{type:'remove',path:'1.0'})).expression,{kind:'all',children:[A]});assert.deepEqual(tree,before);
  assert.equal(accepted(editor.update({kind:'all',children:[{kind:'any',children:[B]}]},{type:'remove',path:'0.0'})).expression,null);assert.equal(accepted(editor.update(tree,{type:'remove',path:'root'})).expression,null);
});
test('pure invalid paths, impossible moves and leaf group-kind edits refuse without changing data',()=>{
  const tree={kind:'all',children:[A,B]},before=clone(tree);
  for(const action of [{type:'remove',path:'__proto__'},{type:'remove',path:'00'},{type:'remove',path:'9'},{type:'move',path:'root',delta:1},{type:'move',path:'0',delta:-1},{type:'move',path:'1',delta:1},{type:'kind',path:'0',kind:'any'},{type:'wrap',path:'0',kind:'invented'}])assert.equal(editor.update(tree,action).ok,false,JSON.stringify(action));
  assert.deepEqual(tree,before);
});
test('pure flat conversion preserves DFS AND order and refuses real OR rather than weakening it',()=>{
  const nested={kind:'all',children:[A,{kind:'all',children:[B,{kind:'any',children:[C]}]}]};assert.deepEqual(accepted(editor.flatRules(nested)),[A.rule,B.rule,C.rule]);assert.deepEqual(accepted(editor.flatRules(null)),[]);assert.deepEqual(accepted(editor.flatRules(A)),[A.rule]);
  const disjunction={kind:'any',children:[A,B]},before=clone(disjunction);assert.equal(editor.flatRules(disjunction).ok,false);assert.deepEqual(disjunction,before);
});
test('pure clone rejects accessors, cycles, sparse children and shared branches without running getters',()=>{
  let reads=0;const getter={};Object.defineProperty(getter,'kind',{enumerable:true,get(){reads++;return'rule';}});assert.equal(editor.update(getter,{type:'remove',path:'root'}).ok,false);assert.equal(reads,0);
  const cycle={kind:'all',children:[]};cycle.children.push(cycle);for(const expression of [cycle,{kind:'all',children:new Array(1)},{kind:'all',children:[A,A]},{kind:'all',children:[null]}])assert.equal(editor.update(expression,{type:'remove',path:'root'}).ok,false);
});
test('pure node paths describe the actual hierarchy, not flattened OR groups',()=>{
  const tree={kind:'any',children:[{kind:'all',children:[A,B]},{kind:'all',children:[C]}]};assert.deepEqual(editor.nodes(tree).map(({path,depth,node})=>[path,depth,node.kind]),[['root',0,'any'],['0',1,'all'],['0.0',2,'rule'],['0.1',2,'rule'],['1',1,'all'],['1.0',2,'rule']]);
});

function fixture(){
  const document={activeElement:null,roots:[]};
  class Element{
    constructor(tag='input',attached=false){this.tag=tag;this.attached=attached;this.children=[];this.parent=null;this.dataset={};this.attributes={};this.value='';this.hidden=false;this.disabled=false;this.events=new Map();this.id='';this.textContent='';}
    get isConnected(){return this.attached||Boolean(this.parent?.isConnected);}
    get options(){return this.children.filter(child=>child.tag==='option');}
    replaceChildren(){for(const child of this.children)child.parent=null;this.children=[];if(this.tag==='select')this.value='';}
    append(...children){for(let child of children){if(typeof child==='string'){const text=new Element('#text');text.textContent=child;child=text;}child.parent=this;this.children.push(child);}if(this.tag==='select'&&!this.value)this.value=this.options[0]?.value||'';}
    addEventListener(type,handler){if(!this.events.has(type))this.events.set(type,[]);this.events.get(type).push(handler);}
    emit(type){for(const handler of this.events.get(type)||[])handler({target:this,preventDefault(){}});}
    setAttribute(key,value){this.attributes[key]=value;}
    focus(){document.activeElement=this;}
    querySelectorAll(selector){const all=[];function visit(node){for(const child of node.children){all.push(child);visit(child);}}visit(this);return all.filter(node=>selector.split(',').some(part=>{part=part.trim();if(part.startsWith('[data-')){const name=part.slice(6,-1).replace(/-([a-z])/g,(_,letter)=>letter.toUpperCase());return Object.hasOwn(node.dataset,name);}return node.tag===part;}));}
    querySelector(selector){return this.querySelectorAll(selector)[0]||null;}
  }
  const controls=new Map(),selects=new Set(['table-rule-column','table-rule-op','table-sort-column','table-sort-direction','table-rule-mode','table-expression-target']);
  document.getElementById=id=>{for(const root of document.roots)for(const node of[root,...root.querySelectorAll('button,input,select,ol,li,div,strong,option')])if(node.id===id)return node;if(!controls.has(id)){const node=new Element(selects.has(id)?'select':'input',true);node.id=id;controls.set(id,node);document.roots.push(node);}return controls.get(id);};
  document.createElement=tag=>new Element(tag);document.createTextNode=text=>{const node=new Element('#text');node.textContent=text;return node;};
  const storageCalls=[],window={document,TextEncoder,localStorage:{getItem:()=>null,setItem:(...args)=>storageCalls.push(args),removeItem:(...args)=>storageCalls.push(args)},addEventListener(){}};
  window.window=window;const context=vm.createContext(window);
  for(const file of['table-explorer.js','table-expression-editor.js','table-explorer-ui.js'])vm.runInContext(fs.readFileSync(path.join(__dirname,'../'+file),'utf8'),context,{filename:file});
  const materialize=vm.runInContext('(text)=>JSON.parse(text)',context);
  const model={owner:{projectId:'crawl-a',section:'titles',generation:1,segment:null},sort:materialize('{"key":"url","direction":1}'),changes:0,edits:[],ui:null};
  const identity=vm.runInContext('({identity:row=>row.url})',context);
  const ui=window.TableExplorerUI.create({owner:()=>({...model.owner}),sort:()=>model.sort,setSort:sort=>{model.sort=sort;},identity,queryEdited:()=>model.edits.push({version:ui.queryVersion(),draft:clone(ui.draft())}),changed:()=>{model.changes++;ui.render();}});model.ui=ui;ui.synchronize(columns);ui.render();
  const get=id=>document.getElementById(id),mode=value=>{get('table-rule-mode').value=value;get('table-rule-mode').emit('change');};
  function rule(key,op,values={}){get('table-rule-column').value=key;get('table-rule-column').emit('change');get('table-rule-op').value=op;get('table-rule-op').emit('change');get('table-rule-text').value=values.text??'';get('table-rule-min').value=String(values.min??'');get('table-rule-max').value=String(values.max??'');get('table-rule-form').emit('submit');}
  const target=value=>{get('table-expression-target').value=value;get('table-expression-target').emit('change');},button=(name,value,path)=>get('table-expression-editor').querySelectorAll('button,select').find(node=>node.dataset[name]===value&&(path===undefined||node.dataset.expressionPath===path));
  return{ui,model,document,get,mode,rule,target,button,storageCalls,install:value=>ui.installPrepared(materialize(JSON.stringify(value))),rows:(rows,schema,search,sort)=>ui.rows(rows,schema,search,materialize(JSON.stringify(sort)))};
}
test('simple mode retains the exact legacy draft shape and explicit conversion clones its AND rules',()=>{
  const f=fixture();assert.equal(f.ui.queryVersion(),1);assert.deepEqual(clone(accepted(f.ui.draft())),{rules:[],visibleKeys:['url','title','count']});f.rule('count','min',{min:0});f.rule('title','contains',{text:' Cafe\u0301 '});const previous=clone(accepted(f.ui.draft()));
  f.mode('grouped');assert.equal(f.ui.queryVersion(),2);assert.deepEqual(clone(accepted(f.ui.draft())),{expression:{kind:'all',children:previous.rules.map(rule=>({kind:'rule',rule}))},visibleKeys:previous.visibleKeys,queryVersion:2});assert.equal(f.model.edits.at(-1).version,1);assert.deepEqual(f.model.edits.at(-1).draft.value,previous);assert.deepEqual(f.storageCalls,[]);
  const detached=accepted(f.ui.draft());detached.expression.children[0].rule.min=999;detached.visibleKeys.pop();assert.equal(accepted(f.ui.draft()).expression.children[0].rule.min,0);assert.equal(accepted(f.ui.draft()).visibleKeys.length,3);
});
test('actual control callbacks construct a genuinely nested four-level mixed AND/OR tree',()=>{
  const f=fixture();f.mode('grouped');f.rule('count','min',{min:0});f.get('table-expression-wrap-all').emit('click');f.rule('title','contains',{text:'Cafe\u0301'});f.target('1');f.get('table-expression-wrap-any').emit('click');f.rule('count','max',{max:4});f.target('1.1');f.get('table-expression-wrap-all').emit('click');f.rule('count','known');f.target('root');f.get('table-expression-wrap-any').emit('click');f.rule('title','empty');
  const expected={kind:'any',children:[{kind:'all',children:[A,{kind:'any',children:[leaf('title','contains',{text:'Cafe\u0301'}),{kind:'all',children:[C,leaf('count','known')]}]}]},leaf('title','empty')]};assert.deepEqual(clone(accepted(f.ui.draft()).expression),expected);assert.equal(f.get('table-expression-error').hidden,true);assert.equal(f.ui.hasRules(),true);assert.equal(f.ui.canExport(),true);assert.deepEqual(f.storageCalls,[]);
});
test('group type, sibling movement and removal controls edit only accepted tree nodes with immediate invalidation',()=>{
  const f=fixture();f.mode('grouped');f.rule('count','min',{min:0});f.rule('title','empty');const before=clone(accepted(f.ui.draft()));const kind=f.button('expressionGroupKind','root');kind.value='any';kind.emit('change');assert.equal(f.model.edits.at(-1).draft.value.expression.kind,'all');assert.equal(accepted(f.ui.draft()).expression.kind,'any');
  f.button('expressionMove','up','1').emit('click');assert.deepEqual(clone(accepted(f.ui.draft()).expression.children),[leaf('title','empty'),A]);f.button('expressionRemove','1').emit('click');assert.deepEqual(clone(accepted(f.ui.draft()).expression),{kind:'any',children:[leaf('title','empty')]});assert.notDeepEqual(clone(accepted(f.ui.draft())),before);assert.deepEqual(f.storageCalls,[]);
});
test('invalid depth/range edits preserve accepted expression/rows and visibly disable export until corrected',()=>{
  const f=fixture();f.mode('grouped');f.rule('count','min',{min:0});for(let i=0;i<4;i++)f.get('table-expression-wrap-all').emit('click');const acceptedTree=clone(accepted(f.ui.draft()).expression),rows=[{url:'a',title:'A',count:0},{url:'b',title:'B',count:null}],selected=f.rows(rows,columns,'',{key:'url',direction:1});assert.deepEqual([...selected],[rows[0]]);
  f.get('table-expression-wrap-any').emit('click');assert.equal(f.ui.canExport(),false);assert.equal(f.ui.draft().ok,false);assert.equal(f.get('table-expression-error').hidden,false);assert.deepEqual([...f.rows(rows,columns,'',{key:'url',direction:1})],[rows[0]]);
  f.button('expressionGroupKind','root').value='any';f.button('expressionGroupKind','root').emit('change');assert.equal(f.ui.canExport(),true);assert.deepEqual(clone(accepted(f.ui.draft()).expression),{...acceptedTree,kind:'any'});
  f.rule('count','range',{min:5,max:0});assert.equal(f.ui.canExport(),false);assert.equal(f.get('table-expression-error').hidden,false);assert.deepEqual([...f.rows(rows,columns,'',{key:'url',direction:1})],[rows[0]]);
});
test('OR conversion refusal keeps the tree and mode; Clear removes AST without implicitly switching mode',()=>{
  const f=fixture();f.mode('grouped');f.rule('count','min',{min:0});f.get('table-expression-wrap-any').emit('click');f.rule('title','empty');const before=clone(accepted(f.ui.draft()).expression),rows=[{url:'a',title:'',count:null},{url:'b',title:'known',count:1}];
  f.mode('simple');assert.equal(f.ui.queryVersion(),2);assert.equal(f.get('table-rule-mode').value,'grouped');assert.equal(f.get('table-expression-error').hidden,false);assert.equal(f.ui.canExport(),false);assert.deepEqual([...f.rows(rows,columns,'',{key:'url',direction:1})],rows);
  f.button('expressionGroupKind','root').value='any';f.button('expressionGroupKind','root').emit('change');assert.deepEqual(clone(accepted(f.ui.draft()).expression),before);f.get('table-rules-clear').emit('click');assert.equal(f.ui.queryVersion(),2);assert.equal(accepted(f.ui.draft()).expression,null);assert.equal(f.ui.hasRules(),false);assert.equal(f.ui.canExport(),true);f.mode('simple');assert.equal(f.ui.queryVersion(),1);assert.deepEqual(clone(accepted(f.ui.draft())).rules,[]);
});
test('explicit all-only and unary-any conversion preserves DFS rule order and original v1 state shape',()=>{
  const f=fixture(),expression={kind:'all',children:[A,{kind:'all',children:[B,{kind:'any',children:[C]}]}]};accepted(f.install({expression,search:'',sort:{key:'url',direction:1},visibleKeys:['url']}));f.ui.render();f.mode('simple');assert.equal(f.ui.queryVersion(),1);assert.deepEqual(clone(accepted(f.ui.draft())),{rules:[A.rule,B.rule,C.rule],visibleKeys:['url']});assert.deepEqual(f.storageCalls,[]);
});
test('v2 installation validates every field before mutation, copies data and performs no hooks/storage/render',()=>{
  const f=fixture(),baseline=clone(accepted(f.ui.draft())),calls=f.model.changes,edits=f.model.edits.length,expression={kind:'any',children:[A,B]},view={expression,search:'café',sort:{key:'count',direction:-1},visibleKeys:['url']};
  for(const invalid of [{...view,rules:[]},{...view,expression:{kind:'all',children:[]}},{...view,sort:{key:'missing',direction:1}},{...view,visibleKeys:['invented']}]){assert.equal(f.install(invalid).ok,false);assert.equal(f.ui.queryVersion(),1);assert.deepEqual(clone(accepted(f.ui.draft())),baseline);}
  accepted(f.install(view));assert.equal(f.model.changes,calls);assert.equal(f.model.edits.length,edits);assert.equal(f.ui.queryVersion(),2);expression.children[0].rule.min=999;assert.equal(accepted(f.ui.draft()).expression.children[0].rule.min,0);assert.deepEqual(f.storageCalls,[]);
  accepted(f.install({rules:[C.rule],search:'',sort:{key:'url',direction:1},visibleKeys:['url','count']}));assert.equal(f.ui.queryVersion(),1);assert.deepEqual(clone(accepted(f.ui.draft())),{rules:[C.rule],visibleKeys:['url','count']});assert.equal(f.model.changes,calls);assert.equal(f.model.edits.length,edits);
});
test('accepted OR query preserves every matching original occurrence beyond pagination and hidden search columns',()=>{
  const f=fixture(),rows=Array.from({length:400},(_,index)=>({url:'https://owned.test/'+index,title:index%2?'Other':'Cafe\u0301',count:index})),expression={kind:'all',children:[{kind:'any',children:[leaf('count','range',{min:0,max:149}),leaf('count','range',{min:200,max:349})]},leaf('title','contains',{text:'café'})]};
  accepted(f.install({expression,search:'CAFÉ',sort:{key:'count',direction:-1},visibleKeys:['url']}));const selected=f.rows(rows,columns,'CAFÉ',{key:'count',direction:-1}),expected=[];for(let index=398;index>=0;index-=2)if(index<=149||index>=200&&index<=349)expected.push(rows[index]);assert.equal(expected.length,150);assert.deepEqual([...selected],expected);assert.equal(selected[0],rows[348]);assert.deepEqual([...f.ui.columns(columns)].map(column=>column.key),['url']);
});
test('detached and old-revision group controls cannot change current rules, owner or export state',()=>{
  const f=fixture();f.mode('grouped');f.rule('count','min',{min:0});const old=f.button('expressionRemove','root');f.get('table-expression-wrap-any').emit('click');const before=clone(accepted(f.ui.draft())),edits=f.model.edits.length;assert.equal(old.isConnected,false);old.emit('click');assert.deepEqual(clone(accepted(f.ui.draft())),before);assert.equal(f.model.edits.length,edits);
  const previous=f.button('expressionRemove','root');f.model.owner.generation++;previous.emit('click');assert.deepEqual(clone(accepted(f.ui.draft())),before);assert.equal(f.model.edits.length,edits);assert.equal(f.ui.canExport(),true);
  f.model.owner.projectId='crawl-b';f.ui.resetOwner();previous.emit('click');f.ui.synchronize(columns);f.ui.render();assert.equal(f.ui.queryVersion(),1);assert.deepEqual(clone(accepted(f.ui.draft())).rules,[]);assert.equal(f.model.edits.length,edits);assert.deepEqual(f.storageCalls,[]);
});
test('node selection does not change the query, and action rerender restores connected focus',()=>{
  const f=fixture();f.mode('grouped');f.rule('count','min',{min:0});f.rule('title','empty');const before=clone(accepted(f.ui.draft())),edits=f.model.edits.length;const choose=f.button('expressionSelect','1');choose.focus();choose.emit('click');assert.deepEqual(clone(accepted(f.ui.draft())),before);assert.equal(f.model.edits.length,edits);assert.equal(f.get('table-expression-target').value,'1');assert.equal(f.document.activeElement.isConnected,true);
  const wrap=f.button('expressionWrap','any','1');wrap.focus();wrap.emit('click');assert.equal(f.document.activeElement.isConnected,true);assert.equal(f.document.activeElement.dataset.expressionFocus,'wrap-any-1');assert.equal(accepted(f.ui.draft()).expression.children[1].kind,'any');
});
