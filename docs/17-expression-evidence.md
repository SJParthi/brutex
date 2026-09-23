# Explicit expression research and saved evidence V1

This route evaluates one named Boolean expression over the existing stored
signal column. It does not enumerate all Boolean functions, generate ordered
event permutations, price expression trades, or issue institutional admission.
The AND ladder keeps its existing extinction and pruning semantics. Applying
its anti-monotonic support proof to an OR expression would be incorrect.

## What a user can ask for

```text
cli expression-stored VENDOR UNDERLYING RUNG YEAR MONTH '0 & (63 | !369)'
```

The identifiers are the live numeric positions or their exact canonical names
from the Rust vocabulary. `!` means NOT, `&` means AND, `|` means OR; precedence
is NOT, AND, OR. Parentheses can change grouping. Retired, void, unallocated and
unknown positions refuse. VIX is not introduced into the vocabulary.

| Case | Evaluated answer |
|---|---|
| True AND unknown | Unknown |
| False AND unknown | False |
| True OR unknown | True |
| False OR unknown | Unknown |
| NOT unknown | Unknown |
| A definite true expression | Signal hit |
| A false expression | Definite miss |
| An unknown expression | Recorded unknown; never a hit |

`Column::known()` supplies actual availability evidence alongside truth. A
missing volume/reference/warm-up requirement must not become a false predicate
whose negation creates a signal. False availability is deliberately conservative
for several families; [the indicator inventory](15-indicator-readiness.md)
lists them. This means some otherwise decidable false answers remain unknown.
The route preserves the shared column's warm-up exclusions.

V1 accepts at most 4,096 source bytes, 1,151 postfix instructions and nesting
below 32 recursive operand levels. The program capacity holds 384 individually
negated leaves with binary joins; all 328 currently live positions fit together
using numeric identifiers. Long repeated names can still hit the separate
source-byte bound. These are explicit representation/resource refusals, not a
caller depth parameter on the AND sweep. A future serialized capacity change
requires a new format version.

## Identity and duplicate meaning

The ordinary nine tagged, length-framed identity terms are retained in their
original order. Expression runs extend **the params term** from its original
32 bytes to `32 parameter bytes || "BTXEXPR1" || complete descriptor`. They do
not truncate the expression to the legacy policy's `u64` field. Legacy AND
identities are unchanged.

Whitespace, redundant parentheses, numeric/name aliases and commutative
siblings share a canonical descriptor. General Boolean equivalence is not
claimed: associativity, distributivity and simplification are not a universal
constant-time duplicate detector. The full descriptor distinguishes operators
and negation even when referenced bit masks are identical. BLAKE3 identities
provide cryptographic collision resistance, not a mathematical assertion that
different unlimited inputs can never collide in 256 bits.

Input loading and digesting must precede identity construction. The durable
attempt start and descriptor are saved before the indicator fold or expression
evaluation. Verified clean build provenance is required by the existing stored
boundary. An unverified dirty build refuses; this command does not bypass it.

## Fixed-width descriptor

All integer fields are little endian. The descriptor is **3,457 bytes**.

| Offset | Width | Meaning |
|---:|---:|---|
| 0 | 2 | Expression semantics version, 1 |
| 2 | 2 | Active instruction count, 1 through 1,151 |
| 4 | 3,453 | 1,151 three-byte instruction slots |

Each slot contains an opcode byte and a two-byte operand. Opcode 1 is a live
bit identifier; 2 is NOT; 3 is AND; 4 is OR. Operators require operand zero.
Unused slots are all zero. Active padding, stack underflow, multiple leftover
values, noncanonical sibling order, wrong version and nonzero unused bytes
refuse during `Expression::decode`. A decoded expression can evaluate again
without a second-language runtime or the original source text.

## Historical signal file

Each execution attempt owns:

```text
STORE/expression-v1/FULL_ID/ATTEMPT/expression-v1.rows
```

The attempt sequence is allocated by the shared
[sweep evidence protocol](16-sweep-evidence.md). Separate attempts never
overwrite one another. The file is **3,497 + 56 × rows + 64 bytes**.

