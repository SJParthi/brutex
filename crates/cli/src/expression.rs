//! Durable exact-expression signal research over stored historical evidence.
//!
//! Expression V1 has its own full identity and fixed-stride evidence. It is
//! never inserted into a ledger that would decode its bits as an AND strategy.

use runner::expression::{Expression, Summary, Truth};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"BTXEXPR1";
const HEADER: usize = 40 + runner::expression::ENCODED_LEN;
const DATA_BYTES: usize = 24;
const STRIDE: usize = 56;
const FOOTER: usize = 64;

/// Dispatch the explicit stored-expression command with shared refusal text.
pub(crate) fn stored(
    feed: &str,
    underlying: &str,
    rung: &str,
    year: &str,
    month: &str,
    source: &str,
    out: &mut String,
) -> u8 {
    let Ok(year) = year.parse::<u16>() else {
        return crate::refuse(out, "YEAR must be a number like 2026");
    };
    let Ok(month @ 1..=12) = month.parse::<u8>() else {
        return crate::refuse(out, "MONTH must be 1..=12");
    };
    match run(feed, underlying, rung, year, month, source) {
        Ok(report) => {
            out.push_str(&report);
            crate::OK
        }
        Err(why) => crate::refuse(out, &why),
    }
}

fn run(
    feed: &str,
    underlying: &str,
    rung: &str,
    year: u16,
    month: u8,
    source: &str,
) -> Result<String, String> {
    let expression = Expression::parse(source).map_err(|why| {
        format!(
            "expression refused: {why:?}; use live names or bit numbers, !, &, | and parentheses"
        )
    })?;
    crate::swept_rung(rung)?;
    let commit = crate::commit_stamp().ok_or_else(|| "this build has no verified clean commit stamp; the expression cannot run before its identity is recordable".to_owned())?;
    let vendor = crate::parse_vendor(feed)?;
    let root = crate::store_root()?;
    let loaded = crate::stored::load(&root, vendor, underlying, rung, year, month)?;
    let span = ((year, month), (year, month));
    let daily = crate::stored::load_daily_context(&root, vendor, underlying, span, &loaded.bars)?;
    let minute =
        crate::stored::load_exact_minute_context(&root, vendor, underlying, span, &loaded.bars)?;
    let digest = crate::stored_anchored_digest(&loaded.bars, &minute, &daily)?;
    let run = runner::identity::Run {
        mask: expression.referenced(),
        direction: runner::identity::Direction::Undirected,
        instrument: &loaded.key,
        timeframe: loaded.timeframe,
        params: runner::identity::Params::of(engine::Ladder::with_min_hits(1)),
        data_digest: digest,
        commit,
        feed: loaded.vendor.as_str(),
    };
    let identity = runner::expression::identity(&run, &expression);
    // Both lifecycle and descriptor sync before the indicator fold or expression
    // evaluation. A failure here prevents the computation, not just its report.
    let attempt = crate::sweep_evidence::begin(
        &root,
        identity,
        crate::sweep_evidence::Operation::Expression,
    )?;
    let directory =
        attempt_directory(&root, identity, attempt.token()).map_err(|e| e.to_string())?;
    let mut writer =
        EvidenceWriter::begin(&directory, identity, &expression).map_err(|e| e.to_string())?;
    let column = crate::stored_anchored_column(
        &loaded.bars,
        &daily,
        &minute,
        crate::stored::rung_length_micros(rung)?,
        crate::stored::vwap_availability(&loaded.key),
    )?;
    let summary = runner::expression::evaluate(&column, &expression, |index, verdict| {
        let bar = loaded.bars.get(index).ok_or_else(|| {
            std::io::Error::other("expression source is outside its historical slice")
        })?;
        writer.row(index, bar.ts_micros, verdict)
    })
    .map_err(|why| format!("expression evidence refused: {why:?}"))?;
    let path = writer.finish(summary).map_err(|e| e.to_string())?;
    if !summary.reconciles()
        || summary.evaluated == 0
        || !column.census().reconciles()
        || column.census().refused() != 0
    {
        attempt.finish(crate::sweep_evidence::Completion::Refused)?;
        return Err(format!(
            "expression has no complete valid sample: evaluated {}, refused bars {}; saved evidence {}",
            summary.evaluated,
            column.census().refused(),
            path.display()
        ));
    }
    attempt.finish(crate::sweep_evidence::Completion::Completed)?;
    let mut report = String::from(crate::STORED_PROVENANCE);
    let _ = writeln!(
        report,
        "EXPLICIT EXPRESSION V1\n  expression: {source}\n  feed: {} · {underlying} · {} · {year}-{month:02}\n  identity: {}\n  evaluated: {} · true: {} · false: {} · unknown: {}\n  warming bars: {}\n  evidence: {}\nSIGNAL RESEARCH ONLY: this evaluates the named expression; it does not enumerate all Boolean expressions, price trades, or grant institutional admission. Unknown is never admitted as a hit.",
        loaded.vendor.as_str(),
        loaded.timeframe,
        hex(&identity),
        summary.evaluated,
        summary.hits,
        summary.misses,
        summary.unknown,
        column.census().warming,
        path.display()
    );
    Ok(report)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

pub(crate) fn attempt_directory(
    root: &Path,
    identity: [u8; 32],
    token: u64,
) -> std::io::Result<PathBuf> {
    let mut directory = root.to_path_buf();
    for (position, part) in [
        "expression-v1".to_owned(),
        hex(&identity),
        token.to_string(),
    ]
    .into_iter()
    .enumerate()
    {
        let parent = directory.clone();
        directory.push(part);
        match fs::create_dir(&directory) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && position < 2 => {
                if !fs::symlink_metadata(&directory)?.file_type().is_dir() {
                    return Err(std::io::Error::other(
                        "expression ancestor is not a real directory",
                    ));
                }
            }
            Err(e) => return Err(e),
        }
        // Sync each parent entry, including the format and identity directories.
        // Syncing only the leaf would not persist newly created ancestors.
        File::open(&parent)?.sync_all()?;
    }
    Ok(directory)
}

