//! F01 `universe::load`: the names of the universe list in byte order, with their
//! ordinals, the one series alias and a 32-bit id (PREVIEW-SPEC section 3.3).

use std::collections::BTreeMap;

/// The one listed name read from another name's stored series.
pub const ALIAS: (&str, &str) = ("ICICILIFE", "ICICIPRULI");

/// Why a universe list was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No name at all.
    Empty,
    /// A byte-order mark, a carriage return or a byte that is not UTF-8.
    Encoding,
    /// An empty line before the end (line number, 1-based).
    EmptyLine(usize),
    /// A name with a byte outside `A-Z`, `0-9`, `&` and `-` (line number).
    BadName(usize),
    /// The same name twice (both line numbers).
    Duplicate(usize, usize),
    /// Two names on one stored series (the alias and its target both listed).
    AliasAndTarget,
    /// Two names with one 32-bit id (both names).
    IdCollision(String, String),
}

/// One loaded name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    /// The listed spelling.
    pub name: String,
    /// The stored series it reads.
    pub series: String,
    /// Its place in byte order, from 0.
    pub ordinal: usize,
    /// FNV-1a of the series spelling, 32 bits.
    pub id: u32,
}

/// FNV-1a over bytes, 32 bits.
#[must_use]
pub fn fnv1a32(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in bytes {
        h ^= u32::from(*b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

fn name_ok(line: &str) -> bool {
    !line.is_empty()
        && line
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'&' || b == b'-')
}

/// Loads a universe list with the real id function.
///
/// # Errors
/// Every [`Refusal`] the text earns; a duplicate is named by both line numbers.
pub fn load(text: &[u8]) -> Result<Vec<Name>, Vec<Refusal>> {
    load_with(text, fnv1a32)
}

/// Loads a universe list with a given id function (a test seam for collisions).
///
/// # Errors
/// As [`load`]; when an id collision and a duplicate spelling are both present, both
/// are listed, the collision first.
pub fn load_with(text: &[u8], id: fn(&[u8]) -> u32) -> Result<Vec<Name>, Vec<Refusal>> {
    if text.starts_with(&[0xEF, 0xBB, 0xBF]) || text.contains(&b'\r') {
        return Err(vec![Refusal::Encoding]);
    }
    let Ok(body) = std::str::from_utf8(text) else {
        return Err(vec![Refusal::Encoding]);
    };
    let body = body.strip_suffix('\n').unwrap_or(body);
    if body.is_empty() {
        return Err(vec![Refusal::Empty]);
    }
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    let mut duplicate = None;
    for (i, line) in body.split('\n').enumerate() {
        let n = i + 1;
        if line.is_empty() {
            return Err(vec![Refusal::EmptyLine(n)]);
        }
        if !name_ok(line) {
            return Err(vec![Refusal::BadName(n)]);
        }
        if let Some(first) = seen.insert(line, n) {
            duplicate = duplicate.or(Some(Refusal::Duplicate(first, n)));
        }
    }
    if seen.contains_key(ALIAS.0) && seen.contains_key(ALIAS.1) {
        return Err(vec![Refusal::AliasAndTarget]);
    }
    let mut refusals = Vec::new();
    let mut ids: BTreeMap<u32, &str> = BTreeMap::new();
    let mut out = Vec::with_capacity(seen.len());
    for (ordinal, name) in seen.keys().enumerate() {
        let series = if *name == ALIAS.0 { ALIAS.1 } else { name };
        let h = id(series.as_bytes());
        if let Some(other) = ids.insert(h, name) {
            refusals.push(Refusal::IdCollision((*other).to_owned(), (*name).to_owned()));
        }
        out.push(Name { name: (*name).to_owned(), series: series.to_owned(), ordinal, id: h });
    }
    refusals.extend(duplicate);
    if refusals.is_empty() { Ok(out) } else { Err(refusals) }
}

#[cfg(test)]
#[path = "universe_tests.rs"]
mod tests;
