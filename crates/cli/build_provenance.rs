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

/// Whether a repository-relative path lies under the front end, `web/`.
///
/// `CLAUDE.md` §2 forbids any crate to build from `web/`, so nothing there can
/// be a compilation input. **This is the ONE statement of that boundary, and
/// both halves of the proof ask it.** The tracked comparison used to spell it
/// inline while the untracked walk did not spell it at all, so a tree whose
/// tracked `web/` edits were correctly ignored still lost its stamp to an
/// untracked `web/` build artifact, under the false reason "could affect
/// compilation". Two copies of one boundary is the shape [`IgnoreRule`]'s own
/// history already records going wrong. D-0691.
fn front_end(path: &str) -> bool {
    path.starts_with("web/")
}

fn relevant_entries(entries: &BTreeMap<String, Entry>) -> BTreeMap<String, Entry> {
    entries
        .iter()
        .filter(|(path, _)| !front_end(path))
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
            // The front end is outside the proof for the same reason the
            // tracked comparison drops it, so nothing beneath it is listed.
            // Asked of the directory's `word/` spelling, which is exactly the
            // prefix every path beneath it carries: `web/` itself is skipped, a
            // nested `crates/x/web/` or a sibling `webpack/` is not, and a
            // plain FILE named `web` is still an ordinary untracked input.
            if front_end(&child_base) {
                continue;
            }
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
        /// HEAD's root tree: the first object below the commit that `verify`
        /// decodes, so a delta or a damaged body here is on the proof's path.
        tree: [u8; OID_LEN],
    }

    fn commit_text(tree: [u8; OID_LEN]) -> String {
        format!(
            "tree {}\nauthor Test <test@example.invalid> 0 +0000\ncommitter Test <test@example.invalid> 0 +0000\n\nfixture\n",
            encode_oid(tree)
        )
    }

    impl Fixture {
        fn new() -> Self {
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
            let serial = NEXT.fetch_add(1, Ordering::Relaxed);
            let root = fresh_directory(
                std::env::temp_dir()
                    .join(format!("brutex-provenance-{}-{serial}", std::process::id())),
            );
            fs::create_dir_all(root.join(".git/objects")).expect("objects");
            fs::create_dir_all(root.join(".git/refs/heads")).expect("refs");

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
            let commit = commit_text(root_tree);
            let head = encode_oid(write_object(&root, "commit", commit.as_bytes()));
            fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").expect("HEAD");
            fs::write(root.join(".git/refs/heads/main"), format!("{head}\n")).expect("head ref");
            write_index(&root, &entries);
            Self {
                root,
                entries,
                head,
                tree: root_tree,
            }
        }

        /// Commits `entries` as they now stand: trees, commit, branch, index.
        fn recommit(&mut self) {
            self.tree = write_tree_of(&self.root, &self.entries, "");
            let commit = commit_text(self.tree);
            self.head = encode_oid(write_object(&self.root, "commit", commit.as_bytes()));
            fs::write(
                self.root.join(".git/refs/heads/main"),
                format!("{}\n", self.head),
            )
            .expect("head ref");
            write_index(&self.root, &self.entries);
        }

        /// Tracks a symbolic link at `path` whose target is `target`.
        #[cfg(unix)]
        fn track_symlink(&mut self, path: &str, target: &str) {
            std::os::unix::fs::symlink(target, self.root.join(path)).expect("tracked symlink");
            let oid = write_object(&self.root, "blob", target.as_bytes());
            self.entries.insert(
                path.to_owned(),
                Entry {
                    mode: 0o120_000,
                    oid,
                },
            );
            self.recommit();
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

    /// Writes the tree for every entry beneath `prefix` and returns its id.
    /// Git orders a directory as if its name ended in `/`.
    fn write_tree_of(
        root: &Path,
        entries: &BTreeMap<String, Entry>,
        prefix: &str,
    ) -> [u8; OID_LEN] {
        let mut children = BTreeMap::new();
        let mut directories = BTreeSet::new();
        for (path, entry) in entries {
            let Some(rest) = path.strip_prefix(prefix) else {
                continue;
            };
            match rest.split_once('/') {
                Some((directory, _)) => {
                    directories.insert(directory.to_owned());
                }
                None => {
                    children.insert(rest.to_owned(), (entry.mode, rest.to_owned(), entry.oid));
                }
            }
        }
        for directory in directories {
            let oid = write_tree_of(root, entries, &format!("{prefix}{directory}/"));
            children.insert(format!("{directory}/"), (0o40000, directory, oid));
        }
        let listed = children
            .values()
            .map(|(mode, name, oid)| (*mode, name.as_str(), *oid))
            .collect::<Vec<_>>();
        write_tree(root, &listed)
    }

    /// The index Git writes: a name length of 0xFFF or more is declared 0xFFF.
    fn write_index(root: &Path, entries: &BTreeMap<String, Entry>) {
        write_index_declaring(root, entries, saturated);
    }

    fn write_index_declaring(
        root: &Path,
        entries: &BTreeMap<String, Entry>,
        declared: impl Fn(&str) -> u16,
    ) {
        let bytes = index_bytes(entries, declared, *b"DIRC", 2, &[]);
        fs::write(root.join(".git/index"), bytes).expect("index");
    }

    /// Git's saturating name-length rule, as [`write_index`] applies it.
    fn saturated(path: &str) -> u16 {
        u16::try_from(path.len().min(0x0fff)).expect("saturated name length")
    }

    /// An index's bytes: `magic` and `version`, every entry with the name
    /// length `declared` gives it, each extension as signature, big-endian
    /// size and data, and the SHA-1 trailer over all of it.
    fn index_bytes(
        entries: &BTreeMap<String, Entry>,
        declared: impl Fn(&str) -> u16,
        magic: [u8; 4],
        version: u32,
        extensions: &[([u8; 4], &[u8])],
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&magic);
        bytes.extend_from_slice(&version.to_be_bytes());
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
            bytes.extend_from_slice(&declared(path).to_be_bytes());
            bytes.extend_from_slice(path.as_bytes());
            bytes.push(0);
            while (bytes.len() - start) % 8 != 0 {
                bytes.push(0);
            }
        }
        for (signature, data) in extensions {
            bytes.extend_from_slice(signature);
            bytes.extend_from_slice(
                &u32::try_from(data.len())
                    .expect("extension size")
                    .to_be_bytes(),
            );
            bytes.extend_from_slice(data);
        }
        let checksum = Sha1::digest(&bytes);
        bytes.extend_from_slice(&checksum);
        bytes
    }

    /// How HEAD's root tree is stored in a fixture pack.
    #[derive(Clone, Copy, Debug)]
    enum DeltaFixture {
        /// Every object whole.
        Plain,
        /// The root tree as an `OFS_DELTA` against a padded base tree.
        Ofs,
        /// The root tree as a `REF_DELTA` naming that base by id.
        Reference,
        /// The root tree whole, under its own id, but with bytes that are not
        /// that id's: see [`misnamed_tree`].
        Misnamed,
    }

    /// HEAD's root tree with its `crates` entry's mode written `040000`
    /// instead of `40000`. Octal parsing reads both as the same mode, so these
    /// bytes list exactly the entries HEAD's tree lists, and only their id
    /// differs from the one HEAD names.
    fn misnamed_tree(tree: &[u8]) -> Vec<u8> {
        let at = tree
            .windows(13)
            .position(|window| window == b"40000 crates\0")
            .expect("root tree lists crates");
        let mut misnamed = tree[..at].to_vec();
        misnamed.push(b'0');
        misnamed.extend_from_slice(&tree[at..]);
        misnamed
    }

    /// Which offset table a fixture pack index uses for its entries.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Offsets {
        /// The 31-bit table, as every pack under 2 GiB has.
        Small,
        /// Every entry through the 64-bit table: the high bit set in the
        /// 31-bit slot and the low bits naming the 64-bit row.
        Large,
    }

    /// Where a packed entry's bytes come from.
    #[derive(Clone, Copy, Debug)]
    enum Base {
        Whole,
        /// An `OFS_DELTA` against the entry at this position in the same pack.
        Ofs(usize),
        /// A `REF_DELTA` naming its base by object id.
        Ref([u8; OID_LEN]),
    }

    struct PackEntry {
        oid: [u8; OID_LEN],
        /// The object's own kind code, 1..=4; a delta is written as 6 or 7.
        kind: u8,
        base: Base,
        /// The object's bytes when whole, its delta instructions otherwise.
        payload: Vec<u8>,
        /// The inflated size the entry header declares.
        declared: usize,
    }

    impl PackEntry {
        fn whole(kind: u8, data: Vec<u8>) -> Self {
            let name = object_kind_code(kind).expect("whole object kind");
            Self {
                oid: object_oid(name, &data),
                kind,
                base: Base::Whole,
                declared: data.len(),
                payload: data,
            }
        }

        fn delta(oid: [u8; OID_LEN], kind: u8, base: Base, delta: Vec<u8>) -> Self {
            Self {
                oid,
                kind,
                base,
                declared: delta.len(),
                payload: delta,
            }
        }
    }

    /// One delta instruction, encoded by [`encode_delta`] exactly as Git does.
    #[derive(Clone, Copy)]
    enum Op<'a> {
        Insert(&'a [u8]),
        Copy { offset: usize, size: usize },
    }

    fn encode_delta(base_len: usize, result_len: usize, ops: &[Op<'_>]) -> Vec<u8> {
        let mut delta = Vec::new();
        write_delta_varint(&mut delta, base_len);
        write_delta_varint(&mut delta, result_len);
        for op in ops {
            match *op {
                Op::Insert(bytes) => {
                    for literal in bytes.chunks(127) {
                        delta.push(u8::try_from(literal.len()).expect("literal length"));
                        delta.extend_from_slice(literal);
                    }
                }
                Op::Copy { offset, size } => {
                    assert!(u32::try_from(offset).is_ok(), "four offset bytes");
                    assert!(size <= 0x00ff_ffff || size == 0x1_0000, "three size bytes");
                    // A size of exactly 0x10000 is written with NO size byte.
                    let size = if size == 0x1_0000 { 0 } else { size };
                    let mut opcode = 0x80_u8;
                    let mut arguments = Vec::new();
                    for (index, bit) in [0x01_u8, 0x02, 0x04, 0x08].into_iter().enumerate() {
                        let byte = (offset >> (8 * index)) & 0xff;
                        if byte != 0 {
                            opcode |= bit;
                            arguments.push(u8::try_from(byte).expect("offset byte"));
                        }
                    }
                    for (index, bit) in [0x10_u8, 0x20, 0x40].into_iter().enumerate() {
                        let byte = (size >> (8 * index)) & 0xff;
                        if byte != 0 {
                            opcode |= bit;
                            arguments.push(u8::try_from(byte).expect("size byte"));
                        }
                    }
                    delta.push(opcode);
                    delta.extend_from_slice(&arguments);
                }
            }
        }
        delta
    }

    /// Writes `entries`, in order, as `pack-<name>.pack` beside a version-2
    /// index, and returns each entry's pack offset.
    fn write_pack(
        object_root: &Path,
        name: &str,
        entries: &[PackEntry],
        offsets: Offsets,
    ) -> Vec<usize> {
        let mut pack = Vec::new();
        pack.extend_from_slice(b"PACK");
        pack.extend_from_slice(&2_u32.to_be_bytes());
        pack.extend_from_slice(
            &u32::try_from(entries.len())
                .expect("pack count")
                .to_be_bytes(),
        );
        let mut at = Vec::with_capacity(entries.len());
        for entry in entries {
            let offset = pack.len();
            at.push(offset);
            let (kind, prefix) = match entry.base {
                Base::Whole => (entry.kind, Vec::new()),
                Base::Ofs(position) => (6, encode_ofs_distance(offset - at[position])),
                Base::Ref(oid) => (7, oid.to_vec()),
            };
            write_pack_header(&mut pack, kind, entry.declared);
            pack.extend_from_slice(&prefix);
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&entry.payload).expect("pack compress");
            pack.extend_from_slice(&encoder.finish().expect("finish pack object"));
        }
        let pack_checksum: [u8; OID_LEN] = Sha1::digest(&pack).into();
        pack.extend_from_slice(&pack_checksum);

        let mut sorted = entries
            .iter()
            .zip(&at)
            .map(|(entry, offset)| (entry.oid, *offset))
            .collect::<Vec<_>>();
        sorted.sort_unstable();
        let mut index = Vec::new();
        index.extend_from_slice(&[0xff, b't', b'O', b'c']);
        index.extend_from_slice(&2_u32.to_be_bytes());
        for first in 0_u8..=u8::MAX {
            let count = sorted.iter().filter(|(oid, _)| oid[0] <= first).count();
            index.extend_from_slice(&u32::try_from(count).expect("fanout count").to_be_bytes());
        }
        for (oid, _) in &sorted {
            index.extend_from_slice(oid);
        }
        for _ in &sorted {
            index.extend_from_slice(&0_u32.to_be_bytes());
        }
        for (row, (_, offset)) in sorted.iter().enumerate() {
            let slot = match offsets {
                Offsets::Small => u32::try_from(*offset).expect("small fixture pack"),
                Offsets::Large => 0x8000_0000 | u32::try_from(row).expect("large row"),
            };
            index.extend_from_slice(&slot.to_be_bytes());
        }
        if offsets == Offsets::Large {
            for (_, offset) in &sorted {
                index.extend_from_slice(&u64::try_from(*offset).expect("offset").to_be_bytes());
            }
        }
        index.extend_from_slice(&pack_checksum);
        let index_checksum = Sha1::digest(&index);
        index.extend_from_slice(&index_checksum);

        let pack_dir = object_root.join("pack");
        fs::create_dir_all(&pack_dir).expect("pack directory");
        fs::write(pack_dir.join(format!("pack-{name}.pack")), pack).expect("pack file");
        fs::write(pack_dir.join(format!("pack-{name}.idx")), index).expect("pack index");
        at
    }

    struct Loose {
        oid: [u8; OID_LEN],
        kind: u8,
        data: Vec<u8>,
        path: PathBuf,
    }

    fn read_loose(path: &Path) -> (String, Vec<u8>) {
        let mut decoded = Vec::new();
        ZlibDecoder::new(fs::read(path).expect("loose bytes").as_slice())
            .read_to_end(&mut decoded)
            .expect("loose inflate");
        let nul = decoded
            .iter()
            .position(|byte| *byte == 0)
            .expect("header nul");
        let header = std::str::from_utf8(&decoded[..nul]).expect("object header");
        let (kind, _) = header.split_once(' ').expect("header fields");
        (kind.to_owned(), decoded[nul + 1..].to_vec())
    }

    fn loose_path(root: &Path, oid: [u8; OID_LEN]) -> PathBuf {
        let hex = encode_oid(oid);
        root.join(".git/objects").join(&hex[..2]).join(&hex[2..])
    }

    fn read_loose_objects(object_root: &Path) -> Vec<Loose> {
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
                let (kind, data) = read_loose(&file.path());
                let kind = match kind.as_str() {
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
                objects.push(Loose {
                    oid: decode_oid(&hex).expect("loose oid"),
                    kind,
                    data,
                    path: file.path(),
                });
            }
        }
        objects.sort_by_key(|object| object.oid);
        objects
    }

    /// Pseudo-random bytes ahead of the tree in the base, so the base puts the
    /// root tree's delta more than one seven-bit `OFS_DELTA` group away -- which
    /// [`move_loose_objects_to_pack`] asserts -- and the copy that reaches past
    /// them needs a second offset byte.
    const PAD: usize = 300;

    fn padding() -> Vec<u8> {
        let mut state = 0x9E37_79B9_u32;
        (0..PAD)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                u8::try_from(state >> 24).expect("top byte")
            })
            .collect()
    }

    /// The root tree's delta against `PAD ‖ tree`: one literal byte, then a
    /// copy of the rest from offset `PAD + 1`. `intact == false` copies from
    /// `PAD` instead, which is in range and the right length and yields the
    /// wrong bytes.
    fn root_tree_delta(tree: &[u8], intact: bool) -> Vec<u8> {
        let offset = if intact { PAD + 1 } else { PAD };
        encode_delta(
            PAD + tree.len(),
            tree.len(),
            &[
                Op::Insert(&tree[..1]),
                Op::Copy {
                    offset,
                    size: tree.len() - 1,
                },
            ],
        )
    }

    /// Moves every loose object into one pack. For `Ofs` and `Reference`,
    /// HEAD's root tree -- which `verify` must decode to walk HEAD -- is stored
    /// as a delta against a padded base tree written ahead of every other
    /// object; for `Misnamed` it is stored whole as [`misnamed_tree`]'s bytes.
    fn move_loose_objects_to_pack(
        fixture: &Fixture,
        delta: DeltaFixture,
        offsets: Offsets,
        intact: bool,
    ) {
        let object_root = fixture.root.join(".git/objects");
        let tree = read_loose(&loose_path(&fixture.root, fixture.tree)).1;
        let base = matches!(delta, DeltaFixture::Ofs | DeltaFixture::Reference).then(|| {
            let mut data = padding();
            data.extend_from_slice(&tree);
            write_object(&fixture.root, "tree", &data)
        });
        let mut loose = read_loose_objects(&object_root);
        let rank = |oid| {
            if Some(oid) == base {
                0_u8
            } else if base.is_some() && oid == fixture.tree {
                2
            } else {
                1
            }
        };
        loose.sort_by_key(|object| (rank(object.oid), object.oid));
        let entries = loose
            .iter()
            .map(|object| match delta {
                DeltaFixture::Ofs if object.oid == fixture.tree => {
                    PackEntry::delta(object.oid, 2, Base::Ofs(0), root_tree_delta(&tree, intact))
                }
                DeltaFixture::Reference if object.oid == fixture.tree => PackEntry::delta(
                    object.oid,
                    2,
                    Base::Ref(base.expect("reference base")),
                    root_tree_delta(&tree, intact),
                ),
                DeltaFixture::Misnamed if object.oid == fixture.tree => {
                    let misnamed = misnamed_tree(&tree);
                    PackEntry {
                        oid: object.oid,
                        kind: 2,
                        base: Base::Whole,
                        declared: misnamed.len(),
                        payload: misnamed,
                    }
                }
                _ => PackEntry::whole(object.kind, object.data.clone()),
            })
            .collect::<Vec<_>>();
        let at = write_pack(&object_root, "fixture", &entries, offsets);
        if matches!(delta, DeltaFixture::Ofs) {
            assert!(
                at[at.len() - 1] - at[0] > 0x7f,
                "the OFS distance must need more than one seven-bit group"
            );
        }
        for object in loose {
            fs::remove_file(object.path).expect("remove loose object");
        }
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

    /// A new, empty directory at `path`. Anything already there was left by an
    /// earlier run killed before its `Drop` under the same process id, and is
    /// removed first, so no test starts on another's leftovers. `create_dir`,
    /// not `create_dir_all`, so a path that reappears in between refuses.
    fn fresh_directory(path: PathBuf) -> PathBuf {
        match fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            Err(why) => panic!("left-behind scratch {}: {why}", path.display()),
        }
        fs::create_dir(&path).expect("scratch directory");
        path
    }

    /// A throwaway directory under the OS temp directory, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let serial = NEXT.fetch_add(1, Ordering::Relaxed);
            Self::at(std::env::temp_dir().join(format!(
                "brutex-provenance-{label}-{}-{serial}",
                std::process::id()
            )))
        }

        fn at(path: PathBuf) -> Self {
            Self(fresh_directory(path))
        }

        /// A scratch directory holding an empty `.git/objects` store.
        fn store(label: &str) -> Self {
            let scratch = Self::new(label);
            fs::create_dir_all(scratch.0.join(".git/objects/pack")).expect("object store");
            scratch
        }

        fn objects(&self) -> ObjectStore {
            ObjectStore::new(&self.0.join(".git"))
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A scratch path left behind by an earlier run -- one killed before its
    /// `Drop`, under the same process id -- is emptied before a test uses it,
    /// never reused with what it held.
    #[test]
    fn a_scratch_path_left_behind_is_emptied_not_reused() {
        let path = std::env::temp_dir().join(format!(
            "brutex-provenance-left-behind-{}",
            std::process::id()
        ));
        fs::create_dir_all(path.join(".git/objects/pack")).expect("left-behind store");
        fs::write(path.join(".git/objects/pack/pack-stale.idx"), b"stale").expect("left behind");
        let scratch = Scratch::at(path);
        assert_eq!(
            fs::read_dir(&scratch.0).expect("scratch listing").count(),
            0,
            "nothing left behind is reused"
        );
    }

    /// A pack in which `links[i]` is a delta against `links[i + 1]` for every
    /// `i < deltas` and the last link is whole, so reading `links[0]` resolves
    /// exactly `deltas` deltas. Written deepest first, so every `OFS_DELTA`
    /// base precedes its delta. Returns the id of `links[0]`.
    fn delta_chain(scratch: &Scratch, deltas: usize, reference: bool) -> [u8; OID_LEN] {
        let data = (0..=deltas)
            .map(|link| format!("chain link {link}\n").into_bytes())
            .collect::<Vec<_>>();
        let oids = data
            .iter()
            .map(|bytes| object_oid("blob", bytes))
            .collect::<Vec<_>>();
        let mut entries = Vec::with_capacity(data.len());
        entries.push(PackEntry::whole(3, data[deltas].clone()));
        for link in (0..deltas).rev() {
            let base = if reference {
                Base::Ref(oids[link + 1])
            } else {
                Base::Ofs(entries.len() - 1)
            };
            let delta = encode_delta(
                data[link + 1].len(),
                data[link].len(),
                &[Op::Insert(&data[link])],
            );
            entries.push(PackEntry::delta(oids[link], 3, base, delta));
        }
        write_pack(
            &scratch.0.join(".git/objects"),
            "chain",
            &entries,
            Offsets::Small,
        );
        oids[0]
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
    /// a `SvelteKit` build directory ignored by `web/.gitignore:2` and therefore
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

    /// **The untracked walk honours the same front-end boundary as the tracked
    /// comparison.** A front-end rebuild writes new hashed chunks under
    /// `web/build/` that are untracked until committed. No crate compiles from
    /// `web/` (`CLAUDE.md` §2), and the tracked comparison already drops it, so
    /// the walk refusing here was a false "could affect compilation" that
    /// unstamped every build after a front-end rebuild. D-0691.
    #[test]
    fn an_untracked_front_end_build_artifact_keeps_the_stamp() {
        let fixture = Fixture::new();
        let chunks = fixture.root.join("web/build/_app/immutable/chunks");
        fs::create_dir_all(&chunks).expect("front-end build directory");
        fs::write(chunks.join("B9xQ2f7a.js"), b"export const chunk = 1;\n").expect("new chunk");
        fs::write(fixture.root.join("web/new-page.rs"), b"fn main() {}\n").expect("web source");
        let verification = fixture.verify(None);
        assert_eq!(
            verification.commit.as_deref(),
            Some(&*fixture.head),
            "nothing under web/ is a compilation input: {}",
            verification.reason
        );
        assert_eq!(verification.reason, "verified clean HEAD");
    }

    /// And the boundary is exactly `web/` at the repository root, no wider:
    /// the same prefix `relevant_entries` drops. A nested directory that is
    /// merely NAMED `web`, a sibling whose name only starts with `web`, and a
    /// plain file named `web` are all still untracked inputs, each refused by
    /// name.
    #[test]
    fn an_untracked_file_outside_the_front_end_still_refuses() {
        let fixture = Fixture::new();
        for path in [
            "crates/cli/web/build/chunk.js",
            "webpack/build/chunk.js",
            "web",
        ] {
            let full = fixture.root.join(path);
            fs::create_dir_all(full.parent().expect("untracked parent")).expect("parent");
            fs::write(&full, b"export const chunk = 1;\n").expect("untracked file");
            let verification = fixture.verify(None);
            assert!(
                verification.commit.is_none(),
                "{path} must refuse the stamp"
            );
            assert_eq!(
                verification.reason,
                format!("an untracked file could affect compilation: {path}")
            );
            fs::remove_file(&full).expect("remove untracked file");
        }
        assert_eq!(
            fixture.verify(None).commit.as_deref(),
            Some(&*fixture.head),
            "with the strays removed the empty directories left behind hold no input"
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
        // `?` is one character; the rest of the pattern must still match.
        assert!(!glob_matches("?arget", "tarxet"));
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
        move_loose_objects_to_pack(&fixture, DeltaFixture::Plain, Offsets::Small, true);
        assert_eq!(fixture.verify(None).commit.as_deref(), Some(&*fixture.head));
    }

    /// **The delta is HEAD's root tree, the first object below the commit that
    /// `verify` decodes.** The predecessor deltified two blobs, and `verify`
    /// reads no blob object -- it hashes the working file instead -- so no test
    /// ever decoded a delta and the `REF_DELTA` branch never ran: `apply_delta`
    /// could return `Some(vec![0])` with the whole suite green. GAP14-55, D-0710.
    #[test]
    fn clean_ofs_and_ref_delta_packs_are_proved() {
        for delta_fixture in [DeltaFixture::Ofs, DeltaFixture::Reference] {
            let fixture = Fixture::new();
            move_loose_objects_to_pack(&fixture, delta_fixture, Offsets::Small, true);
            let verification = fixture.verify(None);
            assert_eq!(
                verification.commit.as_deref(),
                Some(&*fixture.head),
                "{delta_fixture:?}: {}",
                verification.reason
            );
            assert_eq!(verification.reason, "verified clean HEAD");
        }
    }

    /// The converse, and the proof the delta above is on the path: a copy one
    /// byte early is in range and the right length, and yields other bytes --
    /// the tree's first byte twice, so its first entry's mode reads `1100644`
    /// -- whose id is not the one HEAD names, so the tree cannot be proved.
    ///
    /// Two checks refuse those bytes: the packed object's id check, and the
    /// tree walk's mode check behind it. So this test does not pin the id
    /// check on its own; [`a_packed_object_whose_bytes_are_not_its_id_refuses`]
    /// is the case only the id check can refuse.
    #[test]
    fn a_root_tree_delta_that_yields_other_bytes_refuses() {
        for delta_fixture in [DeltaFixture::Ofs, DeltaFixture::Reference] {
            let fixture = Fixture::new();
            let tree = read_loose(&loose_path(&fixture.root, fixture.tree)).1;
            let mut base = padding();
            base.extend_from_slice(&tree);
            let yielded = apply_delta(&base, &root_tree_delta(&tree, false))
                .expect("in range and the right length");
            assert!(yielded.starts_with(b"1100644 "), "{delta_fixture:?}");
            assert_ne!(object_oid("tree", &yielded), fixture.tree);
            move_loose_objects_to_pack(&fixture, delta_fixture, Offsets::Small, false);
            let verification = fixture.verify(None);
            assert!(verification.commit.is_none(), "{delta_fixture:?}");
            assert_eq!(
                verification.reason, "the HEAD tree cannot be proved",
                "{delta_fixture:?}"
            );
        }
    }

    /// A packed object is proved only when its bytes are the id it is stored
    /// under. HEAD's root tree is packed whole under its own id, as
    /// [`misnamed_tree`]'s bytes: walked under their true id they list exactly
    /// HEAD's entries, so no check but the packed id check can tell them from
    /// the tree HEAD names, and it refuses them.
    #[test]
    fn a_packed_object_whose_bytes_are_not_its_id_refuses() {
        let fixture = Fixture::new();
        let tree = read_loose(&loose_path(&fixture.root, fixture.tree)).1;
        let misnamed = misnamed_tree(&tree);
        let true_id = write_object(&fixture.root, "tree", &misnamed);
        assert_ne!(true_id, fixture.tree);
        let mut listed = BTreeMap::new();
        let store = ObjectStore::new(&fixture.root.join(".git"));
        assert!(walk_tree(&store, true_id, "", &mut listed, 0));
        assert_eq!(
            listed, fixture.entries,
            "the misnamed bytes list HEAD's entries"
        );
        fs::remove_file(loose_path(&fixture.root, true_id)).expect("remove true-id copy");

        move_loose_objects_to_pack(&fixture, DeltaFixture::Misnamed, Offsets::Small, true);
        let verification = fixture.verify(None);
        assert!(verification.commit.is_none());
        assert_eq!(verification.reason, "the HEAD tree cannot be proved");
    }

    /// Every entry reached through the 64-bit offset table: the 31-bit slot
    /// carries the high bit and the 64-bit row number, not the offset.
    #[test]
    fn a_pack_index_read_through_its_64_bit_offset_table_is_proved() {
        for delta_fixture in [DeltaFixture::Plain, DeltaFixture::Ofs] {
            let fixture = Fixture::new();
            move_loose_objects_to_pack(&fixture, delta_fixture, Offsets::Large, true);
            let verification = fixture.verify(None);
            assert_eq!(
                verification.commit.as_deref(),
                Some(&*fixture.head),
                "{delta_fixture:?}: {}",
                verification.reason
            );
        }
    }

    /// `MAX_DELTA_DEPTH` deltas resolve and one more does not, on both the
    /// `OFS_DELTA` and the `REF_DELTA` path, so the bound is exactly where it
    /// is declared and each path counts its own depth.
    #[test]
    fn a_delta_chain_resolves_to_exactly_max_delta_depth_and_no_further() {
        for reference in [false, true] {
            let scratch = Scratch::store("chain");
            let head = delta_chain(&scratch, MAX_DELTA_DEPTH, reference);
            let object = scratch
                .objects()
                .read_oid(&encode_oid(head))
                .unwrap_or_else(|| panic!("{MAX_DELTA_DEPTH} deltas, reference {reference}"));
            assert_eq!(object.kind, "blob");
            assert_eq!(object.data, b"chain link 0\n");

            let scratch = Scratch::store("chain-deep");
            let head = delta_chain(&scratch, MAX_DELTA_DEPTH + 1, reference);
            assert!(
                scratch.objects().read_oid(&encode_oid(head)).is_none(),
                "{} deltas, reference {reference}",
                MAX_DELTA_DEPTH + 1
            );
        }
    }

    /// A whole object whose entry header misstates its inflated size, a delta
    /// whose header misstates the delta's own length, and a pack of an
    /// unsupported version are each refused rather than read.
    #[test]
    fn a_packed_entry_whose_header_misstates_it_refuses() {
        let blob = b"whole object\n".to_vec();
        let oid = object_oid("blob", &blob);
        let scratch = Scratch::store("whole-size");
        let mut entry = PackEntry::whole(3, blob);
        entry.declared += 1;
        write_pack(
            &scratch.0.join(".git/objects"),
            "size",
            &[entry],
            Offsets::Small,
        );
        assert!(scratch.objects().read_oid(&encode_oid(oid)).is_none());

        let scratch = Scratch::store("delta-size");
        let head = delta_chain(&scratch, 1, false);
        let pack = scratch.0.join(".git/objects/pack/pack-chain.pack");
        let intact = fs::read(&pack).expect("chain pack");
        assert!(scratch.objects().read_oid(&encode_oid(head)).is_some());
        for version in [1_u32, 4] {
            let mut altered = intact.clone();
            altered[4..8].copy_from_slice(&version.to_be_bytes());
            fs::write(&pack, altered).expect("versioned pack");
            assert!(
                scratch.objects().read_oid(&encode_oid(head)).is_none(),
                "pack version {version}"
            );
        }
        let mut version_three = intact;
        version_three[4..8].copy_from_slice(&3_u32.to_be_bytes());
        fs::write(&pack, version_three).expect("version 3 pack");
        assert!(scratch.objects().read_oid(&encode_oid(head)).is_some());

        let mut entries = vec![PackEntry::whole(3, b"chain link 1\n".to_vec())];
        let delta = encode_delta(13, 13, &[Op::Insert(b"chain link 0\n")]);
        let mut misstated = PackEntry::delta(head, 3, Base::Ofs(0), delta);
        misstated.declared += 1;
        entries.push(misstated);
        write_pack(
            &scratch.0.join(".git/objects"),
            "chain",
            &entries,
            Offsets::Small,
        );
        assert!(scratch.objects().read_oid(&encode_oid(head)).is_none());
    }

    /// Every copy argument byte position, a skipped byte between two present
    /// ones, a copy with no offset byte, and the size Git writes with no size
    /// byte at all (0x10000) each decode to the bytes they name.
    #[test]
    fn every_copy_argument_byte_and_the_implicit_64k_size_decode() {
        let base = (0..0x0100_0010_usize)
            .map(|index| u8::try_from(index % 251).expect("pattern byte"))
            .collect::<Vec<_>>();
        let ops = [
            // Offset bytes one and four present, two and three skipped.
            (0x0100_0002, 5),
            // Offset bytes one to three; no size byte means 0x10000.
            (0x0001_0203, 0x1_0000),
            // Offset byte two alone; size bytes one and three, two skipped.
            (0x0000_0100, 0x01_0001),
            // No offset byte at all; size bytes one and two.
            (0, 0x0203),
        ];
        let mut expected = Vec::new();
        let mut encoded = Vec::new();
        for (offset, size) in ops {
            expected.extend_from_slice(&base[offset..offset + size]);
            expected.extend_from_slice(b"literal");
            encoded.push(Op::Copy { offset, size });
            encoded.push(Op::Insert(b"literal"));
        }
        let delta = encode_delta(base.len(), expected.len(), &encoded);
        assert_eq!(apply_delta(&base, &delta).as_deref(), Some(&*expected));
    }

    /// A delta that does not describe its result exactly is refused, never
    /// truncated, padded or read past.
    #[test]
    fn a_delta_that_does_not_describe_its_result_exactly_refuses() {
        let base = b"0123456789".to_vec();
        let copy = [Op::Copy { offset: 2, size: 4 }];
        assert_eq!(
            apply_delta(&base, &encode_delta(10, 4, &copy)).as_deref(),
            Some(b"2345".as_slice())
        );
        let truncated_insert = {
            let mut delta = encode_delta(10, 4, &[]);
            delta.extend_from_slice(&[4, b'a', b'b']);
            delta
        };
        let truncated_copy = {
            let mut delta = encode_delta(10, 4, &[]);
            delta.extend_from_slice(&[0x80 | 0x01 | 0x10, 2]);
            delta
        };
        let reserved_opcode = {
            let mut delta = encode_delta(10, 0, &[]);
            delta.push(0);
            delta
        };
        for (why, delta) in [
            ("declared base size", encode_delta(9, 4, &copy)),
            ("result longer than declared", encode_delta(10, 3, &copy)),
            ("result shorter than declared", encode_delta(10, 5, &copy)),
            (
                "copy past the base",
                encode_delta(10, 4, &[Op::Copy { offset: 8, size: 4 }]),
            ),
            ("truncated insert", truncated_insert),
            ("truncated copy argument", truncated_copy),
            ("reserved opcode zero", reserved_opcode),
            ("unterminated size", vec![0x80, 0x80]),
            ("size wider than a word", vec![0xff; 11]),
        ] {
            assert_eq!(apply_delta(&base, &delta), None, "{why}");
        }
    }

    /// A tree nested 128 deep below the root is walked and one level more is
    /// refused.
    #[test]
    fn a_tree_is_walked_to_depth_128_and_no_further() {
        for (levels, proved) in [(129_usize, true), (130, false)] {
            let scratch = Scratch::store("deep-tree");
            let blob = write_object(&scratch.0, "blob", b"leaf\n");
            let mut tree = write_tree(&scratch.0, &[(0o100_644, "leaf", blob)]);
            for _ in 1..levels {
                tree = write_tree(&scratch.0, &[(0o40000, "d", tree)]);
            }
            let mut entries = BTreeMap::new();
            assert_eq!(
                walk_tree(&scratch.objects(), tree, "", &mut entries, 0),
                proved,
                "{levels} tree levels"
            );
            if proved {
                let leaf = format!("{}leaf", "d/".repeat(levels - 1));
                assert_eq!(entries.keys().collect::<Vec<_>>(), [&leaf]);
            }
        }
    }

    /// A tree entry must be one plain path segment with a mode Git writes, and
    /// a path may be named once.
    #[test]
    fn a_tree_entry_that_is_not_one_plain_segment_or_mode_refuses() {
        let scratch = Scratch::store("names");
        let blob = write_object(&scratch.0, "blob", b"x\n");
        let proved = |entries: &[(u32, &str, [u8; OID_LEN])]| {
            let tree = write_tree(&scratch.0, entries);
            walk_tree(&scratch.objects(), tree, "", &mut BTreeMap::new(), 0)
        };
        for mode in [0o100_644, 0o100_755, 0o120_000, 0o160_000] {
            assert!(proved(&[(mode, "name", blob)]), "mode {mode:o}");
        }
        assert!(
            proved(&[(0o100_644, "...", blob)]),
            "only . and .. are special"
        );
        for name in ["", ".", "..", "a/b"] {
            assert!(!proved(&[(0o100_644, name, blob)]), "name {name:?}");
        }
        assert!(!proved(&[(0o100_600, "name", blob)]), "mode 100600");
        assert!(
            !proved(&[(0o100_644, "name", blob), (0o100_755, "name", blob)]),
            "a path named twice"
        );
    }

    /// A loose object's body must hash to its name and its header must state
    /// its length. Each is damaged alone on HEAD's root tree, with the other
    /// left true, and either one alone refuses the tree. The damaged body is a
    /// same-length rename, so a verifier that let it through would walk it and
    /// refuse later, for another reason.
    #[test]
    fn a_loose_object_must_hash_to_its_name_and_state_its_length() {
        for body_damaged in [true, false] {
            let fixture = Fixture::new();
            let path = loose_path(&fixture.root, fixture.tree);
            let (kind, mut data) = read_loose(&path);
            let declared = if body_damaged {
                let at = data
                    .windows(10)
                    .position(|window| window == b"Cargo.toml")
                    .expect("root tree names Cargo.toml");
                data[at + 9] = b'm';
                data.len()
            } else {
                data.len() + 1
            };
            let mut plain = format!("{kind} {declared}\0").into_bytes();
            plain.extend_from_slice(&data);
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&plain).expect("compress damaged object");
            fs::write(&path, encoder.finish().expect("finish damaged object"))
                .expect("damaged object");
            let verification = fixture.verify(None);
            assert!(verification.commit.is_none(), "body damaged {body_damaged}");
            assert_eq!(
                verification.reason, "the HEAD tree cannot be proved",
                "body damaged {body_damaged}"
            );
        }
    }

    /// An index entry's declared name length must equal the name unless it is
    /// saturated at 0xFFF, which Git writes for any name that long or longer.
    #[test]
    fn an_index_name_length_must_agree_unless_it_saturates() {
        let fixture = Fixture::new();
        write_index_declaring(&fixture.root, &fixture.entries, |path| {
            let length = u16::try_from(path.len()).expect("fixture path");
            if path == "crates/cli/src/lib.rs" {
                length + 1
            } else {
                length
            }
        });
        let verification = fixture.verify(None);
        assert!(verification.commit.is_none());
        assert_eq!(verification.reason, "the Git index cannot be proved");

        let scratch = Scratch::store("long-name");
        let long = format!("{}file", "d/".repeat(2100));
        assert!(long.len() > 0x0fff);
        let entries = BTreeMap::from([(
            long,
            Entry {
                mode: 0o100_644,
                oid: [7; OID_LEN],
            },
        )]);
        write_index(&scratch.0, &entries);
        let index = read_index(&scratch.0.join(".git/index")).expect("saturated length");
        assert_eq!(index.entries, entries);
        write_index_declaring(&scratch.0, &entries, |_| 0x0ffe);
        assert!(read_index(&scratch.0.join(".git/index")).is_none());
    }

    /// The index format's own bounds: a `DIRC` of version 2 or 3, any number
    /// of entries including none, optional (uppercase) extensions that end
    /// exactly at the trailer, and entry names that are plain relative paths.
    /// Every index here is sealed correctly; the trailer check is held by
    /// [`an_index_is_read_only_whole_and_unconflicted`].
    #[test]
    fn an_index_is_read_only_within_its_formats_bounds() {
        let scratch = Scratch::store("index-format");
        let path = scratch.0.join(".git/index");
        let read = |bytes: Vec<u8>| {
            fs::write(&path, bytes).expect("index");
            read_index(&path).map(|index| index.entries)
        };
        let entries = BTreeMap::from([(
            "src/lib.rs".to_owned(),
            Entry {
                mode: 0o100_644,
                oid: [9; OID_LEN],
            },
        )]);
        let none = BTreeMap::new();
        assert_eq!(
            read(index_bytes(&none, saturated, *b"DIRC", 2, &[])),
            Some(none),
            "an index of no entries is 32 bytes and valid"
        );
        for (why, version, extensions) in [
            ("version 3", 3, &[][..]),
            (
                "an optional extension",
                2,
                &[(*b"TREE", b"cache".as_slice())][..],
            ),
            (
                "two optional extensions",
                2,
                &[(*b"TREE", b"cache".as_slice()), (*b"REUC", b"".as_slice())][..],
            ),
        ] {
            assert_eq!(
                read(index_bytes(
                    &entries, saturated, *b"DIRC", version, extensions
                ))
                .as_ref(),
                Some(&entries),
                "{why}"
            );
        }
        let mut overrun = index_bytes(&entries, saturated, *b"DIRC", 2, &[(*b"TREE", b"cache")]);
        let body = overrun.len() - OID_LEN;
        overrun.truncate(body);
        overrun[body - 9..body - 5].copy_from_slice(&6_u32.to_be_bytes());
        let trailer = Sha1::digest(&overrun);
        overrun.extend_from_slice(&trailer);
        for (why, bytes) in [
            ("magic", index_bytes(&entries, saturated, *b"DIRX", 2, &[])),
            (
                "version 1",
                index_bytes(&entries, saturated, *b"DIRC", 1, &[]),
            ),
            (
                "version 4",
                index_bytes(&entries, saturated, *b"DIRC", 4, &[]),
            ),
            (
                "a mandatory extension",
                index_bytes(&entries, saturated, *b"DIRC", 2, &[(*b"link", b"x")]),
            ),
            ("an extension past the trailer", overrun),
            ("fewer bytes than a header and trailer", b"DIRC".to_vec()),
        ] {
            assert_eq!(read(bytes), None, "{why}");
        }
        for name in ["a/../b", "a//b", "/a", "..", ""] {
            let named = BTreeMap::from([(
                name.to_owned(),
                Entry {
                    mode: 0o100_644,
                    oid: [9; OID_LEN],
                },
            )]);
            assert_eq!(
                read(index_bytes(&named, saturated, *b"DIRC", 2, &[])),
                None,
                "name {name:?}"
            );
        }
    }

    /// HEAD may resolve through packed refs alone, and only the line naming
    /// HEAD's own ref counts. A ref that traverses or has an empty segment is
    /// refused even where the path it spells resolves to a real ref.
    #[test]
    fn a_head_ref_resolves_through_packed_refs_and_never_traverses() {
        let fixture = Fixture::new();
        let git = fixture.root.join(".git");
        fs::remove_file(git.join("refs/heads/main")).expect("loose ref");
        let other = "1".repeat(40);
        fs::write(
            git.join("packed-refs"),
            format!(
                "# pack-refs with: peeled fully-peeled sorted \n{other} refs/heads/aaa\n^{other}\n{} refs/heads/main\n",
                fixture.head
            ),
        )
        .expect("packed refs");
        let verification = fixture.verify(None);
        assert_eq!(
            verification.commit.as_deref(),
            Some(&*fixture.head),
            "{}",
            verification.reason
        );

        fs::create_dir_all(git.join("refs/heads/x")).expect("a directory to traverse");
        fs::write(git.join("refs/heads/main"), format!("{}\n", fixture.head)).expect("loose ref");
        for reference in [
            "refs/heads/x/../main",
            "refs/heads//main",
            "/refs/heads/main",
            "",
        ] {
            fs::write(git.join("HEAD"), format!("ref: {reference}\n")).expect("HEAD");
            let verification = fixture.verify(None);
            assert!(verification.commit.is_none(), "{reference:?}");
            assert_eq!(
                verification.reason, "HEAD is not one canonical SHA-1 commit",
                "{reference:?}"
            );
        }
    }

    /// The one object [`least_pack_index`] lists.
    const LEAST: &[u8] = b"a pack index at its least length\n";

    /// Writes a version-2 pack index `PACK_INDEX_HEADER + 40 - short` bytes
    /// long that lists [`LEAST`] alone, and a pack holding it where that index
    /// says, and returns its id.
    ///
    /// So short an index has no room for its entry table AND its trailer: its
    /// last 20 bytes, which must be the SHA-1 of every byte before them, are
    /// read back as the entry's CRC and offset too, and one byte short, their
    /// first byte is also the last byte of the entry's id. `pack_offset` reads
    /// no fanout word but the last, so the first carries `nonce`, found once by
    /// search so that the SHA-1 names an offset under 1 MiB and, one byte
    /// short, begins with the id's last byte. The assertions below hold those
    /// properties, so nothing about either file is out of format but its
    /// length.
    fn least_pack_index(scratch: &Scratch, short: usize, nonce: u32) -> [u8; OID_LEN] {
        let oid = object_oid("blob", LEAST);
        let mut index = vec![0xff, b't', b'O', b'c'];
        index.extend_from_slice(&2_u32.to_be_bytes());
        index.extend_from_slice(&nonce.to_be_bytes());
        index.extend_from_slice(&[0; 254 * 4]);
        index.extend_from_slice(&1_u32.to_be_bytes());
        index.extend_from_slice(&oid[..OID_LEN - short]);
        let trailer = Sha1::digest(&index);
        index.extend_from_slice(&trailer);
        assert_eq!(index.len(), PACK_INDEX_HEADER + 40 - short);
        assert_eq!(
            index[PACK_INDEX_HEADER..PACK_INDEX_HEADER + OID_LEN],
            oid,
            "short {short}: the entry's id is whole"
        );
        let slot = PACK_INDEX_HEADER + OID_LEN + 4;
        let offset =
            usize::try_from(be_u32(&index[slot..slot + 4]).expect("offset slot")).expect("offset");
        assert!(
            (12..1 << 20).contains(&offset),
            "short {short}: nonce {nonce} names offset {offset}"
        );
        let mut pack = b"PACK".to_vec();
        pack.extend_from_slice(&2_u32.to_be_bytes());
        pack.extend_from_slice(&1_u32.to_be_bytes());
        pack.resize(offset, 0);
        write_pack_header(&mut pack, 3, LEAST.len());
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(LEAST).expect("compress least");
        pack.extend_from_slice(&encoder.finish().expect("finish least"));
        let directory = scratch.0.join(".git/objects/pack");
        fs::write(directory.join("pack-least.pack"), pack).expect("least pack");
        fs::write(directory.join("pack-least.idx"), index).expect("least index");
        oid
    }

    /// A pack index is at least `PACK_INDEX_HEADER + 40` bytes, its header and
    /// its two trailing checksums. One of exactly that length, listing one
    /// object, is read; one a byte shorter, in format in every other way, is
    /// not. The three mutants D-0710 first recorded as survivors on that bound
    /// each fail this test.
    #[test]
    fn a_pack_index_of_exactly_its_least_length_is_read_and_a_byte_shorter_is_not() {
        let scratch = Scratch::store("least-index");
        let oid = least_pack_index(&scratch, 0, 4_929);
        let object = scratch
            .objects()
            .read_oid(&encode_oid(oid))
            .expect("an index of exactly the least length is read");
        assert_eq!((object.kind, object.data.as_slice()), ("blob", LEAST));

        let scratch = Scratch::store("short-index");
        let oid = least_pack_index(&scratch, 1, 235_050);
        assert!(
            scratch.objects().read_oid(&encode_oid(oid)).is_none(),
            "an index a byte shorter is not read"
        );
    }

    /// A repository may hold several packs. An object is looked up in each
    /// index in turn, so the search for one the first index does not hold must
    /// end, whether it sorts before, between or after that index's entries.
    #[test]
    fn an_object_in_a_later_pack_is_found_past_every_miss_in_an_earlier_one() {
        let scratch = Scratch::store("two-packs");
        let objects = scratch.0.join(".git/objects");
        let mut first = (0..3)
            .map(|index| PackEntry::whole(3, format!("first pack {index}\n").into_bytes()))
            .collect::<Vec<_>>();
        first.sort_by_key(|entry| entry.oid);
        write_pack(&objects, "a", &first, Offsets::Small);
        let mut wanted = (0..64)
            .map(|index| PackEntry::whole(3, format!("second pack {index}\n").into_bytes()))
            .collect::<Vec<_>>();
        wanted.sort_by_key(|entry| entry.oid);
        let low = first.first().expect("first").oid;
        let high = first.last().expect("last").oid;
        let below = wanted.iter().position(|entry| entry.oid < low);
        let between = wanted
            .iter()
            .position(|entry| entry.oid > low && entry.oid < high);
        let above = wanted.iter().position(|entry| entry.oid > high);
        let picks = [below, between, above]
            .into_iter()
            .map(|pick| pick.expect("64 ids cover every gap of three"))
            .collect::<Vec<_>>();
        write_pack(&objects, "b", &wanted, Offsets::Small);
        for pick in picks {
            let entry = &wanted[pick];
            let object = scratch
                .objects()
                .read_oid(&encode_oid(entry.oid))
                .expect("found in the second pack");
            assert_eq!(object.data, entry.payload);
        }
        assert!(scratch.objects().read_oid(&"f".repeat(40)).is_none());
    }

    /// Every directory that holds a tracked input is watched, from the root
    /// down, so adding a file anywhere the proof looked reruns it.
    #[test]
    fn the_proof_watches_every_directory_that_holds_a_tracked_input() {
        let fixture = Fixture::new();
        let verification = fixture.verify(None);
        assert_eq!(verification.commit.as_deref(), Some(&*fixture.head));
        assert_eq!(
            verification.watched_directories,
            [
                fixture.root.clone(),
                fixture.root.join("crates"),
                fixture.root.join("crates/cli"),
                fixture.root.join("crates/cli/src"),
            ]
        );
    }

    /// An executable entry is proved by the executable bit on disk as well as
    /// its bytes: clearing the bit on a `100755` entry, or setting it on a
    /// `100644` one, refuses.
    #[cfg(unix)]
    #[test]
    fn an_executable_entry_is_proved_by_its_mode_bit_on_disk() {
        use std::os::unix::fs::PermissionsExt as _;
        let mut fixture = Fixture::new();
        let script = fixture.root.join("crates/cli/run.sh");
        fs::write(&script, b"#!/bin/sh\n").expect("script");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("chmod +x");
        let oid = write_object(&fixture.root, "blob", b"#!/bin/sh\n");
        fixture.entries.insert(
            "crates/cli/run.sh".to_owned(),
            Entry {
                mode: 0o100_755,
                oid,
            },
        );
        fixture.recommit();
        let verification = fixture.verify(None);
        assert_eq!(
            verification.commit.as_deref(),
            Some(&*fixture.head),
            "{}",
            verification.reason
        );
        for (path, mode) in [("crates/cli/run.sh", 0o644), ("Cargo.toml", 0o755)] {
            let full = fixture.root.join(path);
            let before = fs::metadata(&full).expect("mode").permissions();
            fs::set_permissions(&full, fs::Permissions::from_mode(mode)).expect("chmod");
            let verification = fixture.verify(None);
            assert!(verification.commit.is_none(), "{path} {mode:o}");
            assert_eq!(
                verification.reason, "the working tree does not equal the index",
                "{path} {mode:o}"
            );
            fs::set_permissions(&full, before).expect("restore mode");
        }
    }

    /// A `.gitignore` line that begins with `#` is a comment, not a pattern,
    /// so a file of that name is still an untracked input.
    #[test]
    fn a_gitignore_comment_is_not_a_pattern() {
        let mut fixture = Fixture::new();
        fixture.stage("crates/cli/.gitignore", b"scratch/\n*.tmp\n#held.rs\n");
        fixture.recommit();
        assert_eq!(fixture.verify(None).commit.as_deref(), Some(&*fixture.head));
        fs::write(fixture.root.join("crates/cli/#held.rs"), b"held\n").expect("held file");
        let verification = fixture.verify(None);
        assert!(verification.commit.is_none());
        assert_eq!(
            verification.reason,
            "an untracked file could affect compilation: crates/cli/#held.rs"
        );
    }

    /// Every object kind Git stores reads back, loose and packed, under its own
    /// name; a loose object of any other kind refuses.
    #[test]
    fn every_object_kind_reads_back_loose_and_packed() {
        let loose = Scratch::store("loose-kinds");
        for kind in ["commit", "tree", "blob", "tag"] {
            let data = format!("a {kind} body\n").into_bytes();
            let oid = write_object(&loose.0, kind, &data);
            let object = loose
                .objects()
                .read_oid(&encode_oid(oid))
                .unwrap_or_else(|| panic!("loose {kind}"));
            assert_eq!((object.kind, object.data), (kind, data));
        }
        let note = write_object(&loose.0, "note", b"not a Git kind\n");
        assert!(loose.objects().read_oid(&encode_oid(note)).is_none());

        let packed = Scratch::store("packed-kinds");
        let entries = (1..=4)
            .map(|code| PackEntry::whole(code, format!("packed kind {code}\n").into_bytes()))
            .collect::<Vec<_>>();
        write_pack(
            &packed.0.join(".git/objects"),
            "kinds",
            &entries,
            Offsets::Small,
        );
        for entry in &entries {
            let object = packed
                .objects()
                .read_oid(&encode_oid(entry.oid))
                .unwrap_or_else(|| panic!("packed kind {}", entry.kind));
            assert_eq!(Some(object.kind), object_kind_code(entry.kind));
            assert_eq!(object.data, entry.payload);
        }
    }

    /// A pack index is read only with its own magic and version 2, and only
    /// when its trailer is the SHA-1 of every byte before it.
    #[test]
    fn a_pack_index_of_another_magic_version_or_checksum_is_not_read() {
        let scratch = Scratch::store("idx-header");
        let head = delta_chain(&scratch, 0, false);
        let path = scratch.0.join(".git/objects/pack/pack-chain.idx");
        let intact = fs::read(&path).expect("pack index");
        assert!(scratch.objects().read_oid(&encode_oid(head)).is_some());
        for (why, at, bytes) in [
            ("magic", 0, [0xff, b't', b'O', b'd']),
            ("version 1", 4, 1_u32.to_be_bytes()),
            ("version 3", 4, 3_u32.to_be_bytes()),
        ] {
            let mut altered = intact.clone();
            let body = altered.len() - OID_LEN;
            altered.truncate(body);
            altered[at..at + 4].copy_from_slice(&bytes);
            let trailer = Sha1::digest(&altered);
            altered.extend_from_slice(&trailer);
            fs::write(&path, altered).expect("altered pack index");
            assert!(
                scratch.objects().read_oid(&encode_oid(head)).is_none(),
                "{why}"
            );
        }
        let mut unsealed = intact;
        *unsealed.last_mut().expect("trailer") ^= 1;
        fs::write(&path, unsealed).expect("unsealed pack index");
        assert!(
            scratch.objects().read_oid(&encode_oid(head)).is_none(),
            "a trailer that is not the SHA-1 of the index"
        );
    }

    /// An index is read only whole and unconflicted: a trailer that is not the
    /// SHA-1 of what precedes it, an entry carrying a merge stage (0x1000,
    /// 0x2000 or both) or the extended flag (0x4000), and a name listed twice
    /// each refuse. Built the same way, two distinct names read back, so the
    /// repeated name is refused for being repeated.
    #[test]
    fn an_index_is_read_only_whole_and_unconflicted() {
        let scratch = Scratch::store("index-whole");
        let path = scratch.0.join(".git/index");
        let read = |bytes: Vec<u8>| {
            fs::write(&path, bytes).expect("index");
            read_index(&path).map(|index| index.entries)
        };
        let entry = Entry {
            mode: 0o100_644,
            oid: [9; OID_LEN],
        };
        let entries = BTreeMap::from([("src/lib.rs".to_owned(), entry.clone())]);
        assert_eq!(
            read(index_bytes(&entries, saturated, *b"DIRC", 2, &[])).as_ref(),
            Some(&entries)
        );
        let mut unsealed = index_bytes(&entries, saturated, *b"DIRC", 2, &[]);
        *unsealed.last_mut().expect("trailer") ^= 1;
        assert_eq!(read(unsealed), None, "a trailer that is not the SHA-1");
        for (flag, version) in [(0x1000, 2), (0x2000, 2), (0x3000, 2), (0x4000, 3)] {
            assert_eq!(
                read(index_bytes(
                    &entries,
                    |name| saturated(name) | flag,
                    *b"DIRC",
                    version,
                    &[]
                )),
                None,
                "flags {flag:#06x}"
            );
        }
        // Two entries under one header: two names, then one name twice.
        let listed = |names: [&str; 2]| {
            let mut bytes = b"DIRC".to_vec();
            bytes.extend_from_slice(&2_u32.to_be_bytes());
            bytes.extend_from_slice(&2_u32.to_be_bytes());
            for name in names {
                let one = index_bytes(
                    &BTreeMap::from([(name.to_owned(), entry.clone())]),
                    saturated,
                    *b"DIRC",
                    2,
                    &[],
                );
                bytes.extend_from_slice(&one[INDEX_HEADER..one.len() - OID_LEN]);
            }
            let trailer = Sha1::digest(&bytes);
            bytes.extend_from_slice(&trailer);
            bytes
        };
        assert_eq!(
            read(listed(["a.rs", "b.rs"])).map(|entries| entries.len()),
            Some(2)
        );
        assert_eq!(read(listed(["a.rs", "a.rs"])), None, "a name listed twice");
    }

    /// The tree a commit names must be a tree object. HEAD's commit is made to
    /// name a BLOB holding exactly its root tree's bytes, which parse to
    /// exactly HEAD's entries, and the tree cannot be proved.
    #[test]
    fn a_head_tree_that_is_not_a_tree_object_refuses() {
        let mut fixture = Fixture::new();
        let tree = read_loose(&loose_path(&fixture.root, fixture.tree)).1;
        let blob = write_object(&fixture.root, "blob", &tree);
        let commit = commit_text(blob);
        fixture.head = encode_oid(write_object(&fixture.root, "commit", commit.as_bytes()));
        fs::write(
            fixture.root.join(".git/refs/heads/main"),
            format!("{}\n", fixture.head),
        )
        .expect("head ref");
        let verification = fixture.verify(None);
        assert!(verification.commit.is_none());
        assert_eq!(verification.reason, "the HEAD tree cannot be proved");
    }

    /// A tracked file is proved only as a regular file. A `100755` entry is
    /// replaced on disk by a symbolic link to a file outside the repository
    /// with the same bytes; the link's own mode carries executable bits, so
    /// only the file-type check can refuse it, and it does.
    #[cfg(unix)]
    #[test]
    fn a_tracked_file_replaced_by_a_link_to_the_same_bytes_refuses() {
        use std::os::unix::fs::PermissionsExt as _;
        let mut fixture = Fixture::new();
        let script = fixture.root.join("crates/cli/run.sh");
        fs::write(&script, b"#!/bin/sh\n").expect("script");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("chmod +x");
        let oid = write_object(&fixture.root, "blob", b"#!/bin/sh\n");
        fixture.entries.insert(
            "crates/cli/run.sh".to_owned(),
            Entry {
                mode: 0o100_755,
                oid,
            },
        );
        fixture.recommit();
        assert_eq!(fixture.verify(None).commit.as_deref(), Some(&*fixture.head));

        let outside = Scratch::new("link-target");
        let target = outside.0.join("run.sh");
        fs::write(&target, b"#!/bin/sh\n").expect("target");
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).expect("chmod +x");
        fs::remove_file(&script).expect("remove script");
        std::os::unix::fs::symlink(&target, &script).expect("link");
        assert!(executable(
            &fs::symlink_metadata(&script).expect("link metadata")
        ));
        assert_eq!(fs::read(&script).expect("through the link"), b"#!/bin/sh\n");
        let verification = fixture.verify(None);
        assert!(verification.commit.is_none());
        assert_eq!(
            verification.reason,
            "the working tree does not equal the index"
        );
    }

    /// A tracked symbolic link is proved by its target, and retargeting it on
    /// disk alone refuses: the link's content is its target, not the file it
    /// points at.
    #[cfg(unix)]
    #[test]
    fn a_tracked_symlink_is_proved_by_its_target() {
        let mut fixture = Fixture::new();
        fixture.track_symlink("crates/cli/src/alias.rs", "lib.rs");
        let verification = fixture.verify(None);
        assert_eq!(
            verification.commit.as_deref(),
            Some(&*fixture.head),
            "{}",
            verification.reason
        );
        let link = fixture.root.join("crates/cli/src/alias.rs");
        fs::remove_file(&link).expect("remove link");
        std::os::unix::fs::symlink("elsewhere.rs", &link).expect("retarget link");
        let verification = fixture.verify(None);
        assert!(verification.commit.is_none());
        assert_eq!(
            verification.reason,
            "the working tree does not equal the index"
        );
    }

    /// A linked worktree reads HEAD and its index from its own git directory
    /// and every ref from the shared one, so the packed refs it watches are
    /// the SHARED file: the private directory never holds one.
    #[test]
    fn a_linked_worktree_watches_the_shared_packed_refs() {
        let fixture = Fixture::new();
        let linked = Scratch::new("linked");
        let private = fixture.root.join(".git/worktrees/linked");
        fs::create_dir_all(&private).expect("private git directory");
        fs::write(private.join("HEAD"), "ref: refs/heads/main\n").expect("private HEAD");
        fs::write(private.join("commondir"), "../..\n").expect("commondir");
        fs::copy(fixture.root.join(".git/index"), private.join("index")).expect("index");
        fs::write(
            linked.0.join(".git"),
            format!("gitdir: {}\n", private.display()),
        )
        .expect("gitdir file");
        for path in fixture.entries.keys() {
            let to = linked.0.join(path);
            fs::create_dir_all(to.parent().expect("parent")).expect("linked parent");
            fs::copy(fixture.root.join(path), to).expect("linked file");
        }
        let verification = verify(&linked.0.join("crates/cli"), None);
        assert_eq!(
            verification.commit.as_deref(),
            Some(&*fixture.head),
            "{}",
            verification.reason
        );
        let common = fixture.root.join(".git").canonicalize().expect("common");
        let private = private.canonicalize().expect("private");
        assert_eq!(
            verification.watched_files[..3],
            [
                private.join("HEAD"),
                private.join("index"),
                common.join("packed-refs")
            ]
        );
        assert!(
            !verification
                .watched_files
                .contains(&private.join("packed-refs"))
        );
    }

    /// # This is the one test that cannot run outside a checkout, and it made
    /// gate 18 unrunnable on this crate
    ///
    /// It read `repository(manifest).expect("test runs in a checkout")`, and
    /// the expectation is true of every place a person runs `cargo test` — and
    /// false of the one place CI runs `cargo mutants`. **`cargo-mutants` copies
    /// the source tree WITHOUT `.git`**, so `repository` returns `None`, this
    /// panics, and the run ends with *"cargo test failed in an unmutated tree,
    /// so no mutants were tested"* before a single mutation is applied.
    ///
    /// Gate 18 invokes `cargo mutants --in-diff` with no `--in-place`, so **any
    /// diff touching `crates/cli` failed at baseline**. The gate refuses that
    /// honestly — *"a timeout or an unviable mutant is not a pass, and this step
    /// will not call it one"* — which means it was not silently absent; it was
    /// loudly impossible, which is worse for a different reason: the mutation
    /// evidence for the largest crate in the workspace could never be collected.
    ///
    /// # Why a skip and not a panic, and why that is not a test asserting
    /// nothing
    ///
    /// Without `.git` the property is UNMEASURABLE, not false. §4 bans a test
    /// that asserts nothing; it does not require asserting where there is no
    /// evidence, and §3 rule 6 asks for the opposite. So the skip is LOUD — it
    /// names the reason on stderr — and the assertions below are untouched
    /// wherever a checkout exists, which is every developer machine and every
    /// CI job that runs `actions/checkout`.
    #[test]
    fn the_checkout_head_tree_object_is_decodable() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Some(repo) = repository(manifest) else {
            eprintln!(
                "SKIPPED the_checkout_head_tree_object_is_decodable: no `.git` above {}. \
                 This is the copied tree `cargo-mutants` builds, where the property is \
                 unmeasurable rather than false. Every assertion below still runs in a \
                 checkout.",
                manifest.display()
            );
            return;
        };
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
