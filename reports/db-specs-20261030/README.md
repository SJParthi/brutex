# Ingest + /db parts: cloud build rules (30 Oct 2026)

This folder is planning material for cloud sessions. It is never merged into `feat/db-fix`.
Each part named here is built by its own cloud session on its own branch, then merged back into
`feat/db-fix` on the operator's Mac, one at a time, after a second agent checks it there.
One owner per part: while a cloud session owns a part, the Mac never builds it.

Files: this `README.md` (the rules), `CONTEXT.md` (background every part shares) and one
`<NAME>.md` per part (its full specification, prerequisites, owned files, findings and checks).
They are pushed by the Mac and may arrive after you start: see "Start and hold" below.

## Start and hold (the switches the Mac controls)

Cloud sessions share one usage budget with the Mac, so the Mac starts and holds them with three
kinds of file on `origin/cloud/db-base-20261030`, in this folder:

- `GO` - the wave is open.
- `GO-<NAME>` - YOUR part may build. The Mac can stop one part by removing its `GO-<NAME>` without
  stopping the others.
- `HOLD` - every part holds.

You build only while `GO` AND `GO-<NAME>` exist and `HOLD` does not. (This replaces the 24 Oct rule,
under which `GO-<NAME>` overrode `HOLD`: now `HOLD` holds every part, and a part with no `GO-<NAME>`
never builds. Why: under the 24 Oct rule a part that had its `GO-<NAME>` ignored `HOLD`, so no single
file could stop the whole wave when the shared usage budget ran short; now `HOLD` stops every part and
removing one `GO-<NAME>` stops one.)

1. Before any build work, wait until `reports/db-specs-20261030/<NAME>.md`, `GO` and `GO-<NAME>` all
   exist and `HOLD` does not:
   `git fetch -q origin cloud/db-base-20261030 && git cat-file -e FETCH_HEAD:reports/db-specs-20261030/<NAME>.md && git cat-file -e FETCH_HEAD:reports/db-specs-20261030/GO && git cat-file -e FETCH_HEAD:reports/db-specs-20261030/GO-<NAME> && ! git cat-file -e FETCH_HEAD:reports/db-specs-20261030/HOLD 2>/dev/null`
   Poll with one blocking shell call of up to 9 minutes (`until <the test>; do sleep 60; done`),
   repeated, with no other work in between. Waiting is expected, not a fault.
2. While you work, run the same test at every save point (about every 20 minutes). If it fails
   (`HOLD` exists, or `GO` or `GO-<NAME>` is gone): finish the step you are in, commit, push, write
   your report (`STATUS: HELD`), then wait as in step 1. Never start a new step while the test fails.
   Never abandon work because of a hold.

## Before you build

1. Your part's file names its **prerequisites**. Every one must be merged into
   `origin/feat/db-fix`: `git log origin/feat/db-fix --first-parent --merges --format=%s | grep "^Merge <name>:"`.
   If one is missing, write `STATUS: BLOCKED` with the missing names, push, and wait for GO again
   (the Mac re-arms you when they merge). Never build on an unmerged part.
   A link your file marks FILES_ONLY ("merge first, do not block the start") does NOT block your start:
   build now; only your merge-back waits for it. Your part's file names any such link.
2. **Base first (operator decision D1, 27 Oct 2026).** Nothing merges into `feat/db-fix` before
   `BASE-REDS-1023` has merged, with no exception. This holds your MERGE-BACK, never your building:
   build, gate and report now; the Mac merges your branch back only after
   `Merge BASE-REDS-1023:` is on `feat/db-fix`. `BASE-REDS-1023` is not a prerequisite in the sense of
   step 1, so its absence never makes you `STATUS: BLOCKED` before you build.
3. Branch `cloud/db-<NAME>-20261030` from the newest `origin/feat/db-fix`. Merge the newest
   `origin/feat/db-fix` into it again before your final gates.
4. Read `CLAUDE.md` (the law), `CONTEXT.md`, then your part's file in full. Edit only the files your
   part owns; a file outside that list is kept minimal and named in your report.

## What differs in the cloud

- **Paths.** The texts name folders on the operator's Mac (`/Volumes/WD_BLACK/...`, `wt/`, `tgt/`,
  `state/`, a `bin/cargo` queue, shadow servers, ports). None exist here. Work in your checkout and use
  plain `cargo`. Everything those folders held that your part needs is copied into your part's file.
