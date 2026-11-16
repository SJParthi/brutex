# JOB-MUTANTS: engine v3.4, cargo-mutants, slice K of 30

Branch `cloud/spikefade-v34int-20261116`, at the commit your first message names. **CODE ONLY.** The repository is public:
every push publishes.

Labels: EXACT = a count. MEASURED = a number from a named run. ESTIMATE = measured inputs times a model. Nothing here is a
trading result.

## 0. The job

You are box K (K = 0 to 29) of 30. Run cargo-mutants 27.1.0 on the whole workspace, slice `--shard K/30`, with the last
wave's flags (`--jobs 2 --cargo-arg=--locked` and the exact-name skips of step 4). Push the results, and only the results,
to the NEW branch `cloud-results/v34-mutants-K-of-30`.

Read this whole file once first. Then do the steps in order, and run every block in bash as written. The first line of
section 1b is the only place where you type K and the commit.

## 1. Rules

- **Code only.** Never fetch, read or write market data. Never fetch, use or print a presigned link. Never print a secret,
  a token or an environment dump.
- **Network:** this repository on github.com, and the Rust downloads (rustup, and crates.io for `cargo install`). Nothing
  else.
- **Results only.** Push only files under `results/mutants/shard-K-of-30/`, and only to `cloud-results/v34-mutants-K-of-30`.
  - Never change code.
  - Never push to another branch, never force, never delete a branch.
- **Keep your turn busy until your final message.** An idle session's VM pauses and loses background work.
  - Start every long command with `bg.sh` (it runs it under nohup).
  - Then wait in the FOREGROUND with `wait.sh`. Each call returns within 270 s; give your Bash tool a 300 s timeout for it.
  - Push a progress commit at least every 15 minutes (step 6).
- **Stop on any failure this file does not cover.** Run `bash ~/final.sh "STOPPED <reason>"` (step 9), then report.
- **A fresh VM** (`~/mut` is gone after a pause): start again at section 1b. `mkbranch.sh` then continues your pushed
  branch, and the final commit replaces any progress files.

## 1b. Your values, then the helper scripts

Put your K and the 40-hex commit of your first message in the first line, then run the whole block once.

