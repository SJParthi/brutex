# RESULT — group rest (branch pr74/g18-rest)

Head: **fea3659**, on top of origin/final/all-fixes 969493e1 (it has not moved, so there was nothing to merge). Decisions D-2070..D-2088; invariant rows G18-rest-01..36.

All mutation evidence comes from cargo-mutants 26.2.0, run as root:
- in the main checkout (`--in-place`), or
- in the world-readable worktree `/home/user/wt-g18`, where the uid-65534 child tests run, or
- with CI's flags (`--minimum-test-timeout 900 --timeout-multiplier 2 --cap-lints true --cargo-arg=--locked --test-tool nextest --cargo-test-arg=--max-fail=1:immediate --cargo-test-arg=--test-threads=1 --no-shuffle`) plus `--build-timeout 300` where marked **CI+bt**.

"Killed by" names a test that fails on the mutant and passes on the fix.

## Survivors from survivors-rest.md (47) and those added later

| item | fixed? | commit | evidence |
|---|---|---|---|
| costs regime.rs:181 `&&`→`\|\|` rate_on | yes, test | a54f945 | caught, killed by `regime::tests::a_refusal_names_the_nearest_verified_row_not_a_later_one` |
| costs trip.rs:1085 `>`→`>=` charge_stack_legs | yes, restructured (`buy.gst().max(sell.gst())`, equivalent mutant removed) | a54f945 | mutant absent from `cargo mutants --list` |
| indicators gap.rs:387/388 `>`→`>=`, `<`→`<=` GapFib::fold | yes, restructured (`max`/`min`) | a54f945 | mutants absent from `--list` |
| indicators pattern.rs:647 `<`→`<=` | yes, test | a54f945 | caught, killed by `pattern::exemplars::in_neck_closes_at_or_just_into_the_prior_body` |
| indicators trend.rs:329 `>`→`>=` Atr::fold | yes, test | a54f945 | caught, killed by `trend::tests::the_period_th_range_completes_the_seed_mean_exactly` |
| lake footer.rs:106, 207, 240 ×3, 248 | yes, tests | 92bcd1a | caught, killed by `footer::tests::{a_skip_of_exactly…, a_map_alternates…, zigzag_decodes_both_signs, a_bool_takes_a_byte…}`; all of footer.rs CI+bt: 84 tested, 80 caught, 4 unviable, 0 timeout |
| pull support/mod.rs:42 (fn→()), :62 `&&`→`\|\|` | yes, tests (D-2073) | e3e8370, 37e4c56 | caught in wt-g18, killed by `support::tests::the_body_runs_in_a_child_where_the_bits_bind` and `a_child_that_ran_no_test_fails_the_parent` |
| store support/mod.rs:42, :62 | yes, same tests | 3de8a19, 37e4c56 | caught in wt-g18 (with baseline run) |
| pull archive.rs:134 `*`→`+` | yes, test pins 268,435,456 | e3e8370 | 4 caught, killed by `archive::tests::a_member_past_the_byte_cap_is_refused_before_it_is_read` |
| pull calendar.rs:1404 guard→true last_day | yes, restructured (one `saturating_add(span.saturating_sub(1))`) | e3e8370 | guard mutant absent from `--list`; last_day →0/1/−1 caught in the in-diff run |
| pull fetch.rs:845 `&&`→`\|\|` land_rows | yes, test | e3e8370 | caught (baseline run), killed by `pipeline::fetching_and_landing_is_one_call_over_the_same_seam` |
| pull http.rs:695 `==`→`!=` wait_for_permit | yes, test | e3e8370 | caught, killed by `http::tests::a_reservation_in_the_future_is_slept_to_and_counted` |
| pull http.rs:997 delete `\\` arm | yes, test | e3e8370 | caught, killed by `http::tests::an_escaped_quote_inside_a_key_does_not_end_the_key` |
| pull ingest.rs:2509 guard→false check_day | yes, restructured into `daycheck_headline` + test (D-2075) | e3e8370 | guard true/false caught, killed by `ingest::tests::a_clean_day_check_is_info_and_anything_else_warns` |
| pull masters.rs:397 `!=`→`==`, :449 `==`→`!=`, `&&`→`\|\|` | yes, new emit-site row (D-2076) | e3e8370 | 3 caught in wt-g18, killed by `emit_sites::every_emit_site_in_this_crate_reaches_a_file` |
| pull rolling.rs:613 `<`→`<=` push_pair | yes, test | e3e8370 | caught, killed by `rolling::tests::the_control_escape_stops_below_the_space` |
| pull session.rs:1274 `<`→`<=` irregular_verdict | yes, restructured (open asked first) | e3e8370 | 3 caught, killed by `session::tests::the_ingest_verdict_keeps_exactly_what_the_calendar_owes_on_irregular_days` |
| store file.rs:2660, 2699, 2747, 2748, 2912 | yes, tests (D-2077) | 3de8a19 | caught in wt-g18 |
| store file.rs:3312 ×4, 3535, 3752 ×2, 4089, 4090, 4279 | yes, tests (D-2078) | 3de8a19 | file.rs survivors in wt-g18 at d170a21: 14 tested, 14 caught |
| store open_flags.rs:127 → Ok(Default) | yes, restructured into one fn (D-2079) | 3de8a19 | the remaining whole-fn mutant needs `File: Default` and does not compile, so it is unviable |
| telemetry tail.rs:565 `>`→`>=`, `==` | yes, test | 92bcd1a | caught, killed by `tail::tests::a_line_of_exactly_the_cap_is_decoded_not_dropped` |
| vocab expression_search.rs:247 guard→true | yes, restructured (the `checked_sub` is the depth check) | a54f945 | guard mutant absent from `--list` |
| vocab table.rs:1607 `>`→`>=` | yes, restructured (`longest_name` recursion) | 3565e47 | absent; see D-2083 |
| store format.rs:1056 Overlay::same_bytes→true (shard 182) | yes, test (D-2082) | 3b18a82 | 3 tested, 3 caught, killed by `unit::an_overlay_matches_only_its_own_bytes` |
| vocab fnv1a/name_index build hang (shards 146, 150) | yes, recursion (D-2083) | 3565e47 | CI+bt over the 39 table mutants: 13 caught, 26 unviable, 0 missed, 0 timeout |
| TIMEOUT telemetry tail.rs:503 `>`→`>=` walk_back (shard 58) | yes, passes bounded by a range (D-2084) | bb73610 | CI+bt over walk_back: 13 tested, 13 caught, 0 timeout |
| TIMEOUT lake footer.rs:96 Cursor::byte→Ok(1) (shard 49) | yes, walk bounded at 2n+2 passes (D-2085) | d170a21 | CI+bt over all of footer.rs: 84 tested, 80 caught, 4 unviable, 0 timeout |
| TIMEOUT pull masters.rs:1282 holds_exactly→Ok(true) (shard 10) | yes, the FIFO test releases both directions (D-2085) | d170a21 | CI+bt over holds_exactly: 5 tested, 5 caught |
| untested store: remove_index guard true/false, refuse_if_sealed guard→true, missing_below `&&`→`\|\|` | yes, tests (D-2086) | aed23ec | CI+bt in wt-g18: 5 tested, 5 caught |
| own-diff MISSED: ingest check_day→() | yes, returns the emitted level (D-2087) | 445ad02 | caught in the r-mine2 in-diff run, killed by `ingest::tests::zerodha_minutes_are_checked_against_the_pulled_day_and_no_other_vendor_is` |
| untested pull: daycheck.rs:67 `+`→`-` ist_day | yes, test (D-2088) | fea3659 | CI+bt: 2 tested, 2 caught (`+`→`-`, `+`→`*`), killed by `daycheck::tests::the_ist_day_turns_at_ist_midnight` |

