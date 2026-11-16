# JOB-LINUX-TESTS: run the engine v3.4 test suite on Linux (job J1, box 0 of 1)

Labels: MEASURED = a number from a named run. ESTIMATE = measured inputs times a model. EXACT = a count. No figure here is a
trading result.

## The job

| item | value |
|---|---|
| repository | `SJParthi/brutex` on GitHub (public) |
| code branch | `cloud/spikefade-v34int-20261116`: one orphan commit. Your first message names its sha. |
| job and box | job `v34-tests`, box k = 0 of n = 1 |
| results branch | `cloud-results/v34-tests-0-of-1`. It is NEW: you create it. |

Build the engine's test binary from this branch exactly as the Mac built it. Then run the Mac's test command.
- Record the status of every test name.
- Compare the statuses with the Mac's list `MAC-TESTS.txt`.
  - The list holds 217 names, MEASURED on the Mac by integrate:D on 16 Nov 2026: 216 passed, 0 failed, 1 ignored.
  - The ignored test is `split18::tests::v32b_split_cost_97_months`.

**The Mac's command.** It comes from integrate:D's log `v34/integrate/build/t11.log`, which ran on the final source with
manifest `5f1de014...`:

```
cargo test --release --offline -p spikefade -- --test-threads 4
```

It ran with `CARGO_BUILD_JOBS=4` and `RUST_TEST_THREADS=4`, and no `SF_*` variable was set.

**This box runs the same command with 2 additions:**
- `--locked`. It refuses any change to `Cargo.lock` and has no other effect.
- `--exact --skip NAME` for the 10 tests that need data this branch does not hold (section 8).

**What changed from the last wave's method** (the boxes of 16 Nov 2026 on `cloud/spikefade-shard-20261116`):

| | last wave | this job |
|---|---|---|
| what runs | a dev-profile `cargo test --locked --workspace` baseline before cargo-mutants | the Mac's own release command for the engine package, and no mutants |
| data skips | 7 | 10. The 3 new ones are v3.4's real-table tests. |
| the 2 zstd tests | skipped | run, and reported as KNOWN-ZSTD-PATH |
| failures | not re-run | every failure re-run alone, to tell contention from a real difference |

## 1. Rules (the whole job)

- **Code only.**
  - Never fetch market data.
  - The only network use is apt, rustup and GitHub (the clone and your pushes).
  - Never print a secret, a token or a signed link. This job has none.
- **Push only the results.**
  - Push ONLY the folder `results/v34-tests/box-0-of-1/`, and ONLY to the NEW branch `cloud-results/v34-tests-0-of-1`.
  - Never change a code file, never push another branch, never force-push.
- **Keep your turn busy until the job ends.** An idle cloud box pauses and loses background work.
  - Run `run.sh` with nohup and poll it at most every 270 s with short commands (section 3).
  - Never end the turn while it runs.
- **Progress commits:**
  - the first push after step 2c;
  - then at least one every 15 minutes while the script runs;
  - one more at the end.

  `push.sh` makes each one (section 5).
- **Stop rule.** Stop if a step of sections 2-3 fails: the tools, the clone, the commit sha, the code manifest, the
  toolchain or the build.
  - Push what the results folder holds. `SUMMARY.txt` then says STOPPED and why.
  - Then report.
  - A failing TEST is a result, not a stop: the script finishes every stage.
- **Environment:** set no `SF_*` environment variable. The script unsets any it finds.

## 2. Set up (foreground commands)

**2a. Tools,** as on the Mac: zstd, unzip, ps, `/usr/bin/shasum` (from the perl package), git and jq. When sudo is missing,
run as root without it.

```bash
SUDO=$(command -v sudo || true)
$SUDO apt-get update -q && $SUDO apt-get install -y -q zstd unzip procps perl git jq
command -v cargo || { curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal && . ~/.cargo/env; }
```

A tool that is still missing after this shows as MISSING in `box.txt`. A test that fails because a tool is missing is a box
fault, not a code finding: say so in the final message.

