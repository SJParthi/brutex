STATUS: IN PROGRESS

# R4-17 — Zerodha series identity: listing dates, landing refusals (day rung too), history ends at delisting, store scan, token guard

Branch `cloud/db-R4-17-20261017`, cut from `origin/feat/db-fix` 3911d1b9, with
`origin/feat/db-fix` 62396df0 (AF-07) merged in. Prerequisites: none (R4-17.md),
confirmed against `git log origin/feat/db-fix --merges --format=%s`.

## Done

| Item | What landed | Where |
|---|---|---|
| Z5 (listing rule) | `pull::listing`: per (NSE, symbol, ISIN) listing day and end of history from the NEWEST receipted `NSE_CM_security_*.csv.gz` under `<store>/session-masters`, read through `cash_session_cache::read_local_lifecycle` (no second decoder). A newer master that does not check is passed over BY NAME. `0` (TATACONSUM, TMPV), blank, absent, invalid, after-master, conflicting and contradicted listings are UNKNOWN and refuse nothing. A name in no master is labelled `inferred_from_vendor_span` and refuses nothing. O(1) `span`; `for_store` loads once per store root and remembers only a loaded master. | `crates/pull/src/listing.rs`, `listing_tests.rs` |
| Z5 (source) | NSE Masters Data v1.6 fetched and read (HTTP 200, 817,499 bytes, 39 pages, sha256 `a1e45b6de312ebf05298fac8a1c4fbe337b8339b7eb110ac75705925bd05487e`): §2.3 p7 the 1980 epoch, §3.1.1 p9 Listing/Expulsion/Re-Admission Date LONG. The listing refusal is ON. What the PDF does NOT say (the MII CSV's abbreviated column names) is labelled. | `docs/00-charter.md` (new section) |
| ING-01 c, d (day-rung arm) | A non-intraday window is screened ONLY for a price at or below zero (index, share, future; never an option) and a CASH day before its listing. It is never judged by range. | `identity_screen::day_arm`; `series_identity::judge_day` |
| ING-01 c, d; RT-DATA-02 a (minute arm) | Per minute, in order: non-positive price; before listing; the WP-13d day-rung verdicts, where a master's listing replaces `before_first_day_bar` (DALBHARAT's minutes are its own); then the in-session jump. One `MinuteJudge` is shared by the landing and the scan. | `identity_screen::minute_arm`; `series_identity::{MinuteJudge, JumpRun, jump_bps, JUMP_BPS}` |
| RowCause | `NonPositivePrice` `non_positive_price`, `BeforeListing` `before_listing`, `InSessionJump` `in_session_jump`. No code renamed. | `crates/pull/src/fetch.rs` |
| COR-07 (history ends) | `fetch::fetch_and_land_listed`: a 404 or empty answer for a CASH window starting strictly after the master's removal day (no readmission) is `Landed::history_ends = Some(HistoryEnds{delisted_on})` (`history_ends_delisted`), with one info `pull.history_ends` event. Not a failure. A still-listed 404, a window including the removal day, other refusals, indices and arriving rows are as before. | `crates/pull/src/fetch.rs` |
| RT-DATA-02 b, Z6 (store scan) | `cli series-identity` judges every stored bar by the landing's own rules and counts in-session jumps per month. It exits non-zero on any refusal or jump. Each flagged bar gets a measured cause (`zero_price`, `before_listing`, `before_first_day_bar`, `basis_mismatch` label-only against GDFL 1s folded by `pull::fold` within 100 bps, `foreign_level`, `no_gdfl_witness`). It prints the per-share jump distribution (p50/p99/p999/max), which IS the `JUMP_BPS` measurement, and writes ONE content-addressed quarantine PLAN under `<store>/audit/series-identity/` (key, rung, month, whole_month or every stamp, cause, action) via `store::staging`. It writes no bar. | `crates/cli/src/series_identity.rs`, `series_identity_tests.rs`; `identity_screen::scan_series` |
| CON-13 | Source-shape test over the six named modules: no key body, key-builder parameter or `EntryKey`/`PathParts` literal names a token. The module list is closed (no other non-test file of core/store/pull/api defines those names), and a self-test fixture with `instrument_token` must be refused. | `crates/pull/tests/token_identity.rs` |
| Item 6 | The `state/fix/...` path is gone from `series_identity.rs`'s `FOREIGN_BAND_BPS` comment. | `crates/pull/src/series_identity.rs` |
| Bench | Gate 8 rows C-ZI-01 (listing probe, 300 → 3,000 rows) and C-ZI-02 (identity judge per minute, 1x/10x/100x). | `crates/pull/benches/ratio/identity.rs` |
| Docs | D-4990 (listing rule), D-4991 (history ends), D-4992 (day-rung arm, amends WP-13d), D-4993 (in-session jump; JUMP_BPS PROVISIONAL), D-4994 (store scan, measured causes, plan), D-4995 (token guard); docs/04 ZI-01..ZI-09; docs/06 R4-17 section with the C-ZI numbers and every limit. | `docs/05-decisions.md`, `docs/04-invariants.md`, `docs/06-limits.md` |

