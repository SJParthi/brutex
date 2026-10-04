# Tests/docs/security pass 14 (tds14): docs/02-store-format.md and docs/01-architecture.md against the code

Checkout: /home/claude/wt/zero3 at 1f4de71. Audit only. One throwaway test was run in a scratch worktree, which has since been removed.

Counts: 6 new findings (2 medium, 4 low). 9 verification rows (8 FIXED, 1 PARTIAL).

## What was checked and matched (no finding)

- Bar slot (docs/02 §2): offsets 0/8/10/12/16/24/32/40/48/52/56/60 match header.rs:123-148. The CRC domain 0..56 plus 60..64 matches header.rs:936-946. Magic BRUTEXB3/BRUTEXB2, version 3 or 2, HEADER_LEN 32768, slot stride 16384, stride 56, 73-record 4088-byte block (format.rs:111-181, layout.rs:102-195). OI_NULL is i64::MIN (format.rs:221). The record field order matches Bar::image (format.rs:874-884).
- Block CRC: CRC-32C over only the covered prefix (block.rs:75-86), matching §6.
- Census §11.4, §11.5 and §11.5a entry offsets (manifest.rs:465-515), exchange and segment codes (manifest.rs:1211-1244), CLOSE_NULL i64::MIN (:253).
- The following match their encoders field by field:
  - §12.1/12.2 (result_set.rs:34-45, 89-105)
  - §13 frontier v7 (frontier.rs:400-505)
  - §14 V4 coverage (population.rs:2201-2210)
  - §17, all four payloads (execution_capability.rs:630-655, 878-917, 2937-2953, 3065-3120), and the 24-byte header (:2316-2323)
  - §19 header (stored_data_completeness.rs:1119-1140)
  - §27 runs.bin (results.rs:388-431)
  - §31 parameter and disposition records (execution_v3.rs:511-580, 875-920)
  - recovery journal BXRJ (api recovery_journal.rs:164-191)
  - outer invocation journal BXOPAU01 (operation_audit.rs:27-36, 180-202)
- docs/01 §1 table against all 13 manifests: exact. CLAUDE.md §5 block: exact. The six shareable crates name no external package.
- Names docs/01 uses, all present:
  - engine: `Ladder::walk_into` (engine lib.rs:1459, private), `Frontier`, `Sweep`, `Tally`
  - runner: `RankedOutcome`, `Ranked`, `Lens::{Detectability,Payoff}`, `run_prepared_ranked_by_reporting`, `is_complete`, `Forward`
  - store: `store::catalog::walk`
  - cli: `cli::pool`, `batch::sweep_under`
  - pull: `session::{Day,IstMoment,DropReason,DropCensus,Window::wire_to}`, `manifest::Manifest`, `calendar::kind_of`
  - engine and vocab: `Column::support`, and the two named tests (engine column.rs:471, vocab mask.rs:437).
- lake writes no file (no create/write/OpenOptions in lake/src), so docs/02 has nothing to describe for it.

## What gate core/tests/graph.rs covers, and what it does not

It covers only docs/01 §1's table, which it reads by `include_str!` (graph.rs:48):
- every `| \`crate\`` row against the workspace members, with invented and duplicate rows refused before the member filter (graph.rs:472-498, 617-662);
- the third cell's backticked member names against each manifest's member dependencies, in both directions (:665-685). Normal, dev, build and `target.*` tables all count (:214-227), even though the doc says the table is `--edges normal`;
- the members list against MANIFESTS (:602);
- acyclicity, by Kahn's algorithm (:688);
- the four roots having no deps (:728);
- the shareable six closing over themselves (:751).

It does not read:
- the ASCII diagram (the doc says so);
- the "Owns" column;
- the "zero external packages" claim (declared_deps filters to members, :323-340);
- CLAUDE.md §5's block;
- §2 to §6 (data flow, path, concurrency, cost table);
- any module or type name.

## Append-only check (CLAUDE.md §3 rule 8)

Everything before 2026-09-23 is squashed into ffa41c6, so `git log -p` cannot show earlier in-place edits. Since then:
- 5b47e0d minted bar v3 rather than changing v2. Correct.
- 3f22aed/D-1763 changed the doc only: census v3 and frontier v7 were already minted in code.
- 6493317 (D-1353/1354) made the v2 reader refuse a non-zero reserved field or an undefined flag bit. That tightens validation; no written byte means anything new.

No new in-place change was found. The only known breach is still BRUTXEP1/BRUX2PR1 v1 going from 640 to 648 bytes (P1-16-03, below).

---

### P14-01 medium: docs/02 §11.3 census slot table puts "reserved" over the last two bytes of `vendor`, and leaves bytes 12..16 out (ran)