**2b. The code branch, the results branch and the two scripts.** Replace `<SHA>` with the commit sha of your first
message.

```bash
cd ~ && git clone --branch cloud/spikefade-v34int-20261116 --single-branch https://github.com/SJParthi/brutex.git v34t && cd ~/v34t
[ "$(git rev-parse HEAD)" = "<SHA>" ] && echo "commit OK" || echo "commit DIFFERS: STOP"
git checkout -b cloud-results/v34-tests-0-of-1
rustc -V
mkdir -p ~/v34t-run
awk '/^```bash run.sh$/{f=1; next} f && /^```$/{f=0} f' JOB-LINUX-TESTS.md > ~/v34t-run/run.sh
awk '/^```bash push.sh$/{f=1; next} f && /^```$/{f=0} f' JOB-LINUX-TESTS.md > ~/v34t-run/push.sh
sha256sum ~/v34t-run/run.sh ~/v34t-run/push.sh
```

**What each command must print:**
- `rustc -V` must print `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
  - `rust-toolchain.toml` makes rustup install 1.97.1 at that point, with rustfmt, clippy, llvm-tools-preview and the
    wasm32 target. ESTIMATE: 1-3 min.
- The two script hashes must be:
  - `run.sh` = `c570a12976e8a30457096f940e246174d0bf0dd2008270da732cfbfd31a33759`
  - `push.sh` = `a1f43c7760235223aa55061db4126732f8c9f7ad66c08c68a562ba1a0f840866`

Any other output: STOP.

**2c. The first progress commit.**

```bash
mkdir -p ~/v34t/results/v34-tests/box-0-of-1
echo "$(date -u +%FT%TZ) set up: $(cd ~/v34t && rustc -V); commit $(git -C ~/v34t rev-parse HEAD)" >> ~/v34t/results/v34-tests/box-0-of-1/progress.md
bash ~/v34t-run/push.sh
```

## 3. Run (detached) and poll (keeps the turn busy)

```bash
cd ~ && nohup bash ~/v34t-run/run.sh <SHA> > ~/v34t-run/run.out 2>&1 < /dev/null &
```

**Poll** with this command, which waits at most 260 s:

```bash
for i in $(seq 1 26); do case "$(cat ~/v34t-run/state 2>/dev/null)" in DONE|STOPPED) break;; esac; sleep 10; done; cat ~/v34t-run/state; tail -n 3 ~/v34t-run/run.out
```

Repeat the poll until the state is `DONE` or `STOPPED`.

**When to push** (`bash ~/v34t-run/push.sh`):
- after a poll when 15 min or more have passed since the last push;
- after the poll that first shows state `E` (the build has finished);
- at `DONE` or `STOPPED`.

**States:** `B` checkout, `C` box facts, `D` build, `E` test list, `F` the Mac's command, `G` failures re-run alone, `H1` and
`H2` diagnostics, `I` comparison, then `DONE` or `STOPPED`.

**If the VM lost the job:** the state has not changed for 30 min and `pgrep -f 'cargo|rustc'` finds nothing. Run the nohup
line again. The script is safe to re-run.

**If foreground `sleep` is refused,** poll with your session's own wait tool. Still wait at most 270 s at a time.

## 4. run.sh (extracted by step 2b; do not edit it)

```bash run.sh
#!/usr/bin/env bash
# run.sh: job v34-tests (J1), box 0 of 1. The engine v3.4 test suite on Linux, compared with the Mac's list.
# Extracted from JOB-LINUX-TESTS.md section 4. Usage, from anywhere: bash ~/v34t-run/run.sh <commit sha of the first message>
# Reads the checkout ~/v34t (branch cloud/spikefade-v34int-20261116). Writes ~/v34t/results/v34-tests/box-0-of-1/ and ~/v34t-run/.
# Never fetches data, never pushes (the session pushes), never edits a code file.
SHA_WANT=$1
REPO=$HOME/v34t
RUN=$HOME/v34t-run
RES=$REPO/results/v34-tests/box-0-of-1
CARGO=${V34T_CARGO:-cargo}
MAN_N=151
MAN_SHA=80fa426f2370a97377d5d35600db260af36b30f488526920f6e539cc84a7a34a
RUSTC_WANT='rustc 1.97.1 (8bab26f4f 2026-07-14)'
MAC_IGNORED='split18::tests::v32b_split_cost_97_months'
SKIPS='menu::tests::sf3_menu_and_sets_load_with_the_signed_counts
menu::tests::sf3_menu_fail_closed
menu::tests::sf3_distance_floor_counted
menu::tests::sf3_tp_floor_not_armed_at_or_above_entry
menu::tests::sf3_tp_theta_level_and_absence
tests3::sf3_ttp_replaces_tp_close
tests33s::v33_search1d_sets_load_and_count
tests34i::v34i_f3_real_rows_spot
tests34i::v34i_f7_rules_tables_load_dated_and_minrange
tests34i::v34i_f7_opr_file_is_next_session_and_columns'
mkdir -p "$RUN" "$RES/alone" "$RES/diag"
p() { printf '%s %s\n' "$(date -u +%FT%TZ)" "$*" | tee -a "$RES/progress.md"; }
st() { echo "$1" > "$RUN/state"; }
die() {
  p "STOP: $*"
  printf 'SUMMARY job=v34-tests box=0/1 status=STOPPED reason="%s" commit=%s total_wall=%ss\n' "$*" "$(git -C "$REPO" rev-parse HEAD 2>/dev/null)" "$(( $(date +%s) - T0 ))" > "$RES/SUMMARY.txt"
  st STOPPED
  exit 1
}
first_msg() { awk '/panicked at/{getline; print; exit}' "$1" | tr '\t' ' ' | cut -c1-200; }
T0=$(date +%s)
st B
cd "$REPO" || die "no checkout at $REPO"
for v in $(env | sed -n 's/^\(SF_[A-Za-z0-9_]*\)=.*/\1/p'); do unset "$v"; done
export CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4

