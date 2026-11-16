//! `Lists::offer_learn(.., run)` (decision D-S14, addition 2): a bounded best-k list of (key, code) whose entries may be RUNS of
//! consecutive codes with one key. Offering a run of n codes once is exactly n single offers (property OR-EQ): the list keeps the
//! smallest codes, holds at most k codes, and adds n to the offer and tie counters (P18: no silent dedupe).
//!
//! Cost (SC-OFFER, plan 8.4): one early-reject compare against the worst kept entry; on accept O(log k) in the ordered map, plus at
//! most one split of the worst run when the bound is passed (a NAMED bounded exception: log2 of k = 20,000 is 15 steps).

use std::collections::BTreeMap;

use crate::acc::Key;

/// A code: 256 bits, little-endian limbs (`sniper_kernel::code::Code`'s layout; `common_wf::Code` on the lift).
pub type Code = [u64; 4];

fn code_cmp(a: &Code, b: &Code) -> std::cmp::Ordering {
    for i in (0..4).rev() {
        match a.get(i).cmp(&b.get(i)) {
            std::cmp::Ordering::Equal => {}
            x => return x,
        }
    }
    std::cmp::Ordering::Equal
}

/// A code as a big-endian sortable value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ck(pub Code);

impl Ord for Ck {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        code_cmp(&self.0, &o.0)
    }
}

impl PartialOrd for Ck {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}

fn plus(c: Code, x: u64) -> Code {
    let mut out = c;
    let mut carry = u128::from(x);
    for limb in &mut out {
        let s = u128::from(*limb) + carry;
        *limb = u64::try_from(s & u128::from(u64::MAX)).unwrap_or(0);
        carry = s >> 64;
    }
    out
}

/// A bounded best-k list (the head is the best: the smallest key, then the smallest code).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lists {
    k: u64,
    held: BTreeMap<(Key, Ck), u64>,
    /// The worst held entry (`held`'s last key), kept in step with every change of `held` (speed build: the reject test of
    /// every offer reads it without walking the map).
    worst: Option<(Key, Ck)>,
    total: u64,
    /// Codes offered (runs counted by their length).
    pub offered: u64,
    /// Entries (runs) offered.
    pub runs: u64,
    /// Accepted entries.
    pub accepts: u64,
    /// The best key ever offered and the codes offered with it (ties at #1, P18).
    pub top: Option<(Key, u64)>,
}

impl Lists {
    /// An empty list of at most `k` codes (`k` is a data row: `stage2_top` = 20,000).
    #[must_use]
    pub fn new(k: u64) -> Self {
        Self { k: k.max(1), held: BTreeMap::new(), worst: None, total: 0, offered: 0, runs: 0, accepts: 0, top: None }
    }

    /// Offers the run of `n` codes `code .. code + n - 1`, all with key `key`. Returns the step count.
    pub fn offer_learn(&mut self, key: Key, code: Code, n: u64) -> u64 {
        if n == 0 {
            return 1;
        }
        self.offered += n;
        self.runs += 1;
        match &mut self.top {
            Some((tk, tn)) if *tk == key => *tn += n,
            Some((tk, _)) if key < *tk => self.top = Some((key, n)),
            None => self.top = Some((key, n)),
            _ => {}
        }
        let mut steps = 1_u64;
        if self.total >= self.k
            && let Some(worst) = &self.worst
            && (key, Ck(code)) >= *worst
        {
            return steps;
        }
        self.held.insert((key, Ck(code)), n);
        self.accepts += 1;
        self.total += n;
        steps += 15;
        while self.total > self.k {
            let Some((wk, wn)) = self.held.pop_last() else { break };
            let over = self.total - self.k;
            if wn > over {
                self.held.insert(wk, wn - over);
                self.total -= over;
            } else {
                self.total -= wn;
            }
            steps += 1;
        }
        self.worst = self.held.last_key_value().map(|(k, _)| *k);
        steps
    }

    /// One single offer (a run of 1).
    pub fn offer(&mut self, key: Key, code: Code) -> u64 {
        self.offer_learn(key, code, 1)
    }

