//! Fail-closed proof that a commit stamp names the source Cargo is compiling.
//!
//! This module is shared verbatim by `build.rs` and its integration tests.  It
//! starts no process.  Git's index and object database are the authority: HEAD,
//! index, and working tree must describe the same non-front-end tree before a
//! stamp is returned.

use crate::commit_stamp::canonical;
use flate2::read::ZlibDecoder;
use sha1::{Digest as _, Sha1};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

const OID_LEN: usize = 20;
const INDEX_HEADER: usize = 12;
const INDEX_ENTRY_FIXED: usize = 62;
const PACK_INDEX_HEADER: usize = 8 + 256 * 4;
const MAX_DELTA_DEPTH: usize = 128;

#[derive(Clone, Debug)]
pub(crate) struct Verification {
    pub(crate) commit: Option<String>,
    pub(crate) watched_files: Vec<PathBuf>,
    pub(crate) watched_directories: Vec<PathBuf>,
    pub(crate) reason: String,
}

#[derive(Clone, Debug)]
struct Repository {
    root: PathBuf,
    git_dir: PathBuf,
    common_dir: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    mode: u32,
    oid: [u8; OID_LEN],
}

#[derive(Clone, Debug)]
struct Index {
    entries: BTreeMap<String, Entry>,
}

#[derive(Clone, Debug)]
struct Object {
    kind: &'static str,
    data: Vec<u8>,
}

/// Proves the stamp or returns an unstamped decision with a terse cause.
///
/// An explicit value has no authority of its own. It must be canonical, equal
/// the locally resolvable HEAD, and pass the same tree proof as an automatic
/// stamp. `None` selects HEAD but does not weaken any other check.
// The proof is intentionally linear: every early return is one fail-closed
// gate, and splitting those gates across helpers would hide their order.
#[allow(clippy::too_many_lines)]
pub(crate) fn verify(manifest_dir: &Path, explicit: Option<&OsStr>) -> Verification {
    let Some(repo) = repository(manifest_dir) else {
        return refused("repository metadata is unavailable");
    };

    let mut watched_files = vec![repo.git_dir.join("HEAD"), repo.git_dir.join("index")];
    if repo.common_dir == repo.git_dir {
        watched_files.push(repo.git_dir.join("packed-refs"));
    } else {
        watched_files.push(repo.common_dir.join("packed-refs"));
    }

    let Some(head) = head_commit(&repo, &mut watched_files) else {
        return refusal_with_watch(
            watched_files,
            Vec::new(),
            "HEAD is not one canonical SHA-1 commit",
        );
    };
    if let Some(raw) = explicit {
        let Some(given) = raw.to_str() else {
            return refusal_with_watch(
                watched_files,
                Vec::new(),
                "the explicit stamp is not UTF-8",
            );
        };
        if !canonical(given) {
            return refusal_with_watch(
                watched_files,
                Vec::new(),
                "the explicit stamp is not 40 lowercase hexadecimal characters",
            );
        }
        if given != head {
            return refusal_with_watch(
                watched_files,
                Vec::new(),
                "the explicit stamp does not equal local HEAD",
            );
        }
    }

    let index_path = repo.git_dir.join("index");
    let Some(index) = read_index(&index_path) else {
        return refusal_with_watch(watched_files, Vec::new(), "the Git index cannot be proved");
    };
    let store = ObjectStore::new(&repo.common_dir);
    let Some(commit) = store.read_oid(&head) else {
        return refusal_with_watch(
            watched_files,
            Vec::new(),
            "the HEAD commit object cannot be proved",
        );
    };
    if commit.kind != "commit" {
        return refusal_with_watch(
            watched_files,
            Vec::new(),
            "HEAD does not name a commit object",
        );
    }
    let Some(tree_oid) = commit_tree(&commit.data) else {
        return refusal_with_watch(
            watched_files,
            Vec::new(),
            "the HEAD commit has no canonical tree",
        );
    };
    let mut head_entries = BTreeMap::new();
    if !walk_tree(&store, tree_oid, "", &mut head_entries, 0) {
        return refusal_with_watch(watched_files, Vec::new(), "the HEAD tree cannot be proved");
    }

    let relevant_index = relevant_entries(&index.entries);
    let relevant_head = relevant_entries(&head_entries);
    let watched_directories = watched_directories(&repo.root, relevant_index.keys());
    watched_files.extend(
        relevant_index
            .keys()
            .map(|path| repo.root.join(path))
            .collect::<Vec<_>>(),
    );
    if relevant_index != relevant_head {
        return refusal_with_watch(
            watched_files,
            watched_directories,
            "the index does not equal HEAD",
        );
    }
    if !worktree_matches(&repo.root, &relevant_index) {
        return refusal_with_watch(
            watched_files,
            watched_directories,
            "the working tree does not equal the index",
        );
    }
    if let Some(found) = first_untracked_input(&repo.root, &index.entries) {
        // NAMED, because the predecessor's unnamed "an untracked file could
        // affect compilation" was a refusal an operator could not act on: it
        // fired on a tree `git status` called clean, and finding out which file
        // meant reading this build script.
        return refusal_with_watch(
            watched_files,
            watched_directories,
            &format!("an untracked file could affect compilation: {found}"),
        );
    }

    Verification {
        commit: Some(head),
        watched_files,
        watched_directories,
        reason: String::from("verified clean HEAD"),
    }
}

fn refused(reason: &str) -> Verification {
    Verification {
        commit: None,
        watched_files: Vec::new(),
        watched_directories: Vec::new(),
        reason: reason.to_owned(),
    }
}

fn refusal_with_watch(
    watched_files: Vec<PathBuf>,
    watched_directories: Vec<PathBuf>,
    reason: &str,
) -> Verification {
    Verification {
        commit: None,
        watched_files,
        watched_directories,
        reason: reason.to_owned(),
    }
}

fn repository(manifest_dir: &Path) -> Option<Repository> {
    let mut here = manifest_dir;
    loop {
        let marker = here.join(".git");
        if marker.is_dir() {
            return Some(Repository {
                root: here.to_path_buf(),
                git_dir: marker.clone(),
                common_dir: marker,
            });
        }
        if marker.is_file() {
            let text = fs::read_to_string(&marker).ok()?;
            let raw = text.strip_prefix("gitdir:")?.trim();
            let git_dir = normalize(here.join(raw));
            if !git_dir.is_dir() {
                return None;
            }
            let common_dir = match fs::read_to_string(git_dir.join("commondir")) {
                Ok(common) => normalize(git_dir.join(common.trim())),
                Err(_) => git_dir.clone(),
            };
            return common_dir.is_dir().then_some(Repository {
                root: here.to_path_buf(),
                git_dir,
                common_dir,
            });
        }
        here = here.parent()?;
    }
}

