# Pass 9: law against code (final/all-fixes-zero @ 1f4de71)

Read-only audit of `/home/claude/wt/zero3`, detached at `1f4de71` ("docs: CE-19's decision (D-1981) and invariant (ZE-02)").

Scope:
- The factual claims in the TRACKED `CLAUDE.md` §1, §3 (rules 4, 7 and 8), §4, §6, §7 and §8, and the matching claims in `AGENTS.md`.
- A trace of 20 vendor, exchange and cost constants to `docs/00-charter.md`.

§3 rule 3 (identity), §2 and §5 (P6-*) and the cli surface (P8-*) are skipped, as the brief asks.

Caveat: the session-context copy of `CLAUDE.md` at `/home/claude/brutex` is older than the tracked file. In that copy, rule 4 still describes lane-local `kept` and `extend`, and rule 7 says "`cli` and `runner` pass `Absent`". D-1440 and D-1764 already corrected both in the tracked file, so they are not reported here.

No cargo was run. Every new finding is low and is argued from source with grep or sed output.

## Verdict

**1f4de71: 4 new findings, all low. There are no medium or high findings.**

Every scope claim in §1 holds in code, and so do these:
- the INDIAVIX exclusion
- the BSE refusal
- the `is_sweepable` predicate
- the gross labels
- the Selection V6 exclusion of equities

Every engine claim in rule 4 holds, as do these:
- `Column::support`
- the pre-sized `HashSet<u32>`
- the removed `seen` set
- `drain` reserving `out`, then lane counts, then a serial push
- the bench ids

The other sections also hold:
- **Rule 7:** the `PastPrefix` count is exact.
- **§4 and §6:** there is no depth knob, ORM, writable mmap or dynamic schema.
- **§7:** prices are integers, and the OI sentinel is `i64::MIN`.
- **§8:** no tracked file holds a real parameter path, and no credential is read from the environment or a file.

The new defects:
- **P9-01:** a stored statistic is rounded, which contradicts §7.
- **P9-02:** dated instrument facts in `costs` cite no charter source.
- **P9-03:** a Groww daily window cap is encoded that the charter says is absent.
- **P9-04:** rustdoc still describes the removed `seen` set in the present tense.

## Table: claim, where checked, holds?, evidence

