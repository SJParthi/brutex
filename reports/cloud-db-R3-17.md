STATUS: IN PROGRESS

# R3-17 — one upkeep runner (cloud build report)

**BLOCKED: prerequisite R3-06 is not merged into `origin/feat/db-fix`. Per the cloud build
rules ("If one is missing, say so in your report and stop: never build on top of an unmerged
part") nothing has been built. No source file, test, doc, `Cargo.toml` or `Cargo.lock` was
edited; this report is the only file on the branch that differs from the base.**

Branch `cloud/db-R3-17-20261017`, cut from `origin/feat/db-fix` at
`50093fa3314164d2fb9fc4931da4689eaba1e2e5` (`Merge R3-35: Importer file split …`), fetched
17 Oct 2026.

## Prerequisites

Command, run against that head:

```
git log origin/feat/db-fix --merges --format=%s | grep -E "^Merge (R3-06|R3-16|R3-36):"
```

Output (two lines; R3-06 has none):

```
Merge R3-36: Store steps as one checked list: every pending run with its command, its prerequisite and its proof
Merge R3-16: Shared safe file helpers: one way to create a temporary file, one way to open a file for reading
```

| Prerequisite | Merged into `feat/db-fix`? | Evidence |
|---|---|---|
| R3-06 | **no** | no merge subject; no commit subject or body on `feat/db-fix` names `R3-06`; no remote branch's last 300 subjects name it; there is no `reports/db-specs-20261016/R3-06.md` (it is a Mac-started part). The base only forward-references it: `crates/store/src/sealed.rs` carries "R3-06's bytes" as an opaque month-summary section, and `docs/06-limits.md` row L-R401-04 says a file the census does not list stays unseen "until the census recount (R3-06) lists it". |
| R3-16 | yes | `Merge R3-16: Shared safe file helpers …` |
| R3-36 | yes | `Merge R3-36: Store steps as one checked list …` |

Why R3-06 has to land first, from the specs: R3-06 owns the month summary (v1 bytes, which
SC-2's sealed section carries and R4-27 extends to v2) and the census recount and
`pull/src/gaps.rs` (named in R4-19's spec). R3-17's own goal lists "month summary" and
"a recount" among the builders the upkeep runner drives, and R3-17 owns
`crates/cli/src/summary_build.rs`. Building that runner before the summary format exists
would mean inventing a format R3-06 owns.

## Done

- Prerequisites checked (above).
- Owned-file inventory on the base, so the relaunched session starts from facts:

| Owned file (spec spelling) | On the base at `50093fa3` |
|---|---|
| `crates/cli/src/srt_build.rs` | absent — the tree has `crates/cli/src/srt_builder.rs` |
| `crates/cli/src/srt_build_tests.rs` | absent — the tree has `crates/cli/src/srt_builder_tests.rs` |
| `crates/cli/src/tix_rebuild.rs` | absent — the tree has `crates/cli/src/tix_rebuilder.rs` |
| `crates/cli/src/tix_rebuild_tests.rs` | absent — the tree has `crates/cli/src/tix_rebuilder_tests.rs` |
| `crates/cli/src/census_recount.rs` | present, 472 lines |
| `crates/cli/src/census_recount_tests.rs` | present, 621 lines |
| `crates/cli/src/gdfl_rekey.rs` | present, 303 lines |
| `crates/cli/src/gdfl_rekey_tests.rs` | present, 261 lines |
| `crates/pull/src/upkeep.rs` | absent (new file) |
| `crates/pull/src/upkeep_tests.rs` | absent (new file) |
| `crates/cli/src/upkeep_command.rs` | absent (new file) |
| `crates/cli/src/summary_build.rs` | absent (new file) |

  The spec's `srt_build*` and `tix_rebuild*` names do not match the tree's `srt_builder*` and
  `tix_rebuilder*`. Whether R3-17 owns the existing `*_builder` / `*_rebuilder` files (most
  likely: same modules, spelling drift in the spec) or is meant to add new ones is for the Mac
  to settle when it relaunches this part.

## Left (all of it)

The whole work package: the upkeep runner (`pull::upkeep`, `cli upkeep`), the per-month
"what is missing" listing, N-worker builders as tracked runs with run id, progress, Stop,
resume and the import's locks, the unstamped-build check on every store-writing verb, the
four named tests (`an_upkeep_run_stops_and_resumes_where_it_stopped`,
`upkeep_and_an_import_exclude_each_other_on_a_month`,
`the_sort_index_is_the_same_bytes_at_1_and_8_workers`, `a_failed_index_open_is_retried`),
DB-027 and DB-N14, the D-entries, `docs/04-invariants.md` rows and `docs/06-limits.md` rows.

## Gate results

Not run: no code was changed, so there is nothing of this part's to gate. The base's own
results are unchanged by this branch (the only diff is this report). The cargo-deny baseline
the task names (RUSTSEC-2026-0344 ruzstd 0.8.2; RUSTSEC-2026-0346/0347/0348/0349 rustls
0.23.45, owned by DENY-1) was not re-measured here.

## NEEDS THE MAC

1. **Merge R3-06 into `feat/db-fix`**, then relaunch R3-17 from the newest `feat/db-fix`.
   The check this part must pass before it starts:
   ```
   git fetch origin
   git log origin/feat/db-fix --merges --format=%s | grep -E "^Merge (R3-06|R3-16|R3-36):"
   ```
   It must print three lines.
2. **Settle the owned-file spelling** (`srt_build*` / `tix_rebuild*` in the spec against
   `srt_builder*` / `tix_rebuilder*` in the tree) in R3-17's spec before the relaunch.
3. **DB-027's decode change is not a cloud file.** Its fix hint points at the day-form strike
   decode in `crates/pull/src/gdfl_nfo.rs`, which a Mac merge-back is changing and R3-17 does
   not own. The plausibility band (against the chain median or same-day spot), its D-entry and
   the journalled `StrikeImplausible` refusal at decode time belong to whoever owns
   `gdfl_nfo.rs`. R3-17's share is the re-key of the three stored names
   (`NSE-FNO-NIFTY-2020-06-25-191250000-CE`, `-191270000-CE`, `-191300000-CE`) through
   `crates/cli/src/gdfl_rekey.rs` as an upkeep step, which needs a real-store run on the
   operator's yes (R3-36's step list) and cannot be done here.
4. Every real-data measurement in the goal (months per second at 1, 4 and 8 workers per
   builder on a private copy; one options month set and one share; the DB-N14 reproduction of
   `/bars/window.json` on `:8080` and a shadow with p50/p99/max and n) is a Mac check. None was
   taken here, and none is claimed.
