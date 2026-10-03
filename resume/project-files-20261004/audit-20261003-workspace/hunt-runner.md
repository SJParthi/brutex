# hunt-runner: financial and statistical correctness of `crates/runner/src` at 1087e54

## Verdict

The runner's statistics mostly match their textbook definitions. I checked each against its formula with file:line: the Wilson lower bound, White's Reality Check, Hansen's SPA recentring (g_c), the Romano–Wolf stepdown (2005 rounds and 2016 Alg. 4.1 cumulative max), the stationary-bootstrap mean block length, the Newey–West Bartlett cross-sum algebra, the PBO-style exact midrank and bottom-half rule, purge/embargo, anchored and rolling folds, Bonferroni via the tail quantile, and the run-identity framing. Two things also hold: trade timing (a signal on bar N enters on bar N+1 at the open for the best case and at the printed extreme for the worst), and the D-1410 fix that measures cadence on the prefix only.

I found one NEW medium-severity defect and proved it with a probe. On the **horizon time-exit bar** the grid credits a TARGET fill. That bar *opens at* the time-exit deadline, yet `ExitChoices::of` drops the `Time` exit whenever any order fired by that offset. So even the **pessimistic** reading (the one every selector ranks on) books the target level instead of the time-exit fill. On one probed trade the pessimistic P&L went from −75 to +440 paisa on the identical exit bar.

The other findings are low or info:
- a 5% boundary mismatch between White/SPA `clears()` and the exact Romano–Wolf rule in the same report (proved),
- `significance::p_value` tail inaccuracy (proved; no production caller),
- stale "325 cells" docs,
- SPA studentized with an i.i.d. rather than a long-run SE,
- a 64-bit truncated policy digest inside the run identity.

At HEAD, the earlier runner findings I re-checked are fixed. The probe files are deleted, and `git status --porcelain` shows no `hunt-runner` file.

## Findings

