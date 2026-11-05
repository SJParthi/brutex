STATUS: BLOCKED OUTSIDE

# report-BASE-COV-PULL-PS2 (TOKEN BASE-COV-PULL-PS2-20261105-2)

Final report, 5 Nov 2026 10:56 IST. Session BASE-COV-PULL-PS2 of the 3 Nov 2026 cloud wave, group G2 of
`SHARDS.md`, continued on a new account from the earlier session's last pushed commit `f4fe72bf744c` (its TOKEN
`BASE-COV-PULL-PS2-20261103-1`, now retired). Branch `cloud/cov-pull-ps2-20261103`, cut from
`2e7a0b6d3b568190024e2e9b405e803805f3fdca` (PS1's GREEN-TIP). Owner file
`reports/base-cloud-20261103/OWNER-BASE-COV-PULL-PS2.md`: commit `4f0ffe7c` (the first token), `6987bc77` (this
one). Code tip **`02f268e24f8bbf230099ed53ccdb8656aadf68d2`** (the fix json's `head`). This file replaces the interim report (whose "Update" section
had been spliced into the middle of its baseline line; its figures are restated below).

**Why `BLOCKED OUTSIDE` (CLOUD.md item 10: only items for the Mac or the operator are left).** Everything a cloud
session can do for G2 is done: every one of G2's uncovered rows that an input, a direct call, a test-code change or
a test seam can reach is driven by a test that asserts it, and every other row is classified with its proof. What
stays open is the operator's (L-BCP-3: the 54 `#[ignore]` tests, real-data tests and timings run by hand, whose
bodies and helpers hold 1,594 of G2's rows, so the bar as written is not held) and the Mac's (Route B whole:pull on its pin, macOS rows,
non-root runs, the D1 landing); PSA writes D-7336's docs entry from the draft below.

## Outcome

- **G2 by Route B at the code tip (run 5, the kit's script): lines 58,766/60,545 (1,779 uncovered), branches 4,140/4,724 (584 sides).** All 2,363 open rows are classified:
  (F) 623 (614 by pattern, 9 by hand), (U) 146 (21 of them on a private-field type invariant,
  marked), IGNORED (L-BCP-3) 1,594, **GAP 0**. Whole pull: lines 159,895/166,046 (6,151 uncovered), branches 11,903/14,346 (2,443 sides) (222 files).
- **This continuation:** 20 new tests and 5 test-code edits in 12 code commits drove the 62 rows a test could reach (60
  found by run 1: 33 behind a host fault, 12 behind an input or a direct call, 1 a race, 14 test-code arms; and 2
  one-sided rows of this session's own new seam code found by run 2), through test seams of D-7336: planted host
  faults in seven modules and steps keyed by path. No production behaviour changed: every seam is `#[cfg(test)]`.
- **Red checks:** all 234 new tests of both sessions (212 of the earlier session, red-checked here by two helpers
  in worktrees of their own; 22 of this continuation) shown red at their own assertion under a hand change, then
  undone: 234 red, 0 not red (`red-PS2.txt.md`).
- **Gates on the code tip:** fmt and clippy `-D warnings` clean; the pull suite 33 binaries, 3177 passed, 6 failed, 54 ignored, the six failures being
  G3's root-only tests, each passing as uid 65534 (host reds, NEEDS THE MAC).

## Kit check

All 26 manifest rows match `MANIFEST.md` (sha256 of the whole file, or of the fenced block extracted with the
manifest's own awk), and `MANIFEST.md` hashes to `078a849c...c6479`; `jq -e .` accepts the five json blocks. Checked
again by this continuation on 5 Nov.

## Host and pins

Linux x86_64 cloud container, uid 0, 4 cores, 15 GiB memory, 25-30 GiB free on `/` (a new container: the earlier
session's target dirs, profiles and lists did not survive the account switch). Stable `rustc 1.97.1 (8bab26f4f
2026-07-14)` (rust-toolchain.toml) for fmt, clippy and the suite. Route B: `rustup toolchain install
nightly-2026-08-08 --profile minimal --component llvm-tools-preview` gave exactly the pins: `rustc 1.99.0-nightly
(1a98b1e13 2026-08-07)`; `llvm-cov` and `llvm-profdata` `LLVM version 23.1.0-rust-1.99.0-nightly`. The kit's
`d3-route-b-cloud.sh` is run unchanged, as `D3_W=/home/user/ps2-work D3_DIR=<kit> D3_NIGHTLY=<nightly>
D3_LLVM_BIN=<nightly>/lib/rustlib/x86_64-unknown-linux-gnu/bin D3_CARGO=<nightly>/bin/cargo d3-route-b-cloud.sh
/home/user/brutex /home/user/ps2-work/deep /home/user/ps2-work/prof/<tip12>-<run>-B <log dir> <set> pull`, with
`CARGO_PROFILE_DEV_DEBUG=0` and `TMPDIR=/dev/shm/ps2-tmp` (mode 1777). The profile folder is outside the 0700
scratchpad, and a watcher sets `<P>/pull` to mode 1777 the moment the script makes it, so the D-0995 children
(uid 65534) write their profiles (PS1's host notes 1 and 2). One stable target dir (`/home/user/ps2-work/stable`)
for the gates and the red checks.

**Host note (pid reuse), for PS3 and the kit.** This host's `kernel.pid_max` was 32768, and the pull suite
spawns enough processes that the pids wrap within one run (in run 4, a child's profile of pid 4,268 was written 18 s
after one of pid 12,210). The kit names a profile `%p-%m.profraw`, so a uid-65534 child (D-0995) given the pid of an
earlier root process of the same binary is refused the merge into that root-owned 0644 file and writes no profile,
silently. In runs 3 and 4 one child's profile was lost
(`alerts::ps2_tests::a_kept_name_the_host_will_not_link_stops_the_replacement`: its body, and the one `alerts.rs`
side only it reaches, read as 28 uncovered rows, 26 lines and 2 sides, though the test passed; run alone, the same instrumented binary
writes that child's profile). Every run wraps (run 2's pids also fell from 22,917 to 15,006 within 19 s), so which
child loses is a matter of where the wrap lands. A loss can only remove coverage, so it shows as rows no class
explains. **Remedy, applied before run 5:** `kernel.pid_max` raised from 32768 to 4194304 (root may write it on this
VM; nothing else changes), so no run wraps; run 5's pids rise without falling back (below). PS3 and the kit should do
the same before Route B on a cloud host.

## Coverage: Route B figures

The set is every `src/` and `tests/` file of `crates/pull` at the tip, whole (`reading whole:pull`; benches are not
built by `cargo test`, as in the kit's `f98a899398f6` set): 222 coverage lines. G2 is SHARDS.md's 77 files and the
16 test files this group added (`*_ps2_tests.rs` beside their modules, and `tests/alerts_ps2.rs`): 93 files.

| run | tree | tests (instrumented) | whole:pull lines | whole:pull branches | G2 lines | G2 branches |
|---|---|---|---|---|---|---|
| START (earlier session) | `2e7a0b6d` | 2939 passed, 10 failed, 54 ignored | 153,379/160,118 | 11,562/14,244 | 52,247/54,617 (77 files) | 3,800/4,622 |
| earlier session | `c1ff1cc9` | (its report) | not given | not given | 1,836 lines open (92 files) | 599 sides open |
| 1 (this session) | `6987bc77` | 3157 passed, 6 failed, 54 ignored | (G2 set only) | (G2 set only) | 58,148/59,974 (1,826 uncovered) | 4,120/4,712 (592 sides) |
| 2 (dev reading) | `b6363918` | 3175 passed, 6 failed, 54 ignored | (reading stopped at 65 of 222 files) | | 58,736/60,515 (1,779) | 4,138/4,724 (586) |
| 3 (dev reading) | `e8ceb492` | 3177 passed, 6 failed, 54 ignored | (reading stopped at 103 of 222 files) | | 58,739/60,544 (1,805: 26 of them a lost child profile, host note) | 4,137/4,724 (587: 2 of them the lost child) |
| 4 (dev reading) | `02f268e2` | 3177 passed, 6 failed, 54 ignored | (reading stopped at 36 of 222 files) | | 58,740/60,545 (1,805: 26 of them the same lost child) | 4,138/4,724 (586: 2 of them the lost child) |
| **5 (final)** | **code tip `02f268e24f8bbf230099ed53ccdb8656aadf68d2`**, `kernel.pid_max` raised | 3177 passed, 6 failed, 54 ignored | **159,895/166,046 (6,151 uncovered)** | **11,903/14,346 (2,443 sides)** | **58,766/60,545 (1,779 uncovered)** | **4,140/4,724 (584 sides)** |

Run 5: ROUTE-B START 2026-11-05 10:24:45 IST, build to 10:25:36, tests to 10:29:09, every file read by 2026-11-05 10:53:53 IST; ROUTE-B DONE, exit 0; its pids rose from 15,960 to 54,684 without falling back (the old limit was 32,768), so none wrapped (host note), and every G2 row it leaves is one a class explains. From run 4 to run 5, on the
same code, G2's list lost exactly the lost child's 28 rows (27 in `alerts_ps2_tests.rs`, and `alerts.rs:1730:25
False`) and gained none.
Runs 1 and 5 are the kit script's own readings (`ROUTE-B DONE ... every package measured and read (pull)`, exit
0). Runs 2 to 4 were read for G2 by this session's parallel dev reader (the same merged profile, the same `llvm-cov show` flags and the same
`d3-covset.mjs`, four files at a time; for run 1 it printed exactly the kit script's figures, and for run 5 it
does again: the G2 totals of both readings are identical, file by file), and their kit readings were stopped once more rows were found, since a later run replaced
them. The instrumented suites fail exactly the six host reds below.

**Every one of G2's 2,363 uncovered rows at the code tip is classified** (`gaps-PS2.txt.md` lists each with
its class and text; `fu-PS2.txt.md` gives each (U) its proof and each (F) its reason):

| class | rows | what it is |
|---|---|---|
| (F) by pattern | 614 | a failing arm in test code: inside `panic!`/`unreachable!`, an assert's message rows, a `matches!`'s failing arm or an assert condition's one unreached side, a let-else whose else panics |
| (F) by hand | 9 | the last operand of an `||` inside an assert (its False side fails it), and test-only rows read by hand |
| (U) | 125 | unreachable by any input, file, folder, variable or direct call, each with its proof from the code (constant tables and widths, a value built in the same body, a callee that never answers that variant for this site's arguments) |
| (U, type invariant) | 21 | unreachable unless a unit test builds a private struct by hand against every constructor (`Day` past 9999-12-31, a hand-built `RunningSums`, `Bucket(0)`, `DayZips`, `CmScope`, `Reading`, `SlotTable`, `SourceCache`): marked for the operator, as the earlier session marked its 23 |
| IGNORED (L-BCP-3) | 1,594 | inside the 54 `#[ignore]` tests (real-data tests, and timings run by hand), or in a test helper only they reach: the operator's, never an exception |
| GAP | 0 | none: every row a test can reach is driven |

The (U) proofs were first drafted by four read-only analysis agents (Plan type: no edit, no build), one per batch of
the 216 rows run 1 left for reading by hand; this session spot-checked them against the code (the stub image's
widths for every vendor, the migration's split sums through `Manifest::partitions`, every production vendor session
row `Continuous` and opening at 555 on both sides of the fold's guard, `open_requires(.., true)` never answering
`Ok(None)`, the alert span's index), and each proof cites its lines. The Mac re-checks every proof for its target.

## What was done

### The earlier session (TOKEN `BASE-COV-PULL-PS2-20261103-1`, 3 Nov, commits `4f0ffe7c`..`f4fe72bf`)

42 commits, three workers on one host (this session and two helpers, forks B and C, in worktrees of their own,
cherry-picked here). 212 new tests (by `#[test]`, `git diff 2e7a0b6d f4fe72bf`) and edits to G2's test code, by file
group: `census_store` (36 tests on
its unreached arms, then 14 through test seams for its race windows), `alerts` (11, and one test of its shipped
walk seam, `tests/alerts_ps2.rs`), `emit_sites` and every G2 test file whose own code left a non-failing side
unreached (made the failing side or reached), the `gdfl_import` sidecars (`gdfl_import_filing`, `_quality`,
`_census`), `response_keep`, `zerodha_spool`, `landing_run`, `repair_plan`, `calendar`, `fno`, `chain`,
`gdfl_rekey`, `gdfl_run`, `gdfl_run_ledger`, `gdfl_cm`, `gdfl_v3`, `gdfl_dayzips`, `gdfl_format`, `fold`, `riskfree`,
`padded`, `daycheck` and `csv`, and the integration files `broker`, `zerodha_identity` and `cloud_mutants_import`.
Its test seams (D-7336, step form): `census_store::seam` (11 names), `response_keep` (`keep.missing`), `gdfl_rekey`
(`rekey.appending`), `gdfl_run_ledger` (`ledger.measured`), `repair_plan` (`repair.opened`, `repair.counted`),
`alerts` (`BEFORE_RENAME`), and the filing's test door (`inject::refuse_spawn`). At `c1ff1cc9` it measured G2 at
1,836 lines and 599 sides open and classified every one: (F) 610, (U) 162, IGNORED 1,597, and 66 not yet closed
(FAULT 35, HARNESS 24, HOST 4, RACE 3); its last commits drove 16 (U) rows a direct call reaches and the HARNESS
and HOST rows. Its lists did not survive the container.

### This continuation (TOKEN `BASE-COV-PULL-PS2-20261105-2`, 5 Nov, commits `6987bc77`..`02f268e2`, then the reports)

Route B run 1 on its first commit, every one of G2's 2,418 open rows classified (the pattern classifier for (F) and
the `#[ignore]` bodies; four read-only analysis agents drafted a class and a proof for each of the 216 rows left
for reading by hand, and this session spot-checked them against the code), and the 60 rows found reachable (33 by a host fault, 12 by an input or a
direct call, 1 by a race, 14 test-code arms) driven, each by a test that asserts its words or effects, and the 2
one-sided rows run 2 then found in this session's own path-keyed steps:

| commit | files (and their test files) | tests | rows driven |
|---|---|---|---|
| `a4494cf7` | census_store: a planted host fault (`seam::fail`/`answer`) at a segment entry's type, a kept census's measure, the requirements read, the seal's stub write | 4 | 16 |
| `f275750c` | gdfl_run_ledger: the ledger read's fault; landing_run (a seam of its own): the claim's unmeasured hold | 2 | 6 |
| `0c51a2c7` | zerodha_spool (a seam of its own) and gdfl_rekey: a `flock` the host refuses (`answer_lock`), at the live probe, the writer's live lock and the re-key's ledger lock | 3 | 8 |
| `027a8cd4` | response_keep: the first `try_lock` refused, the waiter's `flock` refused (a fault taken on the caller's thread and handed to the waiter), the waiter answering after its caller's wait ran out (a step at the timeout, a step keyed by the lock for the waiter's answer) | 3 | 5 |
| `ec06c852` | gdfl_run (a seam of its own): a census verb's claim write, the progress record's write | 2 | 3 |
| `734a49b7` | gdfl_import_filing: a file whose symbol names no instrument (direct call); a month whose folder another writer replaced before its confirm (a step in the test door) | 2 | 7 |
| `05c56c9c` | census_store: a single-file census at its 2,097,152-entry ceiling | 1 | 1 |
| `078db458` | census_store_kept_tests (the two runs meet a held lock every time), gdfl_rekey_tests (a seventh round one after the other; a cycle named from its earliest line), gdfl_backfill_tests (a fixture fact asserted), tests/store_evidence (a dropped replay's scratch) | 1 + 4 edits | 14 |
| `596cf383` | response_keep: the timed-out step sits in `Parked::abandon` (clippy `single_match_else`) | 0 | 0 |
| `b6363918` | census_store_tests: a check's comment says what it checks | 0 | 0 |
| `e8ceb492` | gdfl_import_filing_ps2_tests, response_keep_tests: the two path-keyed steps of this session wait for their own file and path (run 2 found one side of each unreached; this closed response_keep's) | 2 | 1 |
| `02f268e2` | gdfl_import_filing_ps2_tests: the door's step is also asked under another root (run 3: the lookup's branch is its first operand, the root) | 0 (1 edit) | 1 |

The 20 new tests, by name: `ingest::census_store::ps2_tests::{an_epoch_entry_whose_type_the_host_cannot_tell_is_refused_by_its_path,
a_kept_census_the_host_cannot_measure_is_not_current, a_requirements_file_the_host_cannot_read_refuses_the_census,
a_seal_whose_stub_cannot_be_written_leaves_the_census_in_place, a_single_file_at_its_ceiling_refuses_a_new_row_uncompacted}`;
`gdfl_run::ledger::ps2_tests::a_ledger_the_host_cannot_read_gives_no_id_and_writes_nothing`;
`ingest::landing_run::tests::a_claim_the_host_cannot_measure_is_taken_but_not_remembered`;
`zerodha_verify::spool::cov_tests::{a_live_probe_whose_lock_the_host_refuses_is_refused_by_name,
a_writer_whose_live_lock_the_host_refuses_appends_nothing}`; `gdfl_rekey::tests::ps2::an_apply_whose_lock_the_host_refuses_writes_nothing`;
`response_keep::tests::{a_pack_lock_the_host_refuses_is_refused_at_once_by_name,
a_waiter_that_answers_just_after_the_wait_ran_out_hands_over_the_lock,
a_waiter_whose_flock_the_host_refuses_refuses_its_caller_by_name}`; `gdfl_run::cov_tests::{a_census_claim_the_host_cannot_write_is_refused_by_its_file,
a_progress_write_the_host_refuses_is_kept_by_name_and_not_counted}`; `gdfl_import::filing::ps2_tests::{a_file_whose_symbol_names_no_instrument_is_the_filings_own_refusal,
a_month_whose_folder_was_replaced_before_its_confirm_keeps_the_day_open}`; tests/store_evidence.rs
`a_dropped_replay_removes_its_scratch`; `gdfl_import::filing::ps2_tests::the_door_s_step_before_a_confirm_waits_for_its_own_file`;
`response_keep::tests::a_step_planted_for_a_path_waits_for_its_own_name_and_path`.

No production behaviour changed: every seam module and call site is `#[cfg(test)]` (D-7336), and the one
production-file edit outside a seam is the `Parked::abandon` hook's move (also `#[cfg(test)]`). No test was
weakened, deleted or `#[ignore]`d; no `#[ignore]` test was touched.

## Red checks (every new test, both sessions)

The earlier session's report left "my red checks" undone, and only one of its commits recorded one. So every new
test of both sessions was red-checked here: one hand change to the production arm the test drives (for a test of a
test fixture, seam or door, to that fixture, seam or door), the run, the failure read (it must be the test's own
assertion about that arm), then the change undone (`git checkout`). **234 tests, 234 red, 0 not red.**

- **212 of the earlier session**, by two helper agents, each in a git worktree of its own detached at `6987bc77`
  with a target dir of its own (`/home/user/ps2-work/red1`, `red2`), batching only changes in different functions,
  each test run alone: R1 102 tests in 13 builds (69 minutes), R2 110 tests in 10 builds (64 minutes). Both left
  their worktrees clean; this session then removed the worktrees and their local branches (never pushed).
- **22 of this continuation** (20 new tests and the 2 edited ones that state something new), by this session in
  five builds of the main checkout, after each commit.

Each row (test, hand change, failure) is in `red-PS2.txt.md`.

## Gates (stable 1.97.1, this host, on the code tip `02f268e24f8bbf230099ed53ccdb8656aadf68d2`)

Run at `a4cea3dd`, which changes only `reports/` after the code tip, so the tree built is the code tip's.

- `cargo fmt --all --check`: rc 0.
- `cargo clippy -p pull --all-targets --locked -- -D warnings`: rc 0. (An earlier run, before `596cf383`,
  found one pedantic `single_match_else` in this session's own seam, the timeout arm made a block; `596cf383` fixed
  it by moving the hook into `Parked::abandon`, which restores that match's text.)
- `cargo test -p pull --locked --no-fail-fast`: 33 binaries, 3177 passed, 6 failed, 54 ignored (the whole gate run, fmt to the uid-65534 check, 10:33:09 IST to 10:38:43 IST). The six failures are G3's root-only
  tests, the host reds PS1 named, none in a G2 file:
  `gdfl_dropbox::tests::repair_r1::a_journal_that_cannot_be_written_stops_the_press_before_any_byte`,
  `gdfl_dropbox::tests::repair_r1::a_read_only_store_root_is_not_writable_not_lost`,
  `gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name`,
  `gdfl_run::ia2_tests::every_id_the_runs_ledger_gives_is_written_down_twice`,
  `zerodha_verify::tests::a_refused_open_keeps_the_hosts_words`,
  `zerodha_verify::tests::a_write_beside_a_merge_that_fails_is_named`. **Proof (CLOUD.md item 8):** the same lib
  test binary run as uid 65534 (`runuser -u nobody -- env TMPDIR=/dev/shm/ps2-tmp <lib test binary> --exact <the
  six>`, cwd `crates/pull`): ok. 6 passed; 0 failed; 0 ignored; 0 measured; 2687 filtered out; finished in 0.01s. Every Route B run's instrumented suite fails the same six and no other.
- Benches: none run (no step of G2 is a timed one; gate 8 is the Mac's).

## Lists (each one fenced block, `.txt.md`)

- `reports/base-cloud-20261103/gaps-PS2.txt.md`: run 5's per-file table of all 222 files of the set (G2's marked),
  the whole-pull and G2 totals, and every uncovered line and side of G2's 93 files with its class and text.
- `reports/base-cloud-20261103/fu-PS2.txt.md`: G2's (F)/(U) list: every (U) and (U, type invariant) row with its
  proof from the code, every (F) row with its reason, the GAP rows (none), and the IGNORED rows' counts by file.
- `reports/base-cloud-20261103/red-PS2.txt.md`: every new test of both sessions with its hand change and the line
  it failed at (234 rows).
- `reports/base-cloud-20261103/fix-BASE-COV-PULL-PS2.json.md`: the FIX report.

## Decision draft (D-7336, this session's id; PSA writes the docs entry)

### D-7336 (draft, BASE-COV-PULL-PS2): an arm only another writer or a failing host reaches is driven through a test seam, never excused

**Context.** Recipe v4 step 4 counts a production refusal or error arm as a GAP until a test drives its input, and
"needs a fault", "a race" or "hard to reach" is never a (U) proof. Some arms of G2's files are reached only when
another process changes the store between two host calls (a rename, a cut, a name made or replaced), or when the
host refuses a call that no file, folder or name can make fail (the measure of an open descriptor, a read or write
of a regular file, a `flock` other than contention, a thread spawn, a directory entry's type).

**Decision.** Each such arm is driven by a test through a seam at its call site, in one of two forms:

1. **A step between two host calls** (the pattern of `ingest::census_store::seam`): `seam::plant(name, step)` and
   `seam::at(name, path)`. A test plants one closure under a name; the next pass of that name on the same thread runs
   it once, with the path the code is at, as another writer landing between the two calls would. In `census_store`
   (11 names), `response_keep` (`keep.missing`, `lock.timed_out`), `gdfl_rekey` (`rekey.appending`),
   `gdfl_run_ledger` (`ledger.measured`), `repair_plan` (`repair.opened`, `repair.counted`) and `alerts`
   (`BEFORE_RENAME`). A step for a call another thread makes is keyed by the test's own path, not by thread:
   `response_keep`'s `plant_for`/`at_path` (`lock.answered`, the pack lock's waiter) and the filing's test door
   `inject` (`plant_before_confirm`, beside its planted swaps and answers).
2. **A planted host fault**: `seam::fail(name)` and `seam::answer(name, host)` (`answer_lock` for a call that
   answers `TryLockError`; `taken` to hand the fault to the thread that makes the call). The call is made, and its
   answer is the host's error instead, once. In `census_store` (`segment.typed`, `kept.measured`, `reader.read`,
   `stub.written`), `gdfl_run_ledger` (`ledger.read`), `landing_run` (`claim.measured`), `zerodha_spool`
   (`live.probed`, `live.taken`), `gdfl_rekey` (`ledger.locked`), `response_keep` (`lock.tried`, `lock.waited`) and
   `gdfl_run` (`claim.written`, `progress.written`); and the filing's `inject::refuse_spawn` (the host's refusal of
   a worker thread).

**Rules.** Every seam module and every call site is `#[cfg(test)]`: a build without tests holds no seam line and
behaves exactly as before. (The one shipped seam is `alerts`' walk seam, whose `#[cfg(not(test))]` twin does
nothing and is run by `tests/alerts_ps2.rs`, the library as `api` and `cli` link it.) Steps and faults are per
thread, or keyed by a store path of the test's own, so tests running at once never meet each other's; each is taken
once; production code never plants one. A seam changes no behaviour of the code it sits in: it lets a test choose
what another writer or the host does at one point, and every test that uses one asserts the arm's own words or
effects and then shows the same call, unplanted, answering as before.

**Consequence.** G2's FAULT and RACE rows are driven and asserted, not listed as exceptions. A new arm of this
kind gets a seam and a test. The figures are the report's (Route B, G2 before and after).

## Follow-ups and notes (none blocks this group)

1. **A test that does not pin the guard it is named for** (for the deferred mutants pass): R1 found that
   `gdfl_run::ledger::ps2_tests::a_claims_line_too_long_for_a_record_is_never_read` still passes with `claimed()`'s
   `if body.len() > BODY { return None; }` (`gdfl_run_ledger.rs:527-529`) removed: such a line fails `Entry::of`
   anyway (byte 126 is not a space, or the CRC never matches). Its red check came from another change, in
   `claims_tail`. The guard is redundant for every line this reader can be handed; a mutant there would survive.
2. **Whole-pull figures are for reference**: the bar this group answers for is G2's 93 files; the rows of other
   groups' files are theirs (and some of them are now covered by G2's tests, which no figure here claims).
3. **The `#[ignore]` tests** (L-BCP-3) are untouched; their bodies, and the helpers only they reach, are the
   IGNORED rows. G2's files hold 23 of the crate's 54: 11 need the operator's real data (a store or archive named
   by a variable: `tests/store_evidence.rs` 5, `gdfl_ia3_tests` 2, `gdfl_v3_tests`, `fnowork_futures_tests`,
   `disk_projection_tests`, `census_store_kept_tests` 1 each), and 12 are timings or measurements on invented data,
   ignored for their run time (`gdfl_run_tests` 3, `gdfl_rekey_tests` 3, `landing_run_tests`, `gdfl_dayzips_tests`,
   `census_store_tests`, `census_store_kept_tests`, `calendar`, `alerts_tests` 1 each): a coverage pass that also runs
   those 12 with `--ignored` would count their rows, which is the operator's choice under L-BCP-3. A store-free test now
   drives one real-data helper's cleanup (`store_evidence`'s `Replay` drop), because it needed no real data.
4. The helpers' worktrees were made and removed in this session (`.claude/worktrees/`, branches
   `worktree-agent-*`, never pushed), and helper R2 removed the 42 scratch folders and one socket link its own red
   runs left in this session's TMPDIR (only names carrying the pid of its own test processes); nothing else was deleted,
   and nothing outside G2's files and the reports was edited (`git diff --name-only 2e7a0b6d HEAD` lists only G2
   files and `reports/`).

## NEEDS THE MAC

Route B whole:pull on its pin (aarch64-apple-darwin; every (U) proof above is stated for it and re-checked there),
the shadow GET re-check, macOS rows (this Linux host cannot show them; the Mac's run does), non-root 14-core
tests (the six G3 host reds above, and the uid-65534 children here), the 54 ignored tests (L-BCP-3: real-data tests,
and timings run by hand), the D1 landing. PSA writes D-7336's docs entry from the draft above.
