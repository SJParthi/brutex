# Exact priced-candidate evidence, version 1

This format captures the exact policy-selected exit cell for **both directions
of every retained signal candidate actually evaluated by each visited screen
pass**. It preserves nonwinning candidates and explicit no-cell outcomes.
It does not call every potential combination priced. The upstream retention
limit, screen cap, budget calibration and visited policy tiers still bound the
work. Nor does it store every exit-grid cell, bootstrap sample or validation-fold
grid as a separate candidate.

## Authoritative execution boundary

`cli::screen` still calls the same `grid::evaluate_over`, `shown_cell`, side
comparison, consistency checks and report selection. When each side's actual
grid is in scope, it passes an `Evaluated` value to
`candidate_trades::Capture::record`. The capture retains the supplied execution
bars and projected column; a caller cannot substitute a new slice per row.
It checks the supplied selection against the same policy selector and calls
`grid::materialize_cell` with that exact cell, mask, side, horizon and actual
ladders. That engine function compares the complete replayed cell and reconciles
trade count, both financial totals, worst trade and drawdown. There is no
winner-trade substitution or inferred execution direction.

The cell's `admitted` field records **cell rules only**. Later calendar checks,
policy-tier choice and institutional authorities are separate facts. The
capture's invocation ordinal is not the displayed policy-tier label or final
frontier rank. Its rank is the one-based retained signal-evidence position.
The manifest repeats the full mask and direction so these distinctions remain
checkable without guessing from report order.

`audit_bars_work`, reached by the stored monthly/range audit and screen paths,
starts capture before pricing. `trade_and_screen` seals the capture before it
returns the chosen or unadmitted result. Both publication branches call
`Capture::confirm` again immediately before writing the legacy parent, because
validation can take time after pricing. A refusal returns before the parent
publication. A callback error is latched: not-yet-started candidates and sides
check it and skip work; a grid already executing finishes its current engine
call. No immediate interruption inside that grid is claimed.

## Paths and byte layout

All files live under:

`results/candidate-trades-v1/<full-run-id>/<attempt-token>/`

Existing result, frontier, trade and receipt formats are untouched. Each file
has a 16-byte header: eight-byte magic, little-endian `u32` version **1**, and
four reserved zero bytes. It ends with the full 32-byte BLAKE3 digest of its
header and payload. This is corruption detection, not a digital signature.
Unknown headers, flags, reserved values, extents and trailing fields refuse.
Numbers below are little-endian; signed prices and ppm remain `i64` integers.

| File | Magic | Payload |
|---|---|---|
| `start.bin` | `BRPCST01` | Parent identity 32 bytes, attempt `u64`, execution digest 32 bytes. Exactly 120 total bytes. |
| `tier-<i>.bin` | `BRPCTR01` | 176 fixed payload bytes followed by the counted explicit stop values at 8 bytes each. |
| `<tier>-<rank>-<side>-candidate.bin` | `BRPCCA01` | 536 fixed payload bytes plus actual stop, target and trailing values at 8 bytes each. |
| `<tier>-<rank>-<side>-trades.bin` | `BRPCTX01` | Zero or more existing chosen-trade `Row` records at stride 136. |
| `catalog.bin` | `BRPCCT01` | Parent identity, attempt, execution digest, tier count: 80 bytes. Then each tier's 32-byte digest, `u64` side count, and one 32-byte manifest digest per side outcome. |

A side is **0 long, 1 short**. A key is `(tier, rank, side)` with zero-based tier
and one-based rank. Catalog positions are deterministic: rank 1 long, rank 1
short, rank 2 long, rank 2 short. Worker completion order cannot reorder them.

A tier writes these ten words first: invocation index, eligible count,
evaluated count, horizon, requested rung count, step-present flag, step,
forced-present flag, forced stop, ratios flag. The eleven policy words follow:
maximum MAE, minimum reward/risk, minimum win rate, minimum trades, minimum
assurance, minimum weakest period, minimum return/drawdown, protective-exit
flag, minimum fill headroom, minimum average reward/risk, visible top count.
The final counted section is the explicit stop ladder passed to evaluation.

A candidate writes: parent identity; attempt; tier/rank/side; six mask words;
execution digest; tier-file digest; admitted flag; cell-present flag; thirty
cell words; signals; refused paths; three counted actual ladder sections;
candidate trade identity; and trade-file digest.

The thirty cell words preserve the complete existing `grid::Cell`, in order:
stop, target, trailing-stop index, trailing-profit arm and trail indices;
trades, wins, pessimistic, optimistic, fill cost; fixed-stop, trailing-stop,
trailing-profit, target and timed-exit counts; ambiguous bars and gaps;
winner MAE, winner MFE, all MAE, worst MAE; gross win, gross loss, best trade,
minimum win; bars held, maximum losing and winning streaks; worst trade and
maximum drawdown. Optional indices use `u64::MAX` for absence. A trailing
profit requires both indices. With no cell the complete cell payload is zero
apart from absent-index sentinels, and `admitted` is false.

