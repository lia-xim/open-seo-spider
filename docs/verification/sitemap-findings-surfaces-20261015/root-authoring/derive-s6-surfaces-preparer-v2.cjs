const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex'),insist=(c,m)=>{if(!c)throw new Error(m);};
const source=path.join(__dirname,'prepare-sitemap-findings-surfaces-v1.cjs'),target=path.join(__dirname,'prepare-sitemap-findings-surfaces-v2.cjs'),receiptPath=path.join(__dirname,'derive-s6-surfaces-preparer-v2.json');
insist(!fs.existsSync(target)&&!fs.existsSync(receiptPath),'fresh derivative outputs required');
const raw=fs.readFileSync(source);insist(sha(raw)==='a6f69c237ab6d62ec5cc3d348e710004b1422975ee264b630ca6dd75a4c8c634','reviewed original preparer changed');let text=raw.toString();
function replace(before,after){insist(text.split(before).length===2,'derivation anchor changed');text=text.replace(before,after);}
replace("const cliPostfixPin=process.argv[2];insist(/^[a-f0-9]{64}$/.test(cliPostfixPin??''),'Supply exact reviewed immutable CLI postfix manifest SHA');",`const cliPostfixPin='ca05854c67af70e45b2c0bc92bf0fda0c6379656c20bbcea1b26eaae828b825c';
const cliReviewPath=path.join(base,'s6-cli-mcp-postfix-review-v1','receipt.json');
const cliReviewPin='f4b98a23dc14f6b75bf6c3b9a2c735bce9e6fa0b084d4c3893eb0ffe0b4ab29f';
const receiptPath=path.join(__dirname,'prepare-sitemap-findings-surfaces-v2.json');
insist(process.argv.length===2,'No caller-supplied authorization pin is accepted');
insist(!fs.existsSync(receiptPath),'fresh preparation receipt required before any source output');
insist(sha(fs.readFileSync(cliReviewPath))===cliReviewPin,'independent CLI postfix approval receipt changed');`);
replace('required_test_names:requiredCount,lineage,approved_product_bindings:','required_test_names:requiredCount,lineage,cli_postfix_review:{path:cliReviewPath,sha256:cliReviewPin,scope:"INDEPENDENT_BOUNDED_SOURCE_CLOSURE_NOT_RUNTIME_ACCEPTANCE"},approved_product_bindings:');
replace("fs.writeFileSync(path.join(__dirname,'prepare-sitemap-findings-surfaces-v1.json'),JSON.stringify(result,null,2)+'\\n',{flag:'wx'});","fs.writeFileSync(receiptPath,JSON.stringify(result,null,2)+'\\n',{flag:'wx'});");
const bytes=Buffer.from(text);fs.writeFileSync(target,bytes,{flag:'wx'});const result={status:'PREPARER_SOURCE_POSTFIX_AUTHORED_NOT_EXECUTED',original_sha256:sha(raw),target,sha256:sha(bytes),fixed_cli_manifest_sha256:'ca05854c67af70e45b2c0bc92bf0fda0c6379656c20bbcea1b26eaae828b825c',independent_cli_closure_sha256:'f4b98a23dc14f6b75bf6c3b9a2c735bce9e6fa0b084d4c3893eb0ffe0b4ab29f',original_changed:false,source_freeze_executed:false};fs.writeFileSync(receiptPath,JSON.stringify(result,null,2)+'\n',{flag:'wx'});process.stdout.write(JSON.stringify(result)+'\n');