# B: the checkout is the commit of the first message and its code manifest is the expected one
HEAD_SHA=$(git rev-parse HEAD)
[ "$HEAD_SHA" = "$SHA_WANT" ] || die "HEAD $HEAD_SHA is not the commit of the first message ($SHA_WANT)"
LC_ALL=C find Cargo.toml Cargo.lock rust-toolchain.toml crates pinned -type f ! -path '*/target/*' | LC_ALL=C sort | xargs sha256sum > "$RES/SOURCE-SHA256-v34int.txt"
n=$(wc -l < "$RES/SOURCE-SHA256-v34int.txt" | tr -d ' ')
m=$(sha256sum "$RES/SOURCE-SHA256-v34int.txt" | cut -c1-64)
{ [ "$n" = "$MAN_N" ] && [ "$m" = "$MAN_SHA" ]; } || die "code manifest is $n files $m, expected $MAN_N files $MAN_SHA"
p "B checkout $HEAD_SHA, code manifest $n files $m: OK"

# C: the box and the toolchain
st C
{
  echo "uname: $(uname -a)"
  echo "nproc: $(nproc)"
  grep MemTotal /proc/meminfo
  echo "cgroup memory.max: $(cat /sys/fs/cgroup/memory.max 2>/dev/null || echo n/a)"
  echo "disk: $(df -P -BG . | tail -1)"
  grep -m1 'model name' /proc/cpuinfo
  echo "rustc: $(rustc -V)"
  echo "cargo: $($CARGO -V)"
  for t in zstd unzip gzip ps hostname shasum date jq; do printf 'tool %s: %s\n' "$t" "$(command -v "$t" || echo MISSING)"; done
  echo "user: uid $(id -u)"
} > "$RES/box.txt" 2>&1
RUSTC=$(rustc -V 2>&1)
[ "$RUSTC" = "$RUSTC_WANT" ] || die "rustc is '$RUSTC', expected '$RUSTC_WANT'"
p "C box facts in box.txt; $RUSTC"

