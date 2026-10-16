//! Durable quota accounting only. No store creation, reconstruction, website requests or CSV actions.
use anyhow::Result;
use open_seo_spider_core::{CrawlQuotaSummary, CrawlStore};
use std::path::Path;

pub(crate) fn read_summary(path: &Path) -> Result<Option<CrawlQuotaSummary>> {
    CrawlStore::open_read_only(path)?.crawl_quota_summary()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quota_cli_accepts_only_explicit_store_and_has_no_mutating_or_fake_export_options() {
        use clap::Parser;
        let cli = crate::Cli::try_parse_from(["spider", "crawl-quotas", "saved.sqlite"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(crate::Command::CrawlQuotas { .. })
        ));
        for option in ["--csv", "--read", "--seed-url", "--max-urls"] {
            assert!(
                crate::Cli::try_parse_from([
                    "spider",
                    "crawl-quotas",
                    "saved.sqlite",
                    option,
                    "other"
                ])
                .is_err()
            );
        }
    }
    #[test]
    fn malformed_quota_store_is_not_converted_or_replaced_by_readonly_service() {
        let path = std::env::temp_dir().join(format!(
            "spider-malformed-quotas-{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let before = b"not a sqlite store; preserve this evidence";
        std::fs::write(&path, before).unwrap();
        let result = read_summary(&path);
        let after = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(result.is_err());
        assert_eq!(after, before);
    }
    #[test]
    fn missing_quota_store_is_never_created_by_readonly_service() {
        let path = std::env::temp_dir().join(format!(
            "spider-absent-quotas-{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        assert!(!path.exists());
        assert!(read_summary(&path).is_err());
        assert!(!path.exists());
    }
}
