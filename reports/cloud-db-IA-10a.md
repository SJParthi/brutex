STATUS: IN PROGRESS

# IA-10a (cloud): the api's journal readers name member keys and holes, the day band's run from the event (D-4896), the receipt's census and horizon

Branch `cloud/db-IA-10a-20261024`, cut from `origin/feat/db-fix` at `e851da1ca`; the newest
`origin/feat/db-fix` (`bf9b1c8f`, R4-18) merged in on 25 Oct 2026 (`574d653c`), no conflict.
Scope in force: `reports/db-specs-20261024/IA-10a.md` section 3.2 (the read side only, D-6900).
Nothing of the dropped half is built: no api child mode, no `/gdfl/*` route, no alerts route,
no restart scan, no alert put.

## Prerequisites

All merged on `origin/feat/db-fix` (checked 24 Oct 2026): IA-2, IA-16, IA-3, IA-11r.

## Done

- Item 0: `pull::gdfl_cm::member_share` is `pub` (one line and its doc).
- One accessor beside `hole_is_repairable`: `pull::gdfl_import::hole_reasons()`, the five
  store-hole reasons by pull's own list (outside ownership, named below).
- Item 1: `crates/api/src/import_journal.rs`: a row keeps where a member key's `+` is
  (`member_at`), `DayRow::base`, `DayRow::member`, `DayRow::hole` (`{reason, repairable}`),
  `member_key`, `member_symbol` (share members by `member_share`, index members behind
  `NSE-INDEX-`); a parse-time start index of the `run` lines (`runs_by_start`) with
  `Parsed::started_of`, `holds_run`, `run_running_at` (D-4896).
- Item 2: `crates/api/src/imports.rs`: rows carry `base_filter`, `member`, `member_symbol`,
  `hole`; each logged refusal carries its record's `run`; counts put a hole under `holes` by
  reason and `holes_unrepairable`, never under `counts.incomplete`; `outcome=incomplete` lists
  the plain ones; `Owners::names` reads a member key as naming its member and closing its base's
  kind-day; `placed_of` places a refusal by its own run (D-4896), else by time; `RunsAsked::Every`
  replaced by `RunsAsked::Unasked`.
- Items 3 and 6: `crates/api/src/dayflags.rs`: `Names::names` reads a member key; the band no
  longer decodes every `run` line: a refusal is placed through `imports::placed_of` and the
  parse's indexes; superseded refusals say `run_from`; the record says `placed`; a member close's
  refusal is its member's only when `pull`'s `CmScope::member_of` names its file that member.
- Item 4: `crates/api/src/feeds_route.rs`: an open member key is not a segment open day; the
  reach's `from` counts the open member keys and names the first (D-TBD-IA-10a-1 item 1).
- Item 5: `crates/api/src/provenance.rs`: `filter_holds` reads a member key; `symbol_rows` takes
  a member key's row as its member's day row and drops the base row it reopened, so its bars are
  never credited to the base close's run; day rows carry `base_filter`, `member`, `hole`; the
  month carries `journal_days_holes`.
- Item 7: `crates/cli/src/gdfl_import_command.rs`: the receipt prints the source's census
  (`Census::of`, `render_census`: CM zip ignored/misfiled/invalid, tick store misfiled/stray/walk
  fault, options zips "none") and the calendar horizon (`render_horizon`, from
  `record_horizon(today)`; nothing before 1 January of the last sourced year, D-5156).
- Ledgers: D-4896 headed (`docs/05-decisions.md`) and its forward reference pointed at it;
  `D-TBD-IA-10a-1` (three rules the spec left to the builder); docs/04 rows IA-JR-01, IA-GR-34
  (reworded), IA-RS-01..06; IA1-01 and IA-LG-03 amended where this change made their last clause
  false; docs/06 L-IA-39 rewritten, L-IA10A-1..3.
- 25 Oct: merged the newest `origin/feat/db-fix`; four tests added for the lines the coverage
  run found unreached (below); clippy clean after them.

