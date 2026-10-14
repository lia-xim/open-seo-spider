# Bounded compatibility-wrapper postfix review

No confirmed findings in the3 exact guarded deltas. Manifestc1da9605e574e302edeaece94e7f220b73c7df9df76fdbc12502b0f7e4035024 and all before/after hashes match the immutable Control inputs. Actual normalized-text transforms were checked, not compiler execution:

- sitemap_coverage/verified_read.rs:727 old project_read default wrapper now has exactly one cfg(test); all controlled/inner function bodies unchanged.
- sitemap_membership_analysis/window.rs:26 old prepare_window_analysis default wrapper now has exactly one cfg(test); its controlled/inner implementation/signature unchanged.
- sitemap_membership_analysis.rs:15-20 moves exactly that default function's reexport to cfg(test), retaining the production reexports of AnalysisReadInput/VerifiedAnalysisRead/prepare_window_analysis_with_control/project_prepared_window. All code outside this import block is exact.

Independent original Root reachability search found prepare_window_analysis calls only in sitemap_membership_analysis/window_tests.rs and sitemap_read_control_tests.rs; project_read calls only in sitemap_coverage/verified_read_tests.rs and sitemap_read_control_tests.rs. These modules are actual cfg(test) children. Removing only the two default wrappers from non-test compilation preserves their existing tests and avoids anticipated dead_code/unused-import under -D warnings. No compiler warning or diagnostic was observed. verify_raw/verify_source remain production-used at store/sitemap_raw_read.rs:240-241 and were correctly not gated. No blanket cfg(test) or function-body change.

## Copy/guard advice for next CSV suite/build

Read old255-source suite/build files only, never imported or invoked. New routes must replace both source/repo paths, new frozen manifest path/SHA/count/name list/minimum, new result directory and exact build->suite proof/result binding. Do not carry255/253/1035 or old qualification as current CSV proof. Derive exact numbers from the final new manifest. RuntimeRoot remains the original task outputs/open-seo-spider; Cargo cwd remains the frozen source, with guarded checkout equality. Keep forwardslash Windows JS absolute literals, HOLD before directory mutation/renderer import/child, actual error->close->finally log lifecycle, separate owned warmed renderer and previous-target backup before build. Installed/shared executables remain untouched.

Concrete narrow environment guard: old builder rejects RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS/RUSTC_WRAPPER/RUSTC_WORKSPACE_WRAPPER at19-21; old suite blindly inherits process.env at24. Copy the same rejection into the new CSV suite before any child/renderer. Otherwise externally supplied cap-lints/wrapper settings could weaken the claimed -D-warnings check. Root accepted this recommendation; no new runner is evaluated here and no old execution proof is transferred.

All Cargo/compiler/tests/SQLite/native/HTTP/browser gates NOT_RUN. Actual combined fmt0 is Root-reported only. This own review wrote no product/Root/Git/runtime files.
