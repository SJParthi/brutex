# CI workflow commentary

Moved from `.github/workflows/ci.yml` to keep the workflow below the GitHub Actions
500 KB file limit. Jobs, commands, triggers, and dependencies are unchanged.

Source: [GitHub Actions limits](https://docs.github.com/en/actions/reference/limits#workflow-file-size).

The notes below retain their original order and line positions from commit `19187a4ebadbb2573547c91d49badb3f83019162`.

```yaml
# Original line 7
  # TWO EVENTS THAT ARE NOT A COMMIT, AND WHY A GATE SET NEEDS THEM.
# Original line 8
  #
# Original line 9
  # Every gate in this file was only ever executed by a push or a pull request,
# Original line 10
  # so `main` was validated exactly once -- at the instant it was written -- and
# Original line 11
  # never again. That is not the same as `main` being green, because half of
# Original line 12
  # what these gates read is not in the repository:
# Original line 13
  #
# Original line 14
  #   * `dtolnay/rust-toolchain@stable` resolves to whatever stable is TODAY, and
# Original line 15
  #     gate 6b runs clippy with `-D warnings`. A new lint turns a green tree red
# Original line 16
  #     with no commit.
# Original line 17
  #   * gate 3 runs `cargo deny check`, whose verdict is the RustSec advisory
# Original line 18
  #     database at fetch time. A disclosure published tomorrow is a real failure
# Original line 19
  #     of a tree nobody touched, and it is the one this repository most wants to
# Original line 20
  #     hear about early.
# Original line 21
  #   * `ubuntu-24.04` is an image that is rebuilt; gate 8's ratio and gate 18's
# Original line 22
  #     baseline are timings taken on it.
# Original line 23
  #
# Original line 24
  # `workflow_dispatch` is the manual half: an operator can ask the question
# Original line 25
  # "is main green right now" without inventing a commit to ask it with, which is
# Original line 26
  # what an empty commit on a shared tree amounts to.
# Original line 27
  #
# Original line 28
  # `schedule` is the unattended half. Weekly rather than daily, DELIBERATELY:
# Original line 29
  # gate 18 installs cargo-mutants from source and gate 8 runs the bench suite,
# Original line 30
  # so a run is expensive, and a cadence nobody reads is a cadence that gets
# Original line 31
  # switched off. Monday morning is before the week's work rather than after it.
# Original line 32
  #
# Original line 33
  # SCHEDULED RUNS ONLY EVER FIRE ON THE DEFAULT BRANCH. GitHub takes the cron
# Original line 34
  # from the workflow file on `main`, so this line does nothing at all until it
# Original line 35
  # is merged there -- said out loud because a cron that has never fired reads
# Original line 36
  # identically to one that is broken.
# Original line 39
    # 03:17 UTC Monday. Not on the hour: GitHub queues cron runs and the ones on
# Original line 40
    # the hour queue longest, which for a job set this long is real delay.
# Original line 44
  # THE EVENT NAME IS PART OF THE GROUP, AND IT HAS TO BE NOW THAT THERE ARE FOUR.
# Original line 45
  #
# Original line 46
  # `cancel-in-progress` cancels the older run whenever a newer one joins the
# Original line 47
  # group. Before the two events above, the group was one push-or-PR series per
# Original line 48
  # ref and cancelling the older member of it is exactly right. With a cron and a
# Original line 49
  # manual button added, `github.ref` for all three of push-to-main, the weekly
# Original line 50
  # run and a dispatch is `refs/heads/main` -- so a Monday 03:17 run landing on
# Original line 51
  # top of a push to main would CANCEL the push's validation, and the commit
# Original line 52
  # would carry a cancelled check for a reason that has nothing to do with it.
# Original line 53
  # Splitting by event keeps the "newest wins" behaviour inside each series and
# Original line 54
  # never across them.
# Original line 63
  # ==========================================================================
# Original line 64
  # GATE 1 + 2 — the load-bearing gates. No toolchain, no cache, seconds.
# Original line 65
  # These run first so a violation fails before anything expensive starts.
# Original line 66
  # ==========================================================================
# Original line 71
      # `fetch-depth: 0`, AND IT IS GATE 1E THAT NEEDS IT. This job used the
# Original line 72
      # default shallow clone for its whole life, which was fine while every
# Original line 73
      # step here read `git ls-files`. Gate 1e now runs `cargo test` under a
# Original line 74
      # shadowed PATH, and `crates/core/tests/findings.rs` asserts
# Original line 75
      # `rev-parse --is-shallow-repository` is not "true" — it refuses rather
# Original line 76
      # than pass vacuously, because on a shallow clone it cannot tell "this
# Original line 77
      # commit hash is wrong" from "this clone has no history". Without this
# Original line 78
      # line gate 1e would go red for a reason that has nothing to do with §2,
# Original line 79
      # which is the worst shape of failure: red for the wrong reason trains a
# Original line 80
      # reader to ignore the gate. The `build`, `coverage` and `mutants` jobs
# Original line 81
      # already check out this way and say so for the same reason.
# Original line 2912
  # ==========================================================================
# Original line 2913
  # GATES 3-6 — the toolchain gates.
# Original line 2914
  # ==========================================================================
# Original line 7458
      # `fetch-depth: 0`, BECAUSE TWO TESTS READ GIT HISTORY.
# Original line 7459
      #
# Original line 7460
      # `crates/core/tests/findings.rs` resolves the commit hashes named in the
# Original line 7461
      # findings ledger and checks each is real and on this branch. On the
# Original line 7462
      # default SHALLOW clone no commit resolves, so the tests cannot tell "this
# Original line 7463
      # hash is wrong" from "this clone has no history" — and they refuse rather
# Original line 7464
      # than pass vacuously, which is why CI went red with the fix printed in the
# Original line 7465
      # assertion message. The `mutants` job already checks out this way for the
# Original line 7466
      # same reason.
# Original line 7475
      # Cargo refuses a virtual workspace with no members, so between the
# Original line 7476
      # automation commit and the first crate there is genuinely nothing to
# Original line 7477
      # compile. This probe says so out loud rather than letting a green tick
# Original line 7478
      # imply a build happened. It disarms itself permanently the moment
# Original line 7479
      # crates/core lands, and it is checked against the tracked tree, not the
# Original line 7480
      # manifest, so it cannot be satisfied by an untracked directory.
# Original line 7502
      # `--offline` only succeeds if every crate is already in CARGO_HOME.
# Original line 7503
      # A newly added dependency has never been downloaded on a runner, so
# Original line 7504
      # gate 4 structurally rejected the FIRST dependency the workspace ever
# Original line 7505
      # gained -- it passed until now only because there were none. Fetching
# Original line 7506
      # with --locked first preserves the whole point of the gate (build the
# Original line 7507
      # EXACT locked versions, resolve nothing) while letting it actually run.
# Original line 7601
  # ==========================================================================
# Original line 7602
  # Coverage — 100% line and branch, no omit list.
# Original line 7603
  # ==========================================================================
# Original line 7609
      # `fetch-depth: 0`, BECAUSE TWO TESTS READ GIT HISTORY.
# Original line 7610
      #
# Original line 7611
      # `crates/core/tests/findings.rs` resolves the commit hashes named in the
# Original line 7612
      # findings ledger and checks each is real and on this branch. On the
# Original line 7613
      # default SHALLOW clone no commit resolves, so the tests cannot tell "this
# Original line 7614
      # hash is wrong" from "this clone has no history" — and they refuse rather
# Original line 7615
      # than pass vacuously, which is why CI went red with the fix printed in the
# Original line 7616
      # assertion message. The `mutants` job already checks out this way for the
# Original line 7617
      # same reason.
# Original line 7822
  # ==========================================================================
# Original line 7823
  # GATE 18 — no surviving mutant on what this change touched.
# Original line 7824
  # ==========================================================================
# Original line 7829
    # THIS JOB HAD NO CLOCK, AND A JOB WITH NO CLOCK DOES NOT FAIL — IT VANISHES.
# Original line 7830
    #
# Original line 7831
    # Without `timeout-minutes` a job inherits GitHub's 6-hour ceiling, and what
# Original line 7832
    # happens at that ceiling is a CANCELLATION, not a failure: the step never
# Original line 7833
    # reports, the log is cut mid-line, and the run page shows a grey square. Two
# Original line 7834
    # runs ended exactly that way having tested ZERO mutants, and nothing
# Original line 7835
    # distinguishes that from a gate that had nothing to do. `ci-ok` does catch
# Original line 7836
    # it -- it refuses any `needs` result that is not `success` -- so the build
# Original line 7837
    # went red, for a reason no reader could name. A gate whose failure mode is
# Original line 7838
    # unreadable teaches people to re-run it rather than read it.
# Original line 7839
    #
# Original line 7840
    # 240 MINUTES, AND THE NUMBER IS DERIVED RATHER THAN PICKED. It has to be
# Original line 7841
    # under the 6-hour ceiling so that THIS clock fires first and GitHub records
# Original line 7842
    # a timeout against this job with the log intact. It has to be over the
# Original line 7843
    # budget the step below computes, or the step's own arithmetic would be a
# Original line 7844
    # lie. The step reserves 40 minutes of it for `cargo install cargo-mutants`
# Original line 7845
    # (built from source; the toolchain cache does not cover `~/.cargo/bin`) and
# Original line 7846
    # for the unmutated baseline, and spends the remaining 200 on mutants. If
# Original line 7847
    # this number changes, JOB_MINUTES in the step must change with it — they are
# Original line 7848
    # two halves of one statement and the step says so.
# Original line 7851
      # THE WHOLE HISTORY, because this gate's scope IS the diff. With the
# Original line 7852
      # default shallow clone there is no merge base to diff against and the
# Original line 7853
      # step would mutate nothing while reporting success — the exact silent
# Original line 7854
      # pass CLAUDE.md §4 calls a fallback that hides a failure.
# Original line 8039
  # ==========================================================================
# Original line 8040
  # GATE 8 — per-operation cost stays constant as input grows.
# Original line 8041
  # ==========================================================================
# Original line 8046
    # THE BENCH BUDGET, WRITTEN DOWN. Gate 14 proves these benches exist and
# Original line 8047
    # have the shape of a ratio measurement; this job is what executes them,
# Original line 8048
    # and a bench is the one step in this file whose cost is chosen by the
# Original line 8049
    # person writing the measurement rather than by the size of the tree. The
# Original line 8050
    # Actions default is 360 minutes — six hours of a pinned runner for a loop
# Original line 8051
    # that will never converge. Each bench takes a minimum over 60 trials and
# Original line 8052
    # crates/core's worst case allocates a 4 MiB field per repetition, so the
# Original line 8053
    # ceiling that matters is set here rather than discovered on a blocked
# Original line 8054
    # queue. A breach of THIS is not a slow ratio, it is a bench that stopped
# Original line 8055
    # terminating, and it should be read that way.
# Original line 8089
  # ==========================================================================
# Original line 8090
  # Aggregate — this is the only required check on the branch rule.
# Original line 8091
  # ==========================================================================
# Original line 8092
  # The job NAME is what a branch-protection rule and the auto-merge workflow
# Original line 8093
  # match on, so it is spelled exactly as the required check: `ci-ok`.
# Original line 8094
  # ==========================================================================
# Original line 8095
  # GATE W — THE BROWSER TREE.
# Original line 8096
  #
# Original line 8097
  # This job was `.github/workflows/web.yml`, on the reasoning that the engine's
# Original line 8098
  # gates and the browser's gates should not be confused for one another. True,
# Original line 8099
  # and beside the point: `needs:` CANNOT NAME A JOB IN ANOTHER WORKFLOW FILE.
# Original line 8100
  # A separate workflow could only have been made blocking by adding a second
# Original line 8101
  # required check in branch protection, which is a repository setting and not a
# Original line 8102
  # file in this tree. `ci-ok` already lists its needs and branch protection
# Original line 8103
  # already requires `ci-ok`, so moving the job here makes Gate W binding with
# Original line 8104
  # no setting to change. The distinction survives in the job's name.
# Original line 8105
  #
# Original line 8106
  # IT INSTALLS NODE, AND THAT DOES NOT WEAKEN GATE 1E. Jobs do not share a
# Original line 8107
  # machine. Gate 1e shadows twelve JS binaries with failing stubs on its own
# Original line 8108
  # runner and proves cargo builds and tests without them there; a different job
# Original line 8109
  # installing Node on a different runner cannot reach it. CLAUDE.md §2 forbids
# Original line 8110
  # a CRATE depending on the front-end toolchain, not a CI file knowing it
# Original line 8111
  # exists.
# Original line 8112
  # ==========================================================================
# Original line 8125
      # `ci` and NOT `install`. `npm install` may rewrite the lockfile, which
# Original line 8126
      # would make the rebuild below a comparison against a dependency set the
# Original line 8127
      # repository never agreed to.
# Original line 8131
      # ----------------------------------------------------------------------
# Original line 8132
      # GATE W1 — THE COMMITTED BUNDLE IS THE ONE THIS SOURCE PRODUCES.
# Original line 8133
      #
# Original line 8134
      # This check is only possible because `svelte.config.js` sets
# Original line 8135
      # `version.name` to a digest of the source rather than `Date.now()`.
# Original line 8136
      # With the default clock version, rebuilding IDENTICAL source rewrote 18
# Original line 8137
      # of 39 tracked files -- the timestamp enters the client manifest, which
# Original line 8138
      # changes that chunk's content hash, which changes every filename that
# Original line 8139
      # references it. A gate that failed on every run for a reason that was
# Original line 8140
      # never the source would train a reader to ignore it.
# Original line 8141
      #
# Original line 8142
      # Source maps are gitignored, so they cannot enter this diff.
# Original line 8143
      # ----------------------------------------------------------------------
# Original line 8163
      # ----------------------------------------------------------------------
# Original line 8164
      # GATE W2 — THE TESTS THAT EXIST ACTUALLY RUN.
# Original line 8165
      #
# Original line 8166
      # node's own runner, so this needs no dependency beyond the toolchain
# Original line 8167
      # already installed above.
# Original line 8168
      # ----------------------------------------------------------------------
# Original line 8172
      # ----------------------------------------------------------------------
# Original line 8173
      # GATE W3 — TYPES DO NOT GET WORSE.
# Original line 8174
      #
# Original line 8175
      # IT IS A FLOOR AT ZERO NOW, AND IT ARRIVED THERE THE WAY A RATCHET IS
# Original line 8176
      # MEANT TO. It began as a ratchet at 64, and this comment used to explain
# Original line 8177
      # why: demanding zero would have meant either fixing 64 errors in the
# Original line 8178
      # commit that added the gate -- unrelated work, badly scoped -- or
# Original line 8179
      # shipping a gate that was red from birth, which is a gate nobody reads.
# Original line 8180
      # "No worse than today" was enforceable then and got stricter each time
# Original line 8181
      # somebody lowered the number: 64, then 61, and now 0.
# Original line 8182
      #
# Original line 8183
      # THE LAST 42 CAME OUT IN ONE PASS BECAUSE THEY WERE FOUR ROOTS, NOT 42
# Original line 8184
      # FAULTS. `live`, `board`, `tradeList` and `combos` each declared
# Original line 8185
      # `rungs: []` / `groups: []` / `rows: []` inside `$state(...)` with no
# Original line 8186
      # type; TypeScript inferred `never[]`, and every downstream read of a
# Original line 8187
      # property became its own error. One untyped `span(pick)` accounted for
# Original line 8188
      # ten more the same way. That is the shape of this whole class -- the
# Original line 8189
      # error is never reported at the line that causes it -- and it is exactly
# Original line 8190
      # why a ratchet that only ever counts is worth keeping.
# Original line 8191
      #
# Original line 8192
      # A rise is now a refusal. If a legitimate change genuinely cannot reach
# Original line 8193
      # zero, RAISE THIS NUMBER IN THIS FILE and say why in
# Original line 8194
      # `docs/06-limits.md` -- do not delete the gate, and do not silence the
# Original line 8195
      # file it is about.
# Original line 8196
      # ----------------------------------------------------------------------
# Original line 8245
      # ----------------------------------------------------------------------
# Original line 8246
      # GATE W4 — ORPHANED CSS DOES NOT GROW.
# Original line 8247
      #
# Original line 8248
      # The Svelte compiler already finds this and already prints it; nothing
# Original line 8249
      # read it. 86 distinct selectors are styled in `db` and `ingest` for
# Original line 8250
      # markup that no longer exists — a whole inline calendar superseded by
# Original line 8251
      # `DayField`, an entire earlier generation of the view bar (`.viewbar`,
# Original line 8252
      # `.vtab`, `.vbudget`, `.vsum`) superseded by `.segs`/`.seg`, and a month
# Original line 8253
      # toggle. Each is a rule whose markup was deleted while the styling, and
# Original line 8254
      # usually the reasoning in the comment beside it, stayed behind.
# Original line 8255
      #
# Original line 8256
      # That is not cosmetic. It is the measurable signature of the pattern
# Original line 8257
      # that produced the worst findings on this page: safeguards that were
# Original line 8258
      # written, argued for in a comment, and then orphaned by a later markup
# Original line 8259
      # deletion, leaving the comment still asserting a guarantee the code no
# Original line 8260
      # longer delivers. `.seg` is the proof it cuts both ways — the orphaned
# Original line 8261
      # CSS is what let the removed view toggle be RESTORED to its original
# Original line 8262
      # shape rather than guessed at.
# Original line 8263
      #
# Original line 8264
      # IT IS NOW A FLOOR AT ZERO, and it arrived there the way a ratchet is
# Original line 8265
      # supposed to: 86 -> 35 -> 33 -> 10 -> 0, each step a set of rules the
# Original line 8266
      # compiler had already proved could not match. From here the gate refuses
# Original line 8267
      # the FIRST orphaned selector rather than the eighty-seventh.
# Original line 8268
      #
# Original line 8269
      # It began as a ratchet, for the same reason W3 is one: a floor of zero
# Original line 8270
      # would mean clearing every one inside the commit that adds the gate.
# Original line 8271
      # The ceiling has come down from 86 to 35 as the provably-dead rules were
# Original line 8272
      # removed; what remains needs a judgement per selector, because each has
# Original line 8273
      # a class token that DOES appear in the markup and only the combination
# Original line 8274
      # is unmatchable. This gate keeps the number from climbing while that
# Original line 8275
      # waits, and prints the new ceiling whenever it falls.
# Original line 8276
      #
# Original line 8277
      # Counted DISTINCT, because the client and server passes each report the
# Original line 8278
      # same selector once.
# Original line 8279
      # ----------------------------------------------------------------------
# Original line 8280
      # ----------------------------------------------------------------------
# Original line 8281
      # GATE W5 — A COMMENT DOES NOT SWALLOW THE RULE BENEATH IT.
# Original line 8282
      #
# Original line 8283
      # Three times in this tree a comment's closing `*/` and the selector line
# Original line 8284
      # under it were deleted together, fusing the two:
# Original line 8285
      #
# Original line 8286
      #     /* The hint sits INSIDE the input's right padding{
# Original line 8287
      #       margin-left: calc(-1 * var(--s6));
# Original line 8288
      #
# Original line 8289
      # The comment then runs to the next `*/` further down, so those
# Original line 8290
      # declarations -- and every rule and comment between -- sit inside a
# Original line 8291
      # comment. THE CSS STILL PARSES AND THE BUILD STAYS GREEN. The styling
# Original line 8292
      # simply stops applying, `svelte-check` cannot see it, and Gate W4 cannot
# Original line 8293
      # count it, because to every tool involved the text is prose. 36 lines in
# Original line 8294
      # /db and a whole `.foothold` block in /ingest were inert this way.
# Original line 8295
      #
# Original line 8296
      # The shape test is that SOME LINE OF THE COMMENT BODY ends in a brace,
# Original line 8297
      # and it used to be that the comment's FIRST line did. That narrower
# Original line 8298
      # reading was measured and it misses the common case: the fusion happens
# Original line 8299
      # wherever the `*/` was deleted, and a comment three prose lines long
# Original line 8300
      # loses its closer on the THIRD line, not the first --
# Original line 8301
      #
# Original line 8302
      #     /* THE DAY AND MONTH FIELDS TAKE /INGEST BOX, for the same reason
# Original line 8303
      #        ... two more lines of prose ...
# Original line 8304
      #        a focus underline for the whole group{
# Original line 8305
      #
# Original line 8306
      # -- so the first line ends in a word and the gate walked past a rule
# Original line 8307
      # that had been inert ever since. Widening it to any line found four such
# Original line 8308
      # fusions in `web/src/routes/db/+page.svelte` on the day it was widened
# Original line 8309
      # and NOTHING in the other twelve files, which is the discrimination the
# Original line 8310
      # narrow test was claiming and did not have. A comment that merely
# Original line 8311
      # CONTAINS a brace still cannot fool it: the test is that a line ENDS in
# Original line 8312
      # one, which is what a selector does and what prose about a brace does
# Original line 8313
      # not. Counting every `{` instead was measured too and it is not usable
# Original line 8314
      # -- it fires on a dozen comments across four files that quote a brace
# Original line 8315
      # mid-sentence.
# Original line 8316
      #
# Original line 8317
      # The `/*` against `*/` count below is printed and is NOT a failure, and
# Original line 8318
      # it says so in the line it prints. It cannot be one: several of the `/*`
# Original line 8319
      # in `/db` sit inside other comments, where CSS gives them no meaning, so
# Original line 8320
      # that file is short of balanced with zero unterminated comments. No
# Original line 8321
      # number is pinned here on purpose -- `web/` is edited by its own lane
# Original line 8322
      # and the counts move under this file. The scan above is the check; the
# Original line 8323
      # count is context printed beside it.
# Original line 8324
      # ----------------------------------------------------------------------
```

## Mutation execution update — 20 September 2026

D-0629 replaces the serial preflight described in the historical notes above.
The complete changed-line enumeration is partitioned, not sampled. The planner
and each bounded job independently verify its round-robin assignment, and the
outcome checker refuses missing, duplicate, foreign, surviving and timed-out
cases. Each job uses two isolated workers and at most 200 cases; at most eight
jobs run together. A matrix requiring more than 256 jobs refuses before work.

The same 240-minute deadline, 40-minute reserve and estimated 120 seconds per
case now describe a two-worker job. These are admission arithmetic, not a
throughput measurement. Each nonzero command exit still refuses. The ordinary
workspace test job and coverage thresholds remain unchanged. The final ci-ok
job requires both the planner and all mutation jobs to succeed.

GitHub documents the [256-job matrix limit](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idstrategymatrix)
and [standard runner billing](https://docs.github.com/en/actions/concepts/billing-and-usage).
This public repository uses standard ubuntu-24.04 runners, with one-day
retention for diagnostic assignment and outcome lists. No paid larger runner
is introduced.