fn header_bytes(identity: [u8; 32], expression: &Expression) -> [u8; HEADER] {
    let mut bytes = [0; HEADER];
    for (slot, byte) in bytes.iter_mut().zip(
        MAGIC
            .iter()
            .copied()
            .chain(identity)
            .chain(expression.encode()),
    ) {
        *slot = byte;
    }
    bytes
}

fn count_bytes(summary: Summary) -> [u8; 32] {
    let mut bytes = [0; 32];
    for (slot, count) in bytes.chunks_exact_mut(8).zip([
        summary.evaluated,
        summary.hits,
        summary.misses,
        summary.unknown,
    ]) {
        slot.copy_from_slice(&count.to_le_bytes());
    }
    bytes
}

pub(crate) struct EvidenceWriter {
    pending: PathBuf,
    published: PathBuf,
    file: BufWriter<File>,
    hash: brutex_core::blake3::Hasher,
    observed: Summary,
    previous: Option<u64>,
    header_seal: [u8; 32],
}

impl EvidenceWriter {
    pub(crate) fn begin(
        directory: &Path,
        identity: [u8; 32],
        expression: &Expression,
    ) -> std::io::Result<Self> {
        let pending = directory.join("expression-v1.pending");
        let mut file = BufWriter::new(
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&pending)?,
        );
        file.get_ref().lock()?;
        let mut hash = brutex_core::blake3::Hasher::new();
        let header = header_bytes(identity, expression);
        file.write_all(&header)?;
        hash.update(&header);
        file.flush()?;
        file.get_ref().sync_all()?;
        File::open(directory)?.sync_all()?;
        let header_seal = hash.finalize();
        Ok(Self {
            pending,
            published: directory.join("expression-v1.rows"),
            file,
            hash,
            observed: Summary::default(),
            previous: None,
            header_seal,
        })
    }

    pub(crate) fn row(
        &mut self,
        source: usize,
        timestamp: i64,
        truth: Truth,
    ) -> std::io::Result<()> {
        let source = u64::try_from(source).map_err(std::io::Error::other)?;
        if self.previous.is_some_and(|previous| previous >= source) {
            return Err(std::io::Error::other("expression source order is invalid"));
        }
        let mut observed = self.observed;
        observed.evaluated = observed
            .evaluated
            .checked_add(1)
            .ok_or_else(|| std::io::Error::other("expression row count overflow"))?;
        let count = match truth {
            Truth::True => &mut observed.hits,
            Truth::False => &mut observed.misses,
            Truth::Unknown => &mut observed.unknown,
        };
        *count = count
            .checked_add(1)
            .ok_or_else(|| std::io::Error::other("expression verdict count overflow"))?;
        let mut bytes = [0_u8; STRIDE];
        for (slot, byte) in bytes.iter_mut().zip(
            source
                .to_le_bytes()
                .into_iter()
                .chain(timestamp.to_le_bytes())
                .chain([match truth {
                    Truth::False => 0,
                    Truth::True => 1,
                    Truth::Unknown => 2,
                }]),
        ) {
            *slot = byte;
        }
        let data = bytes
            .get(..DATA_BYTES)
            .ok_or_else(|| std::io::Error::other("expression row width"))?;
        let seal = row_seal(self.header_seal, self.observed.evaluated, data);
        for (slot, byte) in bytes.iter_mut().skip(DATA_BYTES).zip(seal) {
            *slot = byte;
        }
        self.file.write_all(&bytes)?;
        self.hash.update(&bytes);
        self.observed = observed;
        self.previous = Some(source);
        Ok(())
    }

    pub(crate) fn finish(mut self, summary: Summary) -> std::io::Result<PathBuf> {
        if !summary.reconciles() || summary != self.observed {
            return Err(std::io::Error::other(
                "expression summary does not reconcile",
            ));
        }
        let body = summary
            .evaluated
            .checked_mul(56)
            .and_then(|n| n.checked_add(u64::try_from(HEADER).ok()?))
            .ok_or_else(|| std::io::Error::other("expression evidence size overflow"))?;
        let bytes = body
            .checked_add(64)
            .ok_or_else(|| std::io::Error::other("expression evidence size overflow"))?;
        self.file.flush()?;
        if self.file.get_ref().metadata()?.len() != body {
            return Err(std::io::Error::other("expression pending length changed"));
        }
        let counts = count_bytes(summary);
        self.file.write_all(&counts)?;
        self.hash.update(&counts);
        self.file.write_all(&self.hash.finalize())?;
        self.file.flush()?;
        self.file.get_ref().sync_all()?;
        let expected = self.hash.finalize();
        verify_written(self.file.get_mut(), &self.pending, bytes, expected)?;
        // Link publishes without ever replacing an existing immutable result.
        fs::hard_link(&self.pending, &self.published)?;
        // Recheck the published name too: a pathname can be exchanged between
        // its preflight and hard_link even while the original inode is locked.
        verify_written(self.file.get_mut(), &self.published, bytes, expected)?;
        crate::result_set::file_generation(self.file.get_ref(), &self.pending)
            .map_err(std::io::Error::other)?;
        fs::remove_file(&self.pending)?;
        if let Some(directory) = self.published.parent() {
            File::open(directory)?.sync_all()?;
        }
        Ok(self.published)
    }
}

