# CI integration audit — 19 September 2026

The workflow remains below GitHub's per-file size limit. The following two
introductory shell-comment blocks were moved here without moving commands,
changing patterns, or changing gate dependencies.

```text
          # WHY THIS EXISTS. crates/pull is the one crate whose safety argument
          # is "every path segment written down here is invented", and until
          # this gate that sentence was a comment. Gate 1c cannot check it: it
          # only matches a slash-JOINED path with a well-known environment
          # name, so a bare `const ORG: &str = "<the real org>";` passes it —
          # demonstrated, not assumed. See docs/06-limits.md §18.
          #
          # WHAT IT CHECKS. Every double-quoted literal under crates/pull that
          # could BE a parameter path segment — lower-case, [a-z0-9_-], at most
          # MAX_SEGMENT_LEN bytes — must appear in the list below. Comments and
          # doc examples are scanned too: a real segment pasted into a doc
          # example publishes it exactly as effectively as a constant does.
          #
          # THE ONE SHAPE THAT IS NOT SCANNED, and why it is a tightening and
          # not a hole: a literal that is EXACTLY `YYYY-MM-DD` is skipped. An
          # ISO-8601 calendar date names no account, no environment, no vendor
          # and no field — it carries zero bytes of the information this gate
          # protects — and the crate's date arithmetic generates them without
          # limit, one per boundary a test pins. Nine were on the list at the
          # commit this exclusion was added: leap days, month rolls, a century
          # non-leap year. Left in, they would have grown until the allowlist
          # was a wall of dates and nobody read the words next to them, which
          # is the failure mode of an allowlist. The exclusion is anchored at
          # both ends and requires exactly four-two-two digits, so `2024-02`,
          # `prod-2024-01-01` and any segment with a letter in it are still
          # scanned. All-digit literals are still scanned — see group 10.
          #
          # WHAT IT CANNOT CHECK. That the words below are not themselves an
          # operator's real segments. `groww` and `dhan` are on the list because
          # crates/core has tracked them since its first commit, and an operator
          # whose parameter path uses the broker's public name as its vendor
          # segment has published that segment by their own choice. That is
          # operator-side and docs/06-limits.md §18 says so.
          #
          # A NEW ENTRY IS A DELIBERATE ACT. Adding a word here is the moment to
          # ask whether it is invented. That is the whole mechanism, and it is
          # why the list is split into NAMED GROUPS with a comment over each
          # rather than one undifferentiated block: a reviewer has to read which
          # group a new word joined, and a word that fits no group is the
          # question this gate exists to ask. The comments live BETWEEN the
          # assignments and never inside them — `printf '%s\n' $allowed`
          # word-splits, so a `#` inside a string would silently become an
          # allowed word, and so would every word after it on that line.
          #
          # ---- group 1: the shape's own vocabulary, and the one real region.
          # `/<org>/<env>/<vendor>/<field>` written out, plus `ap-south-1`,
          # which CLAUDE.md §8 states in the clear.
          # WHY THIS EXISTS. Gate 8 measures a ratio at 1x/10x/100x, which
          # catches a bound that has ALREADY broken on a path someone thought
          # to write a bench for. The audit that produced this gate found four
          # O(1) defects that no gate could see, because each was a *source
          # construct* that is superlinear by nature and none of them had a
          # bench: a map reserved to exactly its load so the next insert
          # rehashed (2.4 ms at 50k entries), a page render that sorted the
          # whole universe per request, an encoder nothing called, and a
          # coverage gate that claimed 100% while exiting 1. Gate 8 says "this
          # path got slower". Gate 11 says "this construct may not appear".
          #
          # WHAT IT SCANS. Tracked `crates/*/src/**.rs` only, and inside each
          # file everything EXCEPT the `#[cfg(test)] mod NAME { ... }` blocks.
          # Test code and benches are exempt on purpose: a test asserting that
          # a refusal happens legitimately unwraps, sorts and panics.
          #
          # IT USED TO BE "THE REGION BEFORE THE FIRST `#[cfg(test)]`", AND
          # THAT BOUNDARY WAS THE BUG IT WARNED ABOUT. Truncating at an
          # attribute hides every line after it, and an attribute is not only
          # ever a test module: `costs/src/trip.rs` carries `#[cfg(test)]` on a
          # const fn 876 lines above its tests, `costs/src/dated.rs` on a const
          # at line 55, `greeks/src/bsm.rs` on a STATEMENT inside the shipping
          # body of `Checked::greeks`. One attribute shrank the scan exactly as
          # silently as two, and the two-attribute check never fired on the
          # single-attribute case — 790 lines of production code were invisible
          # while the gate reported success, `render::json_string` and
          # `server::universe_label` among them, both of which sit AFTER a test
          # module where no truncation rule of any shape can reach them.
          #
          # So the boundary is a SUBTRACTION now, not a cut: from a
          # `#[cfg(test)]`, skip blank lines, `//` comments and attributes to
          # find the item it covers; if that item is an inline `mod NAME {`,
          # drop through to the matching `}` at the same indent; otherwise keep
          # the attribute and the item and carry on scanning. A file may hold
          # as many test modules as it likes, in any position, and none of them
          # hides a line. D-0065.
          #
          # NO UNTRUSTED INPUT REACHES THIS STEP. The inputs are `git ls-files`
          # and the static patterns below, the same discipline gate 1c states.
          # Nothing from the event payload is expanded. `git ls-files` also
          # means an UNTRACKED file is invisible — which is right for CI, where
          # the checkout holds only tracked files, but means a local run can
          # disagree with CI until the file is added.
          #
          # WHAT IT CANNOT SEE, stated plainly because a gate that oversells
          # itself is worse than no gate:
          #   * Indirection. `.sort()` behind a trait, a `HashMap` arrived at
          #     by `.collect()`, a float behind a type alias or a generic
          #     `T: Float` — all invisible. This is a TEXT scan, not a type
          #     check. It refuses the spelling, not the behaviour.
          #   * Macro-generated code. Expanded after this gate has run.
          #   * `/* ... */` block comments are NOT stripped; only whole lines
          #     whose first non-blank characters are `//` (which covers `///`
          #     and `//!`). A banned token inside a block comment or inside a
          #     string literal is a false positive, and the allowlist is the
          #     escape hatch for it.
          #   * "Per-request" is not decidable from text. Rule 4 refuses every
          #     sort in non-test source and leans on the allowlist to record,
          #     one file at a time, WHY a given sort is bounded. The reasons
          #     are written beside the entries and are the reviewable part.
          #   * A test module whose closing `}` is not at the module keyword's
          #     own indent — which `cargo fmt` cannot produce, and gate 6a
          #     enforces `cargo fmt` — is not delimitable by this scanner, and
          #     that is a HARD FAILURE rather than a guess. "I could not tell
          #     where this module ends" is a thing a gate may say; quietly
          #     scanning half of it is not.
          #   * `#[cfg(test)] mod tests {` written on ONE line is not
          #     recognised as a module and its body IS scanned. That direction
          #     is loud — the test bodies refuse under rules 4 and 5 — never
          #     silent, which is why it is left rather than pattern-matched at.
          #
          # THE ESCAPE HATCH IS HERE, NEVER IN THE CODE. No magic comment, no
          # `// gate-11-ok` scattered through crates/. Each entry is
          # `path max_occurrences`, and the count is pinned at what the file
          # measures TODAY — so a legitimate existing use stays green while the
          # NEXT one is a red build and a visible diff on this file. That is
          # gate 1d's mechanism: adding a line here is the moment to ask
          # whether the construct is really justified.
          # ---- rule 1. No allowlist. docs/07 layer 4 is unconditional. ------
          # AND IT STAYS EMPTY. The one candidate this rule ever had was
          # `core::vendor::board_of`, which binary-searched three const tables
          # of 6, 2 and 120 entries, once per equity master row. The audit that
          # found it could have written the first entry here with a defensible
          # reason — the tables are compile-time fixed and nothing per-bar
          # reaches them — and did not, because `core::universe::MemberIndex`
          # already exists for exactly this, was built to retire a
          # `binary_search` over 750 entries, and is smaller machinery than the
          # paragraph justifying an exemption would have been. D-0065.
          # `partition_point` is the same O(log n) shape as `binary_search` and was
          # outside rule 1 entirely -- an audit found it live at
          # crates/store/src/file.rs:956 while this rule reported ZERO occurrences.
          #
          # That call is legitimate and the reason is checkable: it partitions the
          # INCOMING BATCH at the last held timestamp, so the log is over a
          # caller-supplied slice whose length is the append size, not over the
          # stored history. `survey` has already proven the batch strictly
          # increasing, which is what makes a partition correct there at all.

