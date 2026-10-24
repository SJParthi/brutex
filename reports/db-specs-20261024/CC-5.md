# CC-5 — Copies register (cloud build specification, 24 Oct 2026)

This file is the whole specification of part CC-5 for a cloud session that cannot read the
operator's Mac. Everything the Mac builder of CC-5 would have been pointed at is copied here,
verbatim where it is a specification (fenced `text` blocks, each with its source file and line
numbers). Read `CLAUDE.md`, then `README.md` and `CONTEXT.md` in this folder, then this file in full.

Scope note: this is the **brutex** repository only. "TickVault" appears on the Mac only as the
name of a cloud environment; nothing in this part touches any other repository.

---

## 1. Header

| Field | Value |
|---|---|
| Name | **CC-5** |
| Title | Copies register: every extra copy listed with its bytes, reader and proof; hard links to live archives flagged red; a retirement list for the operator (nothing deleted by code) |
| Kind | `nw` (a new-work part from the 20 Oct follow-ups; its spec is carried in `__ARGS.extraPackages[].spec`, not in EMBED; the launcher's non-`ia` branch of `specStep` applies) |
| Priority rank | **59 of 137** in `__ARGS.priority` (0-based index 58: right after `R4-27-OPT`, right before `CC-8`) |
| Critical path | **No** (not in `__ARGS.criticalPath`) |
| In the build set | Yes (`__ARGS.only` contains CC-5) |
| Decision block | **D-7040..D-7049** (see section 5) |
| Matrix row owned | **FM-607** |
| Base | `feat/db-fix` at `e851da1ca743ae308c8437d5055e0cbaa1d88d83` ("Merge DATA-2: ...") |

### Prerequisites

PREREQUISITES = (package deps `[]` + `__ARGS.extraDeps["CC-5"]` = none) − `__ARGS.dropDeps["CC-5"]` = none
→ **NONE. CC-5 may start at once.**

FILES_ONLY links (`__ARGS.filesOnly["CC-5"]`): **none**.

Parts the spec names as already in place (not launcher prerequisites; checked against
`git log feat/db-fix --first-parent --merges --format=%s`, 222 merge subjects, and `__ARGS.doneExtra`):

| Part | What CC-5 uses from it | Status at e851da1ca |
|---|---|---|
| R3-05 | settings generations (`crates/store/src/settings.rs`, `/settings.json`, `POST /settings`) | **merged** (`Merge R3-05:` present; also in doneExtra) |
| R4-01 | the /ingest panel registry (`web/src/lib/ingest-panels.js`) | **merged** (`Merge R4-01:` present; also in doneExtra) |
| R4-06 | the no-delete source-shape test (`crates/pull/tests/no_scheduler_delete.rs`) | **merged** (`Merge R4-06:` present; also in doneExtra) |
| R4-07 | the downloader that writes only into a new per-run staging folder | **merged** |
| WP-81 | `/disk.json` (`crates/api/src/disk_route.rs`) | **merged** (in EMBED.done) |

Parts that depend on CC-5 (they wait for CC-5, not the other way round):

| Part | Edge | Status |
|---|---|---|
| R4-29 | `__ARGS.extraDeps["R4-29"]` = `["CC-5","IA-3b","SURV-1"]`: R4-29 adds `extra_copies_bytes` to `/disk.json` by calling `pull::copies_register::total(...)` | NOT MERGED (not started) |
| WP-99 | `__ARGS.extraDeps["WP-99"]` includes CC-5 (WP-99 runs last) | NOT MERGED |

Launcher flags in force for the whole run: `noCritic: true`, `noGate: true`; `niftyFirst` unset
(so the NIFTY FIRST paragraph of `fixPrompt` is included, see 2.3). `specAppend["CC-5"]`,
`extraDeps["CC-5"]`, `dropDeps["CC-5"]`, `filesOnly["CC-5"]`, `pendingRepair["CC-5"]`,
`roundBase["CC-5"]`: all **absent**. CC-5 is not in `doneExtra`, `cloudOwned`, `parked`,
`skipFix` or `verified`.

---

## 2. The package's own text

### 2.1 Package entry (`__ARGS.extraPackages[]`, name CC-5)

Source: `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/cloud-20261024/args.json`
(`.extraPackages[38]`; the same object is line 6 of the launcher
`state/fix/db-fix-round-2-run0738.js.fat`). Fields: `deps: []`, `kind: "nw"`,
`source: "state/followups-20261020 (deeplist-gaps.json [2] (CC-5 owns FM-607, which no part owns))"`,
`decisionBlock: "D-7040..D-7049"`. The spec, verbatim:

```text
DECISION BLOCK D-7040..D-7049, recorded in state/decision-blocks.json by the main session. Read CLAUDE.md first: it is law. Delete nothing, ever. No path typed in tracked code. Rust only outside web/. Never the Py-language, not even its name.

YOUR FULL SPEC IS state/cc-gaps-20261017/CC-5.md (FM-607). Read it in full; it binds wherever this text does not amend it.

AMENDED:
(a) ONE-COMMAND (D-6900): no alert kind and no background walk. Do not edit crates/pull/src/alerts.rs. The register walks its roots at three moments, and only then:
- the first request of GET /disk/copies.json after start;
- a request that finds a root's own stat has changed;
- a run of the new read-only verb `cli disk-copies` (crates/cli/src/disk_copies_command.rs plus one dispatch line).
The verb always walks, and exits non-zero when any live archive is hard-linked under another name. A hazard is shown as a red row on the route, on the /ingest panel (web/src/routes/ingest/panels/Copies.svelte, through R4-01's registry) and in the verb's output.
(b) /disk.json's extra_copies_bytes field belongs to R4-29, which owns disk_route.rs (edge R4-29 -> CC-5). Expose one pub function, pull::copies_register::total(...), and R4-29 calls it.
(c) Append R3-05's settings key extra_copy_roots yourself (R3-05 is merged). Unset is a named wait, never a default.
(d) Never delete, move or rename. The retirement list is text for the operator, with no delete command in it: 'the operator removes these by hand; the app never does'.
(e) A hard link is the same (st_dev, st_ino) pair, compared within one walk. An inode number alone is unique only per volume, so it is never compared across devices.
(f) A root's own stat changes only when its direct entries change, so a link made deeper in the tree does not change it. Every answer of the route therefore carries walked_at and the roots it walked, and says how old the walk is. The verb's answer is always a fresh walk.

REAL TREE, READ-ONLY. On 20 Oct, ls -li measured seven names in tools-work/options-queue, `<year>.preserved.<ts>.zip.part` for 2020-2026. Each shares its inode with /Volumes/WD_BLACK/Options/<year>.zip, with nlink 2 (for example, inode 28850662 is 2020.zip). Six other .zip.part names there are not linked. The register must find exactly those seven hard links, plus the four copy groups of CC-5.md, within the stated measurement.

TESTS: every test CC-5.md lists, failing first.
- a_hard_linked_part_name_to_a_configured_archive_raises_one_warning: read 'warning' as the red row and the non-zero exit, since no alert exists.
- a_copy_with_no_reader_is_listed_with_its_bytes
- a_receipt_in_a_copy_root_is_named_as_its_proof
- the_register_never_removes_or_renames_a_file
- an_unset_extra_copy_roots_setting_is_a_named_wait_not_a_default
- the_retirement_list_contains_no_delete_command
- the_same_inode_on_another_device_is_not_a_link
- a_cached_answer_names_its_walk_time

BENCH: /disk/copies.json p50, p99 and max at n=200, served from memory, at 1x, 10x and 100x files. The walk time is measured and stated in docs/06, because the walk is not O(1).

MATRIX: FM-607. DEPS: none (R3-05, R4-01 and R4-06 are merged). GATES: CLAUDE.md section 9.
```

### 2.2 The full spec it points to: `state/cc-gaps-20261017/CC-5.md` (binds where 2.1 does not amend it)

Source: `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/cc-gaps-20261017/CC-5.md`,
lines 1-91 (5,753 bytes, 17 Oct 2026), verbatim. **Where it conflicts with 2.1, 2.1 wins**: in
particular the alert kind in `alerts.rs` (its "Sequenced edits" bullet 2) is CANCELLED by
amendment (a), and the `disk_route.rs` field (its "What done means" item 4) is R4-29's, not
yours (amendment (b)).

```text
# CC-5: Copies register: every extra copy listed with its size, reader and proof; hard links to live archives flagged; a retirement list handed to the operator (no deletion by code)

Draft part spec, 17 Oct 2026. Failure-matrix row: **FM-607** (owner CC-5). Source: `state/compress-check-20261013/gaps.json` CC-5 (kind vendor-sources, generated 13 Oct 2026).

**This part never deletes, moves or renames anything.** The deletions themselves (the extra copies below) and the retirement switches belong to the operator. They are listed in `state/compress-check-20261013/gaps.json` under `triage_20261017.operator_only`, and nothing here acts on them.

## Problem (evidence from gaps.json and its check file)

559 GiB of extra copies have no reader:

| Copy | Size | Notes |
|---|---|---|
| `tools-work/options-nested-proof` | 426.8 GiB | 1,997 day zips again, renamed `day-<n>.zip`, none hard-linked |
| 6 partial `.zip.part` downloads in `tools-work/options-queue` | 81,481,744,630 B (75.9 GiB) | |
| The uncompressed extracted CSV of 25-30 Sep (`tools-work/update-20260925-30/extracted`) | 12.68 GiB | 82,073 files |
| Plain-text proof lists | 44.3 GiB | 2.9-25% of their size at zstd -3 |

This is a third of the drive.

**The hazard.** Seven `<year>.preserved.<ts>.zip.part` names in `tools-work/options-queue` are hard links (same inode, nlink 2) to the live Options 2020-2026 year zips. The old downloader resumed with `curl -C -` into `<out>.part` (download.sh lines 8-10). A downloader resuming into one of those names would change a vendor archive in place, and these are the only originals. Whether any tool still targets those names is UNVERIFIED.

No page or route lists these copies. `/disk.json` counts the store, not the work folders.

## Already in place

- `/disk.json` (`crates/api/src/disk_route.rs`, WP-81) serves the store's bytes by kind and reads `ops/backups.log`.
- R4-07's downloader writes only into a new per-run staging folder and never overwrites, so it never resumes into the hard-linked names. The hazard is the old external scripts, and any future tool that reuses those names.
- R4-06's no-delete source-shape test lists the scheduler-reachable modules. This part adds itself to that list.

## Owned files

- `crates/pull/src/copies_register.rs` (new) and `copies_register_tests.rs` (new):
  - walks the roots the operator names in one settings key (`extra_copy_roots`, appended through R3-05's settings; no default, no typed path);
  - per root, gives bytes, file count and the newest mtime;
  - per file: `nlink`, inode, and whether that inode is shared with a file under a configured live source root (CM archive, options archives, tick store, store);
  - "reader": whether any configured source root, setting or store step names the path;
  - "proof": whether a receipt in the root (a VERIFIED/ALL-PROVED file) states what it proves.
  - The walk is O(files) and bounded by a cap. It is cached and re-measured when a root's own stat changes, and served from memory.
- `crates/api/src/copies_route.rs` (new) and `copies_route_tests.rs` (new): `GET /disk/copies.json`, the register, plus `retirement_list`. That is a plain list (path, bytes, why it has no reader, the proof that a kept copy holds the same bytes) for the operator to act on by hand.
- `web/src/routes/ingest/panels/Copies.svelte` (new, through R4-01's registry), plain tables.

## Sequenced edits

- `crates/pull/tests/no_scheduler_delete.rs` (R4-06): add `pull/src/copies_register.rs`.
- `crates/pull/src/alerts.rs` (R3-21 / R4-05): one alert kind, `live_archive_hard_linked` (path, archive, inode), put once per (path, inode).
- R3-05's settings table: one appended key, `extra_copy_roots`.

## What done means

1. Every extra copy is listed with its bytes, reader (or "none found"), proof (or "none") and whether it duplicates a live source.
2. A non-archive name that shares an inode with a configured live archive raises one alert and a red row. A `*.part` or `*.tmp` name that does so is the hazard case.
3. The retirement list is generated, shown and downloadable as text. It contains no command that deletes. The words say "the operator removes these by hand; the app never does".
4. `/disk.json` gains one field, `extra_copies_bytes` (sequenced one-line edit in `disk_route.rs` after R3-25 / R4-29 merge), so the projection sees the space.

## How it is verified

Failing-first tests, fixture trees built in code:
- `a_hard_linked_part_name_to_a_configured_archive_raises_one_warning`
- `a_copy_with_no_reader_is_listed_with_its_bytes`
- `a_receipt_in_a_copy_root_is_named_as_its_proof`
- `the_register_never_removes_or_renames_a_file` (counting filesystem seam)
- `an_unset_extra_copy_roots_setting_is_a_named_wait_not_a_default`
- `the_retirement_list_contains_no_delete_command`

Real data, read-only on the Mac:
- The register over `tools-work/` lists the four copies above with sizes within the stated measurement.
- It finds exactly the seven hard-linked `.zip.part` names.

Bench:
- `/disk/copies.json` p50/p99/max at n=200, served from memory, under the ratio gate at 1x/10x/100x files.
- The walk time is measured and stated in `docs/06`, because the walk is not O(1).

Gates: CLAUDE.md section 9.

## Prerequisites

- R3-05 (merged: settings).
- R4-01 (merged: panel registry).
- R4-06 for the no-delete list.

## Should it join an existing part?

Small (S). It could join R4-29, which owns `disk_route.rs` and the `/disk.json` projection. As a separate part it owns its own route file and touches `disk_route.rs` by one field after R4-29.

## Rules

- `CLAUDE.md` law. Rust only outside `web/`. Never an interpreted runtime or its name.
- Delete nothing, ever.
- No path typed in tracked code.
- Fail loudly.
- Measured p50/p99/max.
```

### 2.3 `specAppend`, and later amendments that reach CC-5

- `__ARGS.specAppend["CC-5"]`: **absent**. The one-command cut (D-6900), the no-alert rule and the
  R4-29 field hand-off are already folded into the spec text in 2.1 as amendments (a)-(f).
- The other later amendments (options combined 20 Oct, no-fresh 23 Oct, R4-09b, R4-19d, priorities)
  carry no text for CC-5. Their only effect on CC-5 is its place in the priority list (section 1).
- Text in OTHER parts that names CC-5, verbatim (so you know what they expect of you):

Source: `args.json` `.specAppend["R4-29"]`, item (4) of its "FOLLOW-UPS 20 Oct 2026" block:

```text
(4) CC-5's extra copies: /disk.json gains extra_copies_bytes from pull::copies_register::total. That is one field (edge R4-29 -> CC-5).
```

Source: `args.json` `.specAppend["WP-99"]`, line 3:

```text
FOLLOW-UPS 20 Oct 2026 (state/followups-20261020). You are scheduled now and run LAST. Your edges now include every part this follow-up brought into the build (IA-13, R4-29, R4-24, SC-8, SC-11, R3-20, R3-33, R3-34) and every new package (IA-3b, CC-1, CC-3, CC-5, CC-7, CC-8, CC-14, CC-19, VKI-1, and AG-06b from the fix-B follow-ups).
```

Source: `args.json` `._note_followups_20261020` (excerpt naming CC-5's block and the D-6900 rule):

```text
NEW PACKAGES (10, decision blocks D-7000..D-7119, registered by the main session): IA-3b D-7000, CC-1 D-7010 (CC-2 folded in), CC-3 D-7020, CC-7 D-7030 (CC-17 folded in), CC-5 D-7040, CC-8 D-7050, CC-14 D-7060 (CC-13 and CC-15 folded in), CC-19 D-7070, VKI-1 D-7080, AG-06b D-7100; ... No timer, watcher or unattended job is added (D-6900); every scrub, sweep, restate and recount is an attended verb.
```

Source: `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/followups-20261020/main-session.md`
lines 446-452 (the follow-up item that created CC-5 as a real part):

```text
### 7. deeplist-gaps.json [2] (CC-5 owns FM-607, which no part owns)

already_covered: false

TELL THE OPERATOR NOW. This is a hazard check; nothing was changed. On 20 Oct, a read-only ls -li found seven names in work-20260925/tools-work/options-queue that are hard links to the live vendor originals /Volumes/WD_BLACK/Options/2020.zip..2026.zip: same inode, nlink 2. For example, 2020.preserved.1790441703254395000.zip.part has inode 28850662, which is 2020.zip. A downloader that resumed into one of those names would change a vendor original in place, and no off-drive copy exists. Removing the .part names is the operator's hand step: unlinking one of two links leaves the archive's data intact. Claude never deletes.

ASSEMBLER: CC-5 is in .priority (run2021 index 128). Move it to right after CC-1, and add it to .only.
```

(The final priority list places CC-5 at index 58, after the priority re-orders of 21-23 Oct.)

### 2.4 What the Mac builder is told: `specStep(p)` and `fixPrompt(p)`

Source: `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/db-fix-round-2-run0738.js.fat`.
CC-5 is kind `nw`, so `specStep` takes its non-`ia` branch, lines 347-348, verbatim:

```text
  return `1. Read your package: jq '[.work_packages[] | select((.name | split(" ")[0]) == "${p.name}")][0]' ${W}/state/board/fix-plan.json
   Read the full text of every finding it lists (observed, expected, evidence, location, fix_hint) from ${W}/state/board/audit-result.json (key "confirmed", match on "id"). Read the plan's summary and all_sources_view in fix-plan.json if your package touches them. D-number assignments are in WP-02's decisions_needed. ${p.spec || ''}`
```

Results for CC-5 (checked 24 Oct):
- `fix-plan.json` `.work_packages[]` with first word `CC-5`: **none** (empty array). CC-5 is newer
  than the plan; its whole specification is 2.1 + 2.2.
- `audit-result.json` key `confirmed`: **no finding names CC-5** (zero matches). The defect it
  answers is matrix row FM-607 (section 3.2).
- `${p.spec}` is 2.1. D-numbers are CC-5's own block D-7040..D-7049, not WP-02's list.

`fixPrompt(p)`, lines 351-373, is the rest of what the builder reads. Its steps, with their
cloud translation (the Mac-only folders, ports, locks and shadow scripts do not exist in the cloud;
see `README.md`):

| fixPrompt step | Text (summary of the verbatim prompt) | In the cloud |
|---|---|---|
| preamble | `drainFirst` (Mac hold switch), then the shared CONTEXT | `README.md` "Start and hold"; `CONTEXT.md` |
| owner line | `YOU OWN WORK PACKAGE CC-5: "<title>"` | same |
| NIFTY FIRST | prove the work first on NIFTY selections, in the shared instrument-agnostic path, then BANKNIFTY, one F&O stock, one illiquid strike | CC-5 reads no bars and no instrument: it walks folders. The rule's intent that applies: ONE common code path for every root and every archive kind, never a special case for the Options folder or any one name. Say so in your report. |
| reuse an earlier attempt | reuse `wt/P-CC-5` and `fix-CC-5.json`/`verify-CC-5-r<n>.json` if present | none exist on the Mac today (CC-5 has never been started); in the cloud, reuse your own branch `cloud/db-CC-5-20261024` if it exists |
| SAVE AS YOU GO | progress file with an entry per finished piece, a NEXT line, a WIP commit at least every 20 minutes, never squash yourself | same, on your cloud branch, plus `reports/cloud-db-CC-5.md` |
| RULES AUDIT | the `rules-audit-20261011.md` section for the part | **no section for CC-5** (section 3.3); the operator's standing rules in `README.md` bind |
| 1 | `specStep` (above) | this file |
| 2 | worktree `fix/db-CC-5` from `feat/db-fix`; pool `tgt/pool-0` and its lock | branch from `origin/feat/db-fix`; plain `cargo` |
| 3 | do the package completely, tests fail before and pass after; edit only owned files; list any other file in `files_outside_ownership` | same. CC-5 is a new-work part with its own decision block: it lands its own rows in `docs/05-decisions.md`, `docs/04-invariants.md` and `docs/06-limits.md` (re-grep the highest number first; use only D-7040..D-7049) |
| 4 | gates: `cargo fmt --all --check`; clippy `-D warnings` and `cargo test --locked` per touched crate; web build and test if `web/` changed; p50/p99/max for every route touched (n >= 200 or the crate bench) | same, plus the full `README.md` gate list |
| 5 | release build, shadow server on port **8510** (8400 + CC-5's index 110 in the launcher's package list), GET only; a write path only on a root you own; reproduce each finding's selection | **NEEDS THE MAC** (section 6) |
| 5b | self-check: whole diff against repo law and the operator's rules; >= 8 extreme cases (>= 2 combinations, >= 1 out-of-the-box); re-run every fixed finding | same, on fixtures |
| 6 | commit with finding ids and D-numbers, ending `Co-Authored-By`; never push | commit and push only your cloud branch |
| 7 | real-store writes listed under `store_writes_needed`, never run | CC-5 needs none (it writes nothing to the store except, on the operator's own action, one settings generation through `POST /settings`) |
| 8 | JSON report `state/fix/round2/fix-CC-5.json` | `reports/cloud-db-CC-5.md` |

The verify agent (`verifyPrompt`, lines 375-392) re-checks: the whole diff against repo law
(O(1) per operation, naming every loop over files a request can reach; append-only; no fallback that
hides a failure; Rust only outside `web/`; no test that cannot fail), every claimed fix on a
shadow server on port 8610, its own p50/p99/max, and >= 8 attacks of its own. It is BLOCKING if a
state change has no saved and audited record, or a failure no log or page can show. Build so that
it passes the first time.

---

## 3. Findings, matrix, rules audit, earlier cloud spec

### 3.1 fix-plan work package and audit findings

None (see 2.4). Kind `nw`: there is no design row in `ingest-automation.md` (zero matches for
CC-5) and no critic part (`critic-round7-20261011.md`: zero matches).

### 3.2 The failure-matrix row CC-5 owns: FM-607

Source: `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/e2e-failure-matrix.json`,
`.rows[]` with `id == "FM-607"` (the only row whose `owner` is CC-5), every field verbatim:

```text
id:        FM-607
stages:    S1, S9, S10
severity:  high
status:    not_handled
status_note: (no such field on this row)
owner:     CC-5
owner_note: New part CC-5 (draft spec state/cc-gaps-20261017/CC-5.md). It is small and could join R4-29, the /disk.json projection owner. The deletions themselves and the retirement switches are the OPERATOR's (listed in state/compress-check-20261013/gaps.json triage_20261017.operator_only); no code acts on them.
scenario:  559 GiB of extra copies, a third of the drive, have no reader and appear on no page. They are: options-nested-proof (426.8 GiB, 1,997 renamed day zips); 6 partial .zip.part downloads (75.9 GiB); the uncompressed extracted CSV of 25-30 Sep (12.68 GiB, 82,073 files); and plain-text proof lists (44.3 GiB). Seven '<year>.preserved.<ts>.zip.part' names in tools-work/options-queue are hard links (same inode, nlink 2) to the live Options 2020-2026 year zips.
how:       A downloader that resumes into a '.zip.part' name (the old download.sh uses curl -C -) changes a live vendor archive in place. Separately, the space is invisible to /disk.json and to the disk projection.
expected:  A copies register. The operator names the extra-copy roots in one setting (no default and no typed path). Every root is listed with bytes, file count, reader (or none), proof receipt (or none) and whether it duplicates a live source. A non-archive name that shares an inode with a configured live archive raises one live_archive_hard_linked alert and a red row. A plain retirement list is generated for the operator to act on by hand. The code never deletes, moves or renames anything, and joins R4-06's no-delete list. /disk.json gains extra_copies_bytes.
test:      pull copies_register tests: a_hard_linked_part_name_to_a_configured_archive_raises_one_warning, a_copy_with_no_reader_is_listed_with_its_bytes, the_register_never_removes_or_renames_a_file, an_unset_extra_copy_roots_setting_is_a_named_wait_not_a_default, the_retirement_list_contains_no_delete_command.
evidence:  CC-5 (gaps.json; check-vendor-sources.json, re-measured 13 Oct): 7 hard links (nlink 2) of the 2020-2026 year zips under '.zip.part' names; 6 partials 81,481,744,630 B; options-nested-proof du 426.8 GiB; update-20260925-30/extracted 12.68 GiB with 'No reader found'. Whether any tool still targets the hard-linked names is UNVERIFIED.
sources:   CC-5      verdicts: CC-5 = not_handled/high
origin:    compress-check-20261013 gaps.json, triaged 2026-10-17      lens: ingest/db
related:   R4-29, R4-06, R4-07, FM-082
```

Read FM-607's `expected` through amendment (a): **"one live_archive_hard_linked alert" becomes the
red row plus the verb's non-zero exit** (no alert kind under D-6900), and **"/disk.json gains
extra_copies_bytes" is R4-29's one-line edit**, fed by your `pull::copies_register::total(...)`.

Related row (owned by R4-11, not yours; it names FM-607 in its `related` list), scenario verbatim:

```text
FM-082 (owner R4-11, not_handled): Combination: the operator runs the prepared deletion list while an import is still reading that zip. The open descriptor keeps the inode alive, so the current run finishes. The next run or the timer finds the days gone, journals day_not_held, and exits 0.
```

Consequence for CC-5: the retirement list must name, per entry, whether the bytes are the ONLY copy
or have a proven twin, so the operator's hand step never removes a live source an import still needs.

### 3.3 Rules audit

`grep -n '^## .*CC-5' state/design/rules-audit-20261011.md`: **no section** (the audit predates the
part). The operator's standing rules in `README.md` bind in full.

### 3.4 16 Oct cloud spec

`origin/cloud/db-base-20261016:reports/db-specs-20261016/CC-5.md` **does not exist** (CC-5 was
created on 20 Oct). Nothing to add.

### 3.5 The source gap entry and the operator-only list

Source: `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/compress-check-20261013/gaps.json`,
`.unowned_gaps[]` id CC-5, verbatim:

```text
"id": "CC-5",
"kind": "vendor-sources",
"what": "559 GiB of extra copies have no reader: options-nested-proof (426.8 GiB, 1,997 renamed day zips), 6 partial .zip.part downloads (75.9 GiB), the uncompressed extracted CSV of 25-30 Sep (12.7 GiB), and plain-text proof lists (44.3 GiB, 2.9-25% at zstd -3). Seven '.zip.part' names are hard links to the live 2020-2026 year zips.",
"why_it_matters": "A third of the drive. A downloader resuming into a hard-linked '.zip.part' name would change a vendor archive in place.",
"proposed_package_title": "Copies register: list every extra copy with size, reader and proof, flag hard links to live archives, and hand the operator a retirement list (no deletion by code)",
"files_likely": ["crates/api/src/disk_route.rs", "docs/36-ops-runbook.md"],
"depends_on": ["WP-81"],
"test_or_bench_needed": "test: a hard-linked .part name to a configured archive raises a warning",
"owner": "CC-5", "owner_kind": "new", "matrix_rows": ["FM-607"],
"spec": "state/cc-gaps-20261017/CC-5.md",
"owner_note": "No spec lists extra copies or flags hard links to live archives. The register part never deletes. The deletions themselves are the operator's (gaps.json triage_20261017.operator_only). It could join R4-29 (/disk.json).",
"triaged": "2026-10-17"
```

`.triage_20261017.operator_only[0]`, verbatim (what the retirement list is FOR; no code acts on it):

```text
CC-5 deletions: tools-work/options-nested-proof (426.8 GiB); 6 partial .zip.part downloads in tools-work/options-queue (75.9 GiB); tools-work/update-20260925-30/extracted (12.68 GiB); the plain-text proof lists (44.3 GiB); the 7 hard-linked <year>.preserved.<ts>.zip.part names (no bytes freed, removes the in-place-change hazard). Never acted on by code.
```

Evidence in words from `check-vendor-sources.json` (13 Oct re-measure, read-only):
- `tools-work/options-queue` holds 13 `*.zip.part` files: 7 are hard links (nlink 2) of the
  2020-2026 year zips and cost no extra bytes; 6 are unlinked partial downloads, 81,481,744,630 B.
- `tools-work/options-nested-proof` is a full third copy (du 426.8 GiB, 1,997 renamed day zips, none
  hard-linked). Its receipts: `run-<n>/ALL_VERIFIED.txt` (says every configured archive passed nested
  ZIP structure and CRC checks) and per year `<year>/VERIFIED.tsv` (says what it proves: outer
  structure and CRC, inner structure and every entry CRC; its `source_identity` column records the
  source archive as `dev:inode:size:mtime_s:mtime_ns:ctime_s:ctime_ns`). These are the "proof
  receipt" shapes the register reads.
- **The volume's `st_dev` is not stable across mounts** (same file, `check-vendor-sources.json`,
  13 Oct): the nine year zips matched their 27 Sep receipts in inode, size, mtime, mtime_ns and
  ctime, and ONLY `st_dev` had changed (the drive was remounted in between). So a receipt's recorded
  `dev` never proves or disproves identity on its own, and this is why amendment (e) compares
  `(st_dev, st_ino)` only within one walk. A proof rule that matches a receipt's `source_identity`
  against today's file must say which fields it compares and must not require `dev` to be equal.
- The old update flow's receipt shape for the 12 loose days (same file): `update.sh` ran
  `unzip -tqq`, then `zipproof list`, then `zipproof prove`, whose `logs/<stem>.prove.out` reads
  `ALL-PROVED` (the "ALL-PROVED" receipt name of 2.2).
- `tools-work/update-20260925-30/extracted/` (NSE_Tick/INDICES, NSE_Tick/STOCKS,
  NSE_Options_Tick/2026) is an uncompressed fourth copy of the 12 loose days: 82,073 files,
  du 12.68 GiB, no reader found in `crates/` or the design (design row A8 retired extracted folders
  as an import source).
- Proof-list receipts named there: `tickstore-data/verify-options-<year>.txt`, `verify-years.log`,
  the options-nested-proof receipts and `options-queue/<year>.zipproof-list.log`; none is shown in
  the app.
- **The "44.3 GiB of plain-text proof lists" IS itemised** (added by the independent check, 24 Oct):
  `check-vendor-sources.json` `.compression[5]` ("Vendor-source proof leftovers and scan results
  (plain-text lists and ledgers)", measured by `du` 13 Oct; the per-day list count is from
  `audit-vendor-sources.json` `.compression[4]`). Its four folders, in words (sizes as measured;
  they sum to 44.28 GiB):

  | Folder (Mac) | Size, 13 Oct | Reader at e851da1ca |
  |---|---|---|
  | `tickstore-data/prove-20261004` (one `all-proven.txt` of 2,841,424,643 B plus 12,011 per-day lists) | 5,562,496 KiB = 5.30 GiB | none found in `crates/` or `web/src` |
  | the `_v3` extraction ledger under `/Volumes/WD_BLACK/NSE_Options_Tick` (6,178 files) | 2,497,436 KiB = 2.38 GiB | **a reader module exists**: `pull::gdfl_v3` (IA-3-OPT, merged), root from `BRUTEX_GDFL_V3_LEDGER`; no production caller yet (only its own tests call it) |
  | `tools-work/fullscan-20260930` (its cash `files.tsv` alone 2,016,813,525 B) | 19,857,568 KiB = 18.94 GiB | none found |
  | `tools-work/optrescan-20261001` | 18,522,220 KiB = 17.66 GiB | none found |

  Two of these sit INSIDE folders that are live sources: `prove-20261004` is a subfolder of
  `/Volumes/WD_BLACK/brutex/tickstore-data` (the tick store's folder in the 13 Oct checks), and the
  `_v3` ledger sits beside the operator's options tick folders. So "under a configured live root"
  must not by itself mean "has a reader" or "is a live source": the reader rule and the walk's
  root/live-root overlap rule are decisions of CC-5's block (section 5), each with a fixture test.

  The same row says, verbatim: "Design A7 plans to read the _v3 ledger as completeness evidence, so
  it must stay readable as it is. Compressing or deleting any of this is the operator's call." Design
  row A7 (`ingest-automation.md` line 3751) makes it "read-only completeness evidence
  (`pull::gdfl_v3`)"; that module is at the head (`crates/pull/src/gdfl_v3.rs`, `pub mod gdfl_v3;`
  in `crates/pull/src/lib.rs` line 196, `V3_LEDGER_VAR` at line 72). **Consequence for CC-5:** the
  `_v3` ledger is NOT a reader-less copy. If the operator names it as a copy root, the reader rule
  must find it (the configured `BRUTEX_GDFL_V3_LEDGER` root names that path) and the retirement list
  must never offer it as "no reader". So the operator-only list's "plain-text proof lists (44.3 GiB)"
  holds at most about 41.9 GiB of reader-less bytes. State that difference; never hide it.
- The old `download.sh` (outside the repo) resumed with `curl -C -` into `<out>.part`, then renamed
  only on success with the size equal to Content-Length.

---

## 4. Files

### 4.1 Owned by CC-5 (create)

| File | What |
|---|---|
| `crates/pull/src/copies_register.rs` | the register: roots from the setting, one bounded walk, per-root bytes / file count / newest mtime, per-file nlink and `(st_dev, st_ino)`, the live-archive link check, reader, proof, the retirement list, the cache keyed on each root's own stat, and `pub fn total(...)` for R4-29 |
| `crates/pull/src/copies_register_tests.rs` | its tests (fixture trees built in code; a counting filesystem seam) |
| `crates/api/src/copies_route.rs` | `GET /disk/copies.json` (register + `retirement_list`, `walked_at`, the roots walked, the walk's age) |
| `crates/api/src/copies_route_tests.rs` | its tests |
| `crates/cli/src/disk_copies_command.rs` | the read-only verb `cli disk-copies`: always a fresh walk; non-zero exit when any live archive is hard-linked under another name |
| `web/src/routes/ingest/panels/Copies.svelte` | the /ingest panel through R4-01's registry: plain tables, the hazard as a red row, the retirement list as downloadable text |
| web unit test for the panel's pure helpers (for example `web/src/lib/copies.js` + `web/tests/copies.test.js`) | optional, new files only |
| `crates/api/benches/ratio/copies.rs` (new) | the n=200 bench at 1x/10x/100x files |

### 4.2 Shared lines CC-5 must add (minimal; name each in "files outside ownership")

| File | Owner | The edit |
|---|---|---|
| `crates/pull/src/lib.rs` | shared | one `pub mod copies_register;` (with its `#[cfg(test)] #[path = "copies_register_tests.rs"]` test module, as siblings do) |
| `crates/api/src/server.rs` | shared | `#[path = "copies_route.rs"] mod copies_route;` beside `disk_route` (line 1630) and one `.route("/disk/copies.json", ...)` beside `/disk.json` (line 16122) |
| `crates/api/src/operation_audit_tests.rs` | shared | the new route in `EXEMPT` (as `DATA`, like `/disk.json`) or `AUDITED`: the test refuses an unlisted route (D-1445) |
| `crates/api/src/pool_table_tests.rs` | shared | the new route's pool row (as `/disk.json`: `pool(Named::OpsRead, "copies_route.rs", ...)`, or `Runs::Memory` for a cache hit) |
| `crates/cli/src/lib.rs` | shared | `mod disk_copies_command;`, one dispatch arm, and `"disk-copies"` in `COMMANDS` (line 3112, `[&str; 60]` → 61) |
| `crates/cli/src/store_write.rs` | shared | `"disk-copies"` in the read-only (non-writer) verb list, with a one-line reason comment |
| `crates/cli/src/sweep_wiring_tests.rs` | shared | `"disk-copies"` in the non-sweep list |
| `crates/pull/tests/no_scheduler_delete.rs` | R4-06 (merged) | add `crates/pull/src/copies_register.rs` to `DECLARED` (`[&str; 12]` → 13) and `LISTED` (`[(&str,&str); 4]` → 5, `include_str!("../src/copies_register.rs")`). Adding `copies_route.rs` and `disk_copies_command.rs` too is in the spirit of "joins R4-06's no-delete list" |
| `crates/store/src/settings.rs` (+ `settings_tests.rs`) | R3-05 (merged) | one appended `Key` (amendment (c)); `KEYS` 11 → 12; `ALL`, `name`, `slot`, `check`, `check_stored` arms |
| `crates/api/src/settings_route.rs` (+ `settings_route_tests.rs`) | R3-05 / R4-06 (SEC-07) | **two arms, required to compile**: `fn default_of` (line 192) and `pub const fn reader_of` (line 395) are exhaustive `match key` blocks with no `_` arm, so a new `Key` variant does not build without them. `default_of` arm: `(None, "<named wait: no copy root is named until one is saved>")`, never a default value (amendment (c), like `SourceGdfl`'s `"not set: ..."`); `reader_of` arm: `Ok(...)` naming `pull::copies_register`, `GET /disk/copies.json` and `cli disk-copies`. `fallback_reader_of` (line 377) has a `_ => None` arm and needs no edit |
| `docs/02-store-format.md` §42 | shared | the key's row in the settings key table (D-4974's format) |
| `crates/api/benches/ratio.rs` | shared | one `#[path = "ratio/copies.rs"] mod copies;` |
| `docs/05-decisions.md`, `docs/04-invariants.md`, `docs/06-limits.md`, `docs/11-findings.md` if a finding is closed | ledgers | CC-5's own rows only, D-7040..D-7049; the walk's cost row in docs/06 |

### 4.3 Files of other parts CC-5 must NOT touch

- `crates/api/src/disk_route.rs` — R4-29's (`extra_copies_bytes` is R4-29's one-line edit, calling your `total`).
- `crates/pull/src/alerts.rs` — no alert kind (D-6900, amendment (a)); R3-21 / R4-05 own it.
- `crates/pull/src/disk_projection.rs` — R4-07 / R4-11 / R4-29.
- `crates/pull/src/gdfl_dropbox.rs` and every downloader or importer — R4-07, IA-*, R4-11.
- `web/src/routes/ingest/+page.svelte` — the registry mounts your panel with no edit of the page.
- `web/src/lib/ingest-panels.js`, `IngestPanelSlot.svelte` — R4-01's contract; use it, do not change it.
- `crates/api/src/settings_route.rs` — R3-05 / R4-06 (SEC-07): beyond the two required arms of
  4.2 (`default_of`, `reader_of`), no edit. GET `/settings.json` and `POST /settings` then serve the
  new key from `Key::ALL`; the SEC-07 `secret_shaped` refusal stays as it is. (Corrected by the
  independent check, 24 Oct: an earlier draft said no route edit was needed at all, which does not
  compile.)
- Any file under `/Volumes/...` or `tools-work/`: never written, moved, renamed or deleted.

---

## 5. Decision numbers

From `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/decision-blocks.json`:

```text
{ "block": "D-7040..D-7049", "owner": "CC-5 (20 Oct follow-ups)" }
```

None of D-7040..D-7049 appears in `docs/05-decisions.md` at e851da1ca (the newest entries are
D-7131..D-7138, other blocks). Use only these ten. Choices the spec leaves open, each to be a
decision in this block (suggested order, not binding):

1. The setting's name and value form. The spec names it `extra_copy_roots`; every existing key at the
   head is dotted lower-case (`source.gdfl`, `disk.floor`). Keep the spec's name or spell it in the
   dotted family, and record which. Its value holds SEVERAL absolute folders: choose the separator
   (a path may hold a comma), the most roots, and the per-path rules (reuse `settings::directory` /
   `path_shape`: absolute, at most 1,024 bytes, no `..`, exists at save).
2. What counts as a "configured live source root" (spec: CM archive, options archives, tick store,
   store) and where each is read from, with no typed path (section 7 lists what the head has). The
   reader rule also needs the `_v3` ledger root (`BRUTEX_GDFL_V3_LEDGER`, read by `pull::gdfl_v3`):
   it is a configured root that names a folder, so a copy root under it has a reader (section 3.5).
   Read every `BRUTEX_GDFL_*` root through the one existing rule, `pull::gdfl_run::source_root`
   (unset or empty is the named `RunRefusal::SourceUnset`, never a default), not a second reader.
3. The walk's cap (files and depth) and what an over-cap root answers (refused by name, never a
   silent partial).
4. The cache rule: a root's own stat, plus `walked_at` and age on every answer (amendment (f)).
5. The reader rule and the proof rule (which receipt names count, what a receipt must state).
6. The retirement list's text form and its fixed sentence.
7. The verb's exit codes.

---

## 6. NEEDS THE MAC (run at merge-back, read-only on real data)

The cloud cannot run any of this. Write each into `reports/cloud-db-CC-5.md` with its result
`NOT RUN (cloud)`. The Mac runs them in its step 5 with the part's shadow port **8510**
(the second agent: 8610), pool `tgt/pool-0`, binaries in `tgt/bin-CC-5/release/`.

W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925

1. **The real hard links, before anything (read-only baseline).**
   `ls -li $W/tools-work/options-queue/*.zip.part /Volumes/WD_BLACK/Options/20*.zip`
   Expected: seven `<year>.preserved.<ts>.zip.part` names (2020-2026), each with nlink 2 and the
   same inode as `/Volumes/WD_BLACK/Options/<year>.zip` (20 Oct: inode 28850662 = 2020.zip); six
   other `.zip.part` names with nlink 1, together 81,481,744,630 B. If this differs, the operator
   has acted since: record the new baseline, never "fix" the tree.

2. **A private settings root for the check (never a POST to a shadow whose bars are the real
   store).** Make a store root owned entirely by the check (an empty temp store, no bars), serve it
   with the CC-5 release `api` on port 8510, and save one generation through
   `curl -s -X POST http://127.0.0.1:8510/settings -H 'content-type: application/json' -d '{"expected":0,"changed_by":"cc5-check","reason":"copies register check","set":{"<the key>":"<the roots>"}}'`
   with the roots = `$W/tools-work/options-nested-proof`, `$W/tools-work/options-queue`,
   `$W/tools-work/update-20260925-30/extracted` and the four proof-list folders of section 3.5
   (`/Volumes/WD_BLACK/brutex/tickstore-data/prove-20261004`, `$W/tools-work/fullscan-20260930`,
   `$W/tools-work/optrescan-20261001`, and the `_v3` ledger under
   `/Volumes/WD_BLACK/NSE_Options_Tick`; all four present by `ls -d` on 24 Oct), and the live-source
   settings/environment pointing at `/Volumes/WD_BLACK/Options` (options archives), the CM archive,
   the tick store and the real store root (read-only use). Expected: `200`, a new generation; a
   value with no such folder is refused naming the key. Also check: the value is NOT refused
   `secret_shaped` (a folder path is not a `/<org>/<env>/<vendor>/<field>` shape).

3. **Unset is a named wait.** Before step 2, `curl -s http://127.0.0.1:8510/disk/copies.json`
   Expected: a named wait (the key named, no default root walked), not an empty register and not 500.

4. **The register over the real tree (GET only).**
   `curl -s http://127.0.0.1:8510/disk/copies.json | jq '{walked_at, roots: [.roots[] | {path, bytes, files, reader, proof, duplicates_live}], hazards: [.. | objects | select(.hazard? == true)] | length}'`
   (field names as built.) Expected, within the 13 Oct measurement:
   options-nested-proof about 426.8 GiB with proof = its `ALL_VERIFIED.txt` / `VERIFIED.tsv`
   receipts and no reader; the six partials 81,481,744,630 B, no reader; extracted about 12.68 GiB,
   82,073 files, no reader; the proof lists about 44.3 GiB in all (prove-20261004 about 5.30 GiB,
   fullscan about 18.94 GiB, optrescan about 17.66 GiB, no reader; the `_v3` ledger about 2.38 GiB
   WITH a reader when `BRUTEX_GDFL_V3_LEDGER` is set, and kept off the retirement list);
   **exactly seven red hazard rows**, one
   per hard-linked `.zip.part` name, each naming its archive and `(st_dev, st_ino)`.
   `walked_at` and the walk's age are present on every answer.

5. **The verb.** `$W/tgt/bin-CC-5/release/cli disk-copies; echo "exit=$?"` with `BRUTEX_STORE` set to
   the private root of step 2. Expected: the same seven hazards, the four groups, a fresh
   `walked_at`, and a NON-ZERO exit while the seven links exist.

6. **Nothing changed.** Repeat step 1 and compare: identical names, inodes, nlink and sizes. Also
   `find $W/tools-work -newer <a stamp file made before step 4> -print | head` prints nothing written
   by the check (the register opens nothing for write).

7. **The cache rule on the real tree.** Two GETs in a row: the second is served from memory with the
   same `walked_at` and a larger age. Then `touch` a NEW empty file inside a private copy-root you own
   (never inside `tools-work/`), GET again: re-walked, new `walked_at`.

8. **The bench.** `cargo bench -p api --bench ratio -- copies` (or the name built) and n >= 200
   sequential `curl` GETs of `/disk/copies.json` against 8510: p50, p99, max at 1x/10x/100x files;
   the cold walk's time over the real `tools-work/` measured and written into the `docs/06` row.

9. **The /ingest panel.** Open `http://127.0.0.1:8510/ingest` (browser pane, only if no other agent
   uses it): the Copies panel mounts through the registry, shows the four groups and seven red rows,
   the retirement list downloads as text, contains no delete command and says the operator removes
   these by hand and the app never does. Fits 375 px.

10. **After merge, on :8080 (the main session's redeploy, not the builder's):** steps 3-4 and 9 again
    on the real app; screenshot to the operator.

Store writes needed: **none** by code. The settings generation of step 2 on the REAL store is the
operator's own `POST /settings` (or his yes) at merge-back.

---

## 7. Code facts at e851da1ca (grep, not guesses)

Owned files: **all absent at the head** — `crates/pull/src/copies_register.rs`,
`copies_register_tests.rs`, `crates/api/src/copies_route.rs`, `copies_route_tests.rs`,
`crates/cli/src/disk_copies_command.rs`, `web/src/routes/ingest/panels/Copies.svelte`.
`git grep 'extra_cop\|copies_register\|copies.json\|disk-copies'` over `crates web/src docs`: **no
match**. You start from nothing.

Existing code you build on:

- **Settings** — `crates/store/src/settings.rs` (1,463 lines; R3-05, format docs/02 §42, D-4974).
  `pub enum Key` (11 keys: `WindowFrom`, `WindowTo`, `OpenFeed`, `OpenInstrument`, `OpenSegment`,
  `OpenTimeframe`, `OpenOrder`, `AlwaysShown`, `SourceGdfl`, `SourceMasters`, `DiskFloor`),
  `pub const KEYS: usize = 11`, `Key::ALL`, `Key::name` (dotted names such as `source.gdfl`,
  `disk.floor`), `Key::of_name`, `Key::env` (only `SourceMasters` → `BRUTEX_MASTERS`), `Key::slot`,
  `Settings::get`, `pub fn check(key, value)` (empty value refused: "unset the key to restore its
  fallback"; one line; per-key arms; `SourceGdfl | SourceMasters => directory(value)`),
  `fn check_stored` (source folders checked for shape only via `path_shape`), `Reader::new/refresh/newest`,
  `pub fn save`, `MAX_PATH_BYTES = 1_024`, `MAX_FILE_BYTES = 64 KiB`. `DiskFloor` was appended by
  R4-07 without a format-version bump (VERSION stays 1): the precedent for appending your key.
  Drift: docs/02 §42 (heading line 4784, "The ten keys" at line 4818) still says ten and has no
  `disk.floor` row. The same section says an unset key "is answered by its named fallback ... every
  other key by its caller's own default"; CC-5's key is the exception amendment (c) makes, so its
  docs/02 row must say "unset: a named wait, no default", matching its `default_of` arm (4.2).
- **Settings route** — `crates/api/src/settings_route.rs`: `settings_json` (GET `/settings.json`),
  `settings_post` (`POST /settings`, body `{"expected", "changed_by", "reason", "set", "unset"}`),
  `pub fn effective(snapshot, key) -> Effective` (line 355), SEC-07 refusal through
  `brutex_core::secret_shape` (a folder path passes; a 4-segment path ending in a credential field
  name is refused).
- **`/disk.json`** — `crates/api/src/disk_route.rs` (2,136 lines, R4-29's): `pub(super) async fn disk_json`
  (line 1269), `DISK_TTL = 30 s`, `fn file_id(meta) -> (u64, u64)` returning `(dev, ino)` (line 387)
  — the same `(st_dev, st_ino)` rule amendment (e) asks for, used there to count a census file once;
  ops reads run outside Tokio's worker pool (`crates/api/src/detail.rs` line 538, the `OpsRead` pool).
  Declared in `server.rs` at line 1630 (`#[path = "disk_route.rs"] mod disk_route;`), routed at
  line 16122. `server.rs` is 36,278 lines.
- **Live source roots at the head** (names only, no typed path): `pull::gdfl_run::TICKSTORE_VAR =
  "BRUTEX_GDFL_TICKSTORE"`, `CM_ARCHIVE_VAR = "BRUTEX_GDFL_CM_ARCHIVE"`, `NFO_ARCHIVES_VAR =
  "BRUTEX_GDFL_NFO_ARCHIVES"` (`crates/pull/src/gdfl_run.rs` lines 85-89, "the directory of the
  vendor's yearly options zips"), all read through `pub fn source_root(var, env)` (`gdfl_run.rs`
  line 221: unset or empty is `RunRefusal::SourceUnset { var }`, "named, never defaulted");
  `pull::gdfl_v3::V3_LEDGER_VAR = "BRUTEX_GDFL_V3_LEDGER"` (`crates/pull/src/gdfl_v3.rs` line 72,
  the `_v3` extraction ledger, "No default (D-4867)"); the store root from `BRUTEX_STORE`
  (`server.rs` line 257, `store_dir_from` at line 291); settings `source.gdfl` and `source.masters`.
- **No-delete test** — `crates/pull/tests/no_scheduler_delete.rs` (526 lines, R4-06):
  `const DECLARED: [&str; 12]`, `const LISTED: [(&str, &str); 4]` (by `include_str!`), named
  `ALLOWED` exceptions; refuses std/tokio removes, `unlink`, `unlinkat`, `rmdir`, `set_len`,
  `ftruncate`, any `rename`, `truncate(true)`, `File::create`, `libc::remove`, `Command` in listed
  production code; `every_declared_module_that_exists_is_listed` fails for a declared file that
  exists unlisted.
- **Panel registry** — `web/src/lib/ingest-panels.js` (`panelRegistry`, `answerLine`, `createPoller`):
  every `web/src/routes/ingest/panels/*.svelte` mounts by its `<script module>` exports `order`
  (finite number) and `title` (non-empty string); one prop `ctx` (`{ feed, display, archive }`);
  answers 404/503 with one named line; titles every control; fits 375 px; polls at most once a
  second and never while hidden. Existing panels: `ReadPools.svelte` (order 90), `ZerodhaRepair.svelte`.
- **cli verbs** — dispatch in `crates/cli/src/lib.rs` (e.g. line 3044
  `["tick-to-second-check", arguments @ ..] if !arguments.is_empty() => tick_second_check::command(arguments, out)`),
  `const COMMANDS: [&str; 60]` (line 3112); verb files `*_command.rs` (for example
  `alerts_command.rs`, `upkeep_command.rs`); `store_write.rs` keeps the writer/reader split and its
  test refuses a verb in neither or both; `sweep_wiring_tests.rs` lists non-sweep verbs.
- **Benches** — `crates/api/benches/ratio.rs` mounts `ratio/*.rs` by `#[path]` (calendar, census,
  ..., settings, sources, spot, window); `crates/pull/benches/ratio.rs` likewise (day_table,
  disk_projection, fno_membership, identity, tickstore_write).
- **Crate graph** — `pull` depends on `core costs greeks store telemetry`, so `pull::copies_register`
  may read `store::settings`; `api` and `cli` both depend on `pull`. No new arrow is needed.
- **Downloader** — `crates/pull/src/gdfl_dropbox.rs` (R4-07, merged) writes only into a new per-run
  staging folder; the hazard is outside tools and future name reuse, as the spec says.
- **`docs/36-ops-runbook.md`** exists (named in the gap's `files_likely`; the copy procedure that
  appends `ops/backups.log`). A short "extra copies and the retirement list" paragraph there is
  optional and must contain no delete command.

---

## Return to the Mac (fill in your report)

`STATUS`, gates with commands, p50/p99/max with n and method, the >= 8 extreme cases (suggested:
empty root; one-file root; a root at the cap; a root that vanishes mid-walk; a hard link across two
devices with an equal inode number (amendment (e)); a link made deep in the tree after a cached walk
(amendment (f)); a symlink loop; a name with a comma or a line break; an unreadable subfolder; 20
concurrent GETs during a cold walk; a `.tmp` name linked to an archive; a receipt that is torn or
states nothing; a receipt whose recorded `dev` differs from today's after a remount while inode,
size and times match (section 3.5); a copy root that lies inside a configured live root, as
`prove-20261004` does), files outside ownership (the 4.2 list, `settings_route.rs` included), D-numbers
used, and the NEEDS THE MAC list of section 6.
