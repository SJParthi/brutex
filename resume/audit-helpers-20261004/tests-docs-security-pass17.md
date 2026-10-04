# tests-docs-security pass 17 (tds17): docs/06-limits.md and docs/07-plan.md against the code

Tree: /home/claude/wt/zero3 at 1f4de71 (read-only). No cargo run. Every finding was traced from source.
Not re-reported: p12num-1, p13num-5, conc14-2, CE-92, P13-04.

## Counts

- docs/06 limit statements checked by hand: **64** (table A).
- docs/07 items checked: **50** (table B). That covers every DONE row in §2, R-1 to R-10, every §3 BLOCKED row, every §4 NEXT row, §6, §7.1 to §7.3, §9.1, §9.3, §10 and the OPEN grammar section.
- Mechanical cross-check of docs/06: 647 backticked `a::b` code paths, all of which resolve to a definition, a field or std. There are also 166 cited test names. 162 of them exist. The other 4 are explained: one test was renamed when it was inverted (D-1410), two live in `web/saved-backtest/viewer.rs`, and one is described as historical.
- New findings: **21**, all real at 1f4de71. 1 is medium (P17-01) and 20 are low.

## Findings

### P17-01 medium: the Ingest page reads a receipt fact the server no longer emits, so truncated failures are reported as "no failure"
- `web/src/routes/ingest/+page.svelte:5732`: `const raw = factValue.get('Members failed');`
- The server emits `facts.push(("Failure diagnostics", done.failures.len().to_string()));` at `crates/api/src/server.rs:10625`, and its own test asserts `assert!(!page.contains("Members failed"));` at `server.rs:19837`.
- No `Members failed` fact is emitted anywhere in `crates/`. So `failedCount` is always 0, and `hiddenFailures = Math.max(0, failedCount - namedFailures.length)` is always 0.
- Why it is wrong: the server sends at most five `Failed` facts (`for f in done.failures.iter().take(5)`, server.rs:10650). On a run with 6 or more failures, the page says "The run reported no failure for any of them" (+page.svelte:6018). It says the same per row (:5895). Both are false statements about a run that did record failures. This is the "fallback that hides a failure" that `CLAUDE.md` §4 bans. The header comment at :13 still describes the old key.
- Repro (not run, traced): ingest an archive whose members fail more than five times. The receipt carries `Failure diagnostics` = N and 5 `Failed` rows. The page shows every silent instrument as "reported no failure".
- Fix: read `factValue.get('Failure diagnostics')`. Both numbers count `done.failures`, so the subtraction stays valid. Add a `web/tests` case that pins the key against `server.rs`.

### P17-02 low: docs/07's grammar section still lists D-0753 ("one node is not O(1)") as OPEN; the code is incremental
- `docs/07-plan.md:1041`: "Each grammar choice revalidates its whole prefix … Closing it needs an incremental validator whose per-position state `decode` can rebuild".
- The code: `crates/vocab/src/expression_search.rs:208` `let Some((start, depth)) = self.place(at) else {`, and `place` is at :238. The whole-prefix `valid_prefix` is now `#[cfg(test)]` only (:379, :386). docs/06 lines 11153 and 11386 already record this as fixed (o1engine-23).
- Fix: mark the bullet closed by o1engine-23/D-0983. Keep the remaining caveats: the sibling comparison is bounded by the program length, and there is no bench.

### P17-03 low: docs/07 §10 marks four handled interruptions as open, and closes on "a hard stop today"
- Rows 12/13 say "❌ retry with backoff". Retry exists: `with_retry` (server.rs:9677) has a throttle ladder and a 5xx ladder (`SERVER_ERROR_ATTEMPTS`), and tests `the_retry_policy_ladders_back_off_and_then_stop` (:29452).
- Row 14 says "No loop yet … ❌ per-instrument isolation". The loop exists at server.rs:7928, with the comment "PER-INSTRUMENT ISOLATION. One instrument that fails does not abort the other 799" (:7856).
- Row 15 says "Rerun … redoes everything". Not so: `autopilot.rs:22` "The resume point comes from the store, not from a cursor."
- The closing line (docs/07:542) says "#11 … It is a hard stop today". That contradicts row 11's own ✅: `credential_halts` re-reads, and a rotated value continues the run (server.rs:8059-8069).
- Fix: flip rows 12-15 to ✅ with their anchors, and delete the closing sentence.

