use super::*;
use std::{collections::VecDeque,sync::{Mutex,atomic::{AtomicUsize,Ordering}}};
use tokio::io::{AsyncReadExt,AsyncWriteExt};
fn options()->ListImportOptions{ListImportOptions{format:crate::ListImportFormat::Txt,limits:Default::default()}}
fn plan(input:&str)->ApiEnrichmentPlan{prepare_api_enrichment(input.as_bytes(),&options(),&ApiEnrichmentOptions{retries:0,..Default::default()}).unwrap()}
fn good()->Vec<u8>{br#"{"lighthouseResult":{"requestedUrl":"https://example.test/a","finalUrl":"https://elsewhere.test/final","fetchTime":"2026-10-16T00:00:00Z","lighthouseVersion":"13.0","categories":{"performance":{"score":0.75}},"audits":{"largest-contentful-paint":{"numericValue":1234.5}}}}"#.to_vec()}
struct Queue{values:Mutex<VecDeque<PageSpeedReply>>,seen:Mutex<Vec<String>>}
impl Queue{fn new(values:Vec<PageSpeedReply>)->Self{Self{values:Mutex::new(values.into()),seen:Default::default()}}}
impl PageSpeedTransport for Queue{fn execute<'a>(&'a self,r:PageSpeedRequest<'a>,_:ApiCancellation)->BoxFuture<'a,std::result::Result<PageSpeedReply,ApiFailureCode>>{Box::pin(async move{self.seen.lock().unwrap().push(r.requested_url.into());self.values.lock().unwrap().pop_front().ok_or(ApiFailureCode::Network)})}}
fn reply(status:u16,bytes:Vec<u8>)->PageSpeedReply{PageSpeedReply{status,bytes,retry_after_secs:Some(0)}}

#[test]
fn api_only_import_provenance_deduplicates_execution_not_occurrences_and_refuses_url_credentials(){
    let input="https://example.test/a#one\nhttps://example.test/a#two\nnot a URL\nhttps://example.test/b\n";
    let p=plan(input);assert_eq!(p.unique_urls,["https://example.test/a","https://example.test/b"]);assert_eq!(p.input.rows.len(),4);assert_eq!(p.input.rows[0].raw_value,"https://example.test/a#one");assert_eq!(p.input.rows[1].raw_value,"https://example.test/a#two");assert!(p.input.rows[2].normalized_url.is_none());assert_eq!(p.input.input_sha256,api_sha(input.as_bytes()));assert_eq!(p.input.website_requests,0);
    assert!(prepare_api_enrichment(b"https://user:password@example.test/",&options(),&Default::default()).is_err());
    let mut o=ApiEnrichmentOptions::default();o.version=2;assert!(o.validate().is_err());o.version=1;o.retries=3;assert!(o.validate().is_err());
    assert!(serde_json::from_value::<ApiEnrichmentOptions>(serde_json::json!({"endpoint":"http://127.0.0.1/"})).is_err());
}
#[tokio::test]
async fn api_only_typed_provider_metrics_are_separate_from_site_facts_and_absence_is_not_zero(){
    let q=Queue::new(vec![reply(200,good()),reply(200,b"{}".to_vec())]);let report=run_api_enrichment(plan("https://example.test/a\nhttps://example.test/b"),&q,None,Default::default()).await.unwrap();
    assert_eq!(report.provider_request_attempts,2);assert_eq!(report.application_website_requests,0);assert_eq!(report.observations[0].api_http_status,Some(200));let m=report.observations[0].metrics.as_ref().unwrap();assert_eq!(m.performance_score,Some(0.75));assert_eq!(m.largest_contentful_paint_ms,Some(1234.5));assert_eq!(m.first_contentful_paint_ms,None);assert_eq!(m.provider_final_url.as_deref(),Some("https://elsewhere.test/final"));assert_eq!(report.observations[1].outcome,ApiOutcome::Unavailable);assert!(report.observations[1].metrics.is_none());validate_api_report(&report).unwrap();
    let mut out=Vec::new();export_api_enrichment_csv(&report,&mut out).unwrap();let mut csv=csv::Reader::from_reader(out.as_slice());assert_eq!(csv.headers().unwrap().get(7),Some("api_http_status"));assert_eq!(csv.records().count(),2);
}
#[tokio::test]
async fn api_only_transient_retry_call_budget_and_authentication_are_bounded(){
    let mut p=plan("https://example.test/a\nhttps://example.test/b");p.options.retries=2;p.options.max_provider_calls=3;p.fingerprint=api_sha(&bounded_api_json(&(API_ENRICHMENT_ENGINE,&p.input,&p.options,&p.unique_urls),API_REPORT_BYTES).unwrap());
    let q=Queue::new(vec![reply(429,b"{}".to_vec()),reply(200,good()),reply(429,b"{}".to_vec())]);let r=run_api_enrichment(p,&q,None,Default::default()).await.unwrap();assert_eq!(r.provider_request_attempts,3);assert_eq!(q.seen.lock().unwrap().len(),3);assert_eq!(r.observations[0].outcome,ApiOutcome::Success);assert_eq!(r.observations[1].failure,Some(ApiFailureCode::CallBudget));assert_eq!(r.observations[1].outcome,ApiOutcome::BudgetExceeded);
    let q=Queue::new(vec![reply(401,b"do not persist provider error or credentials".to_vec())]);let r=run_api_enrichment(plan("https://example.test/a"),&q,None,Default::default()).await.unwrap();assert_eq!(r.provider_request_attempts,1);assert_eq!(r.observations[0].failure,Some(ApiFailureCode::Authentication));assert!(!String::from_utf8(bounded_api_json(&r,API_REPORT_BYTES).unwrap()).unwrap().contains("do not persist"));
}
#[tokio::test]
async fn api_only_cancel_drops_inflight_transport_and_never_starts_remaining_urls(){
    struct Held{entered:tokio::sync::Notify,dropped:Arc<AtomicUsize>}
    impl PageSpeedTransport for Held{fn execute<'a>(&'a self,_:PageSpeedRequest<'a>,_:ApiCancellation)->BoxFuture<'a,std::result::Result<PageSpeedReply,ApiFailureCode>>{Box::pin(async move{struct DropFlag(Arc<AtomicUsize>);impl Drop for DropFlag{fn drop(&mut self){self.0.fetch_add(1,Ordering::SeqCst);}}let _flag=DropFlag(self.dropped.clone());self.entered.notify_one();std::future::pending().await})}}
    let h=Arc::new(Held{entered:Default::default(),dropped:Default::default()});let cancel=ApiCancellation::default();let c=cancel.clone();let worker=h.clone();let task=tokio::spawn(async move{run_api_enrichment(plan("https://example.test/a\nhttps://example.test/b"),worker.as_ref(),None,c).await});tokio::time::timeout(Duration::from_secs(2),h.entered.notified()).await.unwrap();cancel.cancel();let r=tokio::time::timeout(Duration::from_secs(2),task).await.unwrap().unwrap().unwrap();assert_eq!(h.dropped.load(Ordering::SeqCst),1);assert_eq!(r.provider_request_attempts,1);assert_eq!(r.outcome,"cancelled");assert!(r.observations.iter().all(|o|o.outcome==ApiOutcome::Cancelled));
    let h=Held{entered:Default::default(),dropped:Default::default()};let mut p=plan("https://example.test/a\nhttps://example.test/b");p.options.max_job_ms=100;p.fingerprint=api_sha(&bounded_api_json(&(API_ENRICHMENT_ENGINE,&p.input,&p.options,&p.unique_urls),API_REPORT_BYTES).unwrap());let r=tokio::time::timeout(Duration::from_secs(2),run_api_enrichment(p,&h,None,Default::default())).await.unwrap().unwrap();assert_eq!(h.dropped.load(Ordering::SeqCst),1);assert_eq!(r.provider_request_attempts,1);assert_eq!(r.outcome,"partial");assert!(r.observations.iter().all(|o|o.failure==Some(ApiFailureCode::JobDeadline)));
}
#[tokio::test]
async fn api_only_hash_plan_response_caps_and_invalid_metrics_refuse_without_silent_coercion(){
    let mut p=plan("https://example.test/a");p.fingerprint.push('0');let q=Queue::new(vec![]);assert!(run_api_enrichment(p,&q,None,Default::default()).await.is_err());assert!(q.seen.lock().unwrap().is_empty());
    let q=Queue::new(vec![]);assert!(run_api_enrichment(plan("https://example.test/a"),&q,Some("test-not-an-environment-key"),Default::default()).await.is_err());assert!(q.seen.lock().unwrap().is_empty());
    let mut p=plan("https://example.test/a");p.options.max_response_bytes=2;p.fingerprint=api_sha(&bounded_api_json(&(API_ENRICHMENT_ENGINE,&p.input,&p.options,&p.unique_urls),API_REPORT_BYTES).unwrap());let q=Queue::new(vec![reply(200,good())]);let r=run_api_enrichment(p,&q,None,Default::default()).await.unwrap();assert_eq!(r.observations[0].failure,Some(ApiFailureCode::ResponseBudget));
    let q=Queue::new(vec![reply(200,br#"{"lighthouseResult":{"categories":{"performance":{"score":2}}}}"#.to_vec())]);let r=run_api_enrichment(plan("https://example.test/a"),&q,None,Default::default()).await.unwrap();assert_eq!(r.observations[0].failure,Some(ApiFailureCode::InvalidSchema));assert!(r.observations[0].metrics.is_none());assert!(bounded_api_json(&r,1).is_err());
}
#[test]
fn api_only_production_request_has_fixed_https_origin_and_secret_is_not_serialized(){
    let http=PageSpeedHttp::new().unwrap();let r=PageSpeedRequest{requested_url:"http://127.0.0.1:12345/path?x=1&y=2",strategy:PageSpeedStrategy::Desktop,timeout_ms:1000,max_response_bytes:1024,key:Some("test-only-secret")};let req=http.request(&r).build().unwrap();assert_eq!(req.url().scheme(),"https");assert_eq!(req.url().host_str(),Some("www.googleapis.com"));assert_eq!(req.url().path(),"/pagespeedonline/v5/runPagespeed");assert_eq!(req.url().query_pairs().find(|(k,_)|k=="url").unwrap().1,r.requested_url);assert_eq!(req.url().query_pairs().find(|(k,_)|k=="key").unwrap().1,"test-only-secret");let p=plan("https://example.test/a");assert!(!String::from_utf8(bounded_api_json(&p,API_REPORT_BYTES).unwrap()).unwrap().contains("test-only-secret"));assert_eq!(safe_csv("  =1+1"),"'  =1+1");assert_eq!(safe_csv("https://example.test/"),"https://example.test/");
}
#[tokio::test]
async fn api_only_real_local_mock_api_response_does_not_fetch_the_supplied_website_or_follow_redirect(){
    // Both loopback endpoints are tests only. No product endpoint override is introduced.
    struct HttpMock{endpoint:String,client:reqwest::Client}
    impl PageSpeedTransport for HttpMock{fn execute<'a>(&'a self,r:PageSpeedRequest<'a>,_:ApiCancellation)->BoxFuture<'a,std::result::Result<PageSpeedReply,ApiFailureCode>>{Box::pin(async move{let response=self.client.get(&self.endpoint).query(&[("url",r.requested_url)]).send().await.map_err(|_|ApiFailureCode::Network)?;let status=response.status().as_u16();let bytes=response.bytes().await.map_err(|_|ApiFailureCode::Network)?.to_vec();Ok(reply(status,bytes))})}}
    let trap=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();let website=format!("http://{}/actual-site",trap.local_addr().unwrap());let (stop_tx,mut stop_rx)=tokio::sync::oneshot::channel();let hits=Arc::new(AtomicUsize::new(0));let seen=hits.clone();let trap_task=tokio::spawn(async move{loop{tokio::select!{_= &mut stop_rx=>break,r=trap.accept()=>{let (_stream,_)=r.unwrap();seen.fetch_add(1,Ordering::SeqCst);}}}});
    for status in [200,302]{let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();let endpoint=format!("http://{}/mock-psi",listener.local_addr().unwrap());let target=website.clone();let peer=tokio::spawn(async move{let (mut stream,_)=listener.accept().await.unwrap();let mut request=Vec::new();loop{let mut b=[0;1024];let n=stream.read(&mut b).await.unwrap();assert!(n>0);request.extend_from_slice(&b[..n]);assert!(request.len()<16384);if request.windows(4).any(|x|x==b"\r\n\r\n"){break;}}let text=String::from_utf8(request).unwrap();assert!(text.starts_with("GET /mock-psi?url="));let body=if status==200{good()}else{Vec::new()};let head=format!("HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nLocation: {target}\r\nConnection: close\r\n\r\n",body.len());stream.write_all(head.as_bytes()).await.unwrap();stream.write_all(&body).await.unwrap();stream.shutdown().await.unwrap();});
        let mock=HttpMock{endpoint,client:reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).build().unwrap()};let r=tokio::time::timeout(Duration::from_secs(3),run_api_enrichment(plan(&website),&mock,None,Default::default())).await.unwrap().unwrap();peer.await.unwrap();assert_eq!(r.provider_request_attempts,1);assert_eq!(r.observations[0].api_http_status,Some(status));assert_eq!(r.application_website_requests,0);if status==302{assert_eq!(r.observations[0].failure,Some(ApiFailureCode::ProviderHttp));}}
    stop_tx.send(()).unwrap();trap_task.await.unwrap();assert_eq!(hits.load(Ordering::SeqCst),0);
}

#[tokio::test]
async fn api_only_http200_lighthouse_runtime_errors_and_missing_measurements_are_not_success(){
    for (body,outcome,failure) in [
        (br#"{"lighthouseResult":{"runtimeError":{"code":"NO_FCP","message":"test-only-secret-must-not-survive"},"categories":{"performance":{"score":0.9}}}}"#.to_vec(),ApiOutcome::Failed,Some(ApiFailureCode::LighthouseRuntime)),
        (br#"{"lighthouseResult":{"requestedUrl":"https://example.test/a","fetchTime":"2026-10-16T00:00:00Z"}}"#.to_vec(),ApiOutcome::Unavailable,None),
        (br#"{"lighthouseResult":{}}"#.to_vec(),ApiOutcome::Unavailable,None),
    ]{let q=Queue::new(vec![reply(200,body)]);let r=run_api_enrichment(plan("https://example.test/a"),&q,None,Default::default()).await.unwrap();assert_eq!(r.observations[0].api_http_status,Some(200));assert_eq!(r.observations[0].outcome,outcome);assert_eq!(r.observations[0].failure,failure);assert!(r.observations[0].metrics.is_none());if outcome==ApiOutcome::Failed{assert_eq!(r.outcome,"partial");}let raw=String::from_utf8(bounded_api_json(&r,API_REPORT_BYTES).unwrap()).unwrap();assert!(!raw.contains("test-only-secret-must-not-survive"));assert!(!raw.contains("NO_FCP"));}
}
#[tokio::test]
async fn api_only_unicode_normalized_url_exact8192_and_plus1_refuse_before_attempt_and_revalidate_legacy_plan(){
    let prefix="https://example.test/";let raw=format!("{prefix}{}{}","é".repeat(1000),"a".repeat(8192-prefix.len()-6000));assert!(raw.len()<8192);let exact=plan(&raw);assert_eq!(exact.unique_urls[0].len(),8192);let over=format!("{raw}a");assert!(prepare_api_enrichment(over.as_bytes(),&options(),&Default::default()).is_err());
    // A plan produced before the new prepare guard still faces the original run-time authority guard.
    let mut legacy=exact;legacy.input=import_url_list(over.as_bytes(),&options()).unwrap();legacy.unique_urls=vec![legacy.input.rows[0].normalized_url.clone().unwrap()];assert_eq!(legacy.unique_urls[0].len(),8193);legacy.fingerprint=api_sha(&bounded_api_json(&(API_ENRICHMENT_ENGINE,&legacy.input,&legacy.options,&legacy.unique_urls),API_REPORT_BYTES).unwrap());let q=Queue::new(vec![]);assert!(run_api_enrichment(legacy,&q,None,Default::default()).await.is_err());assert!(q.seen.lock().unwrap().is_empty());
}
