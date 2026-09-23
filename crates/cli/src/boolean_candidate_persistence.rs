//! Separate immutable program catalog bodies with receipt-last completion.
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use super::{BooleanCoordinateV1, Expression, ResearchFamilyV1, Side, display};
use brutex_core::blake3::hash;

#[path = "boolean_observation_file.rs"]
mod observation;
pub(crate) use observation::Observation;

fn known_namespace(namespace: &str) -> Result<(), String> {
    if matches!(
        namespace,
        "boolean-candidates-v1"
            | "boolean-statistics-v1"
            | "boolean-admission-v1"
            | "boolean-oos-v1"
            | "boolean-qualification-v1"
            | "index-consistency-v1"
            | "index-stop-candidates-v1"
            | "index-stop-qualification-v1"
            | "index-stop-source-context-v1"
            | "index-stop-catalog-context-v1"
            | "index-stop-vix-reference-v1"
    ) {
        Ok(())
    } else {
        Err("unknown Boolean authority namespace".to_owned())
    }
}

pub(crate) struct Pending {
    directory: PathBuf,
    owner: File,
    generation: crate::result_set::FileGeneration,
}
impl Pending {
    pub(super) fn directory(&self) -> &Path {
        &self.directory
    }
    pub(crate) fn verify_body(&self, digest: [u8; 32], bytes: u64) -> Result<(), String> {
        self.require_owner()?;
        let actual = read_exact(&self.directory.join("body.bin"), bytes)?;
        if actual.len() as u64 != bytes || hash(&actual) != digest {
            return Err("Boolean candidate body changed before publication".to_owned());
        }
        Ok(())
    }
    fn require_owner(&self) -> Result<(), String> {
        let path = self.directory.join("owner.lock");
        crate::result_set::require_generation_unchanged(
            self.generation,
            crate::result_set::file_generation(&self.owner, &path)?,
            &path,
        )
    }
    pub(crate) fn finish(
        &self,
        identity: [u8; 32],
        payload: [u8; 32],
        bytes: u64,
    ) -> Result<[u8; 32], String> {
        self.require_owner()?;
        let encoded = receipt(identity, payload, bytes);
        write_or_equal(&self.directory.join("complete.bin"), &encoded)?;
        File::open(&self.directory)
            .map_err(display)?
            .sync_all()
            .map_err(display)?;
        Ok(hash(&encoded))
    }
}

pub(super) fn prepare(root: &Path, identity: [u8; 32], body: &[u8]) -> Result<Pending, String> {
    prepare_in_namespace(root, "boolean-candidates-v1", identity, body)
}

pub(crate) fn prepare_in_namespace(
    root: &Path,
    namespace: &str,
    identity: [u8; 32],
    body: &[u8],
) -> Result<Pending, String> {
    known_namespace(namespace)?;
    let base = root.join(namespace);
    directory(root, &base)?;
    let directory_path = base.join(crate::identity_hex(&identity));
    directory(&base, &directory_path)?;
    let owner_path = directory_path.join("owner.lock");
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&owner_path)
    {
        Ok(file) => {
            file.sync_all().map_err(display)?;
        }
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(why) => return Err(display(why)),
    }
    let owner = crate::readonly_file::open(&owner_path).map_err(display)?;
    owner.try_lock().map_err(|why| {
        format!("Boolean candidate namespace already owned or lock refused: {why}")
    })?;
    let generation = crate::result_set::file_generation(&owner, &owner_path)?;
    if owner.metadata().map_err(display)?.len() != 0 {
        return Err("Boolean candidate owner contains unexpected bytes".to_owned());
    }
    write_or_equal(&directory_path.join("body.bin"), body)?;
    File::open(&directory_path)
        .map_err(display)?
        .sync_all()
        .map_err(display)?;
    crate::result_set::require_generation_unchanged(
        generation,
        crate::result_set::file_generation(&owner, &owner_path)?,
        &owner_path,
    )?;
    Ok(Pending {
        directory: directory_path,
        owner,
        generation,
    })
}