```bash
K=__K__; SHA=__COMMIT__
date -u +%FT%TZ > ~/box.start
printf 'export PATH="$HOME/.cargo/bin:$PATH"\nK=%s\nSHA=%s\nBR=cloud-results/v34-mutants-%s-of-30\nR=results/mutants/shard-%s-of-30\n' "$K" "$SHA" "$K" "$K" > ~/job.env
cat > ~/bg.sh <<'EOF'
#!/bin/bash
# bg.sh NAME CMD...: run CMD under nohup; output to ~/NAME.log, exit code to ~/NAME.exit, pid to ~/NAME.pid
n=$1; shift
: > ~/"$n".exit
nohup bash -c '"$@"; echo $? > ~/'"$n"'.exit' _ "$@" > ~/"$n".log 2>&1 &
echo $! > ~/"$n".pid
echo "started $n pid $(cat ~/"$n".pid)"
EOF
cat > ~/wait.sh <<'EOF'
#!/bin/bash
# wait.sh NAME [OUTDIR]: wait at most 260 s for job NAME, then print its state, the tail of its log and the outcome counts
n=$1; o=${2:-}
timeout 260 tail --pid="$(cat ~/"$n".pid)" -f /dev/null
if [ -s ~/"$n".exit ]; then echo "$n ENDED exit=$(cat ~/"$n".exit)"; else echo "$n RUNNING"; fi
date -u +%FT%TZ; tail -n 3 ~/"$n".log | cut -c1-300
if [ -n "$o" ]; then for f in caught missed timeout unviable; do printf '%s=%s ' "$f" "$(cat "$o/mutants.out/$f.txt" 2>/dev/null | grep -c .)"; done; echo; fi
EOF
cat > ~/mkbranch.sh <<'EOF'
#!/bin/bash
# mkbranch.sh: be on the result branch: made from the code commit, or continued from its pushed tip after a fresh start
. ~/job.env; cd ~/mut || exit 1
[ "$(git rev-parse --abbrev-ref HEAD)" = "$BR" ] && exit 0
if git ls-remote --exit-code origin "refs/heads/$BR" >/dev/null 2>&1; then
  git fetch -q origin "$BR" && git checkout -q -b "$BR" FETCH_HEAD
else
  git checkout -q -b "$BR" "$SHA"
fi
git rev-parse --abbrev-ref HEAD
EOF
cat > ~/scan.sh <<'EOF'
#!/bin/bash
# scan.sh: refuse a push when anything outside $R is staged, or a staged file holds a presign marker, a key or token
# shape, or an e-mail shape
. ~/job.env; cd ~/mut || exit 1
out=$(git diff --cached --name-only | grep -v "^$R/")
[ -z "$out" ] || { echo "STAGED OUTSIDE $R:"; echo "$out" | head; exit 1; }
if git grep --cached -I -n -i -E -e 'x-am[z]-|awsaccesskeyi[d]|aws_secr[e]t|private[ ]key-----|gh[pousr]_[A-Za-z0-9]{30,}|github_pa[t]_|sk-an[t]-|A[KS]IA[A-Z0-9]{16}' -e '[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}' -- "$R"; then
  echo "SCAN HIT: nothing pushed"; exit 2
fi
echo "scan clean"
EOF
cat > ~/progress.sh <<'EOF'
#!/bin/bash
# progress.sh: copy the outcome lists so far, write PROGRESS.txt and STATUS.txt, scan, commit, push
. ~/job.env; cd ~/mut || exit 1
bash ~/mkbranch.sh >/dev/null || exit 1
mkdir -p "$R"
cpl() { mkdir -p "$2"; for f in caught missed timeout unviable; do if [ -f "$1/mutants.out/$f.txt" ]; then cp "$1/mutants.out/$f.txt" "$2/"; else : > "$2/$f.txt"; fi; done; }
cpl ~/mo "$R"
[ -d ~/mo-core/mutants.out ] && cpl ~/mo-core "$R/core-followup"
[ -d ~/mo-recheck/mutants.out ] && cpl ~/mo-recheck "$R/recheck-jobs1"
c() { if [ -f "$1" ]; then grep -c . "$1"; else echo 0; fi; }
t=$(( $(c "$R/caught.txt") + $(c "$R/missed.txt") + $(c "$R/timeout.txt") + $(c "$R/unviable.txt") ))
echo "PROGRESS $(date -u +%FT%TZ) shard=$K/30 commit=$SHA phase=$(cat ~/phase.txt 2>/dev/null) tested=$t of $(c ~/list-shard.txt) caught=$(c "$R/caught.txt") missed=$(c "$R/missed.txt") timeout=$(c "$R/timeout.txt") unviable=$(c "$R/unviable.txt")" > "$R/PROGRESS.txt"
echo "RUNNING $(date -u +%FT%TZ)" > "$R/STATUS.txt"
git add -- "$R"
bash ~/scan.sh || exit 3
git commit -q -m "v3.4 mutants $K/30: progress, $t tested" || true
git push -q origin "$BR" && echo "pushed $(git rev-parse --short HEAD): $t tested"
EOF
cat > ~/baseline.sh <<'EOF'
#!/bin/bash
# baseline.sh [TAG]: sections B1, B2a, B2b, B3a, B3b into ~/baseline.log (each with its command, rc and wall);
# with a TAG only B3a and B3b run again
. ~/job.env; cd ~/mut || exit 1
SK=(); while read -r t; do [ -n "$t" ] && SK+=(--skip "$t"); done < ~/skips.txt
TT=(--test bound_probe --test cost_invariants --test lint --test zerodha_index)
sec() { local n=$1; shift; echo "################ SECTION $n ################"; echo "\$ $*"; local t0; t0=$(date +%s%N); "$@"; local rc=$?; echo "=== SECTION $n rc=$rc wall_ms=$(( ($(date +%s%N) - t0) / 1000000 )) ==="; }
tag=${1:-}
{
  if [ -z "$tag" ]; then
    sec B1 cargo build --locked --workspace --tests --keep-going
    sec B2a cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade
    sec B2b cargo test --locked --no-fail-fast -p core "${TT[@]}"
  fi
  sec "B3a$tag" cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade -- --exact "${SK[@]}"
  sec "B3b$tag" cargo test --locked --no-fail-fast -p core "${TT[@]}" -- --exact "${SK[@]}"
} >> ~/baseline.log 2>&1
EOF
cat > ~/run-main.sh <<'EOF'
#!/bin/bash
# run-main.sh: the slice, the last wave's flags plus the case arguments of step 3
. ~/job.env; cd ~/mut || exit 1
SK=(); while read -r t; do [ -n "$t" ] && SK+=(--skip "$t"); done < ~/skips.txt
CA=(); read -r -a CA < ~/case-args.txt
date -u +%FT%TZ > ~/main.start
cargo mutants --workspace --shard "$K/30" --jobs 2 --cargo-arg=--locked "${CA[@]}" -o ~/mo -- -- --exact "${SK[@]}"
rc=$?
date -u +%FT%TZ > ~/main.end
exit $rc
EOF
cat > ~/run-core.sh <<'EOF'
#!/bin/bash
# run-core.sh (case C only): the slice's core mutants again, against core's 4 compilable test targets (output ~/mo-core)
. ~/job.env; cd ~/mut || exit 1
SK=(); while read -r t; do [ -n "$t" ] && SK+=(--skip "$t"); done < ~/skips.txt
grep '^crates/core/' ~/list-shard.txt > ~/core-part.txt
cargo mutants --workspace -f 'crates/core/**' --list > ~/core-list.txt
cmp -s ~/core-part.txt ~/core-list.txt || { echo "the core selection is not the slice's core part: STOP"; exit 9; }
cargo mutants --workspace -f 'crates/core/**' --jobs 2 --cargo-arg=--locked --cargo-arg=--test=bound_probe --cargo-arg=--test=cost_invariants --cargo-arg=--test=lint --cargo-arg=--test=zerodha_index -o ~/mo-core -- -- --exact "${SK[@]}"
EOF
cat > ~/find-timing.sh <<'EOF'
#!/bin/bash
# find-timing.sh: the CAUGHT mutants whose test log names ONLY wall-clock gate tests as FAILED -> ~/recheck-names.txt
. ~/job.env
O=~/mo/mutants.out
: > ~/recheck-names.txt
node -e 'const o = require(process.argv[1]); for (const x of o.outcomes) if (x.summary === "CaughtMutant") console.log(x.scenario.Mutant.name + "\t" + x.log_path);' "$O/outcomes.json" |
while IFS=$'\t' read -r name log; do
  failed=$(grep -E '^test .+ \.\.\. FAILED$' "$O/$log" | sed -E 's/^test (.+) \.\.\. FAILED$/\1/' | sort -u)
  [ -n "$failed" ] || continue
  [ -z "$(printf '%s\n' "$failed" | grep -v -x -F -f ~/timing-tests.txt)" ] && printf '%s\n' "$name" >> ~/recheck-names.txt
done
echo "timing-only catches: $(grep -c . ~/recheck-names.txt)"
EOF
cat > ~/run-recheck.sh <<'EOF'
#!/bin/bash
# run-recheck.sh: the timing-only catches again with --jobs 1 (output ~/mo-recheck); the last wave's shard 26 re-check
. ~/job.env; cd ~/mut || exit 1
[ -s ~/recheck-names.txt ] || { echo "no timing-only catches"; exit 0; }
SK=(); while read -r t; do [ -n "$t" ] && SK+=(--skip "$t"); done < ~/skips.txt
CA=(); read -r -a CA < ~/case-args.txt
SEL=()
for f in $(cut -d: -f1 ~/recheck-names.txt | sort -u); do SEL+=(-f "$f"); done
while IFS= read -r n; do SEL+=(-F "^$(printf '%s' "$n" | sed -e 's/[][\\.+*?(){}|^$]/\\&/g')\$"); done < ~/recheck-names.txt
cargo mutants --workspace "${SEL[@]}" --list > ~/recheck-list.txt
miss=$(grep -v -x -F -f ~/recheck-list.txt ~/recheck-names.txt | grep -c .)
[ "$miss" = 0 ] || { echo "the re-check list lacks $miss of the names: STOP"; exit 9; }
cargo mutants --workspace "${SEL[@]}" --jobs 1 --cargo-arg=--locked "${CA[@]}" -o ~/mo-recheck -- -- --exact "${SK[@]}"
EOF
cat > ~/final.sh <<'EOF'
#!/bin/bash
# final.sh [STATUS]: copy every result file, write SUMMARY.txt and STATUS.txt, scan, commit, push, verify the push
. ~/job.env; cd ~/mut || exit 1
st=${1:-DONE}
bash ~/mkbranch.sh >/dev/null || exit 1
mkdir -p "$R"
cpo() { mkdir -p "$2"; for f in outcomes.json caught.txt missed.txt timeout.txt unviable.txt; do [ -f "$1/mutants.out/$f" ] && cp "$1/mutants.out/$f" "$2/"; done; }
cpo ~/mo "$R"
[ -d ~/mo-core/mutants.out ] && cpo ~/mo-core "$R/core-followup"
[ -d ~/mo-recheck/mutants.out ] && cpo ~/mo-recheck "$R/recheck-jobs1"
[ -s ~/recheck-names.txt ] && mkdir -p "$R/recheck-jobs1" && cp ~/recheck-names.txt "$R/recheck-jobs1/"
[ -f ~/recheck-list.txt ] && cp ~/recheck-list.txt "$R/recheck-jobs1/"
for f in baseline.log skips.txt list-shard.txt; do [ -f ~/"$f" ] && cp ~/"$f" "$R/"; done
c() { if [ -f "$1" ]; then grep -c . "$1"; else echo 0; fi; }
v() { cat ~/"$1" 2>/dev/null | head -1; }
ts() { date -d "$1" +%s 2>/dev/null || echo 0; }
rm_=0; rc_=0; rx_=0
if [ -s ~/recheck-names.txt ] && [ -d ~/mo-recheck/mutants.out ]; then
  rc_=$(grep -x -F -f ~/recheck-names.txt ~/mo-recheck/mutants.out/caught.txt 2>/dev/null | grep -c .)
  rm_=$(grep -x -F -f ~/recheck-names.txt ~/mo-recheck/mutants.out/missed.txt 2>/dev/null | grep -c .)
  rx_=$(grep -v -x -F -f ~/recheck-names.txt ~/recheck-list.txt 2>/dev/null | grep -c .)
fi
core=""
[ -d ~/mo-core/mutants.out ] && core=" core_caught=$(c ~/mo-core/mutants.out/caught.txt) core_missed=$(c ~/mo-core/mutants.out/missed.txt) core_timeout=$(c ~/mo-core/mutants.out/timeout.txt) core_unviable=$(c ~/mo-core/mutants.out/unviable.txt) core_exit=$(v core.exit)"
tt=$(grep -o -i -E 'timeout to [0-9.]+ ?s' ~/main.log 2>/dev/null | head -1 | grep -o -E '[0-9.]+')
same=$( [ "$(git rev-parse HEAD:crates 2>/dev/null)" = "$(git rev-parse "$SHA:crates" 2>/dev/null)" ] && echo yes || echo NO )
echo "SUMMARY v34-mutants shard=$K/30 status=${st%% *} commit=$SHA code_same=$same cargo_mutants=$(cargo mutants --version 2>/dev/null | awk '{print $2}') rustc=\"$(rustc --version 2>/dev/null)\" case=$(v case.txt) list_total=$(v list-all.count) list_sha256=$(v list-all.sha) list_check=$(v list-check.txt) slice=$(c ~/list-shard.txt) slice_sha256=$(sha256sum < ~/list-shard.txt 2>/dev/null | cut -c1-64) caught=$(c ~/mo/mutants.out/caught.txt) missed=$(c ~/mo/mutants.out/missed.txt) timeout=$(c ~/mo/mutants.out/timeout.txt) unviable=$(c ~/mo/mutants.out/unviable.txt) exit=$(v main.exit) test_timeout_s=${tt:-none} run_start=$(v main.start) run_end=$(v main.end) run_wall_s=$(( $(ts "$(v main.end)") - $(ts "$(v main.start)") )) box_wall_s=$(( $(date +%s) - $(ts "$(v box.start)") )) skips=$(c ~/skips.txt) skips_sha256=$(sha256sum < ~/skips.txt 2>/dev/null | cut -c1-64) timing_only=$(c ~/recheck-names.txt) recheck_caught=$rc_ recheck_missed=$rm_ recheck_extra=$rx_$core" > "$R/SUMMARY.txt"
echo "$st $(date -u +%FT%TZ)" > "$R/STATUS.txt"
[ -f "$R/NOTES.md" ] || echo "# Mutation slice $K/30 of engine v3.4: notes (the box writes these; see step 9)" > "$R/NOTES.md"
git add -- "$R"
bash ~/scan.sh || exit 3
git commit -q -m "v3.4 mutants $K/30: ${st%% *}" || true
git push -q origin "$BR" || exit 4
[ "$(git ls-remote origin "refs/heads/$BR" | cut -f1)" = "$(git rev-parse HEAD)" ] && echo "PUSH VERIFIED $BR $(git rev-parse HEAD)"
cat "$R/SUMMARY.txt"
EOF
chmod +x ~/*.sh; ls ~/*.sh
```

