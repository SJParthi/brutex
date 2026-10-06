# Usage guard (owner request 2026-10-06 ~03:55 UTC: "take care of usage, pause, hold, resume, continue")

## Readings and calibration
- Owner's screenshot (~03:50 UTC, plan Max 20x): 5-hour window 38% (resets 07:10 UTC = rate_limit_info.resetsAt 1791270600), weekly all models 10% (resets "Tue 4:30 AM" owner local time; if IST that is Mon 2026-10-12 23:00 UTC: INFERRED, not confirmed), weekly Fable 0%.
- No usage meter is readable from a cloud container (no OAuth token; provider managed by host). Estimator: sum of `external_metadata.usage.cost_usd` over every brutex-20261006 session + the coordinator, from get_session.
- Calibration at 03:52 UTC: sum $188.23 (14 sessions, all created inside this window) = 38% -> about $4.95 per 5-hour percent; weekly about $18.8 per percent (assumes this week's usage before 02:10 UTC was small; UNVERIFIED).
- Burn with 13 sessions: about $4.7/min ($280/h). Other sessions of the account (Mac GDFL, Tickvault) are invisible to the estimator, so thresholds are set conservatively.

## Thresholds (estimated %, conservative)
| Signal | Action |
|---|---|
| 5-hour >= 85% estimated, or any session's rate_limit_info.status != "allowed" | PAUSE all sessions (message below, priority now); coordinator saves state; send_later at reset + 2 min |
| 5-hour reset (07:10, 12:10, 17:10, 22:10 UTC ...) | snapshot every session's cost as the new baseline; RESUME all sessions |
| weekly >= 50% estimated | attack lenses finish their current round, hand over, no new rounds |
| weekly >= 80% estimated | only PR #74 work continues: Gate 18 fixers, coordinator integration, merging already-finished branches |
| weekly >= 93% estimated | PAUSE everything until the weekly reset |

## PAUSE message (send with priority now)
PAUSE (usage guard, from the coordinator): the account's usage limit is nearly reached. Within your next few tool calls: (1) do not arm monitors, wakeups or send_later; let a build or mutation run finish only if it is nearly done, else stop it; (2) commit everything on YOUR OWN branch (unvalidated work as one commit titled "WIP (paused, not validated)") and push it; (3) write resume/<your folder>/PAUSE-20261006.md on fix-queue: head SHA, done, in progress, exact next steps, commands to restart; (4) end your turn. If a background job wakes you during the pause, write its result into the PAUSE file in one tool call and end the turn. Start nothing new until a message that begins with RESUME arrives.

## RESUME message
RESUME (usage guard): the usage window has reset. Read your PAUSE-20261006.md, and continue your task from its next steps under the same rules.
