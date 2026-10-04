# tests-docs-security pass 19 (tds19): strength of the web front end's own tests

Checkout: /home/claude/wt/zero3 at 1f4de71 (read-only). The method was mutation testing. I copied web/ into the scratchpad, planted one hand mutation at a time, ran only the test files that cover that module under Node 22.22.0 (`node --test`), and restored the file. There is no node_modules and nothing was installed. Tests that import `svelte/compiler` ran against a 1-line stub that throws. My harness compares the failing-test set against an unmutated baseline, so a test that fails only because of the stub never counts as a kill. Harness: scratchpad/mut/{muts*.mjs,run*.mjs}. Results: scratchpad/mut/results.txt.

Earlier IDs not repeated: P13-01 (stale bundle / W1), P15-17 (gate 6d), P2-02-01 (proxy scanner) and P2-02-02 (prefix timing tests).

## 1. Inventory

- web/tests: 86 `*.test.js` files with 797 `test(` calls, plus 5 fixture modules. There are no `src/**/*.test.*` files. Native Rust tests under web/ belong to gate 6d (P15-17).
- Routes with **no test at all**: `src/routes/markets/+page.svelte` (4,243 lines) and `src/routes/mapping/+page.svelte` (1,260 lines). The audit, gaps, ingest and terminal routes each have 1 file that reads them. db has 5, autopilot 2 and backtest 19.
- src/lib modules that no test imports: `condition-groups.js` (see P19-02), `feeds.svelte.js`, `index.svelte.js`, `runtime-inspection.svelte.js` (6 lines) and `saved-timeframes.js`. saved-timeframes.js is reached indirectly through boolean-qualified-search, qualified-campaign and research-tester.
- Every other src/lib/*.js module has at least one test that imports the shipped module.

## 2. Hand mutations (54 run; 41 killed, 13 survived)

| # | module:mutation | result / killing test |
|---|---|---|
| M01 | money.js rupee drops padStart(2) | KILLED "a paisa integer becomes rupees, always to two places" |
| M02 | rupee drops the minus sign | KILLED "a negative paisa keeps its sign" |
| M03 | exact() rounds | KILLED "exact does NOT round" |
| M04 | LOC en-IN -> en-US | KILLED "the grouping is INDIAN" |
| M05 | basisPoints `>=` -> `>` (half) | KILLED "a half rounds away from zero in BOTH directions" |
| M06 | basisPoints step always +1 | KILLED same + "mirror-image loss" |
| M07 | bpsText drops '-' | KILLED "a ratio reads as a signed percentage" |
| M08 | dirOf(undefined) -> flat | KILLED "`none` is its own word" |
| M09 | monthLabel accepts month 13 | KILLED "an unparseable month key comes back verbatim" |
| M10 | IST zone -> UTC | KILLED "18:29Z and 18:30Z are different IST days" |
| M50 | hourCycle h23 -> h24 | KILLED "midnight is 00:00 and never 24:00" |
| M12 | timeLabel empty -> '' not '—' | KILLED |
| M13 | gap-verdict drops 'short' | KILLED |
| M14 | drops 'nothing owed' | KILLED |
| M15 | isWholeAudit drops month.length===months | KILLED |
| M16 | isWholeAudit drops covers_span | KILLED |
| M17 | evidence_error kind absent -> bad | **SURVIVED** (P19-04) |
| M18 | chargeScope counts a whitespace note | KILLED |
| M19 | coverScope 1 stock -> "spot indices only" | KILLED |
| M20 | equity_note ignored for an index name | KILLED |
| M21/M22 | roundedScaledRatio half rules | KILLED (both) |
| M23 | supportRatioKey unreduced | KILLED |
| M24 | compareRuns sort reversed | KILLED |
| M25 | fillAssumptionGap `>` -> `>=` | KILLED |
| M26 | classify partial-span months clause | SURVIVED. This is an equivalent mutant: validateRun already refuses months_found<months_asked with whole_span true (comparison.test.js:225) |
| M27 | exactIntegerDelta unchecked | KILLED |
| M28/M29/M30 | store-census cache / 304 validator / retainedKey | KILLED (all) |
| M31/M32/M33 | feed-summary degraded header / wrong feed / foreign absent | KILLED (all) |
| M35/M37 | frontier admitted recount / stop_unchecked | KILLED |
| M34/M36/M38/M39/M40/M41 | frontier validator | **SURVIVED** (P19-03; M40 is near-equivalent) |
| M42/M43 | frontier-pages sameRules / next_page | KILLED |
| M44 | rows.js accepts negative rows | KILLED |
| M45/M46/M47 | backtest rankRows | **SURVIVED** (P19-01) |
| M51 | impliedConditions direction | **SURVIVED** (P19-02) |
| M48/M49 | page behaviour kept only as text in a comment | **SURVIVED** (P19-05) |
| M54 | ingest isoOfEpochDay off by one | **SURVIVED** (P19-06) |

M11 did not apply (the pattern was not unique); M50 is its replacement. M52 and M53 (the db decorateRow) were inconclusive under the svelte stub, because the test that would catch them (database-preparation.test.js:24-28) needs the real compiler. They are not counted.

## 3-5. Findings

### P19-01 medium: the backtest page's operator ranking `rankRows` has no test, and its three core rules can be inverted with the suite green
- Where: web/src/routes/backtest/+page.svelte:886-980; used at :982 and :1168.
- Code: `w.drawdown * (1 - norm(r.max_drawdown, ranges.drawdown)) +` … `if (!Number.isFinite(lo) || hi === lo) return 0.5;` … `scored.sort((a, b) => b.score - a.score || a.rank - b.rank);`
- Why it is wrong: this function orders the "best of these" table using the operator's eleven weighted criteria. It is defined inside the page, so node cannot import it. No test extracts it, and `grep -rn rankRows web/tests` is empty. Inverting "less drawdown is better", scoring a constant column 0 instead of 0.5, or sorting ascending all leave every test green. CLAUDE.md §4 says a surviving mutant is a missing test.
- Repro (ran): M45, M46 and M47 against all 15 dependency-free tests that read backtest/+page.svelte. All three SURVIVED.
- Fix: move rankRows (with span/norm/losingPct/lossRatio) into `src/lib/rank-rows.js` and import it in the page. Add tests for: lower drawdown ranks higher, a constant column contributes 0.5, null is excluded from the range, ties break on `rank`, and the order is descending.

### P19-02 low: `impliedConditions` ships untested; flipping its direction rule marks the wrong pivot levels as restatements
- Where: web/src/lib/condition-groups.js:160-194; rendered at backtest/+page.svelte:7344-7374 ("{names.length - implied.size} of {names.length} carry the setup").
- Code: `if (above ? member.rank > tightest.rank : member.rank < tightest.rank) {`
- Why it is wrong: no test imports condition-groups.js. With the comparison swapped, the page dims the informative condition, says the tightest level is "implied", and prints the same count. The answer is wrong and looks the same.
- Repro (ran): M51 SURVIVED. `grep -rn condition-groups web/tests` is empty.
- Fix: add tests/condition-groups.test.js covering above s2+s3 (s3 implied), below s1+s2 (s2 implied), opposite sides (none implied), `_band` stripping, and non-pivot families.

### P19-03 low: six frontier-validator rules can be deleted or changed without a test failing, including the browser copy of the Wilson bound that Rust says must match to the fourth basis point
- Where: web/src/lib/frontier-analytics.js:73, :78, :85, :185, :240, :249; tests/frontier-analytics.test.js and frontier-pages.test.js.
- Code: `const z = 1.959964;`, `return Math.trunc(Math.min(10_000, …`, `if (body.count > body.rules.top) {`, `expected === I64_MAX ? wire === null :`, `if (row.rank !== at + 1 || seenRanks.has(row.rank)) {`
- Why it is wrong: crates/runner/src/grid.rs:465-467 writes z out in full "because the rounded value shifts the bound in the fourth basis point and this number is compared against a rule". The browser copy re-derives `meets.assurance` and refuses the whole frontier when the two disagree. The only fixture is 3/4 and 1/4 wins against a 3,000 bp rule, so lowering z or switching trunc to round is not caught. Separately: no fixture has count > rules.top. No fixture has ranks that skip 1 (only a duplicate rank is tested). No fixture has a finite reward_to_risk_bp where Rust sends null; the "nullable ratios" test only checks that null is accepted.
- Repro (ran): M36 (z=1.645), M41 (trunc→round), M38 (drop top check), M39 (accept any wire for I64_MAX), M34 (drop `rank !== at+1`) all SURVIVED both test files. M40 (drop the losses identity) also survived, but the avg_loss check covers it.
- Fix: add a table of (wins, trades, expected bp) generated from `GridCell::assurance_bp` at boundary values, and assert that `meets.assurance` flips at min_assurance_bp ±1. Add one case each for rows.length > top, ranks [2,3], and `reward_to_risk_bp: 300` with losses 0.

### P19-04 low: gap-verdict's colour for missing session evidence is unpinned
- Where: web/src/lib/gap-verdict.js:28 `if (month.evidence_error != null) return { word: 'unverified', kind: 'absent' };`
- Why it is wrong: tests/gap-verdict.test.js:77-79 checks `.word` and `isWholeAudit` only. Changing `kind` to 'bad' (red, "damaged") passes, but the precedence test's title promises that missing evidence is named distinctly from damage.
- Repro (ran): M17 SURVIVED.
- Fix: assert `.kind === 'absent'` at :78.

### P19-05 low: tests whose titles claim runtime behaviour assert only that text exists somewhere in the source, so a comment satisfies them
- Where: tests/bounded-comparison.test.js:29-34 ("…aborted on close/replacement…"); tests/cash-identity.test.js:12-17 ("…never renders an unknown denominator as zero"). The same shape appears in about 18 other tests, for example backtest-ui.test.js:238 and charge-scope.test.js:22-30.
- Code: `assert.match(page,/return \(\) => \{ boardSeq \+= 1; boardAbort\?\.abort\(\); \}/)`; `assert.ok(page.includes("? 'unverified' : n(r.exp)"))`
- Why it is wrong: I commented out the effect's cleanup (`/* return () => {…abort()…}; */`), so closing the board no longer aborts. Separately, I rendered `/ {n(r.exp)}` with the old expression left in an HTML comment, so an unknown denominator now renders as a number. Both tests stay green. They pin spelling, not behaviour, and pass whenever the text survives in any form.
- Repro (ran): M48 and M49 SURVIVED.
- Fix: extract the expressions through `svelte/compiler` `parse` and evaluate them, as gap-verdict.test.js:166-190 and database-page-fixture.js already do. At minimum, match against the parsed script/template AST with comments stripped.

### P19-06 low: two tests exercise a test-local copy of page arithmetic rather than the shipped code
- Where: tests/calendar-owed.test.js:59-61 `function isoOf(day) { return new Date(day * DAY_MS).toISOString().slice(0, 10); }`, which duplicates ingest/+page.svelte:1053-1055 `isoOfEpochDay`, the function the page passes to `withheldDays` at :1099. Also tests/completeness.test.js:27-45 `deco`/`decorate`, which duplicates db/+page.svelte `decorateRow` (:1049-1126), while the file's header (:4-5) says "nothing here is a copy of the page's arithmetic".
- Why it is wrong: the shipped epoch-day converter is untested. Shifting it by one millisecond (every withheld day moves one day earlier) passes calendar-owed, cash-identity and find. completeness.test.js's `pct`/`short` assertions test its own copy. database-preparation.test.js:22-28 does drive the real decorateRow in CI, so that half is a false header rather than a coverage hole.
- Repro (ran): M54 SURVIVED. M52/M53 were inconclusive without real svelte.
- Fix: export `isoOfEpochDay` from calendar-owed.js (or dates.js), use it in the page and the test, and correct completeness.test.js's header or use the fixture's real decorateRow.

### P19-07 low: ask.test.js's timeout test cannot finish on Node 22 or earlier; the project's own recorded toolchain is Node 22, and nothing pins 24
- Where: web/tests/ask.test.js:36-50, against web/src/lib/ask.js:62 `AbortSignal.timeout(ms)`. web/package.json has no `engines`, and there is no .nvmrc. CI pins Node 24 (ci.yml:7798). docs/05-decisions.md:55930 (D-1507) records rebuilding "under Node 22".
- Why it is wrong: the fake fetch never settles, and Node keeps no handle open for `AbortSignal.timeout`'s unref'd timer. The event loop drains, the test is cancelled ("Promise resolution is still pending but the event loop has already resolved"), and the next two tests are cancelled with it. `node --test web/tests/*.test.js` exits 1 for a non-defect on the documented local toolchain. Each of the three passes when run alone with `--test-name-pattern`, except the timeout test. CI on Node 24 is green (run 37221916556, W2 success).
- Repro (ran): `node --test tests/ask.test.js` on Node 20, 21 and 22 gives pass 1, cancelled 3, exit 1.
- Fix: keep the loop alive inside the test (`const hold = setInterval(() => {}, 1000)`, cleared in `finally`), or add `"engines": {"node": ">=24"}` and say so in docs.

## 5. CI wiring (no new finding)
- W2 (ci.yml:7825-7826) runs `node --test web/tests/*.test.js` after `npm ci`, so real svelte is present. It is not `continue-on-error`. The `web` job runs on pull_request and push to main, and auto-merge.yml:259-297 refuses on any failed check run.
- Tests import `../src/lib/*`, `../saved-backtest/*` and `../masters.js`. Page tests read `src/routes/*/+page.svelte`. **No test reads web/build.** W1 (:7806) is the only link between tested source and the served bundle (P13-01).
- W2 calls node directly, so npm's `pretest` (policy-guide `--check`) does not run there. W1's `prebuild` and W3's `precheck` run it, so it is still gated.
- W3 (:7828) is svelte-check with CEILING=0 and fails when the summary line is missing. Sound.
