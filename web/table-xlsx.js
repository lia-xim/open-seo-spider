'use strict';
(function(host,factory){
  if(typeof module==='object'&&module.exports)module.exports=factory();
  else host.TableXlsx=factory();
})(typeof globalThis==='object'?globalThis:this,function(){
// Pure, complete-workbook planning. The caller owns query/owner freshness and
// download effects. Raw scalars are never inferred from CSV-escaped strings.
const MIME='application/vnd.openxmlformats-officedocument.spreadsheetml.sheet';
const LIMITS=Object.freeze({maxRows:1048575,maxColumns:16384,maxCells:250000,
  maxCellCodeUnits:32767,maxCellBytes:131068,maxCellLineFeeds:253,
  maxPayloadBytes:16*1024*1024,maxWorksheetBytes:32*1024*1024,maxWorkbookBytes:32*1024*1024});
const NS='http://schemas.openxmlformats.org/spreadsheetml/2006/main';
const REL='http://schemas.openxmlformats.org/officeDocument/2006/relationships';
const utf8=new TextEncoder(),own=(value,key)=>Object.prototype.hasOwnProperty.call(value,key);
const failures=new WeakMap();
function fail(code,message){const error=new Error(message);failures.set(error,Object.freeze({code,message}));throw error;}
function result(action){
  try{return{ok:true,value:action()};}
  catch(error){return{ok:false,error:failures.get(error)||Object.freeze({code:'invalid_data',message:'Die XLSX-Datei konnte nicht vollständig vorbereitet werden.'})};}
}
function record(value,fields){
  if(!value||typeof value!=='object'||![Object.prototype,null].includes(Object.getPrototypeOf(value)))fail('shape','Ein Datenobjekt mit eigenen Feldern ist erforderlich.');
  const keys=Reflect.ownKeys(value);
  if(keys.length!==fields.length||keys.some(key=>typeof key!=='string'||!fields.includes(key)))fail('shape','Die Tabellenprojektion enthält unbekannte oder fehlende Felder.');
  const output={};
  for(const key of fields){
    const d=Object.getOwnPropertyDescriptor(value,key);
    if(!d||!own(d,'value')||!d.enumerable)fail('shape','Berechnete oder versteckte Felder sind keine vollständigen Exportdaten.');
    Object.defineProperty(output,key,{value:d.value,enumerable:true});
  }
  return output;
}
function arrayLength(value,cap,limitCode='shape'){
  if(!Array.isArray(value)||Object.getPrototypeOf(value)!==Array.prototype)fail('shape','Eine vollständige Datenliste ist erforderlich.');
  const d=Object.getOwnPropertyDescriptor(value,'length');
  if(!d||!own(d,'value')||!Number.isSafeInteger(d.value)||d.value<0)fail('shape','Die Datenliste enthält keine gültige Länge.');
  if(d.value>cap)fail(limitCode,'Die vollständige Tabelle überschreitet die Exportgrenze.');
  return d.value;
}
function dense(value,cap,limitCode='shape'){
  const length=arrayLength(value,cap,limitCode);
  if(Reflect.ownKeys(value).length!==length+1)fail('shape','Lücken oder zusätzliche Listenfelder werden nicht exportiert.');
  const output=[];
  for(let index=0;index<length;index++){
    const d=Object.getOwnPropertyDescriptor(value,String(index));
    if(!d||!own(d,'value')||!d.enumerable)fail('shape','Jede Zelle muss als eigener Datenwert vorliegen.');
    output.push(d.value);
  }
  return output;
}
function scalar(value,limits){
  if(value===null)return{value,bytes:0};
  if(typeof value==='boolean')return{value,bytes:value?4:5};
  if(typeof value==='string'){
    if(value.length>limits.maxCellCodeUnits)fail('cell_limit','Eine Zelle überschreitet ihre UTF-16-Zeichengrenze.');
    let bytes=0,lineFeeds=0;
    for(let index=0;index<value.length;index++){
      const code=value.charCodeAt(index);
      if(code>=0xd800&&code<=0xdbff){
        const low=value.charCodeAt(++index);
        if(!(low>=0xdc00&&low<=0xdfff))fail('unicode','Eine Zelle enthält ein ungepaartes UTF-16-Surrogat.');
        bytes+=4;
      }else{
        if(code>=0xdc00&&code<=0xdfff)fail('unicode','Eine Zelle enthält ein ungepaartes UTF-16-Surrogat.');
        bytes+=code<0x80?1:code<0x800?2:3;
        if(code===10&&++lineFeeds>limits.maxCellLineFeeds)fail('cell_limit','Eine Zelle überschreitet die zulässige Zahl der Zeilenumbrüche.');
      }
      if(bytes>limits.maxCellBytes)fail('cell_limit','Eine Zelle überschreitet ihre UTF-8-Bytegrenze.');
    }
    return{value,bytes};
  }
  if(typeof value==='number'){
    const absolute=Math.abs(value);
    if(!Number.isFinite(value)||value!==0&&(absolute<2.2251e-308||absolute>9.99999999999999e307))fail('number','Eine Zahl liegt außerhalb des darstellbaren Excel-Bereichs.');
    const text=String(value),digits=text.replace(/^-/,'').split('e')[0].replace('.','').replace(/^0+/,'').replace(/0+$/,'');
    if(digits.length>15)fail('number','Eine Zahl benötigt mehr als 15 signifikante Dezimalziffern; sie wird nicht gerundet.');
    return{value:Object.is(value,-0)?0:value,bytes:utf8.encode(text).length};
  }
  fail('scalar','Nur Text, ausdrücklich numerische Werte, Wahrheitswerte oder unbekannte Werte sind exportierbar.');
}
function column(value,limits,context){
  const c=record(value,context?['key','label','value']:['key','label']);
  if(typeof c.key!=='string'||!/^[A-Za-z_][A-Za-z0-9_.-]{0,127}$/.test(c.key)||typeof c.label!=='string'||!c.label.length)fail('columns','Jede Spalte benötigt einen eindeutigen Schlüssel und eine vollständige Überschrift.');
  const label=scalar(c.label,limits),checked=context?scalar(c.value,limits):null;
  return{key:c.key,label:label.value,labelBytes:label.bytes,...context?{value:checked.value,valueBytes:checked.bytes}:{}};
}
function escaped(text){
  // Scan the original once. Adjacent escape patterns may share an underscore;
  // generated escape sequences must never be protected or decoded a second time.
  const parts=[];
  for(let index=0;index<text.length;index++){
    const code=text.charCodeAt(index),ch=text[index];
    if(ch==='_'&&/^x[0-9a-fA-F]{4}_/.test(text.slice(index+1,index+7)))parts.push('_x005F_');
    else if(code===13||code<32&&code!==9&&code!==10||code===0xfffe||code===0xffff)parts.push('_x'+code.toString(16).toUpperCase().padStart(4,'0')+'_');
    else if(ch==='&')parts.push('&amp;');
    else if(ch==='<')parts.push('&lt;');
    else if(ch==='>')parts.push('&gt;');
    else parts.push(ch);
  }
  return parts.join('');
}
function address(column){let value='';for(let n=column+1;n;n=Math.floor((n-1)/26))value=String.fromCharCode(65+(n-1)%26)+value;return value;}
class Sink{
  constructor(cap){this.cap=cap;this.length=0;this.parts=[];}
  add(text){const bytes=utf8.encode(text);if(this.length+bytes.length>this.cap)fail('worksheet_limit','Die vollständige Arbeitsblattdatei überschreitet ihre Bytegrenze.');this.parts.push(bytes);this.length+=bytes.length;}
  finish(){const bytes=new Uint8Array(this.length);let at=0;for(const part of this.parts){bytes.set(part,at);at+=part.length;}return bytes;}
}
const crcTable=Uint32Array.from({length:256},(_,number)=>{let value=number;for(let bit=0;bit<8;bit++)value=value&1?0xedb88320^(value>>>1):value>>>1;return value>>>0;});
function crc32(bytes){let crc=0xffffffff;for(const byte of bytes)crc=crcTable[(crc^byte)&255]^(crc>>>8);return(crc^0xffffffff)>>>0;}
function zipStored(parts,cap){
  const entries=parts.map(([name,content])=>({name:utf8.encode(name),data:typeof content==='string'?utf8.encode(content):content}));
  const size=22+entries.reduce((sum,item)=>sum+30+item.name.length+item.data.length+46+item.name.length,0);
  if(size>cap)fail('workbook_limit','Die vollständige XLSX-Datei überschreitet ihre Bytegrenze.');
  const bytes=new Uint8Array(size),view=new DataView(bytes.buffer),central=[];let offset=0;
  const u16=(at,number)=>view.setUint16(at,number,true),u32=(at,number)=>view.setUint32(at,number,true);
  for(const entry of entries){
    const local=offset,crc=crc32(entry.data);u32(offset,0x04034b50);u16(offset+4,20);u16(offset+6,0x800);u16(offset+8,0);u16(offset+10,0);u16(offset+12,33);u32(offset+14,crc);u32(offset+18,entry.data.length);u32(offset+22,entry.data.length);u16(offset+26,entry.name.length);u16(offset+28,0);
    bytes.set(entry.name,offset+30);bytes.set(entry.data,offset+30+entry.name.length);offset+=30+entry.name.length+entry.data.length;central.push({...entry,local,crc});
  }
  const start=offset;
  for(const entry of central){
    u32(offset,0x02014b50);u16(offset+4,20);u16(offset+6,20);u16(offset+8,0x800);u16(offset+10,0);u16(offset+12,0);u16(offset+14,33);u32(offset+16,entry.crc);u32(offset+20,entry.data.length);u32(offset+24,entry.data.length);u16(offset+28,entry.name.length);u16(offset+30,0);u16(offset+32,0);u16(offset+34,0);u16(offset+36,0);u32(offset+38,0);u32(offset+42,entry.local);
    bytes.set(entry.name,offset+46);offset+=46+entry.name.length;
  }
  u32(offset,0x06054b50);u16(offset+4,0);u16(offset+6,0);u16(offset+8,entries.length);u16(offset+10,entries.length);u32(offset+12,offset-start);u32(offset+16,start);u16(offset+20,0);
  return bytes;
}
function create(overrides={}){
  const limits={...LIMITS};
  if(!overrides||typeof overrides!=='object'||![Object.prototype,null].includes(Object.getPrototypeOf(overrides)))throw Error('Die XLSX-Grenzen müssen eigene Datenfelder sein.');
  for(const key of Reflect.ownKeys(overrides)){
    const d=Object.getOwnPropertyDescriptor(overrides,key);
    if(typeof key!=='string'||!own(LIMITS,key)||!d||!own(d,'value')||!d.enumerable||!Number.isSafeInteger(d.value)||d.value<(key==='maxRows'||key==='maxCellLineFeeds'?0:1)||d.value>LIMITS[key])throw Error('XLSX-Grenzen dürfen nur bekannte Grenzen absenken.');
    limits[key]=d.value;
  }
  Object.freeze(limits);const issued=new WeakSet();
  function prepareTable(input){return result(()=>{
    const data=record(input,['columns','rows','context']),baseLength=arrayLength(data.columns,limits.maxColumns,'columns_limit'),contextLength=arrayLength(data.context,limits.maxColumns,'columns_limit'),columnCount=baseLength+contextLength,rowCount=arrayLength(data.rows,limits.maxRows,'rows_limit');
    if(!baseLength)fail('columns','Mindestens eine Datenspalte ist erforderlich.');
    if(columnCount>limits.maxColumns)fail('columns_limit','Daten- und Herkunftsspalten überschreiten gemeinsam die Spaltengrenze.');
    const cellSlots=(rowCount+1)*columnCount;
    if(cellSlots>limits.maxCells)fail('cells_limit','Die Zellgrenze zählt Überschriften, unbekannte Werte und Herkunftsspalten mit.');
    let inputBytes=0,payloadBytes=0,unknownCells=0;
    function inputSize(bytes){inputBytes+=bytes;if(inputBytes>limits.maxPayloadBytes)fail('payload_limit','Die vollständigen Quelldaten überschreiten die Nutzdaten-Bytegrenze.');}
    function payloadSize(bytes){payloadBytes+=bytes;if(payloadBytes>limits.maxPayloadBytes)fail('payload_limit','Die vollständigen ausgegebenen Werte überschreiten die Nutzdaten-Bytegrenze.');}
    const base=dense(data.columns,limits.maxColumns).map(value=>{const c=column(value,limits,false);inputSize(c.labelBytes);payloadSize(c.labelBytes);return c;});
    const context=dense(data.context,limits.maxColumns).map(value=>{const c=column(value,limits,true);inputSize(c.labelBytes+c.valueBytes);payloadSize(c.labelBytes);return c;});
    if(base.length!==baseLength||context.length!==contextLength)fail('shape','Die Spaltenliste änderte sich während der Vorbereitung.');
    const columns=[...base,...context].map(c=>Object.freeze({key:c.key,label:c.label}));
    if(new Set(columns.map(c=>c.key)).size!==columnCount||new Set(columns.map(c=>c.label)).size!==columnCount)fail('columns','Schlüssel oder physische Überschriften kommen mehrfach vor.');
    const originals=dense(data.rows,limits.maxRows);
    if(originals.length!==rowCount)fail('shape','Die Zeilenliste änderte sich während der Vorbereitung.');
    const rows=originals.map(row=>{
      const cells=dense(row,baseLength);
      if(cells.length!==baseLength)fail('shape','Jede Zeile muss vollständig zur gewählten Spaltenprojektion passen.');
      const output=cells.map(value=>{const checked=scalar(value,limits);inputSize(checked.bytes);payloadSize(checked.bytes);if(checked.value===null)unknownCells++;return checked.value;});
      for(const c of context){payloadSize(c.valueBytes);if(c.value===null)unknownCells++;output.push(c.value);}
      return Object.freeze(output);
    });
    const snapshot=Object.freeze({columns:Object.freeze(columns),rows:Object.freeze(rows),stats:Object.freeze({rowCount,columnCount,cellSlots,payloadBytes,inputBytes,unknownCells})});
    issued.add(snapshot);return snapshot;
  });}
  function encode(snapshot){return result(()=>{
    if(!snapshot||typeof snapshot!=='object'||!issued.has(snapshot))fail('snapshot','Nur eine unveränderliche Vorschau dieses XLSX-Planers kann exportiert werden.');
    const sheet=new Sink(limits.maxWorksheetBytes);
    sheet.add('<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="'+NS+'"><dimension ref="A1:'+address(snapshot.columns.length-1)+(snapshot.rows.length+1)+'"/><sheetData>');
    function row(values,index){
      sheet.add('<row r="'+index+'">');
      values.forEach((value,column)=>{
        const ref=address(column)+index;
        if(value===null)sheet.add('<c r="'+ref+'"/>');
        else if(typeof value==='string')sheet.add('<c r="'+ref+'" t="inlineStr"><is><t xml:space="preserve">'+escaped(value)+'</t></is></c>');
        else sheet.add('<c r="'+ref+'" t="'+(typeof value==='boolean'?'b':'n')+'"><v>'+(typeof value==='boolean'?(value?1:0):String(value))+'</v></c>');
      });
      sheet.add('</row>');
    }
    row(snapshot.columns.map(c=>c.label),1);snapshot.rows.forEach((values,index)=>row(values,index+2));sheet.add('</sheetData></worksheet>');
    const worksheetBytes=sheet.length;
    const parts=[
      ['[Content_Types].xml','<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>'],
      ['_rels/.rels','<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="'+REL+'/officeDocument" Target="xl/workbook.xml"/></Relationships>'],
      ['xl/workbook.xml','<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="'+NS+'" xmlns:r="'+REL+'"><sheets><sheet name="Ergebnisse" sheetId="1" r:id="rId1"/></sheets></workbook>'],
      ['xl/_rels/workbook.xml.rels','<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="'+REL+'/worksheet" Target="worksheets/sheet1.xml"/></Relationships>'],
      ['xl/worksheets/sheet1.xml',sheet.finish()]
    ];
    return Object.freeze({bytes:zipStored(parts,limits.maxWorkbookBytes),worksheetBytes,rows:snapshot.rows.length,columns:snapshot.columns.length});
  });}
  return Object.freeze({prepareTable,encode,limits});
}
return Object.freeze({create,LIMITS,MIME});
});