# D: build the test binary (the Mac command's profile, --no-run)
st D
p "D build start: cargo test --release --locked --offline -p spikefade --no-run (CARGO_BUILD_JOBS=4)"
s=$(date +%s)
timeout 3600 $CARGO test --release --locked --offline -p spikefade --no-run > "$RES/build.log" 2>&1
rc=$?
BUILD_S=$(( $(date +%s) - s ))
[ $rc -eq 124 ] && die "build hit its 60 min timeout (build.log)"
[ $rc -eq 0 ] || die "build rc $rc after ${BUILD_S}s (build.log)"
p "D build rc 0, wall ${BUILD_S}s (MEASURED)"

# E: every test name the Linux binary holds, against the Mac list
st E
timeout 600 $CARGO test --release --locked --offline -p spikefade -- --list > "$RES/list.txt" 2> "$RUN/list.err" || die "test --list failed (list.err in ~/v34t-run)"
sed -n 's/: test$//p' "$RES/list.txt" | LC_ALL=C sort > "$RES/linux-names.txt"
LC_ALL=C sort "$REPO/MAC-TESTS.txt" > "$RUN/mac.sorted"
comm -23 "$RUN/mac.sorted" "$RES/linux-names.txt" > "$RES/only-mac.txt"
comm -13 "$RUN/mac.sorted" "$RES/linux-names.txt" > "$RES/only-linux.txt"
p "E list: Linux $(wc -l < "$RES/linux-names.txt" | tr -d ' ') names, Mac $(wc -l < "$RUN/mac.sorted" | tr -d ' '); only on the Mac $(wc -l < "$RES/only-mac.txt" | tr -d ' '), only on Linux $(wc -l < "$RES/only-linux.txt" | tr -d ' ')"

# F: the Mac's command (+ --locked) with the 10 data tests skipped by exact name
st F
printf '%s\n' "$SKIPS" > "$RES/skipped.txt"
set --
for t in $SKIPS; do set -- "$@" --skip "$t"; done
p "F run start: cargo test --release --locked --offline -p spikefade -- --test-threads 4 --exact + 10 x --skip (skipped.txt)"
s=$(date +%s)
timeout 1800 $CARGO test --release --locked --offline -p spikefade -- --test-threads 4 --exact "$@" > "$RES/test-run.log" 2>&1
rc=$?
RUN_S=$(( $(date +%s) - s ))
awk '$1=="test" && $3=="..." {x=$4; sub(/,$/, "", x); print $2 "\t" x}' "$RES/test-run.log" > "$RUN/status.tsv"
awk -F'\t' '$2=="ok"{print $1}' "$RUN/status.tsv" | LC_ALL=C sort > "$RES/passed.txt"
awk -F'\t' '$2=="FAILED"{print $1}' "$RUN/status.tsv" | LC_ALL=C sort > "$RES/failed.txt"
awk -F'\t' '$2=="ignored"{print $1}' "$RUN/status.tsv" | LC_ALL=C sort > "$RES/ignored.txt"
grep -E '^test .* has been running for over' "$RES/test-run.log" | awk '{print $2}' | LC_ALL=C sort -u > "$RES/timeout.txt"
[ $rc -eq 124 ] && die "the test run hit its 30 min timeout (test-run.log; still running: timeout.txt)"
NP=$(wc -l < "$RES/passed.txt" | tr -d ' '); NF=$(wc -l < "$RES/failed.txt" | tr -d ' '); NI=$(wc -l < "$RES/ignored.txt" | tr -d ' ')
p "F run rc $rc, wall ${RUN_S}s (MEASURED): passed $NP, failed $NF, ignored $NI; $(grep -m1 '^test result:' "$RES/test-run.log")"

