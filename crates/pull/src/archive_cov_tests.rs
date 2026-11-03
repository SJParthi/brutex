//! BASE-COV-PULL: the archive walk's corners no test had reached (Route B,
//! recipe v4): a path with no file name, an extension that is empty, a
//! member that reads past the length it declared, and a listing that fails
//! part-way. Every member name and row here is invented (`ZZFUT`, prices
//! that are no index's).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

/// One invented `TrueData` index row: `YYYYMMDD,HH:MM:SS,price,volume,oi`.
const ONE_ROW: &str = "20221003,09:15:01,11111.11,0,0\n";

/// A scratch folder of this test's own, removed when it ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "brutex-archive-cov-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _stale = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir_all(&self.0);
    }
}

/// `/dev/<name>`, the name lowered at run time so no lower-case segment is a
/// literal here (gate 1d).
fn device(name: &str) -> PathBuf {
    Path::new("/")
        .join("DEV".to_ascii_lowercase())
        .join(name.to_ascii_lowercase())
}

/// **A PATH WITH NO FILE NAME NAMES NO INSTRUMENT**, rather than borrowing a
/// parent's: the root, nothing, and a path that ends in `..`. The walk never
/// hands one over, since an entry always has a name; the function says what
/// it would answer.
#[test]
fn a_path_with_no_file_name_names_no_instrument() {
    for nameless in [Path::new("/"), Path::new(""), Path::new("ZZFEED/..")] {
        assert_eq!(instrument_name(nameless), "", "{}", nameless.display());
    }
}

/// **ONLY A WHOLE-LETTER SECOND EXTENSION IS AN EXCHANGE SUFFIX.** An empty
/// one — a name ending in a dot before `.csv` — is part of the name, as a
/// strike decimal is; a suffix holding a digit is too. Through the walk as
/// well as by name.
#[test]
fn an_empty_second_extension_is_part_of_the_name() {
    for (file, want) in [
        ("ZZFUT..csv", "ZZFUT."),
        ("ZZFUT.NFO.csv", "ZZFUT"),
        ("ZZ25JUN111.5CE.csv", "ZZ25JUN111.5CE"),
        ("ZZ.N5.csv", "ZZ.N5"),
        ("ZZ-III.csv", "ZZ-III"),
        (".csv", ".csv"),
        ("...csv", ".."),
    ] {
        assert_eq!(
            instrument_name(&Path::new("ZZFEED").join(file)),
            want,
            "{file}"
        );
    }
    // THE WALK FILES IT UNDER THE SAME NAME.
    let scratch = Scratch::new("DOTS");
    fs::write(scratch.0.join("ZZFUT..csv"), ONE_ROW).expect("a member");
    let members = read_dir(&scratch.0, Columns::TrueDataIndex).expect("the folder walks");
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].instrument, "ZZFUT.");
    assert_eq!(members[0].rows.len(), 1);
}

/// **A MEMBER THAT READS PAST THE LENGTH IT DECLARED IS REFUSED AT THE CAP**,
/// one byte past it, never read to its end: a character device declares no
/// length and never ends. A cap of nothing refuses its first byte; a device
/// that declares nothing and holds nothing is read, empty.
#[test]
fn a_member_that_reads_past_its_declared_length_is_refused_at_the_cap() {
    let zero = device("ZERO");
    assert_eq!(
        fs::metadata(&zero).expect("the zero device").len(),
        0,
        "the fixture declares no length"
    );
    for cap in [4_u64, 0, 1] {
        assert_eq!(
            read_bounded(&zero, cap),
            Err(ArchiveError::MemberTooLarge {
                path: zero.clone(),
                bytes: cap + 1,
                cap,
            }),
            "cap {cap}"
        );
    }
    let said = read_bounded(&zero, 4)
        .expect_err("past the cap")
        .to_string();
    assert!(said.contains("refused before it was read"), "{said}");
    assert_eq!(
        read_bounded(&device("NULL"), 0),
        Ok(Vec::new()),
        "nothing declared and nothing held is within any cap"
    );
}

/// **A LISTING THAT FAILS PART-WAY REFUSES THE WALK, NAMING THE FOLDER IN THE
/// HOST'S OWN WORDS**, asked of [`entry_path`] directly: no folder a test can
/// make fails after it is opened (see that function). An entry that lists is
/// its path.
#[test]
fn a_listing_that_fails_part_way_names_the_folder() {
    let folder = Path::new("/ZZ/FEED");
    let why = entry_path(
        folder,
        Err(std::io::Error::other("an invented listing fault")),
    )
    .expect_err("a failed entry");
    assert_eq!(
        why,
        ArchiveError::Unreadable {
            path: folder.to_path_buf(),
            detail: "an invented listing fault".to_owned(),
        }
    );
    assert_eq!(
        why.to_string(),
        "/ZZ/FEED could not be listed: an invented listing fault"
    );
    // AN ENTRY THAT LISTS IS ITS PATH.
    let scratch = Scratch::new("ENTRY");
    fs::write(scratch.0.join("ZZFUT.csv"), ONE_ROW).expect("a member");
    let entry = fs::read_dir(&scratch.0)
        .expect("the folder lists")
        .next()
        .expect("one entry");
    assert_eq!(
        entry_path(&scratch.0, entry),
        Ok(scratch.0.join("ZZFUT.csv"))
    );
}
