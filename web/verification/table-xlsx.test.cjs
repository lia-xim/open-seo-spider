'use strict';
const test=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const vm=require('node:vm');
const {crc32}=require('node:zlib');
const Xlsx=require('../table-xlsx.js');

const PARTS=['[Content_Types].xml','_rels/.rels','xl/workbook.xml','xl/_rels/workbook.xml.rels','xl/worksheets/sheet1.xml'];
function success(result){assert.equal(result.ok,true,JSON.stringify(result.error));return result.value;}
function refuses(result,code){assert.equal(result.ok,false);assert.equal(result.error.code,code);assert.equal(Object.hasOwn(result,'value'),false);return result;}
function input(rows=[['value']],columns=[{key:'value',label:'Value'}],context=[]){return{columns,rows,context};}
function bytes(data=input(),limits){const writer=Xlsx.create(limits);return success(writer.encode(success(writer.prepareTable(data)))).bytes;}

// An actual-byte reader, independent of the writer's address/CRC/escaping helpers.
// This deliberately accepts only the writer's documented five-part stored ZIP.
function archive(data){
  const b=Buffer.from(data),last=b.length-22;
  assert.ok(last>=0);assert.equal(b.readUInt32LE(last),0x06054b50);
  assert.equal(b.readUInt16LE(last+4),0);assert.equal(b.readUInt16LE(last+6),0);
  assert.equal(b.readUInt16LE(last+8),5);assert.equal(b.readUInt16LE(last+10),5);assert.equal(b.readUInt16LE(last+20),0);
  const start=b.readUInt32LE(last+16),size=b.readUInt32LE(last+12);
  assert.equal(start+size,last);let local=0,central=start;const entries=new Map();
  for(const name of PARTS){
    assert.equal(b.readUInt32LE(local),0x04034b50);assert.equal(b.readUInt16LE(local+4),20);
    assert.equal(b.readUInt16LE(local+6),0x800);assert.equal(b.readUInt16LE(local+8),0);
    assert.equal(b.readUInt16LE(local+10),0);assert.equal(b.readUInt16LE(local+12),33);
    const check=b.readUInt32LE(local+14),length=b.readUInt32LE(local+18),nameLength=b.readUInt16LE(local+26);
    assert.equal(b.readUInt32LE(local+22),length);assert.equal(b.readUInt16LE(local+28),0);
    assert.equal(b.toString('utf8',local+30,local+30+nameLength),name);
    const content=b.subarray(local+30+nameLength,local+30+nameLength+length);
    assert.equal(crc32(content),check,'native zlib CRC32: '+name);
    assert.equal(b.readUInt32LE(central),0x02014b50);
    assert.equal(b.readUInt16LE(central+4),20);assert.equal(b.readUInt16LE(central+6),20);
    assert.equal(b.readUInt16LE(central+8),0x800);assert.equal(b.readUInt16LE(central+10),0);
    assert.equal(b.readUInt16LE(central+12),0);assert.equal(b.readUInt16LE(central+14),33);
    assert.equal(b.readUInt32LE(central+16),check);assert.equal(b.readUInt32LE(central+20),length);assert.equal(b.readUInt32LE(central+24),length);
    assert.equal(b.readUInt16LE(central+28),nameLength);assert.equal(b.readUInt16LE(central+30),0);assert.equal(b.readUInt16LE(central+32),0);
    assert.equal(b.readUInt16LE(central+34),0);assert.equal(b.readUInt16LE(central+36),0);assert.equal(b.readUInt32LE(central+38),0);assert.equal(b.readUInt32LE(central+42),local);
    assert.equal(b.toString('utf8',central+46,central+46+nameLength),name);
    entries.set(name,new TextDecoder('utf-8',{fatal:true}).decode(content));
    local+=30+nameLength+length;central+=46+nameLength;
  }
  assert.equal(local,start);assert.equal(central,last);return entries;
}
function col(index){let n=index+1,out='';while(n>0){const remainder=(n-1)%26;out=String.fromCharCode(65+remainder)+out;n=(n-1-remainder)/26;}return out;}
function decodeText(wire){
  // Decode XML once, then ST_Xstring once. Never recursively decode protected text.
  assert.doesNotMatch(wire,/[<>\r\u0000-\u0008\u000b\u000c\u000e-\u001f\ufffe\uffff]/u);
  assert.doesNotMatch(wire,/&(?!(?:amp|lt|gt);)/);
  return wire.replace(/&(amp|lt|gt);/g,(_,name)=>({amp:'&',lt:'<',gt:'>'})[name]).replace(/_x([0-9a-fA-F]{4})_/g,(_,hex)=>String.fromCharCode(parseInt(hex,16)));
}
function worksheet(data){
  const parts=archive(data),xml=parts.get('xl/worksheets/sheet1.xml');
  assert.doesNotMatch(xml,/<(?:f|hyperlink|extLst)(?:\s|>)/);
  const match=/^<\?xml version="1.0" encoding="UTF-8" standalone="yes"\?><worksheet xmlns="http:\/\/schemas\.openxmlformats\.org\/spreadsheetml\/2006\/main"><dimension ref="(A1:[A-Z]+[1-9][0-9]*)"\/><sheetData>(.*)<\/sheetData><\/worksheet>$/s.exec(xml);
  assert.ok(match,'complete worksheet envelope');const rows=[],types=[],coordinates=[];let rowAt=0;
  for(const row of match[2].matchAll(/<row r="([1-9][0-9]*)">(.*?)<\/row>/gs)){
    assert.equal(row.index,rowAt);rowAt+=row[0].length;assert.equal(Number(row[1]),rows.length+1);
    const values=[],kinds=[],refs=[];let cellAt=0;
    for(const cell of row[2].matchAll(/<c r="([A-Z]+[1-9][0-9]*)"(?:\/>| t="(inlineStr|n|b)">(.*?)<\/c>)/gs)){
      assert.equal(cell.index,cellAt);cellAt+=cell[0].length;assert.equal(cell[1],col(values.length)+row[1]);refs.push(cell[1]);kinds.push(cell[2]??'blank');
      if(!cell[2])values.push(null);
      else if(cell[2]==='inlineStr'){
        const text=/^<is><t xml:space="preserve">(.*)<\/t><\/is>$/s.exec(cell[3]);assert.ok(text);values.push(decodeText(text[1]));
      }else{
        const value=/^<v>([^<]+)<\/v>$/.exec(cell[3]);assert.ok(value);
        if(cell[2]==='b'){assert.match(value[1],/^[01]$/);values.push(value[1]==='1');}
        else{assert.match(value[1],/^-?(?:[0-9]+(?:\.[0-9]+)?)(?:e[+-]?[0-9]+)?$/);assert.ok(Number.isFinite(Number(value[1])));values.push(Number(value[1]));}
      }
    }
    assert.equal(cellAt,row[2].length);rows.push(values);types.push(kinds);coordinates.push(refs);
  }
  assert.equal(rowAt,match[2].length);assert.equal(match[1],'A1:'+col(rows[0].length-1)+rows.length);
  for(const row of rows)assert.equal(row.length,rows[0].length);
  return{parts,xml,rows,types,coordinates,dimension:match[1]};
}

