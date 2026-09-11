# Resumable Boolean grammar search

`cli expression-search-stored VENDOR UNDERLYING RUNG YEAR MONTH BITS MIN_HITS CANDIDATES NODES`

The priced variant is
`cli expression-backtest-stored VENDOR UNDERLYING RUNG YEAR MONTH BITS MIN_HITS CANDIDATES NODES HORIZON GRID_RUNGS GRID_STEP_PPM`.
It uses the shared execution grid and records both directions' exact selected
cell and trades for every expression it actually evaluates. The explicit
positive grid controls and complete applied cell rules bind the PARAMS term.
The source signal column is causally projected onto the actual separately
loaded minute execution slice for a coarse rung. Those execution bytes bind
the data digest. Neither variant supplies institutional admission policy.

`BITS` is `all`, or sorted unique live condition IDs separated by commas.
The two final positive numbers are per-invocation work budgets. They pause the
same search; they neither restrict expression depth nor certify exhaustion.
Repeating the command with the same data and policy resumes saved progress.

| Question | Exact answer |
|---|---|
| What is enumerated? | Every structurally canonical postfix program in the fixed Expression V1 wire grammar over the selected alphabet, in instruction-count then lexicographic order. |
| Which operators? | AND, OR and NOT at every structurally valid position; mixed nesting, repeated conditions and nested NOT are included. |
| What ends a complete search? | Exhaustion of the existing 1,151-instruction wire language. No runtime depth parameter exists. This finite space can be astronomically large. |
| Does AND pruning apply? | No. Adding OR or NOT does not preserve the AND support-pruning argument. Invalid stack shapes and noncanonical sibling order are pruned instead. |
| What does deduplication mean? | Each canonical syntax program is emitted once. Different programs can compute equivalent Boolean functions; general algebraic-equivalence deduplication is not claimed. |
| Does this search event order? | No. Ordered temporal sequences require a separate causal strategy language and identity. |
| What is evaluated? | Exact stored, anchored indicator rows with separate truth and availability masks. Only definitely true expressions are hits. Missing evidence stays unknown. |
| Does a qualifying hit count admit a strategy? | No. MIN_HITS reports support qualification. The signal command saves signal research; the priced command prices every evaluated expression and records cell-rule outcomes, including explicit no-cell outcomes. Neither mints institutional approval. |

## Identity and persistence

The search identity extends the existing PARAMS term with domain `BTXEXS01`
and the complete initial cursor descriptor. All nine existing run terms remain
bound, including actual anchored input digest, clean verified commit and feed.
Later cursor progress cannot change the search identity. Each evaluated
expression has its own full `BTXEXPR1` program identity, durable start, exact
source timestamp/truth rows and completed child attempt. Search starts use the
new append-only operation value 7, `expression-search`; existing values keep
their meanings. VIX never enters these identities or the alphabet.

The cursor is exactly **3,086 bytes**: eight-byte magic `BTXEGN01`, three
little-endian `u16` values (alphabet count, instruction length, current offset),
finished and reserved bytes, 384 `u16` alphabet slots and 1,151 `u16` traversal
slots. Unused slots are zero. The decoder validates live sorted unique positions,
bounds, padding, partial stack shape and canonical sibling order. A structural
decoder is not authentication; the caller's envelope supplies identity and seal.

The search state is exactly **3,246 bytes**: magic `BTXESR01`, the cursor,
four `u64` counters (work, candidates, qualifying candidates and saved rows),
previous checkpoint sequence and 32-byte seal, optional candidate identity,
attempt and four-count signal summary, then presence/exhaustion flags and six
zero bytes. Every linked predecessor and its counters must reconcile. Every
referenced candidate must have its exact completed expression attempt and
fully verified signal file. Each transition is replayed against the grammar
using at most 4,096 recorded node operations; a syntactically valid cursor
cannot skip candidates or assert zero-work early exhaustion. Progress-only
checkpoints keep long grammar intervals within this same bound. Cold resume
and final outer publication re-read this complete linked history. The priced
variant additionally verifies the exact full program, execution digest, tier
controls and both trade children through the strict candidate reader. Missing
children are refusals, never zero rows.

