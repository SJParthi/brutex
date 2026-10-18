//! The walker's source-shape tests (FIB-5): no typed number in any walker
//! file, and no bar at or after 15:10:00 kept for any exit to read.

use vocab::fib_cube::untyped_numbers;

/// The shipping part of `source`: everything before its test module.
fn shipping(source: &str) -> &str {
    vocab::fib_cube::shipping_part(source)
}

/// NO TYPED NUMBERS (operator 16 Oct 11:24 IST): every FIB-5 file holds no
/// numeric literal but 0 and 1 in its shipping part; the generated exit set
/// is the only source of an exit value.
#[test]
fn every_fib5_file_types_no_number() {
    for (name, source) in [
        ("walk.rs", include_str!("../walk.rs")),
        ("walk/exits.rs", include_str!("exits.rs")),
        ("walk/index.rs", include_str!("index.rs")),
        ("walk/slot.rs", include_str!("slot.rs")),
        ("codec.rs", include_str!("../codec.rs")),
        ("types.rs", include_str!("../types.rs")),
    ] {
        let typed = untyped_numbers(source);
        assert!(typed.is_empty(), "{name} types {typed:?}");
    }
}

/// FM-F039: the index keeps no bar at or after 15:10:00, and the slot reads
/// bars only through the index, so no exit can read one.
#[test]
fn no_bar_at_or_after_15_10_is_kept_or_read() {
    let index = shipping(include_str!("index.rs"));
    let cut = "if u64::from(*second) >= SESSION_AUTO_EXIT_BY_SECS {\n                continue;";
    assert!(index.contains(cut), "the cut before any bar is kept");
    assert_eq!(
        index.matches("kept.push(").count(),
        1,
        "one way into the index"
    );
    let slot = shipping(include_str!("slot.rs"));
    assert!(
        !slot.contains("input.bars"),
        "the slot never reads the input bars"
    );
    assert!(!slot.contains("Candle"), "nor any candle");
    assert!(slot.contains("self.side.bars.get(self.cursor)"));
}

/// The walker's files do no I/O and call nothing that prints (gates 17 and
/// 22), checked here too so a later edit fails in this crate's own tests.
#[test]
fn the_walker_files_do_no_io_and_print_nothing() {
    for (name, source) in [
        ("walk.rs", include_str!("../walk.rs")),
        ("walk/exits.rs", include_str!("exits.rs")),
        ("walk/index.rs", include_str!("index.rs")),
        ("walk/slot.rs", include_str!("slot.rs")),
        ("codec.rs", include_str!("../codec.rs")),
    ] {
        let code = shipping(source);
        for banned in [
            "std::fs",
            "File::",
            "OpenOptions",
            "read_dir",
            "std::process",
            "include_bytes!",
            "println!",
            "eprintln!",
            "telemetry",
        ] {
            assert!(!code.contains(banned), "{name} names {banned}");
        }
    }
}