- Where: docs/02-store-format.md:531-533 vs crates/pull/src/manifest.rs:457-462, :1955, :3439-3443, :2007.
- Evidence:
  - The doc has `| 48 | 8 | \`vendor\` |`, then `| 54 | 6 | reserved | zero |`, then `| 60 | 4 | \`crc\` |`. Rows 48+8 and 54 overlap.
  - The doc has no row for bytes 12..16.
  - The code has `const H_VENDOR: usize = 48; const H_VENDOR_LEN: usize = 8;`, which makes reserved bytes 56..60. `write_text(&mut out, H_VENDOR, self.vendor.as_str())` writes up to 8 bytes.
  - The decoder never checks bytes 12..16 or 56..60.
- Why it is wrong: §11 says "This section is the authority now". A reader built from it treats bytes 54..56 as reserved zero. Vendors `truedata` (8 bytes) and `zerodha` (7) write into those bytes. Such a reader either refuses those censuses as non-zero reserved bytes, or reads the vendor as `trueda`/`zeroda` and fails the vendor cross-check. The pull test only checks `slot[48..54] == b"groww\0"` (unit.rs:7557), so it passes with the wrong table.
- Repro (ran): a throwaway test printed `ManifestHeader::genesis(v).image()`:
  - TrueData: `48..56=[116,114,117,101,100,97,116,97]`, `54..60=[116,97,0,0,0,0]`
  - Zerodha: `54..60=[97,0,0,0,0,0]`
  - Both decode back as their vendor.
- Fix: change the rows to `| 48 | 8 | vendor |`, `| 56 | 4 | reserved |` and add `| 12 | 4 | reserved |`. State that the reader does not check these bytes, or refuse non-zero ones as the bar slot does (D-1353). Extend M-29 to a long vendor name.

### P14-02 medium: docs/02 never gives the byte layout of the `.crc`, `.ovl.crc` and `.grk.crc` checksum files, which version 3 makes mandatory

- Where:
  - docs/02-store-format.md:346-352: "Each block's CRC-32C lives in a sidecar at the same **block index** ... indexed by `i / 73`". This is the only description.
  - Doc §8 table, :410-418.
  - Code: crates/store/src/file.rs:2056-2064 (`// FOUR BYTES AT \`block * 4\`` ... `block.saturating_mul(4), &sum.to_le_bytes()`) and :2618 ("Four bytes at `block · 4`").
- Why it is wrong: the file has no header, no magic and no version. Each entry is a little-endian u32 at offset `4·block`, and that appears only in Rust comments. §2.1 makes the sidecar mandatory at v3, so a sealed month whose sidecar is missing is refused. A second reader built from the doc must verify blocks but cannot locate or decode an entry: it does not know the width, the byte order, whether there is a header, or the CRC parameters beyond the name "CRC-32C". This is the "Rust comment as the authority for bytes" situation that §11 says it was written to end. The same applies to `.ovl.crc` and `.grk.crc`.
- Repro: `grep -n -i 'four bytes\|block \* 4\|u32' docs/02-store-format.md` finds nothing about the sidecar (not run beyond the grep).
- Fix: add a §6 table: "`.crc`: headerless; entry *b* = CRC-32C (Castagnoli, reflected 0x82F63B78, init/xorout 0xFFFFFFFF) of the covered bytes of block *b*, u32 little-endian at `4·b`; length `4·blocks_for(n_valid)`; same layout per family at that family's records-per-block". Pin it with a store::docs test.

### P14-03 low: docs/02 states no byte order for the bar header or record, and no layout for the `.ovl` and `.grk` records

- Where:
  - docs/02-store-format.md §2 table (:66-79) and §3 (:125-140), which presents a `#[repr(C)]` struct.
  - §8 (:410-418) gives `.ovl` and `.grk` as "24-byte records, version 9" and "80-byte records, version 8" only.
- Evidence:
  - format.rs:837-845: "`#[repr(C)]` fixes the field *order*, not the byte order ... Little-endian matches the header". Every field is `to_le_bytes`.
  - Overlay: format.rs:290-304 (`BRUTEXB9`, 170 records per block) and :376-378 (`ts_micros`, `spot` in paisa or OI_NULL, `iv_micros`).
  - Greeks: :444-488 (`BRUTEXB8`, 51 per block, f64 greeks, provenance codes 0/1/2) and :634-643.
  - `grep -rn 'BRUTEXB9\|BRUTEXB8' docs/` returns nothing.
- Why it is wrong: the doc's struct reads as a native-endian memory image, which the code comment rejects. The two sibling formats have no magic, field offsets, units, sentinels or block sizes anywhere in docs.
- Repro: the grep above (not run otherwise).
- Fix: add "all integers little-endian" to §2 and §3, and add an §8.1 and §8.2 with the overlay and greeks field tables, magics and records per block.

