# Pass 10: test strength by hand mutation (final/all-fixes-zero @ 1f4de71)

This is a read-only audit of `/home/claude/wt/zero3`, detached at `1f4de71` ("docs: CE-19's decision (D-1981) and invariant (ZE-02)").

**Scope.** The production lines under `crates/` that changed between `331b05c` and `1f4de71` (234 files, +22,161/-2,707). From them, 50 high-risk sites were picked: refusal guards, boundary comparisons, rounding direction, rollback paths, lock and barrier memory, checksum checks and admission gates.

**Method.** For each site:
- one mutation is named;
- the test that kills it is found, by name and by its exact assertion;
- or the site is shown to have no killing test.

Each mutation is tagged:
- **CM:** cargo-mutants 26.2.0 (gate 18) generates it.
- **hand:** only a hand mutation would make it.

A CM survivor turns gate 18 red whenever the line falls inside a diff. A hand survivor is a missing test under CLAUDE.md §4 and §9, but no gate would see it.

**Cargo.** Cargo was run only to prove the medium findings. Each run used the throwaway worktree `/home/claude/wt/scratch-tds10` at `1f4de71` with `CARGO_BUILD_JOBS=2`, applied one mutation at a time and ran no workspace-wide suite. cargo-mutants was not run.

## Verdict

**1f4de71: 7 new findings: 2 medium (P10-01, P10-02) and 5 low (P10-03 to P10-07). No high findings.**

The two medium findings:
- **P10-01 (medium, ran):** the GST-rate choice in `costs::trip::charge_stack_legs` carries a CM mutant that survives the whole `costs` suite.
- **P10-02 (medium, ran):** reversing the documented `(k, i)` order in `cli::ordered::Turns::ready` is a CM mutant. The ordering test cannot kill it, because the test compares runs only with each other and never with the input order.

Of the 50 sites, 43 have a killing test, and most of those test the exact boundary: equal bodies, `limit=1000`, `p = 0.05`, `MAX_DEPTH`, 18 and 19 schema elements, and block length equal to periods. Rollback and barrier memory are well defended. Every rollback deletion and every failed-barrier "memory" deletion that was tried is killed by a byte-identity or `BarrierFailed` assertion.

**`cfg(test)` and `mutants::skip`.** No `#[mutants::skip]` exists in the tree, and none was added. Three production functions gained a `#[cfg(test)]` / `#[cfg(not(test))]` fork since `331b05c`:
- **`cli::fixed_tail::sync_or_roll_back` and `sync_all_hooked`.** These have a reason and a decision: D-1902 says the "thread-local fault ... is compiled out of production".
- **`cli` lib.rs `AUDIT_INPUT_LOADS` / `RUNG_SPAN_LOADS` counters.** These are test-only counters, and D-1557 cites the proof test that reads them.
- **`store::file::barrier` (file.rs:3627-3645).** This one has no decision: D-1907 describes the barrier memory but not the injected `tests::sync_fault_fires()` fork. This is P10-07.

## Table: function | mutation | killing test + assertion | verdict

