//! A bounded, non-recursive walk of the thrift footer, run BEFORE `parquet`
//! is allowed to parse it.
//!
//! # Why this exists (CE-12, CE-13, D-1980)
//!
//! `parquet` 59.2 trusts two things a file states about itself, and either one
//! ends the process rather than returning an error:
//!
//! * **a list length.** Its thrift reader calls `Vec::with_capacity(len)` on
//!   the declared size before reading one element. A 21-byte file declaring a
//!   schema of two billion elements asked for 206 GB and the allocator
//!   aborted; [`crate::error::LakeError::FooterUnreadable`] never returned.
//! * **schema depth.** It rebuilds the schema tree recursively with no depth
//!   limit. A million one-child groups (an 8 MB file) overflowed the stack.
//!
//! An abort cannot be caught, so the only honest place to refuse is before
//! the call. This walk reads the compact-protocol footer with an explicit
//! stack, and refuses:
//!
//! * a list, set, map or string whose declared length exceeds the footer bytes
//!   still unread — every element of every thrift type occupies at least one
//!   byte, so a length the bytes cannot hold is a lie, and once the walk has
//!   stepped over every element the length is proven by bytes present;
//! * nesting deeper than [`MAX_DEPTH`] frames;
//! * a schema list longer than [`MAX_SCHEMA_ELEMENTS`] — the root plus the 17
//!   columns of the wider of the two shapes `crate::schema` accepts, so a file
//!   the lake could hold is never refused here, and the tree `parquet` then
//!   recurses over is at most that deep.
//!
//! # What this bounds, stated plainly
//!
//! After the walk, every allocation `parquet` makes from a list length is at
//! most (footer bytes) × (in-memory size of one element). That is a bound by
//! bytes present, not a constant. The footer is inside a file this crate has
//! already read whole, so it is no larger than that.

use crate::error::LakeError;
use crate::schema::Layout;

/// The deepest stack of open structs, lists and maps the walk accepts. A
/// parquet footer nests about nine deep (file, row-group list, row group,
/// column list, column chunk, column metadata, encoding-stats list, one stats
/// entry); 32 leaves room and stops a hostile footer long before any stack.
pub(crate) const MAX_DEPTH: usize = 32;

/// The longest schema list the walk accepts: the root group plus every leaf
/// of the F&O shape, the wider of the two the lake holds.
pub(crate) const MAX_SCHEMA_ELEMENTS: u64 = Layout::Fno.column_count() as u64 + 1;

/// `PAR1` + footer length + `PAR1` around the footer.
const FRAME: usize = 12;

/// Thrift compact type codes.
const BOOL_TRUE: u8 = 1;
const BOOL_FALSE: u8 = 2;
const BYTE: u8 = 3;
const I16: u8 = 4;
const I32: u8 = 5;
const I64: u8 = 6;
const DOUBLE: u8 = 7;
const BINARY: u8 = 8;
const LIST: u8 = 9;
const SET: u8 = 10;
const MAP: u8 = 11;
const STRUCT: u8 = 12;
const UUID: u8 = 13;

/// `FileMetaData.schema` is field 2 of the top-level struct.
const SCHEMA_FIELD: i32 = 2;

/// One open container on the walk's explicit stack.
enum Frame {
    /// A struct, with the last field id read (compact ids are deltas).
    Struct { last: i32 },
    /// A list, set or map: `left` values still to step over. A map's key and
    /// value types alternate, starting with `kinds[0]`.
    Values {
        left: u64,
        kinds: [u8; 2],
        next: usize,
    },
}

/// A cursor over the footer bytes. Every read is checked.
struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn unread(&self) -> u64 {
        self.buf.len().saturating_sub(self.pos) as u64
    }

    fn byte(&mut self) -> Result<u8, LakeError> {
        let b = self
            .buf
            .get(self.pos)
            .copied()
            .ok_or_else(|| refuse("the footer ends inside a value"))?;
        self.pos += 1;
        Ok(b)
    }

    fn skip(&mut self, n: u64) -> Result<(), LakeError> {
        if n > self.unread() {
            return Err(refuse(&format!(
                "a value declares {n} bytes and only {} remain in the footer",
                self.unread()
            )));
        }
        // `n <= unread <= buf.len()`, so this fits a usize and does not wrap.
        self.pos += usize::try_from(n).map_err(|_| refuse("a length does not fit memory"))?;
        Ok(())
    }

    /// An unsigned LEB128 varint of at most ten bytes.
    fn varint(&mut self) -> Result<u64, LakeError> {
        let mut value = 0_u64;
        for shift in (0..70).step_by(7) {
            let b = self.byte()?;
            value |= u64::from(b & 0x7f).checked_shl(shift).unwrap_or(0);
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(refuse("a varint runs past ten bytes"))
    }
}

fn refuse(why: &str) -> LakeError {
    LakeError::FooterUnreadable {
        reason: format!("refused before parsing: {why}"),
    }
}

/// Walks the footer of `file`, whose magics and minimum length the caller
/// has already checked, and refuses anything `parquet` could not survive.
///
/// # Errors
///
/// [`LakeError::FooterUnreadable`], naming what was refused.
pub(crate) fn check(file: &[u8]) -> Result<(), LakeError> {
    let end = file
        .len()
        .checked_sub(8)
        .ok_or_else(|| refuse("the file is too short to hold a footer length"))?;
    let declared = file
        .get(end..end + 4)
        .and_then(|b| <[u8; 4]>::try_from(b).ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| refuse("the file is too short to hold a footer length"))?;
    let room = file.len().saturating_sub(FRAME);
    let len = usize::try_from(declared).unwrap_or(usize::MAX);
    if len > room {
        return Err(refuse(&format!(
            "the footer length {declared} exceeds the {room} bytes between the magics"
        )));
    }
    let footer = file
        .get(end - len..end)
        .ok_or_else(|| refuse("the footer lies outside the file"))?;
    walk(footer)
}

