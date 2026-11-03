# d3-route-b.sh.md

Source: `state/decisions-20261027/d3-route-b.sh` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````sh
#!/bin/bash
# d3-route-b.sh (D123 v4, 27 Oct 2026; shell + jq + node only, no Py-language): D3 COVERAGE by ROUTE B, the scripted
# route of recipe v4 step 4. cargo-llvm-cov 0.8.4 cannot read its objects on the pinned nightly (rustc 1.99.0-nightly
# 1a98b1e13 2026-08-07: 'error: failed to collect object files: not found object files', IA-3b 14:03 and R3-28 14:32
# IST on 27 Oct: the nightly puts test executables in its new build-dir layout); Route B worked on R3-28's tip the same
# day (store 14:33-14:57, pull 14:57-15:31): the nightly's own instrumentation and its OWN llvm-tools (llvm-profdata,
# llvm-cov from the same toolchain, so the profile format always matches; that is the pin).
#   1. pins: $N/bin/rustc -V, $T/llvm-cov --version, $T/llvm-profdata --version; a mismatch is an ENVIRONMENT item;
#   2. ONE instrumented build of every package given (RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch',
#      CARGO_PROFILE_DEV_OPT_LEVEL=0, CARGO_TARGET_DIR=<K>/cov, through the cargo slot queue with the nightly as its
#      real cargo); the objects are the executables THIS build reports (--message-format=json), never a folder scan;
#   3. per package: its own tests (same environment, so nothing is rebuilt) with LLVM_PROFILE_FILE in a NEW folder
#      <P>/<pkg>, then llvm-profdata merge -sparse, then llvm-cov show of every set file of that package (crates/<pkg>/),
#      read by d3-covset.mjs against the set file's coverage items.
# A test that FAILS still writes its profile (its figures count; the failure is a gate matter, printed). A test process
# ended by a SIGNAL (for example a stack overflow under opt-level 0) writes NO profile: the package is NOT PROVED.
# It deletes nothing; it touches <S>.lock and <K>.lock (touch -c) every 60 s while it runs, and (OPS-10, 30 Oct) the
# tracked files of <S> once before its first cargo command.
# Usage (start it in the background as recipe step 0 says, with its log):
#   d3-route-b.sh <S tree> <K deep dir> <P new profile folder> <L new log folder> <set file> <pkg>... [-- <cargo test args after -->]
# Exit 0: every package measured and read (the figures may still show GAPs); 3: something NOT PROVED (see its lines).
set -o pipefail
W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925
D=$W/state/decisions-20261027
N=${D3_NIGHTLY:-$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin}; T=${D3_LLVM_BIN:-$N/lib/rustlib/aarch64-apple-darwin/bin}
CARGO_Q=${D3_CARGO:-$W/bin/cargo}
PIN_RUSTC='rustc 1.99.0-nightly (1a98b1e13 2026-08-07)'; PIN_LLVM='LLVM version 23.1.0-rust-1.99.0-nightly'
S=$1; K=$2; P=$3; L=$4; SET=$5; shift 5 2>/dev/null || { echo "usage: d3-route-b.sh <S> <K> <P> <L> <set file> <pkg>... [-- <test args>]"; exit 2; }
PKGS=(); TARGS=()
while [ $# -gt 0 ]; do if [ "$1" = -- ]; then shift; TARGS=("$@"); break; fi; PKGS+=("$1"); shift; done
[ ${#PKGS[@]} -gt 0 ] || { echo "usage: give at least one package"; exit 2; }
ts() { TZ=Asia/Kolkata date '+%F %T IST'; }
np=0; notp() { echo "ROUTE-B NOT PROVED: $*"; np=1; }
[ -d "$S/.git" ] || [ -f "$S/.git" ] || { echo "ROUTE-B REFUSED: $S is not a git tree"; exit 2; }
[ -s "$SET" ] || { echo "ROUTE-B REFUSED: no set file $SET"; exit 2; }
if [ -e "$P" ] && [ "$(ls -A "$P" | wc -l | tr -d ' ')" != 0 ]; then echo "ROUTE-B REFUSED: profile folder $P is not empty (recipe step 3: a NEW empty folder per measurement)"; exit 2; fi
mkdir -p "$P/build" "$L" || exit 2
echo "ROUTE-B START $(ts) tree $S tip $(git -C "$S" rev-parse HEAD) packages ${PKGS[*]} K $K P $P (entries before: 0)"
( while kill -0 $$ 2>/dev/null; do touch -c "$S.lock" "$K.lock" 2>/dev/null; sleep 60; done ) < /dev/null > /dev/null 2>&1 & BEAT=$!
trap 'kill $BEAT 2>/dev/null' EXIT
# 1. pins
rv=$("$N/bin/rustc" -V 2>&1); cv=$("$T/llvm-cov" --version 2>&1 | grep 'LLVM version'); pv=$("$T/llvm-profdata" --version 2>&1 | grep 'LLVM version')
echo "versions: $rv | llvm-cov ${cv# } | llvm-profdata ${pv# }"
[ "$rv" = "$PIN_RUSTC" ] || { echo "ROUTE-B ENVIRONMENT: rustc is '$rv', pin '$PIN_RUSTC' (install nothing; recipe step 9)"; exit 3; }
case "$cv" in *"$PIN_LLVM"*) ;; *) echo "ROUTE-B ENVIRONMENT: llvm-cov '$cv', pin '$PIN_LLVM'"; exit 3;; esac
case "$pv" in *"$PIN_LLVM"*) ;; *) echo "ROUTE-B ENVIRONMENT: llvm-profdata '$pv', pin '$PIN_LLVM'"; exit 3;; esac
export PATH=$W/bin:$N/bin:$PATH CARGO_QUEUE_REAL=$N/bin/cargo RUSTC=$N/bin/rustc RUSTDOC=$N/bin/rustdoc
export CARGO_TARGET_DIR=$K/cov RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' CARGO_PROFILE_DEV_OPT_LEVEL=0
cd "$S" || exit 2
# OPS-10 (30 Oct 2026, main session; R4-20 check 3 follow-up): the two deep dirs are shared by every part's tree, and
# cargo names a workspace member's artifacts by its path INSIDE the workspace and trusts mtimes, so another tree's
# artifacts can look fresh here. Touch every tracked file of $S (we hold its lock) before the first cargo command.
git ls-files -z | xargs -0 touch || { echo "ROUTE-B REFUSED: could not touch the tracked files of $S (OPS-10)"; exit 2; }
echo "OPS-10: every tracked file of $S touched $(ts)"
# 2. one instrumented build
PA=(); for p in "${PKGS[@]}"; do PA+=(-p "$p"); done
t0=$(date +%s); echo "== build START $(ts): cargo test ${PA[*]} --locked --no-run --message-format=json (RUSTFLAGS='$RUSTFLAGS', opt-level 0, CARGO_TARGET_DIR=$CARGO_TARGET_DIR)"
LLVM_PROFILE_FILE=$P/build/%p-%m.profraw "$CARGO_Q" test "${PA[@]}" --locked --no-run --message-format=json > "$L/build.json" 2> "$L/build.err"; rc=$?
echo "== build rc=$rc END $(ts) ($(( ($(date +%s) - t0) / 60 )) min); $(grep -c '"reason":"compiler-artifact"' "$L/build.json") artifacts, $(grep '"reason":"compiler-artifact"' "$L/build.json" | grep -c '"fresh":true') reused"
[ $rc = 0 ] || { tail -5 "$L/build.err"; echo "ROUTE-B NOT PROVED: the instrumented build failed (rc $rc; $L/build.err)"; exit 3; }
jq -r 'select(.executable != null) | .executable' "$L/build.json" | sort -u > "$P/objects.txt"
no=$(wc -l < "$P/objects.txt" | tr -d ' '); echo "objects: $no ($P/objects.txt, from this build's own report)"
[ "$no" -gt 0 ] || { echo "ROUTE-B NOT PROVED: the build reported no test executable"; exit 3; }
OBJ=("$(head -1 "$P/objects.txt")"); while IFS= read -r o; do OBJ+=("--object=$o"); done < <(tail -n +2 "$P/objects.txt")
# 3. per package
for C in "${PKGS[@]}"; do
  mkdir -p "$P/$C"; echo "profile $P/$C entries before: $(ls -A "$P/$C" | wc -l | tr -d ' ')"
  t0=$(date +%s); echo "== test $C START $(ts)"
  if [ ${#TARGS[@]} -gt 0 ]; then LLVM_PROFILE_FILE=$P/$C/%p-%m.profraw "$CARGO_Q" test -p "$C" --locked --no-fail-fast -- "${TARGS[@]}" > "$L/test-$C.log" 2>&1; rc=$?
  else LLVM_PROFILE_FILE=$P/$C/%p-%m.profraw "$CARGO_Q" test -p "$C" --locked --no-fail-fast > "$L/test-$C.log" 2>&1; rc=$?; fi
  echo "== test $C rc=$rc END $(ts) ($(( ($(date +%s) - t0) / 60 )) min); $(grep -E '^test result' "$L/test-$C.log" | awk '{p+=$4; f+=$6; i+=$8} END {print "tests passed " p+0 " failed " f+0 " ignored " i+0}')"
  grep -E '^test .* FAILED$|^failures:$' "$L/test-$C.log" | grep -v '^failures:$' | head -20 | sed 's/^/  FAILED (a gate matter; its profile still counts): /'
  sig=$(grep -E "process didn't exit successfully.*\(signal: " "$L/test-$C.log" | head -3)
  [ -z "$sig" ] || notp "$C: a test process ended by a signal and wrote no profile, so its tests are missing from the figures: $sig"
  find "$P/$C" -name '*.profraw' > "$P/$C/profraw.list"; nr=$(wc -l < "$P/$C/profraw.list" | tr -d ' '); echo "profraw files: $nr"
  [ "$nr" -gt 0 ] || { notp "$C: no profile written"; continue; }
  "$T/llvm-profdata" merge -sparse -f "$P/$C/profraw.list" -o "$P/$C/merged.profdata" > "$L/merge-$C.log" 2>&1 || { notp "$C: llvm-profdata merge failed ($L/merge-$C.log)"; continue; }
  echo "merged $P/$C/merged.profdata"
  files=$(awk -v c="crates/$C/" '$1 == "coverage" && index($2, c) == 1 {print $2}' "$SET" | sort -u)
  [ -n "$files" ] || echo "note: the set has no coverage line under crates/$C/"
  for f in $files; do
    b=$(printf '%s' "$f" | tr '/' '_')
    "$T/llvm-cov" show --show-branches=count --show-line-counts-or-regions --path-equivalence="$S/crates/$C/src/..,$S/crates/$C" --instr-profile="$P/$C/merged.profdata" "${OBJ[@]}" "$S/$f" > "$L/show-$C-$b.txt" 2> "$L/show-$C-$b.err"
    node "$D/d3-covset.mjs" "$SET" "$f" "$L/show-$C-$b.txt" | tee "$L/figures-$C-$b.txt"; [ "${PIPESTATUS[0]}" = 0 ] || notp "$f: not read ($L/show-$C-$b.err)"
  done
done
others=$(awk -v ps="${PKGS[*]}" 'BEGIN { n = split(ps, p, " ") } $1 == "coverage" { ok = 0; for (i = 1; i <= n; i++) if (index($2, "crates/" p[i] "/") == 1) ok = 1; if (!ok) print $2 }' "$SET" | sort -u)
[ -z "$others" ] || echo "not measured by this call (another package, or Route C/M2 outside crates/): $(echo $others)"
if [ $np = 0 ]; then echo "ROUTE-B DONE $(ts): every package measured and read (${PKGS[*]})"; exit 0; fi
echo "ROUTE-B DONE $(ts) WITH NOT PROVED lines above"; exit 3
`````
