# Research policy guide

`index.html` is a standalone searchable explanation of all37 additional
admission settings. It shows the supplied research profile, not evaluated
strategy results or live service configuration.

Canonical prose: `docs/research-policy/report-source.md`.
Runtime values: `config/intraday-research-v1.toml`.

From `web/`, run `node policy-guide/build-guide.mjs` to regenerate. The `--check`
mode refuses missing/duplicate fields, unknown versions, human limits differing
from exact configuration, or a stale generated page. Standard frontend build,
test and type-check scripts run that check automatically. No Rust crate invokes
or depends on this frontend generator.

Use `cli policy-check FILE MAX_POINTS` to inspect a selected file with the
authoritative Rust parser and runner policy validation. It intentionally ignores
per-gate overrides; a real run's configuration report names applied overrides.
Full details, sources, limitations and edge cases are in the canonical document.
