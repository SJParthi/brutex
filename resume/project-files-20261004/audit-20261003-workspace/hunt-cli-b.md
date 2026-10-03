# hunt-cli-b: cli checkpoint, codec, OOS, VIX and argument audit at 1087e54

## Verdict

I read the shared checkpoint journal (`search_checkpoint.rs`), the checkpoint and resume code of the AND v2, expression, Boolean campaign, grammar, qualified-search and index-stop searches, the record codecs and their version handling, the OOS split in `boolean_oos_*`, the VIX reference door, `fold_audit`, `knobs`, `strict_range_*` and the dispatch in `main.rs`/`lib.rs`. I found no new high or medium defect.

- **Two known defects are still open at HEAD:** the torn `complete` marker (GAP11-0) and publishing past `DIRECTORY_LIMIT` with no check (W2-cli13-5).
- **One new low defect:** `expression-search-stored` writes a fresh checkpoint on every rerun, even when the search is already exhausted. Every other search resumer returns early in that case. Each rerun therefore grows a history that is capped at 1,000,000 links and fully re-verified on every run.
- **Resume is sound where I could prove it.** A Rust probe paused the grammar cursor at every chunk size from 1 to 4096 nodes (29,767 resumes). Each resume went through encode, decode and continue, and every walk gave the same 3,000 candidates and the same work count as the uninterrupted walk.
- **Argument parsing refuses cleanly.** Unknown words, the wrong number of arguments, flags, zero values and non-UTF-8 arguments are all refused by name with exit 2. Nothing falls back to a default.
- **No OOS leakage or VIX leak found** by reading the code. Neither is proven by a probe: both paths need stored data and a clean build stamp.
- **The "known failing" test passed when run alone** at HEAD (149.5 s), so its failure depends on the environment or on load.

## Findings

