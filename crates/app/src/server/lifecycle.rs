//! Local process lifecycle only; no HTTP administrator or MCP session protocol.
use super::*;
use futures::future::BoxFuture;
use open_seo_spider_core::{TaskAdmission, TaskOwner};
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Clone)]
pub(super) struct Lifecycle {
    pub(super) admission: TaskAdmission,
    pub(super) requests: TaskOwner,
    pub(super) blocking: TaskOwner,
    pub(super) crawls: TaskOwner,
}
impl Default for Lifecycle {
    fn default() -> Self {
        let admission = TaskAdmission::default();
        Self {
            requests: TaskOwner::with_admission("http_requests", admission.clone()),
            blocking: TaskOwner::with_admission("http_blocking", admission.clone()),
            crawls: TaskOwner::with_admission("crawl_operations", admission.clone()),
            admission,
        }
    }
}
pub(super) enum ShutdownSource {
    CtrlC,
    OwnedStdin,
    #[cfg(test)]
    Injected(BoxFuture<'static, anyhow::Result<&'static str>>),
}
pub(super) type ShutdownFuture = BoxFuture<'static, anyhow::Result<&'static str>>;
pub(super) type StdinMonitor = Option<tokio::task::JoinHandle<std::io::Result<()>>>;

pub(super) async fn wait_for_eof<R: AsyncRead + Unpin>(mut input: R) -> std::io::Result<()> {
    let mut buffer = [0; 8192];
    loop {
        if input.read(&mut buffer).await? == 0 {
            return Ok(());
        }
    }
}
pub(super) fn shutdown_source(source: ShutdownSource) -> (ShutdownFuture, StdinMonitor) {
    match source {
        ShutdownSource::CtrlC => (
            Box::pin(async {
                tokio::signal::ctrl_c().await?;
                Ok("ctrl_c")
            }),
            None,
        ),
        ShutdownSource::OwnedStdin => {
            let (ended, observed) = tokio::sync::oneshot::channel();
            let monitor = tokio::spawn(async move {
                let result = wait_for_eof(tokio::io::stdin()).await;
                let signal = result
                    .as_ref()
                    .map(|_| ())
                    .map_err(|error| std::io::Error::new(error.kind(), error.to_string()));
                let _ = ended.send(signal);
                result
            });
            (
                Box::pin(async move {
                    tokio::select! {
                        biased;
                        result=observed => { result??; Ok("owned_stdin_eof") },
                        result=tokio::signal::ctrl_c() => { result?; Ok("ctrl_c_waiting_owned_stdin_eof") },
                    }
                }),
                Some(monitor),
            )
        }
        #[cfg(test)]
        ShutdownSource::Injected(signal) => (signal, None),
    }
}

pub(super) async fn request(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    match state
        .lifecycle
        .requests
        .run_async(async move { next.run(request).await })
        .await
    {
        Ok(response) => response,
        Err(error) => api_error(StatusCode::SERVICE_UNAVAILABLE, error).into_response(),
    }
}

pub(super) type PreparedLaunch = (Json<Value>, Arc<RwLock<Job>>);
// Own preparation/reservation and the complete eventual run as ONE task before
// any mutation. An HTTP disconnect loses only the startup result receiver.
pub(super) async fn start_operation<F>(state: AppState, operation: F) -> ApiResult<Json<Value>>
where
    F: std::future::Future<Output = ApiResult<PreparedLaunch>> + Send + 'static,
{
    let owner = state.lifecycle.crawls.clone();
    owner.reap_completed().await;
    let (ready, startup) = tokio::sync::oneshot::channel();
    let ticket = owner
        .spawn_async(async move {
            match operation.await {
                Ok((response, job)) => {
                    let _ = ready.send(Ok(response));
                    // An admitted initialization may finish after global quiesce.
                    // Its launch remains owned; pause before it can schedule IO.
                    if state.lifecycle.admission.is_closed() {
                        let mut guard = job.write().await;
                        if guard.control.request_pause() {
                            guard.status = "pausing".into();
                        }
                    }
                    launch(state, job).await;
                }
                Err(error) => {
                    let _ = ready.send(Err(error));
                }
            }
        })
        .map_err(|error| api_error(StatusCode::SERVICE_UNAVAILABLE, error))?;
    drop(ticket); // Actual worker AND supervisor remain in owner slots.
    startup
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?
}

// Core PDF joins preserve their concrete error type through anyhow contexts.
// A domain crawl error still fails that job without poisoning unrelated drain.
pub(super) fn record_owned_crawl_failure(state: &AppState, error: &anyhow::Error) {
    if error.chain().any(|cause| {
        cause
            .downcast_ref::<open_seo_spider_core::OwnedTaskError>()
            .is_some()
    }) {
        state
            .lifecycle
            .crawls
            .record_failure(format!("Owned Core crawl task failed: {error:#}"));
    }
}

pub(super) fn close_admission(state: &AppState) {
    state.lifecycle.admission.close();
    state.exports.close_admission();
    state.finding_reads.close_admission();
    state.mcp.close_file_tools_admission();
}
pub(super) async fn pause_active(state: &AppState) {
    let jobs: Vec<_> = state.jobs.read().await.values().cloned().collect();
    for job in jobs {
        let mut guard = job.write().await;
        if matches!(guard.status.as_str(), "running" | "pausing") && guard.control.request_pause() {
            guard.status = "pausing".into();
        }
    }
}
pub(super) async fn drain(state: &AppState) -> anyhow::Result<()> {
    pause_active(state).await;
    // All ownership/admission was closed BEFORE the Axum signal resolved.
    // Await all results even after an error; no early ? can drop another drain.
    let (requests, blocking, crawls, csv, reads, files) = tokio::join!(
        state.lifecycle.requests.shutdown_and_drain(),
        state.lifecycle.blocking.shutdown_and_drain(),
        state.lifecycle.crawls.shutdown_and_drain(),
        state.exports.shutdown_and_drain(),
        state.finding_reads.shutdown_and_drain(),
        state.mcp.shutdown_file_tools(),
    );
    let mut errors = Vec::new();
    for result in [
        requests.map_err(anyhow::Error::from),
        blocking.map_err(anyhow::Error::from),
        crawls.map_err(anyhow::Error::from),
        csv,
        reads,
        files,
    ] {
        if let Err(error) = result {
            errors.push(error.to_string());
        }
    }
    anyhow::ensure!(
        errors.is_empty(),
        "Application lifecycle drain failed: {}",
        errors.join("; ")
    );
    Ok(())
}
pub(super) fn receipt(state: &AppState, cause: &str, clean: bool) {
    let stats = [&state.lifecycle.requests, &state.lifecycle.blocking, &state.lifecycle.crawls].map(|owner| {
        let s = owner.snapshot();
        json!({"owner":s.owner,"closing":s.closing,"admitted":s.admitted,"started":s.started,"joined":s.joined,"retained":s.retained,"first_failure":s.first_failure})
    });
    eprintln!(
        "{}",
        json!({"kind":"owned_application_shutdown_v1","cause":cause,"clean":clean,"owners":stats,"qualification":"Local application task joins only; actual process/pipe EOF is an independent outside witness; no stateful MCP session or external renderer-process drain claim"})
    );
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
