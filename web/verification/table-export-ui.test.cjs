'use strict';
// Real pure XLSX writer and Blob; App, native controls and OS downloads are a separate gate.
const test=require('node:test'),assert=require('node:assert/strict');
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const UI=require('../table-export-ui.js'),Xlsx=require('../table-xlsx.js');
const clone=value=>JSON.parse(JSON.stringify(value));
function deferred(){let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return{promise,resolve,reject};}
function data(){return{columns:[{key:'url',label:'URL'},{key:'title',label:'Title'},{key:'count',label:'Count'},{key:'indexed',label:'Indexed'}],rows:[['https://example.test/ä','=literal',42,true],['https://example.test/unknown',null,0,false]],context:[{key:'crawl',label:'Crawl',value:'crawl-a'}]};}
function fixture(options={}){
  const model={available:true,connected:true,stamp:{owner:{projectId:'crawl-a',section:'titles'},generation:1,queryRevision:2,columns:['url','title','count','indexed'],sort:{key:'count',direction:-1},mode:'crawl'},filename:'Titel – geprüft.xlsx',data:data(),reads:0};
  const downloads=[],states=[],yields=[];
  const hooks={writer:options.writer||Xlsx.create(),available:()=>model.available,connected:()=>model.connected,stamp:()=>model.stamp,
    snapshot:()=>{model.reads++;return{ok:true,value:{data:model.data,filename:model.filename}};},
    download:(blob,filename)=>downloads.push({blob,filename}),changed:state=>states.push(state)};
  if(options.deferred)hooks.yieldControl=phase=>{if(phase!==(options.deferPhase||'prepare'))return;const wait=deferred();yields.push(wait);return wait.promise;};
  Object.assign(hooks,options.hooks);
  const controller=UI.create(hooks,options.env===undefined?globalThis:options.env);
  return{model,hooks,controller,downloads,states,yields};
}
function failure(result,code){assert.equal(result.ok,false,JSON.stringify(result));assert.equal(result.error.code,code);assert.equal(Object.hasOwn(result,'value'),false);assert.ok(Object.isFrozen(result.error));return result.error;}
function noDownload(f){assert.equal(f.downloads.length,0);assert.equal(f.controller.state().busy,false);}

test('real writer exports one complete XLSX Blob with exact Unicode filename and MIME',async()=>{
  const f=fixture(),result=await f.controller.run();assert.equal(result.ok,true,JSON.stringify(result));assert.equal(f.model.reads,1);assert.equal(f.downloads.length,1);
  const {blob,filename}=f.downloads[0];assert.ok(blob instanceof Blob);assert.equal(filename,'Titel – geprüft.xlsx');assert.equal(blob.type,Xlsx.MIME);assert.equal(UI.MIME,Xlsx.MIME);assert.equal(result.value.bytes,blob.size);
  const bytes=Buffer.from(await blob.arrayBuffer());assert.equal(bytes.readUInt32LE(0),0x04034b50);assert.equal(bytes.readUInt32LE(bytes.length-22),0x06054b50);
  const wire=bytes.toString('utf8');assert.match(wire,/=literal/);assert.match(wire,/https:\/\/example.test\/ä/);assert.match(wire,/<v>42<\/v>/);assert.match(wire,/<c r="B3"\/>/);assert.match(wire,/crawl-a/);
  assert.deepEqual(f.states.map(s=>[s.busy,s.phase]),[[true,'prepare'],[true,'encode'],[true,'download'],[false,'complete']]);
  assert.deepEqual(Object.keys(f.controller.state()),['busy','phase','error','message']);assert.equal(f.controller.state().error,null);
  assert.ok(Object.isFrozen(f.controller));assert.ok(Object.isFrozen(result));assert.ok(Object.isFrozen(result.value));assert.ok(f.states.every(Object.isFrozen));
});

test('default yield crosses a real task boundary before snapshot and another before encoding',async()=>{
  const f=fixture(),running=f.controller.run();assert.equal(f.controller.state().phase,'prepare');assert.equal(f.model.reads,0);assert.equal(f.downloads.length,0);
  await Promise.resolve();assert.equal(f.model.reads,0);assert.equal(f.downloads.length,0);assert.equal(f.controller.state().busy,true);
  await new Promise(resolve=>setTimeout(resolve,0));assert.equal(f.model.reads,1);assert.equal(f.controller.state().phase,'encode');assert.equal(f.downloads.length,0);
  assert.equal((await running).ok,true);assert.equal(f.downloads.length,1);
});

