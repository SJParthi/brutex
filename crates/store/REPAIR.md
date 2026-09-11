# Explicit historical bar revisions — facility V1

This is an additive store-only seam. It does not activate repaired data in
production. The ordinary `StorePath`, `BarFile::open_existing`, catalog, and
append path continue to address the original month. No original `.bin`, `.crc`,
overlay, lock, or directory is changed by this facility. No rename is used.

## API and caller contract

1. Open the original using `BarFile::open_existing`. Keep its `Header` as the
   expected source snapshot while preparing the complete merged `Vec<Bar>`.
2. Perform calendar/month/timeframe, vendor provenance, and correction-policy
   checks in the caller. The store does not know exchange sessions. Calendar
   completeness is not established by retaining the existing timestamps.
3. Call `repair::publish(root, path, symbol_id, Revision::new(n)?, header, &rows)`.
   Only `FileKind::Bars` is accepted. Prices, counts, OI nulls, and ordering use
   the existing append validator. Rows must be strictly chronological, nonempty,
   and contain every original timestamp. Values at those timestamps may change;
   insertions before, between, and after existing rows are allowed.
4. Explicit consumers use `RevisionReader::open(root, path, symbol_id, revision)`.
   The reader pins the revision's bar and CRC descriptors and shared month lock.
   It exposes headers, indexed reads, and `first_at_or_after`, with no append or
   mutable `BarFile` access. Downstream computation must derive its data digest
   from the actual selected rows, as for an ordinary stored run.

The original must already have a lock sibling and block checksums. A shared
source lock is acquired before validation and retained until publication ends;
an active original writer refuses the operation. The full source scan verifies
CRCs, row validity, strict chronology, and timestamp retention. Source generation
changes since the caller's snapshot refuse before any revision reservation.
This is a repair of valid-but-incomplete or corrected bar values, not a recovery
tool for CRC damage, corrupt headers, or structurally invalid original rows.

Lock order is original shared lock, then a distinct revision writer lock. All
acquisitions are nonblocking. This additive protocol needs an explicit exception
in the shared format document to its old statement that month locks are always
leaves; ordinary append retains its existing single-lock behavior. The public
revision reader takes only revision locks. No revision lock holder waits for an
original lock, and contention is returned to the caller without a wait loop.

## Paths and bytes

For revision ordinal `n`, the alternate root is
`<root>/bar-revisions-v1/<n>/`. Under that root the unmodified `StorePath`
renders `bars/<vendor>/<exchange>/<segment>/<symbol>/.../<yyyy-mm>.bin` and
the `.crc` / `.lock` siblings. Thus the original catalog rooted at
`<root>/bars` never discovers revisions as additional instrument-months.
The same ordinal can be used independently for different logical months.

Each revised month also has two siblings:

| Suffix | Contract |
| --- | --- |
| `.reserved-v1` | Empty, create-once reservation. Its existence prevents every subsequent publisher from reopening the pair for writing. It is never removed by this API. |
| `.repair-v1` | Exactly 144 bytes, completion written last. No valid completion means the revision cannot be served. |

Completion bytes:

| Offset | Bytes | Meaning |
| --- | --- | --- |
| 0 | 8 | `BRXREPV1` magic |
| 8 | 8 | Explicit revision ordinal, unsigned little endian |
| 16 | 64 | Original expected header, encoded with its existing slot CRC |
| 80 | 64 | Revised committed header, encoded with its existing slot CRC |

Both headers retain their existing format/version and CRC contracts. The new
file's bar format remains V2; revision ordinal is not a bar format version.
On open the receipt's magic, ordinal, exact length, header CRCs, symbol,
timeframe, checksum flags, bounds, and agreement with the opened revised
header are checked. Block CRCs are checked lazily by the existing read path.
The source header records the original generation; it is not a cryptographic
source-data digest or an independently authenticated provenance receipt.

## Publication and failures

All input and source validation precedes reservation. A create-new reservation
serializes publishers for exactly one ordinal/month. Unexplained existing target
bars, CRC, lock, or receipt cause refusal and are preserved. A fresh pair is
written through `BarFile` in one complete append, which syncs records, CRCs,
and the committed header using the existing protocol. Every written row is
read back and compared, and all new directory entries through the supplied root
are synced. Only then is the completion created, written, synced, and its
directory hierarchy synced. The revised writer lock is held through this order.

An old reader keeps its original pair throughout. A reader racing publication
either refuses a missing/torn receipt or held writer lock, or opens the complete
revision. It cannot observe an old `.bin` paired with a new `.crc`. Readers may
coexist across versions and within a version. The original may continue to
append after publication; that does not change the published snapshot.

An exact completed retry compares the source header and every offered revised
row, then re-syncs the revised files, completion, and directories. It returns
`Published::Reused`, including when the original has subsequently advanced.
Different rows or a different source header at that ordinal refuse. A missing
or torn completion is never resumed, truncated, overwritten, or silently
treated as the original; it remains available for inspection. After review a
caller can choose another ordinal. This deliberately trades recovery convenience
for avoiding ambiguous partial-write reuse.

