'use strict';
// Package-only preparation check; no Chromium/native runtime or product imports.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const {packageParts,crc32}=require('./xlsx-package-oracle.cjs');
assert.equal(crc32(Buffer.from('123456789')),0xcbf43926,'Published standard CRC32 check vector');
const root=path.resolve(__dirname,'../..'),source=path.join(root,'artifacts/table-xlsx-product-pure-20261021-r1/r2');
const results=[];
for(const name of['edge-scalars','headers-only','wide-ordered']){
  const file=path.join(source,name+'.xlsx'),bytes=fs.readFileSync(file),decoded=packageParts(bytes);
  assert.equal(decoded.entries.length,5);assert.equal(Object.keys(decoded.xml).length,5);
  const negative=Buffer.from(bytes),at=negative.indexOf(Buffer.from('<worksheet'));assert(at>0);negative[at+1]^=1;
  assert.throws(()=>packageParts(negative),/CRC/,'Declared corrupt copy is a reader-only negative control');
  assert.throws(()=>packageParts(Buffer.concat([bytes,Buffer.from([0])])),/EOCD/,'Trailing package bytes are rejected');
  assert.throws(()=>packageParts(bytes,{maxBytes:bytes.length-1}),/byte bound/,'Reader byte cap applies to the actual entire file');
  results.push({source:file,bytes:bytes.length,sha256:decoded.sha256,parts:decoded.entries.map(({name,bytes})=>({name,bytes})),crc_negative:true,trailing_bytes_negative:true,reader_bound_negative:true});
}
// Exercise the exact candidate decoder in isolation; loading the complete driver
// would start native/browser work, so only its self-contained declaration runs.
const driver=fs.readFileSync(path.join(__dirname,'ui-table-xlsx-live.cjs'),'utf8'),start=driver.indexOf('function decodedCSV('),end=driver.indexOf('\nasync function csv(',start);assert(start>0&&end>start);
const csvRecords=require(path.join(root,'web/verification/csv-records.cjs'));
const decode=vm.runInNewContext('('+driver.slice(start,end)+')',{assert,Buffer,csvRecords});
const header=['URL','Überschrift "wörtlich"'],wire='\uFEFF"URL","Überschrift ""wörtlich"""';
assert.deepEqual(decode(Buffer.from(wire),header),[],'Actual header-only form without trailingCRLF keeps its final quote');
assert.deepEqual(decode(Buffer.from(wire+'\r\n'),header),[],'Explicit header-only CRLF remains equivalent');
assert.throws(()=>decode(Buffer.from(wire.slice(0,-1)),header),/Exact ordered CSV header/,'Truncated header is not accepted');
assert.throws(()=>decode(Buffer.from(wire),header.slice().reverse()),/Exact ordered CSV header/,'Column order remains exact');
// Calibrate only independently authored source-record mappers on genuine saved
// R2 reports; these archived files are not substituted for a new native capture.
const mapperSource=driver.slice(driver.indexOf('const titleRows='),driver.indexOf('function rendered('))+driver.slice(driver.indexOf('function allTitles('),driver.indexOf('async function configureAll('));
const mappers=vm.runInNewContext(mapperSource+'\n({titleRows,statusRows,allTitles,selectedTitles,allResponses,responseTwins})',{assert,collator:new Intl.Collator('de-DE',{numeric:true,sensitivity:'base'})});
const oldCaptureFolder=path.join(root,'docs/verification/table-view-portability-20261019/native-r2'),calibration=[];
for(const name of fs.readdirSync(oldCaptureFolder).filter(name=>name.endsWith('-original-project.json')).sort()){
  const source=path.join(oldCaptureFolder,name),job={report:JSON.parse(fs.readFileSync(source))},titles=mappers.allTitles(job),selected=mappers.selectedTitles(job),responses=mappers.allResponses(job),twins=mappers.responseTwins(job);
  assert.equal(job.report.pages.length,153);assert.equal(titles.length,152);assert.equal(selected.length,118);assert.equal(responses.length,153);assert.equal(twins.length,2);assert.equal(titles.find(row=>row.url.endsWith('/items/0000')).title_pixels,0);assert.equal(titles.find(row=>row.url.endsWith('/items/0001')).title_text,null);
  calibration.push({source,pages:153,titles:152,selected:118,responses:153,original_response_twins:twins.map(({url,requested_url,response_index,status})=>({url,requested_url,response_index,status}))});
}
assert.equal(calibration.length,2);
const proof={result:'PASS_PACKAGE_CSV_AND_SOURCE_MAPPER_PREPARATION',reader:'Independent tableless CRC32 + bounded Node ZIP decoder; no product writer import',files:results,csv_header_only:{no_tail:true,CRLF_tail:true,truncated_rejected:true,reordered_rejected:true},archived_mapper_calibration:calibration,not_proven:['Fresh Chromium DOMParser evaluator is syntax checked only.','No actual native or browser acceptance run was executed by this preparation task.','Pre-existing files were created by the core pure proof; they do not establish native download oracles.','Archived R2 record mapper calibration is not a new native fixture capture or changed literal-string proof.']};
fs.writeFileSync(path.join(__dirname,'offline-package-check.json'),JSON.stringify(proof,null,2)+'\n');console.log(JSON.stringify(proof,null,2));
