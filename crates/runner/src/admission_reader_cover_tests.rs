#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "tests state their expectations"
)]
//! BASE-COV-LIBS: every arm of the three canonical admission readers, driven by
//! a direct call with the bytes that reach it.
//!
//! The record decoders hand each reader a buffer whose length `new` already
//! checked, so some refusals (`take` past the end, an offset whose end
//! overflows `usize`, a header too short for its version or payload length)
//! are reached here by calling the reader itself. Each case asserts the exact
//! refusal, its record, offset and figures.
//!
//! Extreme cases: an empty buffer; a one-byte buffer; an offset at
//! `usize::MAX`; every tag value 0..=255 for each enum reader; a reserved
//! payload whose ONLY nonzero byte is its last; a header wrong in every field
//! at once (the first check named); a header both too short and with a wrong
//! magic (length named first); and, out of the box, a buffer that is a valid
//! header for ANOTHER version (refused as a version, never decoded).

use super::{
    ADMISSION_VERSION_V1, ADMISSION_VERSION_V2, ADMISSION_VERSION_V3, AdmissionCanonicalRecordV1,
    AdmissionCanonicalRecordV2, AdmissionCanonicalRecordV3, AdmissionCanonicalRefusalV1,
    AdmissionCanonicalRefusalV2, AdmissionCanonicalRefusalV3, CANONICAL_HEADER_LEN,
    CanonicalReader, CanonicalReaderV2, CanonicalReaderV3, CompletenessV1, DECISION_DOMAIN_V1,
    EVIDENCE_DOMAIN_V1, HypothesisDecisionV1, ObservedI64V1, ObservedU64V1, POLICY_DOMAIN_V1,
};

/// A header followed by `payload`, exactly as the writers lay it out.
fn record(domain: u8, version: u16, payload: &[u8]) -> Vec<u8> {
    let mut bytes = b"BADM".to_vec();
    bytes.push(domain);
    bytes.push(0);
    bytes.extend_from_slice(&version.to_le_bytes());
    let len = u32::try_from(payload.len()).expect("a small payload");
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

/// An observed value: one tag byte and eight payload bytes.
fn observed(tag: u8, payload: [u8; 8]) -> Vec<u8> {
    let mut bytes = vec![tag];
    bytes.extend_from_slice(&payload);
    bytes
}

const R1: AdmissionCanonicalRecordV1 = AdmissionCanonicalRecordV1::Evidence;
const R2: AdmissionCanonicalRecordV2 = AdmissionCanonicalRecordV2::Evidence;
const R3: AdmissionCanonicalRecordV3 = AdmissionCanonicalRecordV3::Decision;

fn v1(payload: &[u8]) -> Vec<u8> {
    record(EVIDENCE_DOMAIN_V1, ADMISSION_VERSION_V1, payload)
}

fn open_v1(bytes: &[u8]) -> CanonicalReader<'_> {
    let len = u32::try_from(bytes.len() - CANONICAL_HEADER_LEN).unwrap();
    CanonicalReader::new(bytes, R1, bytes.len(), EVIDENCE_DOMAIN_V1, len).unwrap()
}

