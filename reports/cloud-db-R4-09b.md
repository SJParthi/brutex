STATUS: IN PROGRESS

# R4-09b — the store's backup verb and its restore drill (AUT-06), cloud build

Branch `cloud/db-R4-09b-20261024`, cut from `origin/feat/db-fix` `e851da1c` (Merge DATA-2), with the
newest `origin/feat/db-fix` `1c094c90` (Merge IA-4, after Merge R4-27) merged in (`36c904bd`).
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
  - `docs/04`: BK-01 to BK-25.
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
| `web/src/lib/disk.js`, `web/src/lib/DiskTile.svelte`, `web/tests/disk.test.js` | spec item 3, "the /db band" (the spec's file 4.1 names them) |
| `crates/api/src/disk_route_backup_tests.rs`, `crates/cli/src/backup_tests.rs` | new test files of this part's own modules |

`config/store-steps.toml` is untouched (gate 30 refuses `merged = true` before the `Merge R4-09b:`
subject exists); the flip and the S-17 rewording are for the Mac's merge commit (below).

## Gates (this cloud host: 4 vCPU Intel Xeon 2.10 GHz, Rust 1.97.1, node 22)

GATES-PLACEHOLDER

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

NEEDS-PLACEHOLDER
