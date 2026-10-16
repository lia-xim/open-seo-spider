# CDN configured-extraction bridge: exact four-class static follow-up

Status: CONCRETE_CLASSIFICATION_TO_CONFIGURED_EXTRACTION_FIELD_BRIDGE_STATIC_ONLY.

The earlier unknown between classification and a real extraction field sink is now partly resolved. For the captured concrete data class, the classifier boolean is written to a field, that same field is read by the custom-extraction guard, and configured results reach an actual `Map<Integer,List<String>>` field. This is static evidence for configured extraction; it does not enumerate fixed additional CDN title, heading or metadata fields.

## Exact capture and bounds

Initial direct member reads captured only:

| Member | Bytes | SHA-256 |
|---|---:|---|
| `seo/spider/seoelements/id1221206073.class` | 7057 | `09533e2fa0c3c0a835f6b7470fb1b8cbf74b066e0c07c47dca7f7f18c3976f60` |
| `seo/spider/extraction/id1657863171.class` | 2885 | `2c8315dc4e991021e4a8bb9f4f67fe72487bdd6dabe463ed233709b2adf2983f` |
| `uk/co/screamingfrog/seospider/data/id1020581975.class` | 41608 | `c499648ef905f89dc89a06bda52dffec27b915ba012ff68b8c0115f5508d898a` |

Five real Methodrefs to `CustomExtractionInfo` in the selected extractor were recorded in `OPTIONAL-INFO-NEED.json` **before** exercising the authorized optional fourth capture. That member is `seo/spider/extraction/CustomExtractionInfo.class`, 6765 bytes, SHA-256 `2f8307bbe62524ef17e7ef987d9e8341baf3e7e392ffbcb7a51bd3864b5b9e90`.

Total: **four members / 58,315 bytes**. Every direct `getinfo/read` passed its 4 MiB per-class bound, CRC, actual class internal name and output containment checks. Initial total remained below 12 MiB and all four below 16 MiB. Each capture phase independently measured the authorized 13,824,663-byte archive before/after with identical SHA-256 `5b6fb252e194c9702e6ba8f6bf589e9870175e1478bcd6148a3f8068d4e18d76` and unchanged file identity. No `namelist`/`infolist` inventory, fifth member, nested enum, superclass or concrete parser was read. ZIP opening inherently parses directory metadata; duplicate-name uniqueness beyond the direct selected lookup is not claimed.

Own bounded Python class parsing decoded 291 Code bodies / 6390 Code bytes across these four exact members to validate and locate the requested methods. The final retained set is 19 selected methods / 346 Code bytes, plus a small local Map conversion window. This is not REA Evidence, a JVM verifier, decompiler execution or runtime observation. No class initializer was executed; its captured instructions are static bytes only.

## Classification flag now has an actual field bridge

`data/id1020581975.id1908791573(Z)V`, Code offset26768, writes local boolean field `id814175767:Z` at PC2. The captured previous twelve-class callers already passed their classifier result to this method.

`data/id1020581975.id1981399840()Z`, Code offset31190, returns that **same field** via `getfield` at PC1. The actual class extends `data/id1883924803`, the receiver type used by the captured guard.

The guard `seoelements/id1221206073.id1909831410(L...data/id1883924803;)Z`, Code offset6310, requires all three conditions:

1. Virtual `data/id1883924803.id1981399840()Z` at PC1 is true. For this captured concrete subtype, its local override returns the classifier-written field.
2. External `data/id315938658.id281146822(L...data/id782393105;)I` at PC8 equals integer2.
3. External `spider/id601314627.id281146822(L...data/id782393105;)Z` at PC16 is true.

The last two helper bodies are outside scope, so their HTML/MIME/status or other meanings remain UNKNOWN. Other runtime receiver subclasses and all-caller dispatch are not established. The prior ordinary-ASCII CDN-inclusive classifier reference is reused; no vendor matching or live crawl was executed here.

## Actual named configuration fields, not guessed getter meanings

The optional fourth class resolves the five extractor-consumed getters directly to actual un-obfuscated fields:

| Getter | Code offset | Actual field |
|---|---:|---|
| `id()ExtractionType` | 4934 | `mExtractionType` |
| `id1909831410()DataType` | 5042 | `mDataType` |
| `id814175767()String` | 5142 | `mExpression` |
| `id1840409942()String` | 5250 | `mAttribute` |
| `id722251721()Z` | 5358 | `mIsValid:Boolean`, then `booleanValue()` |

`mName` and `mInvalidReason` also exist as named declarations and in a field-bound `toString` recipe. They are not thereby proved to be result-map keys; the existing caller uses integer configuration indices.

The extractor's 28-byte constructor stores its two String parameters in separate local fields. Its 99-byte method obtains `mExtractionType`, selects/caches an implementation through an EnumMap/enum factory, checks `mIsValid`, and passes `mExpression`, `mDataType`, `mAttribute` and the caller's integer to the unread `extraction/id903546664` interface. A valid empty result triggers an attempt to append an empty string. Invalid configuration takes the fallback text bound to localization key `configuration.custom.extraction.invalid_config`. Concrete implementation behavior, enum values and the integer's meaning remain UNKNOWN; no XPath/CSS/regex claim follows just from these types.

## Actual Map field sink and readback

`data/id1020581975.id281146822(Map)V`, Code offset29290, writes `id1943073927:Map` at PC15. The field's actual generic Signature is `Map<Integer,List<String>>`.

The setter assigns null, assigns a nonempty map, and leaves the previous field unchanged for an empty map. Local `id1893547943()Map`, Code offset36503, returns `Collections.emptyMap()` for null or returns the field otherwise. This proves the in-memory sink and readback, not durable disk storage.

A local storage-conversion method reads the same field at PC182 and passes it to `storage/id418568783.id281146822(Map,Function,Function,Supplier)Map` at PC199. The interface implementation is outside scope; disk persistence, serialization format and report column labels remain UNKNOWN. Likewise, the extractor's two model String getters and their local decoding branches are retained without assigning raw/rendered/base-URL roles from obfuscated names.

## Result and stopping boundary

The supported static chain is now: classifier result → concrete boolean field → guarded configured extraction → actual named configuration values → indexed string-list Map → concrete model field. Its content predicates and concrete parsers remain unread. This is a useful bridge beyond merely forwarding LinkScope.

Hardcoded additional CDN HTML field names and CDN-specific audit rules proved: **none**. The evidence does not establish that custom extraction is the entirety of the vendor's additional-CDN behavior, nor full vendor parity. No fifth class or inherited factory resolution was attempted. No Open SEO source/product, Root data, Git, Cargo, browser, native process, Java, REA runtime, credential, licence or provider path was accessed or changed.
