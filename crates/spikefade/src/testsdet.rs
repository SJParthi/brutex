//! fix:determinism (16 Nov 2026): the named tests of DET-1 (every subcommand takes the E5 switches; `cube-slice --transport` implies
//! PORTABLE) and DET-2 (`--portable-math`, implied by `--shard on`: every transcendental call through the pinned pure-Rust libm). Each
//! mutant of Cargo.toml's fix:determinism block makes at least one of them FAIL (v34/determinism/build/MUTANTS-DET.txt).
//!
//! The tables under `crates/spikefade/detdata/` (read at run time; `det_gen_tables`, ignored, wrote them on the Mac):
//! - `DET-C2G-ROWS.tsv`: the Greeks inputs of every trade instant of the C2 synthetic set run with the Greeks switch on (`synth --seed
//!   20261116`, 2023-03-28 + 29, main + extra root; the C2 arguments + `--greeks on --tbill`); the plain C2 makes no transcendental call.
//! - `DET-C2G-FN.tsv`: every distinct (function, input) those rows produce under `--portable-math`, with the EXACT f64 bit patterns of the
//!   input and of the pinned libm's result, and the sha256 of the whole ordered call stream.
//! - `DET-C2G-GREEKS.tsv`: per row, the exact bits of every figure `bar_greeks` returns under `--portable-math`.
//!
//! Run on another 64-bit platform, these tests prove (or refute) the same bits there.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::path::Path;

use greeks::hostmath::HostMath;

use crate::kit::{self, Sha256};

fn all_off() {
    for s in 0..kit::COUNT {
        kit::set(s, false);
    }
    greeks::hostmath::set_thread(None);
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sfdet-{name}-{}-{}", std::process::id(), kit::utc_now()));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn v(s: &str) -> Vec<String> {
    s.split(' ').map(str::to_owned).collect()
}

