# Crash and edge-input audit, pass 17: repair and recovery tools fed damaged input

Head: `/home/claude/wt/zero3` @ **1f4de71** (origin/final/all-fixes-zero). Audit only. Nothing in any checkout was edited, and no cargo was run, because no finding is medium or high.

Theme: the tools that repair, heal, truncate, re-seal, quarantine or discard. Each was given the damage it exists to fix, plus damage it does not expect. The tools covered are:
- store revisions (`store/src/repair.rs`, `REPAIR.md`);
- api spot recovery (`api/src/recovery.rs`, `recovery_journal.rs`, `recovery_control.rs`, `/pull/recovery`, `/pull/recovery.json`);
- the degraded census install (`pull/src/ingest.rs:819-858`, `install_census`) and the manifest loader;
- the cli tail tools (`fixed_tail::{heal_torn_tail, heal_torn_header, discard_orphan, roll_back}`, `append_rollback`, Selection V6 `set_aside_abandoned_tail`);
- the strict checksum audit (`store/src/checksum_audit.rs`), used by `checksum-audit`, `sweep-audited-stored` and `audit-audited-range`.

Not re-reported: pass-9 IDs (CE-61..63 and its latent notes, including `repair.rs` FIFO opens), P14-05 (REPAIR.md V2 wording), store2-1 (burnt reservation), the recovery-1..6 rows, cli2-1, conc4-*/conc5-*, and the torn-header class in conc-pass5.

## Verdict

**NOT ZERO. 3 low, 0 medium, 0 high.** There is no panic route, and no case where a tool reports success over a file it left unreadable.
- **Store revisions:** refuse every damage shape loudly and never write the original.
- **API recovery:** refuses every damaged journal loudly. It never reseeds or refunds budgets, and boot logs "Recovery NOT resumed" rather than crashing.
- **The three findings are all in cli-side healing:**
  - one wedge that D-1569 was meant to remove comes back after a second crash (CE-88);
  - two writers cut a renamed foreign file before refusing it, against D-1901's own promise (CE-89);
  - the strict audit refuses a state the store format calls benign, gives no reason, and no tool can clear that state (CE-90).

## New findings

### CE-88 (low): Selection V6's abandoned-tail quarantine is named by offset only, so a second interrupted write at the same offset wedges the rung again

- **Where:** `crates/cli/src/selection_v6.rs:392-405`, called from `persist` at :313 and :341.
  ```rust
  let aside = root.join(format!("{FILE_NAME}.abandoned-{committed}"));
  ...
  Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
      let held = std::fs::read(&aside)...;
      if held != tail {
          return Err(format!("Selection V6 abandoned tail quarantine {} already holds different bytes; nothing repaired", ...));
  ```
- **Why it is wrong:** the committed length only changes when a block commits. So two interrupted writes with no commit in between land at the same `committed` offset. The repro below shows the second quarantine colliding with the first; after that every `persist` refuses. That is the exact wedge D-1569 ("not left to wedge the rung") removed. D-1569 accepted the refusal for a different-bytes quarantine, but not the consequence that a second crash brings the wedge back. The refusal is loud, but the only way out is for an operator to move the file by hand.
- **Repro (not run, traced from source):**
  1. Source A's persist is killed after its payload write. This leaves an unsealed tail tA at offset X.
  2. Source B runs `persist`. tA is not a prefix of B, so it goes to `.abandoned-X` and is cut. B is then killed mid-write, leaving tail tB at X.
  3. Source C (or A again) runs `persist`. The prefix differs, so it tries `.abandoned-X`, which exists and holds tA, not tB. Result: `Err`.
  4. Every source except B's exact retry now refuses at :404.
- **Minimal fix:** name the quarantine by offset plus a content digest, for example `abandoned-<offset>-<blake3(tail) hex[..16]>`. Keep the "same name, same bytes" idempotence check. Add a test that runs the A-B-C sequence above.

### CE-89 (low): Execution V4 and Anchored Search Lineage V4 cut a renamed or foreign file before refusing it, contrary to D-1901

- **Where:**
  - `crates/cli/src/execution_v4.rs:2519`: `crate::fixed_tail::heal_torn_tail(file, path, 0, stride, &[])?;`
  - `crates/cli/src/anchored_search_lineage_v4.rs:747`: `crate::fixed_tail::heal_torn_tail(file, path, 0, width, &[])?;`
  - Both run in the writable open, before any record is decoded.
- **Why it is wrong:**
  - D-1901 (docs/05-decisions.md:57059) says the writer "cuts only when the file begins with its own magic (and version where the header carries one), so a foreign or legacy file is never touched". `fixed_tail.rs:259-262` says the same.
  - These files are headerless, so they pass an empty magic and `heal_torn_tail` skips the check. Yet every record does begin with a 16-byte magic: `MEMBER_MAGIC` / `COMPLETION_MAGIC` (anchored :48-49) and `PARAMETER_MAGIC`..`COMPLETION_MAGIC` (execution_v4 :88-91).
  - Example: a V3 member file (512-byte stride) renamed to the V4 name (768-byte stride) with 5 records, 2,560 bytes. The writer cuts 2,560 mod 768 = 256 bytes of a valid V3 record, syncs the cut, and emits "bytes past the last whole record were never acknowledged". Only then does the decode refuse.
  - The other ledgers that pass `&[]` (candidate_universe:6141, base_evidence_ledger_v2:1681, pre_admission_data:3536/3771) call `verify_header` first, so they are safe.
