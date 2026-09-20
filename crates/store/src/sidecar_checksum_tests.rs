#![cfg(test)]
//! Generated record families must never overwrite one another's integrity proof.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::*;
use crate::format::{Greek, OI_NULL, Overlay, RATE_FROM_OPERATOR, VOL_FROM_VENDOR};
use crate::path::{PathParts, Timeframe, YearMonth};
use brutex_core::vendor::Vendor;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-sidecar-isolation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("exclusively owned scratch directory");
        Self(root)
    }

    fn path(kind: FileKind) -> StorePath<'static> {
        StorePath::new(PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: "FNO",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2025, 7).expect("generated month"),
            file: kind,
        })
        .expect("canonical generated path")
    }

    fn write<W: Row>(&self, kind: FileKind, rows: &[W], replay: bool) {
        let mut file = BarFile::open_or_create(&self.0, Self::path(kind), 13)
            .expect("open generated record family");
        let count = u64::try_from(rows.len()).expect("small fixture");
        let expected = if replay {
            Appended::AlreadyPresent {
                first_index: 0,
                n_valid: count,
            }
        } else {
            Appended::Committed {
                first_index: 0,
                n_valid: count,
            }
        };
        assert_eq!(file.append(rows), Ok(expected), "family {kind:?}");
    }

    fn check<W: Row + std::fmt::Debug>(&self, kind: FileKind, rows: &[W]) {
        let reader = BarFile::open_existing(&self.0, Self::path(kind), 13)
            .expect("read the independently sealed family");
        for (index, expected) in rows.iter().enumerate() {
            assert_eq!(
                reader.read_row::<W>(u64::try_from(index).expect("small index")),
                Ok(*expected),
                "family {kind:?}, row {index}"
            );
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn rows() -> (Vec<Bar>, Vec<Overlay>, Vec<Greek>) {
    let bars: Vec<_> = (0..180)
        .map(|index| Bar {
            ts_micros: 1_751_341_500_000_000 + index * 60_000_000,
            open: 10_000,
            high: 11_000,
            low: 9_000,
            close: 10_500,
            volume: index,
            open_interest: OI_NULL,
        })
        .collect();
    let overlays = bars
        .iter()
        .map(|bar| Overlay {
            ts_micros: bar.ts_micros,
            spot: 2_400_000,
            iv_micros: 250_000,
        })
        .collect();
    let greeks = bars
        .iter()
        .map(|bar| Greek {
            ts_micros: bar.ts_micros,
            spot: 2_400_000,
            volatility: 0.25,
            delta: 0.5,
            gamma: 0.125,
            vega: 1.0,
            theta: -1.0,
            rho: 2.0,
            rate: 0.0,
            provenance: Greek::provenance_of(VOL_FROM_VENDOR, RATE_FROM_OPERATOR, false, 0),
        })
        .collect();
    (bars, overlays, greeks)
}

#[test]
fn record_families_keep_independent_checksums_in_every_write_order_and_exact_replay() {
    use FileKind::{Bars, Greeks, Overlay};
    let _sink = crate::emits::hold_the_sink();
    let (bars, overlays, greeks) = rows();
    for order in [
        [Bars, Overlay, Greeks],
        [Bars, Greeks, Overlay],
        [Overlay, Bars, Greeks],
        [Overlay, Greeks, Bars],
        [Greeks, Bars, Overlay],
        [Greeks, Overlay, Bars],
    ] {
        let fixture = Fixture::new();
        for kind in order {
            match kind {
                Bars => fixture.write(kind, &bars, false),
                Overlay => fixture.write(kind, &overlays, false),
                other => {
                    assert_eq!(other, Greeks);
                    fixture.write(other, &greeks, false);
                }
            }
        }
        fixture.check(Bars, &bars);
        fixture.check(Overlay, &overlays);
        fixture.check(Greeks, &greeks);
        let before: Vec<_> = FileKind::ALL
            .into_iter()
            .map(|kind| {
                let path = Fixture::path(kind).to_path_buf(&fixture.0);
                (path.clone(), fs::read(path).expect("all sibling bytes"))
            })
            .collect();
        fixture.write(Bars, &bars, true);
        fixture.write(Overlay, &overlays, true);
        fixture.write(Greeks, &greeks, true);
        for (path, original) in before {
            assert_eq!(fs::read(path).expect("after replay"), original);
        }
    }
}

#[test]
fn corrupting_one_integrity_sibling_refuses_only_its_record_family() {
    let _sink = crate::emits::hold_the_sink();
    let fixture = Fixture::new();
    let (bars, overlays, greeks) = rows();
    fixture.write(FileKind::Bars, &bars, false);
    fixture.write(FileKind::Overlay, &overlays, false);
    fixture.write(FileKind::Greeks, &greeks, false);
    let bar_bytes = fs::read(Fixture::path(FileKind::Bars).to_path_buf(&fixture.0)).expect("bars");
    for (data, checksum) in [
        (FileKind::Bars, FileKind::Checksums),
        (FileKind::Overlay, FileKind::OverlayChecksums),
        (FileKind::Greeks, FileKind::GreekChecksums),
    ] {
        let path = Fixture::path(checksum).to_path_buf(&fixture.0);
        let original = fs::read(&path).expect("owned integrity bytes");
        fs::write(&path, 0u32.to_le_bytes()).expect("damage only the owned checksum");
        let reader = BarFile::open_existing(&fixture.0, Fixture::path(data), 13).expect("header");
        let refusal = match data {
            FileKind::Bars => reader.read_row::<Bar>(0).map(|_| ()),
            FileKind::Overlay => reader.read_row::<Overlay>(0).map(|_| ()),
            other => {
                assert_eq!(other, FileKind::Greeks);
                reader.read_row::<Greek>(0).map(|_| ())
            }
        };
        assert!(
            matches!(refusal, Err(StoreError::BlockChecksum { block: 0, .. })),
            "{refusal:?}"
        );
        drop(reader);
        if data != FileKind::Bars {
            fixture.check(FileKind::Bars, &bars);
        }
        if data != FileKind::Overlay {
            fixture.check(FileKind::Overlay, &overlays);
        }
        if data != FileKind::Greeks {
            fixture.check(FileKind::Greeks, &greeks);
        }
        assert_eq!(fs::read(&path).expect("no repair"), 0u32.to_le_bytes());
        fs::write(&path, original).expect("restore exact owned bytes");
    }
    fixture.check(FileKind::Bars, &bars);
    fixture.check(FileKind::Overlay, &overlays);
    fixture.check(FileKind::Greeks, &greeks);
    assert_eq!(
        fs::read(Fixture::path(FileKind::Bars).to_path_buf(&fixture.0)).expect("unchanged bars"),
        bar_bytes
    );
}

#[test]
fn a_missing_derived_checksum_never_borrows_or_rewrites_the_bar_checksum() {
    let _sink = crate::emits::hold_the_sink();
    let fixture = Fixture::new();
    let (bars, overlays, greeks) = rows();
    fixture.write(FileKind::Bars, &bars, false);
    fixture.write(FileKind::Overlay, &overlays, false);
    fixture.write(FileKind::Greeks, &greeks, false);
    let bar_crc = Fixture::path(FileKind::Checksums).to_path_buf(&fixture.0);
    let original = fs::read(&bar_crc).expect("independent bar proof");
    for (data, checksum) in [
        (FileKind::Overlay, FileKind::OverlayChecksums),
        (FileKind::Greeks, FileKind::GreekChecksums),
    ] {
        let sidecar = Fixture::path(checksum).to_path_buf(&fixture.0);
        let data_path = Fixture::path(data).to_path_buf(&fixture.0);
        let data_bytes = fs::read(&data_path).expect("unchanged derived records");
        fs::remove_file(&sidecar).expect("remove only the owned derived proof");
        let reader = BarFile::open_existing(&fixture.0, Fixture::path(data), 13)
            .expect("header still reads");
        let refusal = if data == FileKind::Overlay {
            reader.read_row::<Overlay>(0).map(|_| ())
        } else {
            reader.read_row::<Greek>(0).map(|_| ())
        };
        assert_eq!(
            refusal,
            Err(StoreError::ChecksumsMissing {
                path: data_path.clone(),
                sidecar: sidecar.clone(),
                block: 0
            })
        );
        drop(reader);
        let refused = BarFile::open_or_create(&fixture.0, Fixture::path(data), 13)
            .expect_err("an existing sealed stream cannot manufacture its missing proof");
        assert_eq!(
            refused,
            StoreError::Missing {
                path: sidecar.clone(),
                action: Action::Open,
            }
        );
        assert!(
            !sidecar.exists(),
            "neither a reader nor a writer can fabricate integrity evidence"
        );
        assert_eq!(
            fs::read(data_path).expect("retained derived bytes"),
            data_bytes
        );
        assert_eq!(fs::read(&bar_crc).expect("retained bar proof"), original);
        fixture.check(FileKind::Bars, &bars);
    }
}
