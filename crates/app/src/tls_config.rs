pub(crate) fn schema() -> serde_json::Value {
    serde_json::json!({
        "type":["object","null"],"additionalProperties":false,"required":["version"],
        "description":"Crawl-scoped public trust certificates and optional learned HSTS; native fetches/robots only. Absent/null/inactive preserves legacy configuration. TLS certificate and hostname verification stays enabled. Active TLS with the external renderer is refused.",
        "properties":{
            "version":{"type":"integer","const":1},
            "respect_hsts":{"type":"boolean","default":false},
            "trusted_certificates":{"type":"array","maxItems":8,"default":[],"items":{
                "type":"object","additionalProperties":false,"required":["format","name","data"],
                "properties":{"format":{"type":"string","enum":["pem","der"]},"name":{"type":"string","minLength":1,"maxLength":256},"data":{"type":"string","minLength":1,"maxLength":131072}},
                "description":"PEM CERTIFICATE blocks or lowercase hex DER; at most65536 decoded bytes per file/262144 total/32 certificates. Public certificates only; no private keys or persisted filesystem paths."
            }}
        }
    })
}