### P17-04 low: R-3 and NEXT 4a say Groww's daily interval word is unrecorded; it is recorded
- `docs/07-plan.md:95` says "Open only for Groww, whose daily interval word is unrecorded". Line :172 says "A daily pull against that feed refuses by name until it is recorded".
- The code: `crates/pull/src/vendor.rs:4785` `(Granularity::Day1, "1day"),`, sourced as D-0076 ("THE DAY SPELLING IS `1day`, AND IT IS NOW SOURCED", :4771).
- Fix: mark 4a done (D-0076), and drop the qualifier from R-3.

### P17-05 low: docs/07 §4's arithmetic says Groww has no published daily cap; the code says 180 days and calls the plan stale
- docs/07:183: `| Day-level, Groww | none published |`. Line :188 says a 180-day daily cap "appears in no source".
- The code: vendor.rs:4766 `window_caps: &[(Granularity::Minute1, 30), (Granularity::Day1, 180)],`. Its comment says "`docs/07-plan.md` went further and said a 180-day daily cap 'appears in no source' — it appears in the vendor's own limits table … the plan is the stale copy" (:4754).
- The same comment block also contradicts itself: vendor.rs:4744-4746 says "The day row is absent … UNVERIFIED, and absent rather than guessed".
- The window count (80, bound by the month) is unaffected.
- Fix: correct the table row and the paragraph. Delete the stale three lines in vendor.rs. Record the source in `docs/00-charter.md` §4 if it is not already there (golden rule 1).

### P17-06 low: R-2 says "never today" and §9.1/§10 say yesterday is the newest day asked for; the guard admits today once the session has closed
- docs/07:94 R-2: "2020-01-01 → yesterday, never today | `finished_day_only`". §9.1 (docs/07:443) says "Never requested: yesterday is the newest day asked for".
- The code at server.rs:8305: "TODAY IS ASKABLE ONCE ITS SESSION HAS CLOSED". The guard now tests whether the session has ended.
- Fix: restate R-2 and the §9.1 row as "no day whose session has not closed".

### P17-07 low: docs/07 §9.3/§9.4 say the trading calendar and the EXPECT step do not exist; both are built, and the 375-bar row misattributes CAS
- docs/07:476 says "the part with no code yet". :481 says "holidays do not [exist]". :485 says "Without a trading calendar…". :489 quotes docs/06 saying "no trading calendar exists in this build".
- That quote no longer appears anywhere in docs/06 (grep finds nothing).
- `pull::calendar` (calendar.rs) is exactly the "trading calendar derived from data" that §9.3 calls the prerequisite: `kind_of`, `expected_bars`, `sessions_between`, through 2026-09-04. The DIFFER step exists too: `pull::gaps::classify*`.
- Row 3 reads "375 at one-minute, 09:15–15:29 (CAS, from 2026-08-03)". In fact CAS moved the swept index close to 15:15 from that date (session.rs:1113-1115). 375 is the anchor and cash figure, not the index figure after 2026-08-03.
- Fix: rewrite §9.3 rows 2-3 and the paragraph, keep §9.4's "not guaranteed" wording, and drop the dead quote.

### P17-08 low: docs/07 §6 maps duplicate rejection to the wrong bench row; docs/06 §53 says reruns are still owed that docs/04 already records
- docs/07:299: "Duplicate rejection | `C-E-04` | a whole ladder walk". docs/04:198 maps rule-4 duplicate rejection (C-03) to `C-E-10`, which times `engine::primitives::offer`.
- docs/06 §53 (:3666-3675) says the corrected C-03/C-04 benches "have not yet been remeasured … the former ratios are historical". docs/04:2072-2077 records reruns: D-0924 (1.011×/0.993×, 1.015×/1.204×) and D-1448.
- Fix: point docs/07 at C-E-10. Close the C-03/C-04 half of §53 with the two decision numbers. E-08 stays unmeasured, which is still true.

