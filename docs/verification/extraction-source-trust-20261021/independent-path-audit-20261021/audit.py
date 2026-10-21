"""Read-only storage-path qualification audit. Own ignored receipt is sole write."""
from pathlib import Path, PurePosixPath
import copy, gzip, hashlib, json, subprocess, traceback
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
BASE = ROOT / "docs/verification/extraction-source-trust-20261021"
QUAL = BASE / "storage-path-qualification-20261021"
CI = ROOT / "docs/verification/table-xlsx-provenance-20261021/public-ci-905a248-20261021"
sha = lambda b: hashlib.sha256(b).hexdigest()
proof = {"kind": "independent_read_only_inert_archive_filename_qualification", "result": "FAIL", "created_at": datetime.now(timezone.utc).isoformat()}
seen = {}

def read(p):
    assert p.is_file() and not p.is_symlink(), str(p)
    b = p.read_bytes()
    digest = sha(b)
    assert p not in seen or seen[p] == digest, "Input changed during review: " + str(p)
    seen[p] = digest
    return b

def bounded(anchor, name):
    parts = PurePosixPath(name).parts
    assert name and "\\" not in name and ":" not in name and not PurePosixPath(name).is_absolute()
    assert all(x not in ("", ".", "..") for x in parts)
    p = anchor.joinpath(*parts)
    assert p.resolve().is_relative_to(anchor.resolve())
    q = p
    while q != anchor:
        assert not q.is_symlink(), str(q)
        q = q.parent
    return p

def js(p): return json.loads(read(p).decode("utf-8-sig"))

def current_pin(row):
    p = Path(row.get("file", row.get("path")))
    if not p.is_absolute(): p = bounded(ROOT, p.as_posix())
    b = read(p)
    assert sha(b) == row["sha256"], str(p)
    assert "bytes" not in row or len(b) == row["bytes"], str(p)