fn normalize(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn head_commit(repo: &Repository, watched: &mut Vec<PathBuf>) -> Option<String> {
    let raw = fs::read_to_string(repo.git_dir.join("HEAD")).ok()?;
    let head = raw.strip_suffix('\n').unwrap_or(&raw);
    let head = head.strip_suffix('\r').unwrap_or(head);
    if !head.starts_with("ref: ") {
        return canonical(head).then(|| head.to_owned());
    }
    let reference = head.strip_prefix("ref: ")?;
    if reference.is_empty()
        || reference.starts_with('/')
        || reference
            .split('/')
            .any(|part| part.is_empty() || part == "..")
    {
        return None;
    }

    // Per-worktree refs live in the worktree git dir; ordinary branch refs live
    // in the common dir. Trying both is format-defined and still fail closed.
    for base in [&repo.git_dir, &repo.common_dir] {
        let loose = base.join(reference);
        watched.push(loose.clone());
        if let Ok(value) = fs::read_to_string(&loose) {
            let value = value.trim_end_matches(['\r', '\n']);
            if canonical(value) {
                return Some(value.to_owned());
            }
        }
    }
    let packed_path = repo.common_dir.join("packed-refs");
    watched.push(packed_path.clone());
    let packed = fs::read_to_string(packed_path).ok()?;
    packed.lines().find_map(|line| {
        if line.starts_with('#') || line.starts_with('^') {
            return None;
        }
        let (oid, name) = line.split_once(' ')?;
        (name == reference && canonical(oid)).then(|| oid.to_owned())
    })
}

fn read_index(path: &Path) -> Option<Index> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() < INDEX_HEADER + OID_LEN || bytes.get(..4)? != b"DIRC" {
        return None;
    }
    let version = be_u32(bytes.get(4..8)?)?;
    if version != 2 && version != 3 {
        return None;
    }
    let body_len = bytes.len().checked_sub(OID_LEN)?;
    let expected = Sha1::digest(bytes.get(..body_len)?);
    if expected.as_slice() != bytes.get(body_len..)? {
        return None;
    }
    let count = usize::try_from(be_u32(bytes.get(8..12)?)?).ok()?;
    let mut cursor = INDEX_HEADER;
    let mut entries = BTreeMap::new();
    for _ in 0..count {
        let start = cursor;
        let fixed = bytes.get(cursor..cursor.checked_add(INDEX_ENTRY_FIXED)?)?;
        let mode = be_u32(fixed.get(24..28)?)?;
        let mut oid = [0_u8; OID_LEN];
        oid.copy_from_slice(fixed.get(40..60)?);
        let flags = be_u16(fixed.get(60..62)?)?;
        cursor = cursor.checked_add(INDEX_ENTRY_FIXED)?;
        if flags & 0x3000 != 0 {
            return None;
        }
        if flags & 0x4000 != 0 {
            // Extended entries carry skip-worktree/intent-to-add state. A
            // sparse or intent-to-add index is not a complete tree proof.
            return None;
        }
        let nul = bytes
            .get(cursor..body_len)?
            .iter()
            .position(|byte| *byte == 0)?;
        let name = std::str::from_utf8(bytes.get(cursor..cursor.checked_add(nul)?)?).ok()?;
        if name.is_empty()
            || name.starts_with('/')
            || name.split('/').any(|part| part.is_empty() || part == "..")
        {
            return None;
        }
        let declared = usize::from(flags & 0x0fff);
        if declared < 0x0fff && declared != name.len() {
            return None;
        }
        cursor = cursor.checked_add(nul)?.checked_add(1)?;
        let consumed = cursor.checked_sub(start)?;
        let padded = consumed.checked_add(7)? & !7;
        cursor = start.checked_add(padded)?;
        if cursor > body_len
            || entries
                .insert(name.to_owned(), Entry { mode, oid })
                .is_some()
        {
            return None;
        }
    }
    // Extensions are permitted only when every one is structurally bounded.
    // Unknown optional extensions have an uppercase first byte by Git's format;
    // lowercase means mandatory and therefore cannot be ignored.
    while cursor < body_len {
        let signature = bytes.get(cursor..cursor.checked_add(4)?)?;
        let size_start = cursor.checked_add(4)?;
        let size_end = cursor.checked_add(8)?;
        let size = usize::try_from(be_u32(bytes.get(size_start..size_end)?)?).ok()?;
        if !signature.first()?.is_ascii_uppercase() {
            return None;
        }
        cursor = cursor.checked_add(8)?.checked_add(size)?;
        if cursor > body_len {
            return None;
        }
    }
    (cursor == body_len).then_some(Index { entries })
}

struct ObjectStore {
    objects: PathBuf,
}

impl ObjectStore {
    fn new(common_dir: &Path) -> Self {
        Self {
            objects: common_dir.join("objects"),
        }
    }

    fn read_oid(&self, hex: &str) -> Option<Object> {
        if !canonical(hex) {
            return None;
        }
        let oid = decode_oid(hex)?;
        self.read_bytes(oid, 0)
    }

    fn read_bytes(&self, oid: [u8; OID_LEN], depth: usize) -> Option<Object> {
        if depth > MAX_DELTA_DEPTH {
            return None;
        }
        let hex = encode_oid(oid);
        let loose = self.objects.join(hex.get(..2)?).join(hex.get(2..)?);
        if loose.is_file() {
            let compressed = fs::read(loose).ok()?;
            let mut decoded = Vec::new();
            ZlibDecoder::new(compressed.as_slice())
                .read_to_end(&mut decoded)
                .ok()?;
            let nul = decoded.iter().position(|byte| *byte == 0)?;
            let (header, rest) = decoded.split_at(nul);
            let data = rest.get(1..)?;
            let header = std::str::from_utf8(header).ok()?;
            let (kind, size) = header.split_once(' ')?;
            let kind = object_kind(kind)?;
            if size.parse::<usize>().ok()? != data.len() || object_oid(kind, data) != oid {
                return None;
            }
            return Some(Object {
                kind,
                data: data.to_vec(),
            });
        }
        self.read_packed(oid, depth)
    }

    fn read_packed(&self, oid: [u8; OID_LEN], depth: usize) -> Option<Object> {
        let pack_dir = self.objects.join("pack");
        let mut indexes = fs::read_dir(pack_dir)
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension() == Some(OsStr::new("idx")))
            .collect::<Vec<_>>();
        indexes.sort();
        for index_path in indexes {
            let Some(offset) = pack_offset(&index_path, oid) else {
                continue;
            };
            let pack_path = index_path.with_extension("pack");
            let pack = fs::read(pack_path).ok()?;
            let object = self.object_at(&pack, offset, depth, &mut BTreeSet::new())?;
            if object_oid(object.kind, &object.data) == oid {
                return Some(object);
            }
            return None;
        }
        None
    }

    fn object_at(
        &self,
        pack: &[u8],
        offset: usize,
        depth: usize,
        visiting: &mut BTreeSet<usize>,
    ) -> Option<Object> {
        if depth > MAX_DELTA_DEPTH || !visiting.insert(offset) {
            return None;
        }
        if pack.get(..4)? != b"PACK" || !matches!(be_u32(pack.get(4..8)?)?, 2 | 3) {
            return None;
        }
        let mut cursor = offset;
        let first = *pack.get(cursor)?;
        cursor += 1;
        let kind = (first >> 4) & 7;
        let mut size = usize::from(first & 0x0f);
        let mut shift = 4_u32;
        let mut byte = first;
        while byte & 0x80 != 0 {
            byte = *pack.get(cursor)?;
            cursor += 1;
            let part = usize::from(byte & 0x7f).checked_shl(shift)?;
            size = size.checked_add(part)?;
            shift = shift.checked_add(7)?;
            if shift >= usize::BITS {
                return None;
            }
        }

        let base = match kind {
            6 => {
                let mut byte = *pack.get(cursor)?;
                cursor += 1;
                let mut distance = usize::from(byte & 0x7f);
                while byte & 0x80 != 0 {
                    byte = *pack.get(cursor)?;
                    cursor += 1;
                    distance = distance.checked_add(1)?.checked_shl(7)?;
                    distance = distance.checked_add(usize::from(byte & 0x7f))?;
                }
                let base_offset = offset.checked_sub(distance)?;
                Some(self.object_at(pack, base_offset, depth + 1, visiting)?)
            }
            7 => {
                let mut base_oid = [0_u8; OID_LEN];
                base_oid.copy_from_slice(pack.get(cursor..cursor.checked_add(OID_LEN)?)?);
                cursor += OID_LEN;
                Some(self.read_bytes(base_oid, depth + 1)?)
            }
            _ => None,
        };
        let mut inflated = Vec::new();
        ZlibDecoder::new(pack.get(cursor..)?)
            .read_to_end(&mut inflated)
            .ok()?;
        let result = match kind {
            1..=4 => Object {
                kind: object_kind_code(kind)?,
                data: inflated,
            },
            6 | 7 => {
                if inflated.len() != size {
                    return None;
                }
                let base = base?;
                Object {
                    kind: base.kind,
                    data: apply_delta(&base.data, &inflated)?,
                }
            }
            _ => return None,
        };
        visiting.remove(&offset);
        ((kind == 6 || kind == 7) || result.data.len() == size).then_some(result)
    }
}