### P17-09 low: docs/06 §3 and §30 still state the no-script era
- §3 (:35-47) says "Zero hand-written non-Rust source is achievable and enforced" and "the generated loader … is not committed". In fact `web/` is unrestricted (CLAUDE.md §2, D-0052/D-0053), Svelte and JS are hand-written, and `web/build` is committed (D-0068).
- §30 (:2558-2583) says "there is no incremental type-ahead … and there will not be one while §2 stands". But `/typeahead.js` is routed (server.rs:16560), `render.rs:1168` injects it into every Rust page, and docs/06 §82 (:5189) itself describes it.
- §31's limit still holds (`calendar::tests::the_arrows_step_one_month_and_do_not_cross_a_year_boundary`, calendar.rs:868), but its reason ("§2 forbids solving with a script") no longer holds.
- Fix: mark §3 and §30 superseded by D-0052/D-0053/D-0068, and restate §31's reason as a choice.

### P17-10 low: docs/06 §4 reports three per-operation timings that no harness in the repository produced
- docs/06:58-60: "Superset test | 0.192 ns per bar·mask | 94,000 bars × 10,000 masks"; "Trade walk | 1.28 ns … | same harness"; "Frequent-count pass | 0.257 ns …".
- Neither the figures nor the harness appear in any bench, test, docs/04 row or decision (grep of crates/ and docs/). The only "94_000" in crates/ is a candle price at runner/grid.rs:8635.
- `CLAUDE.md` §3 rule 6 says never claim a measurement you did not take. These read as current measurements of this engine, with no date and no machine run.
- Fix: label them as predecessor-repository figures that cannot be reproduced here, or replace them with C-V/C-E bench rows.

### P17-11 low: docs/06 §14 says no bar reader exists and x86_64 is unmeasured; both have changed
- docs/06:655 says "no such run has been made, because no bar reader exists". `store::file::BarFile` reads bars, and `cli sweep-stored` uses it (CLAUDE.md §5).
- docs/06:664 says "`x86_64` timings are UNMEASURED … the speeds on the machine CI actually uses have never been taken". The D-0914 section (:11965-11968) reports C-07, one block's seal, at about 2.45 µs on a shared x86_64 host.
- The extrapolation label on the whole-lake figure is still correct.
- Fix: drop the reader clause, and cite the D-0914 x86_64 figure.

### P17-12 low: docs/06 §28 says in the present tense that CI has no mutation step
- docs/06:1943: "**CI has no `cargo-mutants` step**, so `CLAUDE.md` §9's 'no surviving mutant' bullet is enforced by nobody". The heading still reads "red today".
- `.github/workflows/ci.yml:7536-7550` installs and runs `cargo-mutants 26.2.0` (gate 18, D-0078, docs/06 §43).
- Fix: add a dated note that gate 18 closed this for touched code, and that the workspace-wide gap stays as §43 states it.

### P17-13 low: two source defects docs/06 §28 reported on 2026-08-07 are still in the tree
- `crates/store/src/format.rs:865`: "walks all 448 (field, byte) positions". The test counts 56: `crates/store/tests/unit.rs:639` reads "7 x 8 = 56 placements".
- `crates/api/src/catalog.rs:477`: "that is `docs/04-invariants.md` C-11". C-11 is the pull census bench (docs/04:211). The page bound is C-14, as catalog.rs:344 itself says.
- Fix: change 448 to 56 and C-11 to C-14.

### P17-14 low: docs/06 §43 still says 15 lake mutants survive, but 13 of them are now killed by a test
- docs/06:3216-3235: "The 15 that survive today", with `batch.rs` 13 for `from_cash_columns`, and "the number is 15".
- `crates/lake/src/batch.rs:229` `one_short_column_is_enough_to_refuse_and_every_column_is_checked` drives each column mismatch alone (:252-285). Its doc comment names those exact 13 mutants.
- Fix: restate as at most 2 (`page.rs`), pending a gate-18 run over `crates/lake`.

