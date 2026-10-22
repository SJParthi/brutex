#![cfg(test)]
//! Tests of the `_v3` ledger reader. Every manifest, marker, size and CRC is
//! invented at run time; no vendor line is copied.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use core::fmt::Write as _;

use sha2::Digest as _;

use super::*;
use crate::gdfl_fixtures::{put, scratch};

/// The day of `yyyy-mm-dd`.
fn day(year: u16, month: u8, dom: u8) -> Day {
    Day::new(year, month, dom).unwrap()
}

/// A listing of `d` holding `entries` (name, size, CRC-32), each with no
/// locator, from a day zip of `zip_bytes` where one was read.
fn listing(d: Day, entries: &[(&str, u64, u32)], zip_bytes: Option<u64>) -> NfoDay<()> {
    let mut out = NfoDay::new(d, entries.len());
    for &(name, size, crc) in entries {
        out.push(name, size, crc, ());
    }
    match zip_bytes {
        Some(bytes) => out.with_member_bytes(bytes),
        None => out,
    }
}

/// The manifest the ledger's tool writes for `entries` of `d`: one line a
/// day-zip entry, mapped, with field 4 as measured (1 on a directory).
fn manifest_of(d: Day, entries: &[(&str, u64, u32)]) -> Vec<u8> {
    let zip_name = NfoZips::day_zip_name(d);
    let month = zip_name.split('/').next().unwrap();
    entries
        .iter()
        .fold(String::new(), |mut out, &(name, size, crc)| {
            let path = format!("{month}/{}", name.replace('\\', "/"));
            let dir = u8::from(path.ends_with('/'));
            let _ = writeln!(out, "{path}\t{size}\t{crc:08x}\t{dir}\t0\t{zip_name}");
            out
        })
        .into_bytes()
}

/// `lines`, each ended by a newline, as one text.
fn joined<'a>(lines: impl Iterator<Item = &'a str>) -> String {
    lines.fold(String::new(), |mut out, line| {
        let _ = writeln!(out, "{line}");
        out
    })
}

/// How a test hands `V3Ledger::from_env` its environment.
type Env = dyn Fn(&str) -> Option<OsString>;

/// The ok marker the tool writes for `manifest` of `d`'s zip of `zip_size`
/// bytes: its files, bytes and sha256 counted from the manifest itself.
fn marker_of(d: Day, manifest: &[u8], zip_size: u64) -> String {
    let zip_name = NfoZips::day_zip_name(d);
    let text = String::from_utf8_lossy(manifest);
    let (mut files, mut bytes) = (0_u64, 0_u64);
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if !fields[0].ends_with('/') {
            files += 1;
        }
        bytes += fields
            .get(1)
            .and_then(|size| size.parse::<u64>().ok())
            .unwrap_or(0);
    }
    format!(
        "x3-marker 1\nname_hex={}\ncrc=0badf00d\nsize={zip_size}\nmanifest_sha256={:x}\nday_hex={}\nfiles={files}\nbytes={bytes}\nrun=20990101-000000.1\n",
        hex(zip_name.as_bytes()),
        Sha256::digest(manifest),
        hex(zip_name.strip_suffix(".zip").unwrap().as_bytes()),
    )
}

/// A ledger under a new scratch root holding, for `d`, `manifest` and
/// `marker` where each is given.
fn ledger(tag: &str, d: Day, manifest: Option<&[u8]>, marker: Option<&[u8]>) -> V3Ledger {
    let root = scratch(tag);
    std::fs::create_dir_all(&root).unwrap();
    let ledger = V3Ledger::At { root: root.clone() };
    let rel = |path: PathBuf| {
        path.strip_prefix(&root)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    };
    if let Some(bytes) = manifest {
        put(&root, &rel(ledger.manifest_path(d).unwrap()), bytes);
    }
    if let Some(bytes) = marker {
        put(&root, &rel(ledger.marker_path(d).unwrap()), bytes);
    }
    ledger
}

/// A trusted ledger for `d` holding the manifest of `entries` and its own
/// marker, the day zip of `zip_size` bytes.
fn trusted(tag: &str, d: Day, manifest: &[u8], zip_size: u64) -> V3Ledger {
    let marker = marker_of(d, manifest, zip_size);
    ledger(tag, d, Some(manifest), Some(marker.as_bytes()))
}

