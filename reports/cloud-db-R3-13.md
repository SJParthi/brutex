STATUS: IN PROGRESS

# R3-13 — /db day band and row labels (cloud build report)

Branch `cloud/db-R3-13-20261017`, from `origin/feat/db-fix` 3911d1b9c.

## Prerequisites

Both merged into `origin/feat/db-fix`:

- `Merge WP-11c: import data-quality labels: minute-sampled days and halts` (c4f6e522)
- `Merge WP-62: pull runtime: calendar horizon, idle stop, per-instrument gates, stop/restart/storage safety` (72c91a46)

## Done

- Turn 1: read `CLAUDE.md`, the cloud README, `CONTEXT.md` and `R3-13.md`; mapped every
  finding to the code that owns it (below). A cold build of `api`'s tests finished, exit 0.
- Turn 2: merged the newer `origin/feat/db-fix` (62396df0) into the branch, because AF-02 and
  AF-07 changed `dayflags.rs` (+395/-40). **SC-CLOSE-11 is already fixed on the base by AF-02**
  ("multi-name filters read right on /dayflags.json": `Names` in `journal_rows`, tests in
  `crates/api/src/imports_filter_tests.rs`); R3-13 adds nothing for it.
- Turn 2: wrote `crates/api/src/dayflags_rules.rs` (not yet declared or wired, so the build
  is unchanged): `feed_rungs` (goal 5b, the one function for peers and finest rungs),
  `asked_rung` (goal 1's `timeframe=`), `CasFlags` (SC-CLOSE-8: the share's dated master flag,
  read as `bars_change.rs` reads it, failed loads never kept), `expiry_of` (SC-CLOSE-12) and
  `held_month_before` (goal 2a: 144 `stat`s back, one open).
- Constraint found: `crates/api/src/sources.rs` (not owned) builds `dayflags::Asked` by struct
  literal, so the grid's timeframe is passed beside `Asked`, never as a new field of it.

## Finding → code map (the plan)

| Finding / goal | Where | Fix |
|---|---|---|
| SC-CLOSE-11 | `dayflags.rs` `journal_rows` | DONE ON THE BASE by AF-02 (`Names`); nothing added here |
| SC-CLOSE-8 | `dayflags.rs` `calendar_json` (`CasEligibility::Unknown`) | the share's dated master flag, read as `bars_change.rs` reads it, in the new `dayflags_rules.rs` |
| SC-CLOSE-9 | `web/src/lib/db/dayflags.js` `kindsOf`; `DbDayFlags.svelte` auction line | `cas` only where the served flag is 1; no "kept apart" claim while no auction sidecar is served |
| SC-CLOSE-10 | `web/src/lib/db/notes-labels.js` auction-print label | no ordinary trade labelled the auction; no zero-book claim where the print has a book |
| SC-CLOSE-12 | `dayflags.rs` record; `dayflags.js` | days after an option's expiry are `expired`, owing nothing |
| Goal 1 (band at the grid's timeframe) | `dayflags.rs` `asked`/`PageHeld`; `DbDayFlags.svelte` `bandInstrument` | `timeframe=` counted at that rung; Zerodha 1day: session with minutes and no day bar is missing |
| Goal 2a, DB-031 (scale switch, far months) | `dayflags.rs` `bar_before` (12 opens back) | metadata probe back to the series' previous held month, one open |
| Goal 2b, DB-031 (zero-volume peer, other names) | `dayflags.rs` peer count | the peer looked up under the share's other names |
| Goal 2c, DB-B20 (frozen index, outside day range) | new `dayflags_rules.rs` | one pass per index day; flagged on both feeds |
| Goal 2d, DB-029 (late rows on a recounted day) | `dayflags.rs` `import_json` | late and max_back_s kept from the closing line before the recount; "the day's total over all files" |
| Goal 3 (bar definition 2/3/4) | quality line `definition`; band; `DbProvenance.svelte` | named on the band and in the drawer |
| Goal 4, DB-G38, DB-G40 (adjusted / quantised Zerodha share runs) | `notes-labels.js`, `DbBasisBadge.svelte` | named on the rows' label with the run's first and last day, from the served factor |
| Goal 5a (unknown quality field) | `dayflags.rs` `parse_quality` drops unknown fields | served and drawn under its own name |
| Goal 5b (peers and finest rung) | `dayflags.rs` `peer_of`/`finest` | one function in `dayflags_rules.rs` |
| DB-053 | band | holes and refused spans distinct from "not yet listed" (follows from goal 1) |
| DB-A15 | `crates/pull/src/calendar.rs` comment — NOT an owned file | the band's evidence independent of Zerodha: GDFL traded on a day the calendar calls closed. The stale comment is left for calendar.rs's owner |
| CMP-06 | `dayflags.rs` `sampled_series` | a GDFL share day names the market index's sampling and its own first held second |

## Left

Everything in the table above, the six named tests, the web tests, the bench, the
limits row, and every gate.

## Gate results

Not run yet: the new module is not compiled until it is wired (turn 3).

## NEEDS THE MAC

None yet. Real-data checks and the `/dayflags.json` p50/p99/max on the real store will be
listed here as each fix lands.
