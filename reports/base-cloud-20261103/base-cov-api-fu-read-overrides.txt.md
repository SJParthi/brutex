# base-cov-api-fu-read-overrides.txt.md

Source: `state/fix/round2/base-cov-api-fu-read-overrides.txt` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````text
crates/api/src/backtest.rs:2192  the second operand of an || in the condition, never reached because the first held: not the failing arm
crates/api/src/bars_window_route_tests.rs:2103  the second operand of an || in the condition, never reached: not the failing arm
crates/api/src/disk_route.rs:1362  the second operand of an || in the condition, never reached: not the failing arm
crates/api/src/indexstopvixjson.rs:380  a closure passed inside the compared expression and never called: not the failing arm
crates/api/src/server.rs:28258  the second operand of an || in the condition, never reached: not the failing arm
crates/api/src/server.rs:28259  part of that never-reached operand
crates/api/src/server.rs:28260  part of that never-reached operand
crates/api/src/server.rs:29920  the second operand of an || in the condition, never reached: not the failing arm
crates/api/src/store_wire.rs:477  a builder closure inside the condition, never called because the cache refused first: not the failing arm
crates/api/src/ops_safety_tests.rs:229:16 False  an operand inside an .any() closure opened on the line above: one element's false side does not fail the check
crates/api/src/server.rs:24491:28 False  an operand inside an .any() closure opened above: not the failing arm
crates/api/src/strict_sweep_tests.rs:352:20 False  an operand inside an .any() closure opened above: not the failing arm
crates/api/src/strict_sweep_tests.rs:353:20 False  an operand inside an .any() closure opened above: not the failing arm
crates/api/src/strict_sweep_tests.rs:354:20 False  an operand inside an .any() closure opened above: not the failing arm
crates/api/src/backtest.rs:2191:13 False  the first operand of an || continued on the next line: its false side does not fail the check
crates/api/src/disk_route.rs:1361:13 False  the first operand of an || continued on the next line: not the failing arm
crates/api/src/server.rs:28257:13 False  the first operand of an || continued on the next line: not the failing arm
crates/api/src/server.rs:29919:13 False  the first operand of an || continued on the next line: not the failing arm
crates/api/src/bars_window_route_tests.rs:1640:12 False  an if that picks a value inside the compared expression: not a check
crates/api/src/server.rs:21730:30 False  the body of an .any() closure opened on the line above: not the failing arm
`````