# G: every failed test re-run ALONE once (a failure only under the parallel suite = contention, as the Mac saw once)
st G
: > "$RUN/alone.tsv"
i=0
while IFS= read -r t; do
  [ -n "$t" ] || continue
  i=$((i + 1))
  timeout 600 $CARGO test --release --locked --offline -p spikefade -- --test-threads 1 --exact "$t" > "$RES/alone/$i.log" 2>&1 < /dev/null
  rc=$?
  if grep -qF "test $t ... ok" "$RES/alone/$i.log"; then a=ok; elif [ $rc -eq 124 ]; then a=TIMEOUT; else a=FAILED; fi
  blk=$(awk -v h="---- $t stdout ----" '$0==h{f=1; next} f && (/^---- / || /^failures:/){exit} f' "$RES/test-run.log")
  if printf '%s\n' "$blk" | grep -qF '/opt/homebrew/bin/zstd'; then c=KNOWN-ZSTD-PATH; elif [ "$a" = ok ]; then c=CONTENTION; else c=DIFFERS-ON-LINUX; fi
  msg=$(printf '%s\n' "$blk" | awk '/panicked at/{getline; print; exit}' | tr '\t' ' ' | cut -c1-200)
  printf '%s\t%s\t%s\t%s\n' "$t" "$a" "$c" "alone/$i.log: $msg" >> "$RUN/alone.tsv"
  p "G alone $t: $a ($c)"
done < "$RES/failed.txt"

# H1: the 10 skipped data tests run alone once, to show each fails here for its missing file only (a diagnostic, never a result)
st H1
: > "$RUN/skipdiag.tsv"
i=0
for t in $SKIPS; do
  i=$((i + 1))
  timeout 600 $CARGO test --release --locked --offline -p spikefade -- --test-threads 1 --exact "$t" > "$RES/diag/skip-$i.log" 2>&1 < /dev/null
  if grep -qF "test $t ... ok" "$RES/diag/skip-$i.log"; then r="ok (UNEXPECTED: it ran without its data)"; else r="FAILED as expected without its data: $(first_msg "$RES/diag/skip-$i.log")"; fi
  printf '%s\t%s\n' "$t" "diag/skip-$i.log: $r" >> "$RUN/skipdiag.tsv"
done
p "H1 skipped tests run alone: $(grep -c 'FAILED as expected' "$RUN/skipdiag.tsv") of 10 fail without their data"

# H2: the KNOWN zstd cause, proved: the Mac path made to exist (a symlink to this box's zstd), the known tests re-run alone
st H2
KN=$(awk -F'\t' '$3=="KNOWN-ZSTD-PATH"{print $1}' "$RUN/alone.tsv")
: > "$RUN/zstddiag.tsv"
if [ -n "$KN" ]; then
  Z=$(command -v zstd)
  if [ "$(uname -s)" != Linux ]; then S=NONE; elif [ "$(id -u)" = 0 ]; then S=""; elif sudo -n true 2>/dev/null; then S="sudo -n"; else S=NONE; fi
  if [ -z "$Z" ] || [ "$S" = NONE ]; then
    p "H2 skipped: zstd '$Z', root or sudo: $S"
  else
    [ -e /opt/homebrew/bin/zstd ] || { $S mkdir -p /opt/homebrew/bin && $S ln -s "$Z" /opt/homebrew/bin/zstd; }
    p "H2 /opt/homebrew/bin/zstd -> $Z ($(zstd --version 2>&1 | head -1))"
    i=0
    for t in $KN; do
      i=$((i + 1))
      timeout 600 $CARGO test --release --locked --offline -p spikefade -- --test-threads 1 --exact "$t" > "$RES/diag/zstd-$i.log" 2>&1 < /dev/null
      if grep -qF "test $t ... ok" "$RES/diag/zstd-$i.log"; then r=ok; else r="FAILED: $(first_msg "$RES/diag/zstd-$i.log")"; fi
      printf '%s\t%s\n' "$t" "diag/zstd-$i.log with the Mac path present: $r" >> "$RUN/zstddiag.tsv"
      p "H2 $t with the Mac path present: $r"
    done
  fi
fi

