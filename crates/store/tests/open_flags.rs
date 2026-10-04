//! D-0980: the no-follow open flag is per-architecture, and exactly one module
//! states it.
//!
//! Bit 17 (`1 << 17`) is `O_NOFOLLOW` on `x86_64` Linux only. On `aarch64`
//! Linux the same bit is `O_LARGEFILE` and `O_NOFOLLOW` is `0x8000`, so a
//! hard-coded bit 17 silently follows a final symlink there. These tests pin the
//! value per target, prove the flag refuses a symlink on the host that runs
//! them, and refuse the literal anywhere in `crates/` outside
//! `store::open_flags`.

// A test that cannot fail loudly is a test that asserts nothing; the harness
// panics on a broken invariant instead of threading `Result` through it.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use std::fs;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};

use store::open_flags::{O_NOFOLLOW, O_NOFOLLOW_NONBLOCK, O_NONBLOCK};

/// The x86_64-only no-follow bit, built so this file does not spell it.
const X86_BIT: u64 = 1 << 17;

#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    target_arch = "x86_64"
))]
#[test]
fn x86_64_linux_values_are_the_uapi_ones() {
    assert_eq!(u64::try_from(O_NOFOLLOW), Ok(X86_BIT));
    assert_eq!(O_NONBLOCK, 0x0800);
}

#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    target_arch = "aarch64"
))]
#[test]
fn aarch64_linux_nofollow_is_not_the_x86_bit() {
    // arch/arm64/include/uapi/asm/fcntl.h: O_NOFOLLOW 0100000 (octal).
    assert_eq!(O_NOFOLLOW, 0x8000);
    assert_eq!(O_NONBLOCK, 0x0800);
}

#[cfg(target_os = "macos")]
#[test]
fn macos_values_are_the_sdk_ones() {
    assert_eq!(O_NOFOLLOW, 0x0100);
    assert_eq!(O_NONBLOCK, 0x0004);
}

/// The combined literal is exactly the union of the two per-target flags, and
/// the two share no bit -- which is why it is a literal: over disjoint bits `|`
/// and `^` agree, so an `O_NOFOLLOW | O_NONBLOCK` expression carried an
/// equivalent mutant no test could kill (D-0192).
#[test]
fn the_combined_flag_is_the_disjoint_union_of_the_two() {
    assert_eq!(O_NOFOLLOW & O_NONBLOCK, 0, "the two flags share a bit");
    assert_eq!(O_NOFOLLOW_NONBLOCK, O_NOFOLLOW | O_NONBLOCK);
    assert_eq!(O_NOFOLLOW_NONBLOCK & !(O_NOFOLLOW | O_NONBLOCK), 0);
}

#[cfg(any(target_os = "linux", target_os = "android"))]
const ELOOP: i32 = 40;
#[cfg(target_os = "macos")]
const ELOOP: i32 = 62;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brutex-open-flags-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

#[test]
fn the_flag_refuses_a_final_symlink_that_a_plain_open_follows() {
    let dir = scratch("symlink");
    let target = dir.join("target");
    let link = dir.join("link");
    fs::write(&target, b"evidence").expect("write target");
    std::os::unix::fs::symlink(&target, &link).expect("symlink");

    // Without the flag the link is followed: the control proves the refusal
    // below comes from the flag and not from the fixture.
    let followed = fs::read(&link).expect("plain open follows");
    assert_eq!(followed, b"evidence");

    let refused = fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK)
        .open(&link)
        .expect_err("no-follow open must refuse a final symlink");
    assert_eq!(refused.raw_os_error(), Some(ELOOP), "{refused}");

    // The flag does not refuse an ordinary regular file.
    let plain = fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK)
        .open(&target)
        .expect("regular file opens with the flag");
    assert!(plain.metadata().expect("metadata").is_file());
    let _ = fs::remove_dir_all(&dir);
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read crates dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// True when `line` carries a hex literal whose value is the x86_64-only
/// no-follow bit, however it is spelled (separators, leading zeros, case,
/// integer suffix).
fn names_the_x86_bit(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut at = 0;
    while let Some(found) = lower[at..].find("0x") {
        let start = at + found;
        let preceded =
            start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
        let digits: String = lower[start + 2..]
            .chars()
            .take_while(|c| c.is_ascii_hexdigit() || *c == '_')
            .filter(|c| *c != '_')
            .collect();
        if !preceded && u64::from_str_radix(&digits, 16) == Ok(X86_BIT) {
            return true;
        }
        at = start + 2;
    }
    false
}

#[test]
fn the_literal_detector_matches_every_spelling_and_nothing_wider() {
    assert!(names_the_x86_bit(concat!(
        "custom_flags(0x2",
        "0000 | 0x800)"
    )));
    assert!(names_the_x86_bit(concat!("const F: i32 = 0x2", "0_000;")));
    assert!(names_the_x86_bit(concat!("0X2", "_0000")));
    assert!(names_the_x86_bit(concat!("= 0x000", "2_0000;")));
    assert!(names_the_x86_bit(concat!("(0x2", "0000i32)")));
    assert!(!names_the_x86_bit(concat!("0x2", "00000")));
    assert!(!names_the_x86_bit(concat!("0x2", "0000f")));
    assert!(!names_the_x86_bit("0x8000 | 0x800"));
    assert!(!names_the_x86_bit(concat!("a0x2", "0000")));
    assert!(!names_the_x86_bit(""));
}

#[test]
fn no_crate_hard_codes_the_x86_64_no_follow_bit_outside_open_flags() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crates = manifest.parent().expect("crates dir");
    let owner = manifest.join("src").join("open_flags.rs");
    let mut sources = Vec::new();
    rust_sources(crates, &mut sources);
    assert!(sources.contains(&owner), "open_flags.rs must exist");
    let mut offenders = Vec::new();
    for path in sources.iter().filter(|path| **path != owner) {
        let text = fs::read_to_string(path).expect("read source");
        for (number, line) in text.lines().enumerate() {
            if names_the_x86_bit(line) {
                offenders.push(format!("{}:{}", path.display(), number + 1));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "hard-coded x86_64 O_NOFOLLOW outside store::open_flags: {offenders:#?}"
    );
}
