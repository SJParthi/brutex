# Queued (user asked to throttle 13:14 UTC; cap 3 agents)
Running: w6 (api), w7 (cli), mut1 (mutants)
Queued, resume in order when one finishes (resume with SendMessage to the stopped agent, or a fresh agent on the same worktree with FIXRULES2.md):
1. w8 /home/claude/w8 data/engine: attackdata-4, attackdata-8, o1eng2-1, gaps-1, gaps-3 (D-1570..1575, AFF-40..59). Was about to write failing tests.
2. w9 /home/claude/w9 features: gaps-5, gaps-11, gaps-10 (D-1576..1579, D-1593..1594, AFF-60..79).
- After w6/w7/mut1: send batch commit to CI thread cse_01TPJRnnkg5yuNeRBHDyzaaP BEFORE pushing.
- Keep /mnt/project-files/fix-board/status/sweep.tsv current (id, state found|fixing|branch|pushed|green, commit) after every change.
- mut1 (fb4bd7c) folded by CI thread into its push (mut-all); do NOT push it separately. Wait for its commit, then merge origin into integrate.
