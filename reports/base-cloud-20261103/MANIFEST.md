# MANIFEST.md: the BASE-COV-API kit files, their sources and sha256 (written by kit-api-build.sh)

A fenced file holds its source byte for byte between a line that starts with five backticks and a line that is exactly five backticks. Extract and check it:

    awk 'f && /^`````$/ {exit} f {print} !f && /^`````/ {f=1}' NAME.EXT.md > NAME.EXT && sha256sum NAME.EXT    # json: also jq -e . NAME.EXT

A whole-file row is hashed as it stands. Hand-written (kit/ of `state/cov-api-shards-20261105`): CLOUD.md, CONTEXT.md, DECISIONS.md; generated: SHARDS.md and every row below. Besides the files START A5 names, the kit carries what PLAN 9's BASE kit carried for a coverage shard: the set file and the (F)/(U) lists Route B and the GAP rule read, and the D3 recipe and Route B scripts.

| kit file | source | sha256 | form |
|---|---|---|---|
| `CLOUD.md` | this folder | `46b53f117dce20bcaaf4cfaccd36d4196f08cdbb80bd513ef800ae0a6705e4dc` | whole file |
| `CONTEXT.md` | this folder | `133dd7b1fd74070bd9cc851ef3ccc092e3740cd219ae0340ac39c1a6f328e639` | whole file |
| `DECISIONS.md` | this folder | `52b7fbf82675647d90f3b548b6946ab0ad99fb9f69000b09539020b20cf09052` | whole file |
| `SHARDS.md` | this folder | `293a0e6c13b8bfc4cb365260cbfaa5970a8a96aeae2eaf02eb1867ec4dd23e28` | whole file |
| `RULES-MAC-COV-API.md` | `state/cov-api-shards-20261105/work/BASE-COV-API.repair-r2.prompt.txt` | `d0fcebdeca7fa70b9d5a011e33474895e777377f6b5d36a430f2db2bcc6ade3c` | fenced |
| `verify-BASE-COV-API-r3.json.md` | `state/fix/round2/verify-BASE-COV-API-r3.json` | `1645392f5c072b3ac1f50ed80931d99729a22f59ba9736a6b2a22b9329dbf072` | fenced |
| `base-cov-api-gaps-r2.txt.md` | `state/fix/round2/base-cov-api-gaps-r2.txt` | `8ecc5c25e439679f1c74a40929b01c89916cb18980cefffdce892ec14e26e3dd` | fenced |
| `base-cov-api-fu.txt.md` | `state/fix/round2/base-cov-api-fu.txt` | `51750a67f586fee1b2536efe2334c22d35a3ae9d83d1393ecdd90f32544b4635` | fenced |
| `base-cov-api-fu-read-overrides.txt.md` | `state/fix/round2/base-cov-api-fu-read-overrides.txt` | `306be9823b843dd9b2c6d0bdb6d7b533ee6caad5b79e8c5ca1bc7e84321f3221` | fenced |
| `deep-BASE-COV-API-6ea46e59c074-set.txt.md` | `state/fix/round2/deep-BASE-COV-API-6ea46e59c074-set.txt` | `f35a70d2f47d882d9fb4c6e2a4c8f1827c6f181635d4474d154d985703b597dc` | fenced |
| `fix-BASE-COV-API.json.md` | `state/fix/round2/fix-BASE-COV-API.json` | `513210017901cde7e3713a7d1165a0f2f2f7b93686861cc45619e3c745ef8e93` | fenced |
| `fix-BASE-COV-API-progress.md` | `state/fix/round2/fix-BASE-COV-API-progress.md` | `c4278bfcfb4f2293a78afc1adc1eb99d5453178512bbd956faf8498dd9fa0ec9` | whole file |
| `repair-BASE-COV-API-progress.md` | `state/fix/round2/repair-BASE-COV-API-progress.md` | `6fe8b07b3b0968fedbe407c447c230b0f7678b9e5263615042c687981e4c7f71` | whole file |
| `verify-BASE-COV-API-r3.progress.md` | `state/fix/round2/verify-BASE-COV-API-r3.progress.md` | `ef6e75fa2f8d39f793512b334ae672c3573adff31a415e50146ee297733c0356` | whole file |
| `d3-recipe.txt.md` | `state/decisions-20261027/d3-recipe.txt` | `9ed425e3b6fa309faa4108f8fe2ee7380711e892b863f1a4ea271d11588780e2` | fenced |
| `d3-route-b.sh.md` | `state/decisions-20261027/d3-route-b.sh` | `f3d70ac240cd09a0482f36e1b1b95dfec0969bfaee8f6ea394cc64962d98fb18` | fenced |
| `d3-route-b-cloud.sh.md` | `state/cloud-wave-20261103/kit/d3-route-b-cloud.sh` | `f032b525c13c0c0b2083bb682126a65971a1df3452ed18ee395bc37d4181dfd1` | fenced |
| `d3-covset.mjs.md` | `state/decisions-20261027/d3-covset.mjs` | `ecef3bdc4bd473a9c701f6f8f24e2759a0af3c9246f9c43cb3a67378f0b07adc` | fenced |
| `replay-minus-reports.sh.md` | `state/cloud-wave-20261103/replay-minus-reports.sh` | `f223185db1c35be02ae52563b4ef4f8996b7a88daa7171a3eda55f428c6e13e0` | fenced |
