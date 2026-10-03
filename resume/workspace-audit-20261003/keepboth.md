# keepboth.sh: resolves doc-tail merge conflicts by keeping both sides
```sh
#!/bin/bash
# keep both sides of each conflict (ours then theirs)
for f in "$@"; do
awk 'BEGIN{s=0} /^<<<<<<< /{s=1;next} s==1&&/^\|\|\|\|\|\|\| /{s=3;next} s==3&&/^=======$/{s=2;next} s==1&&/^=======$/{s=2;next} s>=1&&/^>>>>>>> /{s=0;next} s==3{next} {print}' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
done
```
