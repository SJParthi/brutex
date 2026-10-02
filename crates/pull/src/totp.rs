//! Time-based one-time passwords, RFC 6238.
//!
//! # Why this exists, and why it did not until now
//!
//! `CLAUDE.md` §8 says this repository never mints a token, and the reason is
//! sound: a broker issues ONE active token per client, so minting a new one
//! invalidates whatever another system is holding. `tickvault` shares these
//! credentials, and a mint here would take its access down.
//!
//! The word `totp` appeared zero times in this workspace before this file. That
//! was not an oversight — it was the rule implemented exactly.
//!
//! What changed is the evidence. Parameter Store shows the Groww token
//! rewritten daily and the Dhan token untouched since 2026-07-25. A token
//! nothing has refreshed in fourteen days is a token nothing is using, and the
//! sharing arrangement §8 protects is the thing that appears to have stopped.
//!
//! **This module computes a code. It does not mint anything.** Generating a
//! TOTP is arithmetic over a shared secret and the clock; it touches no vendor,
//! invalidates nothing, and is safe to have whether or not the mint is ever
//! wired. The decision that actually carries risk — exchanging a code for a
//! token — is deliberately not here, so that landing this cannot break
//! `tickvault` by accident.
//!
//! # The shape
//!
//! ```text
//! counter = floor(unix_seconds / step)
//! mac     = HMAC-SHA1(base32_decode(secret), counter as 8 bytes, big-endian)
//! offset  = mac[19] & 0x0f
//! code    = (u32 from mac[offset..offset+4], top bit cleared) % 10^digits
//! ```
//!
//! Every constant is RFC 6238's default and every one is named rather than
//! written inline, because a 30 that means "seconds per step" and a 30 that
//! means anything else are the same character.
//!
//! # O(1)
//!
//! Constant in everything that varies. The secret is bounded by
//! [`MAX_SECRET_LEN`], the MAC is one fixed-size block, and the truncation is
//! four bytes at a computed offset. No loop here runs a number of times that
//! depends on the clock, the counter, or the code.
//!
//! Both halves of that are held, and by different tests:
//!
//! * the secret's bound, by
//!   `pull::totp::the_length_bound_is_checked_before_a_single_character_is_decoded`
//!   — the only loop in this file is the base32 decode, and it cannot start on
//!   a secret past the bound;
//! * the counter's width, by
//!   `pull::totp::the_rfc_6238_sha1_vectors_reproduce_exactly` — RFC 6238's
//!   T = 20,000,000,000 vector is a counter with four leading zero bytes, so
//!   reproducing it pins the message at eight big-endian bytes for **every**
//!   counter rather than at however many a shorter encoding would send.
//!
//! Neither is a timing. This has not been benched, and the claim above is a
//! count of operations, not of nanoseconds.

use hmac::{Hmac, Mac};
use sha1::Sha1;

/// Seconds per step. RFC 6238 §4's default (`X = 30`).
///
/// It used to add *"and what every broker documented in `docs/00-charter.md`
/// uses"*. The charter records no step length for any broker — its only TOTP
/// row is Groww's *"TOTP-derived daily token, reset 06:00 IST"* — so that half
/// was a vendor claim with no source, which `CLAUDE.md` §3 rule 1 forbids.
/// Whether a given broker uses 30 s is **UNVERIFIED** here. D-1374.
pub const STEP_SECONDS: u64 = 30;

/// Digits in the code. RFC 6238 §5.3 default.
pub const DIGITS: u32 = 6;

/// The longest base32 secret this reads.
///
/// A bound rather than an unbounded decode, because the input is a credential
/// read from Parameter Store and a length is the one property worth refusing on
/// before doing arithmetic with it. 128 base32 characters is 80 bytes of key —
/// far past the 20 bytes SHA-1's block structure makes meaningful.
pub const MAX_SECRET_LEN: usize = 128;

