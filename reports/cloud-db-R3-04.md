STATUS: DONE

Every gate is green except two the cloud cannot close, both identical on the base: `cargo deny`
fails on exactly the base's five advisories, and `cargo test --workspace` has three failures in
crates this branch does not touch, each failing identically on a checkout of `origin/feat/db-fix`
f9cdb584 in this container (Gate results). The real-data, `:8080` and browser-harness checks are
listed under NEEDS THE MAC; none was run here and none is claimed.

# R3-04 (cloud): /db counts that agree — Bars and Coverage badges, the Coverage tab, Missing and Coverage %, the header line

Branch `cloud/db-R3-04-20261017`, cut from `origin/feat/db-fix` at 50093fa3; `origin/feat/db-fix`
f9cdb584 (R3-06, R3-14 — neither touches an R3-04 file) merged in at a058b0f0.

Prerequisites, confirmed merged into `origin/feat/db-fix` before the branch was cut:
`Merge R3-01: /db address and history: …` (00c07ee0) and `Merge WP-58: All sources page`.

Files changed against `origin/feat/db-fix` — all owned by R3-04, plus this report:
`web/src/routes/db/DbSummary.svelte`, `web/src/routes/db/DbCoverageGrid.svelte`,
`web/src/lib/db/notes-coverage.js`, `web/src/lib/completeness.js`, `web/src/lib/calendar-owed.js`,
`web/tests/completeness.test.js`, `web/tests/db-counts.test.js` (new). No file under `crates/`, no
`Cargo.toml` / `Cargo.lock` / `deny.toml`, no doc, no `web/build`. Nothing written to a store,
nothing POSTed, no server started, no pull request opened.

## Done — the work package, finding by finding