| Section | Width | Meaning |
|---|---:|---|
| Header magic | 8 | ASCII `BTXEXPR1` |
| Identity | 32 | Full expression run identity |
| Program | 3,457 | Exact canonical descriptor |
| Each row | 56 | Source index u64, timestamp i64, truth u8, seven zero bytes, 32-byte row seal |
| Final counts | 32 | Evaluated, hits, misses, unknown; four u64s |
| Seal | 32 | BLAKE3 of header, every row and final counts |

Truth bytes are 0=false, 1=true and 2=unknown. Source indices are strictly
increasing and refer to the original signal slice before warm-up exclusions.
The writer checks order and independently counts its accepted rows; its final
summary must equal those counts. No statistical precision is rounded.

Each row seal is BLAKE3 of `"BTXEXR01"`, the 32-byte header digest, the row's
zero-based ordinal as u64, and its 24 data bytes. This binds a row to its exact
identity, program and position. The final file seal includes these row seals.

New ancestor directory entries are individually synced. The writer first creates
a new `.pending` file, writes and syncs the header, and syncs its directory. Rows
stream into that file. Completion first checks the acknowledged pending length,
writes counts and seal, then flushes and syncs the file. It checks the held file's
identity against the pending pathname and rereads its exact acknowledged bytes.
After a non-replacing hard link, it verifies the published pathname and bytes
again before removing the still-matching pending name and syncing the leaf
directory. These terminal scans are O(rows) with a fixed 8 KiB buffer. The lifecycle terminal record is
written after publication. Failure preserves evidence and does not overwrite an
older completed file. A pending/orphan file is not a completion receipt.

`read` checks identity and a supplied program. `read_saved` recovers the program
and identity from the header, then uses the same strict reader. Both reject
symlinks, nonregular files, oversized or torn files, invalid rows, bad counts,
and seal mismatches. The strict scan completes before any row reaches the
visitor; a visitor failure returns an error immediately. The reader holds a
shared lock on the open file and reuses that handle, so pathname replacement
does not change the handle between validation and row visitation. Every row is
also rechecked against its own seal before its callback. An accidental writer
that ignores the advisory lock and changes a later row between passes is
detected before that changed row escapes; already delivered verified rows may
precede the returned error. These integrity hashes are not authentication
against a malicious actor who can replace data and recompute its seals, or a
proof against every physical storage failure.

## Executable evidence and bounds

The named tests are executable requirements. Their actual pass/fail state is
reported separately for the source snapshot that was tested.

| Requirement | Test surface |
|---|---|
| Independent three-valued semantics | `vocab/tests/expression.rs`: all 27 inputs against an independent ordered truth model |
| Every live bit and full signed vocabulary fit | Individual positions across all six words plus complete positive/negative AND/OR programs |
| Exact persisted program recovery | Codec round trips, maximum capacity boundaries and 1,554 short wire programs compared with independently reconstructed infix trees |
| Operators and every runtime identity term bind | `runner::expression::tests::expression_identity_binds_the_program_and_every_runtime_term` |
| Unknown, bad alignment/order and failed persistence stay explicit | Runner expression streaming tests |
| Exact rows and empty history reopen | CLI expression durable round trip and `read_saved` |
| No corrupt prefix leaks | Every byte position mutated and every shorter file length tested on a fixed fixture |
| Wrong identity, program, size, file type and consumer fail | CLI expression refusal tests |
| No overwrite or forged successful summary | Duplicate publication, pending collision, wrong counts and repeated-source tests |
| No changed row escapes the callback pass | Deliberate in-place modification of the second row from the first callback; only the first verified row is delivered |

Evaluation uses a fixed stack and no per-row heap allocation. Work per row is
bounded by the fixed program capacity; the full pass is O(B) for B retained
bars. Column storage is O(B), output storage O(B), and integrity verification
O(B). Parsing and canonicalization are bounded setup work and are not presented
as an unbounded O(1) parsing algorithm. Disk, locks and scheduling have no
constant wall-clock deadline. Full line/branch coverage, mutation completeness,
real-data execution and deployed dashboard behavior require their own evidence.