/// What a code could not be computed from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TotpError {
    /// The secret was empty.
    Empty,
    /// The secret was longer than [`MAX_SECRET_LEN`].
    TooLong {
        /// How many characters arrived.
        len: usize,
    },
    /// The secret held a character outside the base32 alphabet.
    NotBase32 {
        /// The offending byte, as it was written.
        byte: u8,
    },
    /// The HMAC construction refused the decoded key.
    ///
    /// # Why this exists when no input can produce it
    ///
    /// HMAC is defined for every key length — RFC 2104 hashes a key longer than
    /// the block and zero-pads a shorter one — so `new_from_slice` cannot
    /// actually refuse, and an empty key is already refused above as
    /// [`Self::Empty`].
    ///
    /// It existed here as `unreachable!("HMAC accepts every key length")`, and
    /// the reasoning was sound but the shape was not: `CLAUDE.md` §4 says
    /// degrade loudly and name the reason, or refuse — it has no clause
    /// exempting a case the author is confident cannot arise. CI's gate 11 kept
    /// this file named rather than allowlisted for exactly that reason, calling
    /// it "a real §4 violation left visible for its owner".
    ///
    /// A refusal costs one variant and cannot kill the process that reads a
    /// broker token. If the dependency's contract ever changes underneath this,
    /// the caller sees a sentence instead of a stack trace.
    HmacRefusedTheKey,
}

impl core::fmt::Display for TotpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::Empty => f.write_str("the shared secret is empty"),
            Self::TooLong { len } => write!(
                f,
                "the shared secret is {len} characters, max {MAX_SECRET_LEN}"
            ),
            Self::NotBase32 { byte } => write!(
                f,
                "the shared secret holds byte {byte:#04x}, outside RFC 4648 base32 (A-Z, 2-7)"
            ),
            Self::HmacRefusedTheKey => f.write_str(
                "the HMAC construction refused the decoded secret, which RFC 2104 \
                 says it cannot do for any key length — treat this as a broken \
                 build rather than a bad secret",
            ),
        }
    }
}

impl core::error::Error for TotpError {}

/// Decode an RFC 4648 base32 secret.
///
/// # Why hand-written
///
/// One alphabet and one bit-packing loop. A shared secret is usually written
/// without `=`; trailing RFC 4648 padding is accepted and skipped, and a `=`
/// anywhere but the tail is refused (D-1373). A dependency for twenty lines is the trade
/// `CLAUDE.md` §2's spirit refuses, and `crates/pull` already hand-rolls `SigV4`
/// for the same reason.
///
/// Lower case is accepted. Authenticator apps and vendor consoles disagree on
/// case and the alphabet is case-insensitive by definition, so refusing a
/// lower-case secret would be refusing a correct one. Spaces and hyphens are
/// skipped for the same reason: they are how these are displayed to be read
/// aloud, and an operator pasting one back is not making a mistake.
///
/// # Errors
///
/// [`TotpError`] for an empty secret, one past [`MAX_SECRET_LEN`], or one
/// holding a character the alphabet does not contain.
pub fn base32_decode(secret: &str) -> Result<Vec<u8>, TotpError> {
    let decoded = decode_alphabet(secret);
    if let Err(why) = &decoded {
        note_refusal(why, secret.len());
    }
    decoded
}

