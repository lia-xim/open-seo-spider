# Exact CDN caller capture and bounded static findings

Status: EXACT_CAPTURE_AND_CALLSITE_MAPPING_COMPLETE_ADDITIONAL_FIELDS_UNKNOWN.

The twelve previously selected caller classes are now available as real shipped bytes. This resolves the old bundle's missing-caller-byte boundary. It does not resolve individual additional HTML fields or CDN-specific audits. The earlier `968708a01448a9452132080281a8d6abbc010c1f0b4c6d398d16fe302b740e04` UNKNOWN plan is preserved unchanged.

## Capture and authority

Root explicitly authorized read-only access to the one confirmed installed archive. The actual before and after reads both measured 13,824,663 bytes and SHA-256 `5b6fb252e194c9702e6ba8f6bf589e9870175e1478bcd6148a3f8068d4e18d76`. Regular-file identity, size, timestamp and file identity also agree. Precisely twelve named class members were materialized to E:, totaling 173,058 bytes. Every expected member size, SHA-256 and ZIP CRC passed. No nonselected member payload was read. ZIP central-directory metadata was used only for exact selection and duplicate rejection; no global member inventory or class scan was produced.

The advertised REA extractor has no exact-member selector and extracts all regular files. It was not called. This agent's observed session was targetless, independently of Root's reported archive opening, and exposed no Java/JVM bytecode operation. The authorized fallback used ordinary Python standard-library ZIP reads. `capture.json`, `archive-before.json`, `archive-after.json` and `TOOL-SELECTION.json` preserve that distinction. These shell/parser outputs are not REA Evidence and have no invented REA Evidence IDs. Original inventory artifact/occurrence identities are retained as provenance, not as a claim that REA generated the new interpretation.

No JVM, vendor class initializer, browser, native vendor process or Open SEO process was executed. No product source, Git, Cargo, user profile, licence, credential or provider path was accessed. After the initial exact capture, analysis read only the captured E: bytes and generated E: reports. No later C: archive read or new archive member capture occurred.

## Code coverage

The own bounded class parser verifies class magic, member digests, class identity, CP bounds, every parsed attribute boundary and final EOF. It decodes JVM Code instruction boundaries, wide/switch operands and branch targets, then requires every reported call to be an actual `invokeinterface` using the exact preselected CP owner/name/descriptor. Across the twelve authorized classes, locating all uses decoded 314 Code bodies / 22,657 Code bytes. The retained direct-caller subset is 15 methods / 5,465 bytes and contains **19 actual calls**. All preselected CP references have actual Code uses; none was silently dropped. `SELECTED-CODE.json` retains their PC, absolute class-file offset, exact descriptor, instructions and receiver-field assignment constructors. One already captured direct callee adds 514 bytes in `EXACT-IN-SET-DISPATCH-CODE.json`.

This reader is not a full JVM verifier, decompiler or execution engine. It does not prove runtime reachability, implementation dispatch through injected interface values, alias behavior, every helper's semantics, or an unread sink's field write. Nine bounded observations in `CALLSITE-FINDINGS.json` retain the exact relevant instructions rather than only CP names.

## What the calls actually demonstrate

