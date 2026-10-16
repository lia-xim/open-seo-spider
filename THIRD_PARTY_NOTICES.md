# Third-party notices

Original Open SEO Spider code and documentation are distributed under the [MIT license](LICENSE). Dependency and asset licenses remain separate.

## Bundled font

Roboto is used for bounded text-width measurements. The bundled font is covered by the Apache License 2.0; its full license is retained in [assets/fonts/LICENSE.txt](assets/fonts/LICENSE.txt). Preserve that license with redistributed font assets. A calibrated font profile does not imply universal rendering equivalence.

## Optional renderer dependencies

The checked-in [renderer lockfile](renderer/package-lock.json) declares:

| Package | Locked version | Declared license |
| --- | --- | --- |
| axe-core | 4.14.0 | MPL-2.0 |
| playwright-core | 1.64.0 | Apache-2.0 |

These packages are installed separately by `npm ci`. Their own license and notice files apply when redistributing them. Keep applicable source-availability obligations and notices with any redistributed MPL-covered files; the project's MIT license does not replace those obligations.

## Other dependencies and runtimes

Rust and JavaScript dependencies retain the licenses of their exact locked versions. Preserve their required notices when building redistributable packages; this document is not a complete dependency-license audit.

LanguageTool is a separately installed optional service. This repository does not bundle its engine, language packages or a JRE. Review the license and terms of the exact service distribution you choose; no license classification or compatibility claim is made here.

Chrome, Edge or Chromium must also be installed separately and retain their distribution terms. No Screaming Frog binaries, proprietary source, fonts or license mechanisms are included in this project.
