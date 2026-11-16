//! DB rows with an audit trail (plan 13; REQ72-PLAN A.8's writer rule): TSV with a header, written ONCE (a temp file, fsync,
//! rename; refused if the target exists), in partitions keyed (run, timeframe, side, exit set, level); a correction is a new row
//! naming the row it supersedes. Every written file is a line of `AUDIT.tsv` (file, sha256, writer, IST from `date`), each line
//! carrying the sha of the line before (a hash chain). Every table carries a `labels` column and a row without its standing
//! labels is refused (mutant m26 of the plan's writer tests).

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::sha::{hex, sha256};

/// One run's DB folder.
#[derive(Clone, Debug)]
pub struct Db {
    dir: PathBuf,
    writer: String,
}

/// The genesis link of the chain.
pub const GENESIS: &str = "GENESIS";

impl Db {
    /// Opens (creates) the folder.
    ///
    /// # Errors
    /// The folder cannot be made.
    pub fn open(dir: &Path, writer: &str) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(Self { dir: dir.to_path_buf(), writer: writer.to_string() })
    }

    /// The folder.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn audit_path(&self) -> PathBuf {
        self.dir.join("AUDIT.tsv")
    }

    fn last_link(&self) -> Result<(u64, String), String> {
        let p = self.audit_path();
        if !p.exists() {
            return Ok((0, GENESIS.to_string()));
        }
        let t = fs::read_to_string(&p).map_err(|e| e.to_string())?;
        let lines: Vec<&str> = t.lines().filter(|l| !l.starts_with("seq\t")).collect();
        let n = lines.len() as u64;
        Ok((n, lines.last().map_or_else(|| GENESIS.to_string(), |l| hex(&sha256(l.as_bytes())))))
    }

    /// Writes table `name` once. Returns its sha256.
    ///
    /// # Errors
    /// A header without `labels`, a row whose `labels` cell is empty or whose width differs, a target that exists, or an I/O error.
    pub fn write(&self, name: &str, header: &[&str], rows: &[Vec<String>]) -> Result<String, String> {
        if name.contains('/') || name.starts_with('.') {
            return Err(format!("table name {name} refused"));
        }
        let li = header.iter().position(|h| *h == "labels").ok_or_else(|| format!("{name}: no labels column"))?;
        let mut text = header.join("\t");
        text.push('\n');
        for (i, r) in rows.iter().enumerate() {
            if r.len() != header.len() {
                return Err(format!("{name}: row {} has {} cells, header {}", i + 1, r.len(), header.len()));
            }
            if r.get(li).is_none_or(|x| x.trim().is_empty()) {
                return Err(format!("{name}: row {} has no standing labels (refused)", i + 1));
            }
            if r.iter().any(|c| c.contains('\t') || c.contains('\n')) {
                return Err(format!("{name}: row {} holds a tab or newline", i + 1));
            }
            text.push_str(&r.join("\t"));
            text.push('\n');
        }
        let target = self.dir.join(name);
        if target.exists() {
            return Err(format!("{name}: exists; a table is written once (a correction is a new table naming the one it supersedes)"));
        }
        let tmp = self.dir.join(format!(".{name}.tmp.{}", std::process::id()));
        {
            let mut f = File::create(&tmp).map_err(|e| e.to_string())?;
            f.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        if target.exists() {
            return Err(format!("{name}: appeared while writing (refused)"));
        }
        fs::rename(&tmp, &target).map_err(|e| e.to_string())?;
        let sha = hex(&sha256(text.as_bytes()));
        self.audit(name, &sha)?;
        Ok(sha)
    }

    /// Writes a binary partition once (fixed-width outcome records, an offset index). Returns its sha256.
    ///
    /// # Errors
    /// A target that exists, or an I/O error.
    pub fn write_bin(&self, name: &str, bytes: &[u8]) -> Result<String, String> {
        if name.contains('/') || name.starts_with('.') {
            return Err(format!("partition name {name} refused"));
        }
        let target = self.dir.join(name);
        if target.exists() {
            return Err(format!("{name}: exists; a partition is written once"));
        }
        let tmp = self.dir.join(format!(".{name}.tmp.{}", std::process::id()));
        {
            let mut f = File::create(&tmp).map_err(|e| e.to_string())?;
            f.write_all(bytes).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        fs::rename(&tmp, &target).map_err(|e| e.to_string())?;
        let sha = hex(&sha256(bytes));
        self.audit(name, &sha)?;
        Ok(sha)
    }

    /// SECONDS BUILDER (2026-11-14): the temporary path a STREAMED binary partition is written to (inside the DB folder; renamed by
    /// [`Db::adopt_bin`]).
    #[must_use]
    pub fn tmp_path(&self, name: &str) -> PathBuf {
        self.dir.join(format!(".{name}.tmp.{}", std::process::id()))
    }

    /// SECONDS BUILDER (2026-11-14): adopts a binary partition the caller STREAMED to [`Db::tmp_path`] (synced) with its sha256
    /// computed while writing: written once (refused if the target exists), renamed, audited like [`Db::write_bin`]. A gigabyte
    /// outcome store is never held whole.
    ///
    /// # Errors
    /// A target that exists, a missing temporary file, or an I/O error.
    pub fn adopt_bin(&self, name: &str, sha: &str) -> Result<String, String> {
        if name.contains('/') || name.starts_with('.') {
            return Err(format!("partition name {name} refused"));
        }
        let target = self.dir.join(name);
        if target.exists() {
            return Err(format!("{name}: exists; a partition is written once"));
        }
        let tmp = self.tmp_path(name);
        File::open(&tmp).and_then(|f| f.sync_all()).map_err(|e| format!("{}: {e}", tmp.display()))?;
        fs::rename(&tmp, &target).map_err(|e| e.to_string())?;
        self.audit(name, sha)?;
        Ok(sha.to_string())
    }

    /// SECONDS BUILDER (2026-11-14): [`Db::verify`] reading every file in pieces (a streamed partition of gigabytes is never read
    /// whole). The same checks, the same count.
    ///
    /// # Errors
    /// The first broken link or changed file.
    pub fn verify_stream(&self) -> Result<u64, String> {
        let t = fs::read_to_string(self.audit_path()).map_err(|e| e.to_string())?;
        let mut prev = GENESIS.to_string();
        let mut n = 0_u64;
        for l in t.lines().filter(|l| !l.starts_with("seq\t")) {
            let f: Vec<&str> = l.split('\t').collect();
            let (Some(file), Some(sha), Some(link)) = (f.get(1), f.get(2), f.get(5)) else { return Err(format!("audit line short: {l}")) };
            if *link != prev {
                return Err(format!("audit chain broken at {file}"));
            }
            if hex(&crate::sha::sha256_file(&self.dir.join(file))?) != *sha {
                return Err(format!("{file}: sha changed"));
            }
            prev = hex(&sha256(l.as_bytes()));
            n += 1;
        }
        Ok(n)
    }

    fn audit(&self, name: &str, sha: &str) -> Result<(), String> {
        let p = self.audit_path();
        let (n, prev) = self.last_link()?;
        let new = !p.exists();
        let mut f = OpenOptions::new().create(true).append(true).open(&p).map_err(|e| e.to_string())?;
        if new {
            f.write_all(b"seq\tfile\tsha256\twriter\tist\tprev_line_sha256\n").map_err(|e| e.to_string())?;
        }
        let line = format!("{}\t{name}\t{sha}\t{}\t{}\t{prev}\n", n + 1, self.writer, crate::ist_now());
        f.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())
    }

    /// Verifies the chain and every file's sha. Returns the lines checked.
    ///
    /// # Errors
    /// The first broken link or changed file.
    pub fn verify(&self) -> Result<u64, String> {
        let t = fs::read_to_string(self.audit_path()).map_err(|e| e.to_string())?;
        let mut prev = GENESIS.to_string();
        let mut n = 0_u64;
        for l in t.lines().filter(|l| !l.starts_with("seq\t")) {
            let f: Vec<&str> = l.split('\t').collect();
            let (Some(file), Some(sha), Some(link)) = (f.get(1), f.get(2), f.get(5)) else { return Err(format!("audit line short: {l}")) };
            if *link != prev {
                return Err(format!("audit chain broken at {file}"));
            }
            let b = fs::read(self.dir.join(file)).map_err(|e| e.to_string())?;
            if hex(&sha256(&b)) != *sha {
                return Err(format!("{file}: sha changed"));
            }
            prev = hex(&sha256(l.as_bytes()));
            n += 1;
        }
        Ok(n)
    }
}
