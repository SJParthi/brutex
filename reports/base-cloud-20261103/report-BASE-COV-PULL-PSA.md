STATUS: BLOCKED OUTSIDE
ASSEMBLED: dac5db855377bd0880854aa7826b4de35e179be3

# report-BASE-COV-PULL-PSA (TOKEN BASE-COV-PULL-PSA-20261103-1)

Session BASE-COV-PULL-PSA of the 3 Nov 2026 cloud wave, run on 5 Nov 2026: the assembly of BASE-COV-PULL's three
shards (operator decision D2 part A, `whole:pull`). Branch `cloud/cov-pull-psa-20261103`, written by the replay script
at the ASSEMBLED commit (line 2); owner file `OWNER-BASE-COV-PULL-PSA.md` (commit `4a09570d`). Code tip (the fix
json's `head`): **`3f77a7b0fa6012c9982d29dc5833d0a2bc8ae534`**, whose `.rs` files are those of `cef062ebf194`, the tip
the final gates and Route B run 2 measured (the commits after it change only `docs/` and `reports/`).

## Outcome

- **Step (1)**: PS1, PS2 and PS3 stacked with the kit's `replay-minus-reports.sh`, every proof holding; ASSEMBLED
  `dac5db855377` (174 files over START, 94 commits, no merge).
- **The assembled tree is green as root**: the pull suite passes whole (34 binaries, 3,607 passed, 0 failed, 54
  ignored) with no host red left; fmt and clippy `-D warnings` clean.
- **Step (2)**: every BLOCKED OUTSIDE need of the three shards is settled or named. Settled here: PS3's two `tenor.rs`
  rows (a test in `vendor.rs`), PS1's gate 1d words (648 declared, so gate 1d names only feat/db-fix's own 481), PS1's
  kill-test pair (a kill inside a month's creation made on purpose, no production hook), and the docs entries.
  Named: the Mac's macOS row, the operator's signed-SSM-send rows and L-BCP-3/L-BCP-5, MS1's `upkeep` rows.
- **Step (3)**: Route B whole:pull on the pinned nightly at `cef062ebf194` (run 2): **lines 175,299/179,878 (4,579
  uncovered), branches 12,622/14,464 (1,842 sides)**, 255 files; every uncovered row classed: **(F) 2,231, (U) 364,
  IGNORED 3,798 (L-BCP-3), UNMEASURED 11 (L-BCP-5), GAP 17**, none of the GAPs a cloud session may close.
- **Step (4)**: D-7335 to D-7339 written (`docs/05`), BCP-08..12 flipped to ✓ and BCP-13..21 added (`docs/04`),
  L-BCP-1/-3/-4 restated and L-BCP-5 added (`docs/06`).
- **Step (5)**: final gates green on `cef062ebf194` (below); `origin/feat/db-fix` has not moved (`f01d7a0e4`).
- **The bar is NOT held** (L-BCP-3, L-BCP-5 and 17 GAP rows), so D-7250's narrow reading does NOT end for `pull` here.

**Why line 1 is `BLOCKED OUTSIDE`** (kit `CLOUD.md` item 10): only items for the Mac, the operator, or another
session (MS1) are left; each is named below with its exact change.

## Kit check

All 26 `MANIFEST.md` rows match (sha256 of the whole file, or of the fenced block extracted with the manifest's own
`awk`). The kit scripts used here were copied out and re-hashed before use: `d3-route-b-cloud.sh` `f032b525...`,
`d3-covset.mjs` `ecef3bdc...`, `replay-minus-reports.sh` `f223185d...`. Decision ids: D-7338 and D-7339 are this
session's (kit `DECISIONS.md`); D-7335 to D-7337 are the shards', written here from their drafts. No `^#+ D-733[5-9]`
heading existed at START; the sentences at `docs/05-decisions.md` 87553, 87673 and 87772-87773 that call those ids
unused are older mentions, left as they are (the ledger is append-only).

## Host and pins

Linux x86_64 cloud container, uid 0 (root), 4 cores, 15 GiB memory, 26-30 GiB free on `/`. Stable `rustc 1.97.1
(8bab26f4f 2026-07-14)` (`rust-toolchain.toml`) for fmt, clippy and the suite. Route B: `rustup toolchain install
nightly-2026-08-08 --profile minimal --component llvm-tools-preview`; `rustc -V` prints exactly `rustc 1.99.0-nightly
(1a98b1e13 2026-08-07)`, `llvm-cov` and `llvm-profdata` print `LLVM version 23.1.0-rust-1.99.0-nightly`. One target
dir `/home/user/psa-tgt` (Route B's `CARGO_TARGET_DIR=/home/user/psa-k/cov` is a link to it), `CARGO_PROFILE_DEV_DEBUG=0`,
`TMPDIR=/dev/shm/psa-tmp` (a 16 GiB tmpfs, mode 1777). For Route B, the four conditions of D-7339 (the shards' host
notes): profiles under `/home/user/psa-prof` (mode 0777), the kit script run under umask 0000 with `D3_CARGO` a wrapper
running the nightly's cargo under umask 0022, and `kernel.pid_max` raised from 32,768 to 4,194,304 (no pid wrapped).

## Step (1): the stack

Shard tips on origin, fetched at the start, equal the tips the brief wrote (PS1 `db608fb4fbd4`, PS2 `3b18385cda5b`,
PS3 `f84355216759`), so no tip moved; all three reports read `STATUS: BLOCKED OUTSIDE`. The list: `git diff
--name-only 2e7a0b6d3b56 <TIP> -- . ':(exclude)reports'` prints 60 files for PS2 and 48 for PS3, so none is left out.

| call | START | FROM | TIP | ONTO | NEWREF | the script's line |
|---|---|---|---|---|---|---|
| PS1 | `f98a899398f6` | `5789f4353f6a` (kit) | `db608fb4fbd4` | (START) | `refs/heads/stack/psa-1` | `OK ... = 4e6cbdbc1e7b (26 commits replayed onto f98a899398f6, 10 skipped as reports only; P1-P4 hold; 68 files in X)` |
| PS2 | `2e7a0b6d3b56` | `2e7a0b6d3b56` | `3b18385cda5b` | `stack/psa-1` | `refs/heads/stack/psa-2` | `OK ... = 341fca3998ee (51 commits replayed onto 4e6cbdbc1e7b, 6 skipped as reports only; P1-P4 hold; 60 files in X)` |
| PS3 | `2e7a0b6d3b56` | `2e7a0b6d3b56` | `f84355216759` | `stack/psa-2` | `refs/heads/cloud/cov-pull-psa-20261103` | `OK ... = dac5db855377 (17 commits replayed onto 341fca3998ee, 5 skipped as reports only; P1-P4 hold; 48 files in X)` |

Checked again on the assembled tree: every file a shard changed equals the version of the last shard that changed it
(the two files PS1 and PS3 share, `gdfl_dropbox_cov_tests.rs` and `gdfl_run_start_cov_tests.rs`, PS1 changed only
before the GREEN-TIP PS3 began from, so they equal PS3's); 174 files differ from START outside `reports/` (68 + 60 +
48 - 2); 94 commits over START, no merge. The `stack/psa-*` refs are local only and were never pushed.

## Step (2): every BLOCKED OUTSIDE need

| shard | need | settled how, or whose |
|---|---|---|
| PS3 | `tenor.rs` 203-204 (`TenorError::HoursUnverified`'s words) need a `SessionRefusal`, whose fields are private to `vendor.rs` | **settled** (`a52270b1`): `vendor::tests::a_tenor_refused_for_unverified_hours_quotes_the_refusal`, PS3's drafted test widened over each venue, both window shapes, the calendar's first and last day together, and an empty and a non-ASCII citation; both rows covered in runs 1 and 2 (BCP-20) |
| PS1 (item 5) | the 124 gate 1d words of `base-cov-pull-ps1-1d-words.txt` into `.github/gates_tree.rs` | **settled** (`a52270b1`), and widened to the whole package: gate 1d (the tree's own `gates_tree.rs` and `source_scan.rs`) names 1,129 undeclared literals on the assembled tree, 637 at START, 481 at feat/db-fix; the 648 the package added (156 by attempt 1 and repairs 1-2, 492 by the shards, PS1's 124 among them; none removed) were each read at the literal that produces it (a producer finder mirroring `whole_segment`, `quoted_words` and `joined_pieces`) and declared as `BASE_COV_PULL_LITERAL` with their six kinds; gate 1d now names exactly feat/db-fix's own 481, the same set (BCP-21) |
| PS1 (item 1) | the kill test's unborn-month pair, `gdfl_import_tests.rs` `unborn == 0 \|\| open_day`, reached only when a timed SIGKILL lands inside a month's creation; PS1 asked the operator for a test-only pause point in the store | **settled without a hook** (`cef062eb`): no child can be held there, but what the kill leaves can be written: `Stop::CutInCreation` writes the clean run's journal to its first `begin` line and the month's header region of zeros (`store::file`'s interrupted genesis, D-1521), then judges and resumes it as every kill; it counts in no kill tally (`interrupted` still needs two timed kills inside a day). Both sides covered in run 2 |
| PS1 (item 2) | `ssm.rs` 966, 970, 971 run only after a signed send reached AWS | **the operator's**: an endpoint argument for the signed read (production passing the `https://{host}/` it builds from the region, a test pointing a client at a loopback listener). Not added: a second endpoint for a credential read is the operator's call (L-BCP-4) |
| PS1 (item 3) | `upkeep.rs` (4 rows) and `upkeep_tests.rs` (9 rows) | **MS1's (BASE-MSUM-BACKFILL), or the operator's**: both files are in MS1's list and MS1 is changing them on `fix/db-BASE-MSUM-BACKFILL`; an edit here would collide at the landing. Whichever of the two lands second closes them; rows in `base-cov-pull-fu.txt.md` |
| PS1 (item 4), PS2, PS3 | macOS rows, L-BCP-3, Route B whole:pull on the Mac's pin, the p99 bound of `calendar_tradable_tests.rs:1234` on the Mac's instrumented build | **the Mac's** (NEEDS THE MAC below). The p99 bound passed in both runs here |
| PS1 (item 5), PS2, PS3 | D-7335, D-7336, D-7337 and the invariant rows | **settled**: written from the drafts (step 4) |
| PS1, PS2 (host reds) | nine root-only reds (PS1's list) | **gone**: PS2 (`d16d4ca0`) and PS3 (`d418a3ec`) run them where the bits bind (D-0995); the assembled suite has no failure as root |

## Step (3): Route B whole:pull

Command (both runs; the kit's script unchanged): `umask 0000; D3_W=/home/user/psa-w D3_DIR=/home/user/psa-kit
D3_NIGHTLY=<nightly> D3_LLVM_BIN=<nightly>/lib/rustlib/x86_64-unknown-linux-gnu/bin
D3_CARGO=/home/user/psa-kit/cargo-umask CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/dev/shm/psa-tmp bash d3-route-b-cloud.sh
/home/user/brutex /home/user/psa-k /home/user/psa-prof/<tip12>-<run>-B <log dir> <set> pull`. The set
(`deep-BASE-COV-PULL-PSA-<tip12>-set.txt.md`): `reading whole:pull`, 255 coverage lines (every tracked `.rs` of
`crates/pull` but the 8 bench files, in no test object, as the kit's set), and the mutants lines of the D2 set of
`f01d7a0e4...<tip>` for the deferred pass (made by a generator that reproduces the kit's f98a899398f6 set byte for byte).

| run | tip | times (IST, 5 Nov) | instrumented suite | profiles | whole:pull lines | whole:pull branches |
|---|---|---|---|---|---|---|
| 1 | `a52270b1f00a` | 16:12:43 build 1 min, tests 4 min, read to 16:51:24 | 3,607 passed, 0 failed, 54 ignored | 132 (62 uid 65534) | 175,277/179,855 (4,578 uncovered) | 12,614/14,454 (1,840 sides) |
| 2 | **`cef062ebf194`** | 17:10:38 build 1 min, tests 4 min, read to 17:50:52 | 3,607 passed, 0 failed, 54 ignored | 132 (62 uid 65534) | **175,299/179,878 (4,579 uncovered)** | **12,622/14,464 (1,842 sides)** |

Both: `ROUTE-B DONE ...: every package measured and read (pull)`, exit 0, no NOT PROVED line, no test process ended by
a signal. BEFORE (the second check's measure of `c9d653d5f`, the kit): lines 151,638/158,587 (6,949 uncovered),
branches 11,522/14,216 (2,694 sides).

**By group, run 2** (G1 = every file PS2 and PS3 do not name: PS1's 89 and repair 2's `census_store_cov_tests.rs` and
`gdfl_rekey_cov_tests.rs`, which no shard counted, both whole):

| group | files | lines | branches | (F) | (U) | IGNORED | UNMEASURED | GAP |
|---|---|---|---|---|---|---|---|---|
| G1 | 91 | 57,738/59,292 (1,554) | 4,193/4,954 (761) | 854 | 138 | 1,306 | 0 | 17 |
| G2 | 93 | 58,766/60,545 (1,779) | 4,140/4,724 (584) | 623 | 146 | 1,594 | 0 | 0 |
| G3 | 71 | 58,795/60,041 (1,246) | 4,289/4,786 (497) | 754 | 80 | 898 | 11 | 0 |
| **pull** | **255** | **175,299/179,878 (4,579)** | **12,622/14,464 (1,842)** | **2,231** | **364** | **3,798** | **11** | **17** |

**How the rows were classed.** Each uncovered row of the run carries the class the owning shard's list gives the
same row (file, line, column, side and text equal; each file's lines mapped back to the shard's tip through `git
diff -U0`, which moves only `vendor.rs` and `gdfl_import_tests.rs`): PS1's `base-cov-pull-ps1-fu.txt` (with its rule
that every other G1 row of its run 9 is L-BCP-3), PS2's `gaps-PS2.txt` and `fu-PS2.txt`, PS3's `gaps-PS3.txt` and
`fu-PS3.txt`. In run 1 all 6,418 rows matched, and each group's counts equal its shard's last run exactly (G3 less the
two tenor rows); in run 2 only the five rows of this session's own new test code did not, and each was read by hand:
`gdfl_import_tests.rs:5013` (an `assert_eq!` message) and the four failing sides at `:5171` (the planted stop's
`assert!`), all (F). Checked besides: every G1 IGNORED count per file equals PS1's (`cash_auction.rs` 57,
`cash_session_cache.rs` 168, `gdfl_nfo_tests.rs` 910, `master_snapshots_tests.rs` 127, `reference_data_tests.rs` 44);
every (F) row in a production file lies inside its `#[cfg(test)]` module or a test-only module.

**One class changed.** PS3 listed eleven rows (U) (`cash_session_cache_lock_tests.rs` 377-388, the child of
`a_killed_holder_releases_the_day`). They run, in a child the test kills with SIGKILL, which writes no profile, so
they are not unreachable: the package's list holds them UNMEASURED (L-BCP-5), for the operator.

**GAP rows (17)**, none a cloud session may close:

- `listing_tests.rs:490`, the Darwin errno arm (`cfg!(target_os = "macos")`): the Mac's, never claimed here.
- `ssm.rs` 966, 970, 971, after a signed send reached AWS: the operator's (an endpoint argument for the signed read).
- `upkeep.rs` 1200:16 False, 1580:20 True, 1581, 1676:20 False; `upkeep_tests.rs` 460-462, 1001:9 False, 1001:47 True,
  1657:41 False, 1741-1743: MS1's files.

**Lists** (each one fenced block): `gaps-PSA.txt.md` (run 2's per-file table of all 255 files with each file's group,
the group and whole totals, every uncovered row with its class and who classed it), `base-cov-pull-fu.txt.md` (the
package's (F)/(U) list: the GAP rows with owners, the UNMEASURED rows, every (U) with its proof, every (F) with its
reason, the IGNORED rows counted by file; the Mac copies it to `state/fix/round2/base-cov-pull-fu.txt`),
`deep-BASE-COV-PULL-PSA-cef062ebf194-set.txt.md` (run 2's set; run 1's beside it).

## Step (4): docs

- `docs/05-decisions.md`: D-7335 (PS1), D-7336 (PS2), D-7337 (PS3) from the drafts in their reports; D-7338 (this
  assembly: the stack, the needs settled, the figures, what is left, the bar NOT held); D-7339 (the four conditions
  under which a root cloud host's Route B counts its uid-65534 children).
- `docs/04-invariants.md`: BCP-08 to BCP-12 flipped from "test written, NOT yet run" to ✓ (each of their fifteen
  tests passed by name in the stable run); BCP-13 to BCP-15 (PS1), BCP-16 to BCP-19 (PS3), BCP-20 and BCP-21 (this
  session), each beside tests that pass; BCP-02 named a file and a count, which CI gate 10 refuses ("a ✓ row names no
  test"), and now names its twelve tests.
- `docs/06-limits.md`: L-BCP-1 and L-BCP-3 carry the assembled figures (history kept); L-BCP-4 is down to the signed
  send's three rows; L-BCP-5 added.
- Docs gates run here with the tree's own tools: gate 10 refuses only what it refuses on feat/db-fix (R427-11 names no
  test; HA-02 and AG05-13 point at missing tests: base reds); gate 10b passes; gate 27b names only feat/db-fix's
  duplicate D-6247; gate 15 passes (the forbidden word appears nowhere).

## Step (5): final gates

On `cef062ebf194` (the code tip's `.rs`), stable 1.97.1, root, 4 cores, 17:16:16 to 17:23:06 IST:

- `cargo fmt --all --check`: rc 0.
- `cargo clippy -p pull --all-targets --locked -- -D warnings`: rc 0.
- `cargo test -p pull --locked --no-fail-fast`: rc 0; 34 binaries, **3,607 passed, 0 failed, 54 ignored**. No host red.
- `.github/gates_tree.rs` built and tested as CI does (`rustc --edition=2024 --test -D warnings`): 51 passed;
  `rustfmt --edition 2024 --check` clean.
- Gate 1d on the tree: 481 undeclared, exactly feat/db-fix's own set (base red, BASE-REDS-1023's).
- `origin/feat/db-fix` fetched at the end: `f01d7a0e430e8f9b01e424a299b0bed20738a0ef`, an ancestor of this branch;
  it has not moved past the base, so nothing was merged.

## Tests this session added or changed (each shown red under a hand change, then restored)

| test | file | what it pins | red check |
|---|---|---|---|
| `vendor::tests::a_tenor_refused_for_unverified_hours_quotes_the_refusal` | `crates/pull/src/vendor.rs` (G1's file, test code) | the tenor's unverified-hours words, the refusal quoted last, whole and once (BCP-20) | `tenor.rs`'s arm made to quote the venue instead: failed at its own assertion (`vendor.rs:7936`) |
| `gate_1d_declares_the_base_cov_pull_words` | `.github/gates_tree.rs` | the 648-word group, each once, disjoint from every other group (BCP-21) | one word dropped from the group (scratch copy): failed at its `d.contains` assertion |
| `Stop::CutInCreation` in `a_kill_at_any_point_leaves_no_half_month_and_the_resume_lands_the_same_store`, and `cut_in_creation` | `crates/pull/src/gdfl_import_tests.rs` (G1's file, test code) | a resume from a month cut off in its creation lands the clean run's bytes; the judgement's unborn sides | the store's repair of an existing interrupted genesis switched off (`store/src/file.rs`, `&& !existed`, never committed): failed at the planted stop's resume ("kill 7 after 0 months, 1 unborn ... no header slot survived"); green five times in a row after |

No production line changed in this session. No assertion was weakened, no test deleted or ignored, no
`#[ignore]` test touched.

## Files changed by this session (outside `reports/`)

`crates/pull/src/vendor.rs` and `crates/pull/src/gdfl_import_tests.rs` (G1's files, test code only, edited to settle
the shards' needs as the brief's steps 2 and 3 ask, after PS1 had ended), `.github/gates_tree.rs`,
`docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md` (the assembly's). `Cargo.toml`, `Cargo.lock` and
`crates/pull/src/lib.rs` were not changed (no module needed declaring).

## Benches

None run: no timed step was added or changed (`p50_p99`: none). Gate 8 and every p99 claim are the Mac's.

## NEEDS THE MAC

1. Route B whole:pull on its pin (aarch64-apple-darwin), which replaces these cloud figures and re-checks every (U)
   proof for its target.
2. The shadow GET re-check.
3. macOS rows: `listing_tests.rs:490`; PS1's tests gated `#[cfg(target_os = "linux")]` (an `O_PATH` descriptor, a
   sidecar linked to `/dev/null`, an unmappable thread), not compiled on macOS and so not claimed there; and the rows
   whose macOS run is unproven here (PS3): `ingest`'s post-read cap arm, covered only by a procfs test; the two
   `RUST_MIN_STACK` children; two FIFO tests that open the FIFO read-write; a config file another uid owns
   (`/etc/master.passwd` on macOS); the first block device under `/dev`.
4. Non-root 14-core runs of the whole suite (where no D-0995 child changes uid).
5. L-BCP-3 (the 54 ignored tests: 3,798 rows) and L-BCP-5 (11 rows): the operator's decisions.
6. The D1 landing, after BASE-REDS-1023; the merge is not "Merge BASE-COV-PULL:" while the bar is not held.

## For the operator

- L-BCP-3: synthetic twins, a real-data pass that counts, or another rule (12 of G2's 23 are timings on invented data,
  which a pass with `--ignored` would count).
- L-BCP-5: UNMEASURED, or another rule, for lines that run only in a child killed by SIGKILL.
- `ssm.rs` 966, 970, 971: whether `ssm::get_parameter` may take the URL of its signed POST as an argument (production
  passing the one it builds from the region) so a test can answer a signed read from a loopback listener.
- PS2's 21 (U, type invariant) rows, marked for a decision on private-field invariants.

## For MS1 (BASE-MSUM-BACKFILL)

The 13 GAP rows of `upkeep.rs` and `upkeep_tests.rs` listed above, and PS3's note: `upkeep.rs:907`'s run-status probe
holds `try_lock_shared()` on a raw `File` until close, the lock shape PS3 fixed twice (D-7337 item 4).

## Follow-ups (not changed here)

- From PS1: `master_snapshots::first_listed::listing` has no `carried(route)` guard where `absorb_held` has one;
  `fno_membership`'s reader-thread-panic arm runs only in an unwinding build; `gdfl_cm::auction::append` under a
  relative root whose first folder is new syncs the folder `""` and refuses after its records were written;
  `refusal.rs`'s `Hash` comment omits a field it hashes.
- From PS2 (for the deferred mutants pass): `gdfl_run::ledger::ps2_tests::a_claims_line_too_long_for_a_record_is_never_read`
  does not pin `claimed()`'s length guard (`gdfl_run_ledger.rs` 527-529), which no line it reads can reach.
- From PS3: `store::repair::drop_month` records nothing in the quarantine; wording: `zerodha_verify::kept_window`,
  `gdfl_dropbox::transport_words`, `Config::load` of a bare file name, `LineageRecord::parse` accepting a zero lot,
  `--stored` not deduplicating names already in `only`.
- From the second check (kit): a named pipe at `FORMAT.md` or a `<yyyy>.zip` blocks `Origin::measure`; a `<yyyy>.zip`
  that is a directory adds its metadata length; `note_unattributed` formats the day when `booked` is 0. Its timing
  red (`the_loader_reads_every_table_inside_its_budget` under instrumentation) did not recur: both instrumented suites
  here passed whole.
- Base reds met, not this package's: gate 1d (481 words at feat/db-fix), gate 10 (R427-11, HA-02, AG05-13), gate 27b
  (D-6247 twice).

## Not done

- The bar: L-BCP-3, L-BCP-5 and the 17 GAP rows above.
- Mutants: deferred (operator, 30 Oct); the set files keep their mutants lines (1,952 at `cef062ebf194`).
- `Cargo.toml`, `Cargo.lock`, `lib.rs`: nothing was owed.

## Rules and scratch

Nothing pushed but this branch (never forced, never a pull request, never `main`, `feat/db-fix` or `fix/*`); no merge;
no credential, private value or vendor row (every literal added is invented, or `256265`, a public instrument number
already tracked in `crates/core`); the forbidden language was neither used nor named. Scratch (all made by this
session): `/home/user/psa-tgt`, `/home/user/psa-k`, `/home/user/psa-prof`, `/home/user/psa-kit`,
`/dev/shm/psa-tmp`, the session's scratchpad (node tools, logs, two detached worktrees at `f01d7a0e4` and START for
gate runs), and the local refs `stack/psa-1` and `stack/psa-2`.
