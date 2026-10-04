# conc-pass7: state and determinism audit (run identity, idempotence)

Head: `origin/final/all-fixes-zero` at **1f4de71** (`/home/claude/wt/zero3`, detached, read-only). I read source only and ran no cargo. Both new findings are low.

**Verdict:** 2 new findings (0 high, 0 medium, 2 low): conc7-1 and conc7-2. The nine-term `RunId` construction is sound. Every term is tagged and length-prefixed or fixed-width, all nine are present, and the extension framing cannot collide with the legacy framing. Every operator knob on the recorded screen and audit paths that changes the answer is folded into `Params` (min_hits, ceiling, pair budget, policy). The two defects are in what sits around the identity, not in the hash. conc7-1: the results ledger compares request metadata (the span) that the identity does not carry, so two identical computations collide and the second is refused. conc7-2: the batch verb claims that its identity equals `sweep-stored`'s identity, which is false. HashMap order, rayon fold order, engine drain lanes and float reduction order are clean. Every remaining nondeterminism is an already-known id.

Known ids I re-saw and do not re-report: hunt-conc-3, hunt-conc-5, hunt-conc-8, recovery-3, determinism-1..3, cli2 `latest_for` key (concurrency.md:1335), the WORKERS/MEMORY halt as permanent outcome (concurrency.md:42), the build.rs commit-stamp TOCTOU (concurrency.md:1491), and the screen-budget wall-clock cap (refused on recorded runs, D-0685).

---

## Table A: identity construction (`crates/runner/src/identity.rs`)

| Term | Site | Framing | Verdict |
|---|---|---|---|
| 1 mask | :718-722 | fixed `WORDS*8`, LE, sized from `vocab::mask::WORDS` | OK |
| 2 direction | :725 | 1 byte, tagged + len | OK |
| 3 instrument | :735-775 | exchange/segment/underlying each u32-len-prefixed; `kind` discriminant then fixed-width variant fields, whole blob len-prefixed | OK |
| 4 timeframe | :778 | `term` (tag + u32 len) | OK |
| 5 params | :782-804 | legacy: 32 bytes fixed. Extension: tag, len = 40 + descriptor, params, 8-byte domain, descriptor. The length differs from legacy, so no overlap | OK |
| 6 data_digest | :807 | 32 bytes, `term` | OK |
| 7 vocab_version | :810 | LE, `term` | OK |
| 8 commit | :817 | `term` | OK |
| 9 feed | :823 | `term` | OK |
| `data_digest` | :345-362 | len u64 + 7 fields per bar, fixed width | OK |
| `data_digest_with_execution` | :397-426 | domain-prefixed, tagged terms. `None` returns the legacy digest on purpose | OK |
| `data_digest_with_daily_reference` | :552-648 | domain v2, 13 tagged terms, excluded days hashed with count prefix | OK |
| `Params::with_policy` | :243-256 | count-prefixed u64 list, BLAKE3, **truncated to 64 bits** | Info: a collision needs about 2^32 distinct policy vectors under one otherwise-equal identity. Not reachable by any knob set; recorded, not a finding |
| `term` length | :662-667 | `u32::try_from(len).unwrap_or(u32::MAX)` | Info: saturates only for terms of 4 GiB or more; no term can be that large |

## Table B: production identity sites (cli and runner)

"Inputs outside identity" lists only operator- or environment-controlled inputs. Anything derived from bars is fixed by `data_digest`.

| Site | Params | Inputs that change the result: in identity? | Verdict |
|---|---|---|---|
| lib.rs:6230 `audit-stored` | `of(ladder).with_policy(policy_of(..))` | rules (13 terms incl. protective exits, fill headroom, avg RR), lens, validate, fold rungs, screen cap, ceiling, grid step, horizon, budget (refused), stated stop via `Rules::derived` → `max_mae_ppm`: all in | OK |
| lib.rs:7107 `audit-range` | same | same. **Span (from/to/asked/found) is not in identity but is compared on rerun** | conc7-1 |
| lib.rs:16389 descend / `one_rung` | same | same as 7107 | conc7-1 |
| audited_range_command.rs:210 | `policy_of` | same set; checksum guard bound into digest | OK |
| lib.rs:3590 `sweep-stored` | `stored_month_params` (+MINUTE_GAP_POLICY on ordinary door) | ladder plus gap rule; execution-bound digest | OK |
| batch.rs:671 `sweep-all` | `Params::of(ladder)`, BATCH_CEILING | ladder; anchored (not execution-bound) digest | OK as an identity, but its comment is false (conc7-2) |
| lib.rs:1094 preparation | `with_policy([FOLD-V1])` | domain constant | OK |
| lib.rs:4159/4188 auto + probes | AUTO-V1 / `Params::of(probe)` | ladder per probe | OK |
| expression.rs:67 | `Params::of(min_hits 1)` + BTXEXPR1 descriptor | expression bytes in params extension | OK |
| expression_search.rs:228/578 | `min_hits` + pricing plan words, BTXEXS01 cursor | cursor initial descriptor, plan words | OK |
| validate.rs:2571 (V4 folds) | `with_policy([4, signal_len, horizon, coord_digest])` | fold ladder, horizon, coordinate order | OK |
| candidate_universe.rs:1395 / 4342 | `[EXEC_POLICY_V1, rung_s, horizon]` | exit resolution bound separately by `evaluation_spec_fingerprint` / resolution digest | OK (layered) |
| index_stop.rs:250 | literal `{0, records, programs, 1}` | bootstrap draws/seed/block bound in `PopulationStatisticsProcedureV2` (index_stop_launch.rs:91-106), not the run id | OK (stage-local) |
| boolean_candidate_v1 479/732, boolean_oos_v1 339/538, global_replay 3898, strict_v6_inputs 229, selection_v4_authority 981, population_admission_writer 1995, institutional_evidence 2310, exit_grid_policy 436/5172, signal_candle_stop 926/989 | constants or ladder; catalog words where used | spot-checked: stage policies are bound in their own ledger descriptors | No new defect found |