## 2. Set-up: clone, toolchain, cargo-mutants (ESTIMATE 5 to 10 min)

```bash
. ~/job.env
uname -a; nproc; grep MemTotal /proc/meminfo; cat /sys/fs/cgroup/memory.max 2>/dev/null; df -P -BG ~ | tail -1
command -v cargo || { curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal && . ~/.cargo/env; }
[ -d ~/mut/.git ] || git clone --branch cloud/spikefade-v34int-20261116 --single-branch https://github.com/SJParthi/brutex.git ~/mut
cd ~/mut && git status -sb | head -1
[ "$(git rev-parse HEAD)" = "$SHA" ] && echo "HEAD OK" || echo "HEAD $(git rev-parse HEAD) is not the commit of the first message: STOP"
echo setup > ~/phase.txt
cd ~/mut && bash ~/bg.sh toolchain rustup toolchain install 1.97.1 --profile minimal -c rustfmt -c clippy -c llvm-tools-preview -t wasm32-unknown-unknown
```

Wait with `bash ~/wait.sh toolchain` until it prints `toolchain ENDED exit=0`. Then:

```bash
. ~/job.env; cd ~/mut && rustc --version && cargo --version
cd ~/mut && bash ~/bg.sh install cargo install cargo-mutants --locked --version 27.1.0
```