fn detdata(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("detdata").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

// ------------------------------------------------------------------------------------------------ host math for the tests (this thread)

thread_local! {
    static N_EXP: Cell<u64> = const { Cell::new(0) };
    static N_LN: Cell<u64> = const { Cell::new(0) };
    static REC: RefCell<Vec<(u8, u64, u64)>> = const { RefCell::new(Vec::new()) };
}

fn counting_exp(x: f64) -> f64 {
    N_EXP.with(|c| c.set(c.get() + 1));
    libm::exp(x)
}

fn counting_ln(x: f64) -> f64 {
    N_LN.with(|c| c.set(c.get() + 1));
    libm::log(x)
}

/// The pinned libm, counting each call.
const COUNTING: HostMath = HostMath { exp: counting_exp, ln: counting_ln, name: "the pinned libm, counted (test)" };

fn recording_exp(x: f64) -> f64 {
    let y = libm::exp(x);
    REC.with(|r| r.borrow_mut().push((b'e', x.to_bits(), y.to_bits())));
    y
}

fn recording_ln(x: f64) -> f64 {
    let y = libm::log(x);
    REC.with(|r| r.borrow_mut().push((b'l', x.to_bits(), y.to_bits())));
    y
}

/// The pinned libm, recording every call (function, input bits, output bits) in order.
const RECORDING: HostMath = HostMath { exp: recording_exp, ln: recording_ln, name: "the pinned libm, recorded (test)" };

fn counts() -> (u64, u64) {
    (N_EXP.with(Cell::get), N_LN.with(Cell::get))
}

fn reset_counts() {
    N_EXP.with(|c| c.set(0));
    N_LN.with(|c| c.set(0));
}

// ------------------------------------------------------------------------------------------------ DET-1: the E5 switches everywhere

#[test]
fn det_common_args_strip_and_apply() {
    all_off();
    let a = v("spikefade cube-slice /r /o idx --portable on --tool-zstd /x/zstd --transport 2023-03-28 --portable-math off --tool-gzip /y/gzip 2023-03-29");
    let (rest, p, m, t) = kit::split_common(&a).unwrap();
    assert_eq!(rest, v("spikefade cube-slice /r /o idx --transport 2023-03-28 2023-03-29"), "the switches are removed, every other argument kept in order");
    assert_eq!((p, m), (Some(true), Some(false)));
    assert_eq!(t.get("zstd").map(String::as_str), Some("/x/zstd"));
    assert_eq!(t.get("gzip").map(String::as_str), Some("/y/gzip"));
    // refusals, each by name
    assert!(kit::split_common(&v("spikefade synth --portable maybe")).unwrap_err().contains("--portable on|off"));
    assert!(kit::split_common(&v("spikefade synth --portable-math")).unwrap_err().contains("--portable-math on|off"));
    assert!(kit::split_common(&v("spikefade synth --tool-zstd --portable on")).unwrap_err().contains("--tool-zstd PATH"));
    assert!(kit::split_common(&v("spikefade synth --tool-curl /c")).unwrap_err().contains("no such tool"));
    // applied: PORTABLE on, the tool path in force (no Mac path), the subcommand sees the rest only
    let got = kit::common_args(a.clone()).unwrap();
    assert_eq!(got, rest);
    assert!(kit::on(kit::PORTABLE) && !kit::on(kit::PMATH));
    assert_eq!(kit::tool("zstd"), "/x/zstd");
    // nothing given: the arguments unchanged and every switch off (P0)
    all_off();
    let plain = v("spikefade bck-check /d idx");
    assert_eq!(kit::common_args(plain.clone()).unwrap(), plain);
    assert!(!kit::any());
    assert_eq!(kit::tool("unzip"), "/usr/bin/unzip", "PORTABLE off: the v3.3 Mac path (P0)");
    // --portable-math alone
    assert_eq!(kit::common_args(v("spikefade greeks-tsv --portable-math on")).unwrap(), v("spikefade greeks-tsv"));
    assert!(kit::on(kit::PMATH) && !kit::on(kit::PORTABLE));
    all_off();
}

fn synth_root(name: &str) -> (std::path::PathBuf, &'static str) {
    let d = "2023-03-28";
    let root = tmp(name).join("whole");
    std::fs::create_dir_all(&root).unwrap();
    let (mc, mt) = crate::synth::day(61, crate::cal::days_from_civil(2023, 3, 28), false);
    std::fs::write(root.join(format!("{d}.bin")), crate::cube::encode_with(&mc, None)).unwrap();
    std::fs::write(root.join(format!("{d}.sec.bin")), crate::cube::encode_sec_with(&mt, mc.ymd, None)).unwrap();
    (root, d)
}

#[test]
fn det_cube_slice_transport_portable() {
    // the boxes' C2 failure: cube-slice --transport spawned /opt/homebrew/bin/zstd whatever PATH / SF_ZSTD / --tool-zstd said
    all_off();
    let (root, d) = synth_root("tr");
    let out = root.parent().unwrap().join("transport");
    // a transport slice implies PORTABLE: zstd is the configured tool, never the Mac path
    let a = |o: &Path, tr: bool, day: &str| -> Vec<String> {
        let mut x = v(&format!("spikefade cube-slice {} {} idx", root.display(), o.display()));
        if tr {
            x.push("--transport".into());
        }
        x.push(day.into());
        x
    };
    crate::slice::cmd_cube_slice(&a(&out, true, "2023-03-30")).unwrap();
    assert!(kit::on(kit::PORTABLE), "cube-slice --transport implies PORTABLE (DET-1)");
    assert_ne!(kit::tool("zstd"), "/opt/homebrew/bin/zstd");
    let want = std::env::var("SF_ZSTD").ok().filter(|x| !x.is_empty()).unwrap_or_else(|| "zstd".to_owned());
    assert_eq!(kit::tool("zstd"), want, "under PORTABLE: SF_ZSTD, else the bare name (PATH)");
    // the configured tool is the one spawned: a missing --tool-zstd is refused BY ITS NAME (the Mac path never appears)
    all_off();
    let mut b = a(&out, true, d);
    b.extend(v("--tool-zstd /nonexistent/det/zstd"));
    let b = kit::common_args(b).unwrap();
    let e = crate::slice::cmd_cube_slice(&b).unwrap_err();
    assert!(e.contains("/nonexistent/det/zstd") && !e.contains("/opt/homebrew"), "{e}");
    // an E8 slice (no transport) leaves PORTABLE off (P0) and needs no tool
    all_off();
    crate::slice::cmd_cube_slice(&a(&root.parent().unwrap().join("slice"), false, d)).unwrap();
    assert!(!kit::on(kit::PORTABLE));
    all_off();
}

// ------------------------------------------------------------------------------------------------ DET-2: --portable-math

#[test]
fn det_pmath_switch_and_shard() {
    all_off();
    // shard mode turns it on; an ordinary run only by its own flag (BFILES never: the Mac's E7 reference keeps v3.3's Greeks)
    assert!(kit::pmath_of(None, true), "--shard on implies PMATH");
    assert!(kit::pmath_of(Some(false), true), "a shard cannot turn it off (as PORTABLE / STRICT)");
    assert!(kit::pmath_of(Some(true), false));
    assert!(!kit::pmath_of(None, false), "off by default (P0)");
    let a = |x: &[&str]| -> Vec<String> { ["spikefade", "run3"].iter().chain(x.iter()).map(|s| (*s).to_owned()).collect() };
    kit::from_args(&a(&["--day-major", "--portable-math", "on"])).unwrap();
    assert!(kit::on(kit::PMATH) && !kit::on(kit::PORTABLE));
    assert_eq!(greeks::hostmath::active().map(|m| m.name), Some(crate::det::LIBM_NAME), "PMATH routes the Greeks crate");
    all_off();
    assert!(kit::from_args(&a(&["--day-major"])).unwrap().is_empty() && !kit::on(kit::PMATH));
    assert!(greeks::hostmath::active().is_none(), "off: the platform's functions");
    assert!(kit::from_args(&a(&["--portable-math", "maybe"])).unwrap_err().contains("--portable-math on|off"));
    // the argument digest never counts it (a resume with it still passes the plan check)
    assert_eq!(kit::args_digest(&a(&["/c", "/o", "2023-03-28", "--v33", "--portable-math", "on"])), kit::args_digest(&a(&["/c", "/o", "2023-03-28", "--v33"])));
    all_off();
}

#[test]
fn det_pmath_routes_every_call_site() {
    // every exp / ln of the Greeks crate goes through the host implementation: 2 discount factors + the density + both distribution
    // terms = 5 exp and d1's ln = 1 per model evaluation; a site left on the platform's function (a call-site mutant) changes a count
    all_off();
    let c = greeks::Contract { spot: 2_500_000.0, strike: 2_450_000.0, years_to_expiry: 7.0 / 365.0, rate: 0.065, carry: 0.0 };
    for kind in [greeks::OptionKind::Call, greeks::OptionKind::Put] {
        greeks::hostmath::set_thread(Some(COUNTING));
        reset_counts();
        let g = c.greeks(0.15, kind).unwrap();
        assert_eq!(counts(), (5, 1), "{kind:?}: exp x5 (carry, strike, density, N(d1), N(d2)) and ln x1 (d1)");
        // the routed result is the pinned libm's; with nothing installed it is the platform's (std), the old kit
        greeks::hostmath::set_thread(None);
        let p = c.greeks(0.15, kind).unwrap();
        assert!((g.price - p.price).abs() <= 1e-9 * p.price.abs().max(1.0));
        assert_eq!(counts(), (5, 1), "nothing installed: no host call");
        // the solver: every evaluation routed
        greeks::hostmath::set_thread(Some(COUNTING));
        reset_counts();
        let iv = c.implied_volatility(g.price, kind).unwrap();
        let (ne, nl) = counts();
        // one check (2 discount factors), then per model evaluation d1's ln, the density's exp and 0-2 distribution exps (none in a
        // saturated tail)
        assert_eq!(nl, u64::from(iv.iterations), "solver: one ln per model evaluation");
        assert!(nl >= 3 && ne >= 2 + nl && ne <= 2 + 3 * nl, "solver: {ne} exp, {nl} ln");
    }
    // PMATH through the kit switch installs the pinned libm (process-wide in a release build; this thread here)
    all_off();
    kit::set(kit::PMATH, true);
    assert_eq!(greeks::hostmath::active().map(|m| m.name), Some(crate::det::LIBM_NAME));
    // gk.rs's rate: ln_1p through det under PMATH, exactly the pinned libm's bits
    crate::det::LN1P_PORTABLE.with(|n| n.set(0));
    let r = crate::gk::continuous(55_199);
    assert_eq!(crate::det::LN1P_PORTABLE.with(Cell::get), 1, "the rate's ln_1p is routed");
    let tau = 91.0 / 365.0;
    assert_eq!(r.to_bits(), (libm::log1p(f64::from(55_199_u32) / 1_000_000.0 * tau) / tau).to_bits());
    kit::set(kit::PMATH, false);
    assert!(greeks::hostmath::active().is_none());
    all_off();
}

#[test]
fn det_off_is_the_platform() {
    // P0 at the function level: with the switch off every routed function IS std's (the same call on the same argument), bit for bit;
    // the pinned libm differs from this platform's libm on some of these inputs (on the Mac: 9.3% of exp results), so a switch-off path
    // that took the pinned libm would show here
    all_off();
    let mut n = 0_u64;
    let mut x = -0.75_f64;
    while x < 12.0 {
        for y in [x, x * 1e-3, -x * 0.5, x * 37.0] {
            assert_eq!(greeks::hostmath::exp(y).to_bits(), y.exp().to_bits(), "exp({y})");
            if y > 0.0 {
                assert_eq!(greeks::hostmath::ln(y).to_bits(), y.ln().to_bits(), "ln({y})");
            }
            if y > -1.0 {
                assert_eq!(crate::det::ln_1p(y).to_bits(), y.ln_1p().to_bits(), "ln_1p({y})");
            }
            n += 1;
        }
        x += 1.0 / 4_093.0;
    }
    // the rate's whole input domain (T-bill yields 0.0000% .. 20.0000%)
    for ye4 in 0..=200_000_u32 {
        let s = f64::from(ye4) / 1_000_000.0 * (91.0 / 365.0);
        assert_eq!(crate::det::ln_1p(s).to_bits(), s.ln_1p().to_bits(), "ln_1p at yield e4 {ye4}");
    }
    assert!(n > 100_000);
}

#[test]
fn det_greeks_words_under_pmath() {
    // the report lines that name the Greeks code: v3.3's words byte for byte when off; the pinned libm named under PMATH
    all_off();
    // the words are printed from report3.rs and (checklist row 13 under v3.3's --fix33g) from fixg.rs's ITEM13
    let src = [include_str!("report3.rs"), include_str!("fixg.rs")].concat();
    assert!(crate::fixg::ITEM13.starts_with(crate::det::LABELS[4].0), "checklist row 13's v3.3 words start with the 5th label");
    for (old, new) in crate::det::LABELS {
        assert!(src.contains(old), "the v3.3 words exist in report3.rs / fixg.rs: {old}");
        assert_eq!(crate::det::greeks_label(old), old, "off: unchanged");
        assert!(new.contains("libm 0.2.16"));
    }
    kit::set(kit::PMATH, true);
    for (old, new) in crate::det::LABELS {
        let line = format!("x {old} y");
        assert_eq!(crate::det::greeks_label(&line), format!("x {new} y"), "under PMATH");
    }
    all_off();
}

// ------------------------------------------------------------------------------------------------ DET-2: the exact bits (tables)

/// A greeks-tsv row: key, spot, strike, premium (paisa), tenor s, call, yield e4, units.
type Row = (String, i64, i64, i64, i64, bool, u32, i64);

fn rows(name: &str) -> Vec<Row> {
    detdata(name)
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            let n = |i: usize| f[i].trim().parse::<i64>().unwrap();
            (f[0].to_owned(), n(1), n(2), n(3), n(4), f[5].trim().starts_with('C'), f[6].trim().parse().unwrap(), n(7))
        })
        .collect()
}

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

