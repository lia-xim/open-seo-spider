//! Independent bounded PDF observations through public lopdf APIs.
//! No rendering, OCR, JavaScript execution or password guessing is performed.
use crate::{PdfAnalysisConfig, PdfAnnotationLink, PdfPageText, PdfRecord, ResourceAnalysisStatus};
use anyhow::{Context, bail};
use flate2::{Decompress, FlushDecompress, Status};
use lopdf::{Dictionary, Document, LoadOptions, Object, ObjectId, Stream, content::Content};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use url::Url;

const ENGINE: &str = "lopdf-0.45.0/native-page-text-v1";
const MAX_GRAPH_DEPTH: usize = 128;
const PREVIEW_CHARS: usize = 1024;

pub(crate) fn is_pdf(body: &[u8], content_type: &str) -> bool {
    body.starts_with(b"%PDF-")
        || content_type.split(';').next().unwrap_or("").trim() == "application/pdf"
}

pub(crate) fn incomplete(reason: &str) -> PdfRecord {
    PdfRecord {
        engine: ENGINE.into(),
        status: ResourceAnalysisStatus::Partial,
        parse_error: Some(reason.into()),
        ..Default::default()
    }
}

pub(crate) fn analyze(body: &[u8], url: &Url, config: &PdfAnalysisConfig) -> PdfRecord {
    let mut result = PdfRecord {
        engine: ENGINE.into(),
        ..Default::default()
    };
    if !config.enabled {
        result.status = ResourceAnalysisStatus::Disabled;
        return result;
    }
    let doc = match Document::load_mem_with_options(
        body,
        LoadOptions {
            strict: true,
            max_decompressed_size: Some(config.max_decompressed_bytes),
            ..Default::default()
        },
    ) {
        Ok(doc) => doc,
        Err(error) => {
            if matches!(
                error,
                lopdf::Error::InvalidPassword | lopdf::Error::Decryption(_)
            ) {
                result.encrypted = Some(true);
                result.status = ResourceAnalysisStatus::Encrypted;
            }
            result.parse_error = Some(format!("{error:#}"));
            return result;
        }
    };
    result.version = Some(doc.version.clone());
    result.encrypted = Some(doc.is_encrypted() || doc.was_encrypted());
    if doc.is_encrypted() {
        result.status = ResourceAnalysisStatus::Encrypted;
        result.parse_error =
            Some("PDF requires a password; encrypted content was not analyzed".into());
        return result;
    }
    if doc.objects.len() > config.max_objects {
        result.limits_reached.push("max_objects".into());
        result.status = ResourceAnalysisStatus::Partial;
        result.parse_error = Some(format!(
            "PDF object limit ({}) exceeded",
            config.max_objects
        ));
        return result;
    }
    read_info(&doc, &mut result);
    let catalog = match doc.catalog() {
        Ok(catalog) => catalog,
        Err(error) => {
            result.parse_error = Some(format!("PDF catalog: {error:#}"));
            return result;
        }
    };
    let (pages, tree_complete) = page_tree(&doc, catalog, config, &mut result);
    result.page_count = tree_complete.then_some(pages.len());
    let base = uri_base(&doc, catalog, url, &mut result);
    let mut text = String::new();
    let mut text_complete = tree_complete;
    let mut annotations_complete = tree_complete;
    let mut annotation_count = 0;
    for (index, page_id) in pages.iter().enumerate() {
        let number = (index + 1) as u32;
        let page = match doc.get_dictionary(*page_id) {
            Ok(page) => page,
            Err(error) => {
                result.errors.push(format!("Page {number}: {error:#}"));
                text_complete = false;
                annotations_complete = false;
                continue;
            }
        };
        if annotation_count < config.max_annotations {
            if let Err(error) = read_annotations(
                &doc,
                page,
                number,
                base.as_ref(),
                config,
                &mut annotation_count,
                &mut result,
            ) {
                result
                    .errors
                    .push(format!("Page {number} annotations: {error:#}"));
                annotations_complete = false;
            }
        } else if page.has(b"Annots") {
            annotations_complete = false;
            add_limit(&mut result, "max_annotations");
        }
        if text.len() >= config.max_text_bytes {
            text_complete = false;
            add_limit(&mut result, "max_text_bytes");
            continue;
        }
        result.text_pages_processed += 1;
        let mut row = PdfPageText {
            page: number,
            ..Default::default()
        };
        let budget = config
            .max_text_bytes_per_page
            .min(config.max_text_bytes - text.len());
        match page_text(&doc, *page_id, page, config, budget) {
            Ok(page_text) => {
                row.chars = Some(page_text.chars().count());
                row.word_count = Some(page_text.split_whitespace().count());
                row.preview = Some(page_text.chars().take(PREVIEW_CHARS).collect());
                text.push_str(&page_text);
            }
            Err(error) => {
                let message = format!("{error:#}");
                if message.contains("limit") || message.contains("budget") {
                    row.truncated = true;
                    add_limit(&mut result, "text_or_decompression_budget");
                }
                row.error = Some(message);
                text_complete = false;
            }
        }
        result.text_pages.push(row);
    }
    if result.limits_reached.iter().any(|v| v == "max_annotations") {
        annotations_complete = false;
    }
    result.text_complete = text_complete;
    result.annotations_complete = annotations_complete;
    result.text_preview =
        (!text.is_empty() || text_complete).then(|| text.chars().take(PREVIEW_CHARS).collect());
    if text_complete {
        result.text_chars = Some(text.chars().count());
        result.word_count = Some(text.split_whitespace().count());
        result.text_hash = Some(format!("{:x}", Sha256::digest(text.as_bytes())));
    }
    result.status = if tree_complete
        && text_complete
        && annotations_complete
        && result.errors.is_empty()
        && result.limits_reached.is_empty()
    {
        ResourceAnalysisStatus::Complete
    } else {
        ResourceAnalysisStatus::Partial
    };
    result
}