try:
    current_bytes = read(BASE / "index.json")
    assert sha(current_bytes) == "6acc698667e4306d60ed6e8c0fcc563bc86d65fa1926d6901ca26d81c25b4e0d"
    current = json.loads(current_bytes)
    previous_bytes = read(QUAL / "previous-index.json")
    assert sha(previous_bytes) == "deb282a6314b90f179bdf2a4a9e2668dcd3584bf83b7e32baaf11e715c95d2b2"
    previous = json.loads(previous_bytes)
    transition = js(QUAL / "transition.json")
    assert transition["previous_index_sha256"] == sha(previous_bytes) and transition["current_index_sha256"] == sha(current_bytes)
    assert transition["previous_storage_path"] == "raw/.gitattributes" and transition["current_storage_path"] == "raw/.gitattributes.raw"
    assert transition["source"] == ".gitattributes" and transition["raw_files"] == 373 and transition["raw_contents_unchanged"] is True
    assert not (BASE / "raw/.gitattributes").exists()
    controls = [x for x in current["raw_files"] if x["source"] == ".gitattributes"]
    assert len(controls) == 1 and controls[0]["path"] == "raw/.gitattributes.raw"
    assert controls[0]["raw_bytes"] == transition["raw_bytes"] == 69306
    assert controls[0]["raw_sha256"] == transition["raw_sha256"] == "5f969343baaa349469167465b21d9faf46361c056e386b905c431291c28df21d"
    comparable = copy.deepcopy(current)
    for row in comparable["raw_files"]:
        if row["source"] == ".gitattributes": row["path"] = "raw/.gitattributes"
    comparable["generated_files"] = previous["generated_files"]
    assert comparable == previous, "Index changed beyond one storage path and generated metadata"
    assert len(current["raw_files"]) == current["copy_count"] == 373
    assert len({x["source"] for x in current["raw_files"]}) == len({x["path"] for x in current["raw_files"]}) == 373
    raw_total = stored_total = 0
    for row in current["raw_files"]:
        stored = read(bounded(BASE, row["path"]))
        assert len(stored) == row["stored_bytes"] and sha(stored) == row["stored_sha256"]
        assert row["storage"] in ("identity", "gzip")
        raw = gzip.decompress(stored) if row["storage"] == "gzip" else stored
        assert len(raw) == row["raw_bytes"] and sha(raw) == row["raw_sha256"]
        assert raw == read(bounded(ROOT, row["source"])), row["source"]
        raw_total += len(raw); stored_total += len(stored)
    assert raw_total == current["raw_bytes"] == 13555596 and stored_total == current["stored_raw_bytes"] == 13555596
    assert len(current["generated_files"]) == 2 and current["index_self_excluded"] is True
    for row in current["generated_files"]:
        b = read(bounded(BASE, row["path"]))
        assert len(b) == row["bytes"] and sha(b) == row["sha256"]
    assert sha(read(BASE / "local-result.json")) == "27b7a240b7cf4e168b87196f7fc2327da41a486226939449f9e0af01a3326329"
    before_readme = read(QUAL / "previous-README.md")
    after_readme = read(BASE / "README.md")
    suffix = "\nPublication storage qualification: the captured root `.gitattributes` is stored as `raw/.gitattributes.raw`, with its original bytes and hashes unchanged. This inert filename prevents the archived control file from changing Git normalization of other archived evidence. See the [path qualification](storage-path-qualification-20261021/README.md); the original materialization plan and earlier audit describe the previous storage path.\n".encode()
    assert after_readme == before_readme + suffix
    assert sha(before_readme) == transition["previous_generated_readme_sha256"] == "1777a3b1d40f1b77e008bc0969fbff1f1a49b4ad557ed1a65954f6ec9904959c"
    assert sha(after_readme) == transition["current_generated_readme_sha256"] == "a2acdedbcfa2c2498753f8fa7323758c4d87fac2df1d44a999544c3c2d627634"
    qindex_bytes = read(QUAL / "index.json")
    assert sha(qindex_bytes) == "85d22e79fefd8172734447dc8c03363fb199b1feadc6e95504cbbe18a99c4653"
    qindex = json.loads(qindex_bytes)
    assert len(qindex["files"]) == 4 and qindex["index_self_excluded"] is True
    for row in qindex["files"]:
        b = read(bounded(QUAL, row["path"])); assert len(b) == row["bytes"] and sha(b) == row["sha256"]
    assert {p.relative_to(QUAL).as_posix() for p in QUAL.rglob("*") if p.is_file()} == {x["path"] for x in qindex["files"]} | {"index.json"}
    roots = [BASE, CI]
    active_names = {".gitattributes", ".gitignore", ".gitmodules", ".git"}
    active = [p.relative_to(ROOT).as_posix() for root in roots for p in root.rglob("*") if p.name in active_names]
    assert active == [], active
    relative_roots = [p.relative_to(ROOT).as_posix() for p in roots]
    staged = subprocess.run(["git", "ls-files", "--cached", "-z", "--", *relative_roots], cwd=ROOT, check=True, capture_output=True).stdout.decode().split("\0")
    staged_controls = [p for p in staged if p and PurePosixPath(p).name in active_names]
    assert staged_controls == [], staged_controls
    sample = (BASE / "raw/artifacts/extraction-source-native-20261021-r1.app-source.js").relative_to(ROOT).as_posix()
    attrs = subprocess.run(["git", "check-attr", "-z", "text", "eol", "filter", "working-tree-encoding", "--", sample], cwd=ROOT, check=True, capture_output=True).stdout.decode().split("\0")
    attribute_values = {attrs[n+1]: attrs[n+2] for n in range(0, len(attrs)-1, 3)}
    assert attribute_values["text"] == "unset" and attribute_values["filter"] == attribute_values["working-tree-encoding"] == "unspecified"
    compiler = js(ROOT / "artifacts/extraction-source-rust-20261021-r3/inputs.json")
    pure = js(ROOT / "artifacts/extraction-source-pure-20261021-r2/proof.json")
    semantic = js(ROOT / "artifacts/extraction-source-semantic-20261021-r1.json")
    result = js(BASE / "local-result.json")
    live = js(ROOT / result["native"]["original_child_receipt"])
    for row in compiler + pure["sources"] + live["pins"]: current_pin(row)
    for name, row in semantic["reviewed_current_targets"].items(): current_pin({"file": name, **row})
    assert (len(compiler), len(pure["sources"]), len(live["pins"])) == (423, 49, 22)
    assert pure["complete"] is True and pure["passed"] == 535 and pure["changed"] == []
    assert live["result"] == "PASS" and result["binary_sha256"] == "4d29595df29ff22bb2b56e73c702b59f69bc24e0f0b5ec0f7011eedb3535060e"
    assert sha(read(CI / "index.json")) == "3c94d8085a9b8488e1e8a5e72c3ee6d14ec757369a3e42bc121c4b9eec2743ca"
    assert sha(read(ROOT / "docs/verification/table-xlsx-provenance-20261021/index.json")) == "d2c27a15e2d183e4150edba2c0e9f3071b8210ec72eb076631de022a848ff004"
    proof.update(result="PASS_READ_ONLY_INERT_STORAGE_PATH_AND_UNCHANGED_GATE_BINDINGS", current_index_sha256=sha(current_bytes), previous_index_sha256=sha(previous_bytes), qualification_index_sha256=sha(qindex_bytes), exact_raw_copies=373, raw_bytes=raw_total, generated_files=2, sole_raw_path_delta={"source": ".gitattributes", "before": "raw/.gitattributes", "after": "raw/.gitattributes.raw", "bytes_unchanged": True}, README_append_only=True, qualification_files=4, active_worktree_Git_control_files=active, active_staged_Git_control_files=staged_controls, sample_effective_attributes=attribute_values, compiler_inputs_current_exact=423, pure_inputs_current_exact=49, live_pins_current_exact=22, semantic_targets_current_exact=4, old_provenance_parent_and_CI_indices_unchanged=True, scope="Storage filename/generated metadata qualification only. All raw original bytes and gate bindings rechecked; earlier audit/runtime results remain historical with their earlier paths. No runtime, Git mutation or tracked write by reviewer; Root's separate batch Git-blob attestation remains distinct.")
except Exception:
    proof["failure"] = traceback.format_exc()
finally:
    for p, digest in seen.items():
        if sha(p.read_bytes()) != digest:
            proof["result"] = "FAIL"; proof["failure"] = "Input changed during review: " + str(p); break
    proof["audit_sha256"] = sha(Path(__file__).read_bytes())
    receipt = OUT / "receipt.json"; assert not receipt.exists()
    receipt.write_text(json.dumps(proof, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(proof, indent=2))
    if proof["result"].startswith("FAIL"): raise SystemExit(1)
