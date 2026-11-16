//! `sniper-host`: the run driver. Subcommands: `identity` (laws, data shas), `gate` (the launch gate), `e-sn1` (one context end to
//! end: gate 6), `run` (a family: minutes, or SECONDS 3s / 5s / 10s / 15s / 30s), and the seconds builder's checks `sec-dump` (the fold
//! against `feat-run seconds`), `sec-build` (a seconds family's readings and memory line) and `sec-eq` (every compact kept set
//! against `cands::outer_values`). Every IST label comes from `date`; every value from the data rows.

use std::collections::BTreeMap;
use std::path::PathBuf;

fn args() -> (String, BTreeMap<String, String>) {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().unwrap_or_default();
    let mut m = BTreeMap::new();
    while let Some(k) = it.next() {
        if let Some(name) = k.strip_prefix("--") {
            m.insert(name.to_string(), it.next().unwrap_or_default());
        }
    }
    (cmd, m)
}

fn data_dir(a: &BTreeMap<String, String>) -> PathBuf {
    a.get("data").map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("data"), PathBuf::from)
}

/// `--from` / `--to` (both or neither), IST days.
fn period(a: &BTreeMap<String, String>) -> Result<Option<(i32, i32)>, String> {
    match (a.get("from"), a.get("to")) {
        (None, None) => Ok(None),
        (Some(f), Some(t)) => {
            let f = common_feat::cal::parse_ymd(f).ok_or_else(|| format!("--from {f}: not YYYY-MM-DD"))?;
            let t = common_feat::cal::parse_ymd(t).ok_or_else(|| format!("--to {t}: not YYYY-MM-DD"))?;
            if t < f {
                return Err("--to before --from".to_string());
            }
            Ok(Some((f, t)))
        }
        _ => Err("--from and --to go together".to_string()),
    }
}

