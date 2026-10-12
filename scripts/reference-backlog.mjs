// Independent acceptance contracts from observed names and official capability descriptions.
// This script never reads proprietary method bodies or writes existing capability statuses.
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = resolve(root, 'docs/parity');
const inventory = JSON.parse(await readFile(resolve(output,'reference-cli-inventory.json'),'utf8'));
const catalog = JSON.parse(await readFile(resolve(root,'docs/feature-parity.json'),'utf8'));
const source = path => 'https://www.screamingfrog.co.uk/seo-spider/' + path;
const general = source('user-guide/general/'), tabs = source('user-guide/tabs/'), configuration = source('user-guide/configuration/');
const features = new Map(catalog.features.map(feature=>[feature.id,feature]));
const slug = value => value.toLowerCase().replace(/[^a-z0-9]+/g,'-').replace(/^-|-$/g,'');
const groupFeatures = {
  'AMP':['relationships.amp'],'Accessibility':['quality.accessibility','quality.accessibility_limit'],
  'Analytics':['integrations.ga4','integrations.provider_urls'],'Canonicals':['relationships.canonical','relationships.canonical_checks'],
  'Change Detection':['storage.compare_changes','storage.compare_mapping'],'Content':['metadata.wordcount','metadata.exact','metadata.near','metadata.readability','metadata.spelling','ai.similarity','ai.outliers'],
  'Directives':['robots.meta','robots.indexability'],'External':['crawl.scope','crawl.resources','http.response'],
  'H1':['metadata.headings'],'H2':['metadata.headings'],'Hreflang':['relationships.hreflang','relationships.hreflang_checks'],
  'Images':['images.sources','images.alt','images.size'],'Internal':['crawl.resources','http.response'],
  'JavaScript':['rendering.chromium','rendering.source_diff','rendering.console','rendering.wait'],
  'Links':['links.occurrences','links.depth','links.uncrawlable','links.positions'],'Meta Description':['metadata.descriptions'],
  'Meta Keywords':['metadata.area'],'Mobile':['quality.mobile','relationships.mobile'],'Page Titles':['metadata.titles','metadata.serp_preview'],
  'PageSpeed':['integrations.psi','integrations.cwv'],'Pagination':['relationships.pagination'],
  'Response Codes':['http.response','http.failures','http.redirects','http.refresh','robots.fetch'],
  'Search Console':['integrations.gsc','integrations.inspection'],'Security':['quality.security'],
  'Sitemaps':['sitemap.discovery','sitemap.coverage','sitemap.validity'],'Structured Data':['quality.structured_formats','quality.schema','quality.richresults'],
  'UNDEF':['ui.filter'],'URL':['crawl.normalization','links.fragments','crawl.rewrite'],
  'Validation':['quality.html_head','quality.html_validation','quality.carbon']
};
const priorityFor = tab => ['Response Codes','Directives','Page Titles','Meta Description','H1','H2','Canonicals','Hreflang','Pagination','URL','Links','Security','Validation'].includes(tab)?'R1': ['Content','Images','Internal','External','Sitemaps','Structured Data','JavaScript','Accessibility','AMP','Mobile','Change Detection'].includes(tab)?'R2':'R3';
const make = (positive, negative, expected, fields=[], parameters=[], edges=[], detail={}) => ({positive_fixture:positive,negative_fixture:negative,independent_expected:expected,data_fields:fields,filter_parameters:parameters,edge_fixture_contracts:edges,...detail});

