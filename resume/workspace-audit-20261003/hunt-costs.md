# hunt-costs: costs, greeks, telemetry at 1087e54

## Verdict

`costs`, `greeks` and `telemetry` are in good shape at HEAD. I found no panic, no wrap and no float in `costs`. `greeks` matches the textbook Black-Scholes-Merton formulas. In 700 contracts put-call parity holds to 1.9e-16 relative and delta parity to under 1e-12. Every degenerate input is refused by name.

Gate 17 holds. `vocab`, `engine`, `indicators` and `runner` have no `telemetry` dependency, and their `src` has no `println!`, `eprintln!` or `telemetry::` call.

I found six NEW defects:

1. **STT under-charge (costs):** the STT on a sell leg is charged at the rate in force on the trade's entry day, not the sell day. A long position that sells on 2026-04-01 or later pays 0.10% instead of 0.15%. This under-charges, which contradicts the "always over-charges" claim in `docs/06-limits.md` §27.
2. **Run-id collision on restart (telemetry):** `reserve_run_id` can hand out an id that is already in the log after a restart.
3. **Two processes on one log directory (telemetry):** two sinks on one directory get the same run id and write duplicate `seq` values. They also rotate each other's files, so a file passes its size bound and newest-first file order stops being time order. Two concurrent `cli` commands share `logs/cli`, so this can happen.
4. **Clock jump latches forever (telemetry):** a wall-clock jump forward is latched into every later event and survives restarts.
5. **Stale citation (costs):** a decision citation in the `costs` crate docs points at the wrong entry.
6. **Rule-1 gap (costs):** `docs/00-charter.md` records no cost source at all. That is a gap against `CLAUDE.md` §3 rule 1.

Prior verdicts re-checked at HEAD:
- probeengine-1 (the `PrintedExtreme` low overflow): **FIXED**.
- W1-greeks1-0: **FIXED**.
- W1-greeks1-1: **FIXED**.
- W3-costs1-0 (weekly expiry projected across a regime row): **FIXED**.

All probes were deleted. `git status --porcelain` shows no `hunt-costs` file.

## Findings

