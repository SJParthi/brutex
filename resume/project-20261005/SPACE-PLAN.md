# WD_BLACK: plan to reach 4 TB free

Surveyed 2026-10-04 (read-only, nothing moved or deleted). Sizes are allocated
bytes from `du -sk` unless marked as manifest bytes.

## Where the 6.9 TB goes

| What | Size | Notes |
|---|---|---|
| Extracted options CSVs `NSE_Options_Tick/` | **3.46 TB** | 2018 79 GB, 2019 258, 2020 344, 2021 387, 2022 441, 2023 480, 2024 531, 2025 521, 2026 419 |
| Extracted stocks + indices CSVs `NSE_Tick_.../` | **1.22 TB** | STOCKS 0.80 TB (4.89 M files), INDICES 0.42 TB (161 K files) |
| Options year zips `Options/20xx.zip` | 0.46 TB | Nested: each holds per-day zips (stored, not recompressed) |
| Stocks+indices zip | 0.13 TB | 4,209 inner day zips |
| `tools-work/options-nested-proof` | **0.45 TB** | 1,997 per-day zips unpacked from the year zips: a third copy |
| `tools-work/options-queue` partial downloads | **0.08 TB** | 6 abandoned `.zip.part` files. A further 418 GB there are hard links to `Options/*.zip`, so they use no extra space |
| `work-20260925/tgt` (Rust build output) | 0.42 TB | Rebuildable, but parts may belong to the running GDFL build |
| `verification-20260919` | 0.09 TB | |
| Free | 0.35 TB | Was 692 GiB on 1 Oct, so ~340 GB was used in 3 days |

The data exists three times: as zips, as extracted CSVs (7.5x the zip size:
options 2026 is 55 GB zipped, 413 GB of manifest bytes extracted), and as the
unpacked day zips in options-nested-proof.

The engine's planned GDFL grid tables (design section 4) keep only 1-second
OHLC for two indices, so they cannot serve as the lossless copy needed before
anything is removed.

## Step 1: build the compressed tick store (Rust, read-only on all existing data)

- Read straight from the zips: year zip → inner day zip → CSV. No extracted folder is read or needed.
- One file per market per trading day. Inside it, a fixed table of instruments
  points to one zstd-compressed columnar block per instrument (Ticker, Date,
  Time, LTP, Buy/Sell price and qty, LTQ, OpenInterest), so finding any
  instrument-day is one table lookup plus one block read.
- Lossless by construction: any row whose text would not regenerate
  byte-for-byte is kept verbatim.
- Code goes in the local tools repo (never pushed). No GDFL data goes to GitHub.
- Estimated size: one options day (2.22 GB, 11,524 files) compresses to 167 MB
  with plain zstd -19 (7.5%). At that ratio, all 4.68 TB comes to about 350 GB,
  and columnar encoding should come in smaller. I'll measure on one month before the full run.

## Step 2: proof, before anything is removed

1. Regenerate every one of the ~25.9 M CSVs from the store and match its size
   and CRC-32 against the existing extraction manifests and the zip entries.
   Zero mismatches are required.
2. Match row counts against the earlier full scans: 49.8 B option rows and 18.9 B stock+index rows.
3. Run a second, independent reader to diff a random sample row-by-row against the extracted CSVs.
4. Publish a report with the counts, each mismatch (must be 0) and the store's checksums.

## Step 3: web page

A local page served by the Rust tool: pick market, date and instrument, and
read every tick. Finding a row is O(1): the day file is found by date and the
instrument block by table offset. Then one block is decompressed.

## Step 4: free space (each item needs your explicit yes)

| Order | Remove | Frees | Safe because |
|---|---|---|---|
| A | options-queue partials + options-nested-proof | 0.53 TB | Pure duplicates of the year zips. Safe even before Step 1 |
| B | Extracted CSV folders | 4.68 TB | Only after Step 2 proves every file regenerates exactly |
| C | Zips | 0.59 TB | Optional: move to a drive or cloud you name, or keep them here |

After A and B (minus ~0.35 TB for the new store), free space is about **5.2 TB**
with the original zips still on the drive. So the 4 TB target is met without
moving anything to the cloud.
