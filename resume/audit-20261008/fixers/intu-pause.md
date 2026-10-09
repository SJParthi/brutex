# intu pause — worktree /home/claude/wt-u, branch audit/int-u (base a7a27dc3, head 4e1e6a08)

| item | state | decision | commit |
|---|---|---|---|
| 1 api clippy (lib + lib test) | DONE: `cargo clippy -p api --all-targets --locked -D warnings` clean; full api lib suite green as uid 65534 (incl. 2 stale fxb1 source-shape tests repaired) | D-4470 | bcd2ebd3 |
| 2 cli -> cost model V3 | DONE: 12 switched modules' lib tests green as uid 65534; V3 two-rung digests pinned | D-4471 | 4e1e6a08 |
| 3 telemetry::sync() at clean exit | NOT-STARTED | (D-4472 planned) | - |
| 4 tell -> telemetry::stderr_line, Gate 23 | NOT-STARTED | (D-4473 planned, amend D-4463) | - |
| 5 ⟨a14_*⟩ placeholders, docs/06-limits.md:12624-12625 | NOT-STARTED | D-4440 (fxb1) | - |
| 6 W2-cli10-0 / 0e3eee9a review | NOT-STARTED | - | - |
| 7 final validation | NOT-STARTED | - | - |

Resumer notes:
- No half-edited files; tree clean. No cargo process of mine running.
- Not yet run for item 2: `cargo clippy -p cli --all-targets --locked -- -D warnings`.
- Item 3 plan: in api main and cli main call `telemetry::sync()` after the exit event/`deliver`.
- Item 4: `stderr_line` is a strict superset of `tell` (counts and logs a refused line); route the 7 sites (api main 2, cli lib 3, operation_audit 2) through it and drop `api/src/main.rs:handle:1` and `cli/src/lib.rs:handle:1` from Gate 23's `declared_handles`.
- Item 5 next command: build api tests, then `<api test bin> --ignored --nocapture latency_qualified_campaign_by_history_length` (binary at /home/claude/t-u/debug/deps/api-e5dd81936677b662, run from crates/api as uid 65534).
- Targets: CARGO_TARGET_DIR=/home/claude/t-u, nobody TMPDIR=/home/claude/t-u/tmp.