#[test]
fn a_v1_header_is_refused_by_its_first_wrong_field() {
    let good = v1(&[0; 3]);
    let open = |bytes: &[u8], len: usize, domain: u8, payload: u32| {
        CanonicalReader::new(bytes, R1, len, domain, payload).err()
    };
    assert_eq!(open(&good, good.len(), EVIDENCE_DOMAIN_V1, 3), None);
    assert_eq!(
        open(&good, good.len() + 1, EVIDENCE_DOMAIN_V1, 3),
        Some(AdmissionCanonicalRefusalV1::Length {
            record: R1,
            expected: good.len() + 1,
            actual: good.len(),
        })
    );
    // Empty and one-byte buffers that claim to be complete: the magic read
    // itself runs out, so the refusal is a length at the read's end.
    for short in [&[][..], &[b'B'][..], &good[..11]] {
        assert_eq!(
            open(short, short.len(), EVIDENCE_DOMAIN_V1, 3),
            Some(if short.len() < 4 {
                AdmissionCanonicalRefusalV1::Length {
                    record: R1,
                    expected: 4,
                    actual: short.len(),
                }
            } else {
                AdmissionCanonicalRefusalV1::Length {
                    record: R1,
                    expected: 12,
                    actual: 11,
                }
            }),
            "{short:?}"
        );
    }
    let mut magic = good.clone();
    magic[0] = b'b';
    assert_eq!(
        open(&magic, magic.len(), EVIDENCE_DOMAIN_V1, 3),
        Some(AdmissionCanonicalRefusalV1::Magic { record: R1 })
    );
    assert_eq!(
        open(&good, good.len(), POLICY_DOMAIN_V1, 3),
        Some(AdmissionCanonicalRefusalV1::Domain {
            record: R1,
            actual: EVIDENCE_DOMAIN_V1,
        })
    );
    let mut reserved = good.clone();
    reserved[5] = 9;
    assert_eq!(
        open(&reserved, reserved.len(), EVIDENCE_DOMAIN_V1, 3),
        Some(AdmissionCanonicalRefusalV1::Reserved {
            record: R1,
            offset: 5,
        })
    );
    // Out of the box: a well-formed V2 header is a version, never decoded.
    let other = record(EVIDENCE_DOMAIN_V1, ADMISSION_VERSION_V2, &[0; 3]);
    assert_eq!(
        open(&other, other.len(), EVIDENCE_DOMAIN_V1, 3),
        Some(AdmissionCanonicalRefusalV1::Version {
            record: R1,
            actual: ADMISSION_VERSION_V2,
        })
    );
    assert_eq!(
        open(&good, good.len(), EVIDENCE_DOMAIN_V1, 4),
        Some(AdmissionCanonicalRefusalV1::PayloadLength {
            record: R1,
            actual: 3,
        })
    );
    // Wrong in every field at once: the magic is named, the first check.
    let mut all = good;
    all[0] = 0;
    all[4] = 0;
    all[5] = 1;
    all[6] = 7;
    assert_eq!(
        open(&all, all.len(), POLICY_DOMAIN_V1, 0),
        Some(AdmissionCanonicalRefusalV1::Magic { record: R1 })
    );
}

#[test]
fn a_v1_read_past_its_buffer_or_its_address_space_is_a_length() {
    let bytes = v1(&[1, 2]);
    let mut reader = open_v1(&bytes);
    assert_eq!(
        reader.read_u32(),
        Err(AdmissionCanonicalRefusalV1::Length {
            record: R1,
            expected: CANONICAL_HEADER_LEN + 4,
            actual: bytes.len(),
        })
    );
    assert_eq!(reader.read_u16(), Ok(0x0201));
    assert_eq!(
        reader.read_u8(),
        Err(AdmissionCanonicalRefusalV1::Length {
            record: R1,
            expected: bytes.len() + 1,
            actual: bytes.len(),
        })
    );
    reader.finish();
    let mut far = CanonicalReader {
        bytes: &bytes,
        offset: usize::MAX,
        record: R1,
    };
    assert_eq!(
        far.read_u64(),
        Err(AdmissionCanonicalRefusalV1::Length {
            record: R1,
            expected: usize::MAX,
            actual: bytes.len(),
        })
    );
    assert_eq!(far.offset, usize::MAX, "a refused read never moves");
}

