STATUS: HELD

NEXT (after the hold lifts): merge any newer `origin/feat/db-fix`, rerun the gates and push. Every part of the build is done; the gates that are not green (coverage under 100% with branch coverage unavailable here, mutants not run, and the base reds `cargo deny`, the 27 tests and the CI shell gates) are named below and in NEEDS THE MAC.
HELD at 21:06 IST on 24 Oct, at this save point: commit `604b8f20` on the base branch removed `GO-R4-09b` while `HOLD` stands.

# R4-09b — the store's backup verb and its restore drill (AUT-06), cloud build

Branch `cloud/db-R4-09b-20261024`, cut from `origin/feat/db-fix` `e851da1c` (Merge DATA-2), with
`origin/feat/db-fix` merged in twice: `1c094c90` (Merge IA-4, after Merge R4-27) as `36c904bd`, then
the newest, `7b397bbe` (Merge CLOUD-MUTCLI, after Merge CLOUD-MUTSTORE), as `847cbd8e`. Before the first
merge, both sides had `COMMANDS` at 62: this part's two verbs on one side, R4-27's two on the other.
The merge sets it to 64. The second merge touched no file of
this part and kept git's default subject.
Prerequisite `R4-09a`: merged (`Merge R4-09a:` is in `git log origin/feat/db-fix --first-parent --merges`).
Decision block: D-7175..D-7184; this part uses **D-7175 to D-7181**. D-7137 is LEDGER-1023's; no second one was written.

## What is built

