# slice26 — 1 finding

Slice: crates/cli/src/{results.rs, search_checkpoint.rs, selection.rs, selection_v3.rs, selection_v4.rs, selection_v4_authority.rs, selection_v5.rs, selection_v6.rs, selection_v6_source.rs} (production halves; test modules skimmed).

## F1 [low] results.rs rustdoc states the version-1/version-2 byte counts as the current record layout (seal coverage, stride, payload)
- where: crates/cli/src/results.rs:100-125 (doc on `STRIDE`), :136-141 (doc on `PAYLOAD_BYTES`), :421-447 (doc on `Record::seal_matches`), :1478 (comment in `Results::read`), :60-66 (doc on `VERSION_V2`)
- what: The ledger is now version 3: `STRIDE = 261`, `PAYLOAD_BYTES = STRIDE_BYTES - SEAL_BYTES = 253`, and `seal_of` hashes `raw[..PAYLOAD_BYTES]`, so 253 bytes. The rustdoc on these items still describes older layouts as the current one:
  - `STRIDE` (value 261) has the heading "# 213, and the last eight are the seal" and says "The eight added bytes are `blake3` over the other 205".
  - `PAYLOAD_BYTES` (value 253) says it "is also the stride version 1 used — the eight new bytes are the whole of the difference between the two versions". The same file says version 1 was 205 bytes.
  - `Record::seal_matches` says the seal is "eight bytes of `blake3` over the 205 the record actually carries", and its `# Cost` section says "One hash of a FIXED 205 bytes".
  - `Results::read` says a reader "could land in the middle of another process's 213-byte write". The write is 261 bytes.
  - The doc block attached to `const VERSION_V2: u32 = 2` opens with "Version TWO: version one had no seal". It is two docs merged onto the version-2 constant, and the current `VERSION = 3` has no doc at all.
  This is the authoritative byte description of `runs.bin`: docs/02-store-format.md never gives its stride or seal coverage (grep for 213/261/BRUTEXRS finds nothing). The private helper `seal_matches_v2` (:204-219) gets 205-versus-253 right, and the public `seal_matches` contradicts it. A reader of the public rustdoc would compute every record offset and seal range wrongly.
- evidence: a trace through the constants at :125 (`STRIDE = 261`), :141 (`PAYLOAD_BYTES = STRIDE_BYTES - SEAL_BYTES`), :150 (`SEAL_BYTES = 8`) and :183 (`PAYLOAD_BYTES_V2 + 8 * 6 == PAYLOAD_BYTES`, so 205 + 48 = 253); and `seal_of` at :260-268 hashes `&raw[..PAYLOAD_BYTES]`. The stride tests (`the_stride_is_exactly_what_the_writer_writes`) check the constants, not the prose, so nothing catches the drift.
- fix: Rewrite the four doc blocks to give the v3 figures: stride 261, seal over 253, v2 = 213 with seal over 205, v1 = 205 with no seal. Move the "Version TWO" paragraph off `VERSION_V2` and give `VERSION` its own doc. Optionally add the `runs.bin` header and stride to docs/02-store-format.md, which §10 makes the authority over bytes on disk.

## Known, still present (not re-reported)
- hunt-cli-a-5 (V6 interrupted write blocks the rung) still holds at selection_v6.rs:304-336 and :374-376.
  - A partial trailing block written by a different source makes `persist` refuse.
  - It also makes `require_committed`, and therefore `top_twenty_five`, `snapshot` and `stored_oos_witnesses`, refuse every already-committed selection in that file. Reads break as well as writes.
  - The same class exists in Selection V5 as whole-row orphans (selection_v5.rs:2100-2101, `require_exact_prefix` at :2160-2172). Rows from an interrupted commit of source A refuse every later commit of a different source B on that rung, for good. Committed reads still work there, because `scan` accepts the trailing prefix.
- W2-cli14-1/2/3 (repeated V5/V6 source replays and scans): unchanged.
- GAP11-0 and W2-cli13-5 (search_checkpoint marker torn on create, `DIRECTORY_LIMIT` not checked on publish): unchanged at search_checkpoint.rs:224-230 and :263-267.
- c4b-5 (V5 ragged append): FIXED. `append_with_rollback` at selection_v5.rs:3028-3054 (D-1622).

## What was checked with no finding
- **results.rs.** The v2/v3 dual-stride read path; the open-time identity pass; `absorb_new_records` (shrink, ragged tail and same-length rewrite all refuse); the rollback on `append_locked`, kept separate from `sync_all`; the `with_shared_writer` cache invalidation; `of_identity` and `index_of_identity` arithmetic.
- **search_checkpoint.rs.** Publish ordering (reserve, payload, sync, marker, sync, reverify); `read_saved` length/seal/marker checks; the prefix-bounded discovery in `open_prefix`. The format allowlist differs between `Snapshot` and `Journal` (`and-checkpoint-v2`), but no `Snapshot` caller uses that namespace.
- **selection.rs, v3, v4.** Cohort derivation; proof reconciliation; the two-pass keeper and resolution, including duplicate-digest refusal; the generation and append-only checks in the ledgers. The V1–V4 ledger writers have no production caller; only tests open them writable.
- **selection_v4_authority.rs.** Triple-authority page join and its bindings; matrix marginals.
- **selection_v5.rs.** Projection; rank reconciliation; winner resolution; receipt-last persistence; orphan-prefix retry; bounded file generations.
- **selection_v6.rs and selection_v6_source.rs.** Block layout: the 960-byte envelope is exact (56 + 256 + 72 + 160 + 336 + 48 + 32), winners take 25 × 296 bytes, which fits in 16,352. Also checked: enum discriminants are explicit; the family/terminal shape validation; the parameter-count rule; the seal and identity domains; the persist resume path.
- **Grep of production code** for floats, `as` casts, unwrap/expect/panic and indexing: nothing reachable. A scan of the slice's test modules for tests with no assertion found none.
