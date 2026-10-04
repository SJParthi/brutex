# numeric-pass17 (num17): counting and sizing the search

Target: /home/claude/wt/zero3 @ 1f4de71 (origin/final/all-fixes-zero). Audit only; nothing in any checkout was edited.
Probes ran in a throwaway worktree (scratch-num17, now removed), release profile.

Scope: vocab::expression_search (Cursor, CURSOR_BYTES, MAX_INSTRUCTIONS); vocab::expression (decode canonical order);
cli expression search (expression_search.rs + reader); Boolean-grammar batches and the qualified search
(boolean_grammar_batch.rs, boolean_search_command.rs, boolean_search_record.rs, boolean_search_projection.rs);
runner::search_allocation_v1; cli boolean_search_sizing.rs; candidate_universe.rs counts; the hypothesis counts that feed
Romano-Wolf / alpha spending. Pass-7 (Apriori) and pass-8 (correction formulas) IDs are not re-reported.

## Verdict

1 new finding: p17num-1 (medium). The enumeration, the resume path, the cursor encoding, the size bound and the count
arithmetic all hold, and probes confirm them.

## Checks

| # | Question | Result | Evidence |
|---|---|---|---|
| 1 | Every program exactly once, including across checkpoint/resume | HOLDS | The probe ran `crates/vocab/tests/num17_probe.rs`. Independently, it enumerated every token string over {Bit(a_i), NOT, AND, OR} and filtered them with a stack/tree validator that applies the same sibling rule (`left <= right` under derived `Instruction` Ord). Alphabet 1, lengths 1..8: per-length counts [1,1,3,5,13,29,77,191]. Alphabet 2: [2,2,8,16,54,150,502,1574]. Alphabet 3, lengths 1..7: [3,3,15,33,135,423,1671]. The search output equals the brute-force list element for element, in the same order (length, then rank-lex), with no duplicates. A second run did `decode(encode())` before every 1-node step and produced the same sequence. |
| 2 | Space-size formula | HOLDS (lower bound only) | The only published size is `2^L - 1` (boolean_search_sizing.rs:51-73, D-0580). Each sorted left-deep conjunction is canonical: at every AND, the left subtree starts with `Bit(b1) < Bit(bk)`. Each needs `2k-1 <= 2*328-1 = 655 <= MAX_INSTRUCTIONS = 1151`. The probe shows the live alphabet is 328, matching D-0580 and docs/32 (5.47e98). The decimal builder (116 digits) holds 2^384-1, and `leaves > 64` is exactly the condition for `2^L-1 > u64::MAX`. No total grammar size is claimed, and none is computed. |
| 3 | Hypotheses fed to correction = hypotheses evaluated | HOLDS inside one declared search; FAILS across a forced re-declaration (**p17num-1**) | Inside one batch: `boolean_qualification_v1::measure` (:559-581) puts every later coordinate into the RW family, zero-trade and execution-refused rows included, and refuses `returns.len() != manifest.count`. Empty, refused and retried batches keep their ordinal (boolean_search_record.rs:233, boolean_search_command.rs:281-299). The search projection rescales the raw RW p by `8(b+1)(b+2)` (boolean_search_projection.rs:45-56). The expression search counts every emitted candidate (`state.candidates`), and `verify_history` reconciles it link by link. |
| 4 | u64/u128 overflow at real sizes | HOLDS | Cursor fields are u16, with `count+3 <= 387` and `length <= 1151`. The work counter uses `checked_add`, which gives `Refusal::WorkOverflow`. Expression-search counters use `add()`, which is checked. Batch sizes are checked (boolean_grammar_batch.rs:43-47). Candidate-universe counts (`closed*2`, `closed*cells`) are checked (candidate_universe.rs:5266-5287, 2264-2275). Boolean statistics shape (`candidates*periods*draws*3`) is checked and refused before work (boolean_statistics_v1.rs:290-345). search_allocation widens to u128 and checks every product. Each reaches a named refusal, never a wrap or a saturation. |
| 5 | Cursor encoding canonical | HOLDS | `PartialEq` and `Debug` read the encoding. The probe built every byte string for alphabets 1-2, lengths 1-4, every `at < length` and every digit vector in `0..=count+3` up to `at`. Decode accepted 80 and 228 states, and all of them are states the live search reaches; it accepted 0 unreachable ones. Bytes past `count`/`at`, the pad byte and the finished state are all pinned (expression_search.rs:311-368). Two reachable states can lead to the same next candidate (for example, `digits[at] == count+3` versus the state right after backtracking), but they are different node-work positions, and `verify_transition` replays the exact work delta, so this is not an alias. |

## New findings

### p17num-1 (medium): raising a physical record or byte ceiling restarts the countable alpha sequence over the same programs and data, and the search's own refusal makes that raise unavoidable

- **Where:**
  - cli/src/boolean_search_record.rs:97-99 and :137-161: `Spec::identity` hashes `encode()`, which includes `self.bytes` and `self.records`.
  - cli/src/boolean_search_command.rs:205-225: `bytes`/`records` come from `input.strict` (BRUTEX_CHECKSUM_MAX_BYTES / _MAX_RECORDS), then `let identity = spec.identity(); ... Journal::open(request.input.output, NAMESPACE, identity)`.
  - Same pattern in the index-stop search: cli/src/index_stop_search.rs:679-694 (`config.capture.records`, `config.capture.bytes`, `config.strict.max_bytes()`, `config.strict.max_records()`, `request.batch_programs`, `request.node_allowance` inside `legacy_declaration`) and :401 `Journal::open(request.root, NAMESPACE, hash(declaration))`.