fn pack_offset(path: &Path, wanted: [u8; OID_LEN]) -> Option<usize> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() < PACK_INDEX_HEADER + 40
        || bytes.get(..4)? != [0xff, b't', b'O', b'c']
        || be_u32(bytes.get(4..8)?)? != 2
    {
        return None;
    }
    let body_len = bytes.len().checked_sub(OID_LEN)?;
    if Sha1::digest(bytes.get(..body_len)?).as_slice() != bytes.get(body_len..)? {
        return None;
    }
    let count = usize::try_from(be_u32(bytes.get(8 + 255 * 4..8 + 256 * 4)?)?).ok()?;
    let hashes_start = PACK_INDEX_HEADER;
    let hashes_end = hashes_start.checked_add(count.checked_mul(OID_LEN)?)?;
    let hashes = bytes.get(hashes_start..hashes_end)?;
    let mut low = 0_usize;
    let mut high = count;
    while low < high {
        let middle = low + (high - low) / 2;
        let start = middle.checked_mul(OID_LEN)?;
        let candidate = hashes.get(start..start.checked_add(OID_LEN)?)?;
        match candidate.cmp(wanted.as_slice()) {
            std::cmp::Ordering::Less => low = middle + 1,
            std::cmp::Ordering::Greater => high = middle,
            std::cmp::Ordering::Equal => {
                low = middle;
                high = middle;
            }
        }
    }
    let start = low.checked_mul(OID_LEN)?;
    if hashes.get(start..start.checked_add(OID_LEN)?)? != wanted {
        return None;
    }
    let found = low;
    let offsets_start = hashes_end.checked_add(count.checked_mul(4)?)?;
    let raw = be_u32(
        bytes.get(
            offsets_start.checked_add(found.checked_mul(4)?)?
                ..offsets_start
                    .checked_add(found.checked_mul(4)?)?
                    .checked_add(4)?,
        )?,
    )?;
    if raw & 0x8000_0000 == 0 {
        return usize::try_from(raw).ok();
    }
    let large_index = usize::try_from(raw & 0x7fff_ffff).ok()?;
    let large_start = offsets_start.checked_add(count.checked_mul(4)?)?;
    usize::try_from(be_u64(
        bytes.get(
            large_start.checked_add(large_index.checked_mul(8)?)?
                ..large_start
                    .checked_add(large_index.checked_mul(8)?)?
                    .checked_add(8)?,
        )?,
    )?)
    .ok()
}

fn apply_delta(base: &[u8], delta: &[u8]) -> Option<Vec<u8>> {
    let mut cursor = 0;
    let base_size = delta_varint(delta, &mut cursor)?;
    let result_size = delta_varint(delta, &mut cursor)?;
    if base_size != base.len() {
        return None;
    }
    let mut out = Vec::with_capacity(result_size);
    while cursor < delta.len() {
        let opcode = *delta.get(cursor)?;
        cursor += 1;
        if opcode & 0x80 == 0 {
            let length = usize::from(opcode);
            if length == 0 {
                return None;
            }
            out.extend_from_slice(delta.get(cursor..cursor.checked_add(length)?)?);
            cursor += length;
            continue;
        }
        let mut offset = 0_usize;
        let mut size = 0_usize;
        for (bit, shift) in [(0x01, 0), (0x02, 8), (0x04, 16), (0x08, 24)] {
            if opcode & bit != 0 {
                offset |= usize::from(*delta.get(cursor)?) << shift;
                cursor += 1;
            }
        }
        for (bit, shift) in [(0x10, 0), (0x20, 8), (0x40, 16)] {
            if opcode & bit != 0 {
                size |= usize::from(*delta.get(cursor)?) << shift;
                cursor += 1;
            }
        }
        if size == 0 {
            size = 0x1_0000;
        }
        out.extend_from_slice(base.get(offset..offset.checked_add(size)?)?);
    }
    (out.len() == result_size).then_some(out)
}

fn delta_varint(bytes: &[u8], cursor: &mut usize) -> Option<usize> {
    let mut value = 0_usize;
    let mut shift = 0_u32;
    loop {
        let byte = *bytes.get(*cursor)?;
        *cursor += 1;
        value = value.checked_add(usize::from(byte & 0x7f).checked_shl(shift)?)?;
        if byte & 0x80 == 0 {
            return Some(value);
        }
        shift = shift.checked_add(7)?;
        if shift >= usize::BITS {
            return None;
        }
    }
}

fn commit_tree(commit: &[u8]) -> Option<[u8; OID_LEN]> {
    let text = std::str::from_utf8(commit).ok()?;
    let line = text.lines().next()?;
    let tree = line.strip_prefix("tree ")?;
    canonical(tree).then(|| decode_oid(tree)).flatten()
}

fn walk_tree(
    store: &ObjectStore,
    oid: [u8; OID_LEN],
    prefix: &str,
    out: &mut BTreeMap<String, Entry>,
    depth: usize,
) -> bool {
    if depth > 128 {
        return false;
    }
    let Some(tree) = store.read_bytes(oid, 0) else {
        return false;
    };
    if tree.kind != "tree" {
        return false;
    }
    let mut cursor = 0;
    while cursor < tree.data.len() {
        let Some(rest) = tree.data.get(cursor..) else {
            return false;
        };
        let Some(space) = rest.iter().position(|byte| *byte == b' ') else {
            return false;
        };
        let Some(mode_end) = cursor.checked_add(space) else {
            return false;
        };
        let Some(mode_bytes) = tree.data.get(cursor..mode_end) else {
            return false;
        };
        let Ok(mode_text) = std::str::from_utf8(mode_bytes) else {
            return false;
        };
        let Ok(mode) = u32::from_str_radix(mode_text, 8) else {
            return false;
        };
        let Some(after_space) = space.checked_add(1) else {
            return false;
        };
        let Some(next) = cursor.checked_add(after_space) else {
            return false;
        };
        cursor = next;
        let Some(rest) = tree.data.get(cursor..) else {
            return false;
        };
        let Some(nul) = rest.iter().position(|byte| *byte == 0) else {
            return false;
        };
        let Some(name_end) = cursor.checked_add(nul) else {
            return false;
        };
        let Some(name_bytes) = tree.data.get(cursor..name_end) else {
            return false;
        };
        let Ok(name) = std::str::from_utf8(name_bytes) else {
            return false;
        };
        if name.is_empty() || name.contains('/') || name == "." || name == ".." {
            return false;
        }
        let Some(after_nul) = nul.checked_add(1) else {
            return false;
        };
        let Some(next) = cursor.checked_add(after_nul) else {
            return false;
        };
        cursor = next;
        let Some(oid_end) = cursor.checked_add(OID_LEN) else {
            return false;
        };
        let Some(raw_oid) = tree.data.get(cursor..oid_end) else {
            return false;
        };
        let mut child = [0_u8; OID_LEN];
        child.copy_from_slice(raw_oid);
        cursor = oid_end;
        let path = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}/{name}")
        };
        if mode == 0o40000 {
            if !walk_tree(store, child, &path, out, depth + 1) {
                return false;
            }
        } else if !matches!(mode, 0o100_644 | 0o100_755 | 0o120_000 | 0o160_000)
            || out.insert(path, Entry { mode, oid: child }).is_some()
        {
            return false;
        }
    }
    true
}