for(const [name,mutate] of [
  ['owner',s=>s.owner.projectId='crawl-b'],['generation',s=>s.generation++],['query revision',s=>s.queryRevision++],
  ['visible columns',s=>s.columns.reverse()],['sort',s=>s.sort.direction=1],['mode',s=>s.mode='compare']
])test('mutating the reused '+name+' stamp during yield refuses the old export',async()=>{
  const f=fixture({deferred:true}),running=f.controller.run();mutate(f.model.stamp);f.yields[0].resolve();failure(await running,'stale');noDownload(f);assert.equal(f.model.reads,0);assert.equal(f.controller.state().error.code,'stale');
});

test('equivalent freshly allocated metadata is accepted without relying on object identity',async()=>{
  const f=fixture({deferred:true}),running=f.controller.run();f.model.stamp=clone(f.model.stamp);f.yields[0].resolve();assert.equal((await running).ok,true);assert.equal(f.downloads.length,1);
});

for(const [field,code] of [['available','unavailable'],['connected','detached']])test(field+' is checked again after yield',async()=>{
  const f=fixture({deferred:true}),running=f.controller.run();f.model[field]=false;f.yields[0].resolve();failure(await running,code);noDownload(f);
});

test('unavailable/detached initial controls cannot read or download',async()=>{
  for(const [field,code] of [['available','unavailable'],['connected','detached']]){
    const f=fixture();f.model[field]=false;failure(await f.controller.run(),code);assert.equal(f.model.reads,0);noDownload(f);
  }
});

test('a double programmatic run fails without unlocking or changing the active status',async()=>{
  const f=fixture({deferred:true}),first=f.controller.run(),before=f.controller.state(),count=f.states.length;
  failure(await f.controller.run(),'busy');assert.equal(f.controller.state(),before);assert.equal(f.states.length,count);assert.equal(f.model.reads,0);
  f.yields[0].resolve();assert.equal((await first).ok,true);assert.equal(f.downloads.length,1);
});

for(const method of ['cancel','invalidate'])test(method+' blocks an old completion from unlocking/overwriting a newer operation',async()=>{
  const f=fixture({deferred:true}),old=f.controller.run();f.controller[method]();
  const expected=method==='cancel'?'cancelled':'stale';assert.equal(f.controller.state().error.code,expected);
  f.model.filename='new.xlsx';const newer=f.controller.run(),newState=f.controller.state(),count=f.states.length;
  f.yields[0].resolve();failure(await old,expected);assert.equal(f.controller.state(),newState);assert.equal(f.controller.state().busy,true);assert.equal(f.states.length,count);assert.equal(f.downloads.length,0);
  f.yields[1].resolve();assert.equal((await newer).ok,true);assert.deepEqual(f.downloads.map(d=>d.filename),['new.xlsx']);
});

test('a rejected old yield cannot overwrite the newer completed export',async()=>{
  const f=fixture({deferred:true}),old=f.controller.run();f.controller.cancel();const newer=f.controller.run();f.yields[1].resolve();assert.equal((await newer).ok,true);
  const complete=f.controller.state(),count=f.states.length;f.yields[0].reject(new Error('old task failed'));failure(await old,'cancelled');assert.equal(f.controller.state(),complete);assert.equal(f.states.length,count);assert.equal(f.downloads.length,1);
});

test('stale changes made during snapshot are caught before planning/encoding',async()=>{
  const f=fixture();f.hooks.snapshot=()=>{f.model.stamp.queryRevision++;return{ok:true,value:{data:f.model.data,filename:f.model.filename}};};failure(await f.controller.run(),'stale');noDownload(f);
});

test('a changed query after real planning refuses before encoding/download',async()=>{
  const f=fixture({deferred:true,deferPhase:'encode'}),running=f.controller.run();
  await Promise.resolve();await Promise.resolve();
  assert.equal(f.model.reads,1);assert.equal(f.controller.state().phase,'encode');assert.equal(f.yields.length,1);
  f.model.stamp.queryRevision++;f.yields[0].resolve();failure(await running,'stale');noDownload(f);
});

test('status callbacks cannot silently change export ownership or reenter run',async()=>{
  const f=fixture();let nested;f.hooks.changed=state=>{f.states.push(state);if(state.phase==='prepare'){nested=f.controller.run();f.model.stamp.owner.projectId='other';}};
  failure(await f.controller.run(),'stale');failure(await nested,'busy');noDownload(f);
});

test('detachment in download-phase notification refuses before Blob/download',async()=>{
  const f=fixture();f.hooks.changed=state=>{f.states.push(state);if(state.phase==='download')f.model.connected=false;};failure(await f.controller.run(),'detached');noDownload(f);
});

test('a change during Blob creation is caught by the last freshness check',async()=>{
  let f;class ChangingBlob extends Blob{constructor(...args){super(...args);f.model.stamp.queryRevision++;}}
  f=fixture({env:{Blob:ChangingBlob,setTimeout}});failure(await f.controller.run(),'stale');noDownload(f);
});

