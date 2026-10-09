diff --git a/crates/cli/src/pool.rs b/crates/cli/src/pool.rs
index 0b5b3597..9d1dbdab 100644
--- a/crates/cli/src/pool.rs
+++ b/crates/cli/src/pool.rs
@@ -92,14 +92,20 @@
 //!
 //! Pass 2 is, per instrument, the loads, the column, the projection onto the
 //! one-minute execution series and one `SliceFacts`, `O(B_sig + B_exec)`;
-//! then per union candidate one `grid::evaluate_over`, which walks EVERY row
-//! of the projected column before it prices: `Θ(B_exec + cells × T)`. So
-//! pass 2 is `Θ(I × (B_sig + B_exec) + I × U × (B_exec + cells × T))`, and
-//! because the union U is the union of every instrument's kept frontier, U
-//! grows with I (up to I × `top`): up to `Θ(I² × top × B_exec)` for the rare
-//! setups this pool exists to find, where T is small and `B_exec` is large.
-//! Until D-1702 this said "each the cost of one exit grid over that
-//! instrument's trades",
+//! then one posting pass over the projected column, `Θ(B_exec + P)` for the
+//! `P` rows posted under the bits the union names; then per union candidate
+//! one `grid::evaluate_over_rows` over the posting list of the candidate's
+//! rarest bit, `Θ(R + cells × T)` for that list's `R` rows (D-4707). So pass
+//! 2 is `Θ(I × (B_sig + B_exec + P) + I × U × (R + cells × T))`. `R` counts
+//! the rows of one bit, not the rows the whole mask fires on, so it is at
+//! most `B_exec`, and a rare conjunction of common bits still walks its
+//! rarest bit's rows; the empty mask, which names no bit, walks every row.
+//! Because the union U is the union of every instrument's kept frontier, U
+//! grows with I (up to I × `top`), and in the worst case `R = B_exec` pass 2
+//! is still `Θ(I² × top × B_exec)`. Until D-4707 every candidate walked
+//! every row, `Θ(B_exec + cells × T)`, which is that worst case for every
+//! candidate. Until D-1702 this said "each the cost of one exit grid over
+//! that instrument's trades",
 //! which left out the `B_exec` walk (R9-cli-o1-1). None of these is a rule-4
 //! operation: those bound the per-bar and per-candidate primitives INSIDE the
 //! screen, which are unchanged. The pooled fold is one pass over