| id | severity | crate | file:line | what is wrong | evidence | status |
|---|---|---|---|---|---|---|
| hunt-cli-b-1 | low | cli | crates/cli/src/expression_search.rs:531-564 | `execute` publishes a final checkpoint on every call, outside the loop and with no check for progress. Rerunning an already exhausted search (the loop body never runs), or a run that ends right after a candidate checkpoint, appends one more journal entry. So reruns are not idempotent on disk (CLAUDE.md §3 rule 5). Each entry uses up one of the 1,000,000 journal reservations and history links, and `verify_history` walks the whole chain twice per run (:261, :295). That walk costs O(checkpoints + saved child rows), and each candidate's evidence file is read at up to 64 MiB. Other resumers do this correctly: `boolean_grammar_campaign.rs:178` (`if state.exhausted { ... return Ok(()) }`) and `boolean_campaign.rs:422` (`if state.status == Status::Completed { ... return Ok(receipt) }`) both return without publishing. | `while !state.exhausted && ... { ... }` (:531), then `state.previous = previous; state.last = None; journal.publish(&state.encode(), CHECKPOINT_MAX)?;` (:562-564), with no condition. | NEW. Related to KNOWN:fix-queue_lane1-b.md W2-cli13-5 (DIRECTORY_LIMIT), which names expression_search.rs:551 as a per-candidate publisher, not this no-progress append. The O(history) verify cost is DOCUMENTED: 06-limits.md:8616-8617. |
| hunt-cli-b-2 | info | cli | crates/cli/src/search_checkpoint.rs:79-89 vs :135-145 | The format allowlists disagree. `Journal::open` admits `and-checkpoint-v2`. `Snapshot::open_through`, the read-only observer, does not: it still lists only `and-checkpoint-v1`, which nothing writes since D-0712. No reader of AND v2 exists today (grep across crates/ finds the string only in search_checkpoint.rs and and_checkpoint.rs). A future read-only observer would refuse with "unknown checkpoint format namespace". The v1 namespace also stays writable although no producer uses it. | Snapshot: `"and-checkpoint-v1" \| "expression-search-v1" \| ...` (no v2). Journal: `"and-checkpoint-v1" \| "and-checkpoint-v2" \| ...` | NEW (latent) |
| hunt-cli-b-3 | info | cli | crates/cli/src/boolean_search_integration_tests.rs:54 | The known failing test `boolean_search_command::integration_tests::generated_search_recovers_same_ordinal_and_refuses_missing_ancestry` passes when run alone at HEAD. So its failure is not a deterministic logic defect in the ordinal or ancestry path. It points at an interaction with the full-suite run: load, a shared environment, or build-stamp state. The test spawns its own test binary as a child with `env_clear()` and has no wall-clock bound (:85-131). | `cargo test -p cli --lib generated_search_recovers_same_ordinal_and_refuses_missing_ancestry -- --nocapture` printed `test ... ok` and `test result: ok. 1 passed; 0 failed; ... finished in 149.53s`. The build printed `warning: cli@0.1.0: cli run persistence disabled: an untracked file could affect compilation: crates/api/tests/zz_audit_attacksweep_2.rs`. | Input for the thread that is root-causing it |
| hunt-cli-b-4 | info | cli | crates/cli/src/lib.rs:10303-10308 | `BRUTEX_VALIDATE` treats every value except a trimmed `0` as ON. So `false`, `off` and `no` silently mean "validate", and the value is not recorded through `knobs::refuse`, so the `!! KNOB REFUSED` block never names it. This is the safe direction and is stated in the doc. The strict path refuses anything except `0`/`1` (strict_range_knobs.rs:67). | `raw.is_none_or(\|value\| value.trim() != "0")` | DOCUMENTED: lib.rs:10300-10302 "A malformed or unexpected value leaves it ON" |
| hunt-cli-b-5 | info | cli | boolean_search_command.rs:111-115 vs :103-108, expression_search.rs:49-55, lib.rs:2437 | Numbers are admitted inconsistently. `positive()` in boolean_search_command requires the canonical form (`n.to_string() == raw`), but `month()` in the same file, `expression-search-stored`, `boolean-oos-stored` MAX_POINTS and `sweep`/`audit` accept a leading `+` (Rust `FromStr`). No value is defaulted. Two spellings can only map to one value. | Probe 1: `["sweep","+1","+10"] -> code 1: refused=false` (accepted and run), while `" 1"` is refused. | NEW (cosmetic) |
| re-1 | medium | cli | crates/cli/src/search_checkpoint.rs:262-266, 459-465, 487-490 | Still not fixed at HEAD: the `complete` marker is written as `File::create_new(...)` followed by `write_all(&seal)`. A kill between the two leaves a 0-byte marker. `discover_through` counts it as acknowledged, and every later `read` refuses with "marker width mismatch", for every consumer. | `let mut marker = File::create_new(directory.join("complete")).map_err(error)?; marker.write_all(&seal).and_then(\|()\| marker.sync_all())` | KNOWN: fix-queue_lane1-b.md GAP11-0 (NOT-FIXED in audit-20261003 v1) |
| re-2 | medium | cli | crates/cli/src/search_checkpoint.rs:214-229 | Still not fixed at HEAD: `publish_inner` compares no sequence against `DIRECTORY_LIMIT`. Only boolean_search_command's `admit` (:524-528, `DIRECTORY_LIMIT - 1`) and boolean_campaign's `MAX_CHECKPOINTS` guard against it. `expression_search` and `and_checkpoint` have no guard. | `let sequence = self.next; ... fs::create_dir(&directory)` with no limit check | KNOWN: fix-queue_lane1-b.md W2-cli13-5 |

## Probe evidence (Rust; both probe files deleted afterwards)

**Probe 2: grammar cursor resume.** File `crates/vocab/tests/zz_audit_hunt-cli-b_2.rs`, run with `cargo test -p vocab --test zz_audit_hunt-cli-b_2 -- --nocapture`. Alphabet `[0, 369]`. A reference walk ran uninterrupted to 3,000 candidates. Then, for each chunk size, the walk paused every `chunk` nodes, encoded the cursor, decoded it and continued only from the decoded copy:

