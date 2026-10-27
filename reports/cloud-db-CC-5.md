STATUS: HELD

# CC-5 — Copies register (cloud build report)

Branch `cloud/db-CC-5-20261024`, cut from `origin/feat/db-fix` at `e851da1ca` (Merge DATA-2); the
newest `origin/feat/db-fix`, `a32722c45` (Merge R3-15), merged in at `48e4cdb09` before the final
gates (27 Oct; the earlier final merge was `5901ec77` at `e7f5311c`). Spec: `origin/cloud/db-base-20261024:reports/db-specs-20261024/CC-5.md` (FM-607).
Decision block D-7040..D-7049. Prerequisites: none (spec section 1). FILES_ONLY links: none.

## Done

- `crates/store/src/settings.rs`: key 12 `disk.extra_copy_roots` (`Key::ExtraCopyRoots`, slot 11,
  `KEYS` 11 -> 12), folders split by `|`, the whole value bounded by the settings route's existing
  4,096-byte ceiling (no count of roots typed). `copy_roots()` is the one reader: absolute, no `..`,
  none empty, twice or nested (an ordered set, O(r log r)); `check` adds that each folder exists,
  `check_stored` holds a stored value to its shape only (D-7040).
- `crates/api/src/settings_route.rs`: the two arms a new `Key` needs to compile (`default_of`:
  `(None, UNSET_WAIT)`, a named wait; `reader_of`: names `pull::copies_register`,
  `GET /disk/copies.json` and `cli disk-copies`).
- `crates/pull/src/copies_register.rs` + `copies_register_tests.rs` (35 tests): a read-only `Fs` seam
  (no writing method exists); the live roots read through `gdfl_run::source_root` and the settings
  (D-7041); one walk that enters every directory once by `(st_dev, st_ino)`, follows no link, types no
  cap and runs on `available_parallelism` workers (D-7043); hazards by `(st_dev, st_ino)` within one
  walk, never across devices (D-7042, amendment (e)); the reader rule (D-7044); receipts
  (`ALL_VERIFIED.txt`, `VERIFIED.tsv` matched by inode, size, mtime and ctime and never `st_dev`,
  `*.prove.out` with `ALL-PROVED`) and twins (D-7045); the cache, re-walked only when the roots or a
  root's own stat change, no request waiting on another's walk (D-7046); the retirement list as text
  with no command (D-7047); the audited events (D-7048); `total()` for R4-29.
- `crates/api/src/copies_route.rs` + `copies_route_tests.rs` (8 tests): `GET /disk/copies.json` in the
  ops-read pool; unset = `200 waiting` naming the key; the first walk since start = `503 walking` with
  `Retry-After` to any other request; the register rendered once per walk, joined to each request's
  own head (`age_ms`, `fresh`, `why`, `rewalking`).
- `crates/cli/src/disk_copies_command.rs` (4 tests): `cli disk-copies`, always a fresh walk; exit 0
  only with no hazard and every root walked whole, 1 on a hazard, a partial root, the key unset or a
  refused store root, 2 on any argument.
- The /ingest panel `web/src/routes/ingest/panels/Copies.svelte` (order 95, R4-01's registry; hazards
  red; the retirement list downloadable as text) with `web/src/lib/copies.js` and
  `web/tests/copies.test.js` (10 tests).
- The gate-8 bench `crates/api/benches/ratio/copies.rs` (`/disk/copies.json`, n = 200, 1x/10x/100x).
- `docs/02-store-format.md` §42 row; `docs/04-invariants.md` CC5-01..CC5-16; `docs/05-decisions.md`
  D-7040..D-7048; `docs/06-limits.md` L-CC5-1..4.
- Final merge of `origin/feat/db-fix` `5901ec77`; every gate run on the merged tree (below).

## Left

- PENDING

## Gates (final run, 26 Oct 2026, on the merge with `origin/feat/db-fix` `5901ec77`)

The base comparison is an unmodified worktree of `origin/feat/db-fix` `5901ec77`, built in its own
target and run with the same commands.

| Gate | Command | This branch | Unmodified base `5901ec77` |
|---|---|---|---|
| build | `cargo build --workspace --locked --all-targets -j 4` | `Finished`, 0 warnings (20 m 28 s) | — |
| fmt | `cargo fmt --all --check` | exit 0 | — |
| clippy | `cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings` | `Finished`, no warning | — |
| tests | `cargo test --workspace --locked -j 4 --no-fail-fast -- --skip a_full_calendar_pool_refuses_the_census_and_the_audit --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held` | exit 101: 33 failed in five lib targets (api 12 of 2,950, cli 3 of 2,411, pull 9 of 2,158, store 5 of 431, telemetry 4 of 225), every other target ok; the two skipped hang (below) | — |
| the 35, alone | each test binary `--exact <name> --test-threads=1`, `timeout 300`, from its crate's root | 33 FAILED, 2 TIMEOUT | the SAME 33 FAILED and the same 2 TIMEOUT |
| deny | `cargo deny check` | `advisories FAILED, bans ok, licenses ok, sources ok`: RUSTSEC-2026-0386 (tokio 1.53.1) only | identical (`Cargo.lock` byte-identical): BASE-DENY-1024 |
| CI shell gates | the `run:` blocks of ci.yml's `language-purity` job, `GITHUB_ENV`/`RUNNER_TEMP` emulated | red: 1c, 1d, 10, 11, 12, 19, 23, 27b, 29, 30; all others exit 0 | red: the same ten, with the same findings (counts aside); gate 1d's undeclared list equal after `CC5_LITERAL` |
| gate 1e | (builds and tests the whole workspace a second time with the front end moved aside) | not run here; the build and test rows above cover the engine, and no crate reads `web/` | — |
| web W1 | `npm --prefix web run build` + `gates_jobs w1` (Node 24.22.1) | red: the committed `web/build` is stale | red, the same: the base's own source does not build its committed bundle |
| web W2 | `node --test web/tests/*.test.js` (Node 24.22.1) | `tests 1611, pass 1611, fail 0` | `tests 1601, pass 1601` (this part adds 10) |
| web W3 | `npm --prefix web run check` + `gates_jobs w3 ... 0` | `svelte-check errors: 44 (ceiling 0)`, none in this part's files | the same 44 errors, line for line |
| web W4, W5, W6 | as ci.yml | exit 0 (`distinct unused CSS selectors: 0`, `no comment swallows a rule`) | exit 0 |
| coverage | `CARGO_PROFILE_DEV_OPT_LEVEL=0 cargo +nightly llvm-cov --branch --no-report --locked -j 2` with `-p pull --lib -- copies_register`; `-p store --lib -- settings` (skipping the three root-permission tests); `-p api --lib -- copies_route settings_route`; `-p cli --lib -- disk_copies`; then `cargo +nightly llvm-cov report --branch` (27 Oct, on `da59d4853`) | `copies_register.rs` lines 1005/1005, branches 138/138, functions 118/118, regions 1009/1010 (the one left: `entry?` in `Host::list`, a directory stream that fails after it opened, which no test can stage without a failing device); `copies_route.rs` 261/261 lines, 10/10 branches, 335/335 regions; `disk_copies_command.rs` 417/417 lines, 16/16 branches, 481/481 regions; every line and both sides of every branch this part added to `settings.rs` (129-150, 166, 184, 220, 528-586, 664-668, 808-810) and `settings_route.rs` (240, 425-428) executed | — |
| mutants | PENDING | PENDING | — |