| # | Function (file:line) | Mutation | Killing test, exact assertion | Verdict |
|---|---|---|---|---|
| 1 | `cli::append_rollback::append_with` (append_rollback.rs:49) | hand: delete `file.set_len(end)`, return the error only | `append_rollback::tests::a_short_write_truncates_back_and_the_next_append_lands_on_the_boundary`: `assert_eq!(std::fs::read(&path)..., [1, 2, 3, 4])` | killed |
| 2 | same (:43) | hand: drop `seek(SeekFrom::End(0))` | `an_append_writes_at_the_end_even_when_the_cursor_is_elsewhere`: `assert_eq!(read, [1, 2, 3, 4, 5])` | killed |
| 3 | same (:53) | hand: the failed-rollback arm drops the second error | `a_failed_truncation_names_both_errors`: `refusal.starts_with("cannot append Readonly V1 record: injected EIO; truncation back to 2 bytes also failed: ")` | killed |
| 4 | `cli::fixed_tail::sync_or_roll_back` (fixed_tail.rs:177) | hand: delete `remember_failed_barrier(path)` | `fixed_tail::tests::a_failed_barrier_cuts_the_whole_block_and_is_never_confirmed_later`: `refuse_after_failed_barrier(&path).expect_err("never confirmed")` | killed |
| 5 | same (:178) | hand: delete `roll_back(..)` | same test: `assert_eq!(contents(&path), vec![1; 8], "the whole block is cut")` | killed |
| 6 | `fixed_tail::heal_torn_tail` (:270) | CM: `found <= header` to `>` | `a_torn_tail_is_cut_to_the_last_whole_record_and_reported`: `assert_eq!(healed, Some(TornTail { kept: 46, found: 50 }))` | killed. The hand mutation `<=` to `<` is equivalent, because `spare == 0` when `found == header`. |
| 7 | same (:285) | CM: `leading != magic` to `==` | `a_whole_or_short_file_is_left_alone`: `assert_eq!(heal_torn_tail(.., b"NOTTHIS"), Ok(None))` and `assert_eq!(contents(&path).len(), 29, "a foreign file is never cut")` | killed |
| 8 | `fixed_tail::heal_torn_header` | hand: cut a non-prefix header | `a_strict_prefix_of_the_header_is_cut_to_nothing_and_anything_else_is_left`: `assert_eq!(heal_torn_header(..), Ok(None), "foreign")` | killed |
| 9 | `store::file::BarFile::may_append` (file.rs:2185) | hand: drop the stride/width check | `sidecar_checksum_tests::a_wrong_record_width_is_refused_before_any_committed_bytes_move` (append of overlays into a bar month is `Err(NotABarPath { found: Overlay })`) | killed |
| 10 | `store::file::barrier` (:3638) | hand: drop the `FAILED_BARRIERS.insert` | `file.rs tests::a_failed_append_barrier_is_never_confirmed_by_a_later_append`: `assert_eq!(file.append(&batch), barred, "the same handle")` / `"a reopened handle"` | killed |
| 11 | `store::file::refuse_if_sealed` (:3397) | CM: `meta.len() > 0` to `==` / `<` | `tests/write.rs a_truncated_month_whose_sidecar_proves_records_is_refused_not_reinitialised`: `assert_eq!(refused, Err(StoreError::CommittedRecordsLost { .. }))` | killed |
| 12 | same (:3397) | hand: `> 0` to `>= 0` | none. No test leaves an existing but EMPTY sidecar beside an interrupted genesis. | **SURVIVES: P10-03** |
| 13 | `BarFile::open_or_create` (:1480) | hand: drop the `if existed` guard around `refuse_if_sealed` | `a_deleted_month_file_is_created_again_despite_its_sidecar`: `.expect("a deleted month is created again")` | killed |
| 14 | `store::file::is_interrupted_genesis` (:3381) | hand: drop `rest.iter().all(\|&b\| b == 0)` | `a_region_with_bytes_a_torn_genesis_cannot_leave_is_still_refused`: `assert!(open(..).is_err(), "a byte in the second slot ...")` | killed |
| 15 | `BarFile::diagnose_overlap` (:2856-2860) | CM: `stored.stamp() > row.stamp()` to `<` (swaps NotHeld/Skipped) | `tests/write.rs an_overlap_that_disagrees_is_refused_with_the_real_diagnosis`: `Err(OverlapDisagrees { at: 1, .., conflict: Conflict::NotHeld })` and `.. Conflict::Skipped` | killed |
| 16 | `store::header::Header::commit` (header.rs:399) | hand: drop the `requires_checksums && !checksums_present` refusal | `tests/unit.rs`: `assert_eq!(Header::genesis(7, 60, 0).commit(), Err(FormatError::ChecksumsRequired(3)))` | killed |
| 17 | `Header::decode` (header.rs:519) | hand: drop the v3 flag refusal | `tests/unit.rs`: `assert_eq!(Header::decode(&slot), Err(FormatError::ChecksumsRequired(3)))` | killed |
| 18 | `store::layout::Layout::requires_checksums` | CM: `==` to `!=` | `file.rs a_version_two_month_still_reads_and_appends_at_version_two`, plus #16 | killed |
| 19 | `lake::footer::check` (footer.rs:155) | CM: `len > room` to `>=` | `footer_tests::the_footer_length_must_fit_between_the_magics`: `ok(check(&file))` at len == room == 1 | killed |
| 20 | `lake::footer::walk` (:212) | CM: `stack.len() >= MAX_DEPTH` to `>` | `nesting_is_refused_one_past_the_bound_and_admitted_at_it`: `reason(walk(&nested(MAX_DEPTH))).contains("deeper than 32")` | killed |
| 21 | `lake::footer::schema_bound` (:226) | CM: `*left > MAX_SCHEMA_ELEMENTS` to `>=` | `the_schema_bound_is_the_widest_lake_shape_exactly`: `ok(walk(&schema_footer(18, false)))` | killed |
| 22 | `lake::footer::admit` (:277) | CM: `values > cur.unread()` to `>=` | `a_list_declaring_more_values_than_bytes_is_refused_at_the_edge`: `reason(walk(&over)).contains("ends inside")` (3 values, 3 bytes) | killed |
| 23 | `costs::trip::price` (trip.rs:1236) | hand: swap the Long/Short `(buy, sell)` arms | `each_legs_charge_is_priced_at_its_own_days_regime`: `assert_eq!(long_straddling.stt(), long_after.stt(), ..)` | killed |
| 24 | `costs::trip::charge_stack_legs` (:1066) | hand: STT at `buy.stt()` | same test: `assert_ne!(short_straddling.stt(), long_straddling.stt())` | killed |
| 25 | same (:1085) | CM: `sell.gst().get() > buy.gst().get()` to `<` | none. **Ran:** 202 unit, 4 integration and 37 doc tests pass with the mutant. | **SURVIVES: P10-01** |
| 26 | `runner::admission::losing_rate_ceil` (admission.rs:2899) | hand: drop `+ u64::from(inexact)` | `a_losing_rate_whose_floor_sits_on_the_cap_fails_when_the_exact_rate_is_above`: `assert!(verdict.failed().contains(AdmissionReasonV1::LosingTradeRate))` | killed |
| 27 | same (:2898) | CM: `remainder != 0` to `==` | same test, exact half: `assert!(!verdict.failed().contains(LosingTradeRate))` (20 of 50) | killed |
| 28 | `runner::bootstrap::Verdict::clears` (bootstrap.rs:175) | CM: `p_value <= 0.05` to `>`; hand: to `<` | bootstrap.rs:2059 `assert!(white.clears(), "White at p = 0.05")`; :2248 `assert!(!v.clears(), "no draws cannot clear anything")` | killed |
| 29 | `runner::bootstrap::aligned_for` (:1617) | CM: `block <= periods` to `>`; hand: to `<` | `a_block_longer_than_the_series_is_refused_by_every_entry_point` (`.is_none()` at periods+1); :3095 `assert!(reality_check(&set, 999, 3, periods).is_some())` | killed |
| 30 | `runner::excursion::ppm_ceil_of` (excursion.rs:1500) | hand: ceil to floor (drop `+ 1`); CM: `entry <= 0` to `<` (divide by zero) | `the_gated_adverse_reading_rounds_up_and_the_crossing_reading_floors`: `assert_eq!(c.adverse_ppm_ceil_at(0, entry, side), 10_001)`; `assert_eq!(super::ppm_ceil_of(5, 0), 0)` | killed |
| 31 | `indicators::pattern` engulfing 157/158 (pattern.rs:536) | CM: `bar0.body > bar1.body` to `>=` | `an_equal_body_reversal_lights_neither_engulfing_nor_harami`: `assert!(!mask.get(position), "{position} lit on equal bodies")` | killed |
| 32 | pattern tasuki 213 (:752) | CM: `bar0.close > bar2.high` to `>=` | `every_clause_of_the_reshaped_patterns_is_required_on_its_own` near miss "closes AT the first high: the gap is filled" | killed |
| 33 | `vocab::expression::Tier::of` | CM: `len < 2 * SHALLOW_SLOTS` to `<=` | `assert_eq!(Tier::of(16).slots(), 64)` / `Tier::of(128)` | killed |
| 34 | `vocab::table::name_index` | hand: overwrite on collision instead of probing | `a_name_held_by_two_rows_resolves_to_the_lower_row`: `assert_eq!(rows, [0, 2])` | killed |
| 35 | `api::logs::Ration::admit` (logs.rs:1287) | CM: `>= FAILED_LINE_WINDOW_MS` to `<` / hand `>` | `a_flood_of_failed_requests_writes_a_bounded_number_of_lines`: `assert!(next.write, "a new window writes again")` at exactly `start + WINDOW` | killed |
| 36 | `api::pullrun` leg cap (pullrun.rs:475) | CM: `legs.len() == MAX_RUN_LEGS` to `!=` | pullrun tests: `assert_eq!(count, MAX_RUN_LEGS + 1)` / `why.contains(&MAX_RUN_LEGS.to_string())` | killed |
| 37 | `api::server` bars `limit` (server.rs:3393) | hand: `1..=MAX_WINDOW_LIMIT` to `1..MAX_WINDOW_LIMIT` | `bars_window_route_tests` :435 `("&limit=1000", 1_000)`; :444-445 `"0 is not a page size"`, `"1001 is not a page size"` | killed |
| 38 | `api::autopilot::probe_store_halts` (autopilot.rs:3008) | CM: `attempt >= STORE_PROBES` to `<` / hand `>` | `the_last_failed_write_probe_names_no_next_probe`: `assert!(!last.contains("next probe is in"))` | killed |
| 39 | `api::autopilot::frontier` (:2481, :2503) | hand: delete either clamp | `a_caught_up_feed_stays_on_the_month_still_being_written`: `assert_eq!(at, month(2026, 10), ..)`; `assert_eq!(back, month(2026, 10))` | killed |
| 40 | `cli::cancel::check` (cancel.rs:77) | CM: return `Ok(())`; hand: drop the `OBSERVED` increment | `operator_boundary_tests::a_requested_stop_refuses_at_the_next_month_and_names_the_cancellation`: `why.starts_with(CANCELLED)`, `crate::cancel::observed() >= 3` | killed |
| 41 | `cli::ordered::Turns::ready` (ordered.rs:87) | CM: `other < at` to `other > at` | none. **Ran:** all 3 `ordered::tests` pass with the mutant. The ordering test compares runs only with each other. | **SURVIVES: P10-02** |
| 42 | same | hand: drop `lane.finished \|\|` | same test: a lane that finished blocks the rest, so it hangs to the timeout | killed (timeout) |
| 43 | `cli::result_set::PrefixDigest::require_unchanged` (result_set.rs:664) | CM: `\|\|` to `&&` | result_set.rs:1642 `assert!(why.contains("rewrote already-indexed"))` (same length, different bytes) | killed |
| 44 | `PrefixDigest::extend_from` (:641) | hand: delete the `read != wanted` refusal | none: no test shrinks the file between the scan and the hash | **SURVIVES: P10-05** |
| 45 | `cli::population_admission_v3::append_locked` (population_admission_v3.rs:3987) | hand: drop `&& prepared.decisions.starts_with(&trailing.decisions)` | none: no test of a same-`block_id` trailing block whose rows differ | **SURVIVES: P10-06** |
| 46 | `telemetry::sink` clock `stamp` (sink.rs:598) | hand: `now < self.last_at` to `<=` | none: only the clock-ahead test reads `clock_held` | **SURVIVES: P10-04** |
| 47 | `telemetry::Sink::open` directory lock | hand: skip taking `held_lock` | sink.rs ~:3570 second `Sink::open` refused: `why.contains("another") && why.contains(&dir...)` | killed |
| 48 | `pull::http::number_text` (http.rs:1514) | CM: `point <= 0` to `>`; hand: to `<` | `a_json_price_is_snapped_from_the_vendors_own_text` (`2.5e2` gives 250) kills the CM mutant. Nothing pins `point == 0`. | CM killed; hand **SURVIVES: P10-07** |
| 49 | `pull::http::repeated_key` | hand: return `None` | `an_answer_repeating_a_key_in_one_object_is_refused`: `"an escaped spelling of a held key is a repeat"` | killed |
| 50 | `cli::boolean_candidate_persistence::write_or_equal` (:254, :264) | hand: delete `remove_file` withdrawal; delete the reuse byte check | `a_file_whose_barrier_failed_is_withdrawn_and_the_rerun_commits`: `assert!(!directory.join(name).exists())`; `an_existing_file_with_other_bytes_is_refused_and_kept` | killed |

