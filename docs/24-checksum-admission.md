# Historical checksum admission V1

This opt-in path binds a stored monthly sweep to the exact source files whose
complete bytes and checksum blocks were verified. It reuses the ordinary
stored-month AND kernel. It does not certify vendor truth, a complete calendar,
institutional eligibility or future profitability. Existing stored formats,
legacy run identities and `ReferenceIntegrity` fields are unchanged.

The implementation is in `store::checksum_audit`,
`cli::checksum_receipts`, `cli::audited_stored`, and
`cli::stored_month_kernel`. The decision and invariant entries in the shared
documents govern this additive format; the layouts below describe the current
source exactly.

## Operator commands and scope

```text
cli checksum-audit-stored VENDOR UNDERLYING RUNG YEAR MONTH RECEIPT_ROOT MAX_BYTES
cli sweep-audited-stored VENDOR UNDERLYING RUNG YEAR MONTH MIN_HITS RECEIPT_ROOT MAX_BYTES MAX_RECORDS
cli audit-audited-range VENDOR UNDERLYING RUNG FROM_YEAR FROM_MONTH TO_YEAR TO_MONTH MIN_HITS RECEIPT_ROOT MAX_BYTES MAX_RECORDS
```

These read the existing configured `BRUTEX_STORE` source root. `RECEIPT_ROOT`
must already be a nonsymlink directory. Neither command fetches data or repairs
source bars. The checksum command writes receipts and inspection lifecycle
evidence; it does not sweep. The sweep command requires a verified clean build
stamp before loading inputs, as the ordinary stored command does.

The instrument is resolved through the existing swept-instrument authority:
the named NSE indices and permitted F&O cash equities. No new market surface
is introduced. Cash research retains its separate cost and validation limits.
The eight swept rungs are `1min`, `2min`, `3min`, `5min`, `10min`, `15min`,
`30min` and `60min`. Every coarse rung uses actual stored `1min` execution.
Daily files supply causal reference context; `1day` is not an independently
swept intraday rung. The inspection command can inspect a stored rung without
thereby authorizing it for sweeping.

`MAX_BYTES` bounds the complete data file, including its header region, plus
the complete checksum sidecar **for each source month**. `MAX_RECORDS` bounds
the summed raw committed records of the distinct input files, before their
decoded vectors are allocated. Both sweep ceilings must be positive. These
are physical admission limits, not support, strategy depth or profit settings.

The monthly wrapper requires current signal and minute data, previous-month
minute data, and previous/current-month daily data. Native `1min` reuses one
physical month for signal, execution and current minute reference: four held
month readers. A coarse rung has five. The source-record limit counts those
physical readers once. Copies, converted references, indicator state,
frontiers and output add memory beyond the raw-record count.

The strict range command also prices through the existing `audit_bars` kernel.
It requires every requested signal month, minute execution/reference month,
daily month and the preceding minute/daily context month. Shared files are
audited and charged once across the span. The existing 1,200-month physical
range limit is applied before allocating retained source guards; it is not
strategy depth. Native 1-minute signals reuse exact minute execution; coarse
signals use the same observed 1-minute execution column as ordinary pricing.
The strict path refuses an unsourceable overlay instead of withholding days
and silently shortening the admitted span. All source/receipt authorities
survive moving decoded bars and remain required at terminal publication.

### Additive chronological span receipt

Each chronological signal month adds one 512-byte file under
`RECEIPT_ROOT/audited-spans-v1/<identity>.bin`. Existing monthly `BRHIN001`
bindings and checksum `BRHCRC01` receipts are unchanged.

| Byte offset | Width | Meaning |
|---:|---:|---|
|0|8|`BRHSP001` magic|
|8|8|Little-endian version 1|
|16|32|Previous span-node identity; all zero only for ordinal zero|
|48|8|Zero-based chronological ordinal|
|56|8|Civil year|
|64|8|Civil month|
|72|240|Six ordered (8-byte role number, 32-byte receipt identity) records|
|312|168|Reserved zero bytes|
|480|32|Completion seal|