- `rustc --version` must print `rustc 1.97.1 (8bab26f4f 2026-07-14)`. The repository's `rust-toolchain.toml` pins it.
- Wait with `bash ~/wait.sh install` until `install ENDED exit=0`. Then `cargo mutants --version` must print
  `cargo-mutants 27.1.0`, the last wave's version.
- Anything else: STOP.

## 3. Your slice and its case (EXACT)

```bash
. ~/job.env; cd ~/mut
cat > ~/expected.txt <<'EOF'
list 24429 3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a
0 815 c70307bc7524cee9bb0e40d00de6d98a94cfaad7649648f880176d87d13157d1
1 815 b5e4cdcf5432e2da59dc203823668ca335327122dc42225bad7f8e4ed117875d
2 815 c99a91ff77cfe0cf14c7fc43e6d4ca499c66f349e34900aa0eb394e25793d2d5
3 815 3622e038eeaf9e4836923e59dbb03aa254b577d18dcc410af8464f3c92934fa5
4 815 0c8636dbdb6aa61ea9f3a25d4c9091db8166157f5447f2f378a4f465f1c2d243
5 815 04d001d543295ff87a942495b324a656649fc3b13a1e99ed30b9907de60844a1
6 815 dd17f481eb9a423597d2f394e34cf1618785bbb36a76fe729faf5287c9f56b9e
7 815 81be7935e8485db51bf3955d26b4981b3c3c654fa1f708ddc81180603b4f5972
8 815 fce194eb42c39123c01a9ac50aaae55bb83856879d3f0616bfb42d3754104906
9 815 cff3dbdf73750d195da07a5c75655ee8608d34909105118c24c14a48aa7e2a43
10 815 d42b5c5e97e00a28c3a05aefd911223c1ba400bfbd5d297fbb69aab679810adc
11 815 5c7181169d3226eb8f38285039f6adc54e0e42761c6750970fa49f9ee9932223
12 815 700692f89ccaab286bd48f7f9f41917fde27a951f4d483a5f875dad2c6134b6f
13 815 b182db148ea4d9a8eca5827b155d76f62a676e9a892d3cc06f1b3ace682a12f0
14 815 57fc76c93ea6342dee8d0ee7f938cdeaf636b33847526119f938ff089060789d
15 815 115221b0882bbaf47e249eb5badb7f4c13caa20e61dee506f5b03f2ea202b336
16 815 a48ee0ccbb4d1b21be39e73b2dac950adeee3e9843c59ade6e356f7e11816b43
17 815 c63f3ed5f2191b3b772bc3e6c4c6125a274c55ed4703944753bd100e021782cd
18 815 45b904d605df00434012fcb3cbc152ddb53223742ea571d68acea4ecb31ae59f
19 815 68a587480a6ce53674624f22b4a3f25653a211b69dbdb57778c5e9d36780ca09
20 815 558091359d50285b014caf307a7192a1f6c5de7ef789dcce715895a8791eb332
21 815 ff98a493832f3da4df14058d0a04d12ae903076180357c3e0cc4bc11d2d85062
22 815 c7dfadf0cd8d487c425bd58375833b8c2cd669d15920d5c87f10307daaf276f9
23 815 01e6f1b235292fa13336076d20aff78979a14c0b77779ce0f6aebc3c05cdeeb7
24 815 a946e0e4e255077dab4850796f32789151d4c478d2e68dea4a5876b3147a3858
25 815 923b654787945cee6b438ef138de34be50de42dda13d57a8032f2aa70b935a66
26 815 957e5d42576fd76c62a67f859d8a8050caefac6323994c14adb971abd073f19d
27 815 6e7f4358262841c27af4fc71185ab1ca26739ac775d67ff52cd68533774cd590
28 815 34fbb4e47bab57a2e2ee2e50a8e198da37147df71a409cd5ec584cc2cafaec17
29 794 bc5b8cfcc0383a5756d0c42828ea248e12bed9d8be6f946de436b21b6f89fb28
EOF
cargo mutants --workspace --list > ~/list-all.txt
cargo mutants --workspace --list --shard "$K/30" > ~/list-shard.txt
grep -c . ~/list-all.txt > ~/list-all.count; sha256sum < ~/list-all.txt | cut -c1-64 > ~/list-all.sha
got="$(cat ~/list-all.count) $(cat ~/list-all.sha) / $(grep -c . ~/list-shard.txt) $(sha256sum < ~/list-shard.txt | cut -c1-64)"
want="$(awk '$1=="list"{print $2" "$3}' ~/expected.txt) / $(awk -v k="$K" '$1==k{print $2" "$3}' ~/expected.txt)"
if [ "$got" = "$want" ]; then echo MATCH > ~/list-check.txt; else echo DIFFERS > ~/list-check.txt; fi
echo "list_check $(cat ~/list-check.txt): got $got; table $want"
cut -d/ -f2 ~/list-shard.txt | uniq -c
if ! grep -q '^crates/core/' ~/list-shard.txt; then
  echo A > ~/case.txt; : > ~/case-args.txt
elif ! grep -q -v '^crates/core/' ~/list-shard.txt; then
  echo B > ~/case.txt
  echo '--cargo-arg=--test=bound_probe --cargo-arg=--test=cost_invariants --cargo-arg=--test=lint --cargo-arg=--test=zerodha_index' > ~/case-args.txt
else
  echo C > ~/case.txt
  { for p in $(grep -v '^crates/core/' ~/list-shard.txt | cut -d/ -f2 | sort -u); do printf -- '--test-package %s ' "$p"; done; echo '--baseline=skip --timeout 60'; } > ~/case-args.txt
fi
echo "case $(cat ~/case.txt): $(cat ~/case-args.txt)"
```