## New findings

### P10-01 medium: the GST-rate choice in `charge_stack_legs` is a surviving cargo-mutants mutant (ran)

`crates/costs/src/trip.rs:1085`:
```rust
    let gst_rate = if sell.gst().get() > buy.gst().get() {
        sell.gst()
    } else {
        buy.gst()
    };
```
**Why it is wrong.** `Rates::new` always sets `gst: GST_ON_FEE_BASE` (trip.rs:296). `charge_stack` passes the same `Rates` twice, and `price` passes two sets that are each built by `new`. So the two operands are always equal, and every CM mutant of this line survives:
- `>` to `<` or `==`
- the condition to `true` or `false`

The D-1535 docstring claims a straddling trip "is never charged the lower one", and no test checks that claim. Gate 18 plans mutants for changed lines, so any PR whose diff covers this line turns gate 18 red, and CLAUDE.md §9 makes a surviving mutant a failed definition of done. The `cfg(test)` constructor `Rates::with_all` (trip.rs:314) already exists to build unequal sets, but no test passes two different sets to `charge_stack_legs`.

The same blind spot covers a hand mutation of `per_leg`, pricing the sell leg at `buy`'s exchange/SEBI/IPFT rate. No verified regime boundary changes those three rates, so `each_legs_charge_is_priced_at_its_own_days_regime` asserts `long_straddling.exchange() == long_after.exchange()` and cannot tell which leg's rate was used.

