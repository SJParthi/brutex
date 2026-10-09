diff --git a/crates/runner/src/grid.rs b/crates/runner/src/grid.rs
index bc86aeec..fef05a6a 100644
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
@@ -7144,6 +7180,80 @@ mod tests {
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
index 0482cd1f..f0a6a1fa 100644
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
 
@@ -956,8 +1043,8 @@ fn held_before_hole(
 )]
 #[expect(
     clippy::too_many_arguments,
-    reason = "eight, and the eighth is the D-1186 starting row; the seven before \
-              it are the walk's own inputs and per-slice facts"
+    reason = "eight, and the eighth is the rows it visits (D-1186, D-4707); the \
+              seven before it are the walk's own inputs and per-slice facts"
 )]
 fn walk_core(
     bars: &[Candle],
@@ -967,7 +1054,7 @@ fn walk_core(
     direction: Direction,
     exits: &[Option<SquareOff>],
     facts: &SliceFacts,
-    first_row: usize,
+    rows: impl Iterator<Item = (usize, &'_ ConditionMask, usize)>,
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
