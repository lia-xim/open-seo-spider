# Archived Git control filename qualification

The precommit raw-byte check caught Git treating the archived `.gitattributes` as active configuration, causing normalization of archived App snapshots. No commit had been created. Its captured contents now use the inert filename `raw/.gitattributes.raw`; all 373 original raw contents and their hashes remain unchanged.

`previous-index.json` and `previous-README.md` retain the generated metadata reviewed by the previous audit. `transition.json` identifies the new parent index and the exact path-only change. The earlier audit remains a record of the earlier storage layout. Product sources and executed gates are unchanged. A subsequent Git-blob attestation is required before committing.