// Verify the exact acknowledged bytes through the locked writer handle. This
// final scan uses a fixed buffer and runs outside expression evaluation.
fn verify_written(
    file: &mut File,
    path: &Path,
    bytes: u64,
    expected: [u8; 32],
) -> std::io::Result<()> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(std::io::Error::other(
            "expression publication is not a regular file",
        ));
    }
    let before = crate::result_set::file_generation(file, path).map_err(std::io::Error::other)?;
    if file.metadata()?.len() != bytes {
        return Err(std::io::Error::other(
            "expression acknowledged length changed",
        ));
    }
    file.seek(SeekFrom::Start(0))?;
    let mut left = bytes
        .checked_sub(32)
        .ok_or_else(|| std::io::Error::other("expression seal width"))?;
    let mut buffer = [0; 8192];
    let mut hash = brutex_core::blake3::Hasher::new();
    while left != 0 {
        let count = usize::try_from(left.min(8192)).map_err(std::io::Error::other)?;
        let chunk = buffer
            .get_mut(..count)
            .ok_or_else(|| std::io::Error::other("expression verification buffer width"))?;
        file.read_exact(chunk)?;
        hash.update(chunk);
        left -= u64::try_from(count).map_err(std::io::Error::other)?;
    }
    let mut seal = [0; 32];
    file.read_exact(&mut seal)?;
    if hash.finalize() != expected || seal != expected {
        return Err(std::io::Error::other(
            "expression acknowledged bytes changed before publication",
        ));
    }
    let after = crate::result_set::file_generation(file, path).map_err(std::io::Error::other)?;
    crate::result_set::require_generation_unchanged(before, after, path)
        .map_err(std::io::Error::other)
}