function metadata(tab, filter) {
  const element = {'Page Titles':'title','Meta Description':'meta[name=description]','Meta Keywords':'meta[name=keywords]','H1':'h1','H2':'h2'}[tab];
  const fields = ['raw '+element+' occurrences','normalized text','source position','character count', ...(filter.includes('Pixels')?['pixel width and font provenance']:[])];
  const shared = ['Run same markup on indexable 200 and noindex 200; indexable-only preference must be explicit.', 'Repeat on raw DOM and JS-mutated DOM with source provenance.', 'Use Unicode combining sequence, emoji, escaped entity, NBSP and CRLF; retain raw count separately from normalized value.', 'Test extraction-disabled state; unavailable must not become an empty/missing result.'];
  if(filter==='Missing')return make({markup:`Omit ${element}; then provide empty and whitespace-only elements.`},{markup:`One ${element} with substantive text.`},'Report absence or empty normalized content while retaining whether an element actually existed.',fields,['indexable_only'],shared);
  if(filter==='Duplicate')return make({pages:[`/a and /b: ${element}=Identical controlled value`]}, {pages:[`/c: ${element}=Distinct value`]},'Group matching values by URL with every group member; compare indexability/pagination exclusions explicitly.',fields,['indexable_only','ignore_paginated_duplicates'],[...shared,'Case-only, leading/trailing whitespace and Unicode normalization pairs; equality policy must be recorded.']);
  if(/(?:Over|Below) X (Characters|Pixels)/.test(filter))return make({metric:'Set threshold T=60; supply measurements at T-1,T,T+1.'},{metric:'Set T=80 and re-evaluate retained measurement without refetch.'},`${filter.startsWith('Over')?'Above':'Below'} configured threshold; equality boundary is a measured reference question. Pixel widths require measured font/version, not a character multiplier.`,fields,['threshold','measurement_unit','indexable_only'],shared,{reference_calibration:['Strict versus inclusive boundary','Font family/size and glyph fallback for pixel widths']});
  if(filter==='Multiple')return make({markup:`Two and three ${element} elements with distinct and identical content.`},{markup:`Exactly one ${element}.`},'Retain occurrence count and extraction order; do not silently discard extra values.',fields,[],shared);
  if(filter==='Outside <head>')return make({markup:`Place ${element} in body or after an invalid head child.`},{markup:`Place ${element} inside valid head.`},'Separate extraction from raw source placement; HTML5 repair must not hide original placement.',fields,['perform_html_validation'],[...shared,'Invalid img/div before metadata, omitted head end tag, comments and nested template.']);
  if(filter==='Same as H1')return make({markup:'title and h1 both Independent fixture heading'},{markup:'Distinct title and h1'},'Compare retained title and first heading with explicit normalization.',[...fields,'h1'],[],shared);
  if(filter==='Alt Text in H1')return make({markup:'<h1><img src="/i.png" alt="Heading from image"></h1>'},{markup:'<h1>Visible heading</h1>'},'Store image alternative text contribution separately from directly visible heading text.',fields,[],shared);
  if(filter==='Non-Sequential')return make({markup:tab==='H1'?'<h2>Before</h2><h1>Main</h1>':'<h3>Before</h3><h2>Section</h2>'},{markup:'<h1>Main</h1><h2>Section</h2><h3>Detail</h3>'},'Preserve document-order heading levels and compare ordering; do not imply a universal ranking penalty.',fields,[],[...shared,'h1 then h3, repeated h2, multiple section h1, visually hidden headings; exact reference classification to calibrate.']);
}
const canonical = {
  'Contains Canonical':['One HTML link or HTTP Link rel=canonical','No canonical source','Retain source and resolved URL.'],
  'Self Referencing':['/a canonical to /a','/a canonical to /b','Compare document identity after URL/base resolution.'],
  'Canonicalised':['/a canonical to /b','/a canonical to /a','Flag non-self target without discarding target metadata.'],
  'Missing':['No canonical declaration','Exactly one applicable canonical','Keep configuration-disabled distinct from absent.'],
  'Multiple':['Two identical HTML canonical elements','Exactly one element','Count occurrences even when resolved targets duplicate.'],
  'Non-Indexable Canonical':['Target sequence: 404,503,301,noindex,robots blocked','Fetched indexable self-canonical 200 target','Return target state and reason; unobserved is unknown.'],
  'Multiple Conflicting':['HTML target /b and HTTP target /c','Repeated same resolved target','Report conflicting destinations and their source channels.'],
  'Canonical Is Relative':['href="../b"','href="http://{origin}/b"','Store raw href and resolved URL; relative alone is an advisory.'],
  'Unlinked':['/b discovered only through canonical','/b has a regular anchor from /a','Separate relationship discovery from anchor discoverability within captured scope.'],
  'Invalid Attribute In Annotation':['Canonical with hreflang/lang/media/type attribute','Canonical with rel and href only','Preserve each extra attribute and classify annotation applicability.'],
  'Contains Fragment URL':['Canonical target /b#part','Canonical target /b','Store raw fragment; avoid silently changing declared signal.'],
  'Outside <head>':['Canonical in body or after invalid head child','Canonical in original valid head','Use source placement and parser-repair evidence.']
};
const hreflang = {
  'Contains hreflang':['HTML, HTTP and sitemap language alternate declarations','No language alternate','Retain every source and language/target pair.'],
  'Non-200 hreflang URLs':['Target 404,503,301,robots blocked,no response','Target directly fetched 200','Record requested status separately from final landing; unknown target is not missing reciprocity.'],
  'Unlinked hreflang URLs':['Alternate discoverable only through hreflang','Alternate also has regular anchor','Bound unlinked assertion to observed anchor graph.'],
  'Missing Return Links':['A→B alternate; fetched B omits A','A→B and B→A alternate','Require observed usable B before a missing-return claim.'],
  'Inconsistent Language & Region Return Links':['A self=en; B returns A as fr','B returns A as en','Compare return label with source self-label; retain conflicts.'],
  'Non-Canonical Return Links':['B returns A2 while A2 canonicalizes to A','B returns canonical A','Resolve document aliases without hiding noncanonical declaration.'],
  'Noindex Return Links':['B returns noindex A','B returns indexable A','Evaluate applicable bot directives from HTML and HTTP.'],
  'Incorrect Language & Region Codes':['xx-ZZ,en-UK,eng,zh-Abcd-US','de-DE,en-US,zh-Hant-TW,x-default','Validate active language/script/region code tables with recorded versions.'],
  'Multiple Entries':['Same language points to B and C, or repeated source channels','One distinct target per language','Separate identical repeated declarations from conflicting targets.'],
  'Missing Self Reference':['A lists B but no A','A includes valid self-reference','Advisory with scope; optional self-reference is not automatically a search-indexing error.'],
  'Not Using Canonical':['A canonicalizes to A2 but self hreflang names A','Self hreflang names canonical A2','Compare declaration identity and canonical target.'],
  'Missing X-Default':['Only en/de alternatives','Include one x-default alternate','Optional advisory, not universal error.'],
  'Missing':['No hreflang anywhere','At least one supported source annotation','Absence can be correct for monolingual content.'],
  'Outside <head>':['HTML language link in body','Language link inside raw valid head','HTTP/sitemap alternatives have no HTML head placement requirement.']
};
const links = {
  'Pages Without Internal Outlinks':['HTML page with only external anchors','One crawlable internal anchor','Exclude image/script resources from navigational outlink count.'],
  'Internal Nofollow Outlinks':['Internal anchor rel=nofollow','Internal anchor with follow semantics','Retain occurrence attributes and page-level nofollow separately.'],
  'Internal Outlinks With No Anchor Text':['Empty anchor then linked image without alt','Visible text or accessible image alt','Do not erase occurrence merely because label is empty.'],
  'Non-Descriptive Anchor Text In Internal Outlinks':['Anchors named click here and configured custom phrase','Anchor describes destination','Use configurable phrase list and language context; match exact reference policy later.'],
  'Pages With High External Outlinks':['T+1 external anchors, duplicates and distinct URLs','T-1 external anchors','Record occurrence versus distinct URL basis and threshold comparator.'],
  'Pages With High Internal Outlinks':['T+1 internal anchors, duplicates and distinct URLs','T-1 internal anchors','Exclude resources from anchor metrics; retain separate resource counts.'],
  'Follow & Nofollow Internal Inlinks To Page':['A follows C; B links C with nofollow','Only followed or only nofollow inlinks','Classify target by all observed anchor occurrence flags.'],
  'Internal Nofollow Inlinks Only':['All incoming internal anchors to C nofollow','One followed anchor also targets C','No observed inlinks is not evidence of nofollow-only inlinks.'],
  'Pages With High Crawl Depth':['Seed0→1→2→3→4','Seed directly links target at depth1','Use shortest anchor depth with threshold default candidate4; discovery depth remains separate.'],
  'Outlinks To Localhost':['Anchors to localhost,127.0.0.1,[::1]','Normal same-site hostname','Distinguish diagnostic of production content from intentional local reference fixture.'],
  'Non-Indexable Page Inlinks Only':['Only noindex/redirect pages link to C','At least one indexable page links C','Require complete captured source states; partial crawl limits certainty.'],
  'Pages With Uncrawlable Internal Outlinks':['div href, onclick navigation, a without href, javascript href','a with HTTP href','Store suspicious navigation patterns without equating them to reliable anchor graph edges.']
};
const pagination = {
  'Contains Pagination':['HTML or HTTP rel=next/prev','No next/prev','Store source-aware relationship declarations.'],
  'First Page':['Only next declaration to page2','Only prev declaration to page1','Infer local chain role without Google indexing claims.'],
  'Paginated 2+ Pages':['Page with prev declaration','First page with next only','Retain role plus indexability; URL digits alone are insufficient.'],
  'Pagination URL Not in Anchor Tag':['next/prev targets never appear in anchors','Same target also regular anchor','Relationship discovery does not establish navigational linkage.'],
  'Non-200 Pagination URLs':['next target301/404/503/robots blocked','next target200','Requested and final status both preserved.'],
  'Unlinked Pagination URLs':['Pagination target has no internal anchor inlinks','Anchor also names target','Crawl-scope-limited graph condition.'],
  'Non-Indexable':['Paginated page has noindex or nonself canonical','Indexable self-canonical page','Record reason precedence explicitly.'],
  'Multiple Pagination URLs':['Two next targets or HTML/HTTP disagreement','One next and one prev distinct','Separate source duplicates from target conflicts.'],
  'Pagination Loop':['A next=B; B next=A; or self next','A→B→C acyclic','Detect directed cycle and emit concrete chain.'],
  'Sequence Error':['A next=B but B prev=C','A next=B and B prev=A','Differentiate absent reciprocity, conflicting return, unreachable target and unknown target.']
};
const image = {
  'Over X kB':['Payload sizes 102399,102400,102401 bytes with T=100','One-byte PNG','Preserve byte count; calibrate kB unit and strict boundary.'],
  'Missing Alt Text':['img without alt; alt=""; alt="   "','img with substantive alt','Keep absent attribute and empty value separate; repeated source occurrences required.'],
  'Missing Alt Attribute':['img omits alt','img alt=""','Do not merge omission with intentionally decorative empty alt.'],
  'Alt Text Over X Characters':['alt length T-1,T,T+1 including emoji','Short alt','Length policy and threshold recorded per occurrence.'],
  'Background Images':['CSS background-image loaded in rendered page','img resource only','Capture browser/CSS provenance; not every background requires alt.'],
  'Incorrectly Sized Images':['1000x1000 raster rendered200x200','Raster rendered at natural dimensions','Require intrinsic/display measurements and savings model; docs describe at least4kB estimated savings.'],
  'Missing Size Attributes':['img lacks width and height, then one missing','img has both valid attributes','Record occurrence-specific dimensions, srcset and rendered layout separately.']
};
const urlRules = {
  'Non ASCII Characters':['/café and /😀','/cafe','Store displayed Unicode and encoded request URL.'],
  'Underscores':['/one_two','/one-two','Flag literal underscore in configured URL component.'],
  'Uppercase':['/Upper_Case','/lower-case','Do not lowercase path/query while checking case.'],
  'Parameters':['/a?p=1','/a','Retain all query pairs and empty-value distinction.'],
  'Over X Characters':['URL lengths T-1,T,T+1','Short URL','Count encoded versus display length separately until calibrated.'],
  'Multiple Slashes':['/a//b','/a/b','Keep path double slash distinct from scheme delimiter.'],
  'Repetitive Path':['/a/a/a/a','/a/b/c','Pattern/threshold policy needs reference calibration; do not invent crawl-trap proof.'],
  'Contains Space':['Literal-space href and %20 href','No space','Keep raw href versus encoded request distinct.'],
  'Broken Bookmark':['/a#absent while fetched a lacks id/name','/a#existing with matching id','Test decoded Unicode fragment and legacy name anchors; unknown document is not broken proof.'],
  'Internal Search':['/search?q=x; configured internal-search pattern','Normal content query URL','Configured query/path patterns; no automatic exclusion without user setting.'],
  'GA Tracking Parameters':['?utm_source=x&utm_campaign=y','?product=2','Store tracking pairs; configured removal/rewrite remains separate.']
};

