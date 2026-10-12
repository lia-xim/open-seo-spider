# Text measurement font

`Roboto-Regular.ttf` was downloaded independently from the public Google Fonts
Roboto2 repository, tag v2.138:

- [Font source](https://github.com/googlefonts/roboto-2/blob/v2.138/src/hinted/Roboto-Regular.ttf)
- [Apache-2.0 license source](https://github.com/googlefonts/roboto-2/blob/v2.138/LICENSE)
- Font SHA-256: `56a45233d29f11b4dfb86d248e921939d115778f87325e7ae8cc108383d6664d`
- File size: 515100 bytes
- The repository tag is v2.138; the embedded font name-table version is
  `Version 2.001047; 2015`. The tag is not an assertion about that internal version.

The full Apache2.0 license is retained in LICENSE.txt. Google/Roboto names
identify the upstream font. This file was not extracted from the reference
application, and no renamed reference font is distributed by this project.

The Rust measurement profile quantizes glyph advances at a fixed 150% device
scale, sums them, and converts the total back to logical pixels, with 20px for
titles and 14px for descriptions. This independently calibrated reference
profile is deterministic across operating systems; it does not read or change
the user's display settings. See [calibration](../../docs/research/text-width-calibration.md).
This profile provides comparable
deterministic measurements, not a guarantee of a search engine's font, shaping,
fallback, layout or truncation. Unsupported scripts and combining sequences
remain unknown. Cross-platform AWT and complex-script equivalence is not proven.
