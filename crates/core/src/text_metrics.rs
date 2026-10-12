//! Deterministic local text-width profile using an openly licensed font.
//! This is a simple glyph-advance metric, not browser shaping or a prediction
//! of the font, truncation or layout a search engine will use.
use crate::PageRecord;
use std::sync::OnceLock;
use ttf_parser::Face;

const ROBOTO: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Regular.ttf");
pub(crate) const ENGINE: &str = "roboto-repo-tag-v2.138; glyph-quantization-150-percent-v2; title20px; description14px; latin-greek-cyrillic";

fn font() -> &'static Face<'static> {
    static FACE: OnceLock<Face<'static>> = OnceLock::new();
    FACE.get_or_init(|| Face::parse(ROBOTO, 0).expect("validated embedded public Roboto font"))
}

/// Unsupported scripts, combining marks, missing glyphs or overflow remain
/// unknown rather than being assigned an invented fallback-font width.
pub fn text_width(text: &str, size_px: u32) -> Option<u32> {
    let face = font();
    let units = u32::from(face.units_per_em());
    // Fixed documented reference profile: quantize glyph advances at 150%
    // device scale, then convert the complete advance back to logical pixels.
    // This is reproducible across OSs; it does not inspect/change monitor DPI.
    let physical = text.chars().try_fold(0_u32, |total, character| {
        let code = character as u32;
        let simple = (0x20..=0x2ff).contains(&code)
            || (0x370..=0x52f).contains(&code)
            || (0x1e00..=0x1eff).contains(&code)
            || (0x2000..=0x200a).contains(&code)
            || (0x2010..=0x2027).contains(&code)
            || code == 0x20ac;
        if !simple || character.is_control() {
            return None;
        }
        let glyph = face.glyph_index(character)?;
        let advance = u32::from(face.glyph_hor_advance(glyph)?);
        let pixels = (advance
            .checked_mul(size_px)?
            .checked_mul(3)?
            .checked_add(units)?)
            / (units * 2);
        total.checked_add(pixels)
    })?;
    physical.checked_mul(2)?.checked_add(1).map(|n| n / 3)
}

pub(crate) fn measure_metadata(page: &mut PageRecord) {
    page.title_pixel_widths = page
        .titles
        .iter()
        .map(|text| text_width(text, 20))
        .collect();
    page.description_pixel_widths = page
        .meta_descriptions
        .iter()
        .map(|text| text_width(text, 14))
        .collect();
    page.text_metrics_engine = Some(ENGINE.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_and_wide_titles_use_real_font_advances() {
        assert_eq!(text_width("", 20), Some(0));
        assert_eq!(text_width(&"W".repeat(75), 20), Some(1350));
        assert!(text_width(&"W".repeat(20), 20) > text_width(&"i".repeat(20), 20));
        assert!(text_width("Title in the body", 20).is_some());
        assert!(text_width("Über uns – Preise €", 20).is_some());
    }

    #[test]
    fn device_glyph_quantization_precedes_single_logical_sum_rounding() {
        assert_eq!(text_width("\u{037e}", 20), Some(5));
        assert_eq!(text_width(&"\u{037e}".repeat(2), 20), Some(11));
        assert_eq!(text_width(&"\u{037e}".repeat(3), 20), Some(16));
    }

    #[test]
    fn shaping_and_missing_font_coverage_remain_unknown() {
        assert_eq!(text_width("😀", 20), None);
        assert_eq!(text_width("日本語", 20), None);
        assert_eq!(text_width("العربية", 20), None);
        assert_eq!(text_width("e\u{301}", 20), None);
        assert_eq!(text_width("title\n", 20), None);
    }

    #[test]
    fn multiple_elements_and_unknown_widths_preserve_positions() {
        let mut page = PageRecord {
            titles: vec!["Title".into(), "😀".into(), "".into()],
            meta_descriptions: vec!["Description".into()],
            ..Default::default()
        };
        measure_metadata(&mut page);
        assert_eq!(page.title_pixel_widths.len(), 3);
        assert!(page.title_pixel_widths[0].is_some());
        assert_eq!(page.title_pixel_widths[1], None);
        assert_eq!(page.title_pixel_widths[2], Some(0));
        assert_eq!(page.description_pixel_widths.len(), 1);
        assert!(page.text_metrics_engine.unwrap().contains("title20px"));
    }
}
