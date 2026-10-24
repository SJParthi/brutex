//! One dictionary of reason codes (READ-02, R4-05; D-TBD-R4-05-1).
//!
//! A page shows a code — `needs_month_generation`, `source_crc_mismatch`,
//! `owed` — and until this table the person reading it had to know the code.
//! Each row here says, once, what the code means in plain words, what a
//! person does about it, and the button or step that does it, so the import
//! panel, the day band and `/verify` can put the sentence on hover beside the
//! code without adding a paragraph to any page.
//!
//! # A row
//!
//! `(code, sentence, action, control)`:
//!
//! * `sentence` — what happened, in words a person who never read the code
//!   understands. It starts lower-case (a name such as NSE or GDFL aside) and
//!   ends with no stop, because a page writes it as `code: sentence; control`
//!   ([`title`]): the acceptance's `needs_month_generation: the month already
//!   passed this day; press Repair`.
//! * `action` — what a person does, in the same style.
//! * `control` — the button or step that does it ([`Control`]).
//!
//! # Where the codes come from, and the gate
//!
//! The codes are not this file's. Each is named by its own source: the GDFL
//! importer's refusals (`ImportRefusal::code`) and causes (`cm_cause`,
//! `nfo_cause`), IA-16's failure kinds (`FailKind::code`, which the store's
//! repair refusals reach through `FailKind::of_repair`) and run refusals
//! (`RunRefusal::CODES`), the verdict codes and flags (`store::verdict`), the
//! landing's row causes (`RowCause::code`) and the verifier's reason words
//! (`verdict_words::REASONS`). The gate (`reasons_tests.rs`) collects every
//! code from those sources — by calling a type's own list where it has one,
//! and by reading the code strings out of the source file where it has none,
//! never from a hand-kept copy — and fails on a code with no row here and on
//! a row whose code no source names.
//!
//! **Shared rows.** A package that adds a code adds its row here in the same
//! change; a package that adds a SOURCE of codes adds it to the gate.
//!
//! # Cost
//!
//! [`of`] is one hash probe into an index built once per process: O(1),
//! measured by `pull::reasons::tests::of_finds_every_row_and_nothing_else_in_one_probe`
//! (`docs/06-limits.md` L-R405-03). [`body`] is O(rows), measured by
//! `pull::reasons::tests::building_the_body_is_one_pass_over_the_table` (L-R405-02).
//! [`body`] serialises the whole table and is called once per process by the
//! route that serves it (`/reasons.json`), which keeps the bytes; it is the
//! one event this module emits ([`TARGET`], `reasons table built`), so the
//! table is logged once per build and nothing is logged per request.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::OnceLock;

/// The target of the one event this module emits.
pub const TARGET: &str = "pull.reasons";

/// The `schema` word of the served body. A changed body shape is a new word.
pub const SCHEMA: &str = "reasons-v1";

/// The button or step that does a row's action (D-TBD-R4-05-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// The repair: an import run with a repair reason
    /// (`cli gdfl-import ... --repair-reason TEXT`), which lands a month the
    /// append-only store refuses as a new generation. Label `press Repair`.
    Repair,
    /// A run that reads again the days closed with refusals
    /// (`cli gdfl-import ... --retry-refused`). Label `Retry refused`.
    RetryRefused,
    /// One step of `config/store-steps.toml`, by its id: a store write a
    /// person runs by hand. Label `store step <id>`; the gate refuses an id
    /// the file does not hold or holds withdrawn.
    StoreStep(&'static str),
    /// Something only a person can do, in a few words. Label
    /// `operator: <what>`.
    Operator(&'static str),
    /// Nothing to do: the code states a fact, not a fault. Label `no action`.
    Nothing,
}

impl Control {
    /// The label a page puts on the control.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Repair => "press Repair".to_owned(),
            Self::RetryRefused => "Retry refused".to_owned(),
            Self::StoreStep(id) => format!("store step {id}"),
            Self::Operator(what) => format!("operator: {what}"),
            Self::Nothing => "no action".to_owned(),
        }
    }

    /// The control's kind, as the served body names it.
    #[must_use]
    pub const fn kind(self) -> &'static str {
        match self {
            Self::Repair => "repair",
            Self::RetryRefused => "retry_refused",
            Self::StoreStep(_) => "store_step",
            Self::Operator(_) => "operator",
            Self::Nothing => "none",
        }
    }

    /// The store step a [`Control::StoreStep`] names.
    #[must_use]
    pub const fn step(self) -> Option<&'static str> {
        match self {
            Self::StoreStep(id) => Some(id),
            _ => None,
        }
    }
}

/// One row of the dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reason {
    /// The code, exactly as its source writes it.
    pub code: &'static str,
    /// What happened, in plain words.
    pub sentence: &'static str,
    /// What a person does.
    pub action: &'static str,
    /// The button or step that does it.
    pub control: Control,
}

const fn row(
    code: &'static str,
    sentence: &'static str,
    action: &'static str,
    control: Control,
) -> Reason {
    Reason {
        code,
        sentence,
        action,
        control,
    }
}

use Control::{Nothing, Operator, Repair, RetryRefused, StoreStep};

const RUN_AGAIN: &str = "run the import again";
const NEW_COPY: &str = "after a corrected delivery from the vendor, retry the refused days";
const PULL_AGAIN: &str = "pull the day again";

