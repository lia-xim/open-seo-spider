# Installed Screaming Frog architecture observations

Scope: the user-selected running Windows application, observed on 2026-10-11.
This is an evidence-led architecture inventory, not a claim of full method-level
reconstruction or complete behavior equivalence.

## Artifact identity

- Main JAR SHA-256: `5b6fb252e194c9702e6ba8f6bf589e9870175e1478bcd6148a3f8068d4e18d76`.
- Main JAR: 13,824,663 bytes; 5,708 archive entries; 5,400 Java class files.
- 395 neighboring JAR dependencies in the installed `lib` directory.
- Main class: `uk.co.screamingfrog.seospider.SeoSpider`.
- JAR manifest identifies Maven JAR Plugin 3.4.2 and Java 25 as build toolchain.
- REA `inspect_artifact` evidence:
  `ev_9d0eb65a2fd733794cce97ccb99d72a6f744f2aab6323417a00c3e5521629c34`.

The complete REA record was retained locally after exceeding the MCP transport
response budget. A 13,945,274-byte evidence bundle was exported locally; the
original binaries and complete raw records are not distributed in this repository.

## Runtime observations

The running Windows launcher starts the bundled `jre/bin/javaw.exe` with the main
JAR, G1 GC, string deduplication and a 2 GB maximum heap. `jcmd VM.version` reports
OpenJDK 25.0.3+9-LTS. A loopback listener was present in the Java process; its purpose
has not been established from that observation alone.

Read-only loaded-class diagnostics contain JavaFX UI classes, Apache HTTP client
classes, Apache Derby storage classes and SQLite JDBC classes. Loaded classes establish that those
components were loaded into this process. They do not establish every call path
or demonstrate that every crawl is stored with that backend.

## Bundled components and candidate roles

| Observed dependency | Candidate responsibility | Evidence boundary |
|---|---|---|
| JavaFX 25.0.3 | Desktop interface | Loaded JavaFX control/skin classes observed |
| Apache HttpClient 4.5.14 | HTTP requests and connection pooling | Loaded HTTP connection/parser classes observed |
| jsoup 1.22.1 | HTML parsing and selectors | Static DOM/parser references and typed CDP snapshot-to-jsoup bridge; complete extraction behavior pending |
| Apache Derby 10.17.1.0 | Embedded crawl persistence | Loaded storage/compiler classes observed |
| SQLite JDBC 3.53.0.0 | Additional local storage | Loaded JDBC classes observed; exact database role pending |
| Chromium/headless 147.0.7727.102 wrappers | JavaScript rendering | Bundled platform archives; official product documentation confirms Chromium rendering |
| LanguageTool 6.5 and dictionaries | Spelling/grammar analysis | Bundled dependencies; live grammar workflow pending |
| Lucene 10.1.0 | Indexing/text analysis | Bundled dependency; active role pending |
| PDFBox 3.0.7 | PDF content analysis | Bundled dependency; PDF workflow pending |
| Saxon, Jena, RDF4J, SHACL | XPath/XML/RDF/structured-data processing | Bundled dependencies; individual call paths pending |
| Spring Boot 4.0.5 and MCP SDK 1.1.1 | API/MCP surfaces | Bundled dependencies; local listener attribution pending |

Package groupings include `seo/spider/ui`, `seo/spider/config`,
`seo/spider/spider`, `seo/spider/seoelements`, `seo/spider/columndata`,
`uk/co/screamingfrog/seospider/data`, and `uk/co/screamingfrog/seospider/db`.
Many class names are obfuscated; names and dependency inventory cannot recover
the complete algorithms or data lifecycle.

The read-only `jdeps --multi-release 25 -summary` dependency graph also confirms
static references from the main JAR to multiple bundled libraries. Static
references strengthen the linkage evidence but do not establish which feature
or execution branch uses a library. The executable public entrypoint signature
was inspected without reproducing its implementation in the replacement.

A subsequent scan decoded all 5,400 class constant pools and inspected focused
Java signatures. It establishes concrete parser/browser, JDBC storage,
SQL duplicate-processing and MCP servlet couplings. The detailed, execution-free
evidence and its limits are in [static responsibilities](static-responsibilities.md).
These compiled references do not attribute the previously observed loopback
listener or establish a complete live feature path.

## Independent replacement decisions

Open SEO Spider uses original Rust code, Tokio scheduling, Reqwest HTTP transport,
HTML selectors, SQLite persistence and a local browser interface. It has no
commercial 500-URL cap. Memory, disk, request size, user-selected crawl scope and
the resources of the crawled server still bound practical operation.

The existing application's license behavior is not modified. No proprietary
implementation, assets, binaries, configuration secrets or website crawl data
are included in the replacement repository.

## Remaining research

Full method-level reconstruction, all >300 audit definitions, exact memory and
database modes, rendering parity, every integration, and representative scale
comparisons remain **NOT PROVEN**. `docs/feature-parity.json` is the acceptance
ledger; packaging and interface similarities must not be mistaken for parity.

Official references: [user guide](https://www.screamingfrog.co.uk/seo-spider/user-guide/)
and [product feature overview](https://www.screamingfrog.co.uk/seo-spider/).
