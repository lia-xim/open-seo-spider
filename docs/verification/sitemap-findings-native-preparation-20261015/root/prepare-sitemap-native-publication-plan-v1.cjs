const fs=require('node:fs'),path=require('node:path'),{createHash}=require('node:crypto');
const task=path.dirname(__dirname),base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',sha=b=>createHash('sha256').update(b).digest('hex');
const [browserReviewName,browserClosure,uiReviewName,uiClosure]=process.argv.slice(2);if(process.argv.length!==6||![browserClosure,uiClosure].every(x=>/^[a-f0-9]{64}$/.test(x??''))||![browserReviewName,uiReviewName].every(x=>/^[a-z0-9-]+$/.test(x??'')))throw Error('Actual independent browser/UI review names and SHAs required');
const packages=[
 ['s6-native-acceptance-preparation-v1','18b306748cd301054d79a36cbc1f464691617bbda05ab3db3685644959306155'],
 ['s6-native-acceptance-postfix-v1','cf40c019009836617b0ea24a6a237acc19818265c0bd08df864c63f597820770'],
 ['s6-finding-physical-preparation-v1','78c3d01a5837964dcd1dde7fb11c8172994192c4d6fc3c57096c57afb2d2a10e'],
 ['s6-finding-physical-postfix-v1','50f070b0342d90cc378d360b888e1e5862929ab942762a27709f7567245806b5'],
 ['s6-native-browser-preparation-v1','71ed85b6baf20f9bb5f0c1103a4351d06cc44bd3cae49fe94db489bcf71e6e51'],
 ['s6-native-browser-postfix-v1','eddbf80f1e8f95dfe51e562e1ba94aa9d1389930003009c73e808a8d8f303792'],
 ['s4-source-ui-suite-preparation-v1','7f956d0ade7ed0d70ced3ec92766df5d390c0da9abe674e0665a37d49dd6c2aa']
].map(([name,sha256])=>({name,sha256}));
const reviews=[
 ['s6-native-harness-lifecycle-review-v1','493e806250bef7d3eecd7f6316363c4c58a439cac5cfd04a470dfaec25674fc5'],
 ['s6-native-acceptance-review-v1','c2a18a01a306e487335390e211572e7a5617256d2cce744c00d731033a43f1a6'],
 ['s6-native-acceptance-postfix-review-v1','d7535479b7cda072b1863af839e6174cebfdeec1a98c1a6c2dcee3ecdffed61d'],
 ['s6-finding-physical-review-v1','b511badee8535063caf9097d086e99e09483378f13b111a69888a8a18db75bfa'],
 ['s6-finding-physical-postfix-review-v1','c48bd77696ea8ebd760d66649145523edae38bc442b17057f06469ba10c1368d'],
 ['s6-native-browser-review-v1','b8189980c0c49cf495924cb81f3216a99a6702ea7f3b0fbcccba388a56318fb8'],
 [browserReviewName,browserClosure],
 [uiReviewName,uiClosure]
].map(([name,sha256])=>({name,sha256}));
for(const f of packages){const file=path.join(base,f.name,'manifest.json');if(sha(fs.readFileSync(file))!==f.sha256)throw Error('Package pin changed: '+file);}
for(const f of reviews){const file=path.join(base,f.name,'receipt.json');if(sha(fs.readFileSync(file))!==f.sha256)throw Error('Review pin changed: '+file);}
const task_files=['work/sitemap-findings-surfaces-published-v1.json','work/sitemap-source-processing-ui-published-v1.json','work/promote-sitemap-findings-native-v1.cjs','work/promote-sitemap-findings-native-v1.json','work/promote-sitemap-source-ui-runners-v1.cjs','work/promote-sitemap-source-ui-runners-v1.json','work/prepare-sitemap-native-publication-plan-v1.cjs','work/sitemap-native-preparations-description-v1.md'].map(p=>({path:p,sha256:sha(fs.readFileSync(path.join(task,p)))}));
const plan={schema_version:1,result:'ROOT_SELECTED_REVIEWED_SOURCE_ONLY_NOT_RUN',packages,reviews,task_files,
 selected:{
  native_and_ui_mock:{path:path.join(task,'work/run-sitemap-findings-native-acceptance-v1.mjs'),sha256:'2d72764ee4c27817e2eba6125f19850c4c0b498c9db966eb3ff74b1d8b95bc32',selected_manifest_sha256:packages[1].sha256,closure_sha256:reviews[2].sha256,execution:0},
  physical:{path:path.join(base,'s6-finding-physical-postfix-v1/native-finding-physical-v2.mjs'),sha256:'95c014077ff72565eb48e243aa48e31da79efec1e82b662812108179a48f1f67',selected_manifest_sha256:packages[3].sha256,closure_sha256:reviews[4].sha256,execution:0},
  browser_native:{path:path.join(base,'s6-native-browser-postfix-v1/run-sitemap-findings-browser-native-v1.mjs'),sha256:'581ec6ad181d22b48772f034931c37cf87d91bd7deb7bbb8a24090995532a5dc',selected_manifest_sha256:packages[5].sha256,closure_sha256:browserClosure,execution:0},
  separate_ui_suite_builder:{suite_path:path.join(task,'work/run-sitemap-source-ui-suite-v1.mjs'),suite_sha256:'b4c0ee42c03b6ade2f34f603bd501862568ed07960579d99cec4c14c1a07d43e',builder_path:path.join(task,'work/build-sitemap-source-ui-v2.mjs'),builder_sha256:'1c72e8b67ef818665b081d91689d91c737a9c95f69f9fde954b72a8323f0bded',selected_manifest_sha256:packages[6].sha256,closure_sha256:uiClosure,source_manifest_sha256:'e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5',execution:0,native_acceptance_transferred:false},
  original_versions:'PRESERVED_SUPERSEDED_NOEXEC'
 },
 documentation_utf8:fs.readFileSync(path.join(task,'work/sitemap-native-preparations-description-v1.md'),'utf8'),
 open_gates:['Actual full same-source workspace/Clippy/fmt and owned browser worker gates','Actual SDK/API/CLI, separate13mocks/native20browser/physical12cases themselves','UIe9 source needs its own rebind, actual exact form payload/save/reload/larger source/browser gates','Positive50k/50MiB, incomplete/unknown source matrix; stale/race/cancel/disconnect/deadline/latefault/Join/Shutdown','Selected/recovery/measuredRAM/latency/optimizedRelease/data-preservingInstall/Linux/containers/remoteCI','Manufacturer parity NOT_PROVEN']
};
const out=path.join(task,'work/sitemap-native-preparations-publication-plan-v1.json');fs.writeFileSync(out,JSON.stringify(plan,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({path:out,sha256:sha(fs.readFileSync(out)),packages:packages.length,reviews:reviews.length}));
