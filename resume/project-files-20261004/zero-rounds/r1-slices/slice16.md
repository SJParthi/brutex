# slice16 — 0 findings

I read all 31 production files in the slice, most of them line by line: the candidate-trade capture and its codec, the Boolean candidate, OOS, statistics, qualification, grammar and search producers, their wire codecs and readers, and the command and launch parsers. I checked these:

- **Byte layouts.** Every encoder's fixed field offsets against its decoder:
  - candidate_trades codec, tier and summary;
  - statistics 512-byte records;
  - the qualification plan header (944 / 984 / 1024);
  - the qualification wire header, 1024 and 1136-byte rows;
  - qualified-journal rows (1136);
  - the search record header (2520) and its spec;
  - grammar PLAN/DONE envelopes;
  - the OOS 232-byte header.
- **Bounds.** Counts are checked before allocation, arithmetic is checked, and trailing bytes and padding are refused.
- **Reconciliations.**
  - `Financial` against runner `reconcile_rows`: same zero start for worst and peak.
  - `Sessions::observe` against statistics `project_row`.
  - The later OOS reader's trade exclusivity (`entry_bar > previous exit`) against runner `open_until` (`entry <= until` is blocked).
  - The 15:09 last-exit-bar deadline against runner `trade.rs`.
  - Romano–Wolf stepdown-recurrence checks in both readers.
  - Fold presence (`selected_exit` against `execution_refusal_bits().is_empty()`, kept equal by `CommittedBooleanFamilyV1::require_current`).
- **Resumable journals.**
  - Chain discovery, predecessor ordering and transitions: grammar, search and qualified campaign.
  - Search replay-node admission (`admit_history`) against `Reader::open` / `completed_record` charging.
  - The retry and refusal settlement paths.
- **Docs and limits.**
  - The CUH-06/07/08 invariant rows and their tests.
  - The D-1641 and 06-limits statements for the search, qualified-campaign and OOS cost paths. These are already recorded as W2-cli2-5 and W3-runner2-3.

I found nothing that met the evidence bar. The candidates I examined and dropped are listed below with the reason each was dropped.

- `codec::optional` maps `Some(usize::MAX)` to `None`. Ladder indices cannot reach that value.
- RW availability is labelled `ConstantReturnCandidate` for a one-period row. The layout guarantees at least 2 periods.
- `launch::validate_for_rungs` does not trim symbols, where `scope()` does. That makes it stricter, not wrong.
- The module rustdoc test path in `candidate_trades.rs:27` leaves out the `tests::` segment. This is cosmetic, and `docs/04-invariants.md` has the correct path.

The concurrency findings for `boolean_catalog_prepared.rs:225` and `boolean_oos_command.rs:100` are already known (hunt-conc-2), so I did not re-report them.
