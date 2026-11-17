//! v3.4-e10 build:shard-memory: the named tests of E10 (ship the set-up) with every MUST of v34/shard-mem/CRITIC.md. Each mutant of
//! Cargo.toml's v3.4-e10 block makes at least one of them FAIL (v34/shard-mem/build/MUTANTS-E10.txt).

use std::path::PathBuf;

use crate::acct::SEv;
use crate::e10::{self, DayIn, Flags, Own, Ship};
use crate::kit::{self, Plan, Sha256, sha256_bytes};
use crate::menu::Rd;
use crate::mtable::MarginAt;
use crate::report3::Seg3;
use crate::synth::Rng;
use crate::trade::Refuse;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sfe10-{name}-{}-{}", std::process::id(), kit::utc_now()));
    let _ = std::fs::create_dir_all(&d);
    d
}

const NC: usize = 40;
const REFUSE: [Refuse; 14] = [Refuse::None, Refuse::NoUnderlying, Refuse::NoMarginRow, Refuse::StockExpiryDay, Refuse::StockExpiryNoNext, Refuse::AfterCut, Refuse::BadTape, Refuse::NoReference, Refuse::NoMarginFile, Refuse::NoSpanRow, Refuse::NoElmRate, Refuse::FnoBan, Refuse::ChargesRate, Refuse::BadLot];

fn pick_i64(r: &mut Rng) -> i64 {
    match r.below(5) {
        0 => i64::MIN,
        1 => i64::MAX,
        2 => -i64::try_from(r.below(1 << 40)).unwrap(),
        _ => i64::try_from(r.below(1 << 40)).unwrap(),
    }
}

fn rd_of(r: &mut Rng) -> Rd {
    Rd {
        atr1: if r.below(3) == 0 { None } else { Some(pick_i64(r)) },
        atr5: if r.below(3) == 0 { None } else { Some(pick_i64(r)) },
        hwin: pick_i64(r),
        rise: pick_i64(r),
        v300: match r.below(3) {
            0 => None,
            1 => Some((u128::MAX, u64::MAX)),
            _ => Some((u128::from(r.next()) << 64 | u128::from(r.next()), r.next())),
        },
        theta: match r.below(4) {
            0 => Err("theta_none"),
            1 => Err("theta_no_calendar"),
            _ => Ok((pick_i64(r), pick_i64(r))),
        },
    }
}

fn mx_of(r: &mut Rng, k: usize) -> MarginAt {
    let mut m = MarginAt { lot_exact: pick_i64(r), snap: u8::try_from(r.below(256)).unwrap(), stale: r.below(2) == 1, fallback: r.below(2) == 1, addon: pick_i64(r), fut_fallback: r.below(2) == 1, idxc: u8::try_from(r.below(3)).unwrap(), elm2: i32::try_from(r.below(1 << 30)).unwrap() - (1 << 29), ..MarginAt::default() };
    m.parts = common_fill::margin::Parts { span: pick_i64(r), premium: pick_i64(r), elm: pick_i64(r), exact: pick_i64(r), nse_only: pick_i64(r), rate_h: pick_i64(r), otm: r.below(2) == 1 };
    // every refusal value and None, in turn
    m.refuse = if k % 15 == 14 { None } else { REFUSE.get(k % 15).copied() };
    m
}

/// One synthetic segment day: `n` events (seconds random, so list ids go DOWN as well as up inside a key-sorted list), every record kind,
/// and per config a key-sorted random subset of the day's events.
fn syn_day(seed: u64, di: usize, n: usize, fl: Flags) -> DayIn {
    let mut r = Rng::new(seed, di as u64 + 1);
    let mut d = DayIn { di, prepared: n / 3, ..DayIn::default() };
    for k in 0..n {
        d.evs.push(SEv { di: u16::try_from(di).unwrap(), ci: u32::try_from(r.below(500)).unwrap(), t: 33_300 + u32::try_from(r.below(22_000)).unwrap(), skey: u32::try_from(r.below(9)).unwrap(), tab: crate::trade::EvT::default() });
        d.liq.push((u32::try_from(r.below(61)).unwrap(), u32::try_from(r.below(20)).unwrap()));
        d.tfmask.push(u16::try_from(r.below(1 << 10)).unwrap());
        d.rd.push(rd_of(&mut r));
        let nt = usize::try_from(r.below(11)).unwrap();
        d.tfw.push((0..nt).map(|_| (u8::try_from(r.below(10)).unwrap(), pick_i64(&mut r), pick_i64(&mut r))).collect());
        if fl.vol60 {
            d.vol60.push(r.next());
        }
        if fl.mx {
            d.mx.push(mx_of(&mut r, k));
        }
    }
    for c in 0..NC {
        if c % 7 == 3 {
            d.lists.push(Vec::new());
            continue;
        }
        let mut l: Vec<u32> = (0..u32::try_from(n).unwrap()).filter(|_| r.below(3) == 0).collect();
        l.sort_by_key(|&e| (d.evs[e as usize].t, e));
        d.lists.push(l);
    }
    d
}

fn days_of(seed: u64, fl: Flags, sizes: &[usize]) -> Vec<DayIn> {
    sizes.iter().enumerate().map(|(k, &n)| syn_day(seed, k * 2 + 1, n, fl)).collect()
}

struct Built {
    dir: PathBuf,
    v2: String,
    v1: String,
    own: Own,
}