- **The table** was made on the operator's Mac with cargo-mutants 26.2.0 `--list` on this exact tree (no build). On the
  last wave's branch, 26.2.0 listed exactly 27.1.0's 15,639 mutants, with all 30 slices equal (EXACT).
- **`list_check=DIFFERS` is not a stop.** Say how it differs in NOTES.md and go on. All 30 boxes use the same commit and
  the same cargo-mutants, so their 30 slices still split one list. Your case always comes from your own list.
- **Why three cases.** These are the last wave's own methods.
  - `core`'s lib unit tests and 5 of its integration test targets `include_str!` files of the app repository. This
    branch does not hold them, so those targets cannot compile here (B1 in step 5 shows the 6 targets).
  - cargo-mutants tests each mutant with its own package's tests. So `core`'s mutants need their own handling.

| case | when | arguments added to the command | the last wave's box |
|---|---|---|---|
| A | no `core` mutant in the slice | none | shards 2 to 29 |
| B | only `core` mutants | `--cargo-arg=--test=` for core's 4 compilable targets: `bound_probe`, `cost_invariants`, `lint`, `zerodha_index` | shard 0 |
| C | `core` and another package | `--test-package <each other package> --baseline=skip --timeout 60`, then step 7 | shard 1 (with its `vendor.rs` follow-up) |

