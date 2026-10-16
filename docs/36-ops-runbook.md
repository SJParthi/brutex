# 36 — Operations runbook: deploy, rollback, census seal, backups, free space

**No authority** (`CLAUDE.md` §10: like `docs/12-` to `docs/35-`, this is a
working document). It is the procedure the WP-81 code serves, written down
so that the operator, or the session that deploys, runs the same steps every
time. Where it and a decision disagree, the decision wins and this page is the
stale copy. Findings DB-B51, DB-B58, DB-B59, DB-B65, DB-B68, DB-B70, DB-B71,
DB-B72, DB-B73, DB-B74, DB-B75 and DB-B79 (`state/board/audit-result.json`).

Nothing here names a credential, a vendor row or a parameter path.

---

## 1. What the code now says, and where to read it

| Question | Where the answer is |
|---|---|
| Which build answered this number? | every response's `x-brutex-build` header (the 40-hex commit, or `unstamped`); the line under the nav on every page; the last line of `/health`; the `build:` line of the start banner; the `build` field of the `api.serve listening` event (DB-B71) |
| Which census format answered? | `/store.json`'s `x-brutex-census-format`: `v4-partitioned` or `v3-single` (DB-B68) |
| Has a pre-version-4 writer touched a migrated census? | `/store.json`'s `x-brutex-census-legacy-drift` (empty when nothing moved), the `legacy drift:` lead of `x-brutex-census-reason`, the state word `behind`, and `/audit.json`'s `store.legacy_drift` (DB-B70) |
| How much room is left, how big is the store, what does the census keep, is there a copy? | `GET /disk.json` and the disk line on `/ingest` (DB-B73, DB-B59) |
| Was a request refused because the server was busy? | a 429 always carries `Retry-After`; a full pool first queues a request for up to 250 ms (DB-B51) |
| Is this server allowed to write? | `BRUTEX_READ_ONLY=1` refuses every non-GET/HEAD request (403), grounds the autopilot and does not resume a spot recovery left in flight; the start banner's `READ-ONLY:` and `recovery not resumed:` lines and one `api.recovery` event say so (DB-B79, §7). A build with no commit stamp is held to the same three (DB-B71) |
| Has every block been checked against its checksum? | `cli scrub-blocks VENDOR`; its findings in `audit/scrub-blocks.<vendor>.log` and its pass in `audit/scrub-blocks.<vendor>.cursor`; each feed's `scrub` in `/disk.json` and the disk line on `/ingest`; `/scrub.json?feed&timeframe&key` and the note under the `/db` grid for the series on the page (DB-B74) |
| Is a month file the month its name says? | every bar door asks the month's `.prv` (`StoreError::Misfiled`); `cli scrub-blocks` counts `misfiled` months and the `unbound` months no `.prv` binds (DB-B65, §8) |
| May this build read this store's census? | `manifest/REQUIRES`; a build without a named feature answers the census `unreadable` with the file, the line and the feature (DB-B68, §2) |
| May this `cli` write the store? | a build with no commit stamp refuses every store-writing verb unless `BRUTEX_ALLOW_UNSTAMPED_WRITE=1` (DB-B71, §3) |

## 2. Store-format readers: the census seal (DB-B68)

`cli census-migrate VENDOR` now SEALS the single file the moment it is
superseded:

1. the single file's inode is kept, byte for byte, as
   `manifest/<vendor>.man.v3-superseded` (a hard link, nothing copied);
2. a 32,768-byte stub takes the name `manifest/<vendor>.man`; both of its
   header slots declare manifest version 65535, which no reader built so far
   knows, so a reader built before version 4 answers
   `unknown manifest version 65535` and serves NOTHING as held — instead of
   the superseded census as the whole of it, which is what rolling back to
   `tgt/app5` did (139,295 GDFL instrument-months and 368,855,733 rows gone
   without a word);
