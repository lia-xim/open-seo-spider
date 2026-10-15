const fs=require('node:fs'),path=require('node:path'),{createHash}=require('node:crypto');
const sha=b=>createHash('sha256').update(b).digest('hex'),insist=(c,m)=>{if(!c)throw Error(m);};
const before=path.join(__dirname,'publish-s6-surfaces-source-docs-v2.cjs'),after=path.join(__dirname,'publish-s6-surfaces-source-docs-v3.cjs'),receipt=path.join(__dirname,'derive-s6-publisher-v3.json');
const raw=fs.readFileSync(before);insist(sha(raw)==='c7966c5f8cd45e3dc0ef6d6698af9024ed299ab1c1f5bedf532d0cf87d838c53','v2 changed');
const prior=fs.readFileSync(path.join(__dirname,'sitemap-findings-core-published-v1.json'));insist(sha(prior)==='0473dd85631cc83c75819232c2a1ac2a8f8aa92201f1ab91e2d5b7e145d6f0a1','prior accepted receipt changed');
const accepted=JSON.parse(prior).installed_binary_sha256_actual;insist(accepted==='ab1ac93c82905aa627d100d8622a50acf93dd7fe59c3f854eecdaac667110bcf','accepted binary reference changed');
insist(!fs.existsSync(after)&&!fs.existsSync(receipt),'fresh v3 outputs required');
let code=raw.toString();function replace(a,b){insist(code.split(a).length===2,'exact replacement required');code=code.replace(a,b);}
replace("'publish-s6-surfaces-source-docs-v2.json'","'publish-s6-surfaces-source-docs-v3.json'");
replace('ab1ac93c82905aa627d100d8622a50acf93dd7fe59c3f854eecdaac667110bcf8f',accepted);
replace("work.gates.source_filter_s6_surfaces=",`let historical=0;work.pending_acceptance_de=work.pending_acceptance_de.map(note=>{if(!note.startsWith('S6-Flächenprüfung:'))return note;historical++;return 'Vorherige S6-Flächenplanung: damalige Lücken für alle sieben Sitemap-Filter festgehalten. Gemeinsamer Abfrage-/Exportkern, getrennte Seiten-/Quellversuch-Domänen und API-/CLI-/MCP-/Browserflächen sind inzwischen als ungetesteter Quellcode verbunden; die aktuelle Flächenabnahme bleibt offen. Filter auf den ersten 200 geladenen Zeilen ersetzen diese Abnahmen nicht.';});insist(historical===1,'historical S6 note missing/duplicated');
work.gates.source_filter_s6_surfaces=`);
fs.writeFileSync(after,code,{flag:'wx'});
const result={schema_version:1,result:'SOURCE_ONLY_PUBLISHER_REFERENCE_CORRECTION',input_sha256:sha(raw),output_sha256:sha(Buffer.from(code)),previous_accepted_receipt_sha256:sha(prior),accepted_binary_sha256:accepted,v2_execution:'FAILED_BEFORE_ANY_ARCHIVE_OR_DOCUMENT_WRITE_INVALID_66_CHARACTER_EXPECTED_SHA',source_frozen_297_unchanged:true,publisher_v3_executed:false};
fs.writeFileSync(receipt,JSON.stringify(result,null,2)+'\n',{flag:'wx'});process.stdout.write(JSON.stringify(result)+'\n');
