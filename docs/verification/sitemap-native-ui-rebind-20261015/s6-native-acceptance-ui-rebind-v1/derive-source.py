import json,hashlib
from pathlib import Path
BASE=Path('E:/CodexWork/open-seo-spider-01a12ac3-20261013')
TASK=Path('C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https')
NEW=BASE/'s6-native-acceptance-ui-rebind-v1'
OLD=BASE/'s6-native-acceptance-postfix-v1'
PREP=BASE/'s6-native-acceptance-preparation-v1'
UI=BASE/'sitemap-source-processing-ui-source-v1'
sha=lambda b:hashlib.sha256(b).hexdigest()
def check(p,s):
    b=p.read_bytes();assert sha(b)==s,(str(p),sha(b));return b
check(OLD/'manifest.json','cf40c019009836617b0ea24a6a237acc19818265c0bd08df864c63f597820770')
base_driver=OLD/'run-sitemap-findings-native-acceptance-v1.mjs'
check(base_driver,'2d72764ee4c27817e2eba6125f19850c4c0b498c9db966eb3ff74b1d8b95bc32')
check(TASK/'work/run-sitemap-findings-native-acceptance-v1.mjs','2d72764ee4c27817e2eba6125f19850c4c0b498c9db966eb3ff74b1d8b95bc32')
check(PREP/'findings-native-owned-adapter.mjs','30381b2d523fac41ab4a41f5e2517628c5f8c7a43c115c92b5ed28da441bff5c')
old_adapter=PREP/'findings-ui-mock-owned-adapter.cjs'
check(old_adapter,'dd51478d6be132a29d2c7a19317a9ffa02e207094c3afa10b82e273bf97f4a4d')
check(TASK/'work/sitemap-native-preparations-published-v1.json','ab9c37420bee6861f1317632dd8fbe90cd16dcf69f662d80cd00d76566c0b795')
check(UI/'manifest.json','e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5')
check(BASE/'s4-source-ui-suite-preparation-v1/manifest.json','7f956d0ade7ed0d70ced3ec92766df5d390c0da9abe674e0665a37d49dd6c2aa')
check(TASK/'work/run-sitemap-source-ui-suite-v1.mjs','b4c0ee42c03b6ade2f34f603bd501862568ed07960579d99cec4c14c1a07d43e')
check(TASK/'work/build-sitemap-source-ui-v2.mjs','1c72e8b67ef818665b081d91689d91c737a9c95f69f9fde954b72a8323f0bded')
original_ui_manifest=BASE/'s6-ui-surface-core-v1/manifest.json'
asset_base=json.loads(check(original_ui_manifest,'c9e4cb2821182d8a357ce8f8082668a6892d96cf7929e959187e31321c0ee707'))
current=json.loads((UI/'manifest.json').read_bytes());current_by_path={f['path']:f for f in current['files']}
asset_rows=[];asset_delta=[]
for f in asset_base['files']:
    actual=current_by_path[f['path']];check(UI/f['path'],actual['sha256']);assert (UI/f['path']).stat().st_size==actual['bytes']
    asset_rows.append(dict(path=f['path'],bytes=actual['bytes'],after_sha256=actual['sha256']))
    if f['after_sha256']!=actual['sha256']:asset_delta.append(dict(path=f['path'],before_sha256=f['after_sha256'],after_sha256=actual['sha256'],before_bytes=f['bytes'],after_bytes=actual['bytes']))
