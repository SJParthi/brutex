From 7f7adb96767ee1c3b86578a6f5b2dce2259df848 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:29:23 +0000
Subject: [PATCH 1/8] cli: a recorded tier walk captures the screens it shows
 (G2-5, D-4716)

Before: walk_tiers handed the candidate capture to tier_rows for every
tier it judged, so a recorded walk that admitted nothing captured all
1 + T screens: (1 + T) x (2 + 8C) + 2 fsyncs against a 64 MiB
acknowledgement budget that refuses the whole run on long ladders.
Measured on the 8-session fixture: 2,521 captured tiers for a
2,520-tier ladder.

After: every tier is judged with no capture; only the tier the walk
ends on (the met tier, or the last) is judged again with it. A cascade
captures at most two screens, the operator's policy and that tier, and
its answer is byte-identical. The capture format and its version are
unchanged; docs/19 records which passes are captured. docs/06 states
the bound next to D-1734.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/candidate_trades.rs    |   8 +-
 crates/cli/src/lib.rs                 |  42 +++++-
 crates/cli/src/screen_policy_tests.rs | 209 ++++++++++++++++++++++++++
 docs/04-invariants.md                 |   7 +
 docs/05-decisions.md                  |  45 ++++++
 docs/06-limits.md                     |  19 +++
 docs/19-candidate-trades.md           |  17 +++
 7 files changed, 337 insertions(+), 10 deletions(-)

diff --git a/crates/cli/src/candidate_trades.rs b/crates/cli/src/candidate_trades.rs
index 3d824eef..5e96f24a 100644
--- a/crates/cli/src/candidate_trades.rs
+++ b/crates/cli/src/candidate_trades.rs
@@ -1,6 +1,7 @@
 //! Exact selected-cell trades for every evaluated screen candidate and side.
 //!
-//! Each visited policy tier retains its actual cap and inputs. Immutable child
+//! Each captured screen pass retains its actual cap and inputs. A recorded
+//! tier walk captures only the tier it ends on (D-4716). Immutable child
 //! files are sealed before a catalog can publish the captured set. The catalog
 //! is prepared evidence, not an institutional admission or parent-run success.
 //!