### Census route (item 7a): option (b), and why

The Mac's spec check names a minimal edit in `gdfl_run.rs` (option (a): the run hands its
source's census back). This branch took the spec's option (b) instead (section 11.2 allows it
"name its cost"): the receipt opens the source once more, read only, after the run
(D-TBD-IA-10a-1 item 3, `docs/06-limits.md` L-IA10A-2: O(entries) once a run, never per day,
never on a page). Reason: section 11.3 lists `gdfl_run*.rs` among the files of other parts
(IA-1, IA-2) not to touch, and `KindEnd`/`Report` are matched in `pull`, `pull`'s run tests and
the cli; carrying a census on them widens a shared type for every caller. The cost of (b) is
stated: a second read, and the census is the source as it stands after the run (a source gone
in between is said, never counted as empty: test
`a_source_gone_after_the_plan_is_said_on_the_receipt_with_the_horizon`). If the Mac prefers (a)
at merge-back, it replaces `Census::of`'s two opens with the run's own; nothing else changes.
No edit to `gdfl_run.rs` or its report was made.

### FM-080 (two owners)

Handled here as far as section 3.2 item 7b says, and no further: `cli gdfl-import`'s receipt
prints `pull::calendar::record_horizon(today)` when it `ends_soon` or `ended` (D-5156: from
1 January of the last sourced year, not 30 days), and says "not checked" when the host clock
names no day; on every receipt, including one whose source did not open. Not handled here:
anything of `cli ingest-run` (R4-11 under D-7135 owns the command; it gets the line by calling
the verb), and the dropped calendar-horizon alert (D-6900). The matrix owner is the Mac's to
reconcile. FM-071: `not_fixed`, dropped by D-6900 (not applicable).

## Left

- Mutation testing on the diff (141 mutants): in progress (see Gates).
- Bench C-LG-02 in release for L-IA-39's after-measurement (placeholder
  `L-IA39-AFTER-PLACEHOLDER` in `docs/06-limits.md`).
- Final report.

## Gates

Run 25 Oct 2026 in this container (4 cores, root, about 25 GB free) on the branch after merging
`origin/feat/db-fix` `bf9b1c8f`.

