const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const root=path.resolve(__dirname,'..');
function edit(file,fn){const p=path.join(root,file),before=fs.readFileSync(p,'utf8'),next=fn(before,before.includes('\r\n')?'\r\n':'\n');assert.notEqual(next,before,file+' unchanged');fs.writeFileSync(p,next);}
function replace(s,a,b){assert.equal(s.split(a).length,2,'Expected one '+a.slice(0,90));return s.replace(a,b);}
const fixtures=['ui-smoke.cjs','ui-crawl-control-smoke.cjs','ui-selected-recrawl-smoke.cjs','ui-policy-smoke.cjs','ui-content-smoke.cjs','ui-resource-discovery-smoke.cjs','ui-resource-ledger-smoke.cjs','ui-resource-contract-smoke.cjs','ui-saved-comparison-smoke.cjs'];
for(const file of fixtures)edit('web/verification/'+file,s=>replace(s,"'table-view-portability.js'", "'table-xlsx.js','table-export-ui.js','table-view-portability.js'"));
for(const file of ['sitemap-membership-e2e.mjs','sitemap-assessment-e2e.mjs'])edit('scripts/'+file,s=>replace(s,"'/table-view-portability.js':'web/table-view-portability.js'","'/table-view-portability.js':'web/table-view-portability.js','/table-xlsx.js':'web/table-xlsx.js','/table-export-ui.js':'web/table-export-ui.js'").replace('fifteen actual embedded assets','seventeen actual embedded assets'));
edit('.github/workflows/ci.yml',(s,nl)=>replace(s,'      - run: node --test web/verification/table-view-portability.test.cjs','      - run: node --test web/verification/table-view-portability.test.cjs'+nl+'      - run: node --test web/verification/table-xlsx.test.cjs web/verification/table-export-ui.test.cjs'));
edit('.gitattributes',(s,nl)=>s+nl+'# Preserve exact XLSX ZIPs, CSV downloads and original verification logs.'+nl+'/docs/verification/table-xlsx-export-20261021/** -text whitespace=-blank-at-eol,-blank-at-eof,-space-before-tab,cr-at-eol'+nl);
edit('scripts/check-ui-fixtures.mjs',s=>{
 const start=s.indexOf('const reviewedCurrentTargets={'),end=s.indexOf('\n};',start);assert(start>=0&&end>start);let block=s.slice(start,end);
 for(const file of ['web/app.js','web/index.html']){const raw=fs.readFileSync(path.join(root,file)),entry="  '"+file+"':{bytes:"+raw.length+",sha256:'"+crypto.createHash('sha256').update(raw).digest('hex')+"'},";block=block.replace(new RegExp("  '"+file.replaceAll('.','\\.')+"':\\{[^\\n]+"),entry);}
 return s.slice(0,start)+block+s.slice(end);
});
console.log('Integrated shared projection, UI hooks, HTML/CSS, static assets and CI tests.');
