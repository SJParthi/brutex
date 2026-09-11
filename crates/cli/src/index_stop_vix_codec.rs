//! Fixed additive VIX stamp records, followed only by bounded original reasons.
use super::{
    Image, MAX_REASON_BYTES, Metadata, Month, Row, Setting, Stamp, address, count_stamps, display,
    lookup_identity, month_of, policy_digest,
};
use brutex_core::blake3::Hasher;
use brutex_core::vendor::Vendor;
use indicators::Candle;

const MAGIC: &[u8; 8] = b"BRISVX01";
const HEADER_BYTES: u64 = 200;
const SETTING_BYTES: u64 = 80;
const ROW_BYTES: u64 = 240;
const MONTH_BYTES: u64 = 72;

pub(super) fn maximum_bytes(settings: u64, rows: u64, months: u64) -> Result<u64, String> {
    size(settings, rows, months)?
        .checked_add(
            months
                .checked_mul(MAX_REASON_BYTES as u64)
                .ok_or("VIX refusal extent overflow")?,
        )
        .and_then(|n| n.checked_add(super::RECEIPT_BYTES))
        .ok_or_else(|| "VIX maximum encoded extent overflow".into())
}
fn size(settings: u64, rows: u64, months: u64) -> Result<u64, String> {
    HEADER_BYTES
        .checked_add(
            settings
                .checked_mul(SETTING_BYTES)
                .ok_or("VIX setting extent overflow")?,
        )
        .and_then(|n| n.checked_add(rows.checked_mul(ROW_BYTES)?))
        .and_then(|n| n.checked_add(months.checked_mul(MONTH_BYTES)?))
        .ok_or_else(|| "VIX encoded extent overflow".into())
}
fn publication(raw: &[u8]) -> Result<[u8; 32], String> {
    let mut digest = Hasher::new();
    digest.update(b"brutex-index-stop-vix-publication-v1\0");
    digest.update(raw.get(..40).ok_or("VIX publication prefix missing")?);
    digest.update(raw.get(72..).ok_or("VIX publication suffix missing")?);
    Ok(digest.finalize())
}

