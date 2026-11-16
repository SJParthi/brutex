//! ONE fail-closed RULES parser for the fill (design 3, `rules::Cfg`): every `fill_*` row of a lane's RULES file. A `fill_*` key it does not
//! know, a repeated key, a missing required key or a value outside its set REFUSES (no default is ever guessed). Rows of other prefixes
//! belong to the lane and are ignored here.
//!
//! | key | required | values |
//! |---|---|---|
//! | `fill_law` | yes | `req70-2s` (THE STANDARD) or `req70-2s-cap` (the comparison "with money cap") |
//! | `fill_dial_bp` | yes | comma list of basis points, strictly rising, each 1..=10000, at most 8 (REQ 70b) |
//! | `fill_timing` | yes | `bell` (REQ 70c: the deciding candle's end) |
//! | `fill_crate_version` | yes | this crate's [`crate::VERSION`] |
//! | `fill_session_table_sha256` | no | 64 lowercase hex (the dated session table the host pins, S3) |

use crate::{Dial, FillErr, LAW_STD, LAW_STD_CAP, VERSION};

/// The fill's ruled settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cfg {
    /// Whether the lane runs `req70-2s-cap` (the comparison) instead of the standard.
    pub cap_law: bool,
    /// The dial.
    pub dial: Dial,
    /// The pinned session table's sha256, when given (64 hex bytes).
    pub session_table_sha256: Option<[u8; 64]>,
}

const KNOWN: [&str; 5] = ["fill_law", "fill_dial_bp", "fill_timing", "fill_crate_version", "fill_session_table_sha256"];

impl Cfg {
    /// The settings of `rows` (key, value), fail closed.
    ///
    /// # Errors
    /// [`FillErr::RulesInvalid`] for an unknown or repeated `fill_*` key, a missing required key or a value outside its set;
    /// [`FillErr::DialInvalid`] for a dial that does not rise inside 1..=10000 bp.
    pub fn from_rows(rows: &[(&str, &str)]) -> Result<Self, FillErr> {
        let mut seen: [Option<&str>; 5] = [None; 5];
        for &(k, v) in rows {
            if !k.starts_with("fill_") {
                continue;
            }
            let pos = KNOWN.iter().position(|&x| x == k).ok_or(FillErr::RulesInvalid)?;
            let slot = seen.get_mut(pos).ok_or(FillErr::RulesInvalid)?;
            if slot.is_some() {
                return Err(FillErr::RulesInvalid);
            }
            *slot = Some(v.trim());
        }
        let get = |i: usize| seen.get(i).copied().flatten();
        let cap_law = match get(0).ok_or(FillErr::RulesInvalid)? {
            x if x == LAW_STD => false,
            x if x == LAW_STD_CAP => true,
            _ => return Err(FillErr::RulesInvalid),
        };
        let mut lv = Vec::new();
        for part in get(1).ok_or(FillErr::RulesInvalid)?.split(',') {
            let p = part.trim();
            if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                return Err(FillErr::RulesInvalid);
            }
            lv.push(p.parse::<u16>().map_err(|_| FillErr::RulesInvalid)?);
        }
        let dial = Dial::new(&lv)?;
        if get(2).ok_or(FillErr::RulesInvalid)? != "bell" {
            return Err(FillErr::RulesInvalid);
        }
        if get(3).ok_or(FillErr::RulesInvalid)? != VERSION {
            return Err(FillErr::RulesInvalid);
        }
        let session_table_sha256 = match get(4) {
            None => None,
            Some(h) => {
                let b = h.as_bytes();
                if b.len() != 64 || !b.iter().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c)) {
                    return Err(FillErr::RulesInvalid);
                }
                let mut a = [0_u8; 64];
                a.copy_from_slice(b);
                Some(a)
            }
        };
        Ok(Self { cap_law, dial, session_table_sha256 })
    }
}
