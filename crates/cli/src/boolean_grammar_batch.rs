//! Bounded binary batches from the existing complete fixed Boolean grammar.
//! A work allowance pauses the cursor; it never changes the grammar or claims
//! exhaustion. Binary programs avoid the narrower text parser's nesting limit.

use runner::expression::{ENCODED_LEN, Expression};
use vocab::expression_search::{CURSOR_BYTES, Cursor, Step};

const MAGIC: &[u8; 8] = b"BRBGBP01";
/// Fixed batch envelope. `pub(crate)` so admission can enforce the same bound
/// this module does, BEFORE any source is loaded. D-0601.
pub(crate) const HEADER: usize = 64 + 2 * CURSOR_BYTES;

#[derive(Clone, Copy)]
pub(crate) struct Budget {
    pub programs: u64,
    pub nodes: u64,
    pub bytes: u64,
}

pub(crate) struct Batch {
    before: Cursor,
    after: Cursor,
    work_before: u64,
    work_after: u64,
    programs_before: u64,
    budget: Budget,
    programs: Vec<Expression>,
    exhausted: bool,
}

impl Batch {
    pub(crate) fn prepare(
        before: Cursor,
        work_before: u64,
        programs_before: u64,
        budget: Budget,
    ) -> Result<Self, String> {
        if budget.programs == 0 || budget.nodes == 0 {
            return Err("grammar program and node work allowances must be positive".into());
        }
        let maximum = budget
            .programs
            .checked_mul(ENCODED_LEN as u64)
            .and_then(|n| n.checked_add(HEADER as u64))
            .ok_or("grammar batch capacity overflow")?;
        if maximum > budget.bytes {
            return Err("grammar batch exceeds its complete binary byte admission".into());
        }
        let mut programs = Vec::new();
        programs
            .try_reserve_exact(usize::try_from(budget.programs).map_err(display)?)
            .map_err(display)?;
        let mut after = before.clone();
        let mut work_after = work_before;
        let mut exhausted = false;
        while (programs.len() as u64) < budget.programs {
            let used = work_after
                .checked_sub(work_before)
                .ok_or("grammar work counter moved backwards")?;
            let remaining = budget
                .nodes
                .checked_sub(used)
                .ok_or("grammar node work admission exceeded")?;
            match after.advance(remaining, &mut work_after).map_err(debug)? {
                Step::Candidate(program) => programs.push(program),
                Step::Paused => break,
                Step::Exhausted => {
                    exhausted = true;
                    break;
                }
            }
        }
        programs_before
            .checked_add(programs.len() as u64)
            .ok_or("grammar cumulative program count overflow")?;
        Ok(Self {
            before,
            after,
            work_before,
            work_after,
            programs_before,
            budget,
            programs,
            exhausted,
        })
    }

    pub(crate) fn programs(&self) -> &[Expression] {
        &self.programs
    }

    pub(crate) fn next_cursor(&self) -> Cursor {
        self.after.clone()
    }

    pub(crate) fn follows(&self, cursor: &Cursor, work: u64, programs: u64) -> bool {
        // Only encoded progress is authoritative. The cursor's scratch opcode
        // array can retain a previously visited instruction after backtracking;
        // decoding legitimately reconstructs different unused scratch bytes.
        self.before.encode() == cursor.encode()
            && self.work_before == work
            && self.programs_before == programs
    }

    pub(crate) fn uses_budget(&self, programs: u64, nodes: u64) -> bool {
        self.budget.programs == programs && self.budget.nodes == nodes
    }

    pub(crate) const fn work(&self) -> u64 {
        self.work_after
    }

    pub(crate) fn cumulative_programs(&self) -> Result<u64, String> {
        self.programs_before
            .checked_add(self.programs.len() as u64)
            .ok_or_else(|| "grammar cumulative program count overflow".into())
    }

    pub(crate) const fn exhausted(&self) -> bool {
        self.exhausted
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>, String> {
        let length = self
            .programs
            .len()
            .checked_mul(ENCODED_LEN)
            .and_then(|n| n.checked_add(HEADER))
            .ok_or("grammar binary batch length overflow")?;
        let mut raw = Vec::new();
        raw.try_reserve_exact(length).map_err(display)?;
        raw.extend_from_slice(MAGIC);
        for value in [
            self.work_before,
            self.work_after,
            self.programs_before,
            self.budget.programs,
            self.budget.nodes,
            self.programs.len() as u64,
        ] {
            raw.extend_from_slice(&value.to_le_bytes());
        }
        raw.extend_from_slice(&[u8::from(self.exhausted), 0, 0, 0, 0, 0, 0, 0]);
        raw.extend_from_slice(&self.before.encode());
        raw.extend_from_slice(&self.after.encode());
        for program in &self.programs {
            raw.extend_from_slice(&program.encode());
        }
        Ok(raw)
    }

    /// Reproduce the bounded grammar work before trusting counters or cursors.
    /// The caller separately authenticates these bytes and caps replay work.
    pub(crate) fn decode(raw: &[u8], max_bytes: u64, max_nodes: u64) -> Result<Self, String> {
        if raw.len() as u64 > max_bytes || field::<8>(raw, 0)? != *MAGIC {
            return Err("grammar batch format or byte admission refused".into());
        }
        let programs = number(raw, 32)?;
        let nodes = number(raw, 40)?;
        let count = number(raw, 48)?;
        if nodes > max_nodes
            || count > programs
            || count
                .checked_mul(ENCODED_LEN as u64)
                .and_then(|n| n.checked_add(HEADER as u64))
                != Some(raw.len() as u64)
        {
            return Err("grammar batch count or replay work admission refused".into());
        }
        let before = Cursor::decode(&field(raw, 64)?).map_err(debug)?;
        let expected = Self::prepare(
            before,
            number(raw, 8)?,
            number(raw, 24)?,
            Budget {
                programs,
                nodes,
                bytes: max_bytes,
            },
        )?;
        if expected.encode()? != raw {
            return Err("grammar batch differs from exact bounded traversal".into());
        }
        Ok(expected)
    }
}

fn number(raw: &[u8], at: usize) -> Result<u64, String> {
    Ok(u64::from_le_bytes(field(raw, at)?))
}

fn field<const N: usize>(raw: &[u8], at: usize) -> Result<[u8; N], String> {
    raw.get(at..at.saturating_add(N))
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| "grammar batch field is missing".into())
}

fn display(why: impl std::fmt::Display) -> String {
    why.to_string()
}

fn debug(why: impl std::fmt::Debug) -> String {
    format!("{why:?}")
}

#[cfg(test)]
#[path = "boolean_grammar_batch_tests.rs"]
mod tests;
