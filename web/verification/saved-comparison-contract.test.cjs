'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),os=require('node:os');
const root=path.resolve(process.env.SPIDER_ROOT||path.resolve(__dirname,'../..'));
const ui=require('../saved-comparison.js'),fixture=require('./saved-comparison-fixtures.cjs');
const proofPath=path.resolve(process.env.SPIDER_COMPARISON_UNIT_PROOF||process.env.SPIDER_UI_PROOF||path.join(fs.mkdtempSync(path.join(os.tmpdir(),'open-seo-spider-comparison-unit-')),'unit-results.json'));
const checks=[],clone=value=>JSON.parse(JSON.stringify(value));
function check(name,run) {run();checks.push(name);console.log('PASS '+name);}
check('accepts versioned complete result shape and exact page counts',()=>assert.equal(ui.validateEnvelope(fixture.envelope()).comparison.pages.length,55));
check('unknown and partial facts retain source profiles without empty coercion',()=>{const value=ui.validateEnvelope(fixture.envelope()).comparison.pages[0].fields;assert.equal(value[1].before.state,'partial');assert.equal(value[2].after.profile,'rendered_dom_html-v1');assert.equal(value[3].before.state,'observed_empty');assert.equal(value[3].after.state,'unknown');});
for(const [name,mutate] of [
  ['unknown engine',value=>value.comparison.engine='future'],
  ['wrong counts',value=>value.comparison.counts.changed=2],
  ['duplicate identity',value=>value.comparison.pages[1].comparison_key=value.comparison.pages[0].comparison_key],
  ['missing fact state',value=>delete value.comparison.pages[0].fields[0].before.state],
  ['duplicate field',value=>value.comparison.pages[0].fields.push(clone(value.comparison.pages[0].fields[0]))],
  ['inconsistent added identity',value=>value.comparison.pages[1].before=value.comparison.pages[0].before],
  ['nonzero website request claim',value=>value.website_requests=1],
])check('rejects '+name,()=>{const value=fixture.envelope();mutate(value);assert.throws(()=>ui.validateEnvelope(value));});
check('CSV formula protection includes BOM and whitespace',()=>assert.ok(ui.csvCell(fixture.formula).startsWith('"\'')));
check('CSV quotes and multiline old/new JSON plus profile/provenance preserved',()=>{const value=fixture.envelope(),csv=ui.csvExport(value,value.comparison.pages);assert.ok(csv.startsWith('\uFEFF"Art"'));assert.ok(csv.includes('rendered_dom_html-v1'));assert.ok(csv.includes('text_byte_limit'));assert.ok(csv.includes('""source""'));assert.equal(csv.split('\r\n').length,117);});
check('CSV refuses imprecise integer instead of silently rounding',()=>{const value=fixture.envelope();value.comparison.pages[0].fields[0].before.value=Number.MAX_SAFE_INTEGER+2;assert.throws(()=>ui.csvExport(value,value.comparison.pages),/präzise CSV-Darstellung/);});
check('empty cohort exports explicit CSV header',()=>assert.equal(ui.csvExport(fixture.envelope(),[]).split('\r\n').length,2));
check('safeJSON preserves ordinary metadata and safe integer boundary',()=>{const value={generation:Number.MAX_SAFE_INTEGER,origin:'app_declared',optional:null};assert.equal(ui.safeJSON(value),JSON.stringify(value,null,2));});
check('safeJSON removes rounded numeric metadata while preserving other keys',()=>{const value=JSON.parse('{"generation":9007199254740993,"nested":{"source":"raw_http_html","revisions":[9007199254740993]}}');const text=ui.safeJSON(value);assert.ok(text.includes('präzisen Browser-Darstellung'));assert.ok(!text.includes('9007199254740992'));assert.ok(text.includes('raw_http_html'));assert.ok(text.includes('generation'));});
check('safeJSON keeps exact decimal strings unchanged',()=>{const value={generation:'9007199254740993'};assert.equal(ui.safeJSON(value),JSON.stringify(value,null,2));});
const hashes=Object.fromEntries(['web/saved-comparison.js','web/verification/saved-comparison-fixtures.cjs','web/verification/saved-comparison-contract.test.cjs'].map(file=>[file,crypto.createHash('sha256').update(fs.readFileSync(path.join(root,file))).digest('hex')]));
fs.writeFileSync(proofPath,JSON.stringify({status:'PASS',scope:'pure UI result contract and local CSV helpers; no Rust/API/native proof',node:process.version,source_sha256:hashes,checks},null,2)+'\n');

console.log('Proof: '+proofPath);
