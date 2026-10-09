//! The reader of the GDFL import journal, `imports/gdfl.journal` under the
//! store root (`pull::gdfl_import::journal_path`), for `/imports.json` and
//! `/provenance.json`.
//!
//! # Empty, on purpose
//!
//! The WP-01 seam creates this module so WP-33a, which builds the reader, adds
//! it here and never has to register a module in `server.rs`. It holds nothing
//! yet: no route reads the journal until WP-33a lands, and `/imports.json`
//! answers `501 not built: WP-33a` until then (see `super::imports`). A reader
//! written here before its field names are fixed would be a second definition
//! of a format `pull::gdfl_import` owns -- WP-02 records those names in D-3335.