/// A store written by the writer's own parts (Builder, the list stage, the digests, SETUP.tsv) from synthetic days; `bad_mu` plants a
/// wrong member digest (with v2 recomputed over it: a writer fault only the start check can see).
fn build_store(name: &str, days: Vec<DayIn>, fl: Flags, liqs: &[usize], bad_mu: Option<usize>) -> Built {
    let root = tmp(name);
    let dir = root.join("SETUP");
    let part = e10::part_of(&dir);
    let mut b = e10::Builder::new(&part, fl, NC).unwrap();
    let rundays: Vec<String> = (0..9).map(|k| format!("2023-03-{:02}", 20 + k)).collect();
    let mut segdays = Vec::new();
    for d in days {
        let di = d.di;
        segdays.push(di);
        let date = rundays[di].clone();
        b.push(e10::seal_day(d, fl, NC, None).unwrap(), &date).unwrap();
    }
    b.close().unwrap();
    let st = e10::lists_stage(&b.compact, &b.liq, liqs, false);
    let (_, mut mu, _) = e10::ul_digest(&b.compact, &b.liq, &st, 2);
    if let Some(u) = bad_mu {
        mu[u] = sha256_bytes(b"not the members");
    }
    let ul: Vec<e10::ULine> = st.uls.iter().enumerate().map(|(u, &(rep, v, tfk))| e10::ULine { u, rep, v, tfk, n: st.ul_n[u], cfgs: st.ul_cfgs[u].clone(), m_u: mu[u].clone() }).collect();
    let hdr = e10::v1_prefix(&rundays, &segdays, &e10::pieces_of(&rundays), "s", "m", &[0, 1], &[0, 0], &[true, false], &[]);
    let mut hdr = hdr;
    hdr.extend_from_slice(b"levels L\nlevels per day []\n");
    std::fs::write(part.join("HDR.bin"), &hdr).unwrap();
    let v1 = sha256_bytes(b"v1 of the test store");
    let own = Own { seg: 0, days: rundays.clone(), segdays: segdays.clone(), liqs: liqs.to_vec(), tfdep: false, flags: fl, args: "a".repeat(64), inputs: "b".repeat(64), ncfg: NC };
    let meta = e10::Meta { seg: 0, days: rundays, segdays, nev: b.e0, ncfg: NC, liqs: liqs.to_vec(), tfdep: false, flags: fl, args: own.args.clone(), inputs: own.inputs.clone(), v1: v1.clone(), hdr_len: hdr.len() as u64, hdr_sha: sha256_bytes(&hdr) };
    let core = e10::core_text(&meta, &b.d, &ul);
    let v2 = e10::v2_of(&core, &b.d, &ul);
    e10::write_setup_tsv(&part, &core, &v2, &[("test".to_owned(), "1".to_owned())]).unwrap();
    std::fs::rename(&part, &dir).unwrap();
    Built { dir, v2, v1, own }
}

fn open(b: &Built) -> Result<Ship, String> {
    Ship::open(&b.dir, &b.v2, &b.v1, &b.own)
}

fn edit_setup(b: &Built, f: &dyn Fn(&str) -> String) -> Built {
    let root = tmp("edit");
    let dir = root.join("SETUP");
    std::fs::create_dir_all(dir.join("ev")).unwrap();
    for e in std::fs::read_dir(b.dir.join("ev")).unwrap().flatten() {
        std::fs::copy(e.path(), dir.join("ev").join(e.file_name())).unwrap();
    }
    std::fs::copy(b.dir.join("HDR.bin"), dir.join("HDR.bin")).unwrap();
    let t = std::fs::read_to_string(b.dir.join("SETUP.tsv")).unwrap();
    let e = f(&t);
    assert_ne!(e, t, "the test edit must change SETUP.tsv");
    std::fs::write(dir.join("SETUP.tsv"), e).unwrap();
    Built { dir, v2: b.v2.clone(), v1: b.v1.clone(), own: b.own.clone() }
}

// ------------------------------------------------------------------------------------------------ 1-2: the codec

#[test]
fn e10_record_roundtrip() {
    for (seed, fl) in [(1_u64, Flags { vol60: true, mx: true }), (2, Flags { vol60: false, mx: false }), (3, Flags { vol60: true, mx: false }), (4, Flags { vol60: false, mx: true })] {
        let d = syn_day(seed, 5, 300, fl);
        let raw = e10::enc_section(&d, fl, NC).unwrap();
        let w = e10::dec_section(raw.clone(), fl, 0).unwrap();
        assert_eq!(w.n as usize, d.evs.len());
        for i in 0..d.evs.len() {
            let want = e10::v1_line(d.evs.get(i), d.liq.get(i), d.tfmask.get(i), d.rd.get(i), d.tfw.get(i), d.vol60.get(i), d.mx.get(i));
            assert_eq!(w.line(i), want, "seed {seed} event {i}: every field must round-trip (None / Some, both theta codes, all 14 refusals + None, i64 MIN/MAX, u128::MAX, 0-10 windows)");
        }
        // a trailing byte, a short section, flags of another layout: refused
        let mut t = raw.clone();
        t.push(0);
        assert!(e10::dec_section(t, fl, 0).is_err(), "a trailing byte is refused");
        assert!(e10::dec_section(raw[..raw.len() - 3].to_vec(), fl, 0).is_err(), "a short section is refused");
        assert!(e10::dec_section(raw.clone(), Flags { vol60: !fl.vol60, mx: fl.mx }, 0).is_err(), "another FLAGS layout is refused, never decoded shifted");
    }
    // an unknown theta code is refused at encode (never truncated)
    let fl = Flags::default();
    let mut d = syn_day(9, 1, 5, fl);
    d.rd[2].theta = Err("theta_new_code");
    assert!(e10::enc_section(&d, fl, NC).unwrap_err().contains("theta"));
    // a flagged field missing is refused
    let mut d = syn_day(9, 1, 5, Flags { vol60: true, mx: false });
    d.vol60.pop();
    assert!(e10::enc_section(&d, Flags { vol60: true, mx: false }, NC).is_err());
}

