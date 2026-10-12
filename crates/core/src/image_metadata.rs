//! Native resource dimensions from encoded headers, without pixel decoding.
use crate::{ImageResourceMetadata, ResourceAnalysisStatus};
use anyhow::{Context, bail};
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

const MAX_SVG_BYTES: usize = 2 * 1024 * 1024;
const MAX_SVG_EVENTS: usize = 100_000;
const MAX_SVG_DEPTH: usize = 128;

pub(crate) fn analyze(
    body: &[u8],
    content_type: &str,
    body_complete: bool,
) -> Option<ImageResourceMetadata> {
    let media_type = content_type.split(';').next().unwrap_or("").trim();
    let declared_image = media_type.starts_with("image/");
    let prefix = String::from_utf8_lossy(&body[..body.len().min(4096)]);
    let svg = media_type == "image/svg+xml"
        || prefix
            .trim_start_matches('\u{feff}')
            .trim_start()
            .starts_with("<svg")
        || (prefix.trim_start().starts_with("<?xml")
            && (prefix.contains("<svg") || prefix.contains(":svg")));
    let kind = imagesize::image_type(body).ok();
    if !declared_image && !svg && kind.is_none() {
        return None;
    }
    let mut result = ImageResourceMetadata {
        engine: if svg {
            "quick-xml-0.42/bounded-svg-lengths-v1".into()
        } else {
            "imagesize-0.15.0/encoded-headers".into()
        },
        encoded_bytes: body.len(),
        body_complete,
        format: if svg {
            Some("svg".into())
        } else {
            kind.as_ref()
                .map(|value| format!("{value:?}").to_ascii_lowercase())
        },
        ..Default::default()
    };
    if !body_complete {
        result.status = ResourceAnalysisStatus::Partial;
        result.parse_error =
            Some("Image body capture incomplete; intrinsic dimensions were not established".into());
        return Some(result);
    }
    if svg {
        match svg_dimensions(body) {
            Ok((width, height, view_box)) => {
                result.status = ResourceAnalysisStatus::Complete;
                result.width = width;
                result.height = height;
                result.dimension_unit = Some("css_px".into());
                result.svg_view_box = view_box;
                if width.is_none() || height.is_none() {
                    result.warnings.push("SVG intrinsic dimensions require explicit absolute root width and height; percentages/default viewport/viewBox do not establish them".into());
                }
            }
            Err(error) => {
                result.parse_error = Some(format!("{error:#}"));
            }
        }
    } else {
        match imagesize::blob_size(body) {
            Ok(size) if size.width > 0 && size.height > 0 => {
                result.status = ResourceAnalysisStatus::Complete;
                result.width = Some(size.width as f64);
                result.height = Some(size.height as f64);
                result.dimension_unit = Some("pixel".into());
            }
            Ok(_) => {
                result.parse_error = Some("Image dimensions are zero or invalid".into());
            }
            Err(error) => {
                result.parse_error =
                    Some(format!("Image encoded-header inspection failed: {error}"));
            }
        }
    }
    let mismatch = match result.format.as_deref() {
        Some("png") => media_type != "image/png",
        Some("jpeg") => !matches!(media_type, "image/jpeg" | "image/jpg"),
        Some("gif") => media_type != "image/gif",
        Some("webp") => media_type != "image/webp",
        Some("svg") => media_type != "image/svg+xml",
        _ => false,
    };
    if mismatch {
        result.warnings.push(format!(
            "Detected image format {} differs from declared Content-Type {media_type}",
            result.format.as_deref().unwrap_or("unknown")
        ));
    }
    Some(result)
}

type SvgDimensions = (Option<f64>, Option<f64>, Option<[f64; 4]>);

