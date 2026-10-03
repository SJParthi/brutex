# Concurrency and state audit: resume state (2026-10-03 18:35 UTC)

This is the helper thread "Audit helper: concurrency and state" for the zero-findings audit loop. It only audits and never pushes code. Its findings go to the zero-findings session (session_019avWwsbRWYrj7eHCev5fVr) through send_message.

## Done
- Pass 1: 20 agents at final/all-fixes 331b05c found 63 issues (24 medium, 39 low). Reports are in reports/pass1/.
- Pass 2: 20 agents, sliced by end-to-end flow, found 51 new issues (16 medium, 35 low). This pass also refuted or narrowed 7 pass-1 claims. Reports are in reports/pass2/.
- Pass 3: 2 agents at final/all-fixes-zero 5140aca3 swept two families and found 6 new issues (1 medium, 5 low). Reports are in reports/pass3/ and include coverage tables for 37 durable writers and 33 try_lock sites.
- concurrency.md is the merged file with all three passes. Its live copy is /mnt/project-files/zero-rounds/concurrency.md.
- Fix verification at 5140aca3: D-1760 fixed search-1, cli3-1, replay-2 and indexstop-2(c).
- The zero-findings session has queued every pass-3 item to its torn-ledger fix agent. It will message the final/all-fixes-zero head once that branch merges.

## Next (pass 4): do not start until weekly usage allows
1. Wait for the zero-findings session's message giving the merged final/all-fixes-zero head.
2. Check out that head read-only. Run at most 2 agents using prompt3.md, after updating the commit and setting PASS=4.
   - Agent A, ledger writers: verify every fix the new commits claim, then re-sweep the family.
   - Agent B, try_lock sites: do the same for the non-blocking lock sites.
3. Append a Pass 4 section to concurrency.md and send the delta to the zero-findings session.
4. Stop when a pass finds nothing new.

prompt.md, prompt2.md and prompt3.md are the agent briefs for passes 1 to 3.