**Why the 35 are not this part's.** None is in a file of this part, and each fails or hangs alone on
the unmodified base with the same command. They are tests of a refused permission or a read-only
folder, which this container cannot refuse because it runs as root (store `settings::tests` x3,
`quarantine::tests` x2, telemetry `sink::tests` x4, pull `alerts`, `gdfl_dropbox` x2, `gdfl_import`,
`gdfl_run::ia2_tests::every_id_the_runs_ledger_gives_is_written_down_twice` (a read-only claims
file), `census_store::link_tests` x2, `zerodha_verify` x2, cli `scrub_blocks`, `watchdog_command`
x2, api `audit::cloud_mutant_tests`, `pullrun::cloud_mutant_tests`, `pullrun::restart_tests`,
`server::cloud_mutant_tests_c` x2, `feeds_route`, `detail`), the five slow-reader socket tests of
`server::head_deadline_body_tests`, and the two full-pool children that hang
(`server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`,
`cloud_mutants_bars` `spot_rs::spot_json_is_refused_by_name_when_every_store_read_slot_is_held`).
Every test of this part's files passes in the workspace run.

## Measurements (p50 / p99 / max)

- `/disk/copies.json` served from memory (the gate-8 bench, `crates/api/benches/ratio/copies.rs`,
  `cargo bench -p api --bench ratio --locked -j 2 -- --served-only`, release, this cloud container,
  4 cores, 25 Oct 2026; n = 200 sequential requests per size, one copy root, a hundred invented files
  to a folder): 125,975 / 386,666 / 1,457,036 ns at 100 files; 131,601 / 257,489 / 679,429 ns at
  1,000; 140,852 / 419,218 / 1,139,531 ns at 10,000. p50 ratio 1x -> 100x 1.118x (ceiling 3,000 per
  mille: ok). `docs/06-limits.md` L-CC5-2.
- The walk (not O(1), O(entries), L-CC5-1), the first request, n = 1 each: 1.495 ms at 100 files,
  3.920 ms at 1,000, 20.511 ms at 10,000. The real `tools-work/` tree is UNMEASURED here
  (NEEDS THE MAC 8).
- PENDING (re-measure on the final tree)

## Extreme cases (each a test that fails before this part: the module did not exist)

46 tests across the four Rust files; the 8 the spec names first, then:

1. Empty root and one-file root, counted exactly (`an_empty_root_and_a_one_file_root_are_counted_exactly`).
2. A root that vanishes mid-walk, or is a file, a link or unreadable (`a_root_that_vanishes_or_is_not_a_folder_is_refused_by_name`, `a_root_that_is_a_file_a_link_or_unreadable_is_named_in_its_stamp`).
3. The same inode number on two devices is not a link (`the_same_inode_on_another_device_is_not_a_link`, amendment (e)).
4. A link made deep in the tree after a cached walk is not seen until the root's own stat changes, and every answer says its age (`a_cached_answer_names_its_walk_time`, amendment (f)).
5. A symlink loop and a folder met twice through a bind-style second name are walked once (`a_link_loop_and_a_folder_met_twice_are_walked_once`).
6. Names with a comma, a line break or bytes that are not UTF-8 stay one line, quoted (`names_with_a_comma_a_line_break_or_no_utf8_stay_one_line`, `a_name_that_is_not_utf8_is_served_escaped_and_a_refused_root_has_no_figures`).
7. An unreadable subfolder is a named fault that keeps the root off the retirement list (`an_unreadable_subfolder_is_a_named_fault`).
8. Twenty requests during a cold walk take one walk; the others are answered `Walking` at once (`twenty_requests_during_a_cold_walk_take_one_walk`, `a_request_during_the_first_walk_is_a_503_naming_it`).
9. A torn, empty, silent or misspelled receipt proves nothing (`a_torn_empty_or_silent_receipt_is_no_proof`, `a_torn_or_misspelled_verified_tsv_proves_no_twin`, `a_receipt_read_that_fails_mid_file_is_the_hosts_refusal`).
10. A receipt written before a remount (another `st_dev`, the same inode, size and times) still matches (`a_receipt_from_before_a_remount_still_matches_by_inode_size_and_times`).
11. A copy root inside a live root (as `prove-20261004` sits in the tick store) is read only where that root's reader reads; one that is or holds a live root is never offered (`a_copy_root_inside_a_live_root_is_read_only_where_that_root_reads`, `a_copy_root_that_is_or_holds_a_live_root_is_never_offered`).
12. Clock edges: walk times at, just before and a day before the 1970 epoch are written exactly (`walk_times_at_the_clock_edges_are_written_exactly`, `a_wall_clock_before_1970_is_negative_micros`).
13. A full ops-read pool is refused by name (`a_full_pool_is_refused_by_name_and_an_admitted_answer_passes`).
14. The setting at its byte ceiling exactly is read, one byte past it refused (`the_extra_copy_roots_key_is_checked_and_stored_canonically`).

Combinations: `hazards_of_two_kinds_a_shared_file_and_an_outside_link_in_one_walk` (a `.tmp` second
name of a CM archive file (partial), a second name of a tick-store file, a file shared by two copy
roots and a link to a file outside every root, in one walk); `a_partial_root_a_proven_twin_and_a_folder_inside_a_live_root_are_served` (a
partial root, a proven twin and a folder inside a live root in one answer);
`a_changed_configuration_walks_and_a_held_lock_serves_the_last_walk` (a changed configuration while
another walk holds the lock).

Out of the box: `a_pipe_named_like_a_receipt_is_never_opened` (a FIFO named `VERIFIED.tsv` would block
a reader for ever; it is never opened); `a_socket_in_a_copy_root_is_counted_never_opened`;
`the_retirement_list_contains_no_delete_command` over a folder whose own name holds `rm -rf` (the
words are quoted as a path, never read as a command).

## Files outside ownership (each minimal)