function specification(tab, filter) {
  if(['Page Titles','Meta Description','Meta Keywords','H1','H2'].includes(tab))return metadata(tab,filter);
  const table = {Canonicals:canonical,Hreflang:hreflang,Links:links,Pagination:pagination,Images:image,URL:urlRules}[tab];
  if(table?.[filter]) {
    const [positive,negative,expected]=table[filter];
    return make({setup:positive},{setup:negative},expected,[tab+' source records','requested/final document identity','affected source and target URLs'],['enabled','indexable_only','threshold when applicable'],
      ['Duplicate declarations/occurrences; case and whitespace variations.','Cross-origin target, redirects, blocked/no-response/truncated target: unknown must remain explicit.','Configured scope and disabled extraction are recorded.']);
  }
  if(tab==='Response Codes') {
    const scope=filter.startsWith('Internal ')?'internal':filter.startsWith('External ')?'external':'combined';
    const condition=filter.replace(/^(Internal|External) /,'');
    const scenarios = {'All':['200,301,404,503,timeout and blocked URLs','URL outside selected internal/external class'],
      'Blocked by Robots.txt':['robots Disallow matches selected UA','Equal-length Allow tie'], 'Blocked Resource':['robots blocks critical script/style/image','Allowed resource'],
      'No Response':['Local server closes socket before headers; separate timeout/refusal','Actual HTTP503 response'],
      'Success (2xx)':['200,201,204,206','301/404'], 'Redirection (3xx)':['301,302,303,307,308','200'],
      'Redirection (JavaScript)':['location assignment in rendered document','Raw page without script redirect'],
      'Redirection (Meta Refresh)':['meta http-equiv refresh URL target','No refresh element'],
      'Redirection (HTTP Refresh)':['Refresh HTTP header URL target','No Refresh header'],
      'Redirect Chain':['A301→B302→C200','A301→C200'], 'Redirect Loop':['A301→B302→A301','A301→B200'],
      'Client Error (4xx)':['400,401,403,404,410,429','200/503'], 'Server Error (5xx)':['500,502,503,504','404']};
    const pair=scenarios[condition];
    return make({scope,setup:pair?.[0]??condition},{scope,setup:pair?.[1]??'Different response class'},'Classify initial response/event; preserve every hop plus final observation. A failed transport has no fabricated HTTP status.',
      ['initial_status','final_status','redirect_hops','error_kind','robots_rule','internal/external classification'],['check_external','obey_robots','max_redirects','timeout','retry_policy'],
      ['Relative Location, percent-encoded Location, no Location, repeat hop and same-host different port.','Robots scope on every redirect; retry429/503 only under configured policy.','Each intermediate redirect URL gets its own row under reference-table profile; retain final observation separately.']);
  }
  if(tab==='Directives') {
    if(filter==='Outside <head>')return make({markup:'meta robots in body after head'},{markup:'meta robots inside raw valid head'},'Retain misplaced source declaration and effective policy separately.',['source_position','robots_directives'],['html_validation'],['Bot-specific meta name, header channel and parser head repair.']);
    const token=filter.toLowerCase().replace('max-snippet','max-snippet:0').replace('max-image-preview','max-image-preview:large').replace('max-video-preview','max-video-preview:0').replace('unavailable_after','unavailable_after: 25 Jun 2030 15:00:00 PST');
    return make({html:`<meta name="robots" content="${token}">`,http_header:`X-Robots-Tag: ${token}`},{html:'No relevant directive'},'Expose declaration by applicable bot/source; historic directives remain inventory facts without invented current Google effects.',
      ['raw_meta','raw_header','agent_scope','normalized_tokens','effective_policy'],['indexability_agent'],['Conflicting declarations; none expands to noindex/nofollow; all expands to index/follow.','Unknown bot versus selected bot, multiple HTTP fields, comma-containing unavailable_after timestamp.','Refresh is a navigation event; max-* values are not noindex by themselves.']);
  }
  if(tab==='Internal'||tab==='External')return make({scope:tab.toLowerCase(),content_type:filter,setup:'Serve a valid payload with a matching MIME type.'},{scope:tab.toLowerCase(),setup:'Other MIME type or wrong URL classification.'},'Expose resource class, declared MIME and detected type separately; missing content-type is unknown unless assumption configured.',
    ['content_type','detected_type','resource_kind','status','internal'],['assume_html_without_content_type','resource_crawl_store_flags','CDN_internal_rules'],['MIME parameters/case, extension/type disagreement, octet-stream, no Content-Type, malformed payload.','Subdomain, alternate scheme/port and configured CDN classification.']);
  if(tab==='Security') {
    const header = {'Missing HSTS Header':'Strict-Transport-Security','Missing X-Content-Type-Options Header':'X-Content-Type-Options','Missing X-Frame-Options Header':'X-Frame-Options','Missing Content-Security-Policy Header':'Content-Security-Policy','Missing Secure Referrer-Policy Header':'Referrer-Policy'}[filter];
    if(header)return make({protocol:'https',headers:{[header]:null}},{protocol:'https',headers:{[header]:header==='X-Frame-Options'?'SAMEORIGIN':header==='X-Content-Type-Options'?'nosniff':'Valid configured value'}},'Observe missing/unsafe header under explicit policy; CSP frame-ancestors versus XFO exemption requires calibration.',
      ['protocol','response_headers','security_policy'],['accepted_header_values'],['HTTP URLs, empty header, duplicated headers, report-only CSP, CSP frame-ancestors, invalid value.']);
    const pair = {
      'HTTP URLs':['HTTP URL','HTTPS URL'],'HTTPS URLs':['HTTPS URL','HTTP URL'],
      'Mixed Content':['HTTPS document references HTTP image/script/css/iframe','Only HTTPS resources'],
      'Form URL Insecure':['HTTPS form action HTTP destination','HTTPS form action HTTPS destination'],
      'Form on HTTP URL':['Form in HTTP page','Form in HTTPS page'],
      'Unsafe Cross-Origin Links':['External target=_blank without noopener/noreferrer','Same target with either protective rel'],
      'Bad Content Type':['HTML payload labeled image/png','Valid MIME aligned with body'],
      'Protocol-Relative Resource Links':['img src=//host/path','Explicit HTTPS resource URL']
    }[filter];
    return make({setup:pair?.[0]??filter},{setup:pair?.[1]??'Safe control'},'Record exact source occurrence and policy basis. Mixed navigational link and active/passive resource are distinct; legacy tab-opening advisory is not a confirmed exploit.',
      ['headers','document_protocol','resource_kind','form_action','anchor_target','anchor_rel'],[],['Relative/base URL resolution, whitespace, fragment and data URLs; rendered injected resource.']);
  }
  if(tab==='JavaScript') {
    const key=filter.includes('Page Title')?'title':filter.includes('Meta Description')?'meta description':filter.includes('H1')?'h1':filter.includes('Canonical')?'canonical':filter.includes('Noindex')?'noindex':filter.includes('Nofollow')?'nofollow':null;
    const setup=key?`Raw ${key} absent or value A; execute script to introduce/change/remove ${key} to B.`:filter.includes('Links')?'Insert anchor /js-only after script execution.':filter.includes('Content')?'Insert visible paragraph after execution.':filter.includes('Blocked')?'robots blocks a script/style needed to render.':filter.includes('Errors')?'Throw uncaught Error and issue failed resource request.':filter.includes('Warnings')?'Emit console.warn without exception.':filter.includes('Chrome')?'Trigger browser CDP audit issue and retain its code.':filter.includes('Meta Fragment')?'Raw meta name=fragment content=!': 'Raw URL #! and _escaped_fragment_= query.';
    return make({setup},{setup:key?`Raw and rendered ${key} equal.`:'Rendered document without targeted event.'},'Compare captured raw/rendered data with browser/wait/network provenance; do not invent a pass when render failed or data was truncated.',
      ['raw_metadata','rendered_metadata','raw/rendered URLs','console','network/CDP events','browser_version','viewport','wait_ms'],['rendering_mode','wait_policy','viewport','blocked_resources_policy'],['Delayed timer before/after wait threshold; navigation302/JS; external iframe; shadow DOM; CSP-blocked script; renderer timeout.','Noindex removed by JS retains raw declaration/effective-policy evidence.']);
  }
  if(tab==='Content') {
    const pairs={
      'Exact Duplicates':['Identical complete HTML at two paths; different HTML but same visible text control','Change one full-HTML byte','Group exact raw-HTML identity; reference docs identify MD5, separate independently chosen digest value from grouping equivalence.'],
      'Near Duplicates':['Distinct HTML with highly overlapping selected body text','Two unrelated selected texts','MinHash on chosen text; exact-HTML duplicates excluded; default90% reference and rounding boundary to calibrate.'],
      'Semantically Similar':['Two paraphrases with pinned embeddings','Unrelated topic embeddings','Provider/model/version/vector provenance and threshold; never infer from metadata equality.'],
      'Low Relevance Content':['One outlier topic among a coherent cluster','Cluster-central page','Record embeddings, cohort and scoring model; reference scale remains a calibration question.'],
      'Low Content Pages':['Selected body token count T-1,T,T+1','Substantive body above T','Use configured text area and word policy; nonindexable preference explicit.'],
      'Readability Difficult':['Long multi-clause English paragraph','Plain short English sentences','Record Flesch calculation inputs, language and threshold; precise tokenizer/syllables need calibration.'],
      'Readability Very Difficult':['High-complexity paragraph around lower Flesch boundary','Plain short paragraph','Distinct difficult/very-difficult ranges; non-English should not receive unsupported English score.'],
      'Lorem Ipsum Placeholder':['Selected body contains controlled lorem ipsum passage','Natural copy only','Configurable placeholder matching and source text evidence.'],
      'Soft 404 Pages':['HTTP200 body contains controlled missing-page phrase','Same phrase in useful article plus actual404 error page','Candidate classification with configured phrase list; HTTP status preserved and context review required.'],
      'Spelling Errors':['Selected en-GB text contains controlled misspelling; add it to ignore dictionary','Corrected text','LanguageTool/rule/dictionary version and text offsets; disabled checker is unavailable.'],
      'Grammar Errors':['Selected known-language text violates enabled grammar rule','Corrected text or disabled rule','Rule ID/offset/message and post-crawl rerun without refetch; track false-positive ignore.']
    }[filter];
    return make({setup:pairs?.[0]??filter},{setup:pairs?.[1]??'Control'},pairs?.[2]??'Calibrate text classifier.',
      ['selected_body_text','raw_html_digest','token_counts','language','pair/cohort scores'],['content_area','threshold','indexable_only','provider/model','dictionary_rules'],['Boilerplate-only equality versus main-content difference; empty/text-only pages; Unicode and language boundaries.','Truncated/unavailable body cannot establish similarity; persist algorithm/config versions.'],
      {reference_calibration:filter.includes('Similar')||filter.includes('Relevance')?['Exact embedding scoring/model response not proven.']:filter==='Near Duplicates'?['Token shingle size, MinHash seed/permutation count, rounding before/after threshold.']:[]});
  }
  if(tab==='Sitemaps') {
    const pair={
      'URLs in Sitemap':['Crawled /a listed in sitemap','Crawled /b absent from all loaded sitemaps'],
      'URLs not in Sitemap':['Crawled /b absent from configured sitemap set','Crawled /a listed'],
      'Orphan URLs':['/orphan listed only in sitemap and fetched through sitemap import','/linked listed and anchor-discovered'],
      'Non-Indexable URLs in Sitemap':['Sitemap URL404/301/noindex/canonicalized/blocked','Indexable direct200 sitemap URL'],
      'URLs in Multiple Sitemaps':['Same /a occurs in two sibling sitemap documents','Only one sitemap contains /a'],
      'XML Sitemap with over 50k URLs':['Documents with49999/50000/50001 loc elements','Small valid sitemap'],
      'XML Sitemap over 50MB':['Uncompressed XML bytes just below/equal/above50MiB','Small XML; compress large one separately']
    }[filter];
    return make({setup:pair?.[0]??filter},{setup:pair?.[1]??'Control'},'Retain sitemap membership/provenance and every affected URL; large-format validation can be tested without fetching >500 reference URLs.',
      ['sitemap_documents','membership','uncompressed_bytes','loc_count','URL_status/indexability'],['sitemap_sources','crawl_linked_sitemaps','max_documents'],['Nested indexes/cycles, gzip, malformed XML, relative loc, duplicate entries, xhtml alternates.','Orphan detection limited to captured graph; no data from unconfigured sitemap is not absence proof.']);
  }
  if(tab==='Structured Data') {
    const pair={
      'Contains Structured Data':['One supported structured-data item','No supported markup'], 'Missing':['No supported markup','One item'],
      'Validation Errors':['Unknown Schema type/property and invalid range','Valid type/property/range'],
      'Validation Warnings':['Deprecated/pending vocabulary or nonfatal semantic defect','Supported clean item'],
      'Rich Result Validation Errors':['Recognized feature with missing mandatory property','Complete mandatory properties'],
      'Rich Result Validation Warnings':['Feature missing recommended property','Recommended property present'],
      'Parse Errors':['Malformed JSON-LD trailing comma plus unclosed item','Valid JSON-LD'],
      'Microdata URLs':['itemscope/itemtype/itemprop example','Equivalent plain HTML'],
      'JSON-LD URLs':['application/ld+json object,array,@graph','Ordinary JSON script only'],
      'RDFa URLs':['vocab/typeof/property example','No RDFa attributes'],
      'Rich Result Feature Detected':['Supported Product/Article feature declaration','Unknown/unsupported schema type']
    }[filter];
    return make({setup:pair?.[0]??filter},{setup:pair?.[1]??'Control'},'Separate parse errors, vocabulary validation and versioned search-feature requirements. Recognized type is not Google eligibility proof.',
      ['format','item_graph','types','properties','parse_errors','validator_namespace/version','severity','item/property path'],['formats','schema_validation','rich_result_validation','indexable_only'],['Multiple scripts, top arrays, @graph references, @id cycles, language/value maps, escaped script closing sequences.','Disabled vocabulary/rich-result validator must return unavailable, not zero errors.']);
  }
  if(tab==='Validation') {
    const pair={
      'Invalid HTML Elements in <head>':['img/div before title inside original head','Only valid metadata head children'],
      '<head> Not First In <html> Element':['html child p precedes explicit head','head is first element; comment/whitespace precede it'],
      'Missing <head> Tag':['Original source omits head tags','Explicit one head'], 'Multiple <head> Tags':['Two explicit head start tags','One head'],
      'Missing <body> Tag':['Original source omits body tag','Explicit one body'], 'Multiple <body> Tags':['Two explicit body tags','One body'],
      'HTML Document Over XMB':['HTML decoded bytes T-1,T,T+1','Small HTML'], 'Resource Over XMB':['Binary asset bytes T-1,T,T+1','Small resource'],
      '<body> Element Preceding <html>':['body start before html start','html precedes body'],
      'High Carbon Rating':['Large transferred payload under pinned carbon model','Small transfer under same model']
    }[filter];
    return make({setup:pair?.[0]??filter},{setup:pair?.[1]??'Control'},'Use source token positions for structural checks; repaired DOM alone cannot prove source tags. Byte limits and carbon modeling require versioned units/model.',
      ['raw_source_tokens','DOM_repair','decoded_bytes','transferred_bytes','carbon_model'],['perform_html_validation','size_thresholds','green_hosting'],['Comment/script/template pseudo-tags excluded; script-disabled noscript in head; malformed nesting; compressed response.'],
      {reference_calibration:filter==='High Carbon Rating'?['Carbon equation, energy assumptions, green-host factor and rating boundaries.']:[]});
  }
  if(tab==='Change Detection')return make({snapshots:[`Before/after change only ${filter}; preserve URL identity.`]},{snapshots:['Same values at different times and response latency.']},'Show changed field and both values under stable URL mapping; normalize volatile timestamps, not meaningful data.',
    [filter,'before/after crawl ID','URL_mapping','raw/rendered provenance'],['change_threshold','URL_mapping_rules'],['Added/removed URLs, scheme/host migration, missing before/after value, reorder of sets versus ordered occurrences.','Content differences require persisted body, not title/count proxy.']);
  if(tab==='Analytics'||tab==='Search Console') {
    const metric=filter.includes('Sessions')?'sessions':filter.includes('Bounce')?'bounce_rate':filter.includes('Clicks')?'clicks':null;
    return make({provider:tab,record:metric?{[metric]:filter.includes('70')?0.71:1}:{classification:filter,data_available:true}},{provider:tab,record:metric?{[metric]:0}:{data_available:false,error:'Access denied or no matching data'}},'Join authorized provider data by recorded URL/property/date/device scope. No rows, zero metric, unavailable API and permission error remain distinct.',
      ['provider','property','URL','date_range','metrics','inspection_verdict','raw_provider_error'],['authorized_account','property','period','row_limits','URL_normalization','inspection_enabled'],['Provider-only orphan URLs, canonical mismatch, excluded pages with positive data, inspection quota exhausted.','Mock transport proves parsing only; no live integration/parity claim without authorized account and response evidence.'],{reference_calibration:['Exact provider verdict-to-filter mapping and absence semantics.']});
  }
  if(tab==='PageSpeed'||tab==='Mobile')return make({setup:`A controlled page that triggers ${filter} in a pinned local Lighthouse or authorized PSI response.`},{setup:'Corrected page and separate audit-not-applicable response.'},'Map versioned upstream audit ID/details to the named view; retain score, opportunities, request errors and provider failure separately.',
    ['upstream_audit_id','Lighthouse_version','device','score','numeric_value/units','detail_rows','error'],['PSI_key/authorized_mode','device','category','timeout'],['Desktop/mobile differences, throttling, run variance, deprecated/replaced audit IDs, CrUX field data absent.'],{reference_calibration:['Bind installed filter to exact upstream audit ID; display name alone does not establish it.']});
  if(tab==='AMP')return make({setup:`Create independent AMP document with only the ${filter} condition; for transport/relationship classes use dedicated status/canonical/amphtml targets.`},{setup:'Complete valid paired AMP/non-AMP pages with direct indexable200 targets.'},'Use official version-pinned AMP validator plus independent pair/target audit; distinguish parse/validator/relationship failures.',
    ['amphtml','canonical','non_amp_return','target_status','validator_version','validator_code/line'],['AMP_extract/crawl','validator_ruleset'],['HTTP Link declarations; redirect landing; malformed boilerplate; script version; validator unavailable.'],{reference_calibration:['Exact upstream validator issue-to-filter mapping and combined-error precedence.']});
  if(tab==='Accessibility') {
    const aggregate=filter.includes('Violation')||filter.startsWith('Accessibility Score');
    return make({setup:aggregate?`Pinned axe result contains a ${filter} category/score boundary; add incomplete and untested results separately.`:`Independently authored rendered DOM violates only this accessibility condition: ${filter}. Add a second matching node and one hidden node.`},
      {setup:'Repair the specific DOM condition; unrelated violations may remain.'},'Retain pinned axe rule IDs, impacts, tags, node selectors/html and incomplete results. Map names to rule IDs explicitly; do not claim automated checks establish complete WCAG compliance.',
      ['axe_version','rule_id','impact','tags','violations.nodes','incomplete','passes','inapplicable'],['accessibility_enabled','rendering_required','rule_set','viewport'],['Visible/hidden/disabled element, iframe, open shadow root, repeated nodes and duplicate IDs.','Score equation and score boundaries are reference-calibration requirements; no invented vendor score from violation count.'],
      {reference_calibration:aggregate?['Exact scoring equation/denominator and vendor category mapping.']:['Rule ID mapping to installed reference engine/version','Positive/negative DOM fixture must be executed before contract is finalized.'],contract_status:aggregate?'PROBE_DEFINED':'RULE_FIXTURE_DESIGN_REQUIRED'});
  }
  return make({setup:`Controlled ${tab}/${filter} state.`},{setup:'No matching state.'},'Capture actual classification before specifying a compatibility predicate.',[tab+' raw data'],[],['Unsupported/unknown class must remain explicitly unknown.'],{contract_status:'PROBE_DEFINED',reference_calibration:['Undocumented fallback behavior.']});
}