fn relevant_entries(entries: &BTreeMap<String, Entry>) -> BTreeMap<String, Entry> {
    entries
        .iter()
        .filter(|(path, _)| !path.starts_with("web/"))
        .map(|(path, entry)| (path.clone(), entry.clone()))
        .collect()
}

fn worktree_matches(root: &Path, entries: &BTreeMap<String, Entry>) -> bool {
    entries.iter().all(|(path, expected)| {
        let full = root.join(path);
        match expected.mode {
            0o100_644 | 0o100_755 => {
                let Ok(metadata) = fs::symlink_metadata(&full) else {
                    return false;
                };
                if !metadata.file_type().is_file()
                    || executable(&metadata) != (expected.mode == 0o100_755)
                {
                    return false;
                }
                fs::read(full).is_ok_and(|bytes| object_oid("blob", &bytes) == expected.oid)
            }
            0o120_000 => fs::read_link(full)
                .ok()
                .and_then(|target| target.to_str().map(str::as_bytes).map(ToOwned::to_owned))
                .is_some_and(|bytes| object_oid("blob", &bytes) == expected.oid),
            // A submodule has a second repository state. Refusing is safer than
            // treating the directory bytes as the gitlink's commit.
            _ => false,
        }
    })
}

#[cfg(unix)]
fn executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable(_metadata: &fs::Metadata) -> bool {
    false
}

fn watched_directories<'a>(root: &Path, paths: impl Iterator<Item = &'a String>) -> Vec<PathBuf> {
    let mut directories = BTreeSet::new();
    directories.insert(root.to_path_buf());
    for path in paths {
        let mut parent = Path::new(path).parent();
        while let Some(relative) = parent {
            if relative.as_os_str().is_empty() {
                break;
            }
            directories.insert(root.join(relative));
            parent = relative.parent();
        }
    }
    directories.into_iter().collect()
}

#[derive(Clone)]
/// One parsed `.gitignore` line.
///
/// **This type exists because there were two ignore policies and only one of
/// them was git's.** The predecessor was a hardcoded list matching whole first
/// path segments, so `.gitignore`'s `/logs.pre-wd-black-*`,
/// `/mutants.out*.pre-wd-black-*` and `/.claude/*` all slipped past it: `git
/// status` reported a clean tree while this build script reported "an untracked
/// file could affect compilation" and disabled run persistence on every build.
/// A second copy of a policy is correct the day it is written and silently
/// wrong afterwards, which is the shape `AGENTS.md` §5 exists to refuse.
struct IgnoreRule {
    /// A leading `!`: this rule UN-ignores. Last matching rule wins, as in git.
    negated: bool,
    /// A trailing `/`: matches directories only.
    directory_only: bool,
    /// The pattern contained a `/`, so it is matched against the path relative
    /// to [`Self::base`] rather than against a basename at any depth.
    anchored: bool,
    pattern: String,
    /// The repository-relative directory of the `.gitignore` this rule came
    /// from, with a trailing `/`, or empty for the root file.
    ///
    /// **Git reads a `.gitignore` in EVERY directory**, and its patterns are
    /// relative to that directory. Reading only the root file made
    /// `web/.gitignore`'s `.svelte-kit/` invisible, so a `SvelteKit` build
    /// directory that `git status` correctly ignores refused the stamp.
    base: String,
}

/// Reads the `.gitignore` in `directory`, whose repository-relative path is
/// `base` (empty for the root, otherwise ending in `/`).
///
/// A missing or unreadable file yields no rules, which is the conservative
/// direction: nothing becomes ignored that was not ignored before, so the guard
/// stays at least as strict as it was.
fn load_ignore_rules(directory: &Path, base: &str) -> Vec<IgnoreRule> {
    let Ok(text) = fs::read_to_string(directory.join(".gitignore")) else {
        return Vec::new();
    };
    let mut rules = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (negated, rest) = match line.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, line),
        };
        let (directory_only, rest) = match rest.strip_suffix('/') {
            Some(rest) => (true, rest),
            None => (false, rest),
        };
        if rest.is_empty() {
            continue;
        }
        let anchored = rest.contains('/');
        let pattern = rest.strip_prefix('/').unwrap_or(rest);
        if pattern.is_empty() {
            continue;
        }
        rules.push(IgnoreRule {
            negated,
            directory_only,
            anchored,
            pattern: pattern.to_owned(),
            base: base.to_owned(),
        });
    }
    rules
}

/// Whether `path` is ignored, with the last matching rule winning.
///
/// Rules arrive root-first, so a nested `.gitignore` is appended after the
/// rules it may override -- which is git's own precedence, expressed as
/// ordering rather than as a special case.
fn ignored(rules: &[IgnoreRule], path: &str, is_directory: bool) -> bool {
    let mut verdict = false;
    for rule in rules {
        if rule.directory_only && !is_directory {
            continue;
        }
        // A rule reaches only paths beneath its own `.gitignore`.
        let Some(relative) = path.strip_prefix(rule.base.as_str()) else {
            continue;
        };
        let name = relative.rsplit('/').next().unwrap_or(relative);
        let hit = if rule.anchored {
            glob_matches(&rule.pattern, relative)
        } else {
            glob_matches(&rule.pattern, name)
        };
        if hit {
            verdict = !rule.negated;
        }
    }
    verdict
}

/// Glob match over `*`, `?` and `**`, which is the whole of `.gitignore`'s
/// syntax this repository uses. A single `*` stops at a separator; `**` crosses
/// them, and a leading `**/` also matches zero directories.
fn glob_matches(pattern: &str, text: &str) -> bool {
    glob_at(pattern.as_bytes(), text.as_bytes())
}

fn glob_at(pattern: &[u8], text: &[u8]) -> bool {
    let Some(&head) = pattern.first() else {
        return text.is_empty();
    };
    if head == b'*' {
        if pattern.get(1) == Some(&b'*') {
            let after = pattern.get(2..).unwrap_or_default();
            // `**/` matches zero directories as well as many.
            if after.first() == Some(&b'/') && glob_at(after.get(1..).unwrap_or_default(), text) {
                return true;
            }
            for index in 0..=text.len() {
                if glob_at(after, text.get(index..).unwrap_or_default()) {
                    return true;
                }
            }
            return false;
        }
        let after = pattern.get(1..).unwrap_or_default();
        for index in 0..=text.len() {
            if glob_at(after, text.get(index..).unwrap_or_default()) {
                return true;
            }
            if text.get(index) == Some(&b'/') {
                break;
            }
        }
        return false;
    }
    let Some(&first) = text.first() else {
        return false;
    };
    let rest_pattern = pattern.get(1..).unwrap_or_default();
    let rest_text = text.get(1..).unwrap_or_default();
    if head == b'?' {
        return first != b'/' && glob_at(rest_pattern, rest_text);
    }
    head == first && glob_at(rest_pattern, rest_text)
}

