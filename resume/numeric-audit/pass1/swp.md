# swp — numeric / look-ahead / O(1) audit (pass 1)

Slice: crates/cli/src/candidate_universe.rs, stored.rs, step3_orchestrator.rs, fold_audit.rs
Commit: 331b05c (origin/final/all-fixes)

## Verdict

NO NEW FINDINGS. None of the four files' production code uses floating point (no `f64`/`f32` outside
tests). All count and offset arithmetic is checked (`checked_*` with a named refusal) or bounded
by a guard checked earlier. Every per-bar, per-row and per-candidate loop is O(1) per step, or is a
one-time O(n) verification per universe or month, which the existing docs already cover. No
probes were run, because no candidate defect survived reading the code against the source.

## Findings

None.

## Checked and clean

- stored.rs:117-130 `CalendarExclusion::excludes`: a `.position` over the fixed `[i64; 9]`
  `CHARTER_NON_REGULAR_IST_DAYS`. Bounded at 9, so O(1). Already listed in docs/06-limits.md:13734.
- stored.rs:1161-1234, 1236-1278 `refuse_uncalendared_withheld_bar` / `expected_buckets_v2`: the
  bucket union is rebuilt for each bar, but over at most `MAX_WINDOWS`, so O(1). It runs only for
  bars on charter days.
- stored.rs:1280-1370 `hash_offered_calendar_v2` and :386-450 `_v1`: strictly increasing check;
  offered and measured counts use checked_add; the per-timestamp bucket geometry is bounded.
- stored.rs:2329-2336 span ceiling vs :2818 `admitted_records`: the ceiling is checked against the
  raw header count, and admitted counts bars kept after the charter filter. Held bars <= ceiling
  holds. Each month's allocation n <= remaining. The documented contract (:2201-2206) holds.
  Dropped as a false positive.
- stored.rs:2535-2594 `months_between`: endpoints checked to 1..=12, saturating month count has no
  wrong-value path once endpoints are valid, and MAX_SPAN_MONTHS is checked before the walk.
  next_month / previous_month: no overflow (year<9999 guard, checked_sub).
- stored.rs:2957-3020 daily context: daily records with `day >= last_signal_day` are dropped, so
  there is no same-day or future daily bar. Day-coverage check uses a monotone cursor (O(1) amortised).
- stored.rs:3172-3260 exact minute context: cadence delta uses saturating_sub but the `< 60s`
  check catches negative and duplicate values. Prior-session terminal geometry comes from the
  calendar, not the data.
- stored.rs:2290 / fold_audit.rs:402 `fnv1a(..) as u32`: intended low-32 store id, with an `expect`
  that gives the reason.
- candidate_universe.rs:1516-1600 `CandidateSearchColumnBuilderV1::build`: no look-ahead. The daily
  prefix is strictly `< final_signal_day`, and the minute prefix is `<= session close minute` of
  the last prefix bar, which must be present exactly. Monotone cursors; rebuilt once per
  walk-forward fold (bounded by search_splits).
- candidate_universe.rs:3996-4049 `session_close_minute_v1`: whole-minute and overflow checks,
  bounded window walk, clamps to the session window end.
- candidate_universe.rs:3930-3973 `require_exact_execution_subspan`: one O(n) linear position
  and digest per source verification, not per candidate.
- candidate_universe.rs:1000-1068 `validate()`: rehashes the streams (O(bars)), but it is called
  a constant number of times per production or replay operation (lines 955, 1084, 2753, 4333,
  4492), never per row.
- candidate_universe.rs:3380-3410 reopen: completion audits are indexed in a `HashMap`, with no
  linear search. Page and complete-read use positional reads with checked indices.
- candidate_universe.rs:4327-4500, 4608-4720 Execution V3 replay: series hoisted once per block
  (D-0990); per-cell `classify_coordinate` is O(1) (runner/exit_grid_policy.rs:2561).
- candidate_universe.rs:5039 per-cell replay over a shared CellReplay (documented D-1141).
- candidate_universe.rs:6145-6170 `record_count`: ragged and maximum checks; strides are constants.
  :6330-6363 orphan scan: `total - committed` only after a successful read at `committed`.
- candidate_universe.rs:5400 `within - long_cells_per_mask` is guarded by the `<` branch.
- step3_orchestrator.rs:1229/1287 `join_population_v5_inputs` zip: lengths are checked equal
  before the call (:1222), so zip cannot silently truncate.
- step3_orchestrator.rs:988-1004 family aggregates (including `aggregate_oos_paisa`): checked_add, i64.
- step3_orchestrator.rs:3350 `rung_seconds`: positive whole-second check before division.
- step3_orchestrator.rs:3630 eligibility count: checked.
- fold_audit.rs:197-325 two-cursor compare and :333-355 order check: already covered by
  hunt-cli-b and o1surface2, and still consistent with the code.