/// Every figure of one `bar_greeks` result, f64s as exact bits.
fn greeks_line(r: &Row) -> String {
    let g = crate::gk::bar_greeks(r.1, r.2, r.3, r.4, r.5, r.6, r.7);
    let fs = [g.rate, g.years, g.iv, g.uncertainty, g.delta, g.gamma_r, g.theta_d, g.vega_p, g.rho_p, g.pos_delta, g.pos_gamma, g.pos_theta, g.pos_vega, g.pos_rho];
    format!("{}\t{}\t{}\t{}\t{}\t{}", r.0, g.ok, g.method, g.iterations, fs.iter().map(|&x| hex(x)).collect::<Vec<_>>().join("\t"), g.refusal)
}

/// The ordered call stream of every row under the pinned libm: (function, input bits, output bits), `ln_1p` included.
fn stream(rs: &[Row]) -> Vec<(u8, u64, u64)> {
    REC.with(|r| r.borrow_mut().clear());
    // as a --portable-math run: the rate's ln_1p through det (the pinned libm); then the Greeks crate through the RECORDING copy of the
    // pinned libm (set after PMATH, whose own hook installed the plain one on this thread)
    kit::set(kit::PMATH, true);
    greeks::hostmath::set_thread(Some(RECORDING));
    for r in rs {
        let s = f64::from(r.6) / 1_000_000.0 * (91.0 / 365.0);
        REC.with(|v| v.borrow_mut().push((b'p', s.to_bits(), libm::log1p(s).to_bits())));
        let _ = greeks_line(r);
    }
    greeks::hostmath::set_thread(None);
    kit::set(kit::PMATH, false);
    REC.with(|r| r.borrow_mut().clone())
}