- **Case C:** every mutant of the slice is tested by the other package's tests. cargo-mutants' own baseline is skipped,
  because it would build `core`'s tests, so the timeout is a fixed 60 s. Step 7 then re-tests the slice's `core` mutants
  with case B's targets, and keeps that output apart.
- **Expected (the table):** slice 0 is case C (core 642 + costs 173). Slices 1 (costs) and 2 (costs 324, greeks 345,
  spikefade 146) are case A, and so are slices 3 to 29 (spikefade).

## 4. Skips and wall-clock tests

```bash
cat > ~/skips.txt <<'EOF'
menu::tests::sf3_menu_and_sets_load_with_the_signed_counts
menu::tests::sf3_menu_fail_closed
menu::tests::sf3_tp_floor_not_armed_at_or_above_entry
menu::tests::sf3_distance_floor_counted
menu::tests::sf3_tp_theta_level_and_absence
tests33s::v33_search1d_sets_load_and_count
tests3::sf3_ttp_replaces_tp_close
tests34s::root_digest_same_for_every_form
tests34s::sfz_roundtrip_all_rows
every_declared_cost_invariant_reaches_a_bench
the_document_declares_the_five_operations_rule_4_names
every_measured_cost_invariant_is_declared
every_crate_carries_a_ratio_bench
the_reconciliation_is_reading_something
tests34i::v34i_f3_real_rows_spot
tests34i::v34i_f7_rules_tables_load_dated_and_minrange
tests34i::v34i_f7_opr_file_is_next_session_and_columns
EOF
cat > ~/timing-tests.txt <<'EOF'
split18::tests::v32rb_split_o1_gates
tests32r::v32r_o1_range_max_and_first_reach
tests32r::v32r_o1_decision_at_n100_and_sampled_steps
tests32r::v32r_o1_margin_lookup_ns
EOF
grep -c . ~/skips.txt ~/timing-tests.txt
```

**The 17 skips.** The first 14 are the last wave's list in its order. `tests34::` is now `tests34s::`, because v3.4 renamed
the shard kit's test module. The last 3 are new in v3.4. Each one needs something this branch does not hold:

| tests | cause |
|---|---|
| the 5 `menu::tests::*`, `tests33s::v33_search1d_sets_load_and_count`, `tests3::sf3_ttp_replaces_tp_close` | the exit-rule parameter tables `crates/spikefade/data/EXITS-DYN-*.tsv` (left out of the branch) |
| `tests34s::root_digest_same_for_every_form`, `tests34s::sfz_roundtrip_all_rows` | KNOWN: with every kit switch off the engine calls zstd at the Mac path `/opt/homebrew/bin/zstd` (kit.rs:470). The fix is not in v3.4. Report it as that known cause. |
| the 5 `core` `cost_invariants` names | the app repository's `docs/04-invariants.md`, and `crates/spikefade/benches/ratio.rs` |
| the 3 `tests34i::*` | real tables at an absolute path of the operator's Mac (tests34i.rs line 14), plus the NSE contract files one of them names |

- Every box passes the same 17, as the last wave passed its 14 everywhere. A name that matches no test of a package does
  nothing there.
- **Limit:** a mutant that only a skipped test would catch is counted as missed.

**The 4 wall-clock tests** assert time bounds: p99 and p50 limits on `close`, `account`, range max, decision and margin
lookup. Under `--jobs 2` they can fail on load alone.
- The last wave's slice 26 found 11 of its 83 catches came only from `split18::tests::v32rb_split_o1_gates`.
- With `--jobs 1`, all 11 then passed, so they were missed.
- Step 8 now makes that re-check standard.

## 5. Baseline before cargo-mutants (ESTIMATE 4 to 8 min)

```bash
. ~/job.env; echo baseline > ~/phase.txt; : > ~/baseline.log; bash ~/bg.sh bl bash ~/baseline.sh
```

Wait with `bash ~/wait.sh bl` until `bl ENDED`. (The job is named `bl`, so its console log `~/bl.log` stays apart from
`~/baseline.log`.) Then check:

