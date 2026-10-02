//! Fixed-width text tables whose columns can never touch. D-1420.
//!
//! # The defect this exists to close
//!
//! Every terminal table in this crate was written as one `format!` string of
//! adjacent width specifiers -- `{:<5}{:>10}{:>9}{:>14}` -- with no literal
//! separator between the columns. Padding is the only thing that keeps two
//! figures apart, so a figure that fills its width exactly runs into its
//! neighbour, and a wider one (a money column at `i64::MIN` paisa renders as
//! `-92,233,720,368,547,758.08`, 26 characters) both runs into it and shifts
//! every column after it off its header. `-1,23,45,678.90` beside `12` reads
//! as one number. That is a wrong answer that looks like an answer (`CLAUDE.md`
//! §4).
//!
//! # The rule this module enforces
//!
//! The widths written at each call site are kept as MINIMUMS, so a table whose
//! figures fit renders byte for byte as it always did. A column grows only when
//! a cell (or its header) would otherwise touch the column beside it, and it
//! grows for every row at once, so the header stays over its figures:
//!
//! - a right-aligned column after the first keeps at least one space on its
//!   left;
//! - a left-aligned column before the last keeps at least one space on its
//!   right;
//! - a right-aligned column followed directly by a left-aligned one, which
//!   neither rule separates, gets one literal space between them.
//!
//! Widths are counted in characters, the unit `format!` pads in, so a `·` or a
//! `—` in a cell does not desynchronise a row from its header.
//!
//! A table whose rows were printed as each was computed (`descend`), or whose
//! header sat apart from rows written between other reports (`ledger-v6`), is
//! now laid out where every row it aligns is in hand, so no width has to be
//! guessed from a type's extreme.

/// Which edge of its column a cell is flush against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Align {
    /// `{:<N}`: text, names, labels.
    Left,
    /// `{:>N}`: every figure.
    Right,
}

/// One column: its alignment, its minimum width, and any literal spaces that
/// precede it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Col {
    /// Which edge the cell is flush against.
    pub(crate) align: Align,
    /// The width written at the call site; never narrowed.
    pub(crate) min: usize,
    /// Literal spaces written before this column, for tables that already
    /// separated their columns with `"  "`.
    pub(crate) gap: usize,
}

/// A left-aligned column at least `min` wide.
pub(crate) const fn left(min: usize) -> Col {
    Col {
        align: Align::Left,
        min,
        gap: 0,
    }
}

/// A right-aligned column at least `min` wide.
pub(crate) const fn right(min: usize) -> Col {
    Col {
        align: Align::Right,
        min,
        gap: 0,
    }
}

impl Col {
    /// The same column with `gap` literal spaces before it.
    pub(crate) const fn after(self, gap: usize) -> Self {
        Self { gap, ..self }
    }
}

fn chars(text: &str) -> usize {
    text.chars().count()
}

/// The spaces a column needs beyond its widest cell so it cannot touch a
/// neighbour. A literal gap before the column already separates it on that
/// side.
fn margin(column: Col, first: bool, next: Option<&Col>) -> usize {
    match column.align {
        Align::Right => usize::from(!first && column.gap == 0),
        Align::Left => usize::from(next.is_some_and(|next| next.gap == 0)),
    }
}

/// The width every column renders at for `rows` (the header is a row).
///
/// A row may carry fewer cells than there are columns -- a `REFUSED` row names
/// its first cell and then a sentence -- and its cells still count.
fn widths(columns: &[Col], rows: &[Vec<String>]) -> Vec<usize> {
    columns
        .iter()
        .enumerate()
        .map(|(index, &column)| {
            let widest = rows
                .iter()
                .filter_map(|row| row.get(index))
                .map(|cell| chars(cell))
                .max()
                .unwrap_or(0);
            let needed = margin(column, index == 0, columns.get(index.saturating_add(1)));
            column.min.max(widest.saturating_add(needed))
        })
        .collect()
}

/// One row at `widths`, without indentation or a trailing newline. A short row
/// renders only the cells it has, each padded to its column.
fn line(columns: &[Col], widths: &[usize], cells: &[String]) -> String {
    let mut out = String::new();
    let mut previous: Option<Align> = None;
    for (index, cell) in cells.iter().enumerate() {
        let (Some(column), Some(&width)) = (columns.get(index), widths.get(index)) else {
            // More cells than columns is a call-site bug; it is rendered after
            // a space rather than dropped, so it shows instead of vanishing.
            out.push(' ');
            out.push_str(cell);
            continue;
        };
        out.extend(std::iter::repeat_n(' ', column.gap));
        if column.gap == 0 && column.align == Align::Left && previous == Some(Align::Right) {
            out.push(' ');
        }
        previous = Some(column.align);
        let pad = width.saturating_sub(chars(cell));
        match column.align {
            Align::Left => {
                out.push_str(cell);
                out.extend(std::iter::repeat_n(' ', pad));
            }
            Align::Right => {
                out.extend(std::iter::repeat_n(' ', pad));
                out.push_str(cell);
            }
        }
    }
    out
}

/// A table laid out at one set of widths.
pub(crate) struct Laid {
    /// The header line.
    pub(crate) header: String,
    /// One line per body row, in order.
    pub(crate) rows: Vec<String>,
}

/// `header` then `body`, laid out together so the header sits over the widest
/// figure and no two columns touch on any row. A body row may be short: a
/// refusal carries only its first cell, which still counts toward that
/// column's width, so the sentence the caller writes after it cannot touch it.
pub(crate) fn with_header(columns: &[Col], header: &[&str], body: Vec<Vec<String>>) -> Laid {
    let mut all = Vec::with_capacity(body.len().saturating_add(1));
    all.push(header.iter().map(|&head| head.to_owned()).collect());
    all.extend(body);
    let widths = widths(columns, &all);
    let mut lines = all.iter().map(|row| line(columns, &widths, row));
    Laid {
        header: lines.next().unwrap_or_default(),
        rows: lines.collect(),
    }
}

