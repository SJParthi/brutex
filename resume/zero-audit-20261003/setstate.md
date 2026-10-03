# setstate.py (tracker helper, kept here as text because the repo allows no .py)

Usage: `python3 setstate.py <state> <commit> <ids...>` edits /mnt/project-files/fix-board/status/zero-findings.tsv in place.

```python
import sys
f='/mnt/project-files/fix-board/status/zero-findings.tsv'
state,commit=sys.argv[1],sys.argv[2]; ids=set(sys.argv[3:])
lines=open(f).read().splitlines(); seen=set(); out=[]
for l in lines:
    p=l.split('\t')
    if p[0] in ids: p=[p[0],state,commit]; seen.add(p[0])
    out.append('\t'.join(p))
for i in ids-seen: out.append(f'{i}\t{state}\t{commit}')
open(f,'w').write('\n'.join(out)+'\n')
```