test('minimal workbook has a real five-part ZIP, CRC, package targets and typed cells',()=>{
  const out=worksheet(bytes());assert.deepEqual(out.rows,[['Value'],['value']]);assert.deepEqual(out.types,[['inlineStr'],['inlineStr']]);
  assert.match(out.parts.get('_rels/.rels'),/Type="http:\/\/schemas.openxmlformats.org\/officeDocument\/2006\/relationships\/officeDocument" Target="xl\/workbook.xml"/);
  assert.match(out.parts.get('xl/_rels/workbook.xml.rels'),/Type="http:\/\/schemas.openxmlformats.org\/officeDocument\/2006\/relationships\/worksheet" Target="worksheets\/sheet1.xml"/);
  assert.match(out.parts.get('xl/workbook.xml'),/<sheet name="Ergebnisse" sheetId="1" r:id="rId1"\/>/);
  assert.match(out.parts.get('[Content_Types].xml'),/PartName="\/xl\/worksheets\/sheet1.xml" ContentType="application\/vnd.openxmlformats-officedocument.spreadsheetml.worksheet\+xml"/);
  assert.equal(Xlsx.MIME,'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet');
});
test('the independent ZIP reader rejects a damaged data byte and trailing bytes',()=>{
  const original=bytes(),corrupt=Uint8Array.from(original);corrupt[100]^=1;
  assert.throws(()=>archive(corrupt),/CRC32/);assert.throws(()=>archive(Buffer.concat([original,Buffer.from([0])])));
});
test('all ordered columns, rows and repeated context are retained with raw scalar types',()=>{
  const data=input([[null,'',0,false],[42,'000042',true,'e\u0301'],['=SUM(A1:A2)','+1','-2','@x']],
    ['Unknown','Text','Number','Boolean'].map((label,i)=>({key:'c'+i,label})),[{key:'owner',label:'Crawl ID',value:'local-owner'},{key:'unknown',label:'Unknown context',value:null}]);
  const writer=Xlsx.create(),snapshot=success(writer.prepareTable(data)),encoded=success(writer.encode(snapshot)),out=worksheet(encoded.bytes);
  assert.deepEqual(out.rows,[['Unknown','Text','Number','Boolean','Crawl ID','Unknown context'],...[...data.rows].map(row=>[...row,'local-owner',null])]);
  assert.deepEqual(out.types[1],['blank','inlineStr','n','b','inlineStr','blank']);
  // 47 header bytes + 38 base-value bytes + 11 context bytes per output row.
  assert.deepEqual(snapshot.stats,{rowCount:3,columnCount:6,cellSlots:24,payloadBytes:118,inputBytes:96,unknownCells:4});
  assert.equal(encoded.rows,3);assert.equal(encoded.columns,6);assert.equal(encoded.worksheetBytes,Buffer.byteLength(out.xml));
});
test('formula-looking text, XML metacharacters and CSV-style apostrophes are literal strings',()=>{
  const values=['=1+1','+SUM(A1)','-1','@SUM(A1)',"'=A1",'https://example.test/?a=1&b=<2>','"quoted"',"'single'",' leading\tand trailing '];
  const out=worksheet(bytes(input(values.map(value=>[value]))));assert.deepEqual(out.rows.slice(1),values.map(value=>[value]));
  assert.ok(out.types.flat().every(type=>type==='inlineStr'));assert.doesNotMatch([...out.parts.values()].join(''),/<f>|TargetMode=|<hyperlink|macro|vbaProject/);
});
test('literal overlapping ST_Xstring patterns protect every original escape starter once',()=>{
  const literals=['_x005F_x0041_','_x0041_x0042_','_x005f_x000d_','_x005F_','_x0000_','_xD800_','_X0041_','_x0041_&lt;_x0041_','_x005F_x005F_x0041_','prefix_x0041_suffix'];
  const out=worksheet(bytes(input(literals.map(value=>[value]))));assert.deepEqual(out.rows.slice(1),literals.map(value=>[value]));
  assert.match(out.xml,/_x005F_x005F_x005F_x0041_/);assert.match(out.xml,/_X0041_/);
});
test('representable XML-forbidden scalars use Xstring while TAB and LF remain literal',()=>{
  const controls=Array.from({length:32},(_,i)=>String.fromCharCode(i)).join('')+'\ufffe\uffff';
  const values=[controls,'CR\rLF\nTAB\tend','\r_x000D_\r','\ud83d\ude00 e\u0301 é \udbff\udfff'];
  const out=worksheet(bytes(input(values.map(value=>[value]))));assert.deepEqual(out.rows.slice(1),values.map(value=>[value]));
  assert.match(out.xml,/_x0000_/);assert.match(out.xml,/_x000D_/);assert.match(out.xml,/_xFFFE_/);assert.match(out.xml,/_xFFFF_/);assert.ok(out.xml.includes('\t'));assert.ok(out.xml.includes('\n'));
});
test('headers and context use the same lossless Xstring and Unicode rules',()=>{
  const data=input([['x']], [{key:'a',label:'_x005F_x0041_\r&\u0000'}],[{key:'c',label:'Context\uffff',value:'\u0001_x0001_\ud83d\ude00'}]);
  assert.deepEqual(worksheet(bytes(data)).rows,[[data.columns[0].label,data.context[0].label],['x',data.context[0].value]]);
});
test('unpaired UTF16 in any header, context or late cell refuses the entire snapshot',()=>{
  const writer=Xlsx.create();for(const value of ['\ud800','\udc00','a\ud800b','\ud83d\ude00\udfff']){
    refuses(writer.prepareTable(input([['first'],[value]])),'unicode');
    refuses(writer.prepareTable(input([], [{key:'a',label:value}])),'unicode');
    refuses(writer.prepareTable(input([],undefined,[{key:'c',label:'Context',value}])),'unicode');
  }
  assert.deepEqual(worksheet(bytes(input([['\ud800\udc00'],['\udbff\udfff']]))).rows.slice(1),[['\ud800\udc00'],['\udbff\udfff']]);
});
test('explicit numbers retain 15 significant decimal digits, signs, exponents and bounds',()=>{
  const values=[0,-0,1,-1,123456789012345,1234567890123450,0.123456789012345,1e20,1e307,9.99999999999999e307,2.2251e-308,-2.2251e-308];
  const out=worksheet(bytes(input(values.map(value=>[value]))));assert.deepEqual(out.rows.slice(1),values.map(value=>[Object.is(value,-0)?0:value]));
  assert.ok(out.types.slice(1).every(row=>row[0]==='n'));
});
test('numbers that need rounding or exceed Excel range refuse; identifiers stay text',()=>{
  const writer=Xlsx.create();for(const value of [NaN,Infinity,-Infinity,Number.MIN_VALUE,1e-308,1e308,1234567890123456,9007199254740992,0.30000000000000004])refuses(writer.prepareTable(input([[value]])),'number');
  const values=['9007199254740992','1234567890123456','00001','1e9999'];assert.deepEqual(worksheet(bytes(input(values.map(value=>[value])))).rows.slice(1),values.map(value=>[value]));
});
test('non-scalars are rejected without implicit conversions',()=>{
  let called=0;const object={toString(){called++;return'x';},valueOf(){called++;return 1;}};
  for(const value of [undefined,1n,Symbol('x'),object,()=>1,new Date(),new String('x'),new Number(1)])refuses(Xlsx.create().prepareTable(input([[value]])),'scalar');assert.equal(called,0);
});
test('records require exact own enumerable data fields and never invoke getters',()=>{
  let called=0;const getter=()=>{called++;return[];};
  const cases=[{columns:[],rows:[],context:[],extra:true},Object.assign(Object.create({}),input()),Object.defineProperty(input(),'rows',{get:getter,enumerable:true}),Object.defineProperty(input(),'rows',{value:[],enumerable:false}),{...input(),[Symbol('extra')]:1},input([], [{key:'a',label:'A',extra:1}]),input([], [{key:'a',get label(){called++;return'A';}}]),input([],undefined,[{key:'c',label:'Context',get value(){called++;return'x';}}])];
  for(const data of cases)refuses(Xlsx.create().prepareTable(data),'shape');assert.equal(called,0);
  const data=Object.assign(Object.create(null),input());assert.equal(Xlsx.create().prepareTable(data).ok,true);
});
test('rows and arrays require dense own data slots with ordinary array prototypes',()=>{
  let called=0;const accessor=['x'];Object.defineProperty(accessor,'0',{get(){called++;return'x';},enumerable:true});
  const hidden=['x'];Object.defineProperty(hidden,'0',{value:'x',enumerable:false});
  const symbol=['x'];symbol[Symbol('extra')]=1;const extra=['x'];extra.extra=1;
  class SubArray extends Array{};
  for(const row of [Array(1),accessor,hidden,symbol,extra,new SubArray('x')])refuses(Xlsx.create().prepareTable(input([row])),'shape');
  for(const data of [input(Array(1)),input([],Array(1)),input([],undefined,Array(1))])refuses(Xlsx.create().prepareTable(data),'shape');assert.equal(called,0);
});
test('invalid columns and row width cannot produce an incomplete projection',()=>{
  const writer=Xlsx.create();for(const columns of [[],[{key:'x',label:''}],[{key:'1bad',label:'A'}],[{key:'x y',label:'A'}],[{key:'x'.repeat(129),label:'A'}],[{key:'x',label:'A'},{key:'x',label:'B'}],[{key:'a',label:'A'},{key:'b',label:'A'}]])refuses(writer.prepareTable(input([],columns)),'columns');
  refuses(writer.prepareTable(input([],undefined,[{key:'value',label:'Context',value:'x'}])),'columns');
  refuses(writer.prepareTable(input([],undefined,[{key:'c',label:'Value',value:'x'}])),'columns');
  refuses(writer.prepareTable(input([[]])),'shape');refuses(writer.prepareTable(input([['a','b']])),'shape');
});
test('unexpected proxy failures expose no thrown object fields or partial output',()=>{
  let called=0;const thrown=Object.defineProperties({}, {message:{get(){called++;return'secret';}},code:{get(){called++;return'code';}}});
  const data=new Proxy(input(),{getPrototypeOf(){throw thrown;}});const out=refuses(Xlsx.create().prepareTable(data),'invalid_data');assert.equal(called,0);assert.doesNotMatch(out.error.message,/secret/);
});
test('snapshots are deeply frozen, instance-issued and independent of later input mutation',()=>{
  const writer=Xlsx.create(),data=input([['first']],undefined,[{key:'c',label:'Context',value:'original'}]);const snapshot=success(writer.prepareTable(data));
  for(const value of [snapshot,snapshot.columns,...snapshot.columns,snapshot.rows,...snapshot.rows,snapshot.stats])assert.equal(Object.isFrozen(value),true);
  data.columns[0].label='changed';data.rows[0][0]='changed';data.rows.push(['new']);data.context[0].value='changed';assert.deepEqual(worksheet(success(writer.encode(snapshot)).bytes).rows,[['Value','Context'],['first','original']]);
  for(const foreign of [null,{},JSON.parse(JSON.stringify(snapshot)),new Proxy(snapshot,{}),success(Xlsx.create().prepareTable(input()))])refuses(writer.encode(foreign),'snapshot');
  assert.throws(()=>{snapshot.rows[0][0]='changed';},TypeError);
});
test('every encode returns fresh deterministic bytes; caller mutation cannot alter the planner',()=>{
  const writer=Xlsx.create(),snapshot=success(writer.prepareTable(input())),first=success(writer.encode(snapshot)),second=success(writer.encode(snapshot));
  assert.notEqual(first.bytes,second.bytes);assert.deepEqual(first.bytes,second.bytes);assert.equal(Object.isFrozen(first),true);first.bytes.fill(0);assert.deepEqual(success(writer.encode(snapshot)).bytes,second.bytes);
});
test('limit overrides only lower known integer limits using own data descriptors',()=>{
  let called=0;for(const config of [null,[],new Number(1),{unknown:1},{maxRows:-1},{maxRows:1.5},{maxRows:Xlsx.LIMITS.maxRows+1},{maxCells:0},{[Symbol('x')]:1},Object.defineProperty({},'maxRows',{value:1,enumerable:false}),{get maxRows(){called++;return 1;}}])assert.throws(()=>Xlsx.create(config));assert.equal(called,0);
  const writer=Xlsx.create({maxRows:0,maxCellLineFeeds:0});assert.equal(Object.isFrozen(writer.limits),true);assert.equal(writer.prepareTable(input([])).ok,true);refuses(writer.prepareTable(input([['x']])),'rows_limit');refuses(writer.prepareTable(input([], [{key:'a',label:'a\n'}])),'cell_limit');
});
test('row and combined-column limits accept exact boundaries and reject before sparse traversal',()=>{
  const writer=Xlsx.create({maxRows:2,maxColumns:2});assert.equal(writer.prepareTable(input([['x'],['y']],undefined,[{key:'c',label:'C',value:null}])).ok,true);
  refuses(writer.prepareTable(input(Array(3))),'rows_limit');refuses(writer.prepareTable(input([],undefined,[{key:'c',label:'C',value:null},{key:'d',label:'D',value:null}])),'columns_limit');
  refuses(Xlsx.create().prepareTable(input(Array(Xlsx.LIMITS.maxRows+1))),'rows_limit');
});
test('cell budgets count headers, nulls and each repeated context cell, atomically',()=>{
  const data=input([[null],[null]],undefined,[{key:'c',label:'C',value:null}]);const writer=Xlsx.create({maxCells:6});assert.equal(success(writer.prepareTable(data)).stats.cellSlots,6);refuses(Xlsx.create({maxCells:5}).prepareTable(data),'cells_limit');
  const sparse=Array(1000000);Object.defineProperty(sparse,'0',{get(){throw Error('must not inspect a cell beyond budget');},enumerable:true});refuses(Xlsx.create().prepareTable(input(sparse)),'cells_limit');
});
test('UTF16, UTF8 and line-feed cell limits accept exact values and refuse one above',()=>{
  assert.equal(Xlsx.create({maxCellCodeUnits:5}).prepareTable(input([['a'.repeat(5)]])).ok,true);refuses(Xlsx.create({maxCellCodeUnits:5}).prepareTable(input([['a'.repeat(6)]])),'cell_limit');
  assert.equal(Xlsx.create({maxCellBytes:5}).prepareTable(input([['ä€']])).ok,true);refuses(Xlsx.create({maxCellBytes:5}).prepareTable(input([['€€']])),'cell_limit');
  assert.equal(Xlsx.create().prepareTable(input([['x'.repeat(32767)]])).ok,true);refuses(Xlsx.create().prepareTable(input([['x'.repeat(32768)]])),'cell_limit');
  assert.equal(Xlsx.create().prepareTable(input([['\n'.repeat(253)]])).ok,true);refuses(Xlsx.create().prepareTable(input([['\n'.repeat(254)]])),'cell_limit');
});
test('payload limit counts exact UTF8 output including repeated context and unused source context',()=>{
  const data=input([['ä'],['x']],[{key:'a',label:'A'}],[{key:'c',label:'C',value:'xyz'}]);
  const stats=success(Xlsx.create({maxPayloadBytes:11}).prepareTable(data)).stats;assert.equal(stats.payloadBytes,11);assert.equal(stats.inputBytes,8);
  refuses(Xlsx.create({maxPayloadBytes:10}).prepareTable(data),'payload_limit');
  assert.equal(Xlsx.create({maxPayloadBytes:5}).prepareTable(input([],[{key:'a',label:'A'}],[{key:'c',label:'C',value:'xyz'}])).ok,true);
  refuses(Xlsx.create({maxPayloadBytes:4}).prepareTable(input([],[{key:'a',label:'A'}],[{key:'c',label:'C',value:'xyz'}])),'payload_limit');
});
test('worksheet and ZIP byte limits are exact full-file gates, with no partial bytes',()=>{
  const data=input([['\u0000<&_x0041_']]),writer=Xlsx.create(),snapshot=success(writer.prepareTable(data)),base=success(writer.encode(snapshot));
  const exact=Xlsx.create({maxWorksheetBytes:base.worksheetBytes,maxWorkbookBytes:base.bytes.length});assert.deepEqual(success(exact.encode(success(exact.prepareTable(data)))).bytes,base.bytes);
  const sheetShort=Xlsx.create({maxWorksheetBytes:base.worksheetBytes-1});refuses(sheetShort.encode(success(sheetShort.prepareTable(data))),'worksheet_limit');
  const zipShort=Xlsx.create({maxWorkbookBytes:base.bytes.length-1});refuses(zipShort.encode(success(zipShort.prepareTable(data))),'workbook_limit');assert.deepEqual(success(writer.encode(snapshot)).bytes,base.bytes);
});
test('over ten thousand rows are supported without a product row cap',()=>{
  const data=input(Array.from({length:10001},()=>[null]));const writer=Xlsx.create(),snapshot=success(writer.prepareTable(data));const out=worksheet(success(writer.encode(snapshot)).bytes);
  assert.equal(out.rows.length,10002);assert.equal(out.dimension,'A1:A10002');assert.equal(snapshot.stats.unknownCells,10001);
});
test('the full physical Excel column range reaches XFD and refuses XFE',()=>{
  const columns=Array.from({length:16384},(_,i)=>({key:'c'+i,label:'C'+i}));const out=worksheet(bytes(input([],columns)));assert.equal(out.dimension,'A1:XFD1');assert.equal(out.coordinates[0].at(-1),'XFD1');assert.equal(out.rows[0].length,16384);
  refuses(Xlsx.create().prepareTable(input([], [...columns,{key:'last',label:'Last'}])),'columns_limit');
});
test('zero data rows is a complete headers-only table including declared context',()=>{
  const data=input([],undefined,[{key:'c',label:'Context',value:'unused'}]);const out=worksheet(bytes(data));assert.deepEqual(out.rows,[['Value','Context']]);assert.equal(out.dimension,'A1:B1');
});
test('UMD browser entry works without CommonJS, DOM, storage or network globals',()=>{
  const realm=vm.createContext({TextEncoder});vm.runInContext(fs.readFileSync(path.join(__dirname,'../table-xlsx.js'),'utf8'),realm);
  const result=vm.runInContext(`(()=>{const writer=TableXlsx.create();const prepared=writer.prepareTable({columns:[{key:'x',label:'X'}],rows:[['browser']],context:[]});return writer.encode(prepared.value);})()`,realm);
  assert.equal(result.ok,true);assert.deepEqual(worksheet(result.value.bytes).rows,[['X'],['browser']]);assert.equal(realm.module,undefined);
});
