STATUS: IN PROGRESS

# report-BASE-COV-LIBS-LS3 (session BASE-COV-LIBS-LS3, 3 Nov 2026 cloud wave)

TOKEN `BASE-COV-LIBS-LS3-20261103-1`; branch `cloud/cov-libs-ls3-20261103`, cut from
`origin/cloud/base-kit-libs-20261103` (`eeec369b`, code START `bd04fb299d23`). Shard: crates
`lake`, `indicators`, `vocab` (SHARDS.md, COV-LIBS attempt 5). Decision ids: D-8564, D-8565.

## Kit check

Every kit file this session reads matched MANIFEST.md's sha256 (CLOUD, CONTEXT, DECISIONS,
SHARDS, the progress file whole; RULES-MAC-COV-LIBS, fix-BASE-COV-LIBS.json, the a1-after gaps,
the (F)/(U) list, the ee65c9f6fcbd set, the D3 recipe, both Route B scripts, d3-covset.mjs and
replay-minus-reports.sh by their fenced content).

## Host

Linux x86_64, 4 cores, 15 GiB memory, 29 GiB free disk, runs as root. Gates on the repository's
pinned stable `rustc 1.97.1 (8bab26f4f 2026-07-14)`; one `CARGO_TARGET_DIR` per toolchain;
`TMPDIR` on the `/dev/shm` tmpfs.

## Step 1: the shard's tests at START

`cargo test -p lake -p indicators -p vocab --lib --locked --no-fail-fast`: EXIT 0
(indicators 396 passed, lake 101 passed, vocab 70 passed; 0 failed).
`cargo test -p lake -p indicators -p vocab --locked --no-fail-fast` (every target): EXIT 0,
0 failed; 6 ignored, all real-lake tests (`real_lake.rs` 4, `real_lake_regression.rs` 1,
`refusals.rs` 1: they need `~/.brutex/lake`, L-BCL-3, the Mac's).

## Route B baseline at START (code `bd04fb299d23`, branch tip `6ba720789967`)

Pinned nightly installed here as `nightly-2026-08-08`: `rustc -V` prints exactly
`rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`; its llvm-cov and llvm-profdata print
`LLVM version 23.1.0-rust-1.99.0-nightly`. Run by the kit's `d3-route-b-cloud.sh` (D3_NIGHTLY,
D3_LLVM_BIN, D3_CARGO pointed at that toolchain; no RUSTC_BOOTSTRAP) in a detached worktree at the
tip, set file = every `.rs` of the three crates whole (71 files, the same coverage lines as the kit's
`deep-BASE-COV-LIBS-ee65c9f6fcbd-set.txt` for these crates). 15:11:17-15:18:42 IST: build 0 min,
lake tests 0 min (135 passed, 6 ignored), indicators 4 min (508 passed), vocab 1 min (174 passed).

| crate | lines | uncovered lines | branch sides | uncovered sides |
|---|---|---|---|---|
| indicators | 16498/16540 | 42 | 1915/2186 | 271 |
| lake | 4987/5623 | 636 | 274/366 | 92 |
| vocab | 7636/7741 | 105 | 1126/1286 | 160 |
| shard | 29121/29904 | 783 | 3315/3838 | 523 |

NOT READ by the route (exit 3 lines): `crates/{lake,indicators,vocab}/benches/ratio.rs` (their
`[[bench]]` is `test = false`, so `cargo test --no-run` builds none: L-BCL-1) and
`crates/lake/src/lib.rs` (module declarations only, no executable line).

First classification of the 1,306 rows (recipe v4 step 4, by a brace-matching reader of each
`assert!`/`panic!` invocation in test code, then by hand): 410 (F) candidates, 84 production GAP
rows, 812 test-code GAP rows (of them 271 in the `#[ignore]`d real-lake tests and their helpers).

## Work log

(appended as each step lands)
