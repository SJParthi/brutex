# cloud-mut-api-a-20261016 — mutation and coverage pass on eight `api` files

Status: **IN PROGRESS.** This file is rewritten as results land; a number
that is not in a table below has not been measured yet.

Branch `cloud/mut-api-a-20261016`, cut from `origin/cloud/base-20261016`
(`c9e3472e`, the one-commit snapshot of local `feat/db-fix` at `4756cce0`).

## Scope

`crates/api/src/` — `bars.rs`, `bars_change.rs`, `bars_series.rs`,
`bars_sort.rs`, `gaps_route.rs`, `spot.rs`, `render.rs`, `ladder.rs`.
`dayflags.rs` is not touched. Nothing outside `crates/api` is edited.

## Method (how the numbers below were taken)

| Item | Value |
|---|---|
| toolchain | `1.97.1` (pinned by `rust-toolchain.toml`) |
| cargo-mutants | `27.1.0` |
| cargo-llvm-cov | installed with `--locked` |
| machine | 4 cores, 15 GiB, cloud container, runs as `root` |
| mutants listed | `cargo mutants --list --file <f> -p api` |

Mutants are run by three workers, each in its own full copy of the tree
(`cp -a`, so file times survive and cargo reuses the dependency build), each
with `cargo mutants --in-place --shard k/n --sharding slice`. Every shard is
time-boxed to 25 minutes by `timeout 1500`.

Two deviations from a plain `cargo mutants`, both forced and both stated:

1. **The api crate is built at `opt-level = 0`, incrementally, without debug
   info, for the mutation run only** (`--config profile.{dev,test}.package.api.
   opt-level=0`, `incremental=true`, `debug=0`). The checked-in `[profile.dev]`
   is `opt-level = 3`, `incremental = false`; under it one mutant of this
   212,892-line crate is a multi-minute rebuild and 2,145 mutants do not fit in
   a session. Dependencies keep the checked-in profile. Gates and coverage use
   the checked-in profile unchanged.
2. **The copies have their `.git` moved aside.** `crates/cli/build.rs` stamps
   the commit only when the working tree equals the index, and watches every
   tracked file to decide it; an in-place mutation of an `api` file therefore
   flipped `cli`'s stamp on every mutant and rebuilt `cli` and all of `api`
   each time (measured: `cli` recompiled for each of the first two mutants).
   With no `.git` the stamp is the constant refusal sentinel and only `api`
   rebuilds (measured: 11 s per mutant build). Nothing in `api`'s tests reads
   the stamp of `cli`'s build for these filters (the filtered baseline passes:
   64 of 64 for `spot`).

**Test filters (stage 1).** Each file's mutants run against the `api` library
tests whose path names the module and the routes that call it:

| File | `cargo test --lib -p api -- …` |
|---|---|
| `spot.rs` | `spot optiongreeks chain:: bars_page_tests` |
| `ladder.rs` | `ladder` |
| `render.rs` | `render` |
| `gaps_route.rs` | `gaps gap_peer calendar_route_tests census_request_tests serve_edge_tests` |
| `bars.rs` | `bars spot gaps` |
| `bars_change.rs`, `bars_series.rs`, `bars_sort.rs` | `bars` |

**Stage 2.** Every stage-1 survivor is run again against the WHOLE `api`
library suite (less the 14 tests below) before a test is written for it, so a
survivor reported here is one no existing `api` test kills.

**Ten `api` tests fail on this machine before any change, and every one says
why: it is a permission test, the session runs as `root`, and `root` is not
bound by mode bits.** Each either panics with its own sentence (*"mode 000 does
not stop this process from searching …: run the tests as a user the mode bits
bind"*) or makes a directory `0o555` and expects the write to be refused. They
are passed to every mutation run as `--skip`, and are run again as an
unprivileged user for the gates (below):

```
logs::tests::a_directory_that_turns_unreadable_is_never_answered_as_read
logs::tests::a_ring_file_that_stays_unreadable_is_named_once
logs::tests::logs_names_a_directory_it_may_not_search
pullrun::tests::an_absent_index_is_built_by_the_scan_and_not_known_is_said_when_it_cannot_be
pullrun::tests::an_index_that_cannot_be_read_never_leaves_the_answer_clean
server::tests::a_failure_of_the_store_keeps_the_leg_owed_and_is_never_called_standing
server::tests::a_probe_name_that_cannot_be_unlinked_is_a_fault_by_name
server::tests::a_run_cut_short_by_a_restart_is_logged_when_it_is_found
server::tests::a_store_that_cannot_take_a_write_is_found_by_asking_it
server::tests::an_expired_fno_walk_cut_short_is_seen_at_the_next_start
```

Four more (`folder::…an_unreadable_folder…` and three in
`census_request_tests`) re-run themselves as uid 65534 through
`crate::isolated::where_permission_binds`; they failed only while the test
binary sat under a `0700` root-only scratch directory that uid 65534 cannot
enter, and pass once the copies live under `/home/user` (measured: 1,959
passed, these 10 failed, full `--lib` run in a copy).

## Mutants listed

| File | Mutants |
|---|---:|
| `bars.rs` | 392 |
| `bars_change.rs` | 146 |
| `bars_series.rs` | 330 |
| `bars_sort.rs` | 641 |
| `gaps_route.rs` | 179 |
| `spot.rs` | 123 |
| `render.rs` | 222 |
| `ladder.rs` | 112 |
| **total** | **2,145** |

## Results (partial: five of eight files finished)

`missed` before is under the stage-1 filter. `after` is the same filter plus
`tests/cloud_mutants_bars.rs`, re-run on every stage-1 survivor. `stage 2`
is the whole `--lib` suite, existing tests only, on the survivors.

| File | total | caught | missed before | stage 2 kills | missed after | timeout | unviable |
|---|---:|---:|---:|---:|---:|---:|---:|
| `spot.rs` | 123 | 105 | 5 | 0 of 1 run | **1** | 0 | 13 |
| `ladder.rs` | 112 | 76 | 12 | 6 | **4** (equivalent) | 0 | 24 |
| `gaps_route.rs` | 179 | 125 | 15 | 0 | **0** | 1 | 38 |
| `render.rs` | 222 | 189 | 31 | not run | **1** (equivalent) | 0 | 2 |
| `bars_change.rs` | 146 | 125 | 12 | not run | **4** (equivalent) | 1 | 8 |

The rest of this report (survivors and the tests that killed them, the
equivalent mutants with reasons, coverage and gates) is written when the
remaining three files finish.
