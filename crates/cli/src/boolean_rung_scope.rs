//! Canonical intraday selection with stable physical rung indices.

/// Explicit nonempty selection of the existing eight intraday timeframes.
/// Statistical allocations retain eight reserved units; omitted units are unspent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RungScope(u8);

impl RungScope {
    /// Original all-eight scope; its persisted encodings remain unchanged.
    pub const ALL: Self = Self(u8::MAX);

    /// Canonicalize labels without accepting duplicate or nonintraday entries.
    ///
    /// # Errors
    /// Empty, duplicate, unknown and daily selections are refused before I/O.
    pub fn new(rungs: &[&str]) -> Result<Self, String> {
        let mut mask = 0_u8;
        for rung in rungs {
            let index = crate::ledger_all::LEDGER_RUNGS
                .iter()
                .position(|known| known == rung)
                .ok_or_else(|| format!("unsupported intraday timeframe: {rung}"))?;
            let bit = 1_u8 << index;
            if mask & bit != 0 {
                return Err(format!("duplicate intraday timeframe: {rung}"));
            }
            mask |= bit;
        }
        Self::from_mask(mask)
    }

    pub(crate) fn from_mask(mask: u8) -> Result<Self, String> {
        if mask == 0 {
            Err("at least one intraday timeframe is required".into())
        } else {
            Ok(Self(mask))
        }
    }

    pub(crate) const fn mask(self) -> u8 {
        self.0
    }

    /// Whether a stable physical rung index belongs to this declaration.
    #[must_use]
    pub const fn contains(self, index: usize) -> bool {
        index < 8 && self.0 & (1_u8 << index) != 0
    }

    /// Number of selected timeframes, separate from eight reserved error units.
    #[must_use]
    pub const fn count(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Canonical selected physical indices, never compacted or renumbered.
    pub fn indices(self) -> impl Iterator<Item = usize> {
        (0..8).filter(move |&index| self.contains(index))
    }

    /// Canonical selected labels in the original ledger order.
    #[must_use]
    pub fn labels(self) -> Vec<&'static str> {
        crate::ledger_all::LEDGER_RUNGS
            .iter()
            .enumerate()
            .filter_map(|(index, &rung)| self.contains(index).then_some(rung))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::RungScope;

    #[test]
    fn selections_are_canonical_nonempty_intraday_and_keep_physical_indices() -> Result<(), String>
    {
        let selected = RungScope::new(&["60min", "3min", "1min"])?;
        assert_eq!(selected, RungScope::new(&["1min", "3min", "60min"])?);
        assert_eq!(selected.labels(), ["1min", "3min", "60min"]);
        assert_eq!(selected.indices().collect::<Vec<_>>(), [0, 2, 7]);
        assert_eq!(selected.count(), 3);
        assert!(!selected.contains(1));
        assert!(!selected.contains(8));
        assert_eq!(
            RungScope::new(&crate::ledger_all::LEDGER_RUNGS)?,
            RungScope::ALL
        );
        for invalid in [
            &[][..],
            &["1min", "1min"][..],
            &["day"][..],
            &["daily"][..],
            &["4min"][..],
        ] {
            assert!(RungScope::new(invalid).is_err());
        }
        Ok(())
    }
}
