# fxe report — worktree /home/claude/wt-fx-e, branch audit/fx-e, base 21443a2a

| item | verdict | decision | test | commit |
|---|---|---|---|---|
| srust-1 script via data/frame/srcdoc/handlers (gates 1, 1f) | FIXED (licensed loaders /typeahead.js, /masters.js kept) | D-4490 | FXE-01: `a_script_in_a_data_url_a_frame_or_an_encoded_attribute_is_refused` + 3 more (source_scan.rs) | 0cf6978a |
| srust-4 Gate 12 sed → Rust; refuse sed | FIXED (sed vs `gates-ledger path-declarations` byte-identical on 24,436 lines) | D-4493 | FXE-02: `a_sed_program_in_any_spelling_is_refused` | 8945b088 |
| srust-2 github-script / inline-code inputs | FIXED | D-4491 | FXE-03: `an_action_input_or_variable_that_carries_a_program_is_refused` | dd49f656 |
| srust-5 action allowlist, Node only in web job | FIXED as a gate. NEEDS-OWNER: whether Node actions count under §2 is the owner's ruling; CLAUDE.md not edited | D-4494 (7 actions, SHAs, runtimes, why kept, replacement cost; artifact service from `run:` UNVERIFIED) | FXE-04: `only_a_pinned_action_runs_and_javascript_stays_in_the_web_job`, `every_line_is_read_as_the_job_github_runs_it_in` | dd49f656 |
| srust-3 launch.json parsed | FIXED: config 1 is `cargo run --release -p api -- serve` (was `sh -c`); `gh_json launch` pins both configs; gate 1b admits only `.claude/launch.json` under any `.claude` | D-4492 | FXE-05: `launch_admits_…`, `launch_refuses_…` (gh_json.rs), `gate_1b_admits_one_file_under_claude` | 56c99de4 |

## Checks
- Probes, base then after: P6–P11 (html/literals) pass, then refused by gate 1/1f (P9 was already refused by gate 1). P13 sed passes gate 0, then refused. P12, P14–P17 (github-script, setup-node or npm outside web, unpinned action, INPUT_) pass gate 0, then refused. P18–P23 (perl -e, node -e, duplicate key, .claude/settings.json hook, .claude/commands/x.md, env key) pass gates 0/1/1b/1g/15, then refused by gate 1b. P24 (old sh -c) refused after.
- All language-purity static gates except 1e (0, 1, 1b, 1g, 1f, 1c, 1d, 2, 9, 9b, 10b, 7, 13, 15, 16, 17, 23, 21, 22, 24, 25, 27, 27b, 26, 19, 10, 11, 12, 14) run on the final tree with RUNNER_TEMP under /tmp/claude-0/gate-tmp-fxe: all exit 0.
- `/tmp/claude-0/bin/gates-doc.sh` passes; 0 missing.
- `rustfmt --edition 2024 --check .github/*.rs` passes. clippy-driver `-D warnings`, normal and `--test`, is clean for source_scan, gates_tree and gh_json. Tests: 70, 49 and 11 pass.
- ci.yml is 292,268 bytes, under 524,288.
- Crates: the only change is one assertion in `crates/cli/src/operator_boundary_tests.rs`, a `#[cfg(test)]` module. **NOT BUILT OR RUN**, because this fixer may not build cli. Its predicates were checked against the file with `grep -F`. No production crate code changed, so cargo-mutants was not run (no mutants to make).
- Not verified: whether the preview tools start `cargo` the way they start `npm`. This is marked UNVERIFIED in D-4492.
