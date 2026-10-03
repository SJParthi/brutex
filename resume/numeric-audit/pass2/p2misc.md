# p2misc — numeric pass 2 (origin/final/all-fixes-zero @ 5140aca)

Slice: crates/cli/src/{pool, results, result_set, trades, minute_gaps, vix_reference, columns, sweep_evidence, live, v6_statistics_adapter, research_policy, research, knobs, strict_range_knobs, audited_range}.rs (non-test code).

Verdict: 2 new low findings, both probed. No rounding-direction, overflow, divide-by-zero or look-ahead defect found in the slice.

Probe: `$SCRATCH/probes/p2misc/` (path deps on crates/cli and crates/indicators), `CARGO_TARGET_DIR=$SCRATCH/target cargo run -q`. Verbatim output:

```
15:29 missing on day 19522: holed days = []
09:15 missing on day 19522: holed days = []
11:55 missing on day 19522: holed days = [19522]
dd(A)=200 dd(B)=0 pool dd_bound=max=200 merged pooled dd=100
```

## Findings

### p2misc-1 (low): `cli pool`'s `dd>=` column says it is a lower bound on the pooled drawdown, and it is not one

- Where: crates/cli/src/pool.rs:41-42 (module doc), :146-148 (field doc), :928 `p.dd_bound = p.dd_bound.max(cell.max_drawdown);`, :971-972 (printed legend "a lower bound on the pooled drawdown, not the figure itself"), :195 (it is the second sort key, right after "rule met"). docs/06-limits.md §171 (line 8495) repeats the claim: "the LARGEST single-instrument drawdown among the cells pooled — a lower bound on the pooled figure".
- Why it is wrong: the pooled drawdown, as §171 defines it, is the drawdown of the merged, time-ordered sequence of every instrument's trades. Another instrument's win landing between two losses on the first instrument cuts the merged drawdown below that instrument's own. So the largest single-instrument drawdown can be ABOVE the pooled figure. It can also be below it, when losses line up across instruments. It bounds the pooled figure in neither direction. The code is not miscomputing anything. What is false is the guarantee the report prints, and the drawdown-led ranking uses that number as if it were a guaranteed bound.
- Repro (the probe uses the same drawdown fold as runner/src/grid.rs:3014-3027, with the peak starting at 0): instrument A has trades -100 at t1 and -100 at t3, so dd(A) = 200. Instrument B has +150 at t2, so dd(B) = 0. `dd_bound` = 200. The merged sequence -100, +150, -100 has equity 0, -100, 50, -50, so its drawdown is 100. The column printed 200 and called it a lower bound; the true figure is 100.
- Fix: relabel `dd>=` as "largest single-instrument drawdown (bounds the pooled figure in neither direction)", or compute the merged sequence exactly (§171 already names that change).

### p2misc-2 (low): the minute-gap detector misses a hole at the start or end of a session, so a truncated 15:29 still fails the whole span

- Where: crates/cli/src/minute_gaps.rs:206-223 (`days_with_interior_gaps`) is the only detector. Callers: pool.rs:848-851 and the other `withhold_holed_days` users.
- Code:
  ```rust
  if same_day && step > MINUTE_MICROS && days.last() != Some(&day) {
      days.push(day);
  }
  ```
- Why it is wrong: a hole is only detected between two bars on the same day. A day whose final 15:29 minute is missing, or whose session stops early (GAP12-5 names both "15:29 missing" and "the day stopping at 15:16"), has no interior step, so it is never withheld. The exact-minute join then clamps the last bucket's target to the calendar close and demands that exact minute (indicators/src/anchored.rs:836-858), and returns `MissingClosingMinute`. One such day anywhere fails every coarse rung over the whole span. Stopping that is the module's stated purpose (minute_gaps.rs:28-33: "One such minute anywhere in the span refuses the WHOLE span"). A missing 09:15 is missed the same way. It is harmless for the closing-minute join, but it is still a holed session that gets swept. This fails loudly rather than silently, so the severity is low.
- Repro (probe, output above): one day with minutes 09:15..15:28 (15:29 dropped), followed by a full day, gives `[]`. Minutes 09:16..15:29 (09:15 dropped) gives `[]`. Control: 11:55 dropped gives `[19522]`. The step from the missing 15:29 to the anchored refusal is INFERRED from the code at anchored.rs:836-858 and was not run end to end.
- Fix: also compare each day's first and last stored minute with the calendar session bounds (`pull::calendar::kind_of`, already the authority used for `session_close`). Bump `MINUTE_GAP_POLICY`, because withholding more days changes run identity.

## Checked and clean

- pool.rs `tail_bp` / `profit_factor_bp`: floored hundredths compared with `>= rule_bp` (integer). floor(x) >= r iff x >= r, so the floor hides no breach. Never-lost gives i64::MAX and is demoted by `ranked`. A row with zero wins gives min_win 0 and fails. A row of only flat trades gives MAX and "RULE MET". That matches the single-cell `reward_to_risk_bp` (grid.rs:592-598) and is the same class as run2-2, so it is not new. `ratio_cell` cannot see a negative value because both ratios are >= 0. The fold is one pass over I×U, as documented in §171.
- pool.rs `price_all`: holed days are withheld from the signal span only. That is documented and correct (the execution minutes are used only for validation and gap detection, and the exact-minute context is reloaded from the withheld span).
- vix_reference.rs: exact-slot lookup with an O(1) fixed civil-month index. It never takes the nearest minute; the callers (global_replay*, index_stop_vix) stamp entry/exit for reference only and never use it to admit, rank or filter. No look-ahead path. Duplicate and reordered stamps are refused.
- research_policy.rs `display_limit`: exact integer ppm→%/× formatting. Every ppm field is parsed as u64, so the negative-remainder formatting case cannot be reached. Risk multiples go through i128 with checked narrowing.
- research.rs `months()` (`Δyear*12 + through.month`) is correct only because `from` is fixed at 2020-01. `contains` is half-open [start, end). `midnight_micros` arithmetic stays in range.
- trades.rs `Period::bucket`: the Week/Weekday epoch-Thursday shift is checked (day 0 → weekday 3, 1970-01-05 → week 1). Euclidean div/rem are used. Quarter/Half run on a 1-based month. The `Bucket::take` seed-from-first-trade is correct. i128 sums cannot saturate.
- results.rs / result_set.rs / trades.rs / sweep_evidence.rs codecs: `stride_of` is never 0 and `index_of_identity` guards it anyway. Orphan-byte refusal. `(count-1)` is guarded by `count > 0` / `count == 0`. Page `offset*N` is reached only when `take > 0`, so offset < count.
- live.rs: `expected_rewrites` (count model, float sanctioned, keep==0 and weighed<=keep guarded). Its comment mentions a `max(1)` the code does not have, but keep+displacements >= keep holds anyway (cosmetic). `bar_milli` truncation is the known run1-3.
- columns.rs: width arithmetic is saturating, no numeric issue.
- v6_statistics_adapter.rs: dispatch only. No float rounding or storage of statistics.
- knobs.rs / strict_range_knobs.rs / audited_range.rs: parse filters are strict, source-count arithmetic is checked, and the record ceiling is checked before allocation.
- minute_gaps.rs `withhold`: HashSet membership, O(1) expected per bar, as documented.
