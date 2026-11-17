//! v3.3 SEARCH-1D (MAIN-RULINGS-ENGINE-v22 Addendum 28.1; search:v33, 15 Nov 2026): THE NAMED TEST of the every-year search set's sets
//! file `data/EXITS-DYN-SETS-v33-1D.tsv` (= `state/spikefade-20261114/v33/sets/EXITS-DYN-SETS-v33-1D.tsv`, generated from
//! `EXITS-DYN-SETS-v1.tsv` by `v33/sets/tools/gen-sets-v33-1d.mjs`): the engine loads it, the new role SEARCH1D selects exactly the
//! designed settings, every other selector and row is unchanged, and the combos it covers per segment over 2018-10..2026-10 are exact and
//! at least 1,000,000,000 (pairs = the all-years (config, liquidity) pairs with a non-empty list, lists:v32 run sf3-20261115-204330-v32).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::menu::{FAMILIES, Kind, Menu, Sets};

/// The SEARCH-1D settings (role SEARCH1D).
const N1D: usize = 1_377;
/// SEARCH1D settings per tier T1..T7 (gen-sets-v33-1d.mjs: T1 stop alone, T2 stop + one rule, T3 stop + BE + TSL, T4 stop + GO + TIME,
/// T5 stop + TSL + TTP, T6 stop + TP + TIME, T7 stop + TSL + TP).
const TIERS: [usize; 7] = [6, 396, 360, 90, 525, 0, 0];
/// The SEARCH settings per tier (the whole of each tier in EXITS-DYN-SETS-v1.tsv).
const TIERS_ALL: [usize; 7] = [6, 396, 360, 90, 525, 252, 1_080];
/// (config, liquidity) pairs with a non-empty list over 2018-10-01..2026-10-30, INDEX and STOCK (LISTS.md of lists:v32 run
/// sf3-20261115-204330-v32: 212,822 of 230,400 and 184,268 of 230,400).
const PAIRS: [u128; 2] = [212_822, 184_268];
/// Shares walked (the 25% headline share only) and slot counts per segment (index 5/10/15/20, stock 5/10/20/40).
const SHARES: u128 = 1;
const NSLOT: u128 = 4;
/// COMBOS covered = pairs x settings x shares x N, exact.
const COMBOS: [u128; 2] = [1_172_223_576, 1_014_948_144];
/// sha256 of the sets file (the file is pinned: any edit fails this test).
const SHA: &str = "5ead880f47bfa5c04c5f4ae1275a799d209ccec811c288236c5def46fc54612b";

fn data(f: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join(f)
}

/// The tier of a setting from the families its rules play (the labels: SL 0, TT 1, BE 2, TSL 3, TTP 4, TP 5, GO 6, TIME 7).
fn tier(labels: &[u8]) -> usize {
    let f: BTreeSet<&str> = labels.iter().filter(|&&l| l != 0).filter_map(|&l| FAMILIES.get(usize::from(l)).copied()).collect();
    let k: Vec<&str> = f.into_iter().collect();
    match k.as_slice() {
        [] => 1,
        [_] => 2,
        ["BE", "TSL"] => 3,
        ["GO", "TIME"] => 4,
        ["TSL", "TTP"] => 5,
        ["TIME", "TP"] => 6,
        ["TP", "TSL"] => 7,
        _ => 0,
    }
}

