#!/bin/bash
# usage: gate.sh <gate-label e.g. 11 or 27b>  (run from the worktree root)
# Prints the `run: |` block of the ci.yml step named "Gate <label> —".
f=.github/workflows/ci.yml
line=$(grep -nE "^ *- name: Gate $1 —" "$f" | head -1 | cut -d: -f1)
[ -z "$line" ] && { echo "no gate $1" >&2; exit 2; }
awk -v s="$line" 'NR>s && /^      - /{exit} NR>s && started{print substr($0,11)} NR>s && /^        run: \|/{started=1}' "$f"
