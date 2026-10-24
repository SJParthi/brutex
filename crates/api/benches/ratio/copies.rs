//! Gate 8 for `/disk/copies.json` (CC-5, FM-607): the copies register,
//! served from memory, costs the same however many files its copy roots
//! hold.
//!
//! Each served store is given one copy root of 100, 1,000 and 10,000 invented
//! files (1x, 10x, 100x), a hundred to a folder, named through
//! `disk.extra_copy_roots`, saved through `store::settings::save` itself. The
//! first request walks it (printed as `cold`, the walk's own time, which is
//! O(files) and named in `docs/06-limits.md` L-CC5-1); every timed request
//! after it finds the root's own stat unchanged and is served the held
//! register: one settings `stat`, one `lstat` per configured root and the
//! body rendered once per walk. The needle is the root's file count, so a
//! request answered before the walk of THIS store's root is never timed.
//! THE PROOF OF THE O(roots) CLAIM IS THIS GROWTH BENCH,
//! `crates/api/benches/ratio.rs` (gate 8, `cargo bench -p api --bench
//! ratio`), whose served pass runs `copies_are_flat` and fails when a p50
//! grows past the ratio ceiling from 1x to 100x.

use store::settings::{self, Change, Key, Reader};

use super::refuse;
use super::served::{Served, flat, ok_with};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &["/disk/copies.json"];

/// Files per folder of the copy root.
const PER_FOLDER: usize = 100;

/// Makes `served`'s copy root of `served.census / 10` files and names it in
/// the store's settings. Returns the file count.
fn copy_root_into(served: &Served) -> usize {
    let files = served.census / 10;
    let root = served.root.join("bench-extra-copies");
    for n in 0..files {
        let folder = root.join(format!("f{:05}", n / PER_FOLDER));
        if let Err(why) = std::fs::create_dir_all(&folder)
            .and_then(|()| std::fs::write(folder.join(format!("{n}.bin")), b"invented copy"))
        {
            refuse(&format!("the bench copy root {}: {why}", root.display()));
        }
    }
    let mut reader = Reader::new(&served.root);
    if let Err(fault) = reader.refresh() {
        refuse(&format!(
            "the settings of {}: {}",
            served.root.display(),
            fault.why
        ));
    }
    let change = Change {
        expected: reader.highest(),
        set: vec![(Key::ExtraCopyRoots, root.display().to_string())],
        unset: Vec::new(),
        changed_by: "bench".to_owned(),
        reason: "copies register ratio bench".to_owned(),
        at_micros: 1_700_000_000_000_000,
    };
    if let Err(why) = settings::save(&mut reader, &change, |_| Ok(())) {
        refuse(&format!("disk.extra_copy_roots would not save: {why}"));
    }
    files
}

/// The register at 1x, 10x and 100x files, its p50 ratio gated. Run by
/// `crates/api/benches/ratio.rs`'s `served_routes_are_flat` AFTER the
/// settings rows, whose needle counts the generations it saved itself.
pub(crate) fn copies_are_flat(stores: &mut [Served]) -> bool {
    flat("copies", stores, |served| {
        let files = copy_root_into(served);
        vec![ok_with(
            "the register, served from memory",
            "/disk/copies.json".to_owned(),
            &format!("\"files\":{files},"),
        )]
    })
}