const rows = inventory.export_tab_entries.filter(entry=>entry.kind==='named_filter').map(entry=>{
  const ids=groupFeatures[entry.tab]??[];
  const spec=specification(entry.tab,entry.filter);
  return {id:'filter.'+slug(entry.argument),reference_argument:entry.argument,tab:entry.tab,filter_name:entry.filter,
    kind: ['Internal','External'].includes(entry.tab)?'resource_view':entry.tab==='Response Codes'&&/All|Success|Redirection/.test(entry.filter)?'status_view':'audit_or_analysis_view',
    priority:priorityFor(entry.tab),catalog_feature_ids:ids,contract_status:'DRAFT_CONCRETE',...spec,
    acceptance_basis:'Independently authored test proposal from observed CLI argument identity and official product documentation. Exact reference predicate is not assumed.',
    reference_evidence:{availability:'OBSERVED_CLI_HELP',filter_membership:'NOT_PROVEN',column_observations:'docs/parity/reference-fixture/normalized-observations.json where corresponding fields exist'},
    independent_evidence:{filter_status:'NOT_ASSESSED_INDIVIDUALLY',related_capability_slices:ids.map(id=>({id,status:features.get(id)?.status??'UNKNOWN',implementation_evidence:features.get(id)?.implementation_evidence??[]})),
      policy:'Related capability evidence is context, not proof this filter or its edge cases are implemented. No automatic missing/planned assignment.'},
    source_urls:[tabs,configuration],parity_status:'NOT_PROVEN'};
});
assert.equal(rows.length,416); assert.equal(new Set(rows.map(row=>row.id)).size,416);
const sourceOverrides = {
  'filter.canonicals-invalid-attribute-in-annotation':source('issues/canonicals/invalid-attribute-in-annotation/'),
  'filter.canonicals-outside-head':source('issues/canonicals/canonicals-outside-head/'),
  'filter.page-titles-outside-head':source('issues/page-titles/page-title-outside-head/'),
  'filter.meta-description-outside-head':source('issues/meta-description/meta-description-outside-head/'),
  'filter.directives-outside-head':source('issues/directives/directive-outside-head/'),
  'filter.validation-invalid-html-elements-in-head':source('issues/validation/invalid-html-elements-in-head/'),
  'filter.validation-head-not-first-in-html-element':source('issues/validation/head-not-first-in-html-element/'),
  'filter.images-incorrectly-sized-images':source('issues/images/incorrectly-sized-images/'),
  'filter.images-missing-alt-text':source('issues/images/missing-alt-text/'),
  'filter.content-soft-404-pages':source('issues/content/soft-404-pages/'),
  'filter.content-near-duplicates':source('issues/content/near-duplicates/'),
  'filter.content-exact-duplicates':source('issues/content/exact-duplicates/'),
  'filter.links-pages-with-high-crawl-depth':source('issues/links/pages-with-high-crawl-depth/'),
  'filter.links-pages-with-uncrawlable-internal-outlinks':source('issues/links/pages-with-uncrawlable-internal-outlinks/'),
  'filter.hreflang-missing-return-links':source('issues/hreflang/missing-return-links/'),
  'filter.structured-data-validation-errors':source('issues/structured-data/validation-errors/'),
  'filter.security-unsafe-cross-origin-links':source('issues/security/unsafe-cross-origin-links/'),
  'filter.sitemaps-xml-sitemap-over-50mb':source('issues/sitemaps/xml-sitemap-over-50mb/'),
  'filter.search-console-rich-result-invalid':source('issues/search-console/rich-result-is-invalid/')
};
for(const row of rows)if(sourceOverrides[row.id])row.source_urls=[sourceOverrides[row.id]];
let accessibilityCalibration = null;
try {
  const fixtures = JSON.parse(await readFile(resolve(output,'accessibility-fixtures.json'),'utf8'));
  const results = JSON.parse(await readFile(resolve(output,'accessibility-fixture-results.json'),'utf8'));
  const checks = new Map(results.checks.map(check=>[check.filter_argument,check]));
  for(const fixture of fixtures.fixtures) {
    const row = rows.find(item=>item.reference_argument===fixture.filter_argument);
    const check = checks.get(fixture.filter_argument);
    assert.ok(row && check, `Accessibility tracker binding missing: ${fixture.filter_argument}`);
    row.positive_fixture = { html:fixture.positive_html,expected_signal:check.fixture_status==='INCOMPLETE_REVIEW_PAIR'?'incomplete/manual review':'violation',rule_id:fixture.proposed_axe_rule_id };
    row.negative_fixture = { html:fixture.negative_html,expected_signal:'Target rule absent from violations and incomplete',rule_id:fixture.proposed_axe_rule_id };
    row.contract_status = ['PASS_PAIR','INCOMPLETE_REVIEW_PAIR'].includes(check.fixture_status)?'DRAFT_CONCRETE_WITH_INDEPENDENT_ENGINE_FIXTURE':'RULE_FIXTURE_REFINEMENT_REQUIRED';
    row.reference_calibration = ['Binding to original engine/version and original filter membership remain NOT_PROVEN.','Axe incomplete/manual-review results cannot be silently converted into an automated failure or pass.'];
    row.independent_evidence.engine_fixture = {result:check.fixture_status,rule_id:fixture.proposed_axe_rule_id,axe_version:results.axe_version,browser_version:results.browser_version,
      tested_at:results.tested_at,fixture_sha256:results.fixture_sha256,proof:'docs/parity/accessibility-fixture-results.json',positive:check.positive,negative:check.negative,
      scope:'Independent pinned axe rule execution in Chrome; product UI/API rule selection and original filter mapping are not tested by this run.'};
    row.source_urls.push('https://github.com/dequelabs/axe-core/blob/develop/doc/rule-descriptions.md','https://github.com/dequelabs/axe-core/blob/develop/doc/API.md');
  }
  accessibilityCalibration = {counts:results.counts,proof:'docs/parity/accessibility-fixture-results.json',fixtures:'docs/parity/accessibility-fixtures.json',vendor_mapping:'NOT_PROVEN'};
} catch(error) { if(error.code!=='ENOENT')throw error; }
let referenceMembership = null;
try {
  const measured = JSON.parse(await readFile(resolve(output,'reference-membership/latest-run.json'),'utf8'));
  for(const observation of measured.membership) {
    const row = rows.find(item=>item.reference_argument===observation.filter_argument);
    assert.ok(row, `Reference membership inventory binding missing: ${observation.filter_argument}`);
    row.reference_evidence.filter_membership = 'OBSERVED_LOCAL_FIXTURE_MEMBERSHIP';
    row.reference_evidence.membership_run = {proof:'docs/parity/reference-membership/latest-run.json',fixture:'docs/parity/reference-membership/fixture-cases.json',
      observed_at:measured.observed_at,fixture_sha256:measured.fixture_sha256,file:observation.file,file_sha256:observation.file_sha256,paths:observation.paths,
      pair_status:observation.proof_status,case_checks:observation.case_checks,refined_control:observation.refined_control??null,refined_pair_status:observation.refined_pair_status??null,
      threshold_observation:observation.threshold_observation,
      scope:'Observed original named-filter membership on authored cases under unchanged parameters. Empty sets do not prove full rule behavior. Independent filter-predicate comparison remains NOT_PROVEN.'};
  }
  referenceMembership = {counts:measured.counts,proof:'docs/parity/reference-membership/latest-run.json',profile_parameters_unchanged:measured.profile.changed_keys.length===0,independent_filter_comparison:'NOT_PROVEN'};
} catch(error) { if(error.code!=='ENOENT')throw error; }

