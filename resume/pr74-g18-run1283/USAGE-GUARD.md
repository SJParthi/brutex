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