pub(super) fn encode(image: &Image, max_bytes: u64) -> Result<Vec<u8>, String> {
    let length = image.months.iter().try_fold(
        size(
            image.settings.len() as u64,
            image.rows.len() as u64,
            image.months.len() as u64,
        )?,
        |n, month| {
            n.checked_add(month.unavailable_reason.as_ref().map_or(0, String::len) as u64)
                .ok_or("VIX reason extent overflow")
        },
    )?;
    if length
        .checked_add(super::RECEIPT_BYTES)
        .is_none_or(|n| n > max_bytes)
    {
        return Err("complete VIX companion exceeds byte admission".into());
    }
    let mut out = Vec::new();
    out.try_reserve_exact(address(length)?).map_err(display)?;
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&image.meta.lookup_identity);
    out.extend_from_slice(&[0; 32]);
    out.extend_from_slice(&image.meta.catalog_identity);
    out.extend_from_slice(&image.meta.catalog_completion);
    out.extend_from_slice(&policy_digest());
    let feed = Vendor::ALL
        .iter()
        .position(|feed| feed.as_str() == image.meta.feed)
        .ok_or("VIX saved feed is not canonical")?;
    for word in [
        feed as u64,
        image.settings.len() as u64,
        image.rows.len() as u64,
        image.months.len() as u64,
    ] {
        put(&mut out, word);
    }
    for setting in &image.settings {
        out.extend_from_slice(&setting.run_id);
        out.extend_from_slice(&setting.source_id);
        put(&mut out, setting.first);
        put(&mut out, setting.count);
    }
    for month in &image.months {
        for word in [
            u64::from(month.year),
            u64::from(month.month),
            u64::from(month.records.is_some()),
            month.records.unwrap_or(0),
        ] {
            put(&mut out, word);
        }
        out.extend_from_slice(&month.snapshot_digest.unwrap_or([0; 32]));
        let reason = month.unavailable_reason.as_deref().unwrap_or("");
        put(&mut out, reason.len() as u64);
        out.extend_from_slice(reason.as_bytes());
    }
    for row in &image.rows {
        put(&mut out, row.trade_index);
        out.extend_from_slice(&row.run_id);
        out.extend_from_slice(&row.trade_digest);
        for value in [
            row.entry_micros,
            row.exit_bar_micros,
            row.exit_from_micros,
            row.exit_until_micros,
        ] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        put(&mut out, row.month_index);
        stamp(&mut out, row.entry);
        stamp(&mut out, row.exit);
    }
    if out.len() as u64 != length {
        return Err("VIX encoded length differs from complete admission".into());
    }
    let digest = publication(&out)?;
    out.get_mut(40..72)
        .ok_or("VIX publication field missing")?
        .copy_from_slice(&digest);
    Ok(out)
}
fn put(out: &mut Vec<u8>, word: u64) {
    out.extend_from_slice(&word.to_le_bytes());
}
fn stamp(out: &mut Vec<u8>, stamp: Stamp) {
    let (tag, values) = match stamp {
        Stamp::Exact(bar) => (
            1,
            [
                bar.ts_micros,
                bar.open,
                bar.high,
                bar.low,
                bar.close,
                bar.volume,
                bar.open_interest,
            ],
        ),
        Stamp::Absent => (0, [0; 7]),
        Stamp::Unavailable => (2, [0; 7]),
    };
    put(out, tag);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

pub(super) fn decode(
    raw: &[u8],
    catalog: [u8; 32],
    pin: [u8; 32],
    max_records: u64,
    validation_budget: u64,
) -> Result<Image, String> {
    let mut input = Input { raw, at: 0 };
    if input.bytes::<8>()? != *MAGIC {
        return Err("VIX companion version differs".into());
    }
    let lookup = input.bytes::<32>()?;
    let published = input.bytes::<32>()?;
    if lookup != lookup_identity(catalog, pin)
        || input.bytes::<32>()? != catalog
        || input.bytes::<32>()? != pin
        || input.bytes::<32>()? != policy_digest()
        || published != publication(raw)?
    {
        return Err("VIX companion catalog, policy or publication binding differs".into());
    }
    let feed = Vendor::ALL
        .get(address(input.word()?)?)
        .ok_or("VIX reference feed is not canonical")?;
    let settings = input.word()?;
    let rows = input.word()?;
    let months = input.word()?;
    if settings == 0
        || !settings.is_multiple_of(2)
        || settings > max_records
        || rows > max_records
        || months > rows
        || size(settings, rows, months)? > raw.len() as u64
    {
        return Err("VIX complete record counts exceed exact byte or record admission".into());
    }
    if validation_bytes(rows)? > validation_budget {
        return Err("VIX repeated-minute validation exceeds independent scratch admission".into());
    }
    let mut image = Image {
        meta: Metadata {
            catalog_identity: catalog,
            catalog_completion: pin,
            lookup_identity: lookup,
            publication_id: published,
            completion_digest: [0; 32],
            feed: feed.as_str().into(),
            settings,
            trades: rows,
            exact_stamps: 0,
            absent_stamps: 0,
            unavailable_stamps: 0,
        },
        settings: Vec::new(),
        rows: Vec::new(),
        months: Vec::new(),
    };
    image
        .settings
        .try_reserve_exact(address(settings)?)
        .map_err(display)?;
    image
        .rows
        .try_reserve_exact(address(rows)?)
        .map_err(display)?;
    image
        .months
        .try_reserve_exact(address(months)?)
        .map_err(display)?;
    read_settings(&mut input, &mut image, settings, rows)?;
    for _ in 0..months {
        image.months.push(month(&mut input)?);
    }
    for pair in image.months.windows(2) {
        let first = pair.first().ok_or("VIX month pair missing")?;
        let last = pair.get(1).ok_or("VIX month pair missing")?;
        if (first.year, first.month) >= (last.year, last.month) {
            return Err("VIX months are duplicated or reordered".into());
        }
    }
    for _ in 0..rows {
        let row = Row {
            trade_index: input.word()?,
            run_id: input.bytes()?,
            trade_digest: input.bytes()?,
            entry_micros: input.signed()?,
            exit_bar_micros: input.signed()?,
            exit_from_micros: input.signed()?,
            exit_until_micros: input.signed()?,
            month_index: input.word()?,
            entry: read_stamp(&mut input)?,
            exit: read_stamp(&mut input)?,
        };
        validate(&row, &image.months)?;
        image.rows.push(row);
    }
    if input.at != raw.len() {
        return Err("VIX companion has trailing or omitted bytes".into());
    }
    count_stamps(&mut image)?;
    validate_repeated(&image.rows, validation_budget)?;
    Ok(image)
}

struct Minute<'a> {
    month: u64,
    timestamp: i64,
    stamp: &'a Stamp,
}