## Shared checkpoint journal

The only namespaces are `and-checkpoint-v1` and `expression-search-v1`, directly
under the selected store root. Each full run ID has an exclusive `owner.lock`
and monotonically reserved, 16-digit hexadecimal sequence directories. A new
reservation is synchronized before its payload is created. The payload is a
64-byte header (magic `BTXCHK01`, run ID, sequence, length, eight zero bytes),
caller bytes and a complete 32-byte BLAKE3 seal. All integers are little-endian.
A separate 32-byte `complete` marker contains that seal.

The writer synchronizes and re-reads the held payload, checking its pathname
and file generation, before publishing the completion marker; it synchronizes
the marker and directory and verifies the payload again before acknowledging.
Any publication failure poisons that writer. A later invocation can inspect
the preserved history. An unmarked reservation is never resumed or overwritten,
and its count is disclosed. A marked but damaged latest checkpoint refuses;
the reader does not silently use an older checkpoint instead.

Creating the owner uses exclusive creation. Reopening an existing owner never
uses a create flag, checks a regular file, prohibits symlink following and
checks pathname generation against the held file. A dangling owner symlink
must refuse without creating its foreign target. Read-only snapshots do not
claim the writer lock or create paths; observing an owner lock is only a fact
at the instant of observation, not proof of continuing process health.

Cold discovery admits at most one million directory entries; linked expression
history admits at most one million checkpoints. Expression envelopes admit
8 KiB and individual signal files 64 MiB. These explicit limits can refuse a
large research job and must not be displayed as completed enumeration. BLAKE3
seals detect corruption; they are not signatures against an adversary capable
of replacing a complete history. The absence of a completion marker cannot
prove whether a crash or external deletion caused that absence.

## Cost and verification

The cursor, expression evaluation stack and per-candidate counters use fixed
memory. Total work grows with grammar choices and evaluated historical rows.
Saved history grows with checkpoints and candidate rows; cold authentication
reads that history. Durability and OS scheduling do not have constant latency.

The independent infix oracle in `vocab/tests/expression_search.rs` compares
finite nested grammars against the postfix cursor, including cross-word bits,
and checks 20,000 single-node checkpoint/restores with identical order and work.
CLI tests reopen real immutable files after separate execution batches, match
the continued candidate to an uninterrupted cursor, and refuse a removed child.
Journal tests cover duplicate ownership, interrupted reservations, byte ceilings,
foreign identities, poisoned writes, damaged latest markers/payloads, in-place
mutation and same-size path replacement. Runner identity tests bind the
language/alphabet/data/policy and prove progress-independent search identity.
The three shared pricing tests compare conjunction expressions with every
legacy grid field and materialized cell for both sides, exercise mixed OR/NOT
and unavailable predicates, and reject changed cell summaries. Seven CLI search
tests include sealed false-exhaustion/wrong-successor attacks, priced restart
with deliberate trade-child loss, independent grid/integer bounds, every
malformed policy override and sensitivity to all 14 variable pricing terms.
The real shared projection test requires dropped signals to appear in the
report. Ten expression tests include maximum wire formatting without recursion
and formatter error propagation. The final two additions reject output failure
at every rendered byte and compare a maximum 1,151-instruction left-nested AND
program with independently constructed text and True/False/Unknown outcomes.

## Earlier measured verification checkpoint — 2026-09-06

| Module | Lines | Branches | Functions | Regions | Full mutation evidence |
|---|---:|---:|---:|---:|---|
| `vocab/src/expression.rs` | 362/371, 97.57% | 71/75, 94.67% | 28/28, 100% | 614/655, 93.74% | 106 tested: 95 caught, seven unviable, three timeouts, one raw survivor |
| `vocab/src/expression_search.rs` | 193/197, 97.97% | 67/70, 95.71% | 18/18, 100% | 359/378, 94.97% | 120 tested: 115 caught, five unviable, no survivors or timeouts |

