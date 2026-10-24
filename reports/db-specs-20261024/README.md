# Ingest + /db parts: cloud build rules (24 Oct 2026)

This folder is planning material for cloud sessions. It is never merged into `feat/db-fix`.
Each part named here is built by its own cloud session on its own branch, then merged back into
`feat/db-fix` on the operator's Mac, one at a time, after a second agent checks it there.
One owner per part: while a cloud session owns a part, the Mac never builds it.

Files: this `README.md` (the rules), `CONTEXT.md` (background every part shares) and one
`<NAME>.md` per part (its full specification, prerequisites, owned files, findings and checks).
They are pushed by the Mac and may arrive after you start: see "Start and hold" below.

## Start and hold (the switch the Mac controls)

Cloud sessions share one usage budget with the Mac, so the Mac starts and holds them with two
files on `origin/cloud/db-base-20261024`, in this folder:

- `GO` - you may start building (or continue after a hold).
- `HOLD` - finish the step you are in, then commit, push, update your report and wait.

1. Before any build work, wait until BOTH `reports/db-specs-20261024/<NAME>.md` and
   `reports/db-specs-20261024/GO` exist and `HOLD` does not:
   `git fetch -q origin cloud/db-base-20261024 && git cat-file -e FETCH_HEAD:reports/db-specs-20261024/GO && ! git cat-file -e FETCH_HEAD:reports/db-specs-20261024/HOLD 2>/dev/null`
   Poll with one blocking shell call of up to 9 minutes (`until <the test>; do sleep 60; done`),
   repeated, with no other work in between. Waiting is expected, not a fault.
2. While you work, test the same files at every save point (about every 20 minutes). If `HOLD`
   exists: finish the step you are in, commit, push, write your report (`STATUS: HELD`), then wait
   as in step 1. Never start a new step while `HOLD` exists. Never abandon work because of a hold.

## Before you build

1. Your part's file names its **prerequisites**. Every one must be merged into
   `origin/feat/db-fix`: `git log origin/feat/db-fix --first-parent --merges --format=%s | grep "^Merge <name>:"`.
   If one is missing, write `STATUS: BLOCKED` with the missing names, push, and wait for GO again
   (the Mac re-arms you when they merge). Never build on an unmerged part.
2. Branch `cloud/db-<NAME>-20261024` from the newest `origin/feat/db-fix`. Merge the newest
   `origin/feat/db-fix` into it again before your final gates.
3. Read `CLAUDE.md` (the law), `CONTEXT.md`, then your part's file in full. Edit only the files your
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
- Gates, all green before `STATUS: DONE`: `cargo fmt --all --check`;
  `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked`;
  `cargo deny check` (green on `feat/db-fix` since 17 Oct: it must stay green after you merge the newest
  `origin/feat/db-fix`); the CI shell gates under `.github/` that touch your files; 100% line and branch
  coverage and no surviving mutant on the modules you touched (`cargo llvm-cov --branch`,
  `cargo mutants --in-diff`). If a tool cannot be installed here, say so plainly and list the run
  under NEEDS THE MAC.
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
  list, files outside ownership, the D-numbers used, and the NEEDS THE MAC list.
- When you are done, push, then end your turn. The Mac merges your branch back under the subject
  `Merge <NAME>: ...`.

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
