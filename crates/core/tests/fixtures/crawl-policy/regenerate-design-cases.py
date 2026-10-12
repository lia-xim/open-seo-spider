"""Produce independently authored DESIGN_ONLY acceptance data; no engine or network."""
import copy
import json
from pathlib import Path

ROOT = Path(__file__).parent
BASE = {
    "engine": "open-seo-spider/crawl-policy-v1",
    "scope": {"roots": [{"origin": "https://example.test", "subdomains": False,
                           "folders": ["/shop/"], "exact_paths": []}],
              "outside_folder": "deny", "external": "deny"},
    "query": {"remove_keys": [], "key_match": "percent_decoded_utf8", "order": "preserve"},
    "rewrites": [], "include_patterns": [], "exclude_patterns": [],
    "filter_identity": "fetch", "initial_inputs": {"rewrite": "preserve", "filters": "exempt"},
}
CASES = []

def add(name, raw, expected, policy=None, source_kind="anchor", **extra):
    CASES.append({"id": f"cp-{len(CASES)+1:03}", "name": name, "status": "NOT_PROVEN",
                  "input": {"raw_value": raw, "base_url": "https://example.test/shop/index",
                            "source_kind": source_kind, **extra},
                  "policy": copy.deepcopy(policy or BASE), "expected": expected})

def policy(**changes):
    out = copy.deepcopy(BASE)
    out.update(changes)
    return out

def fetch(url, **extra):
    return {"decision": "fetch_and_traverse", "fetch_candidate_url": url, **extra}

def deny(reason, **extra):
    return {"decision": "deny", "reason_code": reason, **extra}

def literal(rule_id, pattern, value):
    return {"id": rule_id, "pattern": pattern, "replacement": {"kind": "literal", "text": value}}

add("Relative fragment retains observation while fetch omits fragment", "next?b=2&a=1#part",
    fetch("https://example.test/shop/next?b=2&a=1", fragment="part",
          resolved_discovered_url="https://example.test/shop/next?b=2&a=1#part"))
add("Directory itself included", "/shop/", fetch("https://example.test/shop/"))
add("Prefix sibling is outside folder", "/shopping/item", deny("outside_folder"))
add("No inferred slash for folder landing", "/shop", deny("outside_folder"))
add("Explicit exact path permits distinct unslashed landing", "/shop",
    fetch("https://example.test/shop"), policy(scope={**BASE["scope"], "roots": [{**BASE["scope"]["roots"][0], "exact_paths": ["/shop"]}]}))
add("Folder path case is significant", "/Shop/item", deny("outside_folder"))
add("Encoded separator does not form configured folder boundary", "/shop%2Fitem", deny("outside_folder"))
add("Actual URL resolution removes parent segment before scope", "../outside", deny("outside_folder"))
add("Outside folder check has no onward traversal", "/outside",
    {"decision": "check_only", "fetch_candidate_url": "https://example.test/outside", "traverse": False},
    policy(scope={**BASE["scope"], "outside_folder": "check_once"}))
add("Scheme switch requires explicit origin", "http://example.test/shop/a", deny("outside_origin"))
add("Different explicit port requires origin", "https://example.test:8443/shop/a", deny("outside_origin"))
add("Subdomain default is exact", "https://a.example.test/shop/a", deny("outside_origin"))
subdomains = policy(scope={**BASE["scope"], "roots": [{**BASE["scope"]["roots"][0], "subdomains": True}]})
add("Explicit base host permits strict descendant", "https://a.example.test/shop/a", fetch("https://a.example.test/shop/a"), subdomains)
add("Suffix lookalike is never a subdomain", "https://badexample.test/shop/a", deny("outside_origin"), subdomains)
add("External suffix after root is not a subdomain", "https://example.test.attacker.test/shop/a", deny("outside_origin"), subdomains)
add("Credentials rejected before request", "https://user:secret@example.test/shop/a", deny("unsafe_url"))
add("Unsupported protocol does not become HTTP", "javascript:goto('/shop/a')", deny("unsafe_url"))
add("Raw controls rejected without URL parser repair", "https://example.test/shop/a\t", deny("unsafe_url"))
add("Default query bytes and duplicates unchanged", "/shop/a?b=%20&a=+&a=%2f&&flag&empty=&",
    fetch("https://example.test/shop/a?b=%20&a=+&a=%2f&&flag&empty=&"))