**Repro (ran).** In `/home/claude/wt/scratch-tds10` at 1f4de71, `sed -i '1085s/ > / < /'` was applied, then `cargo test -p costs --locked` ran. Result: 202 unit, 4 `tests/adversarial.rs` and 37 doc tests passed, 0 failed. The mutation was then reverted.

**Minimal fix.** Add a unit test in `trip.rs`: call `charge_stack_legs(fills, q, &low, &high)` and `(&high, &low)`, where the two sets are `Rates::with_all` and differ in GST, exchange and SEBI. Assert that GST is the larger rate in both orders and that each per-leg levy uses its own leg's rate.

### P10-02 medium: reversing the documented lane order is a cargo-mutants mutant the ordering test cannot kill

`crates/cli/src/ordered.rs:87`:
```rust
                || lane.performed >= if other < at { round } else { round - 1 }
```
The module docs (ordered.rs:16-22) state the contract: "Event `k` of lane `i` waits until every lower lane has performed its event `k` ... So the writes land sorted by `(k, i)`: round `k` of every lane in input order."

**Why it is wrong.** The CM mutant `other < at` to `other > at` makes every lane wait on the HIGHER lanes instead. Writes then land sorted by `(k, -i)`. That order is still a pure function of the inputs, with no deadlock (the smallest pending event in the reversed order always proceeds) and no schedule dependence.

