id	state	commit	note	title	link
W2-cli5-3	found		Shown on PR #74 only from a group-level claim. At c97ff00 absorb_new_rows still starts with refuse_integrity_failure_for_write (cli/src/frontier.rs:1241) and no head commit names W2-cli5-3	
W3-runner2-1	partial		c4a-verdicts.md:25 rates it PARTIAL at 1087e54 (facts no longer rebuilt; the re-walk remains) and no later head commit names it	
W3-runner2-3	partial		c4a-verdicts.md:24 rates it PARTIAL at 1087e54 and no later head commit names it	
W3-runner2-5	partial		c4a-verdicts.md:33 rates it PARTIAL at 1087e54; the linear search is still at runner/src/exit_grid_policy.rs:573 on c97ff00	
W2-cli1-2	doc	66f140cb	Written limit on PR #74 (D-1631, stated not fixed); the real fix is queued in the attack audit (F8), because nothing stays documented-only	
W2-cli1-3	doc	66f140cb	Written limit on PR #74 (D-1631, stated not fixed); the real fix is queued in the attack audit (F8), because nothing stays documented-only	
W2-cli6-0	doc	513c8b57	Written limit on PR #74 (D-1633, stated not fixed); the real fix is queued in the attack audit (F8), because nothing stays documented-only	
W2-cli16-2	doc	ce4fb1aa	Written limit on PR #74 (D-1634, stated not fixed); the real fix is queued in the attack audit (F8), because nothing stays documented-only	
W2-cli16-3	doc	ce4fb1aa	Written limit on PR #74 (D-1634, stated not fixed); the real fix is queued in the attack audit (F8), because nothing stays documented-only	
W2-cli15-2	doc	f947eece	Written limit on PR #74 (D-1636, stated not fixed); the real fix is queued in the attack audit (F8), because nothing stays documented-only	
W2-cli7-0	doc	cb80b2aa	Written limit on PR #74 (D-1638, stated not fixed); the real fix is queued in the attack audit (F8), because nothing stays documented-only	
W2-cli10-2	doc	ca4fe0f9	Written limit on PR #74 (D-1639, stated not fixed); the real fix is queued in the attack audit (F8), because nothing stays documented-only	
W2-cli14-1	doc	50d870b1	Written limit on PR #74 (D-1642, stated not fixed); the real fix is queued in the attack audit (F8), because nothing stays documented-only	
W2-cli14-2	doc	50d870b1	Written limit on PR #74 (D-1642, stated not fixed); the real fix is queued in the attack audit (F8), because nothing stays documented-only	
W2-cli14-3	doc	50d870b1	Written limit on PR #74 (D-1642, stated not fixed); the real fix is queued in the attack audit (F8), because nothing stays documented-only	
W2-cli12-3	doc		Written limit on PR #74 (D-0934 keeps the whole-file ledger rescan and states its cost; group cli-14); the real fix is queued in the attack audit (F8), because nothing stays documented-only	Results ledger rescans the whole file per append or per row (group cli-14, D-0934)
W2-cli12-4	doc		Written limit on PR #74 (D-0934 keeps the whole-file ledger rescan and states its cost; group cli-14); the real fix is queued in the attack audit (F8), because nothing stays documented-only	Results ledger rescans the whole file per append or per row (group cli-14, D-0934)
D-0742-block	branch	fd45f9d2	On branch zero/numeric (fd45f9d2, not merged; tests partly unrun), per the zero-findings status file; id as the zero-findings status file names it; the source calls it D-0742	D-0742 bootstrap block length is never checked against the period count (block >= 40000 lets 36 of 40 noise strategies pass White, SPA and Romano-Wolf)	https://github.com/SJParthi/brutex/blob/fix-queue/resume/project-files-20261004/zero-rounds/numeric-complexity.md
D-0743-pbo	branch	fd45f9d2	On branch zero/numeric (fd45f9d2, not merged; tests partly unrun), per the zero-findings status file; id as the zero-findings status file names it; the source calls it D-0743	D-0743 PBO is floored into a max-gated field (cli/src/index_stop_qualification_numeric.rs:608) while the p-values beside it round up	https://github.com/SJParthi/brutex/blob/fix-queue/resume/project-files-20261004/zero-rounds/numeric-complexity.md
cli-14	doc		Written limit on PR #74 (D-0934 keeps the whole-file ledger rescans of W2-cli11-0/-1 and W2-cli12-3/-4 and states their cost); the real fix is queued in the attack audit (F8), because nothing stays documented-only	
hunt-ci-1	partial	6d03a68e	The code half is on PR #74 (6d03a68e, D-1604/D-1605: code owners, auto-merge arms a gate change only on an owner approval). The branch-protection setting only the repo owner can change is still open (section 11 of the resume file)	
testgaps-7	found		cb775d9f lists it, but section 11 of the resume file holds it on operator data that is not in the repo	
GAP13-16	pushed	7c069758	Fixed on PR #74 by D-1487 (7c069758, on head c97ff00): render_pooled lays out with crate::columns; RESUME-20261004.md section 4 records the same	
GAP13-13	pushed	16186acd	Fixed on PR #74 by D-1564 (16186acd, on head c97ff00): sweep-all files ledger rows in walk order; the same defect re-reported as hunt-conc-1 is on PR #74	
