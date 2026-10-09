# fxc pause state (branch audit/fx-c, worktree /home/claude/wt-fx-c, base de932e5b)

- 1 rnew-1: DONE (168f7021; docs 60a78c40) - D-4460, FXC-01; targeted tests green.
- 2 sobs-12 (cli): DONE (168f7021) - D-4461, FXC-02; api/src/audit.rs not in scope.
- 3 sobs-4: DONE (168f7021) - D-4462, FXC-03.
- 4 sobs-1: DONE (168f7021) - D-4464, FXC-05; tests/panic_hook.rs integration test NOT yet run.
- 5 r53-1 (cli, api mains): DONE (168f7021) - D-4463, FXC-04; Gate 23 OK; pull/telemetry sites not in scope.
- 6 r3-1: DONE (168f7021) - D-4465, FXC-06; tests/sweep_evidence.rs integration test NOT yet run.
- 12 AC-gates-o1-4: DONE (168f7021) - D-4466, FXC-07; tests/crate_graph_claims.rs NOT yet run.
- 15 r64-1: DONE (168f7021 code, 60a78c40 docs) - D-4469, FXC-10.
- 13 W2-cli10-0: WIP - code + docs 60a78c40, gate 11 allowance 0e3eee9a, clippy fix 5cd98c62 (WIP, not validated). Next: `cargo clippy -p cli --all-targets --locked -- -D warnings`, then rerun `a_base_append_scans_once_and_its_bounded_reopen_still_refuses_damage` (it passed before the type-alias move).
- 14 W2-cli16-1: DONE (60a78c40) - D-4468, FXC-09; strict_v6 fold test green.
- 7 AC-whp-o1-1, 8 cli-14/W2-cli12-3/-4, 9 W2-cli6-0, 10 W2-cli2-5, 11 W2-cli14-1/2/3: NOT-STARTED (measurement items; D-4470..D-4474 unused).

Checks so far: cargo fmt --check OK; clippy api OK; clippy cli FAILED once (complex type in the new test) -> fixed in 5cd98c62, not re-run; targeted cli lib tests 209 passed 0 failed (before 5cd98c62); docs gates 10/10b/27/27b OK; Gates 11, 12, 15, 23 OK.
Not yet run: full `cargo test -p cli --locked` (+ setpriv non-root rerun), `cargo test -p api --locked`, the three integration tests above.