### P17-15 low: docs/06 §85's "floor of five" equivalent core mutants is stale; four of the five mutation sites no longer exist
- docs/06:5413-5417 names `isin.rs (n - 1 - i) % 2` (twice), `universe.rs 1 << 0`, `vendor.rs 1 << 0` and `symbol.rs is_empty → false`.
- The code now reads `if i % 2 == 0` (isin.rs:157) and `Self::Groww => 1,` (vendor.rs:182). No `1 << 0` remains in `crates/core`. `is_empty` is `pub const fn … { false }` (symbol.rs:105-107).
- A future run that reads a floor of five will mistake real survivors for equivalents.
- Fix: re-measure `cargo mutants -p core`, or state the floor as unmeasured since those rewrites.

### P17-16 low (regressed against the doc): `census::held_series` runs per request; docs/06 §32 and the gate-11 rule-4 reason say startup only
- docs/06:2612: "**It is not on a request path.** `api::server::Site::new` computes it once at startup". docs/06:13280: "held_series   startup only. Its one production call site is `Site::new`".
- The code: `store_html` calls `census::held_series(&censuses)` per `show=gaps` request (server.rs:15449, "O(keys log keys) per `show=gaps` request", UC-20/D-1446).
- The allowlist reason a gate carries is the one this gate-11 text exists to keep honest. The text itself says "If any of these moves into one, the entry is wrong".
- Fix: amend §32 and the rule-4 entry to "startup and `/store?show=gaps`".

### P17-17 low: docs/06 §66 says `run_local` decodes every archive feed with GDFL's ten columns; it now reads the layout
- docs/06:4340: "`columns: pull::csv::Columns::Gdfl` … both literals". The code: server.rs:10964 `let Some(layout) = archive.layout(Segment::Fno) else {`, and `columns: layout.shape` (:10976), D-0344 (docs/07:351 already says so).
- The segment literal half still stands.
- Fix: close the columns half of §66 and keep the segment half.

### P17-18 low: docs/06 §49 says a halted feed cannot be revived without a restart; two halt kinds revive themselves
- docs/06:3496: "A halted feed cannot be revived without restarting the server".
- The code: `FeedState::revive` (autopilot.rs:1285) is called when the manifest loads again (`Halt::Census`, :2848-2851) and when a store write probe succeeds (`Halt::Store`, :2987). Credential and configuration halts still need a restart.
- Fix: scope the title to credential and configuration halts.

### P17-19 low: form-size arithmetic. "About 26.5 MB" matches neither unit, and docs/06's D-1202 section still bounds every form at 8,192
- docs/06:14512 and pullrun.rs:79 say "About 26.5 MB". From the constants: `MAX_MEMBER_FORM_BYTES` = 8,192 + 2,000×80 = 168,192, `MAX_LEG_FIELD_BYTES` = 4 + 5×176,384 = 881,924, and `MAX_RUN_FORM_BYTES` = 8,192 + 31×881,924 = **27,347,836 B**. That is 27.35 MB or 26.08 MiB, so 26.5 is neither (computed, not run).
- docs/06:12229 (D-1202): "(a form body: at most `MAX_FORM_BYTES`, 8,192). That is constant per request because both lengths are capped". `/ingest/queue` and `/pull/spot` admit 168,192 bytes. `/pull/run` and `/pull/recovery` admit 27,347,836 (server.rs:16370-16373). So `param`'s k scans cover up to 27 MB on those routes.
- Fix: say "about 27.3 MB (26.1 MiB)", and add the larger form routes to the D-1202 bullet.

### P17-20 low: docs/07 §0 and §7.2 cite lines and a receipt string that no longer exist
- §0 says `.gitignore:30`. The rule is at `.gitignore:46-47`.
- §7.2 cites `core/src/error.rs:91`; the line is now :113.
- §7.2 cites `ingest.rs:442`/576/577 and `244–246`; the code is now at :655 and `balances` at :296.
- §7.2 cites `audit.rs:588`; the code is now at :713.
- Mechanism 4, "a receipt line reading 'Members failed: 596' (`api/src/server.rs:3717`, 3733)", no longer exists. The receipt says "Failure diagnostics" (server.rs:10625; see P17-01).
- §7.1 itself argues that line numbers in a plan go stale and should be dropped.
- Fix: replace the line numbers with item names, and update mechanism 4.