@@ -241,7 +242,7 @@ thread_local! {
 #[cfg(test)]
 thread_local! {
     /// Test-only count of `fsync` calls issued by `write_exact` on this thread.
-    static DURABLE_SYNCS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
+    pub(crate) static DURABLE_SYNCS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
 }
 
 impl<'a> Capture<'a> {
@@ -690,7 +691,8 @@ pub struct Summary {
     pub attempt: u64,
     /// Digest of the actual execution slice.
     pub execution_digest: [u8; 32],
-    /// Number of visited screen invocations, not generated policy tiers.
+    /// Number of captured screen passes, not generated policy tiers: the
+    /// operator's own policy and the tier a recorded walk ended on (D-4716).
     pub tiers: u64,
     /// Captured candidate-side outcomes, including no-cell outcomes.
     pub candidates: u64,
diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index 4ae07321..3589af06 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -13131,6 +13131,11 @@ where
 /// The answer is the reference walk's, `walk_ladder` with `screen` per tier,
 /// byte for byte: proven by
 /// `cli::screen_policy_tests::the_cached_tier_walk_equals_the_full_walk_on_real_screens`.
+///
+/// A capture records only the tier the walk ENDS on: the met tier, or the last
+/// tier when none admits. Every other tier is judged with no capture, so a
+/// recorded walk writes one captured screen whatever `T` is (G2-5, D-4716):
+/// proven by `cli::screen_policy_tests::a_recorded_walk_captures_only_the_tier_it_ends_on`.
 #[expect(
     clippy::too_many_arguments,
     reason = "the six slice inputs every screen takes, the ladder, and the unmet sink"
@@ -13153,11 +13158,33 @@ fn walk_tiers<'t, 'a>(
             price_grids(bars, column, by_evidence, horizon, envelope, pricing, facts)
         },
         |grids, (_, rules)| {
-            let rows = tier_rows(grids, by_evidence, horizon, *rules, pricing)?;
-            Ok(rows
-                .iter()
-                .any(|row| row.admitted)
-                .then(|| finish_screen(rows, bars, column, horizon, *rules, facts)))
+            // JUDGED UNRECORDED; CAPTURED ONLY WHEN SHOWN (G2-5, D-4716). A
+            // tier that admits nothing is not the answer and its rows are never
+            // shown, so capturing it bought a tier file and four `fsync`s per
+            // candidate side for every one of up to 18,480 tiers, against a
+            // 64 MiB budget that then refused the whole recorded run.
+            let rows = tier_rows(
+                grids,
+                by_evidence,
+                horizon,
+                *rules,
+                Pricing {
+                    capture: None,
+                    ..pricing
+                },
+            )?;
+            if !rows.iter().any(|row| row.admitted) {
+                return Ok(None);
+            }
+            // The met tier is the answer: judged again WITH the capture. The
+            // capture only records, so these rows equal the ones above.
+            let rows = match pricing.capture {
+                Some(_) => tier_rows(grids, by_evidence, horizon, *rules, pricing)?,
+                None => rows,
+            };
+            Ok(Some(finish_screen(
+                rows, bars, column, horizon, *rules, facts,
+            )))
         },
         |grids, (_, rules)| {
             let rows = tier_rows(grids, by_evidence, horizon, *rules, pricing)?;
@@ -13792,8 +13819,9 @@ fn tier_rows<'a>(
     rules: Rules,
     pricing: Pricing<'_>,
 ) -> Result<Vec<Screened<'a>>, String> {
-    // ONE CAPTURE TIER PER POLICY JUDGED, as when every policy priced its own
-    // grids: the capture's tier ordinals follow the walk, not the grid passes.
+    // ONE CAPTURE TIER PER CALL THAT CARRIES A CAPTURE. `walk_tiers` passes
+    // one only for the tier it ends on, so the capture's tier ordinals are the
+    // SHOWN screens, not every tier judged (G2-5, D-4716).
     let captured_tier = pricing
         .capture
         .map(|capture| {
diff --git a/crates/cli/src/screen_policy_tests.rs b/crates/cli/src/screen_policy_tests.rs
index 276bd640..f899908b 100644
--- a/crates/cli/src/screen_policy_tests.rs
+++ b/crates/cli/src/screen_policy_tests.rs
@@ -1595,3 +1595,212 @@ fn the_envelope_is_each_floors_minimum_over_its_own_key() {
         (i64::MAX, i64::MAX)
     );
 }
+
+// ---------------------------------------------------------------------------
+// G2-5 (D-4716): a recorded tier walk captures the screens its page shows, not
+// every tier it judged.
+// ---------------------------------------------------------------------------
+
+/// A fresh capture root under the temporary directory, removed on drop.
+struct CaptureRoot(std::path::PathBuf);
+
+impl CaptureRoot {
+    fn new(tag: &str) -> Self {
+        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
+        let path = std::env::temp_dir().join(format!(
+            "brutex-l1fb-capture-{tag}-{}-{}",
+            std::process::id(),
+            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
+        ));
+        std::fs::create_dir_all(&path).expect("a private capture root");
+        Self(path)
+    }
+
+    fn attempt(&self, tag: u8) -> crate::sweep_evidence::Attempt {
+        crate::sweep_evidence::begin(&self.0, [tag; 32], crate::sweep_evidence::Operation::Audit)
+            .expect("a durable audit attempt")
+    }
+}
+
+impl Drop for CaptureRoot {
+    fn drop(&mut self) {
+        let _ = std::fs::remove_dir_all(&self.0);
+    }
+}
+
+/// Runs `work` on a one-thread pool and returns its answer with every capture
+/// `fsync` it issued: on one worker thread the thread-local counter sees the
+/// parallel recording too.
+fn counting_syncs<T: Send>(work: impl FnOnce() -> T + Send) -> (T, u64) {
+    let pool = rayon::ThreadPoolBuilder::new()
+        .num_threads(1)
+        .build()
+        .expect("a one-thread pool");
+    pool.install(|| {
+        candidate_trades::DURABLE_SYNCS.with(|count| count.set(0));
+        let answer = work();
+        (
+            answer,
+            candidate_trades::DURABLE_SYNCS.with(std::cell::Cell::get),
+        )
+    })
+}
+
+/// G2-5, D-4716. A RECORDED cascade whose stated rules admit nothing walks
+/// every generated tier (D-1731), and the candidate capture recorded every
+/// tier it judged: one fsynced tier file and, per candidate side, a trade
+/// replay and two fsynced files. On this fixture's 2,520-tier ladder that was
+/// 2,521 captured screens and `2 × 2,521 + 4 × 2 × 2 × 2,521` fsyncs; on an
+/// operator rung the 64 MiB acknowledgement budget refused the whole run
+/// part-way down the ladder.
+///
+/// The capture now records the two screens the page is made of: the
+/// operator's own, and the mildest tier's, whose table the page prints. Its
+/// cost is two screens whatever the ladder's length, and the answer is the
+/// unrecorded cascade's, byte for byte.
+#[test]
+fn a_recorded_walk_that_admits_nothing_captures_two_screens_not_the_ladder() {
+    let fixture = Ranked::of(8);
+    let by_evidence = fixture.by_evidence(2);
+    let bars = &fixture.bars;
+    let column = &fixture.run.column;
+    let facts = runner::trade::SliceFacts::of(bars, column);
+    let mut rules = Rules::elite(400, 25);
+    rules.min_trades = u64::MAX;
+    let ladder = tiers(bars, by_evidence[0].hits);
+    let mildest = ladder
+        .last()
+        .expect("a generated ladder is never empty here")
+        .rules(rules.top, reference_price(bars));
+    assert!(ladder.len() > 2, "fixture: a ladder longer than two tiers");
+
+    let root = CaptureRoot::new("nothing-admits");
+    let attempt = root.attempt(0x45);
+    let capture =
+        candidate_trades::Capture::begin(&root.0, &attempt, bars, column).expect("capture starts");
+    let ((recorded, summary), syncs) = counting_syncs(|| {
+        let recorded = screen_cascade(
+            bars,
+            column,
+            &by_evidence,
+            Horizon::DEFAULT,
+            rules,
+            Pricing {
+                recording: None,
+                capture: Some(&capture),
+            },
+            true,
+            &facts,
+        )
+        .expect("the recorded cascade runs");
+        (recorded, capture.finish().expect("the capture seals"))
+    });
+    let unrecorded = screen_cascade(
+        bars,
+        column,
+        &by_evidence,
+        Horizon::DEFAULT,
+        rules,
+        NO_PRICING,
+        true,
+        &facts,
+    )
+    .expect("the unrecorded cascade runs");
+
+    // THE ANSWER IS THE UNRECORDED ONE: recording changes no byte of it.
+    assert!(
+        recorded.text.contains("NO TIER MET, INCLUDING THE MILDEST"),
+        "fixture: nothing admits\n{}",
+        recorded.text
+    );
+    assert_eq!(recorded.text, unrecorded.text);
+    assert!(!recorded.admitted_any && !unrecorded.admitted_any);
+    assert_eq!(
+        recorded.selected.map(|chosen| chosen.cell),
+        unrecorded.selected.map(|chosen| chosen.cell)
+    );
+
+    // THE CAPTURE IS THE PAGE'S TWO SCREENS, NOT THE LADDER.
+    assert_eq!(
+        summary.tiers,
+        2,
+        "captured screens on a {}-tier ladder",
+        ladder.len()
+    );
+    let yours = candidate_trades::tier(&root.0, &summary, 0, candidate_trades::DEFAULT_MAX_BYTES)
+        .expect("the operator's screen");
+    let shown = candidate_trades::tier(&root.0, &summary, 1, candidate_trades::DEFAULT_MAX_BYTES)
+        .expect("the shown tier's screen");
+    assert_eq!(yours.rules, rules, "tier 0 is the operator's own policy");
+    assert_eq!(
+        shown.rules, mildest,
+        "tier 1 is the mildest tier, whose table the page prints"
+    );
+    assert_eq!(
+        summary.candidates,
+        2 * (yours.evaluated + shown.evaluated),
+        "every evaluated candidate side of both screens"
+    );
+    // TWO FSYNCS PER FILE: a tier file per screen, two files per candidate
+    // side, and the catalog.
+    assert_eq!(
+        syncs,
+        2 * summary.tiers + 4 * summary.candidates + 2,
+        "the capture's whole durable cost"
+    );
+}
+
+/// G2-5, D-4716. A recorded walk that MEETS a later tier captures that tier
+/// alone: the strictest unmet tiers before it are judged and named UNMET, and
+/// none of them is recorded. Before, every tier the walk judged was captured.
+#[test]
+fn a_recorded_walk_captures_only_the_tier_it_ends_on() {
+    let slice = Slice::of(8);
+    let none_at_39 = crafted(39, 0, 0);
+    let ladder = vec![
+        none_at_39,
+        crafted(2_000, 5_000, 0),
+        crafted(500, 0, 0),
+        crafted(2_000, 0, 0),
+    ];
+    let by_evidence = slice.fixture.by_evidence(2);
+    let root = CaptureRoot::new("met-later");
+    let attempt = root.attempt(0x46);
+    let capture = candidate_trades::Capture::begin(
+        &root.0,
+        &attempt,
+        &slice.fixture.bars,
+        &slice.fixture.run.column,
+    )
+    .expect("capture starts");
+    let mut unmet = Vec::new();
+    let recorded = walk_tiers(
+        &slice.fixture.bars,
+        &slice.fixture.run.column,
+        &by_evidence,
+        Horizon::DEFAULT,
+        Pricing {
+            recording: None,
+            capture: Some(&capture),
+        },
+        &slice.facts,
+        &ladder,
+        |rank| unmet.push(rank),
+    )
+    .map(|walk| walk_shape(&walk))
+    .expect("the recorded walk runs");
+    let summary = capture.finish().expect("the capture seals");
+    let (unrecorded, unrecorded_unmet, _) = slice.cached(&ladder, 2);
+    assert!(recorded.starts_with("met 2"), "fixture: tier 2 meets");
+    assert_eq!(recorded, unrecorded, "recording changes no answer");
+    assert_eq!(unmet, unrecorded_unmet);
+    assert_eq!(unmet, [0, 1]);
+    assert_eq!(summary.tiers, 1, "only the tier the walk ended on");
+    let captured =
+        candidate_trades::tier(&root.0, &summary, 0, candidate_trades::DEFAULT_MAX_BYTES)
+            .expect("the met tier's screen");
+    assert_eq!(captured.rules, ladder[2].1, "the met tier's policy");
+    assert_eq!(summary.candidates, 2 * captured.evaluated);
+}
+
+// ---------------------------------------------------------------------------
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 31e467ea..f26b8a1e 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7052,3 +7052,10 @@ old line regex the same input and watched it pass.
 | G18-api-27 | The seek path's `records unreadable` line names the first file that refused a record, not the first file read (D-2046) | `api::bars::window_tests::the_unreadable_line_names_the_first_damaged_file_not_the_first_file` | ✓ |
 | G18-api-28 | The route test's HTTP exchange is bounded at 30 s per read and write, so a server that admits or answers nothing fails it rather than hanging (D-2047) | `api::ingest::route_tests::the_three_routes_answer_and_none_of_them_shadows_the_front_end` | ✓ |
 | G18-api-29 | A dropped calendar `Landing` marks its flight `Abandoned` (or answered), removes it from the flight table, and wakes every follower (D-2047) | `api::calendar_of::tests::a_calendar_landing_releases_its_flight_and_wakes_its_followers_when_dropped` | ✓ |
+
+### Lane 1-b finishing fixes, fixer B (D-4716 onward)
+
+| Id | Invariant | Proof | |
+|---|---|---|---|
+| L1FB-01 | A recorded cascade whose stated policy and whole tier ladder admit nothing captures exactly two screens, the operator's policy and the mildest tier, answers byte for byte as the unrecorded cascade, and costs exactly `2 × tiers + 4 × candidates + 2` `fsync`s, whatever the ladder's length (D-4716; supersedes D-1734's "one tier per policy judged") | `cli::screen_policy_tests::a_recorded_walk_that_admits_nothing_captures_two_screens_not_the_ladder` | ✓ |
+| L1FB-02 | A recorded tier walk that meets a tier captures that tier alone, with its own rules, and answers as the uncaptured cached walk (D-4716) | `cli::screen_policy_tests::a_recorded_walk_captures_only_the_tier_it_ends_on` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 552ce57a..a43faa2e 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65025,3 +65025,48 @@ on a live leader would derive a second time and lose the single-flight
 guarantee D-1443 exists for. **Honest limit:** the `Landing` kill depends on
 test order. A rename that sorted a single-flight test ahead of it would
 restore the timeout, so the ordering is pinned in the test's own doc.
+
+### D-4716 — A recorded tier walk captures the screens it shows, not every tier it judges — 2026-10-09
+
+**Finding.** G2-5, medium. D-1731 made the tier walk judge every tier, and
+D-1734 kept the candidate capture recording "one tier per policy judged". So
+a recorded walk that admits nothing wrote a tier file and two files per
+candidate side for every tier of the ladder: `(1 + T) × (2 + 8C) + 2` `fsync`s and
+`T × 2C` replays, against an acknowledgement budget of 64 MiB that refuses
+the whole recorded run once it is spent. D-1734's bound did not name it, and
+its 3.2 s measurement was the unrecorded path.
+`a_recorded_walk_that_admits_nothing_captures_two_screens_not_the_ladder`
+measured it on the 8-session fixture before the fix: 2,521 captured tiers
+for a 2,520-tier ladder, where the page shows two screens.
+
+**Decision.** `walk_tiers` judges every tier with no capture (`Pricing {
+capture: None, .. }`). The tier the walk ENDS on is judged again with the
+capture: the met tier inside `judge`, or the last tier in `screen_at`. The
+capture only records, so the re-judged rows equal the unrecorded ones and the
+page, selection and priced map are unchanged byte for byte. A recorded
+cascade therefore captures at most two screens, the operator's own policy and
+the tier the walk ended on: at most `6 + 16C` `fsync`s and `4C` acknowledgement
+slots, whatever `T` is. The met tier pays one extra `tier_rows` over its
+cached grids, `O(C × 2 × K)`.
+
+**What changes in stored results.** Only candidate captures of recorded
+audits and screens whose stated policy admitted nothing and whose ladder was
+walked: their catalog now holds two tiers, not `1 + rank + 1` or `1 + T`, and
+`candidate_side_count` falls with it. Tier ordinals now count SHOWN screens.
+The bytes of every file, the catalog layout and version 1 of
+`docs/19-candidate-trades.md` are unchanged, and every reader pages by the
+tier index it is given, so no format version is cut: a capture made before
+this decision reads exactly as it did. No parent identity, ledger row,
+frontier, report or digest changes. D-1734's sentence that the capture "still
+records one tier per policy judged" is superseded by this entry.
+
+**Proof.** `a_recorded_walk_that_admits_nothing_captures_two_screens_not_the_ladder`
+(recorded text equals unrecorded text; `tiers == 2`, tier 0 the operator's
+rules, tier 1 the mildest tier; `fsync`s `== 2 × tiers + 4 × candidates + 2`)
+and `a_recorded_walk_captures_only_the_tier_it_ends_on` (a met walk captures
+exactly the met tier and answers as the uncaptured cached walk). Both failed
+before the fix: `left: 2521, right: 2` and `left: 3, right: 1`.
+
+**Rejected.** Keeping one capture per judged tier and refusing up front when
+`T × 2C × 33` bytes would exceed the budget: the run would still refuse, only
+sooner, for evidence about tiers the page never shows.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 12e9bf09..7bb7a1c7 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -15560,6 +15560,25 @@ per-candidate primitive from `CLAUDE.md` §3 rule 4.
   anyone should expect to reach, because a key whose envelope admits cells is
   a key whose mildest tier is likely to end the walk.
 
+- **A recorded cascade's capture: at most two screens, whatever `T` is**
+  (G2-5, D-4716, next to D-1734). The bound above is the walk's pricing; a
+  RECORDED walk also writes candidate evidence. Each captured screen is one
+  tier file and two files per candidate side, two `fsync`s each, plus
+  `2 × evaluated` acknowledgement slots of 33 bytes against the capture's
+  64 MiB budget. Until D-4716 every judged tier was captured, so a walk where
+  nothing admits paid `(1 + T) × (2 + 8C) + 2` `fsync`s and `(1 + T) × 2C`
+  replays, and the budget refused the run near `T = 64 MiB / (66 × C)`:
+  about 10,000 tiers at `C = 98` and about 100 at the default
+  `screen_cap()` (derived from the slot size, not measured). Now `walk_tiers`
+  judges every tier with no capture and captures only the tier it ends on,
+  so a cascade captures the operator's own policy and that tier: at most
+  `6 + 16C` `fsync`s, `4C` replays and `4C` slots (`C <= screen_cap()`). The
+  met tier pays one extra `tier_rows` over its cached grids,
+  `O(C × 2 × K)`. COUNTED, not timed:
+  `a_recorded_walk_that_admits_nothing_captures_two_screens_not_the_ladder`
+  measures two captured screens and exactly `2 × tiers + 4 × candidates + 2`
+  `fsync`s on a 2,520-tier ladder (2,521 tiers before the fix).
+
 - **`tiers`, per generated ladder** (W2-cli8-1, D-1726). The work is a fixed
   number of O(N) scans over the bars (`reference_price`, `grid_step_ppm`,
   `grid_rungs`, `max_stop_points`, each once), one
diff --git a/docs/19-candidate-trades.md b/docs/19-candidate-trades.md
index 03f8e6e8..9766d4ea 100644
--- a/docs/19-candidate-trades.md
+++ b/docs/19-candidate-trades.md
@@ -37,6 +37,23 @@ publication. A callback error is latched: not-yet-started candidates and sides
 check it and skip work; a grid already executing finishes its current engine
 call. No immediate interruption inside that grid is claimed.
 
+## Which screen passes are captured (D-4716)
+
+A capture holds the screen passes the page shows, not every tier the walk
+judged. `screen_cascade` screens the operator's own policy first, and that
+pass is captured. When the walk runs, every tier is judged with no capture,
+and only the tier the walk ends on is captured: the strictest tier that
+admitted a row, or the last tier when none did. So a capture holds one or two
+tiers. Before D-4716 every judged tier was captured, `1 + T` tiers on a walk
+that admitted nothing, which spent the 64 MiB acknowledgement budget and
+refused the run on long ladders.
+
+The bytes of every file and the catalog are unchanged, and this stays
+version 1. Tier ordinals count captured passes in the order they ran, as
+before; each tier states its own policy words, so a reader never infers a
+tier's policy from its ordinal. A capture written before D-4716 reads as it
+always did.
+
 ## Paths and byte layout
 
 All files live under:
-- 
2.43.0


From f39848a82e64118623e1f725e24c253ae37fc510 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:29:24 +0000
Subject: [PATCH 2/8] cli, api: zero points is no ceiling; one support domain
 at every screen entry (W2-cli8-10, W2-cli8-11, D-4717, D-4718)

Before: screen_range_in_points loaded the span and then refused a zero
stop ceiling as "0 ppm, which admits nothing", though D-1732 sends the
api's zero there as no ceiling. It and the api's screen command refused
only a zero support, and screen_range refused none, so 100% or more
loaded a span and recorded a screen that could find nothing.

After: screen_range_in_points converts with elite_ceiling_ppm, so zero
is no ceiling. support_ppm_in_domain is the one 1..1_000_000 domain;
parse_support_ppm is the parse followed by it, and screen_range,
screen_range_in_points and the api's screen parser ask it before
anything is read. Tested over a swept index, and the api test drives
the screen command end to end. Gate 23 declares the two new
stdout proof lines (sweeprun.rs to 2, audited_stored_tests.rs 1).

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 .github/workflows/ci.yml               |   8 +-
 crates/api/src/sweeprun.rs             | 179 ++++++++++++++++++++++++-
 crates/cli/src/audited_stored_tests.rs | 128 ++++++++++++++++++
 crates/cli/src/lib.rs                  |  50 +++++--
 docs/04-invariants.md                  |   2 +
 docs/05-decisions.md                   |  57 ++++++++
 6 files changed, 404 insertions(+), 20 deletions(-)

diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 32e24fbb..7088ccca 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -1742,7 +1742,7 @@ jobs:
           # is not a diagnostic: nothing is wrong when it prints.
           declared="$declared api/src/booleanlaunch_tests.rs:println!:2
           api/src/frontierjson.rs:println!:1
-          api/src/sweeprun.rs:println!:1
+          api/src/sweeprun.rs:println!:2
           api/src/trades.rs:println!:2
           api/src/recovery.rs:println!:1
           cli/src/boolean_candidate_tests.rs:eprintln!:2
@@ -1760,6 +1760,12 @@ jobs:
           # was degraded, so a clean run prints nothing.
           declared="$declared pull/src/csv.rs:eprintln!:1
           pull/src/masters.rs:eprintln!:1"
+          # TWO MORE STDOUT PROOF LINES, D-0695's kind (D-4717, D-4718): api
+          # sweeprun's second child prints `ZERO-POINT SCREEN RAN` (counted in
+          # its line above), and a cli child prints `POINTS SCREEN DOORS
+          # CHECKED`. Each sits in a `#[cfg(test)]` module and prints only
+          # after its child passed.
+          declared="$declared cli/src/audited_stored_tests.rs:println!:1"
 
           # CLAUSE A2 — the same question, asked of the handle spellings.
           #
diff --git a/crates/api/src/sweeprun.rs b/crates/api/src/sweeprun.rs
index 8554cc0f..1bf6c90e 100644
--- a/crates/api/src/sweeprun.rs
+++ b/crates/api/src/sweeprun.rs
@@ -3196,13 +3196,11 @@ fn command_from_wire(body: &WireBody) -> Result<Command, Refusal> {
                 "support_ppm",
                 "a whole number of parts per million of this rung's own bars",
             )?;
-            if support_ppm == 0 {
-                return Err(Refusal::Malformed(
-                    "`support_ppm` is 0. Every combination is then frequent, \
-                     the frontier never empties and the walk has no end."
-                        .to_owned(),
-                ));
-            }
+            // `cli`'s ONE SUPPORT DOMAIN, not a copy of half of it: this
+            // refused only 0, so 100% or more ran (W2-cli8-11, D-4718).
+            cli::support_ppm_in_domain(support_ppm).map_err(|why| {
+                Refusal::Malformed(format!("`support_ppm` {support_ppm} is refused: {why}."))
+            })?;
             let max_points: i64 = whole(body, "max_points", "a whole number of index points")?;
             let top: usize = whole(body, "top", "a whole number of rows to list")?;
             // Zero is no ceiling, as at the descent door and in `cli` (D-1732).
@@ -7169,4 +7167,171 @@ mod tests {
         assert!(elsewhere_over(&dir, 0).contains(r#""status":"unknown""#));
         let _ = std::fs::remove_dir_all(dir);
     }
+
+    /// W2-cli8-10, D-4717. A ZERO STOP CEILING IS NO CEILING, END TO END.
+    ///
+    /// `a_stop_ceiling_of_zero_means_no_ceiling_as_cli_reads_it` proved the
+    /// parse only. The command it parsed reached `cli::screen_range_in_points`,
+    /// which loaded the span and then refused zero as a ceiling that "converts
+    /// to 0 ppm, which admits nothing". This one sends the body through
+    /// `command_from` and `conduct_command` over a stored, swept NIFTY month
+    /// and requires the screen to run. In a child process: the store is named
+    /// by `BRUTEX_STORE`, which a test cannot set on itself (`crate::isolated`).
+    #[test]
+    fn a_zero_point_screen_command_runs_as_no_ceiling_end_to_end() {
+        const CHILD: &str = "BRUTEX_TEST_API_ZERO_POINT_SCREEN";
+        if std::env::var_os(CHILD).is_some() {
+            zero_point_screen_child();
+            return;
+        }
+        let root = crate::scratch::path("zero-point-screen");
+        let _ = std::fs::remove_dir_all(&root);
+        std::fs::create_dir_all(&root).expect("the child's store root");
+        write_warmed_nifty_may(&root);
+        let logs = root.join("logs");
+        let out = crate::isolated::rerun(
+            "sweeprun::tests::a_zero_point_screen_command_runs_as_no_ceiling_end_to_end",
+            &[
+                (CHILD, std::ffi::OsStr::new("1")),
+                ("BRUTEX_STORE", root.as_os_str()),
+                ("BRUTEX_LOG_DIR", logs.as_os_str()),
+                ("BRUTEX_VALIDATE", std::ffi::OsStr::new("0")),
+                ("BRUTEX_CEILING", std::ffi::OsStr::new("256")),
+            ],
+        );
+        assert!(out.contains("ZERO-POINT SCREEN RAN"), "{out}");
+        let _ = std::fs::remove_dir_all(&root);
+    }
+
+    /// The child half: one screen command over the stored May 2025.
+    fn zero_point_screen_child() {
+        let asked = command_from(
+            r#"{"feed":"zerodha","underlying":"NIFTY","from_year":2025,"from_month":5,"to_year":2025,"to_month":5,"command":"screen","rung":"1min","support_ppm":999999,"max_points":0,"top":1}"#,
+        )
+        .expect("zero is no ceiling at the parse");
+        let progress = conduct_command(&asked, 1, 2);
+        let said = format!(
+            "{}{}",
+            progress.report.as_deref().unwrap_or(""),
+            progress.refusal.as_deref().unwrap_or("")
+        );
+        assert!(
+            !said.contains("admits nothing"),
+            "zero was read as a ceiling: {said}"
+        );
+        if cli::commit_stamp().is_none() {
+            assert!(said.contains("no verified commit stamp"), "{said}");
+        } else {
+            assert!(progress.refusal.is_none(), "{said}");
+            assert!(said.contains("RESULT RECORDED"), "{said}");
+        }
+        println!("ZERO-POINT SCREEN RAN");
+    }
+
+    /// One generated NSE session of 2025: every minute of the calendar's
+    /// windows, constant prices. Never market data.
+    fn generated_session(month: u8, date: u8) -> Vec<store::format::Bar> {
+        let civil = pull::session::Day::new(2025, month, date).expect("date");
+        let day = i64::from(civil.days_from_epoch());
+        let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
+            return Vec::new();
+        };
+        session
+            .windows
+            .iter()
+            .take(usize::from(session.count))
+            .flat_map(|window| window.from..=window.to)
+            .map(|minute| store::format::Bar {
+                ts_micros: day * 86_400_000_000 + i64::from(minute) * 60_000_000
+                    - pull::session::IST_OFFSET_SECS * 1_000_000,
+                open: 100_000,
+                high: 110_000,
+                low: 90_000,
+                close: 101_000,
+                volume: 100,
+                open_interest: i64::MIN,
+            })
+            .collect()
+    }
+
+    /// The warmed NIFTY store `cli`'s stored fixtures use: the April 30 and
+    /// May 2 seed sessions, then May 5 to 13, at 1min, 5min and 1day.
+    fn write_warmed_nifty_may(root: &std::path::Path) {
+        use store::path::{FileKind, StorePath, Timeframe, YearMonth};
+        let key = brutex_core::instrument::InstrumentKey::index(
+            brutex_core::instrument::Exchange::Nse,
+            "NIFTY",
+        )
+        .expect("NIFTY");
+        let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
+        let symbol = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]);
+        let write = |month: u8, timeframe: Timeframe, rows: &[store::format::Bar]| {
+            let path = StorePath::for_key(
+                brutex_core::vendor::Vendor::Zerodha,
+                &key,
+                timeframe,
+                YearMonth::new(2025, month).expect("month"),
+                FileKind::Bars,
+            )
+            .expect("path");
+            store::file::BarFile::open_or_create(root, path, symbol)
+                .expect("writer")
+                .append(rows)
+                .expect("generated rows");
+        };
+        let days = [(4_u8, 30_u8), (5, 2)]
+            .into_iter()
+            .chain((5..=13).map(|day| (5, day)));
+        for (month, day) in days {
+            let rows = generated_session(month, day);
+            let Some(first) = rows.first().copied() else {
+                continue;
+            };
+            write(month, Timeframe::MINUTE_1, &rows);
+            write(month, Timeframe::DAY_1, &[first]);
+            if month == 5 {
+                let five: Vec<_> = rows.iter().step_by(5).copied().collect();
+                write(month, Timeframe::MINUTE_5, &five);
+            }
+        }
+    }
+
+    /// W2-cli8-11, D-4718. THE `screen` COMMAND'S SUPPORT DOMAIN IS `cli`'s.
+    ///
+    /// This door refused only 0, so 1,000,000 ppm (100%) or more ran and
+    /// recorded a screen that could find nothing, while argv and
+    /// `BRUTEX_SUPPORT_PPM` refused it by name (D-1722). It now asks the one
+    /// validator they ask, and names its sentence.
+    #[test]
+    fn the_screen_command_refuses_the_support_domain_cli_refuses() {
+        for (support, refusal) in [
+            (0_u64, Some("0 would disable extinction")),
+            (1, None),
+            (999_999, None),
+            (1_000_000, Some("1000000 is 100%")),
+            (1_000_001, Some("1000000 is 100%")),
+            (u64::MAX, Some("1000000 is 100%")),
+        ] {
+            let parsed = command_from(&command_body(&format!(
+                r#""command":"screen","rung":"15min","support_ppm":{support},"max_points":20,"top":25"#
+            )));
+            match refusal {
+                None => assert!(
+                    matches!(
+                        parsed,
+                        Ok(super::Command::Screen { support_ppm, .. }) if support_ppm == support
+                    ),
+                    "{support}: {parsed:?}"
+                ),
+                Some(why) => {
+                    let refused = parsed.expect_err("outside the one support domain");
+                    assert!(
+                        refused.why().contains(why) && refused.why().contains("support_ppm"),
+                        "{support}: {}",
+                        refused.why()
+                    );
+                }
+            }
+        }
+    }
 }
diff --git a/crates/cli/src/audited_stored_tests.rs b/crates/cli/src/audited_stored_tests.rs
index 7f1a83d2..d7aad23e 100644
--- a/crates/cli/src/audited_stored_tests.rs
+++ b/crates/cli/src/audited_stored_tests.rs
@@ -3641,3 +3641,131 @@ fn the_minute_gap_census_asks_the_shares_dated_close() {
     );
     let _ignored = std::fs::remove_dir_all(&store);
 }
+
+/// Re-runs one test of this binary in a child whose `BRUTEX_STORE` names
+/// `root`, and requires the child's own proof line.
+fn rerun_over_store(test: &str, child: &str, root: &std::path::Path, proof: &str) {
+    let output = std::process::Command::new(std::env::current_exe().expect("this test binary"))
+        .args(["--exact", test, "--nocapture", "--test-threads=1"])
+        .env(child, "1")
+        .env("BRUTEX_STORE", root)
+        .env("BRUTEX_LOG_DIR", root.join("logs"))
+        .output()
+        .expect("the child starts");
+    let stdout = String::from_utf8_lossy(&output.stdout);
+    assert!(
+        output.status.success(),
+        "{stdout}{}",
+        String::from_utf8_lossy(&output.stderr)
+    );
+    assert!(stdout.contains("1 passed"), "{stdout}");
+    assert!(stdout.contains(proof), "{stdout}");
+}
+
+/// W2-cli8-10 (D-4717) and W2-cli8-11 (D-4718), over a SWEPT index, so each
+/// call reaches the code under test. G18-cli-a-33 asked these doors about
+/// `NOT-A-SWEPT-INDEX`, whose span refuses before either the ceiling or the
+/// support is ever looked at, so it passed whatever they did.
+///
+/// * A support outside `1..1_000_000` is refused by the one validator argv and
+///   `BRUTEX_SUPPORT_PPM` use, at both entries a caller names a support
+///   through, before anything is read or recorded. The points door refused
+///   only zero and `screen_range` nothing. A descent's own steps take the
+///   supports its walk derives and are not entries.
+/// * `MAX_POINTS = 0` is no ceiling (D-1732). `screen_range_in_points` loaded
+///   the span and refused it as a ceiling that "converts to 0 ppm, which
+///   admits nothing". The elite door, fixed by D-1721, is held to the same
+///   assertion here rather than to the absence of an old sentence.
+#[test]
+fn the_points_screen_reads_zero_as_no_ceiling_and_shares_the_support_domain() {
+    const CHILD: &str = "BRUTEX_TEST_POINTS_SCREEN_DOORS";
+    if std::env::var_os(CHILD).is_some() {
+        points_screen_doors_child();
+        return;
+    }
+    let fixture = Fixture::warmed();
+    rerun_over_store(
+        "audited_stored::tests::the_points_screen_reads_zero_as_no_ceiling_and_shares_the_support_domain",
+        CHILD,
+        &fixture.root,
+        "POINTS SCREEN DOORS CHECKED",
+    );
+}
+
+/// The child half, over the warmed NIFTY store `BRUTEX_STORE` names.
+fn points_screen_doors_child() {
+    let _knobs = crate::knobs::serially();
+    crate::knobs::clear_all();
+    crate::knobs::set("BRUTEX_VALIDATE", "0");
+    crate::knobs::set("BRUTEX_CEILING", "256");
+    // THE PREMISE, CHECKED BEFORE THE WORK (G18-cli-a-36, D-2018).
+    assert!(
+        !crate::validate_from_env(),
+        "BRUTEX_VALIDATE=0 turns validation off"
+    );
+    let root = crate::store_root().expect("the fixture store");
+    let span = ((2025, 5), (2025, 5));
+    let policy = crate::Policy {
+        rules: crate::Rules::BASELINE,
+        lens: runner::rank::Lens::Detectability,
+        validate: false,
+    };
+    for (support, why) in [
+        (0_u64, "0 would disable extinction"),
+        (1_000_000, "1000000 is 100%"),
+        (u64::MAX, "1000000 is 100%"),
+    ] {
+        let points =
+            crate::screen_range_in_points("zerodha", "NIFTY", "1min", span, support, 20, 1);
+        let plain =
+            crate::screen_range("zerodha", "NIFTY", "1min", span.0, span.1, support, policy);
+        for (door, page) in [("points", &points), ("screen_range", &plain)] {
+            assert!(
+                page.starts_with("refused: ") && page.contains(why),
+                "{door} {support}: {page}"
+            );
+        }
+    }
+    assert!(
+        !crate::results::Results::path(&root).exists(),
+        "a refused support recorded nothing"
+    );
+    assert_eq!(
+        crate::sweep_evidence::latest(&root, 1_048_576).expect("readable evidence"),
+        None,
+        "and began no attempt"
+    );
+
+    let zero = crate::screen_range_in_points("zerodha", "NIFTY", "1min", span, 999_999, 0, 1);
+    assert!(
+        !zero.contains("admits nothing"),
+        "zero was read as a ceiling: {zero}"
+    );
+    if crate::commit_stamp().is_none() {
+        assert!(zero.contains("no verified commit stamp"), "{zero}");
+    } else {
+        assert!(zero.starts_with(crate::STORED_PROVENANCE), "{zero}");
+        assert!(zero.contains("RESULT RECORDED"), "{zero}");
+    }
+
+    // The elite door over the same swept span. See `elite_ranks_by_the_lens_it_is_given`
+    // for why these two knobs.
+    crate::knobs::set("BRUTEX_MIN_WIN_RATE_BP", "0");
+    crate::knobs::set("BRUTEX_CEILING", "1048576");
+    let elite = crate::elite_descend_in_points_inner(
+        "zerodha",
+        "NIFTY",
+        "1min",
+        span,
+        (0, 1),
+        runner::rank::Lens::Payoff,
+        None,
+    );
+    assert!(
+        !elite.contains("admits nothing"),
+        "zero was read as a ceiling: {elite}"
+    );
+    assert!(elite.contains("ELITE, SELF-TUNING"), "{elite}");
+    crate::knobs::clear_all();
+    println!("POINTS SCREEN DOORS CHECKED");
+}
diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index 3589af06..16ff35ee 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -2819,15 +2819,34 @@ fn parse_sessions(text: &str) -> Result<i64, &'static str> {
 ///
 /// A non-number, zero, or 1,000,000 and anything above it.
 fn parse_support_ppm(text: &str) -> Result<u64, &'static str> {
-    match text.parse::<u64>() {
-        Err(_) => Err("SUPPORT_PPM is not a whole number"),
-        Ok(0) => Err("SUPPORT_PPM must be 1 or more; 0 would disable extinction"),
-        Ok(ppm) if ppm >= 1_000_000 => Err(
+    text.parse::<u64>()
+        .map_err(|_| "SUPPORT_PPM is not a whole number")
+        .and_then(support_ppm_in_domain)
+}
+
+/// `ppm` itself when it is a support [`parse_support_ppm`] admits, or the
+/// sentence that refuses it: the ONE support domain, `1..1_000_000`.
+///
+/// # Every entry asks this (W2-cli8-11, D-4718)
+///
+/// [`parse_support_ppm`] is this after the parse, so argv and
+/// `BRUTEX_SUPPORT_PPM` ask it; [`screen_range`], [`screen_range_in_points`]
+/// and the api's `screen` command take a number already parsed and ask it
+/// directly. Those three refused only zero, so 100% or more loaded a span and
+/// recorded a screen that could find nothing.
+///
+/// # Errors
+///
+/// Zero, or 1,000,000 and anything above it.
+pub const fn support_ppm_in_domain(ppm: u64) -> Result<u64, &'static str> {
+    match ppm {
+        0 => Err("SUPPORT_PPM must be 1 or more; 0 would disable extinction"),
+        1_000_000.. => Err(
             "SUPPORT_PPM is parts per million, so 1000000 is 100%: a pattern on \
                  every bar, which D-0080 excludes, so at or above it nothing can \
                  be frequent",
         ),
-        Ok(ppm) => Ok(ppm),
+        ppm => Ok(ppm),
     }
 }
 
@@ -16698,11 +16717,9 @@ pub fn screen_range_in_points(
     top: usize,
 ) -> String {
     let (from, to) = span;
-    if support_ppm == 0 {
-        return "refused: a support of zero makes every combination frequent, \
-                so the frequent frontier never empties and the walk has no \
-                end.\n"
-            .to_owned();
+    // THE ONE SUPPORT DOMAIN, BEFORE ANYTHING IS READ (W2-cli8-11, D-4718).
+    if let Err(why) = support_ppm_in_domain(support_ppm) {
+        return format!("refused: {why}\n");
     }
     // ZERO IS "NO CEILING BEYOND THE SWEPT LADDER". See `elite_arm` for the
     // full argument; in short, `Rules::admits` already reads `max_mae_ppm == 0`
@@ -16739,8 +16756,12 @@ pub fn screen_range_in_points(
             );
         }
     };
-    let reference = reference_price(&span.bars);
-    let max_mae_ppm = match ceiling_in_ppm(max_points, reference) {
+    // ZERO IS NO CEILING HERE TOO (W2-cli8-10, D-4717). This converted zero
+    // with `ceiling_in_ppm`, which refused it as "0 ppm, which admits
+    // nothing" after the span had been loaded, so the api's `screen` command
+    // passed zero through (D-1732) to a refusal. `elite_ceiling_ppm` is the
+    // one zero rule `elite` already uses.
+    let max_mae_ppm = match elite_ceiling_ppm(max_points, || Ok(reference_price(&span.bars))) {
         Ok(ppm) => ppm,
         Err(why) => {
             drop(span);
@@ -17862,6 +17883,11 @@ pub fn screen_range(
     support_ppm: u64,
     policy: Policy,
 ) -> String {
+    // THE ONE SUPPORT DOMAIN (W2-cli8-11, D-4718): this refused nothing, and
+    // the kernel turned zero into a one-hit threshold.
+    if let Err(why) = support_ppm_in_domain(support_ppm) {
+        return format!("refused: {why}\n");
+    }
     match screen_range_inner(
         vendor_word,
         underlying,
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index f26b8a1e..817c04ed 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7059,3 +7059,5 @@ old line regex the same input and watched it pass.
 |---|---|---|---|
 | L1FB-01 | A recorded cascade whose stated policy and whole tier ladder admit nothing captures exactly two screens, the operator's policy and the mildest tier, answers byte for byte as the unrecorded cascade, and costs exactly `2 × tiers + 4 × candidates + 2` `fsync`s, whatever the ladder's length (D-4716; supersedes D-1734's "one tier per policy judged") | `cli::screen_policy_tests::a_recorded_walk_that_admits_nothing_captures_two_screens_not_the_ladder` | ✓ |
 | L1FB-02 | A recorded tier walk that meets a tier captures that tier alone, with its own rules, and answers as the uncaptured cached walk (D-4716) | `cli::screen_policy_tests::a_recorded_walk_captures_only_the_tier_it_ends_on` | ✓ |
+| L1FB-03 | A zero-point `screen` reaches `cli::screen_range_in_points` as no ceiling and runs over a swept index, from the api command end to end and from the cli door; the elite door agrees (D-4717) | `api::sweeprun::tests::a_zero_point_screen_command_runs_as_no_ceiling_end_to_end`, `cli::audited_stored::tests::the_points_screen_reads_zero_as_no_ceiling_and_shares_the_support_domain` | ✓ |
+| L1FB-04 | `screen_range`, `screen_range_in_points` and the api's `screen` command admit exactly the support domain `1..1_000_000` that argv and `BRUTEX_SUPPORT_PPM` admit, refusing with its sentence before any ledger row or attempt is written (D-4718) | `cli::audited_stored::tests::the_points_screen_reads_zero_as_no_ceiling_and_shares_the_support_domain`, `api::sweeprun::tests::the_screen_command_refuses_the_support_domain_cli_refuses` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index a43faa2e..e88c8180 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65070,3 +65070,60 @@ before the fix: `left: 2521, right: 2` and `left: 3, right: 1`.
 **Rejected.** Keeping one capture per judged tier and refusing up front when
 `T × 2C × 33` bytes would exceed the budget: the run would still refuse, only
 sooner, for evidence about tiers the page never shows.
+
+### D-4717 — `screen_range_in_points` reads a zero stop ceiling as no ceiling — 2026-10-09
+
+**Finding.** W2-cli8-10. D-1732 let the api's `screen` command pass
+`max_points = 0` through to `cli::screen_range_in_points` on the claim that
+`cli` reads zero as no ceiling. That function loaded the span and then
+converted zero with `ceiling_in_ppm`, which refused it as "0 ppm, which
+admits nothing". So the browser's zero-point screen was a refusal after a
+span load. G18-cli-a-33 had asked the door about an unswept instrument,
+which refuses before the ceiling is read, so nothing saw it.
+
+**Decision.** The conversion is `elite_ceiling_ppm`, the one zero rule
+`elite` already uses (D-1721): zero is `max_mae_ppm = 0`, which
+`Rules::admits` and `Levels::forced` read as no ceiling; a positive ceiling
+converts as before, now with `elite_ceiling_ppm`'s refusal sentence. The
+span is still loaded, because the policy's floors are measured off it.
+`cli screen`'s argv arm, which takes `MIN_RR` and builds its own rules, still
+refuses zero at its door by name before reading anything; that verb never
+offered zero and is unchanged.
+
+**What changes in stored results.** A zero-point api screen now runs and
+records a screen where it refused; nothing already recorded changes.
+
+**Proof.** `api::sweeprun::tests::a_zero_point_screen_command_runs_as_no_ceiling_end_to_end`
+sends the body through `command_from` and `conduct_command` over a stored
+NIFTY May in a child; before the fix it read "converts to 0 ppm, which admits
+nothing". `cli::audited_stored::tests::the_points_screen_reads_zero_as_no_ceiling_and_shares_the_support_domain`
+asks the cli door and the elite door the same over a swept index. Each
+child's proof line is declared to gate 23 beside D-0695's.
+
+### D-4718 — Every screen support entry asks the one support domain — 2026-10-09
+
+**Finding.** W2-cli8-11. D-1722 gave argv and `BRUTEX_SUPPORT_PPM` one domain,
+`1..1_000_000`, in `parse_support_ppm`. Three entries take a number already
+parsed and did not ask it: the api's `screen` command and
+`cli::screen_range_in_points` refused only zero, and `cli::screen_range`
+refused nothing, so 100% or more loaded a span and recorded a screen that
+could find nothing, and `screen_range` turned zero into a one-hit threshold.
+
+**Decision.** The domain is `cli::support_ppm_in_domain`, a public `const fn`;
+`parse_support_ppm` is the parse followed by it, so argv and the knob are
+unchanged. `screen_range`, `screen_range_in_points` and the api's `screen`
+parser ask it before anything is read, and refuse with its sentence (the api
+prefixes the field name). The descent's internal steps
+(`screen_range_for_attempt`) take supports the walk derives, not entries, and
+are not routed through it.
+
+**What changes in stored results.** A screen at 0 or at 1,000,000 ppm or more
+through those three doors is refused instead of run; nothing recorded
+changes.
+
+**Proof.** `the_points_screen_reads_zero_as_no_ceiling_and_shares_the_support_domain`
+(0, 1,000,000 and `u64::MAX` refused at the points door and `screen_range`,
+with no ledger and no attempt written; before the fix the points door's zero
+refusal lacked the domain's sentence) and
+`api::sweeprun::tests::the_screen_command_refuses_the_support_domain_cli_refuses`
+(0, 1,000,000, 1,000,001 and `u64::MAX` refused, 1 and 999,999 parsed).
-- 
2.43.0


From d470cf19f11cc0eac5620e4e0f49dee4bf50ed9c Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:29:25 +0000
Subject: [PATCH 3/8] cli: the derived-support rung prepares through the
 audit's inputs; one daily read per build (W2-cli8-6, G2-3, D-4719)

Before: one_rung_cached's derived-support branch built its own column
from an empty withheld set with no minute-hole census: three edge-holed
days cost four builds (five with the audit's), each refused pass wrote
a preparation attempt, an interior-gap day stayed in its column while
the audit withheld it, and an unstamped rung built and recorded first.
column_withholding_at_build reloaded the daily context on every pass.

After: the branch reads the audit's AuditCache entry (census, one
column, one preparation digest) and refuses unstamped before any load;
the wrapper it alone called is removed and G18-cli-a-24 asks the build
directly. The daily context is read once above the retry loop. The
o1cli-2 and o1cli-3 limits and their tests now state one build per rung.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/audited_stored_tests.rs | 220 +++++++++++++++++++++++++
 crates/cli/src/g18_cli_a_tests.rs      |  25 ++-
 crates/cli/src/lib.rs                  | 209 ++++++++++-------------
 crates/cli/src/stored.rs               |  11 ++
 crates/cli/tests/limits_o1cli_2.rs     |  36 ++--
 crates/cli/tests/limits_o1cli_3.rs     |  11 +-
 docs/04-invariants.md                  |   4 +
 docs/05-decisions.md                   |  51 ++++++
 docs/06-limits.md                      |  53 +++---
 9 files changed, 451 insertions(+), 169 deletions(-)

diff --git a/crates/cli/src/audited_stored_tests.rs b/crates/cli/src/audited_stored_tests.rs
index d7aad23e..e16487be 100644
--- a/crates/cli/src/audited_stored_tests.rs
+++ b/crates/cli/src/audited_stored_tests.rs
@@ -3769,3 +3769,223 @@ fn points_screen_doors_child() {
     crate::knobs::clear_all();
     println!("POINTS SCREEN DOORS CHECKED");
 }
+
+/// A warmed NIFTY May whose sessions of the 6th, 8th and 12th stop at 15:24,
+/// as in `sessions_missing_their_closing_minutes_are_withheld_up_front`, and,
+/// when `interior`, whose 9th also lacks 10:57: a minute no five-minute bar
+/// closes on, so only the census can see it.
+fn holed_may(interior: bool) -> Fixture {
+    let fixture = Fixture::warmed();
+    fixture.rewrite_owned_minutes(|day, rows| {
+        if [6_u8, 8, 12].contains(&day) {
+            let keep = rows.len().saturating_sub(5);
+            rows.truncate(keep);
+        }
+        if interior && day == 9 {
+            rows.remove(102);
+        }
+    });
+    fixture
+}
+
+/// The IST epoch days of these May 2025 dates.
+fn may_days(dates: &[u8]) -> Vec<i64> {
+    dates
+        .iter()
+        .map(|date| {
+            i64::from(
+                pull::session::Day::new(2025, 5, *date)
+                    .expect("date")
+                    .days_from_epoch(),
+            )
+        })
+        .collect()
+}
+
+/// A derived-support rung of `holed` under `commit`, counted.
+fn derived_support_rung(holed: &Fixture, commit: Option<&'static str>) -> crate::RungRow {
+    crate::COLUMN_BUILD_ATTEMPTS.with(|count| count.set(0));
+    crate::AUDIT_INPUT_LOADS.with(|loads| loads.set(0));
+    crate::AUTO_SUPPORT_SWEPT.with(|swept| swept.set(None));
+    crate::one_rung_cached(
+        crate::RungAsk {
+            vendor_word: "zerodha",
+            underlying: "NIFTY",
+            rung: "5min",
+            from: (2025, 5),
+            to: (2025, 5),
+            support_ppm: None,
+            attempt: None,
+        },
+        crate::RungStore {
+            root: Ok(holed.root.clone()),
+            commit,
+        },
+        &mut crate::AuditCache::default(),
+    )
+}
+
+/// **A derived-support rung runs the minute-hole census, builds one column,
+/// and sizes its support on the swept population the audit then sweeps.**
+/// W2-cli8-6 and G2-3, D-4719.
+///
+/// `one_rung_cached`'s `auto` branch built its own column from an EMPTY
+/// withheld set: three edge-holed days cost three refused passes, each under
+/// a durable preparation attempt the audit never used, and an interior-gap
+/// day the census withholds stayed in, so `min_hits` was sized on a different
+/// population from the one the audit swept. It now asks the audit's own
+/// `AuditCache` entry, as the named-support branch does.
+#[test]
+fn a_derived_support_rung_sizes_on_the_audits_census_and_builds_once() {
+    const COMMIT: &str = "generated-derived-support-rung-fixture";
+    let _knobs = crate::knobs::serially();
+    crate::knobs::clear_all();
+    crate::knobs::set("BRUTEX_VALIDATE", "0");
+    // THE PREMISE, CHECKED BEFORE THE WORK (G18-cli-a-36, D-2018).
+    assert!(
+        !crate::validate_from_env(),
+        "BRUTEX_VALIDATE=0 turns validation off"
+    );
+    let holed = holed_may(true);
+    let row = derived_support_rung(&holed, Some(COMMIT));
+    assert_eq!(
+        crate::COLUMN_BUILD_ATTEMPTS.with(std::cell::Cell::get),
+        1,
+        "the census withheld every holed day before the one build: {:?}",
+        row.outcome
+    );
+    assert_eq!(crate::AUDIT_INPUT_LOADS.with(std::cell::Cell::get), 1);
+
+    // The audit's own preparation, on an identical store.
+    let twin = holed_may(true);
+    let inputs = crate::load_audit_inputs(
+        &twin.root,
+        Vendor::Zerodha,
+        "NIFTY",
+        "5min",
+        ((2025, 5), (2025, 5)),
+        COMMIT,
+    )
+    .expect("the audit's inputs");
+    assert_eq!(inputs.withheld_days, may_days(&[6, 8, 9, 12]));
+    let swept = inputs.column.census().swept;
+    assert_eq!(
+        crate::AUTO_SUPPORT_SWEPT.with(std::cell::Cell::get),
+        Some(swept),
+        "the support was sized on another population: {:?}",
+        row.outcome
+    );
+    if crate::commit_stamp().is_none() {
+        let why = row.outcome.expect_err("an unstamped build runs no probe");
+        assert!(why.contains("no verified commit stamp"), "{why}");
+    } else {
+        let digest = crate::minute_gaps::bind_withheld(
+            crate::stored_anchored_digest(&inputs.folded, &inputs.exact_minute, &inputs.daily)
+                .expect("the preparation digest"),
+            &inputs.withheld_days,
+        );
+        let statistical = crate::min_hits_for_swept(swept, crate::statistical_support_floor(swept));
+        let affordable =
+            crate::affordable_min_hits(&inputs.column, &twin.root, &inputs.span, digest)
+                .expect("the twin's probe");
+        let record = row.outcome.expect("the rung recorded");
+        assert_eq!(record.min_hits, affordable.max(statistical));
+    }
+    crate::knobs::clear_all();
+}
+
+/// **A derived-support rung with no commit stamp prepares nothing.** W2-cli8-6,
+/// D-4719.
+///
+/// The named-support branch already kept an unstamped rung from writing a
+/// preparation attempt the audit would not have written. The derived branch
+/// stamped its own preparation with the BINARY's stamp, so a rung the audit
+/// was about to refuse as unstamped built its column, durably, first.
+#[test]
+fn an_unstamped_derived_support_rung_prepares_nothing() {
+    let _knobs = crate::knobs::serially();
+    crate::knobs::clear_all();
+    let holed = holed_may(true);
+    let row = derived_support_rung(&holed, None);
+    let why = row.outcome.expect_err("an unstamped rung refuses");
+    assert!(why.contains("no verified commit stamp"), "{why}");
+    assert_eq!(crate::COLUMN_BUILD_ATTEMPTS.with(std::cell::Cell::get), 0);
+    assert_eq!(crate::AUDIT_INPUT_LOADS.with(std::cell::Cell::get), 0);
+    assert_eq!(crate::AUTO_SUPPORT_SWEPT.with(std::cell::Cell::get), None);
+    assert_eq!(
+        crate::sweep_evidence::latest(&holed.root, 1_048_576).expect("readable evidence"),
+        None,
+        "no attempt of any kind was begun"
+    );
+    crate::knobs::clear_all();
+}
+
+/// **A column build's retry loop reads the daily context once, and answers
+/// exactly as the census path does.** W2-cli8-6, D-4719.
+///
+/// The daily context is derived from the WHOLE folded series (D-1781), which
+/// no pass changes, yet every pass of the 64 reloaded it from disk.
+#[test]
+fn a_column_builds_retry_loop_reads_the_daily_context_once() {
+    let edged = holed_may(false);
+    let span = ((2025, 5), (2025, 5));
+    let loaded = stored::load_span(
+        &edged.root,
+        Vendor::Zerodha,
+        "NIFTY",
+        "5min",
+        span.0,
+        span.1,
+    )
+    .expect("the signal span");
+    let folded = loaded.bars.clone();
+    let mut bars = loaded.bars.clone();
+    let mut withheld: Vec<i64> = Vec::new();
+    crate::COLUMN_BUILD_ATTEMPTS.with(|count| count.set(0));
+    stored::DAILY_CONTEXT_LOADS.with(|loads| loads.set(0));
+    let (column, digest) = crate::column_withholding_at_build(
+        &edged.root,
+        Vendor::Zerodha,
+        "NIFTY",
+        span,
+        crate::FoldedSeries {
+            folded: &folded,
+            days: &mut withheld,
+            bars: &mut bars,
+        },
+        stored::rung_length_micros("5min").expect("five minutes"),
+        crate::StoredPreparationBuild {
+            rung: "5min",
+            commit: None,
+        },
+    )
+    .expect("the retried build");
+    assert_eq!(
+        crate::COLUMN_BUILD_ATTEMPTS.with(std::cell::Cell::get),
+        4,
+        "one refused pass per edge-holed day, then the build"
+    );
+    assert_eq!(stored::DAILY_CONTEXT_LOADS.with(std::cell::Cell::get), 1);
+    assert_eq!(withheld, may_days(&[6, 8, 12]));
+
+    let twin = holed_may(false);
+    let inputs = crate::load_audit_inputs(
+        &twin.root,
+        Vendor::Zerodha,
+        "NIFTY",
+        "5min",
+        span,
+        "generated-daily-context-hoist-fixture",
+    )
+    .expect("the census path");
+    assert_eq!(inputs.withheld_days, withheld);
+    assert_eq!(column.census(), inputs.column.census());
+    assert_eq!(
+        digest,
+        crate::minute_gaps::bind_withheld(
+            crate::stored_anchored_digest(&inputs.folded, &inputs.exact_minute, &inputs.daily)
+                .expect("the census digest"),
+            &inputs.withheld_days,
+        )
+    );
+}
diff --git a/crates/cli/src/g18_cli_a_tests.rs b/crates/cli/src/g18_cli_a_tests.rs
index 6a1073b7..e81c4304 100644
--- a/crates/cli/src/g18_cli_a_tests.rs
+++ b/crates/cli/src/g18_cli_a_tests.rs
@@ -105,14 +105,19 @@ fn the_ledger_arms_exit_misused_on_words_they_do_not_understand() {
 }
 
 /// G18-cli-a-24: the stamped preparation never answers without a store.
-/// Unstamped, it refuses for the stamp; stamped, the missing root refuses.
+///
+/// Its subject was `column_withholding_unsourceable_days`, the stamping
+/// wrapper `one_rung`'s derived support alone called. D-4719 removed both the
+/// call and the wrapper, so the stamped build itself is asked: with an
+/// admitted build stamped, a missing root refuses, and nothing is built.
 #[test]
 fn a_stored_preparation_over_a_missing_root_refuses() {
     let folded = vec![candle(0, 2_500_000)];
     let mut days = Vec::new();
     let mut bars = folded.clone();
-    let refused = column_withholding_unsourceable_days(
-        std::path::Path::new("/nonexistent/brutex-g18-store"),
+    let root = std::path::Path::new("/nonexistent/brutex-g18-store");
+    let refused = column_withholding_at_build(
+        root,
         parse_vendor("zerodha").expect("feed"),
         "NIFTY",
         ((2025, 5), (2025, 5)),
@@ -122,13 +127,17 @@ fn a_stored_preparation_over_a_missing_root_refuses() {
             bars: &mut bars,
         },
         60_000_000,
-        "1min",
+        StoredPreparationBuild {
+            rung: "1min",
+            commit: Some("generated-g18-missing-root"),
+        },
     );
     assert!(refused.is_err(), "no store, no column");
-    let why = refused.err().unwrap_or_default();
-    if commit_stamp().is_none() {
-        assert!(why.contains("no verified commit stamp"), "{why}");
-    }
+    assert!(
+        days.is_empty(),
+        "no day was withheld from a store never read"
+    );
+    assert!(!root.exists(), "and none was created");
 }
 
 /// G18-cli-a-25: the anchored digest is a function of the signal: equal
diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index 16ff35ee..3d463cd0 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -972,58 +972,6 @@ fn exact_minute_withholding_unsourceable_days(
     ))
 }
 
-/// Builds the anchored column, WITHHOLDING each day whose close cannot be
-/// sourced.
-///
-/// # Where the refusal actually lives
-///
-/// `MissingClosingMinute` is raised by `overlay_exact_minute_orb_and_gapfib` inside
-/// [`stored_anchored_column`] — not by `load_exact_minute_context`. The overlay
-/// loads successfully; it is the COLUMN BUILD that finds a signal bar whose
-/// close has no matching stored minute. Two fixes wrapped the load and changed
-/// nothing, because the load had already succeeded.
-///
-/// # What it does
-///
-/// Build; on a refusal that names a minute, withhold that IST day from the
-/// signal bars and rebuild — overlay and daily context included, because both
-/// are keyed to the surviving bars and reusing them would describe a span the
-/// column no longer has. Each pass removes at least one day, so it terminates.
-///
-/// It DECLINES rather than substitutes: the tests forbidding minute
-/// substitution still hold, and a hole still refuses when its day is swept.
-/// What changes is that the day is not swept, and every withheld day is emitted
-/// as telemetry so a smaller sample is never a silent one.
-fn column_withholding_unsourceable_days(
-    root: &std::path::Path,
-    vendor: brutex_core::vendor::Vendor,
-    underlying: &str,
-    span: ((u16, u8), (u16, u8)),
-    series: FoldedSeries<'_>,
-    signal_length: i64,
-    // `&str`, NOT `&'static str`. `audit_range_inner` takes its rung from the
-    // command line, so it is borrowed rather than one of `EVERY_RUNG`'s
-    // literals — and this only ever reads it to label an event.
-    rung: &str,
-) -> Result<(indicators::column::Column, [u8; 32]), String> {
-    let commit = commit_stamp().ok_or_else(|| {
-        "the build has no verified commit stamp; no stored condition preparation will run"
-            .to_owned()
-    })?;
-    column_withholding_at_build(
-        root,
-        vendor,
-        underlying,
-        span,
-        series,
-        signal_length,
-        StoredPreparationBuild {
-            rung,
-            commit: Some(commit),
-        },
-    )
-}
-
 #[derive(Clone, Copy)]
 struct StoredPreparationBuild<'a> {
     rung: &'a str,
@@ -1046,6 +994,30 @@ struct FoldedSeries<'a> {
     bars: &'a mut Vec<indicators::Candle>,
 }
 
+/// Builds the anchored column, WITHHOLDING each day whose close cannot be
+/// sourced.
+///
+/// # Where the refusal actually lives
+///
+/// `MissingClosingMinute` is raised by `overlay_exact_minute_orb_and_gapfib` inside
+/// [`stored_anchored_column`] — not by `load_exact_minute_context`. The overlay
+/// loads successfully; it is the COLUMN BUILD that finds a signal bar whose
+/// close has no matching stored minute. Two fixes wrapped the load and changed
+/// nothing, because the load had already succeeded.
+///
+/// # What it does
+///
+/// Build; on a refusal that names a minute, withhold that IST day from the
+/// signal bars and rebuild — the overlay context included, because it is keyed
+/// to the surviving bars and reusing it would describe a span the column no
+/// longer has. The daily context is derived from the whole folded series,
+/// which no pass changes, so it is read once (D-1781, D-4719). Each pass
+/// removes at least one day, so it terminates.
+///
+/// It DECLINES rather than substitutes: the tests forbidding minute
+/// substitution still hold, and a hole still refuses when its day is swept.
+/// What changes is that the day is not swept, and every withheld day is emitted
+/// as telemetry so a smaller sample is never a silent one.
 fn column_withholding_at_build(
     root: &std::path::Path,
     vendor: brutex_core::vendor::Vendor,
@@ -1068,13 +1040,15 @@ fn column_withholding_at_build(
     // ONCE, outside the retry loop: the verdict is a property of the key and
     // does not change when a day is withheld.
     let availability = stored::vwap_availability(&stored::swept_index(underlying)?);
+    // THE DAILY CONTEXT ANCHORS EVERY BAR THE FOLD STEPS, so it is derived
+    // from the whole series (D-1781), which no pass changes: read ONCE, not
+    // once per pass (W2-cli8-6, D-4719). The overlay context is derived from
+    // the swept bars it overlays, which a withheld day changes, so it is
+    // still read per pass.
+    let daily = stored::load_daily_context(root, vendor, underlying, (from, to), whole)?;
     for _ in 0..ATTEMPTS {
         #[cfg(test)]
         COLUMN_BUILD_ATTEMPTS.with(|count| count.set(count.get().saturating_add(1)));
-        // THE DAILY CONTEXT ANCHORS EVERY BAR THE FOLD STEPS, so it is derived
-        // from the whole series; the overlay context from the swept bars it
-        // overlays. D-1781.
-        let daily = stored::load_daily_context(root, vendor, underlying, (from, to), whole)?;
         let exact = stored::load_exact_minute_context(root, vendor, underlying, (from, to), bars)?;
         let digest = crate::minute_gaps::bind_withheld(
             stored_anchored_digest(whole, &exact, &daily)?,
@@ -7547,6 +7521,9 @@ struct AuditInputs {
     /// Every day withheld from the sweep: the holed days, then any day the
     /// overlay could not source.
     withheld_days: Vec<i64>,
+    /// The digest the column's preparation attempt was recorded under, which
+    /// `one_rung`'s derived support names its probes by. D-4719.
+    preparation_digest: [u8; 32],
 }
 
 /// The inputs of the last stored range audit, or its refusal, and the raw
@@ -7600,6 +7577,9 @@ std::thread_local! {
     static AUDIT_INPUT_LOADS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
     /// Test-only: how many times this thread loaded `one_rung`'s raw span. D-1557.
     static RUNG_SPAN_LOADS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
+    /// Test-only: the swept count `one_rung`'s derived support was sized on,
+    /// so a test can compare it with the audit's column. W2-cli8-6, D-4719.
+    static AUTO_SUPPORT_SWEPT: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
 }
 
 /// [`audit_range_kernel_cached`] with a fresh cache: one audit, one load.
@@ -7724,11 +7704,11 @@ fn load_audit_inputs(
     let mut withheld_days = holed_days;
     // THE COLUMN BUILD IS WHAT REFUSES, so the withholding wraps THAT.
     //
-    // `one_rung` guards its own support-derivation build, and this is the
-    // second build on the same span -- one hop later, unguarded, raising the
-    // identical `MissingClosingMinute`. Guarding only the first left the
-    // symptom exactly as it was, which is how a correct fix looked like no fix
-    // at all.
+    // `one_rung` once guarded only its own support-derivation build, and this
+    // was the second build on the same span -- one hop later, unguarded,
+    // raising the identical `MissingClosingMinute`. Guarding only the first
+    // left the symptom exactly as it was, which is how a correct fix looked
+    // like no fix at all. Since D-4719 that derivation reads THIS build.
     let (column, preparation_digest) = column_withholding_at_build(
         root,
         vendor,
@@ -7796,6 +7776,7 @@ fn load_audit_inputs(
         executed_digest,
         folded,
         withheld_days,
+        preparation_digest,
     })
 }
 
@@ -7835,6 +7816,7 @@ fn audit_range_kernel_cached(
         executed_digest,
         folded,
         withheld_days,
+        preparation_digest: _,
     } = cache.inputs(
         AuditKey {
             root: root.clone(),
@@ -15481,65 +15463,48 @@ fn one_rung_cached(ask: RungAsk<'_>, store: RungStore, cache: &mut AuditCache) -
         };
         (min_hits_for_swept(can_hit, ppm), can_hit)
     } else {
-        // A NAMED SUPPORT READS ONLY THE THREE FACTS ABOVE, so a descent's held
-        // span is never copied; the derivation withholds days from its own
-        // copy, as it always withheld them from its own load. D-1557.
-        let mut span = span.clone();
-        // THE DAILY CONTEXT AND THE OVERLAY ARE LOADED INSIDE
-        // `column_withholding_unsourceable_days`, not here.
-        //
-        // Both are keyed to the SURVIVING bars, so a day withheld between
-        // attempts changes both. Loading them once out here and reusing them
-        // across rebuilds would describe a span the column no longer has — and
-        // would pay for a full minute-series load twice besides.
-        let signal_length = match stored::rung_length_micros(rung) {
-            Ok(length) => length,
-            Err(why) => {
-                return RungRow {
-                    rung,
-                    outcome: Err(first_line(why)),
-                    missing: Vec::new(),
-                    excluded: stored::CalendarExclusion::none(),
-                    retention: None,
-                    validation: None,
-                };
-            }
-        };
-        // THE REFUSAL IS HERE, NOT AT THE LOAD, and that distinction cost two
-        // wrong fixes.
-        //
-        // `MissingClosingMinute` is raised by `overlay_exact_minute_orb_and_gapfib`
-        // INSIDE `stored_anchored_column` — the overlay LOADS fine and the
-        // column build is what cannot source a signal bar's close. Withholding
-        // around `load_exact_minute_context` therefore changed nothing: that
-        // call had already succeeded.
+        // THE AUDIT'S OWN PREPARATION, read through the cache
+        // `audit_range_cached` consults next, exactly as the named branch above
+        // reads it (W2-cli8-6, G2-3, D-4719).
         //
-        // MEASURED: one absent minute refused 15min, 10min, 5min, 3min, 2min and
-        // 1min in under half a second each, on every run today. 60min survived
-        // only because no 60-minute bar happened to close on that minute.
+        // This branch built its own column from the raw span and an EMPTY
+        // withheld set. So it ran no minute-hole census: an edge-holed day
+        // cost a refused pass, a reloaded context and a durable preparation
+        // attempt under a digest the audit never used, and a day whose only
+        // hole is one no close demands stayed in the column while the audit
+        // withheld it. `min_hits` was then sized on a swept population the
+        // audit did not sweep, and the column was built twice. Now the census,
+        // the withholding, the one column and its preparation digest are the
+        // audit's, built once.
         //
-        // So the day the refusal NAMES is withheld and the column rebuilt. The
-        // overlay and the daily context are rebuilt too, because both are keyed
-        // to the surviving bars — reusing them would describe a span the column
-        // no longer has.
-        // FOLDED WHOLE: a day withheld below leaves the sweep, not the fold.
-        // D-1781.
-        let folded = span.bars.clone();
-        let mut withheld_days: Vec<i64> = Vec::new();
-        let (column, digest) = match column_withholding_unsourceable_days(
-            &root,
-            vendor,
-            underlying,
-            (from, to),
-            FoldedSeries {
-                folded: &folded,
-                days: &mut withheld_days,
-                bars: &mut span.bars,
+        // UNSTAMPED, IT PREPARES NOTHING, as the named branch keeps an
+        // unstamped rung from writing an attempt the audit would not have
+        // written. It took its stamp from the binary, so a store the audit
+        // was about to refuse got a column, and evidence, first.
+        let Some(commit) = store.commit else {
+            return RungRow {
+                rung,
+                outcome: Err("the build has no verified commit stamp; no stored \
+                              condition preparation will run"
+                    .to_owned()),
+                missing: Vec::new(),
+                excluded: stored::CalendarExclusion::none(),
+                retention: None,
+                validation: None,
+            };
+        };
+        let inputs = match cache.inputs(
+            AuditKey {
+                root: root.clone(),
+                vendor,
+                underlying: underlying.to_owned(),
+                rung: rung.to_owned(),
+                span: (from, to),
+                commit: commit.to_owned(),
             },
-            signal_length,
-            rung,
+            || load_audit_inputs(&root, vendor, underlying, rung, (from, to), commit),
         ) {
-            Ok(column) => column,
+            Ok(inputs) => inputs,
             Err(why) => {
                 return RungRow {
                     rung,
@@ -15551,9 +15516,16 @@ fn one_rung_cached(ask: RungAsk<'_>, store: RungStore, cache: &mut AuditCache) -
                 };
             }
         };
-        let can_hit = column.census().swept;
+        let can_hit = inputs.column.census().swept;
+        #[cfg(test)]
+        AUTO_SUPPORT_SWEPT.with(|swept| swept.set(Some(can_hit)));
         let statistical = min_hits_for_swept(can_hit, statistical_support_floor(can_hit));
-        match affordable_min_hits(&column, &root, &span, digest) {
+        match affordable_min_hits(
+            &inputs.column,
+            &root,
+            &inputs.span,
+            inputs.preparation_digest,
+        ) {
             Ok(affordable) => (affordable.max(statistical), can_hit),
             Err(why) => {
                 return RungRow {
@@ -15567,7 +15539,6 @@ fn one_rung_cached(ask: RungAsk<'_>, store: RungStore, cache: &mut AuditCache) -
             }
         }
     };
-
     // ENTERING THE SWEEP IS ALSO AN EVENT, AND THE SILENCE BELOW IT IS THE LONG
     // ONE.
     //
diff --git a/crates/cli/src/stored.rs b/crates/cli/src/stored.rs
index 06143830..ccd0a2da 100644
--- a/crates/cli/src/stored.rs
+++ b/crates/cli/src/stored.rs
@@ -3427,12 +3427,23 @@ pub fn load_daily_context(
     signal_months: ((u16, u8), (u16, u8)),
     signal: &[Candle],
 ) -> Result<DailyContext, Refusal> {
+    #[cfg(test)]
+    DAILY_CONTEXT_LOADS.with(|loads| loads.set(loads.get().saturating_add(1)));
     let (from, to) = signal_months;
     let warm_from = previous_month(from)?;
     let daily = load_span(root, vendor, underlying, "1day", warm_from, to)?;
     daily_context_from_span(daily, signal)
 }
 
+#[cfg(test)]
+std::thread_local! {
+    /// Test-only: calls of [`load_daily_context`] on this thread, so a test can
+    /// prove a column build's retry loop reads the daily context once
+    /// (W2-cli8-6, D-4719).
+    pub(crate) static DAILY_CONTEXT_LOADS: std::cell::Cell<u64> =
+        const { std::cell::Cell::new(0) };
+}
+
 /// Load stored one-day reference evidence under a pre-allocation record ceiling.
 ///
 /// This is the bounded production counterpart to [`load_daily_context`].  It
diff --git a/crates/cli/tests/limits_o1cli_2.rs b/crates/cli/tests/limits_o1cli_2.rs
index e898e229..e9cfb96f 100644
--- a/crates/cli/tests/limits_o1cli_2.rs
+++ b/crates/cli/tests/limits_o1cli_2.rs
@@ -1,12 +1,12 @@
 //! The all-rungs support probe's second load and build, stated where it is
 //! paid (audit o1cli-2).
 //!
-//! `one_rung` loads a rung's span and, when no support is named, builds its
-//! column to size the affordable floor; the audit kernel then loads
-//! and builds the same span again. Only a code comment admitted it. This file
-//! holds `docs/06-limits.md` to the code: the section must state both costs,
-//! and the calls it describes must still be where it says, so the day the
-//! span is threaded through, this fails and the limit is withdrawn with it.
+//! `one_rung` loads a rung's span; the audit kernel then loads the same span
+//! again. Until D-4719 a derived support also built its own column first, and
+//! the kernel built it again. This file holds `docs/06-limits.md` to the code:
+//! the section must state the costs, and the calls it describes must still be
+//! where it says, so the day the span is threaded through, this fails and the
+//! limit is withdrawn with it.
 
 #![allow(
     clippy::expect_used,
@@ -49,17 +49,17 @@ fn body(head: &str) -> &'static str {
         .expect("its body")
 }
 
-/// THE SECOND LOAD AND BUILD PER RUNG ARE STATED, AND STILL TRUE.
+/// THE SECOND LOAD PER RUNG IS STATED, AND STILL TRUE; THE SECOND BUILD IS
+/// GONE (D-4719).
 #[test]
 fn a_rungs_second_load_and_build_are_stated_and_still_paid() {
-    let limit =
-        limit("## A rung loads its span twice and may build its column twice (audit o1cli-2)");
+    let limit = limit("## A rung loads its span twice and builds its column once (audit o1cli-2)");
     for sentence in [
         "`one_rung` loads the rung's span with `stored::load_span`",
         "the audit kernel `audit_range_kernel` then loads the same span again",
-        "When no support is named, the column is also built twice",
-        "`column_withholding_unsourceable_days` for `affordable_min_hits`, then `column_withholding_at_build`",
-        "two span loads per rung always, and two column builds",
+        "The column is built once per rung whether or not a support is named",
+        "both branches of `one_rung_cached` read `load_audit_inputs` through the `AuditCache` the audit then reads",
+        "two span loads per rung always, and one column build",
         "O(rung bars) each",
     ] {
         assert!(
@@ -71,7 +71,6 @@ fn a_rungs_second_load_and_build_are_stated_and_still_paid() {
     let rung = body("\nfn one_rung_cached(");
     for call in [
         "stored::load_span(",
-        "column_withholding_unsourceable_days(",
         "affordable_min_hits(",
         "audit_range_cached(",
     ] {
@@ -80,6 +79,17 @@ fn a_rungs_second_load_and_build_are_stated_and_still_paid() {
             "`one_rung` no longer calls {call}: update the limit"
         );
     }
+    // D-4719: both support branches prepare through the audit's own loader,
+    // and the derived branch's private build is gone.
+    assert_eq!(
+        rung.matches("load_audit_inputs(").count(),
+        2,
+        "each support branch reads the audit's inputs: update the limit"
+    );
+    assert!(
+        !LIB.contains("fn column_withholding_unsourceable_days("),
+        "a second column build is back: update the limit"
+    );
     assert!(
         rung.find("stored::load_span(") < rung.find("named_ppm.is_some()"),
         "the span is loaded before the named-support branch, so even a named support pays the first load"
diff --git a/crates/cli/tests/limits_o1cli_3.rs b/crates/cli/tests/limits_o1cli_3.rs
index 27afc401..1a6165ef 100644
--- a/crates/cli/tests/limits_o1cli_3.rs
+++ b/crates/cli/tests/limits_o1cli_3.rs
@@ -58,10 +58,10 @@ fn the_parallel_rungs_repeated_minute_reads_are_stated_and_still_paid() {
     for sentence in [
         "`sweep_rungs` runs every rung through `one_rung`, one rung at a time in input order",
         "the execution series `audit_range_kernel` loads",
-        "one per attempt of every column build",
+        "one per attempt of the column build",
         "one per attempt of `exact_minute_withholding_unsourceable_days`",
-        "at least three reads of that rung's one-minute span, four when the support is derived",
-        "some 24 to 32 reads of identical minutes per command",
+        "at least three reads of that rung's one-minute span, derived support or named",
+        "some 24 reads of identical minutes per command",
         "up to 64 attempts",
     ] {
         assert!(
@@ -111,5 +111,8 @@ fn the_parallel_rungs_repeated_minute_reads_are_stated_and_still_paid() {
             "the kernel no longer calls {call}: update the limit"
         );
     }
-    assert!(body("\nfn one_rung_cached(").contains("column_withholding_unsourceable_days("));
+    // D-4719: a derived support reads the kernel's own build, so it adds no
+    // read of its own.
+    assert!(body("\nfn one_rung_cached(").contains("load_audit_inputs("));
+    assert!(!LIB.contains("fn column_withholding_unsourceable_days("));
 }
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 817c04ed..59e23332 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7061,3 +7061,7 @@ old line regex the same input and watched it pass.
 | L1FB-02 | A recorded tier walk that meets a tier captures that tier alone, with its own rules, and answers as the uncaptured cached walk (D-4716) | `cli::screen_policy_tests::a_recorded_walk_captures_only_the_tier_it_ends_on` | ✓ |
 | L1FB-03 | A zero-point `screen` reaches `cli::screen_range_in_points` as no ceiling and runs over a swept index, from the api command end to end and from the cli door; the elite door agrees (D-4717) | `api::sweeprun::tests::a_zero_point_screen_command_runs_as_no_ceiling_end_to_end`, `cli::audited_stored::tests::the_points_screen_reads_zero_as_no_ceiling_and_shares_the_support_domain` | ✓ |
 | L1FB-04 | `screen_range`, `screen_range_in_points` and the api's `screen` command admit exactly the support domain `1..1_000_000` that argv and `BRUTEX_SUPPORT_PPM` admit, refusing with its sentence before any ledger row or attempt is written (D-4718) | `cli::audited_stored::tests::the_points_screen_reads_zero_as_no_ceiling_and_shares_the_support_domain`, `api::sweeprun::tests::the_screen_command_refuses_the_support_domain_cli_refuses` | ✓ |
+| L1FB-05 | A derived-support rung over a span with edge-holed and interior-gap days builds one column through the audit's own inputs, sizes its support on the audit's swept count, and records `min_hits` equal to the larger of the two floors on the audit's column (D-4719) | `cli::audited_stored::tests::a_derived_support_rung_sizes_on_the_audits_census_and_builds_once` | ✓ |
+| L1FB-06 | A derived-support rung with no commit stamp builds nothing, loads no audit inputs and begins no attempt (D-4719) | `cli::audited_stored::tests::an_unstamped_derived_support_rung_prepares_nothing` | ✓ |
+| L1FB-07 | `column_withholding_at_build` reads the daily context once however many passes it retries, and its withholding, census and digest equal the census path's (D-4719) | `cli::audited_stored::tests::a_column_builds_retry_loop_reads_the_daily_context_once` | ✓ |
+| L1FB-08 | `docs/06-limits.md` states one column build per rung, derived support or named, and three minute reads per rung; AU-O1CLI-2's and AU-O1CLI-3's tests now hold that statement and fail if a second build returns (D-4719) | `a_rungs_second_load_and_build_are_stated_and_still_paid` in `crates/cli/tests/limits_o1cli_2.rs`, `the_parallel_rungs_repeated_minute_reads_are_stated_and_still_paid` in `crates/cli/tests/limits_o1cli_3.rs` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index e88c8180..cb4719e6 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65127,3 +65127,54 @@ with no ledger and no attempt written; before the fix the points door's zero
 refusal lacked the domain's sentence) and
 `api::sweeprun::tests::the_screen_command_refuses_the_support_domain_cli_refuses`
 (0, 1,000,000, 1,000,001 and `u64::MAX` refused, 1 and 999,999 parsed).
+
+### D-4719 — A derived-support rung prepares through the audit's own inputs, and a build reads its daily context once — 2026-10-09
+
+**Finding.** W2-cli8-6 and G2-3. D-1662 ran the minute-hole census at every
+stored door but one: `one_rung_cached`'s derived-support branch built its own
+column through `column_withholding_unsourceable_days` from the raw span and an
+EMPTY withheld set. An edge-holed day cost a refused pass, a reloaded context
+and a durable preparation attempt under a digest the audit never used; a day
+whose only hole no close demands stayed in that column while the audit
+withheld it, so `min_hits` was sized on a population the audit did not sweep.
+The branch also stamped its preparation with the binary's stamp rather than
+the rung's, so a rung the audit was about to refuse as unstamped built and
+recorded a column first. Separately, `column_withholding_at_build` reloaded
+the daily context on every one of up to 64 passes although it is derived from
+the whole folded series (D-1781), which no pass changes.
+
+**Decision.** The derived branch reads `cache.inputs(.., load_audit_inputs)`,
+as the named branch does (D-1557): one census, one withholding, one column,
+one preparation attempt, shared with the audit that follows. `can_hit` is
+that column's swept count and `affordable_min_hits` names its probes by the
+preparation digest, now held as `AuditInputs::preparation_digest`. With no
+commit on the rung it refuses before any load, with the sentence the wrapper
+gave. `column_withholding_unsourceable_days` lost its only caller and is
+removed; its doc moved onto `column_withholding_at_build`, and G18-cli-a-24's
+test now asks that build with an admitted stamp. The daily context is read
+once, above the retry loop; the overlay context is still read per pass,
+because a withheld day changes the bars it overlays.
+
+**What changes in stored results.** On a span with no census-withheld day,
+nothing: the withheld set was empty both ways, so the column, digest,
+`AutoSearch` identities and `min_hits` are byte-identical. On a span the
+census withholds from, a derived-support rung's `AutoSearch` identities move
+with their `data_digest` (the audit's withheld set), its `min_hits` may move
+with the swept count, and so may its row; the rung no longer writes its own
+preparation attempts. Rows already recorded keep their identities.
+
+**Limits restated.** AU-O1CLI-2 and AU-O1CLI-3 described the second build as
+current. Their `docs/06-limits.md` sections now state one column build per
+rung and three minute reads per rung, derived or named; their tests are
+updated to hold the new statement, and the heading of o1cli-2 now reads
+"builds its column once".
+
+**Proof.** `a_derived_support_rung_sizes_on_the_audits_census_and_builds_once`
+(three edge-holed days and one interior gap: one build where the unfixed code
+made five, one input load, the sized population equal to the audit's swept
+count, `min_hits` equal to the larger of the floors on the audit's column),
+`an_unstamped_derived_support_rung_prepares_nothing` (no build, no load, no
+attempt; before, four builds) and
+`a_column_builds_retry_loop_reads_the_daily_context_once` (four passes, one
+daily load where there were four, and the same withholding, census and
+digest as the census path).
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 7bb7a1c7..8374bb74 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -11638,14 +11638,16 @@ the counts the tests assert: no bench times the fold.
   input order (in parallel until D-1701), and each rung reads the same
   one-minute span for itself.** For a rung other than
   `1min` the reads are the execution series `audit_range_kernel` loads, one
-  per attempt of every column build (inside `load_exact_minute_context`: the
-  kernel's build, and `one_rung`'s own when the support is derived), and one
-  per attempt of `exact_minute_withholding_unsourceable_days`. That is at
-  least three reads of that rung's one-minute span, four when the support is
-  derived, and the `1min` rung reads it as its own span as well. Across the
-  eight rungs that is some 24 to 32 reads of identical minutes per command,
-  O(minute bars) each, and more when withheld days force a rebuild: each
-  build retries up to 64 attempts and every attempt reads the minutes again.
+  per attempt of the column build (inside `load_exact_minute_context`: the
+  kernel's build, which a derived support shares since D-4719), and one per
+  attempt of `exact_minute_withholding_unsourceable_days`. That is at least
+  three reads of that rung's one-minute span, derived support or named (a
+  derived support paid a fourth until D-4719), and the `1min` rung reads it
+  as its own span as well. Across the eight rungs that is some 24 reads of
+  identical minutes per command, O(minute bars) each, and more when withheld
+  days force a rebuild: each build retries up to 64 attempts and every
+  attempt reads the minutes again. The census withholds every holed day it
+  can see before the first attempt, so a rebuild needs a hole it cannot.
   Loading the minute span once per command and sharing it is possible, since
   the read itself does not depend on the rung, and is not done: each context
   is derived from that rung's surviving bars and digested into its
@@ -11654,25 +11656,26 @@ the counts the tests assert: no bench times the fold.
   `the_parallel_rungs_repeated_minute_reads_are_stated_and_still_paid` in
   `crates/cli/tests/limits_o1cli_3.rs`.
 
-## A rung loads its span twice and may build its column twice (audit o1cli-2)
+## A rung loads its span twice and builds its column once (audit o1cli-2)
 
 - **`one_rung` loads the rung's span with `stored::load_span`, and the audit
-  kernel `audit_range_kernel` then loads the same span again.** When no
-  support is named, the column is also built twice:
-  `column_withholding_unsourceable_days` for `affordable_min_hits`, then
-  `column_withholding_at_build` inside the kernel, each loading its own daily
-  and exact-minute context. So an all-rungs run pays two span loads per rung
-  always, and two column builds per rung when the support is derived, O(rung
-  bars) each. Until this audit only a comment in `stored.rs` admitted it.
-  Threading the loaded span and column into the kernel would remove the
-  second pair; it is not done because the kernel re-derives both from the
-  bars that survive its own withholding and binds them to the preparation
-  digest. Since D-1557 both loads go through one `AuditCache` per command
-  (`one_rung_cached`, and `load_audit_inputs` for the kernel), so a `descend`
-  pays the pair once for its whole ladder, not once per step; a single rung
-  still pays both. Stated from the code's shape; not timed. Held to the code
-  by `a_rungs_second_load_and_build_are_stated_and_still_paid` in
-  `crates/cli/tests/limits_o1cli_2.rs`.
+  kernel `audit_range_kernel` then loads the same span again.** The column is
+  built once per rung whether or not a support is named: both branches of
+  `one_rung_cached` read `load_audit_inputs` through the `AuditCache` the
+  audit then reads, so a derived support sizes `affordable_min_hits` on the
+  audit's own column, census and preparation digest. So an all-rungs run pays
+  two span loads per rung always, and one column build, O(rung bars) each.
+  Until D-4719 a derived support built its own column first with
+  `column_withholding_unsourceable_days`, from an empty withheld set and with
+  no minute-hole census, and the kernel built it again. Reading the row's
+  missing months and calendar exclusions off the kernel's load would remove
+  the first load; it is not done because an unstamped rung reports them
+  without preparing anything, and the kernel's load is the one its
+  preparation digest is bound to. Since D-1557 both loads go
+  through one `AuditCache` per command, so a `descend` pays them once for its
+  whole ladder, not once per step. Stated from the code's shape; not timed.
+  Held to the code by `a_rungs_second_load_and_build_are_stated_and_still_paid`
+  in `crates/cli/tests/limits_o1cli_2.rs`.
 
 ## Condition names resolve through a compile-time index (audit o1engine-24)
 
-- 
2.43.0


From c8691a3c08d89f21dd3b03c52cf4e2297f0ab215 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:29:26 +0000
Subject: [PATCH 4/8] cli: pin one results-ledger open per root for rung
 readback (W2-cli8-4, D-4720)

Before: no test counted recorded_row's ledger opens, so a fresh
Results::open per rung passed every test.

After: a child-process test reads three rungs back by identity and
requires results::OPENS == 1; with a fresh open per rung it fails 3 != 1.
No production code changes. Gate 23 declares the child's stdout
proof line (results_report_tests.rs 1).

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 .github/workflows/ci.yml               | 13 ++---
 crates/cli/src/results_report_tests.rs | 72 ++++++++++++++++++++++++++
 docs/04-invariants.md                  |  1 +
 docs/05-decisions.md                   | 12 +++++
 4 files changed, 92 insertions(+), 6 deletions(-)

diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 7088ccca..2009f92a 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -1760,12 +1760,13 @@ jobs:
           # was degraded, so a clean run prints nothing.
           declared="$declared pull/src/csv.rs:eprintln!:1
           pull/src/masters.rs:eprintln!:1"
-          # TWO MORE STDOUT PROOF LINES, D-0695's kind (D-4717, D-4718): api
-          # sweeprun's second child prints `ZERO-POINT SCREEN RAN` (counted in
-          # its line above), and a cli child prints `POINTS SCREEN DOORS
-          # CHECKED`. Each sits in a `#[cfg(test)]` module and prints only
-          # after its child passed.
-          declared="$declared cli/src/audited_stored_tests.rs:println!:1"
+          # THREE MORE STDOUT PROOF LINES, D-0695's kind (D-4717, D-4718,
+          # D-4720): api sweeprun's second child prints `ZERO-POINT SCREEN
+          # RAN` (counted in its line above), and two cli children print
+          # `POINTS SCREEN DOORS CHECKED` and `READBACK OPENS`. Each sits in a
+          # `#[cfg(test)]` module and prints only after its child passed.
+          declared="$declared cli/src/audited_stored_tests.rs:println!:1
+          cli/src/results_report_tests.rs:println!:1"
 
           # CLAUSE A2 — the same question, asked of the handle spellings.
           #
diff --git a/crates/cli/src/results_report_tests.rs b/crates/cli/src/results_report_tests.rs
index 42d60b37..96507fd4 100644
--- a/crates/cli/src/results_report_tests.rs
+++ b/crates/cli/src/results_report_tests.rs
@@ -583,3 +583,75 @@ fn the_omitted_row_line_starts_one_row_past_the_listing() -> Result<(), Box<dyn
     }
     Ok(())
 }
+
+/// **A range rung reads its row back through one held ledger handle.**
+/// W2-cli8-4, D-4720.
+///
+/// `recorded_row` replaced `latest_for` (D-1700): one identity probe of the
+/// process's shared ledger handle, opened once per root. Its two tests prove
+/// it reads the RIGHT row, and both pass on a `recorded_row` that opened the
+/// ledger afresh for every rung, an O(runs) identity pass per rung, which is
+/// the cost D-1700 removed. Three rungs over one root now open the ledger
+/// once. In a child process: the handle is process-wide, and any other test
+/// using another root between two calls here would evict it.
+#[test]
+fn a_rungs_readback_opens_the_ledger_once_per_root_not_once_per_rung()
+-> Result<(), Box<dyn std::error::Error>> {
+    const CHILD: &str = "BRUTEX_TEST_RUNG_READBACK_OPENS";
+    if std::env::var_os(CHILD).is_some() {
+        return readback_opens_child();
+    }
+    let output = std::process::Command::new(std::env::current_exe()?)
+        .args([
+            "--exact",
+            "results_report_tests::a_rungs_readback_opens_the_ledger_once_per_root_not_once_per_rung",
+            "--nocapture",
+            "--test-threads=1",
+        ])
+        .env(CHILD, "1")
+        .output()?;
+    let stdout = String::from_utf8_lossy(&output.stdout);
+    assert!(
+        output.status.success(),
+        "{stdout}{}",
+        String::from_utf8_lossy(&output.stderr)
+    );
+    assert!(stdout.contains("1 passed"), "{stdout}");
+    assert!(stdout.contains("READBACK OPENS 1 FOR 3 RUNGS"), "{stdout}");
+    Ok(())
+}
+
+/// The child half: three rows written through a private handle, then read
+/// back one rung at a time by identity, counting ledger opens.
+fn readback_opens_child() -> Result<(), Box<dyn std::error::Error>> {
+    let rows: Vec<Record> = (1..=3).map(|id| row(id, 100, 2)).collect();
+    let fixture = Fixture::new(&rows)?;
+    let (found, opens, _) = counted(|| {
+        rows.iter()
+            .map(|record| {
+                let page = format!(
+                    "{}\n  row 0 in x\n  {}{}\n",
+                    crate::RECORDED_HEAD,
+                    crate::RECORDED_IDENTITY,
+                    record.identity_hex()
+                );
+                crate::recorded_row(
+                    &fixture.0,
+                    &page,
+                    crate::RungKey {
+                        feed: "zerodha",
+                        underlying: "NIFTY",
+                        rung: "15min",
+                        from: (2026, 1),
+                        to: (2026, 1),
+                        min_hits: record.min_hits,
+                    },
+                )
+            })
+            .collect::<Result<Vec<_>, String>>()
+    });
+    assert_eq!(found?, rows, "each rung reads its own row");
+    assert_eq!(opens, 1, "one ledger open for every rung of the root");
+    println!("READBACK OPENS {opens} FOR {} RUNGS", rows.len());
+    Ok(())
+}
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 59e23332..a21dad60 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7065,3 +7065,4 @@ old line regex the same input and watched it pass.
 | L1FB-06 | A derived-support rung with no commit stamp builds nothing, loads no audit inputs and begins no attempt (D-4719) | `cli::audited_stored::tests::an_unstamped_derived_support_rung_prepares_nothing` | ✓ |
 | L1FB-07 | `column_withholding_at_build` reads the daily context once however many passes it retries, and its withholding, census and digest equal the census path's (D-4719) | `cli::audited_stored::tests::a_column_builds_retry_loop_reads_the_daily_context_once` | ✓ |
 | L1FB-08 | `docs/06-limits.md` states one column build per rung, derived support or named, and three minute reads per rung; AU-O1CLI-2's and AU-O1CLI-3's tests now hold that statement and fail if a second build returns (D-4719) | `a_rungs_second_load_and_build_are_stated_and_still_paid` in `crates/cli/tests/limits_o1cli_2.rs`, `the_parallel_rungs_repeated_minute_reads_are_stated_and_still_paid` in `crates/cli/tests/limits_o1cli_3.rs` | ✓ |
+| L1FB-09 | Reading three rungs' rows back by identity over one root opens the results ledger once (D-4720) | `cli::results_report_tests::a_rungs_readback_opens_the_ledger_once_per_root_not_once_per_rung` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index cb4719e6..8ce7a915 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65178,3 +65178,15 @@ attempt; before, four builds) and
 `a_column_builds_retry_loop_reads_the_daily_context_once` (four passes, one
 daily load where there were four, and the same withholding, census and
 digest as the census path).
+
+### D-4720 — A rung's readback opens the ledger once per root — 2026-10-09
+
+**Finding.** W2-cli8-4, test gap. D-1700's `recorded_row` reads a rung's row
+through the shared ledger handle, but no test counted opens, so a fresh
+`Results::open(root)?.of_identity(..)` per rung passed every test.
+
+**Decision.** `a_rungs_readback_opens_the_ledger_once_per_root_not_once_per_rung`
+reads three rungs' rows by identity in a child process (the shared handle is
+process-global) and requires `results::OPENS == 1`. With the fresh open it
+failed `left: 3, right: 1`. No production code changes; the child's proof
+line is declared to gate 23 beside D-0695's.
-- 
2.43.0


From aadcc8a4811ea1c5e4f0b1dcbe75978e66f06fe8 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:29:27 +0000
Subject: [PATCH 5/8] cli: pin the results listing's forty-row retention
 (W2-cli8-5, D-4721)

Before: SCB-10's test only checked that results_at lacked a string, so
deleting ListingFold's pop_front passed everything.

After: the test folds 500 rows and checks the window never exceeds
LIST_ROWS and ends holding exactly the newest 40 matching rows at its
starting capacity; without pop_front it fails at row 50. docs/06 now
says the pass is in append order since D-2310.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/screen_policy_tests.rs | 38 +++++++++++++++++++++++++++
 docs/04-invariants.md                 |  1 +
 docs/05-decisions.md                  | 12 +++++++++
 docs/06-limits.md                     |  9 ++++---
 4 files changed, 57 insertions(+), 3 deletions(-)

diff --git a/crates/cli/src/screen_policy_tests.rs b/crates/cli/src/screen_policy_tests.rs
index f899908b..f908e95a 100644
--- a/crates/cli/src/screen_policy_tests.rs
+++ b/crates/cli/src/screen_policy_tests.rs
@@ -739,6 +739,14 @@ fn the_frontier_prefix_equals_the_full_sort_with_each_key_once() {
     assert!(first_accepted_in_order(&items, 2, |item| item.0, |_| false).is_empty());
 }
 /// W2-cli8-5. The listing retains a bounded window, not every matching row.
+///
+/// D-4721: this asserted only that `results_at` lacks the text
+/// `rows.push(record)`, so deleting the window's `pop_front` passed it, and
+/// passed every other test too, because the table prints only `LIST_ROWS`
+/// rows whatever is held. It now drives the fold: over 500 rows, 400 of them
+/// matching, `ListingFold` holds exactly `LIST_ROWS` records, the newest
+/// matching ones in append order, in the capacity it started with; a row the
+/// filter refuses is counted and never held.
 #[test]
 fn the_results_listing_retains_a_bounded_window() {
     let listing = code_of("\nfn results_at(");
@@ -746,6 +754,36 @@ fn the_results_listing_retains_a_bounded_window() {
         !listing.contains("rows.push(record)"),
         "every matching record is retained: {listing}"
     );
+    let ordinal = |record: &crate::results::Record| {
+        u32::from_le_bytes([
+            record.identity[0],
+            record.identity[1],
+            record.identity[2],
+            record.identity[3],
+        ])
+    };
+    let mut fold = ListingFold::new(Some("zerodha"), None);
+    let capacity = fold.newest.capacity();
+    assert!(capacity >= LIST_ROWS, "pre-sized to the window");
+    for at in 0..500_u32 {
+        let mut record = crate::tests::record_for_naming();
+        record.identity = [0; 32];
+        record.identity[..4].copy_from_slice(&at.to_le_bytes());
+        if at % 5 == 4 {
+            record.feed = crate::results::field("dhan");
+        }
+        fold.visit(Ok(record));
+        assert!(fold.newest.len() <= LIST_ROWS, "row {at}: the window grew");
+    }
+    assert_eq!((fold.rows, fold.matching), (500, 400));
+    assert_eq!(fold.newest.len(), LIST_ROWS, "exactly the window is held");
+    assert_eq!(fold.newest.capacity(), capacity, "and it never reallocated");
+    let held: Vec<u32> = fold.newest.iter().map(ordinal).collect();
+    let newest: Vec<u32> = (0..500_u32)
+        .filter(|at| at % 5 != 4)
+        .skip(400 - LIST_ROWS)
+        .collect();
+    assert_eq!(held, newest, "the newest matching rows, in append order");
 }
 
 /// AC-whp-o1-2. The bootstrap family builds its slice facts once, not once
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index a21dad60..4dfe6ced 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7066,3 +7066,4 @@ old line regex the same input and watched it pass.
 | L1FB-07 | `column_withholding_at_build` reads the daily context once however many passes it retries, and its withholding, census and digest equal the census path's (D-4719) | `cli::audited_stored::tests::a_column_builds_retry_loop_reads_the_daily_context_once` | ✓ |
 | L1FB-08 | `docs/06-limits.md` states one column build per rung, derived support or named, and three minute reads per rung; AU-O1CLI-2's and AU-O1CLI-3's tests now hold that statement and fail if a second build returns (D-4719) | `a_rungs_second_load_and_build_are_stated_and_still_paid` in `crates/cli/tests/limits_o1cli_2.rs`, `the_parallel_rungs_repeated_minute_reads_are_stated_and_still_paid` in `crates/cli/tests/limits_o1cli_3.rs` | ✓ |
 | L1FB-09 | Reading three rungs' rows back by identity over one root opens the results ledger once (D-4720) | `cli::results_report_tests::a_rungs_readback_opens_the_ledger_once_per_root_not_once_per_rung` | ✓ |
+| L1FB-10 | The results listing holds at most `LIST_ROWS` rows at every step of its fold, and exactly the newest 40 matching rows at the end, at unchanged capacity (D-4721) | `cli::screen_policy_tests::the_results_listing_retains_a_bounded_window` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 8ce7a915..fe135561 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65190,3 +65190,15 @@ reads three rungs' rows by identity in a child process (the shared handle is
 process-global) and requires `results::OPENS == 1`. With the fresh open it
 failed `left: 3, right: 1`. No production code changes; the child's proof
 line is declared to gate 23 beside D-0695's.
+
+### D-4721 — The results listing's forty-row retention is pinned — 2026-10-09
+
+**Finding.** W2-cli8-5, test gap. Deleting `ListingFold::visit`'s `pop_front`
+passed every test: `results_table` prints only `take(LIST_ROWS)`. SCB-10's
+test asserted only that `results_at` lacks the text `rows.push(record)`.
+
+**Decision.** SCB-10's test, `the_results_listing_retains_a_bounded_window`,
+now folds 500 rows (100 filtered out) and checks at every step that at most
+`LIST_ROWS` are held, then that exactly the newest 40 matching rows are held
+in order at an unchanged capacity. With `pop_front` deleted it failed "row
+50: the window grew". No production code changes.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 8374bb74..937b4825 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -15609,9 +15609,12 @@ per-candidate primitive from `CLAUDE.md` §3 rule 4.
   map probe plus a Wilson bound.
 
 - **`cli results` and `cli top`, per request: `O(ledger rows)` reads, `O(1)`
-  retained** (W2-cli8-5, D-1729). `results_at` makes one newest-first pass
-  over every recorded run and keeps at most `LIST_ROWS` (40) records plus a
-  running best. `newest_complete` makes one pass and keeps one record. Neither
+  retained** (W2-cli8-5, D-1729). `results_at` makes one pass over every
+  recorded run, in the append order of the ledger open's own visit since
+  D-2310 (not newest first), and keeps at most `LIST_ROWS` (40) records, by
+  dropping the oldest held row before each push, plus a running best; the
+  retention is pinned by `the_results_listing_retains_a_bounded_window`
+  (D-4721). `newest_complete` makes one pass and keeps one record. Neither
   can stop early: the best complete run can be anywhere in the ledger. A
   per-request bound below the ledger would need a secondary index, which this
   append-only, path-is-the-index file does not keep. UNVERIFIED as a
-- 
2.43.0


From d3968d90ae457a455944faa417f546e22b59ea0a Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:29:28 +0000
Subject: [PATCH 6/8] cli: pin the measured band's parallelism (W2-cli8-7,
 D-4722)

Before: a sequential loop in measure_top gave the same answer as
par_iter_mut, so reverting the parallelism passed every test.

After: a test-only per-row hook and a two-thread rendezvous require two
rows to be measured at once, and the figures to equal a one-thread
pool's; with iter_mut the test fails.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/lib.rs                 |  17 +++
 crates/cli/src/screen_policy_tests.rs | 154 ++++++++++++++++++++++++++
 docs/04-invariants.md                 |   1 +
 docs/05-decisions.md                  |  13 +++
 4 files changed, 185 insertions(+)

diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index 3d463cd0..c00573fc 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -14540,9 +14540,15 @@ fn measure_top(
     // one the sequential loop computed, whatever the core count. The cost is
     // `O(band x (G + 7 x trades))` -- `G` one exit grid -- divided across cores,
     // and `band` is at most `8 x TOP_CEILING`.
+    #[cfg(test)]
+    let hook = MEASURE_ROW_HOOK.with(|hook| hook.borrow().clone());
     rows.par_iter_mut()
         .take(measured_band(rules.top))
         .for_each(|row| {
+            #[cfg(test)]
+            if let Some(hook) = &hook {
+                hook();
+            }
             // THE SIDE THE ROW WAS PRICED AT, NOT THE PROXY, and getting this wrong
             // was worse than opposite — it was cross-wired.
             //
@@ -14581,6 +14587,17 @@ fn measure_top(
         });
 }
 
+#[cfg(test)]
+std::thread_local! {
+    /// Test-only: called once per measured row of [`measure_top`]'s band, so a
+    /// test can prove two rows are measured at once (W2-cli8-7, D-4722). Read
+    /// on the thread that calls `measure_top`, so no other test's measurement
+    /// sees it.
+    pub(crate) static MEASURE_ROW_HOOK: std::cell::RefCell<
+        Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
+    > = const { std::cell::RefCell::new(None) };
+}
+
 /// The screen's ranked table: one row per printed combination, its side, its
 /// chosen exit's figures and the rule that refused it, then its conditions.
 fn screen_table(out: &mut String, rows: &[Screened<'_>], rules: Rules, reference: i64) {
diff --git a/crates/cli/src/screen_policy_tests.rs b/crates/cli/src/screen_policy_tests.rs
index f908e95a..41606705 100644
--- a/crates/cli/src/screen_policy_tests.rs
+++ b/crates/cli/src/screen_policy_tests.rs
@@ -1842,3 +1842,157 @@ fn a_recorded_walk_captures_only_the_tier_it_ends_on() {
 }
 
 // ---------------------------------------------------------------------------
+// W2-cli8-7 (D-4722): the measured band runs on more than one core at once.
+// ---------------------------------------------------------------------------
+
+/// Where two measured rows meet: the first to arrive waits, bounded, for a
+/// second; a second arriving while the first waits is the overlap.
+#[derive(Default)]
+struct Meeting {
+    state: std::sync::Mutex<(u32, bool, bool)>,
+    met: std::sync::Condvar,
+}
+
+impl Meeting {
+    /// One row arrives. `(waiting, overlapped, gave_up)`.
+    fn arrive(&self) {
+        let mut state = self.state.lock().expect("meeting lock");
+        if state.0 > 0 {
+            state.1 = true;
+            self.met.notify_all();
+            return;
+        }
+        if state.1 || state.2 {
+            return;
+        }
+        state.0 += 1;
+        let (mut state, waited) = self
+            .met
+            .wait_timeout_while(state, std::time::Duration::from_secs(20), |state| !state.1)
+            .expect("meeting wait");
+        state.0 -= 1;
+        if waited.timed_out() {
+            state.2 = true;
+        }
+    }
+
+    fn overlapped(&self) -> bool {
+        self.state.lock().expect("meeting lock").1
+    }
+}
+
+/// The band fixture `the_parallel_band_matches_a_sequential_measurement` uses:
+/// up to forty priced rows of the eight-session slice, Long side.
+fn band_rows<'a>(fixture: &'a Ranked, facts: &runner::trade::SliceFacts) -> Vec<Screened<'a>> {
+    let bars = &fixture.bars;
+    let column = &fixture.run.column;
+    let horizon = Horizon::DEFAULT;
+    let stops = stop_ladder_ppm(bars, horizon.as_bars() as usize);
+    let levels = grid::Levels {
+        rungs: grid_rungs(bars),
+        step_ppm: Some(grid_step_ppm(bars, horizon.as_bars() as usize)),
+        forced: None,
+        ratios: true,
+        stops_ppm: &stops,
+    };
+    fixture
+        .run
+        .ranked
+        .top
+        .iter()
+        .take(40)
+        .enumerate()
+        .filter_map(|(rank, scored)| {
+            let g = grid::evaluate_over(
+                bars,
+                column,
+                &scored.mask,
+                horizon,
+                side_of_direction(Direction::Long),
+                levels,
+                facts,
+            );
+            let cell = g.best().copied()?;
+            Some(Screened {
+                side: Direction::Long,
+                scored,
+                rank,
+                cell,
+                tightest: None,
+                admitted: false,
+                consistency: None,
+                steady: true,
+                calendar_unmeasured: false,
+            })
+        })
+        .collect()
+}
+
+/// W2-cli8-7, D-4722. THE BAND IS MEASURED IN PARALLEL, observed rather than
+/// read off the source. On a two-thread pool each measured row checks in at a
+/// meeting point; the first waits (bounded, 20 s) for a second, and a second
+/// arriving while the first still waits proves two rows were being measured
+/// at the same moment. A sequential loop cannot produce that: its first row
+/// waits alone, gives up, and every later row arrives with nobody waiting.
+/// The figures are still the sequential ones, row for row.
+#[test]
+fn the_measured_band_measures_two_rows_at_once() {
+    let fixture = Ranked::of(8);
+    let facts = runner::trade::SliceFacts::of(&fixture.bars, &fixture.run.column);
+    let mut rules = Rules::elite(0, 25);
+    rules.top = 1;
+    let mut parallel = band_rows(&fixture, &facts);
+    let mut sequential = band_rows(&fixture, &facts);
+    assert!(parallel.len() > 2, "fixture: rows to measure");
+    let meeting = std::sync::Arc::new(Meeting::default());
+    let pool = rayon::ThreadPoolBuilder::new()
+        .num_threads(2)
+        .build()
+        .expect("a two-thread pool");
+    pool.install(|| {
+        let arrive = std::sync::Arc::clone(&meeting);
+        MEASURE_ROW_HOOK.with(|hook| {
+            *hook.borrow_mut() = Some(std::sync::Arc::new(move || arrive.arrive()));
+        });
+        measure_top(
+            &mut parallel,
+            &fixture.bars,
+            &fixture.run.column,
+            Horizon::DEFAULT,
+            rules,
+            &facts,
+        );
+        MEASURE_ROW_HOOK.with(|hook| *hook.borrow_mut() = None);
+    });
+    assert!(
+        meeting.overlapped(),
+        "two rows of the band were never measured at the same moment"
+    );
+    let one = rayon::ThreadPoolBuilder::new()
+        .num_threads(1)
+        .build()
+        .expect("a one-thread pool");
+    one.install(|| {
+        measure_top(
+            &mut sequential,
+            &fixture.bars,
+            &fixture.run.column,
+            Horizon::DEFAULT,
+            rules,
+            &facts,
+        );
+    });
+    assert_eq!(
+        parallel
+            .iter()
+            .map(|row| row.consistency.clone())
+            .collect::<Vec<_>>(),
+        sequential
+            .iter()
+            .map(|row| row.consistency.clone())
+            .collect::<Vec<_>>(),
+        "the parallel band is the sequential band, row for row"
+    );
+}
+
+// ---------------------------------------------------------------------------
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 4dfe6ced..c80490b1 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7067,3 +7067,4 @@ old line regex the same input and watched it pass.
 | L1FB-08 | `docs/06-limits.md` states one column build per rung, derived support or named, and three minute reads per rung; AU-O1CLI-2's and AU-O1CLI-3's tests now hold that statement and fail if a second build returns (D-4719) | `a_rungs_second_load_and_build_are_stated_and_still_paid` in `crates/cli/tests/limits_o1cli_2.rs`, `the_parallel_rungs_repeated_minute_reads_are_stated_and_still_paid` in `crates/cli/tests/limits_o1cli_3.rs` | ✓ |
 | L1FB-09 | Reading three rungs' rows back by identity over one root opens the results ledger once (D-4720) | `cli::results_report_tests::a_rungs_readback_opens_the_ledger_once_per_root_not_once_per_rung` | ✓ |
 | L1FB-10 | The results listing holds at most `LIST_ROWS` rows at every step of its fold, and exactly the newest 40 matching rows at the end, at unchanged capacity (D-4721) | `cli::screen_policy_tests::the_results_listing_retains_a_bounded_window` | ✓ |
+| L1FB-11 | `measure_top` measures two rows of its band at the same moment on a two-thread pool, and the figures equal a one-thread pool's (D-4722) | `cli::screen_policy_tests::the_measured_band_measures_two_rows_at_once` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index fe135561..17983664 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65202,3 +65202,16 @@ now folds 500 rows (100 filtered out) and checks at every step that at most
 `LIST_ROWS` are held, then that exactly the newest 40 matching rows are held
 in order at an unchanged capacity. With `pop_front` deleted it failed "row
 50: the window grew". No production code changes.
+
+### D-4722 — The measured band's parallelism is pinned — 2026-10-09
+
+**Finding.** W2-cli8-7, test gap. `measure_top` measures its band with
+`par_iter_mut`, but a sequential loop gives the same answer, so reverting it
+passed every test.
+
+**Decision.** A test-only hook, `MEASURE_ROW_HOOK`, runs once per measured
+row. `the_measured_band_measures_two_rows_at_once` installs a rendezvous on a
+two-thread pool: it passes only when two rows are inside the hook at once
+(bounded wait of 20 s), and the band's figures equal a one-thread pool's.
+With `iter_mut` it failed "two rows of the band were never measured at the
+same moment".
-- 
2.43.0


From ce01b96090d4a7f0e845fdf831f5a43926f297fa Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:29:29 +0000
Subject: [PATCH 7/8] cli: move the band docs off TOP_CEILING onto their items
 (G2-1, D-4723)

Before: measure_top's and measured_band's doc blocks ran on into
TOP_CEILING's, so all three attached to the constant and the two
functions had none.

After: each block sits on its own item; a test reads the doc directly
above each of the three.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/lib.rs                 | 96 +++++++++++++--------------
 crates/cli/src/screen_policy_tests.rs | 47 +++++++++++++
 docs/04-invariants.md                 |  1 +
 docs/05-decisions.md                  |  9 +++
 4 files changed, 105 insertions(+), 48 deletions(-)

diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index c00573fc..b4582efa 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -14336,18 +14336,43 @@ fn calendar_terms(c: &Consistency) -> (i64, i128) {
     (c.weakest_bp(), c.worst_day)
 }
 
-/// Measure consistency for the rows that will actually be printed.
-///
-/// # Why the caller does not do this inline
-///
-/// Two reasons, and the first is a bug that was measured. Inline, this ran
-/// inside the screening loop -- over `screen_cap()` combinations, ten
-/// thousand by default -- and every one paid for a full trade-by-trade
-/// re-walk when only `top` are ever rendered. A single-month screen that
-/// had taken seconds stopped finishing inside 280.
+/// The most rows one listing may ask to print: `BRUTEX_TOP` and every argv
+/// `TOP` are refused above it, by name (W2-cli8-7, D-1727).
 ///
-/// The second is that [`screen`] was 113 lines with it, past the hundred
-/// clippy enforces.
+/// `measure_top` measures `measured_band(top)` = `8 x top` rows, each a full
+/// exit-grid rebuild plus seven calendar grains, so an unbounded `top` was an
+/// unbounded per-request cost. A thousand printed rows is a page nobody reads
+/// whole; past it the cost grows and the answer does not.
+pub(crate) const TOP_CEILING: usize = 1_000;
+
+/// `BRUTEX_TOP`, or the documented 25 with the unusable value NAMED by
+/// [`crate::knobs::refused`]. Zero and anything above [`TOP_CEILING`] are
+/// unusable: zero lists nothing, and past the ceiling the measured band is an
+/// unbounded per-request cost (D-1727).
+fn top_from_knob() -> usize {
+    let Some(raw) = crate::knobs::var("BRUTEX_TOP") else {
+        return 25;
+    };
+    crate::knobs::machine_count(&raw, TOP_CEILING).unwrap_or_else(|| {
+        crate::knobs::refuse_value("BRUTEX_TOP", &raw);
+        25
+    })
+}
+
+// Every TOP the door admits is one the frontier can serve (D-1981).
+const _: () = assert!(TOP_CEILING <= frontier::MAX_ROWS);
+
+/// The refusal for a `TOP` outside `1..=TOP_CEILING`, or `None`.
+pub(crate) const fn top_refusal(top: usize) -> Option<&'static str> {
+    if top == 0 {
+        Some("TOP must be 1 or more")
+    } else if top > TOP_CEILING {
+        Some("TOP must be 1000 or fewer: each printed row costs eight measured rows")
+    } else {
+        None
+    }
+}
+
 /// How many rows get their seven-grain calendar measured, given a wanted top N.
 ///
 /// # The circularity this exists to break
@@ -14388,43 +14413,6 @@ fn calendar_terms(c: &Consistency) -> (i64, i128) {
 /// what is printed, which is enough for the calendar gate to demote a measured
 /// row and still have a measured replacement, and independent of how wide the
 /// search was.
-/// The most rows one listing may ask to print: `BRUTEX_TOP` and every argv
-/// `TOP` are refused above it, by name (W2-cli8-7, D-1727).
-///
-/// `measure_top` measures `measured_band(top)` = `8 x top` rows, each a full
-/// exit-grid rebuild plus seven calendar grains, so an unbounded `top` was an
-/// unbounded per-request cost. A thousand printed rows is a page nobody reads
-/// whole; past it the cost grows and the answer does not.
-pub(crate) const TOP_CEILING: usize = 1_000;
-
-/// `BRUTEX_TOP`, or the documented 25 with the unusable value NAMED by
-/// [`crate::knobs::refused`]. Zero and anything above [`TOP_CEILING`] are
-/// unusable: zero lists nothing, and past the ceiling the measured band is an
-/// unbounded per-request cost (D-1727).
-fn top_from_knob() -> usize {
-    let Some(raw) = crate::knobs::var("BRUTEX_TOP") else {
-        return 25;
-    };
-    crate::knobs::machine_count(&raw, TOP_CEILING).unwrap_or_else(|| {
-        crate::knobs::refuse_value("BRUTEX_TOP", &raw);
-        25
-    })
-}
-
-// Every TOP the door admits is one the frontier can serve (D-1981).
-const _: () = assert!(TOP_CEILING <= frontier::MAX_ROWS);
-
-/// The refusal for a `TOP` outside `1..=TOP_CEILING`, or `None`.
-pub(crate) const fn top_refusal(top: usize) -> Option<&'static str> {
-    if top == 0 {
-        Some("TOP must be 1 or more")
-    } else if top > TOP_CEILING {
-        Some("TOP must be 1000 or fewer: each printed row costs eight measured rows")
-    } else {
-        None
-    }
-}
-
 const fn measured_band(top: usize) -> usize {
     const WIDEN: usize = 8;
     const FLOOR: usize = 32;
@@ -14432,6 +14420,18 @@ const fn measured_band(top: usize) -> usize {
     if widened < FLOOR { FLOOR } else { widened }
 }
 
+/// Measure consistency for the rows that will actually be printed.
+///
+/// # Why the caller does not do this inline
+///
+/// Two reasons, and the first is a bug that was measured. Inline, this ran
+/// inside the screening loop -- over `screen_cap()` combinations, ten
+/// thousand by default -- and every one paid for a full trade-by-trade
+/// re-walk when only `top` are ever rendered. A single-month screen that
+/// had taken seconds stopped finishing inside 280.
+///
+/// The second is that [`screen`] was 113 lines with it, past the hundred
+/// clippy enforces.
 fn measure_top(
     rows: &mut [Screened<'_>],
     bars: &[indicators::Candle],
diff --git a/crates/cli/src/screen_policy_tests.rs b/crates/cli/src/screen_policy_tests.rs
index 41606705..ed51689d 100644
--- a/crates/cli/src/screen_policy_tests.rs
+++ b/crates/cli/src/screen_policy_tests.rs
@@ -1996,3 +1996,50 @@ fn the_measured_band_measures_two_rows_at_once() {
 }
 
 // ---------------------------------------------------------------------------
+// G2-1 (D-4723) and G2-2 (D-4724): documentation attached to its own item, and
+// no limit describing a removed function as current.
+// ---------------------------------------------------------------------------
+
+/// The `///` lines directly above the item that starts at `head` in `lib.rs`.
+fn doc_above(head: &str) -> String {
+    let source = include_str!("lib.rs");
+    let (before, _) = source
+        .split_once(head)
+        .unwrap_or_else(|| panic!("{head} is no longer in lib.rs"));
+    let mut doc: Vec<&str> = before
+        .lines()
+        .rev()
+        .take_while(|line| line.trim_start().starts_with("///"))
+        .collect();
+    doc.reverse();
+    doc.join("\n")
+}
+
+/// G2-1, D-4723. Commit b489169 (D-1727) inserted `TOP_CEILING` between three
+/// doc blocks and their items, so `measure_top`'s and `measured_band`'s docs
+/// both attached to the constant and the two functions carried none. Each doc
+/// now sits on its own item.
+#[test]
+fn each_screen_band_doc_sits_on_its_own_item() {
+    const MEASURE: &str = "Measure consistency for the rows that will actually be printed";
+    const BAND: &str = "How many rows get their seven-grain calendar measured";
+    const CEILING: &str = "The most rows one listing may ask to print";
+    let measure = doc_above("\nfn measure_top(");
+    let band = doc_above("\nconst fn measured_band(");
+    let ceiling = doc_above("\npub(crate) const TOP_CEILING");
+    assert!(measure.contains(MEASURE), "measure_top's doc:\n{measure}");
+    assert!(
+        !measure.contains(BAND) && !measure.contains(CEILING),
+        "{measure}"
+    );
+    assert!(band.contains(BAND), "measured_band's doc:\n{band}");
+    assert!(!band.contains(MEASURE) && !band.contains(CEILING), "{band}");
+    assert!(
+        ceiling.trim_start().starts_with(&format!("/// {CEILING}")),
+        "TOP_CEILING's doc:\n{ceiling}"
+    );
+    assert!(
+        !ceiling.contains(MEASURE) && !ceiling.contains(BAND),
+        "{ceiling}"
+    );
+}
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index c80490b1..8cd04caa 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7068,3 +7068,4 @@ old line regex the same input and watched it pass.
 | L1FB-09 | Reading three rungs' rows back by identity over one root opens the results ledger once (D-4720) | `cli::results_report_tests::a_rungs_readback_opens_the_ledger_once_per_root_not_once_per_rung` | ✓ |
 | L1FB-10 | The results listing holds at most `LIST_ROWS` rows at every step of its fold, and exactly the newest 40 matching rows at the end, at unchanged capacity (D-4721) | `cli::screen_policy_tests::the_results_listing_retains_a_bounded_window` | ✓ |
 | L1FB-11 | `measure_top` measures two rows of its band at the same moment on a two-thread pool, and the figures equal a one-thread pool's (D-4722) | `cli::screen_policy_tests::the_measured_band_measures_two_rows_at_once` | ✓ |
+| L1FB-12 | `measure_top`, `measured_band` and `TOP_CEILING` each carry their own doc and no other's (D-4723) | `cli::screen_policy_tests::each_screen_band_doc_sits_on_its_own_item` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 17983664..f997906f 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65215,3 +65215,12 @@ two-thread pool: it passes only when two rows are inside the hook at once
 (bounded wait of 20 s), and the band's figures equal a one-thread pool's.
 With `iter_mut` it failed "two rows of the band were never measured at the
 same moment".
+
+### D-4723 — The band docs sit on `measure_top` and `measured_band` — 2026-10-09
+
+**Finding.** G2-1. Three doc blocks ran on with no item between them, so
+`measure_top`'s and `measured_band`'s docs both attached to `TOP_CEILING`.
+
+**Decision.** Each block moved onto its own item; `TOP_CEILING` keeps only
+its own. `each_screen_band_doc_sits_on_its_own_item` reads the doc directly
+above each of the three items; it failed on `measure_top`'s empty doc.
-- 
2.43.0


From fc5291b65f82bf50d2b4d9af0c6f790231effe70 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:29:30 +0000
Subject: [PATCH 8/8] docs: two latest_for limits no longer state a removed
 function as current (G2-2, D-4724)

Before: two docs/06-limits.md bullets stated latest_for's O(runs) per
call as a current cost, though D-1700 removed it.

After: both name D-1700 and recorded_row; a test requires every bullet
headed by latest_for to name D-1700.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/screen_policy_tests.rs | 31 +++++++++++++++++++++++++++
 docs/04-invariants.md                 |  1 +
 docs/05-decisions.md                  | 11 ++++++++++
 docs/06-limits.md                     | 20 ++++++++++-------
 4 files changed, 55 insertions(+), 8 deletions(-)

diff --git a/crates/cli/src/screen_policy_tests.rs b/crates/cli/src/screen_policy_tests.rs
index ed51689d..bd95b55c 100644
--- a/crates/cli/src/screen_policy_tests.rs
+++ b/crates/cli/src/screen_policy_tests.rs
@@ -2043,3 +2043,34 @@ fn each_screen_band_doc_sits_on_its_own_item() {
         "{ceiling}"
     );
 }
+
+/// G2-2, D-4724. `latest_for` was removed by D-1700. Two bullets of
+/// `docs/06-limits.md` still stated its O(runs) cost as current beside the
+/// corrected copies; every bullet naming it must say it is gone.
+#[test]
+fn no_limit_states_the_removed_latest_for_as_current() {
+    let limits = include_str!("../../../docs/06-limits.md");
+    let bullets: Vec<&str> = limits
+        .split("\n- ")
+        .skip(1)
+        .map(|bullet| {
+            let paragraph = bullet.split_once("\n\n").map_or(bullet, |(head, _)| head);
+            paragraph
+                .split_once("\n#")
+                .map_or(paragraph, |(head, _)| head)
+        })
+        .filter(|bullet| {
+            bullet
+                .lines()
+                .next()
+                .is_some_and(|head| head.contains("latest_for`"))
+        })
+        .collect();
+    assert!(bullets.len() >= 2, "the corrected bullets remain");
+    for bullet in bullets {
+        assert!(
+            bullet.contains("D-1700"),
+            "a bullet still states `latest_for` as current:\n{bullet}"
+        );
+    }
+}
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 8cd04caa..7e596414 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7069,3 +7069,4 @@ old line regex the same input and watched it pass.
 | L1FB-10 | The results listing holds at most `LIST_ROWS` rows at every step of its fold, and exactly the newest 40 matching rows at the end, at unchanged capacity (D-4721) | `cli::screen_policy_tests::the_results_listing_retains_a_bounded_window` | ✓ |
 | L1FB-11 | `measure_top` measures two rows of its band at the same moment on a two-thread pool, and the figures equal a one-thread pool's (D-4722) | `cli::screen_policy_tests::the_measured_band_measures_two_rows_at_once` | ✓ |
 | L1FB-12 | `measure_top`, `measured_band` and `TOP_CEILING` each carry their own doc and no other's (D-4723) | `cli::screen_policy_tests::each_screen_band_doc_sits_on_its_own_item` | ✓ |
+| L1FB-13 | No `docs/06-limits.md` bullet headed by `latest_for` states it without D-1700, which removed it (D-4724) | `cli::screen_policy_tests::no_limit_states_the_removed_latest_for_as_current` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index f997906f..a8028a1a 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65224,3 +65224,14 @@ same moment".
 **Decision.** Each block moved onto its own item; `TOP_CEILING` keeps only
 its own. `each_screen_band_doc_sits_on_its_own_item` reads the doc directly
 above each of the three items; it failed on `measure_top`'s empty doc.
+
+### D-4724 — Two `latest_for` limits no longer state a removed function as current — 2026-10-09
+
+**Finding.** G2-2. Two `docs/06-limits.md` bullets stated `latest_for`'s
+O(runs) per call as a current cost, while D-1700 removed the function and
+other bullets already said so.
+
+**Decision.** Both bullets now name D-1700 and `recorded_row`.
+`no_limit_states_the_removed_latest_for_as_current` requires every limit
+bullet headed by `latest_for` to name D-1700; it failed on the first stale
+bullet.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 937b4825..4388392b 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -13034,10 +13034,12 @@ not:
   audit, which consumes them), the O(bars) scans that derive its horizon,
   grid rungs, floors and policy from the held bars, and its own sweep. NOT
   MEASURED.
-- **`latest_for` (D-1567).** O(runs) per call: it opens the results ledger,
-  which builds the identity index and hashes the file, before its backward
-  scan. Called once per rung of `range-all`, `pool` pass 1 and every `descend`
-  step.
+- **`latest_for` (D-1567, removed by D-1700).** No longer a cost: the
+  function is gone. Its O(runs) per call, a ledger open before a backward
+  scan once per rung of `range-all`, `pool` pass 1 and every `descend` step,
+  is history; `recorded_row` replaced it, and the first `latest_for` bullet
+  of this section states what that costs. Restated by D-4724, which found
+  this bullet still describing the removed function as current.
 ## Audit fixes — D-1480 onward, 3 October 2026
 
 **A credential watch's dead-value check is O(d), not O(1) (v3a-1, D-1482).**
@@ -15045,10 +15047,12 @@ UNVERIFIED for the rest:
 - **`api::server::form_read_bound` (D-1592).** "O(1)": four comparisons since D-1770 (two before)
   against literal paths. `api::server::form_read_bound_is_wide_only_on_the_member_routes`
   proves which route gets which bound; nothing times the call.
-- **`cli::latest_for` (D-1567).** The stated O(runs) per call (the bullet
-  above) rests on the audit's measurement (14.13x open cost for 10x rows,
-  o1surface2-4). `crates/cli/benches/ratio.rs` deliberately does not time
-  `Results::open`, so no tracked bench repeats it.
+- **`cli::latest_for` (D-1567, removed by D-1700).** The O(runs) per call
+  this bullet once rested on the audit's measurement (14.13x open cost for
+  10x rows, o1surface2-4) belongs to a removed function. Its replacement
+  `recorded_row` is stated from the code's shape and not timed:
+  `crates/cli/benches/ratio.rs` deliberately does not time `Results::open` or
+  the shared handle's refresh. Restated by D-4724.
 ## A rate span published slower than one permit a second keeps the old floor — D-1769, 3 October 2026
 
 `pull::rate::Window::floor_of` floors every span at one permit a second in its
-- 
2.43.0

