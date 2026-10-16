# Explicit historical bar revisions — facility V1

This is an additive store-only seam. On its own a published revision is
inert: the ordinary `StorePath`, catalog and append path address the original
month. No original `.bin`, `.crc`, overlay, lock, or directory is changed by
this facility. No rename is used.

**Served once promoted (D-3350, WP-13c).** `crate::generation` is the one
authority that says which generation of a month every reader opens: a revision
becomes the served generation when `generation::promote` appends one
CRC-sealed record to the month's pointer under
`<root>/bar-generations-v1/…/<yyyy-mm>.gen-v1`. From then on
`BarFile::open_existing`, `open_existing_audited`, `index_existing` and
`provenance::read` resolve the revision, and `BarFile::open_or_create` refuses
the original (`StoreError::Superseded`). `repair::repair_month` is the whole
repair: the served month plus the offered bars, a held bar never dropped or
changed, published at the next free ordinal and promoted. See "Production
promotion" below for how each blocker this document named is met.

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

Each header keeps the format/version and CRC contract of the file it came
from. The revised file is created through `BarFile::open_or_create`, so it is
born at `store::layout::Layout::CURRENT` — bar **version 3** since D-1571, with
mandatory block checksums — whatever the source month's version. A revision of
a version-2 month therefore embeds a version-2 original header and a version-3
revised header; the two versions share one bar geometry (56-byte stride,
73-record blocks, the same slot), so the records are byte-compatible. The
revision ordinal is not a bar format version. (This said "The new file's bar
format remains V2", which stopped being true at D-1571;
tests-docs-security-pass14 P14-05, D-1959.)
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

Source, offered and revised rows are each capped at `row_ceiling(path)`, the
most bars the path's month can hold on the path's rung: the month's length
over the rung's width (D-4875 (1), D-4898, IA-17). That is 2,678,400 for a
31-day one-second month, 44,640 for its one-minute month and 31 for its daily
one, and 28 days of the rung for a February. A count past it is refused
`RowLimit { rows, ceiling }` before a bar is read. `MAX_ROWS`, 2,678,400, is
the largest value the ceiling takes and is compared against nowhere. The
ceiling was a flat 100,000, under which no one-second month of a full session
calendar fits (a GDFL NIFTY one-second month holds about 409,000 bars), so no
GDFL month could be repaired (DB-N26, WP-12d), and then a flat 2,678,400 on
every rung. What one repair holds in memory is `docs/06-limits.md` L-IA-26;
bench C-RP-01 times it at a full month. Ordinals are 1..=1,024 per
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

### Private directory-creation refusal — 27 September 2026 proof follow-up

`repair::tests::a_post_preflight_directory_obstruction_refuses_without_publication`
first performs the existing source/batch preflight with real shared source
locks, then creates a regular-file obstruction in the revision directory tree.
The unchanged private `write_revision` body returns `NotADirectory` naming
`create revision directories`, the exact requested directory, and
`publication_may_be_visible: false`. Source/CRC/lock and obstruction bytes
remain unchanged; revision data, CRC, lock, reservation and receipt paths each
return the expected obstruction error.

The source remains locked until the test releases its explicit lock owner.
Removing only the injected obstruction then permits public `publish` to return
`Created`, and the reader returns all four merged rows exactly while preserving
the original source bytes. This is a real filesystem failure at the private
write boundary, followed by a public success control. It does not exercise
public `publish`'s failure/release path, a scheduled race, a receipt/sync fault
or power-loss durability. No production hook or publication policy changed.

The main agent must allocate an append-only decision entry and invariant IDs
in the shared documents; this work explicitly does not edit those files. The
decision to record is: bounded explicit V1 revisions, unchanged bar V2 geometry,
source-generation check and timestamp retention, create-once reservation,
completion last, strict exact retries, preservation of unfinished evidence,
and no transparent promotion. Rejected: replacing the live `.bin` and `.crc`
with separate renames, changing append to insert/overwrite, and selecting the
highest discovered ordinal without a committed authority.

## Production promotion (D-3350, WP-13c)

The blockers the next section named, and how `crate::generation` meets each:

- **One versioned authority.** The month's pointer file is append-only; its
  last record that verifies names the revision served, with that revision's
  committed header (`Pointer::describes`). A reader that resolves the pointer
  opens one complete revision pair, whose files never change after
  promotion; a revision whose header no longer matches its record is refused
  (`StoreError::Generation`), never served. A torn last record is stepped over
  with a `store.generation` warning; a pointer of which no record verifies is
  refused, never read as the original.
- **Writer admission.** `BarFile::open_or_create` asks the pointer before it
  creates anything and again under the month's exclusive lock, and refuses a
  superseded month. `generation::promote` holds the original month's lock
  shared from its source check until its record is written, so a writer
  holding the month refuses the promotion (`StoreError::Locked`) and a writer
  arriving afterwards finds the pointer under its own lock. Both locks are
  non-blocking, so holding two cannot deadlock.
- **Indexed siblings.** `repair_month` refuses a month with an overlay or
  greeks sibling (`RepairError::IndexedSibling`): an insertion would shift
  every row it indexes. The `.tix` is the revision's own, written when it was
  published and rebuilt in place by `index_existing`.
- **A caller's own sibling of the month.** `repair_month_with` takes one
  step, `before_promote`, and calls it with the revision's root after
  `publish` has returned (`Created` or `Reused`) and before
  `generation::promote`: the revision is whole and durable and no reader is
  served from it, so a file the step writes beside the revision's bars is
  first seen by readers at the promotion, with the bars. `store` names no type
  of the caller's. A step that refuses stops the repair unpromoted
  (`RepairError::BeforePromote`, the step's words kept); the revision stays
  published and unserved, and the same repair offered again reuses it, runs
  the step again and promotes, so the step must be repeatable. `repair_month`
  is the same call with no step, byte for byte. D-4875 (5), D-4898; tests
  `store/tests/month_repair.rs`.
- **Caches.** A cache keyed by a month's file keys by
  `generation::served_bars_path`, so a promotion is a new key
  (`api::bars_series`). The census row is the served month's header, written
  by the landing that repaired it.
- **Provenance.** A revision carries no `.prv`: its records would index the
  original's bars. `/provenance.json` reads the revision's absence as
  unrecorded, and the repair journal `audit/<feed>.repairs` names the repair.

## Exact blocker for production promotion (superseded by the section above)

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
