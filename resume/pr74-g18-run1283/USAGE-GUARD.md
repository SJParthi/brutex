# Usage guard (owner request 2026-10-06 ~03:55 UTC: "take care of usage, pause, hold, resume, continue")

## Readings and calibration
- Owner's screenshot (~03:50 UTC, plan Max 20x): 5-hour window 38% (resets 07:10 UTC = rate_limit_info.resetsAt 1791270600), weekly all models 10% (resets "Tue 4:30 AM" owner local time; if IST that is Mon 2026-10-12 23:00 UTC: INFERRED, not confirmed), weekly Fable 0%.
- No usage meter is readable from a cloud container (no OAuth token; provider managed by host). Estimator: sum of `external_metadata.usage.cost_usd` over every brutex-20261006 session + the coordinator, from get_session.
- Calibration at 03:52 UTC: sum $188.23 (14 sessions, all created inside this window) = 38% -> about $4.95 per 5-hour percent; weekly about $18.8 per percent (assumes this week's usage before 02:10 UTC was small; UNVERIFIED).
- Burn with 13 sessions: about $4.7/min ($280/h). Other sessions of the account (Mac GDFL, Tickvault) are invisible to the estimator, so thresholds are set conservatively.

- Second reading (owner screenshot ~03:53 UTC): 5-hour 41%, weekly 11%. Recalibrated: about $4.6 per 5-hour percent; 13 sessions burn about 1% of the 5-hour window per minute, and about 1 weekly percent per 3 five-hour percent.

## Thresholds (estimated %, conservative)
| Signal | Action |
|---|---|
| 5-hour >= 70% estimated, or any session's rate_limit_info.status != "allowed" | PAUSE all sessions (message below, priority now); wrap-up adds about 5-10%, so it lands near 80-85%; coordinator saves state; RESUME at reset + 2 min |
| 5-hour reset (07:10, 12:10, 17:10, 22:10 UTC ...) | snapshot every session's cost as the new baseline; RESUME the sessions the weekly tier allows |
| weekly < 60% | all 13 sessions run |
| weekly 60-80% | attack lenses and zero-findings save and stop until the weekly reset; PR #74 work and WS2-WS4 hand-overs continue |
| weekly 80-93% | only PR #74 work (Gate 18 fixes, integration, CI follow-ups); a reserve is kept for the next CI result (about 20 h after each push) |
| weekly >= 93% | PAUSE everything until the weekly reset |

## PAUSE message (send with priority now)
PAUSE (usage guard, from the coordinator): the account's usage limit is nearly reached. Within your next few tool calls: (1) do not arm monitors, wakeups or send_later; let a build or mutation run finish only if it is nearly done, else stop it; (2) commit everything on YOUR OWN branch (unvalidated work as one commit titled "WIP (paused, not validated)") and push it; (3) write resume/<your folder>/PAUSE-20261006.md on fix-queue: head SHA, done, in progress, exact next steps, commands to restart; (4) end your turn. If a background job wakes you during the pause, write its result into the PAUSE file in one tool call and end the turn. Start nothing new until a message that begins with RESUME arrives.

## RESUME message
RESUME (usage guard): the usage window has reset. Read your PAUSE-20261006.md, and continue your task from its next steps under the same rules. Owner rule (08:10 UTC): use ONLY Opus 5.5 (claude-opus-5-5) for every agent and subagent. Never pass a haiku or sonnet model override; omit the model so it inherits Opus 5.5.

## Poll log
| UTC | Sum of session cost_usd | Estimate (cost / 4.6) | Note |
|---|---|---|---|
| 03:52 | $188.23 | 41% (owner meter 03:53: 41%) | calibration point |
| 04:06 | $211.36 | ~46% | burn fell to ~$1.7/min while most sessions wait on builds and mutation runs. cost_usd lags inside long turns (running subagents are counted only when they return), so the pause decision takes the HIGHER of this estimate and a time-based one from the last owner reading. |
| 04:23 | $255.79 (summed by hand; the haiku poller's own total, $274.89, was wrong) | ~56% (time-based also ~56%) | no session warning; below the 70% line; next poll 04:38 UTC (trig_01Xn24XAxZCiXtRN85SiWhyA); weekly ~16% |
| 04:44 | $312.53 (summed by hand) | ~68% (+ unreported subagent spend, so >= 70%) | PAUSE sent to all 13 sessions, priority now; resume trigger 07:12 UTC (trig_01UoWW9ugvVA2FCgxT1nT1gM) |

## RECALIBRATION 2026-10-06 06:05 UTC (owner screenshot: 5-hour 99%, weekly 27%)
- The cost_usd sum UNDERCOUNTED the meter by about 30%: $349.26 summed vs 99% shown -> about **$3.5 per 5-hour percent** (not $4.6). The coordinator's own context (713k tokens, re-read every step) is a large share. The 04:45 pause landed far later than intended.
- New rule for every window: baseline = sum at the window start; estimate = (sum - baseline) / 3.5; **PAUSE at 60% estimated**. Owner readings override the estimate whenever given.
- Window 07:10-12:10: baseline $349.26 (all sessions idle 04:45-07:13). Only the PR #74 path runs: g18 cli-a, cli-b, api, runner, rest, WS2 (hand-over only, no new attack rounds), WS3 (batch3 only), WS4 (batch3). WS5 and lenses L1-L4 stay PAUSED until the weekly estimate shows room (weekly 27% at 06:05; about 16% weekly per full window).
- Coordinator keeps its own steps few; no chatty monitor (each wake re-reads its whole context); polls by send_later.

## WEEKLY GUARD (added 07:35 UTC 2026-10-06)

Measured, not estimated:
- Owner screenshots: the weekly meter went from 10% to 27% while the 5-hour
  meter went from 38% to 99%. One 5-hour point is about 0.28 weekly points,
  so one full 5-hour window is about 28% of the week (about 3.6 windows per
  week).
- Every session's rate_limit_info reads rateLimitType seven_day,
  status allowed_warning, resetsAt 1791846000. That is 2026-10-12 23:00 UTC,
  which is Tue 13 Oct 04:30 IST.
- The owner's own Tickvault session (session_01DysSVGYv7sS3FmL1kmpKwr) spends
  from the same limits: its cost_usd was 313.36 at 07:31 UTC. It is not ours
  to pause.

Extrapolation, to be labelled as one:
weekly ~= 27 + 0.28 x (5-hour % used since 07:10). At 07:31 that is about
31%. The owner's screenshots override it.

At full speed the remaining ~69% is about 2.5 windows. The week would run out
between about 20:00 UTC tonight and 03:00 UTC tomorrow. That locks every Claude
session on the account, Tickvault and GDFL included, until Tue 04:30 IST.

Default until the owner picks another plan:
- This window (07:10-12:10): all 13 run, with the 5-hour pause at 60% as
  before.
- At the 12:12 resume: landing work only. That is the 5 Gate 18 fixers, WS2,
  WS3, WS4 and the coordinator. L1-L4 and WS5 (discovery) stay paused.
- Weekly at 85% or more (extrapolated or read): pause every session the
  coordinator runs until the weekly reset. The rest of the week is left for
  the owner's own sessions.
- Each poll also checks connection_status. If an idle session reads
  "disconnected", its container was reclaimed and its background run died,
  so send it a restart message. (L2 was disconnected at 07:31 and restarted
  at 07:33.)

07:51 poll: the 14 brutex sessions sum to $474.76, which is $125.50 since 07:10.
At $3.5 per point that estimates 35.9% of this window. Tickvault was $322.43
(+$9.07 since 07:31). Tickvault spends less than it did at calibration, so that
estimate leans high; the conservative number stands. Weekly extrapolation: about
37%. All 13 workers were connected. Next poll at 08:10 (trig_01AhHDB3ZyczWMxNVvnUG8Ss).

How to read the "not allowed" clause in the poll triggers (07:58): every session has
read seven_day/allowed_warning since at least 07:31. That is the weekly pace warning,
and the WEEKLY GUARD above handles it, because the owner said to run everything. An
immediate PAUSE still follows from a five_hour warning, from any status "rejected",
or from the 5-hour estimate reaching 60%.

08:07 poll: the 14 brutex sessions sum to $557.19, which is $207.93 since 07:10.
That estimates 59.4% of the window, and cost_usd trails real spend. WS5 alone went
$76.15 -> $137.97 between 07:51 and 08:06. Tickvault was $339.20.
PAUSE was sent to all 13 sessions (priority now). Builds and mutation runs already
going may finish, since they cost no usage.
The zero-defect audit workflow wf_33b651ee-26e was stopped at 08:08 so the sessions
keep room to save their work.
RESUME trigger: 12:12 UTC (trig_01VA1zhRBcZL5kLMKpFrS152), plan B unless the owner
picks A or C.
Workflow relaunch: Workflow({scriptPath:
"/root/.claude/projects/-home-user-brutex/a9c2d522-edad-5d9f-8141-66605ab12cdb/workflows/scripts/zero-defect-gap-audit-wf_33b651ee-26e.js",
resumeFromRunId: "wf_33b651ee-26e", args: {"base": "origin/final/all-fixes",
"branches": ["pr74/g18-cli-a", "pr74/g18-cli-b", "pr74/g18-api", "pr74/g18-runner",
"pr74/g18-rest", "claude/attack-data-pipeline-hgxmw9", "fixboard/pr74-batch3",
"wip/audit-batch3", "zero/next", "attack/observability", "attack/o1-p99",
"attack/permutations", "attack/one-authority"]}}). Add "fixboard/pr74-batch4" to
"branches", because WS3 started it at 07:51. That changes the args, so the cached
finder results replay only where the prompt text is unchanged; accept it.
Weekly extrapolation at 08:07: about 27 + 0.28 x 59 = 44%.

08:16: the owner asked for the work to keep running, so all 12 unfinished sessions got RESUME
(L2 was done). The pause line was raised to 88% for this window.
08:26 poll (read directly, Opus only): the 14 brutex sessions sum to $632.46, which is
$283.20 since 07:10. That estimates 80.9%. WS3's cost_usd has not moved since 07:31
($16.43) even though it is running, so the real figure is higher. Tickvault was $367.86
(+$28.67 since 08:06). PAUSE was sent to all 12 at 08:28. RESUME comes at 12:12
(trig_01VA1zhRBcZL5kLMKpFrS152), together with the audit workflow relaunch.
Weekly extrapolation: about 27 + 0.28 x 85 = 51%.

RECALIBRATION 2 (08:56 UTC). Owner meter screenshot: session 53% (resets in 3h15m), weekly
all models 41%, weekly Fable 0%, plan Max (20x). The $3.5-per-point factor had read 88%, so it
was far too high: the 08:28 PAUSE was unnecessary. The brutex sum was $656.30 at 08:39 and
barely moved while paused.
New estimate: 53 + (sum - 656.30) / 5.8. That is $307 of brutex spend for 53 points; Tickvault
spends from the same window.
Lines: stop the audit workflow at 80%; PAUSE every session at 90%. Weekly brake at 85%. The
weekly moved 27 -> 41 while the window went 99 (prev) + 53 (this one).
All 12 unfinished sessions got RESUME at 08:56. The audit workflow was relaunched as task
wrp8x51vb, resumed from run wf_33b651ee-26e with fixboard/pr74-batch4 added.

09:10: owner meter screenshot read session 71% (resets in 3h00m) and weekly 46%. The window
rose 18 points in 15 min, about 1.2 points/min. At 09:11 the brutex sum was $745.25 and
Tickvault $429.58 (+$42 since 08:39). New factor: about $4.9 of brutex spend per point.
09:18: the audit workflow wrp8x51vb was stopped at the 80% line. Its cached agents replay at
12:12 (run wf_33b651ee-26e).
09:23: PAUSE was sent to all 12 unfinished sessions at about 88-90% by the meter's pace.
RESUME is at 12:12 (trig_01VA1zhRBcZL5kLMKpFrS152).
Weekly extrapolation: 46 + 0.28 x 19 = about 51%.

12:12 RESUME (window 12:10-17:10). The 14 brutex sessions summed to $807.93 and Tickvault to
$579.08; Tickvault spent about $138 between 09:18 and 12:12, while brutex was paused. WS4 and
the rest fixer recorded a five_hour allowed_warning (resetsAt 12:10), so the old window really
was near full.
RESUME went to all 11 paused sessions. L2 got its findings-ledger fix task. L4 had already
resumed on its own. The audit workflow was relaunched as ww8p80jjb from run wf_33b651ee-26e.
Next poll: 12:40.

## 12:26 UTC — audit workflow stopped on usage (owner meter: session 13%, weekly 57%)
- Owner asked whether this many agents is needed. Weekly is the binding limit: 57% on day 1 of 7
  (resets Mon Oct 12 23:00 UTC). One full 5h window ~ +28 weekly points.
- Stopped wf_33b651ee-26e (task ww8p80jjb) after Find 7/9 (43 agents, ~797k tokens). The 17
  candidate findings are saved in audit-find-17.json beside this file. Not started: web and
  concurrency finders, 34 verify agents, critic, gap round.
- Replacement verify, no agents: the coordinator checks each candidate against docs/11, 06, 05
  and 04 by hand, then routes survivors to the owning session, which verifies by writing the
  failing test first (refute = no failing test, say so). #10 (Gate 18 build-timeout multiplier,
  my own commit 46439dee) is the coordinator's own critical-path item.
- Weekly brake proposed lower: 75% (was 85%).

## 12:40 UTC — owner approved the recommendations ("whichever is recommended go ahead")
- Weekly brake is now 75% (was 85%). 5h lines unchanged: PAUSE all at 90%.
- PARKED after their current step: L1 (attack/observability), L3 (attack/permutations),
  L4 (attack/one-authority). L2 parks after the findings-ledger fix on attack/o1-p99. Each
  writes PARK-L<n>.md here. Resume = send RESUME with the PARK note's next step.
- Gap-audit candidates: hand check against docs/05, 06, 04 and 11.
  - #11 KNOWN: coverage at 90% lines / 89% regions is a recorded limit (D-0677, D-1610,
    docs/06-limits.md:237). No action.
  - #10 coordinator's own: Gate 18 --build-timeout-multiplier 2 (D-2090, integ 46439dee, unpushed).
    - Run 1283 shard 110 baseline: 598s build + 2045s test (CARGO_BUILD_JOBS=2, rust-cache
      restored, 195 lock packages).
    - Worker 2's first build is cold (copy_target=false), so ×2 = 1196s is likely too tight.
      Measuring cold vs warm-deps locally (/tmp/claude-0/bt-measure.log) before choosing the
      bound. Not pushing ×2.
  - Routed, after each session's current hand-over (verify by a failing test first; refuted =
    say so):
    - WS2: #1 #16 (D-3680..3683)
    - WS3: #2 #3 #4 #12 #13 #14 #17 on fixboard/pr74-batch5 (D-3684..3691)
    - WS4: #7 #8 #9 #15 (D-3692..3695)
    - WS5: #5 #6 look-ahead (D-3696..3699)

