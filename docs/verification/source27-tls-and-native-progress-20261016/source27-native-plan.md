# Source27 TLS/HSTS native plan — not run

Current C2 source: 2cfa8d828de09905a68ab12c4193fc5875a245d76a540025008f37887b501ff9/881141; all13 named source files are rehashed and pinned in PLAN.NOT-RUN.json. This is planning, not a harness, source NoF or runtime grant.

Use the existing POST /api/crawls TLS config and existing CLI/MCP/UI frontdoors. Generate an isolated public localhost CA/leaf for an owned loopback HTTP/TLS fixture; .NET CertificateRequest through PS7 is a proposed Root option, not yet proven. Preserve certificate and hostname verification; no OS/browser trust changes.

The finite proposal covers default untrusted, PEM/DER trust, wrong hostname, document/robots HSTS, disabled HSTS and real saved UI staging. Five CLI children include three positives and two precise pre-TCP config refusals. The proposed8 direct children are App+Chrome+MCP+5CLI, with8 App jobs and3 positive CLI reports. HTTP15 is provisional until the fixture routes are fixed; TCP/handshake totals must be actually observed.

The denied-quota.start/no-TCP case is explicitly backend-only: its current test uses a private hook and manually prepared HSTS state. A successful backend test does not prove a public Native interleaving. Active renderer TLS remains refused/PARTIAL.

Root must bind actual source/suite/build/PE/fixture/grant and fresh source review, retain every resource owner and raw EOF, and drain all late browser events under the reviewed same-population quiescence contract. No execution was performed by the planner. Full exact config limits, source markers, manual expectations and missing inputs are in the JSON.