remove = policy(query={**BASE["query"], "remove_keys": ["utm_source"]})
add("Named removal deletes every selected occurrence", "/shop/a?utm_source=x&keep=%20&utm_source=y&keep=+",
    fetch("https://example.test/shop/a?keep=%20&keep=+"), remove)
add("Names are exact and case-sensitive", "/shop/a?UTM_source=x&utm_source_extra=y",
    fetch("https://example.test/shop/a?UTM_source=x&utm_source_extra=y"), remove)
add("Percent-decoded UTF8 comparison does not reencode retained tokens", "/shop/a?utm%5Fsource=x&a=%2f",
    fetch("https://example.test/shop/a?a=%2f"), remove)
add("Encoded-key profile does not decode for comparison", "/shop/a?utm%5Fsource=x",
    fetch("https://example.test/shop/a?utm%5Fsource=x"), policy(query={**BASE["query"], "remove_keys": ["utm_source"], "key_match": "encoded_bytes"}))
add("Invalid encoded key remains unknown and unchanged", "/shop/a?utm%ZZsource=x",
    fetch("https://example.test/shop/a?utm%ZZsource=x", query_key_warning=True), remove)
add("Plus is literal in key matching", "/shop/a?a+b=x&a%20b=y",
    fetch("https://example.test/shop/a?a+b=x"), policy(query={**BASE["query"], "remove_keys": ["a b"]}))
add("All real query tokens removed deletes query", "/shop/a?utm_source=x",
    fetch("https://example.test/shop/a"), remove)
add("An actually retained blank token retains empty query marker", "/shop/a?utm_source=x&",
    fetch("https://example.test/shop/a?"), remove)
add("Stable raw-key ordering keeps repeated-key order and values", "/shop/a?b=1&a=%20&a=+&b=2",
    fetch("https://example.test/shop/a?a=%20&a=+&b=1&b=2"), policy(query={**BASE["query"], "order": "stable_encoded_key"}))
rewrite = policy(rewrites=[literal("legacy", "/shop/old", "/shop/new")])
add("Rewrite retains raw and discovered identity", "/shop/old", fetch("https://example.test/shop/new", normalized_discovered_url="https://example.test/shop/old", changed_steps=1), rewrite)
add("First match profile changes one substring per rule", "/shop/old/old", fetch("https://example.test/shop/new/old"), policy(rewrites=[literal("first", "old", "new")]))
add("Literal dollar is not a capture", "/shop/old", fetch("https://example.test/shop/$1"), policy(rewrites=[literal("literal", "old", "$1")]))
add("Explicit capture template uses validated named groups", "/shop/old/123", fetch("https://example.test/shop/new/123"), policy(rewrites=[{"id":"capture", "pattern":r"/old/(?P<id>[0-9]+)$", "replacement":{"kind":"captures", "template":"/new/${id}"}}]))
add("Rules run once in saved order", "/shop/old", fetch("https://example.test/shop/final"), policy(rewrites=[literal("one","old","new"),literal("two","new","final")]))
add("Revisited changed identity is explicit cycle", "/shop/old", deny("rewrite_cycle"), policy(rewrites=[literal("one","old","new"),literal("two","new","old")]))
add("No-op replacement is not a cycle", "/shop/old", fetch("https://example.test/shop/old"), policy(rewrites=[literal("noop","old","old")]))
add("Unsafe rewrite cannot reach network", "/shop/a", deny("unsafe_rewrite"), policy(rewrites=[literal("bad",r"^https://", "javascript:")]))
add("Credential-bearing rewrite rejected", "/shop/a", deny("unsafe_rewrite"), policy(rewrites=[literal("bad",r"https://", "https://user:secret@")]))
add("Rewrite does not authorize foreign origin", "/shop/a", deny("outside_origin"), policy(rewrites=[literal("host", "example.test", "other.test")]))
add("Fragment-bearing rewrite is explicit error", "/shop/a", deny("unsafe_rewrite"), policy(rewrites=[literal("fragment",r"$", "#new")]))
add("Initial seeds preserved by default", "https://example.test/shop/old", fetch("https://example.test/shop/old"), rewrite, source_kind="seed")
add("Initial rewrite opt-in is explicit", "https://example.test/shop/old", fetch("https://example.test/shop/new"), policy(rewrites=rewrite["rewrites"], initial_inputs={"rewrite":"apply","filters":"exempt"}), source_kind="list_page")
add("Excludes take precedence over includes", "/shop/denied", deny("excluded", matched_rule_ids=["include-1", "exclude-1"]), policy(include_patterns=["/shop/"],exclude_patterns=["denied"]))
add("Filter fetch identity after rewrite", "/shop/old", deny("excluded"), policy(rewrites=rewrite["rewrites"],exclude_patterns=["/new"]))
add("Filter original identity is separately selectable", "/shop/old", fetch("https://example.test/shop/new"), policy(rewrites=rewrite["rewrites"],exclude_patterns=["/new"],filter_identity="discovered"))
add("Initial filter exemption preserves literal operator list", "https://example.test/shop/denied", fetch("https://example.test/shop/denied"), policy(exclude_patterns=["denied"]), source_kind="list_page")
add("Initial filtering opt-in denies seed visibly", "https://example.test/shop/denied", deny("excluded"), policy(exclude_patterns=["denied"],initial_inputs={"rewrite":"preserve","filters":"apply"}), source_kind="seed")
add("Exclude means no guesses about downstream links", "/shop/denied", deny("excluded", discovered_descendants=[]), policy(exclude_patterns=["denied"]))

