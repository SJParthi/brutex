//! `gen-served` (S1, recheck R9 / plan s10 (8)): prints, through the generation-aware store door the host reads with
//! (`store::file::BarFile::open_existing`, the same door as feat_io's `zstore::read_month`), the bars and sessions SERVED for
//! Zerodha CASH ADANIENT 1min and Zerodha INDEX NIFTY / BANKNIFTY / INDIAVIX 1min in the four front-hole months
//! (2020-12, 2021-12, 2022-12, 2024-11), and whether each month is served from a promoted revision. Read only.
//!
//! `gen-served --store ROOT`

use std::collections::BTreeSet;
use std::path::PathBuf;

use brutex_core::vendor::Vendor;
use idx_cal::day;
use idx_cal::storeio::{self, MonthFile};
use store::path::Timeframe;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let Some(root) = a.iter().position(|x| x == "--store").and_then(|i| a.get(i + 1)).map(PathBuf::from) else {
        eprintln!("usage: gen-served --store ROOT");
        std::process::exit(2);
    };
    println!("segment\tsymbol\tmonth\tserved_bars\tsessions\tfirst_session\tlast_session\tgeneration_pointer");
    for (seg, sym) in [("CASH", "ADANIENT"), ("INDEX", "NIFTY"), ("INDEX", "BANKNIFTY"), ("INDEX", "INDIAVIX")] {
        for (y, m) in [(2020u16, 12u8), (2021, 12), (2022, 12), (2024, 11)] {
            let gp = root.join("bar-generations-v1").join("bars").join("zerodha").join("NSE").join(seg).join(sym).join("1min").join(format!("{y:04}-{m:02}.gen-v1"));
            let ptr = if gp.is_file() { "yes" } else { "no" };
            match storeio::open_month(&root, Vendor::Zerodha, seg, sym, None, Timeframe::MINUTE_1, y, m) {
                MonthFile::Open(f) => {
                    let mut days = BTreeSet::new();
                    let n = f.records();
                    for i in 0..n {
                        if let Ok(b) = f.read_record(i) {
                            days.insert(storeio::ist_of(b.ts_micros).0);
                        }
                    }
                    let fmt = |d: Option<&i32>| d.map_or_else(|| "-".to_string(), |d| day::fmt(*d));
                    println!("{seg}\t{sym}\t{y:04}-{m:02}\t{n}\t{}\t{}\t{}\t{ptr}", days.len(), fmt(days.iter().next()), fmt(days.iter().next_back()));
                }
                MonthFile::Missing => println!("{seg}\t{sym}\t{y:04}-{m:02}\tmissing\t-\t-\t-\t{ptr}"),
                MonthFile::Refused(e) => println!("{seg}\t{sym}\t{y:04}-{m:02}\trefused: {e}\t-\t-\t-\t{ptr}"),
            }
        }
    }
}
