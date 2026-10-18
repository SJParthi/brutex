//! The new-work authority words — `cal1`, `pit`, `data1`, `data2` and `of1` —
//! routed here by FIB-8 and wired by FIB-10. Until FIB-10, [`register`] refuses
//! every one of them by name: an authority that is not built is never quietly
//! replaced by an interim one (`CLAUDE.md` §4).

/// The new-work authority words.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "read by `register`, which FIB-8 routes to")
)]
pub(super) const WORDS: &[&str] = &["cal1", "pit", "data1", "data2", "of1"];

/// Registers a new-work authority by its word. Refuses every word until
/// FIB-10, and a word that is none of them always.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "FIB-8 routes the authority words here; until then only the tests call it"
    )
)]
pub(super) fn register(word: &str) -> Result<(), String> {
    if WORDS.contains(&word) {
        Err(format!(
            "authority `{word}` is not built yet: it arrives with FIB-10"
        ))
    } else {
        Err(format!(
            "`{word}` is not a new-work authority word; the words are {}",
            WORDS.join(", ")
        ))
    }
}
