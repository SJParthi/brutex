# Data-path attack thread — resume state (saved 2026-10-04 ~18:50 UTC, 5-hour usage pause)

Project thread: "data path attack" (Brutex project). D-block D-3100..3199.

## Branches
- `claude/attack-data-pipeline-hgxmw9` (off final/all-fixes 956424c): committed greeks D-3100/3101, pricing D-3110..3112, decoders D-3150..3157. Plus a WIP commit (if present) of attackers still running at pause: ingest (D-3120..), fold (D-3130..3132, report done), store (D-3140..), pricing follow-ups D-3114..3116, decoder follow-up D-3158.
- `claude/attack-gdfl` (off origin/feat/gdfl-import 8e6b521): gdfl-names D-3160..3163 committed; follow-up in progress = replace blanket FormsAmbiguous with the SOURCED era rule (cutover trade day 2019-02-01; before it only NIFTY/BANKNIFTY weeklies are dated, all else monthly; Mac census 20.9M names, 0 ambiguous). gdfl-seconds (D-3170..) WIP.
- No PR is opened (user rule: one PR, #74). Hand both branches to the PR 74 CI thread via the coordinator when the round is clean.

## Not yet done
1. Docs: append decision entries, invariant rows (prefixes DPG-, DPP-, DPI-, DPF-, DPS-, DPD-, DPN-, DPT-) from the r1-*.md drafts into docs/05, docs/04, docs/11. None written yet.
2. Workspace checks: cargo fmt --check, clippy -D warnings (attackers left too_many_lines at http.rs:1247, ingest.rs:1759 and pedantic lints in tests/attack_ingest.rs, gdfl_fixtures.rs, gdfl_seconds_attack_tests.rs), cargo test --workspace (as uid 65534 for store perms).
3. Round 2 over the same 8 areas; repeat until a round finds zero.
4. Comparison-table Artifact (attack | cases | failures | fixed | evidence).
5. Open named items: D-3113 Saturday expiry needs a charter-sourced rule; real-zip questions sent to the Mac GDFL thread (see r1-gdfl-names.md).

## Update 19:16 UTC (pause)
- Committed since: fold D-3130..3132 (1b1c4dc), masters D-3158 (dbb4153), gdfl-names era rule D-3160/3161/3164 (0389b3c on claude/attack-gdfl).
- Agents stopped at pause: ingest, store, gdfl-seconds, pricing follow-ups (D-3114 vendor premium below intrinsic; D-3115 chain exchange check; D-3116 chain dedupe by contract). Their partial work is in WIP commits "WIP at 5-hour pause" on both branches; re-check them with tests before trusting.
- gdfl-seconds open asks: key contracts by decoded (underlying, expiry, strike, side) across the 2019-02-01 rename (ACC19FEB1260PE -> ACC28FEB191260PE); fix gdfl_import_tests.rs:905 collision test (assert named refusal; FormsAmbiguous with NIFTY24JAN1910500CE on 2019-01-15); flaky NotFound at gdfl_seconds_attack_tests.rs:222; nfo_day starts_with prefix filter.
- NEXT TASK on resume: wire the monthly-expiry table (Mac census, 1,218,362 tickers, 0 disagreements; table on Mac at /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/name-census/monthly-expiry-table.txt) into gdfl_nfo.rs as a sourced const so MonthlyExpiryUnstated stops firing:
  2018-09->2018-09-27, 2018-10->2018-10-25, 2018-11->2018-11-29, 2018-12->2018-12-27, 2019-01->2019-01-31, 2019-02->2019-02-28, 2019-03->2019-03-28 (stock only);
  long-dated index monthlies 2019-06-27, 2019-09-26, 2019-12-26, 2020-06-25, 2020-12-31, 2021-06-24, 2021-12-30, 2022-12-29.
  Citation: NSE contract specifications https://nseindia.com/products/content/derivatives/equities/contract_specifitns.htm — OPTIDX: "Last Thursday of the expiry month. If the last Thursday is a trading holiday, then the expiry day is the previous trading day." (stock rule not quoted on that page; stocks sourced from the vendor last-trade days + dated twins). A month outside the table stays MonthlyExpiryUnstated. One test per month.
- Then: docs entries, workspace fmt/clippy/test, round 2, Artifact.
