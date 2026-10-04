//! **Reading committed Selection V6 records for display.** D-1578.
//!
//! # Why this exists
//!
//! `ledger-v6` commits one Selection V6 block per rung under
//! `ROOT/selection/<rung>/global-selection-v6.bin`, and until this reader the
//! only way to see one was the text that verb printed while it ran. No `api`
//! route and no page read the file (audit-20261003 gaps-10), while
//! `docs/07-plan.md` §11 order 5 asks that the API and dashboard expose the
//! same receipt identities, counters and decisions the CLI does.
//!
//! # What a read here is, and what it is not
//!
//! Every block is checked exactly as the commit door checks it: magic,
//! version, semantic identity and completion seal ([`super::verify_block`]),
//! through the same no-follow, single-link open ([`super::open`]) under a
//! shared lock, with the file's generation unchanged across the read. The
//! decode then follows the encoder field for field and refuses anything the
//! encoder cannot write: a winner count above 25, a Top-10 count that is not
//! the prefix, a family or direction code outside its enum, or a nonzero byte
//! in the unused tail.
//!
//! It does NOT re-authenticate the upstream Execution V4 / Population V6
//! chain. That is what [`super::CommittedStoredSelectionV6`] does, and only
//! the commit door can build one. A record read here is a sealed stored
//! record, not a fresh Selection V6 capability, and the API says so.
//!
//! # Equities
//!
//! Selection V6 has two families, NIFTY (code 1) and BANKNIFTY (code 2), and
//! `CLAUDE.md` §1 says no equity result may enter it until a charter-sourced
//! equity charge stack exists. A block naming any other family code is refused
//! by name, never shown under a guessed label; [`selection_v6_family`] refuses
//! an equity asked for by name with the same sentence.

use std::io::Read as _;
use std::io::Seek as _;
use std::path::Path;

use super::{BLOCK_BYTES, FILE_NAME, SEAL_AT, SELECTION_V6_BLOCK_BYTES};

/// The sentence every equity refusal on this surface carries.
pub const SELECTION_V6_EQUITY_REFUSAL: &str = "Selection V6 holds the NIFTY and BANKNIFTY \
     families only. No equity result may enter Selection V6 or execution authority until a \
     charter-sourced equity charge stack exists (CLAUDE.md §1, D-0509, D-0681).";

/// The two Selection V6 families by their stored code.
const FAMILIES: [(u64, &str); 2] = [(1, "NIFTY"), (2, "BANKNIFTY")];

/// The family a word names, or the refusal for a word that is not one.
///
/// A cash equity on the engine surface is refused with
/// [`SELECTION_V6_EQUITY_REFUSAL`], so a request for one is told why rather
/// than handed an empty list it could read as "no winner".
///
/// # Errors
///
/// Any word that is not `NIFTY` or `BANKNIFTY`.
pub fn selection_v6_family(word: &str) -> Result<&'static str, String> {
    for (_, family) in FAMILIES {
        if family == word {
            return Ok(family);
        }
    }
    if crate::stored::any_cash_equity([word]) {
        return Err(format!(
            "{word} is a cash equity. {SELECTION_V6_EQUITY_REFUSAL}"
        ));
    }
    Err(format!(
        "{word} is not a Selection V6 family. {SELECTION_V6_EQUITY_REFUSAL}"
    ))
}

/// One ranked winner exactly as its block stores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredSelectionV6Winner {
    /// Zero-based rank in the stored Top-25.
    pub rank: u32,
    /// `NIFTY` or `BANKNIFTY`; no other family decodes.
    pub family: &'static str,
    /// The candidate's strategy digest.
    pub strategy_digest: [u8; 32],
    /// The Execution V4 disposition this winner retains.
    pub disposition_id: [u8; 32],
    /// The selected exit's identity.
    pub selected_exit_digest: [u8; 32],
    /// The disposition's position in the whole execution ledger.
    pub global_sequence: u64,
    /// The disposition's position within its family.
    pub family_sequence: u64,
    /// The shared ranking's score.
    pub score: u64,
    /// `long` or `short`.
    pub direction: &'static str,
    /// The combination's six condition-mask words.
    pub mask_words: [u64; 6],
    /// Maximum drawdown, paisa.
    pub drawdown: u64,
    /// Largest single loss, paisa.
    pub worst_loss: u64,
    /// Losing trades per million.
    pub losing_rate_ppm: u64,
    /// Losing trades.
    pub losing_trades: u64,
    /// Winning trades.
    pub winning_trades: u64,
    /// Winning trades per million.
    pub win_rate_ppm: u64,
    /// Mean win, paisa.
    pub average_win: u64,
    /// Mean loss, paisa.
    pub average_loss: u64,
    /// The ranking's assurance figure, per million.
    pub assurance_ppm: u64,
    /// Pessimistic total, paisa.
    pub pessimistic_profit: i64,
    /// `None` is an undefined ratio, distinct from a measured zero.
    pub loss_ratio_ppm: Option<u64>,
    /// `None` is an undefined ratio, distinct from a measured zero.
    pub reward_to_risk_ppm: Option<u64>,
}

