# Numeric audit pass 1 — SLICE=core (crates/core, crates/telemetry)

Commit 331b05c (origin/final/all-fixes). Audit only; no repo file touched.

## Verdict

The money path, membership tables and telemetry hot path held. There are no new
high or medium findings. One low finding: a doc comment claims an order that the
code does not produce. Survivorship (today's F&O list applied back through
history) is real and still open, but it is already recorded as gaps-7, in
docs/07-plan.md cash-stock item 3 and in ST-02 ("membership is not
point-in-time history"), so it is not reported again here.

Probe: `$SCRATCH/probes/core` (path dependency on crates/core). It was built
and run with `CARGO_TARGET_DIR=$SCRATCH/target`. The output below is verbatim.

```
f64   -0.005 -> Ok(0)   text   -0.005 -> Ok(0)
f64   -1.005 -> Ok(-100)   text   -1.005 -> Ok(-100)
f64   -0.015 -> Ok(-1)   text   -0.015 -> Ok(-1)
f64    0.015 -> Ok(2)   text    0.015 -> Ok(2)
f64   -0.145 -> Ok(-14)   text   -0.145 -> Ok(-14)
text i64::MAX/100 '92233720368547758.07' -> Ok(9223372036854775807)
text '-92233720368547758.07' -> Ok(-9223372036854775807)
text '-92233720368547758.075' -> Ok(-9223372036854775807)
text '.' -> Err(NotDecimal) ; '-.5' -> Ok(-50) ; '5.' -> Ok(500)
-0.0 f64 -> Ok(0)
contract 2025-09-30-500000-CE vs 2025-09-30-1500000-CE : Greater  (strike 5000.00 vs 15000.00)
FNO: len 213 unique 213 over-24 0
NTM: len 750 unique 750 over-24 0
N500: len 500 unique 500 over-24 0
N200: len 200 unique 200 over-24 0
N100: len 100 unique 100 over-24 0
N50: len 50 unique 50 over-24 0
FNO index names present: 5/5; shares 208
N50\N100 0 N100\N200 0 N200\N500 0 N500\NTM 0
FNO isin mismatches 0
FNO_INDEX contains 'reliance' false 'RELIANCE' true
```

## Findings

### core-1 — low — crates/core/src/instrument.rs:391-395 (and `Contract`'s derived `Ord`, instrument.rs:441)

```rust
/// The format is chosen so that sorting the names sorts by underlying,
/// then expiry, then strike — which is the order a human reads an option
/// chain in.
...
} => write!(f, "-{}-{}-{}", expiry, strike.raw(), side.as_str()),
```

**Why it is wrong:** the strike is written as a decimal integer with no zero
padding. Sorting the names therefore compares strikes lexicographically, not
numerically. A strike with more digits sorts before a smaller strike with fewer
digits whenever its first digit is lower. `Contract` derives `PartialOrd, Ord`
over the same zero-padded bytes, so `Contract`'s own ordering has the same
defect.

**Repro (from the probe):** expiry 2025-09-30, Call, strikes 500000 paisa
(₹5,000) and 1500000 paisa (₹15,000).
- `Contract::of(..).cmp(..)` gives `Greater`. The ₹5,000 strike sorts after
  the ₹15,000 one.
- Expected (per the doc): `Less`.
- The same happens for any pair of strikes that differ in digit count, such as
  ₹50 against ₹150 on a low-priced stock.

**Impact:** contracts are stored and never swept, so no ranking or run identity
is affected. The comment is a false claim about order. A future consumer that
relies on it, such as an option-chain view sorted by name or by `Contract`,
would list strikes out of order. Fix: either zero-pad the strike (this changes
the path format, so it needs a decision entry) or correct the sentence.

## Checked and clean

- **`Paisa::from_rupees_half_up`** (price.rs:113-171).
  - NaN and ±inf are refused.
  - The range check is exclusive at both ends, so i64::MIN (the OI sentinel) cannot come out of it.
  - It uses floor plus a fraction ≥ 0.5 test, which is true half-up toward +inf for both signs, with no biased `+0.5` addition.
  - The `as i64` cast is guarded by the range check.
  - -0.0 maps to 0.
  - The probe confirms negative ties go toward +inf: -0.015 → -1 and -0.145 → -14.
- **`Paisa::from_rupee_text_half_up`** (price.rs:222-293).
  - It uses checked mul and add, the length is refused before parsing, and whitespace is ASCII only.
  - Signs are applied asymmetrically and correctly (a negative rounds up in magnitude only past an exact half).
  - It cannot produce i64::MIN: '-92233720368547758.075' gives -9223372036854775807.
  - It agrees with the float path on every sampled tie.
- **Paisa arithmetic.** There is no `Add`, `Mul` or `From<f64>` impl, only `checked_add` and `checked_sub`, which return `Overflow`. `rupees_trunc` and `paisa_part` are const division and remainder by 100 and cannot overflow.
- **`vendor::parse_expiry` and `parse_strike`.** The digit folds are bounded by fixed widths. The strike comes from text, exact and strictly positive (D-1311). The `as u8` at vendor.rs:315 is guarded.
- **`MemberIndex` (universe.rs:4317-4470).**
  - It is open-addressed, built at compile time, at most half full, and the table size is a power of two.
  - Lookups are capped at SYMBOL_CAPACITY before hashing, and the hit and miss probe bounds are pinned by tests.
  - `of_equity` makes six O(1) probes and `nse_isin` one probe plus an index. No linear scan over the 208 or 213 names is on any per-bar or per-call path.
  - Callers that walk `FNO_UNDERLYINGS` (runner/research_family.rs, pull/fno.rs, api/ingest.rs) enumerate the whole universe on purpose, once per job, not per bar.
- **List integrity (probe).**
  - All six lists have no duplicates, and no member is longer than 24 bytes.
  - All 5 index names are in FNO_UNDERLYINGS, which leaves 208 shares and matches CLAUDE.md §1.
  - The tiers nest strictly: N50 ⊂ N100 ⊂ N200 ⊂ N500 ⊂ NTM.
  - The FNO ISIN array is aligned with `nse_isin`.
  - Membership is case-sensitive, which is consistent with the uppercase-canonical `Symbol`.
- **`InstrumentKey::is_sweepable`.** One O(1) probe plus at most 5 comparisons, with no allocation.
- **blake3.rs.** `wrapping_add` is the specified u32 arithmetic, and the code is checked against independent official vectors that include multi-chunk lengths.
- **telemetry `Sink::emit_for_run`.**
  - The fast reject is one relaxed load. The per-target floor walk is bounded by MAX_TARGET_LEVELS.
  - `seq` uses `saturating_add` and the `written`, `dropped` and `rotations` counters are u64 relaxed `fetch_add`, which cannot wrap in practice.
  - The float encoder does not allocate. The byte span uses `try_from` with saturation.
  - The clock is clamped to be monotonic inside the lock.
  - `reserve_run_id` is checked and capped at MAX_SAFE_RUN_ID.
- **telemetry floats.**
  - Non-finite values are written as the strings "NaN", "Infinity" and "-Infinity".
  - The reader refuses an integer literal that fits neither i64 nor u64, and a decimal that overflows to inf, instead of rounding either.
  - Whole floats get ".0" so their type survives the round trip.
- **telemetry clock.** `millis_of` saturates. `civil_from_days` uses floor division on the negative side and cannot overflow for any day count that `now_millis` can produce.
