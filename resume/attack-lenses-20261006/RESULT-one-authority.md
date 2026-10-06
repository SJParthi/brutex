# RESULT — lens L4 one-authority (branch `attack/one-authority`)

Status: rounds 1-3 fixed and pushed; round-3 validation (non-root tests, cargo-mutants) in progress; round 4 (exit
round) not yet run. This file is updated at each round.

| round | checked | found | refuted | fixed | evidence |
|---|---|---|---|---|---|
| 1 | (a) gate 1 family incl. spawn scan; (b) calendar, fold, costs, tick, vocab, universe, run identity, store paths, web copies; (c) index-name special-casing, resumability, caps; (d) §5 graph, §1 counts, §10 table, invariant rows, D-citations | 13 | 5 (csv::paisa strictness D-1494; recovery index list and research_family NTM gate D-0682; §10 count; P13-03 launch.json owned elsewhere) | 8: D-3500..D-3507 | each fix's test failed first (outputs in the D entries); fmt, clippy -D warnings, language-purity gates (not 1e) green; core/pull/lake/cli(1944)/api(1482) green non-root |
| 2 | fresh agents over (a) manifests, lock, includes, workflow logic; (b) 09:15/IST/civil-date/cost/vocab/identity/path/web copies, resumability | 11 | 5 (09:15 consts pinned; NSE_OPEN_MINUTE_V2 frozen; family encoding decoders refuse; forced-exit pinned; CODEOWNERS parse latent → 06-limits) | 7: D-3508..D-3514 (+ mutation-marker gate from coordinator) | tests red first; W1-W5 green for the web fix; 77 test binaries green non-root on 7cf031b |
| 3 | adversarial review of the lens's own diff + lens (d) leftovers | 9 | 0 | 9 folded into D-3500/3503/3511/3512 + D-3515 | reviewer's bypass inputs added to each gate's refused list; planted-file checks for path test and AGENTS §10 copy |

## Fixed (decision | invariant | finding)
| item | fixed? | commit | evidence |
|---|---|---|---|
| D-3500 spawn-scan spellings | yes | 0edc1a0, 2f4be13 | `a_spawn_through_another_spelling_of_the_constructor_is_refused` |
| D-3501 stale `ring` manifest comments + check | yes | 0edc1a0 | `a_manifest_comment_naming_a_banned_crate_says_it_is_banned` (5 stale mentions named) |
| D-3502 CLAUDE/AGENTS §5 pictures vs manifests | yes | 0edc1a0 | `the_law_pictures_of_the_graph_are_the_manifests` (red on drifted picture) |
| D-3503 gates 10b/27 `ID — claim` / `ID:` cells | yes | d02e10a, 2f4be13 | tool tests red first |
| D-3504 dangling D-2710, D-1619, I-41 + citations test | yes | d02e10a | `every_cited_decision_heads_an_entry` |
| D-3505 shift_six negative tie | yes | d02e10a | differential test over 3,100 texts |
| D-3506 one lock / ledger path | yes | d02e10a, 2f4be13 | `each_shared_results_path_is_built_in_exactly_one_place` |
| D-3507 swept_surface + 210-row /store axis | yes | d02e10a | `the_swept_surface_is_exactly_what_is_sweepable_admits`, census test |
| D-3508 committed mutation marker refused | yes | 830cafb | planted marker at tail.rs:503 → gate 1 exit 1 |
| D-3509 §10 table vs docs/ (+AGENTS copy, D-3515) | yes | 830cafb, 2f4be13 | planted docs/36-probe.md → red |
| D-3510 nightly toolchain doors | yes | a9ca724, 2f4be13 | gate 1g / build-keys tests |
| D-3511 gate 0 sed/make/rustup/git/env/xargs/find; gate 12 sed ported | yes | 7cf031b, 2f4be13 | `a_program_runner_that_is_not_an_interpreter_is_refused` |
| D-3512 one IST offset | yes | 7cf031b, 2f4be13 | `the_ist_offset_is_spelled_only_by_its_authorities` |
| D-3513 ssm stamp_of on telemetry civil date | yes | 7cf031b | `the_amz_date_is_the_civil_stamp_of_the_clock` |
| D-3514 web month presets 555→375 | yes | 7cf031b | web/tests/backtest-presets.test.js; W1-W5 |
| D-3515 round-3 corrections | yes | 2f4be13 | see entry |

## Already tracked (not mine)
- P13-03 `.claude/launch.json` runs `sh -c` (audit-helpers → zero-findings).

## Recorded, not fixed (docs/06-limits.md, round-2 section)
- civil-date copies (graph-forced by gate 22); ~20 rung lists; sweep-all/pool/range-all/descend not resumable;
  auto-merge CODEOWNERS text parse (latent).

## Needs the owner
- Whether `rust-toolchain.toml` / `Cargo.toml` join auto-merge's sensitive paths (D-3510). UNVERIFIED that it is wanted.

## Coordinator queue answers
- Duplicate `### D-0370` / `### D-0372`: D-0684 tabled both copies (subjects, writing commits) and kept them (append-only,
  cited); gate 27b already refuses every duplicate decision id and pins these five to exactly two. No change (D-3515).