#[test]
fn e10_section_lists_roundtrip() {
    let fl = Flags { vol60: false, mx: false };
    // a day of more than 65,536 events: ids beyond u16, lists whose ids go down inside the key order (negative deltas), empty configs
    let d = syn_day(11, 3, 70_000, fl);
    assert!(d.lists.iter().any(|l| l.windows(2).any(|w| w[1] < w[0])), "the test needs negative deltas");
    let raw = e10::enc_section(&d, fl, NC).unwrap();
    let w = e10::dec_section(raw, fl, 0).unwrap();
    let mut got = Vec::new();
    for (c, l) in d.lists.iter().enumerate() {
        w.cfg_day(u32::try_from(c).unwrap(), &mut got).unwrap();
        assert_eq!(&got, l, "config {c}'s day list must round-trip");
    }
    // the in-memory varint lists: global ids beyond 2^24, many days, the same sequences
    let mut cp = e10::Compact::new(NC);
    let mut want: Vec<Vec<u32>> = vec![Vec::new(); NC];
    let mut e0 = (1_u64 << 24) + 7;
    for day in 0..3_u64 {
        let dd = syn_day(20 + day, 1, 2_000, fl);
        cp.push_day(e0, &dd.lists).unwrap();
        for (c, l) in dd.lists.iter().enumerate() {
            want[c].extend(l.iter().map(|&x| u32::try_from(e0 + u64::from(x)).unwrap()));
        }
        e0 += 2_000;
    }
    let mut g = Vec::new();
    for (c, l) in want.iter().enumerate() {
        e10::LSrc::list_into(&cp, c, &mut g);
        assert_eq!(&g, l, "config {c}: the varint lists must give back the whole list");
    }
    // CRITIC S10: a list out of key order is refused
    let mut d2 = syn_day(12, 1, 50, fl);
    let l = d2.lists.iter_mut().find(|l| l.len() > 3).unwrap();
    l.reverse();
    assert!(e10::enc_section(&d2, fl, NC).unwrap_err().contains("key order"));
}

// ------------------------------------------------------------------------------------------------ 3-4: the same rule, the same digest

fn seg_of_days(days: &[DayIn]) -> Seg3 {
    let mut sr = Seg3 { days: days.iter().map(|d| d.di).collect(), piece_of_day: vec![0; 9], ..Seg3::default() };
    sr.lists = vec![Vec::new(); NC];
    let mut e0 = 0_u32;
    for d in days {
        sr.evs.extend(d.evs.iter().cloned());
        sr.liq.extend(&d.liq);
        sr.tfmask.extend(&d.tfmask);
        sr.rd.extend(d.rd.iter().copied());
        sr.tfw.extend(d.tfw.iter().cloned());
        sr.vol60.extend(&d.vol60);
        sr.mx.extend(d.mx.iter().copied());
        for (c, l) in d.lists.iter().enumerate() {
            sr.lists[c].extend(l.iter().map(|&x| e0 + x));
        }
        e0 += u32::try_from(d.evs.len()).unwrap();
    }
    sr
}

#[test]
fn e10_lists_stage_same_rule() {
    for (seed, tfdep) in [(31_u64, false), (32, true)] {
        let mut days = days_of(seed, Flags::default(), &[400, 0, 350, 500]);
        // repair:shard-memory (the host suite at 20:45 IST: "the test needs shared lists" failed: syn_day's random lists are never equal):
        // every config c with c % 5 == 4 repeats config c - 1's list on every day, so (config, value) pairs share unique lists (the
        // first-seen numbering, the FULL compare and ul_cfgs are then exercised); ids < 40 share the timeframe part, so tf_dep shares too
        for d in &mut days {
            for c in (4..NC).step_by(5) {
                let l = d.lists[c - 1].clone();
                d.lists[c] = l;
            }
        }
        let mut sr = seg_of_days(&days);
        let liqs = [0_usize, 1, 2, 4, 7];
        let mut cp = e10::Compact::new(NC);
        let mut e0 = 0_u64;
        for d in &days {
            cp.push_day(e0, &d.lists).unwrap();
            e0 += d.evs.len() as u64;
        }
        let mut log = String::new();
        crate::report3::build_lists_derived(&mut sr, &liqs, tfdep, &mut log, "test");
        for (what, st) in [("vectors", e10::lists_stage(&e10::VecSrc(&sr.lists), &sr.liq, &liqs, tfdep)), ("stored varints", e10::lists_stage(&cp, &sr.liq, &liqs, tfdep))] {
            assert_eq!(st.uls, sr.uls, "{what} seed {seed}: the unique lists (first-seen numbering) must be build_lists_derived's");
            assert_eq!(st.ul_cfgs, sr.ul_cfgs, "{what}: every (config, value) pair");
            assert_eq!(st.removed, sr.removed, "{what}: the removals");
            for (u, &n) in st.ul_n.iter().enumerate() {
                assert_eq!(n as usize, sr.ul(u).unwrap().len(), "{what}: list {u}'s member count");
            }
        }
        assert!(sr.uls.len() > 10 && sr.ul_cfgs.iter().any(|c| c.len() > 1), "the test needs shared lists");
    }
}