## Own changes (in-diff of 969493e1..445ad02, crates only)

CI+bt, in place in /home/user/wt-g18 with baseline run: **66 mutants tested in 16m: 37 caught, 29 unviable, 0 missed, 0 timeout.** An earlier run over 969493e1..d170a21 (63 tested) had found the one MISSED fixed in 445ad02. fea3659 adds only a test and docs.

## Validation at fea3659
- `cargo fmt --check` clean, and `cargo clippy --workspace --all-targets --locked -- -D warnings` clean (at aed23ec; pull clippy at 445ad02).
- Static gates (language-purity job, all steps except 1e): PASS at fea3659.
- Tests as uid 65534 (`setpriv --reuid=65534`) for costs, indicators, lake, telemetry, vocab, pull and store at d170a21: 75 binaries, 2476 passed. The only failure is store `cited_commits` (5 tests); git and the root-owned target tmpdir are unreadable to nobody, the file is untouched, and it passes as root (6/6).
- Root test run of pull, store, telemetry, lake and vocab after each change: green.

## Timing (p50/p99/max per call, ns; scratch harness /tmp/claude-0/tbench-*, release, 3000 batches of 1000, idle box, 2 runs each)

| path | base 969493e1 | head |
|---|---|---|
| costs charge_stack | 75–78 / 124–151 / 212–285 | 81 / 141–147 / 170–199 |
| indicators GapFib::step | 61–62 / 105 / 141–247 | 63 / 116–118 / 153–185 |
| pull Calendar::last_day | 0.7 / 0.7–1.3 / 20–23 | 0.7 / 1.4–1.5 / 15–20 |
| pull Window::verdict (irregular day) | 27 / 57–83 / 82–245 | 27–28 / 45 / 56–73 |
| vocab index_of | 17 / 32–74 / 46–181 | 17 / 32–36 / 68–85 |
| vocab Cursor::advance | 87–95 / 131–191 / 165–760 | 128–130 / 171–172 / 191–211 |