The only ordering test is `ordered_tests::shared_durable_writes_follow_the_inputs_not_the_schedule`. Its assertions are `assert_eq!(first_slowest, one)`, `assert_eq!(last_slowest, one)` and `assert_eq!(rerun, one)`. They compare runs with each other and never compare `one` with the input order. The test's own doc says "one thread filed workers 0, 1, 2…", but nothing asserts it. So the mutant passes, and the journal and ledger order that the module and D-1556 document (input order) is unpinned. The other test, `a_turn_outside_a_lane_or_inside_a_held_one_never_waits`, only checks the returned `Vec`, which `map` builds by joining in input order whatever the turn order is.

**Repro (ran).** In `/home/claude/wt/scratch-tds10` at 1f4de71, `sed -i "s/if other < at { round }/if other > at { round }/" crates/cli/src/ordered.rs` was applied, then `cargo test -p cli --lib --locked -- ordered::` ran. All 3 `ordered::tests` passed with the mutant, including `shared_durable_writes_follow_the_inputs_not_the_schedule ... ok` (1.38s). The worktree was then removed.

**Minimal fix.** In `shared_durable_writes_follow_the_inputs_not_the_schedule`, also assert that the ledger identities are `(0..WORKERS).map(|w| id(w, 0))` in that order. Also assert that the journal's first `WORKERS` rows are each worker's outer `begin` in input order.