# I: one row per test name (Mac list and Linux list together), the comparison and the summary
st I
LC_ALL=C sort -u "$RUN/mac.sorted" "$RES/linux-names.txt" > "$RUN/all.txt"
awk -F'\t' -v OFS='\t' -v ign="$MAC_IGNORED" '
  FILENAME == ARGV[1] { mac[$1] = 1; next }
  FILENAME == ARGV[2] { lin[$1] = 1; next }
  FILENAME == ARGV[3] { st[$1] = $2; next }
  FILENAME == ARGV[4] { sk[$1] = 1; next }
  FILENAME == ARGV[5] { al[$1] = $2; cl[$1] = $3; nt[$1] = $4; next }
  FILENAME == ARGV[6] { sd[$1] = $2; next }
  FILENAME == ARGV[7] { zd[$1] = $2; next }
  {
    t = $1
    m = (t in mac) ? ((t == ign) ? "ignored" : "ok") : "-"
    l = (t in st) ? st[t] : ((t in sk) ? "SKIPPED-DATA" : ((t in lin) ? "NOT-RUN" : "-"))
    a = (t in al) ? al[t] : "-"
    note = "-"
    if (!(t in mac)) c = "ONLY-LINUX"
    else if (!(t in lin)) c = "ONLY-MAC"
    else if (l == "SKIPPED-DATA") { c = "SKIPPED-DATA"; note = (t in sd) ? sd[t] : "-" }
    else if (l == m) c = "SAME"
    else if (l == "FAILED") { c = (t in cl) ? cl[t] : "DIFFERS-ON-LINUX"; note = (t in nt) ? nt[t] : "-"; if (t in zd) note = note " | " zd[t] }
    else if (l == "NOT-RUN") c = "NOT-RUN"
    else c = "DIFFERS-ON-LINUX"
    print t, m, l, a, c, note
  }' "$RUN/mac.sorted" "$RES/linux-names.txt" "$RUN/status.tsv" "$RES/skipped.txt" "$RUN/alone.tsv" "$RUN/skipdiag.tsv" "$RUN/zstddiag.tsv" "$RUN/all.txt" > "$RES/tests-linux.tsv"
{
  echo "# one row per test: name, Mac status (t11.log), Linux status, Linux status alone, class, note"
  awk -F'\t' '$5 != "SAME"' "$RES/tests-linux.tsv"
} > "$RES/compare.txt"
jq -R -s 'split("\n") | map(select(length > 0) | split("\t") | {name: .[0], mac: .[1], linux: .[2], alone: .[3], class: .[4], note: .[5]})' "$RES/tests-linux.tsv" > "$RES/outcomes.json" || p "I outcomes.json not written (jq missing?)"
cnt() { awk -F'\t' -v c="$1" '$5 == c' "$RES/tests-linux.tsv" | wc -l | tr -d ' '; }
echo "memory.peak: $(cat /sys/fs/cgroup/memory.peak 2>/dev/null || echo n/a)" >> "$RES/box.txt"
TOT=$(( $(date +%s) - T0 ))
printf 'SUMMARY job=v34-tests box=0/1 status=DONE commit=%s manifest=%s/%s rustc="%s" mac=%s(216 ok, 1 ignored) linux_listed=%s passed=%s failed=%s ignored=%s skipped_data=%s same=%s known_zstd=%s contention=%s differs_on_linux=%s not_run=%s only_mac=%s only_linux=%s build_wall=%ss run_wall=%ss total_wall=%ss\n' \
  "$HEAD_SHA" "$n" "$m" "$RUSTC" "$(wc -l < "$RUN/mac.sorted" | tr -d ' ')" "$(wc -l < "$RES/linux-names.txt" | tr -d ' ')" "$NP" "$NF" "$NI" "$(cnt SKIPPED-DATA)" "$(cnt SAME)" \
  "$(cnt KNOWN-ZSTD-PATH)" "$(cnt CONTENTION)" "$(cnt DIFFERS-ON-LINUX)" "$(cnt NOT-RUN)" "$(cnt ONLY-MAC)" "$(cnt ONLY-LINUX)" "$BUILD_S" "$RUN_S" "$TOT" > "$RES/SUMMARY.txt"