pub(super) fn verify(
    directory: &Path,
    identity: [u8; 32],
    payload: [u8; 32],
    bytes: u64,
    completion: [u8; 32],
) -> Result<(), String> {
    let expected = receipt(identity, payload, bytes);
    let before = read_exact(&directory.join("complete.bin"), 112)?;
    if before != expected || hash(&before) != completion {
        return Err("Boolean completion receipt changed".to_owned());
    }
    let body = read_exact(&directory.join("body.bin"), bytes)?;
    if body.len() as u64 != bytes || hash(&body) != payload {
        return Err("Boolean candidate body no longer matches completion".to_owned());
    }
    let after = read_exact(&directory.join("complete.bin"), 112)?;
    if before != after {
        return Err("Boolean completion changed during body verification".to_owned());
    }
    Ok(())
}

fn receipt(identity: [u8; 32], payload: [u8; 32], bytes: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(112);
    out.extend_from_slice(b"BRBLCM01");
    out.extend_from_slice(&identity);
    out.extend_from_slice(&payload);
    out.extend_from_slice(&bytes.to_le_bytes());
    let digest = hash(&out);
    out.extend_from_slice(&digest);
    out
}

fn directory(parent: &Path, path: &Path) -> Result<(), String> {
    match std::fs::create_dir(path) {
        Ok(()) => {}
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(why) => return Err(display(why)),
    }
    if !std::fs::symlink_metadata(path)
        .map_err(display)?
        .file_type()
        .is_dir()
    {
        return Err("Boolean candidate namespace is not a real directory".to_owned());
    }
    File::open(parent)
        .map_err(display)?
        .sync_all()
        .map_err(display)
}

fn write_or_equal(path: &Path, body: &[u8]) -> Result<(), String> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(body).map_err(display)?;
            file.sync_all().map_err(display)?;
            let before = crate::result_set::file_generation(&file, path)?;
            if file.metadata().map_err(display)?.len() != body.len() as u64 {
                return Err("Boolean evidence write was not retained".to_owned());
            }
            crate::result_set::require_generation_unchanged(
                before,
                crate::result_set::file_generation(&file, path)?,
                path,
            )
        }
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_exact(path, body.len() as u64)? != body {
                return Err("Boolean evidence already exists with different or incomplete bytes; history preserved".to_owned());
            }
            Ok(())
        }
        Err(why) => Err(display(why)),
    }
}

pub(super) fn read_exact(path: &Path, max_bytes: u64) -> Result<Vec<u8>, String> {
    read_held(path, max_bytes).map(|(_, _, body)| body)
}
pub(super) fn read_held(
    path: &Path,
    max_bytes: u64,
) -> Result<(File, crate::result_set::FileGeneration, Vec<u8>), String> {
    let mut file = crate::readonly_file::open(path).map_err(display)?;
    file.try_lock_shared()
        .map_err(|why| format!("Boolean evidence is busy or cannot be locked: {why}"))?;
    let before = crate::result_set::file_generation(&file, path)?;
    let bytes = file.metadata().map_err(display)?.len();
    if bytes > max_bytes {
        return Err("Boolean candidate read exceeds explicit byte ceiling".to_owned());
    }
    let count = usize::try_from(bytes).map_err(display)?;
    let mut body = Vec::new();
    body.try_reserve_exact(count).map_err(display)?;
    body.resize(count, 0);
    file.read_exact(&mut body).map_err(display)?;
    crate::result_set::require_generation_unchanged(
        before,
        crate::result_set::file_generation(&file, path)?,
        path,
    )?;
    Ok((file, before, body))
}