```

## Construct review and limits

Gate 11 still scans production source, rejects undeclared occurrences, and pins
counts per file. Test files are excluded only when Rust itself excludes their
entire contents with `#![cfg(test)]`, or their crate-root module has the test
attribute. An ordinary filename or comment cannot suppress the scanner.

The additional reviewed sites fall into these categories:

| Rule | Actual operation | Reason and limit |
|---|---|---|
| Search | Historical cash-day membership | Replaced binary search with a pre-sized hash set; no new search allowance. Building the set remains linear in historical days. |
| Floats | Statistical bit decoding, probability ratios, outcome aggregates | Statistical quantities retain floating-point precision. Stored prices and execution prices remain integer paisa. |
| Map creation | Ledger/header validation, replay admission, uniqueness sets | Empty construction is followed by fallible reservation before insertion, or reservation in the receiving cache loader. This does not make opening a ledger constant-time. |
| Map creation | Optional replay caches | Empty/reset caches are populated as work is admitted. Their lifetime cost depends on admitted records; there is no whole-cache constant-time guarantee. |
| Sort/selection | Canonical persisted identity, deterministic reports, replay order, quantiles | Work is proportional to the input admitted at setup/report time. Index-stop ranking sorts an authenticated, budget-admitted cache generation before serving pages. Sorting is not a per-bar primitive or a constant-time report guarantee. |
| Linear search | Fixed rung/vendor arrays, bounded lifecycle records, selected exit cells, CSV field names | Each operation searches its explicitly admitted input. Fixed arrays are bounded independently of history; variable grids/headers are setup work and not an O(1) claim. |
| Membership | Integer ranges, fixed arrays, hash sets, configuration strings | Range checks and hash membership are not slice scans. Fixed arrays have compile-time size. Variable configuration/validation inputs retain input-dependent cost. |

