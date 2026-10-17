STATUS: IN PROGRESS

# R3-04 (cloud): /db counts that agree — Bars and Coverage badges, the Coverage tab, Missing and Coverage %, the header line

Branch `cloud/db-R3-04-20261017`, cut from `origin/feat/db-fix` at 50093fa3.
Prerequisites confirmed merged into `origin/feat/db-fix`: `Merge R3-01: …` (00c07ee0) and `Merge WP-58: All sources page`.

Files edited (all owned by R3-04): `web/src/routes/db/DbSummary.svelte`, `web/src/routes/db/DbCoverageGrid.svelte`,
`web/src/lib/db/notes-coverage.js`, `web/src/lib/completeness.js`, `web/src/lib/calendar-owed.js`,
`web/tests/completeness.test.js`, `web/tests/db-counts.test.js` (new), plus this report.
No file under `crates/`, no `Cargo.toml` / `Cargo.lock` / `deny.toml`, no doc.

## Done

1. **Bars badge = the server's count on both tabs (DB-048).** `barsBadge` + `barsQueryKey` (notes-coverage.js);
   DbSummary keeps the last *settled* server total under its query key (feed, rung, months, series, bounds, nonce —
   never page or order) and reads it back on Coverage. No day/time bound: census count, labelled. A bound and no
   server count yet: `—` with the reason, never another set's number. No request of its own (source-shape test).
2. **Coverage badge = one count on either tab (the 4-against-7 probe).** `covCount` = matched months the window
   touches + absent months of each matched series the window touches; the Coverage grid, its pager and both tabs
   read it.
3. **Coverage follows the From/To window (DB-046).** `coverWindow` / `monthInWindow`; a month is listed when the
   window touches it and judged whole (said in the counted line: "months touching …"). A window that empties the
   grid says so once, both ranges in order (`windowEmptyWhy`), in the empty grid.
4. **Per day, against the calendar (DB-014).** `owedLedger` gains `unheld*` prefix sums: what sessions with held
   digit `0` owe, read off the series' own `/calendar.json?symbol&segment` answer the grid already reads.
   `judgeRow`: missing = max(owed − held, owed on unheld sessions), so a surplus on one day never pays for a session
   with no bar (note `unheld_sessions`). A month holding more than it owes with nothing known missing is `over`
   (missing and ratio `null`, note `surplus_unplaced`), never `full` / 100.00%. Ratio = (owed − missing) / owed.
5. **Never 100.00% for a short month.** `coverPct` rounds down (99.99% for one bar of 4,72,500), `<0.01%` for a
   sliver; used by every percentage on the grid, the counted line and the month cards (`pctText` no longer used there).
6. **Drawer totals agree with the rows.** `coverTotals` counts both the counted line and the drawer; `missingText`
   prints `absent −N` for an absent month so the Missing total is the sum of the figures shown.
7. **Header line (SC: "11,650 CE").** `contractTail` prints the strike in rupees; `headerRange` shows the From/To
   window "in view" (ordered) or the months "on disk"; the newest close is shown only when it falls in the window,
   else "newest stored close … is after/before the window".
8. **Refused front named beside the coverage figure; empty grid says the same words.** `refusedFrontOf` over the
   day band's own cells (`front` kind, D-3315) — a box beside coverage and the same sentence in the empty grid.
9. **SC-CLOSE-17.** A future is owed `owed.derivatives[]` era counts (375 → 385 from 2026-08-03, served by
   `/calendar.json` since SC-CLOSE-16), never the index count; an answer with no derivatives row judges no future
   (named). A share on auction days is judged by `casFlag`: eligible → 360, flag 0 → 375, unknown (the census
   carries none) → between the two counts it is `unmeasured` with `cas_unresolved`, never `full`. A 1s future is
   trade-driven (the calendar's derivatives row at 1s is a ceiling).
10. **Cost on the Bars tab.** The Coverage count indexes only the matched series' rows (Set probes over the census);
    the whole-census index stays Coverage-only.

## Left

- Mutation pass 3 on the three touched modules (in progress; pass 2 left 335 survivors, since pinned by
  tests or removed as equivalent spellings), the cargo suite on the merged tree, then the final report.
- Merged `origin/feat/db-fix` (f9cdb584: R3-06, R3-14) into the branch at a058b0f0; neither touches an R3-04 file.

## Gate results (so far)

- `cargo fmt --check`: EXIT 0. `cargo clippy --workspace --all-targets --locked -- -D warnings`: EXIT 0.
- `cargo test --workspace --locked`: as root, api's permission tests fail by their own message ("run the tests as a
  user the mode bits bind"); re-running as an unprivileged user — result pending. No Rust file differs from the base.
- `cargo deny check`: EXIT 1 with exactly the base's five advisories (RUSTSEC-2026-0344 ruzstd; RUSTSEC-2026-0346,
  -0347, -0348, -0349 rustls); bans, licenses, sources ok. `git diff origin/feat/db-fix -- Cargo.toml Cargo.lock
  deny.toml crates` is empty, so deny on the base is the same five.
- Web: `node --test` on the touched tests green; full `npm test` — no new failures (see final report).
- Coverage (node `--experimental-test-coverage`, whole web suite): completeness.js, calendar-owed.js,
  notes-coverage.js each 100.00% line, 100.00% branch, 100.00% functions.

## NEEDS THE MAC

To be completed in the final report.
