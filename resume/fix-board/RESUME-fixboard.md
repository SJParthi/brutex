# Fix Board thread: resume state (2026-10-04 13:42 UTC)

Saved ahead of a 5-hour usage pause. GitHub state wins over this file.

## The board
- Artifact (republish to this same URL every refresh): https://claude.ai/artifact/5Jr1kKxEUEmzZF9f19YfTi (version 10 at 13:20 UTC).
- Hourly routine trig_01BEoTSneJLtU8Rb1PLXNhWe fires at :19.
- Builder: `resume/fix-board/builder` (Rust, standalone). Steps and the refresh command: `resume/fix-board/README.md`.
- Last refresh 13:2x UTC: 1,065 distinct findings (404 on PR #74, 308 branch, 116 fixing, 6 partial, 13 doc, 218 not started). PR #74 head 8c9313c.

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

## Next
1. Hourly refresh as above.
2. Finish zero-p8 and send its hashes to zero-findings; ask it for more unassigned items.
3. Tests running as root fail with PermissionDenied for re-exec-as-nobody tests when binaries sit under a 0700 dir; run those binaries from a readable dir.
