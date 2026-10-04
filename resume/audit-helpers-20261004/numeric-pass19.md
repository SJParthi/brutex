# Numeric audit, pass 19: NSE-INDIAVIX as reference-only data

Head: `/home/claude/wt/zero3` @ **1f4de71** (origin/final/all-fixes-zero). Audit only. Nothing was edited and nothing was run; all results come from reading the source.

Scope: every production reader of `NSE-INDIAVIX`.

- `cli::vix_reference` (`VixReferenceMonth`, `slot_of`, `stamp`).
- `cli::index_stop_vix` and its codec, `api::indexstopvixjson`, `web/src/lib/index-stop-vix.js` and `IndexStopVix.svelte`.
- Global Replay V1, V3 and V4 VIX catalogs and identities.
- The vocabulary, runner, engine, indicators and costs crates, checked for any VIX path.
- Price snapping in `core::price` and the store's `ohlc_is_sane`.

## Verdict

**2 new findings, both low.** There are no high or medium findings. The stamping is exact-minute only. Absence is a variant, never 0 and never the previous value. The IST month and slot arithmetic is consistent. VIX never reaches the vocabulary, ranking, the 9-term run identity or the replay identity. Prior VIX findings checked: 4 FIXED, 2 NOT FIXED (see the verification table).

## Checklist results

| # | Question | Result | Evidence |
|---|---|---|---|
| 1a | Which minute is stamped? | The **entry minute** and the **exit minute**, never the signal minute. Index-stop stamps `entry_micros` and `exit_bar_micros`, which for a forced 15:10 close is the 15:09 bar. Global replay stamps `row.entry_micros` and `row.exit_micros`, which are the 1-min execution bars' open stamps (grid.rs:4198-4199). | index_stop_vix.rs:530-531; global_replay_v4.rs:635-636; global_replay_v3.rs:1044-1045; global_replay.rs:1676-1677 |
| 1b | IST alignment | `slot_of` derives the IST civil day and minute-of-day through `pull::session::IstMoment`. It refuses an off-grid minute or a timestamp from another month. A slot whose stored ts differs is refused, not treated as nearest. | vix_reference.rs:230-242, 261-297 |
| 1c | Missing, stale or other-day value | An empty slot gives `VixStamp::Absent`. There is no previous-value fill, no 0 and no nearest-minute search. Index-stop separates `Absent` (a valid month with no bar) from `Unavailable` (the month loader refused). The JSON has `candle:null` for both, and the web refuses a candle on either. A different day cannot be stamped, because the slot is exact-ts. | vix_reference.rs:8-15, 232-234; indexstopvixjson.rs:208-217; index-stop-vix.js `checkStamp` |
| 2 | Look-ahead | Every stamp is from a minute **after** the signal: signal_candle_stop refuses `signal_micros >= entry_micros` (:352), and entry is the next bar's open (:1290). It is a full candle, so its high/low/close run to entry+60 s. No decision, cost or score reads it. The index-stop UI says so in so many words (IndexStopVix.svelte:15-17). Global Replay V3/V4 store the same candle under the label "stamp at entry" with no caveat, but no api or web surface reads those records. This is not a finding: it is reference-only by construction. | as cited |
| 3a | Vocabulary, indicators, engine, runner, costs | `grep -i vix` finds only test or guard lines: portfolio.rs:942 is a test, and costs venue.rs refuses INDIAVIX. `InstrumentKey::is_sweepable` is false for INDIAVIX, and `VixReferenceMonth::open` refuses if that ever changes. | vix_reference.rs:91-95; instrument.rs:841-846 |
| 3b | Ranking, filters, sort keys | Ranking and qualification never read `index_stop_vix`. The api serves VIX only on `/index-stop-vix.json`, one trade page at a time, and it has no filter or sort parameter. The web has no VIX filter. The backtest page lets an operator pick INDIAVIX, but the run route refuses it (stored.rs:3615-3623). | server.rs:16803; index-stop-vix.js |
| 3c | Run identity, data_digest | VIX is not an input to any runner identity or digest: there is no `vix` in crates/runner/src outside one test. The index-stop VIX `lookup_identity` binds only catalog identity and pin. In V4, `replay_id` filters out kind-5 VIX rows (global_replay_v4.rs:329-331), and only `publication_id` includes them. In V3, `replay_id` excludes VIX, but `money_id` includes it (see p19num-1). | as cited |
| 4 | Storage: paisa and tick | VIX goes through the same `Paisa` half-up 2-dp path as the indices. There is **no 0.05 tick snap** anywhere in pull or core. The lake survey measured every stored VIX OHLC on the 2-dp grid, with a maximum deviation of 9.3e-10 (price.rs:670-685, rows `INDIAVIX` 8.18 / 86.64). So no distortion was observed. If a vendor ever sent 4-dp VIX, the half-up snap would lose at most 0.005 points, silently. That is what §7 prescribes, so it is not filed. The one defect here is the unit label (p19num-2). | price.rs:113-270 |
| 5 | Global replay and VIX month identity | The month key is the IST civil month via `IstMoment` in V1 (`year_month_of`), V3 (`StoredVixCatalogV3::stamp`), V4 (`VixCatalog::stamp`) and index-stop (`month_of`). All of them use the same authority as `slot_of`. Index-stop refuses an entry and exit in different months (:486-488), which cannot happen intraday. V4 keys `(feed, month)`, so the VIX feed must equal the execution feed. | global_replay.rs:1700-1712; global_replay_v3.rs:512-527; global_replay_v4.rs:491-517; index_stop_vix.rs:597-606 |

## Findings

### p19num-1 (low): docs/02 says VIX "never changes ... money ... identity" for Global Replay V3, but the V3 money id hashes both VIX stamps

