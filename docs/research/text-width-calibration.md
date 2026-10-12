# Text-width calibration against a bounded reference fixture

Observed on 2026-10-12. Full vendor parity: **NOT_PROVEN**.

The remaining ordinary title-width differences in the retained local reference
fixture are reproducible through Java's device-scale quantization. An
independently written Canvas probe using the project's existing public font,
on this Windows desktop at scale 1.5, reproduces every available title and
description width in the two metadata CSV exports. A public-font integer
advance formula reproduces the same values. An asset replacement is unnecessary
for this observed scope.

The complete per-position observations, font identities, historical native
expectations, independent Java measurements and candidate values are saved in
[text-width-calibration.json](../parity/text-width-calibration.json). This is a
measurement artifact, not evidence that a new Rust profile is integrated.

A subsequent native gate now records integration separately in
[native-text-width-comparison.json](../parity/native-text-width-comparison.json).
The actual Rust offline API matches all 58 present metadata measurements from
the hash-verified retained CSVs; two absent-element display placeholders are
excluded. `scripts/reference-width-profile.mjs` reproduces this gate without
starting the original application or fetching the listed URLs. The ordinary
crawler comparison also matches its present HTML-200 title widths. Initial
redirect/error-response metadata and unknown values remain separate observations.

## Reference and experiment scope

The original reference is Screaming Frog SEO Spider 24.3's ordinary CLI run of
the independently authored local fixture at `2026-10-12T07:08:34.587Z`. This
research reuses its retained `Page Titles:All` and `Meta Description:All` CSVs;
it does not start another vendor crawl or modify vendor settings. Fixture
SHA-256:

`928545b1b70ae07da917fddd0fd1fd137878e7819326741dbbd6c23cb4607072`

Each CSV has 29 rows for HTTP-200 HTML observations. One page has a second title
and another page has a second description. We measure each first slot and the
one actually present second slot, rather than counting all empty second columns.
Each field also has one first-slot CSV zero for an absent element. Its display
value is retained, but it is excluded from present-element match counts.

| Scope | Title | Description |
|---|---:|---:|
| First CSV slots | 29 | 29 |
| Additional present second slots | 1 | 1 |
| Present elements | 29 | 29 |
| Nonempty present elements | 27 | 28 |
| Absent-element display placeholders | 1 | 1 |
| Candidate and public Canvas match present reference elements | 29/29 | 29/29 |
| All retained CSV slot values match, including the separately labelled placeholder | 30/30 | 30/30 |
| Historical native scale-1 prediction matches present elements | 4/29 | 1/29 |

These are measured text fields, not features, filter memberships or a coverage
percentage. Empty declared elements measure zero; absent metadata remains an
absent element, and unsupported text remains unknown.

The independently authored Java probe uses only public AWT APIs. It creates a
font from an explicit path, registers it in the process, and measures both a
family-selected `Font` and the directly created font with `Canvas.getFontMetrics`.
No application classes are loaded. It applies the system look and feel without
opening a window. Java is Temurin `25.0.3+9-LTS`; the observed Windows screen is
144 DPI, with default and font-render-context transforms both 1.5. The returned
context has fractional metrics and antialiasing both false. Registering a font
affects this probe process only. No operating-system DPI, installed fonts or
application preferences were changed.

