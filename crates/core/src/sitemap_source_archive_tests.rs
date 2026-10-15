//! AUTHORED / NOT_COMPILED / NOT_RUN. Config and actual binary filesystem tests.
//! FS tests below do NOT attest an HTTP capture/measurement/native process.
use super::*;

#[test]
fn source_archive_optional_config_is_strict_omitted_and_independent() {
    let absent: CrawlConfig = serde_json::from_str("{}").unwrap();
    let null: CrawlConfig = serde_json::from_str(r#"{"sitemap_source_archive":null}"#).unwrap();
    assert_eq!(
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&null).unwrap()
    );
    assert!(
        serde_json::to_value(&absent)
            .unwrap()
            .get("sitemap_source_archive")
            .is_none()
    );
    let explicit: CrawlConfig = serde_json::from_str(r#"{"sitemap_source_archive":{}}"#).unwrap();
    assert_eq!(explicit.sitemap_source_archive, Some(Default::default()));
    assert_eq!(
        explicit.sitemap_source_body_limit().unwrap(),
        absent.sitemap_source_body_limit().unwrap()
    );
    assert_eq!(
        explicit.sitemap_source_parser_limits().unwrap(),
        absent.sitemap_source_parser_limits().unwrap()
    );
    assert_eq!(
        explicit.sitemap_membership_ledger_limits().unwrap(),
        absent.sitemap_membership_ledger_limits().unwrap()
    );
    for value in [
        "[]",
        "false",
        "1",
        r#"{"extra":1}"#,
        r#"{"max_bytes_per_body":67108865}"#,
        r#"{"max_total_body_bytes":0}"#,
        r#"{"max_capture_records":100001}"#,
        r#"{"max_capture_records":1.5}"#,
        r#"{"max_bytes_per_body":"64M"}"#,
        r#"{"retain_post_http_entity":false,"retain_decoded_xml":false}"#,
        r#"{"max_capture_records":1,"max_capture_records":2}"#,
    ] {
        assert!(
            serde_json::from_str::<CrawlConfig>(&format!("{{\"sitemap_source_archive\":{value}}}"))
                .is_err(),
            "{value}"
        );
    }
    let source:CrawlConfig=serde_json::from_str(r#"{"sitemap_source_processing":{"max_post_http_entity_bytes":67108864,"max_decoded_xml_bytes":67108864}}"#).unwrap();
    assert!(source.sitemap_source_archive.is_none());
}

#[test]
fn source_archive_binary_writer_preserves_actual_64mib_and_owned_lease() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("source.sitemap-bodies");
    let mut lease = files::ArchiveWriter::acquire(&root).unwrap();
    assert!(files::ArchiveWriter::acquire(&root).is_err());
    let mut bytes = vec![0x5au8; MAX_SITEMAP_ARCHIVE_BODY_BYTES as usize];
    bytes[0] = 0xef;
    *bytes.last_mut().unwrap() = 0xa7;
    let body = SitemapArchivedBody {
        body_id: digest(b"binary-64mib-fs-unit-only"),
        representation: SitemapBodyRepresentation::DecodedXml,
        extent: SitemapBodyExtent::Complete,
        bytes: bytes.len() as u64,
        sha256: digest(&bytes),
    };
    let mut published = lease
        .publish(&[(body.clone(), bytes.as_slice())], &Default::default())
        .unwrap();
    published.retain();
    lease.release().unwrap();
    let stored = root.join(format!("{}.bin", body.body_id));
    assert_eq!(
        std::fs::metadata(&stored).unwrap().len(),
        MAX_SITEMAP_ARCHIVE_BODY_BYTES
    );
    assert_eq!(digest(std::fs::read(&stored).unwrap()), body.sha256);
    assert!(!root.join(".__oss_body_active").exists());
    // Same immutable object can be verified/reused; a corrupt one is not replaced.
    std::fs::write(&stored, b"corruption").unwrap();
    let mut retry = files::ArchiveWriter::acquire(&root).unwrap();
    assert!(
        retry
            .publish(&[(body, bytes.as_slice())], &Default::default())
            .is_err()
    );
    retry.release().unwrap();
    assert_eq!(std::fs::read(stored).unwrap(), b"corruption");
}

#[test]
fn source_archive_uncommitted_owned_files_cleanup_and_foreign_staging_refusal() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("archive");
    let mut writer = files::ArchiveWriter::acquire(&root).unwrap();
    let data = b"\xef\xbb\xbf<?xml version='1.0'?>\r\n<urlset><!-- &amp; --></urlset>";
    let body = SitemapArchivedBody {
        body_id: digest(b"owned-small-fs-unit"),
        representation: SitemapBodyRepresentation::DecodedXml,
        extent: SitemapBodyExtent::Complete,
        bytes: data.len() as u64,
        sha256: digest(data),
    };
    {
        let _not_committed = writer
            .publish(&[(body.clone(), data.as_slice())], &Default::default())
            .unwrap();
    }
    assert!(!root.join(format!("{}.bin", body.body_id)).exists());
    std::fs::write(root.join("foreign.txt"), b"preserve foreign input").unwrap();
    assert!(
        writer
            .publish(&[(body, data.as_slice())], &Default::default())
            .is_err()
    );
    writer.release().unwrap();
    assert_eq!(
        std::fs::read(root.join("foreign.txt")).unwrap(),
        b"preserve foreign input"
    );
}

#[test]
fn source_archive_sidecar_preserves_full_database_filename_and_separates_writers() {
    let temp = tempfile::tempdir().unwrap();
    let sqlite = files::archive_path(&temp.path().join("a.sqlite")).unwrap();
    let db = files::archive_path(&temp.path().join("a.db")).unwrap();
    let no_extension = files::archive_path(&temp.path().join("a")).unwrap();
    assert_eq!(sqlite, temp.path().join("a.sqlite.sitemap-bodies"));
    assert_eq!(db, temp.path().join("a.db.sitemap-bodies"));
    assert_ne!(sqlite, db);
    assert_ne!(sqlite, no_extension);
    assert_ne!(db, no_extension);
    let mut first = files::ArchiveWriter::acquire(&sqlite).unwrap();
    let mut second = files::ArchiveWriter::acquire(&db).unwrap();
    assert!(files::ArchiveWriter::acquire(&sqlite).is_err());
    second.release().unwrap();
    first.release().unwrap();
}

#[cfg(unix)]
#[test]
fn source_archive_root_and_leaf_symlinks_are_refused_without_outside_writes() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let outside = temp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("witness"), b"unchanged").unwrap();
    let root = temp.path().join("root");
    symlink(&outside, &root).unwrap();
    assert!(files::ArchiveWriter::acquire(&root).is_err());
    assert_eq!(
        std::fs::read(outside.join("witness")).unwrap(),
        b"unchanged"
    );
    std::fs::remove_file(&root).unwrap();
    let mut writer = files::ArchiveWriter::acquire(&root).unwrap();
    let data = b"unchanged";
    let body = SitemapArchivedBody {
        body_id: digest(b"symlink-leaf-fs-unit"),
        representation: SitemapBodyRepresentation::PostHttpEntity,
        extent: SitemapBodyExtent::Complete,
        bytes: data.len() as u64,
        sha256: digest(data),
    };
    symlink(
        outside.join("witness"),
        root.join(format!("{}.bin", body.body_id)),
    )
    .unwrap();
    assert!(
        writer
            .publish(&[(body, data.as_slice())], &Default::default())
            .is_err()
    );
    writer.release().unwrap();
    assert_eq!(std::fs::read(outside.join("witness")).unwrap(), data);
}
