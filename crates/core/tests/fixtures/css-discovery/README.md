# Independently authored CSS inventory fixtures

These fifteen small supplied-text controls are original test data for the native `css-discovery-v1` profile. They were not exported or copied from Screaming Frog. Inputs never trigger requests; declared dependency candidates, syntax coverage and URL eligibility are separate observations.

`css_discovery::tests::authored_fixture_controls_preserve_order_and_coverage` consumes every record. Other inline tests assert exact UTF-8 spans, source hashes, HTTP-safe resolution, contextual hints, global positive bounds and no prefix result after overflow. An authored fixture is not a passing gate until its actual Cargo execution is recorded separately.

Primary behavior sources: [public cssparser0.37 Parser](https://docs.rs/cssparser/0.37.0/cssparser/struct.Parser.html), [CSS Syntax](https://www.w3.org/TR/css-syntax-3/#consume-ident-like-token), [CSS Images](https://www.w3.org/TR/css-images-4/#image-set-notation), [CSS Namespace declarations](https://www.w3.org/TR/css-namespaces-3/#declaration), [CSS relative URL basis](https://www.w3.org/TR/css-values-4/#relative-urls).