Role order is the existing six-role monthly order. Node identity hashes bytes
0..480 under `brutex-historical-span-input-binding-v1\0`; the completion seal
uses `brutex-historical-span-input-completion-v1\0`. Publication reuses the
existing exact-prefix append/sync protocol and lifetime shared receipt locks.
All predecessor nodes remain held. DATA_DIGEST binds the ordinary executed
digest and final span identity under `brutex-stored-checksum-admitted-span-v1\0`.
Changing role order, source bytes, range predecessor or span order changes that
authority. A month missing its source or receipt cannot become a shorter run.

The command records a clean-commit preparation attempt before the indicator
fold, then invokes the shared audit lifecycle and actual parent writer. The
strict guard runs before computation and immediately before authenticated
terminal sealing. Successful no-survivor completion is distinct from cold
indicator refusal. Generated actual-kernel tests cover both native/coarse
publication and retry, plus source replacement, span-node damage and missing
acknowledged ranking at the real terminal boundary. These tests are named
`actual_strict_range_kernel_publishes_and_reuses_native_and_coarse_evidence`
and `actual_strict_range_terminal_refuses_changed_source_binding_or_acknowledged_rank`.

## From cold audit to retained authority

`BarFile::open_existing_audited` opens the existing month lock, bar file and
checksum sidecar with nonblocking, no-follow final-component opens. It retains
the original shared month lock. The ordinary writer therefore cannot acquire
its exclusive month lock while the audited reader exists. A busy source is an
explicit refusal; the reader does not wait for the writer to finish.

The cold audit verifies the committed header, exact physical file lengths,
required checksum flag, supported layout and every committed checksum block.
The current format has a 32,768-byte header region and 56-byte bar records.
The audit hashes the complete header region, committed data region, checksum
sidecar and full data file independently. Its fixed block scratch buffer is
8,192 bytes. Header storage is a fallibly allocated fixed 32,768-byte buffer.
Unsupported, unsealed, truncated, extended or over-limit input refuses.

`AuditedBarFile` owns the actual `BarFile`; its authority cannot be constructed
from detached `Evidence` bytes. `AdmittedMonth` additionally owns the exact
receipt handle and expected 512-byte image. The receipt's shared lock is held
for that authority's lifetime, including final sweep publication. The six-role
binding uses the same retained receipt reader. Exact existing receipts are
reused through the read door, so another identical reader does not require an
exclusive publication lock.

A warm `read_record(index)` reads and verifies the particular format block,
then decodes the requested row from that same byte buffer. It does not trust
the ordinary reader's remembered last-block result. Source and receipt
generations are checked around the read. Generation checks bind named paths
to the held handles using device, inode, length, modification/change time and
link count; aliases, replacements, deletions and mutations refuse. The warm
receipt check compares all 512 bytes with the exact expected image.

The loader feeds these verified rows into the same `stored::decode_loaded`
calendar filter as the ordinary reader. The daily and minute spans use the
same existing causal context converters. It does not implement a second
calendar or indicator fold.

## Exact bytes and identities

All ranges below are half-open byte offsets. Numeric fields are little-endian
`u64` words unless stated otherwise. Signed timestamps preserve their exact
two's-complement bits. Unused bytes are zero.

### Detached checksum evidence: 256 bytes

| Offsets | Content |
|---|---|
| `0..8` | Magic `BRCAS001` |
| `8..112` | Thirteen words, in order: format version, record stride, header flags, generation, committed record count, first timestamp, last timestamp, symbol ID, timeframe seconds, header-region bytes, complete data-file bytes, sidecar bytes, block count |
| `112..144` | BLAKE3 of complete header region |
| `144..176` | BLAKE3 of all committed raw record bytes |
| `176..208` | BLAKE3 of exact entire checksum sidecar |
| `208..240` | BLAKE3 of exact entire data file |
| `240..256` | Reserved zero bytes |

This is a projection of measured facts. It neither holds files nor mints an
audited reader. The `data-file bytes` word includes the header; the separate
data digest covers the record region only.

### Source receipt: 512 bytes

Path: `RECEIPT_ROOT/checksum-receipts-v1/<receipt-identity>.bin`.

| Offsets | Content |
|---|---|
| `0..8` | Magic `BRHCRC01` |
| `8..16` | Version word `1` |
| `16..48` | Canonical source identity |
| `48..80` | Receipt identity |
| `80..336` | The exact 256-byte checksum evidence |
| `336..480` | Reserved zero bytes |
| `480..512` | Completion seal |

