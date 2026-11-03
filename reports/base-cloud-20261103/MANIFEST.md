# MANIFEST.md: the BASE kit files, their sources and sha256 (written by kit-base-build.sh at kit time)

A fenced file holds its source byte for byte between a line that starts with five backticks and a line that is exactly five backticks. Extract and check it:

    awk 'f && /^`````$/ {exit} f {print} !f && /^`````/ {f=1}' NAME.EXT.md > NAME.EXT && sha256sum NAME.EXT    # json: also jq -e . NAME.EXT

A whole-file row is hashed as it stands. Hand-written here (kit-base/ of the plan folder): CLOUD.md, CONTEXT.md, DECISIONS.md; generated: SHARDS.md and every row below.

| kit file | source | sha256 | form |
|---|---|---|---|
| `CLOUD.md` | this folder | `b2c553b398447122ee721e9ba64b28d11f3c847d975cf84b67729a4ade6b8429` | whole file |
| `CONTEXT.md` | this folder | `70ccae63dcb4f151f76269c5e99b46e4d7911697a210daa90897e4dc6e672af3` | whole file |
| `DECISIONS.md` | this folder | `ecd5b701c08030ba8d4812d7666965367e259cc069dd3335358de5713d60bf51` | whole file |
| `SHARDS.md` | this folder | `6fd9660097b6230ec35220d69c60e4ad67c437192d7c6d5fa4c379182834b68c` | whole file |
| `RULES-MAC-COV-LIBS.md` | `state/cloud-wave-20261103/kit/prompts/BASE-COV-LIBS.fix-attempt4.prompt.txt` | `8c03af371190d1549e85b790093d18b8c54f6a21c5151efcc6f0b1f2225de977` | fenced |
| `RULES-MAC-COV-PULL.md` | `state/cloud-wave-20261103/kit/prompts/BASE-COV-PULL.repair-r2.prompt.txt` | `21783a199ef7b748a6f4d47b861c52712f0f420cef5f5e2871a74e07b0283d49` | fenced |
| `RULES-MAC-MSUM.md` | `state/cloud-wave-20261103/repair2-plan-work/BASE-MSUM-BACKFILL.fix.prompt.txt` | `d6fb52dcd7184d884be3f4e8e56a3527ab172636c12462f5b0c906f7578dc8f5` | fenced |
| `fix-BASE-COV-LIBS-progress.md` | `state/fix/round2/fix-BASE-COV-LIBS-progress.md` | `1576da4285f5cec057851f08aa717f4178e387d8f4d328260ea91a61ba736a02` | whole file |
| `repair-BASE-COV-PULL-progress.md` | `state/fix/round2/repair-BASE-COV-PULL-progress.md` | `c359a72a2526fe611ca973f182219aba88660fa4411b4d2e5f037aaa971870c9` | whole file |
| `fix-BASE-MSUM-BACKFILL-progress.md` | `state/fix/round2/fix-BASE-MSUM-BACKFILL-progress.md` | `367f295f2d5575cc4e0d2657b3810ea88aec4730deab2744c882e6a02483b291` | whole file |
| `fix-BASE-COV-LIBS.json.md` | `state/fix/round2/fix-BASE-COV-LIBS.json` | `1146cb9601d165d96f3d84ff1f7906243b8f62421faac868de274c73206b8b93` | fenced |
| `fix-BASE-COV-PULL.json.md` | `state/fix/round2/fix-BASE-COV-PULL.json` | `57d2aaeb2891e19e0a2e723e1014a0952e65d5a082ba82ef092ec5837fa26e9d` | fenced |
| `verify-BASE-COV-PULL-r2.json.md` | `state/fix/round2/verify-BASE-COV-PULL-r2.json` | `fb43b4d0fb7aa683304b24716c03e80319e32fe78c5d75ded2206877eb1c25c9` | fenced |
| `fix-BASE-MSUM-BACKFILL.json.md` | `state/fix/round2/fix-BASE-MSUM-BACKFILL.json` | `ba4c9ac4e631c9f996012714c1f362c45c333db5c68d1164b77d404daea34101` | fenced |
| `packages.json.md` | `state/cloud-wave-20261103/repair2-plan-work/packages.json` | `0e86485dad9d6bcda7f58a53e27f75e072235f3cac8e5e7122a75c7fbb365ea2` | fenced |
| `deep-BASE-COV-PULL-c9d653d5f914-gaps.txt.md` | `state/fix/round2/deep-BASE-COV-PULL-c9d653d5f914-gaps.txt` | `178b2ee0c14986f6b06fec704201a44e03346394b9e316fcf1903a5f6a851cec` | fenced |
| `deep-BASE-COV-PULL-f98a899398f6-set.txt.md` | `state/fix/round2/deep-BASE-COV-PULL-f98a899398f6-set.txt` | `2eafbf44079c4caa2ba9132d8590c91841c666a6864027e0bfb69caa768523f5` | fenced |
| `base-cov-pull-fu.txt.md` | `state/fix/round2/base-cov-pull-fu.txt` | `7d8263f9bebf665e2371568235a172332649ab4e5808d3396a0b3df35bf2c891` | fenced |
| `base-cov-libs-gaps-a1-after.txt.md` | `state/fix/round2/base-cov-libs-gaps-a1-after.txt` | `baa5fe2f4c1fbfcc62d4597c2e11bcd02b38adb1c07601fe75d426a9844dc98a` | fenced |
| `base-cov-libs-fu.txt.md` | `state/fix/round2/base-cov-libs-fu.txt` | `85e2001f567d000581807afcaad0adb5994fed7af5ce95e70ba54a03b41955f8` | fenced |
| `deep-BASE-COV-LIBS-ee65c9f6fcbd-set.txt.md` | `state/fix/round2/deep-BASE-COV-LIBS-ee65c9f6fcbd-set.txt` | `e9afb6563d21300008ddacf13f8b4f5693cc8d045b3d3d5f63bf4b0dec4b626f` | fenced |
| `d3-recipe.txt.md` | `state/decisions-20261027/d3-recipe.txt` | `9ed425e3b6fa309faa4108f8fe2ee7380711e892b863f1a4ea271d11588780e2` | fenced |
| `d3-route-b.sh.md` | `state/decisions-20261027/d3-route-b.sh` | `f3d70ac240cd09a0482f36e1b1b95dfec0969bfaee8f6ea394cc64962d98fb18` | fenced |
| `d3-route-b-cloud.sh.md` | `state/cloud-wave-20261103/kit/d3-route-b-cloud.sh` | `f032b525c13c0c0b2083bb682126a65971a1df3452ed18ee395bc37d4181dfd1` | fenced |
| `d3-covset.mjs.md` | `state/decisions-20261027/d3-covset.mjs` | `ecef3bdc4bd473a9c701f6f8f24e2759a0af3c9246f9c43cb3a67378f0b07adc` | fenced |
| `replay-minus-reports.sh.md` | `state/cloud-wave-20261103/replay-minus-reports.sh` | `f223185db1c35be02ae52563b4ef4f8996b7a88daa7171a3eda55f428c6e13e0` | fenced |
