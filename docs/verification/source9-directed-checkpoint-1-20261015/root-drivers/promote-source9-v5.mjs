import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
const E='E:/CodexWork/open-seo-spider-01a12ac3-20261013',R=E+'/mcp-file-tools-v1',D=E+'/combined-source-v9-draft-v5';
const H=b=>createHash('sha256').update(b).digest('hex');
const read=(p,sha)=>{const b=fs.readFileSync(p);assert.equal(H(b),sha,p);return JSON.parse(b)};
const ref=p=>{const b=fs.readFileSync(p);return{path:p,sha256:H(b),bytes:b.length}};
const git=a=>execFileSync('C:/Program Files/Git/cmd/git.exe',a,{cwd:R,encoding:'utf8',windowsHide:true,maxBuffer:4*1024*1024});
assert.equal(process.argv.length,6);assert.equal(process.argv[2],'--format-review');assert.equal(process.argv[4],'--format-review-sha');
assert(!fs.existsSync(E+'/results/combined-native-outer-v4/active.lock'));
assert(!fs.existsSync(D+'/root-promotion.json'));
const parent=read(E+'/combined-source-v8/manifest.json','58ab1ff000b9ec5ffa005eeab573a0cde3f4af4bb7c9c132b4d85251ea4de720');
const pre=read(D+'/preformat.json','92518c2f3f545049acc48ada6a0c3160d30bf14cc041df29486486ec80a643b3');
const clippy=read(D+'/actual-preliminary-clippy.json','af5c3c6beb71a4797aff14ac90be9a928c48b8c4cd89c409ba529fdf4ed8548d');
assert.equal(clippy.result,'PASS_PRELIMINARY_CLIPPY_ONLY');assert.equal(clippy.executed_tests,0);assert(clippy.source_final_verified&&clippy.actual_close_event);assert.deepEqual(clippy.exit,{code:0,signal:null});
const compositionReview=read(E+'/source9-composition-independent-review-v5/receipt.json','556fb2c9c37148a9cc8e8c02846ceaed5fc8e55333a21f3ca2b9e5a3cdb66f30');
const formatReview=read(process.argv[3],process.argv[5]);
for(const j of [compositionReview,formatReview]){assert.equal(j.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(j.findings,[])}
assert.equal(compositionReview.proposal_manifest.sha256,'92518c2f3f545049acc48ada6a0c3160d30bf14cc041df29486486ec80a643b3');
assert(JSON.stringify(formatReview).includes('48d0b60660f158288787eea67d2b69e3084cc0db754ccd43222a212c4bf625e2'),'Review must bind actual V5 format');
assert(JSON.stringify(formatReview).includes('92518c2f3f545049acc48ada6a0c3160d30bf14cc041df29486486ec80a643b3'),'Review must bind actual V5 composition');
assert.equal(parent.files.length,335);assert.equal(clippy.files.length,358);assert.equal(pre.owned_paths.length,30);assert.equal(pre.new_authored_test_functions.length,37);
const old=new Map(parent.files.map(f=>[f.path,f])),files=new Map(clippy.files.map(f=>[f.path,f])),owned=new Set(pre.owned_paths);assert.equal(files.size,358);assert.equal(owned.size,30);
for(const f of parent.files){const b=fs.readFileSync(R+'/'+f.path);assert.equal(H(b),f.sha256,'Current Root parent source '+f.path);assert.equal(b.length,f.bytes)}
for(const f of files.values()){const b=fs.readFileSync(D+'/source/'+f.path);assert.equal(H(b),f.sha256,'Actual tested source '+f.path);assert.equal(b.length,f.bytes);if(!owned.has(f.path)){assert(old.has(f.path));assert.equal(f.sha256,old.get(f.path).sha256)}}
const changes=[...owned].sort().map(name=>{assert(!path.isAbsolute(name)&&!name.includes('..'));const before=old.get(name),after=files.get(name);assert(after);if(!before)assert(!fs.existsSync(R+'/'+name),'New source already exists');return{path:name,before_sha256:before?.sha256??'ABSENT',after_sha256:after.sha256,bytes:after.bytes}});
assert.equal(changes.filter(x=>x.before_sha256==='ABSENT').length,23);assert.equal(changes.filter(x=>x.before_sha256!=='ABSENT').length,7);
assert.equal(git(['diff','--cached','--name-only']).trim(),'');
assert.equal(git(['rev-parse','HEAD']).trim(),'63e187c45032455a74165e60adc02624410d4206');
const privateRepo=JSON.parse(execFileSync('C:/Program Files/GitHub CLI/gh.exe',['repo','view','lia-xim/open-seo-spider','--json','isPrivate,nameWithOwner'],{encoding:'utf8',windowsHide:true}));assert(privateRepo.isPrivate);assert.equal(privateRepo.nameWithOwner,'lia-xim/open-seo-spider');
const attrsRule='web/broken-sources.* -text whitespace=blank-at-eol,blank-at-eof,space-before-tab,cr-at-eol';
assert(!fs.readFileSync(R+'/.gitattributes','utf8').includes(attrsRule));fs.appendFileSync(R+'/.gitattributes','\n'+attrsRule+'\n');
for(const change of changes){const target=R+'/'+change.path;fs.mkdirSync(path.dirname(target),{recursive:true});fs.copyFileSync(D+'/source/'+change.path,target);assert.equal(H(fs.readFileSync(target)),change.after_sha256)}
for(const f of files.values())assert.equal(H(fs.readFileSync(R+'/'+f.path)),f.sha256,'Promoted final source '+f.path);
const promotion={kind:'root_actual_source9_v5_product_promotion',recorded_at:new Date().toISOString(),result:'EXACT_REVIEWED_FORMATTED_PRELIMINARY_COMPILED_BYTES_PROMOTED',source_inputs:358,parent_source_manifest_sha256:H(fs.readFileSync(E+'/combined-source-v8/manifest.json')),composition:ref(D+'/preformat.json'),composition_review:ref(E+'/source9-composition-independent-review-v5/receipt.json'),formatter:ref(D+'/actual-format.json'),formatter_review:ref(process.argv[3]),preliminary_clippy:ref(D+'/actual-preliminary-clippy.json'),files:changes,all358_root_source_hashes_verified:true,executed_tests:0,installation:'UNCHANGED_SOURCE5_R3'};
fs.writeFileSync(D+'/root-promotion.json',JSON.stringify(promotion,null,2)+'\n',{flag:'wx'});
for(let i=0;i<changes.length;i+=15)git(['add','--',...changes.slice(i,i+15).map(x=>x.path)]);
git(['add','--','.gitattributes']);
assert.deepEqual(git(['diff','--cached','--name-only','-z']).split('\0').filter(Boolean).sort(),[...changes.map(x=>x.path),'.gitattributes'].sort());git(['diff','--cached','--check']);
git(['commit','-m','Add saved link-source reports and directory architecture views']);const commit=git(['rev-parse','HEAD']).trim();
for(const f of changes){const b=execFileSync('C:/Program Files/Git/cmd/git.exe',['show',commit+':'+f.path],{cwd:R,windowsHide:true,maxBuffer:4*1024*1024});assert.equal(H(b),f.after_sha256,f.path)}
git(['push','origin','HEAD']);assert.equal(git(['ls-remote','origin','refs/heads/codex/mcp-file-tools-20261015']).trim().split(/\s+/)[0],commit);
const publication={kind:'actual_private_source9_code_publication',recorded_at:new Date().toISOString(),source_commit:commit,branch:'codex/mcp-file-tools-20261015',repository:'lia-xim/open-seo-spider',is_private:true,owned_committed_paths:changes.map(x=>x.path),additional_tracked_files:['.gitattributes'],source_inputs:358,all358_root_source_hashes_verified:true,code_and_owned_raw_byte_attributes:true,full_suite:'NOT_RUN',optimized_candidate:'NOT_BUILT',installation:'UNCHANGED_SOURCE5_R3'};
fs.writeFileSync(D+'/private-publication.json',JSON.stringify(publication,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({result:promotion.result,commit,owned:changes.length,source_inputs:358,full_suite:'NOT_RUN',publication:ref(D+'/private-publication.json')}));