@@ -1092,9 +1098,16 @@ pub(crate) fn union_of(
 /// # Cost
 ///
 /// Per instrument: the loads, `O(B_sig + B_exec)` for the column, the projection
-/// and one hoisted `SliceFacts`; then per candidate one `grid::evaluate_over`,
-/// which walks every row of the projected column -- `Θ(B_exec)` -- before it
-/// prices `cells × T`. `docs/06-limits.md` states the whole pass (R9-cli-o1-1).
+/// and one hoisted `SliceFacts`; then one [`Postings`] pass over the projected
+/// column, `Θ(B_exec + P)` for `P` postings; then per candidate the rarest of
+/// its bits' posting lists, `O(BITS)`, and one
+/// `grid::evaluate_over_rows` over that list -- `Θ(|rarest| + cells × T)`. The
+/// empty mask fires on every row and has no list to read, so it alone keeps
+/// `grid::evaluate_over`'s `Θ(B_exec)` walk. Until D-4707 every candidate
+/// walked every row. The figures are the same either way: the walk changes
+/// state only on a row the mask fires on, and every such row sets every bit the
+/// mask names, so it is on each of their lists. `docs/06-limits.md` states the
+/// whole pass (R9-cli-o1-1).
 fn price_all(
     root: &std::path::Path,
     vendor: brutex_core::vendor::Vendor,
@@ -1125,7 +1138,13 @@ fn price_all(
         stops_ppm: &stop_rungs,
     };
     let facts = runner::trade::SliceFacts::of(bars, &column);
-    Ok(union
+    let named = union
+        .iter()
+        .fold(vocab::ConditionMask::ZERO, |all, candidate| {
+            all.union(&vocab::ConditionMask::from_words(candidate.words))
+        });
+    let postings = Postings::of(&column, &named);
+    union
         .iter()
         .map(|candidate| {
             let mask = vocab::ConditionMask::from_words(candidate.words);
@@ -1133,12 +1152,85 @@ fn price_all(
                 Direction::Long => runner::excursion::Side::Long,
                 Direction::Short => runner::excursion::Side::Short,
             };
-            let g = grid::evaluate_over(bars, &column, &mask, horizon, side, levels, &facts);
-            crate::shown_cell(&g, rules)
+            let g = match postings.rarest(&mask) {
+                Some(rows) => grid::evaluate_over_rows(
+                    bars, &column, &mask, horizon, side, levels, &facts, rows,
+                )?,
+                None => grid::evaluate_over(bars, &column, &mask, horizon, side, levels, &facts),
+            };
+            Ok(crate::shown_cell(&g, rules)
                 .map(|(cell, _admitted)| cell)
-                .filter(|cell| cell.trades > 0)
+                .filter(|cell| cell.trades > 0))
         })
-        .collect())
+        .collect()
+}
+
+/// The column rows each condition bit is set on, for the bits a union names
+/// (R9-cli-o1-1, D-4707).
+///
+/// A mask fires on a row only when the row sets every bit the mask names, so
+/// the rows any one of those bits is set on hold every row the mask fires on.
+/// [`Self::rarest`] hands the shortest such list to
+/// `runner::trade::walk_over_rows`, which asks each listed row the full mask
+/// test and so passes over a listed row that does not fire exactly as the full
+/// walk does.
+///
+/// # Cost
+///
+/// [`Self::of`] is one pass over the column: per row, one six-word
+/// intersection with the named bits and one push per bit left set, so
+/// `Θ(B_exec + P)` for `P` postings in all, and `P` rows of memory, amortised
+/// over the pushes as `Vec::push` is.
+/// [`Self::rarest`] looks at each bit a mask names once: `O(BITS)`, no row read.
+/// A list is as long as its bit is common: a rare conjunction of common bits
+/// still walks the rows of its rarest bit, up to `B_exec`.
+struct Postings {
+    /// One list per mask position, `ConditionMask::BITS` of them, ascending.
+    /// A position the union does not name keeps an empty list it is never
+    /// asked for.
+    rows: Vec<Vec<usize>>,
+}
+
+impl Postings {
+    /// Every row of `column` posted under each bit of `named` it sets.
+    fn of(column: &indicators::column::Column, named: &vocab::ConditionMask) -> Self {
+        let width = usize::try_from(vocab::ConditionMask::BITS).unwrap_or(0);
+        let mut rows: Vec<Vec<usize>> = (0..width).map(|_| Vec::new()).collect();
+        for (row, bits) in column.bits().iter().enumerate() {
+            for (word, set) in bits.intersect(named).words().into_iter().enumerate() {
+                let mut left = set;
+                while left != 0 {
+                    // `word < WORDS` and the offset is below 64, so the
+                    // position is below `BITS` and the list is there.
+                    let at = word * 64 + left.trailing_zeros() as usize;
+                    if let Some(list) = rows.get_mut(at) {
+                        list.push(row);
+                    }
+                    left &= left - 1;
+                }
+            }
+        }
+        Self { rows }
+    }
+
+    /// The shortest posting list among the bits `mask` names, or `None` for
+    /// the empty mask, which names no bit and fires on every row.
+    fn rarest(&self, mask: &vocab::ConditionMask) -> Option<&[usize]> {
+        let mut best: Option<&[usize]> = None;
+        for (word, set) in mask.words().into_iter().enumerate() {
+            let mut left = set;
+            while left != 0 {
+                let at = word * 64 + left.trailing_zeros() as usize;
+                if let Some(list) = self.rows.get(at)
+                    && best.is_none_or(|shortest| list.len() < shortest.len())
+                {
+                    best = Some(list.as_slice());
+                }
+                left &= left - 1;
+            }
+        }
+        best
+    }
 }
 
 /// One instrument's span over one month range, prepared exactly as the screen
@@ -3153,10 +3245,12 @@ mod tests {
             "crate::project_onto_execution(",
             "&span.bars, &column, execution, native, horizon,",
         ];
-        const PRICE: [&str; 3] = [
+        const PRICE: [&str; 5] = [
             "prepare_span(root, vendor, underlying, rung, from, to)?",
             "SliceFacts::of(bars, &column)",
-            "evaluate_over(bars, &column,",
+            "Postings::of(&column, &named)",
+            "postings.rarest(&mask)",
+            "evaluate_over_rows(",
         ];
         let pool = include_str!("pool.rs");
         let body_of = |head: &str| {
@@ -3858,6 +3952,282 @@ mod tests {
         crate::knobs::clear_all();
     }
 
+    /// Pass 2 as it priced until D-4707: one `grid::evaluate_over` per
+    /// candidate, walking every row of the prepared column. The oracle the
+    /// posting-list walk is held to, so its inputs are built here exactly as
+    /// `price_all` builds them.
+    fn priced_over_every_row(
+        root: &std::path::Path,
+        rung: &'static str,
+        span: ((u16, u8), (u16, u8)),
+        union: &[Candidate],
+    ) -> Vec<super::Priced> {
+        let vendor = brutex_core::vendor::Vendor::Zerodha;
+        let super::PreparedSpan {
+            bars,
+            column,
+            horizon,
+            rules,
+        } = super::prepare_span(root, vendor, "NIFTY", rung, span.0, span.1)
+            .expect("premise: the span prepares");
+        let bars = bars.as_slice();
+        let hold = usize::try_from(horizon.as_bars()).unwrap_or(usize::MAX);
+        let stop_rungs = crate::stop_ladder_ppm(bars, hold);
+        let levels = grid::Levels {
+            rungs: crate::grid_rungs(bars),
+            step_ppm: Some(crate::grid_step_ppm(bars, hold)),
+            forced: Some(rules.max_mae_ppm),
+            ratios: true,
+            stops_ppm: &stop_rungs,
+        };
+        let facts = runner::trade::SliceFacts::of(bars, &column);
+        union
+            .iter()
+            .map(|candidate| {
+                let mask = vocab::ConditionMask::from_words(candidate.words);
+                let side = match candidate.direction {
+                    Direction::Long => runner::excursion::Side::Long,
+                    Direction::Short => runner::excursion::Side::Short,
+                };
+                let g = grid::evaluate_over(bars, &column, &mask, horizon, side, levels, &facts);
+                crate::shown_cell(&g, rules)
+                    .map(|(cell, _admitted)| cell)
+                    .filter(|cell| cell.trades > 0)
+            })
+            .collect()
+    }
+
+    /// How many rows of `column` set each mask position, `BITS` of them.
+    fn rows_per_bit(column: &indicators::column::Column) -> Vec<usize> {
+        let width = usize::try_from(vocab::ConditionMask::BITS).expect("a small width");
+        (0..width)
+            .map(|bit| {
+                let bit = u32::try_from(bit).expect("a small position");
+                column.bits().iter().filter(|bits| bits.get(bit)).count()
+            })
+            .collect()
+    }
+
+    /// A union drawn from the prepared column itself: the empty mask, the
+    /// first ten bits set on some rows but not all, five neighbouring pairs
+    /// and one triple of those, and a table bit no row sets, each both ways.
+    fn drawn_union(column: &indicators::column::Column) -> Vec<Candidate> {
+        let counts = rows_per_bit(column);
+        let table = vocab::table::TABLE.len();
+        let partial: Vec<u32> = counts
+            .iter()
+            .enumerate()
+            .filter(|&(_, &n)| n > 0 && n < column.len())
+            .filter_map(|(bit, _)| u32::try_from(bit).ok())
+            .take(10)
+            .collect();
+        assert!(partial.len() >= 4, "premise: bits that split the rows");
+        let never = counts
+            .iter()
+            .take(table)
+            .enumerate()
+            .filter(|&(_, &n)| n == 0)
+            .filter_map(|(bit, _)| u32::try_from(bit).ok())
+            .next()
+            .expect("premise: a table bit no row sets");
+        let mut masks = vec![vocab::ConditionMask::ZERO];
+        masks.extend(
+            partial
+                .iter()
+                .map(|&b| vocab::ConditionMask::ZERO.with_bit(b)),
+        );
+        masks.extend(partial.windows(2).take(5).filter_map(|pair| match pair {
+            &[a, b] => Some(vocab::ConditionMask::ZERO.with_bit(a).with_bit(b)),
+            _ => None,
+        }));
+        let pick = |at: usize| *partial.get(at).expect("premise: four split bits");
+        masks.push(
+            vocab::ConditionMask::ZERO
+                .with_bit(pick(0))
+                .with_bit(pick(2))
+                .with_bit(pick(3)),
+        );
+        masks.push(vocab::ConditionMask::ZERO.with_bit(never));
+        masks
+            .iter()
+            .flat_map(|mask| {
+                [Direction::Long, Direction::Short].map(|direction| Candidate {
+                    words: mask.words(),
+                    direction,
+                })
+            })
+            .collect()
+    }
+
+    /// **R9-cli-o1-1, D-4707: pass 2 walks each candidate over its rarest
+    /// bit's posting list, and every cell is the cell the every-row walk
+    /// prices.** At 5min (May 2025) and 60min (March to May) on the generated
+    /// random-walk store, `price_all` equals [`priced_over_every_row`]
+    /// candidate for candidate, and the whole grid over the listed rows
+    /// equals the whole grid over every row, cell for cell. Premises: a
+    /// non-empty mask traded over a list shorter than the column, the never
+    /// set bit priced nothing over an empty list, and the empty mask, which
+    /// has no list, traded.
+    #[test]
+    fn pass_two_over_posting_lists_prices_what_every_row_prices() {
+        let _knobs = crate::knobs::serially();
+        crate::knobs::clear_all();
+        crate::audited_stored::with_varied_store(|root| {
+            let vendor = brutex_core::vendor::Vendor::Zerodha;
+            for (rung, span) in [
+                ("5min", ((2025, 5), (2025, 5))),
+                ("60min", ((2025, 3), (2025, 5))),
+            ] {
+                let super::PreparedSpan {
+                    bars,
+                    column,
+                    horizon,
+                    rules,
+                } = super::prepare_span(root, vendor, "NIFTY", rung, span.0, span.1)
+                    .expect("premise: the span prepares");
+                let union = drawn_union(&column);
+                let priced = super::price_all(root, vendor, "NIFTY", rung, span.0, span.1, &union)
+                    .expect("pass 2 prices the span");
+                assert_eq!(
+                    priced,
+                    priced_over_every_row(root, rung, span, &union),
+                    "{rung}: the posting-list cells are the every-row cells"
+                );
+
+                let named = union.iter().fold(vocab::ConditionMask::ZERO, |all, c| {
+                    all.union(&vocab::ConditionMask::from_words(c.words))
+                });
+                let postings = super::Postings::of(&column, &named);
+                let hold = usize::try_from(horizon.as_bars()).unwrap_or(usize::MAX);
+                let stop_rungs = crate::stop_ladder_ppm(&bars, hold);
+                let levels = grid::Levels {
+                    rungs: crate::grid_rungs(&bars),
+                    step_ppm: Some(crate::grid_step_ppm(&bars, hold)),
+                    forced: Some(rules.max_mae_ppm),
+                    ratios: true,
+                    stops_ppm: &stop_rungs,
+                };
+                let facts = runner::trade::SliceFacts::of(&bars, &column);
+                let (mut narrower, mut unlisted, mut never) = (0, 0, 0);
+                for (candidate, cell) in union.iter().zip(&priced) {
+                    let mask = vocab::ConditionMask::from_words(candidate.words);
+                    let side = match candidate.direction {
+                        Direction::Long => runner::excursion::Side::Long,
+                        Direction::Short => runner::excursion::Side::Short,
+                    };
+                    let every =
+                        grid::evaluate_over(&bars, &column, &mask, horizon, side, levels, &facts);
+                    match postings.rarest(&mask) {
+                        Some(rows) => {
+                            let listed = grid::evaluate_over_rows(
+                                &bars, &column, &mask, horizon, side, levels, &facts, rows,
+                            )
+                            .expect("a posting list is ascending and inside the column");
+                            assert_eq!(listed, every, "{rung} {mask:?} {side:?}: the whole grid");
+                            narrower += usize::from(cell.is_some() && rows.len() < column.len());
+                            never += usize::from(rows.is_empty() && cell.is_none());
+                        }
+                        None => {
+                            assert!(mask.is_empty(), "only the empty mask has no list");
+                            unlisted += usize::from(cell.is_some());
+                        }
+                    }
+                }
+                assert!(
+                    narrower > 0,
+                    "premise: a {rung} mask traded over a shorter list"
+                );
+                assert_eq!(never, 2, "premise: the {rung} never-set bit, both ways");
+                assert!(unlisted > 0, "premise: the {rung} empty mask traded");
+            }
+        });
+        crate::knobs::clear_all();
+    }
+
+    /// D-4707: each named bit's posting list is exactly the rows that set
+    /// it, ascending; a bit the union does not name has no rows posted; and
+    /// `rarest` hands back the shorter list of two, whichever bit holds it,
+    /// and nothing for the empty mask.
+    #[test]
+    fn postings_list_each_named_bits_rows_and_rarest_is_the_shortest() {
+        let _knobs = crate::knobs::serially();
+        crate::knobs::clear_all();
+        crate::audited_stored::with_varied_store(|root| {
+            let vendor = brutex_core::vendor::Vendor::Zerodha;
+            let span = super::prepare_span(root, vendor, "NIFTY", "5min", (2025, 5), (2025, 5))
+                .expect("premise: the span prepares");
+            let column = &span.column;
+            let counts = rows_per_bit(column);
+            let split: Vec<u32> = counts
+                .iter()
+                .enumerate()
+                .filter(|&(_, &n)| n > 0 && n < column.len())
+                .filter_map(|(bit, _)| u32::try_from(bit).ok())
+                .collect();
+            let (named_bits, unnamed) = split.split_at(split.len() / 2);
+            assert!(!unnamed.is_empty(), "premise: a set bit left unnamed");
+            let named = named_bits
+                .iter()
+                .fold(vocab::ConditionMask::ZERO, |all, &b| all.with_bit(b));
+            let postings = super::Postings::of(column, &named);
+            assert_eq!(
+                postings.rows.len(),
+                usize::try_from(vocab::ConditionMask::BITS).expect("a small width")
+            );
+            for (bit, list) in postings.rows.iter().enumerate() {
+                let bit = u32::try_from(bit).expect("a small position");
+                let want: Vec<usize> = if named.get(bit) {
+                    column
+                        .bits()
+                        .iter()
+                        .enumerate()
+                        .filter(|(_, bits)| bits.get(bit))
+                        .map(|(row, _)| row)
+                        .collect()
+                } else {
+                    Vec::new()
+                };
+                assert_eq!(list, &want, "bit {bit}");
+            }
+            assert!(
+                unnamed.iter().all(|&b| postings
+                    .rows
+                    .get(usize::try_from(b).expect("small"))
+                    .is_some_and(Vec::is_empty)),
+                "an unnamed bit has no rows posted"
+            );
+            assert_eq!(postings.rarest(&vocab::ConditionMask::ZERO), None);
+            let mut uneven = 0;
+            for pair in named_bits.windows(2) {
+                let &[a, b] = pair else { continue };
+                let list = |bit: u32| {
+                    postings
+                        .rows
+                        .get(usize::try_from(bit).expect("small"))
+                        .expect("a list per position")
+                        .as_slice()
+                };
+                let mask = vocab::ConditionMask::ZERO.with_bit(a).with_bit(b);
+                let rarest = postings.rarest(&mask).expect("a non-empty mask has a list");
+                let (low, high) = if list(a).len() <= list(b).len() {
+                    (list(a), list(b))
+                } else {
+                    (list(b), list(a))
+                };
+                assert_eq!(rarest.len(), low.len(), "bits {a} and {b}: the shorter");
+                if low.len() < high.len() {
+                    assert!(
+                        core::ptr::eq(rarest, low),
+                        "bits {a} and {b}: the rarer bit's list"
+                    );
+                    uneven += 1;
+                }
+            }
+            assert!(uneven > 0, "premise: two named bits of different counts");
+        });
+        crate::knobs::clear_all();
+    }
+
     /// **Pass 2 withholds the day pass 1 withholds when that day's closing
     /// minute cannot be sourced, and prices the rest exactly where the audit
     /// path does.** D-1707, closing the difference D-1702 stated.
diff --git a/crates/cli/tests/limits_doc_drift.rs b/crates/cli/tests/limits_doc_drift.rs
index 549ab383..40cb337d 100644
--- a/crates/cli/tests/limits_doc_drift.rs
+++ b/crates/cli/tests/limits_doc_drift.rs
@@ -110,23 +110,29 @@ fn section_126_names_the_forward_cursor_that_cuts_fold_prefixes_not_a_binary_sea
 
 #[test]
 fn section_113_prices_a_trade_walk_by_the_column_rows_it_visits() {
-    let walk = function(TRADE, "fn walk_core(");
-    // Since D-1186 the walk starts at `first_row`, the first row that can
-    // fire, and visits every row from there; `index` stays the column row.
-    let row_loop = "for (offset, (bits, &signal)) in rows.enumerate() {";
+    let walk = function(TRADE, "fn walk_core<'r>(");
+    // Since D-4707 the walk loops over the rows its caller hands it, each
+    // carrying its column row as `index`: every row from `first_row` on
+    // (`rows_from`, D-1186), or exactly the rows `walk_over_rows` was given
+    // (`rows_listed`).
+    let row_loop = "for (index, bits, signal) in rows {";
     let at = walk
         .find(row_loop)
-        .expect("walk_core no longer loops over the column rows; re-measure §113");
+        .expect("walk_core no longer loops over the rows it is handed; re-measure §113");
+    let after = flat(&walk[at + row_loop.len()..]);
+    assert!(
+        after.starts_with("if !fires(bits, index) { continue; }"),
+        "the row loop no longer asks `fires` of every row it visits first",
+    );
+    let from = flat(function(TRADE, "fn rows_from("));
     assert!(
-        flat(&walk[..at]).contains(".get(first_row..)"),
+        from.contains(".get(first_row..)") && from.contains("first_row.saturating_add(offset)"),
         "the walk no longer starts at its first live row; re-measure §113",
     );
-    let after = flat(&walk[at + row_loop.len()..]);
+    let listed = flat(function(TRADE, "fn rows_listed<'c>("));
     assert!(
-        after.starts_with(
-            "let index = first_row.saturating_add(offset); if !fires(bits, index) { continue; }"
-        ),
-        "the row loop no longer asks `fires` of every row it visits first",
+        listed.contains("rows.iter()") && !listed.contains(".get(first_row"),
+        "`rows_listed` no longer yields exactly the listed rows; re-measure §113",
     );
 
     let text = flat(section(113));
@@ -138,6 +144,11 @@ fn section_113_prices_a_trade_walk_by_the_column_rows_it_visits() {
         text.contains("A trade walk is linear in the rows of the column it walks"),
         "§113 does not state the row-linear bound",
     );
+    assert!(
+        text.contains("`walk_over_rows` hands it exactly the strictly ascending rows")
+            && text.contains("so that walk is linear in the list"),
+        "§113 does not state the listed walk's bound (D-4707)",
+    );
 }
 
 #[test]
diff --git a/crates/runner/src/grid.rs b/crates/runner/src/grid.rs
index bc86aeec..43f7dc85 100644
--- a/crates/runner/src/grid.rs
+++ b/crates/runner/src/grid.rs
@@ -1995,6 +1995,42 @@ pub fn evaluate_over(
     )
 }
 
+/// [`evaluate_over`], walking only the column rows `rows` names (D-4707).
+///
+/// What `rows` must hold is [`crate::trade::walk_over_rows`]'s contract:
+/// strictly ascending, and every row the mask fires on. On such a list the
+/// grid equals [`evaluate_over`]'s exactly, because the walk is the only part
+/// that reads the column and the rest prices its trades.
+///
+/// # Errors
+///
+/// [`crate::trade::walk_over_rows`]'s refusal of the list; nothing is priced.
+#[expect(
+    clippy::too_many_arguments,
+    reason = "evaluate_over's seven inputs and the rows the walk visits"
+)]
+pub fn evaluate_over_rows(
+    bars: &[Candle],
+    column: &Column,
+    mask: &ConditionMask,
+    horizon: Horizon,
+    side: Side,
+    levels: Levels<'_>,
+    facts: &crate::trade::SliceFacts,
+    rows: &[usize],
+) -> Result<Grid, String> {
+    let timed =
+        crate::trade::walk_over_rows(bars, column, mask, horizon, direction_of(side), facts, rows)?;
+    Ok(evaluate_timed(
+        bars,
+        side,
+        levels,
+        &timed,
+        facts,
+        ExitFamilies::All,
+    ))
+}
+
 /// [`evaluate`] with an explicit exit-family population.
 ///
 /// Derives the same ladders as legacy evaluation, including its target
