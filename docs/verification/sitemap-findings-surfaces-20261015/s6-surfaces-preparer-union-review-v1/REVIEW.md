# Bounded v3 required-test union closure

Reviewed v3 SHA01955968f1481e2ef93b3cb1845c4b02bb4bdbb54741cd173044fc09b4778e78; node --check exit0. No preparer/deriver/Git loop/freeze/compiler/test/native execution by this reviewer. Root reported its v2 validation safely stopped at requiredCount before any output; current read-only TestPath checks independently found both the source output and v2 preparation receipt absent.

Actual pinned previous275 manifest72dd26c3108c3e3432932e5bfb02d7f75eb8aeed9e7d71b24629511e81289f8d has341unique(path,name) identities. `crates/core/src/sitemap_membership_analysis_tests.rs` appears in TWO groups with8and2distinct names. A read-only aggregation of that metadata confirms v2's last-wins Map retains333names, while union across all groups retains341. No test functions were run or recollected by a compiler. The earlier v2 closure addressed its two specific approval/fresh-receipt paths, not this newly observed manifest aggregation case.

v3line57 creates one Set per path and adds every name from every previous group before adding new-test names. It therefore preserves all10names from the duplicate-path groups and all341previous identities. The subsequent requiredCount guard remains341+new, and minimum remains1123+new. New functions still derive after-minus-actualHEAD-before; no historical tests are relabeled new.

Read-only mechanical inverse comparison: replacing only the v3 receipt filename with v2 and reverting only the old-group union statement reproduces the entire v2 script EXACTLY. The actual derivation source was read and uses those two unique checked anchors against fixed v2 SHA9458d8f5…. Fixed CLIca058, independent closuref4b98, no-caller-pin rule, early receipt absence, previous275/HEAD898/lineage/actual-payload guards and all write ordering remain identical to the prior reviewed v2. Original v2 itself still hashes9458d8f5…. A v3 source copy is preserved for review evidence only.

Result: this specific metadata-loss trigger is CLOSED_IN_V3_SOURCE_NOT_EXECUTED; no additional confirmed fix-delta findings. No full source freeze, final source/test counts, runtime acceptance or compiler PASS is claimed.
