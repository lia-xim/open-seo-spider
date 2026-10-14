# Additive schema guard

The frozen CONTRACT.md (SHA404647902334c34b6aee33ea8218634d8716a1264f4763f2d1426d7e4579c71f) remains unchanged. Integrator identified an initialized-empty schema recovery hole in unconditional CREATE IF NOT EXISTS.

Use `pub(crate) initialize_writable_schema(connection:&Connection)->anyhow::Result<()>` at writable Store open after existing metadata/legacy schema creation, instead of directly execute_batch(SCHEMA). It inspects the bounded marker and table inventory BEFORE CREATE, rejects marker/table gaps and any partial schema, checks existing full shapes (and orphan nonempty rows without marker), and atomically creates both tables only for a genuinely absent uninitialized sidecar. Read-only load never calls it and never migrates.

Core adds one actual SQLite regression for dropped initialized-empty tables and explicit fresh creation after removing the test marker. Compiler/tests/native remain NOT_RUN. No breaking API/DTO changes.