/// Why a refused secret is worth a line, and why the line cannot carry any of
/// it.
///
/// A refusal here is terminal for the run: no code means no exchange, and no
/// exchange means the pull does not authenticate. Without this event that
/// failure arrives at the vendor as a rejected code and nothing anywhere says
/// the secret never decoded in the first place — the operator debugs the
/// broker instead of the one line of configuration that is actually wrong.
///
/// **Nothing that arrived is written back out.** `CLAUDE.md` §8 keeps every
/// parameter path out of a tracked file because this repository is public, and
/// a log is a file an operator pastes into an issue; a shared secret is the
/// value at the end of one of those paths and inherits the rule with room to
/// spare. So the line carries the KIND of refusal and the LENGTH, and stops.
///
/// [`TotpError::NotBase32`] carries the offending byte and its `Display` shows
/// it, because an operator fixing their own configuration is looking at the
/// secret anyway. This does not pass that byte along. It is a character of the
/// credential as supplied, and "it is not a character the alphabet decodes" is
/// an argument about the key, not about the string it was pasted from. `fault`
/// says which of the three things went wrong, which is what a reader needs to
/// choose between "the file has no secret", "the field ran into the next one"
/// and "somebody transcribed an O as a zero".
///
/// The length is the exception the rule already grants: a count is not a value,
/// and `chars` against `max` is the whole diagnosis for the over-long case.
fn note_refusal(why: &TotpError, len: usize) {
    let fault = match *why {
        TotpError::Empty => "empty",
        TotpError::TooLong { .. } => "too-long",
        TotpError::NotBase32 { .. } => "not-base32",
        // NOT A SECRET FAULT AT ALL, and the word says so. The other three name
        // something the operator can fix in their own configuration; this one
        // says the build is wrong, and sending an operator to re-read their
        // secret over it would be the wrong instruction.
        TotpError::HmacRefusedTheKey => "hmac-refused",
    };
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("pull.totp", "the shared secret will not decode")
            .with("fault", telemetry::Value::Str(fault))
            .with("chars", telemetry::Value::Uint(len as u64))
            .with("max", telemetry::Value::Uint(MAX_SECRET_LEN as u64)),
    );
}

/// The decode itself, with no observation in it.
///
/// Split out so there is exactly ONE place a refusal becomes a log line, and
/// so that place sees a `TotpError` rather than the secret. Four early returns
/// each writing their own event would be four chances to reach for the input
/// while it is still in scope; here it is not in scope at all.
fn decode_alphabet(secret: &str) -> Result<Vec<u8>, TotpError> {
    if secret.is_empty() {
        return Err(TotpError::Empty);
    }
    if secret.len() > MAX_SECRET_LEN {
        return Err(TotpError::TooLong { len: secret.len() });
    }

    let mut out = Vec::with_capacity(secret.len() * 5 / 8 + 1);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    // Whether padding has begun. RFC 4648 §6 padding is a TAIL: once a `=` has
    // been read, only more `=` (or a display separator) may follow.
    let mut padded = false;

    for byte in secret.bytes() {
        // Separators as displayed.
        if matches!(byte, b' ' | b'-') {
            continue;
        }
        // `=` is padding and carries no bits — at the END. It used to be
        // skipped wherever it stood, so `GEZD=GNBV` decoded to the same key as
        // `GEZDGNBV` and produced a valid-looking code from a secret that was
        // mis-transcribed: the exact failure `a_bad_character_is_refused_and_
        // not_skipped` names. A data character after padding is refused, and
        // the byte reported is the `=` that broke the tail. D-1373.
        if byte == b'=' {
            padded = true;
            continue;
        }
        if padded {
            return Err(TotpError::NotBase32 { byte: b'=' });
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a',
            b'2'..=b'7' => byte - b'2' + 26,
            other => return Err(TotpError::NotBase32 { byte: other }),
        };
        acc = (acc << 5) | u32::from(value);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            // The cast is safe: the shift leaves exactly the low 8 bits.
            out.push(u8::try_from((acc >> bits) & 0xff).unwrap_or(0));
        }
    }

    if out.is_empty() {
        return Err(TotpError::Empty);
    }
    Ok(out)
}

