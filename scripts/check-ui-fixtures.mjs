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
// Historical harnesses retain their original receipt identities. The disposable
// run below checks their unchanged scenarios against explicitly reviewed current
// source bytes; it does not reissue a historical source receipt.
const historicalTargets={
  'web/app.js':{bytes:446430,sha256:'6a4aed16b49cae4c085444db4d5ba47f2b21a0f1e5726036089c45ea8dfa1e93'},
  'web/index.html':{bytes:101518,sha256:'8a518fef91f12090a736dcf46ef146a17293d5f2e2b673e1a941cc0602243db8'},
  'web/spelling.js':{bytes:14485,sha256:'e7c125c42d42a4f8727a45182eaae7425a7a25a70ca7ecd95000d8d0bbb258ff'},
  'web/spelling.html':{bytes:5690,sha256:'82bbd39d0887391801e67890a9803c6027f26ee7135fc6f7793e841be3677d71'},
};
const reviewedCurrentTargets={
  'web/app.js':{bytes:523097,sha256:'b985eba3a91774ca7f8c738f4dce82fe69d89e580182f464bf6deb081b64861c'},
  'web/index.html':{bytes:122204,sha256:'e329fc2fa6d4346c171d8b6df10d1bd1c3bf23f52f3c81e34a3ccf5a47679258'},
  'web/spelling.js':historicalTargets['web/spelling.js'],
  'web/spelling.html':historicalTargets['web/spelling.html'],
};
const temp=fs.mkdtempSync(path.join(os.tmpdir(),'open-seo-spider-ui-')),created=[],results=[];
try{
  for(const [name,file,relative,groups]of specs){
    const input=path.join(root,'web','verification',file),text=fs.readFileSync(input,'utf8');
    const match=text.match(/const targets=(\[[\s\S]*?\]);/);assert(match,'Missing frozen target contract in '+file);
    const expected=JSON.parse(match[1]);assert.equal(expected.length,relative.length);
    const targets=relative.map((r,i)=>{
      assert.deepEqual(expected[i],{path:r,...historicalTargets[r]},'Historical harness target contract changed in '+file);
      const target=path.resolve(root,r).replaceAll('\\','/'),raw=fs.readFileSync(target);
      const sha256=createHash('sha256').update(raw).digest('hex');
      assert.equal(sha256,reviewedCurrentTargets[r].sha256,'Reviewed current fixture hash must match '+r);
      assert.equal(raw.length,reviewedCurrentTargets[r].bytes,'Reviewed current fixture byte count must match '+r);
      return{path:target,sha256,bytes:raw.length};
    });
    // Rebind only the explicit target metadata in a disposable copy. Preserve
    // every scenario, bound, semantic assertion, and the original harness file.
    const disposable=path.join(temp,file);fs.writeFileSync(disposable,text.replace(match[1],JSON.stringify(targets)),{flag:'wx'});created.push(disposable);
    const child=spawnSync(process.execPath,[disposable,...targets.map(r=>r.path)],{cwd:root,encoding:'utf8',windowsHide:true,timeout:30000,maxBuffer:1048576});
    assert.equal(child.error,undefined,name+' fixture process failed');assert.equal(child.status,0,child.stderr);assert.equal(child.signal,null);
    const value=JSON.parse(child.stdout);
    if(name==='spelling'){assert.equal(value.kind,'source28_saved_spelling_ui_VM_fixture_result');assert.equal(value.groups,9);assert.equal(value.variants,27);assert.equal(value.actual_browser,false);assert.equal(value.actual_engine,false);}
    else {assert.equal(value.result,name==='sitemap'?'PASS_EXTRACTED_SITEMAP_BUNDLE_UI_VM':'PASS_EXTRACTED_UI_VM');assert.equal(value.cases.length,groups);}
    results.push({name,groups,variants:value.variants??null,passed:true});
  }
  console.log(JSON.stringify({kind:'portable_saved_UI_fixture_checks',passed:true,groups:results.reduce((n,r)=>n+r.groups,0),actual_browser:false,actual_engine:false,scope:'Current semantic regression of unchanged historical harness scenarios against explicitly reviewed source bytes; not a historical receipt reissue, browser or native engine proof.',historical_harness_target_contracts_preserved:true,reviewed_current_targets:reviewedCurrentTargets,results}));
}finally{
  // Only exact files created above are removed; never recursively remove a path.
  for(const file of created){assert.equal(path.dirname(file),temp);fs.unlinkSync(file);}
  fs.rmdirSync(temp);
}