/// One family's terminal envelope inside a block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredSelectionV6Family {
    /// `NIFTY` or `BANKNIFTY`.
    pub family: &'static str,
    /// `evaluated`, `insufficient-for-cscv` or `naturally-extinct`.
    pub terminal: &'static str,
    /// Candidates the family produced.
    pub candidate_count: u64,
    /// Candidates evaluated.
    pub evaluated_count: u64,
    /// Decisions recorded.
    pub decision_count: u64,
}

/// One sealed Selection V6 block, decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredSelectionV6Record {
    /// The block's semantic identity.
    pub identity: [u8; 32],
    /// The rung, in seconds.
    pub rung_seconds: u64,
    /// The holding period, in execution bars.
    pub horizon_bars: u64,
    /// The retained Population V6 identity.
    pub population_id: [u8; 32],
    /// The retained Execution V4 completion identity.
    pub execution_completion_id: [u8; 32],
    /// The ranking policy's digest.
    pub policy_digest: [u8; 32],
    /// Execution rows the selection ranked.
    pub row_count: u64,
    /// Rows execution authorized.
    pub authorized_count: u64,
    /// Rows the execution policy refused.
    pub policy_refused_count: u64,
    /// NIFTY's envelope, then BANKNIFTY's.
    pub families: [StoredSelectionV6Family; 2],
    /// Candidates the ranking considered.
    pub considered: u64,
    /// Candidates admitted.
    pub admitted: u64,
    /// Candidates refused.
    pub refused: u64,
    /// Candidates unmeasured.
    pub unmeasured: u64,
    /// The stored Top-25, in rank order; the first ten are the Top-10.
    pub winners: Vec<StoredSelectionV6Winner>,
}

/// What one rung's directory holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoredSelectionV6Rung {
    /// No Selection V6 file at the rung's path. The path is named.
    Absent(String),
    /// One page of the file's blocks, in append order.
    Records {
        /// Whole blocks the file holds, from its length alone.
        total: u64,
        /// The first block shown, counted from zero.
        from: u64,
        /// Blocks `from..from + records.len()`; empty past the end.
        records: Vec<StoredSelectionV6Record>,
    },
    /// The file exists and could not be read whole; the reason is named and
    /// no block from it is shown.
    Refused(String),
}

/// Every intraday rung's Selection V6 records beneath a `ledger-v6` ROOT, in
/// the ledger's rung order: one page of at most `limit` blocks per rung,
/// starting at block `from`.
///
/// **A page costs O(limit), whatever the file holds.** Blocks have a fixed
/// stride, so block `from` is one seek away and the file's block count is its
/// length. The reader once read every block and refused a file holding more
/// than its cap, so a rung's 65th commit made that rung unreadable for good
/// (Rust and O(1) sweep OS-5, D-2303). Proved through the API that pages it,
/// `api::selectionv6json::tests::the_page_selectors_are_bounded_and_strict`
/// (`api::tests::the_page_selectors_are_bounded_and_strict` to gate 12), and
/// by the paging assertions in `cli::selection_v6::tests`. Duplicate identities are refused
/// within the page; the commit door already refuses them across the whole
/// file before it appends.
#[must_use]
pub fn read_stored_selection_v6(
    root: &Path,
    from: u64,
    limit: u64,
) -> Vec<(&'static str, StoredSelectionV6Rung)> {
    crate::ledger_all::LEDGER_RUNGS
        .into_iter()
        .map(|rung| (rung, read_rung(root, rung, from, limit)))
        .collect()
}

fn read_rung(root: &Path, rung: &'static str, from: u64, limit: u64) -> StoredSelectionV6Rung {
    let directory = root.join("selection").join(rung);
    let file = directory.join(FILE_NAME);
    match std::fs::symlink_metadata(&file) {
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            return StoredSelectionV6Rung::Absent(file.display().to_string());
        }
        Err(why) => {
            return StoredSelectionV6Rung::Refused(format!("{}: {why}", file.display()));
        }
        Ok(_) => {}
    }
    match read_blocks(&directory, rung, from, limit) {
        Ok((total, records)) => StoredSelectionV6Rung::Records {
            total,
            from,
            records,
        },
        Err(why) => StoredSelectionV6Rung::Refused(format!("{}: {why}", file.display())),
    }
}