/// v3.3 ADDENDUM 28.1, THE NAMED TEST: the SEARCH-1D sets file loads with the exact counts, SEARCH1D is a subset of SEARCH (R1 kept),
/// every other selector and every row equal to round B's v1 file, the s27 menu covered (every non-SL rule with every small stop), and the
/// combos per segment exact and >= 1,000,000,000 (and below it with one setting fewer than the rule's minimum count).
#[test]
fn v33_search1d_sets_load_and_count() {
    let menu = Menu::load(&data("EXITS-DYN-MENU-v1.tsv")).unwrap_or_else(|e| panic!("{e}"));
    let v1 = Sets::load(&data("EXITS-DYN-SETS-v1.tsv"), &menu).unwrap_or_else(|e| panic!("{e}"));
    let v33 = Sets::load(&data("EXITS-DYN-SETS-v33-1D.tsv"), &menu).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v33.sha, SHA, "the sets file is pinned");
    assert_eq!((v1.name.as_str(), v33.name.as_str()), ("EXITS-DYN-SETS-v1.tsv", "EXITS-DYN-SETS-v33-1D.tsv"), "RESULTS names the loaded file");
    assert_eq!((menu.len(), v1.settings.len(), v33.settings.len()), (95, 8_181, 8_181));

    // every row equal to v1's except the new role bit; v1 holds no SEARCH1D row
    for (a, b) in v1.settings.iter().zip(&v33.settings) {
        assert_eq!((&a.id, a.set, a.in_s0, a.search, &a.rules, &a.labels, a.sl, &a.key, a.v2, a.fams_on), (&b.id, b.set, b.in_s0, b.search, &b.rules, &b.labels, b.sl, &b.key, b.v2, b.fams_on));
        assert!(!a.s1d, "v1 has no SEARCH1D row");
        assert!(!b.s1d || b.search, "{}: SEARCH1D must be a SEARCH setting", b.id);
    }
    // every other selector selects exactly what it selects on v1 (SEARCH = SEARCH + SEARCH1D = the same 2,709, same order)
    for sel in ["S0", "S1", "S2", "S3", "S4", "S1+S3", "S1-S3", "S1-S4", "ALL", "SEARCH"] {
        assert_eq!(v1.select(sel), v33.select(sel), "{sel}");
    }
    assert_eq!(v33.select("SEARCH").map(|v| v.len()), Ok(2_709));
    assert_eq!(v1.select("SEARCH1D").map(|v| v.len()), Ok(0), "v1 selects no SEARCH1D setting");

    // SEARCH1D: exactly N1D settings, each a small stop (SL-P 2/3/5 or SL-A1 0.10/0.25/0.50: R1 / Q16 / REQ 47)
    let s1d = v33.select("SEARCH1D").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(s1d.len(), N1D);
    let mut per_tier = [0_usize; 7];
    let mut per_tier_all = [0_usize; 7];
    let mut per_stop: BTreeMap<String, usize> = BTreeMap::new();
    for (i, s) in v33.settings.iter().enumerate() {
        if !s.search {
            continue;
        }
        let t = tier(&s.labels);
        assert!((1..=7).contains(&t), "{}: no tier", s.id);
        per_tier_all[t - 1] += 1;
        if s1d.contains(&i) {
            per_tier[t - 1] += 1;
            let sl = &menu.rules[usize::from(s.sl)];
            assert!(sl.search && matches!((sl.kind, sl.p1), (Kind::SlP, 2 | 3 | 5) | (Kind::SlA1, 10 | 25 | 50)), "{}: not a small stop", s.id);
            *per_stop.entry(sl.id.clone()).or_default() += 1;
        }
    }
    assert_eq!(per_tier, TIERS);
    assert_eq!(per_tier_all, TIERS_ALL);
    let want_stop: BTreeMap<String, usize> = [("SL.P.2", 377), ("SL.P.3", 377), ("SL.P.5", 377), ("SL.A1.10", 82), ("SL.A1.25", 82), ("SL.A1.50", 82)].into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
    assert_eq!(per_stop, want_stop);
    // the s27 menu as built is covered: every non-SL menu rule appears with each of the 6 small stops (T2 whole: 6 x 66 = 396 pairs)
    let mut pairs_sl_rule: BTreeSet<(u16, u16)> = BTreeSet::new();
    for &i in &s1d {
        let s = &v33.settings[i];
        for &r in &s.rules {
            if r != s.sl {
                pairs_sl_rule.insert((s.sl, r));
            }
        }
    }
    let non_sl = menu.rules.iter().filter(|r| !r.kind.sl()).count();
    assert_eq!(non_sl, 66);
    let small: BTreeSet<u16> = s1d.iter().map(|&i| v33.settings[i].sl).collect();
    assert_eq!(small.len(), 6);
    for &sl in &small {
        assert_eq!(pairs_sl_rule.iter().filter(|p| p.0 == sl).count(), non_sl, "every rule with stop {}", menu.rules[usize::from(sl)].id);
    }

    // COMBOS covered per segment over 2018-10..2026-10: exact, >= 1,000,000,000; the rule's minimum count is the smallest that reaches it
    for seg in 0..2 {
        let c = PAIRS[seg] * N1D as u128 * SHARES * NSLOT;
        assert_eq!(c, COMBOS[seg]);
        assert!(c >= 1_000_000_000, "segment {seg}: {c} combos");
    }
    let min_pairs = PAIRS[0].min(PAIRS[1]);
    let s_min = 1_000_000_000_u128.div_ceil(min_pairs * SHARES * NSLOT);
    assert_eq!(s_min, 1_357);
    assert!(min_pairs * (s_min - 1) * SHARES * NSLOT < 1_000_000_000);
    assert!(N1D as u128 >= s_min && N1D as u128 - s_min == 20, "whole tiers: 20 settings above the minimum count");

    // fail closed (R1 for the new role): a SEARCH1D row on a COMPARISON stop, an unknown role
    let t = std::fs::read_to_string(data("EXITS-DYN-SETS-v33-1D.tsv")).unwrap_or_default();
    let bad = t.replacen("XD-SLP:10-GO0-BE0-TT0-TS0-TP0-TM0\tS1\t1\tCOMPARISON", "XD-SLP:10-GO0-BE0-TT0-TS0-TP0-TM0\tS1\t1\tSEARCH1D", 1);
    assert_ne!(bad, t);
    assert!(Sets::parse(&bad, &menu).err().is_some_and(|e| e.starts_with("role_mismatch")));
    let bad2 = t.replacen("\tSEARCH1D\t", "\tSEARCH2D\t", 1);
    assert!(Sets::parse(&bad2, &menu).err().is_some_and(|e| e.starts_with("role_mismatch")));
    assert!(v33.select("SEARCH2D").is_err());
}
