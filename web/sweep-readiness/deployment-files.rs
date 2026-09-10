//! Bounded byte verification for the standalone staging observer.
use std::collections::BTreeSet;
use std::fs::{self, File, Metadata};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Parse bounded report JSON without accepting last-key-wins ambiguity.
/// Serde's default finite recursion limit remains enabled. Integer visitors keep
/// exact i64/u64 values; decimal metadata uses serde_json's existing float form.
pub(super) fn json(raw: &[u8], ceiling: u64) -> Result<serde_json::Value, String> {
    use serde::Deserialize;
    if raw.len() as u64 > ceiling {
        return Err("JSON input exceeds its byte limit".into());
    }
    let mut parser = serde_json::Deserializer::from_slice(raw);
    let UniqueJson(value) = UniqueJson::deserialize(&mut parser)
        .map_err(|_| "JSON input is malformed, duplicated or too deeply nested")?;
    parser
        .end()
        .map_err(|_| "JSON input has trailing content")?;
    Ok(value)
}

struct UniqueJson(serde_json::Value);
impl<'de> serde::Deserialize<'de> for UniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(parser: D) -> Result<Self, D::Error> {
        parser.deserialize_any(UniqueVisitor)
    }
}

struct UniqueVisitor;
impl<'de> serde::de::Visitor<'de> for UniqueVisitor {
    type Value = UniqueJson;
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("JSON with unique object keys")
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(UniqueJson(serde_json::Value::Null))
    }
    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueJson(serde_json::Value::Bool(value)))
    }
    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueJson(serde_json::Value::Number(value.into())))
    }
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueJson(serde_json::Value::Number(value.into())))
    }
    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
        serde_json::Number::from_f64(value)
            .map(|value| UniqueJson(serde_json::Value::Number(value)))
            .ok_or_else(|| E::custom("nonfinite JSON number"))
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.visit_string(value.to_owned())
    }
    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueJson(serde_json::Value::String(value)))
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        self,
        mut items: A,
    ) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(UniqueJson(value)) = items.next_element()? {
            values.push(value);
        }
        Ok(UniqueJson(serde_json::Value::Array(values)))
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        mut items: A,
    ) -> Result<Self::Value, A::Error> {
        let mut values = serde_json::Map::new();
        while let Some(key) = items.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(serde::de::Error::custom("duplicate JSON object key"));
            }
            let UniqueJson(value) = items.next_value()?;
            values.insert(key, value);
        }
        Ok(UniqueJson(serde_json::Value::Object(values)))
    }
}

#[cfg(not(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
)))]
compile_error!("staging observer requires verified macOS or Linux x86_64/aarch64 file flags");

#[derive(Debug)]
pub(super) struct Verified {
    file: File,
    path: PathBuf,
    before: Metadata,
}

/// Finite tree census retained until response settlement, not a writer lease.
#[derive(Debug)]
pub(super) struct Inventory {
    root: PathBuf,
    expected: BTreeSet<PathBuf>,
    limit: usize,
    rust_only: bool,
}
impl Inventory {
    pub(super) fn capture(root: &Path, limit: usize, rust_only: bool) -> Result<Self, String> {
        let mut expected = tree(root, limit)?;
        if rust_only {
            expected.retain(|path| path.extension().is_some_and(|v| v == "rs"));
        }
        Ok(Self {
            root: root.to_path_buf(),
            expected,
            limit,
            rust_only,
        })
    }
    pub(super) fn entries(&self) -> &BTreeSet<PathBuf> {
        &self.expected
    }
    fn require_current(&self) -> Result<(), String> {
        let current = Self::capture(&self.root, self.limit, self.rust_only)?;
        if current.expected != self.expected {
            return Err("previously verified complete file census changed during preflight".into());
        }
        Ok(())
    }
}

