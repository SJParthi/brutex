//! v3.4 build:shard: the named tests of the shard kit (cloud-kit/CLOUD-KIT-DESIGN.md E1-E6, E8, E9, E12; CLOUD-KIT-CRITIC.md F1-F5). Each
//! mutant of Cargo.toml's v3.4 block makes at least one of them FAIL (v34/shard/build/MUTANTS-V34-SHARD.txt).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::acct::{Sum, Tops};
use crate::kit::{self, Bck, Plan, Sha256, sha256_bytes};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sf34-{name}-{}-{}", std::process::id(), kit::utc_now()));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn all_off() {
    for s in 0..kit::COUNT {
        kit::set(s, false);
    }
}

// ------------------------------------------------------------------------------------------------ E5: sha256, tools, clock, sorted reads

#[test]
fn sha256_fips_vectors() {
    assert_eq!(sha256_bytes(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert_eq!(sha256_bytes(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(sha256_bytes(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"), "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
    let a = vec![b'a'; 1_000_000];
    assert_eq!(sha256_bytes(&a), "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
    // incremental updates of any split give the one-shot digest
    let msg: Vec<u8> = (0..1_000_u32).map(|i| u8::try_from(i % 251).unwrap()).collect();
    for cut in [0_usize, 1, 55, 56, 63, 64, 65, 127, 500, 999, 1_000] {
        let mut h = Sha256::new();
        h.update(&msg[..cut]);
        h.update(&msg[cut..]);
        assert_eq!(h.hex(), sha256_bytes(&msg), "split at {cut}");
    }
}

#[test]
fn portable_sha256_in_process() {
    // E5: under PORTABLE the cube sha256 is computed in-process: the plain hex for ANY file name. /usr/bin/shasum escapes a name holding a
    // newline (its line starts with a backslash), so the external tool kept under PORTABLE (mutant m_kit_shasum_path) is caught here.
    all_off();
    let d = tmp("sha");
    let p = d.join("a\nb");
    std::fs::write(&p, b"x").unwrap();
    kit::set(kit::PORTABLE, true);
    let got = crate::cube::sha256(&p);
    kit::set(kit::PORTABLE, false);
    assert_eq!(got, sha256_bytes(b"x"));
    assert_eq!(kit::sha256_file(&p).as_deref(), Some(sha256_bytes(b"x").as_str()));
}

#[test]
fn tools_from_env() {
    // E5: without PORTABLE the v3.3 Mac paths (P0); under PORTABLE the environment (SF_<TOOL>) or the bare name (PATH lookup), never a Mac path
    all_off();
    assert_eq!(kit::tool("zstd"), "/opt/homebrew/bin/zstd");
    assert_eq!(kit::tool("unzip"), "/usr/bin/unzip");
    assert_eq!(kit::tool("gzip"), "/usr/bin/gzip");
    kit::set(kit::PORTABLE, true);
    for t in ["zstd", "unzip", "gzip", "ps"] {
        let want = std::env::var(format!("SF_{}", t.to_ascii_uppercase())).ok().filter(|x| !x.is_empty()).unwrap_or_else(|| t.to_owned());
        assert_eq!(kit::tool(t), want, "tool {t} under PORTABLE");
        assert!(!kit::tool(t).starts_with("/opt/homebrew") && !kit::tool(t).starts_with("/usr/bin/") || std::env::var(format!("SF_{}", t.to_ascii_uppercase())).is_ok());
    }
    kit::set(kit::PORTABLE, false);
}

#[test]
fn read_dir_sorted() {
    let d = tmp("dir");
    // 64 names created in a scrambled order: a file system's own order (APFS hash order, ext4 htree order) is not the name order
    let names: Vec<String> = (0..64_u32).map(|i| format!("f{:03}", (i * 37) % 64)).collect();
    for n in &names {
        std::fs::write(d.join(n), n).unwrap();
    }
    let got: Vec<String> = kit::sorted_dir(&d).unwrap().into_iter().map(|x| x.0).collect();
    let mut want = names.clone();
    want.sort();
    assert_eq!(got, want);
}

#[test]
fn ist_label_offset() {
    assert_eq!(kit::ist_label(0), "1970-01-01 05:30:00 IST");
    // 2026-11-16 05:04:59 UTC = 10:34:59 IST
    let t = u64::try_from(crate::cal::days_from_civil(2026, 11, 16) * 86_400 + 5 * 3_600 + 4 * 60 + 59).unwrap();
    assert_eq!(kit::ist_label(t), "2026-11-16 10:34:59 IST");
    assert_eq!(kit::utc_label(t), "2026-11-16T05:04:59Z");
    // 18:30 UTC rolls to the next IST day
    let t2 = u64::try_from(crate::cal::days_from_civil(2026, 12, 31) * 86_400 + 18 * 3_600 + 30 * 60).unwrap();
    assert_eq!(kit::ist_label(t2), "2027-01-01 00:00:00 IST");
}

#[test]
fn proc_readers_parse_linux_text() {
    assert_eq!(kit::proc_status_rss_kb("Name:\tspikefade\nVmPeak:\t 9 kB\nVmRSS:\t  123456 kB\n"), Some(123_456));
    // a command holding spaces and parentheses: the fields after the LAST ')'
    let stat = "4242 (spike (fade) x) R 1 2 3 4 5 6 7 8 9 10 1500 250 0 0 20 0 1 0";
    assert_eq!(kit::proc_stat_ticks(stat), Some(1_750));
}

// ------------------------------------------------------------------------------------------------ F4: STRICT

#[test]
fn strict_bhav_fatal() {
    // a bhavcopy the index holds but cannot be read (unzip missing): STRICT records it as fatal; lenient (v3.3) swallows it (prev close 0)
    all_off();
    let d = tmp("bhav");
    std::fs::write(d.join("BhavCopy_NSE_FO_0_0_0_20240105_F_0000.csv.zip"), b"not a zip").unwrap();
    let mut b = crate::bhav::Bhav::default();
    b.add_dir(&d).unwrap();
    kit::set_tool_t("unzip", "/nonexistent/unzip-for-the-test");
    let day = crate::cal::days_from_civil(2024, 1, 5);
    let before = kit::strict_errs().len();
    assert!(crate::bhav::strict_day(&b, day).is_none());
    assert_eq!(kit::strict_errs().len(), before, "lenient: nothing recorded");
    kit::set(kit::STRICT, true);
    let b2 = crate::bhav::Bhav::default();
    let mut b2 = b2;
    b2.add_dir(&d).unwrap();
    assert!(crate::bhav::strict_day(&b2, day).is_none());
    let errs = kit::strict_errs();
    kit::set(kit::STRICT, false);
    assert!(errs.len() > before && errs.iter().any(|e| e.starts_with("bhavcopy:")), "STRICT must record the failure: {errs:?}");
}

#[test]
fn kit_flags_refusals() {
    all_off();
    let a = |v: &[&str]| -> Vec<String> { std::iter::once("spikefade").chain(std::iter::once("run3")).chain(v.iter().copied()).map(str::to_owned).collect() };
    assert!(kit::from_args(&a(&["--shard", "on", "--day-major", "--checkpoint", "/tmp/x"])).unwrap_err().contains("needs --batch-plan"));
    assert!(kit::from_args(&a(&["--cube-transport"])).unwrap_err().contains("needs --shard on"));
    assert!(kit::from_args(&a(&["--shard", "maybe"])).is_err());
    // nothing on: no kit words, every switch off (P0)
    all_off();
    assert!(kit::from_args(&a(&["--day-major"])).unwrap().is_empty());
    assert!(!kit::any());
    assert_eq!(kit::parse_set("0,4,8").unwrap(), [0, 4, 8].into_iter().collect::<BTreeSet<usize>>());
    assert_eq!(kit::parse_set("2-4,9").unwrap(), [2, 3, 4, 9].into_iter().collect::<BTreeSet<usize>>());
    assert!(kit::parse_set("4-2").is_err() && kit::parse_set("").is_err() && kit::parse_set("x").is_err());
}

// ------------------------------------------------------------------------------------------------ E1 / E2: the plan and the shard's batches

fn plan_of(cuts: &[usize], digest: &str) -> Plan {
    let mut b = Vec::new();
    let mut p0 = 0;
    for (k, &p1) in cuts.iter().enumerate() {
        b.push((p0, p1, (k as u128 + 1) * 1_000, (p0..p1).map(|x| u32::try_from(x * 3 % 97).unwrap()).collect()));
        p0 = p1;
    }
    Plan { path: "test".into(), sha: String::new(), seg: 0, days: vec!["2023-03-28".into(), "2023-03-29".into()], nset: 7, nl: p0, digest: digest.into(), args: "e".repeat(64), order: "config-sorted".into(), per_list: 28, cut: "--lists-per-batch test".into(), b, setup: String::new() }
}

#[test]
fn plan_roundtrip_and_refusals() {
    let dg = "a".repeat(64);
    let p = plan_of(&[3, 5, 9, 10], &dg);
    let t = p.text();
    let q = Plan::parse(&t).unwrap();
    assert_eq!(q.text(), t);
    assert_eq!(q.sha, sha256_bytes(t.as_bytes()));
    assert!(Plan::parse(&t.replace("END\n", "")).unwrap_err().contains("truncated"));
    assert!(Plan::parse(&t.replace("LISTS\t10", "LISTS\t11")).unwrap_err().contains("gap"));
    assert!(Plan::parse(&t.replace("B\t1\t3\t5", "B\t1\t4\t5")).is_err());
    assert!(Plan::parse(&t.replace("CUT\t", "NEWTAG\t")).unwrap_err().contains("unknown tag"));
    assert!(Plan::parse(&t.replace(&dg, "short")).unwrap_err().contains("digest"));
    // the run's own set-up checked against the plan: the digest (E6 / F5) and the list ids at every position
    let ord: Vec<usize> = (0..10).map(|x| x * 3 % 97).collect();
    let days = q.days.clone();
    assert!(q.check(0, &days, 7, &dg, &"e".repeat(64), &ord, true, 28).is_ok());
    assert!(q.check(0, &days, 7, &"b".repeat(64), &"e".repeat(64), &ord, true, 28).unwrap_err().contains("list digest"), "another set-up with the same counts must be refused");
    let mut ord2 = ord.clone();
    ord2.swap(1, 2);
    assert!(q.check(0, &days, 7, &dg, &"e".repeat(64), &ord2, true, 28).unwrap_err().contains("batch 0"));
    assert!(q.check(1, &days, 7, &dg, &"e".repeat(64), &ord, true, 28).is_err());
    assert_eq!(q.batch_at(5).unwrap(), (2, 9));
    assert!(q.batch_at(4).is_err());
}

#[test]
fn batch_plan_target_independent() {
    // E2: with a plan the batches are the plan's whatever this target's size_of gate says (lpb); without one, v3.3's uniform cut
    let p = plan_of(&[3, 5, 9, 10], &"a".repeat(64));
    for lpb in [1_usize, 2, 4, 7, 100] {
        let mut b0 = 0;
        let mut got = Vec::new();
        while b0 < p.nl {
            let (k, b1) = crate::report3::batch_next(Some(&p), b0, lpb, p.nl).unwrap();
            got.push((k, b0, b1));
            b0 = b1;
        }
        assert_eq!(got, vec![(0, 0, 3), (1, 3, 5), (2, 5, 9), (3, 9, 10)], "lpb {lpb}");
    }
    assert_eq!(crate::report3::batch_next(None, 8, 4, 10).unwrap(), (2, 10));
}

#[test]
fn shard_walks_assigned_batches() {
    let set: BTreeSet<usize> = [1, 3, 5].into_iter().collect();
    assert!(kit::shard_walks(1, Some(&set), false));
    assert!(kit::shard_walks(5, Some(&set), false), "the last assigned batch is walked");
    assert!(!kit::shard_walks(2, Some(&set), false));
    assert!(!kit::shard_walks(3, Some(&set), true), "a verified file is not walked again");
    assert!(!kit::shard_walks(0, None, false));
}

// ------------------------------------------------------------------------------------------------ E6 / F5: the list digest

#[test]
fn fingerprint_has_list_digest() {
    let mk = |t0: u32| {
        let mut sr = crate::report3::Seg3 { days: vec![0, 1], piece_of_day: vec![0, 0], ..crate::report3::Seg3::default() };
        sr.evs = (0..300_u32).map(|i| crate::acct::SEv { di: u16::try_from(i % 2).unwrap(), ci: i % 17, t: 33_300 + i * 7 + if i == 150 { t0 } else { 0 }, skey: i % 5, tab: crate::trade::EvT::default() }).collect();
        sr.liq = (0..300_u32).map(|i| (i % 11, i % 3)).collect();
        sr.lists = vec![(0..300).step_by(2).collect(), (0..300).step_by(3).collect()];
        sr.uls = vec![(0, 0, 0), (1, 1, u8::MAX)];
        sr.ul_cfgs = vec![vec![(0, 0)], vec![(1, 1)]];
        sr.ul_evs = vec![(0..300).step_by(2).collect(), (0..300).step_by(3).collect()];
        sr
    };
    let (a, b, c) = (mk(0), mk(0), mk(1));
    let d = |sr: &crate::report3::Seg3, th: usize| kit::list_digest(sr, &["2023-03-28".into(), "2023-03-29".into()], &[], &[0, 1], "s", "m", th);
    assert_eq!(d(&a, 1), d(&b, 7), "the same set-up: the same digest whatever the thread count");
    assert_ne!(d(&a, 3), d(&c, 3), "one event's second differs (counts equal): the digest must differ (critic F5)");
    let mut e = mk(0);
    e.lists = vec![(0..300).step_by(2).collect(), (1..301).step_by(3).collect()];
    assert_ne!(d(&a, 2), d(&e, 2), "one config list's members differ (counts equal)");
}

// ------------------------------------------------------------------------------------------------ E1 / F2 / F3: the batch file

fn sum(rng: &mut crate::synth::Rng, list: u32, x: u16, n: u8) -> Sum {
    // keys drawn from a small range so equal keys (ties) are frequent: the total order beyond the key decides
    let net = i64::try_from(rng.below(6)).unwrap() * 1_000 - 2_000;
    Sum { seg: 0, list, x, n, net, trades: u32::try_from(1 + rng.below(3)).unwrap(), wins: 1, mdd: i64::try_from(rng.below(3)).unwrap() * 100, ndays: 2, days_pos: u16::try_from(rng.below(3)).unwrap(), tie: u32::try_from(rng.below(4)).unwrap(), ..Sum::default() }
}

/// The engine's per-batch accumulation, simulated: every account offered to the groups' Tops, the window rows (a candidate or a flag) and
/// the split folds, in the finalize order (list, setting, share, N).
#[derive(Default)]
struct Acc {
    tops: Vec<Tops>,
    win: Vec<(u64, u64, u64, kit::WinBest)>,
    flagged: Vec<crate::report3::Flag>,
    best: Vec<Vec<Option<crate::split18::Best>>>,
    steps: u64,
    accts: u64,
    read: u64,
    offered: u64,
}

impl Acc {
    fn new() -> Self {
        Self { tops: (0..2).map(|_| Tops::new(4)).collect(), win: vec![(0, 0, 0, None); 3], best: vec![vec![None; 1], vec![None; 2]], ..Self::default() }
    }
    fn offer(&mut self, s: &Sum, seed: u64) {
        let mut r = crate::synth::Rng::new(seed, u64::from(s.list) * 1_000 + u64::from(s.x) * 10 + u64::from(s.n));
        self.accts += 1;
        self.steps += 10 + r.below(5);
        self.read += 1;
        for g in 0..2 {
            if g == 0 || s.list % 2 == 0 {
                self.tops.get_mut(g).unwrap().offer(*s);
            }
        }
        for row in 0..3_usize {
            if r.below(4) == 0 {
                self.win[row].2 += 1;
                self.flagged.push(crate::report3::Flag { row: u8::try_from(row).unwrap(), list: s.list, si: u32::from(s.x), l: 1, ni: s.n, sm: *s });
            } else {
                self.win[row].0 += 1;
                let key = (i64::try_from(r.below(3)).unwrap(), 0, 0);
                let f = crate::wf::Fig { net: s.net, trades: 1, ..crate::wf::Fig::default() };
                crate::report3::row_take(&mut self.win[row].3, key, *s, f, f);
            }
        }
        self.offered += 1;
        for si in 0..2_usize {
            for fi in 0..(si + 1) {
                let key = (u16::try_from(r.below(2)).unwrap(), 0, 0, i64::try_from(r.below(2)).unwrap(), 0);
                let tie = (s.tie, s.x, s.list, 0, s.n);
                let slot = &mut self.best[si][fi];
                if slot.as_ref().is_none_or(|b| crate::split18::better_pick(&key, &tie, b)) {
                    *slot = Some(crate::split18::Best { key, tie, sm: *s, learn: crate::split18::Fig18::default(), test: crate::split18::Fig18::default(), after: crate::split18::Fig18::default(), fresh_ok: true });
                }
            }
        }
    }
    fn bck(&self, k: usize, nb: usize, p0: usize, p1: usize, nl: usize, fp: &str, tie52: bool) -> Bck {
        Bck {
            fp: fp.to_owned(),
            k,
            nb,
            p0,
            p1,
            nl,
            tie52,
            splits18: false,
            steps: self.steps,
            accts: self.accts,
            nbk: [(self.accts, self.accts / 2), (0, 0), (0, 0), (0, 0)],
            safe: [1, 0, 2, 0, 3, 4, 5],
            tops: self.tops.iter().map(Tops::canonical).collect(),
            win: self.win.clone(),
            flagged: self.flagged.clone(),
            s18: (self.read, self.offered),
            s18shape: self.best.iter().map(Vec::len).collect(),
            s18b: self.best.iter().enumerate().flat_map(|(si, v)| v.iter().enumerate().filter_map(move |(fi, b)| b.map(|b| (si, fi, b)))).collect(),
            acct: (self.accts, "0".repeat(64)),
            body_sha: String::new(),
            ns_sum: 77,
            s18n: vec![5, 6],
            prov: vec![("source".into(), "S".into()), ("rustc".into(), "R".into()), ("inputs".into(), "I".into()), ("plan".into(), "P".into())],
        }
    }
}

fn accounts(seed: u64, nl: usize) -> Vec<Vec<Sum>> {
    let mut r = crate::synth::Rng::new(seed, 1);
    (0..nl).map(|l| (0..6_u16).flat_map(|x| (0..2_u8).map(move |n| (x, n))).map(|(x, n)| sum(&mut r, u32::try_from(l).unwrap(), x, n)).collect()).collect()
}

fn tops_text(t: &[Tops]) -> Vec<String> {
    t.iter().flat_map(|t| t.canonical().1.into_iter().map(|v| v.iter().map(|(k, s)| format!("{k:?}|{}", crate::ckpt::sum_line(s))).collect::<Vec<_>>().join(";"))).collect()
}

#[test]
fn bck_roundtrip_canonical() {
    crate::sw::set(crate::sw::SPLITS18, false);
    crate::sw::set(crate::sw::TIE52, false);
    let lists = accounts(5, 6);
    let mut a = Acc::new();
    let mut b = Acc::new();
    // the same accounts in two insertion orders: the heaps differ, the canonical body does not (critic F2)
    for l in &lists {
        for s in l {
            a.offer(s, 9);
        }
    }
    for l in lists.iter().rev() {
        for s in l.iter().rev() {
            b.offer(s, 9);
        }
    }
    let (ba, bb) = (a.bck(0, 1, 0, 6, 6, "FP", false), b.bck(0, 1, 0, 6, 6, "FP", false));
    assert_eq!(tops_text(&a.tops), tops_text(&b.tops));
    let (ta, _) = (ba.body(), bb.body());
    let p = Bck::parse(&ba.text()).unwrap();
    assert_eq!(p.body(), ta, "parse . text = the same body");
    assert_eq!(p.body_sha, sha256_bytes(ta.as_bytes()));
    assert_eq!(p.prov_of("source"), Some("S"));
    assert!(!ta.contains("TIMING") && ba.text().contains("TIMING\tns_sum=77"), "timing outside the hashed body");
}

#[test]
fn bck_refuses_unknown_tag_and_shape() {
    crate::sw::set(crate::sw::SPLITS18, false);
    let mut a = Acc::new();
    for l in accounts(6, 3) {
        for s in &l {
            a.offer(s, 3);
        }
    }
    let t = a.bck(1, 4, 2, 3, 9, "FP", false).text();
    assert!(Bck::parse(&t).is_ok());
    // a body edit with its sha256 recomputed (a forged or a later version's file): only the parser's own rules can refuse it
    let forge = |f: &dyn Fn(&str) -> String| -> String {
        let body: String = t.lines().take_while(|l| !l.starts_with("BODY")).map(|l| format!("{l}\n")).collect();
        let bad = f(&body);
        let rest: String = t.lines().skip_while(|l| !l.starts_with("BODY")).skip(1).map(|l| format!("{l}\n")).collect();
        format!("{bad}BODY\t{}\n{rest}", sha256_bytes(bad.as_bytes()))
    };
    assert!(Bck::parse(&forge(&|b: &str| b.to_owned())).is_ok(), "the forge itself keeps a valid file valid");
    // F3: a line without a merge law (a new tag, as a later engine version might write) is REFUSED, never skipped
    assert!(Bck::parse(&forge(&|b: &str| b.replacen("ACCT\t", "S18X\t1\t2\nACCT\t", 1))).is_err(), "an unknown tag must be refused (critic F3)");
    assert!(Bck::parse(&t.replace("END\n", "")).unwrap_err().contains("truncated"));
    // an edited body without its sha256 recomputed fails the sha256
    let edited = t.replacen("SUMS\t", "SUMS\t9", 1);
    assert!(Bck::parse(&edited).unwrap_err().contains("sha256"));
    // a T line of a group / view outside the file's shape: refused (ckpt::load skips it silently)
    assert!(Bck::parse(&forge(&|b: &str| b.replacen("T\t0\t", "T\t7\t", 1))).unwrap_err().contains("outside the file's shape"));
    // an S18B pick of a fold outside the file's S18SHAPE: refused
    assert!(Bck::parse(&forge(&|b: &str| b.replacen("S18B\t1\t0\t", "S18B\t1\t5\t", 1))).is_err());
}

#[test]
fn bck_write_never_overwrites() {
    crate::sw::set(crate::sw::SPLITS18, false);
    let d = tmp("bckw");
    let mut a = Acc::new();
    for s in accounts(8, 1).concat() {
        a.offer(&s, 1);
    }
    let b = a.bck(0, 2, 0, 1, 2, "FP", false);
    let p = kit::write_bck(&d, "idx", &b).unwrap();
    assert!(kit::write_bck(&d, "idx", &b).is_ok(), "a byte-identical body is kept");
    let mut c = b.clone();
    c.steps += 1;
    assert!(kit::write_bck(&d, "idx", &c).unwrap_err().contains("another body"));
    assert!(kit::bck_done(&d, "idx", 0, "FP").unwrap());
    assert!(kit::bck_done(&d, "idx", 0, "OTHER").is_err(), "a verified file of another run is refused");
    // a damaged file is moved aside and counts as not done (walked again)
    std::fs::write(&p, "garbage").unwrap();
    assert!(!kit::bck_done(&d, "idx", 0, "FP").unwrap());
    assert!(!p.exists());
}

// ------------------------------------------------------------------------------------------------ E3: the merge

fn merged_vs_sequential(seed: u64, cuts: &[usize], shards: usize, tie52: bool) {
    crate::sw::set(crate::sw::SPLITS18, false);
    crate::sw::set(crate::sw::TIE52, tie52);
    let nl = *cuts.last().unwrap();
    let lists = accounts(seed, nl);
    // sequential: one pass over every list in batch order (the single machine's cumulative state)
    let mut seq = Acc::new();
    for l in &lists {
        for s in l {
            seq.offer(s, seed);
        }
    }
    // batches: each its own accumulator, written as batch files into shard folders (round robin), then read back in scrambled order
    let base = tmp(&format!("merge{seed}"));
    let dirs: Vec<PathBuf> = (0..shards).map(|i| base.join(format!("s{i}"))).collect();
    let mut p0 = 0;
    let plan = plan_of(cuts, &"c".repeat(64));
    for (k, &p1) in cuts.iter().enumerate() {
        let mut a = Acc::new();
        for l in lists.get(p0..p1).unwrap() {
            for s in l {
                a.offer(s, seed);
            }
        }
        let mut b = a.bck(k, cuts.len(), p0, p1, nl, "FP", tie52);
        b.prov.retain(|x| x.0 != "plan");
        b.prov.push(("plan".into(), plan.sha.clone()));
        let dir = dirs.get(k % shards).unwrap();
        std::fs::create_dir_all(dir).unwrap();
        kit::write_bck(dir, "idx", &b).unwrap();
        p0 = p1;
    }
    let mut ds = dirs.clone();
    ds.reverse();
    let (bs, st, _) = crate::ckmerge::read_all(&ds, "idx", &plan, Some("S")).unwrap();
    assert_eq!(st.files, cuts.len());
    let (c, mst) = crate::ckmerge::merge(bs, cuts.len()).unwrap();
    assert_eq!((c.steps, c.accts, c.next_b0, c.batches), (seq.steps, seq.accts, nl, cuts.len()));
    assert_eq!(mst.safe, [cuts.len() as u64, 0, 2 * cuts.len() as u64, 0, 3 * cuts.len() as u64, 4 * cuts.len() as u64, 5 * cuts.len() as u64]);
    assert_eq!(tops_text(&c.tops), tops_text(&seq.tops), "the merged top-K = the one-pass top-K");
    for (w, s) in c.win.iter().zip(&seq.win) {
        assert_eq!((w.0, w.1, w.2), (s.0, s.1, s.2));
        assert_eq!(w.3.as_ref().map(|x| (x.0, crate::ckpt::sum_line(&x.1))), s.3.as_ref().map(|x| (x.0, crate::ckpt::sum_line(&x.1))));
    }
    let fl = |v: &[crate::report3::Flag]| v.iter().map(|f| (f.row, f.list, f.si, f.ni, crate::ckpt::sum_line(&f.sm))).collect::<Vec<_>>();
    assert_eq!(fl(&c.flagged), fl(&seq.flagged), "flagged rows in batch order = the single run's push order");
    let want = crate::split18::ckpt_lines_of(seq.read, seq.offered, 0, &[], &seq.best);
    let got: Vec<String> = c.sp18.iter().filter(|l| !l.starts_with("S18N") && !l.starts_with("S18\t")).cloned().collect();
    assert_eq!(got, want.iter().filter(|l| !l.starts_with("S18N") && !l.starts_with("S18\t")).cloned().collect::<Vec<_>>());
    assert!(c.sp18.first().unwrap().starts_with(&format!("S18\t{}\t{}\t", seq.read, seq.offered)));
}

#[test]
fn merge_equals_sequential_synthetic() {
    merged_vs_sequential(11, &[2, 3, 7, 8, 12], 4, false);
    merged_vs_sequential(12, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13], 3, false);
    merged_vs_sequential(13, &[13], 1, false);
    // TIE52 (the tie key before the list): equal keys arrive in any identity order, so only the total order keeps the right one
    merged_vs_sequential(14, &[2, 3, 7, 8, 12], 4, true);
    merged_vs_sequential(15, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13], 2, true);
    crate::sw::set(crate::sw::TIE52, false);
}

#[test]
fn merge_refusals() {
    crate::sw::set(crate::sw::SPLITS18, false);
    let plan = plan_of(&[2, 4, 6], &"d".repeat(64));
    let base = tmp("mref");
    let lists = accounts(21, 6);
    let mk = |k: usize, fp: &str, src: &str| {
        let mut a = Acc::new();
        for l in lists.get(k * 2..k * 2 + 2).unwrap() {
            for s in l {
                a.offer(s, 21);
            }
        }
        let mut b = a.bck(k, 3, k * 2, k * 2 + 2, 6, fp, false);
        b.prov = vec![("source".into(), src.into()), ("rustc".into(), "R".into()), ("inputs".into(), "I".into()), ("plan".into(), plan.sha.clone())];
        b
    };
    let (d1, d2, d3) = (base.join("a"), base.join("b"), base.join("c"));
    for d in [&d1, &d2, &d3] {
        std::fs::create_dir_all(d).unwrap();
    }
    kit::write_bck(&d1, "idx", &mk(0, "FP", "S")).unwrap();
    kit::write_bck(&d1, "idx", &mk(2, "FP", "S")).unwrap();
    // gap: batch 1 missing
    let (bs, _, _) = crate::ckmerge::read_all(&[d1.clone()], "idx", &plan, Some("S")).unwrap();
    assert!(crate::ckmerge::merge(bs, 3).unwrap_err().contains("missing"), "a gap must be refused");
    // a byte-identical duplicate is accepted and counted
    kit::write_bck(&d2, "idx", &mk(1, "FP", "S")).unwrap();
    kit::write_bck(&d3, "idx", &mk(1, "FP", "S")).unwrap();
    let (bs, st, _) = crate::ckmerge::read_all(&[d1.clone(), d2.clone(), d3.clone()], "idx", &plan, Some("S")).unwrap();
    assert_eq!((st.files, st.dups, bs.len()), (4, 1, 3));
    assert!(crate::ckmerge::merge(bs, 3).is_ok());
    // a duplicate whose body differs is refused
    let mut x = mk(1, "FP", "S");
    x.steps += 1;
    let d4 = base.join("d");
    std::fs::create_dir_all(&d4).unwrap();
    kit::write_bck(&d4, "idx", &x).unwrap();
    assert!(crate::ckmerge::read_all(&[d1.clone(), d2.clone(), d4.clone()], "idx", &plan, Some("S")).unwrap_err().contains("different bodies"));
    // a foreign engine source (E4) is refused
    let d5 = base.join("e");
    std::fs::create_dir_all(&d5).unwrap();
    kit::write_bck(&d5, "idx", &mk(1, "FP", "OTHER-SOURCE")).unwrap();
    assert!(crate::ckmerge::read_all(&[d1.clone(), d5.clone()], "idx", &plan, None).unwrap_err().contains("provenance"), "another source tree must be refused (E4)");
    assert!(crate::ckmerge::read_all(&[d5.clone()], "idx", &plan, Some("S")).unwrap_err().contains("not the expected"));
    // another fingerprint (another --set, plan or switch) is refused
    let d6 = base.join("f");
    std::fs::create_dir_all(&d6).unwrap();
    kit::write_bck(&d6, "idx", &mk(1, "FP-OTHER", "S")).unwrap();
    let (bs, _, _) = crate::ckmerge::read_all(&[d1.clone(), d6.clone()], "idx", &plan, Some("S")).unwrap();
    assert!(crate::ckmerge::merge(bs, 3).unwrap_err().contains("another run"));
    // a truncated file is refused by name
    let d7 = base.join("g");
    std::fs::create_dir_all(&d7).unwrap();
    let t = mk(1, "FP", "S").text();
    std::fs::write(kit::bck_path(&d7, "idx", 1), &t[..t.len() / 2]).unwrap();
    assert!(crate::ckmerge::read_all(&[d1, d7], "idx", &plan, Some("S")).unwrap_err().contains("truncated"));
    // a D2-1 safety counter that must be 0 (exits at or before booking) is refused (critic F16)
    let mut y = mk(0, "FP", "S");
    y.safe[3] = 1;
    assert!(crate::ckmerge::merge(vec![y, mk(1, "FP", "S"), mk(2, "FP", "S")], 3).unwrap_err().contains("must be 0"));
}

// ------------------------------------------------------------------------------------------------ E8 (F1) / E9 / E12: slices, transport, synthetic

fn synth_roots(seed: u64) -> (crate::cube::Cube, crate::cube::SecTape, crate::cube::Cube, crate::cube::SecTape) {
    let d = crate::cal::days_from_civil(2023, 3, 28);
    let (mc, mt) = crate::synth::day(seed, d, false);
    let (xc, xt) = crate::synth::day(seed, d, true);
    (mc, mt, xc, xt)
}

#[test]
fn synthetic_is_reproducible() {
    let (a, at, _, _) = synth_roots(7);
    let (b, bt, _, _) = synth_roots(7);
    let (c, _, _, _) = synth_roots(8);
    let eb = |x: &crate::cube::Cube| crate::cube::encode_with(x, None);
    assert_eq!(sha256_bytes(&eb(&a)), sha256_bytes(&eb(&b)), "same seed, same bytes");
    assert_eq!(crate::cube::encode_sec_with(&at, a.ymd, None), crate::cube::encode_sec_with(&bt, b.ymd, None));
    assert_ne!(sha256_bytes(&eb(&a)), sha256_bytes(&eb(&c)), "another seed, other bytes");
    // the builder's order (index first), both segments present, seconds rows, and at least one spike (a minute HIGH >= 1.8 x the first open)
    let (c0, c1) = crate::slice::seg_range(&a, 0).unwrap();
    assert!(c0 == 0 && c1 > 0 && c1 < a.cons.len());
    assert!(!at.rows.is_empty());
    let spike = a.cons.iter().enumerate().any(|(ci, k)| k.first_open > 0 && (0..crate::cal::NM).any(|m| i64::from(a.high[crate::cube::Cube::ix(ci, m)]) * 10 >= i64::from(k.first_open) * 18));
    assert!(spike, "the synthetic day carries spikes");
    // the decoded file equals the struct written
    let back = crate::cube::decode_cube(&eb(&a), Path::new("synthetic")).unwrap();
    assert_eq!(back.close, a.close);
    assert_eq!(back.cumval, a.cumval);
}

#[test]
fn slice_equals_full_for_segment() {
    // E8 + critic F1: the index slice of the main root merged with the extra root allows BOTH ranges; every array of every allowed
    // contract equals the whole merged day; a read of a stock contract is counted (it would read zeros with no error)
    let (mc, mt, xc, xt) = synth_roots(31);
    let (ncm, nce) = (mc.cons.len(), xc.cons.len());
    let mut whole = mc.clone();
    let mut wt = mt.clone();
    crate::cube::merge(&mut whole, xc.clone()).unwrap();
    crate::cube::merge_sec(&mut wt, xt.clone());
    for extra_sliced in [false, true] {
        let (b, s) = crate::slice::slice_of(&mc, &mt, 0).unwrap();
        let mut c = crate::cube::decode_cube(&b, Path::new("main")).unwrap();
        let mut t = crate::cube::decode_sec(&s, Path::new("main")).unwrap();
        let (xe, xte) = if extra_sliced {
            let (eb, es) = crate::slice::slice_of(&xc, &xt, 0).unwrap();
            (crate::cube::decode_cube(&eb, Path::new("x")).unwrap(), crate::cube::decode_sec(&es, Path::new("x")).unwrap())
        } else {
            (xc.clone(), xt.clone())
        };
        crate::cube::merge(&mut c, xe).unwrap();
        crate::cube::merge_sec(&mut t, xte);
        let (_, n_idx) = crate::slice::seg_range(&mc, 0).unwrap();
        assert_eq!(c.lim.as_deref(), Some(&[(0, n_idx), (ncm, ncm + nce)][..]), "both index ranges allowed (critic F1)");
        let v0 = kit::SLICE_VIOLATIONS.load(std::sync::atomic::Ordering::Relaxed);
        for ci in (0..n_idx).chain(ncm..ncm + nce) {
            for (a1, b1) in [(c.wvol(ci, 0, 100), whole.wvol(ci, 0, 100)), (c.wval(ci, 3, 300), whole.wval(ci, 3, 300))] {
                assert_eq!(a1, b1, "contract {ci}");
            }
            for m in [0, 17, 200, crate::cal::NM - 1] {
                assert_eq!(c.close_at(ci, m), whole.close_at(ci, m));
            }
            let (x, y) = (t.of(ci), wt.of(ci));
            assert!(x.len() == y.len() && x.iter().zip(y).all(|(p, q)| p.sod == q.sod && p.high == q.high && p.vol == q.vol), "contract {ci}'s seconds");
        }
        let v1 = kit::SLICE_VIOLATIONS.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(v1, v0, "no read inside the allowed ranges is counted (an extra-index contract included: critic F1)");
        // a stock contract: zeros, and the read counted
        assert!(n_idx < ncm);
        assert_eq!(c.wvol(n_idx, 0, 100), 0);
        let v2 = kit::SLICE_VIOLATIONS.load(std::sync::atomic::Ordering::Relaxed);
        assert!(v2 > v1, "a read outside the slice must be counted");
    }
    // the stock slice of the main root: its own range only
    let (b, s) = crate::slice::slice_of(&mc, &mt, 1).unwrap();
    let c = crate::cube::decode_cube(&b, Path::new("stk")).unwrap();
    let t = crate::cube::decode_sec(&s, Path::new("stk")).unwrap();
    let (s0, s1) = crate::slice::seg_range(&mc, 1).unwrap();
    crate::slice::check_slice_equal(&mc, &mt, &c, &t, s0, s1).unwrap();
}

#[test]
fn sfz_roundtrip_all_rows() {
    // E9: the transport file decodes to the slice's exact bytes (both parts); a damaged byte is refused by its section sha256
    all_off();
    // fix:determinism: transport files are a shard's form, made and read under PORTABLE (zstd from SF_ZSTD or PATH); with PORTABLE off this
    // test spawned the Mac path /opt/homebrew/bin/zstd and FAILED on a Linux box (MEASURED on the Mac with that path denied)
    kit::set(kit::PORTABLE, true);
    let (mc, mt, xc, xt) = synth_roots(41);
    for (c, t) in [(&mc, &mt), (&xc, &xt)] {
        let (b, s) = crate::slice::slice_of(c, t, 0).unwrap();
        let z = crate::slice::sfz_encode(&b, &s, 0, c.ymd).unwrap();
        assert!(z.len() < b.len() + s.len(), "compressed");
        let p = Path::new("t.sfz");
        assert_eq!(crate::slice::sfz_decode(&z, p, true).unwrap(), b);
        assert_eq!(crate::slice::sfz_decode(&z, p, false).unwrap(), s, "every seconds row rebuilt");
        let mut bad = z.clone();
        let n = bad.len();
        bad[n - 3] ^= 0x55;
        assert!(crate::slice::sfz_decode(&bad, p, false).is_err() || crate::slice::sfz_decode(&bad, p, true).is_err());
        assert!(crate::slice::sfz_decode(&z[..z.len() - 1], p, true).is_err(), "a truncated file is refused");
    }
    // an UNSORTED second inside a contract is carried exactly (zigzag deltas: exactness never depends on the order)
    let mut t2 = mt.clone();
    if let (Some(a), Some(b)) = (t2.rows.get(0).copied(), t2.rows.get(1).copied()) {
        t2.rows[0] = b;
        t2.rows[1] = a;
    }
    let (b, s) = crate::slice::slice_of(&mc, &t2, 0).unwrap();
    let z = crate::slice::sfz_encode(&b, &s, 0, mc.ymd).unwrap();
    assert_eq!(crate::slice::sfz_decode(&z, Path::new("u.sfz"), false).unwrap(), s);
}

#[test]
fn root_digest_same_for_every_form() {
    // E4 / F4: a root's day digest is the same whether the root holds the whole cube, the slice files or the transport file
    all_off();
    let base = tmp("forms");
    let (whole, sl, tr) = (base.join("whole"), base.join("slice"), base.join("transport"));
    std::fs::create_dir_all(&whole).unwrap();
    let (mc, mt, _, _) = synth_roots(51);
    let d = "2023-03-28";
    std::fs::write(whole.join(format!("{d}.bin")), crate::cube::encode_with(&mc, None)).unwrap();
    std::fs::write(whole.join(format!("{d}.sec.bin")), crate::cube::encode_sec_with(&mt, mc.ymd, None)).unwrap();
    let args = |out: &Path, tr: bool| -> Vec<String> { let mut v: Vec<String> = vec!["spikefade".into(), "cube-slice".into(), whole.display().to_string(), out.display().to_string(), "idx".into()]; if tr { v.push("--transport".into()); } v.push(d.into()); v };
    crate::slice::cmd_cube_slice(&args(&sl, false)).unwrap();
    crate::slice::cmd_cube_slice(&args(&tr, true)).unwrap();
    assert!(crate::slice::cmd_cube_slice(&args(&sl, false)).unwrap_err().contains("never overwritten"));
    let w = crate::slice::root_day_digest(&whole, d, 0).unwrap();
    let s = crate::slice::root_day_digest(&sl, d, 0).unwrap();
    let t = crate::slice::root_day_digest(&tr, d, 0).unwrap();
    assert_eq!(w, s);
    assert_eq!(w, t);
    assert!(w.0 && w.1 && w.2.len() == 64);
    assert_eq!(crate::slice::root_day_digest(&whole, "2023-03-29", 0).unwrap(), (false, false, "-".to_owned()), "an absent day is recorded as absent");
    // the stock segment's digest differs from the index's
    assert_ne!(crate::slice::root_day_digest(&whole, d, 1).unwrap().2, w.2);
    // a slice of the other segment is refused
    assert!(crate::slice::root_day_digest(&sl, d, 1).unwrap_err().contains("segment"));
}

#[test]
fn args_digest_ignores_runtime_and_paths() {
    // E2 / E6: the argument digest counts every result-bearing flag and value, never a folder, a path, a day, the order, or a runtime flag
    let v = |s: &str| -> Vec<String> { s.split(' ').map(str::to_owned).collect() };
    let a = v("spikefade run3 /c1 /o1 2023-03-28 2023-03-29 --set S0 --liq 0,7 --v33 --threads 12 --batch-plan /p --batch-files on --checkpoint /x --var /a/v --cube-extra /e1 --ngrid-idx 5,10,15,20 --batch-accounts 120000");
    let b = v("spikefade run3 /c2 /o2 2023-03-28 2023-03-29 --threads 1 --shard on --batch-set 0,4 --checkpoint /y --batch-plan /q --var /b/v --cube-extra /e2 --cube-transport --ngrid-idx 5,10,15,20 --v33 --liq 0,7 --set S0 --tool-zstd /usr/local/bin/zstd");
    assert_eq!(kit::args_digest(&a), kit::args_digest(&b), "runtime flags, paths, folders and the argument order never count");
    let mut c = a.clone();
    *c.iter_mut().find(|x| *x == "5,10,15,20").unwrap() = "5,10,20,40".to_owned();
    assert_ne!(kit::args_digest(&a), kit::args_digest(&c), "another grid value counts");
    let mut d = a.clone();
    d.extend(["--lot-guard".to_owned(), "off".to_owned()]);
    assert_ne!(kit::args_digest(&a), kit::args_digest(&d), "another switch counts");
    let mut e = a.clone();
    e.retain(|x| x != "--v33");
    assert_ne!(kit::args_digest(&a), kit::args_digest(&e), "a boolean flag counts");
    // the plan check refuses a run with another argument digest, before its first batch
    let mut p = plan_of(&[3, 5], &"a".repeat(64));
    p.args = kit::args_digest(&a);
    let ord: Vec<usize> = (0..5).map(|x| x * 3 % 97).collect();
    let days = p.days.clone();
    assert!(p.check(0, &days, 7, &"a".repeat(64), &kit::args_digest(&b), &ord, true, 28).is_ok());
    assert!(p.check(0, &days, 7, &"a".repeat(64), &kit::args_digest(&c), &ord, true, 28).unwrap_err().contains("argument digest"), "a shard with another flag must be refused");
}