#[test]
fn a_v1_bool_and_every_v1_tag_answer_only_their_domain() {
    for tag in 0..=u8::MAX {
        let bytes = v1(&[tag]);
        let at = CANONICAL_HEADER_LEN;
        let refused = AdmissionCanonicalRefusalV1::Tag {
            record: R1,
            offset: at,
            actual: tag,
        };
        let bool_answer = open_v1(&bytes).read_bool();
        let completeness = open_v1(&bytes).read_completeness();
        let decision = open_v1(&bytes).read_hypothesis_decision();
        match tag {
            0 => {
                assert_eq!(bool_answer, Ok(false));
                assert_eq!(completeness, Ok(CompletenessV1::Complete));
                assert_eq!(decision, Ok(HypothesisDecisionV1::RejectedNull));
            }
            1 => {
                assert_eq!(bool_answer, Ok(true));
                assert_eq!(completeness, Ok(CompletenessV1::Incomplete));
                assert_eq!(decision, Ok(HypothesisDecisionV1::DidNotReject));
            }
            2 | 3 => {
                assert_eq!(bool_answer, Err(refused));
                let (c, d) = if tag == 2 {
                    (CompletenessV1::Unmeasured, HypothesisDecisionV1::Unmeasured)
                } else {
                    (CompletenessV1::Refused, HypothesisDecisionV1::Refused)
                };
                assert_eq!(completeness, Ok(c));
                assert_eq!(decision, Ok(d));
            }
            _ => {
                assert_eq!(bool_answer, Err(refused));
                assert_eq!(completeness, Err(refused));
                assert_eq!(decision, Err(refused));
            }
        }
    }
}

#[test]
fn a_v1_observed_value_keeps_its_payload_zero_unless_it_is_measured() {
    let at = CANONICAL_HEADER_LEN;
    let last_only = [0, 0, 0, 0, 0, 0, 0, 1];
    let minus = (-5_i64).to_le_bytes();
    for (tag, payload) in [
        (0, minus),
        (1, [0; 8]),
        (2, [0; 8]),
        (1, last_only),
        (2, last_only),
        (3, [0; 8]),
        (255, minus),
    ] {
        let bytes = v1(&observed(tag, payload));
        let u = open_v1(&bytes).read_observed_u64();
        let i = open_v1(&bytes).read_observed_i64();
        match (tag, payload == [0; 8]) {
            (0, _) => {
                assert_eq!(u, Ok(ObservedU64V1::Measured(u64::from_le_bytes(minus))));
                assert_eq!(i, Ok(ObservedI64V1::Measured(-5)));
            }
            (1, true) => {
                assert_eq!(u, Ok(ObservedU64V1::Unmeasured));
                assert_eq!(i, Ok(ObservedI64V1::Unmeasured));
            }
            (2, true) => {
                assert_eq!(u, Ok(ObservedU64V1::Refused));
                assert_eq!(i, Ok(ObservedI64V1::Refused));
            }
            (1 | 2, false) => {
                let reserved = AdmissionCanonicalRefusalV1::Reserved {
                    record: R1,
                    offset: at + 1,
                };
                assert_eq!(u, Err(reserved));
                assert_eq!(i, Err(reserved));
            }
            _ => {
                let refused = AdmissionCanonicalRefusalV1::Tag {
                    record: R1,
                    offset: at,
                    actual: tag,
                };
                assert_eq!(u, Err(refused));
                assert_eq!(i, Err(refused));
            }
        }
    }
    // A tag with no payload behind it: the payload read is the refusal.
    let cut = v1(&[0, 1, 2]);
    assert!(matches!(
        open_v1(&cut).read_observed_u64(),
        Err(AdmissionCanonicalRefusalV1::Length { expected, .. }) if expected == at + 9
    ));
    assert!(matches!(
        open_v1(&cut).read_observed_i64(),
        Err(AdmissionCanonicalRefusalV1::Length { expected, .. }) if expected == at + 9
    ));
}

