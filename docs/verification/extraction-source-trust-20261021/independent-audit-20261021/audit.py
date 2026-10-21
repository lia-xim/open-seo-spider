"""Read-only original/publication audit; writes only its own ignored receipt."""
from pathlib import Path, PurePosixPath
import csv, gzip, hashlib, io, json, re, traceback
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
sha = lambda b: hashlib.sha256(b).hexdigest()
proof = {"kind": "independent_read_only_extraction_publication_audit", "result": "FAIL", "created_at": datetime.now(timezone.utc).isoformat(), "packages": []}
observed = {}

def regular(p):
    assert p.is_file() and not p.is_symlink(), str(p)
    return p

def contained(anchor, name):
    assert isinstance(name, str) and name and "\\" not in name and ":" not in name
    parts = PurePosixPath(name).parts
    assert not PurePosixPath(name).is_absolute() and all(x not in ("", ".", "..") for x in parts)
    p = anchor.joinpath(*parts)
    assert p.resolve().is_relative_to(anchor.resolve()), str(p)
    q = p
    while q != anchor:
        assert not q.is_symlink(), str(q)
        q = q.parent
    return regular(p)

def read(p):
    b = regular(p).read_bytes()
    if p in observed:
        assert sha(b) == observed[p], "Changed during review: " + str(p)
    else:
        observed[p] = sha(b)
    return b

def js(name):
    return json.loads(read(ROOT / name).decode("utf-8-sig"))

def pin(row, absolute=False):
    p = Path(row.get("file", row.get("path")))
    if not absolute or not p.is_absolute():
        p = contained(ROOT, p.as_posix())
    b = read(p)
    assert sha(b) == row["sha256"], str(p)
    if "bytes" in row:
        assert len(b) == row["bytes"], str(p)

def package(name, digest, raw_count, modern):
    base = ROOT / name
    index_bytes = read(base / "index.json")
    assert sha(index_bytes) == digest
    index = json.loads(index_bytes)
    rows = index["raw_files"]
    assert len(rows) == raw_count
    src, dst = set(), set()
    total_raw = total_stored = compressed = 0
    for row in rows:
        source = row["source"] if modern else row["path"]
        destination = row["path"] if modern else row["destination"]
        assert source not in src and destination not in dst
        src.add(source); dst.add(destination)
        original = read(contained(ROOT, source))
        stored = read(contained(base, destination))
        if modern:
            assert row["storage"] in ("identity", "gzip")
            assert len(stored) == row["stored_bytes"] and sha(stored) == row["stored_sha256"]
            raw = gzip.decompress(stored) if row["storage"] == "gzip" else stored
            assert len(raw) == row["raw_bytes"] and sha(raw) == row["raw_sha256"]
            compressed += row["storage"] == "gzip"
        else:
            raw = stored
            assert len(raw) == row["bytes"] and sha(raw) == row["sha256"]
        assert raw == original, source
        total_raw += len(raw); total_stored += len(stored)
    generated = index["generated_files"]
    assert len(generated) == 2
    for row in generated:
        assert row["path"] not in dst
        dst.add(row["path"])
        b = read(contained(base, row["path"]))
        assert len(b) == row["bytes"] and sha(b) == row["sha256"]
    assert index.get("index_self_excluded", True)
    actual = {p.relative_to(base).as_posix() for p in base.rglob("*") if p.is_file()}
    assert dst | {"index.json"} <= actual
    extra = actual - (dst | {"index.json"})
    # A later, independently indexed child supplement is outside this finite
    # parent index. Never silently accept extra raw or root-level records.
    assert not extra or (modern and all("/" in x and not x.startswith("raw/") for x in extra)), extra
    if modern:
        assert index["copy_count"] == raw_count and index["raw_bytes"] == total_raw and index["stored_raw_bytes"] == total_stored
    proof["packages"].append({"path": name, "index_sha256": digest, "unique_original_sources": len(src), "raw_copies": len(rows), "generated": len(generated), "gzip": compressed, "raw_bytes": total_raw, "stored_raw_bytes": total_stored, "all_original_bytes_exact": True, "unindexed_additional_child_roots_not_reviewed": sorted({x.split('/')[0] for x in extra})})
    return index

