// Production import state regressions. Transport is mocked; browser/API proof is separate.
import test from 'node:test';
import assert from 'node:assert/strict';
import helpers from '../list-import.js';
const {ImportPreviewState,buildListOptions,summarizeRows,MAX_INPUT_BYTES}=helpers;
const deferred=()=>{let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no});return {promise,resolve,reject}};
const file=(name,text)=>{const bytes=new TextEncoder().encode(text);return {name,size:bytes.length,arrayBuffer:async()=>bytes.buffer}};
const options={format:{kind:'txt'}};
const envelope=(bytes,rows=[])=>({engine:'fixture_only',input_sha256:'a'.repeat(64),input_bytes:bytes,options,selected_sheet:null,selected_column:null,rows,website_requests:0,warnings:[],limitations:[]});
const row=(index,url,error=null,kind='page_url')=>({source_index:index,source_row:index,source_line:index,sheet:null,cell:null,raw_value:url||'invalid',cell_type:'plain_text',formula_metadata:null,candidate_kind:kind,normalized_url:url,fragment:null,error});

test('closing during parse ignores late response and preserves completed acceptance',async()=>{
  const pending=deferred();const state=new ImportPreviewState({transport:()=>pending.promise});state.setEditedText('A');
  const parsing=state.preview();state.cancelPending();pending.resolve(envelope(1,[row(1,'http://owned.invalid/a')]));await parsing;
  assert.equal(state.state.phase,'selected');assert.equal(state.state.result,null);assert.throws(()=>state.accept());
  state.transport=async()=>envelope(1,[row(1,'http://owned.invalid/a')]);await state.preview();state.accept();state.cancelPending();
  assert.equal(state.state.phase,'accepted');assert.equal(state.state.accepted.rows[0].normalized_url,'http://owned.invalid/a');
});

test('closing during file read prevents old source restoration',async()=>{
  const pending=deferred();const state=new ImportPreviewState({transport:async()=>{throw new Error('Unexpected transport')}});
  const reading=state.setFile({name:'old.txt',size:1,arrayBuffer:()=>pending.promise});state.cancelPending();pending.resolve(new Uint8Array([65]).buffer);await reading;
  assert.equal(state.state.source,null);assert.equal(state.state.phase,'empty');
});

test('manual text import retains exact UTF8 bytes including CRLF empty and invalid lines',async()=>{
  let requested;const original='http://owned.invalid/ä#part\r\n\r\n invalid \n';
  const state=new ImportPreviewState({transport:async(path,body)=>{requested={path,body};return envelope(Buffer.byteLength(original),[row(1,'http://owned.invalid/%C3%A4'),row(2,null,'Empty'),row(3,null,'Invalid')])}});
  state.setEditedText(original);await state.preview();assert.equal(Buffer.from(requested.body.content_base64,'base64').toString('utf8'),original);assert.deepEqual(requested.body.options,{format:{kind:'txt'}});assert.equal(state.state.result.rows.length,3);
});

test('format adapter preserves exact names and constructs only tagged explicit selections',()=>{
  const csv=buildListOptions({kind:'csv',delimiter:'semicolon',column_kind:'header',header_name:' URL ',header_row:1,start_row:2});
  assert.deepEqual(csv,{format:{kind:'csv',delimiter:'semicolon',column:{kind:'header',name:' URL ',row:1},start_row:2}});
  const xlsx=buildListOptions({kind:'xlsx',sheet_kind:'name',sheet_name:' Hidden ',column_kind:'index',column_index:2,start_row:1});
  assert.equal(xlsx.format.sheet.value,' Hidden ');assert.equal(xlsx.format.formula_policy,'reject');
  assert.deepEqual(buildListOptions({kind:'xml'}),{format:{kind:'xml'}});
  assert.throws(()=>buildListOptions({kind:'csv',delimiter:'comma',column_kind:'header',header_name:'URL',header_row:2,start_row:2}));
  assert.throws(()=>buildListOptions({kind:'xls'}));
});

test('client byte cap prevents read and transport before oversize file input',async()=>{
  let reads=0,calls=0;const state=new ImportPreviewState({transport:async()=>{calls++}});
  await state.setFile({name:'oversize.xlsx',size:MAX_INPUT_BYTES+1,arrayBuffer:async()=>{reads++;return new ArrayBuffer(0)}});
  assert.equal(reads,0);assert.equal(calls,0);assert.equal(state.state.phase,'error');assert.equal(state.state.source,null);
});

test('newer file selection wins against old completed file read and old error',async()=>{
  const first=deferred(),second=deferred();const state=new ImportPreviewState({transport:async()=>{throw new Error('not expected')}});
  const oldRead=state.setFile({name:'old.csv',size:1,arrayBuffer:()=>first.promise});
  await state.setFile(file('new.txt','B'));first.resolve(new TextEncoder().encode('A').buffer);await oldRead;
  assert.equal(state.state.source.name,'new.txt');assert.equal(state.state.source.content_base64,'Qg==');
  const oldError=state.setFile({name:'old-error.xlsx',size:1,arrayBuffer:()=>second.promise});
  await state.setFile(file('newer.txt','C'));second.reject(new Error('stale unreadable file'));await oldError;
  assert.equal(state.state.source.name,'newer.txt');assert.equal(state.state.error,'');
});