| # | Claim | Where checked | Holds? | Evidence |
|---|---|---|---|---|
| 1 | §1 "the 208 F&O underlyings that are shares" = `FNO_UNDERLYINGS` less `FNO_INDEX_UNDERLYINGS` | core universe.rs:914 (`[&str;213]`), :1150 (`[&str;5]`), test :5983 `assert_eq!(shares, 208)` | yes | Extracted both arrays (ran awk): 213 unique, and `comm -23 fno ntm` = exactly BANKNIFTY FINNIFTY MIDCPNIFTY NIFTY NIFTYNXT50. The AGENTS.md §1 phrasing, "intersection of F&O and total-market tables", is also 208. |
| 2 | §1 sweep surface = 2 indices + F&O shares, NSE only | core instrument.rs:265 `SWEPT`, :354-371 `is_sweepable`; cli stored.rs:1916 `swept_index` (every stored-sweep door) | yes | The index arm walks `SWEPT`. The cash arm requires `Exchange::Nse`, an `FNO_INDEX` hit and none of the 5 index names. Every other (segment, kind) is `false`. |
| 3 | §1 BSE and MCX not swept, not pulled | `Exchange` has only Nse/Bse; pull vendor.rs:877 `(Exchange::Bse, _) => None`; pull fnowork.rs:81 `const VENUE: Exchange = Exchange::Nse` | yes | BSE codes survive only in manifest decoding (manifest.rs:1214) and in the cost tables, which the append-only store needs. |
| 4 | §1 `NSE-INDIAVIX` never in vocabulary | vocab tests/expression.rs:205 `"NSE_INDIAVIX"` gives `UnavailableBit`; `grep -i vix crates/vocab/src crates/engine/src crates/indicators/src` = 0 | yes | |
| 5 | §1 INDIAVIX never in ranking or run identity | cli index_stop_vix.rs:1-3 ("cannot enter strategy/source/search identities, ranking or prices"); global_replay.rs:150 ("execution identity, deliberately independent of India VIX"), :156 (publication identity only); stored.rs:3615-3623 (`load` refuses INDIAVIX) | yes | VIX enters only the *publication* digest of a stamped replay. That digest is not the §3 rule 3 run identity. |
| 6 | §1 equities labelled gross of every charge wherever rendered | runner audit.rs:130-153 `CostScope::report_note` / `CASH_EQUITY_GROSS`; cli lib.rs:8186 `render_top_record` (`stored_provenance` + `SHARE_MEAN_LEGEND`); api backtest.rs `equity_note`, frontierjson/livejson/trades/candidatejson ("gross"); web charge-scope.js | yes (sampled) | `/expression-search.json` serves signal counts only, with no money. `/engine/top.json` reuses `render_top_record`. Every route was not exhaustively walked. |
| 7 | §1 no equity result enters Selection V6 or execution authority | cli selection_v6_source.rs:66-68, :122-124 (`ExecutionV4Family::{Nifty,BankNifty}` only); ledger_v6.rs:75 `ROUTE_FAMILIES = ["NIFTY","BANKNIFTY"]` | yes, by type | No equity variant exists in the family enum. |
| 8 | Rule 4: `Column::support` is one fixed-six-word `hits` per bar | engine column.rs:105-109 (single `fold` over `rows`); vocab mask.rs:18 `WORDS = 6`, :144-152 (six unrolled words, no branch) | yes | Bench ids C-E-02 (ratio.rs:323), C-E-09 (:582) and C-E-11 (:418) exist. C-E-02b is in docs/04-invariants.md:2017. |
| 9 | Rule 4: duplicate rejection is one pre-sized `HashSet<u32>::insert` at k=1; the `seen` set is gone at k≥2 | engine lib.rs:1507-1508 (`HashSet::new()` then `try_reserve(live.len())`), :141-143 `offer` | yes | The engine's live code holds no `seen`. Two stale doc mentions remain (P9-04). |
| 10 | Rule 4: result append is one `Vec::push` through `primitives::append`; `drain` reserves `out` for the batch, lanes fill `counts`, one serial push loop | engine lib.rs:149-151, :2359-2396 | yes | `out.try_reserve(batch.len())` comes before the scope. Workers write `counts.chunks_mut`, and a serial loop then `append`s. k=1 reserves `live.len()` (:1501). |
| 11 | Rule 7: `PastPrefix` has zero production call sites (one doc comment, two declarations) | `grep -rn PastPrefix crates` | yes | indicators lib.rs:35 (doc), :193, :197 (decl). :1178, :1184, :1187, :1443 and :1451 are all after `#[cfg(test)]` (:665). |
| 12 | Rule 7: `Column::build` streams `bars.iter().enumerate()`, one bar at a time; `Evaluator` is never given the slice | indicators column.rs:544-558 | yes for `Evaluator` | The anchored path differs; see the note under P9-04. |
| 13 | Rule 7: stored callers select availability by kind (index Absent, equity Present) | cli stored.rs:2065-2066 `vwap_availability`; hard-coded `Absent` at ledger_v6.rs:342, ledger_all.rs:1116 | yes | Every hard-coded non-test `Absent` is on an index-only family (NIFTY/BANKNIFTY). The others are inside `#[cfg(test)]` modules. |
| 14 | Rule 8: condition bits are never renumbered or reused | vocab tests/table.rs:119, :182, :229, :310, :384, :468 | yes, today | The document and the table are cross-checked, and tombstones keep their index. No digest of the historical name list is pinned, so a coordinated rename of the doc and the table would pass. This is a review-only guard and is not reported as a defect. |
| 15 | §4 no depth parameter on the sweep | engine `Ladder` fields lib.rs:1010-1016 (min_hits, ceiling, pair_budget, lanes); all 120 `BRUTEX_*` knobs listed | yes | `depth` occurs only as a reached-depth OUTPUT (results.rs:321, batch.rs:115). `BRUTEX_CEILING` is a candidate budget that halts loudly. |
| 16 | §4 no query planner or ORM, and no writable mmap | Cargo.lock: no diesel, sqlx, sea-orm, rusqlite or memmap; no `MmapMut`, `map_mut` or `PROT_WRITE` in crates | yes | `serde_json` is used only by api and pull for wire JSON. |
| 17 | §6 there is no `k` parameter: no default, token or env | as row 15 | yes | |
| 18 | §7 prices are paisa `i64`; i64::MIN is the OI null; zero means zero | indicators lib.rs:1653 `OI_NULL = i64::MIN`; lake bar.rs:37; pull fetch.rs:964 `unwrap_or(i64::MIN)`; csv.rs:1399 (a literal `i64::MIN` is refused) | yes | The money half was already confirmed in pass 7, Theme 3. |
| 19 | §7 the tick grid is snapped half-up once, at the write boundary | core price.rs `from_rupees_half_up` (pass 7); lake bar.rs:161 delegates | yes | Not re-derived. |
| 20 | §7 statistics are never rounded for storage | cli frontier.rs:268-275, :414-416, :689-691; runner outcome.rs:1559, :1695-1713 | **no** | **P9-01** |
| 21 | §8 no literal parameter path in any tracked file | `git grep` for 4-segment `"/a/b/c/d"` strings, and for `/<x>/(prod|live|dev|…)/(dhan|zerodha|groww…)/` | yes | Only placeholders appear: `/org/env/vendor/field`, `/orgone/testenv/…`, `/a/b/c/d`. `config/` holds one research TOML. |
| 22 | §8 credential never from env, file or prompt; never minted | `git grep 'env::var(…TOKEN|SECRET|KEY…)'` = 0; pull totp.rs:18-23 (computes a code, exchanges nothing) | yes | |
| 23 | Rule 1: every vendor, exchange or cost fact used in code has a charter source (20 sampled) | see the next table | **partly** | Vendor rates and caps are sourced. Cost rates are known-unsourced (hunt-costs-5). **P9-02** and **P9-03** are new. |