fn stream_sha(s: &[(u8, u64, u64)]) -> String {
    let mut h = Sha256::new();
    for (f, x, y) in s {
        h.update(&[*f]);
        h.update(&x.to_le_bytes());
        h.update(&y.to_le_bytes());
    }
    h.hex()
}

fn fn_name(f: u8) -> &'static str {
    match f {
        b'e' => "exp",
        b'l' => "ln",
        _ => "ln_1p",
    }
}

#[test]
fn det_c2g_fn_table_bits() {
    // every distinct (function, input) of the C2 set's Greeks: the pinned libm gives the table's exact bits on THIS platform, through
    // every route (libm itself, the Greeks crate's hostmath, det::ln_1p under PMATH); and the whole ordered call stream of the rows
    // (each input depends on earlier results) has the table's sha256
    all_off();
    let t = detdata("DET-C2G-FN.tsv");
    let want_sha = t.lines().find_map(|l| l.strip_prefix("# stream sha256 ")).map(|s| s.split_whitespace().next().unwrap_or("").to_owned()).unwrap();
    let mut n = 0;
    for l in t.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let f: Vec<&str> = l.split('\t').collect();
        let x = f64::from_bits(u64::from_str_radix(f[1], 16).unwrap());
        let y = u64::from_str_radix(f[2], 16).unwrap();
        let got = match f[0] {
            "exp" => [libm::exp(x).to_bits(), {
                greeks::hostmath::set_thread(Some(crate::det::LIBM));
                let r = greeks::hostmath::exp(x).to_bits();
                greeks::hostmath::set_thread(None);
                r
            }],
            "ln" => [libm::log(x).to_bits(), {
                greeks::hostmath::set_thread(Some(crate::det::LIBM));
                let r = greeks::hostmath::ln(x).to_bits();
                greeks::hostmath::set_thread(None);
                r
            }],
            "ln_1p" => [libm::log1p(x).to_bits(), {
                kit::set(kit::PMATH, true);
                let r = crate::det::ln_1p(x).to_bits();
                kit::set(kit::PMATH, false);
                r
            }],
            o => panic!("function {o}"),
        };
        assert_eq!(got, [y, y], "{} of {:016x}", f[0], x.to_bits());
        n += 1;
    }
    assert!(n > 100, "{n} table rows");
    let s = stream(&rows("DET-C2G-ROWS.tsv"));
    assert_eq!(stream_sha(&s), want_sha, "the C2 set's ordered call stream ({} calls)", s.len());
    all_off();
}