const workflowContracts = [
  ['crawl.robots-agent-groups','R1',['robots.matching'],'robots: wildcard+specific agents, Allow/Disallow ties, encoded path, redirects and missing/503 robots.','Selected-agent request log proves denied targets never fetched; fail-policy explicitly recorded.'],
  ['crawl.url-identity','R1',['crawl.normalization'],'Relative/base href, fragments, IDN, default ports, percent encoding, query order/duplicates and trailing slash.','Only specified normalizations deduplicate; path/query case or order changes are not silently merged.'],
  ['crawl.scope-credentials','R1',['crawl.scope','crawl.headers_proxy'],'Two localhost origins with custom Authorization header; same-origin redirects versus foreign redirect/resources.','No configured credentials reach foreign origin; known request provenance retained.'],
  ['crawl.include-exclude','R1',['crawl.include_exclude'],'Explicit seed exempt, overlapping include/exclude expressions, sitemap-discovered URL, invalid regex.','Apply validated policy before enqueue; no network request for excluded discovery.'],
  ['crawl.request-retry','R1',['crawl.timeouts_retries'],'503 twice then200;429 Retry-After; closed socket; streaming body stalls.','Bounded configured retry/backoff, error category and request counts; no fake zero status.'],
  ['crawl.user-resource-limits','R1',['crawl.limits','crawl.speed'],'URL cap/depth/path sampling, max bytes, cancel while workers active, per-origin throttling.','User limits are provenance; lower or absent limit preserves no commercial cap in independent engine.'],
  ['data.request-and-final-document','R1',['http.response','http.redirects'],'A301→B302→C200 with C title/canonical/noindex and A Location/header.','Keep initial response and final document observations separate. Reference-table display can hide final metadata on A without deleting C observation.'],
  ['data.redirect-intermediate-rows','R1',['http.redirects'],'A→B→C; B only encountered as redirect hop and later as direct seed.','Hop ledger always retained; compatibility table adds B row with its initial302 once; direct fetch provenance remains distinguishable.'],
  ['data.anchor-resource-metrics','R1',['links.occurrences','crawl.resources'],'Page contains three img occurrences, two repeated anchors, one canonical and one stylesheet.','All resources retained by kind; reference Links view counts anchors only. Separate occurrence and unique-target/source metrics.'],
  ['data.resource-indexability','R1',['robots.indexability','images.sources','images.pdf'],'200 PNG, PDF, XML, HTML plus header-noindex variants.','Separate crawler fetchability, Google-style resource indexing eligibility, HTML document indexability and policy profile; non-HTML is not intrinsically non-indexable.'],
  ['analysis.independent-link-score','R2',['links.score'],'Directed graph with dangling node, self-loop, repeated edges, nofollow, page nofollow, partial boundary.','Deterministic PageRank facts/config and scope; reference LinkScore requires algorithm calibration and cannot be claimed from normalized rank alone.'],
  ['analysis.orphan-source-cohort','R2',['links.orphans','sitemap.coverage','integrations.provider_urls'],'Sitemap-only, GA-only/GSC-only, self-link-only, partial-crawl and fetched anchor-discovered candidates.','Observed cohort and graph scope bound claim; unavailable integration cannot prove no orphan.'],
  ['analysis.content-area','R2',['metadata.area','metadata.wordcount'],'Header/nav/footer identical; main text differs; CSS selector include/exclude and content types HTML/PDF.','Store selected text/selection config and tokenization; keep full-HTML digest separate from text similarity.'],
  ['rendering.wait-interactions','R2',['rendering.wait','rendering.interact'],'Timer after load, fetch-inserted DOM, click/scroll action, infinite requests, post-action navigation.','Bounded actions/wait with recorded browser/config; distinguish wait elapsed, script failure and completed observation.'],
  ['rendering.iframes-shadow-dom','R2',['rendering.flatten'],'Same/foreign-origin iframe and open/closed shadow roots with anchors/metadata.','Source provenance retained, explicit accessibility/browsing boundaries; unsupported closed root is unknown.'],
  ['extraction.css-xpath-regex','R2',['rendering.custom_extract'],'Text/innerHTML/outerHTML/attribute; zero/multiple values; XPath text/attr/nodes; regex capture groups.','Validate before crawl; columns remain stable; unavailable/invalid extraction distinct from zero match.'],
  ['extraction.custom-search','R2',['rendering.custom_search'],'Contains/not-contains, literal/regex, raw/rendered HTML/text, case sensitivity, overlapping matches.','Named filter results with scope/config retained; binary/nonHTML and disabled extraction explicit.'],
  ['extraction.custom-javascript','R2',['rendering.javascript','rendering.snippet_library'],'Action then extraction promise; throw/reject, selector missing, primitive/object return and internal-only policy.','Return structured per-snippet result/error; no implicit API messages or cross-origin credentials.'],
  ['storage.stop-resume-crash','R1',['storage.database','storage.snapshots','modes.control'],'Pause/cancel, terminate actual process midfrontier, restart/resume, disk-write failure.','Completed URL state/frontier/config persisted; user-visible resumable state and no refetch of completed seed.'],
  ['storage.project-config-auth','R2',['storage.projects','storage.config','storage.portable'],'Export/import project with named config, credentials redacted, relative paths and schema upgrade.','Versioned owned format, portable nonsecret state, no proprietary .seospider compatibility claim without separate proof.'],
  ['storage.compare-mapping','R2',['storage.compare_mapping','storage.compare_export'],'Before live host to staging host/path rewrite; added/removed; redirect identity; fields reordered.','Explicit mapping profile and normalized differences; mapping collision is error, not silent merge.'],
  ['auth.basic-digest-form','R3',['crawl.headers_proxy','integrations.oauth'],'Owned fixture401 Basic/Digest and session form requiring cookie/CSRF; failed auth and logout.','Authorized credentials only exact allowed origin; no login/2FA/security prompt automation or license change.'],
  ['provider.oauth-token-lifecycle','R3',['integrations.oauth'],'Provider mock with expiry/refresh/revoke, permission denied, quota and pagination.','Provider parsing proof distinct from authorized live proof; no token in repo/export/log.'],
  ['provider.psi-cwv','R3',['integrations.psi','integrations.cwv'],'Device-specific PSI/Lighthouse audit sets and missing CrUX field cohort.','Pinned lab versus field provenance; missing cohort is unavailable, not zero.'],
  ['provider.backlinks','R3',['integrations.majestic','integrations.ahrefs','integrations.moz'],'Owned/mock URL lookup with redirect/canonical mismatch and quota errors.','Service-specific metrics/units/account/query timestamp retained; account fees/quotas not bypassed.'],
  ['ai.prompts-embeddings','R3',['ai.providers','ai.prompts','ai.embeddings','ai.usage'],'Prompt input containing untrusted page instructions; embedding vectors/missing model/quota/usage.','Provider/key authorization, returned model/vector dimensions/version and usage; no silent external call from crawl default.'],
  ['ui.filter-details-export','R1',['ui.filter','ui.details','ui.overview','automation.tables'],'Select one URL/issue, AND/OR search across columns, stable sort/pagination, CSV/XLSX export with commas/newlines/formulas.','View count matches exact serialized predicate; source/target lower rows reachable; export security and quoting tested.'],
  ['ui.graphs-segments','R2',['links.crawl_graph','links.directory_graph','links.graphs_3d','links.segments'],'Crawl/directory/content cluster layouts with selected segment and large partial graph.','Topology versus layout separated; SVG/image/HTML export and segment membership explicit; drawing alone is not graph-algorithm proof.'],
  ['automation.cli-scheduler-mcp','R2',['automation.cli','automation.scheduler','automation.mcp','automation.mcp_transport'],'Headless CLI nonzero failure, overlapping schedules/timezones, MCP stdio/HTTP lifecycle and tool errors.','Same config/data behavior across transports; schedule persists/restarts; actual SDK tests bound protocol claims.'],
  ['automation.reports-assets','R2',['automation.reports','automation.assets','automation.notifications','automation.looker'],'Multiple export tabs, screenshot/source archive, empty report skip, folder timestamping, notification disabled.','Owned deterministic artifacts and separate explicit authorization for any external message/cloud write.'],
  ['modes.list-sitemap-serp-api','R2',['modes.list','modes.serp','modes.apis'],'URL list dedupe, sitemap index import, SERP preview with selected device, integration-only URL cohort.','Explicit traversal rules/mode provenance; preview is not measured Google search appearance.'],
  ['platform.native-wsl-container','R2',['storage.portable','automation.cli'],'Native Windows release, Ubuntu CI, browser installation and Docker start/stop/data volume.','Actual install/run/protocol evidence per platform; absent Docker/WSL run stays NOT_PROVEN.']
].map(([id,priority,ids,input,expected])=>({id,priority,catalog_feature_ids:ids,fixture_contract:input,independent_expected:expected,
  acceptance_status:'DRAFT_NOT_EXECUTED_AS_A_COMPLETE_WORKFLOW',reference_parity:'NOT_PROVEN',
  related_capability_statuses:ids.map(id=>({id,status:features.get(id)?.status??'UNKNOWN'})),source_urls:[configuration,general]}));