/// Every row, grouped by the source that first names its code. A code two
/// sources share has one row, whose words fit both.
pub static TABLE: &[Reason] = &[
    // ── The GDFL importer's refusals (`ImportRefusal::code`) ──────────────
    row(
        "kind_unknown",
        "the import was asked for a kind other than indices, stocks or options",
        "run the import again naming indices, stocks or options",
        Operator("name a known kind"),
    ),
    row(
        "range_backwards",
        "the first day asked for is after the last day",
        "run again with the first day on or before the last",
        Operator("fix the day range"),
    ),
    row(
        "volume_overflow",
        "a traded quantity, or one second's total of them, is too large to store",
        "report the file to the vendor; the day stays open and the next run reads it again",
        Operator("report the file to the vendor"),
    ),
    row(
        "fold",
        "the rows of a file could not be folded into one-second bars",
        "read the fold's own words for this day in /logs, fix the cause, then run the import again",
        Operator("read the log, then run again"),
    ),
    row(
        "journal",
        "the import journal could not be read or written",
        "check that the store's imports folder is readable and writable, then run the import again",
        Operator("fix the journal, then run again"),
    ),
    row(
        "filter_name",
        "a symbol filter names something that is not a symbol as the store files it",
        "run again with each symbol spelt exactly as the store files it",
        Operator("fix the symbol filter"),
    ),
    row(
        "stamp_past_the_day",
        "a row is stamped at or after midnight of its own day, so it would land on the next day",
        "report the file to the vendor; the day stays open and the next run reads it again",
        Operator("report the file to the vendor"),
    ),
    row(
        "too_many_entries",
        "the census is full, so the day's bars are on disk but not counted",
        "recount the census to make room, then run the import again to resume from this day",
        Operator("recount the census, then resume"),
    ),
    row(
        "census_refused",
        "the census refused the day's batch, so nothing of it was counted and the run stopped",
        "read the census refusal in /logs, fix it, then run the import again to resume",
        Operator("fix the census, then resume"),
    ),
    row(
        "filing",
        "the store would not file a converted file's bars, for a reason it named in its own words",
        "read the store's words for this file in /logs; the file is held back and the next run reads it again",
        Operator("read the log, then run again"),
    ),
    row(
        "day_not_held",
        "the source holds no file for a trading day the calendar owes",
        "ask the vendor for the day, then run the import again over it",
        Operator("get the day from the vendor"),
    ),
    row(
        "provenance",
        "the record of where a landed file came from could not be written, so its bars are stored but unrecorded",
        "check that the store's sidecar files are writable, then run the import again",
        Operator("fix the sidecar, then run again"),
    ),
    row(
        "day_ledger",
        "the GDFL day ledger could not be read, or would not take the day's lines",
        "check the day ledger under the store's imports folder, then run the import again; the day stays incomplete until then",
        Operator("fix the day ledger, then run again"),
    ),
    row(
        "lineage",
        "the options' lineage or lot ledger could not be read or written",
        "check the lineage and lot ledgers under the store's imports folder, then run the import again",
        Operator("fix the lineage ledger, then run again"),
    ),
    row(
        "durability",
        "the computer refused to make the day's writes durable, so the run stopped at that day",
        "check the disk and its health, then run the import again; it reads the day again",
        Operator("check the disk, then run again"),
    ),
    row(
        "filter_renamed",
        "an options filter names a symbol the lineage files under a newer name",
        "run again with the newer name the lineage files it under",
        Operator("use the newer symbol"),
    ),
    row(
        "underlying_split",
        "the store already holds live options under an older name of this underlying, so the run would open a second name for them",
        "move the old name's options under the new name first, then run again",
        Operator("re-key the old name first"),
    ),
    row(
        "stopped",
        "the run was asked to stop, and stopped between two files",
        "run the import again; it resumes from this day",
        Operator("run again to resume"),
    ),
    row(
        "held_back",
        "an earlier file of the same instrument and month failed in this run, so this later file waited",
        "fix the earlier day's refusal; the next run files the earlier day first, then this one",
        Operator("fix the earlier day first"),
    ),
    row(
        "no_wanted_file",
        "the day's listing held no file the import wants, on a day the calendar is open",
        "ask the vendor for a corrected delivery of the day; the next run reads it then",
        Operator("get a corrected delivery"),
    ),
    row(
        "absent_from_listing",
        "a swept index is missing from the vendor's own listing of the day",
        NEW_COPY,
        RetryRefused,
    ),
    row(
        "stored_index_absent",
        "a stored index is missing from a day inside the span it was published for",
        NEW_COPY,
        RetryRefused,
    ),
    row(
        "disk_full",
        "the store's disk is full, so nothing more could be written",
        "free space on the store's disk, then run again; it resumes where it stopped",
        Operator("free disk space"),
    ),
    row(
        "store_lost",
        "the store's folder went away during the run, so nothing more was written",
        "reconnect the store's drive, then run the import again; it reads the day again",
        Operator("reconnect the store"),
    ),
    row(
        "store_not_writable",
        "the store refused a write because of a permission or a read-only drive",
        "make the store's folder writable, then run the import again",
        Operator("make the store writable"),
    ),
    row(
        "free_space_unknown",
        "the free space of the store's disk could not be measured, so the free-space floor could not be checked",
        "check that the store's drive is mounted and readable, then run the import again",
        Operator("check the store's drive"),
    ),
    row(
        "source_lost",
        "the source's folder went away during the run while the store stayed",
        "reconnect the source's drive, then run the import again",
        Operator("reconnect the source"),
    ),
    row(
        "held_back_cap",
        "more instruments were held back than one run can hold, so the run ended",
        "fix the first held-back refusal /logs names, then run the import again; it continues past it",
        Operator("fix the first held-back day"),
    ),
    // ── The vendor's files, as the importer judges them (`cm_cause`, ─────
    //    `nfo_cause`). A vendor cause closes the day; the rest leave it open.
    row(
        "folder_unknown",
        "a folder in the vendor's delivery is not named as a day of a known tree",
        NEW_COPY,
        RetryRefused,
    ),
    row(
        "extension_unknown",
        "a wanted file has an extension other than csv",
        NEW_COPY,
        RetryRefused,
    ),
    row(
        "ambiguous_case_twins",
        "one folder holds two files for one ticker whose names differ only in the case of the extension",
        "keep one of the two copies in the delivery, then retry the refused days",
        RetryRefused,
    ),
    row(
        "header_unknown",
        "the file's first line is not a header the vendor has used before",
        "compare the header with the vendor's; after a corrected file or a build that reads it, retry the refused days",
        RetryRefused,
    ),
    row(
        "ticker_mismatch",
        "the ticker inside the file is not the file's own name",
        NEW_COPY,
        RetryRefused,
    ),
    row(
        "date_mismatch",
        "the date inside the file is not the day its folder names",
        NEW_COPY,
        RetryRefused,
    ),
    row(
        "malformed_row",
        "a row of the file is not ten well-formed fields",
        NEW_COPY,
        RetryRefused,
    ),
    row(
        "price_refused",
        "a price in the file is not a plain number of at most two decimals, or a traded row is priced at zero",
        NEW_COPY,
        RetryRefused,
    ),
    row(
        "rows_over_cap",
        "the file holds more rows than one file may",
        "ask the vendor whether the file is whole; after a corrected file, retry the refused days",
        RetryRefused,
    ),
    row(
        "no_session_rows",
        "the file holds no row inside the trading session",
        "check with the vendor that the day traded; after a corrected file, retry the refused days",
        RetryRefused,
    ),
    row(
        "index_ends_in_session",
        "an index file ends before the session's close",
        "ask the vendor for the whole day's file, then retry the refused days",
        RetryRefused,
    ),
    row(
        "extension_missing",
        "a wanted file has no extension at all, so it was read as another file",
        NEW_COPY,
        RetryRefused,
    ),
    row(
        "too_many_bad_rows",
        "more of the file's rows were refused than a file may lose and still count as the file",
        NEW_COPY,
        RetryRefused,
    ),
    row(
        "symbol_not_in_use",
        "the file is named for a symbol the exchange was not using that day",
        "record the dated symbol change, then read the day again by name",
        StoreStep("S-36"),
    ),
    row(
        "archive_duplicate_day",
        "the vendor's archive holds two zips for one day of one tree",
        "remove the extra copy from the archive, then run the import again",
        Operator("remove the extra copy"),
    ),
    row(
        "archive_member_compressed",
        "a day zip inside the archive is itself compressed, so its contents cannot be found",
        "re-zip the archive with the day zips stored as they are, or use a build that reads it, then run the import again",
        Operator("re-zip the archive"),
    ),
    row(
        "archive_method_unknown",
        "a file in a day zip is compressed by a method this build cannot read",
        "re-zip the day with a common method, or use a build that reads it, then run the import again",
        Operator("re-zip the day"),
    ),
    row(
        "archive_member_encrypted",
        "a file in a day zip is encrypted",
        "ask the vendor for an unencrypted copy, then run the import again",
        Operator("get an unencrypted copy"),
    ),
    row(
        "archive_prefixed",
        "the archive has extra bytes in front that its own offsets do not account for",
        "download the archive again, then run the import again",
        Operator("download the archive again"),
    ),
    row(
        "day_zip_misfiled",
        "a day zip sits under a folder other than the one its own name files it in",
        "move the day zip to its own folder, then run the import again",
        Operator("move the day zip"),
    ),
    row(
        "day_file_misfiled",
        "a tick-store day file sits outside its year and month folder",
        "move the day file into its month folder, then run the import again",
        Operator("move the day file"),
    ),
    row(
        "calendar_not_regular",
        "the calendar knows this day and it is not a regular full session",
        "check the day against the exchange's circulars; the day is read once the calendar records its session",
        Operator("check the calendar"),
    ),
    row(
        "calendar_unmeasured",
        "the day is outside the span the calendar has measured",
        "extend the calendar to cover the day, then run the import again",
        Operator("extend the calendar"),
    ),
    row(
        "not_swept",
        "the instrument has no GDFL file name, so it is outside the swept instruments",
        "check the instrument list; an instrument outside the swept ones is never imported",
        Operator("check the instrument list"),
    ),
    row(
        "archive_unavailable",
        "the vendor's archive could not be opened or read",
        "check the archive's drive and path, then run the import again",
        Operator("check the archive"),
    ),
    row(
        "archive_malformed",
        "the archive's bytes are not a valid zip",
        "download the archive again, then run the import again",
        Operator("download the archive again"),
    ),
    row(
        "source_length_mismatch",
        "a file read from the source is not as long as the vendor's archive says it is",
        "download the file or the archive again, then run the import again",
        Operator("download the file again"),
    ),
    row(
        "source_crc_mismatch",
        "a file read from the source does not match the checksum the vendor's archive states",
        "download the file or the archive again, then run the import again",
        Operator("download the file again"),
    ),
    row(
        "archive_member_corrupt",
        "a compressed file in a day zip does not decompress",
        "download the archive again, then run the import again",
        Operator("download the archive again"),
    ),
    row(
        "tick_store_malformed",
        "a tick-store day file breaks a rule of its own format",
        "rebuild that day file from the vendor's archive, then run the import again",
        Operator("rebuild the day file"),
    ),
    row(
        "tick_store_unavailable",
        "a tick-store day file exists but could not be opened or read",
        "check the tick store's drive and the file's permissions, then run the import again",
        Operator("check the tick store"),
    ),
    row(
        "calendar_window_not_landed",
        "the calendar records special-session windows for the day, but the store keeps no bar at their first minute, so nothing would land",
        "report it: the calendar and the store's session rule disagree, and the day is read once they agree",
        Operator("report the disagreement"),
    ),
    row(
        "auction_eligibility_unknown",
        "a closing-auction row belongs to a share whose eligibility that day is not known",
        "install the day's security master, then run the import again",
        Operator("install the security master"),
    ),
    row(
        "auction_sidecar",
        "the month's closing-auction file refused the day's prints",
        "read the refusal in /logs, fix the auction file, then run the import again",
        Operator("fix the auction file"),
    ),
    row(
        "ticker_unparsed",
        "an option ticker reads as neither of the vendor's formats",
        "after a corrected file, or a build that reads the format, retry the refused days",
        RetryRefused,
    ),
    row(
        "ticker_ambiguous",
        "an option ticker has more than one reading in its era",
        "after a corrected file, or a build that reads the format, retry the refused days",
        RetryRefused,
    ),
    row(
        "contract_named_twice",
        "two tickers of one day name one contract, so both were refused",
        "ask the vendor which ticker is right; after a corrected delivery, retry the refused days",
        RetryRefused,
    ),
    row(
        "duplicate_ticker",
        "two entries of one day name the same ticker",
        NEW_COPY,
        RetryRefused,
    ),
    row(
        "open_interest_refused",
        "an open interest figure is too large to be real",
        NEW_COPY,
        RetryRefused,
    ),
    row(
        "strike_implausible",
        "an option's strike is far beyond the day's usual strikes of its underlying, so no real contract carries it",
        "ask the vendor whether the contract is real; after a corrected delivery, retry the refused days",
        RetryRefused,
    ),
    row(
        "monthly_expiry_unmeasured",
        "a monthly option's expiry needs a day the calendar has not measured",
        "extend the calendar to cover the expiry, then run the import again",
        Operator("extend the calendar"),
    ),
    row(
        "expiry_passed",
        "a monthly option traded after the expiry the calendar gives it, so neither is believed",
        "check the expiry against the exchange's circulars; the day is read again once the calendar learns it",
        Operator("check the expiry"),
    ),
    row(
        "expiry_unsettled",
        "the option's expiry could not be settled against the calendar",
        "check the expiry against the exchange's circulars; the day is read again once the calendar learns it",
        Operator("check the expiry"),
    ),
    row(
        "underlying_refused",
        "the option's underlying is not a symbol the store can file",
        "check the underlying's symbol; the day is read again once the store can file it",
        Operator("check the underlying symbol"),
    ),
    row(
        "underlying_stale",
        "the option file is named for an underlying symbol the exchange was not using that day",
        "record the dated symbol change, then read the day again by name",
        StoreStep("S-36"),
    ),
    row(
        "contract_unrenderable",
        "the option contract cannot be written as the store's contract name",
        "report the contract; a build that can write it reads the day again",
        Operator("report the contract"),
    ),
    // ── IA-16's failure kinds (`FailKind::code`). The store's repair ──────
    //    refusals reach the page through `FailKind::of_repair`.
    row(
        "month_busy",
        "another process held the month, so it was not written or compared this time",
        "run again once the other process has finished",
        Operator("run again later"),
    ),
    row(
        "needs_month_generation",
        "the month already passed this day",
        "run the import with a repair reason for this month, which writes it as a new generation",
        Repair,
    ),
    row(
        "month_superseded",
        "the month is served from a repair, so only a further repair adds this day",
        "run a repair of this month again; a plain import never adds to it",
        Repair,
    ),
    row(
        "bars_differ_held",
        "the store already holds different bars for this time, and a stored bar is never changed",
        "compare the vendor's day with the stored one; no repair fills it",
        Operator("compare the two copies"),
    ),
    row(
        "repair_refused",
        "the repair refused for a reason a later build or a later repair can lift",
        "read the repair's reason in /logs, lift it, then repair again",
        Repair,
    ),
    row(
        "repair_impossible",
        "the repair refused for good, and no run reads this day again",
        "record the day as a known hole; nothing writes it",
        Operator("record the known hole"),
    ),
    row(
        "store_io",
        "the computer refused a read or a write of the store",
        "check the store's disk, then run the import again; the next run retries the same bytes",
        Operator("check the disk, then run again"),
    ),
    row(
        "store_damaged",
        "the month's file failed its own check, such as a checksum, its format or a torn end, so nothing was written to it",
        "check the month and restore it from the backup if it is damaged",
        Operator("check and restore the month"),
    ),
    row(
        "batch_refused",
        "the store refused a batch the importer built wrongly",
        "report it as an importer defect with the day and the file; the file is held back",
        Operator("report an importer defect"),
    ),
    row(
        "month_sealed",
        "the month is sealed and its next generation could not be written, so the file was held back",
        "read the month's repair line in /logs, then run the import again; the next run reads the file again",
        Operator("read the log, then run again"),
    ),
    // ── IA-16's run refusals (`RunRefusal::CODES`) ─────────────────────────
    row(
        "request_line_invalid",
        "the run's request line could not be read",
        "start the run again from the command line or the page",
        Operator("start the run again"),
    ),
    row(
        "stored_with_options",
        "the stored-only narrowing applies to shares and indices, and was asked of an options run",
        "run the options without --stored",
        Operator("drop --stored"),
    ),
    row(
        "symbol_empty",
        "a symbol filter is empty",
        "name each symbol, or leave the filter out to take every symbol",
        Operator("fix the symbol filter"),
    ),
    row(
        "filter_too_long",
        "the symbol filter names more symbols than one run takes",
        "split the symbols over several runs, or leave the filter out",
        Operator("split the filter"),
    ),
    row(
        "snapshot_invalid",
        "the snapshot name is too long, or uses a character other than a letter, a digit, a dot, a dash or an underscore",
        "choose a short name of letters, digits, dots, dashes and underscores",
        Operator("rename the snapshot"),
    ),
    row(
        "log_unavailable",
        "the run's log could not be opened, and no run writes the store without a log",
        "check that the store's logs folder is writable, then run again",
        Operator("fix the log folder"),
    ),
    row(
        "unstamped_write",
        "this build carries no commit stamp, so it may not write the store",
        "run a build made from a clean, committed tree",
        Operator("use a stamped build"),
    ),
    row(
        "source_unset",
        "no setting names the source's folder",
        "set the variable that names the source's folder, then run again",
        Operator("set the source's folder"),
    ),
    row(
        "scope_unreadable",
        "the list of what the run covers could not be built",
        "read the reason in /logs, fix it, then run again",
        Operator("read the log, then run again"),
    ),
    row(
        "source_unopened",
        "the source could not be opened",
        "check the source's drive and path, then run again",
        Operator("check the source"),
    ),
    row(
        "symbol_invalid",
        "a symbol filter names something that is not a symbol as the store files it",
        "run again with each symbol spelt exactly as the store files it",
        Operator("fix the symbol filter"),
    ),
    row(
        "run_busy",
        "another process holds a lock this run needs",
        "wait for that process to finish, as the lock's claim names it, then run again",
        Operator("wait for the other run"),
    ),
    row(
        "run_lock_refused",
        "a lock file could not be opened or taken",
        "check the lock file under the store: it must be a plain file on a store that is there",
        Operator("check the lock file"),
    ),
    row(
        "lock_replaced",
        "a lock file was removed or replaced after it was opened",
        "find what touched the lock file, then run again",
        Operator("check what replaced the lock"),
    ),
    row(
        "lock_stalled",
        "a lock was held longer than a run waits for it",
        "check the process that holds it, then run again",
        Operator("check the lock's holder"),
    ),
    row(
        "census_layout_changed",
        "the census changed between the run's first look and its locks",
        "run again; a second change means another process is writing the census",
        Operator("run again"),
    ),
    row(
        "census_unreadable",
        "the census could not be read",
        "check the store's census files, then run again",
        Operator("check the census"),
    ),
    row(
        "runs_ledger_damaged",
        "the last two records of the runs ledger fail their checks, so no run of either feed starts",
        "look at the runs ledger and replace the damaged one with the store's own command",
        Operator("look at the runs ledger"),
    ),
    row(
        "clock_implausible",
        "the computer's clock cannot be right",
        "set the clock, then run again",
        Operator("set the clock"),
    ),
    row(
        "runs_ledger_unwritten",
        "the runs ledger could not be read or written",
        "check the store's runs ledger file, then run again",
        Operator("check the runs ledger"),
    ),
    row(
        "claim_unwritten",
        "the run's claim or request file could not be written",
        "check the store's disk and permissions, then run again",
        Operator("check the store's disk"),
    ),
    row(
        "repair_reason_invalid",
        "the repair reason is empty, too long, or holds a control character",
        "give a repair reason of one plain line",
        Operator("fix the repair reason"),
    ),
    row(
        "repair_in_open_month",
        "a repair was asked in a month still being traded, which would leave every later day of it needing a further repair",
        "repair after the month's last trading day, or run again with --accept-open-month to accept that",
        Operator("wait for the month to end, or accept it"),
    ),
    // ── The verdict codes (`store::verdict::Code::ALL`) ────────────────────
    row(
        "match",
        "every unit the store holds equals the source's, and none is extra",
        "nothing to do",
        Nothing,
    ),
    row(
        "value_mismatch",
        "a price, a volume or an open interest differs between the store and the source",
        "open the day in /verify and compare the two copies before any repair",
        Operator("compare the two copies"),
    ),
    row(
        "missing_in_store",
        "the store lacks a unit an import says it landed",
        "read the day again with a repair",
        Repair,
    ),
    row(
        "extra_in_store",
        "the store holds a unit the source did not give that day",
        "open the day in /verify and compare the two copies",
        Operator("compare the two copies"),
    ),
    row(
        "count_mismatch",
        "the journal, the census or the listing counts disagree with the store",
        "recount the day and verify it again",
        Operator("recount, then verify again"),
    ),
    row(
        "source_refused",
        "the importer refused the file by name",
        "read the import's own reason for the file, which says what to do",
        Operator("read the import reason"),
    ),
    row(
        "store_without_source",
        "the store holds bars for a name the source lacks that day",
        "open the day in /verify and compare the two copies",
        Operator("compare the two copies"),
    ),
    row(
        "absent_from_store",
        "the file read clean, filed nothing, and was not refused",
        "report the file and the day: nothing was refused, so nothing says why",
        Operator("report the file"),
    ),
    row(
        "store_unreadable",
        "the month's file would not open, or its times did not increase",
        "check the month and restore it from the backup if it is damaged",
        Operator("check and restore the month"),
    ),
    row(
        "no_response_kept",
        "no kept answer of the vendor covers these minutes, so they cannot be vouched for",
        "pull the day again so that its answer is kept",
        Operator("pull the day again"),
    ),
    row(
        "not_compared",
        "these units are not compared, by a recorded rule",
        "nothing to do",
        Nothing,
    ),
    row(
        "source_lacks_day",
        "the source holds no file for a day the calendar opens",
        "ask the vendor for the day",
        Operator("get the day from the vendor"),
    ),
    row(
        "vendor_hole",
        "the vendor answered empty for minutes the calendar owes",
        "nothing the store can do; ask the vendor again later",
        Operator("ask the vendor again later"),
    ),
    row(
        "withheld_by_rule",
        "a unit the calendar owes is withheld by a recorded rule",
        "nothing to do",
        Nothing,
    ),
    row(
        "built_under_earlier_rule",
        "the month was built under a rule set this check does not compare by value",
        "rebuild the month under the current rules to compare it by value",
        Operator("rebuild under the current rules"),
    ),
    row(
        "day_partial",
        "the day closed partly done and this name was not reached",
        RUN_AGAIN,
        Operator("run the import again"),
    ),
    row(
        "no_ledger",
        "Zerodha minutes the calendar owes are missing, and no gap ledger says why",
        "pull the month again so that its gap ledger is written",
        Operator("pull the month again"),
    ),
    row(
        "owed",
        "the source holds the unit, the import owes it, and no import has landed it yet",
        "run the import over the day",
        Operator("run the import"),
    ),
    // ── The verdict flags (`store::verdict::flags::ALL`) ───────────────────
    row(
        "tix_absent",
        "the month has no time index",
        "build the month's time index",
        Operator("build the time index"),
    ),
    row(
        "spot_missing",
        "some option seconds have no spot second of their underlying",
        "import the underlying's day; a spot is never taken from a later second",
        Operator("import the underlying's day"),
    ),
    row(
        "after_index_close",
        "some option seconds come after the index's close",
        "nothing to do; the seconds are marked, never changed",
        Nothing,
    ),
    row(
        "spot_unavailable",
        "the underlying's day is not stored",
        "import the underlying's day",
        Operator("import the underlying's day"),
    ),
    row(
        "peer_minute_absent",
        "Zerodha has no minute where GDFL traded",
        "pull the Zerodha day again; if it stays empty, the vendor never had it",
        Operator("pull the Zerodha day again"),
    ),
    row(
        "peer_unavailable",
        "the check against the other feed could not run",
        "import or pull the other feed's day, then verify again",
        Operator("get the other feed's day"),
    ),
    row(
        "body_not_kept",
        "the vendor's answer was not kept",
        "pull the day again so that its answer is kept",
        Operator("pull the day again"),
    ),
    row(
        "source_changed_since_import",
        "the source changed after it was imported",
        "compare the new source with the stored day; a changed day is stored again as a new generation",
        Operator("compare the new source"),
    ),
    row(
        "archive_day_on_closed_day",
        "the archive holds a day the calendar says was closed",
        "check the day against the exchange's circulars",
        Operator("check the calendar"),
    ),
    row(
        "v3_manifest_disagrees",
        "the vendor's version 3 manifest disagrees with its files",
        "ask the vendor which of the two is right",
        Operator("ask the vendor"),
    ),
    row(
        "lineage_unrecorded",
        "a symbol hands over to another with no dated alias row",
        "record the dated symbol change",
        StoreStep("S-36"),
    ),
    row(
        "details_capped",
        "this record's details reached their cap and the rest were not kept",
        "nothing to do; the counts are whole and only the details are cut",
        Nothing,
    ),
    row(
        "name_cut",
        "the record holds only the first part of a long name",
        "nothing to do",
        Nothing,
    ),
    row(
        "dup_minutes",
        "identical duplicate minutes were counted once",
        "nothing to do",
        Nothing,
    ),
    row(
        "vendor_revised",
        "the vendor revised an answer after the store kept the first",
        "compare the two answers; the store keeps the first",
        Operator("compare the two answers"),
    ),
    row(
        "expiry_after_close",
        "a row traded after the venue's close on the expiry day",
        "nothing to do; the row is marked, never changed",
        Nothing,
    ),
    row(
        "quality_differs",
        "this check's count of a ledger field differs from the ledger's",
        "recount the day's ledger and verify again",
        Operator("recount, then verify again"),
    ),
    row(
        "deferred_pending",
        "an input from another kind of instrument is still pending",
        "import or verify the other kind first, then verify again",
        Operator("verify the other kind first"),
    ),
    row(
        "daycheck_differs",
        "the day bar disagrees with the day's minutes",
        "compare the day bar with its minutes in /verify",
        Operator("compare the day bar"),
    ),
    row(
        "serve_path_differs",
        "what the pages serve differs from the stored rows",
        "report it with the day and the page",
        Operator("report the serve path"),
    ),
    row(
        "lineage_unchecked",
        "a symbol hand-over cannot be judged until its detector is built",
        "nothing to do yet; a later build judges it",
        Nothing,
    ),
    row(
        "source_crc_unrecorded",
        "the stored day has no landing record to compare its source checksum with",
        "write the source records for the months already stored",
        StoreStep("S-25"),
    ),
    // ── The landing's row causes (`RowCause::code`, `HistoryEnds::CODE`) ───
    row(
        "timestamp_refused",
        "a candle's time is one the session calendar cannot judge",
        "check the day in the calendar; the minute lands once the calendar can judge it",
        Operator("check the calendar"),
    ),
    row(
        "cash_eligibility_unknown",
        "a share's closing-auction eligibility is not known for its day",
        "install the day's security master, then pull again",
        Operator("install the security master"),
    ),
    row(
        "price_off_grid",
        "a price does not land on the paisa grid",
        "report the answer to the vendor; the row is refused, never rounded",
        Operator("report the answer"),
    ),
    row(
        "off_grid_candle",
        "a broker candle does not start on the minute grid it was asked for",
        "report the answer to the vendor; the candle is refused, never moved",
        Operator("report the answer"),
    ),
    row(
        "conflicting_candles",
        "two candles at one time carry different values, so neither is kept",
        "pull the day again; if they still conflict, report it to the vendor",
        Operator("pull the day again"),
    ),
    row(
        "before_first_day_bar",
        "a minute falls on a day before the series' first day bar, so it is another security's",
        "nothing to do; the minute is refused and the series starts later",
        Nothing,
    ),
    row(
        "foreign_level",
        "a minute's prices lie far outside its own day's range, so they are another instrument's",
        "report the answer to the vendor; the minute is refused",
        Operator("report the answer"),
    ),
    row(
        "non_positive_price",
        "a price is at or below zero",
        "report the answer to the vendor; the bar is refused",
        Operator("report the answer"),
    ),
    row(
        "before_listing",
        "the bar falls before the share was listed on NSE",
        "nothing to do; no bar is owed before the listing day",
        Nothing,
    ),
    row(
        "in_session_jump",
        "a minute opens far from the previous minute's close inside one run, so another price basis is mixed in",
        "pull the day again; if the jump stays, report it to the vendor",
        Operator("pull the day again"),
    ),
    row(
        "history_ends_delisted",
        "the share was delisted before the window began, so its history has ended",
        "nothing to do",
        Nothing,
    ),
    // ── The verifier's reason words (`verdict_words::REASONS`) ─────────────
    row(
        "vendor_empty",
        "the vendor answered empty for the day",
        "nothing the store can do; pull again later in case the vendor fills it",
        Operator("pull again later"),
    ),
    row(
        "withheld_unmeasured_session",
        "the day's session length is not measured, so its minutes are withheld",
        "measure the session in the calendar, then pull the day again",
        Operator("measure the session"),
    ),
    row(
        "calendar_unverified",
        "the calendar has not verified the day, so it was not compared",
        "verify the day in the calendar, then compare again",
        Operator("verify the calendar day"),
    ),
    row(
        "calendar_past_record",
        "the day is past the calendar's record, so it was not compared",
        "extend the calendar's record, then compare again",
        Operator("extend the calendar"),
    ),
    row(
        "unordered_stamps",
        "the answer's times were out of order, so it was not compared",
        PULL_AGAIN,
        Operator("pull the day again"),
    ),
    row(
        "refused_day",
        "the pull refused the day's rows, so the day is still owed",
        "read the pull's refusal in /logs, then pull the day again",
        Operator("pull the day again"),
    ),
    row(
        "refused_front",
        "the pull refused the front of a window before keeping it, so those days are still owed",
        "read the pull's refusal in /logs, then pull the days again",
        Operator("pull the days again"),
    ),
    row(
        "repaired",
        "the day's minutes were landed by a repair",
        "nothing to do",
        Nothing,
    ),
    row(
        "stored_short",
        "a month stored before the gap ledger holds fewer minutes than it owes",
        "pull the month again",
        Operator("pull the month again"),
    ),
    row(
        "unexplained",
        "minutes the calendar owes are missing, and the gap ledger does not explain them",
        "pull the day again; if they stay missing, report it",
        Operator("pull the day again"),
    ),
    row(
        "index_bucket_incomplete",
        "an index candle lacks a minute it needs, so it is withheld rather than folded short",
        "nothing to do",
        Nothing,
    ),
    row(
        "pre_ledger",
        "the folded candles were built before the fold ledger, so their counts are recorded but not compared by value",
        "fold the month again to compare it by value",
        Operator("fold the month again"),
    ),
    row(
        "receipts_full",
        "the answer's receipt file is full, so the answer was not kept",
        "make room for the receipts, then pull again",
        Operator("make room for the receipts"),
    ),
    row(
        "duplicate_body",
        "the vendor sent the same answer twice, and it was kept once",
        "nothing to do",
        Nothing,
    ),
    row(
        "landed_from_store",
        "the minutes were landed from another store, so no vendor answer was kept",
        "nothing to do",
        Nothing,
    ),
    row(
        "neither_vendor_has_it",
        "neither Zerodha nor GDFL holds these minutes",
        "nothing to do; both vendors lack them",
        Nothing,
    ),
    row(
        "answer_short",
        "the vendor's answer stopped before the end it was asked for",
        PULL_AGAIN,
        Operator("pull the day again"),
    ),
    row(
        "out_of_order",
        "a stored minute is earlier than the one before it",
        "check the month: the store refuses unordered minutes, so this names damage",
        Operator("check the month"),
    ),
    row(
        "duplicate",
        "a stored minute repeats the time of the one before it",
        "check the month: the store refuses repeated minutes, so this names damage",
        Operator("check the month"),
    ),
    row(
        "month_refused",
        "the month's file refused to open, so it was not compared",
        "check the month and restore it from the backup if it is damaged",
        Operator("check the month"),
    ),
    row(
        "pack_full",
        "the answer pack was full, so the answer was not kept",
        "make room for the answer packs, then pull again",
        Operator("make room for the packs"),
    ),
    row(
        "storage_full",
        "the disk was full, so the answer was not kept",
        "free disk space, then pull again",
        Operator("free disk space"),
    ),
    row(
        "refused_answer",
        "the answer's head was refused, so it was not kept",
        PULL_AGAIN,
        Operator("pull the day again"),
    ),
    row(
        "body_unreadable",
        "a kept answer could not be read back",
        "pull the day again so that a readable answer is kept",
        Operator("pull the day again"),
    ),
    row(
        "rung_day_absent",
        "a folded candle size holds no bar of a day its one-minute source holds",
        "fold the month's candle sizes again",
        Operator("fold the month again"),
    ),
    row(
        "pull_halted",
        "the pull halted before it asked for the day, so there is nothing to compare",
        "pull the day again once the halt is cleared",
        Operator("pull the day again"),
    ),
];