The candidate trade identity is BLAKE3 of the domain
`brutex-priced-candidate-v1\0` followed by its encoded manifest with both trailing
digest fields zeroed. It specializes the full nine-term parent identity with
actual candidate/side/tier/grid authority; it does not replace the parent run
identity. Each 136-byte trade row repeats this specialized identity, its exact
sequence and direction. Its eleven `u64`/`i64` measurements retain signal,
entry and exit indices, both outcomes, entry/exit timestamps and both excursion
units. The existing row seal is preserved in addition to the whole-file digest.

## Publication and recovery

Files are `create_new`. Existing bytes can be reused only if their complete
payload and digest equal the newly computed bytes. A conflict or torn existing
file refuses; this implementation never truncates, deletes or overwrites
history. Writes synchronize the file, re-read its retained bytes and synchronize
the containing directory before acknowledgement.

The writer remembers each acknowledged manifest digest at its deterministic
slot. `finish` requires every expected side outcome, verifies original tier
bytes and every manifest, fully verifies each trade body and its financial
reconciliation, and only then publishes `catalog.bin`. Deleting acknowledged
evidence cannot relabel a completed capture as empty. `confirm` repeats that
proof against the retained acknowledgements before legacy parent publication.
It requires the already-saved catalog to remain present and identical;
confirmation never recreates a deleted seal.

A catalog is **prepared exact evidence**, not proof that the whole audit
completed or that its parent was committed. A crash or later validation refusal
can leave a complete catalog beside a refused/incomplete audit. The consumer
must show those facts separately. Without a catalog, `read` returns `None`;
partial children are not a completed empty answer. Normal reruns have distinct
durable attempt tokens. Exact duplicate publication within an attempt reuses
identical files; concurrent independent attempts never share child paths.

## Bounded reader contract

`read(root, identity, attempt, max_bytes)` returns an immutable `Summary` with
identity, attempt, execution digest, tier count, candidate-side count and full
catalog digest. It validates the catalog and original capture reservation.
The summary privately retains the expected per-tier and per-manifest digests.
API requests must pin both attempt and catalog digest; every response should
return them so a browser never joins pages from different snapshots.

`tier(root, &summary, tier, max_bytes)` returns exact policy, grid inputs and
eligible/evaluated counts. `candidates_page(root, &summary, tier, start, limit,
max_bytes)` addresses consecutive side positions, with `1 <= limit <= 256`.
It rechecks the catalog before and after reading, verifies each manifest against
its catalog digest, and shares one byte budget across returned manifest files.
This metadata page does **not** claim the trade bodies were reverified.

`TradeReader::open(root, &summary, key, max_bytes)` validates the exact selected
manifest and every row of its trade file, checks full content/row digests,
sequence, direction, financial aggregates and before/after filesystem
generation. A retained reader's `page(start, limit)` performs direct row seeks
and checks generation and row seals before returning up to 256 rows. Same-length
mutation, truncation and atomic path replacement refuse; cached receipts are
not silently reused across a changed generation.

The default cold-file limit and writer trade-file limit are 64 MiB. The writer
also refuses before a new tier if all retained acknowledgement slots would
exceed 64 MiB. These are explicit evidence-admission limits, not a promise that
all searches fit. Grid replay, cold file verification and catalog confirmation
cost work proportional to their inputs. Capturing every priced candidate adds
one exact replay per evaluated side and persistent space proportional to saved
trades. Warm trade-page addressing is O(page rows); catalog reads and permanent
history are not constant time or constant space. No latency guarantee is made.

## Explicit expression captures

`Capture::begin_expression` binds one immutable, complete
`vocab::expression::Expression` to the same execution slice before its grids
run. It uses `grid::materialize_expression_cell`, which reuses the ordinary
entry/occupancy/exit engine, compares the complete selected cell and reconciles
the exact rows. Unknown predicates never become trade signals. The passed mask
must equal the predicate's referenced bits; those bits do not mean AND.

Expression captures have their own path:
`results/expression-candidate-trades-v1/<full-run-id>/<attempt-token>/`.
The start magic is `BRPEST01` and catalog magic is `BRPECT01`. Their bodies
append the complete fixed-width expression encoding after the otherwise shared
payload. Header version remains 1; the complete program's own version and
padding are validated by the vocabulary decoder. The start and catalog must
contain the identical program. Tier, candidate and trade codecs are shared
without changing the existing AND files. The expression trade identity uses
`brutex-priced-expression-candidate-v1\0`, then the complete encoded program,
then the same zeroed-digest candidate payload.

