# Cloud build sessions for the Fibonacci F&O parts (Tue 3 Nov 2026)

The operator's Mac is the bottleneck: 8 build slots, 14 cores, and part steps queue 3 to 8 hours for a slot. These cloud sessions build pure-code parts that need no data store, so the Mac only checks and lands them. The operator asked for cloud use and full automation on 3 Nov 2026.

## What you have

- This branch, `cloud/fib-kit-20261103b`, is `fix/fib-fno-seam` at `1ad6e24ef8e0` (FNO-SEAM, passed) plus this folder.
- `RULES-MAC.md` holds the exact rules every Mac builder gets (sections PRE, R2 and KEEP). Follow them, with the adaptations below.
- `PARTS.md` (your part's section), `parts.json.md` (your part's JSON), `SPEC.md`, `COSTS.md`, `costs-table.json.md`, and the small evidence files the rules cite: `universe.json.md`, `fno218.txt.md`, `fno-data-fix-verify.md`, `strategy-mvp-REPORT.md`.

## Adaptations for the cloud

1. **Paths.** `RULES-MAC.md` names Mac paths under `/Volumes/WD_BLACK/...`. Read `${W}/wt/DBFIX` and `${W}/wt/FIB-<PART>` as this checkout, and `${O}` (`state/fib-strategy-20261030`) as this folder. Evidence files the rules cite and this folder does not hold are on the Mac only: say so in your report and work from the spec.
2. **No store, no app.** There is no WD_BLACK drive and no `:8080` here. A step or test that needs them is out of scope for a cloud build: stop that step, name it in the report, and go on with the rest.
3. **Branch.** Work on a new branch `cloud/fib-<part>-20261103` (lower-case part name) cut from this branch. Commit and `git push -u origin` that branch after every meaningful step, because the account running you may end at any time and only pushed work survives. Never push any other branch, never merge, never open a pull request, never touch `main`, `feat/db-fix` or any `fix/*` branch.
4. **The bar is unchanged.** Every extreme case and permutation the design names, 100% line and branch coverage of the touched set measured against `1ad6e24ef8e0`, `cargo fmt --check`, `clippy -D warnings` on the touched crates, the tests, and the benches for hot paths. Use the repository's pinned toolchain. Coverage: the Mac's Route B is `cargo test` under the pinned nightly with `RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch'`, `CARGO_PROFILE_DEV_OPT_LEVEL=0`, then `llvm-cov` over the touched set. If you can install that nightly here, measure it and report the figures; if you cannot, report coverage UNMEASURED and why. Either way the Mac's independent check re-measures by Route B before anything counts, and benches measured here are advisory.
5. **Report.** Write `reports/fib-cloud-20261103/report-<PART>.md` on your branch: what you built, files and functions, tests by name, gates and coverage with figures or UNMEASURED, benches, decisions drafted (from your part's D block only), anything out of scope. Also write the part's `part-<PART>.md` section as `RULES-MAC.md` asks of a Mac builder. Push it.
6. **Never** the Py-language (never use it or write its name), never a credential, never a deletion of anything you did not create in this session.

Every JSON and text file the rules name is here as Markdown with the same name plus `.md` (`parts.json.md`, `costs-table.json.md`, `universe.json.md`, `fno218.txt.md`), because the repository tracks `.json` only under `.claude/` (CLAUDE.md section 2). Read the fenced block inside each.