### P14-04 low: docs/02 §9 says nothing in `crates/store` can check single-writer exclusion; §5 and the code say the store takes the lock

- Where: docs/02-store-format.md:451, `... the lock is a leaf — never held while acquiring another. Nothing in \`crates/store\` can check it.`, vs :262-268 ("The writer takes an advisory `try_lock` on the `.lock` sibling ... the lock is real") and crates/store/src/file.rs:1417 `let lock = Flock::try_lock(`.
- Why it is wrong: §5 already deleted this same clause for contradicting the lock. §9 kept it. The honest limit is cross-host exclusion on a network filesystem, which §9 does not say.
- Fix: "Enforced by `store::flock` (`try_lock` on `.lock`); not across hosts on a network filesystem."

### P14-05 low: the bar repair spec says a revision file "remains V2"; revisions are created at bar version 3

- Where: crates/store/REPAIR.md:67 `The new file's bar format remains V2; revision ordinal is not a bar format version.` (docs/02:2141-2143 delegates the protocol to this file and says "Original V2 bar geometry is unchanged"). Code: repair.rs:372 `BarFile::open_or_create(&revision_root, path, symbol_id)`, which births at `geometry_of(FileKind::Bars) = Layout::CURRENT` = V3 (file.rs:1300-1305, layout.rs:195).
- Why it is wrong: since D-1571 a revision of a v2 month is a `BRUTEXB3` file with mandatory checksums. The receipt's two embedded headers then carry different versions, and REPAIR.md says they "retain their existing format/version".
- Fix: say the revision is born at `Layout::CURRENT` (v3 since D-1571), whatever the source version. Change docs/02:2143 to "bar geometry (v2 and v3 share it)".

### P14-06 low: docs/01 §3 marks the Step-3 successors "(open)", and docs/02 §23 calls Candidate Universe "audit only, no production writer"; both are reached from a production command

- Where:
  - docs/01-architecture.md:268-271, four rows each ending `(open)`, and :279-282 "it cannot yet construct production Statistics V2".
  - docs/02-store-format.md:1349-1355 "**audit only** ... there is no public production constructor or writer".
- Evidence:
  - candidate_universe.rs:2747 `pub(crate) fn produce_candidate_universe_v1` and :3844 `append_produced_candidate_universe_v1`, called at step3_orchestrator.rs:4033 inside `commit_candidate_family_guarded_v6`.
  - step3_orchestrator is used by ledger_v6.rs:64 and :309, which the CLI dispatches at lib.rs:1919 (`ledger_v6::ledger_v6(&request)`).
  - population_observations_v1, population_statistics_v2/v3, execution_v4 and global_replay_v4 are all called from step3_orchestrator and ledger_v6. docs/02 §20-§21, §25-§26 and §31-§34 describe them as written.
- Why it is wrong: the architecture's data-flow section says the authority chain stops at Pre-Admission. The store-format doc says the first file in that chain has no writer. The code writes the whole chain from an operator command.
- Fix: redraw the §3 Step-3 diagram through Population V6, Statistics V3, Execution V4, Selection V6 and Global Replay V4, naming `ledger_v6`. Replace §23's "audit only" paragraph with the production door (`produce_candidate_universe_v1` via `step3_orchestrator`).

---

## Verification of earlier rows

| Row | Status | Evidence |
|---|---|---|
| P1-16-01 census v3 | FIXED | docs/02:485-640 (v3, BRUTEXM2, contract 80..105, "writes version 3 only"). Bound by pull unit.rs:7525-7538. |
| P1-16-02 frontier v7 | FIXED | docs/02:782-835 matches frontier.rs:400-505. Bound by frontier.rs:2078. |
| P1-16-03 BRUTXEP1 640→648 at v1 | PARTIAL | Doc corrected and the breach recorded (docs/02:1007-1037, :1264-1271). A 640-stride file is now refused by name (execution_capability.rs:110-123, :2366-2369), and the comment at :1377 was fixed. The version was not re-minted: `FORMAT_VERSION: u32 = 1` (:62), so magic and version still name two geometries. |
| P1-16-04 undocumented formats | FIXED | docs/02 §27-§34 (:1600-2119), each bound by a `the_store_format_doc_states_*` test. |
| P1-16-05 mmap in data flow | FIXED | docs/01:211 and :294-298. |
| P1-16-06 "release store" | FIXED | docs/01:395. |
| P1-16-07 "compiled twice" WASM | FIXED | docs/01 §2 (:184-193). |
| P1-16-08 "no catalogue", vendor segment | FIXED | docs/01 §4 (catalog::walk exception) and :209 `bars/<vendor>/...`. |
| P1-14-02 graph.rs invented/duplicate rows | FIXED | graph.rs:472-498, :617-662. |
