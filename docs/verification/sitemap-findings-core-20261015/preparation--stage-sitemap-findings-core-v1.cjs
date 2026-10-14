const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto'),{execFileSync}=require('node:child_process');const sha=b=>createHash('sha256').update(b).digest('hex');
const repo='E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-findings-core-integration-v1';
const git=args=>execFileSync('git',args,{cwd:repo});
const prep=JSON.parse(fs.readFileSync(path.join(__dirname,'prepare-sitemap-findings-core-v1.json')));
const publicationRaw=fs.readFileSync(path.join(__dirname,'publish-sitemap-findings-core-docs-v1.json'));
const publication=JSON.parse(publicationRaw),proofRaw=fs.readFileSync(path.join(repo,publication.proof_path));assert.equal(sha(proofRaw),publication.proof_sha256);
const proof=JSON.parse(proofRaw);assert.equal(proof.source_manifest_sha256,prep.manifest_sha256);assert.equal(proof.new_tests_executed,0);
for(const f of prep.approved_product_bindings)assert.equal(sha(fs.readFileSync(path.join(repo,f.path))),f.sha256,'Owned final product '+f.path);
for(const f of proof.companions){assert(f.path.startsWith('docs/verification/sitemap-findings-core-20261015/'));assert(!f.path.split('/').includes('..'));assert.equal(sha(fs.readFileSync(path.join(repo,f.path))),f.sha256,f.path);}
const docs=['.gitattributes','docs/feature-parity.json','docs/FEATURE_TODO.md','docs/sitemap-findings-core.md',publication.proof_path,...proof.companions.map(f=>f.path)];
const owned=[...prep.approved_product_bindings.map(f=>f.path),...docs].sort();assert.equal(new Set(owned).size,owned.length);
const actual=git(['status','--porcelain=v1','-uall']).toString().trimEnd().split(/\r?\n/).filter(Boolean).map(x=>x.slice(3)).sort();assert.deepEqual(actual,owned,'Only explicit owned product/evidence/TODO scope');
execFileSync(process.execPath,[path.join(repo,'scripts/update-feature-todo.mjs'),'--check'],{cwd:repo});
git(['add','--',...docs]);const staged=git(['diff','--cached','--name-only']).toString().trim().split(/\r?\n/).filter(Boolean).sort();assert.deepEqual(staged,owned);
const bindings=owned.map(p=>{const b=fs.readFileSync(path.join(repo,p));assert.equal(sha(git(['show',':'+p])),sha(b),'Exact staged owned bytes '+p);return {path:p,bytes:b.length,sha256:sha(b)};});
git(['diff','--cached','--check']);
const receipt={schema_version:1,result:'EXACT_OWNED_PRODUCT_DOC_EVIDENCE_STAGED_NOT_RUNTIME_ACCEPTED',recorded_at:new Date().toISOString(),source_manifest_sha256:prep.manifest_sha256,publication_receipt_sha256:sha(publicationRaw),proof_sha256:publication.proof_sha256,staged_files:owned.length,product_files:prep.approved_product_bindings.length,evidence_companions:proof.companions.length,bindings,new_tests_executed:0};
const dest=__filename.replace(/\.cjs$/,'.json');fs.writeFileSync(dest,JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({staged_files:owned.length,products:prep.approved_product_bindings.length,companions:proof.companions.length,source_manifest_sha256:prep.manifest_sha256}));