/// The index of every code in [`TABLE`], built once per process.
fn index() -> &'static HashMap<&'static str, usize> {
    static INDEX: OnceLock<HashMap<&'static str, usize>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut by_code = HashMap::with_capacity(TABLE.len());
        for (at, reason) in TABLE.iter().enumerate() {
            by_code.entry(reason.code).or_insert(at);
        }
        by_code
    })
}

/// The row of a code, or `None` for a code this build has no row for. One
/// hash probe: O(1), measured by
/// `pull::reasons::tests::of_finds_every_row_and_nothing_else_in_one_probe`.
#[must_use]
pub fn of(code: &str) -> Option<&'static Reason> {
    index().get(code).and_then(|at| TABLE.get(*at))
}

/// The hover text of a row: `code: sentence; control`, the acceptance's
/// `needs_month_generation: the month already passed this day; press Repair`.
#[must_use]
pub fn title(reason: &Reason) -> String {
    format!(
        "{}: {}; {}",
        reason.code,
        reason.sentence,
        reason.control.label()
    )
}

/// The longest [`title`] the table holds, in characters: the served
/// `title_cut`. A page cuts a title only past it (at a word, with an
/// ellipsis), so no title of the table is ever cut and the bound moves with
/// the table rather than being a number typed into a page (D-TBD-R4-05-5).
#[must_use]
pub fn title_cut() -> usize {
    TABLE
        .iter()
        .map(|reason| title(reason).chars().count())
        .max()
        .unwrap_or(0)
}