/// The walk itself, over the footer bytes alone.
fn walk(footer: &[u8]) -> Result<(), LakeError> {
    let mut cur = Cursor {
        buf: footer,
        pos: 0,
    };
    let mut stack: Vec<Frame> = Vec::with_capacity(MAX_DEPTH);
    stack.push(Frame::Struct { last: 0 });
    loop {
        let at_top = stack.len() == 1;
        let Some(top) = stack.last_mut() else {
            break;
        };
        let pushed = match top {
            Frame::Struct { last } => {
                let header = cur.byte()?;
                if header == 0 {
                    let _closed = stack.pop();
                    continue;
                }
                let ty = header & 0x0f;
                let delta = header >> 4;
                let id = if delta == 0 {
                    zigzag(cur.varint()?)
                } else {
                    last.saturating_add(i32::from(delta))
                };
                *last = id;
                let opened = value(&mut cur, ty, true)?;
                if at_top && id == SCHEMA_FIELD {
                    schema_bound(opened.as_ref())?;
                }
                opened
            }
            Frame::Values { left, kinds, next } => {
                if *left == 0 {
                    let _closed = stack.pop();
                    continue;
                }
                *left -= 1;
                let ty = kinds.get(*next).copied().unwrap_or(0);
                *next ^= 1;
                value(&mut cur, ty, false)?
            }
        };
        if let Some(frame) = pushed {
            if stack.len() >= MAX_DEPTH {
                return Err(refuse(&format!(
                    "the footer nests deeper than {MAX_DEPTH} levels"
                )));
            }
            stack.push(frame);
        }
    }
    Ok(())
}

/// Refuses a schema list longer than any shape the lake holds.
fn schema_bound(opened: Option<&Frame>) -> Result<(), LakeError> {
    match opened {
        Some(Frame::Values { left, .. }) if *left > MAX_SCHEMA_ELEMENTS => Err(refuse(&format!(
            "the schema declares {left} elements and the lake's widest shape has {MAX_SCHEMA_ELEMENTS}"
        ))),
        _ => Ok(()),
    }
}

fn zigzag(raw: u64) -> i32 {
    // A zigzag i16 in thrift. A value too wide for an i32 cannot equal the
    // schema's id and is stepped over like any other unknown field.
    let magnitude = i64::try_from(raw >> 1).unwrap_or(i64::MAX);
    let signed = if raw & 1 == 0 {
        magnitude
    } else {
        -magnitude - 1
    };
    i32::try_from(signed).unwrap_or(i32::MIN)
}

/// Steps over one value of type `ty`, or returns the frame it opens.
fn value(cur: &mut Cursor<'_>, ty: u8, in_field: bool) -> Result<Option<Frame>, LakeError> {
    match ty {
        BOOL_TRUE | BOOL_FALSE if in_field => Ok(None),
        BOOL_TRUE | BOOL_FALSE | BYTE => cur.byte().map(|_| None),
        I16 | I32 | I64 => cur.varint().map(|_| None),
        DOUBLE => cur.skip(8).map(|()| None),
        UUID => cur.skip(16).map(|()| None),
        BINARY => {
            let n = cur.varint()?;
            cur.skip(n).map(|()| None)
        }
        LIST | SET => list(cur).map(Some),
        MAP => map(cur).map(Some),
        STRUCT => Ok(Some(Frame::Struct { last: 0 })),
        other => Err(refuse(&format!(
            "thrift type {other} is not one parquet writes"
        ))),
    }
}

fn element(kind: u8) -> Result<u8, LakeError> {
    if matches!(kind, BOOL_TRUE..=UUID) {
        Ok(kind)
    } else {
        Err(refuse(&format!(
            "list element type {kind} is not a thrift type"
        )))
    }
}

fn admit(cur: &Cursor<'_>, values: u64) -> Result<(), LakeError> {
    if values > cur.unread() {
        return Err(refuse(&format!(
            "a container declares {values} values and only {} bytes remain in the footer",
            cur.unread()
        )));
    }
    Ok(())
}

fn list(cur: &mut Cursor<'_>) -> Result<Frame, LakeError> {
    let header = cur.byte()?;
    if header == 0 {
        // Some writers emit an empty list as a bare zero byte.
        return Ok(Frame::Values {
            left: 0,
            kinds: [BYTE, BYTE],
            next: 0,
        });
    }
    let kind = element(header & 0x0f)?;
    let short = u64::from(header >> 4);
    let left = if short == 15 { cur.varint()? } else { short };
    admit(cur, left)?;
    Ok(Frame::Values {
        left,
        kinds: [kind, kind],
        next: 0,
    })
}

fn map(cur: &mut Cursor<'_>) -> Result<Frame, LakeError> {
    let pairs = cur.varint()?;
    if pairs == 0 {
        return Ok(Frame::Values {
            left: 0,
            kinds: [BYTE, BYTE],
            next: 0,
        });
    }
    let kv = cur.byte()?;
    let kinds = [element(kv >> 4)?, element(kv & 0x0f)?];
    let left = pairs.saturating_mul(2);
    admit(cur, left)?;
    Ok(Frame::Values {
        left,
        kinds,
        next: 0,
    })
}

#[cfg(test)]
#[path = "footer_tests.rs"]
mod tests;
