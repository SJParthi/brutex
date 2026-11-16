//! `WORDS-STD-v1.tsv` (4.3, section 7): one table of templates; no literal month or year in any template (V-W1); every month a
//! line names comes from a placeholder rendered from the rows in use. `fmt_rs` is lifted from round 3's `common.rs:131-148`.

use crate::windows::has_literal as literal_month_or_year;
use crate::{WfErr, add};

/// Every word id the standard table must hold.
pub const WORD_IDS: &[&str] = &["W01", "W02", "W03", "W04", "W05", "W06", "W07", "W07c", "W07w", "W08", "W08s", "W08v", "W09", "W10", "W11", "W12", "W12b", "W13", "W14", "W14K", "W15", "W16", "W16n", "W17", "W17b", "W18", "W19", "W20", "W21", "W22", "W23", "W24", "W25", "W26", "W27"];

/// The word table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
    t: Vec<(String, String)>,
    /// The identity hash of the file.
    pub ident: u64,
}

impl Words {
    /// Parses and validates a WORDS file.
    ///
    /// # Errors
    /// `BadRow(T, row, rule)`: 1 a literal month or year (V-W1), 2 a missing id, 3 a duplicate id, 4 the header or a bad row.
    pub fn from_text(text: &str) -> Result<Self, WfErr> {
        let bad = |row: usize, rule: u8| WfErr::BadRow { file: b'T', row: u32::try_from(row).unwrap_or(u32::MAX), rule };
        let mut lines = text.lines().filter(|l| !l.is_empty());
        if lines.next() != Some("id\ttemplate\tformatter") {
            return Err(bad(0, 4));
        }
        let mut t: Vec<(String, String)> = Vec::new();
        for (i, l) in lines.enumerate() {
            let c: Vec<&str> = l.split('\t').collect();
            let (Some(id), Some(tpl), Some(_), None) = (c.first(), c.get(1), c.get(2), c.get(3)) else { return Err(bad(i + 1, 4)) };
            if literal_month_or_year(tpl) {
                return Err(bad(i + 1, 1));
            }
            if t.iter().any(|(a, _)| a == id) {
                return Err(bad(i + 1, 3));
            }
            t.push(((*id).to_owned(), (*tpl).to_owned()));
        }
        if WORD_IDS.iter().any(|id| !t.iter().any(|(a, _)| a == id)) {
            return Err(bad(0, 2));
        }
        Ok(Self { t, ident: crate::text_hash(text) })
    }
    /// The template of `id`.
    #[must_use]
    pub fn template(&self, id: &str) -> Option<&str> {
        self.t.iter().find(|(a, _)| a == id).map(|(_, b)| b.as_str())
    }
    /// Renders `id` with its placeholders.
    ///
    /// # Errors
    /// `BadRow(T, 0, 9)` an unknown id or a placeholder with no value.
    pub fn fill(&self, id: &str, vals: &[(&str, &str)]) -> Result<String, WfErr> {
        fill_template(self.template(id).ok_or(WfErr::BadRow { file: b'T', row: 0, rule: 9 })?, vals)
    }
}

/// Renders `{name}` placeholders of a template (a word, or a row's `owner_words`).
///
/// # Errors
/// `BadRow(T, 0, 9)` a placeholder with no value.
pub fn fill_template(tpl: &str, vals: &[(&str, &str)]) -> Result<String, WfErr> {
    let mut out = String::with_capacity(tpl.len() + 64);
    let mut rest = tpl;
    while let Some(i) = rest.find('{') {
        let (head, tail) = rest.split_at(i);
        out.push_str(head);
        let Some(j) = tail.find('}') else {
            out.push_str(tail);
            return Ok(out);
        };
        let name = tail.get(1..j).unwrap_or("");
        let v = vals.iter().find(|(k, _)| *k == name).map(|(_, v)| *v).ok_or(WfErr::BadRow { file: b'T', row: 0, rule: 9 })?;
        out.push_str(v);
        rest = tail.get(j + 1..).unwrap_or("");
    }
    out.push_str(rest);
    Ok(out)
}

/// Rupees with two decimals and Indian digit grouping, from paisa, by integer division (round 3's `fmt_rs`).
#[must_use]
pub fn fmt_rs(paisa: i64) -> String {
    let neg = paisa < 0;
    let a = i128::from(paisa).abs();
    let digits = (a / 100).to_string();
    let p = a % 100;
    let n = digits.len();
    let mut out = String::with_capacity(n + n / 2 + 8);
    for (i, ch) in digits.chars().enumerate() {
        let rest = n - i;
        out.push(ch);
        if rest >= 4 && (rest - 4) % 2 == 0 {
            out.push(',');
        }
    }
    format!("{}Rs {out}.{p:02}", if neg { "-" } else { "" })
}

/// A basis-point share as a percent with two decimals (`12.34%`).
#[must_use]
pub fn fmt_pct(bp: i64) -> String {
    let neg = bp < 0;
    let a = i128::from(bp).abs();
    format!("{}{}.{:02}%", if neg { "-" } else { "" }, a / 100, a % 100)
}

/// A share of `part` in `whole`, in bp, floored (None when `whole` <= 0).
#[must_use]
pub fn share_bp(part: i64, whole: i64) -> Option<i64> {
    if whole <= 0 {
        return None;
    }
    let q = i128::from(part) * 10_000;
    let w = i128::from(whole);
    let d = q.div_euclid(w);
    i64::try_from(d).ok()
}

/// A checked sum of a slice (the luck line's top-k sums).
///
/// # Errors
/// `Overflow`.
pub fn sum(xs: &[i64]) -> Result<i64, WfErr> {
    xs.iter().try_fold(0i64, |a, b| add(a, *b))
}