### P17-21 low: docs/07 NEXT #2 names a blocker that is gone
- docs/07:169 says "Needs an entitlement signal that is not the census". That signal exists: readiness is `SourceKind::needs_credential` for brokers and `archive_ready` (server.rs:8796) for archives, with no census read (:1874-1931).
- What remains is enforcement. `parse_feed` (ingest.rs:1588) still accepts any feed, and `archive_ready` has one caller, the `/feeds.json` path. So R-10's "Advisory only" is still true.
- Fix: restate #2 as "enforce the existing readiness predicate on the POST routes".

## Table A: docs/06 statements checked (64)

Status key: TRUE means the limit still holds. STALE means the code fixed it and the doc understates. PARTIAL means it is partly stale. REGRESSED means the cost is worse than stated. UNBACKED means a measurement is claimed with no evidence. WRONG means the arithmetic is wrong.

| # | Statement | Status | Evidence |
|---|---|---|---|
| 1 | §1 per-op costs gated by gate 8 | TRUE | 13 `crates/*/benches/ratio.rs` |
| 2 | §2 500/min primary, 5/s secondary | TRUE (omits Zerodha 3/s) | rate.rs:297,348,362 |
| 3 | §2 pull resumes from committed work | TRUE | autopilot.rs:22 |
| 4 | §3 generated loader not committed | STALE | P17-09 |
| 5 | §3 zero hand-written non-Rust enforced | STALE | P17-09 |
| 6 | §4 0.192/1.28/0.257 ns table | UNBACKED | P17-10 |
| 7 | §4 7.4 GB, exit 137 | TRUE (backed) | 05-decisions:28062 |
| 8 | §5 328 live positions | TRUE | table.rs:1983-1986 |
| 9 | §5 Itemset width asserted at compile time | TRUE | engine lib.rs:479 |
| 10 | §5 C(328,k) and MiB/GiB columns k=2..4 | TRUE | recomputed |
| 11 | §5 k=4 is 3.53× of 2^27 | TRUE | lib.rs:835 |
| 12 | §5 allocator bound is try_reserve on the result vector | TRUE | lib.rs:2077 |
| 13 | §5 extinct at depth 23, 34,979,095 rows, 5.0 GB | TRUE (backed) | 05-decisions:13337 |
| 14 | §5 stored doors retain through and_checkpoint | TRUE | cli lib.rs:3665 |
| 15 | §5 8.91 GB / 6.35 GB | TRUE (backed) | 05-decisions:28065 |
| 16 | §5 "per §3.6" | dangling ref (no §3.6; means CLAUDE.md §3 rule 6) | docs/06:197 |
| 17 | §7 toolchain 1.97.1, branch coverage unmeasurable | TRUE | rust-toolchain.toml |
| 18 | §7 coverage gate 90/89 | TRUE | ci.yml:7364 |
| 19 | §14 per-block constant (C-07) | TRUE | store bench |
| 20 | §14 no bar reader exists | STALE | P17-11 |
| 21 | §14 x86_64 unmeasured | STALE | P17-11 |
| 22 | §17 census and lookup benches exist | TRUE | pull/benches/ratio.rs:267,388 |
| 23 | §17 loaded index reserved from census | TRUE | pull/tests/unit.rs:1835 |
| 24 | §17 no reconciliation entry point | TRUE | manifest.rs:132 |
| 25 | §23 reservation min(2n, MAX) | TRUE | manifest.rs:404 |
| 26 | §23 MAX_ENTRIES 2,097,152 | TRUE | manifest.rs:272 |
| 27 | §28 CI has no cargo-mutants step | STALE | P17-12 |
| 28 | §28 format.rs "448" defect | TRUE (unfixed) | P17-13 |
| 29 | §28 catalog.rs "C-11" defect | TRUE (unfixed) | P17-13 |
| 30 | §30 no type-ahead possible | STALE | P17-09 |
| 31 | §31 arrows do not cross a year | TRUE (reason stale) | calendar.rs:868 |
| 32 | §32 held_series not on a request path | REGRESSED | P17-16 |
| 33 | gate-11 rule-4: held_series startup only | REGRESSED | P17-16 |
| 34 | §35 no journal record for an unfinished run | TRUE | audit journal at run end |
| 35 | §36 reasons destroyed before any page sees them | PARTIAL | broker refusals go whole to the rolling log (server.rs:10165) |
| 36 | §43 15 lake survivors | STALE | P17-14 |
| 37 | §46 p99 flatness row C-T-01b exists | TRUE | telemetry bench:91,139 |
| 38 | §46 roll ≤ 2×keep_files | TRUE | sink.rs:45 |
| 39 | §49 halt needs a restart | PARTIAL | P17-18 |
| 40 | §51 ITERATION_CEILING = 130 | TRUE | vwap.rs:221-236 |
| 41 | §51 equities reach isqrt (VWAP Present) | TRUE | cli/stored.rs:2063 |
| 42 | §53 E-08 unmeasured | TRUE | docs/04:186 |
| 43 | §53 C-03/C-04 need a rerun | STALE | P17-08 |
| 44 | §66 run_local uses Columns::Gdfl | STALE | P17-17 |
| 45 | §82 W1/W2/W3 gates exist | TRUE | ci.yml:7806-7828 |
| 46 | §82 two front ends, twofrontends test | TRUE (172→180 lines, render.rs:1130→1168) | web/tests |
| 47 | §85 mutation floor is five | STALE | P17-15 |
| 48 | §91 `cli elite`, ScreenCache, descent test | TRUE | lib.rs:2256,16212; audited_stored_tests:2321 |
| 49 | §91 cli ratio bench exists | TRUE | crates/cli/benches/ratio.rs |
| 50 | §93 whole_machine_ceiling/ceiling_from_env; derived_ceiling removed | TRUE | lib.rs:17089,17123 |
| 51 | §94 REFERENCE_CORES 14 | TRUE | lib.rs:17042 |
| 52 | §171 pool dd is a lower bound, not persisted | TRUE | pool.rs:38-41,157 |
| 53 | D-0914 cold read C-BC-01/02, 10,000-floor budget | TRUE | store bench:34,394 |
| 54 | D-0914 cold verify allocates nothing (C-BC-03) | TRUE | file.rs:2050 `vec!` is the write-side seal |
| 55 | D-0994 timing labelled UNVERIFIED | TRUE (labelled) | n/a |
| 56 | D-1434 bisection probe test exists | TRUE | file.rs:4481 |
| 57 | D-1434 bounds 14/16/22 reads | TRUE | recomputed |
| 58 | D-1202 caps 8,192/65,536, hyper 1.11.0 | TRUE | server.rs:16234,16251; Cargo.lock |
| 59 | D-1202 every form ≤ 8,192 | REGRESSED | P17-19 |
| 60 | D-1769 about 26.5 MB | WRONG | P17-19 |
| 61 | D-1769 live spans floor test exists | TRUE | pull/tests/unit.rs:2878 |
| 62 | o1engine-23 incremental grammar node | TRUE | expression_search.rs:208,238 |
| 63 | o1engine-22 tier slots 8/64/576, ≤4.5·len+8 | TRUE | expression.rs:22-28,57 |
| 64 | D-1182 cadence closed, inverted test exists | TRUE | runner/trade.rs:1965 |

