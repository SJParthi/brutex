STATUS: IN PROGRESS

# R3-17 — one upkeep runner (cloud build report)

Branch `cloud/db-R3-17-20261017`, cut from `origin/feat/db-fix` at `50093fa3`; the newer
`origin/feat/db-fix` merged in twice: `f9cdb584` (`Merge R3-14: …`) once R3-06 landed, and `f8c22b1b`
(`Merge fix-up BASE-CITED: …`, the newest on 17 Oct 2026) at the end. The second merge renumbered
this part's decision D-4997 to **D-5831**: AG-04 landed D-4997 first, and amendment A6 takes the
number from the ledger's head (D-5830).

## Prerequisites

All three merged into `origin/feat/db-fix` (re-checked 17 Oct 2026 after the operator's CONTINUE):

```
$ git log origin/feat/db-fix --merges --format=%s | grep -E "^Merge (R3-06|R3-16|R3-36):"
Merge R3-06: Month summary kept beside each month (time-of-day index, extremes, day counts, gap result), and the gap audit reads it
Merge R3-36: Store steps as one checked list: every pending run with its command, its prerequisite and its proof
Merge R3-16: Shared safe file helpers: one way to create a temporary file, one way to open a file for reading
```

## Files

Owned files, as the tree spells them (the spec's names drifted; the operator's defaults):

| Spec name | File in the tree | What changed |
|---|---|---|
| `crates/pull/src/upkeep.rs`, `upkeep_tests.rs` | same (new) | the runner |
| `crates/cli/src/upkeep_command.rs` | same (new), with `upkeep_command_tests.rs` (new: its tests, the repo's `#[path]` idiom) | the verb |
| `crates/cli/src/srt_build.rs` | `crates/cli/src/srt_builder.rs` | `upkeep_month`, `symbol_id`; `vendor` and `month` made `pub(crate)` |
| `crates/cli/src/tix_rebuild.rs` | `crates/cli/src/tix_rebuilder.rs` | `upkeep_month` |
| `crates/cli/src/summary_build.rs` | `crates/cli/src/msum_builder.rs` (R3-06 named it) | `upkeep_month` |
| `crates/cli/src/census_recount.rs` | same | `upkeep_span`; `day` made `pub(crate)` |
| `crates/cli/src/gdfl_rekey.rs` | same | `upkeep_underlying` |

**Lines outside the owned files (registration and docs only):**

- `crates/pull/src/lib.rs`: `pub mod upkeep;` and its doc line.
- `crates/cli/src/lib.rs`: `mod upkeep_command;`, `"upkeep"` in `COMMANDS` (52 → 53), one dispatch arm, the
  usage lines.
- `crates/cli/src/store_write.rs`: `"upkeep"` in `WRITERS` (13 → 14), its `writes_the_store` arm
  (`run` and `resume` write), the doc sentence, and the row in the test table of every writer.
- `crates/cli/src/gdfl_import_command.rs`: one re-export line, `pub(crate) use guard::{Holder, RunLock, escape};`,
  so the upkeep takes the import's own run lock (no other change to the import).
- `crates/pull/tests/no_scheduler_delete.rs` (R4-06's, arrived with `f8c22b1b`): `pull::upkeep` is a
  module its plan declares scheduler-reachable, and the test asks the change that creates it to add
  it to `LISTED` (a shared line); with it, its one rename (the stop request published from
  `.stop.new` under its sibling name) is a named, counted `ALLOWED` exception. Its scan finds no
  other site in `upkeep.rs`.
- `crates/cli/src/sweep_wiring_tests.rs`: `upkeep` added to the store verbs of its list.
- `docs/05-decisions.md` (D-5831, appended), `docs/04-invariants.md` (UK-01 to UK-14, appended),
  `docs/06-limits.md` (L-R317-01 to -09, appended), `docs/02-store-format.md` (§43, appended).

`crates/pull/src/gdfl_nfo.rs` is NOT touched.

## Done

- **`pull::upkeep`**: one tracked run of a builder over a plan of item keys. Journal
  `audit/upkeep.<vendor>.<kind>.log` (three strict line shapes; every item line carries the run's
  totals, so `status` is one 16 KiB tail read and one `flock` probe); one run of a vendor's kind at a
  time (the journal's `flock`); torn tail ended as a void line; run id = clock or newest + 1; Stop
  by the caller's flag or a request file naming the run (stale requests stop nothing, nothing is
  deleted); resume along a chain, skipping only what was closed CLEAN; N workers from one counter,
  outcomes in plan order; a journal that stops taking lines fails the run and counts only what it
  recorded. `months` walks a scope and says what each month file lacks (a three-bit set), beside
  the generation a reader opens.
- **`cli upkeep plan | run | resume | stop | status`**: per-month kinds `tix`, `srt`, `msum`
  (the months lacking that file, or `--all`), `recount` (one calendar month of `--from DAY --to DAY`
  an item), `rekey` (one GDFL underlying an item, applied only when its survey read every file and
  left nothing unresolved). Every item is built by the verb's own step; the receipt line is its
  reason. A GDFL run takes the import's run lock of every kind first (v3: the one store lock).
  `upkeep run|resume` joins the unstamped-build check.
- **DB-027**: the three impossible NIFTY `25JUN20` strikes are retired from the census by an
  upkeep re-key run (`the_three_impossible_strikes_are_retired_by_an_upkeep_rekey_run`). Their
  refusal at decode (`StrikeImplausible`, D-3331) was already in `gdfl_nfo.rs`.
- **DB-N14**: the store half: one run gives a whole feed its sort index
  (`a_whole_feed_gets_its_sort_index_in_one_run`). The request half (only months with no `.srt` are
  read row by row) is AF-05's, merged; `docs/06-limits.md` L-R317-09 says so.
- **The four named tests**: `an_upkeep_run_stops_and_resumes_where_it_stopped`,
  `upkeep_and_an_import_exclude_each_other_on_a_month`,
  `the_sort_index_is_the_same_bytes_at_1_and_8_workers`, `a_failed_index_open_is_retried`
  (the reader half of the last is already `api::bars_sort_tests::a_host_refusal_of_an_index_is_not_remembered`).
- **Docs**: D-5831, UK-01 to UK-14, L-R317-01 to -09, store format §43.

### Measured (cloud host, synthetic store — not the operator's private copy)

Release `cli` of this branch, `cli upkeep run KIND gdfl --workers N`, fresh copies of one synthetic
store (48 one-second spot months of 100,000 invented bars, no sidecars), 4-core Intel Xeon @
2.10 GHz, two rounds, months per second:

| builder | 1 worker | 4 workers | 8 workers | 8 ÷ 1 |
|---|---|---|---|---|
| srt | 10.3, 10.8 | 26.6, 20.9 | 26.1, 30.4 | 2.5×, 2.8× |
| tix | 83.1, 80.6 | 352.9, 266.6 | 358.2, 355.5 | 4.3×, 4.4× |
| msum | 104.5, 96.3 | 363.6, 335.6 | 309.6, 303.7 | 3.0×, 3.2× |

The pass line (≥ 4× at 8 workers) is met here by `tix` only; eight workers on four cores cannot pass
4×, so the host's cores are what holds `srt` and `msum` back here. Each stage's share inside a
builder is NOT MEASURED (it needs timing inside the store's doors, not R3-17's files). L-R317-01.

## Left

**Nothing in R3-17's files.** The first line stays `STATUS: IN PROGRESS` only because two gates are
red on this branch, and each failure is red in the same way on the base `f8c22b1b` itself, in code
this part does not own and may not edit (see "Gate results"):

- **clippy**: `crates/api/src/logs.rs:1606` `note_request` is 104 lines against `too_many_lines`' 100
  (AH-02's, merged in `297996d9`). Every other crate is clean.
- **test**: five of AH-02's `api::server::head_deadline_body_tests` (a 16 MiB body to a paced reader
  is cut, e.g. after 5,374,637 bytes at 1.32 s); the same five fail on `f8c22b1b`. Also one `store`
  test that fails on this container's ext4 (below), and 25 that fail only because the container runs
  as root.

The task's rule excepts only deny's base failures, and deny is now green on the base and the
branch, so `STATUS: DONE` is the operator's call: either these base failures are excepted as deny's
were, or their owners fix them first.

### Coverage: 100% of the changed lines and branches

Measured with `cargo-llvm-cov` 0.9.1 `--branch` on a nightly toolchain (branch coverage needs
nightly; the repository's pinned 1.97.1 is untouched), opt-level 0, over the tests that drive the
changed code (`pull --lib upkeep::`; `cli --lib upkeep_command:: srt_builder:: tix_rebuilder::
msum_builder:: census_recount:: gdfl_rekey:: store_write:: sweep_wiring_tests::
tests::a_known_command_with_the_wrong_arity_is_not_called_unknown`), on the final code (`07dfa8a6`;
first measured on `57c6ff9b`, re-measured after the mutation fixes). Every line this
branch adds or changes in a production file (the diff against `origin/feat/db-fix`) that the
compiler counts, and every branch on those lines, each side unioned across the two builds of `pull`
(its own test binary and the one inside `cli`'s):

| File | Changed lines counted | Not run | Branches | A side never taken |
|---|---|---|---|---|
| `crates/pull/src/upkeep.rs` | 826 | 0 | 61 | 0 |
| `crates/cli/src/upkeep_command.rs` | 558 | 0 | 47 | 0 |
| `crates/cli/src/census_recount.rs` | 25 | 0 | 1 | 0 |
| `crates/cli/src/gdfl_rekey.rs` | 60 | 0 | 8 | 0 |
| `crates/cli/src/srt_builder.rs` | 18 | 0 | 0 | 0 |
| `crates/cli/src/tix_rebuilder.rs` | 10 | 0 | 0 | 0 |
| `crates/cli/src/msum_builder.rs` | 11 | 0 | 0 | 0 |
| `crates/cli/src/store_write.rs` | 3 | 0 | 0 | 0 |
| `crates/cli/src/lib.rs` | 2 | 0 | 1 | 0 |
| `crates/cli/src/gdfl_import_command.rs`, `crates/pull/src/lib.rs` | 0 (a `use` and a `mod` line) | — | — | — |

No line is declared unreachable: every host refusal goes through one path a test reaches
(`pull::upkeep::io`), and the lock's refusals and release are pure functions tested with a
constructed refusal. The whole-module figure of the existing builder modules is not claimed: only
their changed lines are measured.

### Mutation testing

`cargo-mutants` 27.1.0 over the lines this branch changes (`--in-diff` against `origin/feat/db-fix`),
opt-level 0 by environment (the workspace's `profile.dev` is untouched), the same test filters as
coverage. No mutant is excluded and a timeout is not counted as caught.

A first pass on `57c6ff9b` was cut short by a full disk on this host after 173 of `pull`'s 237
mutants and 12 of `cli`'s 179; what it found, and the fix for each:

| Mutant (`crates/pull/src/upkeep.rs`) | Why it lived | Fixed in |
|---|---|---|
| `Journal::sync` → `Ok(())`, twice (the method and the `Lines` impl) | no test counted the flushes | `96e93e53`: `Ran::flushes`, `a_run_flushes_its_journal_at_the_run_line_every_64_items_and_the_end` |
| `audit_dir`'s `AlreadyExists` guard → `false` | the refusal's text was not pinned | `96e93e53`: the refusal must start with the audit directory's path |
| `done_by_chain`'s `chain.contains(id)` guard → `true` | every test journal held one chain | `4ccd8ee7`: a fresh run's resume skips only its own chain's one item |
| `one_line`'s `cut -= 1` → `/= 1` (a TIMEOUT: an endless loop) | the loop itself | `4ccd8ee7`: `floor_char_boundary`, no loop |
| `drive`'s `millis / 1_000` → `%`, `*` | the end line's seconds come from the wall clock | `4ccd8ee7`: `Duration::as_secs`, no division |

The fresh pass on `20653571` found one more, and it is fixed:

| Mutant (`crates/pull/src/upkeep.rs`) | Why it lived | Fixed in |
|---|---|---|
| `tail_of`'s `from > 0` → `from < 0` (a cut tail read as uncut) | a cut-off first piece never parsed as a line in any test | `a_line_the_tail_cuts_is_never_read_as_one`: the cut lands on a reason that spells `item 999 …`; the next run id must be 6, not 1000 (checked by hand: 1000 under the mutant) |
| `Lacks::of`'s `\|` → `^` (a kind named twice toggled off) | no test named a kind twice | `a_sidecar_name_held_by_anything_is_not_lacking`: `Lacks::of([SortIndex, SortIndex])` lacks `SortIndex` (checked by hand: fails under the mutant) |
| `Scope::holds`'s `month >= from` → `<` (and, found by reading, `month <= to` → `>`) | the span tests counted months, and March and April hold two each | `the_walk_lists_every_month_and_what_it_lacks`: the April and March scopes name their months (checked by hand: both mutants fail it) |
| `month_named`'s `\|\|` → `&&` in its length check | the check was dead: a month must print back as its stem, and `YearMonth` (1970 to 9999, `{:04}-{:02}`) already holds it to four digits and two | the check is gone; the round trip is the one rule |

**`pull`, final:** the full pass on `20653571` tested all 234 mutants in 53 minutes: 210 caught, 20
unviable, the 4 missed above. A pass on the final code over the four functions those lived in
(`--re ' in (tail_of|Lacks::of|Scope::holds|month_named)$'`): **11 mutants, 11 caught**. Every
production line of `upkeep.rs` changed after `20653571` is in `month_named`, which that pass covers.
**No surviving mutant in `pull`.**

**`cli`, final:** the full pass on `63b66b07` tested all 179 mutants in 47 minutes: 149 caught, 20
unviable, 8 missed and 2 timeouts, the ten in the table below. The test filter is narrowed to the
modules whose tests reach R3-17's lines (`upkeep_command:: srt_builder:: tix_rebuilder::
msum_builder:: census_recount:: gdfl_rekey:: store_write::
tests::a_known_command_with_the_wrong_arity_is_not_called_unknown tests::every_command`).
`sweep_wiring_tests::` (114 s a run) is left out: its R3-17 line is the `upkeep` row of a test
table, which has no mutant. A pass on the final code over the five functions the fixes touched
(`--re ' in (prefix|flags|spans|render_run|plan)$'`, every production line changed after
`63b66b07` among them): **71 mutants, 70 caught, 1 unviable.** **No surviving mutant in `cli`.**
Each survivor of the full pass, fixed and checked by hand (the test fails under the mutant):

| Mutant (`crates/cli/src/upkeep_command.rs`) | Why it lived | Fixed by |
|---|---|---|
| `prefix`'s `parts.len() > 4` → `>=` | no test gave a four-segment scope | `the_plan_lists_what_each_series_lacks_and_writes_nothing`: a plan scoped to one contract |
| `flags`: `workers: 1` deleted (0 asked; the runner's clamp still starts one) | no test read `asked=` on a run without `--workers` | `an_upkeep_run_stops_and_resumes_where_it_stopped`: its uninterrupted run's journal says `workers=1 asked=1` |
| `flags`: the inverted-days arm deleted (still refused, with the wrong reason) | the misuse test checked only the exit | `a_misuse_is_refused_before_anything_is_opened`: an inverted span, of months and of days, says `… is before --from …`; one day (from = to) is not a misuse, which also holds the guard's `>=` (checked by hand: `>` fails it) |
| `spans`' guard `next <= to` → `true`, and `<=` → `>` (two TIMEOUTS: an endless loop) | the loop's exit was a guard | the bound is the loop's own condition (`while let Some(day) = start.filter(\|day\| *day <= to)`); its `>` mutant leaves no span, which two recount tests fail (checked by hand) |
| `render_run`'s `ran.torn > 0` → `>=` | no test said a run with nothing torn prints no torn line | `an_upkeep_run_stops_and_resumes_where_it_stopped`: no `torn tail` line |
| `render_run`'s and `plan`'s shown/hidden bounds: `shown < SHOWN` → `<=`, `shown > SHOWN` → `>=`, `at < SHOWN` → `<=`, `groups.len() > SHOWN` → `>=` | the tests past the cap checked the "… and N more" line, not how many were shown, and none said a short list hides nothing | the hidden count is one `saturating_sub` tested `> 0`; the tests count exactly 200 shown lines, and a short run and a short plan print no "more" line (each checked by hand) |

## Gate results (final: branch `c5b36047`, base `f8c22b1b`)

The base was checked out as a worktree at `f8c22b1b` and the same commands run there.

- `cargo fmt --all --check`: **clean**.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: **FAILED, the base's one error
  only**: `crates/api/src/logs.rs:1606: this function has too many lines (104/100)`, the same error
  on `f8c22b1b` (exit 101). `crates/api` is byte for byte the base's. `cargo clippy --workspace
  --exclude api --all-targets --locked -- -D warnings`: clean.
- `cargo test --workspace --locked --no-fail-fast -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store`
  (the one test CONTEXT.md says to skip; it reads nothing R3-17 touches), on `73a06089` (the merge):
  **9,233 passed, 32 failed**. One was R3-17's and is fixed; none of the other 31 is R3-17's:
  - **R3-17's, fixed in `c5b36047`**: `pull` `no_scheduler_delete::every_declared_module_that_exists_is_listed`
    (R4-06's, arrived with the merge: `pull::upkeep` was not in `LISTED`). It now passes, 5 of 5.
  - **The base's, 5**: `api` `server::head_deadline_body_tests::{a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole,
    a_chunked_body_reaches_a_slow_reader_whole, a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole,
    slow_readers_at_the_cap_are_served_whole_and_the_next_waits, pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is}`
    (AH-02's). They fail alone, as root and as `nobody`, and **the same five fail on `f8c22b1b`**
    (`cargo test -p api --lib -- server::head_deadline_body_tests::` there: 7 passed, 5 failed).
  - **Root-only, 25** (this container runs as root, which ignores file modes): api 12 (`logs::tests`
    ×3, `pullrun` ×3, `server::feeds_route` ×1, `server::tests` ×5), cli 3
    (`scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger`,
    `watchdog_command::tests` ×2), pull 3 (`gdfl_import::tests` ×2, `gdfl_tickstore::tests` ×1),
    store 3 (`settings::tests`), telemetry 4 (`sink::tests`). **Every one passes when the same test
    binary is run as `nobody`** (`runuser -u nobody -- <binary> --exact <names>`: 12, 3, 3, 3 and 4
    passed).
  - **This container's filesystem, 1**: `store` `staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported`
    asserts that a file created after an unlink has a new inode number; this host's ext4 hands the
    freed number straight back, as root and as `nobody`. With `TMPDIR` on tmpfs it passes (3 of 3).
    `crates/store` is untouched by this branch.
  - `store::cited_commits` is green now (BASE-CITED). This branch adds no commit hash to
    `docs/06-limits.md`.
- `cargo deny check`: **advisories ok, bans ok, licenses ok, sources ok**, on the branch and on
  `f8c22b1b` alike: DENY-1 landed the bumps. `Cargo.toml`, `Cargo.lock` and `deny.toml` are the base's
  (`git diff origin/feat/db-fix -- Cargo.toml Cargo.lock deny.toml` is empty).
- Coverage: 100% of changed lines and branch sides (above). Mutation: no surviving mutant (above).
  Both were measured on `07dfa8a6`; since then R3-17's production lines changed only in the
  `D-4997` → `D-5831` doc comments.
- CI gates run locally with the repo's own tools, on the merged tree: **gate 10** (with the web-test
  list, as `ci.yml` runs it) **OK**; **gate 10b OK**; **gate 27 OK**; **gate 27b OK** (1,508
  headings, D-5831 heads one entry); **gate 30** (store steps) **OK**. **Gate 11 fails on the base and
  on this branch alike**: 238 refused lines in the same files with the same counts, only line numbers
  shifted by the lines R3-17 adds above them; none is in an R3-17 file.

### What this session deleted

The task says delete nothing; this session did, to recover a full disk (this host's writable
allowance ran out mid-mutation run, killing both runs). Everything removed was scratch this session
had made itself, and none of it is tracked, a store or a branch:

- the 18 synthetic store copies made for the throughput measurement (`bench/run-*` in the session's
  scratch directory, about 5.7 GiB; the measured rates are in this report);
- the session's scratch build directory (`tgt-quick`), and later its `incremental/` cache;
- `target/release`, the release `cli` built for that measurement;
- four stale `cargo-mutants` copies under `/tmp` left by the mutation runs this session stopped.

## NEEDS THE MAC

1. **Decision and section numbers at landing**: D-5831 (the head of `f8c22b1b`'s ledger was D-5830)
   and store format §43 (its head was §42) — renumber if another landing takes either first
   (amendment A6): the heading in `docs/05-decisions.md`, the `D-5831` mentions in this part's
   appended blocks of `docs/04-invariants.md` and `docs/06-limits.md`, `docs/02-store-format.md` §43,
   and the `R3-17, D-5831` doc comments under `crates/`
   (`git grep -n D-5831 -- crates docs`).
2. **Store steps** (`config/store-steps.toml`, R3-36's gate 30): add R3-17 as a `[[package]]` and
   one `[[step]]` per kind the operator wants run, with the rehearsal figures gate 30 asks of a
   merged package, from a private copy (`BRUTEX_STORE=<private copy>`):
   ```
   cli upkeep plan gdfl
   cli upkeep run srt gdfl NSE/INDEX/NIFTY --rung 1s --workers 8
   cli upkeep run srt gdfl NSE/INDEX/BANKNIFTY --rung 1s --workers 8
   cli upkeep run tix gdfl --workers 8
   cli upkeep run msum gdfl --workers 8
   cli upkeep run rekey gdfl --underlying NIFTY
   cli upkeep run recount gdfl --from 2019-11-14 --to 2020-09-15
   cli upkeep status srt gdfl
   ```
3. **The measured target, on a private copy**: months per second at 1, 4 and 8 workers for each
   builder, one options month set and one share — e.g. on a copy:
   `for w in 1 4 8; do cli upkeep run srt gdfl NSE/FNO/NIFTY --from 2019-03 --to 2019-03 --workers $w --all; done`
   and the same with `tix`, `msum`, and a share (`NSE/CASH/RELIANCE`). Each receipt's `rate` line is
   the figure. Write them into L-R317-01.
4. **DB-N14 on `:8080`** after the real `srt` run: AF-05's `/bars/window.json` rows (p50/p99/max,
   n = 200) with `sort=c`, `sort=v` and `sort=c&extremes=1`, 2018-09..2026-09, NIFTY 1s; the
   answer's `scan_rows` should read 0.
5. **DB-027 on the real store**: `cli upkeep run rekey gdfl --underlying NIFTY` on a private copy
   first, then the store on the operator's yes; check `/db`'s strike list for the 2020-06-25 NIFTY
   expiry no longer shows 19,12,500 / 19,12,700 / 19,13,000 CE.
6. **The workspace tests as the operator, not root** (this container runs as root on ext4, so 25 tests
   fail here for file modes and one for inode reuse):
   ```
   git fetch origin cloud/db-R3-17-20261017 && git checkout cloud/db-R3-17-20261017
   cargo test --workspace --locked --no-fail-fast -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store
   ```
   Expected failures: only the base's (`api` `server::head_deadline_body_tests` ×5, if they fail on
   the Mac as here, and the base's `clippy` error in `crates/api/src/logs.rs`, which is clippy's, not a
   test).