Env knobs checked against `policy_of` / `Params`: MIN_*_BP (7), MAX_MAE_PPM, MAX_STOP_POINTS (through `Rules::derived` → `max_mae_ppm`), MIN_TRADES, TOP, PROTECTED_EXITS, MIN_FILL_HEADROOM_BP, MIN_AVG_RR_BP, GRID_RUNGS, GRID_RESOLUTION, HORIZON_BARS, SCREEN_CAP, SCREEN_BUDGET_MS, CEILING, VALIDATE, SUPPORT_PPM and SIZING_RATE_BP (both through `min_hits`). None is missing. `available_parallelism` reaches `grid_rungs` and `whole_machine_ceiling`, and both resolved values are folded in.

## Table C: output writers and order or rerun behaviour

| Writer | Order source | Rerun behaviour | Verdict |
|---|---|---|---|
| results ledger `Results::append` (results.rs:1222) | append; `ensure_run_record` (lib.rs:17718) | same id: compare all fields except `finished_micros`. Equal gives `Reused`; differing gives a refusal | Works for true reruns. conc7-1 for a span-only difference |
| frontier / trades / detail receipt (lib.rs ~20700) | sorted top (`rank::ordered`, total `Ord` with mask tiebreak); trades in time order | prepared, then verified before reuse | OK |
| `rank::offer_part` (rank.rs:694) | chunk count from `current_num_threads`; every `Ord` ends in `mask.words()` | partition-independent | OK |
| `engine::drain` (engine lib.rs:2330) | counts per lane, then serial append in batch order | thread-count independent | OK |
| `trades::buckets` (trades.rs:2485) | `HashMap::into_values` then `sort_unstable_by_key(key)` | deterministic | OK |
| api `spot_months_by_identity` consumers (server.rs:2911/34774) | keys collected and sorted | deterministic | OK |
| api `master::skipped_by_reason`, `constituents` claims | sorted | deterministic | OK |
| sweep_evidence attempts | one new token per run (append) | by design: an attempt log, not a result | OK |
| rayon reductions | none (`sum`/`reduce`/`find_any` absent on parallel iterators in non-test code) | n/a | OK |
| wall-clock in result bytes | only `Record::finished_micros`, which is excluded from the rerun comparison | n/a | OK (known non-finding) |

---

## conc7-1 (low): range-audit identity omits the requested span, but the rerun check compares it, so two identical computations collide and the second is refused

- **Where:** `crates/cli/src/lib.rs:7107-7124` (identity), `:7206-7216` (Recording), `:17829-17836` (Record fields), `:17692-17696` (`same_run_answer`), `:17729-17733` (refusal). The same pattern is at `:16389`/`:16440` (descend / `one_rung`).
- **Code:**
  ```rust
  params: Params::of(ladder).with_policy(&policy_of(&span.bars, rules, lens, validate, horizon, rungs)),
  data_digest: *executed_digest,
  ...
  from_year: into.from.0, from_month: into.from.1, to_year: into.to.0, to_month: into.to.1,
  months_asked: into.months_asked, months_found: into.months_found,
  ...
  fn same_run_answer(mut left, mut right) -> bool { left.finished_micros = 0; right.finished_micros = 0; left == right }
  ```
