const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{execFileSync}=require('node:child_process');
const repo='E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-csv-streaming-integration-v1';
const relative='crates/core/src/store/sitemap_read_window.rs',file=path.join(repo,relative);
const old=execFileSync('git',['show','HEAD:'+relative],{cwd:repo}).toString(),current=fs.readFileSync(file,'utf8');
const marker='impl CrawlStore {',end='\n#[cfg(test)]';
const before=old.slice(old.indexOf(marker),old.indexOf(end)),actual=current.slice(current.indexOf(marker),current.indexOf(end));assert.equal(actual,before,'Existing facade method still exact before shared-context extraction');
const replacement=String.raw`impl CrawlStore {
    /// Complete verification and Expected comparison precede continuation
    /// selection. Shared preparation is also used by callback-bound exports.
    pub fn sitemap_read_window_with_membership_analysis(
        &self,
        request: &crate::SitemapMembershipAnalysisRequest,
        expected: Option<&SitemapReadWindowExpectedContext>,
    ) -> anyhow::Result<SitemapReadWindow> {
        request.validate()?;
        with_prepared_sitemap_read(self, expected, &SitemapReadControl::default(), |prepared| {
            let header = prepared.header;
            let rows = header.ledger.as_ref().map(|_| Vec::new());
            let mut read = SitemapReadWindow {
                crawl_config_sha256: header.crawl_config_sha256,
                generation: header.generation,
                ledger: header.ledger,
                inherited: header.inherited,
                rows,
                next_after_sequence: None,
                has_more: false,
                coverage: header.coverage,
                coverage_inventory_sha256: header.coverage_inventory_sha256,
                sitemap_source_measurement_summary: header.sitemap_source_measurement_summary,
                sitemap_source_measurement_context: header.sitemap_source_measurement_context,
                membership_analysis: None,
                reader_engine: header.reader_engine,
                catalog_state_sha256: header.catalog_state_sha256,
                materializing_selected_compatibility: header.materializing_selected_compatibility,
            };
            let filter = raw_filter(request);
            if let Some(after) = request.after_sequence {
                let matching = match prepared.raw {
                    Some(raw) => raw.continuation_matches(&filter, after)?,
                    None => false,
                };
                if !matching {
                    return Err(SitemapReadWindowError::InvalidContinuation.into());
                }
            }
            if let Some(raw) = prepared.raw
                && request.row_limit > 0
            {
                let page = raw.window(&filter, request.after_sequence, request.row_limit)
                    .map_err(|error| {
                        if error.is::<RawWindowByteBudget>() {
                            SitemapReadWindowError::ResponseByteBudget.into()
                        } else {
                            error
                        }
                    })?;
                read.rows = Some(page.rows);
                read.next_after_sequence = page.next_after_sequence;
                read.has_more = page.has_more;
            }
            read.membership_analysis = Some(project_prepared_window(
                prepared.analysis,
                prepared.raw,
                prepared.coverage,
                read.rows.as_deref().unwrap_or(&[]),
                request,
            )?);
            serde_json::to_writer(&mut WindowSize(0), &read).map_err(|error| -> anyhow::Error {
                if error.is_io() {
                    SitemapReadWindowError::ResponseByteBudget.into()
                } else {
                    error.into()
                }
            })?;
            Ok(read)
        })
    }
}
`;
fs.writeFileSync(file,current.slice(0,current.indexOf(marker))+replacement+current.slice(current.indexOf(end)));
console.log(JSON.stringify({refactored:relative,old_method_exact_before:true,compiler_tests:'NOT_RUN_COMPUTE_HOLD'}));