The spec's section 4.2 list: `crates/pull/src/lib.rs` (one `pub mod` and its test module),
`crates/api/src/server.rs` (one `mod` line, one `.route`), `crates/api/src/operation_audit_tests.rs`
(EXEMPT row, DATA), `crates/api/src/pool_table_tests.rs` (OpsRead row), `crates/cli/src/lib.rs`
(`mod`, one dispatch arm, `COMMANDS` 63 -> 64, one usage entry, the zero-argument exemption),
`crates/cli/src/store_write.rs` (reader list), `crates/cli/src/sweep_wiring_tests.rs` (non-sweep
list), `crates/pull/tests/no_scheduler_delete.rs` (`DECLARED` 12 -> 15, `LISTED` 4 -> 7: the register,
the route and the verb), `crates/store/src/settings.rs` + `settings_tests.rs` (the appended key),
`crates/api/src/settings_route.rs` (the two required arms, nothing else), `docs/02-store-format.md`
§42 (the key's row), `crates/api/benches/ratio.rs` (`#[path]` mount and the call), and this part's own
rows in `docs/04`, `05`, `06`. Beyond that list: `web/vite.config.js` (one dev-proxy line for the new
route: `web/tests/proxy.test.js` requires one for every route a page fetches) and
`.github/gates_tree.rs` (`CC5_LITERAL`, gate 1d's declaration of this part's 61 literals,
`DECLARED` 77 -> 78, as R4-28 and IA-10c declared theirs).

## D-numbers used

D-7040, D-7041, D-7042, D-7043, D-7044, D-7045, D-7046, D-7047, D-7048 (CC-5's block). D-7049 unused.

## NEEDS THE MAC

All read-only on real data; none run in the cloud (`NOT RUN (cloud)`). Port 8510 is the part's
shadow port (the second agent's 8610). `W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`.

1. Baseline: `ls -li $W/tools-work/options-queue/*.zip.part /Volumes/WD_BLACK/Options/20*.zip`.
   Expected: seven `<year>.preserved.<ts>.zip.part` names (2020-2026), nlink 2, each the inode of
   `/Volumes/WD_BLACK/Options/<year>.zip`; six other `.zip.part` names, nlink 1, 81,481,744,630 B in
   all. A difference means the operator acted: record the new baseline, never "fix" the tree.
2. Unset is a named wait (a private empty store root served by the CC-5 release `api` on 8510):
   `curl -s http://127.0.0.1:8510/disk/copies.json`. Expected `200`
   `{"state":"waiting","key":"disk.extra_copy_roots","why":"not set: ...","walked":false,...}`;
   `cli disk-copies` prints `DISK COPIES WAITING: disk.extra_copy_roots is not set...`, exit 1.
3. Save the roots on that private root:
   `curl -s -X POST http://127.0.0.1:8510/settings -H 'content-type: application/json' -d '{"expected":0,"changed_by":"cc5-check","reason":"copies register check","set":{"disk.extra_copy_roots":"<root>|<root>|..."}}'`
   with the roots `$W/tools-work/options-nested-proof`, `$W/tools-work/options-queue`,
   `$W/tools-work/update-20260925-30/extracted`, `/Volumes/WD_BLACK/brutex/tickstore-data/prove-20261004`,
   `$W/tools-work/fullscan-20260930`, `$W/tools-work/optrescan-20261001` and the `_v3` ledger folder
   under `/Volumes/WD_BLACK/NSE_Options_Tick`, with `BRUTEX_GDFL_NFO_ARCHIVES=/Volumes/WD_BLACK/Options`,
   `BRUTEX_GDFL_CM_ARCHIVE`, `BRUTEX_GDFL_TICKSTORE` and `BRUTEX_GDFL_V3_LEDGER` set as the operator
   runs. Expected: `200`, a new generation; a folder that does not exist is refused naming the key;
   the value is NOT refused `secret_shaped`.
4. The register over the real tree:
   `curl -s http://127.0.0.1:8510/disk/copies.json | jq '{walked_at, age_ms, roots: [.roots[] | {path, state, bytes, files, reader, proof, duplicates_live}], hazards: .hazard_count, extra: .extra_copies_bytes}'`.
   Expected within the 13 Oct measurement: options-nested-proof about 426.8 GiB, proof its
   `ALL_VERIFIED.txt` / `VERIFIED.tsv` receipts, reader `none found`; options-queue's six partials
   81,481,744,630 B, no reader; extracted about 12.68 GiB, 82,073 files, no reader; prove-20261004
   about 5.30 GiB, fullscan about 18.94 GiB, optrescan about 17.66 GiB, no reader; the `_v3` ledger
   about 2.38 GiB WITH reader `v3_ledger` and absent from `retirement_list`; `hazard_count` 7, one
   `hazards[]` row per hard-linked `.zip.part` name naming its archive, `dev`, `ino`, `nlink` 2,
   `partial` true. (Count with `.hazard_count` or `.hazards | length`: a retirement entry also
   carries a `hazard` field.)
5. The verb: `BRUTEX_STORE=<the private root> $W/tgt/bin-CC-5/release/cli disk-copies; echo "exit=$?"`.
   Expected: the same seven `HAZARD` lines, the roots of step 4, a fresh `walked_at`, the last line
   `DISK COPIES: 7 name(s) of live files are hard-linked under another name (exit 1)`, `exit=1`.
6. Nothing changed: repeat step 1 (identical names, inodes, nlink, sizes), and
   `find $W/tools-work -newer <a stamp made before step 4> -print | head` prints nothing.
7. The cache on the real tree: two GETs in a row, the second with the same `walked_at` and a larger
   `age_ms`, `fresh` false; `touch` a NEW file in a private copy root the check owns (never in
   `tools-work/`), GET again: a new `walked_at`, `why` naming the changed root.
8. The bench: `cargo bench -p api --bench ratio --locked -j 2 -- --served-only` (the `copies` rows),
   and n >= 200 sequential `curl` GETs of `/disk/copies.json` on 8510 after step 4: p50, p99, max;
   the cold walk's time over the real roots written into `docs/06-limits.md` L-CC5-1.
9. The /ingest panel on `http://127.0.0.1:8510/ingest`: the Copies panel mounts through the
   registry, shows the groups and seven red rows, the retirement list downloads as text with no
   command and says "the operator removes these by hand; the app never does"; it fits 375 px.
10. After merge, on :8080 (the main session's redeploy): steps 2, 4 and 9 again; a screenshot to the
    operator.
11. Rebuild `web/build` at merge-back (`npm --prefix web run build`, commit): W1 is red on the base
    itself (its committed bundle is stale); this part does not commit `web/build`.
12. The permission-bit tests (store `settings::tests` x3 among the 35 above) as a non-root user:
    expected green on both the base and this branch.
13. `cargo deny check` after BASE-DENY-1024 bumps tokio: expected green.

Store writes needed: none by code. The settings generation of step 3 on the REAL store is the
operator's own `POST /settings` (or the operator's yes) at merge-back.

## Log

### 2026-10-24 — start

- GO present, HOLD absent, spec present. Branch cut from `e851da1ca`.
- Gate tools: `cargo-deny` 0.20.2, `cargo-llvm-cov` 0.9.1, `cargo-mutants` 27.1.0 installed from
  crates.io (GitHub release downloads answer 403 through the proxy).

### 2026-10-24 — save point 1 (HELD)

- Files: `crates/store/src/settings.rs`, `crates/store/src/settings_tests.rs`,
  `crates/api/src/settings_route.rs`, `crates/pull/src/lib.rs`, `crates/pull/src/copies_register.rs`,
  `crates/pull/src/copies_register_tests.rs` (header only).
- `cargo clippy -p pull -p store --locked`: no warning, no error.
- Commit `cca93bc4`, pushed.
- `HOLD` found on `origin/cloud/db-base-20261024` at this save point: stopped here, waiting for
  `GO` with no `HOLD`.
- NEXT: write `crates/pull/src/copies_register_tests.rs` (the eight named tests first, failing
  before the module, then the extreme cases), run `cargo test -p pull -p store --locked`.

### 2026-10-24 — resumed, save points 2 and 3, then HELD again

- 12:37 IST the Mac lifted HOLD ("activate all the sessions"); work resumed from NEXT.
- The README's new section (the operator's words of 24 Oct, "no typed number outside the Winner
  Gate") was applied: removed `MAX_COPY_ROOTS`, the walk's entry and depth caps, the list caps, the
  receipt read cap and the statement cap. Commit `ad16891c`.
- Wrote `crates/pull/src/copies_register_tests.rs`: the eight named tests and 16 extreme cases.
- `cargo test -p pull --lib --locked copies_register`: `test result: ok. 24 passed; 0 failed`.
- `cargo test -p store --lib --locked settings`: `22 passed; 3 failed`. The three
  (`a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`,
  `a_disk_that_refuses_the_write_is_refused_with_nothing_visible`,
  `a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`) are permission-bit tests;
  this container runs as root, which ignores permission bits. To be shown red on the unmodified
  base in the final gate run (not yet shown).
- `cargo clippy -p pull -p store --all-targets --locked`: no warning.
- `HOLD` back on the base at 12:55 IST ("HOLD for parts without GO-<NAME>"); no `GO-CC-5`. Stopped.
- NEXT: the api route `crates/api/src/copies_route.rs` + `copies_route_tests.rs`, its two
  `server.rs` lines, its rows in `operation_audit_tests.rs` and `pool_table_tests.rs`.

### 2026-10-24 — resumed after the 17:30 reset, save point 4, then HELD again

- 17:30 IST HOLD lifted ("every cloud part builds"). Merged the newest `origin/feat/db-fix`
  (`7b397bbe`, Merge CLOUD-MUTCLI) into the branch: clean, merge commit `c03ceb09`.
- Wrote `crates/api/src/copies_route.rs` and `copies_route_tests.rs`; wired `server.rs`,
  `operation_audit_tests.rs`, `pool_table_tests.rs`.
- `cargo test -p api --lib --locked copies_route`: `test result: ok. 5 passed; 0 failed`.
- `cargo test -p api --lib --locked -- pool_table operation_audit settings_route`:
  `test result: ok. 55 passed; 0 failed`.
- `cargo clippy -p api --all-targets --locked`: one test warning (`manual_assert_eq`), fixed.
- `HOLD` back at 18:15 IST ("HOLD for parts without GO-<NAME>"); no `GO-CC-5`. Stopped.
- NEXT: the cli verb `crates/cli/src/disk_copies_command.rs` (zero arguments; a fresh walk; exit 1
  on a hazard, a wait or a root not walked whole), its dispatch arm, `COMMANDS` 60 -> 61, its usage
  line, `store_write.rs` reader list, `sweep_wiring_tests.rs` non-sweep list, the zero-argument
  exemption in `a_known_command_with_the_wrong_arity_is_not_called_unknown`; then R4-06's
  `no_scheduler_delete.rs` rows.

### 2026-10-24 — GO-CC-5 (19:47 IST), save point 5

- `GO-CC-5` appeared on the base: CC-5 builds while HOLD stands for parts without their own GO.
- Wrote `crates/cli/src/disk_copies_command.rs`; wired `cli/src/lib.rs`, `store_write.rs`,
  `sweep_wiring_tests.rs`; added the three files to R4-06's `no_scheduler_delete.rs`.
- `cargo test -p cli --lib --locked -- disk_copies a_known_command every_command_is_listed the_words`:
  `test result: ok. 7 passed; 0 failed`.
- `cargo test -p cli --lib --locked -- disk_copies every_command a_known_command store_write sweep`:
  `test result: ok. 85 passed; 0 failed`.
- `cargo test -p pull --test no_scheduler_delete --locked`: `test result: ok. 5 passed; 0 failed`.
- `cargo clippy -p cli -p pull --all-targets --locked`: no warning.
- NEXT: the /ingest panel (`web/src/routes/ingest/panels/Copies.svelte`, `web/src/lib/copies.js`,
  `web/tests/copies.test.js`, the rebuilt `web/build`), then the bench `crates/api/benches/ratio/copies.rs`.

### 2026-10-24 — save point 6, then HELD (every GO-<NAME> removed, 20:20 IST)

- The /ingest panel by a parallel subagent: `npm ci --prefix web` ok; `npm --prefix web run build`
  exit 0 (the panel compiles); `node --test web/tests/*.test.js` on Node 24 (CI's):
  `tests 1578, pass 1578, fail 0`; on this container's Node 22: `pass 1573, cancelled 5`
  (`ask.test.js` 3, `census-worker.test.js` 2; files the panel does not touch). W4 `0 (ceiling 0)`,
  W5 `No comment swallows a rule`. W3: `svelte-check errors: 32 (ceiling 0)`, all 32 in files this
  part does not touch (0 in the panel's): red before this part.
- **BASE RED, W1: the committed `web/build` is stale on the unmodified base.** Committed
  `version.json` `80659b2a3b706360`; the base's own source builds `fe2953bf00677395` (eight `/db`
  sources changed since R3-03's rebuild of 19 Oct with no rebuild: R3-04 `f68b710f`, R3-30
  `a258ca1c`, among them). The build is reproducible (two builds byte-identical). This part does
  NOT commit `web/build` (rebuilding it would rewrite other parts' output); with the panel the
  bundle builds as `858702fad398a0d9`. NEEDS THE MAC: rebuild `web/build` at merge-back.
- Shell gates (the extracted `run:` blocks of ci.yml's first job, run on this branch and on an
  unmodified worktree of `origin/feat/db-fix` `7b397bbe`): red on BOTH, base reds owned by
  BASE-REDS-1023: 1c, 1d, 10, 11, 12, 19, 23, 27b, 30. On this branch, gates 10, 11, 12, 19, 27b and
  30 add no finding of this part. Gate 1d adds 55 literals of this part's files to the base's 466
  undeclared (declaring them is an edit of `.github/gates_tree.rs`, left with the gate's owner).
  Gate 23: the `println!` of the timing test was removed (no new finding). Gate 28 was red on this
  branch (`"tmp"` in `is_partial`): fixed by reading partial names from the data. Gate 1 was red only
  while the subagent's rebuild had removed tracked bundle files; `git checkout -- web/build`
  restored them. Not yet re-run after these fixes.
- Bench `cargo bench -p api --bench ratio --locked --no-run -j 1`: `Finished bench profile ... in
  19m 58s` (with 4 jobs the release link of `cli` was killed for memory, SIGKILL).
- Commit `da57ae23`, pushed.
- The served bench run started before the hold (`cargo bench -p api --bench ratio --locked --
  --served-only`) recompiled after the last edit and was killed (SIGKILL: two release links of
  `cli` at once ran out of memory): exit 101, NO figure taken.
- NEXT: build the bench with `-j 1`, then read the served bench's copies rows (`cargo bench -p api --bench ratio --locked -j 1 --
  --served-only`, started before the hold) and write `docs/06-limits.md` L-CC5 rows; re-run the
  shell gates; merge the newest `origin/feat/db-fix`; then fmt, clippy, the workspace tests, deny,
  coverage (nightly installed for `--branch`) and mutants on the touched modules.

### 2026-10-25 — resumed (GO, HOLD lifted 13:1x IST)

- New container; `HOLD` found at start, polled; `GO` with no `HOLD` (base `1dcc33e9`, "Lift HOLD: all
  nine cloud parts build") at the first poll.
- Merged the newest `origin/feat/db-fix` (`666ddf7f`, Merge DATA-1w): two both-sides-added conflicts
  (`docs/04-invariants.md`: CC-5's section beside R3-13's; `docs/05-decisions.md`: D-7040..D-7048
  beside D-7300), resolved by keeping both whole.
- After the merge: `cargo build --workspace --locked --all-targets`: no warning, no error.
  `cargo fmt --all --check`: exit 0. `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings`:
  `Finished`, no warning.
- Shell gates re-run (the `run:` blocks of ci.yml's `language-purity` job, `GITHUB_ENV` emulated so
  gate 0's `SOURCE_SCAN` reaches the rest; gate 1e not run here, it builds a second worktree), on
  this branch and on an unmodified worktree of `origin/feat/db-fix` `666ddf7f`: red on BOTH, the same
  nine, base reds: 1c, 1d, 10, 11, 12, 19, 23, 27b, 30. Findings diffed line by line (paths and
  counts normalised): 1c, 10, 30 identical; 11, 12, 19, 23, 27b differ only in their scanned-line and
  file counts (12's REFUSED list identical modulo line numbers); 1d: 521 undeclared on this branch
  against 466 on the base, the 55 literals of this part's files (left with the gate's owner, as
  recorded at save point 6). Every other gate (0, 1, 1b, 1g, 1f, 2, 9, 9b, 10b, 7, 13, 15, 16, 17, 21,
  22, 24, 28, 25, 27, 26, 14, 29, 29b): exit 0 on this branch.

### 2026-10-25 — new container, save point 7 (GO, no HOLD)

- New container (no target, no gate tools): reinstalled `cargo-deny` 0.20.2, `cargo-llvm-cov` 0.9.1,
  `cargo-mutants` 27.1.0 from crates.io; nightly with `llvm-tools-preview` for `--branch`.
- Merged the newest `origin/feat/db-fix` (`bf9b1c8f`, Merge R4-18): textually clean, merge commit
  `b8f3b34f`. R4-18 added its own verb (`zerodha-quarantine`), so `COMMANDS` is now `[&str; 64]`
  (62 base + R4-18 + CC-5); commit `52718be3`, pushed.
- `cargo build --workspace --locked --all-targets -j 4`: `Finished`, exit 0, no warning.
- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings`: `Finished`, exit 0.
- `cargo deny check`: `advisories FAILED, bans ok, licenses ok, sources ok`; the one error is
  RUSTSEC-2026-0386 (tokio 1.53.1), the BASE item BASE-DENY-1024 owns (Cargo.lock is the Mac's).
- `cargo test --workspace --locked -j 4 --no-fail-fast`: exit 101, 6 targets red, 31 tests:
  29 failed and 2 hung (killed after 15 min each). EVERY ONE is red on an unmodified worktree of
  `origin/feat/db-fix` `bf9b1c8f` too (its test binaries built and run with the same `--exact`
  names, `--test-threads=1`): the same 29 FAILED, and the 2 hung (`timeout 240`, exit 124). None
  is in a file of this part. They are permission-bit tests (this container runs as root), the
  slow-reader socket tests and two full-pool child tests:
  - api lib (8 + 1 hung): `detail::tests::a_refused_admission_names_what_its_bound_and_why`,
    `pullrun::restart_tests::the_journals_own_extremes`,
    `server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted`, the five
    `server::head_deadline_body_tests::*`; HUNG
    `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`
    (its child waits on a barrier; gdb: the body's `Release` guard waiting in `drop`).
    (`logs::r3_24_tests::a_two_week_old_refusal_is_found_by_run_day_symbol_and_code` failed under
    the full parallel run only, and passes alone on both.)
  - api `--test cloud_mutants_bars` (1 hung):
    `spot_rs::spot_json_is_refused_by_name_when_every_store_read_slot_is_held`.
  - cli lib (3): `scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger`,
    `watchdog_command::tests::a_read_only_state_directory_sends_no_notice_at_all_rather_than_many`,
    `watchdog_command::tests::a_run_never_lists_the_notices_or_reads_the_log`.
  - pull lib (9): `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`,
    `gdfl_cm::tests::fixtures_are_built_not_pasted`, `gdfl_dropbox::tests::repair_r1::a_journal_that_cannot_be_written_stops_the_press_before_any_byte`,
    `gdfl_dropbox::tests::repair_r1::a_read_only_store_root_is_not_writable_not_lost`,
    `gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name`,
    `ingest::census_store::link_tests::a_read_only_partition_directory_is_refused_with_the_host_s_words`,
    `ingest::census_store::link_tests::an_idempotent_requirement_opens_nothing_for_writing`,
    `zerodha_verify::tests::a_refused_open_keeps_the_hosts_words`,
    `zerodha_verify::tests::a_write_beside_a_merge_that_fails_is_named`.
  - store lib (5): `quarantine::tests::an_index_publish_refused_after_a_dropped_line_is_finished_by_the_next_apply`,
    `quarantine::tests::an_index_publish_refused_after_a_whole_line_is_finished_by_the_next_apply`,
    and the three `settings::tests::*` permission tests named at save point 3 (now SHOWN red on the base).
  - telemetry lib (4): `sink::tests::a_coverage_file_is_read_line_by_line_and_extended_where_it_stands`,
    `sink::tests::a_failed_delete_is_counted_named_and_left_for_the_next_start`,
    `sink::tests::a_rename_aside_that_cannot_happen_fails_the_roll_by_name`,
    `sink::tests::an_audit_stream_that_cannot_be_looked_for_is_an_error`.
  Every test of this part's files passes in that run (`copies_register::` 25 tests ok, `copies_route` 5,
  `disk_copies_command` 3, `settings::tests::the_extra_copy_roots_key_is_checked_and_stored_canonically`,
  `no_scheduler_delete` 5).
- NEXT: the served bench (`cargo bench -p api --bench ratio --locked -j 2 -- --served-only`, running),
  its copies rows into `docs/06-limits.md` L-CC5; then coverage and mutants on the touched modules
  (disk is 11 GB free: target/ is cleared between them).

### 2026-10-25 — after save point 7 (commits the earlier session pushed without a log entry)

- `9b03bdd3`: `docs/06-limits.md` L-CC5-1..4 with the gate-8 bench's measured figures (the served
  bench, `cargo bench -p api --bench ratio --locked -j 2 -- --served-only`; figures in the rows).
- `7fdc5408`: a coverage run found reachable lines of `copies_register.rs` with no test; 11 tests
  added; `answer()` asks the walk lock (never waiting) before it compares the held answer.
- `532b9fe0`: tests for the unreached lines of `copies_route.rs` and `disk_copies_command.rs`.
- `5233e81b`: `cargo mutants` found `replace > with >= in copy_roots` surviving; a test holds the
  byte ceiling exactly (shown red with the mutant applied by hand, green without).

### 2026-10-25/26 — new container, save point 8 (GO, no HOLD; base `fe6d379e`)

- New container again: reinstalled `cargo-deny` 0.20.2, `cargo-llvm-cov` 0.9.1, `cargo-mutants`
  27.1.0 from crates.io, nightly with `llvm-tools-preview`.
- Merged the newest `origin/feat/db-fix` (`13dcf5e5`, CLOUD-MUTAPIB merge fix): textually clean,
  merge commit `9e6f55fc`, pushed.
- `cargo build --workspace --locked --all-targets -j 3`: `Finished`, no warning.
- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings`: `Finished`, no warning.
- This part's tests after the merge: `copies_register` 35 passed; `copies_route` 8 passed;
  `cli -- disk_copies every_command a_known_command store_write sweep` 87 passed;
  `store settings::tests` 22 passed, 3 FAILED (the three root-permission tests shown red on the
  base at save point 7; `the_extra_copy_roots_key_is_checked_and_stored_canonically` ok);
  `no_scheduler_delete` 5 passed; `api -- pool_table operation_audit settings_route` 55 passed.
- `cargo test --workspace --locked -j 4 --no-fail-fast -- --skip a_full_calendar_pool_refuses_the_census_and_the_audit --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held`
  (the two skipped hang on the base, save point 7): `EXIT=101`; red targets: api lib (12 failed of 2,945),
  cli lib (29 of 2,406), cli `--test gdfl_run_logs` (2 of 4), pull lib (12 of 2,108), store lib (5 of 431),
  store `--test sealed_door` (1 of 1), telemetry lib (4 of 225). None is in a file of this part.
  The 29 named at save point 7 recur (shown red on the base there). NEW since the merges of
  `13dcf5e5` (CLOUD-MUTAPIB and the parts before it), NOT YET shown red on an unmodified base:
  - `a_run_whose_sink_is_held_refuses_before_any_lock`
  - `audit::cloud_mutant_tests::an_unreadable_journal_refuses_the_index_build_and_says_so`
  - `both_second_formats_answer_the_same_bars_at_p99`
  - `gdfl_import_command::cloud_mutant_tests::the_receipt_names_the_workers_and_durability_it_filed_with`
  - `gdfl_import_command::guard::tests::a_change_is_never_accepted_by_a_run_that_skipped_no_day`
  - `gdfl_import_command::guard::tests::a_changed_archive_with_every_day_done_is_named_not_ok`
  - `gdfl_import_command::guard::tests::a_run_that_read_no_file_is_not_accepted_by_running_it_again`
  - `gdfl_import_command::guard::tests::a_source_root_that_vanishes_after_files_were_read_is_named_source_lost`
  - `gdfl_import_command::guard::tests::an_empty_range_owes_nothing_and_exits_ok`
  - `gdfl_import_command::guard::tests::every_day_already_refused_identically_still_fails_when_the_root_is_gone`
  - `gdfl_import_command::guard::tests::one_archive_reached_by_another_spelling_is_the_same_key`
  - `gdfl_import_command::tests::a_dangling_day_file_link_ends_the_run_failed_never_a_day_not_held`
  - `gdfl_import_command::tests::a_named_failure_or_a_refused_run_exits_failed`
  - `gdfl_import_command::tests::a_run_says_a_line_a_day_and_leaves_its_progress_record`
  - `gdfl_import_command::tests::a_run_that_read_no_file_never_exits_ok`
  - `gdfl_import_command::tests::an_unstamped_build_is_refused_a_store_write_unless_allowed`
  - `gdfl_import_command::tests::the_retry_flag_is_never_a_symbol_and_the_run_line_names_the_source`
  - `gdfl_import_command::tests::the_stored_flag_narrows_a_capital_market_run_to_what_d3333_adds`
  - `gdfl_run::tests::a_clock_step_back_never_aborts_a_run`
  - `gdfl_run::tests::a_progress_file_that_cannot_be_written_is_named_and_the_run_goes_on`
  - `gdfl_run::tests::the_kinds_run_in_order_on_the_calling_thread`
  - `pullrun::cloud_mutant_tests::newest_records_that_cannot_be_read_leave_the_answer_not_known`
  - `server::cloud_mutant_tests_c::a_store_that_takes_no_write_trips_the_store_breaker`
  - `server::cloud_mutant_tests_c::a_store_whose_audit_dir_cannot_be_made_is_at_fault`
  - `tick_second_check::tests::a_lock_name_that_is_a_link_a_directory_or_a_second_name_is_refused_and_nothing_goes_through_it`
  - `tick_second_check::tests::a_month_held_only_sealed_is_checked_row_for_row_as_its_raw_form_was`
  - `tick_second_check::tests::a_new_landing_of_a_checked_day_is_checked_again`
  - `tick_second_check::tests::a_pipe_or_a_link_at_one_of_the_checks_own_names_and_a_pipe_archive_are_refused_at_once`
  - `tick_second_check::tests::a_share_directory_the_store_refuses_is_one_cell_and_files_nothing_twice`
  - `tick_second_check::tests::a_share_whose_months_cannot_be_listed_is_filed_month_by_month_with_or_without_a_year`
  - `tick_second_check::tests::a_tick_store_landing_is_checked_against_that_tick_store_day_and_a_changed_day_is_refused`
  - `tick_second_check::tests::a_version_whose_rows_are_not_the_ones_its_index_names_is_refused`
  - `tick_second_check::tests::an_extra_archive_is_compared_beside_the_landing_and_labelled_not_the_landing_source`
  - `tick_second_check::tests::every_status_and_source_is_named_against_the_landing_the_store_recorded`
  - `tick_second_check::tests::no_worker_count_and_no_resumption_changes_a_byte_of_the_version`
  - `two_cli_kinds_at_once_both_log`
  Several are permission or read-only tests (this container runs as root); `cli` lib took 724 s
  under the parallel run. To be shown red on an unmodified worktree of `origin/feat/db-fix`.
- `HOLD` on the base with no `GO-CC-5` (fetched at the end of this run): stopped here.
- NEXT: build an unmodified worktree of `origin/feat/db-fix` and run the NEW reds above by
  `--exact` name with `--test-threads=1` (each one red there too, or a fix here); then
  `cargo deny check`; coverage (`cargo +nightly llvm-cov --branch`) and `cargo mutants --in-diff`
  on the touched modules (clear `target/` between, 11 GB free); the CI shell gates; the final report.

### 2026-10-26 — new container (new account), save point 9 (GO, no HOLD; base `06af67c3`)

- 19:3x IST: new container, branch tip `2b251358` checked out; `GO` with no `HOLD` on the base.
  Reinstalled `cargo-deny` 0.20.2, `cargo-llvm-cov` 0.9.1, `cargo-mutants` 27.1.0 from crates.io and
  nightly with `llvm-tools-preview`.
- Merged the newest `origin/feat/db-fix` (`5901ec77`, Merge BASE-FIXTURE-1025; brings IA-10c, R4-22):
  one conflict, `docs/06-limits.md` (both sides appended a section at the end), resolved by keeping
  both whole, the base's first (D-4891 (2)). Merge commit `e7f5311c`, pushed.
- `cargo build --workspace --locked --all-targets -j 4`: `Finished`, 20 m 28 s, 0 warnings.
- `cargo fmt --all --check`: exit 0.
- `cargo deny check`: `advisories FAILED, bans ok, licenses ok, sources ok`, the one error
  RUSTSEC-2026-0386 (tokio). The same on an unmodified worktree of `origin/feat/db-fix` `5901ec77`
  (`Cargo.lock` byte-identical): the BASE item BASE-DENY-1024 owns.
- CI shell gates (the `run:` blocks of ci.yml's `language-purity` job, `GITHUB_ENV` and
  `RUNNER_TEMP` emulated; gate 1e not run, it builds and tests the whole workspace a second time),
  on this branch and on an unmodified worktree of `5901ec77`: the SAME ten red on both: 1c, 1d, 10,
  11, 12, 19, 23, 27b, 29, 30. Gate 29 is new on the base (R4-18's `/pull/quarantine.json` and
  `/pull/quarantine` have no growth bench; the tool's main run names those two on both, 102 routes
  here against 101, `/disk/copies.json` with no finding). 1c identical; 10, 11, 12, 19, 23, 27b, 30
  differ only in scanned counts and test order. Gate 1d named 543 undeclared literals here against
  482 on the base: the 61 more are all this part's, each read, now declared in
  `.github/gates_tree.rs` (`CC5_LITERAL`, `DECLARED` 77 -> 78, as R4-28 and IA-10c declared theirs):
  after it gate 1d's undeclared list is the base's own, 482, the same set. Gate 0 (the tool's own
  tests) exit 0; gate 6c's `rustfmt --check` and `clippy-driver -D warnings` (both builds) clean on
  the file. Commit `3d683d38`. Every other gate: exit 0 here.
- `cargo mutants --list --in-diff` (the diff of `crates/*/src/*.rs` against `origin/feat/db-fix`):
  270 mutants (copies_register.rs 210, copies_route.rs 21, settings.rs 14, disk_copies_command.rs
  11, settings_route.rs 8, server.rs 4, cli lib.rs 2).
- NEXT: the workspace test run (running); clippy; the new reds on an unmodified worktree of
  `5901ec77` by `--exact` name with `--test-threads=1`; web W1-W5; coverage; mutants; the report.

### 2026-10-26 — save point 10: the workspace run, the base comparison, the web gates

- `cargo test --workspace --locked -j 4 --no-fail-fast -- --skip a_full_calendar_pool_refuses_the_census_and_the_audit --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held`:
  `EXIT=101` in 31 m 52 s; red targets api lib (12 failed of 2,950), cli lib (3 of 2,411), pull lib
  (9 of 2,158), store lib (5 of 431), telemetry lib (4 of 225); every other target ok. The save-point-8
  reds of `gdfl_import_command`, `gdfl_run`, `tick_second_check`, `gdfl_run_logs` and `sealed_door`
  do not recur after the merge of `5901ec77`.
- `cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings`: `Finished`, no warning.
- The 33 and the 2 hangers, each alone (`--exact`, `--test-threads=1`, `timeout 300`, from the crate
  root), on this branch: 33 FAILED, 2 TIMEOUT. On an unmodified worktree of `5901ec77` (its own
  target: `cargo test --no-run --locked -p api -p cli -p pull -p store -p telemetry --lib` and
  `-p api --test cloud_mutants_bars`): the same 33 FAILED and the same 2 TIMEOUT, outcome for
  outcome. None is this part's. The new one since save point 8,
  `gdfl_run::ia2_tests::every_id_the_runs_ledger_gives_is_written_down_twice` (IA-10c), sets a
  claims file read-only and expects the write refused: root writes it.
- Web on Node 24.22.1 (`nvm install 24`; ci.yml's version), this branch / the base: W1 red / red
  (the committed bundle is stale on the base itself); W2 `pass 1611, fail 0` / `pass 1601, fail 0`;
  W3 `svelte-check errors: 44` / the same 44, line for line, none in this part's files; W4, W5, W6
  exit 0 / exit 0. `web/build` restored after each build (`git checkout -- web/build`).
- The report's top rewritten for the final run (gates table, measurements, extreme cases, files
  outside ownership, D-numbers, NEEDS THE MAC).
- NEXT: coverage (`cargo +nightly llvm-cov --branch`, running), then the served bench on the final
  tree, then `cargo mutants --in-diff` over the 270 mutants, then STATUS.

### 2026-10-27 — new container (new account), save point 11 (GO, no HOLD; base `ede2909b`)

- 08:0x IST: new container, branch tip `29d0808a` checked out; `GO` with no `HOLD`. The coverage run
  of save point 10 was cut with no saved result, so it is re-run here from the start. Reinstalled
  `cargo-deny` 0.20.2, `cargo-llvm-cov` 0.9.1, `cargo-mutants` 27.1.0 from crates.io; nightly
  (`rustc 1.101.0-nightly 2026-10-26`) with `llvm-tools-preview`.
- Merged the newest `origin/feat/db-fix` (`a32722c45`, Merge R3-15; brings R4-19, R3-12,
  BASE-DENY-1024, R3-15). Two conflicts, both sides appending: `docs/06-limits.md` (R4-19's and
  R3-12's sections beside L-CC5-1..4; both kept whole, the base's first) and `.github/gates_tree.rs`
  (R4-19 and CC-5 each raised gate 1d's `DECLARED` 77 -> 78 for their own group; both groups kept,
  `DECLARED` 78 -> 79). No new verb, route or settings key on the base side, so `COMMANDS`, the
  route lists and `KEYS` are unchanged. Merge commit `48e4cdb09`, pushed.
- `cargo fmt --all --check`: exit 0. `cargo deny check`: `advisories ok, bans ok, licenses ok,
  sources ok` (tokio 1.53.3 from BASE-DENY-1024): GREEN.
- `cargo build --workspace --locked --all-targets -j 4`: `Finished` in 22 m 06 s, exit 0, no warning.
- CI shell gates (34 `run:` blocks of ci.yml's `language-purity` job, split by a bash read loop,
  `RUNNER_TEMP`/`GITHUB_ENV` emulated, gate 1e not run) on this branch and on an unmodified worktree
  of `a32722c45`: the same ten red on both (1c, 1d, 10, 11, 12, 19, 23, 27b, 29, 30); every other gate
  exit 0 on both. Each red gate's output, sorted with digits normalised and test lines dropped, is
  IDENTICAL on the two trees (0 lines differ); gate 1d's undeclared set is 482 on both, the same set.
- Coverage (`CARGO_PROFILE_DEV_OPT_LEVEL=0 cargo +nightly llvm-cov --branch --no-report --locked`
  per package with this part's test filters, then `cargo +nightly llvm-cov report --branch`): first
  read `copies_register.rs` 100% lines and branches with 17 regions unexecuted, all `?` early returns
  (`SourceIdentity::parse` at each field; two `?`s re-deriving a receipt's kind that could not fail;
  `Host::list`'s refusals). Fixed in `da59d4853` (behaviour unchanged): the identity's seven fields
  are read into a fixed array so every exit is reachable, the walk keeps each receipt's kind beside
  its path, and a new test drives every exit. Re-run on that commit: see the gates table.
- NEXT: the workspace test run (running), web W2 with a browser (running), the served bench, and
  `cargo mutants` per package (`pull` running; then `store`, `cli`, `api`).

### 2026-10-27 — save point 12, then HELD (HOLD at 10:3x IST until the 12:50 IST reset)

- `cargo clippy --workspace --all-targets --locked -j 3 -- -D warnings` on `da59d4853`: exit 0, no
  warning; again on `0e2cdec48` (target `target/quick`): exit 0.
- `cargo test --workspace --locked -j 3 --no-fail-fast -- --skip <the two hangers>` on `da59d4853`:
  `EXIT=101`, 3,275 s; 64 failed: the 33 known root-only reds (all present) and 31 more
  (`gdfl_import_command` x15, `tick_second_check` x11, `gdfl_run` x3, `gdfl_run_logs` x2: save point
  8's list). Cause found: `pull::gdfl_import::DEFAULT_FREE_FLOOR` is 10 GiB, checked before a run's
  first day, and the disk had 6.6 GB free during the run (the mutants' build folders grew beside
  it), so every GDFL import those tests run stopped before its first file ("files read 0").
  All 31 pass alone (`--exact`, `--test-threads=1`, `timeout 300`, from the crate root, a clean
  committed tree, 19 GB free): 31 of 31 exit 0. Not this part's.
- Web on Node 24.22.1 with `BRUTEX_CHROME=/opt/pw-browsers/chromium` (R3-15's browser tests need one):
  W2 here 1,805 of 1,806, the base 1,795 of 1,796; each tree's one failure a timing test under the
  load of the mutants runs (here `a keystroke never walks the census...`, the base `the Speed
  page's own line in a real browser...`), and both pass alone on both trees.
- **R3-15's page budget, met by this part** (D-7049): with the Copies panel as first built `/ingest`
  weighed 608.0 KB on first load against its 600.0 KB budget and the web build failed (W4 red here,
  green on the base). The panel now carries none of the register's code on the first load
  (`$lib/copies.js` on the first poll, `$lib/CopiesTables.svelte` when a walk is drawn): `/ingest`
  595.0 KB, the base 591.0 KB; every other page as on the base. Web gates on that tree: W4, W5, W6
  exit 0; W3 the same 58 svelte-check errors as the base, none in this part's files; W1 red on both
  (the committed bundle is stale on the base itself).
- Served bench (`cargo bench -p api --bench ratio --locked -j 2 -- --served-only`, release, on
  `7f861a620`): the run stops at the base's `reads` row (`too_many_rows`: 102,400 bars past
  `/bars.json`'s 60,000) before the copies rows, as on 25 Oct; re-run with that one line skipped in
  a local uncommitted edit (restored at once, `git status` clean after): copies rows n = 200,
  p50 / p99 / max 94,719 / 227,284 / 356,565 ns at 1x, 108,061 / 307,029 / 860,941 at 10x,
  109,486 / 255,025 / 2,463,316 at 100x; p50 ratio 1x->100x 1.155x (ceiling 3x: ok), cold walk
  1,558 / 2,362 / 19,079 µs. The later commits change no code on the served path.
- `cargo mutants` (27.1.0, `--cap-lints true`, `CARGO_PROFILE_DEV_OPT_LEVEL=0`
  `CARGO_PROFILE_DEV_INCREMENTAL=true` `CARGO_PROFILE_DEV_DEBUG=0` by environment, per package with
  test filters that keep the baseline green): `store` 14 caught of 14; `cli` 12 caught, 1 unviable;
  `api` 23 caught, 8 unviable, 2 timeouts; `pull` (on `da59d4853`) 168 caught, 30 unviable,
  6 missed, 6 timeouts. Every miss and timeout answered in `0e2cdec48` (commit message): five
  missed now killed by new tests, one equivalent guard removed; the hangs fail at once. Rechecks:
  the 32 mutants of the touched `pull` functions 28 caught, 4 unviable; `note_walked` caught by the
  new events test; both `api` `answer` mutants caught.
- Coverage of `copies_register.rs` again on the final code: lines 1003/1003, branches 136/136,
  functions 119/119, regions 1006/1007 (the one left: `entry?` in `Host::list`).
- Ledgers: CC5-03 widened, CC5-17..19 added (`docs/04`); D-7049 (`docs/05`). Gates 1d, 10, 12, 27,
  27b re-run on the final docs: the base's own findings, nothing added.
- RUNNING when HOLD came: the whole `pull` mutant set on the final code (207 mutants, `--lib`
  and `--test copies_register_events`), in the background.
- NEXT (after HOLD lifts): read that run's result; W2 whole on the final tree; the gates table, the
  measurements and NEEDS THE MAC at the top of this report; STATUS.