assert [f['path'] for f in asset_delta]==['web/app.js','web/index.html']
asset_manifest=dict(schema_version=1,kind='ui_e9_mock_asset_manifest_v1',compiler_source_manifest_sha256='e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5',base_ui_asset_manifest_sha256='c9e4cb2821182d8a357ce8f8082668a6892d96cf7929e959187e31321c0ee707',asset_count=6,changed_assets=2,unchanged_assets=4,files=asset_rows,changed_asset_pins=asset_delta,authored_mock_groups=13,executed_mock_groups=0,scope='Current exact six UI assets for mocked-API13 groups only; no native Core/SQLite/CSV or new source-profile form acceptance')
asset_bytes=(json.dumps(asset_manifest,indent=2)+'\n').encode('utf-8')
with (NEW/'UI-ASSETS.json').open('xb') as f:f.write(asset_bytes)
asset_sha=sha(asset_bytes)
A='findings-ui-mock-e9-owned-adapter.cjs';D='run-sitemap-findings-ui-source-acceptance-v1.mjs'
contents={A:old_adapter.read_bytes().decode('utf-8'),D:base_driver.read_bytes().decode('utf-8')}
ops=[]
def change(f,before,after,n=1):
    nl='\r\n' if '\r\n' in contents[f] else '\n'
    before=before.strip('\n').replace('\n',nl);after=after.strip('\n').replace('\n',nl)
    assert contents[f].count(before)==n,(f,before[:130],contents[f].count(before),n)
    contents[f]=contents[f].replace(before,after)
    ops.append(dict(path=f,expected_occurrences=n,before=before,after=after))