/// Held bytes plus complete inventories. Rechecks scan bounded trees and files;
/// overall preflight work is not constant and does not prevent future writes.
#[derive(Debug)]
pub(super) struct Retained {
    pub files: Vec<Verified>,
    pub inventories: Vec<Inventory>,
}
impl Retained {
    pub(super) fn extend(&mut self, mut other: Self) {
        self.files.append(&mut other.files);
        self.inventories.append(&mut other.inventories);
    }
    pub(super) fn require_current(&self) -> Result<(), String> {
        for inventory in &self.inventories {
            inventory.require_current()?;
        }
        for file in &self.files {
            file.require_current()?;
        }
        Ok(())
    }
}
impl Verified {
    pub(super) fn require_current(&self) -> Result<(), String> {
        if generation(&self.before) != generation(&current(&self.file, &self.path)?) {
            return Err("previously verified staged artifact changed during preflight".into());
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
const FLAGS: i32 = 0x100 | 0x4;
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const FLAGS: i32 = 0x20_000 | 0x800;

fn generation(meta: &Metadata) -> (u64, u64, u64, i64, i64, i64, i64) {
    use std::os::unix::fs::MetadataExt;
    (
        meta.dev(),
        meta.ino(),
        meta.len(),
        meta.mtime(),
        meta.mtime_nsec(),
        meta.ctime(),
        meta.ctime_nsec(),
    )
}

fn open(path: &Path) -> Result<File, String> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(FLAGS)
        .open(path)
        .map_err(|_| "staged file cannot be opened without following its final symlink")?;
    current(&file, path)?;
    Ok(file)
}

fn current(file: &File, path: &Path) -> Result<Metadata, String> {
    let held = file
        .metadata()
        .map_err(|_| "staged file metadata unavailable")?;
    let named = fs::symlink_metadata(path).map_err(|_| "staged file disappeared")?;
    if !held.is_file() || !named.is_file() || generation(&held) != generation(&named) {
        return Err("staged file is nonregular, replaced or changing".into());
    }
    Ok(held)
}

pub(super) fn bytes(path: &Path, ceiling: u64) -> Result<Vec<u8>, String> {
    let mut file = open(path)?;
    let before = current(&file, path)?;
    if before.len() > ceiling {
        return Err("bounded metadata file exceeds its byte limit".into());
    }
    let size = usize::try_from(before.len()).map_err(|_| "file byte count is not addressable")?;
    let mut out = Vec::new();
    out.try_reserve_exact(size)
        .map_err(|_| "metadata allocation refused")?;
    file.by_ref()
        .take(ceiling.saturating_add(1))
        .read_to_end(&mut out)
        .map_err(|_| "metadata read refused")?;
    if out.len() as u64 != before.len() || generation(&before) != generation(&current(&file, path)?)
    {
        return Err("metadata changed while being read".into());
    }
    Ok(out)
}

pub(super) fn verify(path: &Path, count: u64, expected: &str) -> Result<Verified, String> {
    let mut file = open(path)?;
    let before = current(&file, path)?;
    if before.len() != count {
        return Err("staged file exact length differs".into());
    }
    let mut hash = brutex_core::blake3::Hasher::new();
    let mut left = count;
    let mut buffer = [0; 65_536];
    while left > 0 {
        let n =
            usize::try_from(left.min(buffer.len() as u64)).map_err(|_| "block width invalid")?;
        file.read_exact(&mut buffer[..n])
            .map_err(|_| "staged file shortened or became unreadable")?;
        hash.update(&buffer[..n]);
        left -= n as u64;
    }
    let actual: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if actual != expected || generation(&before) != generation(&current(&file, path)?) {
        return Err("staged file digest or generation differs".into());
    }
    Ok(Verified {
        file,
        path: path.to_path_buf(),
        before,
    })
}

pub(super) fn tree(root: &Path, limit: usize) -> Result<BTreeSet<PathBuf>, String> {
    let mut files = BTreeSet::new();
    let mut directories = vec![(root.to_path_buf(), 0_u8)];
    let mut entries = 0_usize;
    while let Some((dir, depth)) = directories.pop() {
        if depth > 16 {
            return Err("bundle directory nesting exceeds its finite bound".into());
        }
        for entry in fs::read_dir(&dir).map_err(|_| "bundle directory unreadable")? {
            let entry = entry.map_err(|_| "bundle entry unreadable")?;
            entries = entries.checked_add(1).ok_or("bundle count overflow")?;
            if entries > limit {
                return Err("bundle entry limit exceeded".into());
            }
            let kind = entry
                .file_type()
                .map_err(|_| "bundle entry type unavailable")?;
            if kind.is_dir() {
                directories.push((entry.path(), depth + 1));
            } else if kind.is_file() {
                files.insert(entry.path());
            } else {
                return Err("bundle contains a symlink or nonregular entry".into());
            }
        }
    }
    Ok(files)
}