`Model::And` and `Model::Expression` are explicit reader authorities.
`read_model` reads only the requested namespace; the original `read` remains
AND-only. `Summary::expression` returns the complete saved program. An absent
expression catalog cannot fall back to an AND catalog, even with the same
referenced bits, attempt and parent identity. This capture does not establish
institutional admission for a Boolean expression.

## HTTP and dashboard contract

`/candidate-trades.json` uses the shared bounded detail workers, a 512-byte
query limit, 64 MiB cold-file admission and an 8 MiB JSON response limit. Invalid
queries return 400, exhausted worker capacity 429, and refused evidence 503.
No partial prefix or another candidate's trace replaces a failed request.

Every request supplies `identity` and a positive `attempt`. `model` is
`and-mask` (the compatibility default) or explicitly `expression`. The initial
candidate page may omit `digest`; every continuation or trade request supplies
the full catalog digest. `tier` is a zero-based invocation ordinal. `offset`
is a direct zero-based row offset, not a global-history page number; `limit`
is 1 through 256. Trade requests additionally supply one-based `rank` and
`direction=long|short` together. Unknown, duplicate, empty, noncanonical and
out-of-extent inputs refuse.

The saved response repeats model, identity, attempt, catalog/execution digests,
all tier rules and caps, exact row counts and the next offset. Expression
responses include the complete encoded program and the vocabulary's canonical
fully parenthesized text. `capture=sealed-pricing-evidence` and the separately
read attempt's `audit_completion` are distinct. Missing capture is explicit,
never a measured zero. Every 64-bit value is a decimal string; no statistical
or money field passes through a JavaScript number. One retained API trade reader
supports warm pages for one exact root/model/run/attempt/catalog/candidate key;
generation changes refuse rather than silently refreshing that reader.

The backtest evidence component shows the actual pricing cap, both directions,
each candidate's cell-rule outcome and an exact-trades action. No-cell and
unavailable evidence have separate states. It keeps one candidate page and one
trade page, pins all continuations and rejects stale asynchronous responses.
Condition names come from the existing canonical vocabulary response. Expression
condition names are labelled as referenced bits, with the full predicate shown
separately. Money uses integer paisa formatting; timestamps display IST with
the exact microseconds retained. Building these assets does not deploy or
restart the running service.

## Search ID to exact expression trades

The backtest page has a search-ID panel backed by `/expression-search.json`.
It reads `expression-search-v1` checkpoints without creating directories or
acquiring the writer's exclusive owner lock. `search_checkpoint::Snapshot`
shares the existing verified payload reader with `Journal`; file headers,
full seals, exact sequence/identity, completion markers and generation checks
remain authoritative. Discovery examines at most one million directory entries.
The nonblocking shared-lock probe records whether an owner was observed at that
moment; it is not continuing process liveness.

The initial request carries the full search `identity`. Every continuation
carries `snapshot`/`snapshot_seal` and `cursor`/`cursor_seal` pairs. The API holds
at most eight read-only snapshots, each retaining at most 4096 learned
continuations. A cursor is accepted only after a verified walk from that exact
initial snapshot learned it. Unknown, expired or spliced tokens refuse; they
never reopen a mutable latest result silently. Refreshing an identical snapshot
replaces its cached continuation authority explicitly.

`expression_search::reader::Reader::page` checks one through 256 checkpoint
links, newest first. A link can be progress-only, so a page can contain zero
candidate rows and still carry a next cursor. API and browser default to eight
links; the browser offers smaller pages when evidence is large. Each page
verifies predecessor seals, exact cursor transitions, monotone/reconciled
counters and completed expression-child signal receipts before returning any
prefix. The child identity, attempt, full canonical predicate, signal counts
and optional capture digest lead directly to `CandidateTrades(model=expression)`.
That component retains the supplied capture digest on its first and subsequent
requests, rather than substituting a different catalog.

The page has a shared 64 MiB conservative read admission. Each checkpoint is
charged its full 8192-byte file admission plus two 32-byte marker reads. Each
child reserves four 4096-byte exact-attempt file admissions and two 8192-byte
pricing catalog/start admissions. Signal verification reserves three times
the measured file length before opening the full reader, covering its header
probe, integrity scan and row visitor scan. Initial and final page anchor checks
are charged too. Initial snapshot opening has a separately bounded 8256-byte
checkpoint read plus directory/owner metadata work. Files larger than these
particular view admissions refuse explicitly; a valid persisted file is not a
promise that every browser page fits. The standard expression-search pricing
path records one tier and both directions, whose catalog fits this limit.

The API serializes progress only after at least the latest transition has been
checked. `exhausted-observed` means a recorded cursor exhausted its versioned
language; it is not successful whole-history or market recomputation. Older
history beyond the current page remains unvisited. Aggregate qualifying counts
are recorded counters, not a re-proven support threshold, run identity recipe,
financial selection or institutional admission. The page states these limits.