p "I $(cat "$RES/SUMMARY.txt")"
st DONE
```

## 5. push.sh (extracted by step 2b; do not edit it)

```bash push.sh
#!/usr/bin/env bash
# push.sh: one progress commit of job v34-tests, box 0 of 1 (the results folder ONLY), and its push to the NEW branch
# cloud-results/v34-tests-0-of-1. Extracted from JOB-LINUX-TESTS.md section 5. Usage: bash ~/v34t-run/push.sh
cd "$HOME/v34t" || exit 2
[ "$(git rev-parse --abbrev-ref HEAD)" = cloud-results/v34-tests-0-of-1 ] || { echo "STOP: not on cloud-results/v34-tests-0-of-1"; exit 3; }
git add results/v34-tests/box-0-of-1 || exit 2
bad=$(git diff --cached --name-only | grep -v '^results/v34-tests/box-0-of-1/')
[ -z "$bad" ] || { echo "STOP: staged files outside the results folder: $bad"; exit 3; }
if git grep -I -q -i -E --cached -e 'x-am[z]-(signature|credential|security-token)'; then echo "STOP: a signed-link marker is staged"; exit 3; fi
git diff --cached --quiet || git commit -q -m "v34-tests box 0/1: results, stage $(cat "$HOME/v34t-run/state" 2>/dev/null || echo setup)"
git push -q -u origin cloud-results/v34-tests-0-of-1 && echo "pushed $(git rev-parse HEAD) at $(date -u +%FT%TZ)"
```

## 6. Result files (`results/v34-tests/box-0-of-1/` on `cloud-results/v34-tests-0-of-1`)

| file | what it holds |
|---|---|
| `SUMMARY.txt` | ONE line: status, the commit sha tested, the code manifest (count/sha256), rustc, and the counts (Mac names, Linux names, passed, failed, ignored, skipped for data, SAME, KNOWN-ZSTD-PATH, CONTENTION, DIFFERS-ON-LINUX, NOT-RUN, ONLY-MAC, ONLY-LINUX). It also holds the build, run and total wall times (MEASURED, seconds). |
| `tests-linux.tsv` | One row per test name of the Mac list and the Linux list together. Columns: name, Mac status, Linux status, status alone, class, note. |
| `outcomes.json` | The same rows as JSON objects `{name, mac, linux, alone, class, note}`. |
| `compare.txt` | Every row whose class is not SAME. |
| `passed.txt`, `failed.txt`, `ignored.txt` | Test names by their status in the Mac's command (step F). |
| `skipped.txt` | The 10 data tests, skipped by exact name. |
| `timeout.txt` | Tests libtest reported as running for over 60 s. Normally empty. |
| `list.txt`, `linux-names.txt` | The binary's own `--list`, and the names from it. |
| `only-mac.txt`, `only-linux.txt` | Names in one list and not the other. Normally both are empty. |
| `test-run.log`, `build.log` | The full outputs of steps F and D. |
| `alone/N.log` | Step G: each failed test, re-run alone. |
| `diag/skip-N.log`, `diag/zstd-N.log` | Steps H1 and H2. |
| `box.txt` | The box: uname, nproc, MemTotal, cgroup memory.max and memory.peak, disk, CPU model, rustc, cargo, and the tool paths. |
| `SOURCE-SHA256-v34int.txt` | The code manifest. It must have 151 lines, and its own sha256 must be `80fa426f2370a97377d5d35600db260af36b30f488526920f6e539cc84a7a34a`. |
| `progress.md` | One UTC-stamped line per stage. |

This is a TEST run, not a mutation run. Its passed, failed, ignored and skipped lists take the place of the mutation files
(caught, missed, timeout, unviable); those belong to JOB-MUTANTS.md.

## 7. Classes in `tests-linux.tsv`

| class | meaning |
|---|---|
| SAME | Linux status = Mac status: ok = ok, or ignored = ignored. |
| SKIPPED-DATA | One of the 10 tests skipped for data (section 8). Step H1 runs each one alone once. It is expected to FAIL here, and its note gives the first panic line. |
| KNOWN-ZSTD-PATH | It fails, and its message names `/opt/homebrew/bin/zstd`. This is the known Mac-only path at kit.rs:470 (section 8). Step H2 re-runs it with that path present. |
| CONTENTION | It fails in the parallel suite and passes alone. On 16 Nov the Mac saw this once, for a timing gate under load. |
| DIFFERS-ON-LINUX | It fails alone too, or has some other status than the Mac's. **This is the finding the job looks for.** |
| NOT-RUN | The binary lists it, but step F did not report it. |
| ONLY-MAC, ONLY-LINUX | The name is in one list only. |

## 8. Why 10 tests are skipped and 2 are known to fail

The branch is code only. Its README, section 2, says what was left out.

**Skipped for data (10).** Each one reads a file that is not in this branch:

| test | reads |
|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | `crates/spikefade/data/EXITS-DYN-MENU-v1.tsv` and `EXITS-DYN-SETS-v1.tsv` (exit-rule parameter tables) |
| `menu::tests::sf3_menu_fail_closed` | the same |
| `menu::tests::sf3_distance_floor_counted` | `crates/spikefade/data/EXITS-DYN-MENU-v1.tsv` |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | the same |
| `tests3::sf3_ttp_replaces_tp_close` | the same |
| `tests33s::v33_search1d_sets_load_and_count` | `crates/spikefade/data/EXITS-DYN-MENU-v1.tsv` and `EXITS-DYN-SETS-v33-1D.tsv` |
| `tests34i::v34i_f3_real_rows_spot` | the dated NSE freeze table `STOCK-FREEZE-DATED.tsv`, at a Mac path (tests34i.rs line 14) |
| `tests34i::v34i_f7_rules_tables_load_dated_and_minrange` | the table `F7-PROTECTION-READINGS.tsv`, at the same Mac path |
| `tests34i::v34i_f7_opr_file_is_next_session_and_columns` | the table `F7-OPR-SESSIONS.tsv`, and the NSE contract files it names |

The first 7 are the 7 the last wave skipped. The last 3 are new in v3.4.

**Known to fail on Linux (2), and run anyway:**
- `tests34s::root_digest_same_for_every_form`
- `tests34s::sfz_roundtrip_all_rows`

Both run with every kit switch off. Without PORTABLE, the engine calls zstd at the Mac path `/opt/homebrew/bin/zstd`
(`crates/spikefade/src/kit.rs` line 470). That path is what made the last wave's C2 check read DIFFER.
- The fix is in a later tree (the determinism fix), not in v3.4.
- In the last wave these two tests were skipped under their old module name `tests34`.
- Step H2 makes that path exist, as a symlink to this box's zstd, and re-runs the two tests alone. Then a pass shows the path
  is the only cause.
- H2 runs only on Linux, as root or with sudo that needs no password.

**Do not work around anything else.** A failure outside these classes is the finding: report it.

## 9. Final message

Give these, in this order:
1. the `SUMMARY.txt` line;
2. every row of `compare.txt`, with its class and note;
3. the build wall and the total wall (MEASURED), and `box.txt`'s nproc, MemTotal, cgroup memory.max, memory.peak and CPU
   model;
4. the results branch and the sha of its last commit;
5. anything that did not go as this file says.

## 10. Expected wall (ESTIMATE)

| step | ESTIMATE |
|---|---|
| tools | 2-5 min |
| clone | under 1 min |
| the 1.97.1 toolchain | 1-3 min |
| build (D) | 5-15 min |
| steps E-I | 1-3 min |
| pushes | under 1 min |
| **total** | **10-25 min** |

**Basis:**
- MEASURED on the Mac: a clean release build of the v3.3 shard-kit binary took 3 min 14 s at `-j 4`, under load.
- MEASURED on the Mac: the v3.4 suite took 2.55 s (t11.log).
- MEASURED on the last wave's boxes: the dev-profile engine suite took 5.0 s.
- The release test build of v3.4 on 4 Linux vCPUs has never been measured. That is why the build row is a range.

**Timeouts in the script:**

| step | timeout |
|---|---|
| build | 60 min |
| test list | 10 min |
| the Mac's command | 30 min |
| each single-test run | 10 min |
