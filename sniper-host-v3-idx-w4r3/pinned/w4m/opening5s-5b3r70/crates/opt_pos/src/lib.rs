//! opt_pos (first slice of S5 + S6, SNIPER-IDXOPT-PLAN-v3.1): the index option-chain readings and the shared option
//! pricer's intraday path. Read only on every input.
//!
//! * [`freeze`]: the dated NSE freeze table (12:06 note: PASSED) and the one-order maximum (the largest whole-lot multiple
//!   at or below that day's freeze), with each circular's date for `fit_strict` (Q38).
//! * [`ladder`]: the listed strikes per (bhavcopy file, expiry, side) and the lot candidates (`LISTED-CONTRACTS-IDX.tsv`).
//! * [`secs`]: one contract-day's 1-second rows from the store (valid prints only; derivatives halts cut).
//! * [`chain`]: the busiest rule (REQ 49 corrected + note 7, as the idx1s tool chose it: the comparison column) and the
//!   A2 / s23a ITM-fit rule (the headline), the lot rule C17 / D9.
//! * [`newpx`] (RESUMED WAVE 4): the PRODUCTION option seconds = DP-NEWPX-IDXOPT read back under the DATA law (decoder-failure
//!   seconds read the worse of the two sources and are HOLES; failing contract-days are whole-session holes).
//! * [`price`]: one intraday trade on 1-s blocks: the entry buy, the exit sell, the 15:10 square-off to the dated F&O
//!   end, both protection paths (IDX-R3 (i): the worse is booked), REQ 70a pay, charges by `costs::option_clock`.

pub mod acct;
pub mod chain;
// S7 BUILDER wave 4: the host's INTRADAY index trade (the A2 / s23a pick at the decision + price_intraday_holes per N and share level, XC per path).
pub mod intraday;
pub mod freeze;
pub mod ladder;
pub mod newpx;
pub mod pack;
pub mod pos;
pub mod price;
pub mod secs;
pub mod sess;
