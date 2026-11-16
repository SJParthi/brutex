//! The fill-law table of the sniper (plan 5): THE law is `req70-2s` (crate v2's standard). S3 (MAIN-SIGNED-SNIPER, binding):
//! the "with money cap" beside row is DROPPED, because `req70-2s-cap` "keeps the old HARD cap price as a block skip"
//! (COMMON-FILL-STANDARD-DESIGN.md:242), an entry waiting for a dip, which SNIPER s8 forbids in any headline or beside row. So a
//! row naming it is refused AT LOAD (matrix SN-X45, mutant m-S17), and the page prints the WORDS line S3 in its place.

/// One law row the host may be asked to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostLaw {
    /// THE standard: `req70-2s` (cash: the 2-second block from the candle END).
    Std70,
}

/// Loads a law row by its name.
///
/// # Errors
/// Every name but `req70-2s`; the money-cap law with S3's words.
pub fn load(name: &str) -> Result<HostLaw, String> {
    if name == common_fill::LAW_STD {
        return Ok(HostLaw::Std70);
    }
    if name == common_fill::LAW_STD_CAP {
        if cfg!(feature = "m_s17") {
            // MUTANT m-S17: a beside law that skips a block on price is accepted.
            return Ok(HostLaw::Std70);
        }
        return Err("with money cap: not run (SNIPER s8: no limit or wait-for-a-dip entry in any row)".to_string());
    }
    Err(format!("fill law {name}: not a sniper law (only {})", common_fill::LAW_STD))
}
