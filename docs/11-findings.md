# 11 — Findings ledger

Every finding the 2026-08-11 adversarial sweep produced, with what has been done
about it. **Append only, and no row is ever deleted** — a finding that turns out to be
wrong is marked `REFUTED` with the reason, because a ledger that drops its mistakes
cannot be used to check whether anything was missed.

## How to read it

Ten adversarial lenses initially raised **114** findings. Each lens's output went to a
second agent instructed to refute it and to default to refuting, because a false finding
costs a real change to working code. **26 were killed. With later audits appended, 111
stood in this ledger.** Of those, **37 can lose a real
combination or invent a false one**, which is the only ranking that matters for a
brute-force search.

`ID` is `F-` plus a hash of the finding's own title, so a row cannot be renumbered by
reordering the table and a citation cannot silently come to mean something else — the
failure D-0104 records for D-0076, D-0077 and D-0078.

**`COSTS` means the finding can drop a real combination or invent a false one.**

### Severity

| | |
|---|---|
| `law` | breaks a written rule in `CLAUDE.md` or `docs/00-charter.md` outright |
| `wrong` | produces an incorrect answer on an input that can occur |
| `unguarded` | correct today, with nothing that would fail the build if it stopped being |
| `gap` | something the design cannot express |

### Disposition

| | |
|---|---|
| `FIXED <sha>` | fixed, with a test proved to fail against the pre-fix code |
| `IN PROGRESS` | being worked on now |
| `NEEDS A DECISION` | blocked on a human: a source that does not exist, or a scope call |
| `OPEN` | confirmed, not started, not blocked |
| `REFUTED` | later shown wrong; the reason is recorded and the row stays |

---

## The 37 that can cost a combination

