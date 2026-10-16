// Future Root-run local TCP/TLS fixtures. This module creates and drains every
// listener, handshake and request task; no public network or OS trust mutation.
use super::*;
use crate::{CrawlConfig, Crawler};
use rcgen::{
    BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
};
use std::sync::atomic::AtomicBool;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    task::{JoinHandle, JoinSet},
    time::{Duration, timeout},
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
};

struct LocalTls {
    port: u16,
    ca: String,
    ca_der: Vec<u8>,
    log: Arc<Mutex<Vec<String>>>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}
impl LocalTls {
    async fn start() -> Self {
        let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        let ca_key = KeyPair::generate().unwrap();
        let ca = params.self_signed(&ca_key).unwrap();
        let issuer = Issuer::new(params, ca_key);
        let mut leaf_params = CertificateParams::new(vec!["localhost".into()]).unwrap();
        leaf_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let key = KeyPair::generate().unwrap();
        let leaf = leaf_params.signed_by(&key, &issuer).unwrap();
        let server = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf.der().clone(), ca.der().clone()],
            PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
        )
        .unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(server));
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let log = Arc::new(Mutex::new(Vec::new()));
        let log_task = log.clone();
        let (stop, mut stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut children = JoinSet::new();
            loop {
                tokio::select! {
                    _=&mut stopped=>break,
                    Some(result)=children.join_next(),if !children.is_empty()=>{result.unwrap();},
                    incoming=listener.accept()=>{
                        let (socket,_)=incoming.unwrap();let acceptor=acceptor.clone();let log=log_task.clone();
                        children.spawn(async move {
                            let mut peek=[0u8;1];
                            let Ok(Ok(1))=timeout(Duration::from_secs(3),socket.peek(&mut peek)).await else{return;};
                            if peek[0]==22 {
                                if let Ok(Ok(stream))=timeout(Duration::from_secs(3),acceptor.accept(socket)).await {
                                    serve(stream,true,port,log).await;
                                }
                            }else{serve(socket,false,port,log).await;}
                        });
                    }
                }
            }
            drop(listener);
            while let Some(result) = children.join_next().await {
                result.unwrap();
            }
        });
        Self {
            port,
            ca: ca.pem(),
            ca_der: ca.der().to_vec(),
            log,
            stop: Some(stop),
            task: Some(task),
        }
    }
    fn https(&self, path: &str) -> String {
        format!("https://localhost:{}{path}", self.port)
    }
    fn http(&self, path: &str) -> String {
        format!("http://localhost:{}{path}", self.port)
    }
    fn config(&self, url: String, hsts: bool, robots: bool) -> CrawlConfig {
        CrawlConfig {
            seed_urls: vec![url],
            tls: Some(TlsConfig {
                respect_hsts: hsts,
                trusted_certificates: vec![TrustedCertificate::Pem {
                    name: "local-ca.pem".into(),
                    data: self.ca.clone(),
                }],
                ..Default::default()
            }),
            obey_robots: robots,
            discover_sitemaps: false,
            crawl_links: false,
            concurrency: 1,
            delay_ms: 0,
            request_timeout_secs: 3,
            ..Default::default()
        }
    }
    fn observed(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
    async fn close(mut self) {
        self.stop.take().unwrap().send(()).unwrap();
        let mut task = self.task.take().unwrap();
        match timeout(Duration::from_secs(5), &mut task).await {
            Ok(result) => result.unwrap(),
            Err(_) => {
                task.abort();
                let _ = task.await;
                panic!("local TLS fixture did not drain");
            }
        }
    }
}
impl Drop for LocalTls {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
async fn serve<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    secure: bool,
    port: u16,
    log: Arc<Mutex<Vec<String>>>,
) {
    let result=timeout(Duration::from_secs(3),async {
        let mut bytes=Vec::new();
        while !bytes.ends_with(b"\r\n\r\n"){
            if bytes.len()>=8192{return;}
            let mut byte=[0u8;1];if stream.read_exact(&mut byte).await.is_err(){return;}bytes.push(byte[0]);
        }
        let request=String::from_utf8(bytes).unwrap();
        let path=request.lines().next().unwrap().split_whitespace().nth(1).unwrap().to_owned();
        log.lock().unwrap().push(format!("{}:{path}:secret={}",if secure{"tls"}else{"plain"},request.to_ascii_lowercase().contains("x-seed-secret:")));
        let (status,headers,body)=if !secure{
            if path=="/robots.txt"{("302 Found",format!("Location: https://localhost:{port}/robots.txt\r\n"),String::new())}
            else{("400 Bad Request",String::new(),"unexpected cleartext page".into())}
        }else if path=="/robots.txt"{
            ("200 OK","Strict-Transport-Security: max-age=3600\r\n".into(),"User-agent: *\nAllow: /\n".into())
        }else if path=="/redirect"{
            ("302 Found",format!("Strict-Transport-Security: max-age=3600\r\nLocation: http://localhost:{port}/next\r\n"),String::new())
        }else if path=="/learn"{
            ("200 OK","Strict-Transport-Security: max-age=3600\r\n".into(),format!("<html><title>Learn</title><a href=\"http://localhost:{port}/next\">next</a></html>"))
        }else if path=="/sitemap.xml"{
            ("200 OK","Content-Type: application/xml\r\n".into(),format!("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"><url><loc>https://localhost:{port}/next</loc></url></urlset>"))
        }else{("200 OK",String::new(),"<html><title>Trusted local fixture</title><body>ok</body></html>".into())};
        let content_type=if headers.contains("Content-Type:") { "" } else { "Content-Type: text/html\r\n" };
        let response=format!("HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\n{content_type}Connection: close\r\n\r\n{body}",body.len());
        let _=stream.write_all(response.as_bytes()).await;
        let _=stream.shutdown().await;
    }).await;
    assert!(result.is_ok(), "bounded local request timed out");
}
async fn crawl(config: CrawlConfig) -> crate::CrawlReport {
    let crawler = Crawler::new(config).unwrap();
    timeout(
        Duration::from_secs(12),
        crawler.run(None, Arc::new(AtomicBool::new(false))),
    )
    .await
    .unwrap()
    .unwrap()
}
#[tokio::test]
async fn tls_local_default_rejects_untrusted_ca_explicit_pem_and_der_succeed() {
    let fixture = LocalTls::start().await;
    let mut untrusted = fixture.config(fixture.https("/next"), false, false);
    untrusted.tls = None;
    let rejected = crawl(untrusted).await;
    assert!(rejected.pages[0].error.is_some());
    assert!(rejected.pages[0].status.is_none());
    let accepted = crawl(fixture.config(fixture.https("/next"), false, false)).await;
    assert_eq!(accepted.pages[0].final_status, Some(200));
    assert!(accepted.pages[0].error.is_none());
    let mut der_config = fixture.config(fixture.https("/next"), false, false);
    der_config.tls.as_mut().unwrap().trusted_certificates =
        vec![TrustedCertificate::from_file_bytes("local.der".into(), &fixture.ca_der).unwrap()];
    let accepted_der = crawl(der_config).await;
    assert_eq!(accepted_der.pages[0].final_status, Some(200));
    assert!(accepted_der.pages[0].error.is_none());
    fixture.close().await;
}
#[tokio::test]
async fn tls_local_trust_preserves_hostname_rejection() {
    let fixture = LocalTls::start().await;
    let bad = fixture.config(
        format!("https://127.0.0.1:{}/next", fixture.port),
        false,
        false,
    );
    let report = crawl(bad).await;
    assert!(report.pages[0].error.is_some());
    assert!(report.pages[0].status.is_none());
    assert!(fixture.observed().is_empty());
    fixture.close().await;
}
#[tokio::test]
async fn hsts_local_verified_document_upgrades_custom_port_without_cleartext_or_fake_status() {
    let fixture = LocalTls::start().await;
    let mut cfg = fixture.config(fixture.https("/learn"), true, false);
    cfg.crawl_links = true;
    cfg.max_pages = Some(2);
    let report = crawl(cfg).await;
    let page = report
        .pages
        .iter()
        .find(|p| p.url == fixture.http("/next"))
        .unwrap();
    assert_eq!(page.final_url, fixture.https("/next"));
    assert_eq!(page.status, Some(200));
    assert_eq!(page.final_status, Some(200));
    assert!(page.redirect_chain.is_empty());
    let hop = &page.tls.as_ref().unwrap().hsts_upgrades[0];
    assert_eq!(hop.policy.source_kind, HstsSourceKind::Document);
    assert_eq!(hop.synthetic_status, 307);
    assert!(!hop.network_request_made);
    assert!(hop.https_request_started);
    assert!(
        !fixture
            .observed()
            .iter()
            .any(|line| line.starts_with("plain:/next:"))
    );
    fixture.close().await;
}
#[tokio::test]
async fn hsts_local_robots_can_learn_before_first_page_and_does_not_forward_seed_credentials() {
    let fixture = LocalTls::start().await;
    let mut cfg = fixture.config(fixture.http("/next"), true, true);
    cfg.custom_headers
        .insert("X-Seed-Secret".into(), "only-http-origin".into());
    let report = crawl(cfg).await;
    let page = &report.pages[0];
    assert_eq!(page.final_status, Some(200));
    assert_eq!(page.url, fixture.http("/next"));
    assert_eq!(
        page.tls.as_ref().unwrap().hsts_upgrades[0]
            .policy
            .source_kind,
        HstsSourceKind::Robots
    );
    let observed = fixture.observed();
    assert!(
        observed
            .iter()
            .any(|line| line.starts_with("plain:/robots.txt:"))
    );
    assert!(!observed.iter().any(|line| line.starts_with("plain:/next:")));
    assert!(
        observed
            .iter()
            .filter(|line| line.starts_with("tls:"))
            .all(|line| line.ends_with("secret=false"))
    );
    fixture.close().await;
}
#[tokio::test]
async fn hsts_local_server_redirect_and_local_upgrade_keep_distinct_provenance() {
    let fixture = LocalTls::start().await;
    let report = crawl(fixture.config(fixture.https("/redirect"), true, false)).await;
    let page = &report.pages[0];
    assert_eq!(page.status, Some(302));
    assert_eq!(page.final_status, Some(200));
    assert_eq!(page.redirect_chain.len(), 1);
    assert_eq!(page.redirect_chain[0].status, 302);
    assert_eq!(
        page.tls.as_ref().unwrap().hsts_upgrades[0].network_redirect_index,
        1
    );
    assert!(
        !fixture
            .observed()
            .iter()
            .any(|line| line.starts_with("plain:/next:"))
    );
    fixture.close().await;
}
#[tokio::test]
async fn hsts_local_disabled_mode_keeps_actual_cleartext_behavior() {
    let fixture = LocalTls::start().await;
    let mut cfg = fixture.config(fixture.https("/learn"), false, false);
    cfg.crawl_links = true;
    cfg.max_pages = Some(2);
    let report = crawl(cfg).await;
    let page = report
        .pages
        .iter()
        .find(|p| p.url == fixture.http("/next"))
        .unwrap();
    assert!(page.tls.is_none());
    assert_eq!(page.final_status, Some(400));
    assert!(
        fixture
            .observed()
            .iter()
            .any(|line| line.starts_with("plain:/next:"))
    );
    fixture.close().await;
}
fn with_known_hsts(fixture: &LocalTls, config: CrawlConfig) -> (tempfile::TempDir, Crawler) {
    let crawler = Crawler::new(config).unwrap();
    let normalized = crawler.effective_config();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crawl.sqlite");
    {
        let store = Arc::new(crate::CrawlStore::open(&path).unwrap());
        store.prepare(normalized).unwrap();
        let generation = store.begin_run("prepare-known-hsts").unwrap();
        let cache = HstsCache::new(normalized.tls.as_ref().unwrap()).unwrap();
        cache.attach(store, generation).unwrap();
        // Deliberately prepared policy for admission tests. Actual verified TLS
        // learning is independently exercised by the document and robots tests.
        cache
            .observe(
                &Url::parse(&fixture.https("/learn")).unwrap(),
                &{
                    let mut h = HeaderMap::new();
                    h.insert(
                        "strict-transport-security",
                        reqwest::header::HeaderValue::from_static("max-age=3600"),
                    );
                    h
                },
                HstsSourceKind::Document,
            )
            .unwrap();
    }
    (dir, crawler.with_store(path).unwrap())
}
#[tokio::test]
async fn hsts_local_upgraded_target_rechecks_static_admission_before_send() {
    let fixture = LocalTls::start().await;
    let mut cfg = fixture.config(fixture.http("/next"), true, false);
    cfg.crawl_admission_limits = Some(crate::CrawlAdmissionLimits {
        max_fetch_url_utf8_bytes: Some(fixture.http("/next").len()),
        ..Default::default()
    });
    let (_dir, crawler) = with_known_hsts(&fixture, cfg);
    let report = timeout(
        Duration::from_secs(12),
        crawler.run(None, Arc::new(AtomicBool::new(false))),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        report.pages[0]
            .error
            .as_deref()
            .unwrap()
            .contains("Static crawl admission")
    );
    assert!(!report.pages[0].tls.as_ref().unwrap().hsts_upgrades[0].https_request_started);
    assert!(
        fixture.observed().is_empty(),
        "neither cleartext fallback nor over-limit HTTPS request"
    );
    fixture.close().await;
}
#[tokio::test]
async fn hsts_local_upgraded_target_rechecks_native_policy_before_send() {
    let fixture = LocalTls::start().await;
    let mut cfg = fixture.config(fixture.http("/next"), true, false);
    cfg.crawl_policy = Some(crate::CrawlPolicyConfig {
        scope: crate::CrawlScopeConfig {
            roots: vec![crate::CrawlScopeRoot {
                origin: format!("http://localhost:{}", fixture.port),
                subdomains: false,
                folders: vec!["/".into()],
                exact_paths: vec![],
            }],
            ..Default::default()
        },
        ..Default::default()
    });
    let (_dir, crawler) = with_known_hsts(&fixture, cfg);
    let report = timeout(
        Duration::from_secs(12),
        crawler.run(None, Arc::new(AtomicBool::new(false))),
    )
    .await
    .unwrap()
    .unwrap();
    let page = &report.pages[0];
    assert!(
        page.error
            .as_deref()
            .unwrap()
            .contains("HSTS HTTPS target denied")
    );
    assert_eq!(
        page.tls.as_ref().unwrap().hsts_upgrades[0]
            .crawl_policy
            .as_ref()
            .unwrap()
            .decision,
        crate::CrawlPolicyDecisionKind::Deny
    );
    assert!(
        fixture.observed().is_empty(),
        "an HSTS policy is not a native fetch grant"
    );
    fixture.close().await;
}
