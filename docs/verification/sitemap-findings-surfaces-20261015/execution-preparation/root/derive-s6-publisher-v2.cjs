const fs=require('node:fs'),path=require('node:path'),{createHash}=require('node:crypto');
const input=path.join(__dirname,'publish-s6-surfaces-source-docs-v1.cjs'),output=path.join(__dirname,'publish-s6-surfaces-source-docs-v2.cjs');
const sha=b=>createHash('sha256').update(b).digest('hex'),insist=(c,m)=>{if(!c)throw Error(m);};
const raw=fs.readFileSync(input);let code=raw.toString();
insist(sha(raw)==='90e65dbf6610f595fa6ca7be4243691df49e13ecc42088c7b614999a0698c8e9','reviewed publisher source changed');
insist(!fs.existsSync(output),'fresh output required');
function replace(before,after){insist(code.split(before).length===2,'exact source edit required');code=code.replace(before,after);}
replace("'publish-s6-surfaces-source-docs-v1.json'","'publish-s6-surfaces-source-docs-v2.json'");
replace("const items=[];",`const items=[];
const payloadChecks=[];
function verifyLocalPayload(dir,entry){
 const relative=entry.payload??entry.path,pin=entry.after_sha256??entry.sha256;
 insist(typeof relative==='string'&&!path.isAbsolute(relative),'local declared payload path required');
 insist(typeof pin==='string'&&/^[a-f0-9]{64}$/.test(pin),'declared payload SHA required');
 const absolute=path.resolve(dir,relative),inside=path.relative(dir,absolute);
 insist(inside&&!inside.startsWith('..')&&!path.isAbsolute(inside),'declared payload must stay inside handoff');
 const bytes=fs.readFileSync(absolute);insist(sha(bytes)===pin,'declared handoff payload changed '+absolute);
 if(entry.bytes!==undefined)insist(bytes.length===entry.bytes,'declared payload byte length changed '+absolute);
 payloadChecks.push({path:absolute,bytes:bytes.length,sha256:pin});
}
function verifyLineagePayloads(dir,manifest){
 for(const field of ['entries','files','support_files','authoring_companions'])for(const entry of manifest[field]??[])verifyLocalPayload(dir,entry);
 for(const [field,name] of [['source_pins_sha256','SOURCE-PINS.json'],['handoff_sha256','HANDOFF.md']])if(manifest[field])verifyLocalPayload(dir,{path:name,sha256:manifest[field]});
}
`);
replace("insist(sha(fs.readFileSync(path.join(dir,anchor)))===pin,'archive anchor changed '+name);walk(dir,'',name);", "const anchorBytes=fs.readFileSync(path.join(dir,anchor));insist(sha(anchorBytes)===pin,'archive anchor changed '+name);if(anchor==='manifest.json')verifyLineagePayloads(dir,JSON.parse(anchorBytes));walk(dir,'',name);");
replace("archive_note:'Exact source-only authoring/review bytes, including superseded inputs.","declared_lineage_payload_checks:payloadChecks,archive_note:'Declared local source handoff payloads/support bytes rechecked against each pinned manifest before archive. Additional raw companions captured with their own SHA; external historical references are not replayed. Exact source-only authoring/review bytes, including superseded inputs.");
fs.writeFileSync(output,code,{flag:'wx'});
process.stdout.write(JSON.stringify({input_sha256:sha(raw),output_sha256:sha(Buffer.from(code)),output,executed_publisher:false})+'\n');