The source identity is BLAKE3 of the byte domain
`brutex-historical-checksum-source-v1\0` followed by the canonical relative
`StorePath` string. It binds vendor, instrument, timeframe, month and bar-file
kind without binding a host's absolute store root.

The receipt identity is BLAKE3 of
`brutex-historical-checksum-receipt-v1\0`, then bytes `0..48`, then bytes
`80..480`. The completion seal is BLAKE3 of
`brutex-historical-checksum-completion-v1\0` followed by bytes `0..480`.
Here and below `\0` denotes one zero byte, not two printable characters.
Source path and exact audited header/data/checksum bytes therefore jointly
determine receipt identity. This is a checksum seal, not a secret signature.

### Six-role input binding: 512 bytes

Path: `RECEIPT_ROOT/audited-inputs-v1/<binding-identity>.bin`.

| Offsets | Content |
|---|---|
| `0..8` | Magic `BRHIN001` |
| `8..16` | Version word `1` |
| `16..256` | Six ordered 40-byte entries: role number as `u64`, then its 32-byte source receipt identity |
| `256..480` | Reserved zero bytes |
| `480..512` | Completion seal |

Roles `0..5` are signal, execution, previous daily, current daily, previous
exact minute and current exact minute. Native `1min` repeats the same receipt
in roles 0, 1 and 5; coarse execution and current minute repeat in roles 1 and
5. This repetition records actual roles rather than inventing separate inputs.

Binding identity is BLAKE3 of
`brutex-historical-six-role-input-binding-v1\0` followed by bytes `0..480`.
Its completion seal uses
`brutex-historical-six-role-input-completion-v1\0` over the same bytes.
The identity is in the filename; there is no additional identity field in
this fixed record.

The common stored input digest still binds the actual signal, daily reference,
exact minute context and execution slices. The opt-in wrapper strengthens its
data-digest term with BLAKE3 of
`brutex-stored-checksum-admitted-inputs-v1\0`, the ordinary executed-input
digest, and each ordered `(role as one byte, receipt identity)` pair. The
other eight run-identity terms retain their existing meanings. Ordinary
`sweep-stored` passes no audited guard and retains its ordinary digest.

## Append-only publication and terminal meaning

An audit records a `ChecksumAudit` lifecycle start before its full checksum
scan. Its plan identity binds canonical source, byte ceiling, admission mode
and an expected receipt identity when one was explicitly supplied. That plan
names the request; the actual content receipt is minted only after the full
byte audit. Operation byte `10`, API label `checksum-audit`, means inspection,
not search or institutional admission. Structural refusal events name the
source identity, plan identity, attempt and escaped reason.

A receipt publisher acquires an exclusive lock. An incomplete file must be
an exact prefix of the expected final bytes. The publisher appends only the
missing payload, synchronizes it, appends the missing completion seal,
synchronizes again, and synchronizes the parent directory. A conflicting
prefix or overlong file refuses without overwrite or truncation. An exact
512-byte receipt is checked through the shared reader rather than rewritten.
The six-role binding uses the same publication protocol.

`audit_month` can publish or recover such an exact receipt.
`admit_month(request, expected_identity)` performs a fresh complete audit and
requires the exact prior receipt: it never creates or repairs that receipt.
Strict admission can still write its own lifecycle evidence; it is not a
read-only filesystem inspection. An absent required namespace/root or invalid
canonical path refuses at preflight, before that attempt is created. Strict
admission never creates the receipt namespace.

The strict monthly wrapper retains all source, receipt and binding authorities
while it enters `stored_month_kernel`. That kernel checks them, computes the
full run identity, records its sweep attempt, and only then folds indicators.
The same retained AND checkpoint/search and forward ranking path is used by
ordinary and audited stored sweeps. This command is an unpriced ranked sweep;
it does not claim a selected exit-grid trade or institutional admission.

Its final shared `finish_stored_month` boundary does these steps in order:

1. Check the admitted input snapshot.
2. Verify acknowledged depth/ranked counts and byte digests, then durably seal
   the attempt's actual terminal state.