@@ -6405,7 +6441,9 @@ mod exit_family_tests {
     reason = "the exception every test module in this workspace takes."
 )]
 mod tests {
-    use super::{Cell, Grid, Levels, Streaks, evaluate, evaluate_over, tally_trade};
+    use super::{
+        Cell, Grid, Levels, Streaks, evaluate, evaluate_over, evaluate_over_rows, tally_trade,
+    };
 
     /// p3floor-1, D-1769: the gated mean-loss magnitude rounds UP.
     #[test]
@@ -7144,6 +7182,80 @@ mod tests {
         );
     }
 
+    /// D-4707: a grid over the rows a bit is set on is the grid over every
+    /// row, for every bit and both sides, and so is the empty mask's over
+    /// every row. A refused row list prices nothing.
+    #[test]
+    fn a_grid_over_a_superset_of_the_firing_rows_is_the_same_grid() {
+        let (bars, column) = swept();
+        let facts = crate::trade::SliceFacts::of(&bars, &column);
+        let every: Vec<usize> = (0..column.len()).collect();
+        let width = u32::try_from(vocab::table::TABLE.len()).expect("a small table");
+        let mut priced = 0;
+        for bit in 0..width {
+            let mask = ConditionMask::default().with_bit(bit);
+            let set: Vec<usize> = column
+                .bits()
+                .iter()
+                .enumerate()
+                .filter(|(_, bits)| bits.get(bit))
+                .map(|(row, _)| row)
+                .collect();
+            for side in [Side::Long, Side::Short] {
+                let full = evaluate_over(
+                    &bars,
+                    &column,
+                    &mask,
+                    h(15),
+                    side,
+                    Levels::derived(2),
+                    &facts,
+                );
+                let listed = evaluate_over_rows(
+                    &bars,
+                    &column,
+                    &mask,
+                    h(15),
+                    side,
+                    Levels::derived(2),
+                    &facts,
+                    &set,
+                )
+                .expect("an ascending list inside the column");
+                assert_eq!(full, listed, "bit {bit} {side:?}");
+                priced += usize::from(!full.cells.is_empty());
+            }
+        }
+        assert!(priced > 0, "some bit must price a cell");
+        let empty = ConditionMask::default();
+        let over = |rows: &[usize]| {
+            evaluate_over_rows(
+                &bars,
+                &column,
+                &empty,
+                h(15),
+                Side::Long,
+                Levels::derived(2),
+                &facts,
+                rows,
+            )
+        };
+        assert_eq!(
+            over(&every).expect("every row"),
+            evaluate_over(
+                &bars,
+                &column,
+                &empty,
+                h(15),
+                Side::Long,
+                Levels::derived(2),
+                &facts
+            )
+        );
+        let refused = over(&[2, 1]).expect_err("an unordered list is refused");
+        assert!(refused.contains("does not follow"), "{refused}");
+    }
+
     /// The same slice with one bar in fifty given a very wide range.
     ///
     /// # Why this fixture has to exist
