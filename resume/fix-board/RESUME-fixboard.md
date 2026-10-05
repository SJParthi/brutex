# Fix Board thread: resume state (2026-10-04 18:50 UTC; pausing for the 5-hour limit, resume 22:03 UTC)

Saved ahead of a 5-hour usage pause. GitHub state wins over this file.

## The board
- Artifact (republish to this same URL every refresh): https://claude.ai/artifact/5Jr1kKxEUEmzZF9f19YfTi (version 18 at 23:2x UTC).
- Hourly routine trig_01BEoTSneJLtU8Rb1PLXNhWe fires at :19.
- Builder: `resume/fix-board/builder` (Rust, standalone). Steps and the refresh command: `resume/fix-board/README.md`.
- Last refresh 18:2x UTC: 1,183 rows (495 pushed, 285 branch, 132 fixing, 5 partial, 17 doc, 249 found). PR #74 head fc6dbb9d, run 37223441970. Since then head is 1f588aae (web batch 1 merged).
- This thread's own status file: /mnt/project-files/fix-board/status/fixboard.tsv (run.sh globs every status tsv).

## Local wrapper used for every refresh (recreate as /tmp/claude-0/run.sh; the worktree of fix-queue is /tmp/claude-0/fq)
```
# usage: run.sh "<as-of>" <ABS out-board> <ABS out-snapshot>   (optional SNAP=<snapshot>)
cd /tmp/claude-0/fq
ST=""; for f in /mnt/project-files/fix-board/status/*.tsv; do ST="$ST --status $f"; done
/tmp/claude-0/fbt/release/fixboard --board resume/fix-board/brutex-fix-board.html --out "$2" \
  --snapshot ${SNAP:-resume/fix-board/ledger-snapshot.json.md} --snapshot-out "$3" $ST \
  --corrections resume/fix-board/inputs/corrections.tsv.md --checked-at c97ff00 --same-as resume/fix-board/inputs/same-as.tsv.md \
  --not-a-fix 331b05c --not-a-fix 1087e54 --base-ref 331b05c \
  --zero-dir /mnt/project-files/zero-rounds --repo /home/claude/brutex \
  --checks resume/fix-board/inputs/checks.tsv.md --pr resume/fix-board/inputs/pr.tsv.md \
  --streams resume/fix-board/inputs/streams.tsv.md --as-of "$1"
```
Build: `cd resume/fix-board/builder && CARGO_TARGET_DIR=/tmp/claude-0/fbt cargo build --release`. Then copy the output page to the scratchpad and fq, the snapshot to `ledger-snapshot.json.md`, publish, commit, then `git fetch origin fix-queue && git merge origin/fix-queue && git push origin HEAD:fix-queue` (never force).

## Fixes this thread made for the zero-findings thread (D-2700..2799, invariants FB-)
All on branches from final/all-fixes-zero 1f4de71a; hashes sent to zero-findings (session_01GS9PBjP9nShhuEHrx1N2ia) to merge into staging, then PR #74.
- fixboard/zero-p5 7b9e6124: P5-01..06 (D-2700..2703). P5-07 WIP, no D-2704 yet. New real bug FB-01 (runner BlockExtremes one level short). Zero-findings had P5 already, so it compares and keeps one.
- fixboard/zero-num56 93283ef6: p5num-1/3/4 (D-2712..2714) are being merged by zero-findings. 0ec5fbe6 (p6num) is NOT taken, because it duplicates D-2666. p5num-2 needs a format version (operator); p5num-5 not started.
- fixboard/zero-web12 dca033e1: CE-70..73 (D-2730..2733). Base 1f4de71a already fails Gate W1 (web/build stale); dca033e1 rebuilds it.
- fixboard/zero-p10 7b2ffdd9: P10-01/02/03/05/06/07a killed; P10-04 was already killed (false finding); P10-07b restructured (D-2740).
- fixboard/zero-p8 c26d8db5: P8-01..05 and p8num-1 all done (D-2720..2725, FB-21..27), handed to zero-findings 17:3x UTC. Base failure found: api sweeprun::strict_tests::strict_out_of_domain_request_settings_refuse_before_configuration_slot_or_start (reported to zero-findings).

- fixboard/pr74-web18 (worktree /tmp/claude-0/wt-pr74-web18): batch 1 P17-01, p14num-1, conc18-1, conc18-2 (tip 891d8f8d) MERGED into PR #74 as 1f588aae. Batch 2 conc18-3 5620eae8, conc18-4 e4a6158d, conc18-5 ed207d0e, p14num-2 f460a400 (D-2750), merged with final/all-fixes at 605c5299, handed to PR 74 CI 18:45 UTC. Not merged with origin/zero-work (it does not contain PR #74 head; 58 conflict hunks; PR 74 CI owns that integration). CE-77 left to zero-findings 356b15ae.
- fixboard/pr74-conc-api dee61cfd: all 7 merged into PR #74 as 25bc8aa (PR 74 CI declared the 2 Gate 1d scratch roots).
- fixboard/pr74-api2 (wt /tmp/claude-0/wt-pr74-api2, tgt-conc, D-2770..2779, FB-71..79): conc server1-1/2, server2-1/2, runs-2/3/4, apicache-2. Container restart 22:3x killed the first agent; its work is WIP commit b018bc9a (pushed); a new agent is finishing it.
- fixboard/pr74-ce2 (wt /tmp/claude-0/wt-pr74-ce2, tgt-ce2, D-2751..2759 then D-2790.., FB-91..99): CE-84..94, CE-98..101 (CE-95..97 are zero/docs-batch 768d742d). Container restart killed the first agent; its work is WIP commit 7db179f8 (pushed, unvalidated). Finished locally as 44252d2b + uncommitted mastersrun.rs clippy fix (Duration::from_hours(1)); all checks pass. Push BLOCKED by the auto-mode classifier (CE-98 removes 3 debug_asserts on log writes). Asked the user 00:0x UTC to reply "push ce2"; push only on their word, then hand to PR 74 CI. Reverse-proof of the new tests also blocked. D-2751..2759, D-2790..2794, FB-91..99, FB-101..105 used. Zero-findings confirmed both batches are ours.

## Next
1. On resume (22:03 UTC): fetch, check fixboard/pr74-conc-api and 605c5299 status on head; update fixboard.tsv.
2. Hourly refresh (routine at :19): inputs checks/pr/streams, run.sh, render check, republish, commit to fix-queue.
3. Take more unclaimed found rows (tell zero-findings first). Free D-numbers: D-2751..2759, D-2770..2799.
