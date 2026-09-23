//! Frontend-only publication. No API startup, process control, network, store or
//! service configuration access. All old immutable chunks survive apply/rollback.
//! dry-run SOURCE_BUILD TARGET_BUILD PLAN.json; apply PLAN.json; rollback PLAN.json
//! dry-run writes only the new plan and its sibling rollback/staging directory.
//! Publication is atomic per file, not across the entire multi-file bundle.
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
type Result<T> = std::result::Result<T, String>;
const FILES: usize = 4096;
const FILE_BYTES: u64 = 16 * 1024 * 1024;
const TREE_BYTES: u64 = 256 * 1024 * 1024;
const PLAN_BYTES: u64 = 4 * 1024 * 1024;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn hash(bytes: &[u8]) -> String {
    brutex_core::blake3::hash(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: String,
    bytes: u64,
    hash: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema: u32,
    source: PathBuf,
    target: PathBuf,
    stage: PathBuf,
    source_files: Vec<Entry>,
    target_files: Vec<Entry>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    plan: Plan,
    digest: String,
}
struct Tree {
    files: BTreeMap<String, Vec<u8>>,
}
impl Tree {
    fn entries(&self) -> Vec<Entry> {
        self.files
            .iter()
            .map(|(path, data)| Entry {
                path: path.clone(),
                bytes: data.len() as u64,
                hash: hash(data),
            })
            .collect()
    }
}
fn canonical_dir(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() || !fs::symlink_metadata(path).map_err(err)?.is_dir() {
        return Err("root must be an existing absolute non-symlink directory".into());
    }
    let canonical = fs::canonicalize(path).map_err(err)?;
    if canonical != path {
        return Err("root must use its exact canonical path".into());
    }
    Ok(canonical)
}
fn safe(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && path.is_ascii()
        && path.split('/').all(|s| {
            !s.is_empty()
                && s != "."
                && s != ".."
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
fn immutable(path: &str) -> bool {
    path.starts_with("_app/immutable/")
}
#[cfg(unix)]
fn open_read(path: &Path) -> Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    #[cfg(target_os = "macos")]
    let flags = 0x100 | 0x4;
    #[cfg(target_os = "linux")]
    let flags = 0x20_000 | 0x800;
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Err("platform file flags not verified".into());
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)
        .map_err(err)?;
    if !file.metadata().map_err(err)?.is_file() {
        return Err("input is not a regular file".into());
    }
    file.try_lock_shared().map_err(err)?;
    Ok(file)
}
#[cfg(not(unix))]
fn open_read(_path: &Path) -> Result<File> {
    Err("platform file flags not verified".into())
}
fn read(path: &Path, max: u64) -> Result<Vec<u8>> {
    let before = fs::symlink_metadata(path).map_err(err)?;
    if !before.is_file() || before.len() > max {
        return Err(format!("not a regular bounded file: {}", path.display()));
    }
    let mut f = open_read(path)?;
    let mut data = Vec::new();
    Read::by_ref(&mut f)
        .take(max + 1)
        .read_to_end(&mut data)
        .map_err(err)?;
    let after = fs::symlink_metadata(path).map_err(err)?;
    if data.len() as u64 != before.len()
        || after.len() != before.len()
        || after.modified().map_err(err)? != before.modified().map_err(err)?
        || hash(&data) != hash_handle(&mut f, max)?
    {
        return Err("file changed while reading".into());
    }
    Ok(data)
}
fn hash_handle(file: &mut File, max: u64) -> Result<String> {
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(0)).map_err(err)?;
    let mut data = Vec::new();
    file.take(max + 1).read_to_end(&mut data).map_err(err)?;
    Ok(hash(&data))
}
fn scan(root: &Path) -> Result<Tree> {
    canonical_dir(root)?;
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut files = BTreeMap::new();
    let mut entries = 0usize;
    let mut total = 0u64;
    while let Some((dir, depth)) = pending.pop() {
        for entry in fs::read_dir(dir).map_err(err)? {
            entries += 1;
            if entries > FILES {
                return Err("tree entry ceiling exceeded".into());
            }
            let entry = entry.map_err(err)?;
            let path = entry.path();
            let ty = entry.file_type().map_err(err)?;
            if ty.is_dir() {
                if depth >= 8 {
                    return Err("tree depth ceiling exceeded".into());
                }
                pending.push((path, depth + 1));
                continue;
            }
            if !ty.is_file() {
                return Err("tree has a symlink or special file".into());
            }
            let name = path
                .strip_prefix(root)
                .map_err(err)?
                .to_str()
                .ok_or("non-UTF8 path")?
                .to_owned();
            if !safe(&name) {
                return Err("unsafe asset path".into());
            }
            let data = read(&path, FILE_BYTES)?;
            total = total
                .checked_add(data.len() as u64)
                .ok_or("tree bytes overflow")?;
            if total > TREE_BYTES {
                return Err("tree byte ceiling exceeded".into());
            }
            files.insert(name, data);
        }
    }
    Ok(Tree { files })
}
fn sync(dir: &Path) -> Result<()> {
    File::open(dir).map_err(err)?.sync_all().map_err(err)
}
fn parents(root: &Path, relative: &str) -> Result<()> {
    let mut at = root.to_path_buf();
    for part in Path::new(relative)
        .parent()
        .ok_or("missing asset parent")?
        .components()
    {
        let Component::Normal(part) = part else {
            return Err("unsafe parent".into());
        };
        at.push(part);
        match fs::symlink_metadata(&at) {
            Ok(m) if m.is_dir() => {}
            Ok(_) => return Err("publication parent is not a directory".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&at).map_err(err)?;
                sync(at.parent().ok_or("missing parent")?)?;
            }
            Err(e) => return Err(err(e)),
        }
    }
    Ok(())
}
fn create(path: &Path, data: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(err)?;
    f.write_all(data).map_err(err)?;
    f.sync_all().map_err(err)?;
    sync(path.parent().ok_or("missing parent")?)
}
fn same_device(a: &Path, b: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if fs::metadata(a).map_err(err)?.dev() != fs::metadata(b).map_err(err)?.dev() {
            return Err("stage and target must be on the same filesystem for atomic rename".into());
        }
    }
    Ok(())
}
fn validate(entries: &[Entry]) -> Result<()> {
    if entries.len() > FILES {
        return Err("manifest entry ceiling exceeded".into());
    }
    let mut previous = None;
    let mut total = 0u64;
    for e in entries {
        if !safe(&e.path)
            || previous.is_some_and(|p: &str| p >= e.path.as_str())
            || e.hash.len() != 64
            || !e
                .hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || e.bytes > FILE_BYTES
        {
            return Err("invalid, duplicate or unordered manifest entry".into());
        }
        previous = Some(e.path.as_str());
        total = total.checked_add(e.bytes).ok_or("manifest byte overflow")?;
    }
    if total > TREE_BYTES {
        return Err("manifest tree bytes exceeded".into());
    }
    Ok(())
}
fn require_entries(tree: &Tree, expected: &[Entry]) -> Result<()> {
    if tree.entries() != expected {
        return Err("complete file census or bytes changed".into());
    }
    Ok(())
}
fn stage_plan(source: &Path, target: &Path, path: &Path) -> Result<String> {
    let source = canonical_dir(source)?;
    let target = canonical_dir(target)?;
    if source.starts_with(&target) || target.starts_with(&source) {
        return Err("source and target must be disjoint".into());
    }
    let parent = canonical_dir(path.parent().ok_or("plan must have parent")?)?;
    if !path.is_absolute() || parent.starts_with(&source) || parent.starts_with(&target) {
        return Err("plan must be outside both source and target".into());
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| safe(s))
        .ok_or("invalid plan filename")?;
    let stage = parent.join(format!("{name}.artifacts"));
    same_device(&parent, &target)?;
    let new = scan(&source)?;
    let old = scan(&target)?;
    for required in ["index.html", "_app/version.json"] {
        if !new.files.contains_key(required) || !old.files.contains_key(required) {
            return Err(format!("required entry missing: {required}"));
        }
    }
    for (name, bytes) in &new.files {
        if immutable(name) && old.files.get(name).is_some_and(|prior| prior != bytes) {
            return Err("immutable path collision has different bytes".into());
        }
    }
    fs::create_dir(&stage).map_err(err)?;
    sync(&parent)?;
    let backup = stage.join("backup");
    fs::create_dir(&backup).map_err(err)?;
    for (name, data) in &old.files {
        if !immutable(name) {
            parents(&backup, name)?;
            create(&backup.join(name), data)?;
        }
    }
    let plan = Plan {
        schema: 1,
        source,
        target,
        stage,
        source_files: new.entries(),
        target_files: old.entries(),
    };
    require_entries(&scan(&plan.source)?, &plan.source_files)?;
    require_entries(&scan(&plan.target)?, &plan.target_files)?;
    let digest = hash(&serde_json::to_vec(&plan).map_err(err)?);
    let sealed = Sealed {
        plan,
        digest: digest.clone(),
    };
    create(path, &serde_json::to_vec_pretty(&sealed).map_err(err)?)?;
    Ok(digest)
}
fn load(path: &Path) -> Result<Sealed> {
    let sealed: Sealed = serde_json::from_slice(&read(path, PLAN_BYTES)?).map_err(err)?;
    if sealed.plan.schema != 1
        || sealed.digest != hash(&serde_json::to_vec(&sealed.plan).map_err(err)?)
    {
        return Err("manifest identity differs".into());
    }
    validate(&sealed.plan.source_files)?;
    validate(&sealed.plan.target_files)?;
    canonical_dir(&sealed.plan.target)?;
    canonical_dir(&sealed.plan.stage)?;
    let p = &sealed.plan;
    if p.source.starts_with(&p.target)
        || p.target.starts_with(&p.source)
        || p.stage.starts_with(&p.source)
        || p.stage.starts_with(&p.target)
    {
        return Err("manifest roots overlap".into());
    }
    same_device(&p.stage, &p.target)?;
    Ok(sealed)
}
fn backup(plan: &Plan) -> Result<Tree> {
    let tree = scan(&plan.stage.join("backup"))?;
    require_entries(
        &tree,
        &plan
            .target_files
            .iter()
            .filter(|e| !immutable(&e.path))
            .cloned()
            .collect::<Vec<_>>(),
    )?;
    Ok(tree)
}
fn marker(sealed: &Sealed, name: &str) -> Result<bool> {
    let path = sealed.plan.stage.join(name);
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(err(e)),
        Ok(_) => {
            if read(&path, 128)? != sealed.digest.as_bytes() {
                return Err("publication marker identity differs".into());
            }
            Ok(true)
        }
    }
}
fn record(sealed: &Sealed, name: &str) -> Result<()> {
    if !marker(sealed, name)? {
        create(&sealed.plan.stage.join(name), sealed.digest.as_bytes())?;
    }
    Ok(())
}
fn owner(plan: &Plan) -> Result<File> {
    let path = plan.stage.join("owner.lock");
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(f) => {
            f.sync_all().map_err(err)?;
            sync(&plan.stage)?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(err(e)),
    }
    let f = open_read(&path)?;
    f.unlock().map_err(err)?;
    f.try_lock().map_err(err)?;
    if f.metadata().map_err(err)?.len() != 0 {
        return Err("publication owner is not empty".into());
    }
    Ok(f)
}
fn expected(plan: &Plan, rolled_back: bool) -> Vec<Entry> {
    let mut map: BTreeMap<String, Entry> = plan
        .target_files
        .iter()
        .map(|e| (e.path.clone(), e.clone()))
        .collect();
    for e in &plan.source_files {
        if !rolled_back || immutable(&e.path) {
            map.insert(e.path.clone(), e.clone());
        }
    }
    map.into_values().collect()
}
fn admit_intermediate(plan: &Plan, current: &Tree) -> Result<()> {
    let old: BTreeMap<_, _> = plan.target_files.iter().map(|e| (&e.path, e)).collect();
    let new: BTreeMap<_, _> = plan.source_files.iter().map(|e| (&e.path, e)).collect();
    for e in current.entries() {
        if old.get(&e.path).copied() != Some(&e) && new.get(&e.path).copied() != Some(&e) {
            return Err(format!("unexpected target bytes/path: {}", e.path));
        }
    }
    for e in &plan.target_files {
        if !current.files.contains_key(&e.path) {
            return Err(format!("original target file missing: {}", e.path));
        }
    }
    Ok(())
}
fn publish(plan: &Plan, name: &str, data: &[u8], additive: bool) -> Result<()> {
    parents(&plan.target, name)?;
    let destination = plan.target.join(name);
    if let Ok(meta) = fs::symlink_metadata(&destination) {
        if !meta.is_file() {
            return Err("target asset is not regular".into());
        }
        let held = read(&destination, FILE_BYTES)?;
        if held == data {
            return Ok(());
        }
        if additive {
            return Err("immutable collision during publication".into());
        }
        let digest = hash(&held);
        if !plan
            .target_files
            .iter()
            .chain(&plan.source_files)
            .any(|e| e.path == name && e.bytes == held.len() as u64 && e.hash == digest)
        {
            return Err("target changed before atomic replacement".into());
        }
    }
    let tmp = plan
        .stage
        .join(format!("pending-{}", hash(name.as_bytes())));
    if tmp.try_exists().map_err(err)? {
        if read(&tmp, FILE_BYTES)? != data {
            return Err("pending publication bytes differ".into());
        }
    } else {
        create(&tmp, data)?;
    }
    if additive {
        fs::hard_link(&tmp, &destination).map_err(err)?;
        fs::remove_file(&tmp).map_err(err)?;
    } else {
        fs::rename(&tmp, &destination).map_err(err)?;
    }
    sync(destination.parent().ok_or("destination parent missing")?)?;
    sync(&plan.stage)
}
fn order(name: &str) -> u8 {
    if immutable(name) {
        0
    } else if name == "index.html" {
        2
    } else if name == "_app/version.json" {
        3
    } else {
        1
    }
}
fn apply_with(path: &Path, after_chunks: impl FnOnce()) -> Result<()> {
    let sealed = load(path)?;
    let p = &sealed.plan;
    let _owner = owner(p)?;
    if marker(&sealed, "rollback-started")? {
        return Err("this plan has begun rollback; stage a fresh plan".into());
    }
    let source = scan(&p.source)?;
    require_entries(&source, &p.source_files)?;
    let _backup = backup(p)?;
    let current = scan(&p.target)?;
    if marker(&sealed, "applied")? {
        require_plan(path, &sealed)?;
        return require_entries(&current, &expected(p, false));
    }
    if marker(&sealed, "apply-started")? {
        admit_intermediate(p, &current)?;
    } else {
        require_entries(&current, &p.target_files)?;
    }
    record(&sealed, "apply-started")?;
    let mut rows: Vec<_> = source.files.iter().collect();
    rows.sort_by_key(|(n, _)| (order(n), *n));
    for (name, data) in &rows {
        if immutable(name) {
            publish(p, name, data, true)?;
        }
    }
    after_chunks();
    require_plan(path, &sealed)?;
    require_entries(&scan(&p.source)?, &p.source_files)?;
    backup(p)?;
    admit_intermediate(p, &scan(&p.target)?)?;
    for (name, data) in rows {
        if !immutable(name) {
            publish(p, name, data, false)?;
        }
    }
    require_entries(&scan(&p.source)?, &p.source_files)?;
    require_entries(&scan(&p.target)?, &expected(p, false))?;
    backup(p)?;
    record(&sealed, "applied")?;
    require_entries(&scan(&p.target)?, &expected(p, false))?;
    require_entries(&scan(&p.source)?, &p.source_files)?;
    require_plan(path, &sealed)?;
    Ok(())
}
fn rollback(path: &Path) -> Result<()> {
    let sealed = load(path)?;
    let p = &sealed.plan;
    let _owner = owner(p)?;
    if !marker(&sealed, "apply-started")? {
        return Err("plan was never applied".into());
    }
    let old = backup(p)?;
    let current = scan(&p.target)?;
    admit_intermediate(p, &current)?;
    record(&sealed, "rollback-started")?;
    let mut rows: Vec<_> = old.files.iter().collect();
    rows.sort_by_key(|(n, _)| (order(n), *n));
    for (name, data) in rows {
        publish(p, name, data, false)?;
    }
    let moved = p.stage.join("withdrawn");
    if !moved.try_exists().map_err(err)? {
        fs::create_dir(&moved).map_err(err)?;
    }
    for e in &p.source_files {
        if !immutable(&e.path)
            && !old.files.contains_key(&e.path)
            && p.target.join(&e.path).try_exists().map_err(err)?
        {
            if read(&p.target.join(&e.path), FILE_BYTES)?.len() as u64 != e.bytes
                || hash(&read(&p.target.join(&e.path), FILE_BYTES)?) != e.hash
            {
                return Err("new mutable file changed before rollback".into());
            }
            parents(&moved, &e.path)?;
            fs::rename(p.target.join(&e.path), moved.join(&e.path)).map_err(err)?;
            sync(p.target.join(&e.path).parent().ok_or("missing parent")?)?;
            sync(moved.join(&e.path).parent().ok_or("missing parent")?)?;
        }
    }
    // A partial apply need not have copied every new immutable chunk. All chunks
    // that did land are retained; every original file must be restored exactly.
    let actual = scan(&p.target)?;
    for e in &p.target_files {
        let data = actual.files.get(&e.path).ok_or("rollback file missing")?;
        if hash(data) != e.hash {
            return Err("rollback differs from original bytes".into());
        }
    }
    for e in actual.entries() {
        if !immutable(&e.path) && !p.target_files.contains(&e) {
            return Err("rollback has an unexpected mutable entry".into());
        }
    }
    backup(p)?;
    record(&sealed, "rolled-back")?;
    admit_intermediate(p, &scan(&p.target)?)?;
    require_plan(path, &sealed)?;
    Ok(())
}
fn require_plan(path: &Path, sealed: &Sealed) -> Result<()> {
    if load(path)?.digest != sealed.digest {
        return Err("publication manifest changed during operation".into());
    }
    Ok(())
}
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match args.as_slice() {
        [mode, source, target, plan] if mode == "dry-run" => {
            stage_plan(Path::new(source), Path::new(target), Path::new(plan)).map(|id| {
                format!("STAGED {id}; target unchanged; no backend release clearance")
            })
        }
        [mode, plan] if mode == "apply" => apply_with(Path::new(plan), || {}).map(|()| {
            "APPLIED frontend files only; backend process/store/configuration untouched".into()
        }),
        [mode, plan] if mode == "rollback" => rollback(Path::new(plan)).map(|()| {
            "ROLLED BACK mutable frontend files; immutable chunks retained".into()
        }),
        _ => Err("usage: frontend-publish dry-run SOURCE_BUILD TARGET_BUILD PLAN.json | apply PLAN.json | rollback PLAN.json".into()),
    };
    match result {
        Ok(message) => println!("{message}"),
        Err(why) => {
            eprintln!(
                "REFUSED: {why}; no success is claimed; preserve plan/stage for inspection or rollback"
            );
            std::process::exit(2);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture {
        root: PathBuf,
        source: PathBuf,
        target: PathBuf,
        plan: PathBuf,
    }
    impl Fixture {
        fn new() -> Result<Self> {
            let root = std::env::temp_dir().join(format!(
                "brutex-frontend-publish-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).map_err(err)?;
            let root = fs::canonicalize(root).map_err(err)?;
            let source = root.join("source");
            let target = root.join("target");
            fs::create_dir(&source).map_err(err)?;
            fs::create_dir(&target).map_err(err)?;
            for (base, prefix) in [(&source, "new"), (&target, "old")] {
                for (path, data) in [
                    ("index.html", format!("{prefix} shell")),
                    ("db.html", format!("{prefix} database")),
                    ("_app/version.json", format!("{{\"version\":\"{prefix}\"}}")),
                    (
                        &format!("_app/immutable/{prefix}.js"),
                        format!("{prefix} script"),
                    ),
                ] {
                    parents(base, path)?;
                    create(&base.join(path), data.as_bytes())?;
                }
            }
            create(&source.join("new-page.html"), b"new page")?;
            let plan = root.join("plan.json");
            Ok(Self {
                root,
                source,
                target,
                plan,
            })
        }
        fn stage(&self) -> Result<Sealed> {
            stage_plan(&self.source, &self.target, &self.plan)?;
            load(&self.plan)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    #[test]
    fn staging_apply_retry_and_rollback_preserve_both_chunk_generations() -> Result<()> {
        let f = Fixture::new()?;
        let before = scan(&f.target)?.entries();
        let plan = f.stage()?;
        assert_eq!(scan(&f.target)?.entries(), before);
        assert_eq!(
            backup(&plan.plan)?.entries(),
            before
                .iter()
                .filter(|e| !immutable(&e.path))
                .cloned()
                .collect::<Vec<_>>()
        );
        apply_with(&f.plan, || {})?;
        let applied = scan(&f.target)?.entries();
        apply_with(&f.plan, || panic!("completed retry must not republish"))?;
        assert_eq!(scan(&f.target)?.entries(), applied);
        assert_eq!(read(&f.target.join("index.html"), 100)?, b"new shell");
        fs::rename(&f.source, f.root.join("source-moved")).map_err(err)?;
        rollback(&f.plan)?;
        rollback(&f.plan)?;
        assert_eq!(read(&f.target.join("index.html"), 100)?, b"old shell");
        assert_eq!(
            read(&f.target.join("_app/immutable/old.js"), 100)?,
            b"old script"
        );
        assert_eq!(
            read(&f.target.join("_app/immutable/new.js"), 100)?,
            b"new script"
        );
        assert!(!f.target.join("new-page.html").exists());
        assert_eq!(
            read(&plan.plan.stage.join("withdrawn/new-page.html"), 100)?,
            b"new page"
        );
        assert!(marker(&plan, "applied")? && marker(&plan, "rolled-back")?);
        assert!(apply_with(&f.plan, || {}).is_err());
        Ok(())
    }
    #[test]
    fn late_source_loss_refuses_before_entrypoints_and_same_plan_can_resume() -> Result<()> {
        let f = Fixture::new()?;
        f.stage()?;
        let why = apply_with(&f.plan, || {
            fs::write(f.source.join("index.html"), b"changed").unwrap();
        })
        .unwrap_err();
        assert!(why.contains("census"));
        assert_eq!(read(&f.target.join("index.html"), 100)?, b"old shell");
        assert!(f.target.join("_app/immutable/new.js").is_file());
        fs::write(f.source.join("index.html"), b"new shell").map_err(err)?;
        apply_with(&f.plan, || {})?;
        assert_eq!(read(&f.target.join("index.html"), 100)?, b"new shell");
        Ok(())
    }
    #[test]
    fn concurrent_target_mutation_and_new_file_are_not_overwritten() -> Result<()> {
        for extra in [false, true] {
            let f = Fixture::new()?;
            f.stage()?;
            let path = f
                .target
                .join(if extra { "foreign.html" } else { "index.html" });
            assert!(
                apply_with(&f.plan, || {
                    fs::write(&path, b"foreign writer").unwrap();
                })
                .unwrap_err()
                .contains("unexpected target")
            );
            assert_eq!(read(&path, 100)?, b"foreign writer");
            assert!(!marker(&load(&f.plan)?, "applied")?);
        }
        Ok(())
    }
    #[test]
    fn changed_manifest_and_backup_block_publication() -> Result<()> {
        let f = Fixture::new()?;
        let sealed = f.stage()?;
        fs::write(sealed.plan.stage.join("backup/index.html"), b"corrupt").map_err(err)?;
        assert!(apply_with(&f.plan, || {}).unwrap_err().contains("census"));
        assert!(!f.target.join("_app/immutable/new.js").exists());
        let f = Fixture::new()?;
        f.stage()?;
        assert!(
            apply_with(&f.plan, || {
                fs::write(&f.plan, b"{}").unwrap();
            })
            .is_err()
        );
        assert_eq!(read(&f.target.join("index.html"), 100)?, b"old shell");
        Ok(())
    }
    #[test]
    fn busy_owner_and_foreign_completion_marker_never_look_complete() -> Result<()> {
        let f = Fixture::new()?;
        let sealed = f.stage()?;
        let held = owner(&sealed.plan)?;
        assert!(apply_with(&f.plan, || {}).is_err());
        drop(held);
        create(&sealed.plan.stage.join("applied"), b"wrong identity")?;
        assert!(
            apply_with(&f.plan, || {})
                .unwrap_err()
                .contains("marker identity")
        );
        assert_eq!(read(&f.target.join("index.html"), 100)?, b"old shell");
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_immutable_collision_and_manifest_duplicates_refuse() -> Result<()> {
        let f = Fixture::new()?;
        std::os::unix::fs::symlink(f.target.join("index.html"), f.source.join("link.html"))
            .map_err(err)?;
        assert!(
            stage_plan(&f.source, &f.target, &f.plan)
                .unwrap_err()
                .contains("symlink")
        );
        let f = Fixture::new()?;
        create(
            &f.source.join("_app/immutable/old.js"),
            b"different immutable",
        )?;
        assert!(
            stage_plan(&f.source, &f.target, &f.plan)
                .unwrap_err()
                .contains("immutable path collision")
        );
        let f = Fixture::new()?;
        let mut sealed = f.stage()?;
        sealed
            .plan
            .source_files
            .push(sealed.plan.source_files[0].clone());
        sealed.digest = hash(&serde_json::to_vec(&sealed.plan).map_err(err)?);
        fs::write(&f.plan, serde_json::to_vec(&sealed).map_err(err)?).map_err(err)?;
        assert!(
            load(&f.plan)
                .err()
                .ok_or("duplicates accepted")?
                .contains("duplicate")
        );
        Ok(())
    }
    #[test]
    fn partial_apply_can_rollback_without_finishing_new_chunks_or_mutable_files() -> Result<()> {
        let f = Fixture::new()?;
        let sealed = f.stage()?;
        record(&sealed, "apply-started")?;
        publish(&sealed.plan, "db.html", b"new database", false)?;
        rollback(&f.plan)?;
        assert_eq!(scan(&f.target)?.entries(), sealed.plan.target_files);
        assert!(marker(&sealed, "rolled-back")?);
        Ok(())
    }
}
