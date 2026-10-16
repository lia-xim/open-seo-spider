import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';

// Immutable Parent12 proof only. No former validator is imported or run.
const BASE="E:/CodexWork/open-seo-spider-01a12ac3-20261013";
const TASK='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
export const PARENT_SHA="3b62e68b32b2d23e375977cac8d5d0dad313c8f144eb43b4ab2896fae7687b18";
const PARENT_SUITE_SHA="8bba7750a13d0e7ec1de843d7599b0aab48e9ac5cffdf24eff6f0e26618d27d6";
const PARENT_BUILD_SHA="7512cfdc9e4f09d5c847c6ac89fa92face47172d5a8c673df0a2bbd67f6b46f4";
const PARENT_BINARY_SHA="f7ce325cf73d16f1c4943ebf98dc8a2e888e93ee25c999bc7f2c10d1b1a1a65c";
const PARENT_BINARY_BYTES=43650560;
const ROLE_PINS=[
  {
    "role": "raw_http_head",
    "manifest": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/html-head-diagnostics-proposal-v3/manifest.json",
      "sha256": "cb09004da70e0e7fd240029bb269d7c960a3567409c6e58783ab87901975c83c",
      "bytes": 92171
    },
    "independent_review": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/html-head-diagnostics-independent-review-v3/receipt.json",
      "sha256": "65d66a7fb013771c8c0d45338d59a376510ac9c4bd525fef64f2ba834e3a6373",
      "bytes": 2925
    }
  },
  {
    "role": "cdn_classification",
    "manifest": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/cdn-classification-proposal-v2/manifest.json",
      "sha256": "d95340c661c9e4c745abb7a7a7606c8b718aaae2fc3b6277c7054e53d9d12406",
      "bytes": 91311
    },
    "independent_review": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\cdn-classification-independent-review-v2\\receipt.json",
      "sha256": "8bf84119f6e0bb903253f0f814c018fce94d2067b9ad852d4da6eb430a1de171",
      "bytes": 2109
    },
    "ancestor_independent_review": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/cdn-classification-independent-review-v1/receipt.json",
      "sha256": "a8290023fd7877f8d89e13f440ac1112b61f67dfc49664ef5861083ce9f18eaf",
      "bytes": 2705
    }
  },
  {
    "role": "cdn_legacy_test_fixture_repair",
    "manifest": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\cdn-fetch-context-test-fixtures-proposal-v1\\manifest.json",
      "sha256": "671e60a75c1d60bb575f2e0b38955c573dc5cb0dbab50ac8ec8971a9acb90264",
      "bytes": 2844
    },
    "independent_review": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\cdn-fetch-context-test-fixtures-independent-review-v1\\receipt.json",
      "sha256": "b149a8899ad24c8593ad007505788ee012d3b3906e4611acfc0c594618c00506",
      "bytes": 2132
    }
  }
];
const FIXED={
  "preformat_manifest": {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/combined-source-v13-draft-v3/preformat.json",
    "sha256": "13547987d8f394701cdd898f667f0f556f4fdf1b41895354df6cd1254ac2bfcf",
    "bytes": 245980
  },
  "composition_review": {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/source13-composition-independent-review-v3/receipt.json",
    "sha256": "098a54f49c41681ae152287db511296e55f13d6b2813237ba494498468e87116",
    "bytes": 5028
  },
  "actual_format": {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/source13-preliminary-format-v3/proof.json",
    "sha256": "5fe1fd874de5ac4e561d6b800e1e98361c1601ef93a57ee01ce77934f1c27397",
    "bytes": 119049
  },
  "formatter_review": {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/source13-format-independent-review-v3/receipt.json",
    "sha256": "dd529c2e3469814914bd21e1c4d6dbff77f0e198c8d784a5d2579e3ef7f3f373",
    "bytes": 73652
  },
  "preliminary_clippy": {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/source13-preliminary-clippy-v3/proof.json",
    "sha256": "e0c68a51118241c1f5f47c9254fba8dea335b3ebf10956d4585bef3933bc1ed4",
    "bytes": 70801
  },
  "directed_rust_tests": {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/source13-new-rust-tests-v1/proof.json",
    "sha256": "cca1c30b50fe607eb90739a4aa9b86e02acd2ac16824da2260c7f957ed3bb405",
    "bytes": 81418
  },
  "frontend_tests": {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/source13-frontend-tests-v3/proof.json",
    "sha256": "f7fe8e35b7d8bd0d2dd5a17f89b2e47e267c13320ac35ddc760f69e70764ec8a",
    "bytes": 6162
  }
};
const HASH=/^[a-f0-9]{64}$/,COMMIT=/^[a-f0-9]{40}$/;
const sha=b=>createHash('sha256').update(b).digest('hex');
const normalized=p=>path.resolve(p).toLowerCase();
const sort=(a,b)=>a<b?-1:a>b?1:0;
const profile=Object.freeze({opt_level:1,debug_assertions:true,overflow_checks:true,debug_info:0,environment:{CARGO_PROFILE_TEST_OPT_LEVEL:'1',CARGO_PROFILE_TEST_DEBUG_ASSERTIONS:'true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS:'true',CARGO_PROFILE_TEST_DEBUG:'0',CARGO_PROFILE_DEV_DEBUG:'0'}});
const gates=['live_browser_worker_executes_javascript_for_core_crawl','live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness','live_browser_worker_pause_drains_render_and_retains_rendered_discoveries','live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion'];
export function relativeFile(value){assert.equal(typeof value,'string');assert(value&&!value.includes('\\')&&!value.includes(':')&&!path.posix.isAbsolute(value));assert(value.split('/').every(p=>p&&p!=='.'&&p!=='..'));return value;}
function sameRef(a,b){assert(a&&b);assert.equal(normalized(a.path),normalized(b.path));assert.equal(a.sha256,b.sha256);if(b.bytes!==undefined)assert.equal(a.bytes,b.bytes);}
function sameRoles(actual,expected){assert(Array.isArray(actual));assert.equal(actual.length,expected.length);for(const [i,r]of expected.entries()){assert.deepEqual(Object.keys(actual[i]).sort(),Object.keys(r).sort());assert.equal(actual[i].role,r.role);sameRef(actual[i].manifest,r.manifest);sameRef(actual[i].independent_review,r.independent_review);if(r.ancestor_independent_review)sameRef(actual[i].ancestor_independent_review,r.ancestor_independent_review);}}
function sameRoleReferences(actual,expected,field){assert(Array.isArray(actual));assert.equal(actual.length,expected.length);for(const [i,r]of expected.entries()){assert.equal(actual[i].role,r.role);sameRef(actual[i],r[field]);}}
function sameNodeCases(actual,expected){assert.equal(actual.length,expected.length);for(const [i,e]of expected.entries()){assert.equal(actual[i].role,e.role);assert.equal(actual[i].status,e.status);assert.deepEqual(actual[i].names,e.names);sameRef(actual[i].source,e.source);}}
function actualClose(v){assert.equal(v.actual_close_event,true);assert(Number.isSafeInteger(v.pid)&&v.pid>0);assert.deepEqual(v.exit,{code:0,signal:null});}
function actualPipes(v){assert.deepEqual(v.io_errors,[]);for(const side of ['stdout','stderr']){assert.equal(v.streams[side].end,true);assert.equal(v.streams[side].close,true);assert(Number.isSafeInteger(v.streams[side].bytes)&&v.streams[side].bytes>=0);}}
function rowsMap(rows){assert(Array.isArray(rows));const m=new Map();for(const f of rows){relativeFile(f.path);assert.deepEqual(Object.keys(f).sort(),['bytes','path','sha256']);assert.match(f.sha256,HASH);assert(Number.isSafeInteger(f.bytes)&&f.bytes>=0);assert(!m.has(f.path));m.set(f.path,f);}assert.deepEqual([...m.keys()],[...m.keys()].sort(sort));assert.equal(new Set([...m.keys()].map(p=>p.toLowerCase())).size,m.size);return m;}
function applyOps(before,ops,rel){let current=before;for(const o of ops){assert.equal(o.path,rel);assert(Number.isSafeInteger(o.offset)&&o.offset>=0&&o.offset<=current.length);assert.equal(sha(current),o.before_sha256);const old=Buffer.from(o.before_base64,'base64'),next=Buffer.from(o.after_base64,'base64');assert(current.subarray(o.offset,o.offset+old.length).equals(old));current=Buffer.concat([current.subarray(0,o.offset),next,current.subarray(o.offset+old.length)]);assert.equal(sha(current),o.after_sha256);}let inverse=current;for(const o of [...ops].reverse()){assert.equal(sha(inverse),o.after_sha256);const old=Buffer.from(o.before_base64,'base64'),next=Buffer.from(o.after_base64,'base64');assert(inverse.subarray(o.offset,o.offset+next.length).equals(next));inverse=Buffer.concat([inverse.subarray(0,o.offset),old,inverse.subarray(o.offset+next.length)]);assert.equal(sha(inverse),o.before_sha256);}assert(inverse.equals(before));return current;}
function fnNames(source){return [...source.matchAll(/#\[(?:tokio::)?test(?:\([^\]]*\))?\]\s*(?:async\s+)?fn\s+(\w+)\s*\(/g)].map(m=>m[1]);}
function noFindings(review){assert.equal(review.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(review.findings,[]);}


const CURRENT={
  "parent_manifest": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-source-v12\\manifest.json",
    "sha256": "3b62e68b32b2d23e375977cac8d5d0dad313c8f144eb43b4ab2896fae7687b18",
    "bytes": 754464
  },
  "parent_suite": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-suite-v12\\proof.json",
    "sha256": "8bba7750a13d0e7ec1de843d7599b0aab48e9ac5cffdf24eff6f0e26618d27d6",
    "bytes": 658399
  },
  "parent_build": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-optimized-build-v12\\proof.json",
    "sha256": "7512cfdc9e4f09d5c847c6ac89fa92face47172d5a8c673df0a2bbd67f6b46f4",
    "bytes": 100643
  },
  "old_388_driver": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-v13-driver-preparation-v2\\manifest.json",
    "sha256": "6ab448c737f35069d45b447e657af7d9f5fa1abaea4db5abeed89423976512ab",
    "bytes": 8622
  },
  "old_388_driver_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-v13-driver-independent-review-v2\\receipt.json",
    "sha256": "e505c5478b3acd8ae7595f44614e7c996364ccbfd50ec4089d9bf4431b44ee06",
    "bytes": 1813
  },
  "old_388_root_contract": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-v13-driver-preparation-v2\\ROOT-CONTRACT.md",
    "sha256": "73c7e323a38e91f1060cbaf0dbae85e0f7800def78498bbbff0c50a3ba5a4b26",
    "bytes": 6089
  },
  "d3_composition": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-source-v13-draft-v3\\preformat.json",
    "sha256": "13547987d8f394701cdd898f667f0f556f4fdf1b41895354df6cd1254ac2bfcf",
    "bytes": 245980
  },
  "d3_composition_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source13-composition-independent-review-v3\\receipt.json",
    "sha256": "098a54f49c41681ae152287db511296e55f13d6b2813237ba494498468e87116",
    "bytes": 5028
  },
  "api4": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-enrichment-proposal-v4\\manifest.json",
    "sha256": "fd02497bd24a034f2c6ef6374d9ab60d9d4af6d01a24e0a53b5e0dbbfdb1c994",
    "bytes": 22331
  },
  "api4_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-enrichment-independent-review-v4\\receipt.json",
    "sha256": "1a8f757c76472737140387dff140dfe983c492fc508b1d9aa6e25abd5f200f0f",
    "bytes": 2746
  },
  "current_composition": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-composed-preview-v3\\preformat.json",
    "sha256": "84d54108e57430bca1481fc499c566e7cdbb61770d1462feeea990047acc30ec",
    "bytes": 163396
  },
  "current_composition_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-composed-preview-root-independent-review-v3\\receipt.json",
    "sha256": "87ee1aee040d152ea6bdceea696824957ba8adab508ab436444d963ce9aef542",
    "bytes": 1136
  },
  "current_derivation": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-composed-preview-v3\\composition-derivation.json",
    "sha256": "59a91901408703554ff5e115e432fc9e09c1b9e65619b7337379564bc76fdb5f",
    "bytes": 87638
  },
  "current_test_inventory": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-composed-preview-v3\\test-inventory.json",
    "sha256": "2db49deec9ea68f3254fd845f8eceb500d67ccec7aabc121c5f276db91d7533b",
    "bytes": 172011
  },
  "preview_producers": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-preview-driver-preparation-v3\\manifest.json",
    "sha256": "efe3abd0145059855ab55aa8d55ee060017b6f3f7aeaa95bcbd1625ec8ee15f6",
    "bytes": 80071
  },
  "preview_producer_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-preview-driver-root-independent-review-v3\\receipt.json",
    "sha256": "46a09523cc95a5da622bb5b621c2ee898fc2b141c46742c7ecc72fb315634522",
    "bytes": 1745
  },
  "actual_format": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\api-only-preview-format-v3\\proof.json",
    "sha256": "6b06c2d4715c71c6232de12644b01914d92014bc4515a716b134d3eeaba98ffb",
    "bytes": 131387
  },
  "formatter_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-preview-format-independent-review-v3\\receipt.json",
    "sha256": "7a7f87efd7471ea4335fb2c7094e89c927d301ce28221909b7f789990138eb26",
    "bytes": 77140
  },
  "actual_clippy": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\api-only-preview-clippy-v3\\proof.json",
    "sha256": "942a81197f08cc41d42ff6224d4792ab2547c9255711fd1214b4acd5a9ff2855",
    "bytes": 80956
  },
  "actual_twenty_tests": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\api-only-preview-twenty-rust-tests-v3\\proof.json",
    "sha256": "12d671faf3da1c6b990ae121aaab849be01c7b0192ebcedbcf63e6ed73f863ba",
    "bytes": 91484
  },
  "current_runtime_manifest": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\browser-runtime\\source12-runtime2\\runtime-manifest.json",
    "sha256": "2dd2e6f599eb0aa4586fab3edd7de406929f484dd4d0c0229e88a2f0a55ea06f",
    "bytes": 83687
  },
  "current_runtime_artifact_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source12-current-runtime-manifest-independent-review-v1\\receipt.json",
    "sha256": "a42061eb1517b60683a57ea00e148573e3320f028298ac4eacfa72f63997fe5f",
    "bytes": 2576
  },
  "parent_renderer_four": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\renderer-four-compatibility-v12-runtime2\\proof.json",
    "sha256": "62c3e6934dd4c716819776f0d3336cdcbbf3c52a3804e00b7e47676c0e956281",
    "bytes": 22363
  },
  "runtime_bridge_preparation": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source12-historical-current-runtime-bridge-preparation-v3\\manifest.json",
    "sha256": "b09da50b5dfb3d5b1a92dc9d803a161767f9b515c6d5c3b1f4e4e6f093a33e10",
    "bytes": 10585
  },
  "runtime_bridge_source_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source12-historical-current-runtime-bridge-independent-review-v3\\receipt.json",
    "sha256": "d897f80540c9bf14d9804ff62bd6ecebb03f72fa4c7ce10749d0c521f97a0b55",
    "bytes": 2552
  },
  "runtime_contract": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source12-historical-current-runtime-bridge-preparation-v3\\runtime-contract.mjs",
    "sha256": "e43c083d1988167440e6f3d02e955d74bf31f12618526c6da4bed06bf887241d",
    "bytes": 17212
  },
  "old_388_blueprint": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-v13-driver-preparation-v2\\ROOT-APPROVAL-BLUEPRINT.json",
    "sha256": "9df83c6c1f272cf2abafc63149b08a9384aa95ae0266fd53e4745f6baf0e9796",
    "bytes": 82772
  },
  "api_frontend_tests": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\api-only-frontend-tests-v1\\proof.json",
    "sha256": "7a6c2d19b522d6669c0b029bb814292d8e0b71efd6e52e85979450869790ac4e",
    "bytes": 3314
  }
};
const PARENT_CORE={
  "source_manifest_sha256": "3b62e68b32b2d23e375977cac8d5d0dad313c8f144eb43b4ab2896fae7687b18",
  "suite_proof_sha256": "8bba7750a13d0e7ec1de843d7599b0aab48e9ac5cffdf24eff6f0e26618d27d6",
  "build_proof_sha256": "7512cfdc9e4f09d5c847c6ac89fa92face47172d5a8c673df0a2bbd67f6b46f4",
  "candidate_binary_sha256": "f7ce325cf73d16f1c4943ebf98dc8a2e888e93ee25c999bc7f2c10d1b1a1a65c",
  "candidate_bytes": 43650560,
  "build_driver_sha256": "95c7ac504e12d80c874804149e3a8d56a68435554a4fb2fac26ca2ee88ce95d0",
  "source_commit": "5d50a10f3974c9afe79909b963427e67dd8fb001"
};
const ALL_ROLE_PINS=[
  {
    "role": "raw_http_head",
    "manifest": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/html-head-diagnostics-proposal-v3/manifest.json",
      "sha256": "cb09004da70e0e7fd240029bb269d7c960a3567409c6e58783ab87901975c83c",
      "bytes": 92171
    },
    "independent_review": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/html-head-diagnostics-independent-review-v3/receipt.json",
      "sha256": "65d66a7fb013771c8c0d45338d59a376510ac9c4bd525fef64f2ba834e3a6373",
      "bytes": 2925
    }
  },
  {
    "role": "cdn_classification",
    "manifest": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/cdn-classification-proposal-v2/manifest.json",
      "sha256": "d95340c661c9e4c745abb7a7a7606c8b718aaae2fc3b6277c7054e53d9d12406",
      "bytes": 91311
    },
    "independent_review": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\cdn-classification-independent-review-v2\\receipt.json",
      "sha256": "8bf84119f6e0bb903253f0f814c018fce94d2067b9ad852d4da6eb430a1de171",
      "bytes": 2109
    },
    "ancestor_independent_review": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/cdn-classification-independent-review-v1/receipt.json",
      "sha256": "a8290023fd7877f8d89e13f440ac1112b61f67dfc49664ef5861083ce9f18eaf",
      "bytes": 2705
    }
  },
  {
    "role": "cdn_legacy_test_fixture_repair",
    "manifest": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\cdn-fetch-context-test-fixtures-proposal-v1\\manifest.json",
      "sha256": "671e60a75c1d60bb575f2e0b38955c573dc5cb0dbab50ac8ec8971a9acb90264",
      "bytes": 2844
    },
    "independent_review": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\cdn-fetch-context-test-fixtures-independent-review-v1\\receipt.json",
      "sha256": "b149a8899ad24c8593ad007505788ee012d3b3906e4611acfc0c594618c00506",
      "bytes": 2132
    }
  },
  {
    "role": "api_only_enrichment",
    "manifest": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-enrichment-proposal-v4\\manifest.json",
      "sha256": "fd02497bd24a034f2c6ef6374d9ab60d9d4af6d01a24e0a53b5e0dbbfdb1c994",
      "bytes": 22331
    },
    "independent_review": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-enrichment-independent-review-v4\\receipt.json",
      "sha256": "1a8f757c76472737140387dff140dfe983c492fc508b1d9aa6e25abd5f200f0f",
      "bytes": 2746
    }
  },
  {
    "role": "api_mcp_inventory_fixture_compatibility",
    "manifest": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-mcp-inventory-fixtures-proposal-v1\\manifest.json",
      "sha256": "f5ed1436b1842076fe1e22f1f9f6c2e0417441e87c79e0b7c6c1063ac90982d6",
      "bytes": 3467
    },
    "independent_review": {
      "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-mcp-inventory-fixtures-independent-review-v1\\receipt.json",
      "sha256": "9610a920d73ba62d2fab1abcd377c3ad5b2a6b1a9f8f5ad12c6caf00334d6251",
      "bytes": 1785
    }
  }
];
const RUNTIME_APPROVAL_REFS={
  "current_runtime_manifest": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\browser-runtime\\source12-runtime2\\runtime-manifest.json",
    "sha256": "2dd2e6f599eb0aa4586fab3edd7de406929f484dd4d0c0229e88a2f0a55ea06f",
    "bytes": 83687
  },
  "current_runtime_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source12-current-runtime-manifest-independent-review-v1\\receipt.json",
    "sha256": "a42061eb1517b60683a57ea00e148573e3320f028298ac4eacfa72f63997fe5f",
    "bytes": 2576
  },
  "runtime_bridge_preparation": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source12-historical-current-runtime-bridge-preparation-v3\\manifest.json",
    "sha256": "b09da50b5dfb3d5b1a92dc9d803a161767f9b515c6d5c3b1f4e4e6f093a33e10",
    "bytes": 10585
  },
  "runtime_bridge_review": {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/source12-historical-current-runtime-bridge-independent-review-v3/receipt.json",
    "sha256": "d897f80540c9bf14d9804ff62bd6ecebb03f72fa4c7ce10749d0c521f97a0b55",
    "bytes": 2552
  },
  "parent_renderer_compatibility": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\renderer-four-compatibility-v12-runtime2\\proof.json",
    "sha256": "62c3e6934dd4c716819776f0d3336cdcbbf3c52a3804e00b7e47676c0e956281",
    "bytes": 22363
  }
};
const APPROVAL_EVIDENCE={
  "preformat_manifest": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-composed-preview-v3\\preformat.json",
    "sha256": "84d54108e57430bca1481fc499c566e7cdbb61770d1462feeea990047acc30ec",
    "bytes": 163396
  },
  "composition_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-composed-preview-root-independent-review-v3\\receipt.json",
    "sha256": "87ee1aee040d152ea6bdceea696824957ba8adab508ab436444d963ce9aef542",
    "bytes": 1136
  },
  "actual_format": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\api-only-preview-format-v3\\proof.json",
    "sha256": "6b06c2d4715c71c6232de12644b01914d92014bc4515a716b134d3eeaba98ffb",
    "bytes": 131387
  },
  "formatter_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-preview-format-independent-review-v3\\receipt.json",
    "sha256": "7a7f87efd7471ea4335fb2c7094e89c927d301ce28221909b7f789990138eb26",
    "bytes": 77140
  },
  "preliminary_clippy": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\api-only-preview-clippy-v3\\proof.json",
    "sha256": "942a81197f08cc41d42ff6224d4792ab2547c9255711fd1214b4acd5a9ff2855",
    "bytes": 80956
  },
  "directed_rust_tests": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\api-only-preview-twenty-rust-tests-v3\\proof.json",
    "sha256": "12d671faf3da1c6b990ae121aaab849be01c7b0192ebcedbcf63e6ed73f863ba",
    "bytes": 91484
  },
  "frontend_tests": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\api-only-frontend-tests-v1\\proof.json",
    "sha256": "7a6c2d19b522d6669c0b029bb814292d8e0b71efd6e52e85979450869790ac4e",
    "bytes": 3314
  },
  "d3_preformat_manifest": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-source-v13-draft-v3\\preformat.json",
    "sha256": "13547987d8f394701cdd898f667f0f556f4fdf1b41895354df6cd1254ac2bfcf",
    "bytes": 245980
  },
  "d3_composition_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source13-composition-independent-review-v3\\receipt.json",
    "sha256": "098a54f49c41681ae152287db511296e55f13d6b2813237ba494498468e87116",
    "bytes": 5028
  },
  "d3_directed_rust_tests": {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/source13-new-rust-tests-v1/proof.json",
    "sha256": "cca1c30b50fe607eb90739a4aa9b86e02acd2ac16824da2260c7f957ed3bb405",
    "bytes": 81418
  },
  "d3_frontend_tests": {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/source13-frontend-tests-v3/proof.json",
    "sha256": "f7fe8e35b7d8bd0d2dd5a17f89b2e47e267c13320ac35ddc760f69e70764ec8a",
    "bytes": 6162
  }
};
function verifyParentEvidence(manifest,pin){
 const task=TASK,source=path.join(BASE,'combined-source-v12'),repo=manifest.source_worktree;
 const suiteRoot=path.join(BASE,'results/combined-suite-v12'),buildRoot=path.join(BASE,'results/combined-optimized-build-v12');
 const driverPath="E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-v12-driver-preparation-v6\\run-combined-suite-v12.mjs",suiteDriverSha="415f61e5823b01ce6d10bf40c08a708015ee1188138c33d57107907998912271";
 const builderPath="E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-v12-driver-preparation-v6\\build-combined-optimized-v12.mjs",builderSha="95c7ac504e12d80c874804149e3a8d56a68435554a4fb2fac26ca2ee88ce95d0";
 pin(driverPath,suiteDriverSha);pin(builderPath,builderSha);pin("E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-suite-v12\\owned-worker.json","bb27fbd1c504c76d57725161b9daf9289f50b0c50f1f0039e4da893498dc6e58");
 const expectedManifestSha=PARENT_SHA,expectedSuiteProofSha=PARENT_SUITE_SHA;
 const authenticatedSuite=JSON.parse(pin(path.join(suiteRoot,'proof.json'),PARENT_SUITE_SHA));
 const lineage=authenticatedSuite.source_lineage,names=manifest.required_tests.flatMap(g=>g.names),minimumExpected=1428,expectedTestProfile=profile,rendererGates=gates;
 const expectedNewFullNames=manifest.all_required_full_test_names;
 const release={build_proof_sha256:PARENT_BUILD_SHA,candidate_binary_sha256:PARENT_BINARY_SHA};
 const suitePath=path.join(suiteRoot,'proof.json');
 const boundedRead=(file,max)=>{const st=fs.lstatSync(file);assert(st.isFile()&&!st.isSymbolicLink()&&st.size<=max);const bytes=fs.readFileSync(file);assert.equal(bytes.length,st.size);return pin(file,sha(bytes),max);};
 const parseSummaries=text=>[...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(m=>({state:m[1],passed:+m[2],failed:+m[3],ignored:+m[4]}));
const suiteCommands = [
  ['format', ['fmt', '--all', '--check']],
  ['workspace-live', ['test', '--workspace', '--locked', '--', '--include-ignored', '--test-threads=1']],
  ['clippy', ['clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings']],
];

function verifySuite() {
  assert.equal(sha(fs.readFileSync(driverPath)), suiteDriverSha, 'Exact actual suite producer required');
  const proofBytes = boundedRead(path.join(suiteRoot, 'proof.json'), 8 * 1024 * 1024);
  assert.equal(sha(proofBytes), expectedSuiteProofSha, 'Explicit actual suite proof hash required');
  const proof = JSON.parse(proofBytes);
  assert.equal(proof.kind, 'combined_full_suite_v12');
  assert.equal(proof.result, 'PASS_COMBINED_WORKSPACE_V12_REAL_RENDERER_FMT_CLIPPY');
  assert.equal(proof.driver_sha256, suiteDriverSha);
  assert.equal(proof.source_manifest_sha256, expectedManifestSha);
  assert.equal(path.resolve(proof.source_directory), path.resolve(source));
  assert.equal(path.resolve(proof.source_worktree), path.resolve(repo));
  assert.equal(proof.source_inputs, manifest.files.length);
  assert.deepEqual(proof.source_manifest_metadata, Object.fromEntries(Object.entries(manifest)
    .filter(([key]) => !['files', 'required_tests'].includes(key))));
  assert.deepEqual(proof.source_lineage, lineage);
  assert.deepEqual(proof.test_profile, expectedTestProfile);
  assert.equal(proof.required_test_names, names.length);
  assert.equal(proof.required_test_names_verified, true);
  assert.equal(proof.minimum_expected, minimumExpected);
  assert.equal(proof.owned_renderer_gates_verified, true);
  assert.deepEqual(proof.owned_renderer_gates, rendererGates);
  assert.equal(proof.owned_worker_closed, true);
  assert.equal(proof.owned_fixture_closed, true);
  assert.equal(proof.source_final_verified, true);
  assert.equal(proof.user_renderer_contacted, false);
  assert.deepEqual(proof.failures, []);
  for (const port of [proof.owned_worker_port, proof.owned_fixture_port]) {
    assert(Number.isInteger(port) && port >= 1 && port <= 65535 && ![8787, 8790].includes(port));
  }
  assert.equal(proof.runs.length, 3);
  const digests = [{path: 'proof.json', sha256: sha(proofBytes), bytes: proofBytes.length}];
  let workspaceText, workspaceSummary;
  for (const [index, [name, args]] of suiteCommands.entries()) {
    const run = proof.runs[index];
    assert.equal(run.name, name);
    assert.deepEqual(run.args, args);
    assert(Number.isInteger(run.pid) && run.pid > 0);
    assert.equal(run.actual_close_event, true);
    assert.equal(run.spawn_error, null);
    assert.deepEqual(run.exit, {code: 0, signal: null});
    const log = boundedRead(path.join(suiteRoot, name + '.log'), 64 * 1024 * 1024);
    assert.equal(sha(log), run.log_sha256, 'Raw suite log SHA mismatch: ' + name);
    assert.equal(log.length, run.log_bytes);
    const summaries = parseSummaries(log.toString('utf8'));
    assert.deepEqual(summaries, run.summaries, 'Reparse actual suite summaries: ' + name);
    const json = boundedRead(path.join(suiteRoot, name + '.json'), 8 * 1024 * 1024);
    assert.deepEqual(JSON.parse(json), run, 'Actual individual child-close receipt differs');
    digests.push({path: name + '.log', sha256: sha(log), bytes: log.length},
      {path: name + '.json', sha256: sha(json), bytes: json.length});
    if (name === 'workspace-live') { workspaceText = log.toString('utf8'); workspaceSummary = summaries; }
  }
  assert(workspaceSummary.length > 0, 'A PASS label with no actual tests is rejected');
  assert(workspaceSummary.every(summary => summary.state === 'ok'));
  const passed = workspaceSummary.reduce((n, x) => n + x.passed, 0);
  const failed = workspaceSummary.reduce((n, x) => n + x.failed, 0);
  const ignored = workspaceSummary.reduce((n, x) => n + x.ignored, 0);
  assert(passed >= minimumExpected);
  assert.equal(failed, 0); assert.equal(ignored, 0);
  assert.equal(proof.passed, passed); assert.equal(proof.failed, failed); assert.equal(proof.ignored, ignored);
  const seen = new Map();
  for (const match of workspaceText.matchAll(/^test ([^\r\n]+?) \.\.\. (ok|FAILED|ignored[^\r\n]*)\r?$/gm)) {
    const leaf = match[1].split('::').at(-1);
    if (!seen.has(leaf)) seen.set(leaf, []);
    seen.get(leaf).push({full: match[1], status: match[2]});
  }
  const rows = [...names, ...rendererGates].map(name => {
    const found = seen.get(name) ?? [];
    assert.equal(found.length, 1, 'Missing/duplicate actual suite test: ' + name);
    assert.equal(found[0].status, 'ok', 'Actual suite test not green: ' + name);
    return {required: name, actual: found[0].full, status: found[0].status};
  });
  for (const expected of expectedNewFullNames) {
    const found = seen.get(expected.split('::').at(-1)) ?? [];
    assert.equal(found.length, 1, 'New authored full-module result missing/duplicate: ' + expected);
    assert.equal(found[0].full, expected, 'New authored function executed in wrong Cargo module');
    assert.equal(found[0].status, 'ok');
  }
  const nameBytes = boundedRead(path.join(suiteRoot, 'required-test-results.json'), 8 * 1024 * 1024);
  assert.deepEqual(JSON.parse(nameBytes), {required_names: names.length, renderer_names: 4,
    results: rows, passed, failed, ignored});
  digests.push({path: 'required-test-results.json', sha256: sha(nameBytes), bytes: nameBytes.length});
  const workerBytes = boundedRead(path.join(suiteRoot, 'owned-worker.json'), 8 * 1024 * 1024);
  const worker = JSON.parse(workerBytes);
  assert.equal(worker.origin, 'http://127.0.0.1:' + proof.owned_worker_port);
  assert.equal(worker.fixture_origin, 'http://127.0.0.1:' + proof.owned_fixture_port);
  assert.equal(worker.health.browser.available, true);
  assert.equal(worker.warmup.title, 'Owned ready');
  assert.equal(worker.warmup.status, 200);
  assert.equal(worker.warmup.final_url, worker.fixture_origin + '/');
  assert.equal(worker.user_renderer_contacted, false);
  assert.equal(path.resolve(worker.renderer_runtime_root), path.resolve(task, 'outputs/open-seo-spider'));
  assert.deepEqual(worker.renderer_input_hashes, manifest.files.filter(file => file.path.startsWith('renderer/')));
  assert.equal(worker.renderer_input_hashes.length, 4);
  for (const file of worker.renderer_input_hashes) {
    const bytes = boundedRead(path.join(source, file.path), 64 * 1024 * 1024);
    assert.equal(bytes.length, file.bytes); assert.equal(sha(bytes), file.sha256);
  }
  assert.equal(path.resolve(worker.chrome_path), path.resolve('C:/Program Files/Google/Chrome/Application/chrome.exe'));
  assert.match(worker.chrome_sha256, /^[a-f0-9]{64}$/);
  assert.equal(worker.chrome_sha256,"9ca2beaa7a9fd1109f01e562788c22e045a01cec0f6015ade689325af7f8a521");
  digests.push({path: 'owned-worker.json', sha256: sha(workerBytes), bytes: workerBytes.length});
  return {proof_sha256: sha(proofBytes), source_manifest_sha256: expectedManifestSha,
    suite_driver_sha256: suiteDriverSha, passed, failed, ignored, required_names: names.length, test_profile: expectedTestProfile, source_lineage: lineage,
    renderer_gates: 4, actual_three_closes: true, owned_resources_closed: true,
    new_authored_full_names: expectedNewFullNames, new_authored_full_names_verified: true,
    final_sources_verified: true, independently_reparsed_logs: true, digests};
}

 const admittedSuite=verifySuite();
function verifyBuild() {
  assert.equal(sha(boundedRead(builderPath, 1024 * 1024)), builderSha);
  const raw = boundedRead(path.join(buildRoot, 'proof.json'), 8 * 1024 * 1024), build = JSON.parse(raw);
  assert.equal(sha(raw), release.build_proof_sha256);
  assert.equal(build.kind, 'combined_optimized_build_v12');
  assert.equal(build.result, 'PASS_COMBINED_OPTIMIZED_BUILD_V12');
  assert.equal(build.builder_sha256, builderSha);
  assert.equal(build.explicit_suite_proof_sha256,expectedSuiteProofSha);
  assert.equal(build.source_manifest_sha256, expectedManifestSha);
  assert.equal(build.source_commit, manifest.source_commit);
  assert.equal(path.resolve(build.source_directory), path.resolve(source));
  assert.equal(path.resolve(build.source_worktree), path.resolve(repo));
  assert.equal(build.source_inputs, manifest.files.length);
  assert.deepEqual(build.source_lineage, lineage);
  assert.equal(build.source_final_verified, true); assert.equal(build.suite_final_verified, true);
  assert.deepEqual(build.failures, []);
  assert.deepEqual(build.required_suite, {path: suitePath, ...admittedSuite});
  assert.deepEqual(build.compiler_profile, {mode: 'optimized', lto: 'thin', codegen_units: 1, strip: true,
    declared_in: 'Cargo.toml', external_profile_overrides_rejected: true});
  const expected = [
    ['cargo-version', 'C:/Users/matth/.cargo/bin/cargo.exe', ['--version', '--verbose']],
    ['rustc-version', 'C:/Users/matth/.cargo/bin/rustc.exe', ['--version', '--verbose']],
    ['build', 'C:/Users/matth/.cargo/bin/cargo.exe', ['build', '--release', '-p', 'open-seo-spider', '--locked']],
  ];
  assert.equal(build.runs.length, 3);
  const digests = [{path: 'proof.json', sha256: sha(raw), bytes: raw.length}];
  for (const [i, [name, executable, args]] of expected.entries()) {
    const run = build.runs[i]; assert.equal(run.name, name); assert.deepEqual(run.args, args);
    assert.equal(run.executable, executable); assert(Number.isSafeInteger(run.pid) && run.pid > 0);
    assert.equal(run.actual_close_event, true); assert.equal(run.spawn_error, null);
    assert.deepEqual(run.exit, {code: 0, signal: null});
    const log = boundedRead(path.join(buildRoot, name + '.log'), 64 * 1024 * 1024);
    assert.equal(sha(log), run.log_sha256); assert.equal(log.length, run.log_bytes);
    const json = boundedRead(path.join(buildRoot, name + '.json'), 8 * 1024 * 1024);
    assert.deepEqual(JSON.parse(json), run);
    if (name === 'build') assert(/Finished\s+`release`\s+profile\s+\[optimized\]/.test(log.toString('utf8')));
    digests.push({path: name + '.log', sha256: sha(log), bytes: log.length}, {path: name + '.json', sha256: sha(json), bytes: json.length});
  }
  const binary = path.join(buildRoot, 'candidate/open-seo-spider.exe');
  assert.equal(path.resolve(build.binary.path), path.resolve(binary));
  const bytes = boundedRead(binary, 256 * 1024 * 1024), hash = sha(bytes);
  assert.equal(hash, build.binary.sha256); assert.equal(hash, release.candidate_binary_sha256);
  assert.equal(bytes.length, build.binary.bytes);
  assert(bytes.length >= 64 && bytes[0] === 0x4d && bytes[1] === 0x5a);
  const pe = bytes.readUInt32LE(60);
  assert(pe <= bytes.length - 4 && bytes.subarray(pe, pe + 4).equals(Buffer.from([0x50, 0x45, 0, 0])));
  return {proof_sha256: sha(raw), binary_path: binary, binary_sha256: hash, binary_bytes: bytes.length, digests};
}

 const admittedBuild=verifyBuild();assert.equal(admittedBuild.binary_sha256,PARENT_BINARY_SHA);assert.equal(admittedBuild.binary_bytes,PARENT_BINARY_BYTES);
 return{...admittedSuite,full_names:expectedNewFullNames,build:admittedBuild,parent_renderer_files_scope:'immutable_frozen_source12_only',parent_chrome_identity_scope:'actual_authenticated_parent_worker_capture_only'};
}



function verifyD3Evidence({data,dataLog,pin,repo,baseline,parentFiles,parentRaw,parentManifestRef}){
 const oldBlueprint=data(CURRENT.old_388_blueprint),d3format=data(FIXED.actual_format),a={...FIXED,parent_manifest:parentManifestRef,source_files:d3format.source_files.map(({path,after_sha256,bytes})=>({path,sha256:after_sha256,bytes})),files:oldBlueprint.files};
 const pre=data(a.preformat_manifest),composition=data(a.composition_review);noFindings(composition);sameRef(composition.proposal_manifest,a.preformat_manifest);sameRoles(composition.source_proposals,ROLE_PINS);
 assert.equal(pre.kind,'combined_source_v13_head_cdn_source_draft');assert.equal(pre.status,'COMPOSED_SOURCE_ONLY_THREE_REVIEWED_LEGACY_FIXTURES_REPAIRED');
 sameRef(pre.parent_manifest,a.parent_manifest);assert.equal(pre.parent_source_commit,baseline.source_commit);sameRoles(pre.source_proposals,ROLE_PINS);assert.equal(pre.root_mutated,false);
 assert(path.isAbsolute(pre.source_directory)&&path.isAbsolute(pre.preformat_directory));
 assert.notEqual(normalized(pre.source_directory),normalized(pre.preformat_directory));assert.notEqual(normalized(pre.source_directory),normalized(repo));
 const authoredMap=rowsMap(pre.files),authored=[...authoredMap.values()];assert.equal(authored.length,388);
 const owned=new Map(),rolePayloads=new Map();
 for(const role of ROLE_PINS){
  const m=data(role.manifest),review=data(role.independent_review);noFindings(review);sameRef(review.proposal_manifest,role.manifest);
  if(role.ancestor_independent_review)noFindings(data(role.ancestor_independent_review));
  assert(path.isAbsolute(m.source_directory));
  for(const f of m.files){relativeFile(f.path);assert(!/NOT.IN.CARGO|BENCHMARK|benchmark/i.test(f.path));const raw=pin(path.join(m.source_directory,f.path),f.sha256);assert.equal(raw.length,f.bytes);rolePayloads.set(role.role+'\0'+f.path,raw);}
 }
 for(const o of pre.owned_changes){
  relativeFile(o.path);assert(!owned.has(o.path));assert(Array.isArray(o.roles)&&o.roles.length>0);assert.equal(new Set(o.roles).size,o.roles.length);
  for(const role of o.roles)assert(rolePayloads.has(role+'\0'+o.path));
  const before=parentFiles.get(o.path);assert.equal(o.before_sha256,before?.sha256??'ABSENT');assert.equal(o.before_bytes,before?.bytes??0);
  assert.equal(o.sha256,authoredMap.get(o.path)?.sha256);assert.equal(o.bytes,authoredMap.get(o.path)?.bytes);owned.set(o.path,o);
 }
 const declaredUnion=new Set([...rolePayloads.keys()].map(k=>k.split('\0')[1]));assert.deepEqual([...owned.keys()].sort(sort),[...declaredUnion].sort(sort));
 const counts={owned:owned.size,modified:[...owned.values()].filter(o=>parentFiles.has(o.path)).length,new:[...owned.values()].filter(o=>!parentFiles.has(o.path)).length};
 counts.inputs=parentFiles.size+counts.new;counts.unchanged=parentFiles.size-counts.modified;
 assert.deepEqual(counts,{owned:21,modified:14,new:7,inputs:388,unchanged:367});
 assert.equal(pre.source_inputs,counts.inputs);assert.equal(pre.owned_paths,counts.owned);assert.equal(pre.changed_parent_inputs,counts.modified);assert.equal(pre.new_input_files,counts.new);assert.equal(pre.unchanged_parent_inputs,counts.unchanged);
 assert.deepEqual([...authoredMap.keys()],[...new Set([...parentFiles.keys(),...owned.keys()])].sort(sort));
 // The reviewed two-arm merge has shared insertion anchors. Replay its exact
 // whole-state operations in Head->CDN order, then the three fixture operations.
 const repair=data(pre.composition_derivation);sameRef(composition.composition_derivation,pre.composition_derivation);
 sameRef(repair.fixture_manifest,ROLE_PINS[2].manifest);sameRef(repair.fixture_review,ROLE_PINS[2].independent_review);
 const previous=data(repair.previous_composition),previousReview=data(repair.previous_composition_review);noFindings(previousReview);sameRef(previousReview.proposal_manifest,repair.previous_composition);
 sameRef(pre.previous_composition,repair.previous_composition);sameRef(pre.previous_composition_review,repair.previous_composition_review);
 const merge=data(repair.previous_complete_composition_derivation);sameRef(previous.composition_derivation,repair.previous_complete_composition_derivation);sameRef(merge.parent_manifest,a.parent_manifest);
 assert.deepEqual(merge.role_order,['raw_http_head','cdn_classification']);assert.equal(merge.operations.length,41);assert.equal(repair.operations.length,3);
 const allOps=[...merge.operations,...repair.operations],expected=new Map();
 assert(allOps.every(o=>parentRaw.has(o.path)&&owned.has(o.path)));
 for(const f of authored){
  const raw=dataLog({path:path.join(pre.preformat_directory,f.path),sha256:f.sha256,bytes:f.bytes});
  if(parentRaw.has(f.path)){
   const steps=allOps.filter(o=>o.path===f.path);if(owned.has(f.path)){assert(steps.length>0);assert(applyOps(parentRaw.get(f.path),steps,f.path).equals(raw));}else{assert.equal(steps.length,0);assert(raw.equals(parentRaw.get(f.path)));}
  }else{
   const o=owned.get(f.path);assert(o&&o.roles.length===1);assert(raw.equals(rolePayloads.get(o.roles[0]+'\0'+f.path)));
  }
  expected.set(f.path,raw);
 }
 const inventory=data(pre.test_inventory);sameRef(inventory.parent_manifest,a.parent_manifest);assert.deepEqual(inventory.new_authored_test_functions,pre.new_authored_test_functions);
 const newFunctions=pre.new_authored_test_functions.map(t=>{
  assert.equal(t.windows,true);assert.equal(t.unix,true);assert.match(t.name,/^[A-Za-z_][A-Za-z0-9_]*$/);assert(t.full_name.endsWith('::'+t.name));assert(owned.get(t.path)?.roles.includes(t.role));assert.equal(fnNames(expected.get(t.path).toString()).filter(n=>n===t.name).length,1);
  return{path:t.path,name:t.name,full_name:t.full_name,windows:true,unix:true};
 }).sort((a,b)=>sort(a.path,b.path)||sort(a.name,b.name));
 assert.equal(newFunctions.length,28);assert.equal(new Set(newFunctions.map(t=>t.name)).size,28);assert.equal(new Set(newFunctions.map(t=>t.full_name)).size,28);
 const requiredTests=structuredClone(baseline.required_tests);
 for(const t of newFunctions){let g=requiredTests.find(g=>g.path===t.path);if(!g){g={path:t.path,names:[]};requiredTests.push(g);}assert(!g.names.includes(t.name));g.names.push(t.name);}
 for(const g of requiredTests)g.names.sort(sort);requiredTests.sort((a,b)=>sort(a.path,b.path));
 const required=requiredTests.flatMap(g=>g.names),full=[...baseline.all_required_full_test_names,...newFunctions.map(t=>t.full_name)].sort(sort),minimum=baseline.minimum_expected_workspace_tests+newFunctions.length;
 assert.equal(required.length,674);assert.equal(new Set(required).size,674);assert.equal(full.length,678);assert.equal(new Set(full).size,678);assert.equal(minimum,1456);
 assert.deepEqual(pre.required_tests,requiredTests);assert.deepEqual(inventory.required_tests,requiredTests);assert.deepEqual(pre.all_required_full_test_names,full);assert.deepEqual(inventory.all_required_full_test_names,full);assert.equal(pre.required_test_names,required.length);assert.equal(pre.minimum_expected_workspace_tests,minimum);
 const final=rowsMap(a.source_files);assert.equal(final.size,388);assert.deepEqual([...final.keys()],[...authoredMap.keys()]);
 const allFormatRows=[],bindings=[];
 for(const f of authored){
  const after=final.get(f.path),formatted=dataLog({path:path.join(pre.source_directory,f.path),sha256:after.sha256,bytes:after.bytes});
  if(owned.has(f.path)){const o=owned.get(f.path);bindings.push({roles:o.roles,path:f.path,before_sha256:parentFiles.get(f.path)?.sha256??null,before_bytes:parentFiles.get(f.path)?.bytes??null,authored_sha256:f.sha256,authored_bytes:f.bytes,after_sha256:after.sha256,bytes:after.bytes});}else assert.deepEqual(after,parentFiles.get(f.path));
  if(!expected.get(f.path).equals(formatted))assert(owned.has(f.path)&&f.path.endsWith('.rs'),'Only owned Rust may be formatted');
  allFormatRows.push({path:f.path,before_sha256:f.sha256,before_bytes:f.bytes,after_sha256:after.sha256,bytes:after.bytes});
 }
 assert.deepEqual(a.files,bindings);
 const changedFormatRows=allFormatRows.filter(f=>f.before_sha256!==f.after_sha256),formatChanges=changedFormatRows.map(({before_bytes,...r})=>r);
 const fmt=data(a.actual_format);assert.equal(fmt.kind,'actual_source13_preliminary_format');assert.equal(fmt.status,'PASS_ACTUAL_CARGO_FMT');sameRef(fmt.preformat_manifest,a.preformat_manifest);
 assert(Number.isSafeInteger(fmt.actual_process.pid)&&fmt.actual_process.pid>0);assert.equal(fmt.actual_process.status,0);assert.equal(fmt.actual_process.signal,null);assert.equal(fmt.actual_process.error,null);assert.equal(fmt.actual_process.observation,'spawnSync returned after child and pipe completion');
 assert.equal(normalized(fmt.command.cwd),normalized(pre.source_directory));assert.equal(normalized(fmt.command.executable),normalized('C:/Users/matth/.cargo/bin/cargo.exe'));assert.deepEqual(fmt.command.args.slice(0,3),['fmt','--all','--manifest-path']);assert.equal(fmt.command.args.length,4);assert.equal(normalized(fmt.command.args[3]),normalized(path.join(pre.source_directory,'Cargo.toml')));
 assert.deepEqual(fmt.source_files,allFormatRows);assert.deepEqual(fmt.changed_files,changedFormatRows.map(f=>f.path));for(const side of ['stdout','stderr'])dataLog(fmt[side]);assert.equal(fmt.root_repo_mutated,false);
 const formatter=data(a.formatter_review);noFindings(formatter);sameRef(formatter.preformat_manifest,a.preformat_manifest);sameRef(formatter.actual_format_process_receipt,a.actual_format);sameRef(formatter.composition_review,a.composition_review);assert.deepEqual(formatter.files,formatChanges);assert.deepEqual(formatter.formatted_files,a.source_files);assert.equal(formatChanges.length,9);
 const clippy=data(a.preliminary_clippy);assert.equal(clippy.kind,'actual_source13_preliminary_clippy');assert.equal(clippy.status,'PASS_ACTUAL_PRELIMINARY_CLIPPY');sameRef(clippy.format,a.actual_format);assert.equal(normalized(clippy.source_directory),normalized(pre.source_directory));assert.equal(normalized(clippy.command.cwd),normalized(pre.source_directory));assert.equal(normalized(clippy.command.executable),normalized('C:/Users/matth/.cargo/bin/cargo.exe'));assert.deepEqual(clippy.command.args,['clippy','--workspace','--all-targets','--locked','--','-D','warnings']);assert.deepEqual(clippy.profile,profile.environment);
 const cp=clippy.actual_process;assert(Number.isSafeInteger(cp.pid)&&cp.pid>0);assert.equal(cp.spawned,true);assert.deepEqual(cp.exit,{code:0,signal:null});assert.deepEqual(cp.close,cp.exit);assert.equal(cp.error,null);assert.equal(cp.timed_out,false);assert.equal(clippy.executed_tests,0);assert.equal(clippy.source_final_verified,true);assert.deepEqual(clippy.source_files,a.source_files);assert.equal(clippy.root_repo_mutated,false);
 function closedLogs(proof){const logs={};for(const side of ['stdout','stderr']){const log=proof.logs[side];assert.equal(log.end,true);assert.equal(log.error,null);assert.equal(normalized(log.path),normalized(log.reference.path));logs[side]=dataLog(log.reference);assert.equal(logs[side].length,log.bytes);}return logs;}
 for(const raw of Object.values(closedLogs(clippy)))assert(!/^error(?:\[|:)/m.test(raw.toString()));
 // Both current supplemental gates bind this exact formatted union; neither
 // changes Cargo suite counts or substitutes for the later full workspace run.
 const directed=data(a.directed_rust_tests);assert.equal(directed.kind,'actual_source13_new_rust_function_regressions');assert.equal(directed.status,'PASS_ALL_28_NEW_RUST_FUNCTIONS_ONLY');sameRef(directed.preformat_manifest,a.preformat_manifest);sameRef(directed.actual_format,a.actual_format);sameRef(directed.preliminary_clippy,a.preliminary_clippy);assert.deepEqual(directed.source_files,a.source_files);assert.equal(normalized(directed.source_directory),normalized(pre.source_directory));assert.equal(normalized(directed.command.cwd),normalized(pre.source_directory));assert.equal(normalized(directed.command.executable),normalized('C:/Users/matth/.cargo/bin/cargo.exe'));
 const directedNames=pre.new_authored_test_functions.map(t=>t.full_name);
 assert.deepEqual(directed.command.args,['test','--workspace','--locked','--','--include-ignored','--test-threads=1','--exact',...directedNames]);assert.deepEqual(directed.profile,profile.environment);
 const dp=directed.process;assert(Number.isSafeInteger(dp.pid)&&dp.pid>0);assert.equal(dp.spawned,true);assert.deepEqual(dp.exit,{code:0,signal:null});assert.deepEqual(dp.close,dp.exit);assert.equal(dp.spawn_error,null);assert.equal(dp.timed_out,false);
 const dl=closedLogs(directed),actualDirected=[...dl.stdout.toString().matchAll(/^test (\S+) \.\.\. (ok|FAILED|ignored)\s*$/gm)].map(m=>({name:m[1],result:m[2]}));
 assert.deepEqual(actualDirected,directed.actual_test_results);assert.deepEqual(actualDirected.map(t=>t.name).sort(sort),[...directedNames].sort(sort));assert(actualDirected.every(t=>t.result==='ok'));assert.deepEqual(directed.expected_function_names,directedNames);assert.equal(directed.executed_test_functions,28);assert.equal(directed.all_required_new_names_exactly_once,true);assert.equal(directed.source_final_verified,true);assert.equal(directed.root_repo_mutated,false);assert.equal(directed.full_workspace_suite,'NOT_RUN_BY_THIS_GATE');
 const nodeCases=pre.additional_non_cargo_cases;assert.deepEqual(nodeCases,inventory.frontend);assert.equal(nodeCases.length,2);
 for(const n of nodeCases){const old=dataLog(n.original_source),current=dataLog(n.current_source);assert(old.equals(current));assert.equal(n.cases.length,3);assert.equal(new Set(n.cases).size,3);const names=[...current.toString().matchAll(/(?:test|it)\('([^']+)'/g)].map(m=>m[1]);assert.deepEqual(names,n.cases);for(const r of [n.actual_product_target,...n.additional_product_targets]){const rel=path.relative(pre.source_directory,r.path).replaceAll('\\','/');relativeFile(rel);assert.deepEqual({path:rel,sha256:r.sha256,bytes:r.bytes},final.get(rel));dataLog(r);}}
 const frontend=data(a.frontend_tests);assert.equal(frontend.kind,'actual_source13_frontend_regressions');assert.equal(frontend.result,'PASS_SIX_UNION_FRONTEND_SOURCE_CASES_ONLY');sameRef(frontend.preformat,a.preformat_manifest);sameRef(frontend.composition_review,a.composition_review);sameRef(frontend.actual_format,a.actual_format);sameRef(frontend.formatter_review,a.formatter_review);assert.deepEqual(frontend.node_sources,nodeCases);sameRef(frontend.actual_product_target,nodeCases[0].actual_product_target);
 assert.equal(normalized(frontend.command.executable),normalized('C:/Program Files/nodejs/node.exe'));assert.equal(normalized(frontend.command.cwd),normalized(path.dirname(a.preformat_manifest.path)));assert.deepEqual(frontend.command.args,['--test','--test-reporter=tap',...nodeCases.map(n=>n.current_source.path)]);
 const np=frontend.actual_process;assert(Number.isSafeInteger(np.pid)&&np.pid>0);assert.equal(np.status,0);assert.equal(np.signal,null);assert.equal(np.error,null);assert.equal(np.observation,'spawnSync returned after process and pipes completed');
 const tap=dataLog(frontend.stdout).toString();dataLog(frontend.stderr);const tapNames=[...tap.matchAll(/^ok \d+ - (.+)\r?$/gm)].map(m=>m[1]);const expectedCases=nodeCases.flatMap(n=>n.cases).sort(sort);assert.deepEqual(tapNames.sort(sort),expectedCases);assert.deepEqual(frontend.actual_passed_names,expectedCases);assert.equal(frontend.passed,6);assert.equal(frontend.failed,0);assert.equal(frontend.all388_inputs_final_unchanged,true);assert.equal(frontend.rust_tests_executed,0);assert.equal(frontend.browser_invocations,0);

 return{pre,sourceFiles:final,owned,newFunctions,nodeCases};
}
function textOps(before,ops,rel){let current=before;for(const o of ops){assert.equal(o.path,rel);assert.equal(sha(current),o.before_sha256);const x=Buffer.from(o.before),y=Buffer.from(o.after);assert(current.subarray(o.offset,o.offset+x.length).equals(x));current=Buffer.concat([current.subarray(0,o.offset),y,current.subarray(o.offset+x.length)]);assert.equal(sha(current),o.after_sha256);}let back=current;for(const o of [...ops].reverse()){assert.equal(sha(back),o.after_sha256);const y=Buffer.from(o.after);assert(back.subarray(o.offset,o.offset+y.length).equals(y));back=Buffer.concat([back.subarray(0,o.offset),Buffer.from(o.before),back.subarray(o.offset+y.length)]);assert.equal(sha(back),o.before_sha256);}assert(back.equals(before));return current;}
function binaryIdentity(r,cap){assert(path.isAbsolute(r.path)&&HASH.test(r.sha256));assert(Number.isSafeInteger(r.bytes)&&r.bytes>0&&r.bytes<=cap);const before=fs.lstatSync(r.path,{bigint:true});assert(before.isFile()&&!before.isSymbolicLink());assert.equal(before.size,BigInt(r.bytes));const fd=fs.openSync(r.path,'r'),hash=createHash('sha256'),buf=Buffer.alloc(1024*1024),keys=['dev','ino','size','mtimeNs','ctimeNs','birthtimeNs'];let total=0;try{const opened=fs.fstatSync(fd,{bigint:true});for(const k of keys)assert.equal(opened[k],before[k]);for(;;){const n=fs.readSync(fd,buf,0,Math.min(buf.length,r.bytes-total+1),null);if(!n)break;total+=n;assert(total<=r.bytes);hash.update(buf.subarray(0,n));}assert.equal(total,r.bytes);const ended=fs.fstatSync(fd,{bigint:true});for(const k of keys)assert.equal(ended[k],before[k]);}finally{fs.closeSync(fd);}const after=fs.lstatSync(r.path,{bigint:true});for(const k of keys)assert.equal(after[k],before[k]);assert.equal(hash.digest('hex'),r.sha256);}
function physicalRuntime(data,sourceFiles){
 const v=data(CURRENT.current_runtime_manifest),rr=data(CURRENT.current_runtime_artifact_review),prep=data(CURRENT.runtime_bridge_preparation),pr=data(CURRENT.runtime_bridge_source_review);noFindings(rr);noFindings(pr);
 assert.equal(v.kind,'source12_current_owned_runtime_manifest');assert.equal(v.status,'ROOT_CAPTURED_PINNED_RUNTIME_NOT_COMPATIBILITY');assert.equal(v.compatibility_proof,null);assert.deepEqual(v.core,PARENT_CORE);
 sameRef(v.preparation,CURRENT.runtime_bridge_preparation);sameRef(v.source_review,CURRENT.runtime_bridge_source_review);assert.equal(rr.kind,'source12_current_runtime_manifest_independent_review');sameRef(rr.proposal_manifest,CURRENT.current_runtime_manifest);sameRef(rr.capture_preparation,v.preparation);sameRef(rr.capture_source_review,v.source_review);sameRef(rr.capture_script,prep.runtime_capture);sameRef(pr.proposal_manifest,CURRENT.runtime_bridge_preparation);assert.equal(pr.kind,'source12_historical_current_runtime_bridge_independent_review');
 assert.equal(v.chrome.version,'155.0.8059.80');assert.equal(normalized(v.node.path),normalized(process.execPath));assert.equal(v.node.version,process.versions.node);binaryIdentity(v.node,128*1024*1024);
 const regular=p=>{let at=path.resolve(p);for(;;){assert(!fs.lstatSync(at).isSymbolicLink(),'Reparse path refused: '+at);const up=path.dirname(at);if(up===at)break;at=up;}};
 function tree(root,expected,maxFiles,maxTotal){assert(path.isAbsolute(root));regular(root);assert(normalized(root).startsWith(normalized(BASE)+path.sep)||normalized(root).startsWith(normalized(TASK)+path.sep));assert(expected.files.length<=maxFiles&&expected.total_bytes<=maxTotal);const files=[],dirs=[];let total=0;function visit(rel){const p=path.join(root,rel);regular(p);const st=fs.lstatSync(p);assert(!st.isSymbolicLink());if(st.isDirectory()){if(rel)dirs.push(rel.replaceAll('\\','/'));for(const n of fs.readdirSync(p).sort(sort))visit(path.join(rel,n));}else{assert(st.isFile());const key=rel.replaceAll('\\','/'),r=expected.files.find(f=>f.path===key);assert(r);assert.equal(st.size,r.bytes);assert(Number.isSafeInteger(r.bytes)&&r.bytes>=0&&r.bytes<=512*1024*1024);total+=r.bytes;assert(total<=maxTotal);if(r.bytes)binaryIdentity({path:p,sha256:r.sha256,bytes:r.bytes},512*1024*1024);else assert.equal(sha(fs.readFileSync(p)),r.sha256);files.push({path:key,sha256:r.sha256,bytes:r.bytes});assert(files.length<=maxFiles);}}visit('');files.sort((a,b)=>sort(a.path,b.path));dirs.sort(sort);assert.deepEqual(files,expected.files);assert.deepEqual(dirs,expected.dirs);assert.equal(total,expected.total_bytes);assert.equal(sha(Buffer.from(JSON.stringify(files))),expected.tree_sha256);}
 tree(v.chrome.root,v.chrome,1024,1024*1024*1024);sameRef(v.chrome.executable,{path:path.join(v.chrome.root,'chrome.exe'),sha256:v.chrome.files.find(f=>f.path==='chrome.exe').sha256,bytes:v.chrome.files.find(f=>f.path==='chrome.exe').bytes});
 assert.equal(v.renderer.packages.length,2);for(const p of v.renderer.packages){assert(['playwright-core','axe-core'].includes(p.name));assert.equal(p.version,p.name==='playwright-core'?'1.64.0':'4.14.0');tree(p.root,p,8192,64*1024*1024);const b=fs.readFileSync(p.package_json.path);assert.equal(b.length,p.package_json.bytes);assert.equal(sha(b),p.package_json.sha256);const j=JSON.parse(b);assert.equal(j.name,p.name);assert.equal(j.version,p.version);assert.equal(Object.keys(j.dependencies??{}).length,0);}
 const renderer=[...sourceFiles.values()].filter(f=>f.path.startsWith('renderer/'));assert.equal(renderer.length,4);assert.deepEqual(v.renderer.input_hashes,renderer);for(const f of renderer){const b=fs.readFileSync(path.join(v.renderer.runtime_root,f.path));assert.equal(b.length,f.bytes);assert.equal(sha(b),f.sha256);}
 // This is physical identity evidence captured for Source12, never a Source13 compatibility result.
 data(CURRENT.parent_renderer_four);return{current_runtime_manifest:CURRENT.current_runtime_manifest,current_runtime_review:CURRENT.current_runtime_artifact_review,capture_preparation:CURRENT.runtime_bridge_preparation,capture_source_review:CURRENT.runtime_bridge_source_review,parent_renderer_compatibility:CURRENT.parent_renderer_four,chrome:v.chrome.executable,chrome_version:v.chrome.version,node:v.node,tree_sha256:v.chrome.tree_sha256,renderer_inputs:renderer,scope:'PHYSICAL_RUNTIME_IDENTITY_ONLY_FRESH_SOURCE13_RENDERER_TESTS_REQUIRED'};
}
function closedPreview(p,pin){assert.equal(p.passed,true);const a=p.actual_process;assert(Number.isSafeInteger(a.pid)&&a.pid>0);assert.equal(a.spawned,true);assert.equal(a.spawn_error,null);assert.equal(a.timed_out,false);assert.equal(a.output_overflow,false);assert.deepEqual(a.exit,{code:0,signal:null});assert.deepEqual(a.close,a.exit);assert.deepEqual(p.cleanup_signals,[]);const logs={};assert.deepEqual(Object.keys(p.logs).sort(),['stderr','stdout']);for(const[k,l]of Object.entries(p.logs)){assert.equal(l.end,true);assert.equal(l.closed,true);assert.equal(l.error,null);assert.equal(normalized(l.path),normalized(l.reference.path));const b=pin(l.reference.path,l.reference.sha256,64*1024*1024);assert.equal(b.length,l.reference.bytes);assert.equal(b.length,l.bytes);assert.equal(b.length,l.stored_bytes);logs[k]=b.toString('utf8');}return logs;}
function sameFingerprints(a,b,pin){sameRef(a.node.executable,b.node.executable);assert.equal(a.node.version,b.node.version);assert.deepEqual(a.node.versions,b.node.versions);for(const k of['cargo','rustc']){sameRef(a[k].executable,b[k].executable);assert.equal(a[k].stdout,b[k].stdout);for(const f of[a,b]){sameRef(f[k].process.executable,f[k].executable);assert.equal(closedPreview(f[k].process,pin).stdout,f[k].stdout);}}}
function loadPredecessorEvidence({sourceDirectory,repo}){
 assert.equal(normalized(sourceDirectory),normalized(path.join(BASE,'combined-source-v13')));assert.equal(normalized(repo),normalized(path.join(BASE,'mcp-file-tools-v1')));const references=new Map();
 const pin=(p,expected,max=8*1024*1024)=>{assert(path.isAbsolute(p));p=path.resolve(p);assert.match(expected,HASH);const st=fs.lstatSync(p);assert(st.isFile()&&!st.isSymbolicLink()&&st.size<=max,'Bounded regular evidence required: '+p);const b=fs.readFileSync(p);assert.equal(b.length,st.size);assert.equal(sha(b),expected,p);const r={path:p,sha256:expected,bytes:b.length},key=normalized(p);if(references.has(key))assert.deepEqual(references.get(key),r);else references.set(key,r);return b;};
 const data=r=>{assert(r&&path.isAbsolute(r.path)&&Number.isSafeInteger(r.bytes)&&r.bytes>=0);const b=pin(r.path,r.sha256);assert.equal(b.length,r.bytes);return JSON.parse(b);},dataLog=r=>{const b=pin(r.path,r.sha256);assert.equal(b.length,r.bytes);return b;};
 const baselinePath=path.join(BASE,'combined-source-v12/manifest.json'),baseline=JSON.parse(pin(baselinePath,PARENT_SHA));assert.equal(baseline.kind,'combined_source_v12');assert.equal(baseline.source_commit,PARENT_CORE.source_commit);assert.deepEqual(baseline.test_profile,profile);assert.equal(baseline.source_inputs,381);assert.equal(baseline.required_test_names,646);assert.equal(baseline.minimum_expected_workspace_tests,1428);assert.equal(baseline.all_required_full_test_names.length,650);
 const parentFiles=rowsMap(baseline.files),parentRaw=new Map();assert.equal(parentFiles.size,381);for(const f of parentFiles.values()){const b=pin(path.join(BASE,'combined-source-v12',f.path),f.sha256);assert.equal(b.length,f.bytes);parentRaw.set(f.path,b);}const parentSuiteVerified=verifyParentEvidence(baseline,pin),parentSuiteReference=CURRENT.parent_suite,parentBuildReference=CURRENT.parent_build;
 const d3=verifyD3Evidence({data,dataLog,pin,repo,baseline,parentFiles,parentRaw,parentManifestRef:CURRENT.parent_manifest});
 const pre=data(CURRENT.current_composition),cr=data(CURRENT.current_composition_review);noFindings(cr);sameRef(cr.proposal_manifest,CURRENT.current_composition);assert.equal(pre.kind,'api_only_composed_400_input_preview');assert.equal(pre.source_inputs,400);assert.equal(pre.parent_source_inputs,388);assert.equal(pre.owned_paths,18);assert.equal(pre.modified_parent_inputs,6);assert.equal(pre.new_input_files,12);assert.equal(pre.unchanged_parent_inputs,382);assert.equal(pre.root_mutated,false);assert.equal(pre.frozen_manifest,null);assert.equal(pre.source_commit,null);sameRef(pre.api_proposal,CURRENT.api4);sameRef(pre.api_review,CURRENT.api4_review);
 const api=data(CURRENT.api4),ar=data(CURRENT.api4_review);noFindings(ar);sameRef(ar.proposal_manifest,CURRENT.api4);assert.equal(api.files.length,18);const currentApi=new Map();for(const f of api.files){relativeFile(f.path);const b=pin(path.join(api.source_directory,f.path),f.sha256);assert.equal(b.length,f.bytes);currentApi.set(f.path,b);}sameRef(pre.composition_derivation,CURRENT.current_derivation);const deriv=data(CURRENT.current_derivation);sameRef(deriv.basis_preformat,FIXED.preformat_manifest);sameRef(deriv.basis_actual_format,FIXED.actual_format);sameRef(deriv.basis_formatter_review,FIXED.formatter_review);sameRef(deriv.api_v4_manifest,CURRENT.api4);sameRef(deriv.api_v4_review,CURRENT.api4_review);
 const v1=data(deriv.api_v1_manifest),v1payload=new Map();for(const f of v1.files){const b=pin(path.join(v1.source_directory,f.path),f.sha256);assert.equal(b.length,f.bytes);v1payload.set(f.path,b);}assert.equal(v1payload.size,18);
 const groups=deriv.ancestral_operations;assert.equal(groups.length,6);assert.equal(groups.reduce((n,g)=>n+g.operations.length,0),21);assert.equal(deriv.revision_operations.length,9);assert.equal(deriv.prior_repair_operations.length,2);assert.equal(deriv.repair_operations.length,2);
 const computed=new Map(d3.sourceFiles);for(const [rel,b]of computed)computed.set(rel,dataLog({path:path.join(d3.pre.source_directory,rel),sha256:b.sha256,bytes:b.bytes}));
 for(const[rel,b]of v1payload){const base=computed.get(rel),g=groups.find(x=>x.path===rel);if(base){assert(g);assert.equal(sha(base),g.before_sha256);const actual=textOps(base,g.operations.map(o=>({...o,path:rel})),rel);assert(actual.equals(b));assert.equal(sha(actual),g.after_sha256);}else assert(!g);computed.set(rel,b);}
 // Product deltas and the one separately declared Node verification source have distinct authorities.
 const nodePath='api-enrichment-ui.test.cjs',v1Node=v1.additional_non_cargo_verification,currentNode=api.additional_non_cargo_verification;
 for(const [declared,manifestRef]of [[v1Node,deriv.api_v1_manifest],[currentNode,CURRENT.api4]]){assert.equal(declared.cargo_names,false);assert.equal(normalized(declared.source.path),normalized(path.join(path.dirname(manifestRef.path),nodePath)));}assert(!v1payload.has(nodePath)&&!computed.has(nodePath)&&!pre.files.some(f=>f.path===nodePath));
 const nodeOps=deriv.revision_operations.filter(o=>!v1payload.has(o.path));assert.equal(nodeOps.length,1);assert.equal(nodeOps[0].path,nodePath);const nodeVerificationAfter=textOps(dataLog(v1Node.source),nodeOps,nodePath);assert(nodeVerificationAfter.equals(dataLog(currentNode.source)));
 const productRevisionOps=deriv.revision_operations.filter(o=>v1payload.has(o.path));
 for(const ops of [productRevisionOps,deriv.prior_repair_operations,deriv.repair_operations])for(const rel of [...new Set(ops.map(o=>o.path))]){assert(v1payload.has(rel)&&computed.has(rel));computed.set(rel,textOps(computed.get(rel),ops.filter(o=>o.path===rel),rel));}
 for(const[rel,b]of currentApi)assert(computed.get(rel).equals(b));const authoredMap=rowsMap(pre.files);assert.equal(authoredMap.size,400);assert.equal(computed.size,400);for(const[rel,r]of authoredMap){assert.equal(sha(computed.get(rel)),r.sha256);assert.equal(computed.get(rel).length,r.bytes);assert(dataLog({...r,path:path.join(pre.preformat_directory,rel)}).equals(computed.get(rel)));}
 const inv=data(pre.test_inventory);sameRef(inv.api_proposal,CURRENT.api4);assert.deepEqual(inv.new_authored_test_functions,pre.new_authored_test_functions);assert.equal(pre.new_authored_test_functions.length,20);const apiTests=pre.new_authored_test_functions.map(t=>{assert(t.windows&&t.unix);assert.equal(fnNames(computed.get(t.path).toString()).filter(n=>n===t.name).length,1);assert(t.full_name.endsWith('::'+t.name));return{path:t.path,name:t.name,full_name:t.full_name,windows:true,unix:true};});
 const newFunctions=[...d3.newFunctions,...apiTests].sort((a,b)=>sort(a.path,b.path)||sort(a.name,b.name));assert.equal(newFunctions.length,48);assert.equal(new Set(newFunctions.map(t=>t.name)).size,48);assert.equal(new Set(newFunctions.map(t=>t.full_name)).size,48);const requiredTests=structuredClone(baseline.required_tests);for(const t of newFunctions){let g=requiredTests.find(g=>g.path===t.path);if(!g){g={path:t.path,names:[]};requiredTests.push(g);}assert(!g.names.includes(t.name));g.names.push(t.name);}for(const g of requiredTests)g.names.sort(sort);requiredTests.sort((a,b)=>sort(a.path,b.path));const required=requiredTests.flatMap(g=>g.names),full=[...baseline.all_required_full_test_names,...newFunctions.map(t=>t.full_name)].sort(sort),minimum=1428+48;assert.equal(required.length,694);assert.equal(new Set(required).size,694);assert.equal(full.length,698);assert.equal(new Set(full).size,698);assert.equal(minimum,1476);
 const fmt=data(CURRENT.actual_format);assert.equal(fmt.kind,'actual_api_only_preview_format');assert.equal(fmt.result,'PASS_FORMAT_COMMAND_ONLY');sameRef(fmt.preformat_manifest,CURRENT.current_composition);sameRef(fmt.composition_review,CURRENT.current_composition_review);sameRef(fmt.producer_preparation,CURRENT.preview_producers);sameRef(fmt.producer_review,CURRENT.preview_producer_review);closedPreview(fmt.process,pin);assert.deepEqual(fmt.process.args,['fmt','--all','--manifest-path',path.join(pre.source_directory,'Cargo.toml')]);assert.equal(normalized(fmt.process.cwd),normalized(pre.source_directory));assert.equal(fmt.source_final_verified,true);assert.equal(fmt.root_repo_mutated,false);assert.deepEqual(fmt.profile,profile.environment);
 const final=rowsMap(fmt.source_files.map(({path,after_sha256,bytes})=>({path,sha256:after_sha256,bytes})));assert.equal(final.size,400);assert.deepEqual([...final.keys()],[...authoredMap.keys()]);const expectedFormat=[...authoredMap.values()].map(f=>({path:f.path,before_sha256:f.sha256,before_bytes:f.bytes,after_sha256:final.get(f.path).sha256,bytes:final.get(f.path).bytes}));assert.deepEqual(fmt.source_files,expectedFormat);const changed=expectedFormat.filter(f=>f.before_sha256!==f.after_sha256);assert.equal(changed.length,14);assert.deepEqual(fmt.changed_files,changed.map(f=>f.path));
 const fr=data(CURRENT.formatter_review);noFindings(fr);sameRef(fr.preformat_manifest,CURRENT.current_composition);sameRef(fr.actual_format_process_receipt,CURRENT.actual_format);sameRef(fr.composition_review,CURRENT.current_composition_review);assert.deepEqual(fr.formatted_files,[...final.values()]);assert.deepEqual(fr.files,changed.map(({path,before_sha256,after_sha256,bytes})=>({path,before_sha256,after_sha256,bytes})));
 const owned=new Map(),bindings=[];for(const f of final.values()){const p=parentFiles.get(f.path),b=dataLog({path:path.join(pre.source_directory,f.path),sha256:f.sha256,bytes:f.bytes});if(p?.sha256===f.sha256&&p.bytes===f.bytes)continue;const roles=[...(d3.owned.get(f.path)?.roles??[]),...(currentApi.has(f.path)?['api_only_enrichment']:[])];assert(roles.length);owned.set(f.path,{path:f.path,roles});bindings.push({path:f.path,roles,before_sha256:p?.sha256??'ABSENT',before_bytes:p?.bytes??0,authored_sha256:authoredMap.get(f.path).sha256,authored_bytes:authoredMap.get(f.path).bytes,after_sha256:f.sha256,bytes:f.bytes});if(sha(computed.get(f.path))!==f.sha256)assert(f.path.endsWith('.rs'));}
 const counts={owned:owned.size,modified:[...owned.keys()].filter(x=>parentFiles.has(x)).length,new:[...owned.keys()].filter(x=>!parentFiles.has(x)).length,inputs:final.size,unchanged:381-[...owned.keys()].filter(x=>parentFiles.has(x)).length};assert.deepEqual(counts,{owned:34,modified:15,new:19,inputs:400,unchanged:366});
 const cl=data(CURRENT.actual_clippy);assert.equal(cl.kind,'actual_api_only_preview_preliminary_clippy');assert.equal(cl.result,'PASS_PRELIMINARY_CLIPPY_ONLY');sameRef(cl.actual_format,CURRENT.actual_format);sameRef(cl.formatter_review,CURRENT.formatter_review);sameRef(cl.preformat_manifest,CURRENT.current_composition);assert.deepEqual(cl.source_files,[...final.values()]);assert.deepEqual(cl.profile,profile.environment);assert.deepEqual(cl.process.args,['clippy','--workspace','--all-targets','--locked','--','-D','warnings']);const clogs=closedPreview(cl.process,pin);assert(!/^error(?:\[|:)/m.test(clogs.stderr));assert.equal(cl.executed_tests,0);assert.equal(cl.source_final_verified,true);
 const tt=data(CURRENT.actual_twenty_tests);assert.equal(tt.kind,'actual_api_only_preview_twenty_new_rust_regressions');assert.equal(tt.result,'PASS_TWENTY_NEW_RUST_FUNCTIONS_ONLY');sameRef(tt.preformat_manifest,CURRENT.current_composition);sameRef(tt.actual_format,CURRENT.actual_format);sameRef(tt.formatter_review,CURRENT.formatter_review);sameRef(tt.preliminary_clippy,CURRENT.actual_clippy);assert.deepEqual(tt.source_files,[...final.values()]);assert.deepEqual(tt.profile,profile.environment);sameFingerprints(tt.fingerprints,cl.fingerprints,pin);sameFingerprints(tt.fingerprints,fmt.fingerprints,pin);const names=apiTests.map(t=>t.full_name).sort(sort);assert.deepEqual(tt.expected_function_names,names);assert.deepEqual(tt.process.args,['test','--workspace','--locked','--','--include-ignored','--test-threads=1','--exact',...names]);const logs=closedPreview(tt.process,pin),actual=[...logs.stdout.matchAll(/^test (\S+) \.\.\. (ok|FAILED|ignored)\s*$/gm)].map(m=>({name:m[1],result:m[2]})),summaries=[...logs.stdout.matchAll(/^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*$/gm)].map(m=>({result:m[1],passed:+m[2],failed:+m[3],ignored:+m[4]}));assert.deepEqual(actual,tt.actual_test_results);assert.deepEqual(actual.map(x=>x.name).sort(sort),names);assert(actual.every(x=>x.result==='ok'));assert.deepEqual(summaries,tt.summaries);assert(summaries.length&&summaries.every(x=>x.result==='ok'&&!x.failed&&!x.ignored));assert.equal(summaries.reduce((n,x)=>n+x.passed,0),20);assert.equal(tt.executed_test_functions,20);assert.equal(tt.all_twenty_names_exactly_once,true);assert.equal(tt.source_final_verified,true);
 const n5=data(CURRENT.api_frontend_tests);assert.equal(n5.kind,'actual_api_only_frontend_helper_tests');assert.equal(n5.result,'PASS_FIVE_API_ONLY_FRONTEND_HELPER_CASES_ONLY');assert.equal(n5.passed,5);assert.equal(n5.failed,0);assert.deepEqual(n5.failures,[]);const np=n5.actual_process;assert(np.spawned&&Number.isSafeInteger(np.pid)&&np.pid>0);assert.equal(np.spawn_error,null);assert.deepEqual(np.exit,{code:0,signal:null});assert.deepEqual(np.close,np.exit);const nl={};for(const l of np.logs){assert(l.end&&l.closed);assert.equal(l.error,null);nl[l.name]=dataLog(l).toString();}assert.deepEqual([...nl.stdout.matchAll(/^ok \d+ - (.+)\r?$/gm)].map(m=>m[1]).sort(sort),[...n5.names].sort(sort));assert.equal(n5.names.length,5);const v2=data(deriv.api_v2_manifest),v2Node=v2.additional_non_cargo_verification;assert.equal(v2Node.cargo_names,false);assert.equal(normalized(v2Node.source.path),normalized(path.join(path.dirname(deriv.api_v2_manifest.path),nodePath)));sameRef(n5.test_source,v2Node.source);sameRef(n5.frontend_target,v2Node.target);assert(nodeVerificationAfter.equals(dataLog(v2Node.source)));assert(nodeVerificationAfter.equals(dataLog(n5.test_source)));assert.deepEqual(currentNode.names,n5.names);assert.equal(v1Node.names.length,4);assert.deepEqual(currentNode.names.slice(0,4),v1Node.names);assert.deepEqual([...nodeVerificationAfter.toString('utf8').matchAll(/\btest\('([^']+)'/g)].map(m=>m[1]),n5.names);const target=dataLog(n5.frontend_target);assert.equal(sha(target),final.get('web/api-enrichment.js').sha256);
 const runtimeIdentity=physicalRuntime(data,final);
 return{baselinePath,baselineSha:PARENT_SHA,baseline,parentFiles,parentRaw,parentSuiteVerified,parentSuiteReference,parentBuildReference,sourceFiles:final,requiredTests,required,expectedNewFullNames:full,newFullNames:newFunctions.map(t=>t.full_name),newFunctions,counts,minimum,owned,bindings,nodeCases:[...d3.nodeCases,{role:'api_only_enrichment',cases:n5.names,source:n5.test_source,target:n5.frontend_target,evidence:CURRENT.api_frontend_tests}],references,data,dataLog,pin,pre,d3,n5};
}

const FIXTURE_PINS={
  "proposal": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-mcp-inventory-fixtures-proposal-v1\\manifest.json",
    "sha256": "f5ed1436b1842076fe1e22f1f9f6c2e0417441e87c79e0b7c6c1063ac90982d6",
    "bytes": 3467
  },
  "review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-mcp-inventory-fixtures-independent-review-v1\\receipt.json",
    "sha256": "9610a920d73ba62d2fab1abcd377c3ad5b2a6b1a9f8f5ad12c6caf00334d6251",
    "bytes": 1785
  },
  "preview": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-inventory-fixture-preview-v1\\preformat.json",
    "sha256": "b100eda37741d66772168862e4c8c8d256f9f3c51f8f6cbb88e1a94a419e5c13",
    "bytes": 79239
  },
  "composition_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-inventory-fixture-composition-independent-review-v1\\receipt.json",
    "sha256": "89d039097c5cd9c06d693a217ad5442ac9cde402f58d6fa5ca21cd0058eb8c9d",
    "bytes": 2352
  },
  "producer": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-inventory-fixture-driver-preparation-v1\\manifest.json",
    "sha256": "78c3c6bf859a2eba7c1bd310c09e0623db0b04dfd5b825215a2627bd2e40e474",
    "bytes": 5520
  },
  "producer_review": {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\api-only-inventory-fixture-driver-independent-review-v1\\receipt.json",
    "sha256": "74ccef3f2943d40166b0a615c520a346654d0a7ed3b23ff5fac52ff7ce0f78a5",
    "bytes": 2274
  }
};
function finalFixtureStage(a,old){
 const {data,dataLog,pin}=old;
 sameRef(a.fixture_proposal,FIXTURE_PINS.proposal);sameRef(a.fixture_review,FIXTURE_PINS.review);
 sameRef(a.fixture_preformat,FIXTURE_PINS.preview);sameRef(a.fixture_composition_review,FIXTURE_PINS.composition_review);
 const fp=data(FIXTURE_PINS.proposal),fr=data(FIXTURE_PINS.review),pre=data(a.fixture_preformat),cr=data(a.fixture_composition_review);noFindings(fr);noFindings(cr);sameRef(fr.proposal_manifest,FIXTURE_PINS.proposal);sameRef(cr.proposal_manifest,a.fixture_preformat);
 assert.equal(pre.kind,'api_only_inventory_fixture_400_input_preview');assert.equal(pre.status,'COMPOSED_SOURCE_ONLY_NOT_RUN');sameRef(pre.basis_preformat,CURRENT.current_composition);sameRef(pre.basis_actual_twenty_tests,CURRENT.actual_twenty_tests);sameRef(pre.fixture_proposal,FIXTURE_PINS.proposal);sameRef(pre.fixture_review,FIXTURE_PINS.review);assert.equal(pre.source_inputs,400);assert.equal(pre.parent_source_inputs,400);assert.equal(pre.owned_paths,2);assert.equal(pre.modified_parent_inputs,2);assert.equal(pre.new_input_files,0);assert.equal(pre.unchanged_parent_inputs,398);assert.deepEqual(pre.parent12_union,{inputs:400,owned:36,modified:17,new:19,unchanged:364});assert.equal(pre.root_mutated,false);assert.equal(pre.frozen_manifest,null);assert.equal(pre.source_commit,null);assert.deepEqual(pre.new_authored_test_functions,[]);
 const expectedPaths=['crates/app/src/mcp/wiring_tests.rs','crates/app/src/server/lifecycle_tests.rs'];assert.deepEqual(fp.files.map(f=>f.path).sort(sort),expectedPaths);assert.equal(fp.production_changes,0);assert.deepEqual(fp.new_authored_test_functions,[]);const revision=data(fp.revision),deriv=data(pre.composition_derivation);assert.equal(revision.operations.length,5);assert.deepEqual(deriv.operations,revision.operations);sameRef(deriv.fixture_proposal,FIXTURE_PINS.proposal);sameRef(deriv.fixture_review,FIXTURE_PINS.review);sameRef(deriv.actual_predecessor_tests,CURRENT.actual_twenty_tests);
 const expected=new Map();for(const f of old.sourceFiles.values())expected.set(f.path,dataLog({path:path.join(old.pre.source_directory,f.path),sha256:f.sha256,bytes:f.bytes}));
 for(const rel of expectedPaths){const f=fp.files.find(f=>f.path===rel);assert(f);const next=textOps(expected.get(rel),revision.operations.filter(o=>o.path===rel),rel);assert.equal(sha(next),f.sha256);assert.equal(next.length,f.bytes);assert(dataLog({path:path.join(fp.source_directory,rel),sha256:f.sha256,bytes:f.bytes}).equals(next));expected.set(rel,next);}
 const authoredMap=rowsMap(pre.files);assert.equal(authoredMap.size,400);for(const[rel,r]of authoredMap){const b=expected.get(rel);assert(b);assert.equal(sha(b),r.sha256);assert.equal(b.length,r.bytes);assert(dataLog({...r,path:path.join(pre.preformat_directory,rel)}).equals(b));if(!expectedPaths.includes(rel))assert.deepEqual(r,old.sourceFiles.get(rel));}
 const inv=data(pre.test_inventory);assert.deepEqual(inv.retained_test_functions,pre.retained_test_functions);assert.deepEqual(inv.new_authored_test_functions,[]);assert.equal(inv.required_test_names_forecast,694);assert.equal(inv.full_test_names_forecast,698);assert.equal(inv.minimum_workspace_tests_forecast,1476);assert.deepEqual(inv.parent_all_required_full_test_names,old.expectedNewFullNames);
 const retained=old.baseline.required_tests.filter(g=>expectedPaths.includes(g.path)).flatMap(g=>g.names.map(name=>({path:g.path,name,full_name:old.baseline.all_required_full_test_names.find(n=>n.endsWith('::'+name))})));assert.equal(retained.length,22);assert.deepEqual(pre.retained_test_functions,retained);const names=retained.map(t=>t.full_name).sort(sort);assert.equal(new Set(names).size,22);assert(names.every(n=>n.startsWith('mcp::file_wiring_tests::')||n.startsWith('server::lifecycle::tests::')));
 const producer=data(FIXTURE_PINS.producer),producerReview=data(FIXTURE_PINS.producer_review);noFindings(producerReview);sameRef(producerReview.proposal_manifest,FIXTURE_PINS.producer);sameRef(producer.preview,a.fixture_preformat);sameRef(producer.fixture_proposal,FIXTURE_PINS.proposal);sameRef(producer.fixture_review,FIXTURE_PINS.review);assert.equal(producer.kind,'api_only_inventory_fixture_preview_producers_preparation');assert.equal(producer.files.length,4);for(const f of producer.files)dataLog({path:path.join(path.dirname(FIXTURE_PINS.producer.path),f.path),sha256:f.sha256,bytes:f.bytes});
 function gateBase(v){sameRef(v.producer_preparation,FIXTURE_PINS.producer);sameRef(v.producer_review,FIXTURE_PINS.producer_review);sameRef(v.composition_review,a.fixture_composition_review);sameRef(v.preformat_manifest,a.fixture_preformat);assert.equal(normalized(v.source_directory),normalized(pre.source_directory));assert.deepEqual(v.profile,profile.environment);assert.equal(v.incremental,false);assert.equal(normalized(v.cargo_target_directory),normalized(path.join(BASE,'cargo-target-api-only-preview-v1')));assert.equal(v.root_repo_mutated,false);assert.equal(v.original_installed_data_touched,false);assert.equal(v.provider_requests_by_driver,0);assert.equal(v.source_final_verified,true);assert.equal(v.full_workspace_suite,'NOT_RUN_BY_THIS_GATE');assert.equal(v.binary,'NOT_RUN');assert.equal(v.native,'NOT_RUN');}
 const fmt=data(a.fixture_actual_format);gateBase(fmt);assert.equal(fmt.kind,'actual_api_only_preview_format');assert.equal(fmt.result,'PASS_FORMAT_COMMAND_ONLY');closedPreview(fmt.process,pin);assert.equal(normalized(fmt.process.cwd),normalized(pre.source_directory));assert.deepEqual(fmt.process.args,['fmt','--all','--manifest-path',path.join(pre.source_directory,'Cargo.toml')]);
 const final=rowsMap(fmt.source_files.map(({path,after_sha256,bytes})=>({path,sha256:after_sha256,bytes})));assert.equal(final.size,400);assert.deepEqual([...final.keys()],[...authoredMap.keys()]);const allRows=[...authoredMap.values()].map(f=>({path:f.path,before_sha256:f.sha256,before_bytes:f.bytes,after_sha256:final.get(f.path).sha256,bytes:final.get(f.path).bytes}));assert.deepEqual(fmt.source_files,allRows);const changes=allRows.filter(f=>f.before_sha256!==f.after_sha256);assert(changes.every(f=>expectedPaths.includes(f.path)));assert.deepEqual(fmt.changed_files,changes.map(f=>f.path));for(const f of final.values()){dataLog({...f,path:path.join(pre.source_directory,f.path)});if(!expectedPaths.includes(f.path))assert.deepEqual(f,old.sourceFiles.get(f.path));}
 const formatter=data(a.fixture_formatter_review);noFindings(formatter);sameRef(formatter.preformat_manifest,a.fixture_preformat);sameRef(formatter.actual_format_process_receipt,a.fixture_actual_format);sameRef(formatter.composition_review,a.fixture_composition_review);assert.deepEqual(formatter.formatted_files,[...final.values()]);assert.deepEqual(formatter.files,changes.map(({path,before_sha256,after_sha256,bytes})=>({path,before_sha256,after_sha256,bytes})));
 const cl=data(a.fixture_preliminary_clippy);gateBase(cl);assert.equal(cl.kind,'actual_api_only_preview_preliminary_clippy');assert.equal(cl.result,'PASS_PRELIMINARY_CLIPPY_ONLY');sameRef(cl.actual_format,a.fixture_actual_format);sameRef(cl.formatter_review,a.fixture_formatter_review);assert.equal(cl.executed_tests,0);assert.deepEqual(cl.source_files,[...final.values()]);assert.equal(normalized(cl.process.cwd),normalized(pre.source_directory));assert.deepEqual(cl.process.args,['clippy','--workspace','--all-targets','--locked','--','-D','warnings']);const clogs=closedPreview(cl.process,pin);assert(Object.values(clogs).every(text=>!/^error(?:\[|:)/m.test(text)));sameFingerprints(cl.fingerprints,fmt.fingerprints,pin);
 const tests=data(a.fixture_retained_tests);gateBase(tests);assert.equal(tests.kind,'actual_api_only_inventory_fixture_regressions');assert.equal(tests.result,'PASS_TWENTYTWO_RETAINED_MCP_LIFECYCLE_FUNCTIONS_ONLY');sameRef(tests.actual_format,a.fixture_actual_format);sameRef(tests.formatter_review,a.fixture_formatter_review);sameRef(tests.preliminary_clippy,a.fixture_preliminary_clippy);assert.deepEqual(tests.source_files,[...final.values()]);assert.deepEqual(tests.expected_function_names,names);assert.equal(tests.executed_test_functions,22);assert.equal(tests.all_retained_names_exactly_once,true);assert.equal(normalized(tests.process.cwd),normalized(pre.source_directory));assert.deepEqual(tests.process.args,['test','-p','open-seo-spider','--bin','open-seo-spider','--locked','--','--include-ignored','--test-threads=1','--exact',...names]);const logs=closedPreview(tests.process,pin),observed=[...logs.stdout.matchAll(/^test (\S+) \.\.\. (ok|FAILED|ignored)\s*$/gm)].map(m=>({name:m[1],result:m[2]})),summaries=[...logs.stdout.matchAll(/^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*$/gm)].map(m=>({result:m[1],passed:+m[2],failed:+m[3],ignored:+m[4]}));assert.deepEqual(observed,tests.actual_test_results);assert.deepEqual(observed.map(t=>t.name).sort(sort),names);assert(observed.every(t=>t.result==='ok'));assert.deepEqual(summaries,tests.summaries);assert(summaries.length&&summaries.every(s=>s.result==='ok'&&s.failed===0&&s.ignored===0));assert.equal(summaries.reduce((n,s)=>n+s.passed,0),22);assert(!/^error(?:\[|:)/m.test(logs.stderr));sameFingerprints(tests.fingerprints,cl.fingerprints,pin);
 const owned=new Map(),bindings=[];for(const f of final.values()){const p=old.parentFiles.get(f.path);if(p?.sha256===f.sha256&&p.bytes===f.bytes)continue;const roles=[...(old.owned.get(f.path)?.roles??[]),...(expectedPaths.includes(f.path)?['api_mcp_inventory_fixture_compatibility']:[])];assert(roles.length);owned.set(f.path,{path:f.path,roles});bindings.push({path:f.path,roles,before_sha256:p?.sha256??'ABSENT',before_bytes:p?.bytes??0,authored_sha256:authoredMap.get(f.path).sha256,authored_bytes:authoredMap.get(f.path).bytes,after_sha256:f.sha256,bytes:f.bytes});}
 const counts={owned:owned.size,modified:[...owned.keys()].filter(p=>old.parentFiles.has(p)).length,new:[...owned.keys()].filter(p=>!old.parentFiles.has(p)).length,inputs:final.size,unchanged:381-[...owned.keys()].filter(p=>old.parentFiles.has(p)).length};assert.deepEqual(counts,{owned:36,modified:17,new:19,inputs:400,unchanged:364});
 return{pre,sourceFiles:final,owned,bindings,counts,fixtureGateReferences:Object.fromEntries(['fixture_preformat','fixture_composition_review','fixture_actual_format','fixture_formatter_review','fixture_preliminary_clippy','fixture_retained_tests'].map(k=>[k,a[k]]))};
}

function loadApproval({approvalPath,expectedApprovalSha,sourceDirectory,repo}){
 const old=loadPredecessorEvidence({sourceDirectory,repo});const {baselinePath,baseline,parentFiles,parentRaw,parentSuiteVerified,parentSuiteReference,parentBuildReference,requiredTests,required,newFunctions,newPaths,newFullNames,minimum,references,data,dataLog,pin}=old;const full=old.expectedNewFullNames;
 // The future Root approval binds the actual final fixture gates. Nulls cannot be read/admitted.
 const pending={path:approvalPath,sha256:expectedApprovalSha,bytes:fs.statSync(approvalPath).size};const preliminaryApproval=data(pending);const stage=finalFixtureStage(preliminaryApproval,old),final=stage.sourceFiles,owned=stage.owned,bindings=stage.bindings,counts=stage.counts;const runtimeIdentity=physicalRuntime(data,final);
 assert.match(expectedApprovalSha,HASH);const rootApprovalReference={path:approvalPath,sha256:expectedApprovalSha,bytes:fs.statSync(approvalPath).size},a=data(rootApprovalReference);assert.equal(a.schema_version,1);assert.equal(a.kind,'root_approved_combined_source_v13');assert.equal(a.status,'REVIEWED_MERGED_FORMATTED_COMMITTED_AND_PRIVATELY_PUSHED');sameRef(a.parent_manifest,CURRENT.parent_manifest);sameRef(a.parent_suite,parentSuiteReference);sameRef(a.parent_build,parentBuildReference);assert.match(a.source_commit,COMMIT);assert.notEqual(a.source_commit,baseline.source_commit);assert.equal(typeof a.source_branch,'string');assert(a.source_branch.length>0&&a.source_branch.length<=256);assert.equal(normalized(a.source_worktree),normalized(repo));assert.equal(a.private_repository_verified,true);sameRoles(a.source_proposals,ALL_ROLE_PINS);sameRef(a.fixture_proposal,FIXTURE_PINS.proposal);sameRef(a.fixture_review,FIXTURE_PINS.review);
 for(const[k,ref]of Object.entries(APPROVAL_EVIDENCE))sameRef(a[k],ref);assert.deepEqual(a.source_files,[...final.values()]);assert.deepEqual(a.files,bindings);assert.deepEqual(a.runtime_identity,RUNTIME_APPROVAL_REFS);
 const preparation=data(a.preparation_manifest),prepReview=data(a.preparation_review);noFindings(prepReview);sameRef(prepReview.proposal_manifest,a.preparation_manifest);sameRef(prepReview.binder,preparation.binder);assert.equal(preparation.kind,'combined_source_v13_api400_direct_parent_driver_preparation');assert.equal(preparation.files.length,4);sameRoles(preparation.source_proposals,ALL_ROLE_PINS);for(const f of preparation.files){relativeFile(f.path);const b=pin(path.join(path.dirname(a.preparation_manifest.path),f.path),f.sha256);assert.equal(b.length,f.bytes);}const self=fileURLToPath(import.meta.url);assert.equal(normalized(self),normalized(path.join(TASK,'work/validate-combined-source-v13-api400.mjs')));const selfRow=preparation.files.find(f=>f.path==='validate-combined-source-v13-api400.mjs');assert(selfRow);pin(self,selfRow.sha256);
 const pub=data(a.private_publication);assert.equal(pub.kind,'actual_private_source13_code_publication');assert.equal(pub.source_commit,a.source_commit);assert.equal(pub.parent_code_commit,baseline.source_commit);assert.equal(pub.branch,a.source_branch);assert.equal(pub.repository,'lia-xim/open-seo-spider');assert.equal(pub.is_private,true);assert.equal(pub.source_inputs,400);assert.equal(pub.all400_root_source_hashes_verified,true);assert.equal(pub.all400_commit_blob_hashes_verified,true);assert.equal(pub.remote_exact,true);assert.equal(pub.code_and_owned_raw_byte_attributes,true);assert.deepEqual(pub.owned_committed_paths,[...owned.keys()].sort(sort));assert.deepEqual(pub.additional_tracked_files,[]);for(const[k,r]of Object.entries(APPROVAL_EVIDENCE))sameRef(pub[k],r);sameRef(pub.fixture_proposal,FIXTURE_PINS.proposal);sameRef(pub.fixture_review,FIXTURE_PINS.review);for(const k of Object.keys(stage.fixtureGateReferences))sameRef(pub[k],a[k]);assert.deepEqual(pub.runtime_identity,RUNTIME_APPROVAL_REFS);
 for(const f of final.values())for(const root of[sourceDirectory,repo]){const b=pin(path.join(root,f.path),f.sha256);assert.equal(b.length,f.bytes);}
 return{baselinePath,baselineSha:PARENT_SHA,baseline,parentFiles,parentRaw,parentSuiteVerified,parentSuiteReference,parentBuildReference,rootApproval:a,rootApprovalReference,sourceFiles:final,requiredTests,required,expectedNewFullNames:full,newFullNames:newFunctions.map(t=>t.full_name),newFunctions,newPaths:[...owned.keys()].filter(x=>!parentFiles.has(x)).sort(sort),counts,minimum,bindings,nodeCases:old.nodeCases,references,preparation,runtimeIdentity,fixtureGateReferences:stage.fixtureGateReferences,verifyRuntimeIdentity:()=>physicalRuntime(data,final)};
}

function manifestFrom(c,repo){const validatorPath=fileURLToPath(import.meta.url);return{schema_version:1,kind:'combined_source_v13',result:'FROZEN_REVIEWED_SOURCE_FULL_SUITE_NOT_RUN',root_approval:c.rootApprovalReference,parent_source_manifest_sha256:PARENT_SHA,parent_source_inputs:381,source_inputs:c.counts.inputs,inherited_required_test_names:646,new_input_files:c.counts.new,changed_parent_inputs:c.counts.modified,unchanged_parent_inputs:c.counts.unchanged,minimum_expected_workspace_tests:c.minimum,required_test_names:c.required.length,new_authored_test_functions:c.newFunctions.length,test_profile:profile,files:[...c.sourceFiles.values()],required_tests:c.requiredTests,new_authored_tests:c.newFunctions,all_required_full_test_names:c.expectedNewFullNames,new_authored_full_test_names:c.newFullNames,source_proposals:ALL_ROLE_PINS,parent_suite:c.parentSuiteReference,parent_build:c.parentBuildReference,source_commit:c.rootApproval.source_commit,source_branch:c.rootApproval.source_branch,source_worktree:path.resolve(repo),approved_product_bindings:c.bindings,additional_non_cargo_cases:c.nodeCases,runtime_identity:c.runtimeIdentity,runtime_identity_references:RUNTIME_APPROVAL_REFS,supplemental_evidence:APPROVAL_EVIDENCE,fixture_stage:c.fixtureGateReferences,validator:{path:validatorPath,sha256:sha(fs.readFileSync(validatorPath))},references:[...c.references.values()]};}
export function createManifest({approvalPath,expectedApprovalSha,sourceDirectory,repo=path.join(BASE,'mcp-file-tools-v1')}){const c=loadApproval({approvalPath,expectedApprovalSha,sourceDirectory,repo});return manifestFrom(c,repo);}
export function validateSource13Manifest({manifest,sourceDirectory,repo=path.join(BASE,'mcp-file-tools-v1')}){const c=loadApproval({approvalPath:manifest.root_approval.path,expectedApprovalSha:manifest.root_approval.sha256,sourceDirectory,repo});assert.deepEqual(manifest,manifestFrom(c,repo));const files=c.sourceFiles;const lineage=Object.freeze({parent_source_manifest_sha256:PARENT_SHA,parent_suite_proof_sha256:PARENT_SUITE_SHA,parent_build_proof_sha256:PARENT_BUILD_SHA,source_proposals:ALL_ROLE_PINS,root_approval_sha256:manifest.root_approval.sha256,parent_inputs:381,required_parent_names:646,unchanged_parent_inputs:c.counts.unchanged,changed_parent_inputs:c.counts.modified,new_input_files:c.counts.new,delta_bindings:c.counts.owned,new_named_windows_functions:c.newFullNames,actual_required_names:c.required.length,actual_source_inputs:c.counts.inputs,minimum_expected_workspace_tests:c.minimum});return{...c,files,rendererFiles:[...files.values()].filter(f=>f.path.startsWith('renderer/')),lineage,pinnedReferences:c.references};}
