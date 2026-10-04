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