- **CDN test UI output:** `ui/crawl_config/e/id281146822`, Code offset 7131, invokes the concrete classifier constructor at PC64 and its boolean method at PC70 / class offset7201. The branch at75 chooses the literal keys `configuration.cdns.test.internal` at78 or `configuration.cdns.test.external` at86. Both reach `Label.setText(String)` at96. This is classification feedback, with an actual named UI sink.
- **Scope propagation:** both `spider/a/id281146822` methods call the classifier at PC29 (class offsets17205/17330), immediately passing the `LinkScope` result to `data/id.id281146822(LinkScope)` at34. Both `spider/id100783116` methods do likewise at PC23→28 (classifier class offsets9056/9553). `t/id` merely returns LinkScope at PC5→10. `y/id281146822` passes it into `data/id439112727(String,LinkPathType,LinkScope)` at PC156. These prove typed scope argument flow, not the sink implementations' stored fields.
- **Unmapped boolean destination:** `spider/id814175767` PC72→80 and `spider/b/id281146822` PC39→44 pass a classifier boolean to `data/id1020581975.id1908791573(Z)V`. That recipient class is outside the twelve captured members. No human field name or actual persisted write is proved from the obfuscated method name.
- **List filtering / production:** `id1943073927` has four actual classifier calls at PC62/233/331/345 in conditional List filtering. `id360930495` stores its classifier result in local5 at66; local4 and local5 both gate the main block at151/156. That block obtains typed data lists and adds item strings under further guards. Two separate false-arm flags and subsequent filtering remain visible. This is concrete list control flow, not an independently identified HTML parser field.
- **Configured extraction dispatch, with unknown CDN cause:** the large `id1247909229` method invokes a `seoelements/id1221206073` predicate at PC1129, then consults `SpiderCustomExtractionConfig`, obtains `CustomExtractionInfo` at1238, calls an extractor returning List at1249, builds a Map at1282 and passes it to `data/id1020581975.id281146822(Map)` at1311. This is an actual typed configured-extraction dispatch chain. Its preceding predicate, extractor body, semantic result names and final storage are unread. The nearby classifier calls at156/652 do not establish that CDN configuration enabled any particular extraction.

The nine findings also retain the direct UI predicate, the boolean wrapper using LinkScope enum comparisons, and the combined predicate in `id1247909229`. These are scoped observations; an obfuscated getter, setter-like signature or package containing `seoelements` is not assigned a human SEO meaning without independent mapping.

## Additional fields and audits

Additional CDN-dependent HTML field names proved: **none**. Title, description, H1/H2, canonical, robots, hreflang, structured data, image metadata and link attributes remain **UNKNOWN** as additional CDN extractions. No exact CDN-specific audit rule/finding identifier is proved. This negative result is bounded to this captured path and is not a claim that the vendor lacks such behavior.

The classifier's classification result, a constructed link record, a configured extractor result and a durable SEO field are separate facts. The already known ordinary-ASCII classification reference is not promoted into Unicode/port/escaping parity, a network grant or full vendor equivalence.

## Exact remaining static targets, before implementation

Any next archive capture needs a separate exact member selection with newly verified class identities; none below was captured in this pass:

1. `uk/co/screamingfrog/seospider/data/id1020581975.class`: resolve the observed `id1908791573(Z)V` destination and `id281146822(Map)V` destination to actual field writes, getters, serialization/report labels and future consumers. This is the shortest missing bridge from classification/configured results to stored data.
2. `uk/co/screamingfrog/seospider/data/id.class` and `data/id439112727.class`: resolve the exact LinkScope builder/constructor assignments. Scope output must remain distinct from extra HTML values.
3. `seo/spider/seoelements/id1221206073.class`, `seo/spider/extraction/id1657863171.class` and the referenced `CustomExtractionInfo` class: resolve the actual configured-extraction guard, parser operations, value shape and stable names. Preserve every configuration/content guard and show a path where CDN configuration, rather than site matching or another flag, changes the result.
4. Receiver origin remains unresolved in injected-interface paths. A selected factory Methodref on `spider/id814175767` at PC63 has **no matching declaration in that captured class**. Its actual superclass is `spider/id722251721`; inherited resolution is a next question, not a proved factory body. `FACTORY-ATTEMPT-NOT-DECLARED.json` records the failed local-declaration assumption. Do not substitute the Methodref return type for construction evidence.

The first report-building attempt also requested UI PC56, which is not an instruction start; the actual `new` is PC55. The draft and failure are retained, the selector was corrected, and the completed findings pin actual instruction starts. Neither failed helper attempt contributes a positive finding.

No Rust change is proposed from these unknown field names. A later source comparison and implementation task needs the exact proved value/guard/persisted sink and separate own-source evidence. The present deliverable is the authenticated byte availability, actual callsite map, bounded branch/argument observations and explicit next proof obligations.
