//! Complete loaded typed reports bound to the exact same compared clones.
//! This does not preserve raw input-file bytes or attest SQL/generation history.
use super::*;
use open_seo_spider_core::{COMPARISON_REPORT_HASH_PROFILE, ComparedSnapshot};
use sha2::{Digest, Sha256};

const BUNDLE_BINDING: &str = "same-typed-reports-as-comparison";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OriginalReportSnapshot {
    pub report: CrawlReport,
    pub typed_report_sha256: String,
    pub typed_report_json_bytes: usize,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OriginalReportsBundle {
    pub schema_version: u32,
    pub representation: String,
    pub binding: String,
    pub before: OriginalReportSnapshot,
    pub after: OriginalReportSnapshot,
}
struct ReportDigest {
    hash: Sha256,
    bytes: usize,
}
impl Write for ReportDigest {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let next = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("original report byte count overflow"))?;
        if next > MAX_RESPONSE_BYTES {
            return Err(std::io::Error::other(
                "original report cannot fit response budget",
            ));
        }
        self.hash.update(bytes);
        self.bytes = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn report_digest(report: &CrawlReport) -> Result<(String, usize), SavedComparisonFailure> {
    let mut digest = ReportDigest {
        hash: Sha256::new(),
        bytes: 0,
    };
    serde_json::to_writer(&mut digest, report)
        .map_err(|error| failure("original_report_bundle_byte_budget", error.to_string()))?;
    Ok((format!("{:x}", digest.hash.finalize()), digest.bytes))
}
fn validate_report(
    report: &CrawlReport,
    sha: &str,
    bytes: usize,
    expected: &ComparedSnapshot,
) -> Result<(), SavedComparisonFailure> {
    let (actual_sha, actual_bytes) = report_digest(report)?;
    if expected.typed_report_sha_profile != COMPARISON_REPORT_HASH_PROFILE
        || sha != actual_sha
        || sha != expected.typed_report_sha256
        || bytes != actual_bytes
        || bytes != expected.typed_report_json_bytes
    {
        return Err(failure(
            "original_report_binding_mismatch",
            "Complete loaded typed report differs from the compared snapshot hash/profile/byte count",
        ));
    }
    Ok(())
}
impl OriginalReportSnapshot {
    fn new(
        report: CrawlReport,
        expected: &ComparedSnapshot,
    ) -> Result<Self, SavedComparisonFailure> {
        let value = Self {
            report,
            typed_report_sha256: expected.typed_report_sha256.clone(),
            typed_report_json_bytes: expected.typed_report_json_bytes,
        };
        validate_report(
            &value.report,
            &value.typed_report_sha256,
            value.typed_report_json_bytes,
            expected,
        )?;
        Ok(value)
    }
}
impl OriginalReportsBundle {
    pub(super) fn new(
        comparison: &SavedComparisonResult,
        before: CrawlReport,
        after: CrawlReport,
    ) -> Result<Self, SavedComparisonFailure> {
        Ok(Self {
            schema_version: 1,
            representation: COMPARISON_REPORT_HASH_PROFILE.into(),
            binding: BUNDLE_BINDING.into(),
            before: OriginalReportSnapshot::new(before, &comparison.before_snapshot)?,
            after: OriginalReportSnapshot::new(after, &comparison.after_snapshot)?,
        })
    }
    pub(super) fn validate(
        &self,
        comparison: &SavedComparisonResult,
    ) -> Result<(), SavedComparisonFailure> {
        if self.schema_version != 1
            || self.representation != COMPARISON_REPORT_HASH_PROFILE
            || self.binding != BUNDLE_BINDING
        {
            return Err(failure(
                "original_report_binding_mismatch",
                "Unsupported original-report bundle contract",
            ));
        }
        validate_report(
            &self.before.report,
            &self.before.typed_report_sha256,
            self.before.typed_report_json_bytes,
            &comparison.before_snapshot,
        )?;
        validate_report(
            &self.after.report,
            &self.after.typed_report_sha256,
            self.after.typed_report_json_bytes,
            &comparison.after_snapshot,
        )
    }
}