- **`cli backup --to <dir> [--plan] [--mib-per-s N]`** (`crates/cli/src/backup.rs`, new):
  - **What it copies.** Every month each census names, with every sibling (`FileKind::ALL` but `.lock`),
    its generation pointer and every revision with its `.repair-v1`. Then the folders `manifest/`,
    `imports/`, `audit/`, `ops/` (less the verbs' own two logs), `verify/`, `reference/` and
    `session-masters/`. Then every archive the GDFL journal's `run` lines name: the CM zip, the
    options folder, and the tick-store root.
  - **What it does not copy is named.** Every other top-level name of the store root is printed by
    every plan and run (`NOT COVERED`).
  - **How each file is copied.** To `<dir>/brutex-backup-v1/runs/<run>/{store,archives}/...` as
    `store::staging::create`, fsync, rename (refused if the name exists), folder fsync. Then one record
    goes into the append-only `manifest.log` (path, length, ns mtime, BLAKE3, run, unit).
  - **What it never does.** Nothing on the target is deleted or overwritten. A changed file is a new
    name; a touched file (same bytes) is a new record over the held copy.
  - **The manifest.** Read once per run into a pre-sized map, then one probe per file. The map is
    reserved for the run's inserts, so no per-file step rehashes.
  - **Locks.** Each month is copied under its shared lock (the original's and every revision's). A
    writer-held month is a `busy` line, tried once more at the end, and left for the next run.
  - **Refusals, by name and before any write.** `same_device` (st_dev), `inside_store`,
    `inside_archive`, `not_a_directory`, `target_missing`, `target_path_too_long` (from the widest
    receipt the line can carry), `receipt_unwritable`, `nothing_listed`, `insufficient_space`,
    `space_unknown`, `journal_unreadable`, `target_busy` (one run per target, exclusive lock) and
    `manifest_unreadable` (a line this build cannot read).
  - **Cut runs.** A killed run's torn manifest tail is ended as a void line by the next run, and an
    unfinished write inside a run is ended before the next line. A run the target stops appends
    `stopped`, with no receipt.
  - **`--plan`.** Counts and bytes per class, new and changed files, free space, and the names not
    covered. It writes nothing on either side.
  - **The receipt.** On reaching its end a run appends ONE receipt to `<store>/ops/backups.log`, in
    the runbook's own line: `copied=<UTC> dest=<path> raw_bytes= store_bytes= by=cli-backup run=
    state=<complete|partial> files= bytes= copied_files= copied_bytes= unchanged= busy= missing=
    unreadable= commit=`. A partial run exits 1.
- **`cli backup-drill --from <dir> [--months N]`** (N = 3), same file:
  - **Which months.** N months chosen from the manifest across kinds (segment), rotated by the drill
    id; busy months are skipped and named.
  - **Where they go.** A new folder under the system temp dir, named by the drill id and the process
    id, refused inside the store or the target, and left for inspection.
  - **What it checks.** Every file against its manifest BLAKE3, and against the store byte for byte
    under the month's shared lock. A store file that moved since the copy is checked against the
    digest only and counted `moved`.
  - **Its record.** One line to `<store>/ops/backup-drills.log`
    (`drilled= from= drill= result=<pass|fail|incomplete> months= asked= held= files= bytes= moved=
    failed= busy= scratch= commit= month=...`, capped at 1,024 bytes with `more=`).
- **Registration** (`crates/cli/src/lib.rs`): `mod backup;`, the re-exports (`BackupReceipt`,
  `BackupDrillLine`, `BACKUP_RECEIPTS`, `BACKUP_DRILLS`, `backup_decoded`), two dispatch arms, two
  `USAGE` entries, and `COMMANDS` 64 (60 + R4-27's two + these two).
- **`/disk.json`** (`crates/api/src/disk_route.rs`, read side only): `backup` keeps `recorded`, `last`
  and `why`, and gains:
  - `receipt`: every word, `null` when a hand receipt lacks it;
  - `target`: `present`, `missing`, `not_a_directory` or `unreadable`, from one `stat` of the decoded
    `dest=`;
  - `age_days`;
  - `stale` and `stale_why` by name ("backup stale: last copy <date>; target missing", "; N days old
    (more than 7)", "; the copy is dated after now", "; the last run was partial");
  - `stale_after_days`;
  - `drill`: the newest whole line of `ops/backup-drills.log`, or `recorded:false`.

  It is computed when read, within `DISK_TTL` (30 s). No alert kind is added. The drill half is its
  own function (`drill_json`).
- **Web** (`web/src/lib/disk.js`, `DiskTile.svelte`, `web/tests/disk.test.js`):
  - Every page's standing line shows the server's `stale_why`, and a drill that did not pass.
  - The `/ingest` tile shows the copy's date, target, file count and bytes, and the last drill.
  - The server's `stale_after_days` wins over the page's 7.
- **Docs.**
  - `docs/36-ops-runbook.md` §4: the verb replaces the `rsync --delete-after` step. It adds the
    receipt words, the drill, and restoring into the store by hand from the manifest (a month whole,
    a tick-store day's version files together, never `rK+1` without `rK`; then census-recount and
    scrub-blocks). It adds the local-only list (FM-277).
  - `docs/04`: BK-01 to BK-35.
  - `docs/05`: D-7175 to D-7181.
  - `docs/06`: L-BK-01 to L-BK-05, and the gate-11 allowance reason.

## Decision numbers used

| D | What it locks |
|---|---|
| D-7175 | what is copied: census months with every generation, the seven listed folders, the journal's archives; what is not, named; `nothing_listed` |
| D-7176 | target layout, run-named copies, the manifest and its strict reading, torn tails, the unchanged rule, one run per target |
| D-7177 | the receipt words; partial and stopped runs; `backup` a writer except `--plan` |
| D-7178 | the drill: selection, scratch folder (R4-11's `role=scratch` marker is not built and not needed), `ops/backup-drills.log`, `backup-drill` a writer |
| D-7179 | `/disk.json` staleness by name, when read; drill shown; no alert kind |
| D-7180 | the tick store copied as a journal-named archive; restoring INTO the store is the operator's, by the manifest, a day's versions together (the answer PREC-5's builder reads) |
| D-7181 | serial copy to one target, unthrottled by default; manifest synced every 256 records or 2 s |

D-7182 to D-7184 are unused.

## Files outside ownership (minimal, each named)

| File | Why |
|---|---|
| `crates/cli/src/store_write.rs` | `"backup"`, `"backup-drill"` in `WRITERS` (19 → 21); `"backup" => !arguments.contains(&"--plan"),` (the one arm shape gate 30 reads); rows in `a_plan_or_a_dry_run_is_not_a_write_and_its_applying_form_is` and `every_writing_verb_refuses_a_store_whose_bars_folder_is_a_link` |
| `.github/gates_ledger.rs` | `ALLOW_MEMBER` for `crates/cli/src/store_write.rs` 7 → 8 with its reason (docs/06), as IA-9 raised it 6 → 7 for `verify-merge`: gate 30 requires `!arguments.contains(&"--plan")` and gate 11 rule 7 counts it. No gate logic changed. |
| `crates/cli/src/sweep_wiring_tests.rs` | `\| "backup" \| "backup-drill"` added to its list of verbs that are not sweeps (`only_actual_computation_verbs_can_establish_sweep_status` failed on the full run without it) |
| `web/src/lib/disk.js`, `web/src/lib/DiskTile.svelte`, `web/tests/disk.test.js` | spec item 3, "the /db band" (the spec's file 4.1 names them) |
| `crates/api/src/disk_route_backup_tests.rs`, `crates/cli/src/backup_tests.rs` | new test files of this part's own modules |

`config/store-steps.toml` is untouched (gate 30 refuses `merged = true` before the `Merge R4-09b:`
subject exists); the flip and the S-17 rewording are for the Mac's merge commit (below).

## Gates (this cloud host: 4 vCPU Intel Xeon 2.10 GHz, Rust 1.97.1, node 22)

Every command below was run on the branch head after the last merge of `origin/feat/db-fix`
(`7b397bbe`, Merge CLOUD-MUTCLI) unless it says otherwise. "Base red" means the same result was produced
here on the unmodified base: `1c094c90`, checked out in this checkout or a worktree.

| Gate | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all --check` | **green** (exit 0) |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | **green** (exit 0, `Finished`) |
| tests: this part | `cargo test -p cli --locked --lib -- backup:: store_write:: sweep_wiring_tests:: every_command_is_listed_in_both_places` and `cargo test -p api --locked --lib -- disk_route` | **green**: 47 backup tests (+2 ignored measurements), every store_write and sweep_wiring test, 26 disk_route tests (6 new) |
| tests: workspace | `cargo test --workspace --locked --no-fail-fast` (api run apart, see below) | **red, every failure base red**. Results: cli lib 2,434 passed, 3 failed; pull 1,982 passed, 9 failed; store 390 passed, 3 failed; telemetry 221 passed, 4 failed; api lib 2,468 passed, 8 failed; every other target, the api integration targets and the doc tests green. **Every one of those 27 failures fails on the unmodified base `1c094c90` too**, rerun there by exact name. None names a file of this part. pull, store and telemetry are byte-identical to `origin/feat/db-fix` `7b397bbe`. The cli, pull, store and telemetry failures are read-only-permission tests, and this container runs as root (uid 0), which writes through the modes they set: cli's are `scrub_blocks` unwritable ledger and two `watchdog_command` read-only state directory tests. The api lib's 8 are `head_deadline_body_tests` (5), `detail`, `pullrun::restart_tests` and `feeds_route`. Two api tests hang on this host, on the base as well (`timeout` exit 124), and were skipped by name: `a_full_calendar_pool_refuses_the_census_and_the_audit` (lib) and `spot_json_is_refused_by_name_when_every_store_read_slot_is_held` (`cloud_mutants_bars`, 58 of 58 others pass) |
| deny | `cargo deny check` | **red, base red**: `advisories FAILED, bans ok, licenses ok, sources ok`, RUSTSEC-2026-0386 (tokio `AsyncFd`, tokio 1.53.1 locked, fixed in >= 1.53.3). Same on the unmodified base `1c094c90`. This part changes no manifest and no lock (CONTEXT 4.10): NEEDS THE MAC |
| CI shell gates | every step of the `language-purity` job of `.github/workflows/ci.yml`, run in order with `RUNNER_TEMP` and `GITHUB_ENV` passed between steps (Gate 1e, a full build, not run here) | red on this branch: 1c, 1d, 10, 11, 12, 19, 23, 27b, 30. **The same set on the base**, and no refusal in any of them names a file of this part. Gate 11's `store_write.rs` is at its raised allowance 8 of 8. Gate 28 is green (`backup.rs: 1 temporary name(s), 1 call(s) of store::staging::create`). Gate 23 clause C is green for `backup.rs`. All other steps green, 14 and 29 included. Reds: 30 (IA-1, IA-4, R4-27, R3-17 flags), 27b (D-6247), 23, 11, 12, 10, 19, 1c and 1d belong to other parts (BASE-REDS-1023 per CONTEXT 4.10) |
| gate 6c | `rustfmt --edition 2024 --check .github/gates_ledger.rs`, `clippy-driver --edition=2024 [-D warnings] [--test]` on it, its test binary | **green** (46 passed) |
| web W2 | `npm test` (web, node 22) | 1565 pass, 0 fail, **5 cancelled, base red**: the same 5 are cancelled on the base (`ask.test.js` 3, `census-worker.test.js` 2; timers in this container). `node --test tests/disk.test.js`: 9 of 9 pass |
| web W3 | `npm run check` | 32 errors, **base red**: the same 32 on the base, none in this part's files |
| web W1 | `npm run build` | builds (exit 0); `web/build` reset afterwards (not this part's) |
| coverage | `cargo llvm-cov --locked -p cli --lib -- backup:: store_write:: sweep_wiring_tests::` and `-p api --lib -- disk_route`, then `cargo llvm-cov report` (stable 1.97.1, a clean profile) | **not 100%, red**. Measured on the final head (`847cbd8e`) from a clean profile: 74 cli tests and 26 disk_route tests pass. `backup.rs`: lines 90.99% (185 of 2,054 missed), regions 88.48% (401 of 3,480 missed), functions 73.82%. `disk_route.rs` (whole file, most of it older than this part): lines 88.90% (159 of 1,433 missed), regions 87.78%. `store_write.rs`: lines 98.71% (5 of 387 missed). The misses are mostly the host-refusal closures (a failed fsync, rename, stat or read that this filesystem does not produce on demand). **Branch coverage is unavailable here**: `--branch` needs a nightly toolchain, which is not installed. NEEDS THE MAC |
| mutants | `cargo mutants --list --in-diff <diff>` | **not run here**: 575 mutants (493 `backup.rs`, 71 `disk_route.rs`, 8 `lib.rs`, 3 `store_write.rs`), about two days of cli/api rebuilds on this 4-vCPU host: NEEDS THE MAC |

## Measurements

| What | n, method | p50 | p99 | max |
|---|---|---|---|---|
| per-file step (one probe of the book + one record formatted), book of 10,000 paths | 1,000 sequential calls, `measure_the_per_file_step` (`--ignored`, `BRUTEX_MEASURE_OUT`), dev profile opt-level 3 | 1.29 µs | 5.73 µs | 121.9 µs |
| same, book of 100,000 paths | same | 1.39 µs | 5.33 µs | 16.5 µs |
| copy rate (one-time pass sample) | one run, 268,581,239 bytes in 12 files (3 invented months + a 256 MiB archive), source and target on ONE cloud disk, `measure_the_copy_rate` | 169 MiB/s | | |

- The per-file step is flat from 1x to 10x.
- The structural proof is `the_per_file_step_probes_the_manifest_once`: probes equal files listed on
  a fresh manifest and on fuller ones.
- **The full copy's time is an extrapolation, not a measurement.** About 1.6 TB at 169 MiB/s is
  about 2.5 hours. The real rate depends on the second drive and on one fsync pair per file (L-BK-01:
  NEEDS THE MAC).
- `/disk.json` p50/p99/max over n ≥ 200 GETs needs a running api over a store: NEEDS THE MAC. The
  added cost per measurement is two 4 KiB tail reads and one `stat`, inside the existing 30 s cache.

## Extreme cases (each a test that fails before this part: the verbs, fields and functions did not exist)

Single cases:

1. Empty store refused (`an_empty_store_is_refused_and_nothing_is_written`).
2. A one-record month and a zero-length file copied and drilled (`a_second_backup_copies_nothing`,
   `the_drill_restores_three_months_byte_equal`).
3. Garbage parameters for both verbs (`garbage_parameters_are_misuse`).
4. Target absent, or a file (`a_target_that_is_a_file_or_absent_is_refused_by_name`).
5. A foreign or torn manifest (`a_manifest_with_a_foreign_line_refuses_and_copies_nothing`,
   `a_manifest_line_the_disk_did_not_finish_is_ended_before_the_next`).
6. A link in a listed folder (`a_link_in_a_listed_folder_is_skipped_by_name`).
7. A renamed or missing archive (`a_missing_archive_is_named_and_the_run_is_partial`).
8. An older generation and its pointer (`an_older_generation_and_its_pointer_are_copied`).
9. Too little space (`too_little_space_is_refused_before_a_byte_is_written`).
10. Two runs at once (`a_second_run_at_once_is_refused_target_busy`).
11. Touched, same bytes (`a_touched_file_is_recorded_not_copied`).
12. A torn receipt log, and a receipt log that is a link (`a_torn_receipt_log_is_ended_before_the_receipt`,
    `a_receipt_log_that_is_a_link_is_refused_before_a_copy`).
13. A tampered copy (`a_tampered_copy_fails_the_drill_by_name`).
14. More months asked than held (`a_drill_asked_for_more_months_than_held_is_incomplete`).
15. An unreadable pointer, and an unreadable journal run line (`a_month_whose_pointer_cannot_be_read_is_copied_and_named`,
    `a_journal_run_line_whose_archive_cannot_be_read_is_named`).
16. Every way a copy goes stale (api, `every_way_a_copy_goes_stale_is_named`).
17. A torn drill line (api, `only_a_whole_drill_line_is_a_recorded_drill`).
18. The year and leap edges of the instant reader (api, `an_instant_is_read_in_unix_seconds`).
19. A month the census names and the disk lacks (`a_month_the_census_names_and_the_disk_lacks_is_named_missing`).
20. A store with no `ops/`, and an `ops` that is a file (`a_store_with_no_ops_folder_gets_one_and_an_ops_file_is_refused`).
21. A census that will not load (`a_census_that_will_not_load_is_named_and_the_run_is_partial`).
22. `--mib-per-s` is paid with no month held (`the_rate_limit_is_paid_after_each_unit`).
23. A device that cannot be told (`a_device_that_cannot_be_told_is_refused`).
24. A copy gone from the target and a file gone from the store during a drill
    (`a_drill_names_a_copy_gone_from_the_target_and_a_file_gone_from_the_store`).
25. An archive that grew (`a_changed_archive_is_a_new_copy_by_name_and_size`).
26. More records than one sync batch, and a link standing for a month's sibling
    (`many_files_sync_as_they_go_and_a_linked_sibling_is_skipped`).
27. An archive path that is a link or climbs with `..` (`an_archive_that_is_a_link_or_climbs_is_named_and_not_copied`).
28. A run line that is not UTF-8, a manifest that is a folder, a capped list of names, and the binary's
    entry points refusing a misuse before reading any root (`small_refusals_and_the_machines_own_answers`,
    `the_binary_entry_points_refuse_a_misuse_before_reading_any_root`).

Combinations:

- **C1.** The target fills up mid-copy while a month is writer-held, then resumes
  (`a_target_that_fills_up_mid_copy_while_a_month_is_writer_held_then_resumes`, the spec's own
  example).
- **C2.** A kill, then a changed source, then a resume (`a_kill_then_a_changed_source_then_a_resume`).
- **C3.** A drill while one month is busy and another has moved (`a_drill_while_one_month_is_busy_and_another_moved`).
- **C4.** Too old, unplugged and partial at once (api, in `every_way_a_copy_goes_stale_is_named`).

Out of the box:

- **O1.** A target inside a source archive, the tick store's root (`a_target_inside_a_source_archive_is_refused`).
- **O2.** A target path holding a space and a percent sign stays one receipt word that decodes back
  (`a_target_path_with_a_space_and_a_percent_sign_is_one_receipt_word`).
- **O3.** The verbs' own logs are never listed, so a run never makes the next copy something (in BK-01).

Found and fixed by this part's own tests and self-check:

- A `stopped` line appended after an unfinished write was joined onto the fragment, making a
  manifest line no run could read.
- An unreadable generation pointer silently dropped its newest revision ordinal.
- A journal run line with an undecodable `archive=` was skipped in silence.

## The S-17 text prepared for the Mac's merge commit (`config/store-steps.toml`)

In `Merge R4-09b: ...`:

- set `[[package]] name = "R4-09b"` to `merged = true`;
- in S-17, replace the two `planned: cli backup ...` command lines with
  `"cli backup --to <SECOND_DRIVE>"` and `"cli backup-drill --from <SECOND_DRIVE> --months 3"`;
- replace the dry run's `planned: cli backup --to <SECOND_DRIVE> --plan (...)` with
  `"cli backup --to <SECOND_DRIVE> --plan"` (read-only by `writes_the_store`);
- set `not_rehearsed = "waits for operator: no second drive is attached"`;
- the `writes` field may add "the drill line goes to <store>/ops/backup-drills.log (D-7178)";
- then run gate 30 exactly as CI runs it.

## NEEDS THE MAC

Paths as in the spec §6: `W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`, branch checked out at
`$W/wt/P-R4-09b`, shadow port **8516**, build pool **pool-6**. The real store is read-only for every
check; `cli backup` / `cli backup-drill` are never run with the real store and a real target here (that
is S-17, the operator's).

1. **Build (pool-6, under its lock):** `export CARGO_TARGET_DIR=$W/tgt/pool-6; mkdir $W/tgt/pool-6.lock && (cd $W/wt/P-R4-09b && git ls-files -z | xargs -0 touch && cargo build --release -p api -p cli); mkdir -p $W/tgt/bin-R4-09b/release && cp $W/tgt/pool-6/release/{api,cli} $W/tgt/bin-R4-09b/release/; rmdir $W/tgt/pool-6.lock`
2. **Shadow server (GET only):** `$W/state/fix/stop.sh R4-09b; $W/state/fix/shadow.sh R4-09b && $W/state/fix/serve.sh R4-09b 8516 $W/tgt/bin-R4-09b $W/wt/P-R4-09b`.
   - `curl -s http://127.0.0.1:8516/disk.json | jq .backup` -> expected on the real store (no `ops/`): `recorded: false`, `why` naming `ops/backups.log` and "no copy of this store is recorded", `stale: true`, `stale_why: "backup stale: no copy of this store is recorded"`, `stale_after_days: 7`, `drill: {recorded: false, why: ... "no restore drill is recorded" ...}`; no date, no file count.
   - n >= 200 sequential GETs of `/disk.json` -> p50/p99/max with n (the first, cold read reported apart; the answer is cached 30 s).
   - `/db` and `/ingest` in the browser pane: the standing band reads "NO COPY of this store is recorded."; the `/ingest` tile reads "no restore drill recorded".
3. **A private root `P=$W/shadow/R4-09b-priv`** (real directories, `cp -c` copies of three months of different kinds the census lists, e.g. one NIFTY index-spot 1s month, `CASH/360ONE/1s/2025-03`, one option month, with their census rows/epochs, generation and revision files, `manifest/`, `imports/gdfl.journal`); `stat -f %d $P` on WD_BLACK. A target image: `hdiutil create -size 2g -fs APFS -volname R409B-TGT $W/shadow/r409b-tgt.dmg && hdiutil attach $W/shadow/r409b-tgt.dmg`.
   - `BRUTEX_STORE=$P $W/tgt/bin-R4-09b/release/cli backup --to /Volumes/R409B-TGT --plan` -> counts and bytes per class, `NOT COVERED` naming the root's other top-level names, "nothing was written"; `find /Volumes/R409B-TGT | wc -l` equal before/after; no `$P/ops/backups.log` line.
   - Refusals, each exit 1 and nothing written: `--to $P/sub` -> `BACKUP REFUSED (inside_store)`; `--to $W/shadow/r409b-same` -> `(same_device)`; `--to /Volumes/R409B-TGT/afile` -> `(not_a_directory)`; a 1 MB image -> `(insufficient_space)`.
   - `cli backup --to /Volumes/R409B-TGT` -> `-- complete`; `manifest.log`'s `file` lines = files listed; one receipt at the end of `$P/ops/backups.log` beginning `copied=<UTC> dest=/Volumes/R409B-TGT`; a shadow over `$P` answers `backup.recorded: true`, `receipt.files`/`bytes` equal to the run's, `target: "present"`, `stale: false`.
   - A second `cli backup` -> `copied 0 file(s)`, a receipt with `copied_files=0`.
   - `kill -9` during the largest month's copy, then a rerun -> `-- complete`; `cmp` every copy against `$P`; the run folder of the killed run holds one `.tmp` no record names.
   - Hold one month's writer lock in `$P` while `cli backup` runs -> `BUSY` names it, `state=partial busy=1`, exit 1; released, the next run copies it.
   - `cli backup-drill --from /Volumes/R409B-TGT --months 3` -> `PASS: 3 of 3 month(s) restored`, three kinds incl. the NIFTY month, one line in `$P/ops/backup-drills.log` with `result=pass months=3`; `shasum -a 256` of the three months in `$P` unchanged; the scratch folder under `$TMPDIR` named in the answer.
   - `hdiutil detach /Volumes/R409B-TGT`, then `/disk.json` over `$P` -> `stale_why: "backup stale: last copy <date>; target missing"`.
   - Per-file step and copy rate on the M4 Pro and the drive pair: `BRUTEX_MEASURE_OUT=$W/shadow/r409b-measure.txt cargo test -p cli --locked --lib -- backup::tests::measure_the_per_file_step backup::tests::measure_the_copy_rate --ignored` (the copy-rate test runs in the host temp dir; for the drive pair, time the real `cli backup` above with `/usr/bin/time -l`), for the docs/06 L-BK-01 row and its extrapolation.
   - The real store untouched: `ls /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store/ops` still absent; `find <store> -maxdepth 2 -newer <stamp>` empty.
4. **Whether the real journal names the tick store** (D-7180, UNVERIFIED here): `grep -c ' source=tickstore ' <store>/imports/gdfl.journal` and the distinct `archive=` values (`grep -o 'archive=[^ ]*' <store>/imports/gdfl.journal | sort -u`), read-only. If the tick-store root is not named, PREC-5's step records that the tick store is not covered.
5. **Gates on the Mac after the merge into `feat/db-fix`:** the README's gates; gate 30 exactly as CI runs it, after, in the merge commit `Merge R4-09b: ...`, `merged = true` and the S-17 rewording above; `cargo test -p api --test store_step_forms`.
6. **`cargo deny check`:** red on RUSTSEC-2026-0386 (tokio `AsyncFd`, tokio 1.53.1 locked, fixed in >= 1.53.3) on this branch AND on the unmodified base `1c094c90` (both run here, 24 Oct): a base red; `Cargo.lock` is not this part's (CONTEXT 4.10). Needs the lock bump by its owner.
7. **Coverage and mutants:** line/region coverage of `backup.rs` and `disk_route.rs` measured here on stable (above); branch coverage needs a nightly toolchain (`cargo llvm-cov --branch`), not installed here. `cargo mutants --in-diff` lists **575** mutants on this diff (493 `backup.rs`, 71 `disk_route.rs`, 8 `lib.rs`, 3 `store_write.rs`); at 4-6 min per cli/api rebuild on this 4-vCPU host that is about two days, so NOT RUN here: `git diff origin/feat/db-fix -- crates/ > /tmp/r409b.diff && cargo mutants --in-diff /tmp/r409b.diff --jobs 4` on the Mac or the CI mutants job.
8. **Web:** W1 (`web/build` regenerated: this part reset it, the deploy's rebuild is the main session's); W2 here 1565 pass, 0 fail, 5 cancelled, the same 5 cancelled on the unmodified base (`ask.test.js` 3, `census-worker.test.js` 2: timers in this container); W3 32 svelte-check errors, the same 32 on the base, none in this part's files.
9. **Deploy and look (main session):** open `/db` and `/ingest` on `:8080` and send the operator a screenshot of the band ("NO COPY of this store is recorded." until S-17 runs).
10. **The store write itself is the operator's (S-17):** the operator names the second drive; `cli backup --to <SECOND_DRIVE> --plan`, a measured sample, the first full copy with every writer stopped, one `cli backup-drill --from <SECOND_DRIVE> --months 3`, then S-17's census-recount and scrub-blocks checks on the copy. Size about 1.6 TB or more (an extrapolation `--plan` measures first).
11. **The base reds this host shows (not this part's):** the 27 test failures above (the cli, pull, store and telemetry ones are read-only-permission tests run as root; the api lib's 8 fail by the same names on the base, cause not investigated), the two api tests that hang here on the base too, `cargo deny`, the CI shell gates 1c, 1d, 10, 11, 12, 19, 23, 27b and 30, web W2's 5 cancelled and W3's 32 errors. Each is the same on the unmodified base. On the Mac, run as the operator's user rather than root, the permission tests are expected to pass, an expectation not checked here.
12. **The merge commit `847cbd8e`** carries git's default subject (`Merge remote-tracking branch 'origin/feat/db-fix' into ...`). It is the branch's own merge of the base, not the `Merge R4-09b:` commit the Mac writes into `feat/db-fix`.