```
chunk 1: 3000 candidates identical after 29767 encode/decode resumes; work 29767 vs uninterrupted 29767
chunk 2: 3000 candidates identical after 16051 encode/decode resumes; work 29767 vs uninterrupted 29767
chunk 3: 3000 candidates identical after 11348 encode/decode resumes; work 29767 vs uninterrupted 29767
chunk 5: 3000 candidates identical after 7515 encode/decode resumes; work 29767 vs uninterrupted 29767
chunk 7: 3000 candidates identical after 5780 encode/decode resumes; work 29767 vs uninterrupted 29767
chunk 4096: 3000 candidates identical after 3000 encode/decode resumes; work 29767 vs uninterrupted 29767
bad magic -> Some(Cursor)
finished flag 2 -> Some(Cursor)
digit overflow -> Some(Cursor)
duplicate offered -> Some(Alphabet)
test result: ok. 1 passed
```

- **What this proves:** at the cursor level, resuming after a pause at any node boundary gives the same candidates and the same work count as an uninterrupted walk. Malformed headers, digits and alphabets are refused.
- **What it does not prove:** the exhausted end state. Reaching `finished` needs the whole 1,151-instruction grammar to be enumerated, which is out of reach. That is DOCUMENTED in 06-limits.md:8612-8617.

**Probe 1: CLI argument admission.** File `crates/cli/tests/zz_audit_hunt-cli-b_1.rs`, run with `cargo test -p cli --test zz_audit_hunt-cli-b_1 -- --nocapture`. The calls go through `cli::run` and `cli::run_durable_os`:

```
["--help"] -> code 2: refused: `--help` is not a command this build knows
["sweep", "1"] -> code 2: refused: `sweep` is a command, but not with 1 argument. ...
["sweep", "1", "10", "--verbose"] -> code 2: refused: `sweep` is a command, but not with 3 arguments. ...
["sweep", "1", "10", "10"] -> code 2: refused: `sweep` is a command, but not with 3 arguments. ...
["sweep", "0", "10"] -> code 2: refused: SESSIONS is outside 1..=3650
["sweep", "1", "0"] -> code 2: refused: MIN_HITS must be 1 or more; 0 would disable extinction
["sweep", " 1", "10"] -> code 2: refused: SESSIONS is not a whole number
["boolean-oos-stored"] -> code 2: refused: `boolean-oos-stored` is a command, but not with 0 arguments. ...
["boolean-qualified-search-stored", "a"] -> code 2: refused: ... not with 1 argument. ...
["sweep","+1","+10"] -> code 1: refused=false
non-utf8 -> code 2: refused: argument 2 is not valid UTF-8 (read as `�1`); every command word is text, so nothing was dispatched
```

Every dispatch arm is a slice pattern of fixed length (lib.rs:2174-2318), so an extra, duplicated or flag-like argument always changes the arity and is refused. No command has optional flags, except the optional 18th TIMEFRAMES argument of `boolean-qualified-search-stored`. That argument is parsed by `RungScope::new`, which refuses empty, duplicate and unknown rungs (boolean_rung_scope.rs:16-31).

`git status --porcelain` afterwards shows no hunt-cli-b file. The untracked `zz_audit_*` files that remain belong to other workers.

## Checked by reading, with no defect found

- **AND checkpoint v2 crash windows** (and_checkpoint.rs:252-277, 513-536).
  - A kill in the middle of a level leaves orphan chunks that name the boundary before them. `recover` reads at most that boundary and rejects an orphan that is not the next depth.
  - A depth-1 orphan with `previous == 0` restarts the walk cleanly.
  - The boundary decoder enforces exact length, ascending chunk sequences that all precede the boundary, and nonzero lengths. `Replay` checks seal, length, depth and index for each chunk.
  - The final reopen compares the acknowledged (sequence, seal).
  - Gap: no probe, because `walk_within` is `pub(crate)`.
- **Qualified-search record codec** (boolean_search_record.rs:57-73, 164-207, 365-374, 415-519).
  - The record magic must pair with the declaration magic, and each projection version has an exact `spec_bytes`.
  - V4 checks the index-consistency policy digest, the reason padding must be zero, and the predecessor must be all-or-nothing.
  - `transition` (boolean_search_reader.rs:397-436) forbids skipping, repeating or reassigning an unfinished reservation.