| id | sev | crate | file:line | what is wrong | evidence | status |
|---|---|---|---|---|---|---|
| hunt-costs-1 | medium | costs | `crates/costs/src/trip.rs:1222-1226` (`Rates::resolve(.., trip.entry().day())`), `trip.rs:364-369` | The STT rate is picked by the ENTRY day for both legs. STT is a sell-side levy, so a long position that buys before a rate change and sells after it pays the old rate. For the 2026-04-01 change that is 0.10% instead of 0.15%, an under-charge of 1/3 of the tax. `docs/06-limits.md` §27 says "every figure this crate produces is at least the real charge … always in the same direction", and this case breaks that. §27 does not list this case. The only place it shows up is the test `the_regime_is_the_entry_days_and_the_exit_day_never_moves_it` (trip.rs:1649), which pins the behaviour as intended (DEC-COST-002), not as a limit. Today nothing in production calls `costs::trip::price`: no non-doc caller outside `crates/costs` (grep). So the defect is latent until options pricing is wired. | Probe `zz_audit_hunt-costs_1` (long, 1000 units, flat 100.00 then 120.00 bars, entry 2026-03-31, exit 2026-04-01): `LONG sell_notional=11995000 stt_charged=12000 rate_entry=10000 rate_sell_day=15000 stt_at_sell_day_rate=18000`. ₹120 charged, ₹180 at the sell day's rate. | NEW (the behaviour is pinned by an in-crate test; the under-charge is not documented in 06-limits §27, which claims the opposite) |
| hunt-costs-2 | low | telemetry | `crates/telemetry/src/sink.rs:620-628` (doc), `:1032-1039` (`reserve_run_id`), `:791` (`reserved_run: AtomicU64::new(seq)`) | On open, the reserved-id counter starts at the last `seq` found on disk. The doc says: "A caller that reserves an id and then writes at least one event carrying it therefore makes the next process start strictly above every id still present in the log." That is false whenever the process reserves more ids than events land. For example, two reservations are made, only the second id's event is written, then the process is killed or the first id's event is filtered. The restarted sink then hands out an id that is already in the log. The fallback caller is `cli::run_with_sink` (`crates/cli/src/lib.rs:2136`). It is used when `operation_audit::current_id()` is None, which means every non-sweep command. | Probe `zz_audit_hunt-costs_2::probe_reserved_id_reused…`: `first process: r1=1 r2=2` / `after restart: next reserved id = 2; r2 in log = 2`. | NEW |
| hunt-costs-3 | medium | telemetry, cli | `crates/telemetry/src/lib.rs:56-89` (`install` refuses only a second sink in ONE process), `sink.rs:1267-1303` (`roll`), `crates/cli/src/lib.rs:2995` (`store.join("logs").join("cli")`), `crates/cli/src/main.rs:63` (every `cli` process installs a sink) | `cli/src/lib.rs:3042` says two sinks on one path "each roll the other's file away", but that is prevented only inside one process. Two `cli` processes run at the same time (a sweep holding the execution lease plus any non-sweep command, which takes no lease: `run_durable` at `cli/src/lib.rs:2096-2098`) each open `logs/cli/events.ndjson`. Each resumes the same `seq` and reserves the same run id. One process's roll renames the file the other is still appending to, and that file then grows past its bound under a rotated name. `tail` walks files newest-first and stops a `since` query at the first older record, so the merged order is no longer time order. | Probe `probe_two_sinks_one_directory` (max 2048 B, keep 4, 60 alternating events): `process A reserved 1, process B reserved 1`. `events.2.ndjson len=4068` is past the 2048 bound and `events.3.ndjson len=0`. `records read=60 distinct seq=30`, so every `seq` appears twice. Newest-first the records read `59,57,55,…` (only B's events), while A's interleaved events sit in older files. Both `health()` report `rotations: 1, rotation_failures: 0` and `is_loud() == false`. | NEW |
| hunt-costs-4 | low | telemetry | `crates/telemetry/src/sink.rs:579-582` (`Inner::stamp` = `now.max(last_at)`), `:1381-1391` (`resume_point` seeds `last_at` from disk) | The monotonic clamp has no forward bound. If the wall clock is ever set forward by mistake (here to 2100), every later event is stamped with that future time until real time catches up. Because `resume_point` reads the last stamp back on open, this survives every restart. Stamps are silently wrong, `since` windows are wrong, and nothing reports it in `Health`. I found no mention of a forward clock jump in sink.rs or 06-limits (grep). | Probe `probe_forward_clock_latches_across_restart`: after patching the last record's `ms` to 4102444800000 and reopening, the next event is `seq=2 ms=4102444800000 ts=2100-01-01T00:00:00.000Z msg=after restart, clock correct`, while `now_millis=1791002721309`. | NEW |
| hunt-costs-5 | low (law) | costs, docs | `docs/00-charter.md` (grep for STT, stamp, IPFT, brokerage, GST, COSTS_VERIFIED, or a cost-rate figure finds nothing; line 176 only says "Cost modelling belongs to the options translation layer"); `crates/costs/src/rate.rs:8-11`, `regime.rs:322-345` | `CLAUDE.md` §3 rule 1: "Every claim about … a cost is traceable to a source recorded in `docs/00-charter.md`." Every `costs` rate and dated row cites the predecessor repository's `COSTS_VERIFIED` §n, plus circular numbers given in source comments. The charter records none of them. `docs/06-limits.md` §26 admits the predecessor's citations "are the whole warrant". The rates themselves are standard published figures, but the source trail the rule requires is outside this repository. | grep output above: the charter has no cost-rate source. `docs/05-decisions.md` D-0041 records the port, not the sources. | NEW (partially DOCUMENTED: 06-limits §26 "Not measured", which states the warrant but not the rule-1 gap) |
| hunt-costs-6 | info (doc-false) | costs | `crates/costs/src/lib.rs:65` | "See `docs/05-decisions.md` D-0039" names the record encoder decision (`05-decisions.md:2214`). The `costs` crate decision is D-0041 (`05-decisions.md:2498`). Also stale: lib.rs:5-7 "It does not yet compute a charge, a GST total or a net P&L" and rate.rs:236-238 "the arithmetic … is stage 2's". Stage 3 (`trip.rs`) does both. | quoted lines | NEW |
| hunt-costs-7 | info | costs | `crates/costs/src/fill.rs:536-545` (`worst_case_fills` sell floor) | The sell floor lifts a sub-tick sell to `TICK`, which is a better fill than the bar printed. It then reports the lift as extra slippage: `realized = tick + |anchor - fill|`. A low of 0 gives `sell=5` and `realized_slip_per_unit=10`, where the true economic slip on that leg is -5 + 5 = 0. The same file's `PrintedExtreme` arm (fill.rs:437-448) refuses a sub-tick low because "flooring it upward would flatter the seller", so the two anchors disagree on this rule. No production caller exists (`runner/src/trade.rs:3295-3296` asserts that runner does not use `AdverseExtreme`). | code quoted; consistent with fill.rs's own test (`if low >= -100 { assert_eq!(worst.map(Fills::sell), Ok(TICK)) }`) | NEW (inert) |
| hunt-costs-8 | info | costs | `crates/costs/src/regime.rs:316-326` (STT anchor 0.0625% from 1990-01-01) | The pre-2023-04-01 0.05% options STT is not encoded (UNVERIFIED in the row's own text). It cannot be reached through `trip::price`, because the NSE exchange charge refuses every date before 2024-10-01. It can be reached through the public `stt_options_rate`. | `straddling 2024-10-01: Err(Unverified(Refusal { charge: "exchange transaction charge (options premium)" …` (probe 1) | DOCUMENTED: 06-limits §25 "Two dated windows…" item 1 |

### Re-verified prior items

| prior id | status at 1087e54 | evidence |
|---|---|---|
| probeengine-1 (`PrintedExtreme` low `i64::MIN` overflow at trip.rs:938) | FIXED-since-prior | fill.rs:454-459 `if sell.raw() < TICK.raw() { return Err(CostError::BelowTick { quantity: "sell printed-extreme fill", .. }) }`. The trip.rs:935-946 comment now states the invariant. |
| W1-greeks1-0 (discount factor overflow) | FIXED | bsm.rs `check`: `if ![carry_discount, forward, discounted_strike].iter().all(..is_finite) { return Err(NotRepresentable) }`. Probe: the underflow direction (q=10, T=100) gives `call=Ok((0.0, 0.0, 0.0))`, a correct limit with no refusal needed. |
| W1-greeks1-1 (bisection ulp) | FIXED | solver.rs `BISECTION_STEPS = 75`, `MAX_ITERATIONS = 2+8+75+1 = 86`, matching the lib.rs doc "86 model evaluations". |
| W3-costs1-0 (weekly expiry projected across a regime row) | FIXED | expiry.rs `next_weekly_on` loop re-reads the regime at each row start crossed. The monthly path re-reads the regime at the rolled month's 15th (expiry.rs `next_monthly_on`), so it has no equivalent defect: NIFTY asked 2025-08-29 rolls to Sept, which reads Tuesday. |
| o1engine-42..48 (costs and greeks hot paths) | still true | See the table below. |

## Greeks: formula check (by reading, then probe)

- d1 = (ln(S/K) + (r − q + σ²/2)T)/(σ√T) and d2 = d1 − σ√T match the textbook.
- Price, delta, gamma, vega, theta (per year) and rho for the call and the put match the textbook BSM-with-yield forms, where `forward` = S·e^{−qT}, the dividend-discounted spot. That variable name is misleading but the arithmetic is right.
- The normal CDF is Hart 5666 / West (2005), with the small tail computed directly.
- Probe parity: `worst |C-P-(Se^-qT-Ke^-rT)|/(S+K) = 1.94e-16`, delta parity under 1e-12 in all 700 cases.
- Edge cases from the probe:
  - T=0 → `Expired`; T=-0 → `Expired`; NaN → `NotFinite`.
  - σ=0 → `NotPositive`; S=0 and K=0 → `NotPositive`.
  - IV of NaN or inf → `NotFinite`.
  - A quote at or below intrinsic → `PriceBelowIntrinsic`.
  - A quote below the σ=1e-6 price → `OutsideVolatilityRange`.
  - The ATM quote 7.9656 → `Ok(0.2000000000000002, Newton, 6)`.
- T=5e-324 or σ=1e-300 at the money gives price 0 with gamma about 1e159 to 1e297. That is finite and mathematically the limit, so it is not refused.

## Hot-path table (O(1))

| path | file:line | unit | cost | verdict | note |
|---|---|---|---|---|---|
| `RegimeTable::rate_on` | regime.rs:166-194 | per lookup | loop over a fixed `[Option<Row>; 2]` | O(1) | compile-time bound |
| `DatedTable::value_on` (lot, step, expiry) | dated.rs | per lookup | fixed `MAX_LATER_ROWS` | O(1) | — |
| `next_weekly_on` | expiry.rs `next_weekly_on` | per call | at most `MAX_LATER_ROWS+1` passes, each a fixed array walk | bounded | stated in the module doc (D-0770) |
| `charge_stack` / `price` | trip.rs:1031, 1185 | per trip | straight-line i128 arithmetic | O(1) | the 06-limits §27 bench numbers are the operator's, not re-measured here |
| `swept_slot` | venue.rs | per call | linear over a 2-entry `SWEPT`, string compare | O(1) (fixed 2) | — |
| greeks closed form | bsm.rs `Checked::greeks` | per eval | fixed exp/ln/CDF | O(1) in code | exp/ln timing UNMEASURED (06-limits §29) |
| IV solver | solver.rs `search`/`bracket` | per solve | 1 to 86 evaluations | bounded, not flat | DOCUMENTED 06-limits §29 |
| `Sink::emit` filtered | sink.rs:1089-1108 | per event | one relaxed load and compare, plus a bounded `level_for` over ≤8 targets | O(1) | — |
| `Sink::emit` written | sink.rs:1109-1222 | per event | mutex, encode capped by MAX_* bytes, one write; a roll does ≤ `2·keep_files` renames | bounded | DOCUMENTED 06-limits §46 |
| `tail` | tail.rs:480-560 | per query | O(scanned bytes) ≤ `max_scan_bytes` | NOT O(1), bounded by budget | DOCUMENTED (prior o1store-45) |

## Gate 17

- `crates/{vocab,engine,indicators,runner}/Cargo.toml` `[dependencies]` lists no `telemetry`.
- grep for `telemetry::`, `emit_if!`, `println!` or `eprintln!` in their `src` (comment lines excluded) finds nothing.
- CI step at `.github/workflows/ci.yml:2980` "Gate 17 — nothing logs from inside the sweep".

**HOLDS.**

## Probes run, then deleted

1. `crates/costs/tests/zz_audit_hunt-costs_1.rs` (2 tests, ok)
2. `crates/telemetry/tests/zz_audit_hunt-costs_2.rs` (3 tests, ok)
3. `crates/greeks/tests/zz_audit_hunt-costs_3.rs` (1 test, ok)

All three were removed. `git status --porcelain | grep hunt-costs` is empty, and the temporary directories were removed.
