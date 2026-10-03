#![cfg(test)]
//! Torn-write recovery of the receipt-last publication files (D-1760).
use super::{prepare_in_namespace, read_exact, write_or_equal};
use std::fs;
use std::path::PathBuf;

/// A private root, named for its owner so two tests cannot collide.
fn scratch(name: &str) -> PathBuf {
    let mut root = std::env::temp_dir();
    root.push(format!("brutex-persist-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("scratch root is creatable");
    root
}

#[test]
fn a_fresh_write_lands_whole_and_an_identical_retry_is_accepted() -> Result<(), String> {
    let root = scratch("fresh");
    let path = root.join("body.bin");
    write_or_equal(&path, b"0123456789")?;
    assert_eq!(read_exact(&path, 10)?, b"0123456789");
    write_or_equal(&path, b"0123456789")?;
    assert_eq!(read_exact(&path, 10)?, b"0123456789");
    Ok(())
}

#[test]
fn a_torn_prefix_is_finished_rather_than_wedging_the_identity() -> Result<(), String> {
    let root = scratch("torn");
    let path = root.join("body.bin");
    fs::write(&path, b"01234").map_err(|why| why.to_string())?;
    write_or_equal(&path, b"0123456789")?;
    assert_eq!(read_exact(&path, 64)?, b"0123456789");
    let empty = root.join("empty.bin");
    fs::write(&empty, b"").map_err(|why| why.to_string())?;
    write_or_equal(&empty, b"abc")?;
    assert_eq!(read_exact(&empty, 64)?, b"abc");
    Ok(())
}

#[test]
fn bytes_that_are_not_a_prefix_or_are_longer_are_still_refused() -> Result<(), String> {
    let root = scratch("refuse");
    for (name, existing) in [
        ("diverged", &b"01x34"[..]),
        ("same-length", &b"012345678X"[..]),
        ("longer", &b"0123456789AB"[..]),
        ("one-longer", &b"0123456789A"[..]),
    ] {
        let path = root.join(name);
        fs::write(&path, existing).map_err(|why| why.to_string())?;
        assert!(write_or_equal(&path, b"0123456789").is_err(), "{name}");
        assert_eq!(fs::read(&path).map_err(|why| why.to_string())?, existing, "{name}");
    }
    Ok(())
}

#[test]
fn a_publication_cut_short_after_its_directory_resumes_to_a_receipt() -> Result<(), String> {
    let root = scratch("resume");
    let identity = [7_u8; 32];
    let body = b"the whole intended body".to_vec();
    let directory = root
        .join("index-stop-vix-reference-v1")
        .join(crate::identity_hex(&identity));
    fs::create_dir_all(&directory).map_err(|why| why.to_string())?;
    fs::write(directory.join("body.bin"), &body[..9]).map_err(|why| why.to_string())?;
    let pending = prepare_in_namespace(&root, "index-stop-vix-reference-v1", identity, &body)?;
    let digest = brutex_core::blake3::hash(&body);
    pending.verify_body(digest, body.len() as u64)?;
    let completion = pending.finish(identity, digest, body.len() as u64)?;
    drop(pending);
    super::verify(&directory, identity, digest, body.len() as u64, completion)
}