diff --git a/crates/runner/src/trade.rs b/crates/runner/src/trade.rs
index 0482cd1f..6a72799b 100644
--- a/crates/runner/src/trade.rs
+++ b/crates/runner/src/trade.rs
@@ -790,10 +790,97 @@ pub fn walk_over_from(
         direction,
         facts.exits(),
         facts,
-        first_row,
+        rows_from(column, first_row),
     )
 }
 
+/// [`walk_over`], visiting only the column rows `rows` names (D-4707).
+///
+/// `rows` must be strictly ascending column rows and must hold every row the
+/// mask fires on. Each listed row is still asked [`ConditionMask::hits`], so a
+/// listed row that does not fire is passed over exactly as the full walk
+/// passes it over. The walk's state changes only on a row that fires, and
+/// every bar it records is a slice index read from `sources`, never a
+/// position in the list, so on such a list the answer equals [`walk_over`]'s
+/// exactly. A list that leaves out a firing row answers a different question
+/// and nothing here can see it: the list must hold every firing row by
+/// construction. `crates/cli`'s pool pricing passes the rows of one of the
+/// mask's own bits, which every firing row sets.
+///
+/// # Errors
+///
+/// A row at or past the column's end, or one not strictly after the row
+/// before it. Nothing is walked then.
+///
+/// # Cost
+///
+/// O(|rows|) to check the list, then the walk over it: one mask test per
+/// listed row, and per firing row what [`walk`] states. No row outside the
+/// list is read.
+pub fn walk_over_rows(
+    bars: &[Candle],
+    column: &Column,
+    mask: &ConditionMask,
+    horizon: Horizon,
+    direction: Direction,
+    facts: &SliceFacts,
+    rows: &[usize],
+) -> Result<Trades, String> {
+    let len = column.bits().len().min(column.sources().len());
+    let mut previous: Option<usize> = None;
+    for &row in rows {
+        if row >= len {
+            return Err(format!(
+                "row {row} is past the column's {len} rows; nothing was walked"
+            ));
+        }
+        if let Some(before) = previous.filter(|&before| row <= before) {
+            return Err(format!(
+                "row {row} does not follow row {before}; the rows must ascend strictly, \
+                 and nothing was walked"
+            ));
+        }
+        previous = Some(row);
+    }
+    Ok(walk_core(
+        bars,
+        column,
+        |bits, _| bits.hits(mask),
+        horizon,
+        direction,
+        facts.exits(),
+        facts,
+        rows_listed(column, rows),
+    ))
+}
+
+/// Every column row from `first_row` on, as `(row, bits, source)`, read as
+/// slices (D-1186). A `first_row` past the column yields nothing.
+fn rows_from(
+    column: &Column,
+    first_row: usize,
+) -> impl Iterator<Item = (usize, &ConditionMask, usize)> {
+    column
+        .bits()
+        .get(first_row..)
+        .unwrap_or_default()
+        .iter()
+        .zip(column.sources().get(first_row..).unwrap_or_default())
+        .enumerate()
+        .map(move |(offset, (bits, &signal))| (first_row.saturating_add(offset), bits, signal))
+}
+
+/// Exactly the column rows `rows` names, in its order, as `(row, bits,
+/// source)` (D-4707). [`walk_over_rows`] has checked every row is inside the
+/// column, so none is passed over here.
+fn rows_listed<'c>(
+    column: &'c Column,
+    rows: &'c [usize],
+) -> impl Iterator<Item = (usize, &'c ConditionMask, usize)> {
+    rows.iter()
+        .filter_map(move |&row| Some((row, column.bits().get(row)?, *column.sources().get(row)?)))
+}
+
 /// Price definite expression signals through the same entry, occupancy and
 /// exit state machine as the mask walk. Unknown is never a trade signal.
 ///
