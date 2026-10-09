# fxw: web refusals (audit/fx-w, base de932e5b)

Every test was recorded failing on the base (scratchpad/fxw/*-base.txt), then fixed and green.

| item | verdict | decision/inv | test | commit |
|---|---|---|---|---|
| W1 swept "0 of N" | FIXED | D-3211/OBSV-12 | swept-surface | 55075d2e |
| W2 census/master stamp | FIXED | D-3212/13 | catalogue-loader | 24a877bb |
| W3 frontier | FIXED | D-3213/14 | frontier-pages | 5d58116b |
| W4 /live.json | FIXED | D-3214/15 | live-top-binding | 0b50e7c8 |
| W5 sweep-evidence | FIXED | D-3215/16 | sweep-evidence | 1290586a |
| W6 running.why | FIXED | D-3216/17 | refusal | 301fc233 |
| W7 launch config | FIXED | D-3217/18 | boolean-launch-page | e8c86d88 |
| F1 audit envelope | FIXED | D-3218/19 | detail-refusal | 882b46b6 |
| F2 backtest readers | FIXED | D-3220/22 | backtest-refusals | f897ac75 |
| F3 undated heap / null generation as 0 | FIXED | D-3219/20,21 | live-top-binding, audit-pages | caeee072 |
| F4 13 readers | FIXED | D-3221/23 | plain-refusals | 14475f66 |
| F5 command replies | FIXED | D-3222/24 | refusal + launch tests | 4e8c0059 |
| F6 ingest 0% with no baseline | FIXED | D-3223/25 | ingest-progress | 0e9dde5d |
| F7 ingest Stop | FIXED | D-3224/26 | ingest-errors | 92329269 |
| ingest /pull/run catch says "nothing written" after a timeout | NEEDS-OWNER | none | none | none |
| `handler_completed:false` kept unknown | NEEDS-OWNER (policy) | none | none | none |

Other commits:
- 37deed9e: test harness and pin follow-ups.
- e2ce3d1e: docs/11-findings.md narrative.
- edd83f69: web/build.

**Mutants.** cargo-mutants was not run. Each changed function was checked by reasoning, and some mutants were run by hand. A replaced return value, a deleted reason, a flipped boundary (60s, 4096) or a `??`/`||` swap each fails a test. Two survivors are equivalent:
- the `break` in fetchTrades;
- typeof checks that come before `isFinite`/`isSafeInteger`.

**Checks:**
- `node --test web/tests/*.test.js`:
  - Run 2: 922 pass, 0 fail, 3 cancelled. The 3 are the Node-22 cancellations in ask.test.js.
  - Run 1: 1 failure in calendar-loader. It uses a 10ms timer, it is not in my diff, and it passed 3 of 3 runs alone. I read it as a timing flake under parallel load.
- `npm --prefix web run check`: 0 errors, 0 warnings.
- Build: exit 0 and committed. A rebuild was identical, and Gate W1 passed.
- gates_jobs self-tests: 26 pass.
- Gates W3, W4, W5 and W6: OK.
- gates-doc.sh: gates 27, 27b, 10b and 10 OK (rc 0).
