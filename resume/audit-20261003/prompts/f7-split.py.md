#!/usr/bin/env python3
"""Stage-k content builder for F7's multi-item files. usage: split.py <stage> -> writes index entries."""
import subprocess, sys, re, os
os.chdir('/home/claude/wt-fix7')
BASE='3afa02c3'
stage=int(sys.argv[1])
def git(*a, inp=None):
    return subprocess.run(['git',*a],input=inp,capture_output=True,check=True).stdout
def base(path):
    return git('show',f'{BASE}:{path}').decode()
def work(path):
    return open(path).read()
def put(path, content):
    sha=git('hash-object','-w','--stdin',inp=content.encode()).decode().strip()
    mode='100644'
    git('update-index','--add','--cacheinfo',f'{mode},{sha},{path}')

# hunk-based for server.rs
def hunks(path):
    d=git('diff','-U0',BASE,'--',path).decode().split('\n')
    hs=[];cur=None
    for ln in d:
        m=re.match(r'^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@',ln)
        if m:
            cur={'os':int(m.group(1)),'oc':int(m.group(2)) if m.group(2) is not None else 1,'add':[]}
            hs.append(cur);continue
        if cur is None: continue
        if ln.startswith('+') and not ln.startswith('+++'): cur['add'].append(ln[1:])
    return hs
def apply_hunks(path, keep):
    lines=base(path).split('\n')
    hs=hunks(path)
    for h in sorted(hs,key=lambda h:h['os'],reverse=True):
        if not keep(h['os']): continue
        if h['oc']==0:
            lines[h['os']:h['os']]=h['add']
        else:
            lines[h['os']-1:h['os']-1+h['oc']]=h['add']
    return '\n'.join(lines)

SERVER={1:[16765,16778,16787,16791,17120,17163,17185,17434],2:[16930,17298],3:[784,10908,11016,14835,16055,26128,32178],4:[15936,30776,30779]}
def server_item(os_):
    for k,v in SERVER.items():
        if os_ in v: return k
    raise SystemExit(f'unassigned server hunk {os_}')

FILES={1:['Cargo.toml','Cargo.lock','crates/api/Cargo.toml'],
       3:['crates/api/src/autopilot.rs','crates/api/src/ingest.rs','crates/api/src/pullrun.rs','crates/api/src/recovery.rs'],
       4:['crates/api/src/logs.rs'],
       5:[l for l in git('diff','--name-only',BASE,'--','crates/runner','crates/cli').decode().split() ]+['crates/runner/tests/hole_after_exit.rs'],
       6:['crates/store/src/header.rs','crates/store/tests/fault.rs'],
       8:['.github/workflows/ci.yml']}
if stage==0:
    put('.github/workflows/ci.yml', work('.github/workflows/ci.yml'))
    sys.exit(0)
for k,paths in FILES.items():
    if k==stage:
        for p in paths: put(p, work(p))
if stage in SERVER:
    put('crates/api/src/server.rs', apply_hunks('crates/api/src/server.rs', lambda o: server_item(o)<=stage))
# decisions: blocks
dec=work('docs/05-decisions.md'); b=base('docs/05-decisions.md')
assert dec.startswith(b)
tail=dec[len(b):]
parts=re.split(r'(?=\n### D-151\d )',tail)
assert parts[0]=='' , repr(parts[0][:80])
num={1510:1,1511:2,1512:3,1513:4,1514:5,1515:6,1516:7}
keep=[p for p in parts[1:] if num[int(re.match(r"\n### D-(\d+)",p).group(1))]<=stage]
put('docs/05-decisions.md', b+''.join(keep))
inv=work('docs/04-invariants.md'); b=base('docs/04-invariants.md')
assert inv.startswith(b)
rows=inv[len(b):].splitlines(keepends=True)
rnum={'AGC-02':1,'AGC-03':2,'AGC-04':3,'AGC-05':4,'AGC-06':5,'AGC-07':5,'AGC-08':6}
put('docs/04-invariants.md', b+''.join(r for r in rows if rnum[r[2:8]]<=stage))
# limits
lim=work('docs/06-limits.md'); b=base('docs/06-limits.md')
head_marker='\n## Audit fixer 7: body deadline'
i=lim.index(head_marker)
body_part=lim[:i]; section=lim[i:]
# section bullets
sec_lines=section.split('\n- ')
sec_head=sec_lines[0]; bullets=sec_lines[1:]
bnum=[1,2,3,4,5]
assert len(bullets)==5, len(bullets)
# body part edits: compute by hunks relative to base
def lim_keep(os_):
    # D-1200 bullet -> 1, D-1183/D-1191 notes -> 5
    t=b.split('\n')[os_-1] if os_-1 < len(b.split('\n')) else ''
    return 1 if 'head deadline covers the HEAD' in t else 5
hs=hunks('docs/06-limits.md')
lines=b.split('\n')
for h in sorted(hs,key=lambda h:h['os'],reverse=True):
    add=h['add']
    if any('Audit fixer 7: body deadline' in a for a in add):
        continue
    k=1 if any('BODY_READ_TIMEOUT' in a for a in add) else 5
    if k>stage: continue
    if h['oc']==0: lines[h['os']:h['os']]=add
    else: lines[h['os']-1:h['os']-1+h['oc']]=add
out='\n'.join(lines)
inc=[bl for bl,k in zip(bullets,bnum) if k<=stage]
if inc:
    out=body_rebuilt=out
    final_body=lim[:i]
    # the working file's body is base-with-edits with its tail newlines trimmed
    out=out.rstrip('\n')+'\n'+section.split('\n- ')[0]+''.join('\n- '+x for x in inc)
    if not out.endswith('\n'): out+='\n'
put('docs/06-limits.md', out)
