# Cloud mutation and coverage pass: four cli files, 2026-10-16

Branch `cloud/mut-cli-20261016` (draft PR SJParthi/brutex#78), based on
`origin/cloud/base-20261016` (`c9e3472e`, the one-commit snapshot of
`feat/db-fix` at `4756cce0`). The target is `CLAUDE.md` §9 for exactly these
four files, which changed on `feat/db-fix`:

- `crates/cli/src/gdfl_import_command.rs`
- `crates/cli/src/readonly_file.rs`
- `crates/cli/src/srt_builder.rs`
- `crates/cli/src/tix_rebuilder.rs`

**Tests only. No production line changed.** Each file gained one
`#[cfg(test)] mod cloud_mutant_tests` at its end: 20 tests in all (gdfl 5,
readonly 1, srt 11, tix 3). `lib.rs`, `store_write.rs` and
`alerts_command.rs` were not touched, and no cli verb was added.

Every scoped item is `pub(crate)` inside a private module (`mod
gdfl_import_command;` and the rest in `lib.rs`), so nothing in scope can be
reached from an integration test. `crates/cli/tests/cloud_mutants_import.rs`
was therefore not created.

Commits:

| Commit | What |
|---|---|
| `92113a6` | the tests that kill the survivors and cover the reachable lines |
| `ad45a98` | a stale single run, a stray entry alone, compound asserts split |
| this one | this report |

No real bug was found. No vendor byte is used anywhere: the one archive a
test reads is a zip built at run time from invented prices.

## How it was measured

| | |
|---|---|
| tools | cargo-mutants 27.1.0, cargo-llvm-cov 0.9.1, rustc 1.97.1 (the pinned toolchain) |
| mutants per file | `cargo mutants --in-place --file <f> --shard k/n --cargo-arg=--lib --cargo-arg=--locked --minimum-test-timeout 120 -- -- <file>::` |
| shards | tix 0/1, readonly 0/1, gdfl 0/2 and 1/2, srt 0/3, 1/3 and 2/3; each time-boxed with `timeout 1500` (25 min); none hit the box |
| test filter | each file's own module name (`gdfl_import_command::`, `readonly_file::`, `srt_builder::`, `tix_rebuilder::`), which runs both its existing `tests` module and the new `cloud_mutant_tests` |
| build profile | **mutation and coverage runs only:** `CARGO_PROFILE_{DEV,TEST}_OPT_LEVEL=0` and `CARGO_PROFILE_{DEV,TEST}_INCREMENTAL=true` as environment overrides, with a separate target dir. No file changed. The repository's dev profile (opt-level 3, incremental off) rebuilds this 329k-line crate from scratch for every mutant: a cli test build measured 5m52s and 7m49s here. At opt-level 0 an incremental rebuild takes 12 to 33 s. The gates below ran on the repository's own profile. |
| "before" | the base commit `c9e3472` |
| "after" | the full pass ran on `92113a6`. The survivors it left, and every other mutant at the same source sites, were then re-run on the final test head `ad45a98` (table below). |
| coverage | `cargo llvm-cov -p cli --lib --locked --lcov -- <the four filters>`. Branches come from the same command with `--branch`, which is unstable on this stable toolchain and was enabled with `RUSTC_BOOTSTRAP=1`. Read the branch figures as measured with that flag, not as a stable-toolchain report. |

The shard run was killed once, by a 30-minute background limit in this
session, partway through gdfl 1/2 (before). That shard's partial output was
discarded and the shard re-run whole. The tree was checked clean after the
kill; see the artifacts note under "Real bugs".

## Mutants per file

Counts are mutants total / caught / missed / timeout / unviable. "Missed
before → after" is the base commit against the final test head.

| File | Total | Caught before → after | Missed before → after | Timeout | Unviable |
|---|---:|---:|---:|---:|---:|
| `gdfl_import_command.rs` | 62 | 49 → 53 | **4 → 0** | 0 | 9 |
| `readonly_file.rs` | 3 | 0 → 0 | **1 → 1** (equivalent) | 0 | 2 |
| `srt_builder.rs` | 65 | 44 → 57 | **14 → 1** (equivalent) | 0 | 7 |
| `tix_rebuilder.rs` | 30 | 19 → 21 | **4 → 2** (equivalent) | 0 | 7 |
| **all four** | **160** | 112 → 131 | **23 → 4** | 0 | 25 |

Every unviable mutant fails to compile. 23 of the 25 replace a result with
`Default::default()` of a type that has no `Default`: `File`, `StorePath`,
`Vendor`, `Ask`, `Line`, `CmScope`, `TimeLookup`, `Walk`, `Day` or
`ImportKind`. The other two:

- `gdfl_import_command.rs:222:9` (delete the `[y, m, d]` arm) leaves
  `parsed`'s type uninferable (E0282).
- `gdfl_import_command.rs:272:5` (`options_of` → `Default::default()`) calls
  a non-`const` `Default` inside a `const fn` (E0015).

Per shard, before (`c9e3472`) and after (`92113a6`):

| Shard | Before: caught / missed / unviable | After: caught / missed / unviable |
|---|---|---|
| tix 0/1 | 19 / 4 / 7 | 21 / 2 / 7 |
| readonly 0/1 | 0 / 1 / 2 | 0 / 1 / 2 |
| gdfl 0/2 | 25 / 1 / 5 | 26 / 0 / 5 |
| gdfl 1/2 | 24 / 3 / 4 | 27 / 0 / 4 |
| srt 0/3 | 15 / 2 / 5 | 17 / 0 / 5 |
| srt 1/3 | 17 / 3 / 2 | 19 / 1 / 2 |
| srt 2/3 | 12 / 9 / 0 | 20 / 1 / 0 |

The re-run on the final head `ad45a98` covered the 10 mutants at the
remaining sites:

| Mutant | On `ad45a98` |
|---|---|
| `srt_builder.rs:472:26` delete `!` | missed (equivalent, below) |
| `srt_builder.rs:641:38` `>` → `<` | **caught** by the stray-entry-alone case (it was the one srt survivor of the `92113a6` run besides 472:26) |
| `srt_builder.rs:641:38` `>` → `>=` | caught |
| `srt_builder.rs:641:38` `>` → `==` | caught. The first attempt hit the 144 s test timeout while the opt-level-3 gate build ran on the same 4 cores. Re-run alone with `--timeout 600`, it was caught in 19 s by `not_walked_is_said_for_a_link_alone_and_never_for_nothing`, and it was caught in the `92113a6` run too. |
| `tix_rebuilder.rs:312:16` guard → `true` | missed (equivalent, below) |
| `tix_rebuilder.rs:312:16` guard → `false` | caught |
| `tix_rebuilder.rs:316:41` guard → `true` | missed (equivalent, below) |
| `tix_rebuilder.rs:316:41` guard → `false` | caught |
| `tix_rebuilder.rs:316:41` delete `!` | caught |
| `readonly_file.rs:64:5` `regular` → `Ok(Default::default())` | missed (equivalent, below) |

Final count on `ad45a98`: **131 caught, 4 missed (all four equivalent), 0
timeouts, 25 unviable**, out of 160.

## Survivors killed, and by which test

The test named is the one that failed under the mutant in the after run, read
from that mutant's cargo-mutants log.

| Mutant (before: missed) | Killed by |
|---|---|
| `gdfl_import_command.rs:108:21` guard `why.kind() == NotFound` → `true` | `gdfl_import_command::cloud_mutant_tests::an_options_tree_that_cannot_be_listed_or_named_is_refused` (an options tree that is a file is named, never read as empty) |
| `gdfl_import_command.rs:444:5` `render_how` → `()` | `…::the_receipt_names_the_workers_and_durability_it_filed_with`, `…::a_census_refusal_prints_the_receipt_and_how_it_filed_then_the_refusal` |
| `gdfl_import_command.rs:459:5` `process_env` → `None` | `…::the_process_environment_is_read_as_it_is` |
| `gdfl_import_command.rs:464:5` `cli_store` → `Ok(Default::default())` | `…::the_store_root_is_the_clis_own` |
| `tix_rebuilder.rs:487:5` `command` → `0`, and → `1` | `tix_rebuilder::cloud_mutant_tests::the_dispatched_verb_refuses_a_misuse_with_its_usage` (exit 2, MISUSED) |
| `srt_builder.rs:189:36` `>` → `==`, and `>` → `>=` (`MAX_CONTRACTS`) | `srt_builder::cloud_mutant_tests::a_star_walks_exactly_the_contract_bound_and_refuses_one_more` (exactly 100,000 contract directories are walked; 100,001 are refused) |
| `srt_builder.rs:419:22` guard → `true`; `419:46` `&&` → `\|\|` | `…::a_dry_run_names_a_month_of_several_runs_as_one_to_build`; also pinned from the coverage side by `…::one_run_over_fewer_rows_than_committed_is_built_again` |
| `srt_builder.rs:641:23` `\|\|` → `&&`; `641:19` `>` → `==`, `<`, `>=`; `641:38` `>` → `==`, `>=` | `…::not_walked_is_said_for_a_link_alone_and_never_for_nothing` |
| `srt_builder.rs:641:38` `>` → `<` | the same test's stray-entry-alone case, added in `ad45a98` (caught on `ad45a98`; it survived the `92113a6` run) |
| `srt_builder.rs:731:5` `command` → `0`, and → `1` | `srt_builder::cloud_mutant_tests::the_dispatched_verb_refuses_a_misuse_with_its_usage` |

The bound test is the one expensive test added: it creates 100,000 empty
directories (short names, `C000000` up to `C100000`) and runs one dry walk
over them, taking about 16 s at opt-level 0. It is the only way to tell
`>` from `>=` and `==` at `MAX_CONTRACTS` without changing the constant.

## Equivalent mutants, and why

| Mutant | Why no test can kill it |
|---|---|
| `readonly_file.rs:64:5` `regular` → `Ok(Default::default())` | **Not compiled on this host.** It is the stub under `#[cfg(not(any(target_os = "macos", all(target_os = "linux", any(x86_64, aarch64)))))]`, so on Linux x86_64 the mutation changes no compiled byte and every test passes. On any target where the stub does compile, `std::fs::File` has no `Default`, so the mutant would be unviable there. No build of this crate can both compile it and run it. |
| `tix_rebuilder.rs:312:16` guard `matches!(line, Line::Small { .. })` → `true` | The guard matters only when the read-back finds `Bisection(BelowThreshold)` after the index step said `Kept` or `Built`. The store's writer (`index_on_open`) and reader (`decided`) both give `BelowThreshold` iff the month holds fewer than `INDEX_MIN_ROWS` bars **and** no `.tix` name exists. `Kept`/`Built` leave a `.tix` file, and a directory or link at the name is refused before either. So the two disagree only if another process changes the `.tix` between `index_existing` releasing the month's lock and `open_existing` taking it. No deterministic test can place that race, and there is no seam between the two calls. |
| `tix_rebuilder.rs:316:41` guard `!matches!(line, Line::Small { .. })` → `true` | The mirror case: line `Small` (no `.tix`, under the threshold) with a reader that answers `Indexed`, which needs a valid `.tix` to appear in the same unlocked window. Same reason as above. |
| `srt_builder.rs:472:26` delete `!` in `let read_back_body = !matches!(done.outcome, Resorted::Kept)` | This only swaps *which* outcome gets the read-back's body re-check (`SortIndex::verify`). Under the mutant, `Kept` is re-checked and `Built` is not. The store's `sort_index_existing` returns `Kept` only after it has made that same check under the month's exclusive lock, and `Built` only after it has just written the run whole from the verified bars. Telling the two apart needs the body to change between the store's write or check and the read-back, which again is a concurrent change or a store defect. |

## Real bugs

**None in production code.**

One test-hygiene finding in a pre-existing test, not fixed (it is not this
pass's code, and it matters only under a mutant):
`gdfl_import_command::tests::the_stored_flag_narrows_a_capital_market_run_to_what_d3333_adds`
derives its fixture paths from `options_tree(&store)`. Under the mutant
`options_tree` → `PathBuf::new()`, it writes `README`, `HDFC/`, `NIFTY/` and
`TATAMOTORS/` into the test's working directory, `crates/cli/`. The mutant is
caught, but the files outlive the run. The pass deleted them after each run
that made them; nothing of theirs is committed. A fix would build the fixture
from `store.join("bars/gdfl/NSE/FNO")` rather than from the function under
test.

## Coverage, before → after

The scoped modules' own tests (the same filters as the mutation runs), at
opt-level 0. **Production lines** are the lines before each file's first
`#[cfg(test)]` module (gdfl < 556, srt < 735, tix < 491, readonly < 70).
llvm-cov also counts a file's in-file test modules, so the whole-file
figures are shown too.

"After" is the committed head `ad45a98`, whose build carries a commit stamp
like the operator's. In such a build, gdfl's unstamped-write refusal
(lines 316 and 319) cannot run. In an unstamped build (any build from a
dirty tree) the existing `an_unstamped_build_is_refused_a_store_write_unless_allowed`
covers it, and gdfl's production lines measured 302/302.

| File | Production lines | Production branches | Whole file, lines | Whole file, branches |
|---|---|---|---|---|
| `gdfl_import_command.rs` | 294/302 (97.35%) → **300/302 (99.34%)**; 302/302 unstamped | 31/34 → **33/34** | 294/302 → 502/504 (99.60%) | 31/34 → 33/34 (97.06%) |
| `readonly_file.rs` | 18/19 (94.74%) → **19/19 (100%)** | 2/2 → **2/2** | 185/199 → 199/212 (93.87%) | 15/20 → 15/20 |
| `srt_builder.rs` | 350/400 (87.50%) → **386/400 (96.50%)** | 49/54 → **53/54** | 350/400 → 702/716 (98.04%) | 49/54 → 69/70 (98.57%) |
| `tix_rebuilder.rs` | 236/265 (89.06%) → **249/265 (93.96%)** | 25/26 → **25/26** | 236/265 → 319/335 (95.22%) | 25/26 → 27/28 (96.43%) |

### Lines and branches still not covered, and why

| File:line | What | Why no test reaches it |
|---|---|---|
| gdfl 316, 319 (and branch 316) | `stamped_root`'s unstamped refusal | Runs only in an unstamped build; see above. Covered there. |
| srt 428–432 | the `|row| file.read_record(row)…` closure given to `store::sort_index::bind` | `bind` calls it only for a run boundary that is neither row 0 nor `n_valid`, and the guard admits exactly one run covering `0..n_valid`. It runs only under the guard's mutants, which is how those are killed. |
| srt 482–485; tix 302–305; tix 336–339 | the `other =>` arm over `#[non_exhaustive]` `Resorted`, `Reindexed` and `TimeLookup` | Every variant the store has today is matched above it. Unreachable until the store adds a variant. |
| srt 490–493; tix 317–323 (and tix branch 316) | the read-back after the index step finds the month not indexed, or is refused | The writer just built or kept the index under the month's lock, and the reader decides from the same header and file names. Only a change between the two opens reaches these lines, the race described under "Equivalent mutants". |
| srt 661; tix 443 | `Err(why) => Line::Refused { why }` for a path inside the month loop | `ask` proves every month's store path before the loop. For a `*` walk, every contract name comes from `Contract::parse` (A–Z, 0–9 and `-`, at most 24 bytes), which `check_segment` (same alphabet, `MAX_SEGMENT_LEN` 24) always accepts, so `StorePath::new` cannot refuse it there. |
| srt branch 527 | `symlink_metadata(.srt)` failing with a kind other than `NotFound` | The reader has just opened a file in the same directory. Only an I/O fault, or a permission refusal (which uid 0 bypasses), produces another kind. |
| readonly 139, 151–153, 164, 169, 188–196 (branches 137, 150, 162, 166, 185) | failure paths of the pre-existing tests' own helpers (child timeout, `mkfifo` failure, a FIFO open that blocks) | They run only when the door under test regresses. They are not production code. |

## Gates

Run on the final test head `ad45a98` in the main tree, on the repository's
own profile (opt-level 3, incremental off), as root in this container:

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | **clean** (exit 0) |
| `cargo clippy -p cli --all-targets -- -D warnings` | **clean** (exit 0) |
| `cargo test -p cli --locked --no-fail-fast -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store` | **exit 101.** lib: 2050 passed, **1 failed**, 1 ignored, 1 filtered out (the skipped pool test, which runs for 95+ minutes), 737 s. bin `main.rs`: 0 tests. Integration: all 14 files pass, 60 tests (`binary` 7, `c4_cli_02_limits` 3, `crate_graph_claims` 2, `ledger_append_lookup_costs` 6, `ledger_scan_costs` 4, `limits_doc_drift` 5, `limits_o1cli_2` to `_6` 1 each, `pool_and_escape_docs` 11, `resample_matches_fold` 2, `sweep_evidence` 15). Doctests: 1 passed. |

`--no-fail-fast` was added to the task's command only so the integration and
doc tests still run after the lib failure. Without it, cargo stops at the lib
binary. The same gates on `92113a6` gave the same result (lib 2049 passed and
the same 1 failed; integration, bin and doc tests all passed). The 1 failure:

`scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger`
is outside this pass's files and fails only because this container runs as
uid 0: the test sets the ledger to mode `0444` and expects the write to be
refused, and root is not refused. The same test binary passes it as uid 65534
(`setpriv --reuid=65534 --regid=65534 --clear-groups`).

GitHub CI on PR SJParthi/brutex#78 is red on "Gate W — the browser tree" (W1:
`web/build` does not match `web/`) and "Gate 1+2 — one language" (1e:
`core`'s `banned_lists` test cannot parse the workflow step, because the base
snapshot keeps `.github/workflows/` as on `main`), and so on `ci-ok`. Both
fail identically on SJParthi/brutex#75, a sibling PR on the same base that
changes a different crate. This pass touches neither `web/` nor
`.github/workflows/`, and was told never to edit the latter. The reason is
on the PR as a comment.

## NOT DONE

- **The four equivalent mutants are not killed:** readonly 64:5, tix 312:16 and 316:41, srt 472:26. Killing the three race-only ones needs a fault-injection seam between the index step and its read-back in `tix_rebuilder::rebuild` and `srt_builder::build`. That is a production change, and this pass changes tests only.
- **Coverage is not 100% on srt and tix production lines:** the lines above are defensive arms the store's contract makes unreachable. Covering them needs either the same seam or a store that can be told to misbehave.
- **The full mutation pass was not repeated on the final head `ad45a98`.** It ran on `92113a6`. `ad45a98` adds two test cases and splits asserts, and only the mutants at the sites still missed were re-run on it (above).
- **Branch coverage needed `RUSTC_BOOTSTRAP=1`**, because the pinned toolchain is stable. It is recorded here as measured that way.
- **CI on the PR is red** for the two snapshot reasons above. It cannot go green until the branch sits on a base whose workflows and web bundle match its code.
- **`cargo deny check` and the workspace-wide `cargo test`** (the rest of §9) were not run. The task named the three gates above.
- **Not repeated locally:** the 95-minute `the_pool_verb_prints_its_whole_page_on_a_generated_store`. It was skipped as instructed.
