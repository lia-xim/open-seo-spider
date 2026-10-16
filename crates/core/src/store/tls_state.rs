use super::*;
use crate::tls::HstsSnapshot;

fn check_context(connection: &Connection, hash: &str, generation: u64) -> anyhow::Result<()> {
    let run: CrawlRunState =
        metadata(connection, "run_state")?.context("HSTS requires an active run")?;
    if run.generation != generation || run.outcome.is_some() {
        bail!("Stale HSTS writer generation");
    }
    let config: CrawlConfig =
        metadata(connection, "config")?.context("HSTS requires saved configuration")?;
    let tls = config
        .tls
        .as_ref()
        .filter(|c| c.respect_hsts)
        .context("HSTS is not enabled in saved configuration")?;
    if tls.digest()? != hash {
        bail!("HSTS trust/config binding differs");
    }
    Ok(())
}
impl CrawlStore {
    pub(crate) fn load_hsts_state(
        &self,
        hash: &str,
        generation: u64,
    ) -> anyhow::Result<Option<HstsSnapshot>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        check_context(&connection, hash, generation)?;
        let Some(text) = metadata_text(&connection, "tls_hsts_state")? else {
            return Ok(None);
        };
        if text.len() > 8 * 1024 * 1024 {
            bail!("HSTS state exceeds 8MiB");
        }
        let state: HstsSnapshot = serde_json::from_str(&text)?;
        state.validate(hash, generation)?;
        Ok(Some(state))
    }
    pub(crate) fn save_hsts_state(&self, state: &HstsSnapshot) -> anyhow::Result<()> {
        state.validate(&state.tls_config_sha256, state.generation)?;
        let bytes = serde_json::to_string(state)?;
        if bytes.len() > 8 * 1024 * 1024 {
            bail!("HSTS state exceeds 8MiB");
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        check_context(&transaction, &state.tls_config_sha256, state.generation)?;
        put_metadata(&transaction, "tls_hsts_state", &bytes)?;
        transaction.commit()?;
        Ok(())
    }
}