/// The code for one counter value.
///
/// Separated from the clock so a test can pin a counter rather than sleep, the
/// same reason [`crate::rate::Governor`] takes `now_micros` as an argument.
/// RFC 6238's own test vectors are counters, and this is the function they
/// address.
///
/// # Errors
///
/// Whatever [`base32_decode`] refuses, and [`TotpError::HmacRefusedTheKey`] if
/// the HMAC construction ever declines a key RFC 2104 says it must accept.
pub fn code_at_counter(secret: &str, counter: u64) -> Result<String, TotpError> {
    let key = base32_decode(secret)?;

    // `new_from_slice` accepts any length for HMAC — RFC 2104 hashes a key
    // longer than the block and zero-pads a shorter one — so this cannot fail
    // for a non-empty key, and an empty one was refused above.
    //
    // REFUSED RATHER THAN UNREACHABLE. It read
    // `.unwrap_or_else(|_| unreachable!("HMAC accepts every key length"))`, and
    // the sentence was true. §4 still does not have an "unless it cannot
    // happen" clause, and the cost of honouring it here is one variant and one
    // `?`. See `TotpError::HmacRefusedTheKey`.
    let mut mac =
        <Hmac<Sha1> as Mac>::new_from_slice(&key).map_err(|_| TotpError::HmacRefusedTheKey)?;
    mac.update(&counter.to_be_bytes());
    let tag = mac.finalize().into_bytes();

    // DYNAMIC TRUNCATION, RFC 4226 §5.4. The low nibble of the LAST byte picks
    // where to read, so the code is not always drawn from the same four bytes
    // of the MAC.
    //
    // Written with `last` and `get` rather than `tag[19]` and `tag[offset..]`.
    // SHA-1 is 20 bytes and the nibble is at most 15, so every index below is
    // provably in range — but "provably" is an argument, and an argument is
    // what an index is checked against at runtime anyway. The fallible form
    // costs nothing here and cannot panic on a MAC whose width someone later
    // changes.
    let Some(&last) = tag.last() else {
        // Unreachable: `finalize` returns a fixed-size array, never empty.
        return Err(TotpError::Empty);
    };
    let offset = usize::from(last & 0x0f);
    let Some(window) = tag.get(offset..offset + 4) else {
        return Err(TotpError::Empty);
    };
    let slice = u32::from_be_bytes([
        *window.first().unwrap_or(&0),
        *window.get(1).unwrap_or(&0),
        *window.get(2).unwrap_or(&0),
        *window.get(3).unwrap_or(&0),
    ]);
    // The top bit is cleared so the value is positive in signed languages —
    // the spec says so, and a code that differed by platform would be worse
    // than one that is merely odd.
    let truncated = slice & 0x7fff_ffff;

    let modulus = 10_u32.pow(DIGITS);
    Ok(format!(
        "{:0width$}",
        truncated % modulus,
        width = DIGITS as usize
    ))
}

