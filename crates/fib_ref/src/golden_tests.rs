use super::*;

#[test]
fn sha256_matches_the_standard_vectors() {
    assert_eq!(
        hex(&sha256(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        hex(&sha256(
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        )),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    let long = vec![b'a'; 1_000];
    assert_eq!(hex(&sha256(&long)).len(), 64);
}

#[test]
fn there_are_at_least_two_hundred_scenarios_with_distinct_ids() {
    let docs = all();
    assert!(docs.len() >= 200, "{} scenarios", docs.len());
    let mut ids: Vec<&str> = docs.iter().map(|d| d.id.as_str()).collect();
    ids.sort_unstable();
    let n = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), n, "duplicate ids");
    assert!(
        docs.iter().all(|d| d.outputs() > 0),
        "a scenario with no output"
    );
}

#[test]
fn rendering_is_byte_identical_on_a_second_run() {
    let a = files();
    let b = files();
    assert_eq!(a, b);
    assert!(manifest(&a).contains("FORMAT.md"));
}

#[test]
fn write_all_writes_every_file_and_the_manifest_and_refuses_a_bad_path() {
    let dir = std::env::temp_dir().join(format!("fib_ref_write_all_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let n = write_all(&dir).expect("a fresh scratch directory is writable");
    let files = files();
    assert_eq!(n, files.len() + 1);
    for (name, text) in &files {
        let back = std::fs::read_to_string(dir.join(name)).expect("written");
        assert_eq!(&back, text, "{name}");
    }
    let back = std::fs::read_to_string(dir.join("MANIFEST.md")).expect("written");
    assert_eq!(back, manifest(&files));
    // out of the box: a directory below a regular file cannot be made
    assert!(write_all(&dir.join("FORMAT.md").join("below")).is_err());
    std::fs::remove_dir_all(&dir).expect("the scratch directory is this test's own");
}

#[test]
fn the_writer_names_its_usage_writes_and_refuses_by_exit_code() {
    use std::ffi::OsString;
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = writer_main(&[OsString::from("fib_ref_golden")], &mut out, &mut err);
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&err),
        "usage: fib_ref_golden <directory>\n"
    );
    // no argument at all, not even the program's name
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(writer_main(&[], &mut out, &mut err), 2);
    let dir = std::env::temp_dir().join(format!("fib_ref_writer_main_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let args = [
        OsString::from("fib_ref_golden"),
        dir.clone().into_os_string(),
    ];
    assert_eq!(writer_main(&args, &mut out, &mut err), 0);
    assert!(err.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&out),
        format!("wrote {} files into {}\n", files().len() + 1, dir.display())
    );
    // a directory below a regular file: refused, named, exit 1
    let bad = dir.join("INDEX.md").join("below");
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let args = [
        OsString::from("fib_ref_golden"),
        bad.clone().into_os_string(),
    ];
    assert_eq!(writer_main(&args, &mut out, &mut err), 1);
    assert!(out.is_empty());
    assert!(String::from_utf8_lossy(&err).starts_with(&format!("writing {}: ", bad.display())));
    std::fs::remove_dir_all(&dir).expect("the scratch directory is this test's own");
}

#[test]
fn write_all_refuses_when_a_file_it_writes_is_a_directory() {
    // a directory where a scenario file goes, then where the manifest goes
    for blocker in ["FORMAT.md", "MANIFEST.md"] {
        let dir =
            std::env::temp_dir().join(format!("fib_ref_blocked_{}_{blocker}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(blocker)).expect("a scratch directory");
        assert!(write_all(&dir).is_err(), "{blocker}");
        std::fs::remove_dir_all(&dir).expect("the scratch directory is this test's own");
    }
}