/// The first untracked path that could affect compilation, or `None`.
///
/// Returns the path so the refusal can NAME it. The predecessor returned a
/// bare `bool` and the caller printed "an untracked file could affect
/// compilation" with no path, which is a true statement an operator cannot act
/// on — and, once the ignore policies diverged, was not even true.
fn first_untracked_input(root: &Path, tracked: &BTreeMap<String, Entry>) -> Option<String> {
    let rules = load_ignore_rules(root, "");
    walk_untracked(root, root, "", tracked, &rules)
}

fn walk_untracked(
    root: &Path,
    directory: &Path,
    base: &str,
    tracked: &BTreeMap<String, Entry>,
    inherited: &[IgnoreRule],
) -> Option<String> {
    // This directory's own `.gitignore`, appended so it overrides what it
    // inherits. The root's rules are loaded by the caller, so this only adds
    // rules on the way down and never re-reads the same file.
    let local = if base.is_empty() {
        Vec::new()
    } else {
        load_ignore_rules(directory, base)
    };
    let mut merged;
    let rules: &[IgnoreRule] = if local.is_empty() {
        inherited
    } else {
        merged = inherited.to_vec();
        merged.extend(local);
        &merged
    };
    let Ok(read) = fs::read_dir(directory) else {
        return Some(format!("{} cannot be listed", directory.display()));
    };
    let mut paths = Vec::new();
    for found in read {
        let Ok(entry) = found else {
            // Losing even one directory entry means we cannot prove there is
            // no untracked build input behind that error.
            return Some(format!("{} has an unreadable entry", directory.display()));
        };
        paths.push(entry.path());
    }
    paths.sort();
    for path in paths {
        let Ok(relative) = path.strip_prefix(root) else {
            return Some(format!("{} is outside the repository", path.display()));
        };
        let Some(word) = relative.to_str().map(|value| value.replace('\\', "/")) else {
            return Some(format!("{} is not valid UTF-8", path.display()));
        };
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            return Some(format!("{word} cannot be stat'd"));
        };
        let is_directory = metadata.file_type().is_dir();
        // `.git` is never named in `.gitignore` because git does not list its
        // own store; every other exclusion now comes from the one file that
        // decides what git itself ignores.
        if word == ".git" || ignored(rules, &word, is_directory) {
            continue;
        }
        if is_directory {
            let child_base = format!("{word}/");
            if let Some(found) = walk_untracked(root, &path, &child_base, tracked, rules) {
                return Some(found);
            }
        } else if !tracked.contains_key(&word) {
            return Some(word);
        }
    }
    None
}

fn object_kind(value: &str) -> Option<&'static str> {
    match value {
        "commit" => Some("commit"),
        "tree" => Some("tree"),
        "blob" => Some("blob"),
        "tag" => Some("tag"),
        _ => None,
    }
}

fn object_kind_code(value: u8) -> Option<&'static str> {
    match value {
        1 => Some("commit"),
        2 => Some("tree"),
        3 => Some("blob"),
        4 => Some("tag"),
        _ => None,
    }
}

fn object_oid(kind: &str, data: &[u8]) -> [u8; OID_LEN] {
    let mut hasher = Sha1::new();
    hasher.update(kind.as_bytes());
    hasher.update(b" ");
    hasher.update(data.len().to_string().as_bytes());
    hasher.update([0]);
    hasher.update(data);
    hasher.finalize().into()
}

fn decode_oid(value: &str) -> Option<[u8; OID_LEN]> {
    if !canonical(value) {
        return None;
    }
    let mut out = [0_u8; OID_LEN];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(*pair.first()?)?;
        let low = hex_nibble(*pair.get(1)?)?;
        *out.get_mut(index)? = (high << 4) | low;
    }
    Some(out)
}

fn encode_oid(value: [u8; OID_LEN]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(40);
    for byte in value {
        out.push(char::from(
            *HEX.get(usize::from(byte >> 4)).unwrap_or(&b'0'),
        ));
        out.push(char::from(
            *HEX.get(usize::from(byte & 0x0f)).unwrap_or(&b'0'),
        ));
    }
    out
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn be_u16(bytes: &[u8]) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.try_into().ok()?))
}

fn be_u32(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.try_into().ok()?))
}

fn be_u64(bytes: &[u8]) -> Option<u64> {
    Some(u64::from_be_bytes(bytes.try_into().ok()?))
}