The expression survivor changes `left > right` to `left >= right` in
`Parser::binary`. The only newly taken case has equal slices, hence identical
lengths and contents; rotating their concatenation swaps identical halves and
leaves the bytes unchanged. Less-than and greater-than cases follow the same
branches in both versions. This mutation is equivalent by that argument, but
remains a raw survivor in the tool result. The readable guard was retained.
The three timeouts remove progress from the Display work counter or the
parser's space/operand cursor; they are timeouts, not caught assertions.
The raw zero-survivor gate for this module is therefore not met.

Expression coverage still misses checked-access and invalid-stack fallback
paths that validated public programs do not reach. Cursor coverage likewise
misses checked stack access and invalid-rank fallback paths. These defenses
remain in production; no private malformed state was manufactured merely to
turn the report green. Neither module has 100% line or branch coverage, and
these measurements do not establish whole-vocab or whole-workspace closure.

The exact logs are:

- Expression mutation pass: `/private/tmp/brutex-expression-closure-final-mutation-20260906.log`.
- Cursor mutation pass: `/private/tmp/brutex-expression-cursor-final-mutation-20260906.log`.
- Both current coverage summaries and detailed lines: `/private/tmp/brutex-expression-closure-final-summary-20260906.log` and `/private/tmp/brutex-expression-closure-final-lines-20260906.log`.
- Coverage test execution: `/private/tmp/brutex-expression-closure-final-coverage-20260906.log`.
- Isolated engine/vocab tests, 202 passed with zero failed or ignored: `/private/tmp/brutex-engine-vocab-closure-all-tests-20260906.log`.
- Engine/vocab all-target Clippy with warnings denied: `/private/tmp/brutex-resume-expression-closure-clippy-20260906.log`.

Raw mutation artifacts are under `target/sweep-audit-20260906/` in
`expression-closure-final-mutation/mutants.out/` and
`expression-cursor-final-mutation/mutants.out/`. The fresh coverage profile is
under `expression-closure-final-coverage/`. The final source SHA-256 values are
`4d0ad4cfbfd3819a08d730d3b7dedde1034821bb005e8cd6b0edc88238aec064`
for `expression.rs` and
`9a6abd07bcd7a5d80ef5caa61b6ae378e6d45e72c526274ef0e37900ede2907f`
for the unchanged cursor module. Repeated runs are not added as unique
mutations.

The installed `cargo-llvm-cov` 0.8.4 wrapper failed to discover this nightly
toolchain's executables under `debug/build/CRATE/HASH/out/TEST-HASH` after tests
and profile merge succeeded. The measurements above use the actual test
executables with the fresh merged profile through bundled `llvm-cov` directly,
with no data-mismatch warnings. The failed automatic report remains a tooling
limitation, not a passing automation gate.

These finite generated unit fixtures establish bounded differential and
corruption evidence, not all-input correctness. No real-market sweep or
ingestion operation was run for these checks. The dated combined verifier
records the actual executed integration gates.

## Later D-0526 reconciliation — 2026-09-06

The previous section is retained as historical evidence, including the original
timeouts and source hashes. A later repaired-source campaign tested 229 unique
mutants. Exact prefix-underflow replay resolved the timeouts: 216 caught,
12 unviable and one raw equal-sibling rotation survivor with the byte-equivalence
argument retained. No unresolved timeout remained in that campaign.

Later official file coverage was 465/479 lines and 70/75 branches for
`expression.rs`, and 242/245 lines and 68/72 branches for `expression_search.rs`.
The complete vocab suite passed 100 tests and strict Clippy. These are still
below the full coverage requirement. The [combined review](14-sweep-readiness-20260906.md#later-d-0526-repair-evidence)
names the exact logs and source scope. Neither an old heading nor a new passing
subset certifies untested source changes, full market execution or institutional
admission.