- **Repro (not run):** this follows from source. Nothing between `open_child` and `heal_torn_tail` reads any content.
- **Minimal fix:** pass each file's record magic as `magic`. Make `heal_torn_tail` compare only `min(found, magic.len())` leading bytes, so that a first record torn inside its own magic is still cut.

### CE-90 (low): the strict checksum audit refuses a month that holds the interrupted-append extent the store format calls benign, gives no reason, and no tool can clear it

- **Where:** `crates/store/src/checksum_audit.rs:286-296`.
  ```rust
  || before.data.len != data_bytes
  || before.sidecar.len != sidecar_bytes
  ...
  return Err("strict checksum audit refuses unsealed, unsupported, non-exact or over-limit input extents".to_owned());
  ```
- **Why it is wrong:**
  - `docs/02-store-format.md` §7 (:388-395) says a crash between record write and header commit leaves "bytes past the commit". It calls this state interrupted, not damaged: the month opens, ordinary readers serve it, and D-0189 forbids truncating it. D-0688 adds that the sidecar may already hold the tail entry, sealed past the commit.
  - The strict audit, behind `checksum-audit`, `sweep-audited-stored` and `audit-audited-range`, refuses exactly this state. Its one message covers five different causes, so the operator cannot tell "interrupted append" from "unsealed" or "over limit".
  - Nothing in the store or the cli clears the extent. Only a later append at least as long as the dead tail overwrites it. A closed historical month whose retry fetched fewer bars, or whose retry never ran, therefore stays unauditable for good, while the normal path still serves it.
- **Repro (not run):** follows from source. `Generations::read` takes `metadata.len()`; the comparison is exact.
- **Minimal fix:**
  - Return a separate refusal for `data.len > data_bytes` and `sidecar.len > sidecar_bytes` that names the store's "bytes past the commit" state and the byte counts.
  - Then either audit only the committed extent, verifying the tail block through the D-0688 proof that `verify_block_of` already implements, or record in `docs/06-limits.md` that such a month needs a covering re-append before it can be audited.

## Sites checked and found sound

| Tool | Damage fed | Result |
|---|---|---|
| store `repair::publish` | torn source tail block, header/sidecar disagreement, short or long sidecar | refused by `BarFile::open_existing` / `read_record` (block CRC) inside `retain_timestamps`. Nothing reserved, original never written |
| same | two valid header generations | the newest wins. A caller snapshot of the older one gets `StaleSource` before reservation |
| same | v2 (unsealed) or v1 source | `check_header` gives `UnsealedSource`; v1 is `RetiredVersion` |
| same | renamed month, different symbol | `open_existing(symbol_id)` refuses. The receipt binds `symbol_id`/timeframe at `RevisionReader::open:177-182`. A vendor rename is the store-wide no-vendor-in-header limit, not new |
| same | interrupted mid-publish, run again | reservation present, receipt missing, so `Incomplete`. Preserved and never resumed (documented). An exact completed retry is `Reused` and idempotent and rewrites no bytes |
| same | torn or foreign receipt, changed revised header, missing revised `.crc` or `.lock` | `InvalidReceipt` / `Incomplete` / Io, all loud. No arithmetic panic; `split_at` works on a fixed 144-byte array |
| same | ancestor-sync loop | ends at `root` or `parent()==None`, which gives `Incomplete` |
| api `recovery_journal` | torn tail, CRC flip, unknown version or status, nonzero reserved bytes, a key bound to a different body | refused on open and on `snapshot`/`tail`, never truncated (documented; conc-pass11 ENOSPC row) |
| api recovery | `active.bin` / plan / `attempts.bin` / `.stop.bin` renamed from another plan | `validate_seal` binds the scope identity, `active_history` binds pointer keys, `state()` binds the stop key. All refuse |
| api recovery | plan missing after activation | `missing_plan` message, then the successor path. Successor refuses if the predecessor file exists, if it was already activated, or if the scope changed. An interrupted prepare re-runs idempotently (`append_new([])` then the control row) |
| api recovery | corrupt or missing journal at boot | `resume` returns `Err`, and `server.rs:19249` logs "Recovery NOT resumed". No abort |
| api recovery | damaged source month during assess | `read_stamps` refuses, giving `Blocked` plus an audit journal failure row. Never reported as Verified. A dangling `.bin` symlink reads as absent, and stays unverified rather than verified |
| pull census degraded load | newest slot CRC bad | loads the older generation. The run reports a named failure and the whole image is published by rename. Entries are recounted from the bar file on the next touch |
| cli `fixed_tail` users with magic (results, frontier, trades, result_set, admission_store, institutional_statistics, stored_data_completeness, observation V1/V2, sweep_evidence) | foreign or legacy file | leading magic+version check, so nothing is cut |
| cli `discard_orphan` (Population, Admission V3/V4, Statistics V2/V3, Finalization V4) | trailing block with no receipt that is not the exact retry | cut only under the exclusive lock, only after the scan proves the prefix, and only when no receipt of any version names the block. A damaged whole receipt refuses at scan, not discard |
| cli `append_rollback` / `roll_back` | short write | truncates to the pre-write length; a failed rollback names both errors |

Counts: new 3 (CE-88, CE-89, CE-90, all low, not run). No verification rows were requested for this pass.