try:
    extraction = package("docs/verification/extraction-source-trust-20261021", "deb282a6314b90f179bdf2a4a9e2668dcd3584bf83b7e32baaf11e715c95d2b2", 373, True)
    ci = package("docs/verification/table-xlsx-provenance-20261021/public-ci-905a248-20261021", "3c94d8085a9b8488e1e8a5e72c3ee6d14ec757369a3e42bc121c4b9eec2743ca", 11, False)
    assert sha(read(ROOT / "docs/verification/table-xlsx-provenance-20261021/index.json")) == "d2c27a15e2d183e4150edba2c0e9f3071b8210ec72eb076631de022a848ff004"
    rust_root = "artifacts/extraction-source-rust-20261021-r3"
    rust = js(rust_root + "/proof.json")
    assert rust["complete"] is True and rust["changed"] == []
    assert [x["name"] for x in rust["results"]] == ["format", "workspace-tests", "build", "clippy"]
    assert all(x["code"] == 0 and x.get("signal") is None for x in rust["results"])
    assert rust["results"][1]["args"] == ["test", "--workspace", "--locked", "--", "--test-threads=2"]
    counters = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", read(ROOT / (rust_root + "/workspace-tests.stdout.log")).decode("utf-8", "replace"))
    passed, failed, ignored = map(sum, zip(*[(int(a), int(b), int(c)) for a, b, c in counters]))
    assert (passed, failed, ignored) == (1843, 0, 5)
    compiler = js(rust_root + "/inputs.json")
    assert len(compiler) == len({x["file"] for x in compiler}) == 423
    for x in compiler: pin(x)
    binary = read(ROOT / "target/debug/open-seo-spider.exe")
    assert len(binary) == rust["binary"]["bytes"] == 90578432
    assert sha(binary) == rust["binary"]["sha256"] == "4d29595df29ff22bb2b56e73c702b59f69bc24e0f0b5ec0f7011eedb3535060e"
    pure = js("artifacts/extraction-source-pure-20261021-r2/proof.json")
    assert pure["complete"] is True and pure["changed"] == [] and pure["passed"] == 535
    assert sum(x["passed"] for x in pure["results"]) == 535
    assert all(x["code"] == 0 and x["failed"] == 0 and x["passed"] == x["tests"] for x in pure["results"])
    for x in pure["sources"]: pin(x)
    semantic = js("artifacts/extraction-source-semantic-20261021-r1.json")
    assert semantic["passed"] is True and semantic["groups"] == 40 and semantic["actual_browser"] is False and semantic["actual_engine"] is False
    for file, row in semantic["reviewed_current_targets"].items(): pin({"file": file, **row})
    optins = js("artifacts/extraction-source-renderer-optins-20261021.proof.json")
    assert optins["complete"] is True and optins["code"] == 0 and optins["signal"] is None and optins["changed"] == [] and optins["binary_unchanged"] is True
    assert len(optins["passed"]) == len(set(optins["passed"])) == 4
    assert optins["binary_sha256"] == sha(binary)
    optin_counts = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", read(ROOT / "artifacts/extraction-source-renderer-optins-20261021.stdout.log").decode("utf-8", "replace"))
    assert sum(int(x[0]) for x in optin_counts) == 4 and all(int(x[1]) == 0 for x in optin_counts)
    result = js("docs/verification/extraction-source-trust-20261021/local-result.json")
    editorial = js("artifacts/extraction-source-editorial-qualification-20261021.json")
    assert editorial["previous_index_sha256"] == "dfab2ee29d5e637e3a069cff4e401c5dfe9376dfe5f850e6cd82088296a14cfa"
    assert editorial["current_index_sha256"] == "deb282a6314b90f179bdf2a4a9e2668dcd3584bf83b7e32baaf11e715c95d2b2"
    assert editorial["raw_files_unchanged"] == 373
    assert editorial["current_readme"] in extraction["generated_files"]
    proof["post_materializer_generated_qualification"] = {"original_plan_outputs_not_claimed_unchanged": True, "original_index_sha256": editorial["previous_index_sha256"], "current_index_sha256": editorial["current_index_sha256"], "current_README": editorial["current_readme"], "scope": editorial["scope"]}
    native_root = result["selected_final_native"]
    native = js(native_root + "/proof.json")
    wrapper = js(native_root + ".wrapper.json")
    assert wrapper["code"] == 0 and wrapper["signal"] is None and wrapper["binary_unchanged"] is True
    assert native["complete"] is True and native["changed"] == [] and native["tracked_sources_unchanged"] is True and native["binary_unchanged"] is True
    assert native["shutdown"] == {"code": 0, "signal": None}
    assert native["binary_sha256"] == sha(binary)
    assert len(native["results"]) == 1 and native["results"][0]["name"] == "extraction-source-trust" and native["results"][0]["kind"] == "owned-native"
    assert native["results"][0]["code"] == 0 and native["results"][0]["signal"] is None and native["results"][0]["timed_out"] is False
    live_file = result["native"]["original_child_receipt"]
    live_root = (ROOT / live_file).parent
    live = js(live_file)
    assert live["result"] == "PASS" and not live.get("failure")
    assert live["errors"] == live["external_browser_requests"] == live["offline_product_requests"] == []
    for x in live["pins"]: pin(x, True)
    assert len(live["native"]) == 2 and all(x["natural_eof"] is True and x["forced"] is False and x["close"] == {"code": 0, "signal": None} for x in live["native"])
    assert len(live["positive_410_witnesses"]) == 4 and all(x["status"] == 410 for x in live["positive_410_witnesses"])
    assert len(live["files"]) == len({x["label"] for x in live["files"]}) == 19
    assert sum(x["rows"] for x in live["files"]) == 712 and len(live["downloads"]) == 38
    for row in live["files"]:
        for kind in ("xlsx", "csv"):
            b = read(contained(live_root, row["label"] + "." + kind))
            assert len(b) == row[kind + "_bytes"] and sha(b) == row[kind + "_sha256"]
    api = live["api_csv_exports"]
    assert len(api) == 4 and len({x["path"] for x in api}) == 4 and sum(x["rows"] for x in api) == 265
    for row in api:
        assert row["status"] == 200 and row["content_type"] == "text/csv; charset=utf-8" and row["browser_download_events"] == 0
        b = read(contained(live_root, row["path"]))
        assert len(b) == row["bytes"] and sha(b) == row["sha256"] == row["repeat_sha256"]
        cells = list(csv.reader(io.StringIO(b.decode("utf-8-sig"), newline="")))
        assert cells[0] == row["columns"] and len(cells) - 1 == row["rows"] and all(len(x) == 28 for x in cells)
    expected_drivers = ["5ca69d7995dc25950440f274b21a180dd92e6bc1159972b953ef72bcdfa5a271", "dd4a948671e8c7fe3c346285864cdbe72f5b93477dd80b544fa97a44adb1ea70", "a4a1d9cf63dcb1136a39b624132e64d824e0acf14ff5e0a14e0a2f8c26c1cf02", "8066496e72d2af568e6c9e31d67c1341d4efeddf0ccced31666af53d427828e0"]
    for n in (1, 2):
        p = js(f"artifacts/extraction-source-rust-20261021-r{n}/proof.json")
        assert p["complete"] is False and p["failure"] and p["results"][-1]["code"] == 101
    for n in (1, 2, 3, 4):
        stem = f"artifacts/extraction-source-native-20261021-r{n}"
        p, w = js(stem + "/proof.json"), js(stem + ".wrapper.json")
        assert p["complete"] is (n == 4) and w["code"] == (0 if n == 4 else 1)
        assert bool(p.get("failure")) is (n != 4)
        assert sha(read(ROOT / (stem + ".driver-source.cjs"))) == expected_drivers[n - 1]
    assert sha(read(ROOT / "web/verification/ui-extraction-source-trust-live.cjs")) == expected_drivers[-1]
    ci_source = ROOT / "artifacts/table-xlsx-provenance-public-ci-905a248-20261021"
    ci_receipt = json.loads(read(ci_source / "receipt.json"))
    assert ci_receipt["commit"] == "905a2480c54d431a42eae1ed93e43f094a3e7a76" and ci_receipt["run"] == 39609409363
    assert ci_receipt["status"] == "completed" and ci_receipt["conclusion"] == "success" and len(ci_receipt["jobs"]) == 4
    assert all(x["status"] == "completed" and x["conclusion"] == "success" for x in ci_receipt["jobs"])
    registry = read(ci_source / "registered-ui-e2e.mjs").decode()
    registered = [(a, c) for a, _, c in re.findall(r"^\s+\['([a-z0-9-]+)',\s*'([^']+)',\s*'(fixture|live|owned-native)'", registry, re.M)]
    log = read(ci_source / "renderer.log").decode("utf-8", "replace")
    observed_pass = re.findall(r"^\d{4}-[^\r\n ]+ PASS ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$", log, re.M)
    assert len(registered) == len(observed_pass) == len(set(observed_pass)) == 48 and registered == observed_pass
    assert sum(x[1] == "fixture" for x in observed_pass) == 9
    assert len(re.findall(r"^\d{4}-[^\r\n ]+ ##\[group\]Run node scripts/ui-e2e\.mjs\r?$", log, re.M)) == 1
    assert "No combined full49" in " ".join(result["scope_limits"])
    proof.update(result="PASS_READ_ONLY_FINITE_PUBLICATION_AND_SOURCE_BINDINGS", ordinary_Rust={"passed": passed, "failed": failed, "ignored": ignored}, fresh_separate_Core_renderer_optins=4, compiler_inputs_current_exact=len(compiler), selected_pure_tests=535, pure_inputs_current_exact=len(pure["sources"]), semantic_groups=40, semantic_targets_current_exact=len(semantic["reviewed_current_targets"]), live_pins_current_exact=len(live["pins"]), native_selected_suites=1, actual_browser_pairs=19, actual_browser_pair_rows=712, browser_download_events=38, API_only_CSV_files=4, API_only_CSV_rows=265, natural_child_EOF0=2, witnesses410=4, offline_product_refetches=0, original_Rust_FAIL_attempts=[1, 2], original_native_FAIL_attempts=[1, 2, 3], predecessor_public_CI={"commit": ci_receipt["commit"], "run": 39609409363, "jobs": 4, "unique_unfiltered_suites": 48, "fixtures": 9, "native": 39}, scope="Read-only byte-copy, terminal receipt/log, current input, and raw API CSV shape audit. No project runtime executed, no extraction/result oracle rerun, no all49 or GUI/vendor parity claim.")
except Exception:
    proof["failure"] = traceback.format_exc()
finally:
    for p, digest in observed.items():
        if sha(regular(p).read_bytes()) != digest:
            proof["result"] = "FAIL"
            proof["failure"] = "Input changed during final review: " + str(p)
            break
    proof["audit_sha256"] = sha(Path(__file__).read_bytes())
    destination = OUT / "receipt.json"
    assert not destination.exists(), "Fresh reviewer receipt required"
    destination.write_text(json.dumps(proof, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(proof, indent=2))
    if proof["result"].startswith("FAIL"):
        raise SystemExit(1)