// Test fixtures deliberately use direct byte assembly and assertion helpers;
// those operations are confined to synthetic repositories under the OS temp
// directory and never enter the build-time verifier.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        entries: BTreeMap<String, Entry>,
        head: String,
    }

    impl Fixture {
        fn new() -> Self {
            let serial = NEXT.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("brutex-provenance-{}-{serial}", std::process::id()));
            fs::create_dir_all(root.join(".git/objects")).expect("objects");
            fs::create_dir_all(root.join(".git/refs/heads")).expect("refs");

            // `.gitignore` is TRACKED here because it is tracked in the real
            // repository -- `AGENTS.md` §2 admits it by name. It has to be: the
            // walker now takes its exclusions from this file and nowhere else,
            // so an untracked copy would be an untracked file that decides
            // which untracked files count. The patterns below are the SHAPES
            // the real file uses, and the three `*`-bearing ones are exactly
            // the shapes the predecessor's first-segment equality could not
            // match.
            const IGNORE: &[u8] = b"/target\n\
                /.claude/*\n\
                !/.claude/launch.json\n\
                /logs.pre-wd-black-*\n\
                /mutants.out*.pre-wd-black-*\n\
                **/*.log\n";
            let files = [
                (".gitignore", IGNORE),
                ("Cargo.toml", b"[workspace]\n".as_slice()),
                // A NESTED `.gitignore`, tracked, exactly as `web/.gitignore` is.
                // Git reads one in every directory and applies its patterns
                // relative to that directory; reading only the root file made
                // `web/.gitignore`'s `.svelte-kit/` invisible.
                ("crates/cli/.gitignore", b"scratch/\n*.tmp\n".as_slice()),
                (
                    "crates/cli/src/lib.rs",
                    b"pub fn fixture() -> u8 { 1 }\n".as_slice(),
                ),
            ];
            let mut entries = BTreeMap::new();
            for (path, bytes) in files {
                let full = root.join(path);
                fs::create_dir_all(full.parent().expect("file parent")).expect("parent");
                fs::write(&full, bytes).expect("working file");
                let oid = write_object(&root, "blob", bytes);
                entries.insert(
                    path.to_owned(),
                    Entry {
                        mode: 0o100_644,
                        oid,
                    },
                );
            }

            let lib = entries["crates/cli/src/lib.rs"].oid;
            let source_tree = write_tree(&root, &[(0o100_644, "lib.rs", lib)]);
            let nested = entries["crates/cli/.gitignore"].oid;
            let cli_tree = write_tree(
                &root,
                &[
                    (0o100_644, ".gitignore", nested),
                    (0o40000, "src", source_tree),
                ],
            );
            let crates_tree = write_tree(&root, &[(0o40000, "cli", cli_tree)]);
            let manifest = entries["Cargo.toml"].oid;
            let ignore = entries[".gitignore"].oid;
            // Git orders tree entries by name in byte order: `.` (0x2E) before
            // `C` (0x43) before `c` (0x63). `write_tree` writes what it is
            // given, so the order is the caller's to get right.
            let root_tree = write_tree(
                &root,
                &[
                    (0o100_644, ".gitignore", ignore),
                    (0o100_644, "Cargo.toml", manifest),
                    (0o40000, "crates", crates_tree),
                ],
            );
            let commit = format!(
                "tree {}\nauthor Test <test@example.invalid> 0 +0000\ncommitter Test <test@example.invalid> 0 +0000\n\nfixture\n",
                encode_oid(root_tree)
            );
            let head = encode_oid(write_object(&root, "commit", commit.as_bytes()));
            fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").expect("HEAD");
            fs::write(root.join(".git/refs/heads/main"), format!("{head}\n")).expect("head ref");
            write_index(&root, &entries);
            Self {
                root,
                entries,
                head,
            }
        }

        fn manifest(&self) -> PathBuf {
            self.root.join("crates/cli")
        }

        fn verify(&self, explicit: Option<&str>) -> Verification {
            verify(&self.manifest(), explicit.map(OsStr::new))
        }

        fn dirty(&self, path: &str, bytes: &[u8]) {
            fs::write(self.root.join(path), bytes).expect("dirty file");
        }

        fn stage(&mut self, path: &str, bytes: &[u8]) {
            self.dirty(path, bytes);
            let oid = write_object(&self.root, "blob", bytes);
            self.entries.get_mut(path).expect("tracked path").oid = oid;
            write_index(&self.root, &self.entries);
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn write_object(root: &Path, kind: &str, data: &[u8]) -> [u8; OID_LEN] {
        let oid = object_oid(kind, data);
        let hex = encode_oid(oid);
        let directory = root.join(".git/objects").join(&hex[..2]);
        fs::create_dir_all(&directory).expect("object fanout");
        let mut plain = format!("{kind} {}\0", data.len()).into_bytes();
        plain.extend_from_slice(data);
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&plain).expect("compress object");
        let compressed = encoder.finish().expect("finish object");
        fs::write(directory.join(&hex[2..]), compressed).expect("loose object");
        oid
    }

    fn write_tree(root: &Path, entries: &[(u32, &str, [u8; OID_LEN])]) -> [u8; OID_LEN] {
        let mut data = Vec::new();
        for (mode, name, oid) in entries {
            write!(&mut data, "{mode:o} {name}").expect("tree header");
            data.push(0);
            data.extend_from_slice(oid);
        }
        write_object(root, "tree", &data)
    }

    fn write_index(root: &Path, entries: &BTreeMap<String, Entry>) {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"DIRC");
        bytes.extend_from_slice(&2_u32.to_be_bytes());
        bytes.extend_from_slice(
            &u32::try_from(entries.len())
                .expect("entry count")
                .to_be_bytes(),
        );
        for (path, entry) in entries {
            let start = bytes.len();
            bytes.extend_from_slice(&[0; 24]);
            bytes.extend_from_slice(&entry.mode.to_be_bytes());
            bytes.extend_from_slice(&[0; 12]);
            bytes.extend_from_slice(&entry.oid);
            let path_len = u16::try_from(path.len()).expect("fixture path");
            bytes.extend_from_slice(&path_len.to_be_bytes());
            bytes.extend_from_slice(path.as_bytes());
            bytes.push(0);
            while (bytes.len() - start) % 8 != 0 {
                bytes.push(0);
            }
        }
        let checksum = Sha1::digest(&bytes);
        bytes.extend_from_slice(&checksum);
        fs::write(root.join(".git/index"), bytes).expect("index");
    }

    #[derive(Clone, Copy, Debug)]
    enum DeltaFixture {
        Plain,
        Ofs,
        Reference,
    }

    struct Packed {
        oid: [u8; OID_LEN],
        kind: u8,
        data: Vec<u8>,
        loose: PathBuf,
        offset: u32,
    }

    struct DeltaPlan {
        fixture: DeltaFixture,
        base_oid: [u8; OID_LEN],
        target_oid: [u8; OID_LEN],
        base_data: Vec<u8>,
        target_data: Vec<u8>,
    }

    fn read_loose_objects(object_root: &Path) -> Vec<Packed> {
        let mut objects = Vec::new();
        for fanout in fs::read_dir(object_root).expect("object fanout listing") {
            let fanout = fanout.expect("fanout entry");
            if !fanout.file_type().expect("fanout type").is_dir()
                || fanout.file_name() == OsStr::new("pack")
            {
                continue;
            }
            for file in fs::read_dir(fanout.path()).expect("loose listing") {
                let file = file.expect("loose entry");
                let mut decoded = Vec::new();
                ZlibDecoder::new(fs::read(file.path()).expect("loose bytes").as_slice())
                    .read_to_end(&mut decoded)
                    .expect("loose inflate");
                let nul = decoded
                    .iter()
                    .position(|byte| *byte == 0)
                    .expect("header nul");
                let header = std::str::from_utf8(&decoded[..nul]).expect("object header");
                let (kind, _) = header.split_once(' ').expect("header fields");
                let kind = match kind {
                    "commit" => 1,
                    "tree" => 2,
                    "blob" => 3,
                    "tag" => 4,
                    _ => panic!("unexpected fixture object kind"),
                };
                let hex = format!(
                    "{}{}",
                    fanout.file_name().to_string_lossy(),
                    file.file_name().to_string_lossy()
                );
                objects.push(Packed {
                    oid: decode_oid(&hex).expect("loose oid"),
                    kind,
                    data: decoded[nul + 1..].to_vec(),
                    loose: file.path(),
                    offset: 0,
                });
            }
        }
        objects.sort_by_key(|object| object.oid);
        objects
    }

    fn plan_delta(objects: &mut [Packed], fixture: DeltaFixture) -> Option<DeltaPlan> {
        if matches!(fixture, DeltaFixture::Plain) {
            return None;
        }
        let (base_oid, target_oid, base_data, target_data) = {
            let mut blobs = objects.iter().filter(|object| object.kind == 3);
            let base = blobs.next().expect("fixture base blob");
            let target = blobs.next().expect("fixture target blob");
            (base.oid, target.oid, base.data.clone(), target.data.clone())
        };
        let rank = |oid| {
            if oid == base_oid {
                0_u8
            } else if oid == target_oid {
                2_u8
            } else {
                1_u8
            }
        };
        objects.sort_by_key(|object| (rank(object.oid), object.oid));
        Some(DeltaPlan {
            fixture,
            base_oid,
            target_oid,
            base_data,
            target_data,
        })
    }

    fn write_fixture_pack(
        objects: &mut [Packed],
        delta: Option<&DeltaPlan>,
    ) -> (Vec<u8>, [u8; OID_LEN]) {
        let mut pack = Vec::new();
        pack.extend_from_slice(b"PACK");
        pack.extend_from_slice(&2_u32.to_be_bytes());
        pack.extend_from_slice(
            &u32::try_from(objects.len())
                .expect("pack count")
                .to_be_bytes(),
        );
        let mut base_offset = None;
        for object in &mut *objects {
            object.offset = u32::try_from(pack.len()).expect("small fixture pack");
            if delta.is_some_and(|plan| object.oid == plan.base_oid) {
                base_offset = Some(usize::try_from(object.offset).expect("fixture offset"));
            }
            let (kind, prefix, payload) = pack_payload(object, delta, base_offset);
            write_pack_header(&mut pack, kind, payload.len());
            pack.extend_from_slice(&prefix);
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&payload).expect("pack compress");
            pack.extend_from_slice(&encoder.finish().expect("finish pack object"));
        }
        let pack_checksum: [u8; OID_LEN] = Sha1::digest(&pack).into();
        pack.extend_from_slice(&pack_checksum);
        (pack, pack_checksum)
    }

    fn pack_payload(
        object: &Packed,
        delta: Option<&DeltaPlan>,
        base_offset: Option<usize>,
    ) -> (u8, Vec<u8>, Vec<u8>) {
        let Some(plan) = delta.filter(|plan| object.oid == plan.target_oid) else {
            return (object.kind, Vec::new(), object.data.clone());
        };
        let payload = literal_delta(&plan.base_data, &plan.target_data);
        match plan.fixture {
            DeltaFixture::Ofs => {
                let distance = usize::try_from(object.offset)
                    .expect("fixture offset")
                    .checked_sub(base_offset.expect("base precedes delta"))
                    .expect("positive delta distance");
                (6, encode_ofs_distance(distance), payload)
            }
            DeltaFixture::Reference => (7, plan.base_oid.to_vec(), payload),
            DeltaFixture::Plain => unreachable!("plain packs have no delta plan"),
        }
    }

    fn write_fixture_pack_index(objects: &mut [Packed], pack_checksum: [u8; OID_LEN]) -> Vec<u8> {
        objects.sort_by_key(|object| object.oid);
        let mut index = Vec::new();
        index.extend_from_slice(&[0xff, b't', b'O', b'c']);
        index.extend_from_slice(&2_u32.to_be_bytes());
        for first in 0_u8..=u8::MAX {
            let count = objects
                .iter()
                .filter(|object| object.oid[0] <= first)
                .count();
            index.extend_from_slice(&u32::try_from(count).expect("fanout count").to_be_bytes());
        }
        for object in &*objects {
            index.extend_from_slice(&object.oid);
        }
        for _ in &*objects {
            index.extend_from_slice(&0_u32.to_be_bytes());
        }
        for object in &*objects {
            index.extend_from_slice(&object.offset.to_be_bytes());
        }
        index.extend_from_slice(&pack_checksum);
        let index_checksum = Sha1::digest(&index);
        index.extend_from_slice(&index_checksum);
        index
    }

    fn move_loose_objects_to_pack(root: &Path, delta_fixture: DeltaFixture) {
        let object_root = root.join(".git/objects");
        let mut objects = read_loose_objects(&object_root);
        let delta = plan_delta(&mut objects, delta_fixture);
        let (pack, pack_checksum) = write_fixture_pack(&mut objects, delta.as_ref());
        let index = write_fixture_pack_index(&mut objects, pack_checksum);
        let pack_dir = object_root.join("pack");
        fs::create_dir_all(&pack_dir).expect("pack directory");
        fs::write(pack_dir.join("pack-fixture.pack"), pack).expect("pack file");
        fs::write(pack_dir.join("pack-fixture.idx"), index).expect("pack index");
        for object in objects {
            fs::remove_file(object.loose).expect("remove loose object");
        }
    }

    fn literal_delta(base: &[u8], target: &[u8]) -> Vec<u8> {
        let mut delta = Vec::new();
        write_delta_varint(&mut delta, base.len());
        write_delta_varint(&mut delta, target.len());
        for literal in target.chunks(127) {
            delta.push(u8::try_from(literal.len()).expect("literal length"));
            delta.extend_from_slice(literal);
        }
        delta
    }

    fn write_delta_varint(out: &mut Vec<u8>, mut value: usize) {
        loop {
            let mut byte = u8::try_from(value & 0x7f).expect("delta varint group");
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if value == 0 {
                return;
            }
        }
    }

    fn encode_ofs_distance(mut distance: usize) -> Vec<u8> {
        assert!(distance != 0, "an OFS delta must point backwards");
        let mut reverse = vec![u8::try_from(distance & 0x7f).expect("OFS group")];
        distance >>= 7;
        while distance != 0 {
            distance -= 1;
            reverse.push(0x80 | u8::try_from(distance & 0x7f).expect("OFS group"));
            distance >>= 7;
        }
        reverse.reverse();
        reverse
    }

    fn write_pack_header(out: &mut Vec<u8>, kind: u8, mut size: usize) {
        let mut first = (kind << 4) | u8::try_from(size & 0x0f).expect("size nibble");
        size >>= 4;
        if size != 0 {
            first |= 0x80;
        }
        out.push(first);
        while size != 0 {
            let mut byte = u8::try_from(size & 0x7f).expect("size group");
            size >>= 7;
            if size != 0 {
                byte |= 0x80;
            }
            out.push(byte);
        }
    }

    #[test]
    fn only_full_lowercase_sha1_is_canonical() {
        assert!(canonical("0123456789abcdef0123456789abcdef01234567"));
        for invalid in [
            "",
            "0123456",
            "0123456789abcdef0123456789abcdef0123456g",
            "0123456789ABCDEF0123456789ABCDEF01234567",
            " 0123456789abcdef0123456789abcdef01234567",
            "0123456789abcdef0123456789abcdef01234567\n",
        ] {
            assert!(!canonical(invalid), "{invalid:?}");
        }
    }

    #[test]
    fn a_clean_head_is_automatically_and_explicitly_proved() {
        let fixture = Fixture::new();
        assert_eq!(fixture.verify(None).commit.as_deref(), Some(&*fixture.head));
        assert_eq!(
            fixture.verify(Some(&fixture.head)).commit.as_deref(),
            Some(&*fixture.head)
        );
    }

    #[test]
    fn invalid_and_mismatched_explicit_stamps_refuse() {
        let fixture = Fixture::new();
        for invalid in [
            "",
            "0123456",
            "0123456789abcdef0123456789abcdef0123456g",
            "0123456789ABCDEF0123456789ABCDEF01234567",
            "ffffffffffffffffffffffffffffffffffffffff",
        ] {
            assert!(
                fixture.verify(Some(invalid)).commit.is_none(),
                "{invalid:?}"
            );
        }
    }

    #[test]
    fn an_unstaged_tracked_change_refuses() {
        let fixture = Fixture::new();
        fixture.dirty("crates/cli/src/lib.rs", b"pub fn fixture() -> u8 { 2 }\n");
        assert!(fixture.verify(None).commit.is_none());
    }

    #[test]
    fn a_staged_change_that_matches_the_worktree_still_refuses() {
        let mut fixture = Fixture::new();
        fixture.stage("crates/cli/src/lib.rs", b"pub fn fixture() -> u8 { 3 }\n");
        assert!(fixture.verify(None).commit.is_none());
    }

    #[test]
    fn an_untracked_compilation_input_refuses() {
        let fixture = Fixture::new();
        let path = fixture.root.join("crates/cli/src/untracked.rs");
        fs::write(path, b"pub const UNTRACKED: bool = true;\n").expect("untracked source");
        assert!(fixture.verify(None).commit.is_none());
    }

    #[test]
    fn excluded_build_output_does_not_create_a_false_dirty_result() {
        let fixture = Fixture::new();
        let path = fixture.root.join("target/generated.rs");
        fs::create_dir_all(path.parent().expect("target parent")).expect("target");
        fs::write(path, b"generated\n").expect("generated output");
        assert_eq!(fixture.verify(None).commit.as_deref(), Some(&*fixture.head));
    }

    /// **The regression test for the defect this walker was rewritten to fix.**
    ///
    /// All three of these directories are ignored by `.gitignore`, so `git
    /// status` calls the tree clean -- and the predecessor still refused,
    /// because it matched whole first path segments against a hardcoded list
    /// and `logs.pre-wd-black-20260831` is not `logs`. On the real repository
    /// that disabled run persistence on every single build, and the reason it
    /// printed named no file, so the operator had a clean `git status` and an
    /// unstamped binary with nothing connecting them.
    #[test]
    fn gitignored_scratch_beside_a_matching_name_does_not_disable_persistence() {
        let fixture = Fixture::new();
        for directory in [
            ".claude/worktrees.pre-wd-black-20260831",
            "logs.pre-wd-black-20260831",
            "mutants.out.pre-wd-black-20260831",
            "mutants.out.old.pre-wd-black-20260831",
        ] {
            let path = fixture.root.join(directory);
            fs::create_dir_all(&path).expect("scratch directory");
            fs::write(path.join("held.rs"), b"scratch\n").expect("scratch file");
        }
        assert_eq!(
            fixture.verify(None).commit.as_deref(),
            Some(&*fixture.head),
            "a directory git ignores cannot affect compilation"
        );
    }

    /// **Git reads a `.gitignore` in every directory, and this once did not.**
    ///
    /// Found by the naming this commit added: on a clean tree the refusal read
    /// `an untracked file could affect compilation: web/.svelte-kit/ambient.d.ts`,
    /// a SvelteKit build directory ignored by `web/.gitignore:2` and therefore
    /// invisible to `git status`. Reading only the root file is a third ignore
    /// policy, one directory further down.
    #[test]
    fn a_nested_gitignore_is_read_and_is_scoped_to_its_own_directory() {
        let fixture = Fixture::new();
        let nested = fixture.root.join("crates/cli/scratch");
        fs::create_dir_all(&nested).expect("nested scratch");
        fs::write(nested.join("held.rs"), b"scratch\n").expect("scratch file");
        fs::write(fixture.root.join("crates/cli/work.tmp"), b"tmp\n").expect("tmp file");
        assert_eq!(
            fixture.verify(None).commit.as_deref(),
            Some(&*fixture.head),
            "crates/cli/.gitignore covers scratch/ and *.tmp"
        );

        // AND IT REACHES NO FURTHER. The same two names at the repository root
        // are outside that file's directory, so the root's rules alone decide,
        // and the root ignores neither.
        fs::create_dir_all(fixture.root.join("scratch")).expect("root scratch");
        fs::write(fixture.root.join("scratch/held.rs"), b"scratch\n").expect("root scratch file");
        let verification = fixture.verify(None);
        assert!(
            verification.commit.is_none(),
            "a nested rule must not leak upward"
        );
        assert!(
            verification.reason.contains("scratch/held.rs"),
            "and the refusal names it: {}",
            verification.reason
        );
    }

    /// A negation re-admits the path, and a re-admitted path is an ordinary
    /// untracked file again. Proves the `!` rule is read rather than skipped.
    #[test]
    fn a_negated_ignore_rule_puts_the_file_back_under_the_guard() {
        let fixture = Fixture::new();
        let claude = fixture.root.join(".claude");
        fs::create_dir_all(&claude).expect(".claude");
        fs::write(claude.join("settings.local.json"), b"{}\n").expect("ignored sibling");
        assert_eq!(
            fixture.verify(None).commit.as_deref(),
            Some(&*fixture.head),
            "/.claude/* covers the sibling"
        );
        fs::write(claude.join("launch.json"), b"{}\n").expect("negated file");
        assert!(
            fixture.verify(None).commit.is_none(),
            "!/.claude/launch.json un-ignores it, so untracked it must refuse"
        );
    }

    /// §4 asks a refusal to name its reason. The predecessor said only "an
    /// untracked file could affect compilation", which is a true sentence an
    /// operator cannot act on.
    #[test]
    fn the_refusal_names_the_untracked_file() {
        let fixture = Fixture::new();
        fs::write(
            fixture.root.join("crates/cli/src/stray.rs"),
            b"pub const STRAY: bool = true;\n",
        )
        .expect("stray source");
        let verification = fixture.verify(None);
        assert!(verification.commit.is_none());
        assert!(
            verification.reason.contains("crates/cli/src/stray.rs"),
            "the reason must name the file, not merely its existence: {}",
            verification.reason
        );
    }

    #[test]
    fn a_single_star_stops_at_a_separator_and_a_double_star_crosses_it() {
        assert!(glob_matches(
            "logs.pre-wd-black-*",
            "logs.pre-wd-black-20260831"
        ));
        assert!(glob_matches(
            "mutants.out*.pre-wd-black-*",
            "mutants.out.old.pre-wd-black-1"
        ));
        assert!(glob_matches(".claude/*", ".claude/settings.local.json"));
        // A single `*` must not swallow a separator, or `/.claude/*` would
        // match everything beneath a nested directory as well.
        assert!(!glob_matches(".claude/*", ".claude/worktrees/held.rs"));
        assert!(glob_matches("**/*.log", "deep/inside/here.log"));
        // `**/` matches zero directories too.
        assert!(glob_matches("**/*.log", "here.log"));
        assert!(!glob_matches("**/*.log", "here.txt"));
        assert!(glob_matches("?arget", "target"));
        assert!(!glob_matches("?arget", "/target"));
        assert!(glob_matches("target", "target"));
        assert!(!glob_matches("target", "targets"));
    }

    #[test]
    fn an_unanchored_rule_matches_a_basename_at_any_depth_and_an_anchored_one_does_not() {
        let rules = load_ignore_rules(Path::new("/does/not/exist"), "");
        assert!(rules.is_empty(), "a missing .gitignore ignores nothing");

        let fixture = Fixture::new();
        let rules = load_ignore_rules(&fixture.root, "");
        assert!(ignored(&rules, "target", true), "/target is anchored");
        assert!(
            !ignored(&rules, "crates/cli/target", true),
            "an anchored rule must not match deeper"
        );
        assert!(
            ignored(&rules, "crates/pull/session.log", false),
            "**/*.log reaches any depth"
        );
        assert!(
            !ignored(&rules, "crates/cli/src/lib.rs", false),
            "real source is never ignored"
        );
    }

    #[test]
    fn a_clean_pack_only_clone_is_automatically_proved() {
        let fixture = Fixture::new();
        move_loose_objects_to_pack(&fixture.root, DeltaFixture::Plain);
        assert_eq!(fixture.verify(None).commit.as_deref(), Some(&*fixture.head));
    }

    #[test]
    fn clean_ofs_and_ref_delta_packs_are_proved() {
        for delta_fixture in [DeltaFixture::Ofs, DeltaFixture::Reference] {
            let fixture = Fixture::new();
            move_loose_objects_to_pack(&fixture.root, delta_fixture);
            assert_eq!(
                fixture.verify(None).commit.as_deref(),
                Some(&*fixture.head),
                "{delta_fixture:?}"
            );
        }
    }

    #[test]
    fn the_checkout_head_tree_object_is_decodable() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo = repository(manifest).expect("test runs in a checkout");
        let mut watched = Vec::new();
        let head = head_commit(&repo, &mut watched).expect("HEAD");
        let store = ObjectStore::new(&repo.common_dir);
        let commit = store.read_oid(&head).expect("commit object");
        let tree = commit_tree(&commit.data).expect("commit tree");
        let mut entries = BTreeMap::new();
        assert!(walk_tree(&store, tree, "", &mut entries, 0));
        assert!(!entries.is_empty());
    }
}