Java documents that text measurements depend on the device transform and pixel
rounding, and that device-independent image buffers normally have an identity
transform. These contracts motivate a device-context control rather than a
font-version guess. [FontRenderContext](https://docs.oracle.com/en/java/javase/25/docs/api/java.desktop/java/awt/font/FontRenderContext.html),
[GraphicsConfiguration](https://docs.oracle.com/en/java/javase/25/docs/api/java.desktop/java/awt/GraphicsConfiguration.html).

## Font provenance and the version-label trap

The private reference font identifies itself as `Version 2.137; 2017`, with
1,294 glyphs and 2,048 units per em. Its whole-file SHA-256 is
`d5a50f372e797f726c4b6b7fbfba63d836b84a57acee47688d94f30e3e9b59bd`.
Only metadata and bounded measurements are recorded here; its bytes are not
distributed by this project.

The public file already in `assets/fonts/Roboto-Regular.ttf` was acquired from
the upstream repository's **v2.138 tag**, but the selected `src/hinted` file's
internal name table says **`Version 2.001047; 2015`**. These are different facts;
a tag-based engine label must not be read as proof of that file's internal
version. [Existing public font source](https://github.com/googlefonts/roboto-2/blob/v2.138/src/hinted/Roboto-Regular.ttf).

| Public font | Internal version | Bytes | SHA-256 |
|---|---|---:|---|
| Existing upstream `src/hinted` file at tag v2.138 | 2.001047; 2015 | 515100 | `56a45233d29f11b4dfb86d248e921939d115778f87325e7ae8cc108383d6664d` |
| Google Fonts commit `724bf98e`, 2017-03-31 | 2.137; 2017 | 171676 | `79e851404657dac2106b3d22ad256d47824a9a5765458edb72c9102a45816d95` |
| v2.138 release archive's unhinted regular font | 2.138 | 349400 | `f3edb8058e523f5612bfd99d0745e661568ad85e1b6217bc62f786fabae624c6` |

The public Google Fonts commit explicitly adds Roboto v2.137 and includes an
Apache-2.0 license. It is a plausible public upstream candidate for the
reference's declared version, **not a proven identical original binary**.
Its glyph count and revision match the private font; some table hashes match,
but the whole file, name table and several other tables differ. Both public
fonts are independently downloadable. [Official commit](https://github.com/google/fonts/commit/724bf98e9f5cb98a1d3d5044f45a2e286b817401),
[Public 2.137 TTF](https://raw.githubusercontent.com/google/fonts/724bf98e9f5cb98a1d3d5044f45a2e286b817401/apache/roboto/Roboto-Regular.ttf),
[Upstream license](https://raw.githubusercontent.com/google/fonts/724bf98e9f5cb98a1d3d5044f45a2e286b817401/apache/roboto/LICENSE.txt).

The actual v2.138 release is dated 2017-08-02 and distinguishes its Android and
unhinted archives. `roboto-unhinted.zip` has SHA-256
`70f64c718510a601fbcf752aafe644314dacaeb85474dc689c89787c4a72a728`.
The separately downloaded public candidates stay in private experiment files;
the repository font is unchanged. [Official release](https://github.com/googlefonts/roboto-2/releases/tag/v2.138).

The same Canvas experiment produces the following results. Family lookup and
directly created font lookup agree in every tested public-font row.

| Public font at scale 1.5 | Title CSV slots | Description CSV slots |
|---|---:|---:|
| Existing `src/hinted` file | 30/30 | 30/30 |
| Public Google Fonts 2.137 | 30/30 | 30/30 |
| Actual v2.138 unhinted release | 5/30 | 8/30 |

The unhinted release's ordinary horizontal advances also predict the observed
ASCII values through the simple formula below, but its actual AWT measurements
do not. This counterexample shows that a simple advance formula is not a
universal implementation of AWT font hinting. Version names alone do not justify
changing font assets. The existing public font supports the bounded calibration
without distributing a vendor resource.

## Independent integer profile

Let `a` be a public-font glyph's horizontal advance, `P` the nominal size, and
`U` the font's units per em. For a **fixed, explicit reference scale of 3/2**:

```text
physical_glyph_advance = floor((a * P * 3 + U) / (2 * U))
D = sum(physical_glyph_advance for each supported glyph)
logical_width = floor((2 * D + 1) / 3)
```

Use `P=20` for titles and `P=14` for descriptions, with the existing public
font's `U=2048`. Quantize each glyph in physical units, add the integer results,
then convert the complete sum once. All arithmetic should use checked unsigned
integer multiplication and addition; an overflow or unsupported glyph should
remain unknown.

This is independently derived positive nearest-integer rounding. At the physical
step, exact half ties round upward. It agrees with the positive tie rule
specified for Java `Math.round`; it is not evidence that the reference itself
calls that particular method internally. At scale 3/2, the final conversion
produces thirds, so it has no exact half tie. Avoid language-default banker
rounding and avoid rounding each logical character before adding it.
[Java Math rounding contract](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/lang/Math.html#round(double)).

A deliberate tie control uses the public font's Greek question mark U+037E:
advance 512 at size 20 becomes exactly 7.5 physical units. Rounding each to 8,
then converting a sum of one, two or three glyphs, gives logical widths **5, 11,
16**. The independent public-font Canvas probe returns those exact values.

At size 20, `fi` measures 11 through whole-string Canvas measurement, while
adding separately rounded logical `charWidth` values gives 12. The longer
`ffffiiii` control gives 45 through the formula and Canvas, versus 48 from that
character sum. Java's API also warns that string advances can differ from
isolated character advances because of shaping. A bare `charWidth` sum is an
inadequate general string contract. [FontMetrics](https://docs.oracle.com/en/java/javase/25/docs/api/java.desktop/java/awt/FontMetrics.html).

The JSON contains 32 supported control measurements: 16 independently authored
strings at each size, including ASCII punctuation, `AV`, `To`, `fi`, `ffi`,
repeated narrow/wide glyphs, NBSP, precomposed accented Latin, Greek and Cyrillic.
All 32 agree between this formula and the independent public Java probe. They
have **no original application reference measurement** and must not be promoted
to vendor-parity evidence. Eight further controls cover combining text, emoji,
Arabic and Japanese at both sizes; the existing native profile deliberately
returns unknown for them even when AWT reports some fallback or composed width.

## Device-context control and examples

The same existing public font in a headless Canvas process has transform 1.0.
For the 29 retained first-title slots it matches only 5, with mean absolute
error 3.517 and maximum error 8 logical units. A BufferedImage at identity
transform with fractional metrics disabled reproduces the historical native
per-glyph scale-1 results. Merely enabling fractional metrics matches 13 of the
29 first slots, but incorrectly changes the 75-`W` value to 1331. Neither a
global fractional sum nor a global font swap resolves the observed contract.

| Reference case | Original width | Historical scale-1 prediction | Fixed scale-1.5 formula and public Canvas |
|---|---:|---:|---:|
| `/`, title 1 | 491 | 495 | 491 |
| `/baseline`, title 1 | 565 | 570 | 565 |
| `/multiple-title`, title 1 | 601 | 609 | 601 |
| `/multiple-title`, title 2 | 199 | 200 | 199 |
| `/long-title`, 75 `W` characters | 1350 | 1350 | 1350 |

The JSON supplies the complete description results and per-position values.
An additional headless BufferedImage experiment keeps the public font and
antialiasing/fractional-metrics settings fixed and changes **only**
`Graphics2D.scale` between 1 and 1.5. At scale 1 it reproduces the historical
scale-1 predictions; at scale 1.5 it matches all 30 title and all 30 description
CSV slots. This isolates the device-transform effect without a desktop change.
The observed default screen scale, not a fitted free parameter, explains the
calibrated factor 1.5.

## Integration acceptance and remaining limits

A deterministic optional or explicitly named metric profile can use this fixed
reference scale without reading or changing the host's current display settings.
Persist the public font hash, profile version, nominal sizes, rational scale,
rounding rule and supported-script policy with results. Retain a separate
scale-1 profile if existing callers need its semantics. Profile selection and
provenance should be visible in exports and saved projects.

Acceptance before claiming integration:

1. Re-run all 58 present reference elements from the JSON through the actual Rust
   profile, preserving both second positions and the absent-element distinction.
2. Verify the independent tie and accumulation controls, checked overflow,
   unsupported-script `None`, and declared empty-element zero in native tests.
3. Re-run the reference comparison against a newly built executable; mathematical
   predictions do not substitute for an actual native run.
4. Probe fresh original-reference strings near configured pixel thresholds and
   ordinary non-ASCII strings, including normalization, whitespace and repeated
   metadata. Bound this claim to the tested profile and cases.
5. Treat the reference's CJK/Thai/Hiragana/Katakana HTML measurement path,
   fallback fonts, combining text, bidi/shaping and cross-platform/JDK/device-scale
   behavior as separate unproven contracts.

The calibration measures logical advances; it does not prove glyph ink bounds,
browser layout, a current Google results-page font, or real search-result title
and description truncation. Full software parity remains **NOT_PROVEN**.