Every path stays O(1) per call. Cursor::advance is about 40 ns slower at p50 in the head build. Putting the removed `below_depth >= 2` guard back left it at 131–138 ns, so the difference is not caused by this group's `place` change. Base and head differ only in vocab table.rs's compile-time functions, which advance does not call. The cause is **UNVERIFIED** (likely code layout between two separately compiled binaries).

## Incidents (honest record)
- 92bcd1a and 99d8217 carried a committed mutation (`while pos >= 0` in tail.rs:503): I committed while a concurrent in-place run of mine held the file mutated. Fixed in 0d8c386, and reported to the coordinator.
- My first pull/store mutation runs were in a worktree under a root-only directory, so the uid-65534 tests failed for every mutant. Those results were discarded and re-run in /home/user/wt-g18.

## Not done / open
- Coverage (line and branch, 100%) was not measured: **UNVERIFIED**.
- `cargo deny check` was not run in this session: **UNVERIFIED**.
- The Cursor::advance timing gap is unexplained (see Timing).

## Coordinator's untested pull survivors (run on 969493e1), on head fea3659

| item | status | evidence |
|---|---|---|
| daycheck.rs:67 `+`→`-` ist_day | killed (D-2088) | fea3659: 2 tested, 2 caught, killed by `daycheck::tests::the_ist_day_turns_at_ist_midnight` |
| ingest.rs:2504 check_day→() | killed (D-2087) | 445ad02: caught in the r-mine2 in-diff run |
| ingest.rs:2509 report.clean() guard | killed (D-2075); the guard now lives in `daycheck_headline` | guard true and false both caught, killed by `ingest::tests::a_clean_day_check_is_info_and_anything_else_warns` |

## api cases run 1283 never tested (126 `crates/api` lines in untested-mutants-run1283.md, shard 138 included), on fea3659

IN PROGRESS. CI flags with `--build-timeout 1500`, in place in the world-readable worktrees wt-g18 and wt-g18b. About 14 minutes per mutant: roughly 400 s to build and 420 s to test. The baselines passed in both worktrees: 499 s build + 557 s test, and 559 s build + 523 s test.
- Done before the container was reclaimed: 16 tested, 15 caught, 1 unviable (`autopilot.rs:3278 round → Default`), 0 missed, 0 timeout.
- Remaining 110: in 16 chunks of 7 or fewer, each under 2 h, results under /tmp/claude-0/mut/apic/.
- api chunk c01-b (15:06Z, timeout 1500): 7 tested, 5 caught, 2 unviable, 0 missed, 0 timeout.
