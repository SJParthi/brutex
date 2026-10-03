# lane1b/c: cargo-mutants (partial; stopped on usage limit)

Branch lane1b/c at b371913. Diff: `git diff 1087e54...HEAD -- 'crates/*.rs'`. 59 mutants listed.
Run settings: --in-place, --baseline skip, --timeout 900, test filters limited to the touched modules.
CARGO_PROFILE_DEV_OPT_LEVEL=0 and incremental were used, because two earlier attempts at the repo
profile (opt-level 3) took more than 80 min to build the cli baseline on the loaded machine, and one
of them was SIGKILLed (likely OOM). Stopped with SIGINT after 6 of 59. Tree verified clean afterwards.

| outcome | count | mutants |
|---|---|---|
| caught | 3 | lib.rs:19000 record_unadmitted -> Ok(String::new()); lib.rs:19000 -> Ok("xyzzy".into()); operation_audit.rs:585 read -> Ok(None) |
| missed | 1 | readonly_file.rs:81 regular -> Ok(Default::default()). This is the cfg(not(supported-target)) fallback, so it is not compiled on Linux x86_64. Equivalent on this host. The existing `open` fallback has the same shape. |
| unviable | 2 | readonly_file.rs:59 regular -> Ok(Default::default()) (File has no Default); operation_audit.rs:585 read -> Ok(Some(Default::default())) |
| not yet tested | 53 | the rest of the `cargo mutants --list` output |

Survivors I predict but have not run:
- search_checkpoint.rs:326, the publish_marker `NotFound` guard (true/false, ==/!=). It only changes the refusal text when
  removing the temp file fails. To kill it, let the marker_staged hook delete the temp file (NotFound) or replace it
  with a non-empty directory, then assert the exact message. Not written.
