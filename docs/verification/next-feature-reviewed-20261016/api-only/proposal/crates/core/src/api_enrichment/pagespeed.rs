use super::*;
use futures::StreamExt;
/// Fixed HTTPS origin only. There is deliberately no runtime endpoint override.
pub struct PageSpeedHttp { client:reqwest::Client }
impl PageSpeedHttp {
    pub fn new()->Result<Self>{Ok(Self{client:reqwest::Client::builder().https_only(true).no_proxy().redirect(reqwest::redirect::Policy::none()).build()?})}
    pub(super) fn request(&self,r:&PageSpeedRequest<'_>)->reqwest::RequestBuilder{let request=self.client.get(PAGESPEED_ENDPOINT).query(&[("url",r.requested_url),("strategy",r.strategy.as_str()),("category","performance")]).timeout(Duration::from_millis(r.timeout_ms));if let Some(key)=r.key{request.query(&[("key",key)])}else{request}}
}
impl PageSpeedTransport for PageSpeedHttp {
    fn execute<'a>(&'a self,r:PageSpeedRequest<'a>,cancel:ApiCancellation)->BoxFuture<'a,std::result::Result<PageSpeedReply,ApiFailureCode>>{
        Box::pin(async move{
            let request=self.request(&r);
            let response=tokio::select!{biased;_=cancel.cancelled()=>return Err(ApiFailureCode::Cancelled),r=request.send()=>r.map_err(|e|if e.is_timeout(){ApiFailureCode::Timeout}else{ApiFailureCode::Network})?};
            let status=response.status().as_u16();let retry_after_secs=response.headers().get(reqwest::header::RETRY_AFTER).and_then(|v|v.to_str().ok()).and_then(|v|v.parse::<u64>().ok()).map(|v|v.min(30));
            if response.content_length().is_some_and(|n|usize::try_from(n).ok().is_none_or(|n|n>r.max_response_bytes)){return Err(ApiFailureCode::ResponseBudget);}
            let mut stream=response.bytes_stream();let mut bytes=Vec::new();
            loop{let next=tokio::select!{biased;_=cancel.cancelled()=>return Err(ApiFailureCode::Cancelled),n=stream.next()=>n};let Some(chunk)=next else{break};let chunk=chunk.map_err(|_|ApiFailureCode::Network)?;if bytes.len().checked_add(chunk.len()).is_none_or(|n|n>r.max_response_bytes){return Err(ApiFailureCode::ResponseBudget);}bytes.extend_from_slice(&chunk);}
            Ok(PageSpeedReply{status,bytes,retry_after_secs})
        })
    }
}
