# PAUSE — observability lens (L1)

Paused 2026-10-06 on the coordinator's usage-guard message.

- Branch: `attack/observability` (from `origin/final/all-fixes` at 969493e)
- Head: `0016a4664c1ff9a562ca653473a9590667eb2e02` — one commit "WIP (paused, not validated)"
- Not yet done for this commit: fmt, clippy, static gates, Gate 18 mutants, docs rows.

## Done (failing test first, then fix, then green; failing outputs saved locally, re-derive by reverting the fix)

| ID | What | Test | Status |
|---|---|---|---|
| OBSV-01 | `/logs` merged page said `reached_oldest:true` after the union's truncation dropped read records | `api logs::tests::a_union_cut_by_the_limit_has_not_reached_the_oldest` | fixed, green |
| OBSV-02 | `/masters/status.json` folded EACCES/ELOOP/EIO into "absent"; now `present:null`, `unreadable:<reason>`, `restart_required:null`; /mapping page shows it | `api mastersrun::tests::an_unreadable_master_is_named_and_not_reported_absent` | fixed, green |
| OBSV-03 | cash-session cache: reinstall of matching bytes after a failed dir sync returned Ok without syncing | `pull cash_session_cache::tests::a_reinstall_after_a_failed_directory_sync_syncs_again` | fixed, green |
| OBSV-04 | census `append_locked` error stated requested count, not landed count | `pull ingest::tests::a_failed_append_names_how_many_entries_landed` | fixed, green |
| OBSV-05 | live sweep progress refused on `hit_scan_cap` / old torn line from unrelated history; `/logs.json` + `/logs` gain `since=` (named in `ignored` when unreadable); backtest poll passes `since=floor(started_micros/1000)` | `api logs::tests::since_bounds_a_run_poll_to_the_run_and_off_older_damage`, `an_unreadable_since_is_named_and_not_applied`; web `live-progress.test.js` "the live poll is bounded…" | fixed, green (web: 868 pass, svelte-check 0 errors) |
| OBSV-06 | sweep-all refused months (before sweep, or after sweep while filing) emitted no event | `cli batch::tests::a_month_refused_before_sweeping_is_logged_with_its_reason` + extension of `a_walk_whose_swept_month_could_not_be_filed_says_it_swept` | fixed, green |

## In progress

- OBSV-07: `cli::sweep_stored` `Err(why)` arm (crates/cli/src/lib.rs ~3836) emits no event with the reason. Plan: emit `Event::warn("cli.sweep","stored month refused")` with feed, label, reason; test via `sweep_stored("groww","NIFTY","2min",2026,3,100)` (swept_rung refuses first) and `ledger_all::tests::{sink,mark}` + tail on `cli.sweep`.

## Refuted / not fixed (round 1)
- Pull 400/409 before journal (A2), terminal-audit finish discard (A1), census `.man.writing` leftover (A4b), cash-cache parent dir sync, unknown /logs params (C2), operator-route journal rows (EXEMPT by D-0568): refuted.
- Low, open candidates for next round: `/pull/run/stop` leaves no Info event when no recovery plan; `/verify.json` has no page; `/logs` has no field (instrument/feed/reason) filter.

## Next steps
1. Finish OBSV-07.
2. docs: 11-findings rows (IN PROGRESS <branch sha>, per the ledger's main-only sha rule), 05-decisions D-3200..D-3206, 04-invariants OBSV-01..07.
3. Validate: `cargo fmt --check`; `CARGO_BUILD_JOBS=3 cargo clippy --workspace --all-targets -- -D warnings`; touched crates' tests (api, pull, cli) as non-root; static gates from ci.yml language-purity; web `npm test`, `npm run check`.
4. Gate 18 on diff: `git diff origin/final/all-fixes...HEAD -- crates > /tmp/claude-0/mine.diff`; `cargo mutants --in-diff ... -p api|pull|cli` one at a time (cargo-mutants not yet installed: `cargo install cargo-mutants --locked --version 26.2.0`).
5. Merge newest origin/final/all-fixes, push, tracker `observability.tsv`, round 2.

## Restart
```
cd /home/user/brutex && git fetch origin attack/observability && git checkout attack/observability
# non-root test runner: copy test binary to a world-readable dir, run via
# setpriv --reuid=65534 --regid=65534 --clear-groups with TMPDIR/HOME set to a 1777 dir
```