#[test]
fn e10_v1_digest_same_bytes() {
    let fl = Flags { vol60: true, mx: true };
    let days = days_of(41, fl, &[3_000, 2_500, 3_700]);
    let mut sr = seg_of_days(&days);
    let liqs = [0_usize, 1, 7];
    let mut log = String::new();
    crate::report3::build_lists_derived(&mut sr, &liqs, false, &mut log, "test");
    sr.ties = vec![3, 1];
    sr.search = vec![true, false];
    let rundays: Vec<String> = (0..9).map(|k| format!("2023-03-{:02}", 20 + k)).collect();
    let csets = vec![crate::kern::CSet { rules: vec![(false, 2), (true, 1)], sl: 1 }, crate::kern::CSet::default()];
    let mut ps = e10::ParStream::new(4_096);
    for i in 0..sr.evs.len() {
        ps.push(e10::v1_line(sr.evs.get(i), sr.liq.get(i), sr.tfmask.get(i), sr.rd.get(i), sr.tfw.get(i), sr.vol60.get(i), sr.mx.get(i)).as_bytes());
    }
    let ev = ps.finish();
    let st = e10::ListStage { uls: sr.uls.clone(), ul_cfgs: sr.ul_cfgs.clone(), ..e10::ListStage::default() };
    for th in [1_usize, 7] {
        let want = kit::list_digest(&sr, &rundays, &csets, &[4, 9], "S", "M", th);
        let hdr = e10::v1_header(&rundays, &sr.days, &sr.piece_of_day, "S", "M", &[4, 9], &sr.ties, &sr.search, &csets, &sr.lv, &sr.lvd);
        let cfg = e10::cfg_digest(&e10::VecSrc(&sr.lists), th);
        let (ul, _, _) = e10::ul_digest(&e10::VecSrc(&sr.lists), &sr.liq, &st, th);
        let got = e10::v1_assemble(&hdr, sr.evs.len() as u64, &ev, sr.lists.len(), &cfg, sr.uls.len(), &ul);
        assert_eq!(got, want, "{th} threads: v1 from the store's parts must be kit::list_digest's (9,200 events: three event chunks)");
    }
}

// ------------------------------------------------------------------------------------------------ 5-6: v2 binds the store (M1), the start check

#[test]
fn e10_v2_binds_setup() {
    let fl = Flags { vol60: true, mx: true };
    let b = build_store("v2", days_of(51, fl, &[200, 150, 260]), fl, &[0, 1], None);
    let s = open(&b).unwrap();
    assert_eq!(s.v2, b.v2);
    // M1: FLAGS edited (the shard's own switches set to match the edit): v2 must refuse it
    let flip = edit_setup(&b, &|t| t.replace("FLAGS\tvol60=1\tmx=1", "FLAGS\tvol60=0\tmx=1"));
    let mut own2 = flip.own.clone();
    own2.flags = Flags { vol60: false, mx: true };
    assert!(Ship::open(&flip.dir, &flip.v2, &flip.v1, &own2).err().unwrap_or_default().contains("v2 recomputed"), "a FLAGS line outside v2 would decode every record with another layout (M1)");
    // FLAGS of the store against this run's own switches
    let mut own3 = b.own.clone();
    own3.flags = Flags { vol60: false, mx: true };
    assert!(Ship::open(&b.dir, &b.v2, &b.v1, &own3).err().unwrap_or_default().contains("FLAGS"), "the shard derives FLAGS from its own switches");
    // NEV off by one, a D line, a U line, the HDR line, the V1 line: every one is in v2
    for (from, to) in [("NEV\t610", "NEV\t611"), ("NCFG\t40", "NCFG\t41"), ("TFDEP\t0", "TFDEP\t1"), ("LIQS\t0,1", "LIQS\t0,2")] {
        let e = edit_setup(&b, &|t| t.replace(from, to));
        assert!(Ship::open(&e.dir, &e.v2, &e.v1, &e.own).is_err(), "{from} -> {to} must be refused");
    }
    let flip_last = |l: &str| -> String { let c = if l.ends_with('0') { '1' } else { '0' }; format!("{}{c}\n", &l[..l.len() - 1]) };
    let e = edit_setup(&b, &|t| t.lines().map(|l| if l.starts_with("D\t3\t") { flip_last(l) } else { format!("{l}\n") }).collect());
    assert!(Ship::open(&e.dir, &e.v2, &e.v1, &e.own).err().unwrap_or_default().contains("v2"), "a D line edited");
    let e = edit_setup(&b, &|t| t.lines().map(|l| if l.starts_with("U\t0\t") { flip_last(l) } else { format!("{l}\n") }).collect());
    assert!(Ship::open(&e.dir, &e.v2, &e.v1, &e.own).err().unwrap_or_default().contains("v2"), "a U line edited");
    // this run's own facts
    for k in 0..4 {
        let mut o = b.own.clone();
        match k {
            0 => o.args = "c".repeat(64),
            1 => o.inputs = "c".repeat(64),
            2 => {
                o.segdays.pop();
            }
            _ => o.liqs = vec![0],
        }
        assert!(Ship::open(&b.dir, &b.v2, &b.v1, &o).is_err(), "own fact {k} differs: refused");
    }
    // E_d: a section whose bytes differ (a valid frame of OTHER content) is refused when read
    let d = s.d[1].clone();
    let raw = e10::read_raw(&b.dir, &d, true).unwrap();
    let mut bad = raw.clone();
    bad[20] ^= 1;
    let z = crate::slice::zstd_c(&bad).unwrap();
    let root = tmp("ed");
    std::fs::create_dir_all(root.join("ev")).unwrap();
    std::fs::write(root.join("ev/x.sfe.zst"), &z).unwrap();
    let d2 = e10::DLine { file: "ev/x.sfe.zst".to_owned(), off: 0, len: z.len() as u64, ..d };
    assert!(e10::read_raw(&root, &d2, true).unwrap_err().contains("E_d"), "a section is never used without its sha256");
}

