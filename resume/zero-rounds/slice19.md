# slice19 — 1 finding

## F1 [low] Qualification publish re-opens its own receipt with the byte bound passed as the record bound, so the post-write "cold read" gate is weaker than the recovery gate
- where: crates/cli/src/index_stop_qualification.rs:695-700 (`publish`)
- what: `publish` checks the receipt it just wrote by calling `Reader::open(request.root, identity, request.bounds.bytes, request.bounds.bytes)`. The signature is `open(root, identity, max_bytes, max_records)` (:288-293), so the fourth argument is a byte count standing in for a record count. That `max_records` reaches three gates:
  - `codec::decode`'s `count > max_records` check (codec :190).
  - The split-plus-fold total check (codec :220 and :262, `total > max_records`).
  - Both `candidates::Reader::open(.., max_records)` calls (:375, :380).

  At publish these gates are bounded only by `qualification.bytes`, which the launch configuration sets to `4 × max_bytes` (index_stop_launch.rs:118-120, :138). This is not a real record limit.

  The recovery path re-verifies the same child with the real limit: `qualification::verify_search_slot_bounded(.., bytes, records, ..)`, where `records = configuration.capture.records` (index_stop_search_checkpoint.rs:455-465, fed from index_stop_search.rs:402-408). So `produce` can verify, finish its attempt as `Completed`, and return a link that `Frame::done` publishes. Every later `recover` then refuses that same link with "single-stop fold extent exceeds admission".

  This is the same unit mix-up as the known W2-cli6-3 (index_stop_store.rs:310, bytes passed where records belong), but in a different file and call site.
- evidence (trace):
  1. The production configuration has `capture.records = strict.max_records()` and `qualification.bytes = strict.max_bytes() * 4` (index_stop_launch.rs:83, :118-120, :138, :146).
  2. `produce` → `publish` → `Reader::open(.., bounds.bytes, bounds.bytes)` → `open_bounded(.., max_records = 4·max_bytes, ..)`. Then `codec::decode(bytes, identity, 4·max_bytes)` checks `total = split_count + Σ fold counts` against `4·max_bytes`, not against `max_records`.
  3. The recovery path calls `verify_search_slot_bounded(root, id, pin, history_bytes, capture.records, ..)` → `open_bounded(.., max_records = capture.records, ..)` → the same decode, now checked against `capture.records`.
  4. Any qualification with `split_count + count × folds > capture.records` (split_count can be up to the canonical CSCV maximum) passes step 2, is acknowledged, and is then refused in step 3 on every restart. The checkpoint chain cannot then be recovered. The only remedy is to raise `MAX_RECORDS`, which the declaration hashes, so that starts a new search.
  5. git blame: the line has been unchanged since ffa41c6d. No comment explains the duplicated argument.
- fix: Pass a real record limit at publish, and add a test where `split_count + folds` lies between the two limits. Either:
  - add a `records` field to `qualification::Bounds` (and to its identity and codec), or
  - make `qualification::Request` carry `capture.records` from `run_rung` and pass it as the fourth argument to `Reader::open` in `publish`, so the publish gate and the recovery gate are the same.

## What else was checked (no evidenced defect)
- **global_replay_v4 (+ codec, lifecycle, store):**
  - GAP15-18 is fixed. `candidate_budget` now divides by 3, and `10 + selected` exactly matches 1 header + 8 envelopes + selected witnesses + 1 completion.
  - Record seals cover every byte. The completion is appended last, and the final 32 bytes are written after a sync. Prefix-resume compares unaligned prefixes correctly.
  - Kind-5 VIX rows are excluded from the economic id. Money sums use checked i128 arithmetic.
  - Minute grouping is bounded by `MAX_INTENTS_PER_MINUTE`. Constituents are unique per group because `rung_minutes` is part of the key.
  - GAP15-19 (no aggregate ambiguous/gap ceiling) is still present. It is known and not re-reported.
- **index_stop / index_stop_launch / index_stop_search:**
  - Grouped begin/evaluate ordering: `zip`+`take(admitted)` does not over-consume jobs.
  - Measurement window and limit validation.
  - Declaration contents.
  - The later source is loaded from the training start through the later end, as the doc says.
  - `batch_admission` arithmetic.
- **index_stop_qualification (+ codec, numeric, metrics):**
  - Header/row byte widths match the encoder.
  - Decode bounds every extent before allocating.
  - Win/loss definition is consistent with runner (`pessimistic_paisa > 0`).
  - The CSCV segment-sum shortcut is exact under the |total| ≤ i64::MAX guard.
  - Fold partition covers every later period.
  - CSCV is absent (`Unmeasured`, not a pass) when no even segment divisor exists, and this is reported, not hidden.
- **index_consistency / index_consistency_store:**
  - Weekday arithmetic is correct (day 0 = Thursday, Monday ≡ 4 mod 7).
  - V1–V4 day rules, ratio basis and decided-days-empty behaviour.
  - Summary/Week/Evaluation decode reconciliation.
  - Store decode extents and the per-period session digests.
- **global_replay (V1), global_replay_v2, global_replay_v3:**
  - V2's live read path: `open_read`, `index_replay_completions_v2`, `validate_completion_blocks_v2` and `schedule_streams`. These do monotonic blocks, an ordinal check, foreign-replay refusal, full scheduler reconstruction and counter reconciliation.
  - V1 has no production caller, and V3 is `#![expect(dead_code)]` with no caller. Their known perf items (c4a-5) were not re-reported.