const profiles = [
  {id:'native-evidence',policy:'Store requested-response and final/rendered-document data independently; preserve all link/resource kinds and all reasons. Do not discard richer observations to imitate a table.'},
  {id:'reference-table-candidate',policy:'Initial redirect/error rows expose their requested status; final metadata shown in dedicated final-document fields. Add every intermediate redirect URL to compatibility table. Count anchors only in Links-view outlinks; expose IMG/resource metrics separately.'},
  {id:'resource-indexability-candidate',policy:'Document indexability and resource indexing eligibility are separate booleans/reasons. PNG/JPEG/PDF can be indexable resources; content type alone is not a noindex directive.'},
  {id:'reason-taxonomy-candidate',policy:'Map localized reference reason names to semantic codes while retaining all underlying reasons and precedence. Cosmetic label match is separate from decision match.'},
  {id:'measurement-calibration',policy:'Pixel width, content tokenization, rank, MinHash and score boundaries require recorded algorithm/config/version. Approximate measurements must never masquerade as calibrated reference values.'}
];
const result = {schema_version:1,created_at:new Date().toISOString(),project:'Open SEO Spider',reference_version:'24.3',
  purpose:'1:1 tracker for every installed named CLI filter plus concrete workflow and edge-case proposals; this expands but does not overwrite the 144 grouped capability catalog.',
  full_product_parity:'NOT_PROVEN',source_inventory:'docs/parity/reference-cli-inventory.json',
  counts:{named_filter_records:rows.length,workflow_contracts:workflowContracts.length,
    contract_statuses:Object.fromEntries([...new Set(rows.map(row=>row.contract_status))].map(status=>[status,rows.filter(row=>row.contract_status===status).length])),
    priorities:Object.fromEntries(['R1','R2','R3'].map(priority=>[priority,rows.filter(row=>row.priority===priority).length]))},
  count_policy:'416 named filter views are not 416 unique algorithms or all GUI features. 857 matching cells from one retained comparison are neither features nor a coverage percentage. Configured slots and All aggregates are tracked separately in the CLI inventory.',
  status_policy:'No filter is automatically marked missing/planned/implemented. Individual membership must be measured. Related catalog evidence shows verified slices only and does not transfer acceptance status.',
  mapping_profiles:profiles,accessibility_calibration:accessibilityCalibration,reference_membership:referenceMembership,filters:rows,workflow_contracts:workflowContracts,
  slot_templates:[...new Set(inventory.export_tab_entries.filter(entry=>entry.kind==='configured_slot').map(entry=>entry.tab+':'+entry.instance_template))].map(template=>({template,acceptance:'Independent named configuration, ordered outputs, zero/multiple result values, disabled/failed states and roundtrip persistence.',parity_status:'NOT_PROVEN'})),
  static_architecture_context:{artifact_sha256:'5b6fb252e194c9702e6ba8f6bf589e9870175e1478bcd6148a3f8068d4e18d76',
    retained_observations:['5,400 application classes and395 packaged JARs.','Typed jsoup DOM parsing and CDP browser snapshot bridge.','HTTP client integrations and CDP Network/Fetch/DOMSnapshot/Audits responsibilities.','JDBC Derby/SQLite adapter paths plus typed duplicate categories.','Spring/MCP streamable servlet and stdio transport references.'],
    boundary:'Retained static metadata establishes compiled couplings/presence, not active runtime algorithms, ownership, settings or full behavior. Original normal local CLI now directly observed Derby shutdown for its own fixture database. No vendor source/assets copied.'},
  limitations:['Accessibility rule fixtures are independently calibrated where recorded; unresolved signal/control cases and original rule-ID/filter binding still require proof.','AMP, PSI/mobile and provider-specific rules require versioned upstream responses/validators and authorized live gates where relevant.','Public documentation and observed labels may disagree; preserve the installed argument identity and flag calibration questions.']};