#[test]
fn e10_start_check_binds_members() {
    let fl = Flags { vol60: false, mx: true };
    let b = build_store("mu", days_of(61, fl, &[300, 280, 330]), fl, &[0, 1, 7], None);
    let s = open(&b).unwrap();
    let all: Vec<usize> = (0..s.u.len()).collect();
    assert!(s.start_check(&all, 3).unwrap().contains("every M_u verified"));
    // a writer whose member digest of list 2 is wrong (v2 consistent with it): only the start check can see it
    let bad = build_store("mu-bad", days_of(61, fl, &[300, 280, 330]), fl, &[0, 1, 7], Some(2));
    let sb = open(&bad).unwrap();
    assert!(sb.start_check(&all, 2).unwrap_err().contains("list 2"), "members that are not M_u must be refused before the first batch");
    assert!(sb.start_check(&[0, 1], 2).is_ok(), "a shard that does not walk list 2 is not refused for it");
}

// ------------------------------------------------------------------------------------------------ 7-9: the window guards (G1, M2), the walked parts (M3)

fn win_parts(s: &Ship, w: &e10::Window, lists: &[usize]) -> (Vec<usize>, Vec<(Vec<u32>, Vec<u32>)>) {
    let mut m3: Vec<Sha256> = Vec::new();
    e10::ship_parts(s, w, lists, &mut m3).unwrap()
}

#[test]
fn e10_window_guard() {
    let fl = Flags { vol60: true, mx: false };
    let b = build_store("g1", days_of(71, fl, &[120, 90]), fl, &[0, 1], None);
    let s = open(&b).unwrap();
    let di = s.d[0].di;
    let mut w = s.window(di).unwrap();
    let lists: Vec<usize> = (0..s.u.len().min(6)).collect();
    let (want, subs) = win_parts(&s, &w, &lists);
    let id = e10::WinId { di: w.di, e0: w.e0, n: w.n };
    let mut sr = Seg3::default();
    w.swap(&mut sr);
    let d = s.dline(di).unwrap().clone();
    assert!(e10::guard(&sr, id, &d, fl, di, true, &want, &subs).is_empty(), "a correct window passes");
    let mut want2 = want.clone();
    want2.push(w.n as usize);
    assert!(!e10::guard(&sr, id, &d, fl, di, true, &want2, &subs).is_empty(), "G1: an id outside the window handed to stage A is counted");
    let mut subs2 = subs.clone();
    subs2[0].0.push(w.n + 5);
    assert!(!e10::guard(&sr, id, &d, fl, di, true, &want, &subs2).is_empty(), "G1: an id outside the window in a walked sub-list is counted (the kernel would skip it silently)");
}

#[test]
fn e10_window_identity_guard() {
    let fl = Flags { vol60: true, mx: true };
    let b = build_store("m2", days_of(81, fl, &[140, 160, 120]), fl, &[0, 1], None);
    let s = open(&b).unwrap();
    let lists: Vec<usize> = (0..s.u.len().min(5)).collect();
    // every day's window as the stage B loop gets it: the guard alone must find nothing
    for d in s.d.clone() {
        let mut w = s.window(d.di).unwrap();
        let (want, subs) = win_parts(&s, &w, &lists);
        let id = e10::WinId { di: w.di, e0: w.e0, n: w.n };
        let mut sr = Seg3::default();
        w.swap(&mut sr);
        let g = e10::guard(&sr, id, &d, fl, d.di, true, &want, &subs);
        assert!(g.is_empty(), "day {}: the store's own window must pass M2: {g:?}", d.di);
    }
    // planted: day 0's window used for day 1; the exact margins dropped; the day's cube not loaded
    let (d0, d1) = (s.d[0].clone(), s.d[1].clone());
    let mut w0 = e10::dec_section(e10::read_raw(&b.dir, &d0, true).unwrap(), fl, d0.e0).unwrap();
    let (want, subs) = win_parts(&s, &w0, &lists);
    let id0 = e10::WinId { di: w0.di, e0: w0.e0, n: w0.n };
    let mut sr = Seg3::default();
    w0.swap(&mut sr);
    assert!(!e10::guard(&sr, id0, &d1, fl, d1.di, true, &want, &subs).is_empty(), "M2: another day's window is counted");
    assert!(!e10::guard(&sr, id0, &d0, fl, d0.di, false, &want, &subs).is_empty(), "M2: an unloaded cube is counted");
    sr.mx.clear();
    assert!(!e10::guard(&sr, id0, &d0, fl, d0.di, true, &want, &subs).is_empty(), "M2: an empty mx under FLAGS mx=1 is counted (stage A would keep the PROXY margin)");
    let mut sr2 = Seg3::default();
    let mut w1 = e10::dec_section(e10::read_raw(&b.dir, &d1, true).unwrap(), fl, d1.e0).unwrap();
    let id1 = e10::WinId { di: w1.di, e0: w1.e0, n: w1.n };
    w1.swap(&mut sr2);
    sr2.rd.pop();
    assert!(!e10::guard(&sr2, id1, &d1, fl, d1.di, true, &[], &[]).is_empty(), "M2: a short rd is counted (stage A would read a default)");
}

#[test]
fn e10_walk_parts_bound() {
    let fl = Flags { vol60: false, mx: false };
    let b = build_store("m3", days_of(91, fl, &[220, 0, 260, 240]), fl, &[0, 1, 2, 7], None);
    let s = open(&b).unwrap();
    let blist: Vec<usize> = (0..s.u.len()).collect();
    assert!(s.start_check(&blist, 2).is_ok(), "the start check derives with day_sub directly");
    let mut m3: Vec<Sha256> = blist.iter().map(|&u| s.m3_start(u).unwrap()).collect();
    for d in &s.d {
        let w = s.window(d.di).unwrap();
        let _ = e10::ship_parts(&s, &w, &blist, &mut m3).unwrap();
    }
    let bad: Vec<usize> = m3.into_iter().zip(&blist).filter(|(h, u)| h.clone().hex() != s.u[**u].m_u).map(|(_, &u)| u).collect();
    assert!(bad.is_empty(), "M3: every list's walked parts must hash to its M_u (lists {bad:?} differ)");
}

