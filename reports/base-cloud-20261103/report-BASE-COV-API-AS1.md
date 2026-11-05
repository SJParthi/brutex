STATUS: IN PROGRESS

# BASE-COV-API-AS1 (shard G1 of `SHARDS.md`), BASE-COV-API repair round 3, 5 Nov 2026

TOKEN `BASE-COV-API-AS1-20261105-1`; branch `cloud/cov-api-as1-20261105`, cut from the kit commit
`f4111a0132473d8771d8624a9538b1362926f70b`; START `6ea46e59c07488eeccff388407976041852aae2c`.

This report is written early and rewritten as the work lands; line 1 says where it stands. The final
version carries the figures, the lists and the fix json.

## Done so far

- Kit read and checked: every kit file's sha256 equals `MANIFEST.md`'s.
- Route B on this host: the pinned nightly installs here and every pin matches (`rustc 1.99.0-nightly
  (1a98b1e13 2026-08-07)`, `llvm-cov`/`llvm-profdata` `LLVM version 23.1.0-rust-1.99.0-nightly`).
- Blocking item (3), G1's files: committed in `c4969e7a` and `87e75385`; every new test shown red under a
  hand change (ten hand changes, ten tests red at their new assertions).

## In progress

- Blocking item (1), G1's GAP rows.
