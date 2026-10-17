STATUS: IN PROGRESS

# Cloud mutation and coverage pass B, continued — `crates/api`, 2026-10-17

Branch `cloud/mut-api-b-20261017`, cut from `origin/feat/db-fix` at
`f9cdb584` (2026-10-17 12:16 IST). It carries the 16 Oct pass
(`origin/cloud/mut-api-b-20261016`, cut from `cloud/base-20261016` =
`c9e3472e5`, the snapshot of `feat/db-fix` at `4756cce0`) re-applied with
`git apply -3`, then finishes what that pass left open.

**This report holds no authority** (`CLAUDE.md` §10): it is evidence of what
was measured on this date, on this machine.

## Re-apply

`git diff c9e3472e5 origin/cloud/mut-api-b-20261016 -- . ':(exclude)reports' | git apply -3`
applied to all eleven files with **no conflict**. `feat/db-fix` had changed
four of them since the snapshot (`emitted.rs`, `import_journal.rs`,
`pullrun.rs`, `server.rs`); none of those hunks overlapped the pass's.

## Done

- (in progress)

## Left

- (in progress)

## Gates

- (in progress)

## NEEDS THE MAC

- (in progress)