fn svg_dimensions(body: &[u8]) -> anyhow::Result<SvgDimensions> {
    if body.len() > MAX_SVG_BYTES {
        bail!("SVG byte limit ({MAX_SVG_BYTES}) exceeded");
    }
    let mut reader = Reader::from_reader(body);
    reader.config_mut().check_end_names = true;
    reader.config_mut().check_comments = true;
    let mut depth = 0;
    let mut root_seen = false;
    let mut root_closed = false;
    let mut sizing_stylesheet = false;
    let mut dimensions = (None, None, None);
    for _ in 0..MAX_SVG_EVENTS {
        match reader.read_event().context("SVG XML parsing")? {
            Event::Start(node) => {
                if node.local_name().as_ref() == "style" {
                    sizing_stylesheet = true;
                }
                validate_attributes(&node)?;
                if depth == 0 {
                    if root_seen || root_closed {
                        bail!("SVG contains multiple root elements");
                    }
                    dimensions = root_dimensions(&node)?;
                    root_seen = true;
                }
                depth += 1;
                if depth > MAX_SVG_DEPTH {
                    bail!("SVG nesting depth limit exceeded");
                }
            }
            Event::Empty(node) => {
                if node.local_name().as_ref() == "style" {
                    sizing_stylesheet = true;
                }
                validate_attributes(&node)?;
                if depth == 0 {
                    if root_seen || root_closed {
                        bail!("SVG contains multiple root elements");
                    }
                    dimensions = root_dimensions(&node)?;
                    root_seen = true;
                    root_closed = true;
                }
            }
            Event::End(_) => {
                if depth == 0 {
                    bail!("SVG has an unmatched closing element");
                }
                depth -= 1;
                if depth == 0 {
                    root_closed = true;
                }
            }
            Event::DocType(_) => {
                bail!("SVG DTD declarations are unsupported; external entities are never loaded")
            }
            Event::PI(instruction) if instruction.target() == "xml-stylesheet" => {
                sizing_stylesheet = true
            }
            Event::Text(text) if depth == 0 && !text.trim().is_empty() => {
                bail!("Unexpected text outside SVG root");
            }
            Event::CData(_) | Event::GeneralRef(_) if depth == 0 => {
                bail!("Unexpected data outside SVG root")
            }
            Event::GeneralRef(reference) if reference.resolve_char_ref()?.is_none() => {
                quick_xml::escape::unescape(&format!("&{};", &*reference))?;
            }
            Event::Eof => {
                if !root_seen || !root_closed || depth != 0 {
                    bail!("SVG root is missing or incomplete");
                }
                if sizing_stylesheet {
                    dimensions.0 = None;
                    dimensions.1 = None;
                }
                return Ok(dimensions);
            }
            _ => (),
        }
    }
    bail!("SVG event limit exceeded")
}

fn validate_attributes(node: &BytesStart<'_>) -> anyhow::Result<()> {
    for attribute in node.attributes() {
        attribute?.normalized_value(quick_xml::XmlVersion::Implicit1_0)?;
    }
    Ok(())
}

fn root_dimensions(node: &BytesStart<'_>) -> anyhow::Result<SvgDimensions> {
    let name = node.name();
    let name = name.as_ref();
    let (prefix, local_name) = name
        .split_once(':')
        .map_or((None, name), |(prefix, name)| (Some(prefix), name));
    if local_name != "svg" {
        bail!("Image XML root is not SVG");
    }
    let mut width = None;
    let mut height = None;
    let mut view_box = None;
    let mut namespace = None;
    let namespace_key =
        prefix.map_or_else(|| "xmlns".to_owned(), |prefix| format!("xmlns:{prefix}"));
    let mut style = None;
    for attr in node.attributes() {
        let attr = attr?;
        let key = attr.key.as_ref();
        let value = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)?;
        match key {
            "width" => width = absolute_length(&value)?,
            "height" => height = absolute_length(&value)?,
            "viewBox" => {
                let values: Vec<f64> = value
                    .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
                    .filter(|value| !value.is_empty())
                    .map(str::parse)
                    .collect::<Result<_, _>>()?;
                if values.len() != 4
                    || values.iter().any(|value| !value.is_finite())
                    || values[2] <= 0.0
                    || values[3] <= 0.0
                {
                    bail!("Invalid SVG viewBox");
                }
                view_box = Some([values[0], values[1], values[2], values[3]]);
            }
            "style" => style = Some(value.into_owned()),
            key if key == namespace_key => namespace = Some(value.into_owned()),
            _ => (),
        }
    }
    if namespace
        .as_deref()
        .is_some_and(|namespace| namespace != "http://www.w3.org/2000/svg")
        || (prefix.is_some() && namespace.is_none())
    {
        bail!("Invalid or unbound SVG root namespace");
    }
    // CSS can override the root attributes. Avoid substituting attribute lengths
    // for computed dimensions when an inline sizing declaration is present.
    if let Some(style) = style {
        for declaration in style.split(';') {
            if let Some((name, _)) = declaration.split_once(':') {
                match name.trim().to_ascii_lowercase().as_str() {
                    "width" => width = None,
                    "height" => height = None,
                    _ => (),
                }
            }
        }
    }
    Ok((width, height, view_box))
}

