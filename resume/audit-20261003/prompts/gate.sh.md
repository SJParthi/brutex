#!/bin/bash
# usage: gate.sh <line-of-name> ; extracts the run: | block of that step
f=.github/workflows/ci.yml
awk -v s="$1" 'NR>s && /^      - /{exit} NR>s && started{print substr($0,11)} NR>s && /^        run: \|/{started=1}' $f