await mkdir(output,{recursive:true});
await writeFile(resolve(output,'granular-acceptance.json'),JSON.stringify(result,null,2)+'\n');
const lines=['# Granular acceptance backlog','',`Created ${result.created_at}; reference24.3. Full vendor parity **NOT PROVEN**.`,
  '',`${rows.length} installed named filters each have one stable tracker record; ${workflowContracts.length} additional workflow contracts cover behavior outside filters. Existing 144-capability statuses are untouched.`,
  '', 'A filter record separates observed argument availability, proposed positive/control inputs, data/parameters, edge cases, related implementation evidence and unmeasured vendor membership. Existing implemented/partial/planned capability slices are retained as context; they are not inherited as per-filter statuses.',
  '', '**The 857 matching cells in the first measured comparison are exported cell comparisons, not features and not a product coverage percentage.**',
  '', '| Priority | Named filter records | Meaning |','| --- | ---: | --- |',
  ...Object.entries(result.counts.priorities).map(([priority,count])=>`| ${priority} | ${count} | ${priority==='R1'?'Status, robots, metadata, URL and relationship correctness':priority==='R2'?'Resource/content/browser/validator and comparison detail':'Authorized upstream metrics and verdicts'} |`),
  '', '| Tab | Named views | Contract detail |','| --- | ---: | --- |',
  ...[...new Set(rows.map(row=>row.tab))].map(tab=>`| ${tab} | ${rows.filter(row=>row.tab===tab).length} | ${tab==='Accessibility'&&accessibilityCalibration?'Authored DOM pairs + actual pinned axe signal/control evidence; original mapping unproven':rows.filter(row=>row.tab===tab&&row.contract_status==='RULE_FIXTURE_DESIGN_REQUIRED').length?'Per-rule DOM design and pinned axe binding still required':'Individual proposed trigger/control/edge contracts in JSON'} |`),
  ...(accessibilityCalibration?['','Independent accessibility fixture results: '+Object.entries(accessibilityCalibration.counts).map(([name,count])=>`${name}=${count}`).join(', ')+'. Manual-review signals remain separate from automated violations. See `accessibility-fixtures.json` and `accessibility-fixture-results.json`.']:[]),
  ...(referenceMembership?['','Original named-filter membership: '+Object.entries(referenceMembership.counts).map(([name,count])=>`${name}=${count}`).join(', ')+'. See `reference-membership/membership.md`; unknown eligibility and contradictory proposals remain recorded, without changing implementation or parity statuses.']:[]),
  '', '## Candidate data/display profiles','',...profiles.flatMap(profile=>[`- **${profile.id}**: ${profile.policy}`]),
  '', '## Additional workflow acceptance','',...workflowContracts.flatMap(workflow=>[`- **${workflow.id} (${workflow.priority})** — ${workflow.fixture_contract} Expected: ${workflow.independent_expected}`]),
  '', '## Source and proof boundaries','',
  '- Current installed help is retained as names/metadata only in `reference-cli-inventory.json`:1149 arguments across34 tabs,416 named filters,700 configurable slots,33 All aggregates.',
  '- `fixture-cases.json` contains original independent fixture HTML/transport cases. `reference-comparison.json` retains actual values and individual mismatches for both engines.',
  '- The original normal CLI succeeds on the small local fixture with missing licence. Paid feature/config entitlement, proprietary project-file compatibility and complete reference filter behavior are not established.',
  '- Existing static JAR metadata is architectural context; it is not execution evidence or recovered proprietary algorithms.',
  '', 'Official primary sources for reviewed specifics:',...Object.entries(sourceOverrides).map(([id,url])=>`- [${id}](${url})`),
  '', `General references: [configuration](${configuration}), [tabs](${tabs}), [normal CLI](${general}).`,
  '', 'Regenerate with `node scripts/reference-backlog.mjs`; validate that exactly416 unique named arguments remain mapped. Do not count unresolved contract design as implementation work completed.'];
await writeFile(resolve(output,'granular-acceptance.md'),lines.join('\n')+'\n');
console.log(JSON.stringify({outputs:['docs/parity/granular-acceptance.json','docs/parity/granular-acceptance.md'],counts:result.counts}));