/// A backslash day in the zip 2.0 shape: no directory entry, every file
/// `<folder>\Options\<T>.NFO.csv`.
fn backslash_day() -> (Day, Vec<(&'static str, u64, u32)>) {
    (
        day(2019, 4, 1),
        vec![
            (
                "GFDLNFO_TICK_01042019\\Options\\ZZINV25APR19900CE.NFO.csv",
                4_111,
                0x1234_abcd,
            ),
            (
                "GFDLNFO_TICK_01042019\\Options\\ZZINV25APR19900PE.NFO.csv",
                3_222,
                0x0000_0001,
            ),
            (
                "GFDLNFO_TICK_01042019\\Options\\ZZINV25APR19950CE.NFO.csv",
                0,
                0,
            ),
        ],
    )
}

/// A forward-slash day in the 2026-09-24 shape: two directory entries,
/// then the files.
fn forward_day() -> (Day, Vec<(&'static str, u64, u32)>) {
    (
        day(2026, 9, 24),
        vec![
            ("GFDLNFO_TICK_24092026/", 0, 0),
            ("GFDLNFO_TICK_24092026/Options/", 0, 0),
            (
                "GFDLNFO_TICK_24092026/Options/ZZINV29SEP26700CE.NFO.csv",
                9_001,
                0xdead_beef,
            ),
            (
                "GFDLNFO_TICK_24092026/Options/ZZINV29SEP26700PE.NFO.csv",
                8_002,
                0xffff_ffff,
            ),
        ],
    )
}

/// D-4814: a listing entry maps to `<MON_yyyy>/` plus the entry with every
/// `\` turned to `/`, directory lines kept on both sides, and a manifest the
/// tool wrote for the listing agrees line for line, on a backslash day and on
/// a forward-slash day with its two directory lines.
#[test]
fn a_v3_manifest_line_maps_to_its_listing_path() {
    let (d, entries) = backslash_day();
    let manifest = manifest_of(d, &entries);
    let text = std::str::from_utf8(&manifest).unwrap();
    assert!(
        text.starts_with(
            "APR_2019/GFDLNFO_TICK_01042019/Options/ZZINV25APR19900CE.NFO.csv\t4111\t1234abcd\t0\t0\tAPR_2019/GFDLNFO_TICK_01042019.zip\n"
        ),
        "{text}"
    );
    let v3 = trusted("v3-map-back", d, &manifest, 77);
    assert_eq!(
        v3.check(d, Some(&listing(d, &entries, Some(77)))),
        V3::Agrees { lines: 3 }
    );
    // A listing that read no zip (the tick store) is held to the lines only.
    assert_eq!(
        v3.check(d, Some(&listing(d, &entries, None))),
        V3::Agrees { lines: 3 }
    );

    let (d, entries) = forward_day();
    let manifest = manifest_of(d, &entries);
    let text = std::str::from_utf8(&manifest).unwrap();
    assert!(
        text.starts_with("SEP_2026/GFDLNFO_TICK_24092026/\t0\t00000000\t1\t0\tSEP_2026/GFDLNFO_TICK_24092026.zip\nSEP_2026/GFDLNFO_TICK_24092026/Options/\t0\t00000000\t1\t0\t"),
        "{text}"
    );
    let v3 = trusted("v3-map-fwd", d, &manifest, 99);
    let answer = v3.check(d, Some(&listing(d, &entries, Some(99))));
    assert_eq!(answer, V3::Agrees { lines: 4 });
    assert_eq!(answer.code(), 1);
    assert_eq!(answer.to_string(), "v3: agrees (4 lines)");
    // The directory lines are lines: a listing without them disagrees.
    let files_only = listing(d, &entries[2..], Some(99));
    let V3::Disagrees { count, first } = v3.check(d, Some(&files_only)) else {
        panic!("a listing without its directories agreed");
    };
    assert_eq!(count, 4);
    assert_eq!(
        first,
        Difference {
            path: "SEP_2026/GFDLNFO_TICK_24092026/Options/ZZINV29SEP26700CE.NFO.csv".into(),
            what: Differs::Path {
                manifest: "SEP_2026/GFDLNFO_TICK_24092026/".into()
            },
        }
    );
    // The paths are the documented ones, under `<yyyy>/_v3/`.
    let V3Ledger::At { root } = &v3 else {
        panic!("a configured ledger is unset");
    };
    let hex_name = "5345505f323032362f4746444c4e464f5f5449434b5f32343039323032362e7a6970";
    assert_eq!(
        v3.manifest_path(d).unwrap(),
        root.join(format!("2026/_v3/manifest/{hex_name}.ok.tsv"))
    );
    assert_eq!(
        v3.marker_path(d).unwrap(),
        root.join(format!("2026/_v3/ok/{hex_name}.ok"))
    );
}

/// D-4814: a trusted manifest that differs is named by the count of lines
/// that differ and the first difference: one size, one CRC and one missing
/// path, each first in its own day; then a path, a source and the day zip's
/// own size; a listing longer than the manifest; and a day the source does
/// not hold at all.
#[test]
fn a_disagreeing_v3_manifest_names_its_first_difference() {
    let (d, entries) = forward_day();
    let manifest = manifest_of(d, &entries);
    let v3 = trusted("v3-differ", d, &manifest, 99);
    let check =
        |edited: &[(&str, u64, u32)], zip: Option<u64>| v3.check(d, Some(&listing(d, edited, zip)));
    let path = |at: usize| format!("SEP_2026/{}", entries[at].0);

    // One size.
    let mut sized = entries.clone();
    sized[2].1 += 1;
    assert_eq!(
        check(&sized, Some(99)),
        V3::Disagrees {
            count: 1,
            first: Difference {
                path: path(2),
                what: Differs::Size {
                    listed: 9_002,
                    manifest: 9_001
                },
            },
        }
    );
    // One CRC.
    let mut crc = entries.clone();
    crc[3].2 = 0x0000_0002;
    let answer = check(&crc, Some(99));
    assert_eq!(
        answer,
        V3::Disagrees {
            count: 1,
            first: Difference {
                path: path(3),
                what: Differs::Crc {
                    listed: 2,
                    manifest: 0xffff_ffff
                },
            },
        }
    );
    assert_eq!(answer.code(), 2);
    assert_eq!(
        answer.to_string(),
        format!(
            "v3: disagrees (1 lines; first {}: crc 00000002 listed, ffffffff in the manifest)",
            path(3)
        )
    );
    // One missing path: the listing ends a line early.
    assert_eq!(
        check(&entries[..3], Some(99)),
        V3::Disagrees {
            count: 1,
            first: Difference {
                path: path(3),
                what: Differs::NotInListing,
            },
        }
    );
    // Both a size and a CRC: two lines, the size's first.
    let mut both = sized.clone();
    both[3].2 = 0x0000_0002;
    let V3::Disagrees { count, first } = check(&both, Some(99)) else {
        panic!("two differences agreed");
    };
    assert_eq!((count, first.path), (2, path(2)));
    // A path the manifest names otherwise, at the same place.
    let mut renamed = entries.clone();
    renamed[2].0 = "GFDLNFO_TICK_24092026/Options/ZZINV29SEP26701CE.NFO.csv";
    let V3::Disagrees { count, first } = check(&renamed, Some(99)) else {
        panic!("a renamed path agreed");
    };
    assert_eq!(count, 1);
    assert_eq!(first.what, Differs::Path { manifest: path(2) });
    // The day zip's own size against the marker's, counted as one line.
    assert_eq!(
        check(&entries, Some(100)),
        V3::Disagrees {
            count: 1,
            first: Difference {
                path: "SEP_2026/GFDLNFO_TICK_24092026.zip".into(),
                what: Differs::DayZipSize {
                    listed: 100,
                    ledger: 99
                },
            },
        }
    );
    // A listing longer than the manifest.
    let mut longer = entries.clone();
    longer.push((
        "GFDLNFO_TICK_24092026/Options/ZZINV29SEP26750CE.NFO.csv",
        5,
        5,
    ));
    let V3::Disagrees { count, first } = check(&longer, Some(99)) else {
        panic!("a longer listing agreed");
    };
    assert_eq!(count, 1);
    assert_eq!(first.what, Differs::NotInManifest);
    assert_eq!(
        first.to_string(),
        "SEP_2026/GFDLNFO_TICK_24092026/Options/ZZINV29SEP26750CE.NFO.csv: listed, not in the manifest"
    );
    // A manifest for a day the source does not hold: every line differs.
    let none: Option<&NfoDay<()>> = None;
    assert_eq!(
        v3.check(d, none),
        V3::Disagrees {
            count: 4,
            first: Difference {
                path: "SEP_2026/GFDLNFO_TICK_24092026/".into(),
                what: Differs::NotInListing,
            },
        }
    );

    // A line filed from another day zip, everything else equal.
    let other = String::from_utf8(manifest.clone())
        .unwrap()
        .replacen(
            "\tSEP_2026/GFDLNFO_TICK_24092026.zip\n",
            "\tSEP_2026/GFDLNFO_TICK_23092026.zip\n",
            1,
        )
        .into_bytes();
    let v3 = trusted("v3-source", d, &other, 99);
    assert_eq!(
        v3.check(d, Some(&listing(d, &entries, Some(99)))),
        V3::Disagrees {
            count: 1,
            first: Difference {
                path: "SEP_2026/GFDLNFO_TICK_24092026/".into(),
                what: Differs::Source {
                    manifest: "SEP_2026/GFDLNFO_TICK_23092026.zip".into()
                },
            },
        }
    );

    // Every difference's words.
    let words = [
        (Differs::NotInListing, "in the manifest, not listed"),
        (
            Differs::Path {
                manifest: "M".into(),
            },
            "the manifest names M at this line",
        ),
        (
            Differs::Size {
                listed: 1,
                manifest: 2,
            },
            "size 1 listed, 2 in the manifest",
        ),
        (
            Differs::Source {
                manifest: "Z".into(),
            },
            "the manifest files it from Z",
        ),
        (
            Differs::DayZipSize {
                listed: 3,
                ledger: 4,
            },
            "day zip of 3 bytes, 4 in the ok marker",
        ),
    ];
    for (what, want) in words {
        assert_eq!(what.to_string(), want);
    }
}

/// D-4814: a manifest is trusted only when its ok marker agrees with it; a
/// marker whose sha256 differs (one byte of the manifest flipped after the
/// marker was written) answers Unverified, never Agrees, and so does every
/// other marker fault, each by name.
#[test]
fn an_unverified_v3_manifest_is_not_trusted() {
    let (d, entries) = forward_day();
    let manifest = manifest_of(d, &entries);
    let marker = marker_of(d, &manifest, 99);
    let good = listing(d, &entries, Some(99));
    let answer = |tag: &str, manifest: &[u8], marker: Option<&str>| {
        ledger(tag, d, Some(manifest), marker.map(str::as_bytes)).check(d, Some(&good))
    };
    let unverified = |why: &str| V3::Unverified {
        why: why.to_owned(),
    };

    // One manifest byte flipped after the marker: the listing still agrees
    // with the flipped size, and the day is still not trusted.
    let flipped = String::from_utf8(manifest.clone())
        .unwrap()
        .replacen("\t9001\t", "\t9002\t", 1);
    let mut edited = entries.clone();
    edited[2].1 = 9_002;
    let v3 = ledger(
        "v3-flip",
        d,
        Some(flipped.as_bytes()),
        Some(marker.as_bytes()),
    );
    let got = v3.check(d, Some(&listing(d, &edited, Some(99))));
    assert_eq!(
        got,
        unverified("manifest sha256 differs from its ok marker")
    );
    assert_eq!(got.code(), 3);
    assert_eq!(
        got.to_string(),
        "v3: unverified (manifest sha256 differs from its ok marker)"
    );

    // No marker, and a marker that cannot be read as a file.
    assert_eq!(
        answer("v3-nomark", &manifest, None),
        unverified("no ok marker")
    );
    let dir_marker = ledger("v3-dirmark", d, Some(manifest.as_slice()), None);
    std::fs::create_dir_all(dir_marker.marker_path(d).unwrap()).unwrap();
    let V3::Unverified { why } = dir_marker.check(d, Some(&good)) else {
        panic!("a directory at the marker's name was trusted");
    };
    assert!(why.starts_with("ok marker unreadable ("), "{why}");

    // Each line of the marker, broken one at a time.
    let edit = |from: &str, to: &str| marker.replacen(from, to, 1);
    let cases: Vec<(String, &str)> = vec![
        (
            edit("x3-marker 1", "x3-marker 2"),
            "ok marker does not start \"x3-marker 1\"",
        ),
        (edit("run=", "run "), "ok marker line 9 is not key=value"),
        (
            format!("{marker}colour=blue\n"),
            "ok marker line 10 names an unknown key \"colour\"",
        ),
        (format!("{marker}files=4\n"), "ok marker states files twice"),
        (
            edit("\nrun=20990101-000000.1", ""),
            "ok marker states no run",
        ),
        (
            marker.replace("\nname_hex=", "\nname_hex_gone="),
            "ok marker line 2 names an unknown key \"name_hex_gone\"",
        ),
        (
            edit("name_hex=53", "name_hex=54"),
            "ok marker names another day zip",
        ),
        (
            edit("day_hex=53", "day_hex=54"),
            "ok marker names another day folder",
        ),
        (
            edit("crc=0badf00d", "crc=0badf00"),
            "ok marker crc is not 8 hex digits",
        ),
        (
            edit("crc=0badf00d", "crc=0badf00g"),
            "ok marker crc is not 8 hex digits",
        ),
        (edit("size=99", "size=+99"), "ok marker size is not decimal"),
        (
            edit("files=2", "files=two"),
            "ok marker files is not decimal",
        ),
        (
            edit("bytes=17003", "bytes="),
            "ok marker bytes is not decimal",
        ),
        (
            edit("files=2", "files=3"),
            "ok marker states 3 files of 17003 bytes, the manifest 2 of 17003",
        ),
        (
            edit("bytes=17003", "bytes=17004"),
            "ok marker states 2 files of 17004 bytes, the manifest 2 of 17003",
        ),
    ];
    for (at, (broken, why)) in cases.iter().enumerate() {
        assert_ne!(broken, &marker, "case {at} edited nothing");
        assert_eq!(
            answer(&format!("v3-mark-{at}"), &manifest, Some(broken.as_str())),
            unverified(why),
            "case {at}"
        );
    }
    // Each required key missing, one at a time.
    for key in MARKER_KEYS {
        let without = joined(
            marker
                .lines()
                .filter(|line| !line.starts_with(&format!("{key}="))),
        );
        assert_eq!(
            answer(&format!("v3-miss-{key}"), &manifest, Some(without.as_str())),
            unverified(&format!("ok marker states no {key}")),
            "{key}"
        );
    }
    // A marker that is not text, and one over its cap.
    let v3 = ledger(
        "v3-bin",
        d,
        Some(manifest.as_slice()),
        Some(&[0xff_u8, 0xfe, b'\n'][..]),
    );
    assert_eq!(
        v3.check(d, Some(&good)),
        unverified("ok marker is not UTF-8")
    );
    let huge = format!(
        "{marker}{}",
        "#".repeat(usize::try_from(MARKER_MAX_BYTES).unwrap())
    );
    assert_eq!(
        answer("v3-bigmark", &manifest, Some(huge.as_str())),
        unverified("ok marker over 4096 bytes")
    );
    // A marker beside no manifest is not an absent day.
    let lone = ledger("v3-lone", d, None, Some(marker.as_bytes()));
    assert_eq!(
        lone.check(d, Some(&good)),
        unverified("an ok marker with no manifest for SEP_2026/GFDLNFO_TICK_24092026.zip")
    );
    // A manifest that cannot be read as a file.
    let dir_manifest = ledger("v3-dirman", d, None, Some(marker.as_bytes()));
    std::fs::create_dir_all(dir_manifest.manifest_path(d).unwrap()).unwrap();
    let V3::Unverified { why } = dir_manifest.check(d, Some(&good)) else {
        panic!("a directory at the manifest's name was trusted");
    };
    assert!(why.starts_with("manifest unreadable ("), "{why}");
}

/// D-4867: the root comes from `BRUTEX_GDFL_V3_LEDGER` alone; unset or
/// empty, every day says so by the variable's name and no path is formed,
/// so nothing is ever read from a default place.
#[test]
fn an_unset_v3_ledger_is_named_never_defaulted() {
    let d = day(2019, 4, 1);
    let unset = |_: &str| None;
    let empty = |_: &str| Some(OsString::new());
    let envs: [&Env; 2] = [&unset, &empty];
    for env in envs {
        let v3 = V3Ledger::from_env(env);
        assert_eq!(v3, V3Ledger::Unset);
        assert_eq!(v3.manifest_path(d), None);
        assert_eq!(v3.marker_path(d), None);
        let none: Option<&NfoDay<()>> = None;
        let answer = v3.check(d, none);
        assert_eq!(
            answer,
            V3::Absent {
                why: "BRUTEX_GDFL_V3_LEDGER unset".into()
            }
        );
        assert_eq!(answer.code(), 0);
        assert_eq!(
            answer.to_string(),
            "v3: absent (BRUTEX_GDFL_V3_LEDGER unset)"
        );
    }
    // Set, it names that root and only that variable is read.
    let asked = std::cell::RefCell::new(Vec::new());
    let set = |var: &str| {
        asked.borrow_mut().push(var.to_owned());
        Some(OsString::from("/nowhere/v3"))
    };
    assert_eq!(
        V3Ledger::from_env(&set),
        V3Ledger::At {
            root: PathBuf::from("/nowhere/v3")
        }
    );
    assert_eq!(*asked.borrow(), [V3_LEDGER_VAR]);
    assert_eq!(V3_LEDGER_VAR, "BRUTEX_GDFL_V3_LEDGER");
    // A day zip the ledger holds nothing for is named, not an error.
    let v3 = ledger("v3-nothing", d, None, None);
    let none: Option<&NfoDay<()>> = None;
    let answer = v3.check(d, none);
    assert_eq!(
        answer,
        V3::Absent {
            why: "no manifest for APR_2019/GFDLNFO_TICK_01042019.zip".into()
        }
    );
    assert_eq!(answer.code(), 0);
}

/// D-4814: a manifest line is exactly 6 tab fields; a 5- or 7-field line, a
/// CRC that is not 8 hex digits, a size that is not decimal, an empty path,
/// a field 4 or 5 that is not decimal, a line that is not UTF-8 and a torn
/// last line are each named with their one-based line number, never skipped.
#[test]
fn a_malformed_v3_line_is_named_with_its_line_number() {
    let (d, entries) = forward_day();
    let good = listing(d, &entries, Some(99));
    let manifest = String::from_utf8(manifest_of(d, &entries)).unwrap();
    let lines: Vec<&str> = manifest.lines().collect();
    let zip = "SEP_2026/GFDLNFO_TICK_24092026.zip";
    // Line 3 replaced by `bad`, the rest as written.
    let with_line3 = |bad: &str| {
        let mut out = String::new();
        for (at, line) in lines.iter().enumerate() {
            out.push_str(if at == 2 { bad } else { line });
            out.push('\n');
        }
        out
    };
    let path = "SEP_2026/GFDLNFO_TICK_24092026/Options/ZZINV29SEP26700CE.NFO.csv";
    let cases = [
        (
            format!("{path}\t9001\tdeadbeef\t0\t0"),
            "5 tab fields, not 6",
        ),
        (
            format!("{path}\t9001\tdeadbeef\t0\t0\t{zip}\textra"),
            "7 tab fields, not 6",
        ),
        (
            format!("{path}\t9001\tdeadbeeg\t0\t0\t{zip}"),
            "crc \"deadbeeg\" is not 8 hex digits",
        ),
        (
            format!("{path}\t9001\tdeadbee\t0\t0\t{zip}"),
            "crc \"deadbee\" is not 8 hex digits",
        ),
        (
            format!("{path}\t9,001\tdeadbeef\t0\t0\t{zip}"),
            "size \"9,001\" is not decimal",
        ),
        (
            format!("{path}\t-1\tdeadbeef\t0\t0\t{zip}"),
            "size \"-1\" is not decimal",
        ),
        (format!("\t9001\tdeadbeef\t0\t0\t{zip}"), "an empty path"),
        (
            format!("{path}\t9001\tdeadbeef\tx\t0\t{zip}"),
            "field 4 \"x\" is not decimal",
        ),
        (
            format!("{path}\t9001\tdeadbeef\t0\t\t{zip}"),
            "field 5 \"\" is not decimal",
        ),
    ];
    for (at, (bad, why)) in cases.iter().enumerate() {
        let text = with_line3(bad);
        let v3 = trusted(&format!("v3-bad-{at}"), d, text.as_bytes(), 99);
        let answer = v3.check(d, Some(&good));
        assert_eq!(
            answer,
            V3::Malformed {
                line: 3,
                why: (*why).to_owned()
            },
            "case {at}"
        );
        assert_eq!(answer.code(), 3);
        assert_eq!(
            answer.to_string(),
            format!("v3: unverified (manifest line 3: {why})")
        );
    }
    // A line that is not UTF-8, named by its line.
    let mut binary = with_line3("PLACEHOLDER").into_bytes();
    let at = binary
        .windows(11)
        .position(|w| w == b"PLACEHOLDER")
        .unwrap();
    binary[at] = 0xff;
    let v3 = trusted("v3-utf8", d, &binary, 99);
    assert_eq!(
        v3.check(d, Some(&good)),
        V3::Malformed {
            line: 3,
            why: "not UTF-8".into()
        }
    );
    // A torn last line: its newline lost, its six fields whole.
    let torn = manifest.strip_suffix('\n').unwrap();
    let v3 = trusted("v3-torn", d, torn.as_bytes(), 99);
    assert_eq!(
        v3.check(d, Some(&good)),
        V3::Malformed {
            line: 4,
            why: "torn: no newline at its end".into()
        }
    );
    // An empty line in the middle is a line of one field.
    let gap = with_line3("");
    let v3 = trusted("v3-gap", d, gap.as_bytes(), 99);
    assert_eq!(
        v3.check(d, Some(&good)),
        V3::Malformed {
            line: 3,
            why: "1 tab fields, not 6".into()
        }
    );
}

/// The extremes: an empty manifest against an empty and a non-empty
/// listing; a manifest over its cap; the largest measured day (26,820
/// lines) agreeing in one pass; and every answer's code.
#[test]
fn the_v3_extremes_are_each_named() {
    let d = day(2018, 9, 3);
    let none: Option<&NfoDay<()>> = None;
    // Empty: no line, no file, no byte.
    let v3 = trusted("v3-empty", d, b"", 5);
    assert_eq!(
        v3.check(d, Some(&listing(d, &[], Some(5)))),
        V3::Agrees { lines: 0 }
    );
    assert_eq!(v3.check(d, none), V3::Agrees { lines: 0 });
    let V3::Disagrees { count, first } = v3.check(
        d,
        Some(&listing(d, &[("GFDLNFO_TICK_03092018/", 0, 0)], Some(5))),
    ) else {
        panic!("an empty manifest agreed with an entry");
    };
    assert_eq!((count, first.what), (1, Differs::NotInManifest));
    assert_eq!(first.path, "SEP_2018/GFDLNFO_TICK_03092018/");

    // Over the cap: never read whole, never trusted.
    let v3 = ledger("v3-bigman", d, None, None);
    let path = v3.manifest_path(d).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MANIFEST_MAX_BYTES + 1).unwrap();
    drop(file);
    let marker = v3.marker_path(d).unwrap();
    std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
    std::fs::write(&marker, b"x3-marker 1\n").unwrap();
    assert_eq!(
        v3.check(d, none),
        V3::Unverified {
            why: format!("manifest over {MANIFEST_MAX_BYTES} bytes")
        }
    );

    // The largest day: 26,820 lines, two of them directories.
    let names: Vec<String> = (0..26_818)
        .map(|at| format!("GFDLNFO_TICK_03092018\\Options\\ZZINV27SEP18{at:05}CE.NFO.csv"))
        .collect();
    let mut big: Vec<(&str, u64, u32)> = vec![
        ("GFDLNFO_TICK_03092018/", 0, 0),
        ("GFDLNFO_TICK_03092018/Options/", 0, 0),
    ];
    big.extend(names.iter().enumerate().map(|(at, name)| {
        (
            name.as_str(),
            100 + u64::try_from(at).unwrap(),
            u32::try_from(at).unwrap(),
        )
    }));
    let manifest = manifest_of(d, &big);
    let v3 = trusted("v3-big", d, &manifest, 1_000_000);
    let started = std::time::Instant::now();
    assert_eq!(
        v3.check(d, Some(&listing(d, &big, Some(1_000_000)))),
        V3::Agrees { lines: 26_820 }
    );
    eprintln!(
        "v3 check of 26,820 lines ({} bytes): {:?}",
        manifest.len(),
        started.elapsed()
    );
}

/// D-4814: the ledger is evidence and is never written, created, renamed,
/// removed, locked or flocked; its lock files are never named. Pinned on
/// this module's own code (comments aside): one read-only `File::open` and
/// nothing that writes.
#[test]
fn the_v3_ledger_is_never_opened_for_writing() {
    let source = include_str!("gdfl_v3.rs");
    let code = joined(
        source
            .lines()
            .take_while(|line| !line.starts_with("#[cfg(test)]"))
            .filter(|line| !line.trim_start().starts_with("//")),
    );
    for banned in [
        "OpenOptions",
        "File::create",
        "fs::write",
        "write_all",
        ".write(",
        "create_dir",
        "remove_",
        "rename",
        "set_len",
        "set_permissions",
        "flock",
        "lock(",
        "try_lock",
        "lock.guard",
        "place.lock",
        "hard_link",
        "symlink",
        "copy(",
    ] {
        assert!(!code.contains(banned), "gdfl_v3.rs names {banned}");
    }
    assert_eq!(code.matches("File::open").count(), 1, "one read-only open");
    assert!(code.contains("fn read_capped(path: &Path, cap: u64)"));
}

/// The operator's REAL ledger against the REAL year zips, read only
/// (IA-3-OPT's proof on real data; D-4814). Per year it counts the
/// manifests and ok markers; for each day of `BRUTEX_V3_FROM..=BRUTEX_V3_TO`
/// (default 2018-01-01..2026-12-31) or each day of `BRUTEX_V3_DAYS`
/// (`yyyy-mm-dd,...`) it lists the day through `NfoZips`, checks it against
/// the ledger, holds each ok marker's `crc` and `size` to the day zip's own
/// record in its year zip, and prints every answer that is not `agrees`, the
/// tallies and the check's p50, p99 and max. Nothing is written; no vendor
/// line is printed, only counts, codes and names.
#[test]
#[ignore = "requires BRUTEX_GDFL_V3_LEDGER and BRUTEX_GDFL_NFO_ARCHIVES; read-only"]
fn the_real_v3_ledger_agrees_with_the_real_year_zips() {
    use crate::gdfl_nfo::NfoSource as _;
    let env = |var: &str| std::env::var_os(var);
    let ledger = V3Ledger::from_env(&env);
    let V3Ledger::At { root } = &ledger else {
        panic!("BRUTEX_GDFL_V3_LEDGER is unset");
    };
    let dir = std::path::PathBuf::from(env("BRUTEX_GDFL_NFO_ARCHIVES").expect("archives"));
    let zips = NfoZips::new(&dir);
    // The first day asked lists the directory and walks every year zip once.
    let started = std::time::Instant::now();
    let first = zips.day(day(2019, 4, 1));
    eprintln!(
        "v3 first ask (directory listed, every year zip walked): {:?}, held={}",
        started.elapsed(),
        matches!(first, Ok(Some(_)))
    );
    let started = std::time::Instant::now();
    let again = zips.day(day(2019, 4, 2));
    eprintln!(
        "v3 a later ask: {:?}, held={}",
        started.elapsed(),
        matches!(again, Ok(Some(_)))
    );
    let parse = |text: &str| {
        let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
        day(
            parts[0],
            u8::try_from(parts[1]).unwrap(),
            u8::try_from(parts[2]).unwrap(),
        )
    };
    // Per year: manifests, markers.
    for year in 2018..=2026_u16 {
        let count = |sub: &str, suffix: &str| {
            std::fs::read_dir(root.join(format!("{year}/_v3/{sub}")))
                .unwrap()
                .filter(|name| {
                    name.as_ref()
                        .unwrap()
                        .file_name()
                        .to_str()
                        .is_some_and(|n| n.ends_with(suffix))
                })
                .count()
        };
        let (manifests, markers) = (count("manifest", ".ok.tsv"), count("ok", ".ok"));
        eprintln!("v3 {year}: manifests={manifests} ok_markers={markers}");
        assert_eq!(manifests, markers, "{year}: one ok marker a manifest");
        assert!(manifests > 0, "{year}: no manifest");
    }
    // Each day zip's own record, by name, from each year zip's directory.
    let mut records = std::collections::HashMap::new();
    for year in 2018..=2026_u16 {
        let Ok(file) = std::fs::File::open(dir.join(format!("{year}.zip"))) else {
            continue;
        };
        let len = file.metadata().unwrap().len();
        for entry in crate::gdfl_archive::zip_entries(&file, 0, len).unwrap() {
            records.insert(entry.name.clone(), (entry.len, entry.crc32));
        }
    }
    let days: Vec<Day> = if let Ok(list) = std::env::var("BRUTEX_V3_DAYS") {
        list.split(',').map(parse).collect()
    } else {
        let from = std::env::var("BRUTEX_V3_FROM").map_or(day(2018, 1, 1), |t| parse(&t));
        let to = std::env::var("BRUTEX_V3_TO").map_or(day(2026, 12, 31), |t| parse(&t));
        let mut out = Vec::new();
        let mut at = from;
        while at <= to {
            out.push(at);
            at = at.succ().unwrap();
        }
        out
    };
    let (mut codes, mut held, mut marker_agrees, mut marker_differs) =
        ([0_usize; 4], 0_usize, 0_usize, 0_usize);
    let mut nanos: Vec<u128> = Vec::new();
    let mut lines = 0_usize;
    for d in days {
        let listing = match zips.day(d) {
            Ok(listing) => listing,
            Err(why) => {
                eprintln!("v3 {d}: source refused: {why}");
                continue;
            }
        };
        if listing.is_some() {
            held += 1;
        }
        let started = std::time::Instant::now();
        let answer = ledger.check(d, listing.as_ref());
        let took = started.elapsed().as_nanos();
        if listing.is_none() && answer.code() == 0 {
            continue; // neither holds the day: a weekend or a holiday
        }
        nanos.push(took);
        codes[usize::from(answer.code())] += 1;
        if let V3::Agrees { lines: n } = answer {
            lines += n;
        } else {
            eprintln!("v3 {d}: {answer}");
        }
        // The marker's crc and size against the day zip's own record.
        let name = NfoZips::day_zip_name(d);
        if let (Ok(text), Some(&(len, crc))) = (
            std::fs::read_to_string(ledger.marker_path(d).unwrap()),
            records.get(&name),
        ) {
            let value = |key: &str| {
                text.lines()
                    .find_map(|line| line.strip_prefix(&format!("{key}=")))
                    .unwrap()
                    .to_owned()
            };
            if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
                marker_agrees += 1;
            } else {
                marker_differs += 1;
                eprintln!("v3 {d}: the ok marker's crc/size are not the day zip's");
            }
        }
    }
    nanos.sort_unstable();
    let at = |q: usize| {
        nanos
            .get((nanos.len().saturating_sub(1)) * q / 100)
            .copied()
            .unwrap_or(0)
    };
    eprintln!(
        "v3 tallies: held={held} absent={} agrees={} disagrees={} unverified={} lines={lines} marker_is_day_zip={marker_agrees} marker_differs={marker_differs}",
        codes[0], codes[1], codes[2], codes[3]
    );
    eprintln!(
        "v3 check: n={} p50={} us p99={} us max={} us",
        nanos.len(),
        at(50) / 1_000,
        at(99) / 1_000,
        nanos.last().copied().unwrap_or(0) / 1_000
    );
    assert_eq!(
        codes[2] + codes[3],
        0,
        "a real day disagrees or is unverified"
    );
    assert_eq!(marker_differs, 0);
}
