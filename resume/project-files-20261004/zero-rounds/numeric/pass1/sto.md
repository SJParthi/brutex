# SLICE sto — crates/store, crates/lake (numeric / look-ahead / O(1)) — pass 1

Commit audited: 331b05c (origin/final/all-fixes). Audit only; nothing in the repo was edited.

## Verdict

The store and lake numeric boundary holds. Store does no snapping itself: it gets paisa `i64`s that were already snapped once by `core::price::Paisa::from_rupee_text_half_up` (pull) or `from_rupees_half_up` (lake). The bar write gate refuses negative prices, broken OHLC ordering, negative volume, and negative open interest other than the sentinel. Every offset and count is computed with `checked_*` or saturating arithmetic. Bar lookup by index is arithmetic, O(1). The one search, `first_at_or_after`, is a documented bisection (D-1434).

I found two new low-severity items. Both concern the **overlay** sidecar (`.ovl`: spot plus implied volatility), which is the one record kind with no write-boundary gate:

1. the store accepts any `Overlay`, including a negative spot and a negative IV;
2. its only producer, `pull::rolling`, silently files an unreadable IV cell as "absent" (`OI_NULL`). This is cross-slice: the code is in pull, but its output lands in this store.

Neither file kind has a production reader today (`FileKind::Overlay` appears outside store only at `pull/src/ingest.rs:2722`, the writer), so the damage is limited to what ends up on disk.

## Findings

### STO-1 — low — `Overlay` has no write-boundary sanity gate: negative spot and negative IV commit
- **Where:** crates/store/src/format.rs:1007-1009
  ```rust
  fn is_sane(&self) -> bool {
      true
  }
  ```
- **Why it is wrong:** `Bar::ohlc_is_sane` (format.rs:771) says the sign check belongs at the WRITE boundary, "the one every path crosses", and `Greek::is_sane` checks finiteness. The overlay's `spot` is paisa and its `iv_micros` is a volatility. Neither can be negative unless it is the `OI_NULL` sentinel, yet `append` accepts any value. In practice `pull::rolling::paisa` refuses a negative spot. It does not refuse a negative IV: `shift_six("-0.25")` is pinned by a test as "a sign is carried" (rolling.rs:1177), and that value reaches the disk (see STO-2's probe).
- **Repro (probe ran, `$SCRATCH/probes/sto`):** `BarFile::open_or_create(.., FileKind::Overlay, ..)`, then `append(&[Overlay{ts_micros: T0, spot: -5, iv_micros: -125_000}])`. Expected: refusal. Actual:
  ```
  overlay spot=-5 iv=-125000: Ok(Committed { first_index: 0, n_valid: 1 })
  overlay spot=i64::MIN+1: Ok(Committed { first_index: 1, n_valid: 2 })
  ```
- **Fix shape:** `is_sane`: `(spot == OI_NULL || spot >= 0) && (iv_micros == OI_NULL || iv_micros >= 0)`.

### STO-2 — low (cross-slice: pull) — an unreadable rolling IV cell is stored as "absent", and negative IVs round half-away-from-zero
- **Where:** crates/pull/src/rolling.rs:633 and :841-847
  ```rust
  iv_micros: iv.map_or(OI_NULL, |a| a.get(at).map_or(OI_NULL, micros_of)),
  ...
  _ => return OI_NULL,
  ...
  shift_six(text.trim()).unwrap_or(OI_NULL)
  ```
- **Why it is wrong:**
  - **Silent fallback.** A cell that is present but malformed (`"abc"`, `"12.5%"`, `true`, or a JSON number serialised in exponent form such as `1e-7`) becomes `OI_NULL`, which means "the vendor stated nothing". That is the fallback that hides a failure, which CLAUDE.md §4 bans. The same function refuses a bad spot cell, and D-0952 refuses a bad OI cell for exactly this reason ("passed ... straight through — which IS `OI_NULL`, so a number the vendor sent was filed as the absence it did not state"). IV was not covered. The unit test's comment (rolling.rs:1178, "AND ANYTHING THAT IS NOT A DECIMAL IS REFUSED rather than guessed at") describes `shift_six` returning `None`. The caller then turns that `None` into an ordinary absent value.
  - **Wrong rounding direction for negatives.** `shift_six` rounds the magnitude half-up and then negates. That is half-away-from-zero, not the half-up toward +inf that `core` uses for prices (`-0.005` gives 0 there). `"-0.0000005"` gives -1 where half-up gives 0. This only matters together with STO-1, because a negative IV is accepted at all.