// ------------------------------------------------------------------------------------------------ 10: the plan's SETUP line

#[test]
fn e10_plan_setup_line() {
    let base = Plan { path: "t".into(), seg: 0, days: vec!["2023-03-28".into()], nset: 2, nl: 3, digest: "d".repeat(64), args: "a".repeat(64), order: "creation".into(), per_list: 8, cut: "x".into(), b: vec![(0, 3, 9, vec![0, 1, 2])], ..Plan::default() };
    let t0 = base.text();
    assert!(!t0.contains("SETUP"), "a plan without a store keeps the kit's text byte for byte");
    assert_eq!(Plan::parse(&t0).unwrap().setup, "");
    let mut p = base.clone();
    p.setup = "e".repeat(64);
    let t = p.text();
    assert!(t.ends_with(&format!("SETUP\t{}\nEND\n", "e".repeat(64))));
    let q = Plan::parse(&t).unwrap();
    assert_eq!(q.setup, p.setup);
    assert_eq!(q.text(), t);
    assert!(Plan::parse(&t.replace("END\n", &format!("SETUP\t{}\nEND\n", "f".repeat(64)))).is_err(), "SETUP once");
    assert!(Plan::parse(&t.replace(&"e".repeat(64), "XYZ")).is_err(), "SETUP is 64 lower-case hex");
    // the store against the plan: SETUP = v2 and DIGEST = V1, or refused
    let fl = Flags::default();
    let b = build_store("plan", days_of(101, fl, &[60, 70]), fl, &[0], None);
    assert!(Ship::open(&b.dir, &"0".repeat(64), &b.v1, &b.own).err().unwrap_or_default().contains("SETUP line"));
    assert!(Ship::open(&b.dir, &b.v2, &"0".repeat(64), &b.own).err().unwrap_or_default().contains("DIGEST"), "a store whose V1 is not the plan's list digest is refused");
}

// ------------------------------------------------------------------------------------------------ 11-12: R1 and R2 are exact

#[test]
fn e10_bhav_lru_exact() {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    let load = |k: i64| -> Result<Arc<(i64, u64)>, String> { if k == 13 { Err("no file".to_owned()) } else { Ok(Arc::new((k, (k as u64).wrapping_mul(0x9e37_79b9) ^ 77))) } };
    for cap in [0_usize, 2, 3, 5] {
        let cache: Mutex<HashMap<i64, Arc<(i64, u64)>>> = Mutex::new(HashMap::new());
        let ticks: Mutex<(u64, HashMap<i64, u64>)> = Mutex::new((0, HashMap::new()));
        let mut r = Rng::new(7, cap as u64);
        for _ in 0..2_000 {
            let k = i64::try_from(r.below(12)).unwrap();
            let v = e10::lru_get(&cache, &ticks, cap, k, &load).unwrap();
            assert_eq!(*v, *load(k).unwrap(), "cap {cap}: session {k} must read its own file's parse");
            if cap > 0 {
                assert!(cache.lock().unwrap().len() <= cap, "cap {cap}: never more than {cap} sessions held");
            }
        }
        assert!(e10::lru_get(&cache, &ticks, cap, 13, &load).is_err(), "a failing parse stays a failure (STRICT records it)");
    }
}

#[test]
fn e10_levels_stream_equal() {
    use crate::fix33::DaySmp;
    for (seed, w) in [(1_u64, 1_usize), (2, 2), (3, 60), (4, 5)] {
        let mut r = Rng::new(seed, 99);
        let n = 40;
        let days: Vec<Option<DaySmp>> = (0..n)
            .map(|_| match r.below(6) {
                0 => None,
                // CRITIC S11: segment present, every timeframe's samples empty (still a PRESENT day of the causal window)
                1 => Some(vec![(Vec::new(), Vec::new()); 10]),
                _ => Some(crate::fix33::sort_samples((0..10).map(|_| ((0..1 + r.below(30)).map(|_| r.below(5_000)).collect(), (0..r.below(30)).map(|_| r.below(9_000_000)).collect())).collect())),
            })
            .collect();
        let want = crate::fix33::causal_levels(&days, w, 3);
        let mut ring = e10::LvRing::new(n, w);
        for (d, x) in days.iter().enumerate() {
            ring.feed(d, x.clone().unwrap_or_default());
        }
        assert_eq!(format!("{:?}", ring.cl), format!("{want:?}"), "seed {seed} W {w}: the streamed causal levels must be causal_levels'");
    }
}

// ------------------------------------------------------------------------------------------------ 13-16: the writer's self-check, args, header, refusals

#[test]
fn e10_writer_self_check() {
    let fl = Flags { vol60: true, mx: true };
    let d = syn_day(111, 2, 50, fl);
    assert!(e10::seal_day(d.clone(), fl, NC, None).is_ok());
    // one byte of event 0's second (offset 11 header + 2 di + 4 ci = 17) changed after the encode: the decode-back must refuse the day
    let e = e10::seal_day(d, fl, NC, Some(17)).unwrap_err();
    assert!(e.contains("decode-back"), "{e}");
}