3. `manifest/v4/<vendor>/SEALED` records the superseded file, its length and
   `reader_since=bd6b6f5a441aedc492ec19f75ad9d294de413fb4`, the first commit
   whose API reads a version-4 census. It is staged as `SEALED.tmp`, flushed
   and renamed, so the name never holds half a receipt; a re-run READS it
   (`format=census-seal-v1`, `superseded=`, `superseded_bytes=`,
   `reader_since=<40 hex>`, each a whole line) rather than trusting that a
   file of that name exists. A receipt no seal finished writing is kept as
   `SEALED.torn-<unix>` and a whole one written; one of another `format=` is
   refused by name. A deploy script reads `reader_since` only from a file
   whose first line is `format=census-seal-v1`.

The GDFL census was migrated on 2026-10-10, before the seal existed. Seal it
once, after the WP-81 build is live (store write; the main session's, WP-80):

```
BRUTEX_STORE=<real store> <tgt>/release/cli census-migrate gdfl --seal
```

It refuses, naming both lengths, if `manifest/gdfl.man` is no longer the
length the migration recorded (count those months first with
`cli census-recount`). Run again, it confirms and writes nothing.

4. `manifest/REQUIRES` gains the line
   `requires=census-v4 vendor=<vendor> since=<commit> at=<unix seconds>`
   (append-only text, first line `format=store-requires-v1`).

The stub refuses the builds that ALREADY exist. `manifest/REQUIRES` refuses
every build from here on: a census is located only after that file is read,
and a line naming a feature the build does not have (or a file it cannot
read, a line of no known shape, another `format=`) makes the vendor's census
`unreadable` by name — `/store.json` answers 503 with the file, the line and
the feature — instead of serving whichever older shape the build knows. The
next format change appends one line; nothing else has to remember. A server
that is ALREADY RUNNING reads the file when it next loads that vendor's
census (a census change, or a restart), not the instant the line is appended:
after a seal or a format change, restart the API, or confirm
`x-brutex-census-state` on `/store.json` before trusting it.

**Any build a deploy or a rollback starts must descend from `reader_since`:**

```
git -C <repo> merge-base --is-ancestor bd6b6f5a441aedc492ec19f75ad9d294de413fb4 <candidate commit>
```

## 3. Deploy with a rollback (DB-B72)

What `state/fix/deploy-8080.sh` did wrong, measured on 10 Oct: it rebuilt
`wt/APP/web/build` in place 7.7 minutes before it stopped the old API (so the
old API served the new bundle), pointed `BRUTEX_WEB` at that mutable tree (so
every target served the same bundle), stopped the old server before the new
one was known to answer, exited with the old server dead when the new one did
not answer, and wrote no `deploys.log` line. The procedure:

1. **Snapshot first** (§4) when the deploy is followed by any store write.
2. Build api and cli in release into the warm build directory, then copy both
   binaries AND `web/build` into a new `tgt/<name>/` — `tgt/<name>/web/build`
   is that deploy's frozen front end, and `BRUTEX_WEB=tgt/<name>/web`.
3. Refuse the candidate unless its commit descends from every
   `manifest/v4/*/SEALED`'s `reader_since` (§2) and its binary is stamped
   (`/inspection.json`'s `commit` is not null).
4. Start it on a side port with `BRUTEX_READ_ONLY=1` over a **shadow root of
   the store, never the store root itself**, and probe: `/health` answers,
   `/db` answers 200, `/store.json?feed=gdfl`'s `x-brutex-census-state` is not
   `unreadable`, and `x-brutex-build` equals the candidate's commit. Stop it.
   Any failure: stop here, the old server still serves.

   *Why a shadow.* One store root is served by one api: the candidate started
   over the real root while :8080 serves it exits at once with `REFUSED:
   another brutex api is already serving this store` (`serve.lock`),
   `BRUTEX_READ_ONLY=1` or not — read-only refuses writes, it does not share
   the lock, and a read-only server still writes its own lock, log,
   invocation journal and pull-journal read index (§7). This step said "over
   the real store" until 12 Oct and could not be run as written. A shadow
   root (§7) is a new directory whose `manifest/`, `imports/` and `audit/`
   are clones of the store's (`cp -c`, copy-on-write) and whose `bars/` is a
   link to the store's `bars/`.

   *What the probe proves and what it does not.* It proves the candidate
   binary starts, serves its pages, reads THIS store's census as it stood
   when the shadow was cloned (seal, `REQUIRES`, format) and reads its bars.
   It does not prove the candidate takes the live store's `serve.lock`, nor
   that it reads a census a writer has moved since the clone: step 5 is the
   first time it meets the real root, which is why step 5 probes again and
   rolls back.
5. Record the running target (`tgt/current`) as `tgt/previous`; stop the old
   server; start the candidate on :8080; probe as in step 4. If it fails,
   start `tgt/previous` again on :8080 with its own `BRUTEX_WEB`, and say so.
6. Point `tgt/current` at the candidate and append one line to
   `state/fix/deploys.log`:
   `<UTC> new=<name>@<commit> previous=<name>@<commit> result=<ok|rolled-back> census=<format words>`.

A rollback is step 5 with `tgt/previous` as the candidate, after step 3's
reader check.

A server that is already running reads `manifest/REQUIRES` when it loads a
census, not on a timer: a line added while it runs was seen to change nothing
until the server was restarted. Sealing (§2) is therefore followed by a
restart of every server over that root, and step 3 is checked against the
file, never against what a running server answers.

**Never write over the file of a program that is running** (rules audit
RA-334): a deploy copies into a NEW `tgt/<name>/` (step 2) and moves
`tgt/current` to it; it never rebuilds or copies over the `api` or `cli` a
live server or a running import was started from. A child run whose program
file changed under it can die mid-day, and the next one then refuses
`build_skew`: the action is to restart the app from the new target, then
press Resume.

**Which build is stamped, and what an unstamped one may do.** A binary
carries its commit when every tracked and untracked file that can be a
compilation input equals `HEAD` at build time. `web/` is OUTSIDE that proof
(`crates/cli/build_provenance.rs`, `front_end`: no crate may build from `web/`,
`CLAUDE.md` §2), so `npm run build` before `cargo build` does NOT cost the
stamp: a `cli` built over 59 modified or deleted `web/build` paths carried its
commit (measured by the second agent's check of WP-81, 2026-10-11). An earlier
edition of this section said the opposite; it was wrong, and the deploy order
needs no change for stamping. What does cost the stamp is a dirty Rust source,
`Cargo.toml`, `Cargo.lock` or toolchain file, or a tree with no `.git`.

An unstamped build names itself on every answer (`x-brutex-build: unstamped`,
the page's build line, the banner) and writes nothing:

- `cli` refuses every store-writing verb (`gdfl-import`, `census-recount`,
  `census-migrate`, `gdfl-dayflags`, `scrub-blocks`, `tix-rebuild` without
  `--dry-run`, `gdfl-rekey --apply`, `zerodha-repair --from-store`) with
  `this build carries no commit stamp`.
- the API answers 403 with the same sentence to every request that is not a
  GET or a HEAD — a Zerodha pull, an import queued from `/ingest`, a repair, a
  masters refresh, a sweep — keeps the autopilot on the ground, and prints
  `NO WRITES: …` in its start banner (DB-B71).

`BRUTEX_ALLOW_UNSTAMPED_WRITE=1` in the environment of either binary lets it
write on purpose; its journal, scrub and provenance lines still say
`unstamped`. Before a deploy, check the candidate:
`curl -sI http://127.0.0.1:<side port>/health | grep -i x-brutex-build` must
show 40 hex digits.

## 4. Backups and snapshots (DB-B73)

One drive holds the purchased archives (`Options/`, the capital-market zip,
the tick store) and the whole store; Time Machine has no destination and
excludes both. Until a second drive is attached nothing below can be done, and
`/disk.json` says `"recorded": false` on every read — loudly, on purpose.

- **Independent copy.** A second drive of at least the store plus the raw
  archives. Copy raw and store with `rsync -a --delete-after` (raw archives
  never change, so after the first copy only new files move), then append one
  line to `<store>/ops/backups.log`:
  `copied=<UTC> dest=<path> raw_bytes=<n> store_bytes=<n> by=<who>`.
  `/disk.json` serves the newest WHOLE line that carries `copied=` and `dest=`
  as `backup.last`; a newest line that is anything else (a note, a shell error,
  half a line) is `recorded: false` with `unreadable receipt`, never "the last
  copy". The log must be a regular file: a link or a FIFO at its name is
  refused by name. Write the date as `copied=YYYY-MM-DDTHH:MM:SSZ` (UTC): every
  page carries a standing line while no copy is recorded, while the last one
  is more than 7 days old, or while its `copied=` word is not a date in that
  form or is after now (`web/src/lib/CopyAlert.svelte`, rules audit RA-163).
  The line is the page's; nothing sends an alert when no page is open (the
  alert kind is NEW-25's).
- **Snapshot before every format or bulk write** (census migrate or seal,
  recount, a re-import, a deploy that is followed by one) — **NOT PROVED on
  this drive; do the drill below before relying on it** (rules audit RA-288).
  What is known, read on this Mac on 2026-10-11 (macOS 27.0.1):
  `/Volumes/WD_BLACK` is APFS (`mount`); `tmutil listlocalsnapshots
  /Volumes/WD_BLACK` lists none; `tmutil destinationinfo` answers `No
  destinations configured`; and `man tmutil` gives `localsnapshot` NO mount
  point — "Create new local Time Machine snapshots of all APFS volumes
  included in the Time Machine backup". This drive is excluded from Time
  Machine, so `tmutil localsnapshot /Volumes/WD_BLACK`, the form earlier
  editions of this section and the WP-80 notes give, has never been shown to
  snapshot it. Nobody has taken a snapshot of this drive, timed one, measured
  what it pins, or restored one month from one.
- **The drill** (the operator's; it changes what the drive keeps, so no agent
  ran it). On a scratch APFS volume first, then once on this drive: (1) take
  the snapshot and confirm it with `tmutil listlocalsnapshots <mount point>` —
  if the list is still empty the command did not cover the volume, and the
  undo does not exist; (2) write one month into a scratch store on that
  volume; (3) mount the snapshot read-only and copy that one month's files
  back by name; (4) compare bytes (`shasum -a 256`) with the copy taken before
  step 2; (5) write the commands, the times and `df` before and after here. If
  one month cannot be brought back alone, say so here. Until the drill is
  written into this section, the undo of a bad bulk write is: the store's own
  append-only history (a repair is a new generation, a seal keeps the single
  file by hard link, a compaction keeps every epoch) and the independent copy
  above — which does not exist yet either.
- **A snapshot is deleted once the run it guarded is checked green** (rules
  audit RA-297): `tmutil deletelocalsnapshots <mount point | date>` (`man
  tmutil`). A kept snapshot pins every block replaced after it — repair
  generations, compacted census epochs, rolled logs — as space no file
  listing shows. Whether `df`'s free figure, which is what `/disk.json`
  serves, falls by it is UNVERIFIED and is part of the drill (step 5).
  `/disk.json` does not list snapshots. `tmutil listlocalsnapshots
  /Volumes/WD_BLACK` before every bulk write: an old one there is a deletion
  that was forgotten.
- **Test shadows are not backups.** `shadow.sh` deletes and re-clones a shadow
  of the same name on every run.

## 5. Free space and the census's retention (DB-B73, DB-B59)

`GET /disk.json` answers, measured at most every 30 seconds:

- `filesystem` — total, used and free bytes under the store root and its
  mount point (one `statvfs(2)` in the server process, D-5240; a root that
  does not exist or is no directory is refused by name);
- `feeds[]` — per vendor: months, rows, `bar_bytes_lower_bound` (a header
  region per month plus 56 bytes per bar; sidecars and time indexes not
  counted), and every census file with its role;
- `census_kept_bytes` — the bytes of older epochs and superseded single files;
- `projection: null` — the server does not know how much raw data remains to
  import, and projects nothing. The audit's figure (388.2 GB of options raw
  left at a measured 3.69 store-to-raw ratio, about +1.43 TB) is an
  extrapolation the operator compares against `free_bytes`.

**Retention rule:** every census epoch and every superseded single file is
kept and never deleted (`CLAUDE.md` §3 rule 8; `census_store` "never
deleted"). Each compaction adds one epoch of the partition's size (about 280
MB per FNO compaction at the full options import, extrapolated). Archiving an
old epoch OFF this drive after a verified backup is the only reduction this
page allows; it needs a decision, and `docs/06-limits.md` must state the
budget (WP-99).

## 6. Mid-file rot (DB-B74)

`/verify.json`'s scrub reads the first and last record of each month, so a
rotted block in the middle passes it. The full-block pass is a CLI verb:

```
BRUTEX_STORE=<store> <tgt>/release/cli scrub-blocks gdfl [--months N] [--mib-per-s 64]
```

It reads every committed block of every month the census holds, against its
stored checksum, in path order, at most 64 MiB/s by default, and writes only
two append-only files per vendor under `<store>/audit/`:

* `scrub-blocks.<vendor>.log`, the findings: a `run` line per run (its id,
  the build's commit, the pass), one `bad` line per damaged block (month,
  block, records, the store's own reason), `unverifiable` for a month whose
  `.crc` is gone, `misfiled` for a month whose `.prv` names another month
  (§8), `busy` for a month a writer held (skipped, not waited for: the pass
  did NOT read it) and `missing` for a month the census holds and the disk
  does not;
* `scrub-blocks.<vendor>.cursor`, where the pass stands: `progress` and `done`
  lines carrying the pass's totals. A run resumes after the last `progress`
  cursor; after `done` the next run starts the next pass. One run per vendor
  at a time (a second is refused by name); a cursor file no line of which can
  be read is refused, never restarted in silence.

A run killed, or out of disk, in the middle of an append leaves a line no
newline ends. The next run ends it first, in both files, as a void line (the
fragment followed by ` #void`), writes `torn <file> bytes=<n> run=<id>` to the
ledger and prints `torn tail ended`; no reader reads a void line, and a cursor
line is read strictly — exactly the words the scrub writes, in its order — by
one parser the CLI and the API share. Run ids never repeat for a vendor (the
clock's second, or the next after the newest id on file). Both files must be
plain files: a link, a FIFO or a directory at either name is refused by name,
by the CLI and by `/scrub.json` and `/disk.json` (`recorded: false` with the
reason). `scrub-blocks` is a store writer to the unstamped-build rule (§3).

`/disk.json` serves each feed's pass, state, totals and standing findings,
and `/ingest`'s disk line says `<feed>: scrub pass N …, M months, K bad
blocks` — or `<feed>: blocks never scrubbed`. `/scrub.json?feed=<vendor>&
timeframe=<rung>&key=<series>` answers the findings that still stand for up
to 64 series (32 findings each, the rest counted), from an index of the WHOLE
ledger kept by byte offset: only the lines appended since the last request
are read (at most 16 MiB per request; a longer backlog after a restart
answers `truncated: true` until the next requests have caught up), so no
standing finding drops out behind newer ones; `/db` prints them under its
grid. A finding stands until a later COMPLETE pass reads its month
without finding it. A month being read refuses its writer for as long as
its blocks take to read, and no longer: what the rate limit owes is slept
with no month open (rules audit RA-128; `--mib-per-s` no longer lengthens the
hold). The hold is O(blocks of one month) and is NOT MEASURED on this drive,
so a scrub and an import of one vendor can still meet on a month — the scrub
counts it `busy` and moves on, the importer sees the month locked and
retries. A busy month is NOT READ by that pass and nothing is lost by it:
what earlier passes found in the month still stands (each such finding says
the pass that found it and the pass that kept it), `/db` names the month as
not read under the grid, the `/ingest` tile says `N months NOT READ (busy)`,
and the verb's own output ends with a `NOT READ` line; the next pass reads
it. A pass whose totals say `busy=` above zero is therefore not a clean
pass, and no page shows it as one. Nothing yet makes the scrub yield to a live run of the kind or says
`scrub paused` on a page; until that exists, prefer not to run both for one
vendor. A pass also walks the census as it stood when its run began, in path
order: months an import adds BEHIND the cursor while the pass runs are not
read by that pass, and its `done` line does not say so — they are the next
pass's. `complete` therefore means "every month the census held when each run
of the pass began, from its cursor on", not "every month now on disk". On the
operator's ~437 GB of bars a pass is hours at the default rate (extrapolated
from the rate, not measured). A month suspected of rot can also be read whole:
`/bars.json?...&month=<yyyy-mm>` answers 206 with `faulted_records` and one
sentence per damaged block (DB-B75). Repair is a re-import into a new
generation (DB-N22), never an edit.

## 7. Test shadows (DB-B79)

`state/fix/shadow.sh` LINKS the real `bars/` tree (`ln -s`, writable); its
comment says "read-only", which is false. The protection is the store writer's
refusal of a linked path component (`store::file`, `refuse_links_below`).
Start every test server with `BRUTEX_READ_ONLY=1` (in `serve.sh`). With it:

* no request that is not a GET or a HEAD reaches a handler (403, by name);
* the autopilot never flies;
* a spot recovery left in flight is **not resumed at start**. The shadow
  clones `audit/`, and `audit/recovery-v1/` is where an in-flight plan lives:
  until 12 Oct the start resumed it whatever this variable said — vendor
  requests and writes through the linked bars, started by no request. Now
  the banner says `recovery not resumed: read-only (BRUTEX_READ_ONLY=1); …`
  with the plan's identity when one is in flight (`no plan is in flight`
  otherwise), the same sentence is one `api.recovery` event, and the plan's
  journals are read through a shared lock and not opened for append. A
  server that may write resumes the plan at its own start.

A build with no commit stamp is held to the same three rules, for its own
reason (`NO WRITES:` and `recovery not resumed: unstamped …`).

What a read-only server still writes, all under its own root and none of it a
bar, a sidecar, a census or a vendor journal: `serve.lock`, `telemetry/` (its
log), `audit/invocations-v1` (audited reads) and `audit/pull.journal.idx`
(the pull journal's read index, rebuilt beside the old one when stale). On a
shadow those are the shadow's clones. One root is still served by one api at
a time, read-only or not (§3 step 4).

## 8. A month filed under another name (DB-B65)

A bar header's own identity is the low 32 bits of a hash of the SYMBOL text
and the bar length: the NIFTY index, every NIFTY option contract and both
vendors share one id, so a month file copied by hand to a sibling strike
opened cleanly and was served as the other instrument. Every bar door
(`BarFile::open_or_create`, `open_existing` and a repair revision's open
through `open_existing_at`, `index_existing`, `open_existing_audited`) now
first asks the month's `.prv` sidecar (D-3334), whose sealed header
binds itself to the whole store path — vendor, exchange, segment, symbol,
contract, bar length, month. A `.prv` that names another month refuses the
open as `StoreError::Misfiled`, naming both files and the first field that
differs; `cli census-recount` and `cli scrub-blocks` open through the same
door, so a recount refuses such a month by name and a scrub pass counts it.

**Limit, stated.** The check needs a `.prv`. A month written before D-3334
has none: it is `unbound`, bound by its symbol hash alone, and is served as
before. `cli scrub-blocks` counts those months (`unbound=` on its cursor
line; `/disk.json` and the `/ingest` disk line show the count). They gain a
`.prv` when they are re-imported as a new generation (WP-80). One extra
`open` per month opened; a month with no sidecar costs the one `open` that
answers `ENOENT`.
