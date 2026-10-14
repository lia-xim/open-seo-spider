# Bounded preparation review

This is a read-only source review of the exact generator/template bytes listed in `source-pins.json`. It does not execute the preparation generator, stage Git paths, run the future suite or builder, or establish a compiled/tested result. The generator was still an authored preparation candidate at the initial read; a later frozen-output check, if received, is recorded separately.

No confirmed defect was found in the reviewed preparation/route/count/source-closure scope.

## Source closure

The generator pins the previous manifest and checks all its source files (lines 15–18), asserts exactly the 16 explicitly owned product paths against Git porcelain (19–32), checks immutable Raw/Source handoff manifests (33–36) and Root facade review payload pins (37–38), then stages only those explicit paths (39). Each staged compiler payload is compared byte-for-byte through SHA against the actual working file (40–44). New paths must be exactly eight; the union must contain 246 files (45–48). All previous unowned inputs must retain their prior hashes (49–53). The source copy is verified after each file copy, and its manifest binds all 246 SHA/byte pairs and the explicit staged bindings (74–83).

The immutable original Raw handoff is preserved; the later Root collation postfix is captured through the explicit current staged/payload hashes. This review does not substitute the original Raw v1 hash for the postfixed product hash.

## Test inventory

The prior actual manifest is 238 files, minimum 957 tests and 175 required names. The new named groups are 15 Raw, nine Source, six Store facade, four CLI and two actual additional existing-file Source groups: 36 authored functions. Resulting requirements are 246 inputs, minimum 993 and 211 unique required names. The generator checks all these counts and uniqueness (55–71). These are authored inventory requirements; the generator explicitly records zero new tests executed.

## Future runner generation

The two old template hashes are pinned before replacement (85–89). Replacement first writes the actual streaming repository as a JSON string literal (90), updates source/result routes, manifest SHA, exact count tokens and scope qualification (91–98), then independently checks the extracted literal's type, absolute Windows path, normalized identity and existing directory (99–103). No stale old source route or 238/175/957 count token may remain. New runner files use exclusive creation and `node --check` only (104–106). This reviewer did not invoke that syntax command or execute the generator.

The replacement order converts `sitemap-large-processing-suite-v2` into `sitemap-streaming-read-suite-v1`; the builder's suite proof path follows that same route. Its debug/optimized result paths become `sitemap-streaming-read-<mode>-build-v1`. Both use the future `sitemap-streaming-read-source-v1` manifest and the actual streaming integration checkout.

## Hold, runtime and binary boundaries

The suite template's coordinated-release assertion is line 13, before result directory writes (22), Cargo spawn (35), dynamic renderer import (50), renderer listen and fixture listen. Built-in module imports and pure path setup occur before the assertion; no renderer/application import does. The builder assertion is line 14, before suite proof reads, output writes and process spawn. Generator preparation itself includes explicit Git staging and light `node --check`; it is not a heavy-execution gate and must only be run by the authorized Root preparation step.

The future suite verifies frozen-source and checkout hashes before work and again after work; runtime renderer inputs are bound to the same manifest before dynamic import and after the suite. It uses an owned ephemeral worker/fixture and asserts their ports are neither 8787 nor 8790. Child close events and owned worker/fixture closure are part of the existing template receipt; no actual closure was observed in this static review.

The builder requires a successful suite receipt with the same manifest hash, test/name minima, zero ignored and owned worker closure. It verifies both source and checkout input bytes/SHA before build and after build/finally, compiles from the frozen source directory, preserves any previous shared build target to an exclusive receipt-local file before invoking the build (93–103), and publishes only to its new result directory after actual Windows executable validation. The user-installed executable is not an output path of this code. These source properties are not a claim that a future compiler/build succeeded.

## Explicit limits

This review does not execute Git status/add/show or validate an actual new staging result. It does not validate arbitrary future edits beyond the pinned bytes, compiler dependency resolution outside the manifest, measured memory/latency, native behavior, full selected/coverage/CSV streaming, installation or feature parity. Generator failure can leave a fresh partial preparation directory; its exclusive-create assertions fail rather than silently replacing prior artifacts. This review is not an atomic-preparation guarantee.

## Actual frozen output follow-up

After Root performed the preparation, a bounded file-read/hash follow-up verified the actual manifest `1fc7c77677c82e7c9a9460aa30a8b261a54a2f7567e41a1b04638c965c93f85c`: all 246 frozen files and their current checkout counterparts match the byte/hash inventory; all 238 previous immutable files remain unchanged; eight paths are new; the 16 staged binding records match the full manifest. This last check compares published binding records and file bytes, not Git index data read independently by this reviewer.

All 211 required names are unique and actually authored in the manifest's frozen test sources. The minimum remains 993, with 36 new authored cases and zero executed cases in this source-only preparation. The actual preparation result SHA is `62993b3283c5fd527a657415f77d496174b5ff57a3ed77bd07786da873da4363`.

The actual suite (`ef3920ec29f3cf77dc62788b79fea8636698359cba1b104ccfc0e1b81fdce444`) and build runner (`1e7068826646cc939b407d69158eed9a4e111bc935a5ea905a075655a78ecf14`) were read and copied as evidence. Both contain the exact new manifest SHA, actual JSON-quoted streaming checkout, correct frozen source, updated route/count tokens, hold assertion before writes/spawn/dynamic imports, and frozen-source process cwd. Current runtime renderer source hashes also match the same manifest. `frozen-output-check.json` records these file-read checks. Neither runner, generator, Node syntax command nor any Git/compile/test/native/browser process was executed by this reviewer. No new confirmed finding arose in this actual-output follow-up.