- **Repro (probe ran, `$SCRATCH/probes/sto2`, real `pull::rolling::read` with the Dhan by-offset spec, `PriceScale::Rupees`, one bar, `iv:[<cell>]`):**
  ```
  iv cell          0.125 -> Ok, overlay.iv() = Some(125000) (raw iv_micros 125000)
  iv cell          "abc" -> Ok, overlay.iv() = None (raw iv_micros -9223372036854775808)
  iv cell        "12.5%" -> Ok, overlay.iv() = None (raw iv_micros -9223372036854775808)
  iv cell          -0.25 -> Ok, overlay.iv() = Some(-250000) (raw iv_micros -250000)
  iv cell   "-0.0000005" -> Ok, overlay.iv() = Some(-1) (raw iv_micros -1)
  iv cell    "0.0000005" -> Ok, overlay.iv() = Some(1) (raw iv_micros 1)
  iv cell           1e-7 -> Ok, overlay.iv() = None (raw iv_micros -9223372036854775808)
  iv cell           true -> Ok, overlay.iv() = None (raw iv_micros -9223372036854775808)
  ```
  Expected: `"abc"`, `"12.5%"`, `true` and `1e-7` are refused by name (as spot and OI are). `-0.25` is refused as not a volatility, or at minimum `-0.0000005` gives 0.
- **Not previously reported:** no hit in docs/11-findings.md, docs/06-limits.md, D-0952, or the audit-20261003-workspace reports. hunt-pull-2 covers the price snap-to-zero only.

## Dropped as known / false positive
- A Greek re-run with `-0.0` reports `AlreadyPresent` although the bytes differ. I reproduced it (stored row0 is `00..00`, the `-0.0` re-offer gives `AlreadyPresent`, and the overlap-plus-extension path committed row 2 as `..80`). This is already attackdata-2, and docs/06-limits.md:2210 records `0.0 == -0.0`. Not new.
- Lake `f64` to paisa on decimal ties. Probe: `1.235 -> 124`, `-1.235 -> -124`, `238.605 -> 23861`, `-238.605 -> -23860`, `-0.005 -> 0`, `0.005 -> 1`, `-0.125 -> -12`, `0.125 -> 13`. Every exact binary tie goes to +inf. The `1.235` pair follows the rounded product, which is the documented `a_decimal_tie_is_usually_not_a_binary_tie` limit (AU-PROBESTORE-6). Not new.
- hunt-store-1 (the bar door's `table_of` admits overlay and greeks geometries): still present at HEAD (`table_of` falls through to `Layout::KNOWN`). It is already reported and is a type-confusion item, not a numeric one. Not re-reported.

## Checked and clean
- Snap count: store performs no rounding. Prices enter as already-snapped `i64`. Lake converts each price cell exactly once, through `paisa_from_lake` to `core::Paisa::from_rupees_half_up` (floor plus exact-remainder compare, half-up on both signs). That function refuses NaN and infinities, and refuses `i64::MIN` exactly (so it cannot alias `OI_NULL`).
- OI sentinel: `format::OI_NULL == i64::MIN`, `Bar::oi()` maps it to `None`, and `counts_are_sane` admits it as the only legal negative. Lake refuses a PRESENT `i64::MIN` (`OpenInterestIsNullSentinel`) and maps a null to the sentinel. No sum or average of OI or volume exists anywhere in store or lake (grep for `+=`, `.sum(`, `fold(` found none outside tests).
- Volume: `i64`, negative refused at `survey`. No arithmetic is done on it in either crate, so it cannot overflow there.
- Offsets and counts: `Layout::offset_of`, `record_byte_range`, `covered_byte_range` and `block_byte_range` all use `checked_mul`/`checked_add`. `records_in_block` subtraction is bounded by the `block < blocks` proof. Bisection midpoint is `low + (high-low)/2`. `as` casts in non-test store code are const-context widenings (`u32`/`usize` to `u64`/`i64`), the deliberate `moneyness_steps` sign-extending decode, and `repair.rs` `usize` to `u64`. None is lossy. Lake uses `usize::try_from` for footer lengths and `i128` for the row-count sum.
- Bar lookup: `read_record(index)` is `offset_of` arithmetic plus a cached block verify, O(1). `first_at_or_after` is a bisection of at most ceil(log2(n+1)) reads, documented as not O(1) (D-1434, docs/06-limits.md). `already_stored` and `suffix_that_follows` run per ingest batch only, are documented, and are off the sweep.
- `Admission::admit`: month bounds and grid congruence. `ts + anchor` cannot overflow inside a month, and the width is nonzero.
- Greek row write gate: `is_finite` on all seven `f64`s. Statistics stay full `f64`, not rounded (§7).
- Lake greeks: full `f64`, the eight-value all-or-nothing null rule, and strike parsing by integer `checked_mul`.
- data_digest cost: lives in `runner::identity` (other slice). It is one linear hash per run, not per bar or per candidate.
- Look-ahead: neither crate makes a decision. Store reads by index or timestamp only, and lake decodes whole row groups for a caller (lake has no consumer in the workspace).
