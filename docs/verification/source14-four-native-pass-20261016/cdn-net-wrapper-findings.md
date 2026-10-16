# Two bounded static closures

Root captured exactly the two declarations named by the preceding E-only seed document. The unchanged archive is SHA `5b6fb252e194c9702e6ba8f6bf589e9870175e1478bcd6148a3f8068d4e18d76`, 13,824,663 bytes. This is ordinary selected ZIP/class analysis, with no JVM, initializer, connection, or vendor-process execution. It is not REA Evidence or a runtime observation.

## Concrete callback dispatch

`seo/spider/net/id281146822.class`, 2,812 bytes, SHA `927e5cf98304262723876b5fc6058300548f77f21b87cd737d3b193b61e15e58`, contains six Code bodies totaling 281 bytes. Its exact `id281146822(String)V` method is Code offset2097, length229. PC0 loads this receiver; PC1 unconditionally invokes the same class's virtual `id281146822()V` before normal return atPC4. The exception region covers PC0..4. The no-argument declaration is abstract.

The previously captured inner class concretely extends that net class and implements the exact no-argument method with an eight-byte callback to the retained Worker's `id()V`. Together with the previously captured Worker construction and inherited String-entry invocation, this closes that local static dispatch chain to the method containing the established `getResponseCode` and `getContentType` write origins. It does not establish every intra-method branch, prove that any request was made, identify which concrete HttpURLConnection implementation was returned or its header/decoding behavior, or show that all model writes use this path.

The remaining branches in the229-byte entry inspect an IOException's text and contain retry-related calls and property writes. Their callees and all preconditions have not been fully analyzed here; no complete retry, TLS, or server-compatibility contract is claimed. No property write was executed by this investigation.

## Exact wrapper return

`uk/co/screamingfrog/utils/Z/id1840409942.class`, 988 bytes, SHA `e8eda08fd25e8d680fccb26ad757dca1ec5fcd57c3dd9d00fe1d74b7812cc7c0`, contains five Code bodies totaling53 bytes. Constructor `(String, int, int)V`, Code727/20, stores the first String argument unchanged in field `id281146822:String` atPC6 and the two integers in separate fields. Getter `id281146822()String`, Code801/5, directly returns that same field, without a transform.

This resolves the preceding utility report's outstanding wrapper-return equality for the inspected constructor/getter pair. Its earlier local substring, case-conversion-length and low-surrogate-boundary observations remain the exact static scope; they establish neither grapheme correctness nor every caller's behavior. Null and exceptional paths retain their actual earlier branch qualifications.

## Evidence limits

`capture.json` SHA `1bf8afa802500c9c7656ade420a9c14343f08e2c4853985cb467c31955d41db6` retains the actual selected method bodies and archive identities. `local-all-methods-root-r2.json` SHA `61314d0edcbb7cb8c996ecbaaf5770ac8f3ce51e86be15594888cf187e65110b`,61,887 bytes, records all11 Code bodies334 bytes and the abstract declaration. The first local-inspection JSON writer failed on the own reader's callable metadata field; its incomplete `local-all-methods-root.json` is preserved and is not evidence. The second writer selects only serializable class/field/method data. This repair involved own inspection serialization and no target action.

Root authored this interpretation after reading the displayed whole method bodies. Independent review is separate. No Rust, feature status, HTTP test, installation, or global manufacturer-parity result follows from these static closures. The later alternative content producer, concrete connection class, global writer completeness, disk persistence and runtime behavior remain unproved.
