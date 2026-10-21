const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{spawn}=require('node:child_process'),{createHash}=require('node:crypto');
const root=path.resolve(__dirname,'..'),build=JSON.parse(fs.readFileSync(path.join(__dirname,'table-xlsx-build-gates-20261021-r1/proof.json')));assert.equal(build.complete,true,'Wait for all owned Cargo gates and frozen-input check');
const hash=file=>createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const draft=path.join(__dirname,'table-xlsx-native-draft-20261021');
for(const[file,pin]of [['ui-table-xlsx-live.cjs','1bf7dbb2ea1a7b1c40355880ba1dd2a2f2815462e36fcbc17cb62f848ed7e050'],['xlsx-package-oracle.cjs','4757d8804b61004c150ebee570edbc7f5009abe3055e5c5016f5fe5136502729']]){
 const source=path.join(draft,file);assert.equal(hash(source),pin);fs.copyFileSync(source,path.join(root,'web/verification',file),fs.constants.COPYFILE_EXCL);
}
const registry=path.join(root,'scripts/ui-e2e.mjs'),old=fs.readFileSync(registry,'utf8'),nl=old.includes('\r\n')?'\r\n':'\n';
const needle="  ['table-view-portability', 'ui-table-view-portability-live.cjs', 'owned-native'],";assert.equal(old.split(needle).length,2);fs.writeFileSync(registry,old.replace(needle,needle+nl+"  ['table-xlsx', 'ui-table-xlsx-live.cjs', 'owned-native'],"));
const out=path.join(__dirname,'table-xlsx-native-run-20261021-r1');assert(!fs.existsSync(out));fs.mkdirSync(out);
const binary=build.binary.path;assert.equal(hash(binary),build.binary.sha256);const chrome=build.environment.OPEN_SEO_BROWSER_PATH;
assert.equal(hash(chrome),'36a30716f0994b377f1ce6b47cfe35399b9719e31d597fe0ea0d8f3b62aed532');
const env={...process.env,SPIDER_ROOT:root,SPIDER_BINARY:binary,SPIDER_BINARY_SHA256:build.binary.sha256,OPEN_SEO_BROWSER_PATH:chrome,OPEN_SEO_COORDINATED_EXECUTION_WINDOW:'confirmed',SPIDER_TABLE_XLSX_OUTPUT:path.join(out,'native')};
const stdout=fs.createWriteStream(path.join(out,'stdout.log')),stderr=fs.createWriteStream(path.join(out,'stderr.log')),started=Date.now();
const child=spawn(process.execPath,[path.join(root,'web/verification/ui-table-xlsx-live.cjs')],{cwd:root,env,windowsHide:true,stdio:['ignore','pipe','pipe']});child.stdout.pipe(stdout);child.stderr.pipe(stderr);let timedOut=false,launchError;child.once('error',error=>launchError=String(error));
const timer=setTimeout(()=>{timedOut=true;child.kill();},240000);
child.once('close',async(code,signal)=>{clearTimeout(timer);await Promise.all([new Promise(resolve=>stdout.closed?resolve():stdout.once('close',resolve)),new Promise(resolve=>stderr.closed?resolve():stderr.once('close',resolve))]);const proof={kind:'actual_table_xlsx_native_driver_invocation',code,signal,timedOut,launchError,duration_ms:Date.now()-started,timeout_ms:240000,binary:build.binary,driver_sha256:hash(path.join(root,'web/verification/ui-table-xlsx-live.cjs')),oracle_sha256:hash(path.join(root,'web/verification/xlsx-package-oracle.cjs')),complete:code===0&&!timedOut&&!launchError};fs.writeFileSync(path.join(out,'invocation.json'),JSON.stringify(proof,null,2)+'\n');console.log(JSON.stringify(proof));if(!proof.complete)process.exitCode=1;});
