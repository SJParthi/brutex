# Boolean catalog research: what each saved stage proves

This route keeps the full meaning of AND, OR and NOT programs when pricing
eligible NSE indices and cash stocks. It uses real retained historical OHLCV,
intraday execution and the fixed 15:10 IST deadline. It does not turn futures,
options or VIX into swept instruments.

| Stage | What is saved | What completion means | What it does not mean |
|---|---|---|---|
| Candidate capture | Every supplied program, both directions, every resolved exit coordinate, exact completed trades, accepted sessions including zero-trade sessions, and the actual grid settings | The complete requested family catalog was computed and its source/body receipts were checked | Exhaustive Boolean grammar search, all-timeframe completion or an acceptable strategy |
| Statistics | The complete canonical candidate population, aligned session evidence, exact procedure settings, family tests and every cross-validation split | The recorded procedures ran on that exact complete population | A passing policy, later-period validation or future profitability |
| Research admission | Every candidate's measured/missing/refused evidence and all failed, unmeasured and refused reason masks against the selected 39-rule policy | The comparisons and their complete results were saved | Selection V6 authority, deployment approval or a guarantee |
| Read-only observation | Pinned bounded pages from authenticated saved receipts and their retained ancestor links | These pages belong to that particular checked saved evidence snapshot | Permission to author another authority record or a fresh check of the original raw-market files |

The [37-setting guide](22-research-policy.md) explains the research profile.
The two existing rules complete the 39-field policy. Execution time rules are
separate and mandatory. Cost-excluded research remains labelled cost-excluded.

## Command and boundaries

`cli boolean-catalog-stored VENDOR SYMBOLS RUNG FROM_Y FROM_M TO_Y TO_M CATALOG_FILE HORIZON MAX_POINTS OUTPUT_ROOT`

`SYMBOLS` is an explicit comma-separated list within the existing research
surface. `RUNG` is one of the canonical intraday timeframes. `HORIZON` is a
positive number of one-minute execution bars; the 15:10 deadline still applies.
The catalog file contains one numeric Boolean expression per non-comment line.
For example, `52 & 53`, `52 | 53` and `!52` illustrate different program syntax;
they are not recommended trading strategies.

The command validates the selected policy before market preparation. It requires
a clean identified build, strict source-receipt configuration, the exact requested
months and explicit resource limits. Independent families may run concurrently.
If one family refuses, saved successful family evidence remains available but
the whole requested cohort is not declared complete.

The dashboard uses its existing server-owned `BRUTEX_STORE` evidence root.
For dashboard visibility it must name the command's `OUTPUT_ROOT`; a separately
archived CLI result is not automatically visible in a differently configured
service. Browser requests cannot choose a filesystem path, and readers do not
scan alternate roots. The command reports this location explicitly.

The input catalog has a 65,536-byte format ceiling and must be accepted in full.
It is not a depth cap. Duplicate canonical program bytes, invalid syntax and
an empty catalog refuse. Algebraically equivalent but differently encoded
programs are not claimed to be semantically deduplicated.

## Difficult outcomes that remain visible

| Situation | Recorded behavior |
|---|---|
| Every predicate is Unknown | Preserve Unknown; a legitimately empty coordinate can have zero trades without becoming missing evidence |
| No accepted exit cell | Retain the actual grid refusal and coordinate; do not invent a winning cell |
| One candidate has constant session returns | Keep it in the population; the existing Romano–Wolf method may be unavailable for the complete family |
| Session counts cannot form the required equal splits | Refuse that statistical layout without dropping or padding real sessions |
| Later OOS evidence has not been produced | Preserve Unmeasured; training statistics cannot fill those fields |
| Body or source changes before publication | Refuse completion; retained partial evidence is not a successful empty result |
| A reader meets an active publisher | Refuse a busy observation without accepting bytes that are still being published |
| A budget cannot cover complete work | Refuse with the actual reason; no accepted candidate or history prefix |

## Storage domains

All three stages use separate versioned namespaces under the explicit output
root: `boolean-candidates-v1`, `boolean-statistics-v1` and
`boolean-admission-v1`. An identity names a directory containing `body.bin`,
`complete.bin` and an empty `owner.lock` used for publication coordination.

The shared 112-byte `BRBLCM01` completion has magic at byte 0, identity at 8,
body BLAKE3 at 40, body length at 72 and the BLAKE3 of the preceding 80 bytes at
80. Integers are little-endian. Completion is written after checked body
publication and synchronized before acknowledgment. An existing identical body
is verified and reused; different bytes are not silently overwritten.

The candidate body has a 224-byte header: `BRBOOL01`, its identity, the 32-byte
common cohort digest, the 128-byte full research family and three u64 counts for
programs, sessions and coordinates.
Full programs use 3,457 bytes each; accepted sessions use signed 8-byte day
identities. Each of the two frozen grid descriptors uses 384 base bytes plus
16 bytes per requested rational percentile and 8 bytes per actual axis level.
Each coordinate has 392 fixed bytes followed by 32 bytes per session and 88 bytes
per exact trade. Every count
is bounded and reconciled before a reader publishes a page. The grid descriptors
retain the requested horizon, actual policy, exact observed axes and source
identities; ordinal numbers alone are insufficient for later interpretation.

Statistics use 512-byte records: a manifest, one source record per family,
one record per candidate, one per split and one family-test record. Admission
has a 512-byte manifest followed by 1024-byte candidate records containing the
448-byte `BRAPRO01` comparison. Reserved bytes stay zero. None of these formats
is accepted as an older two-index candidate, statistics, admission or selection
format. Decisions D-0536 and D-0537 define these boundaries.

Cold statistics and admission observations check their linked candidate
identities, completion pins, common cohort, programs and session order. They
cannot reconstruct the original source-root path from hashes alone and do not
claim to re-audit raw-market files. Only the live authoring pipeline retains
the source capabilities needed for that stronger check.

## Verification and performance limits

The implementation and its tests are being integrated on 7 September 2026.
Consult the [readiness comparison](14-sweep-readiness-20260906.md) and each saved
test log for the source actually checked. This document is a contract, not a
claim that the new route has passed every release gate or run a full market
campaign.

Source-file limits and per-family capture limits are physical bounds. They are
not a total process RAM guarantee. Candidate storage grows with retained trades
and sessions; statistics grow with candidates, periods, resamples and splits.
Cold authentication reads the complete saved body. Fixed-width lookup and bounded
warm pages do not make total enumeration, history or disk latency O(1).