| id | sev | crate | file:line | what is wrong | evidence | status |
|---|---|---|---|---|---|---|
| hunt-runner-1 | **medium** | runner | `grid.rs:4305`, `grid.rs:5137-5143`, `grid.rs:3796-3810` (and `:2291-2298`, `:3269-3276`), `excursion.rs:1128`, `grid.rs:1754` | A fixed **target** that is first touched on the horizon time-exit bar is filled at its level, in BOTH readings. That bar is stamped exactly at the deadline (`trade.rs:1463` `horizon_bar` returns only the bar at `entry_ts + H·step`), and the time exit's best fill is that bar's OPEN (`trade.rs:1337` `Anchor::Open`; `grid.rs:1754` `return Some(bar.open)`). So any level touched inside that minute is touched after the deadline price. The crossing walk includes the exit bar (`crossings_checked(bars, path.entry_bar, path.exit_bar, ..)`, `for offset in 0..=to-from`). `pess_off = span.min(stop_at).min(target_at)…` equals `span`. Then `ExitChoices::of` sets `timed: no_order.then_some(Ended::Time)` with `no_order = !stop_fired && !target_fired && …`, which removes `Time` from the choices. `read_trip`'s pessimistic reading therefore picks the target and never the time exit. Even under the most generous reading (time exit and target both possible inside that minute), the pessimistic reading should be min(target, time-worst), not the target. Stops and trails on that bar are not affected, because they slip to `bar.low`, which equals the time-exit worst. **Effect:** an upward bias in `Cell::pessimistic` for target-carrying cells. That figure is what `Grid::best`, `sharpest`, `best_within`, the resolved V1 policy (`evaluate_resolved_policy_v1` → `one_variant`), walk-forward OOS (`with_levels_over_from`) and `per_trade` rows all rank or print. It is reachable on every horizon exit; forced 15:09 exits are the documented 15:09-candle convention and are excluded. | Probe `zz_audit_hunt-runner_1` (deleted), `synthetic::sessions(8)`, H=15, long, `ConditionMask::ZERO`. Only the first baseline trade's exit bar `high += 1000`. Output: `first baseline trade: entry_bar=1877 exit_bar=1892 … (exit-entry = 15 min)`; `exit bar (ts = entry + 15 min = deadline): open=2502051 high=2503111 low=2501991 ; max high on bars before it=2502174`; `target level = entry_open + 200ppm = 2502506`; `baseline row: exit_bar=1892 best=45 worst=-75`; `target   row: exit_bar=1892 best=500 worst=440`; `target cell: … pessimistic=-7419` vs `baseline cell: … pessimistic=-7934`; `trade::walk same trade: exit_bar=1892 best=45 worst=-75 forced=false`. The same exit bar is priced at +440 pessimistic against −75 for the time exit; the target fill (2,502,506) is above the deadline price (open 2,502,051). Searched `known/*.md`, `prior/*`, `docs/11-findings.md`, `docs/06-limits.md`, `docs/05-decisions.md` for exit-bar/time-exit/deadline crossings: no match. `docs/05-decisions.md:35072` documents only the forced-15:09 candle convention. The unit test at `grid.rs:8387-8420` (`time_exit: 2`, target firing on bar 2) exercises this shape and asserts only ambiguity counts. | NEW |
| hunt-runner-2 | low | runner | `bootstrap.rs:169-171`, `bootstrap.rs:995-1002`, `bootstrap.rs:1469-1475`; rendered at `audit.rs:1291`; called together at `cli/src/lib.rs:18007-18027` | One report judges three family-wise tests "at the same 5%" (cli's own comment), but they use different boundaries. `Verdict::clears` is `p_value < 0.05` (strict), while Romano–Wolf rejects when `(1+count)/(B+1) <= alpha` (inclusive; `admissible = floor((B+1)·alpha_ppm/1e6)`). When `p == 0.05` exactly, White and SPA print `clears 5% = NO` while the stepdown names the same strategy as rejected. In production `draws = max(2000, n).clamp(1000, 100000)` (`cli/src/lib.rs:4186-4194`), so an exact tie needs `(B+1) % 20 == 0` and `beaten = (B+1)/20 − 1`. That is possible but rare. | Probe `zz_audit_hunt-runner_2::probe_alpha_boundary` (deleted), one strongly positive series, `draws=19`: `white p=0.05 clears=false \| spa p=0.05 clears=false \| romano_wolf rejected at 50_000 ppm: [Rejected { strategy: 0, round: 0 }]` | NEW |
| hunt-runner-3 | info | runner | `significance.rs:318-320`, `:398-411` | `p_value(t) = 2(1−Φ(|t|))` uses A&S 7.1.26 erf (absolute error about 1.5e-7) and forms `1 − Φ`. Relative error grows in the tail, and the value reaches exactly 0 for t ≳ 8.3. For a multiple-testing procedure such as BH over millions of tests, where thresholds are around 1e-9, this is unusable. No production caller exists: the only non-test reference is a comment at `outcome.rs:2172`, and `benjamini_hochberg` has no production caller either. | Probe `probe_p_value_tail`: `t=6 … ratio=1.0036`, `t=7 … ratio=1.0065`, `t=8 p_value=1.3322676295501878e-15 reference=1.24419211485435e-15 ratio=1.0708`, `t=9 p_value=0e0 reference=2.2571768119077e-19` | NEW (no live impact) |
| hunt-runner-4 | info | runner (doc) | `significance.rs:94-95`, `:520`, `:1090-1095` | The docs say the exit grid is "325 cells at the shipped four rungs", and the reachability bound at `:520` (and the test literal at `:1095`) multiply by 325. The module's own test asserts `grid::variants(4,4,4) == 625` (`:631`), and `grid.rs:56-58` states 625. So the stated bound is understated by about 1.9x. | `assert_eq!(shipped, 625, "the shipped DEFAULT_RUNGS grid");` beside `/// … argmaxes over as many as 325 cells at the shipped four rungs` | NEW (doc drift) |
| hunt-runner-5 | low | runner | `bootstrap.rs:1597-1618` (`summarise`), used by `spa` `:551-640` and Romano–Wolf | Hansen (2005) studentizes by ω̂_k, a consistent estimate of the long-run (HAC/bootstrap) standard deviation. Here `standard_error = sqrt(sample variance / n)` is i.i.d. Under serially correlated returns this changes the size of the `−sqrt(2 ln ln n)` recentring gate, i.e. which strategies are dropped from the null. Observed and resampled statistics use the same scale, so the test remains a valid max-type test. The gate is the textbook deviation. | `standard_error: (variance / n as f64).sqrt()` with `variance = Σ(x−mean)²/(n−1)` | NEW (deviation, low) |
| hunt-runner-6 | info | runner | `identity.rs:243-252` | `Params::with_policy` folds the operator's policy choices into the identity as the first 8 bytes of BLAKE3, a 64-bit truncation. The expression extension path (`identity.rs:779-794`) explicitly refuses "a truncated hash folded into the legacy u64 policy field". Two policies collide with probability 2^-64 per pair; that is negligible, but it is a weaker binding than the rest of the identity. | `head.copy_from_slice(digest.get(..8)…); self.policy = u64::from_le_bytes(head);` | NEW (info) |

### Textbook checks that passed (no finding)

| area | file:line | check |
|---|---|---|
| Entry/exit timing | `trade.rs:982-1000`, `1323-1345`, `1463-1477` | Signal N → entry N+1 (the same index for a reprojected `Sourced::Fill`, where `align.rs:179-186` requires the exact close instant and the same IST day). Best = open, worst = printed extreme on both legs. The horizon exit only at the exact deadline bar. |
| Look-ahead in cadence | `outcome.rs:979-1029`, `trade.rs:1019-1031` | Prefix two-heap median per bar (D-1410). The one forward read (`out[i]=out[i+1]` when no gap has been seen yet) is the trade's own first held step, as documented. |
| Intrabar ordering | `excursion.rs:1128-1240`, `1264-1300`; `grid.rs:4003-4100` | Trail retreat is measured from the pre-bar peak; arming takes effect on the next bar; stop and target on one bar are recorded as ambiguous; pessimistic = min over reachable choices; a gap at the open owns the exit. |
| MAE/MFE | `grid.rs:4386-4440`; `excursion.rs:751-790` | MAE from `entry_pess`, MFE from `entry_opt`, both from prefix extremes up to `pess_off` (accepted bars only, because a hole at or before the exit un-prices the path). |
| Forward window | `outcome.rs:620-690`, `769-935` | The monotone deque is correct, including the rebuild branch for a backward query; excursions over `[i+1, exit]`. |
| Newey–West | `outcome.rs:1731-1735`, `1803-1855`, `2251-2258` | Weights `a_o=(H−s)+o` and cross = A − m·B + m²·C are algebraically exact; SE = sqrt(S/(n−1)/n). |
| White RC / SPA / RW | `bootstrap.rs:415-509`, `551-640`, `1264-1395`, `1454-1570` | max √n·mean vs recentred max; SPA g_c (drop the very negative strategies, no recentring); RW bar = (B−a)-th order statistic ⇔ `#{>x}+1 ≤ a`; Alg. 4.1 cumulative max over integer numerators; (k+1)/(B+1). |
| Stationary bootstrap | `bootstrap.rs:1663-1688` | Restart probability 1/block, so the mean block equals `block`; wraps; seeded SplitMix64 (deterministic). |
| PBO-style placement | `pbo.rs:190-224`, `98-103`, `143-148` | Midrank×2 = 2·better + tied − 1; bottom half ⇔ rank2 > n−1; `at_or_above_half` ⇔ bottom ≥ ceil(folds/2). It is documented as NOT CSCV (`pbo.rs:3-15`). |
| Wilson | `grid.rs:463-512`, `admission.rs:3370-3385` | Correct lower-bound formula; the table values (12/12 → 7575) recompute. |
| Splits | `split.rs:85-125`, `213-247`, `276-305` | Labels end at boundary−1+h < test_start; tests tile without overlap. |
| Walk-forward leakage | `validate.rs:4618-4650`, `4710-4760`, `4890-5045` | Side, exit rungs and ladders come from training only; the train execution prefix stops before the first test signal's stamp; OOS uses the training ladders. Rolling-fold cold-start is documented at `validate.rs:1909-1924`. |
| Rank/topn ties | `rank.rs:120-131`, `topn.rs:531-560`; `validate.rs:4790-4810` | Total orders end in mask/digest; validation takes the first strict maximum. |
| Run identity | `identity.rs:658-662`, `690-818` | Nine tagged, u32-length-prefixed terms. Mask sized from `vocab::mask::WORDS`. The instrument kind is framed. The extension params length (40+len) is distinct from the legacy 32. |
| Resample | `resample.rs:242-253` | The 09:15 anchor restarts daily; the bucket is emitted only once a later bucket is seen. |

## Prior findings re-checked at 1087e54

| prior id | status now | evidence |
|---|---|---|
| W3-runner3-6 (ratio bitmap refused SL+TP) | FIXED-since-prior | `grid.rs:2133-2153` `let _ = ratios;` (D-1140) |
| W3-runner2-7 (hole after exit un-priced the trade) | FIXED-since-prior | `grid.rs:4209-4232` `first_refused().map_or(.., \|hole\| hole <= exit_offset)` (D-1183) |
| AC-whp-cx-1 (asymmetry/path keys long-only) | FIXED-since-prior | `outcome.rs:1253-1262`, `1475-1482` side read from `mean_paisa < 0.0` (D-1178) |
| probeengine-2 (resample anchored at midnight) | FIXED-since-prior | `resample.rs:99-100`, `242-253` open anchor, daily restart (D-1430) |
| o1runner-4 / o1runner-5 (duplicate SliceFacts per fold) | FIXED-since-prior | `validate.rs:4963` one `test_facts` reused by `walk_over_from(.., &test_facts, ..)`; `validate.rs:4701` `forward_over(trade_train, horizon, &facts)` |
| o1runner-7 (RW recomputes per round) | FIXED-since-prior | `bootstrap.rs:1454-1570` single suffix walk, null stats computed once (D-0932/D-0973) |
| o1runner-11 (audit sorts all cells) | FIXED-since-prior | `audit.rs:983-990` `select_nth_unstable_by_key(shown - 1, ..)` then sorts the head |
| o1runner-2 | FIXED-since-prior per commit `23f88f0` (D-1188) | evidence is the commit message only; file:line UNVERIFIED |
| o1runner-1, -3, -6, -8, -9, -10 | UNVERIFIED this pass | out of scope of the statistical hunt; commits `ad4583c` (D-1177, o1runner-8), `069f045` (D-0931, o1runner-3) and `6a56942` (o1runner-9/-10) claim fixes, not re-read |

## Hot-path table (paths touched by this hunt)

| path | file:line | unit | cost | verdict | why / documented |
|---|---|---|---|---|---|
| `trade::walk_core` per signal | `trade.rs:894-1240` | signal | fixed reads plus one `HashMap` probe in `horizon_bar` | expected O(1) | hash probe; stated at `trade.rs:316-330` |
| `excursion::crossings_with` per bar | `excursion.rs:1128-1240` | held bar | O(arm rungs) per bar | bounded (rung count) | stated `excursion.rs:798-861` |
| `grid::one_variant` per candidate per cell | `grid.rs:4234-4480` | candidate×cell | indexed reads into `Crossings` | O(1) | — |
| `WindowExtremes::over` | `outcome.rs:645-690` | forward query | amortised | amortised O(1) | backward query rebuilds Θ(window); `outcome.rs:609-616` |
| `OverlapWindow::observe` | `outcome.rs:1803-1845` | hit | amortised | amortised O(1) | D-1174 |
| `prefix_median_steps_over` | `outcome.rs:979-1029` | bar (once per slice) | O(log g) | NOT O(1), bounded | the per-slice O(n log n) named in `docs/06-limits.md` (stated at `outcome.rs:972-978`) |
| bootstrap White/SPA/RW | `bootstrap.rs:415`, `551`, `1264`, `1454` | per run | O(B·N·S) | NOT O(1) (per-run boundary) | stated at each doc; UNVERIFIED by a bench |
| `portfolio::canonical_order` / `duplicate_ordering_key` | `portfolio.rs:550-575`, `641-660` | minute | ≤ 200² compares | bounded constant | fixed `MAX_INTENTS_PER_MINUTE = 200` |

## Probe record

- `crates/runner/tests/zz_audit_hunt-runner_1.rs`: run with `cargo test -p runner --test zz_audit_hunt-runner_1 -- --nocapture`. Output is quoted under hunt-runner-1. Deleted.
- `crates/runner/tests/zz_audit_hunt-runner_2.rs`: two tests; output is quoted under hunt-runner-2 and -3. Deleted.
- `git status --porcelain | grep hunt-runner` gives no output. The only remaining runner file, `zz_audit_o1eng2_3.rs`, belongs to another worker.
