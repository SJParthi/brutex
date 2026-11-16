//! Monitoring (plan 13): the launch gate and the heartbeat. The gate holds the lanes' memory gate (start nothing new at memory
//! pressure level 4, or when swap grew by more than 2 GB since the base), the plan's free-swap row (F30) and the disk floor
//! (1,090 GiB, S11), each a RUN data row. Readings come from `sysctl` and `df` (no privilege); an unreadable value HOLDS.

use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::rows::RunRows;

fn out(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd).args(args).output().ok().and_then(|o| String::from_utf8(o.stdout).ok())
}

/// The memory pressure level (`kern.memorystatus_vm_pressure_level`: 1 normal, 2 warn, 4 critical).
#[must_use]
pub fn pressure() -> Option<u32> {
    out("sysctl", &["-n", "kern.memorystatus_vm_pressure_level"]).and_then(|s| s.trim().parse().ok())
}

fn mb(field: &str) -> Option<u64> {
    let f = field.trim().trim_end_matches('M');
    let (i, _) = f.split_once('.').unwrap_or((f, "0"));
    i.parse().ok()
}

/// Swap (used MB, free MB) from `vm.swapusage`.
#[must_use]
pub fn swap() -> Option<(u64, u64)> {
    let s = out("sysctl", &["vm.swapusage"])?;
    parse_swap(&s)
}

/// Parses `vm.swapusage: total = 36864.00M  used = 35966.81M  free = 897.19M  (encrypted)`.
#[must_use]
pub fn parse_swap(s: &str) -> Option<(u64, u64)> {
    let pick = |name: &str| -> Option<u64> {
        let i = s.find(&format!("{name} = "))?;
        let rest = s.get(i + name.len() + 3..)?;
        mb(rest.split_whitespace().next()?)
    };
    Some((pick("used")?, pick("free")?))
}

/// Free GiB of the volume holding `path` (`df -g`).
#[must_use]
pub fn disk_free_gib(path: &Path) -> Option<u64> {
    let s = out("df", &["-g", &path.to_string_lossy()])?;
    s.lines().nth(1)?.split_whitespace().nth(3)?.parse().ok()
}

/// The gate's reading.
#[derive(Clone, Debug, Default)]
pub struct Gate {
    /// Every check passed.
    pub ok: bool,
    /// The named reasons it holds.
    pub hold: Vec<String>,
    /// The readings.
    pub pressure: Option<u32>,
    /// See `pressure`.
    pub swap_used_mb: Option<u64>,
    /// See `pressure`.
    pub swap_free_mb: Option<u64>,
    /// See `pressure`.
    pub disk_free_gib: Option<u64>,
}

/// The launch gate against the RUN rows, the swap base (MB used when the lane started) and the volume of the run's writes
/// (`extra_gib` its estimated writes).
///
/// # Errors
/// A RUN row missing.
pub fn launch_gate(run: &RunRows, swap_base_mb: u64, volume: &Path, extra_gib: u64) -> Result<Gate, String> {
    let mut g = Gate { pressure: pressure(), disk_free_gib: disk_free_gib(volume), ..Gate::default() };
    if let Some((u, f)) = swap() {
        g.swap_used_mb = Some(u);
        g.swap_free_mb = Some(f);
    }
    let pmax: u32 = run.num("pressure_max")?;
    let growth: u64 = run.num("swap_growth_max_mb")?;
    let free_min: u64 = run.num("swap_free_min_mb")?;
    let floor: u64 = run.num("disk_floor_gib")?;
    match g.pressure {
        Some(p) if p <= pmax => {}
        Some(p) => g.hold.push(format!("memory pressure level {p} > {pmax}")),
        None => g.hold.push("memory pressure unreadable".to_string()),
    }
    match g.swap_used_mb {
        Some(u) if u <= swap_base_mb + growth => {}
        Some(u) => g.hold.push(format!("swap used {u} MB > base {swap_base_mb} + {growth} MB")),
        None => g.hold.push("swap unreadable".to_string()),
    }
    if let Some(f) = g.swap_free_mb
        && f < free_min
    {
        g.hold.push(format!("free swap {f} MB < {free_min} MB (F30)"));
    }
    match g.disk_free_gib {
        Some(d) if d >= floor + extra_gib => {}
        Some(d) => g.hold.push(format!("disk free {d} GiB < floor {floor} + writes {extra_gib} GiB")),
        None => g.hold.push("disk free unreadable".to_string()),
    }
    g.ok = g.hold.is_empty();
    Ok(g)
}

/// The process's resident size, MB (`ps`), when readable.
#[must_use]
pub fn rss_mb() -> Option<u64> {
    out("ps", &["-o", "rss=", "-p", &std::process::id().to_string()]).and_then(|s| s.trim().parse::<u64>().ok()).map(|k| k / 1024)
}

/// Appends one heartbeat line (`HEARTBEAT.tsv`; the header on first write).
///
/// # Errors
/// An I/O error.
pub fn heartbeat(path: &Path, fields: &[(&str, String)]) -> Result<(), String> {
    let new = !path.exists();
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path).map_err(|e| e.to_string())?;
    if new {
        let h: Vec<&str> = std::iter::once("ist").chain(fields.iter().map(|x| x.0)).collect();
        writeln!(f, "{}", h.join("\t")).map_err(|e| e.to_string())?;
    }
    let v: Vec<String> = std::iter::once(crate::ist_now()).chain(fields.iter().map(|x| x.1.clone())).collect();
    writeln!(f, "{}", v.join("\t")).map_err(|e| e.to_string())
}