- **Real data.** There is no market data store, no `:8080` app, no shadow server and no vendor file
  here. Build and test on invented fixtures (prices that cannot be mistaken for real ones; never a
  real vendor row). List every real-data check, store write, shadow-server check and `:8080` check
  under **NEEDS THE MAC** in your report with the exact command and the expected answer, so the Mac
  runs it at merge-back. Never invent a measurement; label anything unmeasured as unmeasured.
- **Private settings.** Never write a Dropbox link, token, credential path, parameter path, account
  id or e-mail address into any file. They live only in the operator's local untracked files.
- **Decision numbers.** Use only the `D-` numbers your part's file assigns you. If it assigns none,
  write `D-TBD-<NAME>-1`, `-2`, ... and list them in the report; the Mac assigns real numbers from its
  registry at merge-back (two branches once wrote the same number; this prevents it).
- **The host.** The 27 Oct cloud sessions ran as root on 4 cores, and tests that root or 4 cores
  defeat were red there (`CONTEXT.md` section 4.10). Show such a red the same on the unmodified
  `origin/feat/db-fix` and list it under NEEDS THE MAC; never skip it silently.

## The operator's standing rules (binding, verbatim intent)

- **O(1) at p99, everywhere.** Every operation you add or touch is O(1) per operation. Measure p50,
  p99 and max (n >= 200 sequential calls, or the crate's bench) and report n and the method. Anything
  that cannot be O(1) is refused or bounded by name, never silently slow, and named for
  `docs/06-limits.md`.
- **Everything in parallel** where it is safe, never serial by habit.
- **100% common, runtime-dynamic, incremental, scalable.** One shared code path for every
  instrument, feed and timeframe (never a special case for one name); a new day computes only itself;
  adding data or rows scales without code edits. The Winner Gate numbers are the only fixed numbers in
  the project: every other threshold, window, bucket or limit is generated from the data.
- **Saved, audited, logged, tracked, searchable, monitored, visible.** Every state change is saved
  and audited with a record you can search; every failure, refusal and extreme value is tracked so it
  can be found, debugged, monitored and seen where the operator looks (`/logs`, `/db` or the part's own
  page) through a read that stays O(1) at p99. A state change with no saved, audited record, or a
  failure no log or page can show, is a blocking defect.
- **Every extreme case, including out of the box.** At least 8 extreme cases per change with tests
  (empty, one-record and maximum inputs; month, year and expiry boundaries; special sessions; renamed
  symbols; duplicate, out-of-order and torn records; crash and resume; full disk; clock edges), of which
  at least 2 are COMBINATIONS of extremes at once and at least 1 is an out-of-the-box case no list
  names. Every test fails before your change and passes after it.
- Fallbacks never hide a failure: degrade loudly with the reason, or refuse.

## Repository law and gates

- `CLAUDE.md` is law: Rust only outside `web/`; no interpreted runtime; prices are paisa `i64`; no
  look-ahead; append-only history; no depth parameter; honest limits.
- Never use the Py-language and never write its name anywhere. Shell, `jq` and Rust only.
- Say "order flow delta" and "order flow cumulative delta", never the bare word (the store's `delta`
  is the option greek).
- Gates, all green before `STATUS: DONE`:
  - `cargo fmt --all --check`;
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  - `cargo test --workspace --locked`;
  - `cargo deny check` at CI gate 3's pin, cargo-deny 0.19.0 (`.github/workflows/ci.yml`). It was red
    on the base from 24 to 27 Oct on RUSTSEC-2026-0386 (tokio 1.53.1) and is green again since
    BASE-DENY-1024 merged on 27 Oct (tokio 1.53.3 in `Cargo.lock`). It must stay green after you merge
    the newest `origin/feat/db-fix`;
  - the CI shell gates under `.github/` that touch your files (several are red at the base:
    `CONTEXT.md` section 4.10);
  - **deep tests on your D2 set (operator decisions D2 and D3, 27 Oct).** Your D2 set is every Rust
    file you added (whole), every function you added or changed (whole, the outermost `fn` item from
    its first doc or attribute line to its closing brace, including the function a deletion-only hunk
    cuts into), and every added line outside a function; `.rs` files outside `crates/` (`.github/*.rs`,
    `web/**/*.rs`) count the same way. (This narrow reading holds for a crate until `Merge BASE-COV-<G>:`
    for it is in the tip you measure, G = CLI, API, PULL or LIBS; from then on that whole crate is the
    coverage set, never the mutant set: `CONTEXT.md` 4.14. None is on `feat/db-fix` at `f01d7a0e4`.)
    On that set: 100% line and branch coverage, and no surviving
    mutant. Mutants run on the D2 set, never on whole touched files: `cargo mutants --in-diff` alone
    sees only the changed lines, so list every mutant of your files
    (`cargo mutants --list --json --file <f> ...`), keep those whose span meets a set item, and run
    exactly those (`--file <f> ... --re <regex>`; CI pins cargo-mutants 26.2.0). Measure coverage with
    the tools this host has (CI pins cargo-llvm-cov 0.8.4; branch figures need a nightly toolchain), and
    report the set, the tool versions and each figure. A figure no tool here can produce is NOT PROVED,
    never "covered", and NOT PROVED never passes. A gap or a surviving mutant in the set is never
    excused because `feat/db-fix` has the same one.
  If a tool cannot be installed here, say so plainly and list the run under NEEDS THE MAC.
- **NEEDS THE MAC, always:** the D3 re-measure of your D2 set by recipe v4 at merge-back (Route B
  coverage on the Mac's pinned nightly and its own llvm-tools; mutants on the D2 set, sharded). Your
  figures are the builder's; the Mac's figures decide.
- **A red the base shares is never a pass (D1).** For every gate red on your branch, run the same
  command on the unmodified `origin/feat/db-fix` and give both outputs. A red equally present there is
  written "NOT PROVED here: red on feat/db-fix too (owner <part>)", with both command outputs, never as
  passed, "same as base", "baseline" or "ok". Comparing your branch's refusals with the base's only
  tells whose a red is: a refusal your branch adds is yours to fix. Your part merges back only when
  the Mac's D1 landing gate proves every required gate green on the merged tree.
- Commit messages: what changed and why, the finding ids and D-numbers, ending with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Push only your own branch. Never push `main`, `feat/db-fix` or this base branch, never open a pull
  request, never delete or force-push anything.

## Working and reporting

- Work continuously until the part is done; no CONTINUE message is needed. Commit and push your
  branch at every save point (about every 20 minutes), so a usage limit loses at most a few minutes.
- Keep `reports/cloud-db-<NAME>.md` on your branch. Its first line is one of `STATUS: DONE` (complete,
  every gate green), `STATUS: IN PROGRESS`, `STATUS: HELD` or `STATUS: BLOCKED`. Then: what is done,
  what is left, every gate's result with its command, the p50/p99/max measurements, the extreme-case
  list, the D2 set with its coverage and mutant figures, files outside ownership, the D-numbers used,
  and the NEEDS THE MAC list.
- When your own work is complete and every red left is the base's (with the base's command output) or
  this host's, the first line is `STATUS: BLOCKED`, followed by each such red by name, as CC-5, IA-10a,
  IA-17b and R3-20 wrote on 27 Oct. Never `STATUS: DONE` on a red, whoever owns it.
- When you are done, push, then end your turn. The Mac merges your branch back under the subject
  `Merge <NAME>: ...`, after `BASE-REDS-1023` has merged (D1).

## The operator's own words (24 Oct 2026, pasted again; binding on every step)

```text
Always achieve O(1) even with p99 everywhere.
Always work on everything parallely.
Always ensure saving into db and auditing logging tracking capturing debugging finding searching
monitoring dashboard visualising extremes analyzing covering, easily accessible everything, by
covering entirely each and every nook and corner, but always ensure to achieve O(1) always.
Make everything a 100 percent common runtime dynamic incremental scalable approach.
Always achieve O(1) everywhere.
Especially cover all kinds of extreme permutations, combinations of exceptions, errors, situations,
conditions, bugs, scenarios, ideas, etc., including out of the box.
```

How each line is checked in your part (the same as on the Mac): O(1) per operation with p50/p99/max
measured and reported; steps run as parallel subagents where they are independent; every state change
saved and audited with a searchable record, every failure, refusal and extreme value visible on
`/logs`, `/db` or the part's own page through an O(1) p99 read; one common runtime-dynamic,
incremental, scalable code path with no typed number outside the Winner Gate; at least 8 extreme cases
with at least 2 combinations and 1 out-of-the-box case, each test failing before and passing after.
The independent check (see your effort message) treats any miss as BLOCKING.
