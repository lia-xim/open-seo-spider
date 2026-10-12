# Filter and text-width investigation

Observed2026-10-12 against the installed original24.3 JAR, SHA-256
`5b6fb252e194c9702e6ba8f6bf589e9870175e1478bcd6148a3f8068d4e18d76`.
REA retained inventory and inspection evidence
`ev_617a062dd88f552e52f4771b31bb21896196a4c65ff21c04da9970dabea71da2`
and `ev_9d0eb65a2fd733794cce97ccb99d72a6f744f2aab6323417a00c3e5521629c34`
were refreshed against this hash. The inspection completed; its complete response
exceeded the MCP transport budget, so the full retained evidence was exported
locally rather than interpreting a transport error as missing analysis.

The configured REA Java decompiler provider was unavailable. A locally downloaded
[Vineflower1.12.0](https://github.com/Vineflower/vineflower/releases/tag/1.12.0)
was used for focused, read-only class inspection. Its JAR was verified against the
official release digest
`1dfcfe974395734fa467ce620661c7623d05ba83670de0529b1fbd63ff548b9d`.
Decompiler output and original class/font bytes remain outside the project.
No reference class was executed to obtain the observations below.

| Evidence | Derived hypothesis | Verification boundary |
| --- | --- | --- |
| SeoElementFilterKey enum class,153911 bytes; localized-name/English-name/type/stat/bit-position fields | Filters have identifiers, names, categories and bitfield/stat metadata. | The large static initializer could not be reconstructed reliably. Its output is not a complete filter algorithm or source recovery. Original CLI help provides the concrete416 named export-filter inventory instead. |
| seo.spider.j.id1840409942 refers to AWT FontMetrics and platform-specific width paths. Related configuration refers to sfroboto at20px for titles and14px for descriptions. | Width measurement depends on font and platform behavior rather than character count alone. | Static coupling does not prove every live call path, shaping rule, fallback or operating-system result. |
| SpiderFilterWatermarkConfig defaults include title chars30–60, description chars70–155, headings70, alt100, URL115, low word count200 and high outlink thresholds1000/10. | These are configurable issue-filter thresholds for this installed build. | Default constants alone do not prove boundary inclusion, filter membership or saved-profile overrides. Dedicated runtime positive and negative fixtures are required. |
| Actual original CLI CSV on the authored fixture reports75 W characters as1350px. | A whole-pixel glyph metric is a useful candidate for ASCII comparison. | This is one genuine reference observation, not broad text-measurement parity. |

Open SEO Spider's implementation is independently written Rust. Its bundled
Roboto font was downloaded from the public Google Fonts v2.138 source with the
upstream Apache2.0 license; it is not the reference application's renamed font.
See [font provenance](../../assets/fonts/README.md) and
[actual local comparison](../parity/reference-comparison.md). Widths that need
unsupported shaping or missing glyphs remain unknown.

The earlier observations above are retained as historical evidence. The later
[independent text-width calibration](text-width-calibration.md) and its
[per-position measurement artifact](../parity/text-width-calibration.json)
identify the observed Windows scale-1.5 quantization, reproduce the bounded
title/description CSV values with the existing public font, and distinguish that
measurement proof from a new native integration gate. They also clarify that
the public source tag and the selected font's internal version are different
provenance facts.

The runtime comparison also distinguishes initial URL status from final response
metadata, intermediary redirect rows, anchor metrics from resource references,
resource-indexability profiles and localized reason labels. These differences
are explicitly retained as acceptance gaps; extra captured observations are not
silently discarded to improve a comparison score.
