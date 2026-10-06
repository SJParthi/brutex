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
RESUME (usage guard): the usage window has reset. Read your PAUSE-20261006.md, and continue your task from its next steps under the same rules.

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
