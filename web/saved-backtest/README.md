# Saved backtest inspection

This is a focused, read-only browser view of a separately saved Boolean search.
It uses the existing frontend receipt validators and Rust API readers. It does
not start the full API, recover a pull, start a sweep, or write an ordinary run
ledger. It is not a replacement for the full service's release verification.

The browser opens `/backtest?identity=<search-id>`; optional `pin`, `batch`,
`rung`, `offset` and `setting` select exact saved evidence. A selected setting
must be in the requested 32-row page. Rungs are zero-based and map to the
existing eight intraday timeframes. Older continued pages first resolve the
exact child completion. No browser parameter chooses a directory.

Build the frontend with the already installed frontend toolchain, from `web/`:

```
node node_modules/vite/bin/vite.js build --config saved-backtest/vite.config.js
node --test tests/saved-backtest.test.js
```

The standalone Rust viewer is compiled against the reviewed API/CLI libraries.
The measured build commands, exact library hashes and native test outputs for
the current inspection are retained in
`target/sweep-audit-20260906/page-load-20260907/viewer-verification.md`.
Rebuilding the engine never requires this frontend directory or its tools.

The native launch contract is:

```
viewer STORE WEB LOOPBACK_IP PORT OBSERVATION_BYTES REPLAY_NODES
```

`BRUTEX_STORE`, `BRUTEX_BOOLEAN_OBSERVATION_BYTES` and
`BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES` must explicitly match those arguments.
Both roots must be absolute existing directories and must not overlap.
`WEB` names this frontend's built output. Static assets are loaded once into a
bounded memory snapshot, so an explicit viewer restart activates changed
assets. `/viewer.json` shows the selected root, build identity, bounds and
read-only capabilities. Unknown routes, control methods and cross-origin reads
refuse rather than falling through to the live service.

The table keeps training and later periods separate, preserves refusal reasons,
and pages actual trade observations and zero-trade sessions. Returns exclude
costs; repeated observations across settings are not unique market executions.
The page does not reauthenticate current raw-market files or certify statistical
admission merely because a trade page can be read.