#[test]
fn det_c2g_greeks_rows_bits() {
    // every figure of every C2-set row under --portable-math (the kit switch, as a run sets it): the table's exact bits
    all_off();
    let rs = rows("DET-C2G-ROWS.tsv");
    let t = detdata("DET-C2G-GREEKS.tsv");
    let want: Vec<&str> = t.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).collect();
    assert_eq!(want.len(), rs.len());
    kit::set(kit::PMATH, true);
    for (r, w) in rs.iter().zip(&want) {
        assert_eq!(greeks_line(r), *w, "row {}", r.0);
    }
    all_off();
}

/// Writes the three tables (ignored; run once on the Mac: `SF_DET_GEN_OUT=<dir> cargo test --release -p spikefade -- --ignored
/// det_gen_tables`), from `DET-C2G-ROWS.tsv`.
#[test]
#[ignore = "writes the tables; run by hand"]
fn det_gen_tables() {
    all_off();
    let out = std::env::var("SF_DET_GEN_OUT").unwrap();
    let rs = rows("DET-C2G-ROWS.tsv");
    let s = stream(&rs);
    let distinct: BTreeSet<(u8, u64, u64)> = s.iter().copied().collect();
    let mut fnt = format!("# fix:determinism: every distinct (function, input) of DET-C2G-ROWS.tsv's {} rows under --portable-math, the pinned libm 0.2.16's result; f64 as exact IEEE-754 bits (hex)\n# stream sha256 {} ({} calls in order, ln_1p included)\n# function\tinput_bits\toutput_bits\tinput\toutput\n", rs.len(), stream_sha(&s), s.len());
    for (f, x, y) in &distinct {
        fnt.push_str(&format!("{}\t{x:016x}\t{y:016x}\t{:e}\t{:e}\n", fn_name(*f), f64::from_bits(*x), f64::from_bits(*y)));
    }
    std::fs::write(Path::new(&out).join("DET-C2G-FN.tsv"), fnt).unwrap();
    kit::set(kit::PMATH, true);
    let mut gt = String::from("# fix:determinism: bar_greeks of every DET-C2G-ROWS.tsv row under --portable-math; f64 as exact IEEE-754 bits (hex)\n# key\tok\tmethod\titerations\trate\tyears\tiv\tuncertainty\tdelta\tgamma_r\ttheta_d\tvega_p\trho_p\tpos_delta\tpos_gamma\tpos_theta\tpos_vega\tpos_rho\trefusal\n");
    for r in &rs {
        gt.push_str(&greeks_line(r));
        gt.push('\n');
    }
    std::fs::write(Path::new(&out).join("DET-C2G-GREEKS.tsv"), gt).unwrap();
    all_off();
}