### The 20 sampled constants

| Constant | Code | Charter row | Traced? |
|---|---|---|---|
| Groww 500/min | pull rate.rs:362 | §4 Groww "Rate limit 500 requests per minute" | yes |
| Groww 8/s | rate.rs:374 | §4 "Production ceiling is 8/s, chosen not measured" | yes (UNVERIFIED, so labelled) |
| Dhan 5/s | rate.rs:297 | §4 Dhan "5/s" | yes |
| Dhan 100,000/day | rate.rs:328 | §4 Dhan "100,000/day" | yes |
| Zerodha 3/s | rate.rs:348 | §4z "3 requests/second" | yes |
| Groww 1-min cap 30 | vendor.rs:4766 | §4 "30 days per request" (the conflict with 7 is recorded) | yes |
| **Groww daily cap 180** | vendor.rs:4766 | §4 "Window cap, daily — UNVERIFIED … Encoded as **absent**" | **no (P9-03)** |
| Dhan 1-min cap 90 | vendor.rs:4483 | §4 Dhan "90 days per request" | yes |
| Zerodha caps 60 / 2000 | vendor.rs:5448 | §4z forum thread 7756 | yes |
| Session open 09:15, last 15:29 | pull calendar.rs:126, :133 | §3 | yes |
| Outage day 18,682 | calendar.rs:144 | §3 "IST day 18,682" | yes |
| Region ap-south-1 | config.rs:146 | §4 Credentials | yes |
| Brokerage ₹20 per order | costs rate.rs:120, :129 | none | no (known: hunt-costs-5) |
| SEBI fee, NSE IPFT, stamp 0.003%, GST 18% | rate.rs:149, :163, :210, :242 | none | no (hunt-costs-5) |
| STT 0.0625 / 0.10 / 0.15 % | regime.rs:316-345 | none | no (hunt-costs-5) |
| NSE exchange charge 0.03503 % | regime.rs:390-397 | none | no (hunt-costs-5) |
| Option tick ₹0.05 | rate.rs:252 | §3 has only "Tick grid 2 decimal places" | no (cites `COSTS_VERIFIED` §11; inside hunt-costs-5's "cost" scope) |
| **Lot sizes (NIFTY 75/50/25/75/65; BANKNIFTY 25/15/30/35/30)** | costs lot.rs:115-196 | none | **no (P9-02)** |
| **Strike steps 50 / 100** | strike.rs:118-121, :139-187 | none | **no (P9-02)** |
| **Weekly expiry weekdays (Thu→Tue 2025-09-02, …)** | expiry.rs:128-256 | none | **no (P9-02)** |

---

## New findings

### P9-01 low: the frontier ledger stores the t-statistic, the payoff ratio and the mean RE-SCALED and ROUNDED, against §7's "never rounded for storage"; D-0287's justification misquotes §7

**Where.**
- cli frontier.rs:267-275: `mean_milli_paisa: i64`, `t_milli: i64`, `payoff_bp: i64`, written to disk at :414-416.
- Filled at :689-691 from runner outcome.rs:
  - `t_milli()` → `milli` (:1695-1713): `let out = scaled.round() as i64;`, and NaN is stored as 0.
  - `payoff_bp()` (:1559): `let clamped = ratio.max(0.0) as i64;`, which TRUNCATES the ratio to hundredths.

**Why it is wrong.** `CLAUDE.md` §7: *"Statistical values (Sharpe, p-values, ratios) keep full precision and are never rounded for storage."* A t-statistic and a win/loss ratio are exactly that class. Here they are stored as fixed-point integers:
- t is rounded to 0.001.
- The payoff is truncated to 0.01. For example, 11.11 is stored as 11; `outcome.rs:2670` asserts this ("truncated from 11.11").
- The mean is rounded to 0.001 paisa.

These stored values are what later readers rank and compare: `cli elite` ranks on `payoff_bp`, and `/live.json` compares `t_milli` with the Bonferroni bar (CE-7's fix exists because of that rounding). D-0287 (docs/05-decisions.md:21751-21756) justifies the scaling with *"§7 keeps floats off any path whose output is compared"*. §7 does not say that. The ledger entry rests on a misquotation, and `CLAUDE.md` wins (§10). The `.grk` precedent at docs/05-decisions.md:38871 applied the real sentence and stored the bits unrounded.

**Repro (not run, source only).** `grep -n "t_milli\|payoff_bp" crates/cli/src/frontier.rs` shows the `i64` fields and the `to_le_bytes` writes. outcome.rs:2670 is an existing test that pins the truncation.

**Minimal fix.** Choose one:
- Store `Edge::t` and the payoff ratio as `f64::to_bits()` (new frontier version, append-only).
- Record a `docs/05-decisions.md` entry and a `CLAUDE.md` §7 sentence that carve out display-scaled ranking columns, and correct D-0287's quotation.

Either way, make `payoff_bp` round rather than truncate. The truncation is the "systematically toward zero" bias that D-0287 itself rejected for `t`.

### P9-02 low: dated instrument and exchange facts in `costs` (lot sizes, strike steps, expiry weekdays) cite a predecessor document that neither the repository nor the charter holds, and the known-unsourced note covers only "cost rates"

**Where.**
- costs lot.rs:115-196: e.g. `LotSize::new_const(65)`, "TRACK2_OPTIONS_SPEC §8.3 (`NSE Jan-2026 circular`)".
- strike.rs:146-170: 50 and 100 rupee steps, "TRACK2_OPTIONS_SPEC §5".
- expiry.rs:128-256: Thursday/Tuesday regimes, "TRACK2_OPTIONS_SPEC §2".

**Why it is wrong.** §3 rule 1 says every claim about *an exchange, an instrument or a cost* traces to a source recorded in `docs/00-charter.md`. These rows are marked `DatedRow::verified`, but:
- `git grep -l TRACK2_OPTIONS_SPEC` matches only the three `costs` files. No document in `docs/` names it.
- `docs/00-charter.md` has no lot-size, strike-step or expiry-weekday row (grep "lot", "strike", "expiry", "FAOP", "SEBI/HO" = 0 hits for these facts).

The known gap, hunt-costs-5, is recorded at docs/06-limits.md:14119 and docs/11-findings.md:690. Its scope is *"no charter source for any cost rate"*. A lot size is a contract specification, not a rate. A wrong lot size scales every quantity-proportional charge by up to 3x (lot.rs:5-10). So the larger instrument-fact gap is not recorded as UNVERIFIED anywhere.

**Repro (ran).** `git grep -l TRACK2_OPTIONS_SPEC` returns `crates/costs/src/{expiry,lot,strike}.rs` only. `grep -n -i "lot size\|FAOP\|strike" docs/00-charter.md` finds no row for these facts.

**Minimal fix.** Choose one:
- Add charter rows for the circulars that are actually cited (`SEBI/HO/MRD-PoD2/CIR/P/2024/00181`, `FAOP/68747`, `FAOP70616`, …), with lanes.
- Widen hunt-costs-5's limits and findings rows to name lot sizes, strike steps and expiry weekdays as UNVERIFIED under rule 1.

### P9-03 low: Groww's daily window cap is encoded as 180 days, while the charter row says it is UNVERIFIED and "Encoded as absent in `window_caps`"; the code comment contradicts itself

**Where.** pull vendor.rs:4744-4766:

```rust
// The day row is absent for the same reason as the vendor above: the
// charter records no day-level cap for this feed either. UNVERIFIED,
// and absent rather than guessed.
// BOTH ROWS THE VENDOR PUBLISHES, and the daily one was missing.
// `Groww Docs/11-backtesting.md`, "Backtesting Data Limits": ...
window_caps: &[(Granularity::Minute1, 30), (Granularity::Day1, 180)],
```

**Why it is wrong.** `docs/00-charter.md` §4 Groww says: *"Window cap, daily | **UNVERIFIED.** No day-level figure is published in any source this repository has read … Encoded as **absent** in `pull::vendor::HttpSpec::window_caps`"*. The code encodes 180 and cites `Groww Docs/11-backtesting.md`, a source the charter does not record. Its first sentence then still says the row is absent. Either the charter is stale (the later reading should be recorded with its lane) or the constant is unsourced under rule 1. Both documents cannot be right. The comment notes that it changes no request today, because the month split binds first. What it does change is the vendor fact that the descriptor publishes.

**Repro (ran).** `grep -n "Day1, 180" crates/pull/src/vendor.rs` (:4766), and `grep -n "Window cap, daily" docs/00-charter.md` gives the Groww row "Encoded as absent". `git log -S"(Granularity::Day1, 180)"` shows the row dates from ffa41c6.

**Minimal fix.**
- Record `Groww Docs/11-backtesting.md` "Backtesting Data Limits" in charter §4 as the daily-cap source, with its lane.
- Replace the stale "absent" sentence there.
- Delete the contradicting first comment paragraph at vendor.rs:4744-4746.

### P9-04 low: rustdoc still describes `Ladder::walk`'s mask-keyed `seen` set in the present tense; `CLAUDE.md` rule 4 and engine lib.rs say it was removed

**Where.**
- engine column.rs:115-116: "[`crate::Ladder::walk`]'s `seen` set rejects a duplicate by its MASK".
- column.rs:624-625: "`Ladder::walk`'s mask-keyed `seen` calls those three distinct candidates".

**Why it is wrong.** `CLAUDE.md` §3 rule 4 says: *"At k≥2 the prefix join is injective, so its former candidate `seen` set was removed"*. engine lib.rs:65, :743, :783 and :2335 all say it is gone. The `support_fingerprinted` rationale therefore argues against a mechanism that no longer exists. The real present-tense fact is that the injective prefix join never emits a repeat mask, and distinct masks that select the same bars are still counted as distinct trials.

**Repro (ran).** `grep -n "seen" crates/engine/src/column.rs` returns :115 and :625.

**Minimal fix.** Reword both sentences in the past tense ("the former `seen` set, and today the injective prefix join, treat these as distinct …").

**Note on rule 7 (not a finding, recorded for the next editor).** Rule 7 says look-ahead holds by the shape of the fold and that the evaluator "is never given the slice". On the shipping anchored door this is not the mechanism:
- `AnchoredColumn::build_required` is the door (cli lib.rs:2608, candidate_universe.rs:1664, step3_orchestrator.rs:4776).
- That door's `AnchoredEvaluator` is given the full `&[DailyReference]` series, future days included (indicators anchored.rs:283-286).
- Look-ahead is prevented there by an index cursor that advances only across days strictly before the signal day (anchored.rs:483-489). That is the "consumer that indexes" case rule 7 describes, guarded by a cursor rather than by `PastPrefix`.

The property holds, but rule 7 names only the streaming half.