test('snapshot Result refusal keeps its exact reason; a thrown read fails closed',async()=>{
  const f=fixture({hooks:{snapshot:()=>({ok:false,error:{code:'query_incomplete',message:'Rows are incomplete.'}})}});
  const reason=failure(await f.controller.run(),'query_incomplete');assert.equal(reason.message,'Rows are incomplete.');assert.equal(f.controller.state().error,reason);assert.equal(f.controller.state().message,reason.message);noDownload(f);
  const thrown=fixture({hooks:{snapshot:()=>{throw new Error('private detail');}}});failure(await thrown.controller.run(),'read_failed');noDownload(thrown);
});

test('actual 32768 UTF-16 cell refusal preserves the writer cell_limit code and message',async()=>{
  const f=fixture();f.model.data.rows[0][1]='a'.repeat(32768);const direct=f.hooks.writer.prepareTable(f.model.data);assert.equal(direct.ok,false);
  const reason=failure(await f.controller.run(),'cell_limit');assert.deepEqual(reason,direct.error);assert.deepEqual(f.controller.state().error,direct.error);assert.equal(f.states.some(s=>s.phase==='encode'),false);noDownload(f);
});

test('actual writer encoding-byte refusal emits no partial Blob or download',async()=>{
  const f=fixture({writer:Xlsx.create({maxWorkbookBytes:1})});failure(await f.controller.run(),'workbook_limit');noDownload(f);assert.equal(f.states.some(s=>s.phase==='download'),false);
});

for(const name of ['','../report.xlsx','folder\\report.xlsx','report.csv','bad\u0000.xlsx'])test('invalid filename '+JSON.stringify(name)+' is refused before export',async()=>{
  const f=fixture();f.model.filename=name;failure(await f.controller.run(),'filename');noDownload(f);
});

test('Blob unavailable, constructor failure and incomplete Blob refuse without dispatch',async()=>{
  for(const [env,code] of [[{setTimeout},'blob_unavailable'],[{setTimeout,Blob:class{constructor(){throw new Error('full');}}},'blob_failed'],[{setTimeout,Blob:class{constructor(){this.size=0;this.type=UI.MIME;}}},'blob_failed']]){
    const f=fixture({env});failure(await f.controller.run(),code);noDownload(f);
  }
});

test('download dispatch failure is surfaced once and never retried',async()=>{
  let calls=0;const f=fixture({hooks:{download:()=>{calls++;throw new Error('blocked');}}});failure(await f.controller.run(),'download_failed');assert.equal(calls,1);noDownload(f);
});

test('yield failure and missing task scheduler refuse before download',async()=>{
  const f=fixture({hooks:{yieldControl:()=>Promise.reject(new Error('stopped'))}});failure(await f.controller.run(),'yield_failed');noDownload(f);
  const absent=fixture({env:{Blob}});failure(await absent.controller.run(),'yield_failed');noDownload(absent);
});

test('opaque, cyclic and accessor stamps fail closed without invoking the getter',async()=>{
  let reads=0;const accessor={};Object.defineProperty(accessor,'query',{enumerable:true,get(){reads++;return 1;}});
  const cycle={};cycle.self=cycle;
  for(const stamp of [undefined,new Date(),cycle,accessor]){const f=fixture();f.model.stamp=stamp;failure(await f.controller.run(),'stamp');noDownload(f);assert.equal(f.model.reads,0);}
  assert.equal(reads,0);
});

test('a renderer exception fails closed and a reentrant new operation remains owned',async()=>{
  const f=fixture({hooks:{changed:()=>{throw new Error('render');}}});failure(await f.controller.run(),'ui_failed');noDownload(f);assert.equal(f.controller.state().error.code,'ui_failed');
  const g=fixture({deferred:true});let newer;let replaced=false;
  g.hooks.changed=state=>{g.states.push(state);if(state.phase==='cancelled'&&!replaced){replaced=true;newer=g.controller.run();throw new Error('old renderer');}};
  const old=g.controller.run();g.controller.cancel();assert.equal(g.controller.state().busy,true);const nextState=g.controller.state();g.yields[0].resolve();failure(await old,'cancelled');assert.equal(g.controller.state(),nextState);
  g.yields[1].resolve();assert.equal((await newer).ok,true);assert.equal(g.downloads.length,1);
});

test('UMD browser export loads without document, network or storage globals',()=>{
  const context=vm.createContext({});vm.runInContext(fs.readFileSync(path.join(__dirname,'../table-export-ui.js'),'utf8'),context);
  assert.equal(typeof context.TableExportUI.create,'function');assert.equal(context.TableExportUI.MIME,Xlsx.MIME);assert.ok(Object.isFrozen(context.TableExportUI));
});
