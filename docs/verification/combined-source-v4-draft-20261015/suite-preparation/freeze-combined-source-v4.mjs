import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

// No default future commit, ledger hash or Root approval is accepted.
const argv = process.argv.slice(2);
const options = new Map();
for (let index = 0; index < argv.length; index += 2) {
  assert(index + 1 < argv.length && /^--[a-z-]+$/.test(argv[index]));
  assert(!options.has(argv[index]), 'Duplicate option: ' + argv[index]);
  options.set(argv[index], argv[index + 1]);
}
const requiredOptions = ['--source-commit', '--ledger-manifest', '--ledger-sha', '--root-approval', '--root-approval-sha'];
assert.deepEqual([...options.keys()].sort(), [...requiredOptions].sort());
const expectedCommit = options.get('--source-commit');
assert.match(expectedCommit, /^[a-f0-9]{40}$/);
for (const key of ['--ledger-sha', '--root-approval-sha']) assert.match(options.get(key), /^[a-f0-9]{64}$/);
assert.notEqual(options.get('--ledger-sha'),'ee7f4782991d40bbe50da48675cab501c96b4fef6d1f4346296be1be29396652','Rejected capacity v1 allocated the nested limits value before its byte guard; a fresh reviewed revision is mandatory');
assert.equal(options.get('--ledger-sha'),'9da86f091f5bba9ba70eca5415542cfaf9fb43947c9f5bb1259c192b2d170a82','Only the actual final capacity-v3 authored manifest is admitted');
for (const key of ['--ledger-manifest', '--root-approval']) assert(path.isAbsolute(options.get(key)), key + ' must be absolute');
const base = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
assert.equal(path.resolve(options.get('--ledger-manifest')),path.resolve(base,'sitemap-membership-capacity-v3/manifest.json'));
const task = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const repo = path.join(base, 'mcp-file-tools-v1');
const out = path.join(base, 'combined-source-v4');
const branch = 'codex/mcp-file-tools-20261015';
const parentPath = path.join(base, 'combined-source-v3/manifest.json');
const parentSha = '01bfa8efcb6745c5521be5e533ae63f2f15679318b095a29eb3ab925fef95bdf';
const sqlPath = path.join(base, 'sitemap-page-context-sql-proposal-v1/manifest.json');
const sqlSha = 'b355222a8fe61353122a22b754b59936bab269a248cdce07d2a7d8b85da2dba3';
const sqlReviewPath = path.join(base, 'sitemap-page-context-sql-independent-review-v1/receipt.json');
const sqlReviewSha = '46d1166a7cceb5c8a1715034b96e76ddf54508bbc4f2fc24a4ccfe4e72b6d8e3';
const gh = 'C:/Program Files/GitHub CLI/gh.exe';
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const self = fileURLToPath(import.meta.url);
assert.equal(path.resolve(self),path.resolve(task,'work/freeze-combined-source-v4.mjs'),'Root must promote this exact new freezer name before execution');
const freezerSha = sha(fs.readFileSync(self));
const git = args => execFileSync('git', args, {cwd:repo, encoding:'utf8', windowsHide:true, maxBuffer:8*1024*1024}).trim();
const refs = [];
function readPinned(filePath, expected) {
  assert(path.isAbsolute(filePath));
  assert.match(expected, /^[a-f0-9]{64}$/);
  const stat = fs.lstatSync(filePath);
  assert(stat.isFile() && !stat.isSymbolicLink(), 'Ordinary file required: ' + filePath);
  const raw = fs.readFileSync(filePath);
  assert.equal(sha(raw), expected, filePath);
  refs.push({path:filePath, sha256:expected, bytes:raw.length});
  return JSON.parse(raw);
}
function relativeFile(value) {
  assert.equal(typeof value, 'string');
  assert(value.length > 0 && !value.includes('\\') && !value.includes(':'));
  assert(!path.posix.isAbsolute(value));
  assert(value.split('/').every(part => part.length > 0 && part !== '.' && part !== '..'));
  return value;
}
function testNames(text) {
  return [...text.matchAll(/#\[(?:tokio::)?test(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)\b/g)].map(match => match[1]);
}
const parent = readPinned(parentPath, parentSha);
assert.equal(parent.kind, 'combined_source_v3');
assert.equal(parent.files.length, 301);
assert.equal(parent.required_test_names, 421);
assert.equal(parent.minimum_expected_workspace_tests, 1203);
const sql = readPinned(sqlPath, sqlSha);
assert.equal(sql.kind, 'sitemap_page_context_sql_source_only_proposal_v1');
assert.equal(sql.source_manifest_sha256, parentSha);
assert.equal(sql.files.length, 3);
const sqlReview = readPinned(sqlReviewPath, sqlReviewSha);
assert.equal(sqlReview.result, 'NO_FINDINGS_WITHIN_BOUNDED_SOURCE_REVIEW_NOT_RUNTIME_ACCEPTED');
assert.deepEqual(sqlReview.findings, []);
assert.equal(sqlReview.proposal_manifest.sha256, sqlSha);
const ledger = readPinned(options.get('--ledger-manifest'), options.get('--ledger-sha'));
assert.equal(ledger.kind,'sitemap_membership_capacity_source_proposal_v3');
assert.equal(ledger.files.length,15);
assert.equal(ledger.source_manifest_sha256, parentSha, 'Ledger must be derived from this exact parent');
assert(Array.isArray(ledger.files) && ledger.files.length > 0);
assert(Array.isArray(ledger.new_test_names) && ledger.new_test_names.length > 0);
for (const name of ledger.new_test_names) assert.match(name,/^[A-Za-z_][A-Za-z_0-9]*(?:::[A-Za-z_][A-Za-z_0-9]*)+$/);
const approval = readPinned(options.get('--root-approval'), options.get('--root-approval-sha'));
assert.equal(approval.kind, 'root_approved_combined_source_v4');
assert.equal(approval.parent_source_manifest_sha256, parentSha);
assert.equal(approval.source_commit, expectedCommit);
assert.equal(approval.source_branch, branch);
assert.equal(path.resolve(approval.source_worktree), path.resolve(repo));
assert.equal(approval.status, 'REVIEWED_FORMATTED_COMMITTED_AND_PRIVATELY_PUSHED');
assert.deepEqual(approval.proposals, [
  {role:'sql', path:sqlPath, sha256:sqlSha, review_path:sqlReviewPath, review_sha256:sqlReviewSha},
  {role:'ledger', path:options.get('--ledger-manifest'), sha256:options.get('--ledger-sha'), review_path:approval.proposals?.[1]?.review_path, review_sha256:approval.proposals?.[1]?.review_sha256},
]);
const ledgerReview = readPinned(approval.proposals[1].review_path, approval.proposals[1].review_sha256);
assert.equal(path.resolve(approval.proposals[1].review_path),path.resolve(base,'sitemap-membership-capacity-review-v1/receipt.json'));
assert.equal(approval.proposals[1].review_sha256,'d9504ec7b44aa5d102cef7c6b270b0445412aa813f67978113edb6c53981f18b');
assert.equal(ledgerReview.proposal_manifest?.sha256 ?? ledgerReview.manifest_sha256, options.get('--ledger-sha'));
assert.deepEqual(ledgerReview.findings, [], 'Independent ledger review must have no unresolved findings');
assert.match(ledgerReview.result, /^NO_FINDINGS/);
assert(Array.isArray(approval.files));
assert(Array.isArray(approval.new_authored_tests));

const parentFiles = new Map(parent.files.map(file => [relativeFile(file.path), file]));
assert.equal(parentFiles.size, 301);
const parentTextCache = new Map();
function parentText(filePath) {
  if (!parentFiles.has(filePath)) return '';
  if (!parentTextCache.has(filePath)) {
    const item = parentFiles.get(filePath);
    const absolute = path.join(base,'combined-source-v3',filePath);
    const raw = fs.readFileSync(absolute);
    assert.equal(sha(raw),item.sha256);
    assert.equal(raw.length,item.bytes);
    refs.push({path:absolute,sha256:item.sha256,bytes:item.bytes});
    parentTextCache.set(filePath,raw.toString('utf8'));
  }
  return parentTextCache.get(filePath);
}
const expectedChanges = new Map();
const expectedAuthoredTests = [];
for (const [role, proposal, proposalPath] of [['sql', sql, sqlPath], ['ledger', ledger, options.get('--ledger-manifest')]]) {
  for (const item of proposal.files) {
    relativeFile(item.path);
    assert(!expectedChanges.has(item.path), 'Proposal source ownership collision: ' + item.path);
    assert.match(item.after_sha256, /^[a-f0-9]{64}$/);
    const before = parentFiles.get(item.path);
    assert.equal(item.before_sha256, before?.sha256 ?? 'ABSENT', 'Proposal before pin differs from accepted parent');
    const proposalFile = path.join(path.dirname(proposalPath), 'source', item.path);
    const raw = fs.readFileSync(proposalFile);
    assert.equal(sha(raw), item.after_sha256, 'Authored proposal source: ' + item.path);
    assert.equal(raw.length, item.after_bytes);
    refs.push({path:proposalFile, sha256:sha(raw), bytes:raw.length});
    expectedChanges.set(item.path, {role, item, proposalFile});
    if (item.path.endsWith('.rs')) {
      const oldNames = testNames(parentText(item.path));
      const authoredNames = testNames(raw.toString('utf8'));
      for (const name of oldNames) assert(authoredNames.includes(name), 'Reviewed proposal removed an existing test declaration: '+item.path+'::'+name);
      const addedNames = authoredNames.filter(name => !oldNames.includes(name));
      if (addedNames.length) expectedAuthoredTests.push({path:item.path,names:addedNames});
    }
  }
}
assert.equal(approval.files.length, expectedChanges.size);
const deltaPins = new Map();
for (const item of approval.files) {
  relativeFile(item.path);
  assert(!deltaPins.has(item.path));
  const expected = expectedChanges.get(item.path);
  assert(expected, 'Root approval cannot add an unreviewed source path');
  assert.equal(item.role, expected.role);
  assert.equal(item.before_sha256, expected.item.before_sha256);
  assert.equal(item.proposal_sha256, expected.item.after_sha256);
  assert.match(item.after_sha256, /^[a-f0-9]{64}$/);
  assert(Number.isSafeInteger(item.bytes) && item.bytes > 0);
  assert.equal(typeof item.format_only, 'boolean');
  if (item.after_sha256 !== item.proposal_sha256) assert.equal(item.format_only, true, 'Only Root-reviewed formatter changes may follow the reviewed authored source');
  deltaPins.set(item.path, item);
}
assert.equal(git(['status', '--porcelain']), '', 'Root must finish the exact clean reviewed commit first');
assert.equal(git(['rev-parse', 'HEAD']), expectedCommit);
assert.equal(git(['branch', '--show-current']), branch);
assert.equal(git(['ls-remote', 'origin', 'refs/heads/' + branch]), expectedCommit + '\trefs/heads/' + branch);
assert.match(git(['remote', 'get-url', 'origin']), /(?:github\.com[/:])lia-xim\/open-seo-spider(?:\.git)?$/);
const privateRepo = JSON.parse(execFileSync(gh, ['repo', 'view', 'lia-xim/open-seo-spider', '--json', 'nameWithOwner,isPrivate'], {cwd:repo, encoding:'utf8', windowsHide:true, maxBuffer:1024*1024}));
assert.equal(privateRepo.nameWithOwner, 'lia-xim/open-seo-spider');
assert.equal(privateRepo.isPrivate, true, 'Actual current private repository required');
const changedSinceParent = git(['diff', '--name-only', '--diff-filter=ACDMRTUXB', parent.source_commit, expectedCommit]).split(/\r?\n/).filter(Boolean);
const compileSensitive = value => parentFiles.has(value) || value === 'Cargo.toml' || value === 'Cargo.lock' || value === 'build.rs' || /^(?:crates|assets|renderer)\//.test(value) || /^web\/(?!verification\/)/.test(value);
for (const filePath of changedSinceParent.filter(compileSensitive)) assert(deltaPins.has(filePath), 'Unexpected changed/new compile input: ' + filePath);
const currentPaths = [...new Set([...parentFiles.keys(), ...expectedChanges.keys()])].sort();
assert.equal(new Set(currentPaths.map(value => value.toLowerCase())).size, currentPaths.length, 'Windows source path aliases rejected');
const files = [], deltas = [];
for (const filePath of currentPaths) {
  const stat = fs.lstatSync(path.join(repo, filePath));
  assert(stat.isFile() && !stat.isSymbolicLink(), 'Ordinary source required: ' + filePath);
  const raw = fs.readFileSync(path.join(repo, filePath));
  const actual = {path:filePath, sha256:sha(raw), bytes:raw.length};
  const before = parentFiles.get(filePath), delta = deltaPins.get(filePath);
  if (delta) {
    assert.equal(actual.sha256, delta.after_sha256, filePath);
    assert.equal(actual.bytes, delta.bytes, filePath);
    assert.notEqual(actual.sha256, before?.sha256, 'Unchanged source is not an approved delta');
    deltas.push({path:filePath, before_sha256:before?.sha256 ?? 'ABSENT', after_sha256:actual.sha256, bytes:actual.bytes});
  } else {
    assert(before);
    assert.equal(actual.sha256, before.sha256, 'Unreviewed source change: ' + filePath);
    assert.equal(actual.bytes, before.bytes);
  }
  const committed = execFileSync('git', ['show', expectedCommit + ':' + filePath], {cwd:repo, windowsHide:true, maxBuffer:64*1024*1024});
  assert.equal(sha(committed), actual.sha256, 'Working bytes must equal the explicitly published commit: ' + filePath);
  files.push(actual);
}
const addedTests = [];
for (const [filePath] of deltaPins) {
  if (!filePath.endsWith('.rs')) continue;
  const oldText = parentText(filePath);
  const newText = fs.readFileSync(path.join(repo, filePath), 'utf8');
  const oldNames = testNames(oldText), names = testNames(newText);
  for (const name of oldNames) assert(names.includes(name), 'Existing actual test declaration removed: ' + filePath + '::' + name);
  const authored = names.filter(name => !oldNames.includes(name));
  if (authored.length) addedTests.push({path:filePath, names:authored});
}
addedTests.sort((a,b) => a.path.localeCompare(b.path,'en'));
expectedAuthoredTests.sort((a,b) => a.path.localeCompare(b.path,'en'));
assert.deepEqual(addedTests,expectedAuthoredTests,'Formatter-only approval must preserve every authored test function');
assert.deepEqual(approval.new_authored_tests, addedTests, 'Root must explicitly pin every actual new test declaration and its real source path');
const sqlAdded = addedTests.find(group => group.path === 'crates/core/src/sitemap_membership_analysis_tests.rs');
assert.deepEqual(sqlAdded?.names, sql.new_named_tests);
const requiredTests = structuredClone(parent.required_tests);
for (const group of addedTests) {
  let found = requiredTests.find(existing => existing.path === group.path);
  if (!found) { found = {path:group.path, names:[]}; requiredTests.push(found); }
  found.names.push(...group.names);
}
const names = requiredTests.flatMap(group => group.names);
const addedCount = addedTests.reduce((total, group) => total + group.names.length, 0);
const ledgerNewNames = expectedAuthoredTests.filter(group => expectedChanges.get(group.path).role === 'ledger').flatMap(group => group.names);
assert.deepEqual([...ledgerNewNames].sort(),ledger.new_test_names.map(name=>name.split('::').at(-1)).sort(),'Every authored ledger test must retain its explicitly declared Cargo module path');
const newFullNames = [...sql.new_named_tests.map(name=>'sitemap_membership_analysis::tests::'+name),...ledger.new_test_names];
assert.equal(newFullNames.length,addedCount);
assert.equal(new Set(newFullNames).size,newFullNames.length);
assert.equal(names.length, 421 + addedCount);
assert.equal(new Set(names).size, names.length, 'Actual result leaf names must be unambiguous');
for (const group of requiredTests) {
  const text = fs.readFileSync(path.join(repo, group.path), 'utf8');
  for (const name of group.names) assert(new RegExp('\\bfn\\s+' + name + '\\b').test(text), group.path + '::' + name);
}
function verifyCurrent() {
  assert.equal(sha(fs.readFileSync(self)),freezerSha,'Freezer changed');
  assert.equal(git(['status', '--porcelain']), '');
  assert.equal(git(['rev-parse', 'HEAD']), expectedCommit);
  for (const ref of refs) { const raw = fs.readFileSync(ref.path); assert.equal(sha(raw), ref.sha256); assert.equal(raw.length, ref.bytes); }
  for (const item of files) { const raw = fs.readFileSync(path.join(repo,item.path)); assert.equal(sha(raw),item.sha256); assert.equal(raw.length,item.bytes); }
}
verifyCurrent();
assert(!fs.existsSync(out), 'Fresh immutable V4 source output required');
fs.mkdirSync(out);
for (const item of files) {
  const to = path.join(out,item.path);
  fs.mkdirSync(path.dirname(to), {recursive:true});
  fs.writeFileSync(to, fs.readFileSync(path.join(repo,item.path)), {flag:'wx'});
  assert.equal(sha(fs.readFileSync(to)),item.sha256);
}
verifyCurrent();
const newInputs = files.filter(item => !parentFiles.has(item.path)).length;
const changedInputs = deltas.length - newInputs;
const manifest = {
  schema_version:1, kind:'combined_source_v4', result:'FROZEN_REVIEWED_SOURCE_FULL_SUITE_NOT_RUN',
  recorded_at:new Date().toISOString(), source_worktree:repo, source_commit:expectedCommit,
  freezer_sha256:freezerSha,
  source_branch:branch, private_repository:'lia-xim/open-seo-spider', actual_private_publication:privateRepo,
  source_inputs:files.length, parent_source_directory:path.join(base,'combined-source-v3'),
  parent_source_manifest_sha256:parentSha, parent_source_inputs:301,
  unchanged_parent_inputs:301-changedInputs, changed_parent_inputs:changedInputs, new_input_files:newInputs,
  approved_product_bindings:deltas, new_authored_test_functions:addedCount,
  inherited_required_test_names:421, required_test_names:names.length,
  minimum_expected_workspace_tests:1203+addedCount, required_tests:requiredTests,
  new_authored_tests:addedTests, new_authored_full_test_names:newFullNames,
  root_approval:{path:options.get('--root-approval'),sha256:options.get('--root-approval-sha')},
  source_proposals:approval.proposals, references:refs,
  test_profile:{opt_level:1,debug_assertions:true,overflow_checks:true,debug_info:0},
  execution:{full_workspace:'NOT_RUN_ON_V4',four_real_renderer_gates:'NOT_RUN_ON_V4',clippy:'NOT_RUN_ON_V4',optimized_build:'NOT_RUN',native_sdk_browser_physical_platform:'NOT_RUN_ON_V4'},
  accepted_feature_counts_unchanged:{implemented:30,partial:68,planned:46}, installed_runtime_modified:false,manufacturer_parity:'NOT_PROVEN',
  qualification:'The accepted V3 lineage is preserved with all301 parent inputs and421 required names. Only explicitly reviewed SQL and ledger-capacity deltas plus Root-reviewed formatter changes are admitted. Every actual new test function in changed/new Rust files is appended to the source-bound required names and the1203 baseline minimum. The current clean source commit and matching remote branch were checked with actual private repository metadata. Historical V3 PASS and V3 native results are lineage only. A fresh V4 full workspace run, four genuine owned Chromium gates, formatter, all-target Clippy, optimized build and unchanged native/browser/physical/platform/installation gates remain required. No latency, RSS, physical-boundary success or full vendor parity is asserted.',
  files,
};
const bytes = Buffer.from(JSON.stringify(manifest,null,2)+'\n');
fs.writeFileSync(path.join(out,'manifest.json'),bytes,{flag:'wx'});
console.log(JSON.stringify({path:path.join(out,'manifest.json'),sha256:sha(bytes),commit:expectedCommit,files:files.length,parent_delta_files:changedInputs,new_input_files:newInputs,required_names:names.length,minimum:manifest.minimum_expected_workspace_tests}));