#[test]
fn e10_args_runtime_flags() {
    let v = |s: &str| -> Vec<String> { s.split(' ').map(str::to_owned).collect() };
    let a = v("spikefade run3 /c /o 2023-03-28 --set S0 --liq 0,7 --v33 --day-major --segs idx --threads 12");
    for extra in ["--bhav-cache 16", "--levels-stream on", "--write-setup /s/SETUP-idx", "--setup-from /x/setup", "--setup-audit on", "--setup-recompute on"] {
        let mut b = a.clone();
        b.extend(v(extra));
        assert_eq!(kit::args_digest(&a), kit::args_digest(&b), "{extra} is runtime-only: the argument digest must not change (a SHIP_R shard and its writer agree)");
    }
    let mut c = a.clone();
    c.extend(v("--liq-cover-s 30"));
    assert_ne!(kit::args_digest(&a), kit::args_digest(&c), "a result-bearing flag still counts");
}

#[test]
fn e10_header_bound() {
    let fl = Flags::default();
    let b = build_store("hdr", days_of(121, fl, &[40, 30]), fl, &[0], None);
    let s = open(&b).unwrap();
    let rundays: Vec<String> = (0..9).map(|k| format!("2023-03-{:02}", 20 + k)).collect();
    let segdays = b.own.segdays.clone();
    let good = e10::v1_prefix(&rundays, &segdays, &e10::pieces_of(&rundays), "s", "m", &[0, 1], &[0, 0], &[true, false], &[]);
    assert!(s.header_check(&good).is_ok());
    let other_ties = e10::v1_prefix(&rundays, &segdays, &e10::pieces_of(&rundays), "s", "m", &[0, 1], &[0, 7], &[true, false], &[]);
    assert!(s.header_check(&other_ties).is_err(), "another tie vector: refused");
    let other_cset = e10::v1_prefix(&rundays, &segdays, &e10::pieces_of(&rundays), "s", "m", &[0, 1], &[0, 0], &[true, false], &[crate::kern::CSet::default()]);
    assert!(s.header_check(&other_cset).is_err(), "another compiled setting: refused");
}

#[test]
fn e10_flags_refusals() {
    let a = |v: &[&str]| -> Vec<String> { std::iter::once("spikefade").chain(std::iter::once("run3")).chain(v.iter().copied()).map(str::to_owned).collect() };
    assert!(e10::from_args(&a(&["--write-setup", "/x", "--setup-from", "/y"])).unwrap_err().contains("never both"));
    assert!(e10::from_args(&a(&["--setup-from", "/y", "--mode", "POSITIONAL"])).unwrap_err().contains("POSITIONAL"), "CRITIC M8 / R-15: a positional run refused by name");
    assert!(e10::from_args(&a(&["--setup-audit", "on"])).unwrap_err().contains("needs --setup-from"));
    assert!(e10::from_args(&a(&["--write-setup", "/x"])).unwrap_err().contains("--write-plan"), "the writer IS the plan run");
    assert!(e10::from_args(&a(&["--setup-from", "/y"])).unwrap_err().contains("--shard on"));
    // CRITIC S2: a shard whose plan names a store reads it; only a Mac proof recomputes it
    assert!(e10::s2_check(true, true, false, false).unwrap_err().contains("--setup-from"));
    assert!(e10::s2_check(true, true, true, false).is_ok() && e10::s2_check(true, true, false, true).is_ok() && e10::s2_check(true, false, false, false).is_ok() && e10::s2_check(false, true, false, false).is_ok());
}

// ------------------------------------------------------------------------------------------------ 17-18: the shift invariance (CRITIC 1), G2

#[test]
fn e10_shift_invariant() {
    let fl = Flags::default();
    let d = syn_day(131, 4, 600, fl);
    let e0 = 100_003_u32;
    // the global event array: e0 placeholders of another day, then this day's events
    let mut all: Vec<SEv> = vec![SEv { di: 1, ..SEv::default() }; e0 as usize];
    all.extend(d.evs.iter().cloned());
    let subs_l: Vec<(Vec<u32>, Vec<u32>)> = d.lists.iter().map(|l| (l.clone(), crate::kern::list_keys(&d.evs, l))).collect();
    let subs_g: Vec<(Vec<u32>, Vec<u32>)> = d.lists.iter().map(|l| {
        let g: Vec<u32> = l.iter().map(|&x| x + e0).collect();
        let k = crate::kern::list_keys(&all, &g);
        (g, k)
    }).collect();
    for (a, b) in subs_l.iter().zip(&subs_g) {
        assert_eq!(a.1, b.1, "the in-day keys are the same under the shift");
    }
    let tfs: Vec<Option<u8>> = (0..subs_l.len()).map(|k| if k % 3 == 0 { Some(2) } else { None }).collect();
    assert_eq!(crate::report3::day_classes33(&subs_l, &tfs), crate::report3::day_classes33(&subs_g, &tfs), "D2-1 classes: first-seen numbering, full compares: the same under the shift");
    let ids_l: Vec<usize> = (0..d.evs.len()).collect();
    let ids_g: Vec<usize> = ids_l.iter().map(|&i| i + e0 as usize).collect();
    let cl = crate::report3::stage_a_chunks33(&d.evs, ids_l, 4);
    let cg = crate::report3::stage_a_chunks33(&all, ids_g, 4);
    assert_eq!(cl.len(), cg.len());
    for (a, b) in cl.iter().zip(&cg) {
        assert_eq!(a.0, b.0);
        assert_eq!(a.1.iter().map(|&x| x + e0 as usize).collect::<Vec<_>>(), b.1, "stage A items: the same events in the same order under the shift");
    }
}