fn read_blocks(
    directory: &Path,
    rung: &str,
    from: u64,
    limit: u64,
) -> Result<(u64, Vec<StoredSelectionV6Record>), String> {
    let rung_seconds = u64::try_from(crate::stored::rung_length_micros(rung)? / 1_000_000)
        .map_err(|why| why.to_string())?;
    let (mut file, path) = super::open(directory, false)?;
    file.lock_shared().map_err(|why| why.to_string())?;
    let result = (|| {
        let before = crate::result_set::file_generation(&file, &path)?;
        if before.len % BLOCK_BYTES != 0 {
            return Err(
                "Selection V6 has an incomplete trailing block (a commit interrupted or in \
                 progress); no block is shown"
                    .to_owned(),
            );
        }
        let total = before.len / BLOCK_BYTES;
        let shown = total.saturating_sub(from).min(limit);
        if shown > 0 {
            let offset = from
                .checked_mul(BLOCK_BYTES)
                .ok_or("Selection V6 page offset overflow")?;
            file.seek(std::io::SeekFrom::Start(offset))
                .map_err(|why| why.to_string())?;
        }
        let mut records =
            Vec::with_capacity(usize::try_from(shown).map_err(|why| why.to_string())?);
        let mut identities = std::collections::HashSet::with_capacity(records.capacity());
        for _ in 0..shown {
            let mut block = [0; SELECTION_V6_BLOCK_BYTES];
            file.read_exact(&mut block).map_err(|why| why.to_string())?;
            let record = decode_block(&block)?;
            if record.rung_seconds != rung_seconds {
                return Err(format!(
                    "a block records rung {} s under the {rung} directory",
                    record.rung_seconds
                ));
            }
            if !identities.insert(record.identity) {
                return Err("Selection V6 contains duplicate committed identities".to_owned());
            }
            records.push(record);
        }
        crate::result_set::require_generation_unchanged(
            before,
            crate::result_set::file_generation(&file, &path)?,
            &path,
        )?;
        Ok((total, records))
    })();
    let released = file.unlock().map_err(|why| why.to_string());
    result.and_then(|records| released.map(|()| records))
}

/// A cursor over a block's payload, refusing a read past the seal.
struct Decoder<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Decoder<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self
            .at
            .checked_add(N)
            .ok_or("Selection V6 decode overflow")?;
        let slice = self
            .bytes
            .get(self.at..end)
            .ok_or("Selection V6 decode ran past the payload")?;
        self.at = end;
        slice
            .try_into()
            .map_err(|_| "Selection V6 decode width".to_owned())
    }
    fn digest(&mut self) -> Result<[u8; 32], String> {
        self.take::<32>()
    }
    fn number(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take::<8>()?))
    }
    fn ratio(&mut self) -> Result<Option<u64>, String> {
        let defined = self.number()?;
        let value = self.number()?;
        match (defined, value) {
            (1, value) => Ok(Some(value)),
            (0, 0) => Ok(None),
            _ => Err("Selection V6 ratio has an invalid definedness word".to_owned()),
        }
    }
}

fn family_name(code: u64) -> Result<&'static str, String> {
    for (stored, family) in FAMILIES {
        if stored == code {
            return Ok(family);
        }
    }
    Err(format!(
        "family code {code} is not NIFTY (1) or BANKNIFTY (2). {SELECTION_V6_EQUITY_REFUSAL}"
    ))
}

/// The two family envelopes, NIFTY's then BANKNIFTY's; any other code is
/// refused with the equity sentence.
fn decode_families(d: &mut Decoder<'_>) -> Result<[StoredSelectionV6Family; 2], String> {
    let mut families = Vec::with_capacity(2);
    for (expected, _) in FAMILIES {
        for _ in 0..4 {
            d.digest()?;
        }
        let code = d.number()?;
        if code != expected {
            return Err(format!(
                "family envelope {} holds code {code}. {SELECTION_V6_EQUITY_REFUSAL}",
                families.len()
            ));
        }
        let terminal = match d.number()? {
            1 => "evaluated",
            2 => "insufficient-for-cscv",
            3 => "naturally-extinct",
            other => {
                return Err(format!(
                    "family terminal code {other} is not one Selection V6 writes"
                ));
            }
        };
        families.push(StoredSelectionV6Family {
            family: family_name(code)?,
            terminal,
            candidate_count: d.number()?,
            evaluated_count: d.number()?,
            decision_count: d.number()?,
        });
    }
    families
        .try_into()
        .map_err(|_| "Selection V6 did not decode two family envelopes".to_owned())
}