fn add_limit(result: &mut PdfRecord, name: &str) {
    if !result.limits_reached.iter().any(|v| v == name) {
        result.limits_reached.push(name.into());
    }
}

fn text_string(obj: &Object) -> anyhow::Result<String> {
    let bytes = obj.as_str()?;
    if bytes.starts_with(b"\xfe\xff") && !bytes.len().is_multiple_of(2) {
        bail!("Malformed odd-length UTF-16BE PDF text string");
    }
    Ok(lopdf::decode_text_string(obj)?
        .trim_start_matches('\u{feff}')
        .to_owned())
}

fn read_info(doc: &Document, result: &mut PdfRecord) {
    let Ok(info) = doc.trailer.get(b"Info") else {
        return;
    };
    let info = match doc.dereference(info).and_then(|(_, obj)| obj.as_dict()) {
        Ok(info) => info,
        Err(error) => {
            result.errors.push(format!("Info dictionary: {error:#}"));
            return;
        }
    };
    for key in [
        "Title",
        "Author",
        "Subject",
        "Keywords",
        "Creator",
        "Producer",
        "CreationDate",
        "ModDate",
    ] {
        if let Ok(value) = info.get(key.as_bytes()) {
            match doc
                .dereference(value)
                .map_err(anyhow::Error::from)
                .and_then(|(_, obj)| text_string(obj))
            {
                Ok(value) if value.len() <= 65536 => {
                    result.info_metadata.insert(key.into(), value);
                }
                Ok(_) => {
                    result
                        .errors
                        .push(format!("Info {key} exceeds metadata value limit"));
                }
                Err(error) => {
                    result.errors.push(format!("Info {key}: {error:#}"));
                }
            }
        }
    }
}

fn uri_base(
    doc: &Document,
    catalog: &Dictionary,
    url: &Url,
    result: &mut PdfRecord,
) -> Option<Url> {
    let Ok(uri) = catalog.get(b"URI") else {
        return Some(url.clone());
    };
    let base = doc
        .dereference(uri)
        .map_err(anyhow::Error::from)
        .and_then(|(_, obj)| Ok(obj.as_dict()?))
        .and_then(|dict| Ok(dict.get_deref(b"Base", doc)?))
        .and_then(text_string)
        .and_then(|base| url.join(&base).map_err(Into::into));
    match base {
        Ok(base) if matches!(base.scheme(), "http" | "https") => Some(base),
        Ok(base) => {
            result.warnings.push("PDF URI base uses a non-HTTP(S) scheme; relative URI actions do not establish web discovery URLs".into());
            Some(base)
        }
        Err(error) => {
            result.errors.push(format!("PDF URI base: {error:#}"));
            None
        }
    }
}

enum TreeStep {
    Enter(ObjectId, usize),
    Exit(usize, usize),
}