### Failing-first tests (each named in the work package exists)

Proved by disabling each arm and re-running (`crates/pull/tests/zerodha_identity.rs`): with the
day arm, the jump rule, the listing and history-ends switched off,
`a_session_interleaving_two_price_bases_is_refused_at_landing`,
`a_zero_price_day_bar_is_refused_by_name_at_landing`, `a_day_bar_before_listing_is_refused_by_name`,
`a_minute_before_listing_is_refused_by_name` and `a_404_after_delisting_is_history_ending_not_a_failure`
FAIL; restored, they pass. `the_day_rung_is_never_judged_by_range`, `epoch_zero_listing_refuses_nothing`,
`a_name_absent_from_every_master_is_labelled_inferred` and
`a_halt_splits_the_run_and_a_sessions_first_minute_is_never_a_jump` guard against OVER-refusal, so they pass
either way by design. `the_store_scan_counts_in_session_jumps_and_fails_on_a_seeded_copy` and
`piind_minutes_are_named_by_cause_before_any_write` assert output the old verb never printed.
`no_store_or_census_key_is_built_from_a_token` and `a_token_field_in_a_fixture_key_builder_is_refused` are
in `token_identity.rs`.

### Files outside the owned list (shared lines; say so at merge-back)

- `crates/pull/src/lib.rs`: +3 lines, `pub mod listing;` with its doc line.
- `crates/pull/benches/ratio.rs`: +6 lines, the `#[path = "ratio/identity.rs"] mod identity;` line and one `ok &=` call in `main`.
- `docs/00-charter.md`, `docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md`: appended sections only.
- Decision numbers D-4990..D-4995 are PROVISIONAL: the head of the ledger was D-4978 on 62396df0; renumber at landing if another part took them.
- `crates/pull/src/ingest.rs` is NOT edited (its call at :1249 and `refused_row_failures` count the new causes as they are).

## Left

- Coverage (nightly `--branch`) and mutants on the touched modules: running now.

