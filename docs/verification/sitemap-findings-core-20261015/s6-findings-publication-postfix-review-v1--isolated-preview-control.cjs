// Independent isolated generator-preview control. Never imports/evaluates the
// publisher, invokes Git/children, or mutates any Root path. Own output only.
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const {createHash} = require('node:crypto');
const {fileURLToPath,pathToFileURL} = require('node:url');
const sha = b => createHash('sha256').update(b).digest('hex');
const repo = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-findings-core-integration-v1';
const scriptPath = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https/work/publish-sitemap-findings-core-docs-v1.cjs';
const scriptRaw = fs.readFileSync(scriptPath);
assert.equal(sha(scriptRaw), '1b9a11de5034e651b5751cdd4b1639810911007880e1889947959cf9f5545704');
const script = scriptRaw.toString();
const ledgerPath = path.join(repo, 'docs/feature-parity.json');
const granularPath = path.join(repo, 'docs/parity/granular-acceptance.json');
const generatorPath = path.join(repo, 'scripts/update-feature-todo.mjs');
const todoPath = path.join(repo, 'docs/FEATURE_TODO.md');
const ledgerBefore = fs.readFileSync(ledgerPath);
const granularBefore = fs.readFileSync(granularPath);
const generatorBefore = fs.readFileSync(generatorPath);
const actualProposedLedgerBlock = script.slice(script.indexOf('const ledgerPath='), script.indexOf('const proof='));
assert(actualProposedLedgerBlock.startsWith('const ledgerPath='));
assert(!/writeFileSync|execFileSync/.test(actualProposedLedgerBlock));
const planContext = {repo,path,assert,Buffer,proofPath:'docs/verification/sitemap-findings-core-preparation.json', fs:{readFileSync(p){assert.equal(path.resolve(p),path.resolve(ledgerPath));return Buffer.from(ledgerBefore);}}};
vm.runInNewContext(actualProposedLedgerBlock + ';globalThis.proposed=ledger;',planContext,{timeout:5000});
const proposed = JSON.parse(JSON.stringify(planContext.proposed));
const start = script.indexOf('const importLines=');
const stop = script.indexOf('assert.equal(sha(fs.readFileSync(attrPath))',start);
const previewBlock = script.slice(start,stop);
assert(start>0&&stop>start&&previewBlock.includes('preview(false);')&&previewBlock.includes('preview(true);'));
assert(!previewBlock.includes('execFileSync')&&!previewBlock.includes('mkdirSync'));
const results = [];
function control(name, ledger, granular, expectedError) {
  let memoryWrites = 0;
  const sandbox = {path,vm,assert,createHash,fileURLToPath,pathToFileURL,generatorPath,ledgerPath,granularPath,todoPath,generatorBefore,
    ledgerBytes:Buffer.from(JSON.stringify(ledger,null,2)+'\n'),granularBefore:Buffer.from(JSON.stringify(granular)),
    Buffer:{from(value){memoryWrites++;return Buffer.from(value);}}};
  let error = null;
  try {vm.runInNewContext(previewBlock+';globalThis.result={messages:previewMessages,bytes:previewBytes};',sandbox,{timeout:12000});}
  catch(e){error=e.message;}
  if(expectedError){assert(error&&error.includes(expectedError),name+': '+error);assert.equal(memoryWrites,0);}
  else {assert.equal(error,null);assert.equal(memoryWrites,1);assert(sandbox.result.messages[1].includes('31/67/46'));assert(sandbox.result.bytes.length>0);}
  results.push({name,matched_expected_result:true,error,memory_todo_writes:memoryWrites,root_fs_writes:0,output_bytes:sandbox.result?.bytes?.length??null,output_sha256:sandbox.result?.bytes?sha(sandbox.result.bytes):null,messages:sandbox.result?.messages??[]});
}
const granular = JSON.parse(granularBefore);
control('actual_proposed_ledger_full_generator_render_and_check',proposed,granular,null);
const wrongGranular = JSON.parse(JSON.stringify(granular)); wrongGranular.filters.pop();
control('wrong_granular_shape_fails_before_memory_todo_write',proposed,wrongGranular,'Review granular inventory count changes explicitly');
const wrongRun = JSON.parse(JSON.stringify(proposed)); wrongRun.features[0].implementation_evidence=[{verification_run_ids:['__independent_missing_run__']}];
control('unknown_run_reference_fails_before_memory_todo_write',wrongRun,granular,'Unknown verification run __independent_missing_run__');
const output={schema_version:1,status:'PASS_ISOLATED_GENERATOR_PREVIEW_CONTROLS_ONLY',publisher_sha256:sha(scriptRaw),generator_sha256:sha(generatorBefore),ledger_before_sha256:sha(ledgerBefore),granular_sha256:sha(granularBefore),publisher_evaluated:false,children_spawned:false,root_fs_writes:0,product_tests_executed:0,results};
const target=path.join(__dirname,'preview-control-results.json');fs.writeFileSync(target,JSON.stringify(output,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(output));