Tally: TRUE 41 (including 3 backed by decision entries and 2 still-unfixed defects). STALE 14. PARTIAL 2. REGRESSED 3. UNBACKED 1. WRONG 1. Dangling reference 1. Doc-mapped fixed-vs-stated: FIXED-in-code but not recorded (STALE) 14.

## Table B: docs/07 items checked (50)

| # | Item | Status | Evidence |
|---|---|---|---|
| 1 | §0 web/build committed, read at run time | TRUE | assets.rs |
| 2 | §0 launch.json tracked, two configs | TRUE (line ref stale) | P17-20 |
| 3 | §0 W1 ties build to src | TRUE | ci.yml:7806 |
| 4 | R-1 catalog::tracked shared | TRUE | catalog.rs:325 |
| 5 | R-2 never today | STALE | P17-06 |
| 6 | R-3 Groww day word open | STALE | P17-04 |
| 7 | R-4 /pull/fno routed | TRUE | server.rs:16576 |
| 8 | R-9 thirteen ratio benches | TRUE | 13 files |
| 9 | R-10 advisory only | TRUE | P17-21 |
| 10 | DONE: split_window takes Option | TRUE | session.rs:1218 |
| 11 | DONE: Outcome::Empty code 4 | TRUE | audit.rs:364 |
| 12 | DONE: keeps does not allocate | TRUE | census.rs:943 |
| 13 | DONE: filtered returns Cow | TRUE | census.rs:1017 |
| 14 | DONE: totp exists | TRUE | pull/src/totp.rs |
| 15 | DONE: page built from Feed::ALL; parse_feed walks Feed::ALL | TRUE | ingest.rs:1588 |
| 16 | DONE: census positional append | TRUE | docs/06 D-1446 corrections, `append_locked` |
| 17 | BLOCKED: TrueData SymbolAtRoot | TRUE (blocker exists) | vendor.rs:4996 |
| 18 | BLOCKED: TOTP mint (never mints) | TRUE | CLAUDE.md §8 |
| 19 | BLOCKED: GDFL licence; nothing built | TRUE | no GFDLCM reader in crates/ |
| 20 | BLOCKED: misfiled months / Dhan token | operator-side; cannot be checked here | n/a |
| 21 | NEXT #2 entitlement signal missing | PARTIAL/STALE | P17-21 |
| 22 | NEXT #3 done (D-0064) | TRUE | assets.rs |
| 23 | NEXT #6 GDFL nothing built | TRUE | grep |
| 24 | §4 Groww day "none published" | STALE | P17-05 |
| 25 | §5 three descriptor fields closed | TRUE | vendor.rs:1593,1861,2025 |
| 26 | §6 gate 8 3.0× ceiling, exit(1) | TRUE | engine bench:62,244 |
| 27 | §6 hits source-shape test | TRUE | mask.rs:437 |
| 28 | §6 duplicate rejection = C-E-04 | STALE | P17-08 |
| 29 | §6 result append = C-E-11 | TRUE | docs/04:199; engine lib.rs:149 |
| 30 | §6 E-08 unmeasured | TRUE | docs/04:186 |
| 31 | §7.1 run_local literals remain | TRUE | server.rs:10978-10982 |
| 32 | §7.2 symbol byte set | TRUE | symbol.rs:72-76 |
| 33 | §7.2 error wording | TRUE (line stale) | error.rs:113 |
| 34 | §7.2 five loud mechanisms | PARTIAL | P17-20 |
| 35 | §7.2 no zip reader | TRUE | grep |
| 36 | §7.2 1-min archive needs seconds | TRUE | csv.rs:242 |
| 37 | §7.3 PriceScale::Rupees on archives | TRUE | vendor.rs:5027,5110 |
| 38 | §7.3 RecordShape read only by tests/const | TRUE | vendor.rs:7762-7765 |
| 39 | §9.1 mechanisms (suffix_that_follows, CensusLock) | TRUE | file.rs:2888; ingest.rs:2990 |
| 40 | §9.3 no calendar or holidays | STALE | P17-07 |
| 41 | §9.3 375 bars (CAS) | WRONG | P17-07 |
| 42 | §10 row 11 re-read | TRUE | server.rs:8059 |
| 43 | §10 rows 12/13 retry | STALE | P17-03 |
| 44 | §10 row 14 isolation | STALE | P17-03 |
| 45 | §10 row 15 progress | STALE | P17-03 |
| 46 | §10 row 19 reconcile not built | TRUE | §17 |
| 47 | §10 "#11 hard stop today" | STALE | P17-03 |
| 48 | §11 ledger-v6 dispatched | TRUE | cli lib.rs:2289-2292 |
| 49 | OPEN D-0753 | STALE | P17-02 |
| 50 | cash checkpoint: calendar ends 2026-08-21 | PARTIAL (now 2026-09-04, calendar.rs:114) | n/a |

Blocked-by rows: of the five §3 rows, three blockers still exist (TrueData, TOTP/tickvault, GDFL licence). Two are operator-side and cannot be checked here. The one NEXT row whose stated blocker is gone is #2 (P17-21).