## Verification

Named tests are in `crates/cli/src/candidate_trades/tests.rs`: exact two-side
replay and paging; repeated publication and conflicting selection; explicit
no-cell/multiple-tier metadata; missing, truncated and foreign acknowledged
children before and after sealing; warm-reader mutation and replacement;
concurrent publication and same-identity attempts; real screen nonwinner traces
and mixed policy results; actual pricing callback refusal.
`actual_audit_refuses_a_failed_capture_before_publishing_its_parent` reaches
the real audit path and proves no legacy parent is saved after a failed capture
start. `final_confirmation_never_recreates_a_missing_sealed_catalog` covers
post-seal deletion. `expression_capture_replays_or_not_without_relabelling_the_same_referenced_and_bits`
compares contradictory and tautological expressions over identical referenced
bits, with exact replay, explicit namespace and immutable descriptor checks.

API tests in `crates/api/src/candidatejson.rs` exercise real public capture
files across a 256-row boundary, exact large integers, missing versus sealed
evidence and lost-child refusal. Frontend protocol tests in
`web/tests/candidate-trades.test.js` cover pinned pages, exact candidate/side
authority, damaged extents, no-cell results, canonical vocabulary names and
explicit expression versus AND model refusal.

`expression_search::reader::tests` covers reading while a writer owns the
namespace, absence without directory creation, zero-candidate continuation,
exact child counts, foreign cursors, lost children, changed checkpoints, byte
admission and freshly sealed but inconsistent transitions. API tests live in
`crates/api/src/expressionsearchjson.rs`; browser protocol tests are in
`web/tests/expression-search.test.js`. They cover exact large counters,
snapshot/cursor pairing, missing evidence, malformed pages and the distinction
between observed cursor exhaustion and historical assurance.

Test/build results belong in the separately archived verifier output. This
format document describes the invariants and tests without claiming a gate
that has not run, universal crash coverage, complete mutation closure or full
institutional readiness.

## Contended reads and failed-page recovery

Candidate catalog, tier, manifest and cold/warm trade readers use a nonblocking
shared file lock. A conflicting exclusive lock produces a named busy refusal;
it does not occupy a detail worker waiting for that owner. Search checkpoint
and signal-receipt reads use the same refusal policy. Their writer locks and
all byte formats are unchanged. Exact saved evidence can be retried after the
owner releases its lock.

The shared `readonly_file` helper opens candidate, checkpoint and expression
evidence with `O_NOFOLLOW | O_NONBLOCK`, then requires a regular handle and
matching filesystem generation. A final-component symlink cannot be followed,
and a FIFO swapped into that position cannot wait for a peer. All subsequent
expression header/body opens use this helper too. Supported flags are verified
for macOS and Linux x86_64/aarch64; other targets explicitly refuse this read
boundary rather than guessing their ABI. The Linux values come from its
[primary UAPI header](https://github.com/torvalds/linux/blob/master/include/uapi/asm-generic/fcntl.h);
the macOS values are recorded in the local SDK's `sys/fcntl.h`.

Intermediate directory traversal still depends on a trusted store root: this
is not a descriptor-relative path sandbox. Filesystem metadata/device I/O and
regular-file reads retain their operating-system scheduling limits. Neither
the open flags nor nonblocking advisory locks establish constant total latency.

The named regressions are
`busy_catalog_tier_manifest_and_cold_warm_trade_reads_refuse_without_waiting`,
`candidate_paths_refuse_symbolic_aliases_to_otherwise_valid_evidence`,
`busy_checkpoint_and_signal_receipts_refuse_without_stranding_a_reader`,
`failed_multi_child_page_learns_nothing_and_the_same_cursor_can_retry`, and
`complete_page_admission_charges_both_signal_passes_and_all_child_receipts`.
`readonly_file::tests` additionally checks regular/directory/symlink handling
and a FIFO with no writer. The FIFO test uses a separately bounded native Rust
test process, which is terminated if the open ever waits for a peer; a failed
nonblocking flag therefore fails the test without hanging the test suite.
Contention tests release their fixture lock before asserting, so a blocking
regression fails without leaving a worker stranded. The page-admission test
rejects a budget one byte below the documented conservative cold-read cost,
then verifies the same complete page at its admitted bound. A failed page
does not teach a new continuation, and retry retains its original cursor.

Both browser detail clients preserve a server refusal reason only when the
response has the known refused schema, an empty row list and a nonempty reason
of at most 4096 characters. Unknown schemas, oversized reasons, partial rows,
non-JSON proxy errors and absent responses keep the generic failure message.
`web/tests/detail-refusal.test.js` proves this boundary; an error response never
becomes a result page.