test('format selection during same-file read survives without replacing a newer source',async()=>{
  const read=deferred();const state=new ImportPreviewState({transport:async()=>{throw new Error('not expected')}});
  const pending=state.setFile({name:'selected.csv',size:1,arrayBuffer:()=>read.promise});
  const csv=buildListOptions({kind:'csv',delimiter:'tab',column_kind:'index',column_index:1,start_row:1});state.setOptions(csv);
  read.resolve(new TextEncoder().encode('A').buffer);await pending;
  assert.equal(state.state.source.name,'selected.csv');assert.deepEqual(state.state.options,csv);assert.equal(state.state.phase,'selected');
});

test('late workbook inspection cannot restore file provenance after URL text edit',async()=>{
  const pending=deferred(),calls=[];const state=new ImportPreviewState({transport:(path,body)=>{calls.push({path,body});return pending.promise}});
  await state.setFile(file('book.xlsx','X'));const inspecting=state.inspect({max_rows:100});state.setEditedText('http://owned.invalid/edited');
  pending.resolve({...envelope(1),sheets:[{index:1,name:'URLs',sheet_type:'WorkSheet',visibility:'Visible'}]});await inspecting;
  assert.deepEqual(calls,[{path:'/api/lists/inspect',body:{content_base64:'WA==',limits:{max_rows:100}}}]);
  assert.equal(state.state.inspection,null);assert.equal(state.state.source.origin,'edited_text');assert.deepEqual(state.state.options,options);
});

test('changed options invalidate pending preview and prohibit stale acceptance',async()=>{
  const pending=deferred();const state=new ImportPreviewState({transport:()=>pending.promise});await state.setFile(file('list.txt','A'));state.setOptions(options);
  const preview=state.preview();state.setOptions({format:{kind:'xml'}});pending.resolve(envelope(1,[row(1,'http://owned.invalid/a')]));await preview;
  assert.equal(state.state.result,null);assert.equal(state.state.phase,'selected');assert.throws(()=>state.accept());
});

test('old request error cannot replace newer successful preview',async()=>{
  const old=deferred(),latest=deferred();let calls=0;const state=new ImportPreviewState({transport:()=>++calls===1?old.promise:latest.promise});await state.setFile(file('list.txt','A'));state.setOptions(options);
  const first=state.preview();state.setOptions(options);const second=state.preview();latest.resolve(envelope(1,[row(1,'http://owned.invalid/current')]));await second;old.reject(new Error('stale response'));await first;
  assert.equal(state.state.phase,'preview');assert.equal(state.state.error,'');assert.equal(state.state.result.rows[0].normalized_url,'http://owned.invalid/current');
});

test('acceptance preserves invalid rows duplicates order and sitemap identity without starting a crawl',async()=>{
  const rows=[row(1,'http://owned.invalid/a'),row(2,null,'invalid scheme'),row(3,'http://owned.invalid/a'),row(4,'http://owned.invalid/sitemap.xml',null,'sitemap_document')],calls=[];
  const state=new ImportPreviewState({transport:async(path,body)=>{calls.push({path,body});return envelope(1,rows)}});await state.setFile(file('list.txt','A'));state.setOptions(options);await state.preview();const accepted=state.accept();
  assert.deepEqual(accepted.seed_urls,['http://owned.invalid/a']);assert.deepEqual(accepted.sitemap_urls,['http://owned.invalid/sitemap.xml']);assert.deepEqual(accepted.input_imports[0].rows,rows);
  assert.deepEqual(summarizeRows(rows),{rows:4,accepted:3,invalid:1,duplicates:1,unique_addresses:2,page_urls:['http://owned.invalid/a'],sitemap_urls:['http://owned.invalid/sitemap.xml']});
  assert.equal(calls.length,1);assert.equal(calls[0].path,'/api/lists/import');accepted.input_imports[0].rows[0].raw_value='mutated';assert.equal(state.state.accepted.rows[0].raw_value,'http://owned.invalid/a');
  state.setEditedText('http://owned.invalid/new');assert.equal(state.state.accepted,null);assert.equal(state.state.result,null);assert.equal(calls.length,1);assert.throws(()=>state.accept());
});

test('nonzero website requests or malformed row envelope never becomes accepted preview',async()=>{
  for(const result of [{...envelope(1),website_requests:1},envelope(1,[{...row(1,'http://owned.invalid/a'),candidate_kind:'unknown'}])]){
    const state=new ImportPreviewState({transport:async()=>result});await state.setFile(file('list.txt','A'));state.setOptions(options);await state.preview();assert.equal(state.state.phase,'error');assert.equal(state.state.result,null);assert.throws(()=>state.accept());
  }
});