- **Why it is wrong:** `load_span` tolerates absent months (stored.rs:2817 pushes them onto `missing`, and the report names them at lib.rs:6510). The executed digest hashes only the bars actually loaded (signal, 1-minute, daily, eligibility), so a span widened by months that hold nothing on any rung produces the same `RunId`. The Record carries `from_*`, `to_*` and `months_asked`, and `same_run_answer` compares them. So the second request computes a byte-identical answer, finds its identity already recorded, sees the span fields differ, and is refused: "its deterministic fields differ from this exact rerun. The existing row was kept and the new answer was refused." That message is false, because the computations were identical. This breaks §3 rule 5 in the opposite direction from the usual collisions: same computation, refused rerun. The requested span then has no row, so a span-keyed read (`latest_for`) finds nothing for it.
- **Repro (not run, from source):** pick a feed whose history for NIFTY starts at 2021-01. Run `cli audit-range <feed> NIFTY 15min 2021 01 2021 03 <h>`, which writes row R. Then run `cli audit-range <feed> NIFTY 15min 2020 12 2021 03 <h>`. 2020-12 is absent at 15min, 1min and 1day, so `missing = [2020-12]`, the bars and digest are unchanged, and the `RunId` is equal. `ensure_run_record` → `verify` → `same_run_answer` is false (`from_month` 12 ≠ 1, `months_asked` 4 ≠ 3), so the command reports NOT RECORDED. The common real case is a trailing span: `to` set to the current month before any of it is pulled, versus `to` set to the previous month.
- **Minimal fix:** pick one of two.
  1. Fold the requested span into identity: append `from`, `to` and `months_asked` to `policy_of` (positionally appended, per its contract).
  2. Leave span metadata out of `same_run_answer`: compare only computed fields and report `Reused` with the original row's span.

  Option 1 matches how the record is keyed by `latest_for`.

## conc7-2 (low): `batch` claims its identity equals `sweep-stored`'s for the same month; it never does

- **Where:** `crates/cli/src/batch.rs:666-670` and `:801-805`.
- **Code:**
  ```rust
  // ... Same construction as `sweep_stored`, so sweeping a month here and sweeping it alone
  // produce the same 64 hex characters, which is the only thing that makes the two reports comparable.
  let id = runner::identity::identity(&Run { params: Params::of(ladder), data_digest: digest /* stored_anchored_digest */, .. });
  ```
- **Why it is wrong:** `sweep_stored` (lib.rs:3585-3609) uses `stored_executed_digest`, which always wraps the anchored digest with `bind_execution_digest` (lib.rs:2639-2657), even when the execution slice is the signal series. It also uses `stored_month_params`, which adds MINUTE_GAP_POLICY on the ordinary door. Batch uses the bare `stored_anchored_digest` and `Params::of`. The digests and policies differ for every month, so the 64 hex characters never match. The comment states as fact a comparability that cannot exist (§3 rule 6). Both verbs also append rows for the same (feed, underlying, rung, month) under different identities.
- **Repro (not run, from source):** compare batch.rs:619 with lib.rs:3585 and 2645, and batch.rs:685 with lib.rs:3602/3538.
- **Minimal fix:** correct both comments to say that batch keys the anchored, non-withheld computation and that its identity intentionally differs from `sweep-stored`'s. If comparability is wanted, build batch's identity through `stored_month_params` and `stored_executed_digest`, which also needs the execution series loaded.

---

## Checked and not a finding

- **`policy_of` positional append contract** (lib.rs:10623-10820): 20 terms, appended only. Negative rule values cannot occur because `Rules::stated` uses `nonnegative_floor`. `from_ne_bytes(to_ne_bytes())` is a bit reinterpretation and does not depend on endianness.
- **`AuditCache`** (lib.rs:6753-6850): scoped to one descent. Its key includes root, vendor, underlying, rung, span and commit, and the cached digest always matches the cached bars.
- **Validation V4 fold identity** (validate.rs:2571-2830): refusals are kept in (candidate, side) order. Parallel `collect` is indexed.
- **Index-stop bootstrap seed, draws and block** (index_stop_launch.rs:91-106): bound into the statistics procedure, which is its own stage's identity.
- **Cross-construction digest collisions:** the plain `data_digest` input (8 + 56n bytes) and the domain-prefixed constructions have different byte lengths and prefixes. No equal-bytes overlap exists.

## Verification tally (prior ids re-seen this pass)

| id | Status at 1f4de71 | Evidence |
|---|---|---|
| hunt-conc-8 (`SharedBy` global) | NOT FIXED (known, info) | `SWEEPS_SHARING_THIS_MACHINE` store/Drop unchanged (lib.rs:16914-16935); identity still honest because the divided ceiling is in `Params.ceiling` |
| hunt-conc-5 (lanes in report) | NOT FIXED (known, info) | index_stop_search.rs:206 `available_parallelism` |
| recovery-3 | not re-checked this pass | out of slice |
| `finished_micros` non-finding | CONFIRMED | excluded by `same_run_answer` (lib.rs:17692) |
