STATUS: IN PROGRESS

# BASE-COV-API-AS3 (shard G3, repair r3, 5 Nov 2026): working report

This file is rewritten as the session goes; line 1 is the state of the branch now.

- Branch `cloud/cov-api-as3-20261105`, cut from the kit commit `f4111a0132473d8771d8624a9538b1362926f70b`;
  START `6ea46e59c07488eeccff388407976041852aae2c`. Token `BASE-COV-API-AS3-20261105-1`.
- Host: 4 cores, 15 GB, root, Linux x86_64. Pinned nightly installed as `nightly-2026-08-08`:
  `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, `LLVM version 23.1.0-rust-1.99.0-nightly` (llvm-cov and
  llvm-profdata), so Route B can be run here; figures are this host's (the Mac re-measures).
- Done so far (pushed): verify r3 item (3) for `bars_series.rs` and `feeds_route.rs`; GAP rows of
  `bars_series.rs`, `feeds_route.rs`, `serve_edge_tests.rs`, `ops_safety_tests.rs`,
  `server_cloud_mutant_tests_b.rs`, `_c.rs`, `_d.rs`. The other G3 files are being worked in parallel
  and land in later commits.
- Not yet: the Route B measurement of the code tip, `gaps-AS3.txt.md`, `fu-AS3.txt.md`, the gates
  on the tip, `fix-BASE-COV-API-AS3.json.md`.