/// A saved signal row; source indices name the exact stored signal slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// Position in the source history before warming/refusal exclusions.
    pub source: u64,
    /// Exact timestamp in microseconds.
    pub timestamp: i64,
    /// True, false or unavailable.
    pub truth: Truth,
}

/// Recover the saved identity and program, verify the entire evidence file,
/// then visit its rows. The operator need not retain the original source text.
///
/// # Errors
/// The same type, size, format, integrity and visitor refusals as [`read`].
pub fn read_saved(
    path: &Path,
    max_bytes: u64,
    visit: impl FnMut(Row) -> std::io::Result<()>,
) -> std::io::Result<([u8; 32], Expression, Summary)> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > max_bytes {
        return Err(std::io::Error::other(
            "expression evidence type/byte ceiling refused",
        ));
    }
    let mut header = [0_u8; HEADER];
    crate::readonly_file::open(path)?.read_exact(&mut header)?;
    let mut identity = [0_u8; 32];
    for (slot, byte) in identity.iter_mut().zip(header.iter().skip(8)) {
        *slot = *byte;
    }
    let descriptor = header
        .get(40..)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| std::io::Error::other("expression descriptor width refused"))?;
    let expression = Expression::decode(descriptor)
        .map_err(|why| std::io::Error::other(format!("expression descriptor refused: {why:?}")))?;
    let summary = read(path, identity, &expression, max_bytes, visit)?;
    Ok((identity, expression, summary))
}

/// UNVERIFIED performance: no named cost test or measured latency bound is established here.
/// Strictly validate and visit a saved expression signal file with bounded RAM.
/// The file is scanned before any row is returned. This integrity scan is
/// O(rows); it is not advertised as constant-time historical verification.
///
/// # Errors
/// Missing, oversized, unsealed, corrupt, misidentified or malformed evidence,
/// or a visitor failure. `max_bytes` is a caller resource ceiling.
pub fn read(
    path: &Path,
    identity: [u8; 32],
    expression: &Expression,
    max_bytes: u64,
    mut visit: impl FnMut(Row) -> std::io::Result<()>,
) -> std::io::Result<Summary> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > max_bytes {
        return Err(std::io::Error::other(
            "expression evidence type/byte ceiling refused",
        ));
    }
    let minimum = u64::try_from(HEADER + FOOTER).map_err(std::io::Error::other)?;
    let body = metadata
        .len()
        .checked_sub(minimum)
        .filter(|n| n.is_multiple_of(56))
        .ok_or_else(|| std::io::Error::other("expression evidence is torn"))?;
    let mut file = crate::readonly_file::open(path)?;
    file.try_lock_shared().map_err(|why| {
        std::io::Error::other(format!(
            "expression evidence is busy or cannot be read: {why}"
        ))
    })?;
    let opened = file.metadata()?;
    if !opened.is_file() || opened.len() != metadata.len() {
        return Err(std::io::Error::other(
            "expression evidence changed during open",
        ));
    }
    let mut header = [0_u8; HEADER];
    file.read_exact(&mut header)?;
    if header != header_bytes(identity, expression) {
        return Err(std::io::Error::other(
            "expression evidence identity/descriptor mismatch",
        ));
    }
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(&header);
    let header_seal = hash.finalize();
    let mut observed = Summary::default();
    let mut previous = None;
    for ordinal in 0..body / 56 {
        let mut bytes = [0; STRIDE];
        file.read_exact(&mut bytes)?;
        hash.update(&bytes);
        let row = checked_row(&bytes, header_seal, ordinal)?;
        if previous.is_some_and(|p| p >= row.source) {
            return Err(std::io::Error::other("expression source order is invalid"));
        }
        previous = Some(row.source);
        observed.evaluated += 1;
        match row.truth {
            Truth::True => observed.hits += 1,
            Truth::False => observed.misses += 1,
            Truth::Unknown => observed.unknown += 1,
        }
    }
    let mut footer = [0; FOOTER];
    file.read_exact(&mut footer)?;
    let expected_counts = count_bytes(observed);
    hash.update(&expected_counts);
    if footer
        .iter()
        .copied()
        .ne(expected_counts.into_iter().chain(hash.finalize()))
    {
        return Err(std::io::Error::other(
            "expression evidence seal/count mismatch",
        ));
    }
    if file.metadata()?.len() != opened.len()
        || file.metadata()?.modified()? != opened.modified()?
    {
        return Err(std::io::Error::other(
            "expression evidence changed during verification",
        ));
    }
    // Reuse the verified open handle; a replacement pathname cannot swap it.
    file.seek(SeekFrom::Start(
        u64::try_from(HEADER).map_err(std::io::Error::other)?,
    ))?;
    for ordinal in 0..body / 56 {
        let mut bytes = [0; STRIDE];
        file.read_exact(&mut bytes)?;
        // Verify this exact row again before publishing it. An accidental
        // in-place change after the first scan cannot escape as a valid row.
        visit(checked_row(&bytes, header_seal, ordinal)?)?;
    }
    Ok(observed)
}

