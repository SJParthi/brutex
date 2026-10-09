//! Gate 8 for the All sources routes: `/sources.json`, `/sources/bars.json`
//! and `/sources/day.json`.
//!
//! Nothing to time yet. The routes are registered by the WP-01 seam and answer
//! `501 not built: WP-37` until WP-37 builds them, and a figure for a route that
//! does no work would be a measurement of nothing presented as a bound
//! (`CLAUDE.md` §3 rule 6). This module says so on every run instead of staying
//! silent, and WP-37 replaces the line with its rows.

/// Says, on the bench's own output, that this family has no cost row yet.
pub(crate) fn not_built_yet() {
    println!(
        "  sources (WP-37)                                NOT MEASURED — the three \
         routes answer 501 not built; no cost row until they do work"
    );
}