3. Invoke the legacy parent-summary publisher.

Lost or corrupted acknowledged children cannot reach step 3. If step 3
refuses, the completed computation evidence remains, the command returns the
publication error, and no successful parent publication is claimed. A
completed attempt describes that completed computation snapshot, not perpetual
availability of the input files or proof of a later summary append. Resource
halts and refused samples retain their distinct terminal classifications.

The original priced `audit_bars` path uses the same finalization helper through
`AuditEvidence::seal` on all four publication exits: halted/refused search,
no retained candidate, priced candidates with no admitted selection, and an
admitted selection. The two priced exits recheck the exact candidate capture
before sealing depth/ranked evidence and calling their legacy publisher.
An attempt is taken when finalization begins; an early return that retains
its attempt is explicitly completed as Refused. A later parent-write refusal
cannot change an already completed computation into a second terminal event.
No completion is inferred from a result banner, elapsed time or live-file
cleanup. These are final authenticated snapshots; readers still verify saved
artifacts and can refuse later external corruption.

## Measured tests and remaining limits

The following focused checks were confirmed from their logs on 6 September
2026. They use generated temporary storage fixtures, not assertions of market
performance. New Cargo work used two build workers and two test threads.

| Area | Confirmed checks | Evidence log under `target/sweep-audit-20260906/` |
|---|---|---|
| Store audit | 7 passed: exact header/data/sidecar/full-file digests across empty, partial and full block boundaries; every record-byte and CRC-byte corruption in a small month; strict extents; held lock; source FIFO refusal; mutation, replacement and alias refusal | `checksum-store-final-tests.log` |
| Receipts | 7 passed: exact identity/reopen; every prefix length 0 through 512; conflicting bytes/extension; strict prior-receipt admission; changed source; warm receipt corruption/loss/alias refusal; lifetime shared lock and exclusive publication contention; nonblocking FIFO opens | `checksum-cli-frozen-tests.log` |
| Monthly loader | 3 passed: native/coarse rows and context equality; all five source replacements and binding corruption; exact and insufficient ceilings; missing prior context | `audited-stored-frozen-tests.log` |
| Shared publication | 4 passed: deleted/corrupted acknowledged depth or ranking cannot reach the actual parent writer; input guard refusal; sealed rows visible before parent callback; explicit failed parent callback after completed computation | `stored-month-publication-tests.log` |
| Original priced-audit finalization | 4 passed: actual warmed audit with late depth/ranking deletion or corruption; real successful empty-ranking publication control; one-shot fault cleanup; exact two-side priced capture control and lost catalog, tier, candidate or trade evidence | `audit-publication-tests-v2.log` |

The publication proofs are named
`lost_or_corrupt_acknowledged_children_never_reach_the_real_parent_writer`,
`strict_guard_refusal_precedes_terminal_and_parent_publication`,
`genuine_terminal_and_all_acknowledged_rows_exist_before_parent_publication`,
and `parent_write_refusal_keeps_completed_computation_and_returns_the_exact_failure`.
The monthly equality proof is
`native_and_coarse_use_exact_audited_rows_and_the_same_calendar_converters`.
The actual audit fault proof is
`actual_audit_late_depth_or_ranking_loss_cannot_publish_a_parent`; its positive
control is `actual_audit_publishes_only_after_the_real_empty_ranking_is_sealed`.
`audit_finalizer_rechecks_exact_priced_catalog_and_children_before_terminal`
uses real grid/materialization output for both sides.
`test_fault_guard_cannot_leak_an_unconsumed_callback` checks the test-only
thread-local fault injection cleanup. The ordinary production build contains
no fault callback.
The coordinator's final verifier records later source-wide gates separately;
these focused results alone are not full coverage or mutation closure.

Cold audit and full-month loading cost O(source bytes/records). A warm row
check has fixed-block work and fixed receipt work. A complete combinatorial
search, all output history, terminal detail verification and filesystem
latency do not become O(1). Audit source-byte ceilings do not bound cumulative
receipt/lifecycle storage, total process memory or total search time.

