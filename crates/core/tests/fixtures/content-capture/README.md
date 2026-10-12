# Independently authored HTML selection fixtures

`selection.json` contains original tiny test documents and exact expected text for Open SEO Spider's declared capture profile. It contains no copied vendor source or assets. The module's `authored_selection_fixtures` test consumes every case. Other module tests cover missing/truncated/decoded sources, raw versus rendered identity, root HTML language, template fragment traversal, limits, hashes and nested DOM scale.

These contracts describe this engine's output. They do not establish Screaming Frog or browser computed-visibility parity. The `explicit-script-style-inclusion` case intentionally removes all exclusion selectors; default capture excludes both tags.
