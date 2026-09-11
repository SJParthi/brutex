# Incremental historical Boolean campaigns

These Rust commands connect complete supplied catalogs, the existing fixed
AND/OR/NOT grammar, all eight intraday timeframes and separately measured
later-period comparisons. They use strict retained real OHLCV. No vendor pull,
tick reconstruction, guessed missing bar or synthetic price fallback is added.

| Need | What the implementation does | What completion means |
|---|---|---|
| Every intraday timeframe | Uses `1min`, `2min`, `3min`, `5min`, `10min`, `15min`, `30min`, `60min` in the existing canonical order | All eight requested training stages have saved receipts |
| Eligible indices and stocks | Preserves NIFTY/BANKNIFTY and the recorded F&O cash membership snapshot; executes independent families in bounded parallel workers | The complete explicitly named family set was computed |
| Full expressions | Keeps original versioned program bytes, including AND, OR, NOT and unknown indicator states | Every supplied program and both directions were captured |
| Interruptions and work limits | Saves immutable predecessor-linked progress; resumes incomplete work and verifies completed work before skipping it | A pause or refusal remains a pause or refusal |
| Later-period checking | Reuses every frozen training exit on strictly later retained OHLCV, including zero-trade settings and zero-return sessions | The saved later comparison was measured; it is not strategy admission |
| 3:10 PM exit | Reuses one-minute execution and the existing 15:10 IST deadline | A 15:09 bar may close at the deadline; a 15:10-open bar is too late |
| Database and audit trail | Uses the repository's versioned append-only store, exact identities, durable bodies, completion receipts and lifecycle events | Successful publication requires the actual source/body checks |
| Easy progress view | Shows eight timeframe rows and exact catalog, statistics and research-comparison links | Recorded progress and saved child receipts, separately from full detail authentication |
| Search completeness | Advances the fixed grammar cursor only after the exact batch completes all eight timeframes | Work allowance does not prove exhaustion; separate batch statistics do not prove full-search significance |

## Commands

All commands require a clean stamped build, the existing server-owned
`BRUTEX_STORE`, strict receipt configuration and the research policy described
in [the policy guide](22-research-policy.md). Source and output roots are
distinct explicit authorities. Observing the output requires the dashboard's
`BRUTEX_STORE` to name that output root.

```
cli boolean-campaign-stored VENDOR SYMBOLS FROM_Y FROM_M TO_Y TO_M CATALOG_FILE HORIZON MAX_POINTS RUNG_JOBS OUTPUT_ROOT
```

`SYMBOLS` is an explicit comma-separated research scope. `CATALOG_FILE` is one
expression per line; the existing bounded text parser refuses incomplete,
invalid or duplicate canonical programs. `HORIZON` counts one-minute execution
bars. `MAX_POINTS` uses the existing paisa conversion and delegated research
profile. `RUNG_JOBS` admits one through eight timeframe jobs for this invocation.
It is a work allowance, not a strategy depth setting. The same request resumes
the same saved source/program/policy campaign; changing those inputs creates a
different identity. Completed stages are not silently recalculated with new data.

A deliberately paused standalone campaign returns a nonzero exit status and
preserves its explicit paused checkpoint. Automation must inspect the saved
state to distinguish a work allowance from a refusal; neither is full campaign
completion. Its dashboard link identifies the exact campaign and automatically
follows acknowledged progress while work is running. Paused, refused and
completed states stop automatic refresh; opened details retain their own pins.

```
cli boolean-grammar-campaign-stored VENDOR SYMBOLS FROM_Y FROM_M TO_Y TO_M BITS HORIZON MAX_POINTS BATCH_PROGRAMS NODE_ALLOWANCE OUTPUT_ROOT
```

`BITS` is `all` or a sorted unique set of live vocabulary IDs. The existing
fixed V1 grammar enumerates canonical syntax, including repeated conditions
and nested operators. Algebraic equivalence is not inferred. The supplied work
allowances bound one batch; no grammar depth parameter is introduced. Binary
programs bypass text-source nesting limits without weakening the fixed wire
validator. The saved cursor cannot advance past an incomplete eight-timeframe
batch. An unchanged request resumes after checking the complete acknowledged
history, original batch programs and freshly admitted source identities.
It reserves room for the complete next checkpoint transition before pricing,
including interrupted reservations and the shared directory limit. Exhausted
history refuses visibly; it is not reported as completed grammar. Original
source guards remain held and checked through the final grammar acknowledgment.

```
cli boolean-oos-stored VENDOR SYMBOLS RUNG FROM_Y FROM_M TO_Y TO_M CATALOG_FILE HORIZON MAX_POINTS OUTPUT_ROOT LATER_FROM_Y LATER_FROM_M LATER_TO_Y LATER_TO_M
```

The later range must start after the training range. The command captures the
complete original catalog and observes its same coordinates later. Original
and later programs, family, feed, build, evaluator, horizon and frozen exit
parameters cannot be substituted. Read-only later pages authenticate the
original catalog and the later artifact together; they cannot reconstruct the
opaque source authority needed to publish new evidence.

## Resource and statistical limits

Family workers share the explicit capture/source byte and record allowances.
Timeframes are admitted sequentially so eight separate worker pools do not
silently multiply that budget. These ceilings are not a total RAM guarantee.
Cold source admission, checkpoint discovery, full child authentication, grammar
replay, statistical calculations and total storage all require work proportional
to their inputs. Fixed per-operation engine measurements do not establish
constant total latency or constant storage for arbitrary history.

Campaign overview authentication reads the saved snapshot and fixed completion
receipts. It intentionally states that child bodies have not been opened;
the existing bounded detail routes authenticate those bodies when requested.
An oversized detail refuses explicitly instead of showing a partial success.

The research profile remains unchanged. A measured later comparison does not
supply missing anchored folds or a complete later institutional selection
transaction. Statistical tests are scoped to each explicit catalog and
timeframe; they are not a correction across every grammar batch or every
timeframe. Cost-excluded research amounts are not net trading profits. No
passing strategy, production readiness, universal test coverage or exhaustive
search runtime is inferred from a saved completion marker.

Implementation choices are recorded in D-0539 through D-0541. Fresh combined
checks and real-data evidence must name their actual source checkpoint; older
green runs do not verify later code edits.