fn page_tree(
    doc: &Document,
    catalog: &Dictionary,
    config: &PdfAnalysisConfig,
    result: &mut PdfRecord,
) -> (Vec<ObjectId>, bool) {
    let root = match catalog.get(b"Pages").and_then(Object::as_reference) {
        Ok(root) => root,
        Err(error) => {
            result.errors.push(format!("Page tree root: {error:#}"));
            return (Vec::new(), false);
        }
    };
    let mut stack = vec![TreeStep::Enter(root, 0)];
    let mut seen = HashSet::new();
    let mut pages = Vec::new();
    let mut complete = true;
    while let Some(step) = stack.pop() {
        let (id, depth) = match step {
            TreeStep::Exit(start, count) => {
                if pages.len() - start != count {
                    result.errors.push(format!(
                        "Page tree Count {count} does not match observed descendants {}",
                        pages.len() - start
                    ));
                    complete = false;
                }
                continue;
            }
            TreeStep::Enter(id, depth) => (id, depth),
        };
        if depth > MAX_GRAPH_DEPTH || seen.len() >= config.max_objects {
            add_limit(result, "page_tree_depth_or_objects");
            complete = false;
            break;
        }
        if !seen.insert(id) {
            result.errors.push(format!(
                "Page tree cycle or duplicate node {} {}",
                id.0, id.1
            ));
            complete = false;
            continue;
        }
        let node = match doc.get_dictionary(id) {
            Ok(node) => node,
            Err(error) => {
                result
                    .errors
                    .push(format!("Page tree node {}: {error:#}", id.0));
                complete = false;
                continue;
            }
        };
        match node.get(b"Type").and_then(Object::as_name) {
            Ok(b"Page") => {
                if pages.len() >= config.max_pages {
                    add_limit(result, "max_pages");
                    complete = false;
                    break;
                }
                pages.push(id);
            }
            Ok(b"Pages") => {
                match node
                    .get(b"Count")
                    .and_then(Object::as_i64)
                    .ok()
                    .and_then(|v| usize::try_from(v).ok())
                {
                    Some(count) => {
                        if id == root {
                            result.declared_page_count = Some(count);
                        }
                        stack.push(TreeStep::Exit(pages.len(), count));
                    }
                    None => {
                        result
                            .errors
                            .push(format!("Page tree node {} has invalid Count", id.0));
                        complete = false;
                    }
                }
                match node.get_deref(b"Kids", doc).and_then(Object::as_array) {
                    Ok(kids) => {
                        if kids.len() > config.max_objects
                            || stack.len().saturating_add(kids.len()) > config.max_objects
                        {
                            add_limit(result, "max_objects");
                            complete = false;
                            break;
                        }
                        for kid in kids.iter().rev() {
                            match kid.as_reference() {
                                Ok(id) => stack.push(TreeStep::Enter(id, depth + 1)),
                                Err(error) => {
                                    result.errors.push(format!("Page tree child: {error:#}"));
                                    complete = false;
                                }
                            }
                        }
                    }
                    Err(error) => {
                        result.errors.push(format!("Page tree Kids: {error:#}"));
                        complete = false;
                    }
                }
            }
            _ => {
                result.errors.push(format!(
                    "Unsupported page tree node type at {} {}",
                    id.0, id.1
                ));
                complete = false;
            }
        }
    }
    (pages, complete)
}

fn read_annotations(
    doc: &Document,
    page: &Dictionary,
    number: u32,
    base: Option<&Url>,
    config: &PdfAnalysisConfig,
    count: &mut usize,
    result: &mut PdfRecord,
) -> anyhow::Result<()> {
    if !page.has(b"Annots") {
        return Ok(());
    }
    let annotations = page.get_deref(b"Annots", doc)?.as_array()?;
    let mut failures = Vec::new();
    for value in annotations {
        if *count >= config.max_annotations {
            add_limit(result, "max_annotations");
            break;
        }
        *count += 1;
        let row = (|| -> anyhow::Result<Option<PdfAnnotationLink>> {
            let (_, value) = doc.dereference(value)?;
            let annotation = value.as_dict()?;
            if annotation.get(b"Subtype")?.as_name()? != b"Link" {
                return Ok(None);
            }
            if !annotation.has(b"A") {
                return Ok(None);
            } // Local destinations are not web URI actions.
            let action = annotation.get_deref(b"A", doc)?.as_dict()?;
            if action.get(b"S")?.as_name()? != b"URI" {
                return Ok(None);
            }
            let raw_uri = text_string(action.get_deref(b"URI", doc)?)?;
            if raw_uri.len() > 65536 {
                bail!("URI annotation exceeds value limit");
            }
            let url = crate::normalize_url(&raw_uri, base).map(|v| v.to_string());
            Ok(Some(PdfAnnotationLink {
                page: number,
                raw_uri,
                url,
            }))
        })();
        match row {
            Ok(Some(row)) => result.annotation_links.push(row),
            Ok(None) => (),
            Err(error) => failures.push(format!("{error:#}")),
        }
    }
    if !failures.is_empty() {
        bail!("{}", failures.join("; "));
    }
    Ok(())
}