/// The served table: its bytes, and the tag that names them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    /// The JSON document, whole.
    pub bytes: Vec<u8>,
    /// `"<build>.<digest>"`: the build that serves it and the first 16 hex of
    /// the BLAKE3 of the bytes, so an unstamped build's tag still changes
    /// whenever its bytes do (D-TBD-R4-05-3).
    pub etag: String,
    /// The rows served.
    pub rows: usize,
}

/// The `/reasons.json` document for `build`, and its tag. Called once per
/// process by the route, which keeps the bytes; each call says so in one
/// event ([`TARGET`], `reasons table built`).
#[must_use]
pub fn body(build: &str) -> Body {
    let rows: Vec<serde_json::Value> = TABLE
        .iter()
        .map(|reason| {
            serde_json::json!({
                "code": reason.code,
                "sentence": reason.sentence,
                "action": reason.action,
                "control": reason.control.label(),
                "control_kind": reason.control.kind(),
                "step": reason.control.step(),
                "title": title(reason),
            })
        })
        .collect();
    let document = serde_json::json!({
        "schema": SCHEMA,
        "build": build,
        "count": TABLE.len(),
        "title_cut": title_cut(),
        "rows": rows,
    });
    let bytes = document.to_string().into_bytes();
    let digest = brutex_core::blake3::hash(&bytes);
    let mut etag = format!("\"{build}.");
    for byte in digest.iter().take(8) {
        let _infallible_into_a_string = write!(etag, "{byte:02x}");
    }
    etag.push('"');
    note_built(TABLE.len(), bytes.len(), build);
    Body {
        bytes,
        etag,
        rows: TABLE.len(),
    }
}

fn note_built(rows: usize, bytes: usize, build: &str) {
    let count = |n: usize| telemetry::Value::Uint(u64::try_from(n).unwrap_or(u64::MAX));
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(TARGET, "reasons table built")
            .with("rows", count(rows))
            .with("bytes", count(bytes))
            .with("build", telemetry::Value::Str(build)),
    );
}

#[cfg(test)]
#[path = "reasons_tests.rs"]
mod tests;
