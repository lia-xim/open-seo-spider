import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(resolve(root,'renderer/package.json'));
const axe = require('axe-core');
const { chromium } = require('playwright-core');
const { findBrowserExecutable } = await import('../renderer/server.mjs');
const inventory=JSON.parse(await readFile(resolve(root,'docs/parity/reference-cli-inventory.json'),'utf8'));
const filters=inventory.export_tab_entries.filter(entry=>entry.tab==='Accessibility'&&entry.kind==='named_filter'&&!entry.filter.includes('Violation')&&!entry.filter.startsWith('Accessibility Score'));
const doc=(body='',{head='',lang='en',title='Independent rule fixture',h1=true,main=true,bodyAttrs='',htmlAttrs=''}={})=>`<!doctype html><html ${lang===null?'':`lang="${lang}"`} ${htmlAttrs}><head>${title===null?'':`<title>${title}</title>`}${head}</head><body ${bodyAttrs}>${main?'<main>':''}${h1?'<h1>Independent accessible heading</h1>':''}${body}${main?'</main>':''}</body></html>`;
const image='data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jXnQAAAAASUVORK5CYII=';
// These are independent proposed mappings to public axe identifiers, not recovered vendor mappings.
const specs=[
 ['accesskeys','<button accesskey="a">One</button><button accesskey="a">Two</button>','<button accesskey="a">One</button><button accesskey="b">Two</button>'],
 ['presentation-role-conflict','<button role="presentation">One</button>','<div role="presentation">One</div>'],
 ['tabindex','<button tabindex="2">One</button>','<button tabindex="0">One</button>'],
 ['scrollable-region-focusable','<div style="height:30px;overflow:auto">'+('Long controlled text<br>'.repeat(20))+'</div>','<div tabindex="0" style="height:30px;overflow:auto">'+('Long controlled text<br>'.repeat(20))+'</div>'],
 ['skip-link','<a style="position:absolute;left:-10000px" href="#main-missing">Skip to main content</a>','<a style="position:absolute;left:-10000px" href="#main-content">Skip to main content</a><div id="main-content" tabindex="-1">Content</div>'],
 ['aria-required-attr','<div role="checkbox" aria-label="Choice"></div>','<div role="checkbox" aria-label="Choice" aria-checked="false"></div>'],
 ['aria-text','<div role="text"><button>Focusable</button></div>','<div role="text">Plain text</div>'],
 ['aria-conditional-attr','<input type="checkbox" aria-checked="true" aria-label="Choice">','<input type="checkbox" aria-checked="false" aria-label="Choice">'],
 ['aria-valid-attr-value','<button aria-expanded="nonsense">Expand</button>','<button aria-expanded="false">Expand</button>'],
 ['aria-valid-attr','<button aria-spelledwrong="x">Name</button>','<button aria-label="Name">Name</button>'],
 ['aria-command-name','<div role="button" tabindex="0"></div>','<div role="button" tabindex="0" aria-label="Action"></div>'],
 ['aria-dialog-name','<div role="dialog">Dialog body</div>','<div role="dialog" aria-label="Dialog name">Dialog body</div>'],
 ['aria-input-field-name','<div role="textbox" contenteditable="true"></div>','<div role="textbox" contenteditable="true" aria-label="Input"></div>'],
 ['aria-meter-name','<div role="meter" aria-valuemin="0" aria-valuemax="10" aria-valuenow="2"></div>','<div role="meter" aria-label="Meter" aria-valuemin="0" aria-valuemax="10" aria-valuenow="2"></div>'],
 ['aria-progressbar-name','<div role="progressbar" aria-valuenow="2"></div>','<div role="progressbar" aria-label="Progress" aria-valuenow="2"></div>'],
 ['aria-allowed-role','<input type="text" role="heading" aria-label="Input">','<input type="text" aria-label="Input">'],
 ['aria-required-parent','<div role="listitem">Orphan</div>','<div role="list"><div role="listitem">Item</div></div>'],
 ['aria-roles','<div role="not-real-role">Text</div>','<div role="button" aria-label="Button">Text</div>'],
 ['aria-toggle-field-name','<div role="switch" aria-checked="false"></div>','<div role="switch" aria-checked="false" aria-label="Toggle"></div>'],
 ['aria-tooltip-name','<div role="tooltip"></div>','<div role="tooltip">Hint</div>'],
 ['aria-treeitem-name','<div role="tree"><div role="treeitem"></div></div>','<div role="tree"><div role="treeitem">Tree item</div></div>'],
 ['aria-required-children','<div role="list">Text only</div>','<div role="list"><div role="listitem">Item</div></div>'],
 ['aria-deprecated-role','<div role="directory"><div role="listitem">Item</div></div>','<div role="list"><div role="listitem">Item</div></div>'],
 ['aria-braille-equivalent','<button aria-braillelabel="Braille"></button>','<button aria-label="Action" aria-braillelabel="Braille"></button>'],
 ['aria-hidden-focus','<div aria-hidden="true"><button>Hidden focus</button></div>','<div aria-hidden="true"><button tabindex="-1">Hidden unfocusable</button></div>'],
 ['aria-hidden-body',doc('',{bodyAttrs:'aria-hidden="true"'}),doc(''),true],
 ['aria-prohibited-attr','<p aria-label="Unsupported">Text</p>','<p>Text</p>'],
 ['aria-allowed-attr','<div role="button" aria-selected="true" aria-label="Action"></div>','<div role="button" aria-label="Action"></div>'],
 ['duplicate-id-aria','<span id="name">One</span><span id="name">Two</span><input aria-labelledby="name">','<span id="name">One</span><span id="other">Two</span><input aria-labelledby="name">'],
 ['bypass',doc('<header><nav><a href="#">Nav</a></nav></header><p>Body</p>',{main:false,h1:false}),doc('<main><h1>Main</h1></main>'),true],
 ['region',doc('<p>Content outside landmarks</p>',{main:false}),doc('<p>Content inside main</p>'),true],
 ['landmark-one-main',doc('<p>Body</p>',{main:false}),doc('<p>Body</p>'),true],
 ['landmark-no-duplicate-banner',doc('<header>Banner1</header><header>Banner2</header>',{main:false}),doc('<header>Banner</header>',{main:false}),true],
 ['landmark-banner-is-top-level','<div role="navigation"><div role="banner">Nested</div></div>',doc('<div role="banner">Top</div>',{main:false}),false,true],
 ['landmark-no-duplicate-contentinfo',doc('<footer>Footer1</footer><footer>Footer2</footer>',{main:false}),doc('<footer>Footer</footer>',{main:false}),true],
 ['landmark-no-duplicate-main',doc('<main>Main1</main><main>Main2</main>',{main:false}),doc('<main>Main</main>',{main:false}),true],
 ['landmark-complementary-is-top-level','<nav><aside>Nested aside</aside></nav>',doc('<aside>Top aside</aside>',{main:false}),false,true],
 ['landmark-contentinfo-is-top-level','<div role="contentinfo">Nested footer</div>',doc('<div role="contentinfo">Top footer</div>',{main:false}),false,true],
 ['landmark-main-is-top-level','<div role="main">Nested main</div>',doc('<div role="main">Top main</div>',{main:false}),false,true],
 ['landmark-unique','<nav>One</nav><nav>Two</nav>','<nav aria-label="Primary">One</nav><nav aria-label="Secondary">Two</nav>'],
 ['label','<input>','<label for="x">Field</label><input id="x">'],
 ['label-title-only','<input title="Field">','<label for="x">Field</label><input id="x">'],
 ['form-field-multiple-labels','<label for="x">One</label><label for="x">Two</label><input id="x">','<label for="x">One</label><input id="x">'],
 ['autocomplete-valid','<label>Field<input autocomplete="invalid-token"></label>','<label>Field<input autocomplete="given-name"></label>'],
 ['frame-title','<iframe srcdoc="<p>Frame</p>"></iframe>','<iframe title="Controlled frame" srcdoc="<p>Frame</p>"></iframe>'],
 ['frame-title-unique','<iframe title="Same" srcdoc="<p>One</p>"></iframe><iframe title="Same" srcdoc="<p>Two</p>"></iframe>','<iframe title="One" srcdoc="<p>One</p>"></iframe><iframe title="Two" srcdoc="<p>Two</p>"></iframe>'],
 ['frame-tested','<iframe title="Frame" srcdoc="<p>Not injected</p>"></iframe>','<p>No untested frame</p>'],
 ['frame-focusable-content','<iframe title="Frame" tabindex="-1" srcdoc="<button>Focus</button>"></iframe>','<iframe title="Frame" tabindex="0" srcdoc="<button>Focus</button>"></iframe>'],
 ['document-title',doc('',{title:null}),doc(''),true],
 ['page-has-heading-one',doc('<p>Body</p>',{h1:false}),doc('<p>Body</p>'),true],
 ['heading-order','<h3>Skipped level</h3>','<h2>Next level</h2>'],
 ['empty-heading','<h2></h2>','<h2>Section</h2>'],
 ['meta-viewport-large',doc('',{head:'<meta name="viewport" content="maximum-scale=2">'}),doc('',{head:'<meta name="viewport" content="width=device-width,initial-scale=1">'}),true],
 ['meta-viewport',doc('',{head:'<meta name="viewport" content="user-scalable=no">'}),doc('',{head:'<meta name="viewport" content="width=device-width,initial-scale=1">'}),true],
 ['html-lang-valid',doc('',{lang:'xyzxyz'}),doc('',{lang:'en'}),true],
 ['html-has-lang',doc('',{lang:null}),doc('',{lang:'en'}),true],
 ['html-xml-lang-mismatch',doc('',{lang:'en',htmlAttrs:'xml:lang="fr"'}),doc('',{lang:'en',htmlAttrs:'xml:lang="en"'}),true],
 ['valid-lang','<p lang="xyzxyz">Text</p>','<p lang="en">Text</p>'],
 ['meta-refresh',doc('',{head:'<meta http-equiv="refresh" content="5;url=#">'}),doc(''),true],
 ['meta-refresh-no-exceptions',doc('',{head:'<meta http-equiv="refresh" content="5;url=#">'}),doc(''),true],
 ['input-image-alt',`<input type="image" src="${image}">`,`<input type="image" src="${image}" alt="Submit">`],
 ['image-alt',`<img src="${image}">`,`<img src="${image}" alt="Controlled image">`],
 ['object-alt','<object type="image/svg+xml" data="data:image/svg+xml,%3Csvg%3E%3C/svg%3E"></object>','<object aria-label="Image" type="image/svg+xml" data="data:image/svg+xml,%3Csvg%3E%3C/svg%3E"></object>'],
 ['area-alt',`<img src="${image}" alt="Map" usemap="#map"><map name="map"><area href="#" shape="rect" coords="0,0,100,100"></map>`,`<img src="${image}" alt="Map" usemap="#map"><map name="map"><area href="#" alt="Destination" shape="rect" coords="0,0,100,100"></map>`],
 ['image-redundant-alt',`<p>Controlled text<img src="${image}" alt="Controlled text"></p>`,`<p>Controlled text<img src="${image}" alt="Independent illustration"></p>`],
 ['role-img-alt','<div role="img">Graphic</div>','<div role="img" aria-label="Graphic">Graphic</div>'],
 ['svg-img-alt','<svg role="img" width="40" height="40"><rect width="40" height="40"></rect></svg>','<svg role="img" aria-label="Graphic" width="40" height="40"><rect width="40" height="40"></rect></svg>'],
 ['server-side-image-map',`<a href="#"><img src="${image}" alt="Map" ismap></a>`,`<a href="#"><img src="${image}" alt="Map"></a>`],
 ['video-caption','<video controls src="data:video/mp4,"></video>','<video controls src="data:video/mp4,"><track kind="captions" src="data:text/vtt,WEBVTT"></video>'],
 ['no-autoplay-audio','<audio autoplay src="data:audio/wav,"></audio>','<audio controls src="data:audio/wav,"></audio>'],
 ['button-name','<button></button>','<button>Action</button>'],
 ['avoid-inline-spacing','<p style="line-height:12px!important;letter-spacing:0px!important">Text spacing</p>','<p>Text spacing</p>'],
 ['input-button-name','<input type="button">','<input type="button" value="Action">'],
 ['link-in-text-block','<p style="color:#000">Text <a href="#" style="color:#111;text-decoration:none">link</a> text</p>','<p>Text <a href="#" style="text-decoration:underline">link</a> text</p>'],
 ['link-name','<a href="#"></a>','<a href="#">Destination</a>'],
 ['identical-links-same-purpose','<a href="https://example.invalid/one">Same</a><a href="https://example.invalid/two">Same</a>','<a href="https://example.invalid/one">One</a><a href="https://example.invalid/two">Two</a>'],
 ['select-name','<select><option>Choice</option></select>','<label>Choice<select><option>Choice</option></select></label>'],
 ['summary-name','<details><summary></summary>Body</details>','<details><summary>Details</summary>Body</details>'],
 ['marquee','<marquee>Moving text</marquee>','<p>Static text</p>'],
 ['blink','<blink>Blinking text</blink>','<p>Static text</p>'],
 ['color-contrast-enhanced','<p style="color:#666;background:#fff">Contrast text</p>','<p style="color:#000;background:#fff">Contrast text</p>'],
 ['color-contrast','<p style="color:#aaa;background:#fff">Contrast text</p>','<p style="color:#000;background:#fff">Contrast text</p>'],
 ['target-size','<button style="width:10px;height:10px;padding:0;min-width:0;font-size:8px">A</button><button style="width:10px;height:10px;padding:0;min-width:0;font-size:8px">B</button>','<button style="width:40px;height:40px">A</button><button style="width:40px;height:40px">B</button>'],
 ['nested-interactive','<button><a href="#">Nested</a></button>','<button>Action</button><a href="#">Separate</a>'],
 ['listitem','<li>Orphan item</li>','<ul><li>Item</li></ul>'],
 ['list','<ul><div>Invalid child</div></ul>','<ul><li>Item</li></ul>'],
 ['definition-list','<dl><dd>Definition without term</dd></dl>','<dl><dt>Term</dt><dd>Definition</dd></dl>'],
 ['dlitem','<dt>Orphan term</dt><dd>Orphan definition</dd>','<dl><dt>Term</dt><dd>Definition</dd></dl>'],
 ['th-has-data-cells','<table><tr><th>Lonely</th></tr></table>','<table><tr><th scope="col">Header</th></tr><tr><td>Value</td></tr></table>'],
 ['td-headers-attr','<table><tr><th id="head">Head</th><td headers="missing">Value</td></tr></table>','<table><tr><th id="head">Head</th><td headers="head">Value</td></tr></table>'],
 ['empty-table-header','<table><tr><th></th><td>Value</td></tr></table>','<table><tr><th>Header</th><td>Value</td></tr></table>'],
 ['table-duplicate-name','<table summary="Same"><caption>Same</caption><tr><td>Value</td></tr></table>','<table><caption>Caption</caption><tr><td>Value</td></tr></table>'],
 ['scope-attr-valid','<table><tr><th scope="invalid">Head</th><td>Value</td></tr></table>','<table><tr><th scope="row">Head</th><td>Value</td></tr></table>']
];
assert.equal(specs.length,filters.length);
const fixtures=specs.map(([rule,positive,negative,positiveWhole=false,negativeWhole=positiveWhole],index)=>({filter_argument:filters[index].argument,
  proposed_axe_rule_id:rule,mapping_status:'INDEPENDENT_CANDIDATE_NOT_VENDOR_PROVEN',positive_html:positiveWhole?positive:doc(positive),negative_html:negativeWhole?negative:doc(negative)}));
