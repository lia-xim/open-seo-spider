const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{spawn}=require('node:child_process'),{createHash}=require('node:crypto');
const root=path.resolve(__dirname,'..'),out=path.join(__dirname,'extraction-source-pure-20261021-r1');assert(!fs.existsSync(out));fs.mkdirSync(out);
const yaml=fs.readFileSync(path.join(root,'.github/workflows/ci.yml'),'utf8');
const selected=[...yaml.matchAll(/^\s*(?:-\s*)?run:\s*node --test (web\/verification\/[^\r\n]+)$/gm)].map(match=>match[1].trim().split(/\s+/)).filter(files=>files.every(file=>!file.includes('browser')));assert(selected.some(files=>files.includes('web/verification/table-source-availability.test.cjs')));
const hash=file=>createHash('sha256').update(fs.readFileSync(path.join(root,file))).digest('hex'),sources=[...new Set(['web/app.js','.github/workflows/ci.yml','scripts/check-ui-fixtures.mjs',...selected.flat(),...fs.readdirSync(path.join(root,'web')).filter(name=>name.endsWith('.js')).map(name=>'web/'+name)])].map(file=>({file,sha256:hash(file)}));
const results=[];let failure;
(async()=>{try{for(let index=0;index<selected.length;index++){
 const files=selected[index],start=Date.now(),child=spawn(process.execPath,['--test',...files],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});let stdout='',stderr='',launchError;child.stdout.on('data',bytes=>stdout+=bytes.toString('utf8'));child.stderr.on('data',bytes=>stderr+=bytes.toString('utf8'));child.once('error',error=>launchError=String(error));
 const status=await new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));fs.writeFileSync(path.join(out,index+'.stdout.log'),stdout);fs.writeFileSync(path.join(out,index+'.stderr.log'),stderr);
 const count=name=>{const match=stdout.match(new RegExp('^(?:#|ℹ) '+name+' (\\d+)','m'));assert(match,'Missing '+name+' for '+files.join(','));return Number(match[1]);};
 const record={files,...status,launchError,duration_ms:Date.now()-start,tests:count('tests'),passed:count('pass'),failed:count('fail')};results.push(record);assert.equal(status.code,0,files.join(','));assert.equal(record.failed,0);assert.equal(record.tests,record.passed);
}}catch(error){failure=String(error.stack);}finally{
 const changed=sources.filter(pin=>hash(pin.file)!==pin.sha256);const proof={kind:'fresh_selected_CI_Node_tests_no_browser',created_at:new Date().toISOString(),sources,results,failure,changed,passed:results.reduce((n,row)=>n+row.passed,0),complete:!failure&&changed.length===0&&results.length===selected.length};fs.writeFileSync(path.join(out,'proof.json'),JSON.stringify(proof,null,2)+'\n');console.log(JSON.stringify({complete:proof.complete,passed:proof.passed,failure:proof.failure,changed:proof.changed}));if(!proof.complete)process.exitCode=1;
}})();