/// The code for a wall-clock second.
///
/// `unix_seconds` is an argument rather than a call to the clock inside, for
/// the reason `CLAUDE.md` §3 rule 5 gives about reruns: a code whose value
/// depends on when it ran is a code no test can pin.
///
/// # Errors
///
/// Whatever [`base32_decode`] refuses.
pub fn code_at(secret: &str, unix_seconds: u64) -> Result<String, TotpError> {
    code_at_counter(secret, unix_seconds / STEP_SECONDS)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;

    /// The refusal that replaced an `unreachable!` still says something useful.
    ///
    /// # Why a variant no input can produce is still worth a test
    ///
    /// It cannot be reached through `code_at_counter` — RFC 2104 defines HMAC
    /// for every key length, which is exactly why the old code wrote
    /// `unreachable!("HMAC accepts every key length")` and was right about the
    /// fact while wrong about the shape. What a test CAN pin is that the two
    /// things a reader would meet — the sentence and the telemetry word — exist
    /// and point at the build rather than at the operator's secret. Sending
    /// somebody to re-read a correct secret is the wrong instruction, and it is
    /// the one a copied-from-a-neighbour message would have given.
    #[test]
    fn the_hmac_refusal_blames_the_build_and_not_the_secret() {
        let said = TotpError::HmacRefusedTheKey.to_string();
        assert!(
            said.contains("RFC 2104"),
            "it cites the rule that makes this impossible: {said}"
        );
        assert!(
            said.contains("broken build"),
            "and it tells the reader where to look, which is NOT at their \
             secret: {said}"
        );
        for other in [
            TotpError::Empty,
            TotpError::TooLong { len: 999 },
            TotpError::NotBase32 { byte: b'0' },
        ] {
            assert_ne!(
                other.to_string(),
                said,
                "each refusal is its own sentence; a shared one would make the \
                 four indistinguishable in a log"
            );
        }
    }

    /// RFC 6238 Appendix B, the SHA-1 rows.
    ///
    /// The spec's test secret is the ASCII string `12345678901234567890`, which
    /// is `GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ` in base32. These are the
    /// published values, not values this implementation produced — a test that
    /// asserts what the code already does proves only that it is deterministic.
    #[test]
    fn the_rfc_6238_sha1_vectors_reproduce_exactly() {
        const SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        for (unix_seconds, want) in [
            (59_u64, "287082"),
            (1_111_111_109, "081804"),
            (1_111_111_111, "050471"),
            (1_234_567_890, "005924"),
            (2_000_000_000, "279037"),
            (20_000_000_000, "353130"),
        ] {
            assert_eq!(
                code_at(SECRET, unix_seconds).expect("a legal secret"),
                want,
                "RFC 6238 Appendix B, T = {unix_seconds}"
            );
        }
    }

    /// A code is stable inside its step and changes at the boundary.
    ///
    /// This is the property an operator actually relies on: a code read at
    /// :00 and used at :29 is the same code, and one read at :29 and used at
    /// :31 is not. Asserted at the boundary rather than in the middle, because
    /// an off-by-one in the division shows up nowhere else.
    #[test]
    fn a_code_holds_for_its_whole_step_and_turns_over_at_the_boundary() {
        const SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        let base = 1_700_000_000 - (1_700_000_000 % STEP_SECONDS);

        let first = code_at(SECRET, base).expect("legal");
        assert_eq!(
            code_at(SECRET, base + STEP_SECONDS - 1).expect("legal"),
            first,
            "the last second of a step carries the same code as the first"
        );
        assert_ne!(
            code_at(SECRET, base + STEP_SECONDS).expect("legal"),
            first,
            "and the next step does not"
        );
    }

    /// Every code is exactly [`DIGITS`] characters, leading zeros included.
    ///
    /// `1_234_567_890` is in the vector set above precisely because its code is
    /// `005924` — a value that formats as four characters unless the width is
    /// stated, and a four-character code is rejected by every vendor.
    #[test]
    fn a_code_keeps_its_leading_zeros() {
        const SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        for t in 0..600_u64 {
            let code = code_at(SECRET, t * 7).expect("legal");
            assert_eq!(
                code.len(),
                DIGITS as usize,
                "{code} is not {DIGITS} characters"
            );
            assert!(code.bytes().all(|b| b.is_ascii_digit()), "{code}");
        }
    }

    /// The alphabet is case-insensitive, and display separators are skipped.
    ///
    /// A secret is shown grouped and in either case depending on the console
    /// that printed it. Refusing a correctly-transcribed one would be refusing
    /// the operator, not the input.
    #[test]
    fn case_and_display_separators_do_not_change_the_secret() {
        const CANONICAL: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        for spelling in [
            "gezdgnbvgy3tqojqgezdgnbvgy3tqojq",
            "GEZD GNBV GY3T QOJQ GEZD GNBV GY3T QOJQ",
            "GEZD-GNBV-GY3T-QOJQ-GEZD-GNBV-GY3T-QOJQ",
            "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ====",
        ] {
            assert_eq!(
                base32_decode(spelling).expect("legal"),
                base32_decode(CANONICAL).expect("legal"),
                "{spelling:?} is the same secret"
            );
        }
    }

    /// Every refusal names what was wrong, and none of them is generic.
    ///
    /// A secret that will not decode is an operator-facing failure — they have
    /// to fix it — so "malformed" is not an answer. `CLAUDE.md` §4: degrade
    /// loudly and name the reason.
    #[test]
    fn every_refusal_says_which_thing_was_wrong() {
        assert_eq!(base32_decode(""), Err(TotpError::Empty));
        assert_eq!(base32_decode("   "), Err(TotpError::Empty));

        let long = "A".repeat(MAX_SECRET_LEN + 1);
        assert_eq!(
            base32_decode(&long),
            Err(TotpError::TooLong {
                len: MAX_SECRET_LEN + 1
            })
        );

        // `0`, `1` and `8` are the characters base32 leaves out precisely
        // because they are confusable with O, I and B.
        for bad in ['0', '1', '8', '9', '!'] {
            let secret = format!("GEZD{bad}GNBV");
            assert_eq!(
                base32_decode(&secret),
                Err(TotpError::NotBase32 { byte: bad as u8 }),
                "{bad} is not in the alphabet"
            );
        }

        // And the messages are distinct, so a log tells them apart.
        let seen: Vec<String> = [
            TotpError::Empty,
            TotpError::TooLong { len: 999 },
            TotpError::NotBase32 { byte: b'0' },
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        for (n, one) in seen.iter().enumerate() {
            for other in &seen[n + 1..] {
                assert_ne!(one, other, "two refusals read identically");
            }
        }
    }

    /// A secret this module refuses is never silently treated as another.
    ///
    /// The failure worth preventing: a bad character skipped rather than
    /// refused would produce a VALID-LOOKING code from the wrong key, which the
    /// vendor rejects and nothing here explains.
    #[test]
    fn a_bad_character_is_refused_and_not_skipped() {
        assert!(code_at("GEZD0GNBV", 59).is_err());
        assert!(base32_decode("GEZD0GNBV").is_err());
    }

    /// Padding is a tail, never a separator.
    ///
    /// Before D-1373 a `=` was skipped wherever it stood, so a secret with one
    /// mis-keyed into its middle decoded to a shorter, DIFFERENT key and
    /// produced a valid-looking code — refused at the vendor with nothing
    /// here to say why.
    #[test]
    fn padding_inside_the_secret_is_refused_and_only_a_tail_is_skipped() {
        for inside in [
            "GEZD=GNBV",
            "=GEZDGNBV",
            "GEZDGNBV=A",
            "GEZDGNBV==A==",
            "GE=-ZD",
        ] {
            assert_eq!(
                base32_decode(inside),
                Err(TotpError::NotBase32 { byte: b'=' }),
                "{inside:?} has data after padding"
            );
            assert!(code_at(inside, 59).is_err(), "{inside:?} mints no code");
        }
        // A tail of padding, with or without display separators after it, is
        // still the canonical secret.
        let canonical = base32_decode("GEZDGNBV").expect("legal");
        for tail in ["GEZDGNBV=", "GEZDGNBV======", "GEZDGNBV== ", "GEZDGNBV=-="] {
            assert_eq!(
                base32_decode(tail).expect("legal tail"),
                canonical,
                "{tail:?}"
            );
        }
        // And a secret that is ALL padding carries no key.
        assert_eq!(base32_decode("===="), Err(TotpError::Empty));
    }

    /// The length bound is checked BEFORE a character is decoded, which is what
    /// makes the only loop here bounded by a constant.
    ///
    /// The module header rests its cost claim on "the secret is bounded by
    /// [`MAX_SECRET_LEN`]". [`every_refusal_says_which_thing_was_wrong`] above
    /// proves the refusal exists; it cannot see *where* it happens, because its
    /// over-long secret is all `A` and would decode cleanly either way.
    ///
    /// So the fixture here is over-long **and** unspellable: characters past the
    /// bound that the alphabet does not contain. If the length test were moved
    /// after the decode loop — or dropped so the loop ran to the end of whatever
    /// arrived — the refusal would come back as `NotBase32` instead, and this
    /// assertion is the difference between the two.
    #[test]
    fn the_length_bound_is_checked_before_a_single_character_is_decoded() {
        let past_bound = format!("{}!!!!", "A".repeat(MAX_SECRET_LEN));
        assert_eq!(
            base32_decode(&past_bound),
            Err(TotpError::TooLong {
                len: MAX_SECRET_LEN + 4
            }),
            "the length is refused first; a decode would have reported the '!'"
        );

        // At the bound it decodes, so the guard is `>` and not `>=`: the
        // constant is a length this module accepts, not one it refuses.
        let at_bound = "A".repeat(MAX_SECRET_LEN);
        let key = base32_decode(&at_bound).expect("exactly at the bound is legal");
        assert_eq!(
            key.len(),
            MAX_SECRET_LEN * 5 / 8,
            "128 base32 characters are 80 key bytes"
        );
    }

    /// **NO PATH OUTSIDE THIS MODULE COMPUTES A CODE.** D-0956.
    ///
    /// Two comments said otherwise: `pull::vendor`'s Groww `Auth` row said the
    /// vendor's `api-key` "is spent by `pull::totp` to MINT the daily token",
    /// and CI's group-18 note said this module exists "so a stale broker token
    /// can be RECOGNISED". Neither was ever wired. `CLAUDE.md` §8 forbids the
    /// first outright, and a stale token is re-read from Parameter Store, not
    /// recognised by a code. This walks every `.rs` file under `crates/*/src`
    /// and fails on any that names this module's two code functions or
    /// glob-imports it, so the day something trades a code for a token it has
    /// to delete this test, in the open, to do it.
    ///
    /// Strict on purpose: it does not try to tell a `#[cfg(test)]` module from
    /// production code. A test elsewhere that needs a code computes it here.
    #[test]
    fn no_path_outside_this_module_computes_a_code() {
        use std::path::{Path, PathBuf};

        // Upper-case and lowered at run time so this file adds no lower-case
        // segment-shaped literal to `crates/pull` (CI gate 1d).
        let needle = "CODE_AT".to_ascii_lowercase();
        let glob = "TOTP::*".to_ascii_lowercase();
        let source = "SRC".to_ascii_lowercase();

        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let crates = manifest.parent().expect("crates/pull sits under crates/");
        let workspace = crates
            .parent()
            .expect("crates/ sits under the workspace root");
        let this = workspace.join(file!());
        assert!(this.is_file(), "file!() resolves from the workspace root");

        let mut stack: Vec<PathBuf> = std::fs::read_dir(crates)
            .expect("crates/ is readable")
            .map(|entry| entry.expect("a directory entry").path().join(&source))
            .filter(|dir| dir.is_dir())
            .collect();
        let mut scanned = 0_usize;
        let mut offenders = Vec::new();
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("a source directory is readable") {
                let path = entry.expect("a directory entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if !path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("RS"))
                {
                    continue;
                }
                let text = std::fs::read_to_string(&path).expect("a source file is UTF-8");
                if path == this {
                    // The module's own text is the control: it must contain the
                    // needle, or the needle is wrong and the walk proves nothing.
                    assert!(
                        text.contains(&needle),
                        "the needle names this module's functions"
                    );
                    continue;
                }
                scanned += 1;
                if text.contains(&needle) || text.contains(&glob) {
                    offenders.push(path.display().to_string());
                }
            }
        }
        // Not vacuous: thirteen crates' sources were read, not an empty walk.
        assert!(scanned > 100, "only {scanned} source files were read");
        assert!(
            offenders.is_empty(),
            "a TOTP code is computed outside pull::totp — CLAUDE.md section 8 \
             forbids minting a token here, and nothing else needs one: {offenders:?}"
        );
    }
}
