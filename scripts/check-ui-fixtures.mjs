import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const args=process.argv.slice(2);
assert(args.length===0||(args.length===2&&args[0]==='--root'),'Usage: node scripts/check-ui-fixtures.mjs [--root <checkout>]');
const root=args.length?path.resolve(args[1]):fileURLToPath(new URL('../',import.meta.url));
const specs=[
  ['content','ui-content-reporting.cjs',['web/app.js','web/index.html'],8],
  ['sitemap','ui-sitemap-bundle-reporting.cjs',['web/app.js','web/index.html'],15],
  ['tls','ui-tls-certificate-staging.cjs',['web/app.js','web/index.html'],8],
  ['spelling','ui-saved-spelling.cjs',['web/spelling.js','web/spelling.html'],9],
];
const temp=fs.mkdtempSync(path.join(os.tmpdir(),'open-seo-spider-ui-')),created=[],results=[];
try{
  for(const [name,file,relative,groups]of specs){
    const input=path.join(root,'web','verification',file),text=fs.readFileSync(input,'utf8');
    const match=text.match(/const targets=(\[[\s\S]*?\]);/);assert(match,'Missing frozen target contract in '+file);
    const expected=JSON.parse(match[1]);assert.equal(expected.length,relative.length);
    const targets=relative.map((r,i)=>{
      const target=path.resolve(root,r).replaceAll('\\','/'),raw=fs.readFileSync(target);
      const sha256=createHash('sha256').update(raw).digest('hex');
      assert.equal(sha256,expected[i].sha256,'Fixture hash must match current '+r);
      assert.equal(raw.length,expected[i].bytes,'Fixture byte count must match current '+r);
      return{path:target,sha256,bytes:raw.length};
    });
    // Historical fixtures pin complete target bytes. Port only their local paths
    // in a disposable copy; keep every hash, bound, scenario and assertion.
    const disposable=path.join(temp,file);fs.writeFileSync(disposable,text.replace(match[1],JSON.stringify(targets)),{flag:'wx'});created.push(disposable);
    const child=spawnSync(process.execPath,[disposable,...targets.map(r=>r.path)],{cwd:root,encoding:'utf8',windowsHide:true,timeout:30000,maxBuffer:1048576});
    assert.equal(child.error,undefined,name+' fixture process failed');assert.equal(child.status,0,child.stderr);assert.equal(child.signal,null);
    const value=JSON.parse(child.stdout);
    if(name==='spelling'){assert.equal(value.kind,'source28_saved_spelling_ui_VM_fixture_result');assert.equal(value.groups,9);assert.equal(value.variants,27);assert.equal(value.actual_browser,false);assert.equal(value.actual_engine,false);}
    else {assert.equal(value.result,name==='sitemap'?'PASS_EXTRACTED_SITEMAP_BUNDLE_UI_VM':'PASS_EXTRACTED_UI_VM');assert.equal(value.cases.length,groups);}
    results.push({name,groups,variants:value.variants??null,passed:true});
  }
  console.log(JSON.stringify({kind:'portable_saved_UI_fixture_checks',passed:true,groups:results.reduce((n,r)=>n+r.groups,0),actual_browser:false,actual_engine:false,results}));
}finally{
  // Only exact files created above are removed; never recursively remove a path.
  for(const file of created){assert.equal(path.dirname(file),temp);fs.unlinkSync(file);}
  fs.rmdirSync(temp);
}