fn absolute_length(value: &str) -> anyhow::Result<Option<f64>> {
    let value = value.trim();
    if value.is_empty() {
        bail!("Empty SVG length");
    }
    if value.ends_with('%') || value.eq_ignore_ascii_case("auto") {
        return Ok(None);
    }
    let units = [
        ("px", 1.0),
        ("in", 96.0),
        ("cm", 96.0 / 2.54),
        ("mm", 96.0 / 25.4),
        ("q", 96.0 / 101.6),
        ("pt", 96.0 / 72.0),
        ("pc", 16.0),
    ];
    let (number, multiplier) = units
        .iter()
        .find_map(|(unit, multiplier)| {
            value
                .strip_suffix(unit)
                .map(|number| (number.trim(), *multiplier))
        })
        .unwrap_or((value, 1.0));
    match number.parse::<f64>() {
        Ok(number) => {
            let pixels = number * multiplier;
            if !pixels.is_finite() || pixels <= 0.0 || pixels > 1_000_000_000.0 {
                bail!("SVG absolute length is nonpositive, nonfinite or exceeds dimension limit");
            }
            Ok(Some(pixels))
        }
        Err(_)
            if ["em", "ex", "rem", "vw", "vh", "vmin", "vmax", "ch"]
                .iter()
                .any(|unit| value.ends_with(unit))
                || value.starts_with("calc(") =>
        {
            Ok(None)
        }
        Err(error) => Err(error).context("Invalid SVG absolute length"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_absolute_units_and_namespace_preserve_real_viewbox() {
        let svg = br#"<s:svg xmlns:s="http://www.w3.org/2000/svg" width="1in" height="2.54cm" viewBox="-1,2,100,200"><s:path d=""/></s:svg>"#;
        let result = analyze(svg, "image/svg+xml", true).unwrap();
        assert_eq!(result.status, ResourceAnalysisStatus::Complete);
        assert_eq!(result.width, Some(96.0));
        assert_eq!(result.height, Some(96.0));
        assert_eq!(result.svg_view_box, Some([-1.0, 2.0, 100.0, 200.0]));
    }

    #[test]
    fn svg_unknown_viewport_inline_css_and_partial_capture_never_invent_dimensions() {
        for svg in [
            r#"<svg width="100%" viewBox="0 0 10 20"/>"#,
            r#"<svg viewBox="0 0 10 20"/>"#,
            r#"<svg width="100" height="200" style="width:50%;height:auto"/>"#,
            r#"<svg width="100" height="200"><style>svg{width:50%}</style></svg>"#,
            r#"<?xml-stylesheet href="remote.css"?><svg width="100" height="200"/>"#,
        ] {
            let result = analyze(svg.as_bytes(), "image/svg+xml", true).unwrap();
            assert_eq!(result.width, None);
            assert_eq!(result.height, None);
            assert!(!result.warnings.is_empty());
        }
        let result = analyze(b"<svg width='100' height='200'/>", "image/svg+xml", false).unwrap();
        assert_eq!(result.status, ResourceAnalysisStatus::Partial);
        assert!(result.width.is_none() && result.height.is_none());
    }

    #[test]
    fn svg_invalid_xml_dtd_dimensions_and_bounds_are_errors() {
        for svg in [
            "<svg><g></svg>",
            "<!DOCTYPE svg SYSTEM 'https://example.org/external.dtd'><svg/>",
            "<svg width='-10'/>",
            "<svg/><svg/>",
            "<not-svg/>",
            "<svg width='1' width='2'/>",
            "<svg><g data-name='&unbound;'/></svg>",
            "<svg>&unbound;</svg>",
        ] {
            let result = analyze(svg.as_bytes(), "image/svg+xml", true).unwrap();
            assert_eq!(result.status, ResourceAnalysisStatus::Error, "{svg}");
            assert!(result.parse_error.is_some());
            assert!(result.width.is_none());
        }
        let deeply_nested = format!("<svg>{}{}</svg>", "<g>".repeat(129), "</g>".repeat(129));
        assert!(svg_dimensions(deeply_nested.as_bytes()).is_err());
        assert!(svg_dimensions(&vec![b' '; MAX_SVG_BYTES + 1]).is_err());
    }

    #[test]
    fn jpeg_gif_and_claimed_broken_images_use_encoded_header_evidence() {
        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&[7, 0, 9, 0, 0, 0, 0]);
        let result = analyze(&gif, "image/png", true).unwrap();
        assert_eq!(result.width, Some(7.0));
        assert_eq!(result.height, Some(9.0));
        assert_eq!(result.format.as_deref(), Some("gif"));
        assert!(!result.warnings.is_empty());
        let result = analyze(b"this is not an image", "image/png", true).unwrap();
        assert_eq!(result.status, ResourceAnalysisStatus::Error);
        assert!(result.width.is_none());
        assert!(analyze(b"plain text response", "text/plain", true).is_none());
    }
}