| ID | Sev | Finding | Where | Disposition |
|---|---|---|---|---|
| `F-62400B` | `law` | An afternoon Muhurat session IS a session to the evaluator: its one-hour OHLC becomes the previous-day anchor, enters Prev5, and becomes the gap family's previous-session edge — the three things charter §3… | `crates/indicators/src/evaluator.rs:212-219 (the session boundary is `ist_day` inequality and nothing else), :280-298…` | FIXED f170f08 — `Calendar`, defaulting to the charter's six VERIFIED dates |
| `F-E63A74` | `wrong` | A bar the Evaluator REFUSES has already been folded by six stateful modules, so it changes the next bar's mask — while D-0097 records that invariant as restored and quotes the very arm that tears it | `crates/indicators/src/evaluator.rs:222-247 (six `?` folds before vwap) and evaluator.rs:249-255 (the…` | FIXED 646c6a8 — same fix; this is the same defect from a second lens |
| `F-081A83` | `wrong` | A refusal raised after seven modules and the session rollover have already folded the bar leaves the evaluator half-updated, so the next bar's bits are computed from a bar the caller was told was refused | `/Users/parthi/IdeaProjects/brutex/crates/indicators/src/evaluator.rs:213-219 (the rollover, which the finder omitted)…` | FIXED 646c6a8 — commit-on-success; `stepped` takes self by value |
| `F-122234` | `wrong` | Below a 9-paisa anchor the Fibonacci rung LEVELS collapse onto one paisa and six positions fire on one bar — falsifying a const assertion, D-0076, and two named tests that all state the property unconditionally | `crates/vocab/src/tolerance.rs:190-199 (the const bound); crates/indicators/src/lib.rs:453-465 (CurDayFib::rung_level)…` | OPEN |
| `F-8CFA47` | `wrong` | Clamped pivot levels collapse R4 onto R5 and fabricate six band bits, and one crate holds two opposite policies for the same overflow | `crates/indicators/src/daily.rs:339-354 (clamp_i64), :146-197 (from_previous_session), :439-448 (the R4/R5/S4/S5 plan…` | FIXED 7ab6135 — `Err(Unusable::LevelOverflows)` instead of clamping |
| `F-9D146E` | `wrong` | Positions 19 and 25 are tombstoned as duplicates of 17 and 18, and the two pairs are banded by different constants over different bases — the retired predicates were up to 16.67x tighter, and are the opposite… | `crates/vocab/src/table.rs:187 and :193 (the tombstones); crates/indicators/src/fib.rs:44-47 (the premise);…` | NEEDS A DECISION — §3.8 forbids un-retiring; needs two appends |
| `F-0C28E4` | `wrong` | Support's denominator includes bars on which whole families are structurally unable to answer, so a genuinely frequent combination is pushed under min_hits and dies at k=1 | `crates/engine/src/lib.rs:172-175 (`Sweep.bars` is every loaded mask) and 423-428 (`support` excludes nothing);…` | PARTLY FIXED 13737f0 — the boundary is published (`warmed_up`); no caller consumes it yet, 06-limits §59 |
| `F-A022F6` | `wrong` | The 200-period EMA and the SuperTrend emit on the SECOND candle of a run: bit 2 close_above_ema200 is 'close above the previous close' | `crates/indicators/src/trend.rs:170-176 (Ema::fold seeds scaled = target), :191-196 (value() returns Some once seeded),…` | FIXED 7ab6135 — emission gated on a folded counter, not the accessors |
| `F-777BEE` | `wrong` | The 5-bar fractal swing window straddles the overnight gap: it both WIDENS the near_swing band 67x and, on a gap the other way, SUPPRESSES a swing the session's own bars would have confirmed | `crates/indicators/src/trend.rs:660-679 (`TrendState::step`/`fold` -- no IST-day check anywhere in the module),…` | OPEN |
| `F-5872CE` | `wrong` | The pivot band collapses whenever the previous session's close sits near the midpoint of its range, and the twelve banded above/below positions silently degrade into bare comparisons | `crates/indicators/src/daily.rs:146-152 (`from_previous_session` refuses only `high < low` and a span that leaves i64),…` | OPEN |
| `F-17D703` | `wrong` | TrendState::bits takes &mut self and is not idempotent, and one stray call permanently changes the mask the sweep later gets | `crates/indicators/src/trend.rs:688 (pub fn bits(&mut self, ...)), :723-726 (observe runs inside the emit), :599-606…` | FIXED 934f72a — `classify`/`advance` split, `bits(&self)` |
| `F-9A880B` | `wrong` | VWAP sigma is dominated by a floor-division artefact — 912 paisa reported where the true dispersion is a sixth of a paisa — but it cannot bite any run this engine sweeps | `crates/indicators/src/vwap.rs:326-340 (mean and mean_sq each floored before the square)` | OPEN |
| `F-31B8FA` | `wrong` | `prior_n_bearish` (38) fires on three UNCHANGED bars: a bit that reads as a measurement and is an artefact of a two-valued answer to a three-valued question | `crates/indicators/src/session.rs:226 (`*newest = Some(bar.close > bar.open)`), :296-298 (bit 38 is…` | FIXED 7ab6135 — an `Option<Ordering>` ring; a flat bar is neither direction |
| `F-5FB5C9` | `wrong` | fib::emit decides the rung with an exact test and hands vocab a truncated level, so the emitted band is the INTERSECTION of two disagreeing predicates — one closing price narrower than either | `crates/indicators/src/fib.rs:125-146 (the exact `near`) and fib.rs:165-181 (the truncated level handed to set_near).…` | OPEN |
| `F-F52950` | `unguarded` | A previous session whose S4 and S5 both clamp onto `i64::MIN` makes two vocabulary positions one predicate — and puts §7's open-interest sentinel where a price goes, which another module in this crate refuses… | `crates/indicators/src/daily.rs:188-194 (the S-ladder), :339-355 (`clamp_i64` saturating to `i64::MIN`), :521-522;…` | FIXED 7ab6135 — same refusal; no rung saturates onto the sentinel |
| `F-87AB98` | `unguarded` | An unrepresentable cross-bar true range freezes the SuperTrend latch while positions 64/65 keep being emitted from the stale stop | `crates/indicators/src/trend.rs:222-232 (cross-bar true range widened to i128 and then narrowed by…` | OPEN |
| `F-72F226` | `unguarded` | Four band bases are computed with saturating_sub, so at the i64 extremes the band is silently up to 2x too narrow instead of refused | `crates/indicators/src/orb.rs:264; crates/indicators/src/fib.rs:336 (five-session span);…` | FIXED c2c6f04 — `checked_sub` at all four sites, including trend.rs |
| `F-777A8F` | `unguarded` | One pre-open print inverts gap_up_day into gap_down_day, and corrupts tomorrow's whole pivot, PDH/PDL and five-session ladder — not just today's day_open | `crates/indicators/src/session.rs:186-200 (the fold keys on ist_day only), session.rs:365-370 (48/49),…` | OPEN |
| `F-E94FAD` | `gap` | A run's first ~5 sessions carry bits that depend on how much history preceded the window, and the bound is recorded nowhere | `crates/indicators/src/evaluator.rs:174-277; crates/indicators/src/trend.rs:163-196;…` | PARTLY FIXED 13737f0 — `sessions_until_every_family_can_answer` makes the bound readable |
| `F-D8A934` | `gap` | ATR is computed on every bar and never becomes a bit, so no combination can say 'this bar is large for this market right now' | `crates/indicators/src/trend.rs:200-266 (Atr, Wilder-smoothed, held scaled); its only read is SuperTrend::fold at…` | OPEN |
| `F-B1088A` | `gap` | Cross-instrument conditions are absent from crates/indicators, and closing the gap needs a run-identity decision before any code | `crates/indicators/src/evaluator.rs:179 (step takes ONE &Candle); CLAUDE.md §3 rule 3 (a singular instrument and a…` | NEEDS A DECISION — needs a run-identity decision first |
| `F-D6094E` | `gap` | NOT is a real gap: zero of the 234 live positions has a single-bit complement, 61 have a disjunctive complement that is exact only while its preconditions hold, and 173 have none at all | `crates/vocab/src/table.rs:161-827 (no `not_*`, `no_*`, `away_*`, `off_*`, `absent_*` or `never_*` name among the live…` | NEEDS A DECISION — 234 complement positions against 108 free |
| `F-FD106B` | `gap` | No position compares the close to TODAY'S OPEN, and Evaluator.running_open is written and never read anywhere in the repository | `crates/indicators/src/evaluator.rs:115 (declaration), 154 (initialiser), 268 (assignment) — and no read;…` | OPEN |
| `F-FD58AE` | `gap` | No position compares the close to the previous day's CLOSE — the level every percent-change quote is measured against — and PreviousSession.close is already in the struct | `crates/vocab/src/table.rs:179-184 (13-18 are PDH/PDL only, no close); crates/indicators/src/session.rs:121…` | OPEN |
| `F-3B7092` | `gap` | No position in the 234-bit vocabulary can name a session's shape, so the 2025 afternoon Muhurat is labelled exactly like a regular NIFTY afternoon and the sweep cannot isolate or exclude it | `crates/indicators/src/session.rs:329-348 (44-47 from `minutes_since_open`), :34-54 (`DayWindows`, the four windows and…` | OPEN — the anchor half is fixed by f170f08; naming a session's SHAPE still needs a position |
| `F-CD7D0E` | `gap` | No round-number or strike-grid position exists, and the blocker on adding one is golden rule 1, not the instrument's grid | `crates/vocab/src/table.rs:161-556 — not one of the 276 positions is a function of the close modulo anything` | NEEDS A DECISION — §3.1 blocks inventing a grid |
| `F-344A30` | `gap` | Nothing reconciles the caller's `live` list against the bits actually present in `bar_bits`, and no production caller of `walk` exists — so the sweep's coverage of the vocabulary is whatever a future caller… | `/Users/parthi/IdeaProjects/brutex/crates/engine/src/lib.rs:260 (`pub fn walk(self, bar_bits: &[ConditionMask], live:…` | OPEN |
| `F-8E15DC` | `gap` | Positions 19 and 25 were retired on a predicate identity that D-0079's two-width split falsified, and the fib-width band on yesterday's high and low is now expressible by no live position | `crates/vocab/src/table.rs:36-40 (the table claims "Why they are the same predicate") and :187,:193;…` | NEEDS A DECISION — same as above |
| `F-F9C23A` | `gap` | Structure.last is the in-force market-structure direction and is never published, so bits 56-59 are events on a handful of bars with no regime behind them | `crates/indicators/src/trend.rs:531-535 (Structure { last: Option<Trend> }, private); trend.rs:726-739 (56-59 set only…` | OPEN |
| `F-506ED7` | `gap` | Support is counted over bars where the emitting family could not answer, and no document records the consequence for `min_hits` | `/Users/parthi/IdeaProjects/brutex/crates/engine/src/lib.rs:303 (support over the whole column) · :316 (`else if hits…` | PARTLY FIXED 13737f0 — recorded in 06-limits §59 with the measurement |
| `F-F612A5` | `gap` | The D-0080 always-true exclusion is justified by a property of the WINDOW, and its recorded justification describes 39 positions the guard can never see | `crates/engine/src/lib.rs:310-315 (the guard), :72-75 (Why::AlwaysTrue), :291-298 (the NotLive check that runs first),…` | OPEN |
| `F-63A782` | `gap` | The `params` term is both unsettable and unreadable: four UNVERIFIED threshold sets are hardcoded inside Evaluator::new and nothing can report which a run used | `crates/indicators/src/evaluator.rs:145-169 and :231; crates/indicators/src/daily.rs:473-475;…` | NEEDS A DECISION — part of run identity, which needs blake3 |
| `F-F7DD09` | `gap` | The always-true exclusion drops the conjunct a caller needs to reproduce the run: the reported mask omits a filter that was in force, and only the loaded-window support argument justifies it | `/Users/parthi/IdeaProjects/brutex/crates/engine/src/lib.rs:310-315 (`} else if hits == bars { … Why::AlwaysTrue }`) ·…` | OPEN |
| `F-67A868` | `gap` | The live set a sweep was offered is not recoverable from its output, and vocab_version 3 cannot tell a 232-live vocabulary from a 234-live one | `crates/vocab/src/lib.rs:70-100; crates/vocab/src/table.rs:865-874 (LIVE), :1004-1016 (popcount 234);…` | NEEDS A DECISION — part of run identity |
| `F-3E4B8C` | `gap` | Time of day is four coarse buckets: no first five minutes, no last five minutes, no day of week — and the two arithmetic ones are free | `crates/indicators/src/session.rs:64-69 (DayWindows::CLASSICAL 60/165/285/375), session.rs:329-347;…` | OPEN |
| `F-35640C` | `gap` | Two of the four Fibonacci ladders have no side-of-rung information of any kind: the prev-5 and gap ladders carry Kind::Near only and no anchor above/below position exists for either | `crates/vocab/src/table.rs — every near_fib_* row is Kind::Near: 188-197 (20-29), 249-250 (69-70), 304-307 (106-109),…` | OPEN |
| `F-8F6ED2` | `gap` | min_hits = 1 — the constructor's floor, the value 0 becomes, and the value the gate-8 bench uses — cannot terminate in bounded memory at vocabulary width, and with_min_hits's own comment claims the opposite | `crates/engine/src/lib.rs:209-227 (the floor and its comment), :343-356 (every level retained in sweep.levels), :367…` | OPEN |

---

## The other 74

| ID | Sev | Finding | Where | Disposition |
|---|---|---|---|---|
| `F-8EC62F` | `wrong` | A duplicate position in the engine's `live` list is reported as a candidate "evaluated against the bars and found too rare", and reconciles() blesses it | `crates/engine/src/lib.rs:298-301 (silent dedup, no counter); lib.rs:319-332 (infrequent by subtraction, duplicates…` | FIXED 575c330 — duplicates counted at their own site; `infrequent` no longer a residual |
| `F-429E2A` | `wrong` | A repeated non-live position is named once per occurrence, so one excluded position appears twice in the run output and the exclusion count disagrees with the number of excluded positions | `/Users/parthi/IdeaProjects/brutex/crates/engine/src/lib.rs:290-298 (the `NotLive` push happens BEFORE `offered.insert`…` | FIXED 575c330 — the dedup probe runs BEFORE the liveness check, so one position is named once |
| `F-90CEB4` | `wrong` | A repeated position in `live` is deduplicated silently and absorbed into `infrequent`, and a repeated tombstone is named once per occurrence — two buckets wrong in opposite directions with reconciles() still… | `crates/engine/src/lib.rs:299-301 (silent dedup), :291-298 (NotLive pushed per occurrence, before the dedup probe),…` | FIXED 575c330 — both buckets counted at their own sites, in both directions |
| `F-2C2964` | `wrong` | At k=1 a duplicated position is counted as `infrequent`, `duplicates` is hardcoded to 0, and `infrequent` is derived by subtraction so the five-bucket reconciliation cannot falsify it | `/Users/parthi/IdeaProjects/brutex/crates/engine/src/lib.rs:299-301 (`if !offered.insert(p) { continue; }` — counted…` | PARTLY FIXED 575c330 — the hardcoded 0 and the subtraction are gone; the k=1 reconciliation still holds by construction |
| `F-C7BAD0` | `wrong` | Fourteen pivot `Near` bits and twenty `Above`/`Below` bits share one band width, and the ten levels are a proven total order, so ONE fact emits 512 itemsets at an identical hit count — a redundancy class… | `crates/indicators/src/daily.rs:164-173 (the recurrence), :510 and :516 (the one shared `half`), :521-522…` | OPEN |
| `F-D8F140` | `wrong` | Support counts bars, and seven daily families are constant for all 375 bars of a session, so min_hits=30 is satisfied by one twelfth of one session out of 1,630 — and `Itemset` carries nothing that can tell… | `crates/engine/src/lib.rs:93-98 (`Itemset` has exactly `mask` and `hits`), :260 (`walk` carries no time), :423-428…` | OPEN |
| `F-2E05B5` | `wrong` | The band on all 14 pivot `near_*` bits is half the CPR width, so their own width is set by where yesterday closed inside its range — 0 to range/6 — which manufactures a definitional dependence with the three… | `crates/indicators/src/daily.rs:510 and :516-517 (one `half` for all 14), :251-285 (cpr_width = &#124;2C-H-L&#124;/3), :293-309…` | OPEN |
| `F-ADF0D5` | `wrong` | The engine reports duplicate `live` positions as `infrequent`, and `reconciles()` cannot see it because `infrequent` is a residual | `crates/engine/src/lib.rs:299-301 (the duplicate `continue`), :321-324 (generated/infrequent at k=1), :325-326 (the…` | FIXED 575c330 — the residual is gone; `infrequent` is incremented where it is decided |
| `F-F66B8F` | `wrong` | The engine's own anti-loss guard is satisfied by mis-filing: a duplicated live position is reported as `infrequent`, and a duplicated dead position produces duplicate exclusion rows | `crates/engine/src/lib.rs:321-335 (generated counts the raw list, duplicates hardcoded 0, infrequent by subtraction),…` | FIXED 575c330 — neither mis-filing remains: a duplicate is a duplicate, an exclusion is named once |
| `F-012171` | `wrong` | `Excluded.support` is documented as measured and carries a fabricated 0 for every NotLive refusal, including positions no bar can represent | `crates/engine/src/lib.rs:292-296 (the push), :85-86 (the doc)` | FIXED 575c330 — `Excluded.support` is `Option<u64>`, `None` for every `NotLive` |
| `F-61FB62` | `wrong` | docs/06-limits.md §5 states that the memory bound surfaces as a loud resource refusal, and no refusal exists anywhere: `walk` returns `Sweep` not `Result`, retains every level for the whole run, and has no… | `docs/06-limits.md:90-93 (the claim) and :82 (32 bytes); crates/engine/src/lib.rs:260 (returns `Sweep`), :353 (every…` | PARTLY FIXED 6b02545 — §5 now says there is NO refusal, with real numbers; the refusal itself is still unwritten |
| `F-0491E4` | `wrong` | k=1 reports positions as `infrequent` that were never measured, `duplicates` is hardcoded to 0, and `reconciles()` is arithmetically incapable of failing at k=1 | `crates/engine/src/lib.rs:321-335 (infrequent by subtraction, `duplicates: 0`), :325-326 (the false comment), :299-301…` | PARTLY FIXED 575c330 — nothing is reported unmeasured now; `reconciles()` is still unfalsifiable at k=1 |
| `F-918CA8` | `wrong` | pat_rising_three_methods (176) / pat_falling_three_methods (177) omit both clauses their own vocabulary row states, and are symmetric in the three middle bars | `crates/indicators/src/pattern.rs:712-721 (176) and :722-731 (177). The claim they violate is the row comment at…` | OPEN |
| `F-E5BD44` | `wrong` | prior_n_bearish (38) and prior_alternating (39) fire off a direction ring that files a FLAT bar as bearish, contradicting the same module's own tested claim | `crates/indicators/src/session.rs:226 (`*newest = Some(bar.close > bar.open);`) against :254-259 (bits 30/31 use strict…` | OPEN |
| `F-BD8E69` | `unguarded` | A bar refused AFTER the rollover has already closed the books: `has_yesterday` flips true and a session is counted by a record nobody accepted, and `last_ts` is left behind so `self.day` can then walk backwards | `crates/indicators/src/evaluator.rs:212-219 (rollover + close_the_books) runs before :223-255 (nine fallible module…` | OPEN |
| `F-82E7D1` | `unguarded` | C-I-04 is one-sided and its recorded measurement is already 23% on the cheap side (0.772x), so the row cannot fail in the direction it drifted | `crates/indicators/benches/ratio.rs:437-464 — calls `ratio` at :459 while `two_sided_ratio` sits at :130 in the same…` | OPEN |
| `F-E3B6A6` | `unguarded` | DailyLevels::from_daily_bar accepts the record Candle::check refuses, and an uncontained close makes twelve to seventeen pivot positions fire together — near_pdh and near_pdl on the same bar | `crates/indicators/src/daily.rs:146-196 (from_previous_session checks high<low and range overflow, never…` | FIXED 010f682 — `Unusable::CloseOutsideRange`, the refusal `Candle::check` already made |
| `F-33EF83` | `unguarded` | Gate 18 is RED on this branch: cargo-mutants 26.2.0 finds 5 missed mutants and 1 infinite-loop timeout in crates/engine/src/lib.rs, and every line of that file is inside the gate's --in-diff scope | `crates/engine/src/lib.rs:154 (reconciles), :236 (min_hits), :263 (Sweep.bars), :343 (the while condition), :444…` | OPEN |
| `F-5D1805` | `unguarded` | The five-bucket reconciliation is a tautology at k=1, and cargo-mutants shows the predicate is unpinned at EVERY k | `crates/engine/src/lib.rs:322-324 (infrequent as residual), :146-161 (reconciles), :935-984 (the only test)` | PARTLY FIXED 575c330 — each bucket is counted rather than derived, so a sixth outcome would fail it; the k=1 identity still holds by construction and cargo-mutants was not re-run |
| `F-EBD1B5` | `unguarded` | The pivot ladder is a 13-deep implication chain and the D-0080 exclusion guard — the mechanism built for exactly this pathology — provably cannot see it, so one fact is reported as 4,096 support-identical… | `crates/indicators/src/daily.rs:510-522 (one shared `half` for every level) and 164-173 (the monotone ladder);…` | OPEN |
| `F-18C058` | `unguarded` | TrendThresholds periods are public, unvalidated i128; a degenerate one freezes or overshoots the average while still reporting it as a measurement | `crates/indicators/src/trend.rs:76-84 (public fields); trend.rs:181-187 (Ema::fold returns when denominator <= 0);…` | OPEN |
| `F-62285F` | `unguarded` | V-02 and V-03, the crate's two order-safety proofs, drive 3 of the 9 position sources -- and the invariant rows that name them point at modules that do not exist while gate 10 matches on the bare function name | `crates/indicators/tests/invariants.rs:63-77 (`bits_over` constructs only CurDayFib, Patterns, SessionState), :91 (V-02…` | OPEN |
| `F-B3B34C` | `unguarded` | `a_threshold_above_the_bar_count_finds_nothing_and_says_so` never reaches the threshold comparison — min_hits could be ignored at k=1 and it would pass | `crates/engine/src/lib.rs:905-911 (the test), against the D-0080 exclusion arms at :302-318` | OPEN |
| `F-A9DB2D` | `unguarded` | bos_bullish/bos_bearish (56/57) are latched STATE bits that fire on every bar past the swing while choch_* (58/59) fire once, and Structure::last never resets -- so one structural break contributes an… | `crates/indicators/src/trend.rs:563-607 (`Structure::observe` recomputes `broke_up = close > swing.price` at :569 on…` | OPEN |
| `F-873717` | `unguarded` | inside_day (50) / outside_day (51) are RUNNING bits, so 50 is set on 300 of 375 bars of a day that ended as an outside day | `crates/indicators/src/session.rs:376-381 -- bits 50/51 compare `dh`/`dl`, the RUNNING extremes seeded at :199-201 and…` | OPEN |
| `F-9A6BC3` | `unguarded` | sigma() answers Some(0) with two contributing bars, aliasing nine band positions — the exact artefact MIN_FOR_SIGMA is documented to prevent | `crates/indicators/src/vwap.rs:317-327 (the MIN_FOR_SIGMA gate counts bars instead of measuring dispersion)` | OPEN |
| `F-8E12FD` | `unguarded` | vocab's bench ratio has no floor and no `at_ps == 0` check, so C-V-01 cannot detect a MISS cheaper than a HIT — the exact failure the row exists for | `crates/vocab/benches/ratio.rs:88-104 (ratio), :128-140 (C-V-01), :20-21 (the claim), against…` | OPEN |
| `F-E37BD2` | `unguarded` | vocab's float scan closes its file list against src/ and tests/ but never opens benches/, and CI gate 11 excludes that directory too — so nothing scans it | `crates/vocab/tests/no_float.rs:32-51 (SOURCES, TEST_SOURCES), :132-167 (the_source_list_matches_the_directory), :5-7…` | OPEN |
| `F-2BAB23` | `gap` | "Multiple strategies" — a disjunction of masks used together — has no representation anywhere in the design and no recorded limit saying so | `/Users/parthi/IdeaProjects/brutex/crates/engine/src/lib.rs:93-98 (`Itemset`) · :165-176 (`Sweep`) ·…` | OPEN |
| `F-4F9FB4` | `gap` | A disjunction whose disjuncts are each below min_hits is not two entries in the result set — it is absent, and no field of Sweep, Frontier or Itemset can hold a union count | `crates/engine/src/lib.rs:316-318 (k=1 keeps a position only at hits >= min_hits), :343 (the walk ends when the…` | OPEN |
| `F-6675BA` | `gap` | A five-long pure-shape implication chain 226=>227=>224=>32=>34 exists and is exact, but the redundancy itself is a recorded deliberate choice — only the engine consequence is new | `crates/indicators/src/pattern.rs:390-416 (224/226/227), pattern.rs:185-206 (Shape ratio helpers), pattern.rs:88-99…` | OPEN |
| `F-E3A26D` | `gap` | A position that fails min_hits at k=1 is named nowhere — and the k=1 `infrequent` count is a residual subtraction, so it silently absorbs duplicated `live` entries and `reconciles()` cannot notice | `crates/engine/src/lib.rs:316-318 (the silent arm), :322-326 (the residual and the false comment), :63-66 (D-0080's…` | OPEN |
| `F-88A45C` | `gap` | An out-of-session bar is refused by two of the nine sources and folded by the other seven, and the crate's feed contract is stated nowhere | `crates/indicators/src/session.rs:196-201 (`day_open` is the first bar of the IST day whatever the clock says) and…` | OPEN |
| `F-17C551` | `gap` | Bits 52/143 and 53/144 are exact duplicate LIVE positions, named in a code comment and in no ledger entry, and both are counted in the 234-live figure | `crates/indicators/src/vwap.rs:451-463 (`set(mask, 52); set(mask, 143);` under one condition); contrast…` | OPEN |
| `F-7236E8` | `gap` | CurDayFib::level and the evaluation path disagree by one paisa for a negative rung: one truncates toward zero, the other floors | `crates/indicators/src/lib.rs:369 (`/ 1000` in the public `level`) against lib.rs:454 (`div_euclid(1000)` in…` | OPEN |
| `F-4CF89A` | `gap` | E-02's 12,288-sweep grid never builds a candidate spanning two words, never reaches k=5, and cannot construct a selective prune above k=3 | `/Users/parthi/IdeaProjects/brutex/crates/engine/src/lib.rs:506-584 · :508 (`const P: u32 = 4`) · :513 (`shapes = [&[],…` | OPEN |
| `F-68918D` | `gap` | E-08 attributes the memory peak to the surviving frontier; the peak object is the per-level `seen` set, which holds precisely the candidates that do NOT survive | `docs/06-limits.md:3220-3222; crates/engine/src/lib.rs:366 (seen), :385 (the probe, before the prune and before…` | OPEN |
| `F-FA3656` | `gap` | Every one of the 234 live positions is price location or candle shape, so no combination at any depth can express volatility, expiry proximity, day of week, or the second swept instrument — and a 10-period… | `crates/vocab/src/table.rs:160-545; crates/indicators/src/trend.rs:199-218 (`Atr`) and :317-325 (its only consumer);…` | OPEN |
| `F-75E033` | `gap` | Itemset records `hits` but not WHICH bars hit, so attaching a forward outcome to a survivor costs a full re-scan per set | `crates/engine/src/lib.rs:93-98 (`Itemset { mask, hits }`), :260 (`walk` takes `&[ConditionMask]` and `&[u32]`, no…` | OPEN |
| `F-ACAFBE` | `gap` | Nothing in the three crates or the docs addresses multiple comparisons, and the arithmetic says no rule with 30 hits can survive correction past ~2.7e7 tested itemsets while docs/06-limits.md §5 contemplates… | `docs/06-limits.md:70-93 (§5 is entirely about memory), docs/10-shared-core.md:161-168 (the metric is delegated),…` | OPEN |
| `F-B485D2` | `gap` | Order across bars is expressible only as (earlier LEVEL, current bar); no bit-valued predicate can lag another, so no multi-step strategy is reachable -- and the boundary is written in zero tracked files | `crates/indicators/src/evaluator.rs:221-255 (one mask per bar, nine sources unioned -- the finder cited 216-250, off by…` | OPEN |
| `F-96475D` | `gap` | Route (a), sequence bits, does not fit: a complete 2-step family over the live vocabulary is 54,756 positions against a free budget of 108 | `crates/vocab/src/mask.rs:18 (WORDS=6), :35 (`size_of == WORDS*8`), :44 (BITS); crates/vocab/src/table.rs:832 (COUNT),…` | OPEN |
| `F-33F508` | `gap` | Route (b), an ordered-tuple engine layer, breaks the hit test, the join, and -- with any window constraint -- the anti-monotonicity guarantee E-02 rests on | `crates/vocab/src/mask.rs:144 (`hits`), crates/engine/src/lib.rs:361-415 (`next_level`), :375 (union), :385…` | OPEN |
| `F-026507` | `gap` | Run identity is computed nowhere, and the exemption that excuses it rests on a reason that is no longer true | `crates/engine/src/lib.rs:164-176, :259-358; crates/vocab/src/lib.rs:56-83; .github/workflows/ci.yml:1613-1617 (gate 10…` | PARTLY FIXED 53b1886 — the false exemption reason is corrected; the identity function is still absent |
| `F-5A2BBE` | `gap` | Ten pivot-band positions are one totally ordered implication chain: the engine reports 1,023 combinations with exactly 10 distinct supports, and docs/06-limits.md §5 names the wrong mechanism for it | `crates/indicators/src/daily.rs:521-522 (Rel::Above/Below, close vs level ± the SAME band_half), daily.rs:426-437 and…` | OPEN |
| `F-3E458E` | `gap` | The module cost table's justification "capacity reserved per level" is true only at k=1; every level from k=2 starts at Vec::new() | `crates/engine/src/lib.rs:39 (the claim), :273 (the only reserve), :367 (`let mut out: Vec<Itemset> = Vec::new();`)` | FIXED d5c4c25 — the row states where reservation happens instead of claiming it per level |
| `F-D593C1` | `gap` | The two tests that carry §3 rule 7 for this crate drive three of nine sources and never the aggregate, the comment justifying that is now stale, and the suffix test's own documentation claims a mutation it… | `crates/indicators/tests/invariants.rs:63-77 (`bits_over` drives `CurDayFib`, `Patterns`, `SessionState` only, each…` | OPEN |
| `F-841F0D` | `gap` | Two of the three assertions in `the_fractal_is_derived_from_the_ring` cannot fail — the build breaks first; and the two const assertions above are one claim written twice | `crates/indicators/src/trend.rs:1320-1328 (the test), :137-142 (the const assertions)` | OPEN |
| `F-F2FC17` | `gap` | V-02 and V-03 are proved over 3 of the 9 position sources, and Evaluator::step — where the rollover and the `yesterday` install live — is never prefix- or suffix-tested | `crates/indicators/tests/invariants.rs:63-77 (bits_over drives CurDayFib, Patterns, SessionState only), :91-105…` | OPEN |
| `F-C8BDE2` | `gap` | V-02's cited mechanism is not a mechanism: PastPrefix, the index-guarded accessor §3 rule 7 names, has no caller anywhere in the workspace | `crates/indicators/src/lib.rs:92-132 (PastPrefix), :25-28 (the claim), docs/04-invariants.md:87, CLAUDE.md §3 rule 7 —…` | OPEN |
| `F-CEC7A0` | `gap` | V-04 ('time-of-day and VWAP bits are cleared on a daily timeframe') is unprovable in this crate and unproven by the test named for it | `docs/04-invariants.md:89; the named test is `daily_mask_clears` at crates/indicators/tests/invariants.rs:144-168,…` | OPEN |
| `F-27F1C2` | `gap` | `Sweep.excluded` is emitted in caller-argument order while `frequent` in the same struct is canonicalised | `crates/engine/src/lib.rs:292/:305/:311 (push order follows the caller's slice), :320 and :405 (frequent IS sorted),…` | OPEN |
| `F-6BA6D0` | `gap` | `Why` has no abstention variant, so the twenty VWAP positions return from every sweep as `AlwaysFalse` with `support: 0`, and docs/03-vocabulary.md §4 still says two positions and three instruments | `crates/engine/src/lib.rs:67-78 (`Why` is exactly AlwaysFalse / AlwaysTrue / NotLive) and :304-309;…` | OPEN |
| `F-10B225` | `gap` | `Why` has no variant for "the emitting family could not answer on this window", so 20 permanently-uncomputable VWAP positions are reported with the same reason as a condition the market genuinely never met | `/Users/parthi/IdeaProjects/brutex/crates/engine/src/lib.rs:67-78 (`Why`) · :304-309 (the AlwaysFalse push) ·…` | OPEN |
| `F-828BFD` | `gap` | `levels.last().frequent.is_empty()` is a structural tautology of walk, and `depth <= 3` on three positions can never fire | `crates/engine/src/lib.rs:1101-1105 and :1109, inside a_zero_threshold_is_raised_to_one_and_the_ladder_still_dies,…` | OPEN |
| `F-5A669C` | `gap` | clamp_i64's stated justification is false: a saturated level passes every bare comparison rather than failing every band test | `crates/indicators/src/daily.rs:331-338 (the doc claim); daily.rs:521-524 (Rel::Above / Rel::Below use the clamped…` | OPEN |
| `F-C9971A` | `gap` | docs/03-vocabulary.md still reports 234 live positions when 20 of them cannot fire on either swept instrument, so the effective vocabulary on the engine surface is 214 and the headline 2^234 is 2^214 | `docs/03-vocabulary.md:554 ('live &#124; 234') and :56-64 (§4 names only 52-53); docs/10-shared-core.md:151-158 (admits all…` | OPEN |
| `F-170044` | `gap` | docs/03-vocabulary.md §4 names 2 permanently-abstaining VWAP positions where the code makes 20, and still says "the three engine instruments" | `/Users/parthi/IdeaProjects/brutex/docs/03-vocabulary.md:62-64 ·…` | OPEN |
| `F-4DCEC0` | `gap` | docs/04-invariants.md V-02..V-05 name three modules that do not exist, and their `—` glyph rests on a justification that is no longer true | `docs/04-invariants.md:87 (indicators::barrier::no_lookahead), :88 and :90 (indicators::proptest::*), :89…` | OPEN |
| `F-B40027` | `gap` | inside_day and outside_day are running predicates over the forming day, not day types — but the running form is the only form §3 rule 7 permits, so the defect is the label and the missing completed-day family | `crates/indicators/src/session.rs:376-380 (dh <= ph && dl >= pl over the RUNNING extremes), session.rs:191-197 (the…` | OPEN |
| `F-255C6F` | `gap` | min_hits == bars is unsatisfiable by construction and reports identically to a satisfiable threshold nothing met | `crates/engine/src/lib.rs:310-318, :905-911 (the one test)` | OPEN |
| `F-DD700B` | `wrong` | The two NSE disaster-recovery Saturdays were absent from `CHARTER_NON_REGULAR_IST_DAYS`, so a 105-bar drill became the previous-day anchor. Measured on the store: for 2024-03-04 the pivot moved 144.5 index points and the CPR width 6.2x, enough to flip `cpr_class`; `prev5` stayed contaminated five sessions | `crates/indicators/src/evaluator.rs:159` | FIXED c6273645 — added as days 7 and 8, the two free slots `Calendar` already had; `the_eight_non_regular_days_are_the_charter_dates` recomputes both day numbers from the calendar date; charter row added |
| `F-A918E3` | `wrong` | `docs/00-charter.md` says 2021-02-24 forces exit at 16:50, implying a 17:00 close; `crates/pull/src/calendar.rs:252` said it "traded 09:15 and stopped at 10:08" and asserted 54 expected bars. The store holds 54. So the gaps audit's denominator was derived from the truncated series it exists to audit, and `GET /gaps` reported `lost_minutes: 0` for 2021-02 on a day the charter says traded seven hours longer — the P-70 tautology | `crates/pull/src/calendar.rs` vs `docs/00-charter.md` | IN PROGRESS — D-0420 and its named tests implement the sourced exchange/index split in this working tree; keep this status until an eventual commit SHA can close it honestly |
| `F-DD1DFC` | `wrong` | `median_step_micros` allocates and sorts `bars.len()-1` inside `walk_with`, so an O(n log n) sort runs PER CANDIDATE — up to 10,000 per rung under `par_iter`, twice per candidate in the walk-forward. Introduced this session by the horizon fix; the sibling `forced_exits` was hoisted out of the same loop for the same reason and this was left behind. Invisible to the ratio gates because it scales uniformly | `crates/runner/src/trade.rs:315` | IN PROGRESS — D-0421 code and `the_per_candidate_walk_derives_nothing_and_sorts_nothing` are green; retain this status until the eventual commit SHA can replace it honestly |
| `F-9DD4F6` | `gap` | walk's width check is a dead disjunct: no input can make `p >= ConditionMask::BITS` the deciding condition | `crates/engine/src/lib.rs:290-298, and an_out_of_range_position_is_refused_and_named at :1030-1043` | OPEN |
| `F-A22DDA` | `wrong` | Phase-1 root admission now covers the API startup lock, census and autopilot plus the CLI binary before logging/dispatch, but an existing wrong directory or different removable volume at the same pathname still passes; no sealed sentinel or expected device identity binds the root, the CLI holds no root directory capability after preflight, and direct library writers can bypass that process boundary | `crates/api/src/autopilot.rs (store_writable); crates/api/src/census.rs (read_all); crates/api/src/server.rs (take_serve_lock); crates/cli/src/lib.rs (preflight_store_root); crates/cli/src/main.rs (startup ordering); docs/06-limits.md §155` | IN PROGRESS — D-0473 and RV-01 green-close absent/non-directory recreation, false all-absent census and initial CLI side effects only; sentinel/device identity, held-capability all-writer admission, TOCTOU/hot-unplug handling and physical eject/full/read-only proofs remain unimplemented or unproven |
| `F-232FBF` | `wrong` | Execution V3 authenticated ledger rows only against identities and seals derived from those same rows, never against a retained Population V5 authority | `crates/cli/src/execution_v3.rs:1240-1326 (PreparedExecutionV3::validate), :2373-2382 (ExecutionV3SuccessorDisposition::authenticate), :2603-2642 (validate_complete_block)` | IN PROGRESS — the current module still takes caller-shaped PreparedExecutionV3 data and reconstructs completed blocks from its own ledger bytes; no CommittedStoredPopulationV5 production input or before/after source comparison has landed, so no fix is claimed |
| `F-A02E36` | `gap` | ExecutionV3Authority drops the live Population V5 source after reopen, so a successor disposition cannot prove its source remained unchanged | `crates/cli/src/execution_v3.rs:2337-2362 (ExecutionV3Authority contains only receipt + ledger); crates/cli/src/population_v5.rs:2451-2495 (the retained CommittedStoredPopulationV5 capability that is not carried)` | IN PROGRESS — the source-retaining production wrapper named by the Execution V3 doc comment is absent; no source-retention or stale-source proof is claimed |
| `F-328603` | `wrong` | Execution V3 lacked complete three-schedule validation for its five exit-coordinate slots: stop maps to Stop, target and TTP arm to Target, and TSL and TTP trail to Trail | `crates/cli/src/execution_v3.rs:164-170 (three axes), :2718-2872 (segment/order/bounds validators), :3966-3981 (fixture omits Trail)` | IN PROGRESS — the current untracked Stage-A rewrite now expresses that mapping and demands all three schedules, but its fixture still supplies only Stop and Target and no current test, Clippy or locked-workspace proof exists; no fix is claimed |
| `F-A5D460` | `wrong` | Execution V3 accepts run, grid, column, context and Runner digests without reproducing them from live execution authorities | `crates/cli/src/execution_v3.rs:687-787 (ExecutionV3DispositionRecord::validate only requires and self-seals the digests), :2878-2997 (validate_disposition_prefix joins only the ledger's own parameter/source fields)` | IN PROGRESS — no retained Runner execution, evaluated-grid, one-minute column or causal-context capability is an input to the current commit/authentication boundary; no fix is claimed |
| `F-63B197` | `gap` | Execution V3 has no persisted source-authenticated ranking metrics for Selection V5 to recompute | `crates/cli/src/execution_v3.rs:687-721 (disposition fields), :2365-2550 (successor projection); the module contains no ranking-metrics record or authenticated metrics accessor` | IN PROGRESS — support_hits alone cannot authorize win/loss, return, drawdown or risk-adjusted ranking; the metric authority and recomputation join are absent and no fix is claimed |
| `F-BB41C0` | `wrong` | The PolicyRefused fixture drops the nonzero training-run identity even though the Candidate contract requires one | `crates/cli/src/execution_v3.rs:725-749 (every disposition requires a nonzero execution_run_id), :4030-4034 (PolicyRefused writes zero); crates/cli/src/candidate_universe.rs:1615-1622 (Candidate row requires the run)` | IN PROGRESS — the current fixture contradicts both record validation and the upstream Candidate contract, so the current Execution V3 suite cannot be called green; no repair or gate result is claimed |
| `F-C29A38` | `wrong` | Execution V3 checks common rung only across caller-authored parameters and never against the Population V5 candidate rows | `crates/cli/src/execution_v3.rs:1285-1302 and :2913-2918 (parameter-only common-rung checks); crates/cli/src/population_v5.rs:744-806 (V5 retains the canonical Candidate record that Execution V3 never reads)` | IN PROGRESS — a source-bound seam must prove each V5 Candidate rung before a block can reject mixed-rung evidence or name one rung; that seam is absent and no fix is claimed |
| `F-D3D96B` | `law` | One fixed-offset authenticated Execution V3 lookup hashes every held ledger file twice before and after the read | `crates/cli/src/execution_v3.rs:2221-2260 (two require_unchanged calls), :2322-2333 (all held files), :3311-3420 (two full-file hashes per generation check)` | OPEN — the lookup is O(total held file bytes), not O(1), and no measured constant-cost generation capability or replacement design has landed |

---

## The 26 that were killed

Recorded because a sweep that reports only what survived is hiding its own error rate,
and because a killed finding is the one most likely to be raised again.

| Finding | Why it was wrong |
|---|---|
| `gap_up_day`/`gap_down_day`, `inside_day`/`outside_day` and `close_in_upper_third`/`close_in_lower_third` each leave a HIGH-frequency state with no bit at all | Not false — fully subsumed. Every fact is correct: 50 is `dh <= ph && dl >= pl` and 51 is `dh > ph && dl < pl` (session.rs:376-381), so an ordinary higher-high-higher-low day sets neither; 48/49 need `today_open != prev_close` (:366-371,… |
| The vocabulary contains pairs of live positions whose conjunction is provably empty, and the ladder reports them as `infrequent` — an empirical verdict on a… | The premises are true and the load-bearing claim is false. The premises check out: bit 101 needs `minutes_since_open >= 60` for window 3 to be marked closed (orb.rs:207-210 via `mark_closed`) while bit 44 needs `since < 60`… |
| Sweep.excluded is ordered by the caller's live slice, so two runs with identical bars and identical identity terms render differently | The mechanism is real — I measured walk(bars, &[9,7,0,1]) -> excluded [9,7] and walk(bars, &[7,9,0,1]) -> [7,9] — but the stated failure is not. `live` is a PARAMETER of walk, and the two calls pass different arguments, so §3 rule 5… |
| Excluded.support reports a measurement that was never taken for a NotLive position | The row carries its own disambiguator and I verified it: walk(&bars, &[9_999]) yields exactly `Excluded { position: 9999, support: 0, reason: NotLive }`. `reason` is in the same struct, it is the mechanism D-0080 requires for naming why a… |
| The `mask` term has no defined byte encoding, so whoever writes the hash will make run identity machine-dependent by default | Two kills. First, the failure is unreachable on every target this repository builds for: rust-toolchain.toml pins targets wasm32-unknown-unknown plus the host, and wasm32, aarch64-apple-darwin and x86_64 are all little-endian, so… |
| The float guard is a source scan in vocab only; indicators and engine rely on a lint that vocab's own test argues is insufficient | The central claim — that `pub struct Stat(f64)` in crates/indicators or crates/engine 'passes cargo clippy -D warnings and every gate' — is false. CI gate 11 rule 2 (.github/workflows/ci.yml:2173-2174) greps `\b(f32&#124;f64)\b` over every… |
| `direction` is a term of the run-identity hash and appears nowhere in the swept layer, so long and short over the same conditions are the same combination | Three independent kills. (1) The premise singles out one term of eight and none of the eight exists. `grep -rl blake3 crates/` returns exactly ONE file — `/Users/parthi/IdeaProjects/brutex/crates/vocab/src/lib.rs`, and the only hit in it… |
| `Frontier::frequent`'s documented order is the inverse of what the code does: the doc says support-primary, `sort_canonically` is mask-primary and the `hits`… | The worked example is factually wrong, and I measured it. The finding says 'at k=1 with bit 0 frequent at 500,000 hits and bit 200 frequent at 30 hits, `frequent[0]` is bit 0's singleton'. I built exactly that column — bit 0 on 5 bars,… |
| `direction` is a term in run identity and no part of the engine ever sees one, so a long run and a short run produce byte-identical output under two different… | The concrete failure describes code that does not exist. I grepped `crates/engine/src` and `crates/vocab/src` for identity machinery: `direction` appears in exactly one place in either crate, a doc comment at crates/vocab/src/lib.rs:58,… |
| Sub-claim of finding 9: three closes around the current-day 0.618 rung are 'indistinguishable from each other' | Falsified by running the finder's own example through the real Evaluator. On a session spanning 2,400,000..2,500,000: close 2,430,000 sets bit 43 close_in_lower_third, close 2,445,000 sets nothing, close 2,470,000 sets bit 42… |
| Sub-claim of finding 9: the Fibonacci ladders' 68-96% silence is a defect | crates/vocab/src/tolerance.rs:41-58 records the measured sweep at five candidate widths and states the reason R/100 was chosen: 'its busiest rung fires on about 3% of bars: often enough to carry information, rarely enough to… |
| Sub-claim of finding 12: a pre-open print must be kept out of 'the ORB windows' | crates/indicators/src/orb.rs:216-221 already returns early when minutes_since_open is None, with the comment 'A pre-open print is not part of any opening range, and ingest is supposed to have dropped it — but this module does not rely on… |
| Sub-claim of finding 6: inside_day / outside_day are 'wrong' and should classify a day type | A completed-day inside/outside classification for TODAY at bar N requires bars N+1..375, which is look-ahead — forbidden by CLAUDE.md §3 rule 7 and made unwritable by PastPrefix (lib.rs:92-100). The running form is the only legal form, so… |
| Sub-claim of findings 1 and 2: the implication chains can lose a combination | Self-contradicted by the finder's own notes and provably false. hits(A+b) is a subset of hits(A), so equal support means equal hit sets, so hits(A+b+C) = hits(A+C): a chain adds no bar and removes none, anti-monotonicity is untouched, and… |
| Sub-claim of findings 4 and 5: the previous-day close and today's open are 'not derivable' at any k | One-directional implicants exist and the findings' enumerations of alternatives are incomplete. {48 gap_up_day, 66 close_above_gap_mid} implies close > pdc, because 48 puts the open above pdc so midpoint(pdc, open) > pdc; {49, 67} implies… |
| Sub-claim of finding 11: the round-number grid must be caller-supplied because it differs between the two swept instruments | Round-number levels expressed in index points are the same grid on both: 100 index points is 10,000 paisa on NIFTY and on BANKNIFTY alike. What differs between them is the STRIKE grid (NIFTY 50, BANKNIFTY 100). The right reason to make… |
| Sub-claim of finding 3: cross-instrument is structurally impossible because engine::walk takes one bar-bit column | engine/src/lib.rs:260 takes &[ConditionMask] — an opaque column whose provenance it never inspects — and would accept a cross-instrument column with no change. The engine is not an obstacle. The obstacles are crates/indicators'… |
| The 84%-duplicate join redundancy is a finding (the measurable half of engine-edges finding 9) | Already designed, counted and published per level. `Frontier::duplicates` is a public field, incremented at :386 on every repeat, and its own doc at :116-117 states that the join reaches the same k-set from several pairs so `generated`… |
| The popcount guard at :379-381 is a §9 surviving-mutant violation | The mutation used to demonstrate it, `if cand.popcount() != k` -> `if false`, is not a mutation the gate applies. cargo-mutants 26.2.0 — the version D-0078 pins and .github/workflows/ci.yml:3145 installs — generated `src/lib.rs:379:36:… |
| A single 34-bit bar alone forces C(34,17) frequent 17-sets (the mechanism claimed by engine-edges finding 1) | An earlier guard on the same path forbids it. With exactly one bar every position has hits == bars, so the D-0080 arm at :310 excludes all of them as AlwaysTrue before k=1 — EXECUTED: one bar with all 234 live positions offered returns… |
| §3 rule 7 look-ahead IS constructible: `GapFib::step` accepts a receding timestamp and installs a LATER session's last-3-minute candle as an EARLIER bar's… | The mechanism reproduces — I drove `GapFib::step` with three bars of IST day 21001 and then three of day 21000 and got `GapLeg { x1: 2599000, x2: 2498980, direction: Down }` on the earlier day, x1 taken from a session 24 hours in that… |
| The previous-day anchor has no recency bound, so a session on the far side of a hole in the store gets a full pivot ladder from a session a month old | Three independent reasons. (1) The concrete failure does not reproduce. I fed IST day 20000 (60 bars) then IST day 20031 (60 bars) through the real Evaluator and measured 48 gap_up_day 0/60, 13 close_above_pdh 0/60, 184… |
| The VWAP family folds the current bar before emitting, and its two-observation sigma makes band1_upper exactly equal to the larger of the two bars' typical… | The arithmetic is right and it is not a defect. band1_upper = max(a,b)/3 at n=2 with equal volumes is simply what a POPULATION standard deviation means over two observations: sigma = half the spread, so mean + sigma = the larger… |
| Evaluator::positions() is guarded by CARDINALITY only, never by identity — a void position can displace a live one and the test still passes | Measured, twice, and the mutant dies both ways. I copied crates/{vocab,indicators} to a scratch workspace (byte-identical to HEAD, verified with diff) and built the exact mutant: daily::positions() publishes 240 (BitStatus::Void) in the… |
| The daily-module const assertion is `41 + 3 == 44` — two literals — and its message claims something about positions() and plan() that it never reads | The premise is right and the consequence is measured false. `const _: () = assert!(WIDTH_STATES_AT + WIDTH_STATES.len() == 44, ...)` does evaluate 41 + 3 == 44 and names neither function. But the drift it is accused of missing does not go… |
| C-I-02's recorded measurement is a 1.87x data dependence reported green — the 3.0x ceiling passes the thing the row forbids | Already recorded, in the two documents CLAUDE.md §10 makes the authority, so this is noise rather than news — and the finding's own proposed fix is what is already written. docs/06-limits.md §52, `One candle's cost varies 1.87x with what… |

---

## What this ledger does NOT claim

It does not claim the 99 are all the defects that exist. Ten lenses is ten lenses, and
`docs/06-limits.md` records what the sweep could not see: no bar data was read, so
nothing here was checked against real market behaviour, and the full pipeline
(candle → bits → ladder → output) has never been composed in a test.

It does not claim a `FIXED` row is beyond question. It claims a test exists that was
**shown to fail** against the code before the fix — which is a different and smaller
claim than correctness, and the only one that can be made mechanically.

<!-- rows-digest: 2139d535ece43d40 -->
<!-- dispositions: FIXED 18 · IN PROGRESS 10 · NEEDS A DECISION 7 · OPEN 68 · PARTLY FIXED 8 · REFUTED 0 · total 111 -->

## 2026-09-01 appended successor finding — D-0498

`F-23A9AE` (`wrong`) — **Candidate V1 statistical singleton is unreachable
because every closed mask expands to both Long and Short execution-coordinate
rows.** D-0492, D-0495, D-0496 and their invariant/plan prose called
`InsufficientForCscv` one real Candidate row. Statistics was counting one
mask/family hypothesis; Candidate V1 was counting persisted
mask×direction×exit rows. `CandidateUniverseReceiptV1::validate` now makes the
actual production law explicit: `closed_itemsets * (long cells + short cells)`,
and both directional grids are nonempty, so a nonzero receipt is at least two.
The upstream insufficient enum/codec shape remains append-only history, but it
is production-unreachable from current Candidate V1 and must not be invented at
Population V6.

Evidence: `crates/cli/src/candidate_universe.rs` receipt reconciliation and
`nonempty_candidate_receipt_is_exact_two_sided_mask_expansion`;
`crates/cli/src/population_statistics_v3.rs` retains the abstract `row_count ==
1` terminal; D-0498's Population V6 boundary accepts E/E, E/X, X/E and X/X and
refuses the five insufficient-containing pairs. Disposition: **IN PROGRESS** —
the semantic correction and static Rust/tests are present, but the focused
serialized Cargo proof is still pending. This post-audit append is deliberately
outside the immutable 2026-08-11 sweep tables and their guarded 111-row digest;
it neither deletes nor silently repurposes an earlier finding row.

## Post-audit append — 2026-09-02, surfaced by wiring `ledger-all`

**The dhan feed holds a NIFTY 1-minute bar inside the 2024-03-02 midday break,
and no command on the live path reports it.**

Severity `wrong`. **COSTS**: a bar that no session traded enters the column, so
every mask evaluated at that row is evidence from a minute the exchange was shut.

2024-03-02 is an NSE disaster-recovery Saturday and it traded **two** windows,
09:15–09:59 and 11:30–12:29 — 45 + 60 = 105 bars. Both `pull::calendar`
(`Observed::from_runs(19_784, &[(555, 599), (690, 749)])`, and its own test
asserting *"45 + 60, not 195"*) and `indicators::evaluator` (`19_784, // 2024-03-02
Sat, 09:15–09:59 + 11:30–12:29 — disaster recovery`) record it correctly. The
store does not agree with them.

Driving `cli ledger-all dhan 2024 3 2024 3 …` refuses with

> calendar receipt V2 timestamp 1709353800000000 is bucket 45 on measured open
> IST day 19784, but that bucket intersects no measured session window

1709353800000000 µs is 2024-03-02 10:00:00 IST — bucket 45 counting from the
09:15 open, the **first minute of the break**. The receipt is built by
`stored::calendar_receipt_v2_for_bars`, which maps `bar.ts_micros` straight off
the loaded span, so the timestamp is a bar the store actually holds.

**It is feed-specific, and that is the point.** The same command over the same
month:

| feed | 2024-03 |
|---|---|
| `dhan` | refuses — bar at 10:00, outside both windows |
| `zerodha` | clean; runs through to Search V4 replay |
| `truedata`, `gdfl`, `groww` | no bars for the month |

Two vendors redistributing one NSE feed disagree about whether a minute exists.
`CLAUDE.md` §3 rule 3 added `feed` as the ninth term of the run identity for
exactly this, and this is the first measured instance of the two differing on
something other than their bytes.

**Nothing on the live path catches it.** `cli verify dhan NIFTY` passes every
content check it makes — strictly-increasing time, byte-identical reruns, no
look-ahead, clean refusals — because none of them asks whether a bar's minute is
inside a measured session window. The check exists only in `calendar_receipt_v2`,
which until `ledger-all` had no caller that ran.

Disposition: **OPEN**. The bar count for day 19784 in the dhan store has not been
measured, so it is not yet known whether this is one stray minute or the whole
90-minute break; and whether the correct repair is to drop the out-of-window
bars or to re-pull the day is a decision for the operator, not this ledger. What
is settled is that the anomaly is real, reproducible, confined to one feed, and
invisible to every command that shipped before this one.

---

## F-EXITSHAPE-TWO-RULES · two exit-shape rules for one requirement, in one repository

**Severity: `wrong`. COSTS: yes — it can select a shape the operator ruled out by name.**

`crates/cli` and the Step-3 chain disagree about which exit shapes are acceptable,
and neither rule implies the other.

| path | admits | enforced at |
|---|---|---|
| the screen | `{stop AND (tsl OR ttp)}` | `Rules::protects`, `crates/cli/src/lib.rs` |
| `ledger-all` / Step-3 | `{stop AND target}` | `execution_refusal_bits`, `crates/runner/src/exit_grid_policy.rs:2679-2685` |

`chosen_is_in_bounds` (`exit_grid_policy.rs:2673-2683`) returns `false` unless BOTH
`stop` and `target` are `Some`, and the refusal bitset stops at
`FORCED_STOP_MISMATCH = 1 << 5` — there is no trailing bit. Neither
`execution_refusal_bits` nor `chosen_is_in_bounds` reads `tsl` or `ttp`.

So the Step-3 chain admits, by construction, the exact shape the screen refuses by
name: a fixed stop and a fixed target, which cap the loser and the winner alike.
`crates/cli/src/lib.rs`'s own test says so — *"A STOP AND A TARGET IS NOT ENOUGH,
and this is the case the operator ruled out by name"*.

None of the 39 Step-3 admission gates (`ledger_all.rs:392-428`) is an exit-shape
gate, and `population.rs:441-474` and `candidate_universe.rs:5482-5511` validate
exit coordinates while accepting `stop: None`.

### The fix, and why it was NOT applied here

The obvious change — append `MISSING_TRAIL = 1 << 6`, add it to `ALL`, and set it
in `execution_refusal_bits` when `tsl` and `ttp` are both `None` — was written,
compiled, and **reverted**.

It breaks a stated contract:
`validated_canonical_ordinals_name_every_authorizable_cell_exactly_once`
(`exit_grid_policy.rs:5479`) asserts that every in-bounds canonical cell is
authorizable, and the canonical grid legitimately produces `stop+target` cells
with no trail. Making the rule unconditional refuses them, and the only way to
keep that test green is to weaken it — which is the test that asserts nothing
`CLAUDE.md` §4 bans, hiding a behavioural change to the institutional path behind
a green suite.

The correct fix is a POLICY FIELD, not a constant: a new field on
`ExitGridPolicyV1` threaded through `ExitGridPolicyV1::new` as a parameter rather
than an environment read — `runner` may make exactly one env read and
`validate::fold_rungs` (`validate.rs:1567`) already spends it. Callers to update:
`cli/src/ledger_all.rs:246`, `cli/src/candidate_universe.rs:7191`,
`cli/src/anchored_search_lineage_v4.rs:1932`,
`cli/src/execution_disposition_v2.rs:3493`, `cli/src/execution_capability.rs:3239`
and `:3520`. The policy is versioned and persisted (`EXIT_GRID_POLICY_VERSION_V1`,
`selector_byte` at `exit_grid_policy.rs:4037`), so it also moves run identity —
`crates/runner/src/identity.rs` must be read before it lands.

The canonical coordinate space CAN already express a trailing exit:
`chosen_axes_are_in_bounds` (`:2686-2702`) bounds-checks `tsl` and `ttp.trail`
against `trail_levels_ppm`. Only the admission rule is missing, not the shape.

Disposition: **OPEN**. Real, reproduced from source, and deliberately not patched
in a rush. `ledger-all` cannot complete today for an unrelated reason — it needs
BANKNIFTY data, which the store does not hold — so nothing is currently selecting
through the weaker rule.