fn row_seal(header: [u8; 32], ordinal: u64, data: &[u8]) -> [u8; 32] {
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(b"BTXEXR01");
    hash.update(&header);
    hash.update(&ordinal.to_le_bytes());
    hash.update(data);
    hash.finalize()
}

fn checked_row(bytes: &[u8; STRIDE], header: [u8; 32], ordinal: u64) -> std::io::Result<Row> {
    let data = bytes
        .get(..DATA_BYTES)
        .ok_or_else(|| std::io::Error::other("expression row width"))?;
    let expected = row_seal(header, ordinal, data);
    if bytes.iter().skip(DATA_BYTES).copied().ne(expected) {
        return Err(std::io::Error::other("expression row seal mismatch"));
    }
    decode_row(bytes)
}

fn decode_row(bytes: &[u8; STRIDE]) -> std::io::Result<Row> {
    let mut source = [0; 8];
    let mut timestamp = [0; 8];
    for (slot, byte) in source.iter_mut().zip(bytes) {
        *slot = *byte;
    }
    for (slot, byte) in timestamp.iter_mut().zip(bytes.iter().skip(8)) {
        *slot = *byte;
    }
    let truth = match bytes.get(16) {
        Some(0) => Truth::False,
        Some(1) => Truth::True,
        Some(2) => Truth::Unknown,
        _ => return Err(std::io::Error::other("invalid expression truth")),
    };
    if bytes.iter().skip(17).take(7).any(|b| *b != 0) {
        return Err(std::io::Error::other("invalid expression row padding"));
    }
    Ok(Row {
        source: u64::from_le_bytes(source),
        timestamp: i64::from_le_bytes(timestamp),
        truth,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> std::io::Result<Self> {
            let path = std::env::temp_dir().join(format!(
                "brutex-expression-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(std::io::Error::other)?
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path)?;
            Ok(Self(path))
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn expression() -> std::io::Result<Expression> {
        Expression::parse("0 | !369")
            .map_err(|why| std::io::Error::other(format!("fixture: {why:?}")))
    }

    fn fixture(directory: &Path) -> std::io::Result<(PathBuf, Summary)> {
        let mut writer = EvidenceWriter::begin(directory, [7; 32], &expression()?)?;
        writer.row(2, 1_000, Truth::True)?;
        writer.row(4, 2_000, Truth::False)?;
        writer.row(9, 3_000, Truth::Unknown)?;
        let summary = Summary {
            evaluated: 3,
            hits: 1,
            misses: 1,
            unknown: 1,
        };
        Ok((writer.finish(summary)?, summary))
    }

    fn refused_without_rows(path: &Path, identity: [u8; 32], expr: &Expression, ceiling: u64) {
        let mut rows = 0;
        let result = read(path, identity, expr, ceiling, |_| {
            rows += 1;
            Ok(())
        });
        assert!(result.is_err(), "invalid evidence must refuse");
        assert_eq!(rows, 0, "no prefix may escape a failed integrity scan");
    }

    #[test]
    fn durable_rows_round_trip_exact_sources_unknowns_and_empty_history() -> std::io::Result<()> {
        let scratch = Scratch::new()?;
        let (path, summary) = fixture(&scratch.0)?;
        let mut rows = Vec::new();
        assert_eq!(
            read(&path, [7; 32], &expression()?, u64::MAX, |row| {
                rows.push(row);
                Ok(())
            })?,
            summary
        );
        assert_eq!(
            rows,
            vec![
                Row {
                    source: 2,
                    timestamp: 1_000,
                    truth: Truth::True
                },
                Row {
                    source: 4,
                    timestamp: 2_000,
                    truth: Truth::False
                },
                Row {
                    source: 9,
                    timestamp: 3_000,
                    truth: Truth::Unknown
                },
            ]
        );
        let (saved_identity, saved_expression, saved_summary) =
            read_saved(&path, u64::MAX, |_| Ok(()))?;
        assert_eq!(saved_identity, [7; 32]);
        assert_eq!(saved_expression, expression()?);
        assert_eq!(saved_summary, summary);
        assert!(!scratch.0.join("expression-v1.pending").exists());
        assert_eq!(
            fs::metadata(&path)?.len(),
            u64::try_from(HEADER + 3 * STRIDE + FOOTER).map_err(std::io::Error::other)?
        );
        let empty = Scratch::new()?;
        let path =
            EvidenceWriter::begin(&empty.0, [7; 32], &expression()?)?.finish(Summary::default())?;
        assert_eq!(
            read(&path, [7; 32], &expression()?, u64::MAX, |_| Err(
                std::io::Error::other("no rows expected")
            ))?,
            Summary::default()
        );
        Ok(())
    }

    #[test]
    fn every_single_byte_mutation_and_every_truncation_refuse_before_publication()
    -> std::io::Result<()> {
        let scratch = Scratch::new()?;
        let (path, _) = fixture(&scratch.0)?;
        let original = fs::read(&path)?;
        let attacked = scratch.0.join("attacked.rows");
        let expr = expression()?;
        for index in 0..original.len() {
            let mut bytes = original.clone();
            if let Some(byte) = bytes.get_mut(index) {
                *byte ^= 1;
            }
            fs::write(&attacked, &bytes)?;
            refused_without_rows(&attacked, [7; 32], &expr, u64::MAX);
        }
        for length in 0..original.len() {
            fs::write(
                &attacked,
                original
                    .get(..length)
                    .ok_or_else(|| std::io::Error::other("fixture slice"))?,
            )?;
            refused_without_rows(&attacked, [7; 32], &expr, u64::MAX);
        }
        let mut trailing = original.clone();
        trailing.extend_from_slice(&[0; STRIDE]);
        fs::write(&attacked, &trailing)?;
        refused_without_rows(&attacked, [7; 32], &expr, u64::MAX);
        Ok(())
    }

    #[test]
    fn wrong_identity_program_budget_type_and_visitor_failure_stay_explicit() -> std::io::Result<()>
    {
        let scratch = Scratch::new()?;
        let (path, _) = fixture(&scratch.0)?;
        let expr = expression()?;
        refused_without_rows(&path, [8; 32], &expr, u64::MAX);
        let other = Expression::parse("0 & !369")
            .map_err(|why| std::io::Error::other(format!("{why:?}")))?;
        refused_without_rows(&path, [7; 32], &other, u64::MAX);
        refused_without_rows(&path, [7; 32], &expr, fs::metadata(&path)?.len() - 1);
        refused_without_rows(&scratch.0, [7; 32], &expr, u64::MAX);
        refused_without_rows(&scratch.0.join("missing"), [7; 32], &expr, u64::MAX);
        let mut visits = 0;
        let failure = read(&path, [7; 32], &expr, u64::MAX, |_| {
            visits += 1;
            Err(std::io::Error::other("consumer failed"))
        });
        assert_eq!(visits, 1);
        assert!(failure.is_err_and(|e| e.to_string() == "consumer failed"));
        #[cfg(unix)]
        {
            let link = scratch.0.join("link");
            std::os::unix::fs::symlink(&path, &link)?;
            refused_without_rows(&link, [7; 32], &expr, u64::MAX);
        }
        Ok(())
    }

    #[test]
    fn completed_evidence_is_never_overwritten_and_bad_summaries_never_publish()
    -> std::io::Result<()> {
        let scratch = Scratch::new()?;
        let (path, _) = fixture(&scratch.0)?;
        let original = fs::read(&path)?;
        let writer = EvidenceWriter::begin(&scratch.0, [7; 32], &expression()?)?;
        assert!(writer.finish(Summary::default()).is_err());
        assert_eq!(fs::read(&path)?, original);
        assert!(EvidenceWriter::begin(&scratch.0, [7; 32], &expression()?).is_err());
        let other = Scratch::new()?;
        let writer = EvidenceWriter::begin(&other.0, [7; 32], &expression()?)?;
        assert!(
            writer
                .finish(Summary {
                    evaluated: 0,
                    hits: 1,
                    misses: 0,
                    unknown: 0
                })
                .is_err()
        );
        assert!(!other.0.join("expression-v1.rows").exists());
        Ok(())
    }

    #[test]
    fn replaced_or_mutated_pending_evidence_cannot_be_acknowledged_as_published()
    -> std::io::Result<()> {
        let summary = Summary {
            evaluated: 1,
            hits: 1,
            misses: 0,
            unknown: 0,
        };
        for attack in 0..6 {
            let scratch = Scratch::new()?;
            let foreign = Scratch::new()?;
            let mut other = EvidenceWriter::begin(&foreign.0, [8; 32], &expression()?)?;
            other.row(2, 1_000, Truth::True)?;
            let foreign_path = other.finish(summary)?;
            let foreign_bytes = fs::read(&foreign_path)?;
            let mut writer = EvidenceWriter::begin(&scratch.0, [7; 32], &expression()?)?;
            writer.row(2, 1_000, Truth::True)?;
            writer.file.flush()?;
            let pending = writer.pending.clone();
            match attack {
                0 => fs::remove_file(&pending)?,
                1 => {
                    fs::remove_file(&pending)?;
                    fs::hard_link(&foreign_path, &pending)?;
                }
                2 | 3 => {
                    let mut file = fs::OpenOptions::new().write(true).open(&pending)?;
                    let offset = if attack == 2 { 8 } else { HEADER + 8 };
                    file.seek(SeekFrom::Start(
                        u64::try_from(offset).map_err(std::io::Error::other)?,
                    ))?;
                    file.write_all(&[99])?;
                }
                4 => fs::OpenOptions::new()
                    .write(true)
                    .open(&pending)?
                    .set_len(0)?,
                _ => fs::OpenOptions::new()
                    .append(true)
                    .open(&pending)?
                    .write_all(&[99])?,
            }
            assert!(
                writer.finish(summary).is_err(),
                "attack {attack} must refuse"
            );
            assert!(!scratch.0.join("expression-v1.rows").exists());
            assert_eq!(
                fs::read(&foreign_path)?,
                foreign_bytes,
                "foreign evidence must remain unchanged"
            );
        }
        Ok(())
    }

    #[test]
    fn writer_refuses_wrong_counts_repeated_sources_and_reused_attempt_paths() -> std::io::Result<()>
    {
        let scratch = Scratch::new()?;
        let path = attempt_directory(&scratch.0, [7; 32], 1)?;
        assert!(attempt_directory(&scratch.0, [7; 32], 1).is_err());
        let next = attempt_directory(&scratch.0, [7; 32], 2)?;
        assert_ne!(path, next);
        let mut writer = EvidenceWriter::begin(&path, [7; 32], &expression()?)?;
        writer.row(4, 2_000, Truth::True)?;
        assert!(writer.row(4, 3_000, Truth::False).is_err());
        assert!(writer.row(2, 1_000, Truth::False).is_err());
        assert!(
            writer
                .finish(Summary {
                    evaluated: 1,
                    hits: 0,
                    misses: 1,
                    unknown: 0
                })
                .is_err()
        );
        assert!(!path.join("expression-v1.rows").exists());
        let other = Scratch::new()?;
        fs::write(other.0.join("expression-v1"), b"not a directory")?;
        assert!(attempt_directory(&other.0, [7; 32], 1).is_err());
        Ok(())
    }

    #[test]
    fn malformed_expression_command_refuses_before_touching_a_market_store() {
        for (year, month, expected) in [
            ("year", "1", "YEAR must"),
            ("2026", "0", "MONTH must"),
            ("2026", "13", "MONTH must"),
        ] {
            let mut out = String::new();
            assert_eq!(
                stored("invalid", "invalid", "invalid", year, month, "0", &mut out),
                crate::MISUSED
            );
            assert!(out.contains(expected), "{out}");
        }
        let mut out = String::new();
        assert_eq!(
            stored(
                "invalid", "invalid", "invalid", "2026", "1", "0 || 369", &mut out
            ),
            crate::MISUSED
        );
        assert!(out.contains("expression refused"), "{out}");
    }

    #[test]
    fn a_later_row_changed_during_visitation_never_escapes_its_own_seal() -> std::io::Result<()> {
        let scratch = Scratch::new()?;
        let (path, _) = fixture(&scratch.0)?;
        let mut visited = Vec::new();
        let result = read(&path, [7; 32], &expression()?, u64::MAX, |row| {
            visited.push(row);
            if visited.len() == 1 {
                // Deliberately ignore the advisory lock to model an accidental
                // noncooperating writer between the two read passes.
                let mut file = fs::OpenOptions::new().write(true).open(&path)?;
                file.seek(SeekFrom::Start(
                    u64::try_from(HEADER + STRIDE + 8).map_err(std::io::Error::other)?,
                ))?;
                file.write_all(&9_999_i64.to_le_bytes())?;
                file.sync_all()?;
            }
            Ok(())
        });
        assert!(result.is_err_and(|e| e.to_string() == "expression row seal mismatch"));
        assert_eq!(
            visited,
            vec![Row {
                source: 2,
                timestamp: 1_000,
                truth: Truth::True
            }]
        );
        Ok(())
    }

    #[test]
    fn generated_column_expression_lifecycle_and_reopened_rows_agree_end_to_end()
    -> std::io::Result<()> {
        use indicators::evaluator::{Calendar, Evaluator, Widths};
        let scratch = Scratch::new()?;
        let bars = runner::synthetic::sessions(8);
        let expression =
            Expression::parse("!30").map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        let identity = [19; 32]; // Explicit generated fixture identity, not market provenance.
        let attempt = crate::sweep_evidence::begin(
            &scratch.0,
            identity,
            crate::sweep_evidence::Operation::Expression,
        )
        .map_err(std::io::Error::other)?;
        let directory = attempt_directory(&scratch.0, identity, attempt.token())?;
        let mut writer = EvidenceWriter::begin(&directory, identity, &expression)?;
        assert!(
            crate::sweep_evidence::read(&scratch.0, identity, u64::MAX)
                .map_err(std::io::Error::other)?
                .is_some_and(|e| e.completion == crate::sweep_evidence::Completion::Running)
        );
        let mut evaluator = Evaluator::with_calendar(
            Widths::pinned().map_err(|e| std::io::Error::other(format!("{e:?}")))?,
            indicators::vwap::Availability::Absent,
            indicators::pattern::Thresholds::CLASSICAL,
            Calendar::all_regular(),
        );
        let column = indicators::column::Column::build(&bars, &mut evaluator);
        assert!(column.census().reconciles());
        assert_eq!(column.census().refused(), 0);
        assert!(!column.bits().is_empty());
        let summary = runner::expression::evaluate(&column, &expression, |source, truth| {
            let bar = bars
                .get(source)
                .ok_or_else(|| std::io::Error::other("fixture source"))?;
            writer.row(source, bar.ts_micros, truth)
        })
        .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        assert_eq!(
            summary.unknown, 0,
            "bar direction has definite availability"
        );
        let path = writer.finish(summary)?;
        attempt
            .finish(crate::sweep_evidence::Completion::Completed)
            .map_err(std::io::Error::other)?;
        let mut rows = 0;
        let (saved_identity, saved_expression, saved_summary) =
            read_saved(&path, u64::MAX, |row| {
                let source = usize::try_from(row.source).map_err(std::io::Error::other)?;
                let bar = bars
                    .get(source)
                    .ok_or_else(|| std::io::Error::other("reopened source"))?;
                assert_eq!(row.timestamp, bar.ts_micros);
                assert_eq!(
                    row.truth,
                    if bar.close <= bar.open {
                        Truth::True
                    } else {
                        Truth::False
                    }
                );
                rows += 1;
                Ok(())
            })?;
        assert_eq!(rows, column.len());
        assert_eq!(saved_identity, identity);
        assert_eq!(saved_expression, expression);
        assert_eq!(saved_summary, summary);
        assert!(
            crate::sweep_evidence::read(&scratch.0, identity, u64::MAX)
                .map_err(std::io::Error::other)?
                .is_some_and(|e| e.completion == crate::sweep_evidence::Completion::Completed)
        );
        Ok(())
    }
}
