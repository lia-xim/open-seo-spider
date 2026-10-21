'use strict';
// Authoring-only exact replacements. No files outside the assigned App path.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const file=path.resolve(__dirname,'../../web/app.js');let source=fs.readFileSync(file,'utf8');
function replace(before,after){assert.equal(source.split(before).length-1,1,before);source=source.replace(before,after);}
replace("  const urlColumn = textColumn(","  const booleanExportValue = (row,key) => !row._unavailable&&typeof row[key]==='boolean'?row[key]:null;\n  const booleanColumn = (key,label,getter) => ({...textColumn(key,label,getter),exportValue:row=>booleanExportValue(row,key)});\n  const urlColumn = textColumn(");
replace("exportValue: row => typeof row.indexable==='boolean'?row.indexable:null","exportValue: row => booleanExportValue(row,'indexable')");
for(const [key,label]of [['nofollow','Nofollow'],['evidence_complete','Beispiele vollständig'],['body_complete','Body vollständig'],['document_metadata_available','HTML-Metadaten verfügbar'],['hidden_attribute','hidden-Attribut'],['eligible','Deklaration akzeptiert'],['target_complete','Ziel vollständig'],['text_complete','Text vollständig'],['annotations_complete','Annotationen vollständig'],['truncated','Abgeschnitten'],['httpOnly','HttpOnly'],['secure','Secure']]){
  // body_complete occurs in both response and image-file schemas.
  const expression=new RegExp("textColumn\\('"+key+"',\\s*'"+label+"'",'g'),matches=source.match(expression);assert.equal(matches?.length,key==='body_complete'?2:1,key);
  source=source.replace(expression,match=>match.replace('textColumn','booleanColumn'));
}
replace("textColumn('headers_known', 'Header erfasst', row => row.headers == null ? 'Unbekannt' : 'Ja')","{...textColumn('headers_known', 'Header erfasst', row => row.headers == null ? 'Unbekannt' : 'Ja'),exportValue:row=>!row._unavailable&&row.headers&&typeof row.headers==='object'&&!Array.isArray(row.headers)?true:null}");
replace("textColumn('self_reference', 'Selbstreferenz', (row) => row.canonical.length ? row.canonical.includes(row.rendered && !row.renderer_error ? row.rendered_url || row.final_url || row.url : row.final_url || row.url) ? 'Ja' : 'Nein' : '—')","{...textColumn('self_reference', 'Selbstreferenz', (row) => row.canonical.length ? row.canonical.includes(row.rendered && !row.renderer_error ? row.rendered_url || row.final_url || row.url : row.final_url || row.url) ? 'Ja' : 'Nein' : '—'),exportValue:row=>!row._unavailable&&Array.isArray(row.canonical)&&row.canonical.length&&row.canonical.every(value=>typeof value==='string')?row.canonical.includes(row.rendered&&!row.renderer_error?row.rendered_url||row.final_url||row.url:row.final_url||row.url):null}");
fs.writeFileSync(file,source);console.log('Exact Boolean export-only replacements applied.');