fn resources(doc: &Document, page_id: ObjectId) -> anyhow::Result<Option<&Dictionary>> {
    let mut id = page_id;
    let mut seen = HashSet::new();
    loop {
        if !seen.insert(id) || seen.len() > MAX_GRAPH_DEPTH {
            bail!("PDF inherited resources cycle/depth limit");
        }
        let node = doc.get_dictionary(id)?;
        if node.has(b"Resources") {
            return Ok(Some(node.get_deref(b"Resources", doc)?.as_dict()?));
        }
        if !node.has(b"Parent") {
            return Ok(None);
        }
        id = node.get(b"Parent")?.as_reference()?;
    }
}

fn content_bytes(doc: &Document, page: &Dictionary, max: usize) -> anyhow::Result<Vec<u8>> {
    if !page.has(b"Contents") {
        return Ok(Vec::new());
    }
    let value = page.get_deref(b"Contents", doc)?;
    let items: Vec<&Object> = match value {
        Object::Array(values) => values.iter().collect(),
        _ => vec![value],
    };
    let mut bytes = Vec::new();
    for item in items {
        let (_, obj) = doc.dereference(item)?;
        let stream = obj.as_stream()?;
        let remaining = max.saturating_sub(bytes.len());
        let decoded = strict_stream_bytes(stream, remaining)?;
        if decoded.len().saturating_add(bytes.len()) > max {
            bail!("PDF combined page decompression limit exceeded");
        }
        bytes.extend_from_slice(&decoded);
    }
    Ok(bytes)
}

fn strict_stream_bytes(stream: &Stream, max: usize) -> anyhow::Result<Vec<u8>> {
    if !stream.dict.has(b"Filter") {
        if stream.content.len() > max {
            bail!("PDF page decompression limit exceeded");
        }
        return Ok(stream.content.clone());
    }
    let filters = stream.filters()?;
    if let Ok(parameters) = stream.dict.get(b"DecodeParms")
        && !matches!(parameters, Object::Null)
    {
        let dictionaries = match parameters {
            Object::Array(values) => values.as_slice(),
            value => std::slice::from_ref(value),
        };
        for dictionary in dictionaries {
            if matches!(dictionary, Object::Null) {
                continue;
            }
            let dictionary = dictionary.as_dict()?;
            if dictionary.has(b"Predictor") && dictionary.get(b"Predictor")?.as_i64()? != 1 {
                bail!("PDF nontrivial stream predictors are unsupported by native text profile");
            }
        }
    }
    let mut bytes = stream.content.clone();
    for filter in filters {
        if !matches!(filter, b"FlateDecode" | b"Fl") {
            bail!(
                "Unsupported PDF native text stream filter {}",
                String::from_utf8_lossy(filter)
            );
        }
        bytes = strict_zlib(&bytes, max)?;
    }
    if bytes.len() > max {
        bail!("PDF decompression limit exceeded");
    }
    Ok(bytes)
}

fn strict_zlib(input: &[u8], max: usize) -> anyhow::Result<Vec<u8>> {
    let mut decoder = Decompress::new(true);
    let mut result = Vec::new();
    loop {
        let remaining = max.saturating_sub(result.len());
        let mut buffer = vec![0; remaining.saturating_add(1).min(65536)];
        let before_in = decoder.total_in();
        let before_out = decoder.total_out();
        let status = decoder
            .decompress(
                &input[before_in as usize..],
                &mut buffer,
                FlushDecompress::Finish,
            )
            .context("Invalid PDF zlib stream")?;
        let written = (decoder.total_out() - before_out) as usize;
        if written > remaining {
            bail!("PDF decompression limit exceeded");
        }
        result.extend_from_slice(&buffer[..written]);
        if status == Status::StreamEnd {
            if input[decoder.total_in() as usize..]
                .iter()
                .any(|byte| !byte.is_ascii_whitespace())
            {
                bail!("PDF zlib stream contains unexpected trailing data");
            }
            return Ok(result);
        }
        if decoder.total_in() == before_in && decoder.total_out() == before_out {
            bail!("PDF zlib stream is truncated or malformed before stream end");
        }
    }
}