pub(super) fn encode(
    identity: ([u8; 32], [u8; 32]),
    family: ResearchFamilyV1,
    programs: &[Expression],
    sessions: &[i64],
    rows: &[BooleanCoordinateV1],
    grids: (
        &[super::ResearchResolvedExitGridV1; 2],
        runner::outcome::Horizon,
    ),
    max_bytes: u64,
) -> Result<Vec<u8>, String> {
    let expected = encoded_size(programs.len(), sessions.len(), rows)?
        .checked_add(super::grid_context::size(grids.0)?)
        .ok_or("Boolean grid body size overflow")?;
    if expected > max_bytes {
        return Err("Boolean candidate body exceeds byte ceiling".to_owned());
    }
    let mut out = Vec::new();
    out.try_reserve_exact(usize::try_from(expected).map_err(display)?)
        .map_err(display)?;
    out.extend_from_slice(b"BRBOOL01");
    out.extend_from_slice(&identity.0);
    out.extend_from_slice(&identity.1);
    out.extend_from_slice(&family.encode());
    for count in [programs.len(), sessions.len(), rows.len()] {
        word(&mut out, count as u64);
    }
    for program in programs {
        out.extend_from_slice(&program.encode());
    }
    for day in sessions {
        signed(&mut out, *day);
    }
    out.extend_from_slice(&super::grid_context::encode(grids.0, grids.1)?);
    for row in rows {
        out.extend_from_slice(&row.identity);
        word(&mut out, row.program_index as u64);
        out.extend_from_slice(&row.run);
        word(
            &mut out,
            match row.side {
                Side::Long => 0,
                Side::Short => 1,
            },
        );
        word(&mut out, row.ordinal);
        cell(&mut out, &row.cell);
        word(&mut out, row.refusal.bits());
        word(&mut out, row.periods.len() as u64);
        word(&mut out, row.trades().len() as u64);
        word(&mut out, row.support_sessions());
        let summary = row.summary();
        for count in [
            summary.evaluated,
            summary.hits,
            summary.misses,
            summary.unknown,
        ] {
            word(&mut out, count);
        }
        for period in &row.periods {
            signed(&mut out, period.day);
            signed(&mut out, period.return_paisa);
            word(&mut out, period.trades);
            word(&mut out, period.wins);
        }
        for trade in &row.trades {
            for value in [trade.signal_bar, trade.entry_bar, trade.exit_bar] {
                word(&mut out, value as u64);
            }
            for value in [
                trade.best,
                trade.worst,
                trade.entry_micros,
                trade.exit_micros,
                trade.adverse,
                trade.adverse_paisa,
                trade.favourable,
                trade.favourable_paisa,
            ] {
                signed(&mut out, value);
            }
        }
    }
    if out.len() as u64 != expected {
        return Err("Boolean candidate codec cardinality drift".to_owned());
    }
    Ok(out)
}

pub(super) fn base_size(programs: usize, sessions: usize) -> Result<u64, String> {
    (programs as u64)
        .checked_mul(vocab::expression::ENCODED_LEN as u64)
        .and_then(|size| size.checked_add((sessions as u64).checked_mul(8)?))
        .and_then(|size| size.checked_add(224))
        .ok_or_else(|| "Boolean descriptor size overflow".to_owned())
}
pub(super) fn row_size(periods: usize, trades: u64) -> Result<u64, String> {
    (periods as u64)
        .checked_mul(32)
        .and_then(|size| size.checked_add(trades.checked_mul(88)?))
        .and_then(|size| size.checked_add(392))
        .ok_or_else(|| "Boolean coordinate size overflow".to_owned())
}
fn encoded_size(
    programs: usize,
    sessions: usize,
    rows: &[BooleanCoordinateV1],
) -> Result<u64, String> {
    rows.iter()
        .try_fold(base_size(programs, sessions)?, |sum, row| {
            sum.checked_add(row_size(row.periods.len(), row.trades.len() as u64)?)
                .ok_or_else(|| "Boolean body size overflow".to_owned())
        })
}
fn word(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn signed(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn optional(out: &mut Vec<u8>, value: Option<usize>) {
    word(out, value.map_or(u64::MAX, |v| v as u64));
}
fn cell(out: &mut Vec<u8>, value: &runner::grid::Cell) {
    for index in [
        value.stop,
        value.target,
        value.tsl,
        value.ttp.map(|v| v.arm),
        value.ttp.map(|v| v.trail),
    ] {
        optional(out, index);
    }
    word(out, value.trades);
    word(out, value.wins);
    for number in [value.pessimistic, value.optimistic, value.fill_cost] {
        signed(out, number);
    }
    for number in [
        value.stopped,
        value.trailed_stop,
        value.trailed_profit,
        value.targeted,
        value.timed_out,
        value.ambiguous_bars,
        value.gapped,
    ] {
        word(out, number);
    }
    for number in [
        value.winner_mae,
        value.winner_mfe,
        value.all_mae,
        value.worst_mae,
        value.gross_win,
        value.gross_loss,
        value.best_trade,
        value.min_win,
    ] {
        signed(out, number);
    }
    word(out, value.bars_held);
    word(out, u64::from(value.max_losing_streak));
    word(out, u64::from(value.max_winning_streak));
    signed(out, value.worst_trade);
    signed(out, value.max_drawdown);
}
