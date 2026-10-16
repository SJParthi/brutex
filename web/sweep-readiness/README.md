# Repeat the sweep readiness checks

From the repository root:

```sh
rustc --edition=2024 -D warnings web/sweep-readiness/verify.rs -o target/sweep-readiness-verify
target/sweep-readiness-verify
```

Open `web/sweep-readiness/index.html` in a browser. It works locally without a
server or a front-end build. The automated check runner is Rust; Node is used
only for the browser tests, from `web/`.

The verifier executes the named checks, saves stdout/stderr and exit codes under
`target/sweep-readiness/<unique-run>/`, and publishes the latest `evidence.js`
beside the page. It returns nonzero if any check fails, cannot start, or relevant
source differences are detected at a checkpoint. Every completed run retains its own evidence
copy. The latest file is a convenience view, not an append-only market ledger.

Use `target/sweep-readiness-verify --focused` for the sweep tests, result
regressions, stored expression/search/lifecycle/API evidence checks, checkpoint
continuation, exact candidate trades, Selection V6/Global Replay V4, sweep lint
checks, dependency policy, all frontend regression files, browser type checks
and the production frontend build. The frontend test glob includes saved-trade
joins, exact invocation IDs, failed-page retries and new tests added later; a
hand-picked list cannot silently omit them.
That named subset excludes the three full-workspace checks and does not clear
their failures. Any other argument is refused. The language and extension
boundary outside `web/` is checked by one checker, CI gate 1
(`.github/gates_tree.rs gate-1`, with the scanner it runs); this verifier's
former `--purity` mode was a second, weaker copy of that rule and was removed
(D-2325).

CI gate 6d builds this file with `--test` and runs its tests, which compiles
the audit but does not start it, and builds and runs `probes/support_lanes.rs`
and every other native root under `web/` it finds (D-2324).

The full run executes each broad gate once: comparison consistency, workspace
formatting, all workspace tests, all-target workspace Clippy, dependency policy,
all browser regression files, browser type checks and production build. Its workspace test command
already includes every focused Rust target, so those diagnostic subsets are not
repeated. This avoids redundant provenance rebuilds while retaining their tests.

This is a finite audit workflow, not a recurring background monitor. It contacts
no broker and starts no production sweep. Tests may read optional local fixtures,
including an existing lake-directory census. `cargo deny`
may contact the dependency-advisory service. An interrupted verification remains
marked incomplete. Logs are local ignored build artifacts; save the entire
evidence directory when sharing a report outside this checkout.

The historical rows are generated from the dated human-reviewed snapshot in
`docs/14-sweep-readiness-20260906.md`; the current integration table comes from
`docs/31-backtest-integration-20260908.md`. Unsupported statuses, missing fields
or incomplete tables refuse generation. After reviewing those reports, run
`node sweep-readiness/build-review.mjs` from `web/` to refresh the table. The
verifier checks that both the launch-decision table and detailed rows agree.
Per-row local evidence links are preserved; missing artifacts still require
their actual saved directory. This frontend-only projection does not build
or run the Rust backend. Rerunning tests refreshes the check panel; it does not
automatically re-audit the report's source findings or update its dated claims.

The FNV fingerprint samples relevant source and check configuration before and
after checks; a detected difference remains a failure even if files later revert.
Changes that occur and revert between checkpoints are not detected. It is not a
cryptographic attestation, the engine's RunId, or proof against changes after
the verifier exits. Full coverage, mutation results, benchmark measurements,
real-data provenance and deployed-UI verification remain separate evidence.

`probes/support_lanes.rs` preserves the two-bar boundary that previously caused
a capacity-overflow panic. It now requires a successful answer identical to one
worker. The same regression is part of the engine test suite; historical failure
logs remain archived separately.
