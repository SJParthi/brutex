crates/pull/src/masters.rs:1282:5: replace holds_exactly -> Result<bool, crate::ingest::Unbounded> with Ok(true)  (shard 10, TIMEOUT)
crates/telemetry/src/tail.rs:503:15: replace > with >= in walk_back  (shard 58, TIMEOUT 4112s test; also reproduced by the coordinator)
crates/lake/src/footer.rs:96:9: replace Cursor<'_>::byte -> Result<u8, LakeError> with Ok(1)  (shard 49, TIMEOUT 4108s test)