@@ -823,7 +910,7 @@ pub fn walk_expression_over(
         direction,
         facts.exits(),
         facts,
-        0,
+        rows_from(column, 0),
     ))
 }
 
@@ -897,7 +984,7 @@ pub fn walk_with(
         direction,
         exits,
         &SliceFacts::of(bars, column),
-        0,
+        rows_from(column, 0),
     )
 }
 
@@ -956,10 +1043,10 @@ fn held_before_hole(
 )]
 #[expect(
     clippy::too_many_arguments,
-    reason = "eight, and the eighth is the D-1186 starting row; the seven before \
-              it are the walk's own inputs and per-slice facts"
+    reason = "eight, and the eighth is the rows it visits (D-1186, D-4707); the \
+              seven before it are the walk's own inputs and per-slice facts"
 )]
-fn walk_core(
+fn walk_core<'r>(
     bars: &[Candle],
     column: &Column,
     mut fires: impl FnMut(&ConditionMask, usize) -> bool,
@@ -967,7 +1054,7 @@ fn walk_core(
     direction: Direction,
     exits: &[Option<SquareOff>],
     facts: &SliceFacts,
-    first_row: usize,
+    rows: impl Iterator<Item = (usize, &'r ConditionMask, usize)>,
 ) -> Trades {
     let h = horizon.as_bars() as usize;
     let mut out = Trades::default();
@@ -975,17 +1062,11 @@ fn walk_core(
     // and a new fill is eligible only strictly after it.
     let mut open_until: Option<usize> = None;
 
-    // FROM `first_row`, AS SLICES (D-1186). Rows before it are not visited;
-    // `index` stays the column row, so `fires` and every recorded bar are
-    // unchanged for the rows that are.
-    let rows = column
-        .bits()
-        .get(first_row..)
-        .unwrap_or_default()
-        .iter()
-        .zip(column.sources().get(first_row..).unwrap_or_default());
-    for (offset, (bits, &signal)) in rows.enumerate() {
-        let index = first_row.saturating_add(offset);
+    // THE ROWS THE CALLER NAMES, IN COLUMN ORDER: every row from a first row
+    // on (`rows_from`, D-1186), or exactly a listed few (`rows_listed`,
+    // D-4707). `index` is the column row either way, so `fires` and every
+    // recorded bar are the same for each row visited.
+    for (index, bits, signal) in rows {
         if !fires(bits, index) {
             continue;
         }
@@ -2016,6 +2097,86 @@ mod tests {
         );
     }
 
+    /// D-4707: a walk over the rows a mask's own bit is set on, or over every
+    /// row, equals the full walk, for every bit alone and paired with its
+    /// neighbour, both sides, and for the empty mask over every row. A list
+    /// out of order, repeating a row or naming a row past the column is
+    /// refused; the column's last row is admitted; an empty list walks
+    /// nothing.
+    #[test]
+    fn a_walk_over_a_superset_of_the_firing_rows_equals_the_full_walk() {
+        let (bars, column) = swept();
+        let facts = super::SliceFacts::of(&bars, &column);
+        let every: Vec<usize> = (0..column.len()).collect();
+        let width = u32::try_from(vocab::table::TABLE.len()).expect("a small table");
+        let (mut compared, mut narrower) = (0, 0);
+        for bit in 0..width {
+            let set: Vec<usize> = column
+                .bits()
+                .iter()
+                .enumerate()
+                .filter(|(_, bits)| bits.get(bit))
+                .map(|(row, _)| row)
+                .collect();
+            for other in [bit, (bit + 1) % width] {
+                let mask = ConditionMask::ZERO.with_bit(bit).with_bit(other);
+                for direction in [Direction::Long, Direction::Short] {
+                    let full = super::walk_over(&bars, &column, &mask, h(5), direction, &facts);
+                    for rows in [&set, &every] {
+                        let listed = super::walk_over_rows(
+                            &bars,
+                            &column,
+                            &mask,
+                            h(5),
+                            direction,
+                            &facts,
+                            rows,
+                        )
+                        .expect("an ascending list inside the column");
+                        assert_eq!(full, listed, "bits {bit}+{other} {direction:?}");
+                    }
+                    compared += usize::from(full.signals > 0);
+                    narrower += usize::from(full.signals > 0 && set.len() < every.len());
+                }
+            }
+        }
+        assert!(compared > 0, "some mask must actually fire");
+        assert!(narrower > 0, "some firing bit must skip rows");
+        let empty = ConditionMask::ZERO;
+        let all = super::walk_over_rows(
+            &bars,
+            &column,
+            &empty,
+            h(5),
+            Direction::Long,
+            &facts,
+            &every,
+        )
+        .expect("every row");
+        assert_eq!(
+            all,
+            super::walk_over(&bars, &column, &empty, h(5), Direction::Long, &facts)
+        );
+        let last = column.len().checked_sub(1).expect("a non-empty column");
+        let walk = |rows: &[usize]| {
+            super::walk_over_rows(&bars, &column, &empty, h(5), Direction::Long, &facts, rows)
+        };
+        for (rows, why) in [
+            (vec![1, 0], "row 0 does not follow row 1"),
+            (vec![3, 3], "row 3 does not follow row 3"),
+            (vec![0, column.len()], "is past the column's"),
+        ] {
+            let refused = walk(&rows).expect_err("a bad list is refused");
+            assert!(refused.contains(why), "{refused}");
+        }
+        assert_eq!(
+            walk(&[last]).expect("the last row is inside").signals,
+            1,
+            "the empty mask fires on the one row listed"
+        );
+        assert_eq!(walk(&[]).expect("no rows"), Trades::default());
+    }
+
     /// THE LOOK-AHEAD D-1182 PINNED, NOW INVERTED — D-1410.
     ///
     /// A [`indicators::column::Sourced::Signal`] slice used to take its cadence
@@ -3123,7 +3284,7 @@ mod tests {
     fn the_per_candidate_walk_derives_nothing_and_sorts_nothing() {
         let source = include_str!("trade.rs");
         let start = source
-            .find("fn walk_core(")
+            .find("fn walk_core<'r>(")
             .expect("the hot path must still be called walk_core");
         let rest = source.get(start..).unwrap_or_default();
         let end = rest
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 2342b948..a0b962ef 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -6806,10 +6806,13 @@ the forward vector and the session table remain linear in B and cannot be O(1)
 total while retaining one answer per input bar.
 
 The larger operations keep their real bounds. A trade walk is linear in the
-rows of the column it walks: `walk_core` visits every row from the first one
-that can fire (D-1186; rows before it are all-zero and cannot) and asks `fires`
-of each, so a row that never fires still costs one test, and the signal count bounds only the work after a row
-fires. (This sentence used to say the walk was linear in the signals it decides,
+rows of the column it walks: `walk_core` visits the rows its caller hands it
+and asks `fires` of each, so a row that never fires still costs one test, and the signal count bounds only the work after a row
+fires. Every entry point but one hands it every row from the first one that
+can fire (`rows_from`, D-1186; rows before it are all-zero and cannot).
+`walk_over_rows` hands it exactly the strictly ascending rows its caller
+lists, which must hold every row the mask fires on (`rows_listed`, D-4707), so
+that walk is linear in the list, plus O(|list|) to check its order. (This sentence used to say the walk was linear in the signals it decides,
 which understated it; D-1204 corrects it.) Excursion/crossing construction reads the relevant
 paths. An exit grid evaluates its bounded configured cells over ordered
 candidates, and chosen-row replay plus persistence is output-sensitive. The
@@ -8786,15 +8789,29 @@ execution series (D-1702): per instrument the loads, column, projection and
 one `SliceFacts`, O(B_sig + B_exec) -- times W + 1 for the column, where W is
 the number of exact-minute-unsourceable days withheld, because pass 1's own
 build reloads both contexts and rebuilds after each one (D-1707, at most 64) --
-and per union candidate one
-`grid::evaluate_over`, which walks every row of the projected column before it
-prices, Θ(B_exec + cells × T). So pass 2 is
-Θ(I × (B_sig + B_exec) + I × U × (B_exec + cells × T)). U is the union of
-every instrument's kept frontier (at most `top` rows a run), so U grows with
-I, up to I × `top`, and pass 2 is up to Θ(I² × top × B_exec) when T is small
-and B_exec large -- the rare-setup case the pool exists for. This said "I × U
-grid evaluations, each O(cells × T)", which left the B_exec walk out
-(R9-cli-o1-1). The fold is one pass over I × U cells. None of this is a rule-4
+then one posting pass over the projected column (D-4707): per row one
+six-word intersection with the bits the union names and one push per bit left
+set, Θ(B_exec + P) time and Θ(P) memory for the P rows posted. Per union
+candidate pass 2 then picks the shortest posting list among the bits the mask
+names, O(BITS) with no row read, and runs one `grid::evaluate_over_rows` over
+it, Θ(R + cells × T) for the list's R rows. So pass 2 is
+Θ(I × (B_sig + B_exec + P) + I × U × (R + cells × T)), with P at most B_exec
+times the named bits and R at most B_exec. R counts the rows the RAREST BIT
+is set on, not the rows the mask fires on: a rare conjunction of common bits
+still walks every row of its rarest bit, and the empty mask, which names no
+bit, keeps `grid::evaluate_over`'s walk of every row. U is the union of every
+instrument's kept frontier (at most `top` rows a run), so U grows with I, up
+to I × `top`, and where R = B_exec pass 2 is still Θ(I² × top × B_exec): the
+lists shorten the walk, they do not change the worst case. The cells are the
+every-row walk's cells exactly, because the walk changes state only on a row
+the mask fires on and each such row is on every one of its bits' lists
+(`pass_two_over_posting_lists_prices_what_every_row_prices`). Until D-4707
+every candidate walked every row of the projected column, Θ(B_exec + cells ×
+T), and pass 2 was Θ(I × (B_sig + B_exec) + I × U × (B_exec + cells × T)), up
+to Θ(I² × top × B_exec) when T is small and B_exec large -- the rare-setup
+case the pool exists for. Before that this said "I × U grid evaluations, each
+O(cells × T)", which left the B_exec walk out (R9-cli-o1-1). The fold is one
+pass over I × U cells. None of this is a rule-4
 primitive, and none of it is constant in I or U. Stated from the code's shape;
 not timed.
 