- **Quoted code (boolean_search_command.rs:554-558):**
  ```rust
  if count
      .checked_mul(record.spec.nodes)
      .is_none_or(|n| n > record.spec.records)
  {
      return Err("search complete-history replay allowance reached before pricing; no allocation was recycled".into());
  ```
  `count` is `journal.acknowledged() + needed + 1`, and each batch adds at least two records.
- **Why it is wrong:**
  - Every declared search eventually stops on this refusal. With RECORDS=10,000 and NODES=100 that happens after about 50 batches. index_stop_search_checkpoint.rs:215 (`journal.acknowledged() >= budget.records`) does the same.
  - The only way to continue is to raise the record (or byte) ceiling. That changes `Spec::identity`, opens an empty journal, and starts again at ordinal 0 from `request.initial`.
  - Batch 0, with the largest share `alpha/16` per rung, then re-tests exactly the programs and data the old declaration already examined. So does batch 1 with `alpha/48`, and so on.
  - Each re-declaration therefore spends a fresh alpha on hypotheses that were already tested. The family-wise error over what the operator actually looked at becomes `k*alpha` after k re-declarations.
  - This contradicts the allocation contract at runner/src/search_allocation_v1.rs:10-12: "refused, skipped and unexecuted slots never replenish another slot. Changing directories, seeds or batch sizes must not restart the spending sequence".
  - It also contradicts docs/30-search-wide-qualification.md (Restart row: must not claim "Retrying starts a fresh easier testing budget"). That document's list of things that legitimately create a new search ("data, grammar, batch boundaries, policy or statistical procedure") does not include physical admission ceilings.
  - The refusal text itself says "no allocation was recycled", but the only path forward recycles the whole allocation.
  - `bytes` and `records` do not change which programs a batch contains. `Budget.bytes = bytes/4` is only a capacity check (boolean_grammar_batch.rs:43-49).
- **Repro (ran):** a throwaway test was appended to cli/src/boolean_search_record_tests.rs and run with `cargo test --release -p cli --lib num17_`:
  - Setup: `generated_record().spec` cloned with `bytes*2` and `records+1`.
  - Both specs validate. `identity()` differs.
  - `Batch::prepare(spec.cursor(), 0, 0, spec.budget())` yields identical programs and an identical next cursor.
  - `search_allocation_v1::allocate(0, rung, alpha).threshold()` is identical for all 8 rungs.
  - Output: `NUM17 identities differ, batch-0 programs equal: 2`, `test result: ok`.
- **Minimal fix:**
  - Remove `bytes` and `records` from `Spec::encode` (boolean search), and the capture/strict byte and record limits from `legacy_declaration` (index-stop). They are read admissions, the role `ReadBudget.history_bytes` already plays in index-stop. Keep them as per-invocation admission only.
  - Batch-shape knobs such as `programs`/`nodes`/`batch_programs`/`node_allowance` (which do change batch boundaries) can stay in the identity, but only together with an explicit statement that changing them is a new search.
  - Bind the alpha sequence to a key that excludes all of these: the sources, policy, alphabet and rung scope. One option is a per-OUTPUT spend ledger keyed by that, which refuses a second declaration over an already-spent prefix.
  - Add a test: raising BRUTEX_CHECKSUM_MAX_RECORDS after the replay refusal resumes the same journal at the next ordinal.

## Verification of prior items in this scope (state at 1f4de71)

| Item | State | Evidence |
|---|---|---|
| o1engine-23 / D-0983 incremental `place` instead of whole-prefix re-walk | FIXED | vocab/src/expression_search.rs:208 (one `place` per choice), :222-245; exhaustive equivalence test :455-497; the num17 brute force agrees on full programs |
| D-0750 cursor equality is the encoding (scratch ignored) | FIXED | expression_search.rs:77-83 |
| D-0754 `Debug` prints encoded fields only | FIXED | expression_search.rs:85-108 |
| D-1562 / hunt-cli-b-1 exhausted expression search appends no checkpoint on rerun | FIXED | cli/src/expression_search.rs:537-539 |
| D-0601 index-stop batch byte admission at admission time | FIXED | cli/src/index_stop_search.rs:528-545, called at :570 |
| p2bool "sizing decimal builder exact, no floats" | FIXED (still holds) | boolean_search_sizing.rs:51-73; 328-leaf decimal pinned in its test |
| p8 "countable alpha spending telescopes to alpha" | FIXED (formula); scope gap is p17num-1 | runner/src/search_allocation_v1.rs:157-176 |

## Notes (not findings)

- Structurally distinct but semantically equal programs (`a`, `!!a`, `a&a`) are separate members of an RW family. This is conservative, and vocab/src/expression_search.rs:1-9 states it.
- Within a batch, selecting a subset of rungs still charges `alpha/8` per selected rung ("unselected units are unspent"). This is conservative.
- `minimum_draws` grows as `8(b+1)(b+2)/alpha`, so later ordinals become unreachable with a fixed number of draws. That direction is conservative and documented (search_allocation_v1.rs:16-17).