await writeFile(resolve(root,'docs/parity/accessibility-fixtures.json'),JSON.stringify({schema_version:1,axe_version:axe.version,mapping_basis:'Independent mapping by documented rule semantics; installed vendor engine mapping not measured.',fixtures},null,2)+'\n');
const browser=await chromium.launch({headless:true,executablePath:await findBrowserExecutable()});
const checks=[];
try {
 const context=await browser.newContext({viewport:{width:1280,height:800}});
 const page=await context.newPage();
 for(const fixture of fixtures){
  const results=[];
  for(const html of [fixture.positive_html,fixture.negative_html]){
   await page.setContent(html,{waitUntil:'load'});
   await page.addScriptTag({content:axe.source});
   results.push(await page.evaluate(async rule=>{
    const result=await window.axe.run(document,{runOnly:{type:'rule',values:[rule]},rules:{[rule]:{enabled:true}}});
    return {violations:result.violations.map(item=>({id:item.id,impact:item.impact,tags:item.tags,nodes:item.nodes.map(node=>({target:node.target,failure_summary:node.failureSummary}))})),incomplete:result.incomplete.map(item=>({id:item.id,nodes:item.nodes.length})),passes:result.passes.map(item=>item.id),inapplicable:result.inapplicable.map(item=>item.id)};
   },fixture.proposed_axe_rule_id));
  }
  const triggered=results[0].violations.some(item=>item.id===fixture.proposed_axe_rule_id), review=results[0].incomplete.some(item=>item.id===fixture.proposed_axe_rule_id);
  const clean=!results[1].violations.some(item=>item.id===fixture.proposed_axe_rule_id), reviewClean=!results[1].incomplete.some(item=>item.id===fixture.proposed_axe_rule_id);
  checks.push({filter_argument:fixture.filter_argument,proposed_axe_rule_id:fixture.proposed_axe_rule_id,
   fixture_status:triggered&&clean?'PASS_PAIR':review&&clean&&reviewClean?'INCOMPLETE_REVIEW_PAIR':!triggered?'POSITIVE_NOT_CONFIRMED':'CONTROL_STILL_VIOLATES',
   positive:results[0],negative:results[1],vendor_mapping:'NOT_PROVEN'});
 }
 await context.close();
}finally{await browser.close();}
const result={schema_version:1,tested_at:new Date().toISOString(),axe_version:axe.version,browser_version:browser.version(),fixture_sha256:createHash('sha256').update(JSON.stringify(fixtures)).digest('hex'),viewport:{width:1280,height:800},
 scope:'Independent local browser→pinned axe rule fixture calibration. No original Screaming Frog JS rendering executed; vendor rule mapping and whole-product parity NOT_PROVEN.',
 counts:Object.fromEntries(['PASS_PAIR','INCOMPLETE_REVIEW_PAIR','POSITIVE_NOT_CONFIRMED','CONTROL_STILL_VIOLATES'].map(status=>[status,checks.filter(check=>check.fixture_status===status).length])),checks};
await writeFile(resolve(root,'docs/parity/accessibility-fixture-results.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({counts:result.counts,axe_version:axe.version,fixtures:fixtures.length,unconfirmed:checks.filter(check=>check.fixture_status!=='PASS_PAIR').map(check=>({filter:check.filter_argument,status:check.fixture_status,positive_incomplete:check.positive.incomplete}))}));