```bash
grep -E '^=== SECTION|^error: could not compile' ~/baseline.log
awk '/^#+ SECTION B2/{f=1} /^#+ SECTION B3/{f=0} f' ~/baseline.log | grep -E '^test .+ \.\.\. FAILED$' | sed -E 's/^test (.+) \.\.\. FAILED$/\1/' | sort -u > ~/b2-failed.txt
echo "B2 failed: $(grep -c . ~/b2-failed.txt)"; echo "not in skips.txt:"; grep -v -x -F -f ~/skips.txt ~/b2-failed.txt
tac ~/baseline.log | awk '{print} /^#+ SECTION B3a/{exit}' | tac | grep -c -E '^test .+ \.\.\. FAILED$|^test result: FAILED'
```

- **B1:** rc 101, and exactly 6 `could not compile` errors, all for `core`: its lib test, and the test targets
  `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph`. If any other target fails to compile,
  STOP.
- **B2** (no skips). Expected: every failure is in `skips.txt`. MEASURED on the Mac, the engine suite gives 216 passed,
  0 failed and 1 ignored there, so here the expected failures are the 17 listed (12 engine, 5 core).
- **A failure that is not in `skips.txt`** may be added to `skips.txt` only if `baseline.log` shows it fails because a
  file or a tool path is absent on this box, for example `No such file or directory`, a Mac path, or no `/bin/ps`.
  - Write it into NOTES.md as `ENV-EXTRA <test>: <the error line>`.
  - Then run `bash ~/bg.sh bl2 bash ~/baseline.sh -r2`, and wait with `bash ~/wait.sh bl2`. Only the sections `B3a-r2`
    and `B3b-r2` run again, appended to `~/baseline.log`.
  - Any other failure (a wrong value, a panic in engine code, a hang): STOP.
- **B3** (with the skips): 0 failed tests and `rc=0` in both B3 sections. Otherwise STOP.
- The last command line counts failures in the LAST B3 pair only. It must print 0.

## 6. The run (case A, B or C): start, wait, push progress

```bash
. ~/job.env; bash ~/mkbranch.sh; echo main > ~/phase.txt; bash ~/bg.sh main bash ~/run-main.sh
```

**Then loop until the run ends.**
- Run `bash ~/wait.sh main ~/mo`, which returns within 270 s.
- After every third wait, run `bash ~/progress.sh`. That is at most 15 minutes apart.
- The first `progress.sh` creates your branch on the remote. If `progress.sh` fails, STOP.
- **No progress:** if `tested=` in two progress lines 45 minutes apart is the same, run `kill $(cat ~/main.pid)` and STOP
  with reason `no progress`.