| Finding / goal | What changed | Named test(s) (web/tests) |
|---|---|---|
| **DB-048** Bars badge vs pager (434 against 443) | `barsBadge` + `barsQueryKey` (notes-coverage.js). DbSummary keeps the last *settled* server total under its query key — feed, rung, months, series, day/time bounds, re-read stamp; never page or order — and shows it on Coverage too. No day/time bound and no server count: the census count, labelled as such. A bound and no server count yet (a deep link to Coverage): `—` with the reason, never another set's number. The badge sends no request (source-shape test). | `the_bars_badge_is_the_servers_count_on_both_tabs` |
| Coverage badge read 4 on Bars, 7 on Coverage | `covCount` = matched months the window touches + absent months of each matched series the window touches; the Coverage grid, its pager and both tabs read it. Computed from an index of the matched series and rungs only (see Measured). | same test, and `coverage_follows_the_day_window_or_says_it_does_not` |
| **DB-046** dates vs Coverage | Coverage follows the From/To window: `coverWindow` (ordered), `monthInWindow`; a month is listed when the window touches it and judged whole, and the counted line says "months touching …". A window that empties Coverage says so once, both ranges in order (`windowEmptyWhy`), in the empty grid. | `coverage_follows_the_day_window_or_says_it_does_not`, `the window and the header at their boundaries` |
| **DB-014** per day, against the calendar | `owedLedger` gains `unheld*` prefix sums (what sessions with held digit `0` owe, from the series' own `/calendar.json?symbol&segment` answer the grid already reads). `judgeRow`: missing = max(owed − held, owed on unheld sessions), so a surplus never pays for a session with no bar (`unheld_sessions`); held = owed − missing. A month holding more than it owes with nothing known missing is `over` — no missing count, no ratio, never `full`, never 100.00% (`surplus_unplaced`). | `a_surplus_day_never_hides_a_short_day` |
| **DB-014** 100.00% for a short month | `coverPct` rounds DOWN (99.99% for 1 bar of 4,72,500; `<0.01%` for a sliver); every percentage on the grid, the counted line and the month cards uses it. | `a_month_missing_one_bar_is_never_100_percent` |
| **DB-014** drawer totals vs rows | `coverTotals` counts the counted line and the drawer; `missingText` prints `absent −N`, so the drawer's Missing is the sum of the figures in its column. | `the drawer's totals agree with the rows it prints`, `the counted line counts each kind once …` |
| Header line: strike in paisa, whole series | `contractTail` (`11,650 CE`); `headerRange` shows the From/To window "in view" (ordered) or the months "on disk"; the newest close shows only when it falls in the window, else "newest stored close … is after/before the window" (`closeInWindow`). | `the_header_prints_a_strike_as_a_price` |
| Refused front beside the coverage figure; empty grid says the same | `refusedFrontOf` over the day band's own cells (`front`, D-3315): a box beside the coverage figure and the same sentence in the empty grid. No request of its own. | `a refused front is named beside the coverage figure …` |
| **SC-CLOSE-17** | A future is owed `owed.derivatives[]` era counts (375, then 385 from 2026-08-03 — served by `/calendar.json` since SC-CLOSE-16), never the index count; an answer with no derivatives row judges no future (named); special/halt/unsized days are unknown to a future; a 1s future is trade-driven. A share on auction days is judged by `casFlag`: 1 → 360, 0 → 375; none (the census carries none) → between the counts `unmeasured` with `cas_unresolved`, never `full`; under both, short by at least the eligible shortfall. | `a share on auction days is judged by its dated flag …`, `a future is owed the derivatives venue's own count …` |

**Behaviour a reviewer should know changed** (each tested):
- `owedLedger` now **refuses** a `special` or `halts` entry with no day (it skipped them), as `withheldDays` refuses a malformed run; `readLedger` turns that into a named `why`.
- `isTradeDriven('1s', 'FNO', null)` is now `true` (a 1s future), and `isTradeDriven(tf, 'FNO', undefined)` stays `false`.
- The old CAS test asserted an unflagged share at 360 a session reads `full`; that was the SC-CLOSE-17 defect. It now passes `casFlag: true` for that case, and the unflagged case is its own test.
- Several guards that could never change an answer were removed so that no equivalent mutant remains (the repo's rule, e.g. `telemetry/src/sink.rs`): the span bounds are a `max`/`min` (a row's span cannot pass its expiry or the store's reach), `derivedPartial` keeps only the guard that changes an answer, `absentMonths` keeps only the empty-first-month guard. Each removal is commented at the site.
- `whyText` / `noteText` read only own keys (`constructor` is no reason).

## Measured here (node 22, this cloud machine; synthetic rows; not the browser targets)

Scratch scripts, not committed; p50 / p99 / max with n:

| measure | figure |
|---|---|
| Coverage count on the Bars tab, BEFORE this part's index fix (whole-census series index), 241,080 rows, n = 20 | 130.5 / 205.5 / 205.5 ms |
| Coverage count, matched-series-and-rungs index, 243,810 census rows, one instrument (105 matched rows), n = 200 | 5.09 / 34.85 / 48.44 ms |
| same, every instrument at one rung (27,090 matched rows), n = 200 | 27.06 / 47.64 / 49.15 ms |
| `judgeRow` per row with the per-day floor read, n = 200,000 | 0.014 / 0.087 / 44.2 ms (the max is one GC pause) |
| `coverTotals` over 50,000 judged rows, n = 200 | 3.59 / 10.33 / 10.54 ms |
| Bars badge (key + choice) per render, 300 series in the plan, n = 10,000 | 0.009 / 0.039 / 2.72 ms |
| `owedLedger` build, 3,195 days, once per calendar read, n = 200 | 0.38 / 4.67 / 7.18 ms |

The two coverage-count rows were taken while a cargo build shared the 4 CPUs. The page-side
targets (first content, longest pause, memory, wire bytes, request count) need the browser
harness and real census sizes: NEEDS THE MAC §5. Not measured here, not claimed.

## Gate results (branch tip after merging f9cdb584)

- `cargo fmt --check`: EXIT 0. `cargo clippy --workspace --all-targets --locked -- -D warnings`: EXIT 0
  (both on the merged tree, run as an unprivileged user).
- `cargo test --workspace --locked -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store`
  (the known slow cli test skipped, per CONTEXT): **not green in this container.** Run as root, api's
  permission tests refuse by their own message ("run the tests as a user the mode bits bind"), so the
  suite was run as an unprivileged user (with git's `safe.directory` set for it): **9,090 passed, 3 failed,
  47 ignored.** This branch changes no `.rs`, `Cargo.toml` or `Cargo.lock`, so every test binary is the
  base's. The three, each re-run alone on the branch and on a checkout of `origin/feat/db-fix` f9cdb584:
  - `store` `staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported` —
    "it is a new file, not the old one cut": this filesystem hands the new file the old one's inode
    number. Branch 3/3 fail, base 3/3 fail.
  - `store` `tests/cited_commits.rs` `every_commit_the_store_records_cite_is_on_main_or_says_why_not` —
    `docs/06-limits.md` (from the base) cites 023d6ab8, 079ad4af, 4756cce0, 521066e6, 72c91a46, which are
    on `feat/db-fix` and not on `origin/main`. Branch fail, base fail.
  - `cli` `boolean_search_command::integration_tests::generated_search_recovers_same_ordinal_and_refuses_missing_ancestry`
    — "only a durably readable reservation publishes an identity" (left 0, right 1). Branch fail, base fail.
  The 12 root-only api failures of the first run pass as the unprivileged user; of the five
  git-history tests that failed on "dubious ownership", four pass once `safe.directory` is set and
  the fifth is the `cited_commits` failure above.
- `cargo deny check`: EXIT 1 on the branch AND on the base checkout, with exactly the five advisories
  RUSTSEC-2026-0344 (ruzstd 0.8.2) and RUSTSEC-2026-0346, -0347, -0348, -0349 (rustls 0.23.45);
  bans, licenses and sources ok. DENY-1 owns them; nothing here touches Cargo files.
- Web, the touched tests: `node --test tests/completeness.test.js tests/db-counts.test.js
  tests/calendar-owed.test.js tests/calendar-span.test.js tests/calendar-dense.test.js
  tests/calendar-loader.test.js tests/calendar-building.test.js tests/db-coverage-life.test.js`:
  79 pass, 0 fail. The named tests are in `completeness.test.js` (`a_surplus_day_never_hides_a_short_day`,
  `a_month_missing_one_bar_is_never_100_percent`) and `db-counts.test.js` (the other three).
- Web, the whole suite (`npm test`, 1,437 tests): 1,431 pass, 1 fail, 5 cancelled. The five cancelled
  (`ask.test.js` x3, `census-worker.test.js` x2: "Promise resolution is still pending …") are cancelled
  identically on the base checkout; the one failure is `db-search.test.js`'s "inside one frame (16 ms)"
  p99 timing, which fails only while a cargo build shares the CPUs and passes run alone. No test that
  reads an R3-04 file fails.
- `svelte-check`: 0 errors and 0 warnings in any R3-04 file; 7 errors elsewhere (`gap-verdict.js` from
  the R3-06 merge, `ingest-months.js`, `routes/ingest/+page.svelte`, `tests/settings-ceilings.test.js`).
- Production build (`vite build`, in a scratch copy so the tracked `web/build` is untouched): succeeds,
  no warning in an R3-04 file.
- **Coverage** (node `--experimental-test-coverage`, the whole web suite): `completeness.js`,
  `calendar-owed.js`, `notes-coverage.js` — each **100.00% line, 100.00% branch, 100.00% functions**.
  The two Svelte components cannot be measured under node; every rule they apply is a function in
  those three modules, and source-shape tests bind each component to its function.
- **Mutation** (Stryker 8.7.1 installed in a scratch directory, nothing added to `web/package.json`;
  command runner over the eight test files above; all default mutators): the three touched modules,
  **1,735 mutants, 0 surviving** — 1,717 killed, 18 by timeout. Six passes: 335 survivors on the first
  full pass, then 14, 15, 4, 2, 0. Each survivor was either killed by a new assertion or, where no test
  could ever tell it from the original (an equivalent mutant), removed by restructuring the code so
  the operator no longer exists — the rule the repo already follows in `telemetry/src/sink.rs` and
  `engine/src/keep.rs`; nothing is suppressed with a disable comment. The last two (pass 6) were
  re-checked deterministically: each mutant re-applied to the final sources and the tests run.

## Left

Nothing in the work package. Not done here by rule, and listed under NEEDS THE MAC: the real-store
re-check of the controls and probes, the browser-harness figures, the `web/build` rebuild, and the
ledger and invariant entries (this part owns no doc).

Scratch left in place (nothing deleted): a detached worktree of `origin/feat/db-fix` at
`/tmp/r304-gates/base` (registered in `.git/worktrees`) and one in the session scratchpad; build
output in `/tmp/r304-gates/base-target`; a local unprivileged user `tester` in this container.

## NEEDS THE MAC

Nothing below was run in the cloud: there is no market data store, no `:8080` app, no shadow
server and no browser harness here. Every check is read-only; none POSTs, none writes the store,
none touches `:8080`. Run from a worktree of `cloud/db-R3-04-20261017` merged as the runbook
merges, with the machine's build queue first on `PATH` and a target directory of its own:

```sh
export PATH=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/bin:$PATH
export CARGO_TARGET_DIR=<this check's own target dir>
```

`P` is the port of a FRESH shadow server of the merged branch started with `state/fix/shadow.sh`
and `state/fix/serve.sh` (never `:8080`, never another agent's shadow), and

```sh
GET() { curl -s --noproxy '' "http://127.0.0.1:$P$1"; }
OPEN() { echo "http://127.0.0.1:$P/db?$1"; }   # paste into the browser harness / a browser
```

### 1. Merge order and the shared file

1. `web/src/lib/completeness.js` is also edited by R3-02. R3-04 changed, there: `denominators`
   (one `Math.max`, same answers), `isTradeDriven` (a 1s future), two new exports (`owedKind`,
   `coverPct`), the `JudgeContext` and `MonthRoll` typedefs, the body of `judgeRow` (operator window,
   span bounds, own held days, the calendar branch, the store-wide and derived checks),
   `derivedPartial`, `absentMonths`, `rollUpMonths`, `rowId` and `monthVerdict`. Unchanged:
   `denomKey`, `isSole`, `segmentOfKey`, `peersAtMost`, `istDayOf`, `STORE_WIDE_SHARE` and the module
   header. If R3-02 lands first, merge and run:
   ```sh
   cd web && node --test tests/completeness.test.js tests/db-counts.test.js "tests/db-*.test.js"
   ```
2. Then the whole web suite and the type check (the cloud's results are in Gate results):
   ```sh
   cd web && npm test && npm run check
   ```

### 2. The gates the cloud could not close here

This branch changes no file under `crates/` and no `Cargo.toml` / `Cargo.lock` / `deny.toml`
(`git diff origin/feat/db-fix HEAD -- crates Cargo.toml Cargo.lock deny.toml` is empty), so every
Rust test binary is the base's. Run as the operator (not root):

```sh
cargo test --workspace --locked
cargo test -p cli --locked the_pool_verb_prints_its_whole_page_on_a_generated_store   # the known slow one, skipped here
# Three tests failed here, in crates this branch does not touch (their outcome on the Mac is not
# claimed; each reads this machine's filesystem or the branch's history, not R3-04 code):
#   store  staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported
#          ("it is a new file, not the old one cut": this filesystem handed the new file the old inode number)
#   store  tests/cited_commits.rs every_commit_the_store_records_cite_is_on_main_or_says_why_not
#          (cites 023d6ab8, 079ad4af, 4756cce0, 521066e6, 72c91a46 — feat/db-fix commits not yet on origin/main)
#   cli    boolean_search_command::integration_tests::generated_search_recovers_same_ordinal_and_refuses_missing_ancestry
#          ("only a durably readable reservation publishes an identity", left 0 right 1)
cargo test -p store --locked a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported
cargo test -p store --locked --test cited_commits
cargo test -p cli --locked generated_search_recovers_same_ordinal_and_refuses_missing_ancestry
# cargo deny: the five advisories in Gate results fail on the base identically; DENY-1 owns them.
cargo deny check
```

### 2b. The tracked front-end build

`web/build/` is tracked and is rebuilt from the merged tree at merge-back (as R3-05 repair 5 and
R3-06 repair 1 did); it is not an R3-04 file and was not touched here. A production build of this
branch's `web/` in a scratch copy succeeded with no warning in any R3-04 file.

```sh
cd web && npm run build && git add build && git commit -m "R3-04: web build from the merged tree"
```

### 3. The calendar answers the per-day floor and the futures count read

R3-04 reads two `/calendar.json` fields it did not read before. Confirm the real answer carries
them (no server change was made):

```sh
# the series' own held digits for BANKNIFTY 1s, March 2022: 2022-03-07 must be a session with digit 0
GET '/calendar.json?feed=gdfl&rung=1s&symbol=BANKNIFTY&segment=INDEX' | jq '{from, held: (.held|length), sessionsNotHeld}'
# the derivatives eras at 1min: 375 to 2026-08-02, 385 from 2026-08-03
GET '/calendar.json?feed=zerodha&rung=1min' | jq '.owed.derivatives'
```

### 4. The controls and probes of the work package, by link, on the shadow

Read each on the Bars tab, on the Coverage tab, and back on Bars; then reload the link.
Expected after R3-04 is written beside each.

| # | link (`OPEN '…'`) | expected |
|---|---|---|
| 1 | `feed=gdfl&seg=spot&sym=NIFTY&tf=1s&from=2020-03-01&to=2020-03-31` | Bars badge = Bars pager total, on Bars and on Coverage; Coverage badge = Coverage rows = Coverage pager, same on both tabs |
| 2 | same + `&view=census` (deep link straight to Coverage) | Bars badge `—` (a window narrows and Bars has not counted it), title says so; after one visit to Bars it shows the server's count on both tabs |
| 3 | `feed=gdfl&seg=spot&sym=BANKNIFTY&tf=1s&from=2020-03-01&to=2020-03-31&view=census` | as 2; counted line says "months touching 01 Mar 2020 → 31 Mar 2020" |
| 4 | `feed=gdfl&seg=spot&sym=NIFTY&tf=1s&view=census` | Missing and Coverage per row; no month with a session held at 0 reads full or 100.00%; a month whose count exceeds what it owes reads `+N` (surplus unplaced), never full |
| 5 | row click on 2022-03 of `feed=gdfl&seg=spot&sym=BANKNIFTY&tf=1s&view=census` | 2022-03 short by at least 22,500 (2022-03-07, note `unheld_sessions`); drawer Missing = the sum of the figures in its Missing column (absent months print `absent −N`) |
| 6 | `feed=gdfl&seg=options&sym=NIFTY&tf=1s&exp=2019-03-28&k=1160000&side=CE&from=2019-03-15&to=2019-03-15` | Bars badge = pager (the 434), on Bars and Coverage (never the census 443); header "15 Mar 2019 → 15 Mar 2019 in view" |
| 7 | `feed=gdfl&seg=spot&sym=RELIANCE&tf=1s&from=2024-10-01&to=2024-10-31` | pager vs badges agree on both tabs |
| 8 | `feed=gdfl&seg=options&sym=NIFTY&tf=1s&exp=2019-03-28&from=2024-10-01&to=2024-10-31` (+ `&view=census`) | Bars 0, Coverage 0 on both tabs; the empty Coverage grid says once "The From and To window, 01 Oct 2024 → 31 Oct 2024, holds no month … this selection holds 01 … 2019 → 28 Mar 2019"; header names "newest stored close 28 Mar 2019 is before the window" |
| 9 | `feed=gdfl&seg=options&sym=NIFTY&tf=1s&exp=2019-03-28` | header line: a strike as a price (`11,650 CE`), never `1165000 CE` |
| 10 | `feed=gdfl&seg=options&sym=NIFTY&tf=1s&exp=2020-03-26&k=1160000&side=CE` | the Coverage tab's own number read on Bars, on Coverage, back on Bars: one number (the 4-against-7 probe) |
| 11 | `feed=gdfl&seg=options&sym=NIFTY&tf=1s` (Expired options, no expiry) by deep link and Back | empty Coverage grid and the counted boxes agree; with a refused-front day in the band, the box beside coverage and the empty grid carry the same sentence |
| 12 | switch from NIFTY options to `feed=gdfl&seg=options&sym=BANKNIFTY&tf=1s&exp=<any expiry>` | header line with more than one contract names the newest close's contract as `NN,NNN CE/PE` |
| 13 | `feed=zerodha&seg=spot&sym=<a CAS-eligible share>&tf=1min&view=census` on 2026-08/09 | auction months between 360 and 375 a session read `unmeasured` with "share-month(s) the auction flag decides", never full; a share under both reads short |
| 14 | `feed=<a feed holding futures at 1min>&seg=futures&sym=NIFTY&tf=1min&exp=<an expiry after 2026-08-03>&view=census` (only if one is stored) | owed 385 a session from 2026-08-03 (the 15:40 close), never the index's 375; with no derivatives row in the answer, "not measured" naming it |

### 5. The measured page-side target (browser harness, n = 200, cold and warm)

For links 1, 4, 6 and 8 above, on the Bars tab and on the Coverage tab, each cold (fresh
profile) and warm: first content p50/p99/max (target p99 < 300 ms), longest main-thread pause
(target none over 50 ms), JS heap (target < 100 MB), bytes on the wire (target < 400 KB), and the
count of requests (the badges must add none: the request list must equal the base's for the
same link). Run the same on a shadow of `feat/db-fix` before the merge for the before figures.
Write p50, p99, max and n for each into `docs/06-limits.md`.

Node-side figures taken here (synthetic census, this cloud machine, a cargo build competing for
the CPU): see "Measured here" above. The browser figures are not measured and not claimed.

### 6. Ledger and invariants (CLAUDE.md §9; this part owns no doc)

Decision, number taken at landing from the ledger's head (amendment A6):

> **D-<next> — /db counts that agree: the Bars badge is the server's count for the query on
> either tab; Coverage lists and counts the months the From and To window touches; Missing and
> Coverage are per day where the series' held days are read; a surplus no month total can place
> is never "full"; a future is owed the derivatives venue's count; a share on auction days with
> no dated flag on the wire is not judged between the two counts (R3-04; DB-048, DB-014, DB-046,
> SC-CLOSE-17).** 1. The Bars badge reads the live window answer or the last settled one kept
> under its query key (feed, rung, months, series, bounds, re-read stamp; never page or order);
> the census count only with no day or time bound; else `—`. No request of its own. 2. Coverage
> lists a month when the window touches one of its days and judges it whole; the same count is
> its tab's on both tabs and its pager's. 3. Missing = max(owed − held, owed on sessions whose
> own held digit is `0`); held = owed − missing. 4. A month holding more than it owes with
> nothing known missing is `over`: no missing count, no ratio, never full, never 100.00%; only a
> per-day count can clear it. 5. Percentages are rounded DOWN to two decimals. 6. A future owes
> `owed.derivatives[]` era counts on regular and auction days and nothing known on special, halt
> and unsized days; an answer with no derivatives row judges no future; a future at 1s is
> trade-driven. 7. A share on auction days owes 360 when its dated flag is 1, 375 when 0, and,
> with no flag (the census carries none), between the two counts it is unmeasured
> (`cas_unresolved`). 8. A `/calendar.json` `special` or `halts` entry with no day is refused by
> name, as a malformed `withheld` run is. Follow-up, not done here: serve the dated flag on the
> series' calendar answer so (7) can judge every share.

Invariants for `docs/04-invariants.md` (a new R3-04 section):

| Id | Invariant | Proof |
|---|---|---|
| R304-01 | One count for one selection and one window: the Bars badge is the server's count on both tabs, never another query's | `the_bars_badge_is_the_servers_count_on_both_tabs` (web/tests/db-counts.test.js) |
| R304-02 | Coverage lists, counts and pages the months the From and To window touches, and says once, in order, when the window empties it | `coverage_follows_the_day_window_or_says_it_does_not` |
| R304-03 | A surplus on one day never pays for a session with no bar; a surplus no month total can place is never full | `a_surplus_day_never_hides_a_short_day` (web/tests/completeness.test.js) |
| R304-04 | A month missing a bar never reads 100.00% | `a_month_missing_one_bar_is_never_100_percent` |
| R304-05 | The header prints a strike as a price and describes the window in view | `the_header_prints_a_strike_as_a_price` |
| R304-06 | The drawer's totals are the sum of the figures its rows print | `the drawer's totals agree with the rows it prints` |
| R304-07 | A future is owed the derivatives venue's count, never an index's; a share on auction days is judged by its dated flag and never called full between the counts without one | `a future is owed the derivatives venue's own count…`, `a share on auction days is judged by its dated flag…` |