## 14:44 UTC — WEEKLY BRAKE (75%) reached; brutex fleet PAUSED; PR 74 pushed
- Estimate at 14:42: 5h window ~73%, weekly ~75%. Tickvault (the owner's session) spent $45.31 in
  21 min, against brutex's $6.36.
- PAUSE sent at 14:44 to WS5, WS3, WS2, WS4, cli-a (finish its hand-over only), api and rest.
  Each lets running jobs finish, records them, starts nothing new and writes PAUSE-*-1444.md.
  L1..L4 were already parked; runner and cli-b are done.
- PUSHED integ 3694ef66 to final/all-fixes (fast-forward from 969493e1, 89 commits, no GDFL or
  data paths, no live-mutant marker).
  - Validated: fmt; gates 29/29; clippy -D warnings; tests 122/125 as uid 65534, with the 3
    remaining failures passing as root (findings 14/14, pull unit 162/162, store cited_commits
    6/6).
  - Contains: D-2090 and D-2091 (Gate 18 build bound ×4); the runner (806a463), rest (fea3659) and
    cli-b (27b77704) fixers; L2 (aafe0d2); L3 (8635409 plus the 8908f88a ledger fix).
  - Not yet in: cli-a (finishing), api (0be9754b, partial proof), and WS2..WS5 and L1/L4 (not
    handed over). CI is expected to show their run-1283 survivors again.
- RESUME = the next 5h window (17:10 UTC) only if weekly allows. The weekly resets Mon Oct 12
  23:00 UTC. The owner decides.

## 14:52 UTC — usage check (owner: "check the usage and pause and resume")
- Estimate: 5h window ~77%; rate_limit_info has switched to five_hour/allowed_warning, resetting
  17:10 UTC (1791306600). Weekly ~76%, resetting Mon 12 Oct 23:00 UTC (1791846000).
- Since 14:42: brutex +$10.79 (sum $1015.00), Tickvault +$9.63 ($735.67). Tickvault set itself to
  resume at 00:18 IST.
- Fleet stays PAUSED. WS5 was still finishing its in-flight command at 14:52; cli-a is finishing
  its hand-over only.
- RESUME plan (triggers armed):
  - 17:12 UTC (trig_01U56UBvcyHEdqqVaPjitwHV): MINIMAL resume, PR 74 critical path only. Read CI
    run 1285, fix non-Gate-18 failures, and resume only the owners of named survivors (likely api,
    cli-a). Hard cap: re-pause everything at weekly 80%.
  - Mon 12 Oct 23:02 UTC (trig_01SG3BVzJf3qpRKW9i1ArED4): FULL resume of every session from its
    PAUSE/PARK note. New-week lines: weekly 70% by day 3; 5h PAUSE at 90%.
  - CI check 15:31 UTC (trig_01DhayNM7e5WcmsfBpceWQF7) is still armed.

## 17:15 UTC — the 5h window HIT its limit ~15:15; estimate correction; nobody resumes
- WS5 and WS3 show "You've hit your session limit · resets 5:10pm (UTC)" (five_hour: rejected).
  The 14:52 estimate (~77%) was low. Correction: the cost-based estimate lags by more than the
  ~3-4 points assumed. Treat it as a floor and lean ~+15.
- Weekly is therefore likely ~80-85% (≈ 57 + (100-13) × 0.3). That is past the 80% hard cap of the
  17:12 minimal-resume plan, so NO session was resumed at 17:12. Hold until the weekly reset
  (Mon 12 Oct 23:00 UTC = Tue 13 Oct 04:30 IST; trig_01SG3BVzJf3qpRKW9i1ArED4 armed for 23:02).
- New 5h baseline (17:15): brutex $1032.26, Tickvault $740.20. Tickvault paused itself until
  00:18 IST.
- PR 74:
  - Run 1285 (3694ef66): ALL fast gates GREEN — gates 1+2, W, 3-6 (fmt, clippy, deny, tests),
    8 (O(1)), and the Gate 18 enumerate step. Coverage was still running, and the shards wait on
    coverage.
  - fbdabaec (integ = 3694ef66 + cli-a 7ad5c66e + api 0be9754b) validated (fmt, 29/29 gates,
    clippy, tests; env-only failures pass as root). Pushed at 17:14 → run 1286 (37502065146);
    1285 was cancelled by concurrency. Gate 18 (202 shards, max-parallel 20) is expected to take
    ~30 h.
  - Cheap CI check armed for 19:50 UTC.

## ~17:33 UTC — OWNER METER (truth): 5h session 9% (resets 22:10 UTC), WEEKLY 85% (resets Tue 13 Oct 04:30 IST)
- Calibration: from 12:24 (weekly 57%, session 13%) to now, session +96 points (13→100, then 0→9)
  and weekly +28, so weekly ≈ 0.29 × session points. Confirmed.
- 15% weekly is left for ~6.2 days, about half of one 5h window in total.
- The brutex fleet stays PAUSED (past the 80% hard cap). The coordinator does only cheap CI
  checks (19:50, then ~10 h apart).
- The 9% of this window used since 17:10 is mostly the owner's GDFL session (active: "audit
  fix-plan + Zerodha 1m pull") plus the coordinator. Tickvault resumes itself at 00:18 IST. Both
  draw on the same weekly 15%.
- Option for the owner (NOT taken without a yes): when run 1286 finishes (~Oct 7 evening UTC),
  spend up to ~8% of the weekly waking only the owners of named survivors, so PR 74 can go green
  before the reset. Default is to hold until Tue 04:30 IST.

## ~17:40 UTC — OWNER APPROVED: spend up to 8% weekly on PR 74 survivors (cap 93%)
- The owner answered "go ahead" to the option above. Weekly HARD CAP for this week: 93%
  (from 85%).
- Use: only after CI run 1286 (37502065146, fbdabaec) finishes. Wake ONLY the owners of named
  Gate 18 survivors (cli-a, cli-b, api, runner, rest), fix only those, then one push. Everything
  else stays paused.
- Survivor-round trigger armed for Oct 7 20:30 UTC; it re-arms +2 h while the run is still going.
  The cheap CI check at 19:50 UTC Oct 6 is still armed. Full RESUME at Mon 12 Oct 23:02 UTC.

## 17:46 UTC — handoff prompt for a NEW ACCOUNT written (owner's request)
- The owner asked for one precise prompt to continue every held, paused or unfinished job from a different
  Claude account (Claude Code project, beta), delivered at the latest when weekly reaches 95%.
- Written: `resume/HANDOFF-NEW-ACCOUNT.md`. It holds the prompt and the per-workstream table. It is
  refreshed at every coordinator wake, and before the 93% stop.
- Takeover lock: the new account first writes `resume/HANDOFF-CLAIM.md`. Every routine on this account
  checks that file and stands down if it exists, so two coordinators never push `final/all-fixes`.
- Costs now: brutex fleet $1048.25 (was $1032.26 at 17:15); Tickvault $751.59 (was $740.20).
- Estimate (lean high; GDFL is unmeasured, so ×1.5):
  - the extra $27.4 is about 5.6 points of the 5h window, ×1.5 = 8.4;
  - weekly ≈ 85 + 0.29 × 8.4 ≈ 87.4%.
  The owner meter overrides.

## 17:51 UTC — OWNER METER 17:49: weekly 87%, 5h session 15% (resets 22:10 UTC). Handoff refreshes 95→100
- Owner: "once it touches 95 percent, keep updating the prompt for every percentage until it reaches 100".
- Tickvault is blocked on the owner: "87% weekly usage; awaiting decision on pausing depth build". It is the owner's
  session, so the coordinator does not touch it.
- Costs at 17:51: this session $134.88, Tickvault $763.62, other brutex sessions $915.56 (all paused).

### CURRENT FORMULA (re-anchor here on every owner screenshot)
- Anchor A = 87% at M_A = $1814.06 (17:49 owner meter; M = 915.56 + this session + Tickvault).
- est = A + (M − M_A) / 7.9.
- Where 7.9 comes from:
  - $4.9 per 5h-window point;
  - weekly ≈ 0.31 × window points (12:24 → 17:49 owner readings);
  - × 2, because the owner's GDFL bridge session is unmeasured and is assumed to equal the measured spend.
  It LEANS HIGH. Label every number "est"; the owner meter overrides.
- If any other brutex session is woken, add its cost change since 17:51 to M.
- LAST_SENT = none.

### WATCH PROCEDURE (run on every "USAGE WATCH" wake, and inside every CI-check and survivor-round wake)
1. **Lock.** If origin/fix-queue has resume/HANDOFF-CLAIM.md, the new account has taken over:
   1. disable every enabled trigger of this session (list_triggers);
   2. tell the owner in one line;
   3. stop.
2. **Measure and estimate.**
   1. Call get_session with no id (this session) and get_session session_01DysSVGYv7sS3FmL1kmpKwr (Tickvault).
   2. M = 915.56 + both costs (+ any woken brutex session's change).
   3. est = CURRENT FORMULA.
3. **Refresh** if est ≥ 95 and floor(est) > LAST_SENT:
   1. In resume/HANDOFF-NEW-ACCOUNT.md, update the "Last refreshed" block (UTC time, "weekly ~N% (est)", the anchor).
   2. Update the CI run 1286 line: actions_get get_workflow_run, plus get_job_logs failed_only=true
      return_content=false for the failed shard count.
   3. Update any workstream row whose state changed.
   4. Push fix-queue: fetch, merge, push.
   5. Copy the file to the scratchpad, then SendUserFile it and PushNotification
      "weekly ~N% (est): handoff prompt refreshed".
   6. Set LAST_SENT = N here.
4. **93% stop.** If est ≥ 93, no brutex work starts (the survivor round only pauses). Only this watch keeps running.
5. **Next wake.** Schedule one only if no pending reminder named "usage watch" exists (list_triggers).
   - Use send_later with name "usage watch" and the message "USAGE WATCH: in /tmp/claude-0/fq fetch and merge
     origin/fix-queue, then run the WATCH PROCEDURE in resume/pr74-g18-run1283/USAGE-GUARD.md".
   - Delay: if est ≥ 95, 20 min; otherwise clamp((95 − est) × 15, 20, 120) min.
6. **Log.** Append one line here (time, M, est, action) and push. Below 95, say nothing to the owner unless asked.
7. **End.** It ends at 100%: the session can no longer run. The last refresh is the one at 99 (or the last whole
   percent reached).

## 19:53 UTC — USAGE WATCH (CI check wake): weekly ~98% est (lower bound); handoff refreshed and sent
- Costs: this session $135.59, Tickvault $935.15 (+$171.5 since 17:51), other brutex $915.56. M = $1986.30;
  ΔM = +$172.24 since the anchor.
- FORMULA CORRECTION: the ×2 GDFL factor assumed GDFL equals the measured spend. It does not hold when Tickvault
  dominates: it gives 108.8%, which is impossible because sessions are still allowed.
- CURRENT FORMULA (replaces the 17:51 one):
  - est_low = 87 + ΔM / 15.8, where 15.8 = $4.9 per window point ÷ 0.31; measured spend only.
  - Now est_low = 97.9. The real figure is ≥ est_low because GDFL is not counted.
  - Anchor stays 87% at M_A = $1814.06.
  - LAST_SENT = 97.
- CI run 1286: in progress, 6 jobs, 0 failed; Gate 18 shards not started yet.
- WS3 (8a41aa26): the line-1358 survivors are killed, 6/6 caught; line 1337 and api Gate 18 are still open.
- Next usage watch in 20 min.
