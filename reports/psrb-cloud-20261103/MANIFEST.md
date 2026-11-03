# MANIFEST.md: the PSRB kit files, their sources and sha256 (written by kit-build.sh at kit time)

A fenced file holds its source byte for byte between a line that starts with five backticks and a line that is exactly five backticks. Extract and check it:

    awk 'f && /^`````$/ {exit} f {print} !f && /^`````/ {f=1}' NAME.EXT.md > NAME.EXT && sha256sum NAME.EXT

A whole-file row is hashed as it stands (`sha256sum NAME.md`). Hand-written: CLOUD.md. PARTS.md is a byte copy of `state/strategy-psrb-20261103/PARTS.md`.

| kit file | source | sha256 | form |
|---|---|---|---|
| `CLOUD.md` | this folder | `19dff3574450fcba9fa34a6c12f8638a1b69d9ff9ad47de1b236734939b36025` | whole file |
| `PARTS.md` | `state/strategy-psrb-20261103/PARTS.md` | `1a96fc9bf5ddbfd57c4e060ef5d96fe03156727695727c803b9a860dcc5e1a6b` | whole file |
| `d3-recipe.txt.md` | `state/decisions-20261027/d3-recipe.txt` | `9ed425e3b6fa309faa4108f8fe2ee7380711e892b863f1a4ea271d11588780e2` | fenced |
| `d3-route-b.sh.md` | `state/decisions-20261027/d3-route-b.sh` | `f3d70ac240cd09a0482f36e1b1b95dfec0969bfaee8f6ea394cc64962d98fb18` | fenced |
| `d3-route-b-cloud.sh.md` | `state/cloud-wave-20261103/kit/d3-route-b-cloud.sh` | `f032b525c13c0c0b2083bb682126a65971a1df3452ed18ee395bc37d4181dfd1` | fenced |
| `d3-covset.mjs.md` | `state/decisions-20261027/d3-covset.mjs` | `ecef3bdc4bd473a9c701f6f8f24e2759a0af3c9246f9c43cb3a67378f0b07adc` | fenced |
| `sim.mjs.md` | `state/strategy-psrb-20261103/work/sim.mjs` | `c78e72b8f629fa8a6f32c3cbba11d476af81600ca8cb88b47979a174a380c003` | fenced |
| `fixtures.mjs.md` | `state/strategy-psrb-20261103/work/fixtures.mjs` | `e481803384cad51fa4937fc61e473823f7d3999814f3dc927b72695ee469d95a` | fenced |
| `fix2.mjs.md` | `state/strategy-psrb-20261103/work/fix2.mjs` | `d03d3cebf44a1715fbef56e6bcfc973cc9e290f9bcec6dcbc5bd338ed0908264` | fenced |
| `rsi-run.mjs.md` | `state/strategy-psrb-20261103/repair1-work/rsi-run.mjs` | `a99a83adbca542faa06874e7085eb994b73c101902392c7f064808b82f333f0d` | fenced |
| `min-run.mjs.md` | `state/strategy-psrb-20261103/repair1-work/min-run.mjs` | `3c18cb3cc64ca5faa8e9b925769b1396a0802cf0ed6ee19871f770cf55c6cc62` | fenced |
| `faithful-edge.mjs.md` | `state/strategy-psrb-20261103/repair1-work/faithful-edge.mjs` | `2ed69004e574486aaa603313dfc47cc8cb4b6dca8850a4ebe4a6e528ee8b0d5b` | fenced |
