STATUS: IN PROGRESS

# report-BASE-COV-API-AS2 (BASE-COV-API repair round 3, shard G2, 5 Nov 2026)

Branch `cloud/cov-api-as2-20261105`, cut from the kit commit
`f4111a0132473d8771d8624a9538b1362926f70b`; START `6ea46e59c07488eeccff388407976041852aae2c`.
Owners in order: token `BASE-COV-API-AS2-20261105-1` (commits `dd7f285b`..`f13f4f57`), the main
session's pause `PAUSED-BY-MAIN-20261105-1420` (`0eff0822`), and this continuation,
`BASE-COV-API-AS2-20261105-3` (from `8ed8a90f`). The earlier session wrote no report, so its
commits are its record; this file is the shard's one report from here on.

## Kit check

Every kit file's sha256 matches `MANIFEST.md` (the fenced ones extracted with the manifest's
awk line). Pinned nightly installed as `nightly-2026-08-08`: `rustc -V` prints exactly
`rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, its llvm-cov and llvm-profdata print
`LLVM version 23.1.0-rust-1.99.0-nightly`. Host: Linux, root, 4 cores, 15 GB memory, 30 GB
free disk; TMPDIR on a tmpfs folder of mode 1777.

## Done by the earlier session (token -1), read from its commits

- (b) Blocking item 2, `026e8be7`: the crowd child of
  `a_sources_question_it_cannot_read_or_a_full_pool_is_said_by_name` writes `child-ran` into
  a folder its parent names and reads, as `served_routes_tests.rs`' children do; the
  `println!` is gone. Its commit says gate 23's step run by hand lists the same 38 lines as
  f01d7a0e4 and no `api/src/sources_tests.rs` line (re-run by this session: see Gates).
- (c) Blocking item 3, `026e8be7`, `39046efa`, `f13f4f57`: `sources_tests.rs` that test
  13 extremes (10 / 2 / 1); `research_doors_tests.rs` a second combination; the small-size
  timing wrappers of `dayflags_rules_tests.rs` (3 tests), `disk_route.rs` and `server.rs`
  each have a new test beside them with a named `Extremes:` line (counts below, re-checked
  by this session).
- Two `should_panic` tests close the GAP rows `sources_tests.rs:429` and
  `research_doors_tests.rs:35`.

## This session

Work in progress: item (a), G2's GAP rows (1,972 rows at r2: 1,192 / 131 / 110 lines and
412 / 115 / 12 sides, production / test / ignored).

NEXT: close G2 production GAP rows file by file with tests; push after each file.
