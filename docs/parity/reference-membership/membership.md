# Measured reference filter membership

Run 2026-10-12T07:36:11.277Z; original24.3, normal missing-licence CLI, unchanged profile parameters. Full product parity **NOT PROVEN**.

78 named filter exports +9 All cohorts (87 CSVs), 75 local requests. 64 proposed trigger/control pairs confirmed; 4 additional triggers have an explicitly refined clean control. 10 remain partial.

Every exact CLI argument maps to a measured path set in `latest-run.json` (`filter_paths`). Empty sets are observations; they do not establish an unimplemented rule or successful acceptance.

| Filter | Rows | Pair evidence | Remaining contradiction |
| --- | ---: | --- | --- |
| Page Titles:Missing | 3 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Page Titles:Duplicate | 5 | REFERENCE_TRIGGER_AND_REFINED_CONTROL_OBSERVED | control: /baseline (PROPOSAL_DIFFERS_FROM_REFERENCE) |
| Page Titles:Over X Characters | 17 | REFERENCE_TRIGGER_AND_REFINED_CONTROL_OBSERVED | control: /baseline (PROPOSAL_DIFFERS_FROM_REFERENCE) |
| Page Titles:Below X Characters | 3 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Page Titles:Over X Pixels | 14 | REFERENCE_TRIGGER_AND_REFINED_CONTROL_OBSERVED | control: /baseline (PROPOSAL_DIFFERS_FROM_REFERENCE) |
| Page Titles:Below X Pixels | 3 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Page Titles:Same as H1 | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Page Titles:Multiple | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Page Titles:Outside <head> | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Meta Description:Missing | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Meta Description:Duplicate | 5 | REFERENCE_TRIGGER_AND_REFINED_CONTROL_OBSERVED | control: /baseline (PROPOSAL_DIFFERS_FROM_REFERENCE) |
| Meta Description:Over X Characters | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Meta Description:Below X Characters | 6 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Meta Description:Over X Pixels | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Meta Description:Below X Pixels | 5 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Meta Description:Multiple | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Meta Description:Outside <head> | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| H1:Missing | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| H1:Duplicate | 5 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| H1:Over X Characters | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| H1:Multiple | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| H1:Non-Sequential | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| H1:Alt Text in H1 | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| H2:Missing | 50 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| H2:Duplicate | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| H2:Over X Characters | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| H2:Multiple | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| H2:Non-Sequential | 0 | PARTIAL_REFERENCE_OBSERVATION | proposed_trigger: /heading-order (PROPOSAL_DIFFERS_FROM_REFERENCE) |
| Canonicals:Contains Canonical | 6 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Canonicals:Self Referencing | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Canonicals:Canonicalised | 5 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Canonicals:Missing | 56 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Canonicals:Multiple | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Canonicals:Non-Indexable Canonical | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Canonicals:Multiple Conflicting | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Canonicals:Canonical Is Relative | 6 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Canonicals:Unlinked | 0 | PARTIAL_REFERENCE_OBSERVATION | proposed_trigger: /canonical-unlinked (PROPOSAL_DIFFERS_FROM_REFERENCE) |
| Canonicals:Invalid Attribute In Annotation | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Canonicals:Contains Fragment URL | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Canonicals:Outside <head> | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Directives:Index | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Directives:Noindex | 4 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Directives:Follow | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Directives:Nofollow | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Directives:Refresh | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Directives:Outside <head> | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| URL:Non ASCII Characters | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| URL:Underscores | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| URL:Uppercase | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| URL:Parameters | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| URL:Over X Characters | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| URL:Multiple Slashes | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| URL:Repetitive Path | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| URL:Contains Space | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| URL:Broken Bookmark | 0 | PARTIAL_REFERENCE_OBSERVATION | proposed_trigger: /links (PROPOSAL_DIFFERS_FROM_REFERENCE) |
| URL:GA Tracking Parameters | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Links:Pages Without Internal Outlinks | 55 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Links:Internal Nofollow Outlinks | 2 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Links:Internal Outlinks With No Anchor Text | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Links:Non-Descriptive Anchor Text In Internal Outlinks | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Links:Follow & Nofollow Internal Inlinks To Page | 0 | PARTIAL_REFERENCE_OBSERVATION | proposed_trigger: /baseline (PROPOSAL_DIFFERS_FROM_REFERENCE) |
| Links:Internal Nofollow Inlinks Only | 0 | PARTIAL_REFERENCE_OBSERVATION | proposed_trigger: /nofollow-only-target (URL_NOT_IN_TAB_COHORT) |
| Links:Pages With High Crawl Depth | 0 | PARTIAL_REFERENCE_OBSERVATION | proposed_trigger: /depth-5 (PROPOSAL_DIFFERS_FROM_REFERENCE) |
| Links:Non-Indexable Page Inlinks Only | 0 | PARTIAL_REFERENCE_OBSERVATION | proposed_trigger: /nonindex-only-target (PROPOSAL_DIFFERS_FROM_REFERENCE) |
| Links:Pages With Uncrawlable Internal Outlinks | 0 | PARTIAL_REFERENCE_OBSERVATION | proposed_trigger: /links-uncrawlable (PROPOSAL_DIFFERS_FROM_REFERENCE) |
| Links:Outlinks To Localhost | 22 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Response Codes:Blocked by Robots.txt | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Response Codes:Success (2xx) | 68 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Response Codes:Redirection (3xx) | 4 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Response Codes:Client Error (4xx) | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Response Codes:Server Error (5xx) | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Response Codes:Internal Blocked by Robots.txt | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Response Codes:Internal Success (2xx) | 68 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Response Codes:Internal Redirection (3xx) | 4 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Response Codes:Internal Client Error (4xx) | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Response Codes:Internal Server Error (5xx) | 1 | REFERENCE_POSITIVE_AND_CONTROL_OBSERVED |  |
| Response Codes:Internal Redirect Chain | 0 | PARTIAL_REFERENCE_OBSERVATION | proposed_trigger: /redirect-one (PROPOSAL_DIFFERS_FROM_REFERENCE) |
| Response Codes:Internal Redirect Loop | 0 | PARTIAL_REFERENCE_OBSERVATION | proposed_trigger: /redirect-loop-one (PROPOSAL_DIFFERS_FROM_REFERENCE) |

The `All` cohort must contain a fixture before absence from a named filter can support a negative control. Non-followed nofollow targets and post-analysis-dependent filters remain explicit unknowns when their intended trigger is absent.

Initial/final aliases are preserved: redirect-one → redirect-two → baseline; baseline query variants are distinct observations with identical documents. Metadata duplicate controls therefore need distinct non-alias documents.

Profile SHA before a532165295810f953b6f7c89a0217834de096b141e7257dc075366d582d9ee70, after 791b5ba85eb643da77f27d3603d97fd70b00e185709c2ebf7434fda8799aa150. Changed property keys: none. Byte difference scope: COMMENT_LINES_ONLY. Byte snapshots are task-local work files; no rollback or parameter write was performed.

Sources: [official CLI](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/), [official tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/).

Run `node scripts/reference-membership.mjs` for a new controlled ordinary reference run; run `node scripts/reference-membership-normalize.mjs` to reparse the retained CSVs without executing the original.
