diff --git a/crates/api/src/backtest.rs b/crates/api/src/backtest.rs
index d8a9f24f..7a4dce4e 100644
--- a/crates/api/src/backtest.rs
+++ b/crates/api/src/backtest.rs
@@ -24,9 +24,10 @@
 //! statements of itself is a format that can diverge. Three things hold it
 //! together:
 //!
-//! 1. **Nothing here writes.** [`File::open`] is read-only and there is no
-//!    append path in this module. A reader that drifts renders wrong; a writer
-//!    that drifts corrupts. Only one crate may write, and it is `cli`.
+//! 1. **Nothing here writes.** [`cli::readonly_file::read`] is read-only and
+//!    there is no append path in this module. A reader that drifts renders
+//!    wrong; a writer that drifts corrupts. Only one crate may write, and it is
+//!    `cli`.
 //! 2. **The stride is asserted against the field sum at compile time** —
 //!    [`FIELD_SUM`] below — which is the same check
 //!    `the_stride_is_exactly_what_the_writer_writes` makes on the writer's
@@ -73,7 +74,6 @@
 //! measurement, however sound it is.
 
 use std::fmt::Write as _;
-use std::fs::File;
 use std::io::SeekFrom;
 use std::path::{Path, PathBuf};
 
diff --git a/crates/cli/src/search_checkpoint_tests.rs b/crates/cli/src/search_checkpoint_tests.rs
index b6bd9287..6d822038 100644
--- a/crates/cli/src/search_checkpoint_tests.rs
+++ b/crates/cli/src/search_checkpoint_tests.rs
@@ -332,7 +332,7 @@ fn a_torn_completion_marker_is_an_interrupted_reservation() -> Result<(), String
                 let mut names: Vec<String> = fs::read_dir(namespace.join("0000000000000002"))
                     .map(|entries| {
                         entries
-                            .filter_map(|entry| entry.ok())
+                            .filter_map(Result::ok)
                             .map(|entry| entry.file_name().to_string_lossy().into_owned())
                             .collect()
                     })