- **Where:** docs/02-store-format.md:2043-2044, against crates/cli/src/global_replay_v3.rs:1181-1189 and :777, :1226-1233.
- **Doc:**
  > India VIX is stamped after admission and never changes selection, execution, money or replay identity (`CLAUDE.md` §1).
- **Code:**
  ```rust
  fn money_id(decision_id: [u8; 32], row: TradeRow, vix: VixPairV3) -> Result<[u8; 32], String> {
      ...
      put_trade_row(&mut hasher, row)?;
      put_vix_stamp(&mut hasher, vix.entry);
      put_vix_stamp(&mut hasher, vix.exit);
  ```
  `ordered_money_digest` is built from these ids, and that digest feeds `publication_id(replay_id, ordered_money_digest)`.
- **Why it is wrong:** a VIX backfill or correction (Absent becoming Exact, or a restated bar) changes every affected `MoneyRecordV3.money_id`. It also changes the ordered money digest, the publication id and the completion id. The money identity is exactly what the sentence says VIX never changes. The V1 section (docs/02:1106-1107) states this correctly: "VIX changes the publication identity but not selection, P&L or the execution-only replay identity". The V3 sentence contradicts V1 and the code. The doc test `the_store_format_doc_states_the_global_replay_v3_this_build_writes` binds this section, so the false sentence is pinned rather than caught. P&L and `replay_id` are not affected.
- **Repro:** not run. Reading the source is enough: change one `VixStamp` in a `VixPairV3` and `money_id` changes, because the stamp is a hash input at :1186-1187.
- **Minimal fix:** reword docs/02:2043-2044 to "never changes selection, execution, P&L or the replay identity; it is hashed into each money id and so into the publication identity". If the doc test pins the old sentence, update it in the same change.

### p19num-2 (low): `/index-stop-vix.json` labels India VIX index points as `*_paisa`

- **Where:** crates/api/src/indexstopvixjson.rs:210-214.
  ```rust
  Stamp::Exact(candle) => json!({"state":"exact","candle":{
      "micros":candle.ts_micros.to_string(),"open_paisa":candle.open.to_string(),
      ... "close_paisa":candle.close.to_string(), ...
  ```
- **Why it is wrong:** VIX is a volatility index in points, not a price in rupees. It is stored ×100 only because it shares the price write path. The wire calls these four fields paisa, a currency unit. The repo's own web page knows better and renders them as "VIX index points" (IndexStopVix.svelte:27, via `indexStopPoints`). The schema is the only artifact that tells a JSON consumer the unit, and here it names the wrong one. A reader of `close_paisa: "1345"` gets ₹13.45, not VIX 13.45. §7 reserves paisa for money, and §1 says VIX is never a price input. Nothing in the engine is computed wrong.
- **Repro:** not run. The test at indexstopvixjson.rs:298-315 already shows the projection emitting `"open_paisa":"1500"` for a VIX candle.
- **Minimal fix:** in a schema-version bump, rename the four fields to `open_hundredths`, `high_hundredths`, `low_hundredths` and `close_hundredths`, or `*_points_x100`. Update `CANDLE` in web/src/lib/index-stop-vix.js and the two projection tests. If the wire must stay frozen, state the unit in the response's `policy` text and in docs/02 §"Original single-stop VIX reference companion".

## Verification of earlier VIX findings

| ID | Status | Evidence at 1f4de71 |
|---|---|---|
| cli3-1 (publish shortcut on directory existence) | FIXED | index_stop_vix.rs:390-392 shortcuts only on `persistence::committed` (a whole receipt). D-1760. |
| replay-2 (retry body differs → `write_or_equal` wedge) | FIXED | boolean_candidate_persistence.rs:119-131: an unreceipted body is discarded and rewritten. D-1760. |
| ledgers-1 (own failed receipt reported as success) | FIXED | index_stop_vix.rs:393-425: only `lost_owner_race` is answered by the saved companion, and every other error is returned. D-1908. |
| CE-8 (V4 holds every VIX month) | FIXED | global_replay_v4.rs:471-486, 505: `make_room` with `VIX_MONTHS_HELD = 4`. D-1769. |
| replay-1 / indexstop-1 (transient lock saved as a permanent Unavailable month) | NOT FIXED | index_stop_vix.rs:563-589 still maps every `VixReferenceMonth::open` error to `unavailable_reason`. `BarFile::open_existing` still uses `Flock::try_lock_shared` (store file.rs:1592-1595). D-1760 records "Not changed. indexstop-1". |
| replay-5 (V4 late non-blocking VIX open refuses the whole replay) | NOT FIXED | global_replay_v4.rs:495-512 still opens each month lazily inside `schedule`, after all OOS work, through the same try-lock. |

## Checked and clean (no finding)

- `VixReferenceMonth::from_file` refuses duplicate slots, non-increasing ts and wrong-month or off-grid rows before it publishes an index. A partial month is never returned.
- Its allocation is a fixed 44,640 slots.
- `snapshot_digest` covers vendor, month, record count and every slot including holes. It is presentation-only.
- The index-stop codec refuses contradictory repeated minutes (`validate_repeated`). It also refuses reordered months, and it refuses padding that is not zero for `Absent` and `Unavailable`.
- The web refuses an `exact` stamp in an empty month, refuses a candle on absent or unavailable stamps, and checks that the entry and exit candle `micros` equal the trade minutes.
- V4 VIX counters (`money_rows += 1`, `pricing_refused += u64::from(bool)`) are bounded by `bounds.records`, so they cannot overflow.
- V1/V3 `validate_vix_stamp` refuses a zero-valued VIX candle that `ohlc_is_sane` admits. V4 and index-stop do not refuse it. Only V1 and V3 can refuse there and V4 is the production writer, so this is not filed.