pub(super) fn validation_bytes(rows: u64) -> Result<u64, String> {
    rows.checked_mul(2)
        .and_then(|n| n.checked_mul(std::mem::size_of::<Minute<'_>>() as u64))
        .ok_or_else(|| "VIX repeated-minute scratch size overflow".into())
}

fn validate_repeated(rows: &[Row], budget: u64) -> Result<(), String> {
    if validation_bytes(rows.len() as u64)? > budget {
        return Err("VIX repeated-minute validation exceeds independent scratch admission".into());
    }
    let count = rows
        .len()
        .checked_mul(2)
        .ok_or("VIX repeated-minute coordinate count overflow")?;
    let mut minutes = Vec::new();
    minutes.try_reserve_exact(count).map_err(display)?;
    for row in rows {
        minutes.push(Minute {
            month: row.month_index,
            timestamp: row.entry_micros,
            stamp: &row.entry,
        });
        minutes.push(Minute {
            month: row.month_index,
            timestamp: row.exit_bar_micros,
            stamp: &row.exit,
        });
    }
    // Deterministic O(T log T) cold work, O(T) admitted scratch. Sorting uses
    // borrowed whole stamps, so prices/states are never reduced to a scalar.
    minutes.sort_unstable_by_key(|minute| (minute.month, minute.timestamp));
    for pair in minutes.windows(2) {
        let first = pair.first().ok_or("VIX repeated-minute pair missing")?;
        let next = pair.get(1).ok_or("VIX repeated-minute pair missing")?;
        if (first.month, first.timestamp) == (next.month, next.timestamp)
            && first.stamp != next.stamp
        {
            return Err(
                "VIX repeated original minute carries contradictory complete stamps".into(),
            );
        }
    }
    Ok(())
}

fn read_settings(
    input: &mut Input<'_>,
    image: &mut Image,
    settings: u64,
    rows: u64,
) -> Result<(), String> {
    let mut through = 0_u64;
    for _ in 0..settings {
        let setting = Setting {
            run_id: input.bytes()?,
            source_id: input.bytes()?,
            first: input.word()?,
            count: input.word()?,
        };
        if setting.run_id == [0; 32] || setting.source_id == [0; 32] || setting.first != through {
            return Err("VIX setting identity or ordered extent differs".into());
        }
        through = through
            .checked_add(setting.count)
            .filter(|n| *n <= rows)
            .ok_or("VIX setting trade extent overflow")?;
        image.settings.push(setting);
    }
    if through != rows {
        return Err("VIX settings do not cover every trade".into());
    }
    Ok(())
}
fn month(input: &mut Input<'_>) -> Result<Month, String> {
    let year = u16::try_from(input.word()?).map_err(display)?;
    let month = u8::try_from(input.word()?).map_err(display)?;
    store::path::YearMonth::new(year, month).map_err(display)?;
    let present = input.word()?;
    let records = input.word()?;
    let snapshot = input.bytes::<32>()?;
    let reason = input.text()?;
    match present {
        0 if records == 0 && snapshot == [0; 32] && !reason.is_empty() => Ok(Month {
            year,
            month,
            records: None,
            snapshot_digest: None,
            unavailable_reason: Some(reason),
        }),
        1 if records <= crate::vix_reference::CIVIL_MONTH_MINUTE_SLOTS as u64
            && snapshot != [0; 32]
            && reason.is_empty() =>
        {
            Ok(Month {
                year,
                month,
                records: Some(records),
                snapshot_digest: Some(snapshot),
                unavailable_reason: None,
            })
        }
        _ => Err("VIX indexed/absent/unavailable month provenance is contradictory".into()),
    }
}
fn validate(row: &Row, months: &[Month]) -> Result<(), String> {
    let original = months
        .get(address(row.month_index)?)
        .ok_or("VIX row names a foreign month")?;
    let month = month_of(row.entry_micros)?;
    if (original.year, original.month) != (month.year(), month.month())
        || month != month_of(row.exit_bar_micros)?
        || row.entry_micros > row.exit_bar_micros
        || row.exit_from_micros < row.exit_bar_micros
        || row.exit_until_micros < row.exit_from_micros
        || row.run_id == [0; 32]
        || row.trade_digest == [0; 32]
    {
        return Err("VIX trade timing/month/native identity binding differs".into());
    }
    for (stamp, timestamp) in [
        (row.entry, row.entry_micros),
        (row.exit, row.exit_bar_micros),
    ] {
        if matches!(stamp, Stamp::Unavailable) != original.unavailable_reason.is_some() {
            return Err("VIX row hides an unavailable reference month".into());
        }
        if let Stamp::Exact(bar) = stamp {
            let stored = store::format::Bar {
                ts_micros: bar.ts_micros,
                open: bar.open,
                high: bar.high,
                low: bar.low,
                close: bar.close,
                volume: bar.volume,
                open_interest: bar.open_interest,
            };
            if original.records == Some(0)
                || bar.ts_micros != timestamp
                || !stored.ohlc_is_sane()
                || !stored.counts_are_sane()
            {
                return Err(
                    "VIX exact candle differs from its original minute or OHLCV bounds".into(),
                );
            }
        }
    }
    Ok(())
}
fn read_stamp(input: &mut Input<'_>) -> Result<Stamp, String> {
    let tag = input.word()?;
    let values = [
        input.signed()?,
        input.signed()?,
        input.signed()?,
        input.signed()?,
        input.signed()?,
        input.signed()?,
        input.signed()?,
    ];
    match (tag, values) {
        (0, [0, 0, 0, 0, 0, 0, 0]) => Ok(Stamp::Absent),
        (2, [0, 0, 0, 0, 0, 0, 0]) => Ok(Stamp::Unavailable),
        (1, [ts_micros, open, high, low, close, volume, open_interest]) => {
            Ok(Stamp::Exact(Candle {
                ts_micros,
                open,
                high,
                low,
                close,
                volume,
                open_interest,
            }))
        }
        _ => Err("VIX stamp tag or absent/unavailable reserved bytes differ".into()),
    }
}
struct Input<'a> {
    raw: &'a [u8],
    at: usize,
}
impl Input<'_> {
    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self.at.checked_add(N).ok_or("VIX field extent overflow")?;
        let value = self
            .raw
            .get(self.at..end)
            .ok_or("VIX record is incomplete")?
            .try_into()
            .map_err(display)?;
        self.at = end;
        Ok(value)
    }
    fn word(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.bytes()?))
    }
    fn signed(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.bytes()?))
    }
    fn text(&mut self) -> Result<String, String> {
        let count = address(self.word()?)?;
        if count > MAX_REASON_BYTES {
            return Err("VIX original diagnostic exceeds reason admission".into());
        }
        let end = self
            .at
            .checked_add(count)
            .ok_or("VIX diagnostic extent overflow")?;
        let value = std::str::from_utf8(
            self.raw
                .get(self.at..end)
                .ok_or("VIX original diagnostic is incomplete")?,
        )
        .map_err(display)?
        .to_owned();
        self.at = end;
        Ok(value)
    }
}
