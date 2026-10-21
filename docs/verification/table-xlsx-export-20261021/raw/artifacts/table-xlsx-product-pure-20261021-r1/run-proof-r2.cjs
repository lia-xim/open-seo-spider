'use strict';
// Authoring-only evidence. This runner does not launch browser/native/Cargo or Git.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{spawnSync}=require('node:child_process');
const root=path.resolve(__dirname,'../..'),writerPath=path.join(root,'web/table-xlsx.js'),testPath=path.join(root,'web/verification/table-xlsx.test.cjs');
const readerPath=path.join(root,'artifacts/table-xlsx-export-draft-20261019/inspect_xlsx.py');
const python='C:/Users/matth/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe';
const Xlsx=require(writerPath),hash=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
const outDir=path.join(__dirname,'r2');fs.mkdirSync(outDir,{recursive:true});
function pin(file){const bytes=fs.readFileSync(file);return{path:path.relative(root,file).replaceAll('\\','/'),bytes:bytes.length,sha256:hash(bytes)};}
function run(name,executable,args){
  const out=spawnSync(executable,args,{cwd:root,encoding:'utf8',env:{...process.env,PYTHONIOENCODING:'utf-8'},timeout:60000,maxBuffer:8*1024*1024});
  fs.writeFileSync(path.join(outDir,name+'.stdout.log'),out.stdout??'');fs.writeFileSync(path.join(outDir,name+'.stderr.log'),out.stderr??'');
  if(out.error||out.status!==0)throw Error(name+' failed: '+(out.error?.message??out.stderr??out.stdout));return out.stdout;
}
const proof={kind:'pure_product_XLSX_actual_byte_controls',created_at:new Date().toISOString(),result:'RUNNING',node:process.version,
  sources:[pin(writerPath),pin(testPath),pin(readerPath),pin(__filename)],limits:Xlsx.LIMITS,
  primary_spec_sources:[
    'https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/d34ae755-c53f-4a44-a363-c6dd3ee018a4',
    'https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.cell?view=openxml-3.0.1',
    'https://support.microsoft.com/en-us/excel/excel-specifications-and-limits',
    'https://www.w3.org/TR/xml/'],
  boundary:'Pure synchronous formatcore only; no App adapter, browser download, native workflow, Excel GUI roundtrip or schema certification.',
  historical_prototype:'artifacts/table-xlsx-export-draft-20261019/r1.proof.json remains FAIL and immutable. Product fixes overlapping literal Xstring escaping.',cases:[]};
try{
  const test=run('node-tests',process.execPath,['--test','--test-reporter=tap',testPath]);
  const counts=Object.fromEntries([...test.matchAll(/^# (tests|pass|fail|skipped|todo) (\d+)$/gm)].map(match=>[match[1],Number(match[2])]));
  if(counts.tests!==27||counts.pass!==27||counts.fail!==0)throw Error('Unexpected Node test population');proof.tests=counts;
  const controls=Array.from({length:32},(_,i)=>String.fromCharCode(i)).join('')+'\ufffe\uffff';
  const edgeRows=[
    [null,'',0,false],['=SUM(A1:A2)','+1',-123.5,true],['-1','@A1','0000123','9007199254740992'],
    ['_x005F_x0041_','_x0041_x0042_','_x005f_x000d_','_X0041_'],
    [controls,'CR\rLF\nTAB\tend','\r_x000D_\r','&lt;&< > "quoted",'],
    ['\ud83d\ude00','é','e\u0301',' leading\ttrailing '],
    [123456789012345,1234567890123450,0.123456789012345,1e20],
    [9.99999999999999e307,2.2251e-308,-2.2251e-308,-0]
  ];
  const declared=[
    {name:'edge-scalars',data:{columns:['First','Second','Third','Fourth'].map((label,i)=>({key:'c'+i,label})),rows:edgeRows,context:[{key:'owner',label:'Owner _x005F_x0041_\r',value:'local\u0000owner'},{key:'unknown',label:'Unknown context',value:null}]}},
    {name:'headers-only',data:{columns:[{key:'a',label:'Text'}],rows:[],context:[{key:'c',label:'Context',value:'unused'}]}},
    {name:'wide-ordered',data:{columns:Array.from({length:28},(_,i)=>({key:'c'+i,label:'Column '+i})),rows:Array.from({length:253},(_,row)=>Array.from({length:28},(_,column)=>column===0?row:column%4===0?null:column%4===1?'r'+row+'c'+column:column%4===2?row%2===0:row+column/10)),context:[{key:'owner',label:'Owner',value:'ordered'}]}}
  ];
  for(const item of declared){
    // Expected matrix derives only from declared test inputs, not writer snapshots.
    const expected={headers:[...item.data.columns,...item.data.context].map(column=>column.label),rows:item.data.rows.map(row=>[...row,...item.data.context.map(column=>column.value)])};
    const planner=Xlsx.create(),prepared=planner.prepareTable(item.data);if(!prepared.ok)throw Error(JSON.stringify(prepared.error));
    const encoded=planner.encode(prepared.value);if(!encoded.ok)throw Error(JSON.stringify(encoded.error));
    const file=path.join(outDir,item.name+'.xlsx'),expectedPath=path.join(outDir,item.name+'.expected.json');
    fs.writeFileSync(file,encoded.value.bytes);fs.writeFileSync(expectedPath,JSON.stringify(expected,null,2)+'\n');
    const read=JSON.parse(run(item.name+'-reader',python,[readerPath,file,expectedPath]));if(read.result!=='PASS')throw Error('Reader did not pass');
    fs.writeFileSync(path.join(outDir,item.name+'.reader.json'),JSON.stringify(read,null,2)+'\n');proof.cases.push({name:item.name,output:pin(file),expected:pin(expectedPath),reader:pin(path.join(outDir,item.name+'.reader.json')),rows:read.rows,decoded_cells:read.decoded_cells,reader_differences:read.openpyxl.actual_reader_differences.length});
  }
  const corrupt=Buffer.from(fs.readFileSync(path.join(outDir,'edge-scalars.xlsx')));corrupt[100]^=1;const bad=path.join(outDir,'bad-crc.xlsx');fs.writeFileSync(bad,corrupt);
  proof.negative=JSON.parse(run('bad-crc-reader',python,[readerPath,bad,'--reject']));if(proof.negative.result!=='REJECTED_BAD_ZIP')throw Error('Bad ZIP was not rejected');
  proof.result='PASS_PURE_FORMAT_CONTROLS';
}catch(error){proof.result='FAIL';proof.error=error.message;process.exitCode=1;}
proof.finished_at=new Date().toISOString();fs.writeFileSync(path.join(outDir,'proof.json'),JSON.stringify(proof,null,2)+'\n');console.log(JSON.stringify(proof,null,2));
