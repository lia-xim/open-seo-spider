# Portable views: complete public CI on66dfe5c

The exact published commit `66dfe5c34bb464f44c14be4c3b2581bb48d81eea` passed all four jobs in [run39329699110](https://github.com/lia-xim/open-seo-spider/actions/runs/39329699110): Windows, Ubuntu, Renderer and container.

The raw Renderer log contains exactly **45 distinct suite PASS records in one unfiltered coordinator invocation**: nine API-fixture suites and36 native suites, including `table-view-portability`. Their names, kinds and order match the exact [committed registry](registered-ui-e2e.mjs). [receipt.json](receipt.json) binds the raw run/job API responses, completed UI step and log. [index.json](index.json) binds every copied raw byte.

This closes the earlier pending full45-suite/public-CI check for that published portability revision. The original R1 failed16-suite invocation and R2 single-suite proof remain unchanged and separately scoped. These results do not prove later XLSX work, all table/detail/export combinations, arbitrary browser/resource/crash behavior or complete manufacturer parity. The earlier601-file local manifest is an explicit source/harness input set, not a transitive metadata closure; this public CI binds the complete committed revision independently.