### P10-03 low: `refuse_if_sealed`'s "empty proves nothing" boundary is untested

`crates/store/src/file.rs:3397`:
```rust
        Ok(meta) if meta.len() > 0 => Err(StoreError::CommittedRecordsLost {
```
The docstring (:3388-3390) says "Absent or empty, it proves nothing was, and the caller may repair." The two tests cover the other cases:
- `a_truncated_month_whose_sidecar_proves_records_is_refused_not_reinitialised`: a non-empty sidecar.
- `a_torn_genesis_slot_with_nothing_committed_is_repaired`: no sidecar at all, `NotFound`.

No test leaves an existing zero-length sidecar beside a zeroed or torn region. That is the state an append leaves if it creates the `.crc` and dies before writing an entry.

**Why it is wrong.** The hand mutation `> 0` to `>= 0` (or `!= u64::MAX`) refuses such a month for ever, which is the exact wedge D-1521 removed. The whole suite would stay green. The CM mutants of this line (`==`, `<`) are killed by row 11.

**Repro.** Not run. This is argued from the two tests' fixtures: `grep -rn 'Checksums' crates/store/tests/write.rs` shows sidecars only from real appends (non-empty) or removed.

**Minimal fix.** Extend `a_torn_genesis_slot_with_nothing_committed_is_repaired`: write an empty `.crc` beside the torn file and assert that the month still opens with `records() == 0`.

### P10-04 low: `clock_held` over-counting same-millisecond events is not caught

`crates/telemetry/src/sink.rs:598`:
```rust
        let held = now < self.last_at;
```
**Why it is wrong.** `Health::clock_held` is documented as counting events "whose clock reading was held up to the floor". The hand mutation `<` to `<=` counts every event stamped in the same millisecond as the previous one, which is routine under load, as "held". The only assertions on the counter are in `a_resumed_floor_ahead_of_the_clock_is_named_and_counted` (sink.rs:3622 and :3629), where the floor is in 2100, so equality never occurs. The operator-facing health figure could drift without any test noticing.

**Repro.** Not run. `grep -n clock_held crates/telemetry crates/api` shows only those two assertions.

**Minimal fix.** Unit-test `stamp` directly: `stamp(5)` then `stamp(5)` returns `(5, false)` both times, and `stamp(4)` returns `(5, true)`.

### P10-05 low: `PrefixDigest::extend_from`'s short-read refusal has no test

