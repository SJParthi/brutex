STATUS: HELD

# IA-10a (cloud): the api's journal readers name member keys and holes, the day band's run from the event (D-4896), the receipt's census and horizon

Branch `cloud/db-IA-10a-20261024`, cut from `origin/feat/db-fix` at `e851da1ca`.
Scope in force: `reports/db-specs-20261024/IA-10a.md` section 3.2 (the read side only, D-6900).
Nothing of the dropped half is built: no api child mode, no `/gdfl/*` route, no alerts route,
no restart scan, no alert put.

## Prerequisites

All merged on `origin/feat/db-fix` (checked 24 Oct 2026): IA-2, IA-16, IA-3, IA-11r.

## Done so far (WIP, not gated)

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
  parse's indexes; superseded refusals say `run_from`; the record says `placed`.
- `cargo check -p api --tests --locked`: clean.

- Item 4: `crates/api/src/feeds_route.rs`: an open member key is not a segment open day; the
  reach's `from` counts the open member keys and names the first (D-TBD-IA-10a-1).
- Item 5: `crates/api/src/provenance.rs`: `filter_holds` reads a member key; `symbol_rows` takes
  a member key's row as its member's day row and drops the base row it reopened, so its bars are
  never credited to the base close's run; day rows carry `base_filter`, `member`, `hole`; the
  month carries `journal_days_holes`.
- Item 7: `crates/cli/src/gdfl_import_command.rs`: the receipt prints the source's census
  (`Census::of`, `render_census`: CM zip ignored/misfiled/invalid, tick store misfiled/stray/walk
  fault, options zips "none") and the calendar horizon (`render_horizon`, from
  `record_horizon(today)`; nothing before 1 January of the last sourced year).
- Tests (all pass here): api `imports_member_tests.rs` (11, one timing ignored):
  `the_api_journal_reader_names_member_keys_and_holes` (IA-JR-01),
  `a_day_bands_refusal_names_its_run_from_the_event_not_the_clock`,
  `a_refusal_is_placed_by_its_own_run_and_by_time_only_without_one`,
  `every_hole_reason_is_a_hole_and_nothing_else_is`,
  `a_hole_closed_again_is_no_longer_a_hole_and_a_torn_one_never_was`,
  `a_plus_with_nothing_on_one_side_names_no_member`,
  `a_member_names_its_symbol_by_pulls_one_rule`,
  `member_holes_across_a_year_end_count_in_their_own_range`,
  `an_empty_or_absent_journal_says_zero_holes_by_name`,
  `an_open_member_key_is_named_and_is_not_the_segments_open_day`,
  `the_day_band_decodes_no_run_line_whatever_the_journal_holds`; provenance
  `a_filter_takes_a_symbol_by_its_exact_name` (widened); cli
  `a_source_census_is_named_on_the_receipt`,
  `the_calendar_horizon_is_warned_on_the_receipt_before_it_passes`.
- `cargo test -p api --lib -- import_journal imports dayflags provenance feeds_route`: 164
  passed, 1 failed: `feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted`,
  whose own message says why: "this host reads a 000 directory (root?)": this container runs
  as root. Not this change's; to be shown on the unmodified base before DONE.
- `cargo test -p cli --lib -- gdfl_import_command`: 62 passed.

## Left

docs/04 rows, docs/05 (D-4896 headed, the forward reference pointed at it), docs/06 (L-IA-39
rewritten, L-IA10A rows), the before-change runs of the new tests on the base, the full gates
(fmt, clippy, workspace tests, deny, CI shell gates 10/11/12/27/27b, coverage, mutants),
the timing run, the NEEDS THE MAC list.

## Gates

Not run yet (WIP).

## Files outside ownership

- `crates/pull/src/gdfl_import.rs`: one `pub const fn hole_reasons()` beside
  `hole_is_repairable` (spec section 11.2 names this route).

## D-numbers

D-4896 (this part's). Others, if needed: `D-TBD-IA-10a-1`, ...

## NEEDS THE MAC

To be written at the end (spec section 13).

## Log

### 2026-10-24 09:34 IST

- Waited for the spec and GO (about 50 minutes of polling), then read README, CONTEXT, IA-10a.
- Edited the five files above; `cargo check -p api --tests --locked` clean.
- HOLD found on `origin/cloud/db-base-20261024` at this save point: committed, pushed, waiting.
- NEXT: when GO returns and HOLD is gone, item 4 (`feeds_route.rs` open-day scan), then item 5
  (`provenance.rs`), then item 7, then tests.

### 2026-10-24 13:20 IST

- The hold lifted at 12:37 IST (`55dfb044`); resumed. Items 4, 5 and 7 built (commit
  `4bba4010`); the tests above written and passing (commit `ab8bc06d`).
- A new HOLD (12:56 IST, `a8dcfea6`) names no GO-IA-10a: committed, pushed, held.
- NEXT: when HOLD is gone (or GO-IA-10a appears): the docs/04, docs/05 and docs/06 entries,
  then the base runs of the new tests, then every gate once, then the report as DONE.

### 2026-10-24 18:35 IST

- The 12:56 hold lifted at 17:34 IST (`f046635b`); resumed. Merged the newest `origin/feat/db-fix`
  (`7b397bbe`: IA-4, CLOUD-MUTSTORE, CLOUD-MUTCLI, R4-27) into the branch, no conflict
  (`bbd315fe`).
- Ledgers: D-4896 headed and its forward reference pointed at it; `D-TBD-IA-10a-1` (three rules
  the spec left to the builder); docs/04 rows IA-JR-01, IA-GR-34 (reworded), IA-RS-01..06 (gate
  12 reads a row id only as `XX-YY-NN`, so not `IA10A-nn`); IA1-01 and IA-LG-03 amended where
  this change made their last clause false; docs/06 L-IA-39 rewritten (its after-measurement
  still a placeholder for bench C-LG-02's rows), L-IA10A-1..3.
- Found and fixed my own defect: a member close counted every refusal of its window as its
  member's, but one run that reopens two members logs both members' refusals in one window.
  Now a refusal is the member's only when `pull`'s `CmScope::member_of` names its file that
  member; IA-JR-01 holds that case.
- CI shell gates of the language-purity job (gates 0 to 29b, all but 1e) run through the
  repository's own step scripts on this tree and on the unmodified base worktree: the same exit
  for every gate; red on both: 1c, 1d, 10, 11, 12, 19, 23, 27b, 30 (BASE-REDS-1023's). No new
  refusal of mine in any of them after the fixes (gate 12: the six cost-claiming doc blocks
  now cite rows; gate 23: the printed timing test removed, bench C-LG-02 measures it). Gate 11
  says three of its allowlist entries are now loose (dayflags rule 4 and rule 6, imports rule 6:
  constructs this change removed); `.github/` is not this part's, so they are left for the
  allowlist's owner.
- `cargo fmt --all --check`: clean after `cargo fmt --all` (my files only). Commit `8170414c`.
- HOLD again (18:16 IST, no GO-IA-10a): committed, pushed, held.
- NEXT: when HOLD is gone: `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked`, the before-change run of the new tests in the base
  worktree, `cargo deny check` and the coverage and mutation tools (to install, pinned as CI
  pins them), bench C-LG-02 in release for L-IA-39, then the final report.
