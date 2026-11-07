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