fn page_text(
    doc: &Document,
    page_id: ObjectId,
    page: &Dictionary,
    config: &PdfAnalysisConfig,
    budget: usize,
) -> anyhow::Result<String> {
    let bytes =
        content_bytes(doc, page, config.max_decompressed_bytes).context("PDF page content")?;
    let operations = Content::decode(&bytes)?.operations;
    let resources = resources(doc, page_id)?;
    let fonts = resources
        .filter(|v| v.has(b"Font"))
        .map(|v| v.get_deref(b"Font", doc).and_then(Object::as_dict))
        .transpose()?;
    let mut encodings = BTreeMap::new();
    let mut active_font: Option<Vec<u8>> = None;
    let mut font_stack = Vec::new();
    let mut in_text = false;
    let mut text = String::new();
    for operation in operations {
        match operation.operator.as_str() {
            "BT" => {
                if in_text {
                    bail!("Nested PDF text objects");
                }
                in_text = true;
            }
            "q" => {
                if font_stack.len() >= MAX_GRAPH_DEPTH {
                    bail!("PDF graphics state depth limit");
                }
                font_stack.push(active_font.clone());
            }
            "Q" => {
                active_font = font_stack.pop().context("Unbalanced PDF graphics state")?;
            }
            "Tf" => {
                let name = operation
                    .operands
                    .first()
                    .context("Missing PDF Tf font operand")?
                    .as_name()?;
                let font = fonts
                    .context("PDF text operator has no font resources")?
                    .get_deref(name, doc)?
                    .as_dict()?;
                if !encodings.contains_key(name) {
                    let subtype = font.get(b"Subtype")?.as_name()?;
                    if !matches!(subtype, b"Type1" | b"TrueType" | b"Type0") {
                        bail!("Unsupported PDF font subtype for native text decoding");
                    }
                    if !font.has(b"Encoding") && !font.has(b"ToUnicode") {
                        let base = font.get(b"BaseFont")?.as_name()?;
                        let standard = [
                            b"Helvetica".as_slice(),
                            b"Helvetica-Bold",
                            b"Helvetica-Oblique",
                            b"Helvetica-BoldOblique",
                            b"Times-Roman",
                            b"Times-Bold",
                            b"Times-Italic",
                            b"Times-BoldItalic",
                            b"Courier",
                            b"Courier-Bold",
                            b"Courier-Oblique",
                            b"Courier-BoldOblique",
                        ];
                        if !standard.contains(&base) {
                            bail!(
                                "PDF font has no explicit character mapping and is not a supported standard Latin font"
                            );
                        }
                    }
                    if font.has(b"ToUnicode") {
                        let cmap = font.get_deref(b"ToUnicode", doc)?.as_stream()?;
                        strict_stream_bytes(cmap, config.max_decompressed_bytes)
                            .context("PDF ToUnicode stream validation")?;
                    }
                    let encoding = font
                        .get_font_encoding_with_limit(doc, config.max_decompressed_bytes)
                        .context("PDF font decoding")?;
                    encodings.insert(name.to_vec(), encoding);
                }
                active_font = Some(name.to_vec());
            }
            "Tj" | "TJ" | "'" | "\"" => {
                if !in_text {
                    bail!("PDF text-showing operator outside a text object");
                }
                let name = active_font
                    .as_ref()
                    .context("PDF text operator uses an unknown font")?;
                let encoding = encodings
                    .get(name)
                    .context("PDF font encoding unavailable")?;
                if matches!(operation.operator.as_str(), "'" | "\"") {
                    append(&mut text, "\n", budget)?;
                }
                let operand = if operation.operator == "\"" {
                    operation.operands.get(2)
                } else {
                    operation.operands.first()
                }
                .context("Missing PDF text operand")?;
                let items = match operand {
                    Object::Array(items) if operation.operator == "TJ" => items.as_slice(),
                    _ => std::slice::from_ref(operand),
                };
                for item in items {
                    match item {
                        Object::String(value, _) => {
                            let decoded = Document::decode_text(encoding, value)?;
                            if decoded.contains('\u{fffd}') {
                                bail!(
                                    "PDF text has unmapped or invalid characters; complete text counts are unknown"
                                );
                            }
                            append(&mut text, &decoded, budget)?;
                        }
                        Object::Integer(_) | Object::Real(_) if operation.operator == "TJ" => (),
                        _ => bail!("Unsupported or malformed PDF text operand"),
                    }
                }
            }
            "ET" | "T*" => {
                if !in_text {
                    bail!("PDF text positioning/end operator outside a text object");
                }
                if !text.is_empty() && !text.ends_with('\n') {
                    append(&mut text, "\n", budget)?;
                }
                if operation.operator == "ET" {
                    in_text = false;
                }
            }
            "Do" => {
                let name = operation
                    .operands
                    .first()
                    .context("Missing PDF Do operand")?
                    .as_name()?;
                let xobjects = resources
                    .context("PDF XObject has no resources")?
                    .get_deref(b"XObject", doc)?
                    .as_dict()?;
                let object = xobjects.get_deref(name, doc)?.as_stream()?;
                match object.dict.get(b"Subtype")?.as_name()? {
                    b"Image" => (),
                    b"Form" => {
                        bail!("Form XObject text is unsupported by this native PDF text profile")
                    }
                    _ => bail!("Unsupported PDF XObject subtype"),
                }
            }
            _ => (),
        }
    }
    if !font_stack.is_empty() {
        bail!("Unbalanced PDF graphics state");
    }
    if in_text {
        bail!("PDF text object is not terminated");
    }
    Ok(text)
}

