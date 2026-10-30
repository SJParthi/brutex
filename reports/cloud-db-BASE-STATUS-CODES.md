STATUS: IN PROGRESS

# Cloud build report: BASE-STATUS-CODES (30 Oct 2026)

Part: `BASE-STATUS-CODES` — one status rule through one typed classifier: a client's malformed input is
400, a key that names nothing is one not-found answer on every route, 503 only for what exists but cannot
be read (assurance 29 Oct n=31).

- Branch: `cloud/db-BASE-STATUS-CODES-20261030`, cut from `origin/feat/db-fix` at
  `f01d7a0e430e8f9b01e424a299b0bed20738a0ef` (Merge R4-26), the newest head on 30 Oct 12:43 IST.
- Specification: `origin/cloud/db-base-20261030:reports/db-specs-20261030/BASE-STATUS-CODES.md`, read in
  full with `CLAUDE.md` (from `origin/feat/db-fix`), `README.md` and `CONTEXT.md` of that folder.
- Prerequisites: none (the part file, section 1). FILES_ONLY links: none.
- D1: the merge-back waits for `Merge BASE-REDS-1023:` (not on `feat/db-fix` at `f01d7a0e4`: `grep -c` = 0).
- Decision block: D-7550..D-7559 (D-7550 is this part's entry).
- Host: 4 cores, running as root (uid 0), rustc 1.97.1 (the repository's pin), Node 22.22.0, npm 10.9.4.

## Log

### 2026-10-30 12:43 IST

- Waited on the switches from 10:28 IST with one blocking 9-minute poll at a time (README step 1); the
  test printed `OPEN 12:43:15 IST` (HOLD removed by `b9e9ed39`, GO and GO-BASE-STATUS-CODES present).
- Branch cut from `origin/feat/db-fix` `f01d7a0e4`.
- NEXT: read the code the part owns (`sources.rs` `Refusal`, the seven identity routes' refusal paths,
  `server.rs` `route_table`), then write `status_rule.rs` and `status_rule_tests.rs` test first.

### 2026-10-30 13:35 IST

- Done (commits `6829bded`, `9c9d3c52`, `a635684a`, all pushed):
  - `crates/api/src/status_rule.rs` (new): `Class` (400/404/410/416/422/429/503), `Rule`/`Malformed`
    (the store's own `check_segment` for a symbol; 64 lowercase hex, never all zeros, for an
    identity), `probe`/`probe_identity`/`absent` (one `symlink_metadata`, never a listing),
    `Fault::identity_body` (one body), `serve`/`page` (one path for every identity route of the part).
  - `sources.rs`: `Refusal`'s constructors take the table's status; `Refusal::store` no longer reads
    its sentence for a `410`; `Instrument::parse` checks `symbol=` by the store's rule (400 before
    the store) — a function outside the FILES line, see "Files outside ownership".
  - The nine identity routes of the seven owned files answer through `serve`/`page`: probe before
    any reader, typed absences as the one 404, frontier's store faults 400 -> 503, sweep-evidence's
    page past the extent 503 -> 416 (D-7551).
  - Web readers: `detail-refusal.js` (`REFUSAL_STATUSES`, `notFoundWhy`), `sweep-evidence.js`,
    `expression-search.js`, `candidate-trades.js`, `frontier-pages.js`; tests moved in
    `web/tests/sweep-evidence.test.js` and `frontier-pages.test.js`, new
    `web/tests/status-rule-readers.test.js` (6 tests: red 6/6 on the base readers, green 6/6 here).
  - Ledgers: docs/05 D-7550, D-7551; docs/04 STATUS-01..12.
- Slip, recorded plainly: at 13:1x IST one shell command started the Py-language interpreter by
  mistake (a typed command, not a script); it read nothing, ran no code and wrote no file, and was
  stopped at once (`TaskStop`). No file of this branch names or uses the language.
- NEXT: run the api tests of the touched modules (in progress), fill the table's outside-route
  answers from the measured run, write docs/06 L-STATUS-01..03 with the measured timings, then the
  full gates.

### 2026-10-30 13:50 IST

- Switches checked at 13:41 IST: `HOLD` absent, `GO` and `GO-BASE-STATUS-CODES` present;
  `origin/feat/db-fix` still `f01d7a0e4`.
- The first full `npm test` (1,802 tests) ended `fail 15, cancelled 3`. Two were this part's: the
  old 200 "missing" cases of `web/tests/candidate-trades.test.js` and `expression-search.test.js`,
  moved to the 404 answer (both readers' own tests). `detail-refusal.js` now quotes all seven class
  words, read by the reader test from `Class::word` in `status_rule.rs` (never typed twice).
  `tests/ask.test.js` (3 cancelled) is red on the unmodified base too (shown below under Gates);
  the 12 browser-harness tests are being compared with the base the same way.
- `frontierjson.rs`'s unknown-identity test now asserts the one 404; the table test prints what
  every outside route answers so its expectation is copied from a run.