/// One stored winner at `rank`.
fn decode_winner(d: &mut Decoder<'_>, rank: u32) -> Result<StoredSelectionV6Winner, String> {
    let strategy_digest = d.digest()?;
    let disposition_id = d.digest()?;
    let selected_exit_digest = d.digest()?;
    let family = family_name(d.number()?)?;
    let global_sequence = d.number()?;
    let family_sequence = d.number()?;
    let score = d.number()?;
    let direction = match d.number()? {
        1 => "long",
        2 => "short",
        other => {
            return Err(format!(
                "winner direction code {other} is not long (1) or short (2)"
            ));
        }
    };
    let mut mask_words = [0_u64; 6];
    for word in &mut mask_words {
        *word = d.number()?;
    }
    Ok(StoredSelectionV6Winner {
        rank,
        family,
        strategy_digest,
        disposition_id,
        selected_exit_digest,
        global_sequence,
        family_sequence,
        score,
        direction,
        mask_words,
        drawdown: d.number()?,
        worst_loss: d.number()?,
        losing_rate_ppm: d.number()?,
        losing_trades: d.number()?,
        winning_trades: d.number()?,
        win_rate_ppm: d.number()?,
        average_win: d.number()?,
        average_loss: d.number()?,
        assurance_ppm: d.number()?,
        pessimistic_profit: i64::from_le_bytes(d.take::<8>()?),
        loss_ratio_ppm: d.ratio()?,
        reward_to_risk_ppm: d.ratio()?,
    })
}

/// One sealed block, decoded field for field as `encode_block` wrote it.
///
/// # Errors
///
/// A block [`super::verify_block`] refuses, or one whose fields the encoder
/// could not have written.
pub(crate) fn decode_block(block: &super::Block) -> Result<StoredSelectionV6Record, String> {
    super::verify_block(block)?;
    let identity = block
        .get(24..56)
        .ok_or("Selection V6 identity slot")?
        .try_into()
        .map_err(|_| "Selection V6 identity width")?;
    let mut d = Decoder {
        bytes: block.get(..SEAL_AT).ok_or("Selection V6 payload bound")?,
        at: 56,
    };
    let population_id = d.digest()?;
    let _population_completion = d.digest()?;
    let _population_ordered = d.digest()?;
    let execution_completion_id = d.digest()?;
    let _ordered_dispositions = d.digest()?;
    let _nifty_authority = d.digest()?;
    let _banknifty_authority = d.digest()?;
    let policy_digest = d.digest()?;
    let rung_seconds = d.number()?;
    let horizon_bars = d.number()?;
    let _parameters = d.number()?;
    let _percentiles = d.number()?;
    let row_count = d.number()?;
    let _nifty_count = d.number()?;
    let _banknifty_count = d.number()?;
    let authorized_count = d.number()?;
    let policy_refused_count = d.number()?;
    for _ in 0..(4 + 16) {
        d.number()?;
    }
    let families = decode_families(&mut d)?;
    let considered = d.number()?;
    let admitted = d.number()?;
    let refused = d.number()?;
    let unmeasured = d.number()?;
    let winner_count = d.number()?;
    let top_ten = d.number()?;
    if winner_count > 25 || top_ten != winner_count.min(10) {
        return Err(format!(
            "Selection V6 records {winner_count} winner(s) and a Top-10 of {top_ten}; the \
             encoder writes at most 25 and a Top-10 that is their prefix"
        ));
    }
    let _ordered_digest = d.digest()?;
    let mut winners =
        Vec::with_capacity(usize::try_from(winner_count).map_err(|why| why.to_string())?);
    for rank in 0..u32::try_from(winner_count).map_err(|why| why.to_string())? {
        winners.push(decode_winner(&mut d, rank)?);
    }
    if d.bytes
        .get(d.at..)
        .is_none_or(|tail| tail.iter().any(|byte| *byte != 0))
    {
        return Err("Selection V6 unused slots are not zero".to_owned());
    }
    Ok(StoredSelectionV6Record {
        identity,
        rung_seconds,
        horizon_bars,
        population_id,
        execution_completion_id,
        policy_digest,
        row_count,
        authorized_count,
        policy_refused_count,
        families,
        considered,
        admitted,
        refused,
        unmeasured,
        winners,
    })
}