`crates/cli/src/result_set.rs:641`:
```rust
        if read != wanted {
            return Err(format!(
                "{} ended at byte {} while its validated prefix up to byte {to} was being hashed",
```
**Why it is wrong.** This refusal was added since `331b05c` (35e05d1). If a peer truncates the manifest between the receipt scan and the hash, the digest covers fewer bytes than `covered` claims. Deleting the guard (hand) lets that happen, and the next `require_unchanged` would then compare against a digest of a different range. `grep -rn 'ended at byte' crates/cli/src` finds only the definition. The CM mutant (`!=` to `==`) is killed, because it refuses every honest growth.

**Repro.** Not run.

**Minimal fix.** Add a test that calls `PrefixDigest::over(&mut file, path, len + STRIDE)` on a file shorter than `to` and asserts the "ended at byte" refusal.

### P10-06 low: Admission V3's exact-retry prefix check is unpinned

`crates/cli/src/population_admission_v3.rs:3986-3990`:
```rust
            if trailing.block_id == prepared.source.block_id
                && prepared.decisions.starts_with(&trailing.decisions)
            {
                return self.complete_trailing(prepared, &trailing);
            }
```
**Why it is wrong.** `complete_trailing` (:4119) appends only `prepared.decisions[trailing.len()..]` and never compares the rows already held. So the `starts_with` clause is the only thing that keeps a same-block trailing prefix with different rows from being completed into a mixed block.

`partial_orphan_corruption_reserve_and_lock_bytes_refuse` tests only two cases:
- an exact prefix (same block, same rows), at :6471;
- a foreign block (`block_id = [0xEE; 32]`), at :6491.

So deleting the clause (hand) passes. The parallel ledgers pin the same rule:
- `population_finalization_v3.rs:2444` refuses "is not exact retry";
- anchored lineage V4 has `assert_refuses(writer.append(&foreign_retry), "is not exact retry")` (:2506).

**Repro.** Not run.

**Minimal fix.** In the same test, write `prepared.decisions[0]` with one field changed and resealed under the SAME `block_id`. Assert that the retry discards it (the file is cut, then the block commits with the prepared rows) rather than completing it.

### P10-07 low: two untested branches, and one undocumented test-only fork

**(a) `pull::http::number_text`, `crates/pull/src/http.rs:1514`:** `if point <= 0 {`. A vendor exponent whose decimal point lands exactly at the front, such as `5e-1` or `2.5e-1` rupees, takes the `point == 0` branch. The hand mutation `<= 0` to `< 0` sends it to the split branch, which produces `".5"`. `csv::paisa` then refuses that (empty whole part), so a valid 50-paisa price would be refused. No test pins it: the only negative exponent anywhere is `1e-7` in `rolling.rs:1213`, and that has `point = -6` and is refused for its precision anyway.

Fix: add `("5e-1", 50)` to `a_json_price_is_snapped_from_the_vendors_own_text`.

**(b) `store::file::barrier`, `crates/store/src/file.rs:3627-3645`:**
```rust
    #[cfg(test)]
    let synced = if tests::sync_fault_fires() { Err(io::Error::other("injected sync fault")) } else { file.sync_all() };
    #[cfg(not(test))]
    let synced = file.sync_all();
```
This is a `cfg(test)`/`cfg(not(test))` fork added since `331b05c` (4cee129). D-1907 records the barrier memory but not this injector. The `cfg(not(test))` line is never compiled under any test, so no test can kill a mutation of it. The cli counterpart's injector is recorded by D-1902 ("compiled out of production").

Fix: add one sentence to D-1907, or reuse the D-1902 wording. Better still, route the production call through one `fn sync(file) -> io::Result<()>` that both configurations share, so the line that ships is the line that is tested, as `fixed_tail::sync_all_hooked` already does.

Neither part was run.

## Notes

- **Equivalent mutants found:**
  - row 6, `found <= header` to `<`;
  - `excursion::ppm_ceil_of`'s `move_paisa <= 0` to `< 0`, where a move of 0 yields 0 either way.

  They are listed so that a later pass does not report them.
- **Cargo cleanup.** After the runs, the scratch worktree `/home/claude/wt/scratch-tds10` was removed. The target dir `/home/claude/wt/target-tds10` holds only build output.
