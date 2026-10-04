# crash-edge pass 13 (ce13): arithmetic and panic routes reachable from outside input

Tree: /home/claude/wt/zero3 at 1f4de71 (read-only). Release profile: overflow-checks = true, panic = "abort" (Cargo.toml:184, :188).
Scope: production code (not `#[cfg(test)]`) in core, store, lake, pull, costs, greeks, indicators, runner, cli and api.
Method: I stripped the test modules from a copy of the tree (about 298k lines of non-comment production code). Then I grepped for every unchecked `+ - * / %`, `+= -= *=`, `.sum()`/`.product()` on integers, unary negation and `abs()`, `<<`, `as` casts under `#[allow]`/`#[expect]`, `clamp`, `step_by`, `chunks`/`windows`, `split_at`/`split_off`, `repeat`, `div_euclid`/`rem_euclid`, `Instant`/`SystemTime`/`Duration` arithmetic, and the numeric query, argv and env parsers. I read each hit that sits on an external input. The workspace denies `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `cast_possible_truncation` and `cast_sign_loss`, and pedantic is enforced by `-D warnings` (Cargo.toml:78-91). So the only crash routes left are plain arithmetic and std calls that panic on bad arguments.

Result: **1 new finding (low).** Every other site I examined holds: a type bounds it, or an earlier check guards it (see the table below). No verification rows were assigned to this pass.

---

### CE-74 (low): `/pull`'s live row counter adds every vendor's census total with a plain `.sum()`, so two CRC-valid manifests whose totals add up to more than u64::MAX abort the api server

- **Where:** api/src/pullrun.rs:608-615
  ```rust
  pub(crate) fn rows_now(site: &Site) -> u64 {
      let (censuses, _) = crate::server::census_now(site);
      censuses
          .iter()
          .filter_map(census::VendorCensus::counters)
          .map(|(_months, rows, _entries)| rows)
          .sum()
  }
  ```
- **Why it is wrong:**
  - `Iterator::sum::<u64>` uses checked `Add`. Under `overflow-checks = true` an overflow panics, and with `panic = "abort"` the panic takes down the whole api process.
  - Each operand is one vendor manifest's `total_rows`. pull/src/manifest.rs:2518-2535 proves each total is a `checked_add` over that one manifest's entries. **Nothing bounds the sum across manifests.** `census::read_all` (api/src/census.rs:533-536) loads one manifest per `Vendor::ALL`.
  - Nothing bounds one entry's `rows` either: `Entry::check` (pull/src/manifest.rs:1455-1466) only refuses 0. A month can hold at most about 44,640 one-minute bars, or about 2.7 M at one-second, yet an entry claiming `u64::MAX` rows loads as `Census::Held` if its CRC32C is right.
  - `rows_now` runs when every pull run starts (pullrun.rs:1032), on every `ROWS_TICK` of the spawned ticker (:1051), and before and after each pass (:1065, :1068). The first POST that starts a pull therefore aborts the server. Restarting does not help: the next pull aborts again, until someone repairs a manifest by hand.
  - Same class as CE-61 (a CRC-valid huge counter aborts the api), at a different site that CE-61's fix does not cover.
- **Repro (not run):**
  1. Write vendor A's `manifest` with one entry whose `rows = u64::MAX`, `n_keys = 1` and `total_rows = u64::MAX`, with valid entry and slot CRCs.
  2. Give vendor B any normal census holding at least 1 row.
  3. Start a pull from /pull. `rows_now` computes `u64::MAX + rows_B`, the add panics, and the process aborts.
- **Minimal fix:** fold with `checked_add`, and show an overflowing total as unknown or refused instead of as a number. For example: `.try_fold(0u64, |a, r| a.checked_add(r))`, then treat `None` as "row total exceeds u64; census implausible" in `PullProgress`. Optionally, also refuse an `Entry` whose `rows` exceeds the record capacity of its timeframe's month in `Entry::check`, which closes the whole class for every reader.

---

## Sites checked and why each holds

| Site | Input | Why it holds |
|---|---|---|
| pull/src/fetch.rs:922 epoch -> micros | vendor timestamp | `checked_mul(1_000_000)` refuses with `TimestampRefused` |
| pull/src/fetch.rs:815 `timestamp - IST_OFFSET_SECS` | vendor text stamp | text arms come from `local_seconds`, bounded by `Day::new` (year <= 9999) and h/m/s range checks (http.rs:3757-3773) |
| pull/src/http.rs:3664 zone offset | vendor `+HHMM` | hours <= 14 and minutes <= 59 checked first |
| pull/src/http.rs:1516/1520 `"0".repeat(..)` | vendor exponent `1e999999` | the `point` window is limited to `MAX_PRICE_TEXT` places before the repeat |
| pull/src/csv.rs:392 `-total`, :806 epoch | archive CSV | `total` comes from `checked_mul`/`checked_add` (non-negative, so the negation is safe); the day is bounded |
| pull/src/request_minutes.rs:55,133 | vendor stamps | done in i128 |
| pull/src/session.rs:745-776 casts | any epoch second | `checked_add`, `local < 0` refused, `day_number > U32_DAYS_MAX` refused |
| pull/src/tenor.rs:255 | stamp, expiry | both days are bounded `Day`s, so the product is under 2^23 days * 86,400 |
| pull/src/rate.rs:626 `credit -=` | clock | only reached after `shortfall()` returns `None` in the same call; `earn` saturates |
| pull/src/manifest.rs:2518 per-census total | manifest bytes | `checked_add`, refused with `RowTotalOverflow` |
| store/src/file.rs:3072 `ts + anchor` | store stamp | stamp already checked inside the month; width >= 1 s |
| store/src/path.rs:642-649 | YearMonth | `YearMonth::new` restricts the year to 1970..=9999 |
| store/src/layout.rs:482,546 | header n_valid | `block < blocks` and `records <= records_per_block` |
| lake/src/footer.rs:149,161,240 | parquet bytes | `checked_sub(8)`, `len <= room`; zigzag `-i64::MAX - 1` is exact |
| lake/src/page.rs:507 `values_read +=` | page header i32 | needs about 2^32 pages before overflow; the negative count is refused right after |
| indicators pattern/session/daily/trend/lib | bar prices incl. i64::MIN/MAX | widened to i128 before `- abs *` |
| indicators/src/lib.rs:122 `ist_day` | any stamp | `saturating_add` + `div_euclid` |
| indicators/src/vwap.rs:309,376 | volume | `v <= 0` refused; accumulator capped at `ACC_CEILING` |
| costs/src/trip.rs:944 `sell - buy` | fills | both notionals are in [0, i64::MAX] |
| costs/src/strike.rs:277,347 | spot, moneyness | i128; spot <= 0 refused |
| runner/src/audit.rs:225 ppm | bar close | `before.close <= 0` skipped; i128 |
| runner/src/exit_grid_policy.rs:3469,4159 `/ open` | bars | `open <= 0` refused with `NonPositiveOpen` (:3510) before use |
| runner grid.rs:5545, excursion.rs:1488/1499, signal_candle_stop.rs:1623 `/ entry` | entry price | `entry <= 0` returns or refuses first |
| runner/src/pbo.rs:99,136,266,320; split.rs:93,219,282 | counts | divisor 0 refused first |
| runner/src/grid.rs:586,4746,4763; admission.rs:1684,3030 | counts | zero guarded, or `.max(1)`, or the constructor refuses 0 (admission.rs:3003) |
| runner/src/grid.rs:10698 `step_by(stride)` | ladder | `div_ceil(..).max(1)` |
| runner/src/outcome.rs:2079 `% wheel.len()` | n/a | `vec![..; capacity.max(1)]` |
| runner/src/trade.rs:1502 | stamp | i128 then `try_from` |
| cli/src/boolean_qualification_plan.rs:496,529 `end - start + 1` | plan file u64 | `admit` first passes both through `Day::from_days(u32)` (<= 9999) |
| cli/src/stored_data_completeness.rs:989 `windows(len)` | bars | `StreamFactsV1::of` refuses an empty execution stream first (:116) |
| cli/src/index_stop_qualification_numeric.rs:535 `chunks(width)` | period count | `derive_layout` filters `periods_per_segment > 0` |
| cli/src/expression_search_reader.rs:315, boolean_candidate_v1.rs:666/778 | checkpoint bytes | preceded by `<` checks or `checked_sub` charges |
| cli/src/lib.rs:1259/1355 MAX_POINTS/MIN_RR/TOP | argv | sign and zero checked; `points_to_ppm_at` saturates |
| cli Rules env knobs (`BRUTEX_MIN_*`) | env | used only in comparisons; `assurance_floor_bp` saturates |
| cli/src/pool.rs:195 `abs()` | worst loss | i128; `worst == 0` returns first |
| api/src/server.rs:4296 `basis_points` | bars | CE-2 fix: checked; base >= 1 |
| api/src/server.rs:2470 `clamp(begins, held)` | query window | `begins` filtered to `< held` |
| api/src/bars.rs:1226 `split_off(offset)` | query offset | `offset >= len` returns empty first |
| api/src/detail.rs:243, candidatejson.rs:101, indexstop*json, booleanjson | query ints | canonical u64, `checked_mul`/`checked_add`; `offset + n` only for rows that exist |
| api/src/sweeprun.rs:1274 `max_points` | POST body | `<= 0` refused; cli rejects `checked_mul(100)` overflow (boolean_search_launch.rs:55) |
| api/src/server.rs:1404 `+= rows` (one census) | manifest | bounded by that census's checked total |
| api/src/server.rs:9391/9552, autopilot.rs:510/1302 shifts and ladders | attempt counters | const-asserted `THROTTLE_ATTEMPTS <= 64`; counts capped |
| api/src/ingest.rs:1791 `epoch_secs`, autopilot due/now | clock pre-1970 / far future | `try_from` + `saturating_neg`; every `due` uses `saturating_add` |
| api/src/server.rs:17316/17380/18552 `Instant + d` | constants | `HEAD_READ_TIMEOUT`, `SHUTDOWN_GRACE` are compile-time |
| tokio `sleep(Duration::from_millis(x))` (pull/http.rs:703, masters.rs:1323, server.rs:8619,8730) | ladders | tokio clamps a far deadline; values bounded |
| api/src/census.rs:581 cast | clock | `Day` limits the year to 9999 before the cast |
| core/src/vendor.rs:1811 digit fold | master expiry text | `split('-')` gives exactly 3 parts of fixed width, all digits |