| Gate | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all --check` | clean |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean (and again `-p api -p cli` after the new tests) |
| deny | `cargo deny check` (cargo-deny 0.19.0, as CI pins) | `advisories ok, bans ok, licenses ok, sources ok` |
| tests | `cargo test --workspace --locked --no-fail-fast` | 11,171 passed, 30 failed, every failure also fails on the unmodified base (below); two test processes hung and were stopped (below) |
| coverage | `cargo llvm-cov --no-fail-fast --locked -p api -p cli --lib` (stable; lines and regions) | every added line of the touched source files covered except one (below) |
| mutants | `cargo mutants --in-diff` | in progress |

**The 30 failures are environmental, the same set on the base.** Re-run on a worktree of the
unmodified `origin/feat/db-fix` `bf9b1c8f` with its own target directory
(`cargo test --locked --no-fail-fast -p api -p cli -p pull -p store -p telemetry --lib -- <the 30 names>`):
the same 30 fail (`diff` of the two sorted FAILED lists: identical). Causes, from their own
messages: this container runs as root, so every "a read-only / unwritable / 000 directory" test
fails (pull `census_store`, `gdfl_dropbox`, `zerodha_verify`, `alerts`; store `quarantine`,
`settings`; telemetry `sink`; cli `scrub_blocks`, `watchdog_command`; api
`feeds_route::the_window_is_refused_loudly_when_degraded_or_inverted`); 4 cores make api's
timed tests fail (`head_deadline_body_tests` x5, `detail::a_refused_admission...`,
`pullrun::restart_tests::the_journals_own_extremes`); and `pull`'s
`gdfl_cm::tests::fixtures_are_built_not_pasted` names `crates/cli/src/tick_second_check_tests.rs`
(DATA-1w's, merged on `feat/db-fix`), not a file of this part.

**Two hangs, both on the base too.** `api::server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`
(its child process idled at 0% CPU for over 20 minutes, here and on the base worktree, where it
idled 5 minutes before being stopped) and `api` `tests/cloud_mutants_bars.rs`
`spot_rs::spot_json_is_refused_by_name_when_every_store_read_slot_is_held` (18 minutes at load
0): each process was stopped so the run could finish; neither file is this part's.

**A disk-space fact for whoever reruns this here.** With the container's allowance below 10 GiB
free, 15 `cli` `gdfl_import_command` tests fail with the importer's own refusal "the disk is
full ... below the free-space floor of 10737418240 bytes (CONC-031)". That is the floor working,
not a defect; with space freed all 77 pass.

**Coverage, on the added lines.** `cargo llvm-cov` here is stable, so `--branch` (nightly only)
was not run: branch coverage is under NEEDS THE MAC. Line coverage of every line this branch
adds to `import_journal.rs`, `imports.rs`, `dayflags.rs`, `feeds_route.rs`, `provenance.rs`,
`gdfl_import_command.rs`, `gdfl_cm.rs` and `gdfl_import.rs` (from `git diff -U0` against
`origin/feat/db-fix`, joined with the lcov `DA` records): all covered but
`import_journal.rs:582`, the `Err` arm of `u32::try_from(self.runs.len())` (a journal holding
more than `u32::MAX` `run` lines), which is IA-11r's line moved into a new block, not a new
branch. The first run found ten lines unreached; four tests now reach them:

- `api::server::imports::member_tests::a_share_members_hole_is_its_shares_and_an_options_member_key_is_shown_not_read`
  (a share member and its `.BE` series on the share's day band; the out-of-the-box options close
  under a member key, shown and never read as a member tree; a refusal naming no swept symbol on
  a member-only day);
- `api::server::imports::member_tests::a_member_only_day_owns_its_own_members_file_and_no_other`;
- `cli::gdfl_import_command::tests::a_source_gone_after_the_plan_is_said_on_the_receipt_with_the_horizon`
  (the census of an archive removed after the plan; the horizon on an unopened receipt).

A cargo fact for this container: `cli`'s build script watches the repository root as a
directory, which cargo scans recursively, `target/` included, so with the default target
directory every cargo command rebuilds `cli` and `api` (about 400 s). Use a
`CARGO_TARGET_DIR` outside the checkout, as the Mac's pools do.

## Extreme cases (tests)

Each new test names `pull`'s or the journal's own constants; no fixture is a real vendor row.
Every one fails before this change: on the base none of `member`, `base_filter`, `hole`,
`holes`, `holes_unrepairable`, `refusals.placed`, `Census` or `render_horizon` exists, so each
test either does not build there or asserts a field the base never writes.

1. Empty and absent journal: zero holes by name (`an_empty_or_absent_journal_says_zero_holes_by_name`).
2. One record: a day whose only key is a member key, journalled, no made-up `*` row (IA-JR-01).
3. Maximum: 10,000 `run` lines, the band decodes none (`the_day_band_decodes_no_run_line_whatever_the_journal_holds`).
4. Year boundary: member holes across 31 Dec / 1 Jan count in their own range (`member_holes_across_a_year_end_count_in_their_own_range`).
5. Torn and duplicate records: a hole closed again is no longer a hole, a torn one never was (`a_hole_closed_again_is_no_longer_a_hole_and_a_torn_one_never_was`).
6. Malformed keys: a `+` with nothing on one side names no member (`a_plus_with_nothing_on_one_side_names_no_member`).
7. Renamed symbols / series: `.BE` and `.BZ` members map to their share, `.SM` does not (`a_member_names_its_symbol_by_pulls_one_rule`).
8. Out of order / clock: a refusal placed by its own run, by time only without one (`a_refusal_is_placed_by_its_own_run_and_by_time_only_without_one`, `a_day_bands_refusal_names_its_run_from_the_event_not_the_clock`).
9. Clock edges: the horizon on the day before the last sourced year, its first day, the last day, the day after, 400 days after, and a clock naming no day (`the_calendar_horizon_is_warned_on_the_receipt_before_it_passes`).
10. Crash between plan and run: the source gone after the plan (`a_source_gone_after_the_plan_is_said_on_the_receipt_with_the_horizon`).
11. Combination: a member key, a hole and two members of one run in one window, attributed by `pull`'s member of the file (IA-JR-01).
12. Combination: a share member, a `.BE` series and an unrepairable hole at once (`a_share_members_hole_is_its_shares_and_an_options_member_key_is_shown_not_read`).
13. Combination: a member-only day, a swept symbol and a log-only row (`a_member_only_day_owns_its_own_members_file_and_no_other`).
14. Out of the box: an `options` close under a member key, which no writer makes, is shown under its own key and never read as a member tree (test 12's second half); an open member key is not the segment's open day (`an_open_member_key_is_named_and_is_not_the_segments_open_day`).
15. Every hole reason is a hole, by `pull`'s list, and nothing else is (`every_hole_reason_is_a_hole_and_nothing_else_is`).
16. The census at its bounds: 17 misfiled, 16 named and 1 counted as more; a stopped walk; a source with none (`a_source_census_is_named_on_the_receipt`).

## Measurements (O(1) at p99)

UNMEASURED here so far: bench C-LG-02's day-band rows (L-IA-39 after D-4896) are still to be run
in release. The HTTP timings of section 13.4 are the Mac's.

## Files outside ownership

- `crates/pull/src/gdfl_import.rs`: one `pub const fn hole_reasons()` beside
  `hole_is_repairable` (spec section 11.2 names this route).
- `crates/api/benches/ratio/logs_dirs.rs`: the module doc and the day-band rows beside C-LG-02
  say the band is flat since D-4896 (bench text only; the gate it feeds is unchanged).
- `crates/api/src/dayflags_rules_tests.rs`: two lines, a test helper's call after
  `RunsAsked::Every` was removed.
- `crates/api/src/provenance_tests.rs`: `a_filter_takes_a_symbol_by_its_exact_name` widened to
  member keys.
- Not edited: `gdfl_run.rs` and its report (census route (b) above), `audit_json.rs` (read
  through `imports::summary`, unchanged), `.github/**`.
- Gate 11 says three of its allowlist entries are now loose (dayflags rule 4 and rule 6,
  imports rule 6: constructs this change removed); `.github/` is not this part's, so they are left
  for the allowlist's owner (see NEEDS THE MAC).

## D-numbers

D-4896 (this part's, headed once). `D-TBD-IA-10a-1` (three rules: an open member key, a member
close's refusals, the receipt's census read), for the Mac to number.

## NEEDS THE MAC

Every command below is the spec's section 13, run as written there; this list adds only what
this container could not do. `W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`, worktree
`$W/wt/P-IA-10a` on `fix/db-IA-10a` holding this branch. Never `:8080`, never the real store.

1. **Gates on the Mac (13.1).** `cargo fmt --all --check`;
   `cargo clippy -p pull -p api -p cli --all-targets -- -D warnings`;
   `cargo test -p pull -p api -p cli --locked`; `cargo build --release -p api -p cli`; then
   `stop.sh`/`shadow.sh`/`serve.sh IA-10a 8405`. Expected: every gate green (the 30 failures and
   two hangs listed under Gates are this container's root user and 4 cores; on the Mac, as a
   normal user, they pass as they do on `feat/db-fix`); `OURS`.
2. **Branch coverage.** `cargo +nightly llvm-cov --branch --no-fail-fast --locked -p api -p cli -p pull --lib`
   over the eight touched source files. Expected: every line and branch this diff adds covered,
   except `import_journal.rs`'s `u32::try_from(self.runs.len())` `Err` arm (IA-11r's line, a
   journal of more than `u32::MAX` `run` lines), named here rather than excused.
3. **Mutants not finished here** (see Gates for what was run and its result): the remainder by
   `cargo mutants --in-diff <(git diff origin/feat/db-fix -- crates/api/src crates/pull/src crates/cli/src/gdfl_import_command.rs ':!*_tests.rs') -p api -p pull -p cli`.
   Expected: 0 missed.
4. **Real journal, unchanged answers (13.2).** Re-count the real journal first
   (`awk '$1!="run" && $4 ~ /\+/' <journal> | wc -l` expected 0;
   `grep '^incomplete' <journal> | grep -o 'reason=[a-z_]*' | sort | uniq -c` expected only
   `14 reason=filing` among hole-or-filing reasons, none of the five hole reasons). Then the eight
   `curl | jq -S` answers of 13.2 from the baseline build and from this build on 8405, `diff`ed
   after deleting only `base_filter`, `member`, `member_symbol`, `hole`, `holes`,
   `holes_unrepairable`, `journal_days_holes`, the day band record's `placed`, `run_from`. Expected:
   identical; `holes` empty or all 0, `holes_unrepairable` 0, no `hole` object on any row;
   `/feeds.json`'s open days unchanged; one F&O stock month and one illiquid strike's
   provenance month the same.
5. **Member keys and holes on the shadow's cloned journal (13.3).** Append the four synthetic
   lines of 13.3 (and the share-member pair on `*+NSE-CASH-<sym>` and `*+NSE-CASH-<sym>.BE`) to
   `$W/shadow/IA-10a/imports/gdfl.journal`, restart. Expected exactly as 13.3 lists: the two
   member rows with `base_filter` `*` and their members; `hole` `{needs_month_generation, true}`
   and `{bars_differ_held, false}`; holes by reason 1 + 1, `holes_unrepairable` 1; the `*` row
   `done`; no invented `logged_only` row; NIFTYIT's day band and provenance month show `D` as a
   hole, NIFTY's unchanged; `/audit.json` agrees with `/imports.json`. Then a lone
   `begin indices D2 *+NSE-INDEX-NIFTYIT`: expected `/feeds.json` does NOT list `D2` as an open
   segment day; the reach's `from` counts 1 open member key and names it (D-TBD-IA-10a-1 item 1).
   `/db` in the browser pane only if no other agent uses it: NIFTYIT's band shows `D`'s hole.
6. **Timings (13.4).** n = 200 sequential GETs each (`curl -s -o /dev/null -w '%{time_total}'`)
   of `/imports.json?feed=gdfl`, `/dayflags.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2019-10&to=2019-10`
   and `/provenance.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2019-10`, on the
   real journal and after appending 100,000 synthetic `run` lines to the shadow's journal; p50,
   p99, max. Expected: `/dayflags.json` of the same order at both sizes (11 Oct before D-4896:
   p50 1.74 ms against 103.85 ms). Write the numbers into L-IA-39's after-row in
   `docs/06-limits.md` beside this container's bench figures.
7. **The receipt (13.5)** on the private root `$W/shadow/IA-10a-rx` exactly as 13.5 (the `bars`
   link moved aside, never deleted; the CM archive path from the local configuration, never
   written down). Expected: the census lines (ignored, misfiled, invalid, each at most 16 names
   and "and N more"); on any day of 2026 the horizon line (`last_day` 2026-12-31, `ends_soon`
   true, `owed_year` 2027; `days_left` is 68 on 24 Oct, so 31 Dec minus the run's day); the real
   store's journal size and mtime unchanged. And `indices tickstore` if configured.
8. **Gate 11's three loose allowlist entries** (dayflags rule 4 and rule 6, imports rule 6):
   warnings, not failures; for the allowlist's owner to drop at merge-back if wanted.
9. **Store writes:** none (13.6).

## Log

### 2026-10-24 09:34 IST

- Waited for the spec and GO (about 50 minutes of polling), then read README, CONTEXT, IA-10a.
- Edited the five files above; `cargo check -p api --tests --locked` clean.
- HOLD found; committed, pushed, waiting.

### 2026-10-24 13:20 IST

- The hold lifted at 12:37 IST; items 4, 5 and 7 built (`4bba4010`); tests written (`ab8bc06d`).
- A new HOLD (12:56 IST) named no GO-IA-10a: committed, pushed, held.

### 2026-10-24 18:35 IST

- Resumed at 17:34 IST; merged `origin/feat/db-fix` `7b397bbe` (`bbd315fe`); the ledgers;
  my own defect fixed (a member close's refusals by `pull`'s member of the file); CI shell gates
  0 to 29b on this tree and the base: same exits; HOLD again at 18:16 IST.

### 2026-10-25 (this session; another account's session before it is unreachable)

- GO-IA-10a present. Found the branch at `4f6251aa` (three commits after the 18:35 report:
  a merge of `feat/db-fix` `666ddf7f`, clippy, the bench doc). Merged `bf9b1c8f` (`574d653c`).
- fmt, clippy, deny green; workspace tests run and the 30 failures shown on the base; coverage
  run; four tests added for the unreached lines (`283810c0`, `885a00d9`, `191ce354`).
- NEXT: mutants on the diff (api and pull files against `api`'s tests, then the cli file),
  bench C-LG-02 in release, NEEDS THE MAC, STATUS.

### 2026-10-26 19:32 IST (a new account's session; the 25 Oct session is unreachable)

- GO-IA-10a present. Found the branch at `5ca8a885` (the 25 Oct report plus a merge of
  `feat/db-fix` `4acb43f7`, CLOUD-MUTAPIB, `72330455`, and its NEEDS THE MAC commit).
- Merged the newest `origin/feat/db-fix` `5901ec77` (IA-10c, R4-22, BASE-FIXTURE-1025):
  `386d3e52`, no conflict; of the owned and outside-ownership files only `gdfl_cm.rs` and
  `gdfl_import.rs` changed underneath (R4-22), and `member_share` stays `pub`,
  `hole_reasons()` stays beside `hole_is_repairable`.
- Tools reinstalled in this container (cargo-mutants 27.1.0, cargo-llvm-cov 0.9.1,
  cargo-deny 0.19.0). `cargo mutants --list --in-diff` gives 124 mutants on the api and pull
  files and 17 on the cli file (141, the 25 Oct count).
- NEXT: the api/pull mutants (running), then the cli file's, then bench C-LG-02 in release,
  then the final gates, NEEDS THE MAC, STATUS.

### 2026-10-26 20:35 IST

- api/pull mutants, first pass (stopped by this container's 30-minute background limit after
  40 outcomes; resumed with `--iterate`): 37 caught, 2 unviable, 1 MISSED:
  `crates/api/src/dayflags.rs:1501:37: replace < with <= in refusals_of` (no test had more
  superseded refusals than `MAX_REFUSALS_NAMED`). New test
  `server::dayflags::tests::superseded_refusals_past_the_named_bound_are_counted_not_named`
  (one past the bound, alternately placed by the event's run and by time; named in log order,
  each `run_from`, every one counted): `cargo test -p api --lib --locked -- superseded_refusals_past_the_named_bound`
  `test result: ok. 2 passed` (with its neighbour) (`ba036668`).
- Bench `ratio` built in release (`cargo bench -p api --bench ratio --locked --no-run`, exit 0);
  it runs once the mutants stop loading the machine (its rows gate on ratios).
- NEXT: the remaining 85 api/pull mutants (running, `--iterate`), the cli file's 17, bench
  C-LG-02, the final gates, NEEDS THE MAC, STATUS.