/// Checks that `row` is separated into exactly the columns of `header` and that
/// each of its cells shares its header's edge: the left edge for a left column
/// and the right edge for a right column. Cells must hold no internal spaces,
/// and any words after the columns must be as many in the row as in the header.
#[cfg(test)]
pub(crate) fn assert_under(header: &str, row: &str, aligns: &[Align]) -> Result<(), String> {
    fn spans(text: &str) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut start = None;
        for (index, ch) in text.chars().enumerate() {
            match (ch == ' ', start) {
                (false, None) => start = Some(index),
                (true, Some(from)) => {
                    out.push((from, index));
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(from) = start {
            out.push((from, text.chars().count()));
        }
        out
    }
    let heads = spans(header);
    let cells = spans(row);
    // A one-word tail after the last column (`conditions`, a name) is allowed
    // when the row carries the same count; only the columns are compared.
    if heads.len() < aligns.len() || cells.len() != heads.len() {
        return Err(format!(
            "{} header words and {} row words for {} columns -- two columns touch\n{header}\n{row}",
            heads.len(),
            cells.len(),
            aligns.len()
        ));
    }
    for (index, ((align, head), cell)) in aligns.iter().zip(&heads).zip(&cells).enumerate() {
        let shares_edge = match align {
            Align::Left => head.0 == cell.0,
            Align::Right => head.1 == cell.1,
        };
        if !shares_edge {
            return Err(format!(
                "column {index} ({align:?}) is off its header\n{header}\n{row}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{Align, assert_under, left, line, right, widths, with_header};

    /// Every row, the header first, as `with_header` lays them out.
    fn lay_out(columns: &[super::Col], rows: &[Vec<String>]) -> Vec<String> {
        let (header, body) = rows.split_first().expect("a header row");
        let header: Vec<&str> = header.iter().map(String::as_str).collect();
        let laid = with_header(columns, &header, body.to_vec());
        std::iter::once(laid.header).chain(laid.rows).collect()
    }

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|&c| c.to_owned()).collect()
    }

    #[test]
    fn figures_that_fit_render_exactly_as_the_format_string_did() {
        let columns = [left(5), right(10), right(9)];
        let rows = [row(&["rank", "hits", "n"]), row(&["1", "123", "45"])];
        let laid = lay_out(&columns, &rows);
        assert_eq!(laid[0], format!("{:<5}{:>10}{:>9}", "rank", "hits", "n"));
        assert_eq!(laid[1], format!("{:<5}{:>10}{:>9}", "1", "123", "45"));
    }

    #[test]
    fn a_figure_that_fills_or_overflows_its_width_widens_the_column_for_every_row() {
        let columns = [left(5), right(10), right(9)];
        let money = "-92,233,720,368,547,758.08";
        let rows = [
            row(&["rank", "hits", "n"]),
            row(&["12345", "1234567890", money]),
            row(&["1", "2", "3"]),
        ];
        let laid = lay_out(&columns, &rows);
        let aligns = [Align::Left, Align::Right, Align::Right];
        for body in &laid[1..] {
            assert_under(&laid[0], body, &aligns).expect("separated and aligned");
        }
        // The old format string touched on exactly this row.
        let old = format!("{:<5}{:>10}{:>9}", "12345", "1234567890", money);
        assert!(assert_under(&laid[0], &old, &aligns).is_err(), "{old}");
    }

    #[test]
    fn a_right_column_then_a_left_column_gets_one_literal_space() {
        let columns = [right(3), left(3), right(3)];
        let rows = [row(&["abc", "def", "g"])];
        assert_eq!(lay_out(&columns, &rows)[0], "abc def   g");
    }

    #[test]
    fn a_literal_gap_already_separates_and_adds_nothing() {
        let columns = [right(4), right(4).after(2)];
        let rows = [row(&["rank", "1234"])];
        assert_eq!(lay_out(&columns, &rows)[0], "rank  1234");
    }

    #[test]
    fn a_short_row_counts_and_renders_only_its_cells() {
        let columns = [left(4), right(4)];
        let rows = [row(&["abcdef"]), row(&["a", "b"])];
        let w = widths(&columns, &rows);
        assert_eq!(w, vec![7, 4]);
        assert_eq!(line(&columns, &w, &rows[0]), "abcdef ");
        assert_eq!(line(&columns, &w, &row(&["a", "b", "c"])), "a         b c");
    }

    #[test]
    fn multibyte_cells_are_counted_in_characters() {
        let columns = [left(2), right(3)];
        let rows = [row(&["·", "—"])];
        assert_eq!(lay_out(&columns, &rows)[0], "·   —");
    }

    #[test]
    fn empty_input_is_the_minimums() {
        assert_eq!(widths(&[left(3), right(7)], &[]), vec![3, 7]);
        let laid = with_header(&[left(3), right(7)], &[], Vec::new());
        assert_eq!(laid.header, "");
        assert!(laid.rows.is_empty());
    }

    #[test]
    fn the_checker_refuses_touching_and_misaligned_rows() {
        let aligns = [Align::Left, Align::Right];
        assert!(assert_under("a     b", "x     y", &aligns).is_ok());
        assert!(assert_under("a     b", "x    yy", &aligns).is_ok());
        assert!(assert_under("a     b", "xxxxxxy", &aligns).is_err());
        assert!(assert_under("a     b", "x      y", &aligns).is_err());
        assert!(assert_under("a     b", " x     y", &aligns).is_err());
    }
}
