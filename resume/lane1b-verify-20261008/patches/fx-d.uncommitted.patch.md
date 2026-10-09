diff --git a/crates/cli/src/stored.rs b/crates/cli/src/stored.rs
index 5ccba417..ba3f459c 100644
--- a/crates/cli/src/stored.rs
+++ b/crates/cli/src/stored.rs
@@ -1540,7 +1540,8 @@ enum ReceiptSessionV2 {
 /// dated authority the stored read asks ([`CashCloses::session_close_minute`]),
 /// so the receipt, the overlay and the minute-gap census cannot disagree about
 /// a share's close. `None` is the index calendar, unchanged. O(1): one civil
-/// conversion, and on a dated day one close lookup.
+/// conversion, and on a dated day one close lookup. **UNVERIFIED as a measured
+/// bound**, read off the source and recorded in `docs/06-limits.md` (D-4748).
 fn receipt_session_v2(
     day: i64,
     session: Session,