fn main() {
    #[cfg(feature = "m_s16")]
    {
        // MUTANT m-S16: the binary calls the mutant file (the build test must catch the symbol it links).
        let p = common_fill::FillPath::default();
        let rows: Vec<common_fill::SecIn> = Vec::new();
        if let Ok(ix) = common_fill::SecIndex::new(&rows, 0, 1_000_000)
            && let Ok(t) = common_fill::Tape::new(&rows, &ix, None)
        {
            let _ = sniper_host::mutant_s16::mutant_entry(&p, &t);
        }
    }
    let (cmd, a) = args();
    let res: Result<Vec<String>, String> = match cmd.as_str() {
        "identity" => sniper_host::esn1::load_inputs(&data_dir(&a)).map(|inp| {
            let mut v = vec![sniper_host::VERSION.to_string(), sniper_host::SEC_VERSION.to_string(), sniper_host::law_line(), sniper_host::ist_now()];
            v.extend(inp.shas.iter().map(|(f, s)| format!("{s}  {f}")));
            v.push(format!("views {}; exits {}; dials {}", inp.views.len(), inp.exits.len(), inp.dials.len()));
            // S7: the instrument rows (law table L1-L8 passed at load).
            v.push(format!("{}  {}", inp.instr_sha.1, inp.instr_sha.0));
            v.push(format!("instrument {:?} underlying {} segment {} block {} s hold {:?}; rows {:?}", inp.instr.kind, inp.instr.underlying(), inp.instr.segment, inp.instr.block_s, inp.instr.hold, inp.instr.rows));
            v
        }),
        "gate" => sniper_host::esn1::load_inputs(&data_dir(&a)).and_then(|inp| {
            let base: u64 = a.get("swap-base").and_then(|x| x.parse().ok()).unwrap_or(0);
            let g = sniper_host::monitor::launch_gate(&inp.run, base, &PathBuf::from(a.get("out").map_or("/Volumes/WD_BLACK", String::as_str)), 0)?;
            Ok(vec![format!("{g:?}")])
        }),
        "e-sn1" => sniper_host::esn1::load_inputs(&data_dir(&a)).and_then(|inp| {
            let threads = a.get("threads").and_then(|x| x.parse().ok()).map_or_else(|| inp.run.num("threads"), Ok)?;
            let spec = sniper_host::esn1::Spec {
                tf: a.get("tf").cloned().unwrap_or_else(|| "3min".to_string()),
                long: a.get("side").is_none_or(|s| s == "long"),
                exit: a.get("exit").cloned().unwrap_or_else(|| "XS-10-30".to_string()),
                out: PathBuf::from(a.get("out").cloned().ok_or("--out DIR is required")?),
                run_id: a.get("run-id").cloned().unwrap_or_else(|| "esn1".to_string()),
                threads,
                swap_base_mb: a.get("swap-base").and_then(|x| x.parse().ok()).ok_or("--swap-base MB is required (the lane's swap base)")?,
            };
            sniper_host::esn1::run_context(&inp, &spec).map(|r| r.lines)
        }),
        "run" => sniper_host::esn1::load_inputs(&data_dir(&a)).and_then(|inp| {
            let threads = a.get("threads").and_then(|x| x.parse().ok()).map_or_else(|| inp.run.num("threads"), Ok)?;
            let exits: Vec<String> = match a.get("exits").map(String::as_str) {
                None | Some("ALL") => inp.exits.iter().map(|e| e.id.clone()).collect(),
                Some(l) => l.split(',').map(str::to_string).collect(),
            };
            let sides = match a.get("sides").map(String::as_str) {
                Some("long") => vec![true],
                Some("short") => vec![false],
                _ => vec![true, false],
            };
            let spec = sniper_host::search::RunSpec {
                tf: a.get("tf").cloned().unwrap_or_else(|| "3min".to_string()),
                sides,
                exits,
                slice: a.get("slice").filter(|s| s.as_str() != "ALL").map(|s| data_dir(&a).join(s)),
                out: PathBuf::from(a.get("out").cloned().ok_or("--out DIR is required")?),
                run_id: a.get("run-id").cloned().unwrap_or_else(|| "run".to_string()),
                threads,
                swap_base_mb: a.get("swap-base").and_then(|x| x.parse().ok()).ok_or("--swap-base MB is required (the lane's swap base)")?,
                data: data_dir(&a),
                n_sweep: a.get("n").filter(|s| s.as_str() != "ALL").map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect()),
                period: period(&a)?,
                // DIALS BUILD: --context-dials on (default) | off (binary k byte for byte). PORTER (secfd): off = secf-a byte for byte.
                context_dials: match a.get("context-dials").map(String::as_str) {
                    None | Some("on") => true,
                    Some("off") => false,
                    Some(x) => return Err(format!("--context-dials {x}: on or off")),
                },
            };
            sniper_host::search::run_family(&inp, &spec)
        }),
        "sec-dump" => period(&a).and_then(|p| {
            let (f, t) = p.ok_or("--from and --to are required")?;
            let out = PathBuf::from(a.get("outdir").cloned().ok_or("--outdir DIR is required")?);
            // S7: the symbol is the instrument data row of the data folder (SNIPER-INSTRUMENT-v1.tsv), never a code constant.
            let inp = sniper_host::esn1::load_inputs(&data_dir(&a))?;
            sniper_host::secrun::sec_dump(a.get("tf").map_or("5s", String::as_str), f, t, &out, inp.instr.underlying())
        }),
        "sec-build" => sniper_host::esn1::load_inputs(&data_dir(&a)).and_then(|inp| {
            let threads = a.get("threads").and_then(|x| x.parse().ok()).unwrap_or(4);
            sniper_host::secrun::sec_build(&inp, a.get("tf").map_or("30s", String::as_str), period(&a)?, threads, a.get("htf").is_none_or(|x| x != "0"))
        }),
        "sec-eq" => sniper_host::esn1::load_inputs(&data_dir(&a)).and_then(|inp| {
            let threads = a.get("threads").and_then(|x| x.parse().ok()).unwrap_or(4);
            // PORTER (secfd): --context-dials on (default, as `run`) | off (secf-a's sec-eq exactly).
            let ctx_on = match a.get("context-dials").map(String::as_str) {
                None | Some("on") => true,
                Some("off") => false,
                Some(x) => return Err(format!("--context-dials {x}: on or off")),
            };
            sniper_host::secrun::sec_eq_run_ctx(&inp, a.get("tf").map_or("60min", String::as_str), period(&a)?, threads, ctx_on)
        }),
        _ => Err("usage: sniper-host run --tf 3min|30s [--sides long|short|both] [--exits ALL|id,id] [--slice SLICE-FIRST-v1.tsv|ALL] [--n ALL|20,5,100] [--context-dials on|off] --out DIR --run-id ID --swap-base MB [--threads N] [--from YYYY-MM-DD --to YYYY-MM-DD (seconds only: a labelled smoke)] | sec-dump --tf 5s --from D --to D --outdir DIR | sec-build --tf 3s [--from D --to D] [--htf 0] [--threads N] | sec-eq --tf 60min|30s [--from D --to D] [--context-dials on|off] [--threads N] | identity | gate [--swap-base MB] | e-sn1 --tf 3min --side long --exit XS-10-30 --out DIR --run-id ID --swap-base MB [--threads N]".to_string()),
    };
    match res {
        Ok(lines) => {
            for l in lines {
                println!("{l}");
            }
        }
        Err(e) => {
            eprintln!("sniper-host: {e}");
            std::process::exit(2);
        }
    }
}