#[test]
fn e10_setup_audit_detects() {
    let fl = Flags { vol60: false, mx: true };
    let days = days_of(141, fl, &[90, 110]);
    let sr = seg_of_days(&days);
    let b = build_store("g2", days, fl, &[0, 1], None);
    let s = open(&b).unwrap();
    let mk = |sr: &Seg3| e10::Audit { evs: sr.evs.clone(), liq: sr.liq.clone(), tfmask: sr.tfmask.clone(), rd: sr.rd.clone(), tfw: sr.tfw.clone(), vol60: sr.vol60.clone(), mx: sr.mx.clone(), lists: sr.lists.clone() };
    *s.audit.lock().unwrap() = Some(mk(&sr));
    for d in &s.d {
        assert_eq!(s.audit_window(&s.window(d.di).unwrap()), 0, "day {}: the store's window = the in-memory set-up", d.di);
    }
    // a planted difference in a config list (one member of config 1 moved to another event)
    let mut sr2 = sr.clone_lists();
    let l = sr2.lists.iter_mut().find(|l| l.len() > 2).unwrap();
    l[1] = l[1].saturating_add(1);
    *s.audit.lock().unwrap() = Some(mk(&sr2));
    let bad: u64 = s.d.iter().map(|d| s.audit_window(&s.window(d.di).unwrap())).sum();
    assert!(bad > 0, "G2 must see a config list that differs");
    // a planted difference in a record
    let mut sr3 = sr.clone_lists();
    sr3.evs[5].t += 1;
    *s.audit.lock().unwrap() = Some(mk(&sr3));
    let bad: u64 = s.d.iter().map(|d| s.audit_window(&s.window(d.di).unwrap())).sum();
    assert!(bad > 0, "G2 must see a record that differs");
}

trait CloneLists {
    fn clone_lists(&self) -> Seg3;
}

impl CloneLists for Seg3 {
    fn clone_lists(&self) -> Seg3 {
        Seg3 { evs: self.evs.clone(), liq: self.liq.clone(), tfmask: self.tfmask.clone(), rd: self.rd.clone(), tfw: self.tfw.clone(), vol60: self.vol60.clone(), mx: self.mx.clone(), lists: self.lists.clone(), days: self.days.clone(), ..Seg3::default() }
    }
}


// ------------------------------------------------------------------------------------------------ 19: setup-stats (CHECK F-1)

#[test]
fn e10_setup_stats_union() {
    let fl = Flags { vol60: true, mx: false };
    // the first segment day is the smallest, so most batches' largest union lies on a later day (a scan of day 0 alone differs)
    let days = days_of(151, fl, &[40, 330, 0, 290]);
    let b = build_store("stats", days.clone(), fl, &[0, 1, 2, 7], None);
    let s = open(&b).unwrap();
    let nu = s.u.len();
    assert!(nu >= 6, "the test store needs several unique lists ({nu})");
    // a plan cutting the unique lists 3 at a time (store order), PERLIST 8
    let bb: Vec<(usize, usize, u128, Vec<u32>)> = (0..nu)
        .step_by(3)
        .map(|p0| {
            let p1 = (p0 + 3).min(nu);
            (p0, p1, 0, (p0..p1).map(|u| u32::try_from(u).unwrap()).collect())
        })
        .collect();
    let pl = Plan { per_list: 8, b: bb, ..Plan::default() };
    let rows = e10::batch_unions(&s, &pl, 3).unwrap();
    assert_eq!(rows.len(), pl.b.len());
    for (k, (_, _, _, ids)) in pl.b.iter().enumerate() {
        // brute force from the synthetic days themselves (never the store): per day, the set of local ids of each list's representative
        // config that pass the list's liquidity value
        let mut best: (u64, Option<usize>, u32) = (0, None, 0);
        let mut nonempty = 0_u64;
        for d in &days {
            let mut set = std::collections::BTreeSet::new();
            for &u in ids {
                let ul = &s.u[u as usize];
                for &l in d.lists.get(ul.rep as usize).map_or(&[][..], Vec::as_slice) {
                    let (c, g) = d.liq[l as usize];
                    if crate::read::liq_pass(usize::from(ul.v), c, g) {
                        set.insert(l);
                    }
                }
            }
            let n = set.len() as u64;
            if n > 0 {
                nonempty += 1;
            }
            if n > best.0 {
                best = (n, Some(d.di), u32::try_from(d.evs.len()).unwrap());
            }
        }
        let r = &rows[k];
        assert_eq!((r.umax, r.day, r.n_d, r.days, r.lists), (best.0, best.1, best.2, nonempty, ids.len()), "batch {k}: U_k*, its day, n_d and the non-empty days = the brute-force per-day union");
        if let Some(di) = r.day {
            assert_eq!(r.raw_d, s.dline(di).unwrap().raw, "batch {k}: raw_d = the U_k* day's raw section bytes");
        }
    }
    assert!(rows.iter().any(|r| r.day.is_some_and(|d| d != s.d[0].di)), "the test needs a batch whose largest union is not on the first segment day");
    // pred_k: the formula's terms, in bytes
    assert_eq!(e10::pred_bytes(522, 5_508, 1_000, 6_000, 3_000_000), 522 * 5_508 * 1_800 + 1_800 * 1_048_576 + 1_000 * 36_200 + 2 * (6_000 * 715 + 3_000_000));
}

#[test]
fn e10_mem_startup_line() {
    // ship:shard-memory (CHECK-r2 fix): the SHIP_R start-up line names the memory only (VmHWM / VmRSS from /proc on Linux, the ps RSS
    // elsewhere), never a batch fact: batch 1's own E10 MEM line keeps its batch-day union U* (the F-1 proof reads it per batch)
    let s = e10::mem_now();
    assert!(s.starts_with("VmHWM ") || s.starts_with("RSS "), "{s}");
    assert!(!s.contains("U*") && !s.contains("run day"), "no batch facts at start-up: {s}");
}