    /// True when EVERY key whose leading parts are `prefix` would be rejected by [`Lists::offer_learn`] now, whatever its later
    /// parts and its code: the list is full and `prefix` is strictly greater than the worst kept key's leading parts (then
    /// `(key, code) > worst`). An empty prefix never rejects. Such a key is also strictly worse than the best key ever offered
    /// (the best key is always kept, so it is at most the worst kept key), so the ties at #1 cannot change either.
    #[must_use]
    pub fn rejects_prefix(&self, prefix: &[i64]) -> bool {
        if prefix.is_empty() || self.total < self.k {
            return false;
        }
        let Some((worst, _)) = &self.worst else { return false };
        for (i, &x) in prefix.iter().enumerate() {
            match x.cmp(&worst.0.get(i).copied().unwrap_or(0)) {
                std::cmp::Ordering::Greater => return true,
                std::cmp::Ordering::Less => return false,
                std::cmp::Ordering::Equal => {}
            }
        }
        false
    }

    /// Counts a run of `n` codes whose key [`Lists::rejects_prefix`] rejected, exactly as `offer_learn` counts a rejected run
    /// (offered += n, runs += 1, nothing kept, the ties at #1 unchanged). Returns the step count of that rejection.
    pub fn count_rejected(&mut self, n: u64) -> u64 {
        if n > 0 {
            self.offered += n;
            self.runs += 1;
        }
        1
    }

    /// The held codes, best first, each run expanded (OR-EQ compares these).
    #[must_use]
    pub fn expanded(&self) -> Vec<(Key, Code)> {
        let mut v = Vec::new();
        for ((k, c), n) in &self.held {
            for i in 0..*n {
                v.push((*k, plus(c.0, i)));
            }
        }
        v
    }

    /// The held entries, best first: (key, first code, run length).
    pub fn entries(&self) -> impl Iterator<Item = (Key, Code, u64)> + '_ {
        self.held.iter().map(|((k, c), n)| (*k, c.0, *n))
    }

    /// Codes held.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.total
    }

    /// No code held.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// The same list with every held code moved up by `by` (a 256-bit add; the counters and the ties at #1 kept): the host adds
    /// a trade-model digit (N, searched inside the search, D-S10) above the dials' digits before it merges the per-N lists. `None`
    /// when a code would leave 256 bits.
    #[must_use]
    pub fn shifted(&self, by: Code) -> Option<Self> {
        let mut held = BTreeMap::new();
        for ((k, c), n) in &self.held {
            let mut out = [0_u64; 4];
            let mut carry = 0_u128;
            for (i, o) in out.iter_mut().enumerate() {
                let s = u128::from(c.0.get(i).copied().unwrap_or(0)) + u128::from(by.get(i).copied().unwrap_or(0)) + carry;
                *o = u64::try_from(s & u128::from(u64::MAX)).ok()?;
                carry = s >> 64;
            }
            if carry != 0 {
                return None;
            }
            held.insert((*k, Ck(out)), *n);
        }
        let worst = held.last_key_value().map(|(k, _)| *k);
        Some(Self { k: self.k, held, worst, total: self.total, offered: self.offered, runs: self.runs, accepts: self.accepts, top: self.top })
    }

    /// The list bound k.
    #[must_use]
    pub const fn bound(&self) -> u64 {
        self.k
    }

    /// Joins another shard's list (associative; P18's merge): every held entry re-offered, counters added.
    pub fn merge(&mut self, o: &Self) {
        let (off, runs) = (self.offered, self.runs);
        for (k, c, n) in o.entries() {
            self.offer_learn(k, c, n);
        }
        self.offered = off + o.offered;
        self.runs = runs + o.runs;
        if let Some((k, n)) = o.top {
            match &mut self.top {
                Some((tk, tn)) if *tk == k => {
                    // offer_learn above already added the held part of o's ties; recount from o's own counter.
                    let held_top: u64 = o.entries().filter(|e| e.0 == k).map(|e| e.2).sum();
                    *tn = tn.saturating_sub(held_top) + n;
                }
                Some((tk, _)) if k < *tk => self.top = Some((k, n)),
                None => self.top = Some((k, n)),
                _ => {}
            }
        }
    }
}
