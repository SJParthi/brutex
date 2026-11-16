//! Every printed line from `data/WORDS-SNIPER-v1.tsv` (verbatim from the binding texts). A missing id is a refusal, never a default.

use std::collections::BTreeMap;

/// The words rows.
#[derive(Clone, Debug, Default)]
pub struct Words {
    map: BTreeMap<String, String>,
}

impl Words {
    /// Parses `id  text  source`.
    ///
    /// # Errors
    /// A row without a source or a repeated id.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut map = BTreeMap::new();
        for l in text.lines() {
            if l.starts_with('#') || l.starts_with("id\t") || l.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = l.split('\t').collect();
            let (Some(id), Some(t), Some(s)) = (f.first(), f.get(1), f.get(2)) else { return Err(format!("WORDS row short: {l}")) };
            if s.trim().is_empty() {
                return Err(format!("WORDS row {id} has no source"));
            }
            if map.insert((*id).to_string(), (*t).to_string()).is_some() {
                return Err(format!("WORDS row {id} repeated"));
            }
        }
        for need in ["S12", "FIXED_AFTER", "S3", "S6A", "S6B", "BARSRC", "FEW_UNSEEN", "FEW_LEARN", "COUNTS", "PROOF", "NOHEAD"] {
            if !map.contains_key(need) {
                return Err(format!("WORDS row {need} missing"));
            }
        }
        Ok(Self { map })
    }

    /// A line with its `{name}` slots filled.
    #[must_use]
    pub fn line(&self, id: &str, slots: &[(&str, String)]) -> String {
        let mut s = self.map.get(id).cloned().unwrap_or_else(|| format!("[WORDS {id} missing]"));
        for (k, v) in slots {
            s = s.replace(&format!("{{{k}}}"), v);
        }
        s
    }

    /// The trade-count words of one pick (plan 9, D-S13): always the counts; the "too few" lines under their thresholds.
    #[must_use]
    pub fn trade_count(&self, n_learn: u32, sessions: u32, m_unseen: u32) -> Vec<String> {
        let th = |id: &str, d: u32| self.map.get(id).and_then(|x| x.parse::<u32>().ok()).unwrap_or(d);
        let mut v = vec![self.line("COUNTS", &[("n", n_learn.to_string()), ("s", sessions.to_string()), ("m", m_unseen.to_string())])];
        if m_unseen < th("FEW_THRESH_UNSEEN", 30) {
            v.push(self.line("FEW_UNSEEN", &[("m", m_unseen.to_string())]));
        }
        if n_learn < th("FEW_THRESH_LEARN", 10) {
            v.push(self.line("FEW_LEARN", &[("n", n_learn.to_string())]));
        }
        v
    }

    /// The standing label block of every ADANIENT-alone result and kid row (S10): s12 beside the verdict words, fixed_after
    /// beside the train/test count, the proof step, the bar source, the S3 and S6 lines, no headline.
    #[must_use]
    pub fn standing(&self, proof_step: u8, proof_name: &str) -> Vec<String> {
        vec![
            self.line("S12", &[]),
            self.line("FIXED_AFTER", &[]),
            self.line("PROOF", &[("step", proof_step.to_string()), ("name", proof_name.to_string())]),
            self.line("BARSRC", &[]),
            self.line("S3", &[]),
            self.line("S6A", &[]),
            self.line("S6B", &[]),
            self.line("NOHEAD", &[]),
        ]
    }
}
