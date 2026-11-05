# replay-minus-reports.sh.md

Source: `state/cloud-wave-20261103/replay-minus-reports.sh` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````sh
#!/bin/bash
# replay-minus-reports.sh (3 Nov 2026, repair 1 of the cloud-wave PLAN; shell + git only, never the Py-language).
# Takes a cloud session's branch into the Mac (or into an assembly session's branch) WITHOUT its kit commit and WITHOUT
# any file under reports/: each own commit (FROM..TIP, oldest first) is replayed onto ONTO (default START) as the same
# change outside reports/, with the commit's own message, author and author date. A commit that touched only reports/
# is skipped; a commit that touched both is split by path (only its non-reports part is replayed). A merge is refused.
# It writes git objects in a temporary index and, at the end, ONE new ref (NEWREF) only if every proof holds. It never
# moves an existing ref, never touches a work tree, never fetches or pushes. Run it under nice -n 15.
#
# Usage: replay-minus-reports.sh <git dir or worktree> <START> <FROM> <TIP> <NEWREF refs/heads/...> [ONTO]
#   START: the commit the session's kit sits on (FROM's change over START must be under reports/ only)
#   FROM : the kit commit the session's branch was cut from (or START itself); FROM..TIP are the session's own commits
#   ONTO : where to replay (default START). An assembly passes the previous shard's replayed tip here, so shards that
#          touch disjoint files are stacked one by one, each with its own proofs.
#   fib part:  START 1ad6e24ef8e05e8836e511873e8352071dd98fa0, FROM fa9022bac962620528735ab97a61d6ab6d3dedbb (KIT1)
#   base shard: START = the base kit commit's parent (the Mac tip), FROM = the base kit commit
# Proofs (exit 1 with STOP-<name>; nothing is written to NEWREF then). X = the files FROM..TIP changes outside reports/:
#   P0 FROM..TIP has no merge; NEWREF is new; START is an ancestor of FROM and FROM changes nothing outside reports/
#   P1 every file of X in the new tip equals TIP's version            (the shard's code is exactly the cloud's)
#   P1b the new tip differs from ONTO outside reports/ only in X       (nothing else moved; an overlap stops here or at P1)
#   P1c when ONTO is START: the new tree equals TIP's outside reports/ (the strong form)
#   P2 the new tip's reports/ equals ONTO's                            (no report reaches the branch; ONTO's files stay)
#   P3 ONTO..new tip has no merge and exactly the replayed count of commits (messages are the sources', by construction)
#   P4 FROM is not an ancestor of the new tip, unless FROM is ONTO or one of its ancestors
set -euo pipefail
G=${1:?git dir or worktree}; S=${2:?START}; FROM=${3:?FROM}; T=${4:?TIP}; NEWREF=${5:?NEWREF}; ONTO=${6:-$2}
g() { git -C "$G" "$@"; }
S=$(g rev-parse --verify "$S^{commit}"); FROM=$(g rev-parse --verify "$FROM^{commit}"); T=$(g rev-parse --verify "$T^{commit}"); ONTO=$(g rev-parse --verify "$ONTO^{commit}")
case "$NEWREF" in refs/heads/*) ;; *) echo "STOP-NEWREF: give a full refs/heads/... name" >&2; exit 1;; esac
if g rev-parse -q --verify "$NEWREF" >/dev/null; then echo "STOP-NEWREF-EXISTS: $NEWREF" >&2; exit 1; fi
g merge-base --is-ancestor "$FROM" "$T" || { echo "STOP-FROM-NOT-ANCESTOR-OF-TIP" >&2; exit 1; }
g merge-base --is-ancestor "$S" "$FROM" || { echo "STOP-START-NOT-ANCESTOR-OF-FROM" >&2; exit 1; }
[ -z "$(g diff --name-only "$S" "$FROM" -- ':(exclude)reports')" ] || { echo "STOP-FROM-CHANGES-CODE: FROM differs from START outside reports/" >&2; exit 1; }
[ -z "$(g rev-list --merges "$FROM..$T")" ] || { echo "STOP-MERGE-COMMIT in $FROM..$T" >&2; exit 1; }
TMPD=$(mktemp -d); trap 'rm -rf "$TMPD"' EXIT
g diff --name-only --no-renames "$FROM" "$T" -- ':(exclude)reports' | LC_ALL=C sort -u > "$TMPD/X"
export GIT_INDEX_FILE="$TMPD/index"
g read-tree "$ONTO"
cur=$ONTO; n=0; skipped=0
for c in $(g rev-list --reverse "$FROM..$T"); do
  if [ -z "$(g diff --name-only "$c^" "$c" -- ':(exclude)reports')" ]; then skipped=$((skipped + 1)); echo "skip $(g log -1 --format='%h %s' "$c" | cut -c1-100) (reports only)"; continue; fi
  g diff --binary --full-index "$c^" "$c" -- ':(exclude)reports' > "$TMPD/p"
  g apply --cached "$TMPD/p" || { echo "STOP-APPLY: $c does not apply on the replayed tree (an overlap with ONTO?)" >&2; exit 1; }
  tree=$(g write-tree)
  msg=$(g log -1 --format=%B "$c")
  cur=$(GIT_AUTHOR_NAME="$(g log -1 --format=%an "$c")" GIT_AUTHOR_EMAIL="$(g log -1 --format=%ae "$c")" GIT_AUTHOR_DATE="$(g log -1 --format=%aI "$c")" \
        g commit-tree "$tree" -p "$cur" -m "$msg")
  n=$((n + 1)); echo "took $(g log -1 --format='%h %s' "$c" | cut -c1-100) -> ${cur:0:12}"
done
unset GIT_INDEX_FILE
[ "$n" -gt 0 ] || { echo "STOP-NOTHING-TO-REPLAY" >&2; exit 1; }
while IFS= read -r f; do g diff --quiet "$T" "$cur" -- "$f" || { echo "STOP-P1: $f differs from TIP's version" >&2; exit 1; }; done < "$TMPD/X"
g diff --name-only --no-renames "$ONTO" "$cur" -- ':(exclude)reports' | LC_ALL=C sort -u > "$TMPD/Y"
[ -z "$(LC_ALL=C comm -23 "$TMPD/Y" "$TMPD/X")" ] || { echo "STOP-P1b: files changed outside X: $(LC_ALL=C comm -23 "$TMPD/Y" "$TMPD/X" | head -5 | tr '\n' ' ')" >&2; exit 1; }
if [ "$ONTO" = "$S" ]; then g diff --quiet "$T" "$cur" -- ':(exclude)reports' || { echo "STOP-P1c: the code tree differs from TIP outside reports/" >&2; exit 1; }; fi
g diff --quiet "$ONTO" "$cur" -- reports || { echo "STOP-P2: reports/ differs from ONTO" >&2; exit 1; }
[ -z "$(g rev-list --merges "$ONTO..$cur")" ] && [ "$(g rev-list --count "$ONTO..$cur")" = "$n" ] || { echo "STOP-P3: commit count or a merge" >&2; exit 1; }
if ! g merge-base --is-ancestor "$FROM" "$ONTO" && g merge-base --is-ancestor "$FROM" "$cur"; then echo "STOP-P4: FROM is in the new history" >&2; exit 1; fi
g update-ref "$NEWREF" "$cur" ""
echo "OK $NEWREF = $cur ($n commits replayed onto ${ONTO:0:12}, $skipped skipped as reports only; P1-P4 hold; $(wc -l < "$TMPD/X" | tr -d ' ') files in X)"
`````
