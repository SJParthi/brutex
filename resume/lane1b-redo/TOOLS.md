# Lane 1-b redo tooling (scratch only, never commit into the repo tree)

Run CI steps locally: `python3 gate.py <worktree> <label>` (labels: `LIST`). Gate 0 first with RUNNER_TEMP=<dir>; then export SOURCE_SCAN=$RUNNER_TEMP/source-scan for the scanner gates.

## gate.py
```python
#!/usr/bin/env python3
"""Usage: gate.py <repo-root> <gate-label e.g. '11' or '27b' or 'ALLSTATIC'>
Extracts the run: block of the step named 'Gate <label> —' from .github/workflows/ci.yml and runs it with bash -e from repo root."""
import sys, re, subprocess, os
root, label = sys.argv[1], sys.argv[2]
lines = open(os.path.join(root, '.github/workflows/ci.yml')).read().split('\n')
def extract(lbl):
    for i,l in enumerate(lines):
        if re.match(r'\s*- name: Gate %s —' % re.escape(lbl), l):
            ind = len(l) - len(l.lstrip())
            j = i+1; body=None
            while j < len(lines):
                t = lines[j]
                if t.strip() and (len(t)-len(t.lstrip())) <= ind: break
                if re.match(r'\s*run: \|', t):
                    rind = len(t)-len(t.lstrip()); body=[]; j+=1
                    while j < len(lines):
                        u = lines[j]
                        if u.strip() and (len(u)-len(u.lstrip())) <= rind: break
                        body.append(u[rind+2:] if len(u)>rind else ''); j+=1
                    return '\n'.join(body)
                j+=1
    return None
labels = [label]
if label == 'LIST':
    for l in lines:
        m = re.match(r'\s*- name: Gate (\S+) —', l)
        if m: print(m.group(1))
    sys.exit(0)
body = extract(label)
if body is None: print('no gate', label); sys.exit(2)
r = subprocess.run(['bash','-eo','pipefail','-c', body], cwd=root)
print('GATE %s exit %d' % (label, r.returncode)); sys.exit(r.returncode)
```

## union.py (resolve append-only docs conflicts: keep ours, blank line, then theirs)
```python
import sys,re
for p in sys.argv[1:]:
    out=[];st=0;n=0
    for l in open(p).read().split('\n'):
        if l.startswith('<<<<<<< '): st=1;n+=1;continue
        if st and l.startswith('||||||| '): st=3;continue
        if st and l.strip()=='=======':
            st=2
            if out and out[-1].strip()!='': out.append('')
            continue
        if st and l.startswith('>>>>>>> '): st=0;continue
        if st==3: continue
        out.append(l)
    open(p,'w').write('\n'.join(out)); print(p,n,'hunks')
```