The source locations below identify the reviewed occurrences before the repairs
and formatting. Line numbers are review evidence, not a machine interface.
Removed search/map occurrences remain here as evidence of the defects corrected.

```text
1 crates/api/src/server.rs:6608     days.retain(|day| historical.binary_search(day).is_err());
2 crates/api/src/booleanevidencejson.rs:423     let value = f64::from_bits(bits);
2 crates/cli/src/boolean_qualification_reader.rs:525             Some(bits) if f64::from_bits(*bits) > 0.0 => 1,
2 crates/cli/src/boolean_qualification_wire.rs:332         let statistic = f64::from_bits(r[6]);
2 crates/cli/src/boolean_qualification_wire.rs:411         if !f64::from_bits(test[0]).is_finite()
2 crates/cli/src/boolean_qualification_wire.rs:452             f64::from_bits(last)
2 crates/cli/src/boolean_qualification_wire.rs:453                 .total_cmp(&f64::from_bits(stat))
2 crates/cli/src/boolean_qualification_wire.rs:474     Ok((n as f64 / d as f64).to_bits())
2 crates/cli/src/boolean_statistics_reader.rs:607         if !f64::from_bits(test[0]).is_finite()
2 crates/cli/src/boolean_statistics_reader.rs:636                 || !f64::from_bits(values[2]).is_finite()
2 crates/cli/src/boolean_statistics_reader.rs:677     Ok((numerator as f64 / denominator as f64).to_bits())
2 crates/runner/src/outcome.rs:860     pub mean_paisa: f64,
2 crates/runner/src/outcome.rs:869     pub win_sum: f64,
2 crates/runner/src/outcome.rs:881     pub adverse_sum: f64,
2 crates/runner/src/outcome.rs:887     pub favourable_sum: f64,
2 crates/runner/src/outcome.rs:903     pub loss_sum: f64,
2 crates/runner/src/outcome.rs:920     pub min_win_paisa: f64,
2 crates/runner/src/outcome.rs:927     pub max_win_paisa: f64,
2 crates/runner/src/outcome.rs:932     pub max_loss_paisa: f64,
3 crates/api/src/recovery.rs:1021         std::collections::HashMap::new();
3 crates/api/src/server.rs:2848     let mut dated = std::collections::HashMap::new();
3 crates/api/src/server.rs:4791             calendars: std::sync::Mutex::new(std::collections::HashMap::new()),
3 crates/api/src/server.rs:6652     let mut days = std::collections::HashSet::new();
3 crates/api/src/server.rs:6871     let mut dated = std::collections::HashMap::new();
3 crates/api/src/server.rs:29744         std::collections::HashMap::new();
3 crates/api/src/store_wire.rs:62                 bodies: HashMap::new(),
3 crates/cli/src/all_rung_selection_v6.rs:73     let mut identities = std::collections::HashSet::new();
3 crates/cli/src/boolean_candidate_reader.rs:254     let mut unique = std::collections::HashSet::new();
3 crates/cli/src/boolean_candidate_v1.rs:380     let mut identities = std::collections::HashSet::new();
3 crates/cli/src/boolean_catalog_command.rs:130     let mut identities = std::collections::HashSet::new();
3 crates/cli/src/global_replay_v4.rs:326         months: HashMap::new(),
3 crates/cli/src/index_stop.rs:640     let mut seen = std::collections::HashSet::new();
3 crates/cli/src/index_stop_store.rs:334     let mut runs = std::collections::HashSet::new();
3 crates/cli/src/pool.rs:433     let mut seen: HashSet<Candidate> = HashSet::new();
3 crates/cli/src/population_v6.rs:1437     let mut set = HashSet::new();
3 crates/cli/src/population_v6.rs:1955                 receipts: HashMap::new(),
3 crates/cli/src/population_v6.rs:1983         self.receipts = HashMap::new();
3 crates/cli/src/population_v6.rs:3178         let mut seen = std::collections::HashSet::new();
3 crates/cli/src/selection_v6.rs:265     let mut identities = HashSet::new();
3 crates/cli/src/selection_v6_source.rs:96     let mut by_strategy = HashMap::new();
3 crates/cli/src/selection_v6_source.rs:104     let mut dispositions = HashSet::new();
3 crates/engine/src/lib.rs:1405         let mut offered: HashSet<u32> = HashSet::new();
3 crates/engine/src/resume.rs:198         let mut offered = std::collections::HashSet::new();
3 crates/engine/src/resume.rs:273         let mut seen = std::collections::HashSet::new();
3 crates/pull/src/cash_auction.rs:393         let mut by_symbol = HashMap::new();
3 crates/pull/src/cash_auction.rs:394         let mut by_native_id = HashMap::new();
3 crates/pull/src/cash_auction.rs:510         let mut seen = std::collections::HashSet::new();
3 crates/pull/src/cash_session_cache.rs:262     let mut result = HashMap::new();
3 crates/pull/src/cash_session_cache.rs:302     let mut pending = HashMap::new();
4 crates/api/src/constituents.rs:745     lacks.sort_unstable();
4 crates/api/src/constituents.rs:789             names.sort_unstable();
4 crates/api/src/constituents.rs:829             ids.sort_unstable_by(|a, b| a.as_str().cmp(b.as_str()));
4 crates/api/src/indexstopqualificationjson.rs:250     ranked.sort_unstable_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
4 crates/api/src/indexstoprankingjson.rs:222     rows.sort_unstable_by(|a, b| {
4 crates/api/src/merge.rs:291         out.sort_unstable();
4 crates/api/src/merge.rs:511     out.conflicts.sort_unstable();
4 crates/api/src/merge.rs:562     out.eligibility.sort_unstable();
4 crates/api/src/merge.rs:613     assertions.sort_unstable_by(|(av, ak, a), (bv, bk, b)| {
4 crates/api/src/recovery.rs:194     out.sort_by_key(|item| {
4 crates/api/src/recovery.rs:305     scans.sort_by_key(|row| {
4 crates/api/src/server.rs:1120     listing.sort_unstable_by_key(|(key, _)| {
4 crates/api/src/server.rs:2671         keys.sort_unstable();
4 crates/api/src/server.rs:5564     months.sort_unstable();
4 crates/api/src/server.rs:5790             named.sort_unstable();
4 crates/api/src/server.rs:6540     issues.sort_unstable();
4 crates/api/src/server.rs:6611     days.sort_unstable();
4 crates/api/src/server.rs:6684     numbers.sort_unstable();
4 crates/api/src/server.rs:6889     targets.sort_unstable();
4 crates/cli/src/boolean_observation_file.rs:121         ordered.sort_unstable_by(|a, b| a.directory.cmp(&b.directory));
4 crates/cli/src/boolean_qualified_journal.rs:105         canonical_units.sort_unstable();
4 crates/cli/src/boolean_qualified_observer.rs:46         sorted.sort_unstable();
4 crates/cli/src/boolean_statistics_v1.rs:237         sources.sort_unstable_by_key(|source| scope.position(&source.family().instrument()));
4 crates/cli/src/global_replay_v4.rs:482     order.sort_unstable_by_key(|attempt| {
4 crates/cli/src/index_stop_vix.rs:479     queries.sort_unstable_by_key(|query| (query.month.year(), query.month.month(), query.row));
4 crates/cli/src/index_stop_vix_codec.rs:294     minutes.sort_unstable_by_key(|minute| (minute.month, minute.timestamp));
4 crates/cli/src/lib.rs:4386     let (_, &mut nth, _) = travels.select_nth_unstable(at);
4 crates/cli/src/lib.rs:4415     let (_, &mut nth, _) = ranges.select_nth_unstable(at);
4 crates/cli/src/lib.rs:4520     let (_, &mut median, _) = ranges.select_nth_unstable(middle);
4 crates/cli/src/lib.rs:9126     out.sort_by_key(|t| {
4 crates/cli/src/lib.rs:11429     rows.sort_by_key(|r| money_key(&r.cell));
4 crates/cli/src/lib.rs:11485     rows.sort_by_key(|r| {
4 crates/cli/src/lib.rs:15871     ordered.sort_by_key(|scored| {
4 crates/cli/src/live.rs:662     runs.sort_by_key(|run| run.identity);
4 crates/cli/src/live.rs:971         census.runs.sort_by_key(|run| run.identity);
4 crates/cli/src/pool.rs:390     rows.sort_by_key(|s| match &s.outcome {
4 crates/cli/src/pool.rs:618     pooled.sort_by_key(|p| core::cmp::Reverse(p.key(rule_bp)));
4 crates/pull/src/cash_session_cache.rs:300     days.sort_unstable();
4 crates/pull/src/ingest.rs:2014     months.sort_unstable();
4 crates/runner/src/grid.rs:1651     let (_, &mut target, _) = ran.select_nth_unstable(at);
4 crates/runner/src/grid.rs:1699     all.sort_unstable();
4 crates/runner/src/grid.rs:1768     all.sort_unstable();
4 crates/runner/src/research_family.rs:267         families.sort_unstable_by_key(|family| (family.tag, family.instrument.underlying));
6 crates/api/src/store_wire.rs:135     if let Some(census) = censuses.iter().find(|c| c.vendor == feed) {
6 crates/api/src/sweeprun.rs:2185     let Some(marker) = lifecycle.records.iter().find(|record| {
6 crates/cli/src/lib.rs:1486     let Some(known) = EVERY_RUNG.iter().find(|r| **r == rung) else {
6 crates/cli/src/lib.rs:1631     let Some(known) = EVERY_RUNG.iter().find(|r| **r == rung) else {
6 crates/cli/src/lib.rs:7815     let Some(cell) = exits.cells.iter().find(|cell| **cell == selected.cell) else {
6 crates/cli/src/lib.rs:9949         .or_else(|| rows.iter().find(|row| row.cell.trades > 0))
6 crates/cli/src/lib.rs:13222     let Some(known) = EVERY_RUNG.iter().find(|r| **r == rung) else {
6 crates/cli/src/lib.rs:13897     let Some(known) = EVERY_RUNG.iter().find(|r| **r == rung) else {
6 crates/cli/src/lib.rs:14268     if let Some(bad) = rungs.iter().find(|r| !EVERY_RUNG.contains(r)) {
6 crates/pull/src/cash_auction.rs:527                 .map(|wanted| header.iter().position(|name| name == wanted)),
7 crates/api/src/booleanevidencejson.rs:99         if !(1..=256).contains(&limit)
7 crates/api/src/booleanjson.rs:88         if !(1..=256).contains(&limit)
7 crates/api/src/booleanlaunch.rs:126             .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
7 crates/api/src/booleanoosjson.rs:62         if !(1..=256).contains(&limit)
7 crates/api/src/booleansearchjson.rs:71         if !(1..=256).contains(&limit) {
7 crates/api/src/expressionsearchjson.rs:47         if !(1..=256).contains(&limit) {
7 crates/api/src/index_consistency_projection.rs:150     if asked.offset > total || !(1..=256).contains(&asked.limit) {
7 crates/api/src/indexstopjson.rs:57             || !(1..=256).contains(&limit)
7 crates/api/src/indexstoplaunch.rs:397             .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
7 crates/api/src/indexstopqualificationjson.rs:72             || !(1..=256).contains(&limit)
7 crates/api/src/indexstopvixjson.rs:48         if !(1..=PAGE_ROWS).contains(&limit) || offset.checked_add(limit).is_none() {
7 crates/api/src/merge.rs:507             raw_ambiguous.contains(&(vendor, l.key)),
7 crates/api/src/merge.rs:695         (Some(candidate), Some(isin)) if asserted.contains(&(candidate, isin)) => candidate,
7 crates/api/src/pullrun.rs:813     let retry_cycle = outcomes.contains(&PassOutcome::Retry);
7 crates/api/src/pullrun.rs:939         let repairing = outcomes.contains(&PassOutcome::Retry);
7 crates/api/src/pullrun.rs:959         if outcomes.contains(&PassOutcome::Retry) {
7 crates/api/src/recovery.rs:46         if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
7 crates/api/src/recovery.rs:1220                 !held_days.contains(&day)
7 crates/api/src/recovery.rs:1296             if daily_days.contains(&day) || minute_days.contains(&day) {
7 crates/api/src/recovery.rs:1303                 if daily_days.contains(&day) || minute_days.contains(&day) =>
7 crates/api/src/recovery.rs:1442             if daily.contains(&day) || minutes.contains(&day) {
7 crates/api/src/recovery.rs:1449                 if daily.contains(&day) {
7 crates/api/src/recovery.rs:1457                 if daily.contains(&day) {
7 crates/api/src/recovery.rs:1459                 } else if minutes.contains(&day) {
7 crates/cli/src/audited_range.rs:93         if limits.contains(&0) {
7 crates/cli/src/boolean_campaign.rs:287     if !(1..=8).contains(&request.max_rung_jobs) || request.programs.is_empty() {
7 crates/cli/src/boolean_campaign.rs:310     if !(1..=8).contains(&request.max_rung_jobs) || request.programs.is_empty() {
7 crates/cli/src/boolean_candidate_v1.rs:371     .contains(&0)
7 crates/cli/src/boolean_catalog_prepared.rs:186         if !crate::ledger_all::LEDGER_RUNGS.contains(&rung) {
7 crates/cli/src/boolean_qualification_plan.rs:493             || self.minimums.contains(&0)
7 crates/cli/src/boolean_qualification_plan.rs:495             || self.limits.contains(&0)
7 crates/cli/src/boolean_qualification_wire.rs:314     if !(1..=3).contains(&class)
7 crates/cli/src/boolean_qualified_journal.rs:107             || units.contains(&[0; 32])
7 crates/cli/src/boolean_qualified_observer.rs:48             || sorted.contains(&[0; 32])
7 crates/cli/src/boolean_search_record.rs:123         if self.sources.contains(&[0; 32])
7 crates/cli/src/boolean_statistics_reader.rs:579     if (1..=3).contains(&value) {
7 crates/cli/src/boolean_statistics_v1.rs:221         if bounds.words().contains(&0)
7 crates/cli/src/boolean_statistics_v1.rs:343         || bounds.words().contains(&0)
7 crates/cli/src/candidate_trades.rs:396                 if !evaluated.grid.cells.contains(&cell) {
7 crates/cli/src/expression_search_reader.rs:150         if cursor.is_some_and(|anchor| !self.admitted.contains(&anchor)) {
7 crates/cli/src/expression_search_reader.rs:207             if !self.admitted.contains(&next) && self.admitted.len() >= MAX_CONTINUATIONS {
7 crates/cli/src/index_consistency.rs:1200     let excluded = indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS.contains(&day);
7 crates/cli/src/index_consistency.rs:1553     let excluded = u64::from(indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS.contains(&day));
7 crates/cli/src/index_stop.rs:623     if [limits.programs, limits.records, limits.bytes].contains(&0)
7 crates/cli/src/index_stop.rs:651         || !crate::ledger_all::LEDGER_RUNGS.contains(&rung)
7 crates/cli/src/index_stop_qualification.rs:83         if [self.bootstrap_work, self.split_work, self.memory_bytes].contains(&0) {
7 crates/cli/src/index_stop_qualification_codec.rs:199         .contains(&[0; 32])
7 crates/cli/src/index_stop_qualification_codec.rs:200         || bounds.words().contains(&0)
7 crates/cli/src/index_stop_qualification_numeric.rs:25         || facts.bounds.words().contains(&0)
7 crates/cli/src/index_stop_search_reader.rs:401             .contains(&0)
7 crates/cli/src/index_stop_source_context.rs:138         .contains(&0)
7 crates/cli/src/index_stop_source_context_codec.rs:278         || [source_id, source_binding, calendar].contains(&[0; 32])
7 crates/cli/src/index_stop_source_context_codec.rs:481                 || !(exit.low..=exit.high).contains(&trade.optimistic_exit_paisa)
7 crates/cli/src/index_stop_source_context_codec.rs:482                 || !(exit.low..=exit.high).contains(&trade.pessimistic_exit_paisa)
7 crates/cli/src/index_stop_store.rs:450     if [record.run_id, record.source_id, record.evaluation_digest].contains(&[0; 32])
7 crates/cli/src/lib.rs:802                 if dropped.contains(&day) {
7 crates/cli/src/lib.rs:912                 if dropped.contains(&day) {
7 crates/cli/src/lib.rs:2133     if COMMANDS.contains(&word) {
7 crates/cli/src/lib.rs:2253         Ok(n) if (1..=3650).contains(&n) => Ok(n),
7 crates/cli/src/lib.rs:7697     if EVERY_RUNG.contains(&rung) {
7 crates/cli/src/minute_gaps.rs:225         if withheld.contains(&indicators::ist_day(bar.ts_micros)) {
7 crates/cli/src/operation_audit.rs:173                 .all(|b| b.is_ascii_alphanumeric() || b" /._-".contains(&b))
7 crates/cli/src/operation_audit.rs:174             || (self.response_status != 0 && !(100..=599).contains(&self.response_status))
7 crates/cli/src/selection_v6_source.rs:220     if ![60, 120, 180, 300, 600, 900, 1800, 3600].contains(&execution.rung())
7 crates/engine/src/resume.rs:284             if !correct || !seen.insert(excluded.position) || !offered.contains(&excluded.position)
7 crates/runner/src/expression_validation.rs:187         if sessions.contains(&0) {
5c crates/api/src/sweeprun.rs:3145 mod admission_tests;
5d crates/api/src/render.rs:3123             called = true;
5d crates/api/src/render.rs:3124             vec!["unexpected".to_owned()]
5d crates/api/src/render.rs:3129             called = true;
5d crates/api/src/render.rs:3130             vec!["fixture".to_owned()]
5d crates/cli/src/readonly_file.rs:74         std::os::unix::fs::symlink(&path, &alias)?;
5d crates/cli/src/readonly_file.rs:77         std::fs::remove_file(&path)?;
5d crates/cli/src/readonly_file.rs:78         assert!(
5d crates/cli/src/readonly_file.rs:80             "a dangling symlink cannot create a target"
5d crates/cli/src/readonly_file.rs:84     }
5d crates/cli/src/readonly_file.rs:94         }
5d crates/cli/src/readonly_file.rs:99                 .arg(&path)
5d crates/cli/src/readonly_file.rs:118                     "nonblocking FIFO child must refuse normally"
```

No latency, coverage, mutation, or universal O(1) guarantee follows from passing
this text scanner. The unmeasured performance documentation is explicitly marked
UNVERIFIED in the source and recorded in `docs/06-limits.md`.