change(A,'// Prepared UI/API fixture only. This never proves native Core/SQLite predicates.','// Current e9 UI/API mock fixture only. Thirteen original mock groups; no native Core/SQLite/profile-form proof.')
change(A,"const manifestBytes=fs.readFileSync(args.manifest);assert.equal(sha(manifestBytes),args['manifest-sha256']);const manifest=JSON.parse(manifestBytes);",f"""
const manifestBytes=fs.readFileSync(args.manifest);assert.equal(sha(manifestBytes),args['manifest-sha256']);const manifest=JSON.parse(manifestBytes);
const expectedCompilerSHA='e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5';
assert.equal(args['manifest-sha256'],'{asset_sha}','Only current e9 six-asset mock manifest');assert.equal(manifest.kind,'ui_e9_mock_asset_manifest_v1');assert.equal(manifest.compiler_source_manifest_sha256,expectedCompilerSHA);assert.equal(manifest.files.length,6);assert.equal(new Set(manifest.files.map(file=>file.path)).size,6);
assert.equal(sourceRoot,fs.realpathSync('E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-source-processing-ui-source-v1'));assert.equal(sha(fs.readFileSync(path.join(sourceRoot,'manifest.json'))),expectedCompilerSHA);
""")
change(A,"assert.equal(releaseReceipt.kind,'root_released_sitemap_findings_ui_fixture');","assert.equal(releaseReceipt.kind,'root_released_sitemap_findings_ui_e9_fixture_v1');assert.equal(releaseReceipt.compiler_source_manifest_sha256,expectedCompilerSHA);")
change(A,"assert.equal(sha(bytes),pin.after_sha256);sourcePins.push", "assert.equal(sha(bytes),pin.after_sha256);assert.equal(bytes.length,pin.bytes);sourcePins.push")
# Guard controls and all13 test bodies/cleanup below remain exact.
with (NEW/A).open('xb') as f:f.write(contents[A].encode('utf-8'))
adapter_sha=sha(contents[A].encode('utf-8'))
change(D,"const repo = path.join(root, 'sitemap-findings-surfaces-integration-v1');","const repo = path.join(root, 'sitemap-source-processing-ui-v1');")
change(D,"const source = path.join(root, 'sitemap-findings-surfaces-source-v1');","const source = path.join(root, 'sitemap-source-processing-ui-source-v1');")
change(D,"const expectedSourceSha = '69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751';","const expectedSourceSha = 'e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5';")
change(D,'results/sitemap-findings-${gate}-${mode}-acceptance-v1','results/sitemap-findings-ui-source-${gate}-${mode}-acceptance-v1')
change(D,"assert.equal(release.kind, 'root_released_sitemap_findings_acceptance_driver_v1');","assert.equal(release.kind, 'root_released_sitemap_findings_ui_source_acceptance_v1');")
start=contents[D].index('// This is a fixed historical source publication checkpoint, not a current')
end=contents[D].index('const ownedRendererGates =',start)
old_publication=contents[D][start:end]
lineage=(BASE/'s4-source-ui-suite-preparation-v1/LINEAGE-GUARD.txt').read_bytes().decode('utf-8')
lineage=lineage.replace('\r\n','\n').rstrip('\n')
new_publication=lineage+"""
// Historical preparation checkpoint only: no native acceptance is transferred.
const preparationsPublicationPath=path.join(task,'work/sitemap-native-preparations-published-v1.json');
const expectedPreparationsPublicationSHA='ab9c37420bee6861f1317632dd8fbe90cd16dcf69f662d80cd00d76566c0b795';
const preparationsPublicationRaw=fs.readFileSync(preparationsPublicationPath);assert.equal(sha(preparationsPublicationRaw),expectedPreparationsPublicationSHA);
const preparationsPublication=JSON.parse(preparationsPublicationRaw);assert.equal(preparationsPublication.commit,'70f2f45f92a896a12d81b3a46558c2b50f0e8db7');assert.equal(preparationsPublication.repository.isPrivate,true);assert.equal(preparationsPublication.current_ui_source_manifest_sha256,expectedSourceSha);assert.equal(preparationsPublication.native_basis_source_manifest_sha256,'69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751');assert.equal(preparationsPublication.native_acceptance_transferred_to_changed_ui,false);
const publicationPath=path.join(task,'work/sitemap-source-processing-ui-published-v1.json'),expectedPublicationSHA='4316704b11e47d7f49bb541e9b1db9b1c31ac9c53d5b68dd2c47ab339b4bc24b';
const publicationLineage=Object.freeze({current_source:sourceUILineage,prior_preparations:{path:preparationsPublicationPath,sha256:expectedPreparationsPublicationSHA,commit:preparationsPublication.commit,result:preparationsPublication.result,native_acceptance_transferred:false}});
assert.equal(sha(fs.readFileSync(path.join(task,'work/run-sitemap-source-ui-suite-v1.mjs'))),'b4c0ee42c03b6ade2f34f603bd501862568ed07960579d99cec4c14c1a07d43e');
assert.equal(sha(fs.readFileSync(path.join(task,'work/build-sitemap-source-ui-v2.mjs'))),'1c72e8b67ef818665b081d91689d91c737a9c95f69f9fde954b72a8323f0bded');
"""
# Preserve the one original trailing newline by exact literal source operation.
change(D,old_publication,new_publication+'\n')
change(D,'results/sitemap-findings-surfaces-suite-v1/proof.json','results/sitemap-source-ui-suite-v1/proof.json')
change(D,"assert.equal(suite.source_manifest_sha256, expectedSourceSha);","assert.equal(suite.source_manifest_sha256, expectedSourceSha);assert.deepEqual(suite.source_lineage,sourceUILineage,'Actual current UI-suite returned lineage');")
change(D,'results/sitemap-findings-surfaces-${mode}-build-v2','results/sitemap-source-ui-${mode}-build-v2')
change(D,"assert.equal(build.source_manifest_sha256, expectedSourceSha);","assert.equal(build.source_manifest_sha256, expectedSourceSha);assert.deepEqual(build.source_lineage,sourceUILineage,'Actual current UI-build returned lineage');")
change(D,"const uiAdapterPath = path.join(handoff,'findings-ui-mock-owned-adapter.cjs');", "const uiAdapterPath = path.join(root,'s6-native-acceptance-ui-rebind-v1/findings-ui-mock-e9-owned-adapter.cjs');")
change(D,"const uiAdapterSHA = 'dd51478d6be132a29d2c7a19317a9ffa02e207094c3afa10b82e273bf97f4a4d';",f"const uiAdapterSHA = '{adapter_sha}';")
change(D,"const uiManifestPath = path.join(root,'s6-ui-surface-core-v1/manifest.json');","const uiManifestPath = path.join(root,'s6-native-acceptance-ui-rebind-v1/UI-ASSETS.json');")
change(D,"const uiManifestSHA = 'c9e4cb2821182d8a357ce8f8082668a6892d96cf7929e959187e31321c0ee707';",f"const uiManifestSHA = '{asset_sha}';")
change(D,"assert.equal(uiManifest.files.length,6);","assert.equal(uiManifest.files.length,6);assert.equal(uiManifest.compiler_source_manifest_sha256,expectedSourceSha);assert.equal(uiManifest.kind,'ui_e9_mock_asset_manifest_v1');")
change(D,"  assert.equal(sha(fs.readFileSync(publicationPath)),expectedPublicationSHA,'Fixed publication receipt changed');", "  assert.equal(sha(fs.readFileSync(publicationPath)),expectedPublicationSHA,'Fixed UI publication receipt changed');assert.deepEqual(verifyUiLineage(),sourceUILineage);assert.equal(sha(fs.readFileSync(preparationsPublicationPath)),expectedPreparationsPublicationSHA,'Historical preparation checkpoint changed');")
change(D,"kind:'root_released_sitemap_findings_ui_fixture'","kind:'root_released_sitemap_findings_ui_e9_fixture_v1'")
change(D,'source_publication:publicationLineage,actual_close_clock:completionClock,source_manifest_sha256:', 'source_publication:publicationLineage,source_lineage:sourceUILineage,actual_close_clock:completionClock,source_manifest_sha256:')
change(D,"assert.equal(path.resolve(path.dirname(self), '..'), path.resolve(task), 'Promote only to pinned Task/work before execution');","assert.equal(path.resolve(path.dirname(self), '..'), path.resolve(task), 'Promote only to pinned Task/work before execution');assert.equal(path.resolve(self),path.join(task,'work/run-sitemap-findings-ui-source-acceptance-v1.mjs'),'Exact new Task driver name');")
change(D,'  pinFile(binary,binarySHA); pinFile(adapterPath,adapterSHA); pinFile(uiAdapterPath,uiAdapterSHA);',"  pinFile(binary,binarySHA); pinFile(adapterPath,adapterSHA); pinFile(uiAdapterPath,uiAdapterSHA);pinFile(uiManifestPath,uiManifestSHA);pinFile(path.join(task,'work/run-sitemap-source-ui-suite-v1.mjs'),'b4c0ee42c03b6ade2f34f603bd501862568ed07960579d99cec4c14c1a07d43e');pinFile(path.join(task,'work/build-sitemap-source-ui-v2.mjs'),'1c72e8b67ef818665b081d91689d91c737a9c95f69f9fde954b72a8323f0bded');")
# Runtime child, ownership and immutable monotonic completion blocks are not rewritten.
with (NEW/D).open('xb') as f:f.write(contents[D].encode('utf-8'))
guards=[]
for name,original in [(D,base_driver),(A,old_adapter)]:
    inverse=contents[name]
    for op in reversed(ops):
        if op['path']==name:
            assert inverse.count(op['after'])==op['expected_occurrences']
            inverse=inverse.replace(op['after'],op['before'])
    assert inverse.encode('utf-8')==original.read_bytes()
    guards.append(dict(path=name,before_path=str(original),before_sha256=sha(original.read_bytes()),after_sha256=sha(contents[name].encode('utf-8')),bytes=len(contents[name].encode('utf-8'))))
result=dict(schema_version=1,guardian_basis_sha256='2d72764ee4c27817e2eba6125f19850c4c0b498c9db966eb3ff74b1d8b95bc32',ui_adapter_basis_sha256='dd51478d6be132a29d2c7a19317a9ffa02e207094c3afa10b82e273bf97f4a4d',operations=ops,payload_guards=guards,ui_asset_manifest_sha256=asset_sha,ui_asset_changes=asset_delta,ui_unchanged_assets=4,forward='EXACT',inverse='EXACT_ORIGINAL_BYTES',native_adapter_unchanged_sha256='30381b2d523fac41ab4a41f5e2517628c5f8c7a43c115c92b5ed28da441bff5c')
with (NEW/'DERIVATION.json').open('x',encoding='utf-8',newline='\n') as f:json.dump(result,f,indent=2);f.write('\n')
print(json.dumps(dict(operations=len(ops),payload_guards=guards,ui_asset_manifest_sha256=asset_sha)))