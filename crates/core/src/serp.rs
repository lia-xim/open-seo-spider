//! Offline metadata editing: row identity and values never depend on URL fetches.
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

const MAX_ROWS: usize = 10_000;
const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SerpInputRow {
    pub url: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct SerpRow {
    pub source_row: usize,
    pub url: String,
    pub title: String,
    pub description: String,
    pub title_characters: usize,
    pub description_characters: usize,
    pub title_pixels: Option<u32>,
    pub description_pixels: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct SerpPreview {
    pub rows: Vec<SerpRow>,
    pub website_requests: usize,
    pub measurement_profile: &'static str,
    pub limitations: &'static str,
}

pub fn preview_serp(rows: &[SerpInputRow]) -> anyhow::Result<SerpPreview> {
    if rows.len() > MAX_ROWS {
        bail!("Offline editor accepts at most {MAX_ROWS} rows");
    }
    let mut bytes = 0usize;
    for row in rows {
        for value in [&row.url, &row.title, &row.description] {
            if value.len() > 65536 {
                bail!("Each offline field must fit within 65536 UTF-8 bytes");
            }
            bytes = bytes
                .checked_add(value.len())
                .context("Offline input size overflow")?;
        }
    }
    if bytes > MAX_BYTES {
        bail!("Offline input exceeds 8 MiB");
    }
    Ok(SerpPreview {
        rows: rows
            .iter()
            .enumerate()
            .map(|(index, row)| SerpRow {
                source_row: index + 1,
                url: row.url.clone(),
                title: row.title.clone(),
                description: row.description.clone(),
                title_characters: row.title.chars().count(),
                description_characters: row.description.chars().count(),
                title_pixels: crate::text_metrics::text_width(&row.title, 20),
                description_pixels: crate::text_metrics::text_width(&row.description, 14),
            })
            .collect(),
        website_requests: 0,
        measurement_profile: crate::text_metrics::ENGINE,
        limitations: "Approximate local measurements, not predicted Google display. Unsupported scripts/shaping/glyphs have unknown pixel widths; inputs are preserved exactly. No URL is requested.",
    })
}

pub fn import_serp_csv(input: &str) -> anyhow::Result<Vec<SerpInputRow>> {
    if input.len() > MAX_BYTES {
        bail!("CSV input exceeds 8 MiB");
    }
    let mut reader =
        csv::ReaderBuilder::new().from_reader(input.trim_start_matches('\u{feff}').as_bytes());
    let headers = reader.headers().context("Invalid CSV header")?.clone();
    let aliases: [(&str, &[&str]); 3] = [
        ("URL", &["url", "address", "adresse"]),
        (
            "Title",
            &[
                "title",
                "title 1",
                "titel",
                "titel 1",
                "seitentitel",
                "seitentitel 1",
            ],
        ),
        (
            "Description",
            &[
                "description",
                "meta description",
                "meta description 1",
                "meta-beschreibung",
                "meta-beschreibung 1",
            ],
        ),
    ];
    let mut columns = Vec::new();
    for (label, names) in aliases {
        let indices: Vec<_> = headers
            .iter()
            .enumerate()
            .filter(|(_, h)| names.contains(&h.trim().to_lowercase().as_str()))
            .map(|(i, _)| i)
            .collect();
        if indices.len() != 1 {
            bail!(
                "CSV must have exactly one {label} column (URL, Title, Description or supported aliases)"
            );
        }
        columns.push(indices[0]);
    }
    let mut rows = Vec::new();
    for (index, record) in reader.records().enumerate() {
        if index >= MAX_ROWS {
            bail!("CSV accepts at most {MAX_ROWS} data rows");
        }
        let record = record.with_context(|| format!("Invalid CSV data row {}", index + 1))?;
        rows.push(SerpInputRow {
            url: record[columns[0]].into(),
            title: record[columns[1]].into(),
            description: record[columns[2]].into(),
        });
    }
    preview_serp(&rows)?;
    Ok(rows)
}

pub fn export_serp_csv(rows: &[SerpInputRow]) -> anyhow::Result<String> {
    let preview = preview_serp(rows)?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "URL",
        "Title",
        "Description",
        "Source Row",
        "Title Characters",
        "Description Characters",
        "Title Pixels",
        "Description Pixels",
    ])?;
    for row in preview.rows {
        writer.write_record([
            crate::export::csv_text(&row.url),
            crate::export::csv_text(&row.title),
            crate::export::csv_text(&row.description),
            row.source_row.to_string(),
            row.title_characters.to_string(),
            row.description_characters.to_string(),
            row.title_pixels.map(|n| n.to_string()).unwrap_or_default(),
            row.description_pixels
                .map(|n| n.to_string())
                .unwrap_or_default(),
        ])?;
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn german_reference_headers_preserve_authored_values_without_network_requests() {
        let rows = import_serp_csv("Adresse,Titel 1,Meta Description 1\nhttps://fixture.invalid/a,Unser ursprünglicher Titel,Unsere ursprüngliche Beschreibung\n").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Unser ursprünglicher Titel");
        assert_eq!(rows[0].description, "Unsere ursprüngliche Beschreibung");
        assert_eq!(preview_serp(&rows).unwrap().website_requests, 0);
        assert_eq!(
            import_serp_csv("Adresse,Titel,Meta Description 1\nlocal-only,Titel,Beschreibung\n")
                .unwrap()[0]
                .title,
            "Titel"
        );
    }
    #[test]
    fn csv_preserves_order_duplicates_unicode_quotes_and_newlines() {
        let rows=import_serp_csv("\u{feff}URL,Title,Description\r\nhttps://fixture.invalid/a,\"Über \"\"uns\"\"\",one\r\nhttps://fixture.invalid/a,second,\"two\nlines\"\r\n").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].url, rows[1].url);
        assert_eq!(rows[1].description, "two\nlines");
        assert_eq!(rows[0].title, "Über \"uns\"");
        let preview = preview_serp(&rows).unwrap();
        assert_eq!(preview.website_requests, 0);
        assert_eq!(preview.rows[1].source_row, 2);
    }
    #[test]
    fn unknown_width_is_not_zero_and_empty_is_known_zero() {
        let rows = vec![SerpInputRow {
            url: "file:///not-requested".into(),
            title: "😀".into(),
            description: "".into(),
        }];
        let result = preview_serp(&rows).unwrap();
        assert_eq!(result.rows[0].title_characters, 1);
        assert_eq!(result.rows[0].title_pixels, None);
        assert_eq!(result.rows[0].description_pixels, Some(0));
        assert_eq!(result.website_requests, 0);
    }
    #[test]
    fn export_escapes_formulas_and_keeps_exact_project_input() {
        let row = SerpInputRow {
            url: "https://fixture.invalid/".into(),
            title: " \t=SUM(1,2)".into(),
            description: "Quote, \"line\"\nnext".into(),
        };
        let output = export_serp_csv(std::slice::from_ref(&row)).unwrap();
        let mut reader = csv::Reader::from_reader(output.as_bytes());
        let record = reader.records().next().unwrap().unwrap();
        assert_eq!(&record[1], "' \t=SUM(1,2)");
        assert_eq!(&record[2], row.description);
        assert_eq!(row.title, " \t=SUM(1,2)");
    }
    #[test]
    fn malformed_ambiguous_and_oversized_inputs_are_explicit_errors() {
        for input in [
            "url,title\na,b\n",
            "url,title,description,Title\na,b,c,d\n",
            "url,title,description\na,b\n",
        ] {
            assert!(import_serp_csv(input).is_err());
        }
        assert!(preview_serp(&vec![SerpInputRow::default(); MAX_ROWS + 1]).is_err());
        assert!(
            preview_serp(&[SerpInputRow {
                title: "x".repeat(65537),
                ..Default::default()
            }])
            .is_err()
        );
        assert!(
            import_serp_csv(&format!(
                "url,title,description\na,{},b\n",
                "x".repeat(65537)
            ))
            .is_err()
        );
    }
}