fn append(text: &mut String, value: &str, budget: usize) -> anyhow::Result<()> {
    if text.len().saturating_add(value.len()) > budget {
        bail!("PDF text byte budget exceeded");
    }
    text.push_str(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{
        EncryptionState, EncryptionVersion, Permissions, Stream, dictionary,
        text_string as encoded_text,
    };

    fn original_document(texts: &[&str]) -> Document {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let font = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding" });
        let mut kids = Vec::new();
        for text in texts {
            let text = format!("BT /F1 12 Tf ({text}) Tj ET");
            let content = doc.add_object(Stream::new(dictionary! {}, text.into_bytes()));
            let annotation = dictionary! { "Subtype" => "Link", "A" => dictionary! { "S" => "URI", "URI" => encoded_text("../contact") } };
            let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages_id, "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } }, "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()], "Contents" => content, "Annots" => vec![annotation.into()] });
            kids.push(page.into());
        }
        doc.set_object(
            pages_id,
            dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => texts.len() as i64 },
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id, "URI" => dictionary! { "Base" => encoded_text("https://fixture.test/manuals/") } });
        doc.trailer.set("Root", catalog);
        let info = doc.add_object(dictionary! { "Title" => encoded_text("Grüße 日本"), "Author" => encoded_text("Original tests") });
        doc.trailer.set("Info", info);
        doc.trailer.set(
            "ID",
            vec![
                Object::string_literal("original-fixture-id"),
                Object::string_literal("original-fixture-id"),
            ],
        );
        doc
    }

    fn bytes(mut doc: Document) -> Vec<u8> {
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        bytes
    }

    fn run(doc: Document, config: &PdfAnalysisConfig) -> PdfRecord {
        analyze(
            &bytes(doc),
            &Url::parse("https://fixture.test/document.pdf").unwrap(),
            config,
        )
    }

    #[test]
    fn original_pdf_info_pages_compressed_text_and_direct_indirect_uri_annotations() {
        let mut doc = original_document(&["Hello world", "Second original page"]);
        let first_page = *doc.get_pages().get(&1).unwrap();
        let external = doc.add_object(dictionary! { "Subtype" => "Link", "A" => dictionary! { "S" => "URI", "URI" => encoded_text("https://external.test/other#fragment") } });
        doc.get_dictionary_mut(first_page)
            .unwrap()
            .get_mut(b"Annots")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .push(external.into());
        doc.compress();
        let result = run(doc, &PdfAnalysisConfig::default());
        assert_eq!(
            result.status,
            ResourceAnalysisStatus::Complete,
            "{result:#?}"
        );
        assert_eq!(result.page_count, Some(2));
        assert_eq!(result.declared_page_count, Some(2));
        assert_eq!(result.info_metadata["Title"], "Grüße 日本");
        assert_eq!(result.word_count, Some(5));
        assert!(result.text_complete && result.annotations_complete);
        assert_eq!(result.annotation_links.len(), 3);
        assert_eq!(
            result.annotation_links[0].url.as_deref(),
            Some("https://fixture.test/contact")
        );
        assert_eq!(
            result.annotation_links[1].url.as_deref(),
            Some("https://external.test/other")
        );
        assert_eq!(result.annotation_links[2].page, 2);
        assert!(result.text_hash.is_some());
    }

    #[test]
    fn damaged_page_trees_do_not_turn_declared_counts_into_proven_counts() {
        for cycle in [true, false] {
            let mut doc = original_document(&["Original page"]);
            let root = doc
                .catalog()
                .unwrap()
                .get(b"Pages")
                .unwrap()
                .as_reference()
                .unwrap();
            if cycle {
                doc.get_dictionary_mut(root)
                    .unwrap()
                    .set("Kids", vec![Object::Reference(root)]);
            } else {
                doc.get_dictionary_mut(root).unwrap().set("Count", 99);
            }
            let result = run(doc, &PdfAnalysisConfig::default());
            assert_eq!(result.status, ResourceAnalysisStatus::Partial);
            assert!(result.page_count.is_none());
            assert!(result.text_chars.is_none());
            assert!(!result.errors.is_empty());
        }
    }

    #[test]
    fn page_text_annotation_and_object_limits_are_visible_without_fake_zeroes() {
        for config in [
            PdfAnalysisConfig {
                max_pages: 1,
                ..Default::default()
            },
            PdfAnalysisConfig {
                max_text_bytes_per_page: 3,
                ..Default::default()
            },
            PdfAnalysisConfig {
                max_text_bytes: 3,
                ..Default::default()
            },
            PdfAnalysisConfig {
                max_annotations: 1,
                ..Default::default()
            },
            PdfAnalysisConfig {
                max_objects: 1,
                ..Default::default()
            },
            PdfAnalysisConfig {
                max_decompressed_bytes: 5,
                ..Default::default()
            },
        ] {
            let result = run(
                original_document(&["Hello original world", "Second page"]),
                &config,
            );
            assert_eq!(
                result.status,
                ResourceAnalysisStatus::Partial,
                "{result:#?}"
            );
            assert!(!result.limits_reached.is_empty());
            if config.max_annotations != 1 {
                assert!(result.text_chars.is_none());
            }
        }
    }

    #[test]
    fn corrupt_stream_missing_fonts_form_xobjects_and_invalid_annotations_fail_closed() {
        for case in ["stream", "font", "form", "annotation"] {
            let mut doc = original_document(&["Original text"]);
            let page_id = *doc.get_pages().get(&1).unwrap();
            let content_id = doc
                .get_dictionary(page_id)
                .unwrap()
                .get(b"Contents")
                .unwrap()
                .as_reference()
                .unwrap();
            match case {
                "stream" => {
                    let stream = doc
                        .get_object_mut(content_id)
                        .unwrap()
                        .as_stream_mut()
                        .unwrap();
                    stream.dict.set("Filter", "FlateDecode");
                    stream.set_content(b"not-zlib".to_vec());
                }
                "font" => {
                    doc.get_dictionary_mut(page_id)
                        .unwrap()
                        .set("Resources", dictionary! {});
                }
                "form" => {
                    let form = doc.add_object(Stream::new(
                        dictionary! { "Type" => "XObject", "Subtype" => "Form" },
                        b"BT /F1 12 Tf (hidden form text) Tj ET".to_vec(),
                    ));
                    doc.get_dictionary_mut(page_id)
                        .unwrap()
                        .get_mut(b"Resources")
                        .unwrap()
                        .as_dict_mut()
                        .unwrap()
                        .set("XObject", dictionary! { "Fm1" => form });
                    doc.get_object_mut(content_id)
                        .unwrap()
                        .as_stream_mut()
                        .unwrap()
                        .set_plain_content(b"/Fm1 Do".to_vec());
                }
                "annotation" => {
                    doc.get_dictionary_mut(page_id)
                        .unwrap()
                        .set("Annots", vec![Object::Integer(42)]);
                }
                _ => unreachable!(),
            }
            let result = run(doc, &PdfAnalysisConfig::default());
            assert_eq!(
                result.status,
                ResourceAnalysisStatus::Partial,
                "{case}: {result:#?}"
            );
            if case == "annotation" {
                assert!(!result.annotations_complete);
            } else {
                assert!(result.text_chars.is_none());
                assert!(result.text_pages[0].error.is_some());
            }
        }
    }

    #[test]
    fn encrypted_malformed_disabled_and_legitimate_blank_pdfs_remain_distinct() {
        let mut encrypted = original_document(&["Private fixture"]);
        let state = EncryptionState::try_from(EncryptionVersion::V1 {
            document: &encrypted,
            owner_password: "owner-test",
            user_password: "user-test",
            permissions: Permissions::all(),
        })
        .unwrap();
        encrypted.encrypt(&state).unwrap();
        let result = run(encrypted, &PdfAnalysisConfig::default());
        assert_eq!(
            result.status,
            ResourceAnalysisStatus::Encrypted,
            "{result:#?}"
        );
        assert_eq!(result.encrypted, Some(true));
        assert!(result.text_chars.is_none());
        let url = Url::parse("https://fixture.test/invalid.pdf").unwrap();
        let invalid = analyze(b"%PDF-1.7 invalid", &url, &PdfAnalysisConfig::default());
        assert_eq!(invalid.status, ResourceAnalysisStatus::Error);
        assert!(invalid.page_count.is_none() && invalid.parse_error.is_some());
        let disabled = run(
            original_document(&["Text"]),
            &PdfAnalysisConfig {
                enabled: false,
                ..Default::default()
            },
        );
        assert_eq!(disabled.status, ResourceAnalysisStatus::Disabled);
        assert!(disabled.text_chars.is_none());
        let mut blank = original_document(&[""]);
        let page = *blank.get_pages().get(&1).unwrap();
        blank.get_dictionary_mut(page).unwrap().remove(b"Contents");
        let result = run(blank, &PdfAnalysisConfig::default());
        assert_eq!(result.status, ResourceAnalysisStatus::Complete);
        assert_eq!(result.text_chars, Some(0));
        assert_eq!(result.word_count, Some(0));
    }

    #[test]
    fn bad_utf16_info_and_unsupported_uri_actions_do_not_invent_values_or_web_links() {
        let mut doc = original_document(&["Original"]);
        let info = doc.trailer.get(b"Info").unwrap().as_reference().unwrap();
        doc.get_dictionary_mut(info)
            .unwrap()
            .set("Title", Object::string_literal(vec![0xfe, 0xff, 0x00]));
        let page = *doc.get_pages().get(&1).unwrap();
        doc.get_dictionary_mut(page).unwrap().set("Annots", vec![
            Object::Dictionary(dictionary! { "Subtype" => "Link", "A" => dictionary! { "S" => "URI", "URI" => encoded_text("mailto:hello@example.org") } }),
            Object::Dictionary(dictionary! { "Subtype" => "Link", "A" => dictionary! { "S" => "JavaScript", "JS" => encoded_text("throw Error('do not execute')") } }),
        ]);
        let result = run(doc, &PdfAnalysisConfig::default());
        assert_eq!(result.status, ResourceAnalysisStatus::Partial);
        assert!(!result.info_metadata.contains_key("Title"));
        assert_eq!(result.annotation_links.len(), 1);
        assert!(result.annotation_links[0].url.is_none());
        assert_eq!(
            result.annotation_links[0].raw_uri,
            "mailto:hello@example.org"
        );
    }

    #[test]
    fn invalid_or_non_http_pdf_uri_base_does_not_invent_relative_web_urls() {
        for base in ["mailto:test@example.org", "http://[malformed"] {
            let mut doc = original_document(&["Original"]);
            doc.catalog_mut()
                .unwrap()
                .set("URI", dictionary! { "Base" => encoded_text(base) });
            let result = run(doc, &PdfAnalysisConfig::default());
            assert_eq!(result.annotation_links.len(), 1);
            assert_eq!(result.annotation_links[0].url, None);
            assert!(!result.warnings.is_empty() || !result.errors.is_empty());
        }
    }

    #[test]
    fn zlib_checksum_truncation_and_valid_empty_stream_are_distinct() {
        use std::io::Write;
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(b"Original stream").unwrap();
        let encoded = encoder.finish().unwrap();
        assert_eq!(strict_zlib(&encoded, 100).unwrap(), b"Original stream");
        assert!(strict_zlib(&encoded[..encoded.len() - 1], 100).is_err());
        let mut corrupted = encoded.clone();
        *corrupted.last_mut().unwrap() ^= 0xff;
        assert!(strict_zlib(&corrupted, 100).is_err());
        assert!(strict_zlib(&encoded, 5).is_err());
        let empty = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default())
            .finish()
            .unwrap();
        assert!(strict_zlib(&empty, 1).unwrap().is_empty());
    }

    #[test]
    fn content_arrays_concatenate_exact_bytes_and_text_object_errors_are_visible() {
        let mut doc = original_document(&["Unused"]);
        let page = *doc.get_pages().get(&1).unwrap();
        let first = doc.add_object(Stream::new(
            dictionary! {},
            b"BT /F1 12 Tf (Original ".to_vec(),
        ));
        let second = doc.add_object(Stream::new(dictionary! {}, b"stream) Tj ET".to_vec()));
        doc.get_dictionary_mut(page).unwrap().set(
            "Contents",
            vec![Object::Reference(first), Object::Reference(second)],
        );
        let result = run(doc, &PdfAnalysisConfig::default());
        assert_eq!(result.status, ResourceAnalysisStatus::Complete);
        assert_eq!(result.text_preview.as_deref(), Some("Original stream\n"));
        for invalid in [
            "/F1 12 Tf (outside) Tj",
            "BT /F1 12 Tf (unfinished) Tj",
            "BT BT ET",
        ] {
            let mut doc = original_document(&["Unused"]);
            let page = *doc.get_pages().get(&1).unwrap();
            let contents = doc
                .get_dictionary(page)
                .unwrap()
                .get(b"Contents")
                .unwrap()
                .as_reference()
                .unwrap();
            doc.get_object_mut(contents)
                .unwrap()
                .as_stream_mut()
                .unwrap()
                .set_plain_content(invalid.as_bytes().to_vec());
            let result = run(doc, &PdfAnalysisConfig::default());
            assert_eq!(result.status, ResourceAnalysisStatus::Partial);
            assert!(result.text_chars.is_none() && result.text_pages[0].error.is_some());
        }
    }
}