- **Expression checkpoint** (expression_search.rs:432-507, 628-737).
  - `PAYLOAD_BYTES` matches the encoder's byte count exactly: 8+CURSOR_BYTES+32+40+32+8+32+8.
  - The decoder checks the flags, the candidate reference and that the exhaustion flag agrees with the cursor. `verify_transition` replays at most 4,096 nodes for each link.
- **OOS split.**
  - `boolean-oos-stored` refuses `later_from <= to` (boolean_oos_command.rs:56-63).
  - `oos::request` rechecks against the training context, and requires the same commit (boolean_oos_v1.rs:200).
  - The OOS identity binds the training identity, completion, cohort, anchors and training months (boolean_oos_v1.rs:278-319).
  - The qualified, search and index-stop paths enforce the same strictly-later rule: boolean_qualified_command.rs:55, boolean_search_command.rs:69, index_stop_search.rs:548-550.
  - UNVERIFIED by probe: it needs stored months and a clean build stamp.
- **VIX stays reference-only.**
  - `VixReferenceMonth::open` hard-codes `NSE-INDIAVIX/1min` and refuses if that key is ever sweepable (vix_reference.rs:85-95).
  - Global replay V1 keeps VIX out of its execution identity; the test is `vix_changes_publication_but_not_selection_pnl_or_replay_identity` at global_replay.rs:4917.
  - V3 puts VIX stamps only into `money_id` and `publication_id`, which are publication hashes, not run identity (global_replay_v3.rs:1181-1235). The module doc says so (global_replay.rs:14).
  - `index_stop_vix` is a separate namespace (index_stop_vix.rs:1-5).
  - Grammar alphabets are limited to `BitStatus::Live` rows (vocab expression_search.rs:124), and VIX is not a vocabulary row.
- **fold_audit two-cursor pairing** (fold_audit.rs:216-316). I traced missing-middle, extra-bar, shifted-bar and tail cases by hand and each is labelled correctly. `agrees()` requires equal counts, no named disagreement and nothing elided.
- **knobs and strict knobs.** `resolved()` refuses whenever `knobs::refused()` holds anything (strict_range_knobs.rs:174-180). So a strict-validator/reader mismatch cannot pass through silently on that path.

## Hot-path table

| path | file:line | unit | cost | verdict | why, or how it could be O(1) | documented |
|---|---|---|---|---|---|---|
| Journal open / Snapshot open discovery | search_checkpoint.rs:418-476 | one open | one `read_dir` plus a stat for each reservation, up to 1,000,000 | NOT O(1); bounded | Directory discovery is cold. A sealed "latest" pointer would make it O(1). | module doc :3-4; 06-limits D-0524; prior o1cli-48 |
| Expression search `verify_history` | expression_search.rs:628-711 (called at :261 and :295) | one invocation | O(checkpoints + saved candidate rows), each evidence file read at up to 64 MiB | NOT O(1) | Re-verifies all of history twice per run. Hunt-cli-b-1 makes the history grow on no-progress reruns too. | 06-limits.md:8616-8617 |
| Expression `verify_transition` | expression_search.rs:713-737 | one checkpoint link | at most 4,096 grammar nodes | bounded | Fixed replay cap. | 06-limits.md:8615-8616 |
| Grammar cursor advance | vocab expression_search.rs:174-226 | one node | O(1), except the sibling comparison in `place`, which is bounded by length (≤1151) | bounded | | 06-limits.md:11331-11344 |
| Cursor encode/decode | vocab expression_search.rs:278-376 | one checkpoint | O(CURSOR_BYTES = 3,086) plus a replay of `place` for each prefix slot | bounded | Fixed width. | 06-limits.md:8614 |
| AND v2 recover | and_checkpoint.rs:252-277 | one resume | the newest entry plus at most one boundary read; replay streams every named chunk | NOT O(1); O(history bytes) on resume | Rebuilding the engine checkpoint needs every level. | module doc :16-23, D-0712 |
| VIX stamp lookup | vix_reference.rs:255-268 | one trade timestamp | arithmetic plus one slot read | O(1) | Opening a month is O(records) once. | module doc :17-26, labelled UNVERIFIED as measured |
| fold_audit compare | fold_audit.rs:216-316 | one rung-month | O(stored + folded) | bounded linear | A full comparison is inherently linear. | module doc "# Cost" |