Errors after receipt writing starts carry
`publication_may_be_visible: true`. The data may already be readable, while
crash durability remains unestablished. Report uncertainty and retry the exact
request to re-establish durability; do not report rollback. True power-loss,
disk-full, and filesystem write-ordering behavior requires external fault
testing; logical interrupted states are constructed by the tests below.

These guarantees assume an existing local store root, cooperating writers,
and stable directory/lock inodes. As with `BarFile`, ordinary filesystem opens
do not defend against hostile symlink swaps, direct writes, removal of permanent
reservations, or network filesystems with ineffective locks. These revisions
are immutable through this API, not OS-enforced immutable files. Opening the
alternate root with the general `BarFile` writer is outside this protocol.

## Bounds and cost

Source and revised rows are each capped at 100,000. Ordinals are 1..=1,024 per
logical month; no free-ordinal search or implicit latest lookup is performed.
Each publication or completed retry is O(source + revised rows), with a fixed
number of directory operations and bounded buffers besides caller-owned rows.
The source is streamed and not copied into a second vector. Publication is not
claimed to be O(1). Ordinary indexed reads and block checks retain their
existing cost. The ordinal cap bounds namespace growth for a month, not the
number of instruments/months in the store or the caller's allocation before
entering this API. No throughput or durability latency measurement is claimed.

## Invariant proofs and shared-document handoff

`tests/repair.rs` names these proofs:

| Invariant | Test |
| --- | --- |
| Original bar/CRC bytes and open readers survive insertions and corrections | `complete_merge_preserves_original_bytes_and_concurrent_readers` |
| Completed retries are exact, idempotent, and independent of subsequent original appends | `exact_retry_preserves_every_revision_byte_and_rejects_conflicts` |
| Bad input, missing original timestamps, and stale generations write no revision | `invalid_batches_stale_sources_and_deletions_leave_no_revision_tree` |
| Row/ordinal bounds and bar-only scope | `exact_resource_boundaries_and_bar_only_contract` |
| Corrupt or missing original evidence is refused and never created | `corrupt_or_missing_source_evidence_is_never_repaired_or_created` |
| Original writers exclude publication; unsealed inputs refuse | `source_writer_excludes_publication_and_unsealed_sources_refuse` |
| Missing/torn completion refuses and preserves partial evidence | `incomplete_and_torn_publications_are_preserved_and_never_served` |
| Unexplained revision files are preserved | `unexplained_revision_files_are_not_overwritten` |
| Receipt/header damage and missing revised CRC cannot silently serve | `receipt_corruption_changed_headers_and_missing_crc_refuse` |
| Competing publishers cannot mix a pair | `competing_publishers_cannot_mix_rows_or_checksums` |

The main agent must allocate an append-only decision entry and invariant IDs
in the shared documents; this work explicitly does not edit those files. The
decision to record is: bounded explicit V1 revisions, unchanged bar V2 geometry,
source-generation check and timestamp retention, create-once reservation,
completion last, strict exact retries, preservation of unfinished evidence,
and no transparent promotion. Rejected: replacing the live `.bin` and `.crc`
with separate renames, changing append to insert/overwrite, and selecting the
highest discovered ordinal without a committed authority.

## Exact blocker for production promotion

Transparent repaired-month selection is not safe as a store-path substitution
alone. Production integration must provide a single versioned authority that
pins one complete bar/CRC pair, acquired under the stable original month lock
before resolving either file. Existing writer admission must participate: it
must either refuse an original-generation append after promotion or explicitly
create a successor revision; it must not continue writing an invisible base.
Existing independently indexed overlays/greeks, derived manifests, coverage
counts, cached readers, and run data identities must be invalidated or bound to
that same revision. An inserted historical row shifts every later overlay index.

The main agent owns that calendar and API/pull coordination. This patch supplies
explicit read/write revision seams only. It provides no current pointer,
latest-revision resolver, automatic repair producer, revision-to-revision merge
ancestry, or live-store migration. Nothing in it claims a calendar gap is now
filled in the production series.

## Verification at handoff

- `cargo test -p store --locked`: passed, including all 10 new repair tests
  and the existing store unit/integration/doc tests.
- `cargo fmt --check -p store`: passed.
- `cargo clippy -p store --all-targets --locked -- -D warnings`: passed.
- `cargo deny check`: passed; duplicate-dependency warnings were reported.
- `git diff --check -- crates/store`: passed.
- `cargo llvm-cov -p store --locked --summary-only`: reported 93.83% store
  line coverage and 85.90% for `repair.rs`, with a warning that 121 functions
  have mismatched profiling data. An isolated target-directory run repeated
  that warning. These are not clean coverage-gate evidence. Branch coverage
  was not collected, and mutation testing was not run.

The full repository definition of done is therefore not established: the
100% line/branch and no-surviving-mutant gates remain open, as do the shared
decision/invariant entries and broader workspace checks owned by the main
agent. Tests touched only temporary synthetic stores. No live data, service,
vendor endpoint, or shared document was accessed for mutation.
