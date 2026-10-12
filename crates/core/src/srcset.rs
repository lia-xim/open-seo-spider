//! HTML Standard srcset tokenization over attribute values. This records candidates,
//! never selects a browser resource or treats declarations as intrinsic dimensions.
use crate::ImageCandidate;

fn whitespace(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{000c}' | '\r' | ' ')
}

/// Retains rejected descriptor candidates with visible errors, unlike the browser's
/// source-set algorithm which discards them. URLs are not split on embedded commas.
pub(crate) fn parse_srcset(input: &str) -> Vec<ImageCandidate> {
    parse_srcset_bounded(input, 10_000)
}

pub(crate) fn parse_srcset_bounded(input: &str, limit: usize) -> Vec<ImageCandidate> {
    let chars: Vec<char> = input.chars().collect();
    let mut at = 0;
    let mut output = Vec::new();
    while at < chars.len() && output.len() < limit {
        while at < chars.len() && (whitespace(chars[at]) || chars[at] == ',') {
            at += 1;
        }
        if at == chars.len() {
            break;
        }
        let start = at;
        while at < chars.len() && !whitespace(chars[at]) {
            at += 1;
        }
        let mut raw_url: String = chars[start..at].iter().collect();
        let mut descriptors = Vec::new();
        if raw_url.ends_with(',') {
            raw_url.truncate(raw_url.trim_end_matches(',').len());
        } else {
            let mut current = String::new();
            let mut in_parens = false;
            while at < chars.len() {
                let c = chars[at];
                at += 1;
                if c == ',' && !in_parens {
                    if !current.is_empty() {
                        descriptors.push(std::mem::take(&mut current));
                    }
                    break;
                }
                if whitespace(c) && !in_parens {
                    if !current.is_empty() {
                        descriptors.push(std::mem::take(&mut current));
                    }
                } else {
                    if c == '(' {
                        in_parens = true;
                    } else if c == ')' {
                        in_parens = false;
                    }
                    current.push(c);
                }
            }
            if !current.is_empty() {
                descriptors.push(current);
            }
        }
        let mut candidate = ImageCandidate {
            raw_url,
            descriptors,
            eligible: true,
            ..Default::default()
        };
        for descriptor in &candidate.descriptors {
            let invalid = if let Some(value) = descriptor.strip_suffix('w') {
                match integer(value) {
                    Some(width) if candidate.width.is_none() && candidate.density.is_none() => {
                        candidate.width = Some(width);
                        false
                    }
                    _ => true,
                }
            } else if let Some(value) = descriptor.strip_suffix('h') {
                match integer(value) {
                    Some(height) if candidate.height.is_none() && candidate.density.is_none() => {
                        candidate.height = Some(height);
                        false
                    }
                    _ => true,
                }
            } else if let Some(value) = descriptor.strip_suffix('x') {
                match density(value) {
                    Some(density)
                        if candidate.width.is_none()
                            && candidate.height.is_none()
                            && candidate.density.is_none() =>
                    {
                        candidate.density = Some(density);
                        false
                    }
                    _ => true,
                }
            } else {
                true
            };
            if invalid {
                candidate.error = Some(format!(
                    "Invalid or conflicting srcset descriptor {descriptor:?}"
                ));
                candidate.eligible = false;
            }
        }
        if candidate.height.is_some() && candidate.width.is_none() {
            candidate.error = Some("Future-compatible h descriptor requires w descriptor".into());
            candidate.eligible = false;
        }
        output.push(candidate);
    }
    output
}

fn integer(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok().filter(|number| *number > 0)
}

fn density(value: &str) -> Option<f64> {
    // HTML valid floating-point number: optional minus, fractional part and exponent;
    // leading plus, trailing decimal point, NaN/Infinity and whitespace are invalid.
    let bytes = value.as_bytes();
    let mut at = usize::from(bytes.first() == Some(&b'-'));
    let integer_start = at;
    while bytes.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    let integer_digits = at - integer_start;
    let mut fraction_digits = 0;
    if bytes.get(at) == Some(&b'.') {
        at += 1;
        let start = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        fraction_digits = at - start;
        if fraction_digits == 0 {
            return None;
        }
    }
    if integer_digits + fraction_digits == 0 {
        return None;
    }
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        at += 1;
        if matches!(bytes.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        let start = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if start == at {
            return None;
        }
    }
    if at != bytes.len() {
        return None;
    }
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite() && *number >= 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn data_commas_unicode_and_descriptors_do_not_split_urls() {
        let rows = parse_srcset("data:image/png;base64,AAA 1x, bild-ä.png 2x, wide.png 640w");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].raw_url, "data:image/png;base64,AAA");
        assert_eq!(rows[1].density, Some(2.0));
        assert_eq!(rows[2].width, Some(640));
        assert!(rows.iter().all(|row| row.eligible));
    }
    #[test]
    fn rejected_candidates_remain_evidence_and_zero_density_matches_parser_profile() {
        let rows = parse_srcset(
            "a 0w, b 1w 2x, c -1x, d NaNx, e +1x, f 1.x, g 10h, h 10w 20h, i 0x, j 1e2x, k calc(1,2)x",
        );
        assert_eq!(rows.len(), 11);
        assert!(
            rows[..7]
                .iter()
                .all(|row| !row.eligible && row.error.is_some())
        );
        assert!(rows[7].eligible);
        assert_eq!(rows[8].density, Some(0.0));
        assert_eq!(rows[9].density, Some(100.0));
        assert!(!rows[10].eligible);
        assert_eq!(rows[10].descriptors, vec!["calc(1,2)x"]);
    }
    #[test]
    fn adjacent_candidates_without_whitespace_follow_url_tokenization() {
        let rows = parse_srcset("a,b, c, d 2x,");
        assert_eq!(
            rows.iter()
                .map(|row| row.raw_url.as_str())
                .collect::<Vec<_>>(),
            vec!["a,b", "c", "d"]
        );
        assert!(parse_srcset(" ,  , ").is_empty());
    }
}
