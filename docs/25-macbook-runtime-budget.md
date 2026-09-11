# Current MacBook execution baseline — 6 September 2026

Use this host when planning local sweep verification and parallel work. These
are observed capacities, not portable engine constants or a promise of total
constant-time computation. Recheck available CPUs, memory pressure, free disk
and active jobs before a larger run; transient observations below expire.

The operator supplied the eleven System Information screenshots in the local
`MacBook Configuration` folder, dated 11 August 2026. The hardware, graphics,
memory and storage views were inspected. Live read-only system queries on
6 September independently confirmed the CPU model, core split and memory.
Device serial numbers, hardware UUIDs and other identifiers are not copied here.

| Resource | Evidence | Consequence for this task |
|---|---|---|
| CPU | Apple M4 Pro; 14 physical/logical CPUs, 10 performance and 4 efficiency | All parallel processes share these cores. Giving every build or sweep 14 workers would oversubscribe the machine. |
| Memory | 51,539,607,552 bytes (48 GiB); screenshots label it 48 GB LPDDR5 | Account for simultaneously retained columns, frontiers, build/link processes, the browser and existing service. Installed RAM is not currently allocatable RAM. |
| GPU | Screenshot reports 20 cores and Metal 4 | The audited sweep uses Rust CPU kernels. No GPU acceleration or measured GPU speedup is claimed. |
| Storage | Screenshot reports a 500.28 GB internal SSD; live APFS data volume had approximately 84 GiB available | Preserve build/test evidence and historical outputs within current free space. The screenshot's older free-space figure is not today's budget. |
| External storage | Operator confirms an 8 TB drive is currently disconnected and will be mounted later; current disk inventories agree | Include it in future capacity planning, but count no usable bytes until its actual mount, free space and store path are verified. No data has been moved. |
| Power | Live machine on AC power at inspection | A power/thermal state is an observation, not a sustained-performance guarantee. No power settings were changed. |
| Memory pressure | System utility reported 68% free at inspection; swap in use was about 252 MiB | These are different OS measurements, not a guarantee that 68% of physical RAM can be reserved by the sweep. Recheck during longer work. |

## Parallel execution policy for this verification

The current four-agent group divides independent logic, storage and integrity
work. New Cargo builds use two build workers per agent and new Rust test runs
use two test threads while these jobs and the existing recovery service
coexist. Already running checks are allowed to finish. The coordinator does
not run multiple Cargo processes against one target directory. CPU-heavy
mutation jobs and benchmarks retain their actual concurrency and host context
in the evidence; these worker limits are an operating choice, not an optimality
measurement or an engine semantic parameter.

The inspected live store and build target currently resolve under the internal
data volume. An external drive adds persistent capacity, not RAM or CPU cores.
Its filesystem, disconnect handling and measured I/O behavior must be checked
before using it for durable journals or assuming it makes parallel scans faster.

Large historical runs must admit their actual bars, frontier, output and
temporary-file budgets together. Finite batches, fixed-width masks, incremental
indexes and resumed work reduce repeated cost. They do not make an exhaustive
search, all persisted history, a cold integrity scan or filesystem latency O(1).
In particular, a 20-core GPU does not increase Rust CPU worker availability.

The clean snapshot's actual engine/vocabulary/indicator/runner ratio suite
passed on this machine; the exact measurements and source commit are in
`docs/14-sweep-readiness-20260906.md`. Whole-workspace benchmarks, peak-memory
profiles and optimal worker counts are separate measurements, not inferred
from these specifications.

## Instrumented verification observation — 8 September 2026

Two Cargo build workers and two test-harness threads do not cap every internal
worker pool to two CPUs. Engine scoped workers and Rayon ranking can use their
own pools. A read-only stack sample during candidate C's instrumented CLI test
showed `runner::rank::offer_part` and `runner::outcome::edge` in ranking, not a
wait on a store lock. One process observation recorded approximately 1,050%
CPU. That is multi-core CPU usage, not 1,050% of the entire 14-core machine.

The complete instrumented CLI unit suite subsequently passed all 1,174 tests
in 1,772.69 seconds; the ordinary candidate C CLI unit suite passed the same
count in 436.51 seconds. Different instrumentation and concurrent work make
these test timings unsuitable as a production latency benchmark. The source
and logs are retained under `target/sweep-audit-20260908/coverage-c/` and
`target/sweep-audit-20260908/source-gates/`.

A contemporaneous filesystem check reported 188 GiB free, while the OS memory
pressure utility reported 87% free. The latter is an OS indicator, not an
allocatable-memory promise. The disconnected external drive remains excluded.
No machine-wide worker allocator or optimal worker-count guarantee is inferred
from these observations. Further parallel jobs must account for internal pools
as well as their top-level command limits.
