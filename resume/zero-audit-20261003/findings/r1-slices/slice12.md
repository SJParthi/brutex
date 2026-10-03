# slice12 — 2 findings

## F1 [medium] calendar_of: the minute-counter short-circuit marks a holed day as a full 375-bar session (claim "nothing a full read could discover" is false)
- where: crates/api/src/calendar_of.rs:385-405 (the `Ok(file) if file.records() == expected` arm), and the claim in the module header at calendar_of.rs:38-41 ("a month whose minute count is exactly `sessions × 375` has no irregular day in it by arithmetic — there is nothing a full read could discover") plus the arm comment ("A walk could find nothing a subtraction has not already proved").
- what: `expected` counts only days the DAILY rung proves (`by_month`), but `file.records()` counts every minute record in the file. That includes minutes on a day with no daily bar, which the walk arm ignores on purpose (`get_mut`, never `insert`; the module's own test `a_month_whose_minutes_are_not_sessions_times_375_is_walked_into_its_runs` builds this "stray" case as a real input). So a month can match the counter while one daily-proved day is short. Every proved day is then stamped `(OPEN_MINUTE, LAST_MINUTE)`, i.e. `Open(Session::full())`, owed 375. The same day with the same minutes walks to `[(555,719),(725,929)]`, owed 370, when the stray minutes are absent or there is one more or one fewer of them. The derived calendar for day B therefore depends on bars from an unrelated day C, and the "by arithmetic" proof does not hold. Minutes outside 09:15-15:29 inflate the count the same way. The derived calendar feeds `/calendar.json` and `/gaps.json`'s peer denominator.
- evidence: I ran a probe (crates/api/tests/zz_probe_slice12.rs, now deleted) through the public `api::calendar_of::derive`. Daily rung: 2026-01-05 and 2026-01-06. Minute rung: 01-05 full (375); 01-06 with minutes 720-724 missing (370); 5 minutes on 01-07, which has no daily bar. Total 750 = 2 × 375.
  ```
  by_counter=1 walked=0 kind_b=Open(Session { windows: [Window { from: 555, to: 929 }, Window { from: 0, to: 0 }], count: 1 }) expected_b=Some(375)
  full=true
  ```
  With no stray minutes the same January is walked, and 01-06 comes out as two runs owing 370 (the repo's own walked test asserts exactly this for an identical holed day).
- fix: Take the short-circuit only when it is sound. Two options: (a) the minute file's first and last record fall on daily-proved days and the counter matches (still not airtight), or (b) better, also require the file's per-day bound. The minimal correct fix is to drop the "by arithmetic" claim and walk whenever any minute could fall outside a proved day or a session. For example, check that the first and last record timestamps lie inside proved days and inside [OPEN_MINUTE, LAST_MINUTE]; if not, walk. Then add a test with the stray-minutes construction above that asserts the counter path is not taken (or that day B owes 370).

## F2 [low] census: the store grid's always-on axis still says the engine surface is "exactly these two" indices, contradicting CLAUDE.md §1 (D-0506/D-0682)
- where: crates/api/src/census.rs:797-801 (`held_series` doc: "`CLAUDE.md` §1 fixes the engine surface at exactly these two, so their absence is the single most important thing this page can report") and census.rs:1073-1090 (`swept_series` doc: "`CLAUDE.md` §1 fixes the surface at exactly these two"; code returns only BANKNIFTY and NIFTY).
- what: CLAUDE.md §1 now defines the engine surface as the two spot indices plus the cash equities of the 208 F&O share underlyings (D-0506, counted by D-0682). The doc cites §1 for a claim §1 no longer makes. The behaviour that rests on that claim is "the swept series are always on the axis, held or not, so an empty store still names what it is missing". That now covers 2 of 210 swept instruments. A swept equity with no data on disk never appears on `/store`'s coverage grid, and that absence is exactly what the doc says the page exists to report. `crates/api/src/coverage.rs:28-31` in the same slice already states the widened surface correctly, so the two modules disagree.
- evidence: trace. `swept_series()` hard-codes `["BANKNIFTY", "NIFTY"]` with `segment: Segment::Index` (census.rs:1080-1090). `held_series` seeds its output with that list only (census.rs:821). A CASH series enters the axis only through `manifest.held_keys()`, so an unheld swept equity is never in it.
- fix: Either seed `swept_series` from the engine's own swept list (indices plus `FNO_UNDERLYINGS` minus `FNO_INDEX_UNDERLYINGS`, at segment CASH), or correct both doc comments to say the always-on rows are the two indices only and that unheld swept equities are not shown. Whichever is chosen, record it in docs/05-decisions.md.

## What I checked and found sound
- All 21 files' production code was read: request parsers, refusals, caches, paging windows, overflow checks, JSON shapes.
- detail.rs `window`/`seek_window`/`Page::parse` arithmetic.
- The paging and caching in candidatejson, indexstop{json,vix,candles,qualification,ranking}, expressionsearchjson and frontierjson: offset == len gives an empty page, offset > len is refused, cache eviction and pin checks hold. `program_index = index/2` is backed by the store's enforced long/short order (cli index_stop_store.rs:411-417). Frontier's `partial` is always `None` on the read-only handle (frontier.rs:1586).
- credential_law `Watch`: rotation, dead-list and halt paths.
- coverage/constituents slot indexing: `Vendor` discriminants match `Vendor::ALL`, and `Tier` discriminants match `Tier::ALL`.
- folder/indexmap bounds (UC-19 is already fixed).
- The census size, type and FIFO guards.
- index_consistency `reasons()` lists all 14 variants.
- indexstoplaunch/metadata bounds: `max_loss_points*100` is checked in `LaunchInput::validate`, and `finished_micros` is overwritten by the caller in sweeprun.rs:3159.
- The emitted.rs accounting: 61 counted sites matches a grep.
- A probe confirmed the coverage doc's six `no_nse_isin` names: F&O names outside NTM are exactly the 5 index underlyings.