/// The V2 and V3 readers are the same shape with their own record and refusal
/// types: one body of tests, written once, instantiated for each.
macro_rules! later_reader_tests {
    ($module:ident, $reader:ident, $refusal:ident, $record:expr, $version:expr, $other:expr) => {
        mod $module {
            use super::*;

            fn bytes(payload: &[u8]) -> Vec<u8> {
                record(DECISION_DOMAIN_V1, $version, payload)
            }

            fn open(bytes: &[u8]) -> $reader<'_> {
                let len = u32::try_from(bytes.len() - CANONICAL_HEADER_LEN).unwrap();
                $reader::new(bytes, $record, bytes.len(), DECISION_DOMAIN_V1, len).unwrap()
            }

            #[test]
            fn a_header_is_refused_by_its_first_wrong_field() {
                let good = bytes(&[0; 2]);
                let new = |b: &[u8], len: usize, domain: u8, payload: u32| {
                    $reader::new(b, $record, len, domain, payload).err()
                };
                assert_eq!(new(&good, good.len(), DECISION_DOMAIN_V1, 2), None);
                assert_eq!(
                    new(&good, 0, DECISION_DOMAIN_V1, 2),
                    Some($refusal::Length {
                        record: $record,
                        expected: 0,
                        actual: good.len(),
                    })
                );
                // Too short and a wrong magic at once: the length is named.
                assert_eq!(
                    new(b"X", 2, DECISION_DOMAIN_V1, 2),
                    Some($refusal::Length {
                        record: $record,
                        expected: 2,
                        actual: 1,
                    })
                );
                for short in [&[][..], &b"BAD"[..], &b"badm"[..]] {
                    assert_eq!(
                        new(short, short.len(), DECISION_DOMAIN_V1, 2),
                        Some($refusal::Magic { record: $record }),
                        "{short:?}"
                    );
                }
                // A buffer that ends after its magic has no domain byte: it
                // reads as zero, which no record's domain is.
                assert_eq!(
                    new(b"BADM", 4, DECISION_DOMAIN_V1, 2),
                    Some($refusal::Domain {
                        record: $record,
                        actual: 0,
                    })
                );
                assert_eq!(
                    new(&good, good.len(), EVIDENCE_DOMAIN_V1, 2),
                    Some($refusal::Domain {
                        record: $record,
                        actual: DECISION_DOMAIN_V1,
                    })
                );
                let mut reserved = good.clone();
                reserved[5] = 1;
                assert_eq!(
                    new(&reserved, reserved.len(), DECISION_DOMAIN_V1, 2),
                    Some($refusal::Reserved {
                        record: $record,
                        offset: 5,
                    })
                );
                // A header cut before its version reads version zero; one cut
                // before its payload length reads length zero.
                let cut = &good[..7];
                assert_eq!(
                    new(cut, cut.len(), DECISION_DOMAIN_V1, 2),
                    Some($refusal::Version {
                        record: $record,
                        actual: 0,
                    })
                );
                let cut = &good[..9];
                assert_eq!(
                    new(cut, cut.len(), DECISION_DOMAIN_V1, 2),
                    Some($refusal::PayloadLength {
                        record: $record,
                        actual: 0,
                    })
                );
                // Out of the box: another version's well-formed header.
                let other = record(DECISION_DOMAIN_V1, $other, &[0; 2]);
                assert_eq!(
                    new(&other, other.len(), DECISION_DOMAIN_V1, 2),
                    Some($refusal::Version {
                        record: $record,
                        actual: $other,
                    })
                );
                assert_eq!(
                    new(&good, good.len(), DECISION_DOMAIN_V1, 3),
                    Some($refusal::PayloadLength {
                        record: $record,
                        actual: 2,
                    })
                );
            }

            #[test]
            fn a_read_past_its_buffer_or_its_address_space_is_a_length() {
                let b = bytes(&[7, 0, 0, 0, 0, 0, 0, 0, 9]);
                let mut reader = open(&b);
                assert_eq!(reader.read_u64(), Ok(7));
                assert_eq!(
                    reader.read_i64(),
                    Err($refusal::Length {
                        record: $record,
                        expected: CANONICAL_HEADER_LEN + 16,
                        actual: b.len(),
                    })
                );
                assert_eq!(reader.read_u8(), Ok(9));
                reader.finish();
                let mut far = $reader {
                    bytes: &b,
                    offset: usize::MAX,
                    record: $record,
                    expected_len: b.len(),
                };
                assert_eq!(
                    far.read_u8(),
                    Err($refusal::Length {
                        record: $record,
                        expected: usize::MAX,
                        actual: b.len(),
                    })
                );
                let negative = bytes(&(-1_i64).to_le_bytes());
                assert_eq!(open(&negative).read_i64(), Ok(-1));
            }

            #[test]
            fn every_tag_answers_only_its_domain() {
                for tag in 0..=u8::MAX {
                    let b = bytes(&[tag]);
                    let refused = $refusal::Tag {
                        record: $record,
                        offset: CANONICAL_HEADER_LEN,
                        actual: tag,
                    };
                    let completeness = open(&b).read_completeness();
                    let decision = open(&b).read_hypothesis_decision();
                    let expected = match tag {
                        0 => Some((CompletenessV1::Complete, HypothesisDecisionV1::RejectedNull)),
                        1 => Some((CompletenessV1::Incomplete, HypothesisDecisionV1::DidNotReject)),
                        2 => Some((CompletenessV1::Unmeasured, HypothesisDecisionV1::Unmeasured)),
                        3 => Some((CompletenessV1::Refused, HypothesisDecisionV1::Refused)),
                        _ => None,
                    };
                    match expected {
                        Some((c, d)) => {
                            assert_eq!(completeness, Ok(c));
                            assert_eq!(decision, Ok(d));
                        }
                        None => {
                            assert_eq!(completeness, Err(refused));
                            assert_eq!(decision, Err(refused));
                        }
                    }
                }
                let empty = bytes(&[]);
                assert!(matches!(
                    open(&empty).read_completeness(),
                    Err($refusal::Length { .. })
                ));
                assert!(matches!(
                    open(&empty).read_hypothesis_decision(),
                    Err($refusal::Length { .. })
                ));
            }

            #[test]
            fn an_observed_value_names_its_first_nonzero_reserved_byte() {
                let at = CANONICAL_HEADER_LEN;
                for (tag, payload) in [
                    (0_u8, 42_u64.to_le_bytes()),
                    (1, [0; 8]),
                    (2, [0; 8]),
                    (1, [0, 0, 0, 0, 0, 0, 0, 1]),
                    (2, [0, 0, 3, 0, 0, 0, 0, 1]),
                    (4, [0; 8]),
                ] {
                    let b = bytes(&observed(tag, payload));
                    let u = open(&b).read_observed_u64();
                    let i = open(&b).read_observed_i64();
                    let first = payload.iter().position(|byte| *byte != 0);
                    match (tag, first) {
                        (0, _) => {
                            assert_eq!(u, Ok(ObservedU64V1::Measured(42)));
                            assert_eq!(i, Ok(ObservedI64V1::Measured(42)));
                        }
                        (1, None) => {
                            assert_eq!(u, Ok(ObservedU64V1::Unmeasured));
                            assert_eq!(i, Ok(ObservedI64V1::Unmeasured));
                        }
                        (2, None) => {
                            assert_eq!(u, Ok(ObservedU64V1::Refused));
                            assert_eq!(i, Ok(ObservedI64V1::Refused));
                        }
                        (1 | 2, Some(k)) => {
                            let reserved = $refusal::Reserved {
                                record: $record,
                                offset: at + 1 + k,
                            };
                            assert_eq!(u, Err(reserved));
                            assert_eq!(i, Err(reserved));
                        }
                        _ => {
                            let refused = $refusal::Tag {
                                record: $record,
                                offset: at,
                                actual: tag,
                            };
                            assert_eq!(u, Err(refused));
                            assert_eq!(i, Err(refused));
                        }
                    }
                }
                let cut = bytes(&[0, 1]);
                assert!(matches!(
                    open(&cut).read_observed_u64(),
                    Err($refusal::Length { expected, .. }) if expected == at + 9
                ));
                assert!(matches!(
                    open(&cut).read_observed_i64(),
                    Err($refusal::Length { expected, .. }) if expected == at + 9
                ));
                let lone = bytes(&[]);
                assert!(matches!(
                    open(&lone).read_observed_u64(),
                    Err($refusal::Length { expected, .. }) if expected == at + 1
                ));
                assert!(matches!(
                    open(&lone).read_observed_i64(),
                    Err($refusal::Length { expected, .. }) if expected == at + 1
                ));
            }
        }
    };
}

later_reader_tests!(
    v2,
    CanonicalReaderV2,
    AdmissionCanonicalRefusalV2,
    R2,
    ADMISSION_VERSION_V2,
    ADMISSION_VERSION_V3
);
later_reader_tests!(
    v3,
    CanonicalReaderV3,
    AdmissionCanonicalRefusalV3,
    R3,
    ADMISSION_VERSION_V3,
    ADMISSION_VERSION_V1
);