**When `wait.sh` prints `main ENDED exit=N`:**
- **0** (all caught), **2** (some missed) or **3** (some timed out): go on.
- **4** (cargo-mutants' own baseline failed; case A or B): look at the end of `~/main.log`.
  - If a test in `~/timing-tests.txt` failed, start the same command once more.
  - A second 4, or a 4 for any other cause: STOP.
- **Anything else:** STOP.

`cargo mutants` auto-sets the test timeout from its baseline in cases A and B, as in the last wave. Case C uses 60 s.
There is no build timeout, which is the default.

## 7. Case C only: the core follow-up

```bash
. ~/job.env; [ "$(cat ~/case.txt)" = C ] && { echo core > ~/phase.txt; bash ~/bg.sh core bash ~/run-core.sh; }
```

- Loop as in step 6: `bash ~/wait.sh core ~/mo-core`, and `bash ~/progress.sh` after every third wait.
- An exit of 0, 2 or 3 is fine. An exit of 9 means the `-f 'crates/core/**'` list is not the slice's `core` part.
  - Then try `-p core` in its place: edit only `~/run-core.sh`, and run it again.
  - A second 9: STOP.
- This is the last wave's shard 1 follow-up, but applied to every `core` mutant of the slice. With the expected list,
  that is all 642 of them.

## 8. The wall-clock re-check (every case)

```bash
. ~/job.env; echo recheck > ~/phase.txt; bash ~/find-timing.sh && bash ~/bg.sh recheck bash ~/run-recheck.sh
```

- Loop with `bash ~/wait.sh recheck ~/mo-recheck` until it ends.
- **Exit 0, 2 or 3:** fine. **Exit 9:** the names did not select. STOP.
- `recheck-list.txt` may hold extra `delete field ... from struct ... expression` mutants of the same files. cargo-mutants
  27.1.0 does not apply `-F` to them, as the last wave found. They run, and SUMMARY counts them as `recheck_extra`; they
  belong to the re-check only.
- **The official files stay as cargo-mutants wrote them.** A timing-only catch that the re-check finds MISSED is reported
  as `recheck_missed`.
  - corrected caught = caught - recheck_missed
  - corrected missed = missed + recheck_missed

## 9. Final commit and push

1. Write `~/mut/results/mutants/shard-K-of-30/NOTES.md` in plain words. Start with the title line, then cover:
   - the box: `uname`, `nproc`, memory and its cgroup limit;
   - `rustc` and `cargo-mutants` versions, the case and the exact command;
   - `list_check`, and how it differed if it did;
   - the baseline in short (each section's rc and wall);
   - any `ENV-EXTRA` test;
   - the exit codes, and anything odd.
2. Then run:

```bash
. ~/job.env; echo final > ~/phase.txt; bash ~/final.sh
```

- It prints `PUSH VERIFIED` and the SUMMARY line.
- **To stop at any step:** `bash ~/final.sh "STOPPED <reason>"`. It pushes whatever exists, with `STATUS.txt` saying so.
- If the scan prints `SCAN HIT`, push nothing. Report the file and line, and never print the matched value.

## 10. Your final message

- the SUMMARY line;
- the branch, and its last commit;
- the case;
- every `ENV-EXTRA` test or STOP reason;
- the number of MISSED mutants, and the first 30 of them as `file:line: change`. The rest are in `missed.txt`.

## 11. What the branch holds (`results/mutants/shard-K-of-30/`)

| file | content |
|---|---|
| `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt` | cargo-mutants' own output for the slice, unchanged (the last wave's layout) |
| `baseline.log` | sections B1 to B3, each with its command, rc and wall in ms |
| `skips.txt`, `list-shard.txt` | the exact skips, and the slice as cargo-mutants listed it |
| `core-followup/` | case C only: step 7's `outcomes.json` and the 4 lists |
| `recheck-jobs1/` | step 8: `recheck-names.txt`, `recheck-list.txt`, `outcomes.json` and the 4 lists (if any timing-only catch) |
| `SUMMARY.txt` | ONE line (below) |
| `STATUS.txt` | `DONE <time>` or `STOPPED <reason> <time>` (`RUNNING` in progress commits) |
| `PROGRESS.txt` | the last progress line: tested so far and the counts (left as the last progress commit wrote it) |
| `NOTES.md` | the box's notes |

The SUMMARY line holds these fields:

```
SUMMARY v34-mutants shard=K/30 status= commit= code_same= cargo_mutants= rustc="" case= list_total= list_sha256= list_check=
slice= slice_sha256= caught= missed= timeout= unviable= exit= test_timeout_s= run_start= run_end= run_wall_s= box_wall_s=
skips= skips_sha256= timing_only= recheck_caught= recheck_missed= recheck_extra= [core_caught= core_missed= core_timeout=
core_unviable= core_exit=]
```

## 12. The expected list (EXACT, the operator's Mac, no build)

- **24,429 mutants:** core 642, costs 1,312, greeks 345, spikefade 22,130. Slices of 815 (the default `slice` sharding);
  slice 29 holds 794.
- **The engine grew by 8,790 mutants since the last wave** (13,340). Most of the growth is in `fix34.rs` (7,976).
  - 6,818 of them replace the return value of ONE function, `wick_compare`, which returns a 3-part tuple. They are list
    lines 16,067 to 22,884, so slices 19 to 28 are almost all `wick_compare`.
  - 433 more, in slices 18 and 19, replace `BandTable::parse`.
  - The run keeps them all: this is a FULL run, with the last wave's flags.

## 13. Expected wall (ESTIMATE)

**Inputs (MEASURED, the last wave, cargo-mutants 27.1.0, `--jobs 2`, 4-vCPU boxes):**
- spikefade slices of 522 took 1,327 to 3,139 s, median 2,341 s. That is 2.9 to 6.0 s a mutant, median 4.5 s.
- slices 0 to 3 (core, costs, greeks) took 259 to 708 s.

**Model:** 815 mutants a slice, times the last wave's seconds a mutant, times 1.25. The 1.25 is because v3.4's engine
source is 18% larger and it has 217 tests against 169.

| box | run | with set-up (~15 min: toolchain, cargo-mutants install, baseline) and the re-check |
|---|---|---|
| slices 3 to 29 | 50 to 100 min, median about 75 min | about 65 min to 2 h, median about 1.5 h |
| slice 0 (case C, plus the core follow-up), slice 1, slice 2 | 15 to 30 min | 30 to 45 min |

The whole wave, with 30 boxes in parallel, takes about 2 h.

## 14. What changed from the last wave (which ran on the v3.3 kit, commit 15fd4f47)

1. **Source:** the v3.4 tree (F3, F7, the shard kit and round D's fixes) on this branch.
2. **cargo-mutants is pinned** with `--version 27.1.0`. The last wave ran an unpinned install, which resolved to 27.1.0.
3. **Skips:** the same 14 under v3.4's module names, plus the 3 new real-table tests (step 4).
4. **The case comes from a rule on the box's own list** (step 3), not from each box's judgement.
   - The flags are the same three methods the last wave used.
   - In v3.4 the case C slice is 0, not 1, and its core follow-up covers the whole `core` package.
5. **Output goes outside the checkout** (`-o ~/mo`). The last wave wrote to the default `mutants.out` in the checkout,
   except in its re-checks.
6. **Progress commits** at least every 15 minutes, with `STATUS.txt` and `PROGRESS.txt`. The last wave pushed once, at
   the end.
7. **The `--jobs 1` re-check of timing-only catches is standard** (step 8). The last wave's slice 26 did it once, by hand.
8. **One SUMMARY line** in `SUMMARY.txt`. The other files keep the last wave's layout and folder names.
9. **Baseline sections** B1 to B3 replace the last wave's sections 1 to 5. The commands are the same, and the literal
   `cargo test --workspace`, which cannot compile here, is dropped.