SCENARIOS = [
    {"id":"cp-integration-collision", "status":"NOT_PROVEN", "inputs":["/shop/a?utm_source=one","/shop/a?utm_source=two"],"policy":remove,
     "required_observations":["two immutable candidate occurrences","one fetch identity and one HTTP request","neither original tracking URL requested","both aliases survive SQLite/JSON/CSV/project export and resume"]},
    {"id":"cp-integration-robots-fetch", "status":"NOT_PROVEN", "required_observations":["query/removal/regex transforms discovered path","shared matcher checks actual transformed origin/path/query","blocked transformed target has zero document requests","no fabricated matched-rule trace"]},
    {"id":"cp-integration-redirect-stop", "status":"NOT_PROVEN", "required_observations":["initial302 response retained","off-folder/filter-disallowed Location remains observed unrequested target","allowed target checks its actual robots origin/path","redirect locations never run discovery rewriting"]},
    {"id":"cp-integration-credentials", "status":"NOT_PROVEN", "required_observations":["authorized scope expansion/rewritten host gets no original origin custom credentials","robots gets no custom credentials","renderer subresources and navigation have independent isolation proof"]},
    {"id":"cp-integration-graph", "status":"NOT_PROVEN", "required_observations":["raw DOM link and graph target retain actual discovered URL","rewritten fetch alias explicitly labeled","resource references are not fabricated navigation anchors"]},
    {"id":"cp-integration-source-kinds", "status":"NOT_PROVEN", "required_observations":["raw/rendered/PDF/header/sitemap location candidates share actual typed policy","explicit sitemap_document retains kind and is not rewritten or promoted to page seed","no regex scraping or unobserved navigation discovery"]},
    {"id":"cp-integration-preview-zero-network", "status":"NOT_PROVEN", "required_observations":["preview processes ordered duplicate/invalid inputs offline","local supplied URL and sitemap counters stay zero","bounded client/server input validation","stale preview cannot overwrite edited inputs"]},
    {"id":"cp-integration-bounds", "status":"NOT_PROVEN", "required_observations":["positive input/rule/compiledpattern/capture/query/output/ledger guards","all config validation before native/robots HTTP","limit failure explicit without silently omitted decision rows"]},
    {"id":"cp-config-errors", "status":"NOT_PROVEN", "inputs":["unknown capture name","unclosed capture placeholder","unescaped replacement dollar syntax","lookbehind/pattern backreference","duplicate rule id","ambiguous folder scope","oversize pattern or URL","zero limit"],"required_observations":["configuration compile error before every website request"]},
]

payload={"schema":1,"status":"DESIGN_ONLY","vendor_parity":"NOT_PROVEN","engine":"open-seo-spider/crawl-policy-v1","test_count":len(CASES),"executed_tests":0,"candidate_cases":CASES,"integration_scenarios":SCENARIOS,"provenance":"Own authored expectations from independent contract; not original engine membership or runtime proof."}
(ROOT / "authored-policy-cases.json").write_text(json.dumps(payload,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
print(f"Saved {len(CASES)} unexecuted candidate contracts and {len(SCENARIOS)} integration scenarios")