## Gate results (this container: 4 vCPU, root, ext4)

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace --locked --no-fail-fast -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store`
  (the known 95-minute cli pool test skipped, per CONTEXT.md): every R4-17 test passes. 25 tests fail in
  crates and modules R4-17 does not touch, each for an environmental reason:
  - 23 are permission tests that `chmod 000` or otherwise make a path unwritable, and root ignores that:
    api `logs::`, `pullrun::`, `server::` (12); pull `gdfl_import` (2) and `gdfl_tickstore` (1); cli
    `scrub_blocks` (1); store `settings` (3); telemetry `sink` (4). **Re-run as user `nobody`
    (`setpriv --reuid=65534`) every one of them passes.**
  - store `staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported`: ext4
    hands the unlinked inode straight back (1804449 == 1804449); it fails as `nobody` too. `crates/store`
    is unchanged by R4-17.
  - store `cited_commits`: commits cited by feat/db-fix's own `docs/06-limits.md` are not ancestors of
    `origin/main` (feat/db-fix is not merged into main). R4-17's docs cite no commit.
- `cargo deny check`: bans ok, licenses ok, sources ok; **advisories FAILED** on five advisories published
  in the live database: RUSTSEC-2026-0344 (`ruzstd` 0.8.2) and RUSTSEC-2026-0346..0349 (`rustls`).
  `Cargo.lock`, `deny.toml` and every `Cargo.toml` are byte-identical to `origin/feat/db-fix`, so the base
  fails the same way. Not fixed here: the lockfile is shared, and bumping it belongs to its own part.
- `cargo bench -p pull --bench ratio` (gate 8): all ratios within the ceiling, including C-ZI-01 1.013x and
  C-ZI-02 1.176x (10x) / 1.485x (100x). Measured in this container, not on the operator's machine.

## NEEDS THE MAC

Every command is read-only of bars. `<store>` is the operator's real store, and `cli` is the build of this branch
after merge-back. `BRUTEX_STORE` is how `cli` finds the store (`crates/cli/src/lib.rs` `store_root`).

1. **The JUMP_BPS measurement and the CASH quarantine plan** (D-4993, D-4994). It writes only
   `<store>/audit/series-identity/zerodha-CASH-ALL-<digest>.plan`:
   `BRUTEX_STORE=<store> cli series-identity ALL --segment CASH --feed zerodha`
   Record each share's `jump-p99`, `jump-p999` and `jump-max` in `docs/06-limits.md` beside `JUMP_BPS`.
   **Stop the merge and say so if any of these fails:**
   - a clean share's `jump-max` reaches 3,000;
   - PIIND shows no `in_session_jump` in 2018-12..2020-01;
   - any `in_session_jump` day of a share falls on a split or bonus ex-date in `crosscheck_all.json`
     (per share: `BRUTEX_STORE=<store> cli series-identity <SYMBOL> --segment CASH --feed zerodha` lists
     each `(day, in_session_jump)`).
2. **The INDEX scan and its plan**: `BRUTEX_STORE=<store> cli series-identity ALL --segment INDEX --feed zerodha`
   (expect WP-13d's NIFTYNEXT50 and NIFTYPVTBANK `foreign_level`, now with measured causes).
3. **Each named share's measured cause**, to be appended to D-4994 from the output (not before):
   `for s in PIIND VBL PATANJALI UNITDSPR IRFC KFINTECH DALBHARAT MAZDOCK; do BRUTEX_STORE=<store> cli series-identity $s --segment CASH --feed zerodha; done`
   The verification line of R4-17.md expects a non-zero exit naming the PIIND, IRFC, KFINTECH and MAZDOCK
   causes (IRFC and KFINTECH `before_listing` once their master rows are on disk; MAZDOCK `zero_price`).
4. **Gate 8 on the operator's machine**: `cargo bench -p pull --bench ratio`. Record the C-ZI-01 and
   C-ZI-02 lines beside the container's numbers in `docs/06-limits.md`.
5. **The 25 environment-only test failures** above, on the Mac (not root, APFS):
   `cargo test --workspace --locked --no-fail-fast -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store`,
   plus the skipped pool test in the final Gate run.
6. **`cargo deny check`**: the five advisories are on the base too. The Mac's gate run decides whether a
   separate part bumps `ruzstd` (>= 0.9.1) and `rustls`.
7. **A shadow landing (never :8080; a fresh shadow store)**, with a receipted master under its
   `session-masters`: a Zerodha 1day pull of MAZDOCK over the months its zero day bars are in shows
   `non_positive_price` failures by name and lands the rest. A 1min pull of IRFC before its `ListgDt` shows
   `before_listing`. Both need the Kite sign-in, so they run on the Mac.
8. **R4-14** must call `pull::fetch::fetch_and_land_listed` from `broker_run` with the share's
   `pull::listing::for_store(store).span("NSE", symbol, isin)` for history ends to reach the live pull
   (docs/06 names it).