No-follow/nonblocking flags are enabled only on the verified macOS and Linux
x86_64/aarch64 configurations; other hosts refuse. The named final component
cannot silently become a symlink or wait for a FIFO peer. Intermediate
directories remain part of the trusted local root, and OS metadata/device I/O
can still stall. Advisory locks constrain cooperating readers and writers;
this is not authentication against a malicious OS or an actor able to forge
metadata or bypass that cooperation contract.

Crashes can leave a Running attempt or an incomplete exact receipt prefix.
Neither age nor file visibility invents a completion. Filesystem corruption,
full-disk and interrupted-publication cases are handled by explicit errors,
generation/byte checks and append-only recovery where defined; they are not
an unconditional hardware durability guarantee. The clean-build, real-data
launch, complete coverage and mutation gates must be reported from their own
actual evidence.

## Explicit strict range API — D-0527

The CLI command and the explicit `audit-audited-range` command accepted by
`POST /engine/command` share one checked adapter. The HTTP request supplies
feed, eligible underlying, one intraday rung, inclusive month bounds and
positive `min_hits`. It cannot choose store/receipt paths or override physical
checksum limits. Existing applicable pricing controls retain their actual
runtime meaning; incompatible support controls and unusable explicit values
must refuse before a run is admitted.

The strict path shares scalar parsing and physical bounds with the actual CLI
readers. In particular, a value fitting a machine integer does not imply it
fits the horizon's integer type, the result-count type or the existing screen
and grid capacities. API admission validates the effective request/environment
settings; direct CLI execution validates again before opening source admission.
Resolution and the final publication guard also reject recorded knob refusal.
An invalid value cannot produce an apparently successful fallback-policy run.

The server resolves these three physical settings once before claiming the
existing exclusive sweep slot:

| Setting | Meaning |
|---|---|
| `BRUTEX_CHECKSUM_RECEIPTS` | Existing absolute receipt directory; the final component cannot be a symlink |
| `BRUTEX_CHECKSUM_MAX_BYTES` | Positive maximum full-source bytes audited per physical file |
| `BRUTEX_CHECKSUM_MAX_RECORDS` | Positive maximum summed unique raw records across the requested span |

These fields are I/O/allocation policy, not the 37 financial admission choices.
Missing or unusable configuration returns HTTP 503 with
`strict_input_configuration_missing_or_invalid`, `started:false`, and explicit
missing/invalid field names. It does not create a run attempt, take a slot or
fall back to an ordinary range audit. The warning logs field names, never the
configured values or paths. A valid configuration is not evidence that the
source files will pass their later locked, byte-level checks.

Accepted work retains the browser's exact attempt through the shared pricing
transaction. A failed post-fold source check explicitly seals preparation as
refused; missing result publication remains a command failure. Existing worker
cleanup handles abnormal exit, but a running blocking kernel does not acquire
an immediate cancellation mechanism from this adapter.

The strict worker checks the task-completion event's append outcome. A filtered,
dropped or unavailable terminal event is reported visibly as incomplete task
audit, retaining the original report/refusal text. Already-written market-result
evidence remains intact. A failed physical write cannot guarantee a durable
terminal marker; this behavior reports that limit instead of hiding it.

Receipt-checked browser batches now submit this exact command for each selected
instrument and intraday rung. The202 acceptance returns its reserved attempt as
decimal text; status carries the matching `attempt_key` and `where:browser`.
This avoids JavaScript rounding and prevents a later unrelated command from
completing the wrong queue item. One job runs at a time; unknown status or refusal
stops future submissions. The queue belongs to the page, while each accepted
server job and its durable evidence have their own lifetime. It is not a
durable aggregate server queue or an automatic reload-resume authority.

The institutional V6 route uses its own retained source capabilities around the
same strict range loader, after complete admission-policy resolution. It retains
independent raw signal/minute/daily record ceilings and all physical/role
receipts. Actual receipt digests bind the family-specific computation identity;
shared cohort-policy digests identify the integrity mode without hashing one
instrument's data into a policy that both indices must share. Naturally empty
families retain their source and ledger authorities through final selection;
later OOS inputs retain new guards through replay publication.

Neither integration configures or replaces the active backend, supplies the37
institutional thresholds, admits Boolean programs into V6, or extends the
institutional surface to cash stocks.
