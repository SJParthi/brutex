# MANIFEST.md: the PSRB second-wave kit files, their sources and sha256 (written by kit-build-2.sh at kit time)

A fenced file holds its source byte for byte between a line that starts with five backticks and a line that is exactly five backticks. Extract and check it:

    awk 'f && /^`````$/ {exit} f {print} !f && /^`````/ {f=1}' NAME.EXT.md > NAME.EXT && sha256sum NAME.EXT

A whole-file row is hashed as it stands (`sha256sum NAME.md`). Hand-written: CLOUD.md. PARTS.md is a byte copy of `state/strategy-psrb-20261103/PARTS.md`; PARTS-WILLR-CCI.md is a byte copy of `state/strategy-psrb-20261103/PARTS-WILLR-CCI.md`.

| kit file | source | sha256 | form |
|---|---|---|---|
| `CLOUD.md` | this folder | `004a7f9bc33742518e689376265636ad308fadae0684c79cfd7c5b0c60afe609` | whole file |
| `PARTS.md` | `state/strategy-psrb-20261103/PARTS.md` | `edcfdb27d1c79734dcc1eb38404822f1d282e3f213418fa120834eafbe79225b` | whole file |
| `PARTS-WILLR-CCI.md` | `state/strategy-psrb-20261103/PARTS-WILLR-CCI.md` | `0c502de2bf6fb9ab1c376126a2aedf69e27fdd108bb4e3e76636376b16b664a0` | whole file |
| `d3-recipe.txt.md` | `state/decisions-20261027/d3-recipe.txt` | `9ed425e3b6fa309faa4108f8fe2ee7380711e892b863f1a4ea271d11588780e2` | fenced |
| `d3-route-b.sh.md` | `state/decisions-20261027/d3-route-b.sh` | `f3d70ac240cd09a0482f36e1b1b95dfec0969bfaee8f6ea394cc64962d98fb18` | fenced |
| `d3-route-b-cloud.sh.md` | `state/cloud-wave-20261103/kit/d3-route-b-cloud.sh` | `f032b525c13c0c0b2083bb682126a65971a1df3452ed18ee395bc37d4181dfd1` | fenced |
| `d3-covset.mjs.md` | `state/decisions-20261027/d3-covset.mjs` | `ecef3bdc4bd473a9c701f6f8f24e2759a0af3c9246f9c43cb3a67378f0b07adc` | fenced |
| `wr-cci.mjs.md` | `state/strategy-psrb-20261103/work-willr-cci/model/wr-cci.mjs` | `ea3d831f2be802e05ddaaef094838f20890cc481193d164d619556ff5af06b10` | fenced |
| `pops.mjs.md` | `state/strategy-psrb-20261103/work-willr-cci/model/pops.mjs` | `261a5697bab5a9ee435b04a1763036fd05e68b5d4a6a813591d28b7a64b8377d` | fenced |
| `cpr-wr6.mjs.md` | `state/strategy-psrb-20261103/work-willr-cci/model/cpr-wr6.mjs` | `0daa7c91d4d5fe181ebc1a5ae341681463702d81899bb09e74c34c8ce6abe8e3` | fenced |
| `repair1.mjs.md` | `state/strategy-psrb-20261103/specs-1105-repair1-work/repair1.mjs` | `a2f7bd48cabff11a40c89eaf0113ed14cbf7728d4db07af900ddd85e717b4ab4` | fenced |
| `repair3.mjs.md` | `state/strategy-psrb-20261103/specs-1105-repair3-work/repair3.mjs` | `d85ca768b7a2b739549bef0f5ce0bd09ffb286f491c049d132fd71eea120b444` | fenced |
